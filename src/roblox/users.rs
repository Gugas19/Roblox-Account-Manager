//! Users: ids, profiles, presence and Robux.

use super::*;

impl Api {
    // ---------- users ----------

    pub async fn user_id(&self, username: &str, token: Option<&str>) -> Result<i64> {
        let r = self.send(self.request(Method::POST, "https://users.roblox.com/v1/usernames/users", token).json(&json!({ "usernames": [username] }))).await?;
        r.json()
            .and_then(|v| v["data"][0]["id"].as_i64())
            .ok_or_else(|| anyhow!("Failed to get UserId: [{}] {}", r.status, r.body))
    }

    pub async fn user_info(&self, user_id: i64, token: Option<&str>) -> Result<Value> {
        let r = self.get(&format!("https://users.roblox.com/v1/users/{user_id}"), token).await?;
        r.json().ok_or_else(|| anyhow!("[{}] {}", r.status, r.body))
    }

    pub async fn mobile_info(&self, token: &str) -> Result<Option<Value>> {
        let r = self.get("https://www.roblox.com/mobileapi/userinfo", Some(token)).await?;
        Ok(r.json().filter(|_| r.status == 200))
    }

    pub async fn email_info(&self, token: &str) -> Result<String> {
        Ok(self.get("https://accountsettings.roblox.com/v1/email", Some(token)).await?.body)
    }

    pub async fn robux(&self, token: &str, user_id: i64) -> Result<i64> {
        let r = self.get(&format!("https://economy.roblox.com/v1/users/{user_id}/currency"), Some(token)).await?;
        if let Some(robux) = r.json().and_then(|v| v["robux"].as_i64()) {
            return Ok(robux);
        }
        Ok(self.mobile_info(token).await?.and_then(|v| v["RobuxBalance"].as_i64()).unwrap_or(0))
    }

    pub async fn presence(&self, user_ids: &[i64]) -> Result<Vec<Presence>> {
        let mut all = Vec::new();
        for chunk in user_ids.chunks(50) {
            let r = self.send(self.request(Method::POST, "https://presence.roblox.com/v1/presence/users", None).json(&json!({ "userIds": chunk }))).await?;
            if let Some(list) = r.json().and_then(|v| v["userPresences"].as_array().cloned()) {
                all.extend(list.into_iter().filter_map(|p| serde_json::from_value::<Presence>(p).ok()));
            }
        }
        Ok(all)
    }

    pub async fn headshot(&self, user_id: i64) -> Result<Option<String>> {
        let r = self.get(&format!("https://thumbnails.roblox.com/v1/users/avatar-headshot?size=48x48&format=png&userIds={user_id}"), None).await?;
        Ok(r.json().and_then(|v| v["data"][0]["imageUrl"].as_str().map(str::to_string)))
    }
}
