//! `road.swept.query` v1 (plan/road.md Y17, docs/adr/0915 §2): a design vehicle driven along a path (places, a line,
//! polyline or arc of the drawing, or a junction's quadrant) with `kentos_road::swept`: its axles' and body corners'
//! tracks, its off-tracking and its least clearance to obstacles. Nothing is written. The shared cases are
//! `fixtures/commands/v1/road.swept.query.json`.
//!
//! A junction's quadrant: the front axle runs `offset` outward of the curb return (at its radius plus `offset`, round
//! its centre), `approach` metres straight along each leg's tangent before and after; the curb is the return at its
//! radius with the same straight runs. Arcs are taken as chords no longer than the step (`swept::arc_places`).

use kentos_commands::{
    CommandResult, CommandWarning, Entity, MOST_SWEPT_OBSTACLES, MOST_SWEPT_SAMPLES, SweptClearance, SweptQueried, SweptQuery,
};
use kentos_domain::Document;
use kentos_road::swept::{self, Trailer, Vehicle};

use crate::ExecutionContext;
use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::fail;

fn invalid(message: String, path: &str) -> Stop {
    fail(codes::INVALID_SWEPT, message, path)
}

fn vehicle_of(input: &SweptQuery) -> Vehicle {
    let v = &input.vehicle;
    Vehicle {
        wheelbase: v.wheelbase,
        front: v.front,
        rear: v.rear,
        width: v.width,
        trailer: v.trailer.as_ref().map(|t| Trailer {
            kingpin: t.kingpin,
            wheelbase: t.wheelbase,
            front: t.front,
            rear: t.rear,
            width: t.width,
        }),
    }
}

/// The query's own values judged, before anything is read.
fn judged(input: &SweptQuery) -> Result<(), Stop> {
    let given = usize::from(!input.path.is_empty()) + usize::from(input.uid.is_some()) + usize::from(input.junction.is_some());
    if given != 1 {
        return Err(invalid("Tek bir yol verin: path (noktalar), uid (çizimdeki nesne) ya da junction (kavşağın çeyreği).".into(), "path"));
    }
    let probe = swept::drive(&vehicle_of(input), &[[0.0, 0.0], [1.0, 0.0]], 1.0);
    if probe.is_err() {
        return Err(invalid(
            "Aracın dingil aralığı ve genişliği sıfırdan büyük, çıkıntıları eksi olmayan sonlu sayılar olmalı (römorkun da).".into(),
            "vehicle",
        ));
    }
    if !(input.step.is_finite() && input.step > 0.0) {
        return Err(invalid("Örnekleme aralığı (step) sıfırdan büyük olmalı.".into(), "step"));
    }
    if let Some(j) = &input.junction {
        if j.quadrant > 3 {
            return Err(invalid("Kavşağın çeyreği 0 ile 3 arası olmalı.".into(), "junction.quadrant"));
        }
        if !(j.offset.is_finite() && j.offset >= 0.0) {
            return Err(invalid("Bordürden uzaklık (offset) eksi olmayan bir sayı olmalı.".into(), "junction.offset"));
        }
        if !(j.approach.is_finite() && j.approach >= 0.0) {
            return Err(invalid("Kollardaki düz yol (approach) eksi olmayan bir sayı olmalı.".into(), "junction.approach"));
        }
    }
    if !input.path.is_empty() {
        if let Some(k) = input.path.iter().position(|p| !(p[0].is_finite() && p[1].is_finite())) {
            return Err(invalid("Yolun her yeri sonlu iki sayı olmalı.".into(), &format!("path[{k}]")));
        }
        let mut distinct = input.path.clone();
        distinct.dedup();
        if distinct.len() < 2 {
            return Err(invalid("Yolun en az iki ayrı yeri olmalı.".into(), "path"));
        }
    }
    if input.obstacles.len() > MOST_SWEPT_OBSTACLES {
        return Err(invalid(format!("Bir soruda en çok {MOST_SWEPT_OBSTACLES} engel okunur."), "obstacles"));
    }
    Ok(())
}

fn entity(doc: &Document, uid: &str, path: &str) -> Result<Entity, Stop> {
    doc.entities_with_uids()
        .find(|(u, _)| u.to_string() == uid)
        .map(|(_, e)| e.clone())
        .ok_or_else(|| fail(codes::ENTITY_NOT_FOUND, format!("“{uid}” kimlikli nesne çizimde yok."), path))
}

/// An arc entity's counter-clockwise sweep.
fn sweep_of(a0: f64, a1: f64) -> f64 {
    let s = (a1 - a0).rem_euclid(std::f64::consts::TAU);
    if s == 0.0 { std::f64::consts::TAU } else { s }
}

/// A polyline's places along its edges, closed back to its start when `closed`.
fn polyline_places(pts: &[[f64; 2]], bulges: &[f64], closed: bool, step: f64) -> Vec<[f64; 2]> {
    let mut out = vec![pts[0]];
    let n = pts.len();
    let edges = if closed { n } else { n.saturating_sub(1) };
    for i in 0..edges {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let bulge = bulges.get(i).copied().unwrap_or(0.0);
        out.extend(swept::bulge_places(a, b, bulge, step).into_iter().skip(1));
    }
    out
}

fn points(p: &[kentos_commands::Vec2]) -> Vec<[f64; 2]> {
    p.iter().map(|v| [v.x, v.y]).collect()
}

/// The drawing's object as the path.
fn path_of(doc: &Document, uid: &str, step: f64) -> Result<Vec<[f64; 2]>, Stop> {
    match entity(doc, uid, "uid")? {
        Entity::Line(l) => Ok(vec![[l.a.x, l.a.y], [l.b.x, l.b.y]]),
        Entity::Polyline(p) if !p.closed && p.parts.as_ref().is_none_or(|q| q.is_empty()) && p.pts.len() >= 2 => {
            Ok(polyline_places(&points(&p.pts), p.bulges.as_deref().unwrap_or(&[]), false, step))
        }
        Entity::Arc(a) => Ok(swept::arc_places([a.c.x, a.c.y], a.r, a.a0, sweep_of(a.a0, a.a1), step)),
        _ => Err(invalid(format!("“{uid}” yol olamaz: bir çizgi, açık tek parçalı çoklu çizgi ya da yay olmalı."), "uid")),
    }
}

/// The drawing's object as obstacle lines.
fn obstacle_of(doc: &Document, uid: &str, k: usize, step: f64) -> Result<Vec<Vec<[f64; 2]>>, Stop> {
    let path = format!("obstacles[{k}]");
    match entity(doc, uid, &path)? {
        Entity::Line(l) => Ok(vec![vec![[l.a.x, l.a.y], [l.b.x, l.b.y]]]),
        Entity::Polyline(p) if !p.pts.is_empty() => {
            let mut lines = vec![polyline_places(&points(&p.pts), p.bulges.as_deref().unwrap_or(&[]), p.closed, step)];
            for part in p.parts.iter().flatten() {
                if !part.pts.is_empty() {
                    lines.push(polyline_places(&points(&part.pts), part.bulges.as_deref().unwrap_or(&[]), p.closed, step));
                }
            }
            Ok(lines)
        }
        Entity::Arc(a) => Ok(vec![swept::arc_places([a.c.x, a.c.y], a.r, a.a0, sweep_of(a.a0, a.a1), step)]),
        Entity::Circle(c) => Ok(vec![swept::arc_places([c.c.x, c.c.y], c.r, 0.0, std::f64::consts::TAU, step)]),
        _ => Err(invalid(format!("“{uid}” engel olamaz: bir çizgi, çoklu çizgi, yay ya da daire olmalı."), &path)),
    }
}

/// The drawing's object as obstacle lines for a window's picture; none when it is not one.
pub fn obstacle_lines(doc: &Document, uid: &str, step: f64) -> Option<Vec<Vec<[f64; 2]>>> {
    obstacle_of(doc, uid, 0, step).ok()
}

/// A junction quadrant's path and curb.
fn quadrant(doc: &Document, j: &kentos_commands::SweptJunctionPath, step: f64) -> Result<(String, Vec<[f64; 2]>, Vec<[f64; 2]>), Stop> {
    crate::road_junctions::road_project(doc)?;
    let (_, def) = crate::road_junctions::find(doc.road_junctions(), &j.junction, "junction.junction")?;
    let (report, _) = crate::road_junctions::read(doc, def)?;
    let Some(r) = report.returns.get(j.quadrant as usize) else {
        return Err(fail(
            codes::INVALID_JUNCTION,
            format!("“{}” kavşağının bordür dönüşü yok (katlı kavşak); araç izi çeyreğin dönüşünde sürülür.", report.name),
            "junction.junction",
        ));
    };
    let c = [r.cx, r.cy];
    let start = (r.on_a.y - r.cy).atan2(r.on_a.x - r.cx);
    let sweep = r.sweep.to_radians();
    let turn = sweep.signum();
    let along = |a: f64| [-turn * a.sin(), turn * a.cos()];
    let (ta, tb) = (along(start), along(start + sweep));
    let run = |radius: f64| {
        let arc = swept::arc_places(c, radius, start, sweep, step);
        let (first, last) = (arc[0], arc[arc.len() - 1]);
        let mut out = vec![[first[0] - ta[0] * j.approach, first[1] - ta[1] * j.approach]];
        out.extend(arc);
        out.push([last[0] + tb[0] * j.approach, last[1] + tb[1] * j.approach]);
        out.dedup();
        out
    };
    Ok((report.id, run(r.radius + j.offset), run(r.radius)))
}

/// The query answered now, and its warnings.
pub(crate) fn answer(doc: &Document, input: &SweptQuery) -> Result<(SweptQueried, Vec<CommandWarning>), Stop> {
    judged(input)?;
    let step = input.step;
    let (mut path, curb) = if let Some(j) = &input.junction {
        let (id, path, curb) = quadrant(doc, j, step)?;
        (path, Some((id, curb)))
    } else if let Some(uid) = &input.uid {
        (path_of(doc, uid, step)?, None)
    } else {
        (input.path.clone(), None)
    };
    if input.reverse {
        path.reverse();
    }
    let mut lines: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut owner: Vec<(String, bool)> = Vec::new();
    for (k, uid) in input.obstacles.iter().enumerate() {
        for line in obstacle_of(doc, uid, k, step)? {
            lines.push(line);
            owner.push((uid.clone(), false));
        }
    }
    if let Some((id, curb)) = curb {
        lines.push(curb);
        owner.push((id, true));
    }
    let estimate: f64 = path.windows(2).map(|w| ((w[1][0] - w[0][0]).hypot(w[1][1] - w[0][1]) / step).ceil().max(1.0)).sum();
    if estimate + 1.0 > MOST_SWEPT_SAMPLES as f64 {
        return Err(invalid(
            format!("Yol bu aralıkla {MOST_SWEPT_SAMPLES} örnekten çok tutuyor; aralığı büyütün."),
            "step",
        ));
    }
    let drive = swept::drive(&vehicle_of(input), &path, step).map_err(|e| invalid(format!("Araç sürülemiyor: {e}."), "path"))?;
    let units = drive.samples.first().map_or(0, |s| s.bodies.len());
    let corners: Vec<Vec<[f64; 2]>> =
        (0..units).flat_map(|j| (0..4).map(move |i| (j, i))).map(|(j, i)| drive.samples.iter().map(|s| s.bodies[j][i]).collect()).collect();
    let mut warnings = Vec::new();
    let clearance = swept::clearance(&drive, &lines).map(|c| {
        let s = &drive.samples[c.sample];
        let (obstacle, curb) = owner[c.obstacle].clone();
        if c.distance == 0.0 {
            warnings.push(CommandWarning {
                code: codes::SWEPT_TOUCHES.into(),
                message: format!(
                    "Aracın {} gövdesi, yolun {:.2} m'sinde {} değiyor.",
                    if c.body == 0 { "çekici" } else { "römork" },
                    s.distance,
                    if curb { "kavşağın bordürüne".to_owned() } else { format!("“{obstacle}” engeline") }
                ),
                path: Some(if curb { "junction".into() } else { "obstacles".into() }),
            });
        }
        SweptClearance {
            distance: c.distance,
            along: s.distance,
            at: s.front,
            body: u32::try_from(c.body).unwrap_or(u32::MAX),
            obstacle,
            curb,
        }
    });
    let out = SweptQueried {
        length: drive.samples.last().map_or(0.0, |s| s.distance),
        samples: u32::try_from(drive.samples.len()).unwrap_or(u32::MAX),
        front: drive.samples.iter().map(|s| s.front).collect(),
        rear: drive.samples.iter().map(|s| s.rear).collect(),
        trailer: drive.samples.iter().filter_map(|s| s.trailer).collect(),
        corners,
        offtracking: drive.offtracking,
        trailer_offtracking: drive.trailer_offtracking,
        clearance,
    };
    Ok((out, warnings))
}

/// Checks `input` against the document.
pub fn validate(cx: &ExecutionContext<'_>, input: &SweptQuery) -> CommandResult<()> {
    match answer(cx.doc, input) {
        Ok((_, warnings)) => CommandResult::Completed { output: (), warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn plan(cx: &ExecutionContext<'_>, input: &SweptQuery) -> CommandResult<SweptQueried> {
    match answer(cx.doc, input) {
        Ok((output, warnings)) => CommandResult::Completed { output, warnings },
        Err(stop) => stop.into(),
    }
}

/// The answer; reads only.
pub fn execute(cx: &mut ExecutionContext<'_>, input: SweptQuery) -> CommandResult<SweptQueried> {
    plan(cx, &input)
}

/// The drive for a window, or why it does not read.
pub fn preview(doc: &Document, input: &SweptQuery) -> Result<(SweptQueried, Vec<CommandWarning>), String> {
    answer(doc, input).map_err(crate::road_corridors::words)
}
