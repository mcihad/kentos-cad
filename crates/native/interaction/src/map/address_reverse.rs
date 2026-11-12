//! Konumdan adres (`address.reverse`, docs/adr/0229 §13): each click asks
//! the address of its place, snapped as everywhere, from the Adres tab's
//! source (the project's address data or an online provider); the host
//! looks for it off the interface's thread ([`ViewChange::AddressAt`]) and
//! shows it in the tab as one row, its place marked. The point clicked last
//! is ringed. Nothing is written; Esc, Enter or a quick right click ends.

use crate::Vec2;
use crate::format::Format;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Marker, MarkerShape, Pointer, Preview, Tone, Tool, ViewChange};

/// The tool's id; its command is `address.reverse` (Harita › Adres, Ekle › Adres).
pub const ID: &str = "addressReverse";
pub const LABEL: &str = "Konumdan adres";
const STEP: &str = "adresi bulunacak yere tıklayın";

/// Konumdan adres.
#[derive(Clone, Debug, Default)]
pub struct AddressReverse {
    /// The place asked last, ringed.
    last: Option<Vec2>,
}

impl AddressReverse {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Tool for AddressReverse {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, STEP).note("Esc: bitir")
    }

    fn point_count(&self) -> usize {
        0
    }

    fn pointer_move(&mut self, _p: &Pointer, _cx: &mut Context<'_>) {}

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.last = Some(p.world);
        cx.view_changes.push(ViewChange::AddressAt(p.world));
    }

    /// A typed point asks as a click there does.
    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.last = Some(p);
        cx.view_changes.push(ViewChange::AddressAt(p));
        true
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match cx.typed_point(text, self.last, None) {
            Some(p) => self.accept_point(p, cx),
            None => false,
        }
    }

    fn confirm(&mut self, _cx: &mut Context<'_>) -> Flow {
        Flow::Exit
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview {
            markers: self
                .last
                .map(|at| Marker {
                    at,
                    shape: MarkerShape::Ring(6.0),
                    tone: Tone::Accent,
                })
                .into_iter()
                .collect(),
            ..Preview::default()
        }
    }
}
