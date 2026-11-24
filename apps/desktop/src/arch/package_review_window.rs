//! Gelen paketi incele (docs/adr/8104; plan/bim-plan.md BIM-18, B20; `arch.reviewPackage`): the administration's
//! reviewer opens a package as it came (a publication's folder or a lone IFC file) and `file.review.package` reads it
//! off the window's thread. The five layers on top (Paket, Pafta, Model, Konum, Form: holds, fails, not judged); below,
//! one table at a time (Bulgular: the findings, the chosen one's words and objects on the right; Form: the author's and
//! KentOS's answers side by side). Modelde göster opens the package's model in the IFC viewer with the finding's objects
//! chosen and painted; İade notu yaz… writes the error findings as a BCF file (each topic selecting its objects) and,
//! beside it, the same in words (`.txt`).
//!
//! ```text
//! ┌ Gelen paketi incele ────────────────────────────────────────────────────────────────────────────┐
//! │ Paket [/gelen/260123                ] [Klasör…] [IFC…]  İdarenin sistemi [5256] EPSG  [İncele]      │
//! │ ✓ Paket   ✗ Pafta   ✗ Model   ✗ Konum   ✗ Form                                                    │
//! │ [Bulgular] Form                                                                                     │
//! │ ┌ Katman  Bulgu                          Nesne ┐│ Model · MM-DVR-Dış_duvar_25                         │
//! │ │ Konum   Modelin sistemi EPSG:5255 …        — ││ FireRating yok …  Onarım …  Dayanak …               │
//! │ └──────────────────────────────────────────────┘│ [Modelde göster]                                    │
//! │ Yazar [inceleyen]  [İade notu yaz…]                                                       [Kapat] │
//! └─────────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use std::path::{Path, PathBuf};
use std::sync::Arc;
use crate::arch::ArchState;

use iced::futures::channel::oneshot;
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    BcfTopicInput, BcfWrite, CallEnvelope, CallOp, CommandResult, PackageReviewed, ReviewFinding,
};
use kentos_native_application::{ExecutionContext, bcf_exchange};
use kentos_ui::label;
use kentos_ui::theme::typography::scaled;
use kentos_ui::widget::table::{self, Column as Col, Row, Table};
use kentos_ui::widget::{Dialog as Frame, Form, ListDetails, Segmented, text_field};

use crate::app::{App, Message};
use crate::arch;
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Gelen paketi incele";
const LEAD: &str = "İdarenin inceleyeni için: gelen paketi (yayın klasörü ya da tek bir IFC modeli) müellifin çizimi \
                    olmadan katman katman denetler. Bulgular nesneleriyle iade notuna, BCF'ye ve yazıya dönüşür.";
pub const REVIEW: &str = "İncele";
pub const SHOW: &str = "Modelde göster";
pub const RETURN: &str = "İade notu yaz…";
const CLOSE: &str = "Kapat";
const BODY: f32 = 430.0;
const LIST: f32 = 600.0;

/// The table shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tab {
    #[default]
    Findings,
    Form,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Tab::Findings => "Bulgular",
            Tab::Form => "Form",
        })
    }
}

#[derive(Clone, Debug)]
pub enum Outcome {
    Running,
    Done(Arc<PackageReviewed>),
    Refused(String),
}

#[derive(Debug)]
pub(crate) struct Window {
    pub path: String,
    pub system: String,
    pub outcome: Option<Outcome>,
    pub tab: Tab,
    pub chosen: Option<usize>,
    pub author: String,
    pub said: Option<(Kind, String)>,
}

impl Default for Window {
    fn default() -> Self {
        Window {
            path: String::new(),
            system: String::new(),
            outcome: None,
            tab: Tab::Findings,
            chosen: None,
            author: std::env::var("USER").map(|u| u.trim().to_owned()).unwrap_or_default(),
            said: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Path(String),
    PickFolder,
    PickModel,
    Picked(Option<PathBuf>),
    System(String),
    Review,
    Reviewed(Box<Outcome>),
    Tab(Tab),
    Choose(usize),
    Show,
    Author(String),
    Return,
    ReturnTo(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::from(arch::Event::ReviewPackage(e))
}

/// A layer's name as the window says it.
fn layer_words(key: &str) -> &str {
    match key {
        "paket" => "Paket",
        "pafta" => "Pafta",
        "model" => "Model",
        "konum" => "Konum",
        "form" => "Form",
        other => other,
    }
}

/// The package's model file: in its folder, or the lone file itself.
fn model_path(path: &str, r: &PackageReviewed) -> Option<PathBuf> {
    let p = Path::new(path.trim());
    let name = r.model.as_ref()?;
    let at = if p.is_dir() { p.join(name) } else { p.to_path_buf() };
    at.is_file().then_some(at)
}

/// The return note in words: each error finding with its layer, what it says, how to put it right, its article and
/// objects.
pub(crate) fn letter(r: &PackageReviewed, author: &str) -> String {
    let mut out = format!("İade gerekçeleri\nPaket: {}\nİnceleyen: {}\n\n", r.path, author.trim());
    for (k, f) in r.findings.iter().filter(|f| f.error).enumerate() {
        out.push_str(&format!("{}. [{}] {}\n   Onarım: {}\n   Dayanak: {}\n", k + 1, layer_words(&f.layer), f.message, f.repair, f.cite));
        if let Some(file) = &f.file {
            out.push_str(&format!("   Dosya: {file}\n"));
        }
        if !f.objects.is_empty() {
            out.push_str(&format!("   Nesneler (GlobalId): {}\n", f.objects.join(", ")));
        }
        out.push('\n');
    }
    out
}

fn topic_of(f: &ReviewFinding, author: &str) -> BcfTopicInput {
    let title: String = f.message.lines().next().unwrap_or_default().chars().take(kentos_commands::MAX_BCF_TITLE).collect();
    BcfTopicInput {
        guid: None,
        title,
        description: Some(format!("{}\nOnarım: {}\nDayanak: {}", f.message, f.repair, f.cite)),
        topic_type: Some("Issue".into()),
        status: Some("Open".into()),
        priority: None,
        labels: vec![layer_words(&f.layer).to_owned()],
        author: author.trim().to_owned(),
        date: None,
        assigned_to: None,
        objects: f.objects.iter().take(kentos_commands::MAX_BCF_OBJECTS).cloned().collect(),
        comments: Vec::new(),
    }
}

impl App {
    /// Gelen paketi incele (`arch.reviewPackage`).
    pub(crate) fn open_review_package(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        if self.modules.arch().review_window.is_none() {
            self.modules.arch_mut().review_window = Some(Window::default());
        }
        self.dialog = Some(crate::arch::dialog(crate::arch::ArchDialogKind::ReviewPackage));
    }

    fn review_start(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else { return Task::none() };
        let mut model = doc.model.clone();
        let Some(w) = self.modules.arch_mut().review_window.as_mut() else { return Task::none() };
        let typed = w.system.trim().trim_start_matches("EPSG:").trim();
        let system = if typed.is_empty() {
            None
        } else {
            match typed.parse::<u32>() {
                Ok(s) => Some(s),
                Err(_) => {
                    w.outcome = Some(Outcome::Refused(format!(
                        "İdarenin sistemi “{}” bir EPSG kodu değil: 5256 gibi yazın (TUREF / TM36).",
                        w.system.trim()
                    )));
                    return Task::none();
                }
            }
        };
        let mut input = serde_json::json!({"path": w.path.trim()});
        if let Some(s) = system {
            input["system"] = serde_json::json!(s);
        }
        w.outcome = Some(Outcome::Running);
        w.chosen = None;
        w.said = None;
        let (done, wait) = oneshot::channel();
        let spawned = std::thread::Builder::new().name("paket-inceleme".into()).stack_size(64 << 20).spawn(move || {
            let answer = kentos_headless::file_commands::call(
                &mut model,
                CallEnvelope {
                    command: kentos_commands::FILE_REVIEW_PACKAGE.into(),
                    version: None,
                    op: CallOp::Execute,
                    request_id: None,
                    expected: None,
                    relax_constraints: false,
                    input,
                },
            );
            let _ = done.send(answer.result);
        });
        if spawned.is_err() {
            w.outcome = Some(Outcome::Refused("İnceleme başlatılamadı: iş parçacığı açılmadı.".into()));
            return Task::none();
        }
        Task::perform(wait, |answer| {
            let outcome = match answer {
                Ok(CommandResult::Completed { output, .. }) => match serde_json::from_value::<PackageReviewed>(output) {
                    Ok(r) => Outcome::Done(Arc::new(r)),
                    Err(e) => Outcome::Refused(format!("İncelemenin cevabı okunamadı: {e}")),
                },
                Ok(CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error }) => {
                    Outcome::Refused(error.message)
                }
                Ok(_) | Err(_) => Outcome::Refused("İnceleme yarıda kaldı.".into()),
            };
            msg(Event::Reviewed(Box::new(outcome)))
        })
    }

    /// The chosen finding's objects shown in the IFC viewer, on the package's model.
    fn review_show(&mut self) -> Task<Message> {
        let Some(w) = self.modules.arch().review_window.as_ref() else { return Task::none() };
        let Some(Outcome::Done(r)) = &w.outcome else { return Task::none() };
        let Some(f) = w.chosen.and_then(|k| r.findings.get(k)) else { return Task::none() };
        let (keys, path) = (f.objects.clone(), model_path(&w.path, r));
        let Some(path) = path else {
            if let Some(w) = self.modules.arch_mut().review_window.as_mut() {
                w.said = Some((Kind::Info, "Paketin okunabilir bir modeli yok: gösterilecek nesne yok.".into()));
            }
            return Task::none();
        };
        if keys.is_empty()
            && let Some(w) = self.modules.arch_mut().review_window.as_mut()
        {
            w.said = Some((Kind::Info, "Bu bulgu bir nesneyi anmıyor: model yalnız açılır.".into()));
        }
        self.dialog = None;
        self.open_ifc_model_showing(path, keys)
    }

    /// The error findings written as a BCF file at `path` and in words beside it.
    fn review_return(&mut self, path: PathBuf) {
        let Some(w) = self.modules.arch().review_window.as_ref() else { return };
        let Some(Outcome::Done(r)) = &w.outcome else { return };
        let errors: Vec<&ReviewFinding> = r.findings.iter().filter(|f| f.error).collect();
        let model = model_path(&w.path, r);
        // Objects only where the model can show them.
        let topics: Vec<BcfTopicInput> = errors
            .iter()
            .map(|f| {
                let mut t = topic_of(f, &w.author);
                if model.is_none() {
                    t.objects.clear();
                }
                t
            })
            .collect();
        let text_path = path.with_extension("txt");
        let note = letter(r, &w.author);
        let input = BcfWrite {
            path: path.display().to_string(),
            overwrite: true,
            model: model.map(|m| m.display().to_string()),
            topics,
        };
        let Some(doc) = self.document.as_mut() else { return };
        let said = match bcf_exchange::write::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => match std::fs::write(&text_path, note) {
                Ok(()) => (
                    Kind::Ok,
                    format!(
                        "İade notu: {} gerekçe “{}” ve “{}” dosyalarına yazıldı.",
                        output.topics,
                        path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                        text_path.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()
                    ),
                ),
                Err(e) => (Kind::Warn, format!("BCF yazıldı; yazılı not yazılamadı: {e}")),
            },
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                (Kind::Warn, error.message)
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => (Kind::Warn, "Yazılmadı.".into()),
        };
        if let Some(w) = self.modules.arch_mut().review_window.as_mut() {
            w.said = Some(said);
        }
    }

    pub(crate) fn review_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.modules.arch_mut().review_window.as_mut() else { return Task::none() };
        match event {
            Event::Close => {
                self.modules.arch_mut().review_window = None;
                self.dialog = None;
            }
            Event::Path(t) => w.path = t,
            Event::System(t) => w.system = t,
            Event::PickFolder | Event::PickModel => {
                if let crate::app::Picker::File(path) = &self.picker {
                    return Task::done(msg(Event::Picked(Some(path.clone()))));
                }
                let folder = matches!(event, Event::PickFolder);
                return Task::perform(
                    async move {
                        let d = rfd::AsyncFileDialog::new();
                        let picked = if folder {
                            d.set_title("Gelen paketin klasörü").pick_folder().await
                        } else {
                            d.set_title("Gelen IFC modeli").add_filter("IFC (.ifc)", &["ifc", "IFC"]).pick_file().await
                        };
                        picked.map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Picked(p)),
                );
            }
            Event::Picked(None) => {}
            Event::Picked(Some(p)) => w.path = p.display().to_string(),
            Event::Review => return self.review_start(),
            Event::Reviewed(outcome) => {
                if let Outcome::Done(r) = outcome.as_ref() {
                    w.chosen = (!r.findings.is_empty()).then_some(0);
                    w.tab = Tab::Findings;
                }
                w.outcome = Some(*outcome);
            }
            Event::Tab(t) => w.tab = t,
            Event::Choose(k) => {
                w.chosen = Some(k);
                w.said = None;
            }
            Event::Show => return self.review_show(),
            Event::Author(t) => w.author = t,
            Event::Return => {
                let Some(Outcome::Done(r)) = &w.outcome else { return Task::none() };
                if !r.findings.iter().any(|f| f.error) {
                    w.said = Some((Kind::Info, "Hata bulgusu yok: iade gerekçesi yazılmaz.".into()));
                    return Task::none();
                }
                if w.author.trim().is_empty() {
                    w.said = Some((Kind::Warn, "Yazarı yazın: iade notunun yazarı olur.".into()));
                    return Task::none();
                }
                if let crate::app::Picker::File(path) = &self.picker {
                    return Task::done(msg(Event::ReturnTo(Some(path.clone()))));
                }
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("İade notunun BCF dosyası")
                            .add_filter("BCF (.bcf)", &["bcf"])
                            .set_file_name("iade-notu.bcf")
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::ReturnTo(p)),
                );
            }
            Event::ReturnTo(None) => {}
            Event::ReturnTo(Some(p)) => self.review_return(p),
        }
        Task::none()
    }

    pub(crate) fn review_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.modules.arch().review_window else {
            return space::horizontal().into();
        };
        let head = Form::new()
            .label_width(scaled(110.0))
            .field(
                "Paket",
                row![
                    text_field("Yayın klasörü ya da .ifc dosyası", &w.path).on_input(|t| msg(Event::Path(t))).width(Fill),
                    words::secondary("Klasör…", Some(msg(Event::PickFolder))),
                    words::secondary("IFC…", Some(msg(Event::PickModel))),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .field(
                "İdarenin sistemi",
                row![
                    text_field("5256", &w.system).on_input(|t| msg(Event::System(t))).width(scaled(90.0)),
                    label::body("EPSG (boşsa konum değerlendirilmez)"),
                    space::horizontal().width(Fill),
                    words::primary(REVIEW, Some(msg(Event::Review))),
                ]
                .spacing(6)
                .align_y(Center),
            );
        let mut body = Column::new().spacing(10).push(head);
        match &w.outcome {
            None => body = body.push(label::caption("Paketi seçip İncele'ye basın.")),
            Some(Outcome::Running) => body = body.push(words::text_line(Kind::Info, "İnceleniyor…")),
            Some(Outcome::Refused(why)) => body = body.push(words::text_line(Kind::Warn, why.clone())),
            Some(Outcome::Done(r)) => {
                let mut layers = row![].spacing(18).align_y(Center);
                for l in &r.layers {
                    let kind = match l.holds {
                        Some(true) => Kind::Ok,
                        Some(false) => Kind::Error,
                        None => Kind::Info,
                    };
                    layers = layers.push(words::text_line(kind, layer_words(&l.key).to_owned()));
                }
                let errors = r.findings.iter().filter(|f| f.error).count();
                let summary = format!(
                    "{} dosya · {} hata, {} uyarı{}",
                    r.files.len(),
                    errors,
                    r.findings.len() - errors,
                    r.model.as_ref().map_or_else(String::new, |m| format!(" · model {m}"))
                ) + &r.table.as_ref().map_or_else(String::new, |t| format!(" · form EK-9 Tablo {t}"));
                body = body
                    .push(layers)
                    .push(label::caption(summary))
                    .push(Segmented::new([Tab::Findings, Tab::Form], w.tab, |t| msg(Event::Tab(t))));
                body = body.push(match w.tab {
                    Tab::Findings => {
                        ListDetails::new(self.review_table(w, r), self.review_details(w, r)).list_width(LIST).height(BODY).into()
                    }
                    Tab::Form => self.review_form(r),
                });
            }
        }
        let mut foot = row![
            label::body("Yazar"),
            text_field("adınız", &w.author).on_input(|t| msg(Event::Author(t))).width(scaled(160.0)),
            words::secondary(RETURN, Some(msg(Event::Return))),
        ]
        .spacing(8)
        .align_y(Center);
        foot = match &w.said {
            Some((kind, said)) => foot.push(container(words::text_line(*kind, said.clone())).width(Fill)),
            None => foot.push(space::horizontal().width(Fill)),
        };
        let foot = foot.push(words::secondary(CLOSE, Some(msg(Event::Close))));
        Frame::new(TITLE).push(label::caption(LEAD)).scroll(body).footer(foot).width(1160.0).modal(msg(Event::Close))
    }

    fn review_table<'a>(&'a self, w: &'a Window, r: &'a PackageReviewed) -> Element<'a, Message> {
        let mut t = Table::new([
            Col::new("Katman").width(scaled(70.0)),
            Col::new("Bulgu").width(Fill),
            Col::new("Nesne").width(scaled(56.0)).align_right(),
        ])
        .empty("Bulgu yok: paket her katmanda tutuyor.");
        for (k, f) in r.findings.iter().enumerate() {
            let layer = if f.error { layer_words(&f.layer).to_owned() } else { format!("{} (uyarı)", layer_words(&f.layer)) };
            t = t.push(
                Row::new([
                    table::text(layer).into(),
                    table::text(f.message.clone()).into(),
                    table::number(if f.objects.is_empty() { "—".to_owned() } else { f.objects.len().to_string() }).into(),
                ])
                .selected(w.chosen == Some(k))
                .on_press(msg(Event::Choose(k))),
            );
        }
        t.into()
    }

    fn review_details<'a>(&'a self, w: &'a Window, r: &'a PackageReviewed) -> Element<'a, Message> {
        let Some(f) = w.chosen.and_then(|k| r.findings.get(k)) else {
            return label::caption("Bir bulgu seçin.").into();
        };
        let mut form = Form::new()
            .label_width(scaled(90.0))
            .section(format!("{} · {}", layer_words(&f.layer), f.code))
            .field("Bulgu", label::body(f.message.clone()))
            .field("Onarım", label::body(f.repair.clone()))
            .field("Dayanak", label::body(f.cite.clone()));
        if let Some(file) = &f.file {
            form = form.field("Dosya", label::body(file.clone()));
        }
        if !f.objects.is_empty() {
            form = form.field("Nesneler", label::body(f.objects.join(", ")));
        }
        let out = Column::new()
            .spacing(10)
            .push(form)
            .push(row![space::horizontal().width(Fill), words::secondary(SHOW, Some(msg(Event::Show)))].align_y(Center));
        scrollable(container(out).padding(iced::Padding::ZERO.right(14))).height(Fill).into()
    }

    fn review_form<'a>(&'a self, r: &'a PackageReviewed) -> Element<'a, Message> {
        let mut t = Table::new([
            Col::new("No").width(scaled(40.0)).align_right(),
            Col::new("Başlık").width(Fill),
            Col::new("Müellif").width(scaled(90.0)),
            Col::new("KentOS").width(scaled(90.0)),
            Col::new("Durum").width(scaled(140.0)),
        ])
        .empty("Model okunmadı: formun soruları cevaplanamadı.");
        for row_ in &r.rows {
            let state = match row_.agree {
                Some(true) => words::text_line(Kind::Ok, "aynı"),
                Some(false) => words::text_line(Kind::Error, "farklı"),
                None => words::text_line(Kind::Info, if row_.author.is_none() { "form yok" } else { "inceleme" }),
            };
            t = t.push(Row::new([
                table::number(row_.no.to_string()).into(),
                table::text(row_.title.clone()).into(),
                table::text(row_.author.clone().unwrap_or_else(|| "—".into())).into(),
                table::text(row_.kentos.clone()).into(),
                state,
            ]));
        }
        t.height(BODY).into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn review_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.modules.arch().review_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(REVIEW) => Some(msg(Event::Review)),
            Control::Press(SHOW) => Some(msg(Event::Show)),
            Control::Press(RETURN) => Some(msg(Event::Return)),
            Control::Fill("Paket", t) => Some(msg(Event::Path(t.to_owned()))),
            Control::Fill("İdarenin sistemi", t) => Some(msg(Event::System(t.to_owned()))),
            Control::Fill("Yazar", t) => Some(msg(Event::Author(t.to_owned()))),
            Control::Row(n) if n >= 1 => Some(msg(Event::Choose(n - 1))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
