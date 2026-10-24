//! `cad.survey.adjust` v1 (docs/adr/0203 §7–§8, 0622 §9): Yatay ağ dengelemesi and Kot ağı dengelemesi. The network
//! the caller gives is adjusted by the shared core (`survey::adjust`) with the project's a priori standard deviations
//! and grid; a known point given by its name alone is the drawing's, a new point the drawing has starts where it has
//! it. The new points are written, never the known ones: Yatay ağ moves the drawing's points of the same name to their
//! adjusted places with the vertices of lines, polylines and areas on them (the point editor's rule, docs/adr/0153 §3)
//! through `cad.entities.edit`'s `networkAdjust`, and adds the others on the layer given through `cad.entities.create`;
//! Kot ağı gives the drawing's points of the same name their adjusted heights through `levelAdjust`, the vertices on
//! them and their `Z (m)` too, and says the points it has not. One undo step named after the window. The shared cases
//! are `fixtures/commands/v1/cad.survey.adjust.json` (`scripts/fixtures/survey_adjust_command_cases.py`).

use std::collections::BTreeMap;

use kentos_commands::{
    AdjustWork, AngleUnit, CommandResult, CommandWarning, CreateOperation, EditOperation,
    EntitiesCreate, EntitiesEdit, EntitiesEditPlan, EntitiesSetProperties, Entity, EntityEdit,
    EntityGeometry, LevelMethod, PointEntity, PropertiesOperation, SurveyAdjust, SurveyAdjusted,
    SurveyAdjustedPoint, Vec2 as Wire,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::point_editor::follow_point;
use kentos_geometry_core::survey::adjust::Sigmas;
use kentos_geometry_core::survey::adjust::horizontal::{
    self, ApproxPoint, KnownPoint, NetRow, NetworkInput,
};
use kentos_geometry_core::survey::adjust::levelling::{self, KnownHeight, LevelInput, LevelRow};
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_geometry_core::vec2::Vec2;
use kentos_project::format::fixed;

use crate::checks::{self, Stop, error};
use crate::elevation::{paths as elevated_paths, with_paths};
use crate::survey_points::{NewPoint, new_objects, survey_grid};
use crate::{ExecutionContext, codes, create, edit, set};

/// The undo steps' names, the windows'.
pub const NETWORK_STEP: &str = "Yatay ağ dengelemesi";
pub const LEVEL_STEP: &str = "Kot ağı dengelemesi";
/// The kind of the points Yatay ağ adds (their `Tür`).
pub const POINT_KIND: &str = "Ağ noktası";
/// The attribute the Hesap windows write a point's height in beside its z (three decimals).
const Z_ATTRIBUTE: &str = "Z (m)";

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

/// Big letters the Turkish way (i İ, ı I), for names compared alike.
fn upper(s: &str) -> String {
    s.chars()
        .flat_map(|c| match c {
            'i' => vec!['İ'],
            'ı' => vec!['I'],
            c => c.to_uppercase().collect(),
        })
        .collect()
}

fn key(name: &str) -> String {
    upper(js_trim(name))
}

/// The drawing's points named `name` (their label, else their `Ad`, compared the Turkish way), in its order.
fn named_points<'d>(doc: &'d Document, name: &str) -> Vec<&'d PointEntity> {
    let want = key(name);
    doc.entities()
        .filter_map(|e| match e {
            Entity::Point(p) => {
                let own = p
                    .base
                    .label
                    .as_deref()
                    .or_else(|| p.base.attrs.get("Ad").map(String::as_str))
                    .unwrap_or("");
                (!own.is_empty() && !want.is_empty() && key(own) == want).then_some(p)
            }
            _ => None,
        })
        .collect()
}

/// The core's a priori standard deviations, the project's (docs/adr/0203 §1).
fn sigmas(doc: &Document) -> Sigmas {
    let s = doc.settings().sigmas();
    Sigmas {
        direction: s.direction,
        distance: s.distance,
        ppm: s.ppm,
        centering: s.centering,
        zenith: s.zenith,
        levelling: s.levelling,
    }
}

fn finite(v: f64, what: &str, path: String) -> Result<(), Stop> {
    checks::finite(v, what, "Sonlu bir sayı verin.", &path)
}

/// The network's numbers, every one finite.
fn numbers(work: &AdjustWork) -> Result<(), Stop> {
    match work {
        AdjustWork::Horizontal { known, rows } => {
            for (i, k) in known.iter().enumerate() {
                if let Some(at) = k.at {
                    checks::point(at, "Bilinen noktanın", &format!("work.known[{i}].at"))?;
                }
                if let Some(s) = k.sigma {
                    finite(s, "Bilinen noktanın σ'sı", format!("work.known[{i}].sigma"))?;
                }
            }
            for (i, r) in rows.iter().enumerate() {
                if let Some(d) = r.direction {
                    finite(d, "Doğrultu", format!("work.rows[{i}].direction"))?;
                }
                if let Some(d) = r.distance {
                    finite(d, "Kenar", format!("work.rows[{i}].distance"))?;
                }
            }
        }
        AdjustWork::Level { known, rows, .. } => {
            for (i, k) in known.iter().enumerate() {
                if let Some(h) = k.h {
                    finite(h, "Bilinen kot", format!("work.known[{i}].h"))?;
                }
                if let Some(s) = k.sigma {
                    finite(s, "Bilinen kotun σ'sı", format!("work.known[{i}].sigma"))?;
                }
            }
            for (i, r) in rows.iter().enumerate() {
                finite(r.dh, "Kot farkı", format!("work.rows[{i}].dh"))?;
                finite(r.length, "Uzunluk", format!("work.rows[{i}].length"))?;
            }
        }
    }
    Ok(())
}

/// A point to move, raise or lower: its slot and what it becomes.
struct Change {
    slot: Slot,
    to: Vec2,
    z: Option<f64>,
    /// Kot ağı: its `Z (m)` written too (it has one).
    z_attribute: bool,
}

/// What the adjustment writes.
struct Adjusted {
    level: bool,
    points: Vec<SurveyAdjustedPoint>,
    changes: Vec<Change>,
    adds: Vec<NewPoint>,
    absent: Vec<String>,
    f: u32,
    m0: Option<f64>,
    passed: Option<bool>,
}

fn adjusted(doc: &Document, input: &SurveyAdjust) -> Result<Adjusted, Stop> {
    numbers(&input.work)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    match &input.work {
        AdjustWork::Horizontal { known, rows } => {
            let mut given = Vec::with_capacity(known.len());
            for (i, k) in known.iter().enumerate() {
                let at = match k.at {
                    Some(at) => Vec2::new(at.x, at.y),
                    None => match named_points(doc, &k.name).first() {
                        Some(p) => Vec2::new(p.p.x, p.p.y),
                        None => {
                            return Err(refuse(
                                codes::POINT_NOT_FOUND,
                                format!(
                                    "“{}” adlı nokta çizimde yok. Adını denetleyin ya da koordinatlarını verin.",
                                    js_trim(&k.name)
                                ),
                                &format!("work.known[{i}].name"),
                            ));
                        }
                    },
                };
                given.push(KnownPoint {
                    name: k.name.clone(),
                    y: at.x,
                    x: at.y,
                    sigma: k.sigma,
                });
            }
            let rows: Vec<NetRow> = rows
                .iter()
                .enumerate()
                .map(|(i, r)| NetRow {
                    station: r.station.clone(),
                    target: r.target.clone(),
                    direction: r.direction,
                    distance: r.distance,
                    line: Some(i + 1),
                })
                .collect();
            // The new points' places in the drawing, where it has them.
            let mut approx: Vec<ApproxPoint> = Vec::new();
            for r in &rows {
                for name in [&r.station, &r.target] {
                    if approx.iter().any(|a| a.name == *name) {
                        continue;
                    }
                    if let Some(p) = named_points(doc, name).first() {
                        approx.push(ApproxPoint {
                            name: name.clone(),
                            y: p.p.x,
                            x: p.p.y,
                        });
                    }
                }
            }
            let unit = match input.unit.unwrap_or(doc.settings().angle_unit) {
                AngleUnit::Grad => "grad",
                AngleUnit::Deg => "deg",
            };
            let network = NetworkInput {
                unit: unit.to_owned(),
                sigma: sigmas(doc),
                grid: survey_grid(doc.settings()),
                known: given,
                approx,
                rows,
            };
            let result = horizontal::adjust(&network)
                .map_err(|e| refuse(codes::SURVEY_REFUSED, e, "work"))?;
            let known_keys: Vec<String> = known.iter().map(|k| key(&k.name)).collect();
            let mut points = Vec::new();
            let mut changes = Vec::new();
            let mut adds = Vec::new();
            for p in result
                .points
                .iter()
                .filter(|p| !known_keys.contains(&key(&p.name)))
            {
                let to = Vec2::new(p.y, p.x);
                points.push(SurveyAdjustedPoint {
                    name: p.name.clone(),
                    at: Some(Wire { x: to.x, y: to.y }),
                    h: None,
                    sigma: p.sp,
                });
                let there = named_points(doc, &p.name);
                if there.is_empty() {
                    adds.push(NewPoint {
                        name: p.name.clone(),
                        p: to,
                        z: None,
                    });
                }
                for d in there {
                    changes.push(Change {
                        slot: Slot(d.base.id),
                        to,
                        z: d.z,
                        z_attribute: false,
                    });
                }
            }
            Ok(Adjusted {
                level: false,
                points,
                changes,
                adds,
                absent: Vec::new(),
                f: result.f as u32,
                m0: result.m0,
                passed: result.passed,
            })
        }
        AdjustWork::Level {
            method,
            known,
            rows,
        } => {
            let mut given = Vec::with_capacity(known.len());
            for (i, k) in known.iter().enumerate() {
                let h = match k.h {
                    Some(h) => h,
                    None => match named_points(doc, &k.name).iter().find_map(|p| p.z) {
                        Some(z) => z,
                        None => {
                            return Err(refuse(
                                codes::POINT_NOT_FOUND,
                                format!(
                                    "“{}” adlı kotlu nokta çizimde yok. Adını denetleyin ya da kotunu verin.",
                                    js_trim(&k.name)
                                ),
                                &format!("work.known[{i}].name"),
                            ));
                        }
                    },
                };
                given.push(KnownHeight {
                    name: k.name.clone(),
                    h,
                    sigma: k.sigma,
                });
            }
            let network = LevelInput {
                kind: match method {
                    LevelMethod::Geometric => "geometric",
                    LevelMethod::Trigonometric => "trigonometric",
                }
                .to_owned(),
                sigma: sigmas(doc),
                known: given,
                rows: rows
                    .iter()
                    .enumerate()
                    .map(|(i, r)| LevelRow {
                        from: r.from.clone(),
                        to: r.to.clone(),
                        dh: r.dh,
                        length: r.length,
                        line: Some(i + 1),
                    })
                    .collect(),
            };
            let result = levelling::adjust(&network)
                .map_err(|e| refuse(codes::SURVEY_REFUSED, e, "work"))?;
            let known_keys: Vec<String> = known.iter().map(|k| key(&k.name)).collect();
            let mut points = Vec::new();
            let mut changes = Vec::new();
            let mut absent = Vec::new();
            for p in result
                .points
                .iter()
                .filter(|p| !known_keys.contains(&key(&p.name)))
            {
                points.push(SurveyAdjustedPoint {
                    name: p.name.clone(),
                    at: None,
                    h: Some(p.h),
                    sigma: p.sh,
                });
                let there = named_points(doc, &p.name);
                if there.is_empty() {
                    absent.push(p.name.clone());
                }
                for d in there {
                    changes.push(Change {
                        slot: Slot(d.base.id),
                        to: Vec2::new(d.p.x, d.p.y),
                        z: Some(p.h),
                        z_attribute: d.base.attrs.contains_key(Z_ATTRIBUTE),
                    });
                }
            }
            Ok(Adjusted {
                level: true,
                points,
                changes,
                adds: Vec::new(),
                absent,
                f: result.f as u32,
                m0: result.m0,
                passed: result.passed,
            })
        }
    }
}

/// The edits of the changes: each point and the vertices on it, in one list (the window's rule).
fn edits(doc: &Document, changes: &[Change], set_z: bool) -> Vec<EntityEdit> {
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

/// What the writes will be: the edit (its changes checked for locked layers first) and the points to add.
struct Writes {
    edit: Option<EntitiesEdit>,
    create: Option<EntitiesCreate>,
}

fn writes(doc: &Document, input: &SurveyAdjust, a: &Adjusted) -> Result<Writes, Stop> {
    let changes = edits(doc, &a.changes, a.level);
    let layers = doc.layers();
    for change in &changes {
        let EntityEdit::Update { uid, .. } = change else {
            continue;
        };
        let layer = kentos_domain::Uuid::parse_str(uid)
            .ok()
            .and_then(|u| doc.slot_of(u))
            .and_then(|s| doc.get(s))
            .map(|e| e.base().layer_id.as_str())
            .unwrap_or_default();
        if layers.is_locked(layer) {
            let name = layers.get(layer).map_or(layer, |n| n.name.as_str());
            return Err(refuse(
                codes::LAYER_LOCKED,
                format!(
                    "“{name}” katmanı kilitli; dengelenen noktalar ve onlara bağlı çizgiler yazılamaz. Kilidi Katmanlar panelinden açın."
                ),
                "work",
            ));
        }
    }
    let create = if a.adds.is_empty() {
        None
    } else {
        let Some(layer) = input.layer_id.clone() else {
            return Err(refuse(
                codes::NO_LAYER,
                format!(
                    "Çizimde olmayan {} yeni nokta var; eklenecekleri katmanı kimliğiyle verin (layerId).",
                    a.adds.len()
                ),
                "layerId",
            ));
        };
        Some(EntitiesCreate {
            layer_id: layer,
            objects: new_objects(&a.adds, POINT_KIND),
            operation: Some(CreateOperation::NetworkAdjust),
            expected_revision: None,
        })
    };
    let edit = (!changes.is_empty()).then_some(EntitiesEdit {
        operation: if a.level {
            EditOperation::LevelAdjust
        } else {
            EditOperation::NetworkAdjust
        },
        changes,
        expected_revision: None,
    });
    Ok(Writes { edit, create })
}

/// The `Z (m)` written beside a height raised (three decimals).
fn z_input(uid: String, z: f64) -> EntitiesSetProperties {
    EntitiesSetProperties {
        line_pattern: None,
        line_type: None,
        line_type_scale: None,
        transparency: None,
        wipeout: None,
        uids: vec![uid],
        layer_id: None,
        color: None,
        line_weight: None,
        symbol: None,
        attrs: Some(BTreeMap::from([(
            Z_ATTRIBUTE.to_owned(),
            Some(fixed(z, 3)),
        )])),
        label: None,
        operation: PropertiesOperation::Attributes,
        expected_revision: None,
        unlink: false,
        attachments: None,
    }
}

/// Kot ağı's new points the drawing has not, said.
fn not_drawn(absent: &[String]) -> Option<CommandWarning> {
    (!absent.is_empty()).then(|| CommandWarning {
        code: codes::POINTS_NOT_DRAWN.into(),
        message: format!("{} çizimde yok; yazılmadı.", absent.join(", ")),
        path: Some("work".into()),
    })
}

/// The adjustment, the checks of its writes and their warnings.
pub fn validate(cx: &ExecutionContext<'_>, input: &SurveyAdjust) -> CommandResult<()> {
    match plan_in(cx.doc, input) {
        CommandResult::Completed { warnings, .. } => CommandResult::Completed {
            output: (),
            warnings,
        },
        other => crate::trim::relay(other),
    }
}

/// The points as they would be moved, raised or added.
pub fn plan(cx: &ExecutionContext<'_>, input: &SurveyAdjust) -> CommandResult<EntitiesEditPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document (the windows').
pub fn plan_in(doc: &Document, input: &SurveyAdjust) -> CommandResult<EntitiesEditPlan> {
    let (a, w) = match adjusted(doc, input).and_then(|a| writes(doc, input, &a).map(|w| (a, w))) {
        Ok(v) => v,
        Err(stop) => return stop.into(),
    };
    let mut plan = EntitiesEditPlan {
        changed: Vec::new(),
        created: Vec::new(),
        removed: Vec::new(),
        revision: doc.revision().to_string(),
    };
    let mut warnings: Vec<CommandWarning> = Vec::new();
    if let Some(edit) = &w.edit {
        match edit::plan_in(doc, edit) {
            CommandResult::Completed {
                output,
                warnings: more,
            } => {
                plan.changed = output.changed;
                warnings.extend(more);
            }
            other => return crate::trim::relay(other),
        }
    }
    // Kot ağı's Z (m) on the points that have one, as the write gives it.
    for c in a.changes.iter().filter(|c| c.z_attribute) {
        if let (Some(z), Some(e)) = (
            c.z,
            plan.changed.iter_mut().find(|e| e.base().id == c.slot.0),
        ) {
            e.base_mut()
                .attrs
                .insert(Z_ATTRIBUTE.to_owned(), fixed(z, 3));
        }
    }
    if let Some(create) = &w.create {
        match create::plan_in(doc, create) {
            CommandResult::Completed {
                output,
                warnings: more,
            } => {
                plan.created = output.entities;
                warnings.extend(more);
            }
            other => return crate::trim::relay(other),
        }
    }
    warnings.extend(not_drawn(&a.absent));
    CommandResult::Completed {
        output: plan,
        warnings,
    }
}

/// Adjusts the network and writes its new points, one undo step named after the window.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SurveyAdjust,
) -> CommandResult<SurveyAdjusted> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_SURVEY_ADJUST,
        kentos_commands::CAD_SURVEY_ADJUST_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: SurveyAdjust,
) -> CommandResult<SurveyAdjusted> {
    let (a, w) =
        match adjusted(cx.doc, &input).and_then(|a| writes(cx.doc, &input, &a).map(|w| (a, w))) {
            Ok(v) => v,
            Err(stop) => return stop.into(),
        };
    let moved: Vec<String> = a
        .changes
        .iter()
        .filter_map(|c| cx.doc.uid(c.slot))
        .map(|u| u.to_string())
        .collect();
    let group = cx
        .doc
        .begin_group(if a.level { LEVEL_STEP } else { NETWORK_STEP });
    let mut warnings: Vec<CommandWarning> = Vec::new();
    if let Some(edit) = w.edit {
        match edit::execute(cx, edit) {
            CommandResult::Completed { warnings: more, .. } => warnings.extend(more),
            other => {
                cx.doc.cancel_group(group);
                return crate::trim::relay(other);
            }
        }
    }
    for c in a.changes.iter().filter(|c| c.z_attribute) {
        let (Some(uid), Some(z)) = (cx.doc.uid(c.slot), c.z) else {
            continue;
        };
        match set::execute(cx, z_input(uid.to_string(), z)) {
            CommandResult::Completed { warnings: more, .. } => warnings.extend(more),
            other => {
                cx.doc.cancel_group(group);
                return crate::trim::relay(other);
            }
        }
    }
    let (mut created, mut ids) = (Vec::new(), Vec::new());
    if let Some(create) = w.create {
        match create::execute(cx, create) {
            CommandResult::Completed {
                output,
                warnings: more,
            } => {
                created = output.created;
                ids = output.ids;
                warnings.extend(more);
            }
            other => {
                cx.doc.cancel_group(group);
                return crate::trim::relay(other);
            }
        }
    }
    cx.doc.end_group(group);
    warnings.extend(not_drawn(&a.absent));
    CommandResult::Completed {
        output: SurveyAdjusted {
            points: a.points,
            moved,
            created,
            ids,
            absent: a.absent,
            f: a.f,
            m0: a.m0,
            passed: a.passed,
            revision: cx.doc.revision().to_string(),
        },
        warnings,
    }
}
