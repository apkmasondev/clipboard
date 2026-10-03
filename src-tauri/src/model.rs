use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

pub type Result<T> = std::result::Result<T, String>;
pub const MAX_TEXT: usize = 4 * 1024 * 1024;
pub const MAX_IMAGE: usize = 24 * 1024 * 1024;
pub const KINDS: &[&str] = &[
    "text", "rich", "link", "path", "code", "json", "color", "image", "files",
];
pub fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[derive(Debug, Clone, Serialize, Deserialize, Default, PartialEq)]
#[serde(default, rename_all = "camelCase")]
pub struct Content {
    pub title: String,
    pub text: String,
    pub description: String,
    pub category: String,
    pub tags: Vec<String>,
    pub shortcut: String,
    pub is_template: bool,
    pub source: String,
    pub html: Vec<u8>,
    pub rtf: Vec<u8>,
    pub files: Vec<String>,
    pub width: u32,
    pub height: u32,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub id: String,
    pub kind: String,
    pub created: i64,
    pub updated: i64,
    pub bytes: i64,
    pub pinned: bool,
    pub library: bool,
    pub content: Content,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Summary {
    pub id: String,
    pub kind: String,
    pub created: i64,
    pub updated: i64,
    pub bytes: i64,
    pub pinned: bool,
    pub library: bool,
    pub title: String,
    pub preview: String,
    pub category: String,
    pub tags: Vec<String>,
    pub source: String,
    pub shortcut: String,
    pub is_template: bool,
    pub width: u32,
    pub height: u32,
}
impl From<Entry> for Summary {
    fn from(e: Entry) -> Self {
        Self {
            id: e.id,
            kind: e.kind,
            created: e.created,
            updated: e.updated,
            bytes: e.bytes,
            pinned: e.pinned,
            library: e.library,
            title: e.content.title,
            preview: e.content.text.chars().take(240).collect(),
            category: e.content.category,
            tags: e.content.tags,
            source: e.content.source,
            shortcut: e.content.shortcut,
            is_template: e.content.is_template,
            width: e.content.width,
            height: e.content.height,
        }
    }
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Settings {
    pub theme: String,
    pub max_items: u32,
    pub max_days: u32,
    pub max_mb: u32,
    pub blocked_apps: Vec<String>,
    pub disabled_types: Vec<String>,
    pub skip_secrets: bool,
    pub block_unknown: bool,
    pub paused: bool,
    pub autostart: bool,
    pub quick_shortcut: String,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: "system".into(),
            max_items: 2000,
            max_days: 30,
            max_mb: 100,
            blocked_apps: vec![
                "1password.exe",
                "bitwarden.exe",
                "keepass.exe",
                "keepassxc.exe",
                "dashlane.exe",
                "protonpass.exe",
                "enpass.exe",
                "lastpass.exe",
            ]
            .into_iter()
            .map(String::from)
            .collect(),
            disabled_types: vec![],
            skip_secrets: true,
            block_unknown: false,
            paused: false,
            autostart: false,
            quick_shortcut: "Control+Shift+KeyV".into(),
        }
    }
}
impl Settings {
    pub fn validate(&self) -> Result<()> {
        if !(10..=20000).contains(&self.max_items)
            || !(1..=3650).contains(&self.max_days)
            || !(10..=2048).contains(&self.max_mb)
        {
            return Err("Limity: 10–20000 elementów, 1–3650 dni, 10–2048 MB.".into());
        }
        if !["light", "dark", "system"].contains(&self.theme.as_str()) {
            return Err("Nieznany motyw.".into());
        }
        if self
            .disabled_types
            .iter()
            .any(|t| !KINDS.contains(&t.as_str()))
            || self.blocked_apps.len() > 200
            || self.blocked_apps.iter().any(|a| {
                a.len() > 128 || a.contains(['/', '\\']) || !a.to_lowercase().ends_with(".exe")
            })
        {
            return Err("Wykluczenia muszą być nazwami procesów .exe (bez ścieżki).".into());
        }
        Ok(())
    }
}
pub fn validate_content(c: &Content) -> Result<()> {
    if c.text.len() > MAX_TEXT
        || c.title.len() > 300
        || c.description.len() > 4000
        || c.category.len() > 100
        || c.tags.len() > 20
        || c.tags.iter().any(|s| s.len() > 60)
        || c.shortcut.len() > 100
    {
        return Err("Przekroczono limit treści lub metadanych.".into());
    }
    if c.is_template {
        crate::templates::fields(&c.text)?;
    }
    Ok(())
}
pub fn classify(text: &str, rich: bool) -> &'static str {
    let t = text.trim();
    if (t.starts_with('{') || t.starts_with('['))
        && serde_json::from_str::<serde_json::Value>(t).is_ok()
    {
        return "json";
    }
    if !t.contains(char::is_whitespace) && (t.starts_with("https://") || t.starts_with("http://")) {
        return "link";
    }
    if (t.starts_with('#')
        && [4, 5, 7, 9].contains(&t.len())
        && t[1..].chars().all(|c| c.is_ascii_hexdigit()))
        || ((t.starts_with("rgb(") || t.starts_with("rgba(")) && t.ends_with(')'))
    {
        return "color";
    }
    if t.starts_with("\\\\")
        || (t.as_bytes().get(1) == Some(&b':') && t.as_bytes().get(2) == Some(&b'\\'))
    {
        return "path";
    }
    if [
        "function ",
        "const ",
        "let ",
        "fn ",
        "def ",
        "class ",
        "import ",
        "SELECT ",
        "#!/",
        "$ ",
        "npm ",
        "git ",
        "cargo ",
    ]
    .iter()
    .any(|p| t.starts_with(p))
        || (t.contains('\n') && t.contains('{') && t.contains(';'))
    {
        return "code";
    }
    if rich {
        "rich"
    } else {
        "text"
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn formats() {
        for (s, k) in [
            ("{\"ok\":true}", "json"),
            ("{bad}", "text"),
            ("https://example.com/a", "link"),
            ("#fF00aa", "color"),
            ("C:\\work", "path"),
            ("fn main() {}", "code"),
            ("zażółć gęślą", "text"),
        ] {
            assert_eq!(classify(s, false), k);
        }
    }
    #[test]
    fn settings_limits() {
        let mut s = Settings::default();
        assert!(s.validate().is_ok());
        s.max_items = 0;
        assert!(s.validate().is_err());
    }
}
