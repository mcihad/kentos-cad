//! Metraj (docs/adr/0342 §4, §5; `arch.quantities`): the drawing's spaces, walls, openings, slabs, the floor finishes'
//! changes at the passages (docs/adr/0722 §5), ramps (docs/adr/0723 §3) and stairs (docs/adr/0724 §5) measured by
//! `cad.spaces.quantities`, a level's or all, each row with the rule its numbers come from, and the totals; a room's
//! baseboard in its own column. Panoya kopyala copies the table shown as tab-separated text; Dışa aktar ▾ writes the
//! shown table as CSV or all seven tables as a workbook's
//! sheets (table_write.rs). Mahalleri birleştir (`arch.spaceMerge`) is here too: the spaces selected, made one by
//! `cad.spaces.edit`.

use iced::widget::{Column, container, row, text};
use iced::{Center, Element, Fill, Length, Task};
use kentos_commands::{
    CellKind, CommandResult, Entity, SpacesEdit, SpacesOperation, SpacesQuantified,
    SpacesQuantities, WorkbookColumn, WorkbookFormat, WorkbookSheet,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, spaces_edit, spaces_quantities};
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::table;
use kentos_ui::widget::{Choice, Dialog as Frame, Menu, Select};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
/// Mahalleri birleştir's question (docs/adr/0773 §4).
pub const MERGE_TITLE: &str = "Mahalleri birleştir";
pub const TITLE: &str = "Metraj";
const COPY: &str = "Panoya kopyala";
const CLOSE: &str = "Kapat";

#[cfg(test)]
mod export_tests;

/// The table shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Spaces,
    Walls,
    Openings,
    /// The slabs and the passages' floor changes (docs/adr/0722 §5).
    Slabs,
    Transitions,
    /// The ramps (docs/adr/0723 §3).
    Ramps,
    /// The stairs (docs/adr/0724 §5).
    Stairs,
}

impl std::fmt::Display for Tab {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Tab::Spaces => "Mahaller",
            Tab::Walls => "Duvarlar",
            Tab::Openings => "Açıklıklar",
            Tab::Slabs => "Döşemeler",
            Tab::Transitions => "Geçişler",
            Tab::Ramps => "Rampalar",
            Tab::Stairs => "Merdivenler",
        })
    }
}

#[derive(Debug, Clone)]
pub struct Window {
    pub tab: Tab,
    /// The level shown; none: all.
    pub level: Option<String>,
    pub answer: Result<SpacesQuantified, String>,
    /// Sayılar: the worked-out numbers to 15 significant digits (Tam) rather than as shown (docs/adr/0733 §7).
    pub exact: bool,
}

#[derive(Debug, Clone)]
pub enum Event {
    Tab(Tab),
    /// The level's index in the list (0: all).
    Level(usize),
    Copy,
    Export(WorkbookFormat),
    /// Sayılar: Tam or Görünen basamakla.
    Exact(bool),
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
    /// Minimum column widths at the default UI size, logical pixels.
    widths: Vec<f32>,
    shown: Vec<Vec<String>>,
    cells: Vec<Vec<String>>,
}

impl Table {
    /// Measure the actual selected font; a fixed width must not let one cell
    /// draw over its neighbour. Long names wrap; numbers remain whole and scroll.
    fn display_widths(&self) -> Vec<f32> {
        let size = typography::body();
        self.widths
            .iter()
            .enumerate()
            .map(|(i, minimum)| {
                let heading =
                    typography::measured_width(self.heading[i], typography::caption(), false);
                let values = self
                    .shown
                    .iter()
                    .filter_map(|r| r.get(i))
                    .map(|s| typography::measured_width(s, size, false))
                    .fold(0.0, f32::max);
                let values = if self.numbers[i] {
                    values
                } else {
                    values.min(typography::from_default(240.0))
                };
                typography::from_default(*minimum)
                    .max(heading.ceil() + 12.0)
                    .max(values.ceil() + 12.0)
            })
            .collect()
    }
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
            exact: false,
        });
        self.dialog = Some(Dialog::Quantities);
    }

    /// Mahalleri birleştir (`arch.spaceMerge`): the spaces selected made one. Nothing is written before the decision
    /// is said (docs/adr/0773 §4): whose id and number go on, whose numbers, finishes and tags go, the fixtures in it;
    /// Birleştir writes it.
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
            operation: SpacesOperation::Merge { keep: None, number: None },
            expected_revision: None,
        };
        match spaces_edit::plan_in(&doc.model, &input) {
            CommandResult::Completed { output, warnings } => {
                let mut said = merge_words(&doc.model, &output);
                for w in warnings {
                    said.push_str(&format!(" {}", w.message));
                }
                self.space_merging = Some((input, said));
                self.dialog = Some(Dialog::SpaceMerge);
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }

    /// Birleştir: the merge said, written.
    pub(crate) fn space_merge_confirmed(&mut self) {
        const WHAT: &str = "Mahalleri birleştir";
        let Some((input, _)) = self.space_merging.take() else {
            return;
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let n = input.uids.len();
        match spaces_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => {
                let kept: Vec<_> = output
                    .kept
                    .iter()
                    .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                    .filter_map(|u| doc.model.slot_of(u))
                    .collect();
                self.selection.set(kept);
                self.say(Level::Success, format!("{WHAT}: {n} mahal tek mahal oldu."));
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => self.warn(error.message),
            CommandResult::Queued { .. } | CommandResult::Cancelled => {}
        }
    }

    pub(crate) fn space_merge_question(&self) -> Element<'_, Message> {
        let said = self.space_merging.as_ref().map(|(_, w)| w.clone()).unwrap_or_default();
        kentos_ui::widget::overlay::modal(
            kentos_ui::widget::Confirm::new(MERGE_TITLE, Message::DialogConfirmed, Message::DialogClosed)
                .message(said)
                .confirm("Birleştir"),
            Message::DialogClosed,
        )
    }

    /// The three tables of the answer, in the project's units.
    fn quantities_tables(&self, q: &SpacesQuantified) -> [Table; 7] {
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
        let mut spaces = Table {
            name: "Mahaller",
            heading: vec![
                "Numara",
                "Ad",
                "Kullanım",
                "Kat",
                "Yöntem",
                "Alan",
                "Çevre",
                // A room's baseboard (docs/adr/0722 §5); a void has none.
                "Süpürgelik",
                "Yükseklik",
                "Hacim",
                "Net",
                "Eksen",
                "Brüt",
            ],
            numbers: vec![
                false, false, false, false, false, true, true, true, true, true, true, true, true,
            ],
            widths: vec![
                56.0, 96.0, 80.0, 78.0, 48.0, 64.0, 64.0, 72.0, 64.0, 64.0, 60.0, 60.0, 60.0,
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
                        r.baseboard.map_or_else(String::new, len),
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
                        o(r.baseboard),
                        o(r.height),
                        o(r.volume),
                        b("net"),
                        b("centerline"),
                        b("gross"),
                    ]
                })
                .collect(),
        };
        // A void (docs/adr/0708) says so in its own column, in the table and in the cells copied or exported; its area
        // is summed apart. Without a void the table is as it was.
        if q.spaces.iter().any(|r| r.void) {
            spaces.heading.insert(3, "Boşluk");
            spaces.numbers.insert(3, false);
            spaces.widths.insert(3, 56.0);
            for ((shown, cells), room) in spaces
                .shown
                .iter_mut()
                .zip(&mut spaces.cells)
                .zip(&q.spaces)
            {
                let mark = if room.void { "Boşluk" } else { "" }.to_owned();
                shown.insert(3, mark.clone());
                cells.insert(3, mark);
            }
        }
        // A later stale room must not be hidden by the first room's valid rule.
        // The warning goes to both the displayed table and the copied/exported cells.
        if q.spaces.iter().any(|r| r.boundary_break.is_some()) {
            spaces.heading.insert(2, "Durum");
            spaces.numbers.insert(2, false);
            spaces.widths.insert(2, 180.0);
            for ((shown, cells), room) in spaces
                .shown
                .iter_mut()
                .zip(&mut spaces.cells)
                .zip(&q.spaces)
            {
                let status = room
                    .boundary_break
                    .map(|why| format!("Son geçerli sınır: {}", why.words()))
                    .unwrap_or_default();
                shown.insert(2, status.clone());
                cells.insert(2, status);
            }
        }
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
            // A niche's or flue's depth (docs/adr/0719 §4).
            heading: vec![
                "Numara",
                "Biçim",
                "Tür",
                "Genişlik",
                "Yükseklik",
                "Alan",
                "Derinlik",
            ],
            numbers: vec![false, false, false, true, true, true, true],
            widths: vec![72.0, 90.0, 180.0, 88.0, 88.0, 88.0, 88.0],
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
                        r.depth.map_or_else(String::new, len),
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
                        o(r.depth),
                    ]
                })
                .collect(),
        };
        let slabs = Table {
            name: "Döşemeler",
            heading: vec![
                "Tür",
                "Kat",
                "Malzeme",
                "Kalınlık",
                "Üst kot",
                "Alan",
                "Boşluk",
                "Net alan",
                "Çevre",
                "Hacim",
                "Kenar",
                "Kenar uzunluğu",
                "Metraj",
            ],
            numbers: vec![
                false, false, false, true, true, true, true, true, true, true, false, true, false,
            ],
            widths: vec![
                72.0, 80.0, 110.0, 72.0, 72.0, 72.0, 64.0, 72.0, 64.0, 64.0, 80.0, 96.0, 72.0,
            ],
            shown: q
                .slabs
                .iter()
                .map(|r| {
                    vec![
                        r.role.label().to_owned(),
                        level_name(&r.level),
                        words(&r.material),
                        len(r.thickness),
                        r.top.map_or_else(String::new, len),
                        area(r.area),
                        if r.void_area > 0.0 {
                            area(r.void_area)
                        } else {
                            String::new()
                        },
                        area(r.net_area),
                        len(r.perimeter),
                        fixed(r.volume, 3),
                        r.edge.map_or_else(String::new, |e| e.label().to_owned()),
                        if r.edge_length > 0.0 {
                            len(r.edge_length)
                        } else {
                            String::new()
                        },
                        if r.excluded { "Dışında" } else { "Toplamda" }.to_owned(),
                    ]
                })
                .collect(),
            cells: q
                .slabs
                .iter()
                .map(|r| {
                    let n = |v: f64| format!("{v}");
                    vec![
                        r.role.label().to_owned(),
                        level_name(&r.level),
                        words(&r.material),
                        n(r.thickness),
                        r.top.map_or_else(String::new, n),
                        n(r.area),
                        n(r.void_area),
                        n(r.net_area),
                        n(r.perimeter),
                        n(r.volume),
                        r.edge.map_or_else(String::new, |e| e.label().to_owned()),
                        n(r.edge_length),
                        if r.excluded { "Dışında" } else { "Toplamda" }.to_owned(),
                    ]
                })
                .collect(),
        };
        // A room by its number and name (its id when it has neither), for the passages' sides.
        let room = |uid: &str| {
            q.spaces.iter().find(|s| s.uid == uid).map_or_else(
                || uid.to_owned(),
                |s| match (&s.number, &s.name) {
                    (Some(n), Some(a)) => format!("{n} {a}"),
                    (Some(n), None) => n.clone(),
                    (None, Some(a)) => a.clone(),
                    (None, None) => uid.to_owned(),
                },
            )
        };
        let rows: Vec<Vec<String>> = q
            .transitions
            .iter()
            .map(|r| {
                vec![
                    words(&r.number),
                    room(&r.spaces[0]),
                    r.floors[0].clone(),
                    room(&r.spaces[1]),
                    r.floors[1].clone(),
                    len(r.width),
                ]
            })
            .collect();
        let transitions = Table {
            name: "Geçişler",
            heading: vec![
                "Açıklık",
                "Sol mahal",
                "Sol kaplama",
                "Sağ mahal",
                "Sağ kaplama",
                "Şerit",
            ],
            numbers: vec![false, false, false, false, false, true],
            widths: vec![72.0, 120.0, 110.0, 120.0, 110.0, 72.0],
            cells: q
                .transitions
                .iter()
                .zip(&rows)
                .map(|(r, shown)| {
                    let mut cells = shown.clone();
                    cells[5] = format!("{}", r.width);
                    cells
                })
                .collect(),
            shown: rows,
        };
        let ramps = Table {
            name: "Rampalar",
            heading: vec![
                "Kat",
                "Genişlik",
                "Kot farkı",
                "Eğim",
                "Eğimli uzunluk",
                "Orta çizgi",
                "Sahanlık",
                "Alan",
                "Başlangıç",
                "Bitiş",
                "Korkuluk",
            ],
            numbers: vec![
                false, true, true, true, true, true, true, true, true, true, true,
            ],
            widths: vec![
                80.0, 72.0, 72.0, 64.0, 96.0, 80.0, 72.0, 72.0, 80.0, 72.0, 72.0,
            ],
            shown: q
                .ramps
                .iter()
                .map(|r| {
                    vec![
                        level_name(&r.level),
                        len(r.width),
                        len(r.rise),
                        r.slope
                            .map_or_else(String::new, |s| format!("%{}", fixed(s, 2))),
                        len(r.run),
                        len(r.length),
                        r.landings.to_string(),
                        area(r.area),
                        // Bound to its level: said so (docs/adr/0723 §2).
                        if r.base_offset.is_some() {
                            format!("{} · katın", len(r.base))
                        } else {
                            len(r.base)
                        },
                        len(r.top),
                        if r.railing_length > 0.0 {
                            len(r.railing_length)
                        } else {
                            String::new()
                        },
                    ]
                })
                .collect(),
            cells: q
                .ramps
                .iter()
                .map(|r| {
                    let n = |v: f64| format!("{v}");
                    vec![
                        level_name(&r.level),
                        n(r.width),
                        n(r.rise),
                        r.slope.map_or_else(String::new, n),
                        n(r.run),
                        n(r.length),
                        r.landings.to_string(),
                        n(r.area),
                        n(r.base),
                        n(r.top),
                        n(r.railing_length),
                    ]
                })
                .collect(),
        };
        let stairs = Table {
            name: "Merdivenler",
            heading: vec![
                "Kat",
                "Genişlik",
                "Rıht",
                "Rıht yüksekliği",
                "Basamak",
                "Yükseklik",
                "Uzunluk",
                "Sahanlık",
                "Alan",
                "Korkuluk",
                "Yarıçap",
                "İç basamak",
            ],
            numbers: vec![
                false, true, true, true, true, true, true, true, true, true, true, true,
            ],
            widths: vec![
                80.0, 72.0, 56.0, 96.0, 72.0, 88.0, 72.0, 72.0, 72.0, 72.0, 72.0, 80.0,
            ],
            shown: q
                .stairs
                .iter()
                .map(|s| {
                    vec![
                        level_name(&s.level),
                        len(s.width),
                        s.risers.to_string(),
                        len(s.riser_height),
                        len(s.tread),
                        // Following its level's height: said so (docs/adr/0724 §1).
                        if s.height_from_level {
                            format!("{} · katın", len(s.height))
                        } else {
                            len(s.height)
                        },
                        len(s.length),
                        s.landings.to_string(),
                        area(s.area),
                        if s.railing_length > 0.0 {
                            len(s.railing_length)
                        } else {
                            String::new()
                        },
                        s.radius.map_or_else(String::new, len),
                        s.inner_tread.map_or_else(String::new, len),
                    ]
                })
                .collect(),
            cells: q
                .stairs
                .iter()
                .map(|s| {
                    let n = |v: f64| format!("{v}");
                    vec![
                        level_name(&s.level),
                        n(s.width),
                        s.risers.to_string(),
                        n(s.riser_height),
                        n(s.tread),
                        n(s.height),
                        n(s.length),
                        s.landings.to_string(),
                        n(s.area),
                        n(s.railing_length),
                        s.radius.map_or_else(String::new, n),
                        s.inner_tread.map_or_else(String::new, n),
                    ]
                })
                .collect(),
        };
        [spaces, walls, openings, slabs, transitions, ramps, stairs]
    }

    /// CSV has one table: the one shown. Workbooks keep all seven sheets,
    /// including empty headings, under the same selected level filter. A number column's cells as the window shows
    /// them (the project's decimals), or with Sayılar: Tam to 15 significant digits (docs/adr/0733 §7); the words as
    /// they are.
    fn quantities_sheets(
        &self,
        q: &SpacesQuantified,
        tab: Tab,
        format: WorkbookFormat,
    ) -> Vec<WorkbookSheet> {
        let exact = self.quantities.as_ref().is_some_and(|w| w.exact);
        self.quantities_tables(q)
            .into_iter()
            .enumerate()
            .filter(|(i, _)| format != WorkbookFormat::Csv || *i == tab as usize)
            .map(|(_, t)| {
                let rows = t
                    .cells
                    .iter()
                    .zip(&t.shown)
                    .map(|(raw, shown)| {
                        raw.iter()
                            .enumerate()
                            .map(|(j, cell)| {
                                if !t.numbers.get(j).copied().unwrap_or(false) || cell.is_empty() {
                                    return cell.clone();
                                }
                                if exact {
                                    return cell.parse::<f64>().map_or_else(
                                        |_| cell.clone(),
                                        kentos_native_application::table_export::fifteen,
                                    );
                                }
                                // The number the window shows, without its unit.
                                let token = shown
                                    .get(j)
                                    .and_then(|s| s.split_whitespace().next())
                                    .unwrap_or("");
                                let token = token.replace(',', ".");
                                if token.parse::<f64>().is_ok() {
                                    token
                                } else {
                                    cell.clone()
                                }
                            })
                            .collect()
                    })
                    .collect();
                WorkbookSheet {
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
                    rows,
                }
            })
            .collect()
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
            Event::Exact(on) => {
                if let Some(w) = self.quantities.as_mut() {
                    w.exact = on;
                }
                return Task::none();
            }
            Event::Export(format) => {
                let Ok(q) = &w.answer else {
                    return Task::none();
                };
                let sheets = self.quantities_sheets(q, w.tab, format);
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
        let tabs = Segmented::new(
            [
                Tab::Spaces,
                Tab::Walls,
                Tab::Openings,
                Tab::Slabs,
                Tab::Transitions,
                Tab::Ramps,
                Tab::Stairs,
            ],
            w.tab,
            |t| msg(Event::Tab(t)),
        );
        let head = row![
            words::field("Gösterilen", tabs, None),
            words::field("Kat", container(level).width(200), None),
        ]
        .spacing(16)
        .align_y(iced::Bottom);
        let q = match &w.answer {
            Ok(q) => q,
            Err(why) => {
                return Frame::new(TITLE)
                    .push(words::text_line(Kind::Error, why.clone()))
                    .action(words::primary(CLOSE, Some(msg(Event::Close))))
                    .width(720.0)
                    .modal(msg(Event::Close));
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
        let stale = q
            .spaces
            .iter()
            .filter(|r| r.boundary_break.is_some())
            .count();
        let mut summary_lines = vec![
            words::text_line(
                Kind::Info,
                format!(
                    "{} mahal, toplam {}{}; {} duvar, {} uzunluk, {faces}; {} açıklık, {} m².",
                    q.spaces.len(),
                    f.area(t.space_area),
                    if t.void_area > 0.0 {
                        format!(" (boşluklar ayrı: {})", f.area(t.void_area))
                    } else {
                        String::new()
                    },
                    q.walls.len(),
                    f.length(t.wall_length),
                    t.openings,
                    kentos_geometry_core::display::fixed(t.opening_area, 2),
                ),
            ),
            words::text_line(
                if stale == 0 { Kind::Ok } else { Kind::Warn },
                self.quantities_rule(q, w.tab),
            ),
        ];
        if stale > 0 {
            summary_lines.push(words::text_line(Kind::Warn,
                format!("{stale} mahalın sınırı güncel değil; alan ve toplamlar son geçerli sınırı içeriyor.")));
        }
        // Baseboards, slabs and their edges (docs/adr/0722 §5): said only when the drawing has them.
        let mut finish = Vec::new();
        if t.baseboard_length > 0.0 {
            finish.push(format!("süpürgelik {}", f.length(t.baseboard_length)));
        }
        if !q.slabs.is_empty() {
            let out = q.slabs.iter().filter(|s| s.excluded).count();
            finish.push(format!(
                "{} döşeme, net {} ({} m³){}",
                q.slabs.len(),
                f.area(t.slab_area),
                kentos_geometry_core::display::fixed(t.slab_volume, 2),
                if out > 0 {
                    format!(", {out} döşeme metraj dışında")
                } else {
                    String::new()
                }
            ));
        }
        if t.railing_length > 0.0 {
            finish.push(format!("korkuluk {}", f.length(t.railing_length)));
        }
        if t.parapet_length > 0.0 {
            finish.push(format!("parapet {}", f.length(t.parapet_length)));
        }
        if !q.transitions.is_empty() {
            finish.push(format!("{} kaplama geçişi", q.transitions.len()));
        }
        // Ramps (docs/adr/0723 §3).
        if !q.ramps.is_empty() {
            finish.push(format!("{} rampa, {}", q.ramps.len(), f.area(t.ramp_area)));
        }
        if t.ramp_railing_length > 0.0 {
            finish.push(format!(
                "rampa korkuluğu {}",
                f.length(t.ramp_railing_length)
            ));
        }
        // Stairs (docs/adr/0724 §5).
        if !q.stairs.is_empty() {
            finish.push(format!(
                "{} merdiven, {} rıht, {}",
                q.stairs.len(),
                t.stair_risers,
                f.area(t.stair_area)
            ));
        }
        if t.stair_railing_length > 0.0 {
            finish.push(format!(
                "merdiven korkuluğu {}",
                f.length(t.stair_railing_length)
            ));
        }
        if !finish.is_empty() {
            let mut line = finish.join("; ");
            if let Some(first) = line.get(..1) {
                line = first.to_uppercase() + &line[1..];
            }
            summary_lines.push(words::text_line(Kind::Info, format!("{line}.")));
        }
        let summary = words::summary(summary_lines);
        let tables = self.quantities_tables(q);
        let tab = &tables[w.tab as usize];
        let widths = tab.display_widths();
        let table: Element<'_, Message> = if tab.shown.is_empty() {
            words::text_line(
                Kind::Info,
                format!("Bu katta {} yok.", tab.name.to_lowercase()),
            )
        } else {
            let columns = tab.heading.iter().enumerate().map(|(i, h)| {
                let column =
                    table::Column::new(*h).width(Length::Fixed(typography::unscaled(widths[i])));
                if tab.numbers[i] {
                    column.align_right()
                } else {
                    column
                }
            });
            // The head stays while the rows scroll; every column reaches its widest value, the table scrolls sideways.
            let shown = tab.shown.clone();
            let numbers = tab.numbers.clone();
            container(
                table::Table::new(columns)
                    .horizontal()
                    .grid()
                    .height(280.0)
                    .min_width(typography::from_default(700.0))
                    .virtualized(shown.len(), move |i| {
                        table::Row::new(shown[i].iter().enumerate().map(|(j, value)| {
                            if numbers[j] {
                                table::number(value.clone()).into()
                            } else {
                                table::text(value.clone()).into()
                            }
                        }))
                    }),
            )
            .style(style::container::bordered)
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
        // Sayılar (docs/adr/0733 §7): the worked-out numbers as shown, or to 15 significant digits.
        let exact = self.quantities.as_ref().is_some_and(|w| w.exact);
        let export: Element<'_, Message> = row![
            export,
            crate::exchange::words::check(
                exact,
                "Tam sayılar (15 basamak)",
                Some(msg(Event::Exact(!exact)))
            ),
        ]
        .spacing(10)
        .align_y(Center)
        .into();
        let body = Column::new()
            .spacing(12)
            .push(head)
            .push(summary)
            .push(table)
            .push(words::text_line(Kind::Info,
                "CSV: gösterilen tablo. Excel ve ODS: bu kat süzgecindeki yedi tablo ayrı sayfalarda."))
            .push(row![export].width(Fill));
        Frame::new(TITLE)
            .scroll(body)
            .action(words::secondary(COPY, any.then(|| msg(Event::Copy))))
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(980.0)
            .modal(msg(Event::Close))
    }

    /// The rule the shown table's numbers come from: the command's own words.
    fn quantities_rule(&self, q: &SpacesQuantified, tab: Tab) -> String {
        match tab {
            Tab::Spaces => q.spaces.first().map_or_else(
                || "Alan: halkaların alanı, delikler düşülür; hacim: alan × yükseklik.".to_owned(),
                |r| format!("Kural: {}.", r.rule.trim_end_matches('.')),
            ),
            Tab::Walls => q.walls.first().map_or_else(
                || {
                    "Yüzey: taban çizgisinin uzunluğu × yükseklik, duvarı geçen açıklıkların alanı düşülür (niş ve baca düşülmez)."
                        .to_owned()
                },
                |r| format!("Kural: {}.", r.rule),
            ),
            Tab::Openings => {
                "Kural: genişlik × (lento − denizlik); denizliği olmayanın yüksekliği lentodur. Niş ve baca duvardan düşülmez; derinlikleri ayrı."
                    .to_owned()
            }
            Tab::Slabs => q.slabs.first().map_or_else(
                || "Alan: halkaların alanı, delikler düşülür; boşluk: aynı kattaki boşluk mahallerinin örttüğü alan düşülür; hacim: net alan × kalınlık.".to_owned(),
                |r| format!("Kural: {}.", r.rule.trim_end_matches('.')),
            ),
            Tab::Transitions => "Kural: iki yanındaki odaların döşeme kaplaması bilinen ve farklı olan kapı ve döşemeye inen boşluklar; şerit geçişin genişliğidir.".to_owned(),
            Tab::Ramps => q.ramps.first().map_or_else(
                || "Alan: orta çizgi boyunca dönüşlerde gönyeli şeridin plan alanı; eğimli uzunluk: orta çizgi eksi sahanlıklar.".to_owned(),
                |r| format!("Kural: {}.", r.rule.trim_end_matches('.')),
            ),
            Tab::Stairs => q.stairs.first().map_or_else(
                || "Alan: yürüme çizgisi boyunca dönüşlerde gönyeli şeridin plan alanı; döner merdivende iki yayın arası; yükseklik: rıht sayısı × rıht yüksekliği.".to_owned(),
                |s| format!("Kural: {}; yükseklik: rıht sayısı × rıht yüksekliği.", s.rule.trim_end_matches('.')),
            ),
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
            Control::Press("Döşemeler") => Some(msg(Event::Tab(Tab::Slabs))),
            Control::Press("Geçişler") => Some(msg(Event::Tab(Tab::Transitions))),
            Control::Press("Rampalar") => Some(msg(Event::Tab(Tab::Ramps))),
            Control::Press("Merdivenler") => Some(msg(Event::Tab(Tab::Stairs))),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// A merge's decision in words: whose id and number go on, whose number, finishes and tags go with them, the fixtures.
fn merge_words(doc: &kentos_domain::Document, out: &kentos_commands::SpacesEdited) -> String {
    let Some(d) = out.decisions.first() else {
        return String::new();
    };
    let name = |uid: &str| -> String {
        kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|s| match doc.get(s) {
                Some(Entity::Space(x)) => {
                    let f = &x.space;
                    Some(match (f.number.as_deref(), f.name.as_deref().or(f.usage.as_deref())) {
                        (Some(n), Some(w)) => format!("“{n} {w}”"),
                        (Some(n), None) => format!("“{n}”"),
                        (None, Some(w)) => format!("“{w}”"),
                        (None, None) => "mahal".into(),
                    })
                }
                _ => None,
            })
            .unwrap_or_else(|| "mahal".into())
    };
    let by = match d.by {
        kentos_commands::SpaceKeptBy::Given => "verilen",
        kentos_commands::SpaceKeptBy::First => "ilk seçilen",
        kentos_commands::SpaceKeptBy::Largest => "en büyük parça",
    };
    let mut w = format!(
        "Kimliği sürecek mahal: {} ({by}); numarası {}, alanı {} m².",
        name(&d.kept),
        d.number.as_deref().unwrap_or("yok"),
        kentos_geometry_core::display::fixed(d.area, 2)
    );
    for p in &d.pieces {
        let gone = p.uid.as_deref().map_or_else(|| "parça".into(), name);
        let mut bits = vec![format!("numarası {}", p.number.as_deref().unwrap_or("yok"))];
        let f = &p.finishes;
        let finishes: Vec<&str> = [&f.floor, &f.base, &f.wall, &f.ceiling].into_iter().flatten().map(String::as_str).collect();
        if !finishes.is_empty() {
            bits.push(format!("kaplaması {}", finishes.join(", ")));
        }
        if !p.tags.is_empty() {
            bits.push(format!("{} etiketi bağsız kalır", p.tags.len()));
        }
        w.push_str(&format!(" Gidecek: {gone} ({}).", bits.join("; ")));
    }
    if !d.fixtures.is_empty() {
        w.push_str(&format!(" İçinde {} donatı kalır.", d.fixtures.len()));
    }
    w
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn a_stale_room_after_a_valid_room_is_visible_and_exported_with_its_warning() {
        let app = crate::files_testing::app_with_drawing();
        let room = |uid: &str, broken: Option<&str>| {
            serde_json::from_value(json!({
                "uid":uid,"number":uid,"name":"Salon","method":"net","area":12.,"perimeter":14.,
                "boundaryBreak":broken,"rule":"alan: halkaların alanı"
            }))
            .unwrap()
        };
        let mut q = SpacesQuantified {
            spaces: vec![room("M01", None), room("M02", Some("lostEnclosure"))],
            walls: vec![],
            openings: vec![],
            slabs: vec![],
            transitions: vec![],
            ramps: vec![],
            stairs: vec![],
            totals: Default::default(),
        };
        let [table, ..] = app.quantities_tables(&q);
        let status = table.heading.iter().position(|s| *s == "Durum").unwrap();
        assert!(table.shown[0][status].is_empty());
        assert!(
            table.shown[1][status].contains(kentos_commands::SpaceBreak::LostEnclosure.words())
        );
        assert_eq!(
            table.shown[1][status], table.cells[1][status],
            "copy/export lost the stale-area warning"
        );
        q.spaces[1].boundary_break = None;
        let [valid, ..] = app.quantities_tables(&q);
        assert_eq!(
            valid.heading,
            [
                "Numara",
                "Ad",
                "Kullanım",
                "Kat",
                "Yöntem",
                "Alan",
                "Çevre",
                "Süpürgelik",
                "Yükseklik",
                "Hacim",
                "Net",
                "Eksen",
                "Brüt"
            ]
        );
        assert!(
            valid.cells.iter().all(|r| r.len() == 13),
            "valid export shape changed"
        );
    }
}
