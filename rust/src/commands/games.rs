//! Recent games, favorites, game search and universes.

use super::*;

pub(super) async fn place_icons(app: &App, ids: &[i64]) -> std::collections::HashMap<i64, String> {
    if app.setting_bool("Developer", "DisableImages") || app.setting_bool("General", "DisableImages") {
        return Default::default();
    }
    let requests: Vec<(i64, &str, &str)> = ids.iter().map(|i| (*i, "PlaceIcon", "150x150")).collect();
    app.api.thumbnails(&requests).await.unwrap_or_default().into_iter().collect()
}

pub(super) fn add_recent_game(app: Arc<App>, place_id: i64) {
    tauri::async_runtime::spawn(async move {
        let name = app.api.place_name(place_id).await.ok().flatten().unwrap_or_else(|| "Unknown".into());
        let icon = place_icons(&app, &[place_id]).await.remove(&place_id).unwrap_or_default();
        let filtered = filter_name(&name);
        {
            let mut list = app.recent_games.lock().unwrap();
            list.retain(|g| g["Details"]["placeId"].as_i64() != Some(place_id));
            list.push(json!({ "Details": { "filteredName": filtered, "name": name, "placeId": place_id }, "ImageUrl": icon }));
            let max = app.setting_i64("General", "MaxRecentGames", 8).max(1) as usize;
            while list.len() > max {
                list.remove(0);
            }
        }
        app.save_recent_games();
        app.emit("recent-games", recent_list(&app));
    });
}

pub(super) fn filter_name(name: &str) -> String {
    let no_tags = regex::Regex::new(r"[\s]?\[[^]]*\][\s]?").unwrap().replace_all(name, "");
    regex::Regex::new(r"[^a-zA-Z0-9% ._]").unwrap().replace_all(&no_tags, "").to_string()
}

pub(super) fn recent_list(app: &App) -> Vec<Value> {
    app.recent_games
        .lock()
        .unwrap()
        .iter()
        .rev()
        .map(|g| json!({ "placeId": g["Details"]["placeId"], "name": g["Details"]["name"].as_str().or(g["Details"]["filteredName"].as_str()).unwrap_or("?"), "icon": g["ImageUrl"] }))
        .collect()
}

#[tauri::command]
pub fn recent_games() -> Vec<Value> {
    recent_list(&app())
}

#[tauri::command]
pub async fn search_games(term: String, page: i64) -> R<Vec<Value>> {
    let app = app();
    let games = app.api.search_games(&term, page).await.map_err(e)?;
    let mut list: Vec<Value> = games
        .iter()
        .map(|g| {
            let up = g["totalUpVotes"].as_f64().unwrap_or(0.0);
            let down = g["totalDownVotes"].as_f64().unwrap_or(0.0);
            json!({
                "placeId": g["rootPlaceId"].as_i64().or(g["placeId"].as_i64()).unwrap_or(0),
                "name": g["name"].as_str().unwrap_or(""),
                "players": g["playerCount"].as_i64().unwrap_or(0),
                "likes": if up > 0.0 { ((up / (up + down)) * 100.0) as i64 } else { 0 },
                "icon": "",
            })
        })
        .filter(|g| g["placeId"].as_i64().unwrap_or(0) > 0)
        .collect();
    let ids: Vec<i64> = list.iter().filter_map(|g| g["placeId"].as_i64()).collect();
    let icons = place_icons(&app, &ids).await;
    for g in list.iter_mut() {
        if let Some(url) = g["placeId"].as_i64().and_then(|id| icons.get(&id)) {
            g["icon"] = json!(url);
        }
    }
    Ok(list)
}

#[tauri::command]
pub fn favorites() -> Vec<Value> {
    app()
        .favorites
        .lock()
        .unwrap()
        .iter()
        .enumerate()
        .map(|(i, f)| json!({ "index": i, "name": f["Name"], "placeId": f["PlaceID"].as_i64().or(f["Details"]["placeId"].as_i64()), "privateServer": f["PrivateServer"], "icon": f["ImageUrl"] }))
        .collect()
}

#[tauri::command]
pub async fn add_favorite(place: String, job: String, name: Option<String>) -> R<()> {
    let app = app();
    let (id, job) = parse_place(&place, &job);
    let mut place_id = id.ok_or("Invalid PlaceId provided")?;
    if let Some(c) = regex::Regex::new(r"/games/(\d+)/").unwrap().captures(&job) {
        place_id = c[1].parse().unwrap_or(place_id);
    }
    let vip = job.contains("privateServerLinkCode");
    let mut title = match name {
        Some(n) => n,
        None => app.api.place_name(place_id).await.map_err(e)?.ok_or_else(|| format!("Can't add {place_id} to favorites!"))?,
    };
    if vip {
        title = format!("{title} (VIP)");
    }
    let icon = place_icons(&app, &[place_id]).await.remove(&place_id).unwrap_or_default();

    {
        let mut list = app.favorites.lock().unwrap();
        if !vip && list.iter().any(|f| f["PlaceID"].as_i64() == Some(place_id) && f["PrivateServer"].as_str().unwrap_or("").is_empty()) {
            return Ok(());
        }
        list.push(json!({ "Name": title, "PrivateServer": if vip { job } else { String::new() }, "PlaceID": place_id, "Details": { "filteredName": filter_name(&title), "name": title, "placeId": place_id }, "ImageUrl": icon }));
    }
    app.save_favorites();
    Ok(())
}

#[tauri::command]
pub fn rename_favorite(index: usize, name: String) {
    let app = app();
    if let Some(f) = app.favorites.lock().unwrap().get_mut(index) {
        f["Name"] = json!(name);
    }
    app.save_favorites();
}

#[tauri::command]
pub fn remove_favorite(index: usize) {
    let app = app();
    {
        let mut list = app.favorites.lock().unwrap();
        if index < list.len() {
            list.remove(index);
        }
    }
    let _ = std::fs::write(app.dir.join("FavoriteGames.json"), serde_json::to_string(&*app.favorites.lock().unwrap()).unwrap_or_default());
}

#[tauri::command]
pub async fn universe_id(place: String) -> R<i64> {
    let app = app();
    let (id, _) = parse_place(&place, "");
    let details = app.api.place_details(&[id.ok_or("Invalid PlaceId provided")?], app.any_token().as_deref()).await.map_err(e)?;
    details.first().and_then(|d| d["universeId"].as_i64()).ok_or_else(|| "Not found".into())
}

#[tauri::command]
pub async fn universe_places(universe: i64) -> R<Vec<Value>> {
    let app = app();
    let places = app.api.universe_places(universe).await.map_err(e)?;
    let icons = place_icons(&app, &places.iter().map(|p| p.0).collect::<Vec<_>>()).await;
    Ok(places.into_iter().map(|(id, name)| json!({ "placeId": id, "name": name, "icon": icons.get(&id) })).collect())
}
