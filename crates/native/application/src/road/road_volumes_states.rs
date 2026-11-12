//! `road.volumes.states` v1 (plan/road.md Y51, docs/adr/0957; ROAD-VOL-10): a volume's earthwork in its three states,
//! row by row of its factor table. Mass is kept: a cut of `V` m³ in place weighs `ρ_bank·V`, loosened it fills
//! `V·ρ_bank/ρ_loose` m³ and compacted `V·ρ_bank/ρ_compacted`; a fill of `V` m³ compacted asks `V·ρ_compacted/ρ_bank`
//! in place and `V·ρ_compacted/ρ_loose` loosened. The volume's mass ordinate compares the cut and the fill loosened
//! (the factors, or the densities' bank over loose and compacted over loose). A factor below 1 says that a loosened
//! volume is smaller than the one it came from — no soil does that — and is told, as are densities that would. Reads
//! only. The shared cases are `fixtures/commands/v1/road.volumes.states.json`.

use kentos_commands::{CommandResult, CommandWarning, VolumeDef, VolumeStateRow, VolumeStatesRead, VolumesStates};
use kentos_domain::Document;
use kentos_road::alignment::km_text;

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_volumes as common;

/// What a row's factors or densities say that the soil cannot do, in words.
fn checks(cut: Option<f64>, fill: Option<f64>, d: Option<&kentos_commands::VolumeDensities>) -> Vec<String> {
    let mut out = Vec::new();
    match d {
        Some(d) => {
            if d.loose > d.bank {
                out.push(format!("gevşek yoğunluk ({} t/m³) yerindekinden ({} t/m³) büyük: kazı gevşeyince küçülmez", d.loose, d.bank));
            }
            if d.compacted < d.loose {
                out.push(format!(
                    "sıkışık yoğunluk ({} t/m³) gevşekten ({} t/m³) küçük: sıkıştırılan malzeme gevşekten hafif olmaz",
                    d.compacted, d.loose
                ));
            }
        }
        None => {
            if let Some(c) = cut.filter(|c| *c < 1.0) {
                out.push(format!("kabarma 1'den küçük ({c}): gevşek hacim yerindekinden az olamaz"));
            }
            if let Some(f) = fill.filter(|f| *f < 1.0) {
                out.push(format!(
                    "dolgu katsayısı 1'den küçük ({f}): dolgunun istediği gevşek hacim sıkışık hacminden az olamaz; katsayı yerinde hacme göre yazıldıysa kütle ordinatı kazıyı gevşek, dolguyu yerinde sayar"
                ));
            }
        }
    }
    out
}

/// The states of `def` (saved or as typed), and the warnings.
fn states(doc: &Document, def: &VolumeDef) -> Result<(VolumeStatesRead, Vec<CommandWarning>), Stop> {
    let (report, mut warnings, alignment) = common::compute_on(doc, def)?;
    let text = |s: f64| km_text(alignment.km(s), 3);
    let n = def.factors.len();
    // Each computed stretch goes to the row holding its middle (the last row's end included), as the volume's factors.
    let row_of = |m: f64| def.factors.iter().enumerate().position(|(i, f)| f.from <= m && (m < f.to || (i + 1 == n && m <= f.to)));
    let mut sums = vec![(0.0, 0.0); n];
    let (mut un_cut, mut un_fill) = (0.0, 0.0);
    for r in &report.rows {
        let (Some(c), Some(f)) = (r.cut, r.fill) else { continue };
        match row_of(0.5 * (r.from + r.to)) {
            Some(k) => {
                sums[k].0 += c;
                sums[k].1 += f;
            }
            None => {
                un_cut += c;
                un_fill += f;
            }
        }
    }
    let mut rows = Vec::with_capacity(n);
    let (mut cut, mut fill, mut cut_loose, mut fill_loose) = (un_cut, un_fill, un_cut, un_fill);
    let (mut cut_mass, mut fill_mass) = (0.0, 0.0);
    let mut weighed = un_cut == 0.0 && un_fill == 0.0;
    let mut told = Vec::new();
    for (f, (c, fl)) in def.factors.iter().zip(&sums) {
        let (cf, ff) = f.effective();
        let d = f.densities.as_ref();
        let row = VolumeStateRow {
            from: f.from,
            to: f.to,
            from_text: text(f.from),
            to_text: text(f.to),
            source: f.source.clone(),
            densities: f.densities,
            cut_factor: cf,
            fill_factor: ff,
            cut: *c,
            fill: *fl,
            cut_loose: c * cf.unwrap_or(1.0),
            fill_loose: fl * ff.unwrap_or(1.0),
            fill_bank: d.map(|d| fl * d.compacted / d.bank),
            cut_compacted: d.map(|d| c * d.bank / d.compacted),
            cut_mass: d.map(|d| c * d.bank),
            fill_mass: d.map(|d| fl * d.compacted),
            checks: checks(cf, ff, d),
        };
        cut += c;
        fill += fl;
        cut_loose += row.cut_loose;
        fill_loose += row.fill_loose;
        match (row.cut_mass, row.fill_mass) {
            (Some(a), Some(b)) => {
                cut_mass += a;
                fill_mass += b;
            }
            _ => weighed = false,
        }
        for words in &row.checks {
            told.push(format!("km {} – {}: {words}", row.from_text, row.to_text));
        }
        rows.push(row);
    }
    if !told.is_empty() {
        warnings.push(CommandWarning {
            code: codes::VOLUME_FACTOR_PHYSICS.into(),
            message: format!("“{}” kübajının katsayıları zeminin yapamayacağını söylüyor: {}.", def.name, told.join("; ")),
            path: Some("factors".into()),
        });
    }
    let out = VolumeStatesRead {
        volume: def.id.clone(),
        rows,
        unfactored_cut: un_cut,
        unfactored_fill: un_fill,
        cut,
        fill,
        cut_loose,
        fill_loose,
        balance: cut_loose - fill_loose,
        cut_mass: (weighed && n > 0).then_some(cut_mass),
        fill_mass: (weighed && n > 0).then_some(fill_mass),
    };
    Ok((out, warnings))
}

fn answer(doc: &Document, input: &VolumesStates) -> Result<(VolumeStatesRead, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    let (_, def) = common::find(doc.road_volumes(), &input.volume, "volume")?;
    states(doc, def)
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &VolumesStates) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &VolumesStates) -> CommandResult<VolumeStatesRead> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: VolumesStates) -> CommandResult<VolumeStatesRead> {
    plan(cx, &input)
}

/// The states of a volume as typed (the desktop's Kübaj window, before it is saved).
pub fn preview_of(doc: &Document, def: &VolumeDef) -> Result<(VolumeStatesRead, Vec<CommandWarning>), String> {
    states(doc, def).map_err(crate::road_corridors::words)
}
