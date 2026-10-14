//! `cad.entities.offset` v1 (docs/adr/0246): parallel copies of objects
//! named by their persistent ids, from what was asked (a distance and a
//! side, or a point to pass through; one side or both; the sources kept or
//! deleted; a target layer), as one undo step “Ötele”. The desktop's handler
//! over the native document, run by the Ötele tool (its preview is
//! [`plan_in`], its click [`execute`]), the headless server, Python and MCP.
//! The shared cases are `fixtures/commands/v1/cad.entities.offset.json`.
//!
//! Nothing is computed twice: the copies are the shared core's
//! (`offset_entity`, `through_distance`), as the tool always took them, and
//! they are written through `cad.entities.edit`'s `offset` operation
//! ([`crate::edit`]), which gives each copy its source's layer, colour, line
//! weight, elevations and measures. The plan and the write are that one
//! computation over the same document.
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one id; every id lowercase UUID text with hyphens (in order);
//! 2. the distance and the point finite, the distance above zero;
//! 3. the expected revision (every command's, `checks.rs`);
//! 4. every id names an object of the document (in order; a repeated id once);
//! 5. no object on a locked layer; every object of a kind that is offset;
//! 6. the target layer, when one is given;
//! 7. at least one copy made (each that is not, a warning: `radius_collapse`
//!    when the distance overruns a radius or the area's width, else
//!    `offset_failed`).
//!
//! What a copy says besides (docs/adr/0293): `approximate_offset` for an
//! ellipse's, a fit-point or NURBS curve's (a polyline within the core's
//! tolerance of the curve's parallel), `self_intersecting` when its edges
//! cross; the copy is written all the same.

use kentos_contracts::{
    CommandResult, CommandWarning, EditOperation, EntitiesEdit, EntitiesOffset, EntitiesOffsetMade,
    EntitiesOffsetPlan, Entity, EntityEdit, OffsetBy,
};
use kentos_domain::Document;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::intersect::closest_on_edge;
use kentos_geometry_core::ops::curve_cuts::Geometry;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::ops::offset::{OffsetNote, collapses, offset_noted, through_distance};
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;
use crate::edit;
use crate::geometry::{edit_geometry, shape};

/// The undo step's name: the tool's (docs/adr/0047).
pub const LABEL: &str = "Ötele";

/// How far past the object the other side's point is put, metres: enough
/// to be on the far side, close enough to be judged at the same edge (the
/// Ötele tool's İki yana, docs/adr/0140).
const OTHER_SIDE: f64 = 1e-3;

/// What a kind that is not offset is told: the core's own words.
const NOT_OFFSETTABLE: &str = "Yalnızca çizgi, çoklu çizgi, kapalı alan, daire, yay, elips, eğri ve yardımcı çizgiler ötelenebilir.";

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesOffset) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now: the copies as they would be, and the
/// revision to expect for exactly that; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &EntitiesOffset,
) -> CommandResult<EntitiesOffsetPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document alone (the tool's preview).
pub fn plan_in(doc: &Document, input: &EntitiesOffset) -> CommandResult<EntitiesOffsetPlan> {
    let checked = match check(doc, input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    match edit::plan_in(doc, &checked.edit) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: EntitiesOffsetPlan {
                sources: checked.sources,
                entities: output.created,
                removed: output.removed,
                distances: checked.distances,
                revision: output.revision,
            },
            warnings: [checked.warnings, warnings].concat(),
        },
        other => relay(other),
    }
}

/// Checks `input` again and writes the copies, and with `deleteSource` the
/// deletion of their sources, as one undo step “Ötele”, into the open
/// transaction or group if one is.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesOffset,
) -> CommandResult<EntitiesOffsetMade> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_ENTITIES_OFFSET,
        kentos_contracts::CAD_ENTITIES_OFFSET_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesOffset,
) -> CommandResult<EntitiesOffsetMade> {
    let checked = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    match edit::execute(cx, checked.edit) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: EntitiesOffsetMade {
                created: output.created,
                removed: output.removed,
                distances: checked.distances,
                revision: output.revision,
            },
            warnings: [checked.warnings, warnings].concat(),
        },
        other => relay(other),
    }
}

/// The edit's answer that wrote nothing, as this command's.
fn relay<T, U>(result: CommandResult<T>) -> CommandResult<U> {
    match result {
        CommandResult::Completed { .. } | CommandResult::Cancelled => CommandResult::Cancelled,
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::Failed { error } => CommandResult::Failed { error },
    }
}

/// What may be written: the sources, their distances, the edit that writes
/// the copies and the warnings about copies left out.
struct Checked {
    sources: Vec<String>,
    distances: Vec<f64>,
    edit: EntitiesEdit,
    warnings: Vec<CommandWarning>,
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// Whether the core offsets an object of this kind.
pub fn offsettable(entity: &Entity) -> bool {
    matches!(
        entity,
        Entity::Line(_)
            | Entity::Polyline(_)
            | Entity::Polygon(_)
            | Entity::Circle(_)
            | Entity::Arc(_)
            | Entity::Ellipse(_)
            | Entity::Spline(_)
            | Entity::Nurbs(_)
            | Entity::Xline(_)
            | Entity::Ray(_)
    )
}

/// A point just across `target` from `side`: where İki yana's second copy
/// is judged from. `None` when `side` is on the object (it has no far side).
pub fn across(target: &Shape, side: Vec2) -> Option<Vec2> {
    let nearest = entity_edges(target)
        .iter()
        .map(|edge| closest_on_edge(edge, side))
        .min_by(|a, b| a.d.total_cmp(&b.d))?;
    if nearest.d < 1e-9 {
        return None;
    }
    let (ux, uy) = (
        (nearest.p.x - side.x) / nearest.d,
        (nearest.p.y - side.y) / nearest.d,
    );
    Some(Vec2::new(
        nearest.p.x + ux * OTHER_SIDE,
        nearest.p.y + uy * OTHER_SIDE,
    ))
}

/// The copies of `target` asked for: on the side of `side` and, with
/// `both`, on the other; each the copy or why it is not made, and what it says.
pub fn copies(target: &Shape, d: f64, side: Vec2, both: bool) -> Vec<(Geometry, OffsetNote)> {
    let mut out = vec![offset_noted(target, d, side)];
    if both {
        match across(target, side) {
            Some(other) => out.push(offset_noted(target, d, other)),
            None => out.push((
                Geometry::Error(
                    "Nokta nesnenin üzerinde; iki yana ötelemek için bir yanına tıklayın.".into(),
                ),
                OffsetNote::default(),
            )),
        }
    }
    out
}

/// The code an offset's refusal is said with.
fn failed_code(message: &str) -> &'static str {
    if collapses(message) {
        codes::RADIUS_COLLAPSE
    } else {
        codes::OFFSET_FAILED
    }
}

/// The input's own checks, in its fields' order.
fn check_input(input: &EntitiesOffset) -> Result<(), Stop> {
    checks::uids(&input.uids, "Ötelenecek nesne verilmedi.")?;
    match input.by {
        OffsetBy::Distance { distance, side } => {
            checks::finite(
                distance,
                "Öteleme mesafesi",
                "Mesafeyi sonlu bir sayıyla verin.",
                "by.distance",
            )?;
            checks::point(side, "Taraf noktasının", "by.side")?;
            if distance <= 0.0 {
                return Err(refuse(
                    codes::INVALID_DISTANCE,
                    "Öteleme mesafesi sıfırdan büyük olmalı. Pozitif bir mesafe (metre) verin."
                        .into(),
                    "by.distance",
                ));
            }
        }
        OffsetBy::Through { point } => {
            checks::point(point, "Geçilecek noktanın", "by.point")?;
        }
    }
    Ok(())
}

/// The checks in the contract's order, and the edit that writes the copies.
fn check(doc: &Document, input: &EntitiesOffset) -> Result<Checked, Stop> {
    check_input(input)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let found = checks::named(doc, &input.uids)?;
    let layers = doc.layers();
    for (i, _, entity, _) in &found {
        let layer = &entity.base().layer_id;
        if layers.is_locked(layer) {
            let name = layers
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(refuse(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki nesne ötelenemez. Kilidi Katmanlar panelinden açın."
                ),
                &format!("uids[{i}]"),
            ));
        }
        if !offsettable(entity) {
            return Err(refuse(
                codes::NOT_OFFSETTABLE,
                NOT_OFFSETTABLE.into(),
                &format!("uids[{i}]"),
            ));
        }
    }
    // The target layer: known, a layer, not locked; a hidden one is warned of.
    let mut warnings = match &input.layer_id {
        Some(layer) => checks::layer_at(doc, layer, "layerId")?,
        None => Vec::new(),
    };
    let (point, given) = match input.by {
        OffsetBy::Distance { distance, side } => (side, Some(distance)),
        OffsetBy::Through { point } => (point, None),
    };
    let point = Vec2::new(point.x, point.y);
    let mut sources = Vec::with_capacity(found.len());
    let mut distances = Vec::with_capacity(found.len());
    let mut adds = Vec::new();
    let mut removes = Vec::new();
    let mut first_error = None;
    for (i, _, entity, uid) in &found {
        let target = shape(entity);
        let d = given.unwrap_or_else(|| through_distance(&target, point));
        sources.push((*uid).clone());
        distances.push(d);
        let mut made = 0;
        for (copy, note) in copies(&target, d, point, input.both_sides) {
            let made_one = match copy {
                Geometry::Ok(copy) => {
                    if let Some(tol) = note.approximate {
                        warnings.push(CommandWarning {
                            code: codes::APPROXIMATE_OFFSET.into(),
                            message: format!(
                                "Eğrinin ötelenmişi yaklaşık: kopya, eğrinin paralelinden en çok {} mm uzaklaşan bir çoklu çizgi.",
                                approximate_mm(tol)
                            ),
                            path: Some(format!("uids[{i}]")),
                        });
                    }
                    if note.crossings > 0 {
                        warnings.push(CommandWarning {
                            code: codes::SELF_INTERSECTING.into(),
                            message: format!(
                                "Kopya kendini {} yerde kesiyor; fazlasını budayın ya da daha küçük bir mesafe verin.",
                                note.crossings
                            ),
                            path: Some(format!("uids[{i}]")),
                        });
                    }
                    edit_geometry(copy.shape)
                }
                Geometry::Error(e) => {
                    warnings.push(CommandWarning {
                        code: failed_code(&e).into(),
                        message: e.clone(),
                        path: Some(format!("uids[{i}]")),
                    });
                    first_error.get_or_insert(e);
                    None
                }
            };
            if let Some(geometry) = made_one {
                made += 1;
                adds.push(EntityEdit::Add {
                    from: (*uid).clone(),
                    geometry,
                    keep_data: None,
                    layer_id: input.layer_id.clone(),
                    color: None,
                    line_weight: None,
                    attrs: None,
                    label: None,
                });
            }
        }
        if input.delete_source && made > 0 {
            removes.push(EntityEdit::Remove {
                uid: (*uid).clone(),
            });
        }
    }
    if adds.is_empty() {
        let message =
            first_error.unwrap_or_else(|| "Öteleme sonucu geçerli bir şekil oluşmadı.".into());
        return Err(refuse(failed_code(&message), message, "uids"));
    }
    adds.extend(removes);
    Ok(Checked {
        sources,
        distances,
        edit: EntitiesEdit {
            operation: EditOperation::Offset,
            changes: adds,
            expected_revision: input.expected_revision.clone(),
        },
        warnings,
    })
}

/// A tolerance in millimetres as the warning writes it: “0.1”, “0.0054”.
fn approximate_mm(metres: f64) -> String {
    let mm = metres * 1000.0;
    let text = format!("{mm:.4}");
    text.trim_end_matches('0').trim_end_matches('.').to_owned()
}
