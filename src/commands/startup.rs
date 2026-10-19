//! Start up: what the interface needs first, background refreshes.

use super::*;

pub(super) static LOADED: once_cell::sync::OnceCell<LoadState> = once_cell::sync::OnceCell::new();

#[tauri::command]
pub fn bootstrap() -> Value {
    let app = app();
    let state = LOADED.get_or_init(|| app.store.lock().unwrap().load()).clone();
    let language = lang::resolve(&app.setting("General", "Language"));
    let multi_ok = app.update_multi_roblox();

    json!({
        "language": language,
        "table": lang::table(language),
        "languages": lang::LANGUAGES,
        "theme": app.theme.lock().unwrap().values(),
        "settings": settings_map(&app),
        "loadState": state,
        "multiRobloxOk": multi_ok,
        "browserInstalled": browser::installed_browser().is_some(),
        // the last game typed or joined, none on a new install
        "placeId": app.setting("General", "SavedPlaceId"),
        "jobId": app.setting("General", "SavedJobId"),
        "followUser": app.setting("General", "SavedFollowUser"),
        "version": env!("CARGO_PKG_VERSION"),
        "startup": winutil::startup_enabled(),
        "webServer": app.web_server.lock().unwrap().as_ref().map(|h| h.address.clone()),
    })
}

/// Called once the accounts are available (after the password or protection screens)
#[tauri::command]
pub fn accounts_ready() -> Vec<AccountView> {
    let app = app();
    app.ready.store(true, Ordering::SeqCst);
    start_background(app.clone());
    app.views()
}

pub(super) static BACKGROUND: std::sync::Once = std::sync::Once::new();

pub(super) fn start_background(app: Arc<App>) {
    BACKGROUND.call_once(|| {
        // browser download, presence, cookie refresh
        let a = app.clone();
        tauri::async_runtime::spawn(async move { ensure_browser(&a).await.ok() });

        let a = app.clone();
        tauri::async_runtime::spawn(async move {
            loop {
                if a.setting_bool("General", "ShowPresence") {
                    refresh_presence(&a).await;
                }
                let minutes = a.setting_i64("General", "PresenceUpdateRate", 5).max(1) as u64;
                tokio::time::sleep(Duration::from_secs(minutes * 60)).await;
            }
        });

        // Settings > General > When the app starts: an account joins a game, once
        let a = app.clone();
        tauri::async_runtime::spawn(async move {
            let (name, place) = (a.setting("General", "StartupAccount"), a.setting("General", "StartupPlace"));
            let (Some(account), Ok(place_id)) = (a.account(&name), place.trim().parse::<i64>()) else { return };
            tokio::time::sleep(Duration::from_secs(3)).await;
            log::info!("{} joins place {place_id}, as set for when the app starts", account.username);
            let opts = JoinOptions { place_id, privacy_ok: true, ..a.join_options() };
            if let JoinResult::Error(message) = super::joining::join_one(&a, &account, opts).await {
                a.toast("error", format!("{}: {}", account.username, a.tr(&message)));
            }
        });

        let a = app;
        tauri::async_runtime::spawn(async move {
            loop {
                tokio::time::sleep(Duration::from_secs(5 * 60)).await;
                if a.setting_bool("General", "AutoCookieRefresh") {
                    refresh_old_cookies(&a).await;
                }
            }
        });
    });
}

/// How many accounts in a game are asked for the game's name each time (one request each)
const PLACE_NAMES_AT_ONCE: usize = 25;

pub(super) async fn refresh_presence(app: &App) {
    let ids: Vec<i64> = app.store.lock().unwrap().accounts.iter().map(|a| a.user_id).filter(|i| *i > 0).collect();
    if let Ok(mut list) = app.api.presence(&ids).await {
        // the accounts in a game, asked as themselves, for the game's name
        for p in list.iter_mut().filter(|p| p.kind == 2 && p.last_location.as_deref().unwrap_or("").is_empty()).take(PLACE_NAMES_AT_ONCE) {
            let Some(account) = app.store.lock().unwrap().accounts.iter().find(|a| a.user_id == p.user_id).cloned() else { continue };
            let Ok(csrf) = app.csrf(&account).await else { continue };
            if let Ok(Some(own)) = app.api.own_presence(account.user_id, &account.security_token, &csrf).await {
                if own.kind == p.kind {
                    *p = own;
                }
            }
        }
        let names: std::collections::HashMap<i64, String> = app.store.lock().unwrap().accounts.iter().map(|a| (a.user_id, a.username.clone())).collect();
        let mut events = Vec::new();
        let mut sessions = app.sessions.lock().unwrap();
        for p in list {
            let id = p.user_id;
            let entry = sessions.entry(id).or_default();
            if let Some(name) = names.get(&id) {
                events.extend(crate::services::history::presence_events(name, entry.presence.as_ref(), &p));
            }
            entry.presence = Some(p);
        }
        drop(sessions);
        crate::services::history::record(app, events);
    }
    app.accounts_changed();
}

/// Accounts unused for 20 days get a new cookie every 7 days (port of RefreshOldCookies)
pub(super) async fn refresh_old_cookies(app: &App) {
    let candidates: Vec<Account> = app
        .store
        .lock()
        .unwrap()
        .accounts
        .iter()
        .filter(|a| a.field("NoCookieRefresh") != "true" && a.last_use.days_ago() > 20.0 && a.last_attempted_refresh.days_ago() >= 7.0)
        .cloned()
        .collect();

    for account in candidates {
        log::info!("Attempting to refresh {}", account.username);
        app.update_account(&account.username, |a| a.last_attempted_refresh = crate::core::netdate::NetDate::now());
        if let Ok(csrf) = app.csrf(&account).await {
            if app.pin_ok(&account).await.is_ok() {
                if let Ok(r) = app.api.sign_out_other_sessions(&account.security_token, &csrf).await {
                    if let Some(cookie) = r.cookie {
                        app.update_account(&account.username, |a| a.security_token = cookie);
                    }
                }
            }
        }
        app.save_accounts(true);
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}
