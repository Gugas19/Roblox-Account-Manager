//! Window tools for many Roblox windows: grid, hide/show, see-through, close all.

use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::UI::WindowsAndMessaging::{
    GetWindowLongW, IsIconic, SetLayeredWindowAttributes, SetWindowLongW, SetWindowPos, ShowWindow, SystemParametersInfoW, GWL_EXSTYLE, HWND_TOP, LWA_ALPHA, SPI_GETWORKAREA, SWP_NOACTIVATE,
    SWP_SHOWWINDOW, SW_HIDE, SW_RESTORE, SW_SHOWNOACTIVATE, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WS_EX_LAYERED,
};

/// Columns and rows that fit `count` windows best on a screen of this shape
pub fn grid_shape(count: usize, width: i32, height: i32) -> (usize, usize) {
    if count == 0 {
        return (0, 0);
    }
    let mut best = (count, 1);
    let mut best_score = f64::MAX;
    for columns in 1..=count {
        let rows = count.div_ceil(columns);
        let cell = (width as f64 / columns as f64) / (height as f64 / rows as f64);
        // Roblox looks best close to 16:9, empty cells cost a little
        let score = (cell - 16.0 / 9.0).abs() + (columns * rows - count) as f64 * 0.15;
        if score < best_score {
            best_score = score;
            best = (columns, rows);
        }
    }
    best
}

/// The rectangles of a grid on the primary screen's work area
pub fn grid_cells(count: usize) -> Vec<(i32, i32, i32, i32)> {
    let mut area = RECT::default();
    unsafe {
        let _ = SystemParametersInfoW(SPI_GETWORKAREA, 0, Some(&mut area as *mut _ as *mut _), SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS(0));
    }
    let (w, h) = (area.right - area.left, area.bottom - area.top);
    let (columns, rows) = grid_shape(count, w, h);
    (0..count)
        .map(|i| {
            let (c, r) = ((i % columns.max(1)) as i32, (i / columns.max(1)) as i32);
            let (cw, ch) = (w / columns.max(1) as i32, h / rows.max(1) as i32);
            (area.left + c * cw, area.top + r * ch, cw, ch)
        })
        .collect()
}

pub fn arrange(windows: &[HWND]) {
    for (hwnd, (x, y, w, h)) in windows.iter().zip(grid_cells(windows.len())) {
        unsafe {
            if IsIconic(*hwnd).as_bool() {
                let _ = ShowWindow(*hwnd, SW_RESTORE);
            }
            let _ = SetWindowPos(*hwnd, HWND_TOP, x, y, w, h, SWP_NOACTIVATE | SWP_SHOWWINDOW);
        }
    }
}

pub fn set_hidden(windows: &[HWND], hidden: bool) {
    for hwnd in windows {
        unsafe {
            let _ = ShowWindow(*hwnd, if hidden { SW_HIDE } else { SW_SHOWNOACTIVATE });
        }
    }
}

/// See-through windows (alpha 0-255, 255 turns it off)
pub fn set_opacity(windows: &[HWND], alpha: u8) {
    for hwnd in windows {
        unsafe {
            let style = GetWindowLongW(*hwnd, GWL_EXSTYLE);
            if alpha == 255 {
                SetWindowLongW(*hwnd, GWL_EXSTYLE, style & !(WS_EX_LAYERED.0 as i32));
            } else {
                SetWindowLongW(*hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED.0 as i32);
                let _ = SetLayeredWindowAttributes(*hwnd, windows::Win32::Foundation::COLORREF(0), alpha, LWA_ALPHA);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grids() {
        assert_eq!(grid_shape(1, 1920, 1040), (1, 1));
        assert_eq!(grid_shape(2, 1920, 1040), (2, 1));
        assert_eq!(grid_shape(4, 1920, 1040), (2, 2));
        let (c, r) = grid_shape(7, 1920, 1040);
        assert!(c * r >= 7 && c * r <= 9);
        assert_eq!(grid_shape(0, 1920, 1040), (0, 0));
    }
}
