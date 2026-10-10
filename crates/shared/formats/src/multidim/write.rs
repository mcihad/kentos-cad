//! Writing classic NetCDF (docs/adr/0243 §2, §4, §10): a header of fixed-size
//! variables laid one after another (CDF-2, CDF-5 when a variable passes
//! 4 GiB), their values big-endian; a UGRID mesh with its datasets; 2DM and
//! ASCII DAT made one UGRID file. Values go to a sink as they are made, so a
//! long dataset is never held twice.

use super::netcdf::{Attr, NcType, pad};
use super::sms::{DatDataset, Sms2dm};
use super::ugrid::Location;
use crate::raster::RasterError;

/// Where the bytes go.
pub type Sink<'a> = dyn FnMut(&[u8]) -> Result<(), String> + 'a;

/// A variable to write.
#[derive(Clone, Debug, PartialEq)]
pub struct VarOut {
    pub name: String,
    pub dims: Vec<usize>,
    pub kind: NcType,
    pub attrs: Vec<(String, Attr)>,
}

/// A file's header and where its variables' values begin.
#[derive(Clone, Debug, PartialEq)]
pub struct FileLayout {
    pub version: u8,
    pub header: Vec<u8>,
    pub begins: Vec<u64>,
    /// Each variable's values' bytes (unpadded).
    pub sizes: Vec<u64>,
}

fn put_count(out: &mut Vec<u8>, five: bool, n: u64) {
    if five {
        out.extend_from_slice(&n.to_be_bytes());
    } else {
        out.extend_from_slice(&(n as u32).to_be_bytes());
    }
}

fn put_name(out: &mut Vec<u8>, five: bool, s: &str) {
    let b = s.as_bytes();
    put_count(out, five, b.len() as u64);
    out.extend_from_slice(b);
    out.resize(
        out.len() + (pad(b.len() as u64) - b.len() as u64) as usize,
        0,
    );
}

fn put_attrs(out: &mut Vec<u8>, five: bool, attrs: &[(String, Attr)]) {
    if attrs.is_empty() {
        out.extend_from_slice(&0i32.to_be_bytes());
        put_count(out, five, 0);
        return;
    }
    out.extend_from_slice(&12i32.to_be_bytes());
    put_count(out, five, attrs.len() as u64);
    for (k, v) in attrs {
        put_name(out, five, k);
        match v {
            Attr::Text(s) => {
                let b = s.as_bytes();
                out.extend_from_slice(&NcType::Char.code().to_be_bytes());
                put_count(out, five, b.len() as u64);
                out.extend_from_slice(b);
                out.resize(
                    out.len() + (pad(b.len() as u64) - b.len() as u64) as usize,
                    0,
                );
            }
            Attr::Numbers { kind, values } => {
                out.extend_from_slice(&kind.code().to_be_bytes());
                put_count(out, five, values.len() as u64);
                let start = out.len();
                encode(*kind, values, out);
                let n = (out.len() - start) as u64;
                out.resize(out.len() + (pad(n) - n) as usize, 0);
            }
        }
    }
}

/// Values as big-endian bytes of `kind` (NaN as the type's default fill).
pub fn encode(kind: NcType, values: &[f64], out: &mut Vec<u8>) {
    out.reserve(values.len() * kind.size() as usize);
    let fill = kind.default_fill();
    for &v in values {
        let v = if v.is_nan() { fill } else { v };
        match kind {
            NcType::Byte => out.push(v as i8 as u8),
            NcType::Char | NcType::UByte => out.push(v as u8),
            NcType::Short => out.extend_from_slice(&(v as i16).to_be_bytes()),
            NcType::UShort => out.extend_from_slice(&(v as u16).to_be_bytes()),
            NcType::Int => out.extend_from_slice(&(v as i32).to_be_bytes()),
            NcType::UInt => out.extend_from_slice(&(v as u32).to_be_bytes()),
            NcType::Float => out.extend_from_slice(&(v as f32).to_be_bytes()),
            NcType::Double => out.extend_from_slice(&v.to_be_bytes()),
            NcType::Int64 => out.extend_from_slice(&(v as i64).to_be_bytes()),
            NcType::UInt64 => out.extend_from_slice(&(v as u64).to_be_bytes()),
        }
    }
}

/// The header of a file of fixed-size variables, and where each begins.
pub fn file_layout(
    dims: &[(String, u64)],
    gattrs: &[(String, Attr)],
    vars: &[VarOut],
) -> Result<FileLayout, String> {
    let sizes: Vec<u64> = vars
        .iter()
        .map(|v| {
            v.dims
                .iter()
                .try_fold(v.kind.size(), |a, &d| {
                    dims.get(d).and_then(|x| a.checked_mul(x.1))
                })
                .ok_or_else(|| format!("“{}” değişkeni yazılamayacak kadar büyük.", v.name))
        })
        .collect::<Result<_, _>>()?;
    let version = if sizes.iter().any(|&s| pad(s) > 0xFFFF_FFFC) {
        5
    } else {
        2
    };
    let five = version == 5;
    let build = |begins: &[u64]| {
        let mut out = vec![b'C', b'D', b'F', version];
        if five {
            out.extend_from_slice(&0u64.to_be_bytes());
        } else {
            out.extend_from_slice(&0u32.to_be_bytes());
        }
        if dims.is_empty() {
            out.extend_from_slice(&0i32.to_be_bytes());
            put_count(&mut out, five, 0);
        } else {
            out.extend_from_slice(&10i32.to_be_bytes());
            put_count(&mut out, five, dims.len() as u64);
            for (name, len) in dims {
                put_name(&mut out, five, name);
                put_count(&mut out, five, *len);
            }
        }
        put_attrs(&mut out, five, gattrs);
        if vars.is_empty() {
            out.extend_from_slice(&0i32.to_be_bytes());
            put_count(&mut out, five, 0);
        } else {
            out.extend_from_slice(&11i32.to_be_bytes());
            put_count(&mut out, five, vars.len() as u64);
            for (k, v) in vars.iter().enumerate() {
                put_name(&mut out, five, &v.name);
                put_count(&mut out, five, v.dims.len() as u64);
                for &d in &v.dims {
                    put_count(&mut out, five, d as u64);
                }
                put_attrs(&mut out, five, &v.attrs);
                out.extend_from_slice(&v.kind.code().to_be_bytes());
                put_count(
                    &mut out,
                    five,
                    pad(sizes[k]).min(if five { u64::MAX } else { 0xFFFF_FFFF }),
                );
                out.extend_from_slice(&begins[k].to_be_bytes());
            }
        }
        out
    };
    let zeros = vec![0u64; vars.len()];
    let len = build(&zeros).len() as u64;
    let mut begins = Vec::with_capacity(vars.len());
    let mut at = len;
    for s in &sizes {
        begins.push(at);
        at += pad(*s);
    }
    Ok(FileLayout {
        version,
        header: build(&begins),
        begins,
        sizes,
    })
}

/// Writes `values` (of `kind`) to `sink`, padded to four when `last` ends the variable of `total` bytes.
pub fn emit(
    kind: NcType,
    values: &[f64],
    sink: &mut Sink<'_>,
    buf: &mut Vec<u8>,
) -> Result<(), String> {
    buf.clear();
    encode(kind, values, buf);
    sink(buf)
}

/// Pads a variable of `bytes` bytes to four.
pub fn emit_pad(bytes: u64, sink: &mut Sink<'_>) -> Result<(), String> {
    let n = (pad(bytes) - bytes) as usize;
    if n > 0 { sink(&[0u8; 4][..n]) } else { Ok(()) }
}

/// A dataset's values for a step, one per place, pushed onto the vector.
pub type Slab<'a> = Box<dyn FnMut(usize, &mut Vec<f64>) -> Result<(), String> + 'a>;

/// A dataset to write on a mesh.
pub struct DatasetOut<'a> {
    /// The variable's name (ASCII) and the name shown (`long_name`).
    pub name: String,
    pub long_name: String,
    pub location: Location,
    pub double: bool,
    /// Its time axis (index into the file's), none: static.
    pub time: Option<usize>,
    /// A face mask's variable name it names (`kentos_mask`).
    pub mask: Option<String>,
    /// Whether it is a mask (bytes, 1 active).
    pub is_mask: bool,
    /// Its values for step `k` (one per place).
    pub slab: Slab<'a>,
}

/// A time axis to write: moments (`absolute`: ms since 1970) or hours from the start.
#[derive(Clone, Debug, PartialEq)]
pub struct TimeOut {
    pub values: Vec<f64>,
    pub absolute: bool,
}

/// A mesh to write.
pub struct UgridOut<'a> {
    pub x: &'a [f64],
    pub y: &'a [f64],
    pub faces: &'a [Vec<u32>],
    pub geographic: bool,
    pub epsg: Option<u32>,
    pub times: Vec<TimeOut>,
    pub datasets: Vec<DatasetOut<'a>>,
    pub title: String,
}

/// The mesh topology variable a written UGRID file names (`Mesh2d`).
pub const MESH: &str = "Mesh2d";

/// A UGRID file of the mesh and its datasets into `sink`.
pub fn write_ugrid(mut m: UgridOut<'_>, sink: &mut Sink<'_>) -> Result<(), String> {
    let (n, f) = (m.x.len() as u64, m.faces.len() as u64);
    let most = m.faces.iter().map(Vec::len).max().unwrap_or(3).max(3) as u64;
    let mut dims: Vec<(String, u64)> = vec![
        ("nMesh2d_node".into(), n),
        ("nMesh2d_face".into(), f),
        ("max_nMesh2d_face_nodes".into(), most),
    ];
    let time_dim: Vec<usize> = m
        .times
        .iter()
        .enumerate()
        .map(|(k, t)| {
            dims.push((
                if k == 0 {
                    "time".to_owned()
                } else {
                    format!("time_{}", k + 1)
                },
                t.values.len() as u64,
            ));
            dims.len() - 1
        })
        .collect();
    let text = |s: &str| Attr::Text(s.to_owned());
    let mut vars = vec![VarOut {
        name: MESH.into(),
        dims: vec![],
        kind: NcType::Int,
        attrs: vec![
            ("cf_role".into(), text("mesh_topology")),
            ("long_name".into(), text("Topology data of 2D mesh")),
            (
                "topology_dimension".into(),
                Attr::Numbers {
                    kind: NcType::Int,
                    values: vec![2.0],
                },
            ),
            (
                "node_coordinates".into(),
                text("Mesh2d_node_x Mesh2d_node_y"),
            ),
            ("face_node_connectivity".into(), text("Mesh2d_face_nodes")),
            ("face_dimension".into(), text("nMesh2d_face")),
        ],
    }];
    let (xs, ys, unit) = if m.geographic {
        ("longitude", "latitude", ("degrees_east", "degrees_north"))
    } else {
        (
            "projection_x_coordinate",
            "projection_y_coordinate",
            ("m", "m"),
        )
    };
    vars.push(VarOut {
        name: "Mesh2d_node_x".into(),
        dims: vec![0],
        kind: NcType::Double,
        attrs: vec![
            ("standard_name".into(), text(xs)),
            ("units".into(), text(unit.0)),
        ],
    });
    vars.push(VarOut {
        name: "Mesh2d_node_y".into(),
        dims: vec![0],
        kind: NcType::Double,
        attrs: vec![
            ("standard_name".into(), text(ys)),
            ("units".into(), text(unit.1)),
        ],
    });
    vars.push(VarOut {
        name: "Mesh2d_face_nodes".into(),
        dims: vec![1, 2],
        kind: NcType::Int,
        attrs: vec![
            ("cf_role".into(), text("face_node_connectivity")),
            (
                "start_index".into(),
                Attr::Numbers {
                    kind: NcType::Int,
                    values: vec![0.0],
                },
            ),
            (
                "_FillValue".into(),
                Attr::Numbers {
                    kind: NcType::Int,
                    values: vec![-1.0],
                },
            ),
        ],
    });
    let crs = m.epsg.map(|e| {
        vars.push(VarOut {
            name: "crs".into(),
            dims: vec![],
            kind: NcType::Int,
            attrs: vec![("epsg_code".into(), text(&format!("EPSG:{e}")))],
        });
        "crs"
    });
    if let Some(c) = crs {
        vars[0].attrs.push(("grid_mapping".into(), text(c)));
    }
    for (k, t) in m.times.iter().enumerate() {
        let units = if t.absolute {
            "seconds since 1970-01-01 00:00:00"
        } else {
            "hours"
        };
        let mut attrs = vec![
            ("long_name".into(), text("Zaman")),
            ("units".into(), text(units)),
        ];
        if t.absolute {
            attrs.insert(0, ("standard_name".into(), text("time")));
            attrs.push(("calendar".into(), text("standard")));
        }
        vars.push(VarOut {
            name: dims[time_dim[k]].0.clone(),
            dims: vec![time_dim[k]],
            kind: NcType::Double,
            attrs,
        });
    }
    let first_data = vars.len();
    for d in &m.datasets {
        let place = match d.location {
            Location::Node => 0,
            Location::Face => 1,
        };
        let mut vd = Vec::new();
        if let Some(t) = d.time {
            vd.push(time_dim[t]);
        }
        vd.push(place);
        let kind = if d.is_mask {
            NcType::Byte
        } else if d.double {
            NcType::Double
        } else {
            NcType::Float
        };
        let mut attrs = vec![
            ("long_name".into(), text(&d.long_name)),
            ("mesh".into(), text(MESH)),
            ("location".into(), text(d.location.name())),
        ];
        if !d.is_mask {
            attrs.push((
                "_FillValue".into(),
                Attr::Numbers {
                    kind,
                    values: vec![kind.default_fill()],
                },
            ));
        }
        if let Some(mask) = &d.mask {
            attrs.push(("kentos_mask".into(), text(mask)));
        }
        if let Some(c) = crs {
            attrs.push(("grid_mapping".into(), text(c)));
        }
        vars.push(VarOut {
            name: d.name.clone(),
            dims: vd,
            kind,
            attrs,
        });
    }
    let gattrs = vec![
        ("Conventions".into(), text("CF-1.8 UGRID-1.0")),
        ("title".into(), text(&m.title)),
        ("source".into(), text("KentOS")),
    ];
    let layout = file_layout(&dims, &gattrs, &vars)?;
    sink(&layout.header)?;
    let mut buf = Vec::new();
    // Mesh2d and its coordinates.
    emit(NcType::Int, &[0.0], sink, &mut buf)?;
    emit(NcType::Double, m.x, sink, &mut buf)?;
    emit(NcType::Double, m.y, sink, &mut buf)?;
    let mut nodes = Vec::with_capacity(m.faces.len() * most as usize);
    for face in m.faces {
        nodes.extend(face.iter().map(|&k| f64::from(k)));
        nodes.extend(std::iter::repeat_n(-1.0, most as usize - face.len()));
    }
    emit(NcType::Int, &nodes, sink, &mut buf)?;
    drop(nodes);
    if crs.is_some() {
        emit(NcType::Int, &[0.0], sink, &mut buf)?;
    }
    for t in &m.times {
        let v: Vec<f64> = if t.absolute {
            t.values.iter().map(|&ms| ms / 1000.0).collect()
        } else {
            t.values.clone()
        };
        emit(NcType::Double, &v, sink, &mut buf)?;
    }
    let mut slab = Vec::new();
    for (k, d) in m.datasets.iter_mut().enumerate() {
        let var = &vars[first_data + k];
        let steps = d.time.map_or(1, |t| m.times[t].values.len());
        let places = if d.location == Location::Node { n } else { f } as usize;
        for s in 0..steps {
            slab.clear();
            (d.slab)(s, &mut slab)?;
            if slab.len() != places {
                return Err(format!(
                    "“{}” veri setinin {}. adımında {} değer var; {places} olmalı.",
                    d.long_name,
                    s + 1,
                    slab.len()
                ));
            }
            emit(var.kind, &slab, sink, &mut buf)?;
        }
        emit_pad(layout.sizes[first_data + k], sink)?;
    }
    Ok(())
}

/// A NetCDF variable's name from a name shown: Turkish letters as ASCII,
/// anything else not a letter, digit or `_` as `_`, a letter first.
pub fn variable_name(shown: &str) -> String {
    let mut s: String = shown
        .trim()
        .chars()
        .map(|c| match c {
            'ç' => 'c',
            'Ç' => 'C',
            'ğ' => 'g',
            'Ğ' => 'G',
            'ı' => 'i',
            'İ' => 'I',
            'ö' => 'o',
            'Ö' => 'O',
            'ş' => 's',
            'Ş' => 'S',
            'ü' => 'u',
            'Ü' => 'U',
            c if c.is_ascii_alphanumeric() || c == '_' => c,
            _ => '_',
        })
        .collect();
    if !s.starts_with(|c: char| c.is_ascii_alphabetic()) {
        s.insert_str(0, "v_");
    }
    s.truncate(200);
    s
}

/// What a 2DM and its DATs made.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SmsReport {
    pub nodes: usize,
    pub faces: usize,
    /// The datasets written, by their names shown.
    pub datasets: Vec<String>,
    pub notes: Vec<String>,
}

/// One UGRID file of a 2DM and its DATs (§4): Taban kotu from the nodes'
/// heights, each DAT dataset its variable (a vector two, `_x` and `_y`),
/// an activity a face mask; times from `start` (ms since 1970) or the DAT's
/// `RT_JULIAN`, else hours from the start.
pub fn write_sms(
    mesh: &Sms2dm,
    dats: &[(String, Vec<DatDataset>)],
    start: Option<f64>,
    epsg: Option<u32>,
    geographic: bool,
    sink: &mut Sink<'_>,
) -> Result<SmsReport, RasterError> {
    let (n, f) = (mesh.x.len(), mesh.faces.len());
    let mut report = SmsReport {
        nodes: n,
        faces: f,
        ..SmsReport::default()
    };
    if mesh.skipped > 0 {
        report
            .notes
            .push(format!("{} çizgi elemanı atlandı.", mesh.skipped));
    }
    // Every dataset with its file's name for the messages, its place and time axis.
    let mut all: Vec<(&DatDataset, Location)> = Vec::new();
    for (file, list) in dats {
        for d in list {
            let location = if d.count == n {
                Location::Node
            } else if d.count == f {
                Location::Face
            } else {
                return Err(RasterError::new(format!(
                    "“{file}” dosyasının “{}” veri setinin {} değeri var; ağın {n} düğümü ve {f} yüzü var.",
                    d.name, d.count
                )));
            };
            if d.steps.iter().any(|s| s.active.is_some()) && d.cells != f {
                return Err(RasterError::new(format!(
                    "“{file}” dosyasının “{}” veri setinin etkinlikleri {} eleman için; ağın {f} yüzü var.",
                    d.name, d.cells
                )));
            }
            all.push((d, location));
        }
    }
    // Time axes: the datasets' times as moments or hours, the same ones shared.
    let mut times: Vec<TimeOut> = Vec::new();
    let mut axis_of: Vec<usize> = Vec::new();
    for (d, _) in &all {
        let base = start.or(d.reference);
        let values: Vec<f64> = d
            .steps
            .iter()
            .map(|s| match base {
                Some(b) => b + (s.time * d.unit_ms).round(),
                None => s.time * d.unit_ms / 3_600_000.0,
            })
            .collect();
        let t = TimeOut {
            values,
            absolute: base.is_some(),
        };
        let k = match times.iter().position(|x| *x == t) {
            Some(k) => k,
            None => {
                times.push(t);
                times.len() - 1
            }
        };
        axis_of.push(k);
    }
    if times.iter().any(|t| !t.absolute) && !all.is_empty() {
        report.notes.push(
            "Başlangıç zamanı verilmedi: zamanlar başlangıçtan saat; zaman sürgüsü izlenemez."
                .into(),
        );
    }
    let mut taken: Vec<String> = vec!["taban_kotu".into()];
    let mut unique = |base: String| {
        let mut name = base.clone();
        let mut k = 2;
        while taken.contains(&name)
            || [
                "Mesh2d",
                "Mesh2d_node_x",
                "Mesh2d_node_y",
                "Mesh2d_face_nodes",
                "crs",
                "time",
            ]
            .contains(&name.as_str())
            || name.starts_with("time_")
        {
            name = format!("{base}_{k}");
            k += 1;
        }
        taken.push(name.clone());
        name
    };
    let mut datasets: Vec<DatasetOut<'_>> = vec![DatasetOut {
        name: "taban_kotu".into(),
        long_name: "Taban kotu".into(),
        location: Location::Node,
        double: true,
        time: None,
        mask: None,
        is_mask: false,
        slab: Box::new(|_, out| {
            out.extend_from_slice(&mesh.z);
            Ok(())
        }),
    }];
    report.datasets.push("Taban kotu".into());
    for (k, (d, location)) in all.iter().enumerate() {
        let base = unique(variable_name(&d.name));
        let masked = d.steps.iter().any(|s| s.active.is_some());
        let mask_name = masked.then(|| format!("{base}_etkin"));
        let comps: &[(usize, &str)] = if d.vector {
            &[(0, "_x"), (1, "_y")]
        } else {
            &[(0, "")]
        };
        for &(c, suffix) in comps {
            let dd: &DatDataset = d;
            let per = if dd.vector { 2 } else { 1 };
            datasets.push(DatasetOut {
                name: format!("{base}{suffix}"),
                long_name: if dd.vector {
                    format!("{} ({})", dd.name, if c == 0 { "x" } else { "y" })
                } else {
                    dd.name.clone()
                },
                location: *location,
                double: false,
                time: Some(axis_of[k]),
                mask: mask_name.clone(),
                is_mask: false,
                slab: Box::new(move |s, out| {
                    let st = &dd.steps[s];
                    out.extend(st.values.iter().skip(c).step_by(per).map(|&v| f64::from(v)));
                    Ok(())
                }),
            });
        }
        if let Some(m) = &mask_name {
            let dd: &DatDataset = d;
            datasets.push(DatasetOut {
                name: m.clone(),
                long_name: format!("{} etkinliği", dd.name),
                location: Location::Face,
                double: false,
                time: Some(axis_of[k]),
                mask: None,
                is_mask: true,
                slab: Box::new(move |s, out| {
                    match &dd.steps[s].active {
                        Some(a) => out.extend(a.iter().map(|&v| f64::from(v))),
                        None => out.extend(std::iter::repeat_n(1.0, dd.cells)),
                    }
                    Ok(())
                }),
            });
        }
        report.datasets.push(if d.vector {
            format!("{} (vektör)", d.name)
        } else {
            d.name.clone()
        });
    }
    let out = UgridOut {
        x: &mesh.x,
        y: &mesh.y,
        faces: &mesh.faces,
        geographic,
        epsg,
        times,
        datasets,
        title: "2DM ve DAT".into(),
    };
    write_ugrid(out, sink).map_err(RasterError::new)?;
    Ok(report)
}
