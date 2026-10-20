//! `cad.schedules.create` and `cad.schedules.edit` v1 (docs/adr/0344 §3, §4; plan/cad.md `CAD-ARCH-21`, `22`): a
//! schedule of the drawing's openings, spaces, columns or walls placed as a table, one undo step “Çizelge”, and edited,
//! one undo step “Çizelgeyi düzenle”: a cell written to its row's objects, or its columns, filter, sort, grouping and
//! total changed. Its rows are the document's (`kentos_domain::schedules`), written again at every step. The shared
//! cases are `fixtures/commands/v1/cad.schedules.{create,edit}.json`.
//!
//! A cell is the objects' own field when its column's is (a number, a type, a width, a sill, a head, a name, a usage, a
//! tag, a wall's thickness, an attribute), else it is worked out and refused (`cell_computed`); a wall's type names its
//! style, which a schedule does not change. A length is read in the project's unit, a point or a comma before its
//! decimals; empty words take an optional field away.

use kentos_commands::{
    CommandResult, CommandWarning, Entity, EntityBase, ScheduleChange, ScheduleColumn,
    ScheduleField, ScheduleFilter, ScheduleOf, ScheduleOperation, ScheduleSort, SchedulesCreate,
    SchedulesCreated, SchedulesEdit, SchedulesEdited, SchedulesPlan, TableEntity, TableSource,
};
use kentos_domain::{Document, Slot, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;

pub const CREATE_STEP: &str = "Çizelge";
pub const EDIT_STEP: &str = "Çizelgeyi düzenle";

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// `invalid_schedule` for a spec that makes none, `level_not_found` and `layer_not_found` for its filter's unknowns.
fn spec_holds(
    doc: &Document,
    of: ScheduleOf,
    columns: &[ScheduleColumn],
    sort: &[ScheduleSort],
    filter: Option<&ScheduleFilter>,
    at: &str,
) -> Result<(), Stop> {
    if let Some((field, words)) = kentos_commands::schedule_problem(of, columns, sort, filter) {
        return Err(fail(
            codes::INVALID_SCHEDULE,
            words,
            &format!("{at}{field}"),
        ));
    }
    let known: Vec<String> = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.iter().map(|l| l.id.clone()).collect())
        .unwrap_or_default();
    if let Some(f) = filter {
        for (k, l) in f.levels.iter().flatten().enumerate() {
            if !known.contains(l) {
                return Err(fail(
                    codes::LEVEL_NOT_FOUND,
                    format!("Projede “{l}” kimlikli kat yok. Var olan bir katın kimliğini verin."),
                    &format!("{at}filter.levels[{k}]"),
                ));
            }
        }
        for (k, l) in f.layers.iter().flatten().enumerate() {
            if doc.layers().get(l).is_none() {
                return Err(fail(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "“{l}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                    ),
                    &format!("{at}filter.layers[{k}]"),
                ));
            }
        }
    }
    Ok(())
}

fn create_check(
    doc: &Document,
    input: &SchedulesCreate,
) -> Result<(Vec<CommandWarning>, TableEntity), Stop> {
    checks::point(input.at, "Çizelgenin köşesinin", "at")?;
    let columns = input
        .columns
        .clone()
        .unwrap_or_else(|| input.of.default_columns());
    spec_holds(
        doc,
        input.of,
        &columns,
        &input.sort,
        input.filter.as_ref(),
        "",
    )?;
    if let Some(h) = input.height {
        checks::finite(h, "Yazı yüksekliği", "Sonlu bir sayı verin.", "height")?;
        if h <= 0.0 || h > kentos_commands::MAX_TABLE_SIZE {
            return Err(fail(
                codes::INVALID_SCHEDULE,
                format!("Çizelgenin yazı yüksekliği {h}; sıfırdan büyük olmalı."),
                "height",
            ));
        }
    }
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let warnings = checks::layer(doc, &input.layer_id)?;
    let height = input.height.unwrap_or_else(|| {
        doc.settings()
            .annotation_height(kentos_commands::AnnotationKind::Table)
    });
    let table = TableEntity {
        base: EntityBase {
            look: Default::default(),
            id: 0,
            layer_id: input.layer_id.clone(),
            color: input.color.clone(),
            attrs: Default::default(),
            label: None,
            symbol: None,
            line_weight: input.line_weight,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        p: input.at,
        rotation: 0.0,
        height,
        rows: Vec::new(),
        columns: Vec::new(),
        cells: Vec::new(),
        merges: Vec::new(),
        aligns: None,
        header: true,
        grid: None,
        frame: None,
        face: Default::default(),
        source: Some(TableSource::Schedule {
            of: input.of,
            columns,
            filter: input.filter.clone().filter(|f| !f.is_empty()),
            sort: input.sort.clone(),
            group: input.group.unwrap_or(false),
            total: input.total.unwrap_or(false),
            members: Vec::new(),
        }),
        styles: Vec::new(),
        formulas: Vec::new(),
        fields: Vec::new(),
    };
    Ok((warnings, doc.table_worked_out(&table)))
}

fn rows_of(t: &TableEntity) -> u32 {
    match &t.source {
        Some(TableSource::Schedule { members, .. }) => {
            u32::try_from(members.len()).unwrap_or(u32::MAX)
        }
        _ => 0,
    }
}

pub mod create {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &SchedulesCreate) -> CommandResult<()> {
        match create_check(cx.doc, input) {
            Ok((warnings, _)) => CommandResult::Completed {
                output: (),
                warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &SchedulesCreate,
    ) -> CommandResult<SchedulesPlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] on a document: what the schedule window places.
    pub fn plan_in(doc: &Document, input: &SchedulesCreate) -> CommandResult<SchedulesPlan> {
        match create_check(doc, input) {
            Ok((warnings, t)) => CommandResult::Completed {
                output: SchedulesPlan {
                    table: Entity::Table(t),
                    revision: doc.revision().to_string(),
                },
                warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: SchedulesCreate,
    ) -> CommandResult<SchedulesCreated> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_SCHEDULES_CREATE,
            kentos_commands::CAD_SCHEDULES_CREATE_VERSION,
            input,
            write_execute,
        )
    }

    fn write_execute(
        cx: &mut ExecutionContext<'_>,
        input: SchedulesCreate,
    ) -> CommandResult<SchedulesCreated> {
        let (warnings, t) = match create_check(cx.doc, &input) {
            Ok(checked) => checked,
            Err(stop) => return stop.into(),
        };
        let rows = rows_of(&t);
        let slots = match cx.doc.add_many(vec![Entity::Table(t)], CREATE_STEP) {
            Ok(slots) => slots,
            Err(full) => {
                return Stop::failed(error(codes::SLOTS_EXHAUSTED, full.to_string(), None)).into();
            }
        };
        let Some(slot) = slots.first().copied() else {
            return Stop::failed(error(codes::SLOTS_EXHAUSTED, "Yuva yok.".into(), None)).into();
        };
        CommandResult::Completed {
            output: SchedulesCreated {
                uid: cx.doc.uid(slot).map(|u| u.to_string()).unwrap_or_default(),
                id: slot.0,
                rows,
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        }
    }
}

/// The schedule at `uid`: its slot and table.
fn schedule_at(doc: &Document, uid: &str) -> Result<(Slot, TableEntity), Stop> {
    if !checks::is_uid_text(uid) {
        return Err(fail(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; küçük harfli, tireli bir UUID verin."
            ),
            "uid",
        ));
    }
    let slot = Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u));
    let Some((slot, e)) = slot.and_then(|s| doc.get(s).map(|e| (s, e))) else {
        return Err(fail(
            codes::ENTITY_NOT_FOUND,
            format!("“{uid}” kimlikli nesne çizimde yok. Var olan bir çizelgenin kimliğini verin."),
            "uid",
        ));
    };
    match e {
        Entity::Table(t) if matches!(t.source, Some(TableSource::Schedule { .. })) => Ok((slot, t.clone())),
        _ => Err(fail(
            codes::NOT_SCHEDULE,
            "Nesne bir mimari çizelge değil; Çizelge ile yerleştirilen bir tablonun kimliğini verin.".into(),
            "uid",
        )),
    }
}

/// Whether a column of a schedule of `of` is edited in it.
pub fn editable(of: ScheduleOf, field: ScheduleField) -> bool {
    field.editable() && !(field == ScheduleField::Type && of == ScheduleOf::Walls)
}

fn bad_value(words: String) -> Stop {
    fail(codes::INVALID_CELL_VALUE, words, "operation.value")
}

/// A length typed in the project's unit, in metres; `positive`: over 0, else at least 0.
fn length_of(doc: &Document, v: &str, positive: bool) -> Result<f64, Stop> {
    let typed: f64 = v.trim().replace(',', ".").parse().map_err(|_| {
        bad_value(format!(
            "“{v}” bir uzunluk değil; projenin biriminde bir sayı yazın."
        ))
    })?;
    let m = kentos_project::format::Format::of(doc.settings()).to_metres(typed);
    if !m.is_finite() || (positive && m <= 0.0) || m < 0.0 {
        return Err(bad_value(format!(
            "“{v}” bu alana yazılmaz: uzunluk {}sıfırdan büyük olmalı.",
            if positive { "" } else { "eksi olmamalı; " }
        )));
    }
    Ok(m)
}

/// `e` with `value` written to its field `field` (`attr` its attribute's name).
fn written(
    doc: &Document,
    mut e: Entity,
    field: ScheduleField,
    attr: Option<&str>,
    value: &str,
) -> Result<Entity, Stop> {
    let text = || (!value.trim().is_empty()).then(|| value.trim().to_owned());
    use ScheduleField as F;
    match (&mut e, field) {
        (_, F::Attr) => {
            let name = attr.unwrap_or_default().to_owned();
            match text() {
                Some(v) => {
                    e.base_mut().attrs.insert(name, v);
                }
                None => {
                    e.base_mut().attrs.remove(&name);
                }
            }
        }
        (Entity::Opening(o), F::Number) => o.opening.number = text(),
        // A type KentOS knows by its name or id is kept by its id, as Açıklık writes it.
        (Entity::Opening(o), F::Type) => {
            o.opening.style =
                text().map(|v| kentos_commands::opening_type(&v).map_or(v, |t| t.id));
        }
        (Entity::Opening(o), F::Width) => o.opening.width = length_of(doc, value, true)?,
        (Entity::Opening(o), F::Sill) => {
            o.opening.sill = text().map(|v| length_of(doc, &v, false)).transpose()?;
        }
        (Entity::Opening(o), F::Head) => {
            o.opening.head = text().map(|v| length_of(doc, &v, true)).transpose()?;
        }
        (Entity::Space(s), F::Number) => s.space.number = text(),
        (Entity::Space(s), F::Name) => s.space.name = text(),
        (Entity::Space(s), F::Usage) => s.space.usage = text(),
        (Entity::Column(c), F::Tag) => c.column.tag = text(),
        (Entity::Column(c), F::Type) => c.column.kind = text(),
        (Entity::Column(c), F::Width) => c.column.width = length_of(doc, value, true)?,
        (Entity::Wall(w), F::Thickness) => w.wall.width = length_of(doc, value, true)?,
        _ => {
            return Err(fail(
                codes::CELL_COMPUTED,
                "Bu hücre bu nesnenin kendi alanı değil; düzenlenmez.".into(),
                "operation.column",
            ));
        }
    }
    let problem = match &e {
        Entity::Opening(o) => o.opening.problem(),
        Entity::Space(s) => s.space.problem(),
        Entity::Column(c) => c.column.problem(),
        Entity::Wall(w) => w.wall.problem(),
        _ => None,
    };
    if let Some((_, words)) = problem {
        return Err(bad_value(words));
    }
    Ok(e)
}

type Edited = (Vec<CommandWarning>, Slot, TableEntity, Vec<(Slot, Entity)>);

fn edit_check(doc: &Document, input: &SchedulesEdit) -> Result<Edited, Stop> {
    let (slot, t) = schedule_at(doc, &input.uid)?;
    let Some(TableSource::Schedule {
        of,
        columns,
        filter,
        sort,
        group,
        total,
        members,
    }) = t.source.clone()
    else {
        return Err(fail(
            codes::NOT_SCHEDULE,
            "Tablo bir mimari çizelge değil.".into(),
            "uid",
        ));
    };
    let mut work: Vec<(Slot, Entity)> = Vec::new();
    let mut next = t.clone();
    match &input.operation {
        ScheduleOperation::Cell { row, column, value } => {
            let r = *row as usize;
            if r == 0 || r > members.len() {
                return Err(fail(
                    codes::CELL_OUT_OF_RANGE,
                    format!(
                        "Çizelgenin {}. satırı veri satırı değil; veri satırları 1–{} (0 başlık{}).",
                        row,
                        members.len(),
                        if total { ", sonuncusu toplam" } else { "" }
                    ),
                    "operation.row",
                ));
            }
            let Some(c) = columns.get(*column as usize) else {
                return Err(fail(
                    codes::CELL_OUT_OF_RANGE,
                    format!(
                        "Çizelgenin {} sütunu var; {}. sütun yok.",
                        columns.len(),
                        column + 1
                    ),
                    "operation.column",
                ));
            };
            if !editable(of, c.field) {
                return Err(fail(
                    codes::CELL_COMPUTED,
                    format!(
                        "“{}” sütunu çizimden hesaplanır; düzenlenmez. Kaynağın kendi alanını değiştirin.",
                        doc.schedule_heading(c)
                    ),
                    "operation.column",
                ));
            }
            let row_members = &members[r - 1];
            if c.field == ScheduleField::Number && row_members.len() > 1 {
                return Err(fail(
                    codes::SCHEDULE_GROUP_NUMBER,
                    format!(
                        "Bu satır {} nesnedir; aynı numara hepsine yazılmaz. Grubu kaldırıp tek tek numaralayın.",
                        row_members.len()
                    ),
                    "operation.row",
                ));
            }
            if value.chars().any(char::is_control) || value.chars().count() > 200 {
                return Err(bad_value(
                    "Hücrenin yazısı tek satırdır, en çok 200 harftir.".into(),
                ));
            }
            for u in row_members {
                let Some(s) = Uuid::parse_str(u).ok().and_then(|u| doc.slot_of(u)) else {
                    continue;
                };
                let Some(e) = doc.get(s).cloned() else {
                    continue;
                };
                work.push((s, written(doc, e, c.field, c.attr.as_deref(), value)?));
            }
        }
        ScheduleOperation::Set(change) => {
            let ScheduleChange {
                columns: c2,
                filter: f2,
                sort: s2,
                group: g2,
                total: t2,
            } = &**change;
            let columns = c2.clone().unwrap_or(columns);
            let filter = match f2 {
                Some(f) => f.clone().filter(|f| !f.is_empty()),
                None => filter,
            };
            let sort = s2.clone().unwrap_or(sort);
            spec_holds(doc, of, &columns, &sort, filter.as_ref(), "operation.")?;
            next.source = Some(TableSource::Schedule {
                of,
                columns,
                filter,
                sort,
                group: g2.unwrap_or(group),
                total: t2.unwrap_or(total),
                members: Vec::new(),
            });
            next = doc.table_worked_out(&next);
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let table_too = matches!(input.operation, ScheduleOperation::Set(_));
    let written_to = work.iter().map(|(_, e)| e.base().layer_id.as_str());
    for layer in written_to.chain(table_too.then_some(t.base.layer_id.as_str())) {
        if doc.layers().is_locked(layer) {
            let name = doc.layers().get(layer).map_or(layer, |n| n.name.as_str());
            return Err(fail(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; çizelgenin satırı ya da kendisi düzenlenemez. Kilidi Katmanlar panelinden açın."
                ),
                "uid",
            ));
        }
    }
    Ok((Vec::new(), slot, next, work))
}

/// The edit applied to `doc`: its step, the table as the step leaves it and the objects written to.
fn apply(
    doc: &mut Document,
    slot: Slot,
    next: TableEntity,
    work: Vec<(Slot, Entity)>,
) -> (TableEntity, Vec<String>) {
    let edited: Vec<String> = work
        .iter()
        .filter_map(|(s, _)| doc.uid(*s).map(|u| u.to_string()))
        .collect();
    let mut all = work;
    if all.is_empty() {
        all.push((slot, Entity::Table(next.clone())));
    }
    doc.update_many(all, EDIT_STEP);
    // As the step's end writes it (a call's group follows when it closes): the schedule's rule on the drawing now.
    let table = match doc.get(slot) {
        Some(Entity::Table(t)) => doc.table_worked_out(t),
        _ => next,
    };
    (table, edited)
}

pub mod edit {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &SchedulesEdit) -> CommandResult<()> {
        match edit_check(cx.doc, input) {
            Ok((warnings, ..)) => CommandResult::Completed {
                output: (),
                warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &SchedulesEdit,
    ) -> CommandResult<SchedulesEdited> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] on a document: the schedule as the edit would leave it, on a copy; nothing written.
    pub fn plan_in(doc: &Document, input: &SchedulesEdit) -> CommandResult<SchedulesEdited> {
        match edit_check(doc, input) {
            Ok((warnings, slot, next, work)) => {
                let mut copy = doc.reading_copy();
                let (table, edited) = apply(&mut copy, slot, next, work);
                CommandResult::Completed {
                    output: SchedulesEdited {
                        table: Entity::Table(table),
                        edited,
                        revision: doc.revision().to_string(),
                    },
                    warnings,
                }
            }
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: SchedulesEdit,
    ) -> CommandResult<SchedulesEdited> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_SCHEDULES_EDIT,
            kentos_commands::CAD_SCHEDULES_EDIT_VERSION,
            input,
            write_execute,
        )
    }

    fn write_execute(
        cx: &mut ExecutionContext<'_>,
        input: SchedulesEdit,
    ) -> CommandResult<SchedulesEdited> {
        let (warnings, slot, next, work) = match edit_check(cx.doc, &input) {
            Ok(checked) => checked,
            Err(stop) => return stop.into(),
        };
        let (table, edited) = apply(cx.doc, slot, next, work);
        CommandResult::Completed {
            output: SchedulesEdited {
                table: Entity::Table(table),
                edited,
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        }
    }
}
