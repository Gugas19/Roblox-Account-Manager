//! Settings, theme, language and opening folders.

use super::*;

pub(super) fn settings_map(app: &App) -> Value {
    let s = app.settings.lock().unwrap();
    let mut out = serde_json::Map::new();
    for (name, section) in &s.ini.sections {
        out.insert(name.clone(), json!(section.properties.iter().map(|(k, p)| (k.clone(), json!(p.value))).collect::<serde_json::Map<String, Value>>()));
    }
    Value::Object(out)
}

#[tauri::command]
pub fn get_settings() -> Value {
    settings_map(&app())
}

/// Saves a setting and applies it right away, returns a message to show when there is one
/// Settings > Privacy: the app's window left out of screen captures and recordings, when on
pub fn apply_capture_exclusion(app: &App) {
    let on = app.setting_bool("General", "HideFromCaptures");
    if let Some(window) = app.handle.get().and_then(|h| tauri::Manager::get_webview_window(h, "main")) {
        if let Ok(hwnd) = window.hwnd() {
            winutil::exclude_from_capture(windows::Win32::Foundation::HWND(hwnd.0 as _), on);
        }
    }
}

/// Multi Roblox: on, working right now, and whether Roblox's tray process is what stops it
#[tauri::command]
pub fn multi_roblox_status() -> Value {
    let app = app();
    let enabled = app.setting_bool("General", "EnableMultiRbx");
    let working = crate::services::multi_roblox::working();
    let tray = enabled && !working && crate::services::multi_roblox::tray_process().is_some();
    json!({ "enabled": enabled, "working": working, "tray": tray })
}

/// Closes Roblox's background tray process (it has no game open), so Multi Roblox can start working
#[tauri::command]
pub fn close_roblox_tray() -> bool {
    let app = app();
    if let Some(pid) = crate::services::multi_roblox::tray_process() {
        log::info!("Closing Roblox's tray process {pid} for Multi Roblox");
        winutil::kill(pid);
    }
    for _ in 0..20 {
        if app.update_multi_roblox() {
            return true;
        }
        std::thread::sleep(std::time::Duration::from_millis(150));
    }
    false
}

#[tauri::command]
pub fn set_setting(section: String, key: String, value: String) -> R<Option<String>> {
    let app = app();
    app.set_setting(&section, &key, &value);

    match (section.as_str(), key.as_str()) {
        ("General", "EnableMultiRbx") => {
            if !app.update_multi_roblox() {
                return Ok(Some("WARNING: Roblox is currently running, multi roblox will only work once every Roblox window is closed.".into()));
            }
        }
        ("Developer", "EnableWebServer") | ("WebServer", "WebServerPort") | ("WebServer", "AllowExternalConnections") => {
            return webserver::restart(&app).map(|a| a.map(|a| format!("Web server: http://{a}/"))).map_err(e);
        }
        ("Watcher", _) => crate::services::watcher::apply(app.clone()),
        ("General", "HideFromCaptures") => apply_capture_exclusion(&app),
        ("General", "ShowPresence") | ("General", "DisableAgingAlert") => {
            let a = app.clone();
            tauri::async_runtime::spawn(async move { refresh_presence(&a).await });
        }
        _ => {}
    }
    Ok(None)
}

#[tauri::command]
pub fn set_startup(enabled: bool) -> R<()> {
    winutil::set_startup(enabled).map_err(e)
}

#[tauri::command]
pub fn set_language(code: String) -> Value {
    let app = app();
    let code = lang::resolve(&code);
    app.set_setting("General", "Language", code);
    json!({ "language": code, "table": lang::table(code) })
}

#[tauri::command]
pub fn set_theme(key: String, value: String) -> Value {
    let app = app();
    let mut theme = app.theme.lock().unwrap();
    theme.set(&key, &value);
    json!(theme.values())
}

#[tauri::command]
pub fn reset_theme() -> Value {
    let app = app();
    let mut theme = app.theme.lock().unwrap();
    theme.reset();
    json!(theme.values())
}

#[tauri::command]
pub fn pick_file(title: String) -> Option<String> {
    rfd::FileDialog::new().set_title(&title).add_filter("JSON", &["json"]).pick_file().map(|p| p.to_string_lossy().to_string())
}

#[tauri::command]
pub fn open_target(kind: String, value: Option<String>) -> R<()> {
    let app = app();
    let target = match kind.as_str() {
        "github" => "https://github.com/Gugas19/Roblox-Account-Manager".to_string(),
        "logs" => std::env::var("LOCALAPPDATA").unwrap_or_default() + "\\Roblox\\logs",
        "browser" => browser::browser_folder().to_string_lossy().to_string(),
        "data" => app.dir.to_string_lossy().to_string(),
        "profile" => format!("https://www.roblox.com/users/{}/profile", value.unwrap_or_default()),
        "game" => format!("https://www.roblox.com/games/{}", value.unwrap_or_default()),
        // ids only: nothing else gets into the address
        "asset" => format!("https://www.roblox.com/catalog/{}", value.unwrap_or_default().parse::<i64>().map_err(e)?),
        "group" => format!("https://www.roblox.com/communities/{}", value.unwrap_or_default().parse::<i64>().map_err(e)?),
        _ => return Err("unknown".into()),
    };
    winutil::shell_open(&target, None).map_err(e)
}

/// Watcher toggles need the processes list etc., keep it simple: on/off
#[tauri::command]
pub fn watcher_enable(enabled: bool) {
    let app = app();
    app.set_setting("Watcher", "Enabled", if enabled { "true" } else { "false" });
    crate::services::watcher::apply(app);
}
