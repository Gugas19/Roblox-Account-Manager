//! Accounts and AccountData.json, compatible with the C# version in both directions.

use crate::core::crypto;
use crate::core::netdate::NetDate;
use anyhow::{bail, Context, Result};
use indexmap::IndexMap;
use serde::{Deserialize, Deserializer, Serialize};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant, SystemTime};

fn null_string<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.unwrap_or_default())
}

fn null_group<'de, D: Deserializer<'de>>(d: D) -> Result<String, D::Error> {
    Ok(Option::<String>::deserialize(d)?.filter(|g| !g.is_empty()).unwrap_or_else(default_group))
}

fn null_fields<'de, D: Deserializer<'de>>(d: D) -> Result<IndexMap<String, String>, D::Error> {
    let raw = Option::<IndexMap<String, Option<String>>>::deserialize(d)?.unwrap_or_default();
    Ok(raw.into_iter().map(|(k, v)| (k, v.unwrap_or_default())).collect())
}

fn default_group() -> String {
    "Default".to_string()
}

/// Field order and names match what Newtonsoft writes for the C# Account class
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Account {
    #[serde(rename = "Valid", default)]
    pub valid: bool,
    #[serde(rename = "SecurityToken", default, deserialize_with = "null_string")]
    pub security_token: String,
    #[serde(rename = "Username", default, deserialize_with = "null_string")]
    pub username: String,
    #[serde(rename = "LastUse", default)]
    pub last_use: NetDate,
    #[serde(rename = "UserID", default)]
    pub user_id: i64,
    #[serde(rename = "Fields", default, deserialize_with = "null_fields")]
    pub fields: IndexMap<String, String>,
    #[serde(rename = "LastAttemptedRefresh", default)]
    pub last_attempted_refresh: NetDate,
    #[serde(rename = "BrowserTrackerID", default)]
    pub browser_tracker_id: Option<String>,
    #[serde(rename = "Group", default = "default_group", deserialize_with = "null_group")]
    pub group: String,
    #[serde(rename = "Alias", default, deserialize_with = "null_string")]
    pub alias: String,
    #[serde(rename = "Description", default, deserialize_with = "null_string")]
    pub description: String,
    #[serde(rename = "Password", default, deserialize_with = "null_string")]
    pub password: String,
    /// anything a newer/other version wrote, kept as is
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

pub const MAX_ALIAS: usize = 50;
pub const MAX_DESCRIPTION: usize = 5000;
pub const MAX_PASSWORD: usize = 5000;

impl Account {
    pub fn field(&self, name: &str) -> &str {
        self.fields.get(name).map(|s| s.as_str()).unwrap_or("")
    }

    /// Same limits as the C# property setters, too long values are ignored
    pub fn set_alias(&mut self, alias: &str) -> bool {
        if alias.chars().count() > MAX_ALIAS {
            return false;
        }
        self.alias = alias.to_string();
        true
    }

    pub fn set_description(&mut self, description: &str) -> bool {
        if description.chars().count() > MAX_DESCRIPTION {
            return false;
        }
        self.description = description.to_string();
        true
    }

    pub fn set_password(&mut self, password: &str) -> bool {
        if password.chars().count() > MAX_PASSWORD {
            return false;
        }
        self.password = password.to_string();
        true
    }

    /// The group name without its sorting number ("1Main" -> "Main", "007 Bank" -> "Bank")
    pub fn group_title(group: &str) -> String {
        let re = regex::Regex::new(r"^\d{1,3}\s?").unwrap();
        match re.find(group) {
            Some(m) => group[m.end()..].to_string(),
            None => group.to_string(),
        }
    }
}

/// How AccountData.json is protected
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Protection {
    /// Windows DPAPI, tied to this Windows user
    Default,
    /// password encrypted, works on any PC
    Password,
    /// plain JSON, only when NoEncryption.IUnderstandTheRisks.iautamor exists
    None,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum LoadState {
    /// accounts are available
    Ready,
    /// no accounts file yet, the user picks how to protect it
    ChooseProtection,
    /// the file is password protected, ask for it
    NeedsPassword,
    /// nothing loaded but there is a backup, ask whether to load it
    OfferBackup { locked: bool },
    /// the file couldn't be read, a copy was saved as AccountData.json.bak
    Failed { message: String },
}

pub struct AccountStore {
    pub accounts: Vec<Account>,
    dir: PathBuf,
    started: Instant,
    /// SHA-512 of the password, DPAPI protected while in memory (like the C# version)
    password: Option<Vec<u8>>,
    pub entered_password: bool,
}

impl AccountStore {
    pub fn new(dir: &Path) -> AccountStore {
        AccountStore { accounts: Vec::new(), dir: dir.to_path_buf(), started: Instant::now(), password: None, entered_password: false }
    }

    pub fn file(&self) -> PathBuf {
        self.dir.join("AccountData.json")
    }

    fn backup_file(&self) -> PathBuf {
        self.dir.join("AccountData.json.backup")
    }

    pub fn exists(&self) -> bool {
        self.file().exists()
    }

    fn no_encryption(&self) -> bool {
        self.dir.join("NoEncryption.IUnderstandTheRisks.iautamor").exists()
    }

    pub fn protection(&self) -> Protection {
        if self.password.is_some() {
            Protection::Password
        } else if self.no_encryption() {
            Protection::None
        } else {
            Protection::Default
        }
    }

    pub fn has_password(&self) -> bool {
        self.password.is_some()
    }

    fn set_password_hash(&mut self, hash: Option<&[u8]>) -> Result<()> {
        self.password = match hash {
            Some(h) => Some(crypto::protect(h, &[])?),
            None => None,
        };
        Ok(())
    }

    fn password_hash(&self) -> Result<Option<Vec<u8>>> {
        Ok(match &self.password {
            Some(p) => Some(crypto::unprotect(p, &[])?),
            None => None,
        })
    }

    pub(crate) fn parse(data: &[u8]) -> Result<Vec<Account>> {
        let text = String::from_utf8_lossy(data);
        let text = text.trim_start_matches('\u{feff}');
        if text.trim().is_empty() || text.trim() == "null" {
            return Ok(Vec::new());
        }
        Ok(serde_json::from_str(text)?)
    }

    /// Plain JSON or DPAPI protected (any scope)
    fn parse_unlocked(data: &[u8]) -> Result<Vec<Account>> {
        match crypto::unprotect(data, crypto::ENTROPY) {
            Ok(plain) => Self::parse(&plain),
            Err(e) => Self::parse(data).with_context(|| format!("{e}")),
        }
    }

    /// First step of loading, like AccountManager.LoadAccounts without a password
    pub fn load(&mut self) -> LoadState {
        let file = self.file();
        let data = match std::fs::read(&file) {
            Ok(d) => d,
            Err(_) => return LoadState::ChooseProtection,
        };

        if data.is_empty() {
            return self.after_load();
        }

        if crypto::is_password_protected(&data) {
            return LoadState::NeedsPassword;
        }

        match Self::parse_unlocked(&data) {
            Ok(list) => {
                self.accounts = list;
                self.after_load()
            }
            Err(e) => {
                let _ = std::fs::write(self.dir.join("AccountData.json.bak"), &data);
                LoadState::Failed { message: format!("{e}") }
            }
        }
    }

    fn after_load(&mut self) -> LoadState {
        if !self.entered_password && self.accounts.is_empty() {
            if let Ok(backup) = std::fs::read(self.backup_file()) {
                if !backup.is_empty() {
                    return LoadState::OfferBackup { locked: crypto::is_password_protected(&backup) };
                }
            }
        }
        LoadState::Ready
    }

    /// Opens a password protected file
    pub fn unlock(&mut self, password: &str) -> Result<()> {
        if password.chars().count() < 4 {
            bail!("Invalid password, your password must contain 4 or more characters");
        }

        let hash = crypto::password_hash(password);
        let data = std::fs::read(self.file())?;
        let plain = crypto::decrypt(&data, &hash)?;

        self.accounts = Self::parse(&plain)?;
        self.set_password_hash(Some(&hash))?;
        self.entered_password = true;
        Ok(())
    }

    /// Checks a password against the file without loading it (changing the protection asks for it first)
    pub fn check_password(&self, password: &str) -> Result<()> {
        let data = std::fs::read(self.file())?;
        if !crypto::is_password_protected(&data) {
            return Ok(());
        }
        crypto::decrypt(&data, &crypto::password_hash(password))?;
        Ok(())
    }

    /// Loads the backup file the user agreed to, a password protected one becomes the main file
    pub fn load_backup(&mut self) -> Result<LoadState> {
        let backup = std::fs::read(self.backup_file())?;

        if crypto::is_password_protected(&backup) {
            let file = self.file();
            if file.exists() {
                let old = self.dir.join("AccountData.json.old");
                let _ = std::fs::remove_file(&old);
                std::fs::rename(&file, &old)?;
            }
            std::fs::rename(self.backup_file(), &file)?;
            return Ok(LoadState::NeedsPassword);
        }

        self.accounts = Self::parse_unlocked(&backup).context("Failed to load backup file!")?;
        Ok(LoadState::Ready)
    }

    /// Picks the protection for the file and saves right away
    pub fn set_protection(&mut self, password: Option<&str>) -> Result<()> {
        match password {
            Some(p) => {
                if p.chars().count() < 4 {
                    bail!("Invalid password, your password must contain 4 or more characters");
                }
                self.set_password_hash(Some(&crypto::password_hash(p)))?;
            }
            None => self.set_password_hash(None)?,
        }
        self.save(true, true)?;
        // the backup is a copy of the file as it was before: it would keep the old protection (and the cookies under
        // it) for up to 8 hours, so it becomes a copy of the newly protected file
        if self.backup_file().exists() {
            std::fs::copy(self.file(), self.backup_file())?;
        }
        Ok(())
    }

    /// Same rules as the C# SaveAccounts: not in the first 2 seconds (unless forced), never an empty list over the file
    /// (unless forced), a backup of the previous file at most every 8 hours
    pub fn save(&self, bypass_rate_limit: bool, bypass_count_check: bool) -> Result<()> {
        if !bypass_rate_limit && self.started.elapsed() < Duration::from_secs(2) {
            return Ok(());
        }
        if !bypass_count_check && self.accounts.is_empty() {
            return Ok(());
        }

        let file = self.file();
        let old = std::fs::read(&file).unwrap_or_default();
        let backup = self.backup_file();
        let backup_age = std::fs::metadata(&backup).and_then(|m| m.modified()).ok().and_then(|t| SystemTime::now().duration_since(t).ok());

        if backup_age.map(|age| age > Duration::from_secs(8 * 3600)).unwrap_or(true) {
            std::fs::write(&backup, &old)?;
        }

        let json = serde_json::to_string(&self.accounts)?;

        let bytes = match self.password_hash()? {
            Some(hash) => crypto::encrypt(&json, &hash)?,
            None if self.no_encryption() => json.into_bytes(),
            None => crypto::protect(json.as_bytes(), crypto::ENTROPY)?,
        };

        let temp = file.with_extension("json.tmp");
        std::fs::write(&temp, bytes)?;
        std::fs::rename(&temp, &file)?;
        Ok(())
    }

    #[cfg(test)]
    pub fn pretend_started_long_ago(&mut self) {
        self.started = Instant::now() - Duration::from_secs(60);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("ram-rust-test-{}-{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    const CSHARP_JSON: &str = r#"[{"Valid":false,"SecurityToken":"fake-token-1","Username":"NightFalcon_07","LastUse":"2026-10-11T13:23:46.7668383+01:00","UserID":1,"Fields":{},"LastAttemptedRefresh":"0001-01-01T00:00:00","BrowserTrackerID":"124928268464","Group":"1Main","Alias":"Main","Description":"","Password":"pw-one"},{"Valid":false,"SecurityToken":"fake-token-6","Username":"GrindBot_B","LastUse":"2026-10-11T13:23:46.7668383+01:00","UserID":6,"Fields":{"SavedPlaceId":"1818","SavedJobId":"test-job-id"},"LastAttemptedRefresh":"0001-01-01T00:00:00","BrowserTrackerID":null,"Group":"3Farm","Alias":"Farm B","Description":"","Password":""}]"#;

    #[test]
    fn json_round_trips_exactly() {
        let accounts: Vec<Account> = serde_json::from_str(CSHARP_JSON).unwrap();
        assert_eq!(accounts[1].field("SavedJobId"), "test-job-id");
        assert_eq!(serde_json::to_string(&accounts).unwrap(), CSHARP_JSON);
    }

    #[test]
    fn tolerates_nulls_and_unknown_fields() {
        let accounts: Vec<Account> = serde_json::from_str(r#"[{"Username":"A","Group":null,"Alias":null,"Fields":{"X":null},"Future":42}]"#).unwrap();
        assert_eq!(accounts[0].group, "Default");
        assert_eq!(accounts[0].alias, "");
        assert_eq!(accounts[0].field("X"), "");
        assert!(serde_json::to_string(&accounts).unwrap().contains("\"Future\":42"));
    }

    #[test]
    fn group_titles_hide_the_sorting_number() {
        assert_eq!(Account::group_title("1Main"), "Main");
        assert_eq!(Account::group_title("007 Bank"), "Bank");
        assert_eq!(Account::group_title("Default"), "Default");
    }

    #[test]
    fn limits() {
        let mut a = Account::default();
        assert!(a.set_alias("Main"));
        assert!(!a.set_alias(&"a".repeat(51)));
        assert_eq!(a.alias, "Main");
        assert!(!a.set_description(&"a".repeat(5001)));
    }

    #[test]
    fn saves_with_every_protection_and_loads_back() {
        for protection in ["dpapi", "plain", "password"] {
            let dir = temp_dir(protection);
            if protection == "plain" {
                std::fs::write(dir.join("NoEncryption.IUnderstandTheRisks.iautamor"), "").unwrap();
            }

            let mut store = AccountStore::new(&dir);
            assert_eq!(store.load(), LoadState::ChooseProtection);
            store.accounts = serde_json::from_str(CSHARP_JSON).unwrap();
            store.set_protection(if protection == "password" { Some("teste1234") } else { None }).unwrap();

            let raw = std::fs::read(store.file()).unwrap();
            assert_eq!(String::from_utf8_lossy(&raw).contains("NightFalcon_07"), protection == "plain", "{protection}");

            let mut again = AccountStore::new(&dir);
            if protection == "password" {
                assert_eq!(again.load(), LoadState::NeedsPassword);
                assert!(again.unlock("wrong-one").is_err());
                again.unlock("teste1234").unwrap();
            } else {
                assert_eq!(again.load(), LoadState::Ready);
            }
            assert_eq!(again.accounts.len(), 2);
            assert_eq!(again.protection() == Protection::Password, protection == "password");
        }
    }

    #[test]
    fn save_rules() {
        let dir = temp_dir("rules");
        std::fs::write(dir.join("NoEncryption.IUnderstandTheRisks.iautamor"), "").unwrap();
        let mut store = AccountStore::new(&dir);
        store.accounts = serde_json::from_str(CSHARP_JSON).unwrap();

        store.save(false, false).unwrap();
        assert!(!store.file().exists(), "saved in the first 2 seconds");

        store.pretend_started_long_ago();
        store.save(false, false).unwrap();
        assert!(store.file().exists());

        let saved = std::fs::read(store.file()).unwrap();
        store.accounts.clear();
        store.save(false, false).unwrap();
        assert_eq!(std::fs::read(store.file()).unwrap(), saved, "an empty list overwrote the accounts");

        assert!(dir.join("AccountData.json.backup").exists());
    }

    #[test]
    fn a_new_protection_reaches_the_backup_too() {
        let dir = temp_dir("protection-backup");
        let mut store = AccountStore::new(&dir);
        store.load();
        store.accounts = serde_json::from_str(CSHARP_JSON).unwrap();
        store.set_protection(None).unwrap();
        // a backup made under the Windows protection, then a password chosen
        std::fs::write(store.backup_file(), std::fs::read(store.file()).unwrap()).unwrap();
        store.set_protection(Some("teste1234")).unwrap();
        assert_eq!(std::fs::read(store.backup_file()).unwrap(), std::fs::read(store.file()).unwrap(), "the backup kept the old protection");
    }

    #[test]
    fn offers_the_backup_when_nothing_loaded() {
        let dir = temp_dir("backup");
        std::fs::write(dir.join("NoEncryption.IUnderstandTheRisks.iautamor"), "").unwrap();
        std::fs::write(dir.join("AccountData.json"), "[]").unwrap();
        std::fs::write(dir.join("AccountData.json.backup"), CSHARP_JSON).unwrap();

        let mut store = AccountStore::new(&dir);
        assert_eq!(store.load(), LoadState::OfferBackup { locked: false });
        assert_eq!(store.load_backup().unwrap(), LoadState::Ready);
        assert_eq!(store.accounts.len(), 2);
    }

    #[test]
    fn opens_the_csharp_password_file() {
        let dir = temp_dir("csharp");
        std::fs::copy(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/password-teste1234.bin"), dir.join("AccountData.json")).unwrap();
        let mut store = AccountStore::new(&dir);
        assert_eq!(store.load(), LoadState::NeedsPassword);
        store.unlock("teste1234").unwrap();
        assert_eq!(store.accounts[0].username, "NightFalcon_07");
        assert_eq!(store.accounts[0].field("SavedPlaceId"), "1818");
    }
}
