//! A schedule's rows (docs/adr/0344 §3), as the document writes them before a step is recorded.
//!
//! - Its objects: those of its kind in the drawing's order that its filter takes (an opening's level and phase are its
//!   wall's).
//! - Its cells: each column's value of each object (`arch_values`), written without units; empty when it has none.
//! - Its order: by its sort keys in turn, numbers by value, words in the natural order, the empty ones last whatever the
//!   direction; ties keep the drawing's order.
//! - Grouped: rows alike in every column that is not summed (count, area, perimeter, volume, length) become the first
//!   one's place, their summed columns added up (an empty value adds nothing; none at all stays empty).
//! - Its heading: each column's title, else its field's with its unit (`Genişlik (m)`, `Alan (m²)`, `Hacim (m³)`).
//! - Its headings and subtotals (docs/adr/0733 §4): a sort key with `header` puts its value before each group of rows
//!   alike in its column, across the row; one with `footer` puts “‹value› toplamı” and the group's sums after it (an
//!   inner key's before an outer one's). Their `members` are empty.
//! - Its total: the summed columns added up over its rows, `Toplam` in its first column unless that column is summed.
//! - Its look: the heading row, numbers to the right, a worked-out column's data cells in grey, the total in bold.
//!
//! `scripts/fixtures/tag_command_cases.py` writes the same without KentOS's code.

use std::cmp::Ordering;

use kentos_contracts::{
    CellRange, CellStyle, Entity, OpeningForm, ScheduleColumn, ScheduleField, ScheduleFilter,
    ScheduleOf, ScheduleSort, TableAlign,
};
use kentos_geometry_core::text::natural::natural_cmp;

use crate::arch_values::ArchValue;
use crate::document::Document;
use crate::identity::Slot;

/// The words of a schedule's total row's first cell.
pub const SCHEDULE_TOTAL: &str = "Toplam";
/// What a heading or a subtotal says for rows with no value in its column.
pub const SCHEDULE_EMPTY: &str = "(boş)";
/// A worked-out column's data cells' colour.
pub const COMPUTED_COLOR: &str = "#7f8790";

/// A schedule written out.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ScheduleRows {
    /// Its heading, its data rows and its total, if any.
    pub rows: Vec<Vec<String>>,
    /// Each data row's objects, by their persistent ids.
    pub members: Vec<Vec<String>>,
    pub aligns: Vec<TableAlign>,
    pub styles: Vec<CellStyle>,
    /// Its headings, each across its row.
    pub merges: Vec<CellRange>,
    /// Each row's numbers in the unit shown, before they are rounded to the project's decimals (`file.export.table`'s
    /// exact numbers, docs/adr/0733 §7); none for words.
    pub numbers: Vec<Vec<Option<f64>>>,
}

/// Whether a field's cells are numbers, written to the right.
pub fn numeric(f: ScheduleField) -> bool {
    f.length()
        || matches!(
            f,
            ScheduleField::Count
                | ScheduleField::Area
                | ScheduleField::Volume
                | ScheduleField::Share
                | ScheduleField::Gross
        )
}

struct Row {
    values: Vec<Option<ArchValue>>,
    texts: Vec<String>,
    uids: Vec<String>,
}

fn by_value(a: &Option<ArchValue>, at: &str, b: &Option<ArchValue>, bt: &str) -> Ordering {
    match (
        a.as_ref().and_then(ArchValue::number),
        b.as_ref().and_then(ArchValue::number),
    ) {
        (Some(x), Some(y)) => x.partial_cmp(&y).unwrap_or(Ordering::Equal),
        _ => natural_cmp(at, bt),
    }
}

impl Document {
    /// Whether `e` (of the schedule's kind) is one its filter takes.
    fn schedule_takes(&self, e: &Entity, filter: Option<&ScheduleFilter>) -> bool {
        let Some(f) = filter else { return true };
        let arch = match e {
            Entity::Opening(o) => self.tag_target(&o.opening.host).and_then(|w| match w {
                Entity::Wall(w) => Some(w.wall.arch),
                _ => None,
            }),
            Entity::Space(s) => Some(s.space.arch.clone()),
            Entity::Column(c) => Some(c.column.arch.clone()),
            Entity::Wall(w) => Some(w.wall.arch.clone()),
            Entity::Insert(i) => Some(i.arch.clone()),
            _ => None,
        }
        .unwrap_or_default();
        // Its type (docs/adr/0733 §4): an opening's type, a wall's style, a column's kind, a space's usage, a fixture's
        // block name, as the schedule writes them.
        let kind_of = |e: &Entity| -> Option<String> {
            let place = if matches!(e, Entity::Space(_)) { "usage" } else { "type" };
            self.arch_value(e, place, None).map(|v| self.arch_value_text(&v, false))
        };
        f.layers
            .as_ref()
            .is_none_or(|ls| ls.contains(&e.base().layer_id))
            && f.levels
                .as_ref()
                .is_none_or(|ls| arch.level.as_ref().is_some_and(|l| ls.contains(l)))
            && f.phases
                .as_ref()
                .is_none_or(|ps| arch.phase.is_some_and(|p| ps.contains(&p)))
            && f.forms.as_ref().is_none_or(|fs| match e {
                Entity::Opening(o) => fs.contains(&o.opening.form),
                _ => false,
            })
            && f.types
                .as_ref()
                .is_none_or(|ts| kind_of(e).is_some_and(|k| ts.contains(&k)))
    }

    /// Whether an insert is a fixture a schedule lists: not an external reference's or an underlay's.
    fn fixture(&self, e: &Entity) -> bool {
        match e {
            Entity::Insert(i) => self
                .blocks()
                .iter()
                .find(|b| b.id == i.block)
                .is_some_and(|b| b.xref.is_none()),
            _ => true,
        }
    }

    /// A column's heading: its title, else its field's with its unit.
    pub fn schedule_heading(&self, c: &ScheduleColumn) -> String {
        if c.title.is_some() || c.field == ScheduleField::Attr {
            return c.heading();
        }
        let s = self.settings();
        let unit = if c.field.length() {
            Some(crate::text_fields::unit_name(
                s.drawing_unit.unwrap_or_default(),
            ))
        } else {
            match c.field {
                ScheduleField::Area | ScheduleField::Share | ScheduleField::Gross => Some(match s.area_unit {
                    kentos_contracts::AreaUnit::M2 => "m²",
                    kentos_contracts::AreaUnit::Donum => "dönüm",
                    kentos_contracts::AreaUnit::Ha => "ha",
                }),
                ScheduleField::Volume => Some("m³"),
                _ => None,
            }
        };
        match unit {
            Some(u) => format!("{} ({u})", c.field.title()),
            None => c.field.title().to_owned(),
        }
    }

    /// A schedule's rows from the drawing (the module's rule).
    pub fn schedule_rows(
        &self,
        of: ScheduleOf,
        columns: &[ScheduleColumn],
        filter: Option<&ScheduleFilter>,
        sort: &[ScheduleSort],
        group: bool,
        total: bool,
    ) -> ScheduleRows {
        let mut rows: Vec<Row> = Vec::new();
        // The shares are the whole program's: worked out once for the schedule (docs/adr/0784 §3).
        let shares = columns
            .iter()
            .any(|c| c.field.is_share())
            .then(|| self.program_shares().map(|(_, parts)| parts))
            .flatten();
        let share_of = |e: &Entity, field: ScheduleField| -> Option<ArchValue> {
            let Entity::Space(x) = e else { return None };
            let unit = x.space.unit.as_deref()?;
            let (_, part) = shares.as_ref()?.iter().find(|(u, _)| u == unit)?;
            let t = if field == ScheduleField::Share { &part.share } else { &part.gross };
            t.parse::<f64>().ok().map(ArchValue::Area)
        };
        for e in self.entities() {
            if e.kind() != of.kind() || !self.fixture(e) || !self.schedule_takes(e, filter) {
                continue;
            }
            let values: Vec<Option<ArchValue>> = columns
                .iter()
                .map(|c| {
                    if c.field.is_share() {
                        share_of(e, c.field)
                    } else {
                        self.arch_value(e, c.field.name(), c.attr.as_deref())
                    }
                })
                .collect();
            let texts = values
                .iter()
                .map(|v| {
                    v.as_ref()
                        .map(|v| self.arch_value_text(v, false))
                        .unwrap_or_default()
                })
                .collect();
            let uid = self
                .uid(Slot(e.base().id))
                .map(|u| u.to_string())
                .unwrap_or_default();
            rows.push(Row {
                values,
                texts,
                uids: vec![uid],
            });
        }
        rows.sort_by(|a, b| {
            for k in sort {
                let j = k.column as usize;
                let (ea, eb) = (a.texts[j].is_empty(), b.texts[j].is_empty());
                let o = match (ea, eb) {
                    (true, true) => Ordering::Equal,
                    (true, false) => Ordering::Greater,
                    (false, true) => Ordering::Less,
                    _ => {
                        let o = by_value(&a.values[j], &a.texts[j], &b.values[j], &b.texts[j]);
                        if k.descending { o.reverse() } else { o }
                    }
                };
                if o != Ordering::Equal {
                    return o;
                }
            }
            Ordering::Equal
        });
        let summed: Vec<bool> = columns.iter().map(|c| c.field.summed()).collect();
        if group {
            let mut merged: Vec<Row> = Vec::new();
            for r in rows {
                let key = |x: &Row| -> Vec<String> {
                    x.texts
                        .iter()
                        .zip(&summed)
                        .filter(|(_, s)| !**s)
                        .map(|(t, _)| t.clone())
                        .collect()
                };
                match merged.iter_mut().find(|m| key(m) == key(&r)) {
                    Some(m) => {
                        for (j, _) in summed.iter().enumerate().filter(|(_, s)| **s) {
                            m.values[j] = add(&m.values[j], &r.values[j]);
                        }
                        m.uids.extend(r.uids);
                    }
                    None => merged.push(r),
                }
            }
            for m in &mut merged {
                for (j, _) in summed.iter().enumerate().filter(|(_, s)| **s) {
                    m.texts[j] = m.values[j]
                        .as_ref()
                        .map(|v| self.arch_value_text(v, false))
                        .unwrap_or_default();
                }
            }
            rows = merged;
        }
        let width = columns.len();
        let heading: Vec<String> = columns.iter().map(|c| self.schedule_heading(c)).collect();
        let mut out = ScheduleRows {
            rows: vec![heading],
            members: Vec::with_capacity(rows.len()),
            aligns: columns
                .iter()
                .map(|c| {
                    if numeric(c.field) {
                        TableAlign::Right
                    } else {
                        TableAlign::Left
                    }
                })
                .collect(),
            styles: Vec::new(),
            merges: Vec::new(),
            numbers: vec![vec![None; width]],
        };
        let numbers = |values: &[Option<ArchValue>]| -> Vec<Option<f64>> {
            values.iter().map(|v| v.as_ref().and_then(|v| self.arch_value_number(v))).collect()
        };
        let bold = |row: usize| CellStyle {
            range: CellRange {
                row: row as u32,
                col: 0,
                rows: 1,
                cols: width as u32,
            },
            align: None,
            bold: Some(true),
            color: None,
            fill: None,
            decimals: None,
        };
        // The sort keys that put a heading or a subtotal, outermost first (docs/adr/0733 §4).
        let marked: Vec<&ScheduleSort> = sort.iter().filter(|k| k.header || k.footer).collect();
        let footer = |d: usize, sums: &[Option<ArchValue>], value: &str| -> Vec<String> {
            let mut cells: Vec<String> = sums
                .iter()
                .zip(&summed)
                .map(|(v, s)| match (s, v) {
                    (true, Some(v)) => self.arch_value_text(v, false),
                    _ => String::new(),
                })
                .collect();
            for (j, c) in columns.iter().enumerate() {
                if c.field == ScheduleField::Count && cells[j].is_empty() {
                    cells[j] = "0".into();
                }
            }
            let k = marked[d].column as usize;
            let label = if summed[k] { (0..width).find(|j| !summed[*j]) } else { Some(k) };
            if let Some(j) = label {
                let v = if value.is_empty() { SCHEDULE_EMPTY } else { value };
                cells[j] = format!("{v} toplamı");
            }
            cells
        };
        let mut sums: Vec<Option<ArchValue>> = vec![None; width];
        let mut group_sums: Vec<Vec<Option<ArchValue>>> = vec![vec![None; width]; marked.len()];
        let mut group_values: Vec<String> = vec![String::new(); marked.len()];
        let mut prev: Option<Vec<String>> = None;
        // The runs of data rows: their first row and how many (a computed column's grey covers them alone).
        let mut runs: Vec<(usize, usize)> = Vec::new();
        for r in rows {
            let keys: Vec<String> = marked
                .iter()
                .map(|k| r.texts[k.column as usize].clone())
                .collect();
            // The outermost key whose value changes: its group and those inside it close, then open.
            let from = prev.as_ref().map_or(0, |p| {
                p.iter()
                    .zip(&keys)
                    .position(|(a, b)| a != b)
                    .unwrap_or(marked.len())
            });
            if prev.is_some() {
                for d in (from..marked.len()).rev() {
                    if marked[d].footer {
                        let cells = footer(d, &group_sums[d], &group_values[d]);
                        out.styles.push(bold(out.rows.len()));
                        out.rows.push(cells);
                        out.members.push(Vec::new());
                        out.numbers.push(numbers(&group_sums[d]));
                    }
                }
            }
            for d in from..marked.len() {
                group_sums[d] = vec![None; width];
                group_values[d] = keys[d].clone();
                if marked[d].header {
                    let at = out.rows.len();
                    let mut cells = vec![String::new(); width];
                    cells[0] = if keys[d].is_empty() {
                        SCHEDULE_EMPTY.to_owned()
                    } else {
                        keys[d].clone()
                    };
                    if width > 1 {
                        out.merges.push(CellRange {
                            row: at as u32,
                            col: 0,
                            rows: 1,
                            cols: width as u32,
                        });
                    }
                    out.styles.push(bold(at));
                    out.rows.push(cells);
                    out.members.push(Vec::new());
                    out.numbers.push(vec![None; width]);
                }
            }
            for j in 0..width {
                if summed[j] {
                    sums[j] = add(&sums[j], &r.values[j]);
                    for g in &mut group_sums {
                        g[j] = add(&g[j], &r.values[j]);
                    }
                }
            }
            let at = out.rows.len();
            match runs.last_mut() {
                Some((first, n)) if *first + *n == at => *n += 1,
                _ => runs.push((at, 1)),
            }
            out.numbers.push(numbers(&r.values));
            out.rows.push(r.texts);
            out.members.push(r.uids);
            prev = Some(keys);
        }
        if prev.is_some() {
            for d in (0..marked.len()).rev() {
                if marked[d].footer {
                    let cells = footer(d, &group_sums[d], &group_values[d]);
                    out.styles.push(bold(out.rows.len()));
                    out.rows.push(cells);
                    out.members.push(Vec::new());
                    out.numbers.push(numbers(&group_sums[d]));
                }
            }
        }
        for &(first, n) in &runs {
            for (j, c) in columns.iter().enumerate() {
                if !c.field.edits_in(of) {
                    out.styles.push(CellStyle {
                        range: CellRange {
                            row: first as u32,
                            col: j as u32,
                            rows: n as u32,
                            cols: 1,
                        },
                        align: None,
                        bold: None,
                        color: Some(COMPUTED_COLOR.to_owned()),
                        fill: None,
                        decimals: None,
                    });
                }
            }
        }
        if total {
            let mut last: Vec<String> = sums
                .iter()
                .zip(&summed)
                .map(|(v, s)| match (s, v) {
                    (true, Some(v)) => self.arch_value_text(v, false),
                    _ => String::new(),
                })
                .collect();
            for (j, c) in columns.iter().enumerate() {
                if c.field == ScheduleField::Count && last[j].is_empty() {
                    last[j] = "0".into();
                }
            }
            if !summed[0] {
                last[0] = SCHEDULE_TOTAL.to_owned();
            }
            out.styles.push(bold(out.rows.len()));
            out.rows.push(last);
            out.numbers.push(numbers(&sums));
        }
        out
    }
}

/// Two values added: the first one's kind; an empty one adds nothing.
fn add(a: &Option<ArchValue>, b: &Option<ArchValue>) -> Option<ArchValue> {
    use ArchValue::*;
    match (a, b) {
        (None, x) | (x, None) => x.clone(),
        (Some(Length(x)), Some(y)) => y.number().map(|y| Length(x + y)),
        (Some(Area(x)), Some(y)) => y.number().map(|y| Area(x + y)),
        (Some(Volume(x)), Some(y)) => y.number().map(|y| Volume(x + y)),
        (Some(Count(x)), Some(Count(y))) => Some(Count(x + y)),
        (Some(x), _) => Some(x.clone()),
    }
}

/// An openings filter's forms as words (for the window's summary).
pub fn forms_label(forms: &[OpeningForm]) -> String {
    forms
        .iter()
        .map(|f| f.label())
        .collect::<Vec<_>>()
        .join(", ")
}
