//! Live wall dimension recipes (ADR0695). GUI, Python and MCP share these handlers.
use std::collections::{BTreeMap, HashSet};

use kentos_commands::{
    CommandResult, DimensionSet, DimensionSetBreak, DimensionSetEdit, DimensionSetId,
    DimensionSetRowPlace, DimensionSetsChanged, DimensionSetsCreateInput, DimensionSetsEditInput,
    DimensionSetsPlan, DimensionSetsQueried, DimensionSetsQueryInput, Entity, EntityId,
    dimension_sets_problem,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};

fn fail(message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(
        "invalid_dimension_set",
        message.into(),
        Some(path.into()),
    ))
}
fn source_error(why: DimensionSetBreak) -> Stop {
    let message = match why {
        DimensionSetBreak::HostGone => "Kaynak duvar bulunamadı. Başka bir duvar seçin.",
        DimensionSetBreak::UnsupportedWall => {
            "Bu araç şu anda iki uçlu düz duvarı ölçer. Kapalı, çok parçalı veya yaylı duvar desteklenmiyor."
        }
        DimensionSetBreak::OpeningUnplaced => {
            "Duvarın bir açıklığı yerleştirilemedi. Önce açıklığın kaynak bağını düzeltin."
        }
    };
    fail(message, "wall")
}
fn recipe_ok(set: &DimensionSet) -> Result<(), Stop> {
    let mut recipe = set.clone();
    recipe.members.clear(); // membership is derived, never caller input
    if let Some((_, why)) = dimension_sets_problem(&[recipe]) {
        return Err(fail(why, "operation"));
    }
    Ok(())
}
fn create_recipe(doc: &Document, input: &DimensionSetsCreateInput) -> Result<DimensionSet, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    checks::point(input.at, "Ölçü sırasının yeri", "at")?;
    checks::layer(doc, &input.layer_id)?;
    if !checks::is_uid_text(&input.wall) {
        return Err(fail("Geçerli bir duvar kimliği verin.", "wall"));
    }
    let wall = Uuid::parse_str(&input.wall).map_err(|_| fail("Duvar kimliği geçersiz.", "wall"))?;
    let (look, height) = crate::dimension_create::style_of(doc, input.dimension_style.as_deref())?;
    let set = DimensionSet {
        id: DimensionSetId([1; 16]), // internal proposal only; a plan allocates no identity
        wall: EntityId(wall.into_bytes()),
        face: input.face,
        rows: input.rows.clone(),
        openings: input.openings,
        layer_id: input.layer_id.clone(),
        at: input.at,
        gap_mm: input.gap_mm,
        height,
        look,
        row_places: Vec::new(),
        members: Vec::new(),
        suppressed: Vec::new(),
        broken: None,
    };
    recipe_ok(&set)?;
    doc.dimension_set_plan(&set).map_err(source_error)?;
    Ok(set)
}
fn member_entities(doc: &Document, set: &DimensionSet) -> BTreeMap<String, Entity> {
    set.members
        .iter()
        .filter_map(|m| {
            let uid = Uuid::from_bytes(m.uid.0);
            Some((uid.to_string(), doc.get(doc.slot_of(uid)?)?.clone()))
        })
        .collect()
}
fn existing(doc: &Document, id: DimensionSetId) -> Result<&DimensionSet, Stop> {
    doc.dimension_set(id).ok_or_else(|| {
        Stop::failed(error(
            "dimension_set_not_found",
            "Ölçü grubu bulunamadı; yeniden seçin.".into(),
            Some("id".into()),
        ))
    })
}
fn check_edit(doc: &Document, input: &DimensionSetsEditInput) -> Result<DimensionSet, Stop> {
    checks::revision(doc, input.expected_revision.as_deref())?;
    let before = existing(doc, input.id)?;
    let mut checked = HashSet::new();
    for layer in std::iter::once(&before.layer_id).chain(before.members.iter().filter_map(|m| {
        let uid = Uuid::from_bytes(m.uid.0);
        doc.get(doc.slot_of(uid)?).map(|e| &e.base().layer_id)
    })) {
        if checked.insert(layer) {
            checks::layer(doc, layer)?;
        }
    }
    let mut set = before.clone();
    match &input.operation {
        DimensionSetEdit::Move { at, row } => {
            checks::point(*at, "Ölçü sırasının yeri", "operation.at")?;
            if let Some(row) = row {
                if !set.rows.contains(row) {
                    return Err(fail("Bu sıra grupta yok.", "operation.row"));
                }
                match set.row_places.iter_mut().find(|p| p.row == *row) {
                    Some(place) => place.at = *at,
                    None => set
                        .row_places
                        .push(DimensionSetRowPlace { row: *row, at: *at }),
                }
            } else {
                let delta = (at.x - set.at.x, at.y - set.at.y);
                set.at = *at;
                for place in &mut set.row_places {
                    place.at.x += delta.0;
                    place.at.y += delta.1;
                }
            }
        }
        DimensionSetEdit::RowOffset { row, offset } => {
            if !offset.is_finite() {
                return Err(fail("Sıra uzaklığı sonlu olmalı.", "operation.offset"));
            }
            doc.set_dimension_set_row_offset(&mut set, *row, *offset)
                .map_err(source_error)?;
        }
        DimensionSetEdit::Set {
            face,
            rows,
            openings,
            gap_mm,
            dimension_style,
            layer_id,
        } => {
            if let Some(face) = face {
                set.face = *face;
            }
            if let Some(rows) = rows {
                set.rows = rows.clone();
                set.row_places.retain(|p| rows.contains(&p.row));
                set.members.retain(|m| rows.contains(&m.row));
                set.suppressed.retain(|m| rows.contains(&m.row));
            }
            if let Some(openings) = openings {
                set.openings = *openings;
            }
            if let Some(gap) = gap_mm {
                set.gap_mm = *gap;
            }
            if let Some(layer) = layer_id {
                checks::layer(doc, layer)?;
                set.layer_id = layer.clone();
            }
            if let Some(name) = dimension_style {
                (set.look, set.height) = crate::dimension_create::style_of(doc, Some(name))?;
            }
        }
        DimensionSetEdit::RestoreHidden => set.suppressed.clear(),
        DimensionSetEdit::ResetRows => set.row_places.clear(),
        DimensionSetEdit::ResetText { row } => {
            if row.is_some_and(|r| !set.rows.contains(&r)) {
                return Err(fail("Bu sıra grupta yok.", "operation.row"));
            }
        }
        DimensionSetEdit::Detach | DimensionSetEdit::Delete => return Ok(set),
    }
    recipe_ok(&set)?;
    // Text reset can still repair text placement when a source is broken.
    if !matches!(input.operation, DimensionSetEdit::ResetText { .. }) {
        doc.dimension_set_plan(&set).map_err(source_error)?;
    }
    Ok(set)
}
fn planned(doc: &Document, set: &DimensionSet, existing: bool) -> Result<DimensionSetsPlan, Stop> {
    let members = doc.dimension_set_plan(set).map_err(source_error)?;
    let kept: HashSet<_> = members.iter().filter_map(|m| m.uid).collect();
    let removed = set
        .members
        .iter()
        .filter(|m| !kept.contains(&m.uid))
        .map(|m| m.uid.to_string())
        .collect();
    Ok(DimensionSetsPlan {
        set: existing.then_some(set.id),
        entities: members
            .iter()
            .map(|m| Entity::Dimension(m.dimension.clone()))
            .collect(),
        spans: members.iter().map(|m| m.segment.clone()).collect(),
        uids: members
            .iter()
            .filter_map(|m| m.uid.map(|u| u.to_string()))
            .collect(),
        removed,
        revision: doc.revision().to_string(),
    })
}
fn success<T>(output: T) -> CommandResult<T> {
    CommandResult::Completed {
        output,
        warnings: Vec::new(),
    }
}
fn written_error(message: String) -> Stop {
    fail(message, "operation")
}
fn visible_result<T>(doc: &Document, layer: &str, output: T) -> CommandResult<T> {
    CommandResult::Completed {
        output,
        warnings: checks::layer(doc, layer).unwrap_or_default(),
    }
}

pub mod create {
    use super::*;
    pub fn validate(
        cx: &ExecutionContext<'_>,
        input: &DimensionSetsCreateInput,
    ) -> CommandResult<()> {
        match create_recipe(cx.doc, input) {
            Ok(s) => visible_result(cx.doc, &s.layer_id, ()),
            Err(e) => e.into(),
        }
    }
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &DimensionSetsCreateInput,
    ) -> CommandResult<DimensionSetsPlan> {
        plan_in(cx.doc, input)
    }
    pub fn plan_in(
        doc: &Document,
        input: &DimensionSetsCreateInput,
    ) -> CommandResult<DimensionSetsPlan> {
        match create_recipe(doc, input).and_then(|s| planned(doc, &s, false)) {
            Ok(out) => visible_result(doc, &input.layer_id, out),
            Err(e) => e.into(),
        }
    }
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: DimensionSetsCreateInput,
    ) -> CommandResult<DimensionSetsChanged> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_DIMENSION_SETS_CREATE,
            kentos_commands::CAD_DIMENSION_SETS_VERSION,
            input,
            write,
        )
    }
    fn write(
        cx: &mut ExecutionContext<'_>,
        input: DimensionSetsCreateInput,
    ) -> CommandResult<DimensionSetsChanged> {
        let mut set = match create_recipe(cx.doc, &input) {
            Ok(s) => s,
            Err(e) => return e.into(),
        };
        set.id = DimensionSetId(Uuid::now_v7().into_bytes());
        let id = set.id;
        if let Err(why) = cx.doc.add_dimension_set(set) {
            return written_error(why.to_string()).into();
        }
        let uids = cx
            .doc
            .dimension_set(id)
            .map(|s| {
                s.members
                    .iter()
                    .map(|m| m.uid.to_string())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        visible_result(
            cx.doc,
            &input.layer_id,
            DimensionSetsChanged {
                set: id,
                created: uids.clone(),
                uids,
                changed: Vec::new(),
                removed: Vec::new(),
                revision: cx.doc.revision().to_string(),
            },
        )
    }
}
pub mod edit {
    use super::*;
    pub fn validate(
        cx: &ExecutionContext<'_>,
        input: &DimensionSetsEditInput,
    ) -> CommandResult<()> {
        match check_edit(cx.doc, input) {
            Ok(s) => visible_result(cx.doc, &s.layer_id, ()),
            Err(e) => e.into(),
        }
    }
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &DimensionSetsEditInput,
    ) -> CommandResult<DimensionSetsPlan> {
        let set = match check_edit(cx.doc, input) {
            Ok(s) => s,
            Err(e) => return e.into(),
        };
        let before = match existing(cx.doc, input.id) {
            Ok(set) => set,
            Err(e) => return e.into(),
        };
        let mut out = match input.operation {
            DimensionSetEdit::Delete
            | DimensionSetEdit::Detach
            | DimensionSetEdit::ResetText { .. } => DimensionSetsPlan {
                set: Some(set.id),
                entities: Vec::new(),
                spans: Vec::new(),
                removed: Vec::new(),
                uids: Vec::new(),
                revision: cx.doc.revision().to_string(),
            },
            _ => match planned(cx.doc, &set, true) {
                Ok(o) => o,
                Err(e) => return e.into(),
            },
        };
        match input.operation {
            DimensionSetEdit::Delete => {
                out.removed = before.members.iter().map(|m| m.uid.to_string()).collect()
            }
            DimensionSetEdit::Detach => {
                out.uids = before.members.iter().map(|m| m.uid.to_string()).collect()
            }
            DimensionSetEdit::ResetText { row } => {
                for member in &before.members {
                    if row.is_some_and(|r| r != member.row) {
                        continue;
                    }
                    let uid = Uuid::from_bytes(member.uid.0);
                    if let Some(Entity::Dimension(d)) =
                        cx.doc.slot_of(uid).and_then(|s| cx.doc.get(s))
                        && d.text_shift.is_some()
                    {
                        let mut d = d.clone();
                        d.text_shift = None;
                        out.entities.push(Entity::Dimension(d));
                        out.uids.push(uid.to_string());
                        out.spans.push(member.segment());
                    }
                }
            }
            _ => {
                let kept: HashSet<_> = out.uids.iter().collect();
                out.removed = before
                    .members
                    .iter()
                    .map(|m| m.uid.to_string())
                    .filter(|u| !kept.contains(u))
                    .collect();
            }
        }
        visible_result(cx.doc, &set.layer_id, out)
    }
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: DimensionSetsEditInput,
    ) -> CommandResult<DimensionSetsChanged> {
        crate::call::recorded(
            cx,
            kentos_commands::CAD_DIMENSION_SETS_EDIT,
            kentos_commands::CAD_DIMENSION_SETS_VERSION,
            input,
            write,
        )
    }
    fn write(
        cx: &mut ExecutionContext<'_>,
        input: DimensionSetsEditInput,
    ) -> CommandResult<DimensionSetsChanged> {
        let set = match check_edit(cx.doc, &input) {
            Ok(s) => s,
            Err(e) => return e.into(),
        };
        let old = match existing(cx.doc, input.id) {
            Ok(set) => set.clone(),
            Err(e) => return e.into(),
        };
        let before = member_entities(cx.doc, &old);
        let result_layer = set.layer_id.clone();
        let result = cx
            .doc
            .transact("Duvar ölçülerini düzenle", |doc| -> Result<(), String> {
                match &input.operation {
                    DimensionSetEdit::Detach | DimensionSetEdit::Delete => {
                        doc.remove_dimension_set(set.id)
                            .map_err(|e| e.to_string())?;
                        if matches!(input.operation, DimensionSetEdit::Delete) {
                            let slots: Vec<_> = old
                                .members
                                .iter()
                                .filter_map(|m| doc.slot_of(Uuid::from_bytes(m.uid.0)))
                                .collect();
                            doc.remove(&slots);
                        }
                    }
                    DimensionSetEdit::ResetText { row } => {
                        for member in &old.members {
                            if row.is_some_and(|r| r != member.row) {
                                continue;
                            }
                            let Some(slot) = doc.slot_of(Uuid::from_bytes(member.uid.0)) else {
                                continue;
                            };
                            if let Some(Entity::Dimension(d)) = doc.get(slot)
                                && d.text_shift.is_some()
                            {
                                let mut d = d.clone();
                                d.text_shift = None;
                                doc.update(slot, Entity::Dimension(d));
                            }
                        }
                    }
                    _ => {
                        doc.update_dimension_set(set).map_err(|e| e.to_string())?;
                    }
                }
                Ok(())
            });
        if let Err(message) = result {
            return written_error(message).into();
        }
        let after = cx
            .doc
            .dimension_set(input.id)
            .map(|s| member_entities(cx.doc, s))
            .unwrap_or_else(|| {
                before
                    .keys()
                    .filter_map(|u| {
                        let uid = Uuid::parse_str(u).ok()?;
                        Some((u.clone(), cx.doc.get(cx.doc.slot_of(uid)?)?.clone()))
                    })
                    .collect()
            });
        let uids = cx
            .doc
            .dimension_set(input.id)
            .map(|s| s.members.iter().map(|m| m.uid.to_string()).collect())
            .unwrap_or_else(|| {
                old.members
                    .iter()
                    .map(|m| m.uid.to_string())
                    .filter(|u| after.contains_key(u))
                    .collect()
            });
        let created = after
            .keys()
            .filter(|u| !before.contains_key(*u))
            .cloned()
            .collect();
        let removed = before
            .keys()
            .filter(|u| !after.contains_key(*u))
            .cloned()
            .collect();
        let changed = after
            .iter()
            .filter(|(u, e)| before.get(*u).is_some_and(|b| b != *e))
            .map(|(u, _)| u.clone())
            .collect();
        visible_result(
            cx.doc,
            &result_layer,
            DimensionSetsChanged {
                set: input.id,
                uids,
                created,
                removed,
                changed,
                revision: cx.doc.revision().to_string(),
            },
        )
    }
}
pub mod query {
    use super::*;
    fn read(doc: &Document, input: &DimensionSetsQueryInput) -> DimensionSetsQueried {
        DimensionSetsQueried {
            sets: doc
                .dimension_sets()
                .iter()
                .filter(|s| {
                    (input.ids.is_empty() || input.ids.contains(&s.id))
                        && input.wall.is_none_or(|w| w == s.wall)
                })
                .map(|s| (**s).clone())
                .collect(),
            revision: doc.revision().to_string(),
        }
    }
    pub fn validate(
        _cx: &ExecutionContext<'_>,
        _input: &DimensionSetsQueryInput,
    ) -> CommandResult<()> {
        success(())
    }
    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &DimensionSetsQueryInput,
    ) -> CommandResult<DimensionSetsQueried> {
        success(read(cx.doc, input))
    }
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: DimensionSetsQueryInput,
    ) -> CommandResult<DimensionSetsQueried> {
        success(read(cx.doc, &input))
    }
}

/// Preview the complete derived delta without overloading walls/openings result kinds.
pub(crate) fn derived(
    doc: &Document,
    updates: &[(kentos_domain::Slot, Entity)],
    adds: &[Entity],
    removes: &[kentos_domain::Slot],
) -> kentos_commands::DerivedEntitiesPlan {
    let delta = doc.preview_followers(updates, adds, removes);
    kentos_commands::DerivedEntitiesPlan {
        added: delta.added.into_iter().map(|(_, e)| e).collect(),
        updated: delta.updated.into_iter().map(|(_, e)| e).collect(),
        removed: delta
            .removed
            .into_iter()
            .filter_map(|s| doc.uid(s).map(|u| u.to_string()))
            .collect(),
    }
}
