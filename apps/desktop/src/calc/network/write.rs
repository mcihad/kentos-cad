//! Çizime yaz (docs/adr/0203 §8; the web's `writeNetwork`, `writeLevels`):
//! the new points only, never the known ones. Yatay ağ moves the drawing's
//! points of the same name to their adjusted places, the vertices of lines,
//! polylines and areas on them with them (the point editor's rule,
//! docs/adr/0153 §3), and adds the others on the chosen layer; Kot ağı gives
//! the drawing's points of the same name their adjusted heights, their
//! `Z (m)` and the vertices on them too, and says the points it has not. One
//! undo step named after the window; a locked layer refuses it all.

use std::collections::BTreeMap;

use kentos_contracts::{
    CreateOperation, EditOperation, EntitiesEdit, EntitiesSetProperties, Entity, EntityEdit,
    EntityGeometry, PropertiesOperation, Vec2 as Wire,
};
use kentos_domain::{Document as Model, Slot};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::ops::point_editor::follow_point;
use kentos_interaction::calc::{SurveyPoint, add_points};
use kentos_interaction::{Level, fixed, upper_tr};
use kentos_native_application::elevation::paths as elevated_paths;
use kentos_native_application::{ExecutionContext, edit, set};

use super::{LEVEL_TITLE, NETWORK_TITLE, POINT_KIND, named_points};
use crate::app::App;
use crate::points::edit::{in_step, refusal, with_paths};

/// A point to move, raise or lower: its slot and what it becomes.
struct Change {
    slot: Slot,
    to: Vec2,
    z: Option<f64>,
}

/// The edits of `changes` (each point and the vertices on it), in one list.
fn edits(doc: &Model, changes: &[Change], set_z: bool) -> Vec<EntityEdit> {
    let mut out = Vec::new();
    for c in changes {
        let (Some(Entity::Point(p)), Some(uid)) = (doc.get(c.slot), doc.uid(c.slot)) else {
            continue;
        };
        let from = Vec2::new(p.p.x, p.p.y);
        out.push(EntityEdit::Update {
            uid: uid.to_string(),
            geometry: EntityGeometry::Point {
                p: Wire {
                    x: c.to.x,
                    y: c.to.y,
                },
                z: c.z,
                parts: p.parts.clone(),
            },
        });
        for other in doc.entities() {
            if !matches!(
                other,
                Entity::Line(_) | Entity::Polyline(_) | Entity::Polygon(_)
            ) {
                continue;
            }
            let Some(moved) = follow_point(
                &elevated_paths(other),
                from,
                c.to,
                set_z,
                if set_z { c.z } else { None },
            ) else {
                continue;
            };
            if let (Some(geometry), Some(uid)) =
                (with_paths(other, &moved), doc.uid(Slot(other.base().id)))
            {
                out.push(EntityEdit::Update {
                    uid: uid.to_string(),
                    geometry,
                });
            }
        }
    }
    out
}

/// Whether `name` is one of `names`, compared the Turkish way.
fn among(names: &[String], name: &str) -> bool {
    let key = upper_tr(name.trim());
    names.iter().any(|n| upper_tr(n.trim()) == key)
}

impl App {
    /// Yatay ağ dengelemesi's Çizime yaz: the new points moved or added.
    pub(crate) fn network_write(&mut self) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let form = &self.calc.network;
        let Some(Ok(result)) = &form.solved.result else {
            return;
        };
        let known = form.known_names();
        let mut moves = Vec::new();
        let mut adds = Vec::new();
        for p in result.points.iter().filter(|p| !among(&known, &p.name)) {
            let there = named_points(&doc.model, &p.name);
            if there.is_empty() {
                adds.push(SurveyPoint {
                    name: p.name.clone(),
                    p: Vec2::new(p.y, p.x),
                    z: None,
                });
            }
            for d in there {
                moves.push(Change {
                    slot: Slot(d.base.id),
                    to: Vec2::new(p.y, p.x),
                    z: d.z,
                });
            }
        }
        if moves.is_empty() && adds.is_empty() {
            return;
        }
        let layer = form.layer.clone();
        let (moved, added) = (moves.len(), adds.len());
        let mut written: Vec<Slot> = Vec::new();
        let refused = in_step(&mut doc.model, NETWORK_TITLE, |model| {
            if !moves.is_empty() {
                let input = EntitiesEdit {
                    operation: EditOperation::NetworkAdjust,
                    changes: edits(model, &moves, false),
                    expected_revision: None,
                };
                if let Some(e) = refusal(edit::execute(&mut ExecutionContext::new(model), input)) {
                    return Some(e.message);
                }
                written.extend(moves.iter().map(|c| c.slot));
            }
            if !adds.is_empty() {
                let Some(layer) = layer.as_deref() else {
                    return Some("Yeni noktalar için katman seçin.".to_owned());
                };
                match add_points(
                    model,
                    layer,
                    &adds,
                    POINT_KIND,
                    CreateOperation::NetworkAdjust,
                ) {
                    Ok((slots, _)) => written.extend(slots),
                    Err(e) => return Some(e),
                }
            }
            None
        });
        if let Some(refused) = refused {
            self.warn(refused);
            return;
        }
        self.selection.set(written);
        self.say(
            Level::Success,
            format!(
                "{NETWORK_TITLE}: {moved} nokta dengelenmiş yerine taşındı, {added} nokta eklendi (Ctrl+Z geri alır)."
            ),
        );
        self.calc.open = None;
        self.dialog = None;
    }

    /// Kot ağı dengelemesi's Çizime yaz: the new points' heights.
    pub(crate) fn level_write(&mut self) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let form = &self.calc.level;
        let Some(Ok(result)) = &form.solved.result else {
            return;
        };
        let known = form.known_names();
        let mut changes = Vec::new();
        let mut absent = Vec::new();
        for p in result.points.iter().filter(|p| !among(&known, &p.name)) {
            let there = named_points(&doc.model, &p.name);
            if there.is_empty() {
                absent.push(p.name.clone());
            }
            for d in there {
                changes.push((
                    Change {
                        slot: Slot(d.base.id),
                        to: Vec2::new(d.p.x, d.p.y),
                        z: Some(p.h),
                    },
                    d.base.attrs.contains_key("Z (m)"),
                ));
            }
        }
        if changes.is_empty() {
            if !absent.is_empty() {
                self.warn(format!(
                    "{LEVEL_TITLE}: {} çizimde yok; yazılmadı.",
                    absent.join(", ")
                ));
            }
            return;
        }
        let n = changes.len();
        let refused = in_step(&mut doc.model, LEVEL_TITLE, |model| {
            let moves: Vec<Change> = changes
                .iter()
                .map(|(c, _)| Change {
                    slot: c.slot,
                    to: c.to,
                    z: c.z,
                })
                .collect();
            let input = EntitiesEdit {
                operation: EditOperation::LevelAdjust,
                changes: edits(model, &moves, true),
                expected_revision: None,
            };
            if let Some(e) = refusal(edit::execute(&mut ExecutionContext::new(model), input)) {
                return Some(e.message);
            }
            // The Z (m) the Hesap windows write beside it follows (three decimals).
            for (c, has) in &changes {
                let (true, Some(uid), Some(z)) = (*has, model.uid(c.slot), c.z) else {
                    continue;
                };
                let input = EntitiesSetProperties {
                    uids: vec![uid.to_string()],
                    layer_id: None,
                    color: None,
                    line_weight: None,
                    symbol: None,
                    attrs: Some(BTreeMap::from([("Z (m)".to_owned(), Some(fixed(z, 3)))])),
                    label: None,
                    operation: PropertiesOperation::Attributes,
                    expected_revision: None,
                    unlink: false,
                };
                if let Some(e) = refusal(set::execute(&mut ExecutionContext::new(model), input)) {
                    return Some(e.message);
                }
            }
            None
        });
        if let Some(refused) = refused {
            self.warn(refused);
            return;
        }
        self.selection
            .set(changes.iter().map(|(c, _)| c.slot).collect::<Vec<_>>());
        self.say(
            Level::Success,
            format!("{LEVEL_TITLE}: {n} noktanın kotu yazıldı (Ctrl+Z geri alır)."),
        );
        if !absent.is_empty() {
            self.warn(format!(
                "{LEVEL_TITLE}: {} çizimde yok; yazılmadı.",
                absent.join(", ")
            ));
        }
        self.calc.open = None;
        self.dialog = None;
    }
}
