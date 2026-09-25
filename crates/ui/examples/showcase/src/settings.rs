//! Kalıcı ayarlar: tema, vurgu rengi, harita zemini, yazı ve yuva.
//!
//! Ayarlar kullanıcının yapılandırma klasöründe düz bir metin dosyasında
//! tutulur (`$XDG_CONFIG_HOME/kentos-cad/ayarlar`, yoksa
//! `~/.config/kentos-cad/ayarlar`):
//!
//! ```text
//! # KentOS CAD ayarları
//! tema = gece
//! vurgu = turuncu
//! harita-zemini = siyah
//! yazi-ailesi = inter
//! es-aralikli = ibm-plex-mono
//! yazi-boyutu = 14
//! yuva = sol 260: ; sag 360: katmanlar* @0.45 / ozellikler* @0.55; alt 252: tablo* gorevler @1.00
//! ```
//!
//! Bilinmeyen anahtarlar ve bozuk değerler yok sayılır; eksik ayar
//! varsayılanıyla kalır.

use std::path::{Path, PathBuf};
use std::{fs, io};

use kentos_rc::spatial::model_space::Backdrop;
use kentos_rc::theme::typography::{Family, Mono, Typography};
use kentos_rc::theme::{Accent, Mode};
use kentos_rc::widget::Docks;

use crate::message::DockPanel;

/// Uygulamanın saklanan ayarları.
#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    pub mode: Mode,
    /// Vurgu rengi: hazır renk ya da #RRGGBB.
    pub accent: Accent,
    /// Harita zemini; varsayılanı temaya uyar.
    pub backdrop: Backdrop,
    pub typography: Typography,
    /// Yuvadaki panellerin yerleşimi.
    pub docks: Docks<DockPanel>,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            mode: Mode::default(),
            accent: Accent::default(),
            backdrop: Backdrop::default(),
            typography: Typography::default(),
            docks: DockPanel::layout(),
        }
    }
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
                    settings.mode = Mode::ALL
                        .into_iter()
                        .find(|mode| mode_key(*mode) == value)
                        .unwrap_or_default();
                }
                "vurgu" => {
                    if let Some(accent) = Accent::parse(value) {
                        settings.accent = accent;
                    }
                }
                "harita-zemini" => {
                    if let Some(backdrop) = Backdrop::parse(value) {
                        settings.backdrop = backdrop;
                    }
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
                "yuva" => {
                    if let Some(docks) = Docks::load(value, DockPanel::parse) {
                        settings.docks = docks;
                    }
                }
                _ => {}
            }
        }

        settings
    }

    fn render(&self) -> String {
        let mode = mode_key(self.mode);

        format!(
            "# KentOS CAD ayarları\ntema = {mode}\nvurgu = {}\nharita-zemini = {}\nyazi-ailesi = {}\n\
             es-aralikli = {}\nyazi-boyutu = {}\nyuva = {}\n",
            self.accent.key(),
            self.backdrop.key(),
            key_of(self.typography.family.name()),
            key_of(self.typography.mono.name()),
            self.typography.size,
            self.docks.save(|panel| panel.key().to_owned()),
        )
    }
}

/// Temanın dosyadaki adı.
pub fn mode_key(mode: Mode) -> &'static str {
    match mode {
        Mode::Dark => "koyu",
        Mode::Light => "aydinlik",
        Mode::Night => "gece",
        Mode::HighContrast => "karsitlik",
    }
}

/// Ailenin dosyadaki adı: küçük harf, boşluklar tire ("IBM Plex Sans" →
/// "ibm-plex-sans").
fn key_of(name: &str) -> String {
    name.to_ascii_lowercase().replace(' ', "-")
}

#[cfg(test)]
mod tests {
    use iced::{Point, Rectangle, Size};
    use kentos_rc::widget::docking::{self, Side};

    use super::*;

    #[test]
    fn settings_round_trip() {
        let settings = Settings {
            mode: Mode::Night,
            accent: Accent::Custom(0xff8800),
            backdrop: Backdrop::Black,
            typography: Typography {
                family: Family::PlusJakartaSans,
                mono: Mono::JetBrainsMono,
                size: 15.0,
            },
            docks: {
                let mut docks = DockPanel::layout();
                docks.set_size(Side::Right, 410.0);
                docks.update(docking::Event::Collapsed(
                    docking::Slot::Docked(Side::Right, 0),
                    true,
                ));
                docks.close(DockPanel::Tasks);
                docks.float(
                    DockPanel::Table,
                    Rectangle::new(Point::new(120.0, 80.0), Size::new(600.0, 300.0)),
                );
                docks
            },
        };

        // Kapanan panelin kenarı saklanmaz; yeniden açılınca kendi
        // kenarına döner.
        let mut expected = settings.clone();
        expected.docks = Docks::load(
            &settings.docks.save(|panel| panel.key().to_owned()),
            DockPanel::parse,
        )
        .expect("yerleşim");

        assert_eq!(Settings::parse(&settings.render()), expected);
        assert_eq!(
            expected.docks.slot(DockPanel::Table),
            Some(docking::Slot::Floating(0))
        );
    }

    #[test]
    fn broken_lines_keep_the_defaults() {
        let settings = Settings::parse(
            "# yorum\ntema = mor\nyazi-ailesi = comic-sans\nyazi-boyutu = 99\nbilinmeyen = 1\n\
             bozuk satır\nyuva = bozuk\nvurgu = lacivert\nharita-zemini = mavi",
        );

        assert_eq!(settings.mode, Mode::Dark);
        assert_eq!(settings.accent, Accent::Blue);
        assert_eq!(settings.backdrop, Backdrop::Theme);
        assert_eq!(settings.typography.family, Family::IbmPlexSans);
        assert_eq!(settings.typography.size, 18.0);
        assert_eq!(settings.docks, DockPanel::layout());
    }

    #[test]
    fn files_are_written_and_read_back() {
        let folder = std::env::temp_dir().join(format!("kentos-ayarlar-{}", std::process::id()));
        let path = folder.join("alt").join("ayarlar");
        let settings = Settings {
            mode: Mode::Dark,
            accent: Accent::Violet,
            backdrop: Backdrop::Paper,
            typography: Typography {
                family: Family::Inter,
                ..Typography::DEFAULT
            },
            docks: DockPanel::layout(),
        };

        settings.save(&path).expect("ayar dosyası yazılamadı");
        assert_eq!(Settings::load(&path), settings);
        assert_eq!(Settings::load(&folder.join("yok")), Settings::default());

        let _ = fs::remove_dir_all(folder);
    }
}
