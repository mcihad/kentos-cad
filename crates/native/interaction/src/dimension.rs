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
//!   direction; they take no typed number.
//!
//! The style changes only while nothing is picked. The style, the lock and
//! Köşeden stay for as long as the app lives (`Memory`). The text is 2.5
//! paper mm high. Each dimension is written through `cad.entities.create`,
//! one undo step (“Ekle”). The layout and the placing are the shared core's.

use kentos_contracts::{DimensionStyle, Entity, EntityGeometry};
use kentos_domain::Document;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::dimension::{
    DimensionGeom, dimension_offset_at, layout_dimension, linear_angle_for, signed_offset,
};
use kentos_geometry_core::geom::intersect::{Edge, closest_on_edge, line_line};
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::tools::editing::{PickedEdge, edge_arms, radial_dimension, vertex_arms};
use kentos_geometry_core::tools::point_text::{js_trim, point_from_text};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge::{self, Outline};
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
const MODES: [(Mode, &str); 5] = [
    (Mode::Aligned, "H"),
    (Mode::Linear, "D"),
    (Mode::Angular, "A"),
    (Mode::Radius, "R"),
    (Mode::Diameter, "Ç"),
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
        }
    }
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
    }

    fn mode(&self) -> Mode {
        self.memory.dimension_mode
    }

    /// Nothing picked yet: the style can still change.
    fn fresh(&self) -> bool {
        self.d.pts.is_empty() && self.edges.is_empty() && self.circle.is_none()
    }

    /// Stages where a click picks an edge or a circle rather than a point.
    fn picks_edge(&self) -> bool {
        match self.mode() {
            Mode::Angular => !self.memory.dimension_by_vertex && self.edges.len() < 2,
            Mode::Radius | Mode::Diameter => self.circle.is_none(),
            Mode::Aligned | Mode::Linear => false,
        }
    }

    /// Whether the next point places the dimension line (or arc, or leader).
    fn placing(&self) -> bool {
        match self.mode() {
            Mode::Angular if self.memory.dimension_by_vertex => self.d.pts.len() == 3,
            Mode::Angular => self.edges.len() == 2,
            Mode::Radius | Mode::Diameter => self.circle.is_some(),
            Mode::Aligned | Mode::Linear => self.d.pts.len() == 2,
        }
    }

    /// Letter options (the web's `option`): a style while nothing is
    /// picked, Köşeden for the angle, the linear lock once two points are in.
    fn option(&mut self, key: &str, cx: &mut Context<'_>) -> bool {
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
        }
        if cx.memory.dimension_mode == Mode::Linear && self.d.pts.len() == 2 {
            cx.memory.dimension_lock = match key {
                "Y" => Some(0.0),
                "X" => Some(90.0),
                "O" => None,
                _ => return false,
            };
            return true;
        }
        false
    }

    /// A point given (the web's `accept`, then `onPoint`).
    fn accept(&mut self, p: Vec2, cx: &mut Context<'_>) {
        self.d.begin(p, cx);
        if !self.placing() {
            if self.d.last().is_none_or(|last| dist(last, p) > points::SAME) {
                self.d.pts.push(p);
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
                let arms = if self.memory.dimension_by_vertex {
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
        }
    }

    /// Writes the dimension (the web's `commit`): a degenerate one is said and
    /// the tool waits; written or refused, the next one starts.
    fn commit(&mut self, g: Option<DimensionGeom>, cx: &mut Context<'_>) {
        let Some((g, layout)) = g.and_then(|g| layout_dimension(&g).map(|l| (g, l))) else {
            cx.say(
                Level::Warn,
                "Bu yerde ölçü oluşmuyor; ölçülen noktalar çakışıyor ya da yay yarıçapı sıfır.",
            );
            return;
        };
        let mode = self.mode();
        let geometry = EntityGeometry::Dimension {
            a: wire(g.a),
            b: wire(g.b),
            offset: g.offset,
            height: g.height,
            text: None,
            style: mode.contract(),
            angle: g.angle,
            c: g.c.map(wire),
        };
        if points::write_objects(vec![geometry], None, cx).is_some() {
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
        self.d.reset();
    }
}

impl Tool for Dimension {
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
            Mode::Angular => match self.edges.len() {
                0 => "açı ölçüsü için birinci kenara tıklayın",
                1 => "ikinci kenara tıklayın",
                _ => "yayın yerini gösterin ya da yarıçap yazın",
            },
            Mode::Radius | Mode::Diameter if self.circle.is_some() => {
                "ölçünün doğrultusunu gösterin; daireden dışarı çekince yazı dışarı alınır"
            }
            Mode::Radius => "yarıçapı ölçülecek daireye ya da yaya tıklayın",
            Mode::Diameter => "çapı ölçülecek daireye ya da yaya tıklayın",
            Mode::Aligned => match n {
                0 => "hizalı ölçünün ilk noktasını belirtin",
                1 => "ikinci ölçü noktasını belirtin",
                _ => "ölçü çizgisinin yerini gösterin ya da mesafe yazın",
            },
        };
        let mut prompt = Prompt::new(LABEL, step);
        if self.mode() == Mode::Linear && n == 2 {
            let way = match self.memory.dimension_lock {
                Some(0.0) => "yatay",
                Some(_) => "düşey",
                None => "imleçten",
            };
            prompt = prompt
                .option("Yatay ΔY", "Y")
                .option("Düşey ΔX", "X")
                .option_with("Yön", "O", way);
        }
        if self.fresh() {
            if self.mode() == Mode::Angular {
                let other = if self.memory.dimension_by_vertex {
                    "Kenarlardan"
                } else {
                    "Köşeden"
                };
                prompt = prompt.option(other, "K");
            }
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

    fn snaps(&self) -> bool {
        !self.picks_edge()
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
            self.accept(point, cx);
            return;
        }
        let picked = edge::pick(p, cx, any).and_then(|slot| cx.doc.get(slot));
        if self.mode() == Mode::Angular {
            let Some((a, b)) = picked.and_then(|e| straight_edge_at(e, p.raw)) else {
                cx.say(
                    Level::Warn,
                    "Açının kenarı olarak düz bir çizgiye tıklayın; köşe noktasından ölçmek için “Köşeden” seçin.",
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
        let done = if self.option(&upper_tr(js_trim(text)), cx) {
            true
        } else if let Some(n) = points::plain_number(text).filter(|_| {
            self.placing() && !matches!(self.mode(), Mode::Radius | Mode::Diameter)
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
        } else if let Some(p) = point_from_text(text, self.d.last(), self.d.hover, |_| None) {
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
        if self.circle.take().is_some() || self.edges.pop().is_some() {
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
        // The points so far and the cursor, as every point tool draws them.
        Preview {
            strokes,
            ..points::chain_preview(&self.d, format)
        }
    }
}
