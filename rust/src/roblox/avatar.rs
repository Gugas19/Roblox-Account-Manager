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

    pub async fn outfits(&self, user_id: i64) -> Result<Vec<(String, i64)>> {
        let r = self.get(&format!("https://avatar.roblox.com/v1/users/{user_id}/outfits?page=1&itemsPerPage=50"), None).await?;
        Ok(r.json()
            .and_then(|v| v["data"].as_array().cloned())
            .unwrap_or_default()
            .iter()
            .map(|o| (o["name"].as_str().unwrap_or("").to_string(), o["id"].as_i64().unwrap_or(0)))
            .collect())
    }

    pub async fn outfit_details(&self, outfit_id: i64) -> Result<Value> {
        let r = self.get(&format!("https://avatar.roblox.com/v1/outfits/{outfit_id}/details"), None).await?;
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
