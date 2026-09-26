//! Köşe yuvarla and Pah: the web's `FilletTool` and `ChamferTool` on their
//! shared `CornerTool` (`apps/web/src/tools/cornerTools.ts`), step for step
//! (docs/adr/0047):
//!
//! - point at a corner (a polyline's or a closed area's vertex between two
//!   straight edges, or where two lines meet end to end): it is ringed;
//!   click it; or pick two lines (or two neighbouring edges of one polyline)
//!   one after the other, each keeping the side it was picked on;
//! - then pull the mouse along a side: the rounding or the cut grows live
//!   and a click applies it; a typed radius (Köşe yuvarla) or distance, `d`
//!   or `d1,d2` (Pah), applies exactly; Enter reuses the last one, or joins
//!   two separate lines at their meeting point;
//! - Kırp (K) off leaves the sides alone and only adds the arc or the cut;
//!   it, the last radius and the last distances stay for as long as the app
//!   lives ([`crate::tool::Memory`]);
//! - Esc steps back to finding a corner; with none picked, the tool leaves.
//!
//! The sides take their new geometry and the arc or the cut is a new object
//! from the first side, in one undo step through `cad.entities.edit`. The
//! corner found, its reach, the pulled size and every piece are the shared
//! core's (`corner_near`, `pulled_distance`, `corner_of_path`,
//! `fillet_lines`, `chamfer_lines`, `fillet_arc`, `chamfer_line`).

use kentos_contracts::{EditOperation, Entity, EntityEdit};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::bulge::bulge_at;
use kentos_geometry_core::ops::fillet::{
    Chamfer, Corner as Joined, CornerOp, CornerResult, Fillet, Seg, chamfer_lines, corner_of_path,
    fillet_lines,
};
use kentos_geometry_core::ops::vertex::nearest_segment;
use kentos_geometry_core::tools::drawing::offset_along;
use kentos_geometry_core::tools::editing::{
    CornerGeom, CornerSite, chamfer_line, corner_near, fillet_arc, fillet_radius_for,
    lines_corner_at, pulled_distance, vertex_corner,
};
use kentos_native_application::geometry::shape;

use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::points::plain_number;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{
    Context, Flow, Marker, MarkerShape, Memory, Pointer, Preview, Stroke, Tag, Tone, Tool,
};
use crate::{Vec2, js_trim};

/// The fillet tool's id: its command is `tool.fillet`.
pub const FILLET_ID: &str = "fillet";
/// The chamfer tool's id: its command is `tool.chamfer`.
pub const CHAMFER_ID: &str = "chamfer";

/// How near the cursor a corner is found, logical pixels (the web's `HOVER_PX`).
const HOVER_PX: f64 = 12.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Fillet,
    Chamfer,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Stage {
    Find,
    Second,
    Size,
}

/// Where the corner is, in the drawing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Site {
    /// Vertex `vertex` of the path at `slot`.
    Vertex { slot: Slot, vertex: usize },
    /// Two lines; each keeps the side of its pick point.
    Lines {
        first: Slot,
        pick1: Vec2,
        second: Slot,
        pick2: Vec2,
    },
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Corner {
    site: Site,
    geom: CornerGeom,
}

impl Corner {
    fn slots(&self) -> Vec<Slot> {
        match self.site {
            Site::Vertex { slot, .. } => vec![slot],
            Site::Lines { first, second, .. } => vec![first, second],
        }
    }
}

/// A size: a fillet's radius, or a chamfer's two distances.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Op {
    Radius(f64),
    Cut(f64, f64),
}

impl Op {
    fn core(self) -> CornerOp {
        match self {
            Op::Radius(r) => CornerOp::Radius(r),
            Op::Cut(d1, d2) => CornerOp::Chamfer(d1, d2),
        }
    }
}

/// What a size does to the corner: the sides' new geometry and the piece
/// added (the arc or the cut), made from the first side.
struct Plan {
    updates: Vec<(Slot, Shape)>,
    add: Option<(Slot, Shape)>,
}

/// The fillet or the chamfer tool.
#[derive(Clone, Debug)]
pub struct CornerTool {
    kind: Kind,
    stage: Stage,
    corner: Option<Corner>,
    /// The first line or edge picked, and where.
    first: Option<(Slot, Vec2)>,
    /// The pointer's world point, before snapping.
    mouse: Option<Vec2>,
    drawn: Preview,
    seen: Option<(Memory, Format)>,
}

/// The kinds the corner tools take, off locked layers.
fn editable(e: &Entity, doc: &Document) -> bool {
    matches!(
        e,
        Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
    ) && edge::unlocked(e, doc)
}

fn line_of(e: &Entity) -> Option<Seg> {
    match e {
        Entity::Line(l) => Some(Seg {
            a: Vec2::new(l.a.x, l.a.y),
            b: Vec2::new(l.b.x, l.b.y),
        }),
        _ => None,
    }
}

/// A path's corner at vertex `i` (the web's `pathCorner`): none at an open
/// path's end, beside an arc or on a straight run.
fn path_corner(slot: Slot, e: &Entity, i: usize) -> Option<Corner> {
    let (Entity::Polyline(path) | Entity::Polygon(path)) = e else {
        return None;
    };
    let closed = matches!(e, Entity::Polygon(_));
    let n = path.pts.len();
    if i >= n || (!closed && (i == 0 || i + 1 >= n)) {
        return None;
    }
    let prev = (i + n - 1) % n;
    let p = |k: usize| Vec2::new(path.pts[k].x, path.pts[k].y);
    let bulges = path.bulges.as_deref();
    let geom = vertex_corner(
        p(prev),
        p(i),
        p((i + 1) % n),
        bulge_at(bulges, prev),
        bulge_at(bulges, i),
    )?;
    Some(Corner {
        site: Site::Vertex { slot, vertex: i },
        geom,
    })
}

impl CornerTool {
    fn with(kind: Kind) -> Self {
        Self {
            kind,
            stage: Stage::Find,
            corner: None,
            first: None,
            mouse: None,
            drawn: Preview::default(),
            seen: None,
        }
    }

    pub fn fillet() -> Self {
        Self::with(Kind::Fillet)
    }

    pub fn chamfer() -> Self {
        Self::with(Kind::Chamfer)
    }

    fn title(&self) -> &'static str {
        match self.kind {
            Kind::Fillet => "Köşe yuvarla",
            Kind::Chamfer => "Pah",
        }
    }

    fn operation(&self) -> EditOperation {
        match self.kind {
            Kind::Fillet => EditOperation::Fillet,
            Kind::Chamfer => EditOperation::Chamfer,
        }
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
    }

    fn memory(&self) -> Memory {
        self.seen.map(|(m, _)| m).unwrap_or_default()
    }

    fn format(&self) -> Format {
        self.seen.map(|(_, f)| f).unwrap_or_default()
    }

    /// A chamfer's distances as the web says them: `2.000 m`, `1.500 ile 2.000 m`.
    fn cut_text(&self, d1: f64, d2: f64) -> String {
        let f = self.format();
        if d1 == d2 {
            f.length(d1)
        } else {
            format!("{} ile {}", f.length_bare(d1), f.length(d2))
        }
    }

    /// The last size, for Enter.
    fn last(&self) -> Option<Op> {
        let m = self.memory();
        match self.kind {
            Kind::Fillet => m.fillet_radius.map(Op::Radius),
            Kind::Chamfer => m.chamfer.map(|(d1, d2)| Op::Cut(d1, d2)),
        }
    }

    fn remember(&self, op: Op, cx: &mut Context<'_>) {
        match op {
            Op::Radius(r) => cx.memory.fillet_radius = Some(r),
            Op::Cut(d1, d2) => cx.memory.chamfer = Some((d1, d2)),
        }
    }

    /// The size a pull gives: a fillet's radius meets the side where the
    /// cursor is (tangent length), a chamfer cuts there.
    fn op_for_pull(&self, t: f64, c: &Corner) -> Op {
        match self.kind {
            Kind::Fillet => Op::Radius(fillet_radius_for(t, c.geom.phi)),
            Kind::Chamfer => Op::Cut(t, t),
        }
    }

    /// A typed size: a plain number above or at zero (Köşe yuvarla), `d` or
    /// `d1,d2` (Pah, the web's `/^(\d+(?:\.\d+)?)(?:\s*[,;]\s*(\d+(?:\.\d+)?))?$/`).
    fn op_for_text(&self, text: &str) -> Option<Op> {
        match self.kind {
            Kind::Fillet => plain_number(text).filter(|n| *n >= 0.0).map(Op::Radius),
            Kind::Chamfer => {
                let t = js_trim(text);
                let decimal = |s: &str| {
                    let (whole, frac) = match s.split_once('.') {
                        Some((w, f)) => (w, Some(f)),
                        None => (s, None),
                    };
                    let digits = |d: &str| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit());
                    (digits(whole) && frac.is_none_or(digits))
                        .then(|| s.parse::<f64>().ok())
                        .flatten()
                };
                match t.find([',', ';']) {
                    Some(at) => {
                        let d1 = decimal(t[..at].trim_end_matches([' ', '\t']))?;
                        let d2 = decimal(t[at + 1..].trim_start_matches([' ', '\t']))?;
                        Some(Op::Cut(d1, d2))
                    }
                    None => decimal(t).map(|d| Op::Cut(d, d)),
                }
            }
        }
    }

    fn describe(&self, op: Op) -> String {
        match op {
            Op::Radius(r) => format!("Yarıçap {}", self.format().length(r)),
            Op::Cut(d1, d2) => format!("Pah {}", self.cut_text(d1, d2)),
        }
    }

    fn done(&self, op: Op) -> String {
        match op {
            Op::Radius(r) if r > 0.0 => {
                format!("Köşe {} yarıçapla yuvarlandı.", self.format().length(r))
            }
            Op::Cut(d1, d2) if d1 > 0.0 || d2 > 0.0 => {
                format!("Pah kırıldı: {}.", self.cut_text(d1, d2))
            }
            _ => "Çizgiler köşede birleştirildi.".to_owned(),
        }
    }

    /// The nearest corner within reach of the cursor: path vertices and
    /// shared line ends among the editable objects around it, as the core
    /// finds it (`corner_near`, the web's `cornerAt`).
    fn corner_at(&self, p: &Pointer, cx: &Context<'_>) -> Option<Corner> {
        let tol = cx.view.world_length(HOVER_PX);
        let (a, b) = (
            Vec2::new(p.raw.x - tol, p.raw.y - tol),
            Vec2::new(p.raw.x + tol, p.raw.y + tol),
        );
        let doc = &*cx.doc;
        let near: Vec<(Slot, &Entity)> = cx
            .spatial
            .in_rect(a, b, true)
            .into_iter()
            .filter_map(|slot| Some((slot, doc.get(slot)?)))
            .filter(|(_, e)| editable(e, doc))
            .collect();
        let shapes: Vec<Shape> = near.iter().map(|(_, e)| shape(e)).collect();
        let hit = corner_near(&shapes, p.raw, tol, cx.view.world_length(2.0))?;
        let site = match hit.site {
            CornerSite::Vertex { object, vertex } => Site::Vertex {
                slot: near.get(object)?.0,
                vertex,
            },
            CornerSite::Lines {
                first,
                pick1,
                second,
                pick2,
            } => Site::Lines {
                first: near.get(first)?.0,
                pick1,
                second: near.get(second)?.0,
                pick2,
            },
        };
        Some(Corner {
            site,
            geom: hit.corner,
        })
    }

    /// The corner two picks name (the web's `cornerOfPicks`): two lines, or
    /// two neighbouring edges of one path; the reason when they do not.
    fn corner_of_picks(
        &self,
        a: (Slot, Vec2),
        b: (Slot, Vec2),
        cx: &Context<'_>,
    ) -> Result<Corner, &'static str> {
        let (Some(ea), Some(eb)) = (cx.doc.get(a.0), cx.doc.get(b.0)) else {
            return Err("Çizgiler paralel ya da seçilen tarafta çizgi yok.");
        };
        if let (Some(l1), Some(l2)) = (line_of(ea), line_of(eb)) {
            if a.0 == b.0 {
                return Err("İkinci çizgi ilkinden farklı olmalı.");
            }
            let geom = lines_corner_at(l1.a, l1.b, a.1, l2.a, l2.b, b.1)
                .ok_or("Çizgiler paralel ya da seçilen tarafta çizgi yok.")?;
            return Ok(Corner {
                site: Site::Lines {
                    first: a.0,
                    pick1: a.1,
                    second: b.0,
                    pick2: b.1,
                },
                geom,
            });
        }
        if a.0 != b.0 || matches!(ea, Entity::Line(_)) {
            return Err(
                "Çoklu çizgide köşe için köşenin kendisine ya da aynı nesnenin iki komşu kenarına tıklayın.",
            );
        }
        let (Entity::Polyline(path) | Entity::Polygon(path)) = ea else {
            return Err(
                "Çoklu çizgide köşe için köşenin kendisine ya da aynı nesnenin iki komşu kenarına tıklayın.",
            );
        };
        let n = path.pts.len();
        let s = shape(ea);
        let (i, j) = (nearest_segment(&s, a.1), nearest_segment(&s, b.1));
        let closed = matches!(ea, Entity::Polygon(_));
        let v = if j == i + 1 {
            Some(j)
        } else if i == j + 1 {
            Some(i)
        } else if closed && n > 0 && ((i == n - 1 && j == 0) || (j == n - 1 && i == 0)) {
            Some(0)
        } else {
            None
        };
        let v = v.ok_or("Seçilen kenarlar komşu değil; ortak köşesi olan iki kenar seçin.")?;
        path_corner(a.0, ea, v).ok_or(
            "Bu köşenin kenarlarından biri yay; yalnızca düz kenarlar arasındaki köşe işlenebilir.",
        )
    }

    /// What `op` does to the corner (the web's `Corner.plan`), or why it cannot.
    fn plan(&self, c: &Corner, op: Op, doc: &Document) -> Result<Plan, String> {
        match c.site {
            Site::Vertex { slot, vertex } => {
                let e = doc.get(slot).ok_or_else(String::new)?;
                let (Entity::Polyline(path) | Entity::Polygon(path)) = e else {
                    return Err(String::new());
                };
                let closed = matches!(e, Entity::Polygon(_));
                let pts: Vec<Vec2> = path.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
                match corner_of_path(&pts, path.bulges.as_deref(), closed, vertex, &op.core())? {
                    CornerResult::Error(error) => Err(error),
                    CornerResult::Path(p) => {
                        // The whole geometry is written (docs/adr/0047): a closed area keeps its holes.
                        let s = match shape(e) {
                            Shape::Polygon { holes, .. } => Shape::Polygon {
                                pts: p.pts,
                                bulges: p.bulges,
                                holes,
                            },
                            _ => Shape::Polyline {
                                pts: p.pts,
                                bulges: p.bulges,
                                holes: None,
                            },
                        };
                        Ok(Plan {
                            updates: vec![(slot, s)],
                            add: None,
                        })
                    }
                }
            }
            Site::Lines {
                first,
                pick1,
                second,
                pick2,
            } => {
                let l1 = doc.get(first).and_then(line_of).ok_or_else(String::new)?;
                let l2 = doc.get(second).and_then(line_of).ok_or_else(String::new)?;
                let line = |s: Seg| Shape::Line { a: s.a, b: s.b };
                let (joined, piece) = match op {
                    Op::Radius(r) => {
                        let Fillet(j) = fillet_lines(l1, pick1, l2, pick2, r);
                        match j {
                            Joined::Error(e) => return Err(e),
                            Joined::Ok { line1, line2, join } => (
                                (line1, line2),
                                join.map(|a| Shape::Arc {
                                    c: a.c,
                                    r: a.r,
                                    a0: a.a0,
                                    a1: a.a1,
                                }),
                            ),
                        }
                    }
                    Op::Cut(d1, d2) => {
                        let Chamfer(j) = chamfer_lines(l1, pick1, l2, pick2, d1, d2);
                        match j {
                            Joined::Error(e) => return Err(e),
                            Joined::Ok { line1, line2, join } => ((line1, line2), join.map(line)),
                        }
                    }
                };
                Ok(Plan {
                    updates: vec![(first, line(joined.0)), (second, line(joined.1))],
                    add: piece.map(|s| (first, s)),
                })
            }
        }
    }

    /// The arc or the cut alone, for Kırp: hayır.
    fn piece(&self, c: &Corner, op: Op) -> Option<Shape> {
        match op {
            Op::Radius(r) => fillet_arc(&c.geom, r).map(|a| Shape::Arc {
                c: a.c,
                r: a.r,
                a0: a.a0,
                a1: a.a1,
            }),
            Op::Cut(d1, d2) => {
                chamfer_line(&c.geom, d1, d2).map(|p| Shape::Line { a: p.a, b: p.b })
            }
        }
    }

    /// How far the cursor has been pulled along the nearer side, in steps
    /// that suit the zoom (four pixels), from the core.
    fn pulled_distance(&self, c: &Corner, cx: &Context<'_>) -> f64 {
        pulled_distance(&c.geom, self.mouse, cx.view.world_length(4.0))
    }

    fn lock(&mut self, c: Corner, cx: &mut Context<'_>) {
        self.corner = Some(c);
        self.stage = Stage::Size;
        cx.selection.set_hover(None);
    }

    fn reset(&mut self) {
        self.stage = Stage::Find;
        self.corner = None;
        self.first = None;
        self.drawn = Preview::default();
    }

    /// Writes the corner through `cad.entities.edit` (the web's `commit`): the
    /// sides take their whole new geometry, the arc or the cut is a new
    /// object from the first side. A refusal leaves the corner picked.
    fn commit(&mut self, op: Op, cx: &mut Context<'_>) {
        let Some(c) = self.corner else { return };
        let plan = match self.plan(&c, op, cx.doc) {
            Ok(plan) => plan,
            Err(error) => {
                if !error.is_empty() {
                    cx.say(Level::Warn, error);
                }
                return;
            }
        };
        if !self.memory().corner_trim {
            let Some(piece) = self.piece(&c, op) else {
                cx.say(
                    Level::Warn,
                    "Kırpmadan çalışırken sıfırdan büyük bir boyut verin; yoksa eklenecek bir şey yok.",
                );
                return;
            };
            let like = c.slots()[0];
            let from = edge::uid(cx.doc, like);
            let Some(geometry) = edge::geometry(&piece) else {
                return;
            };
            let change = EntityEdit::Add {
                from,
                geometry,
                keep_data: None,
            };
            if edge::write(self.operation(), vec![change], cx).is_none() {
                return;
            }
            self.remember(op, cx);
            let text = format!("{} Kenarlar kırpılmadı.", self.done(op));
            cx.say(Level::Success, text);
            return self.reset();
        }
        let mut changes = Vec::new();
        for (slot, s) in &plan.updates {
            let uid = edge::uid(cx.doc, *slot);
            if let Some(geometry) = edge::geometry(s) {
                changes.push(EntityEdit::Update { uid, geometry });
            }
        }
        if let Some((like, s)) = &plan.add {
            let from = edge::uid(cx.doc, *like);
            if let Some(geometry) = edge::geometry(s) {
                changes.push(EntityEdit::Add {
                    from,
                    geometry,
                    keep_data: None,
                });
            }
        }
        if edge::write(self.operation(), changes, cx).is_none() {
            return;
        }
        self.remember(op, cx);
        let text = self.done(op);
        cx.say(Level::Success, text);
        self.reset();
    }

    /// The preview (the web's `draw`): the first pick; or the corner ringed,
    /// its objects, and with a size the reach along both sides, the result
    /// and the size beside the cursor.
    fn redraw(&mut self, cx: &Context<'_>) {
        self.drawn = Preview::default();
        let doc = &*cx.doc;
        if self.stage == Stage::Second {
            if let Some(e) = self.first.and_then(|(slot, _)| doc.get(slot)) {
                let out = Outline::of(&shape(e), None, 2.0, Tone::Accent);
                self.drawn.strokes = out.strokes;
            }
            return;
        }
        let Some(c) = self.corner else { return };
        let mut out = Outline::default();
        for slot in c.slots() {
            if let Some(e) = doc.get(slot) {
                out = out.and(Outline::of(&shape(e), None, 1.5, Tone::Accent));
            }
        }
        self.drawn.markers.push(Marker {
            at: c.geom.at,
            shape: MarkerShape::Ring(7.0),
            tone: Tone::Accent,
        });
        if self.stage == Stage::Find {
            self.drawn.strokes = out.strokes;
            self.drawn.tag = Some(Tag {
                at: c.geom.at,
                lines: vec!["Köşeyi seçmek için tıklayın".to_owned()],
            });
            return;
        }
        let t = self.pulled_distance(&c, cx);
        let op = self.op_for_pull(t, &c);
        // Where the rounding or the cut starts on each side.
        for u in [c.geom.u1, c.geom.u2] {
            out.strokes.push(
                Stroke::solid(vec![c.geom.at, offset_along(c.geom.at, u, t)], false)
                    .width(2.0)
                    .tone(Tone::Snap),
            );
        }
        if let Ok(plan) = self.plan(&c, op, doc) {
            if !self.memory().corner_trim {
                if let Some(piece) = self.piece(&c, op) {
                    out = out.and(Outline::of(&piece, None, 2.5, Tone::Accent));
                }
            } else {
                for (_, s) in &plan.updates {
                    out = out.and(Outline::of(s, Some([5.0, 3.0]), 2.0, Tone::Accent));
                }
                if let Some((_, s)) = &plan.add {
                    out = out.and(Outline::of(s, None, 2.5, Tone::Accent));
                }
            }
            if let Some(mouse) = self.mouse {
                self.drawn.tag = Some(Tag {
                    at: mouse,
                    lines: vec![self.describe(op), "Tıklayın: uygula".to_owned()],
                });
            }
        }
        self.drawn.strokes = out.strokes;
        self.drawn.marks = out.marks;
    }
}

impl Tool for CornerTool {
    fn id(&self) -> &'static str {
        match self.kind {
            Kind::Fillet => FILLET_ID,
            Kind::Chamfer => CHAMFER_ID,
        }
    }

    fn label(&self) -> &'static str {
        self.title()
    }

    fn prompt(&self) -> Prompt {
        let m = self.memory();
        let f = self.format();
        let trim = if m.corner_trim { "evet" } else { "hayır" };
        let title = self.title();
        match (self.stage, self.kind) {
            (Stage::Second, _) => Prompt::new(
                title,
                "ikinci çizgiyi ya da aynı çoklu çizginin komşu kenarını seçin",
            ),
            (Stage::Find, Kind::Fillet) => {
                let p = Prompt::new(
                    title,
                    "yuvarlanacak köşeye tıklayın ya da sırayla iki çizgi seçin",
                );
                let p = match m.fillet_radius {
                    Some(r) => p.note(format!("son yarıçap {}", f.length(r))),
                    None => p,
                };
                p.option_with("Kırp", "K", trim)
            }
            (Stage::Find, Kind::Chamfer) => {
                let p = Prompt::new(
                    title,
                    "kesilecek köşeye tıklayın ya da sırayla iki çizgi seçin",
                );
                let p = match m.chamfer {
                    Some((d1, d2)) => p.note(format!("son mesafe {}", self.cut_text(d1, d2))),
                    None => p,
                };
                p.option_with("Kırp", "K", trim)
            }
            (Stage::Size, Kind::Fillet) => {
                let p = Prompt::new(
                    title,
                    "fareyi kenar boyunca kaydırıp tıklayın ya da yarıçap yazın",
                );
                let p = match m.fillet_radius {
                    Some(r) => p.option_with("Son yarıçap", "Enter", f.length(r)),
                    None => p,
                };
                p.option_with("Kırp", "K", trim)
            }
            (Stage::Size, Kind::Chamfer) => {
                let p = Prompt::new(
                    title,
                    "fareyi kenar boyunca kaydırıp tıklayın ya da mesafe yazın (d ya da d1,d2)",
                );
                let p = match m.chamfer {
                    Some((d1, d2)) => p.option_with("Son mesafe", "Enter", self.cut_text(d1, d2)),
                    None => p,
                };
                p.option_with("Kırp", "K", trim)
            }
        }
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        false
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.mouse = Some(p.raw);
        if self.stage == Stage::Find {
            self.corner = self.corner_at(p, cx);
            if self.corner.is_some() {
                cx.selection.set_hover(None);
                return self.redraw(cx);
            }
        }
        if self.stage != Stage::Size {
            edge::hover(p, cx, editable);
        }
        self.redraw(cx);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.see(cx);
        self.mouse = Some(p.raw);
        if self.stage == Stage::Size {
            if let Some(c) = self.corner {
                let op = self.op_for_pull(self.pulled_distance(&c, cx), &c);
                self.commit(op, cx);
            }
            self.see(cx);
            return self.redraw(cx);
        }
        if self.stage == Stage::Find
            && let Some(c) = self.corner_at(p, cx)
        {
            self.lock(c, cx);
            return self.redraw(cx);
        }
        let Some(slot) = edge::pick(p, cx, editable) else {
            cx.say(
                Level::Warn,
                "Bir köşeye ya da düzenlenebilir bir çizgi veya çoklu çizgi kenarına tıklayın.",
            );
            return;
        };
        if self.stage == Stage::Find {
            self.first = Some((slot, p.raw));
            self.stage = Stage::Second;
            cx.selection.set_hover(None);
            return self.redraw(cx);
        }
        let Some(first) = self.first else { return };
        match self.corner_of_picks(first, (slot, p.raw), cx) {
            Err(error) => cx.say(Level::Warn, error),
            Ok(c) => {
                self.lock(c, cx);
                self.redraw(cx);
            }
        }
    }

    /// K turns Kırp over (not while the second line is picked); in the size
    /// step a typed size applies at once.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        if upper_tr(js_trim(text)) == "K" && self.stage != Stage::Second {
            cx.memory.corner_trim = !cx.memory.corner_trim;
            self.see(cx);
            self.redraw(cx);
            return true;
        }
        if self.stage != Stage::Size {
            return false;
        }
        let Some(op) = self.op_for_text(text) else {
            return false;
        };
        self.commit(op, cx);
        self.see(cx);
        self.redraw(cx);
        true
    }

    /// In the size step: the last size, or two separate lines joined at their
    /// meeting point. Anywhere else the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.see(cx);
        if self.stage != Stage::Size {
            return Flow::Exit;
        }
        let Some(c) = self.corner else {
            return Flow::Exit;
        };
        let fallback = matches!(c.site, Site::Lines { .. }).then(|| self.op_for_pull(0.0, &c));
        match self.last().or(fallback) {
            Some(op) => self.commit(op, cx),
            None => cx.say(
                Level::Warn,
                "Önce fareyle boyutu gösterip tıklayın ya da bir değer yazın.",
            ),
        }
        self.see(cx);
        self.redraw(cx);
        Flow::Stay
    }

    /// Esc steps back to finding a corner; with none picked, the tool leaves.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.see(cx);
        if self.stage == Stage::Find {
            return false;
        }
        self.reset();
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
