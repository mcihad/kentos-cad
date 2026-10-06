//! Drawing objects and their geometry (`apps/web/src/model/entities.ts`): the 13
//! kinds, and the helpers every tool and view uses (vertices, outlines,
//! bounds, anchors, lengths, areas). An entity keeps the fields the core
//! does not interpret (id, layer, colour, attributes, label, symbol) as they
//! came, so operations that return `{ ...e, … }` in TypeScript give them
//! back unchanged (docs/adr/0008).

use std::borrow::Cow;

use crate::api::json::{FromJson, Json, ToJson, write_str};
use crate::api::{Op, json};
use crate::geom::arc::{
    ArcGeom, DEFAULT_STEP, arc_end, arc_length, arc_mid, arc_start, tessellate_arc,
};
use crate::geom::arrangement::Ring;
use crate::geom::bulge::{bulge_path_length, bulge_path_outline, bulge_ring_area, has_bulges};
use crate::geom::dimension::{DimensionGeom, DimensionLayout, is_new_kind, layout_dimension};
use crate::geom::leader;
use crate::geom::ellipse::{
    EllipseGeom, ellipse_area, ellipse_length, ellipse_point, is_full_ellipse, quadrant_params,
    tessellate_ellipse,
};
use crate::geom::spline::{catmull_rom, spline_length};
use crate::geometry::{
    Bounds, centroid, empty_bounds, extend_bounds, path_length, point_in_polygon, signed_area,
};
use crate::jsmath::{PI, TAU, cos, js_max, sin};
use crate::op;
use crate::text::{Font, TextAlign};
use crate::vec2::Vec2;

/// Half-length (1000 km) an infinite line gets when it meets finite geometry
/// on the CPU (`CONSTRUCTION_REACH`).
pub const CONSTRUCTION_REACH: f64 = 1e6;

/// An insert's attributes as the object holds them (`attrs`, docs/adr/0144
/// §7): its block's attribute texts show the value under their tag.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Attrs(pub Vec<(String, Json)>);

impl Attrs {
    /// The value under `tag` an attribute text shows: a text that is not empty.
    pub fn shown(&self, tag: &str) -> Option<&str> {
        self.0
            .iter()
            .find(|(k, _)| k == tag)
            .and_then(|(_, v)| match v {
                Json::Str(s) if !s.is_empty() => Some(s.as_str()),
                _ => None,
            })
    }
}

/// A hatch's pattern (docs/adr/0186 §1): `solid`, `lines`, `cross`,
/// `pattern` (its line families, named and scaled) or `gradient`.
#[derive(Clone, Debug, PartialEq)]
pub struct HatchPattern {
    pub kind: String,
    /// Degrees: the lines', the pattern's turn or the gradient's direction.
    pub angle: f64,
    /// Metres between user-defined lines.
    pub spacing: f64,
    pub name: Option<String>,
    /// A pattern's metres per unit of its definition.
    pub scale: Option<f64>,
    pub lines: Option<Vec<crate::geom::hatch_pattern::PatternLine>>,
    pub gradient: Option<crate::geom::hatch_pattern::Gradient>,
}

crate::json_struct!(HatchPattern { kind => "type", angle, spacing, name, scale, lines, gradient });

impl HatchPattern {
    /// A user-defined pattern: solid, lines or crossed lines.
    pub fn user(kind: &str, angle: f64, spacing: f64) -> Self {
        Self {
            kind: kind.to_owned(),
            angle,
            spacing,
            name: None,
            scale: None,
            lines: None,
            gradient: None,
        }
    }
}

/// The objects a hatch's region follows (docs/adr/0186 §6), by their
/// persistent ids: the closed object, its islands, the texts and inserts
/// left open, and the point clicked inside.
#[derive(Clone, Debug, PartialEq)]
pub struct HatchAssoc {
    pub outer: String,
    pub islands: Option<Vec<String>>,
    pub cutouts: Option<Vec<String>>,
    pub seed: Vec2,
}

crate::json_struct!(HatchAssoc {
    outer,
    islands,
    cutouts,
    seed
});

/// A part of a multi-part area past its first (docs/adr/0143): its ring in
/// vertex + bulge form and its holes, as the area's own fields hold the
/// first part's. Its vertices' elevations stay with the host, as a hole's do.
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
    pub holes: Option<Vec<Ring>>,
}

crate::json_struct!(Part { pts, bulges, holes });

/// A point of a multi-point object past its first (docs/adr/0174): its
/// place and elevation, as the point's own fields hold the first's.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PointPart {
    pub p: Vec2,
    pub z: Option<f64>,
}

crate::json_struct!(PointPart { p, z });

/// A table's merged range: `rows` × `cols` cells from row `row`, column `col` (docs/adr/0184 §1).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CellRange {
    pub row: usize,
    pub col: usize,
    pub rows: usize,
    pub cols: usize,
}

crate::json_struct!(CellRange {
    row,
    col,
    rows,
    cols
});

impl CellRange {
    /// Whether the cell at `row`, `col` is in it.
    pub fn holds(&self, row: usize, col: usize) -> bool {
        row >= self.row
            && row < self.row + self.rows
            && col >= self.col
            && col < self.col + self.cols
    }
}

impl Part {
    /// The part as a one-part area.
    pub fn shape(&self) -> Shape {
        Shape::Polygon {
            pts: self.pts.clone(),
            bulges: self.bulges.clone(),
            holes: self.holes.clone(),
            parts: None,
        }
    }
}

/// The geometry of an entity, tagged by `kind` as in TypeScript.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Point {
        p: Vec2,
        z: Option<f64>,
        /// A multi-point object's points past its first, whose own are the
        /// fields above (docs/adr/0174); `point_parts` takes it point by point.
        parts: Option<Vec<PointPart>>,
    },
    Line {
        a: Vec2,
        b: Vec2,
    },
    Polyline {
        pts: Vec<Vec2>,
        bulges: Option<Vec<f64>>,
        holes: Option<Vec<Ring>>,
        /// A multi-part polyline's parts past its first, whose own are the
        /// fields above, open and without holes (docs/adr/0174); `path_parts`
        /// takes it part by part.
        parts: Option<Vec<Part>>,
    },
    Polygon {
        pts: Vec<Vec2>,
        bulges: Option<Vec<f64>>,
        holes: Option<Vec<Ring>>,
        /// A multi-part area's parts past its first, whose own are the
        /// fields above (docs/adr/0143); `area_parts` takes it part by part.
        parts: Option<Vec<Part>>,
    },
    Circle {
        c: Vec2,
        r: f64,
    },
    Arc {
        c: Vec2,
        r: f64,
        a0: f64,
        a1: f64,
    },
    Ellipse {
        c: Vec2,
        major: Vec2,
        ratio: f64,
        t0: f64,
        t1: f64,
    },
    Xline {
        p: Vec2,
        dir: Vec2,
    },
    Ray {
        p: Vec2,
        dir: Vec2,
    },
    Spline {
        pts: Vec<Vec2>,
        closed: bool,
    },
    Text {
        p: Vec2,
        text: String,
        height: f64,
        rotation: f64,
        /// Which point of the text `p` is; none: the left of its baseline (docs/adr/0145).
        align: Option<TextAlign>,
        /// The letters' width times this; none: 1.
        width_factor: Option<f64>,
        /// `Some(true)`: its box is filled with the drawing area's colour first; never `Some(false)`.
        mask: Option<bool>,
        /// A multi-line text's box width, metres, line spacing and letter
        /// formats (docs/adr/0182, `text::paragraph`); none: as a text always was.
        box_width: Option<f64>,
        line_spacing: Option<f64>,
        runs: Option<Vec<crate::text::paragraph::Run>>,
        /// Its style, typeface, bold, italic and slant (docs/adr/0183 §2).
        face: crate::text::face::Face,
    },
    Dimension {
        a: Vec2,
        b: Vec2,
        offset: f64,
        height: f64,
        text: Option<String>,
        style: Option<String>,
        angle: Option<f64>,
        c: Option<Vec2>,
        /// The value over the drawing's background; a slope's elevations (docs/adr/0147).
        mask: Option<bool>,
        za: Option<f64>,
        zb: Option<f64>,
        /// Its style, arrowheads, sizes, value's place and writing (docs/adr/0183 §3).
        look: crate::geom::dimension::Look,
    },
    Hatch {
        ring: Vec<Vec2>,
        holes: Option<Vec<Vec<Vec2>>>,
        pattern: HatchPattern,
        /// The objects its region follows (docs/adr/0186 §6).
        assoc: Option<HatchAssoc>,
    },
    /// A block placed in the drawing (docs/adr/0144): the definition `block`
    /// (its id) moved from its base point to `p`, mirrored in its x axis when
    /// `mirror`, scaled by `scale` and turned by `rotation` (radians) about
    /// `p`. Its own geometry is the insertion point until the store expands
    /// the definition; the transforms compose its similarity exactly.
    Insert {
        block: String,
        p: Vec2,
        scale: f64,
        rotation: f64,
        /// `Some(true)` when mirrored; never `Some(false)` (a file writes no false).
        mirror: Option<bool>,
        /// Its attributes: what its block's attribute texts show (§7). Not
        /// the geometry's JSON: an entity's `attrs` stay among its other
        /// fields, where they were, and are read into it as well.
        attrs: Option<Attrs>,
    },
    /// A leader (docs/adr/0146): an arrowhead at `pts[0]`, a line through
    /// `pts` and, with a note, a landing from the last vertex along the
    /// note's direction (`rotation`, degrees) and the note past it. The
    /// arrowhead and the landing are measured by the note's `height`.
    Leader {
        pts: Vec<Vec2>,
        /// The note; none: the arrow alone. Never empty.
        text: Option<String>,
        height: f64,
        rotation: f64,
        /// The arrowhead's name (`open`, `dot`, `none`); none: a filled arrow.
        arrow: Option<String>,
        /// `Some(true)`: the note's box is filled with the drawing area's colour first.
        mask: Option<bool>,
    },
    /// A table (docs/adr/0184): rows and columns of one-line cells hanging
    /// from `p`, its top left corner, turned `rotation` degrees; its rows'
    /// heights and columns' widths, its cells' words row by row, its merged
    /// ranges, its columns' alignments (`left`, `center`, `right`), its
    /// heading row, which lines it draws (`outer`, `rows`, `none`; none: all)
    /// and its cells' face; its source (docs/adr/0184 §5) as it is written,
    /// which the core carries through and does not read.
    Table {
        p: Vec2,
        rotation: f64,
        height: f64,
        rows: Vec<f64>,
        columns: Vec<f64>,
        cells: Vec<Vec<String>>,
        merges: Option<Vec<CellRange>>,
        aligns: Option<Vec<String>>,
        header: Option<bool>,
        grid: Option<String>,
        /// Its frame's width, metres: the outline drawn as a band (docs/adr/0184 §2).
        frame: Option<f64>,
        source: Option<crate::api::json::Json>,
        face: crate::text::face::Face,
    },
}

crate::json_tagged!(Shape, "kind",
    Point => "point" { p, z, parts },
    Line => "line" { a, b },
    Polyline => "polyline" { pts, bulges, holes, parts },
    Polygon => "polygon" { pts, bulges, holes, parts },
    Circle => "circle" { c, r },
    Arc => "arc" { c, r, a0, a1 },
    Ellipse => "ellipse" { c, major, ratio, t0, t1 },
    Xline => "xline" { p, dir },
    Ray => "ray" { p, dir },
    Spline => "spline" { pts, closed },
    Text => "text" { p, text, height, rotation, align, width_factor => "widthFactor", mask, box_width => "boxWidth", line_spacing => "lineSpacing", runs & face: crate::text::face::Face },
    Dimension => "dimension" { a, b, offset, height, text, style, angle, c, mask, za, zb & look: crate::geom::dimension::Look },
    Hatch => "hatch" { ring, holes, pattern, assoc },
    Insert => "insert" { block, p, scale, rotation, mirror; attrs },
    Leader => "leader" { pts, text, height, rotation, arrow, mask },
    Table => "table" { p, rotation, height, rows, columns, cells, merges, aligns, header, grid, frame, source & face: crate::text::face::Face },
);

/// An entity: its geometry and every other field, untouched and in order.
#[derive(Clone, Debug, PartialEq)]
pub struct Entity {
    pub shape: Shape,
    pub rest: Vec<(String, Json)>,
}

impl Entity {
    pub fn new(shape: Shape) -> Entity {
        Entity {
            shape,
            rest: Vec::new(),
        }
    }

    /// The same entity with another geometry (`{ ...e, …geometry }`).
    pub fn with(&self, shape: Shape) -> Entity {
        Entity {
            shape,
            rest: self.rest.clone(),
        }
    }
}

impl FromJson for Entity {
    fn from_json(v: &Json) -> Result<Entity, String> {
        let Json::Obj(fields) = v else {
            return Err("nesne bekleniyordu".into());
        };
        let mut shape = Shape::from_json(v)?;
        if let Shape::Insert { attrs, .. } = &mut shape
            && let Json::Obj(fields) = v.get("attrs")
        {
            *attrs = Some(Attrs(fields.clone()));
        }
        let kind = match v.get("kind") {
            Json::Str(k) => k.as_str(),
            _ => "",
        };
        let rest = fields
            .iter()
            .filter(|(k, _)| k != "kind" && !Shape::owns_field(kind, k))
            .cloned()
            .collect();
        Ok(Entity { shape, rest })
    }
}

impl ToJson for Entity {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        for (k, v) in &self.rest {
            if !first {
                out.push(',');
            }
            first = false;
            write_str(out, k);
            out.push(':');
            v.write_json(out);
        }
        self.shape.write_fields(out, &mut first);
        out.push('}');
    }
}

/// A dimension shape as the layout reads it.
pub fn dimension_geom(s: &Shape) -> Option<DimensionGeom> {
    match s {
        Shape::Dimension {
            a,
            b,
            offset,
            height,
            style,
            angle,
            c,
            za,
            zb,
            look,
            ..
        } => Some(DimensionGeom {
            a: *a,
            b: *b,
            offset: *offset,
            height: *height,
            style: style.clone(),
            angle: *angle,
            c: *c,
            za: *za,
            zb: *zb,
            look: look.clone(),
        }),
        _ => None,
    }
}

pub fn ellipse_geom(c: Vec2, major: Vec2, ratio: f64, t0: f64, t1: f64) -> EllipseGeom {
    EllipseGeom {
        c,
        major,
        ratio,
        t0,
        t1,
    }
}

fn layout_of(s: &Shape) -> Option<DimensionLayout> {
    dimension_geom(s).and_then(|d| layout_dimension(&d))
}

pub fn tessellate_circle(c: Vec2, r: f64, segments: f64) -> Vec<Vec2> {
    let mut pts = Vec::new();
    let mut i = 0.0;
    while i < segments {
        let t = (i / segments) * PI * 2.0;
        pts.push(Vec2::new(c.x + cos(t) * r, c.y + sin(t) * r));
        i += 1.0;
    }
    pts
}

/// Characteristic vertices: grips, snapping and coordinate tables.
pub fn entity_vertices(e: &Shape) -> Vec<Vec2> {
    match e {
        // Part after part, an area's each its ring's then its holes' (docs/adr/0143, 0174).
        _ if is_multi_part(e) => area_parts(e).iter().flat_map(entity_vertices).collect(),
        Shape::Point { p, .. } | Shape::Text { p, .. } | Shape::Insert { p, .. } => vec![*p],
        Shape::Line { a, b } => vec![*a, *b],
        Shape::Polyline { pts, .. } | Shape::Leader { pts, .. } => pts.clone(),
        Shape::Polygon { pts, holes, .. } => match holes {
            Some(h) if !h.is_empty() => pts
                .iter()
                .copied()
                .chain(h.iter().flat_map(|r| r.pts.iter().copied()))
                .collect(),
            _ => pts.clone(),
        },
        Shape::Circle { c, r } => {
            vec![
                *c,
                Vec2::new(c.x + r, c.y),
                Vec2::new(c.x, c.y + r),
                Vec2::new(c.x - r, c.y),
                Vec2::new(c.x, c.y - r),
            ]
        }
        Shape::Arc { c, r, a0, a1 } => {
            let g = ArcGeom {
                c: *c,
                r: *r,
                a0: *a0,
                a1: *a1,
            };
            vec![arc_start(&g), arc_mid(&g), arc_end(&g)]
        }
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = ellipse_geom(*c, *major, *ratio, *t0, *t1);
            let mut out = vec![*c];
            if !is_full_ellipse(&g) {
                out.push(ellipse_point(&g, *t0));
                out.push(ellipse_point(&g, *t1));
            }
            out.extend(
                quadrant_params(&g)
                    .into_iter()
                    .map(|t| ellipse_point(&g, t)),
            );
            out
        }
        Shape::Xline { p, .. } | Shape::Ray { p, .. } => vec![*p],
        Shape::Spline { pts, .. } => pts.clone(),
        // Its corners, its top left first (docs/adr/0184 §2).
        Shape::Table { .. } => crate::geom::table::table_geom(e)
            .map(|t| t.outline().to_vec())
            .unwrap_or_default(),
        Shape::Dimension { a, b, c, .. } => match c {
            Some(c) => vec![*a, *b, *c],
            None => vec![*a, *b],
        },
        Shape::Hatch { ring, holes, .. } => match holes {
            Some(h) if !h.is_empty() => ring
                .iter()
                .copied()
                .chain(h.iter().flatten().copied())
                .collect(),
            _ => ring.clone(),
        },
    }
}

/// Outline as a point list (curves tessellated): previews, bounds and hit tests.
pub fn entity_outline(e: &Shape, segments: f64) -> Vec<Vec2> {
    match e {
        Shape::Circle { c, r } => tessellate_circle(*c, *r, segments),
        Shape::Arc { c, r, a0, a1 } => tessellate_arc(
            &ArcGeom {
                c: *c,
                r: *r,
                a0: *a0,
                a1: *a1,
            },
            DEFAULT_STEP,
        ),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => tessellate_ellipse(
            &ellipse_geom(*c, *major, *ratio, *t0, *t1),
            js_max(64.0, segments * 2.0),
        ),
        Shape::Xline { p, dir } | Shape::Ray { p, dir } => {
            // Long enough for any preview; the renderer clips to the view.
            let r = CONSTRUCTION_REACH;
            let start = if matches!(e, Shape::Ray { .. }) {
                *p
            } else {
                Vec2::new(p.x - dir.x * r, p.y - dir.y * r)
            };
            vec![start, Vec2::new(p.x + dir.x * r, p.y + dir.y * r)]
        }
        Shape::Spline { pts, closed } => catmull_rom(pts, *closed, 16.0),
        Shape::Polyline { pts, bulges, .. } => {
            bulge_path_outline(pts, bulges.as_deref(), false, TAU / segments)
        }
        Shape::Polygon { pts, bulges, .. } => {
            bulge_path_outline(pts, bulges.as_deref(), true, TAU / segments)
        }
        Shape::Dimension { a, b, style, .. } => match layout_of(e) {
            None => vec![*a, *b],
            Some(l) => match style.as_deref().unwrap_or("aligned") {
                "aligned" | "linear" => vec![*a, l.d1, l.d2, *b],
                // What is drawn: a jogged radius's true centre is not, and it
                // may lie hundreds of metres off (docs/adr/0147 §4).
                "jogged" => l.lines.iter().flatten().copied().collect(),
                _ => {
                    let mut out = vec![*a];
                    out.extend(l.lines.iter().flatten().copied());
                    out.push(*b);
                    out
                }
            },
        },
        // Its line on to its landing's end (docs/adr/0146 §4).
        Shape::Leader { pts, .. } => match leader::layout_of(e) {
            Some(l) => leader::drawn_path(pts, &l),
            None => pts.clone(),
        },
        // Its outline, closed (docs/adr/0184 §2).
        Shape::Table { .. } => {
            let mut ring = entity_vertices(e);
            if let Some(&first) = ring.first() {
                ring.push(first);
            }
            ring
        }
        _ => entity_vertices(e),
    }
}

/// Closed ring of a polygon, arcs tessellated: fills, hit tests and hatching.
pub fn polygon_ring(pts: &[Vec2], bulges: Option<&[f64]>) -> Vec<Vec2> {
    if has_bulges(bulges) {
        bulge_path_outline(pts, bulges, true, DEFAULT_STEP)
    } else {
        pts.to_vec()
    }
}

/// Hole rings of a polygon, of every part (arcs tessellated), or a hatch; empty for anything else.
pub fn polygon_holes(e: &Shape) -> Vec<Vec<Vec2>> {
    if is_multi_part(e) {
        return area_parts(e).iter().flat_map(polygon_holes).collect();
    }
    match e {
        Shape::Polygon { holes: Some(h), .. } => h
            .iter()
            .map(|r| polygon_ring(&r.pts, r.bulges.as_deref()))
            .collect(),
        Shape::Hatch { holes: Some(h), .. } => h.clone(),
        _ => Vec::new(),
    }
}

/// An object's parts, each a one-part object of its kind (the first its own
/// fields): an area's (docs/adr/0143), a polyline's and a multi-point
/// object's (docs/adr/0174); any other shape, a one-part object among them,
/// as itself. What holds for one polygon, polyline or point holds part by
/// part: measure, draw, pick and snap so.
pub fn area_parts(e: &Shape) -> Cow<'_, [Shape]> {
    match e {
        Shape::Polygon {
            pts,
            bulges,
            holes,
            parts: Some(parts),
        } if !parts.is_empty() => {
            let mut out = Vec::with_capacity(parts.len() + 1);
            out.push(Shape::Polygon {
                pts: pts.clone(),
                bulges: bulges.clone(),
                holes: holes.clone(),
                parts: None,
            });
            out.extend(parts.iter().map(Part::shape));
            Cow::Owned(out)
        }
        Shape::Polyline {
            pts,
            bulges,
            parts: Some(parts),
            ..
        } if !parts.is_empty() => {
            let mut out = Vec::with_capacity(parts.len() + 1);
            out.push(Shape::Polyline {
                pts: pts.clone(),
                bulges: bulges.clone(),
                holes: None,
                parts: None,
            });
            out.extend(parts.iter().map(|q| Shape::Polyline {
                pts: q.pts.clone(),
                bulges: q.bulges.clone(),
                holes: None,
                parts: None,
            }));
            Cow::Owned(out)
        }
        Shape::Point {
            p,
            z,
            parts: Some(parts),
        } if !parts.is_empty() => {
            let mut out = Vec::with_capacity(parts.len() + 1);
            out.push(Shape::Point {
                p: *p,
                z: *z,
                parts: None,
            });
            out.extend(parts.iter().map(|q| Shape::Point {
                p: q.p,
                z: q.z,
                parts: None,
            }));
            Cow::Owned(out)
        }
        _ => Cow::Borrowed(std::slice::from_ref(e)),
    }
}

/// What an edit that runs along one ring or path (Kır, Buda, Parçala) says of
/// a multi-part object: it would not know which part, and must not lose the
/// others (docs/adr/0143, 0174).
pub const MULTI_PART_REFUSED: &str =
    "Bu işlem çok parçalı nesnede çalışmaz; önce Parçalara ayır ile parçalarına ayırın.";

/// What an edge edit (Buda, Uzat, Kır, Ötele) says of a leader: its
/// arrowhead, line, landing and note are one object (docs/adr/0146 §4).
/// `verb`: “budanamaz”, “uzatılamaz” …
pub fn leader_refused(verb: &str) -> String {
    format!("Kılavuz {verb}; önce Patlat (X) ile çizgisine, ok başına ve notuna ayırın.")
}

/// Whether an area, a polyline or a point has parts past its first.
pub fn is_multi_part(e: &Shape) -> bool {
    match e {
        Shape::Polygon { parts: Some(p), .. } | Shape::Polyline { parts: Some(p), .. } => {
            !p.is_empty()
        }
        Shape::Point { parts: Some(p), .. } => !p.is_empty(),
        _ => false,
    }
}

/// One object of `shapes` in order, their parts flattened: the first part's
/// fields its own, the others its parts. All areas give an area
/// (docs/adr/0143), all polylines a polyline, all points a multi-point
/// object (docs/adr/0174); none for nothing, mixed kinds or another kind.
pub fn join_parts(shapes: &[Shape]) -> Option<Shape> {
    let all: Vec<Shape> = shapes
        .iter()
        .flat_map(|s| area_parts(s).into_owned())
        .collect();
    let (first, rest) = all.split_first()?;
    match first {
        Shape::Polygon { .. } => {
            let mut parts = Vec::with_capacity(all.len());
            for s in &all {
                let Shape::Polygon {
                    pts, bulges, holes, ..
                } = s
                else {
                    return None;
                };
                parts.push(Part {
                    pts: pts.clone(),
                    bulges: bulges.clone(),
                    holes: holes.clone(),
                });
            }
            let mut it = parts.into_iter();
            let first = it.next()?;
            let rest: Vec<Part> = it.collect();
            Some(Shape::Polygon {
                pts: first.pts,
                bulges: first.bulges,
                holes: first.holes,
                parts: (!rest.is_empty()).then_some(rest),
            })
        }
        Shape::Polyline { pts, bulges, .. } => {
            let mut parts = Vec::with_capacity(rest.len());
            for s in rest {
                let Shape::Polyline { pts, bulges, .. } = s else {
                    return None;
                };
                parts.push(Part {
                    pts: pts.clone(),
                    bulges: bulges.clone(),
                    holes: None,
                });
            }
            Some(Shape::Polyline {
                pts: pts.clone(),
                bulges: bulges.clone(),
                holes: None,
                parts: (!parts.is_empty()).then_some(parts),
            })
        }
        Shape::Point { p, z, .. } => {
            let mut parts = Vec::with_capacity(rest.len());
            for s in rest {
                let Shape::Point { p, z, .. } = s else {
                    return None;
                };
                parts.push(PointPart { p: *p, z: *z });
            }
            Some(Shape::Point {
                p: *p,
                z: *z,
                parts: (!parts.is_empty()).then_some(parts),
            })
        }
        _ => None,
    }
}

/// Where `index` falls among an area's parts when each part counts
/// `count(part)` indices (its vertices, its edges, its grips): the part and
/// the index within it; none past the last part.
pub fn locate_part(
    parts: &[Shape],
    index: usize,
    count: impl Fn(&Shape) -> usize,
) -> Option<(usize, usize)> {
    let mut i = index;
    for (k, part) in parts.iter().enumerate() {
        let n = count(part);
        if i < n {
            return Some((k, i));
        }
        i -= n;
    }
    None
}

/// A path's outer vertices: a polyline's, or an area part's ring; 0 for
/// anything else. Outer vertex indices count an area's parts so, part after
/// part (docs/adr/0143).
pub fn outer_count(e: &Shape) -> usize {
    match e {
        Shape::Polyline { pts, .. } | Shape::Polygon { pts, .. } => pts.len(),
        _ => 0,
    }
}

/// The object with part `k` (0: the object's own fields) replaced by `part`,
/// a one-part object of its kind; none when `k` is past its parts or `part`
/// is of another kind.
pub fn replace_part(e: &Shape, k: usize, part: Shape) -> Option<Shape> {
    let mut parts = area_parts(e).into_owned();
    *parts.get_mut(k)? = part;
    join_parts(&parts)
}

/// Whether p is inside a path's outer ring and outside its holes, in any of
/// an area's parts (tessellated test).
pub fn inside_polygon(e: &Shape, p: Vec2) -> bool {
    if is_multi_part(e) {
        return area_parts(e).iter().any(|part| inside_polygon(part, p));
    }
    let (pts, bulges, holes) = match e {
        Shape::Polyline {
            pts, bulges, holes, ..
        }
        | Shape::Polygon {
            pts, bulges, holes, ..
        } => (pts, bulges, holes),
        _ => return false,
    };
    point_in_polygon(p, &polygon_ring(pts, bulges.as_deref()))
        && !holes
            .iter()
            .flatten()
            .any(|h| point_in_polygon(p, &polygon_ring(&h.pts, h.bulges.as_deref())))
}

/// Rotated box of a text at the left of its baseline: its letters' advances in the drawing's
/// typeface (`text`), one line tall and a little over for descenders and accents.
pub fn text_box(p: Vec2, text: &str, height: f64, rotation: f64, font: Font) -> Vec<Vec2> {
    TextPlace::line(p, text, height, rotation, None, None).outline(font)
}

/// Where and how large a text is (a text object, a block's text piece, a
/// draft): its `p`, which point of it `p` is, its height, turn and width
/// factor (docs/adr/0145). Everything that measures a text measures it
/// through this: its box, its pick, its label's origin and its mask.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TextPlace<'a> {
    pub p: Vec2,
    pub text: &'a str,
    pub height: f64,
    /// Degrees, counter-clockwise from east.
    pub rotation: f64,
    pub align: Option<TextAlign>,
    pub width_factor: Option<f64>,
    /// A multi-line text's box width, line spacing and letter formats
    /// (docs/adr/0182); none, none and none for a text of one line.
    pub box_width: Option<f64>,
    pub line_spacing: Option<f64>,
    pub runs: &'a [crate::text::paragraph::Run],
    /// Its own typeface, bold and slant (docs/adr/0183 §2): measured in the
    /// typeface, bold in the bold table; `lean` (the slant's tangent) leans
    /// its box with its letters. None, false and 0 for the project's look.
    pub font: Option<Font>,
    pub bold: bool,
    pub lean: f64,
}

impl<'a> TextPlace<'a> {
    /// A text shape's place; none for any other shape.
    pub fn of(s: &'a Shape) -> Option<TextPlace<'a>> {
        match s {
            Shape::Text {
                p,
                text,
                height,
                rotation,
                align,
                width_factor,
                box_width,
                line_spacing,
                runs,
                face,
                ..
            } => Some(TextPlace {
                p: *p,
                text,
                height: *height,
                rotation: *rotation,
                align: *align,
                width_factor: *width_factor,
                box_width: *box_width,
                line_spacing: *line_spacing,
                runs: runs.as_deref().unwrap_or_default(),
                font: face.font,
                bold: face.is_bold(),
                lean: face.lean(),
            }),
            _ => None,
        }
    }

    /// A text of one line at `p`: no box, spacing or formats.
    pub fn line(
        p: Vec2,
        text: &'a str,
        height: f64,
        rotation: f64,
        align: Option<TextAlign>,
        width_factor: Option<f64>,
    ) -> TextPlace<'a> {
        TextPlace {
            p,
            text,
            height,
            rotation,
            align,
            width_factor,
            box_width: None,
            line_spacing: None,
            runs: &[],
            font: None,
            bold: false,
            lean: 0.0,
        }
    }

    /// Whether it is laid out in lines (docs/adr/0182): it has a line break,
    /// a box, a line spacing or letter formats.
    pub fn is_paragraph(&self) -> bool {
        self.box_width.is_some()
            || self.line_spacing.is_some()
            || !self.runs.is_empty()
            || self.text.contains('\n')
    }

    /// The typeface it is measured in: its own, else `drawing` (the project's).
    pub fn font_or(&self, drawing: Font) -> Font {
        self.font.unwrap_or(drawing)
    }

    /// Its lines and box (docs/adr/0182 §2).
    pub fn layout(&self, font: Font) -> crate::text::paragraph::Layout {
        crate::text::paragraph::lay_out(&crate::text::paragraph::Paragraph {
            text: self.text,
            runs: self.runs,
            height: self.height,
            width_factor: self.width_factor.unwrap_or(1.0),
            box_width: self.box_width,
            line_spacing: self.line_spacing.unwrap_or(1.0),
            along: self.align.map_or(0.0, TextAlign::along),
            font: self.font_or(font),
            bold: self.bold,
        })
    }

    /// Its box's width, how far its last baseline is under its first (0 for
    /// one line), its line count and its baselines' distance.
    fn extent(&self, font: Font) -> (f64, f64, usize, f64) {
        if !self.is_paragraph() {
            let w = crate::text::width_em_in(self.text, self.font_or(font), self.bold)
                * self.height
                * self.width_factor.unwrap_or(1.0);
            return (w, 0.0, 1, 0.0);
        }
        let laid = self.layout(font);
        let n = laid.lines.len();
        (
            laid.width,
            n.saturating_sub(1) as f64 * laid.pitch,
            n,
            laid.pitch,
        )
    }

    /// Where `p` stands from its origin for `align`: along its box and up
    /// from its first baseline (`text::paragraph::rise`).
    fn shares(&self, align: Option<TextAlign>, font: Font) -> (f64, f64) {
        let Some(a) = align else {
            return (0.0, 0.0);
        };
        let (w, _, n, pitch) = self.extent(font);
        (
            a.along() * w,
            crate::text::paragraph::rise(a.up(), self.height, n, pitch),
        )
    }

    /// Its width, metres: its letters' advances in `font` at its height,
    /// times its width factor; a multi-line text's box.
    pub fn width(&self, font: Font) -> f64 {
        self.extent(font).0
    }

    /// Its direction along the baseline and up from it.
    fn axes(&self) -> (Vec2, Vec2) {
        let r = (self.rotation * PI) / 180.0;
        let (c, s) = (cos(r), sin(r));
        (Vec2::new(c, s), Vec2::new(-s, c))
    }

    /// Where its baseline starts, the point its letters are written from:
    /// `p` less its alignment's share of its width along the baseline and
    /// of its height up from it.
    pub fn origin(&self, font: Font) -> Vec2 {
        if self.align.is_none() {
            return self.p;
        }
        let (u, v) = self.axes();
        let (along, up) = self.shares(self.align, font);
        Vec2::new(
            self.p.x - u.x * along - v.x * up,
            self.p.y - u.y * along - v.y * up,
        )
    }

    /// The box from `(x0, y0)` to `(x1, y1)` in metres of its own frame
    /// (along its baseline and up from it), counted from its origin; a
    /// leaning text's leans with its letters (a point `y` up moves `y` times
    /// the slant's tangent along, docs/adr/0183 §2).
    fn frame(&self, font: Font, (x0, y0): (f64, f64), (x1, y1): (f64, f64)) -> Vec<Vec2> {
        let o = self.origin(font);
        let (u, v) = self.axes();
        let k = self.lean;
        let at = |x: f64, y: f64| {
            let x = x + y * k;
            Vec2::new(o.x + u.x * x + v.x * y, o.y + u.y * x + v.y * y)
        };
        vec![at(x0, y0), at(x1, y0), at(x1, y1), at(x0, y1)]
    }

    /// Its rotated box: its width from its origin, one line tall and a
    /// little over for descenders and accents (0.23 of its height under
    /// the baseline, 1.15 over it).
    pub fn outline(&self, font: Font) -> Vec<Vec2> {
        self.outline_grown(font, 0.0)
    }

    /// Its rotated box `margin` wider all round (a hatch leaves it open so,
    /// docs/adr/0186 §4).
    pub fn outline_grown(&self, font: Font, margin: f64) -> Vec<Vec2> {
        let h = self.height * 1.15;
        let (w, below, _, _) = self.extent(font);
        self.frame(
            font,
            (-margin, -h * 0.2 - below - margin),
            (w + margin, h + margin),
        )
    }

    /// Okunur yap (docs/adr/0145 §3): a text that reads upside down (turned
    /// more than 90° and at most 270°, its turn taken from 0 up to 360) is
    /// turned half round about the middle of its box, so the box stays where
    /// it was; its new point and turn. None for a text that reads.
    pub fn readable(&self, font: Font) -> Option<(Vec2, f64)> {
        if !self.is_paragraph() {
            return self.readable_at(self.width(font));
        }
        // A multi-line text turns about its box's middle too (docs/adr/0182): its point moves w·(1 − 2a)
        // along and 0.92·h − below − 2·rise up its old first baseline.
        let r = ((self.rotation % 360.0) + 360.0) % 360.0;
        if !(r > 90.0 && r <= 270.0) {
            return None;
        }
        let (w, below, _, _) = self.extent(font);
        let (along, rise) = self.shares(self.align, font);
        let t = (r * PI) / 180.0;
        let (u, v) = (Vec2::new(cos(t), sin(t)), Vec2::new(-sin(t), cos(t)));
        let along = w - 2.0 * along;
        let up = self.height * 0.92 - below - 2.0 * rise;
        Some((
            Vec2::new(
                self.p.x + u.x * along + v.x * up,
                self.p.y + u.y * along + v.y * up,
            ),
            (r + 180.0) % 360.0,
        ))
    }

    /// `readable` with its width given (the shared cases give it,
    /// fixtures/text/v1/readable.json). Its point moves w·(1 − 2a) along and
    /// h·(0.92 − 2b) up its old baseline, a and b its alignment's shares
    /// (0.92: its box is 0.23 of its height under the baseline and 1.15 over
    /// it); its turn is 180° more.
    pub fn readable_at(&self, width: f64) -> Option<(Vec2, f64)> {
        let r = ((self.rotation % 360.0) + 360.0) % 360.0;
        if !(r > 90.0 && r <= 270.0) {
            return None;
        }
        let (a, b) = self.align.map_or((0.0, 0.0), |a| (a.along(), a.up()));
        let t = (r * PI) / 180.0;
        let (u, v) = (Vec2::new(cos(t), sin(t)), Vec2::new(-sin(t), cos(t)));
        let along = width * (1.0 - 2.0 * a);
        let up = self.height * (0.92 - 2.0 * b);
        Some((
            Vec2::new(
                self.p.x + u.x * along + v.x * up,
                self.p.y + u.y * along + v.y * up,
            ),
            (r + 180.0) % 360.0,
        ))
    }

    /// Hizayı değiştir (docs/adr/0145 §6, Öznitelikler's Hiza): the point of
    /// its box `to` is, so it stays where it is with that alignment.
    pub fn realigned(&self, to: Option<TextAlign>, font: Font) -> Vec2 {
        if !self.is_paragraph() {
            return self.realigned_at(to, self.width(font));
        }
        // A multi-line text by its box and lines (docs/adr/0182).
        let ((a, b), (a2, b2)) = (self.shares(self.align, font), self.shares(to, font));
        let (u, v) = self.axes();
        Vec2::new(
            self.p.x + u.x * (a2 - a) + v.x * (b2 - b),
            self.p.y + u.y * (a2 - a) + v.y * (b2 - b),
        )
    }

    /// `realigned` with its width given (fixtures/text/v1/realign.json): its
    /// point moves w·(a′ − a) along its baseline and h·(b′ − b) up from it,
    /// a, b its alignment's shares and a′, b′ those of `to`.
    pub fn realigned_at(&self, to: Option<TextAlign>, width: f64) -> Vec2 {
        let shares = |a: Option<TextAlign>| a.map_or((0.0, 0.0), |a| (a.along(), a.up()));
        let ((a, b), (a2, b2)) = (shares(self.align), shares(to));
        let (u, v) = self.axes();
        let along = width * (a2 - a);
        let up = self.height * (b2 - b);
        Vec2::new(
            self.p.x + u.x * along + v.x * up,
            self.p.y + u.y * along + v.y * up,
        )
    }

    /// The box its mask fills (docs/adr/0145 §4): its outline with a tenth
    /// of its height around it.
    pub fn mask(&self, font: Font) -> Vec<Vec2> {
        let (h, m) = (self.height * 1.15, self.height * 0.1);
        let (w, below, _, _) = self.extent(font);
        self.frame(font, (-m, -h * 0.2 - m - below), (w + m, h + m))
    }
}

/// Whether the outline is a closed ring.
pub fn is_closed_outline(e: &Shape) -> bool {
    match e {
        Shape::Polygon { .. } | Shape::Circle { .. } | Shape::Hatch { .. } => true,
        Shape::Spline { closed, .. } => *closed,
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => is_full_ellipse(&ellipse_geom(*c, *major, *ratio, *t0, *t1)),
        _ => false,
    }
}

/// The box of an object; text measured in Barlow (the store measures in the project's face, `entity_bounds_in`).
pub fn entity_bounds(e: &Shape) -> Bounds {
    entity_bounds_in(e, Font::DEFAULT)
}

pub fn entity_bounds_in(e: &Shape, font: Font) -> Bounds {
    let mut b = empty_bounds();
    if is_multi_part(e) {
        // Every part's box (docs/adr/0143).
        for part in area_parts(e).iter() {
            let pb = entity_bounds_in(part, font);
            for q in [Vec2::new(pb.min_x, pb.min_y), Vec2::new(pb.max_x, pb.max_y)] {
                extend_bounds(&mut b, q, 0.0);
            }
        }
        return b;
    }
    match e {
        Shape::Circle { c, r } => {
            extend_bounds(&mut b, *c, *r);
            return b;
        }
        Shape::Text { .. } => {
            for q in TextPlace::of(e)
                .map(|t| t.outline(font))
                .unwrap_or_default()
            {
                extend_bounds(&mut b, q, 0.0);
            }
            return b;
        }
        // Construction lines count by their base point only (zoom extents ignores their reach).
        Shape::Xline { p, .. } | Shape::Ray { p, .. } => {
            extend_bounds(&mut b, *p, 0.0);
            return b;
        }
        // Its vertices, its arrowhead, its landing and its note's box (docs/adr/0146 §4).
        Shape::Leader { pts, .. } => {
            let laid = leader::layout_of(e);
            let reach = laid.iter().flat_map(|l| leader::head_reach(&l.head));
            let landing = laid.iter().filter_map(|l| l.landing.map(|[_, end]| end));
            let note = leader::note_place(e).map(|t| t.outline(font));
            for q in pts.iter().copied().chain(reach).chain(landing).chain(note.into_iter().flatten()) {
                extend_bounds(&mut b, q, 0.0);
            }
            return b;
        }
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            for q in tessellate_ellipse(&ellipse_geom(*c, *major, *ratio, *t0, *t1), 256.0) {
                extend_bounds(&mut b, q, 0.0);
            }
            return b;
        }
        _ => {}
    }
    let curved = match e {
        Shape::Arc { .. } | Shape::Spline { .. } | Shape::Dimension { .. } => true,
        Shape::Polyline { bulges, .. } | Shape::Polygon { bulges, .. } => {
            has_bulges(bulges.as_deref())
        }
        _ => false,
    };
    if curved {
        for q in entity_outline(e, 72.0) {
            extend_bounds(&mut b, q, 0.0);
        }
        if let Shape::Dimension {
            a,
            b: bb,
            height,
            style,
            ..
        } = e
        {
            // The new kinds' values stand away from their points (docs/adr/0147).
            let at = is_new_kind(style.as_deref())
                .then(|| layout_of(e).map(|l| l.text_at))
                .flatten()
                .unwrap_or(Vec2::new((a.x + bb.x) / 2.0, (a.y + bb.y) / 2.0));
            extend_bounds(&mut b, at, height * 2.0);
        }
        return b;
    }
    for q in entity_vertices(e) {
        extend_bounds(&mut b, q, 0.0);
    }
    b
}

/// The part a multi-part object's label goes on: an area's largest
/// (docs/adr/0143), a polyline's longest (docs/adr/0174), the first of
/// equals; any other object (a multi-point object too) is its own.
pub fn label_part(e: &Shape) -> Cow<'_, Shape> {
    let measure: fn(&Shape) -> Option<f64> = match e {
        Shape::Polygon { .. } if is_multi_part(e) => entity_area,
        Shape::Polyline { .. } if is_multi_part(e) => entity_length,
        _ => return Cow::Borrowed(e),
    };
    let mut parts = area_parts(e).into_owned();
    let mut best: Option<(f64, usize)> = None;
    for (i, part) in parts.iter().enumerate() {
        let m = measure(part).unwrap_or(0.0);
        if best.is_none_or(|(most, _)| m > most) {
            best = Some((m, i));
        }
    }
    match best {
        Some((_, i)) => Cow::Owned(parts.swap_remove(i)),
        None => Cow::Borrowed(e),
    }
}

/// Where a label sits; None for a path without vertices (the TypeScript's undefined).
pub fn entity_anchor(e: &Shape) -> Option<Vec2> {
    Some(match e {
        // A multi-part area's or polyline's label goes on its `label_part`.
        Shape::Polygon { .. } | Shape::Polyline { .. } if is_multi_part(e) => {
            let part = label_part(e);
            return if is_multi_part(&part) {
                None
            } else {
                entity_anchor(&part)
            };
        }
        Shape::Polygon { pts, bulges, .. } => centroid(&polygon_ring(pts, bulges.as_deref())),
        Shape::Circle { c, .. } | Shape::Ellipse { c, .. } => *c,
        Shape::Arc { c, r, a0, a1 } => arc_mid(&ArcGeom {
            c: *c,
            r: *r,
            a0: *a0,
            a1: *a1,
        }),
        Shape::Line { a, b } => Vec2::new((a.x + b.x) / 2.0, (a.y + b.y) / 2.0),
        Shape::Polyline { pts, .. } | Shape::Spline { pts, .. } | Shape::Leader { pts, .. } => {
            return pts.get(pts.len() / 2).copied();
        }
        Shape::Dimension { a, .. } => layout_of(e).map_or(*a, |l| l.text_at),
        Shape::Hatch { ring, .. } => centroid(ring),
        Shape::Point { p, .. }
        | Shape::Text { p, .. }
        | Shape::Xline { p, .. }
        | Shape::Ray { p, .. }
        | Shape::Insert { p, .. }
        | Shape::Table { p, .. } => *p,
    })
}

pub fn entity_length(e: &Shape) -> Option<f64> {
    match e {
        Shape::Line { a, b } => Some(path_length(&[*a, *b], false)),
        // A multi-part area's perimeter and a multi-part polyline's length
        // are their parts' (docs/adr/0143, 0174).
        Shape::Polygon { .. } | Shape::Polyline { .. } if is_multi_part(e) => {
            Some(area_parts(e).iter().filter_map(entity_length).sum())
        }
        Shape::Polyline {
            pts, bulges, holes, ..
        }
        | Shape::Polygon {
            pts, bulges, holes, ..
        } => {
            // A polygon's perimeter includes its holes (as in GIS).
            let closed = matches!(e, Shape::Polygon { .. });
            let holes = holes.iter().flatten().fold(0.0, |s, h| {
                s + bulge_path_length(&h.pts, h.bulges.as_deref(), true)
            });
            Some(bulge_path_length(pts, bulges.as_deref(), closed) + holes)
        }
        Shape::Circle { r, .. } => Some(2.0 * PI * r),
        Shape::Arc { c, r, a0, a1 } => Some(arc_length(&ArcGeom {
            c: *c,
            r: *r,
            a0: *a0,
            a1: *a1,
        })),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => Some(ellipse_length(&ellipse_geom(*c, *major, *ratio, *t0, *t1))),
        // The curve itself, not the outline it is drawn with (docs/adr/0149 §3).
        Shape::Spline { pts, closed } => Some(spline_length(pts, *closed)),
        // Its vertices' length; the landing does not count (docs/adr/0146 §2).
        Shape::Leader { pts, .. } => Some(path_length(pts, false)),
        // The measured value when it is a length (an angle has none).
        Shape::Dimension { .. } => layout_of(e).filter(|l| l.unit == "length").map(|l| l.value),
        _ => None,
    }
}

pub fn entity_area(e: &Shape) -> Option<f64> {
    match e {
        // A multi-part area's area is its parts' (docs/adr/0143).
        Shape::Polygon { .. } if is_multi_part(e) => {
            Some(area_parts(e).iter().filter_map(entity_area).sum())
        }
        Shape::Polygon {
            pts, bulges, holes, ..
        } => {
            let holes = holes.iter().flatten().fold(0.0, |s, h| {
                s + bulge_ring_area(&h.pts, h.bulges.as_deref()).abs()
            });
            Some(bulge_ring_area(pts, bulges.as_deref()).abs() - holes)
        }
        Shape::Circle { r, .. } => Some(PI * r * r),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => {
            let g = ellipse_geom(*c, *major, *ratio, *t0, *t1);
            is_full_ellipse(&g).then(|| ellipse_area(&g))
        }
        Shape::Hatch { ring, holes, .. } => Some(
            signed_area(ring).abs()
                - holes
                    .iter()
                    .flatten()
                    .fold(0.0, |s, h| s + signed_area(h).abs()),
        ),
        _ => None,
    }
}

/// Geometry-only view of an entity: drops id, layer, colour, attributes and label.
pub fn entity_geometry(e: &Entity) -> Entity {
    const DROP: [&str; 5] = ["id", "layerId", "attrs", "color", "label"];
    Entity {
        shape: e.shape.clone(),
        rest: e
            .rest
            .iter()
            .filter(|(k, _)| !DROP.contains(&k.as_str()))
            .cloned()
            .collect(),
    }
}

pub(crate) static OPS: &[Op] = &[
    op!(
        "tessellateCircle",
        |c: Vec2, r: f64, segments: Option<f64>| tessellate_circle(c, r, segments.unwrap_or(72.0))
    ),
    op!("entityVertices", |e: Entity| entity_vertices(&e.shape)),
    op!("entityOutline", |e: Entity, segments: Option<f64>| {
        entity_outline(&e.shape, segments.unwrap_or(72.0))
    }),
    op!("polygonRing", |r: Ring| polygon_ring(
        &r.pts,
        r.bulges.as_deref()
    )),
    op!("polygonHoles", |e: Entity| polygon_holes(&e.shape)),
    op!("insidePolygon", |e: Entity, p: Vec2| inside_polygon(
        &e.shape, p
    )),
    op!("textBox", |e: Json| text_box_json(&e)),
    // Where a text's point is on it (docs/adr/0145): [along its width, up of its height]; none: [0, 0].
    op!("textAlignShares", |a: Option<TextAlign>| a
        .map_or([0.0, 0.0], |a| [a.along(), a.up()])),
    // Artır (docs/adr/0145 §3): the text with the number it ends with one more; none without one.
    op!("textIncrement", |t: String| crate::text::edit::increment(&t)),
    // Bul ve değiştir: each text as it becomes, none where nothing matched ({wildcard, caseless, wholeWord}).
    op!("textReplace", |texts: Vec<String>,
                        find: String,
                        with: String,
                        how: crate::text::edit::Find| texts
        .iter()
        .map(|t| crate::text::edit::replace(t, &find, &with, how))
        .collect::<Vec<_>>()),
    // Okunur yap: a text's new point and turn, none when it reads (`text_readable_json`).
    op!("textReadable", |t: Json| text_readable_json(&t)),
    // Hizayı değiştir: a text's point with the alignment `to` (null: the left of the baseline), where it stays.
    op!("textRealign", |t: Json, to: Option<TextAlign>| text_realign_json(&t, to)),
    // A multi-line text's label records as the store gives them (docs/adr/0182 §3), for a text to come
    // (its mask's box with `mask`, then its lines): what the tools' previews draw.
    op!("textLines", |t: Json| text_lines_json(&t)),
    // A format toggled over the letters start..end of a text of `len` letters (the editor's buttons).
    op!(
        "textRunsToggle",
        |runs: Vec<crate::text::paragraph::Run>,
         len: usize,
         start: usize,
         end: usize,
         toggle: crate::text::paragraph::Toggle| crate::text::paragraph::toggle(
            &runs, len, start, end, &toggle
        )
    ),
    // The runs after the editor's text `before` became `after`.
    op!("textRunsRetext", |runs: Vec<
        crate::text::paragraph::Run,
    >,
                           before: String,
                           after: String| {
        crate::text::paragraph::retext(&runs, &before, &after)
    }),
    // A text's lines and box (docs/adr/0182 §2): the shared cases' layout (fixtures/text/v1/paragraph.json).
    op!("textLayout", |t: Json| text_layout_json(&t)),
    // Çok satırlı yazı's box from two corners for a text turned `rotation` degrees (docs/adr/0182 §4):
    // its top left corner and its width along the turn (null: no box).
    op!("textCornerBox", |a: Vec2, b: Vec2, rotation: f64| {
        let (corner, width) = crate::text::paragraph::corner_box(a, b, rotation);
        CornerBox { corner, width }
    }),
    op!("isClosedOutline", |e: Entity| is_closed_outline(&e.shape)),
    op!("entityBounds", |e: Entity| entity_bounds(&e.shape)),
    op!("entityAnchor", |e: Entity| entity_anchor(&e.shape)),
    op!("entityLength", |e: Entity| entity_length(&e.shape)),
    op!("entityArea", |e: Entity| entity_area(&e.shape)),
    op!("entityGeometry", |e: Entity| entity_geometry(&e)),
];

/// A text turned to read (`TextPlace::readable`): its new point and turn.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Turned {
    pub p: Vec2,
    pub rotation: f64,
}

crate::json_struct!(out Turned { p, rotation });

/// `textReadable` takes a text's p, height, rotation and, when it has them,
/// its alignment and width factor, and its width in metres (`width`) or its
/// `text` and the drawing typeface as `font` to measure it by.
fn text_readable_json(v: &Json) -> Result<Option<Turned>, String> {
    let font = match v.get("font") {
        Json::Str(id) => Font::from_id(id),
        _ => Font::DEFAULT,
    };
    let text: Option<String> = json::read_field(v, "text")?;
    let runs: Option<Vec<crate::text::paragraph::Run>> = json::read_field(v, "runs")?;
    let place = TextPlace {
        p: json::read_field(v, "p")?,
        text: text.as_deref().unwrap_or(""),
        height: json::read_field(v, "height")?,
        rotation: json::read_field(v, "rotation")?,
        align: json::read_field(v, "align")?,
        width_factor: json::read_field(v, "widthFactor")?,
        box_width: json::read_field(v, "boxWidth")?,
        line_spacing: json::read_field(v, "lineSpacing")?,
        runs: runs.as_deref().unwrap_or_default(),
        font: None,
        bold: bold_of(v)?,
        lean: lean_of(v)?,
    };
    let width: Option<f64> = json::read_field(v, "width")?;
    let turned = match width {
        Some(w) => place.readable_at(w),
        None => place.readable(font),
    };
    Ok(turned.map(|(p, rotation)| Turned { p, rotation }))
}

/// `textRealign` takes a text as `textReadable` does, and the alignment it is to have.
fn text_realign_json(v: &Json, to: Option<TextAlign>) -> Result<Vec2, String> {
    let font = match v.get("font") {
        Json::Str(id) => Font::from_id(id),
        _ => Font::DEFAULT,
    };
    let text: Option<String> = json::read_field(v, "text")?;
    let runs: Option<Vec<crate::text::paragraph::Run>> = json::read_field(v, "runs")?;
    let place = TextPlace {
        p: json::read_field(v, "p")?,
        text: text.as_deref().unwrap_or(""),
        height: json::read_field(v, "height")?,
        rotation: json::read_field(v, "rotation")?,
        align: json::read_field(v, "align")?,
        width_factor: json::read_field(v, "widthFactor")?,
        box_width: json::read_field(v, "boxWidth")?,
        line_spacing: json::read_field(v, "lineSpacing")?,
        runs: runs.as_deref().unwrap_or_default(),
        font: None,
        bold: bold_of(v)?,
        lean: lean_of(v)?,
    };
    let width: Option<f64> = json::read_field(v, "width")?;
    Ok(match width {
        Some(w) => place.realigned_at(to, w),
        None => place.realigned(to, font),
    })
}

/// A text's own bold as the JSON ops read it (docs/adr/0183 §2): with the
/// typeface they measure it in (`font`, the text's own or the drawing's).
fn bold_of(v: &Json) -> Result<bool, String> {
    use crate::api::json::Flat;
    Ok(crate::text::face::Face::read_flat(v)?.is_bold())
}

/// A text's slant's tangent as the JSON ops read it: 0 without one.
fn lean_of(v: &Json) -> Result<f64, String> {
    use crate::api::json::Flat;
    Ok(crate::text::face::Face::read_flat(v)?.lean())
}

/// A text place from JSON as `textBox` reads it, and the typeface it names.
fn text_place_json(v: &Json) -> Result<(String, Vec<crate::text::paragraph::Run>, Font), String> {
    let font = match v.get("font") {
        Json::Str(id) => Font::from_id(id),
        _ => Font::DEFAULT,
    };
    let text: String = json::read_field(v, "text")?;
    let runs: Option<Vec<crate::text::paragraph::Run>> = json::read_field(v, "runs")?;
    Ok((text, runs.unwrap_or_default(), font))
}

/// `textLines` takes a text as `textBox` does, and `mask` when it has one.
fn text_lines_json(v: &Json) -> Result<Vec<f64>, String> {
    let (text, runs, font) = text_place_json(v)?;
    let place = TextPlace {
        p: json::read_field(v, "p")?,
        text: &text,
        height: json::read_field(v, "height")?,
        rotation: json::read_field(v, "rotation")?,
        align: json::read_field(v, "align")?,
        width_factor: json::read_field(v, "widthFactor")?,
        box_width: json::read_field(v, "boxWidth")?,
        line_spacing: json::read_field(v, "lineSpacing")?,
        runs: &runs,
        font: None,
        bold: bold_of(v)?,
        lean: lean_of(v)?,
    };
    let mask: Option<bool> = json::read_field(v, "mask")?;
    let mut out = Vec::new();
    crate::store::labels::paragraph_records(&place, font, mask == Some(true), 0.0, None, &mut out);
    Ok(out)
}

/// Çok satırlı yazı's box (`textCornerBox`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CornerBox {
    pub corner: Vec2,
    pub width: Option<f64>,
}

crate::json_struct!(out CornerBox { corner, width });

/// A text's lines and box as the shared cases write them.
#[derive(Clone, Debug, PartialEq)]
pub struct LaidOut {
    pub lines: Vec<crate::text::paragraph::Line>,
    pub width: f64,
    pub pitch: f64,
    /// Where `p` stands from the origin: along the box and up from the first baseline.
    pub shares: [f64; 2],
}

crate::json_struct!(out LaidOut { lines, width, pitch, shares });

/// `textLayout` takes a text as `textBox` does.
fn text_layout_json(v: &Json) -> Result<LaidOut, String> {
    let (text, runs, font) = text_place_json(v)?;
    let place = TextPlace {
        p: json::read_field(v, "p")?,
        text: &text,
        height: json::read_field(v, "height")?,
        rotation: json::read_field(v, "rotation")?,
        align: json::read_field(v, "align")?,
        width_factor: json::read_field(v, "widthFactor")?,
        box_width: json::read_field(v, "boxWidth")?,
        line_spacing: json::read_field(v, "lineSpacing")?,
        runs: &runs,
        font: None,
        bold: bold_of(v)?,
        lean: lean_of(v)?,
    };
    let laid = place.layout(font);
    let (along, up) = place.shares(place.align, font);
    Ok(LaidOut {
        lines: laid.lines,
        width: laid.width,
        pitch: laid.pitch,
        shares: [along, up],
    })
}

/// `textBox` takes any object with p, text, height and rotation, and an alignment and width factor
/// when it has them (a text entity or a draft), and the drawing typeface as `font` (a `DrawingFont`
/// id; Barlow without one).
fn text_box_json(v: &Json) -> Result<Vec<Vec2>, String> {
    let font = match v.get("font") {
        Json::Str(id) => Font::from_id(id),
        _ => Font::DEFAULT,
    };
    let text: String = json::read_field(v, "text")?;
    let runs: Option<Vec<crate::text::paragraph::Run>> = json::read_field(v, "runs")?;
    let place = TextPlace {
        p: json::read_field(v, "p")?,
        text: &text,
        height: json::read_field(v, "height")?,
        rotation: json::read_field(v, "rotation")?,
        align: json::read_field(v, "align")?,
        width_factor: json::read_field(v, "widthFactor")?,
        box_width: json::read_field(v, "boxWidth")?,
        line_spacing: json::read_field(v, "lineSpacing")?,
        runs: runs.as_deref().unwrap_or_default(),
        font: None,
        bold: bold_of(v)?,
        lean: lean_of(v)?,
    };
    Ok(place.outline(font))
}

impl FromJson for Json {
    fn from_json(v: &Json) -> Result<Json, String> {
        Ok(v.clone())
    }
}
