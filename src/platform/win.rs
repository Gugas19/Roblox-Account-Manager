//! Small Windows helpers: Roblox processes and windows, the Multi Roblox mutex, opening links, the startup entry.

use std::ffi::OsStr;
use std::os::windows::ffi::OsStrExt;
use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use windows::core::PCWSTR;
use windows::Win32::Foundation::{CloseHandle, BOOL, ERROR_ALREADY_EXISTS, GetLastError, HANDLE, HWND, LPARAM, WAIT_ABANDONED, WAIT_OBJECT_0, WPARAM};
use windows::Win32::System::Threading::{CreateMutexW, GetExitCodeProcess, OpenProcess, ReleaseMutex, WaitForSingleObject, PROCESS_QUERY_LIMITED_INFORMATION, PROCESS_SYNCHRONIZE};
use windows::Win32::UI::Shell::{ShellExecuteExW, SEE_MASK_NOCLOSEPROCESS, SHELLEXECUTEINFOW};
use windows::Win32::UI::WindowsAndMessaging::{
    EnumWindows, GetClassNameW, GetForegroundWindow, GetWindow, GetWindowRect, GetWindowTextW, GetWindowThreadProcessId, IsWindowVisible, MoveWindow, PostMessageW, GW_OWNER, SW_SHOWNORMAL,
    WM_CLOSE,
};

pub fn wide(s: &str) -> Vec<u16> {
    OsStr::new(s).encode_wide().chain(std::iter::once(0)).collect()
}

#[derive(Debug, Clone)]
pub struct RobloxProcess {
    pub pid: u32,
    pub command_line: String,
    pub memory_mb: u64,
    pub seconds_running: u64,
}

impl RobloxProcess {
    /// The browser tracker id the process was launched with: "browsertrackerid:123456" in today's roblox-player: link,
    /// "-b 123456" in older launches (only the old form was read, so no window was ever matched to its account)
    pub fn tracker_id(&self) -> Option<String> {
        tracker_id_of(&self.command_line)
    }

    /// The private server's access code, when the process was launched into one (its place launcher URL asks for
    /// RequestPrivateGame): rejoining with it goes back to the same private server, like Roblox's own Reconnect
    pub fn private_access_code(&self) -> Option<String> {
        private_access_code_of(&self.command_line)
    }
}

fn tracker_id_of(command_line: &str) -> Option<String> {
    let re = regex::Regex::new(r"(?:browsertrackerid:|-b )(\d+)").unwrap();
    re.captures(command_line).map(|c| c[1].to_string())
}

fn private_access_code_of(command_line: &str) -> Option<String> {
    let url = urlencoding::decode(command_line).map(|s| s.into_owned()).unwrap_or_else(|_| command_line.to_string());
    if !url.contains("request=RequestPrivateGame") {
        return None;
    }
    let re = regex::Regex::new(r"[?&]accessCode=([0-9A-Za-z\-]+)").unwrap();
    re.captures(&url).map(|c| c[1].to_string())
}

pub fn roblox_processes() -> Vec<RobloxProcess> {
    let mut sys = System::new();
    sys.refresh_processes_specifics(ProcessesToUpdate::All, true, ProcessRefreshKind::new().with_cmd(UpdateKind::Always).with_memory());
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);

    sys.processes()
        .values()
        .filter(|p| p.name().to_string_lossy().eq_ignore_ascii_case("RobloxPlayerBeta.exe"))
        .map(|p| RobloxProcess {
            pid: p.pid().as_u32(),
            command_line: p.cmd().iter().map(|a| a.to_string_lossy().to_string()).collect::<Vec<_>>().join(" "),
            memory_mb: p.memory() / 1024 / 1024,
            seconds_running: now.saturating_sub(p.start_time()),
        })
        .collect()
}

/// The window class of the Roblox player's main window
const ROBLOX_WINDOW_CLASS: &str = "WINDOWSCLIENT";

/// What doesn't change while a Roblox process runs, read once (listing processes costs ~15-20 ms, this is
/// needed every second)
#[derive(Clone)]
pub struct RobloxInfo {
    pub tracker_id: Option<String>,
    pub private_access_code: Option<String>,
    /// unix seconds
    pub started: u64,
}

static ROBLOX_INFO: once_cell::sync::Lazy<std::sync::Mutex<std::collections::HashMap<u32, Option<RobloxInfo>>>> = once_cell::sync::Lazy::new(Default::default);

/// Every Roblox player main window (hidden ones too) with its process, found by the window class: ~0.5 ms
pub fn roblox_main_windows() -> Vec<(u32, HWND)> {
    unsafe extern "system" fn each(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let found = &mut *(lparam.0 as *mut Vec<(u32, HWND)>);
        let mut class = [0u16; 32];
        let n = GetClassNameW(hwnd, &mut class);
        if n > 0 && String::from_utf16_lossy(&class[..n as usize]) == ROBLOX_WINDOW_CLASS {
            let mut pid = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut pid));
            if !found.iter().any(|(p, _)| *p == pid) {
                found.push((pid, hwnd));
            }
        }
        BOOL(1)
    }

    let mut found: Vec<(u32, HWND)> = Vec::new();
    unsafe {
        let _ = EnumWindows(Some(each), LPARAM(&mut found as *mut _ as isize));
    }
    // only the Roblox player (read once per process), and forget the ones that are gone
    let mut cache = ROBLOX_INFO.lock().unwrap();
    cache.retain(|pid, _| found.iter().any(|(p, _)| p == pid));
    found.retain(|(pid, _)| cache.entry(*pid).or_insert_with(|| read_roblox_info(*pid)).is_some());
    found
}

/// The cached details of a Roblox process found by `roblox_main_windows`
pub fn roblox_info(pid: u32) -> Option<RobloxInfo> {
    ROBLOX_INFO.lock().unwrap().get(&pid).cloned().flatten()
}

fn read_roblox_info(pid: u32) -> Option<RobloxInfo> {
    let mut sys = System::new();
    let target = [Pid::from_u32(pid)];
    sys.refresh_processes_specifics(ProcessesToUpdate::Some(&target), true, ProcessRefreshKind::new().with_cmd(UpdateKind::Always));
    let p = sys.process(Pid::from_u32(pid))?;
    if !p.name().to_string_lossy().eq_ignore_ascii_case("RobloxPlayerBeta.exe") {
        return None;
    }
    let process = RobloxProcess { pid, command_line: p.cmd().iter().map(|a| a.to_string_lossy().to_string()).collect::<Vec<_>>().join(" "), memory_mb: 0, seconds_running: 0 };
    Some(RobloxInfo { tracker_id: process.tracker_id(), private_access_code: process.private_access_code(), started: p.start_time() })
}

/// Whether a process still runs (without listing every process)
pub fn process_alive(pid: u32) -> bool {
    const STILL_ACTIVE: u32 = 259;
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
        let mut code = 0u32;
        let alive = GetExitCodeProcess(handle, &mut code).is_ok() && code == STILL_ACTIVE;
        let _ = CloseHandle(handle);
        alive
    }
}

/// Waits (at most `limit`) for a process to end; at once when it's already gone
pub fn wait_process_exit(pid: u32, limit: std::time::Duration) {
    unsafe {
        let Ok(handle) = OpenProcess(PROCESS_SYNCHRONIZE, false, pid) else { return };
        WaitForSingleObject(handle, limit.as_millis().min(u32::MAX as u128) as u32);
        let _ = CloseHandle(handle);
    }
}

pub fn is_roblox_running() -> bool {
    !roblox_processes().is_empty()
}

pub fn kill(pid: u32) {
    let mut sys = System::new();
    sys.refresh_processes(ProcessesToUpdate::Some(&[Pid::from_u32(pid)]), true);
    if let Some(p) = sys.process(Pid::from_u32(pid)) {
        p.kill();
    }
}

/// The visible top level windows of a process
pub fn windows_of(pid: u32) -> Vec<HWND> {
    struct Search {
        pid: u32,
        found: Vec<HWND>,
    }

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = &mut *(lparam.0 as *mut Search);
        let mut window_pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
        if window_pid == search.pid && IsWindowVisible(hwnd).as_bool() && GetWindow(hwnd, GW_OWNER).map(|o| o.0.is_null()).unwrap_or(true) {
            search.found.push(hwnd);
        }
        BOOL(1)
    }

    let mut search = Search { pid, found: Vec::new() };
    unsafe {
        let _ = EnumWindows(Some(callback), LPARAM(&mut search as *mut Search as isize));
    }
    search.found
}

/// Every top level window of a process with a title, hidden ones too
pub fn all_windows_of(pid: u32) -> Vec<HWND> {
    struct Search {
        pid: u32,
        found: Vec<HWND>,
    }

    unsafe extern "system" fn callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = &mut *(lparam.0 as *mut Search);
        let mut window_pid = 0u32;
        GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
        if window_pid == search.pid && GetWindow(hwnd, GW_OWNER).map(|o| o.0.is_null()).unwrap_or(true) && !window_title(hwnd).is_empty() {
            search.found.push(hwnd);
        }
        BOOL(1)
    }

    let mut search = Search { pid, found: Vec::new() };
    unsafe {
        let _ = EnumWindows(Some(callback), LPARAM(&mut search as *mut Search as isize));
    }
    search.found
}

pub fn main_window(pid: u32) -> Option<HWND> {
    windows_of(pid).into_iter().next()
}

pub fn window_title(hwnd: HWND) -> String {
    let mut buffer = [0u16; 512];
    let len = unsafe { GetWindowTextW(hwnd, &mut buffer) };
    String::from_utf16_lossy(&buffer[..len.max(0) as usize])
}

pub fn foreground_window() -> HWND {
    unsafe { GetForegroundWindow() }
}

pub fn close_window(hwnd: HWND) {
    unsafe {
        let _ = PostMessageW(hwnd, WM_CLOSE, WPARAM(0), LPARAM(0));
    }
}

pub fn window_rect(hwnd: HWND) -> Option<(i32, i32, i32, i32)> {
    let mut rect = Default::default();
    unsafe { GetWindowRect(hwnd, &mut rect).ok()? };
    Some((rect.left, rect.top, rect.right - rect.left, rect.bottom - rect.top))
}

pub fn set_window_title(hwnd: HWND, title: &str) {
    let title = wide(title);
    unsafe {
        let _ = windows::Win32::UI::WindowsAndMessaging::SetWindowTextW(hwnd, PCWSTR(title.as_ptr()));
    }
}

/// Screen captures, recordings and screen sharing (OBS, Discord, the Snipping Tool) leave the window out (black or
/// missing), when on; Windows 10 2004 and later
pub fn exclude_from_capture(hwnd: HWND, on: bool) {
    use windows::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE, WDA_NONE};
    unsafe {
        let _ = SetWindowDisplayAffinity(hwnd, if on { WDA_EXCLUDEFROMCAPTURE } else { WDA_NONE });
    }
}

pub fn move_window(hwnd: HWND, x: i32, y: i32, width: i32, height: i32) {
    unsafe {
        let _ = MoveWindow(hwnd, x, y, width, height, true);
    }
}

/// Opens a file, folder, link or protocol (roblox-player:) like double clicking it, optionally waits for the program it started
/// Opens a file, folder, link or protocol. With `wait`, up to that long for the program it started to finish
pub fn shell_open(target: &str, wait: Option<std::time::Duration>) -> anyhow::Result<()> {
    let file = wide(target);
    let verb = wide("open");
    let mut info = SHELLEXECUTEINFOW {
        cbSize: std::mem::size_of::<SHELLEXECUTEINFOW>() as u32,
        fMask: SEE_MASK_NOCLOSEPROCESS,
        lpVerb: PCWSTR(verb.as_ptr()),
        lpFile: PCWSTR(file.as_ptr()),
        nShow: SW_SHOWNORMAL.0,
        ..Default::default()
    };

    unsafe {
        ShellExecuteExW(&mut info).map_err(|e| anyhow::anyhow!("{e}"))?;
        if !info.hProcess.is_invalid() && !info.hProcess.0.is_null() {
            if let Some(limit) = wait {
                WaitForSingleObject(info.hProcess, limit.as_millis().min(u32::MAX as u128) as u32);
            }
            let _ = CloseHandle(info.hProcess);
        }
    }
    Ok(())
}

/// A named mutex held for the app's lifetime
pub struct NamedMutex(HANDLE);

unsafe impl Send for NamedMutex {}
unsafe impl Sync for NamedMutex {}

impl NamedMutex {
    /// Returns the mutex and whether this process owns it (false when someone else already holds it)
    pub fn acquire(name: &str) -> anyhow::Result<(NamedMutex, bool)> {
        let name = wide(name);
        unsafe {
            let handle = CreateMutexW(None, true, PCWSTR(name.as_ptr())).map_err(|e| anyhow::anyhow!("{e}"))?;
            if GetLastError() != ERROR_ALREADY_EXISTS {
                return Ok((NamedMutex(handle), true));
            }
            let wait = WaitForSingleObject(handle, 0);
            Ok((NamedMutex(handle), wait == WAIT_OBJECT_0 || wait == WAIT_ABANDONED))
        }
    }
}

impl Drop for NamedMutex {
    fn drop(&mut self) {
        unsafe {
            let _ = ReleaseMutex(self.0);
            let _ = CloseHandle(self.0);
        }
    }
}

const RUN_KEY: &str = r"Software\Microsoft\Windows\CurrentVersion\Run";
const STARTUP_NAME: &str = "Roblox Account Manager";

pub fn startup_enabled() -> bool {
    winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey(RUN_KEY).and_then(|k| k.get_value::<String, _>(STARTUP_NAME)).is_ok()
}

pub fn set_startup(enabled: bool) -> anyhow::Result<()> {
    let (key, _) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).create_subkey(RUN_KEY)?;
    if enabled {
        key.set_value(STARTUP_NAME, &std::env::current_exe()?.to_string_lossy().to_string())?;
    } else {
        let _ = key.delete_value(STARTUP_NAME);
    }
    Ok(())
}

/// Keeps the startup entry pointing at this exe when the app was moved
pub fn fix_startup_path() {
    if let Ok(key) = winreg::RegKey::predef(winreg::enums::HKEY_CURRENT_USER).open_subkey_with_flags(RUN_KEY, winreg::enums::KEY_ALL_ACCESS) {
        if let (Ok(current), Ok(exe)) = (key.get_value::<String, _>(STARTUP_NAME), std::env::current_exe()) {
            let exe = exe.to_string_lossy().to_string();
            if current != exe {
                let _ = key.set_value(STARTUP_NAME, &exe);
            }
        }
    }
}

/// Roblox's version folder (from the roblox: protocol registration)
pub fn roblox_version_folder() -> Option<std::path::PathBuf> {
    let key = winreg::RegKey::predef(winreg::enums::HKEY_CLASSES_ROOT).open_subkey(r"roblox\DefaultIcon").ok()?;
    let value: String = key.get_value("").ok()?;
    let path = value.trim().trim_matches('"');
    let path = path.split(",").next().unwrap_or(path).trim_matches('"');
    std::path::Path::new(path).parent().map(|p| p.to_path_buf())
}

pub fn message_box(text: &str, title: &str) {
    use windows::Win32::UI::WindowsAndMessaging::{MessageBoxW, MB_ICONERROR, MB_OK};
    let text = wide(text);
    let title = wide(title);
    unsafe {
        MessageBoxW(None, PCWSTR(text.as_ptr()), PCWSTR(title.as_ptr()), MB_OK | MB_ICONERROR);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // shaped like real command lines, made-up values
    const FROM_THIS_APP: &str = r#""C:\Roblox\RobloxPlayerBeta.exe" roblox-player:1+launchmode:play+gameinfo:TICKET+launchtime:1+placelauncherurl:https%3A%2F%2Fassetgame.roblox.com%2Fgame%2FPlaceLauncher.ashx%3Frequest%3DRequestGameJob%26browserTrackerId%3D123456789012%26placeId%3D1818%26gameId%3Dabc+browsertrackerid:123456789012+robloxLocale:en_us"#;
    const PRIVATE_FROM_ROBLOX_APP: &str = r#""C:\Roblox\RobloxPlayerBeta.exe" roblox-player:1+launchmode:play+gameinfo:TICKET+launchtime:1+placelauncherurl:https%3A%2F%2Fwww.roblox.com%2FGame%2FPlaceLauncher.ashx%3Frequest%3DRequestPrivateGame%26browserTrackerId%3D1760000000000001%26placeId%3D1818%26accessCode%3D0a1b2c3d-5642-4eb7-9501-7078f7dcc7f0%26linkCode%3D%26joinAttemptId%3Dx+browsertrackerid:1760000000000001+robloxLocale:en_us"#;

    #[test]
    fn tracker_ids_in_both_forms() {
        assert_eq!(tracker_id_of(FROM_THIS_APP).as_deref(), Some("123456789012"));
        assert_eq!(tracker_id_of(PRIVATE_FROM_ROBLOX_APP).as_deref(), Some("1760000000000001"));
        assert_eq!(tracker_id_of(r#"RobloxPlayerBeta.exe --play -t TICKET -j https://x -b 987654321"#).as_deref(), Some("987654321"));
        assert_eq!(tracker_id_of("RobloxPlayerBeta.exe"), None);
    }

    #[test]
    fn private_servers_give_their_access_code() {
        assert_eq!(private_access_code_of(PRIVATE_FROM_ROBLOX_APP).as_deref(), Some("0a1b2c3d-5642-4eb7-9501-7078f7dcc7f0"));
        assert_eq!(private_access_code_of(FROM_THIS_APP), None);
    }
}