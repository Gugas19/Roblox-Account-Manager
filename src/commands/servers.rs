//! Server list, regions and finding a player.

use super::*;

#[tauri::command]
pub fn list_servers(place: String, other_place: String, account: Option<String>) -> R<u64> {
    let app = app();
    let (id, _) = parse_place(if other_place.trim().len() > 3 { &other_place } else { &place }, "");
    let place_id = id.ok_or("Invalid PlaceId provided")?;
    let generation = app.servers_generation.fetch_add(1, Ordering::SeqCst) + 1;
    *app.last_servers.lock().unwrap() = (place_id, Vec::new());
    let vip_token = account.and_then(|n| app.account(&n)).map(|a| a.security_token);

    tauri::async_runtime::spawn(async move {
        let app = crate::app::app();
        let mut cursor = String::new();
        loop {
            if app.servers_generation.load(Ordering::SeqCst) != generation {
                return;
            }
            match app.api.servers(place_id, &cursor, None).await {
                Ok(page) => {
                    let items: Vec<Value> = page["data"].as_array().cloned().unwrap_or_default().into_iter().map(|s| server_row(&s, "Public")).collect();
                    app.last_servers.lock().unwrap().1.extend(items.iter().filter_map(|s| s["id"].as_str().map(str::to_string)));
                    app.emit("servers", json!({ "generation": generation, "items": items, "done": false }));
                    cursor = page["nextPageCursor"].as_str().unwrap_or("").to_string();
                    if cursor.is_empty() {
                        break;
                    }
                }
                Err(err) => {
                    app.toast("error", err.to_string());
                    break;
                }
            }
        }

        if let Some(token) = vip_token {
            let mut cursor = String::new();
            for _ in 0..20 {
                let Ok(page) = app.api.servers(place_id, &cursor, Some(&token)).await else { break };
                let items: Vec<Value> = page["data"].as_array().cloned().unwrap_or_default().into_iter().map(|s| server_row(&s, "VIP")).collect();
                app.emit("servers", json!({ "generation": generation, "items": items, "done": false }));
                cursor = page["nextPageCursor"].as_str().unwrap_or("").to_string();
                if cursor.is_empty() {
                    break;
                }
            }
        }

        app.emit("servers", json!({ "generation": generation, "items": [], "done": true }));
    });

    Ok(generation)
}

pub(super) fn server_row(s: &Value, kind: &str) -> Value {
    json!({
        "id": if kind == "VIP" { s["name"].clone() } else { s["id"].clone() },
        "accessCode": s["accessCode"], "playing": s["playing"], "maxPlayers": s["maxPlayers"], "ping": s["ping"], "fps": s["fps"],
        "type": kind, "region": "", "playerTokens": s["playerTokens"],
    })
}

#[tauri::command]
pub fn stop_servers() {
    app().servers_generation.fetch_add(1, Ordering::SeqCst);
}

/// Joins a server from the list; with another place id it moves an already open client there (SetServer)
#[tauri::command]
pub async fn join_job(name: String, place: String, job: String, other_place: String) -> R<JoinResult> {
    let app = app();
    let account = app.account(&name).ok_or("No Account Selected!")?;
    if other_place.trim().len() > 3 {
        let place_id: i64 = other_place.trim().parse().map_err(|_| "Invalid PlaceId provided")?;
        let r = app.api.join_game_instance(&account.security_token, place_id, &job, false).await.map_err(e)?;
        return Ok(if r.status == 200 { JoinResult::Success } else { JoinResult::Error(format!("Failed {}: {}", r.status, r.body)) });
    }
    let (place_id, _) = parse_place(&place, "");
    Ok(join_one(&app, &account, JoinOptions { place_id: place_id.ok_or("Invalid PlaceId provided")?, job_id: job, privacy_ok: true, ..app.join_options() }).await)
}

/// Region and ping of servers (port of loadRegion)
#[tauri::command]
pub fn load_regions(name: Option<String>, place: String, ids: Vec<String>) -> R<()> {
    let app = app();
    let account = name.and_then(|n| app.account(&n)).or_else(|| app.store.lock().unwrap().accounts.first().cloned()).ok_or("No Account Selected!")?;
    let (current, _) = app.last_servers.lock().unwrap().clone();
    let (main_place, _) = parse_place(&place, "");
    let universe_place = main_place.map(|p| p != current && current > 0).unwrap_or(false);

    tauri::async_runtime::spawn(async move {
        let app = crate::app::app();
        if universe_place {
            let _ = app.api.join_game(&account.security_token, main_place.unwrap_or(0)).await;
        }
        let format_text = Some(app.setting("General", "ServerRegionFormat")).filter(|f| !f.is_empty()).unwrap_or_else(|| "<city>, <region_code>".into());
        let ip_api = Some(app.setting("General", "IPApiLink")).filter(|f| !f.is_empty()).unwrap_or_else(|| "http://ip-api.com/json/<ip>".into());

        for id in ids.into_iter().take(16) {
            let region = match app.api.join_game_instance(&account.security_token, current, &id, universe_place).await {
                Ok(r) if r.status == 200 => {
                    let js = r.json().unwrap_or_default();
                    let ip = js["joinScript"]["MachineAddress"].as_str().unwrap_or("").to_string();
                    if ip.is_empty() {
                        let status = js["status"].as_i64().unwrap_or(0);
                        let message = match status {
                            6 => "Server Full".to_string(),
                            11 => "Server no longer available".to_string(),
                            12 => "No Access".to_string(),
                            _ => status.to_string(),
                        };
                        app.emit("server-region", json!({ "id": id, "region": format!("{} {}", js["message"].as_str().unwrap_or("Error:"), message) }));
                        continue;
                    }
                    let info = app.api.get(&ip_api.replace("<ip>", &ip), None).await.ok().and_then(|r| r.json()).unwrap_or_default();
                    let mut text = format_text.clone();
                    for c in regex::Regex::new(r"<(\w+)>").unwrap().captures_iter(&format_text) {
                        let value = match &c[1] {
                            "address" => ip.clone(),
                            "port" => js["joinScript"]["ServerPort"].to_string(),
                            key => info[key].as_str().map(str::to_string).or_else(|| info[key].as_f64().map(|n| n.to_string())).unwrap_or_else(|| "?".into()),
                        };
                        text = text.replace(&c[0], &value);
                    }
                    let ping = tokio::task::spawn_blocking(move || crate::platform::ping::ping(&ip, 400)).await.ok().flatten();
                    json!({ "id": id, "region": text, "ping": ping })
                }
                Ok(r) => json!({ "id": id, "region": r.status.to_string() }),
                Err(err) => json!({ "id": id, "region": err.to_string() }),
            };
            app.emit("server-region", region);
        }
    });
    Ok(())
}

/// Finds which server a player is in by comparing avatar headshots
#[tauri::command]
pub fn find_player(username: String, place: String, other_place: String) -> R<u64> {
    let app = app();
    let (id, _) = parse_place(if other_place.trim().len() > 3 { &other_place } else { &place }, "");
    let place_id = id.ok_or("Invalid PlaceId provided")?;
    let generation = app.servers_generation.fetch_add(1, Ordering::SeqCst) + 1;

    tauri::async_runtime::spawn(async move {
        let app = crate::app::app();
        let found = async {
            let user_id = app.api.user_id(username.trim(), app.any_token().as_deref()).await?;
            let headshot = app.api.headshot(user_id).await?.ok_or_else(|| anyhow::anyhow!("Failed to get AvatarUrl of {username}"))?;
            let mut cursor = String::new();
            loop {
                if app.servers_generation.load(Ordering::SeqCst) != generation {
                    return Ok(None);
                }
                let page = app.api.servers(place_id, &cursor, None).await?;
                for s in page["data"].as_array().cloned().unwrap_or_default() {
                    let tokens: Vec<String> = s["playerTokens"].as_array().cloned().unwrap_or_default().iter().filter_map(|t| t.as_str().map(str::to_string)).collect();
                    if app.api.token_headshots(&tokens).await.unwrap_or_default().contains(&headshot) {
                        return Ok(Some(server_row(&s, "Public")));
                    }
                }
                cursor = page["nextPageCursor"].as_str().unwrap_or("").to_string();
                if cursor.is_empty() {
                    return Ok(None);
                }
                tokio::time::sleep(Duration::from_millis(750)).await;
            }
        }
        .await;

        match found {
            Ok(server) => app.emit("player-found", json!({ "generation": generation, "server": server })),
            Err(err) => {
                let err: anyhow::Error = err;
                app.emit("player-found", json!({ "generation": generation, "error": err.to_string() }))
            }
        }
    });
    Ok(generation)
}
