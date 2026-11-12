//! `road.terrains.repair` v1 (plan/road.md Y18.6, docs/adr/0916 §6; ROAD-TIN-17): point records left out of a terrain
//! surface or taken back, by place; the drawing's points stay, a breakline's vertex is never left out. The plan reads
//! the difference first (`kentos_road::volume::difference`, after − before); execute writes the surfaces in one undo
//! step “Yüzeyi onar”. The shared cases are `fixtures/commands/v1/road.terrains.repair.json`.

use kentos_commands::{
    CommandResult, CommandWarning, SurfaceExclusion, TerrainDifferenceAt, TerrainSurfaceDef, TerrainsRepair, TerrainsRepaired,
};
use kentos_domain::Document;
use kentos_road::surface::Surface;
use kentos_road::volume::{self, Triangle};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

/// The undo step's name.
pub const STEP: &str = "Yüzeyi onar";

fn invalid(message: String, path: &str) -> Stop {
    fail(codes::INVALID_REPAIR, message, path)
}

fn key(p: [f64; 2]) -> (u64, u64) {
    ((p[0] + 0.0).to_bits(), (p[1] + 0.0).to_bits())
}

fn triangles(s: &Surface) -> Vec<Triangle> {
    s.triangles.iter().map(|t| t.map(|i| s.vertices[i as usize])).collect()
}

fn count(n: usize) -> u32 {
    u32::try_from(n).unwrap_or(u32::MAX)
}

struct Checked {
    list: Vec<TerrainSurfaceDef>,
    out: TerrainsRepaired,
}

fn check(doc: &Document, input: &TerrainsRepair) -> Result<Checked, Stop> {
    terrains::terrain_project(doc)?;
    if input.exclude.is_empty() && input.restore.is_empty() {
        return Err(invalid("Bir şey sorun: dışarıda bırakılacak (exclude) ya da geri alınacak (restore) yerler.".into(), ""));
    }
    let mut seen = std::collections::BTreeSet::new();
    for (list, name) in [(&input.exclude, "exclude"), (&input.restore, "restore")] {
        for (k, p) in list.iter().enumerate() {
            if !(p[0].is_finite() && p[1].is_finite()) {
                return Err(invalid("Yer sonlu iki sayı olmalı.".into(), &format!("{name}[{k}]")));
            }
            if !seen.insert(key(*p)) {
                return Err(invalid(format!("({}, {}) iki kez verildi.", p[0], p[1]), &format!("{name}[{k}]")));
            }
        }
    }
    let id = crate::road_profiles::surface_id(doc, &input.surface, "surface")?;
    let own = terrains::surfaces(doc).to_vec();
    let at = own.iter().position(|s| s.id == id).ok_or(Stop::Cancelled)?;
    let before_def = own[at].clone();
    let left: std::collections::BTreeSet<(u64, u64)> = before_def.excluded.iter().map(|e| key([e.x, e.y])).collect();
    for (k, p) in input.restore.iter().enumerate() {
        if !left.contains(&key(*p)) {
            return Err(invalid(format!("({}, {}) dışarıda bırakılmamış; geri alınacak bir şey yok.", p[0], p[1]), &format!("restore[{k}]")));
        }
    }
    for (k, p) in input.exclude.iter().enumerate() {
        if left.contains(&key(*p)) {
            return Err(invalid(format!("({}, {}) zaten dışarıda bırakılmış.", p[0], p[1]), &format!("exclude[{k}]")));
        }
    }
    // Every record of the sources, nothing left out: what a place may be.
    let whole = terrains::make(doc, &TerrainSurfaceDef { excluded: Vec::new(), ..before_def.clone() });
    let vertices: std::collections::BTreeSet<(u64, u64)> =
        whole.gathered.input.breaklines.iter().flat_map(|b| b.points.iter().map(|p| key([p[0], p[1]]))).collect();
    let records: std::collections::BTreeSet<(u64, u64)> = whole.gathered.input.points.iter().map(|p| key([p.x, p.y])).collect();
    for (k, p) in input.exclude.iter().enumerate() {
        let path = format!("exclude[{k}]");
        if vertices.contains(&key(*p)) {
            return Err(fail(
                codes::REPAIR_BREAKLINE,
                format!("({}, {}) bir kırık hattın köşesi; onarım kırık hattı ve tasarım noktasını değiştirmez.", p[0], p[1]),
                &path,
            ));
        }
        if !records.contains(&key(*p)) {
            return Err(fail(codes::REPAIR_NOT_A_POINT, format!("({}, {}) yerinde yüzeyin bir nokta kaydı yok.", p[0], p[1]), &path));
        }
    }
    let restore: std::collections::BTreeSet<(u64, u64)> = input.restore.iter().map(|p| key(*p)).collect();
    let mut after_def = before_def.clone();
    after_def.excluded.retain(|e| !restore.contains(&key([e.x, e.y])));
    after_def.excluded.extend(input.exclude.iter().map(|p| SurfaceExclusion { x: p[0], y: p[1] }));
    let made_before = terrains::make(doc, &before_def);
    let made_after = terrains::make(doc, &after_def);
    let (sb, sa) = (made_before.built()?, made_after.built()?);
    // An accepted surface stays accepted: the repair is the user's.
    if before_def.built.as_deref() == Some(made_before.gathered.digest.as_str()) {
        after_def.built = Some(made_after.gathered.digest.clone());
    }
    let d = volume::difference(&triangles(sa), &triangles(sb), &[]);
    let largest = match (d.least, d.largest) {
        (Some(l), Some(h)) => Some(if l.0.abs() > h.0.abs() { l } else { h }),
        (l, h) => l.or(h),
    }
    .filter(|e| e.0 != 0.0)
    .map(|(difference, at)| TerrainDifferenceAt { difference, at });
    let mut list = own;
    list[at] = after_def.clone();
    Ok(Checked {
        out: TerrainsRepaired {
            surface: id,
            excluded: after_def.excluded.iter().map(|e| [e.x, e.y]).collect(),
            corners_before: count(sb.vertices.len()),
            corners_after: count(sa.vertices.len()),
            triangles_before: count(sb.triangles.len()),
            triangles_after: count(sa.triangles.len()),
            cut: d.volumes.cut,
            fill: d.volumes.fill,
            net: d.volumes.fill - d.volumes.cut,
            changed_area: (d.volumes.covered - zero_area(&triangles(sa), &triangles(sb))).max(0.0),
            largest,
            revision: doc.revision().to_string(),
        },
        list,
    })
}

/// The plan area of the pieces where the difference is zero throughout.
fn zero_area(after: &[Triangle], before: &[Triangle]) -> f64 {
    let mut ground = volume::Ground::new(before);
    let mut flat = 0.0;
    ground.overlay(after, |with| {
        if with.iter().all(|(_, d)| *d == 0.0) {
            let n = with.len();
            let mut a = 0.0;
            for i in 0..n {
                let (p, q) = (with[i].0, with[(i + 1) % n].0);
                a += p[0] * q[1] - q[0] * p[1];
            }
            flat += a / 2.0;
        }
    });
    flat
}

fn write(cx: &mut ExecutionContext<'_>, c: Checked) -> CommandResult<TerrainsRepaired> {
    match cx.doc.set_road_surfaces(c.list, STEP) {
        Ok(_) => CommandResult::Completed {
            output: TerrainsRepaired { revision: cx.doc.revision().to_string(), ..c.out },
            warnings: Vec::<CommandWarning>::new(),
        },
        Err(refusal) => fail(codes::INVALID_REPAIR, refusal.0, "").into(),
    }
}

/// Checks `input` against the document, writing nothing.
pub fn validate(cx: &ExecutionContext<'_>, input: &TerrainsRepair) -> CommandResult<()> {
    match check(cx.doc, input).and_then(|c| checks::revision(cx.doc, input.expected_revision.as_deref()).map(|_| c)) {
        Ok(_) => CommandResult::Completed { output: (), warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// The repair's difference, written nowhere.
pub fn plan(cx: &ExecutionContext<'_>, input: &TerrainsRepair) -> CommandResult<TerrainsRepaired> {
    match check(cx.doc, input).and_then(|c| checks::revision(cx.doc, input.expected_revision.as_deref()).map(|_| c)) {
        Ok(c) => CommandResult::Completed { output: c.out, warnings: Vec::new() },
        Err(stop) => stop.into(),
    }
}

/// Writes the surfaces: one undo step.
pub fn execute(cx: &mut ExecutionContext<'_>, input: TerrainsRepair) -> CommandResult<TerrainsRepaired> {
    crate::call::recorded(cx, kentos_commands::ROAD_TERRAINS_REPAIR, kentos_commands::ROAD_TERRAINS_REPAIR_VERSION, input, |cx, input| {
        match check(cx.doc, &input).and_then(|c| checks::revision(cx.doc, input.expected_revision.as_deref()).map(|_| c)) {
            Ok(c) => write(cx, c),
            Err(stop) => stop.into(),
        }
    })
}

/// The repair's difference for a window, or why it is not one.
pub fn preview(doc: &Document, input: &TerrainsRepair) -> Result<TerrainsRepaired, String> {
    check(doc, input).map(|c| c.out).map_err(crate::road_corridors::words)
}
