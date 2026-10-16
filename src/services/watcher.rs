//! Roblox Watcher (port of RobloxWatcher.cs and RobloxProcess.cs).
//! Unlike the C# version it finds each Roblox log file without Sysinternals Handle: the newest log created when the
//! process started belongs to it.

use crate::app::App;
use crate::platform::win as winutil;
use regex::Regex;
use std::collections::{HashMap, HashSet};
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, Instant, SystemTime};

const TIMESTAMP: &str = r"[\d+\-]+T[\d+:]+\.\w+Z,[\d.+]+,\w+,\d+[\s+]?";

#[derive(Default)]
pub struct State {
    pub running: AtomicBool,
    tracked: Mutex<HashMap<u32, Tracked>>,
    seen: Mutex<HashSet<u32>>,
    matches: Mutex<HashMap<String, String>>,
}

struct Tracked {
    log: Option<PathBuf>,
    position: u64,
    started: Instant,
    connected: bool,
    disconnected_at: Instant,
    data_model: i64,
    paused: bool,
    last_line: String,
}

fn default_matches() -> HashMap<String, String> {
    [
        ("DataModelInit", r"\[FLog::UGCGameController\] UGCGameController, initialized DataModel\((\w+)\)"),
        ("DataModelInit2", r"\[FLog::SurfaceController\] SurfaceController\[_:1\]::start dataModel\((\w+)\)"),
        ("DataModelStop", r"\[FLog::UGCGameController\] UGCGameController::leave \(blocking:\d+\) dataModel\((\w+)\)"),
        ("DataModelStop2", r"\[FLog::SurfaceController\] SurfaceController\[_:1\]::stop"),
        ("DataModelPause", r"\[FLog::SurfaceController\] SurfaceController\[_:1\]::pause dataModel\((\w+)\), view\(\w+\), destroyView:\d+\."),
        ("ReturnToApp1", r"\[FLog::SingleSurfaceApp\] returnToLuaApp: \.\.\. App not yet initialized, returning from game\."),
        ("ReturnToApp2", r"\[FLog::SingleSurfaceApp\] returnToLuaApp: \.\.\. App has been initialized, returning from game\."),
    ]
    .iter()
    .map(|(k, v)| (k.to_string(), v.to_string()))
    .collect()
}

/// The built-in patterns. They used to be downloaded from the repository at run time (a C# leftover): nothing from
/// the internet decides how the watcher reads Roblox's logs anymore, a Roblox change ships in an app update
pub async fn update_matches(app: &App) {
    *app.watcher.matches.lock().unwrap() = default_matches();
}

fn logs_folder() -> PathBuf {
    std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default().join("Roblox").join("logs")
}

/// The Roblox player log created closest after the process started
fn find_log(seconds_running: u64, taken: &HashSet<PathBuf>) -> Option<PathBuf> {
    let started = SystemTime::now() - Duration::from_secs(seconds_running);
    let mut best: Option<(Duration, PathBuf)> = None;

    for entry in std::fs::read_dir(logs_folder()).ok()?.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if !name.contains("_Player_") || !name.ends_with("_last.log") || taken.contains(&path) {
            continue;
        }
        let Ok(created) = entry.metadata().and_then(|m| m.created()) else { continue };
        let Ok(after) = created.duration_since(started - Duration::from_secs(5)) else { continue };
        if after > Duration::from_secs(120) {
            continue;
        }
        if best.as_ref().map(|(d, _)| after < *d).unwrap_or(true) {
            best = Some((after, path));
        }
    }

    best.map(|(_, p)| p)
}

/// One scan: memory/title checks, window positions, new processes (every ScanInterval seconds)
pub fn check_processes(app: &App) {
    let watcher = app.settings.lock().unwrap().ini.section("Watcher").cloned().unwrap_or_default();
    let close_low_memory = watcher.get_bool("CloseRbxMemory");
    let memory_limit = watcher.get_i64("MemoryLowValue", 200) as u64;
    let close_title = watcher.get_bool("CloseRbxWindowTitle");
    let expected_title = watcher.get("ExpectedWindowTitle").unwrap_or("Roblox").to_string();
    let remember_positions = watcher.get_bool("SaveWindowPositions");
    let focused = winutil::foreground_window();

    for process in winutil::roblox_processes() {
        let window = winutil::main_window(process.pid);
        if window.map(|w| w == focused).unwrap_or(false) {
            continue; // never touch the window the user is using
        }

        let cmd = &process.command_line;
        if cmd.is_empty() || cmd.starts_with("\\??\\") || (!cmd.contains("-t ") && !cmd.contains("-j ")) {
            continue; // Roblox's second process, closing it would kick with error 268
        }

        if process.seconds_running > 30 {
            if close_low_memory && process.memory_mb < memory_limit {
                log::info!("Killing {}: low memory ({} < {})", process.pid, process.memory_mb, memory_limit);
                winutil::kill(process.pid);
                continue;
            }
            if close_title {
                let title = window.map(winutil::window_title).unwrap_or_default();
                if title != expected_title {
                    log::info!("Killing {}: window title isn't {}, got {}", process.pid, expected_title, title);
                    winutil::kill(process.pid);
                    continue;
                }
            }

            if remember_positions {
                if let (Some(tracker), Some(rect)) = (process.tracker_id(), window.and_then(winutil::window_rect)) {
                    let name = app.store.lock().unwrap().accounts.iter().find(|a| a.browser_tracker_id.as_deref() == Some(tracker.as_str())).map(|a| a.username.clone());
                    if let Some(name) = name {
                        app.update_account(&name, |a| {
                            a.fields.insert("Window_Position_X".into(), rect.0.to_string());
                            a.fields.insert("Window_Position_Y".into(), rect.1.to_string());
                            a.fields.insert("Window_Width".into(), rect.2.to_string());
                            a.fields.insert("Window_Height".into(), rect.3.to_string());
                        });
                        app.save_accounts(false);
                    }
                }
            }
        }

        if app.watcher.seen.lock().unwrap().insert(process.pid) {
            let taken: HashSet<PathBuf> = app.watcher.tracked.lock().unwrap().values().filter_map(|t| t.log.clone()).collect();
            let log = find_log(process.seconds_running, &taken);
            let position = match (&log, watcher.get_bool("IgnoreExistingProcesses") && process.seconds_running > 10) {
                (Some(path), true) => std::fs::metadata(path).map(|m| m.len()).unwrap_or(0), // already running: only new lines matter
                _ => 0,
            };
            app.watcher.tracked.lock().unwrap().insert(
                process.pid,
                Tracked { log, position, started: Instant::now(), connected: false, disconnected_at: Instant::now() + Duration::from_secs(90), data_model: 0, paused: false, last_line: String::new() },
            );
        }
    }
}

/// Reads what the Roblox logs got since last time (every ReadInterval ms)
pub fn read_logs(app: &App) {
    let watcher = app.settings.lock().unwrap().ini.section("Watcher").cloned().unwrap_or_default();
    let exit_on_beta = watcher.get_bool("ExitOnBeta");
    let exit_no_connection = watcher.get_bool("ExitIfNoConnection");
    let timeout = watcher.get_f64("NoConnectionTimeout", 60.0);
    let verify = watcher.get_bool("VerifyDataModel");
    let ignore_existing = watcher.get_bool("IgnoreExistingProcesses");
    let matches = app.watcher.matches.lock().unwrap().clone();
    let rx = |name: &str| matches.get(name).and_then(|p| Regex::new(p).ok());

    let joining = Regex::new(&format!(r"^{TIMESTAMP}\[FLog::Output\] ! Joining game '[\w+\-]{{36}}' place \d+ at [\d+\.]+")).unwrap();
    let disconnect = Regex::new(&format!(r"^{TIMESTAMP}\[FLog::Network\] Sending disconnect with reason: (\d+)")).unwrap();
    let (init, init2, stop, stop2, pause, rta1, rta2) = (rx("DataModelInit"), rx("DataModelInit2"), rx("DataModelStop"), rx("DataModelStop2"), rx("DataModelPause"), rx("ReturnToApp1"), rx("ReturnToApp2"));
    let hex = |s: &str| i64::from_str_radix(s, 16).ok();

    // which tracked processes still run, without listing every process 4 times a second
    let mut tracked = app.watcher.tracked.lock().unwrap();
    tracked.retain(|pid, _| winutil::process_alive(*pid));
    app.watcher.seen.lock().unwrap().retain(|pid| winutil::process_alive(*pid));

    let mut kill = Vec::new();

    for (pid, t) in tracked.iter_mut() {
        if exit_no_connection && timeout > 0.0 && !t.connected && Instant::now() > t.disconnected_at && (Instant::now() - t.disconnected_at).as_secs_f64() > timeout {
            kill.push((*pid, "Lost connection".to_string()));
            continue;
        }

        let Some(path) = &t.log else { continue };
        let Ok(mut file) = std::fs::File::open(path) else { continue };
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if len <= t.position {
            continue;
        }
        let _ = file.seek(SeekFrom::Start(t.position));
        let mut bytes = Vec::new();
        let _ = file.take(len - t.position).read_to_end(&mut bytes);
        let read_before = t.position > 0;
        t.position = len;

        for line in String::from_utf8_lossy(&bytes).split('\n') {
            if joining.is_match(line) {
                t.connected = true;
                continue;
            }
            if disconnect.is_match(line) {
                t.connected = false;
                t.disconnected_at = Instant::now();
                continue;
            }

            if t.data_model <= 0 {
                if let Some(c) = init.as_ref().and_then(|r| r.captures(line)).or_else(|| init2.as_ref().and_then(|r| r.captures(line))) {
                    if let Some(dm) = hex(&c[1]) {
                        t.data_model = dm;
                        t.paused = false;
                        continue;
                    }
                }
            }

            if let Some(c) = pause.as_ref().and_then(|r| r.captures(line)) {
                if hex(&c[1]) == Some(t.data_model) {
                    t.paused = true;
                    continue;
                }
            }

            let returned = rta1.as_ref().map(|r| r.is_match(line)).unwrap_or(false) || rta2.as_ref().map(|r| r.is_match(line)).unwrap_or(false);
            if returned && exit_on_beta {
                let can_kill = ignore_existing || read_before;
                if verify && !t.last_line.is_empty() {
                    let stopped = stop.as_ref().and_then(|r| r.captures(&t.last_line)).and_then(|c| hex(&c[1])) == Some(t.data_model);
                    if (t.paused || stopped) && can_kill {
                        t.data_model = -1;
                        kill.push((*pid, "Beta home menu detected".into()));
                        break;
                    }
                } else if can_kill {
                    kill.push((*pid, "Beta home menu detected".into()));
                    break;
                }
            }

            if verify && t.paused && stop2.as_ref().map(|r| r.is_match(line)).unwrap_or(false) {
                t.data_model = -1;
            }

            t.last_line = line.to_string();
        }
        let _ = t.started;
    }
    drop(tracked);

    for (pid, reason) in kill {
        log::info!("Killing {pid}: {reason}");
        winutil::kill(pid);
    }
}

/// Starts or stops the watcher loops according to the settings
pub fn apply(app: std::sync::Arc<App>) {
    let enabled = app.setting_bool("Watcher", "Enabled");

    if !enabled {
        app.watcher.running.store(false, Ordering::SeqCst);
        return;
    }
    if app.watcher.running.swap(true, Ordering::SeqCst) {
        return; // already running, the loops read the intervals every time
    }

    let scan_app = app.clone();
    tauri::async_runtime::spawn(async move {
        update_matches(&scan_app).await;
        while scan_app.watcher.running.load(Ordering::SeqCst) {
            let a = scan_app.clone();
            let _ = tokio::task::spawn_blocking(move || check_processes(&a)).await;
            tokio::time::sleep(Duration::from_secs(scan_app.setting_i64("Watcher", "ScanInterval", 6).max(1) as u64)).await;
        }
    });

    let read_app = app;
    tauri::async_runtime::spawn(async move {
        while read_app.watcher.running.load(Ordering::SeqCst) {
            let a = read_app.clone();
            let _ = tokio::task::spawn_blocking(move || read_logs(&a)).await;
            tokio::time::sleep(Duration::from_millis(read_app.setting_i64("Watcher", "ReadInterval", 250).max(50) as u64)).await;
        }
    });
}
