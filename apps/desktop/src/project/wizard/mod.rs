//! Dosya → Yeni proje as a wizard (docs/adr/0165 §3, DESIGN.md §7.10.1; the
//! web's `ui/settings/NewProjectWizard.ts`): the project's type, its
//! coordinates, its scale and details, one page at a time beside a rail of
//! the steps and what was chosen in each. The rules are the shared
//! `kentos_project::wizard`, held to the web's by fixtures/project/v1/wizard.json.
//! Nothing changes until Oluştur. Unsaved changes of the drawing on screen
//! are asked about then, over the wizard, so Vazgeç there comes back here;
//! an open cloud project is left first (its changes are sent or kept on
//! this device). The type chosen, and a local project's unit, start the
//! next wizard (Uygulama ayarları → Yeni projeler).

mod art;
mod pages;
mod strip;
#[cfg(test)]
mod tests;

use std::fmt;

use iced::widget::{Id, operation};
use iced::{Element, Task};
use kentos_contracts::{
    DOCUMENT_VERSION_2, DocumentSnapshotV2, DrawingFont, DrawingUnit, Workspace,
};
use kentos_interaction::Level;
use kentos_project::wizard::{Coords, Draft, Kind, STEPS, Step};
use kentos_ui::widget::{Wizard, overlay};

use super::content::{self, NewProject};
use super::{Event as ProjectEvent, Window, message};
use crate::app::{App, Dialog as Asking, Message, Then};
use crate::crs;
use crate::document::Document;

/// Yeni proje's note about the drawing on screen: a warning, or information.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Note {
    pub warn: bool,
    pub text: String,
}

/// What Yeni proje says of the drawing on screen before anything is done
/// (the web's newProjectNote.ts; docs/inventory/parity-audit.md N1): a cloud
/// project that saves by itself is closed and what waits is sent; unsaved
/// changes of a database project that does not save (read-only, archived,
/// gone) are not sent, and what to do is asked at Oluştur; a local
/// drawing's or a file project's unsaved changes (Kaydet saves them) get the
/// unsaved question at Oluştur. Nothing is said over a clean drawing.
/// `cloud`: the open project's name, whether it saves by itself now, and
/// whether it keeps objects in the database.
pub(crate) fn note(name: &str, cloud: Option<(&str, bool, bool)>, dirty: bool) -> Option<Note> {
    if let Some((project, true, _)) = cloud {
        return Some(Note {
            warn: false,
            text: format!(
                "“{project}” bulut projesi kapanır. Bekleyen değişiklikleri buluta gönderilir; gönderilemeyenler bu cihazda kalır ve proje yeniden açılınca geri gelir."
            ),
        });
    }
    if !dirty {
        return None;
    }
    Some(Note {
        warn: true,
        text: match cloud {
            Some((project, _, true)) => format!(
                "“{project}” projesindeki değişiklikleriniz buluta kaydedilmiyor; Oluştur’a basınca ne yapılacağı sorulur."
            ),
            _ => format!(
                "“{name}” içinde kaydedilmemiş değişiklikler var; Oluştur’a basınca önce sorulur."
            ),
        },
    })
}

pub struct State {
    draft: Draft,
    step: Step,
    /// The province search, kept while the wizard is open.
    query: String,
    /// Another scale typed in the “1:” field, as typed.
    own: String,
    /// Why the step could not be left forward, said in the footer.
    status: Option<String>,
    /// The drawing Oluştur built, while the drawing on screen is left.
    pending: Option<Box<Document>>,
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("State")
            .field("draft", &self.draft)
            .field("step", &self.step)
            .field("pending", &self.pending.is_some())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A type's card.
    Type(Workspace),
    /// A card's double click: the type chosen and the next step.
    Choose(Workspace),
    Coords(Coords),
    Unit(DrawingUnit),
    /// A province's row; the chosen one again chooses none.
    Province(u32),
    Search(String),
    /// Enter in the search: its first province.
    SearchFirst,
    System(u32),
    Scale(f64),
    /// The “1:” field as typed.
    OwnScale(String),
    Name(String),
    Font(DrawingFont),
    /// A step on the rail.
    Go(usize),
    Back,
    /// İleri, and Oluştur on the last step.
    Next,
}

fn event(e: Event) -> Message {
    message(ProjectEvent::New(e))
}

impl State {
    /// The app's last type and defaults for new projects (`newProjects.*`);
    /// a coordinate system this version does not know falls back to TUREF / TM36.
    pub fn new(app: &App) -> Self {
        let s = &app.settings;
        let font = serde_json::from_value(s.effective("newProjects.drawingFont"))
            .unwrap_or(DrawingFont::Barlow);
        let unit = serde_json::from_value(s.effective("newProjects.drawingUnit")).ok();
        Self {
            draft: Draft::initial(
                default_type(app),
                s.number("newProjects.srid") as u32,
                font,
                unit,
            ),
            step: Step::Type,
            query: String::new(),
            own: String::new(),
            status: None,
            pending: None,
        }
    }

    /// The step at `index`, if every step before it can be left.
    fn go(&mut self, index: usize) {
        let Some(to) = STEPS.get(index) else { return };
        for s in &STEPS[..index] {
            if let Some(why) = self.draft.blocked(*s) {
                self.step = *s;
                self.status = Some(why);
                return;
            }
        }
        self.step = *to;
        self.status = None;
    }
}

/// The type a new project starts with: the app's default for new projects
/// (`newProjects.workspace`) when it can be chosen, else CBS.
pub(super) fn default_type(app: &App) -> Workspace {
    serde_json::from_value(app.settings.effective("newProjects.workspace"))
        .ok()
        .filter(|w| ready(*w))
        .unwrap_or(crate::catalog::FALLBACK_MODE)
}

/// Whether a mode can be chosen yet (an announced one never is, even if a preference names it).
fn ready(w: Workspace) -> bool {
    crate::catalog::catalog()
        .modes()
        .iter()
        .any(|m| m.id == w && m.ready)
}

/// The drawing, without persistent ids or a source record yet (the web's
/// `replaceWith` of a new project): the first save gives them.
pub fn new_document(o: &NewProject) -> Result<Document, String> {
    let v1 = content::new_project(o)?;
    Document::from_v2(
        DocumentSnapshotV2 {
            format: v1.format,
            version: DOCUMENT_VERSION_2,
            name: v1.name,
            settings: v1.settings,
            origin: v1.origin,
            home_view: v1.home_view,
            layers: v1.layers,
            active_layer: v1.active_layer,
            entities: Vec::new(),
            uids: Vec::new(),
            styles: v1.styles,
            blocks: v1.blocks,
            project_id: None,
            migrated_from: None,
        },
        None,
    )
}

/// The text fields the wizard gives the keyboard to.
pub(super) const SEARCH: &str = "yeni-proje-il";
pub(super) const NAME: &str = "yeni-proje-adi";

/// A page shown: the keyboard to its field, as on the web: the province's
/// search on a page with places, the name on the last, its text chosen, so
/// that typing replaces it.
fn focus(s: &State) -> Task<Message> {
    match s.step {
        Step::Coords if !s.draft.is_local() => operation::focus(Id::new(SEARCH)),
        Step::Details => Task::batch([
            operation::focus(Id::new(NAME)),
            operation::select_all(Id::new(NAME)),
        ]),
        _ => Task::none(),
    }
}

/// A scale typed as 1:N: N a whole number over 0, dots and spaces between its digits allowed.
fn typed_scale(text: &str) -> Option<f64> {
    let digits: String = text
        .chars()
        .filter(|c| *c != '.' && !c.is_whitespace())
        .collect();
    let n: u64 = digits.parse().ok()?;
    (n > 0 && n < 100_000_000).then_some(n as f64)
}

impl App {
    pub(super) fn new_project_event(&mut self, e: Event) -> Task<Message> {
        let Some(Window::New(s)) = &mut self.project else {
            return Task::none();
        };
        let d = &mut s.draft;
        match e {
            Event::Type(w) | Event::Choose(w) => {
                if ready(w) {
                    d.kind = if w == Workspace::Cad {
                        Kind::Cad
                    } else {
                        Kind::Gis
                    };
                    // Another type offers other scales: the chosen one goes back to the type's own.
                    d.plot_scale = None;
                    s.own.clear();
                }
                if matches!(e, Event::Choose(_)) {
                    return self.new_project_next();
                }
            }
            Event::Coords(c) => {
                d.coords = c;
                d.plot_scale = None;
                s.own.clear();
                s.status = None;
                return focus(s);
            }
            Event::Unit(u) => d.unit = u,
            Event::Province(code) => {
                d.province = (d.province != Some(code)).then_some(code);
                // Another province suggests its own zone: a system chosen before follows it again.
                d.srid = None;
            }
            Event::Search(query) => s.query = query,
            Event::SearchFirst => {
                if let Some(p) = kentos_project::provinces::search(&s.query).first() {
                    d.province = Some(p.code);
                    d.srid = None;
                }
            }
            Event::System(srid) => d.srid = Some(srid),
            Event::Scale(n) => {
                d.plot_scale = Some(n);
                s.own.clear();
            }
            Event::OwnScale(text) => {
                if let Some(n) = typed_scale(&text) {
                    d.plot_scale = Some(n);
                }
                s.own = text;
            }
            Event::Name(name) => d.name = name,
            Event::Font(font) => d.font = font,
            Event::Go(index) => {
                s.go(index);
                return focus(s);
            }
            Event::Back => {
                s.go(s.step.index().saturating_sub(1));
                return focus(s);
            }
            Event::Next => return self.new_project_next(),
        }
        s.status = None;
        Task::none()
    }

    /// İleri: the next step, if this one can be left; Oluştur on the last.
    fn new_project_next(&mut self) -> Task<Message> {
        let Some(Window::New(s)) = &mut self.project else {
            return Task::none();
        };
        if let Some(why) = s.draft.blocked(s.step) {
            s.status = Some(why);
            return Task::none();
        }
        let at = s.step.index();
        if at + 1 < STEPS.len() {
            s.go(at + 1);
            return focus(s);
        }
        match new_document(&s.draft.options()) {
            Err(e) => {
                s.status = Some(e);
                Task::none()
            }
            Ok(doc) => {
                s.pending = Some(Box::new(doc));
                self.leave(Then::NewProject)
            }
        }
    }

    /// The drawing on screen is left: the new project goes on screen, on its
    /// home view, its start view (docs/adr/0165 §3; the web's `newProject`).
    pub(crate) fn new_project_ready(&mut self) -> Task<Message> {
        let Some(Window::New(s)) = &mut self.project else {
            return Task::none();
        };
        let Some(doc) = s.pending.take() else {
            return Task::none();
        };
        let (kind, unit) = (s.draft.kind, s.draft.is_local().then_some(s.draft.unit));
        self.close_project_window();
        let settings = doc.settings().clone();
        let name = doc.name().to_owned();
        // The viewport fits its home view, once the drawing area has its size.
        self.show_document(*doc);
        // The next wizard starts on this type, and a local project's unit.
        let mut remembered = vec![(
            "newProjects.workspace",
            serde_json::to_value(kind.workspace()).unwrap_or_default(),
        )];
        if let Some(unit) = unit {
            remembered.push((
                "newProjects.drawingUnit",
                serde_json::to_value(unit).unwrap_or_default(),
            ));
        }
        let _ = self.settings.choose(&remembered);
        let system = crs::title_of(settings.srid);
        self.say(
            Level::Success,
            format!(
                "“{name}” yeni projesi açıldı: {system}, 1:{}. İlk kayıtta dosyanın yeri sorulur.",
                content::js_number(settings.plot_scale)
            ),
        );
        Task::none()
    }

    /// Vazgeç on the unsaved question: the wizard comes back, nothing built kept.
    pub(crate) fn new_project_declined(&mut self) {
        if let Some(Window::New(s)) = &mut self.project {
            s.pending = None;
            self.dialog = Some(Asking::Project);
        }
    }

    /// Enter with no field taking it: İleri, or Oluştur on the last step.
    /// ← and → choose the type on the first step (the web's radio group).
    pub(crate) fn new_project_key(
        &mut self,
        key: iced::keyboard::key::Named,
    ) -> Option<Task<Message>> {
        use iced::keyboard::key::Named;
        let Some(Window::New(s)) = &self.project else {
            return None;
        };
        match key {
            Named::Enter => Some(self.new_project_next()),
            Named::ArrowLeft | Named::ArrowRight if s.step == Step::Type => {
                let other = match s.draft.kind {
                    Kind::Cad => Workspace::Gis,
                    Kind::Gis => Workspace::Cad,
                };
                Some(self.new_project_event(Event::Type(other)))
            }
            _ => None,
        }
    }

    /// What Yeni proje says of the drawing on screen: a database project
    /// that saves by itself sends what waits; one that does not (a viewer's,
    /// an archived or ended one) says its edits are not sent.
    fn new_project_note(&self) -> Option<Note> {
        let doc = self.document.as_ref()?;
        let live = self
            .cloud
            .live
            .as_ref()
            .filter(|l| l.session == doc.session && doc.is_database());
        let cloud = doc.cloud_source().map(|s| {
            let autosaves = live.is_some_and(|l| {
                let state = l.sync.state();
                !state.ended() && state != kentos_cloud::SaveState::ReadOnly
            });
            (s.info.name.as_str(), autosaves, doc.is_database())
        });
        // A database project's unsaved work is what it has not sent.
        let dirty = match live {
            Some(l) => !l.sync.all_sent(),
            None => doc.dirty(),
        };
        note(doc.name(), cloud, dirty)
    }

    pub(super) fn new_project_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let page = match s.step {
            Step::Type => pages::kind(s, &self.theme()),
            Step::Coords => pages::coords(s),
            Step::Details => pages::details(s, self.new_project_note()),
        };
        let mut wizard = Wizard::new("Yeni proje", STEPS.map(Step::name))
            .rail(STEPS.map(|step| s.draft.note(step)))
            .current(s.step.index())
            .body(page)
            .on_step(|i| event(Event::Go(i)))
            .back(event(Event::Back))
            .next(Some(event(Event::Next)))
            .finish("Oluştur", Some(event(Event::Next)))
            .on_cancel(message(ProjectEvent::Close))
            // 1040 × 704 at the default text size (the web's); a lower window keeps a margin.
            .size(960.0, 650.0);
        if let Some(status) = &s.status {
            wizard = wizard.problem(status.clone());
        }
        overlay::blocking(iced::widget::container(wizard).padding(24))
    }
}
