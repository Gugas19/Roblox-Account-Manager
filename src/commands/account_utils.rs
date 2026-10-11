//! Account Utilities window.

use super::*;

pub(super) async fn account_and_csrf(name: &str, pin: bool) -> R<(Arc<App>, Account, String)> {
    let app = app();
    let a = app.account(name).ok_or("No Account Selected!")?;
    if pin {
        app.pin_ok(&a).await.map_err(e)?;
    }
    let csrf = app.csrf(&a).await.map_err(e)?;
    Ok((app, a, csrf))
}

#[tauri::command]
pub async fn change_password(name: String, current: String, new: String) -> R<String> {
    let (app, a, csrf) = account_and_csrf(&name, true).await?;
    let r = app.api.change_password(&a.security_token, &csrf, &current, &new).await.map_err(e)?;
    if !r.ok() {
        return Err("Failed to change password!".into());
    }
    let save = app.setting_bool("General", "SavePasswords");
    let cookie = r.cookie.clone();
    app.update_account(&a.username, |acc| {
        if save {
            acc.set_password(&new);
        }
        if let Some(c) = cookie.clone() {
            acc.security_token = c;
        }
    });
    app.save_accounts(true);
    Ok(if r.cookie.is_some() { "Password changed!".into() } else { "An error occured while changing passwords, you will need to re-login with your new password!".into() })
}

#[tauri::command]
pub async fn change_email(name: String, password: String, email: String) -> R<()> {
    let (app, a, csrf) = account_and_csrf(&name, true).await?;
    if app.api.change_email(&a.security_token, &csrf, &password, &email).await.map_err(e)? {
        Ok(())
    } else {
        Err("Failed to change email, maybe your password is incorrect!".into())
    }
}

#[tauri::command]
pub async fn set_display_name(name: String, display_name: String) -> R<()> {
    let (app, a, csrf) = account_and_csrf(&name, false).await?;
    app.api.set_display_name(&a.security_token, &csrf, a.user_id, &display_name).await.map_err(e)
}

#[tauri::command]
pub async fn add_friend(name: String, username: String) -> R<()> {
    let (app, a, csrf) = account_and_csrf(&name, false).await?;
    let id = app.api.user_id(username.trim(), Some(&a.security_token)).await.map_err(e)?;
    if app.api.send_friend_request(&a.security_token, &csrf, id).await.map_err(e)? {
        Ok(())
    } else {
        Err(format!("Failed to send a friend request to {username}"))
    }
}

/// Blocks a player, or unblocks them when already blocked; returns true when it unblocked
#[tauri::command]
pub async fn toggle_block(name: String, username: String) -> R<bool> {
    let (app, a, csrf) = account_and_csrf(&name, true).await?;
    let id = app.api.user_id(username.trim(), Some(&a.security_token)).await.map_err(|_| format!("Failed to obtain UserId of {username}!"))?;
    let list = app.api.blocked_list(&a.security_token).await.map_err(e)?;
    if !list.ok() {
        return Err("Failed to obtain blocked users list!".into());
    }
    let blocked = regex::Regex::new(&format!(r"\b{id}\b")).unwrap().is_match(&list.body);
    let r = app.api.block(&a.security_token, &csrf, &id.to_string(), blocked).await.map_err(e)?;
    if r.ok() {
        Ok(blocked)
    } else {
        Err(if blocked { format!("Failed to unblock {username}") } else { format!("Failed to block {username}") })
    }
}

#[tauri::command]
pub async fn unlock_pin(name: String, pin: String) -> R<()> {
    if pin.len() != 4 {
        return Err("The PIN must have 4 digits".into());
    }
    let (app, a, csrf) = account_and_csrf(&name, false).await?;
    if app.api.pin_unlocked(&a.security_token).await.unwrap_or(false) {
        return Ok(());
    }
    match app.api.unlock_pin(&a.security_token, &csrf, &pin).await.map_err(e)? {
        Some(seconds) => {
            app.sessions.lock().unwrap().entry(a.user_id).or_default().pin_unlocked_until = Some(std::time::Instant::now() + Duration::from_secs(seconds as u64));
            Ok(())
        }
        None => Err("Failed to unlock the PIN, check the PIN and the account session".into()),
    }
}

#[tauri::command]
pub async fn set_follow_privacy(name: String, privacy: usize) -> R<()> {
    let (app, a, csrf) = account_and_csrf(&name, true).await?;
    if app.api.set_follow_privacy(&a.security_token, &csrf, privacy).await.map_err(e)? {
        Ok(())
    } else {
        Err("Failed".into())
    }
}

#[tauri::command]
pub async fn sign_out_others(name: String) -> R<String> {
    let (app, a, csrf) = account_and_csrf(&name, true).await?;
    let r = app.api.sign_out_other_sessions(&a.security_token, &csrf).await.map_err(e)?;
    if !r.ok() {
        return Err("Failed to log out of other sessions!".into());
    }
    match r.cookie {
        Some(c) => {
            app.update_account(&a.username, |acc| acc.security_token = c.clone());
            app.save_accounts(true);
            Ok("Signed out of all other sessions!".into())
        }
        None => Ok("An error occured, you will need to re-login!".into()),
    }
}
