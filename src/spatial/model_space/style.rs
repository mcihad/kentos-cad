//! Model alanının renkleri.

use iced::{Color, Theme};

use crate::theme::Tokens;
use crate::theme::tokens::{hex, hexa};

/// Model alanı renkleri. Koyu temada AutoCAD'in arduvaz model alanını,
/// aydınlık temada kâğıt zemini izler.
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
    };

    /// Temanın koyu ya da aydınlık olmasına göre renkler.
    pub fn of(theme: &Theme) -> Self {
        if Tokens::of(theme).is_dark {
            Self::DARK
        } else {
            Self::LIGHT
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
