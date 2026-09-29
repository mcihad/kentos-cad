//! İçeren alanı seç (`tool.selectContaining`, docs/adr/0141): a click selects
//! the smallest closed object around it, and clicking the same place again
//! goes up to the next larger one: parcel, then block, then district.
//!
//! - The objects around the point are the store's `containing`: a polygon less
//!   its holes, a circle, a whole ellipse, a closed curve; visible ones only,
//!   smallest first, equal areas in the drawing's order.
//! - A click within the pick tolerance of the previous click goes to the next
//!   in that list and wraps after the last; a click anywhere else starts at the
//!   smallest. It becomes the selection, or with Shift held joins it.
//! - Beside the cursor: `2/3 · 1600.00 m²`, the place in the list and the
//!   area: of the object selected while the cursor is where it was clicked,
//!   else of the one a click here would select, whose outline is highlighted.
//! - The tool stays for more clicks; Esc, Enter or a right click ends it and
//!   the pointer selects again.

use kentos_domain::Slot;
use kentos_geometry_core::geometry::dist;
use kentos_geometry_core::tools::point_text::point_from_text;

use crate::Vec2;
use crate::format::Format;
use crate::log::Level;
use crate::prompt::Prompt;
use crate::tool::{Context, Cursor, Flow, Pointer, Preview, Tag, Tool};

/// The tool's id: its command is `tool.selectContaining`.
pub const ID: &str = "selectContaining";
pub const LABEL: &str = "İçeren alanı seç";

/// İçeren alanı seç.
#[derive(Clone, Debug, Default)]
pub struct SelectContaining {
    /// The last click: where it was and which of the objects around it it took (from 0).
    last: Option<(Vec2, usize)>,
    /// What stands beside the cursor.
    tag: Option<Tag>,
}

impl SelectContaining {
    pub fn new() -> Self {
        Self::default()
    }

    /// `2/3 · 1600.00 m²`: the place in the list, and the area.
    fn describe(index: usize, n: usize, area: f64, f: &Format) -> String {
        format!("{}/{n} · {}", index + 1, f.area(area))
    }

    /// A click at `at`: the smallest area around it, or the next larger when
    /// the place is the last click's. With `add` it joins the selection.
    fn click(&mut self, at: Vec2, add: bool, cx: &mut Context<'_>) {
        cx.selection.set_hover(None);
        let list = cx.spatial.containing(at);
        if list.is_empty() {
            cx.say(Level::Warn, "Tıklanan noktayı içeren kapalı alan yok.");
            self.last = None;
            self.tag = None;
            return;
        }
        let n = list.len();
        // The same place again: one larger; anywhere else: the smallest.
        let index = match self.last {
            Some((was, i)) if dist(was, at) <= cx.pick_tolerance() => (i + 1) % n,
            _ => 0,
        };
        self.last = Some((at, index));
        let (slot, area): (Slot, f64) = list[index];
        cx.selection.take([slot], add);
        let f = cx.format();
        cx.say(
            Level::Info,
            format!("Alan seçildi ({}/{n}, {}).", index + 1, f.area(area)),
        );
        self.tag = Some(Tag {
            at,
            lines: vec![Self::describe(index, n, area, &f)],
        });
    }
}

impl Tool for SelectContaining {
    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        LABEL
    }

    fn prompt(&self) -> Prompt {
        Prompt::new(LABEL, "alanın içine tıklayın")
    }

    fn point_count(&self) -> usize {
        0
    }

    /// The click is inside an area, not on a point: nothing to snap to.
    fn snaps(&self) -> bool {
        false
    }

    /// An object is wanted.
    fn cursor(&self) -> Cursor {
        Cursor::Pick
    }

    fn pointer_move(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        let list = cx.spatial.containing(p.raw);
        let f = cx.format();
        let same_place = self
            .last
            .filter(|(at, _)| dist(*at, p.raw) <= cx.pick_tolerance());
        // The selected object while the cursor stays where it was clicked; a
        // click elsewhere would take the smallest: it is outlined and described.
        let (shown, hover) = match same_place {
            Some((_, i)) if i < list.len() => (Some((i, list[i].1)), None),
            _ => (
                list.first().map(|&(_, area)| (0, area)),
                list.first().map(|&(slot, _)| slot),
            ),
        };
        cx.selection.set_hover(hover);
        self.tag = shown.map(|(i, area)| Tag {
            at: p.raw,
            lines: vec![Self::describe(i, list.len(), area, &f)],
        });
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.click(p.raw, p.shift, cx);
    }

    /// A typed point is a click there.
    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        match point_from_text(text, None, None, |d| cx.track_along(d)) {
            Some(at) => {
                let add = cx.shift;
                self.click(at, add, cx);
                true
            }
            None => false,
        }
    }

    /// A point computed by the point calculator is a click there.
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        let add = cx.shift;
        self.click(p, add, cx);
        true
    }

    /// Enter or a quick right click ends.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        cx.selection.set_hover(None);
        Flow::Exit
    }

    /// Esc ends; nothing to step back to.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        cx.selection.set_hover(None);
        false
    }

    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        false
    }

    fn preview(&self, _format: &Format) -> Preview {
        Preview {
            tag: self.tag.clone(),
            ..Preview::default()
        }
    }
}
