//! `cad.entities.adjoin` v1 (Bitişik alan, docs/adr/0162 §3, 0622 §9): only the new boundary is given, an open path;
//! the region it closes with the shown areas of the neighbour layers is the new area (`ops::adjoin::Neighbours::fill`),
//! written as `cad.polygon.create` writes one, its parts and holes as they are, with its rules (Çakışma beyond the
//! neighbours, Topoloji), one undo step “Bitişik alan”. The Bitişik alan tool previews with the same core and writes
//! through this command. The shared cases are `fixtures/commands/v1/cad.entities.adjoin.json`
//! (`scripts/fixtures/adjoin_command_cases.py`).

use kentos_commands::{CommandResult, EntitiesAdjoin, PolygonCreate, PolygonCreated, PolygonPlan};
use kentos_domain::Document;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::bulge::has_bulges;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::ops::adjoin::Neighbours;
use kentos_geometry_core::ops::areas::areas_of_entity;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::vec2::Vec2;

use crate::checks::{self, Stop, error};
use crate::geometry::shape;
use crate::{ExecutionContext, codes, polygon};

/// The undo step's name.
pub const LABEL: &str = "Bitişik alan";

/// Said when the path closes nothing.
pub const NO_REGION: &str = "Yol komşu alanlarla kapalı bir bölge oluşturmuyor: ilk ve son noktayı komşu alanların içine ya da sınırına koyun.";

fn refuse(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.into())))
}

/// The path's own checks: its points, bulges and box.
fn check_path(input: &EntitiesAdjoin) -> Result<(), Stop> {
    let n = input.pts.len();
    if n < 2 {
        return Err(refuse(
            codes::TOO_FEW_POINTS,
            format!("Bitişik alanın yolunun en az iki noktası olmalı; {n} nokta verildi."),
            "pts",
        ));
    }
    for (i, p) in input.pts.iter().enumerate() {
        checks::point(*p, &format!("{}. noktanın", i + 1), &format!("pts[{i}]"))?;
    }
    if let Some(bulges) = &input.bulges {
        if bulges.len() != n - 1 {
            return Err(refuse(
                codes::BULGE_COUNT,
                format!(
                    "Yay değerleri kenar sayısı kadar olmalı: {} kenar, {} yay değeri verildi. Her kenara bir değer verin; düz kenar için 0.",
                    n - 1,
                    bulges.len()
                ),
                "bulges",
            ));
        }
        for (i, b) in bulges.iter().enumerate() {
            if !b.is_finite() {
                return Err(refuse(
                    codes::NOT_FINITE,
                    format!(
                        "{}. kenarın yay değeri sonlu bir sayı değil (NaN ya da sonsuz). Düz kenar için 0, yay için tan(açı/4) verin.",
                        i + 1
                    ),
                    &format!("bulges[{i}]"),
                ));
            }
        }
    }
    if let Some(w) = &input.within {
        checks::point(w.min, "Kutunun güneybatı köşesinin", "within.min")?;
        checks::point(w.max, "Kutunun kuzeydoğu köşesinin", "within.max")?;
    }
    Ok(())
}

/// The neighbour layers: each a layer, not a group; none given, the area's own.
fn neighbour_layers(doc: &Document, input: &EntitiesAdjoin) -> Result<Vec<String>, Stop> {
    if input.neighbours.is_empty() {
        return Ok(vec![input.layer_id.clone()]);
    }
    for (i, id) in input.neighbours.iter().enumerate() {
        let path = format!("neighbours[{i}]");
        match doc.layers().get(id) {
            None => {
                return Err(refuse(
                    codes::LAYER_NOT_FOUND,
                    format!(
                        "Komşu “{id}” katmanı çizimde yok. Var olan bir katmanın kimliğini verin."
                    ),
                    &path,
                ));
            }
            Some(node) if node.kind != kentos_commands::LayerNodeType::Layer => {
                return Err(refuse(
                    codes::NOT_A_LAYER,
                    format!(
                        "Komşu “{}” bir grup; bölge yalnız katmanların alanlarıyla kapanır.",
                        node.name
                    ),
                    &path,
                ));
            }
            Some(_) => {}
        }
    }
    Ok(input.neighbours.clone())
}

fn slot(id: f64) -> Option<kentos_domain::Slot> {
    (id >= 0.0 && id <= f64::from(u32::MAX) && id.fract() == 0.0)
        .then_some(kentos_domain::Slot(id as u32))
}

/// The region the path closes with the shown areas of the neighbour layers.
fn region(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesAdjoin,
) -> Result<Vec<Area>, Stop> {
    check_path(input)?;
    let layers = neighbour_layers(doc, input)?;
    let own;
    let store = match store {
        Some(store) => store,
        None => {
            own = crate::geometry_store::store_of(doc);
            &own
        }
    };
    let window = match &input.within {
        Some(w) => Bounds {
            min_x: w.min.x.min(w.max.x),
            min_y: w.min.y.min(w.max.y),
            max_x: w.min.x.max(w.max.x),
            max_y: w.min.y.max(w.max.y),
        },
        None => Bounds {
            min_x: f64::NEG_INFINITY,
            min_y: f64::NEG_INFINITY,
            max_x: f64::INFINITY,
            max_y: f64::INFINITY,
        },
    };
    let sets: Vec<Vec<Area>> = store
        .in_rect(&window, true)
        .into_iter()
        .filter_map(slot)
        .filter_map(|s| doc.get(s))
        .filter(|e| layers.iter().any(|l| *l == e.base().layer_id))
        .map(|e| areas_of_entity(&shape(e)))
        .filter(|set| !set.is_empty())
        .collect();
    let pts: Vec<Vec2> = input.pts.iter().map(|p| Vec2::new(p.x, p.y)).collect();
    let bulges = input.bulges.as_deref().filter(|b| has_bulges(Some(b)));
    let region = Neighbours::new(sets).fill(&pts, bulges);
    if region.is_empty() {
        return Err(refuse(codes::NO_REGION, NO_REGION.into(), "pts"));
    }
    Ok(region)
}

/// The polygon the region is written as: its first part's rings, the input's look and rules.
fn base(input: &EntitiesAdjoin, region: &[Area]) -> PolygonCreate {
    let (pts, bulges, holes, _) = crate::area_rules::fields(region).unwrap_or_default();
    PolygonCreate {
        layer_id: input.layer_id.clone(),
        pts,
        bulges,
        holes,
        color: input.color.clone(),
        line_weight: input.line_weight,
        line_type: input.line_type,
        wipeout: None,
        attrs: input.attrs.clone(),
        label: input.label.clone(),
        symbol: input.symbol.clone(),
        rules: input.rules.clone(),
        expected_revision: input.expected_revision.clone(),
    }
}

fn ruled(
    doc: &Document,
    store: Option<&Store>,
    input: &EntitiesAdjoin,
) -> Result<polygon::Ruled, Stop> {
    let region = region(doc, store, input)?;
    polygon::ruled_areas(doc, store, &base(input, &region), region)
}

/// The region's checks and the polygon's, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &EntitiesAdjoin) -> CommandResult<()> {
    match ruled(cx.doc, cx.store, input) {
        Ok((warnings, _, _)) => CommandResult::Completed {
            output: (),
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// The new area as it would be written.
pub fn plan(cx: &ExecutionContext<'_>, input: &EntitiesAdjoin) -> CommandResult<PolygonPlan> {
    match ruled(cx.doc, cx.store, input) {
        Ok((warnings, entity, _)) => CommandResult::Completed {
            output: PolygonPlan {
                entity,
                revision: cx.doc.revision().to_string(),
            },
            warnings,
        },
        Err(stop) => stop.into(),
    }
}

/// Writes the new area (and its neighbours' corners), one undo step “Bitişik alan”.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: EntitiesAdjoin,
) -> CommandResult<PolygonCreated> {
    crate::call::recorded(
        cx,
        kentos_commands::CAD_ENTITIES_ADJOIN,
        kentos_commands::CAD_ENTITIES_ADJOIN_VERSION,
        input,
        |cx, input| {
            let ruled = match ruled(cx.doc, cx.store, &input) {
                Ok(ruled) => ruled,
                Err(stop) => return stop.into(),
            };
            let group = cx.doc.begin_group(LABEL);
            match polygon::write_ruled(cx, ruled) {
                CommandResult::Completed {
                    mut output,
                    warnings,
                } => {
                    cx.doc.end_group(group);
                    // The revision the step ends at, not the one inside it.
                    output.revision = cx.doc.revision().to_string();
                    CommandResult::Completed { output, warnings }
                }
                other => {
                    cx.doc.cancel_group(group);
                    other
                }
            }
        },
    )
}
