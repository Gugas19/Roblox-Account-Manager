//! Updates from the GitHub releases, signed with the project's release key.
//!
//! Every release has `RobloxAccountManager-<version>.zip` and `<zip>.sig`: an Ed25519 signature, made with the
//! maintainer's private key (`cargo run --example sign-release`), of the version followed by the zip. So:
//! - a damaged or changed download, or a release not signed with that key, is refused;
//! - an old signed zip published as a newer version is refused too (the version is part of what's signed);
//! - only newer versions are installed.
//!
//! Check on start and every 6 hours. The verified zip waits in `update\` next to the app, and is verified again
//! when it's installed: the next time the app starts, or right away with "Restart". Builds run from a cargo
//! `target` folder (development, tests) never update themselves.

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
const CHECK_EVERY: Duration = Duration::from_secs(6 * 3600);
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

async fn latest(app: &App) -> Result<Option<Release>> {
    let release: Value = app
        .api
        .http
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
    if disabled() {
        return Status::Disabled;
    }
    if matches!(status(app), Status::Checking | Status::Downloading { .. }) {
        return status(app);
    }
    set(app, Status::Checking);

    let result = async {
        let Some(release) = latest(app).await? else {
            return Ok(Status::UpToDate);
        };
        let version = version_text(release.version);
        if staged(&app_dir()).map(|(v, _, _)| v) == Some(release.version) {
            return Ok(Status::Ready { version });
        }
        if download || app.setting_bool(SECTION, "AutoInstall") {
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

/// At start: installs a downloaded update and starts it. True when the new version was started (this one has
/// to quit). The signature is checked again here. The running exe can be renamed but not replaced, so the old
/// files become `*.old` for a minute.
pub fn install_staged(dir: &Path) -> bool {
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
    std::process::Command::new(dir.join("RobloxAccountManager.exe")).arg("--updated").current_dir(dir).spawn().is_ok()
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

/// "Restart" in the interface: installs the downloaded update now
pub fn restart(app: &App) -> Result<()> {
    if !matches!(status(app), Status::Ready { .. }) {
        bail!("No update is ready");
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

/// Checks a little after start, then every 6 hours, when automatic checks are on
pub fn spawn(app: Arc<App>) {
    if disabled() {
        return;
    }
    tauri::async_runtime::spawn(async move {
        tokio::time::sleep(Duration::from_secs(20)).await;
        loop {
            if app.setting_bool(SECTION, "Check") {
                check(&app, false).await;
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
