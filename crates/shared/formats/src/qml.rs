//! A layer's style as a QGIS layer style file, `.qml` (docs/adr/0555, plan/gis.md CBS-17): QGIS 4.2's `renderer-v2`
//! with its single symbol, categorized, graduated and rule-based renderers, and its simple fill, simple line, simple
//! marker and centroid fill symbol layers. Unlike SLD (docs/adr/0516) a QML says what QGIS draws: sizes keep their
//! unit (mm, pixel, metres in map units), a symbol keeps its opacity, a disabled layer stays disabled, rules nest.
//! What KentOS's style has and these classes have not (a hatch, a gradient, a pattern, a marker line, an SVG or text
//! marker, a data-defined value, an expression the filter grammar cannot say) is named, never dropped in silence;
//! read back, what QGIS has and KentOS has not (another symbol layer class, a label, a data-defined property) is
//! named too.
//!
//! A QML is for one geometry type: the layer's objects choose it (areas before lines before points). Rule filters are
//! QGIS expressions: `"Ad" = 'x'` where KentOS writes `[Ad] = 'x'`; both go through the filter grammar SLD's writer
//! and reader share (`sld::Filter`). QGIS turns markers clockwise, KentOS counter-clockwise.

use std::fmt::Write as _;

use serde_json::{Map, Value, json};

use crate::sld::{
    Filter, Lit, Op, colour, colour_with, esc, field_of, filter_text, fixed, num, parse_filter,
    parse_qgis, round3,
};
pub use crate::sld::{Kinds, Read, Written};

/// A point (1/72 inch), mm.
const POINT_MM: f64 = 25.4 / 72.0;

/// KentOS's marker shapes and QGIS's names for them; the others are named lost.
const SHAPES: [(&str, &str); 15] = [
    ("circle", "circle"),
    ("square", "square"),
    ("diamond", "diamond"),
    ("triangle", "triangle"),
    ("pentagon", "pentagon"),
    ("hexagon", "hexagon"),
    ("octagon", "octagon"),
    ("star", "star"),
    ("cross", "cross"),
    ("x", "cross2"),
    ("line", "line"),
    ("arrow", "arrow"),
    ("arrowhead", "filled_arrowhead"),
    ("chevron", "arrowhead"),
    ("semicircle", "semi_circle"),
];

/// QGIS's names KentOS draws as another shape of its own (read only).
const SHAPES_IN: [(&str, &str); 3] = [
    ("equilateral_triangle", "triangle"),
    ("quarter_circle", "quartercircle"),
    ("cross_fill", "cross"),
];

// ─── Expressions ──────────────────────────────────────────────────────────────────────────────────────

/// A filter as QGIS's expression.
fn qgis_text(f: &Filter) -> String {
    let field = |n: &str| format!("\"{}\"", n.replace('"', "\"\""));
    let lit = |l: &Lit| match l {
        Lit::Num(n) => num(*n),
        Lit::Text(s) => format!("'{}'", s.replace('\'', "''")),
    };
    let wrap = |p: &Filter, in_and: bool| match p {
        Filter::Or(_) if in_and => format!("({})", qgis_text(p)),
        Filter::And(_) if !in_and => format!("({})", qgis_text(p)),
        _ => qgis_text(p),
    };
    match f {
        Filter::Cmp(op, n, l) => {
            let op = if *op == Op::Ne { "<>" } else { op_text(*op) };
            format!("{} {op} {}", field(n), lit(l))
        }
        Filter::Between(n, a, b) => format!("{} BETWEEN {} AND {}", field(n), num(*a), num(*b)),
        Filter::Null(n) => format!("{} IS NULL", field(n)),
        Filter::Like(n, p) => format!("{} LIKE '{}'", field(n), p.replace('\'', "''")),
        Filter::And(parts) => parts
            .iter()
            .map(|p| wrap(p, true))
            .collect::<Vec<_>>()
            .join(" AND "),
        Filter::Or(parts) => parts
            .iter()
            .map(|p| wrap(p, false))
            .collect::<Vec<_>>()
            .join(" OR "),
        Filter::Not(inner) => match &**inner {
            Filter::Null(n) => format!("{} IS NOT NULL", field(n)),
            other => format!("NOT ({})", qgis_text(other)),
        },
    }
}

fn op_text(op: Op) -> &'static str {
    match op {
        Op::Eq => "=",
        Op::Ne => "<>",
        Op::Lt => "<",
        Op::Le => "<=",
        Op::Gt => ">",
        Op::Ge => ">=",
    }
}

// ─── Writing ──────────────────────────────────────────────────────────────────────────────────────────

/// A KentOS unit as QGIS's.
fn unit_out(unit: &str) -> &'static str {
    match unit {
        "m" => "RenderMetersInMapUnits",
        "px" => "Pixel",
        _ => "MM",
    }
}

/// A colour as QGIS writes it, `r,g,b,a`.
fn rgba(v: &Value) -> Option<String> {
    let (rgb, a) = colour(v)?;
    let h = &rgb[1..];
    let c = |i: usize| u8::from_str_radix(&h[i..i + 2], 16).unwrap_or(0);
    Some(format!(
        "{},{},{},{}",
        c(0),
        c(2),
        c(4),
        (a * 255.0).round() as u8
    ))
}

/// The `<Option type="Map">` of a symbol layer.
fn options(pairs: &[(&str, String)], pad: &str) -> String {
    let mut s = format!("{pad}<Option type=\"Map\">\n");
    for (k, v) in pairs {
        let _ = writeln!(
            s,
            "{pad}  <Option name=\"{k}\" type=\"QString\" value=\"{}\"/>",
            esc(v)
        );
    }
    let _ = writeln!(s, "{pad}</Option>");
    s
}

fn enabled_of(layer: &Value) -> &'static str {
    if layer.get("enabled") == Some(&Value::Bool(false)) {
        "0"
    } else {
        "1"
    }
}

fn layer_open(class: &str, layer: &Value, pad: &str) -> String {
    format!(
        "{pad}<layer class=\"{class}\" enabled=\"{}\" locked=\"0\" pass=\"0\">\n",
        enabled_of(layer)
    )
}

/// What of a layer's KentOS look no QGIS class here says.
fn layer_losses(layer: &Value, lost: &mut Vec<String>, what: &str) {
    if layer
        .get("opacity")
        .and_then(Value::as_f64)
        .is_some_and(|o| o < 1.0)
    {
        lost.push(format!(
            "{what}: katmanın saydamlığı QML'e yazılmadı (renklerin ve simgenin saydamlığı yazılır)"
        ));
    }
    if layer
        .get("effects")
        .and_then(Value::as_array)
        .is_some_and(|e| !e.is_empty())
    {
        lost.push(format!("{what}: katmanın çizim efektleri QML'e yazılmadı"));
    }
}

fn line_layer(layer: &Value, pad: &str, lost: &mut Vec<String>, what: &str) -> Option<String> {
    let unit = layer.get("unit").and_then(Value::as_str).unwrap_or("mm");
    let Some(color) = fixed(layer, "color", lost, what).and_then(rgba) else {
        lost.push(format!("{what}: çizginin rengi QML'in renklerinden değil"));
        return None;
    };
    let width = fixed(layer, "width", lost, what)
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let cap = match layer.get("cap").and_then(Value::as_str).unwrap_or("butt") {
        "round" => "round",
        "square" => "square",
        _ => "flat",
    };
    let join = match layer.get("join").and_then(Value::as_str).unwrap_or("miter") {
        "round" => "round",
        "bevel" => "bevel",
        _ => "miter",
    };
    let dash: Vec<String> = layer
        .get("dash")
        .and_then(Value::as_array)
        .map(|d| d.iter().filter_map(Value::as_f64).map(num).collect())
        .unwrap_or_default();
    let ring = match layer.get("rings").and_then(Value::as_str) {
        Some("exterior") => "1",
        Some("interior") => "2",
        _ => "0",
    };
    for (key, words) in [
        ("offset", "paralel kaydırması"),
        ("wave", "dalgası"),
        ("blur", "yumuşak kenarı"),
        ("shift", "gölge kaydırması"),
        ("dashOffset", "kesik başlangıcı"),
    ] {
        if layer
            .get(key)
            .is_some_and(|v| !v.is_null() && v != &json!(0))
        {
            lost.push(format!("{what}: çizginin {words} QML'e yazılmadı"));
        }
    }
    layer_losses(layer, lost, what);
    let u = unit_out(unit).to_owned();
    let mut s = layer_open("SimpleLine", layer, pad);
    s.push_str(&options(
        &[
            ("capstyle", cap.into()),
            ("customdash", dash.join(";")),
            ("customdash_unit", u.clone()),
            ("joinstyle", join.into()),
            ("line_color", color),
            ("line_style", "solid".into()),
            ("line_width", num(round3(width))),
            ("line_width_unit", u),
            ("offset", "0".into()),
            ("offset_unit", "MM".into()),
            ("ring_filter", ring.into()),
            (
                "use_custom_dash",
                if dash.is_empty() { "0" } else { "1" }.into(),
            ),
        ],
        &format!("{pad}  "),
    ));
    let _ = writeln!(s, "{pad}</layer>");
    Some(s)
}

fn fill_layer(layer: &Value, pad: &str, lost: &mut Vec<String>, what: &str) -> Option<String> {
    let Some(color) = fixed(layer, "color", lost, what).and_then(rgba) else {
        lost.push(format!("{what}: dolgunun rengi QML'in renklerinden değil"));
        return None;
    };
    layer_losses(layer, lost, what);
    // KentOS draws a fill's outline as its own line layer: QGIS's outline stays off.
    let mut s = layer_open("SimpleFill", layer, pad);
    s.push_str(&options(
        &[
            ("color", color),
            ("joinstyle", "bevel".into()),
            ("offset", "0,0".into()),
            ("offset_unit", "MM".into()),
            ("outline_color", "0,0,0,0".into()),
            ("outline_style", "no".into()),
            ("outline_width", "0".into()),
            ("outline_width_unit", "MM".into()),
            ("style", "solid".into()),
        ],
        &format!("{pad}  "),
    ));
    let _ = writeln!(s, "{pad}</layer>");
    Some(s)
}

fn marker_layer(layer: &Value, pad: &str, lost: &mut Vec<String>, what: &str) -> Option<String> {
    let t = layer.get("type").and_then(Value::as_str).unwrap_or("");
    if t != "shape" {
        lost.push(format!(
            "{what}: “{t}” işareti QML'e yazılmadı (yalnız şekil işaretleri yazılır)"
        ));
        return None;
    }
    let shape = layer
        .get("shape")
        .and_then(Value::as_str)
        .unwrap_or("circle");
    let name = match SHAPES.iter().find(|(k, _)| *k == shape) {
        Some(&(_, q)) => q,
        None if shape == "ring" => {
            lost.push(format!("{what}: halka işareti QGIS'te yok; daire yazıldı"));
            "circle"
        }
        None if shape == "rectangle" => {
            lost.push(format!(
                "{what}: dikdörtgen işaret QGIS'in basit işaretinde yok; kare yazıldı"
            ));
            "square"
        }
        None => {
            lost.push(format!(
                "{what}: “{shape}” şekli QGIS'in basit işaretinde yok; işaret yazılmadı"
            ));
            return None;
        }
    };
    let unit = layer.get("unit").and_then(Value::as_str).unwrap_or("mm");
    let size = fixed(layer, "size", lost, what)
        .and_then(Value::as_f64)
        .unwrap_or(2.0);
    let fill = fixed(layer, "fill", lost, what)
        .and_then(rgba)
        .unwrap_or_else(|| "0,0,0,0".into());
    let stroke = fixed(layer, "stroke", lost, what).and_then(rgba);
    let stroke_width = layer
        .get("strokeWidth")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    // KentOS turns counter-clockwise, QGIS clockwise.
    let angle = fixed(layer, "rotation", lost, what)
        .and_then(Value::as_f64)
        .map_or(0.0, |r| if r == 0.0 { 0.0 } else { -r });
    if layer
        .get("offset")
        .and_then(Value::as_array)
        .is_some_and(|o| o.iter().any(|v| v.as_f64().is_some_and(|x| x != 0.0)))
    {
        lost.push(format!("{what}: işaretin kaydırması QML'e yazılmadı"));
    }
    if layer
        .get("anchor")
        .and_then(Value::as_str)
        .is_some_and(|a| a != "center")
    {
        lost.push(format!("{what}: işaretin bağlantı noktası QML'e yazılmadı"));
    }
    if layer.get("height").is_some_and(|v| !v.is_null()) {
        lost.push(format!("{what}: işaretin yüksekliği QML'e yazılmadı"));
    }
    if shape != "ring"
        && layer
            .get("hole")
            .is_some_and(|v| v.as_f64().is_some_and(|h| h > 0.0))
    {
        lost.push(format!("{what}: işaretin deliği QML'e yazılmadı"));
    }
    layer_losses(layer, lost, what);
    let u = unit_out(unit).to_owned();
    let mut s = layer_open("SimpleMarker", layer, pad);
    s.push_str(&options(
        &[
            ("angle", num(round3(angle))),
            ("cap_style", "square".into()),
            ("color", fill),
            ("horizontal_anchor_point", "1".into()),
            ("joinstyle", "bevel".into()),
            ("name", name.into()),
            ("offset", "0,0".into()),
            ("offset_unit", "MM".into()),
            (
                "outline_color",
                stroke.clone().unwrap_or_else(|| "0,0,0,0".into()),
            ),
            (
                "outline_style",
                if stroke.is_some() { "solid" } else { "no" }.into(),
            ),
            ("outline_width", num(round3(stroke_width))),
            ("outline_width_unit", u.clone()),
            ("scale_method", "diameter".into()),
            ("size", num(round3(size))),
            ("size_unit", u),
            ("vertical_anchor_point", "1".into()),
        ],
        &format!("{pad}  "),
    ));
    let _ = writeln!(s, "{pad}</layer>");
    Some(s)
}

/// A symbol as QGIS's `<symbol>`, named `name`, of the class `key` (`fill`, `line`, `marker`); none when no layer
/// could be written.
fn symbol_xml(
    sym: &Value,
    key: &str,
    name: &str,
    pad: &str,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<String> {
    let layers = sym
        .get("layers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let inner = format!("{pad}  ");
    let mut body = String::new();
    for (i, layer) in layers.iter().enumerate() {
        let t = layer.get("type").and_then(Value::as_str).unwrap_or("");
        let written = match (key, t) {
            (_, "simpleLine") if key != "marker" => line_layer(layer, &inner, lost, what),
            ("fill", "simpleFill") => fill_layer(layer, &inner, lost, what),
            ("marker", _) => marker_layer(layer, &inner, lost, what),
            ("fill", "centroidMarker") => {
                let marker = layer.get("marker").cloned().unwrap_or(Value::Null);
                let at = format!("{inner}  ");
                match symbol_xml(&marker, "marker", &format!("@{name}@{i}"), &at, lost, what) {
                    Some(m) => {
                        layer_losses(layer, lost, what);
                        let centroid =
                            layer.get("position").and_then(Value::as_str) == Some("centroid");
                        let mut s = layer_open("CentroidFill", layer, &inner);
                        s.push_str(&options(
                            &[
                                ("clip_on_current_part_only", "0".into()),
                                ("clip_points", "0".into()),
                                ("point_on_all_parts", "1".into()),
                                ("point_on_surface", if centroid { "0" } else { "1" }.into()),
                            ],
                            &at,
                        ));
                        s.push_str(&m);
                        let _ = writeln!(s, "{inner}</layer>");
                        Some(s)
                    }
                    None => None,
                }
            }
            (_, other) => {
                let words = match other {
                    "hatchFill" => "tarama dolgusu",
                    "gradientFill" => "degrade dolgu",
                    "patternFill" => "desen dolgusu",
                    "imageFill" => "resim dolgusu",
                    "markerLine" => "işaretli çizgi",
                    "geometryGenerator" => "geometri üretici",
                    "simpleFill" => "dolgu (çizgi simgesinde)",
                    _ => "katman",
                };
                lost.push(format!("{what}: {words} QML'e yazılmadı"));
                None
            }
        };
        if let Some(w) = written {
            body.push_str(&w);
        }
    }
    if body.is_empty() {
        return None;
    }
    if sym
        .get("effects")
        .and_then(Value::as_array)
        .is_some_and(|e| !e.is_empty())
    {
        lost.push(format!("{what}: simgenin çizim efektleri QML'e yazılmadı"));
    }
    let alpha = sym
        .get("opacity")
        .and_then(Value::as_f64)
        .map_or(1.0, |o| o.clamp(0.0, 1.0));
    let mut s = format!(
        "{pad}<symbol alpha=\"{}\" clip_to_extent=\"1\" force_rhr=\"0\" name=\"{}\" type=\"{key}\">\n",
        num(round3(alpha)),
        esc(name)
    );
    s.push_str(&body);
    let _ = writeln!(s, "{pad}</symbol>");
    Some(s)
}

/// The symbol of a set for the QML's class, its library reference resolved.
fn set_symbol(
    set: Option<&Value>,
    key: &str,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<Value> {
    let sym = set?.get(key)?;
    match sym.get("ref").and_then(Value::as_str) {
        Some(id) => {
            let found = library(id);
            if found.is_none() {
                lost.push(format!(
                    "{what}: kitaplıkta olmayan “{id}” simgesi yazılmadı"
                ));
            }
            found
        }
        None => Some(sym.clone()),
    }
}

/// The symbols the writer gathers: each one's XML, numbered as QGIS numbers them.
struct Symbols {
    key: &'static str,
    xml: Vec<String>,
}

impl Symbols {
    /// Adds the set's symbol; its number, or none when nothing of it could be written.
    fn add(
        &mut self,
        set: Option<&Value>,
        library: &dyn Fn(&str) -> Option<Value>,
        lost: &mut Vec<String>,
        what: &str,
    ) -> Option<usize> {
        let sym = set_symbol(set, self.key, library, lost, what)?;
        let n = self.xml.len();
        let x = symbol_xml(&sym, self.key, &n.to_string(), "      ", lost, what)?;
        self.xml.push(x);
        Some(n)
    }
}

fn key_of(n: usize) -> String {
    format!("{{00000000-0000-0000-0000-{n:012}}}")
}

/// The QML's one class of symbols for the layer's geometry: areas, lines, points, in that order.
fn class_of(kinds: Kinds, lost: &mut Vec<String>) -> (&'static str, u8) {
    let all = [
        (kinds.area, "fill", 2, "alanların"),
        (kinds.line, "line", 1, "çizgilerin"),
        (kinds.point, "marker", 0, "noktaların"),
    ];
    let mut on = all.iter().filter(|k| k.0);
    let (_, key, geom, _) = *on.next().unwrap_or(&all[0]);
    let rest: Vec<&str> = on.map(|k| k.3).collect();
    if !rest.is_empty() && kinds != Kinds::ALL {
        lost.push(format!(
            "QML tek geometri türü içindir; {} simgeleri yazılmadı",
            rest.join(" ve ")
        ));
    }
    (key, geom)
}

fn rules_xml(
    rules: &[Value],
    symbols: &mut Symbols,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
    out: &mut String,
    pad: &str,
    count: &mut usize,
) {
    for r in rules {
        let label = r
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("Kural")
            .to_owned();
        let what = format!("“{label}” kuralı");
        let filter = if r.get("isElse") == Some(&Value::Bool(true)) {
            Some("ELSE".to_owned())
        } else {
            match r
                .get("filter")
                .and_then(Value::as_str)
                .filter(|s| !s.trim().is_empty())
            {
                Some(src) => match parse_filter(src) {
                    Some(f) => Some(qgis_text(&f)),
                    None => {
                        lost.push(format!(
                            "{what}: “{src}” ifadesi QGIS'in ifadesine çevrilemedi; kural ve alt kuralları yazılmadı"
                        ));
                        continue;
                    }
                },
                None => None,
            }
        };
        let symbol = symbols.add(r.get("symbols"), library, lost, &what);
        *count += 1;
        let mut attrs = format!(" key=\"{}\" label=\"{}\"", key_of(*count), esc(&label));
        if let Some(f) = &filter {
            let _ = write!(attrs, " filter=\"{}\"", esc(f));
        }
        if let Some(n) = symbol {
            let _ = write!(attrs, " symbol=\"{n}\"");
        }
        if let Some(s) = r.get("minScale").and_then(Value::as_f64) {
            let _ = write!(attrs, " scalemindenom=\"{}\"", num(s));
        }
        if let Some(s) = r.get("maxScale").and_then(Value::as_f64) {
            let _ = write!(attrs, " scalemaxdenom=\"{}\"", num(s));
        }
        if r.get("enabled") == Some(&Value::Bool(false)) {
            attrs.push_str(" checkstate=\"0\"");
        }
        let children = r
            .get("children")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if children.is_empty() {
            let _ = writeln!(out, "{pad}<rule{attrs}/>");
        } else {
            let _ = writeln!(out, "{pad}<rule{attrs}>");
            rules_xml(
                &children,
                symbols,
                library,
                lost,
                out,
                &format!("{pad}  "),
                count,
            );
            let _ = writeln!(out, "{pad}</rule>");
        }
    }
}

/// The QML of a layer drawn by `renderer` (the app's JSON) with its objects' `kinds`; library symbols by id through
/// `library`.
pub fn write(renderer: &Value, kinds: Kinds, library: &dyn Fn(&str) -> Option<Value>) -> Written {
    let mut lost = Vec::new();
    let (key, geom) = class_of(kinds, &mut lost);
    let mut symbols = Symbols {
        key,
        xml: Vec::new(),
    };
    let mut body = String::new();
    let mut rules = 0usize;
    let kind = renderer.get("type").and_then(Value::as_str).unwrap_or("");
    let head = match kind {
        "single" => {
            if symbols
                .add(renderer.get("symbols"), library, &mut lost, "Simge")
                .is_some()
            {
                rules = 1;
            }
            "<renderer-v2 type=\"singleSymbol\" symbollevels=\"0\" forceraster=\"0\" enableorderby=\"0\" referencescale=\"-1\">".to_owned()
        }
        "categorized" => {
            let expr = renderer.get("expr").and_then(Value::as_str).unwrap_or("");
            let attr = match field_of(expr) {
                Some(f) => f,
                None => {
                    lost.push(format!(
                        "Kategorili görünüşün “{expr}” ifadesi bir alan değil; QGIS'in ifadesine çevrilemedi, sınıflar yazılmadı"
                    ));
                    String::new()
                }
            };
            body.push_str("    <categories>\n");
            if !attr.is_empty() {
                let cats = renderer
                    .get("categories")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for c in &cats {
                    let value = c.get("value").and_then(Value::as_str).unwrap_or("");
                    let label = c
                        .get("label")
                        .and_then(Value::as_str)
                        .filter(|l| !l.is_empty())
                        .unwrap_or(value);
                    let Some(n) = symbols.add(
                        c.get("symbols"),
                        library,
                        &mut lost,
                        &format!("“{label}” sınıfı"),
                    ) else {
                        continue;
                    };
                    rules += 1;
                    let render = c.get("enabled") != Some(&Value::Bool(false));
                    let _ = writeln!(
                        body,
                        "      <category label=\"{}\" render=\"{render}\" symbol=\"{n}\" type=\"string\" uuid=\"{}\" value=\"{}\"/>",
                        esc(label),
                        key_of(rules),
                        esc(value)
                    );
                }
                if let Some(n) = symbols.add(
                    renderer.get("other"),
                    library,
                    &mut lost,
                    "“Diğerleri” sınıfı",
                ) {
                    rules += 1;
                    let _ = writeln!(
                        body,
                        "      <category label=\"Diğerleri\" render=\"true\" symbol=\"{n}\" type=\"NULL\" uuid=\"{}\" value=\"NULL\"/>",
                        key_of(rules)
                    );
                }
            }
            body.push_str("    </categories>\n");
            format!(
                "<renderer-v2 attr=\"{}\" type=\"categorizedSymbol\" symbollevels=\"0\" forceraster=\"0\" enableorderby=\"0\" referencescale=\"-1\">",
                esc(&attr)
            )
        }
        "graduated" => {
            let expr = renderer.get("expr").and_then(Value::as_str).unwrap_or("");
            let attr = match field_of(expr) {
                Some(f) => f,
                None => {
                    lost.push(format!(
                        "Aralıklı görünüşün “{expr}” ifadesi bir alan değil; QGIS'in ifadesine çevrilemedi, sınıflar yazılmadı"
                    ));
                    String::new()
                }
            };
            body.push_str("    <ranges>\n");
            if !attr.is_empty() {
                let classes = renderer
                    .get("classes")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                for c in &classes {
                    let (Some(lo), Some(hi)) = (
                        c.get("min").and_then(Value::as_f64),
                        c.get("max").and_then(Value::as_f64),
                    ) else {
                        continue;
                    };
                    let label = c
                        .get("label")
                        .and_then(Value::as_str)
                        .map_or_else(|| format!("{} - {}", num(lo), num(hi)), str::to_owned);
                    let Some(n) = symbols.add(
                        c.get("symbols"),
                        library,
                        &mut lost,
                        &format!("“{label}” sınıfı"),
                    ) else {
                        continue;
                    };
                    rules += 1;
                    let _ = writeln!(
                        body,
                        "      <range label=\"{}\" lower=\"{}\" render=\"true\" symbol=\"{n}\" upper=\"{}\" uuid=\"{}\"/>",
                        esc(&label),
                        num(lo),
                        num(hi),
                        key_of(rules)
                    );
                }
                if rules > 1 {
                    lost.push(
                        "QGIS'in aralıkları alt sınırı dışarıda bırakır (alt < değer ≤ üst, ilki hariç); sınıf sınırındaki değer bir alttaki sınıfa düşer".into(),
                    );
                }
            }
            body.push_str("    </ranges>\n");
            format!(
                "<renderer-v2 attr=\"{}\" type=\"graduatedSymbol\" graduatedMethod=\"GraduatedColor\" symbollevels=\"0\" forceraster=\"0\" enableorderby=\"0\" referencescale=\"-1\">",
                esc(&attr)
            )
        }
        "rules" => {
            let list = renderer
                .get("rules")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_default();
            let mut inner = String::new();
            let mut count = 0usize;
            rules_xml(
                &list,
                &mut symbols,
                library,
                &mut lost,
                &mut inner,
                "      ",
                &mut count,
            );
            rules = count;
            let _ = writeln!(body, "    <rules key=\"{}\">", key_of(0));
            body.push_str(&inner);
            body.push_str("    </rules>\n");
            "<renderer-v2 type=\"RuleRenderer\" symbollevels=\"0\" forceraster=\"0\" enableorderby=\"0\" referencescale=\"-1\">".to_owned()
        }
        other => {
            lost.push(format!(
                "“{other}” görünüşü QML'e yazılmadı (yalnız tek simge, kategorili, aralıklı ve kurallı görünüş yazılır)"
            ));
            String::new()
        }
    };
    if symbols.xml.is_empty() {
        rules = 0;
    }
    let mut xml = String::new();
    xml.push_str("<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>\n");
    xml.push_str("<qgis version=\"4.2.2-KentOS\" styleCategories=\"Symbology\">\n");
    if !head.is_empty() {
        let _ = writeln!(xml, "  {head}");
        xml.push_str(&body);
        xml.push_str("    <symbols>\n");
        for s in &symbols.xml {
            xml.push_str(s);
        }
        xml.push_str("    </symbols>\n");
        xml.push_str("  </renderer-v2>\n");
    }
    let _ = writeln!(xml, "  <layerGeometryType>{geom}</layerGeometryType>");
    xml.push_str("</qgis>\n");
    Written { xml, lost, rules }
}

// ─── Reading ──────────────────────────────────────────────────────────────────────────────────────────

type Node<'a, 'i> = roxmltree::Node<'a, 'i>;

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

/// A symbol layer's options: QGIS 3's `<Option type="Map">` or QGIS 2's `<prop k v>`.
fn props(layer: Node<'_, '_>) -> Map<String, Value> {
    let mut m = Map::new();
    if let Some(map) = child(layer, "Option") {
        for o in map.children().filter(|c| c.is_element()) {
            if let (Some(k), Some(v)) = (o.attribute("name"), o.attribute("value")) {
                m.insert(k.into(), json!(v));
            }
        }
    }
    for p in layer
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == "prop")
    {
        if let (Some(k), Some(v)) = (p.attribute("k"), p.attribute("v")) {
            m.insert(k.into(), json!(v));
        }
    }
    m
}

fn prop<'m>(m: &'m Map<String, Value>, k: &str) -> Option<&'m str> {
    m.get(k).and_then(Value::as_str)
}

fn prop_num(m: &Map<String, Value>, k: &str) -> Option<f64> {
    prop(m, k).and_then(|v| v.trim().parse::<f64>().ok())
}

/// A QGIS colour (`r,g,b,a[,rgb:…]`, or `#rrggbb`) as KentOS's; none when it is fully transparent.
fn colour_in(text: Option<&str>) -> Option<String> {
    let t = text?.trim();
    if t.starts_with('#') {
        return colour(&json!(t)).map(|(rgb, a)| colour_with(&rgb, Some(a)));
    }
    let p: Vec<u8> = t
        .split(',')
        .take(4)
        .filter_map(|x| x.trim().parse::<u8>().ok())
        .collect();
    if p.len() < 3 {
        return None;
    }
    let a = p.get(3).copied().unwrap_or(255);
    if a == 0 {
        return None;
    }
    let rgb = format!("#{:02x}{:02x}{:02x}", p[0], p[1], p[2]);
    Some(colour_with(&rgb, Some(f64::from(a) / 255.0)))
}

/// A size in QGIS's unit as KentOS's: the value and its unit, or none (a percentage).
fn size_in(v: f64, unit: Option<&str>) -> Option<(f64, &'static str)> {
    Some(match unit.unwrap_or("MM") {
        "MM" => (v, "mm"),
        "Point" => (v * POINT_MM, "mm"),
        "Inch" => (v * 25.4, "mm"),
        "Pixel" => (v, "px"),
        // The map's unit: metres in the projected systems KentOS's projects have.
        "MapUnit" | "RenderMetersInMapUnits" => (v, "m"),
        _ => return None,
    })
}

/// A size in `unit` as `to` (the layer's own unit): the paper units turn into each other, the others only stay.
fn size_as(v: f64, unit: Option<&str>, to: &str) -> Option<f64> {
    let (v, u) = size_in(v, unit)?;
    (u == to).then_some(v)
}

/// What a symbol layer has that the reader leaves.
fn data_defined(layer: Node<'_, '_>, lost: &mut Vec<String>, what: &str) {
    let dd = layer
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == "data_defined_properties")
        .flat_map(|d| d.descendants())
        .any(|o| {
            o.attribute("name") == Some("active")
                && o.attribute("value").is_some_and(|v| v == "true")
        });
    if dd {
        lost.push(format!(
            "{what}: veriye bağlı değerler (data defined) alınmadı; sabit değerleri alındı"
        ));
    }
}

fn read_line(
    m: &Map<String, Value>,
    id: String,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<Value> {
    let style = prop(m, "line_style").unwrap_or("solid");
    if style == "no" {
        return None;
    }
    let color = colour_in(prop(m, "line_color").or(prop(m, "color")))?;
    let (width, unit) = size_in(
        prop_num(m, "line_width").unwrap_or(0.26),
        prop(m, "line_width_unit"),
    )
    .unwrap_or_else(|| {
        lost.push(format!(
            "{what}: çizginin yüzde boyu KentOS'ta yok; 0,26 mm alındı"
        ));
        (0.26, "mm")
    });
    let mut l =
        json!({ "id": id, "type": "simpleLine", "color": color, "width": width, "unit": unit });
    let custom = prop(m, "use_custom_dash") == Some("1");
    let dash: Option<Vec<f64>> = if custom {
        let unit_of = prop(m, "customdash_unit");
        let d: Vec<f64> = prop(m, "customdash")
            .unwrap_or("")
            .split(';')
            .filter_map(|p| p.trim().parse::<f64>().ok())
            .filter_map(|v| size_as(v, unit_of, unit))
            .collect();
        if d.is_empty() && !prop(m, "customdash").unwrap_or("").is_empty() {
            lost.push(format!(
                "{what}: kesiklerin birimi çizginin biriminden başka; kesikler alınmadı"
            ));
        }
        (!d.is_empty()).then_some(d)
    } else {
        // Qt's patterns, in widths of the pen (a hairline is one).
        let w = if width > 0.0 { width } else { 1.0 };
        let pattern: &[f64] = match style {
            "dash" => &[4.0, 2.0],
            "dot" => &[1.0, 2.0],
            "dash dot" => &[4.0, 2.0, 1.0, 2.0],
            "dash dot dot" => &[4.0, 2.0, 1.0, 2.0, 1.0, 2.0],
            _ => &[],
        };
        (!pattern.is_empty()).then(|| pattern.iter().map(|p| round3(p * w)).collect())
    };
    if let Some(d) = dash {
        l["dash"] = json!(d);
    }
    l["cap"] = json!(match prop(m, "capstyle").unwrap_or("square") {
        "round" => "round",
        "flat" => "butt",
        _ => "square",
    });
    l["join"] = json!(match prop(m, "joinstyle").unwrap_or("bevel") {
        "round" => "round",
        "miter" => "miter",
        _ => "bevel",
    });
    match prop(m, "ring_filter") {
        Some("1") => l["rings"] = json!("exterior"),
        Some("2") => l["rings"] = json!("interior"),
        _ => {}
    }
    if prop_num(m, "offset").is_some_and(|o| o != 0.0) {
        lost.push(format!("{what}: çizginin paralel kaydırması alınmadı"));
    }
    if prop_num(m, "trim_distance_start").is_some_and(|o| o != 0.0)
        || prop_num(m, "trim_distance_end").is_some_and(|o| o != 0.0)
    {
        lost.push(format!("{what}: çizginin uçlardan kırpması alınmadı"));
    }
    Some(l)
}

fn read_marker(m: &Map<String, Value>, lost: &mut Vec<String>, what: &str) -> Value {
    let name = prop(m, "name").unwrap_or("circle");
    let shape = SHAPES
        .iter()
        .find(|(_, q)| *q == name)
        .map(|(k, _)| *k)
        .or_else(|| SHAPES_IN.iter().find(|(q, _)| *q == name).map(|(_, k)| *k))
        .unwrap_or_else(|| {
            lost.push(format!(
                "{what}: “{name}” işareti KentOS'ta yok; kare alındı"
            ));
            "square"
        });
    let (size, unit) = size_in(prop_num(m, "size").unwrap_or(2.0), prop(m, "size_unit"))
        .unwrap_or_else(|| {
            lost.push(format!(
                "{what}: işaretin yüzde boyu KentOS'ta yok; 2 mm alındı"
            ));
            (2.0, "mm")
        });
    let fill = colour_in(prop(m, "color"));
    let mut v = json!({ "id": "p", "type": "shape", "shape": shape, "size": size, "unit": unit, "fill": fill });
    if prop(m, "outline_style") != Some("no")
        && let Some(stroke) = colour_in(prop(m, "outline_color"))
    {
        v["stroke"] = json!(stroke);
        let w = prop_num(m, "outline_width").unwrap_or(0.0);
        match size_as(w, prop(m, "outline_width_unit"), unit) {
            Some(w) => v["strokeWidth"] = json!(w),
            None => lost.push(format!(
                "{what}: işaretin çizgi kalınlığının birimi boyununkinden başka; kalınlık alınmadı"
            )),
        }
    }
    if let Some(a) = prop_num(m, "angle").filter(|a| *a != 0.0) {
        // QGIS turns clockwise, KentOS counter-clockwise.
        v["rotation"] = json!(-a);
    }
    if prop(m, "offset").is_some_and(|o| {
        o.split(',')
            .any(|x| x.trim().parse::<f64>().is_ok_and(|x| x != 0.0))
    }) {
        lost.push(format!("{what}: işaretin kaydırması alınmadı"));
    }
    if prop(m, "horizontal_anchor_point").is_some_and(|a| a != "1")
        || prop(m, "vertical_anchor_point").is_some_and(|a| a != "1")
    {
        lost.push(format!("{what}: işaretin bağlantı noktası alınmadı"));
    }
    if prop(m, "scale_method") == Some("area") {
        lost.push(format!("{what}: işaretin alana göre ölçeklenmesi alınmadı"));
    }
    v
}

/// A QGIS `<symbol>` as KentOS's, and its class key.
fn read_symbol(s: Node<'_, '_>, lost: &mut Vec<String>, what: &str) -> Option<(String, Value)> {
    let t = s.attribute("type").unwrap_or("");
    let key = match t {
        "fill" | "line" | "marker" => t.to_owned(),
        other => {
            lost.push(format!("{what}: “{other}” simgesi alınmadı"));
            return None;
        }
    };
    let mut layers = Vec::new();
    for (i, layer) in s
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == "layer")
        .enumerate()
    {
        let class = layer.attribute("class").unwrap_or("");
        let m = props(layer);
        data_defined(layer, lost, what);
        let mut taken: Vec<Value> = Vec::new();
        match (key.as_str(), class) {
            ("fill", "SimpleFill") => {
                if prop(&m, "style").unwrap_or("solid") == "solid" {
                    if let Some(c) = colour_in(prop(&m, "color")) {
                        taken.push(
                            json!({ "id": format!("f{i}"), "type": "simpleFill", "color": c }),
                        );
                    }
                } else if prop(&m, "style") != Some("no") {
                    lost.push(format!(
                        "{what}: “{}” fırça deseni KentOS'un dolgusuna çevrilmedi",
                        prop(&m, "style").unwrap_or("")
                    ));
                }
                if prop(&m, "outline_style").unwrap_or("solid") != "no" {
                    let line: Map<String, Value> = [
                        ("line_color", m.get("outline_color")),
                        ("line_width", m.get("outline_width")),
                        ("line_width_unit", m.get("outline_width_unit")),
                        ("line_style", m.get("outline_style")),
                        ("joinstyle", m.get("joinstyle")),
                    ]
                    .into_iter()
                    .filter_map(|(k, v)| v.map(|v| (k.to_owned(), v.clone())))
                    .collect();
                    if let Some(l) = read_line(&line, format!("o{i}"), lost, what) {
                        taken.push(l);
                    }
                }
                if prop(&m, "offset").is_some_and(|o| {
                    o.split(',')
                        .any(|x| x.trim().parse::<f64>().is_ok_and(|x| x != 0.0))
                }) {
                    lost.push(format!("{what}: dolgunun kaydırması alınmadı"));
                }
            }
            ("fill" | "line", "SimpleLine") => {
                if let Some(l) = read_line(&m, format!("l{i}"), lost, what) {
                    taken.push(l);
                }
            }
            ("marker", "SimpleMarker") => taken.push(read_marker(&m, lost, what)),
            ("fill", "CentroidFill") => {
                match child(layer, "symbol").and_then(|sub| read_symbol(sub, lost, what)) {
                    Some((k, sym)) if k == "marker" => {
                        let mut c = json!({ "id": format!("c{i}"), "type": "centroidMarker", "marker": sym });
                        if prop(&m, "point_on_surface") == Some("0") {
                            c["position"] = json!("centroid");
                        }
                        taken.push(c);
                    }
                    _ => lost.push(format!("{what}: merkez işaretinin simgesi alınmadı")),
                }
            }
            (_, other) => lost.push(format!(
                "{what}: QGIS'in “{other}” simge katmanı KentOS'un katmanına çevrilmedi"
            )),
        }
        if layer.attribute("enabled") == Some("0") {
            for l in &mut taken {
                l["enabled"] = json!(false);
            }
        }
        layers.extend(taken);
    }
    if layers.is_empty() {
        return None;
    }
    let mut v = json!({ "type": key, "layers": layers });
    if let Some(a) = s
        .attribute("alpha")
        .and_then(|a| a.parse::<f64>().ok())
        .filter(|a| *a < 1.0)
    {
        v["opacity"] = json!(a);
    }
    Some((key, v))
}

/// The `<symbols>` of a renderer by their names.
fn symbols_of(
    renderer: Node<'_, '_>,
    lost: &mut Vec<String>,
) -> std::collections::HashMap<String, (String, Value)> {
    let mut out = std::collections::HashMap::new();
    if let Some(list) = child(renderer, "symbols") {
        for s in list
            .children()
            .filter(|c| c.is_element() && c.tag_name().name() == "symbol")
        {
            let name = s.attribute("name").unwrap_or("").to_owned();
            if let Some(sym) = read_symbol(s, lost, &format!("“{name}” simgesi")) {
                out.insert(name, sym);
            }
        }
    }
    out
}

/// A symbol as a set of KentOS: under its class's key.
fn set_of(sym: Option<&(String, Value)>) -> Value {
    match sym {
        Some((k, v)) => json!({ k.clone(): v }),
        None => json!({}),
    }
}

fn read_rules(
    parent: Node<'_, '_>,
    symbols: &std::collections::HashMap<String, (String, Value)>,
    lost: &mut Vec<String>,
    count: &mut usize,
) -> Vec<Value> {
    let mut out = Vec::new();
    for r in parent
        .children()
        .filter(|c| c.is_element() && c.tag_name().name() == "rule")
    {
        let label = r
            .attribute("label")
            .filter(|l| !l.is_empty())
            .map_or_else(|| format!("Kural {}", *count + 1), str::to_owned);
        let mut v = json!({ "label": label });
        match r
            .attribute("filter")
            .map(str::trim)
            .filter(|f| !f.is_empty())
        {
            Some(f) if f.eq_ignore_ascii_case("ELSE") => v["isElse"] = json!(true),
            Some(f) => match parse_qgis(f) {
                Some(filter) => v["filter"] = json!(filter_text(&filter)),
                None => {
                    lost.push(format!(
                        "“{label}” kuralının “{f}” süzgeci KentOS'un ifadesine çevrilemedi; kural ve alt kuralları alınmadı"
                    ));
                    continue;
                }
            },
            None => {}
        }
        *count += 1;
        v["id"] = json!(format!("r{count}"));
        if let Some(n) = r.attribute("symbol") {
            v["symbols"] = set_of(symbols.get(n));
        }
        for (attr, key) in [("scalemindenom", "minScale"), ("scalemaxdenom", "maxScale")] {
            if let Some(s) = r
                .attribute(attr)
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|s| *s > 0.0)
            {
                v[key] = json!(s);
            }
        }
        if r.attribute("checkstate") == Some("0") {
            v["enabled"] = json!(false);
        }
        let children = read_rules(r, symbols, lost, count);
        if !children.is_empty() {
            v["children"] = json!(children);
        }
        out.push(v);
    }
    out
}

/// KentOS's renderer of a QML: its `renderer-v2` as the renderer of the same kind.
pub fn read(text: &str) -> Result<Read, String> {
    let doc =
        crate::xml::document_with_dtd(text).map_err(|_| "Dosya okunabilir bir XML değil.".to_owned())?;
    let root = doc.root_element();
    if root.tag_name().name() != "qgis" {
        return Err("Dosya bir QGIS stil dosyası (QML) değil: kök öğe <qgis> değil.".into());
    }
    let Some(r) = child(root, "renderer-v2") else {
        return Err("QML'de QGIS'in görünüşü (renderer-v2) yok.".into());
    };
    let mut lost = Vec::new();
    if child(root, "labeling").is_some() || root.attribute("labelsEnabled") == Some("1") {
        lost.push("etiketler alınmadı; KentOS'ta etiketler ayrıdır".into());
    }
    if child(root, "layerOpacity")
        .and_then(|o| o.text())
        .and_then(|t| t.trim().parse::<f64>().ok())
        .is_some_and(|o| o < 1.0)
    {
        lost.push("katmanın saydamlığı alınmadı (Katman çizimi ayrıdır)".into());
    }
    let symbols = symbols_of(r, &mut lost);
    let kind = r.attribute("type").unwrap_or("");
    let attr_field = |lost: &mut Vec<String>| -> Option<String> {
        let a = r.attribute("attr").unwrap_or("").trim();
        let bare = a
            .strip_prefix('"')
            .and_then(|x| x.strip_suffix('"'))
            .map_or_else(|| a.to_owned(), |x| x.replace("\"\"", "\""));
        let plain = !bare.is_empty()
            && (a.starts_with('"') || bare.chars().all(|c| c.is_alphanumeric() || c == '_'));
        if plain {
            Some(bare)
        } else {
            lost.push(format!(
                "görünüşün “{a}” ifadesi bir alan değil; KentOS'un ifadesine çevrilemedi"
            ));
            None
        }
    };
    let (renderer, rules) = match kind {
        "singleSymbol" => {
            let sym = child(r, "symbols")
                .and_then(|s| s.children().find(|c| c.is_element()))
                .and_then(|s| s.attribute("name"))
                .and_then(|n| symbols.get(n));
            if sym.is_none() {
                return Err("QML'in simgesi KentOS'un çizebileceği bir simge değil.".into());
            }
            (json!({ "type": "single", "symbols": set_of(sym) }), 1)
        }
        "categorizedSymbol" => {
            let Some(field) = attr_field(&mut lost) else {
                return Err("Kategorili görünüşün alanı okunamadı.".into());
            };
            let mut cats = Vec::new();
            let mut other = None;
            let mut n = 0;
            for c in child(r, "categories")
                .into_iter()
                .flat_map(|c| c.children())
                .filter(|c| c.is_element() && c.tag_name().name() == "category")
            {
                let label = c.attribute("label").unwrap_or("");
                let set = set_of(c.attribute("symbol").and_then(|s| symbols.get(s)));
                let enabled = c.attribute("render") != Some("false");
                let vals: Vec<String> = c
                    .children()
                    .filter(|v| v.is_element() && v.tag_name().name() == "val")
                    .filter_map(|v| v.attribute("value").map(str::to_owned))
                    .collect();
                let null = c.attribute("type") == Some("NULL")
                    || (vals.is_empty() && c.attribute("value").is_none_or(str::is_empty));
                n += 1;
                if null && vals.is_empty() {
                    if other.is_some() {
                        lost.push(format!("ikinci “Diğerleri” sınıfı (“{label}”) alınmadı"));
                    } else {
                        other = Some(set);
                    }
                    continue;
                }
                let values = if vals.is_empty() {
                    vec![c.attribute("value").unwrap_or("").to_owned()]
                } else {
                    vals
                };
                for v in values {
                    let mut cat = json!({ "value": v, "label": if label.is_empty() { v.as_str() } else { label }, "symbols": set });
                    if !enabled {
                        cat["enabled"] = json!(false);
                    }
                    cats.push(cat);
                }
            }
            let mut v =
                json!({ "type": "categorized", "expr": format!("[{field}]"), "categories": cats });
            if let Some(o) = other {
                v["other"] = o;
            }
            (v, n)
        }
        "graduatedSymbol" => {
            let Some(field) = attr_field(&mut lost) else {
                return Err("Aralıklı görünüşün alanı okunamadı.".into());
            };
            let mut classes = Vec::new();
            for c in child(r, "ranges")
                .into_iter()
                .flat_map(|c| c.children())
                .filter(|c| c.is_element() && c.tag_name().name() == "range")
            {
                let (Some(lo), Some(hi)) = (
                    c.attribute("lower").and_then(|x| x.parse::<f64>().ok()),
                    c.attribute("upper").and_then(|x| x.parse::<f64>().ok()),
                ) else {
                    continue;
                };
                let cl = json!({
                    "min": lo, "max": hi,
                    "label": c.attribute("label").unwrap_or(""),
                    "symbols": set_of(c.attribute("symbol").and_then(|s| symbols.get(s))),
                });
                if c.attribute("render") == Some("false") {
                    lost.push(format!(
                        "kapalı “{}” sınıfı açık alındı (KentOS'un aralıklarında kapatma yok)",
                        c.attribute("label").unwrap_or("")
                    ));
                }
                classes.push(cl);
            }
            if classes.len() > 1 {
                lost.push("QGIS'in aralıkları alt sınırı dışarıda bırakır (alt < değer ≤ üst, ilki hariç), KentOS'unkiler üst sınırı (alt ≤ değer < üst, sonuncu hariç); sınıf sınırındaki değer bir üstteki sınıfa geçer".into());
            }
            if child(r, "sizescale").is_some_and(|s| s.attribute("scalemethod").is_some())
                || r.attribute("graduatedMethod") == Some("GraduatedSize")
            {
                lost.push("boya göre aralıklı görünüş renge göre alındı".into());
            }
            let n = classes.len();
            (
                json!({ "type": "graduated", "expr": format!("[{field}]"), "classes": classes }),
                n,
            )
        }
        "RuleRenderer" => {
            let mut count = 0;
            let list = child(r, "rules")
                .map(|top| read_rules(top, &symbols, &mut lost, &mut count))
                .unwrap_or_default();
            (json!({ "type": "rules", "rules": list }), count)
        }
        other => {
            return Err(format!(
                "QGIS'in “{other}” görünüşü KentOS'a çevrilmedi (tek simge, kategorili, aralıklı ve kurallı görünüş alınır)."
            ));
        }
    };
    if rules == 0 {
        return Err("QML'de KentOS'un çizebileceği bir sınıf ya da kural yok.".into());
    }
    Ok(Read {
        name: None,
        renderer,
        lost,
        rules,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<Value> {
        None
    }

    fn area() -> Kinds {
        Kinds {
            point: false,
            line: false,
            area: true,
        }
    }

    #[test]
    fn a_categorized_look_goes_out_and_comes_back() {
        let r = json!({ "type": "categorized", "expr": "[kullanim]", "categories": [
            { "value": "Konut", "label": "Konut", "symbols": { "fill": { "type": "fill", "opacity": 0.5, "layers": [
                { "id": "f", "type": "simpleFill", "color": "#e31a1c" },
                { "id": "o", "type": "simpleLine", "color": "#232323", "width": 0.5, "unit": "mm", "cap": "butt", "join": "miter" },
            ] } } },
        ], "other": { "fill": { "type": "fill", "layers": [{ "id": "f", "type": "simpleFill", "color": "#33a02c80" }] } } });
        let w = write(&r, area(), &none);
        assert_eq!(w.rules, 2, "{:?}", w.lost);
        assert!(w.lost.is_empty(), "{:?}", w.lost);
        let back = read(&w.xml).expect("reads");
        assert_eq!(back.renderer["type"], "categorized");
        assert_eq!(back.renderer["expr"], "[kullanim]");
        let fill = &back.renderer["categories"][0]["symbols"]["fill"];
        assert_eq!(fill["opacity"], 0.5);
        assert_eq!(fill["layers"][0]["color"], "#e31a1c");
        assert_eq!(fill["layers"][1]["width"], 0.5);
        assert_eq!(fill["layers"][1]["cap"], "butt");
        assert_eq!(
            back.renderer["other"]["fill"]["layers"][0]["color"],
            "#33a02c80"
        );
    }

    #[test]
    fn rule_filters_turn_between_the_two_languages() {
        let f = parse_filter("[kat] > 3 ve [ad] gibi 'A%' veya [tur] boş değil").expect("parses");
        let q = qgis_text(&f);
        assert_eq!(
            q,
            "(\"kat\" > 3 AND \"ad\" LIKE 'A%') OR \"tur\" IS NOT NULL"
        );
        let back = parse_qgis(&q).expect("reads");
        assert_eq!(back, f);
        let list = parse_qgis("\"k\" IN (1, 'b')").expect("in");
        assert_eq!(filter_text(&list), "[k] = 1 veya [k] = 'b'");
        assert!(parse_qgis("upper(\"k\") = 'A'").is_none());
    }

    #[test]
    fn a_marker_turns_the_other_way_and_keeps_its_unit() {
        let r = json!({ "type": "single", "symbols": { "marker": { "type": "marker", "layers": [
            { "id": "p", "type": "shape", "shape": "x", "size": 6, "unit": "px", "fill": null, "stroke": "#000000", "strokeWidth": 1, "rotation": 30 },
        ] } } });
        let k = Kinds {
            point: true,
            line: false,
            area: false,
        };
        let w = write(&r, k, &none);
        assert!(w.xml.contains("value=\"cross2\""));
        assert!(
            w.xml
                .contains("name=\"angle\" type=\"QString\" value=\"-30\"")
        );
        let back = read(&w.xml).expect("reads");
        let m = &back.renderer["symbols"]["marker"]["layers"][0];
        assert_eq!(m["shape"], "x");
        assert_eq!(m["unit"], "px");
        assert_eq!(m["rotation"], 30.0);
        assert_eq!(m["fill"], Value::Null);
    }

    #[test]
    fn what_a_qml_cannot_say_is_named() {
        let r = json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [
            { "id": "h", "type": "hatchFill", "angle": 45, "spacing": 2, "width": 0.2, "color": "#000000" },
            { "id": "f", "type": "simpleFill", "color": "#ffffff" },
        ] } } });
        let w = write(&r, Kinds::ALL, &none);
        assert_eq!(w.rules, 1);
        assert!(
            w.lost.iter().any(|l| l.contains("tarama dolgusu")),
            "{:?}",
            w.lost
        );
    }
}
