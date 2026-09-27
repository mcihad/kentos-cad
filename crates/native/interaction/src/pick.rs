//! Çizimden: one point on the drawing for a window that asked for it (the
//! web's `tools/pickPointTool.ts`, docs/adr/0070). Snaps and typed
//! coordinates work as in any command. The point goes to the host as a
//! view change ([`ViewChange::Picked`]). Esc, Enter or a quick right click
//! give none. The tool is not in the catalog and is not repeated
//! ([`crate::Session::run`]).

use kentos_geometry_core::tools::point_text::point_from_text;

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
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        self.title
    }

    /// “Poligon hesabı: Başlangıç noktası (A): haritada bir nokta gösterin ya da Y,X yazın [Vazgeç (Esc)]”.
    fn prompt(&self) -> Prompt {
        Prompt::new(
            self.title,
            format!(
                "{}: haritada bir nokta gösterin ya da Y,X yazın",
                self.field
            ),
        )
        .option("Vazgeç", "Esc")
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
        let Some(p) = point_from_text(text, None, self.hover, |_| None) else {
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
