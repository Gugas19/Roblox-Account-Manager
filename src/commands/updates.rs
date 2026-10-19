//! Updates: what's new, checking now, installing the downloaded one.

use super::*;
use crate::services::updater::{self, Status};

#[tauri::command]
pub fn update_status() -> Value {
    json!({ "status": updater::status(&app()), "version": env!("CARGO_PKG_VERSION") })
}

/// `download`: get it right away instead of only telling that there's one
#[tauri::command]
pub async fn update_check(download: bool) -> Status {
    updater::check(&app(), download).await
}

#[tauri::command]
pub async fn update_restart() -> R<()> {
    updater::restart(&app()).await.map_err(e)
}
