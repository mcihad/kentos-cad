//! `road.terrains.query` v1 (plan/road.md Y2.3, docs/adr/0900 §11): the road project's surfaces as their sources give
//! them now (state, digest, what is left, figures), or one's with its heights at places and its section along a line.
//! Reads only. The shared cases are `fixtures/commands/v1/road.terrains.query.json`.

use kentos_commands::{
    CommandResult, MAX_TERRAIN_PLACES, TerrainHeight, TerrainPlace, TerrainSectionPoint, TerrainsQueried, TerrainsQuery,
};
use kentos_road::surface::Place;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::{self as common, fail};

fn answer(doc: &kentos_domain::Document, input: &TerrainsQuery) -> Result<TerrainsQueried, Stop> {
    common::road_project(doc)?;
    let own = common::surfaces(doc);
    let places = || input.at.iter().chain(&input.along);
    if places().any(|p| !(p[0].is_finite() && p[1].is_finite())) {
        return Err(fail(codes::INVALID_PLACES, "Yerlerin koordinatları sonlu sayılar olmalı.".into(), "at"));
    }
    if input.along.len() == 1 {
        return Err(fail(codes::INVALID_PLACES, "Kesit hattı en az iki yerden oluşur.".into(), "along"));
    }
    if input.at.len() + input.along.len() > MAX_TERRAIN_PLACES {
        return Err(fail(
            codes::INVALID_PLACES,
            format!("Bir soruda en çok {MAX_TERRAIN_PLACES} yer olur."),
            "at",
        ));
    }
    let Some(name) = input.surface.as_deref() else {
        if !input.at.is_empty() || !input.along.is_empty() {
            return Err(fail(
                codes::NO_TERRAIN,
                "Kot ve kesit bir yüzeyde sorulur: yüzeyin kimliğini ya da adını (surface) verin.".into(),
                "surface",
            ));
        }
        return Ok(TerrainsQueried {
            surfaces: own.iter().map(|def| common::make(doc, def).report(def)).collect(),
            heights: vec![],
            section: vec![],
        });
    };
    let Some(def) = own
        .iter()
        .find(|s| s.id == name)
        .or_else(|| own.iter().find(|s| s.name.to_lowercase() == name.to_lowercase()))
    else {
        return Err(fail(
            codes::UNKNOWN_TERRAIN,
            format!("“{name}” kimlikli ya da adlı yüzey projede yok."),
            "surface",
        ));
    };
    let made = common::make(doc, def);
    let report = made.report(def);
    let (heights, section) = match &made.surface {
        Ok(s) => (
            s.heights(&input.at)
                .into_iter()
                .zip(&input.at)
                .map(|(h, p)| TerrainHeight {
                    x: p[0],
                    y: p[1],
                    z: h.z,
                    place: match h.place {
                        Place::Inside => TerrainPlace::Inside,
                        Place::Outside => TerrainPlace::Outside,
                        Place::Void => TerrainPlace::Void,
                    },
                })
                .collect(),
            if input.along.is_empty() {
                vec![]
            } else {
                s.section(&input.along)
                    .into_iter()
                    .map(|p| TerrainSectionPoint {
                        station: p.station,
                        x: p.x,
                        y: p.y,
                        z: p.z,
                        gap: p.gap,
                    })
                    .collect()
            },
        ),
        Err(_) => (vec![], vec![]),
    };
    Ok(TerrainsQueried {
        surfaces: vec![report],
        heights,
        section,
    })
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: vec![],
        },
        Err(stop) => stop.into(),
    }
}

/// The answer; writes nothing.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsQuery) -> CommandResult<TerrainsQueried> {
    match answer(cx.doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: vec![],
        },
        Err(stop) => stop.into(),
    }
}

/// The answer; writes nothing.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsQuery) -> CommandResult<TerrainsQueried> {
    plan(cx, &input)
}
