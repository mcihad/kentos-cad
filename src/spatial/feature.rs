//! Vektör veri modeli: geometri, öğe (feature), katman ve alt katman.

use iced::Color;

use super::measure;
use super::{Bounds, LonLat};
use crate::attribute::{Field, ObjectId, Value};

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

/// Tek bir coğrafi öğe: kalıcı numarası, geometrisi ve katmanın şemasına
/// göre sıralanmış öznitelik değerleri.
#[derive(Debug, Clone, PartialEq)]
pub struct Feature {
    /// Katman içindeki kalıcı numara; [`Layer::insert`] atar.
    pub id: ObjectId,
    pub geometry: Geometry,
    /// Katman şemasındaki alanlarla aynı sırada değerler.
    pub values: Vec<Value>,
}

impl Feature {
    pub fn new(geometry: Geometry) -> Self {
        Self {
            id: ObjectId::default(),
            geometry,
            values: Vec::new(),
        }
    }

    pub fn with_values(mut self, values: impl IntoIterator<Item = Value>) -> Self {
        self.values = values.into_iter().collect();
        self
    }

    /// Alanın değeri; alan yoksa boş.
    pub fn value(&self, field: usize) -> &Value {
        static NULL: Value = Value::Null;

        self.values.get(field).unwrap_or(&NULL)
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

/// Alt katman: katman öğelerinin bir alanın değerine göre ayrılmış bir
/// kısmı; kendi rengi ve görünürlüğü vardır. ArcGIS'teki alt tür grup
/// katmanı gibi: yolları türüne, şehirleri bölgesine göre ayırıp ayrı
/// renklendirmek ve açıp kapatmak için.
#[derive(Debug, Clone, PartialEq)]
pub struct Sublayer {
    /// Gösterilen ad; verilmezse değerin kendisi.
    pub name: String,
    /// Öğelerin ayrıldığı alanın bu alt katmana düşen değeri.
    pub value: Value,
    pub color: Color,
    pub visible: bool,
}

impl Sublayer {
    /// Metin değerli alt katman; adı değerin kendisidir.
    pub fn new(value: impl Into<String>, color: Color) -> Self {
        let name = value.into();

        Self {
            value: Value::Text(name.clone()),
            name,
            color,
            visible: true,
        }
    }

    /// Herhangi bir türde değeri olan alt katman.
    pub fn with_value(name: impl Into<String>, value: impl Into<Value>, color: Color) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            color,
            visible: true,
        }
    }
}

/// Vektör katmanı: aynı şemayı paylaşan, aynı biçimde çizilen öğeler.
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
    /// Öznitelik alanları.
    pub schema: Vec<Field>,
    /// Öğenin adı olarak gösterilen alan.
    pub label_field: Option<usize>,
    /// Öğelerin alt katmanlara ayrıldığı alan.
    pub sublayer_field: Option<usize>,
    /// Alt katmanlar; hiçbirine uymayan öğeler katmanın rengiyle çizilir.
    pub sublayers: Vec<Sublayer>,
    /// Numara sırasıyla öğeler.
    pub features: Vec<Feature>,
    last_id: u64,
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
            schema: Vec::new(),
            label_field: None,
            sublayer_field: None,
            sublayers: Vec::new(),
            features: Vec::new(),
            last_id: 0,
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

    /// Öznitelik alanları; ilk alan öğenin adı olarak kullanılır.
    pub fn with_schema(mut self, schema: impl IntoIterator<Item = Field>) -> Self {
        self.schema = schema.into_iter().collect();
        self.label_field = (!self.schema.is_empty()).then_some(0);
        self
    }

    /// Öğeleri ekler ve numaralandırır.
    pub fn with_features(mut self, features: impl IntoIterator<Item = Feature>) -> Self {
        for feature in features {
            self.insert(feature);
        }

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

    /// Öğeleri adı verilen alanın değerine göre alt katmanlara ayırır. Şemada
    /// böyle bir alan yoksa katman ayrılmaz.
    pub fn with_sublayers(
        mut self,
        field: &str,
        sublayers: impl IntoIterator<Item = Sublayer>,
    ) -> Self {
        self.sublayer_field = self.field_index(field);
        self.sublayers = if self.sublayer_field.is_some() {
            sublayers.into_iter().collect()
        } else {
            Vec::new()
        };
        self
    }

    /// Öğenin alt katmanı.
    pub fn sublayer_of(&self, feature: &Feature) -> Option<usize> {
        let value = feature.value(self.sublayer_field?);

        self.sublayers
            .iter()
            .position(|sublayer| sublayer.value == *value)
    }

    /// Öğe çizilir ve seçilebilir mi: katman görünür ve öğenin alt katmanı
    /// (varsa) açık.
    pub fn shows(&self, feature: &Feature) -> bool {
        self.visible
            && self
                .sublayer_of(feature)
                .is_none_or(|sublayer| self.sublayers[sublayer].visible)
    }

    /// Öğenin rengi: alt katmanının rengi, yoksa katmanın rengi.
    pub fn color_of(&self, feature: &Feature) -> Color {
        self.sublayer_of(feature)
            .map_or(self.color, |sublayer| self.sublayers[sublayer].color)
    }

    /// Alt katmandaki öğeler.
    pub fn sublayer_features(&self, sublayer: usize) -> impl Iterator<Item = &Feature> + '_ {
        self.features
            .iter()
            .filter(move |feature| self.sublayer_of(feature) == Some(sublayer))
    }

    /// Öğeyi sona ekler ve yeni bir numara verir. Değerler şemadaki alan
    /// sayısına boş değerlerle tamamlanır.
    pub fn insert(&mut self, mut feature: Feature) -> ObjectId {
        self.last_id += 1;
        feature.id = ObjectId(self.last_id);
        feature
            .values
            .resize(self.schema.len().max(feature.values.len()), Value::Null);

        let id = feature.id;
        self.features.push(feature);
        id
    }

    fn position(&self, id: ObjectId) -> Option<usize> {
        self.features
            .binary_search_by_key(&id, |feature| feature.id)
            .ok()
    }

    pub fn feature(&self, id: ObjectId) -> Option<&Feature> {
        self.position(id).map(|index| &self.features[index])
    }

    pub fn feature_mut(&mut self, id: ObjectId) -> Option<&mut Feature> {
        self.position(id)
            .map(move |index| &mut self.features[index])
    }

    /// Öğeyi siler; diğer öğelerin numaraları değişmez.
    pub fn remove(&mut self, id: ObjectId) -> Option<Feature> {
        self.position(id).map(|index| self.features.remove(index))
    }

    /// Adı verilen alanın numarası.
    pub fn field_index(&self, name: &str) -> Option<usize> {
        self.schema.iter().position(|field| field.name == name)
    }

    /// Öğenin adı: ad alanının değeri, yoksa "#numara".
    pub fn label(&self, feature: &Feature) -> String {
        self.label_field
            .and_then(|index| {
                let text = self.schema.get(index)?.format(feature.value(index));
                (!text.is_empty()).then_some(text)
            })
            .unwrap_or_else(|| format!("#{}", feature.id))
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

/// Katman listesindeki bir öğenin kalıcı adresi: katman sırası ve öğe
/// numarası.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct FeatureRef {
    pub layer: usize,
    pub id: ObjectId,
}

impl FeatureRef {
    pub const fn new(layer: usize, id: ObjectId) -> Self {
        Self { layer, id }
    }

    /// Katman listesindeki öğe; adres geçersizse `None`.
    pub fn resolve(self, layers: &[Layer]) -> Option<(&Layer, &Feature)> {
        let layer = layers.get(self.layer)?;

        layer.feature(self.id).map(|feature| (layer, feature))
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sublayers_split_features_by_value() {
        let red = Color::from_rgb(1.0, 0.0, 0.0);
        let blue = Color::from_rgb(0.0, 0.0, 1.0);

        let mut layer = Layer::lines("Yollar", Color::BLACK)
            .with_schema([Field::text("Ad"), Field::text("Tür")])
            .with_features([("A", "Otoyol"), ("B", "Bulvar"), ("C", "Patika")].map(
                |(name, kind)| {
                    Feature::new(Geometry::Line(vec![
                        LonLat::new(0.0, 0.0),
                        LonLat::new(1.0, 1.0),
                    ]))
                    .with_values([Value::from(name), Value::from(kind)])
                },
            ))
            .with_sublayers(
                "Tür",
                [Sublayer::new("Otoyol", red), Sublayer::new("Bulvar", blue)],
            );

        let [highway, boulevard, path] =
            [1, 2, 3].map(|id| layer.feature(ObjectId(id)).cloned().expect("öğe"));

        assert_eq!(layer.sublayer_of(&highway), Some(0));
        assert_eq!(layer.color_of(&boulevard), blue);
        // Hiçbir alt katmana uymayan öğe katmanın rengini alır.
        assert_eq!(layer.sublayer_of(&path), None);
        assert_eq!(layer.color_of(&path), Color::BLACK);

        layer.sublayers[0].visible = false;
        assert!(!layer.shows(&highway));
        assert!(layer.shows(&boulevard) && layer.shows(&path));
        assert_eq!(layer.sublayer_features(1).count(), 1);

        layer.visible = false;
        assert!(!layer.shows(&boulevard));
    }

    #[test]
    fn missing_sublayer_field_leaves_the_layer_whole() {
        let layer = Layer::points("Noktalar", Color::BLACK)
            .with_schema([Field::text("Ad")])
            .with_sublayers("Tür", [Sublayer::new("A", Color::WHITE)]);

        assert_eq!(layer.sublayer_field, None);
        assert!(layer.sublayers.is_empty());
    }

    #[test]
    fn features_keep_their_numbers_after_removal() {
        let mut layer = Layer::points("Test", Color::BLACK)
            .with_schema([Field::text("Ad")])
            .with_features(["A", "B", "C"].map(|name| {
                Feature::new(Geometry::Point(LonLat::new(0.0, 0.0)))
                    .with_values([Value::from(name)])
            }));

        assert!(layer.remove(ObjectId(2)).is_some());
        assert_eq!(
            layer
                .features
                .iter()
                .map(|feature| feature.id)
                .collect::<Vec<_>>(),
            [ObjectId(1), ObjectId(3)]
        );
        assert_eq!(
            layer
                .feature(ObjectId(3))
                .map(|feature| layer.label(feature)),
            Some("C".to_owned())
        );

        let next = layer.insert(Feature::new(Geometry::Point(LonLat::new(1.0, 1.0))));
        assert_eq!(next, ObjectId(4));
        assert_eq!(
            layer.feature(next).map(|feature| feature.values.len()),
            Some(1)
        );
        assert_eq!(
            layer.feature(next).map(|feature| layer.label(feature)),
            Some("#4".to_owned())
        );
    }
}
