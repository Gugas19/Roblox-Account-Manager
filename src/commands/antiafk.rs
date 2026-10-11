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

#[tauri::command]
pub async fn antiafk_test_webhook() -> R<()> {
    antiafk::test_webhook(&app()).await.map_err(e)
}

#[tauri::command]
pub fn antiafk_reset_stats() {
    antiafk::reset_stats(&app());
}
