//! A module's parts the shell reaches (docs/adr/9199 §3.3–3.4, §4.9; CLAUDE.md §25 rule 3): its profile, messages,
//! windows, commands, marks and preview over the drawing, panels and bottom tabs, a point picked for it, what it shows
//! in 3B görünüm, Öznitelikler's edits it takes, and what it does when the window starts, follows a message, the
//! drawing changes or the marks are cleared. The
//! shell names no module's types: a module's state is in [`ModuleStates`], its messages are [`ModuleEvent`]s, its
//! windows [`ModuleDialog`]s, and each module package gives its [`ModuleParts`] to [`register`] before the window
//! starts (a program in its `main`, the package's tests before they run). [`all`] is the shell's one list of them.

use std::any::Any;
use std::collections::BTreeSet;
use std::fmt;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use iced::widget::canvas;
use iced::{Element, Task};
use kentos_launch::Module;

use kentos_render_wgpu::Camera;
use kentos_ui::theme::Tokens;

use crate::app::{App, Message, Panel};
use crate::bottom::BottomTab;
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
        fn debug<E: Any + fmt::Debug>(
            e: &(dyn Any + Send + Sync),
            f: &mut fmt::Formatter<'_>,
        ) -> fmt::Result {
            match e.downcast_ref::<E>() {
                Some(e) => e.fmt(f),
                None => f.write_str("?"),
            }
        }
        Self {
            module,
            event: Arc::new(event),
            debug: debug::<E>,
        }
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
    fn draw(
        &self,
        frame: &mut canvas::Frame,
        camera: &Camera,
        colors: &MarkColors,
        tokens: &Tokens,
    );
}

/// What a module gives 3B görünüm beside the drawing's building (scene3d_building.rs) and the IFC viewer (ifc_viewer.rs):
/// the models it shows with it and the objects it marks.
pub trait Building3dParts: Sync {
    /// The models shown beside the drawing's: their id, name and file as read.
    fn beside(&self, app: &App) -> Vec<(String, String, Arc<crate::underlay_model::Loaded>)>;
    /// An object of a model beside the drawing chosen in 3D (`object`, its index in the file): the module shows it;
    /// false when `id` is none of its models.
    fn choose_beside(&self, app: &mut App, id: &str, object: usize) -> bool;
    /// What the objects marked depend on, as one number: a change of it marks them again.
    fn marked_key(&self, app: &App) -> u64;
    /// The objects marked (a check's findings), by their keys.
    fn marked(&self, app: &App) -> BTreeSet<String>;
}

/// What Öznitelikler's Tip row asks of the selected elements (docs/adr/8106 §5): follow the type of that id, leave
/// their type, give the type their own values, take its values again, take its last version.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ElementTypeAsk {
    Assign(String),
    Detach,
    Push,
    Reset,
    Refresh,
}

/// A schedule table's settings window or its file, by the table's id (docs/adr/0733 §5, §7).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScheduleAsk {
    Settings(String),
    Export(String),
}

/// What Öznitelikler asks a module before it applies an edit (cad/properties/panel.rs).
#[derive(Debug, Clone, Copy)]
pub enum PropertiesAsk<'a> {
    /// Tipe uygula on an opening (docs/adr/0772): the module's preview asks first.
    OpeningPush(kentos_domain::Slot, &'a kentos_commands::OpeningsOperation),
    /// The Tip row's action (rows/element_types.rs).
    ElementType(&'a ElementTypeAsk),
    /// A schedule's Çizelge ayarları or Dosyaya yaz (rows/entity.rs).
    Schedule(&'a ScheduleAsk),
}

/// What the shell asks a module.
pub trait ModuleParts: Sync {
    /// The module these parts are of.
    fn module(&self) -> Module;
    /// What the module shows of the window (module_profile.rs).
    fn profile(&self) -> &'static dyn crate::module_profile::ModuleProfile;
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
    fn dialog_control(
        &self,
        app: &App,
        kind: u16,
        control: Control<'_>,
    ) -> Result<Option<Message>, String>;
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
    /// A window started, the modules' states in place: what the module's program starts with (its tools' first
    /// style, docs/adr/8007).
    fn started(&self, app: &mut App);
    /// The program started on this device, not under a test: what the module keeps beside the program's history
    /// (`history`, the recent files' folder) is read.
    fn device(&self, app: &mut App, history: &Path);
    /// After every message: the module's work that follows the window (a file read off its thread).
    fn follow(&self, app: &mut App) -> Task<Message>;
    /// Whether Esc in a text box of the module's window closes the window as Esc beside it does (DESIGN.md §7.9): an
    /// editor with its own save, which asks first.
    fn escape_closes(&self, kind: u16) -> bool;
    /// Tamam on a question of the module (`Message::DialogConfirmed`), the question taken off the screen.
    fn dialog_confirmed(&self, app: &mut App, kind: u16);
    /// Whether a toggle command of the module is on; none when it is not the module's.
    fn checked(&self, app: &App, id: &str) -> Option<bool>;
    /// A point picked on the drawing (Çizimden) for a window of the module; false when it asked for none.
    fn picked(&self, app: &mut App, at: Option<kentos_interaction::Vec2>) -> bool;
    /// What the module previews over the drawing while no tool does (a check's chosen fixes).
    fn preview(&self, app: &App) -> Option<kentos_interaction::Preview>;
    /// A tab of the bottom panel; none when it is not the module's.
    fn bottom_tab_view<'a>(&self, app: &'a App, tab: BottomTab) -> Option<Element<'a, Message>>;
    /// What Öznitelikler asks before it applies an edit (cad/properties/panel.rs): the module takes it (its window asks
    /// first, its types' actions); false leaves it to the panel.
    fn properties_ask(&self, app: &mut App, ask: PropertiesAsk<'_>) -> bool;
    /// What it gives 3B görünüm beside the drawing's building; none when nothing.
    fn building3d(&self) -> Option<&dyn Building3dParts>;
    /// Another drawing comes on screen (opened, a tab brought in front): what the module kept of the drawing before
    /// goes.
    fn drawing_shown(&self, app: &mut App);
}

/// The parts registered so far, before [`all`] is first asked.
static REGISTERED: Mutex<Vec<&'static dyn ModuleParts>> = Mutex::new(Vec::new());
/// The parts of this program, fixed when first asked: in the modules' order (`Module::ALL`), one per module.
static ALL: OnceLock<Vec<&'static dyn ModuleParts>> = OnceLock::new();

/// Puts a module package's parts among the program's; a module registered twice is kept once. A program registers its
/// modules in `main` before the window starts; registering after [`all`] was first asked changes nothing and says so.
pub fn register(parts: &'static dyn ModuleParts) -> bool {
    if ALL.get().is_some() {
        return false;
    }
    let mut registered = REGISTERED
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if !registered.iter().any(|p| p.module() == parts.module()) {
        registered.push(parts);
    }
    true
}

/// The modules' parts in this program.
pub fn all() -> &'static [&'static dyn ModuleParts] {
    ALL.get_or_init(|| {
        let registered = REGISTERED
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let mut list: Vec<&'static dyn ModuleParts> = registered.clone();
        // The modules still in the core's package (module_profile.rs).
        for parts in crate::module_profile::IN_CORE {
            if !list.iter().any(|p| p.module() == parts.module()) {
                list.push(*parts);
            }
        }
        list.sort_by_key(|p| Module::ALL.iter().position(|m| *m == p.module()));
        list
    })
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

    /// The modules' start, after the window's own.
    pub(crate) fn modules_started(&mut self) {
        for parts in all() {
            parts.started(self);
        }
    }

    /// The modules' work after every message.
    pub(crate) fn modules_follow(&mut self) -> Task<Message> {
        Task::batch(all().iter().map(|p| p.follow(self)).collect::<Vec<_>>())
    }

    /// A point picked on the drawing, to the module whose window asked for it; false when none did.
    pub(crate) fn module_picked(&mut self, at: Option<kentos_interaction::Vec2>) -> bool {
        all().iter().any(|p| p.picked(self, at))
    }

    /// What a module previews over the drawing while no tool does; the first module's that has one.
    pub(crate) fn module_preview(&self) -> Option<kentos_interaction::Preview> {
        all().iter().find_map(|p| p.preview(self))
    }

    /// A bottom tab of a module; empty when no module of this program has it.
    pub(crate) fn module_bottom_tab_view(&self, tab: BottomTab) -> Element<'_, Message> {
        all()
            .iter()
            .find_map(|p| p.bottom_tab_view(self, tab))
            .unwrap_or_else(|| iced::widget::Column::new().into())
    }

    /// The models the modules show beside the drawing's building in 3B görünüm.
    pub(crate) fn modules_beside(
        &self,
    ) -> Vec<(String, String, Arc<crate::underlay_model::Loaded>)> {
        all()
            .iter()
            .filter_map(|p| p.building3d())
            .flat_map(|b| b.beside(self))
            .collect()
    }

    /// An object of a model beside the drawing's chosen in 3D, to the module whose model it is.
    pub(crate) fn module_choose_beside(&mut self, id: &str, object: usize) {
        for parts in all() {
            if parts
                .building3d()
                .is_some_and(|b| b.choose_beside(self, id, object))
            {
                return;
            }
        }
    }

    /// What the objects the modules mark in 3D depend on, as one number.
    pub(crate) fn modules_marked_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        for b in all().iter().filter_map(|p| p.building3d()) {
            b.marked_key(self).hash(&mut h);
        }
        h.finish()
    }

    /// The objects the modules mark in 3D, by their keys.
    pub(crate) fn modules_marked(&self) -> BTreeSet<String> {
        all()
            .iter()
            .filter_map(|p| p.building3d())
            .flat_map(|b| b.marked(self))
            .collect()
    }

    /// A dock panel of a module; empty when no module of this program has it.
    pub(crate) fn module_panel_view(&self, panel: Panel) -> Element<'_, Message> {
        all()
            .iter()
            .find_map(|p| p.panel_view(self, panel))
            .unwrap_or_else(|| iced::widget::Column::new().into())
    }
}
