//! `cad.entities.transform` v1 (docs/adr/0037): objects named by their
//! persistent ids moved, rotated, scaled or mirrored as one undo step, in
//! place or as copies. The desktop's handler over the native document; the
//! web's is `apps/web/src/product/entitiesTransform.ts`. Both pass the shared
//! cases in `fixtures/commands/v1/cad.entities.transform.json`.
//!
//! The modify tools (Taşı, Kopyala, Döndür, Ölçekle, Aynala) make the
//! selection explicit here (TODOS.md CMD-07): they give the selected
//! objects' ids. The geometry is the shared core's, as on the web: the
//! matrix of the transform (`similarity`) and what it does to each kind of
//! object (`transform_shape`); nothing is computed here.
//!
//! The checks, in order (the first that fails answers):
//! 1. at least one id; every id lowercase UUID text with hyphens (in order);
//! 2. the transform's numbers finite, in their order; a scale factor above
//!    zero; a mirror axis with a direction;
//! 3. the expected revision (every command's, `checks.rs`);
//! 4. every id names an object of the document (in order);
//! 5. not every object on a locked layer (with others, a warning);
//! 6. no coordinate carried past the largest float64 by the transform.
//!
//! Objects on a locked layer are neither changed nor copied. The web's tools
//! used to copy them onto their locked layer; ADR 0037 records the change.

use kentos_contracts::{
    CommandError, CommandResult, CommandWarning, EntitiesTransform, EntitiesTransformPlan,
    EntitiesTransformed, Entity, Transform,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::affine::{Affine, similarity};
use kentos_geometry_core::ops::transform::transform_shape;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
/// The stable codes of the answers (`CommandError.code`, `CommandWarning.code`).
pub use crate::codes;
use crate::geometry::{shape, with_shape};

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesTransform) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: (),
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// What execute would write now: each object as it would be, and the
/// revision to expect for exactly that; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &EntitiesTransform,
) -> CommandResult<EntitiesTransformPlan> {
    let copy = input.copy.unwrap_or(false);
    match check(cx.doc, input) {
        Ok(checked) => CommandResult::Completed {
            output: EntitiesTransformPlan {
                sources: checked.sources.iter().map(|(_, uid)| uid.clone()).collect(),
                entities: checked
                    .moved
                    .into_iter()
                    .map(|mut e| {
                        // A copy's slot is given when it is written.
                        if copy {
                            base_mut(&mut e).id = 0;
                        }
                        e
                    })
                    .collect(),
                locked: checked.locked,
                revision: cx.doc.revision().to_string(),
            },
            warnings: checked.warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes it as one undo step named after the tool:
/// the objects changed in place, or their copies added. Into the open
/// transaction or group, if one is.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesTransform,
) -> CommandResult<EntitiesTransformed> {
    let copy = input.copy.unwrap_or(false);
    let checked = match check(cx.doc, &input) {
        Ok(checked) => checked,
        Err(stop) => return stop.into(),
    };
    let label = label(&input.transform, copy);
    let (changed, created) = if copy {
        match cx.doc.add_many(checked.moved, label) {
            Ok(slots) => {
                let doc = &*cx.doc;
                let created = slots
                    .iter()
                    .filter_map(|slot| doc.uid(*slot))
                    .map(|uid| uid.to_string())
                    .collect();
                (Vec::new(), created)
            }
            Err(full) => {
                return CommandResult::Failed {
                    error: CommandError {
                        code: codes::SLOTS_EXHAUSTED.into(),
                        message: full.to_string(),
                        path: None,
                        revision: None,
                    },
                };
            }
        }
    } else {
        let changes: Vec<(Slot, Entity)> = checked
            .sources
            .iter()
            .map(|(slot, _)| *slot)
            .zip(checked.moved)
            .collect();
        cx.doc.update_many(changes, label);
        let changed = checked.sources.into_iter().map(|(_, uid)| uid).collect();
        (changed, Vec::new())
    };
    CommandResult::Completed {
        output: EntitiesTransformed {
            changed,
            created,
            locked: checked.locked,
            revision: cx.doc.revision().to_string(),
        },
        warnings: checked.warnings,
    }
}

/// The undo step's name: the tool's (docs/adr/0037).
pub fn label(transform: &Transform, copy: bool) -> &'static str {
    match transform {
        Transform::Move { .. } if copy => "Kopyala",
        Transform::Move { .. } => "Taşı",
        Transform::Rotate { .. } => "Döndür",
        Transform::Scale { .. } => "Ölçekle",
        Transform::Mirror { .. } => "Aynala",
    }
}

/// The affine of a transform, built by the shared core (`similarity`), as
/// the web's handler builds it through WASM. None only for numbers the
/// checks would refuse first.
pub fn affine(transform: &Transform) -> Option<Affine> {
    match *transform {
        Transform::Move { dx, dy } => similarity("move", &[dx, dy]),
        Transform::Rotate { center, angle } => similarity("rotate", &[center.x, center.y, angle]),
        Transform::Scale { center, factor } => similarity("scale", &[center.x, center.y, factor]),
        Transform::Mirror { a, b } => similarity("mirror", &[a.x, a.y, b.x, b.y]),
    }
}

/// What may be written: the objects to transform (slot and id) and each as
/// it would be, those that stay on locked layers, and the warning about them.
struct Checked {
    sources: Vec<(Slot, String)>,
    moved: Vec<Entity>,
    locked: Vec<String>,
    warnings: Vec<CommandWarning>,
}

fn locked_message(n: usize) -> String {
    format!(
        "{n} nesne kilitli katmanda olduğu için atlandı. Değiştirmek için katmanın kilidini Katmanlar panelinden açın."
    )
}

/// The transform's own checks, in its fields' order: finite numbers, then
/// what they mean. Its affine when they pass.
fn check_transform(transform: &Transform) -> Result<Affine, Stop> {
    let finite =
        |value: f64, what: &str, fix: &str, path: &str| checks::finite(value, what, fix, path);
    match *transform {
        Transform::Move { dx, dy } => {
            let fix = "Kaydırmayı sonlu bir sayıyla verin.";
            finite(dx, "Doğu (Y) yönündeki kaydırma", fix, "transform.dx")?;
            finite(dy, "Kuzey (X) yönündeki kaydırma", fix, "transform.dy")?;
        }
        Transform::Rotate { center, angle } => {
            checks::point(center, "Merkezin", "transform.center")?;
            finite(
                angle,
                "Dönme açısı",
                "Açıyı sonlu bir sayıyla verin.",
                "transform.angle",
            )?;
        }
        Transform::Scale { center, factor } => {
            checks::point(center, "Merkezin", "transform.center")?;
            finite(
                factor,
                "Ölçek faktörü",
                "Faktörü sonlu bir sayıyla verin.",
                "transform.factor",
            )?;
            if factor <= 0.0 {
                return Err(Stop::Failed(checks::error(
                    codes::INVALID_FACTOR,
                    "Ölçek faktörü sıfırdan büyük olmalı. Pozitif bir faktör verin.".into(),
                    Some("transform.factor".into()),
                )));
            }
        }
        Transform::Mirror { a, b } => {
            checks::point(a, "Eksenin ilk noktasının", "transform.a")?;
            checks::point(b, "Eksenin ikinci noktasının", "transform.b")?;
            // The core's own measure of the axis (`mirror`): zero leaves it no direction.
            let (dx, dy) = (b.x - a.x, b.y - a.y);
            if dx * dx + dy * dy == 0.0 {
                return Err(Stop::Failed(checks::error(
                    codes::INVALID_AXIS,
                    "Simetri ekseninin iki noktası aynı; eksenin yönü yok. Birbirinden ayrı iki nokta verin.".into(),
                    Some("transform.b".into()),
                )));
            }
        }
    }
    affine(transform).ok_or_else(|| {
        Stop::Failed(checks::error(
            codes::NOT_FINITE,
            "Dönüşüm kurulamadı.".into(),
            Some("transform".into()),
        ))
    })
}

/// The checks in the contract's order.
fn check(doc: &Document, input: &EntitiesTransform) -> Result<Checked, Stop> {
    checks::uids(&input.uids, "Dönüştürülecek nesne verilmedi.")?;
    let m = check_transform(&input.transform)?;
    checks::revision(doc, input.expected_revision.as_deref())?;
    let mut sources = Vec::new();
    let mut moved = Vec::new();
    let mut locked = Vec::new();
    let mut overflow = false;
    for (slot, entity, uid) in checks::objects(doc, &input.uids)? {
        if doc.layers().is_locked(&entity.base().layer_id) {
            locked.push(uid.clone());
            continue;
        }
        let before = shape(entity);
        let after = transform_shape(&before, &m);
        overflow |= finite_shape(&before) && !finite_shape(&after);
        // The core keeps an object's kind; a shape of another kind would be its fault, not the input's.
        let Some(e) = with_shape(entity, after) else {
            return Err(Stop::Failed(checks::error(
                codes::NOT_FINITE,
                "Nesne dönüştürülemedi.".into(),
                Some("transform".into()),
            )));
        };
        sources.push((slot, uid.clone()));
        moved.push(e);
    }
    if sources.is_empty() {
        return Err(Stop::Failed(checks::error(
            codes::LAYER_LOCKED,
            locked_message(locked.len()),
            Some("uids".into()),
        )));
    }
    if overflow {
        return Err(Stop::Failed(checks::error(
            codes::NOT_FINITE,
            "Dönüşüm sonucunda sonlu olmayan bir değer çıktı (sayı taşması). Daha küçük bir değer verin.".into(),
            Some("transform".into()),
        )));
    }
    let mut warnings = Vec::new();
    if !locked.is_empty() {
        warnings.push(CommandWarning {
            code: codes::LAYER_LOCKED.into(),
            message: locked_message(locked.len()),
            path: Some("uids".into()),
        });
    }
    Ok(Checked {
        sources,
        moved,
        locked,
        warnings,
    })
}

/// Every number of a shape's geometry is finite (the fields the web's
/// handler walks: points, radii, angles, bulges, heights, offsets, the
/// hatch pattern's angle and spacing).
fn finite_shape(s: &Shape) -> bool {
    use kentos_geometry_core::Vec2;
    fn pt(p: &Vec2) -> bool {
        p.x.is_finite() && p.y.is_finite()
    }
    fn pts(ps: &[Vec2]) -> bool {
        ps.iter().all(pt)
    }
    fn values(vs: &Option<Vec<f64>>) -> bool {
        vs.iter().flatten().all(|v| v.is_finite())
    }
    match s {
        Shape::Point { p, z } => pt(p) && z.is_none_or(f64::is_finite),
        Shape::Line { a, b } => pt(a) && pt(b),
        Shape::Polyline {
            pts: p,
            bulges,
            holes,
        }
        | Shape::Polygon {
            pts: p,
            bulges,
            holes,
        } => {
            pts(p)
                && values(bulges)
                && holes
                    .iter()
                    .flatten()
                    .all(|h| pts(&h.pts) && values(&h.bulges))
        }
        Shape::Circle { c, r } => pt(c) && r.is_finite(),
        Shape::Arc { c, r, a0, a1 } => pt(c) && r.is_finite() && a0.is_finite() && a1.is_finite(),
        Shape::Ellipse {
            c,
            major,
            ratio,
            t0,
            t1,
        } => pt(c) && pt(major) && ratio.is_finite() && t0.is_finite() && t1.is_finite(),
        Shape::Xline { p, dir } | Shape::Ray { p, dir } => pt(p) && pt(dir),
        Shape::Spline { pts: p, .. } => pts(p),
        Shape::Text {
            p,
            height,
            rotation,
            ..
        } => pt(p) && height.is_finite() && rotation.is_finite(),
        Shape::Dimension {
            a,
            b,
            offset,
            height,
            angle,
            c,
            ..
        } => {
            pt(a)
                && pt(b)
                && offset.is_finite()
                && height.is_finite()
                && angle.is_none_or(f64::is_finite)
                && c.as_ref().is_none_or(pt)
        }
        Shape::Hatch {
            ring,
            holes,
            pattern,
        } => {
            pts(ring)
                && holes.iter().flatten().all(|h| pts(h))
                && pattern.angle.is_finite()
                && pattern.spacing.is_finite()
        }
    }
}

fn base_mut(e: &mut Entity) -> &mut kentos_contracts::EntityBase {
    match e {
        Entity::Point(e) => &mut e.base,
        Entity::Line(e) => &mut e.base,
        Entity::Polyline(e) | Entity::Polygon(e) => &mut e.base,
        Entity::Circle(e) => &mut e.base,
        Entity::Arc(e) => &mut e.base,
        Entity::Ellipse(e) => &mut e.base,
        Entity::Spline(e) => &mut e.base,
        Entity::Xline(e) | Entity::Ray(e) => &mut e.base,
        Entity::Text(e) => &mut e.base,
        Entity::Dimension(e) => &mut e.base,
        Entity::Hatch(e) => &mut e.base,
    }
}
