//! Avatars, outfits and buying items.

use super::*;

impl Api {
    // ---------- avatar ----------

    /// Wears an avatar JSON (same format the C# "Wear JSON" took), returns the asset ids that couldn't be worn
    pub async fn set_avatar(&self, token: &str, csrf: &str, avatar: &Value) -> Result<Vec<i64>> {
        let api = "https://avatar.roblox.com/";

        if let Some(kind) = avatar.get("playerAvatarType").and_then(|v| v.as_str()) {
            self.send(self.with_csrf(Method::POST, &format!("{api}v1/avatar/set-player-avatar-type"), token, csrf).json(&json!({ "playerAvatarType": kind }))).await?;
        }

        if let Some(scales) = avatar.get("scales").or_else(|| avatar.get("scale")) {
            self.send(self.with_csrf(Method::POST, &format!("{api}v1/avatar/set-scales"), token, csrf).json(scales)).await?;
        }

        if let Some(colors) = avatar.get("bodyColors") {
            self.send(self.with_csrf(Method::POST, &format!("{api}v1/avatar/set-body-colors"), token, csrf).json(colors)).await?;
        }

        if let Some(assets) = avatar.get("assets") {
            let r = self.send(self.with_csrf(Method::POST, &format!("{api}v2/avatar/set-wearing-assets"), token, csrf).json(&json!({ "assets": assets }))).await?;
            if r.ok() {
                if let Some(invalid) = r.json().and_then(|v| v["invalidAssetIds"].as_array().cloned()) {
                    return Ok(invalid.iter().filter_map(|v| v.as_i64()).collect());
                }
            }
        }

        Ok(Vec::new())
    }

    /// A player's saved outfits. Asked with an account's cookie when there is one: Roblox limits these requests
    /// hard without one (429 after a few). The current route first, the old one if it fails.
    pub async fn outfits(&self, user_id: i64, token: Option<&str>) -> Result<Vec<(String, i64)>> {
        let current = self.get(&format!("https://avatar.roblox.com/v2/avatar/users/{user_id}/outfits?page=1&itemsPerPage=50"), token).await?;
        let r = match current.checked("the outfits") {
            Ok(r) => r,
            Err(_) => self.get(&format!("https://avatar.roblox.com/v1/users/{user_id}/outfits?page=1&itemsPerPage=50"), token).await?.checked("the outfits")?,
        };
        Ok(outfit_list(&r.json().unwrap_or_default()))
    }

    pub async fn outfit_details(&self, outfit_id: i64, token: Option<&str>) -> Result<Value> {
        let r = self.get(&format!("https://avatar.roblox.com/v1/outfits/{outfit_id}/details"), token).await?.checked("the outfit")?;
        r.json().ok_or_else(|| anyhow!("[{}] {}", r.status, r.body))
    }

    pub async fn purchase(&self, token: &str, csrf: &str, product_id: i64, price: i64, seller_id: i64) -> Result<Value> {
        let r = self
            .send(
                self.with_csrf(Method::POST, &format!("https://economy.roblox.com/v1/purchases/products/{product_id}"), token, csrf)
                    .json(&json!({ "expectedCurrency": 1, "expectedPrice": price, "expectedSellerId": seller_id })),
            )
            .await?;
        r.json().ok_or_else(|| anyhow!("[{}] {}", r.status, r.body))
    }
}

/// Outfits as both routes give them: `data` of { id, name }, the ones without an id left out
fn outfit_list(v: &Value) -> Vec<(String, i64)> {
    v["data"].as_array().into_iter().flatten().filter_map(|o| Some((o["name"].as_str().unwrap_or("").to_string(), o["id"].as_i64().filter(|id| *id > 0)?))).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outfits_from_either_route() {
        let v = json!({ "data": [{ "id": 5, "name": "Blue", "isEditable": true }, { "name": "no id" }], "paginationToken": "" });
        assert_eq!(outfit_list(&v), vec![("Blue".to_string(), 5)]);
        assert!(outfit_list(&json!({ "errors": [{ "code": 0 }] })).is_empty());
    }

    #[test]
    fn a_refusal_is_said_in_words() {
        let raw = |status| Raw { status, body: String::new(), headers: HeaderMap::new(), cookie: None };
        assert!(raw(200).checked("x").is_ok());
        assert!(raw(429).checked("x").err().unwrap().to_string().contains("limiting"));
        assert!(raw(500).checked("the outfits").err().unwrap().to_string().contains("(500)"));
    }
}
