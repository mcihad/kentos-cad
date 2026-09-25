//! An SVG file read into the editor's model
//! (`apps/web/src/style/svg/importSvg.ts`). The page parses the XML into
//! plain nodes; this walks them the way a browser would: CSS `<style>`
//! rules (class, id, tag, descendant and child selectors), presentation
//! attributes and `style` with inheritance, every transform, nested
//! `<svg>`, `<defs>`/`<symbol>`/`<use>`, viewBox with units and
//! preserveAspectRatio, and all basic shapes, paths and texts. What the
//! model cannot hold is simplified and counted in the report: gradients and
//! patterns become one flat colour; clip paths, masks, filters, markers and
//! images are left out. Also the drawing's colours (`color_usage`) and
//! their mapping to the symbol's colours (`map_colors`).
//!
//! New shapes and groups are named "\u{1}k" and "\u{2}k" in the order the
//! TypeScript made their ids; the page makes the ids (their text holds the
//! time). The walk keeps its own stack: a deep file never meets the call
//! stack.

use std::collections::{HashMap, HashSet};
use std::rc::Rc;

use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::jsmath::{js_hypot, js_max, js_min, js_round, or, stable_sort, truthy};
use kentos_style_core::js::number;
use kentos_style_core::js::text::{is_space, slice, trim, utf16_len};

use crate::model::{shape_box, transform_shape};
use crate::path::{IDENTITY, Matrix, multiply, parse_path_data};
use crate::shape::{Obj, SubPath};
use crate::values::{
    Color, Flat, Length, PROPS, RawPaint, Rule, XmlNode, XmlTree, clamp01, is_near_black,
    js_number, matches, nums, parse_css, parse_decls, parse_float, parse_hex, parse_length, px_per,
    raw_paint, read_color, read_transform, split_on, starts_with_digit, to_user, view_box_of,
    view_box_transform, with_alpha,
};

// ── The walk ───────────────────────────────────────────────────────────

/// Inherited properties as computed.
#[derive(Clone, Debug)]
struct Computed {
    fill: RawPaint,
    stroke: RawPaint,
    stroke_width: f64,
    fill_opacity: f64,
    stroke_opacity: f64,
    dash: Option<Vec<f64>>,
    cap: &'static str,
    join: &'static str,
    fill_rule: &'static str,
    font_size: f64,
    font_family: String,
    font_weight: f64,
    anchor: &'static str,
    hidden: bool,
}

fn initial() -> Computed {
    Computed {
        fill: RawPaint::Plain(Flat::Color(Color {
            hex: "#000000".into(),
            alpha: 1.0,
        })),
        stroke: RawPaint::Plain(Flat::None),
        stroke_width: 1.0,
        fill_opacity: 1.0,
        stroke_opacity: 1.0,
        dash: None,
        cap: "butt",
        join: "miter",
        fill_rule: "nonzero",
        font_size: 16.0,
        font_family: String::new(),
        font_weight: 400.0,
        anchor: "start",
        hidden: false,
    }
}

const FONT_SIZES: [(&str, f64); 7] = [
    ("xx-small", 9.0),
    ("x-small", 10.0),
    ("small", 13.0),
    ("medium", 16.0),
    ("large", 18.0),
    ("x-large", 24.0),
    ("xx-large", 32.0),
];

/// What a file held that the drawing could not keep as it was.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Report {
    pub shapes: usize,
    /// `<use>` copies expanded.
    pub uses: usize,
    pub gradients: usize,
    pub patterns: usize,
    pub clips: usize,
    pub masks: usize,
    pub filters: usize,
    pub markers: usize,
    pub images: usize,
    /// Shapes whose fill and stroke opacity could not both be kept.
    pub approx_opacity: usize,
    /// References to elements that are not in the file (or refer to themselves).
    pub broken: usize,
    /// The file paints with the symbol's colours itself (currentColor / param()).
    pub symbol_paint: bool,
}

kentos_geometry_core::json_struct!(out Report {
    shapes, uses, gradients, patterns, clips, masks, filters, markers, images,
    approx_opacity => "approxOpacity", broken, symbol_paint => "symbolPaint",
});

/// A colour of the drawing: how much it paints (fill: box area; stroke: box perimeter × width) and how often.
#[derive(Clone, Debug, PartialEq)]
pub struct ColorUse {
    pub color: String,
    pub weight: f64,
    pub count: usize,
}

kentos_geometry_core::json_struct!(out ColorUse { color, weight, count });

/// A tracing reference kept in a drawing's file (docs/STYLE.md §7).
#[derive(Clone, Debug, PartialEq)]
pub struct ReferenceSpec {
    pub href: String,
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub opacity: f64,
    pub locked: bool,
    pub name: String,
}

kentos_geometry_core::json_struct!(out ReferenceSpec { href, x, y, width, height, opacity, locked, name });

/// A JSON value as the TypeScript's template literals and `String()` write it.
pub(crate) fn text_of(v: &Json) -> String {
    match v {
        Json::Str(s) => s.clone(),
        Json::Num(x) => number::to_string(*x),
        Json::Bool(b) => b.to_string(),
        Json::Null => "undefined".into(),
        _ => String::new(),
    }
}

impl FromJson for ReferenceSpec {
    fn from_json(v: &Json) -> Result<ReferenceSpec, String> {
        let n = |k: &str| f64::from_json(v.get(k)).unwrap_or(f64::NAN);
        Ok(ReferenceSpec {
            href: text_of(v.get("href")),
            x: n("x"),
            y: n("y"),
            width: n("width"),
            height: n("height"),
            opacity: n("opacity"),
            locked: crate::shape::truthy(v.get("locked")),
            name: text_of(v.get("name")),
        })
    }
}

/// Which colour becomes the symbol's colour.
#[derive(Clone, Debug, PartialEq)]
pub enum SymbolColor {
    /// Black, unless the file paints with the symbol's colours itself.
    Auto,
    /// 'black' (near-black colours), 'dominant', a hex, or none (null).
    Target(Option<String>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct ImportOptions {
    pub symbol_color: SymbolColor,
    /// The colour that becomes the second colour (param(stroke)).
    pub second_color: Option<String>,
    /// The editor's own source: element ids, `data-name`, `data-group` and hidden shapes are kept.
    pub editor: bool,
}

impl FromJson for ImportOptions {
    fn from_json(v: &Json) -> Result<ImportOptions, String> {
        let present = |k: &str| matches!(v, Json::Obj(f) if f.iter().any(|(n, _)| n == k));
        Ok(ImportOptions {
            symbol_color: match v.get("symbolColor") {
                Json::Str(s) if s == "auto" => SymbolColor::Auto,
                Json::Null if !present("symbolColor") => SymbolColor::Auto,
                Json::Null => SymbolColor::Target(None),
                other => SymbolColor::Target(Some(text_of(other))),
            },
            second_color: match v.get("secondColor") {
                Json::Null => None,
                other => Some(text_of(other)),
            },
            editor: crate::shape::truthy(v.get("editor")),
        })
    }
}

/// The imported drawing, what was left out, the report and the colours;
/// with the number of shape and group ids the page is to make.
pub struct Imported {
    pub doc: Obj,
    /// Element names that were left out.
    pub skipped: Vec<String>,
    pub report: Report,
    /// Fixed colours of the drawing (as read, before mapping), most painted first.
    pub colors: Vec<ColorUse>,
    pub reference: Option<ReferenceSpec>,
    pub ids: usize,
    pub groups: usize,
}

kentos_geometry_core::json_struct!(out Imported { doc, skipped, report, colors, reference, ids, groups });

/// Elements that draw nothing themselves.
const NOT_RENDERED: [&str; 26] = [
    "defs",
    "symbol",
    "clippath",
    "mask",
    "marker",
    "pattern",
    "lineargradient",
    "radialgradient",
    "filter",
    "style",
    "title",
    "desc",
    "metadata",
    "script",
    "namedview",
    "font",
    "font-face",
    "cursor",
    "view",
    "animate",
    "animatemotion",
    "animatetransform",
    "set",
    "mpath",
    "foreignobject",
    "solidcolor",
];

/// Properties as specified on an element (attributes, rules, `style`), in the order they were first set.
#[derive(Clone, Debug, Default)]
struct Props(Vec<(String, String)>);

impl Props {
    fn get(&self, k: &str) -> Option<&str> {
        self.0.iter().find(|(n, _)| n == k).map(|(_, v)| v.as_str())
    }

    fn set(&mut self, k: &str, v: &str) {
        match self.0.iter_mut().find(|(n, _)| n == k) {
            Some(slot) => slot.1 = v.to_string(),
            None => self.0.push((k.to_string(), v.to_string())),
        }
    }
}

#[derive(Clone)]
struct Ctx {
    m: Matrix,
    opacity: f64,
    group: Option<String>,
    /// Ancestors, root first.
    path: Rc<Vec<usize>>,
    /// `<use>` references being expanded (a reference to one of them is a loop).
    uses: Rc<Vec<String>>,
    depth: usize,
}

/// A node still to walk: the node, its parent's computed style and the context.
type Task = (usize, Rc<Computed>, Rc<Ctx>);

// Kinds of references counted in the report, by index.
const GRADIENTS: usize = 0;
const PATTERNS: usize = 1;
const CLIPS: usize = 2;
const MASKS: usize = 3;
const FILTERS: usize = 4;
const MARKERS: usize = 5;

/// `parseFloat(v)` when finite, else the fallback.
fn num(v: Option<&str>, fallback: f64) -> f64 {
    let x = parse_float(v.unwrap_or(""));
    if x.is_finite() { x } else { fallback }
}

/// A number as a template literal writes it.
fn t(x: f64) -> String {
    number::to_string(x)
}

/// `v.replace(/^#/, '')`.
fn strip_hash(v: &str) -> &str {
    v.strip_prefix('#').unwrap_or(v)
}

/// `t.replace(/\s+/g, ' ')`.
fn collapse_space(t: &str) -> String {
    let mut out = String::with_capacity(t.len());
    let mut space = false;
    for c in t.chars() {
        if is_space(c) {
            if !space {
                out.push(' ');
            }
            space = true;
        } else {
            out.push(c);
            space = false;
        }
    }
    out
}

/// `/url\(\s*['"]?#([^'")\s]+)/`: the first reference's id.
fn first_url_id(v: &str) -> Option<String> {
    let mut from = 0;
    while let Some(i) = v[from..].find("url(") {
        let at = from + i;
        let mut r = v[at + 4..].trim_start_matches(is_space);
        if r.starts_with(['\'', '"']) {
            r = &r[1..];
        }
        if let Some(r) = r.strip_prefix('#') {
            let n = r
                .char_indices()
                .find(|&(_, c)| c == '\'' || c == '"' || c == ')' || is_space(c))
                .map_or(r.len(), |(k, _)| k);
            if n > 0 {
                return Some(r[..n].to_string());
            }
        }
        from = at + 1;
    }
    None
}

/// A node's text: its own, or its descendants' text nodes in order.
fn text_content(nodes: &[XmlNode], n: usize) -> String {
    if nodes[n].children.is_empty() {
        return nodes[n].text.clone().unwrap_or_default();
    }
    let mut out = String::new();
    let mut stack: Vec<usize> = nodes[n].children.iter().rev().copied().collect();
    while let Some(k) = stack.pop() {
        let node = &nodes[k];
        if node.tag == "#text" || node.children.is_empty() {
            out.push_str(node.text.as_deref().unwrap_or(""));
        } else {
            stack.extend(node.children.iter().rev());
        }
    }
    out
}

/// The nodes in document order (the root first, each node before its children).
fn document_order(nodes: &[XmlNode]) -> Vec<usize> {
    let mut out = Vec::with_capacity(nodes.len());
    let mut stack = vec![0];
    while let Some(n) = stack.pop() {
        out.push(n);
        stack.extend(nodes[n].children.iter().rev());
    }
    out
}

fn paint(f: &Flat, alpha: f64) -> String {
    match f {
        Flat::Color(c) => with_alpha(&c.hex, alpha),
        Flat::None => "none".into(),
        Flat::Fill => "fill".into(),
        Flat::Stroke => "stroke".into(),
    }
}

fn field(k: &str, v: Json) -> (String, Option<Json>) {
    (k.to_string(), Some(v))
}

fn text_json(s: &str) -> Json {
    Json::Str(s.to_string())
}

struct Importer<'a> {
    nodes: &'a [XmlNode],
    editor: bool,
    rules: Vec<Rule>,
    by_id: HashMap<&'a str, usize>,
    width: f64,
    height: f64,
    report: Report,
    seen: [HashSet<String>; 6],
    skipped: Vec<String>,
    shapes: Vec<Obj>,
    ids: usize,
    groups: usize,
}

impl<'a> Importer<'a> {
    fn fresh_id(&mut self) -> Json {
        let id = format!("\u{1}{}", self.ids);
        self.ids += 1;
        Json::Str(id)
    }

    fn new_group(&mut self) -> String {
        let id = format!("\u{2}{}", self.groups);
        self.groups += 1;
        id
    }

    fn skip(&mut self, tag: &str) {
        if !self.skipped.iter().any(|t| t == tag) {
            self.skipped.push(tag.to_string());
        }
    }

    fn length_x(&self, v: Option<&str>, fallback: f64) -> f64 {
        to_user(v, self.width, 16.0).unwrap_or(fallback)
    }

    fn length_y(&self, v: Option<&str>, fallback: f64) -> f64 {
        to_user(v, self.height, 16.0).unwrap_or(fallback)
    }

    fn length_r(&self, v: Option<&str>, fallback: f64) -> f64 {
        to_user(
            v,
            js_hypot(self.width, self.height) / std::f64::consts::SQRT_2,
            16.0,
        )
        .unwrap_or(fallback)
    }

    // ── Styles ──

    fn specified(&self, n: usize, path: &[usize]) -> Props {
        let node = &self.nodes[n];
        let mut out = Props::default();
        for (k, v) in &node.attrs {
            if PROPS.contains(&k.as_str()) {
                out.set(k, v);
            }
        }
        let mut hits: Vec<(f64, f64, usize)> = Vec::new();
        for (i, r) in self.rules.iter().enumerate() {
            let mut best = -1.0;
            for s in &r.selectors {
                if s.spec > best && matches(s, self.nodes, n, path) {
                    best = s.spec;
                }
            }
            if best >= 0.0 {
                hits.push((best, r.order, i));
            }
        }
        stable_sort(&mut hits, &mut |a, b| {
            let d = a.0 - b.0;
            let v = if d != 0.0 && !d.is_nan() {
                d
            } else {
                a.1 - b.1
            };
            v.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut important = Props::default();
        for &(_, _, i) in &hits {
            for d in &self.rules[i].decls {
                if d.important {
                    important.set(&d.prop, &d.value);
                } else {
                    out.set(&d.prop, &d.value);
                }
            }
        }
        for d in parse_decls(node.attr("style").unwrap_or("")) {
            out.set(&d.prop, &d.value);
        }
        for (k, v) in &important.0 {
            out.set(k, v);
        }
        out
    }

    fn compute(&self, a: &Props, p: &Computed) -> Computed {
        let mut c = p.clone();
        let val = |k: &str| {
            a.get(k)
                .map(trim)
                .filter(|v| !v.is_empty() && *v != "inherit")
        };
        if let Some(fill) = raw_paint(val("fill")) {
            c.fill = fill;
        }
        if let Some(stroke) = raw_paint(val("stroke")) {
            c.stroke = stroke;
        }
        if let Some(size) = val("font-size") {
            c.font_size = match FONT_SIZES.iter().find(|(k, _)| *k == size) {
                Some(&(_, v)) => v,
                None => to_user(Some(size), p.font_size, p.font_size).unwrap_or(p.font_size),
            };
        }
        let sw = to_user(
            val("stroke-width"),
            js_hypot(self.width, self.height) / std::f64::consts::SQRT_2,
            c.font_size,
        );
        if let Some(sw) = sw
            && sw >= 0.0
        {
            c.stroke_width = sw;
        }
        let op = |v: Option<&str>, prev: f64| match v {
            None => prev,
            Some(v) => {
                let x = if v.ends_with('%') {
                    parse_float(v) / 100.0
                } else {
                    parse_float(v)
                };
                if x.is_finite() { clamp01(x) } else { prev }
            }
        };
        c.fill_opacity = op(val("fill-opacity"), p.fill_opacity);
        c.stroke_opacity = op(val("stroke-opacity"), p.stroke_opacity);
        if let Some(dash) = val("stroke-dasharray") {
            let d: Vec<f64> = if dash == "none" {
                Vec::new()
            } else {
                split_on(dash, |ch| is_space(ch) || ch == ',')
                    .into_iter()
                    .map(|t| to_user(Some(t), 100.0, c.font_size).unwrap_or(f64::NAN))
                    .collect()
            };
            c.dash =
                if d.is_empty() || d.iter().any(|x| !(*x >= 0.0)) || d.iter().all(|x| *x == 0.0) {
                    None
                } else if d.len() % 2 == 1 {
                    Some([d.as_slice(), d.as_slice()].concat())
                } else {
                    Some(d)
                };
        }
        match val("stroke-linecap") {
            Some("butt") => c.cap = "butt",
            Some("round") => c.cap = "round",
            Some("square") => c.cap = "square",
            _ => {}
        }
        match val("stroke-linejoin") {
            Some("round") => c.join = "round",
            Some("bevel") => c.join = "bevel",
            Some("miter" | "miter-clip" | "arcs") => c.join = "miter",
            _ => {}
        }
        match val("fill-rule") {
            Some("nonzero") => c.fill_rule = "nonzero",
            Some("evenodd") => c.fill_rule = "evenodd",
            _ => {}
        }
        if let Some(family) = val("font-family") {
            c.font_family = family.to_string();
        }
        if let Some(weight) = val("font-weight") {
            c.font_weight = match weight {
                "bold" | "bolder" => 700.0,
                "normal" | "lighter" => 400.0,
                w => or(js_number(w), p.font_weight),
            };
        }
        match val("text-anchor") {
            Some("start") => c.anchor = "start",
            Some("middle") => c.anchor = "middle",
            Some("end") => c.anchor = "end",
            _ => {}
        }
        if let Some(vis) = val("visibility") {
            c.hidden = vis == "hidden" || vis == "collapse";
        }
        c
    }

    // ── Paint references (gradients and patterns become one flat colour) ──

    fn stops_of(&self, g: usize) -> Vec<usize> {
        let mut g = g;
        let mut depth = 0;
        loop {
            let node = &self.nodes[g];
            let own: Vec<usize> = node
                .children
                .iter()
                .copied()
                .filter(|&k| self.nodes[k].tag.to_lowercase() == "stop")
                .collect();
            if !own.is_empty() || depth > 8 {
                return own;
            }
            let href = strip_hash(node.attr("href").or(node.attr("xlink:href")).unwrap_or(""));
            let next = if href.is_empty() {
                None
            } else {
                self.by_id.get(href).copied()
            };
            match next {
                Some(next) if next != g => {
                    g = next;
                    depth += 1;
                }
                _ => return own,
            }
        }
    }

    fn flatten(&mut self, p: &RawPaint, depth: usize) -> Flat {
        let (ids, fallback) = match p {
            RawPaint::Plain(f) => return f.clone(),
            RawPaint::Url { ids, fallback } => (ids, fallback),
        };
        for (k, id) in ids.iter().enumerate() {
            // Each reference's fallback is the next, one level deeper.
            let depth = depth + k;
            let Some(el) = self.by_id.get(id.as_str()).copied() else {
                continue;
            };
            let tag = self.nodes[el].tag.to_lowercase();
            if tag == "lineargradient" || tag == "radialgradient" {
                self.seen[GRADIENTS].insert(id.clone());
                let stops = self.stops_of(el);
                if stops.is_empty() {
                    return Flat::None;
                }
                let stop = stops[(stops.len() - 1) / 2];
                let a = self.specified(stop, &[]);
                let c = read_color(a.get("stop-color").unwrap_or("black")).unwrap_or(Color {
                    hex: "#000000".into(),
                    alpha: 1.0,
                });
                let so = match a.get("stop-opacity") {
                    Some(v) => clamp01(parse_float(v)),
                    None => 1.0,
                };
                return Flat::Color(Color {
                    hex: c.hex,
                    alpha: c.alpha * if so.is_finite() { so } else { 1.0 },
                });
            }
            if tag == "pattern" {
                self.seen[PATTERNS].insert(id.clone());
                return self.pattern_paint(el, depth).unwrap_or(Flat::Color(Color {
                    hex: "#808080".into(),
                    alpha: 1.0,
                }));
            }
        }
        match fallback {
            Some(f) => f.clone(),
            None => {
                self.report.broken += 1;
                Flat::None
            }
        }
    }

    /// The first fill painted inside a pattern (document order), flattened in turn.
    fn pattern_paint(&mut self, el: usize, depth: usize) -> Option<Flat> {
        let nodes = self.nodes;
        let mut stack = vec![el];
        while let Some(n) = stack.pop() {
            let fill = raw_paint(self.specified(n, &[]).get("fill"));
            if let Some(f) = fill
                && f != RawPaint::Plain(Flat::None)
                && nodes[n].tag != "pattern"
                && depth < 4
            {
                return Some(self.flatten(&f, depth + 1));
            }
            stack.extend(nodes[n].children.iter().rev());
        }
        None
    }

    fn note_ref(&mut self, a: &Props) {
        let refs = [
            ("clip-path", CLIPS),
            ("mask", MASKS),
            ("filter", FILTERS),
            ("marker", MARKERS),
            ("marker-start", MARKERS),
            ("marker-mid", MARKERS),
            ("marker-end", MARKERS),
        ];
        for (k, set) in refs {
            if let Some(id) = first_url_id(a.get(k).unwrap_or("")) {
                self.seen[set].insert(id);
            }
        }
    }

    // ── Shapes ──

    fn put(
        &mut self,
        mut s: Obj,
        c: &Computed,
        ctx: &Ctx,
        n: usize,
        a: &Props,
    ) -> Result<(), String> {
        let m = &ctx.m;
        let scale = (m[0] * m[3] - m[1] * m[2]).abs().sqrt();
        let fill = self.flatten(&c.fill, 0);
        let stroke = self.flatten(&c.stroke, 0);
        let sym_f = matches!(fill, Flat::Fill | Flat::Stroke);
        let sym_s = matches!(stroke, Flat::Fill | Flat::Stroke);
        if sym_f || sym_s {
            self.report.symbol_paint = true;
        }
        let mut opacity = ctx.opacity;
        let fa = match &fill {
            Flat::Color(col) => col.alpha * c.fill_opacity,
            _ => c.fill_opacity,
        };
        let sa = match &stroke {
            Flat::Color(col) => col.alpha * c.stroke_opacity,
            _ => c.stroke_opacity,
        };
        // The symbol's colours carry no alpha: their opacity moves to the shape when only one of them paints.
        if sym_f && fa < 0.999 && (stroke == Flat::None || !sym_s) {
            opacity *= fa;
        } else if sym_s && sa < 0.999 && fill == Flat::None {
            opacity *= sa;
        } else if (sym_f && fa < 0.999) || (sym_s && sa < 0.999) {
            self.report.approx_opacity += 1;
        }
        s.set(
            "fill",
            text_json(&paint(&fill, if sym_f { 1.0 } else { fa })),
        );
        s.set(
            "stroke",
            text_json(&paint(&stroke, if sym_s { 1.0 } else { sa })),
        );
        s.set_num("strokeWidth", js_round(c.stroke_width * scale * 1e6) / 1e6);
        if opacity < 0.999 {
            s.set_num("opacity", js_round(opacity * 1000.0) / 1000.0);
        }
        if stroke != Flat::None
            && let Some(dash) = &c.dash
        {
            s.set(
                "dash",
                Json::Arr(
                    dash.iter()
                        .map(|d| Json::Num(js_round(d * scale * 1e6) / 1e6))
                        .collect(),
                ),
            );
        }
        let is_path = s.kind() == "path";
        // Paths default to round ends and evenodd in the model; SVG defaults are written out.
        if stroke != Flat::None && (is_path || c.cap != "butt") {
            s.set("cap", text_json(c.cap));
        }
        if stroke != Flat::None && (is_path || c.join != "miter") {
            s.set("join", text_json(c.join));
        }
        if is_path && fill != Flat::None {
            s.set("fillRule", text_json(c.fill_rule));
        }
        if let Some(g) = ctx.group.as_deref().filter(|g| !g.is_empty()) {
            s.set("group", text_json(g));
        }
        let nodes = self.nodes;
        let node = &nodes[n];
        if self.editor {
            if let Some(id) = node
                .attr("id")
                .filter(|v| !v.is_empty() && !starts_with_digit(v))
            {
                s.set("id", text_json(id));
            }
            if let Some(name) = node.attr("data-name").filter(|v| !v.is_empty()) {
                s.set("name", text_json(name));
            }
            if a.get("display") == Some("none") {
                s.set("hidden", Json::Bool(true));
            }
        } else {
            let title = node
                .children
                .iter()
                .map(|&k| &nodes[k])
                .find(|k| k.tag == "title");
            let label = node
                .attr("inkscape:label")
                .or_else(|| {
                    title
                        .and_then(|t| {
                            t.children
                                .iter()
                                .map(|&k| &nodes[k])
                                .find(|k| k.tag == "#text")
                        })
                        .and_then(|k| k.text.as_deref())
                })
                .or_else(|| title.and_then(|t| t.text.as_deref()));
            if let Some(label) = label.map(trim).filter(|l| !l.is_empty()) {
                s.set("name", text_json(&slice(label, 0, Some(60))));
            }
        }
        let Some(mut t) = transform_shape(&s, m)? else {
            return Ok(());
        };
        // A rectangle or ellipse sheared into a path keeps the SVG corners.
        if t.kind() == "path" && s.kind() != "path" && t.text("stroke") != Some("none") {
            let cap = t.field("cap").unwrap_or_else(|| text_json("butt"));
            t.set("cap", cap);
            let join = t.field("join").unwrap_or_else(|| text_json("miter"));
            t.set("join", join);
        }
        self.shapes.push(t);
        self.report.shapes += 1;
        Ok(())
    }

    /// A shape's first fields: id, no paint, unit stroke width, then its kind.
    fn base(id: &Json, kind: &str) -> Vec<(String, Option<Json>)> {
        vec![
            field("id", id.clone()),
            field("fill", text_json("none")),
            field("stroke", text_json("none")),
            field("strokeWidth", Json::Num(1.0)),
            field("kind", text_json(kind)),
        ]
    }

    fn path_shape(id: &Json, subs: &[SubPath]) -> Obj {
        let mut f = Importer::base(id, "path");
        f.push(field(
            "subs",
            Json::Arr(subs.iter().map(SubPath::tree).collect()),
        ));
        Obj(f)
    }

    fn walk(&mut self, origin: Matrix) -> Result<(), String> {
        let root: Task = (
            0,
            Rc::new(initial()),
            Rc::new(Ctx {
                m: origin,
                opacity: 1.0,
                group: None,
                path: Rc::new(Vec::new()),
                uses: Rc::new(Vec::new()),
                depth: 0,
            }),
        );
        let mut stack = vec![root];
        let mut later: Vec<Task> = Vec::new();
        while let Some((n, parent, ctx)) = stack.pop() {
            self.step(n, &parent, &ctx, &mut later)?;
            // Children walk next, in order, before the parent's later siblings.
            stack.extend(later.drain(..).rev());
        }
        Ok(())
    }

    /// One element of the walk; the elements to walk next go to `later`, in order.
    fn step(
        &mut self,
        n: usize,
        parent: &Computed,
        ctx: &Ctx,
        later: &mut Vec<Task>,
    ) -> Result<(), String> {
        let nodes = self.nodes;
        let node = &nodes[n];
        let tag = node.tag.to_lowercase();
        if tag == "#text" || NOT_RENDERED.contains(&tag.as_str()) {
            return Ok(());
        }
        let a = self.specified(n, &ctx.path);
        if a.get("display") == Some("none") && !self.editor {
            return Ok(());
        }
        self.note_ref(&a);
        let c = Rc::new(self.compute(&a, parent));
        let own = num(a.get("opacity"), 1.0);
        let m = multiply(&ctx.m, &read_transform(node.attr("transform")));
        let mut path = (*ctx.path).clone();
        path.push(n);
        let mut next = Ctx {
            m,
            opacity: ctx.opacity * clamp01(own),
            group: ctx.group.clone(),
            path: Rc::new(path),
            uses: ctx.uses.clone(),
            depth: ctx.depth + 1,
        };
        let visible = !c.hidden || self.editor;
        match tag.as_str() {
            "svg" => {
                if ctx.depth > 0 {
                    // A nested viewport: its own position, size and viewBox.
                    let x = self.length_x(node.attr("x"), 0.0);
                    let y = self.length_y(node.attr("y"), 0.0);
                    let w = self.length_x(node.attr("width"), self.width);
                    let h = self.length_y(node.attr("height"), self.height);
                    let fit = match view_box_of(node.attr("viewBox")) {
                        Some(b) => view_box_transform(
                            &b,
                            w,
                            h,
                            node.attr("preserveAspectRatio").unwrap_or(""),
                        ),
                        None => IDENTITY,
                    };
                    next.m = multiply(&multiply(&m, &[1.0, 0.0, 0.0, 1.0, x, y]), &fit);
                }
                let next = Rc::new(next);
                later.extend(node.children.iter().map(|&k| (k, c.clone(), next.clone())));
                return Ok(());
            }
            "g" | "a" | "switch" => {
                // A group below the root keeps its shapes together (not an Inkscape layer, not a link).
                let layer = node.attr("inkscape:groupmode") == Some("layer");
                if tag == "g"
                    && ctx.depth > 0
                    && !layer
                    && ctx.group.as_deref().is_none_or(str::is_empty)
                {
                    next.group = Some(
                        match node
                            .attr("data-group")
                            .filter(|g| self.editor && !g.is_empty())
                        {
                            Some(g) => g.to_string(),
                            None => self.new_group(),
                        },
                    );
                }
                let next = Rc::new(next);
                if tag == "switch" {
                    if let Some(&k) = node.children.iter().find(|&&k| nodes[k].tag != "#text") {
                        later.push((k, c.clone(), next));
                    }
                } else {
                    later.extend(node.children.iter().map(|&k| (k, c.clone(), next.clone())));
                }
                return Ok(());
            }
            "use" => {
                let id = strip_hash(node.attr("href").or(node.attr("xlink:href")).unwrap_or(""));
                let target = if id.is_empty() {
                    None
                } else {
                    self.by_id.get(id).copied()
                };
                let Some(target) =
                    target.filter(|t| !ctx.uses.iter().any(|u| u == id) && !ctx.path.contains(t))
                else {
                    self.report.broken += 1;
                    return Ok(());
                };
                self.report.uses += 1;
                let mut uses = (*ctx.uses).clone();
                uses.push(id.to_string());
                let at = multiply(
                    &m,
                    &[
                        1.0,
                        0.0,
                        0.0,
                        1.0,
                        self.length_x(node.attr("x"), 0.0),
                        self.length_y(node.attr("y"), 0.0),
                    ],
                );
                let group = match &ctx.group {
                    Some(g) => Some(g.clone()),
                    None if ctx.depth > 0
                        || node.attr("transform").is_some_and(|t| !t.is_empty()) =>
                    {
                        Some(self.new_group())
                    }
                    None => None,
                };
                let next_path = next.path.clone();
                let mut inner = Ctx {
                    m: at,
                    uses: Rc::new(uses),
                    group,
                    ..next
                };
                let tnode = &nodes[target];
                let ttag = tnode.tag.to_lowercase();
                if ttag == "symbol" || ttag == "svg" {
                    let bx = view_box_of(tnode.attr("viewBox"));
                    let w = self.length_x(
                        node.attr("width").or(tnode.attr("width")),
                        bx.as_ref().map_or(self.width, |b| b[2]),
                    );
                    let h = self.length_y(
                        node.attr("height").or(tnode.attr("height")),
                        bx.as_ref().map_or(self.height, |b| b[3]),
                    );
                    if ttag == "svg" {
                        inner.m = multiply(
                            &inner.m,
                            &[
                                1.0,
                                0.0,
                                0.0,
                                1.0,
                                self.length_x(tnode.attr("x"), 0.0),
                                self.length_y(tnode.attr("y"), 0.0),
                            ],
                        );
                    }
                    if let Some(b) = &bx {
                        inner.m = multiply(
                            &inner.m,
                            &view_box_transform(
                                b,
                                w,
                                h,
                                tnode.attr("preserveAspectRatio").unwrap_or(""),
                            ),
                        );
                    }
                    let sc = Rc::new(self.compute(&self.specified(target, &next_path), &c));
                    let mut path = (*inner.path).clone();
                    path.push(target);
                    let inside = Rc::new(Ctx {
                        path: Rc::new(path),
                        ..inner
                    });
                    later.extend(
                        tnode
                            .children
                            .iter()
                            .map(|&k| (k, sc.clone(), inside.clone())),
                    );
                } else {
                    later.push((target, c.clone(), Rc::new(inner)));
                }
                return Ok(());
            }
            "image" => {
                if node.attr("data-kentos") != Some("reference") {
                    self.report.images += 1;
                    self.skip("image");
                }
                return Ok(());
            }
            _ => {}
        }
        if !visible {
            return Ok(());
        }
        let id = self.fresh_id();
        match tag.as_str() {
            "rect" => {
                let w = self.length_x(node.attr("width"), 0.0);
                let h = self.length_y(node.attr("height"), 0.0);
                if !(w > 0.0 && h > 0.0) {
                    return Ok(());
                }
                let x = self.length_x(node.attr("x"), 0.0);
                let y = self.length_y(node.attr("y"), 0.0);
                let rx = to_user(node.attr("rx").filter(|v| *v != "auto"), self.width, 16.0);
                let ry = to_user(node.attr("ry").filter(|v| *v != "auto"), self.height, 16.0);
                let rx = rx.unwrap_or(ry.unwrap_or(0.0));
                let ry = ry.unwrap_or(rx);
                let rx = js_min(js_max(0.0, rx), w / 2.0);
                let ry = js_min(js_max(0.0, ry), h / 2.0);
                if (rx - ry).abs() < 1e-9 {
                    let mut f = Importer::base(&id, "rect");
                    f.extend([
                        field("x", Json::Num(x)),
                        field("y", Json::Num(y)),
                        field("w", Json::Num(w)),
                        field("h", Json::Num(h)),
                        ("r".to_string(), truthy(rx).then_some(Json::Num(rx))),
                    ]);
                    self.put(Obj(f), &c, &next, n, &a)?;
                } else {
                    // Elliptic corners: a path of four arcs.
                    let d = format!(
                        "M{} {}H{}A{} {} 0 0 1 {} {}V{}A{} {} 0 0 1 {} {}H{}A{} {} 0 0 1 {} {}V{}A{} {} 0 0 1 {} {}Z",
                        t(x + rx),
                        t(y),
                        t(x + w - rx),
                        t(rx),
                        t(ry),
                        t(x + w),
                        t(y + ry),
                        t(y + h - ry),
                        t(rx),
                        t(ry),
                        t(x + w - rx),
                        t(y + h),
                        t(x + rx),
                        t(rx),
                        t(ry),
                        t(x),
                        t(y + h - ry),
                        t(y + ry),
                        t(rx),
                        t(ry),
                        t(x + rx),
                        t(y),
                    );
                    self.put(
                        Importer::path_shape(&id, &parse_path_data(&d)),
                        &c,
                        &next,
                        n,
                        &a,
                    )?;
                }
            }
            "circle" | "ellipse" => {
                let (rx, ry) = if tag == "circle" {
                    let r = self.length_r(node.attr("r"), 0.0);
                    (r, r)
                } else {
                    let rx = if node.attr("rx") == Some("auto") {
                        node.attr("ry")
                    } else {
                        node.attr("rx")
                    };
                    let ry = if node.attr("ry") == Some("auto") {
                        node.attr("rx")
                    } else {
                        node.attr("ry")
                    };
                    (self.length_x(rx, f64::NAN), self.length_y(ry, f64::NAN))
                };
                let rrx = if rx.is_finite() { rx } else { ry };
                let rry = if ry.is_finite() { ry } else { rx };
                if rrx > 0.0 && rry > 0.0 {
                    let mut f = Importer::base(&id, "ellipse");
                    f.extend([
                        field("cx", Json::Num(self.length_x(node.attr("cx"), 0.0))),
                        field("cy", Json::Num(self.length_y(node.attr("cy"), 0.0))),
                        field("rx", Json::Num(rrx)),
                        field("ry", Json::Num(rry)),
                    ]);
                    self.put(Obj(f), &c, &next, n, &a)?;
                }
            }
            "line" => {
                let d = format!(
                    "M{} {}L{} {}",
                    t(self.length_x(node.attr("x1"), 0.0)),
                    t(self.length_y(node.attr("y1"), 0.0)),
                    t(self.length_x(node.attr("x2"), 0.0)),
                    t(self.length_y(node.attr("y2"), 0.0)),
                );
                let mut lc = (*c).clone();
                lc.fill = RawPaint::Plain(Flat::None);
                self.put(
                    Importer::path_shape(&id, &parse_path_data(&d)),
                    &lc,
                    &next,
                    n,
                    &a,
                )?;
            }
            "polyline" | "polygon" => {
                let p = nums(node.attr("points"));
                let pts: Vec<String> = p
                    .chunks_exact(2)
                    .map(|q| format!("{} {}", t(q[0]), t(q[1])))
                    .collect();
                if pts.len() > 1 {
                    let d = format!(
                        "M{}{}",
                        pts.join("L"),
                        if tag == "polygon" { "Z" } else { "" }
                    );
                    self.put(
                        Importer::path_shape(&id, &parse_path_data(&d)),
                        &c,
                        &next,
                        n,
                        &a,
                    )?;
                }
            }
            "path" => {
                let subs = match node.attr("d").filter(|d| !d.is_empty()) {
                    Some(d) => parse_path_data(d),
                    None => Vec::new(),
                };
                if !subs.is_empty() {
                    self.put(Importer::path_shape(&id, &subs), &c, &next, n, &a)?;
                }
            }
            "text" => self.text_shapes(n, c, &next, &a)?,
            _ => {
                if tag != "tspan" && tag != "textpath" {
                    self.skip(&node.tag);
                }
            }
        }
        Ok(())
    }

    /// A text element's lines: every run placed anew (x, y, dx, dy) starts a text shape.
    fn text_shapes(
        &mut self,
        n: usize,
        c0: Rc<Computed>,
        ctx: &Ctx,
        a0: &Props,
    ) -> Result<(), String> {
        struct Line {
            x: f64,
            y: f64,
            text: String,
            c: Rc<Computed>,
        }
        /// Still to read, in order: an element (placed, then its runs), a run element met in its parent, or text.
        enum Item {
            Element(usize, Rc<Computed>, Rc<Vec<usize>>),
            Run(usize, Rc<Computed>, Rc<Vec<usize>>),
            Text(String, Rc<Computed>),
        }
        let nodes = self.nodes;
        let mut lines: Vec<Line> = Vec::new();
        let mut cur: Option<usize> = None;
        let mut px = 0.0;
        let mut py = 0.0;
        let mut stack = vec![Item::Element(n, c0, ctx.path.clone())];
        while let Some(item) = stack.pop() {
            match item {
                Item::Element(el, c, path) => {
                    let node = &nodes[el];
                    // Positions may carry units; em is the element's own font size.
                    let list = |v: Option<&str>, percent: f64| -> Vec<f64> {
                        split_on(v.unwrap_or(""), |ch| is_space(ch) || ch == ',')
                            .into_iter()
                            .map(|t| to_user(Some(t), percent, c.font_size).unwrap_or(f64::NAN))
                            .filter(|x| x.is_finite())
                            .collect()
                    };
                    let xs = list(node.attr("x"), self.width);
                    let ys = list(node.attr("y"), self.height);
                    let dx = list(node.attr("dx"), self.width);
                    let dy = list(node.attr("dy"), self.height);
                    if let Some(&x) = xs.first() {
                        px = x;
                    }
                    if let Some(&y) = ys.first() {
                        py = y;
                    }
                    if let Some(&d) = dx.first() {
                        px += d;
                    }
                    if let Some(&d) = dy.first() {
                        py += d;
                    }
                    if !xs.is_empty() || !ys.is_empty() || !dx.is_empty() || !dy.is_empty() {
                        cur = None;
                    }
                    let mut runs: Vec<Item> = Vec::new();
                    if !node.children.is_empty() {
                        for &k in &node.children {
                            let kn = &nodes[k];
                            if kn.tag == "#text" {
                                runs.push(Item::Text(
                                    kn.text.clone().unwrap_or_default(),
                                    c.clone(),
                                ));
                            } else if ["tspan", "textpath", "a"]
                                .iter()
                                .any(|r| kn.tag.eq_ignore_ascii_case(r))
                            {
                                runs.push(Item::Run(k, c.clone(), path.clone()));
                            }
                        }
                    } else if let Some(text) = &node.text {
                        runs.push(Item::Text(text.clone(), c.clone()));
                    }
                    stack.extend(runs.into_iter().rev());
                }
                Item::Run(k, c, path) => {
                    let ka = self.specified(k, &path);
                    if ka.get("display") == Some("none") {
                        continue;
                    }
                    let kc = Rc::new(self.compute(&ka, &c));
                    let mut kpath = (*path).clone();
                    kpath.push(k);
                    stack.push(Item::Element(k, kc, Rc::new(kpath)));
                }
                Item::Text(raw, c) => {
                    let t = collapse_space(&raw);
                    if cur.is_none() && trim(&t).is_empty() {
                        continue;
                    }
                    let at = match cur {
                        Some(at) => at,
                        None => {
                            lines.push(Line {
                                x: px,
                                y: py,
                                text: String::new(),
                                c: c.clone(),
                            });
                            lines.len() - 1
                        }
                    };
                    cur = Some(at);
                    lines[at].text.push_str(&t);
                    px += if t.is_empty() {
                        0.0
                    } else {
                        crate::model::text_em(&t) * c.font_size
                    };
                }
            }
        }
        for l in lines {
            let text = trim(&l.text);
            if text.is_empty() || l.c.hidden {
                continue;
            }
            let w = l.c.font_weight;
            let family = l.c.font_family.to_lowercase();
            let id = self.fresh_id();
            let s = Obj(vec![
                field("id", id),
                field("kind", text_json("text")),
                field("x", Json::Num(l.x)),
                field("y", Json::Num(l.y)),
                field("text", text_json(text)),
                field("size", Json::Num(l.c.font_size)),
                field(
                    "weight",
                    Json::Num(if w >= 850.0 {
                        900.0
                    } else if w >= 550.0 {
                        700.0
                    } else {
                        400.0
                    }),
                ),
                field(
                    "font",
                    text_json(if family.contains("serif") && !family.contains("sans") {
                        "serif"
                    } else {
                        "sans"
                    }),
                ),
                field("anchor", text_json(l.c.anchor)),
                field("fill", text_json("none")),
                field("stroke", text_json("none")),
                field("strokeWidth", Json::Num(1.0)),
            ]);
            self.put(s, &l.c, ctx, n, a0)?;
        }
        Ok(())
    }
}

/// Pixels of a length in an absolute unit.
fn px_of(l: &Length) -> f64 {
    l.value * px_per(&l.unit).unwrap_or(f64::NAN)
}

/// A length in an absolute unit (not %, em, ex or rem).
fn absolute(l: &Option<Length>) -> bool {
    l.as_ref()
        .is_some_and(|l| !matches!(l.unit.as_str(), "%" | "em" | "ex" | "rem"))
}

/// An SVG file (its elements, as the page parsed them) as a drawing.
pub fn doc_from_svg_tree(tree: &XmlTree, opts: &ImportOptions) -> Result<Imported, String> {
    let nodes = tree.nodes.as_slice();
    let mut by_id: HashMap<&str, usize> = HashMap::new();
    let mut style_texts: Vec<String> = Vec::new();
    let mut reference: Option<ReferenceSpec> = None;
    for n in document_order(nodes) {
        let node = &nodes[n];
        if let Some(id) = node.attr("id").filter(|v| !v.is_empty()) {
            by_id.entry(id).or_insert(n);
        }
        if node.tag.to_lowercase() == "style"
            && node
                .attr("type")
                .is_none_or(|t| t.is_empty() || t.to_ascii_lowercase().contains("css"))
        {
            style_texts.push(text_content(nodes, n));
        }
        if node.tag == "image" && node.attr("data-kentos") == Some("reference") {
            let href = node.attr("href").or(node.attr("xlink:href")).unwrap_or("");
            if href.starts_with("data:image/") {
                reference = Some(ReferenceSpec {
                    href: href.to_string(),
                    x: num(node.attr("x"), 0.0),
                    y: num(node.attr("y"), 0.0),
                    width: num(node.attr("width"), 1.0),
                    height: num(node.attr("height"), 1.0),
                    opacity: num(node.attr("opacity"), 0.5),
                    locked: node.attr("data-locked") != Some("0"),
                    name: node.attr("data-name").unwrap_or("Altlık").to_string(),
                });
            }
        }
    }
    let mut rules: Vec<Rule> = Vec::new();
    for t in &style_texts {
        let more = parse_css(t, rules.len() as f64);
        rules.extend(more);
    }

    // ── Root: canvas, units, physical size ──
    let root = &nodes[0];
    let vb = view_box_of(root.attr("viewBox"));
    let wl = parse_length(root.attr("width"));
    let hl = parse_length(root.attr("height"));
    let px_mm = px_per("mm").unwrap_or(f64::NAN);
    let mut origin: Matrix = IDENTITY;
    let mut size_mm: Option<f64> = None;
    let width;
    let mut height;
    let physical = |l: &Option<Length>| {
        absolute(l)
            && l.as_ref()
                .is_some_and(|l| !l.unit.is_empty() && l.unit != "px")
    };
    match (&vb, &wl, &hl) {
        (Some(vb), _, _) => {
            width = vb[2];
            height = vb[3];
            origin = [1.0, 0.0, 0.0, 1.0, -vb[0], -vb[1]];
            // A viewBox stretched onto a viewport of another shape keeps that stretch.
            let par = trim(root.attr("preserveAspectRatio").unwrap_or(""));
            if let (Some(w), Some(h)) = (&wl, &hl)
                && par.starts_with("none")
                && absolute(&wl)
                && absolute(&hl)
            {
                let k = px_of(h) / px_of(w) / (height / width);
                if (k - 1.0).abs() > 1e-6 {
                    origin = multiply(&[1.0, 0.0, 0.0, k, 0.0, 0.0], &origin);
                    height *= k;
                }
            }
            if let Some(w) = &wl
                && physical(&wl)
            {
                size_mm = Some(px_of(w) / px_mm);
            }
        }
        (None, Some(w), Some(h)) if absolute(&wl) && absolute(&hl) && physical(&wl) => {
            // Without a viewBox user units are px; a size in mm (cm, in, pt) makes that unit the drawing's.
            let k = px_per(&w.unit).unwrap_or(f64::NAN);
            width = w.value;
            height = px_of(h) / k;
            origin = [1.0 / k, 0.0, 0.0, 1.0 / k, 0.0, 0.0];
            size_mm = Some((w.value * k) / px_mm);
        }
        _ => {
            width = match &wl {
                Some(w) if absolute(&wl) => px_of(w),
                _ => 0.0,
            };
            height = match &hl {
                Some(h) if absolute(&hl) => px_of(h),
                _ => 0.0,
            };
        }
    }
    let mut doc_size: Option<f64> = size_mm
        .filter(|s| truthy(*s) && s.is_finite() && *s > 0.0)
        .map(|s| js_round(s * 1000.0) / 1000.0);
    let no_size = vb.is_none() && !truthy(width) && !truthy(height);
    if root.attr("data-size-mm").is_some_and(|v| !v.is_empty()) {
        let v = num(root.attr("data-size-mm"), 0.0);
        if truthy(v) {
            doc_size = Some(v);
        }
    }
    let background = root
        .attr("data-background")
        .filter(|v| !v.is_empty())
        .and_then(read_color)
        .map(|c| c.hex);

    let mut im = Importer {
        nodes,
        editor: opts.editor,
        rules,
        by_id,
        width,
        height,
        report: Report::default(),
        seen: Default::default(),
        skipped: Vec::new(),
        shapes: Vec::new(),
        ids: 0,
        groups: 0,
    };
    im.walk(origin)?;

    // A group of one is no group.
    let mut members: HashMap<String, usize> = HashMap::new();
    for s in &im.shapes {
        if let Some(g) = s.text("group").filter(|g| !g.is_empty()) {
            *members.entry(g.to_string()).or_insert(0) += 1;
        }
    }
    let mut shapes = std::mem::take(&mut im.shapes);
    for s in &mut shapes {
        if s.text("group")
            .is_some_and(|g| !g.is_empty() && members.get(g) == Some(&1))
        {
            s.set_undefined("group");
        }
    }
    let mut doc_width = or(width, 100.0);
    let mut doc_height = or(height, 100.0);
    // No size at all: the canvas takes the drawing's extent.
    if no_size && !shapes.is_empty() {
        let mut max_x = 0.0;
        let mut max_y = 0.0;
        for s in &shapes {
            let b = shape_box(s)?;
            max_x = js_max(max_x, b.max_x);
            max_y = js_max(max_y, b.max_y);
        }
        doc_width = or(max_x.ceil(), 100.0);
        doc_height = or(max_y.ceil(), 100.0);
    }

    let mut report = std::mem::take(&mut im.report);
    report.gradients = im.seen[GRADIENTS].len();
    report.patterns = im.seen[PATTERNS].len();
    report.clips = im.seen[CLIPS].len();
    report.masks = im.seen[MASKS].len();
    report.filters = im.seen[FILTERS].len();
    report.markers = im.seen[MARKERS].len();
    let colors = color_usage(&shapes)?;
    let target: Option<String> = match &opts.symbol_color {
        SymbolColor::Auto => (!report.symbol_paint).then(|| "black".to_string()),
        SymbolColor::Target(t) => t.clone(),
    };
    let second = opts.second_color.as_deref();
    let shapes = if target.as_deref().is_some_and(|t| !t.is_empty())
        || second.is_some_and(|s| !s.is_empty())
    {
        map_color_shapes(&shapes, target.as_deref(), second)?
    } else {
        shapes
    };
    let mut doc = Obj(vec![
        field("width", Json::Num(doc_width)),
        field("height", Json::Num(doc_height)),
        field("shapes", Json::Arr(shapes.iter().map(obj_tree).collect())),
    ]);
    if let Some(s) = doc_size {
        doc.set_num("sizeMm", s);
    }
    if let Some(b) = background {
        doc.set("background", Json::Str(b));
    }
    Ok(Imported {
        doc,
        skipped: std::mem::take(&mut im.skipped),
        report,
        colors,
        reference,
        ids: im.ids,
        groups: im.groups,
    })
}

/// An object as a JSON tree (undefined fields left out).
fn obj_tree(o: &Obj) -> Json {
    Json::Obj(
        o.0.iter()
            .filter_map(|(k, v)| v.clone().map(|v| (k.clone(), v)))
            .collect(),
    )
}

// ── Colour mapping ─────────────────────────────────────────────────────

/// A fixed paint's colour without alpha, upper case (`#RRGGBB`); None for the symbol's colours and none.
fn rgb_of(p: &Json) -> Option<String> {
    match p {
        Json::Str(s) if s.starts_with('#') => Some(slice(s, 0, Some(7)).to_uppercase()),
        _ => None,
    }
}

/// The fixed colours of a drawing, most painted first.
pub fn color_usage(shapes: &[Obj]) -> Result<Vec<ColorUse>, String> {
    let mut uses: Vec<ColorUse> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    let mut add = |p: &Json, weight: f64| {
        let Some(c) = rgb_of(p) else { return };
        let i = *index.entry(c.clone()).or_insert_with(|| {
            uses.push(ColorUse {
                color: c,
                weight: 0.0,
                count: 0,
            });
            uses.len() - 1
        });
        uses[i].weight += weight;
        uses[i].count += 1;
    };
    for s in shapes {
        let b = shape_box(s)?;
        let w = js_max(0.0, b.max_x - b.min_x);
        let h = js_max(0.0, b.max_y - b.min_y);
        add(s.get("fill"), w * h);
        if s.text("stroke") != Some("none") {
            add(s.get("stroke"), 2.0 * (w + h) * s.num("strokeWidth"));
        }
    }
    stable_sort(&mut uses, &mut |a, b| {
        let d = b.weight - a.weight;
        let v = if d != 0.0 && !d.is_nan() {
            d
        } else {
            b.count as f64 - a.count as f64
        };
        v.partial_cmp(&0.0).unwrap_or(std::cmp::Ordering::Equal)
    });
    Ok(uses)
}

/// Fixed colours turned into the symbol's colours: the chosen one (or all
/// near-black ones, or the dominant one) becomes the symbol colour, another
/// the second colour. A mapped colour's alpha moves to the shape's opacity
/// when the shape has no other paint.
pub fn map_color_shapes(
    shapes: &[Obj],
    symbol: Option<&str>,
    second: Option<&str>,
) -> Result<Vec<Obj>, String> {
    let dominant: Option<String> = if symbol == Some("dominant") {
        color_usage(shapes)?.into_iter().next().map(|u| u.color)
    } else {
        None
    };
    let to_fill = |rgb: &str| match symbol {
        Some("black") => is_near_black(rgb),
        Some("dominant") => dominant.as_deref() == Some(rgb),
        Some(s) => !s.is_empty() && rgb == s.to_uppercase(),
        None => false,
    };
    let to_stroke = |rgb: &str| {
        second.is_some_and(|s| !s.is_empty() && rgb == s.to_uppercase()) && !to_fill(rgb)
    };
    Ok(shapes
        .iter()
        .map(|s| {
            let mut opacity = s.opt_num("opacity").unwrap_or(1.0);
            let mut one = |p: &Json, other: &Json| -> Json {
                let Some(rgb) = rgb_of(p) else {
                    return p.clone();
                };
                let to = if to_fill(&rgb) {
                    "fill"
                } else if to_stroke(&rgb) {
                    "stroke"
                } else {
                    return p.clone();
                };
                if let Json::Str(ps) = p
                    && utf16_len(ps) == 9
                    && matches!(other, Json::Str(o) if o == "none")
                {
                    opacity *= parse_hex(&slice(ps, 7, None)) / 255.0;
                }
                text_json(to)
            };
            let fill = one(s.get("fill"), s.get("stroke"));
            let stroke = one(s.get("stroke"), s.get("fill"));
            if &fill == s.get("fill") && &stroke == s.get("stroke") {
                return s.clone();
            }
            let mut out = s.clone();
            out.set("fill", fill);
            out.set("stroke", stroke);
            out.put(
                "opacity",
                (opacity < 0.999).then(|| Json::Num(js_round(opacity * 1000.0) / 1000.0)),
            );
            out
        })
        .collect())
}

/// `mapColors` on a drawing: its shapes mapped, everything else as it was.
pub fn map_colors(doc: &Obj, symbol: Option<&str>, second: Option<&str>) -> Result<Obj, String> {
    let shapes: Vec<Obj> =
        Vec::from_json(doc.get("shapes")).map_err(|e| format!("“shapes”: {e}"))?;
    let mut out = doc.clone();
    out.set(
        "shapes",
        Json::Arr(
            map_color_shapes(&shapes, symbol, second)?
                .iter()
                .map(obj_tree)
                .collect(),
        ),
    );
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::values::XmlNode;

    fn node(
        tag: &str,
        attrs: &[(&str, &str)],
        parent: Option<usize>,
        tree: &mut Vec<XmlNode>,
    ) -> usize {
        tree.push(XmlNode {
            tag: tag.into(),
            attrs: attrs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
            children: Vec::new(),
            text: None,
        });
        let at = tree.len() - 1;
        if let Some(p) = parent {
            tree[p].children.push(at);
        }
        at
    }

    fn options() -> ImportOptions {
        ImportOptions {
            symbol_color: SymbolColor::Target(None),
            second_color: None,
            editor: false,
        }
    }

    #[test]
    fn a_deep_file_never_meets_the_call_stack() {
        // 20 000 nested groups (the TypeScript's recursion ran out of stack a few thousand deep).
        let mut nodes = Vec::new();
        let mut at = node("svg", &[("viewBox", "0 0 10 10")], None, &mut nodes);
        for _ in 0..20_000 {
            at = node(
                "g",
                &[("transform", "translate(0.0001 0)")],
                Some(at),
                &mut nodes,
            );
        }
        node(
            "rect",
            &[("width", "1"), ("height", "1")],
            Some(at),
            &mut nodes,
        );
        let r = doc_from_svg_tree(&XmlTree { nodes }, &options()).unwrap();
        assert_eq!(r.report.shapes, 1);
        let Json::Arr(shapes) = r.doc.get("shapes") else {
            panic!("shapes")
        };
        // Only the outermost group below the root groups; one shape is no group.
        assert_eq!(shapes.len(), 1);
        assert!((num_field(&shapes[0], "x") - 2.0).abs() < 1e-9);
        assert_eq!(r.groups, 1);
    }

    #[test]
    fn a_long_chain_of_uses_expands_and_a_loop_is_broken() {
        // use → group holding a use of the next group → … 2 000 deep, the last one referring back to the first.
        let mut nodes = Vec::new();
        let root = node("svg", &[("viewBox", "0 0 10 10")], None, &mut nodes);
        let defs = node("defs", &[], Some(root), &mut nodes);
        for i in 0..2_000 {
            let next = if i == 1_999 {
                "#g0".to_string()
            } else {
                format!("#g{}", i + 1)
            };
            let g = node("g", &[("id", &format!("g{i}"))], Some(defs), &mut nodes);
            node(
                "rect",
                &[("width", "1"), ("height", "1")],
                Some(g),
                &mut nodes,
            );
            node("use", &[("href", &next)], Some(g), &mut nodes);
        }
        node("use", &[("href", "#g0")], Some(root), &mut nodes);
        let r = doc_from_svg_tree(&XmlTree { nodes }, &options()).unwrap();
        assert_eq!(r.report.uses, 2_000);
        assert_eq!(r.report.broken, 1);
        assert_eq!(r.report.shapes, 2_000);
    }

    fn num_field(v: &Json, k: &str) -> f64 {
        match v.get(k) {
            Json::Num(x) => *x,
            _ => f64::NAN,
        }
    }
}
