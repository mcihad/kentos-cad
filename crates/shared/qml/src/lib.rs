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

use kentos_formats::sld::{
    self, Filter, Lit, Op, colour, colour_with, esc, field_of, filter_text, num,
    parse_filter, parse_qgis, round3,
};
pub use kentos_formats::sld::{Kinds, Read, StyleAsset, Written};

mod labels;
mod thematic;

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

// ─── Data-defined values ───────────────────────────────────────────────────────────────────────────────

/// A value expression's tokens: fields, numbers, `+ - * / %` and parentheses; anything else (a text, a function,
/// a comparison) is not said across the two languages.
#[derive(Clone, Debug, PartialEq)]
enum VTok {
    Field(String),
    Num(String),
    Op(char),
    Open,
    Close,
}

/// The tokens of `src`, KentOS's (`[Ad]` or a bare word a field) or QGIS's (`"Ad"` or a bare word a field).
fn value_tokens(src: &str, qgis: bool) -> Option<Vec<VTok>> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if !qgis && ch == '[' {
            let j = c[i + 1..].iter().position(|&x| x == ']')? + i + 1;
            let name: String = c[i + 1..j].iter().collect::<String>().trim().to_owned();
            if name.is_empty() {
                return None;
            }
            out.push(VTok::Field(name));
            i = j + 1;
        } else if qgis && ch == '"' {
            let mut name = String::new();
            let mut j = i + 1;
            loop {
                let x = *c.get(j)?;
                if x == '"' {
                    if c.get(j + 1) == Some(&'"') {
                        name.push('"');
                        j += 2;
                        continue;
                    }
                    break;
                }
                name.push(x);
                j += 1;
            }
            out.push(VTok::Field(name));
            i = j + 1;
        } else if ch.is_ascii_digit()
            || (ch == '.' && c.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let mut j = i + 1;
            while j < c.len() && (c[j].is_ascii_digit() || c[j] == '.') {
                j += 1;
            }
            let text: String = c[i..j].iter().collect();
            text.parse::<f64>().ok()?;
            out.push(VTok::Num(text));
            i = j;
        } else if "+-*/%".contains(ch) {
            out.push(VTok::Op(ch));
            i += 1;
        } else if ch == '(' {
            out.push(VTok::Open);
            i += 1;
        } else if ch == ')' {
            out.push(VTok::Close);
            i += 1;
        } else if ch.is_alphabetic() || ch == '_' {
            let mut j = i + 1;
            while j < c.len() && (c[j].is_alphanumeric() || c[j] == '_') {
                j += 1;
            }
            let w: String = c[i..j].iter().collect();
            // A word before a parenthesis is a function; a keyword is no field.
            if c[j..].iter().find(|x| !x.is_whitespace()) == Some(&'(') || !sld::bare_field(&w) {
                return None;
            }
            out.push(VTok::Field(w));
            i = j;
        } else {
            return None;
        }
    }
    (!out.is_empty()).then_some(out)
}

/// Tokens as text: fields by `field`, a space around binary operators, none inside parentheses or after a sign.
fn value_text(toks: &[VTok], field: &dyn Fn(&str) -> String) -> String {
    let mut s = String::new();
    let mut prev: Option<&VTok> = None;
    let mut sign = false;
    for t in toks {
        let space = !(prev.is_none()
            || matches!(prev, Some(VTok::Open))
            || matches!(t, VTok::Close)
            || sign);
        if space {
            s.push(' ');
        }
        sign = matches!(t, VTok::Op('-') | VTok::Op('+'))
            && matches!(prev, None | Some(VTok::Op(_)) | Some(VTok::Open));
        match t {
            VTok::Field(n) => s.push_str(&field(n)),
            VTok::Num(n) => s.push_str(n),
            VTok::Op(o) => s.push(*o),
            VTok::Open => s.push('('),
            VTok::Close => s.push(')'),
        }
        prev = Some(t);
    }
    s
}

/// KentOS's value expression as QGIS's, when it is fields, numbers and arithmetic.
fn qgis_value(kentos: &str) -> Option<String> {
    let Some(toks) = value_tokens(kentos, false) else {
        // With functions, word by word (docs/adr/0635 §6).
        return kentos_formats::style_expr::to_qgis(kentos);
    };
    Some(value_text(&toks, &|n| {
        format!("\"{}\"", n.replace('"', "\"\""))
    }))
}

/// QGIS's value expression as KentOS's, when it is fields, numbers and arithmetic.
fn kentos_value(qgis: &str) -> Option<String> {
    let Some(toks) = value_tokens(qgis, true) else {
        return kentos_formats::style_expr::to_kentos(qgis);
    };
    Some(value_text(&toks, &|n| format!("[{n}]")))
}

/// `-(e)`, or `e` when `e` is that already (an angle turned twice comes back as written).
fn negated(e: &str) -> String {
    if let Some(inner) = e.strip_prefix("-(").and_then(|r| r.strip_suffix(')')) {
        let mut depth = 0i32;
        let balanced = inner.chars().all(|ch| {
            match ch {
                '(' => depth += 1,
                ')' => depth -= 1,
                _ => {}
            }
            depth >= 0
        }) && depth == 0;
        if balanced {
            return inner.to_owned();
        }
    }
    format!("-({e})")
}

/// A layer's number at `key`: fixed, or data-defined when its expression says itself in QGIS's (then QGIS's
/// property `prop` takes it, `negate` for an angle turned the other way) and the fallback is written.
fn num_value(
    layer: &Value,
    key: &str,
    prop: &'static str,
    negate: bool,
    dds: &mut Vec<(&'static str, String)>,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<f64> {
    let v = layer.get(key)?;
    if let Some(e) = v.get("expr").and_then(Value::as_str)
        && let Some(q) = qgis_value(e)
    {
        dds.push((prop, if negate { negated(&q) } else { q }));
        return v.get("fallback").and_then(Value::as_f64);
    }
    sld::fixed_for(layer, key, lost, what, "QML'e").and_then(Value::as_f64)
}

/// A layer's colour at `key`: fixed, or data-defined when its expression says itself in QGIS's (then QGIS's property
/// `prop` takes it) and the fallback is written (a transparent colour without one).
fn colour_value(
    layer: &Value,
    key: &str,
    prop: &'static str,
    dds: &mut Vec<(&'static str, String)>,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<String> {
    let v = layer.get(key)?;
    if let Some(e) = v.get("expr").and_then(Value::as_str)
        && let Some(q) = qgis_value(e)
    {
        dds.push((prop, q));
        return Some(
            v.get("fallback")
                .and_then(rgba)
                .unwrap_or_else(|| "0,0,0,0".into()),
        );
    }
    sld::fixed_for(layer, key, lost, what, "QML'e").and_then(rgba)
}

/// A symbol layer's `<data_defined_properties>` of expressions.
fn dd_xml(dds: &[(&'static str, String)], pad: &str) -> String {
    let mut s = format!("{pad}<data_defined_properties>\n{pad}  <Option type=\"Map\">\n");
    let _ = writeln!(
        s,
        "{pad}    <Option name=\"name\" type=\"QString\" value=\"\"/>"
    );
    if dds.is_empty() {
        let _ = writeln!(s, "{pad}    <Option name=\"properties\"/>");
    } else {
        let _ = writeln!(s, "{pad}    <Option name=\"properties\" type=\"Map\">");
        for (name, e) in dds {
            let _ = writeln!(s, "{pad}      <Option name=\"{name}\" type=\"Map\">");
            let _ = writeln!(
                s,
                "{pad}        <Option name=\"active\" type=\"bool\" value=\"true\"/>"
            );
            let _ = writeln!(
                s,
                "{pad}        <Option name=\"expression\" type=\"QString\" value=\"{}\"/>",
                esc(e)
            );
            let _ = writeln!(
                s,
                "{pad}        <Option name=\"type\" type=\"int\" value=\"3\"/>"
            );
            let _ = writeln!(s, "{pad}      </Option>");
        }
        let _ = writeln!(s, "{pad}    </Option>");
    }
    let _ = writeln!(
        s,
        "{pad}    <Option name=\"type\" type=\"QString\" value=\"collection\"/>"
    );
    let _ = writeln!(s, "{pad}  </Option>\n{pad}</data_defined_properties>");
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
    let mut dds = Vec::new();
    let Some(color) = colour_value(layer, "color", "outlineColor", &mut dds, lost, what) else {
        lost.push(format!("{what}: çizginin rengi QML'in renklerinden değil"));
        return None;
    };
    let width =
        num_value(layer, "width", "outlineWidth", false, &mut dds, lost, what).unwrap_or(0.0);
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
    s.push_str(&dd_xml(&dds, &format!("{pad}  ")));
    let _ = writeln!(s, "{pad}</layer>");
    Some(s)
}

fn fill_layer(layer: &Value, pad: &str, lost: &mut Vec<String>, what: &str) -> Option<String> {
    let mut dds = Vec::new();
    let Some(color) = colour_value(layer, "color", "fillColor", &mut dds, lost, what) else {
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
    s.push_str(&dd_xml(&dds, &format!("{pad}  ")));
    let _ = writeln!(s, "{pad}</layer>");
    Some(s)
}

/// What a marker layer's three kinds share: its unit (QGIS's name), size and angle (each data-defined when its
/// expression says itself), and the losses of what QGIS's marker layers lack (offset, anchor).
fn marker_common(
    layer: &Value,
    default_size: f64,
    dds: &mut Vec<(&'static str, String)>,
    lost: &mut Vec<String>,
    what: &str,
) -> (String, f64, f64) {
    let unit = layer.get("unit").and_then(Value::as_str).unwrap_or("mm");
    let size = num_value(layer, "size", "size", false, dds, lost, what).unwrap_or(default_size);
    // KentOS turns counter-clockwise, QGIS clockwise.
    let angle = num_value(layer, "rotation", "angle", true, dds, lost, what)
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
    layer_losses(layer, lost, what);
    (unit_out(unit).to_owned(), size, angle)
}

/// A marker layer as QGIS's: a shape as `SimpleMarker`, an SVG drawing as `SvgMarker` (its text inside the QML,
/// `base64:`), a text as `FontMarker`.
fn marker_layer(layer: &Value, pad: &str, lost: &mut Vec<String>, what: &str) -> Option<String> {
    let t = layer.get("type").and_then(Value::as_str).unwrap_or("");
    match t {
        "shape" => {}
        "svg" => return svg_marker_layer(layer, pad, lost, what),
        "text" => return font_marker_layer(layer, pad, lost, what),
        _ => {
            lost.push(format!(
                "{what}: “{t}” işareti QML'e yazılmadı (şekil, SVG ve yazı işaretleri yazılır)"
            ));
            return None;
        }
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
    let mut dds = Vec::new();
    let (u, size, angle) = marker_common(layer, 2.0, &mut dds, lost, what);
    let fill = colour_value(layer, "fill", "fillColor", &mut dds, lost, what)
        .unwrap_or_else(|| "0,0,0,0".into());
    let stroke = colour_value(layer, "stroke", "outlineColor", &mut dds, lost, what);
    let stroke_width = layer
        .get("strokeWidth")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
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
    s.push_str(&dd_xml(&dds, &format!("{pad}  ")));
    let _ = writeln!(s, "{pad}</layer>");
    Some(s)
}

/// An SVG marker as `SvgMarker`: the drawing is the library asset's text (put into the layer as `svgData` by the
/// host), inside the QML as `base64:`; its fill and outline colours, size and angle.
fn svg_marker_layer(
    layer: &Value,
    pad: &str,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<String> {
    let Some(svg) = layer.get("svgData").and_then(Value::as_str) else {
        lost.push(format!(
            "{what}: SVG işaretinin çizimi kitaplıkta yok; işaret QML'e yazılmadı"
        ));
        return None;
    };
    let mut dds = Vec::new();
    let (u, size, angle) = marker_common(layer, 2.0, &mut dds, lost, what);
    let fill = colour_value(layer, "fill", "fillColor", &mut dds, lost, what)
        .unwrap_or_else(|| "0,0,0,255".into());
    let stroke = colour_value(layer, "stroke", "outlineColor", &mut dds, lost, what);
    let mut s = layer_open("SvgMarker", layer, pad);
    s.push_str(&options(
        &[
            ("angle", num(round3(angle))),
            ("color", fill),
            ("horizontal_anchor_point", "1".into()),
            (
                "name",
                format!("base64:{}", kentos_formats::style_extras::b64_encode(svg.as_bytes())),
            ),
            ("offset", "0,0".into()),
            ("offset_unit", "MM".into()),
            (
                "outline_color",
                stroke.clone().unwrap_or_else(|| "0,0,0,0".into()),
            ),
            // KentOS's SVG marker has no outline width: a drawing's own strokes take QGIS's usual 0.2.
            (
                "outline_width",
                if stroke.is_some() { "0.2" } else { "0" }.into(),
            ),
            ("outline_width_unit", u.clone()),
            ("scale_method", "diameter".into()),
            ("size", num(round3(size))),
            ("size_unit", u),
            ("vertical_anchor_point", "1".into()),
        ],
        &format!("{pad}  "),
    ));
    s.push_str(&dd_xml(&dds, &format!("{pad}  ")));
    let _ = writeln!(s, "{pad}</layer>");
    Some(s)
}

/// A text marker as `FontMarker`: QGIS's marker is one character (the first of the text), its font family and style
/// (KentOS's weight from 600 is bold), colour, halo as the outline, size and angle.
fn font_marker_layer(
    layer: &Value,
    pad: &str,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<String> {
    let mut dds = Vec::new();
    let text = layer.get("text");
    let (chr, dd) = match text {
        Some(t) if t.get("expr").is_some() => {
            let fallback = t.get("fallback").and_then(Value::as_str).unwrap_or("");
            let dd = t
                .get("expr")
                .and_then(Value::as_str)
                .and_then(qgis_value);
            if dd.is_none() {
                lost.push(format!(
                    "{what}: yazı işaretinin metni bir ifadeyle (veriye göre); QML'e yedek değeri yazıldı"
                ));
            }
            (fallback.to_owned(), dd)
        }
        Some(t) => (t.as_str().unwrap_or("").to_owned(), None),
        None => (String::new(), None),
    };
    if let Some(d) = dd {
        dds.push(("char", d));
    }
    if chr.chars().count() > 1 {
        lost.push(format!(
            "{what}: QGIS'in yazı işareti tek karakterdir; metnin yalnız ilk karakteri yazıldı"
        ));
    }
    let first: String = chr.chars().take(1).collect();
    if first.is_empty() && !dds.iter().any(|(p, _)| *p == "char") {
        lost.push(format!(
            "{what}: yazı işaretinin metni boş; işaret QML'e yazılmadı"
        ));
        return None;
    }
    let (u, size, angle) = marker_common(layer, 2.0, &mut dds, lost, what);
    let color = colour_value(layer, "color", "fillColor", &mut dds, lost, what)
        .unwrap_or_else(|| "0,0,0,255".into());
    let bold = layer
        .get("weight")
        .and_then(Value::as_f64)
        .is_some_and(|w| w >= 600.0);
    let italic = layer.get("italic") == Some(&Value::Bool(true));
    let style = match (bold, italic) {
        (true, true) => "Bold Italic",
        (true, false) => "Bold",
        (false, true) => "Italic",
        (false, false) => "Regular",
    };
    let halo = layer.get("halo").filter(|h| h.is_object());
    let (outline, outline_width) = match halo {
        Some(h) => (
            h.get("color")
                .and_then(rgba)
                .unwrap_or_else(|| "0,0,0,0".into()),
            h.get("width").and_then(Value::as_f64).unwrap_or(0.0),
        ),
        None => ("0,0,0,0".into(), 0.0),
    };
    let mut s = layer_open("FontMarker", layer, pad);
    s.push_str(&options(
        &[
            ("angle", num(round3(angle))),
            ("chr", first),
            ("color", color),
            (
                "font",
                layer
                    .get("font")
                    .and_then(Value::as_str)
                    .unwrap_or("Sans Serif")
                    .to_owned(),
            ),
            ("font_style", style.into()),
            ("horizontal_anchor_point", "1".into()),
            ("joinstyle", "miter".into()),
            ("offset", "0,0".into()),
            ("offset_unit", "MM".into()),
            ("outline_color", outline),
            ("outline_width", num(round3(outline_width))),
            ("outline_width_unit", u.clone()),
            ("size", num(round3(size))),
            ("size_unit", u),
            ("vertical_anchor_point", "1".into()),
        ],
        &format!("{pad}  "),
    ));
    s.push_str(&dd_xml(&dds, &format!("{pad}  ")));
    let _ = writeln!(s, "{pad}</layer>");
    Some(s)
}

/// A gradient as `GradientFill` (docs/adr/0571 §1). QGIS draws a linear gradient from its first reference point to
/// its second, a radial one from the first outward to the second's distance; its fractions are of the object's box.
/// So KentOS's linear gradient is the box's west-to-east pair turned by the angle (QGIS turns counter-clockwise too);
/// the sphere is the box's centre out to its corner; the cylinder is the half-box with `spread` reflecting.
fn gradient_layer(
    layer: &Value,
    pad: &str,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<String> {
    let mut dds = Vec::new();
    let mut c1 = (
        colour_value(layer, "color", "fillColor", &mut dds, lost, what),
        "fillColor",
    );
    let mut c2 = (
        colour_value(layer, "color2", "color2", &mut dds, lost, what),
        "color2",
    );
    let (Some(_), Some(_)) = (&c1.0, &c2.0) else {
        lost.push(format!(
            "{what}: degradenin iki rengi QML'in renklerinden değil; degrade yazılmadı"
        ));
        return None;
    };
    let shape = layer
        .get("shape")
        .and_then(Value::as_str)
        .unwrap_or("linear");
    let inverted = layer.get("inverted") == Some(&Value::Bool(true));
    // QGIS's first colour is where its gradient starts: the box's west (linear), its edge (cylinder), its centre (sphere).
    let swap = match shape {
        "spherical" => !inverted,
        _ => inverted,
    };
    if swap {
        std::mem::swap(&mut c1.0, &mut c2.0);
    }
    // The data-defined properties follow their colours.
    for (prop, _) in &mut dds {
        let first = (*prop == "fillColor") != swap;
        *prop = if first { "fillColor" } else { "color2" };
    }
    let (ty, spread, p1, p2) = match shape {
        "spherical" => ("1", "0", "0.5,0.5", "1,1"),
        "cylinder" => ("0", "1", "0,0.5", "0.5,0.5"),
        _ => ("0", "0", "0,0.5", "1,0.5"),
    };
    let angle = if shape == "spherical" {
        0.0
    } else {
        layer.get("angle").and_then(Value::as_f64).unwrap_or(0.0)
    };
    if shape != "spherical" && angle.rem_euclid(90.0) != 0.0 {
        lost.push(format!(
            "{what}: açılı degradenin uçları QGIS'te kutunun yan kenarlarına oturur (KentOS'ta kutunun doğrultudaki izdüşümüne); 0, 90, 180 ve 270 dışındaki açıda renk geçişi biraz kısalır"
        ));
    }
    layer_losses(layer, lost, what);
    let mut s = layer_open("GradientFill", layer, pad);
    s.push_str(&options(
        &[
            ("angle", num(round3(angle))),
            ("color", c1.0.unwrap_or_default()),
            ("color_type", "0".into()),
            ("coordinate_mode", "0".into()),
            ("gradient_color2", c2.0.unwrap_or_default()),
            ("offset", "0,0".into()),
            ("offset_unit", "MM".into()),
            ("reference_point1", p1.into()),
            ("reference_point1_iscentroid", "0".into()),
            ("reference_point2", p2.into()),
            ("reference_point2_iscentroid", "0".into()),
            ("spread", spread.into()),
            ("type", ty.into()),
        ],
        &format!("{pad}  "),
    ));
    s.push_str(&dd_xml(&dds, &format!("{pad}  ")));
    let _ = writeln!(s, "{pad}</layer>");
    Some(s)
}

/// A point pattern as `PointPatternFill` (docs/adr/0571 §2): the marker's cell spacing, the half-spacing shift of
/// every second row (`stagger`), the rotation and the start offset, its marker the sub-symbol.
fn pattern_layer(
    layer: &Value,
    name: &str,
    i: usize,
    pad: &str,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<String> {
    let unit = layer.get("unit").and_then(Value::as_str).unwrap_or("mm");
    let sx = layer.get("spacingX").and_then(Value::as_f64).unwrap_or(0.0);
    let sy = layer.get("spacingY").and_then(Value::as_f64).unwrap_or(0.0);
    if !(sx > 0.0 && sy > 0.0) {
        lost.push(format!("{what}: aralığı olmayan desen QML'e yazılmadı"));
        return None;
    }
    let marker = layer.get("marker").cloned().unwrap_or(Value::Null);
    let at = format!("{pad}  ");
    let sub = symbol_xml(&marker, "marker", &format!("@{name}@{i}"), &at, lost, what)?;
    let more = |key: &str, none: f64| {
        layer
            .get(key)
            .and_then(Value::as_f64)
            .is_some_and(|v| v != none)
    };
    if more("jitter", 0.0) || more("coverage", 1.0) || more("seed", 0.0) {
        lost.push(format!(
            "{what}: desenin rastgele sapması, doluluğu ve tohumu QML'e yazılmadı (QGIS'in nokta deseninde sabit dizilim vardır)"
        ));
    }
    lost.push(format!(
        "{what}: desenin dizilimi QGIS'te nesnenin kutusundan, KentOS'ta çizimin başlangıç noktasından başlar; işaretlerin yeri bir hücre kayabilir"
    ));
    layer_losses(layer, lost, what);
    let stagger = layer.get("stagger") == Some(&Value::Bool(true));
    let offset = layer
        .get("offset")
        .and_then(Value::as_array)
        .map(|o| {
            (
                o.first().and_then(Value::as_f64).unwrap_or(0.0),
                o.get(1).and_then(Value::as_f64).unwrap_or(0.0),
            )
        })
        .unwrap_or((0.0, 0.0));
    let u = unit_out(unit).to_owned();
    let mut s = layer_open("PointPatternFill", layer, pad);
    s.push_str(&options(
        &[
            (
                "angle",
                num(round3(
                    // KentOS turns the grid counter-clockwise, QGIS clockwise.
                    -layer.get("angle").and_then(Value::as_f64).unwrap_or(0.0) + 0.0,
                )),
            ),
            ("clip_mode", "shape".into()),
            ("coordinate_reference", "feature".into()),
            (
                "displacement_x",
                num(round3(if stagger { sx / 2.0 } else { 0.0 })),
            ),
            ("displacement_x_unit", u.clone()),
            ("displacement_y", "0".into()),
            ("displacement_y_unit", u.clone()),
            ("distance_x", num(round3(sx))),
            ("distance_x_unit", u.clone()),
            ("distance_y", num(round3(sy))),
            ("distance_y_unit", u.clone()),
            ("offset_x", num(round3(offset.0))),
            ("offset_x_unit", u.clone()),
            // QGIS counts the offset's y downward, KentOS northward.
            ("offset_y", num(round3(-offset.1 + 0.0))),
            ("offset_y_unit", u),
            ("outline_width_unit", "MM".into()),
            ("random_deviation_x", "0".into()),
            ("random_deviation_y", "0".into()),
            ("seed", "0".into()),
        ],
        &at,
    ));
    s.push_str(&sub);
    let _ = writeln!(s, "{pad}</layer>");
    Some(s)
}

/// A hatch as QGIS's line pattern fill: the lines' angle (both counter-clockwise from east), spacing and offset,
/// its line a simple line of the hatch's colour, width and dashes.
fn hatch_layer(
    layer: &Value,
    name: &str,
    i: usize,
    pad: &str,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<String> {
    let unit = layer.get("unit").and_then(Value::as_str).unwrap_or("mm");
    let spacing = layer.get("spacing").and_then(Value::as_f64).unwrap_or(0.0);
    if spacing <= 0.0 {
        lost.push(format!("{what}: aralığı olmayan tarama QML'e yazılmadı"));
        return None;
    }
    let line = json!({
        "type": "simpleLine", "color": layer.get("color").cloned().unwrap_or(Value::Null),
        "width": layer.get("width").cloned().unwrap_or(json!(0)), "unit": unit, "cap": "butt",
        "dash": layer.get("dash").cloned().unwrap_or(Value::Null),
    });
    let at = format!("{pad}  ");
    let sub = symbol_xml(
        &json!({ "type": "line", "layers": [line] }),
        "line",
        &format!("@{name}@{i}"),
        &at,
        lost,
        what,
    )?;
    for (key, words) in [
        ("stagger", "kaydırması"),
        ("dashOffset", "kesik başlangıcı"),
    ] {
        if layer
            .get(key)
            .and_then(Value::as_f64)
            .is_some_and(|v| v != 0.0)
        {
            lost.push(format!("{what}: taramanın {words} QML'e yazılmadı"));
        }
    }
    layer_losses(layer, lost, what);
    let u = unit_out(unit).to_owned();
    let mut s = layer_open("LinePatternFill", layer, pad);
    s.push_str(&options(
        &[
            (
                "angle",
                num(round3(
                    layer.get("angle").and_then(Value::as_f64).unwrap_or(0.0),
                )),
            ),
            ("clip_mode", "during_render".into()),
            ("coordinate_reference", "feature".into()),
            ("distance", num(round3(spacing))),
            ("distance_unit", u.clone()),
            (
                "offset",
                num(round3(
                    layer.get("offset").and_then(Value::as_f64).unwrap_or(0.0),
                )),
            ),
            ("offset_unit", u),
        ],
        &at,
    ));
    s.push_str(&sub);
    let _ = writeln!(s, "{pad}</layer>");
    Some(s)
}

/// KentOS's marker line placements and QGIS's names for them.
const PLACEMENTS: [(&str, &str); 7] = [
    ("interval", "Interval"),
    ("vertex", "Vertex"),
    ("innerVertex", "InnerVertices"),
    ("first", "FirstVertex"),
    ("last", "LastVertex"),
    ("center", "CentralPoint"),
    ("segmentCenter", "SegmentCenter"),
];

/// A marker line as QGIS's: its placement, interval, offset along the line, rotation and marker.
fn marker_line_layer(
    layer: &Value,
    name: &str,
    i: usize,
    pad: &str,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<String> {
    let placement = layer.get("placement").and_then(Value::as_str).unwrap_or("");
    let Some(&(_, q)) = PLACEMENTS.iter().find(|(k, _)| *k == placement) else {
        lost.push(format!(
            "{what}: işaretli çizginin “{placement}” yerleşimi QGIS'te yok; katman yazılmadı"
        ));
        return None;
    };
    let unit = layer.get("unit").and_then(Value::as_str).unwrap_or("mm");
    let marker = layer.get("marker").cloned().unwrap_or(Value::Null);
    let at = format!("{pad}  ");
    let sub = symbol_xml(&marker, "marker", &format!("@{name}@{i}"), &at, lost, what)?;
    for (key, words) in [("offset", "paralel kaydırması"), ("group", "işaret grubu")] {
        if layer
            .get(key)
            .is_some_and(|v| !v.is_null() && v != &json!(0))
        {
            lost.push(format!("{what}: işaretli çizginin {words} QML'e yazılmadı"));
        }
    }
    layer_losses(layer, lost, what);
    let ring = match layer.get("rings").and_then(Value::as_str) {
        Some("exterior") => "1",
        Some("interior") => "2",
        _ => "0",
    };
    let u = unit_out(unit).to_owned();
    let mut s = layer_open("MarkerLine", layer, pad);
    s.push_str(&options(
        &[
            (
                "interval",
                num(round3(
                    layer.get("interval").and_then(Value::as_f64).unwrap_or(3.0),
                )),
            ),
            ("interval_unit", u.clone()),
            ("offset", "0".into()),
            (
                "offset_along_line",
                num(round3(
                    layer
                        .get("offsetAlong")
                        .and_then(Value::as_f64)
                        .unwrap_or(0.0),
                )),
            ),
            ("offset_along_line_unit", u),
            ("offset_unit", "MM".into()),
            ("placements", q.into()),
            ("ring_filter", ring.into()),
            (
                "rotate",
                if layer.get("rotate") == Some(&Value::Bool(false)) {
                    "0"
                } else {
                    "1"
                }
                .into(),
            ),
        ],
        &at,
    ));
    s.push_str(&sub);
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
            ("fill", "hatchFill") => hatch_layer(layer, name, i, &inner, lost, what),
            ("fill", "gradientFill") => gradient_layer(layer, &inner, lost, what),
            ("fill", "patternFill") => pattern_layer(layer, name, i, &inner, lost, what),
            (_, "markerLine") if key != "marker" => {
                marker_line_layer(layer, name, i, &inner, lost, what)
            }
            (_, other) => {
                let words = match other {
                    "hatchFill" => "tarama dolgusu (çizgi simgesinde)",
                    "gradientFill" => "degrade dolgu (çizgi simgesinde)",
                    "patternFill" => "desen dolgusu (çizgi simgesinde)",
                    "imageFill" => "resim dolgusu",
                    "markerLine" => "işaretli çizgi (işaret simgesinde)",
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
                // A filter with functions goes across word by word (docs/adr/0635 §6).
                Some(src) => match parse_filter(src)
                    .map(|f| qgis_text(&f))
                    .or_else(|| kentos_formats::style_expr::to_qgis(src))
                {
                    Some(f) => Some(f),
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

/// A renderer as QGIS's `<renderer-v2>` element (its symbols for `key`'s class) and the classes and rules written;
/// nothing when no symbol could be written.
fn element(
    renderer: &Value,
    key: &'static str,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
) -> (String, usize) {
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
                .add(renderer.get("symbols"), library, lost, "Simge")
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
                        lost,
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
                    lost,
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
                        lost,
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
                lost,
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
                "“{other}” görünüşü QML'e yazılmadı (tek simge, kategorili, aralıklı, kurallı, ısı haritası, ters alan, kümeleme ve yayma yazılır)"
            ));
            String::new()
        }
    };
    if symbols.xml.is_empty() {
        rules = 0;
    }
    if rules == 0 || head.is_empty() {
        return (String::new(), 0);
    }
    let mut xml = String::new();
    let _ = writeln!(xml, "  {head}");
    xml.push_str(&body);
    xml.push_str("    <symbols>\n");
    for s in &symbols.xml {
        xml.push_str(s);
    }
    xml.push_str("    </symbols>\n");
    xml.push_str("  </renderer-v2>\n");
    (xml, rules)
}

/// The QML of a layer drawn by `renderer` (the app's JSON) with its objects' `kinds`; library symbols by id through
/// `library`.
pub fn write(
    renderer: &Value,
    classes: &[kentos_formats::style_labels::Class<'_>],
    kinds: Kinds,
    library: &dyn Fn(&str) -> Option<Value>,
) -> Written {
    let mut lost = Vec::new();
    let (key, geom) = class_of(kinds, &mut lost);
    let (element, rules) = match renderer.get("type").and_then(Value::as_str) {
        Some("heatmap" | "inverted" | "cluster" | "displacement") => {
            thematic::element(renderer, key, library, &mut lost)
        }
        _ => element(renderer, key, library, &mut lost),
    };
    let (labeling, label_count) = labels::labeling_xml(classes, geom, &mut lost);
    let mut xml = String::new();
    xml.push_str("<!DOCTYPE qgis PUBLIC 'http://mrcc.com/qgis.dtd' 'SYSTEM'>\n");
    if label_count > 0 {
        xml.push_str("<qgis version=\"4.2.2-KentOS\" styleCategories=\"Symbology|Labeling\" labelsEnabled=\"1\">\n");
    } else {
        xml.push_str("<qgis version=\"4.2.2-KentOS\" styleCategories=\"Symbology\">\n");
    }
    if !element.is_empty() {
        xml.push_str(&element);
    }
    xml.push_str(&labeling);
    let _ = writeln!(xml, "  <layerGeometryType>{geom}</layerGeometryType>");
    xml.push_str("</qgis>\n");
    Written {
        xml,
        lost,
        rules,
        labels: label_count,
    }
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
/// A symbol layer's active data-defined properties, in order: the property's name and its expression as QGIS writes
/// it (a field is `"field"`).
fn dd_props(layer: Node<'_, '_>) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Some(props) = layer
        .children()
        .find(|c| c.is_element() && c.tag_name().name() == "data_defined_properties")
        .and_then(|d| child(d, "Option"))
        .and_then(|m| {
            m.children()
                .find(|c| c.is_element() && c.attribute("name") == Some("properties"))
        })
    else {
        return out;
    };
    for p in props.children().filter(|c| c.is_element()) {
        let get = |k: &str| {
            p.children()
                .find(|c| c.is_element() && c.attribute("name") == Some(k))
                .and_then(|c| c.attribute("value"))
        };
        if get("active") != Some("true") {
            continue;
        }
        let name = p.attribute("name").unwrap_or("").to_owned();
        let expr = match (get("type"), get("field"), get("expression")) {
            (Some("2"), Some(f), _) => format!("\"{}\"", f.replace('"', "\"\"")),
            (_, _, Some(e)) => e.to_owned(),
            (_, Some(f), None) => format!("\"{}\"", f.replace('"', "\"\"")),
            _ => continue,
        };
        out.push((name, expr));
    }
    out
}

/// One data-defined property KentOS takes: QGIS's name, the KentOS layer type it goes to, the key it becomes and
/// whether it turns the other way (an angle).
type DdTake = (&'static str, &'static str, &'static str, bool);

/// The layer's data-defined values KentOS takes (`supported`); the static value stays the fallback. The others are said.
fn take_dd(
    layer: Node<'_, '_>,
    supported: &[DdTake],
    taken: &mut [Value],
    lost: &mut Vec<String>,
    what: &str,
) {
    for (name, q) in dd_props(layer) {
        let Some(&(_, kind, key, negate)) = supported.iter().find(|(p, _, _, _)| *p == name) else {
            lost.push(format!(
                "{what}: QGIS'in “{name}” veriye bağlı değeri KentOS'un bu katmanında yok; sabit değeri alındı"
            ));
            continue;
        };
        let Some(e) = kentos_value(&q) else {
            lost.push(format!(
                "{what}: “{name}” değerinin “{q}” ifadesi KentOS'un ifadesine çevrilemedi; sabit değeri alındı"
            ));
            continue;
        };
        let Some(t) = taken
            .iter_mut()
            .find(|t| t.get("type").and_then(Value::as_str) == Some(kind))
        else {
            lost.push(format!(
                "{what}: QGIS'in “{name}” veriye bağlı değerinin katmanı alınmadı; değer alınmadı"
            ));
            continue;
        };
        let expr = if negate { negated(&e) } else { e };
        t[key] = match t.get(key).filter(|f| !f.is_null()).cloned() {
            Some(fallback) => json!({ "expr": expr, "fallback": fallback }),
            None => json!({ "expr": expr }),
        };
    }
}

/// The names of the layer's active data-defined properties.
fn dd_names(layer: Node<'_, '_>) -> Vec<String> {
    dd_props(layer).into_iter().map(|(n, _)| n).collect()
}

fn read_line(
    m: &Map<String, Value>,
    id: String,
    dd_color: bool,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<Value> {
    let style = prop(m, "line_style").unwrap_or("solid");
    if style == "no" {
        return None;
    }
    let color = colour_in(prop(m, "line_color").or(prop(m, "color")));
    if color.is_none() && !dd_color {
        return None;
    }
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
    let mut l = json!({ "id": id, "type": "simpleLine", "width": width, "unit": unit });
    if let Some(c) = color {
        l["color"] = json!(c);
    }
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

/// An ellipse marker's properties as a simple marker's: its shape (`rectangle` a square), its larger side the size
/// (different sides said).
fn ellipse_props(m: &Map<String, Value>, lost: &mut Vec<String>, what: &str) -> Map<String, Value> {
    let mut pm = m.clone();
    let name = match prop(m, "symbol_name").unwrap_or("circle") {
        "rectangle" => "square",
        n => n,
    };
    pm.insert("name".into(), json!(name));
    let (w, h) = (prop_num(m, "symbol_width"), prop_num(m, "symbol_height"));
    let size = match (w, h) {
        (Some(w), Some(h)) => {
            if (w - h).abs() > 1e-9 {
                lost.push(format!(
                    "{what}: elips işaretinin eni ve boyu farklı; büyüğü boy alındı"
                ));
            }
            w.max(h)
        }
        (Some(x), None) | (None, Some(x)) => x,
        (None, None) => 2.0,
    };
    pm.insert("size".into(), json!(num(size)));
    if let Some(u) = m.get("symbol_width_unit") {
        pm.insert("size_unit".into(), u.clone());
    }
    pm
}

/// A filled marker's properties as a simple marker's: the fill and outline of its fill sub-symbol.
fn filled_props(
    layer: Node<'_, '_>,
    m: &Map<String, Value>,
    lost: &mut Vec<String>,
    what: &str,
) -> Map<String, Value> {
    let mut pm = m.clone();
    pm.remove("color");
    pm.insert("outline_style".into(), json!("no"));
    match child(layer, "symbol").and_then(|sub| read_symbol(sub, lost, what)) {
        Some((k, sym)) if k == "fill" => {
            let layers = sym["layers"].as_array().cloned().unwrap_or_default();
            if let Some(c) = layers
                .iter()
                .find(|l| l["type"] == "simpleFill")
                .and_then(|l| l["color"].as_str())
            {
                pm.insert("color".into(), json!(c));
            }
            if let Some(l) = layers.iter().find(|l| l["type"] == "simpleLine")
                && let Some(c) = l["color"].as_str()
            {
                pm.insert("outline_style".into(), json!("solid"));
                pm.insert("outline_color".into(), json!(c));
                pm.insert(
                    "outline_width".into(),
                    json!(num(l["width"].as_f64().unwrap_or(0.0))),
                );
                pm.insert(
                    "outline_width_unit".into(),
                    json!(unit_out(l["unit"].as_str().unwrap_or("mm"))),
                );
            }
        }
        _ => lost.push(format!("{what}: dolgulu işaretin dolgusu alınmadı")),
    }
    pm
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

thread_local! {
    /// The SVG drawings the QML being read carried (`base64:` in an SVG marker), by the ids the markers refer to.
    static ASSETS: std::cell::RefCell<Vec<StyleAsset>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// An SVG marker as KentOS's: the drawing a `base64:` name carries becomes a library drawing (`assets`), a file's
/// path cannot be read here.
fn read_svg_marker(m: &Map<String, Value>, lost: &mut Vec<String>, what: &str) -> Option<Value> {
    let name = prop(m, "name").unwrap_or("");
    let encoded = name
        .strip_prefix("base64:")
        .or_else(|| name.strip_prefix("data:image/svg+xml;base64,"));
    let svg = encoded
        .and_then(kentos_formats::style_extras::b64_decode)
        .map(|b| String::from_utf8_lossy(&b).into_owned())
        .filter(|t| kentos_formats::style_extras::looks_like_svg(t));
    let Some(svg) = svg else {
        lost.push(format!(
            "{what}: SVG işareti “{}” dosyasına bağlı; KentOS çizimin içindeki SVG'yi alır, dosyayı okumaz; işaret alınmadı",
            name.chars().take(60).collect::<String>()
        ));
        return None;
    };
    let id = kentos_formats::style_extras::asset_id("qml", &svg);
    ASSETS.with(|a| {
        let mut a = a.borrow_mut();
        if !a.iter().any(|x| x.id == id) {
            a.push(StyleAsset {
                id: id.clone(),
                name: "QGIS'ten gelen SVG işareti".into(),
                svg,
            });
        }
    });
    let (size, unit) = size_in(prop_num(m, "size").unwrap_or(2.0), prop(m, "size_unit"))
        .unwrap_or_else(|| {
            lost.push(format!(
                "{what}: işaretin yüzde boyu KentOS'ta yok; 2 mm alındı"
            ));
            (2.0, "mm")
        });
    let mut v = json!({ "id": "v", "type": "svg", "asset": id, "size": size, "unit": unit });
    if let Some(c) = colour_in(prop(m, "color")) {
        v["fill"] = json!(c);
    }
    if let Some(c) = colour_in(prop(m, "outline_color")) {
        v["stroke"] = json!(c);
    }
    marker_pose(m, &mut v, lost, what);
    Some(v)
}

/// What a QGIS marker layer's angle, offset and anchor say about a KentOS marker (the angle turns the other way).
fn marker_pose(m: &Map<String, Value>, v: &mut Value, lost: &mut Vec<String>, what: &str) {
    if let Some(a) = prop_num(m, "angle").filter(|a| *a != 0.0) {
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
}

/// A font marker as KentOS's text marker: the character, its font, its style (bold, italic), its colour, its outline
/// as the halo.
fn read_font_marker(m: &Map<String, Value>, lost: &mut Vec<String>, what: &str) -> Value {
    let (size, unit) = size_in(prop_num(m, "size").unwrap_or(2.0), prop(m, "size_unit"))
        .unwrap_or_else(|| {
            lost.push(format!(
                "{what}: işaretin yüzde boyu KentOS'ta yok; 2 mm alındı"
            ));
            (2.0, "mm")
        });
    let mut v = json!({
        "id": "t", "type": "text", "text": prop(m, "chr").unwrap_or(""), "size": size, "unit": unit,
    });
    if let Some(f) = prop(m, "font").filter(|f| !f.is_empty()) {
        v["font"] = json!(f);
    }
    let style = prop(m, "font_style").unwrap_or("");
    if style.to_ascii_lowercase().contains("bold") {
        v["weight"] = json!(700);
    }
    if style.to_ascii_lowercase().contains("italic") || style.to_ascii_lowercase().contains("oblique")
    {
        v["italic"] = json!(true);
    }
    if let Some(c) = colour_in(prop(m, "color")) {
        v["color"] = json!(c);
    }
    if prop_num(m, "outline_width").is_some_and(|w| w > 0.0)
        && let Some(c) = colour_in(prop(m, "outline_color"))
    {
        match size_as(
            prop_num(m, "outline_width").unwrap_or(0.0),
            prop(m, "outline_width_unit"),
            unit,
        ) {
            Some(w) => v["halo"] = json!({ "color": c, "width": w }),
            None => lost.push(format!(
                "{what}: yazı işaretinin çizgi kalınlığının birimi boyununkinden başka; hale alınmadı"
            )),
        }
    }
    marker_pose(m, &mut v, lost, what);
    v
}

/// A gradient fill as KentOS's (docs/adr/0571 §1): the type (linear, radial as the sphere; the reflected half-box pair as
/// the cylinder), the box's direction turned by the angle, the two colours; the data-defined ones follow their colours.
/// Returns the layer and the data-defined properties it takes.
fn read_gradient(
    layer: Node<'_, '_>,
    m: &Map<String, Value>,
    i: usize,
    lost: &mut Vec<String>,
    what: &str,
) -> (Value, Vec<DdTake>) {
    let pair = |k: &str, default: (f64, f64)| {
        prop(m, k)
            .and_then(|t| {
                let mut it = t.split(',').map(|x| x.trim().parse::<f64>().ok());
                Some((it.next()??, it.next()??))
            })
            .unwrap_or(default)
    };
    let p1 = pair("reference_point1", (0.5, 0.0));
    let p2 = pair("reference_point2", (0.5, 1.0));
    let kind = prop(m, "type").or_else(|| prop(m, "gradient_type")).unwrap_or("0");
    let spread = prop(m, "spread").unwrap_or("0");
    let (mut c1, mut c2) = (
        colour_in(prop(m, "color")),
        colour_in(prop(m, "gradient_color2")),
    );
    if prop(m, "color_type") == Some("1") {
        // QGIS 4 writes the ramp's colours in the layer's own properties, QGIS 3 in a `<colorramp>` of its own.
        let r = child(layer, "colorramp").map_or_else(|| m.clone(), props);
        match (colour_in(prop(&r, "color1")), colour_in(prop(&r, "color2"))) {
            (Some(a), Some(b)) => {
                c1 = Some(a);
                c2 = Some(b);
                if prop(&r, "stops").is_some_and(|t| !t.is_empty()) {
                    lost.push(format!(
                        "{what}: degradenin renk rampasının ara durakları alınmadı; ilk ve son rengi alındı"
                    ));
                }
            }
            _ => lost.push(format!(
                "{what}: degradenin renk rampası okunamadı; iki rengi alındı"
            )),
        }
    }
    let half = |a: (f64, f64), b: (f64, f64)| a == (0.0, 0.5) && b == (0.5, 0.5);
    let shape = match kind {
        "1" => "spherical",
        "2" => {
            lost.push(format!(
                "{what}: konik degrade KentOS'ta yok; doğrusal alındı"
            ));
            "linear"
        }
        _ if spread == "1" && half(p1, p2) => "cylinder",
        _ => "linear",
    };
    if shape != "cylinder" && spread != "0" && kind != "1" {
        lost.push(format!(
            "{what}: degradenin yansıtma ve yineleme yayılımı KentOS'ta yok (yalnız kutunun yarısıyla yansıtma silindir olur); alınmadı"
        ));
    }
    let mut g = json!({ "id": format!("g{i}"), "type": "gradientFill", "shape": shape });
    let mut takes: Vec<DdTake> = Vec::new();
    // QGIS's first colour is where its gradient starts: the box's west or edge, or the sphere's centre.
    let (k1, k2) = if shape == "spherical" {
        ("color2", "color")
    } else {
        ("color", "color2")
    };
    if let Some(c) = c1 {
        g[k1] = json!(c);
    }
    if let Some(c) = c2 {
        g[k2] = json!(c);
    }
    takes.push(("fillColor", "gradientFill", k1, false));
    takes.push(("color2", "gradientFill", k2, false));
    if shape != "spherical" {
        let along = libm::atan2(-(p2.1 - p1.1), p2.0 - p1.0).to_degrees();
        let angle = (prop_num(m, "angle").unwrap_or(0.0) + along).rem_euclid(360.0);
        let angle = round3(angle);
        if angle != 0.0 {
            g["angle"] = json!(angle);
        }
        let standard = |a: (f64, f64), b: (f64, f64)| {
            (a == (0.0, 0.5) && b == (1.0, 0.5))
                || (a == (1.0, 0.5) && b == (0.0, 0.5))
                || (a == (0.5, 0.0) && b == (0.5, 1.0))
                || (a == (0.5, 1.0) && b == (0.5, 0.0))
                || half(a, b)
        };
        if !standard(p1, p2) || angle.rem_euclid(90.0) != 0.0 {
            lost.push(format!(
                "{what}: degradenin uçları QGIS'te kutunun kesirleriyle verilir; KentOS'ta kutunun doğrultudaki izdüşümüdür; yön alındı, renk geçişinin uzunluğu kutuya oturtuldu"
            ));
        }
    }
    if prop(m, "coordinate_mode") == Some("1") {
        lost.push(format!(
            "{what}: degradenin görünüm kutusuna göre koordinatı alınmadı (KentOS nesnenin kutusunu kullanır)"
        ));
    }
    if prop(m, "offset").is_some_and(|o| {
        o.split(',')
            .any(|x| x.trim().parse::<f64>().is_ok_and(|x| x != 0.0))
    }) {
        lost.push(format!("{what}: degradenin kaydırması alınmadı"));
    }
    (g, takes)
}

/// A point pattern fill as KentOS's pattern (docs/adr/0571 §2): its marker sub-symbol, cell spacing, the half-spacing
/// shift of every second row, rotation and offset.
fn read_pattern(
    layer: Node<'_, '_>,
    m: &Map<String, Value>,
    i: usize,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<Value> {
    let Some((k, marker)) = child(layer, "symbol").and_then(|sub| read_symbol(sub, lost, what))
    else {
        lost.push(format!("{what}: nokta deseninin işareti alınmadı"));
        return None;
    };
    if k != "marker" {
        lost.push(format!("{what}: nokta deseninin işareti alınmadı"));
        return None;
    }
    let Some((sx, unit)) = size_in(
        prop_num(m, "distance_x").unwrap_or(15.0),
        prop(m, "distance_x_unit"),
    ) else {
        lost.push(format!(
            "{what}: nokta deseninin yüzde aralığı KentOS'ta yok; desen alınmadı"
        ));
        return None;
    };
    let Some(sy) = size_as(
        prop_num(m, "distance_y").unwrap_or(15.0),
        prop(m, "distance_y_unit"),
        unit,
    ) else {
        lost.push(format!(
            "{what}: nokta deseninin iki aralığının birimi ayrı; desen alınmadı"
        ));
        return None;
    };
    let mut v = json!({
        "id": format!("pt{i}"), "type": "patternFill", "marker": marker, "spacingX": sx, "spacingY": sy, "unit": unit,
    });
    let shift = |k: &str| prop_num(m, k).filter(|x| *x != 0.0);
    match (
        shift("displacement_x").map(|d| size_as(d, prop(m, "displacement_x_unit"), unit)),
        shift("displacement_y"),
    ) {
        (Some(Some(d)), None) if (d - sx / 2.0).abs() < 1e-9 => v["stagger"] = json!(true),
        (None, None) => {}
        _ => lost.push(format!(
            "{what}: nokta deseninin kayması KentOS'un yarım aralık kaymasından başka; alınmadı"
        )),
    }
    if let Some(a) = prop_num(m, "angle").filter(|a| *a != 0.0) {
        // QGIS turns the grid clockwise, KentOS counter-clockwise.
        v["angle"] = json!(-a);
    }
    let ox = prop_num(m, "offset_x").unwrap_or(0.0);
    let oy = prop_num(m, "offset_y").unwrap_or(0.0);
    if ox != 0.0 || oy != 0.0 {
        match (
            size_as(ox, prop(m, "offset_x_unit"), unit),
            size_as(oy, prop(m, "offset_y_unit"), unit),
        ) {
            // QGIS counts the offset's y downward, KentOS northward.
            (Some(x), Some(y)) => v["offset"] = json!([x, -y + 0.0]),
            _ => lost.push(format!(
                "{what}: nokta deseninin başlangıç kaymasının birimi aralığınkinden başka; alınmadı"
            )),
        }
    }
    if ["random_deviation_x", "random_deviation_y"]
        .iter()
        .any(|k| prop_num(m, k).is_some_and(|d| d != 0.0))
    {
        lost.push(format!(
            "{what}: nokta deseninin rastgele sapması KentOS'un sapmasıyla aynı değil; alınmadı"
        ));
    }
    Some(v)
}

/// A line pattern fill as KentOS's hatch: its angle, spacing and offset, its line's colour, width and dashes.
fn read_hatch(
    layer: Node<'_, '_>,
    m: &Map<String, Value>,
    i: usize,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<Value> {
    let Some((spacing, unit)) = size_in(
        prop_num(m, "distance").unwrap_or(5.0),
        prop(m, "distance_unit"),
    ) else {
        lost.push(format!(
            "{what}: çizgi deseninin yüzde aralığı KentOS'ta yok; tarama alınmadı"
        ));
        return None;
    };
    let angle = prop_num(m, "angle")
        .or_else(|| prop_num(m, "lineangle"))
        .unwrap_or(45.0);
    let Some((k, sym)) = child(layer, "symbol").and_then(|sub| read_symbol(sub, lost, what)) else {
        lost.push(format!("{what}: çizgi deseninin çizgisi alınmadı"));
        return None;
    };
    let line = sym
        .get("layers")
        .and_then(Value::as_array)
        .and_then(|l| l.first())
        .filter(|l| k == "line" && l.get("type").and_then(Value::as_str) == Some("simpleLine"))
        .cloned();
    let Some(line) = line else {
        lost.push(format!(
            "{what}: çizgi deseninin çizgisi basit çizgi değil; tarama alınmadı"
        ));
        return None;
    };
    if sym
        .get("layers")
        .and_then(Value::as_array)
        .is_some_and(|l| l.len() > 1)
    {
        lost.push(format!(
            "{what}: çizgi deseninin çizgisinin yalnız ilk katmanı alındı"
        ));
    }
    let mut h = json!({
        "id": format!("h{i}"), "type": "hatchFill", "angle": angle, "spacing": spacing, "unit": unit,
        "color": line.get("color").cloned().unwrap_or(json!("#000000")), "width": 0.0,
    });
    let same = line.get("unit").and_then(Value::as_str) == Some(unit);
    if same {
        h["width"] = line.get("width").cloned().unwrap_or(json!(0.0));
        if let Some(d) = line.get("dash") {
            h["dash"] = d.clone();
        }
    } else if line
        .get("width")
        .and_then(Value::as_f64)
        .is_some_and(|w| w > 0.0)
    {
        lost.push(format!(
            "{what}: çizgi deseninin çizgisinin birimi aralığınkinden başka; kalınlığı ve kesikleri alınmadı"
        ));
    }
    if let Some(o) = prop_num(m, "offset").filter(|o| *o != 0.0) {
        match size_as(o, prop(m, "offset_unit"), unit) {
            Some(o) => h["offset"] = json!(o),
            None => lost.push(format!(
                "{what}: çizgi deseninin kaydırmasının birimi aralığınkinden başka; alınmadı"
            )),
        }
    }
    Some(h)
}

/// A marker line as KentOS's: its first placement KentOS has, interval, offset along the line, rotation and marker.
fn read_marker_line(
    layer: Node<'_, '_>,
    m: &Map<String, Value>,
    i: usize,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<Value> {
    let names = prop(m, "placements")
        .or_else(|| prop(m, "placement"))
        .unwrap_or("Interval");
    let mut placement = None;
    for n in names
        .split(['|', ','])
        .map(str::trim)
        .filter(|n| !n.is_empty())
    {
        let found = PLACEMENTS
            .iter()
            .find(|(_, q)| q.eq_ignore_ascii_case(n))
            .map(|(k, _)| *k);
        match (found, placement) {
            (Some(k), None) => placement = Some(k),
            _ => lost.push(format!(
                "{what}: işaretli çizginin “{n}” yerleşimi alınmadı (KentOS'ta tek yerleşim)"
            )),
        }
    }
    let Some(placement) = placement else {
        lost.push(format!(
            "{what}: işaretli çizginin yerleşimi KentOS'ta yok; katman alınmadı"
        ));
        return None;
    };
    let Some((interval, unit)) = size_in(
        prop_num(m, "interval").unwrap_or(3.0),
        prop(m, "interval_unit"),
    ) else {
        lost.push(format!(
            "{what}: işaretli çizginin yüzde aralığı KentOS'ta yok; katman alınmadı"
        ));
        return None;
    };
    let Some((k, marker)) = child(layer, "symbol").and_then(|sub| read_symbol(sub, lost, what))
    else {
        lost.push(format!("{what}: işaretli çizginin işareti alınmadı"));
        return None;
    };
    if k != "marker" {
        lost.push(format!("{what}: işaretli çizginin işareti alınmadı"));
        return None;
    }
    let mut l = json!({
        "id": format!("m{i}"), "type": "markerLine", "marker": marker, "placement": placement,
        "interval": interval, "unit": unit, "rotate": prop(m, "rotate") != Some("0"),
    });
    if let Some(o) = prop_num(m, "offset_along_line").filter(|o| *o != 0.0) {
        match size_as(o, prop(m, "offset_along_line_unit"), unit) {
            Some(o) => l["offsetAlong"] = json!(o),
            None => lost.push(format!(
                "{what}: işaretli çizginin baştan uzaklığının birimi aralığınkinden başka; alınmadı"
            )),
        }
    }
    match prop(m, "ring_filter") {
        Some("1") => l["rings"] = json!("exterior"),
        Some("2") => l["rings"] = json!("interior"),
        _ => {}
    }
    if prop_num(m, "offset").is_some_and(|o| o != 0.0) {
        lost.push(format!(
            "{what}: işaretli çizginin paralel kaydırması alınmadı"
        ));
    }
    Some(l)
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
        let names = dd_names(layer);
        let has = |p: &str| names.iter().any(|n| n == p);
        let mut taken: Vec<Value> = Vec::new();
        let mut supported: Vec<DdTake> = Vec::new();
        match (key.as_str(), class) {
            ("fill", "SimpleFill") => {
                supported = vec![
                    ("fillColor", "simpleFill", "color", false),
                    ("outlineColor", "simpleLine", "color", false),
                    ("outlineWidth", "simpleLine", "width", false),
                ];
                if prop(&m, "style").unwrap_or("solid") == "solid" {
                    let c = colour_in(prop(&m, "color"));
                    if c.is_some() || has("fillColor") {
                        let mut f = json!({ "id": format!("f{i}"), "type": "simpleFill" });
                        if let Some(c) = c {
                            f["color"] = json!(c);
                        }
                        taken.push(f);
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
                    if let Some(l) = read_line(&line, format!("o{i}"), has("outlineColor"), lost, what) {
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
                supported = vec![
                    ("outlineWidth", "simpleLine", "width", false),
                    ("outlineColor", "simpleLine", "color", false),
                ];
                if let Some(l) = read_line(&m, format!("l{i}"), has("outlineColor"), lost, what) {
                    taken.push(l);
                }
            }
            ("marker", "SimpleMarker") => {
                supported = vec![
                    ("size", "shape", "size", false),
                    ("angle", "shape", "rotation", true),
                    ("fillColor", "shape", "fill", false),
                    ("outlineColor", "shape", "stroke", false),
                ];
                taken.push(read_marker(&m, lost, what));
            }
            ("marker", "SvgMarker") => {
                supported = vec![
                    ("size", "svg", "size", false),
                    ("angle", "svg", "rotation", true),
                    ("fillColor", "svg", "fill", false),
                    ("outlineColor", "svg", "stroke", false),
                ];
                if let Some(v) = read_svg_marker(&m, lost, what) {
                    taken.push(v);
                }
            }
            ("marker", "FontMarker") => {
                supported = vec![
                    ("size", "text", "size", false),
                    ("angle", "text", "rotation", true),
                    ("fillColor", "text", "color", false),
                    ("char", "text", "text", false),
                ];
                taken.push(read_font_marker(&m, lost, what));
            }
            ("fill", "GradientFill") => {
                let (g, takes) = read_gradient(layer, &m, i, lost, what);
                supported = takes;
                taken.push(g);
            }
            ("fill", "PointPatternFill") => {
                if let Some(v) = read_pattern(layer, &m, i, lost, what) {
                    taken.push(v);
                }
            }
            ("fill", "LinePatternFill") => {
                if let Some(h) = read_hatch(layer, &m, i, lost, what) {
                    taken.push(h);
                }
            }
            ("fill" | "line", "MarkerLine") => {
                if let Some(l) = read_marker_line(layer, &m, i, lost, what) {
                    taken.push(l);
                }
            }
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
            // QGIS's other marker and fill layers (docs/adr/0635 §7).
            ("marker", "EllipseMarker") => {
                supported = vec![
                    ("angle", "shape", "rotation", true),
                    ("fillColor", "shape", "fill", false),
                    ("outlineColor", "shape", "stroke", false),
                ];
                taken.push(read_marker(&ellipse_props(&m, lost, what), lost, what));
            }
            ("marker", "FilledMarker") => {
                supported = vec![
                    ("size", "shape", "size", false),
                    ("angle", "shape", "rotation", true),
                ];
                let pm = filled_props(layer, &m, lost, what);
                taken.push(read_marker(&pm, lost, what));
            }
            ("fill", "ShapeburstFill") => {
                lost.push(format!(
                    "{what}: QGIS'in kenardan yayılan degradesi (ShapeburstFill) KentOS'ta yok; ilk rengiyle düz dolgu alındı"
                ));
                if let Some(c) = colour_in(prop(&m, "color")) {
                    taken.push(json!({ "id": format!("f{i}"), "type": "simpleFill", "color": c }));
                }
            }
            (_, other) => lost.push(format!(
                "{what}: QGIS'in “{other}” simge katmanı KentOS'un katmanına çevrilmedi"
            )),
        }
        take_dd(layer, &supported, &mut taken, lost, what);
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
            Some(f) => match parse_qgis(f)
                .map(|filter| filter_text(&filter))
                .or_else(|| kentos_formats::style_expr::to_kentos(f))
            {
                Some(filter) => v["filter"] = json!(filter),
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
    ASSETS.with(|a| a.borrow_mut().clear());
    let mut read = read_in(text)?;
    read.assets = ASSETS.with(|a| std::mem::take(&mut *a.borrow_mut()));
    Ok(read)
}

/// KentOS's renderer of QGIS's `<renderer-v2>` and the classes and rules taken; `labels` when the QML has labels
/// (a single symbol KentOS cannot draw is then said, not refused).
fn renderer_in(
    r: Node<'_, '_>,
    labels: bool,
    lost: &mut Vec<String>,
) -> Result<(Value, usize), String> {
    if let Some(got) = thematic::read(r, lost) {
        return got;
    }
    let symbols = symbols_of(r, lost);
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
            match sym {
                None if !labels => {
                    return Err("QML'in simgesi KentOS'un çizebileceği bir simge değil.".into());
                }
                // Its labels are taken still (docs/adr/0635 §3).
                None => {
                    lost.push(
                        "QML'in simgesi KentOS'un çizebileceği bir simge değil; görünüş alınmadı".into(),
                    );
                    (Value::Null, 0)
                }
                Some(_) => (json!({ "type": "single", "symbols": set_of(sym) }), 1),
            }
        }
        "categorizedSymbol" => {
            let Some(field) = attr_field(lost) else {
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
            let Some(field) = attr_field(lost) else {
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
                .map(|top| read_rules(top, &symbols, lost, &mut count))
                .unwrap_or_default();
            (json!({ "type": "rules", "rules": list }), count)
        }
        other => {
            return Err(format!(
                "QGIS'in “{other}” görünüşü KentOS'a çevrilmedi (tek simge, kategorili, aralıklı ve kurallı görünüş alınır)."
            ));
        }
    };
    Ok((renderer, rules))
}

fn read_in(text: &str) -> Result<Read, String> {
    let doc = kentos_formats::xml::document_with_dtd(text)
        .map_err(|_| "Dosya okunabilir bir XML değil.".to_owned())?;
    let root = doc.root_element();
    if root.tag_name().name() != "qgis" {
        return Err("Dosya bir QGIS stil dosyası (QML) değil: kök öğe <qgis> değil.".into());
    }
    let mut lost = Vec::new();
    let classes = labels::read(root, &mut lost);
    let Some(r) = child(root, "renderer-v2") else {
        if classes.is_empty() {
            return Err("QML'de QGIS'in görünüşü (renderer-v2) yok.".into());
        }
        let labels = kentos_formats::style_labels::labelling(classes, &mut lost);
        return Ok(Read {
            name: None,
            renderer: Value::Null,
            lost,
            rules: 0,
            assets: Vec::new(),
            labels,
        });
    };
    if child(root, "layerOpacity")
        .and_then(|o| o.text())
        .and_then(|t| t.trim().parse::<f64>().ok())
        .is_some_and(|o| o < 1.0)
    {
        lost.push("katmanın saydamlığı alınmadı (Katman çizimi ayrıdır)".into());
    }
    let (renderer, rules) = renderer_in(r, !classes.is_empty(), &mut lost)?;
    if rules == 0 && classes.is_empty() {
        return Err("QML'de KentOS'un çizebileceği bir sınıf ya da kural yok.".into());
    }
    let labels = kentos_formats::style_labels::labelling(classes, &mut lost);
    Ok(Read {
        name: None,
        renderer: if rules == 0 { Value::Null } else { renderer },
        lost,
        rules,
        assets: Vec::new(),
        labels,
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
        let w = write(&r, &[], area(), &none);
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
        let w = write(&r, &[], k, &none);
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
            { "id": "h", "type": "imageFill", "asset": "doku", "tileSize": 4, "angle": 45 },
            { "id": "f", "type": "simpleFill", "color": "#ffffff" },
        ] } } });
        let w = write(&r, &[], Kinds::ALL, &none);
        assert_eq!(w.rules, 1);
        assert!(
            w.lost.iter().any(|l| l.contains("resim dolgusu")),
            "{:?}",
            w.lost
        );
    }

    #[test]
    fn a_hatch_a_marker_line_and_data_defined_values_go_out_and_come_back() {
        let r = json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": [
            { "id": "h", "type": "hatchFill", "angle": 30, "spacing": 2.5, "width": 0.2, "color": "#336699", "unit": "mm" },
            { "id": "o", "type": "simpleLine", "color": "#000000", "width": { "expr": "[Kat] / 10", "fallback": 0.3 }, "unit": "mm" },
            { "id": "m", "type": "markerLine", "placement": "interval", "interval": 4, "offsetAlong": 1, "unit": "mm",
              "marker": { "type": "marker", "layers": [{ "id": "p", "type": "shape", "shape": "triangle", "size": { "expr": "[Kat] * 2 + 1", "fallback": 2 },
                "rotation": { "expr": "[yön]", "fallback": 0 }, "unit": "mm", "fill": "#ff0000" }] } },
        ] } } });
        let w = write(&r, &[], area(), &none);
        assert!(w.lost.is_empty(), "{:?}", w.lost);
        assert!(w.xml.contains("class=\"LinePatternFill\""), "{}", w.xml);
        assert!(
            w.xml.contains("value=\"&quot;Kat&quot; / 10\""),
            "{}",
            w.xml
        );
        assert!(w.xml.contains("value=\"-(&quot;yön&quot;)\""), "{}", w.xml);
        let back = read(&w.xml).expect("reads");
        let layers = &back.renderer["symbols"]["fill"]["layers"];
        assert_eq!(layers[0]["type"], "hatchFill");
        assert_eq!(layers[0]["angle"], 30.0);
        assert_eq!(layers[0]["spacing"], 2.5);
        assert_eq!(
            layers[1]["width"],
            json!({ "expr": "[Kat] / 10", "fallback": 0.3 })
        );
        assert_eq!(layers[2]["type"], "markerLine");
        assert_eq!(layers[2]["offsetAlong"], 1.0);
        let p = &layers[2]["marker"]["layers"][0];
        assert_eq!(p["size"]["expr"], "[Kat] * 2 + 1");
        assert_eq!(p["rotation"]["expr"], "[yön]");
        assert_eq!(back.lost, Vec::<String>::new());
    }

    #[test]
    fn value_expressions_say_themselves_or_not() {
        assert_eq!(
            qgis_value("-([a] + 2) * b").as_deref(),
            Some("-(\"a\" + 2) * \"b\"")
        );
        assert_eq!(
            kentos_value("\"Ad Soyad\" % 3").as_deref(),
            Some("[Ad Soyad] % 3")
        );
        // With functions, word by word (docs/adr/0635 §6); what neither says stays out.
        assert_eq!(qgis_value("abs([a])").as_deref(), Some("abs(\"a\")"));
        assert_eq!(kentos_value("round(\"a\", 1)").as_deref(), Some("yuvarla([a], 1)"));
        assert_eq!(qgis_value("$alan * 2"), None);
        assert_eq!(kentos_value("regexp_match(\"a\", 'x')"), None);
        assert_eq!(negated(&negated("\"a\" * 2")), "\"a\" * 2");
    }

    fn area_look(layers: Value) -> Value {
        json!({ "type": "single", "symbols": { "fill": { "type": "fill", "layers": layers } } })
    }

    fn point_look(layers: Value) -> Value {
        json!({ "type": "single", "symbols": { "marker": { "type": "marker", "layers": layers } } })
    }

    fn points() -> Kinds {
        Kinds {
            point: true,
            line: false,
            area: false,
        }
    }

    #[test]
    fn gradients_go_out_and_come_back_in_their_three_shapes() {
        for (shape, inverted, angle) in [
            ("linear", false, 0.0),
            ("linear", true, 90.0),
            ("linear", false, 180.0),
            ("cylinder", false, 0.0),
            ("spherical", false, 0.0),
            ("spherical", true, 0.0),
        ] {
            let g = json!({ "id": "g", "type": "gradientFill", "shape": shape, "angle": angle,
                "inverted": inverted, "color": "#ff0000", "color2": "#0000ff" });
            let w = write(&area_look(json!([g.clone()])), &[], area(), &none);
            assert!(w.lost.is_empty(), "{shape}: {:?}", w.lost);
            assert!(w.xml.contains("class=\"GradientFill\""), "{}", w.xml);
            let back = read(&w.xml).expect("reads");
            assert_eq!(back.lost, Vec::<String>::new(), "{shape}");
            let l = &back.renderer["symbols"]["fill"]["layers"][0];
            assert_eq!(l["shape"], shape);
            assert_eq!(l["angle"].as_f64().unwrap_or(0.0), angle);
            // A reflected sphere and a swapped linear gradient come back with the colours the way they ran.
            let (a, b) = (l["color"].as_str().unwrap(), l["color2"].as_str().unwrap());
            if inverted {
                assert_eq!((a, b), ("#0000ff", "#ff0000"), "{shape} inverted");
            } else {
                assert_eq!((a, b), ("#ff0000", "#0000ff"), "{shape}");
            }
        }
    }

    #[test]
    fn a_gradient_at_a_slant_and_a_plain_conic_one_are_said() {
        let g = json!({ "id": "g", "type": "gradientFill", "shape": "linear", "angle": 30, "color": "#ff0000", "color2": "#0000ff" });
        let w = write(&area_look(json!([g])), &[], area(), &none);
        assert_eq!(w.lost.len(), 1, "{:?}", w.lost);
        assert!(w.lost[0].contains("açılı degradenin"));
        let conic = w
            .xml
            .replace("name=\"type\" type=\"QString\" value=\"0\"", "name=\"type\" type=\"QString\" value=\"2\"");
        let back = read(&conic).expect("reads");
        assert!(back.lost.iter().any(|l| l.contains("konik degrade")), "{:?}", back.lost);
        assert_eq!(back.renderer["symbols"]["fill"]["layers"][0]["shape"], "linear");
    }

    #[test]
    fn data_defined_colours_go_out_and_come_back_on_every_layer() {
        let r = area_look(json!([
            { "id": "g", "type": "gradientFill", "shape": "spherical", "color": { "expr": "[Renk]", "fallback": "#ff0000" },
              "color2": { "expr": "[Renk2]" } },
            { "id": "f", "type": "simpleFill", "color": { "expr": "[Dolgu]", "fallback": "#00ff00" } },
            { "id": "o", "type": "simpleLine", "color": { "expr": "[Kenar]", "fallback": "#000000" }, "width": 0.3, "unit": "mm" },
        ]));
        let w = write(&r, &[], area(), &none);
        assert!(w.lost.is_empty(), "{:?}", w.lost);
        // The sphere's first QGIS colour is its centre, KentOS's second.
        assert!(w.xml.contains("name=\"color2\" type=\"Map\""), "{}", w.xml);
        let back = read(&w.xml).expect("reads");
        assert_eq!(back.lost, Vec::<String>::new());
        let l = &back.renderer["symbols"]["fill"]["layers"];
        assert_eq!(l[0]["color"], json!({ "expr": "[Renk]", "fallback": "#ff0000" }));
        assert_eq!(l[0]["color2"], json!({ "expr": "[Renk2]" }));
        assert_eq!(l[1]["color"], json!({ "expr": "[Dolgu]", "fallback": "#00ff00" }));
        assert_eq!(l[2]["color"], json!({ "expr": "[Kenar]", "fallback": "#000000" }));
        // An expression that is not fields and operators is said, its fallback written.
        let text = area_look(json!([
            { "id": "f", "type": "simpleFill", "color": { "expr": "renk_tablosu([a])", "fallback": "#00ff00" } },
        ]));
        let w = write(&text, &[], area(), &none);
        assert_eq!(w.lost.len(), 1, "{:?}", w.lost);
        assert!(w.lost[0].contains("bir ifadeyle"), "{:?}", w.lost);
        let m = point_look(json!([
            { "id": "p", "type": "shape", "shape": "circle", "size": 3, "unit": "mm",
              "fill": { "expr": "[Renk]", "fallback": "#ff0000" }, "stroke": { "expr": "[Çizgi]" } },
        ]));
        let w = write(&m, &[], points(), &none);
        let back = read(&w.xml).expect("reads");
        let p = &back.renderer["symbols"]["marker"]["layers"][0];
        assert_eq!(p["fill"], json!({ "expr": "[Renk]", "fallback": "#ff0000" }));
        assert_eq!(p["stroke"], json!({ "expr": "[Çizgi]" }));
        assert!(back.lost.is_empty(), "{:?}", back.lost);
    }

    #[test]
    fn a_point_pattern_goes_out_and_comes_back() {
        let r = area_look(json!([
            { "id": "p", "type": "patternFill", "unit": "mm", "spacingX": 6, "spacingY": 4, "stagger": true, "angle": 30,
              "offset": [1, 2], "marker": { "type": "marker", "layers": [
                { "id": "m", "type": "shape", "shape": "star", "size": 2, "unit": "mm", "fill": "#228833" }] } },
        ]));
        let w = write(&r, &[], area(), &none);
        assert_eq!(w.lost.len(), 1, "{:?}", w.lost);
        assert!(w.lost[0].contains("dizilimi"), "{:?}", w.lost);
        assert!(w.xml.contains("class=\"PointPatternFill\""), "{}", w.xml);
        // QGIS turns the grid clockwise and counts the offset's y downward.
        assert!(w.xml.contains("name=\"angle\" type=\"QString\" value=\"-30\""), "{}", w.xml);
        assert!(w.xml.contains("name=\"offset_y\" type=\"QString\" value=\"-2\""), "{}", w.xml);
        assert!(w.xml.contains("name=\"displacement_x\" type=\"QString\" value=\"3\""), "{}", w.xml);
        let back = read(&w.xml).expect("reads");
        assert_eq!(back.lost, Vec::<String>::new());
        let l = &back.renderer["symbols"]["fill"]["layers"][0];
        assert_eq!(l["type"], "patternFill");
        assert_eq!((l["spacingX"].as_f64(), l["spacingY"].as_f64()), (Some(6.0), Some(4.0)));
        assert_eq!(l["stagger"], true);
        assert_eq!(l["angle"], 30.0);
        assert_eq!(l["offset"], json!([1.0, 2.0]));
        assert_eq!(l["marker"]["layers"][0]["shape"], "star");
        // What KentOS's pattern has and QGIS's has not is said.
        let more = area_look(json!([
            { "id": "p", "type": "patternFill", "unit": "mm", "spacingX": 6, "spacingY": 4, "jitter": 0.3, "coverage": 0.5,
              "marker": { "type": "marker", "layers": [{ "id": "m", "type": "shape", "shape": "circle", "size": 2 }] } },
        ]));
        assert!(write(&more, &[], area(), &none).lost.iter().any(|l| l.contains("rastgele sapması")));
    }

    #[test]
    fn an_svg_marker_carries_its_drawing_inside_the_qml() {
        let svg = "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 4 4\"><circle cx=\"2\" cy=\"2\" r=\"2\"/></svg>";
        let r = point_look(json!([
            { "id": "v", "type": "svg", "asset": "agac", "svgData": svg, "size": 4, "unit": "mm", "rotation": 30,
              "fill": "#112233", "stroke": { "expr": "[Çizgi]", "fallback": "#445566" } },
        ]));
        let w = write(&r, &[], points(), &none);
        assert!(w.lost.is_empty(), "{:?}", w.lost);
        assert!(w.xml.contains("class=\"SvgMarker\""), "{}", w.xml);
        assert!(w.xml.contains("value=\"base64:"), "{}", w.xml);
        let back = read(&w.xml).expect("reads");
        assert_eq!(back.lost, Vec::<String>::new());
        assert_eq!(back.assets.len(), 1);
        assert_eq!(back.assets[0].svg, svg);
        let v = &back.renderer["symbols"]["marker"]["layers"][0];
        assert_eq!(v["type"], "svg");
        assert_eq!(v["asset"], back.assets[0].id);
        assert_eq!(v["rotation"], 30.0);
        assert_eq!(v["fill"], "#112233");
        assert_eq!(v["stroke"], json!({ "expr": "[Çizgi]", "fallback": "#445566" }));
        // A marker whose drawing the library lacks, and one that names a file, are said.
        let none_there = point_look(json!([{ "id": "v", "type": "svg", "asset": "yok", "size": 4 }]));
        assert!(write(&none_there, &[], points(), &none).lost.iter().any(|l| l.contains("çizimi kitaplıkta yok")));
        let file = w.xml.replace(&format!("base64:{}", kentos_formats::style_extras::b64_encode(svg.as_bytes())), "/usr/share/agac.svg");
        let back = read(&file).expect_err("nothing is left to read");
        assert!(back.contains("çizebileceği"), "{back}");
    }

    #[test]
    fn a_text_marker_is_a_font_marker() {
        let r = point_look(json!([
            { "id": "t", "type": "text", "text": "★", "font": "Arial", "weight": 700, "italic": true, "size": 6, "unit": "mm",
              "rotation": 15, "color": { "expr": "[Renk]", "fallback": "#aa0000" }, "halo": { "color": "#ffffff", "width": 0.4 } },
        ]));
        let w = write(&r, &[], points(), &none);
        assert!(w.lost.is_empty(), "{:?}", w.lost);
        assert!(w.xml.contains("class=\"FontMarker\""), "{}", w.xml);
        assert!(w.xml.contains("name=\"font_style\" type=\"QString\" value=\"Bold Italic\""), "{}", w.xml);
        let back = read(&w.xml).expect("reads");
        assert_eq!(back.lost, Vec::<String>::new());
        let t = &back.renderer["symbols"]["marker"]["layers"][0];
        assert_eq!(t["text"], "★");
        assert_eq!((t["font"].as_str(), t["weight"].as_f64(), t["italic"].as_bool()), (Some("Arial"), Some(700.0), Some(true)));
        assert_eq!(t["rotation"], 15.0);
        assert_eq!(t["color"], json!({ "expr": "[Renk]", "fallback": "#aa0000" }));
        assert_eq!(t["halo"], json!({ "color": "#ffffff", "width": 0.4 }));
        // QGIS's marker is one character, and its character may come from a field.
        let long = point_look(json!([{ "id": "t", "type": "text", "text": { "expr": "[Kod]", "fallback": "AB" }, "size": 3 }]));
        let w = write(&long, &[], points(), &none);
        assert_eq!(w.lost.len(), 1, "{:?}", w.lost);
        assert!(w.lost[0].contains("yalnız ilk karakter"), "{:?}", w.lost);
        let back = read(&w.xml).expect("reads");
        assert_eq!(back.renderer["symbols"]["marker"]["layers"][0]["text"], json!({ "expr": "[Kod]", "fallback": "A" }));
    }
}

#[cfg(test)]
mod exchange_tests {
    #[test]
    fn qml_does_what_the_profile_says() {
        let wrong = kentos_formats::style_exchange::check("qml", &|renderer, kinds| {
            let w = super::write(renderer, &[], kinds, &|_| None);
            let back = super::read(&w.xml)
                .ok()
                .map(|r| r.renderer)
                .filter(|r| !r.is_null());
            (w.lost, back)
        });
        assert!(wrong.is_empty(), "{wrong:#?}");
    }
}
