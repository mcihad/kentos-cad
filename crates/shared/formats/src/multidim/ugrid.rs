//! UGRID 1.0 two-dimensional meshes in a NetCDF file (docs/adr/0243 §4):
//! the topology variables (`cf_role = "mesh_topology"`), their node
//! coordinates and face–node connectivity, and the data variables on their
//! nodes or faces; two components of a vector paired by their names.

use super::cf::{Axis, axis_of_var, coordinate_of};
use super::netcdf::{Header, NcType, Var};

/// The most nodes and faces a mesh may have, and nodes a face.
pub const MAX_NODES: u64 = 20_000_000;
pub const MAX_FACES: u64 = 20_000_000;
pub const MAX_FACE_NODES: u64 = 16;

/// Where a data variable's values lie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Location {
    Node,
    Face,
}

impl Location {
    pub fn name(self) -> &'static str {
        match self {
            Location::Node => "node",
            Location::Face => "face",
        }
    }
}

/// A mesh topology in the file.
#[derive(Clone, Debug, PartialEq)]
pub struct Topology {
    pub name: String,
    pub node_x: String,
    pub node_y: String,
    pub faces: String,
    pub node_dim: usize,
    pub face_dim: usize,
    /// Nodes a face may have (the connectivity's second dimension).
    pub max_face_nodes: u64,
    /// The connectivity's dimensions are (nodes, faces), not (faces, nodes).
    pub transposed: bool,
    pub start_index: i64,
    /// Values that mark a face's missing nodes (`_FillValue`, and negatives).
    pub fill: Option<f64>,
    /// Longitude and latitude.
    pub geographic: bool,
}

/// A data variable on a mesh (a vector's two components together).
#[derive(Clone, Debug, PartialEq)]
pub struct MeshData {
    pub variable: String,
    /// A vector's y component.
    pub vector: Option<String>,
    pub mesh: String,
    pub location: Location,
    /// Its dimensions but the location's, in order.
    pub slice_dims: Vec<usize>,
    /// A face mask's variable (`kentos_mask`, a DAT's activity; 0 inactive).
    pub mask: Option<String>,
}

/// The meshes of a header, and why the others are not read.
pub fn topologies(h: &Header) -> (Vec<Topology>, Vec<String>) {
    let mut out = Vec::new();
    let mut notes = Vec::new();
    for v in &h.vars {
        if v.text("cf_role").map(str::trim) != Some("mesh_topology") {
            continue;
        }
        match topology(h, v) {
            Ok(t) => out.push(t),
            Err(why) => notes.push(format!("“{}” ağı okunmuyor: {why}", v.name)),
        }
    }
    (out, notes)
}

fn topology(h: &Header, v: &Var) -> Result<Topology, String> {
    let dim = v.attr("topology_dimension").and_then(|a| a.number());
    if dim != Some(2.0) {
        return Err("yalnız iki boyutlu ağlar (topology_dimension 2) okunur.".into());
    }
    let coords: Vec<&str> = v
        .text("node_coordinates")
        .unwrap_or("")
        .split_whitespace()
        .collect();
    if coords.len() != 2 {
        return Err("node_coordinates iki değişken adı olmalı.".into());
    }
    let (Some(a), Some(b)) = (h.var(coords[0]), h.var(coords[1])) else {
        return Err("node_coordinates'in andığı değişkenler dosyada yok.".into());
    };
    // x first, unless the attributes say the other way round.
    let (x, y) = if axis_of_var(a).is_y() || axis_of_var(b).is_x() {
        (b, a)
    } else {
        (a, b)
    };
    if x.dims.len() != 1 || y.dims.len() != 1 || x.dims[0] != y.dims[0] {
        return Err("düğüm koordinatları aynı tek boyutlu olmalı.".into());
    }
    let node_dim = x.dims[0];
    let nodes = h.dim_len(node_dim);
    if !(3..=MAX_NODES).contains(&nodes) {
        return Err(format!(
            "{nodes} düğümü var; 3 ile {MAX_NODES} arasında olmalı."
        ));
    }
    let Some(f) = v
        .text("face_node_connectivity")
        .map(str::trim)
        .and_then(|n| h.var(n))
    else {
        return Err("face_node_connectivity yok.".into());
    };
    if f.dims.len() != 2 || !f.kind.integer() || f.kind == NcType::Char {
        return Err("yüzlerin düğümleri iki boyutlu tam sayı değişken olmalı.".into());
    }
    let named_face = v
        .text("face_dimension")
        .map(str::trim)
        .and_then(|n| h.dims.iter().position(|d| d.name == n));
    let transposed = named_face.is_some_and(|d| f.dims[1] == d && f.dims[0] != d);
    let (face_dim, max_dim) = if transposed {
        (f.dims[1], f.dims[0])
    } else {
        (f.dims[0], f.dims[1])
    };
    let faces = h.dim_len(face_dim);
    let max_face_nodes = h.dim_len(max_dim);
    if !(1..=MAX_FACES).contains(&faces) {
        return Err(format!(
            "{faces} yüzü var; 1 ile {MAX_FACES} arasında olmalı."
        ));
    }
    if !(3..=MAX_FACE_NODES).contains(&max_face_nodes) {
        return Err(format!(
            "bir yüz {max_face_nodes} düğümlü olabiliyor; 3 ile {MAX_FACE_NODES} arası okunur."
        ));
    }
    let start_index = f
        .attr("start_index")
        .and_then(|a| a.number())
        .unwrap_or(0.0);
    if start_index != 0.0 && start_index != 1.0 {
        return Err("start_index 0 ya da 1 olmalı.".into());
    }
    let fill = f
        .attr("_FillValue")
        .or_else(|| f.attr("missing_value"))
        .and_then(|a| a.number());
    let geographic = matches!(axis_of_var(x), Axis::Lon) && matches!(axis_of_var(y), Axis::Lat);
    Ok(Topology {
        name: v.name.clone(),
        node_x: x.name.clone(),
        node_y: y.name.clone(),
        faces: f.name.clone(),
        node_dim,
        face_dim,
        max_face_nodes,
        transposed,
        start_index: start_index as i64,
        fill,
        geographic,
    })
}

/// The data variables on the meshes, vectors paired (§4).
pub fn datasets(h: &Header, meshes: &[Topology]) -> Vec<MeshData> {
    let mut found: Vec<MeshData> = Vec::new();
    for v in &h.vars {
        if v.kind == NcType::Char || v.dims.is_empty() {
            continue;
        }
        let Some(mesh) = v.text("mesh").map(str::trim) else {
            continue;
        };
        let Some(t) = meshes.iter().find(|t| t.name == mesh) else {
            continue;
        };
        let location = match v.text("location").map(str::trim) {
            Some("node") => Location::Node,
            Some("face") => Location::Face,
            _ => continue,
        };
        let own = match location {
            Location::Node => t.node_dim,
            Location::Face => t.face_dim,
        };
        if v.dims.last() != Some(&own) || v.dims[..v.dims.len() - 1].contains(&own) {
            continue;
        }
        // Coordinates and the mesh's own arrays are not data.
        if [&t.node_x, &t.node_y, &t.faces].contains(&&v.name)
            || coordinate_of(h, own).is_some_and(|c| c.name == v.name)
        {
            continue;
        }
        if v.text("cf_role").is_some() {
            continue;
        }
        let mask = v
            .text("kentos_mask")
            .map(str::trim)
            .and_then(|m| h.var(m))
            .filter(|m| m.dims.last() == Some(&t.face_dim) && m.dims.len() == v.dims.len())
            .map(|m| m.name.clone());
        found.push(MeshData {
            variable: v.name.clone(),
            vector: None,
            mesh: t.name.clone(),
            location,
            slice_dims: v.dims[..v.dims.len() - 1].to_vec(),
            mask,
        });
    }
    // A mask is not data of its own.
    let masks: Vec<String> = found.iter().filter_map(|d| d.mask.clone()).collect();
    found.retain(|d| !masks.contains(&d.variable));
    pair_vectors(found)
}

/// Whether two names are a vector's components: one position differs, `x` against `y`; or `u` and `v`.
fn components(a: &str, b: &str) -> bool {
    if (a, b) == ("u", "v") {
        return true;
    }
    let (a, b) = (a.as_bytes(), b.as_bytes());
    if a.len() != b.len() {
        return false;
    }
    let mut diff = a.iter().zip(b).enumerate().filter(|(_, (p, q))| p != q);
    match (diff.next(), diff.next()) {
        (Some((_, (p, q))), None) => {
            matches!(
                (p.to_ascii_lowercase(), q.to_ascii_lowercase()),
                (b'x', b'y')
            )
        }
        _ => false,
    }
}

fn pair_vectors(all: Vec<MeshData>) -> Vec<MeshData> {
    let n = all.len();
    let paired = |i: usize, j: usize| {
        all[j].mesh == all[i].mesh
            && all[j].location == all[i].location
            && all[j].slice_dims == all[i].slice_dims
            && all[j].mask == all[i].mask
            && (components(&all[i].variable, &all[j].variable)
                || components(&all[j].variable, &all[i].variable))
    };
    // A variable's one mate, when it has exactly one.
    let mate: Vec<Option<usize>> = (0..n)
        .map(|i| {
            let mut m = (0..n).filter(|&j| j != i && paired(i, j));
            match (m.next(), m.next()) {
                (Some(j), None) => Some(j),
                _ => None,
            }
        })
        .collect();
    let mut out = Vec::new();
    let mut taken = vec![false; n];
    for i in 0..n {
        if taken[i] {
            continue;
        }
        taken[i] = true;
        match mate[i] {
            Some(j) if !taken[j] && mate[j] == Some(i) => {
                taken[j] = true;
                // The x component names the dataset.
                let (x, y) = if components(&all[i].variable, &all[j].variable) {
                    (i, j)
                } else {
                    (j, i)
                };
                let mut d = all[x].clone();
                d.vector = Some(all[y].variable.clone());
                out.push(d);
            }
            _ => out.push(all[i].clone()),
        }
    }
    out
}
