//! `road.volumes.uncertainty` v1 (plan/road.md Y35, docs/adr/0941): an earthwork volume's uncertainty budget. The
//! ground's heights err by σ (random) and β (a shift): each ground point's weight in the cut and the fill is read on
//! the corridor's top surface against the ground (`kentos_road::volume::sensitivity`), the spreads and the 95 % bound
//! follow (`kentos_road::volume_uncertainty`); the other method's volume is the method's share, the binary64 summation
//! bound the arithmetic's. Reads only. The shared cases are `fixtures/commands/v1/road.volumes.uncertainty.json`.

use kentos_commands::{
    CommandResult, CommandWarning, StretchSpread, VolumeMethod, VolumeSpread, VolumeUncertaintyReport, VolumesUncertainty,
};
use kentos_domain::Document;
use kentos_road::alignment::km_text;
use kentos_road::volume::sensitivity;
use kentos_road::volume_uncertainty::{bias, bound95, numeric_bound, spread};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_corridors as corridors;
use crate::road_terrains::fail;
use crate::road_volumes as common;

/// Over this share of the area within 2σ of the zero line, the cut's and the fill's σ apart are only approximate
/// (docs/adr/0941 §2: up to 0.5 the trials agreed within 2.3 %, at 1.0 they strayed by up to 31 %).
pub const NEAR_SHARE: f64 = 0.5;

fn bad(path: &str, words: &str) -> Stop {
    fail(codes::INVALID_VOLUME_UNCERTAINTY, words.into(), path)
}

fn judged(input: &VolumesUncertainty) -> Result<(), Stop> {
    if !(input.sigma.is_finite() && input.sigma > 0.0 && input.sigma <= 10.0) {
        return Err(bad("sigma", "Kotların standart sapması 0'dan büyük, en çok 10 m olmalı."));
    }
    if input.systematic.is_some_and(|b| !(b.is_finite() && b.abs() <= 10.0)) {
        return Err(bad("systematic", "Sistematik kayma en çok 10 m olabilir."));
    }
    if input.accept.is_some_and(|p| !(p.is_finite() && p > 0.0 && p <= 100.0)) {
        return Err(bad("accept", "Kabul payı 0'dan büyük, en çok yüzde 100 olmalı."));
    }
    Ok(())
}

/// The budget now, and its warnings.
pub(crate) fn answer(
    doc: &Document,
    input: &VolumesUncertainty,
) -> Result<(VolumeUncertaintyReport, Vec<CommandWarning>), Stop> {
    common::road_project(doc)?;
    judged(input)?;
    let (_, def) = common::find(doc.road_volumes(), &input.volume, "volume")?;
    budget(doc, def, input)
}

/// The budget of a volume as typed (the desktop's Kübaj window, before it is saved).
pub fn preview(
    doc: &Document,
    def: &kentos_commands::VolumeDef,
    input: &VolumesUncertainty,
) -> Result<(VolumeUncertaintyReport, Vec<CommandWarning>), String> {
    let words = |stop: Stop| match stop {
        Stop::Failed(e) | Stop::Conflict(e) => e.message,
        Stop::Cancelled => String::new(),
    };
    common::road_project(doc).map_err(words)?;
    judged(input).map_err(words)?;
    budget(doc, def, input).map_err(words)
}

fn budget(
    doc: &Document,
    def: &kentos_commands::VolumeDef,
    input: &VolumesUncertainty,
) -> Result<(VolumeUncertaintyReport, Vec<CommandWarning>), Stop> {
    let (rep, mut warnings) = common::compute(doc, def)?;
    let cdef = doc.road_corridors().iter().find(|c| c.id == def.corridor).ok_or_else(|| {
        fail(codes::UNKNOWN_CORRIDOR, format!("“{}” kimlikli koridor projede yok.", def.corridor), "corridor")
    })?;
    let made = corridors::make(doc, cdef, false)
        .map_err(|stop| fail(codes::INVALID_VOLUME, format!("“{}” koridoru kurulamıyor: {}", cdef.name, corridors::words(stop)), "corridor"))?;
    let Some(surface) = &made.sampler.surface else {
        return Err(fail(
            codes::INVALID_VOLUME,
            format!("“{}” koridorunun yüzeyi kurulamıyor; kotların hacme etkisi okunamaz.", cdef.name),
            "corridor",
        ));
    };
    let (sigma, beta) = (input.sigma, input.systematic.unwrap_or(0.0));
    let near = 2.0 * sigma;
    let all = sensitivity(&made.built.triangles, &surface.vertices, &surface.triangles, near);
    let net: Vec<f64> = all.cut.iter().zip(&all.fill).map(|(c, f)| c - f).collect();
    let spread_of = |value: f64, weights: &[f64]| {
        let s = spread(weights, sigma);
        let b = bias(weights, beta);
        let bound = bound95(s, b);
        VolumeSpread { value, sigma: s, bias: b, bound, accepted: input.accept.map(|p| bound <= p / 100.0 * value.abs() + 1e-9) }
    };
    let (cut, fill) = (spread_of(rep.cut, &all.cut), spread_of(rep.fill, &all.fill));
    let net = spread_of(rep.cut - rep.fill, &net);
    let near_share = if all.covered > 0.0 { all.near_area / all.covered } else { 0.0 };
    let points = all.cut.iter().zip(&all.fill).filter(|(c, f)| c.abs() + f.abs() > 0.0).count() as u32;
    // The method's share: the volume by the other method (the curve correction is the end areas' own).
    let other = match def.method {
        VolumeMethod::EndArea => VolumeMethod::Prismoidal,
        VolumeMethod::Prismoidal | VolumeMethod::Surfaces => VolumeMethod::EndArea,
    };
    let other_def = kentos_commands::VolumeDef { method: other, curve: def.curve && other == VolumeMethod::EndArea, ..def.clone() };
    let other_rep = common::compute(doc, &other_def).ok().map(|(r, _)| r);
    let terms: Vec<f64> = rep.rows.iter().flat_map(|r| [r.cut, r.fill]).flatten().collect();
    let numeric = numeric_bound(terms.len(), terms.iter().map(|v| v.abs()).sum());
    let text = |s: f64| km_text(made.sampler.alignment.km(s), 3);
    let stretches = made
        .built
        .strips
        .iter()
        .map(|(_, from, to, range)| {
            let s = sensitivity(&made.built.triangles[range.clone()], &surface.vertices, &surface.triangles, near);
            StretchSpread {
                from: *from,
                to: *to,
                from_text: text(*from),
                to_text: text(*to),
                cut_sigma: spread(&s.cut, sigma),
                fill_sigma: spread(&s.fill, sigma),
            }
        })
        .collect();
    if near_share > NEAR_SHARE {
        warnings.push(CommandWarning {
            code: codes::VOLUME_UNCERTAINTY_NEAR_ZERO.into(),
            message: format!(
                "“{}” kübajında alanın %{:.0}'i sıfır çizgisinin 2σ yakınında: kazı ve dolgunun σ'sı ayrı ayrı yalnız yaklaşık (netinki kesin).",
                def.name,
                100.0 * near_share
            ),
            path: Some("sigma".into()),
        });
    }
    let over: Vec<&str> = [("kazı", &cut), ("dolgu", &fill)].into_iter().filter(|(_, s)| s.accepted == Some(false)).map(|(w, _)| w).collect();
    if !over.is_empty() {
        warnings.push(CommandWarning {
            code: codes::VOLUME_UNCERTAINTY_OVER.into(),
            message: format!(
                "“{}” kübajında {} %95 sınırı kabul payını (%{}) aşıyor.",
                def.name,
                over.join(" ve "),
                input.accept.map_or_else(String::new, |p| format!("{p}"))
            ),
            path: Some("accept".into()),
        });
    }
    Ok((
        VolumeUncertaintyReport {
            volume: def.id.clone(),
            method: def.method,
            sigma,
            systematic: input.systematic,
            cut,
            fill,
            net,
            cut_area: all.cut_area,
            fill_area: all.fill_area,
            near_share,
            points,
            other_method: other_rep.as_ref().map(|_| other),
            other_cut: other_rep.as_ref().map(|r| r.cut),
            other_fill: other_rep.as_ref().map(|r| r.fill),
            numeric,
            stretches,
        },
        warnings,
    ))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &VolumesUncertainty) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &VolumesUncertainty) -> CommandResult<VolumeUncertaintyReport> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: VolumesUncertainty) -> CommandResult<VolumeUncertaintyReport> {
    plan(cx, &input)
}
