//! Joining games (port of Account.JoinServer, ClientSettingsPatcher and AdjustWindowPosition).

use crate::roblox::Api;
use crate::platform::win as winutil;
use anyhow::Result;
use rand::Rng;
use serde::Serialize;
use std::path::Path;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Default)]
pub struct JoinOptions {
    pub place_id: i64,
    /// a job id, a private server link ("...privateServerLinkCode=...") or a VIP access code
    pub job_id: String,
    pub follow_user: bool,
    pub join_vip: bool,
    pub shuffle: bool,
    pub shuffle_lowest: bool,
    pub shuffle_pages: i64,
    pub auto_close_last: bool,
    pub is_teleport: bool,
    /// the user already agreed to open the account's private server privacy
    pub privacy_ok: bool,
    pub unlock_fps: bool,
    pub max_fps: i64,
    pub custom_client_settings: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(tag = "result", content = "message", rename_all = "camelCase")]
pub enum JoinResult {
    Success,
    /// the account's settings don't allow joining private servers, ask before changing them
    NeedsPrivacy,
    Error(String),
}

pub fn new_tracker_id() -> String {
    let mut rng = rand::thread_rng();
    format!("{}{}", rng.gen_range(100000..175000), rng.gen_range(100000..900000))
}

fn launch_time() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

/// Builds the roblox-player: link the Roblox website opens
pub fn player_link(ticket: &str, tracker: &str, place_launcher: &str) -> String {
    format!(
        "roblox-player:1+launchmode:play+gameinfo:{}+launchtime:{}+placelauncherurl:{}+browsertrackerid:{}+robloxLocale:en_us+gameLocale:en_us+channel:+LaunchExp:InApp",
        ticket,
        launch_time(),
        urlencoding::encode(place_launcher),
        tracker
    )
}

pub fn place_launcher_url(opts: &JoinOptions, tracker: &str, access_code: &str, link_code: &str) -> String {
    if opts.join_vip {
        format!("https://assetgame.roblox.com/game/PlaceLauncher.ashx?request=RequestPrivateGame&placeId={}&accessCode={}&linkCode={}", opts.place_id, access_code, link_code)
    } else if opts.follow_user {
        format!("https://assetgame.roblox.com/game/PlaceLauncher.ashx?request=RequestFollowUser&userId={}", opts.place_id)
    } else {
        let job = !opts.job_id.is_empty();
        format!(
            "https://assetgame.roblox.com/game/PlaceLauncher.ashx?request=RequestGame{}&browserTrackerId={}&placeId={}{}&isPlayTogetherGame=false{}",
            if job { "Job" } else { "" },
            tracker,
            opts.place_id,
            if job { format!("&gameId={}", opts.job_id) } else { String::new() },
            if opts.is_teleport { "&isTeleport=true" } else { "" }
        )
    }
}

/// Copies the custom ClientAppSettings.json or sets the FPS cap, like ClientSettingsPatcher.PatchSettings
pub fn patch_client_settings(opts: &JoinOptions) -> Result<()> {
    let folder = winutil::roblox_version_folder().ok_or_else(|| anyhow::anyhow!("Can't patch ClientAppSettings, folder doesn't exist"))?;

    if !folder.file_name().map(|n| n.to_string_lossy().starts_with("version-")).unwrap_or(false) {
        anyhow::bail!("Can't patch ClientAppSettings, folder doesn't start with 'version-'");
    }

    let settings_folder = folder.join("ClientSettings");
    std::fs::create_dir_all(&settings_folder)?;
    let file = settings_folder.join("ClientAppSettings.json");

    if let Some(custom) = opts.custom_client_settings.as_ref().filter(|c| !c.is_empty() && Path::new(c).exists()) {
        std::fs::copy(custom, &file)?;
    } else if opts.unlock_fps {
        let mut settings = std::fs::read_to_string(&file).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()).filter(|v| v.is_object()).unwrap_or_else(|| serde_json::json!({}));
        settings["DFIntTaskSchedulerTargetFps"] = serde_json::json!(if opts.max_fps > 0 { opts.max_fps } else { 240 });
        std::fs::write(&file, serde_json::to_string(&settings)?)?;
    }
    Ok(())
}

/// Closes the Roblox window this account opened last time, so Roblox doesn't kick it for "same account launched"
async fn close_last_process(tracker: &str) {
    for process in winutil::roblox_processes() {
        if process.tracker_id().as_deref() == Some(tracker) {
            // window handles can't be kept across awaits, look the window up each time
            for _ in 0..2 {
                if let Some(hwnd) = winutil::main_window(process.pid) {
                    winutil::close_window(hwnd);
                }
                tokio::time::sleep(Duration::from_millis(250)).await;
            }
            winutil::kill(process.pid);
        }
    }
}

/// Moves the new Roblox window to where this account's window was last time
pub async fn restore_window_position(tracker: String, rect: (i32, i32, i32, i32)) {
    let ends = Instant::now() + Duration::from_secs(45);

    while Instant::now() < ends {
        tokio::time::sleep(Duration::from_millis(350)).await;

        for process in winutil::roblox_processes() {
            if process.tracker_id().as_deref() != Some(tracker.as_str()) {
                continue;
            }
            if let Some(hwnd) = winutil::main_window(process.pid) {
                winutil::move_window(hwnd, rect.0, rect.1, rect.2, rect.3);
                return;
            }
        }
    }
}

/// Launches Roblox into a game for one account, `tracker` is the account's browser tracker id (created when empty)
pub async fn join(api: &Api, token: &str, tracker: &str, mut opts: JoinOptions) -> JoinResult {
    if let Err(e) = patch_client_settings(&opts) {
        log::warn!("{e}");
    }

    let csrf = match api.csrf(token).await {
        Ok(Some(c)) => c,
        Ok(None) => return JoinResult::Error("ERROR: Account Session Expired, re-add the account or try again. (Invalid X-CSRF-Token)".into()),
        Err(e) => return JoinResult::Error(format!("ERROR: {e}")),
    };

    if opts.shuffle && opts.job_id.is_empty() && !opts.follow_user {
        opts.job_id = api.random_job_id(opts.place_id, opts.shuffle_lowest, opts.shuffle_pages).await.unwrap_or_default();
    }

    let ticket = match api.auth_ticket(token, &csrf).await {
        Ok(Some(t)) => t,
        _ => return JoinResult::Error("ERROR: Invalid Authentication Ticket, re-add the account or try again\n(Failed to get Authentication Ticket, Roblox has probably signed you out)".into()),
    };

    if opts.auto_close_last {
        close_last_process(tracker).await;
    }

    let link_code = regex::Regex::new("privateServerLinkCode=(.+)").unwrap().captures(&opts.job_id).map(|c| c[1].to_string()).unwrap_or_default();
    let mut access_code = opts.job_id.clone();

    if !link_code.is_empty() {
        if let Ok(Some(code)) = api.private_server_access_code(token, &csrf, opts.place_id, &link_code).await {
            opts.join_vip = true;
            access_code = code;
        }
    }

    if opts.join_vip && !opts.privacy_ok {
        if let Ok(false) = api.private_server_privacy_is_everyone(token, &csrf).await {
            return JoinResult::NeedsPrivacy;
        }
    }

    let link = player_link(&ticket, tracker, &place_launcher_url(&opts, tracker, &access_code, &link_code));

    // the launcher can take a while, don't block the async runtime
    let result = tokio::task::spawn_blocking(move || winutil::shell_open(&link, true)).await;

    match result {
        Ok(Ok(())) => JoinResult::Success,
        Ok(Err(e)) => JoinResult::Error(format!("ERROR: Failed to launch Roblox! Try re-installing Roblox.\n\n{e}")),
        Err(e) => JoinResult::Error(format!("ERROR: {e}")),
    }
}

/// roblox-player link to copy (developer mode), opens the game the place/job boxes point to
pub fn shareable_link(ticket: &str, place_id: &str, job_id: &str) -> String {
    let has_job = !job_id.is_empty();
    let tracker = format!("{}{}", rand::thread_rng().gen_range(100000..130000), rand::thread_rng().gen_range(100000..900000));
    format!(
        "<roblox-player://1/1+launchmode:play+gameinfo:{}+launchtime:{}+browsertrackerid:{}+placelauncherurl:https://assetgame.roblox.com/game/PlaceLauncher.ashx?request=RequestGame{}&placeId={}{}+robloxLocale:en_us+gameLocale:en_us>",
        ticket,
        launch_time(),
        tracker,
        if has_job { "Job" } else { "" },
        place_id,
        if has_job { format!("&gameId={job_id}") } else { String::new() }
    )
}

pub fn app_link(ticket: &str) -> String {
    let tracker = format!("{}{}", rand::thread_rng().gen_range(500000..600000), rand::thread_rng().gen_range(10000..90000));
    format!("<roblox-player://1/1+launchmode:app+gameinfo:{}+launchtime:{}+browsertrackerid:{}+robloxLocale:en_us+gameLocale:en_us>", ticket, launch_time(), tracker)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn place_launcher_urls() {
        let mut o = JoinOptions { place_id: 1818, ..Default::default() };
        assert_eq!(place_launcher_url(&o, "123", "", ""), "https://assetgame.roblox.com/game/PlaceLauncher.ashx?request=RequestGame&browserTrackerId=123&placeId=1818&isPlayTogetherGame=false");
        o.job_id = "abc".into();
        assert!(place_launcher_url(&o, "123", "", "").contains("request=RequestGameJob&browserTrackerId=123&placeId=1818&gameId=abc"));
        o.follow_user = true;
        assert!(place_launcher_url(&o, "1", "", "").ends_with("request=RequestFollowUser&userId=1818"));
        o.join_vip = true;
        assert!(place_launcher_url(&o, "1", "code", "link").ends_with("RequestPrivateGame&placeId=1818&accessCode=code&linkCode=link"));
    }

    #[test]
    fn player_link_encodes_the_launcher_url() {
        let link = player_link("TICKET", "42", "https://a.b/c?x=1&y=2");
        assert!(link.starts_with("roblox-player:1+launchmode:play+gameinfo:TICKET+launchtime:"));
        assert!(link.contains("+placelauncherurl:https%3A%2F%2Fa.b%2Fc%3Fx%3D1%26y%3D2+browsertrackerid:42+"));
    }

    #[test]
    fn tracker_ids_look_like_the_csharp_ones() {
        let id = new_tracker_id();
        assert_eq!(id.len(), 12);
        assert!(id.chars().all(|c| c.is_ascii_digit()));
    }
}
