//! İfraz (docs/adr/1069): a parcel cut into a piece of a target size beside a chosen edge, the cut parallel to that
//! edge (the core's `ops::ifraz`). The reference edge is clicked on an area; the target area is typed; the cut, the two
//! pieces and their sizes are shown; Karşı taraf (T) gives the target to the piece across from the edge; Enter cuts
//! through `cad.areas.edit` as Alan böl does (one undo step). Esc and Ctrl+Z go back a step.
//!
//! Stage one: an area of straight edges with no holes, cut by one straight chord. The pieces are geometry: the parcel
//! register's ifraz document and the shares are recorded apart.

use kentos_commands::{AreaWork, Entity};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arrangement::{Area, Source};
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::geom::region::{net_area, split_area};
use kentos_geometry_core::ops::areas::area_of_entity;
use kentos_geometry_core::ops::edge_shift;
use kentos_geometry_core::ops::ifraz::{Cut, cut_for_area};
use kentos_geometry_core::tools::point_text::{js_trim, parse_number};
use kentos_native_application::geometry::shape;

use crate::Vec2;
use crate::edge;
use crate::format::Format;
use crate::log::Level;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{self, Context, Cursor, Flow, Pointer, Preview, Stroke, Tag, Tone, Tool};

/// The tool's id: its command is `tool.subdivide` (IFRAZ).
pub const ID: &str = "subdivide";
pub const LABEL: &str = "İfraz";
pub const NO_EDGE_HERE: &str = "Tıklanan yerde parsel kenarı yok; kilitsiz bir alanın kenarına tıklayın.";
pub const STRAIGHT_ONLY: &str =
    "İfraz şimdilik deliksiz, düz kenarlı tek parçalı alanda çalışır; yaylı ya da delikli parseli Alan böl ile elle kesin.";
/// How far the cut is drawn past the boundary, so that the command meets it there whatever the last digit.
const PAST: f64 = 0.01;

/// The parcel and its reference edge.
#[derive(Clone, Debug)]
struct Picked {
    slot: Slot,
    ring: Vec<Vec2>,
    edge: usize,
    a: Vec2,
    b: Vec2,
    area: Area,
    total: f64,
}

/// The cut asked for: the target typed, the cut, the two pieces (the edge's first) and their sizes.
#[derive(Clone, Debug)]
struct Asked {
    target: f64,
    cut: Cut,
    pieces: Vec<Area>,
}

#[derive(Default)]
pub struct Subdivide {
    picked: Option<Picked>,
    /// Karşı taraf (T): the target is the piece across from the edge.
    across: bool,
    asked: Option<Asked>,
    format: Format,
}

impl Subdivide {
    pub fn new() -> Self {
        Self::default()
    }

    /// The parcel's edge under the click, or why none is taken.
    fn pick(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(slot) = edge::pick_said(p, cx, editable) else {
            cx.say(Level::Warn, NO_EDGE_HERE);
            return;
        };
        let Some(e) = cx.doc.get(slot) else { return };
        let geom = shape(e);
        let (Shape::Polygon { pts, bulges, holes, .. }, Some(area)) = (&geom, area_of_entity(&geom)) else {
            cx.say(Level::Warn, NO_EDGE_HERE);
            return;
        };
        let curved = bulges.as_ref().is_some_and(|b| b.iter().any(|&v| v != 0.0));
        if curved || holes.as_ref().is_some_and(|h| !h.is_empty()) || kentos_geometry_core::entity::is_multi_part(&geom) {
            cx.say(Level::Warn, STRAIGHT_ONLY);
            return;
        }
        match edge_shift::pick(&geom, p.raw) {
            Ok(k) if k.ring == 0 => {
                self.picked = Some(Picked {
                    slot,
                    ring: pts.clone(),
                    edge: k.edge,
                    a: k.a,
                    b: k.b,
                    total: net_area(&area),
                    area,
                });
                self.asked = None;
                cx.selection.set_hover(None);
            }
            Ok(_) => cx.say(Level::Warn, STRAIGHT_ONLY),
            Err(why) => cx.say(Level::Warn, why),
        }
    }

    /// The cut for `target` m² (the piece beside the edge, or across with Karşı taraf), checked against the area the
    /// command cuts: two pieces, the target's within a micrometre of a square metre's millionth.
    fn ask(&mut self, target: f64, cx: &mut Context<'_>) {
        let Some(k) = &self.picked else { return };
        let beside = if self.across { k.total - target } else { target };
        let cut = match cut_for_area(&k.ring, k.edge, beside) {
            Ok(cut) => cut,
            Err(why) => {
                cx.say(Level::Warn, why);
                return;
            }
        };
        let mut pieces = split_area(&k.area, &line_source(&cut));
        if pieces.len() != 2 {
            cx.say(Level::Warn, kentos_geometry_core::ops::ifraz::MORE_PIECES);
            return;
        }
        // The piece beside the edge first: the one of the core's size, else (two equal halves) the one whose middle
        // lies nearer the edge's line.
        let off = |a: &Area| (net_area(a) - cut.near).abs();
        let gap = off(&pieces[1]) - off(&pieces[0]);
        let second = if gap.abs() > 1e-6 * k.total.max(1.0) {
            gap < 0.0
        } else {
            k.from_edge(middle(&pieces[1])) < k.from_edge(middle(&pieces[0]))
        };
        if second {
            pieces.swap(0, 1);
        }
        self.asked = Some(Asked { target, cut, pieces });
    }

    /// The pieces: the one asked for first.
    fn sizes(&self) -> Option<(f64, f64)> {
        let asked = self.asked.as_ref()?;
        let (near, far) = (asked.cut.near, asked.cut.far);
        Some(if self.across { (far, near) } else { (near, far) })
    }

    /// Enter: the parcel cut through the command, the pieces selected.
    fn write(&mut self, cx: &mut Context<'_>) -> Flow {
        let (Some(k), Some(asked), Some((wanted, rest))) = (&self.picked, &self.asked, self.sizes()) else {
            return Flow::Stay;
        };
        let (a, b) = past(&asked.cut);
        let work = AreaWork::Split {
            ids: vec![edge::uid(cx.doc, k.slot)],
            line: Some(vec![points::wire(a), points::wire(b)]),
            cutter: None,
        };
        let Some(out) = crate::area::write_work(work, cx) else {
            return Flow::Stay;
        };
        let created: Vec<Slot> = out
            .result
            .iter()
            .filter_map(|u| u.parse().ok())
            .filter_map(|u| cx.doc.slot_of(u))
            .collect();
        cx.selection.set(created);
        let format = cx.format();
        cx.say(
            Level::Success,
            format!(
                "{LABEL}: parsel ikiye bölündü: istenen {} ({}), kalan {}. Parçalar özgün alanın özniteliklerini taşır; parsel numaralarını ve ifraz kaydını ayrıca işleyin.",
                format.area(wanted),
                if self.across { "karşı taraf" } else { "kenar yanı" },
                format.area(rest)
            ),
        );
        Flow::Exit
    }
}

impl Picked {
    /// A point's distance from the reference edge's line.
    fn from_edge(&self, p: Vec2) -> f64 {
        let (dx, dy) = (self.b.x - self.a.x, self.b.y - self.a.y);
        ((p.x - self.a.x) * dy - (p.y - self.a.y) * dx).abs() / dx.hypot(dy)
    }
}

/// The middle of an area's outer ring (its vertices' mean: enough to tell two pieces apart across a straight cut).
fn middle(a: &Area) -> Vec2 {
    let n = a.outer.pts.len().max(1) as f64;
    let (x, y) = a.outer.pts.iter().fold((0.0, 0.0), |(x, y), p| (x + p.x, y + p.y));
    Vec2::new(x / n, y / n)
}

/// An area not on a locked layer: what the tool cuts.
fn editable(e: &Entity, doc: &Document) -> bool {
    matches!(e, Entity::Polygon(_)) && edge::unlocked(e, doc)
}

/// The cut drawn a little past the boundary at both ends, along its own line.
fn past(cut: &Cut) -> (Vec2, Vec2) {
    let (dx, dy) = (cut.b.x - cut.a.x, cut.b.y - cut.a.y);
    let l = dx.hypot(dy);
    let (ux, uy) = (dx / l * PAST, dy / l * PAST);
    (Vec2::new(cut.a.x - ux, cut.a.y - uy), Vec2::new(cut.b.x + ux, cut.b.y + uy))
}

/// The cut as the command cuts with it (Alan böl's straight line).
fn line_source(cut: &Cut) -> Source {
    let (a, b) = past(cut);
    Source {
        edges: vec![Edge::Seg { a, b }],
        points: Some(vec![a, b]),
        cut: Some(true),
    }
}

/// An area's rings as the preview draws them.
fn rings(a: &Area) -> Vec<Vec<Vec2>> {
    std::iter::once(&a.outer)
        .chain(&a.holes)
        .map(|r| kentos_geometry_core::entity::polygon_ring(&r.pts, r.bulges.as_deref()))
        .collect()
}

impl Tool for Subdivide {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        let side = if self.across { "karşı taraf" } else { "kenar yanı" };
        match (&self.picked, &self.asked) {
            (None, _) => Prompt::new(LABEL, "parselin referans kenarına tıklayın (kesim bu kenara paralel olur)"),
            (Some(_), None) => Prompt::new(
                LABEL,
                format!("ayrılacak alanı yazın ({})", self.format.area_unit_label()),
            )
            .option_with("Karşı taraf", "T", side),
            (Some(_), Some(_)) => Prompt::new(LABEL, "kesimi onaylamak için Enter'a basın ya da başka bir alan yazın")
                .option_with("Karşı taraf", "T", side)
                .option("Onayla", "Enter"),
        }
        .note("Esc: geri")
    }

    fn point_count(&self) -> usize {
        usize::from(self.picked.is_some())
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.format = cx.format();
        Flow::Stay
    }

    fn snaps(&self) -> bool {
        false
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.format = cx.format();
        if self.picked.is_none() {
            edge::hover(p, cx, editable);
        }
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        if self.picked.is_none() {
            self.pick(p, cx);
        }
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.picked.is_none() {
            return false;
        }
        let t = js_trim(text);
        if upper_tr(t) == "T" {
            self.across = !self.across;
            if let Some(target) = self.asked.as_ref().map(|a| a.target) {
                self.ask(target, cx);
            }
            return true;
        }
        match parse_number(t) {
            Some(n) if n.is_finite() => {
                let target = cx.format().area_to_square_metres(n);
                self.ask(target, cx);
            }
            _ => cx.say(Level::Warn, format!("Ayrılacak alan bir sayı olmalı; “{t}” yazıldı.")),
        }
        true
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        match (&self.picked, &self.asked) {
            (None, _) => Flow::Exit,
            (Some(_), None) => {
                cx.say(Level::Warn, "Önce ayrılacak alanı yazın.");
                Flow::Stay
            }
            (Some(_), Some(_)) => self.write(cx),
        }
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.undo_step(cx)
    }

    /// Ctrl+Z and Esc: the cut goes, then the edge.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.asked.take().is_some() {
            return true;
        }
        self.picked.take().is_some()
    }

    fn preview(&self, format: &Format) -> Preview {
        let mut out = Preview::default();
        let Some(k) = &self.picked else { return out };
        out.areas.push(tool::Area {
            rings: rings(&k.area),
            fill: 0.0,
            width: 2.0,
            dash: None,
            fill_tone: Tone::Accent,
        });
        out.strokes.push(Stroke::solid(vec![k.a, k.b], false).width(3.0).tone(Tone::Snap));
        let Some(asked) = &self.asked else {
            out.tag = Some(Tag {
                at: Vec2::new((k.a.x + k.b.x) / 2.0, (k.a.y + k.b.y) / 2.0),
                lines: vec![format!("parsel {}", format.area(k.total))],
            });
            return out;
        };
        // The piece asked for filled, the rest faint; the cut in the danger colour.
        let wanted = usize::from(self.across);
        for (i, piece) in asked.pieces.iter().enumerate() {
            out.areas.push(tool::Area {
                rings: rings(piece),
                fill: if i == wanted { 0.28 } else { 0.10 },
                width: 1.0,
                dash: Some([4.0, 3.0]),
                fill_tone: if i == wanted { Tone::Accent } else { Tone::Snap },
            });
        }
        out.strokes.push(Stroke::solid(vec![asked.cut.a, asked.cut.b], false).width(2.0).tone(Tone::Danger));
        if let Some((wanted, rest)) = self.sizes() {
            out.tag = Some(Tag {
                at: Vec2::new((asked.cut.a.x + asked.cut.b.x) / 2.0, (asked.cut.a.y + asked.cut.b.y) / 2.0),
                lines: vec![
                    format!("istenen {}", format.area(wanted)),
                    format!("kalan {}", format.area(rest)),
                    format!("kenara uzaklık {}", format.length(asked.cut.distance)),
                ],
            });
        }
        out
    }
}
