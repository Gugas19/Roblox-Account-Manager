//! Options (`--place 123`, `--place=123`, `--all`) and picking accounts by name.

use crate::app::App;
use crate::core::accounts::Account;
use anyhow::{bail, Result};

/// A wrong command line: exit code 2 and a pointer to the help
#[derive(Debug)]
pub struct Usage(pub String);

impl std::fmt::Display for Usage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Usage {}

pub fn usage(text: impl Into<String>) -> anyhow::Error {
    Usage(text.into()).into()
}

pub struct Args {
    pub words: Vec<String>,
    flags: Vec<(String, Option<String>)>,
}

impl Args {
    /// `switches` are on/off (`--json`), `valued` take a value (`--place 123`)
    pub fn parse(args: &[String], switches: &[&str], valued: &[&str]) -> Result<Args> {
        let mut words = Vec::new();
        let mut flags = Vec::new();
        let mut rest = args.iter();

        while let Some(arg) = rest.next() {
            let Some(flag) = arg.strip_prefix("--") else {
                words.push(arg.clone());
                continue;
            };
            let (name, inline) = match flag.split_once('=') {
                Some((name, value)) => (name, Some(value.to_string())),
                None => (flag, None),
            };
            if valued.contains(&name) {
                let value = match inline {
                    Some(v) => v,
                    None => rest.next().cloned().ok_or_else(|| usage(format!("--{name} needs a value")))?,
                };
                flags.push((name.to_string(), Some(value)));
            } else if switches.contains(&name) && inline.is_none() {
                flags.push((name.to_string(), None));
            } else {
                return Err(usage(format!("Unknown option --{name}")));
            }
        }
        Ok(Args { words, flags })
    }

    pub fn has(&self, name: &str) -> bool {
        self.flags.iter().any(|(n, _)| n == name)
    }

    pub fn value(&self, name: &str) -> Option<&str> {
        self.flags.iter().rev().find(|(n, _)| n == name).and_then(|(_, v)| v.as_deref())
    }
}

pub fn in_group(group: &str, wanted: &str) -> bool {
    group.eq_ignore_ascii_case(wanted) || Account::group_title(group).eq_ignore_ascii_case(wanted)
}

/// The accounts named on the command line, in that order: username, user id or alias, `--all`, `--group`
pub fn pick(app: &App, args: &Args) -> Result<Vec<Account>> {
    let store = app.store.lock().unwrap();
    let accounts = &store.accounts;
    if args.has("all") {
        return Ok(accounts.clone());
    }

    let mut picked: Vec<Account> = Vec::new();
    let mut add = |a: &Account| {
        if !picked.iter().any(|p| p.username == a.username) {
            picked.push(a.clone());
        }
    };

    if let Some(group) = args.value("group") {
        let members: Vec<&Account> = accounts.iter().filter(|a| in_group(&a.group, group)).collect();
        if members.is_empty() {
            bail!("No group named {group}");
        }
        members.into_iter().for_each(&mut add);
    }

    for name in &args.words {
        if let Some(a) = accounts.iter().find(|a| a.username.eq_ignore_ascii_case(name) || a.user_id.to_string() == *name) {
            add(a);
            continue;
        }
        let by_alias: Vec<&Account> = accounts.iter().filter(|a| !a.alias.is_empty() && a.alias.eq_ignore_ascii_case(name)).collect();
        match by_alias.as_slice() {
            [a] => add(a),
            [] => bail!("No account named {name}"),
            _ => bail!("Several accounts have the alias {name}, use the username"),
        }
    }

    if picked.is_empty() {
        return Err(usage("Name the accounts, or use --all or --group <name>"));
    }
    Ok(picked)
}
