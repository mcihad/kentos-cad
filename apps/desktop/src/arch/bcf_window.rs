//! BCF sorunları (docs/adr/8103; plan/ifc-uyum.md IFC-10; `arch.bcfIssues`): a BCF file's topics read by `cad.bcf.read`
//! (3.0 or 2.1), the IFC file they are about named to lead their objects to it. The topics on the left (DESIGN.md §7.9
//! Liste ve ayrıntı), the chosen one's fields, comments and objects on the right; Çizimde seç selects the drawing's
//! objects the topic's GlobalIds stand for (those KentOS wrote) and shows them.
//!
//! ```text
//! ┌ BCF sorunları ───────────────────────────────────────────────────────────────────────────────┐
//! │ BCF dosyası [/proje/sorunlar.bcf      ] [Aç…]   Model (IFC) [/proje/r2.ifc   ] [Seç…]  [Oku]    │
//! │ ┌ Başlık                  Tür    Durum  Nesne ┐│ Duvar kuzeye kaymış                               │
//! │ │ Duvar kuzeye kaymış     Clash  Open      1  ││ Tür · durum   Clash · Open · High                  │
//! │ └─────────────────────────────────────────────┘│ Yorumlar / Nesneler            [Çizimde seç]       │
//! └──────────────────────────────────────────────────────────────────────────────────────── [Kapat] ┘
//! ```

use std::path::PathBuf;
use std::sync::Arc;
use crate::arch::ArchState;

use iced::widget::{Column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{BcfRead, BcfReadOutput, BcfTopic, CommandResult};
use kentos_domain::{Slot, Uuid};
use kentos_native_application::{ExecutionContext, bcf_exchange};
use kentos_ui::label;
use kentos_ui::theme::typography::scaled;
use kentos_ui::widget::table::{self, Column as Col, Row, Table};
use kentos_ui::widget::{Dialog as Frame, Form, ListDetails, text_field};

use crate::app::{App, Message};
use crate::arch;
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "BCF sorunları";
const LEAD: &str = "Başka bir programın ya da KentOS'un yazdığı BCF dosyasının sorunlarını okur: başlık, tür, durum, \
                    yorumlar ve seçtikleri nesneler. Nesneler model dosyasında ve, KentOS'un yazdığı modelse, çizimde bulunur.";
pub const READ: &str = "Oku";
pub const SELECT: &str = "Çizimde seç";
const CLOSE: &str = "Kapat";
const BODY: f32 = 470.0;
const LIST: f32 = 470.0;

/// Which path a picker is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Field {
    Bcf,
    Model,
}

#[derive(Debug, Default)]
pub(crate) struct Window {
    pub path: String,
    pub model: String,
    pub read: Option<Result<Arc<BcfReadOutput>, String>>,
    pub chosen: Option<usize>,
    pub said: Option<(Kind, String)>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Path(Field, String),
    Pick(Field),
    Picked(Field, Option<PathBuf>),
    Read,
    Choose(usize),
    Select,
    Close,
}

fn msg(e: Event) -> Message {
    Message::from(arch::Event::Bcf(e))
}

/// A BCF date as the window says it: its day and time, the zone as written.
fn when(s: &str) -> String {
    s.replacen('T', " ", 1)
}

impl App {
    /// BCF sorunları (`arch.bcfIssues`).
    pub(crate) fn open_bcf_issues(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        if self.modules.arch().bcf_window.is_none() {
            self.modules.arch_mut().bcf_window = Some(Window::default());
        }
        self.dialog = Some(crate::arch::dialog(crate::arch::ArchDialogKind::Bcf));
    }

    fn bcf_read_now(&mut self) {
        let Some(w) = self.modules.arch().bcf_window.as_ref() else { return };
        let model = (!w.model.trim().is_empty()).then(|| w.model.trim().to_owned());
        let input = BcfRead { path: w.path.trim().to_owned(), model };
        let Some(doc) = self.document.as_mut() else { return };
        let read = match bcf_exchange::read::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => Ok(Arc::new(output)),
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err(error.message)
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => Err("Okunmadı.".into()),
        };
        if let Some(w) = self.modules.arch_mut().bcf_window.as_mut() {
            w.chosen = read.as_ref().ok().and_then(|r| (!r.topics.is_empty()).then_some(0));
            w.read = Some(read);
            w.said = None;
        }
    }

    /// The chosen topic's objects the drawing holds, selected and shown.
    fn bcf_select(&mut self) {
        let Some(w) = self.modules.arch().bcf_window.as_ref() else { return };
        let Some(Ok(r)) = &w.read else { return };
        let Some(t) = w.chosen.and_then(|k| r.topics.get(k)) else { return };
        let gids: Vec<&String> = t.viewpoints.iter().flat_map(|v| &v.selection).collect();
        let uids: Vec<String> =
            r.objects.iter().filter(|o| gids.contains(&&o.global_id)).filter_map(|o| o.uid.clone()).collect();
        let Some(doc) = self.document.as_ref() else { return };
        let slots: Vec<Slot> =
            uids.iter().filter_map(|u| Uuid::parse_str(u).ok()).filter_map(|u| doc.model.slot_of(u)).collect();
        let said = if slots.is_empty() {
            (Kind::Info, "Sorunun nesnelerinin hiçbiri bu çizimde yok: KentOS'un yazdığı modelin nesneleri bulunur.".to_owned())
        } else {
            (Kind::Ok, format!("{} nesne çizimde seçildi.", slots.len()))
        };
        let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
        self.selection.set(slots);
        if !ids.is_empty() {
            self.navigating(|app| app.zoom_to_objects(&ids));
        }
        if let Some(w) = self.modules.arch_mut().bcf_window.as_mut() {
            w.said = Some(said);
        }
    }

    pub(crate) fn bcf_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.modules.arch_mut().bcf_window.as_mut() else { return Task::none() };
        match event {
            Event::Close => {
                self.modules.arch_mut().bcf_window = None;
                self.dialog = None;
            }
            Event::Path(Field::Bcf, t) => w.path = t,
            Event::Path(Field::Model, t) => w.model = t,
            Event::Pick(field) => {
                if let crate::app::Picker::File(path) = &self.picker {
                    return Task::done(msg(Event::Picked(field, Some(path.clone()))));
                }
                let (title, name, ext): (&str, &str, &[&str]) = match field {
                    Field::Bcf => ("Sorunların BCF dosyası", "BCF (.bcf)", &["bcf", "bcfzip"]),
                    Field::Model => ("Sorunların andığı IFC modeli", "IFC (.ifc)", &["ifc", "IFC"]),
                };
                let ext: Vec<String> = ext.iter().map(|e| (*e).to_owned()).collect();
                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title(title)
                            .add_filter(name, &ext)
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    move |p| msg(Event::Picked(field, p)),
                );
            }
            Event::Picked(_, None) => {}
            Event::Picked(Field::Bcf, Some(p)) => {
                w.path = p.display().to_string();
                self.bcf_read_now();
            }
            Event::Picked(Field::Model, Some(p)) => {
                w.model = p.display().to_string();
                if !w.path.trim().is_empty() {
                    self.bcf_read_now();
                }
            }
            Event::Read => self.bcf_read_now(),
            Event::Choose(k) => {
                w.chosen = Some(k);
                w.said = None;
            }
            Event::Select => self.bcf_select(),
        }
        Task::none()
    }

    pub(crate) fn bcf_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.modules.arch().bcf_window else {
            return space::horizontal().into();
        };
        let file = |field: Field, value: &str, hint: &'static str, pick: &'static str| {
            row![
                text_field(hint, value).on_input(move |t| msg(Event::Path(field, t))).width(Fill),
                words::secondary(pick, Some(msg(Event::Pick(field)))),
            ]
            .spacing(6)
            .align_y(Center)
        };
        let head = Form::new()
            .label_width(scaled(110.0))
            .field("BCF dosyası", file(Field::Bcf, &w.path, ".bcf dosyası", "Aç…"))
            .field(
                "Model (IFC)",
                row![file(Field::Model, &w.model, "Sorunların andığı .ifc (isteğe bağlı)", "Seç…"), words::primary(READ, Some(msg(Event::Read)))]
                    .spacing(8)
                    .align_y(Center),
            );
        let mut body = Column::new().spacing(10).push(head);
        match &w.read {
            None => body = body.push(label::caption("BCF dosyasını açın; nesnelerin modeldeki karşılığı için IFC dosyasını da seçin.")),
            Some(Err(why)) => body = body.push(words::text_line(Kind::Warn, why.clone())),
            Some(Ok(r)) => {
                let drawn = r.objects.iter().filter(|o| o.uid.is_some()).count();
                body = body
                    .push(words::text_line(
                        Kind::Info,
                        format!(
                            "BCF {} · {} sorun · {} nesne, {}'i çizimde",
                            r.version,
                            r.topics.len(),
                            r.objects.len(),
                            drawn
                        ),
                    ))
                    .push(ListDetails::new(self.bcf_table(w, r), self.bcf_details(w, r)).list_width(LIST).height(BODY));
            }
        }
        let mut foot = row![].spacing(8).align_y(Center);
        foot = match &w.said {
            Some((kind, said)) => foot.push(container(words::text_line(*kind, said.clone())).width(Fill)),
            None => foot.push(space::horizontal().width(Fill)),
        };
        let foot = foot.push(words::secondary(CLOSE, Some(msg(Event::Close))));
        Frame::new(TITLE).push(label::caption(LEAD)).scroll(body).footer(foot).width(1160.0).modal(msg(Event::Close))
    }

    fn bcf_table<'a>(&'a self, w: &'a Window, r: &'a BcfReadOutput) -> Element<'a, Message> {
        let mut t = Table::new([
            Col::new("Başlık").width(Fill),
            Col::new("Tür").width(scaled(70.0)),
            Col::new("Durum").width(scaled(70.0)),
            Col::new("Nesne").width(scaled(56.0)).align_right(),
        ])
        .empty("Dosyada sorun yok.");
        for (k, x) in r.topics.iter().enumerate() {
            let objects: usize = x.viewpoints.iter().map(|v| v.selection.len()).sum();
            t = t.push(
                Row::new([
                    table::text(x.title.clone()).into(),
                    table::text(x.topic_type.clone()).into(),
                    table::text(x.status.clone()).into(),
                    table::number(objects.to_string()).into(),
                ])
                .selected(w.chosen == Some(k))
                .on_press(msg(Event::Choose(k))),
            );
        }
        t.into()
    }

    fn bcf_details<'a>(&'a self, w: &'a Window, r: &'a BcfReadOutput) -> Element<'a, Message> {
        let Some(t): Option<&BcfTopic> = w.chosen.and_then(|k| r.topics.get(k)) else {
            return label::caption("Soldan bir sorun seçin.").into();
        };
        let mut kind = vec![t.topic_type.clone(), t.status.clone()];
        kind.extend(t.priority.clone());
        let mut form = Form::new()
            .label_width(scaled(110.0))
            .section(t.title.clone())
            .field("Tür · durum", label::body(kind.join(" · ")))
            .field("Yazan", label::body(format!("{} · {}", t.author, when(&t.created))));
        if let Some(a) = &t.assigned_to {
            form = form.field("Atanan", label::body(a.clone()));
        }
        if !t.labels.is_empty() {
            form = form.field("Etiketler", label::body(t.labels.join(", ")));
        }
        if !t.files.is_empty() {
            form = form.field("Model", label::body(t.files.join(", ")));
        }
        if let Some(d) = &t.description {
            form = form.field("Açıklama", label::body(d.clone()));
        }
        let mut out = Column::new().spacing(12).push(form);
        if !t.comments.is_empty() {
            let mut c = Table::new([
                Col::new("Yorum").width(Fill),
                Col::new("Yazan").width(scaled(110.0)),
                Col::new("Gün").width(scaled(200.0)),
            ]);
            for x in &t.comments {
                c = c.push(Row::new([
                    table::text(x.text.clone().unwrap_or_else(|| "(bakış)".into())).into(),
                    table::text(x.author.clone()).into(),
                    table::text(when(&x.date)).into(),
                ]));
            }
            let rows = t.comments.len().clamp(1, 6) as f32 + 1.0;
            out = out.push(c.height(rows * table::row_height() + 2.0));
        }
        let gids: Vec<&String> = t.viewpoints.iter().flat_map(|v| &v.selection).collect();
        let mut o = Table::new([
            Col::new("GlobalId").width(scaled(190.0)),
            Col::new("Modelde").width(Fill),
            Col::new("Çizimde").width(scaled(70.0)),
        ])
        .empty("Sorun bir nesne seçmiyor.");
        for g in &gids {
            let found = r.objects.iter().find(|x| &&x.global_id == g);
            let model = found
                .and_then(|x| x.model.as_ref())
                .map_or_else(|| "—".to_owned(), |m| format!("{} {}", m.class, m.name.clone().unwrap_or_default()));
            let drawn = if found.and_then(|x| x.uid.as_ref()).is_some() { "var" } else { "—" };
            o = o.push(Row::new([table::text((*g).clone()).into(), table::text(model).into(), table::text(drawn).into()]));
        }
        let rows = gids.len().clamp(1, 6) as f32 + 1.0;
        out = out.push(o.height(rows * table::row_height() + 2.0)).push(
            row![space::horizontal().width(Fill), words::secondary(SELECT, Some(msg(Event::Select)))].align_y(Center),
        );
        scrollable(container(out).padding(iced::Padding::ZERO.right(14))).height(Fill).into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn bcf_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.modules.arch().bcf_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(READ) => Some(msg(Event::Read)),
            Control::Press(SELECT) => Some(msg(Event::Select)),
            Control::Fill("BCF dosyası", t) => Some(msg(Event::Path(Field::Bcf, t.to_owned()))),
            Control::Fill("Model (IFC)", t) => Some(msg(Event::Path(Field::Model, t.to_owned()))),
            Control::Row(n) if n >= 1 => Some(msg(Event::Choose(n - 1))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
