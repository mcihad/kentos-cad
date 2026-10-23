//! Sheet publication is a normal cancellable protocol job; the window only gathers options and shows its report.
use std::{path::PathBuf, time::Duration};
use iced::{Element, Fill, Task};
use iced::widget::{button, column, row, scrollable, text_input, progress_bar};
use iced::futures::{channel::mpsc, SinkExt, Stream};
use kentos_commands::{
    PublishSheets, SheetsPublished, SheetPublishMode, CallEnvelope, CallOp, CommandResult,
    FILE_PUBLISH_SHEETS,
};
use kentos_native_application::jobs::Job;
use kentos_ui::{
    label, style,
    widget::{Dialog, Switch},
};
use crate::app::{App, Message};
use super::{Window, Event as Exchange, message};
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
            Event::Plan | Event::Publish => {
                let Some(doc) = &self.document else {
                    return Task::none();
                };
                let mut input = s.input.clone();
                input.sheets = Some(
                    s.rows
                        .iter()
                        .filter(|r| r.selected)
                        .map(|r| r.id.clone())
                        .collect(),
                );
                let op = if matches!(e, Event::Plan) {
                    CallOp::Plan
                } else {
                    CallOp::Execute
                };
                if s.check_only && matches!(op, CallOp::Execute) {
                    return Task::none();
                }
                if matches!(op, CallOp::Execute) {
                    let Some(report) = &s.report else {
                        return Task::none();
                    };
                    if !report.ready {
                        return Task::none();
                    }
                    input.pin = Some(report.source.clone());
                }
                let command = if s.check_only {
                    kentos_commands::FILE_PREFLIGHT_SHEETS
                } else {
                    FILE_PUBLISH_SHEETS
                };
                let encoded = if s.check_only {
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
                s.status = if matches!(op, CallOp::Plan) {
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
                    kentos_headless::file_commands::job_work(envelope),
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
                                    } else if matches!(s.op, CallOp::Execute) {
                                        format!(
                                            "{} pafta, {} PDF yayınlandı: {}",
                                            report.sheets.len(),
                                            report.files.len(),
                                            report.path
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
                            s.status = "Yayın iptal edildi; kısmi teslim oluşturulmadı.".into()
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
                        .on_press_maybe((!busy && i > 0).then_some(event(Event::Move(i, -1))))
                        .style(style::button::secondary),
                    button(label::caption("Aşağı"))
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
                    .on_press_maybe((!busy).then_some(event(Event::All(true))))
                    .style(style::button::secondary),
                button(label::caption("Seçimi kaldır"))
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
                let mut field =
                    text_input("{index}-{sheet}", &s.input.naming).style(style::field::input);
                if !busy {
                    field = field.on_input(|v| event(Event::Naming(v)));
                }
                body = body.push(field);
            } else {
                body = body.push(label::caption("Tek PDF'in dosya adı"));
                let mut field = text_input("paftalar", s.input.name.as_deref().unwrap_or(""))
                    .style(style::field::input);
                if !busy {
                    field = field.on_input(|v| event(Event::Name(v)));
                }
                body = body.push(field);
            }
            let mut path = text_input("Yeni veya boş teslim klasörünün tam yolu", &s.input.path)
                .style(style::field::input);
            if !busy {
                path = path.on_input(|v| event(Event::Path(v)));
            }
            body = body
                .push(label::caption(
                    "Teslim klasörü · dolu bir klasörün üzerine yazılmaz",
                ))
                .push(
                    row![
                        path,
                        button(label::body("Seç…"))
                            .on_press_maybe((!busy).then_some(event(Event::Pick)))
                            .style(style::button::secondary)
                    ]
                    .spacing(6),
                );
        }
        if busy {
            body = body.push(progress_bar(0.0..=1.0, s.progress));
        }
        if !s.status.is_empty() {
            body = body.push(label::body(&s.status));
        }
        if let Some(report) = &s.report {
            let mut results = column![].spacing(4);
            if let Some(resources) = &report.resources {
                results = results.push(label::caption(format!(
                    "Kaynak paketi: {} · {} dosya",
                    resources.project,
                    resources.files.len()
                )));
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
        let ready =
            !busy && matches!(s.op, CallOp::Plan) && s.report.as_ref().is_some_and(|r| r.ready);
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
                .padding([6, 14])
                .style(style::button::secondary),
        )
        .action(
            button(label::body("Denetle"))
                .on_press_maybe((!busy).then_some(event(Event::Plan)))
                .padding([6, 14])
                .style(style::button::secondary),
        );
        if !s.check_only {
            window = window.action(
                button(label::body("Yayınla"))
                    .on_press_maybe(ready.then_some(event(Event::Publish)))
                    .padding([6, 16])
                    .style(style::button::primary),
            );
        }
        kentos_ui::widget::overlay::modal(window, event(Event::Close))
    }
}
