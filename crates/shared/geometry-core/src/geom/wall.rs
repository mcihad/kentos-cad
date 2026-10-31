//! A wall's drawing from its source (docs/adr/0339 §1, plan/cad.md `CAD-ARCH-02`, `12`): its path is its base line;
//! the justification line is the path moved `offset` to its left; the left face lies `left_face` further left (all
//! of the width for a right-justified wall, half for a centred one, none for a left one), the right face a width to
//! the right of it, and the components' boundaries between, in their order from the left face. Each line is the
//! path offset (`geom::offset::offset_bulge_path`: mitred corners, a corner sharper than the mitre limit bevelled,
//! arcs about their centres). At a detail level the wall is drawn as:
//!
//! - its two faces, and across an open wall's two ends a cap each unless its ends are open;
//! - schematically, its body's fill when it has one;
//! - from the standard detail on, each component shown at that detail its hatch (its band, the pattern turned with the
//!   wall's first edge, as large on paper at the drawing's scale as the library's pattern) and each boundary between
//!   two shown components a line (the left one's edge look).
//!
//! The hatches come first, then the lines, so the lines are drawn over them. Nothing here is stored: the store
//! expands a wall into these pieces as it expands a block's insert into its definition's.

use crate::api::json::{FromJson, Json, ToJson, field};
use crate::block::{Piece, PieceStroke};
use crate::entity::{HatchPattern, Shape};
use crate::geom::arc::DEFAULT_STEP;
use crate::geom::bulge::{bulge_path_outline, has_bulges};
use crate::geom::hatch_pattern::library_pattern;
use crate::geom::offset::{OffsetResult, offset_bulge_path};
use crate::geometry::signed_area;
use crate::jsmath::{PI, atan2};
use crate::vec2::Vec2;

/// Conservative path box used by the native wall join candidate search.
/// Keep this arithmetic identical for changed walls and indexed stored walls;
/// it is a candidate policy, not the exact outline (or an arc-bounds solver).
#[allow(
    clippy::disallowed_methods,
    reason = "Preserve the former native reach-box f64 min/max policy, including malformed NaN input; this helper is not a JS geometry operation."
)]
pub fn join_reach_bounds(
    points: impl IntoIterator<Item = Vec2>,
    width: f64,
    offset: f64,
    grow: f64,
) -> crate::geometry::Bounds {
    let r = width + offset.abs() + grow;
    let mut b = crate::geometry::empty_bounds();
    for p in points {
        b.min_x = b.min_x.min(p.x - r);
        b.min_y = b.min_y.min(p.y - r);
        b.max_x = b.max_x.max(p.x + r);
        b.max_y = b.max_y.max(p.y + r);
    }
    b
}

/// Where a wall's body lies across its path (the contract's `WallJustify`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Justify {
    Left,
    #[default]
    Center,
    Right,
}

impl Justify {
    pub fn name(self) -> &'static str {
        match self {
            Justify::Left => "left",
            Justify::Center => "center",
            Justify::Right => "right",
        }
    }

    pub fn of(name: &str) -> Option<Justify> {
        [Justify::Left, Justify::Center, Justify::Right]
            .into_iter()
            .find(|j| j.name() == name)
    }
}

/// How much of a wall is drawn, the least first (the contract's `WallDetail`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Detail {
    Schematic,
    #[default]
    Standard,
    Detailed,
}

impl Detail {
    pub fn name(self) -> &'static str {
        match self {
            Detail::Schematic => "schematic",
            Detail::Standard => "standard",
            Detail::Detailed => "detailed",
        }
    }

    pub fn of(name: &str) -> Option<Detail> {
        [Detail::Schematic, Detail::Standard, Detail::Detailed]
            .into_iter()
            .find(|d| d.name() == name)
    }
}

/// A component's hatch: a library pattern by name (`SOLID` a solid fill), times `scale` its paper size, turned
/// `angle` degrees from the wall's first edge.
#[derive(Clone, Debug, PartialEq)]
pub struct ComponentFill {
    pub pattern: String,
    pub scale: f64,
    pub angle: f64,
    pub color: Option<String>,
}

crate::json_struct!(ComponentFill {
    pattern,
    scale,
    angle,
    color
});

/// The lines along a component's sides.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ComponentEdge {
    pub color: Option<String>,
    pub line_weight: Option<f64>,
    pub line_type: Option<String>,
}

crate::json_struct!(ComponentEdge {
    color,
    line_weight => "lineWeight",
    line_type => "lineType"
});

/// A component's priority when it has none (docs/adr/0340 §1): between a core's 1 and a finish's 5.
pub const DEFAULT_PRIORITY: u8 = 3;

/// A layer of the body, from the left face: its thickness (none: the rest), its look, the least detail it shows and
/// its priority at a join (1 the strongest; none [`DEFAULT_PRIORITY`]).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Component {
    pub name: Option<String>,
    pub material: Option<String>,
    /// The project material it binds to (docs/adr/0785), by its id; carried, not drawn by.
    pub material_id: Option<String>,
    pub thickness: Option<f64>,
    pub edge: Option<ComponentEdge>,
    pub hatch: Option<ComponentFill>,
    pub detail: Detail,
    pub priority: Option<u8>,
    /// Uçta döner (docs/adr/0771 §3): an outermost layer that turns across the wall's free ends and its openings' jambs.
    pub wrap: bool,
}

impl Component {
    pub fn rank(&self) -> u8 {
        self.priority.unwrap_or(DEFAULT_PRIORITY)
    }
}

/// How one end of a wall joins (docs/adr/0340 §1, the contract's `WallJoin`): by the rule (`Auto`), not at all, or
/// at a corner mitred, butting the other wall, or running through it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum JoinMode {
    #[default]
    Auto,
    None,
    Miter,
    Butt,
    Through,
}

impl JoinMode {
    pub fn name(self) -> &'static str {
        match self {
            JoinMode::Auto => "auto",
            JoinMode::None => "none",
            JoinMode::Miter => "miter",
            JoinMode::Butt => "butt",
            JoinMode::Through => "through",
        }
    }

    pub fn of(name: &str) -> Option<JoinMode> {
        [
            JoinMode::Auto,
            JoinMode::None,
            JoinMode::Miter,
            JoinMode::Butt,
            JoinMode::Through,
        ]
        .into_iter()
        .find(|m| m.name() == name)
    }
}

impl ToJson for Component {
    fn write_json(&self, out: &mut String) {
        out.push('{');
        let mut first = true;
        field(out, &mut first, "name", &self.name);
        field(out, &mut first, "material", &self.material);
        if self.material_id.is_some() {
            field(out, &mut first, "materialId", &self.material_id);
        }
        field(out, &mut first, "thickness", &self.thickness);
        field(out, &mut first, "edge", &self.edge);
        field(out, &mut first, "hatch", &self.hatch);
        field(out, &mut first, "detail", self.detail.name());
        field(out, &mut first, "priority", &self.priority.map(f64::from));
        if self.wrap {
            field(out, &mut first, "wrap", &true);
        }
        out.push('}');
    }
}

impl FromJson for Component {
    fn from_json(v: &Json) -> Result<Component, String> {
        let detail = match v.get("detail") {
            Json::Str(s) => Detail::of(s).ok_or_else(|| format!("bilinmeyen ayrıntı “{s}”"))?,
            _ => Detail::Standard,
        };
        Ok(Component {
            name: Option::from_json(v.get("name"))?,
            material: Option::from_json(v.get("material"))?,
            material_id: Option::from_json(v.get("materialId"))?,
            thickness: Option::from_json(v.get("thickness"))?,
            edge: Option::from_json(v.get("edge"))?,
            hatch: Option::from_json(v.get("hatch"))?,
            detail,
            priority: match Option::<f64>::from_json(v.get("priority"))? {
                None => None,
                Some(p) if (1.0..=9.0).contains(&p) && p.fract() == 0.0 => Some(p as u8),
                Some(p) => return Err(format!("bileşenin önceliği 1 ile 9 arası bir tam sayıdır, {p} değil")),
            },
            wrap: matches!(v.get("wrap"), Json::Bool(true)),
        })
    }
}

/// A wall's fields beside its path: whether the path closes, its width, justification, base line offset, components,
/// whether its ends are left open, its schematic fill, how its start and end join and whether its components are
/// cleaned up at a join; and, carried through untouched, its style, height and architectural information (the
/// contract's JSON).
#[derive(Clone, Debug, PartialEq)]
pub struct Spec {
    pub closed: bool,
    pub width: f64,
    pub justify: Justify,
    pub offset: f64,
    pub components: Vec<Component>,
    pub caps_open: bool,
    pub fill: Option<String>,
    pub style: Option<String>,
    pub height: Option<f64>,
    pub arch: Option<Json>,
    /// Its start's and its end's join.
    pub joins: [JoinMode; 2],
    /// Its start's and its end's node decision (docs/adr/0771 §1): where it tees into a host, how many of the host's
    /// components each of its own passes; none: by the priorities.
    pub passes: [Option<Vec<u8>>; 2],
    /// Its components are cleaned up by their priorities where another wall tees into or crosses it.
    pub cleanup: bool,
}

impl Default for Spec {
    fn default() -> Self {
        Self {
            closed: false,
            width: 0.2,
            justify: Justify::Center,
            offset: 0.0,
            components: Vec::new(),
            caps_open: false,
            fill: None,
            style: None,
            height: None,
            arch: None,
            joins: [JoinMode::Auto; 2],
            passes: [None, None],
            cleanup: true,
        }
    }
}

impl crate::api::json::Flat for Spec {
    const NAMES: &'static [&'static str] = &[
        "closed",
        "width",
        "justify",
        "offset",
        "components",
        "caps",
        "fill",
        "style",
        "height",
        "arch",
        "joins",
        "cleanup",
    ];

    fn read_flat(v: &Json) -> Result<Self, String> {
        let justify = match v.get("justify") {
            Json::Str(s) => Justify::of(s).ok_or_else(|| format!("bilinmeyen hiza “{s}”"))?,
            _ => Justify::Center,
        };
        Ok(Spec {
            closed: matches!(v.get("closed"), Json::Bool(true)),
            width: f64::from_json(v.get("width"))?,
            justify,
            offset: Option::<f64>::from_json(v.get("offset"))?.unwrap_or(0.0),
            components: Option::<Vec<Component>>::from_json(v.get("components"))?
                .unwrap_or_default(),
            caps_open: matches!(v.get("caps"), Json::Str(s) if s == "open"),
            fill: Option::from_json(v.get("fill"))?,
            style: Option::from_json(v.get("style"))?,
            height: Option::from_json(v.get("height"))?,
            arch: match v.get("arch") {
                Json::Null => None,
                a => Some(a.clone()),
            },
            joins: {
                let at = |name: &str| match v.get("joins").get(name) {
                    Json::Str(s) => JoinMode::of(s).ok_or_else(|| format!("bilinmeyen birleşim “{s}”")),
                    _ => Ok(JoinMode::Auto),
                };
                [at("start")?, at("end")?]
            },
            passes: {
                let at = |name: &str| -> Result<Option<Vec<u8>>, String> {
                    match v.get("joins").get(name) {
                        Json::Arr(list) => list
                            .iter()
                            .map(|x| match x {
                                Json::Num(n) if *n >= 0.0 && *n <= 255.0 && n.fract() == 0.0 => Ok(*n as u8),
                                _ => Err("düğüm kararı negatif olmayan tam sayılardır".to_owned()),
                            })
                            .collect::<Result<Vec<u8>, String>>()
                            .map(Some),
                        _ => Ok(None),
                    }
                };
                [at("startPass")?, at("endPass")?]
            },
            cleanup: !matches!(v.get("cleanup"), Json::Bool(false)),
        })
    }

    fn write_flat(&self, out: &mut String, first: &mut bool) {
        if self.closed {
            field(out, first, "closed", &true);
        }
        field(out, first, "width", &self.width);
        field(out, first, "justify", self.justify.name());
        if self.offset != 0.0 {
            field(out, first, "offset", &self.offset);
        }
        if !self.components.is_empty() {
            field(out, first, "components", &self.components);
        }
        if self.caps_open {
            field(out, first, "caps", "open");
        }
        field(out, first, "fill", &self.fill);
        field(out, first, "style", &self.style);
        field(out, first, "height", &self.height);
        if let Some(a) = &self.arch {
            field(out, first, "arch", a);
        }
        if self.joins != [JoinMode::Auto; 2] || self.passes.iter().any(Option::is_some) {
            let mut joins: Vec<(String, Json)> = [("start", self.joins[0]), ("end", self.joins[1])]
                .into_iter()
                .filter(|(_, m)| *m != JoinMode::Auto)
                .map(|(k, m)| (k.to_owned(), Json::Str(m.name().to_owned())))
                .collect();
            for (k, p) in [("startPass", &self.passes[0]), ("endPass", &self.passes[1])] {
                if let Some(p) = p {
                    joins.push((k.to_owned(), Json::Arr(p.iter().map(|&x| Json::Num(f64::from(x))).collect())));
                }
            }
            let joins = Json::Obj(joins.into_iter().collect());
            field(out, first, "joins", &joins);
        }
        if !self.cleanup {
            field(out, first, "cleanup", &false);
        }
    }
}

/// What a wall is drawn for: the detail and the drawing's scale (1:`scale`), which sizes its patterns on paper.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct View {
    pub detail: Detail,
    pub scale: f64,
}

impl Default for View {
    fn default() -> Self {
        Self {
            detail: Detail::Standard,
            scale: 100.0,
        }
    }
}

/// Each component's thickness, the rest given to the one without; none when they do not fit the width.
pub fn thicknesses(spec: &Spec) -> Option<Vec<f64>> {
    let w = spec.width;
    let fixed: f64 = spec
        .components
        .iter()
        .filter_map(|c| c.thickness)
        .fold(0.0, |a, b| a + b);
    let rest = spec
        .components
        .iter()
        .filter(|c| c.thickness.is_none())
        .count();
    let slack = 1e-9 * if w > 1.0 { w } else { 1.0 };
    match rest {
        0 if (fixed - w).abs() <= slack => Some(
            spec.components
                .iter()
                .map(|c| c.thickness.unwrap_or(0.0))
                .collect(),
        ),
        1 if fixed < w - slack => Some(
            spec.components
                .iter()
                .map(|c| c.thickness.unwrap_or(w - fixed))
                .collect(),
        ),
        _ => None,
    }
}

/// The left face's distance from the path, positive to its left.
pub fn left_face(spec: &Spec) -> f64 {
    spec.offset
        + match spec.justify {
            Justify::Left => 0.0,
            Justify::Center => spec.width / 2.0,
            Justify::Right => spec.width,
        }
}

/// The distances from the path of the wall's lines, the left face first, then each boundary between two components,
/// the right face last; a wall without components (or whose components do not fit) has its two faces only.
pub fn boundaries(spec: &Spec) -> Vec<f64> {
    let l = left_face(spec);
    let mut out = vec![l];
    if !spec.components.is_empty()
        && let Some(t) = thicknesses(spec)
    {
        let mut at = l;
        for x in &t[..t.len() - 1] {
            at -= x;
            out.push(at);
        }
    }
    out.push(l - spec.width);
    out
}

/// A line of the wall: the path offset `d` to its left, its bulges one per vertex.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub pts: Vec<Vec2>,
    pub bulges: Vec<f64>,
}

impl Line {
    pub fn shape(&self, closed: bool) -> Shape {
        Shape::Polyline {
            pts: self.pts.clone(),
            bulges: has_bulges(Some(&self.bulges)).then(|| self.bulges.clone()),
            holes: None,
            parts: None,
            closed,
            widths: None,
        }
    }

    /// Its points with each arc as chords.
    pub fn outline(&self, closed: bool) -> Vec<Vec2> {
        bulge_path_outline(&self.pts, Some(&self.bulges), closed, DEFAULT_STEP)
    }
}

/// The path offset `d` to its left, or why it cannot be (an arc's radius it would collapse).
pub fn line_at(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool, d: f64) -> Result<Line, String> {
    let n = pts.len();
    let b: Vec<f64> = (0..n)
        .map(|i| bulges.and_then(|b| b.get(i)).copied().unwrap_or(0.0))
        .collect();
    match offset_bulge_path(pts, &b, d, closed) {
        OffsetResult::Path { pts, bulges } => Ok(Line { pts, bulges }),
        OffsetResult::Error { error } => Err(error),
    }
}

/// Every line of the wall (`boundaries`' order), or why one cannot be drawn.
pub fn lines(pts: &[Vec2], bulges: Option<&[f64]>, spec: &Spec) -> Result<Vec<Line>, String> {
    if pts.len() < 2 {
        return Err("Duvarın yolu en az iki noktadır.".into());
    }
    boundaries(spec)
        .into_iter()
        .map(|d| line_at(pts, bulges, spec.closed, d))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|why| format!("Duvar çizilemiyor: {why} Duvarı inceltin ya da yayı büyütün."))
}

/// A wall's body: its outer ring, and a closed wall's hole.
pub type Body = (Vec<Vec2>, Option<Vec<Vec<Vec2>>>);

/// The region between two of its lines: an open wall's the first line on, the second back; a closed wall's the larger
/// ring and the smaller as its hole.
pub(crate) fn band(a: &Line, b: &Line, closed: bool) -> Body {
    let (pa, pb) = (a.outline(closed), b.outline(closed));
    if closed {
        let (outer, inner) = if signed_area(&pa).abs() >= signed_area(&pb).abs() {
            (pa, pb)
        } else {
            (pb, pa)
        };
        return (outer, Some(vec![inner]));
    }
    let mut ring = pa;
    ring.extend(pb.into_iter().rev());
    (ring, None)
}

/// The body's outline: the faces' band (a closed wall's outer ring and its hole).
pub fn body(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    spec: &Spec,
) -> Result<Body, String> {
    let all = lines(pts, bulges, spec)?;
    Ok(band(&all[0], &all[all.len() - 1], spec.closed))
}

/// The body's area, exact on its arcs: an open wall's ring (its left face on, its right face back), a closed wall's
/// band between its faces; none when it cannot be drawn.
pub fn area(pts: &[Vec2], bulges: Option<&[f64]>, spec: &Spec) -> Option<f64> {
    use crate::geom::bulge::{bulge_ring_area, reverse_bulge_path};
    let all = lines(pts, bulges, spec).ok()?;
    let (left, right) = (&all[0], &all[all.len() - 1]);
    if spec.closed {
        return Some(
            (bulge_ring_area(&left.pts, Some(&left.bulges)).abs()
                - bulge_ring_area(&right.pts, Some(&right.bulges)).abs())
            .abs(),
        );
    }
    let back = reverse_bulge_path(&right.pts, Some(&right.bulges), false);
    let mut ring = left.pts.clone();
    let mut b = left.bulges.clone();
    // The left face's last vertex goes straight across the end to the right face's last.
    if let Some(x) = b.last_mut() {
        *x = 0.0;
    }
    ring.extend(back.pts);
    b.extend(back.bulges.unwrap_or_default());
    Some(bulge_ring_area(&ring, Some(&b)).abs())
}

/// The direction of the path's first edge, degrees counter-clockwise from east (its chord when it is an arc).
fn first_direction(pts: &[Vec2]) -> f64 {
    let (a, b) = (pts[0], pts[1]);
    atan2(b.y - a.y, b.x - a.x) * 180.0 / PI
}

fn hatch_of(fill: &ComponentFill, turn: f64, scale: f64) -> Option<HatchPattern> {
    component_hatch(&fill.pattern, fill.scale, turn + fill.angle, scale)
}

/// A component's hatch as its wall draws it (and the material legend's swatch, docs/adr/0721 §5): `pattern` of the
/// library at `angle` degrees, `size` times its paper size at the drawing's scale (1:`scale`); “solid” filled; none
/// for a pattern the library lacks.
pub fn component_hatch(pattern: &str, size: f64, angle: f64, scale: f64) -> Option<HatchPattern> {
    if pattern.eq_ignore_ascii_case("solid") {
        return Some(HatchPattern::user("solid", 0.0, 1.0));
    }
    let found = library_pattern(pattern)?;
    Some(HatchPattern {
        kind: "pattern".into(),
        angle,
        spacing: 1.0,
        name: Some(found.name.to_owned()),
        scale: Some(size * scale / 1000.0),
        lines: Some(found.lines.clone()),
        gradient: None,
        origin: None,
    })
}

fn piece(shape: Shape, color: Option<String>, edge: Option<&ComponentEdge>) -> Piece {
    let edge = edge.cloned().unwrap_or_default();
    Piece {
        shape,
        color: edge.color.or(color),
        line_weight: edge.line_weight,
        by_layer: Vec::new(),
        gates: Vec::new(),
        stroke: PieceStroke {
            line_type: edge.line_type,
            ..PieceStroke::default()
        },
        attribute: None,
    }
}

/// A piece of one of the wall's lines: an open stretch, or the whole line of a closed wall.
#[derive(Clone, Debug, PartialEq)]
pub struct Stroke {
    pub line: Line,
    pub closed: bool,
}

/// What a wall draws, whatever the detail (docs/adr/0340 §2): its lines as the joins left them, the lines across its
/// free ends, each component's band and the body between its faces, each in its pieces. A wall on its own is its
/// whole lines, caps at both ends of an open wall (unless they are open) and one band each; `geom::wall_join` cuts,
/// extends and interrupts them where walls meet.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Drawn {
    /// Each of the wall's lines (`boundaries`' order), its pieces.
    pub lines: Vec<Vec<Stroke>>,
    pub caps: Vec<(Vec2, Vec2)>,
    /// Each component's band, its pieces; none when the components do not fit.
    pub bands: Vec<Vec<Body>>,
    /// The body between the faces, its pieces.
    pub fill: Vec<Body>,
    /// A niche's back and a flue's sides inside the wall where none of its lines runs (docs/adr/0719 §3), drawn as its
    /// faces are.
    pub recesses: Vec<Stroke>,
    /// Where its turning layers close across a free end or a jamb (docs/adr/0771 §3): the component each belongs to
    /// (drawn when that component is) and the line.
    pub closures: Vec<(usize, Line)>,
}

/// A wall's drawing on its own, or why it cannot be drawn.
pub fn drawn(pts: &[Vec2], bulges: Option<&[f64]>, spec: &Spec) -> Result<Drawn, String> {
    let all = lines(pts, bulges, spec)?;
    // Layers that turn at its free ends (docs/adr/0771 §3) are the solver's: a wall on its own turns as in a drawing.
    if spec.components.iter().any(|c| c.wrap) {
        let one = [crate::geom::wall_join::WallIn { pts, bulges, spec }];
        if let Some(Some(d)) = crate::geom::wall_join::solve(&one, None).drawn.into_iter().next() {
            return Ok(d);
        }
    }
    Ok(drawn_of(&all, spec))
}

/// The drawing of a wall's own lines.
pub fn drawn_of(all: &[Line], spec: &Spec) -> Drawn {
    let closed = spec.closed;
    let (left, right) = (&all[0], &all[all.len() - 1]);
    let mut caps = Vec::new();
    if !closed && !spec.caps_open {
        caps.push((left.pts[0], right.pts[0]));
        caps.push((left.pts[left.pts.len() - 1], right.pts[right.pts.len() - 1]));
    }
    let bands = if all.len() == spec.components.len() + 1 {
        (0..spec.components.len())
            .map(|k| vec![band(&all[k], &all[k + 1], closed)])
            .collect()
    } else {
        Vec::new()
    };
    Drawn {
        lines: all
            .iter()
            .map(|l| {
                vec![Stroke {
                    line: l.clone(),
                    closed,
                }]
            })
            .collect(),
        caps,
        bands,
        fill: vec![band(left, right, closed)],
        recesses: Vec::new(),
        closures: Vec::new(),
    }
}

/// The wall's pieces at `view`, hatches and fill first, then its lines; or why it cannot be drawn.
pub fn pieces(
    pts: &[Vec2],
    bulges: Option<&[f64]>,
    spec: &Spec,
    view: View,
) -> Result<Vec<Piece>, String> {
    Ok(pieces_of(&drawn(pts, bulges, spec)?, pts, spec, view))
}

/// A drawing's pieces at `view`, hatches and fill first, then its lines.
pub fn pieces_of(d: &Drawn, pts: &[Vec2], spec: &Spec, view: View) -> Vec<Piece> {
    let mut out = Vec::new();
    let shown = |c: &Component| view.detail >= Detail::Standard && view.detail >= c.detail;
    let comps = if d.bands.len() == spec.components.len() {
        &spec.components[..]
    } else {
        &[]
    };
    let hatch = |ring: &Vec<Vec2>, holes: &Option<Vec<Vec<Vec2>>>, pattern: HatchPattern| Shape::Hatch {
        ring: ring.clone(),
        holes: holes.clone(),
        pattern,
        assoc: None,
        boundary: None,
    };
    if view.detail == Detail::Schematic {
        if let Some(fill) = &spec.fill {
            for (ring, holes) in &d.fill {
                out.push(piece(
                    hatch(ring, holes, HatchPattern::user("solid", 0.0, 1.0)),
                    Some(fill.clone()),
                    None,
                ));
            }
        }
    } else {
        let turn = first_direction(pts);
        for (k, c) in comps.iter().enumerate() {
            let Some(fill) = c.hatch.as_ref().filter(|_| shown(c)) else {
                continue;
            };
            let Some(pattern) = hatch_of(fill, turn, view.scale) else {
                continue;
            };
            for (ring, holes) in &d.bands[k] {
                out.push(piece(hatch(ring, holes, pattern.clone()), fill.color.clone(), None));
            }
        }
    }
    let last = d.lines.len() - 1;
    for k in [0, last] {
        for st in &d.lines[k] {
            out.push(piece(st.line.shape(st.closed), None, None));
        }
    }
    for &(a, b) in &d.caps {
        out.push(piece(Shape::Line { a, b }, None, None));
    }
    for st in &d.recesses {
        out.push(piece(st.line.shape(st.closed), None, None));
    }
    for (c, line) in &d.closures {
        if comps.get(*c).is_some_and(shown) {
            out.push(piece(line.shape(false), None, comps[*c].edge.as_ref()));
        }
    }
    for k in 1..comps.len() {
        if shown(&comps[k - 1]) && shown(&comps[k]) {
            let edge = comps[k - 1].edge.as_ref().or(comps[k].edge.as_ref());
            for st in &d.lines[k] {
                out.push(piece(st.line.shape(st.closed), None, edge));
            }
        }
    }
    out
}

/// The wall's base line as a polyline (what its grips, its stretch and its transforms act on).
pub fn path_shape(pts: &[Vec2], bulges: Option<&[f64]>, closed: bool) -> Shape {
    Shape::Polyline {
        pts: pts.to_vec(),
        bulges: bulges.map(<[f64]>::to_vec),
        holes: None,
        parts: None,
        closed,
        widths: None,
    }
}

/// A wall's base line, or none for another kind.
pub fn path_of(shape: &Shape) -> Option<Shape> {
    match shape {
        Shape::Wall { pts, bulges, wall } => Some(path_shape(pts, bulges.as_deref(), wall.closed)),
        _ => None,
    }
}

/// `wall` along another base line (a polyline, what an edit of its path gave), its other fields kept; none when
/// `path` is no polyline.
pub fn with_path(wall: &Shape, path: Shape) -> Option<Shape> {
    let (Shape::Wall { wall: spec, .. }, Shape::Polyline { pts, bulges, .. }) = (wall, path) else {
        return None;
    };
    let n = pts.len();
    let bulges = bulges.filter(|b| has_bulges(Some(b))).map(|mut b| {
        b.resize(n, 0.0);
        b
    });
    Some(Shape::Wall {
        pts,
        bulges,
        wall: spec.clone(),
    })
}

/// The spec with every length times `s` (`s` > 0): width, offset, thicknesses.
pub fn scaled(spec: &Spec, s: f64) -> Spec {
    let mut out = spec.clone();
    out.width *= s;
    out.offset *= s;
    for c in &mut out.components {
        if let Some(t) = &mut c.thickness {
            *t *= s;
        }
    }
    out
}

/// The wall's outline as edges: its two faces and, open, its caps; its path's when it cannot be drawn.
pub fn outline_edges(shape: &Shape) -> Vec<crate::geom::intersect::Edge> {
    use crate::geom::bulge::bulge_path_edges;
    use crate::geom::intersect::Edge;
    let Shape::Wall { pts, bulges, wall } = shape else {
        return Vec::new();
    };
    let Ok(all) = lines(pts, bulges.as_deref(), wall) else {
        return bulge_path_edges(pts, bulges.as_deref(), wall.closed);
    };
    let (left, right) = (&all[0], &all[all.len() - 1]);
    let mut out = bulge_path_edges(&left.pts, Some(&left.bulges), wall.closed);
    out.extend(bulge_path_edges(
        &right.pts,
        Some(&right.bulges),
        wall.closed,
    ));
    if !wall.closed && !wall.caps_open {
        out.push(Edge::Seg {
            a: left.pts[0],
            b: right.pts[0],
        });
        out.push(Edge::Seg {
            a: left.pts[left.pts.len() - 1],
            b: right.pts[right.pts.len() - 1],
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2::new(x, y)
    }

    fn spec(width: f64, justify: Justify) -> Spec {
        Spec {
            width,
            justify,
            ..Spec::default()
        }
    }

    fn layered() -> Spec {
        let c = |t: Option<f64>, detail: Detail, hatch: Option<&str>| Component {
            thickness: t,
            hatch: hatch.map(|p| ComponentFill {
                pattern: p.into(),
                scale: 1.0,
                angle: 0.0,
                color: None,
            }),
            detail,
            ..Component::default()
        };
        Spec {
            width: 0.25,
            components: vec![
                c(Some(0.02), Detail::Detailed, None),
                c(None, Detail::Standard, Some("ANSI31")),
                c(Some(0.05), Detail::Standard, Some("ANSI37")),
                c(Some(0.03), Detail::Detailed, None),
            ],
            fill: Some("#5A5A5A".into()),
            ..Spec::default()
        }
    }

    #[test]
    fn the_faces_lie_where_the_justification_puts_them() {
        assert_eq!(boundaries(&spec(0.2, Justify::Center)), [0.1, -0.1]);
        assert_eq!(boundaries(&spec(0.2, Justify::Left)), [0.0, -0.2]);
        assert_eq!(boundaries(&spec(0.2, Justify::Right)), [0.2, 0.0]);
        let mut s = spec(0.2, Justify::Left);
        s.offset = 0.05;
        assert_eq!(boundaries(&s), [0.05, 0.05 - 0.2]);
        let b = boundaries(&layered());
        assert_eq!(b.len(), 5);
        assert!((b[1] - 0.105).abs() < 1e-15 && (b[3] - -0.095).abs() < 1e-15);
    }

    #[test]
    fn a_straight_wall_is_its_faces_and_its_caps() {
        let p = [v(0.0, 0.0), v(4.0, 0.0)];
        let got = pieces(&p, None, &spec(0.2, Justify::Center), View::default()).unwrap();
        assert_eq!(got.len(), 4);
        let Shape::Polyline { pts, .. } = &got[0].shape else {
            panic!()
        };
        assert_eq!(pts, &[v(0.0, 0.1), v(4.0, 0.1)]);
        let Shape::Line { a, b } = &got[3].shape else {
            panic!()
        };
        assert_eq!((*a, *b), (v(4.0, 0.1), v(4.0, -0.1)));
    }

    #[test]
    fn a_corner_is_mitred_on_both_faces() {
        let p = [v(0.0, 0.0), v(4.0, 0.0), v(4.0, 3.0)];
        let ls = lines(&p, None, &spec(0.2, Justify::Center)).unwrap();
        assert_eq!(ls[0].pts[1], v(3.9, 0.1));
        assert_eq!(ls[1].pts[1], v(4.1, -0.1));
    }

    #[test]
    fn the_detail_decides_the_components_lines_and_hatches() {
        let p = [v(0.0, 0.0), v(5.0, 0.0)];
        let count = |detail| {
            let got = pieces(
                &p,
                None,
                &layered(),
                View {
                    detail,
                    scale: 50.0,
                },
            )
            .unwrap();
            let hatches = got
                .iter()
                .filter(|x| matches!(x.shape, Shape::Hatch { .. }))
                .count();
            (got.len(), hatches)
        };
        // Schematic: the fill, two faces, two caps.
        assert_eq!(count(Detail::Schematic), (5, 1));
        // Standard: brick and insulation hatched, the line between them, faces and caps.
        assert_eq!(count(Detail::Standard), (2 + 4 + 1, 2));
        // Detailed: the same hatches and all three inner lines.
        assert_eq!(count(Detail::Detailed), (2 + 4 + 3, 2));
    }

    #[test]
    fn a_closed_wall_has_no_caps_and_a_hole_in_its_bands() {
        let p = [v(0.0, 0.0), v(4.0, 0.0), v(4.0, 3.0), v(0.0, 3.0)];
        let s = Spec {
            closed: true,
            fill: Some("#000000".into()),
            ..spec(0.2, Justify::Center)
        };
        let got = pieces(
            &p,
            None,
            &s,
            View {
                detail: Detail::Schematic,
                scale: 100.0,
            },
        )
        .unwrap();
        assert_eq!(got.len(), 3);
        let Shape::Hatch { ring, holes, .. } = &got[0].shape else {
            panic!()
        };
        assert!(signed_area(ring).abs() > signed_area(&holes.as_ref().unwrap()[0]).abs());
    }

    #[test]
    fn the_body_s_area_is_exact() {
        let p = [v(0.0, 0.0), v(4.0, 0.0)];
        assert!((area(&p, None, &spec(0.2, Justify::Center)).unwrap() - 0.8).abs() < 1e-12);
        let ring = [v(0.0, 0.0), v(4.0, 0.0), v(4.0, 3.0), v(0.0, 3.0)];
        let s = Spec {
            closed: true,
            ..spec(0.2, Justify::Center)
        };
        // 4.2 × 3.2 less 3.8 × 2.8.
        assert!((area(&ring, None, &s).unwrap() - (13.44 - 10.64)).abs() < 1e-12);
        // A half ring of radius 5, 0.2 wide: half the annulus between 5.1 and 4.9.
        let arc = [v(-5.0, 0.0), v(5.0, 0.0)];
        let want = PI * (5.1 * 5.1 - 4.9 * 4.9) / 2.0;
        assert!(
            (area(&arc, Some(&[-1.0, 0.0]), &spec(0.2, Justify::Center)).unwrap() - want).abs()
                < 1e-12
        );
    }

    #[test]
    fn an_arc_too_tight_for_the_wall_is_refused() {
        // A half circle of radius 0.05 on the left: the left face would be inside its centre.
        let p = [v(0.0, 0.0), v(0.1, 0.0)];
        assert!(lines(&p, Some(&[1.0, 0.0]), &spec(0.2, Justify::Center)).is_err());
    }
}
