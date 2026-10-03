//! Ölçülendirme: the web's `DimensionTool` (`apps/web/src/tools/dimensionTool.ts`)
//! on its `PointInputTool` base, step for step, with the web's fixes of
//! 26 September (docs/adr/0061):
//!
//! - Hizalı: two points, then where the dimension line goes, or its signed
//!   distance typed (left of the measured direction positive);
//! - Doğrusal ΔY / ΔX: the same, the direction following where the line is
//!   placed unless locked (Yatay ΔY, Düşey ΔX; Yön back to the cursor);
//! - Açı: two straight edges (or, with Köşeden, the vertex and a point on
//!   each arm), then where the arc goes, or its radius typed (taken without
//!   its sign); the sector follows where the arc is placed;
//! - Yarıçap and Çap: a circle, an arc or a path's arc segment, then the
//!   direction; they take no typed number;
//! - Koordinat (docs/adr/0147 §7): a point, then its line's end, or its
//!   length typed (toward the cursor, as every point tool's); the axis
//!   follows the cursor (further up or down: its Y, across: its X) unless
//!   locked (Y koordinatı, X koordinatı; Eksen back to the cursor);
//! - Yay uzunluğu: an arc or a path's arc segment (not a circle), with Kısmi
//!   two points on it, then where the dimension arc goes, or its distance
//!   from the arc typed (out positive);
//! - Kırıklı yarıçap: a circle or an arc, the centre the line starts from,
//!   the point on the arc, then where the jog goes, or its distance typed;
//! - Semt and Eğim: two points, or with Kenardan a straight edge, then where
//!   the arrow goes, or its distance typed (left positive); Eğim takes each
//!   point's elevation from the vertex it snapped to, else asks for it.
//!
//! The style changes only while nothing is picked. The style, the lock and
//! Köşeden stay for as long as the app lives (`Memory`). The text is 2.5
//! paper mm high. Each dimension is written through `cad.entities.create`,
//! one undo step (“Ekle”). The layout and the placing are the shared core's.

use kentos_contracts::{DimensionStyle, Entity, EntityGeometry};
use kentos_domain::Document;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::dimension::{
    DimensionGeom, dimension_offset_at, layout_dimension, linear_angle_for, ordinate_axis_for,
    signed_offset,
};
use kentos_geometry_core::geom::intersect::{Edge, closest_on_edge, line_line};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::tools::editing::{
    PickedEdge, arc_length_ends, edge_arms, radial_dimension, vertex_arms,
};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::geometry::shape;

use kentos_geometry_core::store::snap::SnapHit;

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::elevation;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, DimensionMode as Mode, Flow, Memory, Pointer, Preview, Stroke, Tag, Tone, Tool,
};

/// The tool's id: its command is `tool.dimension`.
pub const ID: &str = "dimension";
/// The prompt's name (the web's `label`); the catalog's is “Ölçülendirme”.
pub const LABEL: &str = "Ölçü";

/// The text's height in paper millimetres.
const HEIGHT_MM: f64 = 2.5;

/// The styles with their keys, in the web's order (`MODE_KEYS`).
const MODES: [(Mode, &str); 10] = [
    (Mode::Aligned, "H"),
    (Mode::Linear, "D"),
    (Mode::Angular, "A"),
    (Mode::Radius, "R"),
    (Mode::Diameter, "Ç"),
    (Mode::Ordinate, "O"),
    (Mode::ArcLength, "U"),
    (Mode::Jogged, "I"),
    (Mode::Azimuth, "T"),
    (Mode::Slope, "E"),
];

impl Mode {
    /// The style's name (the web's `DIMENSION_STYLE_LABEL`).
    pub fn label(self) -> &'static str {
        match self {
            Mode::Aligned => "Hizalı",
            Mode::Linear => "Doğrusal",
            Mode::Angular => "Açı",
            Mode::Radius => "Yarıçap",
            Mode::Diameter => "Çap",
            Mode::Ordinate => "Koordinat",
            Mode::ArcLength => "Yay uzunluğu",
            Mode::Jogged => "Kırıklı yarıçap",
            Mode::Azimuth => "Semt",
            Mode::Slope => "Eğim",
        }
    }

    /// What the log says when one is added (the web's `ADDED`: a compound
    /// noun takes -sü, “Açı ölçüsü”).
    fn added(self) -> &'static str {
        match self {
            Mode::Aligned => "Hizalı ölçü eklendi",
            Mode::Linear => "Doğrusal ölçü eklendi",
            Mode::Angular => "Açı ölçüsü eklendi",
            Mode::Radius => "Yarıçap ölçüsü eklendi",
            Mode::Diameter => "Çap ölçüsü eklendi",
            Mode::Ordinate => "Koordinat ölçüsü eklendi",
            Mode::ArcLength => "Yay uzunluğu ölçüsü eklendi",
            Mode::Jogged => "Kırıklı yarıçap ölçüsü eklendi",
            Mode::Azimuth => "Semt ölçüsü eklendi",
            Mode::Slope => "Eğim ölçüsü eklendi",
        }
    }

    /// The style as the shared layout reads it; none for aligned.
    fn layout_style(self) -> Option<&'static str> {
        match self {
            Mode::Aligned => None,
            Mode::Linear => Some("linear"),
            Mode::Angular => Some("angular"),
            Mode::Radius => Some("radius"),
            Mode::Diameter => Some("diameter"),
            Mode::Ordinate => Some("ordinate"),
            Mode::ArcLength => Some("arcLength"),
            Mode::Jogged => Some("jogged"),
            Mode::Azimuth => Some("azimuth"),
            Mode::Slope => Some("slope"),
        }
    }

    /// The style as the command writes it; none for aligned, as the web leaves it out.
    fn contract(self) -> Option<DimensionStyle> {
        match self {
            Mode::Aligned => None,
            Mode::Linear => Some(DimensionStyle::Linear),
            Mode::Angular => Some(DimensionStyle::Angular),
            Mode::Radius => Some(DimensionStyle::Radius),
            Mode::Diameter => Some(DimensionStyle::Diameter),
            Mode::Ordinate => Some(DimensionStyle::Ordinate),
            Mode::ArcLength => Some(DimensionStyle::ArcLength),
            Mode::Jogged => Some(DimensionStyle::Jogged),
            Mode::Azimuth => Some(DimensionStyle::Azimuth),
            Mode::Slope => Some(DimensionStyle::Slope),
        }
    }

    /// Semt and Eğim: an arrow beside two points, or an edge.
    fn arrowed(self) -> bool {
        matches!(self, Mode::Azimuth | Mode::Slope)
    }

    /// Why a dimension of this style does not form where the cursor is.
    fn no_dimension(self) -> &'static str {
        match self {
            Mode::Ordinate => {
                "Çizginin ucu noktaya çok yakın; imleci noktadan eksene dik yönde uzaklaştırın."
            }
            Mode::ArcLength => {
                "Bu yerde ölçü oluşmuyor; ölçü yayı merkeze ulaşıyor ya da iki nokta aynı yerde."
            }
            Mode::Jogged => {
                "Gösterilen merkez, yarıçap boyunca yaydaki noktadan geride ve yarıçapa yakın olmalı; başka bir yer gösterin."
            }
            Mode::Azimuth | Mode::Slope => {
                "Kenarın iki ucu aynı nokta; ölçülecek kenar yok. Başka bir nokta gösterin."
            }
            _ => "Bu yerde ölçü oluşmuyor; ölçülen noktalar çakışıyor ya da yay yarıçapı sıfır.",
        }
    }
}

/// The arc of an arc, or the arc segment of a path nearest to `p`: Yay
/// uzunluğu's pick (a circle has no ends to measure between).
fn arc_edge_at(e: &Entity, p: Vec2) -> Option<Edge> {
    let mut best: Option<(Edge, f64)> = None;
    for edge in entity_edges(&shape(e)) {
        if !matches!(edge, Edge::Arc { .. }) || arc_length_ends(&edge, None).is_none() {
            continue;
        }
        let d = closest_on_edge(&edge, p).d;
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some((edge, d));
        }
    }
    best.map(|(e, _)| e)
}

/// The straight edge of `e` nearest to `p` (the web's `straightEdgeAt`).
fn straight_edge_at(e: &Entity, p: Vec2) -> Option<(Vec2, Vec2)> {
    let mut best: Option<((Vec2, Vec2), f64)> = None;
    for edge in entity_edges(&shape(e)) {
        let Edge::Seg { a, b } = edge else {
            continue;
        };
        if dist(a, b) < 1e-9 {
            continue;
        }
        let d = closest_on_edge(&edge, p).d;
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some(((a, b), d));
        }
    }
    best.map(|(s, _)| s)
}

/// The circle of a circle, an arc, or the arc segment of a path nearest to
/// `p` (the web's `circleAt`).
fn circle_at(e: &Entity, p: Vec2) -> Option<(Vec2, f64)> {
    match e {
        Entity::Circle(c) => return Some((Vec2::new(c.c.x, c.c.y), c.r)),
        Entity::Arc(a) => return Some((Vec2::new(a.c.x, a.c.y), a.r)),
        _ => {}
    }
    let mut best: Option<((Vec2, f64), f64)> = None;
    for edge in entity_edges(&shape(e)) {
        let Edge::Arc { c, r, .. } = edge else {
            continue;
        };
        let d = closest_on_edge(&edge, p).d;
        if best.is_none_or(|(_, bd)| d < bd) {
            best = Some(((c, r), d));
        }
    }
    best.map(|(c, _)| c)
}

/// An arc length's arc (or, with two points, its part between them) as strokes.
fn arc_strokes(arc: &Edge, between: Option<(Vec2, Vec2)>, width: f32, tone: Tone) -> Vec<Stroke> {
    let Some(ends) = arc_length_ends(arc, between) else {
        return Vec::new();
    };
    let angle = |p: Vec2| kentos_geometry_core::jsmath::atan2(p.y - ends.c.y, p.x - ends.c.x);
    let r = dist(ends.c, ends.a);
    Outline::of(
        &Shape::Arc {
            c: ends.c,
            r,
            a0: angle(ends.a),
            a1: angle(ends.b),
        },
        None,
        width,
        tone,
    )
    .strokes
}

/// The elevation of the point, or of the vertex of a line, a polyline or an
/// area, a snap stands on (docs/adr/0142): Eğim's, else it asks.
fn snapped_elevation(snap: Option<SnapHit>, p: Vec2, cx: &Context<'_>) -> Option<f64> {
    let hit = snap?;
    if !(0.0..=f64::from(u32::MAX)).contains(&hit.id) || dist(hit.point, p) > points::SAME {
        return None;
    }
    elevation::elevation_at(cx.doc.get(kentos_domain::Slot(hit.id as u32))?, hit.point)
}

/// Every object's edge can be picked, on any layer (the web's `pickEdge`).
fn any(_: &Entity, _: &Document) -> bool {
    true
}

/// The dimension tool.
#[derive(Clone, Debug, Default)]
pub struct Dimension {
    d: Taken,
    /// Açı by edges: the two picked edges and where they were clicked.
    edges: Vec<PickedEdge>,
    /// Yarıçap and Çap: the picked circle, its centre and radius.
    circle: Option<(Vec2, f64)>,
    /// Yay uzunluğu: the picked arc (docs/adr/0147 §7).
    arc: Option<Edge>,
    /// Eğim: each point's elevation, beside `d.pts`; none while it is asked for.
    zs: Vec<Option<f64>>,
    /// The snap under the pointer as it went down: where Eğim's elevation comes from.
    snap: Option<SnapHit>,
    /// Doğrusal's Açı (A): the next number is the measuring direction.
    asking_angle: bool,
    /// Açı from an arc: its vertex and ends; from a circle: its centre and
    /// the point clicked on it (docs/adr/0147 §7).
    angle_arc: Option<(Vec2, Vec2, Vec2)>,
    angle_circle: Option<(Vec2, Vec2)>,
    /// The project's units, as of the last call.
    format: Format,
    /// The text's height in metres, as of the last call.
    height: f64,
    /// What the session remembered, as of the last call.
    memory: Memory,
}

impl Dimension {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.height = HEIGHT_MM / 1000.0 * cx.doc.settings().plot_scale;
        self.memory = *cx.memory;
        self.format = cx.format();
    }

    fn mode(&self) -> Mode {
        self.memory.dimension_mode
    }

    /// Nothing picked yet: the style can still change.
    fn fresh(&self) -> bool {
        self.d.pts.is_empty()
            && self.edges.is_empty()
            && self.circle.is_none()
            && self.arc.is_none()
            && self.angle_arc.is_none()
            && self.angle_circle.is_none()
    }

    /// Stages where a click picks an edge or a circle rather than a point.
    fn picks_edge(&self) -> bool {
        match self.mode() {
            Mode::Angular => {
                !self.memory.dimension_by_vertex
                    && self.edges.len() < 2
                    && self.angle_arc.is_none()
                    && self.angle_circle.is_none()
            }
            Mode::Radius | Mode::Diameter | Mode::Jogged => self.circle.is_none(),
            Mode::ArcLength => self.arc.is_none(),
            Mode::Azimuth | Mode::Slope => {
                self.memory.dimension_by_edge && self.d.pts.is_empty()
            }
            Mode::Aligned | Mode::Linear | Mode::Ordinate => false,
        }
    }

    /// Eğim's point whose elevation is asked for (0 or 1), if any.
    fn asking(&self) -> Option<usize> {
        if self.mode() != Mode::Slope {
            return None;
        }
        self.zs.iter().position(Option::is_none)
    }

    /// Whether the next point places the dimension line (or arc, or leader).
    fn placing(&self) -> bool {
        match self.mode() {
            Mode::Angular if self.memory.dimension_by_vertex => self.d.pts.len() == 3,
            Mode::Angular => {
                self.edges.len() == 2
                    || self.angle_arc.is_some()
                    || (self.angle_circle.is_some() && self.d.pts.len() == 1)
            }
            Mode::Radius | Mode::Diameter => self.circle.is_some(),
            Mode::Aligned | Mode::Linear => self.d.pts.len() == 2,
            Mode::Ordinate => self.d.pts.len() == 1,
            Mode::ArcLength => {
                self.arc.is_some() && (!self.memory.arc_partial || self.d.pts.len() == 2)
            }
            Mode::Jogged => self.circle.is_some() && self.d.pts.len() == 2,
            Mode::Azimuth => self.d.pts.len() == 2,
            Mode::Slope => self.d.pts.len() == 2 && self.asking().is_none(),
        }
    }

    /// Letter options (the web's `option`): a style while nothing is
    /// picked, Köşeden for the angle, the linear lock once two points are in.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        if key == "Z" && !self.asking_angle {
            cx.memory.dimension_mask = !cx.memory.dimension_mask;
            return true;
        }
        if self.fresh() {
            if let Some(&(mode, _)) = MODES
                .iter()
                .find(|(_, k)| *k == key || (*k == "Ç" && key == "C"))
            {
                cx.memory.dimension_mode = mode;
                return true;
            }
            if key == "K" && cx.memory.dimension_mode == Mode::Angular {
                cx.memory.dimension_by_vertex = !cx.memory.dimension_by_vertex;
                return true;
            }
            if key == "K" && cx.memory.dimension_mode == Mode::ArcLength {
                cx.memory.arc_partial = !cx.memory.arc_partial;
                return true;
            }
            if key == "K" && cx.memory.dimension_mode.arrowed() {
                cx.memory.dimension_by_edge = !cx.memory.dimension_by_edge;
                return true;
            }
            // Kırıklı yarıçap's I, typed dotted on a Turkish keyboard's English layout.
            if key == "İ" {
                cx.memory.dimension_mode = Mode::Jogged;
                return true;
            }
        }
        if cx.memory.dimension_mode == Mode::Ordinate && self.d.pts.len() == 1 {
            cx.memory.ordinate_lock = match key {
                "Y" => Some(0.0),
                "X" => Some(90.0),
                "O" => None,
                _ => return false,
            };
            return true;
        }
        if cx.memory.dimension_mode == Mode::Linear && self.d.pts.len() == 2 && !self.asking_angle {
            cx.memory.dimension_lock = match key {
                "Y" => Some(0.0),
                "X" => Some(90.0),
                "O" => None,
                "A" => {
                    self.asking_angle = true;
                    return true;
                }
                _ => return false,
            };
            return true;
        }
        false
    }

    /// A point given (the web's `accept`, then `onPoint`).
    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let snap = self.snap.take();
        // Eğim waits for the elevation it asked for.
        if self.asking().is_some() {
            cx.say(
                Level::Warn,
                "Önce noktanın kotunu metre olarak yazın; geri almak için Ctrl+Z.",
            );
            return;
        }
        self.d.begin(p, cx);
        if !self.placing() {
            if self.d.last().is_none_or(|last| dist(last, p) > points::SAME) {
                // Kırıklı yarıçap's point on the arc must leave room for a jog from the centre shown.
                if self.mode() == Mode::Jogged && self.d.pts.len() == 1 {
                    let ok = self.jogged_at(self.d.pts[0], p, 0.0).is_some_and(|g| {
                        kentos_geometry_core::geom::dimension::dimension_fault(&g).is_none()
                    });
                    if !ok {
                        cx.say(Level::Warn, Mode::Jogged.no_dimension());
                        return;
                    }
                }
                self.d.pts.push(p);
                if self.mode() == Mode::Slope {
                    self.zs.push(snapped_elevation(snap, p, cx));
                }
            }
            return;
        }
        let g = self.geom_at(p, None);
        self.commit(g, cx);
    }

    /// The dimension placed at `loc` (the web's `geomAt`). A typed value
    /// replaces the signed distance of the dimension line, or the arc's radius.
    fn geom_at(&self, loc: Vec2, typed: Option<f64>) -> Option<DimensionGeom> {
        let mode = self.mode();
        let geom = |a, b, offset, angle, c| DimensionGeom {
            a,
            b,
            offset,
            height: self.height,
            style: mode.layout_style().map(str::to_owned),
            angle,
            c,
            za: None,
            zb: None,
        };
        match mode {
            Mode::Aligned => {
                let (&a, &b) = (self.d.pts.first()?, self.d.pts.get(1)?);
                let offset = typed.unwrap_or_else(|| signed_offset(a, b, loc));
                Some(geom(a, b, offset, None, None))
            }
            Mode::Linear => {
                let (&a, &b) = (self.d.pts.first()?, self.d.pts.get(1)?);
                let angle = self
                    .memory
                    .dimension_lock
                    .unwrap_or_else(|| linear_angle_for(a, b, loc));
                let g = geom(a, b, 0.0, Some(angle), None);
                let offset = typed.unwrap_or_else(|| dimension_offset_at(&g, loc));
                Some(DimensionGeom { offset, ..g })
            }
            Mode::Angular => {
                let arms = if let Some((c, a, b)) = self.angle_arc {
                    kentos_geometry_core::tools::editing::Arms { c, a, b }
                } else if let Some((c, p1)) = self.angle_circle {
                    vertex_arms(c, p1, *self.d.pts.first()?, loc)
                } else if self.memory.dimension_by_vertex {
                    let (&c, &p1, &p2) =
                        (self.d.pts.first()?, self.d.pts.get(1)?, self.d.pts.get(2)?);
                    vertex_arms(c, p1, p2, loc)
                } else {
                    edge_arms(self.edges.first()?, self.edges.get(1)?, loc)?
                };
                let offset = typed.map_or_else(|| dist(arms.c, loc), f64::abs);
                Some(geom(arms.a, arms.b, offset, None, Some(arms.c)))
            }
            Mode::Radius | Mode::Diameter => {
                let (c, r) = self.circle?;
                let radial = radial_dimension(c, r, loc);
                Some(geom(c, radial.b, radial.offset, None, None))
            }
            Mode::Ordinate => {
                let &a = self.d.pts.first()?;
                let angle = self
                    .memory
                    .ordinate_lock
                    .unwrap_or_else(|| ordinate_axis_for(a, loc));
                Some(geom(a, loc, 0.0, Some(angle), None))
            }
            Mode::ArcLength => {
                let between = match self.d.pts.as_slice() {
                    [p, q] => Some((*p, *q)),
                    _ => None,
                };
                let ends = arc_length_ends(self.arc.as_ref()?, between)?;
                let g = geom(ends.a, ends.b, 0.0, None, Some(ends.c));
                let offset = typed.unwrap_or_else(|| dimension_offset_at(&g, loc));
                Some(DimensionGeom { offset, ..g })
            }
            Mode::Jogged => {
                let (&shown, &on) = (self.d.pts.first()?, self.d.pts.get(1)?);
                let g = self.jogged_at(shown, on, 0.0)?;
                let offset = typed.unwrap_or_else(|| dimension_offset_at(&g, loc));
                Some(DimensionGeom { offset, ..g })
            }
            Mode::Azimuth | Mode::Slope => {
                let (&a, &b) = (self.d.pts.first()?, self.d.pts.get(1)?);
                let offset = typed.unwrap_or_else(|| signed_offset(a, b, loc));
                let mut g = geom(a, b, offset, None, None);
                if mode == Mode::Slope {
                    g.za = *self.zs.first()?;
                    g.zb = *self.zs.get(1)?;
                }
                Some(g)
            }
        }
    }

    /// Açı's first pick on an arc or a circle (docs/adr/0147 §7): Yaydan,
    /// the arc's own angle about its centre (its ends counter-clockwise);
    /// Daireden, from the point clicked, put on the circle, to a second
    /// point. False for a straight edge nearer the click.
    fn angle_from(&mut self, e: &Entity, at: Vec2) -> bool {
        let mut best: Option<(Edge, f64)> = None;
        for edge in entity_edges(&shape(e)) {
            if !matches!(edge, Edge::Arc { .. }) {
                continue;
            }
            let d = closest_on_edge(&edge, at).d;
            if best.as_ref().is_none_or(|(_, bd)| d < *bd) {
                best = Some((edge, d));
            }
        }
        let Some((edge, d)) = best else {
            return false;
        };
        if straight_edge_at(e, at)
            .is_some_and(|(a, b)| closest_on_edge(&Edge::Seg { a, b }, at).d < d)
        {
            return false;
        }
        if let Some(ends) = arc_length_ends(&edge, None) {
            self.angle_arc = Some((ends.c, ends.a, ends.b));
            return true;
        }
        let Edge::Arc { c, r, .. } = edge else {
            return false;
        };
        let l = dist(c, at);
        if l < 1e-9 {
            return false;
        }
        self.angle_circle = Some((
            c,
            Vec2::new(c.x + (at.x - c.x) / l * r, c.y + (at.y - c.y) / l * r),
        ));
        true
    }

    /// Kırıklı yarıçap from the centre shown to the circle's point toward
    /// `on`, its jog `offset` along the radius from the centre shown.
    fn jogged_at(&self, shown: Vec2, on: Vec2, offset: f64) -> Option<DimensionGeom> {
        let (c, r) = self.circle?;
        let l = dist(c, on);
        if l < 1e-9 {
            return None;
        }
        let b = Vec2::new(c.x + (on.x - c.x) / l * r, c.y + (on.y - c.y) / l * r);
        Some(DimensionGeom {
            a: c,
            b,
            offset,
            height: self.height,
            style: Some("jogged".to_owned()),
            angle: None,
            c: Some(shown),
            za: None,
            zb: None,
        })
    }

    /// Writes the dimension (the web's `commit`): a degenerate one is said and
    /// the tool waits; written or refused, the next one starts.
    fn commit(&mut self, g: Option<DimensionGeom>, cx: &mut Context<'_>) {
        let mode = self.mode();
        let Some((g, layout)) = g.and_then(|g| layout_dimension(&g).map(|l| (g, l))) else {
            cx.say(Level::Warn, mode.no_dimension());
            return;
        };
        let geometry = EntityGeometry::Dimension {
            a: wire(g.a),
            b: wire(g.b),
            offset: g.offset,
            height: g.height,
            text: None,
            style: mode.contract(),
            angle: g.angle,
            c: g.c.map(wire),
            mask: self.memory.dimension_mask,
            za: g.za,
            zb: g.zb,
        };
        if let Some(out) = points::write_objects(vec![geometry], None, cx) {
            // Zincir ölçü and Baz ölçü start from the newest straight one (docs/adr/0140).
            if matches!(mode, Mode::Aligned | Mode::Linear) {
                cx.memory.last_dimension = out
                    .created
                    .first()
                    .and_then(|uid| kentos_domain::Uuid::parse_str(uid).ok());
            }
            let value = cx.format().dimension(layout.prefix, layout.unit, layout.value);
            cx.say(Level::Success, format!("{}: {value}", mode.added()));
        }
        self.reset();
    }

    /// Back to the first pick (the web's `reset`): what was written is the
    /// drawing's to undo.
    fn reset(&mut self) {
        self.edges.clear();
        self.circle = None;
        self.arc = None;
        self.zs.clear();
        self.snap = None;
        self.asking_angle = false;
        self.angle_arc = None;
        self.angle_circle = None;
        self.d.reset();
    }
}

impl Tool for Dimension {
    /// A point computed by the point calculator, as if clicked (the web's `acceptPoint`).
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        true
    }
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    /// The web's `promptFor`: the step, then the tool's own options, then
    /// the other styles while nothing is picked.
    fn prompt(&self) -> Prompt {
        let n = self.d.pts.len();
        // Doğrusal's Açı (A): its measuring direction asked for.
        if self.mode() == Mode::Linear && n == 2 && self.asking_angle {
            return Prompt::new(
                LABEL,
                format!(
                    "ölçme doğrultusunu yazın ({}, doğudan saatin tersine)",
                    self.format.angle_unit_name()
                ),
            );
        }
        let step = match self.mode() {
            Mode::Linear => match n {
                0 => "doğrusal ölçünün (ΔY / ΔX) ilk noktasını belirtin",
                1 => "ikinci ölçü noktasını belirtin",
                _ => "ölçü çizgisinin yerini gösterin ya da mesafe yazın",
            },
            Mode::Angular if self.memory.dimension_by_vertex => match n {
                0 => "açının köşesini gösterin",
                1 => "birinci kolun üzerinde bir nokta gösterin",
                2 => "ikinci kolun üzerinde bir nokta gösterin",
                _ => "yayın yerini gösterin ya da yarıçap yazın",
            },
            Mode::Angular if self.placing() => "yayın yerini gösterin ya da yarıçap yazın",
            Mode::Angular if self.angle_circle.is_some() => "açının ikinci noktasını gösterin",
            Mode::Angular => match self.edges.len() {
                0 => "açı ölçüsü için bir kenara, yaya ya da daireye tıklayın",
                _ => "ikinci kenara tıklayın",
            },
            Mode::Radius | Mode::Diameter if self.circle.is_some() => {
                "ölçünün doğrultusunu gösterin; daireden dışarı çekince yazı dışarı alınır"
            }
            Mode::Radius => "yarıçapı ölçülecek daireye ya da yaya tıklayın",
            Mode::Diameter => "çapı ölçülecek daireye ya da yaya tıklayın",
            Mode::Ordinate => match n {
                0 => "koordinat ölçüsünün noktasını belirtin",
                _ => "çizginin ucunu gösterin ya da uzunluğunu yazın",
            },
            Mode::ArcLength if self.arc.is_none() => "yay uzunluğu ölçülecek yaya tıklayın",
            Mode::ArcLength if self.placing() => {
                "ölçü yayının yerini gösterin ya da uzaklık yazın"
            }
            Mode::ArcLength => match n {
                0 => "yayın üstünde ölçünün başlangıcını gösterin",
                _ => "yayın üstünde ölçünün sonunu gösterin",
            },
            Mode::Jogged if self.circle.is_none() => {
                "kırıklı yarıçapı ölçülecek daireye ya da yaya tıklayın"
            }
            Mode::Jogged => match n {
                0 => "çizginin başlayacağı merkezi gösterin",
                1 => "yaydaki noktayı gösterin",
                _ => "kırığın yerini gösterin ya da uzaklığını yazın",
            },
            Mode::Azimuth | Mode::Slope if self.picks_edge() => {
                if self.mode() == Mode::Slope {
                    "eğimi ölçülecek kenara tıklayın"
                } else {
                    "semti ölçülecek kenara tıklayın"
                }
            }
            Mode::Slope if self.asking() == Some(0) => "birinci noktanın kotunu yazın (m)",
            Mode::Slope if self.asking() == Some(1) => "ikinci noktanın kotunu yazın (m)",
            Mode::Azimuth | Mode::Slope => match n {
                0 if self.mode() == Mode::Slope => "eğim ölçüsünün birinci noktasını gösterin",
                0 => "semt ölçüsünün başlangıcını gösterin",
                1 if self.mode() == Mode::Slope => "ikinci noktayı gösterin",
                1 => "kenarın sonunu gösterin",
                _ => "okun yerini gösterin ya da uzaklık yazın",
            },
            Mode::Aligned => match n {
                0 => "hizalı ölçünün ilk noktasını belirtin",
                1 => "ikinci ölçü noktasını belirtin",
                _ => "ölçü çizgisinin yerini gösterin ya da mesafe yazın",
            },
        };
        // An elevation is typed in the project's unit (docs/adr/0165 §2).
        let step = step.replace("(m)", &format!("({})", self.format.length_unit_label()));
        let mut prompt = Prompt::new(LABEL, step);
        if self.mode() == Mode::Linear && n == 2 {
            let way = match self.memory.dimension_lock {
                Some(0.0) => "yatay".to_owned(),
                Some(90.0) => "düşey".to_owned(),
                Some(deg) => self.format.angle(deg * std::f64::consts::PI / 180.0),
                None => "imleçten".to_owned(),
            };
            prompt = prompt
                .option("Yatay ΔY", "Y")
                .option("Düşey ΔX", "X")
                .option_with("Yön", "O", way)
                .option("Açı", "A");
        }
        if self.mode() == Mode::Ordinate && n == 1 {
            let way = match self.memory.ordinate_lock {
                Some(0.0) => "Y",
                Some(_) => "X",
                None => "imleçten",
            };
            prompt = prompt
                .option("Y koordinatı", "Y")
                .option("X koordinatı", "X")
                .option_with("Eksen", "O", way);
        }
        if self.fresh() {
            if self.mode().arrowed() {
                let other = if self.memory.dimension_by_edge {
                    "Noktalardan"
                } else {
                    "Kenardan"
                };
                prompt = prompt.option(other, "K");
            }
            if self.mode() == Mode::ArcLength {
                let other = if self.memory.arc_partial {
                    "Bütün yay"
                } else {
                    "Kısmi"
                };
                prompt = prompt.option(other, "K");
            }
            if self.mode() == Mode::Angular {
                let other = if self.memory.dimension_by_vertex {
                    "Kenarlardan"
                } else {
                    "Köşeden"
                };
                prompt = prompt.option(other, "K");
            }
        }
        // Zemin while nothing is picked and while the dimension is placed (docs/adr/0147 §7).
        if self.fresh() || self.placing() {
            let on = if self.memory.dimension_mask {
                "açık"
            } else {
                "kapalı"
            };
            prompt = prompt.option_with("Zemin", "Z", on);
        }
        if self.fresh() {
            for (mode, key) in MODES {
                if mode != self.mode() {
                    prompt = prompt.option(mode.label(), key);
                }
            }
        }
        prompt
    }

    fn point_count(&self) -> usize {
        self.d.pts.len()
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    /// No snapping while an edge is picked or Eğim waits for an elevation.
    fn snaps(&self) -> bool {
        !self.picks_edge() && self.asking().is_none()
    }

    fn snap_from(&self) -> Option<Vec2> {
        self.d.last()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if self.picks_edge() {
            edge::hover(p, cx, any);
            self.d.hover = Some(p.raw);
            return;
        }
        self.d.hover = Some(self.d.constrain(p, cx));
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        if !self.picks_edge() {
            let point = self.d.constrain(p, cx);
            self.snap = p.snap;
            self.accept(point, cx);
            return;
        }
        let picked = edge::pick(p, cx, any).and_then(|slot| cx.doc.get(slot));
        // Semt and Eğim by an edge: its two ends, Eğim their elevations (docs/adr/0147 §7).
        if self.mode().arrowed() {
            let Some((e, (a, b))) = picked.and_then(|e| straight_edge_at(e, p.raw).map(|s| (e, s)))
            else {
                cx.say(
                    Level::Warn,
                    "Bir çizgiye ya da çoklu çizginin ya da alanın düz kenarına tıklayın; iki nokta göstermek için “Noktalardan” seçin.",
                );
                return;
            };
            if self.mode() == Mode::Slope {
                self.zs = vec![elevation::elevation_at(e, a), elevation::elevation_at(e, b)];
            }
            self.d.pts = vec![a, b];
            cx.selection.set_hover(None);
            return;
        }
        if self.mode() == Mode::ArcLength {
            let Some(arc) = picked.and_then(|e| arc_edge_at(e, p.raw)) else {
                cx.say(
                    Level::Warn,
                    "Bir yaya ya da çoklu çizginin ya da alanın yaylı kenarına tıklayın; tam daire için Yarıçap ya da Çap'ı kullanın.",
                );
                return;
            };
            self.arc = Some(arc);
            cx.selection.set_hover(None);
            return;
        }
        // Açı's first pick on an arc or a circle: Yaydan, the arc's own angle; Daireden, a second point to come.
        if self.mode() == Mode::Angular
            && self.edges.is_empty()
            && let Some(e) = picked
            && self.angle_from(e, p.raw)
        {
            cx.selection.set_hover(None);
            return;
        }
        if self.mode() == Mode::Angular {
            // The first pick also takes an arc or a circle (`angle_from`); the second is a straight edge only.
            let Some((a, b)) = picked.and_then(|e| straight_edge_at(e, p.raw)) else {
                cx.say(
                    Level::Warn,
                    if self.edges.is_empty() {
                        "Açı için düz bir kenara, yaya ya da daireye tıklayın; köşe noktasından ölçmek için “Köşeden” seçin."
                    } else {
                        "Açının kenarı olarak düz bir çizgiye tıklayın; köşe noktasından ölçmek için “Köşeden” seçin."
                    },
                );
                return;
            };
            if let Some(first) = self.edges.first()
                && line_line(first.a, first.b, a, b).is_none()
            {
                cx.say(Level::Warn, "Kenarlar paralel; aralarında açı yok.");
                return;
            }
            self.edges.push(PickedEdge { a, b, at: p.raw });
        } else {
            let Some(circle) = picked.and_then(|e| circle_at(e, p.raw)) else {
                cx.say(
                    Level::Warn,
                    "Bir daireye, yaya ya da çoklu çizginin yay parçasına tıklayın.",
                );
                return;
            };
            self.circle = Some(circle);
        }
        cx.selection.set_hover(None);
    }

    /// An option, a number placing the dimension, or a point. Yarıçap and Çap
    /// are placed by pointing only; a typed point does not pick an edge.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        let done = if self.asking_angle {
            // Doğrusal's measuring direction, in the project's angle unit.
            match points::plain_number(text) {
                Some(a) => {
                    // The web's `(rad * 180) / π`, not `to_degrees`: the stored angle is the same bits on both.
                    cx.memory.dimension_lock =
                        Some(cx.format().angle_from_typed(a) * 180.0 / std::f64::consts::PI);
                    self.asking_angle = false;
                    true
                }
                None => false,
            }
        } else if let Some(i) = self.asking() {
            // Eğim's elevation asked for: a number, metres.
            match points::plain_length(text, cx) {
                Some(z) => {
                    self.zs[i] = Some(z);
                    true
                }
                None => false,
            }
        } else if self.option(&upper_tr(js_trim(text)), cx) {
            true
        } else if let Some(n) = points::plain_length(text, cx).filter(|_| {
            self.placing() && !matches!(self.mode(), Mode::Radius | Mode::Diameter | Mode::Ordinate)
        }) {
            let loc = self
                .d
                .hover
                .or_else(|| self.d.pts.first().copied())
                .or_else(|| self.edges.first().map(|e| e.at))
                .unwrap_or(Vec2::new(0.0, 0.0));
            let g = self.geom_at(loc, Some(n));
            self.commit(g, cx);
            true
        } else if self.picks_edge() {
            false
        } else if let Some(p) = cx.typed_point(text, self.d.last(), self.d.hover) {
            self.accept(p, cx);
            true
        } else {
            false
        };
        self.see(cx);
        done
    }

    /// With something picked it starts over; with nothing, it leaves.
    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.fresh() {
            return Flow::Exit;
        }
        self.reset();
        Flow::Stay
    }

    /// Ctrl+Z, newest first (the web's `undoStep`): a picked circle or the
    /// last picked edge, then the points (the dimension starts over);
    /// otherwise the drawing's undo, which takes back a dimension just written.
    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        // Yay uzunluğu's points on the arc go before the arc, Kırıklı yarıçap's before its circle;
        // Eğim's with their elevations, an edge's two ends together (docs/adr/0147 §7).
        if (self.arc.is_some() || self.circle.is_some() || self.angle_circle.is_some())
            && !self.d.pts.is_empty()
        {
            self.d.pts.pop();
            return true;
        }
        if self.angle_arc.take().is_some() || self.angle_circle.take().is_some() {
            return true;
        }
        if self.mode().arrowed() && !self.d.pts.is_empty() {
            if self.memory.dimension_by_edge {
                self.d.pts.clear();
                self.zs.clear();
            } else {
                self.d.pts.pop();
                self.zs.truncate(self.d.pts.len());
            }
            return true;
        }
        if self.circle.take().is_some() || self.edges.pop().is_some() || self.arc.take().is_some() {
            return true;
        }
        self.d.undo_step(false, cx)
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut strokes = Vec::new();
        // The picked edges and circle in the snap colour, 2 px.
        for e in &self.edges {
            strokes.push(Stroke::solid(vec![e.a, e.b], false).width(2.0).tone(Tone::Snap));
        }
        if let Some((c, r)) = self.circle {
            strokes.extend(Outline::of(&Shape::Circle { c, r }, None, 2.0, Tone::Snap).strokes);
        }
        // Daireden: the arm to the point clicked on the circle, and to the cursor until the second point is given.
        if let Some((c, p1)) = self.angle_circle
            && !self.placing()
        {
            strokes.push(Stroke::solid(vec![c, p1], false).width(2.0).tone(Tone::Snap));
            if let Some(hover) = self.d.hover {
                strokes.push(Stroke::solid(vec![c, hover], false));
            }
        }
        // Yay uzunluğu's arc, and with Kısmi the part between its first point and the cursor.
        if let Some(arc) = &self.arc {
            strokes.extend(arc_strokes(arc, None, 2.0, Tone::Snap));
            if let (Some(&first), None, Some(hover)) =
                (self.d.pts.first(), self.d.pts.get(1), self.d.hover)
            {
                strokes.extend(arc_strokes(arc, Some((first, hover)), 3.0, Tone::Accent));
            }
        }
        let Some(hover) = self.d.hover else {
            return Preview {
                strokes,
                ..Preview::default()
            };
        };
        if self.placing() {
            // The dimension as it would be written, and its value by the cursor.
            let mut tag = None;
            if let Some(l) = self.geom_at(hover, None).and_then(|g| layout_dimension(&g)) {
                strokes.extend(l.lines.iter().map(|&[p, q]| Stroke::solid(vec![p, q], false)));
                tag = Some(Tag {
                    at: hover,
                    lines: vec![format.dimension(l.prefix, l.unit, l.value)],
                });
            }
            return Preview {
                strokes,
                tag,
                ..Preview::default()
            };
        }
        if self.picks_edge() {
            return Preview {
                strokes,
                ..Preview::default()
            };
        }
        // Eğim waiting for an elevation: its points, nothing on the cursor (no next point is asked for).
        if self.asking().is_some() {
            return Preview {
                strokes,
                path: self.d.pts.clone(),
                ..Preview::default()
            };
        }
        // Kırıklı yarıçap's point on the arc: the dimension as it would be, its jog at the centre shown.
        if self.mode() == Mode::Jogged
            && let (Some(&shown), None) = (self.d.pts.first(), self.d.pts.get(1))
        {
            let mut tag = None;
            if let Some(l) = self
                .jogged_at(shown, hover, 0.0)
                .and_then(|g| layout_dimension(&g))
            {
                strokes.extend(l.lines.iter().map(|&[p, q]| Stroke::solid(vec![p, q], false)));
                tag = Some(Tag {
                    at: hover,
                    lines: vec![format.dimension(l.prefix, l.unit, l.value)],
                });
            }
            return Preview {
                strokes,
                tag,
                ..Preview::default()
            };
        }
        // Kısmi's points on the arc: the part lit above, its length by the cursor (no straight line).
        if let Some(arc) = &self.arc {
            let tag = self
                .d
                .pts
                .first()
                .and_then(|&first| arc_length_ends(arc, Some((first, hover))))
                .map(|ends| {
                    let g = DimensionGeom {
                        a: ends.a,
                        b: ends.b,
                        offset: 0.0,
                        height: self.height,
                        style: Some("arcLength".into()),
                        angle: None,
                        c: Some(ends.c),
                        za: None,
                        zb: None,
                    };
                    let length = layout_dimension(&g).map_or(0.0, |l| l.value);
                    Tag {
                        at: hover,
                        lines: vec![format.length(length)],
                    }
                });
            return Preview {
                strokes,
                tag,
                ..Preview::default()
            };
        }
        // The points so far and the cursor, as every point tool draws them.
        Preview {
            strokes,
            ..points::chain_preview(&self.d, format)
        }
    }
}
