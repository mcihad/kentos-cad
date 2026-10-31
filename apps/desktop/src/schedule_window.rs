//! Çizelge ayarları and Dosyaya yaz (docs/adr/0733 §5, §7): a placed schedule's columns, filter (level, phase, type,
//! form), first sort key with its heading and subtotal, grouping and total, changed in one `cad.schedules.edit` step
//! with the plan's row count as a preview; and its cells written as CSV, XLSX or ODS through `file.export.table` (its
//! headings and subtotals, its numbers as shown or exact). Opened from Öznitelikler's Çizelge row. A units schedule's
//! filter is its groups' level, building, wing and kind (docs/adr/0795); its own fields are written to the program.

use std::path::PathBuf;

use iced::widget::{Column, button, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    ArchPhase, CommandResult, Entity, ExportTable, OpeningForm, ScheduleChange, ScheduleColumn,
    ScheduleField, ScheduleFilter, ScheduleOf, ScheduleOperation, ScheduleSort, SchedulesEdit,
    TableNumbers, TableSource, UnitKind,
};
use kentos_interaction::Level;
use kentos_native_application::ExecutionContext;
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message, Picker};
use crate::exchange::words;

/// The windows' titles, which a trace names them by.
pub const TITLE: &str = "Çizelge ayarları";
pub const EXPORT_TITLE: &str = "Çizelgeyi dosyaya yaz";
const PHASES: [ArchPhase; 4] = [
    ArchPhase::Existing,
    ArchPhase::Demolish,
    ArchPhase::New,
    ArchPhase::Temporary,
];
const FORMATS: [(&str, &str); 3] = [
    ("xlsx", "Excel (.xlsx)"),
    ("ods", "OpenDocument (.ods)"),
    ("csv", "CSV (.csv)"),
];

#[derive(Clone, Debug)]
pub struct Window {
    pub uid: String,
    pub of: ScheduleOf,
    pub columns: Vec<ScheduleColumn>,
    pub filter: ScheduleFilter,
    /// The first sort key; more come from Python and MCP and are kept.
    pub sort: Vec<ScheduleSort>,
    pub group: bool,
    pub total: bool,
    /// The plan's data rows, or why it is refused.
    pub preview: Result<u32, String>,
    /// Dosyaya yaz's window rather than the settings.
    pub export: bool,
    pub format: &'static str,
    pub groups: bool,
    pub numbers: TableNumbers,
}

#[derive(Clone, Debug)]
pub enum Event {
    Open(String),
    Export(String),
    Column(ScheduleField, bool),
    Up(usize),
    Down(usize),
    Level(String),
    Phase(ArchPhase),
    Type(String),
    Form(OpeningForm),
    /// A units schedule's filter (docs/adr/0795): a building's or a wing's id, a kind.
    Building(String),
    Wing(String),
    Kind(UnitKind),
    SortColumn(usize),
    Descending(bool),
    Header(bool),
    Footer(bool),
    Group(bool),
    Total(bool),
    Apply,
    Cancel,
    Format(usize),
    Groups(bool),
    Numbers(bool),
    Write,
    Written(Option<PathBuf>),
}

fn msg(e: Event) -> Message {
    Message::Schedule(e)
}

/// A filter list with `v` flipped: added, or taken away (none when it empties).
fn flip<T: PartialEq + Clone>(list: &mut Option<Vec<T>>, v: T) {
    let mut l = list.take().unwrap_or_default();
    match l.iter().position(|x| *x == v) {
        Some(i) => {
            l.remove(i);
        }
        None => l.push(v),
    }
    *list = (!l.is_empty()).then_some(l);
}

impl App {
    fn schedule_window_of(&self, uid: &str, export: bool) -> Option<Window> {
        let doc = self.document.as_ref()?;
        let slot = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.model.slot_of(u))?;
        let Some(Entity::Table(t)) = doc.model.get(slot) else {
            return None;
        };
        let Some(TableSource::Schedule {
            of,
            columns,
            filter,
            sort,
            group,
            total,
            ..
        }) = &t.source
        else {
            return None;
        };
        let mut w = Window {
            uid: uid.to_owned(),
            of: *of,
            columns: columns.clone(),
            filter: filter.clone().unwrap_or_default(),
            sort: sort.clone(),
            group: *group,
            total: *total,
            preview: Ok(0),
            export,
            format: "xlsx",
            groups: true,
            numbers: TableNumbers::Shown,
        };
        w.preview = self.schedule_preview(&w);
        Some(w)
    }

    fn schedule_input(w: &Window) -> SchedulesEdit {
        SchedulesEdit {
            uid: w.uid.clone(),
            operation: ScheduleOperation::Set(Box::new(ScheduleChange {
                columns: Some(w.columns.clone()),
                filter: Some((!w.filter.is_empty()).then(|| w.filter.clone())),
                sort: Some(w.sort.clone()),
                group: Some(w.group),
                total: Some(w.total),
            })),
            expected_revision: None,
        }
    }

    /// How many data rows the change writes, from the command's plan.
    fn schedule_preview(&self, w: &Window) -> Result<u32, String> {
        let doc = self.document.as_ref().ok_or_else(String::new)?;
        match kentos_native_application::schedules_edit::plan_in(
            &doc.model,
            &Self::schedule_input(w),
        ) {
            CommandResult::Completed { output, .. } => match &output.table {
                Entity::Table(t) => Ok(match &t.source {
                    Some(TableSource::Schedule { members, .. }) => {
                        members.iter().filter(|m| !m.is_empty()).count() as u32
                    }
                    _ => 0,
                }),
                _ => Ok(0),
            },
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Ok(0),
        }
    }

    pub(crate) fn schedule_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open(ref uid) | Event::Export(ref uid) => {
                let export = matches!(event, Event::Export(_));
                let uid = uid.clone();
                self.schedule = self.schedule_window_of(&uid, export);
                if self.schedule.is_some() {
                    self.dialog = Some(if export {
                        Dialog::ScheduleExport
                    } else {
                        Dialog::Schedule
                    });
                }
                return Task::none();
            }
            Event::Cancel => {
                self.schedule = None;
                self.dialog = None;
                return Task::none();
            }
            Event::Apply => return self.schedule_apply(),
            Event::Write => return self.schedule_write_ask(),
            Event::Written(path) => {
                self.schedule_written(path);
                return Task::none();
            }
            _ => {}
        }
        let Some(w) = self.schedule.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Column(field, on) => {
                if on {
                    if !w.columns.iter().any(|c| c.field == field) {
                        w.columns.push(ScheduleColumn {
                            field,
                            title: None,
                            attr: None,
                        });
                    }
                } else if w.columns.len() > 1
                    && let Some(i) = w
                        .columns
                        .iter()
                        .position(|c| c.field == field && c.field != ScheduleField::Attr)
                {
                    w.columns.remove(i);
                    // A sort key past the columns goes; a later one moves with its column.
                    w.sort.retain(|k| k.column as usize != i);
                    for k in &mut w.sort {
                        if k.column as usize > i {
                            k.column -= 1;
                        }
                    }
                }
            }
            Event::Up(i) | Event::Down(i) => {
                let j = if matches!(event, Event::Up(_)) {
                    i.checked_sub(1)
                } else {
                    Some(i + 1)
                };
                if let Some(j) = j.filter(|j| *j < w.columns.len()) {
                    w.columns.swap(i, j);
                    for k in &mut w.sort {
                        if k.column as usize == i {
                            k.column = j as u32;
                        } else if k.column as usize == j {
                            k.column = i as u32;
                        }
                    }
                }
            }
            Event::Level(id) => flip(&mut w.filter.levels, id),
            Event::Phase(p) => flip(&mut w.filter.phases, p),
            Event::Type(t) => flip(&mut w.filter.types, t),
            Event::Form(f) => flip(&mut w.filter.forms, f),
            Event::Building(b) => flip(&mut w.filter.buildings, b),
            Event::Wing(x) => flip(&mut w.filter.wings, x),
            Event::Kind(k) => flip(&mut w.filter.kinds, k),
            Event::SortColumn(i) => {
                if i == 0 {
                    w.sort.clear();
                } else {
                    let old = w.sort.first().copied().unwrap_or_default();
                    let key = ScheduleSort {
                        column: (i - 1) as u32,
                        ..old
                    };
                    if w.sort.is_empty() {
                        w.sort.push(key);
                    } else {
                        w.sort[0] = key;
                    }
                }
            }
            Event::Descending(on) | Event::Header(on) | Event::Footer(on) => {
                if w.sort.is_empty() {
                    w.sort.push(ScheduleSort::default());
                }
                let k = &mut w.sort[0];
                match event {
                    Event::Descending(_) => k.descending = on,
                    Event::Header(_) => k.header = on,
                    _ => k.footer = on,
                }
            }
            Event::Group(on) => w.group = on,
            Event::Total(on) => w.total = on,
            Event::Format(i) => w.format = FORMATS[i.min(FORMATS.len() - 1)].0,
            Event::Groups(on) => w.groups = on,
            Event::Numbers(exact) => {
                w.numbers = if exact {
                    TableNumbers::Exact
                } else {
                    TableNumbers::Shown
                }
            }
            _ => {}
        }
        if let Some(w) = self.schedule.clone() {
            let preview = self.schedule_preview(&w);
            if let Some(s) = self.schedule.as_mut() {
                s.preview = preview;
            }
        }
        Task::none()
    }

    /// Tamam: the change as one `cad.schedules.edit` step.
    fn schedule_apply(&mut self) -> Task<Message> {
        let Some(w) = self.schedule.clone() else {
            return Task::none();
        };
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        match kentos_native_application::schedules_edit::execute(
            &mut ExecutionContext::new(&mut doc.model),
            Self::schedule_input(&w),
        ) {
            CommandResult::Completed { output, .. } => {
                let rows = match &output.table {
                    Entity::Table(t) => match &t.source {
                        Some(TableSource::Schedule { members, .. }) => {
                            members.iter().filter(|m| !m.is_empty()).count()
                        }
                        _ => 0,
                    },
                    _ => 0,
                };
                self.announce(
                    Level::Success,
                    format!("Çizelge değişti: {rows} satır. Geri al ile dönülür."),
                );
                self.schedule = None;
                self.dialog = None;
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                if let Some(s) = self.schedule.as_mut() {
                    s.preview = Err(error.message);
                }
            }
            _ => {}
        }
        Task::none()
    }

    /// Yaz…: the path asked (a trace gives it through `Picker::File`).
    fn schedule_write_ask(&mut self) -> Task<Message> {
        let Some(w) = &self.schedule else {
            return Task::none();
        };
        if let Picker::File(path) = &self.picker {
            return Task::done(msg(Event::Written(Some(path.clone()))));
        }
        let ext = w.format;
        let name = format!("{}.{ext}", w.of.label());
        Task::perform(
            async move {
                rfd::AsyncFileDialog::new()
                    .set_title(EXPORT_TITLE)
                    .add_filter(ext.to_uppercase(), &[ext])
                    .set_file_name(name)
                    .save_file()
                    .await
                    .map(|f| {
                        let mut p = f.path().to_path_buf();
                        if p.extension().is_none() {
                            p.set_extension(ext);
                        }
                        p
                    })
            },
            |p| msg(Event::Written(p)),
        )
    }

    fn schedule_written(&mut self, path: Option<PathBuf>) {
        let (Some(path), Some(w), Some(doc)) = (path, self.schedule.clone(), &self.document) else {
            return;
        };
        let input = ExportTable {
            path: path.display().to_string(),
            uid: w.uid.clone(),
            groups: Some(w.groups),
            numbers: w.numbers,
            overwrite: true,
        };
        match kentos_headless::file_commands::export_table(&doc.model, &input) {
            Ok((out, _)) => {
                self.announce(
                    Level::Success,
                    format!(
                        "{} yazıldı: {} satır, {} sütun.",
                        out.path, out.rows, out.columns
                    ),
                );
                for note in out.notes {
                    self.say(Level::Info, note);
                }
                self.schedule = None;
                self.dialog = None;
            }
            Err(e) => self.announce(Level::Error, format!("Çizelge yazılamadı. {}", e.message)),
        }
    }

    /// A units schedule's filter (docs/adr/0795): the program's buildings, their wings and the kinds, each on or off.
    fn schedule_unit_filter<'a>(&self, w: &Window, mut filter: Column<'a, Message>) -> Column<'a, Message> {
        let program = self.document.as_ref().map(|d| d.model.program()).unwrap_or_default();
        if !program.buildings.is_empty() {
            filter = filter.push(label::caption("Bina (hiçbiri: hepsi)"));
            for b in &program.buildings {
                let on = w.filter.buildings.as_ref().is_some_and(|l| l.contains(&b.id));
                filter = filter.push(words::check(on, b.name.clone(), Some(msg(Event::Building(b.id.clone())))));
            }
        }
        let wings: Vec<(String, String)> = program
            .buildings
            .iter()
            .flat_map(|b| b.wings.iter().map(move |x| (x.id.clone(), format!("{} · {}", b.name, x.name))))
            .collect();
        if !wings.is_empty() {
            filter = filter.push(label::caption("Kanat (hiçbiri: hepsi)"));
            for (id, name) in wings {
                let on = w.filter.wings.as_ref().is_some_and(|l| l.contains(&id));
                filter = filter.push(words::check(on, name, Some(msg(Event::Wing(id)))));
            }
        }
        filter = filter.push(label::caption("Bölüm türü (hiçbiri: hepsi)"));
        for k in UnitKind::ALL {
            let on = w.filter.kinds.as_ref().is_some_and(|l| l.contains(&k));
            filter = filter.push(words::check(on, k.label(), Some(msg(Event::Kind(k)))));
        }
        filter
    }

    /// The types the drawing's objects of `of` have, in the natural order (Tür süzgeci's choices).
    fn schedule_types(&self, of: ScheduleOf) -> Vec<String> {
        let Some(doc) = self.document.as_ref() else {
            return Vec::new();
        };
        let place = if of == ScheduleOf::Spaces {
            "usage"
        } else {
            "type"
        };
        let mut out: Vec<String> = doc
            .model
            .entities()
            .filter(|e| e.kind() == of.kind())
            .filter_map(|e| {
                doc.model
                    .arch_value(e, place, None)
                    .map(|v| doc.model.arch_value_text(&v, false))
            })
            .filter(|t| !t.is_empty())
            .collect();
        out.sort_by(|a, b| kentos_geometry_core::text::natural::natural_cmp(a, b));
        out.dedup();
        out
    }

    pub(crate) fn schedule_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.schedule else {
            return label::body("").into();
        };
        if w.export {
            return self.schedule_export_view(w);
        }
        let section = |title: &'static str| label::strong(title);
        // Sütunlar: the kind's fields in order, each on or off; the chosen ones in the schedule's order, movable.
        let mut fields = Column::new().spacing(4);
        for f in ScheduleField::ALL
            .iter()
            .filter(|f| f.reads(w.of) && **f != ScheduleField::Attr)
        {
            let on = w.columns.iter().any(|c| c.field == *f);
            let kind = if f.edits_in(w.of) && w.of == ScheduleOf::Units {
                "programa yazılır"
            } else if f.edits_in(w.of) {
                "nesneye yazılır"
            } else {
                "hesaplanır"
            };
            fields = fields.push(words::check(
                on,
                format!(
                    "{} · {kind}",
                    if w.of == ScheduleOf::Units && *f == ScheduleField::Count { "Mahal sayısı" } else { f.title() }
                ),
                Some(msg(Event::Column(*f, !on))),
            ));
        }
        let mut order = Column::new().spacing(4);
        for (i, c) in w.columns.iter().enumerate() {
            let up = button(label::body("↑"))
                .style(style::button::secondary)
                .padding(kentos_ui::theme::metrics::action_small())
                .on_press_maybe((i > 0).then(|| msg(Event::Up(i))));
            let down = button(label::body("↓"))
                .style(style::button::secondary)
                .padding(kentos_ui::theme::metrics::action_small())
                .on_press_maybe((i + 1 < w.columns.len()).then(|| msg(Event::Down(i))));
            order = order.push(
                row![
                    label::body(format!("{}. {}", i + 1, c.heading())).width(Fill),
                    up,
                    down
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        // Süzgeç.
        let mut filter = Column::new().spacing(4);
        let levels: Vec<(String, String)> = self
            .document
            .as_ref()
            .and_then(|d| d.model.settings().arch.as_ref())
            .map(|a| {
                a.levels
                    .iter()
                    .map(|l| (l.id.clone(), l.name.clone()))
                    .collect()
            })
            .unwrap_or_default();
        if !levels.is_empty() {
            filter = filter.push(label::caption("Kat (hiçbiri: hepsi)"));
            for (id, name) in levels {
                let on = w.filter.levels.as_ref().is_some_and(|l| l.contains(&id));
                filter = filter.push(words::check(on, name, Some(msg(Event::Level(id.clone())))));
            }
        }
        if w.of == ScheduleOf::Units {
            filter = self.schedule_unit_filter(w, filter);
        } else {
            filter = filter.push(label::caption("Evre (hiçbiri: hepsi)"));
            for p in PHASES {
                let on = w.filter.phases.as_ref().is_some_and(|l| l.contains(&p));
                filter = filter.push(words::check(on, p.label(), Some(msg(Event::Phase(p)))));
            }
        }
        let types = if w.of == ScheduleOf::Units { Vec::new() } else { self.schedule_types(w.of) };
        if !types.is_empty() {
            filter = filter.push(label::caption(if w.of == ScheduleOf::Spaces {
                "Kullanım (hiçbiri: hepsi)"
            } else {
                "Tür (hiçbiri: hepsi)"
            }));
            for t in types {
                let on = w.filter.types.as_ref().is_some_and(|l| l.contains(&t));
                filter = filter.push(words::check(on, t.clone(), Some(msg(Event::Type(t)))));
            }
        }
        if w.of == ScheduleOf::Openings {
            filter = filter.push(label::caption("Biçim (hiçbiri: hepsi)"));
            for f in [OpeningForm::Door, OpeningForm::Window, OpeningForm::Opening] {
                let on = w.filter.forms.as_ref().is_some_and(|l| l.contains(&f));
                filter = filter.push(words::check(on, f.label(), Some(msg(Event::Form(f)))));
            }
        }
        // Sıralama: the first key, its heading and subtotal.
        let mut choices = vec![Choice::new("Sıralama yok")];
        choices.extend(w.columns.iter().map(|c| Choice::new(c.heading())));
        let key = w.sort.first().copied();
        let picked = key.map_or(0, |k| k.column as usize + 1);
        let sort = Select::new(choices, Some(picked), |i| msg(Event::SortColumn(i)));
        let k = key.unwrap_or_default();
        let mut sorting = Column::new().spacing(4).push(container(sort).width(260.0));
        if key.is_some() {
            sorting = sorting
                .push(words::check(
                    k.descending,
                    "Azalan",
                    Some(msg(Event::Descending(!k.descending))),
                ))
                .push(words::check(
                    k.header,
                    "Grup başlığı",
                    Some(msg(Event::Header(!k.header))),
                ))
                .push(words::check(
                    k.footer,
                    "Ara toplam",
                    Some(msg(Event::Footer(!k.footer))),
                ));
        }
        sorting = sorting
            .push(words::check(
                w.group,
                "Aynı satırları birleştir",
                Some(msg(Event::Group(!w.group))),
            ))
            .push(words::check(
                w.total,
                "Genel toplam",
                Some(msg(Event::Total(!w.total))),
            ));
        let preview = match &w.preview {
            Ok(n) => words::summary(vec![words::text_line(
                words::Kind::Ok,
                format!("{n} veri satırı yazılır."),
            )]),
            Err(why) => words::summary(vec![words::text_line(words::Kind::Warn, why.clone())]),
        };
        let columns = row![
            Column::new()
                .spacing(8)
                .push(section("Sütunlar"))
                .push(fields)
                .push(section("Sıra"))
                .push(order)
                .width(Fill),
            Column::new()
                .spacing(8)
                .push(section("Süzgeç"))
                .push(filter)
                .width(Fill),
            Column::new()
                .spacing(8)
                .push(section("Sıralama ve gruplar"))
                .push(sorting)
                .width(Fill),
        ]
        .spacing(16);
        let body = Column::new()
            .spacing(12)
            .push(label::caption(if w.of == ScheduleOf::Units {
                format!(
                    "{}. Programa yazılan sütunun hücresi Tabloyu düzenle'de satırın bölümlerine, bina programına yazılır; hesaplanan sütun programdan ve mahallerden okunur.",
                    w.of.label()
                )
            } else {
                format!(
                    "{}. Nesneye yazılan sütunun hücresi Tabloyu düzenle'de satırın nesnelerine yazılır; hesaplanan sütun çizimden okunur.",
                    w.of.label()
                )
            }))
            .push(iced::widget::scrollable(columns).height(420.0))
            .push(preview);
        Frame::new(TITLE)
            .push(body)
            .action(words::secondary("Vazgeç", Some(msg(Event::Cancel))))
            .action(words::primary(
                "Tamam",
                w.preview.is_ok().then(|| msg(Event::Apply)),
            ))
            .width(900.0)
            .modal(msg(Event::Cancel))
    }

    fn schedule_export_view<'a>(&'a self, w: &'a Window) -> Element<'a, Message> {
        let picked = FORMATS
            .iter()
            .position(|(f, _)| *f == w.format)
            .unwrap_or(0);
        let format = Select::new(
            FORMATS
                .iter()
                .map(|(_, l)| Choice::new(*l))
                .collect::<Vec<_>>(),
            Some(picked),
            |i| msg(Event::Format(i)),
        );
        let exact = w.numbers == TableNumbers::Exact;
        let body = Column::new()
            .spacing(10)
            .push(row![label::body("Biçim").width(120.0), container(format).width(260.0)].spacing(10).align_y(Center))
            .push(words::check(w.groups, "Grup başlıkları, ara toplamlar ve genel toplam da", Some(msg(Event::Groups(!w.groups)))))
            .push(words::check(exact, "Hesaplanan sayılar tam (15 anlamlı basamak)", Some(msg(Event::Numbers(!exact)))))
            .push(label::caption(if exact {
                "Hesaplanan sayılar çizimden yeniden okunur, 15 anlamlı basamağa yuvarlanır; hesap tablosunun toplamı çizelgeninkiyle aynı olur."
            } else {
                "Sayılar çizelgede görüldüğü gibi, projenin basamağıyla yazılır. Nesneye yazılan sütunlar (numara, ad, kaplama) hiçbir zaman yuvarlanmaz."
            }));
        Frame::new(EXPORT_TITLE)
            .push(body)
            .action(words::secondary("Vazgeç", Some(msg(Event::Cancel))))
            .action(words::primary("Yaz…", Some(msg(Event::Write))))
            .width(560.0)
            .modal(msg(Event::Cancel))
    }
}
