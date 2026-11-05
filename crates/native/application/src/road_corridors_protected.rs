//! `road.corridors.protected` v1 (plan/road.md Y46, docs/adr/0952; ROAD-GRADE-06): a corridor's footprint as built now
//! against protected areas (`kentos_road::protected`): the area objects on the layers asked; for each, the top surface's
//! cover, the sections whose footprint (the left slope's end to the right's, in plan) reaches into it, from which km to
//! which, and the longest reach. Reads only. The shared cases are `fixtures/commands/v1/road.corridors.protected.json`.

use kentos_commands::{CommandResult, CommandWarning, CorridorProtected, CorridorsProtected, ProtectedArea};
use kentos_commands::Entity;
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::protected::{Area, signed_area};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_corridors as common;
use crate::road_terrains::fail;

fn bad(words: String, path: &str) -> Stop {
    fail(codes::INVALID_PROTECTED_CHECK, format!("Korunan alan denetimi sorusu geçerli değil: {words}"), path)
}

/// The areas on `layers`: each area object's parts, with their ids, parts and layers.
fn areas(doc: &Document, layers: &[String]) -> Vec<(u64, u32, String, Area)> {
    let ring = |pts: &[kentos_commands::Vec2]| pts.iter().map(|p| [p.x, p.y]).collect::<Vec<[f64; 2]>>();
    let mut out = Vec::new();
    for e in doc.entities() {
        let Entity::Polygon(p) = e else { continue };
        if !layers.contains(&p.base.layer_id) {
            continue;
        }
        let holes = |h: &Option<Vec<kentos_commands::RingGeometry>>| h.iter().flatten().map(|r| ring(&r.pts)).collect();
        out.push((u64::from(p.base.id), 0, p.base.layer_id.clone(), Area { outer: ring(&p.pts), holes: holes(&p.holes) }));
        for (k, part) in p.parts.iter().flatten().enumerate() {
            out.push((u64::from(p.base.id), k as u32 + 1, p.base.layer_id.clone(), Area { outer: ring(&part.pts), holes: holes(&part.holes) }));
        }
    }
    out
}

/// The footprint against the areas now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &CorridorsProtected) -> Result<(CorridorProtected, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_corridors(), &input.corridor, "corridor")?;
    if input.layers.is_empty() || input.layers.len() > 20 {
        return Err(bad("korunan alanların katmanları 1 ile 20 arası olur.".into(), "layers"));
    }
    for (i, id) in input.layers.iter().enumerate() {
        if doc.layers().get(id).is_none() {
            return Err(bad(format!("“{id}” kimlikli katman çizimde yok."), &format!("layers[{i}]")));
        }
    }
    let made = common::make(doc, def, false)?;
    let text = |s: f64| km_text(made.sampler.alignment.km(s), 3);
    let plan = |t: &[[f64; 3]; 3]| [[t[0][0], t[0][1]], [t[1][0], t[1][1]], [t[2][0], t[2][1]]];
    let footprint: f64 = made.built.triangles.iter().map(|t| signed_area(&plan(t)).abs()).sum();
    // Each section's footprint in plan: the left slope's end to the right's.
    let feet: Vec<(f64, [f64; 2], [f64; 2])> = made
        .regions
        .iter()
        .flat_map(|r| r.crosses.iter())
        .filter_map(|c| {
            let sec = c.section.as_ref()?;
            let (l, r) = (sec.left.points.last()?[0], sec.right.points.last()?[0]);
            let (sin, cos) = c.line.direction.sin_cos();
            let at = |o: f64| [c.line.at[0] + sin * o, c.line.at[1] - cos * o];
            Some((c.line.station, at(l), at(r)))
        })
        .collect();
    let mut out = Vec::new();
    for (entity, part, layer, area) in areas(doc, &input.layers) {
        let covered: f64 = made.built.triangles.iter().map(|t| area.covered(plan(t))).sum();
        let mut reached: Vec<(f64, f64)> = feet.iter().map(|(s, a, b)| (*s, area.reach(*a, *b))).filter(|(_, r)| *r > 1e-9).collect();
        reached.sort_by(|x, y| x.0.total_cmp(&y.0));
        let longest = reached.iter().copied().fold(None::<(f64, f64)>, |m, x| if m.is_none_or(|m| x.1 > m.1) { Some(x) } else { m });
        let size = area.size();
        let mut rings = vec![area.outer.clone()];
        rings.extend(area.holes.iter().cloned());
        out.push(ProtectedArea {
            entity,
            part,
            layer,
            size,
            covered,
            share: if size > 0.0 { covered / size } else { 0.0 },
            sections: reached.len() as u32,
            from: reached.first().map(|r| r.0),
            to: reached.last().map(|r| r.0),
            from_text: reached.first().map(|r| text(r.0)),
            to_text: reached.last().map(|r| text(r.0)),
            reach: longest.map_or(0.0, |l| l.1),
            reach_at: longest.map(|l| l.0),
            rings,
        });
    }
    let covered: f64 = out.iter().map(|a| a.covered).sum();
    let mut warnings = made.warnings;
    let hit: Vec<&ProtectedArea> = out.iter().filter(|a| a.covered > 1e-6 || a.sections > 0).collect();
    if !hit.is_empty() {
        let words: Vec<String> = hit
            .iter()
            .take(8)
            .map(|a| {
                let km = match (&a.from_text, &a.to_text) {
                    (Some(f), Some(t)) if f != t => format!("km {f} – {t}"),
                    (Some(f), _) => format!("km {f}"),
                    _ => "kesitsiz".into(),
                };
                format!("{} numaralı alan {:.1} m² ({km}, en uzun giriş {:.2} m)", a.entity, a.covered, a.reach)
            })
            .collect();
        let more = if hit.len() > 8 { format!(" ve {} alan daha", hit.len() - 8) } else { String::new() };
        warnings.push(CommandWarning {
            code: codes::CORRIDOR_PROTECTED.into(),
            message: format!("“{}” koridorunun izi korunan alanlara giriyor: {}{more}.", def.name, words.join("; ")),
            path: Some("layers".into()),
        });
    }
    Ok((CorridorProtected { corridor: def.id.clone(), footprint, areas: out, covered }, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &CorridorsProtected) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &CorridorsProtected) -> CommandResult<CorridorProtected> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: CorridorsProtected) -> CommandResult<CorridorProtected> {
    plan(cx, &input)
}

/// The footprint against the areas for a window, or why it does not read.
pub fn preview(doc: &Document, input: &CorridorsProtected) -> Result<(CorridorProtected, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(common::words)
}
