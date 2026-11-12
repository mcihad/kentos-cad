//! `cad.entities.divide` v1 (docs/adr/0254 §5, 0299): points or a block's
//! inserts along an object at equal parts or at a step, the step's first
//! place and the object's own ends as asked, as one undo step “Böl”. The desktop's handler
//! over the native document, run by the Böl tool (its preview is
//! [`plan_in`], its Enter [`execute`]), the headless server, Python and MCP.
//! The shared cases are `fixtures/commands/v1/cad.entities.divide.json`.
//!
//! Where the places fall and the object's direction there are the shared
//! core's (`division_places`); the points or inserts are written through
//! `cad.entities.create`'s `divide` operation ([`crate::creating`]).

use kentos_commands::{
    CommandResult, CreateOperation, DivideEnds, Division, EntitiesCreatePlan, EntitiesCreated,
    EntitiesDivide, Entity, EntityGeometry, MAX_DIVISION_POINTS, Vec2 as Point,
};
use kentos_domain::{Document, Uuid};
use kentos_geometry_core::ops::path::{
    Division as CoreDivision, DivisionEnds, DivisionPlace, division_places,
};

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

/// The places of `e` at `division`, from its start or its end, with the
/// step's first place and the ends as asked (the core's).
pub fn places_of(
    e: &Entity,
    division: Division,
    from_end: bool,
    start: Option<f64>,
    ends: DivideEnds,
) -> Vec<DivisionPlace> {
    let by = match division {
        Division::Parts { count } => CoreDivision::Parts(f64::from(count)),
        Division::Step { length } => CoreDivision::Step(length),
    };
    let ends = match ends {
        DivideEnds::None => DivisionEnds::None,
        DivideEnds::Start => DivisionEnds::Start,
        DivideEnds::End => DivisionEnds::End,
        DivideEnds::Both => DivisionEnds::Both,
    };
    division_places(&shape(e), &by, from_end, start, ends)
}

/// The points of `e` at `division`, from its start or its end (the core's).
pub fn points_of(e: &Entity, division: Division, from_end: bool) -> Vec<Point> {
    places_of(e, division, from_end, None, DivideEnds::None)
        .into_iter()
        .map(|p| Point {
            x: p.at.x,
            y: p.at.y,
        })
        .collect()
}

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The checks, in the contract's order; the objects to create.
fn check(doc: &Document, input: &EntitiesDivide) -> Result<kentos_commands::EntitiesCreate, Stop> {
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
    if let Some(start) = input.start {
        checks::finite(start, "Başlangıç", "Sonlu bir uzunluk verin.", "start")?;
        if start < 0.0 {
            return Err(refuse(
                codes::INVALID_DISTANCE,
                "Başlangıç sıfır ya da sıfırdan büyük sonlu bir uzunluk (metre) olmalı.".into(),
                "start",
            ));
        }
        if matches!(input.division, Division::Parts { .. }) {
            return Err(refuse(
                codes::START_NEEDS_STEP,
                "Başlangıç yalnız aralıkla bölmede verilir; parça sayısıyla bölmede kaldırın."
                    .into(),
                "start",
            ));
        }
    }
    if let Some(block) = &input.block {
        checks::finite(
            block.scale,
            "Bloğun ölçeği",
            "Sonlu bir sayı verin.",
            "block.scale",
        )?;
        if block.scale <= 0.0 {
            return Err(refuse(
                codes::INVALID_SCALE,
                "Bloğun ölçeği sıfırdan büyük olmalı.".into(),
                "block.scale",
            ));
        }
        checks::finite(
            block.rotation,
            "Bloğun dönüşü",
            "Sonlu bir açı verin.",
            "block.rotation",
        )?;
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
    let block = match &input.block {
        Some(b) => {
            let Some(def) = doc.block_named(&b.name) else {
                return Err(refuse(
                    codes::UNKNOWN_BLOCK,
                    format!(
                        "“{}” adlı blok çizimde tanımlı değil. Blok oluştur ile tanımlayın ya da var olan bir bloğun adını verin.",
                        b.name
                    ),
                    "block.name",
                ));
            };
            Some((def.id, b))
        }
        None => None,
    };
    let places = places_of(e, input.division, input.from_end, input.start, input.ends);
    if places.is_empty() {
        return Err(if input.start.is_some() {
            refuse(
                codes::NO_POINTS,
                "Başlangıç nesnenin boyundan uzun; nokta konmadı. Daha kısa bir başlangıç verin."
                    .into(),
                "start",
            )
        } else {
            refuse(
                codes::NO_POINTS,
                "Aralık nesnenin boyundan uzun; nokta konmadı. Daha kısa bir aralık verin.".into(),
                "division.length",
            )
        });
    }
    if places.len() > MAX_DIVISION_POINTS as usize {
        return Err(refuse(
            codes::TOO_MANY_POINTS,
            "10 000'den çok nokta olurdu; daha büyük bir aralık verin.".into(),
            "division",
        ));
    }
    let geometries = places
        .into_iter()
        .map(|place| {
            let p = Point {
                x: place.at.x,
                y: place.at.y,
            };
            match block {
                Some((id, b)) => EntityGeometry::Insert {
                    block: id,
                    p,
                    scale: b.scale,
                    y_scale: None,
                    rotation: b.rotation + if b.align { place.direction } else { 0.0 },
                    mirror: false,
                },
                None => EntityGeometry::Point {
                    p,
                    z: None,
                    parts: None,
                },
            }
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
        kentos_commands::CAD_ENTITIES_DIVIDE,
        kentos_commands::CAD_ENTITIES_DIVIDE_VERSION,
        input,
        |cx, input| match check(cx.doc, &input) {
            Ok(objects) => creating::execute(cx, objects),
            Err(stop) => stop.into(),
        },
    )
}
