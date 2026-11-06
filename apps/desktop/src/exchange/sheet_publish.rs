//! Sheet publication is a normal cancellable protocol job; the window only gathers options and shows its report.
use super::{Event as Exchange, Window, message};
use crate::app::{App, Message};
use iced::futures::{SinkExt, Stream, channel::mpsc};
use iced::widget::{button, column, progress_bar, row, scrollable};
use iced::{Element, Fill, Task};
use kentos_commands::{
    CallEnvelope, CallOp, CommandResult, FILE_PUBLISH_SHEETS, FILE_PUBLISH_STATUS, PublishSheets,
    PublishStatus, PublishStatusReport, SheetPublishMode, SheetPublishState, SheetsPublished,
};
use kentos_native_application::jobs::Job;
use kentos_ui::{
    label, style,
    widget::{Dialog, Switch},
};
use std::{path::PathBuf, time::Duration};
#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    pub name: String,
    pub selected: bool,
}
pub struct State {
    pub id: u64,
    pub rows: Vec<Row>,
    pub input: PublishSheets,
    pub check_only: bool,
    pub report: Option<SheetsPublished>,
    job: Option<Job>,
    captured: Option<kentos_domain::Document>,
    generation: u64,
    op: CallOp,
    pub progress: f32,
    pub status: String,
    close_after: bool,
    /// The delivery folder's last publication against the drawing now, or its stopped one (docs/adr/1090 §6).
    pub folder: Option<PublishStatusReport>,
    /// The job asks the folder's state, not for a publication.
    asking: bool,
}
impl std::fmt::Debug for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("SheetPublish")
            .field("input", &self.input)
            .field("status", &self.status)
            .finish_non_exhaustive()
    }
}
impl State {
    fn changed(&mut self) {
        self.report = None;
        self.status.clear();
    }
    pub fn running(&self) -> bool {
        self.job.is_some()
    }
}
#[derive(Clone, Debug)]
pub enum Event {
    Selected(usize, bool),
    All(bool),
    Move(usize, i32),
    Separate(bool),
    Resources(bool),
    Naming(String),
    Name(String),
    Path(String),
    Pick,
    Picked(u64, Option<PathBuf>),
    Plan,
    Publish,
    /// The delivery folder's state: stale sheets of its publication, a stopped one (docs/adr/1090 §6).
    Status,
    /// Go on with the folder's stopped publication (docs/adr/1090 §7).
    Resume,
    /// Update the folder's publication into a new folder: unchanged pages carried, changed ones made again
    /// (docs/adr/1320 §3).
    Update,
    /// Publish afresh, not as an update.
    Unlink,
    Stop,
    Tick,
    Close,
}
fn event(e: Event) -> Message {
    message(Exchange::SheetPublish(e))
}
pub fn ticks() -> impl Stream<Item = Message> {
    let (mut out, stream) = mpsc::channel(1);
    std::thread::spawn(move || {
        loop {
            std::thread::sleep(Duration::from_millis(100));
            if iced::futures::executor::block_on(out.send(event(Event::Tick))).is_err() {
                break;
            }
        }
    });
    stream
}
impl App {
    pub(crate) fn sheet_publish_running(&self) -> bool {
        matches!(&self.exchange,Some(Window::SheetPublish(s)) if s.running())
    }
    /// Called by generic Escape/close too; a running writer must hear cancellation before disposal.
    pub(crate) fn sheet_publish_stop_for_close(&mut self) -> bool {
        if let Some(Window::SheetPublish(s)) = &mut self.exchange
            && let Some(job) = &mut s.job
        {
            job.cancel();
            s.close_after = true;
            s.status = "İptal ediliyor; geçici dosyalar kaldırılacak…".into();
            return true;
        }
        false
    }
    pub(crate) fn sheet_publish_open(&mut self) -> Task<Message> {
        if self.sheet_publish_running() {
            return Task::none();
        }
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(kept) = doc.model.sheets() else {
            self.warn("Yayınlanacak pafta yok; önce bir pafta oluşturun.");
            return Task::none();
        };
        let Ok((book, _)) = kentos_sheet::project::read_project_sheets(kept) else {
            self.warn("Pafta kitabı okunamadı; ön denetimi açın.");
            return Task::none();
        };
        if book.sheets.is_empty() {
            self.warn("Yayınlanacak pafta yok; önce bir pafta oluşturun.");
            return Task::none();
        }
        let selected = self.sheets.pdf_sheets();
        let options = self.sheets.output_pdf_options();
        let rows = book
            .sheets
            .iter()
            .map(|s| Row {
                id: s.id.clone(),
                name: s.name.clone(),
                selected: selected.contains(&s.id),
            })
            .collect();
        let dir = doc
            .path
            .as_ref()
            .and_then(|p| p.parent())
            .map(PathBuf::from)
            .or_else(|| std::env::current_dir().ok())
            .unwrap_or_else(std::env::temp_dir);
        let input = PublishSheets {
            path: dir
                .join(format!("Paftalar-{}", kentos_plot::book::today()))
                .display()
                .to_string(),
            drawing: doc.path.as_ref().map(|p| p.display().to_string()),
            sheets: None,
            mode: SheetPublishMode::Single,
            resources: false,
            name: Some("paftalar".into()),
            naming: "{index}-{sheet}".into(),
            dpi: None,
            geo: options.geo,
            layers: options.layers,
            pin: None,
            resumable: true,
            resume: false,
            previous: None,
        };
        static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        let id = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.close_exchange();
        self.exchange = Some(Window::SheetPublish(Box::new(State {
            id,
            rows,
            input,
            check_only: false,
            report: None,
            job: None,
            captured: None,
            generation: 0,
            op: CallOp::Plan,
            progress: 0.,
            status: String::new(),
            close_after: false,
            folder: None,
            asking: false,
        })));
        self.dialog = Some(crate::app::Dialog::Exchange);
        Task::none()
    }
    pub(crate) fn sheet_preflight_open(&mut self) -> Task<Message> {
        let _ = self.sheet_publish_open();
        if let Some(Window::SheetPublish(s)) = &mut self.exchange {
            if s.running() {
                return Task::none();
            }
            s.check_only = true;
        }
        self.sheet_publish_event(Event::Plan)
    }
    pub(crate) fn sheet_publish_event(&mut self, e: Event) -> Task<Message> {
        if matches!(e, Event::Close) {
            self.close_exchange();
            return Task::none();
        }
        let Some(Window::SheetPublish(s)) = &mut self.exchange else {
            return Task::none();
        };
        if s.running() && !matches!(e, Event::Stop | Event::Tick) {
            return Task::none();
        }
        match e {
            Event::Selected(i, on) => {
                if let Some(r) = s.rows.get_mut(i) {
                    r.selected = on;
                }
                s.changed();
            }
            Event::All(on) => {
                for r in &mut s.rows {
                    r.selected = on;
                }
                s.changed();
            }
            Event::Move(i, delta) => {
                let j = i as i64 + i64::from(delta);
                if i < s.rows.len() && j >= 0 && (j as usize) < s.rows.len() {
                    s.rows.swap(i, j as usize);
                }
                s.changed();
            }
            Event::Resources(on) => {
                s.input.resources = on;
                s.changed();
            }
            Event::Separate(on) => {
                s.input.mode = if on {
                    SheetPublishMode::Separate
                } else {
                    SheetPublishMode::Single
                };
                if !on {
                    s.input.previous = None;
                }
                s.changed();
            }
            Event::Update => {
                let Some(f) = s.folder.as_ref().filter(|f| {
                    f.state == kentos_commands::PublishFolderState::Published && !f.stale.is_empty()
                }) else {
                    return Task::none();
                };
                let published: Vec<String> = f
                    .sheets
                    .iter()
                    .filter(|x| !matches!(x.state, SheetPublishState::Unpublished | SheetPublishState::Removed))
                    .map(|x| x.sheet.clone())
                    .collect();
                for r in &mut s.rows {
                    r.selected = published.contains(&r.id);
                }
                // The update goes into a new folder beside the delivery, which stays as it was issued.
                let was = f.path.clone();
                let base = format!("{}-{}", was.trim_end_matches(['/', '\\']), kentos_plot::book::today());
                let mut next = base.clone();
                let mut n = 2;
                while std::path::Path::new(&next).exists() {
                    next = format!("{base}-{n}");
                    n += 1;
                }
                s.input.previous = Some(was);
                s.input.path = next;
                s.input.mode = SheetPublishMode::Separate;
                s.input.resumable = false;
                s.folder = None;
                s.changed();
                return Task::done(event(Event::Plan));
            }
            Event::Unlink => {
                s.input.previous = None;
                s.input.resumable = true;
                s.changed();
            }
            Event::Naming(v) => {
                s.input.naming = v;
                s.changed();
            }
            Event::Name(v) => {
                s.input.name = Some(v);
                s.changed();
            }
            Event::Path(v) => {
                s.input.path = v;
                s.changed();
            }
            Event::Pick => {
                let id = s.id;
                if let crate::app::Picker::File(path) = &self.picker {
                    return Task::done(event(Event::Picked(id, Some(path.clone()))));
                }
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Boş teslim klasörünü seçin")
                            .pick_folder()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    move |p| event(Event::Picked(id, p)),
                );
            }
            Event::Picked(id, Some(path)) if id == s.id => {
                s.input.path = path.display().to_string();
                s.changed();
            }
            Event::Picked(..) => {}
            Event::Plan | Event::Publish | Event::Status | Event::Resume => {
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let mut input = s.input.clone();
                s.asking = matches!(e, Event::Status);
                if s.asking {
                    // The folder's state is its own answer: a plan's report would read as if it were about it.
                    s.report = None;
                }
                input.sheets = Some(
                    s.rows
                        .iter()
                        .filter(|r| r.selected)
                        .map(|r| r.id.clone())
                        .collect(),
                );
                let op = if matches!(e, Event::Plan | Event::Status) {
                    CallOp::Plan
                } else {
                    CallOp::Execute
                };
                if s.check_only && matches!(op, CallOp::Execute) {
                    return Task::none();
                }
                if matches!(e, Event::Resume) {
                    if !s.folder.as_ref().and_then(|f| f.stopped.as_ref()).is_some_and(|x| x.resumable) {
                        return Task::none();
                    }
                    input.mode = SheetPublishMode::Separate;
                    input.resume = true;
                    input.previous = None;
                    input.sheets = s.folder.as_ref().and_then(|f| f.stopped.as_ref()).map(|x| x.sheets.clone());
                } else if matches!(op, CallOp::Execute) {
                    let Some(report) = &s.report else {
                        return Task::none();
                    };
                    if !report.ready || doc.model.generation() != s.generation {
                        return Task::none();
                    }
                    input.pin = Some(report.source.clone());
                }
                let command = if s.asking {
                    FILE_PUBLISH_STATUS
                } else if s.check_only {
                    kentos_commands::FILE_PREFLIGHT_SHEETS
                } else {
                    FILE_PUBLISH_SHEETS
                };
                let encoded = if s.asking {
                    serde_json::to_value(PublishStatus { path: input.path.clone(), drawing: input.drawing.clone() })
                } else if s.check_only {
                    serde_json::to_value(kentos_commands::PreflightSheets {
                        drawing: input.drawing.clone(),
                        sheets: input.sheets.clone(),
                        dpi: input.dpi,
                    })
                } else {
                    serde_json::to_value(&input)
                };
                let value = match encoded {
                    Ok(v) => v,
                    Err(e) => {
                        s.status = e.to_string();
                        return Task::none();
                    }
                };
                let envelope = CallEnvelope {
                    command: command.into(),
                    version: Some(1),
                    op,
                    request_id: None,
                    expected: None,
                    relax_constraints: false,
                    input: value,
                };
                s.captured = Some(doc.model.reading_copy());
                s.generation = doc.model.generation();
                s.op = op;
                s.progress = 0.;
                s.status = if s.asking {
                    "Teslim klasörünün yayını çizimle karşılaştırılıyor…"
                } else if matches!(op, CallOp::Plan) {
                    "Paftalar denetleniyor…"
                } else {
                    "Paftalar yayınlanıyor…"
                }
                .into();
                s.job = Some(Job::start_with(
                    "sheet-publish".into(),
                    command.into(),
                    None,
                    &doc.model,
                    kentos_headless::file_commands::job_work(envelope, Vec::new()),
                ));
            }
            Event::Stop => {
                if let Some(job) = &mut s.job {
                    job.cancel();
                    s.status = "İptal isteniyor; teslim klasörü kesinleşmeden durdurulacak…".into();
                }
            }
            Event::Tick => {
                let Some(job) = &mut s.job else {
                    return Task::none();
                };
                let status = if let Some(doc) = &mut self.document
                    && doc.model.generation() == s.generation
                {
                    job.status(&mut doc.model)
                } else if let Some(captured) = &mut s.captured {
                    job.status(captured)
                } else {
                    return Task::none();
                };
                s.progress = status.progress.unwrap_or(0.) as f32;
                if let Some(answer) = status.answer {
                    s.job = None;
                    s.captured = None;
                    match answer.result {
                        CommandResult::Completed { output, .. } if s.asking => {
                            match serde_json::from_value::<PublishStatusReport>(output) {
                                Ok(f) => {
                                    s.status = folder_text(&f);
                                    s.folder = Some(f);
                                }
                                Err(e) => s.status = format!("Yayın durumu okunamadı: {e}"),
                            }
                        }
                        CommandResult::Completed { output, .. } => {
                            let parsed = if s.check_only {
                                serde_json::from_value::<kentos_commands::SheetsPreflighted>(output)
                                    .map(|r| SheetsPublished {
                                        resources: None,
                                        path: String::new(),
                                        sheets: r.sheets,
                                        files: vec![],
                                        issues: r.issues,
                                        source: r.source,
                                        ready: r.ready,
                                        manifest: String::new(),
                                        sha256: String::new(),
                                        scope: Vec::new(),
                                        pages: Vec::new(),
                                        resumed: Vec::new(),
                                        previous: None,
                                        carried: Vec::new(),
                                    })
                            } else {
                                serde_json::from_value::<SheetsPublished>(output)
                            };
                            match parsed {
                                Ok(report) => {
                                    s.status = if s.check_only {
                                        format!(
                                            "{} pafta denetlendi · {} hata, {} uyarı/not.",
                                            report.sheets.len(),
                                            report.issues.iter().filter(|i| i.error).count(),
                                            report.issues.iter().filter(|i| !i.error).count()
                                        )
                                    } else if matches!(s.op, CallOp::Execute) && report.previous.is_some() {
                                        format!(
                                            "Teslim güncellendi: {} pafta yeniden üretildi, {} pafta önceki teslimden olduğu gibi kopyalandı: {}",
                                            report.sheets.len() - report.carried.len(),
                                            report.carried.len(),
                                            report.path
                                        )
                                    } else if matches!(s.op, CallOp::Execute) {
                                        format!(
                                            "{} pafta, {} PDF yayınlandı: {}",
                                            report.sheets.len(),
                                            report.files.len(),
                                            report.path
                                        )
                                    } else if report.ready && report.previous.is_some() {
                                        format!(
                                            "{} pafta yeniden üretilecek, {} pafta önceki teslimden olduğu gibi kopyalanacak.",
                                            report.sheets.len() - report.carried.len(),
                                            report.carried.len()
                                        )
                                    } else if report.ready {
                                        format!(
                                            "{} pafta hazır; {} PDF dosyası yazılacak.",
                                            report.sheets.len(),
                                            report.files.len()
                                        )
                                    } else {
                                        "Yayın engellendi; aşağıdaki kaynak ve ön denetim sorunlarını düzeltin.".into()
                                    };
                                    if self
                                        .document
                                        .as_ref()
                                        .is_none_or(|d| d.model.generation() != s.generation)
                                    {
                                        s.status = format!(
                                            "Çizim bu sırada değişti. Bu sonuç yakalanan {} revizyonuna aittir. Yeni çizim için yeniden denetleyin.",
                                            report.source.revision
                                        );
                                    }
                                    s.report = Some(report);
                                }
                                Err(e) => s.status = format!("Yayın yanıtı okunamadı: {e}"),
                            }
                        }
                        CommandResult::Failed { error } | CommandResult::Conflict { error } => {
                            s.status = error.message;
                            s.report = None;
                        }
                        CommandResult::Cancelled => {
                            s.status = if s.input.mode == SheetPublishMode::Separate && s.input.resumable {
                                "Yayın durduruldu; teslim klasörü oluşturulmadı. Bitmiş sayfalar devam için saklandı: Durum ile görün, Devam et ile sürdürün.".into()
                            } else {
                                "Yayın iptal edildi; kısmi teslim oluşturulmadı.".into()
                            }
                        }
                        _ => {
                            s.status =
                                "Yayın beklenen bir sonuç vermedi; yeniden denetleyin.".into()
                        }
                    }
                    let text = s.status.clone();
                    let close = s.close_after;
                    self.output(text);
                    if close {
                        self.close_exchange();
                    }
                }
            }
            Event::Close => {}
        }
        Task::none()
    }
    pub(crate) fn sheet_publish_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let busy = s.running();
        let mut list = column![].spacing(4);
        for (i, r) in s.rows.iter().enumerate() {
            let check: Element<'_, Message> = if busy {
                Switch::disabled(r.selected).label(r.name.clone()).into()
            } else {
                Switch::new(r.selected, move |on| event(Event::Selected(i, on)))
                    .label(r.name.clone())
                    .into()
            };
            list = list.push(
                row![
                    check,
                    iced::widget::space().width(Fill),
                    button(label::caption("Yukarı"))
                        .padding(kentos_ui::theme::metrics::action())
                        .on_press_maybe((!busy && i > 0).then_some(event(Event::Move(i, -1))))
                        .style(style::button::secondary),
                    button(label::caption("Aşağı"))
                        .padding(kentos_ui::theme::metrics::action())
                        .on_press_maybe(
                            (!busy && i + 1 < s.rows.len()).then_some(event(Event::Move(i, 1)))
                        )
                        .style(style::button::secondary)
                ]
                .spacing(6)
                .align_y(iced::Center),
            );
        }
        let mut body = column![
            label::body(if s.check_only {
                "Seçili paftalar ve yerel kaynakları dosya yazmadan denetlenir."
            } else {
                "Seçili paftalar aşağıdaki sırayla yayınlanır."
            }),
            row![
                button(label::caption("Hepsini seç"))
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press_maybe((!busy).then_some(event(Event::All(true))))
                    .style(style::button::secondary),
                button(label::caption("Seçimi kaldır"))
                    .padding(kentos_ui::theme::metrics::action())
                    .on_press_maybe((!busy).then_some(event(Event::All(false))))
                    .style(style::button::secondary)
            ]
            .spacing(6),
            scrollable(iced::widget::container(list).padding(iced::Padding::new(0.).right(16.)))
                .height(150)
        ]
        .spacing(10);
        if !s.check_only {
            body = body.push(if busy {
                Element::from(
                    Switch::<Message>::disabled(s.input.mode == SheetPublishMode::Separate)
                        .label("Her pafta ayrı PDF"),
                )
            } else {
                Switch::new(s.input.mode == SheetPublishMode::Separate, |v| {
                    event(Event::Separate(v))
                })
                .label("Her pafta ayrı PDF")
                .into()
            });
            body = body.push(if busy {
                Element::from(
                    Switch::<Message>::disabled(s.input.resources)
                        .label("Çizimi ve kaynaklarını ekle"),
                )
            } else {
                Element::from(
                    Switch::new(s.input.resources, |v| event(Event::Resources(v)))
                        .label("Çizimi ve kaynaklarını ekle"),
                )
            });
            if s.input.mode == SheetPublishMode::Separate {
                body = body.push(label::caption(
                    "Dosya adı · {index}, {sheet}, {number}, {revision}",
                ));
                let mut field = kentos_ui::widget::text_field("{index}-{sheet}", &s.input.naming);
                if !busy {
                    field = field.on_input(|v| event(Event::Naming(v)));
                }
                body = body.push(field);
            } else {
                body = body.push(label::caption("Tek PDF'in dosya adı"));
                let mut field = kentos_ui::widget::text_field(
                    "paftalar",
                    s.input.name.as_deref().unwrap_or(""),
                );
                if !busy {
                    field = field.on_input(|v| event(Event::Name(v)));
                }
                body = body.push(field);
            }
            let mut path = kentos_ui::widget::text_field(
                "Yeni veya boş teslim klasörünün tam yolu",
                &s.input.path,
            );
            if !busy {
                path = path.on_input(|v| event(Event::Path(v)));
            }
            body = body
                .push(label::caption(if s.input.previous.is_some() {
                    "Güncellenmiş teslimin yeni klasörü · önceki teslim olduğu gibi kalır"
                } else {
                    "Teslim klasörü · dolu bir klasörün üzerine yazılmaz"
                }))
                .push(
                    row![
                        path,
                        button(label::body("Seç…"))
                            .padding(kentos_ui::theme::metrics::action())
                            .on_press_maybe((!busy).then_some(event(Event::Pick)))
                            .style(style::button::secondary)
                    ]
                    .spacing(6),
                );
        }
        if let Some(was) = s.input.previous.as_ref().filter(|_| !s.check_only) {
            body = body.push(
                row![
                    label::caption(format!(
                        "Önceki teslim: {was} · değişmeyen paftalar baytı baytına ve kendi tarihiyle kopyalanır, değişenler bugünün tarihiyle yeniden üretilir."
                    ))
                    .width(Fill),
                    button(label::caption("Güncellemeyi kaldır"))
                        .padding(kentos_ui::theme::metrics::action())
                        .on_press_maybe((!busy).then_some(event(Event::Unlink)))
                        .style(style::button::secondary)
                ]
                .spacing(6)
                .align_y(iced::Center),
            );
        }
        if busy {
            body = body.push(progress_bar(0.0..=1.0, s.progress));
        }
        if !s.status.is_empty() {
            let stale = s.report.is_some()
                && self
                    .document
                    .as_ref()
                    .is_none_or(|d| d.model.generation() != s.generation);
            body = body.push(if stale {
                label::body(&s.status).style(style::text::warning)
            } else {
                label::body(&s.status)
            });
        }
        if let Some(f) = &s.folder {
            let mut rows = column![label::caption(format!(
                "Çizim şimdi: revizyon {} · yayının kaynağı: {}",
                f.current.revision,
                f.published.as_ref().map_or("—".to_owned(), |p| format!("revizyon {}", p.revision))
            ))]
            .spacing(4);
            for x in &f.sheets {
                let (word, warn) = match x.state {
                    SheetPublishState::Current => ("güncel", false),
                    SheetPublishState::Stale => ("güncelliğini yitirdi", true),
                    SheetPublishState::Unknown => ("karşılaştırılamaz (özetsiz eski yayın)", true),
                    SheetPublishState::Removed => ("kitapta yok", true),
                    SheetPublishState::Unpublished => ("yayında yok", false),
                    SheetPublishState::FileChanged => ("teslim edilen PDF değişmiş", true),
                    SheetPublishState::Failed => ("şimdi üretilemiyor", true),
                };
                let text = format!(
                    "{} · {word}{}{}",
                    x.name,
                    x.date.as_ref().filter(|_| x.state != SheetPublishState::Unpublished).map_or(String::new(), |d| format!(" · {d} tarihli")),
                    x.changes
                        .as_ref()
                        .filter(|_| matches!(x.state, SheetPublishState::Stale | SheetPublishState::FileChanged))
                        .map_or(String::new(), changes_text)
                );
                rows = rows.push(if warn { label::body(text).style(style::text::warning) } else { label::caption(text) });
            }
            if let Some(st) = &f.stopped {
                rows = rows.push(
                    label::body(format!(
                        "Durdurulan yayın ({}): {} sayfa bitti, {} sayfa kaldı{}",
                        match st.reason {
                            kentos_commands::PublishStopReason::Cancelled => "iptal",
                            kentos_commands::PublishStopReason::Failed => "hata",
                        },
                        st.done.len(),
                        st.remaining.len(),
                        if st.resumable { "; Devam et ile sürdürülebilir." } else { "; çizim değişti, baştan yayınlayın." }
                    ))
                    .style(style::text::warning),
                );
            }
            body = body.push(rows);
        }
        if let Some(report) = &s.report {
            let mut results = column![label::caption(format!(
                "Kaynak revizyonu {} · sonraki değişiklikler bu sonuca dahil değildir.",
                report.source.revision
            ))]
            .spacing(4);
            if let Some(resources) = &report.resources {
                results = results.push(label::caption(format!(
                    "Kaynak paketi: {} · {} dosya",
                    resources.project,
                    resources.files.len()
                )));
            }
            for sc in &report.scope {
                let frames = sc
                    .frames
                    .iter()
                    .map(|f| {
                        format!(
                            "{} {}{}{}",
                            f.name,
                            f.denominator.map_or_else(|| "atlas".into(), scale_text),
                            if f.locked { " · kilitli" } else { " · kilitsiz" },
                            if f.layers == "drawing" { " · çizimin katmanları" } else { "" }
                        )
                    })
                    .collect::<Vec<_>>()
                    .join("; ");
                results = results.push(label::caption(format!(
                    "{} · {:.0}×{:.0} mm{} · {}",
                    if sc.number.is_empty() { sc.name.as_str() } else { sc.number.as_str() },
                    sc.paper[0],
                    sc.paper[1],
                    sc.profile.as_ref().map_or(String::new(), |p| format!(" · {p}")),
                    frames
                )));
            }
            if let Some(p) = &report.previous {
                results = results.push(label::caption(format!(
                    "Önceki teslim {} ({}) · kopyalanan: {}",
                    p.path,
                    p.date,
                    if report.carried.is_empty() {
                        "yok".to_owned()
                    } else {
                        report
                            .carried
                            .iter()
                            .map(|id| s.rows.iter().find(|r| &r.id == id).map_or(id.as_str(), |r| r.name.as_str()))
                            .collect::<Vec<_>>()
                            .join(", ")
                    }
                )));
            }
            for pg in &report.pages {
                if report.previous.is_some() {
                    results = results.push(label::caption(format!(
                        "{} · {}",
                        s.rows.iter().find(|r| r.id == pg.sheet).map_or(pg.sheet.as_str(), |r| r.name.as_str()),
                        if report.carried.contains(&pg.sheet) {
                            format!("önceki teslimden kopyalandı ({} tarihli)", pg.date.as_deref().unwrap_or("tarihsiz"))
                        } else {
                            "yeniden üretildi".to_owned()
                        }
                    )));
                }
                results = results.push(label::caption(format!(
                    "Sayfa {} · {} çizgi, {} dolgu, {} yazı ({}), {} kırpma{}{}",
                    s.rows.iter().find(|r| r.id == pg.sheet).map_or(pg.sheet.as_str(), |r| r.name.as_str()),
                    pg.strokes,
                    pg.fills,
                    pg.texts,
                    if pg.fonts.is_empty() { "yazı yüzü yok".to_owned() } else { pg.fonts.join(", ") },
                    pg.clips,
                    if pg.transparent > 0 { format!(", {} saydam", pg.transparent) } else { String::new() },
                    if pg.raster_maps > 0 { format!(", {} resim görünüm", pg.raster_maps) } else { String::new() }
                )));
                for f in &pg.findings {
                    results = results.push(label::caption(format!("Uyarı/not · {} {}", f.message, f.fix)).style(style::text::warning));
                }
            }
            for f in &report.files {
                results = results.push(label::caption(&f.name));
            }
            for i in report
                .issues
                .iter()
                .filter(|i| i.error)
                .chain(report.issues.iter().filter(|i| !i.error))
            {
                let message = format!(
                    "{} · {} · {} {}",
                    if i.error { "Hata" } else { "Uyarı/not" },
                    s.rows
                        .iter()
                        .find(|r| r.id == i.sheet)
                        .map_or("Pafta seti", |r| r.name.as_str()),
                    i.message,
                    i.fix
                );
                results = results.push(if i.error {
                    label::body(message).style(style::text::danger)
                } else {
                    label::caption(message)
                });
            }
            body = body.push(results);
        }

        let ready = !busy
            && matches!(s.op, CallOp::Plan)
            && self
                .document
                .as_ref()
                .is_some_and(|d| d.model.generation() == s.generation)
            && s.report.as_ref().is_some_and(|r| r.ready);
        let mut window = Dialog::new(if s.check_only {
            "Baskı öncesi tam denetim"
        } else {
            "Paftaları toplu yayınla"
        })
        .scroll(body)
        .width(640.)
        .max_height(640.)
        .aside(
            button(label::body(if busy { "Durdur" } else { "Kapat" }))
                .on_press(event(if busy { Event::Stop } else { Event::Close }))
                .padding(kentos_ui::theme::metrics::action())
                .style(style::button::secondary),
        )
        .action(
            button(label::body("Denetle"))
                .on_press_maybe((!busy).then_some(event(Event::Plan)))
                .padding(kentos_ui::theme::metrics::action())
                .style(style::button::secondary),
        );
        if !s.check_only {
            let resumable = s.folder.as_ref().and_then(|f| f.stopped.as_ref()).is_some_and(|x| x.resumable);
            let updatable = s.folder.as_ref().is_some_and(|f| {
                f.state == kentos_commands::PublishFolderState::Published && !f.stale.is_empty()
            });
            window = window
                .action(
                    button(label::body("Durum"))
                        .on_press_maybe((!busy && !s.input.path.trim().is_empty()).then_some(event(Event::Status)))
                        .padding(kentos_ui::theme::metrics::action())
                        .style(style::button::secondary),
                )
                .action(
                    button(label::body("Devam et"))
                        .on_press_maybe((!busy && resumable).then_some(event(Event::Resume)))
                        .padding(kentos_ui::theme::metrics::action())
                        .style(style::button::secondary),
                )
                .action(
                    button(label::body("Güncelle"))
                        .on_press_maybe((!busy && updatable).then_some(event(Event::Update)))
                        .padding(kentos_ui::theme::metrics::action())
                        .style(style::button::secondary),
                );
        }
        if !s.check_only {
            window = window.action(
                button(label::body("Yayınla"))
                    .on_press_maybe(ready.then_some(event(Event::Publish)))
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::primary),
            );
        }
        window.modal(event(Event::Close))
    }
}

/// The folder's state in a line (docs/adr/1090 §6).
fn folder_text(f: &PublishStatusReport) -> String {
    match f.state {
        kentos_commands::PublishFolderState::Published if f.stale.is_empty() => {
            format!("Teslim güncel: {} paftanın sayfası yayındakiyle aynı.", f.sheets.iter().filter(|x| x.state == SheetPublishState::Current).count())
        }
        kentos_commands::PublishFolderState::Published => format!(
            "{} pafta güncelliğini yitirdi. Güncelle: yeni bir klasöre yalnız onlar yeniden üretilir, öbürleri kendi tarihiyle kopyalanır.",
            f.stale.len()
        ),
        kentos_commands::PublishFolderState::Stopped => "Bu klasörün yayını durduruldu; teslim oluşturulmadı.".into(),
        kentos_commands::PublishFolderState::None => "Bu klasörde yayın yok.".into(),
    }
}

/// What changed in a stale page's sources (docs/adr/1320 §2), as a short tail.
fn changes_text(c: &kentos_commands::SheetSourceChanges) -> String {
    let mut parts = Vec::new();
    for (n, word) in [(c.changed.len(), "nesne değişti"), (c.added.len(), "nesne eklendi"), (c.removed.len(), "nesne çıktı")] {
        if n > 0 {
            parts.push(format!("{n} {word}"));
        }
    }
    if c.sheet {
        parts.push("pafta düzeni".into());
    }
    for p in &c.parts {
        parts.push(
            match p.as_str() {
                "settings" => "proje ayarları",
                "layers" => "katmanlar",
                "styles" => "stiller",
                "blocks" => "bloklar",
                "book" => "şablon/profil",
                "resources" => "bağlı resimler",
                other => other,
            }
            .to_owned(),
        );
    }
    if parts.is_empty() { String::new() } else { format!(" · {}", parts.join(", ")) }
}

/// A viewport's scale as the sheet reads it: 1:20, 1:37,5, 2:1.
fn scale_text(d: f64) -> String {
    let n = |v: f64| {
        let t = format!("{v:.3}");
        t.trim_end_matches('0').trim_end_matches('.').replace('.', ",")
    };
    if d > 0.0 && d < 1.0 { format!("{}:1", n(1.0 / d)) } else { format!("1:{}", n(d)) }
}
