//! Metraj (docs/adr/0342 §4, §5; `arch.quantities`): the drawing's spaces, walls and openings measured by
//! `cad.spaces.quantities`, a level's or all, each row with the rule its numbers come from, and the totals. Panoya
//! kopyala copies the table shown as tab-separated text; Dışa aktar ▾ writes the three tables as a workbook's sheets
//! (table_write.rs). Mahalleri birleştir (`arch.spaceMerge`) is here too: the spaces selected, made one by
//! `cad.spaces.edit`.

use iced::widget::{Column, Row, column, container, row, scrollable, text};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CellKind, CommandResult, Entity, SpacesEdit, SpacesOperation, SpacesQuantified,
    SpacesQuantities, WorkbookColumn, WorkbookFormat, WorkbookSheet,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, spaces_edit, spaces_quantities};
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Choice, Dialog as Frame, Menu, Select, overlay};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Metraj";
const COPY: &str = "Panoya kopyala";
const CLOSE: &str = "Kapat";

/// The table shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Spaces,
    Walls,
    Openings,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Tab::Spaces => "Mahaller",
            Tab::Walls => "Duvarlar",
            Tab::Openings => "Açıklıklar",
        })
    }
}

#[derive(Debug, Clone)]
pub struct Window {
    pub tab: Tab,
    /// The level shown; none: all.
    pub level: Option<String>,
    pub answer: Result<SpacesQuantified, String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Tab(Tab),
    /// The level's index in the list (0: all).
    Level(usize),
    Copy,
    Export(WorkbookFormat),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Quantities(e)
}

/// A table: its heading, the columns that hold numbers, its rows as the window shows them and as a sheet's cells.
struct Table {
    name: &'static str,
    heading: Vec<&'static str>,
    numbers: Vec<bool>,
    /// The columns' widths in the window, logical pixels.
    widths: Vec<f32>,
    shown: Vec<Vec<String>>,
    cells: Vec<Vec<String>>,
}

/// A method's short name, a column of its own.
fn short(m: kentos_commands::SpaceMethod) -> &'static str {
    match m {
        kentos_commands::SpaceMethod::Net => "Net",
        kentos_commands::SpaceMethod::Centerline => "Eksen",
        kentos_commands::SpaceMethod::Gross => "Brüt",
    }
}

impl App {
    fn quantities_read(&self, level: Option<&String>) -> Result<SpacesQuantified, String> {
        let Some(doc) = self.document.as_ref() else {
            return Err("Açık çizim yok.".into());
        };
        let input = SpacesQuantities {
            levels: level.map(|l| vec![l.clone()]),
            ..Default::default()
        };
        match spaces_quantities::plan_in(&doc.model, &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err(String::new()),
        }
    }

    /// The project's levels, by id and name.
    fn quantities_levels(&self) -> Vec<(String, String)> {
        self.document
            .as_ref()
            .and_then(|d| d.model.settings().arch.clone())
            .map(|a| a.levels.into_iter().map(|l| (l.id, l.name)).collect())
            .unwrap_or_default()
    }

    /// Metraj (`arch.quantities`): the window.
    pub(crate) fn open_quantities(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let answer = self.quantities_read(None);
        self.quantities = Some(Window {
            tab: Tab::Spaces,
            level: None,
            answer,
        });
        self.dialog = Some(Dialog::Quantities);
    }

    /// Mahalleri birleştir (`arch.spaceMerge`): the spaces selected made one.
    pub(crate) fn space_merge(&mut self) {
        const WHAT: &str = "Mahalleri birleştir";
        let ids = self.selection.ids().to_vec();
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let spaces: Vec<_> = ids
            .iter()
            .copied()
            .filter(|s| matches!(doc.model.get(*s), Some(Entity::Space(_))))
            .collect();
        if spaces.len() < 2 {
            self.warn(format!(
                "{WHAT}: önce birbirine değen en az iki mahal seçin."
            ));
            return;
        }
        let input = SpacesEdit {
            uids: kentos_interaction::properties::uids_of(&doc.model, &spaces),
            operation: SpacesOperation::Merge,
            expected_revision: None,
        };
        match spaces_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                let kept: Vec<_> = output
                    .kept
                    .iter()
                    .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                    .filter_map(|u| doc.model.slot_of(u))
                    .collect();
                self.selection.set(kept);
                self.say(
                    Level::Success,
                    format!("{WHAT}: {} mahal tek mahal oldu.", spaces.len()),
                );
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }

    /// The three tables of the answer, in the project's units.
    fn quantities_tables(&self, q: &SpacesQuantified) -> [Table; 3] {
        let f = self
            .document
            .as_ref()
            .map(|d| kentos_project::format::Format::of(d.model.settings()))
            .unwrap_or_default();
        let fixed = kentos_geometry_core::display::fixed;
        let opt = |v: Option<f64>, k: usize| v.map_or_else(String::new, |v| fixed(v, k));
        let len = |v: f64| f.length_bare(v);
        let area = |v: f64| f.area_bare(v);
        let words = |v: &Option<String>| v.clone().unwrap_or_default();
        let level_name = |id: &Option<String>| {
            id.as_ref().map_or_else(String::new, |id| {
                self.quantities_levels()
                    .into_iter()
                    .find(|(l, _)| l == id)
                    .map_or_else(|| id.clone(), |(_, n)| n)
            })
        };
        let by = |r: &kentos_commands::SpaceRow, m: &str| {
            r.by_method
                .as_ref()
                .and_then(|b| b.get(m))
                .map_or_else(String::new, |v| fixed(*v, 2))
        };
        let spaces = Table {
            name: "Mahaller",
            heading: vec![
                "Numara",
                "Ad",
                "Kullanım",
                "Kat",
                "Yöntem",
                "Alan",
                "Çevre",
                "Yükseklik",
                "Hacim",
                "Net",
                "Eksen",
                "Brüt",
            ],
            numbers: vec![
                false, false, false, false, false, true, true, true, true, true, true, true,
            ],
            widths: vec![
                56.0, 96.0, 80.0, 78.0, 48.0, 64.0, 64.0, 64.0, 64.0, 60.0, 60.0, 60.0,
            ],
            shown: q
                .spaces
                .iter()
                .map(|r| {
                    vec![
                        words(&r.number),
                        words(&r.name),
                        words(&r.usage),
                        level_name(&r.level),
                        short(r.method).to_owned(),
                        area(r.area),
                        len(r.perimeter),
                        r.height.map_or_else(String::new, len),
                        opt(r.volume, 2),
                        by(r, "net"),
                        by(r, "centerline"),
                        by(r, "gross"),
                    ]
                })
                .collect(),
            cells: q
                .spaces
                .iter()
                .map(|r| {
                    let n = |v: f64| format!("{v}");
                    let o = |v: Option<f64>| v.map_or_else(String::new, n);
                    let b = |m: &str| o(r.by_method.as_ref().and_then(|b| b.get(m).copied()));
                    vec![
                        words(&r.number),
                        words(&r.name),
                        words(&r.usage),
                        level_name(&r.level),
                        r.method.label().to_owned(),
                        n(r.area),
                        n(r.perimeter),
                        o(r.height),
                        o(r.volume),
                        b("net"),
                        b("centerline"),
                        b("gross"),
                    ]
                })
                .collect(),
        };
        // A wall style's name: the project's, else KentOS's own.
        let styles: Vec<kentos_commands::WallStyleDef> = self
            .document
            .as_ref()
            .map(|d| d.model.settings().wall_styles.clone())
            .unwrap_or_default()
            .into_iter()
            .chain(kentos_commands::builtin_wall_styles())
            .collect();
        let style_name = |s: &Option<String>| {
            s.as_deref().map_or_else(String::new, |id| {
                styles
                    .iter()
                    .find(|t| t.id == id)
                    .map_or_else(|| id.to_owned(), |t| t.name.clone())
            })
        };
        let walls = Table {
            name: "Duvarlar",
            heading: vec![
                "Stil",
                "Kat",
                "Uzunluk",
                "Kalınlık",
                "Yükseklik",
                "Brüt yüzey",
                "Açıklıklar",
                "Net yüzey",
                "Hacim",
            ],
            numbers: vec![false, false, true, true, true, true, true, true, true],
            widths: vec![120.0, 90.0, 80.0, 80.0, 80.0, 88.0, 88.0, 88.0, 80.0],
            shown: q
                .walls
                .iter()
                .map(|r| {
                    vec![
                        style_name(&r.style),
                        level_name(&r.level),
                        len(r.length),
                        len(r.width),
                        r.height.map_or_else(String::new, len),
                        opt(r.gross_area, 2),
                        fixed(r.openings_area, 2),
                        opt(r.net_area, 2),
                        opt(r.volume, 3),
                    ]
                })
                .collect(),
            cells: q
                .walls
                .iter()
                .map(|r| {
                    let o = |v: Option<f64>| v.map_or_else(String::new, |v| format!("{v}"));
                    vec![
                        style_name(&r.style),
                        level_name(&r.level),
                        format!("{}", r.length),
                        format!("{}", r.width),
                        o(r.height),
                        o(r.gross_area),
                        format!("{}", r.openings_area),
                        o(r.net_area),
                        o(r.volume),
                    ]
                })
                .collect(),
        };
        let types = kentos_commands::builtin_opening_types();
        let type_name = |s: &Option<String>| {
            s.as_deref().map_or_else(String::new, |id| {
                types
                    .iter()
                    .find(|t| t.id == id)
                    .map_or_else(|| id.to_owned(), |t| t.name.clone())
            })
        };
        let openings = Table {
            name: "Açıklıklar",
            heading: vec!["Numara", "Biçim", "Tür", "Genişlik", "Yükseklik", "Alan"],
            numbers: vec![false, false, false, true, true, true],
            widths: vec![72.0, 90.0, 180.0, 88.0, 88.0, 88.0],
            shown: q
                .openings
                .iter()
                .map(|r| {
                    vec![
                        words(&r.number),
                        r.form.label().to_owned(),
                        type_name(&r.style),
                        len(r.width),
                        r.height.map_or_else(String::new, len),
                        opt(r.area, 2),
                    ]
                })
                .collect(),
            cells: q
                .openings
                .iter()
                .map(|r| {
                    let o = |v: Option<f64>| v.map_or_else(String::new, |v| format!("{v}"));
                    vec![
                        words(&r.number),
                        r.form.label().to_owned(),
                        type_name(&r.style),
                        format!("{}", r.width),
                        o(r.height),
                        o(r.area),
                    ]
                })
                .collect(),
        };
        [spaces, walls, openings]
    }

    pub(crate) fn quantities_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.quantities.as_ref() else {
            return Task::none();
        };
        match event {
            Event::Tab(t) => {
                if let Some(w) = self.quantities.as_mut() {
                    w.tab = t;
                }
            }
            Event::Level(i) => {
                let level = i
                    .checked_sub(1)
                    .and_then(|k| self.quantities_levels().get(k).map(|(id, _)| id.clone()));
                let answer = self.quantities_read(level.as_ref());
                if let Some(w) = self.quantities.as_mut() {
                    w.level = level;
                    w.answer = answer;
                }
            }
            Event::Copy => {
                let Ok(q) = &w.answer else {
                    return Task::none();
                };
                let tables = self.quantities_tables(q);
                let t = &tables[w.tab as usize];
                let mut lines = vec![t.heading.join("\t")];
                lines.extend(t.shown.iter().map(|r| r.join("\t")));
                let n = t.shown.len();
                self.say(
                    Level::Info,
                    format!("Metrajın {n} satırı panoya kopyalandı."),
                );
                return iced::clipboard::write(lines.join("\n"));
            }
            Event::Export(format) => {
                let Ok(q) = &w.answer else {
                    return Task::none();
                };
                let sheets = self
                    .quantities_tables(q)
                    .into_iter()
                    .map(|t| WorkbookSheet {
                        name: t.name.to_owned(),
                        columns: t
                            .heading
                            .iter()
                            .zip(&t.numbers)
                            .map(|(h, &n)| WorkbookColumn {
                                name: (*h).to_owned(),
                                kind: if n { CellKind::Number } else { CellKind::Text },
                                decimals: None,
                            })
                            .collect(),
                        rows: t.cells,
                    })
                    .collect();
                return crate::table_write::save(sheets, format, "metraj".to_owned());
            }
            Event::Close => {
                self.quantities = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn quantities_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.quantities else {
            return text("").into();
        };
        let levels = self.quantities_levels();
        let mut names = vec!["Bütün katlar".to_owned()];
        names.extend(levels.iter().map(|(_, n)| n.clone()));
        let chosen = w
            .level
            .as_ref()
            .and_then(|l| levels.iter().position(|(id, _)| id == l))
            .map_or(0, |k| k + 1);
        let level: Element<'_, Message> =
            Select::new(names.into_iter().map(Choice::new), Some(chosen), |i| {
                msg(Event::Level(i))
            })
            .searchable(false)
            .into();
        let tabs = Segmented::new([Tab::Spaces, Tab::Walls, Tab::Openings], w.tab, |t| {
            msg(Event::Tab(t))
        });
        let head = row![
            words::field("Gösterilen", tabs, None),
            words::field("Kat", container(level).width(200), None),
        ]
        .spacing(16)
        .align_y(iced::Bottom);
        let q = match &w.answer {
            Ok(q) => q,
            Err(why) => {
                return overlay::modal(
                    Frame::new(TITLE)
                        .push(words::text_line(Kind::Error, why.clone()))
                        .action(words::primary(CLOSE, Some(msg(Event::Close))))
                        .width(720.0),
                    msg(Event::Close),
                );
            }
        };
        let f = self
            .document
            .as_ref()
            .map(|d| kentos_project::format::Format::of(d.model.settings()))
            .unwrap_or_default();
        let t = &q.totals;
        // A wall's faces need its height (its own or its level's): said when none has one.
        let faces = if q.walls.iter().any(|w| w.net_area.is_some()) {
            format!(
                "net yüzey {} m²",
                kentos_geometry_core::display::fixed(t.wall_net_area, 2)
            )
        } else {
            "yüksekliği olan duvar yok".to_owned()
        };
        let summary = words::summary(vec![
            words::text_line(
                Kind::Info,
                format!(
                    "{} mahal, toplam {}; {} duvar, {} uzunluk, {faces}; {} açıklık, {} m².",
                    q.spaces.len(),
                    f.area(t.space_area),
                    q.walls.len(),
                    f.length(t.wall_length),
                    t.openings,
                    kentos_geometry_core::display::fixed(t.opening_area, 2),
                ),
            ),
            words::text_line(Kind::Ok, self.quantities_rule(q, w.tab)),
        ]);
        let tables = self.quantities_tables(q);
        let tab = &tables[w.tab as usize];
        let width = |i: usize| tab.widths[i];
        let heading = tab
            .heading
            .iter()
            .enumerate()
            .fold(Row::new().spacing(6), |r, (i, h)| {
                let t = text(*h)
                    .font(typography::ui_strong())
                    .size(typography::caption())
                    .width(width(i));
                r.push(if tab.numbers[i] {
                    t.align_x(iced::alignment::Horizontal::Right)
                } else {
                    t
                })
            });
        let mut body = Column::new().spacing(2);
        for r in &tab.shown {
            body = body.push(
                r.iter()
                    .enumerate()
                    .fold(Row::new().spacing(6), |row, (i, c)| {
                        let t = text(c.clone())
                            .size(typography::caption())
                            .wrapping(text::Wrapping::None)
                            .width(width(i));
                        row.push(if tab.numbers[i] {
                            t.align_x(iced::alignment::Horizontal::Right)
                        } else {
                            t
                        })
                    }),
            );
        }
        let table: Element<'_, Message> = if tab.shown.is_empty() {
            words::text_line(
                Kind::Info,
                format!("Bu katta {} yok.", tab.name.to_lowercase()),
            )
        } else {
            container(
                scrollable(column![heading, body].spacing(6))
                    .direction(scrollable::Direction::Both {
                        vertical: Default::default(),
                        horizontal: Default::default(),
                    })
                    .height(280),
            )
            .padding([8, 10])
            .style(style::container::field_box)
            .width(Fill)
            .into()
        };
        let any = !tab.shown.is_empty();
        let face = container(
            row![
                kentos_ui::icon::icon(crate::icons::from_web(Some("export"))).size(14.0),
                kentos_ui::label::body("Dışa aktar"),
                kentos_ui::icon::icon(kentos_ui::icon::Icon::ChevronDown)
                    .size(12.0)
                    .tone(kentos_ui::icon::Tone::Muted),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([4, 10])
        .style(style::container::field_box);
        let export: Element<'_, Message> = kentos_ui::widget::MenuButton::new(face, || {
            crate::table_write::items(Menu::new(), |f| msg(Event::Export(f)))
        })
        .into();
        let body = Column::new()
            .spacing(12)
            .push(head)
            .push(summary)
            .push(table)
            .push(row![export].width(Fill));
        overlay::modal(
            Frame::new(TITLE)
                .push(body)
                .action(words::secondary(COPY, any.then(|| msg(Event::Copy))))
                .action(words::primary(CLOSE, Some(msg(Event::Close))))
                .width(980.0),
            msg(Event::Close),
        )
    }

    /// The rule the shown table's numbers come from: the command's own words.
    fn quantities_rule(&self, q: &SpacesQuantified, tab: Tab) -> String {
        match tab {
            Tab::Spaces => q.spaces.first().map_or_else(
                || "Alan: halkaların alanı, delikler düşülür; hacim: alan × yükseklik.".to_owned(),
                |r| format!("Kural: {}.", r.rule),
            ),
            Tab::Walls => q.walls.first().map_or_else(
                || {
                    "Yüzey: taban çizgisinin uzunluğu × yükseklik, açıklıkların alanı düşülür."
                        .to_owned()
                },
                |r| format!("Kural: {}.", r.rule),
            ),
            Tab::Openings => {
                "Kural: genişlik × (lento − denizlik); denizliği olmayanın yüksekliği lentodur."
                    .to_owned()
            }
        }
    }

    /// The window's buttons by their words (a trace's `dialog` step).
    pub(crate) fn quantities_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.quantities.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(COPY) => Some(msg(Event::Copy)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press("Mahaller") => Some(msg(Event::Tab(Tab::Spaces))),
            Control::Press("Duvarlar") => Some(msg(Event::Tab(Tab::Walls))),
            Control::Press("Açıklıklar") => Some(msg(Event::Tab(Tab::Openings))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
