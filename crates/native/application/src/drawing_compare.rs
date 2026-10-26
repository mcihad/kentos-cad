//! `cad.drawing.compare` (docs/adr/0576 §3): what changed between a drawing as it is and the same drawing as it was,
//! its history some undo steps back or a drawing file. Read only.
//!
//! The objects are paired by their persistent ids (ADR 0014) and told apart by the data compare core (ADR 0179,
//! `ops::compare`, matching by a key: each object's id), so a moved object is a changed one, not an added and a
//! removed. The geometry is the same within the tolerance; the attributes and the layer are equal or not.
//! [`diff`] is the rule the desktop's second view highlights with.

use std::path::Path;

use kentos_commands::{
    COMPARE_LIMIT, COMPARE_TOLERANCE, ChangedObject, CommandResult, CompareAgainst, DrawingCompare,
    DrawingCompared, MAX_COMPARE_LIMIT, MAX_COMPARE_TOLERANCE,
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

fn member(uid: kentos_domain::Uuid, e: &kentos_commands::Entity) -> Member {
    let base = e.base();
    let mut attrs: Vec<(String, String)> = base
        .attrs
        .iter()
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    attrs.push((UID.to_owned(), uid.to_string()));
    attrs.push((LAYER.to_owned(), base.layer_id.clone()));
    Member {
        shape: crate::geometry::shape(e),
        attrs: Attrs(attrs),
    }
}

fn members(doc: &Document) -> (Vec<Member>, Vec<Slot>) {
    let mut out = Vec::new();
    let mut slots = Vec::new();
    for (uid, e) in doc.entities_with_uids() {
        out.push(member(uid, e));
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
    let a = member(old.uid(old_slot)?, old.get(old_slot)?);
    let b = member(new.uid(new_slot)?, new.get(new_slot)?);
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
                        } else {
                            f.clone()
                        }
                    })
                    .collect();
                out.changed.push(Changed {
                    old: a_slots[o],
                    new: b_slots[n],
                    status: match status {
                        Status::Geometry => "geometry",
                        Status::Both => "both",
                        _ => "attributes",
                    },
                    fields,
                    distance: row.distance,
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
