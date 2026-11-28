//! IFC revizyonlarını karşılaştır (docs/adr/8103; plan/ifc-uyum.md IFC-10; `arch.ifcCompare`): two revisions of a model
//! compared by `cad.ifc.compare` off the window's thread, one table at a time (Değişen, Eklenen, Silinen; DESIGN.md §7.9
//! Liste ve ayrıntı: the rows on the left, the chosen object's changes in words on the right); the rows marked are
//! written as issues by `cad.bcf.write`, each topic selecting its object in the newer file.
//!
//! ```text
//! ┌ IFC revizyonlarını karşılaştır ───────────────────────────────────────────────────────────────┐
//! │ Eski revizyon  [/proje/r1.ifc                 ] [Seç…]   Tolerans [0.001] m   [Karşılaştır]     │
//! │ Yeni revizyon  [/proje/r2.ifc                 ] [Seç…]                                          │
//! │ ⚠ 2 nesne değişti · 0 eklendi · 0 silindi · 1 418 aynı                                          │
//! │ [Değişen] Eklenen  Silinen                                                                      │
//! │ ┌ ☐ Sınıf   Ad              Değişiklik       ┐│ IfcWall MM-DVR-İç_duvar_15                        │
//! │ │ ☑ IfcWall MM-DVR-İç_duvar  Taşındı 0,500 m ││ Taşındı  0,500 m (x 0,000 · y 0,500 · z 0,000)     │
//! │ └────────────────────────────────────────────┘│                                                    │
//! │ Yazar [mimar]  [İşaretlileri BCF'ye yaz…]                                              [Kapat] │
//! └─────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::arch::ArchState;
use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use iced::futures::channel::oneshot;
use iced::widget::{Column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    BcfTopicInput, BcfWrite, CommandResult, IfcChange, IfcCompare, IfcCompared, IfcComparedObject,
    IfcValue,
};
use kentos_native_application::{ExecutionContext, bcf_exchange, ifc_compare};
use kentos_ui::label;
use kentos_ui::theme::typography::scaled;
use kentos_ui::widget::table::{self, Column as Col, Row, Table};
use kentos_ui::widget::{Dialog as Frame, Form, ListDetails, Segmented, text_field};

use crate::app::{App, Message};
use crate::arch;
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "IFC revizyonlarını karşılaştır";
const LEAD: &str = "Bir modelin iki revizyonunu GlobalId'leriyle karşılaştırır: eklenen, silinen, taşınan, boyu ya da \
                    özelliği değişen nesneler. İşaretlenenler sorun olarak BCF dosyasına yazılır; başka bir program aynı \
                    nesneyi gösterir.";
pub const COMPARE: &str = "Karşılaştır";
pub const WRITE: &str = "İşaretlileri BCF'ye yaz…";
const CLOSE: &str = "Kapat";
const BODY: f32 = 470.0;
const LIST: f32 = 560.0;

/// Which file a picker is for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Older,
    Newer,
}

/// The table shown.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
pub enum Tab {
    #[default]
    Changed,
    Added,
    Removed,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Tab::Changed => "Değişen",
            Tab::Added => "Eklenen",
            Tab::Removed => "Silinen",
        })
    }
}

/// Where a comparison stands.
#[derive(Clone, Debug)]
pub enum Outcome {
    Running,
    Done(Arc<IfcCompared>),
    Refused(String),
}

#[derive(Debug)]
pub(crate) struct Window {
    pub older: String,
    pub newer: String,
    pub tolerance: String,
    pub outcome: Option<Outcome>,
    pub tab: Tab,
    /// The row chosen in the table shown.
    pub chosen: Option<usize>,
    /// The rows marked to be written as issues.
    pub marked: BTreeSet<(Tab, usize)>,
    pub author: String,
    pub said: Option<(Kind, String)>,
}

impl Default for Window {
    fn default() -> Self {
        Window {
            older: String::new(),
            newer: String::new(),
            tolerance: "0.001".into(),
            outcome: None,
            tab: Tab::Changed,
            chosen: None,
            marked: BTreeSet::new(),
            author: std::env::var("USER")
                .map(|u| u.trim().to_owned())
                .unwrap_or_default(),
            said: None,
        }
    }
}

#[derive(Clone, Debug)]
pub enum Event {
    Path(Side, String),
    Pick(Side),
    Picked(Side, Option<PathBuf>),
    Tolerance(String),
    Compare,
    Compared(Box<Outcome>),
    Tab(Tab),
    Choose(usize),
    Mark(usize, bool),
    Author(String),
    Write,
    WriteTo(Option<PathBuf>),
    Close,
}

fn msg(e: Event) -> Message {
    Message::from(arch::Event::IfcCompare(e))
}

fn metres(v: f64) -> String {
    format!("{v:.3}").replace('.', ",")
}

/// A value as a change says it.
pub(crate) fn value_words(v: Option<&IfcValue>) -> String {
    let Some(v) = v else { return "—".into() };
    match v {
        IfcValue::Text { text } => text.clone(),
        IfcValue::Number { number, .. } => format!("{number}").replace('.', ","),
        IfcValue::Integer { integer, .. } => integer.to_string(),
        IfcValue::Logical { logical } => match logical {
            Some(true) => "doğru".into(),
            Some(false) => "yanlış".into(),
            None => "bilinmiyor".into(),
        },
        IfcValue::Enumeration { items } | IfcValue::List { items } => items.join(", "),
        IfcValue::Bounded { lower, upper, .. } => format!(
            "{}–{}",
            lower.map_or_else(|| "…".to_owned(), |x| format!("{x}")),
            upper.map_or_else(|| "…".to_owned(), |x| format!("{x}"))
        ),
        IfcValue::Reference { step } => format!("#{step}"),
        IfcValue::Empty => "—".into(),
    }
}

fn object_words(o: Option<&IfcComparedObject>) -> String {
    o.map_or_else(
        || "—".to_owned(),
        |o| o.name.clone().unwrap_or_else(|| o.global_id.clone()),
    )
}

/// A change as a field says it: its label and its words.
pub(crate) fn change_words(c: &IfcChange) -> (String, String) {
    match c {
        IfcChange::Class { old, new } => ("Sınıfı".into(), format!("{old} → {new}")),
        IfcChange::Attribute { name, old, new } => {
            let label = match name.as_str() {
                "Name" => "Adı",
                "Description" => "Açıklaması",
                "ObjectType" => "Nesne türü",
                "LongName" => "Uzun adı",
                "Tag" => "Etiketi",
                "PredefinedType" => "Ön tanımlı türü",
                other => other,
            };
            let w = |v: &Option<String>| v.clone().unwrap_or_else(|| "—".into());
            (label.into(), format!("{} → {}", w(old), w(new)))
        }
        IfcChange::Container { old, new } => (
            "Katı".into(),
            format!(
                "{} → {}",
                object_words(old.as_ref()),
                object_words(new.as_ref())
            ),
        ),
        IfcChange::Type { old, new } => (
            "Tipi".into(),
            format!(
                "{} → {}",
                object_words(old.as_ref()),
                object_words(new.as_ref())
            ),
        ),
        IfcChange::Body { old, new } => {
            let w = |b: &bool| if *b { "var" } else { "yok" };
            ("Gövdesi".into(), format!("{} → {}", w(old), w(new)))
        }
        IfcChange::Moved { by, distance } => (
            "Taşındı".into(),
            format!(
                "{} m (x {} · y {} · z {})",
                metres(*distance),
                metres(by.first().copied().unwrap_or(0.0)),
                metres(by.get(1).copied().unwrap_or(0.0)),
                metres(by.get(2).copied().unwrap_or(0.0))
            ),
        ),
        IfcChange::Resized { old, new } => {
            let size = |v: &Vec<f64>| v.iter().map(|x| metres(*x)).collect::<Vec<_>>().join(" × ");
            ("Boyu".into(), format!("{} → {} m", size(old), size(new)))
        }
        IfcChange::Property {
            set,
            from_type,
            name,
            old,
            new,
        } => {
            let label = if *from_type {
                format!("{set}.{name} (tipinden)")
            } else {
                format!("{set}.{name}")
            };
            (
                label,
                format!(
                    "{} → {}",
                    value_words(old.as_ref()),
                    value_words(new.as_ref())
                ),
            )
        }
    }
}

/// What a comparison says, in a line.
pub(crate) fn summary(c: &IfcCompared) -> String {
    let mut s = format!(
        "{} nesne değişti · {} eklendi · {} silindi · {} aynı",
        c.changed.len(),
        c.added.len(),
        c.removed.len(),
        c.unchanged
    );
    if !c.repeated.is_empty() {
        s.push_str(&format!(
            " · {} GlobalId tekrarlı, karşılaştırılmadı",
            c.repeated.len()
        ));
    }
    s
}

/// The issue a marked row is written as.
fn topic_of(c: &IfcCompared, tab: Tab, k: usize, author: &str) -> Option<BcfTopicInput> {
    let named = |o: &IfcComparedObject| match &o.name {
        Some(n) => format!("{} {n}", o.class),
        None => format!("{} {}", o.class, o.global_id),
    };
    let (title, description, objects) = match tab {
        Tab::Changed => {
            let x = c.changed.get(k)?;
            let o = IfcComparedObject {
                global_id: x.global_id.clone(),
                class: x.class.clone(),
                name: x.name.clone(),
            };
            let lines: Vec<String> = x
                .changes
                .iter()
                .map(change_words)
                .map(|(l, w)| format!("{l}: {w}"))
                .collect();
            let first = lines.first().cloned().unwrap_or_default();
            let more = if lines.len() > 1 {
                format!(" (ve {} değişiklik daha)", lines.len() - 1)
            } else {
                String::new()
            };
            (
                format!("{}: {first}{more}", named(&o)),
                lines.join("\n"),
                vec![x.global_id.clone()],
            )
        }
        Tab::Added => {
            let o = c.added.get(k)?;
            (
                format!("Yeni nesne: {}", named(o)),
                format!("Yalnız yeni revizyonda: {}", o.global_id),
                vec![o.global_id.clone()],
            )
        }
        Tab::Removed => {
            let o = c.removed.get(k)?;
            (
                format!("Silinen nesne: {}", named(o)),
                format!("Yalnız eski revizyondaydı: {}", o.global_id),
                Vec::new(),
            )
        }
    };
    let title: String = title.chars().take(kentos_commands::MAX_BCF_TITLE).collect();
    Some(BcfTopicInput {
        guid: None,
        title,
        description: Some(description),
        topic_type: Some("Issue".into()),
        status: Some("Open".into()),
        priority: None,
        labels: Vec::new(),
        author: author.trim().to_owned(),
        date: None,
        assigned_to: None,
        objects,
        comments: Vec::new(),
    })
}

/// The module's `App` methods of this file, as an extension trait (docs/adr/9199 §3.5).
pub(crate) trait IfcCompareWindowApp {
    fn open_ifc_compare(&mut self);
    fn ifc_compare_start(&mut self) -> Task<Message>;
    fn ifc_compare_write(&mut self, path: PathBuf);
    fn ifc_compare_event(&mut self, event: Event) -> Task<Message>;
    fn ifc_compare_view(&self) -> Element<'_, Message>;
    fn ifc_compare_table<'a>(&'a self, w: &'a Window, c: &'a IfcCompared) -> Element<'a, Message>;
    fn ifc_compare_details<'a>(&'a self, w: &'a Window, c: &'a IfcCompared)
    -> Element<'a, Message>;
    fn ifc_compare_control(&self, control: Control<'_>) -> Result<Option<Message>, String>;
}

impl IfcCompareWindowApp for App {
    /// IFC revizyonlarını karşılaştır (`arch.ifcCompare`).
    fn open_ifc_compare(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        if self.modules.arch().ifc_compare_window.is_none() {
            self.modules.arch_mut().ifc_compare_window = Some(Window::default());
        }
        self.dialog = Some(crate::arch::dialog(crate::arch::ArchDialogKind::IfcCompare));
    }

    fn ifc_compare_start(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let mut model = doc.model.clone();
        let Some(w) = self.modules.arch_mut().ifc_compare_window.as_mut() else {
            return Task::none();
        };
        let typed = w.tolerance.trim().replace(',', ".");
        let tolerance = if typed.is_empty() {
            None
        } else {
            match typed.parse::<f64>() {
                Ok(t) if t.is_finite() => Some(t),
                _ => {
                    w.outcome = Some(Outcome::Refused(format!(
                        "Tolerans “{}” bir sayı değil: metre olarak yazın (0.001 bir milimetredir).",
                        w.tolerance.trim()
                    )));
                    return Task::none();
                }
            }
        };
        let input = IfcCompare {
            older: w.older.trim().to_owned(),
            newer: w.newer.trim().to_owned(),
            tolerance,
        };
        w.outcome = Some(Outcome::Running);
        w.chosen = None;
        w.marked.clear();
        w.said = None;
        let (done, wait) = oneshot::channel();
        let spawned = std::thread::Builder::new()
            .name("ifc-karsilastir".into())
            .stack_size(64 << 20)
            .spawn(move || {
                let _ = done.send(ifc_compare::execute(
                    &mut ExecutionContext::new(&mut model),
                    input,
                ));
            });
        if spawned.is_err() {
            w.outcome = Some(Outcome::Refused(
                "Karşılaştırma başlatılamadı: iş parçacığı açılmadı.".into(),
            ));
            return Task::none();
        }
        Task::perform(wait, |answer| {
            let outcome = match answer {
                Ok(CommandResult::Completed { output, .. }) => Outcome::Done(Arc::new(output)),
                Ok(
                    CommandResult::Failed { error }
                    | CommandResult::Conflict { error }
                    | CommandResult::NeedsInput { error },
                ) => Outcome::Refused(error.message),
                Ok(_) | Err(_) => Outcome::Refused("Karşılaştırma yarıda kaldı.".into()),
            };
            msg(Event::Compared(Box::new(outcome)))
        })
    }

    /// The marked rows written as issues at `path`; the window says how it went.
    fn ifc_compare_write(&mut self, path: PathBuf) {
        let Some(w) = self.modules.arch().ifc_compare_window.as_ref() else {
            return;
        };
        let Some(Outcome::Done(c)) = &w.outcome else {
            return;
        };
        let topics: Vec<BcfTopicInput> = w
            .marked
            .iter()
            .filter_map(|(tab, k)| topic_of(c, *tab, *k, &w.author))
            .collect();
        let input = BcfWrite {
            path: path.display().to_string(),
            overwrite: true,
            model: Some(w.newer.trim().to_owned()),
            topics,
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let said =
            match bcf_exchange::write::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                CommandResult::Completed { output, .. } => (
                    Kind::Ok,
                    format!(
                        "{} sorun “{}” dosyasına yazıldı ({} bakış).",
                        output.topics,
                        path.file_name().map_or_else(
                            || path.display().to_string(),
                            |n| n.to_string_lossy().into_owned()
                        ),
                        output.viewpoints
                    ),
                ),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => (Kind::Warn, error.message),
                CommandResult::Queued { .. } | CommandResult::Cancelled => {
                    (Kind::Warn, "Yazılmadı.".into())
                }
            };
        if let Some(w) = self.modules.arch_mut().ifc_compare_window.as_mut() {
            w.said = Some(said);
        }
    }

    fn ifc_compare_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.modules.arch_mut().ifc_compare_window.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Close => {
                self.modules.arch_mut().ifc_compare_window = None;
                self.dialog = None;
            }
            Event::Path(Side::Older, t) => w.older = t,
            Event::Path(Side::Newer, t) => w.newer = t,
            Event::Pick(side) => {
                if let crate::app::Picker::File(path) = &self.picker {
                    return Task::done(msg(Event::Picked(side, Some(path.clone()))));
                }
                let title = if side == Side::Older {
                    "Eski revizyonun IFC dosyası"
                } else {
                    "Yeni revizyonun IFC dosyası"
                };
                return Task::perform(
                    async move {
                        rfd::AsyncFileDialog::new()
                            .set_title(title)
                            .add_filter("IFC (.ifc)", &["ifc", "IFC"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    move |p| msg(Event::Picked(side, p)),
                );
            }
            Event::Picked(_, None) => {}
            Event::Picked(Side::Older, Some(p)) => w.older = p.display().to_string(),
            Event::Picked(Side::Newer, Some(p)) => w.newer = p.display().to_string(),
            Event::Tolerance(t) => w.tolerance = t,
            Event::Compare => return self.ifc_compare_start(),
            Event::Compared(outcome) => {
                if let Outcome::Done(c) = outcome.as_ref() {
                    w.tab = if !c.changed.is_empty() {
                        Tab::Changed
                    } else if !c.added.is_empty() {
                        Tab::Added
                    } else if !c.removed.is_empty() {
                        Tab::Removed
                    } else {
                        Tab::Changed
                    };
                    w.chosen = None;
                }
                w.outcome = Some(*outcome);
            }
            Event::Tab(t) => {
                w.tab = t;
                w.chosen = None;
            }
            Event::Choose(k) => w.chosen = Some(k),
            Event::Mark(k, on) => {
                if on {
                    w.marked.insert((w.tab, k));
                } else {
                    w.marked.remove(&(w.tab, k));
                }
            }
            Event::Author(t) => w.author = t,
            Event::Write => {
                if w.marked.is_empty() {
                    w.said = Some((
                        Kind::Info,
                        "Önce sorun olarak yazılacak satırları işaretleyin.".into(),
                    ));
                    return Task::none();
                }
                if w.author.trim().is_empty() {
                    w.said = Some((
                        Kind::Warn,
                        "Yazarı yazın: BCF her sorunun yazarını ister.".into(),
                    ));
                    return Task::none();
                }
                if let crate::app::Picker::File(path) = &self.picker {
                    return Task::done(msg(Event::WriteTo(Some(path.clone()))));
                }
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Sorunların BCF dosyası")
                            .add_filter("BCF (.bcf)", &["bcf"])
                            .set_file_name("sorunlar.bcf")
                            .save_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::WriteTo(p)),
                );
            }
            Event::WriteTo(None) => {}
            Event::WriteTo(Some(path)) => self.ifc_compare_write(path),
        }
        Task::none()
    }

    fn ifc_compare_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.modules.arch().ifc_compare_window else {
            return space::horizontal().into();
        };
        let file = |side: Side, value: &str, hint: &'static str| {
            row![
                text_field(hint, value)
                    .on_input(move |t| msg(Event::Path(side, t)))
                    .width(Fill),
                words::secondary("Seç…", Some(msg(Event::Pick(side)))),
            ]
            .spacing(6)
            .align_y(Center)
        };
        let head = Form::new()
            .label_width(scaled(110.0))
            .field(
                "Eski revizyon",
                file(Side::Older, &w.older, "Eski revizyonun .ifc dosyası"),
            )
            .field(
                "Yeni revizyon",
                file(Side::Newer, &w.newer, "Yeni revizyonun .ifc dosyası"),
            )
            .field(
                "Tolerans",
                row![
                    text_field("0.001", &w.tolerance)
                        .on_input(|t| msg(Event::Tolerance(t)))
                        .width(scaled(90.0)),
                    label::body("m"),
                    space::horizontal().width(scaled(8.0)),
                    words::primary(COMPARE, Some(msg(Event::Compare))),
                ]
                .spacing(6)
                .align_y(Center),
            );
        let mut body = Column::new().spacing(10).push(head);
        match &w.outcome {
            None => body = body.push(label::caption("İki revizyonu seçip Karşılaştır'a basın.")),
            Some(Outcome::Running) => {
                body = body.push(words::text_line(Kind::Info, "Karşılaştırılıyor…"))
            }
            Some(Outcome::Refused(why)) => {
                body = body.push(words::text_line(Kind::Warn, why.clone()))
            }
            Some(Outcome::Done(c)) => {
                let same = c.changed.is_empty() && c.added.is_empty() && c.removed.is_empty();
                body = body
                    .push(words::text_line(
                        if same { Kind::Ok } else { Kind::Warn },
                        summary(c),
                    ))
                    .push(Segmented::new(
                        [Tab::Changed, Tab::Added, Tab::Removed],
                        w.tab,
                        |t| msg(Event::Tab(t)),
                    ))
                    .push(
                        ListDetails::new(
                            self.ifc_compare_table(w, c),
                            self.ifc_compare_details(w, c),
                        )
                        .list_width(LIST)
                        .height(BODY),
                    );
            }
        }
        let mut foot = row![
            label::body("Yazar"),
            text_field("adınız", &w.author)
                .on_input(|t| msg(Event::Author(t)))
                .width(scaled(160.0)),
            words::secondary(WRITE, Some(msg(Event::Write))),
        ]
        .spacing(8)
        .align_y(Center);
        // What was said takes the room left, so Kapat stays in its place.
        foot = match &w.said {
            Some((kind, said)) => {
                foot.push(container(words::text_line(*kind, said.clone())).width(Fill))
            }
            None => foot.push(space::horizontal().width(Fill)),
        };
        let foot = foot.push(words::secondary(CLOSE, Some(msg(Event::Close))));
        Frame::new(TITLE)
            .push(label::caption(LEAD))
            .scroll(body)
            .footer(foot)
            .width(1160.0)
            .modal(msg(Event::Close))
    }

    fn ifc_compare_table<'a>(&'a self, w: &'a Window, c: &'a IfcCompared) -> Element<'a, Message> {
        let mark = |k: usize| {
            let on = w.marked.contains(&(w.tab, k));
            words::check(on, "", Some(msg(Event::Mark(k, !on))))
        };
        let mut t = match w.tab {
            Tab::Changed => Table::new([
                Col::new("").width(scaled(34.0)),
                Col::new("Sınıf").width(scaled(110.0)),
                Col::new("Ad").width(Fill),
                Col::new("Değişiklik").width(scaled(190.0)),
            ])
            .empty("Değişen nesne yok."),
            Tab::Added | Tab::Removed => Table::new([
                Col::new("").width(scaled(34.0)),
                Col::new("Sınıf").width(scaled(110.0)),
                Col::new("Ad").width(Fill),
                Col::new("GlobalId").width(scaled(190.0)),
            ])
            .empty(if w.tab == Tab::Added {
                "Eklenen nesne yok."
            } else {
                "Silinen nesne yok."
            }),
        };
        match w.tab {
            Tab::Changed => {
                for (k, x) in c.changed.iter().enumerate() {
                    let first = x
                        .changes
                        .first()
                        .map(change_words)
                        .map(|(l, _)| l)
                        .unwrap_or_default();
                    let what = if x.changes.len() > 1 {
                        format!("{first} +{}", x.changes.len() - 1)
                    } else {
                        first
                    };
                    t = t.push(
                        Row::new([
                            mark(k),
                            table::text(x.class.clone()).into(),
                            table::text(x.name.clone().unwrap_or_else(|| x.global_id.clone()))
                                .into(),
                            table::text(what).into(),
                        ])
                        .selected(w.chosen == Some(k))
                        .on_press(msg(Event::Choose(k))),
                    );
                }
            }
            Tab::Added | Tab::Removed => {
                let list = if w.tab == Tab::Added {
                    &c.added
                } else {
                    &c.removed
                };
                for (k, o) in list.iter().enumerate() {
                    t = t.push(
                        Row::new([
                            mark(k),
                            table::text(o.class.clone()).into(),
                            table::text(o.name.clone().unwrap_or_default()).into(),
                            table::text(o.global_id.clone()).into(),
                        ])
                        .selected(w.chosen == Some(k))
                        .on_press(msg(Event::Choose(k))),
                    );
                }
            }
        }
        t.into()
    }

    fn ifc_compare_details<'a>(
        &'a self,
        w: &'a Window,
        c: &'a IfcCompared,
    ) -> Element<'a, Message> {
        let Some(k) = w.chosen else {
            return label::caption("Bir satır seçin: nesnenin değişiklikleri burada okunur; kutuyu işaretlemek onu sorun \
                                   olarak yazılacaklara ekler.")
            .into();
        };
        let mut form = Form::new().label_width(scaled(170.0));
        match w.tab {
            Tab::Changed => {
                let Some(x) = c.changed.get(k) else {
                    return space::horizontal().into();
                };
                form = form
                    .section(format!(
                        "{} {}",
                        x.class,
                        x.name.clone().unwrap_or_default()
                    ))
                    .field("GlobalId", label::body(x.global_id.clone()));
                for ch in &x.changes {
                    let (l, v) = change_words(ch);
                    form = form.field(l, label::body(v));
                }
            }
            Tab::Added | Tab::Removed => {
                let list = if w.tab == Tab::Added {
                    &c.added
                } else {
                    &c.removed
                };
                let Some(o) = list.get(k) else {
                    return space::horizontal().into();
                };
                let side = if w.tab == Tab::Added {
                    "Yalnız yeni revizyonda"
                } else {
                    "Yalnız eski revizyonda"
                };
                form = form
                    .section(format!(
                        "{} {}",
                        o.class,
                        o.name.clone().unwrap_or_default()
                    ))
                    .field("GlobalId", label::body(o.global_id.clone()))
                    .wide(label::caption(side));
            }
        }
        scrollable(container(form).padding(iced::Padding::ZERO.right(14)))
            .height(Fill)
            .into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    fn ifc_compare_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.modules.arch().ifc_compare_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(COMPARE) => Some(msg(Event::Compare)),
            Control::Press(WRITE) => Some(msg(Event::Write)),
            Control::Fill("Eski revizyon", t) => Some(msg(Event::Path(Side::Older, t.to_owned()))),
            Control::Fill("Yeni revizyon", t) => Some(msg(Event::Path(Side::Newer, t.to_owned()))),
            Control::Fill("Tolerans", t) => Some(msg(Event::Tolerance(t.to_owned()))),
            Control::Fill("Yazar", t) => Some(msg(Event::Author(t.to_owned()))),
            Control::Row(n) if n >= 1 => Some(msg(Event::Choose(n - 1))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
