//! `cad.nurbs.create` v1 (docs/adr/0259 §3): one NURBS curve on a named
//! layer, by its control points or through points it passes, open or
//! closed, as one undo step. The curve is built by the shared core
//! (`geom::nurbs_build`): the tools' previews take this command's plan, and
//! Python and an agent get the same curve from the same points.
//! The shared cases are `fixtures/commands/v1/cad.nurbs.create.json`.
//!
//! The checks, in order (the first that fails answers):
//! 1. every coordinate, weight and knot finite, in the input's order;
//! 2. no two fit points in a row at one place (`coincident_points`);
//! 3. a curve: the degree, the counts, the knots (`invalid_nurbs`);
//! 4. the line weight; 5. the expected revision; 6. the layer.

use kentos_contracts::{
    CommandResult, CommandWarning, Entity, EntityBase, NurbsBy, NurbsCreate, NurbsCreated,
    NurbsEntity, NurbsPlan, Vec2,
};
use kentos_domain::Document;
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::nurbs_build::{self, BuildError};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &NurbsCreate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok((warnings, _)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now, and the revision to expect for exactly that; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &NurbsCreate) -> CommandResult<NurbsPlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the curve tools draw as their preview.
pub fn plan_in(doc: &Document, input: &NurbsCreate) -> CommandResult<NurbsPlan> {
    match check(doc, input) {
        Ok((warnings, curve)) => CommandResult::Completed {
            output: NurbsPlan {
                entity: entity(input.clone(), curve),
                revision: doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the curve as one undo step (“Ekle”).
pub fn execute(cx: &mut ExecutionContext<'_>, input: NurbsCreate) -> CommandResult<NurbsCreated> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_NURBS_CREATE,
        kentos_contracts::CAD_NURBS_CREATE_VERSION,
        input,
        write_execute,
    )
}

fn write_execute(cx: &mut ExecutionContext<'_>, input: NurbsCreate) -> CommandResult<NurbsCreated> {
    let (warnings, curve) = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    match checks::add(cx.doc, entity(input, curve)) {
        Ok((id, uid, revision)) => CommandResult::Completed {
            output: NurbsCreated { uid, id, revision },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// The curve as the core builds it: degree, control points, knots, weights.
struct Curve {
    degree: u32,
    ctrl: Vec<Vec2>,
    knots: Vec<f64>,
    weights: Option<Vec<f64>>,
    fit: Option<Vec<Vec2>>,
}

fn core(p: &Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

fn back(p: &CoreVec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

fn invalid(why: String, path: &str) -> Stop {
    Stop::failed(error(codes::INVALID_NURBS, why, Some(path.into())))
}

fn numbers(list: &[f64], what: &str, path: &str) -> Result<(), Stop> {
    for (i, x) in list.iter().enumerate() {
        checks::finite(
            *x,
            &format!("{}. {what}", i + 1),
            "Sonlu bir sayı verin.",
            &format!("{path}[{i}]"),
        )?;
    }
    Ok(())
}

fn points(list: &[Vec2], whose: &str, path: &str) -> Result<(), Stop> {
    for (i, p) in list.iter().enumerate() {
        checks::point(*p, &format!("{}. {whose}", i + 1), &format!("{path}[{i}]"))?;
    }
    Ok(())
}

/// The checks in the contract's order; the warnings and the curve when it may be written.
fn check(doc: &Document, input: &NurbsCreate) -> Result<(Vec<CommandWarning>, Curve), Stop> {
    let curve = match &input.by {
        NurbsBy::Control {
            degree,
            ctrl,
            weights,
            knots,
        } => {
            points(ctrl, "denetim noktasının", "by.ctrl")?;
            if let Some(w) = weights {
                numbers(w, "ağırlık", "by.weights")?;
            }
            if let Some(k) = knots {
                numbers(k, "düğüm", "by.knots")?;
            }
            let pts: Vec<CoreVec2> = ctrl.iter().map(core).collect();
            let built = nurbs_build::by_control(
                *degree as usize,
                &pts,
                weights.as_deref(),
                knots.as_deref(),
                input.closed,
            )
            .map_err(|e| {
                let why = match e {
                    BuildError::Invalid(why) => why,
                    BuildError::Coincident(_) => String::new(),
                };
                let path = if !(1..=nurbs_build::MAX_DEGREE as u32).contains(degree) {
                    "by.degree"
                } else if weights.as_ref().is_some_and(|w| w.len() != ctrl.len()) {
                    "by.weights"
                } else if input.closed && knots.is_some() {
                    "by.knots"
                } else {
                    "by.ctrl"
                };
                invalid(why, path)
            })?;
            Curve {
                degree: *degree,
                ctrl: built.ctrl.iter().map(back).collect(),
                knots: built.knots,
                weights: built.weights,
                fit: None,
            }
        }
        NurbsBy::Fit { pts, degree } => {
            points(pts, "geçiş noktasının", "by.pts")?;
            let core_pts: Vec<CoreVec2> = pts.iter().map(core).collect();
            let asked = degree.unwrap_or(3) as usize;
            let built = nurbs_build::through(&core_pts, asked, input.closed).map_err(|e| match e {
                BuildError::Coincident(i) => {
                    let n = if input.closed {
                        nurbs_build::closed_fit(&core_pts).len()
                    } else {
                        pts.len()
                    };
                    Stop::failed(error(
                        codes::COINCIDENT_POINTS,
                        format!(
                            "{}. ve {}. geçiş noktası aynı yerde; eğri her noktadan bir kez geçer. Yinelenen noktayı çıkarın.",
                            i + 1,
                            (i + 1) % n + 1
                        ),
                        Some(format!("by.pts[{}]", (i + 1) % n)),
                    ))
                }
                BuildError::Invalid(why) => invalid(
                    why,
                    if (1..=nurbs_build::MAX_DEGREE).contains(&asked) {
                        "by.pts"
                    } else {
                        "by.degree"
                    },
                ),
            })?;
            let kept = if input.closed {
                nurbs_build::closed_fit(&core_pts)
                    .iter()
                    .map(back)
                    .collect()
            } else {
                pts.clone()
            };
            Curve {
                degree: built.degree as u32,
                ctrl: built.ctrl.iter().map(back).collect(),
                knots: built.knots,
                weights: None,
                fit: Some(kept),
            }
        }
    };
    if let Some(why) = kentos_contracts::nurbs_problem(
        curve.degree,
        &curve.ctrl,
        &curve.knots,
        curve.weights.as_deref(),
        curve.fit.as_deref(),
        None,
    ) {
        let path = match &input.by {
            NurbsBy::Control {
                weights: Some(w), ..
            } if w.iter().any(|x| *x <= 0.0) => "by.weights",
            NurbsBy::Control { knots: Some(_), .. } => "by.knots",
            _ => "by",
        };
        return Err(invalid(why, path));
    }
    checks::line_weight(input.line_weight, "lineWeight")?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    Ok((checks::layer(doc, &input.layer_id)?, curve))
}

/// The curve as the document stores it; `id` is the document's to give (0 in a plan).
fn entity(input: NurbsCreate, curve: Curve) -> Entity {
    Entity::Nurbs(NurbsEntity {
        base: EntityBase {
            id: 0,
            layer_id: input.layer_id,
            color: input.color,
            attrs: input.attrs.unwrap_or_default(),
            label: input.label,
            symbol: input.symbol,
            line_weight: input.line_weight,
            label_pins: Vec::new(),
            attachments: Vec::new(),
        },
        degree: curve.degree,
        ctrl: curve.ctrl,
        knots: curve.knots,
        weights: curve.weights,
        closed: input.closed,
        periodic: input.closed,
        fit: curve.fit,
        tangents: None,
        fit_tolerance: None,
    })
}
