//! Downloads Chrome for Testing into the same folder the C# version used, so it isn't downloaded twice.

use anyhow::{anyhow, Result};
use futures_util::StreamExt;
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::time::Duration;

pub fn browser_folder() -> PathBuf {
    let base = std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_else(|_| std::env::temp_dir());
    base.join("RobloxAccountManager").join("Chrome")
}

fn version_of(path: &Path) -> Vec<u32> {
    path.file_name()
        .and_then(|n| n.to_str())
        .and_then(|n| n.strip_prefix("Win64-"))
        .map(|v| v.split('.').filter_map(|p| p.parse().ok()).collect())
        .unwrap_or_default()
}

/// The newest installed chrome.exe (layout: Chrome\Win64-<version>\chrome-win64\chrome.exe)
pub fn installed_browser() -> Option<PathBuf> {
    let root = browser_folder().join("Chrome");
    let mut versions: Vec<PathBuf> = std::fs::read_dir(&root).ok()?.filter_map(|e| e.ok()).map(|e| e.path()).filter(|p| p.join("chrome-win64").join("chrome.exe").exists()).collect();
    versions.sort_by_key(|p| version_of(p));
    versions.pop().map(|p| p.join("chrome-win64").join("chrome.exe"))
}

/// The biggest Chrome download accepted (it's about 170 MB)
const MAX_DOWNLOAD: u64 = 600 * 1024 * 1024;

/// The only address Chrome is downloaded from: Google's Chrome for Testing storage, for a plain version number.
/// The version list could point anywhere, and what's downloaded is run
fn download_url(version: &str, listed: &str) -> Result<String> {
    let parts: Vec<&str> = version.split('.').collect();
    if parts.len() != 4 || parts.iter().any(|p| p.is_empty() || p.len() > 6 || !p.chars().all(|c| c.is_ascii_digit())) {
        return Err(anyhow!("Unexpected Chrome version {version:?}"));
    }
    let expected = format!("https://storage.googleapis.com/chrome-for-testing-public/{version}/win64/chrome-win64.zip");
    if listed != expected {
        return Err(anyhow!("Unexpected Chrome download address {listed}"));
    }
    Ok(expected)
}

/// Downloads the latest stable Chrome for Testing, progress goes from 0 to 1
pub async fn install(http: &reqwest::Client, progress: impl Fn(f32)) -> Result<PathBuf> {
    let info: Value = http
        .get("https://googlechromelabs.github.io/chrome-for-testing/last-known-good-versions-with-downloads.json")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    let stable = &info["channels"]["Stable"];
    let version = stable["version"].as_str().ok_or_else(|| anyhow!("no stable version"))?.to_string();
    let url = stable["downloads"]["chrome"]
        .as_array()
        .and_then(|list| list.iter().find(|d| d["platform"] == "win64"))
        .and_then(|d| d["url"].as_str())
        .ok_or_else(|| anyhow!("no win64 download"))?;
    let url = download_url(&version, url)?;

    let target = browser_folder().join("Chrome").join(format!("Win64-{version}"));
    if target.join("chrome-win64").join("chrome.exe").exists() {
        return Ok(target.join("chrome-win64").join("chrome.exe"));
    }

    std::fs::create_dir_all(browser_folder())?;
    let zip_path = browser_folder().join(format!("chrome-{version}.zip"));

    let response = http.get(&url).timeout(Duration::from_secs(1800)).send().await?.error_for_status()?;
    let total = response.content_length().unwrap_or(0);
    if total > MAX_DOWNLOAD {
        return Err(anyhow!("The Chrome download is too big ({total} bytes)"));
    }
    let mut file = tokio::fs::File::create(&zip_path).await?;
    let mut stream = response.bytes_stream();
    let mut read = 0u64;
    let mut hash = sha2::Sha256::default();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        read += chunk.len() as u64;
        if read > MAX_DOWNLOAD {
            drop(file);
            let _ = std::fs::remove_file(&zip_path);
            return Err(anyhow!("The Chrome download is too big ({read} bytes)"));
        }
        sha2::Digest::update(&mut hash, &chunk);
        tokio::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
        if total > 0 {
            progress(read as f32 / total as f32 * 0.95);
        }
    }
    drop(file);
    // what was installed, for anyone checking it against Google's file
    let sha256: String = sha2::Digest::finalize(hash).iter().map(|b| format!("{b:02x}")).collect();
    log::info!("Downloaded {url}: {read} bytes, SHA-256 {sha256}");

    let zip_for_thread = zip_path.clone();
    let target_for_thread = target.clone();
    tokio::task::spawn_blocking(move || -> Result<()> {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&zip_for_thread)?)?;
        let partial = target_for_thread.with_extension("partial");
        let _ = std::fs::remove_dir_all(&partial);
        archive.extract(&partial)?;
        std::fs::rename(&partial, &target_for_thread)?;
        Ok(())
    })
    .await??;

    let _ = std::fs::remove_file(&zip_path);
    progress(1.0);
    log::info!("Browser ready: Chrome {version}");

    Ok(target.join("chrome-win64").join("chrome.exe"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn versions_sort_numerically() {
        assert!(version_of(Path::new("Win64-154.0.8037.57")) > version_of(Path::new("Win64-99.0.1.2")));
    }

    #[test]
    fn chrome_only_comes_from_googles_storage() {
        let good = "https://storage.googleapis.com/chrome-for-testing-public/141.0.7390.122/win64/chrome-win64.zip";
        assert_eq!(download_url("141.0.7390.122", good).unwrap(), good);
        assert!(download_url("141.0.7390.122", "https://evil.example/chrome-win64.zip").is_err());
        assert!(download_url("141.0.7390.122", "http://storage.googleapis.com/chrome-for-testing-public/141.0.7390.122/win64/chrome-win64.zip").is_err());
        // a version that would change the path
        assert!(download_url("../../x", "https://storage.googleapis.com/chrome-for-testing-public/../../x/win64/chrome-win64.zip").is_err());
        assert!(download_url("141.0.7390", good).is_err());
    }
}
