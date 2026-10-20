//! `cad.entities.stretch` v1 (docs/adr/0254 §2): the vertices inside a
//! window moved, the rest kept, as one undo step “Esnet”. The desktop's
//! handler over the native document, run by the Esnet tool (its preview is
//! [`plan_in`], its target point [`execute`]), the headless server, Python
//! and MCP. The shared cases are `fixtures/commands/v1/cad.entities.stretch.json`.
//!
//! Nothing is computed twice: each object's new shape is the shared core's
//! (`stretch_entity`), the neighbours that follow are
//! [`crate::topology_follow`]'s, and they are written through
//! `cad.entities.edit`'s `stretch` operation ([`crate::edit`]).
//!
//! What follows other objects does not stretch by itself (docs/adr/0297 §2):
//! an associative hatch, a linked text and a dimension's tied points stay
//! with their objects, and the step's end moves them; the plan and the answer
//! name them (`dependents`), the plan as they will be: the document's own rule
//! answers (`Document::followers`), on a copy the tool keeps between previews.
//!
//! The checks, in order (the first that fails answers):
//! 1. the window's corners, then `dx` and `dy`, finite;
//! 2. `uids`, when given, not empty and each lowercase UUID text with hyphens;
//! 3. the expected revision (every command's, `checks.rs`);
//! 4. every id names an object;
//! 5. at least one object left to stretch (a listed one without a vertex in
//!    the window, or any on a locked layer, is left out with a warning).

use std::collections::HashMap;

use kentos_commands::{
    CommandResult, CommandWarning, EditOperation, EntitiesEdit, EntitiesStretch,
    EntitiesStretchPlan, EntitiesStretched, Entity, EntityEdit, EntityId, SubRef,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::stretch::stretch_entity;
use kentos_geometry_core::store::Store;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::edit;
use crate::geometry::{edit_geometry, shape};
use crate::geometry_store::store_of;
use crate::topology_follow::{self, slot};

/// The undo step's name: the tool's (docs/adr/0047).
pub const LABEL: &str = "Esnet";

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesStretch) -> CommandResult<()> {
    match check(cx.doc, cx.store, input) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &EntitiesStretch,
) -> CommandResult<EntitiesStretchPlan> {
    plan_in(cx.doc, cx.store, input)
}

/// [`plan`] over the document and, when the host has one, its store.
pub fn plan_in(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesStretch,
) -> CommandResult<EntitiesStretchPlan> {
    plan_with(doc, store, input, &mut None)
}

/// [`plan_in`] with a copy of the document the caller keeps between calls for
/// the objects that follow (the tool's preview at every move,
/// `Document::followers_with`).
pub fn plan_with(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesStretch,
    scratch: &mut Option<Document>,
) -> CommandResult<EntitiesStretchPlan> {
    let checked = match check(doc, store, input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let Some(change) = &checked.edit else {
        return CommandResult::Completed {
            output: EntitiesStretchPlan {
                stretched: Vec::new(),
                followed: Vec::new(),
                dependents: Vec::new(),
                revision: doc.revision().to_string(),
            },
            warnings: checked.warnings,
        };
    };
    match edit::plan_in(doc, change) {
        CommandResult::Completed { output, warnings } => {
            let mut stretched = output.changed;
            let mut followed = stretched.split_off(checked.stretched.min(stretched.len()));
            let dependents = carried(doc, scratch, &mut stretched, &mut followed);
            CommandResult::Completed {
                output: EntitiesStretchPlan {
                    stretched,
                    followed,
                    dependents,
                    revision: output.revision,
                },
                warnings: [checked.warnings, warnings].concat(),
            }
        }
        other => crate::trim::relay(other),
    }
}

/// Checks `input` again and writes the stretch as one undo step, into the
/// open transaction or group if one is.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesStretch,
) -> CommandResult<EntitiesStretched> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ENTITIES_STRETCH,
        kentos_commands::CAD_ENTITIES_STRETCH_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesStretch,
) -> CommandResult<EntitiesStretched> {
    let checked = match check(cx.doc, cx.store, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let Some(change) = checked.edit else {
        return CommandResult::Completed {
            output: EntitiesStretched {
                stretched: Vec::new(),
                followed: Vec::new(),
                dependents: Vec::new(),
                revision: cx.doc.revision().to_string(),
            },
            warnings: checked.warnings,
        };
    };
    // The objects the step's end will move with them, found before it is written (only when anything in the
    // drawing can follow: the plan is the edit's own check, run once more).
    let dependents = if cx.doc.can_follow() {
        match edit::plan_in(cx.doc, &change) {
            CommandResult::Completed { output, .. } => {
                let mut stretched = output.changed;
                let mut followed = stretched.split_off(checked.stretched.min(stretched.len()));
                carried(cx.doc, &mut None, &mut stretched, &mut followed)
                    .iter()
                    .filter_map(|e| cx.doc.uid(Slot(e.base().id)).map(|u| u.to_string()))
                    .collect()
            }
            _ => Vec::new(),
        }
    } else {
        Vec::new()
    };
    match edit::execute(cx, change) {
        CommandResult::Completed { output, warnings } => {
            let mut stretched = output.changed;
            let followed = stretched.split_off(checked.stretched.min(stretched.len()));
            CommandResult::Completed {
                output: EntitiesStretched {
                    stretched,
                    followed,
                    dependents,
                    revision: output.revision,
                },
                warnings: [checked.warnings, warnings].concat(),
            }
        }
        other => crate::trim::relay(other),
    }
}

struct Checked {
    /// The edit to write; none for a zero displacement.
    edit: Option<EntitiesEdit>,
    /// How many of the edit's changes are the stretched objects (the neighbours' after them).
    stretched: usize,
    warnings: Vec<CommandWarning>,
}

fn warning(code: &str, message: String, path: Option<String>) -> CommandWarning {
    CommandWarning {
        code: code.into(),
        message,
        path,
    }
}

/// The objects that follow the stretched and the neighbours (docs/adr/0297 §3), as they will be: a stretched or
/// neighbour object that follows too is given as it ends where it is listed; the others are the answer.
fn carried(
    doc: &Document,
    scratch: &mut Option<Document>,
    stretched: &mut [Entity],
    followed: &mut [Entity],
) -> Vec<Entity> {
    let changes: Vec<(Slot, Entity)> = stretched
        .iter()
        .chain(followed.iter())
        .map(|e| (Slot(e.base().id), e.clone()))
        .collect();
    let after = doc.followers_with(scratch, &changes);
    if after.is_empty() {
        return Vec::new();
    }
    let mut at: HashMap<u32, Entity> = after.into_iter().map(|(s, e)| (s.0, e)).collect();
    for e in stretched.iter_mut().chain(followed.iter_mut()) {
        if let Some(ends) = at.remove(&e.base().id) {
            *e = ends;
        }
    }
    let mut rest: Vec<Entity> = at.into_values().collect();
    rest.sort_by_key(|e| doc.place(Slot(e.base().id)));
    rest
}

/// What of an object follows the objects it is tied to (docs/adr/0297 §2).
enum Ties {
    /// Nothing: it stretches by its own vertices.
    Free,
    /// All of it: an associative hatch whose closed object is there, a linked text whose object is.
    Whole,
    /// A dimension's points `a`, `b`, `c` tied to objects that are there.
    Points([bool; 3]),
}

fn ties(doc: &Document, e: &Entity) -> Ties {
    let there = |id: &EntityId| doc.slot_of(Uuid::from_bytes(id.0)).is_some();
    match e {
        Entity::Hatch(h)
            if h.assoc
                .as_ref()
                .is_some_and(|a| a.outer.as_ref().map_or(a.bounds.iter().any(there), there)) =>
        {
            Ties::Whole
        }
        Entity::Text(t) if t.label_of.as_ref().is_some_and(there) => Ties::Whole,
        Entity::Dimension(d) => match &d.refs {
            Some(r) => {
                let held =
                    |s: &Option<SubRef>| s.as_ref().is_some_and(|s| s.live() && there(&s.uid));
                Ties::Points([held(&r.a), held(&r.b), held(&r.c)])
            }
            None => Ties::Free,
        },
        _ => Ties::Free,
    }
}

/// How the window takes an object.
enum Moved {
    /// None of its vertices is in the window.
    Outside,
    /// Its vertices in the window are those of what it follows: it is left to them.
    Follows,
    /// Its shape with its vertices in the window moved (a dimension's tied points kept).
    To(Box<Shape>),
}

/// The object as the core takes it, the window in which its vertices move, and what of it follows other objects.
fn moves(doc: &Document, e: &Entity, window: &Bounds, dx: f64, dy: f64) -> Moved {
    let was = shape(e);
    let Some(mut after) =
        stretch_entity(&CoreEntity::new(was.clone()), window, dx, dy).map(|g| g.shape)
    else {
        return Moved::Outside;
    };
    match ties(doc, e) {
        Ties::Free => Moved::To(Box::new(after)),
        Ties::Whole => Moved::Follows,
        Ties::Points(held) => {
            let (
                Shape::Dimension { a, b, c, .. },
                Shape::Dimension {
                    a: a2,
                    b: b2,
                    c: c2,
                    ..
                },
            ) = (&was, &mut after)
            else {
                return Moved::To(Box::new(after));
            };
            let inside = |p: kentos_geometry_core::Vec2| {
                p.x >= window.min_x
                    && p.x <= window.max_x
                    && p.y >= window.min_y
                    && p.y <= window.max_y
            };
            let points = [Some(*a), Some(*b), *c];
            let free = points
                .iter()
                .zip(held)
                .any(|(p, held)| p.is_some_and(|p| inside(p) && !held));
            if !free {
                return Moved::Follows;
            }
            if held[0] {
                *a2 = *a;
            }
            if held[1] {
                *b2 = *b;
            }
            if held[2] {
                *c2 = *c;
            }
            Moved::To(Box::new(after))
        }
    }
}

/// The objects the window would stretch, by persistent id, and the
/// warnings of those left out; or the refusal (the tool asks when its
/// window closes, before it knows the displacement).
pub fn objects_in(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesStretch,
) -> CommandResult<Vec<String>> {
    let own;
    let store = match store {
        Some(s) => s,
        None => {
            own = store_of(doc);
            &own
        }
    };
    match found(doc, store, input) {
        Ok((_, targets, warnings)) => CommandResult::Completed {
            output: targets
                .iter()
                .filter_map(|s| doc.uid(*s).map(|u| u.to_string()))
                .collect(),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

fn check(doc: &Document, store: Option<&Store>, input: &EntitiesStretch) -> Result<Checked, Stop> {
    // The host's store when it keeps one, else one of the document: the window's
    // objects and the neighbours are the shown ones.
    let own;
    let store = match store {
        Some(s) => s,
        None => {
            own = store_of(doc);
            &own
        }
    };
    let (window, targets, mut warnings) = found(doc, store, input)?;
    if input.dx == 0.0 && input.dy == 0.0 {
        return Ok(Checked {
            edit: None,
            stretched: 0,
            warnings,
        });
    }
    let mut changes = Vec::with_capacity(targets.len());
    let mut edits = Vec::with_capacity(targets.len());
    for s in &targets {
        let Some(e) = doc.get(*s) else { continue };
        let Moved::To(after) = moves(doc, e, &window, input.dx, input.dy) else {
            continue;
        };
        let after = *after;
        let (Some(uid), Some(geometry)) = (doc.uid(*s), edit_geometry(after.clone())) else {
            continue;
        };
        changes.push(EntityEdit::Update {
            uid: uid.to_string(),
            geometry,
        });
        edits.push((*s, shape(e), after));
    }
    let stretched = changes.len();
    if let Some(topology) = input.topology
        && let Some(n) = topology_follow::neighbours(doc, store, topology.points, &edits)
    {
        changes.extend(n.changes);
        if n.locked > 0 {
            warnings.push(warning(
                codes::NEIGHBOUR_LOCKED,
                format!(
                    "Kilitli katmandaki {} komşu nesne değişmedi; ortak sınır ayrıldı.",
                    n.locked
                ),
                None,
            ));
        }
        if n.invalid > 0 {
            warnings.push(warning(
                codes::NEIGHBOUR_INVALID,
                format!(
                    "{} komşu nesne geçersiz kalacağı için değişmedi (açık yolda 2'den, halkada 3'ten az köşe).",
                    n.invalid
                ),
                None,
            ));
        }
    }
    Ok(Checked {
        edit: Some(EntitiesEdit {
            operation: EditOperation::Stretch,
            changes,
            expected_revision: None,
        }),
        stretched,
        warnings,
    })
}

/// The checks up to the objects: the window, the objects it stretches (in
/// the input's or the drawing's order) and the warnings of those left out.
fn found(
    doc: &Document,
    store: &Store,
    input: &EntitiesStretch,
) -> Result<(Bounds, Vec<Slot>, Vec<CommandWarning>), Stop> {
    let w = &input.window;
    checks::point(w.min, "Pencerenin köşesinin", "window.min")?;
    checks::point(w.max, "Pencerenin köşesinin", "window.max")?;
    checks::finite(
        input.dx,
        "Doğuya (Y) yer değiştirme",
        "Yer değiştirmeyi sonlu bir sayıyla verin.",
        "dx",
    )?;
    checks::finite(
        input.dy,
        "Kuzeye (X) yer değiştirme",
        "Yer değiştirmeyi sonlu bir sayıyla verin.",
        "dy",
    )?;
    if let Some(uids) = &input.uids {
        checks::uids(uids, "Esnetilecek nesne yok.")?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let window = Bounds {
        min_x: w.min.x.min(w.max.x),
        min_y: w.min.y.min(w.max.y),
        max_x: w.min.x.max(w.max.x),
        max_y: w.min.y.max(w.max.y),
    };
    let layers = doc.layers();
    let mut warnings = Vec::new();
    let mut targets: Vec<Slot> = Vec::new();
    let mut locked: Vec<String> = Vec::new();
    match &input.uids {
        Some(uids) => {
            for (i, slot, e, uid) in checks::named(doc, uids)? {
                let moved = moves(doc, e, &window, 0.0, 0.0);
                if matches!(moved, Moved::Outside) {
                    warnings.push(warning(
                        codes::OUTSIDE_WINDOW,
                        "Nesnenin pencerede köşesi yok; esnemedi.".into(),
                        Some(format!("uids[{i}]")),
                    ));
                } else if matches!(moved, Moved::Follows) {
                    warnings.push(warning(
                        codes::FOLLOWS_OBJECT,
                        "Nesne bağlı olduğu nesneyi izler; kendisi esnemedi.".into(),
                        Some(format!("uids[{i}]")),
                    ));
                } else if layers.is_locked(&e.base().layer_id) {
                    locked.push(uid.clone());
                    warnings.push(warning(
                        codes::LOCKED_SKIPPED,
                        "Nesne kilitli katmanda olduğu için atlandı.".into(),
                        Some(format!("uids[{i}]")),
                    ));
                } else {
                    targets.push(slot);
                }
            }
        }
        None => {
            let mut found: Vec<Slot> = store
                .in_rect(&window, true)
                .into_iter()
                .filter_map(slot)
                .filter(|s| {
                    doc.get(*s)
                        .is_some_and(|e| matches!(moves(doc, e, &window, 0.0, 0.0), Moved::To(_)))
                })
                .collect();
            found.sort_unstable();
            let all = found.len();
            for s in found {
                let Some(e) = doc.get(s) else { continue };
                if layers.is_locked(&e.base().layer_id) {
                    locked.extend(doc.uid(s).map(|u| u.to_string()));
                } else {
                    targets.push(s);
                }
            }
            if targets.len() < all {
                warnings.push(warning(
                    codes::LOCKED_SKIPPED,
                    format!(
                        "{} nesne kilitli katmanda olduğu için atlandı.",
                        all - targets.len()
                    ),
                    None,
                ));
            }
        }
    }
    if targets.is_empty() {
        // The objects left out for their locked layer are named (docs/adr/0253 §7).
        return Err(Stop::failed(kentos_commands::CommandError {
            uids: locked,
            ..error(
                codes::NOTHING_TO_STRETCH,
                "Pencerede köşesi olan düzenlenebilir nesne yok. Pencereyi taşınacak köşelerin üstüne verin.".into(),
                Some("window".into()),
            )
        }));
    }
    Ok((window, targets, warnings))
}
