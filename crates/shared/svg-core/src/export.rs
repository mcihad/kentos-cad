//! The drawing written out (`apps/web/src/style/svg/exportSvg.ts`,
//! docs/STYLE.md §7): as a symbol SVG (the symbol's colours stay
//! currentColor and param(stroke), as the library keeps it), as a plain SVG
//! for other programs (colours resolved to the preview colours, alpha as
//! fill/stroke-opacity, the size in mm), or as the editor's source view
//! (one element per line with ids, names and hidden shapes, and where each
//! shape's element sits, in UTF-16 code units as the page counts text).
//! Selection only crops the canvas to the chosen shapes. Also the pixel
//! size of a PNG export and its DPI chunk.

use std::collections::HashSet;

use kentos_geometry_core::api::json::{FromJson, Json, ToJson, write_str};
use kentos_geometry_core::jsmath::{js_max, js_min, js_round};
use kentos_style_core::js::text::{slice, utf16_len};

use crate::import::{ReferenceSpec, text_of};
use crate::model::{element_of, n, shape_box};
use crate::path::{empty_box, grow_box};
use crate::shape::Obj;
use crate::values::parse_hex;

/// `esc`: the text as an attribute value or element text.
fn esc(v: &str) -> String {
    v.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// The canvas an export covers.
#[derive(Clone, Debug, PartialEq)]
pub struct ExportBox {
    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

kentos_geometry_core::json_struct!(out ExportBox { x, y, w, h });

/// The canvas an export covers: the whole drawing, or the chosen shapes with their strokes.
pub fn export_box(
    width: f64,
    height: f64,
    shapes: &[Obj],
    only: Option<&HashSet<String>>,
) -> Result<ExportBox, String> {
    let whole = ExportBox {
        x: 0.0,
        y: 0.0,
        w: width,
        h: height,
    };
    let Some(only) = only else { return Ok(whole) };
    let chosen: Vec<&Obj> = shapes
        .iter()
        .filter(|s| s.text("id").is_some_and(|id| only.contains(id)) && !s.is("hidden"))
        .collect();
    let mut b = empty_box();
    for s in &chosen {
        let sb = shape_box(s)?;
        grow_box(&mut b, sb.min_x, sb.min_y);
        grow_box(&mut b, sb.max_x, sb.max_y);
    }
    if !b.min_x.is_finite() {
        return Ok(whole);
    }
    let pad = chosen.iter().fold(0.0, |m, s| {
        js_max(
            m,
            if s.text("stroke") != Some("none") {
                s.num("strokeWidth") / 2.0
            } else {
                0.0
            },
        )
    });
    Ok(ExportBox {
        x: b.min_x - pad,
        y: b.min_y - pad,
        w: js_max(b.max_x - b.min_x + 2.0 * pad, 1e-3),
        h: js_max(b.max_y - b.min_y + 2.0 * pad, 1e-3),
    })
}

/// How the drawing is written.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SvgTextOptions {
    /// The symbol's colours resolved to these (ink, second): a plain SVG; absent keeps them as parameters.
    pub colors: Option<(String, String)>,
    /// Only these shapes, on a canvas cropped to them.
    pub only: Option<HashSet<String>>,
    /// One element per line, indented.
    pub pretty: bool,
    /// A tracing reference kept in the file (never drawn: it sits in `<defs>`).
    pub reference: Option<ReferenceSpec>,
}

impl FromJson for SvgTextOptions {
    fn from_json(v: &Json) -> Result<SvgTextOptions, String> {
        Ok(SvgTextOptions {
            colors: match v.get("colors") {
                c @ Json::Obj(_) => Some((text_of(c.get("ink")), text_of(c.get("second")))),
                _ => None,
            },
            only: match v.get("only") {
                Json::Arr(ids) => Some(
                    ids.iter()
                        .filter_map(|id| match id {
                            Json::Str(s) => Some(s.clone()),
                            _ => None,
                        })
                        .collect(),
                ),
                _ => None,
            },
            pretty: crate::shape::truthy(v.get("pretty")),
            reference: match v.get("reference") {
                r @ Json::Obj(_) => Some(ReferenceSpec::from_json(r)?),
                _ => None,
            },
        })
    }
}

/// The written text, and each shape's element as a range of UTF-16 code units (in the order shapes were written).
pub struct Written {
    pub text: String,
    pub spans: Vec<(Json, usize, usize)>,
}

impl ToJson for Written {
    fn write_json(&self, out: &mut String) {
        out.push_str("{\"text\":");
        write_str(out, &self.text);
        out.push_str(",\"spans\":[");
        for (i, (id, a, b)) in self.spans.iter().enumerate() {
            if i > 0 {
                out.push(',');
            }
            out.push('[');
            id.write_json(out);
            out.push_str(&format!(",{a},{b}]"));
        }
        out.push_str("]}");
    }
}

/// Text grown line by line, with its length in UTF-16 code units.
struct Out {
    text: String,
    units: usize,
    pretty: bool,
}

impl Out {
    fn line(&mut self, depth: usize, s: &str) {
        if self.pretty {
            self.text.push('\n');
            self.text.push_str(&"  ".repeat(depth));
            self.units += 1 + 2 * depth;
        }
        self.text.push_str(s);
        self.units += utf16_len(s);
    }
}

/// JavaScript's `===` between two field values (undefined as None).
fn same(a: &Option<Json>, b: &Option<Json>) -> bool {
    match (a, b) {
        (None, None) => true,
        (Some(Json::Null), Some(Json::Null)) => true,
        (Some(Json::Bool(x)), Some(Json::Bool(y))) => x == y,
        (Some(Json::Num(x)), Some(Json::Num(y))) => x == y,
        (Some(Json::Str(x)), Some(Json::Str(y))) => x == y,
        _ => false,
    }
}

fn is_truthy(v: &Option<Json>) -> bool {
    v.as_ref().is_some_and(crate::shape::truthy)
}

/// `/^#[0-9a-f]{8}$/i`.
fn hex8(v: &str) -> bool {
    v.len() == 9 && v.starts_with('#') && v[1..].bytes().all(|b| b.is_ascii_hexdigit())
}

fn element(
    s: &Obj,
    paint: &dyn Fn(&Json) -> String,
    plain: bool,
    source: bool,
) -> Result<String, String> {
    let e = element_of(s, &paint(s.get("fill")), &paint(s.get("stroke")))?
        .ok_or_else(|| format!("Bilinmeyen şekil türü “{}”.", s.kind()))?;
    let mut attrs = e.attrs;
    if plain {
        // #RRGGBBAA is not read everywhere: the alpha goes to fill-opacity / stroke-opacity.
        let mut out = Vec::with_capacity(attrs.len() + 2);
        for (k, v) in attrs {
            if (k == "fill" || k == "stroke") && hex8(&v) {
                let key = format!("{k}-opacity");
                out.push((k, slice(&v, 0, Some(7))));
                out.push((key, n(parse_hex(&slice(&v, 7, None)) / 255.0)));
            } else {
                out.push((k, v));
            }
        }
        attrs = out;
    }
    if source {
        let mut front = vec![("id".to_string(), text_of(s.get("id")))];
        if s.is("name") {
            front.push(("data-name".to_string(), text_of(s.get("name"))));
        }
        front.extend(attrs);
        attrs = front;
        if s.is("hidden") {
            attrs.push(("display".to_string(), "none".to_string()));
        }
    }
    let a = attrs
        .iter()
        .map(|(k, v)| format!("{k}=\"{}\"", esc(v)))
        .collect::<Vec<_>>()
        .join(" ");
    Ok(match &e.text {
        Some(t) => format!("<{} {a}>{}</{}>", e.tag, esc(t), e.tag),
        None => format!("<{} {a}/>", e.tag),
    })
}

/// The drawing as SVG text: a symbol SVG, a plain one (`colors`), or the editor's source view.
pub fn write(doc: &Obj, opts: &SvgTextOptions, source: bool) -> Result<Written, String> {
    let plain = opts.colors.is_some();
    let pretty = source || opts.pretty;
    let shapes: Vec<Obj> =
        Vec::from_json(doc.get("shapes")).map_err(|e| format!("“shapes”: {e}"))?;
    let b = export_box(
        doc.num("width"),
        doc.num("height"),
        &shapes,
        opts.only.as_ref(),
    )?;
    let mut root = vec![
        "xmlns=\"http://www.w3.org/2000/svg\"".to_string(),
        format!("viewBox=\"{} {} {} {}\"", n(b.x), n(b.y), n(b.w), n(b.h)),
    ];
    if plain && doc.is("sizeMm") {
        // The physical size: 1 unit = sizeMm / width mm.
        let k = doc.num("sizeMm") / doc.num("width");
        root.push(format!("width=\"{}mm\"", n(b.w * k)));
        root.push(format!("height=\"{}mm\"", n(b.h * k)));
    } else {
        root.push(format!("width=\"{}\"", n(b.w)));
        root.push(format!("height=\"{}\"", n(b.h)));
    }
    if !plain && doc.is("sizeMm") {
        root.push(format!("data-size-mm=\"{}\"", n(doc.num("sizeMm"))));
    }
    if !plain && doc.is("background") {
        root.push(format!(
            "data-background=\"{}\"",
            esc(&text_of(doc.get("background")))
        ));
    }
    let paint = |p: &Json| -> String {
        let p = text_of(p);
        let (ink, second) = match &opts.colors {
            None => ("currentColor", "param(stroke) #000000"),
            Some((ink, second)) => (ink.as_str(), second.as_str()),
        };
        match p.as_str() {
            "fill" => ink.to_string(),
            "stroke" => second.to_string(),
            _ => p,
        }
    };
    let head = format!("<svg {}>", root.join(" "));
    let mut out = Out {
        units: utf16_len(&head),
        text: head,
        pretty,
    };
    let mut spans: Vec<(Json, usize, usize)> = Vec::new();
    if let Some(r) = &opts.reference
        && !plain
        && !source
    {
        let attrs = [
            "data-kentos=\"reference\"".to_string(),
            format!("data-name=\"{}\"", esc(&r.name)),
            format!("data-locked=\"{}\"", if r.locked { 1 } else { 0 }),
            format!("href=\"{}\"", esc(&r.href)),
            format!("x=\"{}\"", n(r.x)),
            format!("y=\"{}\"", n(r.y)),
            format!("width=\"{}\"", n(r.width)),
            format!("height=\"{}\"", n(r.height)),
            format!("opacity=\"{}\"", n(r.opacity)),
            "preserveAspectRatio=\"none\"".to_string(),
        ];
        out.line(1, &format!("<defs><image {}/></defs>", attrs.join(" ")));
    }
    let mut open: Option<Json> = None;
    for s in &shapes {
        let id = s.field("id").unwrap_or(Json::Null);
        let chosen = match &opts.only {
            Some(only) => matches!(&id, Json::Str(id) if only.contains(id)),
            None => true,
        };
        if (s.is("hidden") && !source) || !chosen {
            continue;
        }
        let group = s.field("group");
        let grouped = is_truthy(&group);
        if !same(&group, &open) {
            if is_truthy(&open) {
                out.line(1, "</g>");
            }
            if grouped {
                let g = if plain {
                    "<g>".to_string()
                } else {
                    format!(
                        "<g data-group=\"{}\">",
                        esc(&text_of(group.as_ref().unwrap_or(&Json::Null)))
                    )
                };
                out.line(1, &g);
            }
            open = group;
        }
        let depth = if grouped { 2 } else { 1 };
        let at = out.units + if pretty { 1 + 2 * depth } else { 0 };
        out.line(depth, &element(s, &paint, plain, source)?);
        // A Map: a repeated id keeps its first place and takes the last range.
        match spans
            .iter_mut()
            .find(|(k, _, _)| same(&Some(k.clone()), &Some(id.clone())))
        {
            Some(slot) => {
                slot.1 = at;
                slot.2 = out.units;
            }
            None => spans.push((id, at, out.units)),
        }
    }
    if is_truthy(&open) {
        out.line(1, "</g>");
    }
    out.line(0, "</svg>");
    let mut text = out.text;
    if pretty {
        text.push('\n');
    }
    Ok(Written { text, spans })
}

// ── PNG ────────────────────────────────────────────────────────────────

/// Pixel size of a PNG export.
#[derive(Clone, Debug, PartialEq)]
pub struct PngSize {
    pub width: f64,
    pub height: f64,
}

kentos_geometry_core::json_struct!(out PngSize { width, height });

/// Pixel size of a PNG export of a `w × h` canvas: a width in pixels, or a
/// DPI with the drawing's width in mm (without one, a unit is a CSS pixel,
/// 96 dpi).
pub fn png_size(w: f64, h: f64, px: Option<f64>, dpi: f64, width_mm: Option<f64>) -> PngSize {
    let clamp = |v: f64| js_max(1.0, js_min(8192.0, js_round(v)));
    let width = match px {
        Some(px) => px,
        None => (width_mm.unwrap_or((w * 25.4) / 96.0) / 25.4) * dpi,
    };
    PngSize {
        width: clamp(width),
        height: clamp((width * h) / w),
    }
}

/// CRC-32 (ISO 3309, as PNG chunks carry it).
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = i as u32;
        for _ in 0..8 {
            c = if c & 1 == 1 {
                0xedb8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut c = 0xffff_ffffu32;
    for &b in bytes {
        c = table[((c ^ u32::from(b)) & 0xff) as usize] ^ (c >> 8);
    }
    c ^ 0xffff_ffff
}

/// `ToUint32`: a number as JavaScript stores it in a 32-bit field.
fn to_uint32(x: f64) -> u32 {
    if !x.is_finite() {
        return 0;
    }
    let m = x.trunc() % 4_294_967_296.0;
    (if m < 0.0 { m + 4_294_967_296.0 } else { m }) as u32
}

/// A PNG with its resolution recorded (a pHYs chunk after IHDR, replacing
/// one that was there), so print programs size it right; None when the
/// bytes are not a PNG (the caller keeps them as they are).
pub fn with_png_dpi(png: &[u8], dpi: f64) -> Option<Vec<u8>> {
    let kind = |at: usize| &png[at + 4..at + 8];
    if png.len() < 33 || kind(8) != b"IHDR" {
        return None;
    }
    let ppm = to_uint32(js_round(dpi / 0.0254)).to_be_bytes();
    let mut chunk = Vec::with_capacity(21);
    chunk.extend_from_slice(&9u32.to_be_bytes());
    chunk.extend_from_slice(b"pHYs");
    chunk.extend_from_slice(&ppm);
    chunk.extend_from_slice(&ppm);
    chunk.push(1); // per metre
    let crc = crc32(&chunk[4..17]);
    chunk.extend_from_slice(&crc.to_be_bytes());
    let mut out = Vec::with_capacity(png.len() + 21);
    out.extend_from_slice(&png[..33]);
    out.extend_from_slice(&chunk);
    // Chunks after IHDR, without an old pHYs.
    let mut at = 33usize;
    while at + 12 <= png.len() {
        let len = u32::from_be_bytes([png[at], png[at + 1], png[at + 2], png[at + 3]]);
        let end = (at as u64 + 12 + u64::from(len)).min(png.len() as u64 + 12) as usize;
        if kind(at) != b"pHYs" {
            out.extend_from_slice(&png[at..end.min(png.len())]);
        }
        at = end;
    }
    Some(out)
}
