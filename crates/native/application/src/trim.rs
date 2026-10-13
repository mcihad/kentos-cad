//! `cad.entities.trim` v1 and `cad.entities.extend` v1 (docs/adr/0250):
//! objects cut back to, or grown up to, their boundaries, from what was
//! asked (the picks and the boundaries), as one undo step “Buda” (“Uzat”).
//! The desktop's handler over the native document, run by the Buda and Uzat
//! tools (their preview is [`plan_in`], their click and fence [`execute`]),
//! the headless server, Python and MCP. The shared cases are
//! `fixtures/commands/v1/cad.entities.trim.json` and `.extend.json`.
//!
//! Nothing is computed twice: the pieces are the shared geometry store's
//! (`trim_preview`, `extend_preview`: the boundaries near the target, then
//! the core's `trim_entity`, `extend_entity`), as the tools always took
//! them, and they are written through `cad.entities.edit`'s `trim` and
//! `extend` operations ([`crate::edit`]).
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one pick; a boundary list not empty;
//! 2. every id lowercase UUID text with hyphens (the picks', then the boundaries');
//! 3. every point finite (the picks', then the rectangle's);
//! 4. the expected revision (every command's, `checks.rs`);
//! 5. every id names an object (the picks', then the boundaries');
//! 6. no picked object on a locked layer; for Buda no area with holes;
//! 7. at least one pick does something (each that does not, a warning).

use std::collections::BTreeMap;

use kentos_contracts::{
    CommandResult, CommandWarning, EditOperation, EntitiesEdit, EntitiesTrim, EntitiesTrimPlan,
    EntitiesTrimmed, Entity, EntityEdit, TrimBoundaries,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::{Entity as CoreEntity, Shape};
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::curve_cuts::{Cut, Geometry};
use kentos_geometry_core::ops::offset::through_distance;
use kentos_geometry_core::ops::path::{nearest_s, path_of};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;
use crate::edit;
use crate::geometry::{edit_geometry, shape};
use crate::geometry_store::store_of;

/// What the command does to its picks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Act {
    Trim,
    Extend,
}

impl Act {
    /// The undo step's name: the tool's (docs/adr/0047).
    pub fn label(self) -> &'static str {
        match self {
            Act::Trim => "Buda",
            Act::Extend => "Uzat",
        }
    }

    fn failed(self) -> &'static str {
        match self {
            Act::Trim => codes::TRIM_FAILED,
            Act::Extend => codes::EXTEND_FAILED,
        }
    }

    fn operation(self) -> EditOperation {
        match self {
            Act::Trim => EditOperation::Trim,
            Act::Extend => EditOperation::Extend,
        }
    }
}

/// How near a later pick must lie to a piece an earlier one left to be on it, metres (the fence's rule).
const ON_PIECE: f64 = 1e-6;

/// The words of a cut or a growth that came to nothing, the core having said none.
const NOTHING: [&str; 2] = [
    "Budanacak bir parça yok; sınırlar bu noktanın iki yanında nesneyi kesmiyor.",
    "Uzatılacak bir uç yok; uç hiçbir sınıra ulaşmıyor.",
];

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesTrim, act: Act) -> CommandResult<()> {
    match check(cx.doc, cx.store, input, act) {
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
    input: &EntitiesTrim,
    act: Act,
) -> CommandResult<EntitiesTrimPlan> {
    plan_in(cx.doc, cx.store, input, act)
}

/// [`plan`] over the document and, when the host has one, its store (the tools' preview).
pub fn plan_in(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesTrim,
    act: Act,
) -> CommandResult<EntitiesTrimPlan> {
    let checked = match check(doc, store, input, act) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    match edit::plan_in(doc, &checked.edit) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: EntitiesTrimPlan {
                changed: output.changed,
                created: output.created,
                removed: output.removed,
                revision: output.revision,
            },
            warnings: [checked.warnings, warnings].concat(),
        },
        other => relay(other),
    }
}

/// Checks `input` again and writes what the picks do as one undo step, into
/// the open transaction or group if one is.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesTrim,
    act: Act,
) -> CommandResult<EntitiesTrimmed> {
    let checked = match check(cx.doc, cx.store, &input, act) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    match edit::execute(cx, checked.edit) {
        CommandResult::Completed { output, warnings } => CommandResult::Completed {
            output: EntitiesTrimmed {
                changed: output.changed,
                created: output.created,
                removed: output.removed,
                revision: output.revision,
            },
            warnings: [checked.warnings, warnings].concat(),
        },
        other => relay(other),
    }
}

pub(crate) fn relay<T, U>(result: CommandResult<T>) -> CommandResult<U> {
    match result {
        CommandResult::Completed { .. } | CommandResult::Cancelled => CommandResult::Cancelled,
        CommandResult::Queued { job_id } => CommandResult::Queued { job_id },
        CommandResult::NeedsInput { error } => CommandResult::NeedsInput { error },
        CommandResult::Conflict { error } => CommandResult::Conflict { error },
        CommandResult::Failed { error } => CommandResult::Failed { error },
    }
}

struct Checked {
    edit: EntitiesEdit,
    warnings: Vec<CommandWarning>,
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The changes that replace the object at `slot` by `pieces`: the first
/// piece is the object itself, the others new objects from it; attributes
/// and the label survive a single piece of anything but a closed area. No
/// piece deletes it (the tools' rule since docs/adr/0047).
pub fn replace_changes(doc: &Document, slot: Slot, pieces: &[Shape]) -> Vec<EntityEdit> {
    let polygon = matches!(doc.get(slot), Some(Entity::Polygon(_)));
    let keep = (pieces.len() == 1 && !polygon).then_some(true);
    let id = doc.uid(slot).map(|u| u.to_string()).unwrap_or_default();
    let mut changes = Vec::with_capacity(pieces.len().max(1));
    let mut geometries = pieces.iter().filter_map(|s| edit_geometry(s.clone()));
    match geometries.next() {
        Some(first) => changes.push(EntityEdit::Replace {
            uid: id.clone(),
            geometry: first,
            keep_data: keep,
        }),
        None => changes.push(EntityEdit::Remove { uid: id.clone() }),
    }
    for piece in geometries {
        changes.push(EntityEdit::add(id.clone(), piece, keep));
    }
    changes
}

/// The piece a later pick lies on, of those an earlier one left (the fence's rule).
fn piece_at(pieces: &[Shape], at: Vec2) -> Option<usize> {
    pieces
        .iter()
        .enumerate()
        .map(|(i, p)| (i, through_distance(p, at)))
        .filter(|(_, d)| *d < ON_PIECE)
        .min_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

fn find(doc: &Document, uid: &str) -> Option<Slot> {
    Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u))
}

fn not_found(uid: &str, path: String) -> Stop {
    refuse(
        codes::ENTITY_NOT_FOUND,
        format!(
            "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
        ),
        &path,
    )
}

fn check(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesTrim,
    act: Act,
) -> Result<Checked, Stop> {
    if input.picks.is_empty() {
        let what = match act {
            Act::Trim => "Budanacak",
            Act::Extend => "Uzatılacak",
        };
        return Err(refuse(
            codes::NO_ENTITIES,
            format!("{what} nesne yok. En az bir nesneyi ve tıklandığı yeri verin."),
            "picks",
        ));
    }
    let listed = match &input.boundaries {
        Some(TrimBoundaries::Objects { uids }) => Some(uids.as_slice()),
        _ => None,
    };
    if listed.is_some_and(<[String]>::is_empty) {
        return Err(refuse(
            codes::NO_ENTITIES,
            "Sınır nesnesi yok. Sınırları verin ya da bütün görünen nesneler için “all”u kullanın."
                .into(),
            "boundaries.uids",
        ));
    }
    let uids = input
        .picks
        .iter()
        .enumerate()
        .map(|(i, p)| (p.uid.as_str(), format!("picks[{i}].uid")))
        .chain(
            listed
                .unwrap_or_default()
                .iter()
                .enumerate()
                .map(|(i, u)| (u.as_str(), format!("boundaries.uids[{i}]"))),
        );
    for (uid, path) in uids.clone() {
        if !is_uid_text(uid) {
            return Err(refuse(
                codes::INVALID_UID,
                format!(
                    "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
                ),
                &path,
            ));
        }
    }
    for (i, p) in input.picks.iter().enumerate() {
        checks::point(p.at, "Tıklanan noktanın", &format!("picks[{i}].at"))?;
    }
    if let Some(TrimBoundaries::Area { min, max }) = &input.boundaries {
        checks::point(*min, "Dikdörtgenin köşesinin", "boundaries.min")?;
        checks::point(*max, "Dikdörtgenin köşesinin", "boundaries.max")?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut slots = Vec::with_capacity(input.picks.len());
    for (i, p) in input.picks.iter().enumerate() {
        slots.push(find(doc, &p.uid).ok_or_else(|| not_found(&p.uid, format!("picks[{i}].uid")))?);
    }
    let mut chosen: Option<Vec<f64>> = None;
    if let Some(list) = listed {
        let mut ids = Vec::with_capacity(list.len());
        for (i, u) in list.iter().enumerate() {
            let slot = find(doc, u).ok_or_else(|| not_found(u, format!("boundaries.uids[{i}]")))?;
            ids.push(f64::from(slot.0));
        }
        chosen = Some(ids);
    }
    let layers = doc.layers();
    for (i, slot) in slots.iter().enumerate() {
        let Some(e) = doc.get(*slot) else { continue };
        let layer = &e.base().layer_id;
        if layers.is_locked(layer) {
            let name = layers
                .get(layer)
                .map_or(layer.as_str(), |n| n.name.as_str());
            return Err(refuse(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; üzerindeki nesne değiştirilemez. Kilidi Katmanlar panelinden açın."
                ),
                &format!("picks[{i}]"),
            ));
        }
        if act == Act::Trim
            && matches!(e, Entity::Polygon(p) if p.holes.as_ref().is_some_and(|h| !h.is_empty()))
        {
            return Err(refuse(
                codes::NOT_TRIMMABLE,
                "Adalı alanda budama yapılamaz; iç halkalar kaybolurdu. Önce Patlat ile halkalarına ayırın ya da Alan böl kullanın.".into(),
                &format!("picks[{i}]"),
            ));
        }
    }
    // The boundaries: the host's store when it keeps one, else one of the document.
    let own;
    let store = match store {
        Some(s) => s,
        None => {
            own = store_of(doc);
            &own
        }
    };
    let view = match &input.boundaries {
        Some(TrimBoundaries::Area { min, max }) => Bounds {
            min_x: min.x.min(max.x),
            min_y: min.y.min(max.y),
            max_x: min.x.max(max.x),
            max_y: min.y.max(max.y),
        },
        _ => Bounds {
            min_x: f64::MIN,
            min_y: f64::MIN,
            max_x: f64::MAX,
            max_y: f64::MAX,
        },
    };
    // The picks of each object, in the order the objects are first picked.
    let mut order: Vec<Slot> = Vec::new();
    let mut picks: BTreeMap<Slot, Vec<(usize, Vec2)>> = BTreeMap::new();
    for (i, (p, slot)) in input.picks.iter().zip(&slots).enumerate() {
        let at = Vec2::new(p.at.x, p.at.y);
        let list = picks.entry(*slot).or_default();
        if list.is_empty() {
            order.push(*slot);
        }
        list.push((i, at));
    }
    let mut changes = Vec::new();
    let mut warnings = Vec::new();
    let mut first_why: Option<String> = None;
    let mut warn = |i: usize, why: String, warnings: &mut Vec<CommandWarning>| {
        if first_why.is_none() {
            first_why = Some(why.clone());
        }
        warnings.push(CommandWarning {
            code: act.failed().into(),
            message: why,
            path: Some(format!("picks[{i}]")),
        });
    };
    for slot in order {
        let Some(e) = doc.get(slot) else { continue };
        let except = Some(f64::from(slot.0));
        let list = &picks[&slot];
        match act {
            Act::Trim => {
                let mut pieces = vec![shape(e)];
                let mut done = false;
                for (n, &(i, at)) in list.iter().enumerate() {
                    // The first pick is on the object as it is; a later one on what is left of it.
                    let k = if n == 0 {
                        Some(0)
                    } else {
                        piece_at(&pieces, at)
                    };
                    let Some(k) = k else { continue };
                    let target = CoreEntity::new(pieces[k].clone());
                    match store.trim_preview(&target, at, except, chosen.as_deref(), &view) {
                        Cut::Pieces(now) => {
                            pieces.splice(k..=k, now.into_iter().map(|p| p.shape));
                            done = true;
                        }
                        Cut::Error(why) => {
                            let why = if why.is_empty() {
                                NOTHING[0].into()
                            } else {
                                why
                            };
                            warn(i, why, &mut warnings);
                        }
                    }
                }
                if done {
                    changes.extend(replace_changes(doc, slot, &pieces));
                }
            }
            Act::Extend => {
                let mut current = shape(e);
                let mut used = [false; 2];
                let mut done = false;
                for &(i, at) in list {
                    let Some(path) = path_of(&current).filter(|p| !p.closed) else {
                        warn(
                            i,
                            "Yalnız açık çizgiler, çoklu çizgiler ve yaylar uzatılabilir.".into(),
                            &mut warnings,
                        );
                        continue;
                    };
                    let end = usize::from(nearest_s(&path, at) > path.length / 2.0);
                    if std::mem::replace(&mut used[end], true) {
                        continue;
                    }
                    let target = CoreEntity::new(current.clone());
                    match store.extend_preview(&target, at, except, chosen.as_deref(), &view) {
                        Geometry::Ok(g) => {
                            current = g.shape;
                            done = true;
                        }
                        Geometry::Error(why) => {
                            let why = if why.is_empty() {
                                NOTHING[1].into()
                            } else {
                                why
                            };
                            warn(i, why, &mut warnings);
                        }
                    }
                }
                if let (true, Some(geometry)) = (done, edit_geometry(current)) {
                    changes.push(EntityEdit::Update {
                        uid: doc.uid(slot).map(|u| u.to_string()).unwrap_or_default(),
                        geometry,
                    });
                }
            }
        }
    }
    if changes.is_empty() {
        let i = act as usize;
        let why = first_why.unwrap_or_else(|| NOTHING[i].into());
        return Err(refuse(act.failed(), why, "picks"));
    }
    Ok(Checked {
        edit: EntitiesEdit {
            operation: act.operation(),
            changes,
            expected_revision: None,
        },
        warnings,
    })
}
