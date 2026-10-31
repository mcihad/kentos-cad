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
//! - A units schedule (docs/adr/0795): the building program's groups in its order, a row each (alike ones one row
//!   when grouped), its members the group's spaces in the drawing's order and its `units` the group's id. Its cells:
//!   the number, the name, the kind, the building's and the wing's names, the levels its spaces stand on (the project's
//!   order, joined with “, ”), its rooms' area (`Document::program_rooms`: no voids, no outdoor spaces; empty without
//!   spaces), the recorded area, the share weight as written, the share and the gross (`kentos.share/1`, an independent
//!   unit's), the summed brief of its briefed rooms and how far their area lies outside it, its spaces' count. Its
//!   filter takes a group with a space on one of its levels, of one of its buildings, wings or kinds. Its share, gross
//!   and recorded area are summed with its count and area.
//!
//! `scripts/fixtures/tag_command_cases.py` writes the same without KentOS's code; a units schedule's,
//! `scripts/fixtures/unit_schedule_cases.py`.

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
    /// A units schedule's (docs/adr/0795): each row's program groups, beside `members`; empty for another's.
    pub units: Vec<Vec<String>>,
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
                | ScheduleField::RecordedArea
        )
}

struct Row {
    values: Vec<Option<ArchValue>>,
    texts: Vec<String>,
    uids: Vec<String>,
    /// A units schedule's row's groups.
    units: Vec<String>,
}

/// A range's words as a space's target writes them (docs/adr/0773 §2): “lo–hi m²”, “≥ lo m²”, “≤ hi m²”.
fn range_words(doc: &Document, lo: Option<f64>, hi: Option<f64>) -> Option<ArchValue> {
    let t = |v: f64, unit: bool| doc.arch_value_text(&ArchValue::Area(v), unit);
    match (lo, hi) {
        (Some(lo), Some(hi)) => Some(ArchValue::Text(format!("{}–{}", t(lo, false), t(hi, true)))),
        (Some(lo), None) => Some(ArchValue::Text(format!("≥ {}", t(lo, true)))),
        (None, Some(hi)) => Some(ArchValue::Text(format!("≤ {}", t(hi, true)))),
        (None, None) => None,
    }
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

    /// A column's heading in a schedule of `of`: a units schedule's count is its spaces' (docs/adr/0795).
    pub fn schedule_heading_in(&self, of: ScheduleOf, c: &ScheduleColumn) -> String {
        if of == ScheduleOf::Units && c.title.is_none() && c.field == ScheduleField::Count {
            return "Mahal sayısı".to_owned();
        }
        self.schedule_heading(c)
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
                ScheduleField::Area
                | ScheduleField::Share
                | ScheduleField::Gross
                | ScheduleField::RecordedArea => Some(match s.area_unit {
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

    /// A units schedule's rows (docs/adr/0795): the program's groups its filter takes, in the program's order.
    fn unit_rows(&self, columns: &[ScheduleColumn], filter: Option<&ScheduleFilter>) -> Vec<Row> {
        use kentos_contracts::UnitKind;
        let program = self.program();
        let rooms = self.program_rooms(&program);
        let shares = self.program_shares().map(|(_, parts)| parts);
        let levels: Vec<(String, String)> = self
            .settings()
            .arch
            .as_ref()
            .map(|a| a.levels.iter().map(|l| (l.id.clone(), l.name.clone())).collect())
            .unwrap_or_default();
        // Each group's spaces in the drawing's order: their uids, levels and briefed rooms.
        struct Of {
            uids: Vec<String>,
            levels: Vec<String>,
            lo: Option<f64>,
            hi: Option<f64>,
            briefed: f64,
            briefs: usize,
            open_lo: bool,
            open_hi: bool,
        }
        let mut of: Vec<(String, Of)> = Vec::new();
        for e in self.entities() {
            let Entity::Space(s) = e else { continue };
            let x = &s.space;
            let Some(unit) = x.unit.as_deref().filter(|u| program.unit(u).is_some()) else { continue };
            let i = match of.iter().position(|(u, _)| u == unit) {
                Some(i) => i,
                None => {
                    of.push((
                        unit.to_owned(),
                        Of {
                            uids: Vec::new(),
                            levels: Vec::new(),
                            lo: Some(0.0),
                            hi: Some(0.0),
                            briefed: 0.0,
                            briefs: 0,
                            open_lo: false,
                            open_hi: false,
                        },
                    ));
                    of.len() - 1
                }
            };
            let g = &mut of[i].1;
            g.uids.push(self.uid(Slot(s.base.id)).map(|u| u.to_string()).unwrap_or_default());
            if let Some(l) = x.arch.level.as_ref()
                && !g.levels.contains(l)
            {
                g.levels.push(l.clone());
            }
            // Its briefed rooms (not voids, not outdoor, a brief that gives a range): their ranges summed.
            let brief = x.usage.as_deref().and_then(|u| program.brief(u));
            if let Some(b) = brief.filter(|b| !x.void && !b.outdoor && (b.area_min.is_some() || b.area_max.is_some())) {
                g.briefs += 1;
                g.briefed += kentos_geometry_core::geom::space::area(&crate::arch_values::core_areas(&x.parts));
                match b.area_min {
                    Some(v) => g.lo = g.lo.map(|lo| lo + v),
                    None => g.open_lo = true,
                }
                match b.area_max {
                    Some(v) => g.hi = g.hi.map(|hi| hi + v),
                    None => g.open_hi = true,
                }
            }
        }
        let mut rows = Vec::new();
        for u in &program.units {
            let mine = of.iter().find(|(id, _)| *id == u.id).map(|(_, g)| g);
            if let Some(f) = filter {
                let lv = f.levels.as_ref().is_none_or(|ls| mine.is_some_and(|g| g.levels.iter().any(|l| ls.contains(l))));
                let bd = f.buildings.as_ref().is_none_or(|bs| u.building.as_ref().is_some_and(|b| bs.contains(b)));
                let wg = f.wings.as_ref().is_none_or(|ws| u.wing.as_ref().is_some_and(|w| ws.contains(w)));
                let kd = f.kinds.as_ref().is_none_or(|ks| ks.contains(&u.kind));
                if !(lv && bd && wg && kd) {
                    continue;
                }
            }
            let part = shares.as_ref().and_then(|p| p.iter().find(|(id, _)| *id == u.id)).map(|(_, p)| p);
            let area_of = |t: &str| t.parse::<f64>().ok().map(ArchValue::Area);
            let building = u.building.as_deref().and_then(|b| program.building(b));
            let values: Vec<Option<ArchValue>> = columns
                .iter()
                .map(|c| match c.field {
                    ScheduleField::Number => Some(ArchValue::Text(u.number.clone())),
                    ScheduleField::Name => u.name.clone().map(ArchValue::Text),
                    ScheduleField::UnitKind => Some(ArchValue::Text(u.kind.label().to_owned())),
                    ScheduleField::Building => building.map(|b| ArchValue::Text(b.name.clone())),
                    ScheduleField::Wing => u
                        .wing
                        .as_deref()
                        .and_then(|w| building.and_then(|b| b.wings.iter().find(|x| x.id == w)))
                        .map(|w| ArchValue::Text(w.name.clone())),
                    ScheduleField::Level => mine.and_then(|g| {
                        let names: Vec<String> = levels
                            .iter()
                            .filter(|(id, _)| g.levels.contains(id))
                            .map(|(_, n)| n.clone())
                            .chain(g.levels.iter().filter(|l| !levels.iter().any(|(id, _)| id == *l)).cloned())
                            .collect();
                        (!names.is_empty()).then(|| ArchValue::Text(names.join(", ")))
                    }),
                    ScheduleField::Area => mine.map(|_| {
                        ArchValue::Area(rooms.iter().find(|(id, _)| *id == u.id).map_or(0.0, |(_, a)| *a))
                    }),
                    ScheduleField::RecordedArea => u.recorded_area.map(ArchValue::Area),
                    ScheduleField::ShareWeight => u.share_weight.clone().map(ArchValue::Text),
                    ScheduleField::Share if u.kind == UnitKind::Unit => part.and_then(|p| area_of(&p.share)),
                    ScheduleField::Gross if u.kind == UnitKind::Unit => part.and_then(|p| area_of(&p.gross)),
                    ScheduleField::Target => mine
                        .filter(|g| g.briefs > 0)
                        .and_then(|g| range_words(self, g.lo.filter(|_| !g.open_lo), g.hi.filter(|_| !g.open_hi))),
                    ScheduleField::TargetDiff => mine.filter(|g| g.briefs > 0).and_then(|g| {
                        let brief = kentos_contracts::SpaceBrief {
                            area_min: g.lo.filter(|_| !g.open_lo),
                            area_max: g.hi.filter(|_| !g.open_hi),
                            ..Default::default()
                        };
                        brief.area_difference(g.briefed).map(ArchValue::Area)
                    }),
                    ScheduleField::Count => Some(ArchValue::Count(mine.map_or(0, |g| g.uids.len() as u64))),
                    _ => None,
                })
                .collect();
            let texts = values
                .iter()
                .map(|v| v.as_ref().map(|v| self.arch_value_text(v, false)).unwrap_or_default())
                .collect();
            rows.push(Row {
                values,
                texts,
                uids: mine.map(|g| g.uids.clone()).unwrap_or_default(),
                units: vec![u.id.clone()],
            });
        }
        rows
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
        let shares = (of != ScheduleOf::Units && columns.iter().any(|c| c.field.is_share()))
            .then(|| self.program_shares().map(|(_, parts)| parts))
            .flatten();
        let share_of = |e: &Entity, field: ScheduleField| -> Option<ArchValue> {
            let Entity::Space(x) = e else { return None };
            let unit = x.space.unit.as_deref()?;
            let (_, part) = shares.as_ref()?.iter().find(|(u, _)| u == unit)?;
            let t = if field == ScheduleField::Share { &part.share } else { &part.gross };
            t.parse::<f64>().ok().map(ArchValue::Area)
        };
        // The options a set does not show are not built (docs/adr/0788 §5): a schedule lists the primaries'.
        let out = self.option_out();
        // A units schedule's rows read the whole program, as its shares do (docs/adr/0795 §1; 0784 §2).
        if of == ScheduleOf::Units {
            rows = self.unit_rows(columns, filter);
        }
        for e in self.entities() {
            if of == ScheduleOf::Units {
                break;
            }
            if e.kind() != of.kind()
                || !self.fixture(e)
                || !self.schedule_takes(e, filter)
                || out.contains(&Slot(e.base().id))
            {
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
                units: Vec::new(),
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
        let summed: Vec<bool> = columns.iter().map(|c| c.field.summed_in(of)).collect();
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
                        m.units.extend(r.units);
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
        let heading: Vec<String> = columns.iter().map(|c| self.schedule_heading_in(of, c)).collect();
        let units = of == ScheduleOf::Units;
        let mut out = ScheduleRows {
            rows: vec![heading],
            members: Vec::with_capacity(rows.len()),
            units: Vec::new(),
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
                        if units {
                            out.units.push(Vec::new());
                        }
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
                    if units {
                        out.units.push(Vec::new());
                    }
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
            if units {
                out.units.push(r.units);
            }
            prev = Some(keys);
        }
        if prev.is_some() {
            for d in (0..marked.len()).rev() {
                if marked[d].footer {
                    let cells = footer(d, &group_sums[d], &group_values[d]);
                    out.styles.push(bold(out.rows.len()));
                    out.rows.push(cells);
                    out.members.push(Vec::new());
                    if units {
                        out.units.push(Vec::new());
                    }
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
