//! The kind's own properties in the inspector (design §11): a short list per
//! kind, each a path in the item's JSON, a field of the KentOS inspector and
//! how its value is written there. A change is the core's `SetItemProps`
//! with a JSON merge of that path, for every chosen item of the kind; where
//! the chosen items differ the field says “Çeşitli”.

use kentos_sheet::model::Item;
use kentos_ui::attribute::{Field, Value};
use serde_json::{Value as Json, json};

use crate::designer::Designer;

/// How a property's JSON is shown and written.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum Conv {
    Text,
    /// A text whose absence is the default (a coordinate list's heading,
    /// docs/adr/0206 §3): empty writes none.
    Optional,
    /// A whole number.
    Int,
    /// Micrometres shown as millimetres.
    Mm,
    /// Thousandths of a degree shown as degrees.
    Deg,
    /// Thousandths of a degree shown as degrees to the hundredth (a declination).
    Deg2,
    Bool,
    /// One of these JSON strings or numbers, shown by their names.
    Choice(&'static [(&'static str, &'static str)]),
    /// One of the sheet's maps (the item a scale bar, north arrow or legend reads); none: the first.
    Map,
}

/// A property of the kind: where it is and what it is.
#[derive(Clone, Debug)]
pub(crate) struct Prop {
    pub path: &'static [&'static str],
    pub label: &'static str,
    pub conv: Conv,
    pub multiline: bool,
    pub description: &'static str,
}

const fn prop(path: &'static [&'static str], label: &'static str, conv: Conv) -> Prop {
    Prop {
        path,
        label,
        conv,
        multiline: false,
        description: "",
    }
}

const WEIGHTS: &[(&str, &str)] = &[
    ("400", "Normal"),
    ("500", "Orta"),
    ("600", "Yarı kalın"),
    ("700", "Kalın"),
];
const ALIGN: &[(&str, &str)] = &[("left", "Sol"), ("center", "Orta"), ("right", "Sağ")];
const VALIGN: &[(&str, &str)] = &[("top", "Üst"), ("middle", "Orta"), ("bottom", "Alt")];
const FIT: &[(&str, &str)] = &[("none", "Sığdırma"), ("shrinkToFit", "Küçülterek sığdır")];
const BAR_STYLE: &[(&str, &str)] = &[
    ("singleBox", "Tek kutu"),
    ("doubleBox", "Çift kutu"),
    ("ticks", "Çentikli"),
    ("stepped", "Basamaklı"),
    ("hollow", "Boş kutu"),
    ("numeric", "Sayısal ölçek"),
];
const UNIT: &[(&str, &str)] = &[
    ("auto", "Kendiliğinden"),
    ("m", "Metre"),
    ("km", "Kilometre"),
];
const NORTH: &[(&str, &str)] = &[
    ("grid", "Grid kuzeyi"),
    ("true", "Coğrafi kuzey"),
    ("magnetic", "Manyetik kuzey"),
];
const NORTH_STYLE: &[(&str, &str)] = &[
    ("kentosK", "KentOS"),
    ("simple", "Basit ok"),
    ("compass", "Pusula"),
    ("diagram", "Kuzey çizelgesi"),
];
/// The web's words (kindSections.ts `pictureSection`).
const PICTURE_FIT: &[(&str, &str)] = &[
    ("contain", "İçine sığdır"),
    ("cover", "Kapla"),
    ("stretch", "Ger"),
    ("original", "Özgün boy"),
];
const LINE_END: &[(&str, &str)] = &[
    ("none", "Yok"),
    ("arrow", "Ok"),
    ("dot", "Nokta"),
    ("bar", "Çizgi"),
];
const BORDER: &[(&str, &str)] = &[("single", "Tek çizgi"), ("double", "Çift çizgi")];

/// The fonts of the core's table by their families' names.
fn font_choices() -> &'static [(&'static str, &'static str)] {
    &[
        ("barlow", "Barlow"),
        ("arimo", "Arimo"),
        ("overpass", "Overpass"),
        ("quicksand", "Quicksand"),
        ("architects-daughter", "Architects Daughter"),
        ("courier-prime", "Courier Prime"),
        ("plex-mono", "IBM Plex Mono"),
    ]
}

/// The properties of a kind (its `type`).
pub(crate) fn props_of(kind: &str) -> Vec<Prop> {
    match kind {
        "map" => vec![
            prop(&["kind", "view", "scale"], "Ölçek (1:…)", Conv::Int),
            prop(&["kind", "view", "rotation"], "Dönüş", Conv::Deg),
            prop(&["kind", "labelBand"], "Yazı bandı", Conv::Mm),
        ],
        "text" => vec![
            Prop {
                multiline: true,
                description: "[% @proje_adi %] gibi değerler ve ifadeler yazılabilir.",
                ..prop(&["kind", "content"], "Metin", Conv::Text)
            },
            prop(
                &["kind", "style", "font"],
                "Yazı tipi",
                Conv::Choice(font_choices()),
            ),
            prop(&["kind", "style", "size"], "Boyut", Conv::Mm),
            prop(
                &["kind", "style", "weight"],
                "Kalınlık",
                Conv::Choice(WEIGHTS),
            ),
            prop(&["kind", "style", "italic"], "Eğik", Conv::Bool),
            prop(&["kind", "style", "color"], "Renk", Conv::Text),
            prop(&["kind", "align"], "Yatay hiza", Conv::Choice(ALIGN)),
            prop(&["kind", "valign"], "Dikey hiza", Conv::Choice(VALIGN)),
            prop(&["kind", "wrap"], "Satır kaydır", Conv::Bool),
            prop(&["kind", "fit"], "Sığdırma", Conv::Choice(FIT)),
        ],
        "scaleBar" => vec![
            prop(&["kind", "map"], "Harita", Conv::Map),
            prop(&["kind", "style"], "Biçim", Conv::Choice(BAR_STYLE)),
            prop(&["kind", "segments"], "Bölüm", Conv::Int),
            prop(&["kind", "unit"], "Birim", Conv::Choice(UNIT)),
            prop(&["kind", "showScale"], "Ölçeği yaz", Conv::Bool),
            prop(&["kind", "height"], "Yükseklik", Conv::Mm),
        ],
        // After “Biçim” the inspector's own part: what the paper writes and from what, the date,
        // “Sapmayı elle gir” (inspector_view.rs `north_part`); the value typed and its year only
        // when typed by hand (`refresh_fields`), as the web's `northSection`.
        "northArrow" => vec![
            prop(&["kind", "map"], "Harita", Conv::Map),
            prop(&["kind", "north"], "Kuzey", Conv::Choice(NORTH)),
            prop(&["kind", "style"], "Biçim", Conv::Choice(NORTH_STYLE)),
            prop(&["kind", "declination"], "Elle sapma", Conv::Deg2),
            prop(&["kind", "declinationYear"], "Sapmanın yılı", Conv::Int),
            prop(&["kind", "note"], "Yakınsama notu", Conv::Bool),
            prop(&["kind", "color"], "Renk", Conv::Text),
        ],
        "legend" => vec![
            prop(&["kind", "map"], "Harita", Conv::Map),
            prop(&["kind", "title"], "Başlık", Conv::Text),
            prop(&["kind", "columns"], "Sütun", Conv::Int),
        ],
        "picture" => vec![
            prop(&["kind", "fit"], "Sığdırma", Conv::Choice(PICTURE_FIT)),
            prop(&["kind", "clip"], "Çerçevede kırp", Conv::Bool),
        ],
        "shape" => vec![
            prop(&["kind", "fill"], "Dolgu", Conv::Text),
            prop(&["kind", "stroke", "color"], "Çizgi rengi", Conv::Text),
            prop(&["kind", "stroke", "width"], "Çizgi kalınlığı", Conv::Mm),
        ],
        "line" => vec![
            prop(&["kind", "stroke", "color"], "Renk", Conv::Text),
            prop(&["kind", "stroke", "width"], "Kalınlık", Conv::Mm),
            prop(&["kind", "start"], "Başlangıç", Conv::Choice(LINE_END)),
            prop(&["kind", "end"], "Bitiş", Conv::Choice(LINE_END)),
        ],
        "table" => vec![
            prop(&["kind", "title"], "Başlık", Conv::Text),
            prop(&["kind", "header"], "Başlık satırı", Conv::Bool),
            prop(&["kind", "zebra"], "Zebra rengi", Conv::Text),
        ],
        // Its source, what it gives and the points' names are the inspector's own part
        // (inspector_view.rs `coordinate_part`, docs/adr/0206 §2); its headings here, each empty
        // for its default (§3).
        "coordinateList" => vec![
            prop(&["kind", "title"], "Başlık", Conv::Text),
            prop(&["kind", "decimals"], "Ondalık", Conv::Int),
            prop(&["kind", "z"], "Z sütunu", Conv::Bool),
            Prop {
                description: "İlk nokta son satırda yeniden: kapalı şekil.",
                ..prop(&["kind", "closingRow"], "Kapanış satırı", Conv::Bool)
            },
            Prop {
                description: "Kapalı şeklin alanı, çizimin ölçtüğü gibi (yaylar dahil).",
                ..prop(&["kind", "areaRow"], "Alan satırı", Conv::Bool)
            },
            Prop {
                description: "Sütun başlığı; boş: Nokta.",
                ..prop(
                    &["kind", "columns", "point"],
                    "Nokta başlığı",
                    Conv::Optional,
                )
            },
            Prop {
                description: "Boş: Y (m); yerel projede X (m). Birimi başlığa siz yazarsınız.",
                ..prop(&["kind", "columns", "east"], "Doğu başlığı", Conv::Optional)
            },
            Prop {
                description: "Boş: X (m); yerel projede Y (m).",
                ..prop(
                    &["kind", "columns", "north"],
                    "Kuzey başlığı",
                    Conv::Optional,
                )
            },
            Prop {
                description: "Boş: Z (m).",
                ..prop(&["kind", "columns", "z"], "Z başlığı", Conv::Optional)
            },
            Prop {
                description: "Alan satırının sözü; boş: Alan.",
                ..prop(&["kind", "columns", "area"], "Alan sözü", Conv::Optional)
            },
        ],
        "border" => vec![prop(&["kind", "style"], "Biçim", Conv::Choice(BORDER))],
        _ => Vec::new(),
    }
}

fn at<'a>(v: &'a Json, path: &[&str]) -> Option<&'a Json> {
    path.iter().try_fold(v, |v, k| v.get(*k))
}

/// A map's fixed view; none for another item or an atlas map (docs/adr/0206 §1).
pub(crate) fn fixed_map(i: &Item) -> Option<&kentos_sheet::kinds::FixedView> {
    match &i.kind {
        kentos_sheet::kinds::ItemKind::Map(m) => match &m.view {
            kentos_sheet::kinds::MapView::Fixed(v) => Some(v),
            kentos_sheet::kinds::MapView::Atlas(_) => None,
        },
        _ => None,
    }
}

/// The item's value of a property, as the inspector shows it.
pub(crate) fn read(item_json: &Json, p: &Prop, maps: &[(String, String)]) -> Value {
    let Some(v) = at(item_json, p.path) else {
        return Value::Null;
    };
    match p.conv {
        Conv::Text | Conv::Optional => v
            .as_str()
            .map_or(Value::Null, |s| Value::Text(s.to_owned())),
        Conv::Int => v.as_i64().map_or(Value::Null, Value::Integer),
        Conv::Mm => v.as_f64().map_or(Value::Null, |x| Value::Real(x / 1000.0)),
        Conv::Deg | Conv::Deg2 => v.as_f64().map_or(Value::Null, |x| Value::Real(x / 1000.0)),
        Conv::Bool => v.as_bool().map_or(Value::Null, Value::Bool),
        Conv::Choice(options) => {
            let raw = match v {
                Json::String(s) => s.clone(),
                other => other.to_string(),
            };
            Value::Text(
                options
                    .iter()
                    .find(|(k, _)| *k == raw)
                    .map_or(raw.clone(), |(_, label)| (*label).to_owned()),
            )
        }
        Conv::Map => {
            let id = v.as_str().unwrap_or_default();
            Value::Text(
                maps.iter()
                    .find(|(m, _)| m == id)
                    .map_or_else(|| id.to_owned(), |(_, n)| n.clone()),
            )
        }
    }
}

/// The JSON written for a value the inspector gave; none when it cannot be written (an empty number).
pub(crate) fn write(p: &Prop, value: &Value, maps: &[(String, String)]) -> Option<Json> {
    let leaf = match (p.conv, value) {
        (Conv::Text, Value::Text(s)) => json!(s),
        (Conv::Text, Value::Null) => json!(""),
        (Conv::Optional, Value::Text(s)) if !s.trim().is_empty() => json!(s),
        (Conv::Optional, Value::Text(_) | Value::Null) => Json::Null,
        (Conv::Int, v) => json!(v.as_f64()?.round() as i64),
        (Conv::Mm | Conv::Deg | Conv::Deg2, v) => json!((v.as_f64()? * 1000.0).round() as i64),
        (Conv::Bool, Value::Bool(b)) => json!(b),
        (Conv::Choice(options), Value::Text(label)) => {
            let raw = options.iter().find(|(_, l)| l == label).map(|(k, _)| *k)?;
            raw.parse::<i64>().map_or_else(|_| json!(raw), |n| json!(n))
        }
        (Conv::Map, Value::Text(name)) => json!(
            maps.iter()
                .find(|(_, n)| n == name)
                .map(|(id, _)| id.clone())?
        ),
        _ => return None,
    };
    // {"kind": {"style": {"size": 2500}}}: a merge that changes only that leaf.
    Some(
        p.path
            .iter()
            .rev()
            .fold(leaf, |inner, key| json!({ *key: inner })),
    )
}

/// The inspector field of a property.
pub(crate) fn field_of(p: &Prop, maps: &[(String, String)]) -> Field {
    let mut f = match p.conv {
        Conv::Text | Conv::Optional => Field::text(p.label),
        Conv::Int => Field::integer(p.label),
        // The display rule (ADR 0149): a point, never a comma.
        Conv::Mm => Field::real(p.label, 1).unit("mm").point(),
        Conv::Deg => Field::real(p.label, 1).unit("°").point(),
        Conv::Deg2 => Field::real(p.label, 2).unit("°").point(),
        Conv::Bool => Field::boolean(p.label),
        Conv::Choice(options) => Field::choice(p.label, options.iter().map(|(_, l)| *l)),
        Conv::Map => Field::choice(p.label, maps.iter().map(|(_, n)| n.clone())),
    };
    if p.multiline {
        f = f.multiline();
    }
    if !p.description.is_empty() {
        f = f.description(p.description);
    }
    f
}

/// A property of the chosen items as the inspector holds it.
#[derive(Clone, Debug)]
pub struct KindField {
    pub(crate) prop: Prop,
    pub field: Field,
    pub value: Value,
    /// The chosen items differ here.
    pub varies: bool,
}

/// The sheet's maps by id and name, for the fields that name one.
pub(crate) fn maps_of(items: &[Item]) -> Vec<(String, String)> {
    items
        .iter()
        .filter(|i| matches!(i.kind, kentos_sheet::kinds::ItemKind::Map(_)))
        .map(|i| (i.id.clone(), i.name.clone()))
        .collect()
}

/// The kind fields of the chosen items: those of their kind when they are all one kind.
pub(crate) fn refresh_fields(d: &mut Designer) {
    d.north_info = north_info_of(d);
    let chosen = d.chosen();
    let constraints = chosen.iter().map(|i| i.constraints).collect();
    let kind = chosen.first().map(|i| i.kind.type_name());
    if chosen.is_empty() || chosen.iter().any(|i| Some(i.kind.type_name()) != kind) {
        d.kind_fields.clear();
        d.chosen_constraints = constraints;
        return;
    }
    let maps = maps_of(d.items());
    let jsons: Vec<Json> = chosen
        .iter()
        .filter_map(|i| serde_json::to_value(i).ok())
        .collect();
    // A north arrow's typed declination and its year: only while it is typed by hand (the year
    // then shown empty when there is none, to be typed).
    let hand = chosen.iter().all(|i| match &i.kind {
        kentos_sheet::kinds::ItemKind::NorthArrow(k) => by_hand(k),
        _ => false,
    });
    let mut out = Vec::new();
    for p in props_of(kind.unwrap_or_default()) {
        let typed = matches!(
            p.path,
            ["kind", "declination"] | ["kind", "declinationYear"]
        ) && kind == Some("northArrow");
        if typed && !hand {
            continue;
        }
        let values: Vec<Value> = jsons.iter().map(|j| read(j, &p, &maps)).collect();
        // A property none of the items has (a map without a fixed view's scale) is not shown.
        if values.iter().all(Value::is_null)
            && !matches!(p.conv, Conv::Text | Conv::Optional | Conv::Map)
            && !typed
        {
            continue;
        }
        let varies = values.windows(2).any(|w| w[0] != w[1]);
        out.push(KindField {
            field: field_of(&p, &maps),
            value: if varies {
                Value::Null
            } else {
                values.into_iter().next().unwrap_or_default()
            },
            varies,
            prop: p,
        });
    }
    d.kind_fields = out;
    d.chosen_constraints = constraints;
}

/// Whether a north arrow shows magnetic north (the diagram always does): the web's `showsMagnetic`.
pub(crate) fn shows_magnetic(k: &kentos_sheet::kinds::NorthArrowItem) -> bool {
    use kentos_sheet::kinds::{NorthArrowStyle, NorthKind};
    k.north == NorthKind::Magnetic || k.style == NorthArrowStyle::Diagram
}

/// Whether its declination is typed by hand (a book of before the model: when one was typed).
pub(crate) fn by_hand(k: &kentos_sheet::kinds::NorthArrowItem) -> bool {
    k.declination_hand.unwrap_or(k.declination != 0)
}

/// What the north arrow chosen alone shows, from the core (worked out when the choice or the
/// sheet changes, not every frame).
fn north_info_of(d: &Designer) -> Option<(String, Option<kentos_sheet::display::NorthInfo>)> {
    let chosen = d.chosen();
    let [one] = chosen.as_slice() else {
        return None;
    };
    let kentos_sheet::kinds::ItemKind::NorthArrow(k) = &one.kind else {
        return None;
    };
    if !shows_magnetic(k) {
        return None;
    }
    let sheet = d.open.clone()?;
    let inputs = d.inputs(kentos_sheet::display::RenderMode::Design);
    Some((
        one.id.clone(),
        kentos_sheet::display::north_info(&d.book, &sheet, &one.id, &inputs).ok(),
    ))
}

/// The model's declination at the arrow's place on its date, when the core gave both (for a value
/// typed by hand, what the model would write there and then).
pub(crate) fn model_declination(info: &kentos_sheet::display::NorthInfo) -> Option<f64> {
    kentos_sheet::wmm::declination(info.lat?, info.lon?, info.year?)
}

/// An angle in degrees as the paper writes it: “6°19' D”, “2°10' B” (east positive): the web's `degreesText`.
pub(crate) fn degrees_text(deg: f64) -> String {
    let total = (deg.abs() * 60.0).round() as i64;
    format!(
        "{}°{:02}' {}",
        total / 60,
        total % 60,
        if deg < 0.0 { "B" } else { "D" }
    )
}

/// The declination as the inspector writes it: “6°19' D · WMM2025 · 2026-10”, “2°10' B · elle ·
/// 2024”, or why it is not known (the web's `declinationText`).
pub(crate) fn declination_text(info: &kentos_sheet::display::NorthInfo) -> String {
    use kentos_sheet::display::{DeclinationSource, NorthMissing};
    match (info.missing, info.declination) {
        (Some(NorthMissing::NoPlace), _) => {
            "Hesaplanamıyor: haritanın konumu yok (koordinat sistemi TM ya da UTM değil)".into()
        }
        (Some(NorthMissing::NoDate), _) | (_, None) => "Hesaplanamıyor: paftanın tarihi yok".into(),
        (None, Some(d)) if info.source == Some(DeclinationSource::Hand) => format!(
            "{} · elle{}",
            degrees_text(d),
            info.hand_year
                .map_or_else(String::new, |y| format!(" · {y}"))
        ),
        (None, Some(d)) => format!(
            "{} · {} · {}",
            degrees_text(d),
            info.model,
            info.date
                .as_deref()
                .and_then(|d| d.get(..7))
                .unwrap_or_default()
        ),
    }
}

/// Where the date the declination is for comes from (the web's `FROM`), and whether it is outside
/// the model's years.
pub(crate) fn date_hint(info: &kentos_sheet::display::NorthInfo) -> String {
    use kentos_sheet::display::DateSource;
    let from = match info.date_source {
        Some(DateSource::Sheet) => "paftanın “tarih” değişkeni",
        Some(DateSource::Project) => "projenin “tarih” değişkeni",
        Some(DateSource::Today) => "bugün: paftada ve projede “tarih” değişkeni yok",
        None => "tarih yok",
    };
    format!(
        "{from}.{}",
        if info.in_model == Some(false) {
            " Tarih modelin geçerlilik döneminin dışında: değer yaklaşıktır."
        } else {
            ""
        }
    )
}

/// What the declination's value is from, under it (the web's hint of “Manyetik sapma”).
pub(crate) fn value_hint(info: &kentos_sheet::display::NorthInfo, hand: bool) -> String {
    if hand {
        return match model_declination(info) {
            Some(m) => format!(
                "Modelin değeri: {} · {}{}.",
                degrees_text(m),
                info.model,
                info.date
                    .as_deref()
                    .and_then(|d| d.get(..7))
                    .map_or_else(String::new, |d| format!(" · {d}"))
            ),
            None => "Elle girilen değer yazılır.".into(),
        };
    }
    let place = match (info.lat, info.lon) {
        (Some(lat), Some(lon)) => format!(
            " ({}° K, {}° D)",
            kentos_geometry_core::display::fixed(lat, 4),
            kentos_geometry_core::display::fixed(lon, 4)
        ),
        _ => String::new(),
    };
    format!(
        "Haritanın merkezinde{place}, paftanın tarihinde; NOAA'nın Dünya Manyetik Modeli ({}: {}–{}).",
        info.model, info.valid_from, info.valid_until
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The declination's words are the web's (`northSection.ts`).
    #[test]
    fn the_declination_is_written_as_the_web_writes_it() {
        use kentos_sheet::display::{DateSource, DeclinationSource, NorthInfo, NorthMissing};
        assert_eq!(degrees_text(6.3167), "6°19' D");
        assert_eq!(degrees_text(-2.1667), "2°10' B");
        let info = NorthInfo {
            map: Some("harita".into()),
            lat: Some(39.75),
            lon: Some(35.85),
            convergence: Some(-0.1),
            declination: Some(6.3167),
            source: Some(DeclinationSource::Model),
            hand_year: None,
            missing: None,
            model: "WMM2025".into(),
            valid_from: 2025.0,
            valid_until: 2030.0,
            date: Some("2026-10-03".into()),
            date_source: Some(DateSource::Today),
            year: Some(2026.75),
            in_model: Some(true),
        };
        assert_eq!(declination_text(&info), "6°19' D · WMM2025 · 2026-10");
        assert_eq!(
            value_hint(&info, false),
            "Haritanın merkezinde (39.7500° K, 35.8500° D), paftanın tarihinde; NOAA'nın Dünya Manyetik Modeli (WMM2025: 2025–2030)."
        );
        assert_eq!(
            date_hint(&info),
            "bugün: paftada ve projede “tarih” değişkeni yok."
        );
        let hand = NorthInfo {
            source: Some(DeclinationSource::Hand),
            declination: Some(-2.1667),
            hand_year: Some(2024),
            ..info.clone()
        };
        assert_eq!(declination_text(&hand), "2°10' B · elle · 2024");
        assert!(value_hint(&hand, true).starts_with("Modelin değeri: "));
        let old = NorthInfo {
            date_source: Some(DateSource::Sheet),
            in_model: Some(false),
            ..info.clone()
        };
        assert_eq!(
            date_hint(&old),
            "paftanın “tarih” değişkeni. Tarih modelin geçerlilik döneminin dışında: değer yaklaşıktır."
        );
        let nowhere = NorthInfo {
            missing: Some(NorthMissing::NoPlace),
            declination: None,
            ..info
        };
        assert_eq!(
            declination_text(&nowhere),
            "Hesaplanamıyor: haritanın konumu yok (koordinat sistemi TM ya da UTM değil)"
        );
    }

    #[test]
    fn a_property_is_read_and_written_as_a_merge_of_its_path() {
        let p = prop(&["kind", "style", "size"], "Boyut", Conv::Mm);
        let item = json!({ "kind": { "type": "text", "style": { "size": 2500 } } });
        assert_eq!(read(&item, &p, &[]), Value::Real(2.5));
        assert_eq!(
            write(&p, &Value::Real(3.5), &[]),
            Some(json!({ "kind": { "style": { "size": 3500 } } }))
        );
        let w = prop(
            &["kind", "style", "weight"],
            "Kalınlık",
            Conv::Choice(WEIGHTS),
        );
        assert_eq!(
            read(&json!({ "kind": { "style": { "weight": 600 } } }), &w, &[]),
            Value::Text("Yarı kalın".into())
        );
        assert_eq!(
            write(&w, &Value::Text("Kalın".into()), &[]),
            Some(json!({ "kind": { "style": { "weight": 700 } } }))
        );
        let a = prop(&["kind", "align"], "Hiza", Conv::Choice(ALIGN));
        assert_eq!(
            write(&a, &Value::Text("Sağ".into()), &[]),
            Some(json!({ "kind": { "align": "right" } }))
        );
        let m = prop(&["kind", "map"], "Harita", Conv::Map);
        let maps = vec![("m1".to_owned(), "Harita".to_owned())];
        assert_eq!(
            read(&json!({ "kind": { "map": "m1" } }), &m, &maps),
            Value::Text("Harita".into())
        );
        assert_eq!(
            write(&m, &Value::Text("Harita".into()), &maps),
            Some(json!({ "kind": { "map": "m1" } }))
        );
        assert_eq!(
            write(
                &prop(&["kind", "segments"], "Bölüm", Conv::Int),
                &Value::Null,
                &[]
            ),
            None
        );
    }

    #[test]
    fn every_kind_s_properties_are_its_own() {
        // Each path exists in the kind's defaults (a typo would never show).
        for kind in [
            "map",
            "text",
            "scaleBar",
            "northArrow",
            "legend",
            "picture",
            "shape",
            "line",
            "table",
            "coordinateList",
            "border",
        ] {
            let item = Item::new(
                "x",
                "x",
                kentos_sheet::units::RectUm::new(0, 0, 10, 10),
                kentos_sheet::kinds::default_kind(kind).unwrap(),
            );
            let j = serde_json::to_value(&item).unwrap();
            for p in props_of(kind) {
                // Optional fields of the defaults (a map's link, a shape's fill, a typed
                // declination's year) may be absent.
                let optional = matches!(p.conv, Conv::Map | Conv::Optional)
                    || p.path.last() == Some(&"fill")
                    || p.path.last() == Some(&"zebra")
                    || p.path.last() == Some(&"declinationYear");
                assert!(optional || at(&j, p.path).is_some(), "{kind}: {:?}", p.path);
            }
        }
    }
}
