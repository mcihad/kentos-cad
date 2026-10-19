//! A page's content stream run (ISO 32000-1 §8–§9): the graphics state (`q`, `Q`, `cm`, line width, dash, colours),
//! paths (`m l c v y h re`) painted by `S s f F f* B B* b b*`, form XObjects run in place, texts shown (`Tj TJ ' "`)
//! with their place, height and turn, and marked content's optional content groups as layers. Curves are flattened to
//! within a hundredth of a point. Pictures, shadings and invisible text are not drawn: the pictures and shadings are
//! counted.

use std::collections::HashMap;
use std::rc::Rc;

use super::file::PdfFile;
use super::font::Font;
use super::object::{Dict, Lexer, Obj, Token};
use super::{PdfCurve, PdfLeft, PdfPath, PdfStroke, PdfText};

pub type M = [f64; 6];

pub const IDENTITY: M = [1.0, 0.0, 0.0, 1.0, 0.0, 0.0];

/// `a` then `b` (PDF's row-vector order: a point goes through `a` first).
pub fn mul(a: &M, b: &M) -> M {
    [
        a[0] * b[0] + a[1] * b[2],
        a[0] * b[1] + a[1] * b[3],
        a[2] * b[0] + a[3] * b[2],
        a[2] * b[1] + a[3] * b[3],
        a[4] * b[0] + a[5] * b[2] + b[4],
        a[4] * b[1] + a[5] * b[3] + b[5],
    ]
}

pub fn apply(m: &M, x: f64, y: f64) -> [f64; 2] {
    [m[0] * x + m[2] * y + m[4], m[1] * x + m[3] * y + m[5]]
}

/// Curves are cut into straight pieces within this many points of the curve.
const FLATNESS: f64 = 0.01;
/// The most points a page's paths keep.
pub const MAX_POINTS: usize = 4_000_000;
/// The most operators a page runs (its forms' included).
const MAX_OPERATORS: usize = 20_000_000;
/// The most forms nested.
const MAX_FORMS: usize = 16;

#[derive(Clone)]
struct Gs {
    ctm: M,
    stroke: [u8; 3],
    fill: [u8; 3],
    stroke_n: usize,
    fill_n: usize,
    width: f64,
    dashed: bool,
    // Text state.
    font: Option<Rc<Font>>,
    size: f64,
    char_space: f64,
    word_space: f64,
    hscale: f64,
    leading: f64,
    rise: f64,
    render: i64,
}

fn rgb(vals: &[f64]) -> [u8; 3] {
    let c = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    match vals.len() {
        1 => [c(vals[0]); 3],
        3 => [c(vals[0]), c(vals[1]), c(vals[2])],
        4 => {
            let k = 1.0 - vals[3].clamp(0.0, 1.0);
            [c((1.0 - vals[0]) * k), c((1.0 - vals[1]) * k), c((1.0 - vals[2]) * k)]
        }
        _ => [0, 0, 0],
    }
}

/// What a run gathers.
pub struct Out {
    pub paths: Vec<PdfPath>,
    pub texts: Vec<PdfText>,
    pub layers: Vec<String>,
    pub hidden: Vec<bool>,
    pub left: PdfLeft,
    pub points: usize,
}

pub struct Run<'f, 'a> {
    pub file: &'f PdfFile<'a>,
    pub out: Out,
    /// The optional content groups switched off by the document's default view, by object number.
    pub off: Vec<u32>,
    fonts: HashMap<String, Rc<Font>>,
    operators: usize,
    layer_of_group: HashMap<u32, usize>,
}

impl<'f, 'a> Run<'f, 'a> {
    pub fn new(file: &'f PdfFile<'a>, off: Vec<u32>) -> Self {
        Run {
            file,
            out: Out { paths: Vec::new(), texts: Vec::new(), layers: Vec::new(), hidden: Vec::new(), left: PdfLeft::default(), points: 0 },
            off,
            fonts: HashMap::new(),
            operators: 0,
            layer_of_group: HashMap::new(),
        }
    }

    /// A layer for an optional content group (its name), made the first time.
    fn layer_for(&mut self, group: &Obj) -> Option<usize> {
        let Obj::Ref(n, _) = group else {
            return None;
        };
        if let Some(i) = self.layer_of_group.get(n) {
            return Some(*i);
        }
        let d = self.file.resolve(group);
        let d = d.dict()?;
        // A membership dictionary names its groups: its first.
        let (name, num) = match d.get("Type").and_then(Obj::name) {
            Some("OCMD") => {
                let g = match self.file.field(d, "OCGs") {
                    Obj::Array(a) => a.first().cloned(),
                    o @ Obj::Dict(_) => Some(o),
                    _ => d.get("OCGs").cloned(),
                }?;
                return self.layer_for(&g);
            }
            _ => {
                let name = match self.file.field(d, "Name") {
                    Obj::Str(s) => decode_text_string(&s),
                    _ => format!("Katman {n}"),
                };
                (name, *n)
            }
        };
        let i = match self.out.layers.iter().position(|l| *l == name) {
            Some(i) => i,
            None => {
                self.out.layers.push(name);
                self.out.hidden.push(self.off.contains(&num));
                self.out.layers.len() - 1
            }
        };
        self.layer_of_group.insert(num, i);
        Some(i)
    }

    /// Runs a page's content stream with `resources`, from the initial graphics state with the matrix `ctm`.
    pub fn content(&mut self, data: &[u8], resources: &Dict, ctm: M, depth: usize, forms: &mut Vec<u32>) {
        let gs = Gs {
            ctm,
            stroke: [0; 3],
            fill: [0; 3],
            stroke_n: 1,
            fill_n: 1,
            width: 1.0,
            dashed: false,
            font: None,
            size: 0.0,
            char_space: 0.0,
            word_space: 0.0,
            hscale: 1.0,
            leading: 0.0,
            rise: 0.0,
            render: 0,
        };
        self.run(data, resources, gs, depth, forms);
    }

    /// Runs a content stream from the graphics state `gs` (a form's: the state where it is drawn, ISO 32000-1 §8.10).
    fn run(&mut self, data: &[u8], resources: &Dict, mut gs: Gs, depth: usize, forms: &mut Vec<u32>) {
        let mut stack: Vec<Gs> = Vec::new();
        let mut layers: Vec<Option<usize>> = Vec::new();
        let mut operands: Vec<Obj> = Vec::new();
        let mut parts: Vec<(Vec<[f64; 2]>, bool)> = Vec::new();
        // Each part's cubic pieces, beside `parts` (docs/adr/0352 §2).
        let mut curves: Vec<Vec<PdfCurve>> = Vec::new();
        let mut cur: Vec<[f64; 2]> = Vec::new();
        let mut cur_curves: Vec<PdfCurve> = Vec::new();
        let mut start = [0.0, 0.0];
        let mut last = [0.0, 0.0];
        let mut tm = IDENTITY;
        let mut tlm = IDENTITY;
        let mut lx = Lexer::new(data, 0);
        loop {
            let tok = match lx.token() {
                Ok(Some(t)) => t,
                Ok(None) => break,
                Err(_) => {
                    operands.clear();
                    continue;
                }
            };
            let op = match tok {
                Token::Word(w) => w,
                Token::ArrayStart => {
                    lx.pos -= 1;
                    match lx.object() {
                        Ok(o) => operands.push(o),
                        Err(_) => break,
                    }
                    continue;
                }
                Token::DictStart => {
                    lx.pos -= 2;
                    match lx.object() {
                        Ok(o) => operands.push(o),
                        Err(_) => break,
                    }
                    continue;
                }
                Token::Obj(o) => {
                    operands.push(o);
                    continue;
                }
                _ => {
                    operands.clear();
                    continue;
                }
            };
            self.operators += 1;
            if self.operators > MAX_OPERATORS || self.out.points > MAX_POINTS {
                self.out.left.truncated = true;
                return;
            }
            let n = |i: usize| operands.get(i).and_then(Obj::num).unwrap_or(0.0);
            let nums: Vec<f64> = operands.iter().filter_map(Obj::num).collect();
            let layer = layers.iter().rev().flatten().next().copied();
            match op.as_str() {
                "q" => stack.push(gs.clone()),
                "Q" => {
                    if let Some(g) = stack.pop() {
                        gs = g;
                    }
                }
                "cm" if nums.len() >= 6 => {
                    gs.ctm = mul(&[nums[0], nums[1], nums[2], nums[3], nums[4], nums[5]], &gs.ctm);
                }
                "w" => gs.width = n(0),
                "d" => gs.dashed = operands.first().and_then(Obj::array).is_some_and(|a| !a.is_empty()),
                "gs" => {
                    if let Some(name) = operands.first().and_then(Obj::name)
                        && let Obj::Dict(egs) = self.file.field(resources, "ExtGState")
                        && let Obj::Dict(g) = self.file.field(&egs, name)
                    {
                        if let Some(w) = self.file.field(&g, "LW").num() {
                            gs.width = w;
                        }
                        if let Obj::Array(d) = self.file.field(&g, "D") {
                            gs.dashed = d.first().and_then(Obj::array).is_some_and(|a| !a.is_empty());
                        }
                    }
                }
                "m" => {
                    if cur.len() > 1 {
                        parts.push((std::mem::take(&mut cur), false));
                        curves.push(std::mem::take(&mut cur_curves));
                    }
                    cur_curves.clear();
                    let p = apply(&gs.ctm, n(0), n(1));
                    cur = vec![p];
                    start = p;
                    last = p;
                }
                "l" => {
                    let p = apply(&gs.ctm, n(0), n(1));
                    if cur.is_empty() {
                        cur.push(last);
                    }
                    cur.push(p);
                    last = p;
                }
                "c" | "v" | "y" if nums.len() >= 4 => {
                    let (c1, c2, e) = match op.as_str() {
                        "c" if nums.len() >= 6 => (
                            apply(&gs.ctm, nums[0], nums[1]),
                            apply(&gs.ctm, nums[2], nums[3]),
                            apply(&gs.ctm, nums[4], nums[5]),
                        ),
                        "v" => (last, apply(&gs.ctm, nums[0], nums[1]), apply(&gs.ctm, nums[2], nums[3])),
                        "y" => {
                            let e = apply(&gs.ctm, nums[2], nums[3]);
                            (apply(&gs.ctm, nums[0], nums[1]), e, e)
                        }
                        _ => {
                            operands.clear();
                            continue;
                        }
                    };
                    if cur.is_empty() {
                        cur.push(last);
                    }
                    let at = cur.len() - 1;
                    flatten_cubic([last, c1, c2, e], FLATNESS, &mut cur);
                    cur_curves.push(PdfCurve { at, n: cur.len() - 1 - at, c: [last, c1, c2, e] });
                    last = e;
                }
                "h" => {
                    if cur.len() > 1 {
                        parts.push((std::mem::take(&mut cur), true));
                        curves.push(std::mem::take(&mut cur_curves));
                    }
                    cur.clear();
                    cur_curves.clear();
                    last = start;
                }
                "re" if nums.len() >= 4 => {
                    if cur.len() > 1 {
                        parts.push((std::mem::take(&mut cur), false));
                        curves.push(std::mem::take(&mut cur_curves));
                    }
                    cur_curves.clear();
                    let (x, y, w, h) = (nums[0], nums[1], nums[2], nums[3]);
                    let ring = vec![
                        apply(&gs.ctm, x, y),
                        apply(&gs.ctm, x + w, y),
                        apply(&gs.ctm, x + w, y + h),
                        apply(&gs.ctm, x, y + h),
                    ];
                    last = ring[0];
                    start = ring[0];
                    parts.push((ring, true));
                    curves.push(Vec::new());
                    cur.clear();
                }
                "S" | "s" | "f" | "F" | "f*" | "B" | "B*" | "b" | "b*" | "n" => {
                    if cur.len() > 1 {
                        let closes = op == "s" || op == "b" || op == "b*";
                        parts.push((std::mem::take(&mut cur), closes));
                        curves.push(std::mem::take(&mut cur_curves));
                    }
                    cur.clear();
                    cur_curves.clear();
                    let painted = std::mem::take(&mut parts);
                    let painted_curves = std::mem::take(&mut curves);
                    if op != "n" && !painted.is_empty() {
                        let stroke = matches!(op.as_str(), "S" | "s" | "B" | "B*" | "b" | "b*").then(|| PdfStroke {
                            color: gs.stroke,
                            width: gs.width * scale_of(&gs.ctm),
                            dashed: gs.dashed,
                        });
                        let fill = matches!(op.as_str(), "f" | "F" | "f*" | "B" | "B*" | "b" | "b*").then_some(gs.fill);
                        self.out.points += painted.iter().map(|p| p.0.len()).sum::<usize>();
                        self.out.paths.push(PdfPath { parts: painted, curves: painted_curves, stroke, fill, layer });
                    }
                }
                "W" | "W*" => {}
                "CS" | "cs" => {
                    let comps = operands.first().and_then(Obj::name).map_or(1, |name| self.components(resources, name));
                    if op == "CS" {
                        gs.stroke_n = comps;
                        gs.stroke = [0; 3];
                    } else {
                        gs.fill_n = comps;
                        gs.fill = [0; 3];
                    }
                }
                "SC" | "SCN" => {
                    if gs.stroke_n > 0 && nums.len() >= gs.stroke_n {
                        gs.stroke = rgb(&nums[..gs.stroke_n]);
                    }
                }
                "sc" | "scn" => {
                    if gs.fill_n > 0 && nums.len() >= gs.fill_n {
                        gs.fill = rgb(&nums[..gs.fill_n]);
                    }
                }
                "G" => (gs.stroke, gs.stroke_n) = (rgb(&nums[..nums.len().min(1)]), 1),
                "g" => (gs.fill, gs.fill_n) = (rgb(&nums[..nums.len().min(1)]), 1),
                "RG" => (gs.stroke, gs.stroke_n) = (rgb(&nums[..nums.len().min(3)]), 3),
                "rg" => (gs.fill, gs.fill_n) = (rgb(&nums[..nums.len().min(3)]), 3),
                "K" => (gs.stroke, gs.stroke_n) = (rgb(&nums[..nums.len().min(4)]), 4),
                "k" => (gs.fill, gs.fill_n) = (rgb(&nums[..nums.len().min(4)]), 4),
                "BT" => {
                    tm = IDENTITY;
                    tlm = IDENTITY;
                }
                "Tc" => gs.char_space = n(0),
                "Tw" => gs.word_space = n(0),
                "Tz" => gs.hscale = n(0) / 100.0,
                "TL" => gs.leading = n(0),
                "Ts" => gs.rise = n(0),
                "Tr" => gs.render = n(0) as i64,
                "Tf" => {
                    gs.size = n(1);
                    gs.font = operands.first().and_then(Obj::name).and_then(|name| self.font(resources, name));
                }
                "Td" => {
                    tlm = mul(&[1.0, 0.0, 0.0, 1.0, n(0), n(1)], &tlm);
                    tm = tlm;
                }
                "TD" => {
                    gs.leading = -n(1);
                    tlm = mul(&[1.0, 0.0, 0.0, 1.0, n(0), n(1)], &tlm);
                    tm = tlm;
                }
                "Tm" if nums.len() >= 6 => {
                    tlm = [nums[0], nums[1], nums[2], nums[3], nums[4], nums[5]];
                    tm = tlm;
                }
                "T*" => {
                    tlm = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -gs.leading], &tlm);
                    tm = tlm;
                }
                "Tj" | "'" | "\"" | "TJ" => {
                    if op == "'" || op == "\"" {
                        if op == "\"" {
                            gs.word_space = n(0);
                            gs.char_space = n(1);
                        }
                        tlm = mul(&[1.0, 0.0, 0.0, 1.0, 0.0, -gs.leading], &tlm);
                        tm = tlm;
                    }
                    let items: Vec<Obj> = match (op.as_str(), operands.last()) {
                        ("TJ", Some(Obj::Array(a))) => a.clone(),
                        (_, Some(o @ Obj::Str(_))) => vec![o.clone()],
                        _ => Vec::new(),
                    };
                    self.show(&items, &gs, &mut tm, layer);
                }
                "Do" => {
                    if let Some(name) = operands.first().and_then(Obj::name) {
                        self.xobject(resources, name, &gs, layer, depth, forms, &mut layers);
                    }
                }
                "sh" => self.out.left.shadings += 1,
                "BI" => {
                    // An inline picture: skipped to its EI.
                    let b = lx.src;
                    let mut i = lx.pos;
                    while i + 2 < b.len() {
                        if super::object::is_white(b[i]) && b[i + 1] == b'E' && b[i + 2] == b'I'
                            && b.get(i + 3).is_none_or(|c| super::object::is_white(*c))
                        {
                            break;
                        }
                        i += 1;
                    }
                    lx.pos = (i + 3).min(b.len());
                    self.out.left.images += 1;
                }
                "BMC" => layers.push(None),
                "BDC" => {
                    let props = operands.get(1).cloned();
                    let tag = operands.first().and_then(Obj::name).unwrap_or("");
                    let group = if tag == "OC" {
                        match props {
                            Some(Obj::Name(p)) => match self.file.field(resources, "Properties") {
                                Obj::Dict(pd) => pd.get(&p).cloned(),
                                _ => None,
                            },
                            Some(Obj::Dict(d)) => d.get("OC").cloned(),
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let l = group.and_then(|g| self.layer_for(&g));
                    layers.push(l);
                }
                "EMC" => {
                    layers.pop();
                }
                _ => {}
            }
            operands.clear();
        }
    }

    fn components(&self, resources: &Dict, name: &str) -> usize {
        match name {
            "DeviceGray" | "G" | "CalGray" => return 1,
            "DeviceRGB" | "RGB" | "CalRGB" => return 3,
            "DeviceCMYK" | "CMYK" => return 4,
            "Pattern" => return 0,
            _ => {}
        }
        let Obj::Dict(spaces) = self.file.field(resources, "ColorSpace") else { return 1 };
        match self.file.field(&spaces, name) {
            Obj::Array(a) => match a.first().and_then(Obj::name) {
                Some("ICCBased") => a
                    .get(1)
                    .map(|s| self.file.resolve(s))
                    .and_then(|s| s.dict().and_then(|d| d.get("N")).and_then(Obj::num))
                    .map_or(3, |n| n as usize),
                Some("CalRGB" | "Lab") => 3,
                Some("CalGray") => 1,
                Some("Pattern") => 0,
                // Separation, DeviceN, Indexed: their tints are not a colour here; drawn in black.
                Some("Separation") => 1,
                Some("Indexed") => 1,
                _ => 1,
            },
            Obj::Name(n) => self.components(resources, &n),
            _ => 1,
        }
    }

    fn font(&mut self, resources: &Dict, name: &str) -> Option<Rc<Font>> {
        let Obj::Dict(fonts) = self.file.field(resources, "Font") else { return None };
        let key = match fonts.get(name) {
            Some(Obj::Ref(n, _)) => format!("#{n}"),
            _ => format!("/{name}"),
        };
        if let Some(f) = self.fonts.get(&key) {
            return Some(f.clone());
        }
        let Obj::Dict(d) = self.file.field(&fonts, name) else { return None };
        let f = Rc::new(Font::of(self.file, &d));
        self.fonts.insert(key, f.clone());
        Some(f)
    }

    fn show(&mut self, items: &[Obj], gs: &Gs, tm: &mut M, layer: Option<usize>) {
        let Some(font) = gs.font.clone() else { return };
        let origin = *tm;
        let mut text = String::new();
        let mut unreadable = 0u32;
        let mut any = false;
        for item in items {
            match item {
                Obj::Num(k) => {
                    let tx = -k / 1000.0 * gs.size * gs.hscale;
                    *tm = mul(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], tm);
                    // A wide gap between strings reads as a space.
                    if *k < -200.0 && !text.ends_with(' ') && !text.is_empty() {
                        text.push(' ');
                    }
                }
                Obj::Str(s) => {
                    for (code, letters, w) in font.codes(s) {
                        any = true;
                        match letters {
                            Some(l) => text.push_str(&l),
                            None => unreadable += 1,
                        }
                        let space = if font.code_bytes == 1 && code == 32 { gs.word_space } else { 0.0 };
                        let tx = (w * gs.size + gs.char_space + space) * gs.hscale;
                        *tm = mul(&[1.0, 0.0, 0.0, 1.0, tx, 0.0], tm);
                    }
                }
                _ => {}
            }
        }
        if !any || gs.render == 3 || gs.render == 7 {
            return;
        }
        self.out.left.unreadable += unreadable;
        let text = text.trim_end().to_owned();
        if text.trim().is_empty() {
            return;
        }
        let trm = mul(&[gs.size * gs.hscale, 0.0, 0.0, gs.size, 0.0, gs.rise], &mul(&origin, &gs.ctm));
        let p = [trm[4], trm[5]];
        let height = (trm[2] * trm[2] + trm[3] * trm[3]).sqrt();
        let angle = crate::math::atan2(trm[1], trm[0]);
        if !(height > 0.0 && height.is_finite()) {
            return;
        }
        self.out.texts.push(PdfText { p, height, angle, text, color: gs.fill, layer });
    }

    #[allow(clippy::too_many_arguments)]
    fn xobject(
        &mut self,
        resources: &Dict,
        name: &str,
        gs: &Gs,
        layer: Option<usize>,
        depth: usize,
        forms: &mut Vec<u32>,
        _layers: &mut [Option<usize>],
    ) {
        let Obj::Dict(xs) = self.file.field(resources, "XObject") else { return };
        let r = xs.get(name).cloned();
        let num = match r {
            Some(Obj::Ref(n, _)) => Some(n),
            _ => None,
        };
        let Some(r) = r else { return };
        let obj = self.file.resolve(&r);
        let Obj::Stream(d, _) = &obj else { return };
        match d.get("Subtype").map(|s| self.file.resolve(s)).as_ref().and_then(Obj::name) {
            Some("Image") => self.out.left.images += 1,
            Some("Form") => {
                if depth >= MAX_FORMS || num.is_some_and(|n| forms.contains(&n)) {
                    return;
                }
                let m = match self.file.field(d, "Matrix") {
                    Obj::Array(a) if a.len() >= 6 => {
                        let v: Vec<f64> = a.iter().map(|x| self.file.resolve(x).num().unwrap_or(0.0)).collect();
                        [v[0], v[1], v[2], v[3], v[4], v[5]]
                    }
                    _ => IDENTITY,
                };
                let own = match self.file.field(d, "Resources") {
                    Obj::Dict(r) => r,
                    _ => resources.clone(),
                };
                let Ok(Some(data)) = self.file.stream_data(&obj) else { return };
                // A form with its own optional content group draws on its layer.
                let oc = d.get("OC").cloned();
                let form_layer = oc.and_then(|g| self.layer_for(&g)).or(layer);
                if let Some(n) = num {
                    forms.push(n);
                }
                let before = self.out.paths.len();
                let texts_before = self.out.texts.len();
                let mut inner = gs.clone();
                inner.ctm = mul(&m, &gs.ctm);
                self.run(&data, &own, inner, depth + 1, forms);
                if let Some(l) = form_layer {
                    for p in &mut self.out.paths[before..] {
                        p.layer.get_or_insert(l);
                    }
                    for t in &mut self.out.texts[texts_before..] {
                        t.layer.get_or_insert(l);
                    }
                }
                if num.is_some() {
                    forms.pop();
                }
            }
            _ => {}
        }
    }
}

/// How much a matrix scales lengths (the square root of its area factor).
pub fn scale_of(m: &M) -> f64 {
    (m[0] * m[3] - m[1] * m[2]).abs().sqrt()
}

/// A cubic Bézier's points after its start, within `tol` of the curve (docs/adr/0352 §3): the control points' distance
/// from the chord bounds the curve's, so `ceil(sqrt(0.75 d / tol))` pieces, between 1 and 512, at even steps of t.
pub fn flatten_cubic(c: [[f64; 2]; 4], tol: f64, out: &mut Vec<[f64; 2]>) {
    let [p0, p1, p2, p3] = c;
    let d = |p: [f64; 2]| {
        let (dx, dy) = (p3[0] - p0[0], p3[1] - p0[1]);
        let len = (dx * dx + dy * dy).sqrt();
        if len < 1e-12 {
            crate::math::hypot(p[0] - p0[0], p[1] - p0[1])
        } else {
            ((p[0] - p0[0]) * dy - (p[1] - p0[1]) * dx).abs() / len
        }
    };
    let dist = d(p1).max(d(p2));
    let n = ((dist * 0.75 / tol).sqrt().ceil() as usize).clamp(1, 512);
    for i in 1..=n {
        let t = i as f64 / n as f64;
        let u = 1.0 - t;
        let (a, b, c, e) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
        out.push([
            a * p0[0] + b * p1[0] + c * p2[0] + e * p3[0],
            a * p0[1] + b * p1[1] + c * p2[1] + e * p3[1],
        ]);
    }
}

/// A PDF text string (a name of a layer): UTF-16 with its byte order mark, else PDFDocEncoding (Latin-1 near enough).
pub fn decode_text_string(s: &[u8]) -> String {
    if s.starts_with(&[0xFE, 0xFF]) {
        let units: Vec<u16> = s[2..].chunks(2).map(|c| u16::from(c[0]) << 8 | u16::from(*c.get(1).unwrap_or(&0))).collect();
        return String::from_utf16_lossy(&units);
    }
    if s.starts_with(&[0xEF, 0xBB, 0xBF]) {
        return String::from_utf8_lossy(&s[3..]).into_owned();
    }
    s.iter().map(|&b| b as char).collect()
}
