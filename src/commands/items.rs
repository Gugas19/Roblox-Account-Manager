//! Inventories (Avatar page) and groups (Groups page) of the accounts.

use super::*;
use crate::roblox::{Item, Membership};
use std::collections::HashMap;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Owned<T> {
    account: String,
    list: Vec<T>,
    /// why this account's list couldn't be read
    error: Option<String>,
}

/// Each account's items, read one account after the other
#[tauri::command]
pub async fn inventories(names: Vec<String>) -> Vec<Owned<Item>> {
    let app = app();
    let mut out = Vec::new();
    for a in app.accounts_named(&names) {
        let result = app.api.inventory(&a.security_token, a.user_id).await;
        out.push(match result {
            Ok(list) => Owned { account: a.username.clone(), list, error: None },
            Err(err) => Owned { account: a.username.clone(), list: Vec::new(), error: Some(app.tr(&err.to_string())) },
        });
    }
    out
}

#[tauri::command]
pub async fn asset_thumbnails(ids: Vec<i64>) -> HashMap<i64, String> {
    app().api.asset_thumbnails(&ids).await
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Groups {
    accounts: Vec<Owned<Membership>>,
    icons: HashMap<i64, String>,
}

/// The groups each account is in, and the groups' icons
#[tauri::command]
pub async fn groups(names: Vec<String>) -> Groups {
    let app = app();
    let mut accounts = Vec::new();
    for a in app.accounts_named(&names) {
        accounts.push(match app.api.groups_of(a.user_id).await {
            Ok(list) => Owned { account: a.username.clone(), list, error: None },
            Err(err) => Owned { account: a.username.clone(), list: Vec::new(), error: Some(app.tr(&err.to_string())) },
        });
    }
    let mut ids: Vec<i64> = accounts.iter().flat_map(|o| o.list.iter().map(|g| g.id)).collect();
    ids.sort_unstable();
    ids.dedup();
    let icons = app.api.group_icons(&ids).await;
    Groups { accounts, icons }
}

/// The accounts leave the group; for each one, none when it worked, the reason otherwise
#[tauri::command]
pub async fn group_leave(names: Vec<String>, group_id: i64) -> Vec<(String, Option<String>)> {
    let app = app();
    let mut out = Vec::new();
    for a in app.accounts_named(&names) {
        let result = async {
            let csrf = app.csrf(&a).await?;
            app.api.leave_group(&a.security_token, &csrf, group_id, a.user_id).await
        }
        .await;
        out.push((a.username.clone(), result.err().map(|e| app.tr(&e.to_string()))));
    }
    out
}
