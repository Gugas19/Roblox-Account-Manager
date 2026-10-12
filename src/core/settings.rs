//! RAMSettings.ini and RAMTheme.ini with the C# version's defaults.

use crate::core::ini::{IniFile, Section};
use std::path::{Path, PathBuf};

pub struct Settings {
    pub ini: IniFile,
    path: PathBuf,
}

impl Settings {
    pub fn load(dir: &Path) -> Settings {
        let path = dir.join("RAMSettings.ini");
        let mut settings = Settings { ini: IniFile::load(&path), path };
        settings.apply_defaults();
        settings
    }

    fn apply_defaults(&mut self) {
        let g = self.ini.section_mut("General");
        for (key, value) in [
            ("AccountJoinDelay", "8"),
            ("AsyncJoin", "false"),
            ("DisableAgingAlert", "false"),
            ("SavePasswords", "true"),
            ("EnableMultiRbx", "true"),
            ("MaxRecentGames", "8"),
            ("ShuffleChoosesLowestServer", "false"),
            ("ShufflePageCount", "5"),
            ("IPApiLink", "http://ip-api.com/json/<ip>"),
            ("AutoCookieRefresh", "true"),
            ("AutoCloseLastProcess", "true"),
            ("ShowPresence", "true"),
            ("PresenceUpdateRate", "5"),
            ("UnlockFPS", "false"),
            ("MaxFPSValue", "120"),
        ] {
            g.set_default(key, value);
        }
        if !g.exists("ServerRegionFormat") {
            g.set_with_comment("ServerRegionFormat", "<city>, <countryCode>", Some("Visit http://ip-api.com/json/1.1.1.1 to see available format options"));
        }

        let d = self.ini.section_mut("Developer");
        d.set_default("DevMode", "false");
        d.set_default("EnableWebServer", "false");

        let w = self.ini.section_mut("WebServer");
        for (key, value) in [
            ("WebServerPort", "7963"),
            ("AllowGetCookie", "false"),
            ("AllowGetAccounts", "false"),
            ("AllowLaunchAccount", "false"),
            ("AllowAccountEditing", "false"),
            ("EveryRequestRequiresPassword", "false"),
            ("AllowExternalConnections", "false"),
        ] {
            w.set_default(key, value);
        }

        let updates = self.ini.section_mut("Updates");
        updates.set_default("Check", "true");
        updates.set_default("AutoInstall", "true");

        let afk = self.ini.section_mut("AntiAFK");
        for (key, value) in ANTI_AFK_DEFAULTS {
            afk.set_default(key, value);
        }

        let watcher = self.ini.section_mut("Watcher");
        watcher.set_default("VerifyDataModel", "true");
        watcher.set_default("IgnoreExistingProcesses", "true");
        watcher.set_default("ExpectedWindowTitle", "Roblox");
    }

    pub fn general(&self) -> Section {
        self.ini.section("General").cloned().unwrap_or_default()
    }

    pub fn get(&self, section: &str, key: &str) -> String {
        self.ini.get(section, key).unwrap_or("").to_string()
    }

    pub fn get_bool(&self, section: &str, key: &str) -> bool {
        self.ini.get_bool(section, key)
    }

    pub fn get_i64(&self, section: &str, key: &str, default: i64) -> i64 {
        self.ini.section(section).map(|s| s.get_i64(key, default)).unwrap_or(default)
    }

    pub fn set(&mut self, section: &str, key: &str, value: &str) {
        self.ini.section_mut(section).set(key, value);
    }

    pub fn save(&self) {
        if let Err(e) = self.ini.save(&self.path) {
            log::error!("Can't save RAMSettings.ini: {e}");
        }
    }
}

/// Anti-AFK ([AntiAFK]): interval in seconds, Roblox kicks after 20 minutes idle
pub const ANTI_AFK_DEFAULTS: &[(&str, &str)] = &[
    ("Interval", "540"),
    ("Action", "space"),
    ("Repeat", "3"),
    ("UserSafe", "true"),
    ("IdleSeconds", "5"),
    ("MaxWaitSeconds", "90"),
    ("SkipFocused", "true"),
    ("AutoStart", "false"),
    ("AutoStartIdle", "false"),
    ("AutoRejoin", "false"),
    ("RejoinSameServer", "true"),
    ("AutoReset", "false"),
    ("MuteBackground", "false"),
    ("Efficiency", "false"),
    ("AutoGrid", "false"),
    ("KeepAwake", "true"),
    ("Invisible", "true"),
    ("SkipFullScreen", "true"),
    ("Background", "false"),
    ("WebhookOnStart", "true"),
    ("WebhookOnStop", "true"),
    ("WebhookOnAction", "false"),
    ("WebhookOnRejoin", "true"),
    ("WebhookOnError", "true"),
];

/// Theme colors (RAMTheme.ini, section "RobloxAccountManager"), only the ones the new interface uses
pub struct Theme {
    pub ini: IniFile,
    path: PathBuf,
}

pub const THEME_SECTION: &str = "RobloxAccountManager";

pub const THEME_DEFAULTS: &[(&str, &str)] = &[
    ("AccountsBG", "#1A1C24"),
    ("AccountsFG", "#E6E8EE"),
    ("ButtonsBG", "#262935"),
    ("ButtonsFG", "#E6E8EE"),
    ("FormsBG", "#14161C"),
    ("FormsFG", "#E6E8EE"),
    ("TextBoxesBG", "#1F2230"),
    ("TextBoxesFG", "#E6E8EE"),
    ("AccentColor", "#4F7CFF"),
    ("ShowHeaders", "True"),
    ("DarkTopBar", "True"),
];

impl Theme {
    pub fn load(dir: &Path) -> Theme {
        let path = dir.join("RAMTheme.ini");
        Theme { ini: IniFile::load(&path), path }
    }

    pub fn values(&self) -> serde_json::Map<String, serde_json::Value> {
        let section = self.ini.section(THEME_SECTION);
        THEME_DEFAULTS
            .iter()
            .map(|(k, default)| (k.to_string(), serde_json::Value::String(section.and_then(|s| s.get(k)).unwrap_or(default).to_string())))
            .collect()
    }

    pub fn set(&mut self, key: &str, value: &str) {
        self.ini.section_mut(THEME_SECTION).set(key, value);
        if let Err(e) = self.ini.save(&self.path) {
            log::error!("Can't save RAMTheme.ini: {e}");
        }
    }

    pub fn reset(&mut self) {
        for (k, v) in THEME_DEFAULTS {
            self.ini.section_mut(THEME_SECTION).set(k, v);
        }
        let _ = self.ini.save(&self.path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_match_the_csharp_version_and_keep_existing_values() {
        let dir = std::env::temp_dir().join(format!("ram-settings-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("RAMSettings.ini"), "[General]\nAccountJoinDelay=3\n").unwrap();

        let s = Settings::load(&dir);
        assert_eq!(s.get_i64("General", "AccountJoinDelay", 0), 3);
        assert!(s.get_bool("General", "EnableMultiRbx"));
        assert_eq!(s.get("WebServer", "WebServerPort"), "7963");
        assert!(s.ini.to_text().contains("# Visit http://ip-api.com/json/1.1.1.1 to see available format options\r\nServerRegionFormat=<city>, <countryCode>"));
    }
}
