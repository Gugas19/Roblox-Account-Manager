//! The open app listens on a named pipe, ram-cli sends it a command and prints the answer.
//!
//! One JSON line each way per message. Only processes of the same Windows user take part:
//! ram-cli checks who serves the pipe before sending anything (cookies for `import`).

use super::{run, Out, Reply, Request};
use crate::app::App;
use anyhow::{bail, Result};
use std::os::windows::io::AsRawHandle;
use std::sync::Arc;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::windows::named_pipe::{ClientOptions, NamedPipeServer, ServerOptions};
use windows::Win32::Foundation::{CloseHandle, HANDLE};
use windows::Win32::Security::{GetLengthSid, GetTokenInformation, TokenUser, TOKEN_QUERY, TOKEN_USER};
use windows::Win32::System::Pipes::GetNamedPipeServerProcessId;
use windows::Win32::System::Threading::{GetCurrentProcess, OpenProcess, OpenProcessToken, PROCESS_QUERY_LIMITED_INFORMATION};

const PIPE: &str = r"\\.\pipe\RobloxAccountManager.cli";
const ERROR_PIPE_BUSY: i32 = 231;

/// In the app: answers ram-cli for as long as the app runs
pub fn serve(app: Arc<App>) {
    tauri::async_runtime::spawn(async move {
        let mut first = true;
        loop {
            let server = match ServerOptions::new().first_pipe_instance(first).reject_remote_clients(true).create(PIPE) {
                Ok(s) => s,
                Err(e) => {
                    log::error!("Command line unavailable: {e}");
                    return;
                }
            };
            first = false;
            if server.connect().await.is_err() {
                continue;
            }
            let app = app.clone();
            tauri::async_runtime::spawn(async move {
                if let Err(e) = answer(app, server).await {
                    log::warn!("Command line: {e:#}");
                }
            });
        }
    });
}

async fn answer(app: Arc<App>, pipe: NamedPipeServer) -> Result<()> {
    let (read, mut write) = tokio::io::split(pipe);
    let mut line = String::new();
    tokio::time::timeout(Duration::from_secs(10), BufReader::new(read).read_line(&mut line)).await??;
    let request: Request = serde_json::from_str(&line)?;
    if request.args.is_empty() {
        bail!("empty command");
    }
    log::info!("Command line: {}", request.args[0]);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    let out = Out::new(move |reply| {
        let _ = tx.send(reply);
    });
    tauri::async_runtime::spawn(async move {
        let code = run::execute(&app, request, &out, true).await;
        out.send(Reply::Exit(code));
    });

    while let Some(reply) = rx.recv().await {
        let exit = matches!(reply, Reply::Exit(_));
        write.write_all(format!("{}\n", serde_json::to_string(&reply)?).as_bytes()).await?;
        if exit {
            break;
        }
    }
    write.flush().await?;
    Ok(())
}

/// In ram-cli: runs the command in the open app. `None` when the app isn't open.
pub async fn send(request: &Request, out: &Out) -> Result<Option<i32>> {
    let mut tries = 0;
    let client = loop {
        match ClientOptions::new().open(PIPE) {
            Ok(c) => break c,
            Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) && tries < 100 => {
                tries += 1;
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
            Err(_) => return Ok(None),
        }
    };
    if !same_user(HANDLE(client.as_raw_handle())) {
        bail!("The command line pipe belongs to another Windows user, not using it");
    }

    let (read, mut write) = tokio::io::split(client);
    write.write_all(format!("{}\n", serde_json::to_string(request)?).as_bytes()).await?;

    let mut lines = BufReader::new(read).lines();
    while let Some(line) = lines.next_line().await? {
        match serde_json::from_str::<Reply>(&line)? {
            Reply::Exit(code) => return Ok(Some(code)),
            reply => out.send(reply),
        }
    }
    bail!("Roblox Account Manager closed before answering")
}

/// Whether the process serving the pipe runs as the same Windows user as this one
fn same_user(pipe: HANDLE) -> bool {
    unsafe {
        let mut pid = 0u32;
        if GetNamedPipeServerProcessId(pipe, &mut pid).is_err() {
            return false;
        }
        let Ok(process) = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, false, pid) else { return false };
        let theirs = user_sid(process);
        let _ = CloseHandle(process);
        match (theirs, user_sid(GetCurrentProcess())) {
            (Some(theirs), Some(ours)) => theirs == ours,
            _ => false,
        }
    }
}

unsafe fn user_sid(process: HANDLE) -> Option<Vec<u8>> {
    let mut token = HANDLE::default();
    OpenProcessToken(process, TOKEN_QUERY, &mut token).ok()?;
    let mut size = 0u32;
    let _ = GetTokenInformation(token, TokenUser, None, 0, &mut size);
    let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
    let read = GetTokenInformation(token, TokenUser, Some(buffer.as_mut_ptr().cast()), size, &mut size);
    let _ = CloseHandle(token);
    read.ok()?;

    let sid = (*(buffer.as_ptr() as *const TOKEN_USER)).User.Sid;
    let length = GetLengthSid(sid) as usize;
    Some(std::slice::from_raw_parts(sid.0 as *const u8, length).to_vec())
}
