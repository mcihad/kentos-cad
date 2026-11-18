//! 3B görünüm of a building (docs/adr/8067, 8119; plan/bim-plan.md BIM-06): the model viewer (crates/model-view-ui;
//! docs/adr/8073, 8118) over the drawing's own model — the IFC file the export would write now, written in memory and
//! read back with the reading face (ifc_viewer.rs's reading), so what the architect sees is what the file holds — and
//! the underlay models shown beside it (docs/adr/8101), their objects apart and violet. The viewer's tree with its
//! search, its details, its section card and its switches are its own; this module gives it the drawing:
//!
//! - **Made off the window's thread** from a reading copy (docs/adr/8115 K220): written, read, its bodies made; the
//!   window says so until the first model shows.
//! - **Live:** while the window is open an edit, an undo, or an underlay shown or hidden makes it again; the view stays
//!   as it was (the camera, the open branches, the search, the section, the panes) and the hidden, chosen and found
//!   objects stay by their keys. A change while one is being made makes it once more after it.
//! - **With the plan:** the plan's selection is chosen in the viewer and Denetim's findings are painted; a click's
//!   object is selected in the plan (a click on nothing empties it; an underlay's object shows in Altlık modeller).
//! - **Katlar ▾** at the strip's end: the drawing's storeys, each ticked while it shows (the tree's eye of the storey).
//!
//! It is a view: it never changes the drawing.
//!
//! ```text
//! ┌ 3B görünüm — KentOS CAD ────────────────────────────────────────────────────────────────────────────┐
//! │ ◈ Kuzeydoğu ▾ ⤢ Sığdır ◎ Seçime yaklaş │ Gizle … │ ▱ Mahaller ▭ Açıklıklar ◐ Saydam │ Kesit ▾  ▤ Katlar ▾ ⧉ │
//! ├ Yapı | Sınıf | Gösterilemeyen ┬────────────────────────────────────────────────┬ Özellikler ───────────┤
//! │ ⌕ Ad, sınıf ya da GlobalId    │                                           (K)  │ ▾ Nesne               │
//! │ ▾ Proje                       │     the drawing's model and the underlays      │   Sınıf    IfcWall    │
//! ├───────────────────────────────┴────────────────────────────────────────────────┴───────────────────────┤
//! │ ⌖ Dış_duvar_25  Y …  X …  Z …                    72/72 gövde │ IFC4X3_ADD2 │ ⚠ 2 nesnede bulgu │ ? Fare │
//! └───────────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use iced::futures::channel::oneshot;
use iced::{Element, Task};
use kentos_commands::Entity;
use kentos_domain::{Document, Uuid};
use kentos_ifc::read::Model;
use kentos_ifc::read::bodies::Bodies;
use kentos_model_view::{BodyIn, Category, Details, Input};
use kentos_model_view_ui::{Out, Stage, Viewer};
use kentos_ui::widget::Menu;

use crate::app::{App, Message};
use crate::scene3d_window::{Event, Window};
use crate::style::strip;
use crate::traces::Control;
use crate::underlays::Loaded;

/// How the viewer keys an underlay's object, `altlik:<underlay id>:<its GlobalId>`, and how Altlık modeller is asked
/// for one, `altlik:<underlay id>:<its index in the file>` ([`App::building3d_select`]).
pub(crate) const UNDERLAY_OWNER: &str = "altlik:";
const STOREYS: &str = "Katlar";
const ALL_STOREYS: &str = "Bütün katları göster";
/// The application the drawing's model names in its header: nothing reads it back.
const APPLICATION: &str = "KentOS Mimari";
/// Why 3B görünüm has nothing to show.
pub(crate) const NO_BUILDING: &str = "3B görünümde gösterilecek bir bina gövdesi yok: önce duvar, döşeme, kolon ya da çatı çizin (yol projesinde 3B görünüm araziyi ve koridorları gösterir).";
const FOUND_TIP: &str = "Denetim'de seçilen bulguların nesneleri kırmızıdır; satır seçilmemişse listelenen bütün bulguların. Çizim değişince bulgular Denetle'ye dek gösterilmez.";

static NEXT: AtomicU64 = AtomicU64::new(1);

/// A model in the viewer's input: where its objects start, and what their details are read from.
#[derive(Clone)]
enum Part {
    Drawing { from: usize, bytes: Arc<Vec<u8>>, model: Arc<Model> },
    Underlay { from: usize, id: String, loaded: Arc<Loaded> },
}

impl Part {
    fn from(&self) -> usize {
        match self {
            Part::Drawing { from, .. } | Part::Underlay { from, .. } => *from,
        }
    }
}

/// What a making gives: the viewer's input, the models in it, what the status bar says of the drawing's model.
#[derive(Clone)]
pub(crate) struct Made {
    input: Input,
    parts: Vec<Part>,
    facts: Vec<(String, String)>,
}

impl std::fmt::Debug for Made {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Made").field("objects", &self.input.objects.len()).field("bodies", &self.input.bodies.len()).finish()
    }
}

/// An underlay shown: its id, its name and its file as read.
type Shown = (String, String, Arc<Loaded>);

/// Whether the drawing has what makes a building's body (asked before the model is made): a drawing of rooms or
/// blocks alone has none.
fn has_building(doc: &Document) -> bool {
    doc.entities().any(|e| {
        matches!(
            e,
            Entity::Wall(_)
                | Entity::Slab(_)
                | Entity::Column(_)
                | Entity::Beam(_)
                | Entity::Roof(_)
                | Entity::Stair(_)
                | Entity::Ramp(_)
                | Entity::Railing(_)
        )
    })
}

/// Makes the viewer's input of `doc` (a reading copy) and of the underlays shown: the model the export would write
/// now, read back, with its bodies; none when stopped.
fn make(doc: &Document, underlays: &[Shown], stop: &AtomicBool) -> Option<Result<Made, String>> {
    let (building, _) = kentos_native_application::arch_ifc::building(doc, "", APPLICATION);
    let name = format!("{}.ifc", building.file);
    let text = kentos_ifc::building::write(&building);
    let read = match crate::ifc_viewer::read_bytes(&name, text.into_bytes(), stop, &mut |_| {})? {
        Ok(r) => r,
        Err(why) => {
            return Some(Err(format!(
                "Çizimin modeli geri okunamadı: {why}. Bu KentOS'un hatasıdır; çizimi ve bu iletiyi bildirin."
            )));
        }
    };
    if !read.input.bodies.iter().any(|b| !read.input.objects[b.object].category.is_volume()) {
        return Some(Err(NO_BUILDING.to_owned()));
    }
    let facts = vec![(read.model.summary.schema.clone(), crate::ifc_viewer::schema_tip(&read.model))];
    let mut input = read.input;
    let mut parts = vec![Part::Drawing { from: 0, bytes: read.bytes, model: read.model }];
    for (id, name, loaded) in underlays {
        if stop.load(Ordering::Relaxed) {
            return None;
        }
        // Its objects as its file says them, its project by the underlay's name; its bodies as they stand in the
        // drawing (placed when it was read).
        let none = Bodies { bodies: Vec::new(), not_shown: Vec::new() };
        let mut other = kentos_ifc::read::view::input(name, &loaded.model, &none);
        for o in other.objects.iter_mut().filter(|o| o.category == Category::Project) {
            o.label = format!("{name} (altlık model)");
        }
        other.bodies = loaded
            .bodies
            .iter()
            .map(|b| BodyIn { object: b.owner as usize, positions: b.positions.clone(), triangles: b.triangles.clone() })
            .collect();
        other.not_shown = loaded.not_shown.clone();
        let from = input.objects.len();
        input.append(other, &format!("{UNDERLAY_OWNER}{id}:"));
        parts.push(Part::Underlay { from, id: id.clone(), loaded: loaded.clone() });
    }
    Some(Ok(Made { input, parts, facts }))
}

/// A building in the window.
pub(crate) struct Building3d {
    pub(crate) viewer: Viewer,
    parts: Vec<Part>,
    /// What the status bar says of the drawing's model (its schema), before the findings.
    schema: Vec<(String, String)>,
    /// The drawing's generation and revision, and the underlays (their ids and files), the model was made from.
    made: (u64, u64),
    made_with: Vec<(String, usize)>,
    /// The making under way, its number and stop; whether the drawing changed again while it ran.
    making: Option<(u64, Arc<AtomicBool>)>,
    again: bool,
    /// The plan's selection version and Denetim's findings key the viewer took last.
    taken: u64,
    found: Option<u64>,
    /// The underlay models shown in it (docs/adr/8101).
    pub(crate) underlays: Vec<Shown>,
}

impl std::fmt::Debug for Building3d {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Building3d").field("viewer", &self.viewer).field("made", &self.made).finish_non_exhaustive()
    }
}

/// A closed window stops its making: what it would have given never shows.
impl Drop for Building3d {
    fn drop(&mut self) {
        if let Some((_, stop)) = &self.making {
            stop.store(true, Ordering::Relaxed);
        }
    }
}

impl Building3d {
    /// How many of the drawing's objects have an element's body (a room and an opening are no body): what the IFC
    /// file holds.
    pub(crate) fn bodies(&self) -> usize {
        self.with_body(|c| !c.is_volume())
    }

    /// How many of the drawing's objects of `class` have a body (rooms, the far zones).
    #[cfg(test)]
    pub(crate) fn with_body_of(&self, class: &str) -> usize {
        let Some(v) = self.viewer.view() else { return 0 };
        let input = v.input();
        let mut objects: Vec<usize> =
            input.bodies.iter().map(|b| b.object).filter(|&o| !input.is_apart(o) && input.objects[o].class == class).collect();
        objects.sort_unstable();
        objects.dedup();
        objects.len()
    }

    /// How many of the drawing's objects of the categories `of` have a body (an object's bodies are one after another).
    fn with_body(&self, of: impl Fn(Category) -> bool) -> usize {
        let Some(v) = self.viewer.view() else { return 0 };
        let input = v.input();
        let mut objects: Vec<usize> =
            input.bodies.iter().map(|b| b.object).filter(|&o| !input.is_apart(o) && of(input.objects[o].category)).collect();
        objects.sort_unstable();
        objects.dedup();
        objects.len()
    }

    /// How many bodies the viewer has: the drawing's, its rooms', the underlays'.
    pub(crate) fn count(&self) -> usize {
        self.viewer.view().map_or(0, |v| v.input().bodies.len())
    }

    /// The details of `object` (an index into the viewer's input), read from its model's file.
    fn details(&self, object: usize) -> Option<Result<Details, String>> {
        let part = self.parts.iter().rev().find(|p| p.from() <= object)?;
        let at = object - part.from();
        Some(match part {
            Part::Drawing { bytes, model, .. } => model.details(bytes, at).map_err(|e| e.to_string()),
            Part::Underlay { loaded, .. } => loaded.model.details(&loaded.bytes, at).map_err(|e| e.to_string()),
        })
    }

    /// The chosen object's details read and given to the viewer; why they could not be read.
    fn read_details(&mut self) -> Option<String> {
        let object = self.viewer.view()?.selected()?;
        match self.details(object)? {
            Ok(d) => {
                self.viewer.set_details(object, d);
                None
            }
            Err(e) => Some(e),
        }
    }

    /// An underlay's object by its index into the viewer's input: the underlay's id and the object's index in its file.
    fn underlay_object(&self, object: usize) -> Option<(String, usize)> {
        match self.parts.iter().rev().find(|p| p.from() <= object)? {
            Part::Underlay { from, id, .. } => Some((id.clone(), object - from)),
            Part::Drawing { .. } => None,
        }
    }

    /// What the status bar says of the model: its schema, and how many objects the findings painted are on.
    fn facts(&self) -> Vec<(String, String)> {
        let mut out = self.schema.clone();
        let found = self.viewer.view().map_or(0, |v| v.found().len());
        if found > 0 {
            out.push((format!("{found} nesnede bulgu"), FOUND_TIP.to_owned()));
        }
        out
    }

    /// The drawing's storeys in the viewer: each one's object, its name as the drawing says it (the long name the
    /// tree writes after “—”), and whether it shows.
    fn storeys(&self) -> Vec<(usize, String, bool)> {
        let Some(v) = self.viewer.view() else { return Vec::new() };
        v.input()
            .objects
            .iter()
            .enumerate()
            .filter(|(i, o)| o.category == Category::Storey && !v.input().is_apart(*i))
            .map(|(i, o)| {
                let name = o.label.rsplit_once(" — ").map_or(o.label.as_str(), |(_, long)| long).to_owned();
                (i, name, !v.is_hidden(i))
            })
            .collect()
    }

    /// How many storeys the drawing's model has.
    #[cfg(test)]
    pub(crate) fn storeys_count(&self) -> usize {
        self.storeys().len()
    }

    /// The legend's swatches as the scene shows them now.
    #[cfg(test)]
    pub(crate) fn legend(&self) -> Vec<(&'static str, [f32; 4])> {
        self.viewer.view().map_or_else(Vec::new, |v| v.legend())
    }

    /// How many objects the findings painted are on.
    #[cfg(test)]
    pub(crate) fn found_shown(&self) -> usize {
        self.viewer.view().map_or(0, |v| v.found().len())
    }

    /// Katlar ▾ at the strip's end: a menu of ticks, each storey's while it shows (the menu stays open while ticking),
    /// all of them back at once when some are hidden; its words say how many show when not all do.
    pub(crate) fn storeys_menu(&self) -> Element<'_, Message> {
        let storeys = self.storeys();
        let shown = storeys.iter().filter(|s| s.2).count();
        let words = if shown == storeys.len() { STOREYS.to_owned() } else { format!("{STOREYS} {shown}/{}", storeys.len()) };
        strip::menu(
            crate::icons::from_web(Some("levelsAll")),
            &words,
            "Katlar: gösterilen katlar. Menü açıkken katlar art arda işaretlenir; gizlenen katın gövdeleri ve mahalleri görünmez.",
            move || {
                let mut m = Menu::new();
                for (i, (_, name, on)) in storeys.iter().enumerate() {
                    m = m.check(name.clone(), *on, Message::Scene3d(Event::Storey(i))).stay();
                }
                if storeys.iter().any(|s| !s.2) {
                    m = m.separator().item(ALL_STOREYS, Message::Scene3d(Event::AllStoreys));
                }
                m
            },
        )
    }

    /// The `i`th storey in Katlar shown or hidden.
    pub(crate) fn toggle_storey(&mut self, i: usize) {
        if let Some((object, ..)) = self.storeys().get(i) {
            let _ = self.viewer.update(kentos_model_view_ui::Message::Eye(*object));
        }
    }

    /// Every storey shown again.
    pub(crate) fn show_all_storeys(&mut self) {
        for (object, _, on) in self.storeys() {
            if !on {
                let _ = self.viewer.update(kentos_model_view_ui::Message::Eye(object));
            }
        }
    }

    /// A trace's press on a storey's tick or on Bütün katları göster; the rest is the viewer's.
    pub(crate) fn control(&self, control: &Control<'_>) -> Option<Message> {
        match control {
            Control::Press(ALL_STOREYS) => Some(Message::Scene3d(Event::AllStoreys)),
            Control::Press(name) => {
                self.storeys().iter().position(|s| s.1 == *name).map(|i| Message::Scene3d(Event::Storey(i)))
            }
            _ => None,
        }
    }
}

/// The underlays as shown: their ids and files (a file read again is another).
fn shown_ids(shown: &[Shown]) -> Vec<(String, usize)> {
    shown.iter().map(|(id, _, l)| (id.clone(), Arc::as_ptr(l) as usize)).collect()
}

/// The drawing's persistent id of a GlobalId key; none for an underlay's object or a key that is no GlobalId.
pub(crate) fn uid_of(key: &str) -> Option<Uuid> {
    if key.starts_with(UNDERLAY_OWNER) {
        return None;
    }
    kentos_ifc::step::unguid(key).map(Uuid::from_u128)
}

impl App {
    /// The underlay models shown, as 3B görünüm takes them.
    fn building3d_shown(&self) -> Vec<Shown> {
        self.underlays_shown().into_iter().map(|(u, l)| (u.id, u.name, l)).collect()
    }

    /// 3B görünüm of the drawing's building: the viewer over its model, made off this thread; a drawing without a
    /// building's body says so and opens nothing.
    pub(crate) fn open_building3d(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else { return Task::none() };
        if !has_building(&doc.model) {
            self.warn(NO_BUILDING.to_owned());
            return Task::none();
        }
        let mut viewer = Viewer::new(doc.name());
        viewer.progress("Çizimin modeli kuruluyor", 0, 0);
        let seen = viewer.seen.clone();
        let b = Building3d {
            viewer,
            parts: Vec::new(),
            schema: Vec::new(),
            made: (0, 0),
            made_with: Vec::new(),
            making: None,
            again: false,
            taken: 0,
            found: None,
            underlays: Vec::new(),
        };
        let mut window = Window::new(crate::scene3d_window::Scene::of(Vec::new(), 0), Vec::new(), Some(b), (0, 0));
        // The scene drawn is the viewer's: its frames are the window's.
        window.seen = seen;
        self.scene3d_window = Some(Box::new(window));
        self.show_surface(crate::windows::Surface::Scene3d);
        self.building3d_make()
    }

    /// The model made again from the drawing as it is now, off this thread; while one is being made, once more after.
    fn building3d_make(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else { return Task::none() };
        let now = (doc.model.generation(), doc.model.revision());
        let shown = self.building3d_shown();
        let Some(b) = self.scene3d_window.as_mut().and_then(|w| w.building.as_mut()) else { return Task::none() };
        if b.making.is_some() {
            b.again = true;
            return Task::none();
        }
        let copy = doc.model.reading_copy();
        let id = NEXT.fetch_add(1, Ordering::Relaxed);
        let stop = Arc::new(AtomicBool::new(false));
        b.making = Some((id, stop.clone()));
        b.made = now;
        b.made_with = shown_ids(&shown);
        b.underlays = shown.clone();
        // A test runs no task: the model is made here and now, as its thread would give it.
        if cfg!(test) {
            return match make(&copy, &shown, &stop) {
                Some(result) => self.building3d_made(id, result.map(Arc::new)),
                None => Task::none(),
            };
        }
        let (done, wait) = oneshot::channel();
        std::thread::spawn(move || {
            let _ = done.send(make(&copy, &shown, &stop));
        });
        Task::perform(wait, move |made| {
            let result = match made {
                Ok(Some(result)) => result.map(Arc::new),
                // Stopped: what it would have given never shows.
                Ok(None) => Err(String::new()),
                Err(_) => Err("Çizimin modeli kurulurken iş yarıda kaldı; 3B görünümü yeniden açın.".to_owned()),
            };
            Message::Scene3d(Event::Made(id, result))
        })
    }

    /// A making done: the viewer loaded the first time, else loaded again keeping its view; the chosen object's details
    /// read again; the plan's selection and the findings taken again. A drawing left without a building's body closes
    /// the window.
    pub(crate) fn building3d_made(&mut self, id: u64, result: Result<Arc<Made>, String>) -> Task<Message> {
        let Some(b) = self.scene3d_window.as_mut().and_then(|w| w.building.as_mut()) else { return Task::none() };
        if b.making.as_ref().is_none_or(|(now, _)| *now != id) {
            return Task::none();
        }
        b.making = None;
        match result {
            Ok(made) => {
                let made = Arc::try_unwrap(made).unwrap_or_else(|m| (*m).clone());
                b.schema = made.facts.clone();
                if b.viewer.view().is_none() {
                    b.viewer.load(made.input, made.facts);
                } else {
                    b.viewer.reload(made.input, made.facts);
                }
                b.parts = made.parts;
                let _ = b.read_details();
                // Taken again: the new model chooses and paints as the plan and Denetim say now.
                b.taken = 0;
                b.found = None;
            }
            // Stopped (the window closed meanwhile): nothing to do.
            Err(why) if why.is_empty() => {}
            Err(why) if why == NO_BUILDING => {
                self.scene3d_gone();
                self.warn("Binada gösterilecek gövde kalmadı; 3B görünüm kapandı.".to_owned());
                return Task::none();
            }
            Err(why) if b.viewer.view().is_none() => b.viewer.failed(why),
            Err(why) => self.warn(format!("3B görünüm yenilenemedi: {why}")),
        }
        self.building3d_follow_choice();
        let again = self.scene3d_window.as_mut().and_then(|w| w.building.as_mut()).is_some_and(|b| std::mem::take(&mut b.again));
        if again { self.building3d_make() } else { Task::none() }
    }

    /// A viewer's message: what it answers handled (a choice: the details read and the plan's object selected; İptal
    /// and Kapat: the window closes).
    pub(crate) fn building3d_viewer(&mut self, m: kentos_model_view_ui::Message) {
        let Some(b) = self.scene3d_window.as_mut().and_then(|w| w.building.as_mut()) else { return };
        match b.viewer.update(m) {
            Some(Out::Chosen(chosen)) => self.building3d_chosen(chosen),
            Some(Out::Cancel | Out::Close) => self.scene3d_gone(),
            None => {}
        }
    }

    /// An object chosen in the viewer: its details read, the plan's object selected (a choice of nothing empties the
    /// plan's selection, as a click on nothing in the plan does; an underlay's object shows in Altlık modeller).
    fn building3d_chosen(&mut self, chosen: Option<(usize, String)>) {
        let Some((object, key)) = chosen else {
            self.building3d_select(None);
            return;
        };
        let Some(b) = self.scene3d_window.as_mut().and_then(|w| w.building.as_mut()) else { return };
        let unread = b.read_details();
        let underlay = b.underlay_object(object);
        if let Some(e) = unread {
            self.say(kentos_interaction::Level::Warn, format!("Nesnenin özellikleri okunamadı: {e}"));
        }
        match underlay {
            Some((id, at)) => self.building3d_select(Some(format!("{UNDERLAY_OWNER}{id}:{at}"))),
            None => self.building3d_select(uid_of(&key).map(|u| u.to_string())),
        }
    }

    /// An object selected in the plan by its persistent id (none, or what the drawing has no object for, empties the
    /// selection, as a click on nothing in the plan does) and chosen in 3D; an underlay's object
    /// (`altlik:<underlay id>:<its index in the file>`) shown in Altlık modeller, the plan's selection kept.
    pub(crate) fn building3d_select(&mut self, owner: Option<String>) {
        if let Some(rest) = owner.as_deref().and_then(|o| o.strip_prefix(UNDERLAY_OWNER))
            && let Some((id, object)) = rest.rsplit_once(':')
            && let Ok(object) = object.parse::<usize>()
        {
            if self.arch.underlays_window.is_none() {
                self.open_underlays();
            }
            if let Some(w) = self.arch.underlays_window.as_mut() {
                w.chosen = Some(id.to_owned());
                w.object = Some((id.to_owned(), object));
            }
            return;
        }
        let Some(doc) = self.document.as_ref() else { return };
        let uid = owner.as_deref().and_then(|u| Uuid::parse_str(u).ok());
        let slots: Vec<kentos_domain::Slot> = uid.and_then(|u| doc.model.slot_of(u)).into_iter().collect();
        self.selection.set(slots);
        let version = self.selection.version();
        if let Some(b) = self.scene3d_window.as_mut().and_then(|w| w.building.as_mut()) {
            // Taken now: following the plan does not choose it again.
            b.taken = version;
            if let Some(u) = uid {
                let _ = b.viewer.choose_key(&kentos_ifc::step::guid(u.as_u128()));
            }
        }
    }

    /// 3B görünüm follows the drawing while its window is open (`App::follow_scene3d`): the drawing or the underlays
    /// shown changed, the model is made again; the plan's selection and Denetim's findings are taken when they changed.
    pub(crate) fn building3d_follow(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else { return Task::none() };
        let now = (doc.model.generation(), doc.model.revision());
        let with = shown_ids(&self.building3d_shown());
        let stale = self
            .scene3d_window
            .as_ref()
            .and_then(|w| w.building.as_ref())
            .is_some_and(|b| b.made != now || b.made_with != with);
        let task = if stale { self.building3d_make() } else { Task::none() };
        self.building3d_follow_choice();
        task
    }

    /// The plan's first selected object chosen in the viewer, and Denetim's findings painted, when either changed since
    /// the viewer took them.
    fn building3d_follow_choice(&mut self) {
        let version = self.selection.version();
        let key = self.building3d_found_key();
        let Some(b) = self.scene3d_window.as_ref().and_then(|w| w.building.as_ref()).filter(|b| b.viewer.stage == Stage::Shown)
        else {
            return;
        };
        let (choose, paint) = (b.taken != version, b.found != Some(key));
        if !choose && !paint {
            return;
        }
        let first = self
            .document
            .as_ref()
            .and_then(|doc| self.selection.ids().iter().find_map(|s| doc.model.uid(*s)))
            .map(|u| kentos_ifc::step::guid(u.as_u128()));
        let found: Vec<String> = if paint {
            self.building3d_found()
                .iter()
                .filter_map(|u| Uuid::parse_str(u).ok())
                .map(|u| kentos_ifc::step::guid(u.as_u128()))
                .collect()
        } else {
            Vec::new()
        };
        let Some(b) = self.scene3d_window.as_mut().and_then(|w| w.building.as_mut()) else { return };
        if paint {
            b.found = Some(key);
            b.viewer.set_found(found.iter().map(String::as_str));
            b.viewer.facts = b.facts();
        }
        if choose {
            b.taken = version;
            match first {
                Some(key) => {
                    if b.viewer.choose_key(&key) {
                        let _ = b.read_details();
                    }
                }
                // Nothing selected in the plan: nothing chosen in 3D.
                None => {
                    let _ = b.viewer.update(kentos_model_view_ui::Message::Choose(None));
                }
            }
        }
    }

    /// What the findings shown in 3D depend on, as one number: the check (its drawing's state and how many it
    /// found), the rows chosen, the group shown and whether the drawing changed since.
    pub(crate) fn building3d_found_key(&self) -> u64 {
        use std::hash::{Hash, Hasher};
        let a = &self.arch.arch_audit;
        let mut h = std::collections::hash_map::DefaultHasher::new();
        a.audited.as_ref().map(|x| (x.generation, x.revision, x.findings.len())).hash(&mut h);
        a.selected.hash(&mut h);
        a.chosen.hash(&mut h);
        a.group.hash(&mut h);
        self.arch_audit_stale().hash(&mut h);
        h.finish()
    }

    /// The objects of the findings chosen in Denetim, else of all it lists (its last check, while the drawing is as it
    /// saw it): a drawing's findings (a layer's name, a room's number) can name nearly every object, so the rows chosen
    /// say what to look at.
    pub(crate) fn building3d_found(&self) -> std::collections::BTreeSet<String> {
        if self.arch_audit_stale() {
            return std::collections::BTreeSet::new();
        }
        let chosen = self.arch_audit_chosen();
        if !chosen.is_empty() {
            return chosen.into_iter().flat_map(|f| f.uids.iter().cloned()).collect();
        }
        let Some(a) = self.arch.arch_audit.audited.as_ref() else { return std::collections::BTreeSet::new() };
        self.arch_audit_shown().into_iter().filter_map(|i| a.findings.get(i)).flat_map(|f| f.uids.iter().cloned()).collect()
    }
}
