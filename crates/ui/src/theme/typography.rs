//! Yazı tipleri ve tip ölçeği.
//!
//! Arayüzün yazı ailesi ve boyutu çalışırken değişir: uygulama [`set`] ile
//! yeni bir [`Typography`] verir, sonraki çizimde bütün metinler yeni aile ve
//! boyutla kurulur. Bileşenler yazı tipini ve boyutunu bu modülden okur
//! ([`ui`], [`body`] ...). Satır yüksekliği gibi metni taşıyan ölçüler de yazı
//! boyutuyla büyür ([`scaled`]); tam piksele yuvarlandıkları için çizgiler ve
//! kenarlar her boyutta keskin kalır.
//!
//! | Boyut       | 12 px gövde | 13 px gövde (varsayılan) | 15 px gövde |
//! |-------------|-------------|--------------------------|-------------|
//! | [`caption`] | 11          | 12                       | 14          |
//! | [`body`]    | 12          | 13                       | 15          |
//! | [`heading`] | 13          | 14                       | 16          |
//! | [`title`]   | 14          | 15                       | 17          |
//! | [`figure`]  | 20          | 22                       | 25          |
//!
//! Aileler kütüphaneye gömülüdür (`fonts` özelliği, varsayılan açık); makinede
//! kurulu olmaları gerekmez. Hepsi SIL Open Font License 1.1 ile dağıtılır,
//! kaynakları ve lisansları `assets/fonts` klasöründedir. Uygulama açılışta
//! [`load`] çağırır:
//!
//! ```ignore
//! typography::load();
//! typography::set(Typography { family: Family::Inter, ..Typography::DEFAULT });
//!
//! iced::application(App::new, App::update, App::view)
//!     .default_font(typography::ui())
//!     .run()
//! ```
//!
//! Ayar bütün uygulama için tektir. Bileşenler yazı tipini açıkça verir; iced'in
//! varsayılan yazı tipi (`default_font`) yalnızca açıkça verilmemiş metinlere
//! uygulanır ve açılışta seçilen aile olarak kalır.

use std::ops::RangeInclusive;
use std::sync::atomic::{AtomicU8, AtomicU32, Ordering};

use iced::font::Weight;
use iced::{Font, Length};

/// Arayüz metninin yazı ailesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Family {
    /// Mühendislik çizgili, dar ve sakin bir grotesk.
    #[default]
    IbmPlexSans,
    /// Ekran için çizilmiş; x yüksekliği büyük, küçük boyutta en okunaklısı.
    Inter,
    /// Geometrik, açık ve yumuşak hatlı.
    PlusJakartaSans,
}

impl Family {
    pub const ALL: [Family; 3] = [Family::IbmPlexSans, Family::Inter, Family::PlusJakartaSans];

    /// Yazı tipinin adı; gösterilen ad da budur.
    pub const fn name(self) -> &'static str {
        match self {
            Family::IbmPlexSans => "IBM Plex Sans",
            Family::Inter => "Inter",
            Family::PlusJakartaSans => "Plus Jakarta Sans",
        }
    }

    /// Harflerin ortalama genişliği (em), Türkçe arayüz metninde ölçülmüş:
    /// normal ve yarı kalın.
    const fn advance(self) -> (f32, f32) {
        match self {
            Family::IbmPlexSans => (0.456, 0.474),
            Family::Inter => (0.483, 0.493),
            Family::PlusJakartaSans => (0.472, 0.482),
        }
    }
}

/// Koordinat, ölçü ve komutların eş aralıklı yazı ailesi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Mono {
    #[default]
    IbmPlexMono,
    JetBrainsMono,
}

impl Mono {
    pub const ALL: [Mono; 2] = [Mono::IbmPlexMono, Mono::JetBrainsMono];

    pub const fn name(self) -> &'static str {
        match self {
            Mono::IbmPlexMono => "IBM Plex Mono",
            Mono::JetBrainsMono => "JetBrains Mono",
        }
    }
}

/// Yazı ayarı: aileler ve gövde metninin boyutu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Typography {
    pub family: Family,
    pub mono: Mono,
    /// Gövde metninin boyutu (piksel); diğer boyutlar ve metni taşıyan
    /// ölçüler buna göre belirlenir.
    pub size: f32,
}

impl Typography {
    pub const DEFAULT: Self = Self {
        family: Family::IbmPlexSans,
        mono: Mono::IbmPlexMono,
        size: 13.0,
    };

    /// Gövde metninin alabileceği boyutlar.
    pub const SIZES: RangeInclusive<f32> = 11.0..=18.0;

    /// Boyutu [`Typography::SIZES`] içine alır.
    pub fn clamped(self) -> Self {
        Self {
            size: self.size.clamp(*Self::SIZES.start(), *Self::SIZES.end()),
            ..self
        }
    }

    pub fn ui(&self) -> Font {
        Font::with_name(self.family.name())
    }

    pub fn ui_strong(&self) -> Font {
        Font {
            weight: Weight::Semibold,
            ..self.ui()
        }
    }

    pub fn mono(&self) -> Font {
        Font::with_name(self.mono.name())
    }

    pub fn mono_strong(&self) -> Font {
        Font {
            weight: Weight::Semibold,
            ..self.mono()
        }
    }

    pub fn caption(&self) -> f32 {
        self.size - 1.0
    }

    pub fn body(&self) -> f32 {
        self.size
    }

    pub fn heading(&self) -> f32 {
        self.size + 1.0
    }

    pub fn title(&self) -> f32 {
        self.size + 2.0
    }

    pub fn figure(&self) -> f32 {
        (self.size * 5.0 / 3.0).round()
    }

    /// Gövde metni 12 piksel iken tasarlanmış ölçünün bu ayardaki karşılığı.
    pub fn scaled(&self, px: f32) -> f32 {
        (px * self.size / BASE).round()
    }

    /// Arayüz metninin yaklaşık genişliği; `strong` yarı kalın metin için.
    pub fn text_width(&self, text: &str, size: f32, strong: bool) -> f32 {
        let (normal, semibold) = self.family.advance();
        let advance = if strong { semibold } else { normal };

        chars(text) * size * advance * WIDTH_MARGIN
    }
}

impl Default for Typography {
    fn default() -> Self {
        Self::DEFAULT
    }
}

/// Bileşenlerin ölçüleri gövde metni bu boyuttayken tasarlandı.
const BASE: f32 = 12.0;

/// Eş aralıklı ailelerin harf genişliği (em); ikisinde de aynıdır.
const MONO_ADVANCE: f32 = 0.6;

/// Genişlik tahminlerindeki pay: geniş harfleri çok olan sözcüklerde de
/// tahmin metinden dar kalmasın.
const WIDTH_MARGIN: f32 = 1.15;

static FAMILY: AtomicU8 = AtomicU8::new(0);
static MONO: AtomicU8 = AtomicU8::new(0);
static SIZE: AtomicU32 = AtomicU32::new(Typography::DEFAULT.size.to_bits());

/// Yazı ayarını değiştirir; boyut [`Typography::SIZES`] içinde tutulur.
/// Sonraki çizimde bütün metinler yeni ayarla kurulur.
pub fn set(typography: Typography) {
    let typography = typography.clamped();
    let position = |index: Option<usize>| index.unwrap_or_default() as u8;

    FAMILY.store(
        position(
            Family::ALL
                .iter()
                .position(|family| *family == typography.family),
        ),
        Ordering::Relaxed,
    );
    MONO.store(
        position(Mono::ALL.iter().position(|mono| *mono == typography.mono)),
        Ordering::Relaxed,
    );
    SIZE.store(typography.size.to_bits(), Ordering::Relaxed);
}

/// Geçerli yazı ayarı.
pub fn current() -> Typography {
    Typography {
        family: Family::ALL[usize::from(FAMILY.load(Ordering::Relaxed)) % Family::ALL.len()],
        mono: Mono::ALL[usize::from(MONO.load(Ordering::Relaxed)) % Mono::ALL.len()],
        size: f32::from_bits(SIZE.load(Ordering::Relaxed)),
    }
}

/// Arayüz metni.
pub fn ui() -> Font {
    current().ui()
}

/// Başlıklar ve vurgulu etiketler.
pub fn ui_strong() -> Font {
    current().ui_strong()
}

/// Koordinatlar, ölçüler ve komutlar için eş aralıklı yazı tipi.
pub fn mono() -> Font {
    current().mono()
}

/// Komut adları: yazılabilen anahtar sözcükler (ör. CIZGI).
pub fn mono_strong() -> Font {
    current().mono_strong()
}

/// Kontroller ve gövde metni.
pub fn body() -> f32 {
    current().body()
}

/// Açıklamalar, grup adları, tablo başlıkları, meta bilgisi.
pub fn caption() -> f32 {
    current().caption()
}

/// Menü komutları ve marka adı.
pub fn heading() -> f32 {
    current().heading()
}

/// İletişim kutusu ve seçili öğe başlıkları.
pub fn title() -> f32 {
    current().title()
}

/// Öne çıkan tek bir değer (ör. toplam uzunluk).
pub fn figure() -> f32 {
    current().figure()
}

/// Gövde metni 12 piksel iken tasarlanmış bir ölçünün (satır yüksekliği,
/// metin sütunu genişliği) o anki yazı boyutundaki karşılığı; tam piksele
/// yuvarlanır.
pub fn scaled(px: f32) -> f32 {
    current().scaled(px)
}

/// [`scaled`]'ın tersi: o anki yazı boyutundaki bir ölçünün 12 piksellik
/// gövde metnindeki karşılığı (ör. kullanıcının sürükleyerek verdiği panel
/// genişliğini yazı boyutundan bağımsız saklamak için).
pub fn unscaled(px: f32) -> f32 {
    px * BASE / body()
}

/// Sabit uzunluğu [`scaled`] ile ölçekler; esnek uzunluklar (`Fill`,
/// `Shrink`) olduğu gibi kalır.
pub fn length(length: Length) -> Length {
    match length {
        Length::Fixed(px) => Length::Fixed(scaled(px)),
        other => other,
    }
}

/// Arayüz yazı tipiyle yazılmış metnin yaklaşık genişliği (piksel): ailenin
/// ortalama harf genişliğine göre, küçük bir payla. Genişliği içeriğine göre
/// belirlenen açılır listeler ve menüler için.
pub fn text_width(text: &str, size: f32) -> f32 {
    current().text_width(text, size, false)
}

/// Yarı kalın arayüz metninin yaklaşık genişliği.
pub fn strong_width(text: &str, size: f32) -> f32 {
    current().text_width(text, size, true)
}

/// Eş aralıklı metnin genişliği.
pub fn mono_width(text: &str, size: f32) -> f32 {
    chars(text) * size * MONO_ADVANCE
}

fn chars(text: &str) -> f32 {
    text.chars().count() as f32
}

/// Gömülü yazı tipleri: her ailenin normal ve yarı kalın kesimi.
#[cfg(feature = "fonts")]
pub const FONTS: [&[u8]; 10] = [
    include_bytes!("../../assets/fonts/IBMPlexSans-Regular.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexSans-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/Inter-Regular.ttf"),
    include_bytes!("../../assets/fonts/Inter-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/PlusJakartaSans-Regular.ttf"),
    include_bytes!("../../assets/fonts/PlusJakartaSans-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexMono-Regular.ttf"),
    include_bytes!("../../assets/fonts/IBMPlexMono-SemiBold.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-Regular.ttf"),
    include_bytes!("../../assets/fonts/JetBrainsMono-SemiBold.ttf"),
];

/// Gömülü yazı tiplerini iced'in yazı tipi sistemine yükler. Uygulama açılmadan
/// önce bir kez çağrılır; tekrar çağırmak bir şey yapmaz. `fonts` özelliği
/// kapalıysa aileler makinede kurulu olmalıdır.
pub fn load() {
    #[cfg(feature = "fonts")]
    {
        let mut system = iced::advanced::graphics::text::font_system()
            .write()
            .expect("yazı tipi sistemi kilitlenemedi");

        for font in FONTS {
            system.load_font(std::borrow::Cow::Borrowed(font));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Testler genel ayarı değiştirmez: paralel çalışan diğer testler
    // ölçüleri okur. Hesaplar ayar değeri üzerinde denenir.

    #[test]
    fn scale_follows_the_body_size() {
        let large = Typography {
            family: Family::Inter,
            mono: Mono::JetBrainsMono,
            size: 15.0,
        };

        assert_eq!(large.mono(), Font::with_name("JetBrains Mono"));
        assert_eq!(
            (
                large.caption(),
                large.body(),
                large.heading(),
                large.title()
            ),
            (14.0, 15.0, 16.0, 17.0)
        );
        assert_eq!(large.figure(), 25.0);
        assert_eq!(large.scaled(24.0), 30.0);
        assert_eq!(large.ui_strong().weight, Weight::Semibold);

        assert_eq!(Typography::DEFAULT.figure(), 22.0);
        assert_eq!(Typography::DEFAULT.scaled(1.0), 1.0);
        assert_eq!(
            Typography {
                size: 40.0,
                ..large
            }
            .clamped()
            .size,
            18.0
        );
    }

    #[test]
    fn widths_are_estimated_per_character() {
        let typography = Typography::DEFAULT;

        assert!((mono_width("CIZGI", 10.0) - 30.0).abs() < 1e-3);
        assert!(typography.text_width("Aa", 13.0, true) > typography.text_width("Aa", 13.0, false));

        // Aynı metin Inter'de IBM Plex Sans'tan geniştir.
        let inter = Typography {
            family: Family::Inter,
            ..typography
        };
        assert!(
            inter.text_width("Katman", 13.0, false) > typography.text_width("Katman", 13.0, false)
        );
    }

    #[cfg(feature = "fonts")]
    #[test]
    fn embedded_fonts_carry_the_family_names() {
        let names = [
            Family::IbmPlexSans.name(),
            Family::Inter.name(),
            Family::PlusJakartaSans.name(),
            Mono::IbmPlexMono.name(),
            Mono::JetBrainsMono.name(),
        ];

        for (index, font) in FONTS.iter().enumerate() {
            let name = names[index / 2];
            let needle = name
                .encode_utf16()
                .flat_map(u16::to_be_bytes)
                .collect::<Vec<_>>();

            assert!(
                font.windows(needle.len()).any(|window| window == needle),
                "{name} adı {index}. dosyada yok"
            );
        }
    }
}
