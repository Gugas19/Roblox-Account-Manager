//! Session history page: the events, clearing them, exporting them as CSV.

use super::*;
use crate::services::history;

#[tauri::command]
pub fn history_list() -> Vec<history::Event> {
    history::list(&app())
}

#[tauri::command]
pub fn history_clear() {
    history::clear(&app());
}

/// Saves the events the page shows (its filter) as a CSV file, where the person chooses; the path, or none when
/// they cancelled
#[tauri::command]
pub fn history_export(accounts: Vec<String>) -> R<Option<String>> {
    let events: Vec<history::Event> = history::list(&app()).into_iter().filter(|e| accounts.is_empty() || accounts.contains(&e.account)).rev().collect();
    let name = format!("RobloxAccountManager-history-{}.csv", chrono::Local::now().format("%Y-%m-%d"));
    let Some(path) = rfd::FileDialog::new().set_title("Export history").set_file_name(&name).add_filter("CSV", &["csv"]).save_file() else {
        return Ok(None);
    };
    std::fs::write(&path, history::csv(&events)).map_err(e)?;
    Ok(Some(path.to_string_lossy().to_string()))
}
