//! Uygulama mesajları ve arayüzün sabit seçenek listeleri.

use std::fmt;

use iced::keyboard::Modifiers;

use kentos_rc::attribute::query;
use kentos_rc::icon::Icon;
use kentos_rc::spatial::model_space;
use kentos_rc::spatial::{FeatureRef, SelectionMode, Tool};
use kentos_rc::widget::inspector;

use crate::gallery::{Demo, Page};
use crate::table::Column;

/// Kullanıcının yaptığı her şey.
#[derive(Debug, Clone)]
pub enum Message {
    ModelSpace(model_space::Event),
    ToolSelected(Tool),

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
    LayerVisibility(usize, bool),
    LayerOpacity(usize, f32),
    LayerActivated(usize),
    ZoomToLayer(usize),
    ShowAllLayers,
    HideAllLayers,

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

    // Komut satırı
    CommandInput(String),
    CommandSubmitted,

    ModifiersChanged(Modifiers),
    Escape,
    Tick,
    Quit,
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
