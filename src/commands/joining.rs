//! Joining games, following players, launching several accounts.

use super::*;

/// Place box text to an id: game links, recent game names and stray characters are fine
pub(crate) fn parse_place(text: &str, job: &str) -> (Option<i64>, String) {
    let link = regex::Regex::new(r"/games/(\d+)[/|?]?").unwrap();
    let mut job = job.to_string();
    if text.contains("privateServerLinkCode") && link.is_match(text) {
        job = text.to_string();
    }
    // a private server share link has no place in it: Roblox gives it when joining
    if crate::roblox::server_share_code(text).is_some() {
        return (None, text.trim().to_string());
    }
    let place = match link.captures(text) {
        Some(c) => c[1].to_string(),
        None => {
            let recent = app().recent_games.lock().unwrap().iter().find(|g| g["Details"]["filteredName"].as_str() == Some(text)).and_then(|g| g["Details"]["placeId"].as_i64());
            recent.map(|r| r.to_string()).unwrap_or_else(|| text.chars().filter(|c| c.is_ascii_digit()).collect())
        }
    };
    (place.parse().ok(), job)
}

#[tauri::command]
pub async fn join(names: Vec<String>, place: String, job: String, privacy_ok: bool) -> R<JoinResult> {
    let app = app();
    let (place_id, job) = parse_place(&place, &job);
    // with a share link the place comes from Roblox when joining
    let share = crate::roblox::server_share_code(&job).is_some();
    let place_id = place_id.or(share.then_some(0)).ok_or("Invalid PlaceId provided")?;
    let vip = job.len() > 4 && job.starts_with("VIP:");
    let job_id = if vip { job[4..].to_string() } else { job };

    if place_id > 0 {
        add_recent_game(app.clone(), place_id);
        app.set_setting("General", "SavedPlaceId", &place_id.to_string());
    }

    let accounts = app.accounts_named(&names);
    let opts = JoinOptions { place_id, job_id, join_vip: vip, privacy_ok, ..app.join_options() };

    if accounts.len() > 1 {
        // each account joins its own saved game, except with "Same for all" or a share link someone sent
        let use_saved = !share && !app.setting_bool("General", "SameGameForAll");
        launch_many(app, accounts, opts, use_saved);
        return Ok(JoinResult::Success);
    }

    let account = accounts.into_iter().next().ok_or("No Account Selected!")?;
    Ok(join_one(&app, &account, opts).await)
}

pub(crate) async fn join_one(app: &Arc<App>, account: &Account, opts: JoinOptions) -> JoinResult {
    let tracker = app.ensure_tracker(&account.username);
    let result = launcher::join(&app.api, &account.security_token, &tracker, opts).await;
    app.launch_next.store(true, Ordering::SeqCst);

    if result == JoinResult::Success {
        app.update_account(&account.username, |a| a.last_use = crate::core::netdate::NetDate::now());
        app.save_accounts(false);
        if app.setting_bool("Watcher", "SaveWindowPositions") {
            if let Some(rect) = window_rect(account) {
                tauri::async_runtime::spawn(launcher::restore_window_position(tracker, rect));
            }
        }
    }
    result
}

/// Launches several accounts in the background, the interface shows the progress
pub(super) fn launch_many(app: Arc<App>, accounts: Vec<Account>, opts: JoinOptions, use_saved: bool) {
    tauri::async_runtime::spawn(async move {
        let total = accounts.len();
        app.emit("launching", json!({ "total": total, "done": 0 }));
        let a = app.clone();
        launch_sequence(&app, &accounts, opts, use_saved, move |i, account, result| {
            if let JoinResult::Error(err) = result {
                a.toast("error", format!("{}: {}", account.username, err));
            }
            a.emit("launching", json!({ "total": total, "done": i + 1 }));
        })
        .await;
        app.emit("launching", json!({ "total": 0, "done": 0 }));
    });
}

/// Launches accounts one after the other (delay or "async launching"). With `use_saved` each account's
/// saved place wins over `opts`. `done` hears about every account. A new launch or "cancel" stops it.
pub(crate) async fn launch_sequence(app: &Arc<App>, accounts: &[Account], opts: JoinOptions, use_saved: bool, mut done: impl FnMut(usize, &Account, &JoinResult)) {
    let generation = app.launch_generation.fetch_add(1, Ordering::SeqCst) + 1;
    let delay = app.setting_i64("General", "AccountJoinDelay", 8).max(0) as u64;
    let async_join = app.setting_bool("General", "AsyncJoin");

    for (i, account) in accounts.iter().enumerate() {
        if app.launch_generation.load(Ordering::SeqCst) != generation {
            break;
        }

        let mut o = opts.clone();
        if use_saved && !o.follow_user {
            if let Ok(p) = account.field("SavedPlaceId").parse::<i64>() {
                o.place_id = p;
            }
            if !account.field("SavedJobId").is_empty() {
                o.job_id = account.field("SavedJobId").to_string();
            }
        }
        o.privacy_ok = true;


        app.launch_next.store(false, Ordering::SeqCst);
        let result = join_one(app, account, o).await;
        done(i, account, &result);

        if i + 1 < accounts.len() {
            if async_join {
                while !app.launch_next.load(Ordering::SeqCst) && app.launch_generation.load(Ordering::SeqCst) == generation {
                    tokio::time::sleep(Duration::from_millis(50)).await;
                }
            } else {
                tokio::time::sleep(Duration::from_secs(delay)).await;
            }
        }
    }
}

#[tauri::command]
pub fn cancel_launch() {
    app().launch_generation.fetch_add(1, Ordering::SeqCst);
}

/// Whether a player is in a game, before following them
#[tauri::command]
pub async fn follow_check(username: String) -> R<Value> {
    let app = app();
    let token = app.any_token();
    let user_id = app.api.user_id(username.trim(), token.as_deref()).await.map_err(e)?;
    app.set_setting("General", "SavedFollowUser", username.trim());
    let presence = app.api.presence(&[user_id]).await.map_err(e)?;
    let in_game = presence.first().map(|p| p.kind == 2 && p.place_id.unwrap_or(0) > 0).unwrap_or(false);
    Ok(json!({ "userId": user_id, "inGame": in_game }))
}

#[tauri::command]
pub async fn follow(names: Vec<String>, user_id: i64) -> R<JoinResult> {
    let app = app();
    let accounts = app.accounts_named(&names);
    let opts = JoinOptions { place_id: user_id, follow_user: true, privacy_ok: true, ..app.join_options() };
    if accounts.len() > 1 {
        launch_many(app, accounts, opts, true);
        return Ok(JoinResult::Success);
    }
    let account = accounts.into_iter().next().ok_or("No Account Selected!")?;
    Ok(join_one(&app, &account, opts).await)
}

#[tauri::command]
pub async fn allow_private_servers(name: String) -> R<bool> {
    let app = app();
    let a = app.account(&name).ok_or("No Account Selected!")?;
    app.pin_ok(&a).await.map_err(e)?;
    let csrf = app.csrf(&a).await.map_err(e)?;
    app.api.set_private_server_privacy_everyone(&a.security_token, &csrf).await.map_err(e)
}

/// The game's name and icon for the game panel, both asked at the same time
#[tauri::command]
pub async fn place_info(place: String) -> Option<Value> {
    let (id, _) = parse_place(&place, "");
    let ids = [id?];
    let id = ids[0];
    let app = app();
    let (name, mut icons) = tokio::join!(app.api.place_name(id), super::games::place_icons(&app, &ids));
    let name = name.ok().flatten()?;
    Some(json!({ "name": name, "icon": icons.remove(&id) }))
}

#[tauri::command]
pub fn set_shuffle(on: bool) {
    let app = app();
    app.shuffle.store(on, Ordering::SeqCst);
    app.set_setting("General", "ShuffleJobId", if on { "true" } else { "false" });
}
