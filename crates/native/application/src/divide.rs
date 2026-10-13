//! `cad.entities.divide` v1 (docs/adr/0254 §5): points along an object at
//! equal parts or at a step, as one undo step “Böl”. The desktop's handler
//! over the native document, run by the Böl tool (its preview is
//! [`plan_in`], its Enter [`execute`]), the headless server, Python and MCP.
//! The shared cases are `fixtures/commands/v1/cad.entities.divide.json`.
//!
//! Where the points fall is the shared core's (`division_points`); they are
//! written through `cad.entities.create`'s `divide` operation
//! ([`crate::creating`]).

use kentos_contracts::{
    CommandResult, CreateOperation, Division, EntitiesCreatePlan, EntitiesCreated, EntitiesDivide,
    Entity, EntityGeometry, MAX_DIVISION_POINTS, Vec2 as Point,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::ops::path::{Division as CoreDivision, division_points};

use crate::ExecutionContext;
use crate::checks::{self, Stop, error, is_uid_text};
use crate::codes;
use crate::creating;
use crate::geometry::shape;

/// Whether Böl divides an object of this kind.
pub fn divisible(e: &Entity) -> bool {
    matches!(
        e,
        Entity::Line(_)
            | Entity::Polyline(_)
            | Entity::Polygon(_)
            | Entity::Arc(_)
            | Entity::Circle(_)
            | Entity::Ellipse(_)
            | Entity::Spline(_)
            | Entity::Nurbs(_)
    )
}

/// The points of `e` at `division`, from its start or its end (the core's).
pub fn points_of(e: &Entity, division: Division, from_end: bool) -> Vec<Point> {
    let by = match division {
        Division::Parts { count } => CoreDivision::Parts(f64::from(count)),
        Division::Step { length } => CoreDivision::Step(length),
    };
    division_points(&shape(e), &by, from_end)
        .into_iter()
        .map(|p| Point { x: p.x, y: p.y })
        .collect()
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The checks, in the contract's order; the objects to create.
fn check(doc: &Document, input: &EntitiesDivide) -> Result<kentos_contracts::EntitiesCreate, Stop> {
    if !is_uid_text(&input.uid) {
        return Err(refuse(
            codes::INVALID_UID,
            format!(
                "“{}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi).",
                input.uid
            ),
            "uid",
        ));
    }
    match input.division {
        Division::Parts { count } if !(2..=MAX_DIVISION_POINTS).contains(&count) => {
            return Err(refuse(
                codes::INVALID_COUNT,
                "Parça sayısı 2 ile 10 000 arasında bir tam sayı olmalı.".into(),
                "division.count",
            ));
        }
        Division::Step { length } if !(length.is_finite() && length > 0.0) => {
            return Err(refuse(
                codes::INVALID_DISTANCE,
                "Aralık sıfırdan büyük sonlu bir uzunluk (metre) olmalı.".into(),
                "division.length",
            ));
        }
        _ => {}
    }
    checks::revision(doc, input.expected_revision.as_deref())?;
    let Some(e) = Uuid::parse_str(&input.uid)
        .ok()
        .and_then(|u| doc.slot_of(u))
        .and_then(|s| doc.get(s))
    else {
        return Err(refuse(
            codes::ENTITY_NOT_FOUND,
            format!(
                "“{}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin.",
                input.uid
            ),
            "uid",
        ));
    };
    if !divisible(e) {
        return Err(refuse(
            codes::NOT_DIVISIBLE,
            "Bu türdeki nesne bölünmez: çizgi, çoklu çizgi, alan, yay, daire, elips ya da eğri olmalı.".into(),
            "uid",
        ));
    }
    let points = points_of(e, input.division, input.from_end);
    if points.is_empty() {
        return Err(refuse(
            codes::NO_POINTS,
            "Aralık nesnenin boyundan uzun; nokta konmadı. Daha kısa bir aralık verin.".into(),
            "division.length",
        ));
    }
    if points.len() > MAX_DIVISION_POINTS as usize {
        return Err(refuse(
            codes::TOO_MANY_POINTS,
            "10 000'den çok nokta olurdu; daha büyük bir aralık verin.".into(),
            "division",
        ));
    }
    let geometries = points
        .into_iter()
        .map(|p| EntityGeometry::Point {
            p,
            z: None,
            parts: None,
        })
        .collect();
    Ok(creating::input(
        &input.layer_id,
        geometries,
        &input.look,
        CreateOperation::Divide,
    ))
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesDivide) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(objects) => creating::validate(cx, &objects),
        Err(stop) => stop.into(),
    }
}

/// What execute would write now; writes nothing.
pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &EntitiesDivide,
) -> CommandResult<EntitiesCreatePlan> {
    plan_in(cx.doc, input)
}

/// [`plan`] over the document (the tool's preview).
pub fn plan_in(doc: &Document, input: &EntitiesDivide) -> CommandResult<EntitiesCreatePlan> {
    match check(doc, input) {
        Ok(objects) => creating::plan(doc, &objects),
        Err(stop) => stop.into(),
    }
}

/// Checks `input` again and writes the points as one undo step.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesDivide,
) -> CommandResult<EntitiesCreated> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_ENTITIES_DIVIDE,
        kentos_contracts::CAD_ENTITIES_DIVIDE_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(objects) => creating::execute(cx, objects),
            Err(stop) => stop.into(),
        },
    )
}
