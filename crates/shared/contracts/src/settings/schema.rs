//! The settings of this version, in the order the settings windows show
//! them. The single source: the web reads the generated
//! `settingsSchema.json`, the desktop this function.
//!
//! Scopes follow the web's stores as they were (docs/adr/0023): what the web
//! kept in `kentos.prefs.v1` is `user`, its status bar toggles are `session`,
//! the project's are `project`. The graphics settings are `device` (TODOS.md
//! §7: GPU backend and MSAA belong to the device).

use serde_json::{Value, json};

use super::{
    SETTINGS_SCHEMA_FORMAT, SETTINGS_SCHEMA_VERSION, SettingApply, SettingChoice,
    SettingDescriptor, SettingGroup, SettingHost, SettingScope, SettingType, SettingsPreset,
    SettingsSchema,
};

use SettingHost::{Desktop, Web};

/// Every setting, group and preset of this version.
pub fn settings_schema() -> SettingsSchema {
    SettingsSchema {
        format: SETTINGS_SCHEMA_FORMAT.into(),
        version: SETTINGS_SCHEMA_VERSION,
        groups: groups(),
        settings: settings(),
        presets: presets(),
    }
}

fn groups() -> Vec<SettingGroup> {
    let group = |id: &str, title: &str, description: &str| SettingGroup {
        id: id.into(),
        title: title.into(),
        description: description.into(),
    };
    vec![
        group(
            "drafting",
            "Çizim yardımcıları",
            "Yeni noktanın nereye düşeceği ve yazılan değerin nerede açılacağı.",
        ),
        group("snap", "Kenetleme", "İmlecin hangi noktalara yapışacağı."),
        group(
            "appearance",
            "Görünüm",
            "Tema, vurgu rengi, yazı tipi, arayüz düzeni ve yazı boyutu.",
        ),
        group(
            "newProjects",
            "Yeni projeler",
            "Yeni projelerde önerilen başlangıç değerleri; açık projeyi değiştirmez.",
        ),
        group(
            "graphics",
            "Çizim motoru",
            "Çizim alanını ekran kartında çizen arka uç, kenar yumuşatma ve çözünürlük.",
        ),
        group(
            "project",
            "Proje",
            "Projeyle kaydedilen ayarlar (.kcad, bulut revizyonu); tercihler onları değiştirmez.",
        ),
    ]
}

fn settings() -> Vec<SettingDescriptor> {
    vec![
        // ── Drafting aids ───────────────────────────────────────────────
        boolean("drafting.ortho", false)
            .scope(SettingScope::Session)
            .hosts(&[Web, Desktop])
            .text(
                "Orto",
                "Yeni nokta son noktanın tam yatayına ya da dikeyine düşer. Shift basılıyken tersine döner (F8).",
            ),
        boolean("drafting.polar", false)
            .scope(SettingScope::Session)
            .hosts(&[Web, Desktop])
            .text(
                "Kutupsal izleme",
                "Son noktadan açı adımlarında kılavuz çıkar; imleç yaklaşınca yapışır (F10). Orto açıksa önceliklidir.",
            ),
        number("drafting.polarIncrement", 45)
            .steps(&[
                (json!(15), "15°"),
                (json!(30), "30°"),
                (json!(45), "45°"),
                (json!(90), "90°"),
            ])
            .unit("deg")
            .hosts(&[Web, Desktop])
            .text(
                "Kutupsal izleme açı adımı",
                "Kutupsal izleme açıkken imleç bu açının katlarına kilitlenir.",
            ),
        integer("drafting.snapAperture", 11)
            .range(4.0, 30.0)
            .unit("px")
            .hosts(&[Web, Desktop])
            .text(
                "Kenet yarıçapı",
                "İmlecin bir noktaya yapışması için gereken yakınlık; kapalı alanın ilk köşesine tıklayıp kapatmak da bu yakınlıktadır.",
            ),
        integer("drafting.pickAperture", 5)
            .range(2.0, 15.0)
            .unit("px")
            .hosts(&[Web])
            .text(
                "Seçim yarıçapı",
                "Tıklamanın bir çizgiyi yakalaması için gereken yakınlık.",
            ),
        boolean("drafting.cursorInput", true)
            .hosts(&[Web, Desktop])
            .text(
                "İmleç yanında değer girişi",
                "Komut sırasında yazılan mesafe ve koordinatlar imlecin yanında açılır; kapalıyken komut satırına gider.",
            ),
        boolean("drafting.hoverInfo", true)
            .hosts(&[Web])
            .text(
                "Nesne bilgi kartı",
                "Seçim aracında bir nesnenin üzerinde durunca türü, katmanı, uzunluğu ya da alanı gösterilir.",
            ),
        boolean("drafting.snap", true)
            .scope(SettingScope::Session)
            .hosts(&[Web])
            .text(
                "Kenetleme",
                "Seçili kenet türlerinin tümünü birlikte açıp kapatır (F3).",
            ),
        boolean("drafting.grid", true)
            .scope(SettingScope::Session)
            .hosts(&[Web])
            .text("Izgara", "Çizim alanında ızgarayı gösterir (F7)."),
        boolean("drafting.tracking", true)
            .scope(SettingScope::Session)
            .hosts(&[Web])
            .text(
                "Nesne izleme",
                "Bir kenet noktasının üzerinde kısa süre beklenince o noktadan yatay ve dikey kılavuzlar çıkar (Shift+F3).",
            ),
        // ── Snap kinds ──────────────────────────────────────────────────
        snap_kind(
            "snap.endpoint",
            true,
            "Uç nokta",
            "Çizgi, parsel ve bina köşeleri; dairelerin çeyrek noktaları.",
        ),
        snap_kind(
            "snap.midpoint",
            true,
            "Orta nokta",
            "Kenarların ve yayların tam ortası.",
        ),
        snap_kind("snap.center", true, "Merkez", "Daire ve yay merkezleri."),
        snap_kind(
            "snap.node",
            true,
            "Nokta",
            "Poligon, kot ve tekil noktalar.",
        ),
        snap_kind(
            "snap.intersection",
            true,
            "Kesişim",
            "İki kenarın kesiştiği nokta.",
        ),
        snap_kind(
            "snap.perpendicular",
            true,
            "Dik",
            "Son noktadan kenara inen dikmenin ayağı.",
        ),
        snap_kind(
            "snap.tangent",
            true,
            "Teğet",
            "Son noktadan daire ya da yaya çizilen teğetin değme noktası.",
        ),
        snap_kind(
            "snap.nearest",
            false,
            "En yakın",
            "Kenar üzerindeki en yakın nokta; başka kenet yoksa devreye girer.",
        ),
        // ── Appearance ──────────────────────────────────────────────────
        // The desktop keeps its theme here. The web's theme stays in its layout store
        // (`kentos.ui.v1`) until its appearance settings move (docs/adr/0023).
        choice(
            "appearance.theme",
            "dark",
            &[("dark", "Koyu grafit"), ("light", "Açık pafta")],
        )
        .hosts(&[Desktop])
        .text(
            "Tema",
            "Arayüzün ve çizim alanının renkleri: koyu grafit ya da açık pafta.",
        ),
        choice(
            "appearance.accent",
            "navy",
            &[
                ("navy", "Lacivert"),
                ("amber", "Amber"),
                ("teal", "Petrol yeşili"),
                ("bordeaux", "Bordo"),
            ],
        )
        .hosts(&[Web])
        .text(
            "Vurgu rengi",
            "Çalışan araç, seçim, odak ve seçili öğeler bu renkle gösterilir; çizimdeki seçim rengi de ona uyar.",
        ),
        choice(
            "appearance.uiFont",
            "jakarta",
            &[
                ("jakarta", "Plus Jakarta Sans"),
                ("inter", "Inter"),
                ("plex", "IBM Plex Sans"),
                ("source", "Source Sans 3"),
                ("noto", "Noto Sans"),
                ("roboto", "Roboto"),
                ("system", "Sistem yazı tipi"),
            ],
        )
        .hosts(&[Web])
        .text(
            "Yazı tipi",
            "Menüler, paneller, pencereler ve çizim alanındaki işaret yazıları; çizimdeki yazı nesneleri etkilenmez.",
        ),
        choice(
            "appearance.shell",
            "classic",
            &[("classic", "Klasik"), ("ribbon", "Şerit")],
        )
        .hosts(&[Web])
        .text(
            "Arayüz düzeni",
            "Menüler, araç çubuğu ve kayan araç kutusu ya da sekmeli şerit; ikisi aynı araç ve komutları sunar.",
        ),
        choice(
            "appearance.uiScale",
            "standard",
            &[
                ("small", "Küçük"),
                ("standard", "Standart"),
                ("large", "Büyük"),
                ("xlarge", "Çok büyük"),
                ("xxlarge", "En büyük"),
            ],
        )
        .hosts(&[Web])
        .text(
            "Yazı boyutu",
            "Menüler, paneller ve komut satırı. Çizim etiketleri etkilenmez.",
        ),
        choice(
            "appearance.crosshair",
            "medium",
            &[
                ("small", "Küçük"),
                ("medium", "Orta"),
                ("full", "Tam ekran"),
            ],
        )
        .hosts(&[Web])
        .text("Artı imleç", "Çizim alanındaki imlecin kol uzunluğu."),
        boolean("appearance.startScreen", true)
            .hosts(&[Web])
            .text(
                "Başlangıç ekranı",
                "Uygulama açılınca yeni proje, dosya aç, bulut ve son dosyalar gösterilir.",
            ),
        // ── New projects ────────────────────────────────────────────────
        integer("newProjects.srid", 5256)
            .range(1.0, 999_999.0)
            .hosts(&[Web])
            .text(
                "Koordinat sistemi (EPSG)",
                "Yeni projelerde önerilen koordinat sistemi; açık projenin sistemi değişmez.",
            ),
        choice(
            "newProjects.workspace",
            "hybrid",
            &[("hybrid", "Hibrit"), ("cad", "CAD"), ("gis", "CBS")],
        )
        .hosts(&[Web])
        .text(
            "Çalışma modu",
            "Yeni projelerde önce önerilen çalışma modu; proje kendi modunu saklar.",
        ),
        choice("newProjects.drawingFont", "barlow", DRAWING_FONTS)
            .hosts(&[Web])
            .text(
                "Çizim yazı tipi",
                "Yeni projelerin çizim yazı tipi; proje kendi yazı tipini saklar.",
            ),
        // ── Graphics ────────────────────────────────────────────────────
        choice(
            "graphics.backend",
            "webgl2",
            &[("webgl2", "WebGL2"), ("webgpu", "WebGPU")],
        )
        .scope(SettingScope::Device)
        .hosts(&[Web])
        .apply(SettingApply::Recreate)
        .text(
            "Çizim arka ucu",
            "WebGL2 tüm güncel tarayıcılarda çalışır; WebGPU yeni nesil arayüzdür. Başlatılamazsa WebGL2'ye dönülür.",
        ),
        integer("graphics.msaa", 4)
            .steps(&[
                (json!(1), "Kapalı"),
                (json!(2), "2×"),
                (json!(4), "4×"),
                (json!(8), "8×"),
                (json!(16), "16×"),
            ])
            .unit("sample")
            .scope(SettingScope::Device)
            .hosts(&[Web, Desktop])
            .apply(SettingApply::Recreate)
            .text(
                "Kenar yumuşatma (MSAA)",
                "Pikseldeki örnek sayısı. Aygıtın desteklemediği bir değer istenirse desteklediği en yakın alt değer kullanılır ve nedeni gösterilir.",
            ),
        boolean("graphics.hiDpi", true)
            .scope(SettingScope::Device)
            .hosts(&[Web, Desktop])
            .apply(SettingApply::Recreate)
            .text(
                "Tam çözünürlük (HiDPI)",
                "Retina ve 4K ekranda çizim ekranın tam çözünürlüğünde çizilir. Kapalıyken mantıksal piksel başına bir piksel çizilir: 2× ekranda dörtte bir piksel, daha akıcı kaydırma.",
            ),
        choice(
            "graphics.symbolSize",
            "plot",
            &[("plot", "Çizim ölçeğinde"), ("screen", "Ekranda sabit")],
        )
        .hosts(&[Web])
        .text(
            "Semboller",
            "Çizim ölçeğinde: basılı paftadaki boyları, harita ile büyür ve küçülür. Ekranda sabit: her yakınlıkta aynı boy.",
        ),
        boolean("graphics.lineWeights", true)
            .hosts(&[Web])
            .text(
                "Çizgi kalınlığı",
                "Katman çizgileri kalınlıklarıyla çizilir. Kapalıyken hepsi ince çizilir.",
            ),
        // ── Project ─────────────────────────────────────────────────────
        // Read from the open project (`ProjectSettings`); never from a preference.
        integer("project.srid", 5256)
            .range(1.0, 999_999.0)
            .scope(SettingScope::Project)
            .hosts(&[Web, Desktop])
            .text(
                "Koordinat sistemi (EPSG)",
                "Projenin koordinat sistemi. Atamak koordinatları dönüştürmez.",
            ),
        integer("project.lengthDecimals", 3)
            .range(0.0, 4.0)
            .scope(SettingScope::Project)
            .hosts(&[Web, Desktop])
            .text(
                "Uzunluk basamağı",
                "Koordinatlar, kenar uzunlukları ve mesafeler kaç ondalıkla gösterilir; kaynak koordinat değişmez.",
            ),
        integer("project.areaDecimals", 2)
            .range(0.0, 4.0)
            .scope(SettingScope::Project)
            .hosts(&[Web, Desktop])
            .text(
                "Alan basamağı",
                "Alanlar kaç ondalıkla gösterilir; kaynak geometri değişmez.",
            ),
        choice(
            "project.areaUnit",
            "m2",
            &[
                ("m2", "m²"),
                ("donum", "dönüm (1000 m²)"),
                ("ha", "hektar (10 000 m²)"),
            ],
        )
        .scope(SettingScope::Project)
        .hosts(&[Web, Desktop])
        .text("Alan birimi", "Alanların gösterildiği birim."),
        choice(
            "project.angleUnit",
            "grad",
            &[("grad", "Grad"), ("deg", "Derece")],
        )
        .scope(SettingScope::Project)
        .hosts(&[Web, Desktop])
        .text("Açı birimi", "Semt ve açıların gösterildiği birim."),
        number("project.plotScale", 1000)
            .range(1.0, 1_000_000.0)
            .scope(SettingScope::Project)
            .hosts(&[Web, Desktop])
            .text(
                "Pafta ölçeği",
                "Çizim ölçeğindeki sembollerin ve yazıların basılı boyunu belirler (1:1000 → 1000).",
            ),
        choice(
            "project.workspace",
            "hybrid",
            &[
                ("hybrid", "Hibrit"),
                ("cad", "CAD"),
                ("gis", "CBS"),
                ("plan3d", "3D Plan"),
                ("disaster", "Afet analizi"),
            ],
        )
        .scope(SettingScope::Project)
        .hosts(&[Web, Desktop])
        .text(
            "Çalışma modu",
            "Projenin açıldığı çalışma modu; yalnız sunuşu değiştirir, veriyi değil.",
        ),
        choice("project.drawingFont", "barlow", DRAWING_FONTS)
            .scope(SettingScope::Project)
            .hosts(&[Web, Desktop])
            .text(
                "Çizim yazı tipi",
                "Çizimin kendi yazıları: yazı nesneleri, ölçü değerleri, etiketler. Projeyi açan herkes aynı harfleri görür.",
            ),
    ]
    .into_iter()
    .map(|s| s.0)
    .collect()
}

const DRAWING_FONTS: &[(&str, &str)] = &[
    ("barlow", "Barlow"),
    ("arimo", "Arimo"),
    ("overpass", "Overpass"),
    ("quicksand", "Quicksand"),
    ("architects-daughter", "Architects Daughter"),
    ("courier-prime", "Courier Prime"),
    ("plex-mono", "IBM Plex Mono"),
];

fn presets() -> Vec<SettingsPreset> {
    let preset =
        |id: &str, title: &str, description: &str, msaa: i64, hi_dpi: bool| SettingsPreset {
            id: id.into(),
            group: "graphics".into(),
            title: title.into(),
            description: description.into(),
            values: [
                ("graphics.msaa".to_owned(), json!(msaa)),
                ("graphics.hiDpi".to_owned(), json!(hi_dpi)),
            ]
            .into_iter()
            .collect(),
        };
    // Display only: no preset changes what is saved or how precisely (TODOS.md §8.2).
    vec![
        preset(
            "fast",
            "Hızlı",
            "Kenar yumuşatma yok, mantıksal piksel başına bir piksel: çok büyük çizimlerde en akıcı kaydırma.",
            1,
            false,
        ),
        preset(
            "balanced",
            "Dengeli",
            "Ekranın tam çözünürlüğü, kenar yumuşatma yok; ince çizgiler biraz basamaklı.",
            1,
            true,
        ),
        preset(
            "quality",
            "Kaliteli",
            "4× kenar yumuşatma ve ekranın tam çözünürlüğü.",
            4,
            true,
        ),
    ]
}

/// A descriptor being built: user scope, applied live, version 1 unless said.
struct Build(SettingDescriptor);

fn build(key: &str, kind: SettingType, default: Value) -> Build {
    let group = key.split('.').next().unwrap_or(key).to_owned();
    Build(SettingDescriptor {
        key: key.into(),
        kind,
        default,
        min: None,
        max: None,
        choices: Vec::new(),
        unit: None,
        scope: SettingScope::User,
        hosts: Vec::new(),
        version: 1,
        group,
        title: String::new(),
        description: String::new(),
        sensitive: false,
        apply: SettingApply::Live,
    })
}

fn boolean(key: &str, default: bool) -> Build {
    build(key, SettingType::Boolean, json!(default))
}

fn integer(key: &str, default: i64) -> Build {
    build(key, SettingType::Integer, json!(default))
}

fn number(key: &str, default: i64) -> Build {
    build(key, SettingType::Number, json!(default))
}

fn choice(key: &str, default: &str, choices: &[(&str, &str)]) -> Build {
    let mut b = build(key, SettingType::Enum, json!(default));
    b.0.choices = choices
        .iter()
        .map(|(value, label)| SettingChoice {
            value: json!(value),
            label: (*label).into(),
        })
        .collect();
    b
}

fn snap_kind(key: &str, default: bool, title: &str, description: &str) -> Build {
    boolean(key, default).hosts(&[Web]).text(title, description)
}

impl Build {
    fn range(mut self, min: f64, max: f64) -> Self {
        self.0.min = Some(min);
        self.0.max = Some(max);
        self
    }

    fn steps(mut self, steps: &[(Value, &str)]) -> Self {
        self.0.choices = steps
            .iter()
            .map(|(value, label)| SettingChoice {
                value: value.clone(),
                label: (*label).into(),
            })
            .collect();
        self
    }

    fn unit(mut self, unit: &str) -> Self {
        self.0.unit = Some(unit.into());
        self
    }

    fn scope(mut self, scope: SettingScope) -> Self {
        self.0.scope = scope;
        self
    }

    fn hosts(mut self, hosts: &[SettingHost]) -> Self {
        self.0.hosts = hosts.to_vec();
        self
    }

    fn apply(mut self, apply: SettingApply) -> Self {
        self.0.apply = apply;
        self
    }

    fn text(mut self, title: &str, description: &str) -> Self {
        self.0.title = title.into();
        self.0.description = description.into();
        self
    }
}
