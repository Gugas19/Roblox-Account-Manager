//! Roblox web APIs used by the app (port of Account.cs, Presence.cs, Batch.cs and the ServerList calls).
//! The calls are split by topic, each file adds its methods to `Api`.

mod account;
mod auth;
mod avatar;
mod games;
mod users;

use anyhow::{anyhow, bail, Result};
use reqwest::header::{HeaderMap, HeaderValue, COOKIE, REFERER};
use reqwest::{Client, Method, RequestBuilder, Response, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::time::Duration;

const GAME_REFERER: &str = "https://www.roblox.com/games/4924922222/Brookhaven-RP";

#[derive(Clone)]
pub struct Api {
    pub http: Client,
    /// gamejoin.roblox.com only answers this user agent
    game_join: Client,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct UserInfo {
    pub id: i64,
    pub name: String,
    #[serde(rename = "displayName", default)]
    pub display_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Presence {
    #[serde(rename = "userPresenceType", default)]
    pub kind: i64, // 0 offline, 1 online, 2 in game, 3 in studio
    #[serde(rename = "lastLocation", default)]
    pub last_location: Option<String>,
    #[serde(rename = "placeId", default)]
    pub place_id: Option<i64>,
    #[serde(rename = "gameId", default)]
    pub game_id: Option<String>,
    #[serde(rename = "userId", default)]
    pub user_id: i64,
}

pub struct Raw {
    pub status: u16,
    pub body: String,
    pub headers: HeaderMap,
    pub cookie: Option<String>,
}

impl Raw {
    pub fn ok(&self) -> bool {
        (200..300).contains(&self.status)
    }

    pub fn json(&self) -> Option<Value> {
        serde_json::from_str(&self.body).ok()
    }
}

fn cookie(token: &str) -> HeaderValue {
    HeaderValue::from_str(&format!(".ROBLOSECURITY={}", token)).unwrap_or_else(|_| HeaderValue::from_static(""))
}

fn roblosecurity_from(headers: &HeaderMap) -> Option<String> {
    headers.get_all("set-cookie").iter().filter_map(|v| v.to_str().ok()).find_map(|v| {
        let v = v.trim_start();
        v.strip_prefix(".ROBLOSECURITY=").map(|rest| rest.split(';').next().unwrap_or("").to_string()).filter(|t| !t.is_empty())
    })
}

async fn finish(response: Response) -> Raw {
    let status = response.status().as_u16();
    let headers = response.headers().clone();
    let cookie = roblosecurity_from(&headers);
    let body = response.text().await.unwrap_or_default();
    Raw { status, body, headers, cookie }
}

impl Api {
    pub fn new() -> Api {
        let http = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/154.0.0.0 Safari/537.36")
            .timeout(Duration::from_secs(30))
            .gzip(true)
            .build()
            .expect("http client");
        let game_join = Client::builder().user_agent("Roblox/WinInet").timeout(Duration::from_secs(30)).build().expect("http client");
        Api { http, game_join }
    }

    pub(crate) fn request(&self, method: Method, url: &str, token: Option<&str>) -> RequestBuilder {
        let mut r = self.http.request(method, url);
        if let Some(t) = token.filter(|t| !t.is_empty()) {
            r = r.header(COOKIE, cookie(t));
        }
        r
    }

    pub async fn send(&self, builder: RequestBuilder) -> Result<Raw> {
        Ok(finish(builder.send().await?).await)
    }

    pub async fn get(&self, url: &str, token: Option<&str>) -> Result<Raw> {
        self.send(self.request(Method::GET, url, token)).await
    }
}

impl Default for Api {
    fn default() -> Self {
        Api::new()
    }
}

pub fn is_success(status: StatusCode) -> bool {
    status.is_success()
}
