//! What the command history writes of a call (docs/adr/0284): the command's
//! Turkish title, its input in plain words, what it wrote, its warnings and
//! its error with their codes. Pure: the layers' names come from the caller.

use kentos_contracts::{CallKind, CallRecord, CallStatus, CommandWarning, ProjectPart};
use kentos_domain::{RefusedAs, RefusedCall};
use serde_json::Value;

/// The longest input text before it is cut with “…”.
const INPUT_WIDTH: usize = 240;

/// The product commands' Turkish titles, the catalog's (`commandCatalog.json`;
/// a test holds them to it).
pub(crate) const TITLES: [(&str, &str); 79] = [
    ("cad.polygon.create", "Kapalı alan oluştur"),
    ("cad.line.create", "Çizgi oluştur"),
    ("cad.polyline.create", "Çoklu çizgi oluştur"),
    ("cad.entities.delete", "Nesneleri sil"),
    ("cad.point.create", "Nokta oluştur"),
    ("cad.circle.create", "Daire oluştur"),
    ("cad.arc.create", "Yay oluştur"),
    ("cad.entities.transform", "Nesneleri dönüştür"),
    ("cad.entities.edit", "Nesneleri düzenle"),
    ("cad.entities.array", "Nesneleri diziye kopyala"),
    ("cad.arrays.edit", "Diziyi düzenle"),
    ("cad.entities.explode", "Nesneleri patlat"),
    ("cad.curves.edit", "Eğriyi düzenle"),
    ("cad.entities.match", "Özellik kopyala"),
    ("cad.entities.cleanup", "Çizimi temizle"),
    ("cad.entities.copy", "Panoya kopyala"),
    ("cad.entities.paste", "Yapıştır"),
    ("cad.drawing.audit", "Denetle"),
    ("cad.entities.offset", "Nesneleri ötele"),
    ("cad.entities.order", "Çizim sırasını değiştir"),
    ("cad.nurbs.create", "NURBS eğrisi oluştur"),
    ("cad.dimensions.reassociate", "Ölçüyü yeniden bağla"),
    ("cad.dimensions.create", "Ölçü ekle"),
    ("cad.entities.trim", "Nesneleri buda"),
    ("cad.entities.extend", "Nesneleri uzat"),
    ("cad.entities.fillet", "Köşe yuvarla"),
    ("cad.entities.chamfer", "Pah kır"),
    ("cad.entities.stretch", "Nesneleri esnet"),
    ("cad.entities.break", "Nesneyi kır"),
    ("cad.entities.join", "Nesneleri birleştir"),
    ("cad.entities.reverse", "Yönü çevir"),
    ("cad.entities.lengthen", "Uzat-kısalt"),
    ("cad.entities.divide", "Nesneyi böl"),
    ("cad.entities.boundary", "İçine tıklayarak alan"),
    ("cad.entities.hatch", "Tara"),
    ("cad.entities.create", "Nesneleri ekle"),
    ("cad.entities.set", "Nesnelerin özelliklerini değiştir"),
    ("cad.blocks.define", "Blok tanımla"),
    ("cad.blocks.edit", "Blok tanımını değiştir"),
    ("cad.blocks.insert", "Blok ekle"),
    ("cad.layers.service", "Servis katmanı"),
    ("cad.network.define", "Ağ tanımı"),
    ("cad.addresses.define", "Adres verisi"),
    ("cad.layers.time", "Katmanın zamanı"),
    ("cad.scenarios.edit", "Senaryo"),
    ("cad.layers.filter", "Katman süzgeci"),
    ("cad.layers.compose", "Katman çizimi"),
    ("cad.layers.info", "Bilgi kartı ve eylemler"),
    ("cad.layers.labels", "Etiketler"),
    ("cad.labels.pin", "Etiketi sabitle"),
    ("cad.layers.renderer", "Katman stili"),
    ("cad.layers.edit", "Katman düzenle"),
    ("cad.layers.set", "Katman ağacı"),
    ("cad.layers.states", "Katman durumları"),
    ("cad.layers.find", "Katmanları bul"),
    ("cad.layers.translate", "Katmanları çevir"),
    ("cad.linetypes.edit", "Çizgi tipleri"),
    ("cad.library.edit", "Stil kitaplığı"),
    ("cad.styles.impact", "Stil değişikliğinin etkisi"),
    ("cad.standards.check", "Standardı denetle"),
    ("cad.standards.apply", "Standardı uygula"),
    ("cad.styles.edit", "Yazı ve ölçü stilleri"),
    ("cad.settings.set", "Proje ayarları"),
    ("cad.ucs.set", "Kullanıcı koordinat sistemi"),
    ("cad.views.edit", "Adlı görünümler"),
    ("cad.units.change", "Birimi değiştir"),
    ("file.import.dxf", "DXF içe aktar"),
    ("cad.sheets.edit", "Pafta kitabı"),
    ("cad.changeset.apply", "Değişiklik kümesi"),
    ("cad.rectangle.create", "Dikdörtgen oluştur"),
    ("cad.polygon.regular", "Düzgün çokgen oluştur"),
    ("cad.entities.vertex", "Köşe ve kenar"),
    ("cad.entities.pick", "Tıklanan nesneler"),
    ("cad.entities.select", "Nesne seç"),
    ("cad.snap.find", "Kenet bul"),
    ("cad.points.construct", "Nokta yapımı"),
    ("cad.curves.construct", "Eğri yapımı"),
    ("file.export.dxf", "DXF dışa aktar"),
    ("file.export.pdf", "PDF olarak yazdır"),
];

/// A command's title, or its id when the table has none.
pub(crate) fn title_of(command: &str) -> &str {
    TITLES
        .iter()
        .find(|(id, _)| *id == command)
        .map_or(command, |(_, title)| title)
}

/// An input field's Turkish name; none: the field's own.
fn field_name(key: &str) -> Option<&'static str> {
    Some(match key {
        "a" | "from" => "Başlangıç",
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
        "lineWeight" => "Kalınlık",
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
        _ => return None,
    })
}

/// A number as people read it: at most four decimals, no trailing zeros.
fn number(n: f64) -> String {
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
            if key == "uid" {
                return "1 nesne".to_owned();
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
            let head = tag(map);
            if !deep {
                return head.map_or_else(|| format!("{} alan", map.len()), |(_, t)| t.to_owned());
            }
            let fields: Vec<String> = map
                .iter()
                .filter(|(k, _)| head.is_none_or(|(t, _)| t != k.as_str()))
                .take(4)
                .map(|(k, v)| format!("{} {}", label(k).to_lowercase(), value(k, v, layer, false)))
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

/// The input in words: “Katman: «Yapı» · Merkez: 487020, 4420015 · Yarıçap: 7.5”;
/// the nesting said once, absent and empty fields left out, cut at its width.
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
            (!empty).then(|| format!("{}: {}", label(k), value(k, v, layer, true)))
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
    let text = if said.is_empty() {
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

    #[test]
    fn the_titles_are_the_catalog_s() {
        let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../web/src/contracts/generated/commandCatalog.json");
        let text = std::fs::read_to_string(path).expect("the catalog");
        let catalog: Value = serde_json::from_str(&text).expect("the catalog reads");
        let local: Vec<(String, String)> = catalog["commands"]
            .as_array()
            .expect("its commands")
            .iter()
            .filter(|c| {
                c["hosts"]
                    .as_array()
                    .is_some_and(|h| h.iter().any(|h| h == "desktop"))
            })
            .map(|c| {
                (
                    c["id"].as_str().unwrap_or_default().to_owned(),
                    c["title"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect();
        let mine: Vec<(String, String)> = TITLES
            .iter()
            .map(|(a, b)| ((*a).to_owned(), (*b).to_owned()))
            .collect();
        assert_eq!(
            mine, local,
            "TITLES is the catalog's; write it again from it"
        );
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
            "Nesneler: 2 nesne · İki yana: hayır · Yöntem: distance (uzaklık 0.5, yan 1, 2)"
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
    fn a_refusal_says_its_code() {
        let r = RefusedCall {
            seq: 1,
            command: "cad.circle.create".into(),
            version: 1,
            input: json!({}),
            refused: RefusedAs::Failed,
            error: Some(kentos_contracts::CommandError {
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
