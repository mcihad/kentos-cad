//! Tüm köşeleri yuvarla, Tüm köşelere pah, Yönü çevir and Sadeleştir
//! (docs/adr/0140): four tools that reshape whole objects, on the desktop's
//! selection-first base ([`crate::modify`]), the way the modify tools go:
//!
//! - selected before or after (a click turns an object over, a window adds
//!   what it holds, a confirm goes on);
//! - then the result is drawn live where it will be: the rounded or cut
//!   outlines, the vertices that will go, the new direction with arrows;
//! - a typed value changes it at once and stays for as long as the app lives
//!   ([`crate::tool::Memory`]): a radius (Yuvarla), `d` or `d1,d2` (Pah), a
//!   tolerance in metres (Sadeleştir); Enter, the Uygula button or a quick
//!   right click writes, one undo step named after the tool; Esc leaves;
//! - objects on a locked layer are left out, with the web's warning; what the
//!   tool cannot reshape (a circle has no corners) is counted and said.
//!
//! What is written goes through `cad.entities.edit` (`fillet`, `chamfer`,
//! `reverse`, `simplify`): each changed object is updated with its whole new
//! geometry. The corners, the direction and the thinning are the shared
//! core's (`all_corners`, `reverse`, `simplify`).

use std::collections::HashSet;

use kentos_contracts::{EditOperation, EntityEdit};
use kentos_domain::Slot;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::ops::fillet::CornerOp;
use kentos_geometry_core::ops::reshape::{all_corners, reverse, simplify};

use crate::Vec2;
use crate::corner::parse_cut;
use crate::edge::{self, Outline};
use crate::format::Format;
use crate::log::Level;
use crate::modify::{MAX_GHOSTS, Modify, Stages};
use crate::object::ObjectAction;
use crate::points::plain_number;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Marker, MarkerShape, Memory, Preview, Stroke, Tag, Tone};

/// Tüm köşeleri yuvarla's id: its command is `tool.filletAll`.
pub const FILLET_ALL_ID: &str = "filletAll";
/// Tüm köşelere pah's id: its command is `tool.chamferAll`.
pub const CHAMFER_ALL_ID: &str = "chamferAll";
/// Yönü çevir's id: its command is `tool.reverse`.
pub const REVERSE_ID: &str = "reverse";
/// Sadeleştir's id: its command is `tool.simplify`.
pub const SIMPLIFY_ID: &str = "simplify";

/// The radius or the distance offered before any was typed, metres.
const FIRST_SIZE: f64 = 1.0;
/// The arrows that show a direction and the vertices that go, logical pixels.
const ARROW_PX: f64 = 14.0;
const CROSS_PX: f32 = 4.0;
const DASH: [f32; 2] = [5.0, 3.0];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    FilletAll,
    ChamferAll,
    Reverse,
    Simplify,
}

/// What the tool would write for the selection as it is now, and how to draw it.
#[derive(Clone, Debug, Default)]
struct Plan {
    /// Each object changed and the geometry it takes.
    edits: Vec<(Slot, Shape)>,
    /// Corners done, or vertices removed (Sadeleştir).
    done: usize,
    /// Corners that did not fit or lie beside an arc.
    skipped: usize,
    /// The largest distance of a removed vertex from the new outline, metres.
    deviation: f64,
    /// Objects of a kind the tool does not take.
    ignored: usize,
    /// Objects it takes that stay as they are.
    idle: usize,
    strokes: Vec<Stroke>,
    marks: Vec<Vec2>,
    markers: Vec<Marker>,
}

/// A plan and what it was made from: the drawing as it was and the value.
#[derive(Clone, Debug)]
struct Planned {
    generation: u64,
    value: [f64; 2],
    plan: Plan,
}

/// One of the four tools, after the selection.
#[derive(Clone, Debug)]
pub struct Reshape {
    kind: Kind,
    /// The selected objects the tool works on: not on a locked layer.
    targets: Vec<Slot>,
    /// What the session remembered and the project's units, as of the last event.
    seen: Option<(Memory, Format)>,
    planned: Option<Planned>,
}

impl Reshape {
    fn new(kind: Kind) -> Modify<Self> {
        Modify::with(Self {
            kind,
            targets: Vec::new(),
            seen: None,
            planned: None,
        })
    }

    pub fn fillet_all() -> Modify<Self> {
        Self::new(Kind::FilletAll)
    }

    pub fn chamfer_all() -> Modify<Self> {
        Self::new(Kind::ChamferAll)
    }

    pub fn reverse() -> Modify<Self> {
        Self::new(Kind::Reverse)
    }

    pub fn simplify() -> Modify<Self> {
        Self::new(Kind::Simplify)
    }

    fn operation(&self) -> EditOperation {
        match self.kind {
            Kind::FilletAll => EditOperation::Fillet,
            Kind::ChamferAll => EditOperation::Chamfer,
            Kind::Reverse => EditOperation::Reverse,
            Kind::Simplify => EditOperation::Simplify,
        }
    }

    /// The value the tool works with: the radius, the two distances, the tolerance.
    fn value(&self, m: &Memory) -> [f64; 2] {
        match self.kind {
            Kind::FilletAll => [m.fillet_radius.unwrap_or(FIRST_SIZE), 0.0],
            Kind::ChamferAll => {
                let (d1, d2) = m.chamfer.unwrap_or((FIRST_SIZE, FIRST_SIZE));
                [d1, d2]
            }
            Kind::Simplify => [m.simplify_tolerance, 0.0],
            Kind::Reverse => [0.0, 0.0],
        }
    }

    /// The value as the prompt says it: `yarıçap 1.000 m`, `pah 1.000 ile 2.000 m`.
    fn value_note(&self, m: &Memory, f: &Format) -> Option<String> {
        let [a, b] = self.value(m);
        match self.kind {
            Kind::FilletAll => Some(format!("yarıçap {}", f.length(a))),
            Kind::ChamferAll if a == b => Some(format!("pah {}", f.length(a))),
            Kind::ChamferAll => Some(format!("pah {} ile {}", f.length_bare(a), f.length(b))),
            Kind::Simplify => Some(format!("tolerans {}", f.length(a))),
            Kind::Reverse => None,
        }
    }

    /// Reads a typed value: `None` when the text is not one, `Some` when it
    /// was (a value that cannot be used is said, and the old one stays).
    fn take_value(&self, text: &str, cx: &mut Context<'_>) -> Option<()> {
        match self.kind {
            Kind::FilletAll => {
                let n = plain_number(text)?;
                if n > 0.0 {
                    cx.memory.fillet_radius = Some(n);
                } else {
                    cx.say(Level::Warn, "Yarıçap sıfırdan büyük olmalı.");
                }
            }
            Kind::ChamferAll => {
                let (d1, d2) = parse_cut(text)?;
                if d1 > 0.0 && d2 > 0.0 {
                    cx.memory.chamfer = Some((d1, d2));
                } else {
                    cx.say(Level::Warn, "Pah mesafeleri sıfırdan büyük olmalı.");
                }
            }
            Kind::Simplify => {
                let n = plain_number(text)?;
                if n > 0.0 {
                    cx.memory.simplify_tolerance = n;
                } else {
                    cx.say(Level::Warn, "Tolerans sıfırdan büyük olmalı.");
                }
            }
            Kind::Reverse => return None,
        }
        Some(())
    }

    /// What the tool would do to the targets. `draw` also draws it (the first
    /// [`MAX_GHOSTS`] objects); writing needs none of that.
    fn plan(&self, cx: &Context<'_>, draw: bool) -> Plan {
        let m = *cx.memory;
        let [a, b] = self.value(&m);
        let mut plan = Plan::default();
        let arrow = cx.view.world_length(ARROW_PX);
        for (i, slot) in self.targets.iter().enumerate() {
            let Some(e) = cx.doc.get(*slot) else { continue };
            let core = edge::core(e);
            let drawn = draw && i < MAX_GHOSTS;
            match self.kind {
                Kind::FilletAll | Kind::ChamferAll => {
                    let op = if self.kind == Kind::FilletAll {
                        CornerOp::Radius(a)
                    } else {
                        CornerOp::Chamfer(a, b)
                    };
                    let Some(r) = all_corners(&core, &op) else {
                        plan.ignored += 1;
                        continue;
                    };
                    plan.skipped += r.skipped;
                    if r.done == 0 {
                        plan.idle += 1;
                        if drawn && r.skipped > 0 {
                            // Nothing fits: the object stays, drawn as what is refused.
                            plan.add(Outline::of(&core.shape, Some(DASH), 1.5, Tone::Danger));
                        }
                        continue;
                    }
                    plan.done += r.done;
                    if drawn {
                        plan.add(Outline::of(&r.entity.shape, None, 2.5, Tone::Accent));
                    }
                    plan.edits.push((*slot, r.entity.shape));
                }
                Kind::Reverse => {
                    let Some(r) = reverse(&core) else {
                        plan.ignored += 1;
                        continue;
                    };
                    if drawn {
                        plan.direction(&r.shape, arrow);
                    }
                    plan.done += 1;
                    plan.edits.push((*slot, r.shape));
                }
                Kind::Simplify => {
                    let Some(r) = simplify(&core, a) else {
                        plan.ignored += 1;
                        continue;
                    };
                    if r.done == 0 {
                        plan.idle += 1;
                        continue;
                    }
                    plan.done += r.done;
                    plan.deviation = plan.deviation.max(r.deviation);
                    if drawn {
                        plan.add(Outline::of(&r.entity.shape, None, 2.5, Tone::Accent));
                        plan.removed(&core.shape, &r.entity.shape);
                    }
                    plan.edits.push((*slot, r.entity.shape));
                }
            }
        }
        plan
    }

    /// The plan for the selection as it is and the value as it is, kept until
    /// either changes.
    fn refresh(&mut self, cx: &Context<'_>) {
        self.seen = Some((*cx.memory, cx.format()));
        if self.targets.is_empty() {
            return;
        }
        let value = self.value(cx.memory);
        let generation = cx.doc.generation();
        if self
            .planned
            .as_ref()
            .is_some_and(|p| p.generation == generation && p.value == value)
        {
            return;
        }
        let plan = self.plan(cx, true);
        self.planned = Some(Planned {
            generation,
            value,
            plan,
        });
    }

    /// What the tag beside the cursor and the prompt say of a plan.
    fn summary(&self, plan: &Plan, f: &Format) -> Vec<String> {
        let count = |n: usize, what: &str| format!("{n} {what}");
        match self.kind {
            Kind::FilletAll | Kind::ChamferAll => {
                let verb = if self.kind == Kind::FilletAll {
                    "yuvarlanacak"
                } else {
                    "pahlanacak"
                };
                if plan.done == 0 {
                    return vec![if self.kind == Kind::FilletAll {
                        "Yuvarlanacak köşe yok".to_owned()
                    } else {
                        "Pahlanacak köşe yok".to_owned()
                    }];
                }
                let mut lines = vec![count(plan.done, &format!("köşe {verb}"))];
                if plan.skipped > 0 {
                    lines.push(count(plan.skipped, "köşe atlanacak"));
                }
                lines
            }
            Kind::Reverse => vec![count(plan.done, "nesnenin yönü çevrilecek")],
            Kind::Simplify => {
                if plan.done == 0 {
                    return vec!["Atılacak köşe yok".to_owned()];
                }
                vec![
                    count(plan.done, "köşe atılacak"),
                    format!("en büyük sapma {}", f.length(plan.deviation)),
                ]
            }
        }
    }

    /// What is said when the selection holds nothing the tool takes.
    fn nothing_to_take(&self) -> &'static str {
        match self.kind {
            Kind::FilletAll => {
                "Seçimde çoklu çizgi ya da kapalı alan yok; Tüm köşeleri yuvarla yalnız onların köşelerini yuvarlar."
            }
            Kind::ChamferAll => {
                "Seçimde çoklu çizgi ya da kapalı alan yok; Tüm köşelere pah yalnız onların köşelerini keser."
            }
            Kind::Reverse => {
                "Seçimde yönü çevrilebilecek çizgi, çoklu çizgi, eğri ya da kapalı alan yok."
            }
            Kind::Simplify => {
                "Seçimde çoklu çizgi ya da kapalı alan yok; Sadeleştir yalnız onların köşelerini atar."
            }
        }
    }

    /// What is said when the value changes nothing (the tool stays: another one may).
    fn changes_nothing(&self, plan: &Plan) -> String {
        match self.kind {
            Kind::FilletAll | Kind::ChamferAll if plan.skipped > 0 => format!(
                "Hiçbir köşe {}: {} köşe sığmadı ya da yaya komşu. Daha küçük bir değer yazın.",
                if self.kind == Kind::FilletAll {
                    "yuvarlanamadı"
                } else {
                    "pahlanamadı"
                },
                plan.skipped
            ),
            Kind::FilletAll | Kind::ChamferAll => {
                "Köşe yok: seçilen nesnelerde iki düz kenarın buluştuğu bir köşe bulunmuyor."
                    .to_owned()
            }
            _ => "Bu toleransla atılacak köşe yok; daha büyük bir tolerans yazın.".to_owned(),
        }
    }

    /// The message after the write, as the web's tools say theirs.
    fn done_message(&self, plan: &Plan, f: &Format) -> String {
        let mut text = match self.kind {
            Kind::FilletAll | Kind::ChamferAll => {
                let mut text = if self.kind == Kind::FilletAll {
                    format!("{} köşe yuvarlandı", plan.done)
                } else {
                    format!("{} köşeye pah kırıldı", plan.done)
                };
                if plan.skipped > 0 {
                    text.push_str(&format!(
                        "; {} köşe sığmadığı ya da yaya komşu olduğu için atlandı",
                        plan.skipped
                    ));
                }
                text.push('.');
                text
            }
            Kind::Reverse => format!("{} nesnenin yönü çevrildi.", plan.done),
            Kind::Simplify => format!(
                "Sadeleştir: {} köşe atıldı; en büyük sapma {}.",
                plan.done,
                f.length(plan.deviation)
            ),
        };
        if plan.ignored > 0 {
            let why = if self.kind == Kind::Reverse {
                "yönü olmadığı"
            } else {
                "çoklu çizgi ya da alan olmadığı"
            };
            text.push_str(&format!(" {} nesne {why} için atlandı.", plan.ignored));
        }
        text
    }
}

impl Plan {
    fn add(&mut self, outline: Outline) {
        self.strokes.extend(outline.strokes);
        self.marks.extend(outline.marks);
    }

    /// A reversed object: its outline, an arrow along it at half its length
    /// and a ring at its new start.
    fn direction(&mut self, shape: &Shape, arrow: f64) {
        let outline = Outline::of(shape, None, 1.0, Tone::Accent);
        if let Some(first) = outline.strokes.first() {
            if let Some(start) = first.pts.first() {
                self.markers.push(Marker {
                    at: *start,
                    shape: MarkerShape::Ring(4.0),
                    tone: Tone::Accent,
                });
            }
            if let Some(chevron) = chevron_at_half(&first.pts, first.closed, arrow) {
                self.strokes
                    .push(Stroke::solid(chevron, false).width(2.0).tone(Tone::Accent));
            }
        }
        self.add(outline);
    }

    /// The vertices of `old` that `new` does not have: crossed out.
    fn removed(&mut self, old: &Shape, new: &Shape) {
        let kept: HashSet<(u64, u64)> = corners(new)
            .iter()
            .map(|p| (p.x.to_bits(), p.y.to_bits()))
            .collect();
        for p in corners(old) {
            if !kept.contains(&(p.x.to_bits(), p.y.to_bits())) {
                self.markers.push(Marker {
                    at: p,
                    shape: MarkerShape::Cross(CROSS_PX),
                    tone: Tone::Danger,
                });
            }
        }
    }
}

/// The vertices of a path or an area, holes too.
fn corners(shape: &Shape) -> Vec<Vec2> {
    match shape {
        Shape::Polyline { pts, holes, .. } | Shape::Polygon { pts, holes, .. } => {
            let mut all = pts.clone();
            for ring in holes.iter().flatten() {
                all.extend(ring.pts.iter().copied());
            }
            all
        }
        _ => Vec::new(),
    }
}

/// An arrowhead of arm length `size` pointing along the path at half its
/// length: three points, wing, tip, wing.
fn chevron_at_half(pts: &[Vec2], closed: bool, size: f64) -> Option<Vec<Vec2>> {
    let mut edges: Vec<(Vec2, Vec2)> = pts.windows(2).map(|w| (w[0], w[1])).collect();
    if closed && let (Some(first), Some(last)) = (pts.first(), pts.last()) {
        edges.push((*last, *first));
    }
    let length = |(a, b): &(Vec2, Vec2)| ((b.x - a.x).powi(2) + (b.y - a.y).powi(2)).sqrt();
    let total: f64 = edges.iter().map(length).sum();
    if total.is_nan() || total <= 0.0 {
        return None;
    }
    let mut left = total / 2.0;
    for edge in &edges {
        let l = length(edge);
        if l <= 0.0 {
            continue;
        }
        if left <= l {
            let (a, b) = *edge;
            let (ux, uy) = ((b.x - a.x) / l, (b.y - a.y) / l);
            let tip = Vec2::new(a.x + ux * left, a.y + uy * left);
            // Each wing goes back from the tip and a little to its side.
            let wing = |side: f64| {
                Vec2::new(
                    tip.x - ux * size - uy * size * 0.5 * side,
                    tip.y - uy * size + ux * size * 0.5 * side,
                )
            };
            return Some(vec![wing(1.0), tip, wing(-1.0)]);
        }
        left -= l;
    }
    None
}

impl Stages for Reshape {
    fn id(&self) -> &'static str {
        match self.kind {
            Kind::FilletAll => FILLET_ALL_ID,
            Kind::ChamferAll => CHAMFER_ALL_ID,
            Kind::Reverse => REVERSE_ID,
            Kind::Simplify => SIMPLIFY_ID,
        }
    }

    fn label(&self) -> &'static str {
        match self.kind {
            Kind::FilletAll => "Tüm köşeleri yuvarla",
            Kind::ChamferAll => "Tüm köşelere pah",
            Kind::Reverse => "Yönü çevir",
            Kind::Simplify => "Sadeleştir",
        }
    }

    /// The selection is confirmed: its objects off locked layers are the
    /// targets, and the result is drawn. With nothing the tool takes, it says so and leaves.
    fn begin(&mut self, cx: &mut Context<'_>) -> Flow {
        self.targets = ObjectAction::targets(cx);
        self.planned = None;
        self.refresh(cx);
        let takes = self
            .planned
            .as_ref()
            .is_some_and(|p| p.plan.ignored < self.targets.len());
        if !takes {
            if !self.targets.is_empty() {
                cx.say(Level::Warn, self.nothing_to_take());
            }
            return Flow::Exit;
        }
        Flow::Stay
    }

    fn see(&mut self, cx: &Context<'_>) {
        self.refresh(cx);
    }

    /// The value the tool works with, and that a number changes it.
    fn picking_hint(&self, prompt: Prompt) -> Prompt {
        let (m, f) = self.seen.unwrap_or_default();
        match self.value_note(&m, &f) {
            Some(note) => prompt
                .note(note)
                .then()
                .note("değiştirmek için değer yazın"),
            None => prompt,
        }
    }

    fn picking_input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        self.take_value(text, cx).is_some()
    }

    fn anchor(&self) -> Option<Vec2> {
        None
    }

    fn prompt(&self, _n: usize) -> Prompt {
        let (m, f) = self.seen.unwrap_or_default();
        let step = match self.kind {
            Kind::FilletAll => "yarıçapı yazın",
            Kind::ChamferAll => "mesafeyi yazın: d ya da d1,d2",
            Kind::Reverse => "yeni yönü görün",
            Kind::Simplify => "toleransı yazın (m)",
        };
        let mut prompt = Prompt::new(self.label(), step);
        if let Some(note) = self.value_note(&m, &f) {
            prompt = prompt.note(note).then();
        }
        if let Some(planned) = &self.planned {
            let line = self.summary(&planned.plan, &f).join(", ");
            prompt = prompt.note(line).then();
        }
        prompt.option("Uygula", "Enter")
    }

    /// A click in the stages does nothing: Enter, the button or a quick right click write.
    fn point(&mut self, _p: Vec2, _cx: &mut Context<'_>) -> Flow {
        Flow::Stay
    }

    fn typed(&mut self, text: &str, cx: &mut Context<'_>) -> Option<Flow> {
        self.take_value(text, cx).map(|()| Flow::Stay)
    }

    fn typed_points(&self) -> bool {
        false
    }

    /// Writes what the preview showed, one undo step; a value that changes
    /// nothing says so and the tool stays.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        let plan = self.plan(cx, false);
        if plan.edits.is_empty() {
            let text = self.changes_nothing(&plan);
            cx.say(Level::Warn, text);
            return Flow::Stay;
        }
        let changes: Vec<EntityEdit> = plan
            .edits
            .iter()
            .filter_map(|(slot, shape)| {
                Some(EntityEdit::Update {
                    uid: edge::uid(cx.doc, *slot),
                    geometry: edge::geometry(shape)?,
                })
            })
            .collect();
        if edge::write(self.operation(), changes, cx).is_none() {
            return Flow::Stay;
        }
        let text = self.done_message(&plan, &cx.format());
        cx.say(Level::Success, text);
        Flow::Exit
    }

    fn preview(&self, _hover: Vec2) -> Option<kentos_geometry_core::geom::affine::Affine> {
        None
    }

    fn stage_preview(&self, hover: Option<Vec2>, format: &Format) -> Option<Preview> {
        let plan = &self.planned.as_ref()?.plan;
        let lines = self.summary(plan, format);
        let idle = plan.done == 0;
        Some(Preview {
            strokes: plan.strokes.clone(),
            marks: plan.marks.clone(),
            markers: plan.markers.clone(),
            tag: hover.map(|at| Tag { at, lines }),
            tag_tone: if idle { Tone::Danger } else { Tone::Accent },
            ..Preview::default()
        })
    }
}
