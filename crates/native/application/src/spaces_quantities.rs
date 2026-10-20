//! `cad.spaces.quantities` v1 (docs/adr/0342 §4, plan/cad.md `CAD-ARCH-28`, `29`), read-only: the drawing's spaces,
//! walls and openings measured from their source, those of the levels, phases, usages and layers given; each row says
//! the rule its numbers come from. Nothing is written; the plan answers the same. The shared cases are
//! `fixtures/commands/v1/cad.spaces.quantities.json`.
//!
//! - A space: its rings' area and length; its height its own, else its level's; its volume area × height; a space
//!   from the walls its area by each method too.
//! - A wall: its base line's length; its height its own, else its level's; its gross face length × height; its
//!   openings' area (width × (head − sill), sill absent 0, those with a head); its net face the gross less the openings;
//!   its volume the net face × width.
//! - An opening: its width, its height head − sill, its area.

use std::collections::BTreeMap;

use kentos_commands::{
    ArchInfo, CommandResult, Entity, OpeningRow, QuantityTotals, SpaceMethod, SpaceRow,
    SpacesQuantified, SpacesQuantities, WallRow,
};
use kentos_domain::{Document, Slot};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::spaces::{self, fail};

fn level_height(doc: &Document, arch: &ArchInfo) -> Option<f64> {
    let id = arch.level.as_deref()?;
    doc.settings()
        .arch
        .as_ref()?
        .levels
        .iter()
        .find(|l| l.id == id)?
        .height
}

fn check(doc: &Document, input: &SpacesQuantities) -> Result<SpacesQuantified, Stop> {
    let known: Vec<String> = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.iter().map(|l| l.id.clone()).collect())
        .unwrap_or_default();
    for (k, l) in input.levels.iter().flatten().enumerate() {
        if !known.contains(l) {
            return Err(fail(
                codes::LEVEL_NOT_FOUND,
                format!("Projede “{l}” kimlikli kat yok. Var olan bir katın kimliğini verin."),
                &format!("levels[{k}]"),
            ));
        }
    }
    for (k, l) in input.layers.iter().flatten().enumerate() {
        if doc.layers().get(l).is_none() {
            return Err(fail(
                codes::LAYER_NOT_FOUND,
                format!(
                    "“{l}” kimlikli katman çizimde yok. Var olan bir katmanın kimliğini verin."
                ),
                &format!("layers[{k}]"),
            ));
        }
    }
    let takes = |e: &Entity, arch: &ArchInfo| {
        input
            .layers
            .as_ref()
            .is_none_or(|ls| ls.contains(&e.base().layer_id))
            && input
                .levels
                .as_ref()
                .is_none_or(|ls| arch.level.as_ref().is_some_and(|l| ls.contains(l)))
            && input
                .phases
                .as_ref()
                .is_none_or(|ps| arch.phase.is_some_and(|p| ps.contains(&p)))
    };
    let uid = |e: &Entity| {
        doc.uid(Slot(e.base().id))
            .map(|u| u.to_string())
            .unwrap_or_default()
    };
    let mut out = SpacesQuantified {
        spaces: Vec::new(),
        walls: Vec::new(),
        openings: Vec::new(),
        totals: QuantityTotals::default(),
    };
    // The openings of each wall, by the wall's id.
    let mut on_wall: BTreeMap<String, Vec<(String, f64)>> = BTreeMap::new();
    for e in doc.entities() {
        let Entity::Opening(o) = e else { continue };
        let x = &o.opening;
        let wall = doc.wall_line_of(&x.host).is_some().then(|| x.host.clone());
        let height = x.head.map(|h| h - x.sill.unwrap_or(0.0));
        let area = height.map(|h| x.width * h);
        if let (Some(w), Some(a)) = (&wall, area) {
            on_wall.entry(w.clone()).or_default().push((uid(e), a));
        }
        let host_arch = wall
            .as_ref()
            .and_then(|w| kentos_domain::Uuid::parse_str(w).ok())
            .and_then(|u| doc.slot_of(u))
            .and_then(|s| match doc.get(s) {
                Some(Entity::Wall(w)) => Some(w.wall.arch.clone()),
                _ => None,
            })
            .unwrap_or_default();
        if !takes(e, &host_arch) || input.usages.is_some() {
            continue;
        }
        out.openings.push(OpeningRow {
            uid: uid(e),
            form: x.form,
            style: x.style.clone(),
            number: x.number.clone(),
            wall,
            width: x.width,
            height,
            area,
        });
    }
    for e in doc.entities() {
        match e {
            Entity::Space(s) => {
                let f = &s.space;
                if !takes(e, &f.arch)
                    || input
                        .usages
                        .as_ref()
                        .is_some_and(|us| f.usage.as_ref().is_none_or(|u| !us.contains(u)))
                {
                    continue;
                }
                let area = spaces::area_of(&f.parts);
                let height = f.height.or_else(|| level_height(doc, &f.arch));
                let by_method = f.seed.map(|_| {
                    SpaceMethod::ALL
                        .iter()
                        .filter_map(|m| {
                            let mut g = f.clone();
                            g.method = *m;
                            let g = doc.space_found(&g);
                            g.broken
                                .is_none()
                                .then(|| (m.name().to_owned(), spaces::area_of(&g.parts)))
                        })
                        .collect()
                });
                out.spaces.push(SpaceRow {
                    uid: uid(e),
                    number: f.number.clone(),
                    name: f.name.clone(),
                    usage: f.usage.clone(),
                    level: f.arch.level.clone(),
                    method: f.method,
                    area,
                    perimeter: spaces::perimeter_of(&f.parts),
                    height,
                    volume: height.map(|h| area * h),
                    by_method,
                    rule: format!(
                        "alan: halkaların alanı, delikler düşülür ({}); hacim: alan × yükseklik",
                        f.method.label()
                    ),
                });
            }
            Entity::Wall(w) if input.usages.is_none() => {
                let x = &w.wall;
                if !takes(e, &x.arch) {
                    continue;
                }
                let length = kentos_geometry_core::geom::bulge::bulge_path_length(
                    &crate::walls::core_points(&x.pts),
                    x.bulges.as_deref(),
                    x.closed,
                );
                let height = x.height.or_else(|| level_height(doc, &x.arch));
                let id = uid(e);
                let openings = on_wall.get(&id).cloned().unwrap_or_default();
                let openings_area: f64 = openings.iter().map(|(_, a)| a).sum();
                let gross = height.map(|h| length * h);
                let net = gross.map(|g| g - openings_area);
                out.walls.push(WallRow {
                    uid: id,
                    style: x.style.clone(),
                    level: x.arch.level.clone(),
                    length,
                    width: x.width,
                    height,
                    gross_area: gross,
                    openings_area,
                    net_area: net,
                    volume: net.map(|n| n * x.width),
                    openings: openings.into_iter().map(|(u, _)| u).collect(),
                    rule: "yüzey: taban çizgisinin uzunluğu × yükseklik, açıklıkların alanı düşülür; hacim: net yüzey × kalınlık".into(),
                });
            }
            _ => {}
        }
    }
    out.totals = QuantityTotals {
        space_area: out.spaces.iter().map(|s| s.area).sum(),
        wall_length: out.walls.iter().map(|w| w.length).sum(),
        wall_net_area: out.walls.iter().filter_map(|w| w.net_area).sum(),
        openings: u32::try_from(out.openings.len()).unwrap_or(u32::MAX),
        opening_area: out.openings.iter().filter_map(|o| o.area).sum(),
    };
    Ok(out)
}

pub fn validate(cx: &ExecutionContext<'_>, input: &SpacesQuantities) -> CommandResult<()> {
    match check(cx.doc, input) {
        Ok(_) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn plan(
    cx: &ExecutionContext<'_>,
    input: &SpacesQuantities,
) -> CommandResult<SpacesQuantified> {
    plan_in(cx.doc, input)
}

/// [`plan`] on a document: what the Metraj window shows.
pub fn plan_in(doc: &Document, input: &SpacesQuantities) -> CommandResult<SpacesQuantified> {
    match check(doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

/// Read-only: the same answer as the plan.
pub fn execute(
    cx: &mut ExecutionContext<'_>,
    input: SpacesQuantities,
) -> CommandResult<SpacesQuantified> {
    plan_in(cx.doc, &input)
}
