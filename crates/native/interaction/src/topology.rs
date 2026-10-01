//! Topolojik temizlik (docs/adr/0148): line work and area outlines put right
//! within a tolerance the user gives, shown first and written in one undo
//! step. The finding is the shared core's (`topology_clean`); the web's tool
//! is `apps/web/src/tools/topologyTool.ts`, and both play
//! `fixtures/interaction/v1/topology.json`.
//!
//! - **Scope**, taken when it starts (§2): the selection, else every object
//!   on a visible, unlocked layer. The other objects on visible layers are
//!   supports: never moved, their vertices and edges are where the others go.
//!   Points never move; circles, ellipses and curves are edges only; texts,
//!   dimensions, hatches, blocks, leaders and construction lines take no
//!   part; a hidden layer takes none.
//! - **Tolerance and works** (§3), kept for as long as the app lives
//!   ([`crate::tool::Memory`]): a number typed is the tolerance (T asks for
//!   it); U, K, Z and B turn Uçlar, Köşeler, Uzat and Buda on or off.
//! - **Shown first** (§9): every change marked where it lands at a fixed size
//!   on the screen, the old outlines dashed under the new ones. Enter, the
//!   Uygula button or a quick right click writes through `cad.entities.edit`
//!   (`topology`), each geometry with its elevations as the core carried
//!   them (§6), and leaves; Esc leaves.

use std::collections::HashSet;

use kentos_contracts::{AreaPart, EditOperation, Entity, EntityEdit, EntityGeometry, RingGeometry};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::geom::arc::ArcGeom;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::ops::topology::{
    TopoObject, TopoPath, TopoResult, TopoWorks, arc_path, path_arc, topology_clean,
};
use kentos_native_application::elevation::paths as elevated_paths;
use kentos_native_application::geometry::{entity_of, shape};

use crate::Vec2;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::modify::MAX_GHOSTS;
use crate::points::plain_number;
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Marker, MarkerShape, Preview, Stroke, Tag, Tone, Tool};

/// Topolojik temizlik's id: its command is `tool.topology`.
pub const ID: &str = "topology";
pub const LABEL: &str = "Topolojik temizlik";

/// The tolerance before any is typed, metres (docs/adr/0148 §3).
pub const FIRST_TOLERANCE: f64 = 0.01;
/// The works before any is turned: Köşeler off, the others on.
pub const FIRST_WORKS: TopoWorks = TopoWorks {
    ends: true,
    vertices: false,
    extend: true,
    trim: true,
};
/// The smallest tolerance, metres: the core's “the same place”.
const LEAST: f64 = 1e-6;
/// The changes marked at most.
const MAX_MARKS: usize = 2000;
/// The marks, logical pixels (the web's `ringMark` and `markVertices`).
const RING_PX: f32 = 5.0;
const CROSS_PX: f32 = 3.5;
/// Where a moved vertex was.
const FROM_PX: f32 = 2.5;

/// What the cleanup would write for the drawing as it is, and what it was worked out from.
#[derive(Clone, Debug)]
struct Plan {
    generation: u64,
    tolerance: f64,
    works: TopoWorks,
    result: TopoResult,
    /// The objects that change and the geometry each takes, in the drawing's order.
    edits: Vec<(Slot, EntityGeometry)>,
    supports: usize,
    drawn: Preview,
}

/// The tool.
#[derive(Clone, Debug, Default)]
pub struct Topology {
    /// The objects it corrects, taken when it starts.
    targets: HashSet<Slot>,
    whole: bool,
    /// T: the tolerance is being typed.
    typing: bool,
    plan: Option<Plan>,
    /// The tolerance, the works and the units as of the last event, for the prompt.
    seen: Option<(f64, TopoWorks, Format)>,
    /// The finding last said, so a change that finds the same says nothing again.
    said: String,
    /// The cursor's world point: the tag beside it says the finding.
    hover: Option<Vec2>,
    done: bool,
}

impl Topology {
    pub fn new() -> Self {
        Self::default()
    }

    /// The objects to correct (docs/adr/0148 §2), what is left out said;
    /// false when there are none.
    fn take_scope(&mut self, cx: &mut Context<'_>) -> bool {
        let doc = &*cx.doc;
        let layers = doc.layers();
        self.whole = cx.selection.is_empty();
        let chosen: Vec<&Entity> = if self.whole {
            doc.entities().collect()
        } else {
            cx.selection
                .ids()
                .iter()
                .filter_map(|slot| doc.get(*slot))
                .collect()
        };
        let seen: Vec<&Entity> = chosen
            .into_iter()
            .filter(|e| layers.is_visible(&e.base().layer_id))
            .collect();
        let correctable: Vec<&Entity> = seen.iter().copied().filter(|e| corrected(e)).collect();
        let open: Vec<Slot> = correctable
            .iter()
            .filter(|e| !layers.is_locked(&e.base().layer_id))
            .map(|e| Slot(e.base().id))
            .collect();
        let locked = correctable.len() - open.len();
        let out = seen.iter().filter(|e| !takes_part(e)).count();
        self.targets = open.iter().copied().collect();
        if !self.whole && locked > 0 {
            cx.say(
                Level::Warn,
                format!(
                    "{locked} nesne kilitli katmanda olduğu için düzeltilmez; dayanak olarak kalır."
                ),
            );
        }
        if !self.whole && out > 0 {
            cx.say(
                Level::Warn,
                format!(
                    "{out} nesne topolojik temizliğe katılmaz: yazı, ölçü, tarama, blok, kılavuz ve yardımcı çizgiler girmez."
                ),
            );
        }
        if open.is_empty() {
            cx.say(
                Level::Warn,
                format!("{LABEL}: düzeltilecek çizgi, çoklu çizgi, yay ya da alan yok."),
            );
        }
        !open.is_empty()
    }

    /// The plan for the drawing as it is, kept until the drawing, the
    /// tolerance or the works change. Whether it was worked out again.
    fn refresh(&mut self, cx: &Context<'_>) -> bool {
        let (tolerance, works) = (cx.memory.topology_tolerance, cx.memory.topology_works);
        self.seen = Some((tolerance, works, cx.format()));
        let generation = cx.doc.generation();
        if self.plan.as_ref().is_some_and(|p| {
            p.generation == generation && p.tolerance == tolerance && p.works == works
        }) {
            return false;
        }
        self.plan = Some(self.work_out(cx.doc, tolerance, works, generation));
        true
    }

    /// Says the finding when it changed, so the history and the status bar
    /// keep the current one; when the tool starts, with its scope and what
    /// to do (the web's `tell`).
    fn tell(&mut self, cx: &mut Context<'_>, scope: Option<String>) {
        let Some(plan) = &self.plan else { return };
        let f = cx.format();
        let found = !plan.edits.is_empty();
        let text = if found {
            finding(&plan.result, &f)
        } else {
            format!(
                "{} toleransla düzeltilecek bir şey yok",
                f.length(plan.tolerance)
            )
        };
        if scope.is_none() && text == self.said {
            return;
        }
        let message = match (&scope, found) {
            (Some(scope), true) => format!("{LABEL}: {text}. Enter ile uygulayın ({scope})."),
            (Some(scope), false) => {
                format!("{LABEL}: {text}; daha büyük bir tolerans yazın ({scope}).")
            }
            (None, _) => format!("{LABEL}: {text}."),
        };
        cx.say(if found { Level::Info } else { Level::Warn }, message);
        self.said = text;
    }

    fn work_out(&self, doc: &Document, tolerance: f64, works: TopoWorks, generation: u64) -> Plan {
        let layers = doc.layers();
        let mut objects: Vec<TopoObject> = Vec::new();
        let mut taken: Vec<&Entity> = Vec::new();
        // Every object on a visible layer that takes part, in the drawing's order (ties go by it).
        for e in doc.entities() {
            let layer = &e.base().layer_id;
            if !layers.is_visible(layer) {
                continue;
            }
            let fixed = !self.targets.contains(&Slot(e.base().id)) || layers.is_locked(layer);
            if let Some(o) = object(e, fixed) {
                objects.push(o);
                taken.push(e);
            }
        }
        let supports = objects.iter().filter(|o| o.fixed).count();
        // The tolerance was checked when it was typed; an answer the core refuses changes nothing.
        let result = topology_clean(&objects, tolerance, works).unwrap_or(TopoResult {
            changed: Vec::new(),
            changes: Vec::new(),
            counts: Default::default(),
            max_shift: 0.0,
        });
        let mut edits = Vec::new();
        for c in &result.changed {
            if objects[c.object].paths == c.paths {
                continue;
            }
            let e = taken[c.object];
            if let Some(g) = geometry_of(e, &c.paths) {
                edits.push((Slot(e.base().id), g));
            }
        }
        let drawn = drawn(doc, &edits, &result);
        Plan {
            generation,
            tolerance,
            works,
            result,
            edits,
            supports,
            drawn,
        }
    }

    /// Writes the changes in one step and says what came of it.
    fn write(&mut self, cx: &mut Context<'_>) -> Flow {
        self.refresh(cx);
        let Some(plan) = self.plan.clone() else {
            return Flow::Exit;
        };
        if plan.edits.is_empty() {
            cx.say(
                Level::Warn,
                format!("{LABEL}: düzeltilecek bir şey yok; hiçbir şey değişmedi."),
            );
            return Flow::Exit;
        }
        let changes: Vec<EntityEdit> = plan
            .edits
            .iter()
            .map(|(slot, geometry)| EntityEdit::Update {
                uid: edge::uid(cx.doc, *slot),
                geometry: geometry.clone(),
            })
            .collect();
        if edge::write(EditOperation::Topology, changes, cx).is_none() {
            return Flow::Stay;
        }
        let f = cx.format();
        cx.say(
            Level::Success,
            format!(
                "{LABEL}: {}; {} nesne değişti, en büyük kayma {}.",
                done_text(&plan.result),
                plan.edits.len(),
                f.length(plan.result.max_shift)
            ),
        );
        self.done = true;
        Flow::Exit
    }
}

/// The kinds the cleanup corrects (docs/adr/0148 §2).
fn corrected(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Line(_) | Entity::Polyline(_) | Entity::Arc(_) | Entity::Polygon(_)
    )
}

/// The kinds that take part: the corrected ones and the supports' own.
fn takes_part(e: &Entity) -> bool {
    corrected(e)
        || matches!(
            e,
            Entity::Point(_) | Entity::Circle(_) | Entity::Ellipse(_) | Entity::Spline(_)
        )
}

/// `4 uç birleşir`, `1 uç uzar` …: the counts that are not naught, in the
/// words of now or, `past`, once written (docs/adr/0148 §9).
fn counted(r: &TopoResult, past: bool) -> Vec<String> {
    let c = &r.counts;
    [
        (c.ends, "uç birleşir", "uç birleşti"),
        (c.vertices, "köşe birleşir", "köşe birleşti"),
        (c.extended, "uç uzar", "uç uzadı"),
        (c.trimmed, "uç kısalır", "uç kısaldı"),
        (c.edges, "uç kenara taşınır", "uç kenara taşındı"),
    ]
    .iter()
    .filter(|(n, _, _)| *n > 0)
    .map(|(n, now, then)| format!("{n} {}", if past { then } else { now }))
    .collect()
}

/// The counts as the prompt says them: `4 uç birleşir, 1 uç uzar; en büyük kayma 0.030 m`.
fn finding(r: &TopoResult, f: &Format) -> String {
    format!(
        "{}; en büyük kayma {}",
        counted(r, false).join(", "),
        f.length(r.max_shift)
    )
}

/// The counts as the message after writing says them: `4 uç birleşti, 1 uç uzadı`.
fn done_text(r: &TopoResult) -> String {
    counted(r, true).join(", ")
}

/// The finding beside the cursor, a count a line, the largest move, and what writes it.
fn tag_lines(plan: &Plan, f: &Format) -> Vec<String> {
    if plan.edits.is_empty() {
        return vec!["Düzeltilecek bir şey yok".to_owned()];
    }
    let mut lines = counted(&plan.result, false);
    lines.push(format!(
        "en büyük kayma {}",
        f.length(plan.result.max_shift)
    ));
    lines.push("Enter: uygula".to_owned());
    lines
}

/// A path's bulges as the core takes them: one a vertex, when any bends; none when all are straight.
fn bulges_of(n: usize, bulges: Option<&[f64]>) -> Option<Vec<f64>> {
    let b = bulges?;
    b.iter()
        .any(|v| *v != 0.0)
        .then(|| (0..n).map(|i| b.get(i).copied().unwrap_or(0.0)).collect())
}

/// A circle as a boundary: two half turns (docs/adr/0148 §2).
fn circle_object(c: Vec2, r: f64) -> TopoObject {
    TopoObject {
        kind: "edges".into(),
        fixed: true,
        paths: vec![TopoPath {
            pts: vec![Vec2::new(c.x + r, c.y), Vec2::new(c.x - r, c.y)],
            bulges: Some(vec![1.0, 1.0]),
            closed: true,
            zs: vec![None, None],
        }],
    }
}

/// An edge of an ellipse's or a curve's outline (its 0.1 mm chords, docs/adr/0149 §5.3) as a path of its own.
fn edge_path(edge: &Edge) -> Option<TopoPath> {
    match *edge {
        Edge::Seg { a, b } => Some(TopoPath {
            pts: vec![a, b],
            bulges: None,
            closed: false,
            zs: vec![None, None],
        }),
        Edge::Arc { c, r, a0, sweep } => arc_path(&ArcGeom {
            c,
            r,
            a0,
            a1: a0 + sweep,
        }),
    }
}

fn v(p: &kentos_contracts::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

fn w(p: Vec2) -> kentos_contracts::Vec2 {
    kentos_contracts::Vec2 { x: p.x, y: p.y }
}

/// An object as the cleanup takes it (docs/adr/0148 §2), or none when it
/// takes no part: a line, a polyline and an area by their paths with their
/// elevations (`elevation::paths`' order), an arc as its two ends and a
/// bulge, a point (always fixed), a circle, an ellipse or a curve as edges
/// (always fixed). The web's is `topoObject` (`topologyTool.ts`).
pub fn object(e: &Entity, fixed: bool) -> Option<TopoObject> {
    let paths = || {
        elevated_paths(e)
            .into_iter()
            .map(|p| TopoPath {
                bulges: bulges_of(p.pts.len(), p.bulges.as_deref()),
                pts: p.pts,
                closed: p.closed,
                zs: p.zs,
            })
            .collect()
    };
    let kind = |k: &str, paths: Vec<TopoPath>| {
        Some(TopoObject {
            kind: k.into(),
            fixed,
            paths,
        })
    };
    match e {
        Entity::Line(_) => kind("line", paths()),
        Entity::Polyline(_) => kind("polyline", paths()),
        Entity::Polygon(_) => kind("area", paths()),
        Entity::Arc(a) => match arc_path(&ArcGeom {
            c: v(&a.c),
            r: a.r,
            a0: a.a0,
            a1: a.a1,
        }) {
            Some(path) => kind("arc", vec![path]),
            None => Some(circle_object(v(&a.c), a.r)),
        },
        Entity::Point(p) => Some(TopoObject {
            kind: "point".into(),
            fixed: true,
            paths: vec![TopoPath {
                pts: vec![v(&p.p)],
                bulges: None,
                closed: false,
                zs: vec![p.z],
            }],
        }),
        Entity::Circle(c) => Some(circle_object(v(&c.c), c.r)),
        Entity::Ellipse(_) | Entity::Spline(_) => Some(TopoObject {
            kind: "edges".into(),
            fixed: true,
            paths: entity_edges(&shape(e))
                .iter()
                .filter_map(edge_path)
                .collect(),
        }),
        _ => None,
    }
}

/// The geometry a corrected object takes, as `cad.entities.edit` writes it:
/// its paths back in `elevation::paths`' order (an area's outer ring, its
/// holes, then each other part's ring and holes), each with its elevations
/// explicit; an arc rebuilt from its ends and bulge. None for a kind the
/// cleanup does not correct. The web's is `geometryOf`.
pub fn geometry_of(e: &Entity, paths: &[TopoPath]) -> Option<EntityGeometry> {
    let pts = |p: &TopoPath| p.pts.iter().copied().map(w).collect::<Vec<_>>();
    let ring = |p: &TopoPath| RingGeometry {
        pts: pts(p),
        bulges: bulges_of(p.pts.len(), p.bulges.as_deref()),
        zs: Some(p.zs.clone()),
    };
    match e {
        Entity::Line(_) => {
            let p = paths.first()?;
            Some(EntityGeometry::Line {
                a: w(*p.pts.first()?),
                b: w(*p.pts.get(1)?),
                zs: Some(p.zs.clone()),
            })
        }
        Entity::Polyline(_) => {
            let p = paths.first()?;
            Some(EntityGeometry::Polyline {
                pts: pts(p),
                bulges: bulges_of(p.pts.len(), p.bulges.as_deref()),
                zs: Some(p.zs.clone()),
            })
        }
        Entity::Polygon(area) => {
            let mut next = paths.iter();
            let outer = next.next()?;
            let mut holes = Vec::new();
            for _ in area.holes.iter().flatten() {
                holes.push(ring(next.next()?));
            }
            let mut parts = Vec::new();
            for part in area.parts.iter().flatten() {
                let own = ring(next.next()?);
                let mut part_holes = Vec::new();
                for _ in part.holes.iter().flatten() {
                    part_holes.push(ring(next.next()?));
                }
                parts.push(AreaPart {
                    pts: own.pts,
                    bulges: own.bulges,
                    holes: (!part_holes.is_empty()).then_some(part_holes),
                    zs: own.zs,
                });
            }
            Some(EntityGeometry::Polygon {
                pts: pts(outer),
                bulges: bulges_of(outer.pts.len(), outer.bulges.as_deref()),
                holes: (!holes.is_empty()).then_some(holes),
                zs: Some(outer.zs.clone()),
                parts: (!parts.is_empty()).then_some(parts),
            })
        }
        Entity::Arc(_) => {
            let arc = path_arc(paths.first()?)?;
            Some(EntityGeometry::Arc {
                c: w(arc.c),
                r: arc.r,
                a0: arc.a0,
                a1: arc.a1,
            })
        }
        _ => None,
    }
}

/// Each changed object's old outline dashed in the danger colour under its
/// new one in the accent; at every change a mark of fixed size: a ring for
/// a vertex joined or an end moved onto a line (a small ring where it was
/// and a fine dashed line from there), a green ring and the piece added for
/// an end extended, a struck ring where an end is cut (the web's `draw`).
fn drawn(doc: &Document, edits: &[(Slot, EntityGeometry)], r: &TopoResult) -> Preview {
    let mut strokes = Vec::new();
    let mut marks = Vec::new();
    let mut markers = Vec::new();
    for (slot, geometry) in edits.iter().take(MAX_GHOSTS) {
        let Some(e) = doc.get(*slot) else { continue };
        for outline in [
            Outline::of(&shape(e), Some([5.0, 3.0]), 1.5, Tone::Danger),
            Outline::of(
                &shape(&entity_of(geometry, e.base().clone())),
                None,
                2.0,
                Tone::Accent,
            ),
        ] {
            strokes.extend(outline.strokes);
            marks.extend(outline.marks);
        }
    }
    let ring = |at: Vec2, tone: Tone| Marker {
        at,
        shape: MarkerShape::Ring(RING_PX),
        tone,
    };
    for c in r.changes.iter().take(MAX_MARKS) {
        match c.kind.as_str() {
            "extended" => {
                strokes.push(
                    Stroke::solid(vec![c.from, c.to], false)
                        .width(2.5)
                        .tone(Tone::Snap),
                );
                markers.push(ring(c.to, Tone::Snap));
            }
            "trimmed" => {
                markers.push(ring(c.to, Tone::Danger));
                markers.push(Marker {
                    at: c.to,
                    shape: MarkerShape::Cross(CROSS_PX),
                    tone: Tone::Danger,
                });
            }
            _ => {
                // Where the vertex was (a small ring in the danger colour) and the way to where it goes: seen close up.
                strokes.push(Stroke::dashed(vec![c.from, c.to], false, [3.0, 2.0]).width(1.0));
                markers.push(Marker {
                    at: c.from,
                    shape: MarkerShape::Ring(FROM_PX),
                    tone: Tone::Danger,
                });
                markers.push(ring(c.to, Tone::Accent));
            }
        }
    }
    Preview {
        strokes,
        marks,
        markers,
        ..Preview::default()
    }
}

/// Turns the work a letter names on or off: U Uçlar, K Köşeler, Z Uzat, B
/// Buda. False when the letter names none.
fn toggle(works: &mut TopoWorks, key: &str) -> bool {
    let flag = match key {
        "U" => &mut works.ends,
        "K" => &mut works.vertices,
        "Z" => &mut works.extend,
        "B" => &mut works.trim,
        _ => return false,
    };
    *flag = !*flag;
    true
}

impl Tool for Topology {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn snaps(&self) -> bool {
        false
    }

    fn prompt(&self) -> Prompt {
        let (tolerance, works, f) =
            self.seen
                .unwrap_or((FIRST_TOLERANCE, FIRST_WORKS, Format::default()));
        if self.typing {
            return Prompt::new(
                LABEL,
                format!("toleransı yazın, metre (Enter: {})", f.length(tolerance)),
            );
        }
        let step = match &self.plan {
            Some(plan) if !plan.edits.is_empty() => finding(&plan.result, &f),
            _ => "düzeltilecek bir şey yok".to_owned(),
        };
        let on = |b: bool| if b { "açık" } else { "kapalı" };
        Prompt::new(LABEL, step)
            .option_with("Tolerans", "T", f.length(tolerance))
            .option_with("Uçlar", "U", on(works.ends))
            .option_with("Köşeler", "K", on(works.vertices))
            .option_with("Uzat", "Z", on(works.extend))
            .option_with("Buda", "B", on(works.trim))
            .option("Uygula", "Enter")
    }

    fn point_count(&self) -> usize {
        0
    }

    /// Takes the scope and says what it found; with nothing to correct, says so and leaves.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        if !self.take_scope(cx) {
            return Flow::Exit;
        }
        self.refresh(cx);
        let Some(plan) = &self.plan else {
            return Flow::Exit;
        };
        let (n, supports) = (self.targets.len(), plan.supports);
        let scope = if self.whole {
            format!("bütün çizim: {n} nesne, {supports} dayanak")
        } else {
            format!("seçili {n} nesne, {supports} dayanak")
        };
        self.tell(cx, Some(scope));
        Flow::Stay
    }

    /// The tag follows the cursor; a drawing changed under the tool (an undo) is worked out again.
    fn pointer_move(&mut self, p: &crate::Pointer, cx: &mut Context<'_>) {
        self.hover = Some(p.raw);
        if self.refresh(cx) {
            self.tell(cx, None);
        }
    }

    /// A click does nothing: Enter, the button or a quick right click write.
    fn pointer_down(&mut self, _p: &crate::Pointer, _cx: &mut Context<'_>) {}

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let key = crate::prompt::upper_tr(crate::js_trim(text));
        if !self.typing {
            if toggle(&mut cx.memory.topology_works, &key) {
                self.refresh(cx);
                self.tell(cx, None);
                return true;
            }
            if key == "T" {
                self.typing = true;
                self.refresh(cx);
                return true;
            }
        }
        let Some(n) = plain_number(text) else {
            return false;
        };
        if !(n.is_finite() && n >= LEAST) {
            cx.say(Level::Warn, "Tolerans en az 0.000001 m olmalı.");
            return true;
        }
        cx.memory.topology_tolerance = n;
        self.typing = false;
        self.refresh(cx);
        self.tell(cx, None);
        true
    }

    /// Enter: the tolerance kept while it is typed; otherwise the changes written, and the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.typing {
            self.typing = false;
            self.refresh(cx);
            return Flow::Stay;
        }
        self.write(cx)
    }

    /// Esc while the tolerance is typed goes back to the finding; otherwise the tool leaves.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if !self.typing {
            return false;
        }
        self.typing = false;
        self.refresh(cx);
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn finished(&self) -> bool {
        self.done
    }

    /// What will change, and the finding beside the cursor.
    fn preview(&self, format: &Format) -> Preview {
        let Some(plan) = &self.plan else {
            return Preview::default();
        };
        Preview {
            tag: self.hover.map(|at| Tag {
                at,
                lines: tag_lines(plan, format),
            }),
            ..plan.drawn.clone()
        }
    }
}
