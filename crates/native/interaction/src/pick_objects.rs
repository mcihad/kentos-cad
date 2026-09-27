//! Sahneden seç: objects picked on the drawing for a window's field (the
//! processing window's input objects, docs/adr/0088), as the KentOS UI
//! inspector picks a referenced object on the map. While it runs, a click
//! turns the object under it over in the selection and a drag past 4 px adds
//! what its box holds (left to right a window, right to left a crossing),
//! only objects of the field's kinds (a click on a point where only areas
//! are wanted takes the nearest area's edge instead). Enter, Space or a
//! quick right click keep what is selected; Esc leaves it. Either way the
//! host hears [`ViewChange::PickedObjects`]. The tool is not in the catalog
//! and is not repeated ([`crate::Session::run`]).

use kentos_domain::Slot;
use kentos_geometry_core::jsmath::js_hypot;

use crate::Vec2;
use crate::format::Format;
use crate::prompt::Prompt;
use crate::select::SelectBox;
use crate::tool::{Context, Flow, Pointer, Preview, Tool, ViewChange};

/// The tool's id, as the traces would read it.
pub const ID: &str = "pickObjects";
/// How far the pointer must move with the button down to draw a box, logical pixels.
const DRAG_THRESHOLD: f64 = 4.0;

/// A press on the drawing: where it began and where the pointer is.
#[derive(Clone, Copy, Debug)]
struct Press {
    from: [f64; 2],
    from_world: Vec2,
    to: [f64; 2],
    to_world: Vec2,
    dragging: bool,
}

/// Picks objects for a window's field.
#[derive(Clone, Debug)]
pub struct PickObjects {
    /// The field the objects are for (“Alanlar”), the prompt's name.
    field: String,
    /// The kinds the field takes (`polygon`, `polyline` …); none: any.
    kinds: Option<Vec<String>>,
    press: Option<Press>,
    /// How many are selected, for the prompt.
    count: usize,
    done: bool,
}

impl PickObjects {
    pub fn new(field: impl Into<String>, kinds: Option<Vec<String>>) -> Self {
        Self {
            field: field.into(),
            kinds: kinds.filter(|k| !k.is_empty()),
            press: None,
            count: 0,
            done: false,
        }
    }

    /// Whether the field takes this object.
    fn takes(&self, slot: Slot, cx: &Context<'_>) -> bool {
        self.kinds.as_ref().is_none_or(|kinds| {
            cx.doc
                .get(slot)
                .is_some_and(|e| kinds.iter().any(|k| k == e.kind()))
        })
    }

    /// The object of the field's kinds under the pointer: the most specific
    /// one, else the nearest edge of one it takes.
    fn hit(&self, at: Vec2, cx: &Context<'_>) -> Option<Slot> {
        let tol = cx.pick_tolerance();
        match cx.spatial.pick(at, tol) {
            Some(hit) if self.takes(hit, cx) => Some(hit),
            _ => cx.spatial.pick_edge(at, tol, |s| self.takes(s, cx)),
        }
    }

    fn finish(&mut self, keep: bool, cx: &mut Context<'_>) {
        if !self.done {
            cx.selection.set_hover(None);
            cx.view_changes.push(ViewChange::PickedObjects(keep));
            self.done = true;
        }
    }
}

impl Tool for PickObjects {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        "Sahneden seç"
    }

    /// “Alanlar: nesneleri tıklayın ya da pencereyle seçin (3 seçili) [Bitti
    /// (Enter) / Vazgeç (Esc)]”.
    fn prompt(&self) -> Prompt {
        Prompt::untitled(format!(
            "{}: nesneleri tıklayın ya da pencereyle seçin ({} seçili)",
            self.field, self.count
        ))
        .option("Bitti", "Enter")
        .option("Vazgeç", "Esc")
    }

    fn point_count(&self) -> usize {
        0
    }

    fn activate(&mut self, cx: &mut Context<'_>) -> Flow {
        self.count = cx.selection.len();
        Flow::Stay
    }

    /// Objects are picked, not points: no object snaps.
    fn snaps(&self) -> bool {
        false
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        match &mut self.press {
            Some(press) => {
                press.to = p.screen;
                press.to_world = p.raw;
                let moved = js_hypot(p.screen[0] - press.from[0], p.screen[1] - press.from[1]);
                if !press.dragging && moved > DRAG_THRESHOLD {
                    press.dragging = true;
                    cx.selection.set_hover(None);
                }
            }
            None => {
                let hit = self.hit(p.raw, cx);
                cx.selection.set_hover(hit);
            }
        }
    }

    fn pointer_down(&mut self, p: &Pointer, _cx: &mut Context<'_>) {
        // A click turns one object over; a drag draws a box (decided on release).
        self.press = Some(Press {
            from: p.screen,
            from_world: p.raw,
            to: p.screen,
            to_world: p.raw,
            dragging: false,
        });
    }

    fn pointer_up(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let Some(press) = self.press.take() else {
            return;
        };
        if press.dragging {
            // The box as it was drawn decides: one rule for the look and the query.
            let crossing = SelectBox {
                from: press.from,
                to: press.to,
            }
            .crossing();
            let ids: Vec<Slot> = cx
                .spatial
                .in_rect(press.from_world, press.to_world, crossing)
                .into_iter()
                .filter(|s| self.takes(*s, cx))
                .collect();
            cx.selection.add(ids);
        } else if let Some(hit) = self.hit(p.raw, cx) {
            cx.selection.toggle(hit);
        }
        self.count = cx.selection.len();
    }

    fn input(&mut self, _text: &str, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// Enter, Space or a quick right click: what is selected goes to the field.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        self.finish(true, cx);
        Flow::Exit
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    /// Esc: the field stays as it was.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.finish(false, cx);
        false
    }

    fn finished(&self) -> bool {
        self.done
    }

    fn select_box(&self) -> Option<SelectBox> {
        let press = self.press.filter(|press| press.dragging)?;
        Some(SelectBox {
            from: press.from,
            to: press.to,
        })
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview::default()
    }
}
