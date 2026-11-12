//! `road.junctions.overlap` v1 (plan/road.md Y17.6, docs/adr/0919): where a junction's two corridors overlap in plan,
//! their top surfaces sampled at a grid's cell centres (`kentos-road`'s `junction::shared_cells`), and the secondary
//! corridor's volume there, counted twice when both corridors' volumes are summed. Reads only. The shared cases are
//! `fixtures/commands/v1/road.junctions.overlap.json`.

use kentos_commands::{
    CommandResult, CommandWarning, CorridorDef, JunctionOverlapAt, JunctionsOverlap, JunctionsOverlapped, MOST_OVERLAP_CELLS,
};
use kentos_domain::Document;
use kentos_road::junction::{Located, shared_cells, shared_grid};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_alignments as alignments;
use crate::road_corridors as corridors;
use crate::road_junctions as common;
use crate::road_terrains::fail;

fn bad(words: String, path: &str) -> Stop {
    fail(codes::INVALID_OVERLAP, words, path)
}

fn answer(doc: &Document, input: &JunctionsOverlap) -> Result<(JunctionsOverlapped, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_junctions(), &input.junction, "junction")?;
    let (report, mut warnings) = common::read(doc, def)?;
    let own = doc.road_corridors();
    let mut chosen: Vec<&CorridorDef> = Vec::new();
    match &input.corridors {
        Some(names) => {
            for (k, name) in names.iter().enumerate() {
                chosen.push(corridors::find(own, name, &format!("corridors[{k}]"))?.1);
            }
        }
        None => {
            for leg in &report.legs {
                let on: Vec<&CorridorDef> = own.iter().filter(|c| c.alignment == leg.alignment).collect();
                let name = alignments::alignments(doc).iter().find(|a| a.id == leg.alignment).map_or("", |a| a.name.as_str());
                match on.as_slice() {
                    [one] => chosen.push(one),
                    [] => return Err(bad(format!("“{name}” aksının koridoru yok; corridors ile verin."), "corridors")),
                    _ => return Err(bad(format!("“{name}” aksının {} koridoru var; hangisi, corridors ile verin.", on.len()), "corridors")),
                }
            }
        }
    }
    let mut made = Vec::new();
    for c in &chosen {
        made.push(corridors::make(doc, c, false)?);
    }
    if chosen.len() != 2 {
        return Err(bad(format!("Kolların sırasıyla iki koridor verin; {} verildi.", chosen.len()), "corridors"));
    }
    for (k, (c, leg)) in chosen.iter().zip(&report.legs).enumerate() {
        if c.alignment != leg.alignment {
            return Err(bad(format!("“{}” koridoru kavşağın {}. kolunun aksında değil.", c.name, k + 1), &format!("corridors[{k}]")));
        }
    }
    if input.primary > 1 {
        return Err(bad("Kavşağın alanının sayıldığı kol (primary) 0 ya da 1 olmalı.".into(), "primary"));
    }
    if !(input.cell.is_finite() && input.cell > 0.0) {
        return Err(bad("Izgara aralığı (cell) 0'dan büyük olmalı.".into(), "cell"));
    }
    let (p, q) = (input.primary as usize, 1 - input.primary as usize);
    let Some(ground) = made[q].sampler.surface.as_ref() else {
        return Err(bad(format!("“{}” koridorunun arazi yüzeyi kurulamıyor.", chosen[q].name), "corridors"));
    };
    for c in &chosen {
        if corridors::stale(doc, c) {
            let now = doc.road_alignments().iter().find(|a| a.id == c.alignment).map_or(0, |a| a.revision);
            warnings.push(CommandWarning {
                code: codes::CORRIDOR_STALE.into(),
                message: format!(
                    "“{}” koridorunun aksı değişti (koridor {}. revizyona göre kurulu, aks şimdi {now}.); bölgeleri denetleyip aksa yeniden bağlayın.",
                    c.name, c.alignment_revision
                ),
                path: Some("corridors".into()),
            });
        }
    }
    let (a, b) = (Located::new(&made[p].built.triangles), Located::new(&made[q].built.triangles));
    if let Some((_, size)) = shared_grid(&a, &b, input.cell)
        && size[0].saturating_mul(size[1]) > MOST_OVERLAP_CELLS
    {
        return Err(bad(format!("Aralık çok küçük: {} × {} hücre; en çok {MOST_OVERLAP_CELLS}.", size[0], size[1]), "cell"));
    }
    let cells = shared_cells(&a, &b, input.cell);
    let area_of = input.cell * input.cell;
    let (mut cut, mut fill, mut uncovered, mut sum) = (0.0, 0.0, 0u32, 0.0);
    let mut largest: Option<JunctionOverlapAt> = None;
    let mut ranges = [[f64::INFINITY, f64::NEG_INFINITY]; 2];
    for c in &cells {
        match ground.height(c.at[0], c.at[1]).z {
            Some(z) if z > c.b => cut += area_of * (z - c.b),
            Some(z) => fill += area_of * (c.b - z),
            None => uncovered += 1,
        }
        let d = c.a - c.b;
        sum += d.abs();
        if largest.as_ref().is_none_or(|l| d.abs() > l.difference.abs()) {
            largest = Some(JunctionOverlapAt { x: c.at[0], y: c.at[1], primary: c.a, secondary: c.b, difference: d });
        }
        for (k, m) in [p, q].into_iter().enumerate() {
            if let Some(f) = made[m].sampler.alignment.core.locate(c.at).nearest {
                ranges[k] = [ranges[k][0].min(f.at.station), ranges[k][1].max(f.at.station)];
            }
        }
    }
    let n = cells.len();
    let area = n as f64 * area_of;
    if n > 0 {
        warnings.push(CommandWarning {
            code: codes::JUNCTION_CORRIDORS_OVERLAP.into(),
            message: format!(
                "“{}” ve “{}” koridorları “{}” kavşağında {area:.1} m² örtüşüyor; tali koridorun buradaki {cut:.1} m³ yarması ve {fill:.1} m³ dolgusu iki kez sayılır.",
                chosen[p].name, chosen[q].name, def.name
            ),
            path: Some("corridors".into()),
        });
    }
    let range = |r: [f64; 2]| (r[0] <= r[1]).then_some(r);
    Ok((
        JunctionsOverlapped {
            junction: def.id.clone(),
            primary: input.primary,
            primary_corridor: chosen[p].id.clone(),
            secondary_corridor: chosen[q].id.clone(),
            cell: input.cell,
            cells: n as u32,
            area,
            cut,
            fill,
            uncovered,
            mean_difference: (n > 0).then(|| sum / n as f64),
            largest,
            primary_range: range(ranges[0]),
            secondary_range: range(ranges[1]),
        },
        warnings,
    ))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &JunctionsOverlap) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &JunctionsOverlap) -> CommandResult<JunctionsOverlapped> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: JunctionsOverlap) -> CommandResult<JunctionsOverlapped> {
    plan(cx, &input)
}

/// The overlap for a window, or why it does not read; and the overlapping cells' centres.
pub fn preview(doc: &Document, input: &JunctionsOverlap) -> Result<(JunctionsOverlapped, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
