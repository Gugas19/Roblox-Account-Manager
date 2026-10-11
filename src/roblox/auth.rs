//! Sessions, authentication tickets, the PIN and Quick Log In.

use super::*;

impl Api {
    // ---------- session ----------

    /// The account behind a cookie
    pub async fn authenticated_user(&self, token: &str) -> Result<UserInfo> {
        let r = self.get("https://users.roblox.com/v1/users/authenticated", Some(token)).await?;
        if r.ok() {
            if let Ok(user) = serde_json::from_str::<UserInfo>(&r.body) {
                if user.id > 0 {
                    return Ok(user);
                }
            }
        }

        // older endpoint the C# version used
        let r = self.get("https://www.roblox.com/my/account/json", Some(token)).await?;
        let v = r.json().ok_or_else(|| anyhow!("Invalid Account Session!"))?;
        let id = v["UserId"].as_i64().ok_or_else(|| anyhow!("Invalid Account Session!"))?;
        Ok(UserInfo { id, name: v["Name"].as_str().unwrap_or("").to_string(), display_name: v["DisplayName"].as_str().unwrap_or("").to_string() })
    }

    /// x-csrf-token for a session, None when the cookie isn't valid anymore
    pub async fn csrf(&self, token: &str) -> Result<Option<String>> {
        let r = self
            .send(self.request(Method::POST, "https://auth.roblox.com/v1/authentication-ticket/", Some(token)).header(REFERER, GAME_REFERER).header("Content-Type", "application/json"))
            .await?;

        if r.status != 403 {
            return Ok(None);
        }

        Ok(r.headers.get("x-csrf-token").and_then(|v| v.to_str().ok()).map(str::to_string).filter(|t| !t.is_empty()))
    }

    pub async fn auth_ticket(&self, token: &str, csrf: &str) -> Result<Option<String>> {
        let r = self
            .send(
                self.request(Method::POST, "https://auth.roblox.com/v1/authentication-ticket/", Some(token))
                    .header("X-CSRF-TOKEN", csrf)
                    .header(REFERER, GAME_REFERER)
                    .header("Content-Type", "application/json"),
            )
            .await?;

        Ok(r.headers.get("rbx-authentication-ticket").and_then(|v| v.to_str().ok()).map(str::to_string))
    }

    pub(crate) fn with_csrf(&self, method: Method, url: &str, token: &str, csrf: &str) -> RequestBuilder {
        self.request(method, url, Some(token)).header("X-CSRF-TOKEN", csrf)
    }

    // ---------- PIN ----------

    /// true when the account has no PIN or it's unlocked
    pub async fn pin_unlocked(&self, token: &str) -> Result<bool> {
        let r = self.send(self.request(Method::GET, "https://auth.roblox.com/v1/account/pin/", Some(token)).header(REFERER, "https://www.roblox.com/")).await?;
        if let Some(v) = r.json().filter(|_| r.ok()) {
            let enabled = v["isEnabled"].as_bool().unwrap_or(false);
            let until = v["unlockedUntil"].as_i64().unwrap_or(0);
            return Ok(!enabled || until > 0);
        }
        Ok(false)
    }

    /// Seconds the PIN stays unlocked
    pub async fn unlock_pin(&self, token: &str, csrf: &str, pin: &str) -> Result<Option<i64>> {
        let r = self
            .send(self.with_csrf(Method::POST, "https://auth.roblox.com/v1/account/pin/unlock", token, csrf).header(REFERER, "https://www.roblox.com/").form(&[("pin", pin)]))
            .await?;
        if let Some(v) = r.json().filter(|_| r.ok()) {
            if v["isEnabled"].as_bool().unwrap_or(false) {
                return Ok(v["unlockedUntil"].as_i64().filter(|s| *s > 0));
            }
        }
        Ok(None)
    }

    // ---------- quick log in ----------

    pub async fn quick_login_info(&self, token: &str, csrf: &str, code: &str) -> Result<Option<Value>> {
        let r = self.send(self.with_csrf(Method::POST, "https://apis.roblox.com/auth-token-service/v1/login/enterCode", token, csrf).json(&json!({ "code": code }))).await?;
        Ok(r.json().filter(|_| r.ok()))
    }

    pub async fn quick_login_validate(&self, token: &str, csrf: &str, code: &str) -> Result<bool> {
        let r = self.send(self.with_csrf(Method::POST, "https://apis.roblox.com/auth-token-service/v1/login/validateCode", token, csrf).json(&json!({ "code": code }))).await?;
        Ok(r.ok())
    }
}
