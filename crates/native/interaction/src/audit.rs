//! Denetle (docs/adr/0306; AutoCAD's AUDIT and PURGE): what the drawing carries broken and the named resources nothing
//! uses, shown first; on a confirm the broken objects repaired as one undo step and, when asked, the unused styles
//! removed.
//!
//! - **Found** by `cad.drawing.audit` over the whole drawing: linked texts, associative hatches and dimensions whose
//!   object is gone, texts and dimensions naming a style the project does not have; unused layers, groups, blocks and
//!   styles.
//! - **Options** (kept for the session): Onar (O: repair the broken objects; on), Yazı stilleri (Y: remove the unused
//!   text styles; off), Ölçü stilleri (Ö: remove the unused dimension styles; off).
//! - **Shown first:** the broken objects dashed in the danger colour, the counts on the command line, the unused named
//!   resources in the log. Enter, the Denetle button or a quick right click writes; with nothing to write Enter leaves;
//!   Esc leaves. Unused layers, groups and blocks are removed with Kullanılmayanları temizle.
//!
//! The plan and the write are the command's: the tool gives it the options.

use kentos_contracts::{
    AuditPurge, CommandResult, DrawingAudit, DrawingAudited, ResourceKind, UnusedResource,
};
use kentos_domain::Uuid;
use kentos_native_application::geometry::shape;
use kentos_native_application::{ExecutionContext, drawing_audit};

use crate::edge::Outline;
use crate::format::Format;
use crate::log::Level;
use crate::modify::MAX_GHOSTS;
use crate::points;
use crate::prompt::{Prompt, upper_tr};
use crate::tool::{Context, Cursor, Flow, Preview, Tone, Tool};

/// Denetle's id: its command is `tool.audit`.
pub const ID: &str = "audit";
pub const LABEL: &str = "Denetle";

/// The kinds' names in the counts, in the report's order.
const KINDS: [(ResourceKind, &str); 5] = [
    (ResourceKind::Layer, "katman"),
    (ResourceKind::Group, "grup"),
    (ResourceKind::Block, "blok"),
    (ResourceKind::TextStyle, "yazı stili"),
    (ResourceKind::DimensionStyle, "ölçü stili"),
];

/// What the audit found: `3 bozuk nesne (1 kilitli katmanda)`; none: empty.
fn broken(out: &DrawingAudited, fix: bool) -> String {
    let mut objects: Vec<&str> = Vec::new();
    let mut held: Vec<&str> = Vec::new();
    for f in &out.findings {
        if !objects.contains(&f.uid.as_str()) {
            objects.push(&f.uid);
            if fix && !f.fixed {
                held.push(&f.uid);
            }
        }
    }
    if objects.is_empty() {
        return String::new();
    }
    let mut text = format!("{} bozuk nesne", objects.len());
    if !held.is_empty() {
        text.push_str(&format!(" ({} kilitli katmanda)", held.len()));
    }
    text
}

/// The unused resources counted: `2 katman, 1 grup, 1 blok, 1 yazı stili`.
fn unused(list: &[UnusedResource]) -> String {
    KINDS
        .iter()
        .filter_map(|(kind, name)| {
            let n = list.iter().filter(|u| u.kind == *kind).count();
            (n > 0).then(|| format!("{n} {name}"))
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The unused resources by name for the log: `katman: Boş, Eski / Eski yol; blok: Eski blok`.
fn names(list: &[UnusedResource]) -> String {
    KINDS
        .iter()
        .filter_map(|(kind, name)| {
            let these: Vec<&str> = list
                .iter()
                .filter(|u| u.kind == *kind)
                .map(|u| u.name.as_str())
                .collect();
            (!these.is_empty()).then(|| format!("{name}: {}", these.join(", ")))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The tool.
#[derive(Clone, Debug, Default)]
pub struct Audit {
    plan: Option<DrawingAudited>,
    drawn: Preview,
    /// The options as last seen: Onar, Yazı stilleri, Ölçü stilleri.
    seen: [bool; 3],
    done: bool,
}

impl Audit {
    pub fn new() -> Self {
        Self::default()
    }

    fn request(cx: &Context<'_>) -> DrawingAudit {
        let m = &*cx.memory;
        let mut purge = Vec::new();
        if m.audit_purge_text {
            purge.push(AuditPurge::TextStyles);
        }
        if m.audit_purge_dims {
            purge.push(AuditPurge::DimensionStyles);
        }
        DrawingAudit {
            fix: m.audit_fix,
            purge,
            expected_revision: None,
        }
    }

    /// The command's plan for the options, drawn.
    fn replan(&mut self, cx: &mut Context<'_>) {
        let m = &*cx.memory;
        self.seen = [m.audit_fix, m.audit_purge_text, m.audit_purge_dims];
        self.drawn = Preview::default();
        self.plan = None;
        let input = Self::request(cx);
        match drawing_audit::plan_in(cx.doc, &input) {
            CommandResult::Completed { output, .. } => {
                let mut out = Outline::default();
                let mut seen: Vec<&str> = Vec::new();
                for f in &output.findings {
                    if seen.contains(&f.uid.as_str()) || seen.len() >= MAX_GHOSTS {
                        continue;
                    }
                    seen.push(&f.uid);
                    if let Some(e) = Uuid::parse_str(&f.uid)
                        .ok()
                        .and_then(|u| cx.doc.slot_of(u))
                        .and_then(|s| cx.doc.get(s))
                    {
                        out = out.and(Outline::of(&shape(e), Some([5.0, 3.0]), 2.0, Tone::Danger));
                    }
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

    /// Whether the write would change anything: a repair or a style removed.
    fn writes(&self) -> bool {
        self.plan
            .as_ref()
            .is_some_and(|p| !p.changed.is_empty() || p.unused.iter().any(|u| u.purged))
    }

    /// Writes the repair and the purge through the command.
    fn write(&mut self, cx: &mut Context<'_>) -> Flow {
        if !self.writes() {
            cx.say(
                Level::Info,
                "Denetle: onarılacak ya da temizlenecek bir şey yok.",
            );
            return Flow::Exit;
        }
        let input = Self::request(cx);
        let result = drawing_audit::execute(&mut ExecutionContext::new(cx.doc), input);
        let Some(out) = points::written(result, cx) else {
            return Flow::Stay;
        };
        let mut parts = Vec::new();
        if !out.changed.is_empty() {
            parts.push(format!("{} nesne onarıldı", out.changed.len()));
        }
        let purged: Vec<UnusedResource> = out.unused.iter().filter(|u| u.purged).cloned().collect();
        if !purged.is_empty() {
            parts.push(format!("{} temizlendi", unused(&purged)));
        }
        cx.say(Level::Success, format!("Denetle: {}.", parts.join("; ")));
        self.done = true;
        Flow::Exit
    }
}

impl Tool for Audit {
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
        let [fix, text, dims] = self.seen;
        let mut said = Vec::new();
        if let Some(p) = &self.plan {
            let b = broken(p, fix);
            if !b.is_empty() {
                said.push(b);
            }
            // The kinds are in the log; the line keeps short.
            if !p.unused.is_empty() {
                said.push(format!("{} kullanılmayan kaynak", p.unused.len()));
            }
        }
        let step = if said.is_empty() {
            "çizim sağlam; kullanılmayan kaynak yok".to_owned()
        } else {
            said.join("; ")
        };
        let on = |b: bool| if b { "açık" } else { "kapalı" };
        Prompt::new(LABEL, step)
            .then()
            .option("Denetle", "Enter")
            .option_with("Onar", "O", on(fix))
            .option_with("Yazı stilleri", "Y", on(text))
            .option_with("Ölçü stilleri", "Ö", on(dims))
    }

    fn point_count(&self) -> usize {
        0
    }

    /// Says what the command found: the counts, then the unused resources by name.
    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.replan(cx);
        let Some(p) = &self.plan else {
            return Flow::Exit;
        };
        let b = broken(p, cx.memory.audit_fix);
        let text = if b.is_empty() {
            "Denetle: bozuk ilişki ya da kayıp stil yok.".to_owned()
        } else {
            format!("Denetle: {b}; Enter ile onarın.")
        };
        cx.say(Level::Info, text);
        if !p.unused.is_empty() {
            cx.say(
                Level::Info,
                format!(
                    "Kullanılmayan {}. Stilleri Y ve Ö ile temizleyin; katman, grup ve blokları Kullanılmayanları temizle kaldırır.",
                    names(&p.unused)
                ),
            );
        }
        Flow::Stay
    }

    fn pointer_move(&mut self, _p: &crate::Pointer, _cx: &mut Context<'_>) {}

    /// A click does nothing: Enter, the button or a quick right click write.
    fn pointer_down(&mut self, _p: &crate::Pointer, _cx: &mut Context<'_>) {}

    /// An option's letter turns it on or off; then the plan is made again.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let m = &mut *cx.memory;
        match upper_tr(text.trim()).as_str() {
            "O" => m.audit_fix = !m.audit_fix,
            "Y" => m.audit_purge_text = !m.audit_purge_text,
            "Ö" => m.audit_purge_dims = !m.audit_purge_dims,
            _ => return false,
        }
        self.replan(cx);
        true
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.write(cx)
    }

    fn cancel(&mut self, _cx: &mut Context<'_>) -> bool {
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
