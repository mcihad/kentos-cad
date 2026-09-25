//! The drawing model's geometry (`apps/web/src/style/svg/svgModel.ts`,
//! docs/STYLE.md §7): a shape as an SVG element and the drawing as an SVG
//! file, rectangles and ellipses as paths, boxes, and shapes under affine
//! maps. Rectangles, ellipses and texts keep their kind while a map allows
//! it (so they stay editable as such); anything else becomes a path.

use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::{
    PI, atan2, cos, js_hypot, js_max, js_min, js_round, sin, truthy,
};
use kentos_style_core::js::number;

use crate::path::{
    Matrix, apply, empty_box, grow_box, path_data_of, sub_paths_box, transform_sub_paths,
};
use crate::shape::{Obj, PathNode, SubPath};

/// A number as the SVG file writes it: three decimals at most.
pub fn n(v: f64) -> String {
    let s = number::to_string(js_round(v * 1000.0) / 1000.0);
    if s == "-0" { "0".to_string() } else { s }
}

/// Width of an SVG text in em, in Arial's measures; an empty one counts as one letter (it can still be picked).
pub(crate) fn text_em(text: &str) -> f64 {
    use kentos_geometry_core::text::{Font, width_em};
    width_em(text, Font::from_id("arimo"))
}

fn font(name: &str) -> Option<&'static str> {
    match name {
        "sans" => Some("Arial, Helvetica, sans-serif"),
        "serif" => Some("\"Times New Roman\", Times, serif"),
        _ => None,
    }
}

/// A shape as an element: its tag, attributes in order and text.
pub struct Element {
    pub tag: &'static str,
    pub attrs: Vec<(String, String)>,
    pub text: Option<String>,
}

impl kentos_geometry_core::api::json::ToJson for Element {
    fn write_json(&self, out: &mut String) {
        use kentos_geometry_core::api::json::write_str;
        out.push_str("{\"tag\":");
        write_str(out, self.tag);
        out.push_str(",\"attrs\":{");
        for (i, (k, v)) in self.attrs.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            write_str(out, k);
            out.push(':');
            write_str(out, v);
        }
        out.push('}');
        if let Some(t) = &self.text {
            out.push_str(",\"text\":");
            write_str(out, t);
        }
        out.push('}');
    }
}

struct Attrs(Vec<(String, String)>);

impl Attrs {
    fn set(&mut self, k: &str, v: String) {
        match self.0.iter_mut().find(|(key, _)| key == k) {
            Some(slot) => slot.1 = v,
            None => self.0.push((k.to_string(), v)),
        }
    }
    /// `attrs[k] ??= v`.
    fn default(&mut self, k: &str, v: &str) {
        if !self.0.iter().any(|(key, _)| key == k) {
            self.0.push((k.to_string(), v.to_string()));
        }
    }
    fn remove(&mut self, k: &str) {
        self.0.retain(|(key, _)| key != k);
    }
}

/// A text field's value as a string attribute (`String(v)` for numbers).
fn attr_text(v: &Json) -> String {
    match v {
        Json::Str(s) => s.clone(),
        Json::Num(x) => number::to_string(*x),
        Json::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// A shape as an element: tag and attributes (the view builds DOM from it,
/// the file writes it). `fill` and `stroke` are the shape's paints as the
/// caller shows them (the symbol's colours as KentOS parameters in a file,
/// the editor's colours on its canvas).
pub fn element_of(s: &Obj, fill: &str, stroke: &str) -> Result<Option<Element>, String> {
    let mut attrs = Attrs(vec![
        ("fill".to_string(), fill.to_string()),
        ("stroke".to_string(), stroke.to_string()),
    ]);
    let no_stroke = s.text("stroke") == Some("none");
    if !no_stroke {
        attrs.set("stroke-width", n(s.num("strokeWidth")));
    }
    if let Some(o) = s.opt_num("opacity")
        && o < 1.0
    {
        attrs.set("opacity", n(o));
    }
    if !no_stroke
        && let Json::Arr(d) = s.get("dash")
        && !d.is_empty()
    {
        let parts: Vec<String> = d.iter().map(|v| n(f64::from_json_or_nan(v))).collect();
        attrs.set("stroke-dasharray", parts.join(" "));
    }
    if s.is("cap") {
        attrs.set("stroke-linecap", attr_text(s.get("cap")));
    }
    if s.is("join") {
        attrs.set("stroke-linejoin", attr_text(s.get("join")));
    }
    if s.is("fillRule") {
        attrs.set("fill-rule", attr_text(s.get("fillRule")));
    }
    let rotate = |attrs: &mut Attrs, cx: f64, cy: f64| {
        if s.is("rotate") {
            attrs.set(
                "transform",
                format!("rotate({} {} {})", n(s.num("rotate")), n(cx), n(cy)),
            );
        }
    };
    match s.kind() {
        "rect" => {
            let (x, y, w, h) = (s.num("x"), s.num("y"), s.num("w"), s.num("h"));
            attrs.set("x", n(x));
            attrs.set("y", n(y));
            attrs.set("width", n(w));
            attrs.set("height", n(h));
            if s.is("r") {
                attrs.set("rx", n(s.num("r")));
            }
            rotate(&mut attrs, x + w / 2.0, y + h / 2.0);
            Ok(Some(Element {
                tag: "rect",
                attrs: attrs.0,
                text: None,
            }))
        }
        "ellipse" => {
            let (cx, cy) = (s.num("cx"), s.num("cy"));
            attrs.set("cx", n(cx));
            attrs.set("cy", n(cy));
            attrs.set("rx", n(s.num("rx")));
            attrs.set("ry", n(s.num("ry")));
            rotate(&mut attrs, cx, cy);
            Ok(Some(Element {
                tag: "ellipse",
                attrs: attrs.0,
                text: None,
            }))
        }
        "path" => {
            attrs.set("d", path_data_of(&s.subs()?, 3.0));
            attrs.default("fill-rule", "evenodd");
            attrs.default("stroke-linejoin", "round");
            attrs.default("stroke-linecap", "round");
            Ok(Some(Element {
                tag: "path",
                attrs: attrs.0,
                text: None,
            }))
        }
        "text" => {
            let (x, y) = (s.num("x"), s.num("y"));
            attrs.set("x", n(x));
            attrs.set("y", n(y));
            if let Some(f) = s.text("font").and_then(font) {
                attrs.set("font-family", f.to_string());
            }
            attrs.set("font-size", n(s.num("size")));
            attrs.set("font-weight", attr_text(s.get("weight")));
            attrs.set("text-anchor", attr_text(s.get("anchor")));
            if no_stroke {
                attrs.remove("stroke");
            }
            rotate(&mut attrs, x, y);
            Ok(Some(Element {
                tag: "text",
                attrs: attrs.0,
                text: Some(attr_text(s.get("text"))),
            }))
        }
        _ => Ok(None),
    }
}

trait NumOrNan {
    fn from_json_or_nan(v: &Json) -> f64;
}

impl NumOrNan for f64 {
    fn from_json_or_nan(v: &Json) -> f64 {
        <f64 as kentos_geometry_core::api::json::FromJson>::from_json(v).unwrap_or(f64::NAN)
    }
}

fn esc(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// A paint as an SVG attribute value (the symbol's colours as KentOS parameters).
pub fn paint_value(p: &str) -> &str {
    match p {
        "fill" => "currentColor",
        "stroke" => "param(stroke) #000000",
        _ => p,
    }
}

/// The drawing as an SVG file (groups become <g>, hidden shapes are left out).
pub fn serialize_doc(width: f64, height: f64, shapes: &[Obj]) -> Result<String, String> {
    let mut out = format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" viewBox=\"0 0 {} {}\" width=\"{}\" height=\"{}\">",
        n(width),
        n(height),
        n(width),
        n(height)
    );
    let mut open: Option<String> = None;
    for s in shapes {
        if s.is("hidden") {
            continue;
        }
        let group = s.text("group").map(str::to_string);
        if group != open {
            if open.as_deref().is_some_and(|g| !g.is_empty()) {
                out.push_str("</g>");
            }
            if let Some(g) = group.as_deref().filter(|g| !g.is_empty()) {
                out.push_str(&format!("<g data-group=\"{}\">", esc(g)));
            }
            open = group;
        }
        let fill = paint_value(s.text("fill").unwrap_or(""));
        let stroke = paint_value(s.text("stroke").unwrap_or(""));
        let Some(e) = element_of(s, fill, stroke)? else {
            return Err("Bilinmeyen şekil türü.".into());
        };
        let attrs: Vec<String> = e
            .attrs
            .iter()
            .map(|(k, v)| format!("{k}=\"{}\"", esc(v)))
            .collect();
        match &e.text {
            Some(t) => out.push_str(&format!(
                "<{} {}>{}</{}>",
                e.tag,
                attrs.join(" "),
                esc(t),
                e.tag
            )),
            None => out.push_str(&format!("<{} {}/>", e.tag, attrs.join(" "))),
        }
    }
    if open.as_deref().is_some_and(|g| !g.is_empty()) {
        out.push_str("</g>");
    }
    out.push_str("</svg>");
    Ok(out)
}

// ── Geometry ───────────────────────────────────────────────────────────

fn rad(deg: f64) -> f64 {
    (deg * PI) / 180.0
}

/// A rotation by `deg` degrees about (cx, cy).
pub fn rotate_about(deg: f64, cx: f64, cy: f64) -> Matrix {
    let c = cos(rad(deg));
    let s = sin(rad(deg));
    [c, s, -s, c, cx - c * cx + s * cy, cy - s * cx - c * cy]
}

fn node(x: f64, y: f64, in_: Option<[f64; 2]>, out: Option<[f64; 2]>) -> PathNode {
    PathNode {
        x,
        y,
        in_,
        out,
        ty: None,
    }
}

/// The fields a rectangle or ellipse keeps as a path, in the TypeScript's order.
const BASE: [&str; 12] = [
    "id",
    "fill",
    "stroke",
    "strokeWidth",
    "opacity",
    "dash",
    "cap",
    "join",
    "fillRule",
    "group",
    "hidden",
    "name",
];

/// A rectangle or ellipse as a path (for transforms they cannot keep); paths and texts as they are.
pub fn to_path(s: &Obj) -> Obj {
    let kind = s.kind();
    if kind == "path" || kind == "text" {
        return s.clone();
    }
    // `{ id: s.id, fill: s.fill, … }`: every field, undefined ones too.
    let mut out = Obj(Vec::new());
    for k in BASE {
        out.put(k, s.field(k));
    }
    let mut subs: Vec<SubPath>;
    if kind == "rect" {
        let r = js_min(
            js_min(s.opt_num("r").unwrap_or(0.0), s.num("w") / 2.0),
            s.num("h") / 2.0,
        );
        let k = 0.5523 * r;
        let (x, y, w, h) = (s.num("x"), s.num("y"), s.num("w"), s.num("h"));
        let nodes = if truthy(r) {
            vec![
                node(x + r, y, Some([x + r - k, y]), None),
                node(x + w - r, y, None, Some([x + w - r + k, y])),
                node(x + w, y + r, Some([x + w, y + r - k]), None),
                node(x + w, y + h - r, None, Some([x + w, y + h - r + k])),
                node(x + w - r, y + h, Some([x + w - r + k, y + h]), None),
                node(x + r, y + h, None, Some([x + r - k, y + h])),
                node(x, y + h - r, Some([x, y + h - r + k]), None),
                node(x, y + r, None, Some([x, y + r - k])),
            ]
        } else {
            vec![
                node(x, y, None, None),
                node(x + w, y, None, None),
                node(x + w, y + h, None, None),
                node(x, y + h, None, None),
            ]
        };
        subs = vec![SubPath {
            closed: true,
            nodes,
        }];
        if s.is("rotate") {
            subs = transform_sub_paths(
                &subs,
                &rotate_about(s.num("rotate"), x + w / 2.0, y + h / 2.0),
            );
        }
    } else {
        let (cx, cy, rx, ry) = (s.num("cx"), s.num("cy"), s.num("rx"), s.num("ry"));
        let kx = 0.5523 * rx;
        let ky = 0.5523 * ry;
        subs = vec![SubPath {
            closed: true,
            nodes: vec![
                node(
                    cx + rx,
                    cy,
                    Some([cx + rx, cy - ky]),
                    Some([cx + rx, cy + ky]),
                ),
                node(
                    cx,
                    cy + ry,
                    Some([cx + kx, cy + ry]),
                    Some([cx - kx, cy + ry]),
                ),
                node(
                    cx - rx,
                    cy,
                    Some([cx - rx, cy + ky]),
                    Some([cx - rx, cy - ky]),
                ),
                node(
                    cx,
                    cy - ry,
                    Some([cx - kx, cy - ry]),
                    Some([cx + kx, cy - ry]),
                ),
            ],
        }];
        if s.is("rotate") {
            subs = transform_sub_paths(&subs, &rotate_about(s.num("rotate"), cx, cy));
        }
    }
    out.set_text("kind", "path");
    out.set_subs(&subs);
    out
}

/// Text is measured in Arial's widths (the Arimo table of `kentos_geometry_core::text`): the editor's
/// text is Arial or the browser's sans default; a bold face is a little wider. Kerning is left out.
fn text_box(s: &Obj) -> Bounds {
    let size = s.num("size");
    let w = size * text_em(s.text("text").unwrap_or("")) * (if s.num("weight") >= 700.0 { 1.08 } else { 1.0 });
    let (x, y) = (s.num("x"), s.num("y"));
    let x0 = match s.text("anchor") {
        Some("start") => x,
        Some("middle") => x - w / 2.0,
        _ => x - w,
    };
    let mut b = empty_box();
    let corners = [
        [x0, y - size * 0.75],
        [x0 + w, y - size * 0.75],
        [x0 + w, y + size * 0.22],
        [x0, y + size * 0.22],
    ];
    let m = if s.is("rotate") {
        Some(rotate_about(s.num("rotate"), x, y))
    } else {
        None
    };
    for [cx, cy] in corners {
        let p = match &m {
            Some(m) => apply(m, cx, cy),
            None => [cx, cy],
        };
        grow_box(&mut b, p[0], p[1]);
    }
    b
}

pub fn shape_box(s: &Obj) -> Result<Bounds, String> {
    match s.kind() {
        "text" => Ok(text_box(s)),
        "path" => Ok(sub_paths_box(&s.subs()?)),
        kind => {
            if !s.is("rotate") {
                return Ok(if kind == "rect" {
                    Bounds {
                        min_x: s.num("x"),
                        min_y: s.num("y"),
                        max_x: s.num("x") + s.num("w"),
                        max_y: s.num("y") + s.num("h"),
                    }
                } else {
                    Bounds {
                        min_x: s.num("cx") - s.num("rx"),
                        min_y: s.num("cy") - s.num("ry"),
                        max_x: s.num("cx") + s.num("rx"),
                        max_y: s.num("cy") + s.num("ry"),
                    }
                });
            }
            shape_box(&to_path(s))
        }
    }
}

pub fn shapes_box(shapes: &[Obj]) -> Result<Option<Bounds>, String> {
    let mut b = empty_box();
    for s in shapes {
        let sb = shape_box(s)?;
        grow_box(&mut b, sb.min_x, sb.min_y);
        grow_box(&mut b, sb.max_x, sb.max_y);
    }
    Ok(if b.min_x.is_finite() { Some(b) } else { None })
}

/// `(s.rotate ?? 0) + angle`.
fn rotate_plus(s: &Obj, angle: f64) -> Json {
    Json::Num(s.opt_num("rotate").unwrap_or(0.0) + angle)
}

/// A shape under an affine map. Translations and scalings without rotation
/// keep rectangles and ellipses; rotations keep them when uniform (the
/// rotate property grows); anything else turns them into paths. Text moves
/// and scales its size. None for a shape of no known kind.
pub fn transform_shape(s: &Obj, m: &Matrix) -> Result<Option<Obj>, String> {
    let sx = js_hypot(m[0], m[1]);
    let sy = js_hypot(m[2], m[3]);
    let angle = (atan2(m[1], m[0]) * 180.0) / PI;
    let axis_aligned = m[1].abs() < 1e-12 && m[2].abs() < 1e-12;
    let uniform = (sx - sy).abs() < 1e-9
        && (m[0] * m[2] + m[1] * m[3]).abs() < 1e-9
        && m[0] * m[3] - m[1] * m[2] > 0.0;
    let mut out = s.clone();
    match s.kind() {
        "path" => {
            out.set_subs(&transform_sub_paths(&s.subs()?, m));
            Ok(Some(out))
        }
        "text" => {
            let [x, y] = apply(m, s.num("x"), s.num("y"));
            out.set_num("x", x);
            out.set_num("y", y);
            out.set_num(
                "size",
                s.num("size") * (m[0] * m[3] - m[1] * m[2]).abs().sqrt(),
            );
            if axis_aligned {
                out.put("rotate", s.field("rotate"));
            } else {
                out.set("rotate", rotate_plus(s, angle));
            }
            Ok(Some(out))
        }
        "rect" => {
            if axis_aligned && m[0] > 0.0 && m[3] > 0.0 && !s.is("rotate") {
                let [x, y] = apply(m, s.num("x"), s.num("y"));
                out.set_num("x", x);
                out.set_num("y", y);
                out.set_num("w", s.num("w") * m[0]);
                out.set_num("h", s.num("h") * m[3]);
                out.put(
                    "r",
                    s.is("r")
                        .then(|| Json::Num(s.num("r") * js_min(m[0], m[3]))),
                );
                return Ok(Some(out));
            }
            if uniform {
                let [cx, cy] = apply(
                    m,
                    s.num("x") + s.num("w") / 2.0,
                    s.num("y") + s.num("h") / 2.0,
                );
                let w = s.num("w") * sx;
                let h = s.num("h") * sx;
                out.set_num("x", cx - w / 2.0);
                out.set_num("y", cy - h / 2.0);
                out.set_num("w", w);
                out.set_num("h", h);
                out.put("r", s.is("r").then(|| Json::Num(s.num("r") * sx)));
                out.set("rotate", rotate_plus(s, angle));
                return Ok(Some(out));
            }
            transform_shape(&to_path(s), m)
        }
        "ellipse" => {
            if axis_aligned && !s.is("rotate") {
                let [cx, cy] = apply(m, s.num("cx"), s.num("cy"));
                out.set_num("cx", cx);
                out.set_num("cy", cy);
                out.set_num("rx", s.num("rx") * m[0].abs());
                out.set_num("ry", s.num("ry") * m[3].abs());
                return Ok(Some(out));
            }
            if uniform {
                let [cx, cy] = apply(m, s.num("cx"), s.num("cy"));
                out.set_num("cx", cx);
                out.set_num("cy", cy);
                out.set_num("rx", s.num("rx") * sx);
                out.set_num("ry", s.num("ry") * sx);
                out.set("rotate", rotate_plus(s, angle));
                return Ok(Some(out));
            }
            transform_shape(&to_path(s), m)
        }
        _ => Ok(None),
    }
}

/// Maps box `a` onto box `b` (scaling about their corners).
pub fn box_to_box(a: &Bounds, b: &Bounds) -> Matrix {
    let sx = (b.max_x - b.min_x) / js_max(a.max_x - a.min_x, 1e-9);
    let sy = (b.max_y - b.min_y) / js_max(a.max_y - a.min_y, 1e-9);
    [
        sx,
        0.0,
        0.0,
        sy,
        b.min_x - a.min_x * sx,
        b.min_y - a.min_y * sy,
    ]
}

/// A regular polygon (or star with an inner radius) as a closed path, first corner up.
pub fn regular_polygon(cx: f64, cy: f64, r: f64, sides: f64, inner: Option<f64>) -> SubPath {
    let star = inner.is_some_and(truthy);
    let pts = if star { sides * 2.0 } else { sides };
    // `Array.from({ length })`: a whole, non-negative count.
    let count = if pts > 0.0 && pts.is_finite() {
        pts.floor() as usize
    } else {
        0
    };
    SubPath {
        closed: true,
        nodes: (0..count)
            .map(|i| {
                let a = -PI / 2.0 + (i as f64 * 2.0 * PI) / pts;
                let rr = match inner {
                    Some(v) if star && i % 2 == 1 => v,
                    _ => r,
                };
                PathNode::at(cx + cos(a) * rr, cy + sin(a) * rr)
            })
            .collect(),
    }
}
