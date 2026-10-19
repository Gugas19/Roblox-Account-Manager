//! Updates from the GitHub releases, signed with the project's release key.
//!
//! Every release has `RobloxAccountManager-<version>.zip` and `<zip>.sig`: an Ed25519 signature, made with the
//! maintainer's private key (`cargo run --example sign-release`), of the version followed by the zip. So:
//! - a damaged or changed download, or a release not signed with that key, is refused;
//! - an old signed zip published as a newer version is refused too (the version is part of what's signed);
//! - only newer versions are installed.
//!
//! Check on start and every 30 minutes. The verified zip waits in `update\` next to the app, and is verified again
//! when it's installed: the next time the app starts, or right away with "Restart". Always the newest release: when
//! one came out after the waiting zip was downloaded, that one is fetched instead of installing the older one first.
//! Builds run from a cargo `target` folder (development, tests) never update themselves.

use crate::app::App;
use anyhow::{anyhow, bail, Result};
use base64::Engine;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use futures_util::StreamExt;
use serde::Serialize;
use serde_json::Value;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

const REPO: &str = "Gugas19/Roblox-Account-Manager";
/// The public half of the release signing key
pub const PUBLIC_KEY: &str = "71gWjXxoxVhURCltSi48jbAElKBCchneD7C4m4/9UPQ=";
/// What an update replaces, nothing else is ever taken from the zip
const FILES: &[&str] = &["RobloxAccountManager.exe", "ram-cli.exe"];
const MAX_ZIP: u64 = 100 * 1024 * 1024;
/// Per unpacked file, against zip bombs
const MAX_FILE: u64 = 200 * 1024 * 1024;
const CHECK_EVERY: Duration = Duration::from_secs(30 * 60);
/// The most versions shown in "what's new" when the app is far behind
const MAX_NOTES: usize = 30;
/// How many versions "What's new" in the settings shows
const RECENT_NOTES: usize = 10;
pub const SECTION: &str = "Updates";

#[derive(Serialize, Clone, Debug, PartialEq, Default)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum Status {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available { version: String },
    #[serde(rename_all = "camelCase")]
    Downloading { version: String, progress: f32 },
    /// downloaded and verified, installed when the app starts again
    Ready { version: String },
    Error { message: String },
    /// a development or test build
    Disabled,
}

pub type Version = (u64, u64, u64);

pub fn parse_version(text: &str) -> Option<Version> {
    let mut parts = text.trim().trim_start_matches('v').split('.').map(|p| p.parse::<u64>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

fn version_text(v: Version) -> String {
    format!("{}.{}.{}", v.0, v.1, v.2)
}

pub fn current_version() -> Version {
    parse_version(env!("CARGO_PKG_VERSION")).unwrap_or_default()
}

fn app_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())).unwrap_or_default()
}

fn staging(dir: &Path) -> PathBuf {
    dir.join("update")
}

/// Development and test builds run from cargo's target folder and must never replace themselves
fn disabled() -> bool {
    app_dir().components().any(|c| c.as_os_str().eq_ignore_ascii_case("target"))
}

fn set(app: &App, status: Status) {
    *app.update.lock().unwrap() = status.clone();
    app.emit("update", status);
}

pub fn status(app: &App) -> Status {
    if disabled() {
        return Status::Disabled;
    }
    app.update.lock().unwrap().clone()
}

/// What the release key signs: the version, then the zip (the same in examples/sign-release.rs)
pub fn signed_message(version: Version, zip: &[u8]) -> Vec<u8> {
    let mut message = format!("RobloxAccountManager {}\n", version_text(version)).into_bytes();
    message.extend_from_slice(zip);
    message
}

/// The zip of `version` against the release key
pub fn verify(version: Version, zip: &[u8], signature_b64: &str) -> Result<()> {
    let b64 = base64::engine::general_purpose::STANDARD;
    let key: [u8; 32] = b64.decode(PUBLIC_KEY)?.try_into().map_err(|_| anyhow!("bad public key"))?;
    let signature: [u8; 64] = b64
        .decode(signature_b64.trim())
        .ok()
        .and_then(|s| s.try_into().ok())
        .ok_or_else(|| anyhow!("The update's signature is damaged, it was not installed"))?;
    VerifyingKey::from_bytes(&key)?
        .verify(&signed_message(version, zip), &Signature::from_bytes(&signature))
        .map_err(|_| anyhow!("The update isn't signed with the release key, it was not installed"))
}

struct Release {
    version: Version,
    zip_url: String,
    sig_url: String,
}

fn user_agent() -> String {
    format!("RobloxAccountManager/{}", env!("CARGO_PKG_VERSION"))
}

/// The newest release, when it's newer than this version
async fn latest(http: &reqwest::Client) -> Result<Option<Release>> {
    let release: Value = http
        .get(format!("https://api.github.com/repos/{REPO}/releases/latest"))
        .header("User-Agent", user_agent())
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(30))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let Some(version) = release["tag_name"].as_str().and_then(parse_version) else { return Ok(None) };
    if version <= current_version() {
        return Ok(None);
    }

    let assets = release["assets"].as_array().cloned().unwrap_or_default();
    let url = |name: &str| assets.iter().find(|a| a["name"].as_str() == Some(name)).and_then(|a| a["browser_download_url"].as_str()).map(str::to_string);
    let zip_name = format!("RobloxAccountManager-{}.zip", version_text(version));
    let zip_url = url(&zip_name).ok_or_else(|| anyhow!("The new version has no download"))?;
    let sig_url = url(&format!("{zip_name}.sig")).ok_or_else(|| anyhow!("The new version isn't signed, it can't be installed automatically"))?;
    // downloads only ever come from GitHub, over https
    for u in [&zip_url, &sig_url] {
        if !u.starts_with("https://github.com/") {
            bail!("Unexpected download address");
        }
    }
    Ok(Some(Release { version, zip_url, sig_url }))
}

/// Checks for a newer version; with `download` (or the automatic install setting) it's downloaded right away
pub async fn check(app: &Arc<App>, download: bool) -> Status {
    check_now(app, download, true).await
}

/// `shown`: the interface says "checking" meanwhile (not for the checks in the background, the update button
/// would blink every half hour)
async fn check_now(app: &Arc<App>, download: bool, shown: bool) -> Status {
    if disabled() {
        return Status::Disabled;
    }
    if matches!(status(app), Status::Checking | Status::Downloading { .. }) {
        return status(app);
    }
    if shown {
        set(app, Status::Checking);
    }

    let result = async {
        let Some(release) = latest(&app.api.http).await? else {
            return Ok(Status::UpToDate);
        };
        let version = version_text(release.version);
        let waiting = staged(&app_dir()).map(|(v, _, _)| v);
        if waiting == Some(release.version) {
            return Ok(Status::Ready { version });
        }
        // an older update was already downloaded: it was wanted, so the newest one replaces it
        if download || waiting.is_some_and(|v| v > current_version()) || app.setting_bool(SECTION, "AutoInstall") {
            fetch(app, &release).await?;
            return Ok(Status::Ready { version });
        }
        Ok(Status::Available { version })
    }
    .await;

    let status = result.unwrap_or_else(|e: anyhow::Error| {
        log::warn!("Update: {e:#}");
        Status::Error { message: e.to_string() }
    });
    set(app, status.clone());
    status
}

/// What changed in one version, from its release notes
#[derive(Serialize, Clone, Debug, PartialEq)]
pub struct Notes {
    pub version: String,
    pub date: String,
    pub text: String,
}

/// The notes of every published version newer than this one, newest first; with `recent` the last few versions
/// whatever this one is (to read what changed lately). In `language` when the release has them in it
/// (notes.<language>.md attached to it), in English otherwise.
pub async fn notes(app: &App, recent: bool, language: &str) -> Result<Vec<Notes>> {
    let releases: Value = app
        .api
        .http
        .get(format!("https://api.github.com/repos/{REPO}/releases?per_page=100"))
        .header("User-Agent", user_agent())
        .header("Accept", "application/vnd.github+json")
        .timeout(Duration::from_secs(30))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let mut list = notes_since(&releases, if recent { (0, 0, 0) } else { current_version() }, language);
    if recent {
        list.truncate(RECENT_NOTES);
    }
    // the translations, all at once; a missing or failed one leaves the English text
    let translated = futures_util::future::join_all(list.iter().map(|(_, url)| async move {
        let url = url.as_ref()?;
        let response = app.api.http.get(url).header("User-Agent", user_agent()).timeout(Duration::from_secs(15)).send().await.ok()?.error_for_status().ok()?;
        let text = response.text().await.ok()?;
        (text.len() < 64 * 1024).then(|| whats_new(&text))
    }))
    .await;
    Ok(list.into_iter().zip(translated).map(|((notes, _), text)| Notes { text: text.unwrap_or(notes.text), ..notes }).collect())
}

/// Each newer version's English notes, and where its notes in `language` are when it has them
fn notes_since(releases: &Value, current: Version, language: &str) -> Vec<(Notes, Option<String>)> {
    let wanted = (language != "en" && !language.is_empty() && language.len() <= 10 && language.chars().all(|c| c.is_ascii_alphanumeric() || c == '-'))
        .then(|| format!("notes.{language}.md"));
    let mut list: Vec<(Version, (Notes, Option<String>))> = releases
        .as_array()
        .into_iter()
        .flatten()
        .filter(|r| !r["draft"].as_bool().unwrap_or(true) && !r["prerelease"].as_bool().unwrap_or(false))
        .filter_map(|r| {
            let version = r["tag_name"].as_str().and_then(parse_version).filter(|v| *v > current)?;
            let date = r["published_at"].as_str().unwrap_or_default().chars().take(10).collect();
            let translation = wanted.as_ref().and_then(|name| {
                r["assets"].as_array()?.iter().find(|a| a["name"].as_str() == Some(name))?["browser_download_url"].as_str().filter(|u| u.starts_with("https://github.com/")).map(str::to_string)
            });
            Some((version, (Notes { version: version_text(version), date, text: whats_new(r["body"].as_str().unwrap_or_default()) }, translation)))
        })
        .collect();
    list.sort_by_key(|(v, _)| std::cmp::Reverse(*v));
    list.into_iter().take(MAX_NOTES).map(|(_, n)| n).collect()
}

/// A release's notes without the parts about downloading and checking the files, which the app does itself
fn whats_new(body: &str) -> String {
    let mut keep = true;
    let mut lines = Vec::new();
    for line in body.lines() {
        if let Some(heading) = line.strip_prefix("## ") {
            keep = !["download", "verify", "updating"].contains(&heading.trim().to_lowercase().as_str());
        }
        if keep {
            lines.push(line.trim_end());
        }
    }
    lines.join("\n").trim().to_string()
}

/// Downloads the zip and its signature, checks them, keeps them in `update\` for the next start
async fn fetch(app: &Arc<App>, release: &Release) -> Result<()> {
    let version = version_text(release.version);
    set(app, Status::Downloading { version: version.clone(), progress: 0.0 });
    let http = &app.api.http;

    let signature = http.get(&release.sig_url).header("User-Agent", user_agent()).send().await?.error_for_status()?.text().await?;
    let response = http.get(&release.zip_url).header("User-Agent", user_agent()).timeout(Duration::from_secs(1800)).send().await?.error_for_status()?;
    let total = response.content_length().unwrap_or(0);
    if total > MAX_ZIP {
        bail!("The update is too big");
    }

    let mut zip = Vec::with_capacity(total as usize);
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        zip.extend_from_slice(&chunk?);
        if zip.len() as u64 > MAX_ZIP {
            bail!("The update is too big");
        }
        if total > 0 {
            set(app, Status::Downloading { version: version.clone(), progress: zip.len() as f32 / total as f32 });
        }
    }

    // nothing from the download is kept unless its signature is right
    verify(release.version, &zip, &signature)?;
    unpack(&zip, &mut |_, _| Ok(()))?;

    let target = staging(&app_dir());
    let _ = std::fs::remove_dir_all(&target);
    std::fs::create_dir_all(&target)?;
    std::fs::write(target.join("update.zip"), &zip)?;
    std::fs::write(target.join("update.zip.sig"), signature.trim())?;
    std::fs::write(target.join("version.txt"), &version)?;
    Ok(())
}

/// Each app file in the zip, by its known name only (never a path from the zip), size limited
fn unpack(zip: &[u8], each: &mut dyn FnMut(&str, &[u8]) -> Result<()>) -> Result<()> {
    let mut archive = zip::ZipArchive::new(std::io::Cursor::new(zip))?;
    let mut found = Vec::new();
    for i in 0..archive.len() {
        let entry = archive.by_index(i)?;
        let name = entry.name().rsplit(['/', '\\']).next().unwrap_or_default().to_string();
        let Some(file) = FILES.iter().find(|f| f.eq_ignore_ascii_case(&name)) else { continue };
        if found.contains(file) {
            bail!("The update has {file} twice");
        }
        let mut data = Vec::new();
        entry.take(MAX_FILE + 1).read_to_end(&mut data)?;
        if data.len() as u64 > MAX_FILE {
            bail!("The update is too big");
        }
        each(file, &data)?;
        found.push(*file);
    }
    if !found.contains(&"RobloxAccountManager.exe") {
        bail!("The update has no RobloxAccountManager.exe");
    }
    Ok(())
}

/// The update waiting in `update\`: version, zip, signature
fn staged(dir: &Path) -> Option<(Version, Vec<u8>, String)> {
    let target = staging(dir);
    let version = parse_version(&std::fs::read_to_string(target.join("version.txt")).ok()?)?;
    Some((version, std::fs::read(target.join("update.zip")).ok()?, std::fs::read_to_string(target.join("update.zip.sig")).ok()?))
}

/// Set at start when a newer release than the downloaded update came out: that one gets downloaded instead
static NEWER_THAN_STAGED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

/// At start: installs a downloaded update and starts it, unless a newer one came out since (asked quickly, offline
/// the downloaded one is installed). True when the new version was started (this one has to quit).
pub fn install_staged_at_start(dir: &Path) -> bool {
    if disabled() {
        return false;
    }
    let Some((version, _, _)) = staged(dir) else { return false };
    // test copies never ask GitHub
    if version > current_version() && !dir.join("ram-test-copy").exists() {
        let newest = tauri::async_runtime::block_on(async {
            let http = reqwest::Client::builder().timeout(Duration::from_secs(5)).build().ok()?;
            latest(&http).await.ok().flatten().map(|r| r.version)
        });
        if newest.is_some_and(|n| n > version) {
            log::info!("Update {} is newer than the downloaded {}, getting it first", version_text(newest.unwrap()), version_text(version));
            NEWER_THAN_STAGED.store(true, std::sync::atomic::Ordering::SeqCst);
            return false;
        }
    }
    install_staged(dir)
}

/// Installs the downloaded update and starts it. True when the new version was started (this one has to quit).
/// The signature is checked again here. The running exe can be renamed but not replaced, so the old files become
/// `*.old` for a minute.
fn install_staged(dir: &Path) -> bool {
    if disabled() {
        return false;
    }
    let Some((version, zip, signature)) = staged(dir) else { return false };
    let target = staging(dir);
    if version <= current_version() {
        let _ = std::fs::remove_dir_all(&target);
        return false;
    }
    if let Err(e) = verify(version, &zip, &signature) {
        log::error!("{e}");
        let _ = std::fs::remove_dir_all(&target);
        return false;
    }

    // new files next to the old ones first, then swapped in
    let mut new_files = Vec::new();
    let unpacked = unpack(&zip, &mut |file, data| {
        let path = dir.join(format!("{file}.new"));
        std::fs::write(&path, data)?;
        new_files.push(file.to_string());
        Ok(())
    });
    let remove_new = |files: &[String]| files.iter().for_each(|f| drop(std::fs::remove_file(dir.join(format!("{f}.new")))));
    if let Err(e) = unpacked {
        log::error!("Couldn't unpack the update: {e:#}");
        remove_new(&new_files);
        let _ = std::fs::remove_dir_all(&target);
        return false;
    }

    let mut swapped: Vec<String> = Vec::new();
    let mut failed = false;
    for file in &new_files {
        let (current, old, new) = (dir.join(file), dir.join(format!("{file}.old")), dir.join(format!("{file}.new")));
        let _ = std::fs::remove_file(&old);
        if current.exists() && std::fs::rename(&current, &old).is_err() {
            failed = true;
            break;
        }
        if std::fs::rename(&new, &current).is_err() {
            let _ = std::fs::rename(&old, &current);
            failed = true;
            break;
        }
        swapped.push(file.clone());
    }

    if failed {
        // back to how it was, keep running this version
        for file in &swapped {
            let _ = std::fs::remove_file(dir.join(file));
            let _ = std::fs::rename(dir.join(format!("{file}.old")), dir.join(file));
        }
        remove_new(&new_files);
        log::error!("Couldn't install the update, keeping the current version");
        return false;
    }

    let _ = std::fs::remove_dir_all(&target);
    log::info!("Update {} installed, starting it", version_text(version));
    // the new version waits for this process to be gone: it lets go of the web server's port and the command
    // line's pipe only as it ends, after the lock
    std::process::Command::new(dir.join("RobloxAccountManager.exe"))
        .args(["--updated", "--after", &std::process::id().to_string()])
        .current_dir(dir)
        .spawn()
        .is_ok()
}

/// The previous version's files, once this one has been running for a minute
pub fn clean_old(dir: PathBuf) {
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_secs(60));
        for file in FILES {
            let _ = std::fs::remove_file(dir.join(format!("{file}.old")));
            let _ = std::fs::remove_file(dir.join(format!("{file}.new")));
        }
    });
}

/// "Restart" in the interface: installs the downloaded update now, or the newest one if another came out since
pub async fn restart(app: &Arc<App>) -> Result<()> {
    if !matches!(status(app), Status::Ready { .. }) {
        bail!("No update is ready");
    }
    if let Ok(Some(newest)) = latest(&app.api.http).await {
        if staged(&app_dir()).map(|(v, _, _)| v) != Some(newest.version) {
            if let Err(e) = fetch(app, &newest).await {
                set(app, Status::Error { message: e.to_string() });
                return Err(e);
            }
            set(app, Status::Ready { version: version_text(newest.version) });
        }
    }
    crate::services::antiafk::restore_all(app);
    if !install_staged(&app_dir()) {
        bail!("Couldn't install the update, see log.txt");
    }
    if let Some(handle) = app.handle.get() {
        handle.exit(0);
    }
    Ok(())
}

/// Checks right after start (once the window is up), then every 30 minutes, when automatic checks are on
pub fn spawn(app: Arc<App>) {
    if disabled() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(3)).await;
        if NEWER_THAN_STAGED.load(std::sync::atomic::Ordering::SeqCst) {
            check(&app, true).await;
        }
        loop {
            if app.setting_bool(SECTION, "Check") {
                check_now(&app, false, false).await;
            }
            tokio::time::sleep(CHECK_EVERY).await;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use ed25519_dalek::{Signer, SigningKey};

    #[test]
    fn notes_of_each_newer_version_newest_first() {
        let releases = serde_json::json!([
            { "tag_name": "v4.1.12", "draft": false, "prerelease": false, "published_at": "2026-10-16T10:00:00Z",
              "body": "## What's new\r\n\r\n- **B**\r\n\r\n## Download\r\n\r\n- zip\r\n\r\n## Verify\r\n\r\n| File |" },
            { "tag_name": "v4.1.13", "draft": true, "prerelease": false, "body": "draft" },
            { "tag_name": "v4.1.11", "draft": false, "prerelease": false, "published_at": "2026-10-15T10:00:00Z",
              "body": "## Changes\n\n- A\n\n## Updating\n\nextract the zip" },
            { "tag_name": "v4.1.10", "draft": false, "prerelease": false, "body": "the current one" },
        ]);
        let notes: Vec<Notes> = notes_since(&releases, (4, 1, 10), "en").into_iter().map(|(n, _)| n).collect();
        assert_eq!(notes.iter().map(|n| n.version.as_str()).collect::<Vec<_>>(), ["4.1.12", "4.1.11"]);
        assert_eq!(notes[0].text, "## What's new\n\n- **B**");
        assert_eq!(notes[0].date, "2026-10-16");
        assert_eq!(notes[1].text, "## Changes\n\n- A");
    }

    #[test]
    fn notes_in_the_app_language_when_the_release_has_them() {
        let releases = serde_json::json!([{ "tag_name": "v4.1.25", "draft": false, "prerelease": false, "body": "## What's new", "assets": [
            { "name": "notes.pt-PT.md", "browser_download_url": "https://github.com/x/y/releases/download/v4.1.25/notes.pt-PT.md" },
            { "name": "notes.de.md", "browser_download_url": "https://evil.example/notes.de.md" },
        ] }]);
        let url = |language: &str| notes_since(&releases, (4, 1, 24), language).remove(0).1;
        assert_eq!(url("pt-PT").as_deref(), Some("https://github.com/x/y/releases/download/v4.1.25/notes.pt-PT.md"));
        // English is the release text; no file in that language; only from GitHub; never a path
        assert_eq!(url("en"), None);
        assert_eq!(url("fr"), None);
        assert_eq!(url("de"), None);
        assert_eq!(url("../x"), None);
    }

    #[test]
    fn versions() {
        assert_eq!(parse_version("v4.0.1"), Some((4, 0, 1)));
        assert_eq!(parse_version("4.10.0"), Some((4, 10, 0)));
        assert!(parse_version("v4.10.0") > parse_version("v4.9.9"));
        assert_eq!(parse_version("latest"), None);
        assert_eq!(parse_version("4.0.1.5"), None);
    }

    #[test]
    fn only_zips_signed_with_the_release_key_are_accepted() {
        let zip = b"a release zip";
        let b64 = base64::engine::general_purpose::STANDARD;
        // signed by some other key
        let other = SigningKey::from_bytes(&[7u8; 32]);
        let signature = b64.encode(other.sign(&signed_message((9, 9, 9), zip)).to_bytes());
        assert!(verify((9, 9, 9), zip, &signature).is_err());
        assert!(verify((9, 9, 9), zip, "not base64!").is_err());
        assert!(verify((9, 9, 9), zip, "").is_err());
    }

    #[test]
    fn the_version_is_part_of_the_signature() {
        // an old zip can't be passed off as a newer version: the message differs
        assert_ne!(signed_message((4, 0, 1), b"zip"), signed_message((9, 0, 0), b"zip"));
    }

    fn zip_of(files: &[(&str, &str)]) -> Vec<u8> {
        use std::io::Write;
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        for (name, body) in files {
            zip.start_file(*name, zip::write::SimpleFileOptions::default()).unwrap();
            zip.write_all(body.as_bytes()).unwrap();
        }
        zip.finish().unwrap().into_inner()
    }

    #[test]
    fn unpacks_only_the_app_files_by_name() {
        let zip = zip_of(&[("RobloxAccountManager.exe", "app"), ("sub/ram-cli.exe", "cli"), ("../evil.exe", "no"), ("AccountData.json", "no")]);
        let mut got = Vec::new();
        unpack(&zip, &mut |file, data| {
            got.push((file.to_string(), String::from_utf8_lossy(data).to_string()));
            Ok(())
        })
        .unwrap();
        assert_eq!(got, vec![("RobloxAccountManager.exe".into(), "app".into()), ("ram-cli.exe".into(), "cli".into())]);
    }

    #[test]
    fn refuses_a_zip_without_the_app_or_with_it_twice() {
        assert!(unpack(&zip_of(&[("ram-cli.exe", "cli")]), &mut |_, _| Ok(())).is_err());
        assert!(unpack(&zip_of(&[("RobloxAccountManager.exe", "a"), ("x/RobloxAccountManager.exe", "b")]), &mut |_, _| Ok(())).is_err());
    }
}
