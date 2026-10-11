//! Games, servers, places, universes and thumbnails.

use super::*;

impl Api {
    // ---------- games & servers ----------

    pub async fn place_name(&self, place_id: i64) -> Result<Option<String>> {
        let r = self.send(self.request(Method::GET, &format!("https://economy.roblox.com/v2/assets/{place_id}/details"), None).header("Accept", "application/json")).await?;
        Ok(r.json().filter(|_| r.status == 200).and_then(|v| v["Name"].as_str().map(str::to_string)))
    }

    pub async fn asset_details(&self, asset_id: i64) -> Result<Value> {
        let r = self.send(self.request(Method::GET, &format!("https://economy.roblox.com/v2/assets/{asset_id}/details"), None).header("Accept", "application/json")).await?;
        r.json().filter(|_| r.ok()).ok_or_else(|| anyhow!("[{}] {}", r.status, r.body))
    }

    /// One page of public (or the account's VIP) servers
    pub async fn servers(&self, place_id: i64, cursor: &str, vip_token: Option<&str>) -> Result<Value> {
        let url = match vip_token {
            Some(_) => format!("https://games.roblox.com/v1/games/{place_id}/servers/VIP?sortOrder=Asc&limit=25{}", if cursor.is_empty() { String::new() } else { format!("&cursor={cursor}") }),
            None => format!("https://games.roblox.com/v1/games/{place_id}/servers/public?sortOrder=Asc&limit=100{}", if cursor.is_empty() { String::new() } else { format!("&cursor={cursor}") }),
        };
        let r = self.send(self.request(Method::GET, &url, vip_token).header("Accept", "application/json")).await?;
        if r.status == 429 {
            bail!("Too many requests, wait a moment and try again");
        }
        r.json().filter(|_| r.ok()).ok_or_else(|| anyhow!("[{}] {}", r.status, r.body))
    }

    /// A random server with free slots (port of Utilities.GetRandomJobId)
    pub async fn random_job_id(&self, place_id: i64, lowest: bool, pages: i64) -> Result<String> {
        let mut valid = Vec::new();
        let mut cursor = String::new();

        for _ in 0..pages.max(1) {
            let page = match self.servers(place_id, &cursor, None).await {
                Ok(p) => p,
                Err(_) => break,
            };

            for s in page["data"].as_array().cloned().unwrap_or_default() {
                let playing = s["playing"].as_i64().unwrap_or(0);
                let max = s["maxPlayers"].as_i64().unwrap_or(0);
                if playing != max && playing > 0 && max > 1 {
                    if let Some(id) = s["id"].as_str() {
                        valid.push(id.to_string());
                    }
                }
            }

            cursor = page["nextPageCursor"].as_str().unwrap_or("").to_string();
            if cursor.is_empty() || lowest {
                break;
            }
        }

        if valid.is_empty() {
            return Ok(String::new());
        }
        if lowest {
            return Ok(valid[0].clone());
        }
        use rand::seq::SliceRandom;
        Ok(valid.choose(&mut rand::thread_rng()).cloned().unwrap_or_default())
    }

    /// join-game-instance: makes an already running client move to a server, also gives a server's address
    pub async fn join_game_instance(&self, token: &str, place_id: i64, job_id: &str, teleport: bool) -> Result<Raw> {
        let mut body = json!({ "gameId": job_id, "placeId": place_id });
        if teleport {
            body["isTeleport"] = json!(true);
        }
        Ok(finish(self.game_join.post("https://gamejoin.roblox.com/v1/join-game-instance").header(COOKIE, cookie(token)).json(&body).send().await?).await)
    }

    pub async fn join_game(&self, token: &str, place_id: i64) -> Result<Raw> {
        Ok(finish(self.game_join.post("https://gamejoin.roblox.com/v1/join-game").header(COOKIE, cookie(token)).json(&json!({ "placeId": place_id })).send().await?).await)
    }

    /// Searches games by keyword, or the default sort without one
    pub async fn search_games(&self, keyword: &str, page: i64) -> Result<Vec<Value>> {
        if keyword.trim().is_empty() {
            let r = self.get(&format!("https://games.roblox.com/v1/games/list?model.startRows={}&model.maxRows=50", page * 50), None).await?;
            return Ok(r.json().and_then(|v| v["games"].as_array().cloned()).unwrap_or_default());
        }

        // the omni search API the Roblox site uses
        let url = format!(
            "https://apis.roblox.com/search-api/omni-search?searchQuery={}&sessionId={}&pageType=all&pageToken={}",
            urlencoding::encode(keyword),
            rand::random::<u32>(),
            if page > 0 { page.to_string() } else { String::new() }
        );
        let r = self.get(&url, None).await?;
        let mut games = Vec::new();
        if let Some(results) = r.json().and_then(|v| v["searchResults"].as_array().cloned()) {
            for group in results {
                if group["contentGroupType"].as_str() == Some("Game") {
                    games.extend(group["contents"].as_array().cloned().unwrap_or_default());
                }
            }
        }
        Ok(games)
    }

    pub async fn place_details(&self, place_ids: &[i64], token: Option<&str>) -> Result<Vec<Value>> {
        if place_ids.is_empty() {
            return Ok(Vec::new());
        }
        let ids: Vec<String> = place_ids.iter().map(|i| i.to_string()).collect();
        let r = self.get(&format!("https://games.roblox.com/v1/games/multiget-place-details?placeIds={}", ids.join("&placeIds=")), token).await?;
        Ok(r.json().and_then(|v| v.as_array().cloned()).unwrap_or_default())
    }

    pub async fn universe_places(&self, universe_id: i64) -> Result<Vec<(i64, String)>> {
        let mut places = Vec::new();
        let mut cursor = String::new();

        loop {
            let r = self.get(&format!("https://develop.roblox.com/v1/universes/{universe_id}/places?sortOrder=Asc&limit=100&cursor={cursor}"), None).await?;
            let v = r.json().ok_or_else(|| anyhow!("[{}] {}", r.status, r.body))?;
            for p in v["data"].as_array().cloned().unwrap_or_default() {
                places.push((p["id"].as_i64().unwrap_or(0), p["name"].as_str().unwrap_or("").to_string()));
            }
            cursor = v["nextPageCursor"].as_str().unwrap_or("").to_string();
            if cursor.is_empty() {
                break;
            }
        }

        Ok(places)
    }

    /// Thumbnail URLs for many targets in one request (type: GameIcon, Asset, Outfit, AvatarHeadShot, PlaceIcon...)
    pub async fn thumbnails(&self, requests: &[(i64, &str, &str)]) -> Result<Vec<(i64, String)>> {
        let mut out = Vec::new();
        for chunk in requests.chunks(100) {
            let body: Vec<Value> = chunk
                .iter()
                .map(|(id, kind, size)| json!({ "requestId": format!("{id}:undefined:{kind}:{size}:png:regular"), "type": kind, "targetId": id, "size": size, "format": "png" }))
                .collect();
            let r = self.send(self.request(Method::POST, "https://thumbnails.roblox.com/v1/batch", None).json(&body)).await?;
            for item in r.json().and_then(|v| v["data"].as_array().cloned()).unwrap_or_default() {
                if let (Some(id), Some(url)) = (item["targetId"].as_i64(), item["imageUrl"].as_str()) {
                    out.push((id, url.to_string()));
                }
            }
        }
        Ok(out)
    }

    /// Avatar headshots of the players in a server (they're only given as tokens)
    pub async fn token_headshots(&self, tokens: &[String]) -> Result<Vec<String>> {
        let body: Vec<Value> = tokens
            .iter()
            .map(|t| json!({ "requestId": format!("0:{t}:AvatarHeadshot:48x48:png:regular"), "type": "AvatarHeadShot", "targetId": 0, "token": t, "format": "png", "size": "48x48" }))
            .collect();
        let r = self.send(self.request(Method::POST, "https://thumbnails.roblox.com/v1/batch", None).json(&body)).await?;
        Ok(r.json()
            .and_then(|v| v["data"].as_array().cloned())
            .unwrap_or_default()
            .iter()
            .filter_map(|i| i["imageUrl"].as_str().map(str::to_string))
            .collect())
    }

    /// The access code of a private server link (the page embeds it)
    pub async fn private_server_access_code(&self, token: &str, csrf: &str, place_id: i64, link_code: &str) -> Result<Option<String>> {
        let url = format!("https://www.roblox.com/games/{place_id}?privateServerLinkCode={link_code}");
        let r = self.send(self.with_csrf(Method::GET, &url, token, csrf).header(REFERER, GAME_REFERER)).await?;
        let re = regex::Regex::new(r"Roblox\.GameLauncher\.joinPrivateGame\(\d+,\s*'(\w+-\w+-\w+-\w+-\w+)'").unwrap();
        Ok(re.captures(&r.body).map(|c| c[1].to_string()))
    }

    pub async fn client_version(&self) -> Result<String> {
        let r = self.get("https://clientsettings.roblox.com/v1/client-version/WindowsPlayer", None).await?;
        r.json().and_then(|v| v["clientVersionUpload"].as_str().map(str::to_string)).ok_or_else(|| anyhow!("no version"))
    }
}
