//! Languages: the JSON tables in lang/ (English text is the key), translated in the interface.

use serde::Serialize;

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
    fn saved_languages() {
        assert_eq!(resolve("pt-BR"), "pt-PT");
        assert_eq!(resolve("de"), "de");
    }
}
