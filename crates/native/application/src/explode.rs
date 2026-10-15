//! `cad.entities.explode` v1 (docs/adr/0302; AutoCAD's EXPLODE, Patlat): objects named by their persistent ids
//! broken one level, as one undo step “Patlat”. The pieces are the shared core's (`ops::explode::explode_entity`, a
//! block's `Blocks::explode`), written through `cad.entities.edit`'s `explode` operation ([`crate::edit`]); the
//! Patlat tool, the headless server, Python and MCP run it. The shared cases are
//! `fixtures/commands/v1/cad.entities.explode.json`.
//!
//! An item of an associative array (docs/adr/0300) brings its whole array: every item's insert is exploded and the
//! array is removed in the same step, so its objects are independent (AutoCAD explodes an associative array as one
//! object). A dimension without its own text writes its value as the project shows it (`Format::dimension`).

use std::collections::HashSet;

use kentos_contracts::{
    ArrayId, CommandResult, CommandWarning, EditOperation, EntitiesEdit, EntitiesEditPlan,
    EntitiesEdited, EntitiesExplode, Entity, EntityEdit, LayerNodeType,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::block::Blocks;
use kentos_geometry_core::entity::{Entity as CoreEntity, dimension_geom};
use kentos_geometry_core::geom::dimension::layout_dimension;
use kentos_geometry_core::ops::curve_cuts::Cut;
use kentos_geometry_core::ops::explode::explode_entity;
use kentos_geometry_core::store::Store;
use kentos_project::format::Format;

use crate::ExecutionContext;
use crate::blocks::{core_blocks, core_entity};
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::edit;
use crate::geometry::{drawing_font, edit_geometry, shape};
use crate::trim::relay;

/// What the check gives: the edit, its warnings, and the arrays the exploded items empty.
struct Checked {
    change: EntitiesEdit,
    warnings: Vec<CommandWarning>,
    arrays: Vec<ArrayId>,
}

/// A dimension's measured value as the project shows it: what its exploded text says when it has none of its own.
fn value_text(e: &Entity, f: &Format) -> String {
    let Entity::Dimension(d) = e else {
        return String::new();
    };
    if d.text.as_deref().is_some_and(|t| !t.is_empty()) {
        return String::new();
    }
    let s = shape(e);
    let Some(l) = dimension_geom(&s).and_then(|g| layout_dimension(&g)) else {
        return String::new();
    };
    f.dimension(l.prefix, l.unit, l.value)
}

/// A block's object exploded from the insert `from` (docs/adr/0144): an `add` with the object's own layer when the
/// drawing has it as a layer (else the insert's), its colour and line weight (the insert's when it has none: the
/// core gives them), its attributes and label.
fn block_piece(
    doc: &Document,
    from: &str,
    geometry: kentos_contracts::EntityGeometry,
    piece: &CoreEntity,
) -> EntityEdit {
    let field = |name: &str| piece.rest.iter().find(|(k, _)| k == name).map(|(_, v)| v);
    let text = |name: &str| match field(name) {
        Some(Json::Str(s)) => Some(s.clone()),
        _ => None,
    };
    let layer_id = text("layerId").filter(|id| {
        doc.layers()
            .get(id)
            .is_some_and(|n| n.kind == LayerNodeType::Layer)
    });
    let line_weight = match field("lineWeight") {
        Some(Json::Num(w)) => Some(*w),
        _ => None,
    };
    let attrs = match field("attrs") {
        Some(Json::Obj(fields)) => Some(
            fields
                .iter()
                .filter_map(|(k, v)| match v {
                    Json::Str(s) => Some((k.clone(), s.clone())),
                    _ => None,
                })
                .collect(),
        ),
        _ => None,
    };
    EntityEdit::Add {
        from: from.to_owned(),
        geometry,
        keep_data: None,
        layer_id,
        color: text("color"),
        line_weight,
        attrs,
        label: text("label"),
    }
}

/// The ids to explode: the input's, then the other items of every associative array one of them is an item of,
/// in the arrays' order; and those arrays.
fn with_arrays(doc: &Document, uids: &[String]) -> (Vec<String>, Vec<ArrayId>) {
    let mut all: Vec<String> = uids.to_vec();
    let mut seen: HashSet<String> = uids.iter().cloned().collect();
    let mut arrays = Vec::new();
    for uid in uids {
        let Some((array, _)) = Uuid::parse_str(uid).ok().and_then(|u| doc.array_of_item(u)) else {
            continue;
        };
        if arrays.contains(&array.id) {
            continue;
        }
        arrays.push(array.id);
        for item in &array.items {
            if let Some(u) = item.uid {
                let text = Uuid::from_bytes(u.0).to_string();
                if seen.insert(text.clone()) {
                    all.push(text);
                }
            }
        }
    }
    (all, arrays)
}

/// The checks in the contract's order, and the edit they make.
fn check(doc: &Document, store: Option<&Store>, input: &EntitiesExplode) -> Result<Checked, Stop> {
    checks::uids(&input.uids, "Patlatılacak nesne verilmedi.")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let (all, arrays) = with_arrays(doc, &input.uids);
    let found = checks::named(doc, &all)?;
    let (open, locked) = checks::skip_locked(doc, found)?;
    let f = Format::of(doc.settings());
    let font = drawing_font(doc.settings().drawing_font);
    let own_blocks: Option<Blocks> = store.is_none().then(|| core_blocks(doc.blocks()));
    let blocks = store.map_or_else(|| own_blocks.as_ref(), |s| Some(s.blocks()));
    let mut changes = Vec::new();
    let mut not: Option<(usize, String)> = None;
    let mut skipped = 0;
    for (at, _, e, uid) in open {
        let insert = matches!(e, Entity::Insert(_));
        let cut = match (e, blocks) {
            (Entity::Insert(_), Some(blocks)) => blocks.explode(&core_entity(e)),
            _ => explode_entity(&shape(e), &value_text(e, &f), font),
        };
        match cut {
            Cut::Error(message) => {
                skipped += 1;
                not.get_or_insert((at, message));
            }
            Cut::Pieces(pieces) => {
                changes.push(EntityEdit::Remove { uid: uid.clone() });
                for piece in pieces {
                    let Some(geometry) = edit_geometry(piece.shape.clone()) else {
                        continue;
                    };
                    changes.push(if insert {
                        block_piece(doc, uid, geometry, &piece)
                    } else {
                        EntityEdit::add(uid.clone(), geometry, None)
                    });
                }
            }
        }
    }
    if changes.is_empty() {
        let (at, message) = not.unwrap_or((0, "Patlatılacak bir nesne yok.".into()));
        return Err(Stop::failed(error(
            codes::NOT_EXPLODABLE,
            message,
            Some(format!("uids[{at}]")),
        )));
    }
    let mut warnings: Vec<CommandWarning> = locked.into_iter().collect();
    if let Some((_, message)) = not {
        warnings.push(CommandWarning {
            code: codes::NOT_EXPLODABLE.into(),
            message: format!("{skipped} nesne patlatılamadığı için atlandı: {message}"),
            path: Some("uids".into()),
        });
    }
    Ok(Checked {
        change: EntitiesEdit {
            operation: EditOperation::Explode,
            changes,
            expected_revision: None,
        },
        warnings,
        arrays,
    })
}

/// `cad.entities.explode` v1: Patlat.
pub mod entities_explode {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesExplode) -> CommandResult<()> {
        match check(cx.doc, cx.store, input) {
            Ok(checked) => CommandResult::Completed {
                output: (),
                warnings: checked.warnings,
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &EntitiesExplode,
    ) -> CommandResult<EntitiesEditPlan> {
        match check(cx.doc, cx.store, input) {
            Ok(checked) => match edit::plan_in(cx.doc, &checked.change) {
                CommandResult::Completed { output, warnings } => CommandResult::Completed {
                    output,
                    warnings: [checked.warnings, warnings].concat(),
                },
                other => relay(other),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: EntitiesExplode,
    ) -> CommandResult<EntitiesEdited> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_ENTITIES_EXPLODE,
            kentos_contracts::CAD_ENTITIES_EXPLODE_VERSION,
            input,
            |cx, input| {
                let checked = match check(cx.doc, cx.store, &input) {
                    Ok(checked) => checked,
                    Err(stop) => return stop.into(),
                };
                match edit::execute(cx, checked.change) {
                    CommandResult::Completed { output, warnings } => {
                        // In the same step: the arrays whose items came apart are gone.
                        for id in checked.arrays {
                            cx.doc.remove_array(id);
                        }
                        CommandResult::Completed {
                            output,
                            warnings: [checked.warnings, warnings].concat(),
                        }
                    }
                    other => relay(other),
                }
            },
        )
    }
}
