//! Altlık modeller (docs/adr/8101; plan/ifc-uyum.md IFC-09, B18; `arch.underlays`): other disciplines' IFC models shown
//! read-only under the drawing. The underlays on the left (DESIGN.md §7.9 Liste ve ayrıntı); on the right the chosen
//! one: its file (digest, size, schema, placement), what its read gave (the objects shown and the ones that have no
//! body, with why; how long it took and the memory it holds), its name, discipline and revision, whether it shows, and
//! the object chosen in the plan or in 3B görünüm with its properties as the file writes them, and Denetim: the
//! underlay set beside the drawing's own model (`cad.underlay.check`, underlay_check_view.rs, docs/adr/8102). Every
//! change is `cad.underlay.edit` (one undo step “Altlık model”); a file whose map conversion cannot place it is said
//! with its reason and placed at the origin only when asked.
//!
//! ```text
//! ┌ Altlık modeller ──────────────────────────────────────────────────────────────────────────────┐
//! │ Başka disiplinlerin IFC modelini çizimin altında salt okunur gösterir; model çizime kopyalanmaz. │
//! │ ┌ Ad                 Disiplin  Şema         Durum ┐│ statik.ifc                       [Kaldır]   │
//! │ │ statik.ifc         Statik    IFC4X3_ADD2  Gös.  ││ Dosya     /proje/statik.ifc                 │
//! │ ├────────────────────────────────────────────────┤│ Okuma     214 nesneden 212'si (%99) · 0,4 s  │
//! │ │ [Ekle…]                                         ││ Ad / Disiplin / Revizyon  [Uygula] [Yeniden oku]│
//! │ └────────────────────────────────────────────────┘│ Seçilen nesne  IfcBeam K-101 …  [Planda seç] │
//! │ Statik model eklendi.                                                                  [Kapat] │
//! └────────────────────────────────────────────────────────────────────────────────────────────────┘
//! ```

use crate::arch::ArchState;
use crate::arch_ext::*;
use std::path::PathBuf;

use iced::widget::{Column, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Length, Task};
use kentos_commands::{
    CommandResult, ModelUnderlay, UnderlayChange, UnderlayEdit, UnderlayPlacement,
};
use kentos_interaction::pick::PickPoint;
use kentos_native_application::{ExecutionContext, underlay_edit};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::typography::scaled;
use kentos_ui::widget::table::{self, Column as Col, Row, Table};
use kentos_ui::widget::{Dialog as Frame, Form, ListDetails, text_field};

use crate::app::{App, Message};
use crate::arch;
use crate::exchange::words::{self, Kind};
use crate::traces::Control;
use crate::underlays::State;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Altlık modeller";
const LEAD: &str = "Başka disiplinlerin IFC modelini çizimin altında salt okunur gösterir: planda etkin katın kesme \
                    yüksekliğinde kesilmiş, 3B görünümde gövdesiyle. Model çizime kopyalanmaz, dosyasından okunur.";
pub const ADD: &str = "Ekle…";
pub const ORIGIN: &str = "Başlangıç noktasına yerleştir";
pub const APPLY: &str = "Uygula";
pub const RELOAD: &str = "Yeniden oku";
pub const REMOVE: &str = "Kaldır";
pub const PICK: &str = "Planda seç";
const CLOSE: &str = "Kapat";
const BODY: f32 = 560.0;
const LIST: f32 = 380.0;
const LABELS: f32 = 110.0;

/// The chosen underlay's boxes as they are typed: its name, discipline and revision.
#[derive(Clone, Debug, Default)]
pub struct Typed {
    pub name: String,
    pub discipline: String,
    pub revision: String,
}

#[derive(Debug, Default)]
pub(crate) struct Window {
    pub chosen: Option<String>,
    pub typed: Typed,
    /// A file the map conversion could not place: its path and the reason, until placed at the origin or let go.
    pub unplaced: Option<(String, String)>,
    /// The object chosen in the plan or in 3B görünüm: the underlay's id and the object's index.
    pub object: Option<(String, usize)>,
    pub said: Option<(Kind, String)>,
    /// Denetim: its tolerance and its last answer.
    pub check: crate::underlay_check_view::Check,
}

#[derive(Clone, Debug)]
pub enum Event {
    Choose(String),
    Add,
    Picked(Option<PathBuf>),
    Origin,
    Forget,
    Name(String),
    Discipline(String),
    Revision(String),
    Apply,
    Shown(bool),
    Reload,
    Remove,
    Pick,
    Tolerance(String),
    Check,
    Checked(String, Box<crate::underlay_check_view::Outcome>),
    Tab(crate::underlay_check_view::Tab),
    Clash(usize),
    Close,
}

fn msg(e: Event) -> Message {
    Message::from(arch::Event::Underlays(e))
}

impl Typed {
    fn of(u: Option<&ModelUnderlay>) -> Self {
        match u {
            Some(u) => Typed {
                name: u.name.clone(),
                discipline: u.discipline.clone().unwrap_or_default(),
                revision: u.revision.clone().unwrap_or_default(),
            },
            None => Typed::default(),
        }
    }
}

/// A size as the list says it: 141 965 B as 139 KB, 2,1 MB.
fn size_words(bytes: u64) -> String {
    if bytes < 1024 * 1024 {
        format!("{} KB", bytes.div_ceil(1024))
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0)).replace('.', ",")
    }
}

/// The module's `App` methods of this file, as an extension trait (docs/adr/9199 §3.5).
pub(crate) trait UnderlaysWindowApp {
    fn underlay_list(&self) -> Vec<ModelUnderlay>;
    fn open_underlays(&mut self);
    fn underlays_said(&mut self, kind: Kind, words: impl Into<String>);
    fn underlay_edit(
        &mut self,
        change: UnderlayChange,
    ) -> Result<kentos_commands::UnderlayEdited, (String, String)>;
    fn underlay_add(&mut self, file: String, placement: UnderlayPlacement);
    fn underlays_event(&mut self, event: Event) -> Task<Message>;
    fn underlays_picked(&mut self, p: Option<kentos_interaction::Vec2>) -> bool;
    fn underlays_view(&self) -> Element<'_, Message>;
    fn underlay_status(&self, u: &ModelUnderlay) -> &'static str;
    fn underlays_table<'a>(&'a self, list: &[ModelUnderlay], w: &'a Window)
    -> Element<'a, Message>;
    fn underlay_details<'a>(
        &'a self,
        w: &'a Window,
        list: &[ModelUnderlay],
    ) -> Element<'a, Message>;
    fn underlay_object<'a>(
        &'a self,
        w: &'a Window,
        u: &ModelUnderlay,
        l: &crate::underlays::Loaded,
    ) -> Element<'a, Message>;
    fn underlays_control(&self, control: Control<'_>) -> Result<Option<Message>, String>;
}

impl UnderlaysWindowApp for App {
    fn underlay_list(&self) -> Vec<ModelUnderlay> {
        self.document
            .as_ref()
            .map(|d| d.model.underlays().to_vec())
            .unwrap_or_default()
    }

    /// Altlık modeller (`arch.underlays`): the drawing's underlays, the first chosen.
    fn open_underlays(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let list = self.underlay_list();
        let first = list.first();
        let (keep, check) = self
            .modules
            .arch_mut()
            .underlays_window
            .take()
            .map(|w| (w.object, w.check))
            .unwrap_or_default();
        self.modules.arch_mut().underlays_window = Some(Window {
            chosen: first.map(|u| u.id.clone()),
            typed: Typed::of(first),
            object: keep,
            check,
            ..Window::default()
        });
        self.dialog = Some(crate::arch::dialog(crate::arch::ArchDialogKind::Underlays));
    }

    fn underlays_said(&mut self, kind: Kind, words: impl Into<String>) {
        if let Some(w) = self.modules.arch_mut().underlays_window.as_mut() {
            w.said = Some((kind, words.into()));
        }
    }

    /// `cad.underlay.edit` on the open drawing; its refusal's code and words.
    fn underlay_edit(
        &mut self,
        change: UnderlayChange,
    ) -> Result<kentos_commands::UnderlayEdited, (String, String)> {
        let Some(doc) = self.document.as_mut() else {
            return Err((String::new(), "Açık çizim yok.".into()));
        };
        let input = UnderlayEdit {
            change,
            expected_revision: None,
        };
        match underlay_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err((error.code, error.message)),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {
                Err((String::new(), "Yazılmadı.".into()))
            }
        }
    }

    /// A file added `placement`; a map conversion that cannot place it kept with its reason for Başlangıç noktasına.
    fn underlay_add(&mut self, file: String, placement: UnderlayPlacement) {
        let change = UnderlayChange::Add {
            id: None,
            file: file.clone(),
            name: None,
            discipline: None,
            revision: None,
            placement: Some(placement),
        };
        match self.underlay_edit(change) {
            Ok(done) => {
                let added = done.underlays.iter().find(|u| u.id == done.id).cloned();
                if let Some(w) = self.modules.arch_mut().underlays_window.as_mut() {
                    w.unplaced = None;
                    w.typed = Typed::of(added.as_ref());
                    w.chosen = Some(done.id);
                    w.said = Some((
                        Kind::Ok,
                        format!(
                            "“{}” altlık olarak eklendi.",
                            added.map(|u| u.name).unwrap_or_default()
                        ),
                    ));
                }
            }
            Err((code, why)) if code == kentos_commands::codes::MODEL_UNDERLAY_NOT_PLACED => {
                if let Some(w) = self.modules.arch_mut().underlays_window.as_mut() {
                    w.unplaced = Some((file, why));
                    w.said = None;
                }
            }
            Err((_, why)) => self.underlays_said(Kind::Warn, why),
        }
    }

    fn underlays_event(&mut self, event: Event) -> Task<Message> {
        let list = self.underlay_list();
        let Some(w) = self.modules.arch_mut().underlays_window.as_mut() else {
            return Task::none();
        };
        let chosen = w.chosen.clone();
        match event {
            Event::Close => {
                self.modules.arch_mut().underlays_window = None;
                self.dialog = None;
            }
            Event::Choose(id) => {
                w.typed = Typed::of(list.iter().find(|u| u.id == id));
                w.chosen = Some(id);
                w.said = None;
            }
            Event::Add => {
                if let crate::app::Picker::File(path) = &self.picker {
                    return Task::done(msg(Event::Picked(Some(path.clone()))));
                }
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Altlık olarak gösterilecek IFC modeli")
                            .add_filter("IFC (.ifc)", &["ifc", "IFC"])
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |p| msg(Event::Picked(p)),
                );
            }
            Event::Picked(None) => {}
            Event::Picked(Some(path)) => {
                self.underlay_add(path.display().to_string(), UnderlayPlacement::Map)
            }
            Event::Origin => {
                if let Some((file, _)) = w.unplaced.clone() {
                    self.underlay_add(file, UnderlayPlacement::Origin);
                }
            }
            Event::Forget => w.unplaced = None,
            Event::Name(t) => w.typed.name = t,
            Event::Discipline(t) => w.typed.discipline = t,
            Event::Revision(t) => w.typed.revision = t,
            Event::Apply => {
                let Some(id) = chosen else {
                    return Task::none();
                };
                let Some(u) = list.iter().find(|u| u.id == id) else {
                    return Task::none();
                };
                let t = w.typed.clone();
                let differs = |now: &str, was: Option<&str>| {
                    (now.trim() != was.unwrap_or("")).then(|| now.to_owned())
                };
                let change = UnderlayChange::Set {
                    id,
                    name: (t.name.trim() != u.name).then(|| t.name.clone()),
                    discipline: differs(&t.discipline, u.discipline.as_deref()),
                    revision: differs(&t.revision, u.revision.as_deref()),
                    placement: None,
                    shown: None,
                };
                match self.underlay_edit(change) {
                    Ok(_) => self
                        .underlays_said(Kind::Ok, "Altlığın adı, disiplini ve revizyonu yazıldı."),
                    Err((code, _)) if code == kentos_commands::codes::NOTHING_TO_SET => {
                        self.underlays_said(Kind::Info, "Değişen bir şey yok.");
                    }
                    Err((_, why)) => self.underlays_said(Kind::Warn, why),
                }
            }
            Event::Shown(on) => {
                let Some(id) = chosen else {
                    return Task::none();
                };
                let change = UnderlayChange::Set {
                    id,
                    name: None,
                    discipline: None,
                    revision: None,
                    placement: None,
                    shown: Some(on),
                };
                if let Err((_, why)) = self.underlay_edit(change) {
                    self.underlays_said(Kind::Warn, why);
                }
            }
            Event::Reload => {
                let Some(id) = chosen else {
                    return Task::none();
                };
                match self.underlay_edit(UnderlayChange::Reload { id, file: None }) {
                    Ok(_) => self.underlays_said(Kind::Ok, "Dosya yeniden okundu."),
                    Err((_, why)) => self.underlays_said(Kind::Warn, why),
                }
            }
            Event::Remove => {
                let Some(id) = chosen else {
                    return Task::none();
                };
                match self.underlay_edit(UnderlayChange::Remove { id }) {
                    Ok(done) => {
                        let first = done.underlays.first().cloned();
                        if let Some(w) = self.modules.arch_mut().underlays_window.as_mut() {
                            w.chosen = first.as_ref().map(|u| u.id.clone());
                            w.typed = Typed::of(first.as_ref());
                            w.object = None;
                            w.said = Some((Kind::Ok, "Altlık kaldırıldı; dosyası yerinde.".into()));
                        }
                    }
                    Err((_, why)) => self.underlays_said(Kind::Warn, why),
                }
            }
            Event::Pick => {
                self.modules.arch_mut().underlay_picking = true;
                self.dialog = None;
                self.session.run(Box::new(PickPoint::new(
                    "Altlık nesnesi",
                    "Altlığın plandaki çizgisine tıklayın",
                )));
            }
            Event::Tolerance(t) => w.check.tolerance = t,
            Event::Check => return self.underlay_check_start(),
            Event::Checked(id, outcome) => self.underlay_checked(id, *outcome),
            Event::Tab(t) => w.check.tab = t,
            Event::Clash(k) => self.underlay_clash_chosen(k),
        }
        Task::none()
    }

    /// The point picked for an underlay object (none: given up; the window comes back); false when it was another's.
    fn underlays_picked(&mut self, p: Option<kentos_interaction::Vec2>) -> bool {
        if !std::mem::take(&mut self.modules.arch_mut().underlay_picking) {
            return false;
        }
        if let Some(p) = p {
            // Eight pixels round the click, in the drawing's units.
            let reach = 8.0 / self.viewport.camera.scale.max(f64::MIN_POSITIVE);
            let found = self.underlay_object_at([p.x, p.y], reach);
            if let Some(w) = self.modules.arch_mut().underlays_window.as_mut() {
                match found {
                    Some((id, object)) => {
                        w.chosen = Some(id.clone());
                        w.object = Some((id, object));
                        w.said = None;
                    }
                    None => {
                        w.said = Some((Kind::Info, "Tıklanan yerde altlık çizgisi yok.".into()))
                    }
                }
            }
        }
        self.dialog = Some(crate::arch::dialog(crate::arch::ArchDialogKind::Underlays));
        true
    }

    fn underlays_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.modules.arch().underlays_window else {
            return space::horizontal().into();
        };
        let list = self.underlay_list();
        let body: Element<'_, Message> = ListDetails::new(
            self.underlays_table(&list, w),
            self.underlay_details(w, &list),
        )
        .list_foot(words::secondary(ADD, Some(msg(Event::Add))))
        .list_width(LIST)
        .height(BODY)
        .into();
        let mut foot = row![].spacing(8).align_y(Center);
        if let Some((kind, said)) = &w.said {
            foot =
                foot.push(container(words::text_line(*kind, said.clone())).width(Length::Shrink));
        }
        let foot = foot
            .push(space::horizontal().width(Fill))
            .push(words::secondary(CLOSE, Some(msg(Event::Close))));
        let mut frame = Frame::new(TITLE).push(label::caption(LEAD));
        if let Some((file, why)) = &w.unplaced {
            frame = frame.push(
                container(
                    column![
                        words::text_line(Kind::Warn, why.clone()),
                        row![
                            words::secondary(ORIGIN, Some(msg(Event::Origin))),
                            words::ghost("Vazgeç", Some(msg(Event::Forget))),
                            label::caption(file.clone())
                        ]
                        .spacing(8)
                        .align_y(Center)
                    ]
                    .spacing(6),
                )
                .padding([8, 10])
                .width(Fill)
                .style(style::container::field_box),
            );
        }
        frame
            .scroll(body)
            .footer(foot)
            .width(1160.0)
            .modal(msg(Event::Close))
    }

    fn underlay_status(&self, u: &ModelUnderlay) -> &'static str {
        match self
            .modules
            .arch()
            .underlays
            .states
            .get(&u.id)
            .map(|s| &s.1)
        {
            _ if !u.shown => "Gizli",
            Some(State::Ready(_)) => "Gösteriliyor",
            Some(State::Reading) | None => "Okunuyor…",
            Some(State::Failed(_)) => "Okunamadı",
        }
    }

    fn underlays_table<'a>(
        &'a self,
        list: &[ModelUnderlay],
        w: &'a Window,
    ) -> Element<'a, Message> {
        let mut t = Table::new([
            Col::new("Ad").width(Fill),
            Col::new("Disiplin").width(scaled(80.0)),
            Col::new("Şema").width(scaled(96.0)),
            Col::new("Durum").width(scaled(92.0)),
        ])
        .empty("Altlık yok.");
        for u in list {
            t = t.push(
                Row::new([
                    table::text(u.name.clone()).into(),
                    table::text(u.discipline.clone().unwrap_or_else(|| "—".into())).into(),
                    label::body(u.schema.clone()).into(),
                    label::body(self.underlay_status(u)).into(),
                ])
                .selected(w.chosen.as_deref() == Some(u.id.as_str()))
                .on_press(msg(Event::Choose(u.id.clone()))),
            );
        }
        t.into()
    }

    fn underlay_details<'a>(
        &'a self,
        w: &'a Window,
        list: &[ModelUnderlay],
    ) -> Element<'a, Message> {
        let Some(u) = w
            .chosen
            .as_ref()
            .and_then(|id| list.iter().find(|u| &u.id == id))
            .cloned()
        else {
            return steps();
        };
        let head = row![
            label::strong(u.name.clone()),
            space::horizontal().width(Fill),
            iced::widget::button(label::body(REMOVE))
                .on_press(msg(Event::Remove))
                .padding(kentos_ui::theme::metrics::action())
                .style(style::button::danger_outline)
        ]
        .spacing(8)
        .align_y(Center);
        let state = self
            .modules
            .arch()
            .underlays
            .states
            .get(&u.id)
            .map(|s| s.1.clone());
        let mut facts = Form::new()
            .label_width(scaled(LABELS))
            .field("Dosya", label::body(u.file.clone()))
            .field(
                "Revizyonu",
                label::body(format!(
                    "SHA-256 {}… · {} · {}",
                    &u.sha256[..12],
                    size_words(u.bytes),
                    u.schema
                )),
            )
            .field("Yerleşim", label::body(u.placement.label()));
        facts = match &state {
            Some(State::Ready(l)) => {
                let shape = l.with_shape().max(1);
                let share = l.bodies.len() * 100 / shape;
                facts
                    .field(
                        "Okuma",
                        label::body(format!(
                            "Gösterilen {} / {} nesne (%{share}) · gösterilemeyen {}",
                            l.bodies.len(),
                            l.with_shape(),
                            l.not_shown.len()
                        )),
                    )
                    .field(
                        "Kutusu",
                        label::body(l.bounds.map_or_else(
                            || "—".to_owned(),
                            |b| {
                                format!(
                                    "{:.2} × {:.2} × {:.2} m",
                                    b[3] - b[0],
                                    b[4] - b[1],
                                    b[5] - b[2]
                                )
                                .replace('.', ",")
                            },
                        )),
                    )
                    .field(
                        "Süre ve bellek",
                        label::body(
                            format!(
                                "{:.2} s · yaklaşık {}",
                                l.took.as_secs_f64(),
                                size_words(l.memory() as u64)
                            )
                            .replace('.', ","),
                        ),
                    )
            }
            Some(State::Failed(why)) => {
                facts.field("Okuma", words::text_line(Kind::Warn, why.clone()))
            }
            _ => facts.field("Okuma", label::body("Okunuyor…")),
        };
        let form = Form::new()
            .label_width(scaled(LABELS))
            .section("Altlık")
            .field(
                "Ad",
                text_field("Dosyanın adı", &w.typed.name)
                    .on_input(|t| msg(Event::Name(t)))
                    .width(scaled(300.0)),
            )
            .field(
                "Disiplin",
                text_field("Statik, Mekanik, Elektrik…", &w.typed.discipline)
                    .on_input(|t| msg(Event::Discipline(t)))
                    .width(scaled(300.0)),
            )
            .field(
                "Revizyon",
                text_field("—", &w.typed.revision)
                    .on_input(|t| msg(Event::Revision(t)))
                    .width(scaled(140.0)),
            )
            .row(words::check(
                u.shown,
                "Çizimin altında göster",
                Some(msg(Event::Shown(!u.shown))),
            ))
            .row(
                row![
                    words::primary(APPLY, Some(msg(Event::Apply))),
                    words::secondary(RELOAD, Some(msg(Event::Reload)))
                ]
                .spacing(8),
            );
        // What the file holds first (the object chosen, what has no body), then the underlay's own boxes.
        let mut out = Column::new()
            .spacing(14)
            .push(head)
            .push(facts)
            .push(self.underlay_check_section(w, &u));
        if let Some(State::Ready(l)) = &state {
            out = out.push(self.underlay_object(w, &u, l));
            if !l.not_shown.is_empty() {
                let mut t = Table::new([
                    Col::new("Gösterilemeyen nesne").width(scaled(220.0)),
                    Col::new("Neden").width(Fill),
                ]);
                for (owner, why) in &l.not_shown {
                    t = t.push(Row::new([
                        table::text(l.object_words(*owner)).into(),
                        table::text(why.clone()).into(),
                    ]));
                }
                let rows = l.not_shown.len().clamp(1, 6) as f32 + 1.0;
                out = out.push(t.height(rows * table::row_height() + 2.0));
            }
        }
        let out = out.push(form);
        scrollable(container(out).padding(iced::Padding::ZERO.right(14)))
            .height(Fill)
            .into()
    }

    /// The object chosen in the plan or in 3B görünüm: its details as the file writes them.
    fn underlay_object<'a>(
        &'a self,
        w: &'a Window,
        u: &ModelUnderlay,
        l: &crate::underlays::Loaded,
    ) -> Element<'a, Message> {
        let mut form = Form::new()
            .label_width(scaled(LABELS))
            .section("Seçilen nesne")
            .row(words::secondary(PICK, Some(msg(Event::Pick))));
        match w.object.as_ref().filter(|(id, _)| *id == u.id) {
            None => {
                form = form.wide(label::caption(
                    "Planda altlığın bir çizgisine ya da 3B görünümde gövdesine tıklayın.",
                ))
            }
            Some((_, object)) => {
                // The file's own words: its groups (the object, its place, its sets) as the viewer shows them.
                match l.model.details(&l.bytes, *object) {
                    Ok(d) => {
                        for g in d.groups {
                            form = form.section(g.title);
                            for line in g.lines.into_iter().take(40) {
                                let value = match &line.unit {
                                    Some(unit) => format!("{} {unit}", line.value),
                                    None => line.value.clone(),
                                };
                                form = form.field(line.name, label::body(value));
                            }
                        }
                    }
                    Err(e) => {
                        let o = l.model.objects.get(*object);
                        form = form
                            .field(
                                "Sınıf",
                                label::body(o.map(|o| o.class.clone()).unwrap_or_default()),
                            )
                            .field(
                                "GlobalId",
                                label::body(o.map(|o| o.global_id.clone()).unwrap_or_default()),
                            )
                            .wide(words::text_line(Kind::Warn, e.to_string()));
                    }
                }
            }
        }
        form.into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    fn underlays_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.modules.arch().underlays_window.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(ORIGIN) => Some(msg(Event::Origin)),
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(RELOAD) => Some(msg(Event::Reload)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(PICK) => Some(msg(Event::Pick)),
            Control::Press(crate::underlay_check_view::CHECK) => Some(msg(Event::Check)),
            Control::Fill("Tolerans", t) => Some(msg(Event::Tolerance(t.to_owned()))),
            Control::Row(n) if n >= 1 => Some(msg(Event::Clash(n - 1))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// An empty list's details: what to do, step by step (DESIGN.md §7.5).
fn steps<'a>() -> Element<'a, Message> {
    let list = [
        "Soldaki Ekle…'ye basın ve statik, mekanik ya da elektrik modelinin IFC dosyasını seçin.",
        "Dosyanın harita dönüşümü projenin sistemindeyse yerine oturur; değilse nedeni söylenir.",
        "Planda etkin katın kesme yüksekliğinde kesilmiş, 3B görünümde gövdesiyle görünür.",
    ];
    let mut out = Column::new()
        .spacing(10)
        .push(label::strong("Altlık model yok"))
        .push(label::caption(
            "Model çizime kopyalanmaz: dosyasından salt okunur gösterilir.",
        ));
    for (k, s) in list.iter().enumerate() {
        out = out.push(
            row![
                container(label::strong((k + 1).to_string())).width(scaled(18.0)),
                container(label::body(*s)).width(Fill)
            ]
            .spacing(6),
        );
    }
    out.into()
}
