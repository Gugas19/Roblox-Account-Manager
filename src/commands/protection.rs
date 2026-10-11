//! How AccountData.json is protected (Windows or password).

use super::*;

#[tauri::command]
pub fn choose_protection(password: Option<String>) -> R<()> {
    let app = app();
    let result = app.store.lock().unwrap().set_protection(password.as_deref()).map_err(e);
    result
}

#[tauri::command]
pub fn unlock(password: String) -> R<()> {
    app().store.lock().unwrap().unlock(&password).map_err(e)
}

#[tauri::command]
pub fn check_password(password: String) -> R<()> {
    app().store.lock().unwrap().check_password(&password).map_err(e)
}

#[tauri::command]
pub fn protection() -> Value {
    let app = app();
    let store = app.store.lock().unwrap();
    json!({ "hasPassword": store.has_password(), "protection": store.protection() })
}

#[tauri::command]
pub fn load_backup() -> R<LoadState> {
    app().store.lock().unwrap().load_backup().map_err(e)
}
