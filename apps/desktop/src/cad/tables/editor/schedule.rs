//! A schedule's cells in Tabloyu düzenle (docs/adr/0733 §6, usage review UR-058): what a cell is (written to its
//! row's objects, or read only and why) and the schedule's draft worked out on a copy of the drawing.

#[allow(unused_imports)]
use super::*;

/// What a schedule's cell at `at` is (docs/adr/0733 §6): written to its row's objects (the words say which), or read
/// only and why; none for another table.
pub(crate) fn schedule_cell(
    doc: &kentos_domain::Document,
    t: &kentos_commands::TableEntity,
    at: (usize, usize),
) -> Option<Result<String, String>> {
    let Some(kentos_commands::TableSource::Schedule {
        of,
        columns,
        total,
        members,
        units,
        ..
    }) = &t.source
    else {
        return None;
    };
    let (r, c) = at;
    // A units schedule's row is its program groups (docs/adr/0795).
    let groups = (*of == kentos_commands::ScheduleOf::Units)
        .then(|| units.get(r.wrapping_sub(1)).filter(|g| !g.is_empty()));
    let column = columns.get(c)?;
    if r == 0 {
        return Some(Err(
            "Başlık satırı: sütunun adı belgenin yazdığıdır; düzenlenmez.".into(),
        ));
    }
    let objects = members.get(r - 1).filter(|m| !m.is_empty());
    if let Some(Some(groups)) = groups {
        if !kentos_native_application::schedules::editable(*of, column.field) {
            return Some(Err(format!(
                "Hesaplanır: “{}” bina programından ve mahallerden okunur; düzenlenmez.",
                doc.schedule_heading_in(*of, column)
            )));
        }
        let program = doc.program();
        let names: Vec<String> = groups
            .iter()
            .filter_map(|g| program.unit(g))
            .map(|u| match &u.name {
                Some(n) => format!("{} {n}", u.number),
                None => u.number.clone(),
            })
            .collect();
        return Some(Ok(format!(
            "Bina programına yazılır: {}. Kaydet tek adımda yazar; çizelge ve etiketler izler.",
            names.join(", ")
        )));
    }
    let Some(objects) = objects.filter(|_| groups.is_none()) else {
        let what = if *total && r + 1 == t.cells.len() {
            "Genel toplam"
        } else {
            "Grup başlığı ya da ara toplam"
        };
        return Some(Err(format!(
            "{what}: nesnelerin satırı değil; düzenlenmez."
        )));
    };
    if !kentos_native_application::schedules::editable(*of, column.field) {
        return Some(Err(format!(
            "Hesaplanır: “{}” çizimden okunur; düzenlenmez. Kaynağın kendi alanını değiştirin.",
            doc.schedule_heading(column)
        )));
    }
    let names: Vec<String> = objects
        .iter()
        .filter_map(|u| {
            Uuid::parse_str(u)
                .ok()
                .and_then(|u| doc.slot_of(u))
                .and_then(|s| doc.get(s))
        })
        .filter_map(|e| {
            ["number", "tag", "name", "type"]
                .iter()
                .find_map(|p| {
                    doc.arch_value(e, p, None)
                        .map(|v| doc.arch_value_text(&v, false))
                })
                .filter(|w| !w.is_empty())
        })
        .take(6)
        .collect();
    let more = objects.len().saturating_sub(names.len());
    let mut words = format!("Yazılır: {} nesne", objects.len());
    if !names.is_empty() {
        words.push_str(&format!(" — {}", names.join(", ")));
        if more > 0 {
            words.push_str(&format!(" ve {more} nesne daha"));
        }
    }
    Some(Ok(format!("{words}. Kaydet hepsine tek adımda yazar.")))
}

/// A schedule's draft: its cells are its objects' values, so the draft is the
/// schedule as Kaydet would leave it, worked out on a copy of the drawing (each cell through `cad.schedules.edit`, the
/// sizes through `cad.tables.edit`, as [`App::save_schedule_editor`] writes them); a value the objects refuse is
/// refused here, while the cell is being written. Nothing is written to the drawing.
pub(super) fn schedule_planned(
    doc: &kentos_domain::Document,
    uid: Uuid,
    edits: &[TableEdit],
    saved: &TableEntity,
) -> Result<TableEntity, String> {
    use kentos_commands::{ScheduleOperation, SchedulesEdit};
    let mut copy = doc.reading_copy();
    fn failed<T>(r: CommandResult<T>) -> Option<String> {
        match r {
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Some(error.message),
            _ => None,
        }
    }
    for edit in edits {
        let why = match edit {
            TableEdit::SetCell { row, col, words } => failed(kentos_native_application::schedules_edit::execute(
                &mut ExecutionContext::new(&mut copy),
                SchedulesEdit {
                    uid: uid.to_string(),
                    operation: ScheduleOperation::Cell { row: *row, column: *col, value: words.clone() },
                    expected_revision: None,
                },
            )),
            size @ (TableEdit::RowHeight { .. } | TableEdit::ColumnWidth { .. } | TableEdit::Fit { .. }) => {
                failed(tables_edit::execute(
                    &mut ExecutionContext::new(&mut copy),
                    TablesEdit {
                        operation: TablesOperation::Edit { uid: uid.to_string(), edits: vec![size.clone()] },
                        expected_revision: None,
                    },
                ))
            }
            _ => Some(
                "Çizelgenin satırları, sütunları ve biçimi belgenin yazdığıdır; yalnız hücreler, boylar ve genişlikler düzenlenir."
                    .to_owned(),
            ),
        };
        if let Some(why) = why {
            return Err(why);
        }
    }
    Ok(match copy.slot_of(uid).and_then(|s| copy.get(s)) {
        Some(Entity::Table(t)) => t.clone(),
        _ => saved.clone(),
    })
}
