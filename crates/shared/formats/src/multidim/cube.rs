//! A NetCDF file opened as rasters (docs/adr/0243 §3–§5): its header, the
//! CF grids and UGRID meshes it holds (what Raster ekle and Mesh ekle list),
//! and a reader for one slice of one variable: a grid's rows of the slice,
//! or a mesh's values of the slice drawn as a sanal grid.
//!
//! As the raster reader does, a cube says which bytes it wants
//! ([`Cube::needs`]) and the host hands them over ([`Cube::put`]). The
//! header first ([`Cube::parse`]); then the coordinates and, for a mesh, its
//! nodes and faces (read once, kept: every slice's reader shares them).

use std::collections::HashMap;
use std::sync::Arc;

use kentos_contracts::RasterSample;
use serde::Serialize;

use super::cf::{self, Axis, SliceDim, Unpack};
use super::mesh::{self, Mesh};
use super::netcdf::{self, Header, NcType, Var};
use super::ugrid::{self, Location, MeshData, Topology};
use crate::raster::layout::{CubeDecode, Layout};
use crate::raster::source::{MeshLevels, RasterInfo, Reader};
use crate::raster::{ByteStore, Need, RasterError, Step};

/// Bytes a grid's block of rows aims at.
const BLOCK_BYTES: u64 = 64 * 1024;
/// The most dimensions a slice may have.
pub const MAX_SLICE_DIMS: usize = 8;

/// One of a variable's slice dimensions, as the windows show it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DimInfo {
    pub name: String,
    pub values: Vec<f64>,
    pub time: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
    /// Each value as the windows say it ([`dim_labels`]).
    pub labels: Vec<String>,
}

impl From<&SliceDim> for DimInfo {
    fn from(d: &SliceDim) -> DimInfo {
        DimInfo {
            name: d.name.clone(),
            values: d.values.clone(),
            time: d.time,
            units: d.units.clone(),
            labels: dim_labels(&d.values, d.time, d.units.as_deref()),
        }
    }
}

/// A slice dimension's values as the windows say them (docs/adr/0243 §11): a
/// time's as the time slider does (`GG.AA.YYYY SS:DD`, with seconds when one
/// has them), another's its shortest decimal and units.
pub fn dim_labels(values: &[f64], time: bool, units: Option<&str>) -> Vec<String> {
    use kentos_geometry_core::time::{Unit, show};
    if time {
        let seconds = values.iter().any(|&t| (t as i64).rem_euclid(60_000) != 0);
        let unit = if seconds { Unit::Second } else { Unit::Minute };
        return values.iter().map(|&t| show(t, unit)).collect();
    }
    values
        .iter()
        .map(|&v| match units {
            Some(u) if !u.is_empty() => format!("{} {u}", crate::num::plain(v)),
            _ => crate::num::plain(v),
        })
        .collect()
}

/// A CF grid of the file.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GridInfo {
    pub variable: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
    pub width: u32,
    pub height: u32,
    /// Where it lies, when its coordinates say so.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub affine: Option<[f64; 6]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsg: Option<u32>,
    pub geographic: bool,
    pub sample: RasterSample,
    pub dims: Vec<DimInfo>,
}

/// A dataset of a mesh.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DatasetInfo {
    pub variable: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vector: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub units: Option<String>,
    /// `node` or `face`.
    pub location: String,
    pub sample: RasterSample,
    pub dims: Vec<DimInfo>,
}

/// A mesh of the file.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MeshInfo {
    pub name: String,
    pub nodes: u64,
    pub faces: u64,
    /// x₁, y₁, x₂, y₂.
    pub bbox: [f64; 4],
    /// The sanal grid's default cell (§5).
    pub cell: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epsg: Option<u32>,
    pub geographic: bool,
    pub datasets: Vec<DatasetInfo>,
}

/// What a NetCDF file holds, as Raster ekle and Mesh ekle list it.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CubeInfo {
    pub version: u8,
    pub grids: Vec<GridInfo>,
    pub meshes: Vec<MeshInfo>,
    /// Variables not read, and why.
    pub notes: Vec<String>,
}

/// The variable a raster shows and its slice (the contract's `RasterDataset`
/// and, for a mesh, the raster's own grid).
#[derive(Clone, Debug, PartialEq)]
pub struct Part {
    pub variable: String,
    pub vector: Option<String>,
    pub mesh: Option<String>,
    /// An index for each slice dimension.
    pub slice: Vec<u32>,
    pub affine: [f64; 6],
    pub width: u32,
    pub height: u32,
}

impl Part {
    /// What a raster object shows of its NetCDF file (its dataset and, for a
    /// mesh, its own grid); none for a raster without a dataset.
    pub fn of(r: &kentos_contracts::RasterFields) -> Option<Part> {
        let d = r.dataset.as_ref()?;
        Some(Part {
            variable: d.variable.clone(),
            vector: d.vector.clone(),
            mesh: d.mesh.clone(),
            slice: d.slice(),
            affine: r.affine,
            width: r.width,
            height: r.height,
        })
    }

    /// A part from the JSON a raster's key carries after `#` (the style core's
    /// `raster_key`): `variable`, `vector`, `mesh`, `slice`, a mesh's `affine` and `size`.
    pub fn from_json(text: &str) -> Option<Part> {
        let v: serde_json::Value = serde_json::from_str(text).ok()?;
        let text_of = |k: &str| {
            v.get(k)
                .and_then(serde_json::Value::as_str)
                .map(str::to_owned)
        };
        let nums = |k: &str| -> Vec<f64> {
            v.get(k)
                .and_then(serde_json::Value::as_array)
                .map(|a| a.iter().filter_map(serde_json::Value::as_f64).collect())
                .unwrap_or_default()
        };
        let (affine, size) = (nums("affine"), nums("size"));
        Some(Part {
            variable: text_of("variable")?,
            vector: text_of("vector"),
            mesh: text_of("mesh"),
            slice: nums("slice").iter().map(|&i| i as u32).collect(),
            affine: if affine.len() == 6 {
                [
                    affine[0], affine[1], affine[2], affine[3], affine[4], affine[5],
                ]
            } else {
                [0.0, 1.0, 0.0, 0.0, 0.0, -1.0]
            },
            width: size.first().map_or(1, |&w| w as u32),
            height: size.get(1).map_or(1, |&h| h as u32),
        })
    }
}

/// A raster's key split at its dataset (docs/adr/0243 §5): the file's part and
/// what of the NetCDF file it shows, when any.
pub fn split_key(key: &str) -> (&str, Option<Part>) {
    match key.rfind("#{") {
        Some(at) => match Part::from_json(&key[at + 1..]) {
            Some(p) => (&key[..at], Some(p)),
            None => (key, None),
        },
        None => (key, None),
    }
}

/// What a cube is asked for.
#[derive(Clone, Copy, Debug)]
pub enum Want<'a> {
    /// Everything [`Cube::info`] lists.
    Inspect,
    /// What one raster's reader needs.
    Part(&'a Part),
}

/// A CF grid variable's reading.
#[derive(Clone, Debug)]
struct GridSpec {
    width: u32,
    height: u32,
    flip: bool,
    affine: Option<[f64; 6]>,
    slice: Vec<usize>,
    unpack: Unpack,
    epsg: Option<u32>,
    geographic: bool,
}

/// A CF grid variable's shape, as the tools read it value by value
/// (Zaman serisi, docs/adr/0243 §9).
#[derive(Clone, Debug)]
pub struct GridShape {
    pub width: u32,
    pub height: u32,
    /// The file's rows run south first: the reader's row j is the file's height − 1 − j.
    pub flip: bool,
    /// Its slice dimensions (the file's).
    pub slice: Vec<usize>,
    pub unpack: Unpack,
}

/// A NetCDF file being opened as rasters.
#[derive(Debug)]
pub struct Cube {
    pub header: Arc<Header>,
    pub topologies: Vec<Topology>,
    pub mesh_data: Vec<MeshData>,
    /// Why meshes were not read.
    pub notes: Vec<String>,
    store: ByteStore,
    values: HashMap<String, Arc<Vec<f64>>>,
    meshes: HashMap<String, Arc<Mesh>>,
}

impl Cube {
    /// The cube from the bytes handed over; or the header's prefix still needed.
    pub fn parse(store: &ByteStore, size: u64) -> Result<Step<Cube>, RasterError> {
        Ok(match netcdf::parse(store, size)? {
            Step::Done(h) => Step::Done(Cube::of(h)),
            Step::Need(n) => Step::Need(n),
        })
    }

    /// A cube of a whole file's bytes, everything it holds read (tests, small files).
    pub fn from_bytes(bytes: &[u8]) -> Result<Cube, RasterError> {
        let mut c = Cube::of(netcdf::parse_bytes(bytes)?);
        for n in c.needs(Want::Inspect) {
            let (a, b) = (n.offset as usize, (n.offset + n.len) as usize);
            c.put(n.offset, bytes.get(a..b).unwrap_or(&[]).to_vec());
        }
        Ok(c)
    }

    fn of(h: Header) -> Cube {
        let (topologies, notes) = ugrid::topologies(&h);
        let mesh_data = ugrid::datasets(&h, &topologies);
        Cube {
            header: Arc::new(h),
            topologies,
            mesh_data,
            notes,
            store: ByteStore::new(),
            values: HashMap::new(),
            meshes: HashMap::new(),
        }
    }

    /// Takes bytes the host read (at `offset`).
    pub fn put(&mut self, offset: u64, bytes: Vec<u8>) {
        self.store.put(offset, bytes);
    }

    /// The variables a want reads whole.
    fn wanted(&self, want: Want<'_>) -> Vec<String> {
        let h = &self.header;
        let mut names: Vec<String> = Vec::new();
        let mut add = |n: &str| {
            if !names.iter().any(|x| x == n) {
                names.push(n.to_owned());
            }
        };
        let coords_of = |v: &Var, add: &mut dyn FnMut(&str)| {
            for &d in &v.dims {
                if let Some(c) = cf::coordinate_of(h, d) {
                    add(&c.name);
                }
            }
        };
        let topo_vars = |t: &Topology, add: &mut dyn FnMut(&str)| {
            add(&t.node_x);
            add(&t.node_y);
            add(&t.faces);
        };
        match want {
            Want::Inspect => {
                for v in &h.vars {
                    if self.grid_candidate(v) {
                        coords_of(v, &mut add);
                    }
                }
                for d in &self.mesh_data {
                    if let Some(v) = h.var(&d.variable) {
                        coords_of(v, &mut add);
                    }
                }
                for t in &self.topologies {
                    topo_vars(t, &mut add);
                }
            }
            Want::Part(p) => {
                if let Some(v) = h.var(&p.variable) {
                    coords_of(v, &mut add);
                }
                if let Some(t) = p
                    .mesh
                    .as_deref()
                    .and_then(|m| self.topologies.iter().find(|t| t.name == m))
                {
                    topo_vars(t, &mut add);
                }
            }
        }
        names
    }

    /// The runs still needed for `want`.
    pub fn needs(&self, want: Want<'_>) -> Vec<Need> {
        let mut out = Vec::new();
        for name in self.wanted(want) {
            if self.values.contains_key(&name) {
                continue;
            }
            let Some(v) = self.header.var(&name) else {
                continue;
            };
            for r in self.header.runs(v) {
                if r.len > 0 && self.store.get(r.offset, r.len).is_none() {
                    out.push(r);
                }
            }
        }
        out
    }

    /// A variable's values read whole (once).
    fn values_of(&mut self, name: &str) -> Result<Arc<Vec<f64>>, RasterError> {
        if let Some(v) = self.values.get(name) {
            return Ok(v.clone());
        }
        let h = self.header.clone();
        let Some(v) = h.var(name) else {
            return Err(RasterError::new(format!(
                "NetCDF'te “{name}” değişkeni yok."
            )));
        };
        let mut out = Vec::new();
        for r in h.runs(v) {
            let Some(b) = self.store.get(r.offset, r.len) else {
                return Err(RasterError::new(format!(
                    "NetCDF'in “{name}” değişkeni henüz okunmadı."
                )));
            };
            netcdf::decode(v.kind, b, &mut out);
        }
        let out = Arc::new(out);
        self.values.insert(name.to_owned(), out.clone());
        Ok(out)
    }

    /// Whether a variable may be a grid: numeric, two or more dimensions,
    /// none a mesh's, and its last two Y and X.
    fn grid_candidate(&self, v: &Var) -> bool {
        let h = &self.header;
        if v.kind == NcType::Char
            || v.dims.len() < 2
            || v.attr("mesh").is_some()
            || v.attr("cf_role").is_some()
        {
            return false;
        }
        let n = v.dims.len();
        cf::axis_of(h, v.dims[n - 2]).is_y() && cf::axis_of(h, v.dims[n - 1]).is_x()
    }

    /// Why a variable is not listed as a grid; none for a grid, a coordinate or a mesh's.
    fn not_a_grid(&self, v: &Var) -> Option<String> {
        let h = &self.header;
        if v.kind == NcType::Char
            || v.dims.len() < 2
            || v.attr("mesh").is_some()
            || v.attr("cf_role").is_some()
        {
            return None;
        }
        let n = v.dims.len();
        let (a, b) = (cf::axis_of(h, v.dims[n - 2]), cf::axis_of(h, v.dims[n - 1]));
        if a.is_y() && b.is_x() {
            return None;
        }
        if a.is_x() && b.is_y() {
            return Some(format!(
                "“{}”: son iki boyut X ve Y sırasında; Y ve X olmalı.",
                v.name
            ));
        }
        if a == Axis::Other && b == Axis::Other {
            return None;
        }
        Some(format!("“{}”: son iki boyutu Y ve X değil.", v.name))
    }

    fn grid_spec(&mut self, name: &str) -> Result<GridSpec, RasterError> {
        let h = self.header.clone();
        let Some(v) = h.var(name) else {
            return Err(RasterError::new(format!(
                "NetCDF'te “{name}” değişkeni yok."
            )));
        };
        if !self.grid_candidate(v) {
            return Err(RasterError::new(format!(
                "NetCDF'in “{name}” değişkeni düzenli ızgara değil: son iki boyutu Y ve X olmalı."
            )));
        }
        let n = v.dims.len();
        if n - 2 > MAX_SLICE_DIMS {
            return Err(RasterError::new(format!(
                "“{name}” değişkeninin {} boyutu var; en çok {} okunur.",
                n,
                MAX_SLICE_DIMS + 2
            )));
        }
        let (yd, xd) = (v.dims[n - 2], v.dims[n - 1]);
        let (width, height) = (h.dim_len(xd), h.dim_len(yd));
        if !(1..=u64::from(kentos_contracts::MAX_RASTER_SIDE)).contains(&width)
            || !(1..=u64::from(kentos_contracts::MAX_RASTER_SIDE)).contains(&height)
        {
            return Err(RasterError::new(format!(
                "“{name}” ızgarasının boyu ({width} × {height}) okunmuyor."
            )));
        }
        let coord = |d: usize, me: &mut Cube| -> Result<Option<Arc<Vec<f64>>>, RasterError> {
            match cf::coordinate_of(&h, d) {
                Some(c) => me.values_of(&c.name.clone()).map(Some),
                None => Ok(None),
            }
        };
        let xs = coord(xd, self)?;
        let ys = coord(yd, self)?;
        let (mut affine, mut flip) = (None, false);
        if let (Some(xs), Some(ys)) = (&xs, &ys) {
            let rx = cf::regular(xs);
            let ry = cf::regular(ys);
            match (rx, ry, xs.len(), ys.len()) {
                (Some((x0, dx)), Some((y0, dy)), _, _) => {
                    // Rows north first: a rising y is turned round.
                    flip = dy > 0.0;
                    let top = if flip { ys[ys.len() - 1] } else { y0 };
                    let step = dy.abs();
                    affine = Some([x0 - dx / 2.0, dx, 0.0, top + step / 2.0, 0.0, -step]);
                }
                (_, _, 1, _) | (_, _, _, 1) => {}
                _ => {
                    return Err(RasterError::new(format!(
                        "“{name}” ızgarasının koordinatları düzenli aralıklı değil; okunmuyor."
                    )));
                }
            }
        }
        let geographic = cf::axis_of(&h, xd) == Axis::Lon && cf::axis_of(&h, yd) == Axis::Lat;
        let (mut epsg, _) = cf::crs_of(&h, v);
        if epsg.is_none() && geographic {
            epsg = Some(4326);
        }
        Ok(GridSpec {
            width: width as u32,
            height: height as u32,
            flip,
            affine,
            slice: v.dims[..n - 2].to_vec(),
            unpack: cf::unpack_of(v),
            epsg,
            geographic,
        })
    }

    fn slice_dims(&mut self, dims: &[usize]) -> Result<Vec<SliceDim>, RasterError> {
        let h = self.header.clone();
        let mut out = Vec::with_capacity(dims.len());
        for &d in dims {
            let coords = match cf::coordinate_of(&h, d) {
                Some(c) => Some(self.values_of(&c.name.clone())?),
                None => None,
            };
            out.push(cf::slice_dim(&h, d, coords.as_deref().map(Vec::as_slice)));
        }
        Ok(out)
    }

    fn mesh_of(&mut self, name: &str) -> Result<Arc<Mesh>, RasterError> {
        if let Some(m) = self.meshes.get(name) {
            return Ok(m.clone());
        }
        let list = self.face_lists(name)?;
        let Some(t) = self.topologies.iter().find(|t| t.name == name).cloned() else {
            return Err(RasterError::new(format!("NetCDF'te “{name}” ağı yok.")));
        };
        let x = self.values_of(&t.node_x)?;
        let y = self.values_of(&t.node_y)?;
        let m = Arc::new(Mesh::new(x.to_vec(), y.to_vec(), &list)?);
        self.meshes.insert(name.to_owned(), m.clone());
        Ok(m)
    }

    /// A mesh's faces as the file lists them (their nodes from 0, in order;
    /// the topology's variables read).
    pub fn face_lists(&mut self, name: &str) -> Result<Vec<Vec<u32>>, RasterError> {
        let Some(t) = self.topologies.iter().find(|t| t.name == name).cloned() else {
            return Err(RasterError::new(format!("NetCDF'te “{name}” ağı yok.")));
        };
        let x = self.values_of(&t.node_x)?;
        let conn = self.values_of(&t.faces)?;
        let (faces, most) = (
            self.header.dim_len(t.face_dim) as usize,
            t.max_face_nodes as usize,
        );
        let nodes = x.len() as i64;
        let mut list = Vec::with_capacity(faces);
        for f in 0..faces {
            let mut face = Vec::with_capacity(most);
            for k in 0..most {
                let raw = if t.transposed {
                    conn[k * faces + f]
                } else {
                    conn[f * most + k]
                };
                if t.fill.is_some_and(|fill| raw == fill)
                    || raw < t.start_index as f64
                    || !raw.is_finite()
                {
                    continue;
                }
                let i = raw as i64 - t.start_index;
                if i >= nodes {
                    return Err(RasterError::new(format!(
                        "Ağın {}. yüzü olmayan bir düğümü anıyor.",
                        f + 1
                    )));
                }
                face.push(i as u32);
            }
            list.push(face);
        }
        Ok(list)
    }

    /// A mesh's coordinate system: a dataset's `grid_mapping`'s, else the
    /// topology variable's, else WGS 84 for longitude and latitude.
    pub fn mesh_epsg(&self, mesh: &str, variable: Option<&str>) -> Option<u32> {
        let h = &self.header;
        variable
            .and_then(|n| h.var(n))
            .and_then(|v| cf::crs_of(h, v).0)
            .or_else(|| h.var(mesh).and_then(|tv| cf::crs_of(h, tv).0))
            .or_else(|| {
                self.topologies
                    .iter()
                    .any(|t| t.name == mesh && t.geographic)
                    .then_some(4326)
            })
    }

    /// A mesh's geometry (its topology's variables read: [`Want::Part`]'s needs).
    pub fn mesh(&mut self, name: &str) -> Result<Arc<Mesh>, RasterError> {
        self.mesh_of(name)
    }

    /// A CF grid variable's shape (its coordinates read).
    pub fn grid_shape(&mut self, variable: &str) -> Result<GridShape, RasterError> {
        let g = self.grid_spec(variable)?;
        Ok(GridShape {
            width: g.width,
            height: g.height,
            flip: g.flip,
            slice: g.slice,
            unpack: g.unpack,
        })
    }

    /// Slice dimensions with their values (their coordinates read).
    pub fn slice_dims_of(&mut self, dims: &[usize]) -> Result<Vec<SliceDim>, RasterError> {
        self.slice_dims(dims)
    }

    /// A mesh's dataset by its variable (a vector's x component).
    pub fn dataset(&self, mesh: &str, variable: &str) -> Option<&MeshData> {
        self.mesh_data
            .iter()
            .find(|d| d.mesh == mesh && d.variable == variable)
    }

    /// The slice indices of `part` checked against `dims`.
    pub fn slice_of(&self, part: &Part, dims: &[usize]) -> Result<Vec<u64>, RasterError> {
        self.check_slice(part, dims)
    }

    /// What the file holds (after [`Want::Inspect`]'s needs).
    pub fn info(&mut self) -> Result<CubeInfo, RasterError> {
        let h = self.header.clone();
        let mut grids = Vec::new();
        let mut notes = self.notes.clone();
        for v in &h.vars {
            if let Some(why) = self.not_a_grid(v) {
                notes.push(why);
                continue;
            }
            if !self.grid_candidate(v) {
                continue;
            }
            match self.grid_spec(&v.name) {
                Ok(g) => {
                    let dims = self.slice_dims(&g.slice)?;
                    grids.push(GridInfo {
                        variable: v.name.clone(),
                        long_name: v.text("long_name").map(str::to_owned),
                        units: v.text("units").map(str::to_owned),
                        width: g.width,
                        height: g.height,
                        affine: g.affine,
                        epsg: g.epsg,
                        geographic: g.geographic,
                        sample: g.unpack.sample,
                        dims: dims.iter().map(DimInfo::from).collect(),
                    });
                }
                Err(e) => notes.push(format!("“{}”: {}", v.name, e.0)),
            }
        }
        let mut meshes = Vec::new();
        for t in self.topologies.clone() {
            let m = match self.mesh_of(&t.name) {
                Ok(m) => m,
                Err(e) => {
                    notes.push(format!("“{}” ağı okunmuyor: {}", t.name, e.0));
                    continue;
                }
            };
            let mut datasets = Vec::new();
            let first = self
                .mesh_data
                .iter()
                .find(|d| d.mesh == t.name)
                .map(|d| d.variable.clone());
            let epsg = self.mesh_epsg(&t.name, first.as_deref());
            for d in self.mesh_data.clone().iter().filter(|d| d.mesh == t.name) {
                let Some(v) = h.var(&d.variable) else {
                    continue;
                };
                let dims = self.slice_dims(&d.slice_dims)?;
                datasets.push(DatasetInfo {
                    variable: d.variable.clone(),
                    vector: d.vector.clone(),
                    long_name: v.text("long_name").map(str::to_owned),
                    units: v.text("units").map(str::to_owned),
                    location: d.location.name().to_owned(),
                    sample: super::float_sample(cf::unpack_of(v).sample),
                    dims: dims.iter().map(DimInfo::from).collect(),
                });
            }
            meshes.push(MeshInfo {
                name: t.name.clone(),
                nodes: m.x.len() as u64,
                faces: u64::from(m.faces),
                bbox: m.bbox,
                cell: mesh::default_cell(&m),
                epsg,
                geographic: t.geographic,
                datasets,
            });
        }
        Ok(CubeInfo {
            version: h.version,
            grids,
            meshes,
            notes,
        })
    }

    /// A reader of one slice of one variable (after [`Want::Part`]'s needs).
    pub fn open(&mut self, part: &Part, budget: usize) -> Result<Reader, RasterError> {
        match part.mesh.clone() {
            None => self.open_grid(part, budget),
            Some(m) => self.open_mesh(part, &m, budget),
        }
    }

    fn check_slice(&self, part: &Part, dims: &[usize]) -> Result<Vec<u64>, RasterError> {
        if part.slice.len() != dims.len() {
            return Err(RasterError::new(format!(
                "“{}” değişkeninin {} dilim boyutu var; {} değer verildi.",
                part.variable,
                dims.len(),
                part.slice.len()
            )));
        }
        let mut out = Vec::with_capacity(dims.len());
        for (k, &d) in dims.iter().enumerate() {
            let n = self.header.dim_len(d);
            let i = u64::from(part.slice[k]);
            if i >= n {
                return Err(RasterError::new(format!(
                    "“{}” boyutunun {n} değeri var; {}. değer yok.",
                    self.header.dims[d].name,
                    i + 1
                )));
            }
            out.push(i);
        }
        Ok(out)
    }

    fn open_grid(&mut self, part: &Part, budget: usize) -> Result<Reader, RasterError> {
        let g = self.grid_spec(&part.variable)?;
        let h = self.header.clone();
        let Some(v) = h.var(&part.variable) else {
            return Err(RasterError::new(format!(
                "NetCDF'te “{}” değişkeni yok.",
                part.variable
            )));
        };
        let slice = self.check_slice(part, &g.slice)?;
        let size = g.unpack.raw.size();
        let w = u64::from(g.width);
        let bh = (BLOCK_BYTES / (w * size).max(1)).clamp(1, u64::from(g.height)) as u32;
        let down = g.height.div_ceil(bh);
        let mut offsets = Vec::with_capacity(down as usize);
        let mut counts = Vec::with_capacity(down as usize);
        for by in 0..down {
            let rows = bh.min(g.height - by * bh);
            let file_row = if g.flip {
                g.height - by * bh - rows
            } else {
                by * bh
            };
            let mut idx = slice.clone();
            idx.push(u64::from(file_row));
            idx.push(0);
            let Some(run) = h.run(v, &idx, u64::from(rows) * w) else {
                return Err(RasterError::new(format!(
                    "“{}” ızgarasının satırları dosyanın dışında.",
                    part.variable
                )));
            };
            offsets.push(run.offset);
            counts.push(run.len);
        }
        let layout = Layout {
            file: 0,
            ifd: 0,
            width: g.width,
            height: g.height,
            block_w: g.width,
            block_h: bh,
            across: 1,
            down,
            per_block: 1,
            planar: false,
            bands: 1,
            sample: g.unpack.sample,
            compression: 1,
            predictor: 1,
            white_is_zero: false,
            little: false,
            offsets,
            counts,
            jpeg_tables: None,
            cube: Some(CubeDecode {
                unpack: g.unpack.clone(),
                flip: g.flip,
            }),
        };
        let info = RasterInfo {
            width: g.width,
            height: g.height,
            bands: 1,
            sample: g.unpack.sample,
            color: "gray".into(),
            alpha: false,
            compression: format!("NetCDF {}", h.version),
            tiled: false,
            big: h.version != 1,
            overviews: 0,
            levels: 0,
            affine: g.affine,
            placed_by: if g.affine.is_some() { "netcdf" } else { "none" }.into(),
            epsg: g.epsg,
            geographic: g.geographic,
            nodata: g.unpack.nodata,
            needs_pyramid: false,
        };
        Reader::grid(info, layout, budget)
    }

    fn open_mesh(&mut self, part: &Part, name: &str, budget: usize) -> Result<Reader, RasterError> {
        let mesh = self.mesh_of(name)?;
        let h = self.header.clone();
        let Some(d) = self
            .mesh_data
            .iter()
            .find(|d| d.mesh == name && d.variable == part.variable)
            .cloned()
        else {
            return Err(RasterError::new(format!(
                "“{name}” ağında “{}” veri seti yok.",
                part.variable
            )));
        };
        if d.vector != part.vector {
            return Err(RasterError::new(format!(
                "“{}” veri setinin vektör bileşeni dosyadakiyle aynı değil.",
                part.variable
            )));
        }
        let slice = self.check_slice(part, &d.slice_dims)?;
        let places = match d.location {
            Location::Node => mesh.x.len() as u64,
            Location::Face => u64::from(mesh.faces),
        };
        let block = |var: &str, k: u16, out_float: bool| -> Result<Layout, RasterError> {
            let Some(v) = h.var(var) else {
                return Err(RasterError::new(format!(
                    "NetCDF'te “{var}” değişkeni yok."
                )));
            };
            let mut unpack = cf::unpack_of(v);
            if out_float {
                unpack.sample = super::float_sample(unpack.sample);
            }
            let mut idx = slice.clone();
            idx.push(0);
            let Some(run) = h.run(v, &idx, places) else {
                return Err(RasterError::new(format!(
                    "“{var}” veri setinin değerleri dosyanın dışında."
                )));
            };
            let places = u32::try_from(places).map_err(|_| RasterError::new("Ağ çok büyük."))?;
            Ok(Layout {
                file: 0,
                ifd: 100 + k,
                width: places,
                height: 1,
                block_w: places,
                block_h: 1,
                across: 1,
                down: 1,
                per_block: 1,
                planar: false,
                bands: 1,
                sample: unpack.sample,
                compression: 1,
                predictor: 1,
                white_is_zero: false,
                little: false,
                offsets: vec![run.offset],
                counts: vec![run.len],
                jpeg_tables: None,
                cube: Some(CubeDecode {
                    unpack,
                    flip: false,
                }),
            })
        };
        let mut blocks = vec![block(&d.variable, 0, true)?];
        if let Some(y) = &d.vector {
            blocks.push(block(y, 1, true)?);
        }
        let mask = match &d.mask {
            Some(m) => {
                // The mask lies on the faces; its slab has a face's worth.
                let Some(v) = h.var(m) else {
                    return Err(RasterError::new(format!("NetCDF'te “{m}” değişkeni yok.")));
                };
                let mut idx = slice.clone();
                idx.push(0);
                let Some(run) = h.run(v, &idx, u64::from(mesh.faces)) else {
                    return Err(RasterError::new(format!(
                        "“{m}” maskesinin değerleri dosyanın dışında."
                    )));
                };
                let mut unpack = cf::unpack_of(v);
                unpack.sample = RasterSample::F32;
                unpack.packed = false;
                Some(Layout {
                    file: 0,
                    ifd: 102,
                    width: mesh.faces,
                    height: 1,
                    block_w: mesh.faces,
                    block_h: 1,
                    across: 1,
                    down: 1,
                    per_block: 1,
                    planar: false,
                    bands: 1,
                    sample: RasterSample::F32,
                    compression: 1,
                    predictor: 1,
                    white_is_zero: false,
                    little: false,
                    offsets: vec![run.offset],
                    counts: vec![run.len],
                    jpeg_tables: None,
                    cube: Some(CubeDecode {
                        unpack,
                        flip: false,
                    }),
                })
            }
            None => None,
        };
        let sample = blocks[0].sample;
        let epsg = self.mesh_epsg(name, Some(&d.variable));
        let info = RasterInfo {
            width: part.width,
            height: part.height,
            bands: 1,
            sample,
            color: "gray".into(),
            alpha: false,
            compression: format!("NetCDF {} (mesh)", h.version),
            tiled: true,
            big: h.version != 1,
            overviews: 0,
            levels: 0,
            affine: Some(part.affine),
            placed_by: "netcdf".into(),
            epsg,
            geographic: false,
            nodata: None,
            needs_pyramid: false,
        };
        let levels = MeshLevels {
            mesh,
            location: d.location,
            blocks,
            mask,
            affine: part.affine,
        };
        Reader::mesh(info, levels, budget)
    }
}
