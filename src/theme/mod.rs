//! Tema: renk belirteçleri, vurgu rengi, tip ölçeği ve iced teması.
//!
//! Uygulama `iced::application(..).theme(..)` için [`theme`] fonksiyonunu
//! temayla ([`Mode`]: koyu, aydınlık, gece, yüksek karşıtlık) ve vurgu
//! rengiyle çağırır. Bileşenler bu temadan [`Tokens::of`] ile renklerini
//! okur; uygulamanın renkleri bileşenlere taşıması gerekmez. Tema bir alt
//! ağaca `iced::widget::themer` ile de verilebilir; belirteçler o temayı
//! izler.

pub mod accent;
pub mod motion;
pub mod tokens;
pub mod typography;

use std::sync::{Mutex, PoisonError};

use iced::Theme;

pub use accent::Accent;
pub use tokens::Tokens;

/// Tema.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mode {
    /// CAD programlarının grafit arayüzü (varsayılan).
    #[default]
    Dark,
    /// Kâğıt zeminli aydınlık arayüz.
    Light,
    /// Gece çalışması için çok koyu, az parlak arayüz.
    Night,
    /// Siyah zemin, beyaz yazı, parlak kenarlar.
    HighContrast,
}

impl Mode {
    /// Temalar, seçim listelerindeki sırasıyla.
    pub const ALL: [Mode; 4] = [Mode::Dark, Mode::Light, Mode::Night, Mode::HighContrast];

    /// Koyu ve aydınlık arasında geçiş: koyu temaların hepsi aydınlığa,
    /// aydınlık koyuya döner.
    pub fn toggled(self) -> Self {
        match self {
            Mode::Light => Mode::Dark,
            Mode::Dark | Mode::Night | Mode::HighContrast => Mode::Light,
        }
    }

    pub fn is_dark(self) -> bool {
        self != Mode::Light
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Dark => "Koyu",
            Mode::Light => "Aydınlık",
            Mode::Night => "Gece",
            Mode::HighContrast => "Yüksek karşıtlık",
        }
    }

    /// Temanın belirteçleri, verilen vurgu rengiyle.
    pub fn tokens(self, accent: Accent) -> Tokens {
        Tokens::base(self).with_accent(accent.color(self))
    }

    /// iced temasının teması: KentOS temasıysa yüzey renginden tanınır;
    /// başka bir iced temasında koyu ya da aydınlık sayılır.
    pub fn of(theme: &Theme) -> Self {
        let background = theme.palette().background;

        Self::ALL
            .into_iter()
            .find(|mode| Tokens::base(*mode).surface == background)
            .unwrap_or(if theme.extended_palette().is_dark {
                Mode::Dark
            } else {
                Mode::Light
            })
    }
}

/// Kipe ve vurgu rengine karşılık gelen iced teması. Onay kutusu, kaydırıcı
/// ve açılır liste gibi iced'in kendi bileşenleri de bu tema üzerinden aynı
/// renkleri alır. Son kurulan tema saklanır; aynı seçimle yeniden kurulmaz.
pub fn theme(mode: Mode, accent: Accent) -> Theme {
    static LAST: Mutex<Option<(Mode, Accent, Theme)>> = Mutex::new(None);

    let mut last = LAST.lock().unwrap_or_else(PoisonError::into_inner);

    if let Some((last_mode, last_accent, theme)) = last.as_ref()
        && (*last_mode, *last_accent) == (mode, accent)
    {
        return theme.clone();
    }

    let name = match mode {
        Mode::Dark => "KentOS Koyu",
        Mode::Light => "KentOS Aydınlık",
        Mode::Night => "KentOS Gece",
        Mode::HighContrast => "KentOS Yüksek Karşıtlık",
    };

    let theme = build(name, mode.tokens(accent));
    *last = Some((mode, accent, theme.clone()));
    theme
}

fn build(name: &'static str, tokens: Tokens) -> Theme {
    Theme::custom(
        name,
        iced::theme::Palette {
            background: tokens.surface,
            text: tokens.text,
            primary: tokens.accent,
            success: tokens.success,
            warning: tokens.warning,
            danger: tokens.danger,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_follow_the_theme_accent() {
        let blue = Tokens::of(&theme(Mode::Dark, Accent::Blue));
        assert_eq!(blue.accent, Tokens::DARK.accent);
        assert_eq!(blue.on_accent, iced::Color::WHITE);

        // Açık vurguda yazı koyu olur; aydınlık temada koyu ton seçilir.
        let amber = Tokens::of(&theme(Mode::Dark, Accent::Amber));
        assert_eq!(amber.accent, Accent::Amber.color(Mode::Dark));
        assert_ne!(amber.on_accent, iced::Color::WHITE);

        let light = Tokens::of(&theme(Mode::Light, Accent::Amber));
        assert!(!light.is_dark);
        assert_eq!(light.accent, Accent::Amber.color(Mode::Light));
        assert_eq!(light.on_accent, iced::Color::WHITE);

        // Aynı seçimle kurulan tema önceki temadır.
        assert_eq!(
            theme(Mode::Light, Accent::Amber),
            theme(Mode::Light, Accent::Amber)
        );
    }

    #[test]
    fn every_theme_is_recognised_from_its_iced_theme() {
        for mode in Mode::ALL {
            for accent in Accent::PRESETS {
                let tokens = Tokens::of(&theme(mode, accent));

                assert_eq!(tokens.mode, mode, "{mode:?} {accent:?}");
                assert_eq!(tokens.surface, Tokens::base(mode).surface);
                assert_eq!(tokens.accent, accent.color(mode));
            }
        }

        // Başka bir iced teması koyu ya da aydınlık sayılır.
        assert_eq!(Mode::of(&Theme::Light), Mode::Light);
        assert_eq!(Mode::of(&Theme::Dracula), Mode::Dark);
    }

    #[test]
    fn high_contrast_picks_the_more_readable_text_on_accent() {
        let tokens = Tokens::of(&theme(Mode::HighContrast, Accent::Blue));

        assert_eq!(tokens.text, iced::Color::WHITE);
        assert_eq!(tokens.on_accent, iced::Color::BLACK);
        assert!(tokens.selection().a > Tokens::of(&theme(Mode::Dark, Accent::Blue)).selection().a);

        // Koyu temaların hepsi aydınlığa döner.
        assert_eq!(Mode::Night.toggled(), Mode::Light);
        assert_eq!(Mode::Light.toggled(), Mode::Dark);
        assert!(Mode::HighContrast.is_dark());
    }
}
