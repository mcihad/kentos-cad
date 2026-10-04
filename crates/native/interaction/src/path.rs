//! Kapalı alan, Çoklu çizgi, Mesafe ölç, Alan hesapla, Parsel oluştur and
//! Bitişik alan: the web's `PathTool` (`apps/web/src/tools/pathTool.ts`) with
//! its `closed`, `measureOnly` and `parcelLayer` flags and its `AdjoinTool`
//! (`adjoinTool.ts`), on its `PointInputTool` base (`drawTools.ts`), step for
//! step. One tool, six shapes (docs/adr/0021, 0027, 0067, 0162):
//!
//! - points come from clicks (ortho and polar tracking applied) and from
//!   typed text (the shared grammar, `point_text`);
//! - Y turns the next segments into arcs (continuing tangentially, or shaped
//!   once by Açı, Merkez, Yarıçap, İkinci nokta, Doğrultu), D back to lines;
//!   in line mode U continues the last direction by a typed length; G takes
//!   the last point back;
//! - closed: giving the first corner again closes and finishes the area:
//!   typed exactly, or clicked within the snap aperture once there are three
//!   corners; in arc mode the closing edge is the arc being drawn (ADR 0018);
//! - a confirm with enough points (3 closed, 2 open) writes one object in
//!   one undo step through a product command, as the web's tool does: a
//!   closed area through `cad.polygon.create` (docs/adr/0022), a polyline
//!   through `cad.polyline.create` (docs/adr/0027), a parcel through
//!   `cad.entities.create` on the parcel layer, numbered; the measuring
//!   shapes write nothing and say the length, or the area and perimeter;
//!   with fewer points it warns, writes nothing and starts over.
//!
//! Bitişik alan (docs/adr/0162 §3) draws an open path whose ends lie in or on
//! the neighbouring areas; the region it closes with them is written, filled
//! in the preview as the cursor goes (`adjoin`). With no region, Enter says
//! why and the path stays.
//!
//! İzle (İ, kept for the session; docs/adr/0161 §1) has the next segments
//! of line mode follow the visible line work: an unsnapped pointer within the
//! snap aperture of a line goes onto it, and a segment between two points on
//! connected line work runs along it the shortest way, the line work's own
//! corners and arcs added (the shared core's `ops::trace`); closing on the
//! first corner goes back along it too. Not for Sabit ilk nokta's rays nor
//! İçine tıkla.
//!
//! Akış (A, kept for the session; docs/adr/0161 §3): from the first point on,
//! the pointer leaves a vertex each time it is a step (Adım boyu, B; 1 m at
//! first) from the last; snaps, ortho and tracking do not apply to them.
//!
//! Two options of the measuring shapes are docs/adr/0141's:
//!
//! - Mesafe ölç's Sabit ilk nokta (S) measures every new point from the first
//!   (Netcad's cetvel, rays instead of a chain): the tag and the log say
//!   the distance and semt of each ray, and no total is said. It is offered
//!   before the first point and while it is on; Yay and Uzunluk do not apply.
//! - Alan hesapla's İçine tıkla (I) measures the region a click is inside, as
//!   İçine tıklayarak alan finds it: the visible line work's face, closed
//!   groups inside it as holes (the area is net of them; the perimeter counts
//!   them, as a polygon's). After a measurement Alan olarak çiz (A) writes
//!   the ring last measured (holes included) to the active layer through
//!   `cad.polygon.create`, one undo step named “Alan olarak çiz”.
//!
//! The web's messages are kept word for word. Every calculation is the
//! shared core's (`kentos-geometry-core`); none is written here.

use std::collections::BTreeMap;

use kentos_contracts::{EntitiesCreate, NewObject, PolygonCreate, PolylineCreate, RingGeometry};
use kentos_domain::{Slot, labels};
use kentos_geometry_core::entity::polygon_ring;
use kentos_geometry_core::geom::arc::DEFAULT_STEP;
use kentos_geometry_core::geom::arrangement::{Area as Region, Ring};
use kentos_geometry_core::geom::bulge::{
    bulge_arc, bulge_of_sweep, bulge_path_length, bulge_path_outline, bulge_ring_area,
    bulge_through, has_bulges, segment_tangent, tangent_bulge,
};
use kentos_geometry_core::geom::region::net_area;
use kentos_geometry_core::geometry::{bearing_grad, dist};
use kentos_geometry_core::jsmath::{PI, js_hypot};
use kentos_geometry_core::ops::trace::Traced;
use kentos_geometry_core::tools::drawing::{
    centre_bulge, offset_along, radial_point, radius_bulge, unit_toward,
};
use kentos_geometry_core::tools::locks::square_corner;
use kentos_geometry_core::tools::point_input::Tracking;
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::{ExecutionContext, create, polygon, polyline};

use crate::Vec2;
use crate::adjoin;
use crate::faces;
use crate::format::Format;
use crate::junctions;
use crate::log::Level;
use crate::overlap;
use crate::points::{self, SAME, wire, wire_all};
use crate::prompt::{Prompt, upper_tr};
use crate::second::Second;
use crate::tool::{
    Area, Context, Flow, Label, Marker, MarkerShape, Memory, Pointer, Preview, Stroke, Tag, Tone,
    Tool,
};
use crate::trace_work::WorkCache;

/// The closed-area tool's id: its command is `tool.polygon`.
pub const POLYGON_ID: &str = "polygon";
pub const POLYGON_LABEL: &str = "Kapalı alan";
/// The polyline tool's id: its command is `tool.polyline`.
pub const POLYLINE_ID: &str = "polyline";
pub const POLYLINE_LABEL: &str = "Çoklu çizgi";
/// Mesafe ölç: `tool.measure`.
pub const MEASURE_ID: &str = "measure";
pub const MEASURE_LABEL: &str = "Mesafe ölç";
/// Alan hesapla: `tool.area`.
pub const AREA_ID: &str = "area";
pub const AREA_LABEL: &str = "Alan hesapla";
/// Parsel oluştur: `tool.parcel`. Its prompt names it “Parsel”, as the web's tool.
pub const PARCEL_ID: &str = "parcel";
pub const PARCEL_LABEL: &str = "Parsel";
/// The layer parcels go on and are numbered by (the web's `LAYERS.parcel`, the project template's).
pub const PARCEL_LAYER: &str = "parsel";
/// Alan olarak çiz's undo step (docs/adr/0141).
pub const DRAW_AREA_LABEL: &str = "Alan olarak çiz";

/// Dik kapat with the first and the last edge parallel (docs/adr/0166 §4).
pub const NO_SQUARE_CLOSE: &str = "Dik kapatılamıyor: ilk kenar ile son kenar paralel.";

/// What the tool draws.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shape {
    /// A closed area (kapalı alan): at least 3 corners, closed on its first.
    Closed,
    /// An open polyline (çoklu çizgi): at least 2 points.
    Open,
    /// Mesafe ölç: an open path measured, nothing written.
    MeasureLength,
    /// Alan hesapla: a closed ring measured, nothing written.
    MeasureArea,
    /// Parsel oluştur: a closed area on the parcel layer, numbered.
    Parcel,
    /// Bitişik alan: an open path; the region it closes with the neighbouring areas is written.
    Adjoin,
}

impl Shape {
    fn id(self) -> &'static str {
        match self {
            Shape::Closed => POLYGON_ID,
            Shape::Open => POLYLINE_ID,
            Shape::MeasureLength => MEASURE_ID,
            Shape::MeasureArea => AREA_ID,
            Shape::Parcel => PARCEL_ID,
            Shape::Adjoin => adjoin::ID,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Shape::Closed => POLYGON_LABEL,
            Shape::Open => POLYLINE_LABEL,
            Shape::MeasureLength => MEASURE_LABEL,
            Shape::MeasureArea => AREA_LABEL,
            Shape::Parcel => PARCEL_LABEL,
            Shape::Adjoin => adjoin::LABEL,
        }
    }

    /// Whether it is a ring: closed on its first corner, with an area.
    fn closed(self) -> bool {
        matches!(self, Shape::Closed | Shape::MeasureArea | Shape::Parcel)
    }

    /// Points the shape needs.
    fn min(self) -> usize {
        if self.closed() { 3 } else { 2 }
    }
}

/// How the next arc segment is shaped (AutoCAD's PLINE arc options). The
/// default follows the path's end tangent; the others last one segment.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Spec {
    Tangent,
    /// The included angle in radians (typed in degrees), then the end point.
    Angle {
        sweep: Option<f64>,
    },
    /// The radius (typed), then the end point: the short arc, bending the way the path turns.
    Radius {
        r: Option<f64>,
    },
    /// The centre, then a point on the ray to the end (counter-clockwise).
    Centre {
        c: Option<Vec2>,
    },
    /// A point on the arc, then the end point.
    Second {
        via: Option<Vec2>,
    },
    /// The start direction, then the end point.
    Direction {
        dir: Option<Vec2>,
    },
}

/// The path tool: a closed area or an open polyline.
#[derive(Clone, Debug)]
pub struct Path {
    shape: Shape,
    pts: Vec<Vec2>,
    /// One bulge per drawn segment (pts[i] → pts[i+1]).
    bulges: Vec<f64>,
    /// The effective cursor from the last pointer move.
    hover: Option<Vec2>,
    tracking: Option<Tracking>,
    arc_mode: bool,
    spec: Spec,
    /// A point on the first arc of a path, which has no tangent to follow.
    arc_via: Option<Vec2>,
    /// Line mode: waiting for a typed length along the last direction.
    ask_length: bool,
    /// Akış: waiting for a typed step.
    ask_step: bool,
    /// Bulge of the closing segment: an arc when the area was closed on its first corner in arc mode.
    closing: f64,
    /// Where the button went down, while that click is being taken (closing on the first corner).
    pressed_at: Option<[f64; 2]>,
    /// What the session remembered, as of the last call (the prompt sees no context).
    memory: Memory,
    /// The project's units, as of the last call (Adım boyu's value in the prompt).
    format: Format,
    /// İçine tıkla: where the pointer is and the region around it.
    inside: Option<(Vec2, Option<Region>)>,
    /// The faces of the visible line work, kept while the drawing and the view stand.
    faces: FaceCache,
    /// The area last measured, which Alan olarak çiz writes; kept until the next measurement starts.
    measured: Option<Region>,
    /// The visible line work İzle follows, kept while the view and the drawing stand.
    work: WorkCache,
    /// İzle's way from the last point to the cursor, for the preview: its two
    /// ends and the way (the preview has no context to find it).
    way: Option<(Vec2, Vec2, Traced)>,
    /// Bitişik alan: the neighbouring areas in view, kept while the view, the drawing and the layers stand.
    neighbours: adjoin::NeighbourCache,
    /// Bitişik alan: the region the preview fills (the path to the cursor, or
    /// as the last click left it when the neighbours are many).
    region: Vec<Region>,
}

/// The faces İçine tıkla finds regions in ([`faces::Faces`], which is neither
/// `Clone` nor `Debug`): a copy of the tool starts without them.
#[derive(Default)]
struct FaceCache(Option<faces::Faces>);

impl Clone for FaceCache {
    fn clone(&self) -> Self {
        Self(None)
    }
}

impl std::fmt::Debug for FaceCache {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FaceCache")
    }
}

impl Path {
    pub fn new(shape: Shape) -> Self {
        Self {
            shape,
            pts: Vec::new(),
            bulges: Vec::new(),
            hover: None,
            tracking: None,
            arc_mode: false,
            spec: Spec::Tangent,
            arc_via: None,
            ask_length: false,
            ask_step: false,
            closing: 0.0,
            pressed_at: None,
            memory: Memory::default(),
            format: Format::default(),
            inside: None,
            faces: FaceCache::default(),
            measured: None,
            work: WorkCache::default(),
            way: None,
            neighbours: adjoin::NeighbourCache::default(),
            region: Vec::new(),
        }
    }

    /// The closed-area tool (`tool.polygon`).
    pub fn polygon() -> Self {
        Self::new(Shape::Closed)
    }

    /// The polyline tool (`tool.polyline`).
    pub fn polyline() -> Self {
        Self::new(Shape::Open)
    }

    fn closed(&self) -> bool {
        self.shape.closed()
    }

    fn last(&self) -> Option<Vec2> {
        self.pts.last().copied()
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.memory = *cx.memory;
        self.format = cx.format();
    }

    /// Mesafe ölç with Sabit ilk nokta on: rays from the first point, not a chain.
    fn fixed(&self) -> bool {
        self.shape == Shape::MeasureLength && self.memory.measure_fixed
    }

    /// Alan hesapla with İçine tıkla on: a click measures the region it is inside.
    fn inside_mode(&self) -> bool {
        self.shape == Shape::MeasureArea && self.memory.area_inside
    }

    /// Whether İzle applies now: on, in line mode, neither Sabit ilk nokta's
    /// rays nor İçine tıkla (docs/adr/0161 §1).
    fn tracing(&self) -> bool {
        self.memory.trace && !self.arc_mode && !self.fixed() && !self.inside_mode()
    }

    /// Whether Akış applies now, as İzle does, and no number is waited for (docs/adr/0161 §3).
    fn streaming(&self) -> bool {
        self.memory.stream
            && !self.arc_mode
            && !self.fixed()
            && !self.inside_mode()
            && !self.ask_length
            && !self.ask_step
    }

    /// The point new ones are measured from, and ortho, polar tracking and
    /// typed distances go from: the first with Sabit ilk nokta, else the last.
    fn base(&self) -> Option<Vec2> {
        if self.fixed() {
            self.pts.first().copied()
        } else {
            self.last()
        }
    }

    /// Travel direction at the last corner (the end tangent of the last segment).
    fn tangent(&self) -> Option<Vec2> {
        let n = self.pts.len();
        (n >= 2).then(|| {
            let bulge = self.bulges.get(n - 2).copied().unwrap_or(0.0);
            segment_tangent(self.pts[n - 2], self.pts[n - 1], bulge, true)
        })
    }

    /// Where a segment towards `p` really ends: the centre option puts it on the circle.
    fn end_for(&self, p: Vec2) -> Vec2 {
        match (self.spec, self.last()) {
            (Spec::Centre { c: Some(c) }, Some(last)) if self.arc_mode => {
                radial_point(c, dist(c, last), p).unwrap_or(p)
            }
            _ => p,
        }
    }

    /// The bulge the next segment to `p` would get; `None` when impossible
    /// or still waiting for a value.
    fn next_bulge(&self, p: Vec2) -> Option<f64> {
        let Some(last) = self.last() else {
            return Some(0.0);
        };
        if !self.arc_mode {
            return Some(0.0);
        }
        match self.spec {
            Spec::Angle { sweep } => sweep.map(bulge_of_sweep),
            // Bends the way the path turns towards p; counter-clockwise with no tangent yet.
            Spec::Radius { r } => r.and_then(|r| radius_bulge(last, p, r, self.tangent())),
            Spec::Centre { c } => c.and_then(|c| centre_bulge(c, last, self.end_for(p))),
            Spec::Second { via } => via.map(|via| bulge_through(last, via, p)),
            Spec::Direction { dir } => dir.and_then(|dir| tangent_bulge(last, dir, p)),
            Spec::Tangent => match self.tangent() {
                Some(t) => tangent_bulge(last, t, p),
                None => self.arc_via.map(|via| bulge_through(last, via, p)),
            },
        }
    }

    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        points::echo(p, cx);
        self.on_point(p, cx);
        self.refill(false, cx);
    }

    /// Bitişik alan: the region the preview fills. `live`, the path goes on
    /// to the cursor, unless the neighbours are too many to cut through at
    /// every move (then the region stays as the last click left it).
    fn refill(&mut self, live: bool, cx: &Context<'_>) {
        if self.shape != Shape::Adjoin {
            return;
        }
        let (pts, bulges) = if live {
            self.preview_path()
        } else {
            (self.pts.clone(), self.bulges.clone())
        };
        let neighbours = self.neighbours.get(cx);
        if live && neighbours.edge_count() > adjoin::PREVIEW_EDGES {
            return;
        }
        let region = adjoin::fill(neighbours, &pts, &bulges);
        self.region = region;
    }

    /// The path as the preview draws it: the points given, then İzle's way to
    /// the cursor, or the segment to it (none while a value is waited for).
    fn preview_path(&self) -> (Vec<Vec2>, Vec<f64>) {
        let mut pts = self.pts.clone();
        let mut bulges = self.bulges.clone();
        let end = self.hover.map(|h| self.end_for(h));
        match (self.preview_way(end), end) {
            (Some(way), _) => {
                pts.extend_from_slice(way.pts.get(1..).unwrap_or_default());
                bulges.extend_from_slice(&way.bulges);
            }
            (None, Some(end)) => {
                if let Some(hb) = self.next_bulge(end)
                    && self.last().is_none_or(|last| dist(last, end) > SAME)
                {
                    pts.push(end);
                    bulges.push(hb);
                }
            }
            (None, None) => {}
        }
        (pts, bulges)
    }

    /// İzle's way to `end`, while it still goes from the last point there.
    fn preview_way(&self, end: Option<Vec2>) -> Option<&Traced> {
        self.way
            .as_ref()
            .filter(|(from, to, _)| {
                self.tracing() && Some(*from) == self.last() && Some(*to) == end
            })
            .map(|(_, _, w)| w)
    }

    fn on_point(&mut self, p: Vec2, cx: &mut Context<'_>) {
        if self.inside_mode() {
            return self.click_inside(p, cx);
        }
        if self.fixed() {
            return self.on_ray(p, cx);
        }
        let Some(last) = self.last() else {
            self.pts.push(p);
            // A new measurement starts: what was measured before is no more Alan olarak çiz's.
            self.measured = None;
            return;
        };
        // Options that take a point before the end point.
        if self.arc_mode {
            let tangent = self.tangent();
            match &mut self.spec {
                Spec::Centre { c: c @ None } => {
                    *c = Some(p);
                    return;
                }
                Spec::Second { via: via @ None } => {
                    *via = Some(p);
                    return;
                }
                Spec::Direction { dir: dir @ None } => {
                    if dist(last, p) > SAME {
                        *dir = unit_toward(last, p);
                    }
                    return;
                }
                Spec::Tangent if tangent.is_none() && self.arc_via.is_none() => {
                    self.arc_via = Some(p);
                    return;
                }
                _ => {}
            }
        }
        let end = self.end_for(p);
        if dist(last, end) <= SAME {
            return;
        }
        if self.closes_at(end, cx) {
            return self.close_on_first(cx);
        }
        // İzle: along the line work, its corners and arcs as they are.
        if self.tracing()
            && let Some(way) = self.work.path(last, end, cx)
        {
            self.pts
                .extend_from_slice(way.pts.get(1..).unwrap_or_default());
            self.bulges.extend_from_slice(&way.bulges);
            return;
        }
        let Some(bulge) = self.next_bulge(end) else {
            let text = match self.spec {
                Spec::Radius { r: Some(r) } => format!(
                    "Kiriş yarıçapın iki katından ({}) uzun; daha yakın bir nokta seçin.",
                    cx.format().length(2.0 * r)
                ),
                _ => "Bu nokta yayın tam arkasında kalıyor; başka bir nokta seçin.".to_owned(),
            };
            cx.say(Level::Warn, text);
            return;
        };
        self.pts.push(end);
        self.bulges.push(bulge);
        self.arc_via = None;
        // Arc options shape one segment; the path then continues tangentially.
        self.spec = Spec::Tangent;
    }

    /// A closed shape ends when its first corner is given again (ADR 0018):
    /// typed exactly, or clicked within the snap aperture once there are
    /// three corners. The first corner is never written twice. An open
    /// polyline never closes this way.
    fn closes_at(&self, end: Vec2, cx: &Context<'_>) -> bool {
        let Some(&first) = self.pts.first().filter(|_| self.closed()) else {
            return false;
        };
        if dist(first, end) <= SAME {
            return true;
        }
        let Some(pressed) = self.pressed_at else {
            return false;
        };
        if self.pts.len() < self.shape.min() {
            return false;
        }
        let s = cx.view.to_screen(first);
        js_hypot(s[0] - pressed[0], s[1] - pressed[1]) <= cx.draft.snap_aperture
    }

    fn close_on_first(&mut self, cx: &mut Context<'_>) {
        if self.pts.len() < self.shape.min() {
            cx.say(
                Level::Warn,
                format!(
                    "{} için en az 3 köşe gerekir; ilk köşe ikinci kez eklenmedi.",
                    self.shape.label()
                ),
            );
            return;
        }
        // İzle: the way back to the first corner along the line work, its last edge the closing one.
        if self.tracing()
            && let (Some(&last), Some(&first)) = (self.pts.last(), self.pts.first())
            && let Some(way) = self.work.path(last, first, cx)
        {
            let n = way.pts.len();
            self.pts
                .extend_from_slice(way.pts.get(1..n - 1).unwrap_or_default());
            self.bulges
                .extend_from_slice(way.bulges.get(..n - 2).unwrap_or_default());
            self.closing = way.bulges.get(n - 2).copied().unwrap_or(0.0);
            return self.finish(cx);
        }
        // In arc mode the segment back to the first corner is the arc being drawn.
        let Some(bulge) = self.next_bulge(self.pts[0]) else {
            cx.say(
                Level::Warn,
                "İlk köşe yayın tam arkasında kalıyor; alanı Enter ile düz kenarla kapatın.",
            );
            return;
        };
        self.closing = bulge;
        self.finish(cx);
    }

    /// A ray of Sabit ilk nokta: measured from the first point, said in the log.
    fn on_ray(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let Some(first) = self.pts.first().copied() else {
            self.pts.push(p);
            return;
        };
        if dist(first, p) <= SAME {
            return;
        }
        self.pts.push(p);
        self.bulges.push(0.0);
        let f = cx.format();
        // The direction as the project's type reads it: a semt, or a CAD project's angle (docs/adr/0165 §4).
        let mut text = format!(
            "{}: {}, {} {}",
            self.pts.len() - 1,
            f.length(dist(first, p)),
            f.direction_name().to_lowercase(),
            f.direction(bearing_grad(first, p))
        );
        // The ray's length in the second system's plane too, when it has one (docs/adr/0167 §2).
        if let Some(second) = Second::of(cx.doc.settings()) {
            let ray = Ring {
                pts: vec![first, p],
                bulges: None,
            };
            if let Ok(m) = second.measure(&[ray], false) {
                text.push_str(&format!(
                    " ({} düzleminde {})",
                    second.short(),
                    f.length(m.length)
                ));
            }
        }
        cx.say(Level::Info, text);
    }

    /// İçine tıkla: the region around `p` is measured, as a ring drawn is.
    fn click_inside(&mut self, p: Vec2, cx: &mut Context<'_>) {
        let Some(region) = self.face(p, cx) else {
            cx.say(Level::Warn, "Tıklanan noktayı çevreleyen kapalı bölge yok.");
            return;
        };
        let f = cx.format();
        let text = format!(
            "Alan {}   Çevre {}",
            f.area(net_area(&region)),
            f.length(perimeter(&region))
        );
        cx.say(Level::Success, text);
        let rings: Vec<Ring> = std::iter::once(&region.outer)
            .chain(&region.holes)
            .cloned()
            .collect();
        say_second(&rings, true, cx);
        self.measured = Some(region);
    }

    /// The face of the visible line work around `p`, closed groups inside it as holes.
    fn face(&mut self, p: Vec2, cx: &Context<'_>) -> Option<Region> {
        faces::face_at(&mut self.faces.0, p, true, None, cx)
    }

    /// The three switches of docs/adr/0141: Sabit ilk nokta (S) before the
    /// first point or while it is on, İçine tıkla (I) likewise, Alan olarak çiz
    /// (A) after a measurement. False when the key is none of them now.
    fn switch(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        match (self.shape, key) {
            (Shape::MeasureLength, "S") if self.pts.is_empty() || self.fixed() => {
                cx.memory.measure_fixed = !cx.memory.measure_fixed;
                // A run of one kind is not carried over into the other.
                self.reset();
            }
            (Shape::MeasureArea, "I" | "İ")
                if !self.arc_mode && (self.pts.is_empty() || self.inside_mode()) =>
            {
                cx.memory.area_inside = !cx.memory.area_inside;
                self.reset();
                self.inside = None;
            }
            (Shape::MeasureArea, "A") if self.pts.is_empty() && self.measured.is_some() => {
                self.draw_measured(cx);
            }
            _ => return false,
        }
        self.see(cx);
        true
    }

    /// Alan olarak çiz: the area last measured, holes and all, written to the
    /// active layer through `cad.polygon.create` as one undo step named “Alan
    /// olarak çiz”. A refusal (a locked layer) is the command's own message,
    /// and nothing is written. With the overlap control on (docs/adr/0162 §2),
    /// what overlaps the neighbours is cut away first and the rest written
    /// through `cad.entities.create`; with Topoloji on, the area is joined
    /// with its neighbours corner by corner (§4); all in the same step.
    fn draw_measured(&mut self, cx: &mut Context<'_>) {
        let Some(region) = self.measured.clone() else {
            return;
        };
        let layer = cx.doc.layers().active().to_owned();
        let clipped = overlap::clip_new_area(cx, &region, &layer);
        if let Some(c) = &clipped {
            overlap::say_clipped(cx, c);
        }
        let (areas, area) = match &clipped {
            Some(c) => (c.areas.clone(), overlap::written_area(&c.areas)),
            None => (vec![region.clone()], net_area(&region)),
        };
        if areas.is_empty() {
            return;
        }
        let joining = junctions::join(cx, &areas);
        let areas = joining.as_ref().map_or(areas, |j| j.areas.clone());
        let group = cx.doc.begin_group(DRAW_AREA_LABEL);
        let wrote = if clipped.is_some() {
            overlap::clipped_geometry(&areas)
                .and_then(|g| points::write_objects(vec![g], None, cx))
                .is_some()
        } else {
            let ring = |r: &Ring| RingGeometry {
                pts: wire_all(&r.pts),
                bulges: r.bulges.clone(),
                zs: None,
            };
            let area = &areas[0];
            let input = PolygonCreate {
                layer_id: layer,
                pts: wire_all(&area.outer.pts),
                bulges: area.outer.bulges.clone(),
                holes: (!area.holes.is_empty()).then(|| area.holes.iter().map(ring).collect()),
                color: cx.draft.color.map(str::to_owned),
                line_weight: cx.draft.line_weight,
                attrs: None,
                expected_revision: None,
            };
            let result = polygon::execute(&mut ExecutionContext::new(cx.doc), input);
            points::written(result, cx).is_some()
        };
        if !wrote
            || !joining
                .as_ref()
                .is_none_or(|j| junctions::write_neighbours(j, cx))
        {
            cx.doc.cancel_group(group);
            return;
        }
        cx.doc.end_group(group);
        junctions::say(joining.as_ref(), cx);
        let text = format!("Alan olarak çizildi: {}.", cx.format().area(area));
        cx.say(Level::Success, text);
    }

    /// Option letters: Y, D, U, G and the arc options. False when the key is none of them now.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
        // Rays have no arcs and no direction to go on in: only Geri applies.
        if self.fixed() && key != "G" {
            return false;
        }
        // Dik kapat (docs/adr/0166 §4): a ring of three corners or more, its
        // last edge drawn straight, closes square on its first edge.
        if key == "D" && !self.arc_mode && self.shape.closed() && self.pts.len() >= 3 {
            self.square_close(cx);
            return true;
        }
        let arc = match key {
            "A" => Some(Spec::Angle { sweep: None }),
            "M" => Some(Spec::Centre { c: None }),
            "R" => Some(Spec::Radius { r: None }),
            "İ" | "I" => Some(Spec::Second { via: None }),
            "T" => Some(Spec::Direction { dir: None }),
            _ => None,
        };
        if key == "Y" || key == "D" {
            self.arc_mode = key == "Y";
            self.arc_via = None;
            self.spec = Spec::Tangent;
            self.ask_length = false;
        } else if let (true, Some(arc), false) = (self.arc_mode, arc, self.pts.is_empty()) {
            self.spec = arc;
        } else if !self.arc_mode && key == "U" && !self.pts.is_empty() {
            if self.tangent().is_none() {
                cx.say(
                    Level::Warn,
                    "Uzunlukla devam için önce bir parça çizin; ilk parçanın doğrultusu yok.",
                );
                return true;
            }
            self.ask_length = true;
        } else if !self.arc_mode && (key == "İ" || key == "I") && !self.pts.is_empty() {
            cx.memory.trace = !cx.memory.trace;
            self.memory.trace = cx.memory.trace;
            self.way = None;
        } else if !self.arc_mode && key == "A" && !self.pts.is_empty() {
            cx.memory.stream = !cx.memory.stream;
            self.memory.stream = cx.memory.stream;
        } else if !self.arc_mode && key == "B" && self.memory.stream && !self.pts.is_empty() {
            self.ask_step = true;
        } else if key == "G" && self.ask_step {
            // Geri leaves the step as it was; the points stay.
            self.ask_step = false;
        } else if key == "G" && self.arc_via.is_some() {
            self.arc_via = None;
        } else if key == "G" && !self.pts.is_empty() {
            self.pts.pop();
            self.bulges.pop();
            self.spec = Spec::Tangent;
            // A length waited for along the last direction is not waited for any more (docs/adr/0067).
            self.ask_length = false;
        } else {
            return false;
        }
        true
    }

    /// Dik kapat: the corner where the line through the last corner square
    /// to the last edge meets the line through the first corner square to
    /// the first edge (the edges' chords), then the shape closes. A shape
    /// already square there takes no corner; parallel edges, none at all.
    fn square_close(&mut self, cx: &mut Context<'_>) {
        let n = self.pts.len();
        let (first, last) = (self.pts[0], self.pts[n - 1]);
        let Some(corner) = square_corner(first, self.pts[1], self.pts[n - 2], last) else {
            cx.say(Level::Warn, NO_SQUARE_CLOSE.to_owned());
            return;
        };
        if dist(corner, last) > SAME && dist(corner, first) > SAME {
            points::echo(corner, cx);
            self.pts.push(corner);
            self.bulges.push(0.0);
        }
        self.closing = 0.0;
        self.finish(cx);
    }

    /// Commits the shape when it has enough points, then starts over.
    fn finish(&mut self, cx: &mut Context<'_>) {
        let min = self.shape.min();
        if self.pts.len() < min {
            cx.say(
                Level::Warn,
                format!("{} için en az {min} nokta gerekir.", self.shape.label()),
            );
            self.reset();
            return;
        }
        // Bitişik alan: the region the path closes, with the neighbours the
        // preview had; with none (or a refusal), the path stays.
        if self.shape == Shape::Adjoin {
            let region = adjoin::fill(self.neighbours.get(cx), &self.pts, &self.bulges);
            if adjoin::write(region, cx) {
                self.reset();
            }
            return;
        }
        let pts = self.pts.clone();
        let bulges = self.full_bulges();
        match self.shape {
            // Rays have no total: what each said is all there is.
            Shape::MeasureLength if self.fixed() => {}
            Shape::MeasureLength => {
                let length = bulge_path_length(&pts, bulges.as_deref(), false);
                let text = format!(
                    "Toplam uzunluk {} ({} kenar)",
                    cx.format().length(length),
                    pts.len() - 1
                );
                cx.say(Level::Success, text);
                let path = Ring {
                    pts: pts.clone(),
                    bulges: bulges.clone(),
                };
                say_second(&[path], false, cx);
            }
            Shape::MeasureArea => {
                let area = bulge_ring_area(&pts, bulges.as_deref()).abs();
                let perimeter = bulge_path_length(&pts, bulges.as_deref(), true);
                let f = cx.format();
                let text = format!("Alan {}   Çevre {}", f.area(area), f.length(perimeter));
                cx.say(Level::Success, text);
                let ring = Ring {
                    pts: pts.clone(),
                    bulges: bulges.clone(),
                };
                say_second(&[ring], true, cx);
                self.measured = Some(Region {
                    outer: Ring { pts, bulges },
                    holes: Vec::new(),
                });
            }
            Shape::Parcel => self.create_parcel(&pts, bulges, cx),
            Shape::Closed => {
                if let Some(area) = self.create_polygon(&pts, bulges, cx) {
                    let text = format!("Kapalı alan eklendi: {}", cx.format().area(area));
                    cx.say(Level::Success, text);
                }
            }
            Shape::Open => {
                let length = bulge_path_length(&pts, bulges.as_deref(), false);
                if self.create_polyline(&pts, cx) {
                    let text = format!("Çoklu çizgi eklendi: {}", cx.format().length(length));
                    cx.say(Level::Success, text);
                }
            }
            Shape::Adjoin => {}
        }
        self.reset();
    }

    /// Bulges of the finished shape: one per point, the last for the closing
    /// edge (straight unless the area was closed on its first corner with an
    /// arc; an open polyline has none); none when every edge is straight.
    fn full_bulges(&self) -> Option<Vec<f64>> {
        let mut all = self.bulges.clone();
        all.push(self.closing);
        has_bulges(Some(&all)).then_some(all)
    }

    /// Writes the area through the product command `cad.polygon.create`
    /// (docs/adr/0022), as one undo step. What the web's tool knows
    /// implicitly is explicit in its input (CMD-07): the active layer and
    /// the current colour. The overlap control (docs/adr/0162 §2) cuts what
    /// overlaps the neighbours and writes the rest as one object; Topoloji
    /// joins it with its neighbours corner by corner (§4), in the same step.
    /// The area written; `None`, with the command's message, when it refused
    /// (a locked layer), or when the neighbours covered it; a hidden layer is
    /// written with its warning.
    fn create_polygon(
        &self,
        pts: &[Vec2],
        bulges: Option<Vec<f64>>,
        cx: &mut Context<'_>,
    ) -> Option<f64> {
        let drawn = Region {
            outer: Ring {
                pts: pts.to_vec(),
                bulges: bulges.clone(),
            },
            holes: Vec::new(),
        };
        let layer = cx.doc.layers().active().to_owned();
        let clipped = overlap::clip_new_area(cx, &drawn, &layer);
        if let Some(c) = &clipped {
            overlap::say_clipped(cx, c);
        }
        let (areas, area) = match &clipped {
            Some(c) => (c.areas.clone(), overlap::written_area(&c.areas)),
            None => (vec![drawn], bulge_ring_area(pts, bulges.as_deref()).abs()),
        };
        if areas.is_empty() {
            return None;
        }
        let joining = junctions::join(cx, &areas);
        let areas = joining.as_ref().map_or(areas, |j| j.areas.clone());
        junctions::with_joined(cx, joining.as_ref(), labels::ADD, |cx| {
            if clipped.is_some() {
                let geometry = overlap::clipped_geometry(&areas)?;
                return points::write_objects(vec![geometry], None, cx).map(|_| ());
            }
            let ring = &areas[0].outer;
            let input = PolygonCreate {
                layer_id: layer,
                pts: ring.pts.iter().copied().map(wire).collect(),
                bulges: ring.bulges.clone(),
                holes: None,
                color: cx.draft.color.map(str::to_owned),
                line_weight: cx.draft.line_weight,
                attrs: None,
                expected_revision: None,
            };
            let result = polygon::execute(&mut ExecutionContext::new(cx.doc), input);
            points::written(result, cx).map(|_| ())
        })?;
        junctions::say(joining.as_ref(), cx);
        Some(area)
    }

    /// Writes the polyline through the product command `cad.polyline.create`
    /// (docs/adr/0027), as one undo step, with one bulge per drawn segment;
    /// the command stores the document's per-point form. As for the area:
    /// the active layer and the current colour; false when refused.
    fn create_polyline(&self, pts: &[Vec2], cx: &mut Context<'_>) -> bool {
        let input = PolylineCreate {
            layer_id: cx.doc.layers().active().to_owned(),
            pts: pts.iter().copied().map(wire).collect(),
            bulges: has_bulges(Some(&self.bulges)).then(|| self.bulges.clone()),
            color: cx.draft.color.map(str::to_owned),
            line_weight: cx.draft.line_weight,
            attrs: None,
            expected_revision: None,
        };
        let result = polyline::execute(&mut ExecutionContext::new(cx.doc), input);
        points::written(result, cx).is_some()
    }

    /// Writes the parcel through the product command `cad.entities.create`
    /// on the parcel layer, whatever the active one (docs/adr/0067), in the
    /// current colour: the next number on that layer as its label and its
    /// Parsel, Nitelik “Arsa”, the
    /// other attributes left for Öznitelikler. The deed area is left empty:
    /// it is the title deed's, not the drawing's (CLAUDE.md §7, §23); the
    /// message gives the geometric area. The parcel is selected, so
    /// Öznitelikler shows it. A locked parcel layer is said in the tool's own
    /// words, since another active layer would not help.
    fn create_parcel(&self, pts: &[Vec2], bulges: Option<Vec<f64>>, cx: &mut Context<'_>) {
        let layers = cx.doc.layers();
        if let Some(node) = layers.get(PARCEL_LAYER)
            && layers.is_locked(PARCEL_LAYER)
        {
            let text = format!(
                "“{}” katmanı kilitli; {PARCEL_LABEL} bu katmana yazar. Kilidi Katmanlar panelinden açın.",
                node.name
            );
            cx.say(Level::Warn, text);
            return;
        }
        // The overlap control (docs/adr/0162 §2) on the parcel layer: the parcel is what is left.
        let drawn = Region {
            outer: Ring {
                pts: pts.to_vec(),
                bulges: bulges.clone(),
            },
            holes: Vec::new(),
        };
        let clipped = overlap::clip_new_area(cx, &drawn, PARCEL_LAYER);
        if let Some(c) = &clipped {
            overlap::say_clipped(cx, c);
        }
        let (areas, area) = match &clipped {
            Some(c) => (c.areas.clone(), overlap::written_area(&c.areas)),
            None => (vec![drawn], bulge_ring_area(pts, bulges.as_deref()).abs()),
        };
        // Topoloji (§4): the parcel joined with its neighbours corner by corner, in its step.
        let joining = junctions::join(cx, &areas);
        let areas = joining.as_ref().map_or(areas, |j| j.areas.clone());
        let Some(geometry) = overlap::clipped_geometry(&areas) else {
            return;
        };
        let number = next_parcel(cx).to_string();
        let attrs = [
            ("Ada", ""),
            ("Parsel", number.as_str()),
            ("Mahalle", ""),
            ("Nitelik", "Arsa"),
            ("Tapu alanı (m²)", ""),
            ("Pafta", ""),
        ];
        let input = EntitiesCreate {
            layer_id: PARCEL_LAYER.to_owned(),
            objects: vec![NewObject {
                geometry,
                color: cx.draft.color.map(str::to_owned),
                line_weight: cx.draft.line_weight,
                attrs: Some(BTreeMap::from(attrs.map(|(k, v)| (k.to_owned(), v.to_owned())))),
                label: Some(number.clone()),
            }],
            operation: None,
            expected_revision: None,
        };
        let written = junctions::with_joined(cx, joining.as_ref(), labels::ADD, |cx| {
            // A drawing without the parcel layer gets it, in the parcel's own undo step.
            let opened = crate::standard_layer::open_if_missing(PARCEL_LAYER, "parsel", cx).ok()?;
            let result = create::execute(&mut ExecutionContext::new(cx.doc), input);
            let out = points::written(result, cx);
            match (&out, opened) {
                (Some(_), Some(opened)) => opened.keep(cx),
                (None, Some(opened)) => opened.drop(cx),
                (_, None) => {}
            }
            out
        });
        let Some(out) = written else {
            return;
        };
        junctions::say(joining.as_ref(), cx);
        cx.selection.set(out.ids.iter().map(|&id| Slot(id)).collect::<Vec<_>>());
        let text = format!(
            "Parsel {number} oluşturuldu; geometrik alanı {}. Ada, mahalle ve tapu alanı bilgisini Öznitelikler panelinden girin.",
            cx.format().area(area)
        );
        cx.say(Level::Success, text);
    }

    fn reset(&mut self) {
        self.pts.clear();
        self.bulges.clear();
        self.arc_via = None;
        self.arc_mode = false;
        self.spec = Spec::Tangent;
        self.ask_length = false;
        self.ask_step = false;
        self.closing = 0.0;
        self.way = None;
        self.region.clear();
    }

    /// The effective cursor for the next point: ortho (Shift turns it over)
    /// and polar tracking from the last point, by the shared core.
    fn constrain(&mut self, p: &Pointer, cx: &Context<'_>) -> Vec2 {
        let (point, tracking) = points::constrain(self.base(), p, cx);
        self.tracking = tracking;
        // İzle: an unsnapped point within the snap aperture of the visible
        // line work goes onto it (a snapped or tracked one is where the user wants it).
        if self.tracing() && p.snap.is_none() && !p.tracked {
            let reach = cx.view.world_length(cx.draft.snap_aperture);
            if let Some(on) = self.work.nearest(point, reach, cx) {
                return on;
            }
        }
        point
    }

    fn arc_options(prompt: Prompt, done: bool) -> Prompt {
        let prompt = prompt
            .option("Düz", "D")
            .option("Açı", "A")
            .option("Merkez", "M")
            .option("Yarıçap", "R")
            .option("İkinci nokta", "İ")
            .option("Doğrultu", "T")
            .option("Geri", "G");
        if done {
            prompt.option("Bitir", "Enter")
        } else {
            prompt
        }
    }
}

/// The next parcel number: the highest `Parsel` on the parcel layer, as
/// JavaScript's `parseInt` reads it (“12/1” is 12, a text without leading
/// digits 0), plus one (the web's `createParcel`).
fn next_parcel(cx: &Context<'_>) -> u64 {
    let doc = &*cx.doc;
    doc.entities()
        .filter(|e| e.base().layer_id == PARCEL_LAYER)
        .filter_map(|e| e.base().attrs.get("Parsel"))
        .map(|text| js_parse_int(text).unwrap_or(0))
        .max()
        .unwrap_or(0)
        .saturating_add(1)
}

/// JavaScript's `parseInt(text, 10)` for a whole number that is not negative:
/// white space at the start skipped, the digits up to the first other
/// character; none when there is no digit or a minus sign (which counts as 0
/// in the web's maximum too).
fn js_parse_int(text: &str) -> Option<u64> {
    let rest = text.trim_start_matches(|c: char| c.is_whitespace() || c == '\u{feff}');
    let rest = rest.strip_prefix('+').unwrap_or(rest);
    let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
    if digits.is_empty() {
        return None;
    }
    Some(digits.parse().unwrap_or(u64::MAX))
}

/// The perimeter of a region as İçine tıkla says it: its outer ring's and its
/// islands', as a polygon's (`measure::polygon_perimeter`, holes included as in
/// GIS): the region drawn with Alan olarak çiz shows the same in Öznitelikler.
/// Mesafe ölç's and Alan hesapla's measure in the second system's plane, or
/// why there is none, when the project has a second system (docs/adr/0167 §2).
fn say_second(rings: &[Ring], closed: bool, cx: &mut Context<'_>) {
    if let Some(second) = Second::of(cx.doc.settings()) {
        let line = second.measures_line(rings, closed, &cx.format());
        cx.say(Level::Info, line);
    }
}

fn perimeter(region: &Region) -> f64 {
    std::iter::once(&region.outer)
        .chain(&region.holes)
        .map(|r| bulge_path_length(&r.pts, r.bulges.as_deref(), true))
        .sum()
}

/// A region as the web's `drawArea` draws an area: its outer ring and holes, lightly filled.
fn region_area(region: &Region, fill: f32, dash: Option<[f32; 2]>) -> Area {
    Area {
        rings: std::iter::once(&region.outer)
            .chain(&region.holes)
            .map(|ring| polygon_ring(&ring.pts, ring.bulges.as_deref()))
            .collect(),
        fill,
        width: 2.0,
        dash,
        fill_tone: Tone::Accent,
    }
}

impl Path {
    /// Chips of the step that takes no point yet, and of İçine tıkla: Mesafe
    /// ölç's Sabit ilk nokta; Alan hesapla's İçine tıkla, and Alan olarak çiz
    /// once something was measured.
    fn first_chips(&self, prompt: Prompt) -> Prompt {
        match self.shape {
            Shape::MeasureLength => {
                prompt.toggle("Sabit ilk nokta", "S", self.memory.measure_fixed)
            }
            Shape::MeasureArea => {
                let prompt = prompt.toggle("İçine tıkla", "I", self.memory.area_inside);
                if self.measured.is_some() && self.pts.is_empty() {
                    prompt.option(DRAW_AREA_LABEL, "A")
                } else {
                    prompt
                }
            }
            _ => prompt,
        }
    }

    /// Sabit ilk nokta: the rays from the first point, each numbered as the
    /// log numbers it, and the cursor's own with its distance and semt.
    fn fixed_preview(&self, format: &Format) -> Preview {
        let Some(first) = self.pts.first().copied() else {
            return Preview::default();
        };
        let mut preview = Preview::default();
        preview.markers.push(Marker {
            at: first,
            shape: MarkerShape::Ring(5.0),
            tone: Tone::Snap,
        });
        for (i, &end) in self.pts.iter().enumerate().skip(1) {
            preview.strokes.push(Stroke::solid(vec![first, end], false));
            preview.markers.push(Marker {
                at: end,
                shape: MarkerShape::Ring(3.0),
                tone: Tone::Accent,
            });
            preview.labels.push(Label {
                at: end,
                text: i.to_string(),
                offset: [6.0, -6.0],
                tone: Tone::Snap,
            });
        }
        if let Some(hover) = self.hover.filter(|h| dist(first, *h) > SAME) {
            preview
                .strokes
                .push(Stroke::dashed(vec![first, hover], false, [6.0, 4.0]));
            preview.tag = Some(Tag {
                at: hover,
                lines: vec![
                    format.length(dist(first, hover)),
                    format!(
                        "{} {}",
                        format.direction_name(),
                        format.direction(bearing_grad(first, hover))
                    ),
                ],
            });
            preview.tracking = self.tracking;
        }
        preview
    }

    /// İçine tıkla: the region under the cursor filled with its area beside
    /// it, and the last one measured (what Alan olarak çiz would write) dashed.
    fn inside_preview(&self, format: &Format) -> Preview {
        let hovered = self.inside.as_ref().and_then(|(_, region)| region.as_ref());
        let mut areas = Vec::new();
        if let Some(measured) = self.measured.as_ref().filter(|m| Some(*m) != hovered) {
            areas.push(region_area(measured, 0.1, Some([5.0, 4.0])));
        }
        let mut tag = None;
        if let Some((at, Some(region))) = &self.inside {
            areas.push(region_area(region, 0.16, None));
            let mut lines = vec![
                format!("Alan {}", format.area(net_area(region))),
                format!("Çevre {}", format.length(perimeter(region))),
            ];
            if !region.holes.is_empty() {
                lines.push(format!("{} ada", region.holes.len()));
            }
            tag = Some(Tag { at: *at, lines });
        }
        Preview {
            areas,
            tag,
            ..Preview::default()
        }
    }
}

impl Tool for Path {
    /// A point computed by the point calculator, as if clicked (the web's `acceptPoint`).
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.accept(p, cx);
        true
    }
    fn id(&self) -> &'static str {
        self.shape.id()
    }

    /// Perpendicular and tangent snaps are taken from the last point (the web's `snapFrom`);
    /// with Sabit ilk nokta, from the first.
    fn snap_from(&self) -> Option<Vec2> {
        self.base()
    }

    /// The tangent at the last corner (docs/adr/0166 §1); Sabit ilk nokta's
    /// rays travel nowhere.
    fn travel(&self) -> Option<Vec2> {
        // Rays from a fixed first point, and an arc being drawn, have no edge
        // for Sapma and Dik açı to turn from.
        if self.fixed() || self.arc_mode {
            return None;
        }
        self.tangent()
            .and_then(|t| kentos_geometry_core::tools::locks::unit(Vec2::new(0.0, 0.0), t))
    }

    /// The path so far, for snapping to it (the web's `draftPath`,
    /// docs/adr/0163 §3); Sabit ilk nokta's rays are no path, and İçine
    /// tıkla has no points.
    fn draft_path(&self) -> Option<kentos_geometry_core::entity::Shape> {
        (!self.pts.is_empty() && !self.fixed()).then(|| {
            kentos_geometry_core::entity::Shape::Polyline {
                pts: self.pts.clone(),
                bulges: has_bulges(Some(&self.bulges)).then(|| self.bulges.clone()),
                holes: None,
            }
        })
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    /// İçine tıkla clicks inside a region, not on a point: nothing to snap to.
    fn snaps(&self) -> bool {
        !self.inside_mode()
    }

    fn label(&self) -> &'static str {
        self.shape.label()
    }

    fn prompt(&self) -> Prompt {
        let label = self.shape.label();
        let n = self.pts.len();
        if self.inside_mode() {
            return self.first_chips(Prompt::new(
                label,
                "alanı ölçülecek bölgenin içine tıklayın",
            ));
        }
        if n == 0 && self.shape == Shape::Adjoin {
            return Prompt::new(
                label,
                "ilk noktayı komşu alanın içinde ya da sınırında belirtin",
            );
        }
        if n == 0 {
            return self.first_chips(Prompt::new(label, "ilk noktayı belirtin"));
        }
        let done = n >= self.shape.min();
        // Rays from the first point: no arcs, no direction to go on in.
        if self.fixed() {
            let prompt = Prompt::new(label, "sonraki noktayı belirtin")
                .toggle("Sabit ilk nokta", "S", true)
                .option("Geri", "G");
            return if done {
                prompt.option("Bitir", "Enter")
            } else {
                prompt
            };
        }
        // G takes the point back from here too, rather than start Kapalı alan (docs/adr/0069).
        if self.ask_length {
            return Prompt::new(label, "son doğrultuda devam edilecek uzunluğu yazın")
                .option("Geri", "G");
        }
        if self.ask_step {
            return Prompt::new(label, "akışın adım boyunu yazın").option("Geri", "G");
        }
        if !self.arc_mode {
            let mut prompt = Prompt::new(label, "sonraki noktayı belirtin")
                .option("Yay", "Y")
                .option("Uzunluk", "U")
                .toggle("İzle", "İ", self.memory.trace)
                .toggle("Akış", "A", self.memory.stream);
            if self.memory.stream {
                prompt = prompt.option_with(
                    "Adım boyu",
                    "B",
                    self.format.length(self.memory.stream_step),
                );
            }
            // Dik kapat once a ring has three corners (docs/adr/0166 §4).
            if self.shape.closed() && n >= 3 {
                prompt = prompt.option("Dik kapat", "D");
            }
            let prompt = prompt.option("Geri", "G");
            return if done {
                prompt.option("Bitir", "Enter")
            } else {
                prompt
            };
        }
        let step = match self.spec {
            Spec::Angle { sweep: None } => {
                return Prompt::new(
                    label,
                    "yayın iç açısını derece olarak yazın (artı saat yönünün tersine)",
                );
            }
            Spec::Radius { r: None } => return Prompt::new(label, "yayın yarıçapını yazın"),
            Spec::Angle { .. } | Spec::Radius { .. } => "yayın bitiş noktasını belirtin",
            Spec::Centre { c: Some(_) } => "yayın bitiş doğrultusunu gösterin",
            Spec::Centre { c: None } => "yayın merkezini gösterin",
            Spec::Second { via: Some(_) } | Spec::Direction { dir: Some(_) } => {
                "yayın bitiş noktasını belirtin"
            }
            Spec::Second { via: None } => "yayın üzerinden geçeceği bir nokta belirtin",
            Spec::Direction { dir: None } => "yayın başlangıç doğrultusunu gösterin",
            Spec::Tangent if self.tangent().is_none() && self.arc_via.is_none() => {
                "yayın üzerinden geçeceği bir nokta belirtin"
            }
            Spec::Tangent => "yayın bitiş noktasını belirtin",
        };
        Self::arc_options(Prompt::new(label, step), done)
    }

    fn point_count(&self) -> usize {
        self.pts.len()
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        let point = self.constrain(p, cx);
        self.hover = Some(point);
        // Akış: the pointer leaves a vertex every step it goes, where it is.
        if self.streaming()
            && let Some(last) = self.last()
            && dist(last, p.raw) >= self.memory.stream_step
        {
            self.pts.push(p.raw);
            self.bulges.push(0.0);
        }
        // İzle's way to the cursor, for the preview.
        let end = self.end_for(point);
        self.way = match self.last() {
            Some(last) if self.tracing() => self.work.path(last, end, cx).map(|w| (last, end, w)),
            _ => None,
        };
        if self.inside_mode() {
            let region = self.face(point, cx);
            self.inside = Some((point, region));
        }
        self.refill(true, cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.pressed_at = Some(p.screen);
        let point = self.constrain(p, cx);
        self.accept(point, cx);
        self.pressed_at = None;
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        let key = upper_tr(js_trim(text));
        if self.switch(&key, cx) || self.option(&key, cx) {
            return true;
        }
        let number = parse_number(text);
        let plain = number.filter(|_| !text.contains([',', ';', '@', '<']));
        // Lengths are typed in the project's unit (docs/adr/0165 §2); an angle is not.
        let unit = cx.format();
        if let Some(n) = plain.filter(|_| self.ask_step) {
            if n > 0.0 {
                cx.memory.stream_step = unit.to_metres(n);
                self.memory.stream_step = unit.to_metres(n);
                self.ask_step = false;
            } else {
                cx.say(Level::Warn, "Adım boyu sıfırdan büyük olmalı.");
            }
            return true;
        }
        if let Some(n) = plain {
            if self.ask_length {
                match (self.tangent(), self.last()) {
                    (Some(t), Some(last)) if n > 0.0 => {
                        self.ask_length = false;
                        self.accept(offset_along(last, t, unit.to_metres(n)), cx);
                    }
                    _ => cx.say(Level::Warn, "Uzunluk sıfırdan büyük olmalı."),
                }
                return true;
            }
            if self.arc_mode
                && let Spec::Angle { sweep: None } = self.spec
            {
                if n.abs() < 1e-9 || n.abs() >= 360.0 {
                    cx.say(Level::Warn, "İç açı 0 ile ±360 derece arasında olmalı.");
                } else {
                    self.spec = Spec::Angle {
                        sweep: Some((n * PI) / 180.0),
                    };
                }
                return true;
            }
            if self.arc_mode
                && let Spec::Radius { r: None } = self.spec
            {
                if n > 0.0 {
                    self.spec = Spec::Radius {
                        r: Some(unit.to_metres(n)),
                    };
                } else {
                    cx.say(Level::Warn, "Yarıçap sıfırdan büyük olmalı.");
                }
                return true;
            }
        }
        // Object tracking has no line on the desktop yet: a bare number follows the cursor.
        match cx.typed_point(text, self.base(), self.hover) {
            Some(p) => {
                self.accept(p, cx);
                true
            }
            None => false,
        }
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.pts.is_empty() {
            return Flow::Exit;
        }
        self.finish(cx);
        Flow::Stay
    }

    fn undo_step(&mut self, cx: &mut Context<'_>) -> bool {
        // Newest first: the tool's own Geri (G), which takes back the last
        // point (or the arc's through-point); with no point, the drawing's undo.
        !self.pts.is_empty() && self.option("G", cx)
    }

    fn preview(&self, format: &Format) -> Preview {
        if self.inside_mode() {
            return self.inside_preview(format);
        }
        if self.fixed() {
            return self.fixed_preview(format);
        }
        let end = self.hover.map(|h| self.end_for(h));
        // İzle: the way along the line work in place of the straight segment,
        // while it still goes from the last point to the cursor.
        let way = self.preview_way(end);
        let hb = end
            .filter(|_| way.is_none())
            .and_then(|e| self.next_bulge(e));
        let (pts, mut bulges) = self.preview_path();
        bulges.push(0.0);
        let area_shape = self.closed() && pts.len() >= 3;
        let ring = area_shape.then(|| bulge_path_outline(&pts, Some(&bulges), true, DEFAULT_STEP));
        let path = bulge_path_outline(&pts, Some(&bulges), false, DEFAULT_STEP);
        let last = self.last();
        let mut guides = Vec::new();
        // Helper lines for points given before the end point.
        let guide = match self.spec {
            Spec::Second { via } => via,
            Spec::Centre { c } => c,
            _ => self.arc_via,
        };
        match (guide, last, self.hover) {
            (Some(guide), Some(last), _) => guides.push([last, guide]),
            (None, Some(last), Some(hover)) if self.arc_mode && hb.is_none() => {
                guides.push([last, hover]);
            }
            _ => {}
        }
        if let (Spec::Centre { c: Some(c) }, Some(hover)) = (self.spec, self.hover) {
            guides.push([c, hover]);
        }
        let tag = match (self.hover, last, end) {
            (Some(hover), Some(last), Some(end)) => {
                let arc = hb.and_then(|b| bulge_arc(last, end, b));
                let mut lines = match (way, arc) {
                    (Some(w), _) => vec![
                        format.length(w.length),
                        format!("İzle: {} köşe", w.pts.len().saturating_sub(2)),
                    ],
                    (None, Some(a)) => vec![
                        format!("Yay r {}", format.length(a.r)),
                        format!("Yay boyu {}", format.length(a.r * a.sweep.abs())),
                    ],
                    (None, None) => vec![
                        format.length(dist(last, end)),
                        format!("Semt {}", format.bearing(bearing_grad(last, end))),
                    ],
                };
                if self.shape == Shape::MeasureLength {
                    let total = bulge_path_length(&pts, Some(&bulges), false);
                    lines.push(format!("Toplam {}", format.length(total)));
                }
                if area_shape {
                    let area = bulge_ring_area(&pts, Some(&bulges)).abs();
                    lines.push(format!("Alan {}", format.area(area)));
                }
                // Bitişik alan: the path's length and the region's area.
                if self.shape == Shape::Adjoin {
                    let length = bulge_path_length(&pts, Some(&bulges), false);
                    lines.push(format!("Yol {}", format.length(length)));
                    if !self.region.is_empty() {
                        let area: f64 = self.region.iter().map(net_area).sum();
                        lines.push(format!("Alan {}", format.area(area)));
                    }
                }
                Some(Tag { at: hover, lines })
            }
            _ => None,
        };
        // The area last measured stays in view, dashed, until the next
        // measurement starts; Bitişik alan's region is filled.
        let areas = self
            .measured
            .iter()
            .map(|region| region_area(region, 0.1, Some([5.0, 4.0])))
            .chain(self.region.iter().map(|r| region_area(r, 0.16, None)))
            .collect();
        Preview {
            path,
            ring,
            guides,
            areas,
            tracking: tag.as_ref().and(self.tracking),
            tag,
            ..Preview::default()
        }
    }
}
