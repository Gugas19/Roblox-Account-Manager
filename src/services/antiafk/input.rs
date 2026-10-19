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
    BringWindowToTop, GetForegroundWindow, GetTopWindow, GetWindow, GetWindowLongW, GetWindowThreadProcessId, IsIconic, IsWindow, IsWindowVisible, SetForegroundWindow, SetWindowPos, ShowWindow,
    GWL_EXSTYLE, GW_HWNDNEXT, HWND_TOP, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOOWNERZORDER, SWP_NOSIZE, SW_RESTORE, SW_SHOWMINNOACTIVE, WS_EX_TOPMOST,
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

/// Minimizes without handing the focus to some other window
pub fn minimize(hwnd: HWND) {
    unsafe {
        let _ = ShowWindow(hwnd, SW_SHOWMINNOACTIVE);
    }
}

pub fn was_minimized(hwnd: HWND) -> bool {
    unsafe { IsIconic(hwnd).as_bool() }
}

/// The windows on screen (not minimized, not always on top) from the top of the Z order down
pub fn z_order() -> Vec<HWND> {
    let mut list = Vec::new();
    unsafe {
        let mut next = GetTopWindow(HWND::default()).ok();
        while let Some(hwnd) = next.filter(|h| !h.0.is_null()) {
            let topmost = GetWindowLongW(hwnd, GWL_EXSTYLE) as u32 & WS_EX_TOPMOST.0 != 0;
            if IsWindowVisible(hwnd).as_bool() && !IsIconic(hwnd).as_bool() && !topmost {
                list.push(hwnd);
            }
            next = GetWindow(hwnd, GW_HWNDNEXT).ok();
        }
    }
    list
}

/// Where each moved window goes back to: right under the window that was above it (`None` = the top).
/// Top down, so a moved window above is already back in place when the one under it follows it.
fn restack_plan<T: Copy + PartialEq>(before: &[T], moved: &[T], alive: impl Fn(T) -> bool) -> Vec<(T, Option<T>)> {
    let mut plan = Vec::new();
    for (i, w) in before.iter().enumerate() {
        if moved.contains(w) && alive(*w) {
            plan.push((*w, before[..i].iter().rev().copied().find(|a| alive(*a))));
        }
    }
    plan
}

/// Puts the windows focused for the anti-AFK action back in the order `before` had them, without focusing anything
pub fn restore_z_order(before: &[HWND], moved: &[HWND]) {
    let alive = |h: HWND| unsafe { IsWindow(h).as_bool() && IsWindowVisible(h).as_bool() && !IsIconic(h).as_bool() };
    for (hwnd, above) in restack_plan(before, moved, alive) {
        unsafe {
            let _ = SetWindowPos(hwnd, above.unwrap_or(HWND_TOP), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE | SWP_NOOWNERZORDER);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restack_puts_each_window_under_the_one_above_it() {
        // A B C D, B and D were focused; C is gone
        let plan = restack_plan(&['A', 'B', 'C', 'D'], &['B', 'D'], |w| w != 'C');
        assert_eq!(plan, vec![('B', Some('A')), ('D', Some('B'))]);
        assert_eq!(restack_plan(&['A', 'B'], &['A'], |_| true), vec![('A', None)]);
    }

    /// Real windows (off screen, never focused): one moved to the top goes back under the others
    #[test]
    fn z_order_comes_back() {
        use windows::core::w;
        use windows::Win32::UI::WindowsAndMessaging::{CreateWindowExW, DestroyWindow, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE};
        let make = || unsafe { CreateWindowExW(WS_EX_NOACTIVATE | WS_EX_TOOLWINDOW, w!("STATIC"), w!("ram z test"), WS_POPUP | WS_VISIBLE, -30000, -30000, 10, 10, HWND::default(), None, None, None).unwrap() };
        let (a, b, c) = (make(), make(), make());
        let order = |list: &[HWND]| [a, b, c].map(|w| list.iter().position(|h| *h == w).unwrap());
        let before = z_order();
        unsafe {
            let _ = SetWindowPos(c, HWND_TOP, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
        }
        let moved = order(&z_order());
        assert!(moved[2] < moved[0] && moved[2] < moved[1]);
        restore_z_order(&before, &[c]);
        let after = order(&z_order());
        let first = order(&before);
        for (x, y) in [(0, 1), (1, 2), (0, 2)] {
            assert_eq!(first[x] < first[y], after[x] < after[y]);
        }
        unsafe {
            for h in [a, b, c] {
                let _ = DestroyWindow(h);
            }
        }
    }

    #[test]
    fn actions_from_settings() {
        assert_eq!(Action::from_setting("zoom"), Action::Zoom);
        assert_eq!(Action::from_setting("anything"), Action::Space);
        assert_eq!(Action::from_setting("camera").keys(), &[VK_LEFT.0, VK_RIGHT.0]);
    }
}
