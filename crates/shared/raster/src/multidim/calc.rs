//! Mesh hesaplayıcı (docs/adr/0243 §10): a new dataset on a mesh from an
//! expression over its datasets: KentOS's expression language with the
//! raster calculator's mathematics; a dataset by its variable's name or its
//! shown name (`long_name`), a vector's its magnitude. The datasets the
//! expression names lie on the same places (nodes or faces); those with time
//! steps share them, the static ones stand at every step; a dataset of more
//! than one slice dimension (layers) is refused. A place any dataset has no
//! value at has none (an inactive face of a face dataset has none). Step by
//! step the datasets' slabs are read, the expression worked out over the
//! places, and kept, or met into Zaman özeti's maximum, minimum, mean or
//! sum (places without a value left out; none at all: none). At the end the
//! mesh, the time steps (none summed up) and the dataset are written as a new
//! UGRID file (CDF-2, 32-bit floats, nothing as NC_FILL_FLOAT).

use kentos_expression::exec::Slot;
use kentos_expression::host::{
    Builtin, FieldDef, FieldSource, FieldType, Geometry, Objects, Schema,
};
use kentos_expression::rows::{As, NUMBER};
use kentos_expression::{Expr, compile_with};
use kentos_formats::multidim::cf::{self, Unpack};
use std::sync::Arc;

use kentos_formats::math::hypot;
use kentos_formats::multidim::cube::{Cube, Part};
use kentos_formats::multidim::float_sample;
use kentos_formats::multidim::netcdf::{self, Header, NcType};
use kentos_formats::multidim::ugrid::Location;
use kentos_formats::multidim::write::{self, DatasetOut, TimeOut, UgridOut};
use serde::{Deserialize, Serialize};

use super::{Finished, grouped};

/// Places a batch of the expression works out.
const CHUNK: usize = 65_536;

/// How the steps' values meet (Zaman özeti).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Summary {
    None,
    Max,
    Min,
    Mean,
    Sum,
}

impl Summary {
    /// The steps' value in a sentence (“24 adımın en büyüğü”).
    fn words(self) -> &'static str {
        match self {
            Summary::None => "",
            Summary::Max => "en büyüğü",
            Summary::Min => "en küçüğü",
            Summary::Mean => "ortalaması",
            Summary::Sum => "toplamı",
        }
    }
}

/// Mesh hesaplayıcı's settings.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CalcSpec {
    pub expression: String,
    pub summary: Summary,
    /// The new dataset's shown name.
    pub name: String,
}

/// A variable read whole a step.
#[derive(Clone, Debug)]
struct Var {
    name: String,
    nc: NcType,
    unpack: Unpack,
    /// Its first dimension is the steps'.
    timed: bool,
    /// Its values a step (places, or faces for a mask).
    count: u64,
}

/// A dataset the expression names.
#[derive(Clone, Debug)]
struct Source {
    x: Var,
    y: Option<Var>,
    /// A face dataset's mask (0: the face has no value).
    mask: Option<Var>,
    timed: bool,
    /// Its values while they stand (a static dataset's from the first step on).
    kept: Option<Vec<f64>>,
}

/// A run to read: whose and which variable (0 x, 1 y, 2 mask), where.
#[derive(Clone, Copy, Debug)]
struct RunOf {
    source: usize,
    part: u8,
    offset: u64,
    len: u64,
}

/// The run's state.
pub struct MeshCalc {
    header: Arc<Header>,
    x: Vec<f64>,
    y: Vec<f64>,
    faces: Vec<Vec<u32>>,
    geographic: bool,
    epsg: Option<u32>,
    location: Location,
    places: usize,
    expr: Expr,
    /// The expression's fields' sources, in its order.
    fields: Vec<usize>,
    sources: Vec<Source>,
    steps: usize,
    step_values: Vec<f64>,
    step_absolute: bool,
    step: usize,
    pending: Vec<RunOf>,
    got: Vec<Option<Vec<u8>>>,
    summary: Summary,
    name: String,
    variable: String,
    /// Each step's values (Zaman özeti Yok).
    outputs: Vec<Vec<f32>>,
    acc: Vec<f64>,
    counts: Vec<u32>,
}

impl std::fmt::Debug for MeshCalc {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MeshCalc")
            .field("places", &self.places)
            .field("steps", &self.steps)
            .finish()
    }
}

/// The datasets' names a field may be (each dataset's variable, and its
/// shown name when another's is not the same), by the dataset.
fn names_of(cube: &Cube, mesh: &str) -> Vec<(String, usize)> {
    let h = &cube.header;
    let data: Vec<_> = cube.mesh_data.iter().filter(|d| d.mesh == mesh).collect();
    let mut out: Vec<(String, usize)> = data
        .iter()
        .enumerate()
        .map(|(k, d)| (d.variable.clone(), k))
        .collect();
    for (k, d) in data.iter().enumerate() {
        if let Some(long) = h
            .var(&d.variable)
            .and_then(|v| v.text("long_name"))
            .map(str::trim)
            && !long.is_empty()
            && !out.iter().any(|(n, _)| n == long)
        {
            out.push((long.to_owned(), k));
        }
    }
    out
}

impl MeshCalc {
    /// The run of `spec` over the mesh of `part` (a mesh raster's dataset),
    /// after the cube's `Want::Part` needs for it were read.
    pub fn new(cube: &mut Cube, part: &Part, spec: &CalcSpec) -> Result<MeshCalc, String> {
        let Some(mesh_name) = part.mesh.clone() else {
            return Err(
                "Mesh hesaplayıcı mesh raster ister: Mesh ekle ile eklenen bir raster seçin."
                    .into(),
            );
        };
        let name = spec.name.trim().to_owned();
        if name.is_empty() || name.chars().count() > 256 || name.chars().any(char::is_control) {
            return Err("Veri setinin adı 1–256 harf olmalı.".into());
        }
        let h = cube.header.clone();
        let data: Vec<_> = cube
            .mesh_data
            .iter()
            .filter(|d| d.mesh == mesh_name)
            .cloned()
            .collect();
        let names = names_of(cube, &mesh_name);
        let schema = Schema {
            fields: names
                .iter()
                .map(|(n, k)| FieldDef {
                    name: n.clone(),
                    ty: FieldType::Number,
                    source: FieldSource::User,
                    description: format!("“{}” veri seti", data[*k].variable),
                })
                .collect(),
        };
        let expr = compile_with(&spec.expression, &schema).map_err(|e| e.text())?;
        let listed = || {
            data.iter()
                .map(|d| format!("[{}]", d.variable))
                .collect::<Vec<_>>()
                .join(", ")
        };
        if expr.fields.is_empty() {
            return Err(format!(
                "İfade bir veri seti anmalı (örnek: [{}] * 2).",
                data.first().map_or("derinlik", |d| d.variable.as_str())
            ));
        }
        // Each field's dataset, each dataset once.
        let mut chosen: Vec<usize> = Vec::new();
        let mut fields = Vec::with_capacity(expr.fields.len());
        for f in &expr.fields {
            let Some(&(_, k)) = names.iter().find(|(n, _)| n == f) else {
                return Err(format!(
                    "“{f}” adında veri seti yok. Veri setleri: {}.",
                    listed()
                ));
            };
            let at = match chosen.iter().position(|&c| c == k) {
                Some(p) => p,
                None => {
                    chosen.push(k);
                    chosen.len() - 1
                }
            };
            fields.push(at);
        }
        let location = data[chosen[0]].location;
        let place_word = |l: Location| {
            if l == Location::Node {
                "düğümlerde"
            } else {
                "yüzlerde"
            }
        };
        let mesh = cube.mesh(&mesh_name).map_err(|e| e.0)?;
        let faces = cube.face_lists(&mesh_name).map_err(|e| e.0)?;
        let places = match location {
            Location::Node => mesh.x.len(),
            Location::Face => faces.len(),
        };
        let mut sources = Vec::with_capacity(chosen.len());
        let mut steps: Option<(String, Vec<f64>, bool)> = None;
        for &k in &chosen {
            let d = &data[k];
            if d.location != location {
                return Err(format!(
                    "İfadenin andığı veri setleri aynı konumda olmalı: “{}” {}, “{}” {}.",
                    data[chosen[0]].variable,
                    place_word(location),
                    d.variable,
                    place_word(d.location)
                ));
            }
            let dims = cube.slice_dims_of(&d.slice_dims).map_err(|e| e.0)?;
            let timed = match dims.as_slice() {
                [] => false,
                [t] if t.time || t.name.to_lowercase().contains("time") => {
                    match &steps {
                        Some((first, values, _)) if values != &t.values => {
                            return Err(format!(
                                "“{first}” ile “{}” aynı zaman adımlarında değil.",
                                d.variable
                            ));
                        }
                        Some(_) => {}
                        None => steps = Some((d.variable.clone(), t.values.clone(), t.time)),
                    }
                    true
                }
                _ => {
                    let list: Vec<&str> = dims.iter().map(|x| x.name.as_str()).collect();
                    return Err(format!(
                        "“{}” veri seti katmanlı ({}); Mesh hesaplayıcı katmanlı veri setini okumaz.",
                        d.variable,
                        list.join(", ")
                    ));
                }
            };
            let var = |n: &str, count: u64, float: bool| -> Result<Var, String> {
                let v = h
                    .var(n)
                    .ok_or_else(|| format!("NetCDF'te “{n}” değişkeni yok."))?;
                let mut unpack = cf::unpack_of(v);
                if float {
                    unpack.sample = float_sample(unpack.sample);
                } else {
                    unpack.sample = kentos_contracts::RasterSample::F32;
                    unpack.packed = false;
                }
                Ok(Var {
                    name: n.to_owned(),
                    nc: v.kind,
                    unpack,
                    timed: v.dims.len() >= 2,
                    count,
                })
            };
            sources.push(Source {
                x: var(&d.variable, places as u64, true)?,
                y: match &d.vector {
                    Some(n) => Some(var(n, places as u64, true)?),
                    None => None,
                },
                mask: match (&d.mask, location) {
                    (Some(n), Location::Face) => Some(var(n, faces.len() as u64, false)?),
                    _ => None,
                },
                timed,
                kept: None,
            });
        }
        let (step_values, step_absolute) = match &steps {
            Some((_, v, abs)) => (v.clone(), *abs),
            None => (Vec::new(), false),
        };
        let n_steps = step_values.len().max(1);
        let geographic = cube
            .topologies
            .iter()
            .any(|t| t.name == mesh_name && t.geographic);
        let epsg = cube.mesh_epsg(&mesh_name, Some(&data[chosen[0]].variable));
        Ok(MeshCalc {
            header: h.clone(),
            x: mesh.x.clone(),
            y: mesh.y.clone(),
            faces,
            geographic,
            epsg,
            location,
            places,
            expr,
            fields,
            sources,
            steps: n_steps,
            step_values,
            step_absolute,
            step: 0,
            pending: Vec::new(),
            got: Vec::new(),
            summary: spec.summary,
            variable: write::variable_name(&name),
            name,
            outputs: Vec::new(),
            acc: Vec::new(),
            counts: Vec::new(),
        })
    }

    /// The runs the next step reads: the timed datasets' slabs, and the static ones' at the first.
    pub fn needs(&mut self) -> Result<Vec<(u64, u64)>, String> {
        self.pending.clear();
        self.got.clear();
        if self.done() {
            return Ok(Vec::new());
        }
        let s = self.step as u64;
        for (k, src) in self.sources.iter().enumerate() {
            if src.kept.is_some() {
                continue;
            }
            for (part, v) in [
                (0u8, Some(&src.x)),
                (1, src.y.as_ref()),
                (2, src.mask.as_ref()),
            ] {
                let Some(v) = v else { continue };
                let var = self
                    .header
                    .var(&v.name)
                    .ok_or_else(|| format!("NetCDF'te “{}” değişkeni yok.", v.name))?;
                let idx: Vec<u64> = if v.timed { vec![s, 0] } else { vec![0] };
                let run = self.header.run(var, &idx, v.count).ok_or_else(|| {
                    format!("“{}” değişkeninin değerleri dosyanın dışında.", v.name)
                })?;
                self.pending.push(RunOf {
                    source: k,
                    part,
                    offset: run.offset,
                    len: run.len,
                });
            }
        }
        self.got = vec![None; self.pending.len()];
        Ok(self.pending.iter().map(|r| (r.offset, r.len)).collect())
    }

    pub fn put(&mut self, k: usize, bytes: Vec<u8>) -> Result<(), String> {
        let slot = self.got.get_mut(k).ok_or("Böyle bir parça istenmedi.")?;
        *slot = Some(bytes);
        Ok(())
    }

    pub fn done(&self) -> bool {
        self.step >= self.steps
    }

    pub fn share(&self) -> f64 {
        self.step as f64 / self.steps.max(1) as f64
    }

    /// Works the step out over the places (its runs put).
    pub fn step(&mut self) -> Result<(), String> {
        if self.done() {
            return Ok(());
        }
        if self.got.iter().any(Option::is_none) {
            return Err("Mesh'in değerleri eksik okundu.".into());
        }
        // The slabs read, as the reader keeps them, by source and variable.
        let got = std::mem::take(&mut self.got);
        let pending = std::mem::take(&mut self.pending);
        let mut fresh: Vec<[Option<Vec<f64>>; 3]> = vec![[None, None, None]; self.sources.len()];
        for (r, bytes) in pending.iter().zip(got) {
            let src = &self.sources[r.source];
            let v = match r.part {
                0 => &src.x,
                1 => src.y.as_ref().ok_or("Vektörün ikinci bileşeni yok.")?,
                _ => src.mask.as_ref().ok_or("Maske yok.")?,
            };
            fresh[r.source][r.part as usize] = Some(decode(v, &bytes.unwrap_or_default()));
        }
        // Each source's values at the places: a vector's magnitude, an inactive face none.
        let mut now: Vec<Option<Vec<f64>>> = vec![None; self.sources.len()];
        for (k, f) in fresh.into_iter().enumerate() {
            let [x, y, mask] = f;
            let Some(mut v) = x else { continue };
            if let Some(y) = y {
                for (a, b) in v.iter_mut().zip(y) {
                    *a = hypot(*a, b);
                }
            }
            if let Some(m) = mask {
                for (a, &mm) in v.iter_mut().zip(&m) {
                    if !(mm != 0.0) {
                        *a = f64::NAN;
                    }
                }
            }
            if self.sources[k].timed {
                now[k] = Some(v);
            } else {
                self.sources[k].kept = Some(v);
            }
        }
        let cols_of = |k: usize| -> &[f64] {
            match (&now[k], &self.sources[k].kept) {
                (Some(v), _) => v.as_slice(),
                (None, Some(v)) => v.as_slice(),
                _ => &[],
            }
        };
        let mut out = vec![f64::NAN; self.places];
        let mut start = 0;
        while start < self.places {
            let n = CHUNK.min(self.places - start);
            let cols: Vec<&[f64]> = self
                .fields
                .iter()
                .map(|&f| cols_of(f).get(start..start + n).unwrap_or(&[]))
                .collect();
            let objects = Places {
                expr: &self.expr,
                cols: &cols,
                n,
            };
            let column = self.expr.evaluate_objects(&objects, As::Number);
            for k in 0..n {
                let v = column.numbers.get(k).copied().unwrap_or(f64::NAN);
                let value = column.kinds.get(k) == Some(&NUMBER)
                    && v.is_finite()
                    && cols.iter().all(|c| c.get(k).is_some_and(|x| !x.is_nan()));
                out[start + k] = if value { v + 0.0 } else { f64::NAN };
            }
            start += n;
        }
        match self.summary {
            Summary::None => self.outputs.push(out.iter().map(|&v| v as f32).collect()),
            s => {
                if self.acc.is_empty() {
                    let first = if matches!(s, Summary::Max | Summary::Min) {
                        f64::NAN
                    } else {
                        0.0
                    };
                    self.acc = vec![first; self.places];
                    self.counts = vec![0; self.places];
                }
                for ((a, c), &v) in self.acc.iter_mut().zip(self.counts.iter_mut()).zip(&out) {
                    if !v.is_finite() {
                        continue;
                    }
                    *c += 1;
                    *a = match s {
                        Summary::Max if a.is_nan() || v > *a => v,
                        Summary::Min if a.is_nan() || v < *a => v,
                        Summary::Sum | Summary::Mean => *a + v,
                        _ => *a,
                    };
                }
            }
        }
        self.step += 1;
        Ok(())
    }

    /// The new dataset's variable, the mesh the file names and its steps
    /// (ms since 1970 when they are moments; none summed up or static).
    pub fn output(&self) -> (String, &'static str, Option<(Vec<f64>, bool)>) {
        let timed = self.summary == Summary::None && !self.step_values.is_empty();
        (
            self.variable.clone(),
            write::MESH,
            timed.then(|| (self.step_values.clone(), self.step_absolute)),
        )
    }

    /// Writes the new UGRID file into `sink`; the summary.
    pub fn finish(
        self,
        sink: &mut dyn FnMut(&[u8]) -> Result<(), String>,
    ) -> Result<Finished, String> {
        if !self.done() {
            return Err("Mesh hesaplayıcı bitmeden yazılamaz.".into());
        }
        let timed = self.summary == Summary::None && !self.step_values.is_empty();
        let slabs: Vec<Vec<f32>> = match self.summary {
            Summary::None => self.outputs,
            s => vec![
                self.acc
                    .iter()
                    .zip(&self.counts)
                    .map(|(&a, &c)| match (s, c) {
                        (_, 0) => f32::NAN,
                        (Summary::Mean, c) => (a / f64::from(c)) as f32,
                        _ => a as f32,
                    })
                    .collect(),
            ],
        };
        let empty: u64 = slabs
            .iter()
            .map(|s| s.iter().filter(|v| v.is_nan()).count() as u64)
            .sum();
        let all = slabs.iter().map(|s| s.len() as u64).sum::<u64>();
        let times = if timed {
            vec![TimeOut {
                values: self.step_values.clone(),
                absolute: self.step_absolute,
            }]
        } else {
            Vec::new()
        };
        let m = UgridOut {
            x: &self.x,
            y: &self.y,
            faces: &self.faces,
            geographic: self.geographic,
            epsg: self.epsg,
            times,
            datasets: vec![DatasetOut {
                name: self.variable.clone(),
                long_name: self.name.clone(),
                location: self.location,
                double: false,
                time: timed.then_some(0),
                mask: None,
                is_mask: false,
                slab: Box::new(move |s, out: &mut Vec<f64>| {
                    out.extend(
                        slabs
                            .get(s)
                            .map(Vec::as_slice)
                            .unwrap_or(&[])
                            .iter()
                            .map(|&v| f64::from(v)),
                    );
                    Ok(())
                }),
            }],
            title: self.name.clone(),
        };
        write::write_ugrid(m, sink)?;
        let places = grouped(self.places as u64);
        let word = if self.location == Location::Node {
            "düğüm"
        } else {
            "yüz"
        };
        let mut summary = if self.step_values.is_empty() {
            format!("“{}”: {places} {word}", self.name)
        } else if self.summary == Summary::None {
            format!(
                "“{}”: {places} {word}, {} adım",
                self.name,
                grouped(self.steps as u64)
            )
        } else {
            format!(
                "“{}”: {places} {word}, {} adımın {}",
                self.name,
                grouped(self.steps as u64),
                self.summary.words()
            )
        };
        if empty > 0 {
            summary.push_str(&format!(" ({} değersiz)", grouped(empty)));
        }
        summary.push('.');
        let mut warnings = Vec::new();
        if empty == all && all > 0 {
            warnings.push("Sonucun hiçbir yerinde değer yok.".into());
        }
        Ok(Finished {
            table: None,
            pieces: Vec::new(),
            summary,
            warnings,
        })
    }
}

/// The places of a chunk as the engine's objects: each field's values.
struct Places<'r> {
    expr: &'r Expr,
    cols: &'r [&'r [f64]],
    n: usize,
}

impl<'a> Objects<'a> for Places<'_> {
    fn len(&self) -> usize {
        self.n
    }

    fn field(&self, name: &str, _ty: FieldType, start: usize, mut slot: Slot<'_, 'a>) {
        match self.expr.fields.iter().position(|f| f == name) {
            Some(f) => {
                let col = self.cols[f];
                slot.numbers(|k| {
                    let v = col.get(start + k).copied().unwrap_or(f64::NAN);
                    (!v.is_nan(), v)
                });
            }
            None => slot.numbers(|_| (false, 0.0)),
        }
    }

    fn geometry(&self, _what: Geometry, _start: usize, mut slot: Slot<'_, 'a>) {
        slot.numbers(|_| (false, 0.0));
    }

    fn builtin(&self, _what: Builtin, _start: usize, mut slot: Slot<'_, 'a>) {
        for k in 0..slot.len() {
            slot.text(k, None);
        }
    }
}

/// `places` values of `v` from a slab's bytes, as the reader keeps them.
fn decode(v: &Var, bytes: &[u8]) -> Vec<f64> {
    let mut raw = Vec::with_capacity(v.count as usize);
    netcdf::decode(v.nc, bytes, &mut raw);
    raw.resize(v.count as usize, f64::NAN);
    raw.iter().map(|&r| v.unpack.stored(r)).collect()
}
