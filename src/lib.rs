//! Roblox Account Manager.
//!
//! - `core`: file formats shared with the C# version (settings, accounts, encryption, languages)
//! - `roblox`: the Roblox web APIs
//! - `platform`: Windows helpers (processes, windows, mutexes, ping, log file)
//! - `services`: launching Roblox, the browser, the local web API, the Roblox watcher
//! - `commands`: what the interface (ui/) can ask for
//! - `cli`: the command line (ram-cli.exe), runs commands inside the open app or on its own
//! - `app`: the state they share

pub mod app;
pub mod cli;
pub mod commands;
pub mod core;
pub mod platform;
pub mod roblox;
pub mod services;

use platform::win::{self, NamedMutex};
use std::sync::Arc;
use tauri::menu::{Menu, MenuItem};
use tauri::tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent};
use tauri::{Manager, WindowEvent};

/// Same name as the C# version, so the two never run at the same time on the same files
pub(crate) const INSTANCE_MUTEX: &str = "{93b3858f-3dac-4dc0-99cb-0476efc5adce}";

/// A copy run by the tests (a `ram-test-copy` file next to it) or a development build (cargo's target folder).
/// It runs next to the real app: a lock of its own, its own WebView2 data, and it never touches the Windows
/// startup entry.
pub(crate) fn is_test_copy(dir: &std::path::Path) -> bool {
    dir.join("ram-test-copy").exists() || dir.components().any(|c| c.as_os_str().eq_ignore_ascii_case("target"))
}

/// The single instance lock: shared by every real copy (and the old C# version, they use the same files),
/// one per folder for test copies
pub(crate) fn instance_lock(dir: &std::path::Path) -> String {
    if !is_test_copy(dir) {
        return INSTANCE_MUTEX.to_string();
    }
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(dir.to_string_lossy().to_lowercase().as_bytes());
    format!("{INSTANCE_MUTEX}-test-{}", hash[..8].iter().map(|b| format!("{b:02x}")).collect::<String>())
}

pub fn run() {
    // the app keeps its files next to the exe, like the C# version
    let dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let _ = std::env::set_current_dir(&dir);

    // started by an update: the previous version is quitting, give it a moment to be gone (its lock, and the web
    // server's port and command line pipe it lets go of only as it ends)
    let args: Vec<String> = std::env::args().collect();
    let after_update = args.iter().any(|a| a == "--updated");
    if let Some(pid) = args.iter().position(|a| a == "--after").and_then(|i| args.get(i + 1)).and_then(|p| p.parse().ok()) {
        win::wait_process_exit(pid, std::time::Duration::from_secs(10));
    }
    let lock = instance_lock(&dir);
    let mut instance = NamedMutex::acquire(&lock);
    for _ in 0..if after_update { 50 } else { 0 } {
        if matches!(instance, Ok((_, true))) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        instance = NamedMutex::acquire(&lock);
    }

    let _instance = match instance {
        Ok((mutex, true)) => mutex,
        _ => {
            // opened twice: show the window that's already open instead of an error
            if !cli::pipe::show_existing() {
                win::message_box(
                    "Roblox Account Manager is already open, maybe a copy from another folder or the old version. Close it first.",
                    "Roblox Account Manager",
                );
            }
            return;
        }
    };

    platform::logging::init(&dir, after_update);
    log::info!("Roblox Account Manager {} started in {}", env!("CARGO_PKG_VERSION"), dir.display());
    if after_update {
        log::info!("Started after an update");
    }

    // an update downloaded earlier (and verified again now) replaces this version before anything starts
    if services::updater::install_staged_at_start(&dir) {
        return;
    }
    services::updater::clean_old(dir.clone());
    std::thread::spawn(services::browser::clean_stale_profiles);

    let test_copy = is_test_copy(&dir);
    if test_copy {
        // its own browser data: two apps on the same WebView2 data folder get in each other's way
        std::env::set_var("WEBVIEW2_USER_DATA_FOLDER", dir.join("webview-data"));
    }

    let state = app::init(dir);
    // Roblox ignores the frame rate flag now: what this app wrote there goes
    let unlock_fps_was_on = state.settings.lock().unwrap().unlock_fps_was_on;
    std::thread::spawn(move || services::launcher::remove_dead_fps_flag(unlock_fps_was_on));
    if !test_copy {
        win::fix_startup_path();
    }

    if let Err(e) = services::webserver::restart(&state) {
        log::error!("{e}");
    }
    services::watcher::apply(state.clone());
    services::antiafk::spawn(state.clone());
    cli::pipe::serve(state.clone());
    services::updater::spawn(state.clone());

    let setup_state = state.clone();
    let close_state = state.clone();

    tauri::Builder::default()
        .setup(move |tauri_app| {
            let _ = setup_state.handle.set(tauri_app.handle().clone());
            if let Some(window) = tauri_app.get_webview_window("main") {
                show_window(&window);
            }
            tray(tauri_app, setup_state.clone())?;
            Ok(())
        })
        .on_window_event(move |window, event| {
            if let WindowEvent::CloseRequested { api, .. } = event {
                // with "minimize to tray" the app keeps running (Anti-AFK, web API) behind the tray icon
                if close_state.setting_bool("General", "CloseToTray") {
                    api.prevent_close();
                    let _ = window.hide();
                } else {
                    services::antiafk::restore_all(&close_state);
                }
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::antiafk::antiafk_status,
            commands::antiafk::antiafk_start,
            commands::antiafk::antiafk_stop,
            commands::antiafk::antiafk_now,
            commands::antiafk::antiafk_tool,
            commands::antiafk::antiafk_test_webhook,
            commands::antiafk::antiafk_reset_stats,
            commands::startup::bootstrap,
            commands::startup::accounts_ready,
            commands::protection::choose_protection,
            commands::protection::unlock,
            commands::protection::check_password,
            commands::protection::protection,
            commands::protection::load_backup,
            commands::accounts::list_accounts,
            commands::accounts::remove_accounts,
            commands::accounts::undo_remove,
            commands::accounts::set_alias,
            commands::accounts::set_description,
            commands::accounts::set_account_color,
            commands::accounts::set_cpu_limit,
            commands::accounts::rename_group,
            commands::backup::export_accounts,
            commands::backup::import_backup,
            commands::accounts::set_group_color,
            commands::accounts::move_to_group,
            commands::accounts::reorder,
            commands::accounts::sort_alphabetically,
            commands::accounts::save_place,
            commands::accounts::clear_saved_places,
            commands::accounts::clear_saved_passwords,
            commands::accounts::import_cookies,
            commands::accounts::account_fields,
            commands::accounts::set_field,
            commands::accounts::copy_text,
            commands::accounts::dump_details,
            commands::accounts::quick_login_info,
            commands::accounts::quick_login_confirm,
            commands::joining::join,
            commands::joining::cancel_launch,
            commands::joining::follow_check,
            commands::joining::follow,
            commands::joining::allow_private_servers,
            commands::joining::place_info,
            commands::joining::set_shuffle,
            commands::games::recent_games,
            commands::games::search_games,
            commands::games::favorites,
            commands::games::add_favorite,
            commands::games::rename_favorite,
            commands::games::remove_favorite,
            commands::games::universe_id,
            commands::games::universe_places,
            commands::servers::list_servers,
            commands::servers::stop_servers,
            commands::servers::join_job,
            commands::servers::load_regions,
            commands::servers::find_player,
            commands::avatar::outfits,
            commands::avatar::outfit_json,
            commands::avatar::wear,
            commands::avatar::purchase,
            commands::account_utils::change_password,
            commands::account_utils::change_email,
            commands::account_utils::set_display_name,
            commands::account_utils::add_friend,
            commands::account_utils::toggle_block,
            commands::account_utils::unlock_pin,
            commands::account_utils::set_follow_privacy,
            commands::account_utils::sign_out_others,
            commands::browsing::browser_status,
            commands::browsing::add_account_browser,
            commands::browsing::add_user_pass,
            commands::browsing::open_browser,
            commands::browsing::custom_browsers,
            commands::settings::get_settings,
            commands::settings::set_setting,
            commands::settings::set_startup,
            commands::settings::set_language,
            commands::settings::set_theme,
            commands::settings::reset_theme,
            commands::settings::pick_file,
            commands::settings::open_target,
            commands::settings::watcher_enable,
            commands::updates::update_status,
            commands::updates::update_check,
            commands::updates::update_notes,
            commands::updates::update_restart,
        ])
        .run(tauri::generate_context!())
        .expect("error while running Roblox Account Manager");
}

/// Tray icon: open the window, start/stop Anti-AFK, quit
fn tray(tauri_app: &tauri::App, state: Arc<app::App>) -> tauri::Result<()> {
    let show = MenuItem::with_id(tauri_app, "show", "Roblox Account Manager", true, None::<&str>)?;
    let afk = MenuItem::with_id(tauri_app, "afk", "Anti-AFK: start / stop", true, None::<&str>)?;
    let quit = MenuItem::with_id(tauri_app, "quit", "Quit", true, None::<&str>)?;
    let menu = Menu::with_items(tauri_app, &[&show, &afk, &quit])?;

    let menu_state = state.clone();
    TrayIconBuilder::with_id("main")
        .icon(tauri_app.default_window_icon().cloned().expect("icon"))
        .tooltip("Roblox Account Manager")
        .menu(&menu)
        .show_menu_on_left_click(false)
        .on_menu_event(move |handle, event| match event.id.as_ref() {
            "show" => show_main(handle),
            "afk" => {
                if services::antiafk::is_running(&menu_state) {
                    services::antiafk::stop(&menu_state, true);
                } else {
                    services::antiafk::start(&menu_state, text!("Started from the tray"));
                }
            }
            "quit" => {
                services::antiafk::restore_all(&menu_state);
                handle.exit(0);
            }
            _ => {}
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = event {
                show_main(tray.app_handle());
            }
        })
        .build(tauri_app)?;
    Ok(())
}

pub(crate) fn show_main(handle: &tauri::AppHandle) {
    if let Some(window) = handle.get_webview_window("main") {
        show_window(&window);
        if !own_folder_is_test_copy() {
            let _ = window.unminimize();
            let _ = window.set_focus();
        }
    }
}

fn own_folder_is_test_copy() -> bool {
    std::env::current_exe().ok().and_then(|p| p.parent().map(is_test_copy)).unwrap_or(false)
}

/// Shows the window. A test copy's window opens off screen and never takes the focus: the tests drive the page
/// directly, the person using the PC keeps working undisturbed.
fn show_window(window: &tauri::WebviewWindow) {
    if !own_folder_is_test_copy() {
        // the window doesn't ask for the focus by itself (tauri.conf.json): the real app takes it here
        let _ = window.show();
        let _ = window.set_focus();
        return;
    }
    if let Ok(hwnd) = window.hwnd() {
        use windows::Win32::UI::WindowsAndMessaging::{
            GetWindowLongW, SetLayeredWindowAttributes, SetWindowLongW, SetWindowPos, ShowWindow, GWL_EXSTYLE, HWND_BOTTOM, LWA_ALPHA, SWP_NOACTIVATE, SWP_NOMOVE, SWP_NOSIZE, SW_SHOWNOACTIVATE,
            WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW,
        };
        let hwnd = windows::Win32::Foundation::HWND(hwnd.0 as _);
        unsafe {
            // never activated, no taskbar button, fully see-through, behind every window. It stays where it is:
            // moved off screen, Windows gave it another monitor's scale and the page came out the wrong size.
            let style = GetWindowLongW(hwnd, GWL_EXSTYLE);
            SetWindowLongW(hwnd, GWL_EXSTYLE, style | (WS_EX_NOACTIVATE.0 | WS_EX_TOOLWINDOW.0 | WS_EX_LAYERED.0) as i32);
            let _ = SetLayeredWindowAttributes(hwnd, windows::Win32::Foundation::COLORREF(0), 0, LWA_ALPHA);
            let _ = SetWindowPos(hwnd, HWND_BOTTOM, 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_NOACTIVATE);
            let _ = ShowWindow(hwnd, SW_SHOWNOACTIVATE);
        }
    }
}
