//! Friends of an account, its friend requests, and who can see it online or join it (the same routes as Roblox's own
//! site; Roblox Manager uses them too).

use super::*;
use std::collections::HashSet;

/// A Roblox user in a friends list or a friend request
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: i64,
    pub name: String,
    pub display_name: String,
}

/// One page of users (`data`, `nextPageCursor`); names can be missing, they're filled afterwards
fn users_page(v: &Value) -> (Vec<Person>, Option<String>) {
    let people = v["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|u| {
            let id = u["id"].as_i64().or_else(|| u["userId"].as_i64()).filter(|id| *id > 0)?;
            Some(Person { id, name: u["name"].as_str().unwrap_or_default().to_string(), display_name: u["displayName"].as_str().unwrap_or_default().to_string() })
        })
        .collect();
    let cursor = v["nextPageCursor"].as_str().filter(|c| !c.is_empty()).map(str::to_string);
    (people, cursor)
}

/// Who can join it in experiences, who can see it online: the value now and the ones Roblox allows this account
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Visibility {
    pub field: String,
    pub current: String,
    pub options: Vec<String>,
}

pub const VISIBILITY_FIELDS: [&str; 2] = ["whoCanJoinMeInExperiences", "whoCanSeeMyOnlineStatus"];
const VISIBILITY_URL: &str = "https://apis.roblox.com/user-settings-api/v1/user-settings";

/// A choice is a plain value or { value, label, disabled... }; the disabled ones aren't offered
fn visibility(v: &Value) -> Vec<Visibility> {
    let value = |o: &Value| o.as_str().or_else(|| o["value"].as_str()).map(str::to_string);
    let disabled = |o: &Value| o["isEnabled"] == false || o["enabled"] == false || o["isDisabled"] == true || o["disabled"] == true;
    VISIBILITY_FIELDS
        .iter()
        .filter_map(|field| {
            let setting = &v[*field];
            let current = value(&setting["currentValue"])?;
            let mut options: Vec<String> = Vec::new();
            for o in setting["options"].as_array().into_iter().flatten().filter(|o| !disabled(o)) {
                if let Some(v) = value(o).filter(|v| !options.contains(v)) {
                    options.push(v);
                }
            }
            Some(Visibility { field: field.to_string(), current, options })
        })
        .collect()
}

/// What can be done with a friend or a request
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum FriendAction {
    Accept,
    Decline,
    Remove,
}

impl FriendAction {
    fn route(self) -> &'static str {
        match self {
            FriendAction::Accept => "accept-friend-request",
            FriendAction::Decline => "decline-friend-request",
            FriendAction::Remove => "unfriend",
        }
    }
}

impl Api {
    /// Every page of a users list (at most 40 pages, a cursor seen twice stops it)
    async fn all_users(&self, token: &str, base: &str) -> Result<Vec<Person>> {
        let mut people = Vec::new();
        let mut seen_cursors = HashSet::new();
        let mut cursor: Option<String> = None;
        for _ in 0..40 {
            let url = match &cursor {
                Some(c) => format!("{base}{}cursor={}", if base.contains('?') { '&' } else { '?' }, urlencoding::encode(c)),
                None => base.to_string(),
            };
            let r = self.get(&url, Some(token)).await?;
            if !r.ok() {
                anyhow::bail!("Roblox didn't give the list ({})", r.status);
            }
            let (page, next) = users_page(&r.json().unwrap_or_default());
            people.extend(page);
            match next {
                Some(c) if seen_cursors.insert(c.clone()) => cursor = Some(c),
                _ => break,
            }
        }
        let mut ids = HashSet::new();
        people.retain(|p| ids.insert(p.id));
        self.fill_names(&mut people).await;
        Ok(people)
    }

    /// Names and display names Roblox left out of a list
    async fn fill_names(&self, people: &mut [Person]) {
        let missing: Vec<i64> = people.iter().filter(|p| p.name.is_empty()).map(|p| p.id).collect();
        for batch in missing.chunks(100) {
            let Ok(r) = self.send(self.request(Method::POST, "https://users.roblox.com/v1/users", None).json(&json!({ "userIds": batch, "excludeBannedUsers": false }))).await else { continue };
            let (found, _) = users_page(&r.json().unwrap_or_default());
            for f in found {
                if let Some(p) = people.iter_mut().find(|p| p.id == f.id) {
                    p.name = f.name;
                    p.display_name = f.display_name;
                }
            }
        }
    }

    pub async fn friends(&self, token: &str, user_id: i64) -> Result<Vec<Person>> {
        self.all_users(token, &format!("https://friends.roblox.com/v1/users/{user_id}/friends")).await
    }

    pub async fn friend_requests(&self, token: &str) -> Result<Vec<Person>> {
        self.all_users(token, "https://friends.roblox.com/v1/my/friends/requests?limit=100").await
    }

    pub async fn friend_action(&self, token: &str, csrf: &str, target: i64, action: FriendAction) -> Result<()> {
        let r = self.send(self.with_csrf(Method::POST, &format!("https://friends.roblox.com/v1/users/{target}/{}", action.route()), token, csrf).header("Content-Type", "application/json").body("{}")).await?;
        if !r.ok() {
            anyhow::bail!("Roblox refused it ({})", r.status);
        }
        Ok(())
    }

    /// Presence as seen by this account: friends see each other's game and server
    pub async fn presence_as(&self, user_ids: &[i64], token: &str, csrf: &str) -> Result<Vec<Presence>> {
        let mut all = Vec::new();
        for chunk in user_ids.chunks(50) {
            let r = self.send(self.with_csrf(Method::POST, "https://presence.roblox.com/v1/presence/users", token, csrf).json(&json!({ "userIds": chunk }))).await?;
            if let Some(list) = r.json().and_then(|v| v["userPresences"].as_array().cloned()) {
                all.extend(list.into_iter().filter_map(|p| serde_json::from_value::<Presence>(p).ok()));
            }
        }
        Ok(all)
    }

    /// Avatar headshots, by user
    pub async fn headshots(&self, user_ids: &[i64]) -> std::collections::HashMap<i64, String> {
        let mut out = std::collections::HashMap::new();
        for chunk in user_ids.chunks(100) {
            let ids = chunk.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
            let Ok(r) = self.get(&format!("https://thumbnails.roblox.com/v1/users/avatar-headshot?size=48x48&format=png&userIds={ids}"), None).await else { continue };
            for item in r.json().and_then(|v| v["data"].as_array().cloned()).unwrap_or_default() {
                if let (Some(id), Some(url)) = (item["targetId"].as_i64(), item["imageUrl"].as_str()) {
                    out.insert(id, url.to_string());
                }
            }
        }
        out
    }

    pub async fn visibility(&self, token: &str) -> Result<Vec<Visibility>> {
        let r = self.get(&format!("{VISIBILITY_URL}/settings-and-options?requestedUserSettings={}", VISIBILITY_FIELDS.join(",")), Some(token)).await?;
        if !r.ok() {
            anyhow::bail!("Roblox didn't give the visibility settings ({})", r.status);
        }
        let list = visibility(&r.json().unwrap_or_default());
        if list.is_empty() {
            anyhow::bail!("Roblox didn't give the visibility settings");
        }
        Ok(list)
    }

    /// Changes one, after checking Roblox offers that value to this account
    pub async fn set_visibility(&self, token: &str, csrf: &str, field: &str, value: &str) -> Result<()> {
        let now = self.visibility(token).await?;
        if !now.iter().any(|v| v.field == field && v.options.iter().any(|o| o == value)) {
            anyhow::bail!("Roblox doesn't offer that choice for this account");
        }
        let r = self.send(self.with_csrf(Method::POST, VISIBILITY_URL, token, csrf).json(&json!({ field: value }))).await?;
        if !r.ok() {
            anyhow::bail!("Roblox refused it ({})", r.status);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn users_pages_with_or_without_names() {
        let (people, cursor) = users_page(&json!({ "data": [{ "id": 1, "name": "a", "displayName": "A" }, { "id": 2, "name": null }, { "id": 0 }], "nextPageCursor": "next" }));
        assert_eq!(people, vec![Person { id: 1, name: "a".into(), display_name: "A".into() }, Person { id: 2, name: String::new(), display_name: String::new() }]);
        assert_eq!(cursor.as_deref(), Some("next"));
        assert_eq!(users_page(&json!({ "data": [], "nextPageCursor": "" })).1, None);
    }

    #[test]
    fn visibility_choices_plain_or_described_and_never_disabled_ones() {
        let list = visibility(&json!({
            "whoCanJoinMeInExperiences": { "currentValue": "Followers", "options": [
                { "value": "Followers", "label": "Friends, followers and following" }, "Friends", { "value": "Friends" },
                { "value": "TrustedFriends", "disabled": true }, { "value": "NoOne" } ] },
            "whoCanSeeMyOnlineStatus": { "currentValue": "NoOne", "options": ["Friends", "NoOne"] },
        }));
        assert_eq!(list[0], Visibility { field: "whoCanJoinMeInExperiences".into(), current: "Followers".into(), options: vec!["Followers".into(), "Friends".into(), "NoOne".into()] });
        assert_eq!(list[1].current, "NoOne");
        // nothing understood: nothing offered
        assert!(visibility(&json!({})).is_empty());
    }
}
