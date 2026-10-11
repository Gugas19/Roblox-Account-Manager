//! Account settings: password, email, privacy, display name, friends and blocks.

use super::*;

impl Api {
    // ---------- account settings ----------

    pub async fn set_follow_privacy(&self, token: &str, csrf: &str, privacy: usize) -> Result<bool> {
        let value = ["All", "Followers", "Following", "Friends", "NoOne"].get(privacy).copied().unwrap_or("All");
        let r = self
            .send(
                self.with_csrf(Method::POST, "https://www.roblox.com/account/settings/follow-me-privacy", token, csrf)
                    .header(REFERER, "https://www.roblox.com/my/account")
                    .form(&[("FollowMePrivacy", value)]),
            )
            .await?;
        Ok(r.ok())
    }

    /// Returns the new cookie Roblox gives after the change
    pub async fn change_password(&self, token: &str, csrf: &str, current: &str, new: &str) -> Result<Raw> {
        self.send(
            self.with_csrf(Method::POST, "https://auth.roblox.com/v2/user/passwords/change", token, csrf)
                .header(REFERER, "https://www.roblox.com/")
                .form(&[("currentPassword", current), ("newPassword", new)]),
        )
        .await
    }

    pub async fn change_email(&self, token: &str, csrf: &str, password: &str, email: &str) -> Result<bool> {
        let r = self
            .send(
                self.with_csrf(Method::POST, "https://accountsettings.roblox.com/v1/email", token, csrf)
                    .header(REFERER, "https://www.roblox.com/")
                    .form(&[("password", password), ("emailAddress", email)]),
            )
            .await?;
        Ok(r.ok())
    }

    pub async fn sign_out_other_sessions(&self, token: &str, csrf: &str) -> Result<Raw> {
        self.send(
            self.with_csrf(Method::POST, "https://www.roblox.com/authentication/signoutfromallsessionsandreauthenticate", token, csrf)
                .header(REFERER, "https://www.roblox.com/")
                .header("Content-Type", "application/x-www-form-urlencoded"),
        )
        .await
    }

    pub async fn set_display_name(&self, token: &str, csrf: &str, user_id: i64, name: &str) -> Result<()> {
        let r = self
            .send(self.with_csrf(Method::PATCH, &format!("https://users.roblox.com/v1/users/{user_id}/display-names"), token, csrf).json(&json!({ "newDisplayName": name })))
            .await?;
        if r.status != 200 {
            let message = r.json().and_then(|v| v["errors"][0]["message"].as_str().map(str::to_string));
            bail!("{}", message.unwrap_or_else(|| format!("Something went wrong\n{}: {}", r.status, r.body)));
        }
        Ok(())
    }

    pub async fn send_friend_request(&self, token: &str, csrf: &str, user_id: i64) -> Result<bool> {
        let r = self
            .send(self.with_csrf(Method::POST, &format!("https://friends.roblox.com/v1/users/{user_id}/request-friendship"), token, csrf).header("Content-Type", "application/json").body("{}"))
            .await?;
        Ok(r.ok())
    }

    pub async fn blocked_list(&self, token: &str) -> Result<Raw> {
        self.get("https://accountsettings.roblox.com/v1/users/get-detailed-blocked-users", Some(token)).await
    }

    pub async fn block(&self, token: &str, csrf: &str, user_id: &str, unblock: bool) -> Result<Raw> {
        let url = format!("https://accountsettings.roblox.com/v1/users/{}/{}", user_id, if unblock { "unblock" } else { "block" });
        self.send(self.with_csrf(Method::POST, &url, token, csrf)).await
    }

    pub async fn private_server_privacy_is_everyone(&self, token: &str, csrf: &str) -> Result<bool> {
        let r = self
            .send(self.with_csrf(Method::GET, "https://www.roblox.com/account/settings/private-server-invite-privacy", token, csrf).header(REFERER, "https://www.roblox.com/my/account"))
            .await?;
        Ok(!r.ok() || r.body.contains("\"AllUsers\""))
    }

    pub async fn set_private_server_privacy_everyone(&self, token: &str, csrf: &str) -> Result<bool> {
        let r = self
            .send(
                self.with_csrf(Method::POST, "https://www.roblox.com/account/settings/private-server-invite-privacy", token, csrf)
                    .header(REFERER, "https://www.roblox.com/my/account")
                    .form(&[("PrivateServerInvitePrivacy", "AllUsers")]),
            )
            .await?;
        Ok(r.ok())
    }
}
