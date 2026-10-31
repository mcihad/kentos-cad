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
        // A baseboard reads the doorways (docs/adr/0733 §3).
        ScheduleOf::Spaces => &["space", "opening"],
        ScheduleOf::Columns => &["column", "grid"],
        ScheduleOf::Walls => &["wall", "opening"],
        // A fixture's space (docs/adr/0733 §2).
        ScheduleOf::Fixtures => &["insert", "space"],
    }
}

/// Whether two tables are the same but for the object they are and where they stand (`base`, `p`, `rotation`): what a
/// schedule's rows, widths and look are worked out from (docs/adr/0782 §3).
fn alike_but_placed(a: &TableEntity, b: &TableEntity) -> bool {
    let TableEntity {
        base: _,
        p: _,
        rotation: _,
        height,
        rows,
        columns,
        cells,
        merges,
        aligns,
        header,
        grid,
        frame,
        face,
        source,
        styles,
        formulas,
        fields,
    } = a;
    *height == b.height
        && *header == b.header
        && *frame == b.frame
        && *grid == b.grid
        && *face == b.face
        && *aligns == b.aligns
        && *rows == b.rows
        && *columns == b.columns
        && *merges == b.merges
        && *styles == b.styles
        && *formulas == b.formulas
        && *fields == b.fields
        && *source == b.source
        && *cells == b.cells
}

/// `worked` (another table's working out, alike but placed) as `t`'s: `t`'s own object and place.
fn placed_as(worked: &TableEntity, t: &TableEntity) -> TableEntity {
    let mut next = worked.clone();
    next.base = t.base.clone();
    next.p = t.p;
    next.rotation = t.rotation;
    next
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

    /// A block schedule's rows (docs/adr/0327 §4): a heading (Blok, the attribute, the further columns, Adet), a row
    /// for each block named (none named: each block placed, in the drawing's order of definitions) and, with `by`, for
    /// each of its values (in code point order; an insert without it counts under the empty value), then the total.
    /// A further column (docs/adr/0721 §4) holds its row's inserts' distinct values that are not empty, in the
    /// drawing's order, joined with “; ”; with them a schedule of one block named leaves out its Blok column.
    pub fn blocks_schedule(&self, names: &[String], by: Option<&str>, columns: &[String]) -> Vec<Vec<String>> {
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
        let named = columns.is_empty() || names.len() != 1;
        let mut head = Vec::new();
        if named {
            head.push("Blok".to_owned());
        }
        if let Some(b) = by {
            head.push(b.to_owned());
        }
        head.extend(columns.iter().cloned());
        head.push("Adet".to_owned());
        let mut rows = vec![head];
        let mut total = 0usize;
        // A row: its block's name, its value of `by`, its further cells, its count.
        let row = |name: &str, value: Option<String>, of: &[&kentos_contracts::InsertEntity]| {
            let mut r = Vec::new();
            if named {
                r.push(name.to_owned());
            }
            if let Some(v) = value {
                r.push(v);
            }
            for c in columns {
                let mut seen: Vec<&str> = Vec::new();
                for i in of {
                    if let Some(v) = i.base.attrs.get(c).map(String::as_str).filter(|v| !v.is_empty())
                        && !seen.contains(&v)
                    {
                        seen.push(v);
                    }
                }
                r.push(seen.join("; "));
            }
            r.push(of.len().to_string());
            r
        };
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
                    rows.push(row(name, None, &inserts));
                }
                Some(tag) => {
                    let mut values: std::collections::BTreeMap<String, Vec<&kentos_contracts::InsertEntity>> =
                        Default::default();
                    for i in &inserts {
                        let v = i.base.attrs.get(tag).cloned().unwrap_or_default();
                        values.entry(v).or_default().push(i);
                    }
                    if values.is_empty() {
                        rows.push(row(name, Some(String::new()), &[]));
                    }
                    for (v, of) in values {
                        total += of.len();
                        rows.push(row(name, Some(v), &of));
                    }
                }
            }
        }
        let mut last = vec![TOTAL.to_owned()];
        let blanks = usize::from(named) + usize::from(by.is_some()) + columns.len();
        // "Toplam" takes the first cell; the rest of the cells before the count are empty.
        last.extend(std::iter::repeat_n(String::new(), blanks.saturating_sub(1)));
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
        if let Some(TableSource::Blocks { names, by, columns }) = &t.source {
            let rows = self.blocks_schedule(names, by.as_deref(), columns);
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
            // Its headings across their rows (docs/adr/0733 §4); a schedule has no merge of its own.
            out.merges = written.merges;
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
        // Another primary design option, or objects into or out of one (docs/adr/0788 §5): every schedule reads anew.
        let options_changed = ops.iter().any(|op| matches!(op, Op::ArchOptions { .. }));
        let mut out = Vec::new();
        // Schedules alike but for where they stand work out alike (docs/adr/0782 §3): each is worked out once a step.
        let mut alike: Vec<(Arc<Entity>, Option<TableEntity>)> = Vec::new();
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
                || matches!(&t.source, Some(TableSource::Schedule { of, .. }) if options_changed || schedule_kinds(*of).iter().any(|k| kinds.contains(k)));
            if !reads_drawing && !touched.contains(&current.uid) {
                continue;
            }
            let schedule = matches!(t.source, Some(TableSource::Schedule { .. })) && t.fields.is_empty();
            let seen = schedule
                .then(|| {
                    alike.iter().find(|(e, _)| matches!(&**e, Entity::Table(a) if alike_but_placed(a, t)))
                })
                .flatten();
            let next = match seen {
                Some((_, None)) => continue,
                Some((_, Some(n))) => placed_as(n, t),
                None => {
                    let next = self.table_worked_out(t);
                    let kept = next == *t || next.problem().is_some();
                    if schedule {
                        alike.push((Arc::clone(&current.entity), (!kept).then(|| next.clone())));
                    }
                    if kept {
                        continue;
                    }
                    next
                }
            };
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
