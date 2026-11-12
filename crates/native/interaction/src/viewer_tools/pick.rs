//! Çizimden: one point on the drawing for a window that asked for it (the
//! web's `tools/pickPointTool.ts`, docs/adr/0070). Snaps and typed
//! coordinates work as in any command. The point goes to the host as a
//! view change ([`ViewChange::Picked`]). Esc, Enter or a quick right click
//! give none. The tool is not in the catalog and is not repeated
//! ([`crate::Session::run`]).

use crate::Vec2;
use crate::format::Format;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Tool, ViewChange};

/// The tool's id, as the traces would read it.
pub const ID: &str = "pickPoint";

/// Asks for one point for a window.
#[derive(Clone, Debug)]
pub struct PickPoint {
    /// The window's title (“Poligon hesabı”), the prompt's name.
    title: &'static str,
    /// The field the point is for (“Başlangıç noktası (A)”).
    field: String,
    hover: Option<Vec2>,
    done: bool,
}

impl PickPoint {
    pub fn new(title: &'static str, field: impl Into<String>) -> Self {
        Self {
            title,
            field: field.into(),
            hover: None,
            done: false,
        }
    }

    /// The point, or none, goes to the host; the tool leaves after the call.
    fn finish(&mut self, p: Option<Vec2>, cx: &mut Context<'_>) {
        if !self.done {
            cx.view_changes.push(ViewChange::Picked(p));
            self.done = true;
        }
    }
}

impl Tool for PickPoint {
    /// A computed point is the one picked (the web's `acceptPoint`).
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.finish(Some(p), cx);
        true
    }
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        self.title
    }

    /// “Poligon hesabı: Başlangıç noktası (A): haritada bir nokta gösterin ya
    /// da Y,X yazın [Vazgeç (Esc)]”; without a title (a processing tool's
    /// point, the web's `PickPointTool`) the field's name leads.
    fn prompt(&self) -> Prompt {
        let step = format!(
            "{}: haritada bir nokta gösterin ya da Y,X yazın",
            self.field
        );
        let prompt = if self.title.is_empty() {
            Prompt::untitled(step)
        } else {
            Prompt::new(self.title, step)
        };
        prompt.option("Vazgeç", "Esc")
    }

    fn point_count(&self) -> usize {
        0
    }

    fn pointer_move(&mut self, p: &Pointer, _cx: &mut Context<'_>) {
        self.hover = Some(p.world);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.finish(Some(p.world), cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let Some(p) = cx.typed_point(text, None, self.hover) else {
            return false;
        };
        self.finish(Some(p), cx);
        true
    }

    /// Enter, Space or a quick right click: nothing picked, the window opens again as it was.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.finish(None, cx);
        Flow::Exit
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// Esc: nothing picked; the tool leaves.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.finish(None, cx);
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview::default()
    }

    fn finished(&self) -> bool {
        self.done
    }
}
