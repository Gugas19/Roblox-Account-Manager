//! Outfits, wearing avatars and buying missing items.

use super::*;

#[tauri::command]
pub async fn outfits(username: String) -> R<Vec<Value>> {
    let app = app();
    let token = app.any_token();
    let user_id = app.api.user_id(username.trim(), token.as_deref()).await.map_err(e)?;
    let list = app.api.outfits(user_id, token.as_deref()).await.map_err(|err| app.tr(&err.to_string()))?;
    let requests: Vec<(i64, &str, &str)> = list.iter().map(|(_, id)| (*id, "Outfit", "150x150")).collect();
    let icons: std::collections::HashMap<i64, String> = app.api.thumbnails(&requests).await.unwrap_or_default().into_iter().collect();
    Ok(list.into_iter().map(|(name, id)| json!({ "id": id, "name": name, "icon": icons.get(&id) })).collect())
}

#[tauri::command]
pub async fn outfit_json(outfit_id: i64) -> R<String> {
    let app = app();
    let details = app.api.outfit_details(outfit_id, app.any_token().as_deref()).await.map_err(|err| app.tr(&err.to_string()))?;
    let assets: Vec<Value> = details["assets"].as_array().cloned().unwrap_or_default().iter().map(|a| json!({ "id": a["id"] })).collect();
    Ok(json!({ "playerAvatarType": details["playerAvatarType"], "scales": details["scale"], "bodyColors": details["bodyColors"], "assets": assets }).to_string())
}

#[tauri::command]
pub async fn wear(name: String, avatar: String) -> R<Vec<Value>> {
    let app = app();
    let a = app.account(&name).ok_or("No Account Selected!")?;
    let avatar: Value = serde_json::from_str(&avatar).map_err(|_| "Invalid JSON")?;
    let csrf = app.csrf(&a).await.map_err(e)?;
    let missing = app.api.set_avatar(&a.security_token, &csrf, &avatar).await.map_err(e)?;

    let mut items = Vec::new();
    let requests: Vec<(i64, &str, &str)> = missing.iter().map(|id| (*id, "Asset", "150x150")).collect();
    let icons: std::collections::HashMap<i64, String> = app.api.thumbnails(&requests).await.unwrap_or_default().into_iter().collect();
    for id in missing {
        let d = app.api.asset_details(id).await.unwrap_or_default();
        items.push(json!({ "id": id, "name": d["Name"], "price": d["PriceInRobux"], "productId": d["ProductId"], "sellerId": d["Creator"]["CreatorTargetId"], "forSale": d["IsForSale"], "icon": icons.get(&id) }));
    }
    Ok(items)
}

#[tauri::command]
pub async fn purchase(name: String, product_id: i64, price: i64, seller_id: i64) -> R<Value> {
    let app = app();
    let a = app.account(&name).ok_or("No Account Selected!")?;
    let csrf = app.csrf(&a).await.map_err(e)?;
    app.api.purchase(&a.security_token, &csrf, product_id, price, seller_id).await.map_err(e)
}
