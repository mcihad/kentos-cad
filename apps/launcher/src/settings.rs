//! The launcher's settings (Ayarlar, docs/adr/0800 §5): the theme, the text
//! size and whether the launcher steps aside when an app starts. Kept in
//! `$XDG_CONFIG_HOME/kentos-launcher/ayarlar.json`, written whole and renamed
//! into place. A file that cannot be read, or a value that is not one the
//! launcher knows, means that value's default: never a reason to stop.

use std::path::{Path, PathBuf};

use kentos_ui::theme::typography::Typography;
use serde_json::{Value, json};

use crate::files;

const FORMAT: &str = "kentos.launcher-settings";
const FILE: &str = "ayarlar.json";

/// The theme chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ThemeChoice {
    /// The system's light or dark, followed as it changes.
    #[default]
    System,
    Light,
    Dark,
}

impl ThemeChoice {
    pub const ALL: [ThemeChoice; 3] = [ThemeChoice::System, ThemeChoice::Light, ThemeChoice::Dark];

    const fn key(self) -> &'static str {
        match self {
            ThemeChoice::System => "system",
            ThemeChoice::Light => "light",
            ThemeChoice::Dark => "dark",
        }
    }

    fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|t| t.key() == key)
    }
}

impl std::fmt::Display for ThemeChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            ThemeChoice::System => "Sistem",
            ThemeChoice::Light => "Açık",
            ThemeChoice::Dark => "Koyu",
        })
    }
}

/// The settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Settings {
    pub theme: ThemeChoice,
    /// The body text's size in pixels, KentOS UI's range (11–18).
    pub text_size: f32,
    /// The launcher minimises itself once an app has started.
    pub minimize_on_launch: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::System,
            text_size: Typography::DEFAULT.size,
            minimize_on_launch: false,
        }
    }
}

impl Settings {
    /// `…/kentos-launcher/ayarlar.json`.
    pub fn default_path() -> Option<PathBuf> {
        files::launcher_config().map(|c| c.join(FILE))
    }

    /// The settings in `path`; each value its default where it cannot be read.
    pub fn load(path: &Path) -> Self {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|text| serde_json::from_str::<Value>(&text).ok())
            .map(|value| Self::from_value(&value))
            .unwrap_or_default()
    }

    fn from_value(value: &Value) -> Self {
        let mut settings = Self::default();
        if value.get("format").and_then(Value::as_str) != Some(FORMAT) {
            return settings;
        }
        if let Some(theme) = value
            .get("theme")
            .and_then(Value::as_str)
            .and_then(ThemeChoice::parse)
        {
            settings.theme = theme;
        }
        if let Some(size) = value.get("textSize").and_then(Value::as_f64) {
            let size = size.round() as f32;
            if Typography::SIZES.contains(&size) {
                settings.text_size = size;
            }
        }
        if let Some(minimize) = value.get("minimizeOnLaunch").and_then(Value::as_bool) {
            settings.minimize_on_launch = minimize;
        }
        settings
    }

    /// The file's text.
    pub fn to_json(self) -> String {
        let mut text = serde_json::to_string_pretty(&json!({
            "format": FORMAT,
            "version": 1,
            "theme": self.theme.key(),
            "textSize": self.text_size,
            "minimizeOnLaunch": self.minimize_on_launch,
        }))
        .unwrap_or_default();
        text.push('\n');
        text
    }

    /// Writes the settings whole.
    pub fn save(self, path: &Path) -> std::io::Result<()> {
        files::write_whole(path, &self.to_json())
    }

    /// The text size's name where it is one of the web's steps (DESIGN.md §4.1).
    pub fn size_name(size: f32) -> Option<&'static str> {
        match size as u32 {
            12 => Some("Küçük"),
            13 => Some("Standart"),
            14 => Some("Büyük"),
            15 => Some("Çok büyük"),
            16 => Some("En büyük"),
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_go_round_and_survive_a_broken_file() {
        let dir = files::scratch("settings");
        let path = dir.join(FILE);
        assert_eq!(
            Settings::load(&path),
            Settings::default(),
            "no file: defaults"
        );
        let chosen = Settings {
            theme: ThemeChoice::Light,
            text_size: 18.0,
            minimize_on_launch: true,
        };
        chosen.save(&path).expect("saves");
        assert_eq!(Settings::load(&path), chosen);

        std::fs::write(&path, "{bozuk").expect("written");
        assert_eq!(
            Settings::load(&path),
            Settings::default(),
            "broken: defaults"
        );

        std::fs::write(
            &path,
            r#"{"format": "kentos.launcher-settings", "version": 1, "theme": "mor",
                "textSize": 40, "minimizeOnLaunch": true}"#,
        )
        .expect("written");
        let read = Settings::load(&path);
        assert_eq!(
            read.theme,
            ThemeChoice::System,
            "an unknown theme: its default"
        );
        assert_eq!(
            read.text_size, 13.0,
            "out of KentOS UI's range: its default"
        );
        assert!(read.minimize_on_launch, "the value that reads is kept");

        std::fs::write(&path, r#"{"theme": "dark"}"#).expect("written");
        assert_eq!(
            Settings::load(&path),
            Settings::default(),
            "another format: defaults"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn sizes_are_named() {
        assert_eq!(Settings::size_name(13.0), Some("Standart"));
        assert_eq!(Settings::size_name(18.0), None);
        assert_eq!(ThemeChoice::Dark.to_string(), "Koyu");
    }
}
