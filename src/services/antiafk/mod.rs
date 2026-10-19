//! Anti-AFK (the features of Agzes/AntiAFK-RBX, MIT, rebuilt for this app and improved):
//! keeps every Roblox window from being kicked for idling, without getting in the user's way, knows which account each
//! window is, rejoins disconnected accounts through the launcher, mutes and lightens background windows, arranges them,
//! keeps statistics and can notify a Discord webhook.
//!
//! Settings live in RAMSettings.ini, section [AntiAFK].

pub mod audio;
pub mod cpu;
pub mod efficiency;
pub mod input;
pub mod layout;
pub mod rejoin;
pub mod webhook;

use crate::app::App;
use crate::platform::win as winutil;
use crate::services::launcher::{self, JoinOptions, JoinResult};
use rejoin::LogReader;
use serde::Serialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use windows::Win32::Foundation::HWND;
use windows::Win32::System::Power::{SetThreadExecutionState, ES_CONTINUOUS, ES_SYSTEM_REQUIRED};

const SECTION: &str = "AntiAFK";
/// Roblox kicks after 20 minutes without input
const ROBLOX_IDLE_KICK: u64 = 20 * 60;

pub use crate::core::settings::ANTI_AFK_DEFAULTS as DEFAULTS;


#[derive(Default)]
struct Tracked {
    log: Option<LogReader>,
    account: Option<String>,
    disconnected_since: Option<Instant>,
    handled_disconnect: bool,
}

#[derive(Default)]
struct Stats {
    seconds: u64,
    actions: u64,
    rejoins: u64,
    sessions: u64,
    longest: u64,
}

#[derive(Default)]
pub struct State {
    running: AtomicBool,
    /// the user stopped it while these windows were open, auto start waits for new ones
    stopped_with: Mutex<Vec<u32>>,
    next_action: Mutex<Option<Instant>>,
    session_start: Mutex<Option<Instant>>,
    last_tick: Mutex<Option<Instant>>,
    tracked: Mutex<HashMap<u32, Tracked>>,
    rejoins: Mutex<HashMap<String, Vec<Instant>>>,
    layout: Mutex<Vec<u32>>,
    muted: Mutex<HashSet<u32>>,
    efficient: Mutex<HashSet<u32>>,
    /// each window's CPU limit as last applied (pid -> percent, None = no limit)
    cpu_limits: Mutex<HashMap<u32, Option<u32>>>,
    hidden: AtomicBool,
    reminded: AtomicBool,
    run_now: AtomicBool,
    started: AtomicBool,
    /// a round is due but the user is typing or moving the mouse, since when
    waiting_since: Mutex<Option<Instant>>,
    /// keys are being pressed (on a thread of its own, the engine keeps going)
    acting: AtomicBool,
    /// the last status sent to the interface (without the idle time) and when
    last_sent: Mutex<(String, Option<Instant>)>,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct WindowView {
    pid: u32,
    account: Option<String>,
    title: String,
    place_id: Option<i64>,
    /// playing, disconnected, loading
    state: &'static str,
    included: bool,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    running: bool,
    seconds_to_next: Option<u64>,
    idle_seconds: u64,
    windows: Vec<WindowView>,
    stats: serde_json::Value,
    hidden: bool,
    /// "waiting" for the user to stop, "acting" while keys are pressed, otherwise none
    phase: Option<&'static str>,
}

struct Window {
    pid: u32,
    hwnd: HWND,
    tracker: Option<String>,
    /// the private server it was launched into (its access code), to rejoin that same server
    private_access_code: Option<String>,
    seconds_running: u64,
}

/// Every Roblox window, hidden ones too, so Anti-AFK keeps them active and they stay in the list.
/// Found by their window class every second (cheap), the process details are read once per Roblox.
fn roblox_windows() -> Vec<Window> {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    winutil::roblox_main_windows()
        .into_iter()
        .filter_map(|(pid, hwnd)| {
            let info = winutil::roblox_info(pid)?;
            Some(Window { pid, hwnd, tracker: info.tracker_id, private_access_code: info.private_access_code, seconds_running: now.saturating_sub(info.started) })
        })
        .collect()
}

/// The Roblox windows to show again (never Roblox's other hidden helper windows)
fn hidden_roblox_windows() -> Vec<HWND> {
    winutil::roblox_main_windows().into_iter().map(|(_, hwnd)| hwnd).filter(|h| unsafe { !windows::Win32::UI::WindowsAndMessaging::IsWindowVisible(*h).as_bool() }).collect()
}

impl App {
    fn afk(&self, key: &str) -> String {
        let value = self.setting(SECTION, key);
        if value.is_empty() {
            DEFAULTS.iter().find(|(k, _)| *k == key).map(|(_, v)| v.to_string()).unwrap_or_default()
        } else {
            value
        }
    }

    fn afk_flag(&self, key: &str) -> bool {
        self.afk(key).eq_ignore_ascii_case("true")
    }

    fn afk_number(&self, key: &str) -> u64 {
        self.afk(key).parse::<u64>().unwrap_or(0)
    }
}

fn stats(app: &App) -> Stats {
    Stats {
        seconds: app.afk_number("StatSeconds"),
        actions: app.afk_number("StatActions"),
        rejoins: app.afk_number("StatRejoins"),
        sessions: app.afk_number("StatSessions"),
        longest: app.afk_number("StatLongest"),
    }
}

fn add_stat(app: &App, key: &str, amount: u64) {
    let value = app.afk_number(key) + amount;
    app.settings.lock().unwrap().set(SECTION, key, &value.to_string());
}

pub fn reset_stats(app: &App) {
    for key in ["StatSeconds", "StatActions", "StatRejoins", "StatSessions", "StatLongest"] {
        app.settings.lock().unwrap().set(SECTION, key, "0");
    }
    app.settings.lock().unwrap().save();
}

/// Sends a webhook message when that kind of event is turned on, in the app's language
fn notify(app: &Arc<App>, event: webhook::Event, text: String) {
    let url = app.setting(SECTION, "Webhook");
    if url.is_empty() || (event != webhook::Event::Test && !app.afk_flag(event.setting())) {
        return;
    }
    let (title, text) = (app.tr(event.title()), app.tr(&text));
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        if let Err(e) = webhook::send(&app.api.http, &url, event, &title, &text).await {
            log::warn!("Webhook failed: {e}");
        }
    });
}

pub async fn test_webhook(app: &App) -> anyhow::Result<()> {
    let (title, text) = (app.tr(webhook::Event::Test.title()), app.tr(crate::text!("Roblox Account Manager can post here.")));
    webhook::send(&app.api.http, &app.setting(SECTION, "Webhook"), webhook::Event::Test, &title, &text).await
}

pub fn is_running(app: &App) -> bool {
    app.anti_afk().running.load(Ordering::SeqCst)
}

pub fn start(app: &Arc<App>, reason: &str) {
    let state = app.anti_afk();
    if state.running.swap(true, Ordering::SeqCst) {
        return;
    }
    *state.session_start.lock().unwrap() = Some(Instant::now());
    *state.next_action.lock().unwrap() = Some(Instant::now() + Duration::from_secs(5));
    state.stopped_with.lock().unwrap().clear();
    add_stat(app, "StatSessions", 1);
    app.settings.lock().unwrap().save();

    if app.afk_flag("KeepAwake") {
        unsafe {
            SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
        }
    }

    let count = roblox_windows().len();
    notify(app, webhook::Event::Started, format!("{reason} · Roblox windows: {count}"));
    app.emit("antiafk", status(app));
}

pub fn stop(app: &Arc<App>, by_user: bool) {
    let state = app.anti_afk();
    if !state.running.swap(false, Ordering::SeqCst) {
        return;
    }
    if let Some(start) = state.session_start.lock().unwrap().take() {
        let seconds = start.elapsed().as_secs();
        if seconds > app.afk_number("StatLongest") {
            app.settings.lock().unwrap().set(SECTION, "StatLongest", &seconds.to_string());
        }
        app.settings.lock().unwrap().save();
        notify(app, webhook::Event::Stopped, format!("Ran for {}", format_duration(seconds)));
    }
    if by_user {
        *state.stopped_with.lock().unwrap() = roblox_windows().iter().map(|w| w.pid).collect();
    }
    *state.waiting_since.lock().unwrap() = None;
    unsafe {
        SetThreadExecutionState(ES_CONTINUOUS);
    }
    app.emit("antiafk", status(app));
}

/// A round right away, without waiting for the user to be idle (ignored while one is going)
pub fn run_now(app: &App) {
    let state = app.anti_afk();
    if !state.acting.load(Ordering::SeqCst) {
        state.run_now.store(true, Ordering::SeqCst);
    }
}

pub fn format_duration(seconds: u64) -> String {
    let (h, m) = (seconds / 3600, (seconds % 3600) / 60);
    if h > 0 {
        format!("{h}h {m}m")
    } else {
        format!("{m}m {}s", seconds % 60)
    }
}

fn included_accounts(app: &App) -> Vec<String> {
    app.afk("Accounts").split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect()
}

/// The CPU limit an account has (its "CPULimit" field, percent of the PC), if any
pub fn cpu_limit_of(account: &crate::core::accounts::Account) -> Option<u32> {
    account.field("CPULimit").parse::<u32>().ok().filter(|n| (cpu::MIN..=cpu::MAX).contains(n))
}

fn account_for(app: &App, tracker: &Option<String>) -> Option<String> {
    let tracker = tracker.as_ref()?;
    app.store.lock().unwrap().accounts.iter().find(|a| a.browser_tracker_id.as_deref() == Some(tracker.as_str())).map(|a| a.username.clone())
}

/// Window tools from the interface
pub fn tool(app: &App, kind: &str) -> anyhow::Result<String> {
    let windows: Vec<HWND> = roblox_windows().iter().map(|w| w.hwnd).collect();
    let pids: Vec<u32> = winutil::roblox_processes().iter().map(|p| p.pid).collect();
    let state = app.anti_afk();

    let message = match kind {
        "grid" => {
            layout::arrange(&windows);
            match windows.len() {
                1 => "Arranged 1 window".into(),
                n => format!("Arranged {n} windows"),
            }
        }
        "hide" => {
            layout::set_hidden(&windows, true);
            state.hidden.store(true, Ordering::SeqCst);
            "Roblox windows hidden".into()
        }
        "show" => {
            layout::set_hidden(&hidden_roblox_windows(), false);
            state.hidden.store(false, Ordering::SeqCst);
            "Roblox windows shown".into()
        }
        "seeThrough" => {
            layout::set_opacity(&windows, 150);
            "Done".into()
        }
        "solid" => {
            layout::set_opacity(&windows, 255);
            "Done".into()
        }
        "mute" => {
            audio::set_muted(&pids, true);
            "Roblox muted".into()
        }
        "unmute" => {
            audio::set_muted(&pids, false);
            state.muted.lock().unwrap().clear();
            "Roblox unmuted".into()
        }
        "closeAll" => {
            for pid in &pids {
                winutil::kill(*pid);
            }
            match pids.len() {
                1 => "Closed 1 Roblox process".into(),
                n => format!("Closed {n} Roblox processes"),
            }
        }
        _ => anyhow::bail!("unknown"),
    };
    app.emit("antiafk", status(app));
    Ok(message)
}

/// Whether a due round starts now, keeps waiting for the user to stop typing and moving the mouse,
/// or gives up for now (it never takes the focus while the user is busy)
#[derive(Debug, PartialEq, Eq)]
enum Ready {
    Go,
    Wait,
    GiveUp,
}

/// `busy`: a full screen app is in front (a game, a video), never interrupted either
fn ready(user_safe: bool, idle: u64, idle_needed: u64, busy: bool, waited: Duration, max_wait: Duration) -> Ready {
    if !busy && (!user_safe || idle >= idle_needed) {
        Ready::Go
    } else if waited > max_wait {
        Ready::GiveUp
    } else {
        Ready::Wait
    }
}

/// One anti-AFK round over the chosen windows, stops early when Anti-AFK is stopped
fn cycle(app: &Arc<App>, windows: &[Window], forced: bool) {
    let accounts = included_accounts(app);
    let action = input::Action::from_setting(&app.afk("Action"));
    let repeat = app.afk_number("Repeat").clamp(1, 10) as u32;
    let reset = app.afk_flag("AutoReset");
    let skip_focused = app.afk_flag("SkipFocused") && !forced;
    let previous = input::foreground();
    // how the windows were stacked, so the ones on top go back on top afterwards
    let stacking = input::z_order();
    let hidden = app.anti_afk().hidden.load(Ordering::SeqCst);
    let mut done = 0;
    let mut moved = Vec::new();

    for w in windows {
        if !app.anti_afk().running.load(Ordering::SeqCst) {
            break;
        }
        let account = account_for(app, &w.tracker);
        if !accounts.is_empty() && !account.as_ref().map(|a| accounts.contains(a)).unwrap_or(false) {
            continue;
        }
        // the user is playing in this window right now, it isn't idle
        if skip_focused && w.hwnd == previous && input::idle_seconds() < 60 {
            continue;
        }

        let minimized = input::was_minimized(w.hwnd);
        moved.push(w.hwnd);
        if hidden {
            layout::set_hidden(&[w.hwnd], false);
        }
        if input::focus(w.hwnd) {
            input::perform(action, repeat);
            if reset {
                input::reset_character();
            }
            done += 1;
        }
        if minimized {
            input::minimize(w.hwnd);
        }
        if hidden {
            layout::set_hidden(&[w.hwnd], true);
        }
    }

    if !previous.0.is_null() && done > 0 && !input::was_minimized(previous) {
        input::focus(previous);
    }
    if !moved.is_empty() {
        input::restore_z_order(&stacking, &moved);
    }

    if done > 0 {
        add_stat(app, "StatActions", done as u64);
        notify(app, webhook::Event::Action, format!("Kept {done} window(s) active"));
    }
}

/// How long a rejoin waits for the internet to come back, checking every few seconds
const OFFLINE_WAIT: Duration = Duration::from_secs(30 * 60);

/// Relaunches an account into the game it was disconnected from. It waits for the internet first: a lost connection is
/// often the internet dropping, and closing the window before Roblox can be reached left the account with none
fn rejoin(app: &Arc<App>, pid: u32, account: &str, place: i64, job: Option<String>, private: Option<String>) {
    let Some(acc) = app.account(account) else { return };
    let same_server = app.afk_flag("RejoinSameServer");
    let app = app.clone();
    let account = account.to_string();
    tauri::async_runtime::spawn(async move {
        let started = Instant::now();
        let mut told = false;
        while app.api.client_version().await.is_err() {
            if started.elapsed() > OFFLINE_WAIT {
                notify(&app, webhook::Event::Error, format!("{account}: {}", app.tr(crate::text!("Roblox couldn't be reached for 30 minutes, not rejoining"))));
                return;
            }
            if !told {
                told = true;
                log::info!("{account} lost the connection, waiting for Roblox to be reachable before rejoining");
            }
            tokio::time::sleep(Duration::from_secs(10)).await;
        }

        // still disconnected? (Roblox's own Reconnect, or the window closed, while waiting)
        let still = app.anti_afk().tracked.lock().unwrap().get(&pid).map(|t| t.disconnected_since.is_some()).unwrap_or(false);
        if !still {
            return;
        }
        {
            let mut rejoins = app.anti_afk().rejoins.lock().unwrap();
            let recent = rejoins.entry(account.clone()).or_default();
            recent.retain(|t| t.elapsed() < Duration::from_secs(600));
            if recent.len() >= 3 {
                notify(&app, webhook::Event::Error, format!("{account} keeps getting disconnected, stopped rejoining it for now"));
                return;
            }
            recent.push(Instant::now());
        }

        winutil::kill(pid);
        tokio::time::sleep(Duration::from_secs(3)).await;
        let tracker = app.ensure_tracker(&acc.username);
        let mut job_id = if same_server { job.unwrap_or_default() } else { String::new() };
        // a private server: back into it with its access code, like Roblox's Reconnect (its instance id alone can't
        // be joined, and "any server" would be a public one)
        if let Some(code) = private {
            log::info!("{account}: rejoining the private server it was in");
            let opts = JoinOptions { place_id: place, job_id: code, join_vip: true, privacy_ok: true, ..app.join_options() };
            return finish_rejoin(&app, &account, place, launcher::join(&app.api, &acc.security_token, &tracker, opts).await);
        }
        // the same server, unless Roblox says it can't be joined anymore (it shut down, e.g. for a game update, or
        // it's full): then any server of the game
        if !job_id.is_empty() {
            let status = app.api.join_game_instance(&acc.security_token, place, &job_id, false).await.ok().and_then(|r| r.json()).and_then(|v| v["status"].as_i64());
            if !rejoin::server_joinable(status) {
                log::info!("{account}: server {job_id} can't be joined anymore (status {status:?}), rejoining any server of {place}");
                job_id.clear();
            }
        }
        let opts = JoinOptions { place_id: place, job_id, privacy_ok: true, ..app.join_options() };
        finish_rejoin(&app, &account, place, launcher::join(&app.api, &acc.security_token, &tracker, opts).await);
    });
}

fn finish_rejoin(app: &Arc<App>, account: &str, place: i64, result: JoinResult) {
    match result {
        JoinResult::Success => {
            add_stat(app, "StatRejoins", 1);
            log::info!("{account} rejoined place {place}");
            app.toast("success", format!("Rejoined {account}"));
            notify(app, webhook::Event::Rejoined, format!("{account} rejoined place {place}"));
        }
        JoinResult::Error(message) => {
            log::warn!("{account}: rejoining failed: {message}");
            notify(app, webhook::Event::Error, format!("{account}: {}", app.tr(&message)));
        }
        JoinResult::NeedsPrivacy => notify(app, webhook::Event::Error, format!("{account}: {}", app.tr(crate::text!("Could not rejoin")))),
    }
}

/// Seconds until the next round: the fixed interval, or with "random" a different one each time between the two
/// limits (`roll` from 0 to 1). Always from 30 seconds to a minute before Roblox's idle kick
fn pick_interval(mode: &str, fixed: u64, min: u64, max: u64, roll: f64) -> u64 {
    let limit = |s: u64| s.clamp(30, ROBLOX_IDLE_KICK - 60);
    if mode.eq_ignore_ascii_case("random") {
        let (low, high) = (limit(min.min(max)), limit(min.max(max)));
        low + ((high - low) as f64 * roll.clamp(0.0, 1.0)).round() as u64
    } else {
        limit(fixed)
    }
}

/// The engine, one tick per second on its own thread (Windows' keep awake request is per thread)
pub fn spawn(app: Arc<App>) {
    let state = app.anti_afk();
    if state.started.swap(true, Ordering::SeqCst) {
        return;
    }

    std::thread::spawn(move || loop {
        tick(&app);
        std::thread::sleep(Duration::from_secs(1));
    });
}

fn tick(app: &Arc<App>) {
    let state = app.anti_afk();
    let windows = roblox_windows();
    let pids: Vec<u32> = windows.iter().map(|w| w.pid).collect();
    let running = state.running.load(Ordering::SeqCst);
    let foreground = input::foreground();
    let mut foreground_pid = 0u32;
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(foreground, Some(&mut foreground_pid));
    }

    // stats while running
    {
        let mut last = state.last_tick.lock().unwrap();
        if running {
            if let Some(l) = *last {
                add_stat(app, "StatSeconds", l.elapsed().as_secs_f64().round() as u64);
            }
            // the longest session as it goes: a session that ends with the app closing or updating counts too
            if let Some(session) = state.session_start.lock().unwrap().map(|s| s.elapsed().as_secs()) {
                if session > app.afk_number("StatLongest") {
                    let mut settings = app.settings.lock().unwrap();
                    settings.set(SECTION, "StatLongest", &session.to_string());
                    if session % 60 == 0 {
                        settings.save();
                    }
                }
            }
        }
        *last = Some(Instant::now());
    }

    // follow every window's log (game, server, disconnects)
    {
        let mut tracked = state.tracked.lock().unwrap();
        tracked.retain(|pid, _| pids.contains(pid));
        let taken: HashSet<PathBuf> = tracked.values().filter_map(|t| t.log.as_ref().map(|l| l.path().clone())).collect();
        for w in &windows {
            let t = tracked.entry(w.pid).or_default();
            if t.account.is_none() {
                t.account = account_for(app, &w.tracker);
            }
            if t.log.is_none() {
                t.log = rejoin::find_log(w.seconds_running, &taken).map(LogReader::new);
            }
            if let Some(log) = t.log.as_mut() {
                log.update();
                // opened from the Roblox app or the website: the user in its log tells the account
                if t.account.is_none() {
                    t.account = log.state.user_id.and_then(|id| app.store.lock().unwrap().accounts.iter().find(|a| a.user_id == id).map(|a| a.username.clone()));
                }
                match log.state.disconnected {
                    Some(_) if t.disconnected_since.is_none() => {
                        t.disconnected_since = Some(Instant::now());
                        let who = t.account.as_deref().unwrap_or("not one of the app's accounts, not rejoining");
                        log::info!("Roblox {} lost the connection (account: {who})", w.pid);
                        t.handled_disconnect = false;
                        notify(app, webhook::Event::Disconnected, format!("{} lost the connection", t.account.clone().unwrap_or_else(|| format!("Roblox {}", w.pid))));
                    }
                    None => t.disconnected_since = None,
                    _ => {}
                }
            }
        }
    }

    // auto rejoin a few seconds after a disconnect
    if app.afk_flag("AutoRejoin") {
        let mut todo = Vec::new();
        for (pid, t) in state.tracked.lock().unwrap().iter_mut() {
            if let (Some(since), Some(account), Some(log)) = (t.disconnected_since, t.account.clone(), t.log.as_ref()) {
                if !t.handled_disconnect && since.elapsed() > Duration::from_secs(8) {
                    if let Some(place) = log.state.place_id {
                        t.handled_disconnect = true;
                        let private = windows.iter().find(|w| w.pid == *pid).and_then(|w| w.private_access_code.clone());
                        todo.push((*pid, account, place, log.state.job_id.clone(), private));
                    }
                }
            }
        }
        for (pid, account, place, job, private) in todo {
            rejoin(app, pid, &account, place, job, private);
        }
    }

    // each account's CPU limit on its window (it can change while the game runs)
    {
        let mut applied = state.cpu_limits.lock().unwrap();
        applied.retain(|pid, _| pids.contains(pid));
        cpu::forget_except(&pids);
        for (pid, t) in state.tracked.lock().unwrap().iter() {
            let Some(account) = t.account.as_ref().and_then(|name| app.account(name)) else { continue };
            let wanted = cpu_limit_of(&account);
            // a window without a limit that was never capped needs nothing
            if applied.get(pid) == Some(&wanted) || (wanted.is_none() && !applied.contains_key(pid)) {
                continue;
            }
            if let Err(e) = cpu::set(*pid, wanted) {
                log::warn!("{}: {e}", account.username);
            }
            // tried once per change, not every second
            applied.insert(*pid, wanted);
        }
    }

    // mute / lighten the windows the user isn't looking at
    sync_background(&state.muted, &pids, foreground_pid, app.afk_flag("MuteBackground"), |list, on| {
        audio::set_muted(list, on);
    });
    sync_background(&state.efficient, &pids, foreground_pid, app.afk_flag("Efficiency"), |list, on| {
        for pid in list {
            efficiency::set_efficient(*pid, on);
        }
    });

    // arrange the windows when one opens or closes
    if app.afk_flag("AutoGrid") {
        let mut last = state.layout.lock().unwrap();
        let mut sorted = pids.clone();
        sorted.sort();
        if *last != sorted && !windows.is_empty() {
            layout::arrange(&windows.iter().map(|w| w.hwnd).collect::<Vec<_>>());
            *last = sorted;
        }
    }

    // starting by itself
    if !running && !windows.is_empty() {
        let stopped_with = state.stopped_with.lock().unwrap().clone();
        let new_windows = stopped_with.is_empty() || pids.iter().any(|p| !stopped_with.contains(p));
        if app.afk_flag("AutoStart") && new_windows {
            start(app, crate::text!("Started automatically"));
        } else if app.afk_flag("AutoStartIdle") {
            let idle = input::idle_seconds();
            if idle >= ROBLOX_IDLE_KICK - 2 * 60 && !state.reminded.swap(true, Ordering::SeqCst) {
                app.toast("success", "You've been inactive for 18 minutes, Anti-AFK starts in a minute");
            }
            if idle >= ROBLOX_IDLE_KICK - 60 {
                start(app, crate::text!("Started after inactivity"));
            }
            if idle < 60 {
                state.reminded.store(false, Ordering::SeqCst);
            }
        }
    }
    if windows.is_empty() {
        state.stopped_with.lock().unwrap().clear();
    }

    // the round itself: the engine never waits or presses keys here, so the status keeps coming every second
    let running = state.running.load(Ordering::SeqCst);
    if running {
        let now = Instant::now();
        let forced = state.run_now.load(Ordering::SeqCst);
        let due = state.next_action.lock().unwrap().map(|n| now >= n).unwrap_or(true);
        if (due || forced) && !windows.is_empty() && !state.acting.load(Ordering::SeqCst) {
            let waited = now - *state.waiting_since.lock().unwrap().get_or_insert(now);
            let user_safe = app.afk_flag("UserSafe") && !forced;
            let busy = !forced && app.afk_flag("SkipFullScreen") && input::full_screen_in_front() && !pids.contains(&foreground_pid);
            let max_wait = Duration::from_secs(app.afk_number("MaxWaitSeconds"));
            match ready(user_safe, input::idle_seconds(), app.afk_number("IdleSeconds").max(1), busy, waited, max_wait) {
                Ready::Wait => {}
                Ready::GiveUp => {
                    *state.waiting_since.lock().unwrap() = None;
                    *state.next_action.lock().unwrap() = Some(now + Duration::from_secs(30));
                }
                Ready::Go => {
                    *state.waiting_since.lock().unwrap() = None;
                    state.run_now.store(false, Ordering::SeqCst);
                    state.acting.store(true, Ordering::SeqCst);
                    let app = app.clone();
                    std::thread::spawn(move || {
                        cycle(&app, &roblox_windows(), forced);
                        let state = app.anti_afk();
                        let interval = pick_interval(&app.afk("IntervalMode"), app.afk_number("Interval"), app.afk_number("IntervalMin"), app.afk_number("IntervalMax"), rand::random());
                        *state.next_action.lock().unwrap() = Some(Instant::now() + Duration::from_secs(interval));
                        state.acting.store(false, Ordering::SeqCst);
                        app.emit("antiafk", status(&app));
                    });
                }
            }
        }
        if app.afk_flag("KeepAwake") {
            unsafe {
                SetThreadExecutionState(ES_CONTINUOUS | ES_SYSTEM_REQUIRED);
            }
        }
    }

    send_status(app);
}

/// The status for the interface, only when something in it changed (the idle time aside, it changes every
/// second) or every 5 seconds: sending it every second kept the interface busy for nothing
fn send_status(app: &Arc<App>) {
    let status = status(app);
    let mut key = serde_json::to_value(&status).unwrap_or_default();
    if let Some(o) = key.as_object_mut() {
        o.remove("idleSeconds");
    }
    let key = key.to_string();
    let mut last = app.anti_afk().last_sent.lock().unwrap();
    let due = last.1.map(|t| t.elapsed() >= Duration::from_secs(5)).unwrap_or(true);
    if due || last.0 != key {
        *last = (key, Some(Instant::now()));
        drop(last);
        app.emit("antiafk", status);
    }
}

/// Applies `change(pids, on)` so only the windows the user isn't looking at have the effect, undoes it when turned off
fn sync_background(applied: &Mutex<HashSet<u32>>, pids: &[u32], foreground_pid: u32, enabled: bool, change: impl Fn(&[u32], bool)) {
    let mut applied = applied.lock().unwrap();
    applied.retain(|p| pids.contains(p));

    let wanted: HashSet<u32> = if enabled { pids.iter().copied().filter(|p| *p != foreground_pid).collect() } else { HashSet::new() };
    let to_apply: Vec<u32> = wanted.difference(&applied).copied().collect();
    let to_undo: Vec<u32> = applied.difference(&wanted).copied().collect();

    if !to_apply.is_empty() {
        change(&to_apply, true);
    }
    if !to_undo.is_empty() {
        change(&to_undo, false);
    }
    *applied = wanted;
}

pub fn status(app: &App) -> Status {
    let state = app.anti_afk();
    let accounts = included_accounts(app);
    let tracked = state.tracked.lock().unwrap();
    let s = stats(app);
    let windows: Vec<WindowView> = roblox_windows()
        .into_iter()
        .map(|w| {
            let t = tracked.get(&w.pid);
            let account = t.and_then(|t| t.account.clone()).or_else(|| account_for(app, &w.tracker));
            let log = t.and_then(|t| t.log.as_ref()).map(|l| l.state.clone()).unwrap_or_default();
            WindowView {
                pid: w.pid,
                included: accounts.is_empty() || account.as_ref().map(|a| accounts.contains(a)).unwrap_or(false),
                account,
                title: winutil::window_title(w.hwnd),
                place_id: log.place_id,
                state: if log.disconnected.is_some() { "disconnected" } else if log.place_id.is_some() { "playing" } else { "loading" },
            }
        })
        .collect();

    let running = state.running.load(Ordering::SeqCst);
    let phase = if !running {
        None
    } else if state.acting.load(Ordering::SeqCst) {
        Some("acting")
    } else if state.waiting_since.lock().unwrap().is_some() {
        Some("waiting")
    } else {
        None
    };

    Status {
        running,
        // no countdown while waiting, pressing keys, or with no Roblox window to keep active
        seconds_to_next: if phase.is_some() || windows.is_empty() { None } else { state.next_action.lock().unwrap().map(|n| n.saturating_duration_since(Instant::now()).as_secs()) },
        phase,
        idle_seconds: input::idle_seconds(),
        windows,
        stats: serde_json::json!({ "seconds": s.seconds, "actions": s.actions, "rejoins": s.rejoins, "sessions": s.sessions, "longest": s.longest }),
        hidden: state.hidden.load(Ordering::SeqCst),
    }
}

/// Puts everything back when the app closes: sound, priority, hidden windows, sleep
pub fn restore_all(app: &App) {
    let state = app.anti_afk();
    let muted: Vec<u32> = state.muted.lock().unwrap().drain().collect();
    audio::set_muted(&muted, false);
    for pid in state.efficient.lock().unwrap().drain() {
        efficiency::set_efficient(pid, false);
    }
    if state.hidden.load(Ordering::SeqCst) {
        layout::set_hidden(&hidden_roblox_windows(), false);
    }
    unsafe {
        SetThreadExecutionState(ES_CONTINUOUS);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_interval_is_fixed_or_random_between_limits() {
        assert_eq!(pick_interval("fixed", 540, 0, 0, 0.7), 540);
        // never under 30 seconds or past a minute before Roblox's kick
        assert_eq!(pick_interval("fixed", 5, 0, 0, 0.0), 30);
        assert_eq!(pick_interval("fixed", 5000, 0, 0, 0.0), ROBLOX_IDLE_KICK - 60);
        // random: from the low limit to the high one, whichever way round they were typed
        assert_eq!(pick_interval("random", 0, 300, 900, 0.0), 300);
        assert_eq!(pick_interval("random", 0, 300, 900, 1.0), 900);
        assert_eq!(pick_interval("random", 0, 900, 300, 0.5), 600);
        assert_eq!(pick_interval("Random", 0, 300, 5000, 1.0), ROBLOX_IDLE_KICK - 60);
    }

    #[test]
    fn a_due_round_waits_for_the_user_then_gives_up() {
        let s = Duration::from_secs;
        // user safe off, or the user already idle: right away
        assert_eq!(ready(false, 0, 5, false, s(0), s(90)), Ready::Go);
        assert_eq!(ready(true, 5, 5, false, s(0), s(90)), Ready::Go);
        // the user is typing: wait, up to the limit, then try again later
        assert_eq!(ready(true, 1, 5, false, s(10), s(90)), Ready::Wait);
        assert_eq!(ready(true, 1, 5, false, s(91), s(90)), Ready::GiveUp);
        // a full screen app in front: never, even with the user away
        assert_eq!(ready(true, 999, 5, true, s(10), s(90)), Ready::Wait);
        assert_eq!(ready(false, 999, 5, true, s(91), s(90)), Ready::GiveUp);
    }
}
