//! `cad.entities.break` v1 and `cad.entities.join` v1 (docs/adr/0254 §3,
//! §4): an object broken between two points or split at one, and objects
//! meeting end to end joined, as one undo step “Kır” (“Birleştir”). The
//! desktop's handlers over the native document, run by the Kır and
//! Birleştir tools, the headless server, Python and MCP. The shared cases
//! are `fixtures/commands/v1/cad.entities.break.json` and `.join.json`.
//!
//! Nothing is computed twice: the pieces and the chains are the shared
//! core's (`break_entity`, `join_entities`, `chain`), written through
//! `cad.entities.edit`'s `break` and `join` operations ([`crate::edit`]).

use kentos_contracts::{
    CommandResult, CommandWarning, EditOperation, EntitiesBreak, EntitiesEdit, EntitiesEditPlan,
    EntitiesEdited, EntitiesJoin, Entity, EntityEdit, JOIN_TOLERANCE, TrimBoundaries,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::entity::Entity as CoreEntity;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::breaking::break_entity;
use kentos_geometry_core::ops::curve_cuts::Cut;
use kentos_geometry_core::ops::join::{ChainObject, chain, join_entities};
use kentos_geometry_core::store::Store;
use kentos_geometry_core::vec2::Vec2;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
use crate::codes;
use crate::edit;
use crate::geometry::{edit_geometry, shape};
use crate::geometry_store::store_of;
use crate::topology_follow::slot;
use crate::trim::{relay, replace_changes};

/// The commands' shapes over the edit they compute (`checked`).
fn validate_with(checked: Result<(EntitiesEdit, Vec<CommandWarning>), Stop>) -> CommandResult<()> {
    match checked {
        Ok((_, warnings)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

fn plan_with(
    doc: &Document,
    checked: Result<(EntitiesEdit, Vec<CommandWarning>), Stop>,
) -> CommandResult<EntitiesEditPlan> {
    match checked {
        Ok((change, warnings)) => match edit::plan_in(doc, &change) {
            CommandResult::Completed {
                output,
                warnings: more,
            } => CommandResult::Completed {
                output,
                warnings: [warnings, more].concat(),
            },
            other => relay(other),
        },
        Err(stop) => stop.into(),
    }
}

fn execute_with(
    cx: &mut ExecutionContext<'_>,
    checked: Result<(EntitiesEdit, Vec<CommandWarning>), Stop>,
) -> CommandResult<EntitiesEdited> {
    match checked {
        Ok((change, warnings)) => match edit::execute(cx, change) {
            CommandResult::Completed {
                output,
                warnings: more,
            } => CommandResult::Completed {
                output,
                warnings: [warnings, more].concat(),
            },
            other => relay(other),
        },
        Err(stop) => stop.into(),
    }
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

fn invalid_uid(uid: &str, path: &str) -> Stop {
    refuse(
        codes::INVALID_UID,
        format!(
            "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
        ),
        path,
    )
}

fn not_found(uid: &str, path: &str) -> Stop {
    refuse(
        codes::ENTITY_NOT_FOUND,
        format!(
            "“{uid}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
        ),
        path,
    )
}

fn locked(doc: &Document, e: &Entity, path: &str) -> Option<Stop> {
    let layers = doc.layers();
    let layer = &e.base().layer_id;
    layers.is_locked(layer).then(|| {
        let name = layers.get(layer).map_or(layer.as_str(), |n| n.name.as_str());
        refuse(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli; üzerindeki nesne değiştirilemez. Kilidi Katmanlar panelinden açın."
            ),
            path,
        )
    })
}

fn find(doc: &Document, uid: &str) -> Option<Slot> {
    Uuid::parse_str(uid).ok().and_then(|u| doc.slot_of(u))
}

fn point(p: kentos_contracts::Vec2) -> Vec2 {
    Vec2::new(p.x, p.y)
}

// ── Kır ────────────────────────────────────────────────────────────────

/// The undo step of `cad.entities.break`.
pub const BREAK_LABEL: &str = "Kır";

/// Whether Kır breaks an object of this kind.
pub fn breakable(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Line(_)
            | Entity::Polyline(_)
            | Entity::Polygon(_)
            | Entity::Arc(_)
            | Entity::Circle(_)
            | Entity::Ellipse(_)
            | Entity::Xline(_)
            | Entity::Ray(_)
    )
}

/// The checks of `cad.entities.break`, in order; the edit it writes.
fn check_break(
    doc: &Document,
    input: &EntitiesBreak,
) -> Result<(EntitiesEdit, Vec<CommandWarning>), Stop> {
    if !is_uid_text(&input.uid) {
        return Err(invalid_uid(&input.uid, "uid"));
    }
    checks::point(input.first, "İlk kırma noktasının", "first")?;
    if let Some(second) = input.second {
        checks::point(second, "İkinci kırma noktasının", "second")?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let slot = find(doc, &input.uid).ok_or_else(|| not_found(&input.uid, "uid"))?;
    let Some(e) = doc.get(slot) else {
        return Err(not_found(&input.uid, "uid"));
    };
    if !breakable(e) {
        return Err(refuse(
            codes::NOT_BREAKABLE,
            "Bu türdeki nesne kırılmaz: çizgi, çoklu çizgi, alan, yay, daire, elips, yardımcı çizgi ya da ışın olmalı.".into(),
            "uid",
        ));
    }
    if matches!(e, Entity::Polygon(p) if p.holes.as_ref().is_some_and(|h| !h.is_empty())) {
        return Err(refuse(
            codes::NOT_BREAKABLE,
            "Adalı alanda kırma yapılamaz; iç halkalar kaybolurdu. Önce Patlat ile halkalarına ayırın ya da Alan böl kullanın.".into(),
            "uid",
        ));
    }
    if let Some(stop) = locked(doc, e, "uid") {
        return Err(stop);
    }
    let first = point(input.first);
    let second = input.second.map_or(first, point);
    match break_entity(&CoreEntity::new(shape(e)), first, second) {
        Cut::Error(why) => Err(refuse(
            codes::BREAK_FAILED,
            if why.is_empty() {
                "Nesne bu noktalarda kırılamadı; noktaları nesnenin üstünde verin.".into()
            } else {
                why
            },
            "first",
        )),
        Cut::Pieces(pieces) => {
            let shapes: Vec<_> = pieces.into_iter().map(|p| p.shape).collect();
            Ok((
                EntitiesEdit {
                    operation: EditOperation::Break,
                    changes: replace_changes(doc, slot, &shapes),
                    expected_revision: None,
                },
                Vec::new(),
            ))
        }
    }
}

/// `cad.entities.break` v1: Kır.
pub mod entities_break {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesBreak) -> CommandResult<()> {
        validate_with(check_break(cx.doc, input))
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &EntitiesBreak,
    ) -> CommandResult<EntitiesEditPlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] over the document (the tool's preview).
    pub fn plan_in(doc: &Document, input: &EntitiesBreak) -> CommandResult<EntitiesEditPlan> {
        plan_with(doc, check_break(doc, input))
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: EntitiesBreak,
    ) -> CommandResult<EntitiesEdited> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_ENTITIES_BREAK,
            kentos_contracts::CAD_ENTITIES_BREAK_VERSION,
            input,
            |cx, input| {
                let checked = check_break(cx.doc, &input);
                execute_with(cx, checked)
            },
        )
    }
}

// ── Birleştir ──────────────────────────────────────────────────────────

/// The undo step of `cad.entities.join`.
pub const JOIN_LABEL: &str = "Birleştir";

/// Whether an object can be a chain's (a line, an arc, an open polyline).
fn chainable(e: &Entity) -> bool {
    matches!(e, Entity::Line(_) | Entity::Arc(_) | Entity::Polyline(_))
}

/// The objects a chain walks over, in the drawing's order: the shown lines,
/// arcs and open polylines of `set` (all, or those meeting the rectangle),
/// or the listed ones, shown or not.
fn chain_set(doc: &Document, store: &Store, set: &TrimBoundaries) -> Vec<Slot> {
    let mut slots: Vec<Slot> = match set {
        TrimBoundaries::Objects { uids } => uids.iter().filter_map(|u| find(doc, u)).collect(),
        TrimBoundaries::All | TrimBoundaries::Area { .. } => {
            let view = match set {
                TrimBoundaries::Area { min, max } => Bounds {
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
            store
                .overlapping(&view, None)
                .into_iter()
                .filter_map(|it| slot(it.id))
                .collect()
        }
    };
    slots.sort_unstable();
    slots.dedup();
    slots.retain(|s| doc.get(*s).is_some_and(chainable));
    slots
}

/// The checks of `cad.entities.join`, in order; the edit it writes.
fn check_join(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesJoin,
) -> Result<(EntitiesEdit, Vec<CommandWarning>), Stop> {
    if input.uids.is_empty() {
        return Err(refuse(
            codes::NO_ENTITIES,
            "Birleştirilecek nesne yok. En az bir nesnenin kalıcı kimliğini verin.".into(),
            "uids",
        ));
    }
    if input.chain.is_some() && input.uids.len() > 1 {
        return Err(refuse(
            codes::NO_ENTITIES,
            "Zincir tek bir nesneden (tohum) başlar; bir kimlik verin.".into(),
            "uids",
        ));
    }
    for (i, uid) in input.uids.iter().enumerate() {
        if !is_uid_text(uid) {
            return Err(invalid_uid(uid, &format!("uids[{i}]")));
        }
    }
    if let Some(TrimBoundaries::Objects { uids }) = &input.chain {
        if uids.is_empty() {
            return Err(refuse(
                codes::NO_ENTITIES,
                "Zincirin nesneleri yok. Nesneleri verin ya da bütün görünen nesneler için “all”u kullanın.".into(),
                "chain.uids",
            ));
        }
        for (i, uid) in uids.iter().enumerate() {
            if !is_uid_text(uid) {
                return Err(invalid_uid(uid, &format!("chain.uids[{i}]")));
            }
        }
    }
    let tolerance = input.tolerance.unwrap_or(JOIN_TOLERANCE);
    if !(tolerance.is_finite() && tolerance > 0.0) {
        return Err(refuse(
            codes::INVALID_DISTANCE,
            "Tolerans sıfırdan büyük sonlu bir uzunluk (metre) olmalı.".into(),
            "tolerance",
        ));
    }
    if let Some(TrimBoundaries::Area { min, max }) = &input.chain {
        checks::point(*min, "Dikdörtgenin köşesinin", "chain.min")?;
        checks::point(*max, "Dikdörtgenin köşesinin", "chain.max")?;
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let named = checks::named(doc, &input.uids)?;
    if let Some(TrimBoundaries::Objects { uids }) = &input.chain {
        for (i, uid) in uids.iter().enumerate() {
            if find(doc, uid).is_none() {
                return Err(not_found(uid, &format!("chain.uids[{i}]")));
            }
        }
    }
    let mut warnings = Vec::new();
    let (targets, keep) = match &input.chain {
        Some(set) => {
            let (_, seed, e, _) = named[0];
            if !chainable(e) {
                return Err(refuse(
                    codes::NOT_JOINABLE,
                    "Zincir bir çizgiden, yaydan ya da açık çoklu çizgiden başlar.".into(),
                    "uids[0]",
                ));
            }
            if let Some(stop) = locked(doc, e, "uids[0]") {
                return Err(stop);
            }
            let own;
            let store = match store {
                Some(s) => s,
                None => {
                    own = store_of(doc);
                    &own
                }
            };
            let mut slots = chain_set(doc, store, set);
            if !slots.contains(&seed) {
                slots.push(seed);
            }
            let objects: Vec<ChainObject> = slots
                .iter()
                .filter_map(|s| doc.get(*s))
                .map(|e| ChainObject {
                    shape: shape(e),
                    locked: doc.layers().is_locked(&e.base().layer_id),
                })
                .collect();
            let at = slots.iter().position(|s| *s == seed).unwrap_or(0);
            let found = chain(&objects, at, tolerance.max(1e-9));
            if found.members.len() < 2 {
                return Err(refuse(
                    codes::NOTHING_TO_JOIN,
                    "Bu nesneye ucu ucuna bağlanan nesne yok; zincir kurulamadı.".into(),
                    "uids[0]",
                ));
            }
            if found.locked {
                warnings.push(CommandWarning {
                    code: codes::CHAIN_STOPPED.into(),
                    message: "Zincir kilitli katmandaki bir nesnede durdu.".into(),
                    path: None,
                });
            }
            let members: Vec<Slot> = found
                .members
                .iter()
                .filter_map(|&i| slots.get(i).copied())
                .collect();
            (members, Some(seed))
        }
        None => {
            let mut targets = Vec::with_capacity(named.len());
            for (i, slot, e, _) in named {
                if doc.layers().is_locked(&e.base().layer_id) {
                    warnings.push(CommandWarning {
                        code: codes::LOCKED_SKIPPED.into(),
                        message: "Nesne kilitli katmanda olduğu için birleştirilmedi.".into(),
                        path: Some(format!("uids[{i}]")),
                    });
                } else {
                    targets.push(slot);
                }
            }
            (targets, None)
        }
    };
    // The core names each object by its `id`: the slot.
    let list: Vec<CoreEntity> = targets
        .iter()
        .filter_map(|s| {
            let e = doc.get(*s)?;
            Some(CoreEntity {
                shape: shape(e),
                rest: vec![("id".to_owned(), Json::Num(f64::from(s.0)))],
            })
        })
        .collect();
    let groups = match join_entities(&list, tolerance.max(1e-9)) {
        Ok(joined) => joined.groups,
        Err(why) => return Err(refuse(codes::JOIN_FAILED, why, "uids")),
    };
    if groups.is_empty() {
        return Err(refuse(
            codes::NOTHING_TO_JOIN,
            "Uçları birleşen çizgi, yay ya da açık çoklu çizgi bulunamadı. Toleransı artırmayı deneyin.".into(),
            "uids",
        ));
    }
    let slot_of = |id: &Json| match id {
        Json::Num(n) if *n >= 0.0 && n.fract() == 0.0 => Some(Slot(*n as u32)),
        _ => None,
    };
    // Each chain is its first object (or the seed), joined: its slot, persistent id,
    // layer, colour, attributes and label; the others go.
    let mut changes = Vec::new();
    for g in &groups {
        let slots: Vec<Slot> = g.sources.iter().filter_map(slot_of).collect();
        let first = keep
            .filter(|k| slots.contains(k))
            .or_else(|| slots.first().copied());
        let (Some(first), Some(geometry)) = (first, edit_geometry(g.geometry.shape.clone())) else {
            continue;
        };
        let uid = |s: Slot| doc.uid(s).map(|u| u.to_string()).unwrap_or_default();
        changes.push(EntityEdit::Replace {
            uid: uid(first),
            geometry,
            keep_data: Some(true),
        });
        for s in slots.iter().filter(|&&s| s != first) {
            changes.push(EntityEdit::Remove { uid: uid(*s) });
        }
    }
    Ok((
        EntitiesEdit {
            operation: EditOperation::Join,
            changes,
            expected_revision: None,
        },
        warnings,
    ))
}

/// `cad.entities.join` v1: Birleştir.
pub mod entities_join {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesJoin) -> CommandResult<()> {
        validate_with(check_join(cx.doc, cx.store, input))
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &EntitiesJoin,
    ) -> CommandResult<EntitiesEditPlan> {
        plan_in(cx.doc, cx.store, input)
    }

    /// [`plan`] over the document and, when the host has one, its store (the tool's preview).
    pub fn plan_in(
        doc: &Document,
        store: Option<&Store>,
        input: &EntitiesJoin,
    ) -> CommandResult<EntitiesEditPlan> {
        plan_with(doc, check_join(doc, store, input))
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: EntitiesJoin,
    ) -> CommandResult<EntitiesEdited> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_ENTITIES_JOIN,
            kentos_contracts::CAD_ENTITIES_JOIN_VERSION,
            input,
            |cx, input| {
                let checked = check_join(cx.doc, cx.store, &input);
                execute_with(cx, checked)
            },
        )
    }
}
