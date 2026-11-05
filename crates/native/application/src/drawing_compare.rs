//! `cad.drawing.compare` (docs/adr/0576 §3): what changed between a drawing as it is and the same drawing as it was,
//! its history some undo steps back or a drawing file. Read only.
//!
//! The objects are paired by their persistent ids (ADR 0014) and told apart by the data compare core (ADR 0179,
//! `ops::compare`, matching by a key: each object's id), so a moved object is a changed one, not an added and a
//! removed. The geometry is the same within the tolerance; the attributes and the layer are equal or not.
//! [`diff`] is the rule the desktop's second view highlights with.

use std::collections::{HashMap, HashSet};
use std::path::Path;

use kentos_commands::encoding::sha256_hex;
use kentos_commands::{
    BlockId, Entity, COMPARE_LIMIT, COMPARE_TOLERANCE, ChangedObject, CommandResult, CompareAgainst,
    DrawingCompare, DrawingCompared, MAX_COMPARE_LIMIT, MAX_COMPARE_TOLERANCE,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::compare::{Attrs, Matching, Member, Settings, Status, compare};

use crate::ExecutionContext;
use crate::checks::{Stop, error};
use crate::codes;
use crate::file_import_block::read_drawing;

/// The attribute names the pairing and the layer are carried by: no user attribute has a NUL in its name.
const UID: &str = "\0uid";
const LAYER: &str = "\0layer";
const DEFINITION: &str = "\0blockDefinition";
const LAYER_STYLE: &str = "\0layerStyle";
const DIMENSION_PRESENTATION: &str = "\0dimensionPresentation";
const DIMENSION_ASSOCIATION: &str = "\0dimensionAssociation";

/// One object in both states that differs.
#[derive(Clone, Debug, PartialEq)]
pub struct Changed {
    pub old: Slot,
    pub new: Slot,
    /// `geometry`, `attributes` or `both`.
    pub status: &'static str,
    pub fields: Vec<String>,
    pub distance: Option<f64>,
}

/// What differs between an old state and a new one: by slots of each document, in each one's order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Diff {
    pub same: usize,
    pub changed: Vec<Changed>,
    /// In the new state only.
    pub added: Vec<Slot>,
    /// In the old state only.
    pub removed: Vec<Slot>,
}

/// A placed block's visible definition, including definitions it places. Cached so many copies of one door do not
/// serialize the same definition repeatedly. A cycle is marked rather than recursed forever on damaged input.
fn definition_fingerprint(
    doc: &Document,
    id: BlockId,
    cache: &mut HashMap<BlockId, String>,
    visiting: &mut HashSet<BlockId>,
) -> String {
    if let Some(done) = cache.get(&id) {
        return done.clone();
    }
    if !visiting.insert(id) {
        return format!("cycle:{}", id.to_text());
    }
    let result = if let Some(def) = doc.block(id) {
        let mut bytes = serde_json::to_vec(def).unwrap_or_default();
        for entity in &def.entities {
            if let Entity::Insert(insert) = entity {
                let child = definition_fingerprint(doc, insert.block, cache, visiting);
                bytes.extend_from_slice(&(child.len() as u64).to_le_bytes());
                bytes.extend_from_slice(child.as_bytes());
            }
        }
        sha256_hex(&bytes)
    } else {
        format!("missing:{}", id.to_text())
    };
    visiting.remove(&id);
    cache.insert(id, result.clone());
    result
}

fn member(
    doc: &Document,
    uid: kentos_domain::Uuid,
    e: &Entity,
    cache: &mut HashMap<BlockId, String>,
    layer_styles: &mut HashMap<String, String>,
) -> Member {
    let base = e.base();
    let mut attrs: Vec<(String, String)> = base
        .attrs
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    attrs.push((UID.to_owned(), uid.to_string()));
    attrs.push((LAYER.to_owned(), base.layer_id.clone()));
    if let Some(style) = layer_styles.get(&base.layer_id) {
        attrs.push((LAYER_STYLE.to_owned(), style.clone()));
    } else if let Some(layer) = doc.layers().get(&base.layer_id) {
        if let Ok(style) = serde_json::to_vec(&layer.style) {
            let fingerprint = sha256_hex(&style);
            layer_styles.insert(base.layer_id.clone(), fingerprint.clone());
            attrs.push((LAYER_STYLE.to_owned(), fingerprint));
        }
    }
    if let Entity::Insert(insert) = e {
        attrs.push((
            DEFINITION.to_owned(),
            definition_fingerprint(doc, insert.block, cache, &mut HashSet::new()),
        ));
    }
    if let Entity::Dimension(dimension) = e {
        let presentation = (
            &dimension.text,
            &dimension.style,
            &dimension.mask,
            &dimension.look,
            &dimension.text_shift,
            &dimension.breaks,
            &dimension.annotative,
            &dimension.level,
        );
        if let Ok(bytes) = serde_json::to_vec(&presentation) {
            attrs.push((DIMENSION_PRESENTATION.to_owned(), sha256_hex(&bytes)));
        }
        if let Ok(bytes) = serde_json::to_vec(&dimension.refs) {
            attrs.push((DIMENSION_ASSOCIATION.to_owned(), sha256_hex(&bytes)));
        }
    }
    Member {
        shape: crate::geometry::shape(e),
        attrs: Attrs(attrs),
    }
}

fn members(doc: &Document) -> (Vec<Member>, Vec<Slot>) {
    let mut out = Vec::new();
    let mut slots = Vec::new();
    let mut definitions = HashMap::new();
    let mut layer_styles = HashMap::new();
    for (uid, e) in doc.entities_with_uids() {
        out.push(member(doc, uid, e, &mut definitions, &mut layer_styles));
        slots.push(Slot(e.base().id));
    }
    (out, slots)
}

fn settings(tolerance: f64) -> Settings {
    Settings {
        matching: Matching::Key,
        key: Some(UID.to_owned()),
        search: 0.0,
        tolerance,
        ignore: Some(vec![UID.to_owned()]),
    }
}

/// Whether one object differs between two states, by [`diff`]'s rule: for a follower that knows which objects an
/// edit touched and compares only those (the desktop's revision view, docs/adr/0637 §4). `None` when a slot is
/// empty.
pub fn differs(
    old: &Document,
    old_slot: Slot,
    new: &Document,
    new_slot: Slot,
    tolerance: f64,
) -> Option<bool> {
    let a = member(
        old,
        old.uid(old_slot)?,
        old.get(old_slot)?,
        &mut HashMap::new(),
        &mut HashMap::new(),
    );
    let b = member(
        new,
        new.uid(new_slot)?,
        new.get(new_slot)?,
        &mut HashMap::new(),
        &mut HashMap::new(),
    );
    let rows = compare(&[a], &[b], &settings(tolerance));
    Some(!matches!(
        rows.first().map(|r| r.status),
        Some(Status::Same)
    ))
}

/// What differs between `old` and `new` within `tolerance` metres.
pub fn diff(old: &Document, new: &Document, tolerance: f64) -> Diff {
    let (a, a_slots) = members(old);
    let (b, b_slots) = members(new);
    let mut out = Diff::default();
    for row in compare(&a, &b, &settings(tolerance)) {
        match (row.status, row.old, row.new) {
            (Status::Same, ..) => out.same += 1,
            (Status::Added, _, Some(n)) => out.added.push(b_slots[n]),
            (Status::Removed, Some(o), _) => out.removed.push(a_slots[o]),
            (status, Some(o), Some(n)) => {
                let fields: Vec<String> = row
                    .fields
                    .iter()
                    .map(|f| {
                        if f == LAYER {
                            "layer".to_owned()
                        } else if f == DEFINITION {
                            "blockDefinition".to_owned()
                        } else if f == LAYER_STYLE {
                            "layerStyle".to_owned()
                        } else if f == DIMENSION_PRESENTATION {
                            "dimensionPresentation".to_owned()
                        } else if f == DIMENSION_ASSOCIATION {
                            "dimensionAssociation".to_owned()
                        } else {
                            f.clone()
                        }
                    })
                    .collect();
                let appearance_only = match (old.get(a_slots[o]), new.get(b_slots[n])) {
                    (Some(Entity::Dimension(a)), Some(Entity::Dimension(b))) => {
                        a.a == b.a && a.b == b.b && a.c == b.c && a.offset == b.offset
                            && a.height == b.height && a.angle == b.angle
                            && a.za == b.za && a.zb == b.zb
                    }
                    _ => false,
                };
                out.changed.push(Changed {
                    old: a_slots[o],
                    new: b_slots[n],
                    status: if appearance_only {
                        "attributes"
                    } else {
                        match status {
                            Status::Geometry => "geometry",
                            Status::Both => "both",
                            _ => "attributes",
                        }
                    },
                    fields,
                    distance: if appearance_only { None } else { row.distance },
                });
            }
            // A key problem or a half row: not paired, so told as what it is.
            (_, Some(o), None) => out.removed.push(a_slots[o]),
            (_, None, Some(n)) => out.added.push(b_slots[n]),
            _ => {}
        }
    }
    // By the document's order, which `compare` does not promise.
    out.added.sort_by_key(|s| s.0);
    out.removed.sort_by_key(|s| s.0);
    out.changed.sort_by_key(|c| c.new.0);
    out
}

fn file_name(path: &str) -> String {
    Path::new(path)
        .file_name()
        .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned())
}

/// The other state: the document and what to call it; why not.
fn other(doc: &Document, against: &CompareAgainst) -> Result<(Document, String), Stop> {
    other_state(doc, against, "against")
}

/// A state of the drawing a question names (docs/adr/0576, 0646 §1): the document and what to call it; why not, at
/// `prefix.steps` or `prefix.path`.
pub(crate) fn other_state(
    doc: &Document,
    against: &CompareAgainst,
    prefix: &str,
) -> Result<(Document, String), Stop> {
    match against {
        CompareAgainst::History { steps } => {
            let held = doc.undo_labels();
            let n = *steps as usize;
            doc.past(n)
                .map(|(d, label)| (d, format!("{n} adım geri, “{label}” öncesi")))
                .ok_or_else(|| {
                    let held = held.len();
                    let message = if held == 0 {
                        "Geri alma geçmişinde adım yok; önce çizimde bir değişiklik yapın.".to_owned()
                    } else {
                        format!(
                            "Geri alma geçmişi {held} adım tutuyor; {steps} adım geri istendi. 1 ile {held} arasında bir sayı verin."
                        )
                    };
                    Stop::failed(error(
                        codes::NO_SUCH_STEP,
                        message,
                        Some(format!("{prefix}.steps")),
                    ))
                })
        }
        CompareAgainst::File { path } => {
            let p = Path::new(path);
            if path.trim().is_empty() || !p.is_absolute() {
                return Err(Stop::failed(error(
                    codes::INVALID_FILE_PATH,
                    format!(
                        "“{path}” mutlak bir dosya yolu değil; dosyanın tam yolunu verin (/klasör/ad.kcad)."
                    ),
                    Some(format!("{prefix}.path")),
                )));
            }
            let bytes = std::fs::read(p).map_err(|_| {
                Stop::failed(error(
                    codes::FILE_NOT_FOUND,
                    format!("{path} okunamadı: dosya yok ya da açılamıyor."),
                    Some(format!("{prefix}.path")),
                ))
            })?;
            let unreadable = |why: String| {
                Stop::failed(error(
                    codes::IMPORT_UNREADABLE,
                    format!(
                        "{} bir KentOS çizimi olarak okunamadı: {why}",
                        file_name(path)
                    ),
                    Some(format!("{prefix}.path")),
                ))
            };
            let snapshot = read_drawing(&bytes).map_err(unreadable)?;
            let drawing = Document::from_snapshot_v2(snapshot).map_err(|why| {
                Stop::failed(error(
                    codes::IMPORT_UNREADABLE,
                    format!(
                        "{} bir KentOS çizimi olarak okunamadı: {why}",
                        file_name(path)
                    ),
                    Some(format!("{prefix}.path")),
                ))
            })?;
            Ok((drawing, file_name(path)))
        }
    }
}

/// The ids of `slots` in `doc`, the first `limit`; whether it cut.
fn uids(doc: &Document, slots: &[Slot], limit: usize) -> (Vec<String>, bool) {
    let list = slots
        .iter()
        .take(limit)
        .filter_map(|s| doc.uid(*s))
        .map(|u| u.to_string())
        .collect();
    (list, slots.len() > limit)
}

/// What the command answers; why not.
pub(crate) fn compare_in(doc: &Document, input: &DrawingCompare) -> Result<DrawingCompared, Stop> {
    let tolerance = input.tolerance.unwrap_or(COMPARE_TOLERANCE);
    if !(tolerance.is_finite() && (0.0..=MAX_COMPARE_TOLERANCE).contains(&tolerance)) {
        return Err(Stop::failed(error(
            codes::INVALID_TOLERANCE,
            format!("Tolerans 0 ile {MAX_COMPARE_TOLERANCE} metre arasında olmalı."),
            Some("tolerance".into()),
        )));
    }
    let limit = input.limit.unwrap_or(COMPARE_LIMIT);
    if limit == 0 || limit > MAX_COMPARE_LIMIT {
        return Err(Stop::failed(error(
            codes::INVALID_LIMIT,
            format!("Liste sınırı 1 ile {MAX_COMPARE_LIMIT} arasında bir tam sayı olmalı."),
            Some("limit".into()),
        )));
    }
    let limit = limit as usize;
    let (old, against) = other(doc, &input.against)?;
    let d = diff(&old, doc, tolerance);
    let (added_uids, cut_added) = uids(doc, &d.added, limit);
    let (removed_uids, cut_removed) = uids(&old, &d.removed, limit);
    let changed_objects: Vec<ChangedObject> = d
        .changed
        .iter()
        .take(limit)
        .filter_map(|c| {
            Some(ChangedObject {
                uid: doc.uid(c.new)?.to_string(),
                status: c.status.to_owned(),
                fields: c.fields.clone(),
                distance: c.distance,
            })
        })
        .collect();
    Ok(DrawingCompared {
        against,
        same: d.same as u32,
        changed: d.changed.len() as u32,
        added: d.added.len() as u32,
        removed: d.removed.len() as u32,
        added_uids,
        removed_uids,
        changed_objects,
        truncated: cut_added || cut_removed || d.changed.len() > limit,
        revision: doc.revision().to_string(),
    })
}

pub fn validate(cx: &ExecutionContext<'_>, input: &DrawingCompare) -> CommandResult<()> {
    match compare_in(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(cx: &ExecutionContext<'_>, input: &DrawingCompare) -> CommandResult<DrawingCompared> {
    match compare_in(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Read-only: the same answer as the plan.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: DrawingCompare,
) -> CommandResult<DrawingCompared> {
    plan(cx, &input)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::DocumentSnapshotV2;
    use serde_json::{Value, json};

    #[test]
    fn unchanged_inserts_show_a_changed_block_definition() {
        let old: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/kcad/v2/dynamic-blocks.json"
        ))
        .unwrap();
        let mut revised = old.clone();
        revised["blocks"][0]["entities"][0]["b"]["y"] = json!(1.2);
        let before = Document::from_snapshot_v2(
            serde_json::from_value::<DocumentSnapshotV2>(old).unwrap(),
        )
        .unwrap();
        let after = Document::from_snapshot_v2(
            serde_json::from_value::<DocumentSnapshotV2>(revised).unwrap(),
        )
        .unwrap();
        let delta = diff(&before, &after, COMPARE_TOLERANCE);
        assert_eq!(delta.added.len(), 0);
        assert_eq!(delta.removed.len(), 0);
        assert_eq!(delta.changed.len(), 3, "each visible insert must be marked");
        assert!(delta
            .changed
            .iter()
            .all(|c| c.fields.contains(&"blockDefinition".to_owned())));
        let slot = delta.changed[0].old;
        assert_eq!(
            differs(&before, slot, &after, delta.changed[0].new, COMPARE_TOLERANCE),
            Some(true)
        );
    }

    #[test]
    fn color_only_layer_change_is_an_attribute_difference() {
        let old: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/kcad/v2/dynamic-blocks.json"
        ))
        .unwrap();
        let mut revised = old.clone();
        revised["layers"][1]["style"]["color"] = json!("#6495ED");
        let before = Document::from_snapshot_v2(
            serde_json::from_value::<DocumentSnapshotV2>(old).unwrap(),
        )
        .unwrap();
        let after = Document::from_snapshot_v2(
            serde_json::from_value::<DocumentSnapshotV2>(revised).unwrap(),
        )
        .unwrap();
        let delta = diff(&before, &after, COMPARE_TOLERANCE);
        assert_eq!((delta.same, delta.changed.len()), (0, 3));
        assert!(delta.changed.iter().all(|c| {
            c.status == "attributes" && c.fields == ["layerStyle".to_owned()]
        }));
    }

    #[test]
    fn moved_dimension_text_is_detected_without_false_geometry_change() {
        let old: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/kcad/v2/dimension-options.json"
        ))
        .unwrap();
        let mut revised = old.clone();
        revised["entities"][1]["textShift"]["along"] = json!(2.5);
        let before = Document::from_snapshot_v2(
            serde_json::from_value::<DocumentSnapshotV2>(old).unwrap(),
        )
        .unwrap();
        let after = Document::from_snapshot_v2(
            serde_json::from_value::<DocumentSnapshotV2>(revised).unwrap(),
        )
        .unwrap();
        let delta = diff(&before, &after, COMPARE_TOLERANCE);
        assert_eq!(delta.changed.len(), 1);
        assert_eq!(delta.changed[0].status, "attributes");
        assert_eq!(delta.changed[0].fields, ["dimensionPresentation".to_owned()]);
        assert_eq!(delta.changed[0].distance, None);
    }

    #[test]
    fn dimension_rounding_override_is_not_called_geometry() {
        let old: Value = serde_json::from_str(include_str!(
            "../../../../fixtures/kcad/v2/dimension-options.json"
        ))
        .unwrap();
        let mut revised = old.clone();
        revised["entities"][1]["options"]["round"] = json!(0.1);
        let before = Document::from_snapshot_v2(
            serde_json::from_value::<DocumentSnapshotV2>(old).unwrap(),
        )
        .unwrap();
        let after = Document::from_snapshot_v2(
            serde_json::from_value::<DocumentSnapshotV2>(revised).unwrap(),
        )
        .unwrap();
        let delta = diff(&before, &after, COMPARE_TOLERANCE);
        assert_eq!(delta.changed.len(), 1);
        assert_eq!(delta.changed[0].status, "attributes");
        assert!(delta.changed[0].fields.contains(&"dimensionPresentation".to_owned()));
    }
}
