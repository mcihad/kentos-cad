//! Sunucuya içe aktar and Atlası sunucuda yap (docs/adr/0561 §4): a file import and an atlas as durable jobs on the
//! KentOS server, from the windows that make them here. The import windows (GeoPackage, FlatGeobuf, GML, CSV, DXF) and the
//! atlas window offer the button when the open drawing is a cloud database project; it opens this window, which says what
//! the server will do with the choices made in the window it came from, and asks for it: after the open project's unsent
//! edits went (so the job reads them), an import's file is sent to the server (its size and SHA-256 declared, its bytes
//! in one body) and the job is asked for; an atlas job is asked for with the pin the preview showed, so a project that
//! has moved on is refused by the server as it would be here.
//!
//! Nothing runs on this computer and nothing is written to the open drawing by the ask. The job goes on in the server
//! whatever happens to the window or the app; its progress, its result (what came in, the warnings, the new revision) and
//! an atlas's PDF are in the catalog's Sunucu işleri tab, and a result committed to the project comes back as the
//! project's own change (the events the open project follows, docs/adr/0044).

use std::path::PathBuf;
use std::sync::Arc;

use iced::widget::Column;
use iced::{Element, Task};
use kentos_cloud::api::hex_sha256;
use kentos_cloud::{ApiFailure, Bytes, server_jobs};
use kentos_commands::{
    ImportFormat, JobInputBegin, ProjectAtlasInput, ProjectImportInput, ProjectPermission,
    ServerJob,
};
use kentos_interaction::Level;
use kentos_ui::widget::{Dialog, overlay};
use serde_json::{Map, Value};

use super::words::{self, Kind as Line};
use super::{Event as Exchange, Window, message};
use crate::app::{App, Message};
use crate::cloud::actions::{Settle, reason};
use crate::cloud::catalog::Tab;
use crate::processing::ServerPlace;

/// Where an import's bytes are: a file on this computer, or what the window already read.
#[derive(Clone)]
pub enum Source {
    Path(PathBuf),
    Bytes(Arc<[u8]>),
}

impl std::fmt::Debug for Source {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Source::Path(p) => write!(f, "Path({})", p.display()),
            Source::Bytes(b) => write!(f, "Bytes({} bytes)", b.len()),
        }
    }
}

/// What the window asks the server for.
#[derive(Debug, Clone)]
pub enum Work {
    /// A file taken in by the format's import command: its name, the bytes' place, the command's input without its
    /// path (what the window chose), and the choices in words.
    Import {
        format: ImportFormat,
        name: String,
        source: Source,
        options: Map<String, Value>,
        lines: Vec<String>,
    },
    /// The atlas of a sheet: the job's input (with the pin the preview showed), its name for the lines, how many pages
    /// the preview made, and the choices in words.
    Atlas {
        input: Box<ProjectAtlasInput>,
        sheet: String,
        pages: usize,
        lines: Vec<String>,
    },
}

impl Work {
    /// An import window's work: `input` is the command's input as JSON; its `path` is left out (the server gives the
    /// file it staged).
    pub fn import(
        format: ImportFormat,
        name: impl Into<String>,
        source: Source,
        input: Value,
        lines: Vec<String>,
    ) -> Work {
        let mut options = match input {
            Value::Object(m) => m,
            _ => Map::new(),
        };
        options.remove("path");
        Work::Import {
            format,
            name: name.into(),
            source,
            options,
            lines,
        }
    }

    fn title(&self) -> &'static str {
        match self {
            Work::Import { .. } => "Sunucuya içe aktar",
            Work::Atlas { .. } => "Atlası sunucuda yap",
        }
    }

    fn go(&self) -> &'static str {
        match self {
            Work::Import { .. } => "Sunucuya gönder",
            Work::Atlas { .. } => "Sunucuda yap",
        }
    }
}

#[derive(Debug)]
pub struct State {
    pub work: Work,
    /// The request's id, kept until the server answers: asking again after a lost answer gives the same job.
    pub request: String,
    /// The ask under way.
    pub asked: Option<u64>,
    pub status: Option<(bool, String)>,
    /// The job the server made.
    pub job: Option<Box<ServerJob>>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A window's button: this work, for the open project.
    Open(Box<Work>),
    /// “Sunucuya gönder” / “Sunucuda yap”.
    Send,
    Asked {
        id: u64,
        result: Box<Result<ServerJob, ApiFailure>>,
    },
    /// “Sunucu işleri”: the catalog on the open project's jobs.
    Jobs,
}

fn event(e: Event) -> Message {
    message(Exchange::ServerWork(e))
}

/// The format's name in words.
pub fn format_name(format: ImportFormat) -> &'static str {
    match format {
        ImportFormat::Gpkg => "GeoPackage",
        ImportFormat::Fgb => "FlatGeobuf",
        ImportFormat::Gml => "GML",
        ImportFormat::Csv => "CSV",
        ImportFormat::Dxf => "DXF",
        ImportFormat::Shp => "Shapefile",
        ImportFormat::Geojson => "GeoJSON",
        ImportFormat::Kml => "KML/KMZ",
        ImportFormat::Parquet => "GeoParquet",
    }
}

impl App {
    /// Where the server stands for an import (`atlas` false: it writes objects) or an atlas (a PDF of the project's
    /// data): not a cloud database project (no button), or one the account may ask in, or may not (the button off).
    pub(crate) fn server_work_place(&self, atlas: bool) -> ServerPlace {
        let place = self.server_place();
        if place != ServerPlace::Ready {
            return place;
        }
        let Some(source) = self.document.as_ref().and_then(|d| d.cloud_source()) else {
            return ServerPlace::Absent;
        };
        let (right, what) = if atlas {
            (ProjectPermission::Download, "project.download")
        } else {
            (ProjectPermission::FeatureWrite, "feature.write")
        };
        if !source.info.access.permissions.contains(&right) {
            return ServerPlace::Off(format!("yetkiniz yok ({what})"));
        }
        ServerPlace::Ready
    }

    /// The button an import or atlas window gets: none when the open drawing is no cloud database project, off with
    /// the reason in the tooltip-less line when the account may not, else it opens the window for `work`.
    pub(crate) fn server_work_button(&self, work: Work) -> Option<Element<'_, Message>> {
        let atlas = matches!(work, Work::Atlas { .. });
        let label = if atlas {
            "Atlası sunucuda yap…"
        } else {
            "Sunucuya içe aktar…"
        };
        match self.server_work_place(atlas) {
            ServerPlace::Absent => None,
            ServerPlace::Ready => Some(words::ghost(
                label,
                Some(event(Event::Open(Box::new(work)))),
            )),
            ServerPlace::Off(_) => Some(words::ghost(label, None)),
        }
    }

    pub(super) fn server_work_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Open(work) => {
                self.open_window(Window::ServerWork(State {
                    work: *work,
                    request: server_jobs::request_id(),
                    asked: None,
                    status: None,
                    job: None,
                }));
                Task::none()
            }
            Event::Send => {
                let Some(Window::ServerWork(s)) = &mut self.exchange else {
                    return Task::none();
                };
                if s.asked.is_some() || s.job.is_some() {
                    return Task::none();
                }
                let atlas = matches!(s.work, Work::Atlas { .. });
                if let ServerPlace::Off(why) = self.server_work_place(atlas) {
                    if let Some(Window::ServerWork(s)) = &mut self.exchange {
                        s.status = Some((true, format!("Bu projede sunucu işi istenemez: {why}.")));
                    }
                    return Task::none();
                }
                let Some(Window::ServerWork(s)) = &mut self.exchange else {
                    return Task::none();
                };
                s.status = Some((
                    false,
                    match &s.work {
                        Work::Import { .. } => "Dosya sunucuya gönderiliyor…".to_owned(),
                        Work::Atlas { .. } => "Sunucu işi isteniyor…".to_owned(),
                    },
                ));
                // The open project's unsent edits go first: the server reads the project as it is then.
                if self.cloud.live.is_some() {
                    self.cloud.settling = Some(Settle::ServerWork);
                    return self.flush();
                }
                self.server_work_send()
            }
            Event::Asked { id, result } => {
                let Some(Window::ServerWork(s)) = &mut self.exchange else {
                    return Task::none();
                };
                if s.asked != Some(id) {
                    return Task::none();
                }
                s.asked = None;
                match *result {
                    Ok(job) => {
                        let what = server_jobs::kind_label(&job);
                        s.status = Some((
                            false,
                            format!(
                                "Sunucu işi istendi: {what}. Sunucuda çalışır; pencereyi kapatsanız da sürer."
                            ),
                        ));
                        s.job = Some(Box::new(job));
                        self.say(Level::Info, format!("Sunucu işi istendi: {what}."));
                    }
                    Err(failure) => {
                        // The request id stays: asking again gives the same job if the server did make it.
                        s.status =
                            Some((true, format!("Sunucu işi istenemedi: {}", reason(&failure))));
                    }
                }
                Task::none()
            }
            Event::Jobs => {
                let Some(project) = self
                    .document
                    .as_ref()
                    .and_then(|d| d.cloud_source())
                    .map(|s| s.info.id.clone())
                else {
                    return Task::none();
                };
                self.close_exchange();
                self.dialog = None;
                self.open_catalog_at(Some((project, Tab::Jobs)))
            }
        }
    }

    /// Asks the server for the job (after the open project's unsent edits went): an import sends its file first.
    pub(crate) fn server_work_send(&mut self) -> Task<Message> {
        let (Some(client), Some(source)) = (
            self.cloud.signed_in().cloned(),
            self.document.as_ref().and_then(|d| d.cloud_source()),
        ) else {
            return Task::none();
        };
        let (Some(tenant), Some(project)) = (
            crate::cloud::uuid(&source.info.tenant_id),
            crate::cloud::uuid(&source.info.id),
        ) else {
            return Task::none();
        };
        let id = self.cloud.next_id();
        let Some(Window::ServerWork(s)) = &mut self.exchange else {
            return Task::none();
        };
        s.asked = Some(id);
        let (work, request) = (s.work.clone(), s.request.clone());
        Task::perform(
            async move {
                match work {
                    Work::Import {
                        format,
                        name,
                        source,
                        options,
                        ..
                    } => {
                        let local = |why: String| ApiFailure::new(0, "local", why);
                        let bytes = match source {
                            Source::Path(path) => std::fs::read(&path)
                                .map_err(|e| local(format!("{} okunamadı: {e}", path.display())))?,
                            Source::Bytes(bytes) => bytes.to_vec(),
                        };
                        let begin = JobInputBegin {
                            name: name.clone(),
                            size: bytes.len() as u64,
                            sha256: hex_sha256(&bytes),
                        };
                        let staged = client.begin_job_input(tenant, project, begin).await?;
                        let input_id = crate::cloud::uuid(&staged.id)
                            .ok_or_else(|| local("Sunucunun dosya kimliği okunamadı.".into()))?;
                        client
                            .send_job_input(tenant, project, input_id, Bytes::from(bytes))
                            .await?;
                        let input = ProjectImportInput {
                            upload: staged.id,
                            format: Some(format),
                            options: options.into_iter().collect(),
                            label: Some(name),
                            ..ProjectImportInput::default()
                        };
                        client
                            .create_server_job(
                                tenant,
                                project,
                                server_jobs::import_request(&request, &input),
                            )
                            .await
                    }
                    Work::Atlas { input, .. } => {
                        client
                            .create_server_job(
                                tenant,
                                project,
                                server_jobs::atlas_request(&request, &input),
                            )
                            .await
                    }
                }
            },
            move |result| {
                event(Event::Asked {
                    id,
                    result: Box::new(result),
                })
            },
        )
    }

    pub(super) fn server_work_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let atlas = matches!(s.work, Work::Atlas { .. });
        let place = self.server_work_place(atlas);
        let project = self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source())
            .map(|c| c.info.name.clone())
            .unwrap_or_default();
        let mut body = Column::new().spacing(12);
        let mut lines: Vec<Element<'a, Message>> = Vec::new();
        match &s.work {
            Work::Import {
                format,
                name,
                source,
                lines: chosen,
                ..
            } => {
                let size = match source {
                    Source::Path(p) => std::fs::metadata(p).map_or(0, |m| m.len()),
                    Source::Bytes(b) => b.len() as u64,
                };
                body = body.push(words::file_line(
                    name,
                    format!(
                        "{}, {}",
                        format_name(*format),
                        server_jobs::size_text(size as f64)
                    ),
                ));
                lines.push(words::text_line(
                    Line::Info,
                    format!(
                        "Dosya “{project}” projesinin sunucusuna gönderilir ve içe aktarma orada yapılır; bu bilgisayarda hiçbir şey çalışmaz."
                    ),
                ));
                lines.push(words::text_line(
                    Line::Info,
                    "Sonuç, projenin işin okuduğu revizyonuna tek bir değişiklik olarak yazılır; proje o arada değişirse hiçbir şey yazılmaz ve işi yeniden istersiniz.",
                ));
                for l in chosen {
                    lines.push(words::text_line(Line::Ok, l.clone()));
                }
                if matches!(format, ImportFormat::Csv | ImportFormat::Gml) {
                    lines.push(words::text_line(
                        Line::Warn,
                        "Yan dosyalar (.csvt, .prj, .xsd, .gfs) sunucuya gönderilmez: sütun türleri değerlerden, sistem pencerede ya da seçeneklerde söylenenden belirlenir.",
                    ));
                }
            }
            Work::Atlas {
                sheet,
                pages,
                lines: chosen,
                ..
            } => {
                body = body.push(words::file_line(
                    sheet,
                    format!("{pages} sayfalık atlas, “{project}”"),
                ));
                lines.push(words::text_line(
                    Line::Info,
                    "Atlasın sayfaları sunucuda çizilir; PDF Sunucu işleri sekmesinden indirilir. Bu bilgisayarda hiçbir şey çizilmez, projede hiçbir şey değişmez.",
                ));
                lines.push(words::text_line(
                    Line::Info,
                    "Önizlemenin sabitlediği duruma bağlıdır: proje ya da bağlı bir dosya o arada değişirse sunucu işi reddeder (pin_mismatch) ve önizlemeyi yenileyip yeniden istersiniz.",
                ));
                for l in chosen {
                    lines.push(words::text_line(Line::Ok, l.clone()));
                }
            }
        }
        lines.push(words::text_line(
            Line::Info,
            "İş, pencereyi ya da uygulamayı kapatsanız da sürer; ilerlemesi ve sonucu Sunucu işleri sekmesinde.",
        ));
        if let ServerPlace::Off(why) = &place {
            lines.push(words::text_line(
                Line::Warn,
                format!("Bu projede sunucu işi istenemez: {why}."),
            ));
        }
        body = body.push(words::summary(lines));
        if let Some((error, text)) = &s.status {
            body = body.push(words::text_line(
                if *error { Line::Error } else { Line::Info },
                text.clone(),
            ));
        }
        let ready = place == ServerPlace::Ready && s.asked.is_none() && s.job.is_none();
        let over = s.job.is_some();
        overlay::blocking(
            Dialog::new(s.work.title())
                .scroll(body)
                .action(words::ghost("Sunucu işleri", Some(event(Event::Jobs))))
                .action(words::secondary(
                    if over { "Kapat" } else { "Vazgeç" },
                    Some(message(Exchange::Close)),
                ))
                .action(words::primary(
                    s.work.go(),
                    ready.then(|| event(Event::Send)),
                ))
                .width(660.0)
                .max_height(720.0),
        )
    }
}
