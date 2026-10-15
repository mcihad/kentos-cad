//! Çizimi temizle (docs/adr/0140, 0305; AutoCAD's OVERKILL): what the drawing carries twice, empty, on one line or
//! with vertices that add nothing, shown first, cleaned on a confirm in one undo step.
//!
//! - **Scope:** the selection; with nothing selected, every object on a visible, unlocked layer. A selected object on
//!   a locked layer is left out with a warning.
//! - **Found** by `cad.entities.cleanup` within the tolerance: empty objects and duplicates go (the first stays),
//!   lines on one line that overlap merge into the first, a path's repeated and straight vertices go. Duplicates with
//!   other data (attributes, label, attachments) are not duplicates.
//! - **Options** (kept for the session): Tolerans (T, typed next; 0.001 mm at first), Örtüşenler (Ö: lines that
//!   overlap merge; on), Uç uca (U: lines meeting end to end merge too; off), Köşeler (K: straight vertices go; on),
//!   Görünüş (G: duplicates may differ in colour, line type and its factor, weight, transparency and symbol; off).
//! - **Shown first:** what goes dashed in the danger colour, what changes in the accent, the counts and the farthest
//!   a point that goes lies from what stays on the command line. Enter, the Temizle button or a quick right click
//!   cleans; with nothing found Enter leaves; Esc leaves.
//!
//! The plan and the write are the command's: the tool gives it the scope and the options.

use kentos_contracts::{
    CleanupIgnore, CleanupReport, CommandResult, EntitiesCleanup, EntitiesCleanupPlan,
};
use kentos_domain::Uuid;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, entities_cleanup};

use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::modify::MAX_GHOSTS;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Preview, Tone, Tool};

/// Çizimi temizle's id: its command is `tool.cleanup`.
pub const ID: &str = "cleanup";
pub const LABEL: &str = "Çizimi temizle";

/// The look's properties Görünüş ignores.
const LOOK: [CleanupIgnore; 6] = [
    CleanupIgnore::Color,
    CleanupIgnore::LineType,
    CleanupIgnore::LineTypeScale,
    CleanupIgnore::LineWeight,
    CleanupIgnore::Transparency,
    CleanupIgnore::Symbol,
];

/// What the report says: `2 yinelenen, 1 boş nesne, 1 çizgi 1 çizgiye katılır, 3 köşe`.
fn counts(r: &CleanupReport) -> String {
    let mut parts = Vec::new();
    if r.duplicates > 0 {
        parts.push(format!("{} yinelenen", r.duplicates));
    }
    if r.empty > 0 {
        parts.push(format!("{} boş nesne", r.empty));
    }
    if r.merged > 0 {
        parts.push(format!("{} çizgi {} çizgiye katılır", r.merged, r.merging));
    }
    if r.vertices > 0 {
        parts.push(format!("{} köşe", r.vertices));
    }
    parts.join(", ")
}

/// What the write did: `1 yinelenen, 1 boş nesne silindi; 2 çizgi 1 çizgiye katıldı; 3 köşe atıldı`.
fn done_text(r: &CleanupReport) -> String {
    let mut gone = Vec::new();
    if r.duplicates > 0 {
        gone.push(format!("{} yinelenen", r.duplicates));
    }
    if r.empty > 0 {
        gone.push(format!("{} boş nesne", r.empty));
    }
    let mut parts = Vec::new();
    if !gone.is_empty() {
        parts.push(format!("{} silindi", gone.join(", ")));
    }
    if r.merged > 0 {
        parts.push(format!("{} çizgi {} çizgiye katıldı", r.merged, r.merging));
    }
    if r.vertices > 0 {
        parts.push(format!("{} köşe atıldı", r.vertices));
    }
    parts.join("; ")
}

/// The tool.
#[derive(Clone, Debug, Default)]
pub struct Cleanup {
    /// The selection's ids; none: the whole drawing.
    uids: Option<Vec<String>>,
    plan: Option<EntitiesCleanupPlan>,
    drawn: Preview,
    /// Asking for the tolerance.
    asking: bool,
    /// What the prompt shows: the tolerance and the options as last seen.
    seen: Option<(f64, [bool; 4])>,
    done: bool,
}

impl Cleanup {
    pub fn new() -> Self {
        Self::default()
    }

    fn request(&self, cx: &Context<'_>) -> EntitiesCleanup {
        let m = &*cx.memory;
        EntitiesCleanup {
            uids: self.uids.clone(),
            tolerance: Some(m.cleanup_tolerance),
            ignore: if m.cleanup_ignore_look {
                LOOK.to_vec()
            } else {
                Vec::new()
            },
            overlaps: Some(m.cleanup_overlaps),
            end_to_end: Some(m.cleanup_end_to_end),
            straight_vertices: Some(m.cleanup_straight),
            expected_revision: None,
        }
    }

    /// The command's plan for the scope and the options, drawn.
    fn replan(&mut self, cx: &mut Context<'_>) {
        let m = &*cx.memory;
        self.seen = Some((
            m.cleanup_tolerance,
            [
                m.cleanup_overlaps,
                m.cleanup_end_to_end,
                m.cleanup_straight,
                m.cleanup_ignore_look,
            ],
        ));
        self.drawn = Preview::default();
        self.plan = None;
        let input = self.request(cx);
        match entities_cleanup::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                let mut out = Outline::default();
                for uid in output.removed.iter().take(MAX_GHOSTS) {
                    if let Some(e) = Uuid::parse_str(uid)
                        .ok()
                        .and_then(|u| cx.doc.slot_of(u))
                        .and_then(|s| cx.doc.get(s))
                    {
                        out = out.and(Outline::of(&shape(e), Some([5.0, 3.0]), 2.0, Tone::Danger));
                    }
                }
                for e in output.changed.iter().take(MAX_GHOSTS) {
                    out = out.and(Outline::of(&shape(e), None, 2.0, Tone::Accent));
                }
                self.drawn.strokes = out.strokes;
                self.drawn.marks = out.marks;
                self.plan = Some(output);
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                cx.say(Level::Warn, error.message);
            }
            _ => {}
        }
    }

    fn found(&self) -> Option<&EntitiesCleanupPlan> {
        self.plan
            .as_ref()
            .filter(|p| !p.changed.is_empty() || !p.removed.is_empty())
    }

    /// Writes the cleaning through the command: one step.
    fn clean(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.found().is_none() {
            cx.say(
                Level::Info,
                "Çizimi temizle: temizlenecek bir şey bulunamadı; çizim temiz.",
            );
            return Flow::Exit;
        }
        let input = self.request(cx);
        let result = entities_cleanup::execute(&mut ExecutionContext::new(cx.doc), input);
        let Some(out) = points::written(result, cx) else {
            return Flow::Stay;
        };
        let doc = &*cx.doc;
        cx.selection.retain(|slot| doc.get(slot).is_some());
        let r = &out.report;
        let mut text = format!("Çizimi temizle: {}", done_text(r));
        if r.loss > 0.0 {
            text.push_str(&format!("; en büyük kayma {} m", r.loss));
        }
        text.push('.');
        cx.say(Level::Success, text);
        self.done = true;
        Flow::Exit
    }
}

impl Tool for Cleanup {
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
        let (tol, [overlaps, end_to_end, straight, look]) = self.seen.unwrap_or((
            kentos_contracts::CLEANUP_TOLERANCE,
            [true, false, true, false],
        ));
        if self.asking {
            return Prompt::new(LABEL, "toleransı yazın: bundan yakın noktalar bir sayılır")
                .note(format!("şimdi: {tol} m"));
        }
        let step = match self.found() {
            Some(p) => counts(&p.report),
            None => "temizlenecek bir şey yok".to_owned(),
        };
        let mut prompt = Prompt::new(LABEL, step);
        if let Some(p) = self.found()
            && p.report.loss > 0.0
        {
            prompt = prompt.note(format!("en büyük kayma {} m", p.report.loss));
        }
        let on = |b: bool| if b { "açık" } else { "kapalı" };
        prompt
            .then()
            .option("Temizle", "Enter")
            .option_with("Tolerans", "T", format!("{tol} m"))
            .option_with("Örtüşenler", "Ö", on(overlaps))
            .option_with("Uç uca", "U", on(end_to_end))
            .option_with("Köşeler", "K", on(straight))
            .option_with("Görünüş", "G", on(look))
    }

    fn point_count(&self) -> usize {
        0
    }

    /// Takes the scope and says what the command found.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.uids = (!cx.selection.is_empty()).then(|| {
            cx.selection
                .ids()
                .iter()
                .filter_map(|s| cx.doc.uid(*s))
                .map(|u| u.to_string())
                .collect()
        });
        if self.uids.is_none() {
            // The whole drawing leaves out what is locked or not shown: said.
            let layers = cx.doc.layers();
            let left = cx
                .doc
                .entities()
                .filter(|e| {
                    let l = &e.base().layer_id;
                    layers.is_locked(l) || !layers.is_visible(l)
                })
                .count();
            if left > 0 {
                cx.say(
                    Level::Warn,
                    format!(
                        "{left} nesne kilitli ya da gizli katmanda olduğu için dışarıda bırakıldı."
                    ),
                );
            }
        }
        self.replan(cx);
        let scope = match &self.uids {
            Some(u) => format!("seçili {} nesne", u.len()),
            None => "bütün çizim".to_owned(),
        };
        let text = match self.found() {
            Some(p) => format!(
                "Çizimi temizle: {}. Enter ile temizleyin ({scope}).",
                counts(&p.report)
            ),
            None => format!(
                "Çizimi temizle: bu toleransla temizlenecek bir şey bulunamadı ({scope}). Toleransı ya da seçenekleri değiştirebilirsiniz."
            ),
        };
        cx.say(Level::Info, text);
        Flow::Stay
    }

    fn pointer_move(&mut self, _p: &crate::Pointer, _cx: &mut Context<'_>) {}

    /// A click does nothing: Enter, the button or a quick right click clean.
    fn pointer_down(&mut self, _p: &crate::Pointer, _cx: &mut Context<'_>) {}

    /// An option's letter turns it on or off (Tolerans asks for a number); then the plan is made again.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        if self.asking {
            let Some((_, metres)) = points::plain_value(text, cx) else {
                return false;
            };
            if !(0.0..=kentos_contracts::MAX_CLEANUP_TOLERANCE).contains(&metres) {
                cx.say(Level::Warn, "Tolerans 0 ile 10 m arasında olmalı.");
                return true;
            }
            cx.memory.cleanup_tolerance = metres;
            self.asking = false;
            self.replan(cx);
            return true;
        }
        let m = &mut *cx.memory;
        match upper_tr(text.trim()).as_str() {
            "T" => {
                self.asking = true;
                return true;
            }
            "Ö" => m.cleanup_overlaps = !m.cleanup_overlaps,
            "U" => m.cleanup_end_to_end = !m.cleanup_end_to_end,
            "K" => m.cleanup_straight = !m.cleanup_straight,
            "G" => m.cleanup_ignore_look = !m.cleanup_ignore_look,
            _ => return false,
        }
        self.replan(cx);
        true
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.asking {
            self.asking = false;
            return Flow::Stay;
        }
        self.clean(cx)
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
        if self.asking {
            self.asking = false;
            return true;
        }
        false
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
