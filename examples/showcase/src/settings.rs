//! Kalıcı ayarlar: tema ve yazı ayarı.
//!
//! Ayarlar kullanıcının yapılandırma klasöründe düz bir metin dosyasında
//! tutulur (`$XDG_CONFIG_HOME/kentos-cad/ayarlar`, yoksa
//! `~/.config/kentos-cad/ayarlar`):
//!
//! ```text
//! # KentOS CAD ayarları
//! tema = koyu
//! yazi-ailesi = inter
//! es-aralikli = ibm-plex-mono
//! yazi-boyutu = 14
//! ```
//!
//! Bilinmeyen anahtarlar ve bozuk değerler yok sayılır; eksik ayar
//! varsayılanıyla kalır.

use std::path::{Path, PathBuf};
use std::{fs, io};

use kentos_rc::theme::Mode;
use kentos_rc::theme::typography::{Family, Mono, Typography};

/// Uygulamanın saklanan ayarları.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct Settings {
    pub mode: Mode,
    pub typography: Typography,
}

impl Settings {
    /// Ayar dosyasının yolu; ev klasörü bilinmiyorsa `None`.
    pub fn path() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_CONFIG_HOME")
            .filter(|value| !value.is_empty())
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|home| PathBuf::from(home).join(".config")))?;

        Some(base.join("kentos-cad").join("ayarlar"))
    }

    /// Dosyadaki ayarlar; dosya yoksa ya da okunamıyorsa varsayılanlar.
    pub fn load(path: &Path) -> Self {
        fs::read_to_string(path)
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    pub fn save(&self, path: &Path) -> io::Result<()> {
        if let Some(folder) = path.parent() {
            fs::create_dir_all(folder)?;
        }

        fs::write(path, self.render())
    }

    fn parse(text: &str) -> Self {
        let mut settings = Self::default();

        for line in text.lines() {
            let line = line.trim();

            if line.starts_with('#') {
                continue;
            }

            let Some((key, value)) = line.split_once('=') else {
                continue;
            };

            let value = value.trim();

            match key.trim() {
                "tema" => {
                    settings.mode = match value {
                        "aydinlik" => Mode::Light,
                        _ => Mode::Dark,
                    };
                }
                "yazi-ailesi" => {
                    if let Some(family) = Family::ALL
                        .into_iter()
                        .find(|family| key_of(family.name()) == value)
                    {
                        settings.typography.family = family;
                    }
                }
                "es-aralikli" => {
                    if let Some(mono) = Mono::ALL
                        .into_iter()
                        .find(|mono| key_of(mono.name()) == value)
                    {
                        settings.typography.mono = mono;
                    }
                }
                "yazi-boyutu" => {
                    if let Ok(size) = value.replace(',', ".").parse::<f32>() {
                        settings.typography = Typography {
                            size,
                            ..settings.typography
                        }
                        .clamped();
                    }
                }
                _ => {}
            }
        }

        settings
    }

    fn render(&self) -> String {
        let mode = match self.mode {
            Mode::Dark => "koyu",
            Mode::Light => "aydinlik",
        };

        format!(
            "# KentOS CAD ayarları\ntema = {mode}\nyazi-ailesi = {}\nes-aralikli = {}\nyazi-boyutu = {}\n",
            key_of(self.typography.family.name()),
            key_of(self.typography.mono.name()),
            self.typography.size,
        )
    }
}

/// Ailenin dosyadaki adı: küçük harf, boşluklar tire ("IBM Plex Sans" →
/// "ibm-plex-sans").
fn key_of(name: &str) -> String {
    name.to_ascii_lowercase().replace(' ', "-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn settings_round_trip() {
        let settings = Settings {
            mode: Mode::Light,
            typography: Typography {
                family: Family::PlusJakartaSans,
                mono: Mono::JetBrainsMono,
                size: 15.0,
            },
        };

        assert_eq!(Settings::parse(&settings.render()), settings);
    }

    #[test]
    fn broken_lines_keep_the_defaults() {
        let settings = Settings::parse(
            "# yorum\ntema = mor\nyazi-ailesi = comic-sans\nyazi-boyutu = 99\nbilinmeyen = 1\nbozuk satır",
        );

        assert_eq!(settings.mode, Mode::Dark);
        assert_eq!(settings.typography.family, Family::IbmPlexSans);
        assert_eq!(settings.typography.size, 18.0);
    }

    #[test]
    fn files_are_written_and_read_back() {
        let folder = std::env::temp_dir().join(format!("kentos-ayarlar-{}", std::process::id()));
        let path = folder.join("alt").join("ayarlar");
        let settings = Settings {
            mode: Mode::Dark,
            typography: Typography {
                family: Family::Inter,
                ..Typography::DEFAULT
            },
        };

        settings.save(&path).expect("ayar dosyası yazılamadı");
        assert_eq!(Settings::load(&path), settings);
        assert_eq!(Settings::load(&folder.join("yok")), Settings::default());

        let _ = fs::remove_dir_all(folder);
    }
}
