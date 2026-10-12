//! Signs a release zip for the auto updater, and makes the signing key the first time.
//!
//!     cargo run --example sign-release -- new-key           makes the key (once, keep a backup of it)
//!     cargo run --example sign-release -- <zip>             writes <zip>.sig next to the zip
//!
//! The zip must be named RobloxAccountManager-<version>.zip: the version is signed with it, so an old zip can't be
//! published again as a newer version.
//!
//! The private key lives in %USERPROFILE%\.roblox-account-manager\release-signing-key.txt, never in the repository.
//! The app only installs updates signed with it (its public half is `PUBLIC_KEY` in src/services/updater.rs).
//! Losing the key means existing installs can't update by themselves anymore: keep a copy somewhere safe.

use base64::Engine;
use ed25519_dalek::{Signer, SigningKey};
use rand::RngCore;
use std::path::PathBuf;

fn key_file() -> PathBuf {
    let home = std::env::var("USERPROFILE").expect("USERPROFILE");
    PathBuf::from(home).join(".roblox-account-manager").join("release-signing-key.txt")
}

fn main() {
    let b64 = base64::engine::general_purpose::STANDARD;
    let arg = std::env::args().nth(1).expect("new-key, or the zip to sign");

    if arg == "new-key" {
        let file = key_file();
        if file.exists() {
            eprintln!("{} already exists, not replacing it (updates are checked against it)", file.display());
            std::process::exit(1);
        }
        let mut secret = [0u8; 32];
        rand::rngs::OsRng.fill_bytes(&mut secret);
        let key = SigningKey::from_bytes(&secret);
        std::fs::create_dir_all(file.parent().unwrap()).unwrap();
        std::fs::write(&file, b64.encode(secret)).unwrap();
        println!("private key: {}", file.display());
        println!("public key (PUBLIC_KEY in src/services/updater.rs): {}", b64.encode(key.verifying_key().to_bytes()));
        return;
    }

    let secret: [u8; 32] = b64
        .decode(std::fs::read_to_string(key_file()).expect("no signing key, run: cargo run --example sign-release -- new-key").trim())
        .expect("bad key file")
        .try_into()
        .expect("bad key length");
    let key = SigningKey::from_bytes(&secret);
    let public = b64.encode(key.verifying_key().to_bytes());
    if public != ram::services::updater::PUBLIC_KEY {
        eprintln!("this key ({public}) isn't the one the app trusts ({})", ram::services::updater::PUBLIC_KEY);
        std::process::exit(1);
    }
    let name = std::path::Path::new(&arg).file_name().and_then(|n| n.to_str()).unwrap_or_default();
    let version = name
        .strip_prefix("RobloxAccountManager-")
        .and_then(|n| n.strip_suffix(".zip"))
        .and_then(ram::services::updater::parse_version)
        .expect("the zip must be named RobloxAccountManager-<version>.zip");
    let zip = std::fs::read(&arg).expect("can't read the zip");
    let signature = key.sign(&ram::services::updater::signed_message(version, &zip));
    // the same check the app does before installing
    ram::services::updater::verify(version, &zip, &b64.encode(signature.to_bytes())).expect("signature check failed");
    let out = format!("{arg}.sig");
    std::fs::write(&out, b64.encode(signature.to_bytes())).unwrap();
    println!("{out} written, version {}.{}.{}", version.0, version.1, version.2);
}
