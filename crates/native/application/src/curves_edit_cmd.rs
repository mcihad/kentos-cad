//! `cad.curves.edit` v1 (docs/adr/0303; AutoCAD's PEDIT and SPLINEDIT, Eğriyi düzenle): a polyline or a NURBS
//! curve edited by what changes, as one undo step “Eğriyi düzenle”. The curve is the shared core's
//! (`nurbs_build::through`, `through_tangents`, `by_control`; `Nurbs::insert`, `tessellate`), written through
//! `cad.entities.edit`'s `curveEdit` operation as an `update`, so the object keeps its layer, look and data.
//! The shared cases are `fixtures/commands/v1/cad.curves.edit.json`.
//!
//! A closed (periodic) curve keeps its first `degree` control points (and weights) wrapped after the last:
//! moving or weighing one moves or weighs its copy too, and its points are numbered without the copies.

use kentos_contracts::{
    CommandResult, CurveEdit, CurvesEdit, EditOperation, EntitiesEdit, EntitiesEditPlan,
    EntitiesEdited, Entity, EntityEdit, EntityGeometry, NurbsEntity, PathEntity, Vec2,
};
use kentos_domain::Document;
use kentos_geometry_core::Vec2 as CoreVec2;
use kentos_geometry_core::geom::nurbs::OwnedNurbs;
use kentos_geometry_core::geom::nurbs_build::{self, BuildError};

use crate::ExecutionContext;
use crate::break_join::{Checked, Ties, execute_with, plan_with, validate_with};
use crate::checks::{self, Stop, error, is_uid_text};
use crate::codes;

/// The default degree of a curve made from a polyline (AutoCAD's SPLINE and PEDIT Spline cubic).
const DEGREE: u32 = 3;

fn refuse(code: &str, message: impl Into<String>, path: &str) -> Stop {
    Stop::failed(error(code, message.into(), Some(path.into())))
}

fn core(p: &Vec2) -> CoreVec2 {
    CoreVec2::new(p.x, p.y)
}

fn back(p: &CoreVec2) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

/// The edit's own numbers, in its fields' order.
fn check_numbers(edit: &CurveEdit) -> Result<(), Stop> {
    let point = |p: &Vec2, path: &str| checks::point(*p, "Noktanın", path);
    match edit {
        CurveEdit::AddFit { at, .. }
        | CurveEdit::MoveFit { at, .. }
        | CurveEdit::MoveControl { at, .. } => point(at, "edit.at"),
        CurveEdit::Tangents { start, end } => {
            if let Some(s) = start {
                point(s, "edit.start")?;
            }
            if let Some(e) = end {
                point(e, "edit.end")?;
            }
            Ok(())
        }
        CurveEdit::Weight { weight, .. } => {
            checks::finite(
                *weight,
                "Ağırlık",
                "Ağırlığı sonlu bir sayıyla verin.",
                "edit.weight",
            )?;
            if *weight <= 0.0 {
                return Err(refuse(
                    codes::INVALID_WEIGHT,
                    "Ağırlık sıfırdan büyük olmalı. Başka bir ağırlık verin.",
                    "edit.weight",
                ));
            }
            Ok(())
        }
        CurveEdit::InsertKnot { at } => {
            checks::finite(*at, "Düğüm", "Düğümü sonlu bir sayıyla verin.", "edit.at")
        }
        CurveEdit::ToPolyline { tolerance } => {
            checks::finite(
                *tolerance,
                "Tolerans",
                "Toleransı sonlu bir sayıyla verin.",
                "edit.tolerance",
            )?;
            if *tolerance <= 0.0 {
                return Err(refuse(
                    codes::INVALID_TOLERANCE,
                    "Tolerans sıfırdan büyük olmalı. Başka bir tolerans verin.",
                    "edit.tolerance",
                ));
            }
            Ok(())
        }
        _ => Ok(()),
    }
}

/// A refusal for an edit this kind does not take.
fn not_for(message: &str) -> Stop {
    refuse(codes::EDIT_NOT_APPLICABLE, message, "edit")
}

/// The curve a build gives, or its refusal.
fn built(result: Result<OwnedNurbs, BuildError>, path: &str) -> Result<OwnedNurbs, Stop> {
    result.map_err(|e| match e {
        BuildError::Coincident(i) => refuse(
            codes::COINCIDENT_POINTS,
            format!(
                "{}. ve {}. nokta aynı yerde; eğri her noktadan bir kez geçer. Noktalardan birini taşıyın ya da çıkarın.",
                i + 1,
                i + 2
            ),
            path,
        ),
        BuildError::Invalid(why) => refuse(codes::INVALID_NURBS, why, path),
    })
}

/// A NURBS geometry from a curve the core built.
fn nurbs_geometry(
    c: &OwnedNurbs,
    closed: bool,
    fit: Option<Vec<Vec2>>,
    tangents: Option<[Option<Vec2>; 2]>,
) -> EntityGeometry {
    EntityGeometry::Nurbs {
        degree: c.degree as u32,
        ctrl: c.ctrl.iter().map(back).collect(),
        knots: c.knots.clone(),
        weights: c.weights.clone(),
        closed,
        periodic: closed,
        fit,
        tangents,
        fit_tolerance: None,
    }
}

/// The curve as it is: its numbers for the core.
fn owned(n: &NurbsEntity) -> OwnedNurbs {
    OwnedNurbs {
        degree: n.degree as usize,
        ctrl: n.ctrl.iter().map(core).collect(),
        knots: n.knots.clone(),
        weights: n.weights.clone(),
    }
}

/// A NURBS curve's control points as drawn: a closed one's without its wrapped copies.
fn drawn(n: &NurbsEntity) -> usize {
    if n.closed && n.periodic {
        n.ctrl.len().saturating_sub(n.degree as usize)
    } else {
        n.ctrl.len()
    }
}

/// A polyline's geometry as it is, with `closed` and `bulges` given.
fn polyline(p: &PathEntity, closed: bool, bulges: Option<Vec<f64>>) -> EntityGeometry {
    EntityGeometry::Polyline {
        pts: p.pts.clone(),
        bulges,
        zs: p.zs.clone(),
        ms: p.ms.clone(),
        parts: None,
        closed,
        widths: p.widths.clone(),
    }
}

/// The fit points drawn again through, with the tangents an open curve keeps.
fn refit(
    fit: Vec<Vec2>,
    degree: u32,
    closed: bool,
    tangents: Option<[Option<Vec2>; 2]>,
    path: &str,
) -> Result<EntityGeometry, Stop> {
    let pts: Vec<CoreVec2> = fit.iter().map(core).collect();
    let [start, end] = tangents.unwrap_or([None, None]);
    let curve = if closed {
        built(nurbs_build::through(&pts, degree as usize, true), path)?
    } else {
        built(
            nurbs_build::through_tangents(
                &pts,
                degree as usize,
                start.map(|v| core(&v)),
                end.map(|v| core(&v)),
            ),
            path,
        )?
    };
    let kept = if closed {
        nurbs_build::closed_fit(&pts).iter().map(back).collect()
    } else {
        fit
    };
    let tangents = tangents.filter(|t| !closed && (t[0].is_some() || t[1].is_some()));
    Ok(nurbs_geometry(&curve, closed, Some(kept), tangents))
}

fn index_check(index: u32, count: usize, what: &str) -> Result<usize, Stop> {
    let i = index as usize;
    if i >= count {
        return Err(refuse(
            codes::INVALID_INDEX,
            format!(
                "Eğrinin {count} {what} var; {index} numaralısı yok. 0 ile {} arasında bir numara verin.",
                count.saturating_sub(1)
            ),
            "edit.index",
        ));
    }
    Ok(i)
}

/// A polyline's edit.
fn polyline_edit(p: &PathEntity, edit: &CurveEdit) -> Result<EntityGeometry, Stop> {
    if p.parts.as_ref().is_some_and(|parts| !parts.is_empty()) {
        return Err(refuse(
            codes::MULTI_PART,
            "Çok parçalı çoklu çizgi düzenlenmez; önce Parçalara ayır.",
            "uid",
        ));
    }
    let n = p.pts.len();
    match edit {
        CurveEdit::Close => {
            if p.closed {
                return Err(not_for("Çoklu çizgi zaten kapalı."));
            }
            if n < 3 {
                return Err(refuse(
                    codes::TOO_FEW_POINTS,
                    "Kapanan çoklu çizginin en az üç köşesi olmalı.",
                    "uid",
                ));
            }
            Ok(polyline(p, true, p.bulges.clone()))
        }
        CurveEdit::Open => {
            if !p.closed {
                return Err(not_for("Çoklu çizgi zaten açık."));
            }
            // The closing edge's bulge goes with it.
            let bulges = p.bulges.clone().map(|mut b| {
                if b.len() >= n && n > 0 {
                    b[n - 1] = 0.0;
                }
                b
            });
            let mut g = polyline(p, false, bulges);
            if let EntityGeometry::Polyline {
                widths: Some(w), ..
            } = &mut g
                && w.len() == n
                && let Some(last) = w.last_mut()
            {
                *last = [0.0, 0.0];
            }
            Ok(g)
        }
        CurveEdit::Decurve => {
            if !p
                .bulges
                .as_ref()
                .is_some_and(|b| b.iter().any(|x| *x != 0.0))
            {
                return Err(not_for("Çoklu çizginin yay kenarı yok; zaten düz."));
            }
            Ok(polyline(p, p.closed, None))
        }
        CurveEdit::ToSpline { degree, through } => {
            let degree = degree.unwrap_or(DEGREE);
            if !(1..=nurbs_build::MAX_DEGREE as u32).contains(&degree) {
                return Err(refuse(
                    codes::INVALID_NURBS,
                    format!(
                        "Derece {degree}; 1 ile {} arasında olmalı.",
                        nurbs_build::MAX_DEGREE
                    ),
                    "edit.degree",
                ));
            }
            let pts: Vec<CoreVec2> = p.pts.iter().map(core).collect();
            if *through {
                let curve = built(
                    nurbs_build::through(&pts, degree as usize, p.closed),
                    "edit",
                )?;
                let fit = if p.closed {
                    nurbs_build::closed_fit(&pts).iter().map(back).collect()
                } else {
                    p.pts.clone()
                };
                Ok(nurbs_geometry(&curve, p.closed, Some(fit), None))
            } else {
                let p_degree = nurbs_build::fit_degree(degree as usize, n);
                let curve = built(
                    nurbs_build::by_control(p_degree, &pts, None, None, p.closed),
                    "edit",
                )?;
                Ok(nurbs_geometry(&curve, p.closed, None, None))
            }
        }
        _ => Err(not_for(
            "Çoklu çizgide Kapat, Aç, Düzleştir ve Eğriye çevir yapılır; nokta ve düğüm düzenlemeleri NURBS eğrisinindir.",
        )),
    }
}

/// A NURBS curve's edit.
fn nurbs_edit(c: &NurbsEntity, edit: &CurveEdit) -> Result<EntityGeometry, Stop> {
    let p = c.degree as usize;
    let fit_of = || {
        c.fit.clone().ok_or_else(|| {
            refuse(
                codes::NO_FIT_POINTS,
                "Eğrinin geçiş noktası yok: denetim noktalarıyla çizilmiş ya da denetim noktası düzenlenmiş. Denetim noktası düzenlemelerini kullanın.",
                "edit",
            )
        })
    };
    match edit {
        CurveEdit::Close | CurveEdit::Open => {
            let closing = matches!(edit, CurveEdit::Close);
            if c.closed == closing {
                return Err(not_for(if closing {
                    "Eğri zaten kapalı."
                } else {
                    "Eğri zaten açık."
                }));
            }
            if let Some(fit) = c.fit.clone() {
                return refit(
                    fit,
                    c.degree,
                    closing,
                    if closing { None } else { c.tangents },
                    "edit",
                );
            }
            let count = drawn(c);
            let ctrl: Vec<CoreVec2> = c.ctrl[..count].iter().map(core).collect();
            let weights = c.weights.as_ref().map(|w| w[..count].to_vec());
            let curve = built(
                nurbs_build::by_control(p, &ctrl, weights.as_deref(), None, closing),
                "edit",
            )?;
            Ok(nurbs_geometry(&curve, closing, None, None))
        }
        CurveEdit::AddFit { index, at } => {
            let mut fit = fit_of()?;
            if *index as usize > fit.len() {
                return Err(refuse(
                    codes::INVALID_INDEX,
                    format!(
                        "Yeni geçiş noktası 0 ile {} arasındaki bir yere eklenir; {index} numaralı yer yok.",
                        fit.len()
                    ),
                    "edit.index",
                ));
            }
            fit.insert(*index as usize, *at);
            refit(fit, c.degree, c.closed, c.tangents, "edit.at")
        }
        CurveEdit::MoveFit { index, at } => {
            let mut fit = fit_of()?;
            let i = index_check(*index, fit.len(), "geçiş noktası")?;
            fit[i] = *at;
            refit(fit, c.degree, c.closed, c.tangents, "edit.at")
        }
        CurveEdit::RemoveFit { index } => {
            let mut fit = fit_of()?;
            let i = index_check(*index, fit.len(), "geçiş noktası")?;
            fit.remove(i);
            refit(fit, c.degree, c.closed, c.tangents, "edit.index")
        }
        CurveEdit::Tangents { start, end } => {
            let fit = fit_of()?;
            if c.closed {
                return Err(not_for("Kapalı eğrinin uç teğeti olmaz; önce Aç."));
            }
            refit(fit, c.degree, false, Some([*start, *end]), "edit")
        }
        CurveEdit::MoveControl { index, at } => {
            let count = drawn(c);
            let i = index_check(*index, count, "denetim noktası")?;
            let mut ctrl = c.ctrl.clone();
            ctrl[i] = *at;
            if count < ctrl.len() && i < ctrl.len() - count {
                ctrl[i + count] = *at;
            }
            Ok(EntityGeometry::Nurbs {
                degree: c.degree,
                ctrl,
                knots: c.knots.clone(),
                weights: c.weights.clone(),
                closed: c.closed,
                periodic: c.periodic,
                fit: None,
                tangents: None,
                fit_tolerance: None,
            })
        }
        CurveEdit::Weight { index, weight } => {
            let count = drawn(c);
            let i = index_check(*index, count, "denetim noktası")?;
            let mut w = c.weights.clone().unwrap_or_else(|| vec![1.0; c.ctrl.len()]);
            w[i] = *weight;
            if count < w.len() && i < w.len() - count {
                w[i + count] = *weight;
            }
            let weights = w.iter().any(|x| *x != 1.0).then_some(w);
            Ok(EntityGeometry::Nurbs {
                degree: c.degree,
                ctrl: c.ctrl.clone(),
                knots: c.knots.clone(),
                weights,
                closed: c.closed,
                periodic: c.periodic,
                fit: None,
                tangents: None,
                fit_tolerance: None,
            })
        }
        CurveEdit::RemoveControl { index } => {
            let count = drawn(c);
            let i = index_check(*index, count, "denetim noktası")?;
            let mut ctrl: Vec<CoreVec2> = c.ctrl[..count].iter().map(core).collect();
            ctrl.remove(i);
            let weights = c.weights.as_ref().map(|w| {
                let mut w = w[..count].to_vec();
                w.remove(i);
                w
            });
            let least = if c.closed { p.max(3) } else { p + 1 };
            if ctrl.len() < least {
                return Err(refuse(
                    codes::TOO_FEW_POINTS,
                    format!(
                        "{}. dereceden {} eğrinin en az {least} denetim noktası olmalı; silinirse {} kalır.",
                        p,
                        if c.closed { "kapalı" } else { "açık" },
                        ctrl.len()
                    ),
                    "edit.index",
                ));
            }
            let curve = built(
                nurbs_build::by_control(p, &ctrl, weights.as_deref(), None, c.closed),
                "edit.index",
            )?;
            Ok(nurbs_geometry(&curve, c.closed, None, None))
        }
        CurveEdit::InsertKnot { at } => {
            if c.closed {
                return Err(refuse(
                    codes::INVALID_KNOT,
                    "Kapalı eğriye düğüm eklenmez; önce Aç.",
                    "edit.at",
                ));
            }
            let curve = owned(c);
            let (u0, u1) = curve.view().domain();
            let s = c.knots.iter().filter(|k| **k == *at).count();
            if !(*at > u0 && *at < u1) || s >= p {
                return Err(refuse(
                    codes::INVALID_KNOT,
                    format!(
                        "Düğüm eğrinin aralığının içinde ({u0} ile {u1} arasında) ve orada {p} kereden az olmalı."
                    ),
                    "edit.at",
                ));
            }
            let more = curve.view().insert(*at, 1);
            Ok(nurbs_geometry(&more, false, None, None))
        }
        CurveEdit::ToPolyline { tolerance } => {
            let curve = owned(c);
            let mut pts: Vec<Vec2> = curve
                .view()
                .tessellate(*tolerance)
                .iter()
                .map(back)
                .collect();
            // A closed curve ends where it starts: its last point is its first.
            if c.closed && pts.len() > 1 {
                pts.pop();
            }
            Ok(EntityGeometry::Polyline {
                pts,
                bulges: None,
                zs: None,
                ms: None,
                parts: None,
                closed: c.closed,
                widths: None,
            })
        }
        _ => Err(not_for(
            "NURBS eğrisinde Düzleştir ve Eğriye çevir yapılmaz; onlar çoklu çizginin.",
        )),
    }
}

/// The checks in the contract's order, and the edit.
fn check(doc: &Document, input: &CurvesEdit) -> Result<Checked, Stop> {
    if !is_uid_text(&input.uid) {
        return Err(refuse(
            codes::INVALID_UID,
            format!(
                "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi). Kimliği nesneyi oluşturan komutun çıktısından ya da çizimden alın.",
                input.uid
            ),
            "uid",
        ));
    }
    check_numbers(&input.edit)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let uids = [input.uid.clone()];
    // One object: its refusals are said at `uid`.
    let at_uid = |stop: Stop| {
        let mut e = stop.into_error();
        e.path = Some("uid".into());
        Stop::failed(e)
    };
    let found = checks::named(doc, &uids).map_err(at_uid)?;
    let (open, _) = checks::skip_locked(doc, found).map_err(at_uid)?;
    let Some((_, _, e, _)) = open.into_iter().next() else {
        return Err(not_for("Düzenlenecek eğri yok."));
    };
    let geometry = match e {
        Entity::Polyline(p) => polyline_edit(p, &input.edit)?,
        Entity::Nurbs(c) => nurbs_edit(c, &input.edit)?,
        _ => {
            return Err(refuse(
                codes::NOT_A_CURVE,
                "Eğriyi düzenle çoklu çizgide ve NURBS eğrisinde çalışır. Bir çoklu çizgi ya da NURBS eğrisi verin.",
                "uid",
            ));
        }
    };
    Ok((
        EntitiesEdit {
            operation: EditOperation::CurveEdit,
            changes: vec![EntityEdit::Update {
                uid: input.uid.clone(),
                geometry,
            }],
            expected_revision: None,
        },
        Vec::new(),
        Ties::default(),
    ))
}

/// `cad.curves.edit` v1: Eğriyi düzenle.
pub mod curves_edit {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &CurvesEdit) -> CommandResult<()> {
        validate_with(check(cx.doc, input))
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &CurvesEdit) -> CommandResult<EntitiesEditPlan> {
        plan_in(cx.doc, input)
    }

    /// [`plan`] over the document alone (the tool's preview).
    pub fn plan_in(doc: &Document, input: &CurvesEdit) -> CommandResult<EntitiesEditPlan> {
        plan_with(doc, check(doc, input))
    }

    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: CurvesEdit,
    ) -> CommandResult<EntitiesEdited> {
        crate::call::recorded(
            cx,
            kentos_contracts::CAD_CURVES_EDIT,
            kentos_contracts::CAD_CURVES_EDIT_VERSION,
            input,
            |cx, input| {
                let checked = check(cx.doc, &input);
                execute_with(cx, checked)
            },
        )
    }
}
