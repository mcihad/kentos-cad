//! `cad.pointclouds.overlap` v1 (plan/map-cad.md MAP-13, docs/adr/1500): Tarama uyumu denetimi, read only. The two
//! clouds are read once at full resolution (`pointcloud_areas::Scan`); each point adds to its cell's plane sums and to
//! the sums around every control point within reach. A plane z = c₀ + c₁u + c₂v is fitted by least squares with u, v
//! measured from the cell's centre (a control point's place), so c₀ is the plane's height there; it is solved by
//! Cramer's rule, and a cell whose points lie on a line (the determinant under 10⁻⁹·n³·h⁴, h half the cell or the
//! reach) has no plane. The shared cases are `fixtures/commands/v1/cad.pointclouds.overlap.json`.

use std::collections::BTreeMap;

use kentos_commands::{
    CloudRules, CommandResult, CommandWarning, Entity, LayerNodeType, OverlapCell, OverlapControl, OverlapControlStats,
    PointCloudsOverlap, PointCloudsOverlapped, cloud_rules_default,
};
use kentos_domain::Document;

use crate::ExecutionContext;
use crate::checks::{self, Stop, error};
use crate::codes;
use crate::pointcloud_areas::Scan;
use crate::pointclouds::clouds;

const FIELD: &str = "Ad";
const MOST_WORST: usize = 20;

fn fail(code: &str, message: String, path: &str) -> Stop {
    Stop::failed(error(code, message, Some(path.to_owned())))
}

/// A plane's least-squares sums about a centre (`cad.pointclouds.register`'s control planes too).
#[derive(Clone, Copy, Default)]
pub(crate) struct Sums {
    n: f64,
    u: f64,
    v: f64,
    uu: f64,
    uv: f64,
    vv: f64,
    z: f64,
    uz: f64,
    vz: f64,
}

impl Sums {
    pub(crate) fn add(&mut self, u: f64, v: f64, z: f64) {
        self.n += 1.0;
        self.u += u;
        self.v += v;
        self.uu += u * u;
        self.uv += u * v;
        self.vv += v * v;
        self.z += z;
        self.uz += u * z;
        self.vz += v * z;
    }

    /// The plane's height at the centre; none with fewer than `least` points or points on a line.
    pub(crate) fn height(&self, least: u32, h: f64) -> Option<f64> {
        if self.n < f64::from(least) {
            return None;
        }
        let det3 = |a: [f64; 9]| {
            a[0] * (a[4] * a[8] - a[5] * a[7]) - a[1] * (a[3] * a[8] - a[5] * a[6]) + a[2] * (a[3] * a[7] - a[4] * a[6])
        };
        let m = [self.n, self.u, self.v, self.u, self.uu, self.uv, self.v, self.uv, self.vv];
        let d = det3(m);
        if d.abs() <= 1e-9 * self.n.powi(3) * h.powi(4) {
            return None;
        }
        let mz = [self.z, self.u, self.v, self.uz, self.uu, self.uv, self.vz, self.uv, self.vv];
        Some(det3(mz) / d)
    }
}

fn run(doc: &Document, input: &PointCloudsOverlap) -> Result<(PointCloudsOverlapped, Vec<CommandWarning>), Stop> {
    if input.uids.len() != 2 || input.uids[0] == input.uids[1] {
        return Err(fail(codes::INVALID_INPUT, "İki ayrı nokta bulutu verin: karşılaştırılacak iki tarama.".into(), "uids"));
    }
    checks::uids(&input.uids, "")?;
    let chosen = clouds(doc, &input.uids)?;
    let cell = input.cell.unwrap_or(2.0);
    if !cell.is_finite() || cell <= 0.0 {
        return Err(fail(codes::INVALID_INPUT, format!("Hücre kenarı sonlu ve sıfırdan büyük olmalı (yazılan {cell})."), "cell"));
    }
    let least = input.least.unwrap_or(4);
    if least < 3 {
        return Err(fail(codes::INVALID_INPUT, "Bir düzlem için hücrede en az 3 nokta gerekir.".into(), "least"));
    }
    let radius = input.radius.unwrap_or(cell);
    if !radius.is_finite() || radius <= 0.0 {
        return Err(fail(codes::INVALID_INPUT, format!("Kontrol noktasının çevresi sonlu ve sıfırdan büyük olmalı (yazılan {radius})."), "radius"));
    }
    let scale = match input.scale {
        Some(0) => return Err(fail(codes::INVALID_INPUT, "Ölçek paydası 0 olamaz.".into(), "scale")),
        Some(s) => s,
        None => doc.settings().plot_scale.round().max(1.0) as u32,
    };
    let rules: CloudRules = match &input.rules {
        Some(r) => r.clone(),
        None => cloud_rules_default().ok_or_else(|| fail(codes::INVALID_INPUT, "Yerleşik kural paketi okunamadı.".into(), "rules"))?,
    };
    if rules.id.trim().is_empty() || rules.source.trim().is_empty() {
        return Err(fail(codes::INVALID_INPUT, "Kural paketinin kimliği ya da kaynağı boş; sınırın dayanağı yazılmadan denetim yapılmaz.".into(), "rules"));
    }
    if rules.xy.iter().chain([&rules.z_factor, &rules.interswath]).any(|v| !v.is_finite()) {
        return Err(fail(codes::INVALID_INPUT, "Kural paketinin bir sınırı sonlu bir sayı değil.".into(), "rules"));
    }
    // The control points: named, with a height.
    let field = input.field.as_deref().map(str::trim).filter(|f| !f.is_empty()).unwrap_or(FIELD);
    let mut controls: Vec<(String, f64, f64, f64)> = Vec::new();
    if let Some(layer) = input.control.as_deref().map(str::trim) {
        if !doc.layers().get(layer).is_some_and(|n| n.kind == LayerNodeType::Layer) {
            return Err(fail(codes::LAYER_NOT_FOUND, format!("“{layer}” katmanı çizimde yok."), "control"));
        }
        for e in doc.entities().filter(|e| e.base().layer_id == layer) {
            let Entity::Point(p) = e else { continue };
            let (Some(name), Some(z)) = (e.base().attrs.get(field).map(|v| v.trim().to_owned()).filter(|v| !v.is_empty()), p.z) else {
                continue;
            };
            if !controls.iter().any(|c| c.0 == name) {
                controls.push((name, p.p.x, p.p.y, z));
            }
        }
    }
    let classes = input.classes.clone();
    let boxes: Vec<[f64; 4]> = chosen.iter().map(|(_, c)| [c.cloud.bounds[0], c.cloud.bounds[1], c.cloud.bounds[3], c.cloud.bounds[4]]).collect();
    let mut cells: [BTreeMap<(i64, i64), Sums>; 2] = [BTreeMap::new(), BTreeMap::new()];
    let mut around: Vec<[Sums; 2]> = vec![[Sums::default(); 2]; controls.len()];
    let mut taken = [0u64; 2];
    let mut warnings = Vec::new();
    let mut read = vec![0u64; 2];
    Scan { doc, drawing: input.drawing.as_deref(), chosen: &chosen, boxes: &boxes, what: "Bu karşılaştırma", path: "uids" }.run(
        &mut warnings,
        &mut read,
        &mut |s| {
            if let Some(cs) = &classes
                && !cs.contains(&s.layout.class(s.record))
            {
                return;
            }
            let [x, y, z] = s.xyz;
            let k = s.cloud;
            taken[k] += 1;
            let (i, j) = ((x / cell).floor() as i64, (y / cell).floor() as i64);
            let (cx, cy) = ((i as f64 + 0.5) * cell, (j as f64 + 0.5) * cell);
            cells[k].entry((i, j)).or_default().add(x - cx, y - cy, z);
            for (c, sums) in controls.iter().zip(around.iter_mut()) {
                let (u, v) = (x - c.1, y - c.2);
                if u.abs() <= radius && v.abs() <= radius && u.hypot(v) <= radius {
                    sums[k].add(u, v, z);
                }
            }
        },
    )?;
    let h = cell / 2.0;
    let planes: [BTreeMap<(i64, i64), Option<f64>>; 2] =
        [0, 1].map(|k| cells[k].iter().map(|(key, s)| (*key, s.height(least, h))).collect());
    let filled = [0, 1].map(|k| planes[k].values().filter(|p| p.is_some()).count() as u32);
    let mut diffs: Vec<((i64, i64), f64)> = Vec::new();
    let mut degenerate = 0u32;
    for (key, sa) in &cells[0] {
        let Some(sb) = cells[1].get(key) else { continue };
        if sa.n < f64::from(least) || sb.n < f64::from(least) {
            continue;
        }
        match (planes[0][key], planes[1][key]) {
            (Some(a), Some(b)) => diffs.push((*key, b - a)),
            _ => degenerate += 1,
        }
    }
    let n = diffs.len() as f64;
    let (mean, rmsd, std, max) = if diffs.is_empty() {
        (0.0, 0.0, 0.0, 0.0)
    } else {
        let mean = diffs.iter().map(|d| d.1).sum::<f64>() / n;
        (
            mean,
            (diffs.iter().map(|d| d.1 * d.1).sum::<f64>() / n).sqrt(),
            (diffs.iter().map(|d| (d.1 - mean) * (d.1 - mean)).sum::<f64>() / n).sqrt(),
            diffs.iter().map(|d| d.1.abs()).fold(0.0, f64::max),
        )
    };
    let mut worst = diffs.clone();
    worst.sort_by(|a, b| b.1.abs().total_cmp(&a.1.abs()).then(a.0.cmp(&b.0)));
    let worst: Vec<OverlapCell> = worst
        .into_iter()
        .take(MOST_WORST)
        .map(|((i, j), dz)| OverlapCell { x: (i as f64 + 0.5) * cell, y: (j as f64 + 0.5) * cell, dz })
        .collect();
    let s = f64::from(scale);
    let limit_z = rules.z_factor * (rules.xy[0] * s * s + rules.xy[1] * s + rules.xy[2]) / 100.0;
    let control: Vec<OverlapControl> = controls
        .iter()
        .zip(&around)
        .map(|(c, sums)| OverlapControl {
            name: c.0.clone(),
            z: c.3,
            dz_a: sums[0].height(least, radius).map(|p| p - c.3),
            dz_b: sums[1].height(least, radius).map(|p| p - c.3),
        })
        .collect();
    let stats = |pick: fn(&OverlapControl) -> Option<f64>| -> Option<OverlapControlStats> {
        let v: Vec<f64> = control.iter().filter_map(pick).collect();
        (!controls.is_empty()).then(|| {
            let k = v.len() as f64;
            let koh = if v.is_empty() { 0.0 } else { (v.iter().map(|d| d * d).sum::<f64>() / k).sqrt() };
            OverlapControlStats {
                n: v.len() as u32,
                koh_z: koh,
                mean: if v.is_empty() { 0.0 } else { v.iter().sum::<f64>() / k },
                passed: (!v.is_empty()).then_some(koh <= limit_z),
            }
        })
    };
    let control_a = stats(|c| c.dz_a);
    let control_b = stats(|c| c.dz_b);
    let mut notes = vec![
        "Her hücrede bulutun noktalarına düzlem uydurulur; fark iki düzlemin hücre merkezindeki değerlerinin farkıdır (ikinci − birinci), eğim fark sayılmaz.".to_owned(),
        format!("Şeritler arası sınır {} paketinin örnek değeridir; Türk mevzuatında LiDAR şerit uyumu ölçütü yok, kurum kendi değerini verir.", rules.id),
    ];
    if controls.is_empty() {
        notes.push("Kontrol noktası yok: bulutların mutlak doğruluğu bilinmiyor; bulut ölçüye kesin hazır sayılmaz.".to_owned());
    }
    if diffs.is_empty() {
        notes.push("Örtüşen hücre yok: iki bulut aynı yeri yeterli noktayla kaplamıyor.".to_owned());
    }
    Ok((
        PointCloudsOverlapped {
            rules: rules.id.clone(),
            source: rules.source.clone(),
            cell,
            read: [read[0], read[1]],
            taken,
            filled,
            cells: diffs.len() as u32,
            area: n * cell * cell,
            degenerate,
            rmsd_z: rmsd,
            mean,
            std,
            max,
            limit: rules.interswath,
            passed: (!diffs.is_empty()).then_some(rmsd <= rules.interswath),
            worst,
            control,
            control_a,
            control_b,
            limit_z,
            notes,
        },
        warnings,
    ))
}

pub fn validate(cx: &ExecutionContext<'_>, input: &PointCloudsOverlap) -> CommandResult<()> {
    match run(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &PointCloudsOverlap) -> CommandResult<PointCloudsOverlapped> {
    match run(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: PointCloudsOverlap) -> CommandResult<PointCloudsOverlapped> {
    plan(cx, &input)
}
