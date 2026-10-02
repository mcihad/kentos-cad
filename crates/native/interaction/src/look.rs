//! A look at the drawing for a window that stepped aside (Kenar eşleme's
//! Göster, docs/adr/0159 §9; the web's `tools/lookTool.ts`): the view may be
//! moved, nothing is picked. A click, Enter, a quick right click or Esc ends
//! it; the host hears it as [`ViewChange::Picked`] with no point and brings
//! the window back. The tool is not in the catalog and is not repeated
//! ([`crate::Session::run`]).

use crate::format::Format;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Tool, ViewChange};

/// The tool's id, as the traces would read it.
pub const ID: &str = "look";

/// A look at the drawing until a click or a key.
#[derive(Clone, Debug)]
pub struct Look {
    /// The window's title (“Kenar eşleme”), the prompt's name.
    title: &'static str,
    /// What is looked at (“3. bağ, aralık 85,4 mm”).
    what: String,
    done: bool,
}

impl Look {
    pub fn new(title: &'static str, what: impl Into<String>) -> Self {
        Self {
            title,
            what: what.into(),
            done: false,
        }
    }

    /// The host hears that the look is over; the tool leaves after the call.
    fn finish(&mut self, cx: &mut Context<'_>) {
        if !self.done {
            cx.view_changes.push(ViewChange::Picked(None));
            self.done = true;
        }
    }
}

impl Tool for Look {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        self.title
    }

    /// “Kenar eşleme: 3. bağ, aralık 85,4 mm [Pencereye dön (tıklama, Enter ya da Esc)]”.
    fn prompt(&self) -> Prompt {
        Prompt::new(self.title, self.what.clone())
            .option("Pencereye dön", "tıklama, Enter ya da Esc")
    }

    fn point_count(&self) -> usize {
        0
    }

    fn pointer_move(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}

    fn pointer_down(&mut self, _p: &Pointer, cx: &mut Context<'_>) {
        self.finish(cx);
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.finish(cx);
        Flow::Exit
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.finish(cx);
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview::default()
    }

    fn finished(&self) -> bool {
        self.done
    }
}
