//! Objects packed as numbers (docs/adr/0008: points cross as `Float64Array`):
//! how the page sends whole drawings to the store. A JSON array of 80 000
//! parcels is 26 MB and builds a tree of millions of small values before a
//! single object is read; the packed form is one run of numbers and a short
//! list of strings (layer ids, texts), read straight into shapes.
//! `apps/web/src/wasm/pack.ts` writes it. Every number is a float64, so ids and
//! coordinates arrive bit for bit (−0 and NaN included).
//!
//! Per object: `id, layer, label, kind`, then the kind's fields:
//!
//! | kind | fields |
//! |---|---|
//! | 0 point | x, y, hasZ, z |
//! | 1 line | a.x, a.y, b.x, b.y |
//! | 2 polyline, 3 polygon | path, holes |
//! | 4 circle | c.x, c.y, r |
//! | 5 arc | c.x, c.y, r, a0, a1 |
//! | 6 ellipse | c.x, c.y, major.x, major.y, ratio, t0, t1 |
//! | 7 xline, 8 ray | p.x, p.y, dir.x, dir.y |
//! | 9 spline | points, closed |
//! | 10 text | p.x, p.y, height, rotation, text, align, widthFactor, mask, boxWidth, lineSpacing, runs, textStyle?, font, bold, italic, oblique |
//! | 11 dimension | a.x, a.y, b.x, b.y, offset, height, text?, style?, hasAngle, angle, hasC, c.x, c.y, mask, za, zb, dimStyle?, arrow, arrowSize, extOffset, extBeyond, textGap, centre, decimals, unit?, prefix?, suffix?, font |
//! | 12 hatch | points, hatch holes, pattern type, angle, spacing |
//! | 13 multi-part polygon | path, holes, part count, then per part: path, holes |
//! | 14 insert | p.x, p.y, scale, rotation, mirror, block, attributes |
//! | 15 leader | points, height, rotation, text?, arrow?, mask |
//! | 16 multi-part polyline | path, holes, part count, then per part: path, holes |
//! | 17 multi-point object | x, y, hasZ, z, point count, then per point: x, y, hasZ, z |
//!
//! `layer`, `text` and `style` are indices into the strings (−1: none);
//! `label` is 1 for a non-empty label. `points` is a count n and 2n
//! coordinates; a `path` is points and bulges (a count, −1 for none, then
//! the values); `holes` is a count (−1 for none) of paths; `hatch holes` a
//! count (−1 for none) of point lists; an insert's `attributes` a count
//! (−1 for none) of tag and value pairs of strings, its text values (docs/adr/0144
//! §7: what its block's attribute texts show); a leader's `arrow` (docs/adr/0146)
//! its name, −1 for the filled arrow. A field left out (`z`, `angle`, `c`, a
//! dimension's `za` and `zb`, docs/adr/0147)
//! still takes its numbers, NaN. A text's face and a dimension's look (docs/adr/0183):
//! the style ids, a value's unit, prefix and suffix as strings (−1 none), a typeface's place
//! in the core's tables (−1: the project's), bold, italic and centre as flags, the slant, the
//! sizes and the decimals NaN when absent, an arrowhead's place in `Arrow::ALL` (−1: the
//! tick): all cross, so a moved, copied or pasted object keeps them. A multi-part area (docs/adr/0143) is kind
//! 13, its first part as a polygon's fields and its other parts after them;
//! a one-part area stays kind 3, laid out as it always was. A multi-part
//! polyline (16) and a multi-point object (17) likewise (docs/adr/0174).
//!
//! The store answers in the same layout (`Packer`): moved, copied, arrayed
//! and pasted objects come back as numbers, not JSON
//! (`Store::transform_packed`; the page reads them with `pack.ts`
//! `unpackEntities`).

use std::collections::HashMap;

use super::Store;
use crate::api::json::Json;
use crate::entity::{Attrs, CellRange, HatchPattern, Part, PointPart, Shape};
use crate::geom::affine::Affine;
use crate::geom::arrangement::Ring;
use crate::geom::dimension::{Arrow, Look};
use crate::ops::transform::transform_shape;
use crate::text::face::Face;
use crate::text::paragraph::{Run, Script};
use crate::text::{Font, TextAlign};
use crate::vec2::Vec2;

/// A path's points, bulges and holes.
type Path = (Vec<Vec2>, Option<Vec<f64>>, Option<Vec<Ring>>);

struct Reader<'a> {
    nums: &'a [f64],
    at: usize,
    strings: &'a [String],
}

impl Reader<'_> {
    fn num(&mut self) -> Result<f64, String> {
        let v = *self
            .nums
            .get(self.at)
            .ok_or_else(|| format!("paket {}. sayıda bitti", self.at))?;
        self.at += 1;
        Ok(v)
    }

    /// A count or an index: a whole number, −1 for "none".
    fn int(&mut self) -> Result<Option<usize>, String> {
        let v = self.num()?;
        if v == -1.0 {
            return Ok(None);
        }
        // No count or index can exceed what was sent.
        let limit = self.nums.len().max(self.strings.len()) as f64;
        if v >= 0.0 && v.fract() == 0.0 && v <= limit {
            Ok(Some(v as usize))
        } else {
            Err(format!(
                "paketin {}. sayısı geçersiz bir sayaç ({v})",
                self.at
            ))
        }
    }

    fn count(&mut self) -> Result<usize, String> {
        self.int()?
            .ok_or_else(|| format!("paketin {}. sayısı sayaç olmalı", self.at))
    }

    fn flag(&mut self) -> Result<bool, String> {
        Ok(self.num()? != 0.0)
    }

    /// A typeface by its place in the tables; none at −1 (docs/adr/0183).
    fn font(&mut self) -> Result<Option<Font>, String> {
        match self.int()? {
            None => Ok(None),
            Some(i) => Font::from_index(i)
                .map(Some)
                .ok_or_else(|| format!("paketin {}. sayısı yazı tipi değil ({i})", self.at)),
        }
    }

    fn pt(&mut self) -> Result<Vec2, String> {
        Ok(Vec2::new(self.num()?, self.num()?))
    }

    fn points(&mut self) -> Result<Vec<Vec2>, String> {
        let n = self.count()?;
        (0..n).map(|_| self.pt()).collect()
    }

    fn values(&mut self) -> Result<Option<Vec<f64>>, String> {
        match self.int()? {
            None => Ok(None),
            Some(n) => (0..n)
                .map(|_| self.num())
                .collect::<Result<Vec<_>, _>>()
                .map(Some),
        }
    }

    fn string(&mut self) -> Result<Option<String>, String> {
        match self.int()? {
            None => Ok(None),
            Some(i) => self.strings.get(i).cloned().map(Some).ok_or_else(|| {
                format!(
                    "paketin {}. sayısı olmayan bir metni gösteriyor ({i})",
                    self.at
                )
            }),
        }
    }

    fn path(&mut self) -> Result<Path, String> {
        let pts = self.points()?;
        let bulges = self.values()?;
        let holes = match self.int()? {
            None => None,
            Some(n) => Some(
                (0..n)
                    .map(|_| {
                        Ok(Ring {
                            pts: self.points()?,
                            bulges: self.values()?,
                        })
                    })
                    .collect::<Result<Vec<_>, String>>()?,
            ),
        };
        Ok((pts, bulges, holes))
    }

    fn shape(&mut self, kind: f64) -> Result<Shape, String> {
        if !(kind >= 0.0 && kind <= 18.0 && kind.fract() == 0.0) {
            return Err(format!(
                "paketin {}. sayısı bilinmeyen bir nesne türü ({kind})",
                self.at
            ));
        }
        Ok(match kind as i64 {
            0 | 17 => {
                let p = self.pt()?;
                let has_z = self.flag()?;
                let z = self.num()?;
                // A multi-point object's other points (docs/adr/0174).
                let parts = if kind == 17.0 {
                    let n = self.count()?;
                    let parts = (0..n)
                        .map(|_| {
                            let p = self.pt()?;
                            let has_z = self.flag()?;
                            let z = self.num()?;
                            Ok(PointPart {
                                p,
                                z: has_z.then_some(z),
                            })
                        })
                        .collect::<Result<Vec<_>, String>>()?;
                    Some(parts)
                } else {
                    None
                };
                Shape::Point {
                    p,
                    z: has_z.then_some(z),
                    parts,
                }
            }
            1 => Shape::Line {
                a: self.pt()?,
                b: self.pt()?,
            },
            2 | 3 => {
                let (pts, bulges, holes) = self.path()?;
                if kind == 2.0 {
                    Shape::Polyline {
                        pts,
                        bulges,
                        holes,
                        parts: None,
                    }
                } else {
                    Shape::Polygon {
                        pts,
                        bulges,
                        holes,
                        parts: None,
                    }
                }
            }
            14 => {
                let p = self.pt()?;
                let scale = self.num()?;
                let rotation = self.num()?;
                let mirror = self.flag()?;
                let block = self.string()?.unwrap_or_default();
                let attrs = match self.int()? {
                    None => None,
                    Some(n) => Some(Attrs(
                        (0..n)
                            .map(|_| {
                                let tag = self.string()?.unwrap_or_default();
                                let value = self.string()?.unwrap_or_default();
                                Ok((tag, Json::Str(value)))
                            })
                            .collect::<Result<Vec<_>, String>>()?,
                    )),
                };
                Shape::Insert {
                    p,
                    scale,
                    rotation,
                    mirror: mirror.then_some(true),
                    block,
                    attrs,
                }
            }
            13 | 16 => {
                let (pts, bulges, holes) = self.path()?;
                let n = self.count()?;
                let parts = (0..n)
                    .map(|_| {
                        let (pts, bulges, holes) = self.path()?;
                        Ok(Part { pts, bulges, holes })
                    })
                    .collect::<Result<Vec<_>, String>>()?;
                if kind == 16.0 {
                    Shape::Polyline {
                        pts,
                        bulges,
                        holes,
                        parts: Some(parts),
                    }
                } else {
                    Shape::Polygon {
                        pts,
                        bulges,
                        holes,
                        parts: Some(parts),
                    }
                }
            }
            4 => Shape::Circle {
                c: self.pt()?,
                r: self.num()?,
            },
            5 => Shape::Arc {
                c: self.pt()?,
                r: self.num()?,
                a0: self.num()?,
                a1: self.num()?,
            },
            6 => Shape::Ellipse {
                c: self.pt()?,
                major: self.pt()?,
                ratio: self.num()?,
                t0: self.num()?,
                t1: self.num()?,
            },
            7 => Shape::Xline {
                p: self.pt()?,
                dir: self.pt()?,
            },
            8 => Shape::Ray {
                p: self.pt()?,
                dir: self.pt()?,
            },
            9 => Shape::Spline {
                pts: self.points()?,
                closed: self.flag()?,
            },
            10 => {
                let (p, height, rotation) = (self.pt()?, self.num()?, self.num()?);
                let text = self.string()?.unwrap_or_default();
                // docs/adr/0145: the alignment's place in `TextAlign::ALL` (−1 none), the width
                // factor (NaN none), the mask a flag.
                let align =
                    match self.int()? {
                        Some(i) => Some(*TextAlign::ALL.get(i).ok_or_else(|| {
                            format!("paketin {}. sayısı yazı hizası değil", self.at)
                        })?),
                        None => None,
                    };
                let w = self.num()?;
                let mask = self.flag()?.then_some(true);
                // docs/adr/0182: the box width and line spacing (NaN none), then the runs: a
                // count, each its start, end, flags (`RUN_*`) and colour (a string, −1 none).
                let (box_width, line_spacing) = (self.num()?, self.num()?);
                let count = self.int()?.unwrap_or(0);
                let mut runs = Vec::with_capacity(count.min(1024));
                for _ in 0..count {
                    let (start, end) = (self.num()? as u32, self.num()? as u32);
                    let bits = self.num()? as u32;
                    runs.push(Run {
                        start,
                        end,
                        bold: bits & RUN_BOLD != 0,
                        italic: bits & RUN_ITALIC != 0,
                        underline: bits & RUN_UNDERLINE != 0,
                        script: if bits & RUN_SUPER != 0 {
                            Some(Script::Super)
                        } else if bits & RUN_SUB != 0 {
                            Some(Script::Sub)
                        } else {
                            None
                        },
                        color: self.string()?,
                    });
                }
                // docs/adr/0183: the style, the typeface (−1 the project's), bold, italic, the slant (NaN none).
                let style = self.string()?;
                let font = self.font()?;
                let (bold, italic) = (self.flag()?, self.flag()?);
                let oblique = self.num()?;
                Shape::Text {
                    p,
                    text,
                    height,
                    rotation,
                    align,
                    width_factor: (!w.is_nan()).then_some(w),
                    mask,
                    box_width: (!box_width.is_nan()).then_some(box_width),
                    line_spacing: (!line_spacing.is_nan()).then_some(line_spacing),
                    runs: (!runs.is_empty()).then_some(runs),
                    face: Face {
                        style,
                        font,
                        bold,
                        italic,
                        oblique: (!oblique.is_nan()).then_some(oblique),
                    },
                }
            }
            11 => {
                let a = self.pt()?;
                let b = self.pt()?;
                let offset = self.num()?;
                let height = self.num()?;
                let text = self.string()?;
                let style = self.string()?;
                let has_angle = self.flag()?;
                let angle = self.num()?;
                let has_c = self.flag()?;
                let c = self.pt()?;
                // docs/adr/0147: the mask a flag, the slope's elevations (NaN none).
                let mask = self.flag()?.then_some(true);
                let (za, zb) = (self.num()?, self.num()?);
                // docs/adr/0183: the style, the arrowhead (−1 the tick), the sizes (NaN none), centre, the
                // decimals (NaN none), the unit, prefix and suffix, the typeface.
                let dim_style = self.string()?;
                let arrow =
                    match self.int()? {
                        Some(i) => Some(*Arrow::ALL.get(i).ok_or_else(|| {
                            format!("paketin {}. sayısı ölçü oku değil", self.at)
                        })?),
                        None => None,
                    };
                let mut size = || self.num().map(|x| (!x.is_nan()).then_some(x));
                let (arrow_size, ext_offset, ext_beyond, text_gap) =
                    (size()?, size()?, size()?, size()?);
                let centre = self.flag()?;
                let decimals = self.num()?;
                let decimals = if decimals.is_nan() {
                    None
                } else if decimals >= 0.0
                    && decimals.fract() == 0.0
                    && decimals <= f64::from(u32::MAX)
                {
                    Some(decimals as u32)
                } else {
                    return Err(format!(
                        "paketin {}. sayısı basamak sayısı değil ({decimals})",
                        self.at
                    ));
                };
                let (unit, prefix, suffix) = (self.string()?, self.string()?, self.string()?);
                let font = self.font()?;
                Shape::Dimension {
                    a,
                    b,
                    offset,
                    height,
                    text,
                    style,
                    angle: has_angle.then_some(angle),
                    c: has_c.then_some(c),
                    mask,
                    za: (!za.is_nan()).then_some(za),
                    zb: (!zb.is_nan()).then_some(zb),
                    look: Look {
                        style: dim_style,
                        arrow,
                        arrow_size,
                        ext_offset,
                        ext_beyond,
                        text_gap,
                        centre,
                        decimals,
                        unit,
                        prefix,
                        suffix,
                        font,
                    },
                }
            }
            12 => {
                let ring = self.points()?;
                let holes = match self.int()? {
                    None => None,
                    Some(n) => Some(
                        (0..n)
                            .map(|_| self.points())
                            .collect::<Result<Vec<_>, _>>()?,
                    ),
                };
                Shape::Hatch {
                    ring,
                    holes,
                    pattern: HatchPattern {
                        kind: self.string()?.unwrap_or_default(),
                        angle: self.num()?,
                        spacing: self.num()?,
                    },
                }
            }
            15 => Shape::Leader {
                pts: self.points()?,
                height: self.num()?,
                rotation: self.num()?,
                text: self.string()?,
                arrow: self.string()?,
                mask: self.flag()?.then_some(true),
            },
            // docs/adr/0184: the corner, turn and height; the rows' heights and the columns'
            // widths; each cell's words, row by row; the merged ranges; the alignments (−1
            // none); the heading flag; the lines (−1 all, else `TABLE_GRIDS`); the frame's
            // width (NaN none); the face.
            18 => {
                let p = self.pt()?;
                let (rotation, height) = (self.num()?, self.num()?);
                let rows = self.values()?.unwrap_or_default();
                let columns = self.values()?.unwrap_or_default();
                let mut cells = Vec::with_capacity(rows.len().min(10_000));
                for _ in 0..rows.len() {
                    let row = (0..columns.len())
                        .map(|_| Ok(self.string()?.unwrap_or_default()))
                        .collect::<Result<Vec<_>, String>>()?;
                    cells.push(row);
                }
                let n = self.count()?;
                let mut merges = Vec::with_capacity(n.min(1024));
                for _ in 0..n {
                    merges.push(CellRange {
                        row: self.count()?,
                        col: self.count()?,
                        rows: self.count()?,
                        cols: self.count()?,
                    });
                }
                let aligns = match self.int()? {
                    None => None,
                    Some(n) => Some(
                        (0..n)
                            .map(|_| {
                                let i = self.count()?;
                                TABLE_ALIGNS.get(i).map(|a| (*a).to_owned()).ok_or_else(|| {
                                    format!("paketin {}. sayısı hiza değil ({i})", self.at)
                                })
                            })
                            .collect::<Result<Vec<_>, String>>()?,
                    ),
                };
                let header = self.flag()?.then_some(true);
                let grid = match self.int()? {
                    None => None,
                    Some(i) => {
                        Some(TABLE_GRIDS.get(i).map(|g| (*g).to_owned()).ok_or_else(|| {
                            format!("paketin {}. sayısı çizgi seçeneği değil ({i})", self.at)
                        })?)
                    }
                };
                let frame = self.num()?;
                let style = self.string()?;
                let font = self.font()?;
                let (bold, italic) = (self.flag()?, self.flag()?);
                let oblique = self.num()?;
                Shape::Table {
                    p,
                    rotation,
                    height,
                    rows,
                    columns,
                    cells,
                    merges: (!merges.is_empty()).then_some(merges),
                    aligns,
                    header,
                    grid,
                    frame: (!frame.is_nan()).then_some(frame),
                    // The store draws, picks and snaps: what the table came from is not packed.
                    source: None,
                    face: Face {
                        style,
                        font,
                        bold,
                        italic,
                        oblique: (!oblique.is_nan()).then_some(oblique),
                    },
                }
            }
            _ => {
                return Err(format!(
                    "paketin {}. sayısı bilinmeyen bir nesne türü ({kind})",
                    self.at
                ));
            }
        })
    }
}

/// Writes objects in the packed layout, the reader's other direction:
/// numbers, and the strings they point at, each once in the order first
/// met (as `pack.ts` writes them).
#[derive(Default)]
pub struct Packer {
    pub nums: Vec<f64>,
    pub strings: Vec<String>,
    index: HashMap<String, u32>,
}

/// A multi-line text's run's flags in a pack (docs/adr/0182), as `apps/web/src/wasm/pack.ts` writes them.
const RUN_BOLD: u32 = 1;
const RUN_ITALIC: u32 = 2;
const RUN_UNDERLINE: u32 = 4;
const RUN_SUPER: u32 = 8;
const RUN_SUB: u32 = 16;

fn run_bits(r: &Run) -> u32 {
    u32::from(r.bold) * RUN_BOLD
        + u32::from(r.italic) * RUN_ITALIC
        + u32::from(r.underline) * RUN_UNDERLINE
        + match r.script {
            Some(Script::Super) => RUN_SUPER,
            Some(Script::Sub) => RUN_SUB,
            None => 0,
        }
}

/// A table column's alignment and its lines by their place in a pack (docs/adr/0184).
pub const TABLE_ALIGNS: [&str; 3] = ["left", "center", "right"];
pub const TABLE_GRIDS: [&str; 3] = ["outer", "rows", "none"];

/// A typeface's place in the core's tables; −1 for the project's (docs/adr/0183).
fn font_place(f: Option<Font>) -> f64 {
    f.map_or(-1.0, |f| f64::from(f.index()))
}

fn flag(b: bool) -> f64 {
    if b { 1.0 } else { 0.0 }
}

/// A count, or −1 for a field left out.
fn count<T>(v: Option<&[T]>) -> f64 {
    v.map_or(-1.0, |v| v.len() as f64)
}

impl Packer {
    /// Numbers appended as given: one call per record part keeps the
    /// writer small (every inlined push was a capacity check).
    #[inline(never)]
    fn put(&mut self, xs: &[f64]) {
        self.nums.extend_from_slice(xs);
    }

    fn string(&mut self, s: &str) -> f64 {
        if let Some(&i) = self.index.get(s) {
            return f64::from(i);
        }
        let i = self.strings.len() as u32;
        self.strings.push(s.to_string());
        self.index.insert(s.to_string(), i);
        f64::from(i)
    }

    fn maybe_string(&mut self, s: Option<&str>) -> f64 {
        s.map_or(-1.0, |s| self.string(s))
    }

    #[inline(never)]
    fn points(&mut self, ps: &[Vec2]) {
        self.nums.reserve(1 + 2 * ps.len());
        self.nums.push(ps.len() as f64);
        for p in ps {
            self.nums.extend([p.x, p.y]);
        }
    }

    fn values(&mut self, vs: Option<&[f64]>) {
        self.put(&[count(vs)]);
        if let Some(v) = vs {
            self.put(v);
        }
    }

    /// A path: its points, bulges and holes.
    fn path(&mut self, pts: &[Vec2], bulges: Option<&[f64]>, holes: Option<&[Ring]>) {
        self.points(pts);
        self.values(bulges);
        self.put(&[count(holes)]);
        for h in holes.into_iter().flatten() {
            self.points(&h.pts);
            self.values(h.bulges.as_deref());
        }
    }

    /// One object: `id, layer, label, kind`, then the kind's fields.
    pub fn object(&mut self, id: f64, layer: &str, label: bool, shape: &Shape) {
        let layer = self.string(layer);
        self.put(&[id, layer, flag(label)]);
        const NONE: Vec2 = Vec2::new(f64::NAN, f64::NAN);
        match shape {
            // A multi-point object (docs/adr/0174): kind 17, its other points after the first.
            Shape::Point {
                p,
                z,
                parts: Some(parts),
            } if !parts.is_empty() => {
                self.put(&[17.0, p.x, p.y, flag(z.is_some()), z.unwrap_or(f64::NAN)]);
                self.put(&[parts.len() as f64]);
                for q in parts {
                    self.put(&[q.p.x, q.p.y, flag(q.z.is_some()), q.z.unwrap_or(f64::NAN)]);
                }
            }
            Shape::Point { p, z, .. } => {
                self.put(&[0.0, p.x, p.y, flag(z.is_some()), z.unwrap_or(f64::NAN)]);
            }
            Shape::Line { a, b } => self.put(&[1.0, a.x, a.y, b.x, b.y]),
            // An empty part list is a one-part area, laid out as kind 3 (the web's `packEntities`).
            Shape::Polygon {
                pts,
                bulges,
                holes,
                parts: Some(parts),
            } if !parts.is_empty() => {
                self.put(&[13.0]);
                self.path(pts, bulges.as_deref(), holes.as_deref());
                self.put(&[parts.len() as f64]);
                for p in parts {
                    self.path(&p.pts, p.bulges.as_deref(), p.holes.as_deref());
                }
            }
            // A multi-part polyline (docs/adr/0174): kind 16, as an area's kind 13.
            Shape::Polyline {
                pts,
                bulges,
                holes,
                parts: Some(parts),
            } if !parts.is_empty() => {
                self.put(&[16.0]);
                self.path(pts, bulges.as_deref(), holes.as_deref());
                self.put(&[parts.len() as f64]);
                for p in parts {
                    self.path(&p.pts, p.bulges.as_deref(), p.holes.as_deref());
                }
            }
            Shape::Polyline {
                pts, bulges, holes, ..
            }
            | Shape::Polygon {
                pts, bulges, holes, ..
            } => {
                let polygon = matches!(shape, Shape::Polygon { .. });
                self.put(&[if polygon { 3.0 } else { 2.0 }]);
                self.path(pts, bulges.as_deref(), holes.as_deref());
            }
            Shape::Circle { c, r } => self.put(&[4.0, c.x, c.y, *r]),
            Shape::Arc { c, r, a0, a1 } => self.put(&[5.0, c.x, c.y, *r, *a0, *a1]),
            Shape::Ellipse {
                c,
                major,
                ratio,
                t0,
                t1,
            } => self.put(&[6.0, c.x, c.y, major.x, major.y, *ratio, *t0, *t1]),
            Shape::Xline { p, dir } | Shape::Ray { p, dir } => {
                let ray = matches!(shape, Shape::Ray { .. });
                self.put(&[if ray { 8.0 } else { 7.0 }, p.x, p.y, dir.x, dir.y]);
            }
            Shape::Spline { pts, closed } => {
                self.put(&[9.0]);
                self.points(pts);
                self.put(&[flag(*closed)]);
            }
            Shape::Text {
                p,
                text,
                height,
                rotation,
                align,
                width_factor,
                mask,
                box_width,
                line_spacing,
                runs,
                face,
            } => {
                let t = self.string(text);
                let a = align.map_or(-1.0, |a| {
                    TextAlign::ALL.iter().position(|x| *x == a).unwrap_or(0) as f64
                });
                let runs = runs.as_deref().unwrap_or_default();
                self.put(&[
                    10.0,
                    p.x,
                    p.y,
                    *height,
                    *rotation,
                    t,
                    a,
                    width_factor.unwrap_or(f64::NAN),
                    flag(*mask == Some(true)),
                    box_width.unwrap_or(f64::NAN),
                    line_spacing.unwrap_or(f64::NAN),
                    runs.len() as f64,
                ]);
                for r in runs {
                    let color = r.color.as_deref().map_or(-1.0, |c| self.string(c));
                    self.put(&[
                        f64::from(r.start),
                        f64::from(r.end),
                        f64::from(run_bits(r)),
                        color,
                    ]);
                }
                let style = self.maybe_string(face.style.as_deref());
                self.put(&[
                    style,
                    font_place(face.font),
                    flag(face.bold),
                    flag(face.italic),
                    face.oblique.unwrap_or(f64::NAN),
                ]);
            }
            Shape::Dimension {
                a,
                b,
                offset,
                height,
                text,
                style,
                angle,
                c,
                mask,
                za,
                zb,
                look,
            } => {
                let t = self.maybe_string(text.as_deref());
                let st = self.maybe_string(style.as_deref());
                let ds = self.maybe_string(look.style.as_deref());
                let unit = self.maybe_string(look.unit.as_deref());
                let prefix = self.maybe_string(look.prefix.as_deref());
                let suffix = self.maybe_string(look.suffix.as_deref());
                let at = c.unwrap_or(NONE);
                self.put(&[
                    11.0,
                    a.x,
                    a.y,
                    b.x,
                    b.y,
                    *offset,
                    *height,
                    t,
                    st,
                    flag(angle.is_some()),
                    angle.unwrap_or(f64::NAN),
                    flag(c.is_some()),
                    at.x,
                    at.y,
                    flag(*mask == Some(true)),
                    za.unwrap_or(f64::NAN),
                    zb.unwrap_or(f64::NAN),
                    ds,
                    look.arrow.map_or(-1.0, |a| {
                        Arrow::ALL.iter().position(|x| *x == a).unwrap_or(0) as f64
                    }),
                    look.arrow_size.unwrap_or(f64::NAN),
                    look.ext_offset.unwrap_or(f64::NAN),
                    look.ext_beyond.unwrap_or(f64::NAN),
                    look.text_gap.unwrap_or(f64::NAN),
                    flag(look.centre),
                    look.decimals.map_or(f64::NAN, f64::from),
                    unit,
                    prefix,
                    suffix,
                    font_place(look.font),
                ]);
            }
            Shape::Hatch {
                ring,
                holes,
                pattern,
            } => {
                self.put(&[12.0]);
                self.points(ring);
                self.put(&[count(holes.as_deref())]);
                for h in holes.iter().flatten() {
                    self.points(h);
                }
                let kind = self.string(&pattern.kind);
                self.put(&[kind, pattern.angle, pattern.spacing]);
            }
            // docs/adr/0144: the insertion point, scale, turn, mirror flag, the
            // block's id, and its attributes' texts (§7: what its attribute texts show).
            Shape::Insert {
                block,
                p,
                scale,
                rotation,
                mirror,
                attrs,
            } => {
                let b = self.string(block);
                self.put(&[
                    14.0,
                    p.x,
                    p.y,
                    *scale,
                    *rotation,
                    flag(mirror.unwrap_or(false)),
                    b,
                ]);
                match attrs {
                    None => self.put(&[-1.0]),
                    Some(Attrs(fields)) => {
                        let texts: Vec<(&String, &String)> = fields
                            .iter()
                            .filter_map(|(k, v)| match v {
                                Json::Str(s) => Some((k, s)),
                                _ => None,
                            })
                            .collect();
                        self.put(&[texts.len() as f64]);
                        for (k, v) in texts {
                            let pair = [self.string(k), self.string(v)];
                            self.put(&pair);
                        }
                    }
                }
            }
            // docs/adr/0146: the vertices, height and turn, the note and the arrowhead's name, the mask.
            Shape::Leader {
                pts,
                text,
                height,
                rotation,
                arrow,
                mask,
            } => {
                self.put(&[15.0]);
                self.points(pts);
                let t = self.maybe_string(text.as_deref());
                let a = self.maybe_string(arrow.as_deref());
                self.put(&[*height, *rotation, t, a, flag(*mask == Some(true))]);
            }
            // docs/adr/0184, as the reader's kind 18 says.
            Shape::Table {
                p,
                rotation,
                height,
                rows,
                columns,
                cells,
                merges,
                aligns,
                header,
                grid,
                frame,
                source: _,
                face,
            } => {
                self.put(&[18.0, p.x, p.y, *rotation, *height]);
                self.values(Some(rows));
                self.values(Some(columns));
                for row in cells {
                    for words in row {
                        let w = self.string(words);
                        self.put(&[w]);
                    }
                }
                let merges = merges.as_deref().unwrap_or_default();
                self.put(&[merges.len() as f64]);
                for m in merges {
                    self.put(&[m.row as f64, m.col as f64, m.rows as f64, m.cols as f64]);
                }
                match aligns {
                    None => self.put(&[-1.0]),
                    Some(a) => {
                        self.put(&[a.len() as f64]);
                        for x in a {
                            let i = TABLE_ALIGNS.iter().position(|n| n == x).unwrap_or(0);
                            self.put(&[i as f64]);
                        }
                    }
                }
                let g = grid.as_deref().map_or(-1.0, |g| {
                    TABLE_GRIDS
                        .iter()
                        .position(|n| *n == g)
                        .map_or(-1.0, |i| i as f64)
                });
                let style = self.maybe_string(face.style.as_deref());
                self.put(&[
                    flag(*header == Some(true)),
                    g,
                    frame.unwrap_or(f64::NAN),
                    style,
                    font_place(face.font),
                    flag(face.bold),
                    flag(face.italic),
                    face.oblique.unwrap_or(f64::NAN),
                ]);
            }
        }
    }
}

impl Reader<'_> {
    /// The next object: its id, layer, label flag and geometry.
    fn object(&mut self) -> Result<(f64, String, bool, Shape), String> {
        let id = self.num()?;
        let layer = self.string()?.unwrap_or_default();
        let label = self.flag()?;
        let kind = self.num()?;
        Ok((id, layer, label, self.shape(kind)?))
    }
}

/// Every packed object in order: id, layer, label flag and shape (tests
/// read the store's packed answers back with it too).
pub(crate) fn unpack(
    nums: &[f64],
    strings: &[String],
) -> Result<Vec<(f64, String, bool, Shape)>, String> {
    let mut r = Reader {
        nums,
        at: 0,
        strings,
    };
    let mut out = Vec::new();
    while r.at < nums.len() {
        out.push(r.object()?);
    }
    Ok(out)
}

/// Packed objects moved by each affine in turn, packed again: what
/// `Store::transform_packed` answers, without a store (docs/adr/0037). The
/// web's `cad.entities.transform` handler packs the objects it names and
/// only numbers cross: ids and coordinates, −0 and NaN included, arrive and
/// leave bit for bit, which JSON could not do. Records come affine after
/// affine, each run in the order the objects were packed; nothing is read
/// unless every record is.
pub fn transform_packed_objects(
    nums: &[f64],
    strings: &[String],
    affines: &[Affine],
) -> Result<Packer, String> {
    let objects = unpack(nums, strings)?;
    Ok(moved_packed(&objects, affines))
}

/// Packed objects copied into an array, packed again (the web's
/// `cad.entities.array` handler, docs/adr/0047): `affines` makes the
/// copies' affines from the objects' shapes (`array_transforms`: a polar
/// array that does not turn is placed by their middle), then every object
/// goes by every affine as in [`transform_packed_objects`]. None when
/// `affines` makes none.
pub fn array_packed_objects(
    nums: &[f64],
    strings: &[String],
    affines: impl FnOnce(&[Shape]) -> Option<Vec<Affine>>,
) -> Result<Option<Packer>, String> {
    let objects = unpack(nums, strings)?;
    let shapes: Vec<Shape> = objects.iter().map(|(.., s)| s.clone()).collect();
    Ok(affines(&shapes).map(|ms| moved_packed(&objects, &ms)))
}

/// Every object by every affine, affine after affine, packed.
fn moved_packed(objects: &[(f64, String, bool, Shape)], affines: &[Affine]) -> Packer {
    let mut out = Packer::default();
    for m in affines {
        for (id, layer, label, shape) in objects {
            out.object(*id, layer, *label, &transform_shape(shape, m));
        }
    }
    out
}

impl Store {
    /// Adds or replaces packed objects in order (see the module's table);
    /// `strings` holds the layer ids and texts the numbers point at.
    pub fn put_packed(&mut self, nums: &[f64], strings: &[String]) -> Result<usize, String> {
        let mut r = Reader {
            nums,
            at: 0,
            strings,
        };
        let mut n = 0;
        while r.at < nums.len() {
            let (id, layer, label, shape) = r.object()?;
            self.put(id, &layer, label, shape);
            n += 1;
        }
        self.maybe_rebuild();
        Ok(n)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_every_kind_like_the_json() {
        let strings: Vec<String> = ["a", "Ada 104", "linear", "lines"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let nums = [
            // A polygon with bulges and a hole on layer "a", labelled.
            1.0,
            0.0,
            1.0,
            3.0,
            4.0,
            0.0,
            0.0,
            10.0,
            0.0,
            10.0,
            10.0,
            0.0,
            10.0,
            4.0,
            0.5,
            0.0,
            0.0,
            0.0,
            1.0,
            3.0,
            4.0,
            4.0,
            6.0,
            4.0,
            5.0,
            6.0,
            -1.0,
            // A text: no alignment, width factor or mask (docs/adr/0145); no box, spacing or runs (docs/adr/0182);
            // no face (docs/adr/0183).
            2.0,
            0.0,
            0.0,
            10.0,
            1.0,
            2.0,
            3.0,
            45.0,
            1.0,
            -1.0,
            f64::NAN,
            0.0,
            f64::NAN,
            f64::NAN,
            0.0,
            -1.0,
            -1.0,
            0.0,
            0.0,
            f64::NAN,
            // A linear dimension with an angle and no centre, mask or elevations (docs/adr/0147); no look
            // (docs/adr/0183).
            3.0,
            0.0,
            0.0,
            11.0,
            0.0,
            0.0,
            10.0,
            0.0,
            2.0,
            0.5,
            -1.0,
            2.0,
            1.0,
            90.0,
            0.0,
            f64::NAN,
            f64::NAN,
            0.0,
            f64::NAN,
            f64::NAN,
            -1.0,
            -1.0,
            f64::NAN,
            f64::NAN,
            f64::NAN,
            f64::NAN,
            0.0,
            f64::NAN,
            -1.0,
            -1.0,
            -1.0,
            -1.0,
            // A hatch with one hole.
            4.0,
            0.0,
            0.0,
            12.0,
            3.0,
            0.0,
            0.0,
            4.0,
            0.0,
            4.0,
            4.0,
            1.0,
            3.0,
            1.0,
            1.0,
            2.0,
            1.0,
            2.0,
            2.0,
            3.0,
            45.0,
            1.0,
        ];
        let mut s = Store::new();
        assert_eq!(s.put_packed(&nums, &strings), Ok(4));
        let mut j = Store::new();
        j.put_json(
            r#"[{"id":1,"layerId":"a","label":"7","kind":"polygon","pts":[{"x":0,"y":0},{"x":10,"y":0},{"x":10,"y":10},{"x":0,"y":10}],"bulges":[0.5,0,0,0],"holes":[{"pts":[{"x":4,"y":4},{"x":6,"y":4},{"x":5,"y":6}]}]},
                {"id":2,"layerId":"a","kind":"text","p":{"x":1,"y":2},"height":3,"rotation":45,"text":"Ada 104"},
                {"id":3,"layerId":"a","kind":"dimension","a":{"x":0,"y":0},"b":{"x":10,"y":0},"offset":2,"height":0.5,"style":"linear","angle":90},
                {"id":4,"layerId":"a","kind":"hatch","ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}]],"pattern":{"type":"lines","angle":45,"spacing":1}}]"#,
        )
        .unwrap();
        for id in [1.0, 2.0, 3.0, 4.0] {
            let (a, b) = (s.get(id).unwrap(), j.get(id).unwrap());
            assert_eq!(a.shape, b.shape, "{id}");
            assert_eq!(a.bounds, b.bounds);
            assert_eq!((a.label, a.layer), (b.label, b.layer));
        }
        // Written back from the store's copies, the same numbers and strings
        // (the left-out angle and centre as NaN, as pack.ts writes them).
        let names = s.layer_names();
        let mut w = Packer::default();
        for id in [1.0, 2.0, 3.0, 4.0] {
            let it = s.get(id).unwrap();
            w.object(it.id, names[it.layer as usize], it.label, &it.shape);
        }
        assert_eq!(w.strings, strings);
        assert_eq!(bits(&w.nums), bits(&nums));
    }

    fn bits(nums: &[f64]) -> Vec<u64> {
        nums.iter().map(|x| x.to_bits()).collect()
    }

    /// Geometry of every kind, optional fields given and left out, in TM
    /// coordinates, with −0, NaN and ±∞ (as the core's JSON writes them).
    const SHAPES: &[&str] = &[
        r#"{"kind":"point","p":{"x":486512.34,"y":4420187.52},"z":850.5}"#,
        r##"{"kind":"point","p":{"x":-0,"y":"#NaN"}}"##,
        r##"{"kind":"line","a":{"x":486500.1,"y":4420100.2},"b":{"x":"#Inf","y":"#-Inf"}}"##,
        r#"{"kind":"polyline","pts":[{"x":486500.1,"y":4420100.2},{"x":486540.35,"y":4420101.9},{"x":486538.8,"y":4420141.15}],"bulges":[0.5,-0,0],"holes":[{"pts":[{"x":1,"y":1}]}]}"#,
        r#"{"kind":"polyline","pts":[]}"#,
        r#"{"kind":"polygon","pts":[{"x":486512.34,"y":4420187.52},{"x":486535.757,"y":4420188.723},{"x":486538.221,"y":4420218.986}],"bulges":[0,0.25,0],"holes":[{"pts":[{"x":486520,"y":4420195},{"x":486525,"y":4420195},{"x":486522,"y":4420200}],"bulges":[0,0,-0.1]},{"pts":[{"x":486530,"y":4420200},{"x":486531,"y":4420200},{"x":486531,"y":4420201}]}]}"#,
        r#"{"kind":"polygon","pts":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"bulges":[],"holes":[]}"#,
        r#"{"kind":"circle","c":{"x":486520,"y":4420200},"r":12.5}"#,
        r#"{"kind":"arc","c":{"x":486520,"y":4420200},"r":12.5,"a0":0.3,"a1":6.1}"#,
        r#"{"kind":"ellipse","c":{"x":486520,"y":4420200},"major":{"x":10,"y":-3},"ratio":0.4,"t0":1,"t1":1}"#,
        r#"{"kind":"xline","p":{"x":486520,"y":4420200},"dir":{"x":0.6,"y":0.8}}"#,
        r#"{"kind":"ray","p":{"x":486520,"y":4420200},"dir":{"x":-1,"y":0}}"#,
        r#"{"kind":"spline","pts":[{"x":486520,"y":4420200},{"x":486530,"y":4420210},{"x":486540,"y":4420205}],"closed":true}"#,
        r#"{"kind":"spline","pts":[],"closed":false}"#,
        r#"{"kind":"text","p":{"x":486520,"y":4420200},"text":"Ada 104 😀","height":2,"rotation":-30}"#,
        r#"{"kind":"text","p":{"x":1,"y":2},"text":"","height":0.5,"rotation":0}"#,
        r#"{"kind":"text","p":{"x":486520,"y":4420200},"text":"Ada 104","height":2,"rotation":30,"align":"middleCenter","widthFactor":0.8,"mask":true}"#,
        r#"{"kind":"text","p":{"x":0,"y":0},"text":"B","height":1,"rotation":0,"align":"topRight"}"#,
        // A face of its own (docs/adr/0183 §2): a typeface, bold, italic and a slant.
        r#"{"kind":"text","p":{"x":2,"y":3},"text":"Ada 105","height":2,"rotation":0,"font":"arimo","bold":true,"italic":true,"oblique":15}"#,
        r#"{"kind":"text","p":{"x":2,"y":3},"text":"Ada 106","height":2,"rotation":0,"textStyle":"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101","font":"barlow"}"#,
        r#"{"kind":"dimension","a":{"x":486520,"y":4420200},"b":{"x":486530,"y":4420200},"offset":2,"height":0.5}"#,
        r#"{"kind":"dimension","a":{"x":0,"y":0},"b":{"x":3,"y":4},"offset":-2,"height":0.5,"text":"12,5 m","style":"linear","angle":90}"#,
        r#"{"kind":"dimension","a":{"x":10,"y":0},"b":{"x":0,"y":10},"offset":5,"height":1,"text":"","style":"angular","c":{"x":0,"y":0}}"#,
        r#"{"kind":"dimension","a":{"x":486520,"y":4420200},"b":{"x":486540,"y":4420210},"offset":1.5,"height":0.5,"style":"slope","mask":true,"za":105.25,"zb":-0}"#,
        r#"{"kind":"dimension","a":{"x":0,"y":10},"b":{"x":10,"y":0},"offset":-3,"height":1,"style":"arcLength","c":{"x":0,"y":0}}"#,
        // A look of its own (docs/adr/0183 §3): filled arrowheads, sizes, the value on the line, a typeface.
        r#"{"kind":"dimension","a":{"x":0,"y":0},"b":{"x":8,"y":0},"offset":2,"height":0.5,"arrow":"closed","arrowSize":1.2,"extOffset":0.2,"extBeyond":0.8,"textGap":0.5,"textPlace":"centre","font":"overpass"}"#,
        // The style it follows and how its value is written (docs/adr/0183 §3) cross too.
        r#"{"kind":"dimension","a":{"x":0,"y":0},"b":{"x":8,"y":0},"offset":2,"height":0.5,"dimStyle":"0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0201","decimals":0,"unit":"cm","prefix":"L=","suffix":" cm"}"#,
        r#"{"kind":"hatch","ring":[{"x":0,"y":0},{"x":4,"y":0},{"x":4,"y":4}],"holes":[[{"x":1,"y":1},{"x":2,"y":1},{"x":2,"y":2}],[]],"pattern":{"type":"cross","angle":30,"spacing":0.5}}"#,
        r#"{"kind":"hatch","ring":[{"x":486520,"y":4420200},{"x":486530,"y":4420200},{"x":486525,"y":4420210}],"pattern":{"type":"solid","angle":0,"spacing":1}}"#,
        r#"{"kind":"leader","pts":[{"x":486520,"y":4420200},{"x":486528,"y":4420206}],"text":"Ø150 PVC","height":2,"rotation":30,"arrow":"open","mask":true}"#,
        r#"{"kind":"leader","pts":[{"x":-0,"y":0},{"x":4,"y":3},{"x":9,"y":3}],"height":0.5,"rotation":0}"#,
    ];

    #[test]
    fn writes_and_reads_every_kind_bit_for_bit() {
        use crate::api::json::{self, FromJson, Json};
        let shapes: Vec<Shape> = SHAPES
            .iter()
            .map(|t| Shape::from_json(&Json::parse(t).unwrap()).unwrap())
            .collect();
        let layers = ["parsel", "", "Bina çatısı"];
        let mut w = Packer::default();
        for (i, s) in shapes.iter().enumerate() {
            w.object(i as f64 + 1.0, layers[i % 3], i % 2 == 0, s);
        }
        let back = unpack(&w.nums, &w.strings).unwrap();
        assert_eq!(back.len(), shapes.len());
        for (i, (id, layer, label, shape)) in back.iter().enumerate() {
            assert_eq!(
                (*id, layer.as_str(), *label),
                (i as f64 + 1.0, layers[i % 3], i % 2 == 0)
            );
            // The core's JSON tells −0 from 0 and keeps NaN and ±∞.
            assert_eq!(json::to_string(shape), json::to_string(&shapes[i]), "{i}");
        }
        // Written again, the same numbers; each string once.
        let mut again = Packer::default();
        for (id, layer, label, shape) in &back {
            again.object(*id, layer, *label, shape);
        }
        assert_eq!(bits(&again.nums), bits(&w.nums));
        assert_eq!(again.strings, w.strings);
        assert_eq!(
            w.strings.iter().filter(|s| s.as_str() == "parsel").count(),
            1
        );
        // The store reads it as it reads the page's packs.
        let mut s = Store::new();
        assert_eq!(s.put_packed(&w.nums, &w.strings), Ok(shapes.len()));
        for (i, want) in shapes.iter().enumerate() {
            let it = s.get(i as f64 + 1.0).unwrap();
            assert_eq!(json::to_string(&it.shape), json::to_string(want));
        }
    }

    /// A block's insert (docs/adr/0144) and its attributes' texts (§7): read
    /// back as they went, a value that is not text left out.
    #[test]
    fn an_insert_carries_its_attributes_texts() {
        use crate::api::json::Json;
        use crate::entity::Attrs;
        let text = |s: &str| Json::Str(s.to_owned());
        let with = Shape::Insert {
            block: "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001".into(),
            p: Vec2::new(486520.25, 4420200.5),
            scale: 2.0,
            rotation: -0.0,
            mirror: Some(true),
            attrs: Some(Attrs(vec![
                ("NO".into(), text("R-12")),
                ("KOT".into(), text("")),
                ("Sayı".into(), Json::Num(3.0)),
            ])),
        };
        let without = Shape::Insert {
            block: "b".into(),
            p: Vec2::new(1.0, 2.0),
            scale: 1.0,
            rotation: 0.5,
            mirror: None,
            attrs: None,
        };
        let mut w = Packer::default();
        w.object(1.0, "altyapi", false, &with);
        w.object(2.0, "altyapi", true, &without);
        let back = unpack(&w.nums, &w.strings).unwrap();
        let Shape::Insert {
            attrs,
            rotation,
            mirror,
            ..
        } = &back[0].3
        else {
            panic!("{:?}", back[0].3);
        };
        assert_eq!(
            attrs,
            &Some(Attrs(vec![
                ("NO".into(), text("R-12")),
                ("KOT".into(), text(""))
            ]))
        );
        assert_eq!(
            (rotation.to_bits(), *mirror),
            ((-0.0f64).to_bits(), Some(true))
        );
        assert_eq!(back[1].3, without);
    }

    /// Without a store, the same records as `Store::transform_packed`:
    /// every kind by every affine, affine after affine, id, layer and label
    /// kept, the geometry `transform_shape`'s, −0 and NaN bit for bit.
    #[test]
    fn transforms_packed_objects_as_the_store_does() {
        use crate::api::json::{self, FromJson, Json};
        use crate::geom::affine::{mirror, rotation, scaling, translation};
        let shapes: Vec<Shape> = SHAPES
            .iter()
            .map(|t| Shape::from_json(&Json::parse(t).unwrap()).unwrap())
            .collect();
        let layers = ["parsel", "", "Bina çatısı"];
        let mut w = Packer::default();
        for (i, s) in shapes.iter().enumerate() {
            w.object(i as f64 + 1.0, layers[i % 3], i % 2 == 0, s);
        }
        let affines = [
            translation(12.5, -0.0),
            rotation(0.7, Vec2::new(486520.0, 4420200.0)),
            scaling(2.5, Vec2::new(-0.0, 3.0)),
            mirror(
                Vec2::new(486500.0, 4420100.0),
                Vec2::new(486540.0, 4420160.0),
            ),
        ];
        let out = transform_packed_objects(&w.nums, &w.strings, &affines).unwrap();
        let back = unpack(&out.nums, &out.strings).unwrap();
        assert_eq!(back.len(), shapes.len() * affines.len());
        let mut store = Store::new();
        store.put_packed(&w.nums, &w.strings).unwrap();
        let ids: Vec<f64> = (1..=shapes.len()).map(|i| i as f64).collect();
        let stored = store.transform_packed(&ids, &affines);
        assert_eq!(bits(&out.nums), bits(&stored.nums));
        assert_eq!(out.strings, stored.strings);
        for (k, m) in affines.iter().enumerate() {
            for (i, shape) in shapes.iter().enumerate() {
                let (id, layer, label, got) = &back[k * shapes.len() + i];
                assert_eq!(
                    (*id, layer.as_str(), *label),
                    (i as f64 + 1.0, layers[i % 3], i % 2 == 0)
                );
                // The core's JSON tells −0 from 0 and keeps NaN and ±∞.
                assert_eq!(
                    json::to_string(got),
                    json::to_string(&transform_shape(shape, m)),
                    "{k}, {i}"
                );
            }
        }
        // Nothing to move, nothing back; a broken pack is an error.
        assert!(
            transform_packed_objects(&[], &w.strings, &affines)
                .unwrap()
                .nums
                .is_empty()
        );
        assert!(transform_packed_objects(&w.nums[..5], &w.strings, &affines).is_err());
    }

    #[test]
    fn a_short_or_bad_pack_is_an_error_not_a_panic() {
        let strings = vec!["a".to_string()];
        let mut s = Store::new();
        assert!(s.put_packed(&[1.0, 0.0, 0.0, 1.0, 0.0], &strings).is_err());
        assert!(
            s.put_packed(&[1.0, 5.0, 0.0, 4.0, 0.0, 0.0, 1.0], &strings)
                .is_err()
        );
        assert!(s.put_packed(&[1.0, 0.0, 0.0, 99.0], &strings).is_err());
        assert!(
            s.put_packed(&[1.0, 0.0, 0.0, 2.0, 1e300], &strings)
                .is_err()
        );
        assert!(s.put_packed(&[1.0, 0.0, 0.0, 2.0, 0.5], &strings).is_err());
        assert!(
            s.put_packed(&[1.0, 0.0, 0.0, f64::NAN, 1.0, 2.0], &strings)
                .is_err()
        );
        assert_eq!(s.put_packed(&[], &strings), Ok(0));
    }
}
