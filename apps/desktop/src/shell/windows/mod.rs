//! Windows of their own beside the main one (docs/adr/8060): a dialog the user takes out of the main window into an
//! operating-system window, to put it on another screen and watch the drawing change while working in it.
//!
//! The program is one iced daemon with one state. Every window's messages go through the one `App::update`; each
//! window's view, title and theme are asked by its id (`app.rs`).
//!
//! - [`Surface`] is what a secondary window shows. İşlemler's tool window is the first. Another dialog becomes a
//!   window of its own by adding a variant: its view, title, close and keys in `app.rs`, its state where it lives.
//! - [`Windows`] holds the main window's id and the open secondary windows, each with its surface, and what each
//!   surface remembers ([`memory`]).
//! - The windows follow the state (`App::follow_windows`): a surface that is apart and has something to show gets
//!   its window; one that closes loses it. Whatever closes a tool (Kapat, Esc, Model tasarımcısı) closes its window.
//! - A key typed in a secondary window never reaches the main window's shortcuts or the drawing ([`routing`]).

mod app;
pub(crate) mod memory;
pub(crate) mod routing;
#[cfg(test)]
mod tests;

use iced::window;
use iced::{Point, Size};

pub(crate) use memory::Memory;

/// What a secondary window shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub(crate) enum Surface {
    /// İşlemler's tool window (gis/processing/apart.rs), its Toplu çalıştır table too.
    ProcessingTool,
}

impl Surface {
    pub(crate) const ALL: [Surface; 1] = [Surface::ProcessingTool];

    /// The name its choices are kept by (`ayri-pencereler.json`).
    pub(crate) fn key(self) -> &'static str {
        match self {
            Surface::ProcessingTool => "islem-araci",
        }
    }

    /// The window's size at its first opening and the smallest it may be made, in logical pixels at 12 px text
    /// (`app.rs` grows them with the text size). The tool window's are the in-canvas window's 940 × 660.
    pub(crate) fn sizes(self) -> (Size, Size) {
        match self {
            Surface::ProcessingTool => (Size::new(940.0, 660.0), Size::new(640.0, 420.0)),
        }
    }
}

/// Which window should have the keyboard next.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Focus {
    Main,
    Surface(Surface),
}

/// An open secondary window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Secondary {
    pub(crate) id: window::Id,
    pub(crate) surface: Surface,
    /// The platform has made it (`window::open`'s answer). Until then it is not closed: a close waits for it, else
    /// the window would be made after its close and stay open with nothing in it.
    pub(crate) opened: bool,
}

/// The program's windows.
#[derive(Debug, Default)]
pub(crate) struct Windows {
    /// The main window, opened at start. None without a window (tests, snapshots, the trace player): then every id
    /// that is not a secondary window's is the main window's, as it was with one window.
    main: Option<window::Id>,
    open: Vec<Secondary>,
    /// The main window is closing: nothing opens any more.
    closing: bool,
    /// The window to give the keyboard to after this message.
    focus: Option<Focus>,
    /// The surfaces whose work is in the main window for now (a field picked on the drawing).
    away: Vec<Surface>,
    /// What each surface remembers: whether it opens apart, its size and its place.
    pub(crate) memory: Memory,
}

/// What the windows say.
#[derive(Debug, Clone)]
pub(crate) enum Event {
    /// A secondary window was made by the platform.
    Opened(window::Id),
    /// Its close button: its surface's Kapat.
    CloseRequested(window::Id),
    /// It was closed.
    Closed(window::Id),
    /// It moved or was resized: kept for its next opening.
    Moved(window::Id, Point),
    Resized(window::Id, Size),
    /// Where a surface's window was as it closed (asked of the platform: not every one says when a window moves).
    Placed(Surface, Option<Point>),
    /// A key no widget of a secondary window took (routing.rs).
    Key(window::Id, crate::keys::KeyPress),
}

impl Windows {
    /// The main window, opened at start.
    pub(crate) fn set_main(&mut self, id: window::Id) {
        self.main = Some(id);
    }

    pub(crate) fn main(&self) -> Option<window::Id> {
        self.main
    }

    /// Whether `id` is the main window's.
    pub(crate) fn is_main(&self, id: window::Id) -> bool {
        match self.main {
            Some(main) => main == id,
            None => self.surface_of(id).is_none(),
        }
    }

    /// The surface window `id` shows.
    pub(crate) fn surface_of(&self, id: window::Id) -> Option<Surface> {
        self.open.iter().find(|w| w.id == id).map(|w| w.surface)
    }

    /// The window showing `surface`.
    pub(crate) fn of(&self, surface: Surface) -> Option<Secondary> {
        self.open.iter().find(|w| w.surface == surface).copied()
    }

    /// A window asked of the platform for `surface`.
    pub(crate) fn add(&mut self, id: window::Id, surface: Surface) {
        self.open.retain(|w| w.surface != surface);
        self.open.push(Secondary {
            id,
            surface,
            opened: false,
        });
    }

    /// The platform made window `id`; false when it is not one of these.
    pub(crate) fn opened(&mut self, id: window::Id) -> bool {
        match self.open.iter_mut().find(|w| w.id == id) {
            Some(w) => {
                w.opened = true;
                true
            }
            None => false,
        }
    }

    /// Window `id` goes: its size is written for its surface's next opening.
    pub(crate) fn remove(&mut self, id: window::Id) -> Option<Secondary> {
        let at = self.open.iter().position(|w| w.id == id)?;
        let gone = self.open.remove(at);
        self.memory.write();
        Some(gone)
    }

    /// Whether the main window is closing.
    pub(crate) fn closing(&self) -> bool {
        self.closing
    }

    /// The main window closes: every secondary window goes with it and none opens any more. Their ids, to close.
    pub(crate) fn close_all(&mut self) -> Vec<window::Id> {
        self.closing = true;
        self.focus = None;
        let ids = self.open.drain(..).map(|w| w.id).collect();
        self.memory.write();
        ids
    }

    /// Whether `surface`'s work is in the main window for now; true when that changed.
    pub(crate) fn set_away(&mut self, surface: Surface, away: bool) -> bool {
        let was = self.away.contains(&surface);
        if away && !was {
            self.away.push(surface);
        } else if !away && was {
            self.away.retain(|s| *s != surface);
        }
        away != was
    }

    /// Gives the keyboard to `focus` after this message.
    pub(crate) fn ask_focus(&mut self, focus: Focus) {
        self.focus = Some(focus);
    }

    /// The window to give the keyboard to now: an opened one (a new window takes it by itself).
    pub(crate) fn take_focus(&mut self) -> Option<window::Id> {
        match self.focus.take()? {
            Focus::Main => self.main,
            Focus::Surface(surface) => self.of(surface).filter(|w| w.opened).map(|w| w.id),
        }
    }
}
