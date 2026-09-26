//! Dosya → Yeni proje (the web's `ui/settings/NewProjectDialog.ts`): an
//! empty drawing with the standard layer tree, a work mode, a coordinate
//! system (the app's default for new projects first) and a plot scale.
//! Nothing changes until Oluştur. Unsaved changes of the drawing on screen
//! are asked about then, over this window, so Vazgeç there comes back here;
//! an open cloud project is left first (its changes are sent or kept on
//! this device).

use std::fmt;

use iced::widget::{Column, container, scrollable, text_input};
use iced::{Element, Length, Task};
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
        let workspace = serde_json::from_value(s.effective("newProjects.workspace"))
            .ok()
            .filter(|w| ready(*w))
            .unwrap_or(Workspace::Hybrid);
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

    /// The drawing on screen is left: the new project goes on screen, showing
    /// one sheet around its anchor (the web's `newProject`).
    pub(crate) fn new_project_ready(&mut self) -> Task<Message> {
        let Some(Window::New(s)) = &mut self.project else {
            return Task::none();
        };
        let Some(doc) = s.pending.take() else {
            return Task::none();
        };
        self.close_project_window();
        let settings = doc.settings().clone();
        let origin = doc.model.origin();
        let name = doc.name().to_owned();
        self.show_document(*doc);
        let system = crs::system(settings.srid);
        let degrees = system.is_some_and(|s| s.unit == "degree");
        let b = content::sheet_around(origin, settings.plot_scale, degrees);
        self.viewport.show(&kentos_render_wgpu::Bounds {
            min_x: b.min_x,
            min_y: b.min_y,
            max_x: b.max_x,
            max_y: b.max_y,
        });
        let system = system.map_or_else(
            || format!("EPSG:{}", settings.srid),
            |s| format!("{} (EPSG:{})", s.name, s.srid),
        );
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

    pub(super) fn new_project_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let name = text_input("Proje adı", &s.name)
            .on_input(|t| event(Event::Name(t)))
            .on_submit(event(Event::Create))
            .padding([5, 8])
            .size(typography::body())
            .width(260)
            .style(style::field::input);
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
            .push(group("Çalışma modu", modes(s.workspace, false, |w| event(Event::Mode(w)))))
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
        if let Some(doc) = &self.document {
            if doc.cloud_source().is_some() && doc.is_database() {
                body = body.push(Banner::info(format!(
                    "“{}” bulut projesi kapanır. Bekleyen değişiklikleri buluta gönderilir; gönderilemeyenler bu cihazda kalır ve proje yeniden açılınca geri gelir.",
                    doc.name()
                )));
            } else if doc.dirty() {
                body = body.push(Banner::warning(format!(
                    "“{}” içinde kaydedilmemiş değişiklikler var; Oluştur’a basınca önce sorulur.",
                    doc.name()
                )));
            }
        }
        if let Some(status) = &s.status {
            body = body.push(words::text_line(words::Kind::Error, status.clone()));
        }
        let can = !s.name.trim().is_empty();
        overlay::blocking(
            Dialog::new("Yeni proje")
                .push(container(scrollable(body).height(Length::Shrink)).max_height(640))
                .action(words::secondary(
                    "Vazgeç",
                    Some(message(ProjectEvent::Close)),
                ))
                .action(words::primary("Oluştur", can.then(|| event(Event::Create))))
                .width(760.0),
        )
    }
}
