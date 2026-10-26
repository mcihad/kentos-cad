//! NetCDF-4 (HDF5), Zarr and GRIB files read by KentOS's own code as classic NetCDF (docs/adr/0645): every dimension,
//! variable, value and attribute as the formats' own libraries read them (fixtures/multidim/v2/cases.json, written by
//! scripts/fixtures/chunked_formats_cases.py with netCDF4-python, h5py, zarr-python and ecCodes).

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use kentos_formats::multidim::chunked::{Fetch, VirtualFile, open_file, open_zarr};
use kentos_formats::multidim::netcdf::{self, Attr};
use serde_json::Value;

fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/multidim/v2")
}

struct Local {
    path: PathBuf,
}

impl Fetch for Local {
    fn range(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        let mut f = File::open(&self.path).map_err(|e| e.to_string())?;
        f.seek(SeekFrom::Start(offset)).map_err(|e| e.to_string())?;
        let mut b = vec![0; len as usize];
        f.read_exact(&mut b).map_err(|e| e.to_string())?;
        Ok(b)
    }

    fn object(&self, key: &str) -> Result<Option<Vec<u8>>, String> {
        match std::fs::read(self.path.join(key)) {
            Ok(b) => Ok(Some(b)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.to_string()),
        }
    }
}

fn open(path: &Path) -> VirtualFile {
    let fetch: Arc<dyn Fetch> = Arc::new(Local {
        path: path.to_owned(),
    });
    if path.is_dir() {
        let dir = path.to_owned();
        let list = move |prefix: &str| -> Option<Vec<String>> {
            let mut out = Vec::new();
            for e in std::fs::read_dir(dir.join(prefix)).ok()? {
                let e = e.ok()?;
                let n = e.file_name().to_string_lossy().into_owned();
                if e.path().is_dir() && !n.starts_with('.') {
                    out.push(n);
                }
            }
            Some(out)
        };
        open_zarr(fetch, &list).unwrap_or_else(|e| panic!("{}: {e}", path.display()))
    } else {
        let size = std::fs::metadata(path).unwrap().len();
        let head = fetch.range(0, 8.min(size)).unwrap();
        open_file(fetch, size, &head)
            .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
            .unwrap_or_else(|| panic!("{}: tanınmadı", path.display()))
    }
}

fn same(a: f64, b: &Value, rel: f64) -> bool {
    match b {
        Value::Null => a.is_nan(),
        Value::String(s) if s == "inf" => a == f64::INFINITY,
        Value::String(s) if s == "-inf" => a == f64::NEG_INFINITY,
        _ => {
            let b = b.as_f64().unwrap();
            a == b || (a - b).abs() <= rel * b.abs().max(1.0)
        }
    }
}

#[test]
fn chunked_files_read_as_their_libraries_read_them() {
    let cases: Value =
        serde_json::from_str(&std::fs::read_to_string(root().join("cases.json")).unwrap()).unwrap();
    for c in cases["files"].as_array().unwrap() {
        let name = c["file"].as_str().unwrap();
        let vf = open(&root().join(name));
        assert_eq!(vf.model().format, c["format"].as_str().unwrap(), "{name}");
        let bytes = vf.read(0, vf.size()).unwrap();
        let h = netcdf::parse_bytes(&bytes).unwrap_or_else(|e| panic!("{name}: {}", e.0));
        let dims: Vec<(String, u64)> = h.dims.iter().map(|d| (d.name.clone(), d.len)).collect();
        let want: Vec<(String, u64)> = c["dims"]
            .as_array()
            .unwrap()
            .iter()
            .map(|d| (d[0].as_str().unwrap().to_owned(), d[1].as_u64().unwrap()))
            .collect();
        let mut a = dims.clone();
        let mut b = want.clone();
        a.sort();
        b.sort();
        assert_eq!(a, b, "{name}: boyutlar");
        let grib = name.contains("grib");
        let mut names: Vec<&str> = h.vars.iter().map(|v| v.name.as_str()).collect();
        names.sort();
        let wanted: Vec<&str> = c["vars"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            wanted,
            "{name}: değişkenler ({:?})",
            vf.model().notes
        );
        for w in c["vars"].as_array().unwrap() {
            let vn = w["name"].as_str().unwrap();
            let v = h.var(vn).unwrap();
            let vdims: Vec<&str> = v.dims.iter().map(|&d| h.dims[d].name.as_str()).collect();
            let wdims: Vec<&str> = w["dims"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap())
                .collect();
            assert_eq!(vdims, wdims, "{name}/{vn}: boyutları");
            assert_eq!(
                v.kind.name(),
                w["kind"].as_str().unwrap(),
                "{name}/{vn}: türü"
            );
            let mut got = Vec::new();
            for r in h.runs(v) {
                netcdf::decode(
                    v.kind,
                    &bytes[r.offset as usize..(r.offset + r.len) as usize],
                    &mut got,
                );
            }
            let wv = w["values"].as_array().unwrap();
            assert_eq!(got.len(), wv.len(), "{name}/{vn}: değer sayısı");
            let rel = if grib { 1e-9 } else { 0.0 };
            for (k, (g, e)) in got.iter().zip(wv).enumerate() {
                assert!(same(*g, e, rel), "{name}/{vn}[{k}]: {g} ≠ {e}");
            }
            for (k, a) in w["attrs"].as_object().unwrap() {
                let got = v
                    .attr(k)
                    .unwrap_or_else(|| panic!("{name}/{vn}: “{k}” özniteliği yok"));
                match (got, a.get("text"), a.get("numbers")) {
                    (Attr::Text(t), Some(e), _) => assert_eq!(
                        t.trim_end_matches('\0'),
                        e.as_str().unwrap(),
                        "{name}/{vn}.{k}"
                    ),
                    (Attr::Numbers { values, .. }, _, Some(e)) => {
                        let e = e.as_array().unwrap();
                        assert_eq!(values.len(), e.len(), "{name}/{vn}.{k}");
                        for (x, y) in values.iter().zip(e) {
                            assert!(same(*x, y, 1e-12), "{name}/{vn}.{k}: {x} ≠ {y}");
                        }
                    }
                    other => panic!("{name}/{vn}.{k}: {other:?} ≠ {a}"),
                }
            }
        }
    }
}

/// The header of a virtual file, read as a host reads it (the parser asks for what it needs).
fn header_of(vf: &VirtualFile) -> netcdf::Header {
    let mut store = kentos_formats::raster::ByteStore::new();
    loop {
        match netcdf::parse(&store, vf.size()).unwrap() {
            kentos_formats::raster::Step::Done(h) => return h,
            kentos_formats::raster::Step::Need(n) => {
                store.put(n.offset, vf.read(n.offset, n.len).unwrap())
            }
        }
    }
}

/// docs/adr/0645 §8: a 48 × 721 × 1440 float field as NetCDF-4 and Zarr and 24 GRIB2 messages of the 0.25° grid
/// (`scripts/perf/chunked_formats.py` writes them into .run/perf/chunked): opening, one step's field, a point's
/// series and the whole variable, each from a fresh open (no chunk kept).
#[test]
#[ignore = "timings, run by hand in release"]
fn chunked_timing() {
    use std::time::Instant;
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.run/perf/chunked");
    for (name, var) in [
        ("era.nc", "t2m"),
        ("era.zarr", "t2m"),
        ("gfs.grib2", "t_2m"),
    ] {
        let path = dir.join(name);
        if !path.exists() {
            println!("{name}: yok; önce scripts/perf/chunked_formats.py");
            continue;
        }
        let t = Instant::now();
        let vf = open(&path);
        let h = header_of(&vf);
        let opened = t.elapsed();
        let v = h.var(var).unwrap_or_else(|| {
            panic!(
                "{name}: {var} yok; var olanlar {:?}",
                h.vars.iter().map(|v| &v.name).collect::<Vec<_>>()
            )
        });
        let shape = h.shape(v);
        let (ny, nx) = (shape[shape.len() - 2], shape[shape.len() - 1]);
        let mut index = vec![0u64; shape.len()];
        let field = h.run(v, &index, ny * nx).unwrap();
        let t = Instant::now();
        let bytes = vf.read(field.offset, field.len).unwrap();
        let one = t.elapsed();
        // A step whole: each value its type's bytes (GRIB's decoded values are doubles).
        assert_eq!(bytes.len() as u64, field.len);
        assert_eq!(field.len % (ny * nx), 0);
        // A point's series: one value of every step (each in its own chunk).
        let vf = open(&path);
        let t = Instant::now();
        let steps = shape[0];
        for k in 0..steps {
            index[0] = k;
            index[shape.len() - 2] = ny / 2;
            index[shape.len() - 1] = nx / 3;
            let r = h.run(v, &index, 1).unwrap();
            vf.read(r.offset, r.len).unwrap();
        }
        let series = t.elapsed();
        let vf = open(&path);
        let t = Instant::now();
        let mut whole = 0u64;
        for r in h.runs(v) {
            whole += vf.read(r.offset, r.len).unwrap().len() as u64;
        }
        let all = t.elapsed();
        println!(
            "{name}: açılış {:.1} ms, bir adım ({ny}×{nx}) {:.1} ms, noktanın {steps} adımlık serisi {:.1} ms, bütün değişken ({:.0} MB) {:.0} ms",
            opened.as_secs_f64() * 1e3,
            one.as_secs_f64() * 1e3,
            series.as_secs_f64() * 1e3,
            whole as f64 / 1e6,
            all.as_secs_f64() * 1e3
        );
    }
}
