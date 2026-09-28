//! Biçim: köşe yarıçapları ve gölgeler.
//!
//! Kademeler web'inkidir (DESIGN.md: `--r-xs` 2, `--r-sm` 4, `--r-md` 6,
//! `--r-lg` 10 piksel; `--shadow-float`, `--shadow-pop`):
//!
//! | Kademe | Nerede |
//! |---|---|
//! | [`xs`] | etiketler, renk örnekleri, küçük işaretler |
//! | [`sm`] | düğmeler, giriş alanları, menü satırları |
//! | [`md`] | menüler, açılır paneller, ipuçları, kartlar |
//! | [`lg`] | pencereler, büyük kartlar |
//!
//! Görünüm → Köşeler ([`Corners`]) bütün kademeleri birlikte değiştirir;
//! Görünüm → Gölgeler ([`Shadows`]) yalnız yüzen katmanların gölgesini açar
//! ya da kapatır: yerleşik paneller her zaman düzdür. İkisi yazı ayarı gibi
//! bütün arayüzündür; uygulama onları yalnız değişince kurar ([`set`]).
//! Hap ve daire biçimleri (anahtarın tutamacı, nokta) köşe ayarını izlemez.
//!
//! ```ignore
//! container(menu).style(|theme| container::Style {
//!     border: Border { radius: shape::md().into(), ..border },
//!     shadow: shape::shadow(Level::Pop, &Tokens::of(theme)),
//!     ..
//! })
//! ```

use std::sync::atomic::{AtomicU8, Ordering};

use iced::{Color, Shadow, Vector};

use super::Tokens;

/// Köşelerin biçimi (`appearance.corners`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Corners {
    /// Klasik CAD programları gibi neredeyse keskin.
    Sharp,
    /// Web'in ölçüleri (varsayılan).
    #[default]
    Soft,
    /// Daha yuvarlak.
    Round,
}

impl Corners {
    pub const ALL: [Corners; 3] = [Corners::Sharp, Corners::Soft, Corners::Round];

    /// Ayar dosyasındaki adı.
    pub const fn key(self) -> &'static str {
        match self {
            Corners::Sharp => "sharp",
            Corners::Soft => "soft",
            Corners::Round => "round",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Corners::Sharp => "Keskin",
            Corners::Soft => "Yumuşak",
            Corners::Round => "Yuvarlak",
        }
    }

    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|c| c.key() == key)
    }
}

/// Yüzen katmanların gölgesi (`appearance.shadows`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Shadows {
    /// Gölge yok: katmanlar yalnız kenarlarıyla ayrılır.
    Off,
    /// Hafif gölge (varsayılan).
    #[default]
    Soft,
    /// Belirgin gölge.
    Strong,
}

impl Shadows {
    pub const ALL: [Shadows; 3] = [Shadows::Off, Shadows::Soft, Shadows::Strong];

    /// Ayar dosyasındaki adı.
    pub const fn key(self) -> &'static str {
        match self {
            Shadows::Off => "off",
            Shadows::Soft => "soft",
            Shadows::Strong => "strong",
        }
    }

    pub const fn name(self) -> &'static str {
        match self {
            Shadows::Off => "Kapalı",
            Shadows::Soft => "Hafif",
            Shadows::Strong => "Belirgin",
        }
    }

    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|s| s.key() == key)
    }
}

/// Arayüzün biçimi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Shape {
    pub corners: Corners,
    pub shadows: Shadows,
}

/// Gölgenin kademesi: katman ne kadar yüksekte duruyor.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Level {
    /// İpuçları, küçük açılır kutular (`--shadow-float`).
    Float,
    /// Menüler ve açılır listeler (`--shadow-pop`).
    Pop,
    /// Pencereler.
    Window,
}

static CORNERS: AtomicU8 = AtomicU8::new(1);
static SHADOWS: AtomicU8 = AtomicU8::new(1);

/// Biçimi değiştirir; sonraki çizimde bütün bileşenler yeni biçimle çizilir.
pub fn set(shape: Shape) {
    let index = |i: Option<usize>| i.unwrap_or(1) as u8;
    CORNERS.store(
        index(Corners::ALL.iter().position(|c| *c == shape.corners)),
        Ordering::Relaxed,
    );
    SHADOWS.store(
        index(Shadows::ALL.iter().position(|s| *s == shape.shadows)),
        Ordering::Relaxed,
    );
}

/// Geçerli biçim.
pub fn current() -> Shape {
    Shape {
        corners: Corners::ALL[usize::from(CORNERS.load(Ordering::Relaxed)) % Corners::ALL.len()],
        shadows: Shadows::ALL[usize::from(SHADOWS.load(Ordering::Relaxed)) % Shadows::ALL.len()],
    }
}

/// Yumuşak köşeler için tasarlanmış bir yarıçap, seçili köşelerde: keskinde
/// dörtte birine iner (en çok 2 piksel), yuvarlakta 1,6 katına çıkar.
pub fn radius(soft: f32) -> f32 {
    match current().corners {
        Corners::Sharp => (soft * 0.25).min(2.0),
        Corners::Soft => soft,
        Corners::Round => soft * 1.6,
    }
}

/// Etiketler, renk örnekleri, küçük işaretler.
pub fn xs() -> f32 {
    radius(2.0)
}

/// Düğmeler, giriş alanları, menü satırları.
pub fn sm() -> f32 {
    radius(4.0)
}

/// Menüler, açılır paneller, ipuçları, kartlar.
pub fn md() -> f32 {
    radius(6.0)
}

/// Pencereler, büyük kartlar.
pub fn lg() -> f32 {
    radius(10.0)
}

/// Kademenin gölgesi; gölgeler kapalıyken yoktur. Koyu temada gölge daha
/// yoğundur: koyu zeminde az görünür (DESIGN.md).
pub fn shadow(level: Level, tokens: &Tokens) -> Shadow {
    let (y, blur, alpha) = match level {
        Level::Float => (4.0, 14.0, 0.34),
        Level::Pop => (8.0, 24.0, 0.42),
        Level::Window => (14.0, 40.0, 0.5),
    };
    let (y, blur, alpha) = match current().shadows {
        Shadows::Off => return Shadow::default(),
        Shadows::Soft => (y, blur, alpha),
        Shadows::Strong => (y * 1.4, blur * 1.3, (alpha * 1.4_f32).min(0.75)),
    };
    // Light themes carry about a third of the dark ones' darkness.
    let alpha = if tokens.is_dark { alpha } else { alpha * 0.38 };
    Shadow {
        color: Color::from_rgba(0.0, 0.0, 0.0, alpha),
        offset: Vector::new(0.0, y),
        blur_radius: blur,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The shape is global: the tests that set it hold this.
    static SHAPE: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn the_levels_follow_the_corners_and_the_web_sizes_are_the_soft_ones() {
        let _held = SHAPE.lock();
        set(Shape::default());
        assert_eq!((xs(), sm(), md(), lg()), (2.0, 4.0, 6.0, 10.0));
        set(Shape {
            corners: Corners::Sharp,
            ..Shape::default()
        });
        assert!(lg() <= 2.0 && sm() <= 1.0, "nearly square");
        set(Shape {
            corners: Corners::Round,
            ..Shape::default()
        });
        assert!(md() > 9.0 && lg() > 15.0);
        set(Shape::default());
    }

    #[test]
    fn shadows_rise_with_the_level_and_go_when_turned_off() {
        let _held = SHAPE.lock();
        let dark = Tokens::DARK;
        set(Shape::default());
        let float = shadow(Level::Float, &dark);
        let pop = shadow(Level::Pop, &dark);
        let window = shadow(Level::Window, &dark);
        assert!(float.blur_radius < pop.blur_radius && pop.blur_radius < window.blur_radius);
        assert!(
            shadow(Level::Pop, &Tokens::LIGHT).color.a < pop.color.a,
            "lighter on a light theme"
        );
        set(Shape {
            shadows: Shadows::Off,
            ..Shape::default()
        });
        assert_eq!(shadow(Level::Pop, &dark), Shadow::default());
        set(Shape::default());
    }

    #[test]
    fn keys_read_back() {
        for c in Corners::ALL {
            assert_eq!(Corners::parse(c.key()), Some(c));
        }
        for s in Shadows::ALL {
            assert_eq!(Shadows::parse(s.key()), Some(s));
        }
    }
}
