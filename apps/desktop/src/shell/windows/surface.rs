//! What a window of its own is to the windows (docs/adr/8060 §10): the contract every surface fulfils in its own module
//! ([`OwnWindow`]), and what every surface shares, written once here: taking it out of the main window and bringing it
//! back (the choice kept, another window over the drawing refused with its reason), showing it where it opens, the
//! header button that moves it, its framing (the whole of its own window, or a dialog over the drawing), its default
//! keys, and the answers a trace gets from it. A surface that is always apart (the 3D views, docs/adr/8122) is never
//! brought over the drawing: no choice is kept for it and nothing offers to.
//!
//! A window joins by implementing [`OwnWindow`] for a unit struct in its own module and naming it in
//! [`Surface::spec`]; nothing else of the windows changes (docs/kilavuz/09-yeni-is-tarifleri.md §9.8).

use iced::keyboard::key::Named;
use iced::{Element, Size, Task};
use kentos_ui::widget::{Dialog as Frame, Tip};

use super::{Event, Focus, Surface};
use crate::app::{App, Dialog, Message};
use crate::keys::KeyPress;
use crate::traces::Control;

/// The header button that takes a surface out of the main window.
pub(crate) const APART: &str = "Ayrı pencerede aç";
/// The button that brings it back over the drawing.
pub(crate) const RETURN: &str = "Ana pencereye al";

/// A window the user can take out of the main window into one of the operating system's, to put it on another screen
/// and watch the drawing change while working in it. Implemented once per surface, in the surface's own module.
pub(crate) trait OwnWindow: Sync {
    /// The name its choices are kept by in `ayri-pencereler.json`; never changes once released.
    fn kept_as(&self) -> &'static str;
    /// Its window's size at its first opening and the smallest it may be made, logical pixels at 12 px text.
    fn sizes(&self) -> (Size, Size);
    /// Whether it opens in a window of its own until the user chooses (a view watched beside the plan) or over the
    /// drawing (a tool run on it).
    fn opens_apart(&self) -> bool {
        false
    }
    /// Whether it only ever opens in a window of its own (a 3D view, docs/adr/8122: over the drawing it would cover what
    /// it shows, and two places for one view confuse the work): a kept choice is not followed, Ana pencereye al is
    /// neither shown nor done.
    fn always_apart(&self) -> bool {
        false
    }
    /// What it is, as a message names it in the accusative (“aracı”, “3B görünümü”).
    fn what(&self) -> &'static str;
    /// What “Ayrı pencerede aç” does for it, its tip's body.
    fn apart_tip(&self) -> &'static str;
    /// What “Ana pencereye al” does for it.
    fn return_tip(&self) -> &'static str;
    /// The dialog it is over the drawing (`App::dialog`).
    fn dialog(&self) -> Dialog;
    /// Whether it is open: something to show, in its own window when it is apart.
    fn is_open(&self, app: &App) -> bool;
    /// Whether it can be shown over the drawing now; by default whenever it is open (a pick on the drawing waits).
    fn has_dialog(&self, app: &App) -> bool {
        self.is_open(app)
    }
    /// Whether its work is in the main window for now (a field picked on the drawing): the main window takes the
    /// keyboard meanwhile and gives it back after.
    fn work_in_main(&self, _app: &App) -> bool {
        false
    }
    /// Its own window's title, before the program's name.
    fn title(&self, app: &App) -> String;
    /// Its own window's content, the whole window: usually its dialog through [`placed`].
    fn view<'a>(&self, app: &'a App) -> Element<'a, Message>;
    /// What its Kapat does; its own window's close button is this.
    fn close(&self, app: &mut App) -> Task<Message>;
    /// A key its own window's widgets did not take: by default Tab goes round its fields and Esc closes it.
    fn on_key(&self, app: &mut App, press: &KeyPress) -> Task<Message> {
        match press.named() {
            Some(Named::Tab) if !press.modifiers.control() && !press.modifiers.alt() => {
                kentos_ui::focus::next(press.modifiers.shift())
            }
            Some(Named::Escape) => self.close(app),
            _ => Task::none(),
        }
    }
    /// Its title to a trace while it is apart (a trace step names the window it answers); none: no trace answers it.
    fn trace_title(&self, _app: &App) -> Option<String> {
        None
    }
    /// The message a trace's control sends in it while it is apart; none when it has no such control.
    fn trace_control(&self, _app: &App, _control: Control<'_>) -> Option<Result<Option<Message>, String>> {
        None
    }
}

fn apart_message(surface: Surface, apart: bool) -> Message {
    Message::Windows(Event::Apart(surface, apart))
}

/// A surface's dialog with the header button that moves it (DESIGN.md §7.9): “Ana pencereye al” in its own window,
/// “Ayrı pencerede aç” over the drawing.
pub(crate) fn apart_button<'a>(frame: Frame<'a, Message>, surface: Surface, apart: bool) -> Frame<'a, Message> {
    let spec = surface.spec();
    if apart {
        frame.header_button(
            crate::icons::from_web(Some("windowReturn")),
            Tip::new(RETURN).body(spec.return_tip()),
            apart_message(surface, false),
        )
    } else {
        frame.header_button(
            crate::icons::from_web(Some("externalOpen")),
            Tip::new(APART).body(spec.apart_tip()),
            apart_message(surface, true),
        )
    }
}

/// A surface's dialog where it is: the whole of its own window, or over the drawing, a click beside it sending
/// `dismiss`; the button that moves it in its header either way.
pub(crate) fn placed<'a>(frame: Frame<'a, Message>, surface: Surface, apart: bool, dismiss: Message) -> Element<'a, Message> {
    let frame = apart_button(frame, surface, apart);
    if apart { frame.window() } else { frame.modal(dismiss) }
}

impl App {
    /// Whether `surface` opens in a window of its own: always for a 3D view, else the user's last choice, else its own
    /// way.
    pub(crate) fn is_apart(&self, surface: Surface) -> bool {
        surface.spec().always_apart() || self.windows.memory.apart(surface)
    }

    /// Whether `surface` has its own window now: apart and open.
    pub(crate) fn surface_wanted(&self, surface: Surface) -> bool {
        self.is_apart(surface) && surface.spec().is_open(self)
    }

    /// A surface just opened (its state made): in its own window (brought to the front when it is open already; the
    /// windows following the state make it otherwise), or over the drawing as its dialog.
    pub(crate) fn show_surface(&mut self, surface: Surface) {
        if self.is_apart(surface) {
            self.surface_away(surface);
            self.windows.ask_focus(Focus::Surface(surface));
        } else {
            self.dialog = Some(surface.spec().dialog());
        }
    }

    /// The surface's dialog leaves the main window (it went apart, or it closed); another window there stays.
    pub(crate) fn surface_away(&mut self, surface: Surface) {
        if self.dialog == Some(surface.spec().dialog()) {
            self.dialog = None;
        }
    }

    /// Into a window of its own, or back over the drawing; the choice is kept for its next opening, in the next run
    /// too. Back over the drawing waits while another window is open there, and says so.
    pub(crate) fn set_apart(&mut self, surface: Surface, apart: bool) {
        let spec = surface.spec();
        // A 3D view stays in its own window: nothing to keep, nothing to move (docs/adr/8122).
        if spec.always_apart() {
            return;
        }
        if !apart && self.dialog.is_some_and(|d| d != spec.dialog()) {
            self.warn(format!(
                "Ana pencerede açık bir pencere var: önce onu kapatın, sonra {} ana pencereye alın.",
                spec.what()
            ));
            return;
        }
        self.windows.memory.set_apart(surface, apart);
        if apart {
            self.surface_away(surface);
        } else if spec.has_dialog(self) {
            self.dialog = Some(spec.dialog());
            self.windows.ask_focus(Focus::Main);
        }
    }

    /// The open surface apart that answers a trace, and its title (traces/answers.rs).
    pub(crate) fn apart_trace_title(&self) -> Option<String> {
        Surface::ALL.into_iter().filter(|&s| self.surface_wanted(s)).find_map(|s| s.spec().trace_title(self))
    }

    /// A trace's control in the open surface apart that answers it.
    pub(crate) fn apart_trace_control(&self, control: Control<'_>) -> Option<Result<Option<Message>, String>> {
        Surface::ALL.into_iter().filter(|&s| self.surface_wanted(s)).find_map(|s| s.spec().trace_control(self, control))
    }

    /// A trace's press of the button that moves a window: “Ayrı pencerede aç” in a surface's dialog over the drawing,
    /// “Ana pencereye al” in a surface's own window.
    pub(crate) fn apart_control(&self, control: Control<'_>) -> Option<Message> {
        match control {
            Control::Press(APART) => Surface::ALL
                .into_iter()
                .find(|s| self.dialog == Some(s.spec().dialog()))
                .map(|s| apart_message(s, true)),
            Control::Press(RETURN) => Surface::ALL
                .into_iter()
                .find(|&s| self.surface_wanted(s) && !s.spec().always_apart())
                .map(|s| apart_message(s, false)),
            _ => None,
        }
    }
}
