//! Komut satırı: yazılan komutun çözümlenmesi.
//!
//! Komutlar büyük/küçük harf ve Türkçe karakter duyarsızdır: "çizgi",
//! "Çizgi" ve "CIZGI" aynı komuttur. AutoCAD'in İngilizce kısaltmaları da
//! (L, PL, C, ZE...) kabul edilir.

use kentos_rc::spatial::Tool;
use kentos_rc::theme::Mode;

use crate::message::Setting;

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
    New,
    Help,
    Quit,
}

/// Komutlar ve kabul edilen yazımları. İlk yazım, komutun arayüzde
/// gösterilen adıdır.
const COMMANDS: &[(Command, &[&str])] = &[
    (Command::Tool(Tool::Line), &["CIZGI", "LINE", "L"]),
    (Command::Tool(Tool::Polyline), &["CCIZGI", "PLINE", "PL"]),
    (Command::Tool(Tool::Polygon), &["ALAN", "POLYGON", "POL"]),
    (
        Command::Tool(Tool::Rectangle),
        &["DIKDORTGEN", "RECTANG", "REC"],
    ),
    (Command::Tool(Tool::Circle), &["DAIRE", "CIRCLE", "C"]),
    (Command::Tool(Tool::Point), &["NOKTA", "POINT", "PO"]),
    (Command::Tool(Tool::Select), &["SEC", "SECIM", "SELECT"]),
    (Command::Tool(Tool::Pan), &["PAN", "KAYDIR"]),
    (Command::Tool(Tool::Measure), &["OLC", "MESAFE", "DIST"]),
    (Command::ZoomIn, &["YAKINLASTIR", "ZI", "ZOOMIN", "+"]),
    (Command::ZoomOut, &["UZAKLASTIR", "ZU", "ZOOMOUT", "-"]),
    (Command::FitAll, &["TUMUNU", "ZE", "EXTENTS"]),
    (Command::ResetView, &["SIFIRLA"]),
    (Command::FocusSelection, &["ODAKLA", "SINIR"]),
    (Command::Toggle(Setting::Grid), &["IZGARA", "GRID"]),
    (Command::Toggle(Setting::Snap), &["YAKALA", "SNAP", "OSNAP"]),
    (
        Command::Toggle(Setting::FullCrosshair),
        &["ARTI", "IMLEC", "CROSSHAIR"],
    ),
    (
        Command::Toggle(Setting::Labels),
        &["ETIKET", "ETIKETLER", "LABELS"],
    ),
    (
        Command::Toggle(Setting::ViewCube),
        &["3B", "KUP", "CUBE", "VIEWCUBE"],
    ),
    (Command::Theme(None), &["TEMA"]),
    (Command::Theme(Some(Mode::Light)), &["AYDINLIK", "LIGHT"]),
    (Command::Theme(Some(Mode::Dark)), &["KOYU", "DARK"]),
    (Command::ShowAllLayers, &["KATGOSTER"]),
    (Command::HideAllLayers, &["KATGIZLE"]),
    (Command::ListLayers, &["KATMANLAR", "LAYERS"]),
    (Command::ActiveLayer, &["KATMAN", "AKTIFKATMAN"]),
    (Command::SelectAll, &["TUMUNUSEC", "SELECTALL"]),
    (Command::InvertSelection, &["TERSSEC", "INVERT"]),
    (Command::AttributeTable, &["TABLO", "OZNITELIK", "TABLE"]),
    (Command::SelectByAttributes, &["SORGU", "QSELECT"]),
    (Command::Filter, &["FILTRE", "FILTER"]),
    (Command::Delete, &["SIL", "ERASE", "E"]),
    (Command::Clear, &["TEMIZLE", "IPTAL"]),
    (Command::New, &["YENI", "NEW"]),
    (Command::Help, &["YARDIM", "HELP", "?"]),
    (Command::Quit, &["CIKIS", "KAPAT", "QUIT"]),
];

/// Yazılan komutu çözümler; tanınmayan komutta `None`.
pub fn parse(input: &str) -> Option<Command> {
    let input = normalize(input);

    COMMANDS
        .iter()
        .find(|(_, spellings)| spellings.contains(&input.as_str()))
        .map(|(command, _)| *command)
}

/// Komutun arayüzde gösterilen adı (ör. araç ipuçlarındaki "Komut: CIZGI").
pub fn name(command: Command) -> &'static str {
    COMMANDS
        .iter()
        .find(|(candidate, _)| *candidate == command)
        .map_or("", |(_, spellings)| spellings[0])
}

/// Komut listesi: her komutun gösterilen adı.
pub fn names() -> impl Iterator<Item = &'static str> {
    COMMANDS.iter().map(|(_, spellings)| spellings[0])
}

/// Boşlukları kırpar, Türkçe harfleri ASCII karşılıklarına çevirir ve büyük
/// harfe dönüştürür.
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
    }

    #[test]
    fn every_command_has_a_unique_display_name() {
        let names: Vec<&str> = names().collect();
        let mut unique = names.clone();
        unique.sort_unstable();
        unique.dedup();

        assert_eq!(names.len(), unique.len());
        assert_eq!(name(Command::Tool(Tool::Line)), "CIZGI");
    }
}
