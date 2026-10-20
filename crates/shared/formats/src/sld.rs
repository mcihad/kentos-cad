//! A layer's style as OGC SLD 1.1 with Symbology Encoding 1.1 (docs/adr/0516), the subset QGIS 4.2, GeoServer and
//! MapServer share: rules with OGC filters and scale ranges, polygon, line and point symbolizers with fills, strokes,
//! dashes and well-known marks. What KentOS's style has and SE has not (a hatch, a gradient, a pattern, a marker line,
//! an SVG or text marker, a data-defined size, an expression the filter grammar cannot say) is named, never dropped
//! in silence; read back, what SE has and KentOS's style has not (an external graphic, a label, a function) is named
//! too.
//!
//! The style is the app's JSON (`model/style.ts`): a renderer (`single`, `categorized`, `graduated`, `rules`) whose
//! symbol sets hold marker, line and fill symbols. Sizes in paper millimetres go out in pixels of SE's standard
//! rendering pixel, 0.28 mm, as SE 1.1 §10.2 asks and QGIS writes them; metres with SE's metre unit; screen pixels as
//! pixels. Read back, a pixel stays a screen pixel and a metre a metre, as QGIS reads them. Fields are
//! written as `[Ad]` in KentOS's expressions and as `<ogc:PropertyName>Ad</ogc:PropertyName>` in a filter.

use std::fmt::Write as _;

use serde_json::{Map, Value, json};

/// SE 1.1's standard rendering pixel, mm (§10.2): a size in paper mm is this many pixels.
pub const PIXEL_MM: f64 = 0.28;

const METRE_UOM: &str = "http://www.opengeospatial.org/se/units/metre";
const PIXEL_UOM: &str = "http://www.opengeospatial.org/se/units/pixel";

/// Which geometry the style's symbolizers are for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Kinds {
    pub point: bool,
    pub line: bool,
    pub area: bool,
}

impl Kinds {
    pub const ALL: Kinds = Kinds {
        point: true,
        line: true,
        area: true,
    };
}

/// What the writer made: the document, and what it could not say.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Written {
    pub xml: String,
    pub lost: Vec<String>,
    /// Rules written.
    pub rules: usize,
}

/// What the reader made: KentOS's renderer, the layer's name, and what it could not take.
#[derive(Clone, Debug, PartialEq)]
pub struct Read {
    pub name: Option<String>,
    pub renderer: Value,
    pub lost: Vec<String>,
    pub rules: usize,
}

// ─── Expressions ──────────────────────────────────────────────────────────────────────────────────────

/// A filter as the writer and the reader share it.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Filter {
    Cmp(Op, String, Lit),
    Between(String, f64, f64),
    Null(String),
    Like(String, String),
    And(Vec<Filter>),
    Or(Vec<Filter>),
    Not(Box<Filter>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Op {
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
}

impl Op {
    fn ogc(self) -> &'static str {
        match self {
            Op::Eq => "PropertyIsEqualTo",
            Op::Ne => "PropertyIsNotEqualTo",
            Op::Lt => "PropertyIsLessThan",
            Op::Le => "PropertyIsLessThanOrEqualTo",
            Op::Gt => "PropertyIsGreaterThan",
            Op::Ge => "PropertyIsGreaterThanOrEqualTo",
        }
    }

    fn of_ogc(name: &str) -> Option<Op> {
        Some(match name {
            "PropertyIsEqualTo" => Op::Eq,
            "PropertyIsNotEqualTo" => Op::Ne,
            "PropertyIsLessThan" => Op::Lt,
            "PropertyIsLessThanOrEqualTo" => Op::Le,
            "PropertyIsGreaterThan" => Op::Gt,
            "PropertyIsGreaterThanOrEqualTo" => Op::Ge,
            _ => return None,
        })
    }

    fn text(self) -> &'static str {
        match self {
            Op::Eq => "=",
            Op::Ne => "<>",
            Op::Lt => "<",
            Op::Le => "<=",
            Op::Gt => ">",
            Op::Ge => ">=",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Lit {
    Num(f64),
    Text(String),
}

/// The words KentOS's expressions keep for themselves, which no bare field takes.
const KEYWORDS: [&str; 17] = [
    "ve",
    "veya",
    "değil",
    "boş",
    "gibi",
    "arasında",
    "and",
    "or",
    "not",
    "is",
    "null",
    "like",
    "between",
    "doğru",
    "yanlış",
    "true",
    "false",
];

/// Whether `w` is a bare field name of KentOS's expressions: a word of letters, digits and `_`, not a keyword.
fn bare_field(w: &str) -> bool {
    let mut chars = w.chars();
    chars.next().is_some_and(|c| c.is_alphabetic() || c == '_')
        && w.chars().all(|c| c.is_alphanumeric() || c == '_')
        && !KEYWORDS.contains(&w.to_lowercase().as_str())
}

/// A field alone: `[Ad Soyad]` or a bare word `Ad`.
pub(crate) fn field_of(expr: &str) -> Option<String> {
    let e = expr.trim();
    if let Some(inner) = e.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
        let inner = inner.trim();
        return (!inner.is_empty() && !inner.contains([']', '['])).then(|| inner.to_owned());
    }
    bare_field(e).then(|| e.to_owned())
}

/// The tokens of the expressions the filter grammar can say.
#[derive(Clone, Debug, PartialEq)]
enum Tok {
    Field(String),
    Num(f64),
    Text(String),
    Op(Op),
    Word(String),
    Open,
    Close,
    Comma,
}

fn tokens(src: &str, qgis: bool) -> Option<Vec<Tok>> {
    let c: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < c.len() {
        let ch = c[i];
        if ch.is_whitespace() {
            i += 1;
        } else if ch == '[' {
            let j = c[i + 1..].iter().position(|&x| x == ']')? + i + 1;
            out.push(Tok::Field(
                c[i + 1..j].iter().collect::<String>().trim().to_owned(),
            ));
            i = j + 1;
        } else if ch == '\'' || ch == '"' {
            // A text in either quote, the quote doubled inside it (as KentOS's lexer reads it); in QGIS's
            // expressions a double-quoted word is a field.
            let q = ch;
            let mut s = String::new();
            let mut j = i + 1;
            loop {
                let x = *c.get(j)?;
                if x == q {
                    if c.get(j + 1) == Some(&q) {
                        s.push(q);
                        j += 2;
                        continue;
                    }
                    break;
                }
                s.push(x);
                j += 1;
            }
            out.push(if qgis && q == '"' {
                Tok::Field(s)
            } else {
                Tok::Text(s)
            });
            i = j + 1;
        } else if ch.is_ascii_digit()
            || (ch == '-' && c.get(i + 1).is_some_and(char::is_ascii_digit))
            || (ch == '.' && c.get(i + 1).is_some_and(char::is_ascii_digit))
        {
            let mut j = i + 1;
            while j < c.len()
                && (c[j].is_ascii_digit() || c[j] == '.' || c[j] == 'e' || c[j] == 'E')
            {
                j += 1;
            }
            out.push(Tok::Num(c[i..j].iter().collect::<String>().parse().ok()?));
            i = j;
        } else if ch == '(' {
            out.push(Tok::Open);
            i += 1;
        } else if ch == ')' {
            out.push(Tok::Close);
            i += 1;
        } else if ch == ',' {
            out.push(Tok::Comma);
            i += 1;
        } else if "<>=!".contains(ch) {
            let two: String = c[i..(i + 2).min(c.len())].iter().collect();
            let (op, n) = match two.as_str() {
                "<=" => (Op::Le, 2),
                ">=" => (Op::Ge, 2),
                "<>" | "!=" => (Op::Ne, 2),
                "==" => (Op::Eq, 2),
                _ => match ch {
                    '<' => (Op::Lt, 1),
                    '>' => (Op::Gt, 1),
                    '=' => (Op::Eq, 1),
                    _ => return None,
                },
            };
            out.push(Tok::Op(op));
            i += n;
        } else if ch.is_alphabetic() || ch == '_' {
            let mut j = i + 1;
            while j < c.len() && (c[j].is_alphanumeric() || c[j] == '_') {
                j += 1;
            }
            let w: String = c[i..j].iter().collect();
            // QGIS's `IN`, which is no word of KentOS's.
            if qgis && w.eq_ignore_ascii_case("in") {
                out.push(Tok::Word("in".into()));
            } else if bare_field(&w) {
                out.push(Tok::Field(w));
            } else {
                out.push(Tok::Word(w.to_lowercase()));
            }
            i = j;
        } else {
            return None;
        }
    }
    Some(out)
}

/// The expression as a filter, when the grammar can say it: comparisons of a field with a number or a text,
/// `arasında`/`between`, `boş`/`is null`, `gibi`/`like`, joined by `ve`/`and`, `veya`/`or`, `değil`/`not` and
/// parentheses.
pub(crate) fn parse_filter(src: &str) -> Option<Filter> {
    let toks = tokens(src, false)?;
    let mut at = 0;
    let f = or_of(&toks, &mut at)?;
    (at == toks.len()).then_some(f)
}

/// A QGIS expression as a filter, when the grammar can say it: `"field"` and bare words are fields, `'text'`
/// texts, `AND`, `OR`, `NOT`, `IS NULL`, `LIKE`, `BETWEEN … AND …` and `IN (…)` (one comparison each, joined by OR).
pub(crate) fn parse_qgis(src: &str) -> Option<Filter> {
    let toks = tokens(src, true)?;
    let mut at = 0;
    let f = or_of(&toks, &mut at)?;
    (at == toks.len()).then_some(f)
}

fn word(t: Option<&Tok>, words: &[&str]) -> bool {
    matches!(t, Some(Tok::Word(w)) if words.contains(&w.as_str()))
}

fn or_of(t: &[Tok], at: &mut usize) -> Option<Filter> {
    let mut parts = vec![and_of(t, at)?];
    while word(t.get(*at), &["veya", "or"]) {
        *at += 1;
        parts.push(and_of(t, at)?);
    }
    Some(if parts.len() == 1 {
        parts.remove(0)
    } else {
        Filter::Or(parts)
    })
}

fn and_of(t: &[Tok], at: &mut usize) -> Option<Filter> {
    let mut parts = vec![not_of(t, at)?];
    while word(t.get(*at), &["ve", "and"]) {
        *at += 1;
        parts.push(not_of(t, at)?);
    }
    Some(if parts.len() == 1 {
        parts.remove(0)
    } else {
        Filter::And(parts)
    })
}

fn not_of(t: &[Tok], at: &mut usize) -> Option<Filter> {
    if word(t.get(*at), &["değil", "not"]) {
        *at += 1;
        return Some(Filter::Not(Box::new(not_of(t, at)?)));
    }
    if t.get(*at) == Some(&Tok::Open) {
        *at += 1;
        let f = or_of(t, at)?;
        if t.get(*at) != Some(&Tok::Close) {
            return None;
        }
        *at += 1;
        return Some(f);
    }
    let Some(Tok::Field(name)) = t.get(*at) else {
        return None;
    };
    let name = name.clone();
    *at += 1;
    match t.get(*at) {
        Some(Tok::Op(op)) => {
            let op = *op;
            *at += 1;
            let lit = match t.get(*at)? {
                Tok::Num(n) => Lit::Num(*n),
                Tok::Text(s) => Lit::Text(s.clone()),
                _ => return None,
            };
            *at += 1;
            Some(Filter::Cmp(op, name, lit))
        }
        Some(Tok::Word(w)) if w == "boş" || w == "is" => {
            *at += 1;
            if w == "is" {
                if word(t.get(*at), &["null"]) {
                    *at += 1;
                    return Some(Filter::Null(name));
                }
                if word(t.get(*at), &["not"]) && word(t.get(*at + 1), &["null"]) {
                    *at += 2;
                    return Some(Filter::Not(Box::new(Filter::Null(name))));
                }
                return None;
            }
            if word(t.get(*at), &["değil"]) {
                *at += 1;
                return Some(Filter::Not(Box::new(Filter::Null(name))));
            }
            Some(Filter::Null(name))
        }
        Some(Tok::Word(w)) if w == "gibi" || w == "like" => {
            *at += 1;
            let Some(Tok::Text(p)) = t.get(*at) else {
                return None;
            };
            *at += 1;
            Some(Filter::Like(name, p.clone()))
        }
        Some(Tok::Word(w)) if w == "in" => {
            *at += 1;
            if t.get(*at) != Some(&Tok::Open) {
                return None;
            }
            *at += 1;
            let mut parts = Vec::new();
            loop {
                let lit = match t.get(*at)? {
                    Tok::Num(n) => Lit::Num(*n),
                    Tok::Text(s) => Lit::Text(s.clone()),
                    _ => return None,
                };
                parts.push(Filter::Cmp(Op::Eq, name.clone(), lit));
                *at += 1;
                match t.get(*at)? {
                    Tok::Comma => *at += 1,
                    Tok::Close => {
                        *at += 1;
                        break;
                    }
                    _ => return None,
                }
            }
            Some(if parts.len() == 1 {
                parts.remove(0)
            } else {
                Filter::Or(parts)
            })
        }
        Some(Tok::Word(w)) if w == "arasında" || w == "between" => {
            *at += 1;
            let Some(Tok::Num(a)) = t.get(*at) else {
                return None;
            };
            if !word(t.get(*at + 1), &["ve", "and"]) {
                return None;
            }
            let Some(Tok::Num(b)) = t.get(*at + 2) else {
                return None;
            };
            *at += 3;
            Some(Filter::Between(name, *a, *b))
        }
        _ => None,
    }
}

pub(crate) fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A number as the shortest text that reads back to it.
pub(crate) fn num(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn lit_xml(l: &Lit) -> String {
    match l {
        Lit::Num(n) => format!("<ogc:Literal>{}</ogc:Literal>", num(*n)),
        Lit::Text(s) => format!("<ogc:Literal>{}</ogc:Literal>", esc(s)),
    }
}

fn filter_xml(f: &Filter, out: &mut String, pad: &str) {
    match f {
        Filter::Cmp(op, field, lit) => {
            let _ = writeln!(
                out,
                "{pad}<ogc:{0}><ogc:PropertyName>{1}</ogc:PropertyName>{2}</ogc:{0}>",
                op.ogc(),
                esc(field),
                lit_xml(lit)
            );
        }
        Filter::Between(field, a, b) => {
            let _ = writeln!(
                out,
                "{pad}<ogc:PropertyIsBetween><ogc:PropertyName>{}</ogc:PropertyName><ogc:LowerBoundary><ogc:Literal>{}</ogc:Literal></ogc:LowerBoundary><ogc:UpperBoundary><ogc:Literal>{}</ogc:Literal></ogc:UpperBoundary></ogc:PropertyIsBetween>",
                esc(field),
                num(*a),
                num(*b)
            );
        }
        Filter::Null(field) => {
            let _ = writeln!(
                out,
                "{pad}<ogc:PropertyIsNull><ogc:PropertyName>{}</ogc:PropertyName></ogc:PropertyIsNull>",
                esc(field)
            );
        }
        Filter::Like(field, pattern) => {
            // KentOS's `gibi` takes % and _ as SQL does; the filter names them.
            let _ = writeln!(
                out,
                "{pad}<ogc:PropertyIsLike wildCard=\"%\" singleChar=\"_\" escapeChar=\"\\\"><ogc:PropertyName>{}</ogc:PropertyName><ogc:Literal>{}</ogc:Literal></ogc:PropertyIsLike>",
                esc(field),
                esc(pattern)
            );
        }
        Filter::And(parts) | Filter::Or(parts) => {
            let tag = if matches!(f, Filter::And(_)) {
                "And"
            } else {
                "Or"
            };
            let _ = writeln!(out, "{pad}<ogc:{tag}>");
            for p in parts {
                filter_xml(p, out, &format!("{pad}  "));
            }
            let _ = writeln!(out, "{pad}</ogc:{tag}>");
        }
        Filter::Not(inner) => {
            let _ = writeln!(out, "{pad}<ogc:Not>");
            filter_xml(inner, out, &format!("{pad}  "));
            let _ = writeln!(out, "{pad}</ogc:Not>");
        }
    }
}

/// A filter as KentOS's expression.
pub(crate) fn filter_text(f: &Filter) -> String {
    let lit = |l: &Lit| match l {
        Lit::Num(n) => num(*n),
        Lit::Text(s) => format!("'{}'", s.replace('\'', "''")),
    };
    match f {
        Filter::Cmp(op, field, l) => format!("[{field}] {} {}", op.text(), lit(l)),
        Filter::Between(field, a, b) => format!("[{field}] arasında {} ve {}", num(*a), num(*b)),
        Filter::Null(field) => format!("[{field}] boş"),
        Filter::Like(field, p) => format!("[{field}] gibi '{}'", p.replace('\'', "''")),
        Filter::And(parts) => parts
            .iter()
            .map(|p| wrap(p, true))
            .collect::<Vec<_>>()
            .join(" ve "),
        Filter::Or(parts) => parts
            .iter()
            .map(|p| wrap(p, false))
            .collect::<Vec<_>>()
            .join(" veya "),
        Filter::Not(inner) => match &**inner {
            Filter::Null(field) => format!("[{field}] boş değil"),
            other => format!("değil ({})", filter_text(other)),
        },
    }
}

fn wrap(f: &Filter, in_and: bool) -> String {
    match f {
        Filter::Or(_) if in_and => format!("({})", filter_text(f)),
        Filter::And(_) if !in_and => format!("({})", filter_text(f)),
        _ => filter_text(f),
    }
}

// ─── Writing ──────────────────────────────────────────────────────────────────────────────────────────

/// A colour and its opacity: `#rrggbb`, `#rrggbbaa`, `#rgb`; other forms (a theme token) are none.
pub(crate) fn colour(v: &Value) -> Option<(String, f64)> {
    let s = v.as_str()?.trim();
    let h = s.strip_prefix('#')?;
    if !h.chars().all(|c| c.is_ascii_hexdigit()) {
        return None;
    }
    let full = match h.len() {
        3 => h.chars().flat_map(|c| [c, c]).collect::<String>(),
        4 => h.chars().flat_map(|c| [c, c]).collect::<String>(),
        6 | 8 => h.to_owned(),
        _ => return None,
    };
    let rgb = full[..6].to_ascii_lowercase();
    let alpha = if full.len() == 8 {
        f64::from(u8::from_str_radix(&full[6..8], 16).ok()?) / 255.0
    } else {
        1.0
    };
    Some((format!("#{rgb}"), alpha))
}

/// A size in a layer's unit as SE says it: the value and its unit of measure.
fn size_of(v: f64, unit: &str) -> (f64, &'static str) {
    match unit {
        "m" => (v, METRE_UOM),
        "px" => (v, PIXEL_UOM),
        _ => (v / PIXEL_MM, PIXEL_UOM),
    }
}

pub(crate) fn round3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

/// A fixed value of a symbol layer's property, or why not.
pub(crate) fn fixed<'a>(
    layer: &'a Value,
    key: &str,
    lost: &mut Vec<String>,
    what: &str,
) -> Option<&'a Value> {
    let v = layer.get(key)?;
    if v.is_object() && v.get("expr").is_some() {
        let fallback = v.get("fallback").filter(|f| !f.is_null());
        lost.push(match fallback {
            Some(_) => {
                format!("{what}: “{key}” bir ifadeyle (veriye göre); SLD'ye yedek değeri yazıldı")
            }
            None => format!("{what}: “{key}” bir ifadeyle (veriye göre); SLD'ye yazılamadı"),
        });
        return fallback;
    }
    (!v.is_null()).then_some(v)
}

struct Sym {
    xml: String,
    uom: Option<&'static str>,
}

fn stroke_xml(layer: &Value, lost: &mut Vec<String>, what: &str) -> Option<(String, &'static str)> {
    let unit = layer.get("unit").and_then(Value::as_str).unwrap_or("mm");
    let (rgb, alpha) = fixed(layer, "color", lost, what)
        .and_then(colour)
        .or_else(|| {
            lost.push(format!("{what}: çizginin rengi SLD'nin renklerinden değil"));
            None
        })?;
    let width = fixed(layer, "width", lost, what)
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let (w, uom) = size_of(width, unit);
    let mut s = String::from("<se:Stroke>");
    let _ = write!(
        s,
        "<se:SvgParameter name=\"stroke\">{rgb}</se:SvgParameter>"
    );
    if alpha < 1.0 {
        let _ = write!(
            s,
            "<se:SvgParameter name=\"stroke-opacity\">{}</se:SvgParameter>",
            round3(alpha)
        );
    }
    // A hairline (0) is one pixel, as KentOS draws it.
    let w = if width <= 0.0 { 1.0 } else { round3(w) };
    let uom = if width <= 0.0 { PIXEL_UOM } else { uom };
    let _ = write!(
        s,
        "<se:SvgParameter name=\"stroke-width\">{}</se:SvgParameter>",
        num(w)
    );
    if let Some(join) = layer.get("join").and_then(Value::as_str) {
        let _ = write!(
            s,
            "<se:SvgParameter name=\"stroke-linejoin\">{join}</se:SvgParameter>"
        );
    }
    if let Some(cap) = layer.get("cap").and_then(Value::as_str) {
        let _ = write!(
            s,
            "<se:SvgParameter name=\"stroke-linecap\">{cap}</se:SvgParameter>"
        );
    }
    if let Some(dash) = layer
        .get("dash")
        .and_then(Value::as_array)
        .filter(|d| !d.is_empty())
    {
        let parts: Vec<String> = dash
            .iter()
            .filter_map(Value::as_f64)
            .map(|d| num(round3(size_of(d, unit).0)))
            .collect();
        let _ = write!(
            s,
            "<se:SvgParameter name=\"stroke-dasharray\">{}</se:SvgParameter>",
            parts.join(" ")
        );
    }
    s.push_str("</se:Stroke>");
    for (key, words) in [
        ("offset", "paralel kaydırması"),
        ("wave", "dalgası"),
        ("blur", "yumuşak kenarı"),
        ("shift", "gölge kaydırması"),
    ] {
        if layer
            .get(key)
            .is_some_and(|v| !v.is_null() && v != &json!(0))
        {
            lost.push(format!("{what}: çizginin {words} SLD'ye yazılmadı"));
        }
    }
    Some((s, uom))
}

const MARKS: [(&str, &str); 7] = [
    ("circle", "circle"),
    ("ring", "circle"),
    ("square", "square"),
    ("triangle", "triangle"),
    ("star", "star"),
    ("cross", "cross"),
    ("x", "x"),
];

fn marker_xml(layer: &Value, lost: &mut Vec<String>, what: &str) -> Option<Sym> {
    let t = layer.get("type").and_then(Value::as_str).unwrap_or("");
    if t != "shape" {
        lost.push(format!(
            "{what}: “{t}” işareti SLD'nin işaretlerinden değil (yalnız şekil işaretleri yazılır)"
        ));
        return None;
    }
    let shape = layer
        .get("shape")
        .and_then(Value::as_str)
        .unwrap_or("circle");
    let Some(&(_, wkn)) = MARKS.iter().find(|(k, _)| *k == shape) else {
        lost.push(format!(
            "{what}: “{shape}” şekli SLD'nin iyi bilinen işaretlerinden değil"
        ));
        return None;
    };
    let unit = layer.get("unit").and_then(Value::as_str).unwrap_or("mm");
    let size = fixed(layer, "size", lost, what)
        .and_then(Value::as_f64)
        .unwrap_or(2.0);
    let (size, uom) = size_of(size, unit);
    let mut s = String::from("<se:Graphic><se:Mark>");
    let _ = write!(s, "<se:WellKnownName>{wkn}</se:WellKnownName>");
    if let Some((rgb, a)) = fixed(layer, "fill", lost, what).and_then(colour) {
        let _ = write!(
            s,
            "<se:Fill><se:SvgParameter name=\"fill\">{rgb}</se:SvgParameter>"
        );
        if a < 1.0 {
            let _ = write!(
                s,
                "<se:SvgParameter name=\"fill-opacity\">{}</se:SvgParameter>",
                round3(a)
            );
        }
        s.push_str("</se:Fill>");
    }
    if let Some((rgb, a)) = fixed(layer, "stroke", lost, what).and_then(colour) {
        let w = layer
            .get("strokeWidth")
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        let w = if w <= 0.0 {
            1.0
        } else {
            round3(size_of(w, unit).0)
        };
        let _ = write!(
            s,
            "<se:Stroke><se:SvgParameter name=\"stroke\">{rgb}</se:SvgParameter>"
        );
        if a < 1.0 {
            let _ = write!(
                s,
                "<se:SvgParameter name=\"stroke-opacity\">{}</se:SvgParameter>",
                round3(a)
            );
        }
        let _ = write!(
            s,
            "<se:SvgParameter name=\"stroke-width\">{}</se:SvgParameter></se:Stroke>",
            num(w)
        );
    }
    s.push_str("</se:Mark>");
    let _ = write!(s, "<se:Size>{}</se:Size>", num(round3(size)));
    if let Some(r) = fixed(layer, "rotation", lost, what)
        .and_then(Value::as_f64)
        .filter(|r| *r != 0.0)
    {
        // KentOS turns counter-clockwise, SE clockwise.
        let _ = write!(
            s,
            "<se:Rotation><ogc:Literal>{}</ogc:Literal></se:Rotation>",
            num(round3(-r))
        );
    }
    s.push_str("</se:Graphic>");
    if shape != "ring"
        && layer
            .get("hole")
            .is_some_and(|v| v.as_f64().is_some_and(|h| h > 0.0))
    {
        lost.push(format!("{what}: işaretin deliği SLD'ye yazılmadı"));
    }
    Some(Sym {
        xml: s,
        uom: Some(uom),
    })
}

fn opacity_of(symbol: &Value) -> Option<f64> {
    symbol
        .get("opacity")
        .and_then(Value::as_f64)
        .filter(|o| *o < 1.0)
}

/// A symbol's symbolizers.
fn symbolizers(symbol: &Value, kind: &str, lost: &mut Vec<String>, what: &str) -> Vec<String> {
    let mut out = Vec::new();
    if opacity_of(symbol).is_some() {
        lost.push(format!(
            "{what}: simgenin saydamlığı SLD'ye yazılmadı (renklerin saydamlığı yazılır)"
        ));
    }
    let layers = symbol
        .get("layers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut pending_fill: Option<String> = None;
    for layer in &layers {
        if layer.get("enabled") == Some(&Value::Bool(false)) {
            continue;
        }
        let t = layer.get("type").and_then(Value::as_str).unwrap_or("");
        let uom = |u: Option<&str>| u.map_or(String::new(), |u| format!(" uom=\"{u}\""));
        match (kind, t) {
            ("fill", "simpleFill") => {
                if let Some(f) = pending_fill.take() {
                    out.push(format!("<se:PolygonSymbolizer>{f}</se:PolygonSymbolizer>"));
                }
                let Some((rgb, a)) = fixed(layer, "color", lost, what).and_then(colour) else {
                    lost.push(format!("{what}: dolgunun rengi SLD'nin renklerinden değil"));
                    continue;
                };
                let mut f =
                    format!("<se:Fill><se:SvgParameter name=\"fill\">{rgb}</se:SvgParameter>");
                if a < 1.0 {
                    let _ = write!(
                        f,
                        "<se:SvgParameter name=\"fill-opacity\">{}</se:SvgParameter>",
                        round3(a)
                    );
                }
                f.push_str("</se:Fill>");
                pending_fill = Some(f);
            }
            ("fill", "simpleLine") => {
                let Some((s, u)) = stroke_xml(layer, lost, what) else {
                    continue;
                };
                if layer
                    .get("rings")
                    .and_then(Value::as_str)
                    .is_some_and(|r| r != "all")
                {
                    lost.push(format!(
                        "{what}: kenar çizgisinin halka seçimi SLD'ye yazılmadı"
                    ));
                }
                // The outline joins the fill before it; SE draws a polygon's fill and stroke as one symbolizer.
                let fill = pending_fill.take().unwrap_or_default();
                out.push(format!(
                    "<se:PolygonSymbolizer{}>{fill}{s}</se:PolygonSymbolizer>",
                    uom(Some(u))
                ));
            }
            ("line", "simpleLine") => {
                if let Some((s, u)) = stroke_xml(layer, lost, what) {
                    out.push(format!(
                        "<se:LineSymbolizer{}>{s}</se:LineSymbolizer>",
                        uom(Some(u))
                    ));
                }
            }
            ("marker", _) => {
                if let Some(m) = marker_xml(layer, lost, what) {
                    out.push(format!(
                        "<se:PointSymbolizer{}>{}</se:PointSymbolizer>",
                        uom(m.uom),
                        m.xml
                    ));
                }
            }
            (_, "centroidMarker") => {
                if let Some(m) = layer
                    .get("marker")
                    .and_then(|m| m.get("layers"))
                    .and_then(Value::as_array)
                    .and_then(|l| l.first())
                    && let Some(m) = marker_xml(m, lost, what)
                {
                    {
                        out.push(format!(
                            "<se:PointSymbolizer{}><se:Geometry><ogc:Function name=\"centroid\"><ogc:PropertyName>geometry</ogc:PropertyName></ogc:Function></se:Geometry>{}</se:PointSymbolizer>",
                            uom(m.uom),
                            m.xml
                        ));
                    }
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
                    _ => "katman",
                };
                lost.push(format!("{what}: {words} SLD'ye yazılmadı"));
            }
        }
    }
    if let Some(f) = pending_fill {
        out.push(format!("<se:PolygonSymbolizer>{f}</se:PolygonSymbolizer>"));
    }
    if symbol
        .get("effects")
        .and_then(Value::as_array)
        .is_some_and(|e| !e.is_empty())
    {
        lost.push(format!("{what}: simgenin çizim efektleri SLD'ye yazılmadı"));
    }
    out
}

/// A symbol set's symbolizers for the kinds asked, its library references through `library`.
fn set_symbolizers(
    set: Option<&Value>,
    kinds: Kinds,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
    what: &str,
) -> Vec<String> {
    let Some(set) = set else {
        return Vec::new();
    };
    let mut out = Vec::new();
    for (key, on) in [
        ("fill", kinds.area),
        ("line", kinds.line),
        ("marker", kinds.point),
    ] {
        if !on {
            continue;
        }
        let Some(sym) = set.get(key) else {
            continue;
        };
        let sym = match sym.get("ref").and_then(Value::as_str) {
            Some(id) => match library(id) {
                Some(s) => s,
                None => {
                    lost.push(format!(
                        "{what}: kitaplıkta olmayan “{id}” simgesi yazılmadı"
                    ));
                    continue;
                }
            },
            None => sym.clone(),
        };
        out.extend(symbolizers(&sym, key, lost, what));
    }
    out
}

struct RuleOut {
    name: String,
    filter: Option<Filter>,
    is_else: bool,
    min_scale: Option<f64>,
    max_scale: Option<f64>,
    symbolizers: Vec<String>,
}

fn rule_xml(r: &RuleOut, out: &mut String) {
    out.push_str("        <se:Rule>\n");
    let _ = writeln!(out, "          <se:Name>{}</se:Name>", esc(&r.name));
    let _ = writeln!(
        out,
        "          <se:Description><se:Title>{}</se:Title></se:Description>",
        esc(&r.name)
    );
    if let Some(f) = &r.filter {
        out.push_str("          <ogc:Filter>\n");
        filter_xml(f, out, "            ");
        out.push_str("          </ogc:Filter>\n");
    } else if r.is_else {
        out.push_str("          <se:ElseFilter/>\n");
    }
    if let Some(s) = r.min_scale {
        let _ = writeln!(
            out,
            "          <se:MinScaleDenominator>{}</se:MinScaleDenominator>",
            num(s)
        );
    }
    if let Some(s) = r.max_scale {
        let _ = writeln!(
            out,
            "          <se:MaxScaleDenominator>{}</se:MaxScaleDenominator>",
            num(s)
        );
    }
    for s in &r.symbolizers {
        let _ = writeln!(out, "          {s}");
    }
    out.push_str("        </se:Rule>\n");
}

fn flat_rules(
    rules: &[Value],
    parent: Option<&Filter>,
    kinds: Kinds,
    library: &dyn Fn(&str) -> Option<Value>,
    lost: &mut Vec<String>,
    out: &mut Vec<RuleOut>,
) {
    for r in rules {
        if r.get("enabled") == Some(&Value::Bool(false)) {
            continue;
        }
        let label = r
            .get("label")
            .and_then(Value::as_str)
            .unwrap_or("Kural")
            .to_owned();
        let what = format!("“{label}” kuralı");
        let own = match r
            .get("filter")
            .and_then(Value::as_str)
            .filter(|s| !s.trim().is_empty())
        {
            Some(src) => {
                match parse_filter(src) {
                    Some(f) => Some(f),
                    None => {
                        lost.push(format!("{what}: “{src}” ifadesi SLD'nin süzgeciyle söylenemiyor; kural yazılmadı"));
                        continue;
                    }
                }
            }
            None => None,
        };
        let filter = match (parent, own) {
            (Some(p), Some(o)) => Some(Filter::And(vec![p.clone(), o])),
            (Some(p), None) => Some(p.clone()),
            (None, o) => o,
        };
        let is_else = r.get("isElse") == Some(&Value::Bool(true));
        let syms = set_symbolizers(r.get("symbols"), kinds, library, lost, &what);
        if !syms.is_empty() {
            out.push(RuleOut {
                name: label.clone(),
                filter: filter.clone(),
                is_else: is_else && parent.is_none(),
                min_scale: r.get("minScale").and_then(Value::as_f64),
                max_scale: r.get("maxScale").and_then(Value::as_f64),
                symbolizers: syms,
            });
        }
        if is_else && parent.is_some() {
            lost.push(format!(
                "{what}: iç içe “Diğerleri” kuralı SLD'ye üst kuralının süzgeciyle yazıldı"
            ));
        }
        if let Some(children) = r.get("children").and_then(Value::as_array) {
            flat_rules(children, filter.as_ref(), kinds, library, lost, out);
        }
    }
}

/// The SLD of a layer named `name` drawn by `renderer` (the app's JSON), its symbolizers for `kinds`; library
/// symbols by id through `library`.
pub fn write(
    name: &str,
    renderer: &Value,
    kinds: Kinds,
    library: &dyn Fn(&str) -> Option<Value>,
) -> Written {
    let mut lost = Vec::new();
    let mut rules: Vec<RuleOut> = Vec::new();
    let kind = renderer.get("type").and_then(Value::as_str).unwrap_or("");
    match kind {
        "single" => rules.push(RuleOut {
            name: name.to_owned(),
            filter: None,
            is_else: false,
            min_scale: None,
            max_scale: None,
            symbolizers: set_symbolizers(renderer.get("symbols"), kinds, library, &mut lost, "Simge"),
        }),
        "categorized" => {
            let expr = renderer.get("expr").and_then(Value::as_str).unwrap_or("");
            match field_of(expr) {
                None => lost.push(format!(
                    "Kategorili görünüşün “{expr}” ifadesi bir alan değil; SLD'nin süzgeci alanın değerini sorar, sınıflar yazılmadı"
                )),
                Some(field) => {
                    for c in renderer.get("categories").and_then(Value::as_array).cloned().unwrap_or_default() {
                        if c.get("enabled") == Some(&Value::Bool(false)) {
                            continue;
                        }
                        let value = c.get("value").and_then(Value::as_str).unwrap_or("");
                        let label = c.get("label").and_then(Value::as_str).filter(|l| !l.is_empty()).unwrap_or(value);
                        let lit = match value.parse::<f64>() {
                            Ok(n) if num(n) == value => Lit::Num(n),
                            _ => Lit::Text(value.to_owned()),
                        };
                        rules.push(RuleOut {
                            name: label.to_owned(),
                            filter: Some(Filter::Cmp(Op::Eq, field.clone(), lit)),
                            is_else: false,
                            min_scale: None,
                            max_scale: None,
                            symbolizers: set_symbolizers(
                                c.get("symbols"),
                                kinds,
                                library,
                                &mut lost,
                                &format!("“{label}” sınıfı"),
                            ),
                        });
                    }
                    if let Some(other) = renderer.get("other") {
                        rules.push(RuleOut {
                            name: "Diğerleri".into(),
                            filter: None,
                            is_else: true,
                            min_scale: None,
                            max_scale: None,
                            symbolizers: set_symbolizers(Some(other), kinds, library, &mut lost, "“Diğerleri” sınıfı"),
                        });
                    }
                }
            }
        }
        "graduated" => {
            let expr = renderer.get("expr").and_then(Value::as_str).unwrap_or("");
            match field_of(expr) {
                None => lost.push(format!(
                    "Aralıklı görünüşün “{expr}” ifadesi bir alan değil; SLD'nin süzgeci alanın değerini sorar, sınıflar yazılmadı"
                )),
                Some(field) => {
                    let classes = renderer.get("classes").and_then(Value::as_array).cloned().unwrap_or_default();
                    let n = classes.len();
                    for (k, c) in classes.iter().enumerate() {
                        let (Some(lo), Some(hi)) = (c.get("min").and_then(Value::as_f64), c.get("max").and_then(Value::as_f64)) else {
                            continue;
                        };
                        let label = c.get("label").and_then(Value::as_str).map_or_else(|| format!("{} - {}", num(lo), num(hi)), str::to_owned);
                        // min ≤ value < max; the last class takes its max too.
                        let top = if k + 1 == n { Op::Le } else { Op::Lt };
                        rules.push(RuleOut {
                            name: label.clone(),
                            filter: Some(Filter::And(vec![
                                Filter::Cmp(Op::Ge, field.clone(), Lit::Num(lo)),
                                Filter::Cmp(top, field.clone(), Lit::Num(hi)),
                            ])),
                            is_else: false,
                            min_scale: None,
                            max_scale: None,
                            symbolizers: set_symbolizers(c.get("symbols"), kinds, library, &mut lost, &format!("“{label}” sınıfı")),
                        });
                    }
                }
            }
        }
        "rules" => {
            let list = renderer.get("rules").and_then(Value::as_array).cloned().unwrap_or_default();
            flat_rules(&list, None, kinds, library, &mut lost, &mut rules);
        }
        other => lost.push(format!(
            "“{other}” görünüşü SLD'de yok (yalnız tek simge, kategorili, aralıklı ve kurallı görünüş yazılır)"
        )),
    }
    rules.retain(|r| !r.symbolizers.is_empty());
    let mut xml = String::new();
    xml.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    xml.push_str("<StyledLayerDescriptor xmlns=\"http://www.opengis.net/sld\" xmlns:ogc=\"http://www.opengis.net/ogc\" xmlns:se=\"http://www.opengis.net/se\" xmlns:xlink=\"http://www.w3.org/1999/xlink\" xmlns:xsi=\"http://www.w3.org/2001/XMLSchema-instance\" version=\"1.1.0\" xsi:schemaLocation=\"http://www.opengis.net/sld http://schemas.opengis.net/sld/1.1.0/StyledLayerDescriptor.xsd\">\n");
    xml.push_str("  <NamedLayer>\n");
    let _ = writeln!(xml, "    <se:Name>{}</se:Name>", esc(name));
    xml.push_str("    <UserStyle>\n");
    let _ = writeln!(xml, "      <se:Name>{}</se:Name>", esc(name));
    xml.push_str("      <se:FeatureTypeStyle>\n");
    for r in &rules {
        rule_xml(r, &mut xml);
    }
    xml.push_str("      </se:FeatureTypeStyle>\n    </UserStyle>\n  </NamedLayer>\n</StyledLayerDescriptor>\n");
    Written {
        xml,
        lost,
        rules: rules.len(),
    }
}

// ─── Reading ──────────────────────────────────────────────────────────────────────────────────────────

type Node<'a, 'i> = roxmltree::Node<'a, 'i>;

fn child<'a, 'i>(n: Node<'a, 'i>, name: &str) -> Option<Node<'a, 'i>> {
    n.children()
        .find(|c| c.is_element() && c.tag_name().name() == name)
}

fn children<'a, 'i>(n: Node<'a, 'i>, name: &'static str) -> impl Iterator<Item = Node<'a, 'i>> {
    n.children()
        .filter(move |c| c.is_element() && c.tag_name().name() == name)
}

fn text_of(n: Node<'_, '_>) -> String {
    n.descendants()
        .filter(|d| d.is_text())
        .filter_map(|d| d.text())
        .collect::<String>()
        .trim()
        .to_owned()
}

/// An SE parameter's text (`SvgParameter` or the old `CssParameter`), by name.
fn param(n: Node<'_, '_>, name: &str) -> Option<String> {
    n.children()
        .filter(|c| {
            c.is_element() && matches!(c.tag_name().name(), "SvgParameter" | "CssParameter")
        })
        .find(|c| c.attribute("name") == Some(name))
        .map(text_of)
}

fn read_filter(n: Node<'_, '_>, lost: &mut Vec<String>) -> Option<Filter> {
    let tag = n.tag_name().name();
    let prop = |n: Node<'_, '_>| child(n, "PropertyName").map(text_of);
    let lit = |n: Node<'_, '_>| -> Option<Lit> {
        let t = text_of(child(n, "Literal")?);
        Some(match t.parse::<f64>() {
            Ok(v) if v.is_finite() && !t.starts_with('+') => Lit::Num(v),
            _ => Lit::Text(t),
        })
    };
    if let Some(op) = Op::of_ogc(tag) {
        return Some(Filter::Cmp(op, prop(n)?, lit(n)?));
    }
    match tag {
        "And" | "Or" => {
            let parts: Option<Vec<Filter>> = n
                .children()
                .filter(|c| c.is_element())
                .map(|c| read_filter(c, lost))
                .collect();
            let parts = parts?;
            Some(if tag == "And" {
                Filter::And(parts)
            } else {
                Filter::Or(parts)
            })
        }
        "Not" => Some(Filter::Not(Box::new(read_filter(
            n.children().find(|c| c.is_element())?,
            lost,
        )?))),
        "PropertyIsNull" => Some(Filter::Null(prop(n)?)),
        "PropertyIsBetween" => {
            let lo = text_of(child(child(n, "LowerBoundary")?, "Literal")?)
                .parse()
                .ok()?;
            let hi = text_of(child(child(n, "UpperBoundary")?, "Literal")?)
                .parse()
                .ok()?;
            Some(Filter::Between(prop(n)?, lo, hi))
        }
        "PropertyIsLike" => {
            let wild = n.attribute("wildCard").unwrap_or("%");
            let single = n
                .attribute("singleChar")
                .or_else(|| n.attribute("singleCharacter"))
                .unwrap_or("_");
            let p = text_of(child(n, "Literal")?)
                .replace(wild, "%")
                .replace(single, "_");
            Some(Filter::Like(prop(n)?, p))
        }
        other => {
            lost.push(format!("“{other}” süzgeci KentOS'un ifadesine çevrilemedi"));
            None
        }
    }
}

/// A size read in SE's unit `uom` as a value in a symbol layer's unit: the value and the unit.
fn size_in(v: f64, uom: Option<&str>) -> (f64, &'static str) {
    match uom {
        Some(u) if u.ends_with("/metre") => (v, "m"),
        Some(u) if u.ends_with("/foot") => (v * 0.3048, "m"),
        // A pixel stays a screen pixel, as QGIS reads it.
        _ => (v, "px"),
    }
}

pub(crate) fn colour_with(rgb: &str, opacity: Option<f64>) -> String {
    match opacity.filter(|o| *o < 1.0) {
        Some(o) => format!(
            "{}{:02x}",
            rgb.to_ascii_lowercase(),
            (o.clamp(0.0, 1.0) * 255.0).round() as u8
        ),
        None => rgb.to_ascii_lowercase(),
    }
}

fn read_stroke(s: Node<'_, '_>, uom: Option<&str>, id: &str) -> Value {
    let rgb = param(s, "stroke").unwrap_or_else(|| "#000000".into());
    let opacity = param(s, "stroke-opacity").and_then(|o| o.parse().ok());
    let (width, unit) = size_in(
        param(s, "stroke-width")
            .and_then(|w| w.parse().ok())
            .unwrap_or(1.0),
        uom,
    );
    let mut l = json!({
        "id": id, "type": "simpleLine", "color": colour_with(&rgb, opacity), "width": width, "unit": unit,
    });
    if let Some(dash) = param(s, "stroke-dasharray") {
        let d: Vec<f64> = dash
            .split([' ', ','])
            .filter_map(|p| p.trim().parse::<f64>().ok())
            .map(|v| size_in(v, uom).0)
            .collect();
        if !d.is_empty() {
            l["dash"] = json!(d);
        }
    }
    if let Some(j) =
        param(s, "stroke-linejoin").filter(|j| ["miter", "round", "bevel"].contains(&j.as_str()))
    {
        l["join"] = json!(j);
    }
    if let Some(c) =
        param(s, "stroke-linecap").filter(|c| ["butt", "round", "square"].contains(&c.as_str()))
    {
        l["cap"] = json!(c);
    }
    l
}

fn read_fill(f: Node<'_, '_>, id: &str, lost: &mut Vec<String>) -> Option<Value> {
    if child(f, "GraphicFill").is_some() {
        lost.push("desenli dolgu (GraphicFill) KentOS'un dolgusuna çevrilmedi".into());
        return None;
    }
    let rgb = param(f, "fill").unwrap_or_else(|| "#808080".into());
    let opacity = param(f, "fill-opacity").and_then(|o| o.parse().ok());
    Some(json!({ "id": id, "type": "simpleFill", "color": colour_with(&rgb, opacity) }))
}

fn read_mark(g: Node<'_, '_>, uom: Option<&str>, lost: &mut Vec<String>) -> Option<Value> {
    if child(g, "ExternalGraphic").is_some() {
        lost.push("dış resimli işaret (ExternalGraphic) KentOS'un işaretine çevrilmedi".into());
    }
    let mark = child(g, "Mark")?;
    let wkn = child(mark, "WellKnownName")
        .map(text_of)
        .unwrap_or_else(|| "square".into());
    let shape = match wkn.as_str() {
        "circle" | "square" | "triangle" | "star" | "cross" | "x" => wkn.clone(),
        other => {
            lost.push(format!("“{other}” işareti KentOS'ta yok; kare yazıldı"));
            "square".into()
        }
    };
    let (size, unit) = size_in(
        child(g, "Size")
            .map(text_of)
            .and_then(|s| s.parse().ok())
            .unwrap_or(6.0),
        uom,
    );
    let mut m = json!({ "id": "p", "type": "shape", "shape": shape, "size": size, "unit": unit, "fill": null });
    if let Some(f) = child(mark, "Fill") {
        let rgb = param(f, "fill").unwrap_or_else(|| "#808080".into());
        m["fill"] = json!(colour_with(
            &rgb,
            param(f, "fill-opacity").and_then(|o| o.parse().ok())
        ));
    }
    if let Some(s) = child(mark, "Stroke") {
        let rgb = param(s, "stroke").unwrap_or_else(|| "#000000".into());
        m["stroke"] = json!(colour_with(
            &rgb,
            param(s, "stroke-opacity").and_then(|o| o.parse().ok())
        ));
        let (w, _) = size_in(
            param(s, "stroke-width")
                .and_then(|w| w.parse().ok())
                .unwrap_or(1.0),
            uom,
        );
        m["strokeWidth"] = json!(w);
    }
    if let Some(r) = child(g, "Rotation")
        .map(text_of)
        .and_then(|r| r.parse::<f64>().ok())
    {
        m["rotation"] = json!(-r);
    }
    Some(m)
}

/// A rule's symbol set from its symbolizers.
fn read_symbols(rule: Node<'_, '_>, lost: &mut Vec<String>) -> Value {
    let mut fill_layers = Vec::new();
    let mut line_layers = Vec::new();
    let mut marker_layers = Vec::new();
    for s in rule.children().filter(|c| c.is_element()) {
        let uom = s.attribute("uom");
        match s.tag_name().name() {
            "PolygonSymbolizer" => {
                if let Some(f) = child(s, "Fill")
                    .and_then(|f| read_fill(f, &format!("f{}", fill_layers.len()), lost))
                {
                    fill_layers.push(f);
                }
                if let Some(st) = child(s, "Stroke") {
                    fill_layers.push(read_stroke(st, uom, &format!("o{}", fill_layers.len())));
                }
                if child(s, "Displacement").is_some() || child(s, "PerpendicularOffset").is_some() {
                    lost.push("alanın kaydırması KentOS'un dolgusuna çevrilmedi".into());
                }
            }
            "LineSymbolizer" => {
                if let Some(st) = child(s, "Stroke") {
                    if child(st, "GraphicStroke").is_some() {
                        lost.push(
                            "işaretli çizgi (GraphicStroke) KentOS'un çizgisine çevrilmedi".into(),
                        );
                        continue;
                    }
                    let mut l = read_stroke(st, uom, &format!("l{}", line_layers.len()));
                    if let Some(o) = child(s, "PerpendicularOffset")
                        .map(text_of)
                        .and_then(|o| o.parse::<f64>().ok())
                    {
                        l["offset"] = json!(size_in(o, uom).0);
                    }
                    line_layers.push(l);
                }
            }
            "PointSymbolizer" => {
                if let Some(m) = child(s, "Graphic").and_then(|g| read_mark(g, uom, lost)) {
                    marker_layers.push(m);
                }
            }
            "TextSymbolizer" => lost.push(
                "etiket (TextSymbolizer) görünüşe çevrilmedi; KentOS'ta etiketler ayrıdır".into(),
            ),
            "RasterSymbolizer" => lost.push("raster görünüşü (RasterSymbolizer) çevrilmedi".into()),
            _ => {}
        }
    }
    let mut set = Map::new();
    if !fill_layers.is_empty() {
        set.insert(
            "fill".into(),
            json!({ "type": "fill", "layers": fill_layers }),
        );
    }
    if !line_layers.is_empty() {
        set.insert(
            "line".into(),
            json!({ "type": "line", "layers": line_layers }),
        );
    }
    if !marker_layers.is_empty() {
        set.insert(
            "marker".into(),
            json!({ "type": "marker", "layers": marker_layers }),
        );
    }
    Value::Object(set)
}

struct RuleIn {
    label: String,
    filter: Option<Filter>,
    is_else: bool,
    min_scale: Option<f64>,
    max_scale: Option<f64>,
    symbols: Value,
}

/// The categories of rules each asking one field for one value, or none.
fn as_categories(rules: &[RuleIn]) -> Option<Value> {
    let mut field: Option<&str> = None;
    let mut cats = Vec::new();
    let mut other = None;
    for r in rules {
        if r.min_scale.is_some() || r.max_scale.is_some() {
            return None;
        }
        match &r.filter {
            Some(Filter::Cmp(Op::Eq, f, lit)) => {
                if field.is_some_and(|x| x != f) {
                    return None;
                }
                field = Some(f);
                let value = match lit {
                    Lit::Num(n) => num(*n),
                    Lit::Text(s) => s.clone(),
                };
                cats.push(json!({ "value": value, "label": r.label, "symbols": r.symbols }));
            }
            None if r.is_else && other.is_none() => other = Some(r.symbols.clone()),
            _ => return None,
        }
    }
    let field = field?;
    let mut v = json!({ "type": "categorized", "expr": format!("[{field}]"), "categories": cats });
    if let Some(o) = other {
        v["other"] = o;
    }
    Some(v)
}

/// The classes of rules each asking one field for `min ≤ v < max` (the last `≤ max`), back to back, or none.
fn as_classes(rules: &[RuleIn]) -> Option<Value> {
    let mut field: Option<&str> = None;
    let mut classes = Vec::new();
    let n = rules.len();
    for (k, r) in rules.iter().enumerate() {
        if r.is_else || r.min_scale.is_some() || r.max_scale.is_some() {
            return None;
        }
        let (f, lo, hi, last_closed) = match &r.filter {
            Some(Filter::And(p)) if p.len() == 2 => match (&p[0], &p[1]) {
                (Filter::Cmp(op1, f1, Lit::Num(a)), Filter::Cmp(op2, f2, Lit::Num(b)))
                    if f1 == f2
                        && matches!(op1, Op::Ge | Op::Gt)
                        && matches!(op2, Op::Lt | Op::Le) =>
                {
                    (f1.as_str(), *a, *b, *op2 == Op::Le)
                }
                _ => return None,
            },
            Some(Filter::Between(f, a, b)) => (f.as_str(), *a, *b, true),
            _ => return None,
        };
        if field.is_some_and(|x| x != f) {
            return None;
        }
        // KentOS's classes are min ≤ v < max but the last; QGIS writes min < v ≤ max past the first, which reads
        // the same but at the class bounds (said by the caller).
        let _ = (k, n, last_closed);
        field = Some(f);
        classes.push(json!({ "min": lo, "max": hi, "label": r.label, "symbols": r.symbols }));
    }
    let field = field?;
    Some(json!({ "type": "graduated", "expr": format!("[{field}]"), "classes": classes }))
}

/// KentOS's renderer of an SLD (or an SE `FeatureTypeStyle` alone): rules of one field's values become a categorized
/// renderer, of one field's ranges a graduated one, one rule with no filter a single symbol, others rules.
pub fn read(text: &str) -> Result<Read, String> {
    let doc =
        crate::xml::document(text).map_err(|_| "Dosya okunabilir bir XML değil.".to_owned())?;
    let root = doc.root_element();
    let name = root
        .descendants()
        .find(|n| n.is_element() && n.tag_name().name() == "NamedLayer")
        .and_then(|l| child(l, "Name"))
        .map(text_of);
    let styles: Vec<Node<'_, '_>> = root
        .descendants()
        .filter(|n| n.is_element() && n.tag_name().name() == "FeatureTypeStyle")
        .collect();
    let Some(first) = styles.first() else {
        return Err("Dosyada SLD'nin görünüşü (FeatureTypeStyle) yok.".into());
    };
    let mut lost = Vec::new();
    if styles.len() > 1 {
        lost.push(format!("{} görünüşten yalnız ilki alındı", styles.len()));
    }
    let mut rules = Vec::new();
    for r in children(*first, "Rule") {
        let label = child(r, "Description")
            .and_then(|d| child(d, "Title"))
            .map(text_of)
            .filter(|t| !t.is_empty())
            .or_else(|| child(r, "Name").map(text_of))
            .unwrap_or_else(|| format!("Kural {}", rules.len() + 1));
        let filter = match child(r, "Filter") {
            Some(f) => match f.children().find(|c| c.is_element()) {
                Some(inner) => match read_filter(inner, &mut lost) {
                    Some(f) => Some(f),
                    None => {
                        lost.push(format!(
                            "“{label}” kuralının süzgeci okunamadı; kural alınmadı"
                        ));
                        continue;
                    }
                },
                None => None,
            },
            None => None,
        };
        let scale = |tag: &str| {
            child(r, tag)
                .map(text_of)
                .and_then(|s| s.parse::<f64>().ok())
        };
        let symbols = read_symbols(r, &mut lost);
        if symbols.as_object().is_some_and(Map::is_empty) {
            continue;
        }
        rules.push(RuleIn {
            label,
            is_else: child(r, "ElseFilter").is_some(),
            filter,
            min_scale: scale("MinScaleDenominator"),
            max_scale: scale("MaxScaleDenominator"),
            symbols,
        });
    }
    if rules.is_empty() {
        return Err("SLD'de KentOS'un çizebileceği bir kural yok.".into());
    }
    let count = rules.len();
    let renderer = if rules.len() == 1
        && rules[0].filter.is_none()
        && !rules[0].is_else
        && rules[0].min_scale.is_none()
        && rules[0].max_scale.is_none()
    {
        json!({ "type": "single", "symbols": rules[0].symbols })
    } else if let Some(c) = as_categories(&rules) {
        c
    } else if let Some(c) = as_classes(&rules) {
        c
    } else {
        let list: Vec<Value> = rules
            .iter()
            .enumerate()
            .map(|(k, r)| {
                let mut v =
                    json!({ "id": format!("r{}", k + 1), "label": r.label, "symbols": r.symbols });
                if let Some(f) = &r.filter {
                    v["filter"] = json!(filter_text(f));
                }
                if r.is_else {
                    v["isElse"] = json!(true);
                }
                if let Some(s) = r.min_scale {
                    v["minScale"] = json!(s);
                }
                if let Some(s) = r.max_scale {
                    v["maxScale"] = json!(s);
                }
                v
            })
            .collect();
        json!({ "type": "rules", "rules": list })
    };
    Ok(Read {
        name,
        renderer,
        lost,
        rules: count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: &str) -> Option<Value> {
        None
    }

    #[test]
    fn filters_go_out_and_come_back() {
        for src in [
            "[Ada] = 'A'",
            "[alan] >= 100 ve [alan] < 500",
            "[tür] = 'konut' veya ([kat] > 3 ve [kat] <= 10)",
            "[not] boş",
            "[not] boş değil",
            "[ad] gibi 'Ka%'",
            "[alan] arasında 10 ve 20",
            "değil ([Ada] = 'B')",
        ] {
            let f = parse_filter(src).unwrap_or_else(|| panic!("{src}"));
            let mut xml = String::from("<ogc:Filter xmlns:ogc=\"http://www.opengis.net/ogc\">");
            filter_xml(&f, &mut xml, "");
            xml.push_str("</ogc:Filter>");
            let doc = roxmltree::Document::parse(&xml).expect("xml");
            let inner = doc
                .root_element()
                .children()
                .find(|c| c.is_element())
                .expect("inner");
            let back = read_filter(inner, &mut Vec::new()).expect("reads");
            assert_eq!(back, f, "{src}");
            assert_eq!(parse_filter(&filter_text(&back)), Some(f), "{src}");
        }
        assert!(parse_filter("$alan > 10").is_none());
        assert!(parse_filter("upper([ad]) = 'A'").is_none());
    }

    #[test]
    fn a_categorized_style_goes_out_and_comes_back() {
        let r = json!({ "type": "categorized", "expr": "[ada]", "categories": [
            { "value": "A", "label": "Ada A", "symbols": { "fill": { "type": "fill", "layers": [
                { "id": "f", "type": "simpleFill", "color": "#e41a1c" },
                { "id": "o", "type": "simpleLine", "color": "#222222", "width": 0.28, "unit": "mm" } ] } } },
            { "value": "B", "label": "Ada B", "symbols": { "fill": { "type": "fill", "layers": [
                { "id": "f", "type": "simpleFill", "color": "#377eb8cc" } ] } } } ] });
        let w = write("parsel", &r, Kinds::ALL, &none);
        assert!(w.lost.is_empty(), "{:?}", w.lost);
        assert_eq!(w.rules, 2);
        assert!(
            w.xml
                .contains("<se:SvgParameter name=\"stroke-width\">1</se:SvgParameter>"),
            "{}",
            w.xml
        );
        let back = read(&w.xml).expect("reads");
        assert_eq!(back.renderer["type"], "categorized");
        assert_eq!(back.renderer["expr"], "[ada]");
        assert_eq!(
            back.renderer["categories"][1]["symbols"]["fill"]["layers"][0]["color"],
            "#377eb8cc"
        );
        // 0.28 mm is one pixel, and a pixel comes back a pixel (QGIS reads it so).
        assert_eq!(
            back.renderer["categories"][0]["symbols"]["fill"]["layers"][1]["width"],
            1.0
        );
        assert_eq!(
            back.renderer["categories"][0]["symbols"]["fill"]["layers"][1]["unit"],
            "px"
        );
    }

    #[test]
    fn what_se_cannot_say_is_named() {
        let r = json!({ "type": "single", "symbols": {
            "fill": { "type": "fill", "layers": [ { "id": "h", "type": "hatchFill", "angle": 45, "spacing": 2, "width": 0.2, "color": "#000" } ] },
            "line": { "type": "line", "layers": [ { "id": "l", "type": "simpleLine", "color": { "expr": "[renk]" }, "width": 0.3 } ] },
            "marker": { "ref": "kitapta-yok" } } });
        let w = write("x", &r, Kinds::ALL, &none);
        assert!(
            w.lost.iter().any(|l| l.contains("tarama dolgusu")),
            "{:?}",
            w.lost
        );
        assert!(
            w.lost.iter().any(|l| l.contains("ifadeyle")),
            "{:?}",
            w.lost
        );
        assert!(
            w.lost.iter().any(|l| l.contains("kitaplıkta olmayan")),
            "{:?}",
            w.lost
        );
        let r = json!({ "type": "heatmap" });
        assert!(write("x", &r, Kinds::ALL, &none).lost[0].contains("SLD'de yok"));
    }
}
