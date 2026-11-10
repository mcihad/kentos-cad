//! `cad.sections.query` v1 (docs/adr/8042 §5, read-only): every section mark's section with whether it still is what
//! the model would draw, as BricsCAD's Project Browser marks a drawing out of sync (red) or never made (orange). A
//! section is current when its block's objects, their ids aside, are what drawing it again gives; stale otherwise; not
//! drawn when its mark has no block; gone when its block's mark is no longer in the drawing.

use kentos_commands::{
    BuildingSectionRow, BuildingSectionState, BuildingSectionsQueried, BuildingSectionsQuery,
    CommandResult, Entity, ViewMarkForm,
};
use kentos_domain::{Document, Uuid};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::sections;
use crate::sections_edit::{bare, mark_of};

/// A section mark's section's state.
pub fn state_of(
    doc: &Document,
    mark: Uuid,
    fields: &kentos_commands::ViewMarkFields,
) -> BuildingSectionState {
    let Some(block) = doc.block(sections::block_id(mark)) else {
        return BuildingSectionState::NotDrawn;
    };
    let Some(frame) = sections::frame_of(fields) else {
        return BuildingSectionState::Stale;
    };
    let drawn = sections::draw(doc, &frame, &fields.mark);
    // The block's objects name their layers by id; the drawing names them by their EK-4 names.
    let ids = |name: &str| -> String {
        let id = name.to_ascii_lowercase();
        if doc.layers().get(&id).is_some() {
            return id;
        }
        doc.layers()
            .leaves()
            .into_iter()
            .find(|l| l.name.eq_ignore_ascii_case(name))
            .map_or(id, |l| l.id.clone())
    };
    let now: Vec<Entity> = drawn
        .entities
        .into_iter()
        .map(|mut e| {
            let id = ids(&e.base().layer_id);
            e.base_mut().layer_id = id;
            e
        })
        .collect();
    if bare(&block.entities) == bare(&now) {
        BuildingSectionState::Current
    } else {
        BuildingSectionState::Stale
    }
}

/// Every section mark of the drawing with its section's state, in the drawing's order; then the blocks of marks that
/// are gone.
pub fn rows(doc: &Document) -> Vec<BuildingSectionRow> {
    let mut out = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for (u, e) in doc.entities_with_uids() {
        let Entity::ViewMark(v) = e else { continue };
        if v.mark.form != ViewMarkForm::Section {
            continue;
        }
        let id = sections::block_id(u);
        seen.insert(id);
        let inserts = doc
            .entities()
            .filter(|x| matches!(x, Entity::Insert(i) if i.block == id))
            .count() as u32;
        out.push(BuildingSectionRow {
            mark: Some(u.to_string()),
            name: format!("{} kesiti", sections::label(&v.mark.mark)),
            block: doc.block(id).map(|_| id.to_string()),
            state: state_of(doc, u, &v.mark),
            inserts,
        });
    }
    for b in doc.blocks() {
        if b.name.starts_with("MM-XXXX-KESIT")
            && !seen.contains(&b.id)
            && b.description
                .as_deref()
                .is_some_and(|d| d.contains("kesit işaretinden çıkarır"))
        {
            out.push(BuildingSectionRow {
                mark: None,
                name: format!("{} (işareti yok)", b.name),
                block: Some(b.id.to_string()),
                state: BuildingSectionState::Gone,
                inserts: doc
                    .entities()
                    .filter(|x| matches!(x, Entity::Insert(i) if i.block == b.id))
                    .count() as u32,
            });
        }
    }
    out
}

pub fn validate(cx: &ExecutionContext<'_>, input: &BuildingSectionsQuery) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

fn check(doc: &Document, input: &BuildingSectionsQuery) -> Result<Vec<BuildingSectionRow>, Stop> {
    if let Some(m) = &input.mark {
        let (u, _) = mark_of(doc, m, "mark")?;
        return Ok(rows(doc)
            .into_iter()
            .filter(|r| r.mark.as_deref() == Some(&u.to_string()))
            .collect());
    }
    if let Some(uid) = &input.uid {
        if !checks::is_uid_text(uid) {
            return Err(Stop::failed(error(
                codes::INVALID_UID,
                format!("“{uid}” geçerli bir nesne kimliği değil."),
                Some("uid".into()),
            )));
        }
        let Some(Entity::Insert(i)) = Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|s| doc.get(s))
        else {
            return Err(Stop::failed(error(
                codes::ENTITY_NOT_FOUND,
                format!("“{uid}” kimlikli bir kesit yerleşimi çizimde yok."),
                Some("uid".into()),
            )));
        };
        let block = i.block.to_string();
        return Ok(rows(doc)
            .into_iter()
            .filter(|r| r.block.as_deref() == Some(block.as_str()))
            .collect());
    }
    Ok(rows(doc))
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &BuildingSectionsQuery,
) -> CommandResult<BuildingSectionsQueried> {
    execute_in(cx.doc, input)
}

pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: BuildingSectionsQuery,
) -> CommandResult<BuildingSectionsQueried> {
    execute_in(cx.doc, &input)
}

/// [`execute`] on a document (read-only).
pub fn execute_in(
    doc: &Document,
    input: &BuildingSectionsQuery,
) -> CommandResult<BuildingSectionsQueried> {
    match check(doc, input) {
        Ok(sections) => CommandResult::Completed {
            output: BuildingSectionsQueried {
                sections,
                revision: doc.revision().to_string(),
            },
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}
