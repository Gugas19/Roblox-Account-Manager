//! The browser: adding accounts and opening them.

use super::*;

pub(super) async fn ensure_browser(app: &Arc<App>) -> anyhow::Result<std::path::PathBuf> {
    if let Some(path) = browser::installed_browser() {
        return Ok(path);
    }
    if app.downloading.swap(true, Ordering::SeqCst) {
        while app.downloading.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        return browser::installed_browser().ok_or_else(|| anyhow::anyhow!("Could not download the browser"));
    }

    let progress_app = app.clone();
    let result = browser::install(&app.api.http, move |p| {
        *progress_app.download_progress.lock().unwrap() = p;
        progress_app.emit("browser-download", p);
    })
    .await;
    app.downloading.store(false, Ordering::SeqCst);
    app.emit("browser-download", if result.is_ok() { 1.0 } else { -1.0 });
    if let Err(err) = &result {
        app.toast("error", format!("Could not download the browser:\n\n{err}"));
    }
    result
}

#[tauri::command]
pub fn browser_status() -> Value {
    let app = app();
    json!({ "installed": browser::installed_browser().is_some(), "downloading": app.downloading.load(Ordering::SeqCst), "progress": *app.download_progress.lock().unwrap(), "folder": browser::browser_folder() })
}

pub(super) fn pick_proxy(app: &App) -> Option<browser::Proxy> {
    if !app.setting_bool("General", "UseProxies") {
        return None;
    }
    let text = std::fs::read_to_string(app.dir.join("proxies.txt")).ok()?;
    let list: Vec<browser::Proxy> = text.lines().filter_map(browser::parse_proxy).collect();
    use rand::seq::SliceRandom;
    list.choose(&mut rand::thread_rng()).cloned()
}

pub(super) async fn launch_browser(app: &Arc<App>, size: (i32, i32), position: Option<(i32, i32)>) -> anyhow::Result<Browser> {
    let chrome = ensure_browser(app).await?;
    let (x, y) = position.unwrap_or_else(|| browser::centered(size.0, size.1));
    Browser::launch(&LaunchOptions { chrome, width: size.0, height: size.1, x: Some(x), y: Some(y), data_dir: app.dir.clone(), proxy: pick_proxy(app) }).await
}

/// Opens the Roblox login page, adds the account once the user logs in
pub(super) async fn login_flow(app: Arc<App>, username: Option<String>, password: Option<String>, size: (i32, i32), position: Option<(i32, i32)>, url: Option<String>, script: Option<String>) -> anyhow::Result<()> {
    let browser = launch_browser(&app, size, position).await?;
    browser.navigate(url.as_deref().unwrap_or("https://www.roblox.com/login")).await.ok();
    browser.dark_theme().await;
    browser.run_post_navigate_scripts(&app.dir).await;
    if let Some(s) = script.filter(|s| !s.trim().is_empty()) {
        let _ = browser.evaluate(&s).await;
    }

    if let (Some(u), Some(p)) = (username.as_ref(), password.as_ref()) {
        if browser.wait_for("#login-username", Duration::from_secs(5)).await.unwrap_or(false) {
            let _ = browser.type_into("#login-username", u).await;
        }
        if browser.wait_for("#login-password", Duration::from_secs(5)).await.unwrap_or(false) {
            let _ = browser.type_into("#login-password", p).await;
        }
        let _ = browser.click("#login-button").await;
    }

    match browser.wait_for_login().await? {
        Some((token, typed_password)) => {
            let password = if !typed_password.is_empty() { typed_password } else { password.unwrap_or_default() };
            match app.add_account(&token, &password).await {
                Ok(a) => app.toast("success", format!("Added {}", a.username)),
                Err(err) => app.toast("error", err.to_string()),
            }
            browser.close().await;
        }
        None => browser.close().await,
    }
    Ok(())
}

#[tauri::command]
pub fn add_account_browser() {
    let app = app();
    tauri::async_runtime::spawn(async move {
        if let Err(err) = login_flow(app.clone(), None, None, (880, 740), None, None, None).await {
            app.toast("error", format!("Could not open the browser:\n\n{err}"));
        }
    });
}

/// Logs in user:pass combos, two browsers at a time side by side
#[tauri::command]
pub fn add_user_pass(text: String) {
    let app = app();
    tauri::async_runtime::spawn(async move {
        let size = (455, 485);
        let grid = browser::grid(size.0, size.1);
        let combos: Vec<(String, String)> = text.lines().filter_map(|l| l.trim().split_once(':').map(|(u, p)| (u.to_string(), p.to_string()))).collect();

        for (i, chunk) in combos.chunks(2).enumerate() {
            let mut tasks = Vec::new();
            for (j, (u, p)) in chunk.iter().enumerate() {
                let pos = grid[(i * 2 + j) % grid.len()];
                tasks.push(tauri::async_runtime::spawn(login_flow(app.clone(), Some(u.clone()), Some(p.clone()), size, Some(pos), None, None)));
            }
            for t in tasks {
                if let Ok(Err(err)) = t.await {
                    app.toast("error", format!("Could not open the browser:\n\n{err}"));
                }
            }
        }
    });
}

/// Opens browsers logged in as the accounts, optionally on a page, with a script, or joining a group
#[tauri::command]
pub fn open_browser(names: Vec<String>, url: Option<String>, script: Option<String>, join_group: bool) -> R<()> {
    let app = app();
    let accounts = app.accounts_named(&names);
    if accounts.is_empty() {
        return Err("No Account Selected!".into());
    }

    for account in accounts {
        let app = app.clone();
        let url = url.clone();
        let script = script.clone();
        tauri::async_runtime::spawn(async move {
            let result: anyhow::Result<()> = async {
                let browser = launch_browser(&app, (880, 740), None).await?;
                browser.set_cookie(&account.security_token).await?;
                browser.navigate(url.as_deref().filter(|u| !u.is_empty()).unwrap_or("https://www.roblox.com/home")).await.ok();
                browser.run_post_navigate_scripts(&app.dir).await;
                if let Some(s) = script.filter(|s| !s.trim().is_empty()) {
                    let _ = browser.evaluate(&s).await;
                }
                if join_group && browser.wait_for("#group-join-button", Duration::from_secs(12)).await.unwrap_or(false) {
                    let _ = browser.click("#group-join-button").await;
                }
                browser.detach();
                Ok(())
            }
            .await;
            if let Err(err) = result {
                app.toast("error", format!("Could not open the browser:\n\n{err}"));
            }
        });
    }
    Ok(())
}

/// Several logged out browsers on a page with a script, each adds the account its user logs into
#[tauri::command]
pub fn custom_browsers(url: String, script: String, count: usize) {
    let app = app();
    let size = (550, 440);
    let grid = browser::grid(size.0, size.1);
    for i in 0..count.clamp(1, 15) {
        let app = app.clone();
        let (url, script, pos) = (url.clone(), script.clone(), grid[i % grid.len()]);
        tauri::async_runtime::spawn(async move {
            if let Err(err) = login_flow(app.clone(), None, None, size, Some(pos), Some(url), Some(script)).await {
                app.toast("error", format!("Could not open the browser:\n\n{err}"));
            }
        });
    }
}
