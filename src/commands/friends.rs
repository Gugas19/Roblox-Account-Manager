//! Friends page: an account's friends with where they are, its friend requests, and who can see or join it.

use super::*;
use crate::roblox::{FriendAction, Visibility, VISIBILITY_FIELDS};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FriendView {
    id: i64,
    name: String,
    display_name: String,
    /// 0 offline, 1 online, 2 in a game, 3 in Studio
    kind: i64,
    place_id: Option<i64>,
    job_id: Option<String>,
    /// the game's name when Roblox shares it
    location: Option<String>,
    avatar: Option<String>,
}

fn account(app: &App, name: &str) -> R<Account> {
    app.account(name).ok_or_else(|| "No Account Selected!".to_string())
}

/// The account's friends, the ones playing first, then in Studio, online, offline
#[tauri::command]
pub async fn friends_list(name: String) -> R<Vec<FriendView>> {
    let app = app();
    let a = account(&app, &name)?;
    let people = app.api.friends(&a.security_token, a.user_id).await.map_err(e)?;
    let ids: Vec<i64> = people.iter().map(|p| p.id).collect();
    // asked as the account itself: friends see each other's game and server
    let presence = match app.csrf(&a).await {
        Ok(csrf) => app.api.presence_as(&ids, &a.security_token, &csrf).await.unwrap_or_default(),
        Err(_) => app.api.presence(&ids).await.unwrap_or_default(),
    };
    let avatars = app.api.headshots(&ids).await;
    let mut list: Vec<FriendView> = people
        .into_iter()
        .map(|p| {
            let now = presence.iter().find(|x| x.user_id == p.id);
            let kind = now.map(|x| x.kind).unwrap_or(0);
            FriendView {
                id: p.id,
                avatar: avatars.get(&p.id).cloned(),
                kind,
                place_id: now.and_then(|x| x.place_id).filter(|id| *id > 0 && kind >= 2),
                job_id: now.and_then(|x| x.game_id.clone()).filter(|_| kind == 2),
                location: now.and_then(|x| x.last_location.clone()).filter(|l| kind >= 1 && !l.trim().is_empty()),
                name: p.name,
                display_name: p.display_name,
            }
        })
        .collect();
    let rank = |k: i64| match k {
        2 => 0,
        3 => 1,
        1 => 2,
        _ => 3,
    };
    list.sort_by(|x, y| rank(x.kind).cmp(&rank(y.kind)).then_with(|| x.display_name.to_lowercase().cmp(&y.display_name.to_lowercase())));
    Ok(list)
}

/// Friend requests the account received
#[tauri::command]
pub async fn friend_requests(name: String) -> R<Vec<FriendView>> {
    let app = app();
    let a = account(&app, &name)?;
    let people = app.api.friend_requests(&a.security_token).await.map_err(e)?;
    let avatars = app.api.headshots(&people.iter().map(|p| p.id).collect::<Vec<_>>()).await;
    Ok(people
        .into_iter()
        .map(|p| FriendView { id: p.id, avatar: avatars.get(&p.id).cloned(), kind: 0, place_id: None, job_id: None, location: None, name: p.name, display_name: p.display_name })
        .collect())
}

/// accept or decline a request, or remove a friend
#[tauri::command]
pub async fn friend_act(name: String, target: i64, action: String) -> R<()> {
    let action = match action.as_str() {
        "accept" => FriendAction::Accept,
        "decline" => FriendAction::Decline,
        "remove" => FriendAction::Remove,
        _ => return Err("Unknown action".into()),
    };
    let app = app();
    let a = account(&app, &name)?;
    let csrf = app.csrf(&a).await.map_err(e)?;
    app.api.friend_action(&a.security_token, &csrf, target, action).await.map_err(e)
}

/// Who can join the account and see it online, and what Roblox allows it
#[tauri::command]
pub async fn visibility_get(name: String) -> R<Vec<Visibility>> {
    let app = app();
    let a = account(&app, &name)?;
    app.api.visibility(&a.security_token).await.map_err(e)
}

/// Changes it for each account; for each one, none when it worked, the reason otherwise
#[tauri::command]
pub async fn visibility_set(names: Vec<String>, field: String, value: String) -> R<Vec<(String, Option<String>)>> {
    if !VISIBILITY_FIELDS.contains(&field.as_str()) {
        return Err("Unknown setting".into());
    }
    let app = app();
    let mut results = Vec::new();
    for a in app.accounts_named(&names) {
        let outcome = async {
            let csrf = app.csrf(&a).await?;
            app.api.set_visibility(&a.security_token, &csrf, &field, &value).await
        }
        .await;
        results.push((a.username.clone(), outcome.err().map(|e| e.to_string())));
    }
    Ok(results)
}
