//! Disconnect detection from the Roblox log: which game and server a window is in, and when it lost the connection.
//! Rejoining relaunches the account into the same game (and server) through the app's launcher, instead of the original
//! AntiAFK-RBX approach of checking a pixel and clicking the "Reconnect" button.

use regex::Regex;
use std::collections::HashSet;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct LogState {
    pub place_id: Option<i64>,
    pub job_id: Option<String>,
    pub disconnected: Option<i64>,
}

pub struct LogReader {
    path: PathBuf,
    position: u64,
    pub state: LogState,
}

fn joining() -> Regex {
    Regex::new(r"! Joining game '([0-9a-fA-F\-]{36})' place (\d+) at").unwrap()
}

fn disconnect() -> Regex {
    Regex::new(r"Sending disconnect with reason: (\d+)").unwrap()
}

/// Reads one chunk of log text into the state (last join wins, a later join clears a disconnect)
pub fn apply(state: &mut LogState, text: &str) {
    let (join, leave) = (joining(), disconnect());
    for line in text.lines() {
        if let Some(c) = join.captures(line) {
            state.job_id = Some(c[1].to_string());
            state.place_id = c[2].parse().ok();
            state.disconnected = None;
        } else if let Some(c) = leave.captures(line) {
            state.disconnected = c[1].parse().ok();
        }
    }
}

fn logs_folder() -> PathBuf {
    std::env::var("LOCALAPPDATA").map(PathBuf::from).unwrap_or_default().join("Roblox").join("logs")
}

/// The Roblox player log created right after a process started
pub fn find_log(seconds_running: u64, taken: &HashSet<PathBuf>) -> Option<PathBuf> {
    let started = SystemTime::now() - Duration::from_secs(seconds_running);
    let mut best: Option<(Duration, PathBuf)> = None;

    for entry in std::fs::read_dir(logs_folder()).ok()?.filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.file_name().map(|n| n.to_string_lossy().to_string()).unwrap_or_default();
        if !name.contains("_Player_") || !name.ends_with("_last.log") || taken.contains(&path) {
            continue;
        }
        let Ok(created) = entry.metadata().and_then(|m| m.created()) else { continue };
        let Ok(after) = created.duration_since(started - Duration::from_secs(5)) else { continue };
        if after > Duration::from_secs(120) {
            continue;
        }
        if best.as_ref().map(|(d, _)| after < *d).unwrap_or(true) {
            best = Some((after, path));
        }
    }
    best.map(|(_, p)| p)
}

impl LogReader {
    pub fn new(path: PathBuf) -> LogReader {
        LogReader { path, position: 0, state: LogState::default() }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }

    /// Reads what was added since last time
    pub fn update(&mut self) {
        let Ok(mut file) = std::fs::File::open(&self.path) else { return };
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if len <= self.position {
            return;
        }
        let _ = file.seek(SeekFrom::Start(self.position));
        let mut bytes = Vec::new();
        let _ = file.take(len - self.position).read_to_end(&mut bytes);
        self.position = len;
        apply(&mut self.state, &String::from_utf8_lossy(&bytes));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const LOG: &str = "2026-10-11T20:00:01.123Z,1.2,abc,6 [FLog::Output] ! Joining game '0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0' place 1818 at 128.116.1.2\n\
                       2026-10-11T20:20:01.123Z,1.2,abc,6 [FLog::Network] Sending disconnect with reason: 277\n";

    #[test]
    fn reads_join_and_disconnect() {
        let mut s = LogState::default();
        apply(&mut s, LOG);
        assert_eq!(s.place_id, Some(1818));
        assert_eq!(s.job_id.as_deref(), Some("0f1e2d3c-4b5a-6978-8796-a5b4c3d2e1f0"));
        assert_eq!(s.disconnected, Some(277));

        apply(&mut s, "x [FLog::Output] ! Joining game '11111111-2222-3333-4444-555555555555' place 42 at 1.1.1.1");
        assert_eq!(s.disconnected, None);
        assert_eq!(s.place_id, Some(42));
    }
}
