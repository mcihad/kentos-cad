//! Model alanının renkleri ve zemini.

use iced::{Color, Theme};

use crate::theme::Tokens;
use crate::theme::tokens::{hex, hexa};

/// Model alanının zemini; arayüzün temasından bağımsız seçilebilir (ör.
/// koyu arayüzde kâğıt zeminli harita).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backdrop {
    /// Arayüzün kipine uyar: koyu temada arduvaz, aydınlıkta kâğıt.
    #[default]
    Theme,
    /// AutoCAD'in koyu gri-mavi model alanı.
    Slate,
    /// Klasik AutoCAD'in siyah model alanı.
    Black,
    /// Beyaza yakın kâğıt.
    Paper,
}

impl Backdrop {
    pub const ALL: [Backdrop; 4] = [
        Backdrop::Theme,
        Backdrop::Slate,
        Backdrop::Black,
        Backdrop::Paper,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Backdrop::Theme => "Temaya uy",
            Backdrop::Slate => "Arduvaz",
            Backdrop::Black => "Siyah",
            Backdrop::Paper => "Kâğıt",
        }
    }

    /// Ayar dosyasındaki ve komut satırındaki adı.
    pub fn key(self) -> &'static str {
        match self {
            Backdrop::Theme => "tema",
            Backdrop::Slate => "arduvaz",
            Backdrop::Black => "siyah",
            Backdrop::Paper => "kagit",
        }
    }

    /// Anahtar ya da ad; büyük/küçük harf ve Türkçe karakter ayırmaz.
    pub fn parse(text: &str) -> Option<Self> {
        let text: String = text
            .trim()
            .chars()
            .map(|character| match character {
                'â' | 'Â' => 'a',
                'ğ' | 'Ğ' => 'g',
                'ı' | 'I' | 'İ' => 'i',
                'ş' | 'Ş' => 's',
                'ü' | 'Ü' => 'u',
                'ö' | 'Ö' => 'o',
                'ç' | 'Ç' => 'c',
                other => other.to_ascii_lowercase(),
            })
            .collect();

        Self::ALL.into_iter().find(|backdrop| {
            backdrop.key() == text || (*backdrop == Backdrop::Theme && text == "temaya uy")
        })
    }
}

/// Model alanı renkleri. Koyu temada AutoCAD'in arduvaz model alanını,
/// aydınlık temada kâğıt zemini izler; [`Backdrop`] ile başka bir zemin
/// seçilebilir. Seçim ve tutamaçlar arayüzün vurgu rengindedir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Style {
    pub is_dark: bool,
    pub background: Color,
    pub grid: Color,
    pub grid_major: Color,
    pub grid_label: Color,
    /// Öğe etiketleri ve ölçek çubuğu.
    pub label: Color,
    /// Nokta sembollerinin ve tutamaçların dış çizgisi.
    pub symbol_outline: Color,
    /// Seçili öğenin rengi.
    pub selection: Color,
    /// Seçili öğenin köşelerindeki tutamaçlar.
    pub grip: Color,
    pub measure: Color,
    pub snap: Color,
    pub axis_x: Color,
    pub axis_y: Color,
    /// Tam ekran artı imleç.
    pub crosshair: Color,
    /// Kısa artı imleç ve seçim kutusu.
    pub crosshair_small: Color,
    /// İmleç yanındaki bilgi kutuları.
    pub tag_background: Color,
    pub tag_text: Color,
    /// Pencere seçimi (soldan sağa).
    pub window: Color,
    /// Kesişen seçim (sağdan sola).
    pub crossing: Color,
}

impl Style {
    pub const DARK: Self = Self {
        is_dark: true,
        background: hex(0x212830),
        grid: hex(0x29313b),
        grid_major: hex(0x35404d),
        grid_label: hex(0x6d7a89),
        label: hex(0xd6dde5),
        symbol_outline: hex(0x212830),
        selection: Tokens::DARK.accent,
        grip: hex(0x3d8ff0),
        measure: hex(0xff9446),
        snap: hex(0xf2c53d),
        axis_x: hex(0xe5584f),
        axis_y: hex(0x5cbf62),
        crosshair: hexa(0xd8dce1, 0.55),
        crosshair_small: hexa(0xd8dce1, 0.85),
        tag_background: hexa(0x1d1f23, 0.92),
        tag_text: hex(0xd8dce1),
        window: hex(0x4c9be8),
        crossing: hex(0x5cbf62),
    };

    pub const LIGHT: Self = Self {
        is_dark: false,
        background: hex(0xfbfbf8),
        grid: hex(0xeceeef),
        grid_major: hex(0xd9dde1),
        grid_label: hex(0x7d858e),
        label: hex(0x262b31),
        symbol_outline: hex(0xfbfbf8),
        selection: Tokens::LIGHT.accent,
        grip: hex(0x1b6fd0),
        measure: hex(0xd4620b),
        snap: hex(0xb58500),
        axis_x: hex(0xcc3a31),
        axis_y: hex(0x2f9437),
        crosshair: hexa(0x1e2226, 0.45),
        crosshair_small: hexa(0x1e2226, 0.8),
        tag_background: hexa(0xffffff, 0.95),
        tag_text: hex(0x1e2226),
        window: hex(0x1b6fd0),
        crossing: hex(0x2f9437),
    };

    /// Klasik AutoCAD'in siyah model alanı: saf siyah zemin, silik ızgara,
    /// parlak etiketler.
    pub const BLACK: Self = Self {
        is_dark: true,
        background: hex(0x000000),
        grid: hex(0x15181c),
        grid_major: hex(0x262b31),
        grid_label: hex(0x6a727b),
        label: hex(0xe8ebee),
        symbol_outline: hex(0x000000),
        selection: Tokens::DARK.accent,
        grip: hex(0x3d8ff0),
        measure: hex(0xff9446),
        snap: hex(0xf2c53d),
        axis_x: hex(0xe5584f),
        axis_y: hex(0x5cbf62),
        crosshair: hexa(0xffffff, 0.5),
        crosshair_small: hexa(0xffffff, 0.85),
        tag_background: hexa(0x16181b, 0.94),
        tag_text: hex(0xe8ebee),
        window: hex(0x4c9be8),
        crossing: hex(0x5cbf62),
    };

    /// Temanın zemini: koyu temada arduvaz, aydınlıkta kâğıt.
    pub fn of(theme: &Theme) -> Self {
        Self::with(Backdrop::Theme, theme)
    }

    /// Verilen zeminin renkleri; seçim ve tutamaçlar temanın vurgu
    /// rengindedir.
    pub fn with(backdrop: Backdrop, theme: &Theme) -> Self {
        let tokens = Tokens::of(theme);

        let base = match backdrop {
            Backdrop::Theme if tokens.is_dark => Self::DARK,
            Backdrop::Theme | Backdrop::Paper => Self::LIGHT,
            Backdrop::Slate => Self::DARK,
            Backdrop::Black => Self::BLACK,
        };

        Self {
            selection: tokens.accent,
            grip: tokens.accent,
            ..base
        }
    }

    /// Katman rengini zemine uyarlar: kâğıt zeminde renkleri biraz
    /// koyulaştırır ki ince çizgiler okunaklı kalsın.
    pub fn layer_color(&self, color: Color) -> Color {
        if self.is_dark {
            color
        } else {
            Color {
                r: color.r * 0.78,
                g: color.g * 0.78,
                b: color.b * 0.80,
                a: color.a,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::theme::{Accent, Mode, theme};

    #[test]
    fn backdrops_are_chosen_apart_from_the_interface() {
        let dark = theme(Mode::Dark, Accent::Pink);
        let light = theme(Mode::Light, Accent::Pink);

        assert_eq!(Style::of(&dark).background, Style::DARK.background);
        assert_eq!(Style::of(&light).background, Style::LIGHT.background);

        // Aydınlık arayüzde siyah zemin; seçim yine vurgu renginde.
        let black = Style::with(Backdrop::Black, &light);
        assert!(black.is_dark);
        assert_eq!(black.background, Style::BLACK.background);
        assert_eq!(black.selection, Accent::Pink.color(Mode::Light));

        assert_eq!(Backdrop::parse("KÂĞIT"), Some(Backdrop::Paper));
        assert_eq!(Backdrop::parse("temaya uy"), Some(Backdrop::Theme));
        assert_eq!(Backdrop::parse("mavi"), None);

        for backdrop in Backdrop::ALL {
            assert_eq!(Backdrop::parse(backdrop.key()), Some(backdrop));
        }
    }
}
