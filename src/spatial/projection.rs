//! WGS 84 koordinatları ve Web Mercator görünümü.
//!
//! Veri WGS 84 (boylam/enlem) olarak tutulur; çizim anında EPSG:3857 (Web
//! Mercator) düzlemine, oradan da ekran koordinatlarına dönüştürülür.

use std::f64::consts::PI;

use iced::{Point, Size, Vector};

use super::format;

/// Tek bir karo (tile) genişliği; 0. yakınlaştırmada dünyanın genişliği.
pub const TILE_SIZE: f64 = 256.0;
pub const MIN_ZOOM: f64 = 2.0;
pub const MAX_ZOOM: f64 = 18.0;

const MAX_MERCATOR_LAT: f64 = 85.051_128_779_806_59;

/// WGS 84 elipsoidinin büyük yarı ekseni; Web Mercator küre yarıçapı (metre).
const EARTH_RADIUS: f64 = 6_378_137.0;
/// Ekvator çevresi (metre).
const EARTH_CIRCUMFERENCE: f64 = 2.0 * PI * EARTH_RADIUS;
/// 96 DPI ekranda bir metredeki piksel sayısı.
const PIXELS_PER_METER: f64 = 3_779.527_5;

/// WGS 84 koordinatı (derece).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LonLat {
    pub lon: f64,
    pub lat: f64,
}

impl LonLat {
    pub const fn new(lon: f64, lat: f64) -> Self {
        Self { lon, lat }
    }

    /// Normalize edilmiş Mercator koordinatı (0..1 aralığında).
    fn mercator(self) -> (f64, f64) {
        let x = (self.lon + 180.0) / 360.0;
        let latitude = self
            .lat
            .clamp(-MAX_MERCATOR_LAT, MAX_MERCATOR_LAT)
            .to_radians();
        let y = (1.0 - (latitude.tan() + 1.0 / latitude.cos()).ln() / PI) / 2.0;

        (x, y)
    }

    /// Web Mercator (EPSG:3857) düzlem koordinatı, metre: doğuya X,
    /// kuzeye Y.
    pub fn web_mercator(self) -> (f64, f64) {
        let latitude = self
            .lat
            .clamp(-MAX_MERCATOR_LAT, MAX_MERCATOR_LAT)
            .to_radians();

        (
            EARTH_RADIUS * self.lon.to_radians(),
            EARTH_RADIUS * (PI / 4.0 + latitude / 2.0).tan().ln(),
        )
    }

    /// Normalize edilmiş Mercator koordinatından WGS 84'e dönüş.
    fn from_mercator(x: f64, y: f64) -> Self {
        let lon = x * 360.0 - 180.0;
        let lat = (PI * (1.0 - 2.0 * y)).sinh().atan().to_degrees();

        Self { lon, lat }
    }
}

/// Coğrafi sınır kutusu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub south_west: LonLat,
    pub north_east: LonLat,
}

impl Bounds {
    /// Noktaları kapsayan en küçük kutu; nokta yoksa `None`.
    pub fn from_points(points: impl IntoIterator<Item = LonLat>) -> Option<Self> {
        points
            .into_iter()
            .fold(None, |bounds: Option<Bounds>, point| {
                let single = Bounds {
                    south_west: point,
                    north_east: point,
                };

                Some(bounds.map_or(single, |bounds| bounds.union(single)))
            })
    }

    /// İki kutuyu kapsayan kutu.
    pub fn union(self, other: Self) -> Self {
        Self {
            south_west: LonLat::new(
                self.south_west.lon.min(other.south_west.lon),
                self.south_west.lat.min(other.south_west.lat),
            ),
            north_east: LonLat::new(
                self.north_east.lon.max(other.north_east.lon),
                self.north_east.lat.max(other.north_east.lat),
            ),
        }
    }

    pub fn center(self) -> LonLat {
        LonLat::new(
            (self.south_west.lon + self.north_east.lon) / 2.0,
            (self.south_west.lat + self.north_east.lat) / 2.0,
        )
    }

    /// Kutunun uzun kenarı (derece).
    pub fn span(self) -> f64 {
        (self.north_east.lon - self.south_west.lon)
            .abs()
            .max((self.north_east.lat - self.south_west.lat).abs())
    }
}

/// Haritanın o anki bakış penceresi: merkez, yakınlaştırma ve piksel boyutu.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Viewport {
    pub center: LonLat,
    pub zoom: f64,
    pub size: Size,
}

impl Viewport {
    pub fn new(center: LonLat, zoom: f64, size: Size) -> Self {
        Self { center, zoom, size }
    }

    /// Tüm dünyanın bu yakınlaştırma seviyesindeki piksel genişliği.
    pub fn world_pixels(&self) -> f64 {
        TILE_SIZE * 2f64.powf(self.zoom)
    }

    /// Görünümün ortasındaki ekran noktası.
    pub fn center_point(&self) -> Point {
        Point::new(self.size.width / 2.0, self.size.height / 2.0)
    }

    /// Coğrafi koordinatı ekran noktasına dönüştürür.
    pub fn project(&self, location: LonLat) -> Point {
        let scale = self.world_pixels();
        let (center_x, center_y) = self.center.mercator();
        let (x, y) = location.mercator();

        Point::new(
            ((x - center_x) * scale + f64::from(self.size.width) / 2.0) as f32,
            ((y - center_y) * scale + f64::from(self.size.height) / 2.0) as f32,
        )
    }

    /// Ekran noktasını coğrafi koordinata dönüştürür.
    pub fn unproject(&self, point: Point) -> LonLat {
        let scale = self.world_pixels();
        let (center_x, center_y) = self.center.mercator();

        LonLat::from_mercator(
            center_x + (f64::from(point.x) - f64::from(self.size.width) / 2.0) / scale,
            center_y + (f64::from(point.y) - f64::from(self.size.height) / 2.0) / scale,
        )
    }

    /// Haritayı ekran pikseli cinsinden kaydırır.
    pub fn pan_by(&mut self, delta: Vector) {
        let scale = self.world_pixels();
        let (center_x, center_y) = self.center.mercator();

        self.center = LonLat::from_mercator(
            center_x - f64::from(delta.x) / scale,
            center_y - f64::from(delta.y) / scale,
        );
    }

    /// `anchor` ekran noktasının altındaki coğrafi noktayı sabit tutarak
    /// yakınlaştırır. `delta` yakınlaştırma seviyesi cinsindendir.
    pub fn zoom_by(&mut self, delta: f64, anchor: Point) {
        let before = self.unproject(anchor);

        self.zoom = (self.zoom + delta).clamp(MIN_ZOOM, MAX_ZOOM);

        let after = self.unproject(anchor);
        let (before_x, before_y) = before.mercator();
        let (after_x, after_y) = after.mercator();
        let (center_x, center_y) = self.center.mercator();

        self.center = LonLat::from_mercator(
            center_x + (before_x - after_x),
            center_y + (before_y - after_y),
        );
    }

    /// Kutuyu, kenarlarda `padding` piksel boşluk bırakarak görünüme sığdırır.
    pub fn fit_bounds(&mut self, bounds: Bounds, padding: f32) {
        let (west, south) = bounds.south_west.mercator();
        let (east, north) = bounds.north_east.mercator();

        let width = (f64::from(self.size.width) - 2.0 * f64::from(padding)).max(1.0);
        let height = (f64::from(self.size.height) - 2.0 * f64::from(padding)).max(1.0);

        let span_x = (east - west).abs().max(f64::EPSILON) * TILE_SIZE;
        let span_y = (north - south).abs().max(f64::EPSILON) * TILE_SIZE;

        let zoom_x = (width / span_x).log2();
        let zoom_y = (height / span_y).log2();

        self.zoom = zoom_x.min(zoom_y).clamp(MIN_ZOOM, MAX_ZOOM);
        self.center = LonLat::from_mercator((west + east) / 2.0, (south + north) / 2.0);
    }

    /// Öğeye odaklanır: küçük bir öğeyi (nokta gibi) en az `min_zoom`
    /// seviyesinde ortalar, büyük öğeyi kenar boşluğuyla sığdırır.
    pub fn focus(&mut self, bounds: Bounds, padding: f32, min_zoom: f64) {
        if bounds.span() < 0.01 {
            self.center = bounds.center();
            self.zoom = self.zoom.max(min_zoom);
        } else {
            self.fit_bounds(bounds, padding);
        }
    }

    /// Görünen alanın sınırları.
    pub fn visible_bounds(&self) -> Bounds {
        Bounds {
            south_west: self.unproject(Point::new(0.0, self.size.height)),
            north_east: self.unproject(Point::new(self.size.width, 0.0)),
        }
    }

    /// Merkez enlemindeki metre/piksel değeri.
    pub fn meters_per_pixel(&self) -> f64 {
        EARTH_CIRCUMFERENCE * self.center.lat.to_radians().cos().abs() / self.world_pixels()
    }

    /// Ekran ölçeğinin paydası (1:N), 96 DPI varsayımıyla.
    pub fn scale_denominator(&self) -> f64 {
        self.meters_per_pixel() * PIXELS_PER_METER
    }

    /// Ölçeği 1:`denominator` yapacak yakınlaştırma seviyesi; sınırlara
    /// kırpılmaz. Sonuç [`MIN_ZOOM`] ile [`MAX_ZOOM`] arasında değilse bu
    /// enlemde o ölçeğe gelinemez.
    pub fn zoom_for_scale(&self, denominator: f64) -> f64 {
        let ground = EARTH_CIRCUMFERENCE * self.center.lat.to_radians().cos().abs();

        (ground * PIXELS_PER_METER / (TILE_SIZE * denominator)).log2()
    }

    /// Merkezi koruyarak ölçeği 1:`denominator` yapar; yakınlaştırma
    /// sınırlarına kırpılır.
    pub fn set_scale(&mut self, denominator: f64) {
        self.zoom = self.zoom_for_scale(denominator).clamp(MIN_ZOOM, MAX_ZOOM);
    }

    /// Izgara (graticule) çizgileri için derece adımı; çizgiler arası en az
    /// 70 piksel olacak şekilde seçilir.
    pub fn graticule_step(&self) -> f64 {
        const STEPS: [f64; 14] = [
            30.0, 20.0, 10.0, 5.0, 2.0, 1.0, 0.5, 0.25, 0.1, 0.05, 0.02, 0.01, 0.005, 0.002,
        ];

        let scale = self.world_pixels();

        STEPS
            .into_iter()
            .find(|step| step / 360.0 * scale >= 70.0)
            .unwrap_or(0.002)
    }

    /// Ölçek çubuğu için "yuvarlak" bir mesafe ve etiket döndürür.
    pub fn scale_bar(&self) -> (f64, String) {
        let meters_per_pixel = self.meters_per_pixel();
        let raw = 120.0 * meters_per_pixel;
        let power = 10f64.powf(raw.log10().floor());
        let normalized = raw / power;

        let factor = if normalized < 1.5 {
            1.0
        } else if normalized < 3.5 {
            2.0
        } else if normalized < 7.5 {
            5.0
        } else {
            10.0
        };

        let meters = factor * power;
        let label = if meters >= 1_000.0 {
            format!("{} km", format::pretty(round_to(meters / 1_000.0, 1)))
        } else {
            format!("{} m", format::pretty(meters))
        };

        (meters, label)
    }
}

fn round_to(value: f64, digits: i32) -> f64 {
    let factor = 10f64.powi(digits);
    (value * factor).round() / factor
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn projection_round_trip() {
        let viewport = Viewport::new(LonLat::new(29.0, 41.0), 9.0, Size::new(1000.0, 700.0));
        let location = LonLat::new(28.9784, 41.0082);

        let unprojected = viewport.unproject(viewport.project(location));

        assert!((unprojected.lon - location.lon).abs() < 1e-6);
        assert!((unprojected.lat - location.lat).abs() < 1e-6);
    }

    #[test]
    fn zoom_keeps_anchor_fixed() {
        let mut viewport = Viewport::new(LonLat::new(29.0, 41.0), 9.0, Size::new(1000.0, 700.0));
        let anchor = Point::new(810.0, 140.0);
        let before = viewport.unproject(anchor);

        viewport.zoom_by(2.5, anchor);

        let after = viewport.unproject(anchor);

        assert!((after.lon - before.lon).abs() < 1e-6);
        assert!((after.lat - before.lat).abs() < 1e-6);
    }

    #[test]
    fn fit_bounds_contains_bounds() {
        let mut viewport = Viewport::new(LonLat::new(0.0, 0.0), 4.0, Size::new(800.0, 600.0));
        let bounds = Bounds {
            south_west: LonLat::new(26.0, 36.0),
            north_east: LonLat::new(45.0, 42.0),
        };

        viewport.fit_bounds(bounds, 40.0);

        let south_west = viewport.project(bounds.south_west);
        let north_east = viewport.project(bounds.north_east);

        assert!(south_west.x >= 0.0 && south_west.y <= 600.0);
        assert!(north_east.x <= 800.0 && north_east.y >= 0.0);
    }

    #[test]
    fn bounds_cover_all_points() {
        let bounds = Bounds::from_points([
            LonLat::new(30.0, 40.0),
            LonLat::new(28.0, 41.0),
            LonLat::new(29.0, 39.0),
        ])
        .expect("üç nokta");

        assert_eq!(bounds.south_west, LonLat::new(28.0, 39.0));
        assert_eq!(bounds.north_east, LonLat::new(30.0, 41.0));
        assert_eq!(bounds.center(), LonLat::new(29.0, 40.0));
        assert!(Bounds::from_points([]).is_none());
    }

    #[test]
    fn web_mercator_meters() {
        let (x, y) = LonLat::new(0.0, 0.0).web_mercator();
        assert!(x.abs() < 1e-6 && y.abs() < 1e-6);

        let (x, y) = LonLat::new(180.0, MAX_MERCATOR_LAT).web_mercator();
        assert!((x - 20_037_508.34).abs() < 0.01, "{x}");
        assert!((y - 20_037_508.34).abs() < 0.01, "{y}");

        // İstanbul, EPSG:3857 (pyproj ile doğrulanmış değerler).
        let (x, y) = LonLat::new(28.9784, 41.0082).web_mercator();
        assert!((x - 3_225_860.73).abs() < 0.01, "{x}");
        assert!((y - 5_013_551.24).abs() < 0.01, "{y}");
    }

    #[test]
    fn scale_can_be_set() {
        let mut viewport = Viewport::new(LonLat::new(32.0, 39.0), 6.0, Size::new(900.0, 600.0));

        viewport.set_scale(25_000.0);
        assert!((viewport.scale_denominator() - 25_000.0).abs() < 0.5);

        // Bu enlemde 1:100 ölçeğe gelinemez; en yakın seviyede kalır.
        assert!(viewport.zoom_for_scale(100.0) > MAX_ZOOM);
        viewport.set_scale(100.0);
        assert_eq!(viewport.zoom, MAX_ZOOM);
    }

    #[test]
    fn scale_bar_labels_are_round() {
        let viewport = Viewport::new(LonLat::new(29.0, 41.0), 8.0, Size::new(900.0, 600.0));
        let (meters, label) = viewport.scale_bar();

        assert!(meters > 0.0);
        assert!(label.ends_with("km") || label.ends_with('m'));
    }
}
