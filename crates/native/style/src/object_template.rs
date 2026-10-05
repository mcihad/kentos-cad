//! Nesne şablonları (docs/adr/0176): a template is a drawing recipe for one kind
//! of object: the tool it is drawn with, the layer it goes on (found by
//! name, opened when the drawing lacks it), its colour, line weight, symbol,
//! attributes and label. Templates are the style library's third kind of item
//! (`kind: "template"`, [`crate::library`]). The web's rules are
//! `apps/web/src/model/objectTemplate.ts`; both pass `fixtures/style/v1/object-templates.json`.

use kentos_contracts::blocks::name_key;
use kentos_style_core::js::collate::compare_tr;
use kentos_style_core::js::number;
use serde_json::{Map, Value};

/// The tools a template draws with, and their names as the interface says them.
pub const TEMPLATE_TOOLS: [(&str, &str); 9] = [
    ("point", "Nokta"),
    ("line", "Çizgi"),
    ("polyline", "Çoklu çizgi"),
    ("polygon", "Kapalı alan"),
    ("rectangle", "Dikdörtgen"),
    ("rectangle3", "Döndürülmüş dikdörtgen"),
    ("circle", "Daire"),
    ("text", "Yazı"),
    ("blockInsert", "Blok ekle"),
];

/// A tool's name as the interface says it; the id itself when it is not a template's tool.
pub fn tool_label(tool: &str) -> &str {
    TEMPLATE_TOOLS
        .iter()
        .find(|(id, _)| *id == tool)
        .map_or(tool, |(_, label)| label)
}

/// The methods a template may name, by tool (their options in the tool catalog);
/// a tool not here has none to choose.
pub fn template_methods(tool: &str) -> &'static [&'static str] {
    match tool {
        "circle" => &["2N", "3N", "TTY", "TTT"],
        _ => &[],
    }
}

/// The heaviest line weight, mm (`MAX_LINE_WEIGHT`).
const MAX_LINE_WEIGHT: f64 = 100.0;
const LINE_TYPES: [&str; 4] = ["continuous", "dashed", "dashdot", "dotted"];
/// A text's alignments but the left of the baseline, which is no value (docs/adr/0145).
const TEXT_ALIGNS: [&str; 11] = [
    "baselineCenter",
    "baselineRight",
    "bottomLeft",
    "bottomCenter",
    "bottomRight",
    "middleLeft",
    "middleCenter",
    "middleRight",
    "topLeft",
    "topCenter",
    "topRight",
];

/// A colour: hex with or without alpha, or a theme token (the style file's rule).
fn color_ok(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if matches!(lower.as_str(), "ink" | "paper" | "fg" | "fg-dim") {
        return true;
    }
    let Some(hex) = lower.strip_prefix('#') else {
        return false;
    };
    (hex.len() == 6 || hex.len() == 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

/// Empty or only white space, as Unicode's `White_Space` has it.
fn blank(text: &str) -> bool {
    text.chars().all(char::is_whitespace)
}

fn weight_ok(v: &Value) -> bool {
    v.as_f64()
        .is_some_and(|w| w.is_finite() && (0.0..=MAX_LINE_WEIGHT).contains(&w))
}

/// A value as the web's `String(v)` writes it in a message.
fn shown(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_owned(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => number::to_string(n.as_f64().unwrap_or(f64::NAN)),
        // An array's items joined by commas, a missing one empty.
        Value::Array(items) => items
            .iter()
            .map(|x| if x.is_null() { String::new() } else { shown(x) })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".to_owned(),
    }
}

/// What is wrong with a template, each problem with where it is (`where_`: “öğe 3
/// (Parsel sınırı)”), in the order the fields are read; none for a template the
/// app can draw with. A template read from a file or a library is untrusted data.
pub fn template_issues(template: &Value, where_: &str) -> Vec<String> {
    let Some(template) = template.as_object() else {
        return vec![format!("{where_}: şablon tanımı yok")];
    };
    let mut out = Vec::new();
    let mut say = |text: String| out.push(format!("{where_}: {text}"));
    let get = |key: &str| template.get(key);
    let tool = get("tool").and_then(Value::as_str).unwrap_or("");
    let known = TEMPLATE_TOOLS
        .iter()
        .find(|(id, _)| *id == tool)
        .map(|(_, l)| *l);
    if known.is_none() {
        say(format!(
            "bilinmeyen araç “{}”",
            get("tool").map_or_else(|| "undefined".to_owned(), shown)
        ));
    }
    if let (Some(method), Some(label)) = (get("method"), known) {
        let methods = template_methods(tool);
        if methods.is_empty() {
            say(format!("{label} aracının seçilecek yöntemi yok"));
        } else if !method.as_str().is_some_and(|m| methods.contains(&m)) {
            say(format!("“{}” yöntemi {label} aracında yok", shown(method)));
        }
    }
    match get("layer").and_then(Value::as_object) {
        None => say("katmanı yok".to_owned()),
        Some(layer) => {
            if layer.get("name").and_then(Value::as_str).is_none_or(blank) {
                say("katmanın adı boş".to_owned());
            }
            match layer.get("path").and_then(Value::as_array) {
                Some(path) if path.iter().all(Value::is_string) => {
                    if path.iter().filter_map(Value::as_str).any(blank) {
                        say("katmanın yolunda boş ad var".to_owned());
                    }
                }
                _ => say("katmanın yolu metin listesi olmalı".to_owned()),
            }
            if let Some(c) = layer.get("color")
                && !c.as_str().is_some_and(color_ok)
            {
                say(format!("katmanın rengi geçersiz “{}”", shown(c)));
            }
            if let Some(t) = layer.get("lineType")
                && !t.as_str().is_some_and(|t| LINE_TYPES.contains(&t))
            {
                say(format!("katmanın çizgi tipi bilinmiyor “{}”", shown(t)));
            }
            if layer.get("lineWeight").is_some_and(|w| !weight_ok(w)) {
                say("katmanın kalınlığı 0 ile 100 mm arasında olmalı".to_owned());
            }
        }
    }
    if let Some(c) = get("color")
        && !c.as_str().is_some_and(color_ok)
    {
        say(format!("renk geçersiz “{}”", shown(c)));
    }
    if get("lineWeight").is_some_and(|w| !weight_ok(w)) {
        say("kalınlık 0 ile 100 mm arasında olmalı".to_owned());
    }
    if get("symbol").is_some_and(|s| s.as_str().is_none_or(blank)) {
        say("sembolün kimliği boş".to_owned());
    }
    if let Some(attrs) = get("attrs") {
        match attrs.as_object() {
            None => say("öznitelikler ad ve metin değer olmalı".to_owned()),
            // By name: the web sorts them the same way.
            Some(attrs) => {
                for (name, value) in attrs {
                    if blank(name) {
                        say("öznitelik adı boş".to_owned());
                    }
                    if !value.is_string() {
                        say(format!("“{name}” özniteliğinin değeri metin olmalı"));
                    }
                }
            }
        }
    }
    if get("label").is_some_and(|l| !l.is_string()) {
        say("etiket metin olmalı".to_owned());
    }
    if let Some(point) = get("point") {
        let texts = |p: &Map<String, Value>| {
            ["name", "code"]
                .iter()
                .all(|k| p.get(*k).is_none_or(Value::is_string))
        };
        if tool != "point" {
            say("ad ve kod yalnız nokta şablonunda olur".to_owned());
        } else if !point.as_object().is_some_and(texts) {
            say("noktanın adı ve kodu metin olmalı".to_owned());
        }
    }
    if let Some(text) = get("text") {
        if tool != "text" {
            say("yazı bilgisi yalnız yazı şablonunda olur".to_owned());
        } else {
            match text.as_object() {
                None => say("yazı bilgisi yok".to_owned()),
                Some(text) => {
                    if !text
                        .get("height")
                        .and_then(Value::as_f64)
                        .is_some_and(|h| h.is_finite() && h > 0.0)
                    {
                        say("yazının yüksekliği sıfırdan büyük olmalı".to_owned());
                    }
                    if let Some(a) = text.get("align")
                        && !a.as_str().is_some_and(|a| TEXT_ALIGNS.contains(&a))
                    {
                        say(format!("bilinmeyen hiza “{}”", shown(a)));
                    }
                    if text.get("mask").is_some_and(|m| !m.is_boolean()) {
                        say("zemin açık ya da kapalı olmalı".to_owned());
                    }
                }
            }
        }
    }
    if tool == "blockInsert" {
        if get("block").and_then(Value::as_str).is_none_or(blank) {
            say("blok şablonunun bloğu yok".to_owned());
        }
    } else if get("block").is_some() {
        say("blok yalnız blok şablonunda olur".to_owned());
    }
    out
}

/// A template as the app draws with it (docs/adr/0176 §3), read from a
/// library item's `template`.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Recipe {
    /// One of [`TEMPLATE_TOOLS`].
    pub tool: String,
    /// One of the tool's [`template_methods`].
    pub method: Option<String>,
    /// The layer's groups from the top, its name and its look when opened.
    pub layer_path: Vec<String>,
    pub layer_name: String,
    pub layer_color: Option<String>,
    pub layer_line_type: Option<kentos_contracts::LineType>,
    pub layer_line_weight: Option<f64>,
    /// The objects' own colour and line weight; none: their layer's.
    pub color: Option<String>,
    pub line_weight: Option<f64>,
    pub symbol: Option<String>,
    pub attrs: std::collections::BTreeMap<String, String>,
    pub label: Option<String>,
    /// A point template's first name and code.
    pub point_name: Option<String>,
    pub point_code: Option<String>,
    /// A text template's height, alignment and mask.
    pub text_height: Option<f64>,
    pub text_align: Option<String>,
    pub text_mask: bool,
    /// A block template's block, by name.
    pub block: Option<String>,
}

/// A template's recipe; none for one with an issue ([`template_issues`]).
pub fn read(template: &Value) -> Option<Recipe> {
    if !template_issues(template, "şablon").is_empty() {
        return None;
    }
    let text = |v: Option<&Value>| v.and_then(Value::as_str).map(str::to_owned);
    let layer = template.get("layer")?;
    let point = template.get("point");
    let written = template.get("text");
    Some(Recipe {
        tool: text(template.get("tool"))?,
        method: text(template.get("method")),
        layer_path: layer
            .get("path")
            .and_then(Value::as_array)
            .map(|p| {
                p.iter()
                    .filter_map(Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        layer_name: text(layer.get("name"))?,
        layer_color: text(layer.get("color")),
        layer_line_type: layer
            .get("lineType")
            .and_then(|t| serde_json::from_value(t.clone()).ok()),
        layer_line_weight: layer.get("lineWeight").and_then(Value::as_f64),
        color: text(template.get("color")),
        line_weight: template.get("lineWeight").and_then(Value::as_f64),
        symbol: text(template.get("symbol")),
        attrs: template
            .get("attrs")
            .and_then(Value::as_object)
            .map(|a| {
                a.iter()
                    .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
                    .collect()
            })
            .unwrap_or_default(),
        label: text(template.get("label")),
        point_name: text(point.and_then(|p| p.get("name"))),
        point_code: text(point.and_then(|p| p.get("code"))),
        text_height: written
            .and_then(|t| t.get("height"))
            .and_then(Value::as_f64),
        text_align: text(written.and_then(|t| t.get("align"))),
        text_mask: written
            .and_then(|t| t.get("mask"))
            .and_then(Value::as_bool)
            .unwrap_or(false),
        block: text(template.get("block")),
    })
}

/// A group of the Şablonlar panel (docs/adr/0176 §4): a category path
/// (empty: the templates without one) and its templates, by id and source.
#[derive(Clone, Debug, PartialEq)]
pub struct TemplateGroup {
    pub path: Vec<String>,
    pub items: Vec<(String, crate::library::Source)>,
}

/// Where a source's template goes among those of one name: the project's
/// first, then Kitaplığım's, then the system's.
fn source_rank(source: crate::library::Source) -> u8 {
    match source {
        crate::library::Source::Project => 0,
        crate::library::Source::User => 1,
        crate::library::Source::System => 2,
    }
}

/// Two category paths in Turkish order, part by part; a shorter path before
/// the longer one it begins; the empty one last.
fn compare_paths(a: &[&str], b: &[&str]) -> std::cmp::Ordering {
    if a.is_empty() || b.is_empty() {
        return a.is_empty().cmp(&b.is_empty());
    }
    a.iter()
        .zip(b)
        .map(|(x, y)| compare_tr(x, y))
        .find(|o| o.is_ne())
        .unwrap_or_else(|| a.len().cmp(&b.len()))
}

/// The library's object templates as the Şablonlar panel lists them
/// (docs/adr/0176 §4): every source's (not its symbols nor its assets),
/// grouped by their category paths. The groups go by their paths in Turkish
/// order, part by part, those without a category last; in a group the
/// templates go by their names in Turkish order, one name's templates
/// project first, then Kitaplığım, then the system's. `query`, its white
/// space around left out and folded the Turkish way (`name_key`), is found
/// in a template's name, description, category path, tool's name and
/// layer's name; an empty one keeps every template, and a group left without
/// one is not listed. The web's is `style/templateList.ts`; both pass
/// fixtures/style/v1/template-list.json.
pub fn listed(lib: &crate::library::StyleLibrary, query: &str) -> Vec<TemplateGroup> {
    let q = name_key(query.trim());
    let mut found: Vec<(&crate::library::Item, crate::library::Source)> = lib
        .items(None)
        .into_iter()
        .filter(|(item, _)| item.kind() == crate::library::ItemKind::Template)
        .filter(|(item, _)| {
            if q.is_empty() {
                return true;
            }
            let template = item.template();
            let field = |key: &str| template.and_then(|t| t.get(key)).and_then(Value::as_str);
            let layer = template
                .and_then(|t| t.get("layer"))
                .and_then(|l| l.get("name"))
                .and_then(Value::as_str);
            let path = item.path().join(" / ");
            [
                Some(item.name()),
                item.text("description"),
                Some(path.as_str()),
                Some(tool_label(field("tool").unwrap_or(""))),
                layer,
            ]
            .into_iter()
            .flatten()
            .any(|words| name_key(words).contains(&q))
        })
        .collect();
    found.sort_by(|(x, xs), (y, ys)| {
        compare_paths(&x.path(), &y.path())
            .then_with(|| compare_tr(x.name(), y.name()))
            .then_with(|| source_rank(*xs).cmp(&source_rank(*ys)))
    });
    let mut groups: Vec<TemplateGroup> = Vec::new();
    for (item, source) in found {
        let path: Vec<String> = item.path().into_iter().map(str::to_owned).collect();
        match groups.last_mut() {
            Some(group) if group.path == path => group.items.push((item.id().to_owned(), source)),
            _ => groups.push(TemplateGroup {
                path,
                items: vec![(item.id().to_owned(), source)],
            }),
        }
    }
    groups
}

/// The tool a template of a drawable kind draws with (Seçili nesneden şablon).
fn tool_of_kind(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "point" => "point",
        "line" => "line",
        "polyline" => "polyline",
        "polygon" => "polygon",
        "circle" => "circle",
        "text" => "text",
        "insert" => "blockInsert",
        _ => return None,
    })
}

/// An object kind's name as the interface says it (the web's `ENTITY_KIND_LABEL`).
fn kind_label(kind: &str) -> &'static str {
    match kind {
        "point" => "Nokta",
        "line" => "Çizgi",
        "polyline" => "Çoklu çizgi",
        "polygon" => "Kapalı alan",
        "circle" => "Daire",
        "arc" => "Yay",
        "ellipse" => "Elips",
        "spline" => "Eğri",
        "xline" => "Yardımcı çizgi",
        "ray" => "Işın",
        "text" => "Yazı",
        "dimension" => "Ölçü",
        "hatch" => "Tarama",
        "insert" => "Blok",
        "leader" => "Kılavuz",
        _ => "Nesne",
    }
}

/// The template made from a drawn object (Seçili nesneden şablon, docs/adr/0176
/// §4) and its name, its layer's: the tool is the object's kind's; the layer
/// is its own (`layers`, the drawing's tree), with the groups above it and its
/// look; the object's own colour, line weight and symbol go with it when it has
/// them, and so do its attributes and label, but a point's label is the
/// template's first name and its `Kod` its code (docs/adr/0152); a text gives
/// its height, alignment and mask, an insert its block by name (`block_name`).
/// Another kind is refused, said by its name. The web's is
/// `model/objectTemplate.ts`'s `templateFromObject`; both pass
/// fixtures/style/v1/template-from-object.json.
pub fn from_object(
    entity: &kentos_contracts::Entity,
    layers: &[kentos_contracts::LayerNode],
    block_name: impl Fn(&kentos_contracts::BlockId) -> Option<String>,
) -> Result<(String, Value), String> {
    use kentos_contracts::Entity;
    let kind = entity.kind();
    let Some(tool) = tool_of_kind(kind) else {
        return Err(format!(
            "{} nesnesinden şablon yapılamaz: şablon nokta, çizgi, çoklu çizgi, kapalı alan, daire, yazı ya da blok çizer.",
            kind_label(kind)
        ));
    };
    let base = entity.base();
    let (layer, groups) = find_node(layers, &base.layer_id, &mut Vec::new())
        .map_or((None, Vec::new()), |(node, groups)| (Some(node), groups));
    let name = layer.map_or_else(|| base.layer_id.clone(), |l| l.name.clone());
    let mut layer_value = Map::new();
    layer_value.insert("path".into(), Value::from(groups));
    layer_value.insert("name".into(), Value::from(name.clone()));
    if let Some(l) = layer {
        layer_value.insert("color".into(), Value::from(l.style.color.clone()));
        layer_value.insert(
            "lineType".into(),
            serde_json::to_value(l.style.line_type).unwrap_or(Value::Null),
        );
        layer_value.insert("lineWeight".into(), Value::from(l.style.line_weight));
    }
    let mut t = Map::new();
    t.insert("tool".into(), Value::from(tool));
    t.insert("layer".into(), Value::Object(layer_value));
    if let Some(color) = &base.color {
        t.insert("color".into(), Value::from(color.clone()));
    }
    if let Some(weight) = base.line_weight {
        t.insert("lineWeight".into(), Value::from(weight));
    }
    if let Some(symbol) = &base.symbol {
        t.insert("symbol".into(), Value::from(symbol.clone()));
    }
    let mut attrs = base.attrs.clone();
    let mut label = base.label.clone();
    if kind == "point" {
        let mut point = Map::new();
        if let Some(name) = label.take().filter(|l| !l.is_empty()) {
            point.insert("name".into(), Value::from(name));
        }
        if let Some(code) = attrs.remove("Kod").filter(|c| !c.is_empty()) {
            point.insert("code".into(), Value::from(code));
        }
        if !point.is_empty() {
            t.insert("point".into(), Value::Object(point));
        }
    }
    if !attrs.is_empty() {
        // By name: the map's own order.
        let attrs: Map<String, Value> = attrs
            .into_iter()
            .map(|(k, v)| (k, Value::from(v)))
            .collect();
        t.insert("attrs".into(), Value::Object(attrs));
    }
    if let Some(label) = label.filter(|l| !l.is_empty()) {
        t.insert("label".into(), Value::from(label));
    }
    match entity {
        Entity::Text(text) => {
            let mut written = Map::new();
            written.insert("height".into(), Value::from(text.height));
            if let Some(align) = text.align {
                written.insert("align".into(), Value::from(align.name()));
            }
            if text.mask {
                written.insert("mask".into(), Value::from(true));
            }
            t.insert("text".into(), Value::Object(written));
        }
        Entity::Insert(insert) => {
            let block = block_name(&insert.block).unwrap_or_else(|| insert.block.to_text());
            t.insert("block".into(), Value::from(block));
        }
        _ => {}
    }
    Ok((name, Value::Object(t)))
}

/// The node of `id` in `nodes` and the names of the groups above it, from the top.
fn find_node<'a>(
    nodes: &'a [kentos_contracts::LayerNode],
    id: &str,
    groups: &mut Vec<String>,
) -> Option<(&'a kentos_contracts::LayerNode, Vec<String>)> {
    for node in nodes {
        if node.id == id {
            return Some((node, groups.clone()));
        }
        if node.kind == kentos_contracts::LayerNodeType::Group {
            groups.push(node.name.clone());
            if let Some(found) = find_node(&node.children, id, groups) {
                return Some(found);
            }
            groups.pop();
        }
    }
    None
}

/// What a template's card and preview show (docs/adr/0176): its own symbol when
/// the library has it, else one made of the template's colour and line weight
/// (its layer's when it gives none) in its tool's shape: a dot for a point
/// or a block, a word for a text, a line for a line, an outline for an area.
/// The web's is `style/templateSymbol.ts`.
pub fn preview_symbol(template: &Value, library: &crate::library::StyleLibrary) -> Value {
    if let Some(own) = template
        .get("symbol")
        .and_then(Value::as_str)
        .and_then(|id| library.symbol(id))
    {
        return own.clone();
    }
    let layer = template.get("layer");
    let field = |key: &str| {
        template
            .get(key)
            .or_else(|| layer.and_then(|l| l.get(key)))
            .cloned()
    };
    let color = field("color").unwrap_or_else(|| Value::from("ink"));
    let width = field("lineWeight").unwrap_or_else(|| Value::from(0.35));
    match template.get("tool").and_then(Value::as_str).unwrap_or("") {
        "point" | "blockInsert" => serde_json::json!({ "type": "marker", "layers": [
            { "id": "p", "type": "shape", "shape": "circle", "size": 2.4, "fill": color }
        ] }),
        "text" => serde_json::json!({ "type": "marker", "layers": [
            { "id": "t", "type": "text", "text": "Abc", "size": 3.2, "color": color }
        ] }),
        "line" | "polyline" => serde_json::json!({ "type": "line", "layers": [
            { "id": "l", "type": "simpleLine", "color": color, "width": width }
        ] }),
        _ => serde_json::json!({ "type": "fill", "layers": [
            { "id": "l", "type": "simpleLine", "color": color, "width": width }
        ] }),
    }
}
