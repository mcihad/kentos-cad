//! Bileşen galerisi: sayfalar ve etkileşimli örneklerin durumu.
//!
//! Galeri, kentos-ui'nin kataloğudur. Her sayfa bir grup bileşeni canlı
//! örnekleriyle gösterir; örnekler kendi küçük durumlarını burada tutar ve
//! uygulamanın asıl durumuna dokunmaz.

use std::fmt;

use kentos_ui::attribute::query::Edit;
use kentos_ui::attribute::{
    Condition, Date, DateTime, Field, ObjectId, Operator, Query, Time, Value, text,
};
use kentos_ui::icon::Icon;
use kentos_ui::spatial::{SelectionMode, Tool};
use kentos_ui::widget::Toast;
use kentos_ui::widget::assets;
use kentos_ui::widget::color::Ramp;
use kentos_ui::widget::command_line::Entry;
use kentos_ui::widget::docking::{self, Docks, Side};
use kentos_ui::widget::floating::{self, Placement, Windows};
use kentos_ui::widget::inspector;
use kentos_ui::widget::rulers::{self, Guide, Guides};
use kentos_ui::widget::table::SortOrder;
use kentos_ui::widget::timeline::{self, Playback};
use kentos_ui::widget::tree_view::Place;
use kentos_ui::widget::viewports::{self, Arrangement, Views};

use crate::message::Message;
use crate::sample;
use crate::view::gallery::scene::{Camera, Shading};

/// Galeri sayfaları.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Page {
    Colors,
    Typography,
    Icons,
    Buttons,
    Data,
    Frame,
    Layout,
    Inputs,
    Feedback,
    Attributes,
    Spatial,
}

impl Page {
    pub fn label(self) -> &'static str {
        match self {
            Page::Colors => "Renkler",
            Page::Typography => "Yazı",
            Page::Icons => "İkonlar",
            Page::Buttons => "Düğmeler",
            Page::Data => "Veri",
            Page::Frame => "Çerçeve",
            Page::Layout => "Yerleşim",
            Page::Inputs => "Girdiler",
            Page::Feedback => "Geri bildirim",
            Page::Attributes => "Öznitelikler",
            Page::Spatial => "Mekânsal",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Page::Colors => Icon::Drop,
            Page::Typography => Icon::Type,
            Page::Icons => Icon::Grid,
            Page::Buttons => Icon::Button,
            Page::Data => Icon::Table,
            Page::Frame => Icon::Layout,
            Page::Layout => Icon::Tabs,
            Page::Inputs => Icon::Slider,
            Page::Feedback => Icon::Info,
            Page::Attributes => Icon::Properties,
            Page::Spatial => Icon::Globe,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Page::Colors => "Tema belirteçleri: arayüzün ve model alanının bütün renkleri.",
            Page::Typography => "Tip ölçeği, yazı tipleri ve hazır metin biçimleri.",
            Page::Icons => "16×16 ızgarada çizilmiş vektör ikon seti, boyutları ve tonları.",
            Page::Buttons => "Düğme stilleri, şerit düğmeleri ve ipuçları.",
            Page::Data => {
                "Tablo, sanal tablo, ağaç görünümü, lejant, varlık tarayıcısı, özellik ızgarası, \
                 panel ve giriş alanları."
            }
            Page::Frame => {
                "Kayan pencereler, durum çubuğu, komut kutusu, gezinme çubuğu, bağlam menüsü, \
                 mini araç çubuğu, dairesel menü ve iletişim kutuları."
            }
            Page::Layout => {
                "Sekmeli yuva, görünüm alanları, karşılaştırma perdesi, cetveller ve belge \
                 sekmeleri: çalışma alanının düzeni."
            }
            Page::Inputs => {
                "Birimli sayı, vektör ve açı girişleri; renk seçici ve rampa; anahtar, radyo \
                 grubu, aralık kaydırıcısı, etiket girişi, zaman çizelgesi ve form düzeni."
            }
            Page::Feedback => {
                "Bildirimler, ilerleme ve görevler, onay kutusu, uyarı şeridi, boş ve hata \
                 durumları, adımlı sihirbaz ve özellikler penceresi."
            }
            Page::Attributes => {
                "Nesne inceleyici, öznitelik tablosu, sorgu oluşturucu ve alan türleri."
            }
            Page::Spatial => "ViewCube, pusula, araçlar, nesne yakalama ve Türkçe biçimlendirme.",
        }
    }
}

/// Örneklerle etkileşim.
#[derive(Debug, Clone)]
pub enum Demo {
    /// Etkisi olmayan bir örnek düğmeye basıldı; komut satırına yazılır.
    Pressed(&'static str),
    RowSelected(usize),
    Toggled(usize),
    Checked(bool),
    OpacityChanged(f32),
    TextChanged(String),
    CrsSelected(Crs),
    CommandChanged(String),
    CommandSubmitted,
    /// Komut kutusu örneğinde öneri listesinden seçilen komut.
    CommandRun(String),
    CommandExpanded(bool),
    /// Öznitelikler sayfası: tabloda satır seçildi.
    RecordSelected(usize),
    /// Öznitelikler ızgarası örneği: bölüm açıldı ya da kapandı, katman
    /// seçildi, hücreye yazılan onaylandı.
    SheetToggled(usize),
    SheetLayer(usize),
    SheetHeight(String),
    Inspector(inspector::Event),
    QueryEdited(Edit),
    ModeSelected(SelectionMode),
    /// Tablo sütununa göre sırala ya da yönü çevir.
    Sorted(usize),
    SearchChanged(String),
    /// Seçici örnekleri.
    DatePicked(Option<Date>),
    MomentPicked(Option<DateTime>),
    TimePicked(Option<Time>),
    RegionPicked(Option<usize>),
    /// Ağaç örneği: klasörü aç/kapat.
    ProjectToggled(usize),
    /// Klasörün kutusu: içindekilerin hepsini işaretler ya da kaldırır.
    ProjectFolderChecked(usize),
    ProjectFileChecked(usize),
    ProjectSelected(ProjectRow),
    /// Panel örneğinde paneli açar ya da kapatır.
    PanelToggled(usize),
    /// Kayan pencere örneği.
    Window(floating::Event<DemoPane>),
    /// Kayan pencereleri ilk yerlerine döndürür, kapalıysa açar.
    PanesReset,
    /// Nesne yakalama penceresindeki bir yakalama türü.
    SnapToggled(usize),
    /// Kayan pencerelerin arkasındaki düğme.
    StagePressed,
    /// Örnek bildirim gösterir; [`sample_toasts`] sırasıyla, sonuncusu hepsini.
    Notify(usize),
    /// Sihirbaz örneğinde adım.
    WizardStep(usize),
    /// Özellikler penceresi örneğinde bölüm.
    SectionSelected(usize),
    /// Belge sekmeleri örneği.
    DocumentSelected(usize),
    DocumentClosed(usize),
    DocumentMoved(usize, usize),
    DocumentAdded,
    DocumentSaved,
    /// Sekmeli yuva örneği.
    Dock(docking::Event<DemoPanel>),
    DockReset,
    /// Girdiler sayfası.
    Length(f64),
    Thickness(f64),
    Scale(f64),
    Position([f64; 3]),
    Rotation(f64),
    Bearing(f64),
    Stroke(iced::Color),
    Fill(iced::Color),
    RampChanged(Ramp, usize),
    /// Görünüm alanı örneği.
    Views(viewports::Event),
    Camera(usize, Camera),
    Shading(usize, Shading),
    /// Şerit örneği: galerideki rampa ve daraltma.
    RibbonRamp(usize),
    RibbonCollapsed,
    /// Temel kontroller ve form örneği.
    Switched(usize, bool),
    Method(usize),
    UnitSystem(usize),
    Population((f64, f64)),
    Tags(Vec<String>),
    SheetName(String),
    Paper(usize),
    Latitude(f64),
    TitleBlock(bool),
    /// Sanal tablo örneği: satır seçildi ya da numarasıyla gidildi.
    ParcelSelected(usize),
    /// Taşınabilir ağaç örneği: kaynak ve hedef satır, yer.
    OutlineMoved(usize, usize, Place),
    OutlineOpened(usize),
    OutlineShown(usize),
    OutlineLocked(usize),
    OutlineSelected(usize),
    /// Satırı yerinde adlandırmaya başlar (sağ tık menüsü ya da F2).
    OutlineRename(usize),
    OutlineInput(String),
    OutlineRenamed,
    OutlineCancelled,
    /// Mini araç çubuğu örneği: şekil seçildi (`None` boşluğa tıklandı),
    /// kilitlendi ya da silindi.
    ShapeSelected(Option<usize>),
    ShapeLocked(usize),
    ShapeDeleted(usize),
    ShapesReset,
    /// Dairesel menü örneği.
    RadialOpened,
    RadialClosed,
    RadialChosen(Tool),
    /// Cetvel örneği: kılavuzlar, yakınlık (yüzde) ve y yönü.
    RulerGuide(rulers::Event),
    RulerZoom(f64),
    RulerGuidesCleared,
    /// Pusula örneği: haritanın dönüşü (derece, saat yönünde) ve kuzeye
    /// döndürme.
    MapRotated(f64),
    NorthReset,
    /// Zaman çizelgesi örnekleri: proje takvimi ve animasyon; oynatılırken
    /// zamanlayıcı.
    Project(timeline::Event),
    Animation(timeline::Event),
    Tick(iced::time::Instant),
    /// Lejant örneği: daraltma ve satırı gizleme.
    LegendCollapsed,
    LegendItem(usize),
    /// Varlık tarayıcısı örneği.
    AssetSelected(usize),
    AssetActivated(usize),
    AssetSearch(String),
    AssetCategory(Option<String>),
    AssetView(assets::View),
    /// Karşılaştırma perdesi örneği: perdenin yeri ve yönü.
    CompareMoved(f32),
    CompareVertical(bool),
}

/// Sekmeli yuva örneğinin panelleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoPanel {
    Layers,
    Styles,
    Properties,
    Tasks,
    Output,
    History,
}

impl DemoPanel {
    pub fn title(self) -> &'static str {
        match self {
            DemoPanel::Layers => "Katmanlar",
            DemoPanel::Styles => "Stiller",
            DemoPanel::Properties => "Özellikler",
            DemoPanel::Tasks => "Görevler",
            DemoPanel::Output => "Çıktı",
            DemoPanel::History => "Geçmiş",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            DemoPanel::Layers => Icon::Layers,
            DemoPanel::Styles => Icon::Drop,
            DemoPanel::Properties => Icon::Properties,
            DemoPanel::Tasks => Icon::Progress,
            DemoPanel::Output => Icon::Terminal,
            DemoPanel::History => Icon::Clock,
        }
    }
}

/// Sekmeli yuva örneğinin açılış yerleşimi.
pub fn demo_docks() -> Docks<DemoPanel> {
    let mut docks = Docks::new();

    docks.dock(DemoPanel::Layers, Side::Left);
    docks.dock(DemoPanel::Styles, Side::Left);
    docks.split(DemoPanel::Tasks, Side::Left);
    docks.dock(DemoPanel::Properties, Side::Right);
    docks.dock(DemoPanel::Output, Side::Bottom);
    docks.dock(DemoPanel::History, Side::Bottom);
    docks.show(DemoPanel::Layers, Side::Left);
    docks.set_size(Side::Left, 220.0);
    docks.set_size(Side::Right, 220.0);
    docks.set_size(Side::Bottom, 150.0);
    docks
}

/// Bildirim örnekleri: düğme adı ve bildirim.
pub fn sample_toasts() -> [(&'static str, Toast<Message>); 5] {
    [
        (
            "Bilgi",
            Toast::info("Katman eklendi").body("İstasyonlar: 24 nokta, EPSG:4326."),
        ),
        (
            "Başarı",
            Toast::success("Dışa aktarıldı").body("Türkiye.geojson: 60 öğe, 1,2 MB."),
        ),
        (
            "Uyarı",
            Toast::warning("3 kayıt atlandı").body("Geometrisi boş olan kayıtlar içe aktarılmadı."),
        ),
        (
            "Hata",
            Toast::error("Altlık haritaya bağlanılamadı")
                .body("tiles.kentos.local yanıt vermedi; önbellekteki paftalar gösteriliyor.")
                .action(
                    "Yeniden dene",
                    Message::Gallery(Demo::Pressed("Yeniden dene")),
                ),
        ),
        (
            "Eylemli",
            Toast::success("Çizim silindi")
                .action("Geri al", Message::Gallery(Demo::Pressed("Geri al"))),
        ),
    ]
}

/// Kayan pencere örneğinin pencereleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoPane {
    Snap,
    Layer,
}

/// Nesne yakalama penceresindeki yakalama türleri.
pub const SNAP_KINDS: [&str; 4] = ["Uç nokta", "Orta nokta", "Merkez", "Kesişim"];

/// Kayan pencere örneğinin açılış düzeni.
fn demo_panes() -> Windows<DemoPane> {
    let mut panes = Windows::new();

    panes.open(
        DemoPane::Layer,
        Placement::bottom_right(floating::GAP, floating::GAP),
    );
    panes.open(
        DemoPane::Snap,
        Placement::top_left(floating::GAP, floating::GAP),
    );
    panes
}

/// Ağaç örneğindeki satır: klasör ya da dosya.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectRow {
    Folder(usize),
    File(usize),
}

/// Ağaç örneğindeki klasörlerin dosyaları: 0 proje, 1 paftalar, 2 dış
/// referanslar, 3 plan kararları. Projenin kutusu bütün dosyaları kapsar.
pub const PROJECT_FILES: [&[usize]; 4] = [&[0, 1, 2, 3, 4], &[0, 1], &[2, 3], &[4]];

/// Açılır liste örneğindeki koordinat sistemleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Crs {
    Wgs84,
    WebMercator,
    Tm30,
    Utm36,
}

impl Crs {
    pub const ALL: [Crs; 4] = [Crs::Wgs84, Crs::WebMercator, Crs::Tm30, Crs::Utm36];
}

impl fmt::Display for Crs {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Crs::Wgs84 => "EPSG:4326 WGS 84",
            Crs::WebMercator => "EPSG:3857 Web Mercator",
            Crs::Tm30 => "EPSG:5254 TUREF / TM30",
            Crs::Utm36 => "EPSG:32636 WGS 84 / UTM 36N",
        })
    }
}

/// Galeri örneğindeki komut satırının geçmişi en fazla bu kadar satır tutar.
const HISTORY_LIMIT: usize = 12;

/// Galerinin durumu.
#[derive(Debug, Clone)]
pub struct Gallery {
    pub page: Page,
    pub row: usize,
    pub toggles: [bool; 3],
    pub checked: bool,
    pub opacity: f32,
    pub text: String,
    pub crs: Option<Crs>,
    pub command: String,
    pub history: Vec<Entry>,
    pub command_expanded: bool,

    /// Öznitelikler sayfasının örnek kayıtları: bütün alan türlerini
    /// kullanan bir yapı envanteri.
    pub schema: Vec<Field>,
    pub records: Vec<Vec<Value>>,
    /// İnceleyicide gösterilen kayıt.
    pub record: usize,
    pub inspector: inspector::State,
    pub query: Query,
    pub mode: SelectionMode,
    pub sort: Option<(usize, SortOrder)>,
    pub search: String,

    /// Seçici örneklerinin değerleri.
    pub picked_date: Option<Date>,
    pub picked_moment: Option<DateTime>,
    pub picked_time: Option<Time>,
    pub picked_region: Option<usize>,

    /// Ağaç örneği: açık klasörler, işaretli dosyalar ve seçili satır.
    pub project_open: [bool; 4],
    pub project_checked: [bool; 5],
    pub project_selected: Option<ProjectRow>,
    /// Panel örneğindeki panellerin kapalı olması.
    pub panels_collapsed: [bool; 2],
    /// Öznitelikler ızgarası örneği: kapalı bölümler, seçili katman ve yükseklik.
    pub sheet_closed: [bool; 2],
    pub sheet_layer: usize,
    pub sheet_height: f64,
    /// Kayan pencere örneği: pencereler, açık yakalama türleri ve arkadaki
    /// düğmeye basılma sayısı.
    pub panes: Windows<DemoPane>,
    pub snaps: [bool; 4],
    pub stage_presses: usize,
    /// Sihirbaz ve özellikler penceresi örneklerinde adım ve bölüm.
    pub wizard_step: usize,
    pub section: usize,
    /// Belge sekmeleri örneği: açık çizimler, açık olanı ve yeni çizimin
    /// adındaki sayı.
    pub documents: Vec<Document>,
    pub document: usize,
    pub untitled: usize,
    /// Sekmeli yuva örneğinin yerleşimi.
    pub docks: Docks<DemoPanel>,
    /// Girdiler sayfasının değerleri: uzunluk ve kalınlık metre, ölçek
    /// yüzde, konum metre, açılar derece.
    pub length: f64,
    pub thickness: f64,
    pub scale: f64,
    pub position: [f64; 3],
    pub rotation: f64,
    pub bearing: f64,
    /// Renk seçici örneği: çizgi ve saydam dolgu rengi.
    pub stroke: iced::Color,
    pub fill: iced::Color,
    /// Renk rampası örneği ve seçili durak.
    pub ramp: Ramp,
    pub stop: usize,
    /// Görünüm alanı örneği: düzen, bakışlar ve stiller.
    pub views: Views,
    pub cameras: [Camera; 4],
    pub shadings: [Shading; 4],
    /// Şerit örneği: seçili rampa ve şeridin daraltılmış olması.
    pub ribbon_ramp: usize,
    pub ribbon_collapsed: bool,
    /// Temel kontroller: anahtarlar, seçim yöntemi, birim sistemi, nüfus
    /// aralığı ve etiketler.
    pub switches: [bool; 2],
    pub method: usize,
    pub unit_system: usize,
    pub population: (f64, f64),
    pub tags: Vec<String>,
    /// Form örneği: pafta adı, kâğıt, enlem ve antet.
    pub sheet_name: String,
    pub paper: usize,
    pub latitude: f64,
    pub title_block: bool,
    /// Sanal tablo örneğinde seçili kayıt.
    pub parcel: usize,
    /// Taşınabilir ağaç örneği: satırlar, seçili satır ve adlandırılan
    /// satırla yazılan ad.
    pub outline: Vec<Outline>,
    pub outline_selected: Option<usize>,
    pub outline_renaming: Option<(usize, String)>,
    /// Mini araç çubuğu örneği: seçili şekil, kilitli ve silinmiş şekiller.
    pub shape: Option<usize>,
    pub shapes_locked: [bool; 3],
    pub shapes_deleted: [bool; 3],
    /// Dairesel menü örneği: menü açık mı, seçilen araç.
    pub radial_open: bool,
    pub radial_tool: Tool,
    /// Cetvel örneği: kâğıttaki kılavuzlar (mm) ve yakınlık (yüzde).
    pub ruler_guides: Guides,
    pub ruler_zoom: f64,
    /// Pusula örneği: haritanın saat yönünde dönüşü, derece.
    pub map_rotation: f64,
    /// Zaman çizelgesi örnekleri ve son zamanlayıcı anı.
    pub project: Playback,
    pub animation: Playback,
    pub last_tick: Option<iced::time::Instant>,
    /// Lejant örneği: daraltılmış mı, gizlenen satırlar.
    pub legend_collapsed: bool,
    pub legend_hidden: [bool; 6],
    /// Varlık tarayıcısı örneği.
    pub asset: Option<usize>,
    pub asset_search: String,
    pub asset_category: Option<String>,
    pub asset_view: assets::View,
    /// Karşılaştırma perdesi örneği.
    pub compare_split: f32,
    pub compare_vertical: bool,
}

/// Belge sekmeleri örneğindeki açık çizim.
#[derive(Debug, Clone, PartialEq)]
pub struct Document {
    pub name: String,
    pub dirty: bool,
}

/// Belge sekmeleri örneğinin açılıştaki çizimleri.
fn sample_documents() -> Vec<Document> {
    [
        ("Kadıköy imar planı.dwg", true),
        ("Moda sahil düzenlemesi.dwg", false),
        ("Fikirtepe kentsel dönüşüm alanı, 3. revizyon.dwg", false),
        ("Hasanpaşa.dxf", true),
        ("Rasimpaşa.dxf", false),
        ("Acıbadem.dwg", false),
    ]
    .into_iter()
    .map(|(name, dirty)| Document {
        name: name.to_owned(),
        dirty,
    })
    .collect()
}

/// Mini araç çubuğu örneğindeki şekiller: adı, alandaki yeri ve rengi.
pub const SHAPES: [(&str, iced::Rectangle, iced::Color); 3] = [
    (
        "Parsel 1204/7",
        iced::Rectangle {
            x: 48.0,
            y: 110.0,
            width: 170.0,
            height: 96.0,
        },
        iced::Color::from_rgb8(0xe2, 0xa9, 0x3b),
    ),
    (
        "Yapı adası",
        iced::Rectangle {
            x: 268.0,
            y: 36.0,
            width: 140.0,
            height: 150.0,
        },
        iced::Color::from_rgb8(0xc7, 0x7d, 0xd8),
    ),
    (
        "Park",
        iced::Rectangle {
            x: 458.0,
            y: 132.0,
            width: 150.0,
            height: 84.0,
        },
        iced::Color::from_rgb8(0x8f, 0xc9, 0x5a),
    ),
];

/// Proje takvimi örneğinin aşamaları: gün, ay, yıl ve adı.
pub const MILESTONES: [(u8, u8, i32, &str); 7] = [
    (15, 3, 2024, "İhale"),
    (1, 6, 2024, "Yapı ruhsatı"),
    (10, 9, 2024, "Yıkım"),
    (20, 1, 2025, "Temel"),
    (5, 8, 2025, "Kaba inşaat"),
    (10, 2, 2026, "İnce işler"),
    (30, 9, 2026, "İskân"),
];

/// Animasyon örneğinin kare sayısı (24 kare/saniye, 10 saniye) ve anahtar
/// kareleri.
pub const ANIMATION_FRAMES: u16 = 240;
pub const KEYFRAMES: [u16; 5] = [0, 60, 120, 180, 240];

/// Tarihin Türkiye saatiyle gece yarısı, Unix saniyesi.
pub fn unix(day: u8, month: u8, year: i32) -> f64 {
    Date::new(year, month, day).map_or(0.0, |date| {
        (date.days_since_epoch() * 86_400) as f64 - 3.0 * 3_600.0
    })
}

/// Proje takvimi: 2024 başından 2026 sonuna; 1× hızda saniyede bir ay,
/// adım bir hafta. Oynatma başı temel atılan günde.
fn project_playback() -> Playback {
    let mut playback = Playback::new((unix(1, 1, 2024), unix(31, 12, 2026)), 30.0 * 86_400.0)
        .with_step(7.0 * 86_400.0);

    playback.update(timeline::Event::Seek(unix(20, 1, 2025)));
    playback
}

/// Cetvel örneğinin açılıştaki kılavuzları: A5 kâğıdın 10 mm kenar payı.
fn sample_guides() -> Guides {
    let mut guides = Guides::new();

    for guide in [
        Guide::vertical(10.0),
        Guide::vertical(200.0),
        Guide::horizontal(10.0),
        Guide::horizontal(138.0),
    ] {
        guides.push(guide);
    }

    guides
}

/// Sanal tablo örneğinin kayıt sayısı.
pub const PARCELS: usize = 100_000;

/// Sanal tablo örneğinin mahalleleri ve kullanım kararları.
const DISTRICTS: [&str; 8] = [
    "Caferağa",
    "Osmanağa",
    "Rasimpaşa",
    "Moda",
    "Fikirtepe",
    "Acıbadem",
    "Hasanpaşa",
    "Koşuyolu",
];
const USES: [&str; 5] = ["Konut", "Ticaret", "Karma", "Kamu", "Yeşil alan"];

/// Sanal tablo örneğinin kaydı. Sıra numarasından üretilir; hiçbiri
/// bellekte tutulmaz, tablo yalnızca görünen satırları ister.
#[derive(Debug, Clone, PartialEq)]
pub struct Parcel {
    pub block: usize,
    pub lot: usize,
    pub district: &'static str,
    pub usage: &'static str,
    /// Metrekare.
    pub area: f64,
}

/// Sıra numarasındaki kayıt; aynı numara hep aynı kaydı verir.
pub fn parcel(index: usize) -> Parcel {
    // splitmix64: sıra numarasını dağıtır.
    let mut hash = (index as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
    hash = (hash ^ (hash >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    hash = (hash ^ (hash >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    hash ^= hash >> 31;

    Parcel {
        block: 101 + index / 40,
        lot: 1 + index % 40,
        district: DISTRICTS[index / 2_000 % DISTRICTS.len()],
        usage: USES[(hash % USES.len() as u64) as usize],
        area: 120.0 + ((hash >> 8) % 480_000) as f64 / 100.0,
    }
}

/// Taşınabilir ağaç örneğinin satırı. Satırlar derinlik sırasıyla dizilir;
/// bir satırın altındakiler, ardından gelen daha derin satırlardır.
#[derive(Debug, Clone, PartialEq)]
pub struct Outline {
    pub name: String,
    pub depth: usize,
    pub folder: bool,
    pub open: bool,
    pub visible: bool,
    pub locked: bool,
    /// Katmanın rengi; klasörlerde kullanılmaz.
    pub color: iced::Color,
}

/// Taşınabilir ağaç örneğinin açılıştaki satırları: bir yapı projesinin
/// katman grupları.
fn sample_outline() -> Vec<Outline> {
    let row = |name: &str, depth, folder, color: u32| Outline {
        name: name.to_owned(),
        depth,
        folder,
        open: true,
        visible: true,
        locked: false,
        color: iced::Color::from_rgb8((color >> 16) as u8, (color >> 8) as u8, color as u8),
    };

    let mut rows = vec![
        row("Mimari", 0, true, 0),
        row("Duvarlar", 1, false, 0xd9_d4_c7),
        row("Kapılar", 1, false, 0xe2_a9_3b),
        row("Pencereler", 1, false, 0x4c_9b_e8),
        row("Taşıyıcı sistem", 0, true, 0),
        row("Kolonlar", 1, false, 0xd6_5d_4f),
        row("Kirişler", 1, false, 0xc7_7d_d8),
        row("Tesisat", 0, true, 0),
        row("Elektrik", 1, false, 0xf0_d2_4a),
        row("Sıhhi tesisat", 1, false, 0x3f_b8_a8),
        row("Ölçüler", 0, false, 0x8f_c9_5a),
        row("Notlar", 0, false, 0xa0_a4_ab),
    ];

    rows[5].locked = true;
    rows[7].open = false;
    rows[11].visible = false;
    rows
}

/// Satırın altındakilerle birlikte kapladığı aralık.
fn subtree(rows: &[Outline], index: usize) -> std::ops::Range<usize> {
    let depth = rows[index].depth;
    let end = rows[index + 1..]
        .iter()
        .position(|row| row.depth <= depth)
        .map_or(rows.len(), |offset| index + 1 + offset);

    index..end
}

/// `source` satırını altındakilerle birlikte `target` satırının önüne,
/// ardına ya da (klasörse) içine son çocuk olarak taşır; satırın yeni yerini
/// döndürür. Satır kendi altına taşınamaz.
pub fn move_outline(
    rows: &mut Vec<Outline>,
    source: usize,
    target: usize,
    place: Place,
) -> Option<usize> {
    if source >= rows.len() || target >= rows.len() {
        return None;
    }

    let range = subtree(rows, source);

    if range.contains(&target) || (place == Place::Into && !rows[target].folder) {
        return None;
    }

    let block: Vec<Outline> = rows.drain(range.clone()).collect();
    let target = if target > source {
        target - block.len()
    } else {
        target
    };
    let (at, depth) = match place {
        Place::Before => (target, rows[target].depth),
        Place::After => (subtree(rows, target).end, rows[target].depth),
        Place::Into => {
            rows[target].open = true;
            (subtree(rows, target).end, rows[target].depth + 1)
        }
    };
    let shift = depth as isize - block[0].depth as isize;

    rows.splice(
        at..at,
        block.into_iter().map(|mut row| {
            row.depth = row.depth.saturating_add_signed(shift);
            row
        }),
    );

    Some(at)
}

impl Default for Gallery {
    fn default() -> Self {
        Self {
            page: Page::Colors,
            row: 1,
            toggles: [true, true, false],
            checked: true,
            opacity: 0.8,
            text: String::new(),
            crs: Some(Crs::WebMercator),
            command: String::new(),
            history: vec![
                Entry::Output("Galeri örneği: komutlar burada çalıştırılmaz.".to_owned()),
                Entry::Input("ALAN".to_owned()),
                Entry::Output(
                    "Alan: Kapalı alan çizer. Sağ tık veya Esc alanı kapatır.".to_owned(),
                ),
                Entry::Input("merhaba".to_owned()),
                Entry::Error("Bilinmeyen komut: MERHABA.".to_owned()),
            ],
            command_expanded: false,
            schema: building_schema(),
            records: building_records(),
            record: 0,
            inspector: {
                let mut inspector = inspector::State::new();
                inspector.inspect(Some(0));
                inspector
            },
            query: Query {
                conditions: vec![Condition {
                    field: 2,
                    operator: Operator::GreaterOrEqual,
                    value: "10".to_owned(),
                }],
                ..Query::default()
            },
            mode: SelectionMode::New,
            sort: None,
            search: String::new(),
            picked_date: Date::new(2026, 10, 29),
            picked_moment: None,
            picked_time: Time::new(9, 0, 0),
            picked_region: Some(0),
            project_open: [true, true, true, false],
            project_checked: [true, true, true, false, true],
            project_selected: Some(ProjectRow::File(2)),
            panels_collapsed: [false, false],
            sheet_closed: [false, false],
            sheet_layer: 0,
            sheet_height: 2.5,
            panes: demo_panes(),
            snaps: [true, true, false, true],
            stage_presses: 0,
            wizard_step: 1,
            section: 0,
            documents: sample_documents(),
            document: 0,
            untitled: 1,
            docks: demo_docks(),
            length: 12.5,
            thickness: 0.018,
            scale: 100.0,
            position: [412_350.25, 4_523_180.5, 42.0],
            rotation: 30.0,
            bearing: 135.0,
            stroke: iced::Color::from_rgb8(0xe2, 0xa9, 0x3b),
            fill: iced::Color::from_rgba8(0x4c, 0x9b, 0xe8, 0.4),
            ramp: Ramp::presets()
                .into_iter()
                .find(|(name, _)| *name == "Arazi")
                .map(|(_, ramp)| ramp)
                .unwrap_or_else(|| Ramp::new([])),
            stop: 2,
            views: Views::new(Arrangement::Quad),
            cameras: Camera::ALL,
            shadings: [
                Shading::Wireframe,
                Shading::Hidden,
                Shading::Hidden,
                Shading::ShadedEdges,
            ],
            ribbon_ramp: 0,
            ribbon_collapsed: false,
            switches: [true, false],
            method: 0,
            unit_system: 0,
            population: (1_000_000.0, 6_000_000.0),
            tags: vec!["park".to_owned(), "yeşil alan".to_owned()],
            sheet_name: "Kadıköy imar planı".to_owned(),
            paper: 1,
            latitude: 40.99,
            title_block: true,
            parcel: 0,
            outline: sample_outline(),
            outline_selected: Some(1),
            outline_renaming: None,
            shape: Some(0),
            shapes_locked: [false, false, true],
            shapes_deleted: [false; 3],
            radial_open: false,
            radial_tool: Tool::Select,
            ruler_guides: sample_guides(),
            ruler_zoom: 100.0,
            map_rotation: 30.0,
            project: project_playback(),
            animation: Playback::new((0.0, f64::from(ANIMATION_FRAMES)), 24.0).with_step(1.0),
            last_tick: None,
            legend_collapsed: false,
            legend_hidden: [false, false, false, false, true, false],
            asset: Some(0),
            asset_search: String::new(),
            asset_category: None,
            asset_view: assets::View::Grid,
            compare_split: 0.5,
            compare_vertical: false,
        }
    }
}

impl Gallery {
    /// Zaman çizelgesi örneklerinden biri oynatılıyor mu; zamanlayıcı
    /// yalnızca o sürece çalışır.
    pub fn is_playing(&self) -> bool {
        self.project.playing || self.animation.playing
    }

    /// Komut kutusu örneğine yazılanı geçmişe ekler; komutlar yalnızca asıl
    /// komut kutusunda çalışır.
    fn echo(&mut self, command: &str) {
        if command.is_empty() {
            return;
        }

        self.history.push(Entry::Input(command.to_owned()));
        self.history.push(Entry::Output(
            "Bu kutu yalnızca bir örnek; komutlar sayfanın altındaki asıl komut kutusunda çalışır."
                .to_owned(),
        ));

        let excess = self.history.len().saturating_sub(HISTORY_LIMIT);
        self.history.drain(..excess);
    }

    /// Örnek etkileşimini uygular. Uygulamanın komut satırına yazılacak bir
    /// satır döndürebilir.
    pub fn update(&mut self, demo: Demo) -> Option<String> {
        match demo {
            Demo::Pressed(name) => return Some(format!("Galeri: \"{name}\" düğmesine basıldı.")),
            Demo::RowSelected(row) => self.row = row,
            Demo::Toggled(index) => {
                if let Some(toggle) = self.toggles.get_mut(index) {
                    *toggle = !*toggle;
                }
            }
            Demo::Checked(checked) => self.checked = checked,
            Demo::OpacityChanged(opacity) => self.opacity = opacity,
            Demo::TextChanged(text) => self.text = text,
            Demo::CrsSelected(crs) => self.crs = Some(crs),
            Demo::CommandChanged(command) => self.command = command,
            Demo::CommandSubmitted => {
                let command = std::mem::take(&mut self.command);
                self.echo(command.trim());
            }
            Demo::CommandRun(command) => {
                self.command.clear();
                self.echo(&command);
            }
            Demo::CommandExpanded(expanded) => self.command_expanded = expanded,
            Demo::RecordSelected(record) => {
                if record < self.records.len() {
                    self.record = record;
                    self.inspector.inspect(Some(record as u64));
                }
            }
            Demo::Inspector(event) => match self.inspector.update(event) {
                Some(inspector::Action::Change { id, value }) => {
                    let valid = self
                        .schema
                        .get(id)
                        .is_some_and(|field| field.editable && field.validate(&value).is_ok());

                    if valid
                        && let Some(slot) = self
                            .records
                            .get_mut(self.record)
                            .and_then(|record| record.get_mut(id))
                    {
                        *slot = value;
                    }
                }
                Some(inspector::Action::Pick(_)) => {
                    self.inspector.stop_picking();
                    return Some(
                        "Galeri: haritadan seçim Giriş sekmesindeki nesne inceleyicide çalışır."
                            .to_owned(),
                    );
                }
                Some(inspector::Action::Navigate { object, .. }) => {
                    return Some(format!(
                        "Galeri: #{object} nesnesine gitme Giriş sekmesindeki nesne inceleyicide çalışır."
                    ));
                }
                Some(inspector::Action::CancelPick) | None => {}
            },
            Demo::QueryEdited(edit) => self.query.apply(edit, &self.schema),
            Demo::ModeSelected(mode) => self.mode = mode,
            Demo::Sorted(column) => {
                self.sort = match self.sort {
                    Some((current, order)) if current == column => Some((column, order.reversed())),
                    _ => Some((column, SortOrder::Ascending)),
                };
            }
            Demo::SearchChanged(search) => self.search = search,
            Demo::DatePicked(date) => self.picked_date = date,
            Demo::MomentPicked(moment) => self.picked_moment = moment,
            Demo::TimePicked(time) => self.picked_time = time,
            Demo::RegionPicked(region) => self.picked_region = region,
            Demo::ProjectToggled(folder) => {
                if let Some(open) = self.project_open.get_mut(folder) {
                    *open = !*open;
                }
            }
            Demo::ProjectFolderChecked(folder) => {
                let files = PROJECT_FILES.get(folder).copied().unwrap_or_default();
                let all = files.iter().all(|&file| self.project_checked[file]);

                for &file in files {
                    self.project_checked[file] = !all;
                }
            }
            Demo::ProjectFileChecked(file) => {
                if let Some(checked) = self.project_checked.get_mut(file) {
                    *checked = !*checked;
                }
            }
            Demo::ProjectSelected(row) => self.project_selected = Some(row),
            Demo::PanelToggled(panel) => {
                if let Some(collapsed) = self.panels_collapsed.get_mut(panel) {
                    *collapsed = !*collapsed;
                }
            }
            Demo::SheetToggled(section) => {
                if let Some(closed) = self.sheet_closed.get_mut(section) {
                    *closed = !*closed;
                }
            }
            Demo::SheetLayer(layer) => self.sheet_layer = layer,
            // Ondalık virgül noktadır; sayı olmayan ya da sıfırdan büyük
            // olmayan değer alınmaz, hücre eski değere döner.
            Demo::SheetHeight(text) => {
                if let Ok(height) = text.trim().replacen(',', ".", 1).parse::<f64>()
                    && height > 0.0
                {
                    self.sheet_height = height;
                }
            }
            Demo::Window(event) => self.panes.update(event),
            Demo::PanesReset => self.panes = demo_panes(),
            Demo::SnapToggled(kind) => {
                if let Some(snap) = self.snaps.get_mut(kind) {
                    *snap = !*snap;
                }
            }
            Demo::StagePressed => self.stage_presses += 1,
            // Bildirimler uygulamanın kuyruğuna eklenir.
            Demo::Notify(_) => {}
            Demo::WizardStep(step) => self.wizard_step = step.min(2),
            Demo::SectionSelected(section) => self.section = section,
            Demo::DocumentSelected(document) => {
                self.document = document.min(self.documents.len().saturating_sub(1));
            }
            Demo::DocumentClosed(document) => {
                if document < self.documents.len() {
                    let closed = self.documents.remove(document);

                    if document < self.document || self.document >= self.documents.len() {
                        self.document = self.document.saturating_sub(1);
                    }

                    // Son belge de kapanınca örnek baştan açılır.
                    if self.documents.is_empty() {
                        self.documents = sample_documents();
                        self.document = 0;
                    }

                    if closed.dirty {
                        return Some(format!(
                            "Galeri: \"{}\" kaydedilmeden kapatıldı; gerçek uygulamada önce \
                             onay istenir.",
                            closed.name
                        ));
                    }
                }
            }
            Demo::DocumentMoved(from, to) => {
                if from < self.documents.len() && to < self.documents.len() {
                    let document = self.documents.remove(from);
                    self.documents.insert(to, document);

                    self.document = match self.document {
                        current if current == from => to,
                        current if from < current && current <= to => current - 1,
                        current if to <= current && current < from => current + 1,
                        current => current,
                    };
                }
            }
            Demo::DocumentAdded => {
                self.documents.push(Document {
                    name: format!("Adsız {}.dwg", self.untitled),
                    dirty: false,
                });
                self.untitled += 1;
                self.document = self.documents.len() - 1;
            }
            Demo::DocumentSaved => {
                if let Some(document) = self.documents.get_mut(self.document) {
                    document.dirty = false;
                }
            }
            Demo::Dock(event) => self.docks.update(event),
            Demo::DockReset => self.docks = demo_docks(),
            Demo::Length(length) => self.length = length,
            Demo::Thickness(thickness) => self.thickness = thickness,
            Demo::Scale(scale) => self.scale = scale,
            Demo::Position(position) => self.position = position,
            Demo::Rotation(rotation) => self.rotation = rotation,
            Demo::Bearing(bearing) => self.bearing = bearing,
            Demo::Stroke(color) => self.stroke = color,
            Demo::Fill(color) => self.fill = color,
            Demo::RampChanged(ramp, stop) => {
                self.ramp = ramp;
                self.stop = stop;
            }
            Demo::Views(event) => self.views.update(event),
            Demo::Camera(view, camera) => {
                if let Some(slot) = self.cameras.get_mut(view) {
                    *slot = camera;
                }
            }
            Demo::Shading(view, shading) => {
                if let Some(slot) = self.shadings.get_mut(view) {
                    *slot = shading;
                }
            }
            Demo::RibbonRamp(ramp) => self.ribbon_ramp = ramp,
            Demo::RibbonCollapsed => self.ribbon_collapsed = !self.ribbon_collapsed,
            Demo::Switched(index, on) => {
                if let Some(switch) = self.switches.get_mut(index) {
                    *switch = on;
                }
            }
            Demo::Method(method) => self.method = method,
            Demo::UnitSystem(system) => self.unit_system = system,
            Demo::Population(range) => self.population = range,
            Demo::Tags(tags) => self.tags = tags,
            Demo::SheetName(name) => self.sheet_name = name,
            Demo::Paper(paper) => self.paper = paper,
            Demo::Latitude(latitude) => self.latitude = latitude,
            Demo::TitleBlock(shown) => self.title_block = shown,
            Demo::ParcelSelected(parcel) => self.parcel = parcel.min(PARCELS - 1),
            Demo::OutlineMoved(source, target, place) => {
                self.outline_renaming = None;

                match move_outline(&mut self.outline, source, target, place) {
                    Some(at) => self.outline_selected = Some(at),
                    None => {
                        return Some(
                            "Galeri: satır yalnızca bir klasörün içine taşınabilir.".to_owned(),
                        );
                    }
                }
            }
            Demo::OutlineOpened(row) => {
                if let Some(row) = self.outline.get_mut(row) {
                    row.open = !row.open;
                }
            }
            Demo::OutlineShown(row) => {
                if let Some(row) = self.outline.get_mut(row) {
                    row.visible = !row.visible;
                }
            }
            Demo::OutlineLocked(row) => {
                if let Some(row) = self.outline.get_mut(row) {
                    row.locked = !row.locked;
                }
            }
            Demo::OutlineSelected(row) => self.outline_selected = Some(row),
            Demo::OutlineRename(row) => {
                if let Some(outline) = self.outline.get(row) {
                    self.outline_selected = Some(row);
                    self.outline_renaming = Some((row, outline.name.clone()));
                }
            }
            Demo::OutlineInput(text) => {
                if let Some((_, name)) = &mut self.outline_renaming {
                    *name = text;
                }
            }
            Demo::OutlineRenamed => {
                if let Some((row, name)) = self.outline_renaming.take()
                    && let Some(outline) = self.outline.get_mut(row)
                    && !name.trim().is_empty()
                {
                    outline.name = name.trim().to_owned();
                }
            }
            Demo::OutlineCancelled => self.outline_renaming = None,
            Demo::ShapeSelected(shape) => {
                self.shape = shape.filter(|shape| !self.shapes_deleted[*shape]);
            }
            Demo::ShapeLocked(shape) => {
                if let Some(locked) = self.shapes_locked.get_mut(shape) {
                    *locked = !*locked;
                }
            }
            Demo::ShapeDeleted(shape) => {
                if !self.shapes_locked.get(shape).copied().unwrap_or(true) {
                    self.shapes_deleted[shape] = true;
                    self.shape = None;

                    return Some(format!("Galeri: \"{}\" silindi.", SHAPES[shape].0));
                }
            }
            Demo::ShapesReset => {
                self.shapes_deleted = [false; 3];
                self.shape = Some(0);
            }
            Demo::RadialOpened => self.radial_open = true,
            Demo::RadialClosed => self.radial_open = false,
            Demo::RadialChosen(tool) => self.radial_tool = tool,
            Demo::RulerGuide(event) => self.ruler_guides.update(event),
            Demo::RulerZoom(zoom) => self.ruler_zoom = zoom.clamp(25.0, 800.0),
            Demo::RulerGuidesCleared => self.ruler_guides.clear(),
            Demo::MapRotated(rotation) => self.map_rotation = rotation.rem_euclid(360.0),
            Demo::NorthReset => self.map_rotation = 0.0,
            // Oynatma başlayınca ilk tur eski andan saymasın.
            Demo::Project(event) => {
                self.project.update(event);
                self.last_tick = None;
            }
            Demo::Animation(event) => {
                self.animation.update(event);
                self.last_tick = None;
            }
            Demo::LegendCollapsed => self.legend_collapsed = !self.legend_collapsed,
            Demo::LegendItem(item) => {
                if let Some(hidden) = self.legend_hidden.get_mut(item) {
                    *hidden = !*hidden;
                }
            }
            Demo::AssetSelected(asset) => self.asset = Some(asset),
            Demo::AssetActivated(asset) => {
                self.asset = Some(asset);

                return Some(format!(
                    "Galeri: \"{}\" kullanıldı; Giriş sekmesindeki Kitaplık panelinde bloğu \
                     haritaya yerleştirir.",
                    crate::view::library::BLOCKS
                        .get(asset)
                        .map_or("", |(name, _, _)| *name)
                ));
            }
            Demo::AssetSearch(search) => self.asset_search = search,
            Demo::AssetCategory(category) => self.asset_category = category,
            Demo::AssetView(view) => self.asset_view = view,
            Demo::CompareMoved(split) => self.compare_split = split,
            Demo::CompareVertical(vertical) => self.compare_vertical = vertical,
            Demo::Tick(now) => {
                if let Some(last) = self.last_tick {
                    let elapsed = now.saturating_duration_since(last);

                    self.project.advance(elapsed);
                    self.animation.advance(elapsed);
                }

                self.last_tick = Some(now);
            }
        }

        None
    }

    /// Örnek tablonun satırları: aramaya uyan kayıtlar, sıralı.
    pub fn rows(&self) -> Vec<usize> {
        let search = self.search.trim();

        let mut rows: Vec<usize> = (0..self.records.len())
            .filter(|&record| {
                search.is_empty()
                    || self.schema.iter().enumerate().any(|(field, definition)| {
                        text::contains(&definition.format(self.value(record, field)), search)
                    })
            })
            .collect();

        if let Some((column, order)) = self.sort {
            rows.sort_by(|&a, &b| {
                let ordering = self
                    .value(a, column)
                    .compare(self.value(b, column))
                    .then(a.cmp(&b));

                match order {
                    SortOrder::Ascending => ordering,
                    SortOrder::Descending => ordering.reverse(),
                }
            });
        }

        rows
    }

    /// Kaydın alan değeri; yoksa boş.
    pub fn value(&self, record: usize, field: usize) -> &Value {
        static NULL: Value = Value::Null;

        self.records
            .get(record)
            .and_then(|values| values.get(field))
            .unwrap_or(&NULL)
    }

    /// Sorguyu sağlayan kayıtlar.
    pub fn matches(&self, record: usize) -> bool {
        self.records
            .get(record)
            .is_some_and(|values| self.query.matches(&self.schema, values))
    }
}

/// Örnek yapı envanterinin alanları: kentos-ui'nin bütün alan türleri.
fn building_schema() -> Vec<Field> {
    vec![
        Field::text("Ad")
            .required()
            .description("Yapının tabelada ve ruhsatta geçen adı."),
        Field::choice("Kullanım", ["Konut", "Ticaret", "Karma", "Kamu", "Sanayi"])
            .description("İmar planındaki kullanım kararı."),
        Field::integer("Kat")
            .between(1, 120)
            .description("Zemin üstü kat sayısı."),
        Field::real("Yükseklik", 1)
            .unit("m")
            .description("Zeminden en üst noktaya yükseklik."),
        Field::boolean("Asansör"),
        Field::range("Doluluk", 0.0, 100.0, 5.0)
            .unit("%")
            .description("Kullanılan bağımsız bölümlerin oranı."),
        Field::date("Ruhsat").description("Yapı ruhsatının verildiği tarih."),
        Field::time("Açılış").description("Ziyaretçilere açıldığı saat."),
        Field::datetime("Son denetim").description("Son yapı denetiminin tarihi ve saati."),
        Field::object("Şehir", sample::CITIES).description("Yapının bulunduğu il."),
        Field::text("Ada/parsel")
            .read_only()
            .description("Tapu kaydındaki ada ve parsel; kadastrodan gelir."),
        Field::text("Not").multiline(),
    ]
}

/// Örnek yapı kayıtları. Şehir başvuruları Şehirler katmanının
/// numaralarıdır (1 İstanbul, 2 Ankara, 3 İzmir, 4 Bursa, 5 Antalya).
fn building_records() -> Vec<Vec<Value>> {
    let date = |day, month, year| Date::new(year, month, day).map_or(Value::Null, Value::Date);
    let time = |hour, minute| Time::new(hour, minute, 0).map_or(Value::Null, Value::Time);
    let moment = |day, month, year, hour, minute| match (
        Date::new(year, month, day),
        Time::new(hour, minute, 0),
    ) {
        (Some(date), Some(time)) => Value::DateTime(DateTime::new(date, time)),
        _ => Value::Null,
    };
    let city = |id| Value::Object(ObjectId(id));

    vec![
        vec![
            "Kuleli İş Merkezi".into(),
            "Karma".into(),
            Value::Integer(24),
            Value::Real(96.5),
            true.into(),
            Value::Real(85.0),
            date(12, 3, 2019),
            time(8, 30),
            moment(18, 9, 2026, 14, 30),
            city(1),
            "1204/7".into(),
            "Zemin katta ticari birimler var.\nÇatıda güneş panelleri kurulu.".into(),
        ],
        vec![
            "Çınar Konutları".into(),
            "Konut".into(),
            Value::Integer(12),
            Value::Real(38.4),
            true.into(),
            Value::Real(95.0),
            date(4, 11, 2021),
            Value::Null,
            moment(2, 9, 2026, 10, 0),
            city(2),
            "845/3".into(),
        ],
        vec![
            "Liman Deposu".into(),
            "Sanayi".into(),
            Value::Integer(2),
            Value::Real(11.0),
            false.into(),
            Value::Real(60.0),
            date(21, 6, 2012),
            time(7, 0),
            Value::Null,
            city(3),
            "77/12".into(),
        ],
        vec![
            "Kent Kütüphanesi".into(),
            "Kamu".into(),
            Value::Integer(4),
            Value::Real(18.2),
            true.into(),
            Value::Real(70.0),
            date(15, 1, 2016),
            time(9, 0),
            moment(20, 8, 2026, 16, 45),
            city(4),
            "310/1".into(),
        ],
        vec![
            "Sahil Çarşısı".into(),
            "Ticaret".into(),
            Value::Integer(3),
            Value::Real(12.5),
            false.into(),
            Value::Null,
            date(30, 5, 2018),
            time(10, 0),
            Value::Null,
            city(5),
            "56/9".into(),
        ],
        vec![
            "Kule Rezidans".into(),
            "Konut".into(),
            Value::Integer(38),
            Value::Real(131.0),
            true.into(),
            Value::Real(80.0),
            date(8, 8, 2023),
            Value::Null,
            moment(11, 9, 2026, 11, 20),
            city(1),
            "1204/8".into(),
        ],
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn names(rows: &[Outline]) -> Vec<(usize, &str)> {
        rows.iter()
            .map(|row| (row.depth, row.name.as_str()))
            .collect()
    }

    #[test]
    fn outline_rows_move_with_their_children() {
        let mut rows = sample_outline();

        // Taşıyıcı sistem, çocuklarıyla birlikte Mimari'nin önüne.
        assert_eq!(move_outline(&mut rows, 4, 0, Place::Before), Some(0));
        assert_eq!(
            names(&rows)[..7],
            [
                (0, "Taşıyıcı sistem"),
                (1, "Kolonlar"),
                (1, "Kirişler"),
                (0, "Mimari"),
                (1, "Duvarlar"),
                (1, "Kapılar"),
                (1, "Pencereler"),
            ]
        );

        // Notlar, kapalı Tesisat klasörünün içine: son çocuğu olur, klasör
        // açılır.
        assert_eq!(move_outline(&mut rows, 11, 7, Place::Into), Some(10));
        assert_eq!(
            names(&rows)[7..],
            [
                (0, "Tesisat"),
                (1, "Elektrik"),
                (1, "Sıhhi tesisat"),
                (1, "Notlar"),
                (0, "Ölçüler"),
            ]
        );
        assert!(rows[7].open);

        // Kapılar, Mimari grubunun ardına: en üst düzeye çıkar.
        assert_eq!(move_outline(&mut rows, 5, 3, Place::After), Some(6));
        assert_eq!(
            names(&rows)[3..7],
            [
                (0, "Mimari"),
                (1, "Duvarlar"),
                (1, "Pencereler"),
                (0, "Kapılar"),
            ]
        );
    }

    #[test]
    fn outline_rows_cannot_move_under_themselves_or_into_layers() {
        let mut rows = sample_outline();
        let before = rows.clone();

        assert_eq!(move_outline(&mut rows, 0, 2, Place::After), None);
        assert_eq!(move_outline(&mut rows, 0, 0, Place::Into), None);
        assert_eq!(move_outline(&mut rows, 10, 1, Place::Into), None);
        assert_eq!(move_outline(&mut rows, 99, 1, Place::Before), None);
        assert_eq!(rows, before);
    }

    #[test]
    fn parcels_are_generated_from_their_index() {
        assert_eq!(parcel(0), parcel(0));
        assert_eq!((parcel(0).block, parcel(0).lot), (101, 1));
        assert_eq!((parcel(41).block, parcel(41).lot), (102, 2));
        assert!((0..PARCELS).step_by(997).all(|index| {
            let area = parcel(index).area;
            (120.0..4_920.0).contains(&area)
        }));
    }
}
