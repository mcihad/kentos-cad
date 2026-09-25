//! Vitrin uygulamasının örnek verisi: Türkiye'nin şehirleri, önemli
//! yerleri, ana yolları, nehirleri ve İstanbul ilçeleri (basitleştirilmiş).
//!
//! Her katmanın bir öznitelik şeması vardır; alanlar kentos-ui'nin bütün
//! alan türlerini kullanır. Nesne başvuruları (en yakın şehir, yolların
//! başlangıç ve bitiş şehri) veriden hesaplanır. Bilinmeyen değerler (hız
//! sınırı, bakım tarihi, açılış saati) boş bırakılmıştır; nesne inceleyiciyle
//! doldurulabilir.
//!
//! Şehirler bölgelerine, yollar türlerine, çizimler durumlarına göre alt
//! katmanlara ayrılır; her alt katmanın kendi rengi vardır.

use iced::Color;

use kentos_ui::attribute::{Field, ObjectId, Value};
use kentos_ui::spatial::{Feature, Geometry, Layer, LonLat, Sublayer, measure};

use crate::layer_tree::{Entry, LayerTree};

const DRAWING_COLOR: Color = Color::from_rgb(0.55, 0.86, 0.26);
const CITY_COLOR: Color = Color::from_rgb(0.96, 0.35, 0.38);
const POI_COLOR: Color = Color::from_rgb(0.69, 0.52, 0.97);
const ROAD_COLOR: Color = Color::from_rgb(0.96, 0.62, 0.25);
const RIVER_COLOR: Color = Color::from_rgb(0.29, 0.62, 0.96);
const DISTRICT_COLOR: Color = Color::from_rgb(0.18, 0.56, 0.60);

/// Şehirler katmanının adı; nesne başvuruları bu katmanı hedefler.
pub const CITIES: &str = "Şehirler";

/// Çizim araçlarının ürettiği geometri türleri.
pub const DRAWING_KINDS: [&str; 6] = [
    "Çizgi",
    "Çoklu çizgi",
    "Alan",
    "Dikdörtgen",
    "Daire",
    "Nokta",
];

/// Coğrafi bölgeler ve haritadaki renkleri.
const REGIONS: [(&str, Color); 7] = [
    ("Marmara", Color::from_rgb8(0x5B, 0x9C, 0xF6)),
    ("Ege", Color::from_rgb8(0x36, 0xC5, 0xB5)),
    ("Akdeniz", Color::from_rgb8(0xF2, 0x9E, 0x4C)),
    ("İç Anadolu", Color::from_rgb8(0xE3, 0xC1, 0x5A)),
    ("Karadeniz", Color::from_rgb8(0x6C, 0xCB, 0x7F)),
    ("Doğu Anadolu", Color::from_rgb8(0xE4, 0x77, 0xB8)),
    ("Güneydoğu", Color::from_rgb8(0xEF, 0x64, 0x61)),
];

/// Yol türleri: otoyol katmanın renginde, diğerleri açılarak.
const ROAD_KINDS: [(&str, Color); 3] = [
    ("Otoyol", ROAD_COLOR),
    ("Devlet yolu", Color::from_rgb8(0xE8, 0xC4, 0x68)),
    ("Bulvar", Color::from_rgb8(0xF4, 0xD9, 0xB0)),
];

/// Çizim araçlarının geometri eklediği, boş başlayan katman.
pub fn drawing_layer() -> Layer {
    Layer::lines("Çizimler", DRAWING_COLOR)
        .fill_alpha(0.18)
        .stroke_width(1.8)
        .with_schema([
            Field::text("Ad")
                .required()
                .description("Çizimin adı; haritada ve listelerde gösterilir."),
            Field::choice("Tür", DRAWING_KINDS)
                .read_only()
                .description("Çizimi üreten araç."),
            Field::choice("Durum", ["Taslak", "Onaylandı"])
                .description("Onaylanan çizimler haritada mavi gösterilir."),
            Field::datetime("Oluşturma")
                .read_only()
                .description("Çizimin oluşturulduğu an, Türkiye saatiyle."),
            Field::text("Not")
                .multiline()
                .description("Serbest açıklama; birden çok satır olabilir."),
        ])
        .with_sublayers(
            "Durum",
            [
                Sublayer::new("Taslak", DRAWING_COLOR),
                Sublayer::new("Onaylandı", Color::from_rgb8(0x4F, 0xC3, 0xF7)),
            ],
        )
}

/// Katman ağacı: çizimler en üstte, örnek veri iç içe gruplarda.
///
/// Katman sırası: 0 Çizimler, 1 Şehirler, 2 Önemli Yerler, 3 Karayolları,
/// 4 Nehirler, 5 İlçeler.
pub fn layer_tree() -> LayerTree {
    let mut tree = LayerTree::flat(6);

    let settlement = tree.add_group("Yerleşim", true, vec![Entry::Layer(1), Entry::Layer(2)]);
    let transport = tree.add_group("Ulaşım", true, vec![Entry::Layer(3)]);
    let nature = tree.add_group("Doğal yapı", true, vec![Entry::Layer(4)]);
    let boundaries = tree.add_group("İdari sınırlar", true, vec![Entry::Layer(5)]);
    let istanbul = tree.add_group("İstanbul", false, vec![Entry::Group(boundaries)]);
    let country = tree.add_group(
        "Türkiye",
        true,
        vec![
            Entry::Group(settlement),
            Entry::Group(transport),
            Entry::Group(nature),
            Entry::Group(istanbul),
        ],
    );

    tree.roots = vec![Entry::Layer(0), Entry::Group(country)];
    // Yolların türlere göre alt katmanları açık başlar.
    tree.expanded[3] = true;
    tree
}

/// Örnek veri katmanları.
pub fn layers() -> Vec<Layer> {
    let cities = Layer::points(CITIES, CITY_COLOR)
        .labels_from(5.0)
        .with_schema([
            Field::text("Ad").required().description("İlin resmî adı."),
            Field::choice("Bölge", REGIONS.map(|(region, _)| region))
                .description("Coğrafi bölge; şehirler haritada bölgelerine göre renklenir."),
            Field::integer("Nüfus")
                .unit("kişi")
                .description("2024 adrese dayalı nüfus kayıt sistemi sonucu."),
            Field::integer("Plaka")
                .between(1, 81)
                .description("İl trafik kodu."),
            Field::boolean("Kıyı şehri").description("İlin denize kıyısı var mı."),
            Field::text("Not").multiline(),
        ])
        .with_features(cities())
        .with_sublayers(
            "Bölge",
            REGIONS.map(|(region, color)| Sublayer::new(region, color)),
        );

    let centers: Vec<(ObjectId, LonLat)> = cities
        .features
        .iter()
        .filter_map(|feature| match feature.geometry {
            Geometry::Point(location) => Some((feature.id, location)),
            _ => None,
        })
        .collect();

    // Konuma en yakın şehir.
    let nearest = |location: LonLat| -> Value {
        centers
            .iter()
            .min_by(|(_, a), (_, b)| {
                measure::haversine_meters(*a, location)
                    .total_cmp(&measure::haversine_meters(*b, location))
            })
            .map_or(Value::Null, |(id, _)| Value::Object(*id))
    };

    let mut places = places_of_interest();

    for place in &mut places {
        if let Geometry::Point(location) = place.geometry {
            place.values.push(nearest(location));
        }
    }

    let mut roads = roads();

    for road in &mut roads {
        let ends = (
            road.geometry.vertices().first().copied(),
            road.geometry.vertices().last().copied(),
        );

        if let (Some(start), Some(end)) = ends {
            road.values
                .extend([Value::Null, nearest(start), nearest(end)]);
        }
    }

    vec![
        cities,
        Layer::points("Önemli Yerler", POI_COLOR)
            .labels_from(9.5)
            .with_schema([
                Field::text("Ad").required(),
                Field::choice(
                    "Kategori",
                    [
                        "Tarihi yapı",
                        "Anıt",
                        "Saray",
                        "Köprü",
                        "Doğal alan",
                        "Antik kent",
                        "Manastır",
                        "Kıyı",
                    ],
                ),
                Field::object("En yakın şehir", CITIES).description(
                    "Yere en yakın il merkezi; listeden ya da haritadan değiştirilir.",
                ),
                Field::time("Açılış").description("Ziyaretçilere açıldığı saat."),
                Field::text("Not").multiline(),
            ])
            .with_features(places),
        Layer::lines("Karayolları", ROAD_COLOR)
            .stroke_width(2.2)
            .opacity(0.95)
            .with_schema([
                Field::text("Ad").required(),
                Field::choice("Tür", ["Otoyol", "Devlet yolu", "Bulvar"])
                    .description("Yol sınıfı; haritada türüne göre renklenir."),
                Field::choice("Durum", ["Hizmette", "Yapım aşamasında", "Planlanan"]),
                Field::range("Hız sınırı", 30.0, 140.0, 10.0)
                    .unit("km/sa")
                    .description("Binek araçlar için genel hız sınırı."),
                Field::object("Başlangıç şehri", CITIES).description("Yolun başladığı il."),
                Field::object("Bitiş şehri", CITIES).description("Yolun bittiği il."),
                Field::date("Son bakım").description("En son bakım ya da onarım tarihi."),
            ])
            .with_features(roads)
            .with_sublayers(
                "Tür",
                ROAD_KINDS.map(|(kind, color)| Sublayer::new(kind, color)),
            ),
        Layer::lines("Nehirler", RIVER_COLOR)
            .stroke_width(1.6)
            .opacity(0.95)
            .with_schema([
                Field::text("Ad").required(),
                Field::choice("Tür", ["Nehir", "Çay"]),
                Field::text("Not").multiline(),
            ])
            .with_features(rivers()),
        Layer::polygons("İlçeler", DISTRICT_COLOR)
            .opacity(0.9)
            .with_schema([
                Field::text("Ad").required(),
                Field::text("İl"),
                Field::real("Yaklaşık alan", 1)
                    .unit("km²")
                    .description("Basitleştirilmiş sınırdan hesaplanan yüzölçümü."),
                Field::text("Not").multiline(),
            ])
            .with_features(districts()),
    ]
}

fn cities() -> Vec<Feature> {
    const CITIES: &[(&str, f64, f64, i64, &str, i64, bool)] = &[
        (
            "İstanbul",
            28.9784,
            41.0082,
            15_840_900,
            "Marmara",
            34,
            true,
        ),
        (
            "Ankara",
            32.8597,
            39.9334,
            5_782_000,
            "İç Anadolu",
            6,
            false,
        ),
        ("İzmir", 27.1428, 38.4237, 4_479_500, "Ege", 35, true),
        ("Bursa", 29.0610, 40.1885, 3_214_600, "Marmara", 16, true),
        ("Antalya", 30.7133, 36.8969, 2_696_000, "Akdeniz", 7, true),
        (
            "Konya",
            32.4932,
            37.8746,
            2_296_000,
            "İç Anadolu",
            42,
            false,
        ),
        ("Adana", 35.3213, 37.0000, 2_270_000, "Akdeniz", 1, true),
        (
            "Gaziantep",
            37.3825,
            37.0662,
            2_164_000,
            "Güneydoğu",
            27,
            false,
        ),
        (
            "Şanlıurfa",
            38.7955,
            37.1591,
            2_213_000,
            "Güneydoğu",
            63,
            false,
        ),
        ("Mersin", 34.6415, 36.8000, 1_916_000, "Akdeniz", 33, true),
        (
            "Diyarbakır",
            40.2306,
            37.9144,
            1_818_000,
            "Güneydoğu",
            21,
            false,
        ),
        (
            "Kayseri",
            35.4881,
            38.7312,
            1_441_000,
            "İç Anadolu",
            38,
            false,
        ),
        ("Samsun", 36.3300, 41.2867, 1_371_000, "Karadeniz", 55, true),
        ("Trabzon", 39.7168, 41.0027, 816_000, "Karadeniz", 61, true),
        (
            "Erzurum",
            41.2769,
            39.9043,
            749_000,
            "Doğu Anadolu",
            25,
            false,
        ),
        (
            "Van",
            43.3800,
            38.4942,
            1_141_000,
            "Doğu Anadolu",
            65,
            false,
        ),
    ];

    CITIES
        .iter()
        .map(|&(name, lon, lat, population, region, plate, coastal)| {
            Feature::new(Geometry::Point(LonLat::new(lon, lat))).with_values([
                Value::from(name),
                Value::from(region),
                Value::Integer(population),
                Value::Integer(plate),
                Value::Bool(coastal),
            ])
        })
        .collect()
}

fn places_of_interest() -> Vec<Feature> {
    const PLACES: &[(&str, f64, f64, &str)] = &[
        ("Ayasofya", 28.9800, 41.0086, "Tarihi yapı"),
        ("Galata Kulesi", 28.9744, 41.0256, "Anıt"),
        ("Topkapı Sarayı", 28.9833, 41.0117, "Saray"),
        ("Boğaziçi Köprüsü", 29.0345, 41.0455, "Köprü"),
        ("Kapadokya", 34.8280, 38.6431, "Doğal alan"),
        ("Pamukkale", 29.1206, 37.9204, "Doğal alan"),
        ("Efes", 27.3417, 37.9397, "Antik kent"),
        ("Nemrut Dağı", 38.7410, 37.9808, "Anıt"),
        ("Sümela Manastırı", 39.6575, 40.6900, "Manastır"),
        ("Ölüdeniz", 29.1164, 36.5499, "Kıyı"),
    ];

    PLACES
        .iter()
        .map(|&(name, lon, lat, category)| {
            Feature::new(Geometry::Point(LonLat::new(lon, lat)))
                .with_values([Value::from(name), Value::from(category)])
        })
        .collect()
}

/// Ad ve tür değerleriyle bir çizgi öğesi.
fn line_feature(name: &str, kind: &str, path: Vec<LonLat>) -> Feature {
    Feature::new(Geometry::Line(path)).with_values([Value::from(name), Value::from(kind)])
}

/// Hizmetteki bir yol; hız sınırı ve şehirler `layers` içinde eklenir.
fn road(name: &str, kind: &str, path: Vec<LonLat>) -> Feature {
    let mut feature = line_feature(name, kind, path);
    feature.values.push(Value::from("Hizmette"));
    feature
}

fn roads() -> Vec<Feature> {
    vec![
        road(
            "O-1 (İstanbul – Edirne)",
            "Otoyol",
            coords(&[
                (28.978, 41.060),
                (28.700, 41.150),
                (28.400, 41.200),
                (27.900, 41.250),
                (27.200, 41.350),
                (26.700, 41.600),
                (26.550, 41.680),
            ]),
        ),
        road(
            "O-4 / TEM (İstanbul – Ankara)",
            "Otoyol",
            coords(&[
                (29.050, 41.100),
                (29.500, 41.000),
                (30.200, 40.800),
                (30.900, 40.550),
                (31.400, 40.300),
                (32.000, 40.100),
                (32.600, 39.950),
                (32.860, 39.933),
            ]),
        ),
        road(
            "O-31 (İzmir – Aydın)",
            "Otoyol",
            coords(&[
                (27.143, 38.424),
                (27.400, 38.200),
                (27.700, 38.000),
                (28.000, 37.850),
            ]),
        ),
        road(
            "D-400 Akdeniz Sahil Yolu",
            "Devlet yolu",
            coords(&[
                (29.116, 36.550),
                (30.500, 36.300),
                (31.500, 36.200),
                (32.500, 36.100),
                (33.500, 36.200),
                (34.642, 36.800),
            ]),
        ),
        road(
            "O-52 Güneydoğu Otoyolu",
            "Otoyol",
            coords(&[
                (34.642, 36.800),
                (35.500, 37.000),
                (36.200, 37.200),
                (37.000, 37.100),
                (37.383, 37.066),
            ]),
        ),
        road(
            "D-010 Karadeniz Sahil Yolu",
            "Devlet yolu",
            coords(&[
                (29.050, 41.200),
                (31.000, 41.300),
                (33.000, 41.500),
                (35.000, 41.600),
                (36.330, 41.287),
                (38.300, 41.050),
                (39.717, 41.003),
                (41.000, 41.100),
                (41.277, 39.904),
            ]),
        ),
        road(
            "D-100 / E-5 (İstanbul içi)",
            "Bulvar",
            coords(&[
                (28.830, 40.990),
                (28.920, 41.000),
                (29.000, 41.020),
                (29.060, 41.000),
            ]),
        ),
    ]
}

fn rivers() -> Vec<Feature> {
    vec![
        line_feature(
            "Kızılırmak",
            "Nehir",
            coords(&[
                (39.850, 39.800),
                (39.200, 39.300),
                (38.600, 39.000),
                (38.000, 38.700),
                (37.500, 38.300),
                (36.800, 37.900),
                (36.200, 37.400),
                (35.900, 37.150),
                (35.800, 36.900),
            ]),
        ),
        line_feature(
            "Sakarya",
            "Nehir",
            coords(&[
                (31.000, 38.900),
                (31.200, 39.300),
                (31.000, 39.700),
                (30.700, 40.050),
                (30.400, 40.350),
                (30.100, 40.600),
                (29.900, 40.900),
                (29.900, 41.100),
            ]),
        ),
        line_feature(
            "Yeşilırmak",
            "Nehir",
            coords(&[
                (39.800, 40.200),
                (38.800, 40.400),
                (37.800, 40.550),
                (37.000, 40.650),
                (36.600, 41.100),
                (36.400, 41.300),
            ]),
        ),
        line_feature(
            "Fırat",
            "Nehir",
            coords(&[
                (38.700, 39.000),
                (38.300, 38.600),
                (38.000, 38.200),
                (37.600, 37.800),
                (37.500, 37.400),
                (37.800, 37.000),
                (38.200, 36.800),
            ]),
        ),
        line_feature(
            "Dicle",
            "Nehir",
            coords(&[
                (42.200, 38.300),
                (41.800, 37.900),
                (41.200, 37.600),
                (40.600, 37.400),
                (40.200, 37.200),
                (39.800, 37.000),
                (39.200, 36.900),
            ]),
        ),
        line_feature(
            "Meriç",
            "Nehir",
            coords(&[
                (26.350, 41.650),
                (26.500, 41.300),
                (26.550, 41.000),
                (26.350, 40.750),
                (26.200, 40.550),
            ]),
        ),
    ]
}

fn districts() -> Vec<Feature> {
    const DISTRICTS: &[(&str, f64, f64, f64)] = &[
        ("Fatih", 28.945, 41.014, 0.020),
        ("Beyoğlu", 28.980, 41.033, 0.014),
        ("Beşiktaş", 29.020, 41.060, 0.022),
        ("Şişli", 28.990, 41.065, 0.020),
        ("Kağıthane", 28.972, 41.080, 0.016),
        ("Eyüpsultan", 28.930, 41.070, 0.020),
        ("Gaziosmanpaşa", 28.910, 41.060, 0.014),
        ("Zeytinburnu", 28.905, 40.995, 0.012),
        ("Bakırköy", 28.865, 40.982, 0.020),
        ("Bahçelievler", 28.860, 41.000, 0.015),
        ("Bağcılar", 28.850, 41.040, 0.018),
        ("Küçükçekmece", 28.790, 41.000, 0.025),
        ("Başakşehir", 28.800, 41.080, 0.024),
        ("Üsküdar", 29.030, 41.035, 0.024),
        ("Ümraniye", 29.100, 41.040, 0.022),
        ("Kadıköy", 29.055, 40.995, 0.025),
        ("Ataşehir", 29.105, 40.995, 0.018),
        ("Maltepe", 29.135, 40.935, 0.020),
        ("Kartal", 29.180, 40.900, 0.018),
        ("Beykoz", 29.100, 41.120, 0.030),
        ("Sarıyer", 29.055, 41.150, 0.035),
    ];

    DISTRICTS
        .iter()
        .enumerate()
        .map(|(index, &(name, lon, lat, radius))| {
            let polygon = blob(lon, lat, radius, 7, index as u64 + 1);
            let area = std::f64::consts::PI * (radius * 111.0).powi(2) * 0.85;

            Feature::new(Geometry::Polygon(polygon)).with_values([
                Value::from(name),
                Value::from("İstanbul"),
                Value::Real((area * 10.0).round() / 10.0),
            ])
        })
        .collect()
}

fn coords(points: &[(f64, f64)]) -> Vec<LonLat> {
    points
        .iter()
        .map(|&(lon, lat)| LonLat::new(lon, lat))
        .collect()
}

/// Düzensiz görünümlü, yaklaşık dairesel bir çokgen üretir.
fn blob(center_lon: f64, center_lat: f64, radius: f64, vertices: usize, seed: u64) -> Vec<LonLat> {
    let mut state = seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1;
    let lon_stretch = 1.0 / center_lat.to_radians().cos();

    (0..vertices)
        .map(|index| {
            let angle = std::f64::consts::TAU * index as f64 / vertices as f64;
            let wobble = 0.72 + 0.5 * random_unit(&mut state);
            let distance = radius * wobble;

            LonLat::new(
                center_lon + distance * lon_stretch * angle.cos(),
                center_lat + distance * angle.sin(),
            )
        })
        .collect()
}

/// Basit xorshift tabanlı sözde rastgele sayı üretici (0..1).
fn random_unit(state: &mut u64) -> f64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;

    (*state % 10_000) as f64 / 10_000.0
}
