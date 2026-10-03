//! The digitizing locks on the desktop (docs/adr/0166 §6; the web's
//! `tools/lockGuides.ts` and the value card's chips): what the drawing, the
//! value field and the command line show of them. The session holds them
//! (`kentos_interaction::LockState`); the value field's Tab and lock
//! fields are input.rs's, the menu drawing_menus.rs's.

use kentos_interaction::{Format, Vec2};

use crate::app::App;
use crate::marks::LockMarks;

impl App {
    /// The guides: the locked direction from the reference, the locked
    /// length round it; none without locks or a reference.
    pub(crate) fn lock_marks(&self, format: &Format) -> Option<LockMarks> {
        if !self.locks.any() {
            return None;
        }
        let reference = self.session.lock_reference()?;
        let mut locks = self.locks.clone();
        locks.travel = self.session.travel();
        Some(LockMarks {
            reference,
            direction: locks.direction(format.angles()).map(|d| (d.u, d.both)),
            length: locks.length,
            edge: locks.edge,
        })
    }

    /// The locks as the value field's chips say them (Kalıcı is not a chip).
    pub(crate) fn lock_chips(&self, format: &Format) -> Vec<String> {
        let mut shown = self.locks.clone();
        shown.keep = false;
        shown.words(format)
    }

    /// “Kilit: …” above-left of the cursor, while locks hold the next point
    /// and the cursor is over the drawing; not while the value field is
    /// open: its chips say the locks.
    pub(crate) fn lock_tag(&self, format: &Format) -> Option<(String, Vec2)> {
        if !self.locks.any() || self.field.is_some() {
            return None;
        }
        self.session.lock_reference()?;
        let at = self.viewport.cursor?;
        Some((
            format!("Kilit: {}", self.locks.words(format).join(" · ")),
            at,
        ))
    }

    /// The locks changed with the pointer still (a key in the value field,
    /// a chip's ×, the menu): the running tool sees the pointer again, so
    /// its preview and its measure show the point the locks hold now (the
    /// web's `ViewportController.repoint`).
    pub(crate) fn repoint(&mut self) {
        if self.panning {
            return;
        }
        let Some(world) = self.viewport.cursor else {
            return;
        };
        let [x, y] = self.viewport.camera.world_to_screen(world);
        let p = self.pointer_at(iced::Point::new(x as f32, y as f32));
        self.with_tool(|s, cx| s.pointer_move(&p, cx));
    }

    /// A chip's ×: that lock goes (the length's, or the direction's with its picked edge).
    pub(crate) fn drop_lock(&mut self, length: bool) {
        if length {
            self.locks.length = None;
        } else {
            self.locks.drop_toward();
        }
    }
}
