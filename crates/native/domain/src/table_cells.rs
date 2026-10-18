//! What keeps a table's computed cells (docs/adr/0327 §2–§4) before a step is recorded: a block schedule's rows from
//! the drawing's inserts, the cells bound to a field their source's value, then the formulas worked out. A table that
//! the step changed, or whose fields or schedule read the drawing, is written again; one that comes out the same is
//! left.

use std::sync::Arc;

use kentos_contracts::{
    DrawingFont, Entity, FieldBroken, FieldSource, TableEntity, TableSource, TextField,
    BROKEN_FIELD,
};
use kentos_geometry_core::api::json::{FromJson, Json};
use kentos_geometry_core::entity::Entity as CoreEntity;
use kentos_geometry_core::text::Font;
use crate::document::Document;
use crate::history::Op;
use crate::identity::Uuid;
use crate::store::Stored;

/// The words of a block schedule's last row's first cell.
pub const TOTAL: &str = "Toplam";

/// The kinds a schedule's cells read: its own and those its values come from.
fn schedule_kinds(of: kentos_contracts::ScheduleOf) -> &'static [&'static str] {
    use kentos_contracts::ScheduleOf;
    match of {
        ScheduleOf::Openings => &["opening", "wall"],
        ScheduleOf::Spaces => &["space"],
        ScheduleOf::Columns => &["column", "grid"],
        ScheduleOf::Walls => &["wall", "opening"],
    }
}

fn core(e: &Entity) -> Option<CoreEntity> {
    let json = serde_json::to_string(e).ok()?;
    CoreEntity::from_json(&Json::parse(&json).ok()?).ok()
}

impl Document {
    /// How many inserts of the block named `name` the drawing holds (its own, not those inside other blocks); none
    /// when it has no block so named.
    pub fn block_count(&self, name: &str) -> Option<usize> {
        let id = self.blocks().iter().find(|b| b.name == name)?.id;
        Some(
            self.entities()
                .filter(|e| matches!(e, Entity::Insert(i) if i.block == id))
                .count(),
        )
    }

    /// A block schedule's rows (docs/adr/0327 §4): a heading (Blok, the attribute, Adet), a row for each block named
    /// (none named: each block placed, in the drawing's order of definitions) and, with `by`, for each of its values
    /// (in code point order; an insert without it counts under the empty value), then the total.
    pub fn blocks_schedule(&self, names: &[String], by: Option<&str>) -> Vec<Vec<String>> {
        let defs: Vec<(Option<kentos_contracts::BlockId>, String)> = if names.is_empty() {
            self.blocks().iter().map(|b| (Some(b.id), b.name.clone())).collect()
        } else {
            names
                .iter()
                .map(|n| {
                    let id = self.blocks().iter().find(|b| &b.name == n).map(|b| b.id);
                    (id, n.clone())
                })
                .collect()
        };
        let mut head = vec!["Blok".to_owned()];
        if let Some(b) = by {
            head.push(b.to_owned());
        }
        head.push("Adet".to_owned());
        let mut rows = vec![head];
        let mut total = 0usize;
        for (id, name) in &defs {
            let inserts: Vec<&kentos_contracts::InsertEntity> = self
                .entities()
                .filter_map(|e| match e {
                    Entity::Insert(i) if Some(i.block) == *id => Some(i),
                    _ => None,
                })
                .collect();
            if names.is_empty() && inserts.is_empty() {
                continue;
            }
            match by {
                None => {
                    total += inserts.len();
                    rows.push(vec![name.clone(), inserts.len().to_string()]);
                }
                Some(tag) => {
                    let mut values: std::collections::BTreeMap<String, usize> = Default::default();
                    for i in &inserts {
                        let v = i.base.attrs.get(tag).cloned().unwrap_or_default();
                        *values.entry(v).or_default() += 1;
                    }
                    if values.is_empty() {
                        rows.push(vec![name.clone(), String::new(), "0".to_owned()]);
                    }
                    for (v, n) in values {
                        total += n;
                        rows.push(vec![name.clone(), v, n.to_string()]);
                    }
                }
            }
        }
        let mut last = vec![TOTAL.to_owned()];
        if by.is_some() {
            last.push(String::new());
        }
        last.push(total.to_string());
        rows.push(last);
        rows
    }

    /// The value a table cell's field writes now, as a text field's would (docs/adr/0327 §3).
    pub fn cell_field_value(
        &self,
        source: &FieldSource,
        format: Option<&kentos_contracts::FieldFormat>,
    ) -> Result<String, FieldBroken> {
        let letters = |u: Uuid| match self
            .store
            .slot_of(u)
            .and_then(|s| self.store.get(s))
            .map(|s| &*s.entity)
        {
            Some(Entity::Text(t)) => Some(t.text.clone()),
            _ => None,
        };
        let field = TextField {
            start: 0,
            end: 0,
            source: source.clone(),
            format: format.cloned(),
            broken: None,
        };
        self.field_value(&field, &letters)
    }

    /// `t` with its computed cells written again: a block schedule's rows (its rows and columns following them), its
    /// fields' values, then its formulas' results.
    pub fn table_worked_out(&self, t: &TableEntity) -> TableEntity {
        let mut out = t.clone();
        if let Some(TableSource::Blocks { names, by }) = &t.source {
            let rows = self.blocks_schedule(names, by.as_deref());
            self.take_rows(&mut out, rows);
        }
        // A schedule of the drawing's objects (docs/adr/0344 §3): its rows, members and look.
        if let Some(TableSource::Schedule {
            of,
            columns,
            filter,
            sort,
            group,
            total,
            ..
        }) = &t.source
        {
            let written = self.schedule_rows(*of, columns, filter.as_ref(), sort, *group, *total);
            self.take_rows(&mut out, written.rows);
            out.aligns = Some(written.aligns);
            out.header = true;
            out.styles = written.styles;
            out.formulas.clear();
            out.fields.clear();
            if let Some(TableSource::Schedule { members, .. }) = &mut out.source {
                *members = written.members;
            }
        }
        for i in 0..out.fields.len() {
            let f = &out.fields[i];
            let (words, broken) = match self.cell_field_value(&f.source, f.format.as_ref()) {
                Ok(v) => (v, None),
                Err(b) => (BROKEN_FIELD.to_owned(), Some(b)),
            };
            let (r, c) = (f.row as usize, f.col as usize);
            if let Some(cell) = out.cells.get_mut(r).and_then(|row| row.get_mut(c)) {
                *cell = words;
            }
            out.fields[i].broken = broken;
        }
        kentos_contracts::table_formula::with_formulas_worked_out(&out)
    }

    /// `rows` as `t`'s cells (a schedule's or a file's): a row kept keeps its height, a new one is `ROW` text heights
    /// high; a column kept is as wide as it was or as its words ask, whichever is wider (as Tabloyu güncelle, ADR 0184
    /// §6); what lies past them is dropped.
    pub fn take_rows(&self, t: &mut TableEntity, rows: Vec<Vec<String>>) {
        use kentos_geometry_core::ops::table::{LEAST, ROW};
        let n = rows.len().max(1);
        let m = rows.iter().map(Vec::len).max().unwrap_or(1).max(1);
        let h = t.height;
        let mut cells = rows;
        cells.resize(n, Vec::new());
        for r in &mut cells {
            r.resize(m, String::new());
        }
        t.rows = (0..n)
            .map(|i| t.rows.get(i).copied().unwrap_or(ROW * h))
            .collect();
        let old = std::mem::replace(&mut t.columns, vec![LEAST * h; m]);
        t.cells = cells;
        if t.aligns.as_ref().is_some_and(|a| a.len() != m) {
            t.aligns = None;
        }
        t.clip_to(n, m);
        let font = Font::from_id(DrawingFont::id_of(self.settings().drawing_font));
        let fitted = core(&Entity::Table(t.clone()))
            .and_then(|e| {
                kentos_geometry_core::geom::table::table_geom(&e.shape)
                    .map(|g| kentos_geometry_core::geom::table::fit_widths(&g, font, LEAST * h))
            })
            .unwrap_or_else(|| vec![LEAST * h; m]);
        t.columns = (0..m)
            .map(|j| {
                let f = fitted.get(j).copied().unwrap_or(LEAST * h);
                old.get(j).map_or(f, |&w| w.max(f))
            })
            .collect();
    }

    /// What keeps the tables' computed cells after `ops` (docs/adr/0327), applied: the step takes them in.
    pub(crate) fn follow_tables(&mut self, ops: &[Op]) -> Vec<Op> {
        if !self.store.has_computed_tables() || ops.is_empty() {
            return Vec::new();
        }
        let touched: std::collections::HashSet<Uuid> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Add(s) => Some(s.uid),
                Op::Update { after, .. } => Some(after.uid),
                _ => None,
            })
            .collect();
        let kinds: std::collections::HashSet<&'static str> = ops
            .iter()
            .filter_map(|op| match op {
                Op::Add(s) | Op::Remove(s) | Op::Update { after: s, .. } => Some(s.entity.kind()),
                _ => None,
            })
            .chain(ops.iter().filter_map(|op| match op {
                Op::Update { before, .. } => Some(before.entity.kind()),
                _ => None,
            }))
            .collect();
        let mut out = Vec::new();
        for slot in self.store.computed_table_slots().collect::<Vec<_>>() {
            let Some(current) = self.store.get(slot).cloned() else {
                continue;
            };
            let Entity::Table(t) = &*current.entity else {
                continue;
            };
            // A table only of formulas changes only with itself; fields and schedules read the drawing; a schedule of
            // the drawing's objects only the objects of its kinds (docs/adr/0344 §3).
            let reads_drawing = !t.fields.is_empty()
                || matches!(t.source, Some(TableSource::Blocks { .. }))
                || matches!(&t.source, Some(TableSource::Schedule { of, .. }) if schedule_kinds(*of).iter().any(|k| kinds.contains(k)));
            if !reads_drawing && !touched.contains(&current.uid) {
                continue;
            }
            let next = self.table_worked_out(t);
            if next == *t || next.problem().is_some() {
                continue;
            }
            let op = Op::Update {
                after: Stored {
                    uid: current.uid,
                    entity: Arc::new(Entity::Table(next)),
                },
                before: current,
            };
            self.apply_op(&op);
            out.push(op);
        }
        out
    }
}
