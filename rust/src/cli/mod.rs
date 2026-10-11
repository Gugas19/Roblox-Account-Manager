//! Command line (`ram-cli.exe`).
//!
//! When Roblox Account Manager is open the command runs inside it, through a named pipe, so the window
//! updates live and Anti-AFK can be controlled. Otherwise ram-cli opens the accounts itself, without a window.
//!
//! - `args`: options and picking accounts by name
//! - `run`: the commands
//! - `pipe`: talking to the open app

mod args;
pub mod pipe;
mod run;

use crate::app;
use crate::core::accounts::LoadState;
use crate::platform::win::NamedMutex;
use serde::{Deserialize, Serialize};
use std::io::{IsTerminal, Read, Write};
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

pub const HELP: &str = "\
Roblox Account Manager command line

Usage: ram-cli <command> [options]

Accounts are named by username, user id or alias (any case).
--all picks every account, --group <name> the accounts of a group.

Commands:
  list [--group <name>] [--json]     the accounts (never their cookies)
  join <accounts> [--place <id or game link>] [--job <job id or VIP link>]
                                     launch the accounts into a game, one after the other.
                                     Without --place each account uses its saved place.
  follow <accounts> --user <name>    join the game a player is in
  close <accounts> | --all           close the accounts' Roblox, or every Roblox
  import [file]                      add accounts from cookies, one per line (file or input)
  antiafk [start|stop|now|status]    Anti-AFK (start, stop and now need the app open)
  status                             the app, accounts, Roblox and Anti-AFK at a glance
  open                               show the app window, starting it if needed
  help, version

When Roblox Account Manager is open, commands run inside it. Otherwise ram-cli
opens the accounts file itself: keep ram-cli.exe next to RobloxAccountManager.exe.
A password protected file asks for the password, or reads RAM_PASSWORD.

Exit codes: 0 done, 1 failed, 2 wrong command line.

Examples:
  ram-cli join Main Alt2 --place 920587237
  ram-cli join --group Farm --place https://www.roblox.com/games/920587237
  ram-cli follow Alt1 --user Builderman
  ram-cli close --all
  ram-cli list --json
";

/// What ram-cli sends to the open app
#[derive(Serialize, Deserialize, Debug)]
pub struct Request {
    pub args: Vec<String>,
    /// `import`: the cookies, read by ram-cli (the app can't see its files or input)
    #[serde(default)]
    pub input: Option<String>,
}

impl Request {
    fn new(args: Vec<String>) -> Result<Request, String> {
        let mut input = None;
        if args[0] == "import" {
            input = Some(match args.get(1) {
                Some(file) => std::fs::read_to_string(file).map_err(|e| format!("Can't read {file}: {e}"))?,
                None => {
                    if std::io::stdin().is_terminal() {
                        eprintln!("Paste the cookies, one per line, then press Ctrl+Z and Enter:");
                    }
                    let mut text = String::new();
                    std::io::stdin().read_to_string(&mut text).map_err(|e| e.to_string())?;
                    text
                }
            });
        }
        Ok(Request { args, input })
    }
}

/// What the app answers, one per line
#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Reply {
    Out(String),
    Err(String),
    Exit(i32),
}

/// Where a command writes: the terminal, or the pipe back to ram-cli
#[derive(Clone)]
pub struct Out(Arc<dyn Fn(Reply) + Send + Sync>);

impl Out {
    pub fn new(write: impl Fn(Reply) + Send + Sync + 'static) -> Out {
        Out(Arc::new(write))
    }

    fn terminal() -> Out {
        Out::new(|reply| match reply {
            Reply::Out(text) => println!("{text}"),
            Reply::Err(text) => eprintln!("{text}"),
            Reply::Exit(_) => {}
        })
    }

    pub fn send(&self, reply: Reply) {
        (self.0)(reply)
    }

    pub fn line(&self, text: impl Into<String>) {
        self.send(Reply::Out(text.into()))
    }

    pub fn error(&self, text: impl Into<String>) {
        self.send(Reply::Err(text.into()))
    }
}

/// `ram-cli.exe`, returns the exit code
pub fn main() -> i32 {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        None | Some("help" | "-h" | "--help" | "/?") => {
            print!("{HELP}");
            return 0;
        }
        Some("version" | "-V" | "--version") => {
            println!("ram-cli {}", env!("CARGO_PKG_VERSION"));
            return 0;
        }
        _ => {}
    }

    let request = match Request::new(args) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e}");
            return 1;
        }
    };
    tauri::async_runtime::block_on(start(request))
}

async fn start(request: Request) -> i32 {
    let out = Out::terminal();
    match pipe::send(&request, &out).await {
        Ok(Some(code)) => return code,
        Ok(None) => {}
        Err(e) => {
            out.error(format!("{e:#}"));
            return 1;
        }
    }

    if matches!(request.args[0].as_str(), "open" | "show") {
        return start_app(&out);
    }
    without_app(request, &out).await
}

fn app_dir() -> PathBuf {
    std::env::current_exe().ok().and_then(|p| p.parent().map(|p| p.to_path_buf())).unwrap_or_default()
}

fn start_app(out: &Out) -> i32 {
    let exe = app_dir().join("RobloxAccountManager.exe");
    match std::process::Command::new(&exe).current_dir(app_dir()).spawn() {
        Ok(_) => {
            out.line("Opening Roblox Account Manager");
            0
        }
        Err(e) => {
            out.error(format!("Could not start {}: {e}", exe.display()));
            1
        }
    }
}

/// The app isn't open: open the accounts here, like the app would, and run the command
async fn without_app(request: Request, out: &Out) -> i32 {
    let dir = app_dir();
    let _ = std::env::set_current_dir(&dir);

    let _instance = match NamedMutex::acquire(crate::INSTANCE_MUTEX) {
        Ok((mutex, true)) => mutex,
        _ => {
            // the app may be starting, its command line comes up a moment later
            for _ in 0..20 {
                tokio::time::sleep(Duration::from_millis(150)).await;
                if let Ok(Some(code)) = pipe::send(&request, out).await {
                    return code;
                }
            }
            out.error("Another Roblox Account Manager is open without a command line (the old version?), close it first");
            return 1;
        }
    };

    crate::platform::logging::init(&dir);
    let app = app::init(dir);
    if let Err(e) = open_accounts(&app, &request.args[0]) {
        out.error(e);
        return 1;
    }
    app.ready.store(true, std::sync::atomic::Ordering::SeqCst);
    app.update_multi_roblox();

    run::execute(&app, request, out, false).await
}

fn open_accounts(app: &app::App, command: &str) -> Result<(), String> {
    let state = app.store.lock().unwrap().load();
    match state {
        LoadState::Ready => Ok(()),
        // a first import creates the file with the default (Windows) encryption
        LoadState::ChooseProtection if command == "import" => app.store.lock().unwrap().set_protection(None).map_err(|e| e.to_string()),
        LoadState::ChooseProtection => Ok(()),
        LoadState::NeedsPassword => {
            let password = password()?;
            app.store.lock().unwrap().unlock(&password).map_err(|e| e.to_string())
        }
        LoadState::OfferBackup { .. } => Err("No accounts were loaded but there is a backup file, open Roblox Account Manager to load it".into()),
        LoadState::Failed { message } => Err(message),
    }
}

fn password() -> Result<String, String> {
    if let Ok(p) = std::env::var("RAM_PASSWORD") {
        if !p.is_empty() {
            return Ok(p);
        }
    }
    if !std::io::stdin().is_terminal() {
        return Err("The accounts are password protected: set RAM_PASSWORD, or run ram-cli in a terminal".into());
    }

    eprint!("Password: ");
    let _ = std::io::stderr().flush();
    let echo = EchoOff::new();
    let mut line = String::new();
    let read = std::io::stdin().read_line(&mut line);
    drop(echo);
    eprintln!();
    read.map_err(|e| e.to_string())?;
    Ok(line.trim_end_matches(['\r', '\n']).to_string())
}

/// Hides what's typed (the password) until dropped
struct EchoOff(Option<(windows::Win32::Foundation::HANDLE, windows::Win32::System::Console::CONSOLE_MODE)>);

impl EchoOff {
    fn new() -> EchoOff {
        use windows::Win32::System::Console::*;
        unsafe {
            let Ok(input) = GetStdHandle(STD_INPUT_HANDLE) else { return EchoOff(None) };
            let mut mode = CONSOLE_MODE::default();
            if GetConsoleMode(input, &mut mode).is_err() {
                return EchoOff(None);
            }
            let _ = SetConsoleMode(input, mode & !ENABLE_ECHO_INPUT);
            EchoOff(Some((input, mode)))
        }
    }
}

impl Drop for EchoOff {
    fn drop(&mut self) {
        if let Some((input, mode)) = self.0 {
            unsafe {
                let _ = windows::Win32::System::Console::SetConsoleMode(input, mode);
            }
        }
    }
}
