//! Renk belirteçleri (design tokens).

use iced::Color;

/// Arayüzün bütün renkleri.
///
/// Bileşenler hiçbir rengi doğrudan kullanmaz; stil fonksiyonları o anki
/// temadan [`Tokens::of`] ile belirteçleri alır. Böylece koyu ve aydınlık
/// temalar, ileride eklenecek temalar dahil, tek yerden yönetilir.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Tokens {
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

    /// Temanın koyu ya da aydınlık olmasına göre belirteçleri döndürür.
    pub fn of(theme: &iced::Theme) -> Self {
        if theme.extended_palette().is_dark {
            Self::DARK
        } else {
            Self::LIGHT
        }
    }

    /// Seçili satır, etkin araç ve açık alt menü zemini.
    pub fn selection(&self) -> Color {
        self.accent
            .scale_alpha(if self.is_dark { 0.22 } else { 0.14 })
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
