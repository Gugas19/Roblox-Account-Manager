//! The commands. The same code runs inside the open app (`in_app`) or in ram-cli on its own.

use super::args::{in_group, pick, usage, Args, Usage};
use super::{Out, Request};
use crate::app::App;
use crate::commands::accounts::import_cookies;
use crate::commands::joining::{launch_sequence, parse_place};
use crate::core::accounts::Account;
use crate::platform::win as winutil;
use crate::services::antiafk;
use crate::services::launcher::{JoinOptions, JoinResult};
use anyhow::{anyhow, bail, Result};
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Runs one command and returns the exit code: 0 done, 1 failed, 2 wrong command line
pub async fn execute(app: &Arc<App>, request: Request, out: &Out, in_app: bool) -> i32 {
    match command(app, request, out, in_app).await {
        Ok(()) => 0,
        Err(e) => match e.downcast_ref::<Usage>() {
            Some(wrong) => {
                out.error(format!("{wrong}\nSee: ram-cli help"));
                2
            }
            None => {
                out.error(format!("{e:#}"));
                1
            }
        },
    }
}

async fn command(app: &Arc<App>, request: Request, out: &Out, in_app: bool) -> Result<()> {
    let (name, rest) = request.args.split_first().ok_or_else(|| usage("No command"))?;
    match name.as_str() {
        "list" | "ls" => list(app, &Args::parse(rest, &["json", "all"], &["group"])?, out),
        "join" => join(app, &Args::parse(rest, &["all"], &["group", "place", "job"])?, out).await,
        "follow" => follow(app, &Args::parse(rest, &["all"], &["group", "user"])?, out).await,
        "close" | "kill" => close(app, &Args::parse(rest, &["all"], &["group"])?, out),
        "import" => import(app, request.input.unwrap_or_default(), out).await,
        "antiafk" | "anti-afk" => anti_afk(app, rest, out, in_app),
        "status" => status(app, out, in_app),
        "open" | "show" => {
            if let Some(handle) = app.handle.get() {
                crate::show_main(handle);
            }
            out.line("Roblox Account Manager is open");
            Ok(())
        }
        other => Err(usage(format!("Unknown command '{other}'"))),
    }
}

fn ready(app: &App) -> Result<()> {
    if !app.ready.load(Ordering::SeqCst) {
        bail!("Roblox Account Manager is waiting for its password, unlock it in the window first");
    }
    Ok(())
}

/// Saves now (the app's own saves wait 2 seconds after it starts) and updates the window
fn save(app: &App) {
    if let Err(e) = app.store.lock().unwrap().save(true, false) {
        log::error!("Saving accounts failed: {e}");
    }
    app.accounts_changed();
}

fn list(app: &App, args: &Args, out: &Out) -> Result<()> {
    ready(app)?;
    let mut views = app.views();
    if let Some(group) = args.value("group") {
        views.retain(|v| in_group(&v.group, group));
    }
    if args.has("json") {
        out.line(serde_json::to_string_pretty(&views)?);
        return Ok(());
    }
    if views.is_empty() {
        out.line("No accounts");
        return Ok(());
    }

    let mut rows = vec![["USERNAME".to_string(), "ALIAS".into(), "GROUP".into(), "LAST USED".into(), "DESCRIPTION".into()]];
    for v in &views {
        let days = v.days_unused.floor() as i64;
        let used = match days {
            d if d > 36500 => "never".to_string(),
            0 => "today".into(),
            1 => "yesterday".into(),
            d => format!("{d} days ago"),
        };
        let description: String = v.description.lines().next().unwrap_or("").chars().take(40).collect();
        rows.push([v.username.clone(), v.alias.clone(), v.group_title.clone(), used, description]);
    }
    let widths: Vec<usize> = (0..5).map(|c| rows.iter().map(|r| r[c].chars().count()).max().unwrap_or(0)).collect();
    for row in rows {
        let line: Vec<String> = row.iter().zip(&widths).map(|(cell, width)| format!("{cell:<width$}")).collect();
        out.line(line.join("  ").trim_end());
    }
    Ok(())
}

async fn join(app: &Arc<App>, args: &Args, out: &Out) -> Result<()> {
    ready(app)?;
    let accounts = pick(app, args)?;
    let given = args.value("place");
    let (place, job) = parse_place(given.unwrap_or(&app.setting("General", "SavedPlaceId")), args.value("job").unwrap_or(""));

    let place_id = match place {
        Some(id) => id,
        None if given.is_some() => return Err(usage("--place takes a place id or a game link")),
        None if accounts.iter().all(|a| a.field("SavedPlaceId").parse::<i64>().is_ok()) => 0,
        None => return Err(usage("Which game? Use --place <id or game link>")),
    };
    let vip = job.len() > 4 && job.starts_with("VIP:");
    let job_id = if vip { job[4..].to_string() } else { job };

    let opts = JoinOptions { place_id, job_id, join_vip: vip, privacy_ok: true, ..app.join_options() };
    launch(app, accounts, opts, given.is_none(), out).await
}

async fn follow(app: &Arc<App>, args: &Args, out: &Out) -> Result<()> {
    ready(app)?;
    let accounts = pick(app, args)?;
    let user = args.value("user").ok_or_else(|| usage("Whom to follow? Use --user <username>"))?.trim().to_string();

    let token = app.any_token();
    let user_id = app.api.user_id(&user, token.as_deref()).await.map_err(|e| anyhow!("{e}"))?;
    let presence = app.api.presence(&[user_id]).await.map_err(|e| anyhow!("{e}"))?;
    if !presence.first().map(|p| p.kind == 2 && p.place_id.unwrap_or(0) > 0).unwrap_or(false) {
        bail!("{user} isn't in a game right now, or their joins are private");
    }

    let opts = JoinOptions { place_id: user_id, follow_user: true, privacy_ok: true, ..app.join_options() };
    launch(app, accounts, opts, false, out).await
}

/// One account after the other, like the app, telling how each went
async fn launch(app: &Arc<App>, accounts: Vec<Account>, opts: JoinOptions, use_saved: bool, out: &Out) -> Result<()> {
    let total = accounts.len();
    let (mut done, mut failed) = (0, 0);

    launch_sequence(app, &accounts, opts, use_saved, |i, account, result| {
        done += 1;
        let at = format!("[{}/{total}] {}", i + 1, account.username);
        match result {
            JoinResult::Success => out.line(format!("{at} launched")),
            JoinResult::NeedsPrivacy => {
                failed += 1;
                out.error(format!("{at}: the private server needs its privacy changed, join it once from the app"));
            }
            JoinResult::Error(e) => {
                failed += 1;
                out.error(format!("{at}: {e}"));
            }
        }
    })
    .await;
    save(app);

    if done < total {
        bail!("Stopped after {done} of {total}: another launch started or it was cancelled");
    }
    if failed > 0 {
        bail!("{failed} of {total} failed");
    }
    Ok(())
}

fn close(app: &App, args: &Args, out: &Out) -> Result<()> {
    let processes = winutil::roblox_processes();
    let targets: Vec<u32> = if args.has("all") {
        processes.iter().map(|p| p.pid).collect()
    } else {
        ready(app)?;
        let trackers: Vec<String> = pick(app, args)?.into_iter().filter_map(|a| a.browser_tracker_id).collect();
        processes.iter().filter(|p| p.tracker_id().map(|t| trackers.contains(&t)).unwrap_or(false)).map(|p| p.pid).collect()
    };

    for pid in &targets {
        winutil::kill(*pid);
    }
    out.line(match targets.len() {
        0 => "No Roblox to close".to_string(),
        1 => "Closed 1 Roblox".into(),
        n => format!("Closed {n} Roblox"),
    });
    Ok(())
}

async fn import(app: &App, input: String, out: &Out) -> Result<()> {
    ready(app)?;
    if input.trim().is_empty() {
        return Err(usage("No cookies: ram-cli import <file>, or pass them as input"));
    }
    let results = import_cookies(input).await;
    save(app);

    let failed = results.iter().filter(|r| !r.ok).count();
    for r in &results {
        if r.ok {
            out.line(format!("Added {}", r.text));
        } else {
            out.error(format!("Invalid cookie {}", r.text));
        }
    }
    if failed > 0 {
        bail!("{failed} of {} cookies didn't work", results.len());
    }
    Ok(())
}

fn anti_afk(app: &Arc<App>, rest: &[String], out: &Out, in_app: bool) -> Result<()> {
    let action = rest.first().map(String::as_str).unwrap_or("status");
    match action {
        "status" => {}
        "start" | "stop" | "now" if !in_app => bail!("Anti-AFK runs inside Roblox Account Manager, open it first: ram-cli open"),
        "start" => antiafk::start(app, "Started from the command line"),
        "stop" => antiafk::stop(app, true),
        "now" if !antiafk::is_running(app) => bail!("Anti-AFK isn't running"),
        "now" => antiafk::run_now(app),
        other => return Err(usage(format!("Unknown Anti-AFK action '{other}', use start, stop, now or status"))),
    }

    let status = serde_json::to_value(antiafk::status(app))?;
    let windows = status["windows"].as_array().cloned().unwrap_or_default();
    let line = match (status["running"].as_bool().unwrap_or(false), status["secondsToNext"].as_u64()) {
        (true, Some(s)) => format!("Anti-AFK is on, next action in {}:{:02}", s / 60, s % 60),
        (true, None) => "Anti-AFK is on, waiting for a Roblox window".to_string(),
        (false, _) => "Anti-AFK is off".to_string(),
    };
    out.line(line);
    for w in windows {
        out.line(format!(
            "  {:>6}  {:<20}  {}{}",
            w["pid"].as_u64().unwrap_or(0),
            w["account"].as_str().unwrap_or("?"),
            w["state"].as_str().unwrap_or(""),
            if w["included"].as_bool().unwrap_or(true) { "" } else { ", left alone" }
        ));
    }
    Ok(())
}

fn status(app: &App, out: &Out, in_app: bool) -> Result<()> {
    out.line(format!("Roblox Account Manager {}{}", env!("CARGO_PKG_VERSION"), if in_app { ", open" } else { ", not open" }));
    if app.ready.load(Ordering::SeqCst) {
        out.line(format!("Accounts: {}", app.store.lock().unwrap().accounts.len()));
    } else {
        out.line("Accounts: locked, waiting for the password");
    }
    out.line(format!("Roblox running: {}", winutil::roblox_processes().len()));
    out.line(format!("Anti-AFK: {}", if antiafk::is_running(app) { "on" } else { "off" }));
    let web = app.web_server.lock().unwrap().as_ref().map(|h| h.address.clone());
    out.line(format!("Web API: {}", web.map(|a| format!("http://{a}/")).unwrap_or_else(|| "off".into())));
    Ok(())
}
