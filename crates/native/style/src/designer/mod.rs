//! The symbol designer's model (the web's `ui/style/designerModel.ts`,
//! docs/adr/0094): what each symbol layer type is called and how a new one
//! starts, a layer's one-line summary for the list, form patches turned into
//! layers, new drafts, the layer style slots' starting symbols, the window's
//! titles and words, and the preview's scale. The layer list's edits are in
//! [`list`]. Symbols stay the JSON they are saved as, so fields this side
//! does not edit survive. Both platforms are held to
//! `fixtures/style/v1/designer.json`.

pub mod list;

use kentos_style_core::js::number;
use kentos_style_core::js::text::{lower_tr, trim};
use serde_json::{Map, Value, json};

use crate::classify::{js_round, rounded};
use crate::preview::Geometry;
use crate::renderer::GeometryClass;

pub use list::{
    LayerPath, add_layer, add_parent, can_move, can_remove, duplicate_layer, layer_at, move_layer,
    put_layer, remove_layer, set_enabled, uid,
};

/// The kinds of symbol, as their JSON `type` names them.
pub const KINDS: [&str; 3] = ["fill", "line", "marker"];

/// A layer type's name in the list and the form (`LAYER_LABEL`).
pub fn label(layer_type: &str) -> &'static str {
    match layer_type {
        "simpleFill" => "Dolgu",
        "hatchFill" => "Tarama",
        "patternFill" => "Desen",
        "imageFill" => "Görüntü dolgusu",
        "centroidMarker" => "İç noktada işaret",
        "simpleLine" => "Çizgi",
        "markerLine" => "Çizgi boyunca işaret",
        "shape" => "Şekil",
        "svg" => "SVG çizimi",
        "text" => "Yazı",
        "raster" => "Görüntü",
        _ => "Bilinmeyen katman",
    }
}

/// The layer types a kind of symbol takes, in the order “Katman ekle” lists them (`LAYER_TYPES`).
pub fn layer_types(kind: &str) -> &'static [&'static str] {
    match kind {
        "fill" => &[
            "simpleFill",
            "hatchFill",
            "patternFill",
            "imageFill",
            "simpleLine",
            "markerLine",
            "centroidMarker",
        ],
        "line" => &["simpleLine", "markerLine"],
        "marker" => &["shape", "text", "svg", "raster"],
        _ => &[],
    }
}

/// A layer's type.
pub fn type_of(layer: &Value) -> &str {
    layer.get("type").and_then(Value::as_str).unwrap_or("")
}

/// Whether a layer holds a marker symbol of its own (edited as child rows).
pub fn has_marker(layer: &Value) -> bool {
    matches!(
        type_of(layer),
        "markerLine" | "patternFill" | "centroidMarker"
    )
}

fn dot(id: &str) -> Value {
    json!({ "id": id, "type": "shape", "shape": "circle", "size": 1, "fill": "ink" })
}

/// A new layer of `layer_type` with `id`, in a symbol of kind `context` (`newLayer`).
pub fn new_layer(layer_type: &str, id: &str, context: &str) -> Value {
    match layer_type {
        "simpleFill" => json!({ "id": id, "type": layer_type, "color": "#C9D6E3" }),
        "hatchFill" => {
            json!({ "id": id, "type": layer_type, "angle": 45, "spacing": 2, "width": 0.2, "color": "ink" })
        }
        "patternFill" => json!({
            "id": id, "type": layer_type, "spacingX": 3, "spacingY": 3,
            "marker": { "type": "marker", "layers": [dot("0")] },
        }),
        "imageFill" => json!({ "id": id, "type": layer_type, "asset": "", "tileSize": 5 }),
        "centroidMarker" => json!({
            "id": id, "type": layer_type,
            "marker": { "type": "marker", "layers": [{
                "id": "0", "type": "text", "text": { "expr": "etiket", "fallback": "A" },
                "size": 3, "font": "sans", "weight": 700, "color": "ink",
            }] },
        }),
        "simpleLine" => json!({
            "id": id, "type": layer_type, "color": "ink",
            "width": if context == "fill" { 0.25 } else { 0.35 },
        }),
        "markerLine" => json!({
            "id": id, "type": layer_type, "placement": "interval", "interval": 6, "offsetAlong": 3,
            "rotate": true, "marker": { "type": "marker", "layers": [dot("0")] },
        }),
        "shape" => json!({
            "id": id, "type": layer_type, "shape": "circle", "size": 3, "fill": "ink",
            "stroke": null, "strokeWidth": 0.2,
        }),
        "svg" => json!({ "id": id, "type": layer_type, "asset": "", "size": 5, "fill": "ink" }),
        "text" => json!({
            "id": id, "type": layer_type, "text": "A", "size": 3, "font": "sans", "weight": 700,
            "color": "ink",
        }),
        "raster" => json!({ "id": id, "type": layer_type, "asset": "", "size": 5 }),
        _ => json!({ "id": id, "type": layer_type }),
    }
}

// ── Option lists ───────────────────────────────────────────────────────

/// A choice of a select: its JSON value and what the form calls it.
pub type Choice = (&'static str, &'static str);

pub const UNITS: [Choice; 3] = [("mm", "Kâğıt mm"), ("px", "Ekran px"), ("m", "Harita m")];

pub const SHAPES: [Choice; 20] = [
    ("circle", "Daire"),
    ("ring", "Halka (noktalı)"),
    ("square", "Kare"),
    ("rectangle", "Dikdörtgen"),
    ("diamond", "Baklava"),
    ("triangle", "Üçgen"),
    ("pentagon", "Beşgen"),
    ("hexagon", "Altıgen"),
    ("octagon", "Sekizgen"),
    ("star", "Yıldız"),
    ("cross", "Artı"),
    ("x", "Çarpı"),
    ("line", "Çizgi"),
    ("arrow", "Ok"),
    ("arrowhead", "Ok ucu"),
    ("chevron", "Açık ok ucu (V)"),
    ("semicircle", "Yarım daire"),
    ("quartercircle", "Çeyrek daire"),
    ("gear", "Dişli"),
    ("arc", "Yay (açık)"),
];

/// Shapes drawn as lines only: they have no fill and no hole.
pub const OPEN_SHAPES: [&str; 6] = ["cross", "x", "line", "arrow", "chevron", "arc"];

pub const PLACEMENTS: [Choice; 7] = [
    ("interval", "Aralıklı"),
    ("vertex", "Her köşede"),
    ("innerVertex", "İç köşelerde"),
    ("first", "Başta"),
    ("last", "Sonda"),
    ("center", "Ortada"),
    ("segmentCenter", "Kenar ortalarında"),
];

pub const ANCHORS: [Choice; 9] = [
    ("center", "Orta"),
    ("top", "Üst"),
    ("bottom", "Alt"),
    ("left", "Sol"),
    ("right", "Sağ"),
    ("top-left", "Sol üst"),
    ("top-right", "Sağ üst"),
    ("bottom-left", "Sol alt"),
    ("bottom-right", "Sağ alt"),
];

pub const FONTS: [Choice; 5] = [
    ("sans", "Arial"),
    ("narrow", "Arial Narrow"),
    ("serif", "Times"),
    ("ui", "Arayüz yazısı"),
    ("mono", "Eş aralıklı"),
];

pub const WEIGHTS: [Choice; 5] = [
    ("400", "Normal"),
    ("500", "Orta"),
    ("600", "Yarı kalın"),
    ("700", "Kalın"),
    ("900", "Siyah (Arial Black)"),
];

pub const CAPS: [Choice; 3] = [("butt", "Düz"), ("round", "Yuvarlak"), ("square", "Kare")];

pub const RINGS: [Choice; 3] = [
    ("all", "Hepsi"),
    ("exterior", "Yalnızca dış sınır"),
    ("interior", "Yalnızca adalar"),
];

pub const WAVES: [Choice; 4] = [
    ("none", "Düz çizgi"),
    ("sine", "Dalga (sinüs)"),
    ("zigzag", "Zikzak"),
    ("square", "Kare dalga"),
];

pub const POSITIONS: [Choice; 2] = [
    ("pointOnSurface", "Alanın içinde (her zaman)"),
    ("centroid", "Ağırlık merkezi"),
];

/// A choice's label by its value.
pub fn choice_label(list: &[Choice], value: &str) -> Option<&'static str> {
    list.iter().find(|(v, _)| *v == value).map(|(_, l)| *l)
}

// ── JavaScript's values, where the TypeScript leaned on them ──────────

/// A number as JSON writes it: whole numbers without a point, as JavaScript's.
pub fn js_value(x: f64) -> Value {
    if !x.is_finite() {
        return Value::Null;
    }
    if x.trunc() == x && x.abs() < 9_007_199_254_740_992.0 {
        // -0 is written 0, as JSON.stringify does.
        return Value::from(x as i64);
    }
    Value::from(x)
}

/// JavaScript's truthiness of a JSON value (an absent one is false).
pub fn truthy(v: Option<&Value>) -> bool {
    match v {
        None | Some(Value::Null) => false,
        Some(Value::Bool(b)) => *b,
        Some(Value::Number(n)) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Array(_) | Value::Object(_)) => true,
    }
}

/// A value as a template string writes it (`${v}`).
fn js_text(v: Option<&Value>) -> String {
    match v {
        None => "undefined".into(),
        Some(Value::Null) => "null".into(),
        Some(Value::Bool(b)) => b.to_string(),
        Some(Value::Number(n)) => number::to_string(n.as_f64().unwrap_or(f64::NAN)),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Array(a)) => a
            .iter()
            .map(|x| match x {
                Value::Null => String::new(),
                x => js_text(Some(x)),
            })
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".into(),
    }
}

/// JavaScript's `Number(v)` for what a patch holds.
fn js_number_of(v: Option<&Value>) -> f64 {
    match v {
        None => f64::NAN,
        Some(Value::Null) => 0.0,
        Some(Value::Bool(b)) => f64::from(u8::from(*b)),
        Some(Value::Number(n)) => n.as_f64().unwrap_or(f64::NAN),
        Some(Value::String(s)) => crate::classify::js_number(s),
        Some(_) => f64::NAN,
    }
}

// ── Summaries ──────────────────────────────────────────────────────────

/// A number to two decimals as the list writes it; “ƒ” for one from an expression.
fn num(v: Option<&Value>) -> String {
    match v {
        Some(Value::Number(n)) => rounded(n.as_f64().unwrap_or(f64::NAN), 2),
        _ => "ƒ".into(),
    }
}

fn unit_of(layer: &Value) -> &'static str {
    match layer.get("unit").and_then(Value::as_str) {
        Some("px") => "px",
        Some("m") => "m",
        _ => "mm",
    }
}

/// The suffix a count takes as Turkish reads it: 2'li, 3'lü, 6'lı, 9'lu, 10'lu, 20'li (`countSuffix`).
pub fn count_suffix(n: f64) -> &'static str {
    const ONES: [&str; 10] = ["", "li", "li", "lü", "lü", "li", "lı", "li", "li", "lu"];
    const TENS: [&str; 10] = ["lü", "lu", "li", "lu", "lı", "li", "lı", "li", "li", "lı"];
    let k = n.abs().trunc() as u64;
    if k % 10 != 0 {
        return ONES[(k % 10) as usize];
    }
    if k % 100 != 0 {
        return TENS[((k % 100) / 10) as usize];
    }
    if k % 1000 != 0 { "lü" } else { "li" }
}

/// A layer's line in the list: its colour, size or placement at a glance (`summary`).
pub fn summary(layer: &Value) -> String {
    let g = |k: &str| layer.get(k);
    let u = unit_of(layer);
    match type_of(layer) {
        "simpleFill" => match g("color") {
            Some(Value::String(c)) => c.clone(),
            _ => "ifadeden renk".into(),
        },
        "hatchFill" => format!("{}° · {} {u} aralık", js_text(g("angle")), num(g("spacing"))),
        "patternFill" => format!(
            "{} × {} {u}{}{}",
            num(g("spacingX")),
            num(g("spacingY")),
            if truthy(g("stagger")) {
                ", şaşırtmalı"
            } else {
                ""
            },
            if truthy(g("jitter")) { ", dağınık" } else { "" },
        ),
        "imageFill" => {
            if truthy(g("asset")) {
                format!("{} {u} döşeme", num(g("tileSize")))
            } else {
                "çizim seçilmedi".into()
            }
        }
        "centroidMarker" => {
            if g("position").and_then(Value::as_str) == Some("centroid") {
                "ağırlık merkezinde".into()
            } else {
                "alanın içinde".into()
            }
        }
        "simpleLine" => {
            let dashed = g("dash")
                .and_then(Value::as_array)
                .is_some_and(|d| !d.is_empty());
            let offset = match g("offset") {
                o if !truthy(o) => String::new(),
                Some(Value::Number(_)) => format!(", {} {u} kaydırılmış", num(g("offset"))),
                _ => ", ifadeyle kaydırılmış".into(),
            };
            format!(
                "{} {u}{}{offset}{}",
                num(g("width")),
                if dashed { ", kesikli" } else { "" },
                if truthy(g("wave")) { ", dalgalı" } else { "" },
            )
        }
        "markerLine" => {
            let placement = g("placement").and_then(Value::as_str).unwrap_or("");
            if placement == "interval" {
                let count = g("group")
                    .filter(|gr| truthy(Some(gr)))
                    .and_then(|gr| gr.get("count"))
                    .and_then(Value::as_f64)
                    .filter(|c| *c > 1.0);
                let group = count.map_or_else(String::new, |c| {
                    format!(", {}'{}", number::to_string(c), count_suffix(c))
                });
                format!("her {} {u}{group}", num(g("interval")))
            } else {
                choice_label(&PLACEMENTS, placement)
                    .unwrap_or("")
                    .to_owned()
            }
        }
        "shape" => {
            let shape = g("shape").and_then(Value::as_str);
            let name = shape
                .and_then(|s| choice_label(&SHAPES, s))
                .map_or_else(|| js_text(g("shape")), str::to_owned);
            format!("{name} · {} {u}", num(g("size")))
        }
        "svg" | "raster" => {
            if truthy(g("asset")) {
                format!("{} {u}", num(g("size")))
            } else {
                "çizim seçilmedi".into()
            }
        }
        "text" => match g("text") {
            Some(Value::String(t)) => format!("“{t}”"),
            _ => "öznitelikten".into(),
        },
        _ => String::new(),
    }
}

// ── Form patches ───────────────────────────────────────────────────────

/// A form's change: fields and their new values, None for a field that goes
/// (JavaScript's undefined), and the helper keys the forms use (`offsetX`,
/// `jitterPct` …), in the order they were set.
pub type Patch = Vec<(String, Option<Value>)>;

/// A patch of one field.
pub fn set(key: &str, value: Value) -> Patch {
    vec![(key.to_owned(), Some(value))]
}

/// Turns a form patch (with the helper keys offsetX, jitterPct …) into a layer (`applyPatch`).
pub fn apply_patch(layer: &Value, patch: &[(String, Option<Value>)]) -> Value {
    let mut p: Vec<(String, Option<Value>)> = patch.to_vec();
    let has = |p: &[(String, Option<Value>)], k: &str| p.iter().any(|(x, _)| x == k);
    let get = |p: &[(String, Option<Value>)], k: &str| -> Option<Value> {
        p.iter().find(|(x, _)| x == k).and_then(|(_, v)| v.clone())
    };
    let take = |p: &mut Vec<(String, Option<Value>)>, k: &str| p.retain(|(x, _)| x != k);
    let put = |p: &mut Vec<(String, Option<Value>)>, k: &str, v: Option<Value>| {
        match p.iter_mut().find(|(x, _)| x == k) {
            Some(slot) => slot.1 = v,
            None => p.push((k.to_owned(), v)),
        }
    };
    let pair_of = |v: Option<&Value>| -> [Option<Value>; 2] {
        match v.and_then(Value::as_array) {
            Some(a) => [a.first().cloned(), a.get(1).cloned()],
            None => [Some(json!(0)), Some(json!(0))],
        }
    };
    // `a ?? b`: b when a is absent or null.
    let or = |a: Option<Value>, b: Option<Value>| a.filter(|v| !v.is_null()).or(b);
    if has(&p, "shiftX") || has(&p, "shiftY") {
        let cs = pair_of(layer.get("shift"));
        let x = js_number_of(or(get(&p, "shiftX"), cs[0].clone()).as_ref());
        let y = js_number_of(or(get(&p, "shiftY"), cs[1].clone()).as_ref());
        let nonzero = |v: f64| v != 0.0 && !v.is_nan();
        let shift = (nonzero(x) || nonzero(y)).then(|| json!([js_value(x), js_value(y)]));
        put(&mut p, "shift", shift);
        take(&mut p, "shiftX");
        take(&mut p, "shiftY");
    }
    if has(&p, "offsetX") || has(&p, "offsetY") {
        let offset = pair_of(layer.get("offset"));
        let x = or(get(&p, "offsetX"), offset[0].clone()).unwrap_or(Value::Null);
        let y = or(get(&p, "offsetY"), offset[1].clone()).unwrap_or(Value::Null);
        put(&mut p, "offset", Some(json!([x, y])));
        take(&mut p, "offsetX");
        take(&mut p, "offsetY");
    }
    for (from, to) in [
        ("holePct", "hole"),
        ("teethDepthPct", "teethDepth"),
        ("jitterPct", "jitter"),
        ("coveragePct", "coverage"),
    ] {
        if has(&p, from) {
            let v = js_number_of(get(&p, from).as_ref()) / 100.0;
            put(&mut p, to, Some(js_value(v)));
            take(&mut p, from);
        }
    }
    if has(&p, "groupCount") || has(&p, "groupSpacing") {
        let group = layer.get("group").filter(|gr| gr.is_object());
        let g_count = group.and_then(|gr| gr.get("count")).cloned();
        let g_spacing = group.and_then(|gr| gr.get("spacing")).cloned();
        let (g_count, g_spacing) = if group.is_some() {
            (g_count, g_spacing)
        } else {
            (Some(json!(1)), Some(json!(1)))
        };
        let count = js_round(js_number_of(or(get(&p, "groupCount"), g_count).as_ref())).max(1.0);
        let spacing = js_number_of(or(get(&p, "groupSpacing"), g_spacing).as_ref());
        let value = (count > 1.0)
            .then(|| json!({ "count": js_value(count), "spacing": js_value(spacing) }));
        put(&mut p, "group", value);
        take(&mut p, "groupCount");
        take(&mut p, "groupSpacing");
    }
    if has(&p, "haloWidth") {
        let width = js_number_of(get(&p, "haloWidth").as_ref());
        let halo = match layer.get("halo") {
            Some(h @ Value::Object(_)) => {
                let mut h = h.clone();
                if let Some(o) = h.as_object_mut() {
                    o.insert("width".into(), js_value(width));
                }
                Some(h)
            }
            Some(Value::Null) => Some(Value::Null),
            _ => None,
        };
        put(&mut p, "halo", halo);
        take(&mut p, "haloWidth");
    }
    if p.iter().any(|(k, v)| k == "dash" && matches!(v, Some(Value::Null))) {
        put(&mut p, "dash", None);
    }
    let mut out = layer.as_object().cloned().unwrap_or_default();
    for (k, v) in p {
        match v {
            Some(v) => {
                out.insert(k, v);
            }
            None => {
                out.remove(&k);
            }
        }
    }
    Value::Object(out)
}

// ── The window ─────────────────────────────────────────────────────────

/// A kind of symbol as the window's title names it (`KIND_TITLE`).
pub fn kind_title(kind: &str) -> &'static str {
    match kind {
        "fill" => "Alan sembolü",
        "line" => "Çizgi sembolü",
        _ => "İşaret sembolü",
    }
}

/// The sample objects a kind of symbol is previewed on, and their names (`GEOMETRIES`).
pub fn geometries(kind: &str) -> &'static [(Geometry, &'static str)] {
    match kind {
        "fill" => &[(Geometry::Area, "Alan"), (Geometry::Hole, "Adalı alan")],
        "line" => &[
            (Geometry::Line, "Düz"),
            (Geometry::Bent, "Kırık"),
            (Geometry::Area, "Alan kenarı"),
        ],
        _ => &[(Geometry::Point, "Nokta")],
    }
}

/// A sample's name in the fixture's words (`PreviewGeometry`).
pub fn geometry_key(g: Geometry) -> &'static str {
    match g {
        Geometry::Point => "point",
        Geometry::Line => "line",
        Geometry::Bent => "bent",
        Geometry::Area => "area",
        Geometry::Hole => "hole",
    }
}

/// A symbol being designed: its name, category and the symbol itself.
#[derive(Clone, Debug, PartialEq)]
pub struct Draft {
    pub name: String,
    pub path: Vec<String>,
    pub symbol: Value,
}

/// A new symbol of a kind: one layer of the first type it takes, named after
/// the kind, in `path` (Sembollerim by default) (`newDraft`).
pub fn new_draft(kind: &str, path: Option<&[String]>) -> Draft {
    let first = layer_types(kind).first().copied().unwrap_or("simpleFill");
    Draft {
        name: format!("Yeni {}", lower_tr(kind_title(kind))),
        path: path.map_or_else(|| vec!["Sembollerim".to_owned()], <[String]>::to_vec),
        symbol: json!({ "type": kind, "layers": [new_layer(first, "0", kind)] }),
    }
}

/// What a layer style's slot with no symbol and no plain look starts editing from (`defaultFor`).
pub fn default_for(class: GeometryClass) -> Value {
    match class {
        GeometryClass::Fill => json!({ "type": "fill", "layers": [
            { "id": "0", "type": "simpleFill", "color": "#C9D6E3" },
            { "id": "1", "type": "simpleLine", "color": "ink", "width": 0.2 },
        ] }),
        GeometryClass::Line => json!({ "type": "line", "layers": [
            { "id": "0", "type": "simpleLine", "color": "ink", "width": 0.35 },
        ] }),
        GeometryClass::Marker => json!({ "type": "marker", "layers": [
            { "id": "0", "type": "shape", "shape": "circle", "size": 2.4, "fill": "ink" },
        ] }),
    }
}

/// The window's title: “Alan sembolü tasarımcısı”, or for a symbol inside a
/// layer style “<slot>: alan sembolü”; “ •” when changed (`designerTitle`).
pub fn title(kind: &str, inline: Option<&str>, dirty: bool) -> String {
    let base = match inline {
        Some(slot) => format!("{slot}: {}", lower_tr(kind_title(kind))),
        None => format!("{} tasarımcısı", kind_title(kind)),
    };
    if dirty { format!("{base} •") } else { base }
}

/// The name and category a symbol is saved under: blank ones get “Adsız
/// sembol” in Sembollerim (`savedAs`).
pub fn saved_as(name: &str, path: &[String]) -> (String, Vec<String>) {
    let name = trim(name);
    (
        if name.is_empty() {
            "Adsız sembol".to_owned()
        } else {
            name.to_owned()
        },
        if path.is_empty() {
            vec!["Sembollerim".to_owned()]
        } else {
            path.to_vec()
        },
    )
}

/// The category field's text as a path: “Ana / Alt”, blanks dropped (`pathOf`).
pub fn path_of(text: &str) -> Vec<String> {
    text.split('/')
        .map(|s| trim(s).to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}

/// The preview's scale, paper millimetres to logical pixels (`ZOOM`).
pub mod zoom {
    pub const MIN: f64 = 1.0;
    pub const MAX: f64 = 40.0;
    /// One step of − and +, and of the wheel.
    pub const STEP: f64 = 1.25;
    pub const START: f64 = 4.0;
    /// 1:1: a millimetre of paper at 96 pixels an inch.
    pub const REAL: f64 = 96.0 / 25.4;
}

/// The scale after a step (`zoomed`).
pub fn zoomed(px_per_mm: f64, factor: f64) -> f64 {
    (px_per_mm * factor).max(zoom::MIN).min(zoom::MAX)
}

/// The scale as the preview's bar writes it (`zoomText`).
pub fn zoom_text(px_per_mm: f64) -> String {
    format!("1 mm = {} px", number::to_fixed(px_per_mm, 1))
}

/// The window's fixed words (`DESIGNER_TEXTS`).
pub mod texts {
    pub const LAYERS: &str = "Katmanlar";
    pub const ADD: &str = "Katman ekle";
    pub const INTO_SYMBOL: &str = "Sembole";
    pub const ORDER: &str = "Listede üstteki katman önce, alttaki en son (en üstte) çizilir.";
    pub const LAST_LAYER: &str = "Sembolün en az bir katmanı olmalı.";
    pub const ONE_POINT: &str = "Örnek: tek nokta";
    pub const PICK_LAYER: &str = "Bir katman seçin.";
    pub const INLINE: &str = "Bu sembol katman stilinin içindedir; kitaplığa yazılmaz.";
    pub const CANNOT_EDIT: &str =
        "Bu sembol düzenlenemez: sistem sembollerinin kopyası düzenlenir.";

    /// “Katman ekle”'s header over the marker's layer types.
    pub fn into_marker(parent: &str) -> String {
        format!("“{parent}” işaretine")
    }

    /// The form's note on a marker's layer: which layer places it.
    pub fn in_marker(parent: &str) -> String {
        format!("{parent} işaretinde")
    }

    /// Kaydet refused: the first problem, and how many more.
    pub fn not_saved(first: &str, more: usize) -> String {
        if more > 0 {
            format!("Kaydedilemedi: {first} (ve {more} sorun daha)")
        } else {
            format!("Kaydedilemedi: {first}")
        }
    }

    pub fn saved(name: &str) -> String {
        format!("“{name}” kaydedildi.")
    }
}

/// A JSON object's field, or an empty map for what is not an object.
pub fn object_of(v: &Value) -> Map<String, Value> {
    v.as_object().cloned().unwrap_or_default()
}
