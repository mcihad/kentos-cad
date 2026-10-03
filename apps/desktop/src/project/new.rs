//! Dosya → Yeni proje (the web's `ui/settings/NewProjectDialog.ts`): an
//! empty drawing with the standard layer tree, a work mode, a coordinate
//! system (the app's default for new projects first) and a plot scale.
//! Nothing changes until Oluştur. Unsaved changes of the drawing on screen
//! are asked about then, over this window, so Vazgeç there comes back here;
//! an open cloud project is left first (its changes are sent or kept on
//! this device).

use std::fmt;

use iced::widget::{Column, text_input};
use iced::{Element, Task};
use kentos_contracts::{DOCUMENT_VERSION_2, DocumentSnapshotV2, DrawingFont, Workspace};
use kentos_interaction::Level;
use kentos_ui::widget::{Banner, Dialog, overlay};
use kentos_ui::{style, theme::typography};

use super::content::{self, NewProject};
use super::{Event as ProjectEvent, Window, group, message, modes, scales, setting};
use crate::app::{App, Dialog as Asking, Message, Then};
use crate::crs::{self, PickFor};
use crate::document::Document;
use crate::exchange::words;

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
    name: String,
    srid: u32,
    plot_scale: f64,
    workspace: Workspace,
    drawing_font: DrawingFont,
    /// The coordinate system list's search, kept while the window re-renders.
    query: String,
    status: Option<String>,
    /// The drawing Oluştur built, while the drawing on screen is left.
    pending: Option<Box<Document>>,
}

impl fmt::Debug for State {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("State")
            .field("name", &self.name)
            .field("srid", &self.srid)
            .field("plot_scale", &self.plot_scale)
            .field("workspace", &self.workspace)
            .field("pending", &self.pending.is_some())
            .finish_non_exhaustive()
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    Name(String),
    Scale(f64),
    Mode(Workspace),
    Crs(u32),
    Search(String),
    Create,
}

fn event(e: Event) -> Message {
    message(ProjectEvent::New(e))
}

impl State {
    /// The app's defaults for new projects (`newProjects.*`); a coordinate
    /// system this version does not know falls back to TUREF / TM36.
    pub fn new(app: &App) -> Self {
        let s = &app.settings;
        let srid = s.number("newProjects.srid") as u32;
        let srid = if crs::system(srid).is_some() {
            srid
        } else {
            5256
        };
        let workspace = default_type(app);
        let drawing_font = serde_json::from_value(s.effective("newProjects.drawingFont"))
            .unwrap_or(DrawingFont::Barlow);
        Self {
            name: content::NEW_PROJECT_NAME.to_owned(),
            srid,
            plot_scale: 1000.0,
            workspace,
            drawing_font,
            query: String::new(),
            status: None,
            pending: None,
        }
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

impl App {
    pub(super) fn new_project_event(&mut self, e: Event) -> Task<Message> {
        let Some(Window::New(s)) = &mut self.project else {
            return Task::none();
        };
        match e {
            Event::Name(name) => {
                s.name = name;
                s.status = None;
            }
            Event::Scale(scale) => s.plot_scale = scale,
            Event::Mode(w) => s.workspace = w,
            Event::Crs(srid) => s.srid = srid,
            Event::Search(query) => {
                // Typing a known SRID chooses it (the web's picker).
                let digits: String = query.chars().filter(char::is_ascii_digit).collect();
                if let Ok(srid) = digits.parse::<u32>()
                    && digits.len() >= 4
                    && crs::system(srid).is_some()
                {
                    s.srid = srid;
                }
                s.query = query;
            }
            Event::Create => {
                if s.name.trim().is_empty() {
                    s.status = Some("Proje adı boş olamaz.".to_owned());
                    return Task::none();
                }
                let options = NewProject {
                    name: s.name.clone(),
                    srid: s.srid,
                    plot_scale: s.plot_scale,
                    workspace: s.workspace,
                    drawing_font: s.drawing_font,
                    province: None,
                    drawing_unit: None,
                };
                match new_document(&options) {
                    Err(e) => s.status = Some(e),
                    Ok(doc) => {
                        s.pending = Some(Box::new(doc));
                        return self.leave(Then::NewProject);
                    }
                }
            }
        }
        Task::none()
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
        self.close_project_window();
        let settings = doc.settings().clone();
        let name = doc.name().to_owned();
        // The viewport fits its home view, once the drawing area has its size.
        self.show_document(*doc);
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

    /// Vazgeç on the unsaved question: the window comes back, nothing built kept.
    pub(crate) fn new_project_declined(&mut self) {
        if let Some(Window::New(s)) = &mut self.project {
            s.pending = None;
            self.dialog = Some(Asking::Project);
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
        let name = text_input("Proje adı", &s.name)
            .on_input(|t| event(Event::Name(t)))
            .on_submit(event(Event::Create))
            .padding([5, 8])
            .size(typography::body())
            .width(260)
            .style(style::field::input);
        let name = kentos_ui::widget::focus_ring(name);
        let default_srid = self.settings.number("newProjects.srid") as u32;
        let layers = content::standard_layers(s.plot_scale)
            .iter()
            .map(|n| n.name.clone())
            .collect::<Vec<_>>()
            .join(", ");
        let mut body = Column::new()
            .spacing(16)
            .push(group(
                "Proje",
                Column::new()
                    .spacing(10)
                    .push(setting("Proje adı", Some("İlk kayıtta dosya adı olarak önerilir."), name))
                    .push(setting(
                        "Çizim ölçeği",
                        Some("Yazı yükseklikleri ve pafta çıktıları bu ölçeğe göre hesaplanır."),
                        scales(s.plot_scale, |v| event(Event::Scale(v))),
                    )),
            ))
            .push(group("Proje türü", modes(s.workspace, false, true, |w| event(Event::Mode(w)))))
            .push(group(
                "Koordinat sistemi",
                crs::picker(
                    s.srid,
                    s.srid,
                    default_srid,
                    PickFor::New,
                    &s.query,
                    |srid| event(Event::Crs(srid)),
                    |q| event(Event::Search(q)),
                ),
            ))
            .push(Banner::info(format!(
                "Boş bir çizim açılır. Katmanlar: {layers}. Birimler varsayılanla başlar (uzunluk 3, alan 2 ondalık, m², grad); Dosya → Proje ayarları’ndan değiştirilir."
            )));
        if let Some(note) = self.new_project_note() {
            body = body.push(if note.warn {
                Banner::warning(note.text)
            } else {
                Banner::info(note.text)
            });
        }
        if let Some(status) = &s.status {
            body = body.push(words::text_line(words::Kind::Error, status.clone()));
        }
        let can = !s.name.trim().is_empty();
        overlay::blocking(
            Dialog::new("Yeni proje")
                // The body scrolls; Vazgeç and Oluştur stay in view whatever the window's height.
                .scroll_fill(body)
                .action(words::secondary(
                    "Vazgeç",
                    Some(message(ProjectEvent::Close)),
                ))
                .action(words::primary("Oluştur", can.then(|| event(Event::Create))))
                .width(760.0)
                .max_height(820.0),
        )
    }
}
