//! What an account owns (accessories, clothes, gear, emotes) and the groups it's in: Roblox's inventory and groups
//! routes, as its own site uses them (Roblox Manager uses the same ones).

use super::*;
use std::collections::HashMap;

/// The inventory kinds shown, in the order shown
pub const INVENTORY_TYPES: [&str; 13] =
    ["Hat", "HairAccessory", "FaceAccessory", "NeckAccessory", "ShoulderAccessory", "FrontAccessory", "BackAccessory", "WaistAccessory", "Shirt", "Pants", "TShirt", "Gear", "EmoteAnimation"];

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Item {
    pub id: i64,
    pub name: String,
    pub kind: String,
}

fn inventory_page(v: &Value, kind: &str) -> (Vec<Item>, Option<String>) {
    let items = v["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|i| {
            let id = i["assetId"].as_i64().filter(|id| *id > 0)?;
            Some(Item { id, name: i["assetName"].as_str().unwrap_or_default().to_string(), kind: i["assetType"].as_str().unwrap_or(kind).to_string() })
        })
        .collect();
    (items, v["nextPageCursor"].as_str().filter(|c| !c.is_empty()).map(str::to_string))
}

/// A group an account is in, with its role there
#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Membership {
    pub id: i64,
    pub name: String,
    pub members: i64,
    pub role: String,
    pub rank: i64,
}

fn memberships(v: &Value) -> Vec<Membership> {
    v["data"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|m| {
            let g = &m["group"];
            Some(Membership {
                id: g["id"].as_i64().filter(|id| *id > 0)?,
                name: g["name"].as_str().unwrap_or_default().to_string(),
                members: g["memberCount"].as_i64().unwrap_or(0),
                role: m["role"]["name"].as_str().unwrap_or_default().to_string(),
                rank: m["role"]["rank"].as_i64().unwrap_or(0),
            })
        })
        .collect()
}

impl Api {
    /// Everything the account owns of the kinds shown (as itself: a private inventory shows to its owner)
    pub async fn inventory(&self, token: &str, user_id: i64) -> Result<Vec<Item>> {
        let mut all = Vec::new();
        for kind in INVENTORY_TYPES {
            let mut cursor: Option<String> = None;
            for _ in 0..20 {
                let url = format!(
                    "https://inventory.roblox.com/v2/users/{user_id}/inventory?assetTypes={kind}&sortOrder=Desc&limit=100{}",
                    cursor.as_deref().map(|c| format!("&cursor={}", urlencoding::encode(c))).unwrap_or_default()
                );
                let r = self.get(&url, Some(token)).await?;
                if r.status == 403 {
                    bail!("This account's inventory is private");
                }
                if !r.ok() {
                    bail!("Roblox didn't give the inventory ({})", r.status);
                }
                let (items, next) = inventory_page(&r.json().unwrap_or_default(), kind);
                all.extend(items);
                match next {
                    Some(c) if Some(&c) != cursor.as_ref() => cursor = Some(c),
                    _ => break,
                }
            }
        }
        Ok(all)
    }

    /// Item pictures, by item
    pub async fn asset_thumbnails(&self, ids: &[i64]) -> HashMap<i64, String> {
        let mut out = HashMap::new();
        for chunk in ids.chunks(100) {
            let list = chunk.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
            let Ok(r) = self.get(&format!("https://thumbnails.roblox.com/v1/assets?assetIds={list}&returnPolicy=PlaceHolder&size=150x150&format=Png&isCircular=false"), None).await else { continue };
            for t in r.json().and_then(|v| v["data"].as_array().cloned()).unwrap_or_default() {
                if let (Some(id), Some(url)) = (t["targetId"].as_i64(), t["imageUrl"].as_str()) {
                    out.insert(id, url.to_string());
                }
            }
        }
        out
    }

    pub async fn groups_of(&self, user_id: i64) -> Result<Vec<Membership>> {
        let r = self.get(&format!("https://groups.roblox.com/v2/users/{user_id}/groups/roles"), None).await?;
        if !r.ok() {
            bail!("Roblox didn't give the groups ({})", r.status);
        }
        Ok(memberships(&r.json().unwrap_or_default()))
    }

    pub async fn group_icons(&self, ids: &[i64]) -> HashMap<i64, String> {
        let mut out = HashMap::new();
        for chunk in ids.chunks(100) {
            let list = chunk.iter().map(|i| i.to_string()).collect::<Vec<_>>().join(",");
            let Ok(r) = self.get(&format!("https://thumbnails.roblox.com/v1/groups/icons?groupIds={list}&size=150x150&format=Png&isCircular=false"), None).await else { continue };
            for t in r.json().and_then(|v| v["data"].as_array().cloned()).unwrap_or_default() {
                if let (Some(id), Some(url)) = (t["targetId"].as_i64(), t["imageUrl"].as_str()) {
                    out.insert(id, url.to_string());
                }
            }
        }
        out
    }

    /// Leaves a group (an owner can't leave their own group, Roblox says so)
    pub async fn leave_group(&self, token: &str, csrf: &str, group_id: i64, user_id: i64) -> Result<()> {
        let r = self.send(self.with_csrf(Method::DELETE, &format!("https://groups.roblox.com/v1/groups/{group_id}/users/{user_id}"), token, csrf)).await?;
        if !r.ok() {
            let why = r.json().and_then(|v| v["errors"][0]["message"].as_str().map(str::to_string)).unwrap_or_else(|| r.status.to_string());
            bail!("Roblox refused it ({why})");
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inventory_pages() {
        let (items, cursor) = inventory_page(&json!({ "data": [{ "assetId": 123, "assetName": "Cool Hat", "assetType": "Hat" }, { "assetId": 0 }], "nextPageCursor": "c" }), "Hat");
        assert_eq!(items, vec![Item { id: 123, name: "Cool Hat".into(), kind: "Hat".into() }]);
        assert_eq!(cursor.as_deref(), Some("c"));
        // the kind asked for when Roblox leaves it out
        assert_eq!(inventory_page(&json!({ "data": [{ "assetId": 5, "assetName": "Tee" }] }), "TShirt").0[0].kind, "TShirt");
    }

    #[test]
    fn group_roles() {
        let list = memberships(&json!({ "data": [{ "group": { "id": 7, "name": "Builders", "memberCount": 1200 }, "role": { "name": "Member", "rank": 1 } }, { "group": {} }] }));
        assert_eq!(list, vec![Membership { id: 7, name: "Builders".into(), members: 1200, role: "Member".into(), rank: 1 }]);
    }
}
