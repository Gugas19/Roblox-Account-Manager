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

pub fn run() {
    // the app keeps its files next to the exe, like the C# version
    let dir = std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())).unwrap_or_else(|| std::env::current_dir().unwrap_or_default());
    let _ = std::env::set_current_dir(&dir);

    // started by an update: the previous version is quitting, give it a moment to let go of the lock
    let after_update = std::env::args().any(|a| a == "--updated");
    let mut instance = NamedMutex::acquire(INSTANCE_MUTEX);
    for _ in 0..if after_update { 50 } else { 0 } {
        if matches!(instance, Ok((_, true))) {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(200));
        instance = NamedMutex::acquire(INSTANCE_MUTEX);
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
    if services::updater::install_staged(&dir) {
        return;
    }
    services::updater::clean_old(dir.clone());

    let state = app::init(dir);
    win::fix_startup_path();

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
                let _ = window.show();
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
            commands::accounts::move_to_group,
            commands::accounts::reorder,
            commands::accounts::sort_alphabetically,
            commands::accounts::save_place,
            commands::accounts::clear_saved_places,
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
            commands::joining::place_name,
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
        let _ = window.show();
        let _ = window.unminimize();
        let _ = window.set_focus();
    }
}
