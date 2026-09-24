//! Vurgu rengi: etkin araç, seçim, odak, birincil düğmeler ve öndeki
//! pencerenin çizgisi.
//!
//! Sekiz hazır renk vardır; her birinin koyu ve aydınlık tema için ayrı
//! tonu seçilmiştir. Kullanıcı kendi rengini `#RRGGBB` olarak da verebilir:
//! renk, temanın zemininde okunur kalacak kadar açılır ya da koyulaştırılır.
//! Vurgu zeminindeki yazı, rengin açıklığına göre beyaz ya da koyu olur
//! ([`Tokens::with_accent`](super::Tokens::with_accent)).
//!
//! ```ignore
//! iced::application(App::new, App::update, App::view)
//!     .theme(|app: &App| theme::theme(app.mode, app.accent))
//! ```

use iced::Color;

use super::Mode;
use super::tokens::{Tokens, hex};

/// Vurgunun temanın yüzeyine karşı en az karşıtlığı; yüksek karşıtlık
/// temasında daha yüksek.
const CONTRAST: f32 = 4.0;
const HIGH_CONTRAST: f32 = 7.0;

/// Gece temasında vurgunun yüzeye doğru kısılma oranı.
const NIGHT_DIM: f32 = 0.12;

/// Vurgu rengi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Accent {
    /// Mavi: KentOS'un varsayılanı.
    #[default]
    Blue,
    Turquoise,
    Green,
    Amber,
    Orange,
    Pink,
    Violet,
    /// Renksiz, sakin bir vurgu.
    Gray,
    /// Kullanıcının verdiği renk, `0xRRGGBB`.
    Custom(u32),
}

impl Accent {
    /// Hazır renkler, seçim listelerindeki sırasıyla.
    pub const PRESETS: [Accent; 8] = [
        Accent::Blue,
        Accent::Turquoise,
        Accent::Green,
        Accent::Amber,
        Accent::Orange,
        Accent::Pink,
        Accent::Violet,
        Accent::Gray,
    ];

    /// Arayüzde gösterilen adı; kullanıcının renginde `#rrggbb`.
    pub fn name(self) -> String {
        let name = match self {
            Accent::Blue => "Mavi",
            Accent::Turquoise => "Turkuaz",
            Accent::Green => "Yeşil",
            Accent::Amber => "Kehribar",
            Accent::Orange => "Turuncu",
            Accent::Pink => "Pembe",
            Accent::Violet => "Mor",
            Accent::Gray => "Gri",
            Accent::Custom(rgb) => return format!("#{rgb:06x}"),
        };

        name.to_owned()
    }

    /// Ayar dosyasındaki adı: "mavi", "#ff8800".
    pub fn key(self) -> String {
        match self {
            Accent::Custom(_) => self.name(),
            preset => ascii(&preset.name()),
        }
    }

    /// Hazır rengin adı ya da `#RRGGBB` (başındaki # isteğe bağlı);
    /// büyük/küçük harf ve Türkçe karakter ayırmaz.
    pub fn parse(text: &str) -> Option<Self> {
        let text = text.trim();
        let key = ascii(text);

        if let Some(preset) = Self::PRESETS
            .into_iter()
            .find(|preset| ascii(&preset.name()) == key)
        {
            return Some(preset);
        }

        let digits = text.strip_prefix('#').unwrap_or(text);

        (digits.len() == 6 && digits.chars().all(|digit| digit.is_ascii_hexdigit()))
            .then(|| u32::from_str_radix(digits, 16).ok())
            .flatten()
            .map(Accent::Custom)
    }

    /// Temadaki tonu. Hazır renklerin koyu ve aydınlık tonları elle
    /// seçilmiştir; gece temasında renk biraz kısılır. Her renk, hazırlar
    /// dahil, temanın zemininde okunur kalacak kadar (yüksek karşıtlıkta
    /// 7:1) açılır ya da koyulaştırılır.
    pub fn color(self, mode: Mode) -> Color {
        let base = Tokens::base(mode);

        let (toward, minimum) = match mode {
            Mode::Light => (Color::BLACK, CONTRAST),
            Mode::HighContrast => (Color::WHITE, HIGH_CONTRAST),
            Mode::Dark | Mode::Night => (Color::WHITE, CONTRAST),
        };

        let tone = match mode {
            Mode::Night => mix(self.tone(mode), base.surface, NIGHT_DIM),
            _ => self.tone(mode),
        };

        readable(tone, base.surface, toward, minimum)
    }

    /// Hazır rengin temadaki tonu ya da kullanıcının rengi, düzeltilmeden.
    fn tone(self, mode: Mode) -> Color {
        let (dark, light) = match self {
            Accent::Blue => (0x4c9be8, 0x1b6fd0),
            Accent::Turquoise => (0x2db5ac, 0x08766f),
            Accent::Green => (0x3cb483, 0x137a4b),
            Accent::Amber => (0xe2a93b, 0x8c5c00),
            Accent::Orange => (0xee8446, 0xb04a0b),
            Accent::Pink => (0xe3689b, 0xbc2c6b),
            Accent::Violet => (0x9d86f0, 0x6547cf),
            Accent::Gray => (0xaeb6c0, 0x4f5863),
            Accent::Custom(rgb) => return hex(rgb),
        };

        hex(if mode.is_dark() { dark } else { light })
    }
}

/// Rengi `background` üzerinde en az `minimum` karşıtlığa ulaşana dek
/// `toward` rengine doğru kaydırır.
fn readable(color: Color, background: Color, toward: Color, minimum: f32) -> Color {
    (0..=20)
        .map(|step| mix(color, toward, step as f32 * 0.05))
        .find(|candidate| contrast(*candidate, background) >= minimum)
        .unwrap_or(toward)
}

/// İki rengin karışımı: `amount` 0 ise `color`, 1 ise `other`.
pub(crate) fn mix(color: Color, other: Color, amount: f32) -> Color {
    Color {
        r: color.r + (other.r - color.r) * amount,
        g: color.g + (other.g - color.g) * amount,
        b: color.b + (other.b - color.b) * amount,
        a: color.a,
    }
}

/// Göreli parlaklık (WCAG): 0 siyah, 1 beyaz.
pub(crate) fn luminance(color: Color) -> f32 {
    let linear = |channel: f32| {
        if channel <= 0.040_45 {
            channel / 12.92
        } else {
            ((channel + 0.055) / 1.055).powf(2.4)
        }
    };

    0.2126 * linear(color.r) + 0.7152 * linear(color.g) + 0.0722 * linear(color.b)
}

/// İki rengin karşıtlık oranı (WCAG): 1 ile 21 arası.
pub(crate) fn contrast(a: Color, b: Color) -> f32 {
    let (a, b) = (luminance(a), luminance(b));

    (a.max(b) + 0.05) / (a.min(b) + 0.05)
}

/// Adın karşılaştırma biçimi: küçük harf, Türkçe harfler ASCII.
fn ascii(text: &str) -> String {
    text.chars()
        .map(|character| match character {
            'ç' | 'Ç' => 'c',
            'ğ' | 'Ğ' => 'g',
            'ı' | 'I' | 'İ' => 'i',
            'ö' | 'Ö' => 'o',
            'ş' | 'Ş' => 's',
            'ü' | 'Ü' => 'u',
            other => other.to_ascii_lowercase(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_and_hex_codes_are_parsed() {
        assert_eq!(Accent::parse("mavi"), Some(Accent::Blue));
        assert_eq!(Accent::parse("  YEŞİL "), Some(Accent::Green));
        assert_eq!(Accent::parse("kehribar"), Some(Accent::Amber));
        assert_eq!(Accent::parse("#FF8800"), Some(Accent::Custom(0xff8800)));
        assert_eq!(Accent::parse("ff8800"), Some(Accent::Custom(0xff8800)));
        assert_eq!(Accent::parse("#ff880"), None);
        assert_eq!(Accent::parse("lacivert"), None);

        for accent in Accent::PRESETS
            .into_iter()
            .chain([Accent::Custom(0x0a0b0c)])
        {
            assert_eq!(Accent::parse(&accent.key()), Some(accent), "{accent:?}");
        }
    }

    #[test]
    fn every_accent_reads_on_every_theme() {
        let colors = Accent::PRESETS
            .into_iter()
            .chain([0x000000, 0xffffff, 0xffee00, 0x202040, 0x7f7f7f].map(Accent::Custom));

        for accent in colors {
            for mode in Mode::ALL {
                let minimum = if mode == Mode::HighContrast {
                    HIGH_CONTRAST
                } else {
                    CONTRAST
                };
                let ratio = contrast(accent.color(mode), Tokens::base(mode).surface);

                assert!(ratio >= minimum, "{accent:?} {mode:?}: {ratio}");
            }
        }
    }

    #[test]
    fn readable_custom_colors_are_left_alone() {
        assert_eq!(Accent::Custom(0x4c9be8).color(Mode::Dark), hex(0x4c9be8));
        assert_eq!(Accent::Blue.color(Mode::Light), hex(0x1b6fd0));
        assert!(contrast(Color::WHITE, Color::BLACK) > 20.9);
    }
}
