//! Veri içe aktarma sihirbazı: kaynak dosya, sütunlar, koordinat sistemi ve
//! özet.
//!
//! Vitrinde dosya sistemi yoktur; sihirbaz üç örnek dosya sunar. CSV'de
//! koordinat sütunları seçilir ve değerleri denetlenir, GeoJSON'da geometri
//! dosyanın içindedir, bozuk dosya okunamaz ve hata durumu görünür. Biten
//! içe aktarma haritaya yeni bir katman ekler.

use iced::Color;

use kentos_ui::attribute::{Field, Value};
use kentos_ui::spatial::{Feature, Geometry, Layer, LonLat};

/// Sihirbazın adımları.
pub const STEPS: [&str; 4] = ["Kaynak", "Alanlar", "Koordinat sistemi", "Özet"];

/// Seçilebilen koordinat sistemleri: kod, ad, açıklama ve değerlerin derece
/// olup olmadığı.
pub const SYSTEMS: [(&str, &str, &str, bool); 4] = [
    (
        "EPSG:4326",
        "WGS 84",
        "Enlem ve boylam, derece. GPS alıcıları ve GeoJSON bunu kullanır.",
        true,
    ),
    (
        "EPSG:5254",
        "TUREF / TM30",
        "Türkiye ulusal projeksiyonu, 30° dilimi; metre.",
        false,
    ),
    (
        "EPSG:32636",
        "WGS 84 / UTM 36N",
        "UTM 36. dilim; metre.",
        false,
    ),
    (
        "EPSG:3857",
        "WGS 84 / Pseudo-Mercator",
        "Web haritalarının izdüşümü; metre.",
        false,
    ),
];

/// Sihirbazdaki örnek dosyalar.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Source {
    Stations,
    Parks,
    Broken,
}

impl Source {
    pub const ALL: [Source; 3] = [Source::Stations, Source::Parks, Source::Broken];

    pub fn file(self) -> &'static str {
        match self {
            Source::Stations => "istasyonlar.csv",
            Source::Parks => "milli-parklar.geojson",
            Source::Broken => "sayim-2023.csv",
        }
    }

    pub fn format(self) -> &'static str {
        match self {
            Source::Stations | Source::Broken => "CSV",
            Source::Parks => "GeoJSON",
        }
    }

    pub fn size(self) -> &'static str {
        match self {
            Source::Stations => "1,6 KB",
            Source::Parks => "9,8 KB",
            Source::Broken => "3,1 KB",
        }
    }

    /// Dosya listesindeki kısa özet.
    pub fn summary(self) -> String {
        match self {
            Source::Stations => format!("{} kayıt, {} sütun", STATIONS.len(), CSV_COLUMNS.len()),
            Source::Parks => format!("{} alan, 3 öznitelik", PARKS.len()),
            Source::Broken => "Okunamadı".to_owned(),
        }
    }

    /// Dosya okunamıyorsa nedeni.
    pub fn error(self) -> Option<&'static str> {
        match self {
            Source::Broken => Some(
                "12. satırda 6 sütun bekleniyordu, 5 var; ayraç olarak noktalı virgül \
                 kullanılmış olabilir. Dosyayı düzeltip yeniden deneyin ya da başka bir dosya \
                 seçin.",
            ),
            _ => None,
        }
    }

    /// Koordinatlar sütunlardan mı okunur (CSV), dosyada mı (GeoJSON).
    pub fn has_columns(self) -> bool {
        self.format() == "CSV"
    }

    /// Önerilen katman adı.
    pub fn layer_name(self) -> &'static str {
        match self {
            Source::Stations => "Meteoroloji istasyonları",
            Source::Parks => "Milli parklar",
            Source::Broken => "Sayım 2023",
        }
    }

    pub fn count(self) -> usize {
        match self {
            Source::Stations => STATIONS.len(),
            Source::Parks => PARKS.len(),
            Source::Broken => 0,
        }
    }

    pub fn geometry(self) -> &'static str {
        match self {
            Source::Parks => "Alan",
            _ => "Nokta",
        }
    }
}

/// İstasyon dosyasının sütunları.
pub const CSV_COLUMNS: [&str; 5] = ["istasyon", "il", "enlem", "boylam", "yukseklik_m"];

/// GeoJSON dosyasının öznitelikleri ve türleri.
pub const PARK_FIELDS: [(&str, &str); 3] =
    [("ad", "metin"), ("il", "metin"), ("kurulus", "tam sayı")];

/// Meteoroloji istasyonları: ad, il, enlem, boylam, yükseklik (m).
pub const STATIONS: [(&str, &str, f64, f64, i64); 24] = [
    ("Edirne", "Edirne", 41.68, 26.55, 51),
    ("Kırklareli", "Kırklareli", 41.74, 27.22, 232),
    ("Çanakkale", "Çanakkale", 40.14, 26.40, 6),
    ("Balıkesir", "Balıkesir", 39.65, 27.87, 139),
    ("Akhisar", "Manisa", 38.92, 27.84, 93),
    ("Aydın", "Aydın", 37.85, 27.84, 56),
    ("Muğla", "Muğla", 37.21, 28.36, 646),
    ("Denizli", "Denizli", 37.78, 29.09, 425),
    ("Afyonkarahisar", "Afyonkarahisar", 38.76, 30.54, 1034),
    ("Eskişehir", "Eskişehir", 39.78, 30.52, 801),
    ("Kütahya", "Kütahya", 39.42, 29.98, 969),
    ("Bolu", "Bolu", 40.73, 31.61, 743),
    ("Zonguldak", "Zonguldak", 41.45, 31.79, 135),
    ("Kastamonu", "Kastamonu", 41.38, 33.78, 800),
    ("Sinop", "Sinop", 42.03, 35.15, 32),
    ("Çorum", "Çorum", 40.55, 34.95, 776),
    ("Tokat", "Tokat", 40.31, 36.55, 608),
    ("Sivas", "Sivas", 39.75, 37.02, 1285),
    ("Malatya", "Malatya", 38.35, 38.31, 948),
    ("Elazığ", "Elazığ", 38.67, 39.22, 990),
    ("Van", "Van", 38.50, 43.38, 1671),
    ("Kars", "Kars", 40.60, 43.10, 1775),
    ("Hakkari", "Hakkari", 37.57, 43.74, 1728),
    ("Ceylanpınar", "Şanlıurfa", 36.85, 40.05, 398),
];

/// Milli parklar: ad, il, kuruluş yılı, merkez (enlem, boylam).
pub const PARKS: [(&str, &str, i64, f64, f64); 10] = [
    ("Kaz Dağı", "Balıkesir", 1994, 39.70, 26.85),
    ("Uludağ", "Bursa", 1961, 40.10, 29.12),
    ("Yedigöller", "Bolu", 1965, 40.94, 31.74),
    ("Küre Dağları", "Kastamonu", 2000, 41.72, 33.40),
    ("Göreme Tarihî", "Nevşehir", 1986, 38.64, 34.83),
    ("Beyşehir Gölü", "Konya", 1993, 37.68, 31.53),
    ("Köprülü Kanyon", "Antalya", 1973, 37.18, 31.17),
    ("Nemrut Dağı", "Adıyaman", 1988, 37.98, 38.74),
    ("Munzur Vadisi", "Tunceli", 1971, 39.18, 39.55),
    ("Ağrı Dağı", "Ağrı", 2004, 39.70, 44.30),
];

/// Sihirbazın durumu.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct ImportWizard {
    pub step: usize,
    pub source: Option<Source>,
    /// Boylam (X) ve enlem (Y) sütunları; yalnızca CSV'de.
    pub x: Option<usize>,
    pub y: Option<usize>,
    /// [`SYSTEMS`] içindeki seçim.
    pub system: usize,
    pub name: String,
}

impl ImportWizard {
    /// Dosyayı seçer; koordinat sütunları adlarından tanınır.
    pub fn select(&mut self, source: Source) {
        self.source = Some(source);
        self.x = CSV_COLUMNS.iter().position(|column| *column == "boylam");
        self.y = CSV_COLUMNS.iter().position(|column| *column == "enlem");
        self.system = 0;
        self.name = source.layer_name().to_owned();
    }

    /// Sütundaki sayılar; metin sütununda `None`.
    fn values(column: usize) -> Option<Vec<f64>> {
        match column {
            2 => Some(STATIONS.iter().map(|station| station.2).collect()),
            3 => Some(STATIONS.iter().map(|station| station.3).collect()),
            4 => Some(STATIONS.iter().map(|station| station.4 as f64).collect()),
            _ => None,
        }
    }

    /// Sütunun ilk kaydındaki değer; önizleme için.
    pub fn sample(row: usize, column: usize) -> String {
        let Some(station) = STATIONS.get(row) else {
            return String::new();
        };

        match column {
            0 => station.0.to_owned(),
            1 => station.1.to_owned(),
            2 => format!("{:.2}", station.2),
            3 => format!("{:.2}", station.3),
            _ => station.4.to_string(),
        }
    }

    /// Süren adımın eksiği; yoksa `None` ve İleri etkindir.
    pub fn problem(&self, layers: &[Layer]) -> Option<String> {
        let source = self.source?;

        match self.step {
            0 => source
                .error()
                .map(|_| format!("{} okunamıyor; başka bir dosya seçin.", source.file())),
            1 if source.has_columns() => self.columns_problem(),
            1 => None,
            2 => (!SYSTEMS[self.system].3).then(|| {
                format!(
                    "Değerler derece; {} metre bekler. WGS 84'ü seçin.",
                    SYSTEMS[self.system].1
                )
            }),
            _ => {
                let name = self.name.trim();

                if name.is_empty() {
                    Some("Katmana bir ad verin.".to_owned())
                } else if layers.iter().any(|layer| layer.name == name) {
                    Some(format!("\"{name}\" adında bir katman zaten var."))
                } else {
                    None
                }
            }
        }
    }

    /// Koordinat sütunlarının denetimi: seçili, farklı, sayı ve derece
    /// aralığında olmalı.
    fn columns_problem(&self) -> Option<String> {
        let (Some(x), Some(y)) = (self.x, self.y) else {
            return Some("Boylam (X) ve enlem (Y) sütunlarını seçin.".to_owned());
        };

        if x == y {
            return Some("Boylam ve enlem için farklı sütunlar seçin.".to_owned());
        }

        for (column, name, limit) in [(x, "Boylam", 180.0), (y, "Enlem", 90.0)] {
            let Some(values) = Self::values(column) else {
                return Some(format!(
                    "{name} sütunu sayı olmalı; \"{}\" metin içeriyor.",
                    CSV_COLUMNS[column]
                ));
            };

            if let Some(value) = values.iter().find(|value| value.abs() > limit) {
                return Some(format!(
                    "{name} değerleri −{limit} ile {limit} arasında olmalı; \"{}\" sütununda {value} var.",
                    CSV_COLUMNS[column]
                ));
            }
        }

        None
    }

    /// Sütunlar geçerli ama yer değiştirmiş görünüyorsa uyarı.
    pub fn swapped(&self) -> bool {
        self.x == CSV_COLUMNS.iter().position(|column| *column == "enlem")
            && self.y == CSV_COLUMNS.iter().position(|column| *column == "boylam")
    }

    pub fn is_ready(&self, layers: &[Layer]) -> bool {
        self.source.is_some() && self.problem(layers).is_none()
    }
}

/// İçe aktarılan katman: seçilen sütunlarla okunmuş öğeler.
pub fn layer(source: Source, name: &str, x: usize, y: usize) -> Layer {
    match source {
        Source::Parks => Layer::polygons(name, Color::from_rgb(0.33, 0.78, 0.45))
            .fill_alpha(0.22)
            .stroke_width(1.4)
            .with_schema([
                Field::text("Ad").required(),
                Field::text("İl"),
                Field::integer("Kuruluş").description("Milli park ilan edildiği yıl."),
            ])
            .with_features(PARKS.map(|(park, province, year, lat, lon)| {
                Feature::new(Geometry::Polygon(hexagon(LonLat::new(lon, lat)))).with_values([
                    Value::from(park),
                    Value::from(province),
                    Value::Integer(year),
                ])
            })),
        _ => Layer::points(name, Color::from_rgb(0.98, 0.78, 0.30))
            .labels_from(6.5)
            .with_schema([
                Field::text("Ad").required(),
                Field::text("İl"),
                Field::integer("Yükseklik").unit("m"),
            ])
            .with_features(STATIONS.iter().enumerate().map(|(row, station)| {
                let value = |column| ImportWizard::values(column).map_or(0.0, |values| values[row]);

                Feature::new(Geometry::Point(LonLat::new(value(x), value(y)))).with_values([
                    Value::from(station.0),
                    Value::from(station.1),
                    Value::Integer(station.4),
                ])
            })),
    }
}

/// Merkezin çevresinde altıgen: milli parkın kabaca sınırı.
fn hexagon(center: LonLat) -> Vec<LonLat> {
    (0..6)
        .map(|corner| {
            let angle = f64::from(corner) * std::f64::consts::TAU / 6.0;

            LonLat::new(
                center.lon + 0.32 * angle.cos(),
                center.lat + 0.24 * angle.sin(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn columns_are_checked_for_numbers_and_ranges() {
        let mut wizard = ImportWizard::default();
        wizard.select(Source::Stations);
        wizard.step = 1;

        // Adlarından tanınır: boylam X, enlem Y.
        assert_eq!((wizard.x, wizard.y), (Some(3), Some(2)));
        assert_eq!(wizard.problem(&[]), None);

        wizard.x = Some(1);
        assert!(
            wizard
                .problem(&[])
                .is_some_and(|problem| problem.contains("metin"))
        );

        wizard.x = Some(4);
        assert!(
            wizard
                .problem(&[])
                .is_some_and(|problem| problem.contains("yukseklik_m"))
        );

        wizard.x = Some(2);
        wizard.y = Some(3);
        assert_eq!(wizard.problem(&[]), None);
        assert!(wizard.swapped());
    }

    #[test]
    fn broken_files_metric_systems_and_taken_names_stop_the_wizard() {
        let mut wizard = ImportWizard::default();

        wizard.select(Source::Broken);
        assert!(wizard.problem(&[]).is_some());

        wizard.select(Source::Parks);
        assert_eq!(wizard.problem(&[]), None);

        wizard.step = 2;
        wizard.system = 1;
        assert!(wizard.problem(&[]).is_some());

        wizard.system = 0;
        wizard.step = 3;
        let taken = [layer(Source::Parks, "Milli parklar", 3, 2)];
        assert!(wizard.problem(&taken).is_some());

        wizard.name = "Parklar".to_owned();
        assert!(wizard.is_ready(&taken));
    }

    #[test]
    fn imported_layers_read_the_chosen_columns() {
        let stations = layer(Source::Stations, "İstasyonlar", 3, 2);
        assert_eq!(stations.features.len(), 24);

        let edirne = &stations.features[0].geometry;
        assert_eq!(edirne.vertices(), [LonLat::new(26.55, 41.68)]);

        let parks = layer(Source::Parks, "Parklar", 3, 2);
        assert_eq!(parks.features.len(), 10);
        assert_eq!(parks.features[0].geometry.label(), "Alan");
    }
}
