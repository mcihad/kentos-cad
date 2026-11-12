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
    let Some(drawn) = sections::draw_mark(doc, fields) else {
        return BuildingSectionState::Stale;
    };
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
        if !matches!(v.mark.form, ViewMarkForm::Section | ViewMarkForm::Elevation) {
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
            name: sections::view_name(doc, &v.mark),
            block: doc.block(id).map(|_| id.to_string()),
            state: state_of(doc, u, &v.mark),
            inserts,
        });
    }
    for b in doc.blocks() {
        if (b.name.starts_with("MM-XXXX-KESIT") || b.name.starts_with("MM-XXXX-GORUNUS"))
            && !seen.contains(&b.id)
            && b.description
                .as_deref()
                .is_some_and(|d| {
                    d.contains("KentOS kesit işaretinden çıkarır")
                        || d.contains("KentOS görünüş işaretinden çıkarır")
                })
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

/// The stale sections the sheets `sheets` of `book` print (docs/adr/8042 §6): a placement of a stale section within a
/// fixed map's ground, by the sheet's id and the section's name ("B-B kesiti"), in the sheets' order. Publication says
/// them before it writes, as Archicad's Drawing Manager marks a drawing whose source changed.
pub fn stale_on_sheets(
    doc: &Document,
    book: &kentos_sheet::SheetBook,
    sheets: &[String],
) -> Vec<(String, String)> {
    let mut stale: Vec<(kentos_commands::BlockId, String)> = Vec::new();
    for (u, e) in doc.entities_with_uids() {
        let Entity::ViewMark(v) = e else { continue };
        if matches!(v.mark.form, ViewMarkForm::Section | ViewMarkForm::Elevation)
            && doc.block(sections::block_id(u)).is_some()
            && state_of(doc, u, &v.mark) == BuildingSectionState::Stale
        {
            stale.push((sections::block_id(u), sections::view_name(doc, &v.mark)));
        }
    }
    if stale.is_empty() {
        return Vec::new();
    }
    let placed: Vec<([f64; 4], &str, &str)> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Insert(i) => stale
                .iter()
                .find(|(b, _)| *b == i.block)
                .and_then(|(_, name)| {
                    crate::sheets_fit::extent_of(doc, std::iter::once(e))
                        .map(|x| (x, name.as_str(), e.base().layer_id.as_str()))
                }),
            _ => None,
        })
        .collect();
    let mut out: Vec<(String, String)> = Vec::new();
    for id in sheets {
        let Some(sheet) = book.sheet(id) else {
            continue;
        };
        for item in &sheet.items {
            let Some(c) = kentos_sheet::display::fixed_corners(item) else {
                continue;
            };
            let kentos_sheet::ItemKind::Map(m) = &item.kind else {
                continue;
            };
            // A layer the map does not draw (frozen in it, or left out of its list) prints nothing of it.
            let drawn = |layer: &str| {
                !m.layer_overrides
                    .iter()
                    .any(|o| o.frozen && o.layer == layer)
                    && match &m.layers {
                        kentos_sheet::kinds::MapLayers::List(l) => {
                            l.layers.iter().any(|x| x == layer)
                        }
                        _ => true,
                    }
            };
            let (x0, x1) = c
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), q| {
                    (a.min(q[0]), b.max(q[0]))
                });
            let (y0, y1) = c
                .iter()
                .fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), q| {
                    (a.min(q[1]), b.max(q[1]))
                });
            for (b, name, layer) in &placed {
                if drawn(layer)
                    && b[0] <= x1
                    && b[2] >= x0
                    && b[1] <= y1
                    && b[3] >= y0
                    && !out.iter().any(|(s, n)| s == id && n == name)
                {
                    out.push((id.clone(), (*name).to_owned()));
                }
            }
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
