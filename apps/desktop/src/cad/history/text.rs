//! What the command history writes of a call (docs/adr/0284): the command's
//! Turkish title, its input in plain words, what it wrote, its warnings and
//! its error with their codes. Pure: the layers' names come from the caller.

use kentos_commands::{CallKind, CallRecord, CallStatus, CommandWarning, ProjectPart};
use kentos_domain::{RefusedAs, RefusedCall};
use serde_json::Value;

/// The longest input text before it is cut with “…”.
const INPUT_WIDTH: usize = 240;

/// The product commands' Turkish titles, the catalog's: `catalog/v1/command-titles.json`, written with the catalog
/// from the commands' descriptors and held to them there (`cargo test -p kentos-commands catalog`). Read once, the
/// first time a title is asked; building the catalog itself would take seconds.
fn titles() -> &'static std::collections::HashMap<String, String> {
    static TITLES: std::sync::OnceLock<std::collections::HashMap<String, String>> =
        std::sync::OnceLock::new();
    TITLES.get_or_init(|| {
        let file: Value = serde_json::from_str(include_str!(
            "../../../../../catalog/v1/command-titles.json"
        ))
        .unwrap_or(Value::Null);
        file["titles"]
            .as_object()
            .into_iter()
            .flatten()
            .filter_map(|(id, title)| Some((id.clone(), title.as_str()?.to_owned())))
            .collect()
    })
}

/// A command's title as the catalog gives it; none for an id the catalog does not have.
pub(crate) fn catalog_title(command: &str) -> Option<&'static str> {
    titles().get(command).map(String::as_str)
}

/// A command's title, or its id when the catalog has none.
pub(crate) fn title_of(command: &str) -> &str {
    catalog_title(command).unwrap_or(command)
}

/// An input field's Turkish name; none: the field's own.
fn field_name(key: &str) -> Option<&'static str> {
    Some(match key {
        "a" | "from" => "Başlangıç",
        "anchor" => "Sabit uç",
        "b" | "to" => "Bitiş",
        "a0" => "Başlangıç açısı",
        "a1" => "Bitiş açısı",
        "action" => "Eylem",
        "actions" => "Eylemler",
        "angle" => "Açı",
        "asBlock" => "Blok olarak",
        "at" | "place" => "Yer",
        "attrs" | "attributes" => "Öznitelikler",
        "base" => "Taban noktası",
        "block" => "Blok",
        "blocks" => "Bloklar",
        "host" => "Duvar",
        "openings" => "Açıklıklar",
        "spaces" => "Mahaller",
        "seed" => "İç nokta",
        "grids" => "Akslar",
        "columns" => "Kolonlar",
        "stairs" => "Merdivenler",
        "ramps" => "Rampalar",
        "series" => "Seri",
        "spacings" => "Aralıklar",
        "crossings" => "Aks kesişimleri",
        "tagStart" => "İlk etiket",
        "risers" => "Rıht sayısı",
        "riserHeight" => "Rıht",
        "tread" => "Basamak",
        "landing" => "Sahanlık",
        "rise" => "Yükseklik farkı",
        "slope" => "Eğim",
        "tags" => "Etiketler",
        "of" => "Nesne",
        "marks" => "İşaretler",
        "form" => "Biçim",
        "target" => "Hedef",
        "mark" => "Numara",
        "flip" => "Çevir",
        "surface" => "Yüzey",
        "grid" => "Izgara",
        "size" => "Yazı yüksekliği",
        "origin" => "Başlangıç",
        "content" => "Şablon",
        "leader" => "Kol",
        "all" => "Hepsi",
        "change" => "Değişiklik",
        "sort" => "Sıra",
        "group" => "Grup",
        "total" => "Toplam",
        "row" => "Satır",
        "column" => "Sütun",
        "parts" => "Parçalar",
        "usage" | "usages" => "Kullanım",
        "levels" => "Katlar",
        "colorPlan" => "Renk planı",
        "phases" => "Tadilat durumları",
        "bothSides" => "İki yana",
        "boundaries" => "Sınırlar",
        "boundaryLayer" => "Sınır katmanı",
        "bulges" => "Bükümler",
        "by" => "Yöntem",
        "c" | "center" => "Merkez",
        "chain" => "Zincir",
        "clip" => "Pano içeriği",
        "changes" => "Değişiklikler",
        "closed" => "Kapalı",
        "color" => "Renk",
        "copy" => "Kopya",
        "corner" => "Köşe",
        "count" => "Sayı",
        "deleteSource" => "Kaynağı sil",
        "description" => "Açıklama",
        "dimension" | "measure" => "Ölçü",
        "dimensionStyles" => "Ölçü stilleri",
        "distance" => "Uzaklık",
        "drawingUnit" => "Çizim birimi",
        "dx" => "Doğu farkı",
        "dy" => "Kuzey farkı",
        "fields" => "Alanlar",
        "filter" => "Süzgeç",
        "geometry" => "Geometri",
        "height" => "Yükseklik",
        "holes" => "Delikler",
        "id" => "Kimlik",
        "index" => "Sıra",
        "islands" => "Adalar",
        "kind" => "Tür",
        "kinds" => "Türler",
        "label" => "Etiket",
        "layer" | "layerId" => "Katman",
        "layers" => "Katmanlar",
        "library" => "Kitaplık",
        "lineType" => "Çizgi tipi",
        "lineTypeScale" => "Çizgi tipi ölçeği",
        "lineWeight" => "Çizgi kalınlığı",
        "locked" => "Kilitli",
        "name" => "Ad",
        "note" => "Not",
        "objects" => "Nesneler",
        "offset" => "Öteleme",
        "opacity" => "Opaklık",
        "operation" | "op" => "İşlem",
        "ops" => "İşlemler",
        "p" | "point" => "Nokta",
        "parent" => "Üst grup",
        "path" => "Yol",
        "pattern" => "Desen",
        "picks" => "Tıklananlar",
        "plotScale" => "Çizim ölçeği",
        "points" | "pts" => "Noktalar",
        "r" | "radius" => "Yarıçap",
        "reference" => "Başvuru",
        "region" => "Bölge",
        "renderer" => "Stil",
        "scale" => "Ölçek",
        "side" => "Yan",
        "srid" => "Koordinat sistemi",
        "steps" => "Adımlar",
        "style" => "Stil",
        "symbol" => "Sembol",
        "text" => "Yazı",
        "textStyles" => "Yazı stilleri",
        "time" => "Zaman",
        "tolerance" => "Tolerans",
        "topology" => "Topoloji",
        "transform" => "Dönüşüm",
        "transparency" => "Saydamlık",
        "trim" => "Kırp",
        "uid" => "Nesne",
        "uids" => "Nesneler",
        "unit" => "Birim",
        "value" => "Değer",
        "visible" => "Görünür",
        "widths" => "Genişlikler",
        "window" => "Pencere",
        "wipeout" => "Maske",
        "z" => "Kot",
        // The fields Mimari's commands send, which the row said in English (docs/adr/8004).
        "adopt" => "Ortak duvar",
        "arch" => "Mimari",
        "bubbleSize" => "Balon boyu",
        "bubbles" => "Balonlar",
        "caps" => "Uçlar",
        "cooktop" => "Ocak",
        "depth" => "Derinlik",
        "dishwasher" => "Bulaşık makinesi",
        "distanceReference" => "Ölçü noktası",
        "edge" => "Kenar",
        "extent" => "Kapsam",
        "face" => "Yüz",
        "first" => "İlk numara",
        "fit" => "Uyum",
        "frame" => "Çerçeve",
        "justify" => "Hiza",
        "keepSources" => "Kaynak çizgiler korunur",
        "length" => "Boy",
        "level" => "Kat",
        "material" => "Malzeme",
        "method" => "Alan yöntemi",
        "middle" => "Ortada",
        "nodes" => "Düğümler",
        "number" => "Numara",
        "order" => "Sıra",
        "oven" => "Fırın",
        "pairs" => "Çiftler",
        "radial" => "Eğrisel aks sistemi",
        "rings" => "Halkalar",
        "role" => "Döşeme türü",
        "rotation" => "Açı",
        "sink" => "Evye",
        "slabs" => "Döşemeler",
        "source" => "Kaynak",
        "span" => "Aralık",
        "system" => "Aks sistemi",
        "thickness" => "Kalınlık",
        "type" => "Tür",
        "void" => "Döşeme boşluğu",
        "wall" => "Duvar",
        "walls" => "Duvarlar",
        "width" => "Genişlik",
        _ => return None,
    })
}

/// A number as people read it: at most four decimals, no trailing zeros.
pub(crate) fn number(n: f64) -> String {
    let text = format!("{n:.4}");
    let text = text.trim_end_matches('0').trim_end_matches('.');
    if text == "-0" {
        "0".to_owned()
    } else {
        text.to_owned()
    }
}

/// A point `{x, y}` (east, north: Y, X), or none.
fn point(value: &Value) -> Option<String> {
    let map = value.as_object()?;
    if map.len() != 2 {
        return None;
    }
    Some(format!(
        "{}, {}",
        number(map.get("x")?.as_f64()?),
        number(map.get("y")?.as_f64()?)
    ))
}

/// The tag of a tagged object (`kind`, `op`, `operation`, `type`).
fn tag(map: &serde_json::Map<String, Value>) -> Option<(&'static str, &str)> {
    ["kind", "op", "operation", "type"]
        .into_iter()
        .find_map(|k| map.get(k).and_then(Value::as_str).map(|v| (k, v)))
}

/// A value in words; `deep` objects are said by their tag and fields once.
fn value(key: &str, v: &Value, layer: &dyn Fn(&str) -> Option<String>, deep: bool) -> String {
    match v {
        Value::Null => "yok".to_owned(),
        Value::Bool(b) => if *b { "evet" } else { "hayır" }.to_owned(),
        Value::Number(n) => n.as_f64().map_or_else(|| n.to_string(), number),
        Value::String(s) => {
            if matches!(key, "layer" | "layerId" | "boundaryLayer" | "parent")
                && let Some(name) = layer(s)
            {
                return format!("«{name}»");
            }
            if key == "uid" || is_id(s) {
                return "1 nesne".to_owned();
            }
            if let Some(word) = enum_word(key, s) {
                return word.to_owned();
            }
            // A list of fields names them in Turkish (`fields: ["frame"]`).
            if key == "fields"
                && let Some(name) = field_name(s)
            {
                return name.to_lowercase();
            }
            // A temporary distance's reference and a length's fixed end (docs/adr/0728).
            match (key, s.as_str()) {
                ("to", "grid") => return "aks".to_owned(),
                ("to", "face") => return "yüz".to_owned(),
                ("anchor", "start") => return "baş".to_owned(),
                ("anchor", "end") => return "son".to_owned(),
                ("anchor", "middle") => return "orta".to_owned(),
                _ => {}
            }
            let short: String = s.chars().take(40).collect();
            if short.chars().count() < s.chars().count() {
                format!("{short}…")
            } else {
                short
            }
        }
        Value::Array(items) => {
            if key == "uids" || key == "objects" {
                return format!("{} nesne", items.len());
            }
            if items.iter().all(|i| point(i).is_some()) && !items.is_empty() {
                return format!("{} nokta", items.len());
            }
            if items.len() <= 4 && items.iter().all(|i| !i.is_object() && !i.is_array()) {
                return items
                    .iter()
                    .map(|i| value(key, i, layer, false))
                    .collect::<Vec<_>>()
                    .join(", ");
            }
            format!("{} öğe", items.len())
        }
        Value::Object(map) => {
            if let Some(p) = point(v) {
                return p;
            }
            // An object's tag in Turkish too (`distance` is uzaklık, docs/adr/8004).
            let head = tag(map).map(|(k, t)| (k, enum_word(k, t).unwrap_or(t)));
            if !deep {
                return head.map_or_else(|| format!("{} alan", map.len()), |(_, t)| t.to_owned());
            }
            let fields: Vec<String> = map
                .iter()
                .filter(|(k, _)| head.is_none_or(|(t, _)| t != k.as_str()))
                .take(4)
                .map(|(k, v)| {
                    format!(
                        "{} {}",
                        label_of(k, v).to_lowercase(),
                        value(k, v, layer, false)
                    )
                })
                .collect();
            match head {
                Some((_, t)) if fields.is_empty() => t.to_owned(),
                Some((_, t)) => format!("{t} ({})", fields.join(", ")),
                None => fields.join(", "),
            }
        }
    }
}

fn label(key: &str) -> String {
    field_name(key).map_or_else(|| key.to_owned(), str::to_owned)
}

/// A field's name for its value: a wall's `by` is how far it moves, elsewhere `by` is a way (docs/adr/8004).
fn label_of(key: &str, v: &Value) -> String {
    if key == "by" && v.is_number() {
        return "Miktar".to_owned();
    }
    label(key)
}

/// An object's id, as the journal keeps it (`0192f5a1-6000-…`, `$uid:source`): never shown (docs/adr/8004).
fn is_id(s: &str) -> bool {
    if s.starts_with("$uid") {
        return true;
    }
    let parts: Vec<&str> = s.split('-').collect();
    s.len() == 36
        && parts.iter().map(|p| p.len()).eq([8, 4, 4, 4, 12])
        && parts
            .iter()
            .all(|p| p.chars().all(|c| c.is_ascii_hexdigit()))
}

/// A tag or a choice said in Turkish: `left` is sol, `clear` net (docs/adr/8004).
fn enum_word(key: &str, s: &str) -> Option<&'static str> {
    let keyed = matches!(
        key,
        "kind"
            | "op"
            | "operation"
            | "side"
            | "measure"
            | "reference"
            | "justify"
            | "method"
            | "role"
            | "face"
            | "from"
            | "distanceReference"
            | "of"
    );
    if !keyed {
        return None;
    }
    Some(match s {
        "left" => "sol",
        "right" => "sağ",
        "center" | "middle" => "orta",
        "start" => "baş",
        "end" => "son",
        "corners" => "köşeler",
        "size" => "boyut",
        "area" => "alan",
        "edge" => "kenar",
        "clear" => "net",
        "axis" => "eksen",
        "inner" => "iç net",
        "outer" => "dış ölçü",
        "net" => "net",
        "gross" => "brüt",
        "centerline" => "duvar ekseni",
        "shift" => "kaydır",
        "set" => "değiştir",
        "distance" => "uzaklık",
        "length" => "boy",
        "flip" => "çevir",
        "move" => "taşı",
        "source" => "kaynak boyunca",
        "balcony" => "balkon",
        "terrace" => "teras",
        "floor" => "döşeme",
        "fixtures" => "donatı",
        "openings" => "açıklıklar",
        "spaces" => "mahaller",
        "units" => "bölümler",
        "finishes" => "kaplamalar",
        "direction" => "doğrultu",
        "rehost" => "başka duvara",
        "replaceType" => "türü değiştir",
        "detach" => "tipten ayır",
        "push" => "tipe uygula",
        "refresh" => "tipi yeniden uygula",
        "reset" => "tipe döndür",
        _ => return None,
    })
}

/// The input in words: “Katman: «Yapı» · Merkez: 487020, 4420015 · Yarıçap: 7.5”;
/// the nesting said once, absent and empty fields left out, cut at its width.
pub(crate) fn inputs_for(
    command: Option<&str>,
    input: &Value,
    layer: &dyn Fn(&str) -> Option<String>,
) -> String {
    if command == Some(kentos_commands::CAD_ARRAYS_ADOPT)
        && let Ok(i) = serde_json::from_value::<kentos_commands::ArraysAdopt>(input.clone())
    {
        let missing = i.items.iter().filter(|i| i.uid.is_none()).count();
        let kept = i.items.iter().filter(|i| i.kept).count();
        let kind = match i.layout {
            kentos_commands::ArrayLayout::Grid { .. } => "Izgara",
            kentos_commands::ArrayLayout::Lattice { .. } => "Dönük/eğik ızgara",
            kentos_commands::ArrayLayout::Line { .. } => "Sıra",
            kentos_commands::ArrayLayout::Polar { .. } => "Kutupsal",
            kentos_commands::ArrayLayout::Path { .. } => "Yol boyunca",
        };
        let mm = (i.tolerance.unwrap_or(1e-6) * 1000.)
            .to_string()
            .replace('.', ",");
        return format!(
            "{kind} · {} yer · {} mevcut blok · {missing} eksik · {kept} korunan · Tolerans: {mm} mm",
            i.items.len(),
            i.items.len() - missing
        );
    }
    if command == Some(kentos_commands::CAD_BLOCKS_CONVERT)
        && let Ok(i) = serde_json::from_value::<kentos_commands::BlocksConvert>(input.clone())
    {
        // This command's tolerance is metres, independent of the project's display unit.
        // Keep significant digits: the generic four-decimal summary made 0.001 mm say 0.
        let mm = (i
            .tolerance
            .unwrap_or(kentos_commands::RECOGNITION_TOLERANCE)
            * 1000.)
            .to_string()
            .replace('.', ",");
        let name = value("name", &Value::String(i.name), layer, false);
        let on = |b| if b { "evet" } else { "hayır" };
        return format!(
            "Ad: {name} · {} aday · Kaynak: {} parça · Tolerans: {mm} mm · Döndürme: {} · Aynalama: {} · Farklı boyut: {}",
            i.groups.len(),
            i.source.len(),
            on(i.rotations.unwrap_or(true)),
            on(i.mirrors.unwrap_or(false)),
            on(i.scales.unwrap_or(false))
        );
    }
    let words = inputs(input, layer);
    // A wall's width is its Kalınlık (docs/adr/8003, 8004).
    if command.is_some_and(|c| c.starts_with("cad.walls.")) {
        return words
            .replace("Genişlik:", "Kalınlık:")
            .replace("genişlik ", "kalınlık ");
    }
    words
}

pub(crate) fn inputs(input: &Value, layer: &dyn Fn(&str) -> Option<String>) -> String {
    let Some(map) = input.as_object() else {
        return String::new();
    };
    let first = ["uids", "uid", "layerId", "layer"];
    let mut keys: Vec<&String> = map.keys().collect();
    keys.sort_by_key(|k| first.iter().position(|f| f == k).unwrap_or(first.len()));
    let parts: Vec<String> = keys
        .into_iter()
        .filter(|k| k.as_str() != "expectedRevision")
        .filter_map(|k| {
            let v = &map[k];
            let empty = v.is_null()
                || v.as_array().is_some_and(Vec::is_empty)
                || v.as_object().is_some_and(serde_json::Map::is_empty);
            (!empty).then(|| format!("{}: {}", label_of(k, v), value(k, v, layer, true)))
        })
        .collect();
    let text = parts.join(" · ");
    if text.chars().count() > INPUT_WIDTH {
        let cut: String = text.chars().take(INPUT_WIDTH).collect();
        format!("{cut}…")
    } else {
        text
    }
}

/// A project part's name.
fn part_name(part: ProjectPart) -> &'static str {
    match part {
        ProjectPart::Name => "çizimin adı",
        ProjectPart::Settings => "proje ayarları",
        ProjectPart::Styles => "stil kitaplığı",
        ProjectPart::Sheets => "pafta kitabı",
        ProjectPart::LayerFlags => "katmanların durumu",
        ProjectPart::Constraints => "kısıtlar",
        ProjectPart::DimensionSets => "otomatik ölçüler",
    }
}

/// What a row wrote: “1 nesne eklendi · 2 nesne değişti”, the parts and files.
pub(crate) fn outcome(row: &CallRecord) -> String {
    let mut said = Vec::new();
    for (n, what) in [
        (row.created.len(), "eklendi"),
        (row.changed.len(), "değişti"),
        (row.removed.len(), "silindi"),
    ] {
        if n > 0 {
            said.push(format!("{n} nesne {what}"));
        }
    }
    for part in &row.parts {
        said.push(format!("{} değişti", part_name(*part)));
    }
    for file in &row.effects {
        said.push(format!("dosya yazıldı: {}", file.path));
    }
    let text = if said.is_empty() && row.base != row.revision {
        // Block and other definition edits may have no object ids or named
        // project parts in this journal format. A changed revision still
        // proves a write; do not claim the drawing was unchanged.
        "Çizim güncellendi".to_owned()
    } else if said.is_empty() {
        "Çizimde değişiklik yok".to_owned()
    } else {
        let mut text = said.join(" · ");
        if let Some(first) = text.get(..1) {
            let upper = first.to_uppercase();
            text.replace_range(..1, &upper);
        }
        text
    };
    match row.status {
        CallStatus::Done => text,
        CallStatus::Undone => format!("{text} · geri alındı"),
    }
}

/// A row's title: the command's, or what an edit made without one was.
pub(crate) fn row_title(row: &CallRecord) -> String {
    match (row.kind, &row.command) {
        (CallKind::Command, Some(command)) => title_of(command).to_owned(),
        _ => format!(
            "Komutsuz düzenleme: {}",
            row.label.as_deref().unwrap_or("adsız")
        ),
    }
}

/// A warning as the history writes it: its code, then its message.
pub(crate) fn warning(w: &CommandWarning) -> String {
    format!("{}: {}", w.code, w.message)
}

/// How a refused call ended, its code and message.
pub(crate) fn refusal(r: &RefusedCall) -> String {
    let how = match r.refused {
        RefusedAs::Failed => "Reddedildi",
        RefusedAs::Conflict => "Çakışma",
        RefusedAs::NeedsInput => "Girdi eksik",
        RefusedAs::Cancelled => "Vazgeçildi",
    };
    match &r.error {
        Some(e) => format!("{how}, hiçbir şey yazılmadı · {}: {}", e.code, e.message),
        None => format!("{how}, hiçbir şey yazılmadı"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// Every command the desktop runs has its catalog title, and an id the catalog does not have is said as it is.
    #[test]
    fn the_titles_are_the_catalog_s() {
        assert_eq!(title_of("cad.polygon.create"), "Kapalı alan oluştur");
        assert_eq!(title_of("cad.no.such"), "cad.no.such");
        for (id, _) in kentos_native_application::DESKTOP_COMMANDS {
            assert_ne!(title_of(id), *id, "{id} has a title");
        }
    }

    #[test]
    fn the_input_is_said_in_words() {
        let layer = |id: &str| (id == "yapi").then(|| "Yapı".to_owned());
        assert_eq!(
            inputs(
                &json!({"c": {"x": 487020.0, "y": 4420015.5}, "expectedRevision": "3", "layerId": "yapi", "r": 7.5, "attrs": {}}),
                &layer
            ),
            "Katman: «Yapı» · Merkez: 487020, 4420015.5 · Yarıçap: 7.5"
        );
        assert_eq!(
            inputs(
                &json!({"uids": ["a", "b"], "by": {"kind": "distance", "distance": 0.5, "side": {"x": 1.0, "y": 2.0}}, "bothSides": false}),
                &layer
            ),
            "Nesneler: 2 nesne · İki yana: hayır · Yöntem: uzaklık (uzaklık 0.5, yan 1, 2)"
        );
        assert_eq!(
            inputs(
                &json!({"pts": [{"x": 0, "y": 0}, {"x": 1, "y": 0}, {"x": 1, "y": 1}], "closed": true}),
                &layer
            ),
            "Kapalı: evet · Noktalar: 3 nokta"
        );
    }

    #[test]
    fn repeat_conversion_history_names_millimetres_without_rounding_a_positive_tolerance_to_zero() {
        let mut input = json!({"name":"Yaylı tekrar", "source":["source"],
            "groups":[["source"],["other"]],"mirrors":true});
        let before = input.clone();
        let said = inputs_for(Some(kentos_commands::CAD_BLOCKS_CONVERT), &input, &|_| None);
        assert!(
            said.contains("2 aday · Kaynak: 1 parça · Tolerans: 0,001 mm"),
            "{said}"
        );
        assert!(said.contains("Döndürme: evet · Aynalama: evet · Farklı boyut: hayır"));
        assert_eq!(
            input, before,
            "only the readable summary changes; replay keeps model metres"
        );
        input["tolerance"] = json!(1e-12);
        let said = inputs_for(Some(kentos_commands::CAD_BLOCKS_CONVERT), &input, &|_| None);
        assert!(said.contains("Tolerans: 0,000000001 mm"), "{said}");
        assert_eq!(
            inputs_for(Some("cad.circle.create"), &input, &|_| None),
            inputs(&input, &|_| None),
            "other commands keep their existing summary"
        );
    }

    #[test]
    fn adoption_history_says_missing_and_kept_places_with_model_tolerance_in_millimetres() {
        let input = json!({"source":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f",
            "layout":{"kind":"line","count":3,"dx":2.,"dy":0.,"fit":false},
            "items":[{"uid":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f"},{},
                {"uid":"01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e60","kept":true}]});
        let before = input.clone();
        assert_eq!(
            inputs_for(Some(kentos_commands::CAD_ARRAYS_ADOPT), &input, &|_| None),
            "Sıra · 3 yer · 2 mevcut blok · 1 eksik · 1 korunan · Tolerans: 0,001 mm"
        );
        assert_eq!(input, before, "the replay input stays in model metres");
    }

    #[test]
    fn a_refusal_says_its_code() {
        let r = RefusedCall {
            seq: 1,
            command: "cad.circle.create".into(),
            version: 1,
            input: json!({}),
            refused: RefusedAs::Failed,
            error: Some(kentos_commands::CommandError {
                code: "layer_locked".into(),
                message: "Katman kilitli.".into(),
                ..Default::default()
            }),
        };
        assert_eq!(
            refusal(&r),
            "Reddedildi, hiçbir şey yazılmadı · layer_locked: Katman kilitli."
        );
    }
}
