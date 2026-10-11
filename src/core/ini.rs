//! RAMSettings.ini / RAMTheme.ini, read and written exactly like the C# version (IniFile.cs) so both versions share them.

use indexmap::IndexMap;
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct Property {
    pub value: String,
    pub comment: Option<String>,
}

#[derive(Debug, Clone, Default)]
pub struct Section {
    pub properties: IndexMap<String, Property>,
}

impl Section {
    pub fn get(&self, name: &str) -> Option<&str> {
        self.properties.get(name).map(|p| p.value.as_str())
    }

    pub fn exists(&self, name: &str) -> bool {
        self.properties.contains_key(name)
    }

    /// Same as C# `Get<bool>`: missing or invalid is false
    pub fn get_bool(&self, name: &str) -> bool {
        self.get(name).map(|v| v.trim().eq_ignore_ascii_case("true")).unwrap_or(false)
    }

    pub fn get_i64(&self, name: &str, default: i64) -> i64 {
        self.get(name).and_then(|v| v.trim().parse::<f64>().ok()).map(|v| v as i64).unwrap_or(default)
    }

    pub fn get_f64(&self, name: &str, default: f64) -> f64 {
        self.get(name).and_then(|v| v.trim().parse::<f64>().ok()).unwrap_or(default)
    }

    /// An empty value removes the property, like the C# version
    pub fn set(&mut self, name: &str, value: &str) {
        self.set_with_comment(name, value, None)
    }

    pub fn set_with_comment(&mut self, name: &str, value: &str, comment: Option<&str>) {
        if value.trim().is_empty() {
            self.properties.shift_remove(name);
            return;
        }

        match self.properties.get_mut(name) {
            Some(p) => {
                p.value = value.to_string();
                if let Some(c) = comment {
                    p.comment = Some(c.to_string());
                }
            }
            None => {
                self.properties.insert(name.to_string(), Property { value: value.to_string(), comment: comment.map(str::to_string) });
            }
        }
    }

    pub fn set_bool(&mut self, name: &str, value: bool) {
        self.set(name, if value { "true" } else { "false" })
    }

    pub fn remove(&mut self, name: &str) {
        self.properties.shift_remove(name);
    }

    pub fn set_default(&mut self, name: &str, value: &str) {
        if !self.exists(name) {
            self.set(name, value)
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct IniFile {
    pub sections: IndexMap<String, Section>,
}

impl IniFile {
    pub fn parse(text: &str) -> IniFile {
        let mut ini = IniFile::default();
        let mut current: Option<String> = None;

        for raw in text.lines() {
            let line = raw.trim();

            if line.is_empty() || line.starts_with(';') || line.starts_with('#') {
                continue;
            }

            if line.starts_with('[') && line.ends_with(']') && line.len() >= 2 {
                let mut name = line[1..line.len() - 1].to_string();
                if name == "RBX Alt Manager" {
                    name = "Roblox Account Manager".into(); // old themes
                }
                ini.sections.entry(name.clone()).or_default();
                current = Some(name);
                continue;
            }

            if let Some(section) = current.as_ref().and_then(|s| ini.sections.get_mut(s)) {
                // C#: Split("=", 2, RemoveEmptyEntries), so "=x" and "x=" are skipped
                let parts: Vec<&str> = line.splitn(2, '=').filter(|p| !p.is_empty()).collect();
                if parts.len() != 2 {
                    continue;
                }
                section.set(parts[0].trim(), parts[1].trim());
            }
        }

        ini
    }

    pub fn load(path: &Path) -> IniFile {
        match std::fs::read(path) {
            Ok(bytes) => IniFile::parse(&String::from_utf8_lossy(&bytes).trim_start_matches('\u{feff}')),
            Err(_) => IniFile::default(),
        }
    }

    pub fn section(&self, name: &str) -> Option<&Section> {
        self.sections.get(name)
    }

    pub fn section_mut(&mut self, name: &str) -> &mut Section {
        self.sections.entry(name.to_string()).or_default()
    }

    pub fn get(&self, section: &str, name: &str) -> Option<&str> {
        self.section(section).and_then(|s| s.get(name))
    }

    pub fn get_bool(&self, section: &str, name: &str) -> bool {
        self.section(section).map(|s| s.get_bool(name)).unwrap_or(false)
    }

    pub fn to_text(&self) -> String {
        let mut out = String::new();

        for (name, section) in &self.sections {
            if section.properties.is_empty() {
                continue;
            }

            out.push_str(&format!("[{}]\r\n", name));

            for (key, p) in &section.properties {
                if let Some(c) = &p.comment {
                    out.push_str(&format!("# {}\r\n", c));
                }
                out.push_str(&format!("{}={}\r\n", key, p.value));
            }

            out.push_str("\r\n");
        }

        out
    }

    pub fn save(&self, path: &Path) -> std::io::Result<()> {
        // write next to it first so a crash can't leave a half written settings file
        let temp = path.with_extension("ini.tmp");
        std::fs::write(&temp, self.to_text())?;
        std::fs::rename(&temp, path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_like_the_csharp_version() {
        let ini = IniFile::parse("[General]\nLanguage=pt-PT\n# comment\n; other\nEnableMultiRbx=true\n=x\nbad\n\n[WebServer]\nPassword=abc=def\n[RBX Alt Manager]\nA=1");
        assert_eq!(ini.get("General", "Language"), Some("pt-PT"));
        assert!(ini.get_bool("General", "EnableMultiRbx"));
        assert_eq!(ini.get("WebServer", "Password"), Some("abc=def"));
        assert_eq!(ini.get("Roblox Account Manager", "A"), Some("1"));
        assert_eq!(ini.section("General").unwrap().properties.len(), 2);
    }

    #[test]
    fn writes_back_in_order_and_empty_values_are_removed() {
        let mut ini = IniFile::parse("[General]\nB=2\nA=1\n");
        ini.section_mut("General").set_with_comment("C", "3", Some("hello"));
        ini.section_mut("General").set("B", "");
        ini.section_mut("Empty");
        assert_eq!(ini.to_text(), "[General]\r\nA=1\r\n# hello\r\nC=3\r\n\r\n");
    }

    #[test]
    fn numbers() {
        let ini = IniFile::parse("[G]\nA=8\nB=1.5\nC=x");
        let s = ini.section("G").unwrap();
        assert_eq!(s.get_i64("A", 0), 8);
        assert_eq!(s.get_f64("B", 0.0), 1.5);
        assert_eq!(s.get_i64("C", 7), 7);
        assert_eq!(s.get_i64("D", 9), 9);
    }
}
