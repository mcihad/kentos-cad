//! Tema: renk belirteçleri, vurgu rengi, tip ölçeği ve iced teması.
//!
//! Uygulama `iced::application(..).theme(..)` için [`theme`] fonksiyonunu
//! kipi ve vurgu rengiyle çağırır. Bileşenler bu temadan [`Tokens::of`] ile
//! renklerini okur; uygulamanın renkleri bileşenlere taşıması gerekmez.

pub mod accent;
pub mod tokens;
pub mod typography;

use std::sync::{Mutex, PoisonError};

use iced::Theme;

pub use accent::Accent;
pub use tokens::Tokens;

/// Tema kipi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Mode {
    /// CAD programlarının grafit arayüzü (varsayılan).
    #[default]
    Dark,
    /// Kâğıt zeminli aydınlık arayüz.
    Light,
}

impl Mode {
    pub fn toggled(self) -> Self {
        match self {
            Mode::Dark => Mode::Light,
            Mode::Light => Mode::Dark,
        }
    }

    pub fn is_dark(self) -> bool {
        self == Mode::Dark
    }

    pub fn label(self) -> &'static str {
        match self {
            Mode::Dark => "Koyu",
            Mode::Light => "Aydınlık",
        }
    }

    /// Kipin belirteçleri, verilen vurgu rengiyle.
    pub fn tokens(self, accent: Accent) -> Tokens {
        let base = match self {
            Mode::Dark => Tokens::DARK,
            Mode::Light => Tokens::LIGHT,
        };

        base.with_accent(accent.color(self))
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
}
