//! A module's parts the shell reaches (docs/adr/9199 §3.3–3.4; CLAUDE.md §25 rule 3): its messages, windows, commands,
//! marks over the drawing, panels, and what it does when the drawing changes or the marks are cleared. The shell names
//! no module's types: a module's state is in [`ModuleStates`], its messages are [`ModuleEvent`]s, its windows
//! [`ModuleDialog`]s, and each module gives its [`ModuleParts`]. [`all`] is the shell's one list of them
//! (scripts/dev/module_deps.py allows the module names here alone).

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use iced::widget::canvas;
use iced::{Element, Task};
use kentos_launch::Module;

use kentos_render_wgpu::Camera;
use kentos_ui::theme::Tokens;

use crate::app::{App, Message, Panel};
use crate::traces::Control;
use crate::viewport::MarkColors;

/// A module's message (`Message::Module`): the module and its own event, shared as the shell's messages are cloned.
#[derive(Clone)]
pub struct ModuleEvent {
    pub module: Module,
    event: Arc<dyn Any + Send + Sync>,
    debug: fn(&(dyn Any + Send + Sync), &mut fmt::Formatter<'_>) -> fmt::Result,
}

impl ModuleEvent {
    /// `event` of `module`.
    pub fn new<E: Any + Send + Sync + fmt::Debug>(module: Module, event: E) -> Self {
        fn debug<E: Any + fmt::Debug>(e: &(dyn Any + Send + Sync), f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match e.downcast_ref::<E>() {
                Some(e) => e.fmt(f),
                None => f.write_str("?"),
            }
        }
        Self { module, event: Arc::new(event), debug: debug::<E> }
    }

    /// The event as its module's type, taken out (cloned when the message was cloned before).
    pub fn take<E: Any + Send + Sync + Clone>(self) -> Option<E> {
        let event = Arc::downcast::<E>(self.event).ok()?;
        Some(Arc::try_unwrap(event).unwrap_or_else(|shared| (*shared).clone()))
    }
}

impl fmt::Debug for ModuleEvent {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}(", self.module.id())?;
        (self.debug)(self.event.as_ref(), f)?;
        f.write_str(")")
    }
}

/// A module's window (`Dialog::Module`): the module and the window's number in it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModuleDialog {
    pub module: Module,
    pub kind: u16,
}

/// Every module's state in this program (`App::modules`), each as its module keeps it.
pub struct ModuleStates(Vec<Box<dyn Any>>);

impl ModuleStates {
    /// The states of the modules in this program, as they start.
    pub fn start() -> Self {
        Self(all().iter().map(|p| p.state()).collect())
    }

    /// The state of type `T`; none when no module of this program keeps one.
    pub fn get<T: Any>(&self) -> Option<&T> {
        self.0.iter().find_map(|s| s.downcast_ref::<T>())
    }

    /// The state of type `T`, to change; none when no module of this program keeps one.
    pub fn get_mut<T: Any>(&mut self) -> Option<&mut T> {
        self.0.iter_mut().find_map(|s| s.downcast_mut::<T>())
    }
}

/// A module's marks over the drawing (cad/marks.rs), drawn after the shell's own and before a topology finding.
pub trait ModuleMarks: Send + Sync {
    fn draw(&self, frame: &mut canvas::Frame, camera: &Camera, colors: &MarkColors, tokens: &Tokens);
}

/// What the shell asks a module.
pub trait ModuleParts: Sync {
    /// The module these parts are of.
    fn module(&self) -> Module;
    /// The module's state as a new window starts it (`ModuleStates`).
    fn state(&self) -> Box<dyn Any>;
    /// A message of the module.
    fn event(&self, app: &mut App, event: ModuleEvent) -> Task<Message>;
    /// A command id of the module (ribbon, shortcut, command line); none when it is not the module's.
    fn command(&self, app: &mut App, id: &str) -> Option<Task<Message>>;
    /// Whether a command of the module can run now; none when it is not the module's.
    fn enabled(&self, app: &App, id: &str) -> Option<bool>;
    /// A window of the module.
    fn dialog_view<'a>(&self, app: &'a App, kind: u16) -> Element<'a, Message>;
    /// A window's title, as a usage trace names it.
    fn dialog_title(&self, kind: u16) -> String;
    /// The message a usage trace's `control` on a window of the module sends.
    fn dialog_control(&self, app: &App, kind: u16, control: Control<'_>) -> Result<Option<Message>, String>;
    /// A window of the module closed by the shell (leaving the drawing on screen): what it keeps open goes.
    fn dialog_closed(&self, app: &mut App, kind: u16);
    /// The drawing changed (an undo, a redo, a script's write): a window that follows it reads it again.
    fn document_changed(&self, app: &mut App);
    /// İşareti kaldır: the marks it left on the drawing go.
    fn unmark(&self, app: &mut App);
    /// The marks it leaves on the drawing, in the order they are drawn.
    fn marks(&self, app: &App) -> Vec<Arc<dyn ModuleMarks>>;
    /// Its layers over the drawing area, over the shell's.
    fn area_layers<'a>(&self, app: &'a App) -> Vec<Element<'a, Message>>;
    /// A panel of the dock; none when it is not the module's.
    fn panel_view<'a>(&self, app: &'a App, panel: Panel) -> Option<Element<'a, Message>>;
}

/// The modules' parts in this program.
pub fn all() -> &'static [&'static dyn ModuleParts] {
    &[&crate::map_parts::Map]
}

/// `module`'s parts; none when it is not in this program (or not yet a module of its own, docs/adr/9199 §4).
pub fn parts(module: Module) -> Option<&'static dyn ModuleParts> {
    all().iter().copied().find(|p| p.module() == module)
}

impl App {
    /// The modules' marks on the drawing, in their order (cad/marks.rs).
    pub(crate) fn module_marks(&self) -> Vec<Arc<dyn ModuleMarks>> {
        all().iter().flat_map(|p| p.marks(self)).collect()
    }

    /// A message of a module, to its module.
    pub(crate) fn module_event(&mut self, event: ModuleEvent) -> Task<Message> {
        match parts(event.module) {
            Some(p) => p.event(self, event),
            None => Task::none(),
        }
    }

    /// A window of a module; empty when the module is not in this program.
    pub(crate) fn module_dialog_view(&self, dialog: ModuleDialog) -> Element<'_, Message> {
        match parts(dialog.module) {
            Some(p) => p.dialog_view(self, dialog.kind),
            None => iced::widget::Column::new().into(),
        }
    }

    /// A dock panel of a module; empty when no module of this program has it.
    pub(crate) fn module_panel_view(&self, panel: Panel) -> Element<'_, Message> {
        all()
            .iter()
            .find_map(|p| p.panel_view(self, panel))
            .unwrap_or_else(|| iced::widget::Column::new().into())
    }
}
