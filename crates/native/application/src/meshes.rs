//! A mesh's metadata and the face and values at a place and time (docs/adr/0552): `cad.meshes.describe` reads a mesh
//! raster's NetCDF header and its topology, `cad.meshes.query` finds the face a place lies in and reads the datasets'
//! values there. Both read the file themselves with the formats core's cube (a file where the raster names it,
//! looked for as the project's other links are; a library asset): only what they need, the values one by one.

use kentos_commands::{
    CommandResult, CommandWarning, Entity, MESH_MOST_VALUES, MeshDatasetInfo, MeshDescribed,
    MeshDimension, MeshFace, MeshShown, MeshStepOf, MeshStepValue, MeshValue, MeshesDescribe,
    MeshesDescribed, MeshesQueried, MeshesQuery, RasterEntity,
};
use kentos_domain::Document;
use kentos_formats::multidim::cube::{Cube, Part, Want, dim_labels};
use kentos_formats::multidim::series::Steps;
use kentos_formats::multidim::ugrid::{Location, MeshData};
use kentos_formats::raster::{ByteStore, Step};

use crate::ExecutionContext;
use crate::checks::{self, Stop};
use crate::codes;
use crate::pointclouds::{answer, uids_checked};
use crate::source_bytes::{Bytes, bytes_for};

/// The most a NetCDF header may take before the file is taken for broken.
const HEADER_MOST: u64 = 64 << 20;

/// A raster's NetCDF file opened: its cube, the file to read more of it.
struct Loaded {
    cube: Cube,
    bytes: Bytes,
}

/// The raster's NetCDF file with its header read and what `want` reads whole (the coordinates and the mesh's
/// topology); why not.
fn load(
    doc: &Document,
    drawing: Option<&str>,
    r: &RasterEntity,
    want: &dyn Fn(&Cube) -> Vec<kentos_formats::raster::Need>,
) -> Result<Loaded, String> {
    if r.raster.asset.is_none() && r.raster.file.is_none() && r.raster.url.is_none() {
        return Err("meshin kaynağı yok".into());
    }
    let bytes = bytes_for(
        doc,
        drawing,
        r.raster.asset.as_deref(),
        r.raster.file.as_deref(),
        r.raster.url.as_deref(),
        "adresteki NetCDF burada okunmaz; dosyayı indirip ekleyin",
    )?;
    let size = bytes.size();
    let mut store = ByteStore::new();
    let mut taken = 0u64;
    let mut cube = loop {
        match Cube::parse(&store, size).map_err(|e| e.0)? {
            Step::Done(c) => break c,
            Step::Need(n) => {
                taken += n.len;
                if taken > HEADER_MOST * 2 {
                    return Err("NetCDF'in başlığı çok büyük".into());
                }
                store.put(n.offset, bytes.read(n.offset, n.len)?);
            }
        }
    };
    fill(&mut cube, &bytes, want)?;
    Ok(Loaded { cube, bytes })
}

/// Reads what `want` still needs of `cube` (two passes: the topology's sizes come with the coordinates).
fn fill(
    cube: &mut Cube,
    bytes: &Bytes,
    want: &dyn Fn(&Cube) -> Vec<kentos_formats::raster::Need>,
) -> Result<(), String> {
    for _ in 0..2 {
        let needs = want(cube);
        if needs.is_empty() {
            break;
        }
        for n in needs {
            if n.len > 1 << 30 {
                return Err("NetCDF'in bir değişkeni 1 GB'tan büyük; okunmuyor".into());
            }
            cube.put(n.offset, bytes.read(n.offset, n.len)?);
        }
    }
    Ok(())
}

/// The mesh rasters named (each once, in the input's order), or every one of the drawing in its order.
fn meshes<'a>(
    doc: &'a Document,
    uids: &'a [String],
) -> Result<Vec<(String, &'a RasterEntity)>, Stop> {
    let is_mesh = |r: &RasterEntity| r.raster.dataset.as_ref().is_some_and(|d| d.mesh.is_some());
    if uids.is_empty() {
        return Ok(doc
            .entities_with_uids()
            .filter_map(|(u, e)| match e {
                Entity::Raster(r) if is_mesh(r) => Some((u.to_string(), r)),
                _ => None,
            })
            .collect());
    }
    let named = checks::named(doc, uids)?;
    named
        .into_iter()
        .map(|(at, _, e, uid)| match e {
            Entity::Raster(r) if is_mesh(r) => Ok((uid.clone(), r)),
            _ => Err(Stop::failed(checks::error(
                codes::NOT_A_MESH,
                "Verilen nesne bir mesh rasteri değil; bir meshin (UGRID veri setini gösteren rasterin) kimliğini verin."
                    .to_owned(),
                Some(format!("uids[{at}]")),
            ))),
        })
        .collect()
}

fn dimension(d: &kentos_formats::multidim::cube::DimInfo) -> MeshDimension {
    MeshDimension {
        name: d.name.clone(),
        count: d.values.len() as u32,
        time: d.time,
        units: d.units.clone(),
        first: d.values.first().copied(),
        last: d.values.last().copied(),
        first_label: d.labels.first().cloned(),
        last_label: d.labels.last().cloned(),
    }
}

/// What the raster shows of its dataset.
fn shown_of(r: &RasterEntity) -> Option<MeshShown> {
    let d = r.raster.dataset.as_ref()?;
    Some(MeshShown {
        variable: d.variable.clone(),
        vector: d.vector.clone(),
        steps: d
            .dims
            .iter()
            .map(|dim| MeshStepOf {
                name: dim.name.clone(),
                index: dim.index,
                label: dim.values.get(dim.index as usize).map(|v| {
                    dim_labels(&[*v], dim.time, dim.units.as_deref())
                        .pop()
                        .unwrap_or_default()
                }),
            })
            .collect(),
    })
}

/// One mesh raster described; why it could not be.
fn describe_one(
    doc: &Document,
    drawing: Option<&str>,
    uid: &str,
    r: &RasterEntity,
) -> MeshDescribed {
    let dataset = r.raster.dataset.as_ref();
    let name = dataset.and_then(|d| d.mesh.clone()).unwrap_or_default();
    let mut out = MeshDescribed {
        uid: uid.to_owned(),
        layer: r.base.layer_id.clone(),
        mesh: name.clone(),
        file: r.raster.file.clone(),
        asset: r.raster.asset.clone(),
        srid: r.raster.srid,
        shown: shown_of(r),
        ..MeshDescribed::default()
    };
    let loaded = load(doc, drawing, r, &|c| c.needs(Want::Inspect));
    let Loaded { mut cube, .. } = match loaded {
        Ok(l) => l,
        Err(why) => {
            out.why = Some(why);
            return out;
        }
    };
    let info = match cube.info() {
        Ok(i) => i,
        Err(e) => {
            out.why = Some(e.0);
            return out;
        }
    };
    let Some(m) = info.meshes.iter().find(|m| m.name == name) else {
        let reason = info
            .notes
            .iter()
            .find(|n| n.contains(&format!("“{name}”")))
            .cloned()
            .unwrap_or_else(|| format!("NetCDF'te “{name}” ağı yok"));
        out.why = Some(reason.trim_end_matches('.').to_owned());
        return out;
    };
    out.read = true;
    out.nodes = m.nodes;
    out.faces = m.faces;
    out.bbox = Some(m.bbox);
    out.epsg = m.epsg;
    out.geographic = m.geographic;
    out.datasets = m
        .datasets
        .iter()
        .map(|d| MeshDatasetInfo {
            variable: d.variable.clone(),
            vector: d.vector.clone(),
            long_name: d.long_name.clone(),
            units: d.units.clone(),
            location: d.location.clone(),
            dims: d.dims.iter().map(dimension).collect(),
        })
        .collect();
    if let Ok(mesh) = cube.mesh(&name) {
        out.edges = mesh.edges.len() as u64;
        out.triangles = mesh.tri.len() as u64;
        out.area = Some(mesh.area);
        out.mean_edge = Some(mesh.mean_edge);
    }
    out.notes = info.notes;
    out
}

/// A mesh raster described as `cad.meshes.describe` says it (Öznitelikler's rows).
pub fn described(doc: &Document, drawing: Option<&str>, r: &RasterEntity) -> MeshDescribed {
    describe_one(doc, drawing, "", r)
}

/// What `cad.meshes.describe` says; why not.
pub(crate) fn describe_in(
    doc: &Document,
    input: &MeshesDescribe,
) -> Result<(MeshesDescribed, Vec<CommandWarning>), Stop> {
    uids_checked(&input.uids)?;
    let chosen = meshes(doc, &input.uids)?;
    let drawing = input.drawing.as_deref();
    let mut warnings = Vec::new();
    let mut out = Vec::new();
    for (uid, r) in chosen {
        let d = describe_one(doc, drawing, &uid, r);
        if let Some(why) = &d.why {
            warnings.push(CommandWarning {
                code: codes::MESH_SOURCE_UNREADABLE.into(),
                message: format!("Meshin dosyası okunamadı: {why}."),
                path: Some(format!("meshes[{}]", out.len())),
            });
        }
        out.push(d);
    }
    Ok((
        MeshesDescribed {
            meshes: out,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

/// Twice a face's area, summed exactly (shoelace).
fn face_area(x: &[f64], y: &[f64], nodes: &[u32]) -> f64 {
    let (mut hi, mut lo) = (0.0f64, 0.0f64);
    let o = nodes.first().map_or((0.0, 0.0), |&k| (x[k as usize], y[k as usize]));
    for i in 0..nodes.len() {
        let (a, b) = (nodes[i] as usize, nodes[(i + 1) % nodes.len()] as usize);
        let t = (x[a] - o.0) * (y[b] - o.1) - (x[b] - o.0) * (y[a] - o.1);
        let s = hi + t;
        let bb = s - hi;
        lo += (hi - (s - bb)) + (t - bb);
        hi = s;
    }
    ((hi + lo) / 2.0).abs()
}

/// The datasets of the mesh `name` asked for (`variables`, or every one), in the asked order.
fn datasets_of(
    cube: &Cube,
    name: &str,
    variables: &[String],
) -> Result<Vec<MeshData>, Stop> {
    let all: Vec<&MeshData> = cube.mesh_data.iter().filter(|d| d.mesh == name).collect();
    if variables.is_empty() {
        return Ok(all.into_iter().cloned().collect());
    }
    variables
        .iter()
        .enumerate()
        .map(|(i, v)| {
            all.iter().find(|d| &d.variable == v).map(|d| (*d).clone()).ok_or_else(|| {
                let have: Vec<&str> = all.iter().map(|d| d.variable.as_str()).collect();
                Stop::failed(checks::error(
                    codes::MESH_VARIABLE_NOT_FOUND,
                    format!(
                        "“{v}” veri seti meshte yok; var olanlar: {}.",
                        if have.is_empty() { "hiçbiri".to_owned() } else { have.join(", ") }
                    ),
                    Some(format!("variables[{i}]")),
                ))
            })
        })
        .collect()
}

/// A query that reads `budget` values: refused beyond what one call reads.
fn within_budget(budget: u64) -> Result<(), Stop> {
    if budget > MESH_MOST_VALUES {
        return Err(Stop::failed(checks::error(
            codes::TOO_MANY_MESH_VALUES,
            format!(
                "Sorgu {budget} değer okurdu; bir çağrı en çok {MESH_MOST_VALUES} değer okur. series'i kapatın ya da veri setlerini ayrı ayrı sorun."
            ),
            Some("series".into()),
        )));
    }
    Ok(())
}

/// What `cad.meshes.query` says; why not.
pub(crate) fn query_in(
    doc: &Document,
    input: &MeshesQuery,
) -> Result<(MeshesQueried, Vec<CommandWarning>), Stop> {
    checks::point(input.at, "Aranan yerin", "at")?;
    if let Some(t) = input.time {
        checks::finite(t, "An", "1970'ten beri milisaniye verin.", "time")?;
    }
    uids_checked(&input.uids)?;
    let mut chosen = meshes(doc, &input.uids)?;
    if input.uids.is_empty() {
        chosen.retain(|(_, r)| doc.layers().is_visible(&r.base.layer_id));
    }
    let drawing = input.drawing.as_deref();
    let mut warnings = Vec::new();
    for (k, (uid, r)) in chosen.iter().enumerate() {
        let Some(shown) = r.raster.dataset.as_ref() else {
            continue;
        };
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
        let loaded = load(doc, drawing, r, &move |c| c.needs(Want::Part(&wanted)));
        let Loaded { mut cube, bytes } = match loaded {
            Ok(l) => l,
            Err(why) => {
                warnings.push(CommandWarning {
                    code: codes::MESH_SOURCE_UNREADABLE.into(),
                    message: format!("Meshin dosyası okunamadı: {why}."),
                    path: Some(format!("meshes[{k}]")),
                });
                continue;
            }
        };
        let mesh = match cube.mesh(&name) {
            Ok(m) => m,
            Err(e) => {
                warnings.push(CommandWarning {
                    code: codes::MESH_SOURCE_UNREADABLE.into(),
                    message: format!("Meshin dosyası okunamadı: {}.", e.0.trim_end_matches('.')),
                    path: Some(format!("meshes[{k}]")),
                });
                continue;
            }
        };
        let Some(t) = mesh.locate(input.at.x, input.at.y) else {
            continue;
        };
        let face = mesh.tri_face[t as usize] as usize;
        let datasets = datasets_of(&cube, &name, &input.variables)?;
        let nodes = match cube.face_lists(&name) {
            Ok(mut f) if face < f.len() => f.swap_remove(face),
            _ => Vec::new(),
        };
        let shown_moment = shown
            .dims
            .iter()
            .find(|d| d.time)
            .and_then(|d| d.values.get(d.index as usize).copied());
        let moment = input.time.or(shown_moment);
        let mut values = Vec::new();
        let mut read = 0u64;
        let mut budget = 0u64;
        for d in &datasets {
            let mut dpart = Part {
                variable: d.variable.clone(),
                vector: d.vector.clone(),
                mesh: Some(name.clone()),
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
            if let Err(why) = fill(&mut cube, &bytes, &move |c| c.needs(Want::Part(&pending))) {
                warnings.push(CommandWarning {
                    code: codes::MESH_SOURCE_UNREADABLE.into(),
                    message: format!("Meshin dosyası okunamadı: {why}."),
                    path: Some(format!("meshes[{k}]")),
                });
                continue;
            }
            let dims = cube.slice_dims_of(&d.slice_dims).map_err(|e| {
                Stop::failed(checks::error(
                    codes::MESH_VARIABLE_NOT_FOUND,
                    e.0,
                    Some("variables".into()),
                ))
            })?;
            let tk = dims.iter().position(|x| x.time);
            let units = cube
                .header
                .var(&d.variable)
                .and_then(|v| v.text("units"))
                .map(str::to_owned);
            let per = (if d.location == Location::Node { 3 } else { 1 })
                * (1 + u64::from(d.vector.is_some()))
                + u64::from(d.mask.is_some());
            let mut mv = MeshValue {
                variable: d.variable.clone(),
                vector: d.vector.clone(),
                units,
                location: d.location.name().to_owned(),
                step: None,
                time: None,
                value: None,
                series: Vec::new(),
            };
            // The step at the moment: the last at or before it (the first with no moment).
            let times: &[f64] = tk.map_or(&[], |k| &dims[k].values);
            let step = match (tk, moment) {
                (None, _) => None,
                (Some(_), None) => Some(0),
                (Some(_), Some(m)) => times.partition_point(|&v| v <= m).checked_sub(1),
            };
            let steps_read = match (tk, input.series, step) {
                (None, _, _) => 1,
                (Some(_), true, _) => times.len() as u64,
                (Some(_), false, Some(_)) => 1,
                (Some(_), false, None) => 0,
            };
            budget += steps_read * per;
            within_budget(budget)?;
            if steps_read == 0 {
                values.push(mv);
                continue;
            }
            let which = match (tk, input.series) {
                (None, _) => Steps::Slice,
                (Some(_), true) => Steps::All,
                (Some(_), false) => Steps::One(step.unwrap_or(0)),
            };
            let affine = part.affine;
            let mut series = cube
                .series_of(&dpart, affine, None, &[[input.at.x, input.at.y]], which)
                .map_err(|e| {
                    Stop::failed(checks::error(
                        codes::MESH_VARIABLE_NOT_FOUND,
                        e.0,
                        Some("variables".into()),
                    ))
                })?;
            for run in series.runs.clone() {
                match bytes.read(run.offset, run.len) {
                    Ok(b) => series.put(run.offset, b),
                    Err(why) => {
                        warnings.push(CommandWarning {
                            code: codes::MESH_SOURCE_UNREADABLE.into(),
                            message: format!("Meshin dosyası okunamadı: {why}."),
                            path: Some(format!("meshes[{k}]")),
                        });
                    }
                }
            }
            let rows = series.values().map_err(|e| {
                Stop::failed(checks::error(
                    codes::MESH_VARIABLE_NOT_FOUND,
                    e.0,
                    Some("variables".into()),
                ))
            })?;
            read += steps_read * per;
            let one = |v: f64| (!v.is_nan()).then_some(v);
            match tk {
                None => mv.value = one(rows[0][0]),
                Some(_) => {
                    if let Some(s) = step {
                        mv.step = Some(s as u32);
                        mv.time = Some(times[s]);
                        mv.value = if input.series {
                            rows.get(s).and_then(|r| one(r[0]))
                        } else {
                            one(rows[0][0])
                        };
                    }
                    if input.series {
                        let labels = dim_labels(times, true, None);
                        mv.series = rows
                            .iter()
                            .enumerate()
                            .map(|(i, r)| MeshStepValue {
                                step: i as u32,
                                time: times[i],
                                label: labels[i].clone(),
                                value: one(r[0]),
                            })
                            .collect();
                    }
                }
            }
            values.push(mv);
        }
        let area = face_area(&mesh.x, &mesh.y, &nodes);
        return Ok((
            MeshesQueried {
                face: Some(MeshFace {
                    mesh: uid.clone(),
                    topology: name,
                    face: face as u64,
                    nodes,
                    area,
                }),
                values,
                read,
                revision: doc.revision().to_string(),
            },
            warnings,
        ));
    }
    Ok((
        MeshesQueried {
            face: None,
            values: Vec::new(),
            read: 0,
            revision: doc.revision().to_string(),
        },
        warnings,
    ))
}

pub mod describe {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &MeshesDescribe) -> CommandResult<()> {
        match uids_checked(&input.uids).and_then(|()| meshes(cx.doc, &input.uids).map(|_| ())) {
            Ok(()) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(
        cx: &ExecutionContext<'_>,
        input: &MeshesDescribe,
    ) -> CommandResult<MeshesDescribed> {
        answer(describe_in(cx.doc, input))
    }

    /// Read-only: the same answer as the plan.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: MeshesDescribe,
    ) -> CommandResult<MeshesDescribed> {
        plan(cx, &input)
    }
}

pub mod query {
    use super::*;

    pub fn validate(cx: &ExecutionContext<'_>, input: &MeshesQuery) -> CommandResult<()> {
        let checked = checks::point(input.at, "Aranan yerin", "at")
            .and_then(|()| uids_checked(&input.uids))
            .and_then(|()| meshes(cx.doc, &input.uids).map(|_| ()));
        match checked {
            Ok(()) => CommandResult::Completed {
                output: (),
                warnings: Vec::new(),
            },
            Err(stop) => stop.into(),
        }
    }

    pub fn plan(cx: &ExecutionContext<'_>, input: &MeshesQuery) -> CommandResult<MeshesQueried> {
        answer(query_in(cx.doc, input))
    }

    /// Read-only: the same answer as the plan.
    pub fn execute(
        cx: &mut ExecutionContext<'_>,
        input: MeshesQuery,
    ) -> CommandResult<MeshesQueried> {
        plan(cx, &input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_query_that_reads_more_values_than_one_call_does_is_refused() {
        assert!(within_budget(MESH_MOST_VALUES).is_ok());
        let Err(Stop::Failed(e)) = within_budget(MESH_MOST_VALUES + 1) else {
            panic!("refused");
        };
        assert_eq!(e.code, codes::TOO_MANY_MESH_VALUES);
        assert_eq!(e.path.as_deref(), Some("series"));
        assert!(e.message.contains("2000001 değer"));
    }
}
