//! `road.volumes.ledger` v1 (plan/road.md Y43, docs/adr/0949; ROAD-VOL-16, ROAD-VOL-18): an earthwork volume's total
//! taken apart (`kentos_road::volume_ledger`): its stretches grouped by km or by the corridor's regions, whole, by their
//! middle, so the groups add up to the total; each material its column; the largest stretches of the cut and the fill
//! with their length and mean area over the medians; the concentration along the road; the rounded stretches' sum
//! against the rounded total. Reads only. The shared cases are `fixtures/commands/v1/road.volumes.ledger.json`.

use kentos_commands::{
    CommandResult, CommandWarning, LedgerBy, LedgerColumn, LedgerConcentration, LedgerGroup, LedgerStretch, LedgerTop,
    VolumeLedgerRead, VolumesLedger,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::volume_ledger::{Stretch, concentration, group, reasons, rounding, steps, sum};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::fail;
use crate::road_volumes as common;

fn bad(path: &str, words: &str) -> Stop {
    fail(codes::INVALID_VOLUME_LEDGER, format!("Kübaj defteri sorusu geçerli değil: {words}."), path)
}

fn answer(doc: &Document, input: &VolumesLedger) -> Result<(VolumeLedgerRead, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (top, decimals) = judged(input)?;
    let (_, def) = common::find(doc.road_volumes(), &input.volume, "volume")?;
    ledger(doc, def, input, top, decimals)
}

/// The question's own rules: its step, top and decimals; the top and decimals.
fn judged(input: &VolumesLedger) -> Result<(u32, u32), Stop> {
    if let LedgerBy::Stations { step } = input.by
        && !(step.is_finite() && (1.0..=100_000.0).contains(&step))
    {
        return Err(bad("by.step", "km aralığı 1 ile 100000 m arası olur"));
    }
    let top = input.top.unwrap_or(10);
    if !(1..=100).contains(&top) {
        return Err(bad("top", "en büyük aralıkların sayısı 1 ile 100 arası olur"));
    }
    let decimals = input.decimals.unwrap_or(2);
    if decimals > 3 {
        return Err(bad("decimals", "yuvarlama en çok 3 basamak olur"));
    }
    Ok((top, decimals))
}

/// The ledger of `def` (saved or as typed).
fn ledger(
    doc: &Document,
    def: &kentos_commands::VolumeDef,
    input: &VolumesLedger,
    top: u32,
    decimals: u32,
) -> Result<(VolumeLedgerRead, Vec<CommandWarning>), Stop> {
    let (report, warnings, alignment) = common::compute_on(doc, def)?;
    let text = |s: f64| km_text(alignment.km(s), 3);
    // The columns: the cut and fill, factored, then the materials the volume has.
    let mut columns: Vec<(String, String)> = vec![
        ("cut".into(), "Kazı".into()),
        ("fill".into(), "Dolgu".into()),
        ("factoredCut".into(), "Düzeltilmiş kazı".into()),
        ("factoredFill".into(), "Düzeltilmiş dolgu".into()),
    ];
    if report.strip.is_some() {
        columns.push(("strip".into(), "Bitkisel toprak".into()));
    }
    if report.bench.is_some() {
        columns.push(("bench".into(), "Diş kazısı".into()));
    }
    for l in &report.layers {
        columns.push((format!("layer:{}", l.name), l.name.clone()));
    }
    let stretches: Vec<Stretch> = report
        .rows
        .iter()
        .map(|r| {
            let mut values = vec![
                r.cut,
                r.fill,
                r.cut.map(|c| c * r.cut_factor.unwrap_or(1.0)),
                r.fill.map(|f| f * r.fill_factor.unwrap_or(1.0)),
            ];
            if report.strip.is_some() {
                values.push(r.strip);
            }
            if report.bench.is_some() {
                values.push(r.bench);
            }
            for k in 0..report.layers.len() {
                values.push(r.layers.get(k).copied().flatten());
            }
            Stretch { from: r.from, to: r.to, values }
        })
        .collect();
    let width = columns.len();
    let totals: Vec<f64> = (0..width).map(|k| sum(stretches.iter().filter_map(|s| s.values[k]))).collect();
    let columns_out: Vec<LedgerColumn> = columns
        .iter()
        .enumerate()
        .map(|(k, (key, words))| {
            let present: Vec<f64> = stretches.iter().filter_map(|s| s.values[k]).collect();
            let (rows_rounded, total_rounded, residue) = rounding(&present, decimals);
            LedgerColumn { key: key.clone(), words: words.clone(), total: totals[k], rows_rounded, total_rounded, residue }
        })
        .collect();
    // The groups' ranges and words.
    let (first, last) = (
        stretches.iter().map(|s| s.from).fold(f64::INFINITY, f64::min),
        stretches.iter().map(|s| s.to).fold(f64::NEG_INFINITY, f64::max),
    );
    let (ranges, names): (Vec<(f64, f64)>, Vec<String>) = match input.by {
        LedgerBy::Stations { step } if !stretches.is_empty() => {
            let r = steps(first, last, step);
            let n = r.iter().map(|(a, b)| format!("{} – {}", text(*a), text(*b))).collect();
            (r, n)
        }
        LedgerBy::Stations { .. } => (Vec::new(), Vec::new()),
        LedgerBy::Factors => def
            .factors
            .iter()
            .map(|f| {
                let factor = |w: &str, v: Option<f64>| v.map_or_else(String::new, |v| format!(" · {w} {v}"));
                ((f.from, f.to), format!("{} – {}{}{}", text(f.from), text(f.to), factor("kabarma", f.cut), factor("sıkışma", f.fill)))
            })
            .unzip(),
        LedgerBy::Regions => {
            let cdef = doc.road_corridors().iter().find(|c| c.id == def.corridor);
            cdef.map_or_else(
                || (Vec::new(), Vec::new()),
                |c| {
                    c.regions
                        .iter()
                        .enumerate()
                        .map(|(k, r)| {
                            let template = doc.road_templates().iter().find(|t| t.id == r.template).map_or(r.template.as_str(), |t| t.name.as_str());
                            ((r.from, r.to), format!("Bölge {} · {template}", k + 1))
                        })
                        .unzip()
                },
            )
        }
    };
    let mut grouped = group(&stretches, &ranges);
    let mut names = names;
    // By the factor rows, the stretches under none are a group of their own (ROAD-VOL-21): their factored volume is raw.
    if input.by == LedgerBy::Factors {
        let taken: std::collections::BTreeSet<usize> = grouped.iter().flat_map(|g| g.members.iter().copied()).collect();
        let rest: Vec<usize> = (0..stretches.len()).filter(|i| !taken.contains(i)).collect();
        if !rest.is_empty() {
            let values = (0..width).map(|k| sum(rest.iter().filter_map(|&i| stretches[i].values[k]))).collect();
            let from = rest.iter().map(|&i| stretches[i].from).fold(f64::INFINITY, f64::min);
            let to = rest.iter().map(|&i| stretches[i].to).fold(f64::NEG_INFINITY, f64::max);
            grouped.push(kentos_road::volume_ledger::Group { from, to, members: rest, values });
            names.push("Katsayısız".into());
        }
    }
    let groups: Vec<LedgerGroup> = grouped
        .iter()
        .map(|g| {
            let m = 0.5 * (stretches[g.members[0]].from + stretches[g.members[0]].to);
            let n = ranges.len();
            let k = ranges
                .iter()
                .enumerate()
                .position(|(k, r)| r.0 <= m && (m < r.1 || (k + 1 == n && m <= r.1)))
                .or_else(|| (names.len() > ranges.len()).then_some(ranges.len()));
            LedgerGroup {
                label: k.map_or_else(String::new, |k| names[k].clone()),
                from: g.from,
                to: g.to,
                from_text: text(g.from),
                to_text: text(g.to),
                stretches: g.members.len() as u32,
                values: g.values.clone(),
                shares: g.values.iter().zip(&totals).map(|(v, t)| (*t > 0.0).then(|| v / t)).collect(),
            }
        })
        .collect();
    let residual: Vec<f64> = (0..width).map(|k| sum(grouped.iter().map(|g| g.values[k])) - totals[k]).collect();
    // The cut's and the fill's largest stretches and concentration.
    let lengths: Vec<f64> = stretches.iter().map(|s| s.to - s.from).collect();
    let mut tops = Vec::new();
    let mut crowding = Vec::new();
    for (k, key) in [(0usize, "cut"), (1, "fill")] {
        let vols: Vec<f64> = stretches.iter().map(|s| s.values[k].unwrap_or(0.0)).collect();
        if totals[k] <= 0.0 {
            continue;
        }
        let why = reasons(&lengths, &vols);
        let mut order: Vec<usize> = (0..vols.len()).filter(|&i| vols[i] > kentos_road::volume_ledger::NONE).collect();
        order.sort_by(|&a, &b| vols[b].total_cmp(&vols[a]).then(a.cmp(&b)));
        let mut running = Vec::new();
        let list = order
            .into_iter()
            .take(top as usize)
            .filter_map(|i| {
                let r = why[i]?;
                running.push(vols[i]);
                let s = &stretches[i];
                Some(LedgerStretch {
                    from: s.from,
                    to: s.to,
                    from_text: text(s.from),
                    to_text: text(s.to),
                    volume: vols[i],
                    share: vols[i] / totals[k],
                    cumulative: sum(running.iter().copied()) / totals[k],
                    length: lengths[i],
                    mean_area: vols[i] / lengths[i],
                    length_factor: r.length,
                    area_factor: r.area,
                    reason: if r.area >= r.length { "section" } else { "length" }.into(),
                })
            })
            .collect();
        tops.push(LedgerTop { column: key.into(), stretches: list });
        if let Some(c) = concentration(&lengths, &vols) {
            crowding.push(LedgerConcentration {
                column: key.into(),
                gini: c.gini,
                half: c.half,
                most: c.most,
                curve: c.curve.iter().map(|p| [p.0, p.1]).collect(),
            });
        }
    }
    Ok((
        VolumeLedgerRead {
            volume: def.id.clone(),
            name: def.name.clone(),
            decimals,
            columns: columns_out,
            groups,
            residual,
            top: tops,
            concentration: crowding,
            gaps: report.gaps.clone(),
        },
        warnings,
    ))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &VolumesLedger) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &VolumesLedger) -> CommandResult<VolumeLedgerRead> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: VolumesLedger) -> CommandResult<VolumeLedgerRead> {
    plan(cx, &input)
}

/// The ledger for a window, or why it does not read.
pub fn preview(doc: &Document, input: &VolumesLedger) -> Result<(VolumeLedgerRead, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}

/// The ledger of a volume as typed (the desktop's Kübaj window, before it is saved).
pub fn preview_of(
    doc: &Document,
    def: &kentos_commands::VolumeDef,
    input: &VolumesLedger,
) -> Result<(VolumeLedgerRead, Vec<CommandWarning>), String> {
    judged(input).and_then(|(top, decimals)| ledger(doc, def, input, top, decimals)).map_err(crate::road_corridors::words)
}
