//! Session history: what each account did, kept in `SessionHistory.json` next to the app (the newest 5000 events).
//!
//! Two sources: the Roblox windows on this PC (from their logs, exact: joined a game or another server, lost the
//! connection, rejoined, closed) and Roblox's presence for every account (sampled at the presence refresh, so a short
//! session between two checks can be missed: online, offline, in a game, in Studio). The first presence answer for an
//! account is only a starting point, not an event. Off with Settings > General > Keep a session history.

use crate::app::App;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// The most events kept, the oldest go first
const MAX_EVENTS: usize = 5000;
const FILE: &str = "SessionHistory.json";

static LOCK: Mutex<()> = Mutex::new(());

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Event {
    /// seconds since 1970
    pub at: u64,
    pub account: String,
    /// joined, server, disconnected, rejoined, closed (this PC); online, offline, playing, studio (presence)
    pub kind: String,
    #[serde(default)]
    pub place_id: Option<i64>,
    /// the game's name when known
    #[serde(default)]
    pub place: Option<String>,
    #[serde(default)]
    pub job_id: Option<String>,
}

impl Event {
    pub fn new(account: &str, kind: &str) -> Event {
        Event { at: now(), account: account.to_string(), kind: kind.to_string(), place_id: None, place: None, job_id: None }
    }

    pub fn place(mut self, place_id: Option<i64>, job_id: Option<String>) -> Event {
        self.place_id = place_id;
        self.job_id = job_id;
        self
    }
}

fn now() -> u64 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

fn file(app: &App) -> PathBuf {
    app.dir.join(FILE)
}

fn read(path: &PathBuf) -> Vec<Event> {
    std::fs::read_to_string(path).ok().and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default()
}

fn write(path: &PathBuf, events: &[Event]) {
    let Ok(json) = serde_json::to_string(events) else { return };
    let temp = path.with_extension("json.tmp");
    if std::fs::write(&temp, json).is_ok() {
        let _ = std::fs::rename(&temp, path);
    }
}

/// Adds the events, newest last, keeping the newest MAX_EVENTS
fn append(mut list: Vec<Event>, events: Vec<Event>) -> Vec<Event> {
    list.extend(events);
    if list.len() > MAX_EVENTS {
        list.drain(..list.len() - MAX_EVENTS);
    }
    list
}

pub fn enabled(app: &App) -> bool {
    app.setting("General", "KeepHistory") != "false"
}

pub fn record(app: &App, events: Vec<Event>) {
    if events.is_empty() || !enabled(app) {
        return;
    }
    let _guard = LOCK.lock().unwrap();
    let path = file(app);
    write(&path, &append(read(&path), events));
    app.emit("history", ());
}

/// Newest first
pub fn list(app: &App) -> Vec<Event> {
    let _guard = LOCK.lock().unwrap();
    let mut list = read(&file(app));
    list.reverse();
    list
}

pub fn clear(app: &App) {
    let _guard = LOCK.lock().unwrap();
    let _ = std::fs::remove_file(file(app));
}

/// What a presence change means for the history: nothing for the first answer or no change
pub fn presence_events(account: &str, before: Option<&crate::roblox::Presence>, after: &crate::roblox::Presence) -> Option<Event> {
    let before = before?;
    let kind = match after.kind {
        0 => "offline",
        1 => "online",
        2 => "playing",
        3 => "studio",
        _ => return None,
    };
    let moved = after.kind == 2 && before.kind == 2 && after.place_id.is_some() && (after.place_id != before.place_id || after.game_id != before.game_id);
    if before.kind == after.kind && !moved {
        return None;
    }
    let mut event = Event::new(account, kind).place(after.place_id.filter(|_| after.kind >= 2), after.game_id.clone().filter(|_| after.kind == 2));
    event.place = after.last_location.clone().filter(|l| after.kind >= 2 && !l.trim().is_empty());
    Some(event)
}

/// The events as CSV (a spreadsheet opens it), times in UTC
pub fn csv(events: &[Event]) -> String {
    let field = |s: &str| if s.contains([',', '"', '\n']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s.to_string() };
    let mut out = String::from("time_utc,account,event,place_id,game,job_id\n");
    for e in events {
        let time = chrono::DateTime::from_timestamp(e.at as i64, 0).map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()).unwrap_or_default();
        out += &[
            time,
            field(&e.account),
            e.kind.clone(),
            e.place_id.map(|p| p.to_string()).unwrap_or_default(),
            field(e.place.as_deref().unwrap_or("")),
            e.job_id.clone().unwrap_or_default(),
        ]
        .join(",");
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::roblox::Presence;

    fn presence(kind: i64, place: Option<i64>, game: Option<&str>) -> Presence {
        Presence { kind, last_location: Some("Some Game".into()), place_id: place, game_id: game.map(str::to_string), user_id: 1 }
    }

    #[test]
    fn presence_changes_become_events() {
        // the first answer is only where it starts
        assert_eq!(presence_events("A", None, &presence(2, Some(5), Some("j"))), None);
        assert_eq!(presence_events("A", Some(&presence(1, None, None)), &presence(1, None, None)), None);
        let joined = presence_events("A", Some(&presence(1, None, None)), &presence(2, Some(5), Some("j"))).unwrap();
        assert_eq!((joined.kind.as_str(), joined.place_id, joined.job_id.as_deref(), joined.place.as_deref()), ("playing", Some(5), Some("j"), Some("Some Game")));
        // another server of the same game is an event too, the same one isn't
        assert!(presence_events("A", Some(&presence(2, Some(5), Some("j"))), &presence(2, Some(5), Some("k"))).is_some());
        assert!(presence_events("A", Some(&presence(2, Some(5), Some("j"))), &presence(2, Some(5), Some("j"))).is_none());
        let off = presence_events("A", Some(&presence(2, Some(5), Some("j"))), &presence(0, None, None)).unwrap();
        assert_eq!((off.kind.as_str(), off.place.as_deref()), ("offline", None));
    }

    #[test]
    fn keeps_the_newest_events() {
        let many: Vec<Event> = (0..MAX_EVENTS + 10).map(|i| Event::new(&i.to_string(), "online")).collect();
        let kept = append(Vec::new(), many);
        assert_eq!(kept.len(), MAX_EVENTS);
        assert_eq!(kept[0].account, "10");
    }

    #[test]
    fn csv_quotes_what_needs_it() {
        let mut e = Event::new("Main", "playing").place(Some(5), Some("job".into()));
        e.at = 0;
        e.place = Some("A, \"B\"".into());
        assert_eq!(csv(&[e]), "time_utc,account,event,place_id,game,job_id\n1970-01-01 00:00:00,Main,playing,5,\"A, \"\"B\"\"\",job\n");
    }
}
