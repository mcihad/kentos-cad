//! Komut kutusu: komut kataloğu ve yazılanın çözümlenmesi.
//!
//! Komutlar büyük/küçük harf ve Türkçe karakter duyarsızdır: "çizgi",
//! "Çizgi" ve "CIZGI" aynı komuttur. AutoCAD'in İngilizce kısaltmaları da
//! (L, PL, C, ZE...) kabul edilir. Komut yerine "enlem, boylam" yazılabilir:
//! çizim sürerken nokta ekler, yoksa görünümü oraya ortalar.

use kentos_rc::icon::Icon;
use kentos_rc::spatial::{LonLat, Tool};
use kentos_rc::theme::Mode;
use kentos_rc::widget::command_line;

use crate::message::{Pane, Setting};

/// Çözümlenmiş komut.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Command {
    Tool(Tool),
    ZoomIn,
    ZoomOut,
    FitAll,
    ResetView,
    FocusSelection,
    Toggle(Setting),
    /// `None` temayı değiştirir.
    Theme(Option<Mode>),
    ShowAllLayers,
    HideAllLayers,
    ListLayers,
    ActiveLayer,
    /// Aktif katmanın tablodaki bütün kayıtlarını seçer.
    SelectAll,
    /// Aktif katmanda seçimi tersine çevirir.
    InvertSelection,
    /// Öznitelik tablosunu açar ya da kapatır.
    AttributeTable,
    /// "Öznitelikle seç" penceresini açar.
    SelectByAttributes,
    /// "Tabloyu filtrele" penceresini açar.
    Filter,
    /// Seçili çizimleri siler.
    Delete,
    /// Seçimi ve ölçümü temizler.
    Clear,
    /// Kayan araç penceresini açar ya da öne getirir.
    Pane(Pane),
    /// Veri içe aktarma sihirbazını açar.
    Import,
    New,
    Help,
    /// Vurgu rengini seçtirir.
    Accent,
    /// Harita zeminini seçtirir.
    Backdrop,
    /// Yazı ailesini seçtirir.
    Typeface,
    /// Yazı boyutunu seçtirir.
    TextSize,
    Quit,
}

type Info = command_line::Command<'static>;

/// Komutlar ve komut kutusundaki karşılıkları: ad, okunur ad, kısaltmalar,
/// ikon ve açıklama. Tablonun sırası öneri listesinin sırasıdır.
const COMMANDS: &[(Command, Info)] = &[
    (
        Command::Tool(Tool::Line),
        Info::new("CIZGI", "Çizgi")
            .aliases(&["LINE", "L"])
            .icon(Icon::Line)
            .description(
                "Art arda doğru parçaları çizer; her parça öncekinin bittiği yerden başlar.",
            ),
    ),
    (
        Command::Tool(Tool::Polyline),
        Info::new("CCIZGI", "Çoklu çizgi")
            .aliases(&["PLINE", "PL"])
            .icon(Icon::Polyline)
            .description("Tek parça, çok köşeli çizgi çizer. Bitir ya da boşken Enter tamamlar."),
    ),
    (
        Command::Tool(Tool::Polygon),
        Info::new("ALAN", "Alan")
            .aliases(&["POLYGON", "POL"])
            .icon(Icon::Polygon)
            .description("Kapalı alan çizer. Kapat ya da boşken Enter alanı kapatır."),
    ),
    (
        Command::Tool(Tool::Rectangle),
        Info::new("DIKDORTGEN", "Dikdörtgen")
            .aliases(&["RECTANG", "REC"])
            .icon(Icon::Rectangle)
            .description("Karşılıklı iki köşesinden dikdörtgen çizer."),
    ),
    (
        Command::Tool(Tool::Circle),
        Info::new("DAIRE", "Daire")
            .aliases(&["CIRCLE", "C"])
            .icon(Icon::Circle)
            .description("Merkezinden ve çember üzerindeki bir noktadan daire çizer."),
    ),
    (
        Command::Tool(Tool::Point),
        Info::new("NOKTA", "Nokta")
            .aliases(&["POINT", "PO"])
            .icon(Icon::Point)
            .description("Tıklanan ya da yazılan konuma nokta koyar."),
    ),
    (
        Command::Tool(Tool::Select),
        Info::new("SEC", "Seç")
            .aliases(&["SECIM", "SELECT"])
            .icon(Icon::Select)
            .description("Öğeleri tıklayarak ya da sürükleyerek pencereyle seçer."),
    ),
    (
        Command::Tool(Tool::Pan),
        Info::new("PAN", "Kaydır")
            .aliases(&["KAYDIR"])
            .icon(Icon::Pan)
            .description("Haritayı sürükleyerek kaydırır."),
    ),
    (
        Command::Tool(Tool::Measure),
        Info::new("OLC", "Ölç")
            .aliases(&["MESAFE", "DIST"])
            .icon(Icon::Measure)
            .description("Tıklanan noktalar arasındaki mesafeyi ve toplam uzunluğu ölçer."),
    ),
    (
        Command::ZoomIn,
        Info::new("YAKINLASTIR", "Yakınlaştır")
            .aliases(&["ZI", "ZOOMIN", "+"])
            .icon(Icon::ZoomIn)
            .description("Görünümü bir adım yakınlaştırır."),
    ),
    (
        Command::ZoomOut,
        Info::new("UZAKLASTIR", "Uzaklaştır")
            .aliases(&["ZU", "ZOOMOUT", "-"])
            .icon(Icon::ZoomOut)
            .description("Görünümü bir adım uzaklaştırır."),
    ),
    (
        Command::FitAll,
        Info::new("TUMUNU", "Tümünü gör")
            .aliases(&["ZE", "EXTENTS"])
            .icon(Icon::ZoomExtents)
            .description("Görünür bütün katmanları ekrana sığdırır."),
    ),
    (
        Command::ResetView,
        Info::new("SIFIRLA", "Başlangıç görünümü")
            .icon(Icon::Home)
            .description("Görünümü açılıştaki merkeze ve ölçeğe döndürür."),
    ),
    (
        Command::FocusSelection,
        Info::new("ODAKLA", "Seçime odaklan")
            .aliases(&["SINIR"])
            .icon(Icon::Target)
            .description("Seçili öğelerin tamamını ekrana sığdırır."),
    ),
    (
        Command::Toggle(Setting::Grid),
        Info::new("IZGARA", "Izgara")
            .aliases(&["GRID"])
            .icon(Icon::Grid)
            .description("Enlem-boylam ızgarasını açar ya da kapatır. Kısayolu F7."),
    ),
    (
        Command::Toggle(Setting::Snap),
        Info::new("YAKALA", "Nesne yakalama")
            .aliases(&["SNAP", "OSNAP"])
            .icon(Icon::Magnet)
            .description(
                "Çizerken öğelerin köşelerine yakalamayı açar ya da kapatır. Kısayolu F3.",
            ),
    ),
    (
        Command::Toggle(Setting::FullCrosshair),
        Info::new("ARTI", "Artı imleç")
            .aliases(&["IMLEC", "CROSSHAIR"])
            .icon(Icon::Crosshair)
            .description("İmleci ekran boyu artıya çevirir ya da küçültür."),
    ),
    (
        Command::Toggle(Setting::Labels),
        Info::new("ETIKET", "Etiketler")
            .aliases(&["ETIKETLER", "LABELS"])
            .icon(Icon::Type)
            .description("Öğelerin adlarını haritada gösterir ya da gizler."),
    ),
    (
        Command::Toggle(Setting::ViewCube),
        Info::new("3B", "ViewCube")
            .aliases(&["KUP", "CUBE", "VIEWCUBE"])
            .icon(Icon::Cube)
            .description("Sağ üstteki yön küpünü gösterir ya da gizler."),
    ),
    (
        Command::Theme(None),
        Info::new("TEMA", "Temayı değiştir")
            .icon(Icon::Contrast)
            .description("Koyu ve aydınlık tema arasında geçiş yapar."),
    ),
    (
        Command::Theme(Some(Mode::Light)),
        Info::new("AYDINLIK", "Aydınlık tema")
            .aliases(&["LIGHT"])
            .icon(Icon::Contrast)
            .description("Kâğıt zeminli aydınlık temaya geçer."),
    ),
    (
        Command::Theme(Some(Mode::Dark)),
        Info::new("KOYU", "Koyu tema")
            .aliases(&["DARK"])
            .icon(Icon::Contrast)
            .description("CAD programlarının grafit temasına geçer."),
    ),
    (
        Command::ShowAllLayers,
        Info::new("KATGOSTER", "Katmanları göster")
            .icon(Icon::Eye)
            .description("Bütün katmanları ve grupları görünür yapar."),
    ),
    (
        Command::HideAllLayers,
        Info::new("KATGIZLE", "Katmanları gizle")
            .icon(Icon::EyeOff)
            .description("Bütün katmanları gizler."),
    ),
    (
        Command::ListLayers,
        Info::new("KATMANLAR", "Katman listesi")
            .aliases(&["LAYERS"])
            .icon(Icon::Layers)
            .description("Görünür katmanların adlarını komut geçmişine yazar."),
    ),
    (
        Command::ActiveLayer,
        Info::new("KATMAN", "Aktif katman")
            .aliases(&["AKTIFKATMAN"])
            .icon(Icon::Layers)
            .description("Aktif katmanın adını komut geçmişine yazar."),
    ),
    (
        Command::SelectAll,
        Info::new("TUMUNUSEC", "Tümünü seç")
            .aliases(&["SELECTALL"])
            .icon(Icon::SelectAll)
            .description("Aktif katmanda filtreye uyan bütün öğeleri seçer. Kısayolu Ctrl+A."),
    ),
    (
        Command::InvertSelection,
        Info::new("TERSSEC", "Seçimi tersine çevir")
            .aliases(&["INVERT"])
            .icon(Icon::InvertSelection)
            .description("Aktif katmanda seçili olmayan öğeleri seçer, seçilileri bırakır."),
    ),
    (
        Command::AttributeTable,
        Info::new("TABLO", "Öznitelik tablosu")
            .aliases(&["OZNITELIK", "TABLE"])
            .icon(Icon::Table)
            .description("Öznitelik tablosunu açar ya da kapatır."),
    ),
    (
        Command::SelectByAttributes,
        Info::new("SORGU", "Öznitelikle seç")
            .aliases(&["QSELECT"])
            .icon(Icon::Filter)
            .description("Koşullara uyan öğeleri seçer; seçime ekler ya da çıkarır."),
    ),
    (
        Command::Filter,
        Info::new("FILTRE", "Tabloyu filtrele")
            .aliases(&["FILTER"])
            .icon(Icon::Filter)
            .description("Öznitelik tablosunda yalnızca koşula uyan kayıtları gösterir."),
    ),
    (
        Command::Delete,
        Info::new("SIL", "Sil")
            .aliases(&["ERASE", "E"])
            .icon(Icon::Eraser)
            .description("Seçili çizimleri siler. Kısayolu Delete."),
    ),
    (
        Command::Clear,
        Info::new("TEMIZLE", "Temizle")
            .aliases(&["IPTAL"])
            .icon(Icon::ClearSelection)
            .description("Seçimi, ölçümü ve yarım kalan çizimi temizler."),
    ),
    (
        Command::Pane(Pane::GoTo),
        Info::new("GIT", "Koordinata git")
            .aliases(&["KOORDINAT", "GOTO"])
            .icon(Icon::Target)
            .description(
                "Enlem ve boylam yazıp görünümü ortalayan ya da çizime nokta ekleyen pencereyi açar.",
            ),
    ),
    (
        Command::Pane(Pane::Style),
        Info::new("STIL", "Katman stili")
            .aliases(&["STYLE", "SEMBOL"])
            .icon(Icon::Drop)
            .description("Aktif katmanın rengini, opaklığını ve çizgi kalınlığını değiştiren pencereyi açar."),
    ),
    (
        Command::Import,
        Info::new("ICEAKTAR", "Veri içe aktar")
            .aliases(&["IMPORT", "EKLE"])
            .icon(Icon::Import)
            .description("CSV ya da GeoJSON dosyasını adım adım katman olarak ekleyen sihirbazı açar."),
    ),
    (
        Command::New,
        Info::new("YENI", "Yeni")
            .aliases(&["NEW"])
            .icon(Icon::DocumentNew)
            .description("Görünümü, seçimi ve ölçümü sıfırlar."),
    ),
    (
        Command::Help,
        Info::new("YARDIM", "Kısayollar")
            .aliases(&["HELP", "?"])
            .icon(Icon::Help)
            .description(
                "Klavye kısayollarını ve komut kutusunun tuşlarını gösterir. Kısayolu F1.",
            ),
    ),
    (
        Command::Accent,
        Info::new("VURGU", "Vurgu rengi")
            .aliases(&["ACCENT", "RENK"])
            .icon(Icon::Drop)
            .description(
                "Seçim, etkin araç ve düğmelerin rengini seçtirir: sekiz hazır renk ya da #RRGGBB.",
            ),
    ),
    (
        Command::Backdrop,
        Info::new("ZEMIN", "Harita zemini")
            .aliases(&["ARKAPLAN", "BACKGROUND"])
            .icon(Icon::Layers)
            .description("Model alanının zeminini seçtirir: temaya uyan, arduvaz, siyah ya da kâğıt."),
    ),
    (
        Command::Typeface,
        Info::new("YAZITIPI", "Yazı tipi")
            .aliases(&["FONT"])
            .icon(Icon::Type)
            .description("Arayüzün ve koordinatların yazı ailesini seçtirir: IBM Plex, Inter, Plus Jakarta Sans, JetBrains Mono."),
    ),
    (
        Command::TextSize,
        Info::new("PUNTO", "Yazı boyutu")
            .aliases(&["YAZIBOYUTU", "FONTSIZE"])
            .icon(Icon::Type)
            .description("Arayüz metninin boyutunu seçtirir. Kısayolları Ctrl +, Ctrl − ve Ctrl 0."),
    ),
    (
        Command::Quit,
        Info::new("CIKIS", "Çıkış")
            .aliases(&["QUIT"])
            .icon(Icon::Power)
            .description("KentOS CAD'i kapatır."),
    ),
];

/// Yazılan komutu çözümler; tanınmayan komutta `None`.
pub fn parse(input: &str) -> Option<Command> {
    COMMANDS
        .iter()
        .find(|(_, info)| info.is_spelled(input))
        .map(|(command, _)| *command)
}

/// Komutun arayüzde gösterilen adı (ör. araç ipuçlarındaki "Komut: CIZGI").
pub fn name(command: Command) -> &'static str {
    COMMANDS
        .iter()
        .find(|(candidate, _)| *candidate == command)
        .map_or("", |(_, info)| info.name)
}

/// Komut kutusunun öneri kataloğu.
pub fn catalog() -> impl Iterator<Item = Info> {
    COMMANDS.iter().map(|(_, info)| *info)
}

/// Boşlukları kırpar, Türkçe harfleri ASCII karşılıklarına çevirir ve büyük
/// harfe dönüştürür; tanınmayan komutu gösterirken kullanılır.
pub fn normalize(input: &str) -> String {
    input
        .trim()
        .chars()
        .map(|character| match character {
            'ç' | 'Ç' => 'C',
            'ğ' | 'Ğ' => 'G',
            'ı' | 'İ' | 'i' => 'I',
            'ö' | 'Ö' => 'O',
            'ş' | 'Ş' => 'S',
            'ü' | 'Ü' => 'U',
            other => other.to_ascii_uppercase(),
        })
        .collect()
}

/// "enlem, boylam" biçiminde yazılmış koordinat. Ondalık ayırıcı nokta ise
/// sayılar virgülle, virgül ise noktalı virgülle ya da boşlukla ayrılır:
/// "39.92, 32.85", "39,92; 32,85", "39,92 32,85".
pub fn coordinates(input: &str) -> Option<LonLat> {
    let input = input.trim();

    let (lat, lon) = if let Some(pair) = input.split_once(';') {
        pair
    } else if input.matches(',').count() == 1 && input.contains('.') {
        input.split_once(',')?
    } else {
        let mut parts = input.split_whitespace();
        let pair = (parts.next()?, parts.next()?);

        if parts.next().is_some() {
            return None;
        }

        pair
    };

    let number = |text: &str| text.trim().replace(',', ".").parse::<f64>().ok();
    let (lat, lon) = (number(lat)?, number(lon)?);

    ((-90.0..=90.0).contains(&lat) && (-180.0..=180.0).contains(&lon))
        .then(|| LonLat::new(lon, lat))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turkish_spelling_and_case_are_ignored() {
        for input in ["çizgi", "Çizgi", "CIZGI", "  cizgi "] {
            assert_eq!(parse(input), Some(Command::Tool(Tool::Line)), "{input}");
        }

        assert_eq!(parse("yakınlaştır"), Some(Command::ZoomIn));
        assert_eq!(parse("ölç"), Some(Command::Tool(Tool::Measure)));
        assert_eq!(parse("dikdörtgen"), Some(Command::Tool(Tool::Rectangle)));
        assert_eq!(parse("çıkış"), Some(Command::Quit));
    }

    #[test]
    fn autocad_aliases_are_accepted() {
        assert_eq!(parse("l"), Some(Command::Tool(Tool::Line)));
        assert_eq!(parse("pl"), Some(Command::Tool(Tool::Polyline)));
        assert_eq!(parse("c"), Some(Command::Tool(Tool::Circle)));
        assert_eq!(parse("ze"), Some(Command::FitAll));
        assert_eq!(parse("osnap"), Some(Command::Toggle(Setting::Snap)));
        assert_eq!(parse("qselect"), Some(Command::SelectByAttributes));
    }

    #[test]
    fn attribute_commands() {
        assert_eq!(parse("öznitelik"), Some(Command::AttributeTable));
        assert_eq!(parse("tümünüseç"), Some(Command::SelectAll));
        assert_eq!(parse("terssec"), Some(Command::InvertSelection));
        assert_eq!(parse("filtre"), Some(Command::Filter));
        assert_eq!(parse("sorgu"), Some(Command::SelectByAttributes));
    }

    #[test]
    fn unknown_commands_are_rejected() {
        assert_eq!(parse("merhaba"), None);
        assert_eq!(parse(""), None);
        // Alanı kapatan "Kapat" seçeneğiyle karışmasın diye çıkışın
        // kısaltması değildir.
        assert_eq!(parse("kapat"), None);
    }

    #[test]
    fn every_spelling_belongs_to_one_command() {
        let mut spellings: Vec<String> = catalog()
            .flat_map(|info| std::iter::once(info.name).chain(info.aliases.iter().copied()))
            .map(normalize)
            .collect();
        let count = spellings.len();

        spellings.sort_unstable();
        spellings.dedup();

        assert_eq!(spellings.len(), count);
        assert_eq!(name(Command::Tool(Tool::Line)), "CIZGI");
        assert!(catalog().all(|info| !info.title.is_empty() && !info.description.is_empty()));
    }

    #[test]
    fn coordinates_accept_both_decimal_separators() {
        let ankara = Some(LonLat::new(32.85, 39.92));

        assert_eq!(coordinates("39.92, 32.85"), ankara);
        assert_eq!(coordinates("39.92 32.85"), ankara);
        assert_eq!(coordinates("39,92; 32,85"), ankara);
        assert_eq!(coordinates("39,92 32,85"), ankara);
        assert_eq!(coordinates("-12.5, -77"), Some(LonLat::new(-77.0, -12.5)));

        assert_eq!(coordinates("95, 30"), None);
        assert_eq!(coordinates("cizgi"), None);
        assert_eq!(coordinates("39.9"), None);
        assert_eq!(coordinates("1 2 3"), None);
    }
}
