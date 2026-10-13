//! Shared app state.

use crate::core::accounts::{Account, AccountStore};
use crate::roblox::{Api, Presence};
use crate::core::settings::{Settings, Theme};
use crate::platform::win::NamedMutex;
use crate::services::launcher::JoinOptions;
use std::sync::atomic::Ordering;
use once_cell::sync::OnceCell;
use serde::Serialize;
use serde_json::Value;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tauri::{AppHandle, Emitter};

/// What's known about an account while the app runs (never saved)
#[derive(Default, Clone)]
pub struct Session {
    pub csrf: String,
    pub pin_unlocked_until: Option<Instant>,
    pub presence: Option<Presence>,
}

pub struct App {
    pub dir: PathBuf,
    pub api: Api,
    pub store: Mutex<AccountStore>,
    pub settings: Mutex<Settings>,
    pub theme: Mutex<Theme>,
    pub sessions: Mutex<HashMap<i64, Session>>,
    pub multi_roblox: Mutex<Option<NamedMutex>>,
    pub web_server: Mutex<Option<crate::services::webserver::Handle>>,
    pub shuffle: AtomicBool,
    /// bumped to cancel a running "launch several accounts"
    pub launch_generation: AtomicU64,
    pub launch_next: AtomicBool,
    pub servers_generation: AtomicU64,
    pub downloading: AtomicBool,
    pub download_progress: Mutex<f32>,
    pub recent_games: Mutex<Vec<Value>>,
    pub favorites: Mutex<Vec<Value>>,
    /// the last server list, for the SetRecommendedServer web method
    pub last_servers: Mutex<(i64, Vec<String>)>,
    pub attempted_joins: Mutex<Vec<String>>,
    /// the accounts removed last, with their positions, for "Undo"
    pub removed: Mutex<Vec<(usize, Account)>>,
    pub watcher: crate::services::watcher::State,
    pub antiafk: crate::services::antiafk::State,
    /// the accounts are loaded (false while the window asks for the password)
    pub ready: AtomicBool,
    pub update: Mutex<crate::services::updater::Status>,
    pub handle: OnceCell<AppHandle>,
}

static APP: OnceCell<Arc<App>> = OnceCell::new();

pub fn app() -> Arc<App> {
    APP.get().expect("app not started").clone()
}

pub fn init(dir: PathBuf) -> Arc<App> {
    let app = Arc::new(App {
        api: Api::new(),
        store: Mutex::new(AccountStore::new(&dir)),
        settings: Mutex::new(Settings::load(&dir)),
        theme: Mutex::new(Theme::load(&dir)),
        sessions: Mutex::new(HashMap::new()),
        multi_roblox: Mutex::new(None),
        web_server: Mutex::new(None),
        shuffle: AtomicBool::new(false),
        launch_generation: AtomicU64::new(0),
        launch_next: AtomicBool::new(false),
        servers_generation: AtomicU64::new(0),
        downloading: AtomicBool::new(false),
        download_progress: Mutex::new(0.0),
        recent_games: Mutex::new(load_json_list(&dir.join("RecentGames.json"))),
        favorites: Mutex::new(load_json_list(&dir.join("FavoriteGames.json"))),
        last_servers: Mutex::new((0, Vec::new())),
        attempted_joins: Mutex::new(Vec::new()),
        removed: Mutex::new(Vec::new()),
        watcher: crate::services::watcher::State::default(),
        antiafk: crate::services::antiafk::State::default(),
        ready: AtomicBool::new(false),
        update: Mutex::new(Default::default()),
        handle: OnceCell::new(),
        dir,
    });
    let _ = APP.set(app.clone());
    app
}

fn load_json_list(path: &std::path::Path) -> Vec<Value> {
    std::fs::read_to_string(path).ok().and_then(|t| serde_json::from_str::<Vec<Value>>(t.trim_start_matches('\u{feff}')).ok()).unwrap_or_default()
}

/// Account row for the interface (no cookies or passwords)
#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountView {
    pub username: String,
    pub user_id: i64,
    pub alias: String,
    pub description: String,
    pub group: String,
    pub group_title: String,
    pub days_unused: f64,
    pub presence: i64,
    pub saved_place_id: String,
    pub saved_job_id: String,
    pub has_password: bool,
    /// the account's own color, "#rrggbb"
    pub color: Option<String>,
    /// its group's color
    pub group_color: Option<String>,
    /// its own frame rate limit
    pub fps_limit: Option<i64>,
}

/// A color as the app keeps it: "#rrggbb", anything else is refused
pub fn valid_color(color: &str) -> Option<String> {
    let c = color.trim();
    (c.len() == 7 && c.starts_with('#') && c[1..].chars().all(|ch| ch.is_ascii_hexdigit())).then(|| c.to_ascii_lowercase())
}

impl App {
    pub fn emit<S: Serialize + Clone>(&self, event: &str, payload: S) {
        if let Some(h) = self.handle.get() {
            let _ = h.emit(event, payload);
        }
    }

    pub fn toast(&self, kind: &str, message: impl Into<String>) {
        self.emit("toast", serde_json::json!({ "kind": kind, "message": message.into() }));
    }

    /// Group colors, kept in RAMSettings.ini ([Groups] Colors) as JSON so any group name is safe
    pub fn group_colors(&self) -> std::collections::BTreeMap<String, String> {
        serde_json::from_str(&self.setting("Groups", "Colors")).unwrap_or_default()
    }

    pub fn set_group_color(&self, group: &str, color: Option<String>) {
        let mut colors = self.group_colors();
        match color {
            Some(c) => colors.insert(group.to_string(), c),
            None => colors.remove(group),
        };
        self.set_setting("Groups", "Colors", &serde_json::to_string(&colors).unwrap_or_default());
    }

    pub fn views(&self) -> Vec<AccountView> {
        let group_colors = self.group_colors();
        let store = self.store.lock().unwrap();
        let sessions = self.sessions.lock().unwrap();
        store
            .accounts
            .iter()
            .map(|a| AccountView {
                username: a.username.clone(),
                user_id: a.user_id,
                alias: a.alias.clone(),
                description: a.description.clone(),
                group: a.group.clone(),
                group_title: Account::group_title(&a.group),
                days_unused: if a.last_use.days_ago().is_finite() { a.last_use.days_ago() } else { 99999.0 },
                presence: sessions.get(&a.user_id).and_then(|s| s.presence.as_ref()).map(|p| p.kind).unwrap_or(0),
                saved_place_id: a.field("SavedPlaceId").to_string(),
                saved_job_id: a.field("SavedJobId").to_string(),
                has_password: !a.password.is_empty(),
                color: valid_color(a.field("Color")),
                group_color: group_colors.get(&a.group).and_then(|c| valid_color(c)),
                fps_limit: crate::services::launcher::fps_limit_of(a),
            })
            .collect()
    }

    pub fn accounts_changed(&self) {
        self.emit("accounts", self.views());
    }

    /// Saves (respecting the C# rules) and tells the interface
    pub fn save_accounts(&self, force: bool) {
        if let Err(e) = self.store.lock().unwrap().save(force, false) {
            log::error!("Saving accounts failed: {e}");
            self.toast("error", format!("Failed to save accounts: {e}"));
        }
        self.accounts_changed();
    }

    pub fn account(&self, username: &str) -> Option<Account> {
        self.store.lock().unwrap().accounts.iter().find(|a| a.username == username || a.user_id.to_string() == username).cloned()
    }

    pub fn accounts_named(&self, names: &[String]) -> Vec<Account> {
        let store = self.store.lock().unwrap();
        names.iter().filter_map(|n| store.accounts.iter().find(|a| &a.username == n).cloned()).collect()
    }

    pub fn update_account<F: FnOnce(&mut Account)>(&self, username: &str, change: F) -> bool {
        let mut store = self.store.lock().unwrap();
        match store.accounts.iter_mut().find(|a| a.username == username) {
            Some(a) => {
                change(a);
                true
            }
            None => false,
        }
    }

    /// The most recently used account, for requests that need any valid session
    pub fn any_token(&self) -> Option<String> {
        let store = self.store.lock().unwrap();
        store.accounts.iter().max_by(|a, b| a.last_use.days_ago().partial_cmp(&b.last_use.days_ago()).unwrap_or(std::cmp::Ordering::Equal).reverse()).map(|a| a.security_token.clone())
    }

    /// A fresh CSRF token, also marks the account as used (like the C# GetCSRFToken)
    pub async fn csrf(&self, account: &Account) -> anyhow::Result<String> {
        match self.api.csrf(&account.security_token).await? {
            Some(token) => {
                self.sessions.lock().unwrap().entry(account.user_id).or_default().csrf = token.clone();
                self.update_account(&account.username, |a| a.last_use = crate::core::netdate::NetDate::now());
                self.save_accounts(false);
                Ok(token)
            }
            None => anyhow::bail!("Invalid Account Session!"),
        }
    }

    /// Port of Account.CheckPin
    pub async fn pin_ok(&self, account: &Account) -> anyhow::Result<()> {
        self.csrf(account).await?;
        if let Some(until) = self.sessions.lock().unwrap().get(&account.user_id).and_then(|s| s.pin_unlocked_until) {
            if Instant::now() < until {
                return Ok(());
            }
        }
        if self.api.pin_unlocked(&account.security_token).await? {
            return Ok(());
        }
        anyhow::bail!("Pin required!")
    }

    pub fn setting(&self, section: &str, key: &str) -> String {
        self.settings.lock().unwrap().get(section, key)
    }

    pub fn setting_bool(&self, section: &str, key: &str) -> bool {
        self.settings.lock().unwrap().get_bool(section, key)
    }

    pub fn setting_i64(&self, section: &str, key: &str, default: i64) -> i64 {
        self.settings.lock().unwrap().get_i64(section, key, default)
    }

    pub fn set_setting(&self, section: &str, key: &str, value: &str) {
        let mut s = self.settings.lock().unwrap();
        s.set(section, key, value);
        s.save();
    }

    /// Multi Roblox on/off, false when Roblox already runs (it works once every Roblox window is closed)
    pub fn update_multi_roblox(&self) -> bool {
        let enabled = self.setting_bool("General", "EnableMultiRbx");
        let mut held = self.multi_roblox.lock().unwrap();

        if enabled && held.is_none() {
            match NamedMutex::acquire("ROBLOX_singletonMutex") {
                Ok((mutex, owned)) => {
                    *held = Some(mutex);
                    return owned;
                }
                Err(_) => return false,
            }
        } else if !enabled {
            *held = None;
        }
        true
    }

    /// Adds (or updates) the account behind a cookie, like AccountManager.AddAccount
    pub async fn add_account(&self, token: &str, password: &str) -> anyhow::Result<Account> {
        let token = token.trim();
        let user = self.api.authenticated_user(token).await?;
        let password = if self.setting_bool("General", "SavePasswords") { password.to_string() } else { String::new() };

        let account = {
            let mut store = self.store.lock().unwrap();
            if let Some(existing) = store.accounts.iter_mut().find(|a| a.user_id == user.id) {
                existing.security_token = token.to_string();
                if !password.is_empty() {
                    existing.set_password(&password);
                }
                existing.last_use = crate::core::netdate::NetDate::now();
                existing.valid = true;
                existing.clone()
            } else {
                let mut account = Account { valid: true, security_token: token.to_string(), username: user.name, user_id: user.id, last_use: crate::core::netdate::NetDate::now(), group: "Default".into(), ..Default::default() };
                account.set_password(&password);
                store.accounts.push(account.clone());
                account
            }
        };

        if let Err(e) = self.store.lock().unwrap().save(true, false) {
            log::error!("Saving accounts failed: {e}");
        }
        self.accounts_changed();
        Ok(account)
    }

    /// English text in the app's language, for what doesn't go through the interface (Discord messages)
    pub fn tr(&self, text: &str) -> String {
        crate::core::lang::translate(crate::core::lang::resolve(&self.setting("General", "Language")), text)
    }

    pub fn anti_afk(&self) -> &crate::services::antiafk::State {
        &self.antiafk
    }

    /// Launch settings shared by every join
    pub fn join_options(&self) -> JoinOptions {
        let s = self.settings.lock().unwrap();
        JoinOptions {
            shuffle: self.shuffle.load(Ordering::SeqCst),
            shuffle_lowest: s.get_bool("General", "ShuffleChoosesLowestServer"),
            shuffle_pages: s.get_i64("General", "ShufflePageCount", 5),
            auto_close_last: s.get_bool("General", "AutoCloseLastProcess"),
            unlock_fps: s.get_bool("General", "UnlockFPS"),
            max_fps: s.get_i64("General", "MaxFPSValue", 120),
            custom_client_settings: Some(s.get("General", "CustomClientSettings")).filter(|c| !c.is_empty()),
            ..Default::default()
        }
    }

    /// The account's browser tracker id, created (and saved) the first time
    pub fn ensure_tracker(&self, username: &str) -> String {
        let mut id = String::new();
        self.update_account(username, |a| {
            if a.browser_tracker_id.as_deref().unwrap_or("").is_empty() {
                a.browser_tracker_id = Some(crate::services::launcher::new_tracker_id());
            }
            id = a.browser_tracker_id.clone().unwrap_or_default();
        });
        self.save_accounts(false);
        id
    }

    pub fn save_recent_games(&self) {
        let list = self.recent_games.lock().unwrap().clone();
        let _ = std::fs::write(self.dir.join("RecentGames.json"), serde_json::to_string(&list).unwrap_or_default());
    }

    pub fn save_favorites(&self) {
        let list = self.favorites.lock().unwrap().clone();
        if list.is_empty() && !self.dir.join("FavoriteGames.json").exists() {
            return;
        }
        let _ = std::fs::write(self.dir.join("FavoriteGames.json"), serde_json::to_string(&list).unwrap_or_default());
    }
}

#[cfg(test)]
mod color_tests {
    use super::valid_color;

    #[test]
    fn only_rrggbb_colors_are_kept() {
        assert_eq!(valid_color("#EF4444"), Some("#ef4444".into()));
        assert_eq!(valid_color(" #22c55e "), Some("#22c55e".into()));
        for bad in ["", "red", "#fff", "#12345g", "ef4444", "#ef44445", "url(x)", "#ef4444;background:red"] {
            assert_eq!(valid_color(bad), None, "{bad}");
        }
    }
}
