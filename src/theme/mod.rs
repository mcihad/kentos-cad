//! Tema: renk belirteçleri, tip ölçeği ve iced teması.
//!
//! Uygulama `iced::application(..).theme(..)` için [`theme`] fonksiyonunu
//! kullanır. Bileşenler bu temadan [`Tokens::of`] ile renklerini okur;
//! uygulamanın renkleri bileşenlere taşıması gerekmez.

pub mod tokens;
pub mod typography;

use std::sync::LazyLock;

use iced::Theme;

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

    pub fn tokens(self) -> Tokens {
        match self {
            Mode::Dark => Tokens::DARK,
            Mode::Light => Tokens::LIGHT,
        }
    }
}

/// Kipe karşılık gelen iced teması. Onay kutusu, kaydırıcı ve açılır liste
/// gibi iced'in kendi bileşenleri de bu tema üzerinden aynı renkleri alır.
pub fn theme(mode: Mode) -> Theme {
    static DARK: LazyLock<Theme> = LazyLock::new(|| build("KentOS Koyu", Tokens::DARK));
    static LIGHT: LazyLock<Theme> = LazyLock::new(|| build("KentOS Aydınlık", Tokens::LIGHT));

    match mode {
        Mode::Dark => DARK.clone(),
        Mode::Light => LIGHT.clone(),
    }
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
