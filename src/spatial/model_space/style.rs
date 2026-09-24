//! Model alanının renkleri ve zemini.

use iced::{Color, Theme};

use crate::theme::tokens::{hex, hexa};
use crate::theme::{Mode, Tokens};

/// Model alanının zemini; arayüzün temasından bağımsız seçilebilir (ör.
/// koyu arayüzde kâğıt zeminli harita).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backdrop {
    /// Arayüzün temasına uyar: koyu temada arduvaz, aydınlıkta kâğıt,
    /// gecede gece haritası, yüksek karşıtlıkta siyah.
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
    /// Koyu zeminde katman renklerinin parlaklığı: gece haritasında
    /// renkler kısılır.
    pub brightness: f32,
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
        brightness: 1.0,
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
        brightness: 1.0,
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
        brightness: 1.0,
    };

    /// Gece haritası: mavimsi, çok koyu zemin; ızgara, etiketler ve katman
    /// renkleri kısık.
    pub const NIGHT: Self = Self {
        is_dark: true,
        background: hex(0x0c1016),
        grid: hex(0x141a22),
        grid_major: hex(0x1d2530),
        grid_label: hex(0x58626e),
        label: hex(0xaeb7c1),
        symbol_outline: hex(0x0c1016),
        selection: Tokens::NIGHT.accent,
        grip: Tokens::NIGHT.accent,
        measure: hex(0xe08a45),
        snap: hex(0xd6ae3a),
        axis_x: hex(0xc9544c),
        axis_y: hex(0x4fae5a),
        crosshair: hexa(0xb9c1ca, 0.45),
        crosshair_small: hexa(0xb9c1ca, 0.75),
        tag_background: hexa(0x0b0d11, 0.94),
        tag_text: hex(0xb9c1ca),
        window: hex(0x3f86cf),
        crossing: hex(0x4fae5a),
        brightness: 0.82,
    };

    /// Yüksek karşıtlık haritası: siyah zemin, belirgin ızgara, beyaz
    /// etiketler.
    pub const HIGH_CONTRAST: Self = Self {
        is_dark: true,
        background: hex(0x000000),
        grid: hex(0x2b2f35),
        grid_major: hex(0x4b525b),
        grid_label: hex(0xc4c9cf),
        label: hex(0xffffff),
        symbol_outline: hex(0x000000),
        selection: Tokens::HIGH_CONTRAST.accent,
        grip: Tokens::HIGH_CONTRAST.accent,
        measure: hex(0xffa24d),
        snap: hex(0xffd84a),
        axis_x: hex(0xff6b61),
        axis_y: hex(0x6ee07a),
        crosshair: hexa(0xffffff, 0.7),
        crosshair_small: hex(0xffffff),
        tag_background: hexa(0x000000, 0.94),
        tag_text: hex(0xffffff),
        window: hex(0x6cb4ff),
        crossing: hex(0x6ee07a),
        brightness: 1.0,
    };

    /// Temanın zemini: koyu temada arduvaz, aydınlıkta kâğıt, gecede gece
    /// haritası, yüksek karşıtlıkta siyah.
    pub fn of(theme: &Theme) -> Self {
        Self::with(Backdrop::Theme, theme)
    }

    /// Verilen zeminin renkleri; seçim ve tutamaçlar temanın vurgu
    /// rengindedir.
    pub fn with(backdrop: Backdrop, theme: &Theme) -> Self {
        let tokens = Tokens::of(theme);

        let base = match backdrop {
            Backdrop::Theme => match tokens.mode {
                Mode::Dark => Self::DARK,
                Mode::Light => Self::LIGHT,
                Mode::Night => Self::NIGHT,
                Mode::HighContrast => Self::HIGH_CONTRAST,
            },
            Backdrop::Slate => Self::DARK,
            Backdrop::Black => Self::BLACK,
            Backdrop::Paper => Self::LIGHT,
        };

        Self {
            selection: tokens.accent,
            grip: tokens.accent,
            ..base
        }
    }

    /// Katman rengini zemine uyarlar: kâğıt zeminde renkleri biraz
    /// koyulaştırır ki ince çizgiler okunaklı kalsın; gece haritasında
    /// parlamasın diye kısar.
    pub fn layer_color(&self, color: Color) -> Color {
        if self.is_dark {
            Color {
                r: color.r * self.brightness,
                g: color.g * self.brightness,
                b: color.b * self.brightness,
                a: color.a,
            }
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

    #[test]
    fn themed_maps_follow_night_and_high_contrast() {
        let night = Style::of(&theme(Mode::Night, Accent::Blue));
        let contrast = Style::of(&theme(Mode::HighContrast, Accent::Blue));

        assert_eq!(night.background, Style::NIGHT.background);
        assert_eq!(contrast.background, Style::HIGH_CONTRAST.background);

        // Gece haritasında katman renkleri kısılır.
        let red = Color::from_rgb(1.0, 0.0, 0.0);
        assert!(night.layer_color(red).r < 1.0);
        assert_eq!(Style::DARK.layer_color(red), red);

        // Seçilen zemin temadan bağımsızdır.
        assert_eq!(
            Style::with(Backdrop::Paper, &theme(Mode::Night, Accent::Blue)).background,
            Style::LIGHT.background
        );
    }
}
