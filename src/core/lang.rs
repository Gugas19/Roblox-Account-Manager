//! Languages: the JSON tables in lang/ (English text is the key). The interface translates what it shows,
//! `translate` does the same for what Rust sends elsewhere (Discord messages).

use once_cell::sync::Lazy;
use regex::{Captures, Regex};
use serde::Serialize;
use std::collections::HashMap;

/// Marks English text that gets translated later, so tools/strings.py finds it
#[macro_export]
macro_rules! text {
    ($s:literal) => {
        $s
    };
}

#[derive(Debug, Clone, Serialize)]
pub struct Language {
    pub code: &'static str,
    pub name: &'static str,
    pub flag: &'static str,
}

pub const LANGUAGES: &[Language] = &[
    Language { code: "en", name: "English", flag: "gb" },
    Language { code: "pt-PT", name: "Português (Portugal)", flag: "pt" },
    Language { code: "es", name: "Español", flag: "es" },
    Language { code: "fr", name: "Français", flag: "fr" },
    Language { code: "de", name: "Deutsch", flag: "de" },
];

const TABLES: &[(&str, &str)] = &[
    ("pt-PT", include_str!("../../lang/pt-PT.json")),
    ("es", include_str!("../../lang/es.json")),
    ("fr", include_str!("../../lang/fr.json")),
    ("de", include_str!("../../lang/de.json")),
];

/// The JSON table of a language ("{}" for English, the interface text is English already)
pub fn table(code: &str) -> &'static str {
    TABLES.iter().find(|(c, _)| *c == code).map(|(_, t)| *t).unwrap_or("{}")
}

static PLACEHOLDER: Lazy<Regex> = Lazy::new(|| Regex::new(r"\{(\d+)\}").unwrap());

/// Text built from a template ("{0} lost the connection"), recognized once filled in
struct Template {
    pattern: Regex,
    translation: String,
    slots: Vec<usize>,
}

struct Translator {
    table: HashMap<String, String>,
    templates: Vec<Template>,
}

impl Translator {
    fn new(json: &str) -> Translator {
        let table: HashMap<String, String> = serde_json::from_str(json).unwrap_or_default();
        let mut templates: Vec<Template> = table
            .iter()
            .filter(|(key, _)| PLACEHOLDER.is_match(key))
            .filter_map(|(key, translation)| {
                let (mut source, mut slots, mut last) = (String::from("^"), Vec::new(), 0);
                for c in PLACEHOLDER.captures_iter(key) {
                    let m = c.get(0)?;
                    source += &regex::escape(&key[last..m.start()]);
                    source += r"([\s\S]*?)";
                    slots.push(c[1].parse().ok()?);
                    last = m.end();
                }
                source += &regex::escape(&key[last..]);
                source += "$";
                Some(Template { pattern: Regex::new(&source).ok()?, translation: translation.clone(), slots })
            })
            .collect();
        // the most specific template first
        templates.sort_by_key(|t| std::cmp::Reverse(t.pattern.as_str().len()));
        Translator { table, templates }
    }

    /// Same rules as the interface's translator (ui/src/lib/i18n.tsx)
    fn translate(&self, text: &str, depth: u8) -> String {
        if text.is_empty() {
            return String::new();
        }
        if let Some(t) = self.table.get(text).filter(|t| !t.is_empty()) {
            return t.clone();
        }
        let trimmed = text.trim();
        if trimmed != text {
            if let Some(t) = self.table.get(trimmed) {
                return text.replace(trimmed, t);
            }
        }
        if depth < 2 {
            for t in &self.templates {
                if let Some(found) = t.pattern.captures(text) {
                    return PLACEHOLDER
                        .replace_all(&t.translation, |c: &Captures| {
                            let n: usize = c[1].parse().unwrap_or(usize::MAX);
                            t.slots.iter().position(|s| *s == n).map(|i| self.translate(&found[i + 1], depth + 1)).unwrap_or_default()
                        })
                        .into_owned();
                }
            }
        }
        text.to_string()
    }
}

static TRANSLATORS: Lazy<HashMap<&'static str, Translator>> = Lazy::new(|| TABLES.iter().map(|(code, json)| (*code, Translator::new(json))).collect());

/// English text in a language, also text built from a template and already filled in ("Rejoined NightFalcon")
pub fn translate(code: &str, text: &str) -> String {
    match TRANSLATORS.get(code) {
        Some(t) => t.translate(text, 0),
        None => text.to_string(),
    }
}

/// Saved language, otherwise the Windows language (any Portuguese is pt-PT), otherwise English
pub fn resolve(saved: &str) -> &'static str {
    let saved = if saved.starts_with("pt") { "pt-PT" } else { saved };
    if let Some(l) = LANGUAGES.iter().find(|l| l.code == saved) {
        return l.code;
    }
    detect()
}

fn detect() -> &'static str {
    let name = windows_language();
    if name.starts_with("pt") {
        return "pt-PT";
    }
    let two = name.split('-').next().unwrap_or("");
    LANGUAGES.iter().find(|l| l.code == two).map(|l| l.code).unwrap_or("en")
}

fn windows_language() -> String {
    use windows::Win32::Globalization::GetUserDefaultLocaleName;
    let mut buffer = [0u16; 85];
    let len = unsafe { GetUserDefaultLocaleName(&mut buffer) };
    if len <= 1 {
        return String::new();
    }
    String::from_utf16_lossy(&buffer[..(len - 1) as usize])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_a_table_with_the_same_keys() {
        let reference: serde_json::Map<String, serde_json::Value> = serde_json::from_str(table("pt-PT")).unwrap();
        for l in LANGUAGES.iter().filter(|l| l.code != "en") {
            let t: serde_json::Map<String, serde_json::Value> = serde_json::from_str(table(l.code)).unwrap();
            assert_eq!(t.len(), reference.len(), "{}", l.code);
        }
    }

    #[test]
    fn translates_like_the_interface() {
        assert_eq!(translate("en", "Anti-AFK started"), "Anti-AFK started");
        assert_eq!(translate("pt-PT", "Anti-AFK started"), "Anti-AFK iniciado");
        // a filled in template, with its parts translated too
        assert_eq!(translate("pt-PT", "Started from the tray · Roblox windows: 3"), "Iniciado a partir da área de notificação · Janelas do Roblox: 3");
        assert_eq!(translate("de", "Alt01 lost the connection"), "Alt01 hat die Verbindung verloren");
        // unknown text stays as it is
        assert_eq!(translate("fr", "Something nobody translated"), "Something nobody translated");
    }

    #[test]
    fn saved_languages() {
        assert_eq!(resolve("pt-BR"), "pt-PT");
        assert_eq!(resolve("de"), "de");
    }
}
