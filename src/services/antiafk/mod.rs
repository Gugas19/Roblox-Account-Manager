//! Anti-AFK (the features of Agzes/AntiAFK-RBX, MIT, rebuilt for this app and improved):
//! keeps every Roblox window from being kicked for idling, without getting in the user's way, knows which account each
//! window is, rejoins disconnected accounts through the launcher, mutes and lightens background windows, arranges them,
//! keeps statistics and can notify a Discord webhook.
//!
//! Settings live in RAMSettings.ini, section [AntiAFK].

pub mod audio;
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
    hidden: AtomicBool,
    reminded: AtomicBool,
    run_now: AtomicBool,
    started: AtomicBool,
    /// a round is due but the user is typing or moving the mouse, since when
    waiting_since: Mutex<Option<Instant>>,
    /// keys are being pressed (on a thread of its own, the engine keeps going)
    acting: AtomicBool,
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
    seconds_running: u64,
}

/// Roblox's own window title, how hidden windows are told apart from its helper windows
const ROBLOX_TITLE: &str = "Roblox";

/// A Roblox window hidden with "Hide windows" (only visible windows are found otherwise)
fn hidden_main_window(pid: u32) -> Option<HWND> {
    winutil::all_windows_of(pid).into_iter().find(|h| winutil::window_title(*h) == ROBLOX_TITLE)
}

/// Every Roblox window, hidden ones too, so Anti-AFK keeps them active and they stay in the list
fn roblox_windows() -> Vec<Window> {
    winutil::roblox_processes()
        .into_iter()
        .filter_map(|p| {
            let hwnd = winutil::main_window(p.pid).or_else(|| hidden_main_window(p.pid))?;
            Some(Window { pid: p.pid, hwnd, tracker: p.tracker_id(), seconds_running: p.seconds_running })
        })
        .collect()
}

/// The Roblox windows to show again (never Roblox's other hidden helper windows)
fn hidden_roblox_windows() -> Vec<HWND> {
    winutil::roblox_processes().into_iter().filter_map(|p| hidden_main_window(p.pid)).collect()
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

fn ready(user_safe: bool, idle: u64, idle_needed: u64, waited: Duration, max_wait: Duration) -> Ready {
    if !user_safe || idle >= idle_needed {
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
    let hidden = app.anti_afk().hidden.load(Ordering::SeqCst);
    let mut done = 0;

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

    if !previous.0.is_null() && done > 0 {
        input::focus(previous);
    }

    if done > 0 {
        add_stat(app, "StatActions", done as u64);
        notify(app, webhook::Event::Action, format!("Kept {done} window(s) active"));
    }
}

/// Relaunches an account into the game it was disconnected from
fn rejoin(app: &Arc<App>, pid: u32, account: &str, place: i64, job: Option<String>) {
    {
        let mut rejoins = app.anti_afk().rejoins.lock().unwrap();
        let recent = rejoins.entry(account.to_string()).or_default();
        recent.retain(|t| t.elapsed() < Duration::from_secs(600));
        if recent.len() >= 3 {
            notify(app, webhook::Event::Error, format!("{account} keeps getting disconnected, stopped rejoining it for now"));
            return;
        }
        recent.push(Instant::now());
    }

    let Some(acc) = app.account(account) else { return };
    let same_server = app.afk_flag("RejoinSameServer");
    winutil::kill(pid);

    let app = app.clone();
    let account = account.to_string();
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        let tracker = app.ensure_tracker(&acc.username);
        let opts = JoinOptions { place_id: place, job_id: if same_server { job.unwrap_or_default() } else { String::new() }, privacy_ok: true, ..app.join_options() };
        match launcher::join(&app.api, &acc.security_token, &tracker, opts).await {
            JoinResult::Success => {
                add_stat(&app, "StatRejoins", 1);
                app.toast("success", format!("Rejoined {account}"));
                notify(&app, webhook::Event::Rejoined, format!("{account} rejoined place {place}"));
            }
            JoinResult::Error(message) => notify(&app, webhook::Event::Error, format!("{account}: {}", app.tr(&message))),
            JoinResult::NeedsPrivacy => notify(&app, webhook::Event::Error, format!("{account}: {}", app.tr(crate::text!("Could not rejoin")))),
        }
    });
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

    // stats while running
    {
        let mut last = state.last_tick.lock().unwrap();
        if running {
            if let Some(l) = *last {
                add_stat(app, "StatSeconds", l.elapsed().as_secs_f64().round() as u64);
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
                match log.state.disconnected {
                    Some(_) if t.disconnected_since.is_none() => {
                        t.disconnected_since = Some(Instant::now());
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
                        todo.push((*pid, account, place, log.state.job_id.clone()));
                    }
                }
            }
        }
        for (pid, account, place, job) in todo {
            rejoin(app, pid, &account, place, job);
        }
    }

    // mute / lighten the windows the user isn't looking at
    let mut foreground_pid = 0u32;
    unsafe {
        windows::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(foreground, Some(&mut foreground_pid));
    }
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
            let max_wait = Duration::from_secs(app.afk_number("MaxWaitSeconds"));
            match ready(user_safe, input::idle_seconds(), app.afk_number("IdleSeconds").max(1), waited, max_wait) {
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
                        let interval = Duration::from_secs(app.afk_number("Interval").clamp(30, ROBLOX_IDLE_KICK - 60));
                        *state.next_action.lock().unwrap() = Some(Instant::now() + interval);
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

    app.emit("antiafk", status(app));
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
    fn a_due_round_waits_for_the_user_then_gives_up() {
        let s = Duration::from_secs;
        // user safe off, or the user already idle: right away
        assert_eq!(ready(false, 0, 5, s(0), s(90)), Ready::Go);
        assert_eq!(ready(true, 5, 5, s(0), s(90)), Ready::Go);
        // the user is typing: wait, up to the limit, then try again later
        assert_eq!(ready(true, 1, 5, s(10), s(90)), Ready::Wait);
        assert_eq!(ready(true, 1, 5, s(91), s(90)), Ready::GiveUp);
    }
}
