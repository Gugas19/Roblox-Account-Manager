//! The browser used to add accounts and open them: Chrome for Testing (same folder as the C# version, so it isn't
//! downloaded again) driven through the DevTools protocol.

use anyhow::{anyhow, bail, Context, Result};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::{Child, Command};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;
use tokio_tungstenite::tungstenite::Message;

mod cdp;
mod install;

pub use cdp::Cdp;
pub use install::{browser_folder, install, installed_browser};

/// Settings of BrowserConfig.json next to the exe (same file the C# version read)
#[derive(Default, serde::Deserialize)]
#[serde(default)]
pub struct BrowserConfig {
    #[serde(rename = "CustomArguments")]
    pub custom_arguments: Vec<String>,
    #[serde(rename = "PreNavigateActions")]
    pub pre_navigate_actions: HashMap<String, String>,
    #[serde(rename = "PreNavigateScripts")]
    pub pre_navigate_scripts: Vec<String>,
    #[serde(rename = "PostNavigateScripts")]
    pub post_navigate_scripts: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Proxy {
    pub server: String,
    pub username: String,
    pub password: String,
}

/// "[protocol://]ip:port[@user:pass]" lines of proxies.txt
pub fn parse_proxy(line: &str) -> Option<Proxy> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }
    let (address, credentials) = match line.split_once('@') {
        Some((a, c)) => (a, Some(c)),
        None => (line, None),
    };
    let (username, password) = credentials.and_then(|c| c.split_once(':')).map(|(u, p)| (u.to_string(), p.to_string())).unwrap_or_default();
    Some(Proxy { server: address.to_string(), username, password })
}

#[derive(Clone, Debug)]
pub struct LaunchOptions {
    pub chrome: PathBuf,
    pub width: i32,
    pub height: i32,
    pub x: Option<i32>,
    pub y: Option<i32>,
    pub data_dir: PathBuf,
    pub proxy: Option<Proxy>,
}

pub struct Browser {
    pub cdp: Cdp,
    pub session: String,
    child: Child,
    profile: PathBuf,
    proxy: Option<Proxy>,
}

impl Browser {
    pub async fn launch(options: &LaunchOptions) -> Result<Browser> {
        let profile = std::env::temp_dir().join(format!("ram-browser-{}-{}", std::process::id(), rand::random::<u32>()));
        std::fs::create_dir_all(&profile)?;

        let config: BrowserConfig = std::fs::read_to_string(options.data_dir.join("BrowserConfig.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();

        let mut args = vec![
            "--remote-debugging-port=0".to_string(),
            format!("--user-data-dir={}", profile.display()),
            "--no-first-run".into(),
            "--no-default-browser-check".into(),
            "--disable-blink-features=AutomationControlled".into(), // Roblox shows more captchas to automated browsers
            format!("--window-size={},{}", options.width, options.height),
        ];

        if let (Some(x), Some(y)) = (options.x, options.y) {
            args.push(format!("--window-position={x},{y}"));
        }

        let extension = options.data_dir.join("extension");
        if extension.is_dir() {
            args.push(format!("--disable-extensions-except={}", extension.display()));
            args.push(format!("--load-extension={}", extension.display()));
        }

        if let Some(proxy) = &options.proxy {
            args.push(format!("--proxy-server={}", proxy.server));
        }

        args.extend(config.custom_arguments.iter().cloned());
        args.push("about:blank".into());

        let mut command = Command::new(&options.chrome);
        command.args(&args);
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000); // CREATE_NO_WINDOW for the console part
        }
        let child = command.spawn().with_context(|| format!("Could not start {}", options.chrome.display()))?;

        // Chrome writes the DevTools port and path into the profile
        let port_file = profile.join("DevToolsActivePort");
        let started = Instant::now();
        let endpoint = loop {
            if let Ok(text) = std::fs::read_to_string(&port_file) {
                let mut lines = text.lines();
                if let (Some(port), Some(path)) = (lines.next(), lines.next()) {
                    break format!("ws://127.0.0.1:{}{}", port.trim(), path.trim());
                }
            }
            if started.elapsed() > Duration::from_secs(30) {
                bail!("The browser didn't start");
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        };

        let cdp = Cdp::connect(&endpoint).await?;

        // the first tab
        let mut target = None;
        for _ in 0..50 {
            let targets = cdp.call("Target.getTargets", json!({}), None).await?;
            target = targets["targetInfos"].as_array().and_then(|list| list.iter().find(|t| t["type"] == "page")).and_then(|t| t["targetId"].as_str()).map(str::to_string);
            if target.is_some() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        let target = target.ok_or_else(|| anyhow!("The browser has no tab"))?;

        let attached = cdp.call("Target.attachToTarget", json!({ "targetId": target, "flatten": true }), None).await?;
        let session = attached["sessionId"].as_str().ok_or_else(|| anyhow!("Could not attach to the browser"))?.to_string();

        let browser = Browser { cdp, session, child, profile, proxy: options.proxy.clone() };

        browser.call("Page.enable", json!({})).await?;
        browser.call("Network.enable", json!({})).await?;
        browser.call("Page.addScriptToEvaluateOnNewDocument", json!({ "source": "Object.defineProperty(navigator, 'webdriver', { get: () => undefined })" })).await?;

        if browser.proxy.as_ref().map(|p| !p.username.is_empty()).unwrap_or(false) {
            browser.call("Fetch.enable", json!({ "handleAuthRequests": true })).await?;
            browser.answer_proxy_auth();
        }

        for (url, script) in &config.pre_navigate_actions {
            if url.starts_with("http") {
                let _ = browser.navigate(url).await;
                let _ = browser.evaluate(script).await;
            }
        }
        for script in &config.pre_navigate_scripts {
            let _ = browser.evaluate(script).await;
        }

        Ok(browser)
    }

    /// Scripts from BrowserConfig.json to run after the first page loaded
    pub async fn run_post_navigate_scripts(&self, data_dir: &Path) {
        let config: BrowserConfig = std::fs::read_to_string(data_dir.join("BrowserConfig.json")).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or_default();
        for script in &config.post_navigate_scripts {
            let _ = self.evaluate(script).await;
        }
    }

    fn answer_proxy_auth(&self) {
        let mut events = self.cdp.subscribe();
        let proxy = self.proxy.clone().unwrap_or_default();
        let tx = self.cdp.tx.clone();
        let session = self.session.clone();
        let next = Arc::new(AtomicU64::new(1_000_000));

        tokio::spawn(async move {
            while let Ok(event) = events.recv().await {
                let reply = match event["method"].as_str() {
                    Some("Fetch.authRequired") => json!({ "method": "Fetch.continueWithAuth", "params": { "requestId": event["params"]["requestId"], "authChallengeResponse": { "response": "ProvideCredentials", "username": proxy.username, "password": proxy.password } } }),
                    Some("Fetch.requestPaused") => json!({ "method": "Fetch.continueRequest", "params": { "requestId": event["params"]["requestId"] } }),
                    Some("Closed") => break,
                    _ => continue,
                };
                let mut message = reply;
                message["id"] = json!(next.fetch_add(1, Ordering::SeqCst));
                message["sessionId"] = json!(session);
                let _ = tx.send(Message::Text(message.to_string()));
            }
        });
    }

    pub async fn call(&self, method: &str, params: Value) -> Result<Value> {
        self.cdp.call(method, params, Some(&self.session)).await
    }

    pub async fn set_cookie(&self, token: &str) -> Result<()> {
        let expires = chrono::Utc::now().timestamp() + 365 * 24 * 3600;
        self.call(
            "Network.setCookie",
            json!({ "name": ".ROBLOSECURITY", "value": token, "domain": ".roblox.com", "path": "/", "secure": true, "httpOnly": true, "expires": expires }),
        )
        .await?;
        Ok(())
    }

    pub async fn roblosecurity(&self) -> Result<Option<String>> {
        let cookies = self.call("Network.getCookies", json!({ "urls": ["https://www.roblox.com", "https://auth.roblox.com"] })).await?;
        Ok(cookies["cookies"].as_array().and_then(|list| list.iter().find(|c| c["name"] == ".ROBLOSECURITY")).and_then(|c| c["value"].as_str()).map(str::to_string))
    }

    /// Opens a page and waits for it to load (up to 5 minutes on slow connections)
    pub async fn navigate(&self, url: &str) -> Result<()> {
        let mut events = self.cdp.subscribe();
        self.call("Page.navigate", json!({ "url": url, "referrer": "https://google.com/" })).await?;

        let wait = async {
            while let Ok(event) = events.recv().await {
                match event["method"].as_str() {
                    Some("Page.loadEventFired") => return Ok(()),
                    Some("Closed") => bail!("The browser was closed"),
                    _ => {}
                }
            }
            Ok(())
        };
        tokio::time::timeout(Duration::from_secs(300), wait).await.map_err(|_| anyhow!("The page took too long to load"))?
    }

    pub async fn evaluate(&self, expression: &str) -> Result<Value> {
        let r = self.call("Runtime.evaluate", json!({ "expression": expression, "awaitPromise": true, "returnByValue": true })).await?;
        if let Some(e) = r.get("exceptionDetails") {
            bail!("{}", e["exception"]["description"].as_str().or(e["text"].as_str()).unwrap_or("script error"));
        }
        Ok(r["result"]["value"].clone())
    }

    pub async fn wait_for(&self, selector: &str, timeout: Duration) -> Result<bool> {
        let started = Instant::now();
        let check = format!("!!document.querySelector({})", serde_json::to_string(selector)?);
        while started.elapsed() < timeout {
            if self.evaluate(&check).await.ok().and_then(|v| v.as_bool()).unwrap_or(false) {
                return Ok(true);
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
        Ok(false)
    }

    /// Types like a user would, so React inputs pick it up
    pub async fn type_into(&self, selector: &str, text: &str) -> Result<()> {
        self.evaluate(&format!("document.querySelector({}).focus()", serde_json::to_string(selector)?)).await?;
        self.call("Input.insertText", json!({ "text": text })).await?;
        Ok(())
    }

    pub async fn click(&self, selector: &str) -> Result<()> {
        self.evaluate(&format!("document.querySelector({}).click()", serde_json::to_string(selector)?)).await?;
        Ok(())
    }

    pub async fn dark_theme(&self) {
        let _ = self.evaluate("document.body && (document.body.classList.remove('light-theme'), document.body.classList.add('dark-theme'))").await;
    }

    /// Waits until the user logs in (a session cookie shows up) or closes the browser.
    /// Also remembers the password typed in the login form (when saving passwords is on).
    pub async fn wait_for_login(&self) -> Result<Option<(String, String)>> {
        let mut events = self.cdp.subscribe();
        let password = Arc::new(Mutex::new(String::new()));
        let password_writer = password.clone();
        let closed = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let closed_writer = closed.clone();

        tokio::spawn(async move {
            while let Ok(event) = events.recv().await {
                match event["method"].as_str() {
                    Some("Network.requestWillBeSent") => {
                        let request = &event["params"]["request"];
                        let url = request["url"].as_str().unwrap_or("");
                        if request["method"] == "POST" && url.starts_with("https://auth.roblox.com/v2/login") {
                            if let Some(body) = request["postData"].as_str().and_then(|b| serde_json::from_str::<Value>(b).ok()) {
                                if body["ctype"].as_str().map(|c| c.eq_ignore_ascii_case("username")).unwrap_or(false) {
                                    if let Some(p) = body["password"].as_str() {
                                        *password_writer.lock().await = p.to_string();
                                    }
                                }
                            }
                        }
                    }
                    Some("Closed") => {
                        closed_writer.store(true, Ordering::SeqCst);
                        break;
                    }
                    _ => {}
                }
            }
        });

        let first = self.roblosecurity().await.ok().flatten();

        loop {
            if closed.load(Ordering::SeqCst) {
                return Ok(None);
            }
            tokio::time::sleep(Duration::from_millis(800)).await;

            match self.roblosecurity().await {
                Ok(Some(token)) if Some(&token) != first.as_ref() => {
                    let p = password.lock().await.clone();
                    return Ok(Some((token, p)));
                }
                Ok(_) => {}
                Err(_) => return Ok(None), // closed
            }
        }
    }

    pub async fn close(mut self) {
        let _ = tokio::time::timeout(Duration::from_secs(3), self.cdp.call("Browser.close", json!({}), None)).await;
        tokio::time::sleep(Duration::from_millis(300)).await;
        let _ = self.child.kill();
        let _ = self.child.wait();
        for _ in 0..10 {
            if std::fs::remove_dir_all(&self.profile).is_ok() || !self.profile.exists() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
    }

    /// Lets the user keep using the browser, cleans up once they close it
    pub fn detach(self) {
        tokio::spawn(async move {
            let mut events = self.cdp.subscribe();
            while let Ok(event) = events.recv().await {
                if event["method"] == "Closed" || event["method"] == "Target.detachedFromTarget" || event["method"] == "Inspector.detached" {
                    break;
                }
            }
            self.close().await;
        });
    }
}

/// Window positions for several browsers side by side (bulk add)
pub fn grid(width: i32, height: i32) -> Vec<(i32, i32)> {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN};
    let (screen_w, screen_h) = unsafe { (GetSystemMetrics(SM_CXVIRTUALSCREEN), GetSystemMetrics(SM_CYVIRTUALSCREEN)) };
    let mut cells = Vec::new();
    let mut x = 0;
    while x < screen_w.max(width) {
        let mut y = 0;
        while y < (screen_h - height / 2).max(1) {
            cells.push((x, y));
            y += height;
        }
        x += width;
    }
    if cells.is_empty() {
        cells.push((0, 0));
    }
    cells
}

/// Centered on the primary screen
pub fn centered(width: i32, height: i32) -> (i32, i32) {
    use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXSCREEN, SM_CYSCREEN};
    let (w, h) = unsafe { (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN)) };
    ((w - width).max(0) / 2, (h - height).max(0) / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxies() {
        let p = parse_proxy("socks5://1.2.3.4:999@user:pass").unwrap();
        assert_eq!(p.server, "socks5://1.2.3.4:999");
        assert_eq!((p.username.as_str(), p.password.as_str()), ("user", "pass"));
        assert_eq!(parse_proxy("1.2.3.4:80").unwrap().username, "");
        assert!(parse_proxy("  ").is_none());
    }
}
