//! The budgets of docs/adr/0243 §12, measured in a release build:
//!
//! ```text
//! cargo test --release -p kentos-raster --test all multidim_timing -- --ignored --nocapture --test-threads=1
//! ```
//!
//! Synthetic files in memory: a header of 1 000 grid variables; a mesh of a
//! million triangles (708 × 708 nodes 10 m apart) with a depth over 24 steps;
//! a mesh of a million nodes (1 000 × 1 000) with the same; the first as 2DM
//! and ASCII DAT; a 1440 × 721 grid of 744 hourly steps whose values are
//! made where they are read (3 GB, never held). Each job is run as a host
//! runs it, the runs it asks for handed over from the bytes. With
//! `KENTOS_PERF_FILES` the files are also written into that folder (the 3 GB
//! grid sparse) for the web's measurement (`scripts/perf/raster.mjs --only
//! multidim`).

use std::time::Instant;

use kentos_contracts::{RasterSample, RasterStretch, RasterStyle};
use kentos_formats::multidim::cube::{Cube, Part, Want};
use kentos_formats::multidim::mesh::grid_of_box;
use kentos_formats::multidim::netcdf::{Attr, NcType};
use kentos_formats::multidim::sms;
use kentos_formats::multidim::ugrid::Location;
use kentos_formats::multidim::write::{self, DatasetOut, TimeOut, UgridOut, VarOut};
use kentos_formats::raster::source::Reader;
use kentos_formats::raster::style::dataset_style;
use kentos_formats::raster::{ByteStore, Step};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::vec2::Vec2;
use kentos_raster::inputs::Input;
use kentos_raster::job::READER_BUDGET;
use kentos_raster::multidim::calc::{CalcSpec, MeshCalc, Summary};
use kentos_raster::multidim::series::{NamedPoint, TimeJob};
use kentos_raster::multidim::{MultidimJob, profile};

/// A file for the web's measurement, when `KENTOS_PERF_FILES` names a folder; `size` past the bytes left sparse.
fn keep(name: &str, bytes: &[u8], size: Option<u64>) {
    let Some(dir) = std::env::var_os("KENTOS_PERF_FILES").map(std::path::PathBuf::from) else {
        return;
    };
    std::fs::create_dir_all(&dir).expect("folder");
    let f = std::fs::File::create(dir.join(name)).expect("file");
    std::io::Write::write_all(&mut &f, bytes).expect("bytes");
    if let Some(n) = size {
        f.set_len(n).expect("size");
    }
}

fn ms(t: Instant) -> f64 {
    t.elapsed().as_secs_f64() * 1000.0
}

fn report(what: &str, took: f64, budget: f64) {
    let mark = if took <= budget { "✓" } else { "✗" };
    println!("{mark} {what}: {took:.1} ms (bütçe {budget} ms)");
}

fn text(s: &str) -> Attr {
    Attr::Text(s.to_owned())
}

/// The bytes `bytes[offset .. offset + len]`.
fn run_of(bytes: &[u8], offset: u64, len: u64) -> Vec<u8> {
    bytes[offset as usize..(offset + len) as usize].to_vec()
}

/// A CDF-2 file of `n` grid variables on 16 × 16 cells, its coordinates.
fn many_variables(n: usize) -> Vec<u8> {
    let dims = vec![("x".to_owned(), 16u64), ("y".to_owned(), 16u64)];
    let coordinate = |name: &str, dim: usize, standard: &str| VarOut {
        name: name.into(),
        dims: vec![dim],
        kind: NcType::Double,
        attrs: vec![
            ("standard_name".into(), text(standard)),
            ("units".into(), text("m")),
        ],
    };
    let mut vars = vec![
        coordinate("x", 0, "projection_x_coordinate"),
        coordinate("y", 1, "projection_y_coordinate"),
    ];
    for k in 0..n {
        vars.push(VarOut {
            name: format!("v{k}"),
            dims: vec![1, 0],
            kind: NcType::Float,
            attrs: vec![
                ("long_name".into(), text(&format!("Değişken {k}"))),
                ("units".into(), text("m")),
            ],
        });
    }
    let layout = write::file_layout(&dims, &[], &vars).expect("layout");
    let mut out = layout.header.clone();
    let mut buf = Vec::new();
    for (k, v) in vars.iter().enumerate() {
        let values: Vec<f64> = match k {
            0 => (0..16).map(|i| 500_005.0 + 10.0 * f64::from(i)).collect(),
            1 => (0..16).map(|j| 4_420_005.0 + 10.0 * f64::from(j)).collect(),
            _ => vec![1.5; 256],
        };
        let mut sink = |b: &[u8]| -> Result<(), String> {
            out.extend_from_slice(b);
            Ok(())
        };
        write::emit(v.kind, &values, &mut sink, &mut buf).expect("values");
        write::emit_pad(layout.sizes[k], &mut sink).expect("pad");
    }
    out
}

/// The nodes of an `a` × `b` grid 10 m apart (odd rows shifted by half a
/// metre) and two triangles a cell.
fn mesh_of(a: usize, b: usize) -> (Vec<f64>, Vec<f64>, Vec<Vec<u32>>) {
    let mut x = Vec::with_capacity(a * b);
    let mut y = Vec::with_capacity(a * b);
    for j in 0..b {
        for i in 0..a {
            x.push(500_000.0 + 10.0 * i as f64 + if j % 2 == 1 { 0.5 } else { 0.0 });
            y.push(4_420_000.0 + 10.0 * j as f64);
        }
    }
    let mut faces = Vec::with_capacity(2 * (a - 1) * (b - 1));
    for j in 0..b - 1 {
        for i in 0..a - 1 {
            let p = (j * a + i) as u32;
            let (q, r, s) = (p + 1, p + a as u32 + 1, p + a as u32);
            faces.push(vec![p, q, r]);
            faces.push(vec![p, r, s]);
        }
    }
    (x, y, faces)
}

/// A UGRID file of [`mesh_of`] with a node depth over `steps` hours.
fn mesh_file(a: usize, b: usize, steps: usize) -> Vec<u8> {
    let (x, y, faces) = mesh_of(a, b);
    let xs = x.clone();
    let m = UgridOut {
        x: &x,
        y: &y,
        faces: &faces,
        geographic: false,
        epsg: Some(5254),
        times: vec![TimeOut {
            values: (0..steps).map(|t| 1.7e12 + 3.6e6 * t as f64).collect(),
            absolute: true,
        }],
        datasets: vec![DatasetOut {
            name: "depth".into(),
            long_name: "Su derinliği".into(),
            location: Location::Node,
            double: false,
            time: Some(0),
            mask: None,
            is_mask: false,
            slab: Box::new(move |s, out: &mut Vec<f64>| {
                out.extend(
                    xs.iter()
                        .map(|&v| libm::sin((v - 500_000.0) * 0.001 + s as f64 * 0.1).abs() * 3.0),
                );
                Ok(())
            }),
        }],
        title: "Süre".into(),
    };
    let mut out = Vec::new();
    write::write_ugrid(m, &mut |b: &[u8]| -> Result<(), String> {
        out.extend_from_slice(b);
        Ok(())
    })
    .expect("mesh");
    out
}

/// The cube of `bytes`: its header, then what `want` needs read from them.
fn cube_with(bytes: &[u8], want: Want<'_>) -> Cube {
    let mut store = ByteStore::new();
    let mut cube = loop {
        match Cube::parse(&store, bytes.len() as u64).expect("header") {
            Step::Done(c) => break c,
            Step::Need(n) => store.put(n.offset, run_of(bytes, n.offset, n.len)),
        }
    };
    for n in cube.needs(want) {
        cube.put(n.offset, run_of(bytes, n.offset, n.len));
    }
    cube
}

/// The mesh's depth at step `slice` on its sanal grid (the default cell).
fn mesh_part(cube: &mut Cube, slice: u32) -> Part {
    let info = cube.info().expect("info");
    let m = &info.meshes[0];
    let (affine, width, height) = grid_of_box(m.bbox, m.cell).expect("grid");
    Part {
        variable: "depth".into(),
        vector: None,
        mesh: Some(m.name.clone()),
        slice: vec![slice],
        affine,
        width,
        height,
    }
}

/// A reader with every block its first pixel wants put from `bytes` (a mesh's slab is one block).
fn filled(cube: &mut Cube, part: &Part, bytes: &[u8]) -> Reader {
    let mut reader = cube.open(part, READER_BUDGET).expect("reader");
    for n in reader.needs(0, 0, 0, 1, 1) {
        reader
            .put_block(&n, &run_of(bytes, n.offset, n.len))
            .expect("block");
    }
    reader
}

/// The look of the depth: Viridis from 0 to 3 m, the mesh's lines when `edges`.
fn look(edges: bool) -> RasterStyle {
    let mut st = dataset_style(RasterSample::F32, true, edges);
    st.stretch = RasterStretch::Manual;
    st.min = Some(0.0);
    st.max = Some(3.0);
    st
}

/// Tile (`tx`, `ty`) of `level` coloured, the least of four runs; and its lines.
fn tile(
    reader: &mut Reader,
    level: usize,
    tx: u32,
    ty: u32,
    affine: &[f64; 6],
    edges: bool,
) -> (f64, usize) {
    let mut best = f64::INFINITY;
    let mut lines = 0;
    let st = look(edges);
    let mut out = Vec::new();
    for _ in 0..4 {
        let t = Instant::now();
        let parts = reader
            .tile_parts_with(level, tx, ty, affine, edges)
            .expect("tile");
        parts.render(&st, None, &mut out);
        best = best.min(ms(t));
        lines = parts.lines.len();
    }
    (best, lines)
}

/// The tiles of the finest level whose width fits a 1 536-pixel view, coloured.
fn view(reader: &mut Reader, part: &Part) -> usize {
    let levels = reader.info.levels as usize;
    let level = (0..levels)
        .find(|&l| (part.width >> l) <= 1536)
        .unwrap_or(levels.saturating_sub(1));
    let (w, h) = ((part.width >> level).max(1), (part.height >> level).max(1));
    let st = look(false);
    let mut out = Vec::new();
    let mut n = 0;
    for ty in 0..h.div_ceil(256) {
        for tx in 0..w.div_ceil(256) {
            let parts = reader
                .tile_parts_with(level, tx, ty, &part.affine, false)
                .expect("tile");
            parts.render(&st, None, &mut out);
            n += 1;
        }
    }
    n
}

/// The same million triangles as 2DM and ASCII DAT (a depth over 24 hours).
fn sms_files(a: usize, b: usize) -> (String, String) {
    let mut twodm = String::from("MESH2D\n");
    let mut e = 1;
    for j in 0..b - 1 {
        for i in 0..a - 1 {
            let p = j * a + i + 1;
            twodm.push_str(&format!("E3T {e} {p} {} {} 1\n", p + 1, p + a + 1));
            twodm.push_str(&format!("E3T {} {p} {} {} 1\n", e + 1, p + a + 1, p + a));
            e += 2;
        }
    }
    for j in 0..b {
        for i in 0..a {
            twodm.push_str(&format!(
                "ND {} {} {} 850.0\n",
                j * a + i + 1,
                500_000 + 10 * i,
                4_420_000 + 10 * j
            ));
        }
    }
    let nodes = a * b;
    let mut dat = format!(
        "DATASET\nOBJTYPE \"mesh2d\"\nBEGSCL\nND {nodes}\nNC {}\nNAME \"Su derinliği\"\nTIMEUNITS Hours\n",
        2 * (a - 1) * (b - 1)
    );
    for s in 0..24 {
        dat.push_str(&format!("TS 0 {s}\n"));
        for k in 0..nodes {
            dat.push_str(&format!(
                "{:.3}\n",
                (k % 97) as f64 * 0.03 + f64::from(s) * 0.01
            ));
        }
    }
    dat.push_str("ENDDS\n");
    (twodm, dat)
}

#[test]
#[ignore]
fn multidim_timing() {
    // NetCDF's header and the variables it lists.
    let many = many_variables(1000);
    keep("md-1000.nc", &many, None);
    let t = Instant::now();
    let mut cube = cube_with(&many, Want::Inspect);
    let info = cube.info().expect("info");
    report(
        "NetCDF başlığı ve değişken listesi (1 000 değişken)",
        ms(t),
        50.0,
    );
    assert_eq!(info.grids.len(), 1000, "{:?}", info.notes);

    // A mesh of a million triangles: its opening, a tile fine and coarse, one with its lines.
    let mesh = mesh_file(708, 708, 24);
    keep("md-mesh-708.nc", &mesh, None);
    let t = Instant::now();
    let mut cube = cube_with(&mesh, Want::Inspect);
    let part = mesh_part(&mut cube, 0);
    let mut reader = filled(&mut cube, &part, &mesh);
    let faces = cube.info().expect("info").meshes[0].faces;
    report(
        &format!("Mesh'in açılışı ({faces} yüz: ağ ve kova dizini)"),
        ms(t),
        1000.0,
    );
    let levels = reader.info.levels as usize;
    let (cx, cy) = (part.width / 2 / 256, part.height / 2 / 256);
    let (fine, _) = tile(&mut reader, 0, cx, cy, &part.affine, false);
    report(
        &format!(
            "Mesh karosu, en ince düzey ({} × {} hücre)",
            part.width, part.height
        ),
        fine,
        30.0,
    );
    let (coarse, _) = tile(&mut reader, levels - 1, 0, 0, &part.affine, false);
    report(
        &format!("Mesh karosu, en kaba düzey ({}. düzey)", levels - 1),
        coarse,
        30.0,
    );
    // The coarsest level the lines show at: a tile with the most of them.
    let lined = (0..levels)
        .rev()
        .find(|&l| {
            let p = reader
                .tile_parts_with(
                    l,
                    (part.width >> l) / 2 / 256,
                    (part.height >> l) / 2 / 256,
                    &part.affine,
                    true,
                )
                .expect("tile");
            !p.lines.is_empty()
        })
        .expect("a level with lines");
    let (edges, lines) = tile(
        &mut reader,
        lined,
        (part.width >> lined) / 2 / 256,
        (part.height >> lined) / 2 / 256,
        &part.affine,
        true,
    );
    report(
        &format!("Ağ çizgili karo ({lined}. düzey, {lines} kenar)"),
        edges,
        40.0,
    );

    // Kesit: ten lines across the mesh, ten thousand points, the values from the mesh.
    let lines: Vec<Shape> = (0..10)
        .map(|k| {
            let y = 4_420_300.0 + 600.0 * f64::from(k);
            Shape::Polyline {
                pts: vec![Vec2::new(500_100.0, y), Vec2::new(506_900.0, y + 80.0)],
                bulges: None,
                holes: None,
                parts: None,
            }
        })
        .collect();
    let input = Input::new(
        cube.open(&part, READER_BUDGET).expect("reader"),
        part.affine,
        None,
    )
    .expect("input");
    let t = Instant::now();
    let mut job = MultidimJob::Points(Box::new(
        profile::job(input, &lines, 6.8, 0, ["Y".into(), "X".into()]).expect("kesit"),
    ));
    while !job.done() {
        for (k, (at, len)) in job.needs().expect("needs").into_iter().enumerate() {
            job.put(k, run_of(&mesh, at, len)).expect("put");
        }
        job.step().expect("step");
    }
    let finished = job.finish(&mut |_| Ok(())).expect("finish");
    report(
        &format!("Kesit ({})", finished.summary.trim_end_matches('.')),
        ms(t),
        500.0,
    );
    drop(reader);
    drop(cube);
    drop(mesh);

    // 2DM and ASCII DAT of the same million triangles, 24 steps, into one UGRID file.
    let (twodm, dat) = sms_files(708, 708);
    keep("md-708.2dm", twodm.as_bytes(), None);
    keep("md-708.dat", dat.as_bytes(), None);
    let t = Instant::now();
    let m = sms::read_2dm(twodm.as_bytes()).expect("2dm");
    let d = sms::read_dat(dat.as_bytes()).expect("dat");
    let mut written = 0usize;
    let made = write::write_sms(
        &m,
        &[("derinlik.dat".into(), d)],
        None,
        Some(5254),
        false,
        &mut |b: &[u8]| {
            written += b.len();
            Ok(())
        },
    )
    .expect("write");
    report(
        &format!(
            "2DM ve DAT içe aktarma ({} yüz, 24 adım, {} MB yazıldı)",
            made.faces,
            written >> 20
        ),
        ms(t),
        5000.0,
    );
    drop((twodm, dat, m));

    // A mesh of a million nodes: the first view when the time step changes, then Mesh hesaplayıcı's largest.
    let big = mesh_file(1000, 1000, 24);
    keep("md-mesh-1000.nc", &big, None);
    let mut cube = cube_with(&big, Want::Inspect);
    let first = mesh_part(&mut cube, 0);
    let mut reader = filled(&mut cube, &first, &big);
    let _ = view(&mut reader, &first);
    let t = Instant::now();
    let step = mesh_part(&mut cube, 7);
    let mut reader = filled(&mut cube, &step, &big);
    let tiles = view(&mut reader, &step);
    report(
        &format!("Zaman adımı değişince ilk görünüm (1 000 000 düğüm, {tiles} karo)"),
        ms(t),
        500.0,
    );
    drop(reader);
    let t = Instant::now();
    let spec = CalcSpec {
        expression: "depth * 2".into(),
        summary: Summary::Max,
        name: "En büyük".into(),
    };
    let mut calc = MeshCalc::new(&mut cube, &first, &spec).expect("calc");
    while !calc.done() {
        for (k, (at, len)) in calc.needs().expect("needs").into_iter().enumerate() {
            calc.put(k, run_of(&big, at, len)).expect("put");
        }
        calc.step().expect("step");
    }
    let mut written = 0usize;
    let finished = calc
        .finish(&mut |b: &[u8]| {
            written += b.len();
            Ok(())
        })
        .expect("finish");
    report(
        &format!(
            "Mesh hesaplayıcı ({}; {} MB yazıldı)",
            finished.summary.trim_end_matches('.'),
            written >> 20
        ),
        ms(t),
        3000.0,
    );
    drop(cube);
    drop(big);

    // Zaman serisi on a 1440 × 721 grid of 744 hours: its values made where they are read.
    let (nx, ny, nt) = (1440u64, 721u64, 744u64);
    let dims = vec![
        ("lon".to_owned(), nx),
        ("lat".to_owned(), ny),
        ("time".to_owned(), nt),
    ];
    let axis = |name: &str, dim: usize, units: &str| VarOut {
        name: name.into(),
        dims: vec![dim],
        kind: NcType::Double,
        attrs: vec![("units".into(), text(units))],
    };
    let vars = vec![
        axis("lon", 0, "degrees_east"),
        axis("lat", 1, "degrees_north"),
        axis("time", 2, "hours since 2024-01-01 00:00:00"),
        VarOut {
            name: "t2m".into(),
            dims: vec![2, 1, 0],
            kind: NcType::Float,
            attrs: vec![("units".into(), text("K"))],
        },
    ];
    let layout = write::file_layout(&dims, &[], &vars).expect("layout");
    let mut head = layout.header.clone();
    let coords = [
        (0..nx)
            .map(|i| -180.0 + 0.25 * i as f64)
            .collect::<Vec<f64>>(),
        (0..ny).map(|j| 90.0 - 0.25 * j as f64).collect(),
        (0..nt).map(|k| k as f64).collect(),
    ];
    for (k, c) in coords.iter().enumerate() {
        head.resize(layout.begins[k] as usize, 0);
        write::encode(NcType::Double, c, &mut head);
    }
    // The file's size from its last variable; only the header and coordinates are held.
    let size = layout.begins[3] + layout.sizes[3];
    keep("md-era5.nc", &head, Some(size));
    let mut store = ByteStore::new();
    store.put(0, head.clone());
    let mut cube = match Cube::parse(&store, size).expect("header") {
        Step::Done(c) => c,
        Step::Need(n) => panic!(
            "the header is whole; {} bytes at {} asked for",
            n.len, n.offset
        ),
    };
    for n in cube.needs(Want::Inspect) {
        cube.put(n.offset, run_of(&head, n.offset, n.len));
    }
    let grid = cube
        .info()
        .expect("info")
        .grids
        .into_iter()
        .find(|g| g.variable == "t2m")
        .expect("t2m");
    let affine = grid.affine.expect("placed");
    let part = Part {
        variable: "t2m".into(),
        vector: None,
        mesh: None,
        slice: vec![0],
        affine,
        width: grid.width,
        height: grid.height,
    };
    let points: Vec<NamedPoint> = (0..100)
        .map(|k| NamedPoint {
            name: Some(format!("N{k}")),
            x: -170.0 + 3.37 * f64::from(k),
            y: -60.0 + 1.21 * f64::from(k),
        })
        .collect();
    let xy: Vec<[f64; 2]> = points.iter().map(|p| [p.x, p.y]).collect();
    let begin = layout.begins[3];
    let t = Instant::now();
    let mut series = cube.series(&part, affine, None, &xy).expect("series");
    let runs = series.runs.len();
    let mut bytes = Vec::new();
    for run in series.runs.clone() {
        let first = (run.offset - begin) / 4;
        let values: Vec<f64> = (0..run.len / 4)
            .map(|v| 250.0 + ((first + v) % 50) as f64)
            .collect();
        bytes.clear();
        write::encode(NcType::Float, &values, &mut bytes);
        series.put(run.offset, bytes.clone());
    }
    let mut job = TimeJob::new(series, &points).expect("job");
    job.needs();
    job.step().expect("step");
    let finished = job.finish().expect("finish");
    report(
        &format!(
            "Zaman serisi ({}; {runs} okuma)",
            finished.summary.trim_end_matches('.')
        ),
        ms(t),
        2000.0,
    );
}
