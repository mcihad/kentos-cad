//! Öznitelik tablosunun görünüm modeli: sütunlar, filtre, arama ve sıralama.
//!
//! Arayüzden bağımsızdır; tablo görünümü de "tümünü seç" gibi eylemler de
//! aynı satırları buradan alır.

use std::cmp::Ordering;

use kentos_ui::attribute::{FieldKind, ObjectId, Query, Value, text};
use kentos_ui::spatial::{Feature, FeatureRef, Geometry, Layer, LayerKind, Selection, format};
use kentos_ui::theme::typography;
use kentos_ui::widget::table::SortOrder;

/// Tablonun sütunları: OBJECTID, şemadaki alanlar ve çizgi/alan
/// katmanlarında hesaplanan uzunluk ya da çevre.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Column {
    Id,
    Field(usize),
    Length,
}

impl Column {
    /// Katmanın sütunları, sırasıyla.
    pub fn all(layer: &Layer) -> Vec<Column> {
        let mut columns = vec![Column::Id];
        columns.extend((0..layer.schema.len()).map(Column::Field));

        if layer.kind != LayerKind::Point {
            columns.push(Column::Length);
        }

        columns
    }

    /// Sütun başlığı; birimli alanlarda birim de yazılır: "Hız sınırı
    /// (km/sa)".
    pub fn title(self, layer: &Layer) -> String {
        match self {
            Column::Id => "OBJECTID".to_owned(),
            Column::Field(index) => match layer.schema.get(index) {
                Some(field) => match &field.unit {
                    Some(unit) => format!("{} ({unit})", field.name),
                    None => field.name.clone(),
                },
                None => String::new(),
            },
            Column::Length if layer.kind == LayerKind::Polygon => "Çevre".to_owned(),
            Column::Length => "Uzunluk".to_owned(),
        }
    }

    /// Sütunun genişliği: başlığa ve en uzun hücreye göre, sınırlar içinde.
    /// ArcGIS'teki gibi tablo açılırken içeriğe sığdırılır. Genişlik 12
    /// piksellik gövde metnine göre, seçili yazı ailesinin harf genişliğiyle
    /// hesaplanır; tablo onu yazı boyutuna göre büyütür.
    pub fn width(self, layers: &[Layer], layer: &Layer) -> f32 {
        const BODY: f32 = 12.0;
        const CAPTION: f32 = 11.0;
        const SORT_ARROW: f32 = 14.0;
        const LINK_ICON: f32 = 18.0;

        let title = typography::text_width(&self.title(layer), CAPTION) + SORT_ARROW;

        let longest = layer
            .features
            .iter()
            .map(|feature| self.text(layers, layer, feature))
            .max_by_key(|text| text.chars().count())
            .unwrap_or_default();

        let cell = match self {
            Column::Field(index)
                if matches!(
                    layer.schema.get(index).map(|field| &field.kind),
                    Some(FieldKind::Object { .. })
                ) =>
            {
                typography::text_width(&longest, BODY) + LINK_ICON
            }
            _ if self.is_monospaced(layer) => typography::mono_width(&longest, BODY),
            _ => typography::text_width(&longest, BODY),
        };

        title.max(cell).clamp(56.0, 260.0).ceil()
    }

    /// Sayısal sütunlar sağa hizalanır.
    pub fn is_numeric(self, layer: &Layer) -> bool {
        match self {
            Column::Id | Column::Length => true,
            Column::Field(index) => layer
                .schema
                .get(index)
                .is_some_and(|field| field.is_numeric()),
        }
    }

    /// Sayı, tarih ve saat sütunları eş aralıklı yazılır.
    pub fn is_monospaced(self, layer: &Layer) -> bool {
        match self {
            Column::Id | Column::Length => true,
            Column::Field(index) => layer.schema.get(index).is_some_and(|field| {
                field.is_numeric()
                    || matches!(
                        field.kind,
                        FieldKind::Date | FieldKind::Time | FieldKind::DateTime
                    )
            }),
        }
    }

    /// Hücrede gösterilen metin.
    pub fn text(self, layers: &[Layer], layer: &Layer, feature: &Feature) -> String {
        match self {
            Column::Id => feature.id.to_string(),
            Column::Field(index) => cell_text(layers, layer, feature, index),
            Column::Length => length_text(feature),
        }
    }
}

/// Bir katmanın tablo ayarları. Her katmanın kendi ayarları vardır; aktif
/// katman değişince filtre ve sıralama kaybolmaz.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TableView {
    pub search: String,
    pub selected_only: bool,
    /// Tablo filtresi ("Tabloyu filtrele" penceresinde kurulan sorgu).
    pub filter: Query,
    /// Sıralanan sütun ve yönü.
    pub sort: Option<(Column, SortOrder)>,
}

impl TableView {
    /// Sütun başlığına tıklandı: aynı sütunda yönü çevirir, yeni sütunda
    /// artan sıraya geçer.
    pub fn sort_by(&mut self, column: Column) {
        self.sort = match self.sort {
            Some((current, order)) if current == column => Some((column, order.reversed())),
            _ => Some((column, SortOrder::Ascending)),
        };
    }

    /// Sütunun sıralama yönü; sıralı değilse `None`.
    pub fn order_of(&self, column: Column) -> Option<SortOrder> {
        self.sort
            .and_then(|(current, order)| (current == column).then_some(order))
    }

    /// Tabloda gösterilen satırlar: filtre, arama ve "yalnızca seçili"
    /// uygulanmış, sıralanmış.
    pub fn rows(&self, layers: &[Layer], layer: usize, selection: &Selection) -> Vec<FeatureRef> {
        self.collect(layers, layer, self.selected_only.then_some(selection))
    }

    /// Filtreye ve aramaya uyan satırlar, seçimden bağımsız; "tümünü seç" ve
    /// "tersine çevir" bunlar üzerinde çalışır.
    pub fn matching(&self, layers: &[Layer], layer: usize) -> Vec<FeatureRef> {
        self.collect(layers, layer, None)
    }

    fn collect(
        &self,
        layers: &[Layer],
        layer_index: usize,
        selected: Option<&Selection>,
    ) -> Vec<FeatureRef> {
        let Some(layer) = layers.get(layer_index) else {
            return Vec::new();
        };

        let search = self.search.trim();
        let reference = |feature: &Feature| FeatureRef::new(layer_index, feature.id);

        let mut features: Vec<&Feature> = layer
            .features
            .iter()
            .filter(|feature| self.filter.matches(&layer.schema, &feature.values))
            .filter(|feature| {
                selected.is_none_or(|selection| selection.contains(&reference(feature)))
            })
            .filter(|feature| {
                search.is_empty()
                    || feature.id.to_string() == search.trim_start_matches('#')
                    || (0..layer.schema.len()).any(|field| {
                        text::contains(&cell_text(layers, layer, feature, field), search)
                    })
            })
            .collect();

        if let Some((column, order)) = self.sort {
            features.sort_by(|a, b| {
                let ordering = compare(layers, layer, column, a, b);

                match order {
                    SortOrder::Ascending => ordering,
                    SortOrder::Descending => ordering.reverse(),
                }
            });
        }

        features.into_iter().map(reference).collect()
    }
}

/// Alanın hücrede gösterilen metni. Birim sütun başlığında yazıldığı için
/// değer birimsiz yazılır; nesne başvuruları hedef öğenin adıyla yazılır.
pub fn cell_text(layers: &[Layer], layer: &Layer, feature: &Feature, field: usize) -> String {
    let Some(definition) = layer.schema.get(field) else {
        return String::new();
    };

    match (&definition.kind, feature.value(field)) {
        (FieldKind::Object { target }, Value::Object(id)) => object_label(layers, target, *id),
        (_, value) => definition.format(value),
    }
}

/// Başvurulan öğenin adı; bulunamazsa "#numara".
pub fn object_label(layers: &[Layer], target: &str, id: ObjectId) -> String {
    layers
        .iter()
        .find(|layer| layer.name == target)
        .and_then(|layer| layer.feature(id).map(|feature| layer.label(feature)))
        .unwrap_or_else(|| format!("#{id}"))
}

/// Çizginin uzunluğu ya da alanın çevresi.
pub fn length_text(feature: &Feature) -> String {
    match feature.geometry {
        Geometry::Point(_) => String::new(),
        _ => format::distance(feature.geometry.length_meters()),
    }
}

fn compare(layers: &[Layer], layer: &Layer, column: Column, a: &Feature, b: &Feature) -> Ordering {
    match column {
        Column::Id => a.id.cmp(&b.id),
        Column::Length => a
            .geometry
            .length_meters()
            .total_cmp(&b.geometry.length_meters()),
        Column::Field(field) => match layer.schema.get(field).map(|field| &field.kind) {
            // Nesne başvuruları hedef adına göre sıralanır.
            Some(FieldKind::Object { .. }) => text::compare(
                &cell_text(layers, layer, a, field),
                &cell_text(layers, layer, b, field),
            ),
            _ => a.value(field).compare(b.value(field)),
        },
    }
    .then_with(|| a.id.cmp(&b.id))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sample;
    use kentos_ui::attribute::{Condition, Operator};

    fn layers() -> Vec<Layer> {
        sample::layers()
    }

    #[test]
    fn rows_are_filtered_searched_and_sorted() {
        let layers = layers();
        let cities = 0;
        let mut view = TableView::default();
        let selection = Selection::new();

        assert_eq!(view.rows(&layers, cities, &selection).len(), 16);

        view.filter = Query {
            conditions: vec![Condition {
                field: 1,
                operator: Operator::Equals,
                value: "Marmara".to_owned(),
            }],
            ..Query::default()
        };
        assert_eq!(view.rows(&layers, cities, &selection).len(), 2);

        view.filter = Query::default();
        view.search = "karadeniz".to_owned();
        assert_eq!(view.rows(&layers, cities, &selection).len(), 2);

        view.search = "#3".to_owned();
        assert_eq!(
            view.rows(&layers, cities, &selection),
            [FeatureRef::new(cities, ObjectId(3))]
        );

        view.search.clear();
        view.sort_by(Column::Field(2));
        view.sort_by(Column::Field(2));
        assert_eq!(view.order_of(Column::Field(2)), Some(SortOrder::Descending));

        let largest = view.rows(&layers, cities, &selection)[0];
        let (layer, feature) = largest.resolve(&layers).expect("şehir");
        assert_eq!(layer.label(feature), "İstanbul");
    }

    #[test]
    fn selected_only_shows_the_selection() {
        let layers = layers();
        let mut selection = Selection::new();
        selection.select(FeatureRef::new(0, ObjectId(3)));

        let view = TableView {
            selected_only: true,
            ..TableView::default()
        };

        assert_eq!(
            view.rows(&layers, 0, &selection),
            [FeatureRef::new(0, ObjectId(3))]
        );
        assert_eq!(view.matching(&layers, 0).len(), 16);
    }

    #[test]
    fn titles_carry_units_and_columns_fit_their_content() {
        let layers = layers();
        let cities = &layers[0];

        assert_eq!(Column::Field(2).title(cities), "Nüfus (kişi)");
        assert!(
            Column::Field(0).width(&layers, cities)
                < Column::Field(5).width(&layers, cities) + 200.0
        );
        assert!(Column::Id.width(&layers, cities) >= 56.0);
        assert!(Column::Field(2).is_monospaced(cities));
        assert!(!Column::Field(1).is_monospaced(cities));
    }

    #[test]
    fn references_show_the_target_name() {
        let layers = layers();
        let roads = &layers[2];
        let first = &roads.features[0];
        let start = roads.field_index("Başlangıç şehri").expect("alan");

        assert_eq!(cell_text(&layers, roads, first, start), "İstanbul");
    }
}
