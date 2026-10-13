//! Keys for the anti-AFK action, focusing a Roblox window for a moment and giving the focus back.

use std::time::Duration;
use windows::Win32::Foundation::HWND;
use windows::Win32::System::SystemInformation::GetTickCount;
use windows::Win32::System::Threading::{AttachThreadInput, GetCurrentThreadId};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    GetLastInputInfo, MapVirtualKeyW, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYBD_EVENT_FLAGS, KEYEVENTF_KEYUP, LASTINPUTINFO, MAPVK_VK_TO_VSC, VIRTUAL_KEY, VK_ESCAPE, VK_LEFT, VK_MENU,
    VK_RETURN, VK_RIGHT, VK_SPACE,
};
use windows::Win32::UI::Shell::{SHQueryUserNotificationState, QUNS_BUSY, QUNS_PRESENTATION_MODE, QUNS_RUNNING_D3D_FULL_SCREEN};
use windows::Win32::UI::WindowsAndMessaging::{
    BringWindowToTop, GetForegroundWindow, GetWindowThreadProcessId, IsIconic, SetForegroundWindow, ShowWindow, SW_MINIMIZE, SW_RESTORE,
};

/// What the anti-AFK cycle presses in each window
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    /// jump
    Space,
    /// a step forward and back
    WalkForwardBack,
    /// zoom in and out (I / O)
    Zoom,
    /// turn the camera left and right, doesn't move the character at all
    Camera,
}

impl Action {
    pub fn from_setting(value: &str) -> Action {
        match value {
            "ws" => Action::WalkForwardBack,
            "zoom" => Action::Zoom,
            "camera" => Action::Camera,
            _ => Action::Space,
        }
    }

    fn keys(self) -> &'static [u16] {
        match self {
            Action::Space => &[VK_SPACE.0],
            Action::WalkForwardBack => &[b'W' as u16, b'S' as u16],
            Action::Zoom => &[b'I' as u16, b'O' as u16],
            Action::Camera => &[VK_LEFT.0, VK_RIGHT.0],
        }
    }
}

fn key(vk: u16, up: bool) -> INPUT {
    let scan = unsafe { MapVirtualKeyW(vk as u32, MAPVK_VK_TO_VSC) } as u16;
    INPUT {
        r#type: INPUT_KEYBOARD,
        Anonymous: INPUT_0 { ki: KEYBDINPUT { wVk: VIRTUAL_KEY(vk), wScan: scan, dwFlags: if up { KEYEVENTF_KEYUP } else { KEYBD_EVENT_FLAGS(0) }, time: 0, dwExtraInfo: 0 } },
    }
}

pub fn tap(vk: u16, hold: Duration) {
    unsafe {
        SendInput(&[key(vk, false)], std::mem::size_of::<INPUT>() as i32);
        std::thread::sleep(hold);
        SendInput(&[key(vk, true)], std::mem::size_of::<INPUT>() as i32);
    }
}

pub fn perform(action: Action, repeat: u32) {
    for _ in 0..repeat.max(1) {
        for vk in action.keys() {
            tap(*vk, Duration::from_millis(60));
            std::thread::sleep(Duration::from_millis(60));
        }
    }
}

/// A full screen app or game (or presentation mode) is in front: switching windows would interrupt it
pub fn full_screen_in_front() -> bool {
    unsafe { SHQueryUserNotificationState().map(|s| s == QUNS_BUSY || s == QUNS_RUNNING_D3D_FULL_SCREEN || s == QUNS_PRESENTATION_MODE).unwrap_or(false) }
}

/// Esc, R, Enter: Roblox's reset character
pub fn reset_character() {
    for vk in [VK_ESCAPE.0, b'R' as u16, VK_RETURN.0] {
        tap(vk, Duration::from_millis(50));
        std::thread::sleep(Duration::from_millis(120));
    }
}

/// Seconds since the last keyboard or mouse input anywhere on the PC
pub fn idle_seconds() -> u64 {
    let mut info = LASTINPUTINFO { cbSize: std::mem::size_of::<LASTINPUTINFO>() as u32, dwTime: 0 };
    unsafe {
        if !GetLastInputInfo(&mut info).as_bool() {
            return 0;
        }
        (GetTickCount().wrapping_sub(info.dwTime) / 1000) as u64
    }
}

pub fn foreground() -> HWND {
    unsafe { GetForegroundWindow() }
}

/// Windows only lets the foreground's own thread give the focus away, so join its input for a moment
pub fn focus(hwnd: HWND) -> bool {
    unsafe {
        if IsIconic(hwnd).as_bool() {
            let _ = ShowWindow(hwnd, SW_RESTORE);
        }

        let current = GetForegroundWindow();
        let theirs = GetWindowThreadProcessId(current, None);
        let mine = GetCurrentThreadId();
        let attached = theirs != 0 && theirs != mine && AttachThreadInput(mine, theirs, true).as_bool();

        let _ = BringWindowToTop(hwnd);
        let mut ok = SetForegroundWindow(hwnd).as_bool();

        if attached {
            let _ = AttachThreadInput(mine, theirs, false);
        }

        if !ok || GetForegroundWindow() != hwnd {
            // last resort: an Alt tap counts as user input and unlocks SetForegroundWindow
            tap(VK_MENU.0, Duration::from_millis(10));
            ok = SetForegroundWindow(hwnd).as_bool();
        }

        std::thread::sleep(Duration::from_millis(150));
        ok && GetForegroundWindow() == hwnd
    }
}

pub fn minimize(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_MINIMIZE);
    }
}

pub fn was_minimized(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd).as_bool() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn actions_from_settings() {
        assert_eq!(Action::from_setting("zoom"), Action::Zoom);
        assert_eq!(Action::from_setting("anything"), Action::Space);
        assert_eq!(Action::from_setting("camera").keys(), &[VK_LEFT.0, VK_RIGHT.0]);
    }
}
