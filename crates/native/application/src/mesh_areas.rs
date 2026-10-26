//! A mesh's faces in an area and its values along a path (docs/adr/0645 §7): `cad.meshes.region` and
//! `cad.meshes.profile`. Both read the file as `cad.meshes.query` does (a NetCDF-4, GRIB or Zarr data set too), only
//! the values their places need, and answer each value as the query answers it at a place: a node dataset's value
//! interpolated in the triangle, a face dataset's the face's own, a vector's magnitude, nothing where it is missing.

use kentos_commands::{
    CommandResult, CommandWarning, MESH_MOST_LISTED, MESH_MOST_STATIONS, MESH_MOST_VALUES,
    MeshProfile, MeshRegion, MeshStation, MeshZonal, MeshesProfile, MeshesProfiled, MeshesRegion,
    MeshesRegioned, RasterDataset, RasterEntity,
};
use kentos_domain::Document;
use kentos_formats::multidim::cube::{Cube, Part, Want};
use kentos_formats::multidim::series::Steps;
use kentos_formats::multidim::ugrid::{Location, MeshData};
use kentos_geometry_core::vec2::Vec2 as P;

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::meshes::{Loaded, datasets_of, face_area, fill, load, meshes};
use crate::pointcloud_areas::{Place, Sum, place_in, ring_box, ring_coded, twice_area};
use crate::pointclouds::{answer, uids_checked};
use crate::source_bytes::Bytes;

/// The meshes looked in: the named, or the visible ones of the drawing.
fn chosen<'a>(
    doc: &'a Document,
    uids: &'a [String],
) -> Result<Vec<(String, &'a RasterEntity)>, Stop> {
    uids_checked(uids)?;
    let mut chosen = meshes(doc, uids)?;
    if uids.is_empty() {
        chosen.retain(|(_, r)| doc.layers().is_visible(&r.base.layer_id));
    }
    Ok(chosen)
}

fn unreadable(k: usize, why: &str) -> CommandWarning {
    CommandWarning {
        code: codes::MESH_SOURCE_UNREADABLE.into(),
        message: format!("Meshin dosyası okunamadı: {}.", why.trim_end_matches('.')),
        path: Some(format!("meshes[{k}]")),
    }
}

/// A mesh raster's file opened with what its shown part needs, its topology's name and that part.
fn opened(
    doc: &Document,
    drawing: Option<&str>,
    r: &RasterEntity,
) -> Result<(Loaded, String, Part, RasterDataset), String> {
    let shown = r.raster.dataset.clone().ok_or("meshin veri seti yok")?;
    let name = shown.mesh.clone().unwrap_or_default();
    let part = Part::of(&r.raster).unwrap_or(Part {
        variable: shown.variable.clone(),
        vector: shown.vector.clone(),
        mesh: Some(name.clone()),
        slice: shown.slice(),
        affine: r.raster.affine,
        width: r.raster.width,
        height: r.raster.height,
    });
    let wanted = part.clone();
    let loaded = load(doc, drawing, r, &move |c| c.needs(Want::Part(&wanted)))?;
    Ok((loaded, name, part, shown))
}

/// What a values' read asks: the moment, the budget so far, where a refusal points.
struct Ask<'a> {
    moment: Option<f64>,
    path: &'a str,
    advice: &'a str,
}

/// A dataset's zonal answer with the values of its faces.
type Zonal = (MeshZonal, Vec<f64>);

/// Each dataset's values at `points` at the moment (or the steps the raster shows): its heading and one value a
/// point (NaN: none). A file that cannot be read is said by `Err(why)` inside `Ok`.
#[allow(clippy::too_many_arguments)]
fn values_at(
    cube: &mut Cube,
    bytes: &Bytes,
    name: &str,
    part: &Part,
    shown: &RasterDataset,
    datasets: &[MeshData],
    points: &[[f64; 2]],
    ask: &Ask<'_>,
    budget: &mut u64,
    read: &mut u64,
) -> Result<Result<Vec<Zonal>, String>, Stop> {
    let mut out = Vec::new();
    for d in datasets {
        let mut dpart = Part {
            variable: d.variable.clone(),
            vector: d.vector.clone(),
            mesh: Some(name.to_owned()),
            slice: vec![0; d.slice_dims.len()],
            affine: part.affine,
            width: part.width,
            height: part.height,
        };
        if d.variable == shown.variable {
            dpart.slice = shown.slice();
            dpart.slice.resize(d.slice_dims.len(), 0);
        }
        let pending = dpart.clone();
        if let Err(why) = fill(cube, bytes, &move |c| c.needs(Want::Part(&pending))) {
            return Ok(Err(why));
        }
        let refused = |e: kentos_formats::raster::RasterError| {
            Stop::failed(checks::error(
                codes::MESH_VARIABLE_NOT_FOUND,
                e.0,
                Some("variables".into()),
            ))
        };
        let dims = cube.slice_dims_of(&d.slice_dims).map_err(refused)?;
        let tk = dims.iter().position(|x| x.time);
        let units = cube
            .header
            .var(&d.variable)
            .and_then(|v| v.text("units"))
            .map(str::to_owned);
        let per = (if d.location == Location::Node { 3 } else { 1 })
            * (1 + u64::from(d.vector.is_some()))
            + u64::from(d.mask.is_some());
        let mut head = MeshZonal {
            variable: d.variable.clone(),
            vector: d.vector.clone(),
            units,
            location: d.location.name().to_owned(),
            ..MeshZonal::default()
        };
        let times: &[f64] = tk.map_or(&[], |k| &dims[k].values);
        // The step at the moment (the asked one, else the raster's shown one): the last at or before it, as
        // `cad.meshes.query` takes it; with no moment at all, the first.
        let moment = ask.moment.or_else(|| {
            shown
                .dims
                .iter()
                .find(|x| x.time)
                .and_then(|x| x.values.get(x.index as usize).copied())
        });
        let step = match (tk, moment) {
            (None, _) => None,
            (Some(_), None) => Some(0),
            (Some(_), Some(m)) => times.partition_point(|&v| v <= m).checked_sub(1),
        };
        if tk.is_some() && step.is_none() {
            out.push((head, vec![f64::NAN; points.len()]));
            continue;
        }
        *budget += per * points.len() as u64;
        if *budget > MESH_MOST_VALUES {
            return Err(Stop::failed(checks::error(
                codes::TOO_MANY_MESH_VALUES,
                format!(
                    "Sorgu {} değer okurdu; bir çağrı en çok {MESH_MOST_VALUES} değer okur. {}",
                    *budget, ask.advice
                ),
                Some(ask.path.into()),
            )));
        }
        let which = match step {
            None => Steps::Slice,
            Some(s) => {
                head.step = Some(s as u32);
                head.time = times.get(s).copied();
                Steps::One(s)
            }
        };
        if points.is_empty() {
            out.push((head, Vec::new()));
            continue;
        }
        let mut series = cube
            .series_of(&dpart, part.affine, None, points, which)
            .map_err(refused)?;
        for run in series.runs.clone() {
            match bytes.read(run.offset, run.len) {
                Ok(b) => series.put(run.offset, b),
                Err(why) => return Ok(Err(why)),
            }
        }
        let rows = series.values().map_err(refused)?;
        *read += per * points.len() as u64;
        let values = rows.into_iter().next().unwrap_or_default();
        out.push((head, values));
    }
    Ok(Ok(out))
}

fn moment_checked(time: Option<f64>) -> Result<(), Stop> {
    if let Some(t) = time {
        checks::finite(t, "An", "1970'ten beri milisaniye verin.", "time")?;
    }
    Ok(())
}

fn some(v: f64) -> Option<f64> {
    (!v.is_nan()).then_some(v)
}

// ── Area ───────────────────────────────────────────────────────────────

/// A face's centre: its area's centroid (taken from its first node, so that coordinates in the millions lose
/// nothing); a face of no area, its nodes' mean.
fn centroid(x: &[f64], y: &[f64], nodes: &[u32]) -> [f64; 2] {
    let Some(&first) = nodes.first() else {
        return [f64::NAN, f64::NAN];
    };
    let (ox, oy) = (x[first as usize], y[first as usize]);
    let (mut a, mut cx, mut cy) = (Sum::default(), Sum::default(), Sum::default());
    for i in 0..nodes.len() {
        let (p, q) = (nodes[i] as usize, nodes[(i + 1) % nodes.len()] as usize);
        let (px, py, qx, qy) = (x[p] - ox, y[p] - oy, x[q] - ox, y[q] - oy);
        let c = px * qy - qx * py;
        a.add(c);
        cx.add((px + qx) * c);
        cy.add((py + qy) * c);
    }
    let a = a.value();
    if a == 0.0 {
        let n = nodes.len() as f64;
        let mx = nodes.iter().map(|&k| x[k as usize] - ox).sum::<f64>() / n;
        let my = nodes.iter().map(|&k| y[k as usize] - oy).sum::<f64>() / n;
        return [ox + mx, oy + my];
    }
    [ox + cx.value() / (3.0 * a), oy + cy.value() / (3.0 * a)]
}

/// What `cad.meshes.region` says; why not.
pub(crate) fn region_in(
    doc: &Document,
    input: &MeshesRegion,
) -> Result<(MeshesRegioned, Vec<CommandWarning>), Stop> {
    let outer = ring_coded(&input.ring, "Halka", "ring", codes::INVALID_MESH_AREA)?;
    let mut holes = Vec::new();
    for (h, r) in input.holes.iter().enumerate() {
        holes.push(ring_coded(
            r,
            "Delik",
            &format!("holes[{h}]"),
            codes::INVALID_MESH_AREA,
        )?);
    }
    let mut twice = twice_area(&outer).abs();
    for h in &holes {
        twice -= twice_area(h).abs();
    }
    let area = twice / 2.0;
    if area.is_nan() || area <= 0.0 {
        return Err(Stop::failed(checks::error(
            codes::INVALID_MESH_AREA,
            "Alanın büyüklüğü sıfırdan büyük olmalı (halka eksi delikler); köşeler bir doğru üzerinde ya da delikler halkayı kaplıyor.".into(),
            Some("ring".into()),
        )));
    }
    moment_checked(input.time)?;
    let chosen = chosen(doc, &input.uids)?;
    let bbox = ring_box(&outer);
    let hole_boxes: Vec<[f64; 4]> = holes.iter().map(|h| ring_box(h)).collect();
    let inside = |c: [f64; 2]| {
        let p = P { x: c[0], y: c[1] };
        if !(p.x >= bbox[0] && p.x <= bbox[2] && p.y >= bbox[1] && p.y <= bbox[3]) {
            return false;
        }
        if place_in(&outer, p) == Place::Outside {
            return false;
        }
        !holes.iter().zip(&hole_boxes).any(|(h, hb)| {
            p.x >= hb[0]
                && p.x <= hb[2]
                && p.y >= hb[1]
                && p.y <= hb[3]
                && place_in(h, p) == Place::Inside
        })
    };
    let drawing = input.drawing.as_deref();
    let mut warnings = Vec::new();
    let (mut budget, mut read) = (0u64, 0u64);
    let mut out = Vec::new();
    let ask = Ask {
        moment: input.time,
        path: "ring",
        advice: "Alanı küçültün ya da veri setlerini ayrı ayrı sorun.",
    };
    for (k, (uid, r)) in chosen.iter().enumerate() {
        let (Loaded { mut cube, bytes }, name, part, shown) = match opened(doc, drawing, r) {
            Ok(x) => x,
            Err(why) => {
                warnings.push(unreadable(k, &why));
                continue;
            }
        };
        let (mesh, faces) = match (cube.mesh(&name), cube.face_lists(&name)) {
            (Ok(m), Ok(f)) => (m, f),
            (Err(e), _) | (_, Err(e)) => {
                warnings.push(unreadable(k, &e.0));
                continue;
            }
        };
        // Each face's first triangle: a non-convex face's centre when its centroid falls outside it.
        let mut first = vec![u32::MAX; faces.len()];
        for (t, &f) in mesh.tri_face.iter().enumerate().rev() {
            if let Some(v) = first.get_mut(f as usize) {
                *v = t as u32;
            }
        }
        let mut ids = Vec::new();
        let mut centres = Vec::new();
        let mut areas = Vec::new();
        for (f, nodes) in faces.iter().enumerate() {
            let mut c = centroid(&mesh.x, &mesh.y, nodes);
            let own = mesh
                .locate(c[0], c[1])
                .is_some_and(|t| mesh.tri_face[t as usize] as usize == f);
            if !own && let Some(&t) = first.get(f).filter(|&&t| t != u32::MAX) {
                let [a, b, d] = mesh.tri[t as usize];
                let (a, b, d) = (a as usize, b as usize, d as usize);
                c = [
                    (mesh.x[a] + mesh.x[b] + mesh.x[d]) / 3.0,
                    (mesh.y[a] + mesh.y[b] + mesh.y[d]) / 3.0,
                ];
            }
            if inside(c) {
                ids.push(f as u64);
                centres.push(c);
                areas.push(face_area(&mesh.x, &mesh.y, nodes));
            }
        }
        let datasets = datasets_of(&cube, &name, &input.variables)?;
        let taken = values_at(
            &mut cube,
            &bytes,
            &name,
            &part,
            &shown,
            &datasets,
            &centres,
            &ask,
            &mut budget,
            &mut read,
        )?;
        let taken = match taken {
            Ok(t) => t,
            Err(why) => {
                warnings.push(unreadable(k, &why));
                continue;
            }
        };
        let mut whole = Sum::default();
        for a in &areas {
            whole.add(*a);
        }
        let zonal = taken
            .into_iter()
            .map(|(mut head, values)| {
                let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
                let (mut weight, mut integral, mut n) = (Sum::default(), Sum::default(), 0u64);
                for (v, a) in values.iter().zip(&areas) {
                    if v.is_nan() {
                        continue;
                    }
                    n += 1;
                    lo = lo.min(*v);
                    hi = hi.max(*v);
                    weight.add(*a);
                    integral.add(v * a);
                }
                head.count = n;
                if n > 0 {
                    head.min = Some(lo);
                    head.max = Some(hi);
                    let w = weight.value();
                    head.integral = Some(integral.value());
                    head.mean = (w > 0.0).then(|| integral.value() / w);
                }
                head
            })
            .collect();
        let listed = if input.faces {
            if ids.len() as u64 > MESH_MOST_LISTED {
                return Err(Stop::failed(checks::error(
                    codes::TOO_MANY_MESH_VALUES,
                    format!(
                        "Alanda {} yüz var; bir çağrı en çok {MESH_MOST_LISTED} yüzü sayar. faces'i kapatın ya da alanı küçültün.",
                        ids.len()
                    ),
                    Some("faces".into()),
                )));
            }
            ids.clone()
        } else {
            Vec::new()
        };
        out.push(MeshRegion {
            mesh: uid.clone(),
            topology: name,
            count: ids.len() as u64,
            area: whole.value(),
            faces: listed,
            datasets: zonal,
        });
    }
    Ok((
        MeshesRegioned {
            area,
            meshes: out,
            read,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

// ── Path ───────────────────────────────────────────────────────────────

fn bad_path(why: String, path: &str) -> Stop {
    Stop::failed(checks::error(
        codes::INVALID_MESH_PATH,
        why,
        Some(path.to_owned()),
    ))
}

/// A station: its distance along the path and its place.
type Station = (f64, [f64; 2]);

/// The stations of a path: 0, `step`, 2·`step` … and its end, each with its place.
fn stations_of(path: &[kentos_commands::Vec2], step: f64) -> Result<(f64, Vec<Station>), Stop> {
    if path.len() < 2 {
        return Err(bad_path(
            "Profilin yolu en az iki köşeli olmalı; yolun köşelerini sırayla verin.".into(),
            "path",
        ));
    }
    for (i, p) in path.iter().enumerate() {
        if !(p.x.is_finite() && p.y.is_finite()) {
            return Err(bad_path(
                format!(
                    "Yolun {}. köşesi sonlu bir sayı değil; köşeler planda koordinatlardır.",
                    i + 1
                ),
                &format!("path[{i}]"),
            ));
        }
        if i > 0 && path[i - 1].x == p.x && path[i - 1].y == p.y {
            return Err(bad_path(
                format!(
                    "Yolun {}. köşesi öncekiyle aynı; uzunluğu olmayan parça olmaz.",
                    i + 1
                ),
                &format!("path[{i}]"),
            ));
        }
    }
    if !(step.is_finite() && step > 0.0) {
        return Err(bad_path(
            "Adım sıfırdan büyük olmalı; istasyonların yol boyunca aralığını verin.".into(),
            "step",
        ));
    }
    let lens: Vec<f64> = path
        .windows(2)
        .map(|w| {
            let (dx, dy) = (w[1].x - w[0].x, w[1].y - w[0].y);
            (dx * dx + dy * dy).sqrt()
        })
        .collect();
    let mut starts = Vec::with_capacity(lens.len());
    let mut at = Sum::default();
    for l in &lens {
        starts.push(at.value());
        at.add(*l);
    }
    let length = at.value();
    let whole = (length / step).floor();
    let mut count = whole as u64 + 1;
    if whole * step < length {
        count += 1;
    }
    if count > MESH_MOST_STATIONS {
        return Err(Stop::failed(checks::error(
            codes::TOO_MANY_STATIONS,
            format!(
                "Yol {count} istasyon verirdi; bir çağrı en çok {MESH_MOST_STATIONS} istasyon cevaplar. Adımı büyütün."
            ),
            Some("step".into()),
        )));
    }
    let place = |s: f64| -> [f64; 2] {
        let j = starts
            .partition_point(|&v| v <= s)
            .saturating_sub(1)
            .min(lens.len() - 1);
        let (a, b) = (path[j], path[j + 1]);
        let t = ((s - starts[j]) / lens[j]).clamp(0.0, 1.0);
        [a.x + t * (b.x - a.x), a.y + t * (b.y - a.y)]
    };
    let mut out = Vec::with_capacity(count as usize);
    for i in 0..=(whole as u64) {
        let s = i as f64 * step;
        out.push((s, place(s)));
    }
    if whole * step < length {
        let end = path[path.len() - 1];
        out.push((length, [end.x, end.y]));
    }
    Ok((length, out))
}

/// What `cad.meshes.profile` says; why not.
pub(crate) fn profile_in(
    doc: &Document,
    input: &MeshesProfile,
) -> Result<(MeshesProfiled, Vec<CommandWarning>), Stop> {
    let (length, stations) = stations_of(&input.path, input.step)?;
    moment_checked(input.time)?;
    let chosen = chosen(doc, &input.uids)?;
    let points: Vec<[f64; 2]> = stations.iter().map(|(_, p)| *p).collect();
    let drawing = input.drawing.as_deref();
    let mut warnings = Vec::new();
    let (mut budget, mut read) = (0u64, 0u64);
    let ask = Ask {
        moment: input.time,
        path: "step",
        advice: "Adımı büyütün ya da veri setlerini ayrı ayrı sorun.",
    };
    let mut out = Vec::new();
    for (k, (uid, r)) in chosen.iter().enumerate() {
        let (Loaded { mut cube, bytes }, name, part, shown) = match opened(doc, drawing, r) {
            Ok(x) => x,
            Err(why) => {
                warnings.push(unreadable(k, &why));
                continue;
            }
        };
        let mesh = match cube.mesh(&name) {
            Ok(m) => m,
            Err(e) => {
                warnings.push(unreadable(k, &e.0));
                continue;
            }
        };
        let datasets = datasets_of(&cube, &name, &input.variables)?;
        let taken = match values_at(
            &mut cube,
            &bytes,
            &name,
            &part,
            &shown,
            &datasets,
            &points,
            &ask,
            &mut budget,
            &mut read,
        )? {
            Ok(t) => t,
            Err(why) => {
                warnings.push(unreadable(k, &why));
                continue;
            }
        };
        let rows = stations
            .iter()
            .enumerate()
            .map(|(i, (s, p))| MeshStation {
                station: *s,
                x: p[0],
                y: p[1],
                face: mesh
                    .locate(p[0], p[1])
                    .map(|t| u64::from(mesh.tri_face[t as usize])),
                values: taken
                    .iter()
                    .map(|(_, v)| v.get(i).copied().and_then(some))
                    .collect(),
            })
            .collect();
        out.push(MeshProfile {
            mesh: uid.clone(),
            topology: name,
            datasets: taken.into_iter().map(|(h, _)| h).collect(),
            stations: rows,
        });
    }
    Ok((
        MeshesProfiled {
            length,
            meshes: out,
            read,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

pub mod region {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &MeshesRegion) -> CommandResult<()> {
        let checked = ring_coded(&input.ring, "Halka", "ring", codes::INVALID_MESH_AREA)
            .and_then(|_| chosen(cx.doc, &input.uids).map(|_| ()));
        match checked {
            Ok(()) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &MeshesRegion) -> CommandResult<MeshesRegioned> {
        answer(region_in(cx.doc, input))
    }

    /// Read-only: the same answer as the plan.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: MeshesRegion,
    ) -> CommandResult<MeshesRegioned> {
        plan(cx, &input)
    }
}

pub mod profile {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &MeshesProfile) -> CommandResult<()> {
        let checked = stations_of(&input.path, input.step)
            .and_then(|_| chosen(cx.doc, &input.uids).map(|_| ()));
        match checked {
            Ok(()) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &MeshesProfile) -> CommandResult<MeshesProfiled> {
        answer(profile_in(cx.doc, input))
    }

    /// Read-only: the same answer as the plan.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: MeshesProfile,
    ) -> CommandResult<MeshesProfiled> {
        plan(cx, &input)
    }
}
