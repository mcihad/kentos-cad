//! The virtual mosaic's reader (docs/adr/0517 §2) against GDAL: GDAL's own VRTs of fixtures/raster/v1/mosaic
//! (gdal-*.vrt, made by gdalbuildvrt) read through `vrt::open_mosaic`, their level-0 pixels value for value GDAL's
//! (the raw files beside them, written by `scripts/fixtures/raster_mosaic_cases.py --refs`); the VRTs KentOS's
//! command wrote there (kentos-*.vrt) read the same; a coarse level made of the files' own levels.

use std::path::{Path, PathBuf};

use kentos_contracts::RasterSample;
use kentos_formats::raster::source::{Put, Reader, open_bytes};
use kentos_formats::raster::vrt;

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/raster/v1/mosaic")
}

/// A mosaic opened with its files read whole, and the files' bytes.
fn open(name: &str) -> (Reader, Vec<Vec<u8>>) {
    let text = std::fs::read_to_string(folder().join(name)).unwrap();
    let mut bytes = Vec::new();
    let (reader, paths) = vrt::open_mosaic(&text, &folder(), 64 << 20, |p: &Path, budget| {
        let b = std::fs::read(p).map_err(|e| e.to_string())?;
        open_bytes(&b, None, budget).map_err(|e| e.0)
    })
    .unwrap();
    for p in paths {
        bytes.push(std::fs::read(p).unwrap());
    }
    (reader, bytes)
}

/// The region's pixels once every block it wants is handed over.
fn region(r: &mut Reader, files: &[Vec<u8>], level: usize, w: u32, h: u32) -> kentos_formats::raster::source::Region {
    let needs = r.needs(level, 0, 0, w, h);
    for n in &needs {
        assert!(n.member > 0, "a mosaic's block is one of its files'");
        let b = &files[n.member as usize - 1];
        let put = r.put_block(n, &b[n.offset as usize..(n.offset + n.len) as usize]).unwrap();
        assert_eq!(put, Put::Done);
    }
    r.region(level, 0, 0, w, h).expect("every block handed over")
}

fn raw(name: &str, sample: RasterSample) -> Vec<f64> {
    let b = std::fs::read(folder().join(name)).unwrap();
    match sample {
        RasterSample::U8 => b.iter().map(|&v| f64::from(v)).collect(),
        RasterSample::F32 => b.chunks_exact(4).map(|c| f64::from(f32::from_le_bytes([c[0], c[1], c[2], c[3]]))).collect(),
        other => panic!("{other:?}"),
    }
}

fn same_pixels(vrt_name: &str, raw_name: &str) {
    let (mut r, files) = open(vrt_name);
    let (w, h) = (r.info.width, r.info.height);
    let got = region(&mut r, &files, 0, w, h);
    let want = raw(raw_name, r.info.sample);
    assert_eq!(got.samples.len(), want.len(), "{vrt_name}");
    for (k, &v) in want.iter().enumerate() {
        let g = got.samples.get(k);
        assert!(
            g == v || (g.is_nan() && v.is_nan()),
            "{vrt_name}: pixel {} band {}: KentOS {g}, GDAL {v}",
            k / got.bands as usize,
            k % got.bands as usize
        );
    }
}

#[test]
fn gdals_mosaics_read_as_gdal_reads_them() {
    for name in ["dem-verilen", "dem-kaba", "orto"] {
        same_pixels(&format!("gdal-{name}.vrt"), &format!("gdal-{name}.raw"));
    }
}

#[test]
fn kentos_mosaics_read_as_gdal_reads_its_own() {
    for name in ["dem-verilen", "dem-kaba", "orto"] {
        let k = format!("kentos-{name}.vrt");
        assert!(folder().join(&k).exists(), "{k} yok: KENTOS_WRITE_VRT=1 cargo test -p kentos-native-application --test all mosaic_files");
        same_pixels(&k, &format!("gdal-{name}.raw"));
    }
}

#[test]
fn a_mosaic_says_what_it_is() {
    let (r, _) = open("gdal-dem-verilen.vrt");
    assert_eq!(r.mosaic_files(), 3);
    assert_eq!(r.info.placed_by, "vrt");
    assert_eq!(r.info.epsg, Some(5256));
    assert_eq!(r.info.nodata, Some(-9999.0));
    assert!(!r.info.needs_pyramid);
    let (o, _) = open("gdal-orto.vrt");
    assert_eq!((o.info.bands, o.info.color.as_str()), (3, "rgb"));
}

#[test]
fn a_coarse_level_is_made_of_the_files_levels() {
    let (mut r, files) = open("gdal-orto.vrt");
    let lv = r.levels[1].clone();
    let got = region(&mut r, &files, 1, lv.width, lv.height);
    // The 2023 file lies on top: a pixel well inside it is one of its colours, not 0 (nothing).
    let n = got.samples.len() / 3;
    let covered = (0..n).filter(|&k| (0..3).any(|b| got.samples.get(k * 3 + b) != 0.0)).count();
    assert!(covered > n / 2, "{covered} of {n}");
}
