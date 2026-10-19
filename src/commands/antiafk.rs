//! Anti-AFK window: start/stop, run now, window tools, Discord test, statistics.

use super::*;
use crate::services::antiafk;

#[tauri::command]
pub fn antiafk_status() -> antiafk::Status {
    antiafk::status(&app())
}

#[tauri::command]
pub fn antiafk_start() {
    antiafk::start(&app(), crate::text!("Started manually"));
}

#[tauri::command]
pub fn antiafk_stop() {
    antiafk::stop(&app(), true);
}

#[tauri::command]
pub fn antiafk_now() {
    antiafk::run_now(&app());
}

#[tauri::command]
pub fn antiafk_tool(kind: String) -> R<String> {
    antiafk::tool(&app(), &kind).map_err(e)
}

/// Instances page: brings a Roblox window to the front
#[tauri::command]
pub fn instance_focus(pid: u32) -> R<()> {
    antiafk::focus_instance(&app(), pid).map_err(e)
}

#[tauri::command]
pub fn instance_close(pid: u32) -> R<()> {
    antiafk::close_instance(pid).map_err(e)
}

/// Instances page: the accounts join the server that Roblox is in (a private one too, with its access code, which
/// never leaves Rust)
#[tauri::command]
pub async fn instance_join(pid: u32, names: Vec<String>) -> R<JoinResult> {
    let app = app();
    // the window handle isn't kept: it can't wait across threads
    let antiafk::Instance { account, place_id, job_id, private_access_code, .. } = antiafk::instance(&app, pid).ok_or("That Roblox isn't open anymore")?;
    let place_id = place_id.ok_or("That Roblox isn't in a game yet")?;
    let names: Vec<String> = names.into_iter().filter(|n| Some(n) != account.as_ref()).collect();
    let accounts = app.accounts_named(&names);
    if accounts.is_empty() {
        return Err("Choose the accounts to send there first (not the one already in it)".into());
    }
    let opts = match private_access_code {
        Some(code) => JoinOptions { place_id, job_id: code, join_vip: true, privacy_ok: true, ..app.join_options() },
        None => JoinOptions { place_id, job_id: job_id.unwrap_or_default(), privacy_ok: true, ..app.join_options() },
    };
    if accounts.len() > 1 {
        super::joining::launch_many(app, accounts, opts, false);
        return Ok(JoinResult::Success);
    }
    Ok(super::joining::join_one(&app, &accounts[0], opts).await)
}

#[tauri::command]
pub async fn antiafk_test_webhook() -> R<()> {
    antiafk::test_webhook(&app()).await.map_err(e)
}

#[tauri::command]
pub fn antiafk_reset_stats() {
    antiafk::reset_stats(&app());
}

/// The statistics put aside by earlier resets
#[tauri::command]
pub fn antiafk_stats_history() -> Vec<antiafk::PastStats> {
    antiafk::stats_history(&app())
}

#[tauri::command]
pub fn antiafk_delete_past_stats(id: u64) -> Vec<antiafk::PastStats> {
    antiafk::delete_past_stats(&app(), id);
    antiafk::stats_history(&app())
}
