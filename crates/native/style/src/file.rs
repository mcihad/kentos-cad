//! The .kstil file (the web's `style/file.ts`, docs/STYLE.md §5): styles to
//! export, import and share. Versioned JSON with the assets its symbols draw
//! with embedded, so a file is complete on its own. Everything read from a
//! file is checked (a shared file is untrusted data) and SVG drawings are
//! cleaned of scripts and outside references. Both platforms are held to
//! `fixtures/style/v1/kstil.json`; where the TypeScript leans on regular
//! expressions this scans the text the same way.

use kentos_style_core::js::number;
use kentos_style_core::js::text::{is_space, trim};
use serde_json::{Map, Value, json};

use crate::classify::js_number;
use crate::library::{ItemKind, Source, StyleLibrary, assets_of_symbol, new_item_id};

pub const STYLE_FORMAT: &str = "kentos-style";
pub const STYLE_VERSION: u32 = 1;

/// The time now as JavaScript writes it (`toISOString`): `2026-09-27T12:34:56.789Z`.
pub fn iso_now() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let ms = now.as_millis() as i64;
    let (days, rest) = (ms.div_euclid(86_400_000), ms.rem_euclid(86_400_000));
    // Days to a civil date (Howard Hinnant's algorithm).
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}.{:03}Z",
        rest / 3_600_000,
        rest / 60_000 % 60,
        rest / 1000 % 60,
        rest % 1000
    )
}

/// The chosen items, and the assets their symbols use (from any source), as a
/// file; each item once, an item before its assets (`exportStyles`).
pub fn export_styles(lib: &StyleLibrary, ids: &[&str]) -> Value {
    fn put(lib: &StyleLibrary, id: &str, out: &mut Vec<Value>, seen: &mut Vec<String>) {
        let Some((item, _)) = lib.get(id) else {
            return;
        };
        if seen.iter().any(|s| s == id) {
            return;
        }
        seen.push(id.to_owned());
        out.push(item.value().clone());
        if let Some(symbol) = item.symbol() {
            for a in assets_of_symbol(symbol) {
                put(lib, &a, out, seen);
            }
        }
    }
    let mut items = Vec::new();
    let mut seen = Vec::new();
    for id in ids {
        put(lib, id, &mut items, &mut seen);
    }
    json!({ "format": STYLE_FORMAT, "version": STYLE_VERSION, "exported": iso_now(), "items": items })
}

// ── Validation ─────────────────────────────────────────────────────────

const UNITS: [&str; 3] = ["mm", "px", "m"];
const SHAPES: [&str; 20] = [
    "circle",
    "ring",
    "square",
    "rectangle",
    "diamond",
    "triangle",
    "pentagon",
    "hexagon",
    "octagon",
    "star",
    "cross",
    "x",
    "line",
    "arrow",
    "arrowhead",
    "chevron",
    "semicircle",
    "quartercircle",
    "gear",
    "arc",
];
const PLACEMENTS: [&str; 7] = [
    "interval",
    "vertex",
    "innerVertex",
    "first",
    "last",
    "center",
    "segmentCenter",
];

fn layer_types(kind: &str) -> Option<&'static [&'static str]> {
    match kind {
        "marker" => Some(&["shape", "svg", "text", "raster"]),
        "line" => Some(&["simpleLine", "markerLine"]),
        "fill" => Some(&[
            "simpleFill",
            "hatchFill",
            "patternFill",
            "imageFill",
            "centroidMarker",
            "simpleLine",
            "markerLine",
        ]),
        _ => None,
    }
}

/// `String(v)` of JavaScript, for what a message quotes (absent is `undefined`).
fn js_str(v: Option<&Value>) -> String {
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
                x => js_str(Some(x)),
            })
            .collect::<Vec<_>>()
            .join(","),
        Some(Value::Object(_)) => "[object Object]".into(),
    }
}

fn is_expr(v: Option<&Value>) -> bool {
    v.is_some_and(|v| v.get("expr").is_some_and(Value::is_string))
}

/// A colour of a symbol: #RRGGBB, #RRGGBBAA or a theme token, in any case.
fn is_color(s: &str) -> bool {
    let lower = s.to_ascii_lowercase();
    if matches!(lower.as_str(), "ink" | "paper" | "fg" | "fg-dim") {
        return true;
    }
    let Some(hex) = lower.strip_prefix('#') else {
        return false;
    };
    (hex.len() == 6 || hex.len() == 8) && hex.bytes().all(|b| b.is_ascii_hexdigit())
}

#[derive(Default)]
struct Num {
    min: Option<f64>,
    /// Absent is wrong too (JavaScript's `optional: false`).
    required: bool,
    /// An expression (`{ expr }`) is fine.
    dd: bool,
}

struct Check {
    issues: Vec<String>,
}

impl Check {
    fn bad(&mut self, w: &str, m: impl AsRef<str>) {
        self.issues.push(format!("{w}: {}", m.as_ref()));
    }

    fn num(&mut self, w: &str, v: Option<&Value>, name: &str, o: Num) {
        if v.is_none() && !o.required {
            return;
        }
        if o.dd && is_expr(v) {
            return;
        }
        let Some(x) = v.and_then(Value::as_f64).filter(|x| x.is_finite()) else {
            return self.bad(w, format!("{name} bir sayı olmalı"));
        };
        if let Some(min) = o.min
            && x < min
        {
            self.bad(w, format!("{name} en az {} olmalı", number::to_string(min)));
        }
    }

    fn color(&mut self, w: &str, v: Option<&Value>, name: &str, nullable: bool) {
        if v.is_none() || (nullable && v.is_some_and(Value::is_null)) {
            return;
        }
        if is_expr(v) {
            return;
        }
        if !v.and_then(Value::as_str).is_some_and(is_color) {
            self.bad(
                w,
                format!(
                    "{name} geçerli bir renk değil (#RRGGBB, #RRGGBBAA, ink, paper, fg, fg-dim)"
                ),
            );
        }
    }

    fn walk(&mut self, s: Option<&Value>, w: &str) {
        let Some(s) = s.filter(|s| s.is_object()) else {
            return self.bad(w, "nesne değil");
        };
        let kind = s.get("type");
        let Some(types) = kind.and_then(Value::as_str).and_then(layer_types) else {
            return self.bad(w, format!("bilinmeyen sembol türü “{}”", js_str(kind)));
        };
        let kind = js_str(kind);
        let Some(layers) = s.get("layers").and_then(Value::as_array) else {
            return self.bad(w, "katman listesi yok");
        };
        for (i, l) in layers.iter().enumerate() {
            let lw = format!("{w} › katman {}", i + 1);
            if !l.is_object() {
                self.bad(&lw, "nesne değil");
                continue;
            }
            let g = |k: &str| l.get(k);
            if !g("id").is_some_and(Value::is_string) {
                self.bad(&lw, "kimlik yok");
            }
            let lt = js_str(g("type"));
            if !types.contains(&lt.as_str()) {
                self.bad(&lw, format!("“{kind}” sembolünde “{lt}” katmanı olamaz"));
                continue;
            }
            if g("unit").is_some() && !UNITS.contains(&js_str(g("unit")).as_str()) {
                self.bad(&lw, format!("bilinmeyen birim “{}”", js_str(g("unit"))));
            }
            let min0 = || Num {
                min: Some(0.0),
                ..Num::default()
            };
            let size = || Num {
                min: Some(0.0),
                required: true,
                dd: true,
            };
            let tiny = || Num {
                min: Some(0.0001),
                required: true,
                ..Num::default()
            };
            self.num(&lw, g("opacity"), "opaklık", min0());
            match lt.as_str() {
                "shape" => {
                    if !SHAPES.contains(&js_str(g("shape")).as_str()) {
                        self.bad(&lw, format!("bilinmeyen şekil “{}”", js_str(g("shape"))));
                    }
                    self.num(&lw, g("size"), "boyut", size());
                    self.color(&lw, g("fill"), "dolgu", true);
                    self.color(&lw, g("stroke"), "çizgi rengi", true);
                    self.num(&lw, g("strokeWidth"), "çizgi kalınlığı", min0());
                    self.num(&lw, g("hole"), "delik", min0());
                    self.num(
                        &lw,
                        g("teeth"),
                        "diş sayısı",
                        Num {
                            min: Some(3.0),
                            ..Num::default()
                        },
                    );
                    self.num(&lw, g("teethDepth"), "diş derinliği", min0());
                    self.num(&lw, g("sweep"), "yay açıklığı", min0());
                }
                "svg" | "raster" => {
                    if !g("asset").is_some_and(Value::is_string) {
                        self.bad(&lw, "varlık kimliği yok");
                    }
                    self.num(&lw, g("size"), "boyut", size());
                }
                "text" => {
                    if !g("text").is_some_and(Value::is_string) && !is_expr(g("text")) {
                        self.bad(&lw, "metin yok");
                    }
                    self.num(&lw, g("size"), "boyut", size());
                    self.color(&lw, g("color"), "renk", false);
                }
                "simpleLine" => {
                    self.color(&lw, g("color"), "renk", false);
                    self.num(&lw, g("width"), "kalınlık", size());
                    if let Some(dash) = g("dash").filter(|d| !d.is_null()) {
                        let list = dash.as_array();
                        let bad_part = list.is_none_or(|d| {
                            d.iter()
                                .any(|x| x.as_f64().is_none_or(|x| !x.is_finite() || x < 0.0))
                        });
                        if bad_part {
                            self.bad(&lw, "kesik deseni sıfır ya da pozitif sayılar olmalı");
                        } else if list.is_some_and(|d| {
                            !d.is_empty() && d.iter().all(|x| x.as_f64() == Some(0.0))
                        }) {
                            self.bad(&lw, "kesik deseninin toplamı sıfır olamaz");
                        }
                    }
                    self.num(
                        &lw,
                        g("offset"),
                        "kaydırma",
                        Num {
                            dd: true,
                            ..Num::default()
                        },
                    );
                    self.num(&lw, g("blur"), "yumuşatma", min0());
                    if let Some(shift) = g("shift") {
                        let ok = shift.as_array().is_some_and(|s| {
                            s.len() == 2 && s.iter().all(|v| v.as_f64().is_some_and(f64::is_finite))
                        });
                        if !ok {
                            self.bad(&lw, "sayfa kaydırması iki sayı olmalı");
                        }
                    }
                    if let Some(wave) = g("wave") {
                        let shape = wave.is_object().then(|| js_str(wave.get("shape")));
                        if !shape
                            .is_some_and(|s| matches!(s.as_str(), "sine" | "zigzag" | "square"))
                        {
                            self.bad(&lw, "dalga biçimi sine, zigzag ya da square olmalı");
                        } else {
                            self.num(&lw, wave.get("length"), "dalga boyu", tiny());
                            self.num(
                                &lw,
                                wave.get("amplitude"),
                                "dalga genliği",
                                Num {
                                    min: Some(0.0),
                                    required: true,
                                    ..Num::default()
                                },
                            );
                            self.num(
                                &lw,
                                wave.get("spacing"),
                                "dalga tekrarı",
                                Num {
                                    min: Some(0.0001),
                                    ..Num::default()
                                },
                            );
                            self.num(
                                &lw,
                                wave.get("offsetAlong"),
                                "ilk dalga uzaklığı",
                                Num::default(),
                            );
                        }
                    }
                }
                "markerLine" => {
                    let placement = js_str(g("placement"));
                    if !PLACEMENTS.contains(&placement.as_str()) {
                        self.bad(&lw, format!("bilinmeyen yerleşim “{placement}”"));
                    }
                    if placement == "interval" {
                        self.num(&lw, g("interval"), "aralık", tiny());
                    }
                    self.num(&lw, g("offsetAlong"), "başlangıç uzaklığı", Num::default());
                    self.num(
                        &lw,
                        g("offset"),
                        "kaydırma",
                        Num {
                            dd: true,
                            ..Num::default()
                        },
                    );
                    self.walk(g("marker"), &format!("{lw} › işaret"));
                }
                "simpleFill" => self.color(&lw, g("color"), "renk", false),
                "hatchFill" => {
                    self.num(
                        &lw,
                        g("angle"),
                        "açı",
                        Num {
                            required: true,
                            ..Num::default()
                        },
                    );
                    self.num(&lw, g("spacing"), "aralık", tiny());
                    self.num(
                        &lw,
                        g("width"),
                        "kalınlık",
                        Num {
                            min: Some(0.0),
                            required: true,
                            ..Num::default()
                        },
                    );
                    self.color(&lw, g("color"), "renk", false);
                }
                "patternFill" => {
                    self.num(&lw, g("spacingX"), "yatay aralık", tiny());
                    self.num(&lw, g("spacingY"), "düşey aralık", tiny());
                    self.walk(g("marker"), &format!("{lw} › işaret"));
                }
                "imageFill" => {
                    if !g("asset").is_some_and(Value::is_string) {
                        self.bad(&lw, "varlık kimliği yok");
                    }
                    self.num(&lw, g("tileSize"), "döşeme boyu", tiny());
                }
                "centroidMarker" => self.walk(g("marker"), &format!("{lw} › işaret")),
                _ => {}
            }
            if let Some(m) = g("marker").filter(|m| m.is_object())
                && m.get("type").and_then(Value::as_str) != Some("marker")
            {
                self.bad(&lw, "iç sembol bir işaret sembolü olmalı");
            }
        }
    }
}

/// Problems in a symbol, each with where it is (“katman 2 › işaret › katman 1: …”) (`validateSymbol`).
pub fn validate_symbol(symbol: &Value, place: &str) -> Vec<String> {
    let mut c = Check { issues: Vec::new() };
    c.walk(Some(symbol), place);
    c.issues
}

// ── Cleaning SVG ───────────────────────────────────────────────────────

/// Where `needle` (ASCII, any case) is in `hay` at or after `from`.
fn find_ci(hay: &str, needle: &str, from: usize) -> Option<usize> {
    let (h, n) = (hay.as_bytes(), needle.as_bytes());
    (from..=h.len().checked_sub(n.len())?).find(|&i| h[i..i + n.len()].eq_ignore_ascii_case(n))
}

fn starts_ci(hay: &str, at: usize, needle: &str) -> bool {
    hay.as_bytes()
        .get(at..at + needle.len())
        .is_some_and(|s| s.eq_ignore_ascii_case(needle.as_bytes()))
}

/// JavaScript's `\s` at byte `at`, and its length.
fn space_at(s: &str, at: usize) -> Option<usize> {
    let c = s.get(at..)?.chars().next()?;
    is_space(c).then(|| c.len_utf8())
}

/// Past the whitespace from `at` (`\s*`).
fn skip_space(s: &str, mut at: usize) -> usize {
    while let Some(n) = space_at(s, at) {
        at += n;
    }
    at
}

/// Every match of a pattern replaced: `matcher` says, at a byte where a match
/// may start, where it ends; the text between matches is kept (a global replace).
fn replace_all(s: &str, with: &str, matcher: impl Fn(&str, usize) -> Option<usize>) -> String {
    let mut out = String::with_capacity(s.len());
    let mut at = 0;
    let mut kept = 0;
    while at < s.len() {
        match matcher(s, at) {
            Some(end) if end > at => {
                out.push_str(&s[kept..at]);
                out.push_str(with);
                at = end;
                kept = end;
            }
            _ => at += s[at..].chars().next().map_or(1, char::len_utf8),
        }
    }
    out.push_str(&s[kept..]);
    out
}

/// `<open[^>]*>`: from `open` to the first `>`.
fn tag(open: &'static str) -> impl Fn(&str, usize) -> Option<usize> {
    move |s, at| {
        if !starts_ci(s, at, open) {
            return None;
        }
        s[at + open.len()..]
            .find('>')
            .map(|k| at + open.len() + k + 1)
    }
}

/// `<open[\s\S]*?</close\s*>`: from `open` to the first closing tag after it.
fn element(open: &'static str, close: &'static str) -> impl Fn(&str, usize) -> Option<usize> {
    move |s, at| {
        if !starts_ci(s, at, open) {
            return None;
        }
        let mut from = at + open.len();
        while let Some(k) = find_ci(s, close, from) {
            let end = skip_space(s, k + close.len());
            if s.as_bytes().get(end) == Some(&b'>') {
                return Some(end + 1);
            }
            from = k + 1;
        }
        None
    }
}

/// An attribute's value: `"…"`, `'…'`, or (when `bare`) a run without blanks and `>`.
fn attribute_value(s: &str, at: usize, bare: bool) -> Option<usize> {
    let b = s.as_bytes();
    if let Some(&q @ (b'"' | b'\'')) = b.get(at)
        && let Some(k) = s[at + 1..].find(q as char)
    {
        return Some(at + 1 + k + 1);
    }
    if !bare {
        return None;
    }
    let mut end = at;
    while end < s.len() && space_at(s, end).is_none() && b[end] != b'>' {
        end += s[end..].chars().next().map_or(1, char::len_utf8);
    }
    (end > at).then_some(end)
}

/// `\s+on[a-z]+\s*=\s*("[^"]*"|'[^']*'|[^\s>]+)`: an event handler with the blanks before it.
fn handler(s: &str, at: usize) -> Option<usize> {
    space_at(s, at)?;
    let name = skip_space(s, at);
    if !starts_ci(s, name, "on") {
        return None;
    }
    let mut k = name + 2;
    while s.as_bytes().get(k).is_some_and(u8::is_ascii_alphabetic) {
        k += 1;
    }
    if k == name + 2 {
        return None;
    }
    let eq = skip_space(s, k);
    if s.as_bytes().get(eq) != Some(&b'=') {
        return None;
    }
    attribute_value(s, skip_space(s, eq + 1), true)
}

/// `\s+(?:xlink:)?href\s*=\s*("(?!#|data:image/)[^"]*"|'(?!#|data:image/)[^']*')`:
/// a link out of the file, with the blanks before it.
fn outside_link(s: &str, at: usize) -> Option<usize> {
    space_at(s, at)?;
    let mut k = skip_space(s, at);
    if starts_ci(s, k, "xlink:") {
        k += 6;
    }
    if !starts_ci(s, k, "href") {
        return None;
    }
    let eq = skip_space(s, k + 4);
    if s.as_bytes().get(eq) != Some(&b'=') {
        return None;
    }
    let v = skip_space(s, eq + 1);
    if !matches!(s.as_bytes().get(v), Some(b'"' | b'\'')) {
        return None;
    }
    if s.as_bytes().get(v + 1) == Some(&b'#') || starts_ci(s, v + 1, "data:image/") {
        return None;
    }
    attribute_value(s, v, false)
}

/// `url\(\s*['"]?(?!#)[^)]*\)`: a `url()` that does not point inside the file.
/// Through the pattern's backtracking only a `#` right after the parenthesis keeps it.
fn outside_url(s: &str, at: usize) -> Option<usize> {
    if !starts_ci(s, at, "url(") {
        return None;
    }
    let open = at + 4;
    if s.as_bytes().get(open) == Some(&b'#') {
        return None;
    }
    s[open..].find(')').map(|k| open + k + 1)
}

/// An SVG drawing made safe to keep and draw: no scripts, no event handlers,
/// no foreign objects, no references outside the file (`sanitizeSvg`).
pub fn sanitize_svg(svg: &str) -> String {
    let s = replace_all(svg, "", tag("<?xml"));
    let s = replace_all(&s, "", tag("<!DOCTYPE"));
    let s = replace_all(&s, "", element("<script", "</script"));
    let s = replace_all(&s, "", |s, at| {
        tag("<script")(s, at).filter(|end| s.as_bytes().get(end - 2) == Some(&b'/'))
    });
    let s = replace_all(&s, "", element("<foreignObject", "</foreignObject"));
    let s = replace_all(&s, "", handler);
    let s = replace_all(&s, "", outside_link);
    let s = replace_all(&s, "none", outside_url);
    trim(&s).to_owned()
}

// ── Reading a file ─────────────────────────────────────────────────────

fn validate_item(it: &Value, i: usize) -> Vec<String> {
    let w = format!("öğe {}", i + 1);
    if !it.is_object() {
        return vec![format!("{w}: nesne değil")];
    }
    let g = |k: &str| it.get(k);
    let mut issues = Vec::new();
    if g("id").and_then(Value::as_str).is_none_or(str::is_empty) {
        issues.push(format!("{w}: kimlik yok"));
    }
    if !g("name").is_some_and(Value::is_string) {
        issues.push(format!("{w}: ad yok"));
    }
    if !g("path")
        .and_then(Value::as_array)
        .is_some_and(|p| p.iter().all(Value::is_string))
    {
        issues.push(format!("{w}: kategori yolu metin listesi olmalı"));
    }
    match g("kind").and_then(Value::as_str) {
        Some("symbol") => issues.extend(validate_symbol(
            g("symbol").unwrap_or(&Value::Null),
            &format!("{w} ({})", js_str(g("name"))),
        )),
        Some("asset") => {
            let format = js_str(g("format"));
            if !matches!(format.as_str(), "svg" | "png" | "jpeg") {
                issues.push(format!("{w}: bilinmeyen varlık biçimi"));
            }
            match g("data").and_then(Value::as_str) {
                None => issues.push(format!("{w}: varlık verisi yok")),
                Some(data) if format == "svg" => {
                    // `/<svg[\s>]/i`: any `<svg` followed by a blank or `>`.
                    let mut from = 0;
                    let mut svg = false;
                    while let Some(k) = find_ci(data, "<svg", from) {
                        from = k + 1;
                        if data
                            .get(k + 4..)
                            .and_then(|r| r.chars().next())
                            .is_some_and(|c| c == '>' || is_space(c))
                        {
                            svg = true;
                            break;
                        }
                    }
                    if !svg {
                        issues.push(format!("{w}: SVG çizimi değil"));
                    }
                }
                Some(data) => {
                    let ok = ["data:image/png;base64,", "data:image/jpeg;base64,"]
                        .iter()
                        .any(|p| data.starts_with(p));
                    if !ok {
                        issues.push(format!("{w}: görüntü verisi data: adresi olmalı"));
                    }
                }
            }
            let dim = |k: &str| g(k).and_then(Value::as_f64).is_some_and(|x| x > 0.0);
            let numbers = g("width").is_some_and(Value::is_number)
                && g("height").is_some_and(Value::is_number);
            if !numbers || !dim("width") || !dim("height") {
                issues.push(format!("{w}: boyut yok"));
            }
        }
        _ => issues.push(format!("{w}: bilinmeyen öğe türü “{}”", js_str(g("kind")))),
    }
    issues
}

/// Problems in a file's categories (absent is fine): paths of text, each with an optional order and description.
fn validate_categories(list: Option<&Value>) -> Vec<String> {
    let Some(list) = list else {
        return Vec::new();
    };
    let Some(list) = list.as_array() else {
        return vec!["Dosyadaki kategoriler bir liste değil.".into()];
    };
    list.iter()
        .enumerate()
        .flat_map(|(i, c)| {
            let w = format!("kategori {}", i + 1);
            if !c.is_object() {
                return vec![format!("{w}: nesne değil")];
            }
            let mut issues = Vec::new();
            if !c
                .get("path")
                .and_then(Value::as_array)
                .is_some_and(|p| p.iter().all(Value::is_string))
            {
                issues.push(format!("{w}: yol metin listesi olmalı"));
            }
            if c.get("order")
                .is_some_and(|o| o.as_f64().is_none_or(|x| !x.is_finite()))
            {
                issues.push(format!("{w}: sıra bir sayı olmalı"));
            }
            if c.get("description").is_some_and(|d| !d.is_string()) {
                issues.push(format!("{w}: açıklama metin olmalı"));
            }
            issues
        })
        .collect()
}

/// A .kstil file that was read and checked: its items (SVG drawings cleaned) and categories.
#[derive(Clone, Debug, PartialEq)]
pub struct StyleFile {
    pub items: Vec<Value>,
    /// Only what a category is: its path, order and description.
    pub categories: Option<Vec<Value>>,
}

/// Parses and checks a file's text; the file only when nothing is wrong (`parseStyleFile`).
pub fn parse_style_file(text: &str) -> Result<StyleFile, Vec<String>> {
    let data: Value =
        serde_json::from_str(text).map_err(|_| vec!["Dosya okunamadı: JSON değil.".to_owned()])?;
    if !data.is_object() || data.get("format").and_then(Value::as_str) != Some(STYLE_FORMAT) {
        return Err(vec!["Bu bir KentOS stil dosyası (.kstil) değil.".into()]);
    }
    // A version is a whole number from 1: without one the file is broken, not newer.
    let version = data
        .get("version")
        .and_then(Value::as_f64)
        .filter(|v| v.fract() == 0.0 && *v >= 1.0);
    let Some(version) = version else {
        return Err(vec!["Dosyada sürüm yok; dosya bozuk olabilir.".into()]);
    };
    if version > f64::from(STYLE_VERSION) {
        return Err(vec![format!(
            "Dosya daha yeni bir KentOS sürümüyle yazılmış (sürüm {}); güncelleyip yeniden deneyin.",
            number::to_string(version)
        )]);
    }
    let Some(items) = data.get("items").and_then(Value::as_array) else {
        return Err(vec!["Dosyada öğe listesi yok.".into()]);
    };
    let mut issues: Vec<String> = items
        .iter()
        .enumerate()
        .flat_map(|(i, it)| validate_item(it, i))
        .collect();
    issues.extend(validate_categories(data.get("categories")));
    if !issues.is_empty() {
        return Err(issues);
    }
    let items = items
        .iter()
        .map(|it| {
            let mut it = it.clone();
            if it.get("kind").and_then(Value::as_str) == Some("asset")
                && it.get("format").and_then(Value::as_str) == Some("svg")
                && let Some(data) = it.get("data").and_then(Value::as_str)
            {
                let clean = sanitize_svg(data);
                it["data"] = Value::from(clean);
            }
            it
        })
        .collect();
    let categories = data
        .get("categories")
        .and_then(Value::as_array)
        .map(|list| {
            list.iter()
                .map(|c| {
                    let mut out = Map::new();
                    out.insert("path".into(), c.get("path").cloned().unwrap_or(json!([])));
                    if let Some(o) = c.get("order") {
                        out.insert("order".into(), o.clone());
                    }
                    if let Some(d) = c.get("description") {
                        out.insert("description".into(), d.clone());
                    }
                    Value::Object(out)
                })
                .collect()
        });
    Ok(StyleFile { items, categories })
}

// ── Taking a file in ───────────────────────────────────────────────────

/// What to do with an item whose id the library has.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConflictMode {
    /// Kopya olarak al: under a new id.
    Copy,
    /// Üzerine yaz: only in the same source; elsewhere skipped.
    Replace,
    /// Atla.
    Skip,
}

/// What an import did; `renamed`: ids a copy took (old → new), in order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ImportReport {
    pub added: usize,
    pub replaced: usize,
    pub skipped: usize,
    pub renamed: Vec<(String, String)>,
}

/// A symbol with its asset ids renamed.
fn rename_assets(v: &mut Value, renamed: &[(String, String)]) {
    match v {
        Value::Object(o) => {
            for (k, x) in o.iter_mut() {
                if k == "asset"
                    && let Some(id) = x.as_str()
                    && let Some((_, to)) = renamed.iter().find(|(from, _)| from == id)
                {
                    *x = Value::from(to.clone());
                } else {
                    rename_assets(x, renamed);
                }
            }
        }
        Value::Array(a) => a.iter_mut().for_each(|x| rename_assets(x, renamed)),
        _ => {}
    }
}

/// Adds a file's items to the user's or the project's library
/// (`importStyles`): assets first; an id the library has is replaced (only
/// in the same editable source), taken as a copy under a new id (symbols
/// then point at the renamed assets), or skipped. System items are never replaced.
pub fn import_styles(
    lib: &mut StyleLibrary,
    file: &StyleFile,
    to: Source,
    mode: ConflictMode,
) -> ImportReport {
    let mut report = ImportReport::default();
    let is_asset = |v: &Value| v.get("kind").and_then(Value::as_str) == Some("asset");
    let assets_first = file
        .items
        .iter()
        .filter(|v| is_asset(v))
        .chain(file.items.iter().filter(|v| !is_asset(v)));
    for raw in assets_first {
        let mut item = raw.clone();
        let id = raw
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned();
        if let Some((_, existing)) = lib.get(&id) {
            if mode == ConflictMode::Skip || (mode == ConflictMode::Replace && existing != to) {
                report.skipped += 1;
                continue;
            }
            if mode == ConflictMode::Replace {
                let mut patch = item.as_object().cloned().unwrap_or_default();
                patch.remove("id");
                patch.remove("kind");
                if lib.update(&id, &patch).is_ok() {
                    report.replaced += 1;
                }
                continue;
            }
            let fresh = new_item_id(if to == Source::Project { "p" } else { "u" });
            report.renamed.push((id.clone(), fresh.clone()));
            item["id"] = Value::from(fresh);
        }
        if item.get("kind").and_then(Value::as_str) == Some("symbol")
            && !report.renamed.is_empty()
            && let Some(symbol) = item.get_mut("symbol")
        {
            rename_assets(symbol, &report.renamed);
        }
        if lib.add(to, item).is_ok() {
            report.added += 1;
        }
    }
    for c in file.categories.iter().flatten() {
        lib.add_category(to, c.clone());
    }
    report
}

// ── A new drawing ──────────────────────────────────────────────────────

/// Past a run of bytes that `ok` takes, from `at`; None when there is none.
fn run(s: &str, at: usize, ok: impl Fn(u8) -> bool) -> Option<usize> {
    let b = s.as_bytes();
    let mut k = at;
    while k < b.len() && ok(b[k]) {
        k += 1;
    }
    (k > at).then_some(k)
}

fn number_char(c: u8) -> bool {
    c.is_ascii_digit() || c == b'.'
}

/// Past `[\s,]+` from `at`.
fn separators(s: &str, at: usize) -> Option<usize> {
    let mut k = at;
    loop {
        if s.as_bytes().get(k) == Some(&b',') {
            k += 1;
        } else if let Some(n) = space_at(s, k) {
            k += n;
        } else {
            break;
        }
    }
    (k > at).then_some(k)
}

/// The viewBox's width and height as written (`viewBox="x y w h"`), from the first that reads.
fn view_box_size(s: &str) -> Option<(&str, &str)> {
    let mut from = 0;
    while let Some(k) = find_ci(s, "viewBox", from) {
        from = k + 1;
        let read = || -> Option<(&str, &str)> {
            let eq = skip_space(s, k + 7);
            (s.as_bytes().get(eq) == Some(&b'=')).then_some(())?;
            let q = skip_space(s, eq + 1);
            matches!(s.as_bytes().get(q), Some(b'"' | b'\'')).then_some(())?;
            let a = skip_space(s, q + 1);
            let a = run(s, a, |c| number_char(c) || c == b'-')?;
            let a = separators(s, a)?;
            let a = run(s, a, |c| number_char(c) || c == b'-')?;
            let a = separators(s, a)?;
            let w_end = run(s, a, number_char)?;
            let b = separators(s, w_end)?;
            let h_end = run(s, b, number_char)?;
            Some((&s[a..w_end], &s[b..h_end]))
        };
        if let Some(size) = read() {
            return Some(size);
        }
    }
    None
}

/// `\s<name>\s*=\s*["']([\d.]+)`: the first such attribute's number as written.
fn attribute_number<'s>(s: &'s str, name: &str) -> Option<&'s str> {
    let mut from = 0;
    while let Some(k) = find_ci(s, name, from) {
        from = k + 1;
        // A blank right before the name.
        let blank = s[..k].chars().next_back().is_some_and(is_space);
        if !blank {
            continue;
        }
        let eq = skip_space(s, k + name.len());
        if s.as_bytes().get(eq) != Some(&b'=') {
            continue;
        }
        let q = skip_space(s, eq + 1);
        if !matches!(s.as_bytes().get(q), Some(b'"' | b'\'')) {
            continue;
        }
        if let Some(end) = run(s, q + 1, number_char) {
            return Some(&s[q + 1..end]);
        }
    }
    None
}

/// A new asset from an SVG text (cleaned) with its size from the viewBox, or
/// width and height, or 100 (`svgAsset`).
pub fn svg_asset(name: &str, path: &[String], svg: &str, id: &str) -> Value {
    let clean = sanitize_svg(svg);
    let vb = view_box_size(&clean);
    let size = |from_box: Option<&str>, attr: &str| {
        let text = from_box.or_else(|| attribute_number(&clean, attr));
        let v = text.map_or(100.0, js_number);
        if v == 0.0 || v.is_nan() { 100.0 } else { v }
    };
    let width = size(vb.map(|v| v.0), "width");
    let height = size(vb.map(|v| v.1), "height");
    json!({
        "kind": "asset", "id": id, "name": name, "path": path, "format": "svg",
        "data": clean, "width": width, "height": height,
    })
}

/// The kind a library item is shown as in the manager: a symbol's type, or a drawing.
pub fn kind_of(kind: ItemKind, symbol: Option<&Value>) -> &'static str {
    match (
        kind,
        symbol.and_then(|s| s.get("type")).and_then(Value::as_str),
    ) {
        (ItemKind::Asset, _) => "asset",
        (_, Some("fill")) => "fill",
        (_, Some("line")) => "line",
        _ => "marker",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizing_keeps_inner_links_and_images() {
        let s = sanitize_svg(
            "<?xml version=\"1.0\"?>\n<svg onload=alert(1) xmlns=\"http://www.w3.org/2000/svg\"><script>x</script ><script src='y'/><use href=\"#a\"/><image xlink:href='data:image/png;base64,AA'/><a href=\"http://x\">t</a><rect style=\"fill:url(http://x)\"/><rect style=\"fill:url(#g)\"/></svg>",
        );
        assert_eq!(
            s,
            "<svg xmlns=\"http://www.w3.org/2000/svg\"><use href=\"#a\"/><image xlink:href='data:image/png;base64,AA'/><a>t</a><rect style=\"fill:none\"/><rect style=\"fill:url(#g)\"/></svg>"
        );
    }

    #[test]
    fn iso_time_reads_as_javascript_writes_it() {
        let t = iso_now();
        assert_eq!(t.len(), 24, "{t}");
        assert!(t.ends_with('Z') && t.as_bytes()[10] == b'T');
    }
}
