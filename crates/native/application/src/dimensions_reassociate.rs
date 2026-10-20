//! `cad.dimensions.reassociate` v1 (docs/adr/0249 §3): a dimension's points
//! bound to places on objects, or freed, as one undo step “Ölçüyü yeniden
//! bağla”. A bound point is read from its place on the object now by the
//! core's rule (`ops::subref::point`); from then on the document keeps it
//! with its object before every step is recorded (`kentos_domain`'s
//! `dimension_ties`). The web is paused (docs/adr/0228): the desktop's
//! handler alone, passing `fixtures/commands/v1/cad.dimensions.reassociate.json`.
//!
//! The checks, in the contract's order: the ids, at least one binding, each
//! point once and each place by its rules; the expected revision; the
//! dimension and each place's object are the drawing's; the dimension is
//! one, on an unlocked layer, measuring the point; no place on itself; each
//! place one its object has.

use kentos_commands::{
    CommandResult, DimensionEntity, DimensionPoint, DimensionsReassociate,
    DimensionsReassociatePlan, DimensionsReassociated, Entity, EntityId, SubRef, Vec2,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::dimension_geom;
use kentos_geometry_core::geom::dimension::layout_dimension;

use crate::ExecutionContext;
use crate::blocks::core_entity;
use crate::checks::{self, Stop, error, is_uid_text};
/// The stable codes of the answers (`CommandError.code`).
pub use crate::codes;

/// The undo step's name.
pub const LABEL: &str = "Ölçüyü yeniden bağla";

fn fail(code: &str, message: String, path: String) -> Stop {
    Stop::failed(error(code, message, Some(path)))
}

/// What the checks give: the dimension's slot, it as it would be, and how many points change.
struct Checked {
    slot: Slot,
    dimension: DimensionEntity,
    changed: u32,
}

fn found(doc: &Document, uid: &str) -> Option<(Slot, Uuid)> {
    let u = Uuid::parse_str(uid).ok()?;
    Some((doc.slot_of(u)?, u))
}

fn check(doc: &Document, input: &DimensionsReassociate) -> Result<Checked, Stop> {
    let bad_uid = |uid: &str, path: String| {
        fail(
            codes::INVALID_UID,
            format!(
                "“{uid}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
            ),
            path,
        )
    };
    if !is_uid_text(&input.uid) {
        return Err(bad_uid(&input.uid, "uid".into()));
    }
    for (i, b) in input.points.iter().enumerate() {
        if let Some(to) = &b.to
            && !is_uid_text(&to.uid)
        {
            return Err(bad_uid(&to.uid, format!("points[{i}].to.uid")));
        }
    }
    if input.points.is_empty() {
        return Err(fail(
            codes::NO_ENTITIES,
            "Bağlanacak nokta yok. Ölçünün en az bir noktasını (a, b ya da c) verin.".into(),
            "points".into(),
        ));
    }
    for (i, b) in input.points.iter().enumerate() {
        if input.points[..i].iter().any(|o| o.point == b.point) {
            return Err(fail(
                codes::INVALID_REFS,
                format!(
                    "Ölçünün “{}” noktası iki kez veriliyor; her noktayı bir kez verin.",
                    b.point.name()
                ),
                format!("points[{i}].point"),
            ));
        }
        if let Some(to) = &b.to
            && let Some(problem) = to.place.problem()
        {
            return Err(fail(
                codes::INVALID_REFS,
                problem,
                format!("points[{i}].to.place"),
            ));
        }
        if let Some(to) = &b.to
            && let Some(problem) = kentos_commands::path_problem(&to.path)
        {
            return Err(fail(
                codes::INVALID_REFS,
                problem,
                format!("points[{i}].to.path"),
            ));
        }
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let Some((slot, _)) = found(doc, &input.uid) else {
        return Err(fail(
            codes::ENTITY_NOT_FOUND,
            format!(
                "“{}” kimlikli ölçü çizimde yok: silinmiş ya da başka bir çizimin olabilir.",
                input.uid
            ),
            "uid".into(),
        ));
    };
    let mut objects = Vec::with_capacity(input.points.len());
    for (i, b) in input.points.iter().enumerate() {
        let Some(to) = &b.to else {
            objects.push(None);
            continue;
        };
        let Some((at, u)) = found(doc, &to.uid) else {
            return Err(fail(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                    to.uid
                ),
                format!("points[{i}].to.uid"),
            ));
        };
        objects.push(Some((at, u)));
    }
    let Some(Entity::Dimension(d)) = doc.get(slot) else {
        return Err(fail(
            codes::NOT_A_DIMENSION,
            "Verilen nesne bir ölçü değil; Yeniden bağla yalnız ölçülerin noktalarını bağlar."
                .into(),
            "uid".into(),
        ));
    };
    let layers = doc.layers();
    let layer = &d.base.layer_id;
    if layers.is_locked(layer) {
        let name = layers
            .get(layer)
            .map_or(layer.as_str(), |n| n.name.as_str());
        return Err(fail(
            codes::LAYER_LOCKED,
            format!(
                "“{name}” katmanı kilitli; üzerindeki ölçü değişmez. Kilidi Katmanlar panelinden açın."
            ),
            "uid".into(),
        ));
    }
    let mut next = d.clone();
    let mut refs = d.refs.clone().unwrap_or_default();
    for (i, (b, object)) in input.points.iter().zip(&objects).enumerate() {
        if b.point == DimensionPoint::C && d.c.is_none() {
            return Err(fail(
                codes::INVALID_REFS,
                "Bu ölçünün üçüncü noktası (c) yok; yalnız a ve b bağlanır.".into(),
                format!("points[{i}].point"),
            ));
        }
        let held = match b.point {
            DimensionPoint::A => &mut refs.a,
            DimensionPoint::B => &mut refs.b,
            DimensionPoint::C => &mut refs.c,
        };
        let (Some(to), Some((at, u))) = (&b.to, object) else {
            *held = None;
            continue;
        };
        if *at == slot {
            return Err(fail(
                codes::INVALID_REFS,
                "Ölçünün noktası ölçünün kendisine bağlanamaz; ölçülen nesnenin bir yerini verin."
                    .into(),
                format!("points[{i}].to.uid"),
            ));
        }
        let place = doc
            .get(*at)
            .map(|e| crate::blocks::place_point(doc.blocks(), e, &to.path, to.place));
        let p = match place {
            Some(Ok(p)) => p,
            Some(Err(crate::blocks::PlaceMiss::Piece)) => {
                return Err(fail(
                    codes::PIECE_GONE,
                    crate::blocks::NO_PIECE.into(),
                    format!("points[{i}].to.path"),
                ));
            }
            _ => {
                return Err(fail(
                    codes::NO_SUCH_PLACE,
                    "Nesnede böyle bir yer yok: köşe sırası halkanın dışında ya da yer bu nesne türünde bulunmuyor.".into(),
                    format!("points[{i}].to.place"),
                ));
            }
        };
        let p = Vec2 { x: p.x, y: p.y };
        match b.point {
            DimensionPoint::A => next.a = p,
            DimensionPoint::B => next.b = p,
            DimensionPoint::C => next.c = Some(p),
        }
        *held = Some(SubRef {
            uid: EntityId(*u.as_bytes()),
            place: to.place,
            path: to.path.clone(),
            broken: None,
        });
    }
    next.refs = (!refs.is_empty()).then_some(refs);
    let changed = ["a", "b", "c"]
        .into_iter()
        .filter(|k| point_and_ref(d, k) != point_and_ref(&next, k))
        .count() as u32;
    Ok(Checked {
        slot,
        dimension: next,
        changed,
    })
}

fn point_and_ref<'a>(d: &'a DimensionEntity, name: &str) -> (Option<Vec2>, Option<&'a SubRef>) {
    let r = d.refs.as_ref();
    match name {
        "a" => (Some(d.a), r.and_then(|r| r.a.as_ref())),
        "b" => (Some(d.b), r.and_then(|r| r.b.as_ref())),
        _ => (d.c, r.and_then(|r| r.c.as_ref())),
    }
}

/// The dimension's measured value and its unit, as the core lays it out.
fn measured(d: &DimensionEntity) -> (f64, String) {
    let e = core_entity(&Entity::Dimension(d.clone()));
    dimension_geom(&e.shape)
        .and_then(|g| layout_dimension(&g))
        .map_or((f64::NAN, "length".into()), |l| (l.value, l.unit.into()))
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &DimensionsReassociate) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The dimension as it would be, with its value; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &DimensionsReassociate,
) -> CommandResult<DimensionsReassociatePlan> {
    match check(cx.doc, input) {
        Ok(c) => {
            let (value, unit) = measured(&c.dimension);
            CommandResult::Completed {
                output: DimensionsReassociatePlan {
                    dimension: Entity::Dimension(c.dimension),
                    changed: c.changed,
                    value,
                    unit,
                    revision: cx.doc.revision().to_string(),
                },
                warnings: Vec::new(),
            }
        }
        Err(stop) => stop.into(),
    }
}

/// Writes the dimension as one undo step when any point changes.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: DimensionsReassociate,
) -> CommandResult<DimensionsReassociated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_DIMENSIONS_REASSOCIATE,
        kentos_commands::CAD_DIMENSIONS_REASSOCIATE_VERSION,
        input,
        write_execute,
    )
}

/// [`execute`]'s write; `call::recorded` runs it as one step of the
/// document, all or nothing, and journals it (docs/adr/0253 §1, §3).
fn write_execute(
    cx: &mut ExecutionContext<'_>,
    input: DimensionsReassociate,
) -> CommandResult<DimensionsReassociated> {
    let c = match check(cx.doc, &input) {
        Ok(c) => c,
        Err(stop) => return stop.into(),
    };
    let (value, unit) = measured(&c.dimension);
    let doc = &mut *cx.doc;
    if c.changed > 0 {
        doc.update_many(vec![(c.slot, Entity::Dimension(c.dimension))], LABEL);
    }
    CommandResult::Completed {
        output: DimensionsReassociated {
            changed: c.changed,
            value,
            unit,
            revision: doc.revision().to_string(),
        },
        warnings: Vec::new(),
    }
}
