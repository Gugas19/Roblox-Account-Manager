//! The account list: editing, groups, order, import, copy, details.

use super::*;

#[tauri::command]
pub fn list_accounts() -> Vec<AccountView> {
    app().views()
}

#[tauri::command]
pub fn remove_accounts(names: Vec<String>) {
    let app = app();
    {
        let mut store = app.store.lock().unwrap();
        let removed: Vec<(usize, Account)> = store.accounts.iter().enumerate().filter(|(_, a)| names.contains(&a.username)).map(|(i, a)| (i, a.clone())).collect();
        store.accounts.retain(|a| !names.contains(&a.username));
        *app.removed.lock().unwrap() = removed;
    }
    // an empty list must be saved too when the last account is removed
    if let Err(err) = app.store.lock().unwrap().save(true, true) {
        app.toast("error", format!("Failed to save accounts: {err}"));
    }
    app.accounts_changed();
}

/// Puts the accounts removed last back where they were
#[tauri::command]
pub fn undo_remove() -> usize {
    let app = app();
    let removed = std::mem::take(&mut *app.removed.lock().unwrap());
    let count = removed.len();
    {
        let mut store = app.store.lock().unwrap();
        for (index, account) in removed {
            if store.accounts.iter().any(|a| a.user_id == account.user_id) {
                continue;
            }
            let index = index.min(store.accounts.len());
            store.accounts.insert(index, account);
        }
    }
    app.save_accounts(true);
    count
}

#[tauri::command]
pub fn set_alias(names: Vec<String>, alias: String) -> R<()> {
    let app = app();
    for n in &names {
        let mut ok = true;
        app.update_account(n, |a| ok = a.set_alias(&alias));
        if !ok {
            return Err("The alias can have at most 50 characters".into());
        }
    }
    app.save_accounts(false);
    Ok(())
}

/// The frame rate limit of the accounts (applied when they launch), `None` goes back to the global setting
#[tauri::command]
pub fn set_fps_limit(names: Vec<String>, limit: Option<i64>) -> R<()> {
    let app = app();
    if let Some(n) = limit {
        if !(launcher::FPS_MIN..=launcher::FPS_MAX).contains(&n) {
            return Err(format!("The frame rate limit goes from {} to {}", launcher::FPS_MIN, launcher::FPS_MAX));
        }
    }
    for n in &names {
        app.update_account(n, |a| match limit {
            Some(fps) => drop(a.fields.insert("FPSLimit".into(), fps.to_string())),
            None => drop(a.fields.shift_remove("FPSLimit")),
        });
    }
    app.save_accounts(true);
    Ok(())
}

/// The color of the accounts, `None` takes it away
#[tauri::command]
pub fn set_account_color(names: Vec<String>, color: Option<String>) -> R<()> {
    let app = app();
    let color = match color {
        Some(c) => Some(crate::app::valid_color(&c).ok_or("Not a color")?),
        None => None,
    };
    for n in &names {
        app.update_account(n, |a| match &color {
            Some(c) => drop(a.fields.insert("Color".into(), c.clone())),
            None => drop(a.fields.shift_remove("Color")),
        });
    }
    app.save_accounts(true);
    Ok(())
}

/// The color of a group, `None` takes it away
#[tauri::command]
pub fn set_group_color(group: String, color: Option<String>) -> R<()> {
    let app = app();
    let color = match color {
        Some(c) => Some(crate::app::valid_color(&c).ok_or("Not a color")?),
        None => None,
    };
    app.set_group_color(&group, color);
    app.accounts_changed();
    Ok(())
}

#[tauri::command]
pub fn set_description(names: Vec<String>, description: String) -> R<()> {
    let app = app();
    for n in &names {
        let mut ok = true;
        app.update_account(n, |a| ok = a.set_description(&description));
        if !ok {
            return Err("The description can have at most 5000 characters".into());
        }
    }
    app.save_accounts(false);
    Ok(())
}

#[tauri::command]
pub fn move_to_group(names: Vec<String>, group: String) {
    let app = app();
    let group = if group.trim().is_empty() { "Default".to_string() } else { group.trim().to_string() };
    for n in &names {
        app.update_account(n, |a| a.group = group.clone());
    }
    app.save_accounts(false);
}

/// Renames a group: every account in it moves to the new name, and the group's color goes with them
#[tauri::command]
pub fn rename_group(from: String, to: String) -> R<usize> {
    let app = app();
    let to = to.trim().to_string();
    if to.is_empty() {
        return Err("The group needs a name".into());
    }
    if to.chars().count() > 100 {
        return Err("The group name can have at most 100 characters".into());
    }
    let mut moved = 0;
    {
        let mut store = app.store.lock().unwrap();
        for a in store.accounts.iter_mut().filter(|a| a.group == from) {
            a.group = to.clone();
            moved += 1;
        }
    }
    if let Some(color) = app.group_colors().get(&from).cloned() {
        app.set_group_color(&from, None);
        app.set_group_color(&to, Some(color));
    }
    app.save_accounts(true);
    Ok(moved)
}

/// Drag and drop: puts the accounts before `before` (or at the end of the group) and moves them to its group
#[tauri::command]
pub fn reorder(names: Vec<String>, before: Option<String>, group: String) {
    let app = app();
    {
        let mut store = app.store.lock().unwrap();
        let mut moved: Vec<Account> = Vec::new();
        store.accounts.retain(|a| {
            if names.contains(&a.username) {
                moved.push(a.clone());
                false
            } else {
                true
            }
        });
        for a in moved.iter_mut() {
            a.group = group.clone();
        }
        let index = before
            .and_then(|b| store.accounts.iter().position(|a| a.username == b))
            .or_else(|| store.accounts.iter().rposition(|a| a.group == group).map(|i| i + 1))
            .unwrap_or(store.accounts.len());
        for (i, a) in moved.into_iter().enumerate() {
            store.accounts.insert(index + i, a);
        }
    }
    app.save_accounts(false);
}

#[tauri::command]
pub fn sort_alphabetically() {
    let app = app();
    app.store.lock().unwrap().accounts.sort_by(|a, b| {
        let key = |x: &Account| (!x.username.chars().all(|c| c.is_ascii_digit()), !x.username.chars().any(|c| c.is_alphabetic()), x.username.to_lowercase());
        key(a).cmp(&key(b))
    });
    app.save_accounts(false);
}

#[tauri::command]
pub fn save_place(names: Vec<String>, place_id: String, job_id: String) {
    let app = app();
    let link_place = regex::Regex::new(r"/games/(\d+)/").unwrap().captures(&job_id).map(|c| c[1].to_string());
    for n in &names {
        app.update_account(n, |a| {
            if place_id.is_empty() && job_id.is_empty() {
                a.fields.shift_remove("SavedPlaceId");
                a.fields.shift_remove("SavedJobId");
            } else {
                let place = if job_id.contains("privateServerLinkCode") { link_place.clone().unwrap_or_else(|| place_id.clone()) } else { place_id.clone() };
                a.fields.insert("SavedPlaceId".into(), place);
                a.fields.insert("SavedJobId".into(), job_id.clone());
            }
        });
    }
    app.save_accounts(false);
}

#[tauri::command]
pub fn clear_saved_places() -> usize {
    let app = app();
    let mut count = 0;
    for a in app.store.lock().unwrap().accounts.iter_mut() {
        if a.fields.shift_remove("SavedPlaceId").is_some() | a.fields.shift_remove("SavedJobId").is_some() {
            count += 1;
        }
    }
    app.save_accounts(false);
    count
}

#[derive(Serialize)]
pub struct ImportResult {
    pub text: String,
    pub ok: bool,
}

#[tauri::command]
pub async fn import_cookies(text: String) -> Vec<ImportResult> {
    let app = app();
    let re = regex::Regex::new(r"_\|WARNING:-DO-NOT-SHARE-THIS\.--Sharing-this-will-allow-someone-to-log-in-as-you-and-to-steal-your-ROBUX-and-items\.\|\w+").unwrap();
    let mut tokens: Vec<String> = re.find_iter(&text).map(|m| m.as_str().to_string()).collect();
    if tokens.is_empty() {
        tokens = text.lines().map(|l| l.trim().to_string()).filter(|l| !l.is_empty()).collect();
    }

    let mut results = Vec::new();
    for token in tokens {
        let short = if token.len() > 24 { format!("…{}", &token[token.len() - 12..]) } else { token.clone() };
        match app.add_account(&token, "").await {
            Ok(a) => results.push(ImportResult { text: a.username, ok: true }),
            Err(_) => results.push(ImportResult { text: short, ok: false }),
        }
    }
    results
}

#[tauri::command]
pub fn account_fields(name: String) -> R<Value> {
    let a = app().account(&name).ok_or("No Account Selected!")?;
    Ok(json!(a.fields))
}

#[tauri::command]
pub fn set_field(name: String, field: String, value: String) {
    let app = app();
    app.update_account(&name, |a| {
        if value.is_empty() {
            a.fields.shift_remove(&field);
        } else {
            a.fields.insert(field.clone(), value.clone());
        }
    });
    app.save_accounts(false);
}

/// Text to copy for the selected accounts
#[tauri::command]
pub async fn copy_text(names: Vec<String>, kind: String, place_id: Option<String>, job_id: Option<String>) -> R<String> {
    let app = app();
    let accounts = app.accounts_named(&names);
    if accounts.is_empty() {
        return Err("No Account Selected!".into());
    }

    if (kind == "password" || kind == "combo") && accounts.iter().all(|a| a.password.is_empty()) {
        return Err("No saved password, it's saved when you log in with the browser".into());
    }

    let lines: Vec<String> = match kind.as_str() {
        "username" => accounts.iter().map(|a| a.username.clone()).collect(),
        "password" => accounts.iter().map(|a| a.password.clone()).collect(),
        "combo" => accounts.iter().map(|a| format!("{}:{}", a.username, a.password)).collect(),
        "profile" => accounts.iter().map(|a| format!("https://www.roblox.com/users/{}/profile", a.user_id)).collect(),
        "userId" => accounts.iter().map(|a| a.user_id.to_string()).collect(),
        "token" => accounts.iter().map(|a| a.security_token.clone()).collect(),
        "group" => vec![accounts[0].group.clone()],
        "ticket" | "playerLink" | "appLink" => {
            let mut out = Vec::new();
            for a in &accounts {
                let csrf = app.csrf(a).await.map_err(e)?;
                if let Some(ticket) = app.api.auth_ticket(&a.security_token, &csrf).await.map_err(e)? {
                    out.push(match kind.as_str() {
                        "ticket" if accounts.len() > 1 => format!("{}:{}", a.username, ticket),
                        "ticket" => ticket,
                        "playerLink" => launcher::shareable_link(&ticket, place_id.as_deref().unwrap_or(""), job_id.as_deref().unwrap_or("")),
                        _ => launcher::app_link(&ticket),
                    });
                }
                if kind != "ticket" {
                    break;
                }
            }
            out
        }
        _ => return Err("unknown".into()),
    };

    Ok(lines.join("\n"))
}

#[tauri::command]
pub fn dump_details(names: Vec<String>) {
    let app = app();
    for account in app.accounts_named(&names) {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            let result: anyhow::Result<std::path::PathBuf> = async {
                let info = app.api.user_info(account.user_id, Some(&account.security_token)).await.ok();
                let age = info
                    .as_ref()
                    .and_then(|i| i["created"].as_str())
                    .and_then(|c| chrono::DateTime::parse_from_rfc3339(c).ok())
                    .map(|c| format!("{:.1}", (chrono::Utc::now() - c.with_timezone(&chrono::Utc)).num_seconds() as f64 / 86400.0))
                    .unwrap_or_else(|| "UNKNOWN".into());
                let text = format!(
                    "Username: {}\r\nUserId: {}\r\nRobux: {}\r\nAccount Age: {}\r\nEmail Status: {}\r\nUser Info: {}\r\nOther: {}\r\nFields: {}\r\n",
                    account.username,
                    account.user_id,
                    app.api.robux(&account.security_token, account.user_id).await.unwrap_or(0),
                    age,
                    app.api.email_info(&account.security_token).await.unwrap_or_default(),
                    info.map(|i| i.to_string()).unwrap_or_default(),
                    app.api.mobile_info(&account.security_token).await.ok().flatten().map(|i| i.to_string()).unwrap_or_default(),
                    serde_json::to_string(&account.fields)?
                );
                let folder = app.dir.join("AccountDumps");
                std::fs::create_dir_all(&folder)?;
                let file = folder.join(format!("{}.txt", account.username));
                std::fs::write(&file, text)?;
                Ok(file)
            }
            .await;

            match result {
                Ok(file) => {
                    let _ = winutil::shell_open(&file.to_string_lossy(), None);
                }
                Err(err) => app.toast("error", format!("Could not get the details of {}:\n\n{}", account.username, err)),
            }
        });
    }
}

#[tauri::command]
pub async fn quick_login_info(name: String, code: String) -> R<Value> {
    let app = app();
    let a = app.account(&name).ok_or("No Account Selected!")?;
    if code.trim().len() != 6 {
        return Err("Quick Log In codes requires 6 characters".into());
    }
    let csrf = app.csrf(&a).await.map_err(e)?;
    let info = app.api.quick_login_info(&a.security_token, &csrf, code.trim()).await.map_err(e)?.ok_or("Invalid code")?;
    Ok(json!({ "device": info["deviceInfo"].as_str().unwrap_or("Unknown"), "location": info["location"].as_str().unwrap_or("Unknown") }))
}

#[tauri::command]
pub async fn quick_login_confirm(name: String, code: String) -> R<bool> {
    let app = app();
    let a = app.account(&name).ok_or("No Account Selected!")?;
    let csrf = app.csrf(&a).await.map_err(e)?;
    app.api.quick_login_validate(&a.security_token, &csrf, code.trim()).await.map_err(e)
}
