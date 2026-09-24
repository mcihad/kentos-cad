//! Vektör veri modeli: geometri, öğe (feature) ve katman.

use iced::Color;

use super::measure;
use super::{Bounds, LonLat};

/// Öğe geometrisi.
#[derive(Debug, Clone, PartialEq)]
pub enum Geometry {
    Point(LonLat),
    /// Açık çizgi: iki köşeli doğru parçası ya da çoklu çizgi.
    Line(Vec<LonLat>),
    /// Kapalı alan; ilk köşe sonda tekrarlanmaz.
    Polygon(Vec<LonLat>),
}

impl Geometry {
    /// Geometrinin köşeleri.
    pub fn vertices(&self) -> &[LonLat] {
        match self {
            Geometry::Point(location) => std::slice::from_ref(location),
            Geometry::Line(points) | Geometry::Polygon(points) => points,
        }
    }

    pub fn bounds(&self) -> Option<Bounds> {
        Bounds::from_points(self.vertices().iter().copied())
    }

    /// Geometri türünün okunur adı.
    pub fn label(&self) -> &'static str {
        match self {
            Geometry::Point(_) => "Nokta",
            Geometry::Line(points) if points.len() == 2 => "Çizgi",
            Geometry::Line(_) => "Çoklu çizgi",
            Geometry::Polygon(_) => "Alan",
        }
    }

    /// Çizginin uzunluğu ya da alanın çevresi (metre); noktada sıfır.
    pub fn length_meters(&self) -> f64 {
        match self {
            Geometry::Point(_) => 0.0,
            Geometry::Line(points) => measure::polyline_length_meters(points),
            Geometry::Polygon(points) => measure::ring_perimeter_meters(points),
        }
    }
}

/// Adı, geometrisi ve öznitelikleri olan tek bir coğrafi öğe.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    pub name: String,
    pub geometry: Geometry,
    /// Sırası korunan anahtar-değer öznitelikleri.
    pub properties: Vec<(String, String)>,
}

impl Feature {
    pub fn new(name: impl Into<String>, geometry: Geometry) -> Self {
        Self {
            name: name.into(),
            geometry,
            properties: Vec::new(),
        }
    }

    /// Öznitelik ekler.
    pub fn with(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.properties.push((key.into(), value.into()));
        self
    }

    /// Özniteliğin değeri.
    pub fn property(&self, key: &str) -> Option<&str> {
        self.properties
            .iter()
            .find(|(name, _)| name == key)
            .map(|(_, value)| value.as_str())
    }

    pub fn bounds(&self) -> Option<Bounds> {
        self.geometry.bounds()
    }
}

/// Katmanın geometri türü; çizim ve etiketleme davranışını belirler.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayerKind {
    Point,
    Line,
    Polygon,
}

impl LayerKind {
    pub fn label(self) -> &'static str {
        match self {
            LayerKind::Point => "Nokta",
            LayerKind::Line => "Çizgi",
            LayerKind::Polygon => "Alan",
        }
    }
}

/// Vektör katmanı: aynı biçimde çizilen öğeler.
#[derive(Debug, Clone, PartialEq)]
pub struct Layer {
    pub name: String,
    pub kind: LayerKind,
    pub visible: bool,
    /// 0..1 arası saydamlık.
    pub opacity: f32,
    pub color: Color,
    /// Alan dolgusunun saydamlığı.
    pub fill_alpha: f32,
    pub stroke_width: f32,
    /// Nokta etiketlerinin görünmeye başladığı yakınlaştırma seviyesi.
    pub label_zoom: Option<f64>,
    pub features: Vec<Feature>,
}

impl Layer {
    fn new(name: impl Into<String>, kind: LayerKind, color: Color) -> Self {
        Self {
            name: name.into(),
            kind,
            visible: true,
            opacity: 1.0,
            color,
            fill_alpha: 0.0,
            stroke_width: 1.5,
            label_zoom: None,
            features: Vec::new(),
        }
    }

    /// Nokta katmanı.
    pub fn points(name: impl Into<String>, color: Color) -> Self {
        Self {
            fill_alpha: 1.0,
            ..Self::new(name, LayerKind::Point, color)
        }
    }

    /// Çizgi katmanı.
    pub fn lines(name: impl Into<String>, color: Color) -> Self {
        Self {
            stroke_width: 1.8,
            ..Self::new(name, LayerKind::Line, color)
        }
    }

    /// Alan katmanı.
    pub fn polygons(name: impl Into<String>, color: Color) -> Self {
        Self {
            fill_alpha: 0.28,
            stroke_width: 1.3,
            ..Self::new(name, LayerKind::Polygon, color)
        }
    }

    pub fn with_features(mut self, features: impl IntoIterator<Item = Feature>) -> Self {
        self.features.extend(features);
        self
    }

    pub fn opacity(mut self, opacity: f32) -> Self {
        self.opacity = opacity;
        self
    }

    pub fn fill_alpha(mut self, fill_alpha: f32) -> Self {
        self.fill_alpha = fill_alpha;
        self
    }

    pub fn stroke_width(mut self, stroke_width: f32) -> Self {
        self.stroke_width = stroke_width;
        self
    }

    /// Nokta etiketlerini bu yakınlaştırmadan itibaren gösterir.
    pub fn labels_from(mut self, zoom: f64) -> Self {
        self.label_zoom = Some(zoom);
        self
    }

    /// Görünür ve seçilebilecek kadar opak mı.
    pub fn is_interactive(&self) -> bool {
        self.visible && self.opacity > 0.05
    }

    /// Katmanın bütün öğelerini kapsayan kutu.
    pub fn bounds(&self) -> Option<Bounds> {
        self.features
            .iter()
            .filter_map(Feature::bounds)
            .reduce(Bounds::union)
    }
}

/// Katman listesindeki bir öğenin yeri.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct FeatureRef {
    pub layer: usize,
    pub feature: usize,
}

impl FeatureRef {
    pub const fn new(layer: usize, feature: usize) -> Self {
        Self { layer, feature }
    }

    /// Katman listesindeki öğe; yer geçersizse `None`.
    pub fn resolve(self, layers: &[Layer]) -> Option<(&Layer, &Feature)> {
        let layer = layers.get(self.layer)?;

        layer
            .features
            .get(self.feature)
            .map(|feature| (layer, feature))
    }
}

/// Katman listesindeki görünür öğelerin tamamını kapsayan kutu.
pub fn visible_bounds(layers: &[Layer]) -> Option<Bounds> {
    layers
        .iter()
        .filter(|layer| layer.visible)
        .filter_map(Layer::bounds)
        .reduce(Bounds::union)
}
