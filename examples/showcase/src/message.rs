//! Uygulama mesajları ve arayüzün sabit seçenek listeleri.

use std::fmt;

use iced::Color;
use iced::keyboard::Modifiers;

use kentos_rc::attribute::query;
use kentos_rc::icon::Icon;
use kentos_rc::spatial::model_space::{self, Backdrop};
use kentos_rc::spatial::{FeatureRef, LonLat, SelectionMode, Tool};
use kentos_rc::theme::typography::{self, Typography};
use kentos_rc::theme::{Accent, Mode};
use kentos_rc::widget::docking::{self, Docks, Side};
use kentos_rc::widget::floating::{self, Placement};
use kentos_rc::widget::tree_view::Place;
use kentos_rc::widget::{inspector, toast};

use crate::gallery::{Demo, Page};
use crate::import::Source;
use crate::layer_tree::NodeId;
use crate::properties::{Edit, Section};
use crate::table::Column;

/// Kullanıcının yaptığı her şey.
#[derive(Debug, Clone)]
pub enum Message {
    ModelSpace(model_space::Event),
    ToolSelected(Tool),

    // Model ve düzen sekmeleri
    SheetSelected(usize),
    SheetClosed(usize),
    SheetMoved(usize, usize),
    SheetAdded,
    /// Açık düzenin harita çerçevesinde gezinme.
    SheetView(model_space::Event),

    // Görünüm
    ZoomIn,
    ZoomOut,
    FitAll,
    ResetView,
    /// Seçili öğelerin tamamına odaklanır.
    FocusSelection,
    /// Tek bir öğeye odaklanır (nesne inceleyicideki "Odakla").
    FocusFeature(FeatureRef),

    // Katmanlar
    LayerOpacity(usize, f32),
    LayerColor(usize, Color),
    /// Çizgi ve alan kenarının kalınlığı (piksel).
    LayerStroke(usize, f32),
    LayerActivated(usize),
    ZoomToLayer(usize),
    ShowAllLayers,
    HideAllLayers,

    // Katman ağacı
    TreeSelected(NodeId),
    /// Grubu ya da alt katmanları açar/kapatır.
    TreeToggled(NodeId),
    /// Ağaçta sürüklenip bırakılan girdi: kaynağın ve hedefin anahtarı,
    /// yer.
    TreeMoved(usize, usize, Place),
    /// Katmanın kilidini ve seçilebilirliğini değiştirir.
    LayerLocked(usize),
    LayerSelectable(usize),
    /// Grubu ya da katmanı yerinde yeniden adlandırma.
    RenameStarted(NodeId),
    RenameInput(String),
    RenameSubmitted,
    RenameCancelled,
    /// F2: ağaçta seçili düğüm varsa yeniden adlandırır, yoksa komut
    /// geçmişini açar.
    F2Pressed,
    /// Düğümün onay kutusu: görünürlük.
    TreeChecked(NodeId, bool),
    /// Grubu (ya da `None` ile bütün ağacı) iç içe açar veya kapatır.
    TreeExpandAll(Option<usize>, bool),
    /// Yalnızca düğümü gösterir, diğerlerini gizler.
    ShowOnly(NodeId),
    /// Katmanın bütün alt katmanlarını gösterir ya da gizler.
    SublayersShown(usize, bool),
    ZoomToNode(NodeId),
    /// Düğümün bütün öğelerini seçer.
    SelectNode(NodeId),
    OpenTable(usize),
    /// Çizimler katmanındaki bütün öğeleri siler.
    ClearDrawings,

    // Bağlam menüleri
    SelectFeature(FeatureRef, SelectionMode),
    CenterAt(LonLat),
    CopyCoordinates(LonLat),
    CopyRow(FeatureRef),

    // Seçim ve ölçüm
    SelectAll,
    InvertSelection,
    ClearSelection,
    DeleteSelection,
    ClearMeasurement,
    /// Nesne inceleyicide seçimdeki sonraki (ya da önceki) öğe.
    SelectionStep(bool),
    Inspector(inspector::Event),

    // Öznitelik tablosu
    TableToggled,
    TableRowPressed(FeatureRef),
    TableSearch(String),
    TableSelectedOnly,
    /// Sütun başlığına tıklandı: sütuna göre sırala ya da yönü çevir.
    TableSort(Column),
    FilterCleared,

    // Öznitelikle seç ve filtre
    QueryOpened(QueryPurpose),
    /// Sorgu penceresini verilen katman için açar.
    QueryOpenedFor(QueryPurpose, usize),
    QueryLayerSelected(usize),
    QueryEdited(query::Edit),
    QueryModeSelected(SelectionMode),
    QueryApplied,
    QueryClosed,

    // Ayarlar
    Toggle(Setting),
    ToggleTheme,

    // Şerit, menüler ve iletişim kutuları
    RibbonTabSelected(RibbonTab),
    /// Şeridi daraltır ya da gösterir (Ctrl+F1).
    RibbonCollapsed,
    /// Hızlı erişim düğmesini gösterir ya da gizler.
    QuickToggled(usize),
    AppMenuToggled,
    AppMenuHovered(AppCommand),
    AppCommandPressed(AppCommand),
    ExportPressed(&'static str),
    RecentPressed(usize),
    HelpToggled,
    CommandListRequested,

    // Galeri
    GalleryPageSelected(Page),
    Gallery(Demo),

    // Komut kutusu
    CommandInput(String),
    CommandSubmitted,
    /// Öneri listesinden seçilen komut, adıyla.
    CommandRun(String),
    /// Komut kutusu odakta değilken yazılan metin; kutuya eklenir.
    CommandTyped(String),
    /// Komut geçmişini açar ya da kapatır (F2).
    CommandHistoryToggled,
    /// Etkin komutun istemindeki seçenek.
    Keyword(Keyword),

    // Renk ve zemin
    /// Temayı değiştirir: koyu, aydınlık, gece, yüksek karşıtlık.
    ThemeSelected(Mode),
    /// Vurgu rengini değiştirir: hazır renk ya da #RRGGBB.
    AccentChanged(Accent),
    /// Harita zeminini değiştirir.
    BackdropChanged(Backdrop),

    // Yazı
    /// Yazı ayarını değiştirir: aile, eş aralıklı aile ya da boyut.
    TypographyChanged(Typography),
    /// Yazıyı bir adım büyütür, küçültür ya da varsayılana döndürür.
    TextSize(SizeStep),

    // Kayan araç pencereleri
    /// Pencere sürüklendi, boyutlandırıldı, öne geldi, daraltıldı ya da
    /// kapandı.
    Window(floating::Event<Pane>),
    /// Pencereyi açar; açıksa kapatır.
    PaneToggled(Pane),
    /// Katman stili penceresini verilen katman için açar.
    StyleOpened(usize),
    /// Koordinata git penceresinin alanları.
    GoToLatitude(String),
    GoToLongitude(String),
    /// Görünümü yazılan koordinata ortalar.
    GoToCentered,
    /// Yazılan koordinatı süren çizime nokta olarak ekler.
    GoToPlaced,
    /// Ölçümün kenarlarını ve toplamını panoya yazar.
    CopyMeasurement,

    // Bildirimler
    /// Bildirim kapandı: süresi doldu, kapatıldı ya da eylemi yapıldı.
    ToastClosed(toast::Id),
    /// Son silinen çizimleri geri koyar.
    UndoDelete,

    // Arka plandaki işler
    /// İşleri bir adım ilerletir.
    JobTick,
    JobCancelled(u64),
    JobRetried(u64),
    /// Biten işi görev listesinden kaldırır.
    JobDismissed(u64),
    /// Biten bütün işleri görev listesinden kaldırır.
    JobsCleared,
    /// Uzamsal dizini yeniden oluşturur.
    IndexRequested,

    // Onay ve uyarı şeridi
    /// Açık onay kutusunu onaylar (Enter ya da onay düğmesi).
    ConfirmAccepted,
    /// Açık onay kutusundan vazgeçer.
    ConfirmCancelled,
    /// Enter: bir metin kutusu kullanmadıysa açık onay kutusunu onaylar.
    EnterPressed,
    /// Salt okunur veri şeridini kapatır.
    BannerDismissed,

    // Veri içe aktarma sihirbazı
    ImportOpened,
    ImportSource(Source),
    /// Boylam (X) sütunu.
    ImportX(usize),
    /// Enlem (Y) sütunu.
    ImportY(usize),
    ImportSystem(usize),
    ImportName(String),
    ImportBack,
    /// Sonraki adım; son adımda içe aktarmayı başlatır.
    ImportNext,
    ImportClosed,

    // Katman özellikleri
    PropertiesOpened(usize),
    PropertiesSection(Section),
    PropertiesEdited(Edit),
    PropertiesApplied,
    /// Uygular ve kapatır.
    PropertiesAccepted,
    /// Taslağı atar ve kapatır.
    PropertiesClosed,

    // Yuva
    /// Yuvadaki sürükleme, boyutlandırma, daraltma ve kapatma.
    Dock(docking::Event<DockPanel>),
    /// Paneli açar ya da kapatır.
    PanelToggled(DockPanel),
    /// Paneli gösterir: kapalıysa açar, arkadaysa öne getirir.
    PanelShown(DockPanel),

    // Durum çubuğu
    CoordinateFormatSelected(CoordinateFormat),
    /// Ölçeği 1:N yapar.
    ScaleSelected(f64),

    ModifiersChanged(Modifiers),
    Escape,
    Tick,
    Quit,
}

/// Yuvadaki paneller.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DockPanel {
    Layers,
    /// Özellikler: seçili öğenin nesne inceleyicisi.
    Details,
    /// Aktif katmanın öznitelik tablosu.
    Table,
    /// Arka plandaki işler: ilerleme, iptal, yeniden deneme.
    Tasks,
}

impl DockPanel {
    pub const ALL: [DockPanel; 4] = [
        DockPanel::Layers,
        DockPanel::Details,
        DockPanel::Table,
        DockPanel::Tasks,
    ];

    pub fn title(self) -> &'static str {
        match self {
            DockPanel::Layers => "Katmanlar",
            DockPanel::Details => "Özellikler",
            DockPanel::Table => "Öznitelik tablosu",
            DockPanel::Tasks => "Görevler",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            DockPanel::Layers => Icon::Layers,
            DockPanel::Details => Icon::Properties,
            DockPanel::Table => Icon::Table,
            DockPanel::Tasks => Icon::Progress,
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            DockPanel::Layers => "Katman ağacı: görünürlük, gruplar, opaklık.",
            DockPanel::Details => "Seçili öğenin öznitelikleri ve geometrisi.",
            DockPanel::Table => "Aktif katmanın kayıtları: arama, filtre, sıralama, seçim.",
            DockPanel::Tasks => "Arka plandaki işler: ilerleme, iptal ve yeniden deneme.",
        }
    }

    /// Ayar dosyasındaki adı.
    pub fn key(self) -> &'static str {
        match self {
            DockPanel::Layers => "katmanlar",
            DockPanel::Details => "ozellikler",
            DockPanel::Table => "tablo",
            DockPanel::Tasks => "gorevler",
        }
    }

    pub fn parse(key: &str) -> Option<Self> {
        Self::ALL.into_iter().find(|panel| panel.key() == key)
    }

    /// Hiç açılmamışsa açıldığı kenar.
    pub fn side(self) -> Side {
        match self {
            DockPanel::Layers | DockPanel::Details => Side::Right,
            DockPanel::Table | DockPanel::Tasks => Side::Bottom,
        }
    }

    /// Açılıştaki yerleşim: sağda katmanlar ve özellikler alt alta, altta
    /// öznitelik tablosu ve arkasında görevler.
    pub fn layout() -> Docks<DockPanel> {
        let mut docks = Docks::new();

        docks.dock(DockPanel::Layers, Side::Right);
        docks.split(DockPanel::Details, Side::Right);
        docks.dock(DockPanel::Table, Side::Bottom);
        docks.dock(DockPanel::Tasks, Side::Bottom);
        docks.show(DockPanel::Table, Side::Bottom);
        docks.update(docking::Event::Shared(Side::Right, vec![5.0, 6.0]));
        docks.set_size(Side::Right, DOCK_WIDTH);
        docks.set_size(Side::Bottom, TABLE_HEIGHT);
        docks
    }
}

/// Sağ alanın ve alt alanın açılıştaki boyutu (12 piksellik gövde metnine
/// göre).
pub const DOCK_WIDTH: f32 = 332.0;
pub const TABLE_HEIGHT: f32 = 252.0;

/// Harita üstündeki kayan araç pencereleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    /// Ölç aracının sonuçları; araçla birlikte açılır ve kapanır.
    Measure,
    /// Koordinata git: enlem ve boylam yazarak ortalar ya da nokta ekler.
    GoTo,
    /// Aktif katmanın rengi, opaklığı ve çizgi kalınlığı.
    Style,
}

impl Pane {
    pub fn title(self) -> &'static str {
        match self {
            Pane::Measure => "Ölçüm",
            Pane::GoTo => "Koordinata git",
            Pane::Style => "Katman stili",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Pane::Measure => Icon::Measure,
            Pane::GoTo => Icon::Target,
            Pane::Style => Icon::Drop,
        }
    }

    /// Varsayılan genişlik, 12 piksellik gövde metnine göre.
    pub fn width(self) -> f32 {
        match self {
            Pane::Measure => 248.0,
            Pane::GoTo => 252.0,
            Pane::Style => 268.0,
        }
    }

    /// İlk açıldığı yer: ölçüm ve koordinata git üst kenar boyunca yan
    /// yana, katman stili sağda; sağ üstteki ViewCube ve gezinme çubuğu
    /// açıkta kalır.
    pub fn placement(self) -> Placement {
        let gap = floating::GAP;

        match self {
            Pane::Measure => Placement::top_left(gap, gap),
            Pane::GoTo => {
                Placement::top_left(2.0 * gap + typography::scaled(Pane::Measure.width()), gap)
            }
            Pane::Style => Placement::top_right(96.0, gap),
        }
    }
}

/// Onay bekleyen iş.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Confirmation {
    /// Çizimler katmanındaki bütün öğeleri silmek.
    ClearDrawings,
    /// Çizimler kaydedilmeden çıkmak.
    Quit,
}

/// Yazı boyutunun adımı (Ctrl +, Ctrl −, Ctrl 0).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SizeStep {
    Larger,
    Smaller,
    Default,
}

/// Seçenek bekleyen komut: istem seçenekleri gösterir, seçilince biter.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pending {
    /// YAZITIPI: yazı ailesi ya da eş aralıklı aile.
    Typeface,
    /// PUNTO: gövde metninin boyutu.
    TextSize,
    /// VURGU: vurgu rengi; hazır renk ya da #RRGGBB.
    Accent,
    /// ZEMIN: harita zemini.
    Backdrop,
}

/// Çizim ve ölçüm istemlerinin seçenekleri.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Keyword {
    /// Son noktayı kaldırır.
    Undo,
    /// Çoklu çizgiyi tamamlar ya da çizgi zincirini bitirir.
    Finish,
    /// Alanı kapatır.
    Close,
    /// Ölçümü temizler.
    Clear,
    /// Haritadan seçimi ya da yarım kalan çizimi bırakır.
    Cancel,
}

/// Durum çubuğundaki koordinatın biçimi.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CoordinateFormat {
    /// 41.00820° K  28.97840° D
    #[default]
    Decimal,
    /// 41°00'29.5" K  28°58'42.2" D
    Dms,
    /// Web Mercator düzlemi, metre: X 3.225.861  Y 5.013.551
    Projected,
}

impl CoordinateFormat {
    pub const ALL: [CoordinateFormat; 3] = [
        CoordinateFormat::Decimal,
        CoordinateFormat::Dms,
        CoordinateFormat::Projected,
    ];

    pub fn label(self) -> &'static str {
        match self {
            CoordinateFormat::Decimal => "Ondalık derece",
            CoordinateFormat::Dms => "Derece, dakika, saniye",
            CoordinateFormat::Projected => "Web Mercator, metre",
        }
    }
}

/// Sorgu penceresinin amacı.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryPurpose {
    /// Eşleşen öğeleri seçer ("öznitelikle seç").
    Select,
    /// Öznitelik tablosunda yalnızca eşleşen kayıtları gösterir.
    Filter,
}

/// Açılıp kapatılabilen görüntüleme ayarları.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Setting {
    Grid,
    Snap,
    FullCrosshair,
    Labels,
    ViewCube,
}

impl Setting {
    /// Durum çubuğundaki sırasıyla.
    pub const ALL: [Setting; 5] = [
        Setting::Grid,
        Setting::Snap,
        Setting::FullCrosshair,
        Setting::Labels,
        Setting::ViewCube,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Setting::Grid => "Izgara",
            Setting::Snap => "Yakalama",
            Setting::FullCrosshair => "Artı imleç",
            Setting::Labels => "Etiketler",
            Setting::ViewCube => "ViewCube",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            Setting::Grid => Icon::Grid,
            Setting::Snap => Icon::Magnet,
            Setting::FullCrosshair => Icon::Crosshair,
            Setting::Labels => Icon::Type,
            Setting::ViewCube => Icon::Cube,
        }
    }

    /// Klavye kısayolu ya da komut satırı karşılığı.
    pub fn shortcut(self) -> &'static str {
        match self {
            Setting::Grid => "F7",
            Setting::Snap => "F3",
            Setting::FullCrosshair => "ARTI",
            Setting::Labels => "ETIKET",
            Setting::ViewCube => "3B",
        }
    }
}

/// Şerit sekmeleri. "Giriş" model alanının araçlarını, "Galeri" bileşen
/// kataloğunu içerir; diğerleri çerçevenin CAD düzenini tamamlayan yer
/// tutuculardır.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RibbonTab {
    Home,
    Insert,
    Annotate,
    Analyze,
    View,
    Manage,
    Output,
    Gallery,
}

impl RibbonTab {
    pub const ALL: [RibbonTab; 8] = [
        RibbonTab::Home,
        RibbonTab::Insert,
        RibbonTab::Annotate,
        RibbonTab::Analyze,
        RibbonTab::View,
        RibbonTab::Manage,
        RibbonTab::Output,
        RibbonTab::Gallery,
    ];
}

impl fmt::Display for RibbonTab {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            RibbonTab::Home => "Giriş",
            RibbonTab::Insert => "Ekle",
            RibbonTab::Annotate => "Açıklama",
            RibbonTab::Analyze => "Analiz",
            RibbonTab::View => "Görünüm",
            RibbonTab::Manage => "Yönet",
            RibbonTab::Output => "Çıktı",
            RibbonTab::Gallery => "Galeri",
        })
    }
}

/// Uygulama menüsünün büyük komutları.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppCommand {
    New,
    Open,
    Save,
    SaveAs,
    Export,
    Print,
}

impl AppCommand {
    pub const ALL: [AppCommand; 6] = [
        AppCommand::New,
        AppCommand::Open,
        AppCommand::Save,
        AppCommand::SaveAs,
        AppCommand::Export,
        AppCommand::Print,
    ];

    pub fn label(self) -> &'static str {
        match self {
            AppCommand::New => "Yeni",
            AppCommand::Open => "Aç",
            AppCommand::Save => "Kaydet",
            AppCommand::SaveAs => "Farklı kaydet",
            AppCommand::Export => "Dışa aktar",
            AppCommand::Print => "Yazdır",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            AppCommand::New => "Görünümü, seçimi ve ölçümü sıfırlar",
            AppCommand::Open => "Kayıtlı bir çizimi açar",
            AppCommand::Save => "Çizimi geçerli dosyaya yazar",
            AppCommand::SaveAs => "Yeni bir adla kaydeder",
            AppCommand::Export => "PDF, PNG, DXF veya GeoJSON",
            AppCommand::Print => "Yazıcıya veya çiziciye gönderir",
        }
    }

    pub fn icon(self) -> Icon {
        match self {
            AppCommand::New => Icon::DocumentNew,
            AppCommand::Open => Icon::Folder,
            AppCommand::Save => Icon::Save,
            AppCommand::SaveAs => Icon::SaveAs,
            AppCommand::Export => Icon::Export,
            AppCommand::Print => Icon::Print,
        }
    }

    /// Ayrıntı bölmesinde alt menüsü olan komut.
    pub fn has_submenu(self) -> bool {
        self == AppCommand::Export
    }
}

/// Dışa aktarma biçimleri: ad ve kısa açıklama.
pub const EXPORT_FORMATS: [(&str, &str); 4] = [
    ("PDF", "Sayfa düzeniyle vektör çıktı"),
    ("PNG", "Geçerli görünümün raster görüntüsü"),
    ("DXF", "CAD programları için çizim değişim biçimi"),
    ("GeoJSON", "Öznitelikleriyle birlikte vektör katmanlar"),
];

/// Son kullanılan çizimler: ad, klasör, zaman. İlki şu an açık olan örnek
/// veridir; diğerleri menünün görünümünü tamamlayan örneklerdir.
pub const RECENT_DRAWINGS: [(&str, &str, &str); 5] = [
    ("Türkiye örnek verisi.kcad", "Örnekler", "Açık"),
    ("Ankara imar planı.kcad", "Projeler / Ankara", "Dün"),
    ("İzmir altyapı ağı.kcad", "Projeler / İzmir", "3 gün önce"),
    ("Bursa kadastro paftaları.kcad", "Kadastro", "Geçen hafta"),
    (
        "Kayseri ulaşım ana planı.kcad",
        "Projeler / Kayseri",
        "12 Ağustos",
    ),
];
