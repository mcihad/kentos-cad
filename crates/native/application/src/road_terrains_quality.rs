//! `road.terrains.quality` v1 (plan/road.md Y18.4, docs/adr/0916 §4; ROAD-SURVEY-04, 05): a terrain surface's survey
//! quality, read now with `kentos_road::surface`'s quality: corners nearly at one place, spikes, coverage gaps and
//! check points against the surface by class. Nothing is written; the raw records stay. The shared cases are
//! `fixtures/commands/v1/road.terrains.quality.json`.

use std::collections::BTreeMap;

use kentos_commands::{
    CommandResult, CommandWarning, Entity, MOST_QUALITY_ITEMS, TerrainCheckClass, TerrainCheckRobust, TerrainCheckpoint,
    TerrainCheckpoints,
    TerrainGaps, TerrainNearPair, TerrainSpike, TerrainsQuality, TerrainsQualityRead,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

fn invalid(message: String, path: &str) -> Stop {
    fail(codes::INVALID_ANALYSIS, message, path)
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

/// The query's own values judged, before anything is read.
fn judged(input: &TerrainsQuality) -> Result<(), Stop> {
    if input.near.is_none() && input.spikes.is_none() && input.gaps.is_none() && input.checkpoints.is_none() {
        return Err(invalid("Bir şey sorun: near, spikes, gaps ya da checkpoints.".into(), ""));
    }
    let positive = |v: f64| v.is_finite() && v > 0.0;
    if input.near.is_some_and(|v| !positive(v)) {
        return Err(invalid("Yakın noktaların uzaklığı sıfırdan büyük olmalı.".into(), "near"));
    }
    if let Some(s) = &input.spikes {
        if !positive(s.radius) {
            return Err(invalid("Sivri noktanın komşu erimi sıfırdan büyük olmalı.".into(), "spikes.radius"));
        }
        if !(s.limit.is_finite() && s.limit >= 0.0) {
            return Err(invalid("Sivri noktanın sınırı eksi olmayan bir sayı olmalı.".into(), "spikes.limit"));
        }
    }
    if input.gaps.is_some_and(|v| !positive(v)) {
        return Err(invalid("Boşluk sayılan kenar boyu sıfırdan büyük olmalı.".into(), "gaps"));
    }
    if let Some(c) = &input.checkpoints
        && c.layer.is_none()
        && c.uids.is_empty()
    {
        return Err(invalid("Kontrol noktalarını verin: bir katman (layer) ya da nesneler (uids).".into(), "checkpoints"));
    }
    if input.checkpoints.as_ref().and_then(|c| c.sigma).is_some_and(|s| !(s.is_finite() && s > 0.0 && s <= 10.0)) {
        return Err(invalid("Kontrol noktalarının kendi doğruluğu 0'dan büyük, en çok 10 m olmalı.".into(), "checkpoints.sigma"));
    }
    Ok(())
}

/// The check points asked: the layer's points in the document's order, then the uids' not already taken.
fn checkpoints(doc: &Document, ask: &kentos_commands::TerrainCheckpointsAsk) -> Result<Vec<(String, [f64; 3], String)>, Stop> {
    let class_of = |e: &Entity| {
        ask.class_field.as_ref().and_then(|f| e.base().attrs.get(f).cloned()).unwrap_or_default()
    };
    let point = |uid: &str, e: &Entity, path: &str| match e {
        Entity::Point(p) => match p.z {
            Some(z) => Ok([p.p.x, p.p.y, z]),
            None => Err(invalid(format!("“{uid}” kontrol noktasının kotu yok."), path)),
        },
        _ => Err(invalid(format!("“{uid}” bir nokta değil; kontrol noktası kotlu bir nokta olmalı."), path)),
    };
    let mut out: Vec<(String, [f64; 3], String)> = Vec::new();
    if let Some(layer) = &ask.layer {
        checks::layer_at(doc, layer, "checkpoints.layer").map(|_| ())?;
        for (uid, e) in doc.entities_with_uids() {
            if e.base().layer_id == *layer && matches!(e, Entity::Point(_)) {
                let uid = uid.to_string();
                let p = point(&uid, e, "checkpoints.layer")?;
                out.push((uid, p, class_of(e)));
            }
        }
    }
    for (k, uid) in ask.uids.iter().enumerate() {
        if out.iter().any(|(u, _, _)| u == uid) {
            continue;
        }
        let path = format!("checkpoints.uids[{k}]");
        let Some(e) = doc.entities_with_uids().find(|(u, _)| u.to_string() == *uid).map(|(_, e)| e) else {
            return Err(fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), &path));
        };
        let p = point(uid, e, &path)?;
        out.push((uid.clone(), p, class_of(e)));
    }
    Ok(out)
}

fn statistics(class: &str, differences: &[f64], outside: usize, sigma: Option<f64>) -> TerrainCheckClass {
    let n = differences.len();
    let mean = if n > 0 { differences.iter().sum::<f64>() / n as f64 } else { 0.0 };
    let std = if n > 1 { (differences.iter().map(|d| (d - mean).powi(2)).sum::<f64>() / (n - 1) as f64).sqrt() } else { 0.0 };
    let rmse = if n > 0 { (differences.iter().map(|d| d * d).sum::<f64>() / n as f64).sqrt() } else { 0.0 };
    TerrainCheckClass {
        class: class.to_owned(),
        count: count(n),
        outside: count(outside),
        mean,
        std,
        rmse,
        min: if n > 0 { differences.iter().copied().fold(f64::INFINITY, f64::min) } else { 0.0 },
        max: if n > 0 { differences.iter().copied().fold(f64::NEG_INFINITY, f64::max) } else { 0.0 },
        robust: kentos_road::accuracy::accuracy(differences, sigma).map(|a| TerrainCheckRobust {
            rmse_low: a.rmse_low,
            rmse_high: a.rmse_high,
            median: a.median,
            nmad: a.nmad,
            q68: a.q68,
            q95: a.q95,
            skewness: a.skewness,
            kurtosis: a.kurtosis,
            far: count(a.far.len()),
            combined: a.combined,
        }),
    }
}

/// The quality now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &TerrainsQuality) -> Result<(TerrainsQualityRead, Vec<CommandWarning>), Stop> {
    terrains::road_project(doc)?;
    judged(input)?;
    let id = crate::road_profiles::surface_id(doc, &input.surface, "surface")?;
    let def = terrains::surfaces(doc).iter().find(|s| s.id == id).cloned().ok_or(Stop::Cancelled)?;
    let made = terrains::make(doc, &def);
    let surface = made.built()?;
    let points = match &input.checkpoints {
        Some(ask) => Some(checkpoints(doc, ask)?),
        None => None,
    };
    let mut warnings = Vec::new();
    let mut out = TerrainsQualityRead {
        surface: id,
        corners: count(surface.vertices.len()),
        triangles: count(surface.triangles.len()),
        near_count: None,
        near: Vec::new(),
        spike_count: None,
        spikes: Vec::new(),
        gaps: None,
        checkpoints: None,
    };
    if let Some(near) = input.near {
        let pairs = surface.near_pairs(near);
        out.near_count = Some(count(pairs.len()));
        out.near = pairs
            .into_iter()
            .take(MOST_QUALITY_ITEMS)
            .map(|p| TerrainNearPair { a: p.a, b: p.b, distance: p.distance, dz: p.dz })
            .collect();
    }
    if let Some(ask) = &input.spikes {
        let spikes = surface.spikes(ask.radius, ask.limit);
        out.spike_count = Some(count(spikes.len()));
        out.spikes = spikes
            .into_iter()
            .take(MOST_QUALITY_ITEMS)
            .map(|s| TerrainSpike { at: s.at, mean: s.mean, difference: s.difference, neighbours: count(s.neighbours) })
            .collect();
    }
    if let Some(limit) = input.gaps {
        let g = surface.gaps(limit);
        out.gaps = Some(TerrainGaps { triangles: count(g.triangles), area: g.area, longest: g.longest });
    }
    if let Some(points) = points {
        let mut by: BTreeMap<String, (Vec<f64>, usize)> = BTreeMap::new();
        let (mut every, mut off) = (Vec::new(), 0usize);
        let mut list = Vec::with_capacity(points.len());
        let mut start = 0u32;
        for (uid, p, class) in points {
            let h = surface.height_walking(p[0], p[1], &mut start);
            let entry = by.entry(class.clone()).or_default();
            let difference = h.z.map(|z| p[2] - z);
            match difference {
                Some(d) => {
                    entry.0.push(d);
                    every.push(d);
                }
                None => {
                    entry.1 += 1;
                    off += 1;
                }
            }
            list.push(TerrainCheckpoint { uid, at: [p[0], p[1]], z: p[2], class, surface_z: h.z, difference, far: false });
        }
        let sigma = input.checkpoints.as_ref().and_then(|a| a.sigma);
        // A point is far by its own class's median and NMAD (docs/adr/0945): vegetated ground spreads more.
        let far_by: BTreeMap<&String, (f64, f64)> = by
            .iter()
            .filter_map(|(c, (d, _))| kentos_road::accuracy::accuracy(d, None).map(|a| (c, (a.median, a.nmad))))
            .collect();
        let mut far = 0usize;
        for p in &mut list {
            if let (Some(d), Some((median, nmad))) = (p.difference, far_by.get(&p.class)) {
                p.far = *nmad > 0.0 && (d - median).abs() > kentos_road::accuracy::FAR * nmad;
                far += usize::from(p.far);
            }
        }
        out.checkpoints = Some(TerrainCheckpoints {
            points: list,
            classes: by.iter().map(|(c, (d, o))| statistics(c, d, *o, sigma)).collect(),
            all: statistics("", &every, off, sigma),
        });
        if far > 0 {
            warnings.push(CommandWarning {
                code: codes::TERRAIN_CHECKPOINTS_FAR.into(),
                message: format!(
                    "{far} kontrol noktası sınıfının medyanından 3 NMAD'den uzak: ölçü ya da sınıf hatası olabilir; RMSE onlardan etkilenir, NMAD etkilenmez."
                ),
                path: Some("checkpoints".into()),
            });
        }
    }
    Ok((out, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsQuality) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsQuality) -> CommandResult<TerrainsQualityRead> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsQuality) -> CommandResult<TerrainsQualityRead> {
    plan(cx, &input)
}

/// The quality for a window, or why it does not read.
pub fn preview(doc: &Document, input: &TerrainsQuality) -> Result<(TerrainsQualityRead, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
