//! Account data protection, byte for byte compatible with the C# version:
//! - default: Windows DPAPI with the app's entropy (files from LocalMachine or CurrentUser scope both open)
//! - password: header + salt(16) + nonce(24) + crypto_secretbox_easy(json), key = argon2i(sha512(password))

use anyhow::{anyhow, bail, Result};
use argon2::{Algorithm, Argon2, Params, Version};
use crypto_secretbox::aead::{Aead, KeyInit};
use crypto_secretbox::XSalsa20Poly1305;
use rand::RngCore;
use sha2::{Digest, Sha512};

/// "Roblox Account Manager created by ic3w0lf22 @ github.com ......."
pub const RAM_HEADER: &[u8; 64] = b"Roblox Account Manager created by ic3w0lf22 @ github.com .......";

/// DPAPI entropy used by the C# version for AccountData.json
pub const ENTROPY: &[u8] = &[
    0x52, 0x4f, 0x42, 0x4c, 0x4f, 0x58, 0x20, 0x41, 0x43, 0x43, 0x4f, 0x55, 0x4e, 0x54, 0x20, 0x4d, 0x41, 0x4e, 0x41, 0x47, 0x45, 0x52, 0x20, 0x7c,
    0x20, 0x3a, 0x29, 0x20, 0x7c, 0x20, 0x42, 0x52, 0x4f, 0x55, 0x47, 0x48, 0x54, 0x20, 0x54, 0x4f, 0x20, 0x59, 0x4f, 0x55, 0x20, 0x42, 0x55, 0x59,
    0x20, 0x69, 0x63, 0x33, 0x77, 0x30, 0x6c, 0x66,
];

/// libsodium crypto_pwhash_argon2i "moderate" limits used by libsodium-net's StrengthArgon.Moderate
const OPS_LIMIT: u32 = 6;
const MEM_LIMIT_KIB: u32 = 134_217_728 / 1024;

/// What the C# version keeps as "PasswordHash": SHA-512 of the password (libsodium crypto_hash)
pub fn password_hash(password: &str) -> Vec<u8> {
    Sha512::digest(password.as_bytes()).to_vec()
}

pub fn is_password_protected(data: &[u8]) -> bool {
    data.len() >= RAM_HEADER.len() && &data[..RAM_HEADER.len()] == RAM_HEADER
}

fn derive_key(hash: &[u8], salt: &[u8]) -> Result<[u8; 32]> {
    let params = Params::new(MEM_LIMIT_KIB, OPS_LIMIT, 1, Some(32)).map_err(|e| anyhow!("{e}"))?;
    let mut key = [0u8; 32];
    Argon2::new(Algorithm::Argon2i, Version::V0x13, params).hash_password_into(hash, salt, &mut key).map_err(|e| anyhow!("{e}"))?;
    Ok(key)
}

pub fn encrypt(content: &str, hash: &[u8]) -> Result<Vec<u8>> {
    if content.is_empty() {
        bail!("Missing Content");
    }

    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 24];
    rand::thread_rng().fill_bytes(&mut salt);
    rand::thread_rng().fill_bytes(&mut nonce);

    let key = derive_key(hash, &salt)?;
    let sealed = XSalsa20Poly1305::new(&key.into()).encrypt(&nonce.into(), content.as_bytes()).map_err(|_| anyhow!("encryption failed"))?;

    let mut out = Vec::with_capacity(64 + 16 + 24 + sealed.len());
    out.extend_from_slice(RAM_HEADER);
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&sealed);
    Ok(out)
}

#[derive(Debug)]
pub struct WrongPassword;

impl std::fmt::Display for WrongPassword {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Incorrect Password!")
    }
}

impl std::error::Error for WrongPassword {}

pub fn decrypt(data: &[u8], hash: &[u8]) -> Result<Vec<u8>> {
    if !is_password_protected(data) {
        bail!("Missing RAMHeader");
    }
    if data.len() < 64 + 16 + 24 + 16 {
        bail!("The accounts file is damaged");
    }

    let salt = &data[64..80];
    let nonce: [u8; 24] = data[80..104].try_into().unwrap();
    let key = derive_key(hash, salt)?;

    XSalsa20Poly1305::new(&key.into()).decrypt(&nonce.into(), &data[104..]).map_err(|_| anyhow::Error::new(WrongPassword))
}

#[cfg(windows)]
mod dpapi {
    use anyhow::{anyhow, Result};
    use windows::core::PCWSTR;
    use windows::Win32::Foundation::{LocalFree, HLOCAL};
    use windows::Win32::Security::Cryptography::{CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB};

    fn blob(data: &[u8]) -> CRYPT_INTEGER_BLOB {
        CRYPT_INTEGER_BLOB { cbData: data.len() as u32, pbData: data.as_ptr() as *mut u8 }
    }

    unsafe fn take(out: CRYPT_INTEGER_BLOB) -> Vec<u8> {
        let bytes = std::slice::from_raw_parts(out.pbData, out.cbData as usize).to_vec();
        let _ = LocalFree(HLOCAL(out.pbData as *mut _));
        bytes
    }

    /// CurrentUser scope, only this Windows user can read it back
    pub fn protect(data: &[u8], entropy: &[u8]) -> Result<Vec<u8>> {
        unsafe {
            let input = blob(data);
            let entropy = blob(entropy);
            let mut out = CRYPT_INTEGER_BLOB::default();
            CryptProtectData(&input, PCWSTR::null(), Some(&entropy), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out).map_err(|e| anyhow!("{e}"))?;
            Ok(take(out))
        }
    }

    pub fn unprotect(data: &[u8], entropy: &[u8]) -> Result<Vec<u8>> {
        unsafe {
            let input = blob(data);
            let entropy = blob(entropy);
            let mut out = CRYPT_INTEGER_BLOB::default();
            CryptUnprotectData(&input, None, Some(&entropy), None, None, CRYPTPROTECT_UI_FORBIDDEN, &mut out).map_err(|e| anyhow!("{e}"))?;
            Ok(take(out))
        }
    }
}

#[cfg(windows)]
pub use dpapi::{protect, unprotect};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn password_round_trip() {
        let hash = password_hash("teste1234");
        let sealed = encrypt("[{\"Username\":\"A\"}]", &hash).unwrap();

        assert!(is_password_protected(&sealed));
        assert!(!String::from_utf8_lossy(&sealed).contains("Username"));
        assert_eq!(decrypt(&sealed, &hash).unwrap(), b"[{\"Username\":\"A\"}]");
        assert!(decrypt(&sealed, &password_hash("wrong")).unwrap_err().downcast_ref::<WrongPassword>().is_some());
    }

    /// A file written by the C# version (Tests/Unit, `--fixture`), password "teste1234"
    #[test]
    fn opens_files_from_the_csharp_version() {
        let data = std::fs::read(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/password-teste1234.bin")).unwrap();
        let json = decrypt(&data, &password_hash("teste1234")).unwrap();
        assert!(String::from_utf8(json).unwrap().contains("NightFalcon_07"));
    }

    #[test]
    fn dpapi_round_trip() {
        let sealed = protect(b"secret", ENTROPY).unwrap();
        assert_eq!(unprotect(&sealed, ENTROPY).unwrap(), b"secret");
        assert!(unprotect(&sealed, b"other").is_err());
    }
}
