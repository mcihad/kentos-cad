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
//!   point's elevation from the vertex it snapped to, else asks for it. Semt
//!   is a survey's: a program without its options (KentOS Mimari,
//!   docs/adr/0802) neither offers nor takes it.
//!
//! The style changes only while nothing is picked. The style, the lock and
//! Köşeden stay for as long as the app lives (`Memory`). The text is 2.5
//! paper mm high. Each dimension is written through `cad.dimensions.create`
//! (docs/adr/0255), one undo step (“Ekle”): the picks are its input (the
//! points, the places they snapped to, the edges by where they were
//! clicked), its preview the command's own dimension (`measured_in`). The
//! layout and the placing are the shared core's.

use kentos_commands::{
    DimensionMeasure, DimensionsCreate, EdgeNear, Entity, MeasuredPoint, NewLook,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::dimension::{DimensionGeom, layout_dimension};
use kentos_geometry_core::geom::intersect::{Edge, closest_on_edge, line_line};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::tools::editing::{PickedEdge, arc_length_ends};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_native_application::dimension_create::{
    arc_edge_at, circle_at, measured_in, straight_edge_at,
};
use kentos_native_application::geometry::{core_look, shape};
use kentos_native_application::{ExecutionContext, codes, dimensions_create};

use kentos_commands::{EntityId, SubRef};
use kentos_domain::dimension_ties::contract_place;
use kentos_geometry_core::ops::subref::{place_at, place_in};
use kentos_geometry_core::store::snap::SnapHit;
use kentos_native_application::blocks::core_entity;

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::elevation;
use crate::format::Format;
use crate::log::Level;
use crate::points::{self, Taken, wire};
use crate::prompt::{Prompt, upper_tr};
use crate::styles;
use crate::tool::{
    Context, DimensionMode as Mode, Flow, Memory, Offers, OptionChoice, Pointer, Preview, Stroke,
    Tag, Tone, Tool, ToolOption,
};

/// The tool's id: its command is `tool.dimension`.
pub const ID: &str = "dimension";
/// The prompt's name (the web's `label`); the catalog's is “Ölçülendirme”.
pub const LABEL: &str = "Ölçü";

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

    /// Whether its points are measured points an associative dimension binds (docs/adr/0249 §3): the point methods.
    fn binds(self) -> bool {
        matches!(
            self,
            Mode::Aligned | Mode::Linear | Mode::Ordinate | Mode::Azimuth | Mode::Slope
        )
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

/// The place on an object a snap stands on, when it is one (docs/adr/0249 §3): the snap's kind and exact point name
/// one of the object's places (`ops::subref::place_at`); Yakın, Dik, Teğet and Çeyrek a place along an edge or a
/// curve, and a snap on an insert the place on the piece inside it (docs/adr/0263). An intersection or a typed point
/// is none.
pub(crate) fn snapped_place(snap: Option<SnapHit>, p: Vec2, cx: &Context<'_>) -> Option<SubRef> {
    let hit = snap?;
    if !(0.0..=f64::from(u32::MAX)).contains(&hit.id) || hit.point != p {
        return None;
    }
    let slot = kentos_domain::Slot(hit.id as u32);
    let e = cx.doc.get(slot)?;
    let (path, place) = if matches!(e, Entity::Insert(_)) {
        let blocks = kentos_domain::dimension_ties::core_blocks(cx.doc.blocks());
        place_in(&blocks, &core_entity(e), hit.kind, hit.point)?
    } else {
        (Vec::new(), place_at(&core_entity(e), hit.kind, hit.point)?)
    };
    Some(SubRef {
        uid: EntityId(*cx.doc.uid(slot)?.as_bytes()),
        place: contract_place(place),
        path,
        broken: None,
    })
}

/// A picked object's edge as the command takes it: its persistent id and where it was clicked.
fn near_of(slot: Slot, at: Vec2, cx: &Context<'_>) -> Option<EdgeNear> {
    Some(EdgeNear {
        uid: cx.doc.uid(slot)?.to_string(),
        near: wire(at),
    })
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
    /// The points taken on a place of an object (an end, a vertex, a centre, a middle, a point) by a snap, with the
    /// place: an associative dimension's points follow them (docs/adr/0249 §3).
    places: Vec<(Vec2, SubRef)>,
    /// Doğrusal's Açı (A): the next number is the measuring direction.
    asking_angle: bool,
    /// Açı from an arc: its vertex and ends; from a circle: its centre and
    /// the point clicked on it (docs/adr/0147 §7).
    angle_arc: Option<(Vec2, Vec2, Vec2)>,
    angle_circle: Option<(Vec2, Vec2)>,
    /// The picks as the command takes them: each picked edge's object and
    /// where it was clicked (Açı's edges, Yaydan's arc, the circle, the arc).
    edge_picks: Vec<EdgeNear>,
    angle_pick: Option<EdgeNear>,
    circle_pick: Option<EdgeNear>,
    arc_pick: Option<EdgeNear>,
    /// The dimension the command would write where the pointer is, as of the
    /// last event (the preview's).
    shown: Option<DimensionGeom>,
    /// The project's units, as of the last call.
    format: Format,
    /// The text's height in metres, as of the last call: the dimension
    /// style's, else 2.5 mm's (docs/adr/0183 §4).
    height: f64,
    /// The dimension style's look, as of the last call; none for Standart.
    look: kentos_commands::DimensionLook,
    /// The dimension styles as of the last call: a CAD project's Stil.
    styles: styles::Seen,
    /// Stil: a dimension style's name is being typed.
    asking_style: bool,
    /// What the session remembered, as of the last call.
    memory: Memory,
    /// The options the program offers, as of the last call: Semt is a survey's (docs/adr/0802).
    offers: Offers,
}

impl Dimension {
    pub fn new() -> Self {
        Self::default()
    }

    fn see(&mut self, cx: &Context<'_>) {
        let (look, height) =
            styles::dimension_look(cx, styles::standard_dimension_height(cx.doc.settings()));
        self.height = height;
        self.look = look;
        self.styles = styles::Seen::dimension(cx);
        self.memory = *cx.memory;
        self.format = cx.format();
        self.offers = cx.draft.offers;
    }

    /// Whether the program offers `mode`: Semt only where a survey's options are (docs/adr/0802).
    fn offered(offers: Offers, mode: Mode) -> bool {
        mode != Mode::Azimuth || offers.has(ToolOption::Bearing)
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
            Mode::Azimuth | Mode::Slope => self.memory.dimension_by_edge && self.d.pts.is_empty(),
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
        // Stil while Zemin is offered: a CAD project's dimension styles (docs/adr/0183 §4).
        if key == "S"
            && !self.asking_angle
            && styles::shown(cx.doc.settings())
            && (self.fresh() || self.placing())
        {
            self.asking_style = true;
            return true;
        }
        if self.fresh() {
            if let Some(&(mode, _)) = MODES
                .iter()
                .find(|(_, k)| *k == key || (*k == "Ç" && key == "C"))
                .filter(|(mode, _)| Self::offered(cx.draft.offers, *mode))
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
            if self
                .d
                .last()
                .is_none_or(|last| dist(last, p) > points::SAME)
            {
                // Kırıklı yarıçap's point on the arc must leave room for a jog from the centre shown.
                if self.mode() == Mode::Jogged && self.d.pts.len() == 1 {
                    let ok = self
                        .measure(Some(p))
                        .and_then(|m| {
                            measured_in(cx.doc, &self.input_for(cx, m, p, Some(0.0))).ok()
                        })
                        .is_some_and(|m| {
                            kentos_geometry_core::geom::dimension::dimension_fault(&m.geom)
                                .is_none()
                        });
                    if !ok {
                        cx.say(Level::Warn, Mode::Jogged.no_dimension());
                        return;
                    }
                }
                self.d.pts.push(p);
                if self.binds()
                    && let Some(r) = snapped_place(snap, p, cx)
                {
                    self.places.push((p, r));
                }
                if self.mode() == Mode::Slope {
                    self.zs.push(snapped_elevation(snap, p, cx));
                }
            }
            return;
        }
        self.commit(p, None, cx);
    }

    /// Whether the points taken bind to the places they snapped to: the point methods, and Açı by its vertex
    /// (docs/adr/0320 §1).
    fn binds(&self) -> bool {
        self.mode().binds() || (self.mode() == Mode::Angular && self.memory.dimension_by_vertex)
    }

    /// A measured point: the place it snapped to on the point methods (it
    /// follows it, docs/adr/0249 §3), else its coordinates.
    fn point(&self, p: Vec2) -> MeasuredPoint {
        let place = self
            .binds()
            .then(|| self.places.iter().rev().find(|(q, _)| *q == p))
            .flatten();
        match place {
            Some((_, r)) => MeasuredPoint::Place {
                uid: kentos_domain::Uuid::from_bytes(r.uid.0).to_string(),
                place: r.place,
                path: r.path.clone(),
            },
            None => MeasuredPoint::At { at: wire(p) },
        }
    }

    /// What the picks measure; `on`, Kırıklı yarıçap's point on the arc while
    /// it is not given yet (the pointer's). None while a pick is missing.
    fn measure(&self, on: Option<Vec2>) -> Option<DimensionMeasure> {
        let pts = &self.d.pts;
        let pt = |i: usize| pts.get(i).map(|&p| self.point(p));
        let at = |i: usize| pts.get(i).map(|&p| MeasuredPoint::At { at: wire(p) });
        Some(match self.mode() {
            Mode::Aligned => DimensionMeasure::Aligned {
                a: pt(0)?,
                b: pt(1)?,
            },
            Mode::Linear => DimensionMeasure::Linear {
                a: pt(0)?,
                b: pt(1)?,
                angle: self.memory.dimension_lock,
            },
            Mode::Ordinate => DimensionMeasure::Ordinate {
                a: pt(0)?,
                angle: self.memory.ordinate_lock,
            },
            Mode::Azimuth => DimensionMeasure::Azimuth {
                a: pt(0)?,
                b: pt(1)?,
            },
            Mode::Slope => DimensionMeasure::Slope {
                a: pt(0)?,
                b: pt(1)?,
                za: *self.zs.first()?,
                zb: *self.zs.get(1)?,
            },
            Mode::Angular => {
                if self.angle_arc.is_some() {
                    DimensionMeasure::AngleArc {
                        of: self.angle_pick.clone()?,
                    }
                } else if let Some((c, p1)) = self.angle_circle {
                    DimensionMeasure::AngleVertex {
                        c: MeasuredPoint::At { at: wire(c) },
                        a: MeasuredPoint::At { at: wire(p1) },
                        b: at(0)?,
                    }
                } else if self.memory.dimension_by_vertex {
                    DimensionMeasure::AngleVertex {
                        c: pt(0)?,
                        a: pt(1)?,
                        b: pt(2)?,
                    }
                } else {
                    DimensionMeasure::Angle {
                        first: self.edge_picks.first()?.clone(),
                        second: self.edge_picks.get(1)?.clone(),
                    }
                }
            }
            Mode::Radius => DimensionMeasure::Radius {
                of: self.circle_pick.clone()?,
            },
            Mode::Diameter => DimensionMeasure::Diameter {
                of: self.circle_pick.clone()?,
            },
            Mode::ArcLength => DimensionMeasure::ArcLength {
                of: self.arc_pick.clone()?,
                between: match pts.as_slice() {
                    [p, q] => Some([wire(*p), wire(*q)]),
                    _ => None,
                },
            },
            Mode::Jogged => DimensionMeasure::Jogged {
                of: self.circle_pick.clone()?,
                shown_centre: wire(*pts.first()?),
                on: wire(pts.get(1).copied().or(on)?),
            },
        })
    }

    /// The command's input for `measure` placed at `at` (a typed value
    /// instead of `at`'s distance): the active layer, the chosen style in a
    /// CAD project, Zemin, the current colour and weight, an object
    /// template's attributes, label and symbol.
    fn input_for(
        &self,
        cx: &Context<'_>,
        measure: DimensionMeasure,
        at: Vec2,
        typed: Option<f64>,
    ) -> DimensionsCreate {
        let settings = cx.doc.settings();
        // Doğrusal's level or plumb is the UCS's (docs/adr/0261 §4): turned with its axes, unless given.
        let frame = cx.frame();
        let measure = match measure {
            DimensionMeasure::Linear { a, b, angle: None }
                if frame.angle != 0.0 && self.d.pts.len() >= 2 =>
            {
                let f = |p: Vec2| frame.to_frame(p);
                let along = kentos_geometry_core::geom::dimension::linear_angle_for(
                    f(self.d.pts[0]),
                    f(self.d.pts[1]),
                    f(at),
                );
                DimensionMeasure::Linear {
                    a,
                    b,
                    angle: Some(along + (frame.angle * 180.0) / kentos_geometry_core::jsmath::PI),
                }
            }
            other => other,
        };
        DimensionsCreate {
            measure,
            at: wire(at),
            offset: typed,
            layer_id: cx.doc.layers().active().to_owned(),
            dimension_style: styles::dimension_style(cx.memory, settings)
                .filter(|_| styles::shown(settings))
                .map(|s| s.name.clone()),
            height: None,
            mask: cx.memory.dimension_mask.then_some(true),
            look: NewLook {
                color: cx.draft.color_text(),
                line_weight: cx.draft.line_weight,
                attrs: cx.template_attrs(None),
                label: cx.template_label(),
                symbol: cx.template_symbol(),
            },
            expected_revision: None,
        }
    }

    /// The dimension's input placed at `at`; none while a pick is missing.
    fn request(&self, cx: &Context<'_>, at: Vec2, typed: Option<f64>) -> Option<DimensionsCreate> {
        Some(self.input_for(cx, self.measure(None)?, at, typed))
    }

    /// The dimension the preview shows: the command's where the pointer is
    /// while it is placed; Kırıklı yarıçap's with its jog at the centre shown
    /// while its point on the arc is pointed at.
    fn refresh(&mut self, cx: &Context<'_>) {
        self.shown = None;
        let Some(hover) = self.d.hover else {
            return;
        };
        let input = if self.placing() {
            self.request(cx, hover, None)
        } else if self.mode() == Mode::Jogged && self.circle.is_some() && self.d.pts.len() == 1 {
            self.measure(Some(hover))
                .map(|m| self.input_for(cx, m, hover, Some(0.0)))
        } else {
            None
        };
        self.shown = input
            .and_then(|i| measured_in(cx.doc, &i).ok())
            .map(|m| m.geom);
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

    /// Writes the dimension placed at `at` (a typed value instead of its
    /// distance) through `cad.dimensions.create` (the web's `commit`): one
    /// that does not form is said and the tool waits; written or refused,
    /// the next one starts.
    fn commit(&mut self, at: Vec2, typed: Option<f64>, cx: &mut Context<'_>) {
        let mode = self.mode();
        let Some(input) = self.request(cx, at, typed) else {
            cx.say(Level::Warn, mode.no_dimension());
            return;
        };
        let layout = match measured_in(cx.doc, &input) {
            Ok(m) => layout_dimension(&m.geom),
            Err(e) if e.code == codes::NO_DIMENSION => {
                cx.say(Level::Warn, e.message);
                return;
            }
            // The command says why when it runs.
            Err(_) => None,
        };
        let result = dimensions_create::execute(&mut ExecutionContext::new(cx.doc), input);
        if let Some(out) = points::written(result, cx) {
            // Zincir ölçü and Baz ölçü start from the newest straight one (docs/adr/0140).
            if matches!(mode, Mode::Aligned | Mode::Linear) {
                cx.memory.last_dimension = out
                    .created
                    .first()
                    .and_then(|uid| kentos_domain::Uuid::parse_str(uid).ok());
            }
            if let Some(layout) = layout {
                let value =
                    cx.format()
                        .dimension_in(layout.prefix, layout.unit, layout.value, &self.look);
                cx.say(Level::Success, format!("{}: {value}", mode.added()));
            }
        }
        self.reset();
    }

    /// A click (the web's `onPointerDown`): a point, or an edge or a circle picked.
    fn down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if !self.picks_edge() {
            let point = self.d.constrain(p, cx);
            self.snap = p.snap;
            self.accept(point, cx);
            return;
        }
        let slot = edge::pick(p, cx, any);
        let picked = slot.and_then(|slot| cx.doc.get(slot));
        // The pick as the command takes it: the object and where it was clicked.
        let near = slot.and_then(|slot| near_of(slot, p.raw, cx));
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
            self.arc_pick = near;
            cx.selection.set_hover(None);
            return;
        }
        // Açı's first pick on an arc or a circle: Yaydan, the arc's own angle; Daireden, a second point to come.
        if self.mode() == Mode::Angular
            && self.edges.is_empty()
            && let Some(e) = picked
            && self.angle_from(e, p.raw)
        {
            if self.angle_arc.is_some() {
                self.angle_pick = near;
            }
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
            let Some(near) = near else {
                return;
            };
            self.edges.push(PickedEdge { a, b, at: p.raw });
            self.edge_picks.push(near);
        } else {
            let Some(circle) = picked.and_then(|e| circle_at(e, p.raw)) else {
                cx.say(
                    Level::Warn,
                    "Bir daireye, yaya ya da çoklu çizginin yay parçasına tıklayın.",
                );
                return;
            };
            self.circle = Some(circle);
            self.circle_pick = near;
        }
        cx.selection.set_hover(None);
    }

    /// Ctrl+Z's one step back (see `undo_step`).
    fn undo_once(&mut self, cx: &mut Context<'_>) -> bool {
        // Yay uzunluğu's points on the arc go before the arc, Kırıklı yarıçap's before its circle;
        // Eğim's with their elevations, an edge's two ends together (docs/adr/0147 §7).
        if (self.arc.is_some() || self.circle.is_some() || self.angle_circle.is_some())
            && !self.d.pts.is_empty()
        {
            self.d.pts.pop();
            return true;
        }
        if self.angle_arc.take().is_some() || self.angle_circle.take().is_some() {
            self.angle_pick = None;
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
        if self.circle.take().is_some() {
            self.circle_pick = None;
            true
        } else if self.edges.pop().is_some() {
            self.edge_picks.pop();
            true
        } else if self.arc.take().is_some() {
            self.arc_pick = None;
            true
        } else {
            self.d.undo_step(false, cx)
        }
    }

    /// Back to the first pick (the web's `reset`): what was written is the
    /// drawing's to undo.
    fn reset(&mut self) {
        self.edges.clear();
        self.circle = None;
        self.arc = None;
        self.zs.clear();
        self.snap = None;
        self.places.clear();
        self.asking_angle = false;
        self.asking_style = false;
        self.angle_arc = None;
        self.angle_circle = None;
        self.edge_picks.clear();
        self.angle_pick = None;
        self.circle_pick = None;
        self.arc_pick = None;
        self.shown = None;
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
        // Stil (S): a dimension style's name asked for (docs/adr/0183 §4).
        if self.asking_style {
            return Prompt::new(LABEL, "ölçü stilini menüden seçin ya da adını yazın").option_with(
                "Stil",
                "S",
                self.styles.chosen.clone(),
            );
        }
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
            Mode::ArcLength if self.placing() => "ölçü yayının yerini gösterin ya da uzaklık yazın",
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
        // Stil and Zemin while nothing is picked and while the dimension is placed (docs/adr/0147 §7,
        // 0183 §4).
        if (self.fresh() || self.placing()) && self.styles.shown {
            prompt = prompt.option_with("Stil", "S", self.styles.chosen.clone());
        }
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
                if mode != self.mode() && Self::offered(self.offers, mode) {
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
        // A style the program does not offer is not kept for it (docs/adr/0802).
        if !Self::offered(cx.draft.offers, cx.memory.dimension_mode) {
            cx.memory.dimension_mode = Mode::Aligned;
        }
        self.see(cx);
        self.refresh(cx);
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
        } else {
            self.d.hover = Some(self.d.constrain(p, cx));
        }
        self.refresh(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.down(p, cx);
        self.refresh(cx);
    }
    /// An option, a number placing the dimension, or a point. Yarıçap and Çap
    /// are placed by pointing only; a typed point does not pick an edge.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        let done = if self.asking_style {
            // A name the project has none of is said; the tool waits for another.
            if styles::take_dimension(js_trim(text), cx) {
                self.asking_style = false;
            }
            true
        } else if self.asking_angle {
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
            self.commit(loc, Some(n), cx);
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
        self.refresh(cx);
        done
    }

    /// Stil's menu: Standart, the project's dimension styles and their window (docs/adr/0183 §4).
    fn option_choices(&self, key: &str) -> Vec<OptionChoice> {
        if key == "S"
            && self.styles.shown
            && (self.asking_style || ((self.fresh() || self.placing()) && !self.asking_angle))
        {
            return self
                .styles
                .choices(styles::DIMENSION_STYLES_ENTRY, styles::DIMENSION_STYLES);
        }
        Vec::new()
    }

    fn choose_option(&mut self, key: &str, typed: &str, cx: &mut Context<'_>) -> bool {
        let open = self.asking_style || ((self.fresh() || self.placing()) && !self.asking_angle);
        if key != "S" || !open || !styles::shown(cx.doc.settings()) {
            return false;
        }
        if styles::take_dimension(typed, cx) {
            self.asking_style = false;
        }
        self.see(cx);
        self.refresh(cx);
        true
    }

    /// With something picked it starts over; with nothing, it leaves.
    /// A style's name is words: Space types a space (docs/adr/0183 §4).
    fn takes_words(&self) -> bool {
        self.asking_style
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        if self.asking_style {
            self.asking_style = false;
            return Flow::Stay;
        }
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
        let undone = self.undo_once(cx);
        self.refresh(cx);
        undone
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut strokes = Vec::new();
        // The picked edges and circle in the snap colour, 2 px.
        for e in &self.edges {
            strokes.push(
                Stroke::solid(vec![e.a, e.b], false)
                    .width(2.0)
                    .tone(Tone::Snap),
            );
        }
        if let Some((c, r)) = self.circle {
            strokes.extend(Outline::of(&Shape::Circle { c, r }, None, 2.0, Tone::Snap).strokes);
        }
        // Daireden: the arm to the point clicked on the circle, and to the cursor until the second point is given.
        if let Some((c, p1)) = self.angle_circle
            && !self.placing()
        {
            strokes.push(
                Stroke::solid(vec![c, p1], false)
                    .width(2.0)
                    .tone(Tone::Snap),
            );
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
            if let Some(l) = self.shown.as_ref().and_then(layout_dimension) {
                strokes.extend(
                    l.lines
                        .iter()
                        .map(|&[p, q]| Stroke::solid(vec![p, q], false)),
                );
                // A style's filled arrowheads and dots, outlined (docs/adr/0183 §3).
                strokes.extend(
                    l.fills
                        .iter()
                        .flatten()
                        .map(|ring| Stroke::solid(ring.clone(), true)),
                );
                tag = Some(Tag {
                    at: hover,
                    lines: vec![format.dimension_in(l.prefix, l.unit, l.value, &self.look)],
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
        if self.mode() == Mode::Jogged && self.d.pts.len() == 1 {
            let mut tag = None;
            if let Some(l) = self.shown.as_ref().and_then(layout_dimension) {
                strokes.extend(
                    l.lines
                        .iter()
                        .map(|&[p, q]| Stroke::solid(vec![p, q], false)),
                );
                strokes.extend(
                    l.fills
                        .iter()
                        .flatten()
                        .map(|ring| Stroke::solid(ring.clone(), true)),
                );
                tag = Some(Tag {
                    at: hover,
                    lines: vec![format.dimension_in(l.prefix, l.unit, l.value, &self.look)],
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
                        look: core_look(&self.look),
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
