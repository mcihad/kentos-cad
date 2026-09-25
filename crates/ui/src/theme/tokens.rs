//! Renk belirteçleri (design tokens).

use iced::Color;

use super::Mode;
use super::accent::{contrast, mix};

/// Arayüzün bütün renkleri.
///
/// Bileşenler hiçbir rengi doğrudan kullanmaz; stil fonksiyonları o anki
/// temadan [`Tokens::of`] ile belirteçleri alır. Böylece dört tema (koyu,
/// aydınlık, gece, yüksek karşıtlık) tek yerden yönetilir. Vurgu rengi
/// temanın `primary` rengidir ([`theme`](super::theme)); vurgunun üzerine
/// gelme tonu ve vurgu zeminindeki yazı ondan türetilir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens {
    /// Belirteçlerin teması.
    pub mode: Mode,
    pub is_dark: bool,

    /// En dış çerçeve: sekme şeridi ve durum çubuğu.
    pub window: Color,
    /// Şerit, paneller ve menülerin gövdesi.
    pub surface: Color,
    /// Tablo hücreleri, özellik değerleri ve ikincil yüzeyler.
    pub surface_alt: Color,
    /// Üzerine gelinen satır ve düğmeler.
    pub surface_hover: Color,
    /// Panel başlıkları ve özellik ızgarasının kategori satırları.
    pub header: Color,
    /// Metin girişleri ve açılır listeler.
    pub field: Color,
    /// Bölücüler ve kenarlar (1 piksellik çizgiler).
    pub border: Color,

    pub text: Color,
    /// İkincil metin: açıklamalar, başlık meta bilgisi, pasif sekmeler.
    pub muted: Color,

    /// Etkin araç, seçim ve odak rengi.
    pub accent: Color,
    /// Vurgunun daha güçlü hâli: üzerine gelme ve vurgu zemininde metin.
    pub accent_hover: Color,
    /// Vurgu zemini üzerindeki metin ve ikonlar.
    pub on_accent: Color,

    /// Açılır menüler, ipuçları ve iletişim kutuları.
    pub popover: Color,

    pub success: Color,
    pub warning: Color,
    pub danger: Color,
}

impl Tokens {
    pub const DARK: Self = Self {
        mode: Mode::Dark,
        is_dark: true,

        window: hex(0x1d1f23),
        surface: hex(0x282b30),
        surface_alt: hex(0x2f3338),
        surface_hover: hex(0x3a3f46),
        header: hex(0x33373d),
        field: hex(0x18191c),
        border: hex(0x40454d),

        text: hex(0xd8dce1),
        muted: hex(0x8d959f),

        accent: hex(0x4c9be8),
        accent_hover: hex(0x6fb1f0),
        on_accent: hex(0xffffff),

        popover: hex(0x2a2d32),

        success: hex(0x5cbf62),
        warning: hex(0xf2c53d),
        danger: hex(0xe5584f),
    };

    pub const LIGHT: Self = Self {
        mode: Mode::Light,
        is_dark: false,

        window: hex(0xdcdfe3),
        surface: hex(0xeceef0),
        surface_alt: hex(0xf7f8f9),
        surface_hover: hex(0xd5dae0),
        header: hex(0xe1e4e8),
        field: hex(0xffffff),
        border: hex(0xbcc2c9),

        text: hex(0x1e2226),
        muted: hex(0x5c636c),

        accent: hex(0x1b6fd0),
        accent_hover: hex(0x155cae),
        on_accent: hex(0xffffff),

        popover: hex(0xf7f8f9),

        success: hex(0x2f9437),
        warning: hex(0xb58500),
        danger: hex(0xcc3a31),
    };

    /// Gece çalışması için: mavimsi, çok koyu yüzeyler ve daha az parlak
    /// yazı; gözü yormaz, karanlık odada ekran parlamaz.
    pub const NIGHT: Self = Self {
        mode: Mode::Night,
        is_dark: true,

        window: hex(0x0b0d11),
        surface: hex(0x12151b),
        surface_alt: hex(0x171b22),
        surface_hover: hex(0x20252e),
        header: hex(0x191d24),
        field: hex(0x0a0c10),
        border: hex(0x272d36),

        text: hex(0xb9c1ca),
        muted: hex(0x77808c),

        accent: hex(0x4589cb),
        accent_hover: hex(0x60a0d8),
        on_accent: hex(0xffffff),

        popover: hex(0x151920),

        success: hex(0x4fae5a),
        warning: hex(0xd6ae3a),
        danger: hex(0xd4544b),
    };

    /// Yüksek karşıtlık: siyah zemin, beyaz yazı, parlak kenarlar. Bölgeler
    /// zemin tonlarıyla değil kenarlarla ayrılır; yazılar en az 7:1
    /// karşıtlıktadır.
    pub const HIGH_CONTRAST: Self = Self {
        mode: Mode::HighContrast,
        is_dark: true,

        window: hex(0x000000),
        surface: hex(0x000000),
        surface_alt: hex(0x0d0d0d),
        surface_hover: hex(0x2a2a2a),
        header: hex(0x161616),
        field: hex(0x000000),
        border: hex(0xb3b3b3),

        text: hex(0xffffff),
        muted: hex(0xd6d6d6),

        accent: hex(0x6cb4ff),
        accent_hover: hex(0x8fc6ff),
        on_accent: hex(0x000000),

        popover: hex(0x0a0a0a),

        success: hex(0x6ee07a),
        warning: hex(0xffd84a),
        danger: hex(0xff6b61),
    };

    /// Temanın vurgusuz, sabit belirteçleri.
    pub const fn base(mode: Mode) -> Self {
        match mode {
            Mode::Dark => Self::DARK,
            Mode::Light => Self::LIGHT,
            Mode::Night => Self::NIGHT,
            Mode::HighContrast => Self::HIGH_CONTRAST,
        }
    }

    /// Temanın belirteçleri: temanın seti ([`Mode::of`]), vurgu rengiyle.
    pub fn of(theme: &iced::Theme) -> Self {
        Self::base(Mode::of(theme)).with_accent(theme.palette().primary)
    }

    /// Vurgu rengi verilmiş belirteçler. Üzerine gelme tonu koyu temada
    /// açılarak, aydınlıkta koyulaşarak bulunur. Vurgu zeminindeki yazı
    /// beyazdır; vurgu beyazla okunamayacak kadar açıksa koyu olur. Yüksek
    /// karşıtlıkta hangisi daha okunaklıysa o seçilir.
    pub fn with_accent(self, accent: Color) -> Self {
        let accent_hover = if self.is_dark {
            mix(accent, Color::WHITE, 0.18)
        } else {
            mix(accent, Color::BLACK, 0.16)
        };

        let dark = hex(0x16181b);
        let white = contrast(Color::WHITE, accent);

        let on_accent = match self.mode {
            Mode::HighContrast if contrast(Color::BLACK, accent) > white => Color::BLACK,
            Mode::HighContrast => Color::WHITE,
            _ if white >= 2.8 => Color::WHITE,
            _ => dark,
        };

        Self {
            accent,
            accent_hover,
            on_accent,
            ..self
        }
    }

    /// Seçili satır, etkin araç ve açık alt menü zemini.
    pub fn selection(&self) -> Color {
        self.accent.scale_alpha(match self.mode {
            Mode::HighContrast => 0.4,
            Mode::Light => 0.14,
            Mode::Dark | Mode::Night => 0.22,
        })
    }

    /// Zeminin ne olduğundan bağımsız, hafif bir durum katmanı: koyu temada
    /// açık, aydınlık temada koyu. Üzerine gelinen ya da basılı öğeler için.
    pub fn layer(&self, alpha: f32) -> Color {
        if self.is_dark {
            Color::from_rgba(1.0, 1.0, 1.0, alpha)
        } else {
            Color::from_rgba(0.0, 0.0, 0.0, alpha)
        }
    }

    /// Yakalama işaretleri ve kılavuzları: model alanında nesne yakalaması,
    /// kayan pencerelerde kenar yakalaması aynı sarıyla çizilir.
    pub fn snap(&self) -> Color {
        self.warning
    }

    /// Devre dışı öğelerin metni ve ikonları.
    pub fn disabled(&self) -> Color {
        self.muted.scale_alpha(0.55)
    }

    /// Açılır menü ve iletişim kutularının gölgesi.
    pub fn shadow(&self) -> Color {
        Color::from_rgba(0.0, 0.0, 0.0, if self.is_dark { 0.4 } else { 0.14 })
    }

    /// Kalıcı (modal) iletişim kutularının arkasındaki karartma.
    pub fn scrim(&self) -> Color {
        Color::from_rgba(0.0, 0.0, 0.0, 0.25)
    }
}

/// `0xRRGGBB` biçimindeki rengi çözer.
pub const fn hex(rgb: u32) -> Color {
    Color::from_rgb(
        ((rgb >> 16) & 0xff) as f32 / 255.0,
        ((rgb >> 8) & 0xff) as f32 / 255.0,
        (rgb & 0xff) as f32 / 255.0,
    )
}

/// `0xRRGGBB` biçimindeki rengi verilen saydamlıkla çözer.
pub const fn hexa(rgb: u32, alpha: f32) -> Color {
    let color = hex(rgb);

    Color::from_rgba(color.r, color.g, color.b, alpha)
}
