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
    ResolveReason, ResolveReasonText, SETTINGS_SCHEMA_FORMAT, SETTINGS_SCHEMA_VERSION,
    SettingApply, SettingChoice, SettingDescriptor, SettingErrorCode, SettingErrorText,
    SettingGroup, SettingHost, SettingScope, SettingType, SettingsPreset, SettingsSchema,
};

use SettingHost::{Desktop, Web};

/// Every setting, group and preset of this version, and the messages.
pub fn settings_schema() -> SettingsSchema {
    SettingsSchema {
        format: SETTINGS_SCHEMA_FORMAT.into(),
        version: SETTINGS_SCHEMA_VERSION,
        groups: groups(),
        settings: settings(),
        presets: presets(),
        errors: SettingErrorCode::ALL
            .iter()
            .map(|&code| SettingErrorText {
                code,
                message: code.message().into(),
            })
            .collect(),
        reasons: ResolveReason::ALL
            .iter()
            .map(|&reason| ResolveReasonText {
                reason,
                message: reason.message().into(),
            })
            .collect(),
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
            "Tema, vurgu rengi, yazı tipi ve yazı boyutu.",
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
        group(
            "cloud",
            "Bulut",
            "Projelerin saklandığı, paylaşıldığı KentOS sunucusu.",
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
            .hosts(&[Web, Desktop])
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
        // Off by default (the owner, 26 Sep 2026): the command line shows the same step and
        // options more neatly; the strip stays a choice.
        boolean("drafting.commandBar", false)
            .hosts(&[Web, Desktop])
            .text(
                "Komut şeridi",
                "Komut çalışırken çizim alanının üstünde de aracın adı, beklediği adım ve seçenekleri (kenar sayısı, yöntem, kopya…) düğme olarak gösterilir. Kapalıyken (varsayılan) bunların hepsi, nokta hesabı ve tek seferlik kenet alttaki komut satırındadır.",
            ),
        boolean("drafting.hoverInfo", true)
            .hosts(&[Web, Desktop])
            .text(
                "Nesne bilgi kartı",
                "Seçim aracında bir nesnenin üzerinde durunca türü, katmanı, uzunluğu ya da alanı gösterilir.",
            ),
        boolean("drafting.snap", true)
            .scope(SettingScope::Session)
            .hosts(&[Web, Desktop])
            .text(
                "Kenetleme",
                "Seçili kenet türlerinin tümünü birlikte açıp kapatır (F3).",
            ),
        boolean("drafting.grid", true)
            .scope(SettingScope::Session)
            .hosts(&[Web, Desktop])
            .text("Izgara", "Çizim alanında ızgarayı gösterir (F7)."),
        boolean("drafting.tracking", true)
            .scope(SettingScope::Session)
            .hosts(&[Web, Desktop])
            .text(
                "Nesne izleme",
                "Bir kenet noktasının üzerinde kısa süre beklenince o noktadan yatay ve dikey kılavuzlar çıkar (Shift+F3).",
            ),
        // Topological editing (docs/adr/0160 §1): off at the start of every session.
        boolean("drafting.topology", false)
            .scope(SettingScope::Session)
            .hosts(&[Web, Desktop])
            .text(
                "Topolojik düzenleme",
                "Tutamaçla taşınan köşe, kenar ortasından eklenen köşe ve biçimlenen yay, görünen ve kilitsiz katmanlardaki komşu nesnelerin ortak köşe ve kenarlarında da birlikte değişir.",
            ),
        boolean("drafting.topologyPoints", false)
            .scope(SettingScope::Session)
            .hosts(&[Web, Desktop])
            .text(
                "Noktalar da",
                "Topolojik düzenlemede nokta nesneleri de ortak köşe sayılır ve köşeyle birlikte taşınır; kapalıyken ölçü noktaları yerinde kalır.",
            ),
        // The overlap control (docs/adr/0162 §1): Serbest at the start of every session.
        choice(
            "drafting.overlap",
            "allow",
            &[
                ("allow", "Serbest"),
                ("layer", "Kendi katmanında önle"),
                ("layers", "Seçili katmanlarda önle"),
            ],
        )
        .scope(SettingScope::Session)
        .hosts(&[Web, Desktop])
        .text(
            "Çakışma denetimi",
            "Çizilen yeni alanın komşu alanlarla örtüşen kısmı: serbest bırakılır ya da yeni alanın katmanındaki veya seçilen katmanlardaki görünen alanlarla örtüşen kısmı çıkarılarak yazılır.",
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
        // The snap additions (docs/adr/0163 §1): off at first.
        snap_kind(
            "snap.centroid",
            false,
            "Ağırlık merkezi",
            "Kapalı alanın ve kapalı çoklu çizginin ağırlık merkezi; delikler çıkarılır, parçalar alanlarıyla sayılır.",
        ),
        snap_kind(
            "snap.extension",
            false,
            "Uzantı",
            "Bir uçta durunca o kenarın devamı: düz kenarın doğrultusu, yayın çemberi; başka kenarlarla kesişimleri de.",
        ),
        snap_kind(
            "snap.parallel",
            false,
            "Paralel",
            "Bir kenarda durunca son noktadan o kenara paralel doğru.",
        ),
        snap_kind(
            "snap.grid",
            false,
            "Karelaj",
            "İmleç karelaj aralığındaki ızgaranın en yakın düğümüne gider; yakında başka kenet yoksa.",
        ),
        number("snap.gridEast", 1)
            .range(0.001, 100_000.0)
            .unit("m")
            .hosts(&[Web, Desktop])
            .text(
                "Karelaj aralığı Y",
                "Karelaj düğümleri arasındaki doğu (Y) uzaklığı; düğümler bu aralığın katı olan koordinatlardadır.",
            ),
        number("snap.gridNorth", 1)
            .range(0.001, 100_000.0)
            .unit("m")
            .hosts(&[Web, Desktop])
            .text(
                "Karelaj aralığı X",
                "Karelaj düğümleri arasındaki kuzey (X) uzaklığı; düğümler bu aralığın katı olan koordinatlardadır.",
            ),
        boolean("snap.self", true)
            .hosts(&[Web, Desktop])
            .text(
                "Çizilmekte olan nesneye",
                "Çizilen yolun önceki köşelerine ve kenarlarına da kenetlenir.",
            ),
        integer("snap.scaleMin", 0)
            .range(0.0, 100_000_000.0)
            .hosts(&[Web, Desktop])
            .text(
                "Kenedin en yakın ölçeği",
                "Görünüm 1:N'den yakınken kenet kapalı (N bu değer); 0: sınır yok.",
            ),
        integer("snap.scaleMax", 0)
            .range(0.0, 100_000_000.0)
            .hosts(&[Web, Desktop])
            .text(
                "Kenedin en uzak ölçeği",
                "Görünüm 1:N'den uzakken kenet kapalı (N bu değer); 0: sınır yok.",
            ),
        // ── Appearance ──────────────────────────────────────────────────
        // The same keys on both platforms (docs/adr/0126): the settings file
        // carries the look from one to the other. Older keys are read into
        // these ([`super::renamed_setting`]).
        choice(
            "appearance.theme",
            "dark",
            &[
                ("dark", "Koyu grafit"),
                ("light", "Açık pafta"),
                ("night", "Gece"),
                ("highContrast", "Yüksek karşıtlık"),
            ],
        )
        .hosts(&[Web, Desktop])
        .text(
            "Tema",
            "Arayüzün renkleri: koyu grafit, açık pafta, karanlık odada parlamayan gece ya da siyah zeminli yüksek karşıtlık.",
        ),
        // A preset or #rrggbb: the web's four and the desktop's eight, near
        // twins joined (turkuaz → teal, kehribar → amber), in hue order.
        text("appearance.accent", "navy", 16)
            .steps(&[
                (json!("navy"), "Lacivert"),
                (json!("blue"), "Mavi"),
                (json!("teal"), "Petrol yeşili"),
                (json!("green"), "Yeşil"),
                (json!("amber"), "Amber"),
                (json!("orange"), "Turuncu"),
                (json!("bordeaux"), "Bordo"),
                (json!("pink"), "Pembe"),
                (json!("violet"), "Mor"),
                (json!("gray"), "Gri"),
            ])
            .color()
            .version(2)
            .hosts(&[Web, Desktop])
            .text(
                "Vurgu rengi",
                "Çalışan araç, seçim, odak ve seçili öğeler bu renkle gösterilir; çizimdeki seçim rengi de ona uyar. Hazır renklerden biri ya da #RRGGBB; renk temanın zemininde okunur kalacak kadar ayarlanır.",
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
        .hosts(&[Web, Desktop])
        .text(
            "Yazı tipi",
            "Menüler, paneller, pencereler ve çizim alanındaki işaret yazıları; çizimdeki yazı nesneleri etkilenmez.",
        ),
        choice(
            "appearance.monoFont",
            "plexMono",
            &[("plexMono", "IBM Plex Mono"), ("jetbrains", "JetBrains Mono")],
        )
        .hosts(&[Web, Desktop])
        .text(
            "Eş aralıklı yazı",
            "Komut satırının, koordinatların ve ifade alanlarının yazısı.",
        ),
        integer("appearance.textSize", 13)
            .range(11.0, 18.0)
            .unit("px")
            .version(2)
            .hosts(&[Web, Desktop])
            .text(
                "Yazı boyutu",
                "Menüler, paneller ve komut satırı; arayüz onunla büyür. Çizim etiketleri etkilenmez.",
            ),
        // The shape of the chrome (docs/adr/0127): every radius together, and
        // the shadow of what floats; docked panels stay flat.
        choice(
            "appearance.corners",
            "soft",
            &[
                ("sharp", "Keskin"),
                ("soft", "Yumuşak"),
                ("round", "Yuvarlak"),
            ],
        )
        .hosts(&[Web, Desktop])
        .text(
            "Köşeler",
            "Düğmelerin, alanların, menülerin ve pencerelerin köşeleri: klasik CAD gibi keskin, yumuşak ya da yuvarlak.",
        ),
        choice(
            "appearance.shadows",
            "soft",
            &[("off", "Kapalı"), ("soft", "Hafif"), ("strong", "Belirgin")],
        )
        .hosts(&[Web, Desktop])
        .text(
            "Gölgeler",
            "Menülerin, açılır listelerin, ipuçlarının ve pencerelerin gölgesi; yerleşik paneller her zaman düzdür.",
        ),
        choice(
            "appearance.drawingBackground",
            "mode",
            &[
                ("mode", "Türe göre"),
                ("theme", "Temaya uy"),
                ("slate", "Arduvaz"),
                ("black", "Siyah"),
                ("paper", "Kâğıt"),
            ],
        )
        .hosts(&[Desktop])
        .text(
            "Çizim zemini",
            "Çizim alanının zemini, arayüzün temasından bağımsız: projenin türüne göre (CAD'de arduvaz, CBS'de tema; ADR 0165), temaya uyar, arduvaz, siyah (klasik AutoCAD) ya da kâğıt.",
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
        .hosts(&[Web, Desktop])
        .text("Artı imleç", "Çizim alanındaki imlecin kol uzunluğu."),
        boolean("appearance.startScreen", true)
            .hosts(&[Web, Desktop])
            .text(
                "Başlangıç ekranı",
                "Uygulama açılınca yeni proje, dosya aç, bulut ve son dosyalar gösterilir.",
            ),
        // ── New projects ────────────────────────────────────────────────
        integer("newProjects.srid", 5256)
            .range(1.0, 999_999.0)
            .hosts(&[Web, Desktop])
            .text(
                "Koordinat sistemi (EPSG)",
                "Yeni projelerde önerilen koordinat sistemi; açık projenin sistemi değişmez.",
            ),
        choice(
            "newProjects.workspace",
            "gis",
            &[("cad", "CAD"), ("gis", "CBS")],
        )
        .hosts(&[Web, Desktop])
        .text(
            "Proje türü",
            "Yeni proje sihirbazında önce seçili gelen tür; proje kendi türünü saklar (ADR 0165).",
        ),
        choice(
            "newProjects.drawingUnit",
            "m",
            &[("mm", "Milimetre"), ("cm", "Santimetre"), ("m", "Metre")],
        )
        .hosts(&[Web, Desktop])
        .text(
            "Çizim birimi",
            "Koordinat sistemi olmayan yeni CAD projesinin birimi; proje kendi birimini saklar (ADR 0165).",
        ),
        choice("newProjects.drawingFont", "barlow", DRAWING_FONTS)
            .hosts(&[Web, Desktop])
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
                "Çizimin pikseldeki örnek sayısı. Aygıtın desteklemediği bir değer istenirse desteklediği en yakın alt değer kullanılır ve nedeni gösterilir. Arayüz ve çizim alanındaki araç önizlemeleri, etiket kutuları ve işaretler bu ayardan etkilenmez.",
            ),
        boolean("graphics.hiDpi", true)
            .scope(SettingScope::Device)
            .hosts(&[Web, Desktop])
            .apply(SettingApply::Recreate)
            .text(
                "Tam çözünürlük (HiDPI)",
                "Retina ve 4K ekranda çizim ekranın tam çözünürlüğünde çizilir. Kapalıyken çizim mantıksal piksel başına bir piksel çizilir: 2× ekranda dörtte bir piksel, daha akıcı kaydırma. Arayüz ve çizim alanındaki araç önizlemeleri, etiket kutuları ve işaretler her zaman tam çözünürlüktedir.",
            ),
        choice(
            "graphics.symbolSize",
            "plot",
            &[("plot", "Çizim ölçeğinde"), ("screen", "Ekranda sabit")],
        )
        // The desktop draws styled layers too (docs/adr/0090).
        .hosts(&[Web, Desktop])
        .text(
            "Semboller",
            "Çizim ölçeğinde: basılı paftadaki boyları, harita ile büyür ve küçülür. Ekranda sabit: her yakınlıkta aynı boy.",
        ),
        boolean("graphics.lineWeights", true)
            .hosts(&[Web, Desktop])
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
            "gis",
            &[
                ("cad", "CAD"),
                ("gis", "CBS"),
                ("plan3d", "3D Plan"),
                ("disaster", "Afet analizi"),
            ],
        )
        .scope(SettingScope::Project)
        .hosts(&[Web, Desktop])
        .text(
            "Proje türü",
            "CAD ya da CBS: sahnesi, eksen ve açı düzeni ve şeridi türe göredir (ADR 0165).",
        ),
        choice("project.drawingFont", "barlow", DRAWING_FONTS)
            .scope(SettingScope::Project)
            .hosts(&[Web, Desktop])
            .text(
                "Çizim yazı tipi",
                "Çizimin kendi yazıları: yazı nesneleri, ölçü değerleri, etiketler. Projeyi açan herkes aynı harfleri görür.",
            ),
        // ── Cloud ───────────────────────────────────────────────────────
        // The web talks to the server it was loaded from; the desktop is told
        // which (docs/adr/0041). Plain http only to this computer (kentos-cloud).
        text("cloud.server", "http://127.0.0.1:8787", 2048)
            .hosts(&[Desktop])
            .text(
                "Sunucu adresi",
                "Bulut projelerinin saklandığı KentOS sunucusu. Bu bilgisayardaki sunucuya http, başka her sunucuya https ile bağlanılır.",
            ),
        // The last signed-in account's id (not a secret; the session never is kept): the
        // projects this device keeps for it open without a connection (docs/adr/0043).
        text("cloud.account", "", 64)
            .hosts(&[Desktop])
            .text(
                "Son hesap",
                "Bu bilgisayarda son giriş yapan hesabın kimliği; bağlantı yokken o hesabın bu cihazdaki projeleri açılır. Oturum ve parola saklanmaz.",
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
            "Çizimde kenar yumuşatma yok, mantıksal piksel başına bir piksel: çok büyük çizimlerde en akıcı kaydırma. Arayüz tam kalitede kalır.",
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
        color: false,
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

/// Free text of at most `max` characters.
fn text(key: &str, default: &str, max: u32) -> Build {
    let mut b = build(key, SettingType::Text, json!(default));
    b.0.max = Some(f64::from(max));
    b
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
    // The desktop's tools snap by the same kinds (docs/adr/0029).
    boolean(key, default)
        .hosts(&[Web, Desktop])
        .text(title, description)
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

    /// A text that may also be `#rrggbb` besides its choices.
    fn color(mut self) -> Self {
        self.0.color = true;
        self
    }

    /// A change of meaning or domain: a new version (docs/adr/0023).
    fn version(mut self, version: u32) -> Self {
        self.0.version = version;
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
