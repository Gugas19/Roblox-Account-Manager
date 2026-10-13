//! Backups of the accounts: the same password encryption as a protected AccountData.json, so a backup is never
//! readable without its password, opens on any PC, and can even be used as an AccountData.json.

use super::*;
use crate::core::accounts::AccountStore;
use crate::core::crypto;

#[derive(Serialize)]
pub struct ImportedBackup {
    pub added: usize,
    pub existing: usize,
}

/// The accounts of `incoming` that `current` doesn't have (same user id or username) are added
pub fn merge_accounts(current: &mut Vec<Account>, incoming: Vec<Account>) -> ImportedBackup {
    let (mut added, mut existing) = (0, 0);
    for account in incoming {
        let known = current.iter().any(|a| (account.user_id != 0 && a.user_id == account.user_id) || a.username.eq_ignore_ascii_case(&account.username));
        if known {
            existing += 1;
        } else {
            current.push(account);
            added += 1;
        }
    }
    ImportedBackup { added, existing }
}

/// Asks where to save, then writes the accounts encrypted with `password`. `None`: the user cancelled.
#[tauri::command]
pub fn export_accounts(password: String) -> R<Option<String>> {
    if password.chars().count() < 4 {
        return Err("At least 4 characters. It can't be recovered if you forget it.".into());
    }
    let app = app();
    let accounts = app.store.lock().unwrap().accounts.clone();
    if accounts.is_empty() {
        return Err("No accounts yet".into());
    }

    let name = format!("RobloxAccountManager-backup-{}.json", chrono::Local::now().format("%Y-%m-%d"));
    let Some(path) = rfd::FileDialog::new().set_title("Export accounts").set_file_name(&name).add_filter("Accounts backup", &["json"]).save_file() else {
        return Ok(None);
    };
    let json = serde_json::to_string(&accounts).map_err(e)?;
    let sealed = crypto::encrypt(&json, &crypto::password_hash(&password)).map_err(e)?;
    std::fs::write(&path, sealed).map_err(e)?;
    log::info!("Exported {} accounts to a backup", accounts.len());
    Ok(Some(path.to_string_lossy().to_string()))
}

/// Opens a backup with its password and adds the accounts that aren't here yet
#[tauri::command]
pub fn import_backup(path: String, password: String) -> R<ImportedBackup> {
    let data = std::fs::read(&path).map_err(e)?;
    if !crypto::is_password_protected(&data) {
        return Err("That file isn't an accounts backup".into());
    }
    let plain = crypto::decrypt(&data, &crypto::password_hash(&password)).map_err(e)?;
    let incoming = AccountStore::parse(&plain).map_err(e)?;

    let app = app();
    let result = merge_accounts(&mut app.store.lock().unwrap().accounts, incoming);
    app.save_accounts(true);
    log::info!("Imported a backup: {} added, {} already there", result.added, result.existing);
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn account(name: &str, id: i64) -> Account {
        Account { username: name.into(), user_id: id, ..Default::default() }
    }

    #[test]
    fn only_new_accounts_are_added() {
        let mut current = vec![account("Main", 1), account("Alt", 2)];
        let result = merge_accounts(&mut current, vec![account("main", 1), account("Renamed", 2), account("New", 3)]);
        assert_eq!((result.added, result.existing), (1, 2));
        assert_eq!(current.iter().map(|a| a.username.as_str()).collect::<Vec<_>>(), ["Main", "Alt", "New"]);
    }

    #[test]
    fn a_backup_opens_only_with_its_password() {
        let json = serde_json::to_string(&vec![account("Main", 1)]).unwrap();
        let sealed = crypto::encrypt(&json, &crypto::password_hash("backup-pass")).unwrap();
        assert!(crypto::is_password_protected(&sealed));
        assert!(!String::from_utf8_lossy(&sealed).contains("Main"));
        assert!(crypto::decrypt(&sealed, &crypto::password_hash("wrong")).is_err());
        let plain = crypto::decrypt(&sealed, &crypto::password_hash("backup-pass")).unwrap();
        assert_eq!(AccountStore::parse(&plain).unwrap()[0].username, "Main");
    }
}
