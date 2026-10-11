//! Bilgi (docs/adr/0225 §5; the web's `tools/identifyTool.ts`): a click on
//! an object opens its card beside the click, in the drawing: the host draws
//! it from [`ViewChange::InfoCard`] (its layer's card's title, attributes and
//! text, the object's attachments, the layer's actions). Another object's
//! click moves the card to it; a click on nothing and Esc close it; Esc again
//! leaves. The object is the click's first candidate (docs/adr/0187 §1).

use kentos_domain::Slot;
use kentos_native_application::geometry::shape;

use crate::edge::Outline;
use crate::format::Format;
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tone, Tool, ViewChange};

/// The tool's id: its command is `tool.identify`.
pub const ID: &str = "identify";
/// Its name, and the step it asks for.
pub const LABEL: &str = "Bilgi";
const STEP: &str = "bilgisi gösterilecek nesneye tıklayın";

/// Bilgi: the card's object, ringed.
#[derive(Clone, Debug, Default)]
pub struct IdentifyTool {
    shown: Option<Slot>,
    drawn: Preview,
}

impl IdentifyTool {
    fn show(&mut self, shown: Option<(Slot, crate::Vec2)>, cx: &mut Context<'_>) {
        self.shown = shown.map(|(s, _)| s);
        let mut preview = Preview::default();
        if let Some(e) = self.shown.and_then(|s| cx.doc.get(s)) {
            let outline = Outline::of(&shape(e), None, 2.5, Tone::Accent);
            preview.strokes.extend(outline.strokes);
            preview.marks.extend(outline.marks);
        }
        self.drawn = preview;
        cx.view_changes.push(ViewChange::InfoCard(shown));
    }
}

impl Tool for IdentifyTool {
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
        Prompt::new(LABEL, STEP)
    }

    fn point_count(&self) -> usize {
        0
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
        cx.selection.set_hover(hit);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let hit = cx.spatial.pick(p.raw, cx.pick_tolerance());
        self.show(hit.map(|s| (s, p.raw)), cx);
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        if self.shown.is_some() {
            self.show(None, cx);
        }
        Flow::Exit
    }

    /// Esc closes the card first; then it leaves.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        if self.shown.is_none() {
            cx.view_changes.push(ViewChange::InfoCard(None));
            return false;
        }
        self.show(None, cx);
        true
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        self.drawn.clone()
    }
}
