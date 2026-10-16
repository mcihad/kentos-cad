//! `cad.dimensions.query` v1 (docs/adr/0321 §3), read only: each dimension's measured value (the core's layout of its
//! geometry) apart from what it shows (`Format::dimension_value`: its options, and a written value with `<>` the
//! measured one), whether what it shows is an override, and what its points follow. A written value without `<>` is
//! not the geometry's measure: a program reading a drawing's sizes reads `measured`. The web is paused
//! (docs/adr/0228): the desktop's handler alone, passing `fixtures/commands/v1/cad.dimensions.query.json`.

use kentos_contracts::{
    CommandResult, DimensionBond, DimensionEntity, DimensionRead, DimensionStyle, DimensionsQuery, DimensionsRead,
    Entity,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::entity::dimension_geom;
use kentos_geometry_core::geom::dimension::layout_dimension;
use kentos_project::format::Format;

use crate::ExecutionContext;
use crate::blocks::core_entity;
use crate::checks::{Stop, error, is_uid_text};
pub use crate::codes;

fn fail(code: &str, message: String, path: String) -> Stop {
    Stop::failed(error(code, message, Some(path)))
}

/// The kind's name on the wire.
fn kind_name(s: Option<DimensionStyle>) -> &'static str {
    match s {
        None | Some(DimensionStyle::Aligned) => "aligned",
        Some(DimensionStyle::Linear) => "linear",
        Some(DimensionStyle::Angular) => "angular",
        Some(DimensionStyle::Radius) => "radius",
        Some(DimensionStyle::Diameter) => "diameter",
        Some(DimensionStyle::Ordinate) => "ordinate",
        Some(DimensionStyle::ArcLength) => "arcLength",
        Some(DimensionStyle::Jogged) => "jogged",
        Some(DimensionStyle::Azimuth) => "azimuth",
        Some(DimensionStyle::Slope) => "slope",
    }
}

/// What a dimension's points follow: the points it measures (the first two, the third when it has one; a coordinate
/// its first) against those with a reference.
fn bond(d: &DimensionEntity) -> DimensionBond {
    let Some(refs) = &d.refs else {
        return DimensionBond::Free;
    };
    if refs.each().any(|(_, r)| !r.live()) {
        return DimensionBond::Broken;
    }
    let bound = refs.each().count();
    let needed = if refs.rule.is_some() {
        bound
    } else {
        match d.style {
            Some(DimensionStyle::Ordinate) => 1,
            _ if d.c.is_some() && d.style == Some(DimensionStyle::Angular) => 3,
            _ => 2,
        }
    };
    if bound >= needed {
        DimensionBond::Bound
    } else {
        DimensionBond::Partial
    }
}

/// One dimension as read.
pub fn read(format: &Format, uid: Uuid, d: &DimensionEntity) -> DimensionRead {
    let e = core_entity(&Entity::Dimension(d.clone()));
    let (measured, unit, prefix) = dimension_geom(&e.shape)
        .and_then(|g| layout_dimension(&g))
        .map_or((f64::NAN, "length", ""), |l| (l.value, l.unit, l.prefix));
    let text = d.text.as_deref().filter(|t| !t.is_empty());
    DimensionRead {
        uid: uid.to_string(),
        kind: kind_name(d.style).into(),
        measured,
        unit: unit.into(),
        shown: format.dimension_value(prefix, unit, measured, &d.look, text),
        overridden: text.is_some_and(|t| !t.contains("<>")),
        bond: bond(d),
    }
}

fn check(doc: &Document, input: &DimensionsQuery) -> Result<Vec<(Uuid, DimensionEntity)>, Stop> {
    let Some(uids) = &input.uids else {
        return Ok(doc
            .entities()
            .filter_map(|e| match e {
                Entity::Dimension(d) => Some((doc.uid(Slot(d.base.id))?, d.clone())),
                _ => None,
            })
            .collect());
    };
    for (i, u) in uids.iter().enumerate() {
        if !is_uid_text(u) {
            return Err(fail(
                codes::INVALID_UID,
                format!(
                    "“{u}” geçerli bir nesne kimliği değil; kimlik küçük harfli, tireli bir UUID'dir (01925f3e-7c1a-7d2b-9e4f-0a1b2c3d4e5f gibi)."
                ),
                format!("uids[{i}]"),
            ));
        }
    }
    let mut out = Vec::with_capacity(uids.len());
    for (i, u) in uids.iter().enumerate() {
        let found = Uuid::parse_str(u).ok().and_then(|id| Some((id, doc.slot_of(id)?)));
        let Some((id, slot)) = found else {
            return Err(fail(
                codes::ENTITY_NOT_FOUND,
                format!(
                    "“{u}” kimlikli nesne çizimde yok: silinmiş ya da başka bir çizimin olabilir. Var olan bir nesnenin kimliğini verin."
                ),
                format!("uids[{i}]"),
            ));
        };
        match doc.get(slot) {
            Some(Entity::Dimension(d)) => out.push((id, d.clone())),
            _ => {
                return Err(fail(
                    codes::NOT_A_DIMENSION,
                    "Verilen nesne bir ölçü değil; yalnız ölçüler okunur.".into(),
                    format!("uids[{i}]"),
                ));
            }
        }
    }
    Ok(out)
}

fn answer(doc: &Document, input: &DimensionsQuery) -> CommandResult<DimensionsRead> {
    match check(doc, input) {
        Ok(list) => {
            let format = Format::of(doc.settings());
            CommandResult::Completed {
                output: DimensionsRead {
                    dimensions: list.iter().map(|(u, d)| read(&format, *u, d)).collect(),
                    revision: doc.revision().to_string(),
                },
                warnings: Vec::new(),
            }
        }
        Err(stop) => stop.into(),
    }
}

pub fn validate(cx: &ExecutionContext<'_>, input: &DimensionsQuery) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// The answer, as `execute` gives it: the query writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &DimensionsQuery) -> CommandResult<DimensionsRead> {
    answer(cx.doc, input)
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: DimensionsQuery) -> CommandResult<DimensionsRead> {
    crate::call::recorded(
        cx,
        kentos_contracts::CAD_DIMENSIONS_QUERY,
        kentos_contracts::CAD_DIMENSIONS_QUERY_VERSION,
        input,
        |cx, input| answer(cx.doc, &input),
    )
}
