//! The windows in the app (docs/adr/8060): each window's view, title and theme by its id, the windows' events, the
//! windows following the state, and the program's end with the main window. Every surface is asked through its
//! [`OwnWindow`](super::OwnWindow) contract: a new surface changes nothing here.

use iced::widget::space;
use iced::{Element, Size, Task, Theme, window};
use kentos_ui::theme::typography;

use super::{Event, Focus, Surface};
use crate::app::{App, Message};

impl App {
    /// A window's view: the main window's ([`App::view`]), or its surface's. A window not known any more (it is
    /// closing) shows nothing: never the main window's view in a second window.
    pub fn window_view(&self, id: window::Id) -> Element<'_, Message> {
        if self.windows.is_main(id) {
            return self.view();
        }
        let content = match self.windows.surface_of(id) {
            Some(surface) => surface.spec().view(self),
            None => space().into(),
        };
        kentos_ui::widget::presented(content).into()
    }

    /// A window's title: the main window's ([`App::title`]), or its surface's with the program's name, “Tampon —
    /// KentOS GIS”: the taskbar and the window switcher tell the two apart by it.
    pub fn window_title(&self, id: window::Id) -> String {
        if self.windows.is_main(id) {
            return self.title();
        }
        let program = self
            .program
            .map_or("KentOS CAD", kentos_launch::Module::title);
        match self.windows.surface_of(id) {
            Some(surface) => format!("{} — {program}", surface.spec().title(self)),
            None => program.to_owned(),
        }
    }

    /// Every window has the interface's theme: the same colours in both. The faces and the text size are the
    /// program's own and the same everywhere; each window follows its own screen's pixel density.
    pub fn window_theme(&self, _: window::Id) -> Theme {
        self.theme()
    }

    /// What the windows say.
    pub(crate) fn windows_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Opened(id) => {
                // Asked for and given up before the platform made it: it goes now.
                if !self.windows.opened(id) {
                    return window::close(id);
                }
            }
            Event::CloseRequested(id) => {
                return match self.windows.surface_of(id) {
                    Some(surface) => self.surface_closed(surface),
                    None => window::close(id),
                };
            }
            Event::Closed(id) => {
                // Closed without being asked by the app: its surface closes too, else it would open again.
                if let Some(gone) = self.windows.remove(id) {
                    return self.surface_closed(gone.surface);
                }
            }
            Event::Moved(id, at) => {
                if let Some(surface) = self.windows.surface_of(id) {
                    self.windows.memory.keep_place(surface, at);
                }
            }
            Event::Resized(id, size) => {
                if let Some(surface) = self.windows.surface_of(id) {
                    self.windows.memory.keep_size(surface, size);
                }
            }
            Event::Placed(surface, Some(at)) => self.windows.memory.keep_place(surface, at),
            Event::Placed(_, None) => {}
            Event::Key(id, press) => {
                if let Some(surface) = self.windows.surface_of(id) {
                    return surface.spec().on_key(self, &press);
                }
            }
            Event::Apart(surface, apart) => self.set_apart(surface, apart),
        }
        Task::none()
    }

    /// After every message the windows follow the state: a surface apart with something to show gets its window,
    /// one with nothing to show loses it. When a surface's work moves to the main window (a field picked on the
    /// drawing) the main window gets the keyboard, and the surface's window gets it back after. Then the keyboard
    /// goes where it was asked to.
    pub(crate) fn follow_windows(&mut self) -> Task<Message> {
        if self.windows.closing() {
            return Task::none();
        }
        let mut tasks = Vec::new();
        for surface in Surface::ALL {
            let wanted = self.surface_wanted(surface);
            let away = wanted && surface.spec().work_in_main(self);
            let moved = self.windows.set_away(surface, away);
            match self.windows.of(surface) {
                None if wanted => tasks.push(self.open_surface_window(surface)),
                Some(w) if !wanted && w.opened => {
                    self.windows.remove(w.id);
                    tasks.push(close_kept(w.id, surface));
                }
                Some(w) if moved && w.opened => self.windows.ask_focus(if away {
                    Focus::Main
                } else {
                    Focus::Surface(surface)
                }),
                _ => {}
            }
        }
        if let Some(id) = self.windows.take_focus() {
            tasks.push(window::gain_focus(id));
        }
        Task::batch(tasks)
    }

    /// The main window closes, its unsaved work asked about first (cloud/leaving.rs): the secondary windows go,
    /// then the main window, then the program. A daemon does not end with its last window.
    pub(crate) fn close_every_window(&mut self, main: window::Id) -> Task<Message> {
        let others = self.windows.close_all();
        Task::batch(others.into_iter().map(window::close))
            .chain(window::close(main))
            .chain(iced::exit())
    }

    fn open_surface_window(&mut self, surface: Surface) -> Task<Message> {
        let (id, opened) = window::open(self.window_settings(surface));
        self.windows.add(id, surface);
        opened.map(|id| Message::Windows(Event::Opened(id)))
    }

    /// A surface's window: its last size (else its own, grown with the text), where it was this run (else in the
    /// middle of the main window's screen), the program's icon and its name to the desktop.
    pub(crate) fn window_settings(&self, surface: Surface) -> window::Settings {
        let grow = |s: Size| Size::new(typography::scaled(s.width), typography::scaled(s.height));
        let (size, min) = surface.sizes();
        window::Settings {
            size: self
                .windows
                .memory
                .size(surface)
                .unwrap_or_else(|| grow(size)),
            min_size: Some(grow(min)),
            position: self
                .windows
                .memory
                .place(surface)
                .map_or(window::Position::Centered, window::Position::Specific),
            icon: kentos_ui::widget::brand::window_icon(),
            // Its close button is its surface's Kapat (`surface_closed`).
            exit_on_close_request: false,
            // The main window's class: the desktop groups the two as one program (docs/adr/0802).
            #[cfg(target_os = "linux")]
            platform_specific: window::settings::PlatformSpecific {
                application_id: self
                    .program
                    .map_or("kentos-cad", kentos_launch::Module::program)
                    .to_owned(),
                ..Default::default()
            },
            ..window::Settings::default()
        }
    }

    /// A surface's window closed by its close button: its Kapat.
    fn surface_closed(&mut self, surface: Surface) -> Task<Message> {
        surface.spec().close(self)
    }
}

/// Closes a surface's window, its place asked first and kept for its next opening this run. Without a window
/// manager to say so (X11 with none) a window's moves are never told; Wayland tells no place at all (none kept).
fn close_kept(id: window::Id, surface: Surface) -> Task<Message> {
    window::position(id)
        .map(move |at| Message::Windows(Event::Placed(surface, at)))
        .chain(window::close(id))
}
