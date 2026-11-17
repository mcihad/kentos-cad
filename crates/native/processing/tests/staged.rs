//! A result's file written beside its place and put there when whole (docs/adr/8057 §120, plan/eklenti.md
//! ECO-QA-10), with the faults injected on real files: a sink dropped unfinished, a place that cannot take the file (a
//! folder of that name there), a folder that takes no file. Each leaves the place as it was and no piece beside it.
//! Before, a file that could not be put in place left its `.yaziliyor` piece beside it, on both hosts.

use std::path::{Path, PathBuf};

use kentos_commands::RasterSample;
use kentos_formats::raster::write::{Geo, Image, Writer};
use kentos_formats::raster::{Samples, TILE};
use kentos_processing::cloud_files::{ReadAt, copc_file};
use kentos_processing::files::Sink;
use kentos_processing::staged;

/// A folder of the test's own, empty.
fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-staged-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    dir
}

/// What the folder holds, by name.
fn held(dir: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(dir)
        .expect("lists")
        .filter_map(|e| e.ok())
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

/// A place a file cannot be put at: a folder of that name, holding a file of the user's.
fn taken_place(dir: &Path, name: &str) -> PathBuf {
    let place = dir.join(name);
    std::fs::create_dir_all(&place).expect("a folder at the place");
    std::fs::write(place.join("kullanicinin.txt"), "kullanıcının dosyası").expect("written");
    place
}

fn left_as_it_was(place: &Path) {
    assert!(place.is_dir());
    assert_eq!(
        std::fs::read_to_string(place.join("kullanicinin.txt")).expect("reads"),
        "kullanıcının dosyası"
    );
}

#[test]
fn a_whole_file_is_put_in_place_and_no_piece_stays() {
    let dir = folder("whole");
    let path = dir.join("sonuc.bin");
    let mut sink: Box<dyn Sink> = Box::new(staged::create(&path).expect("opens"));
    sink.write(b"abcdef").expect("writes");
    sink.patch(1, b"XY").expect("patches");
    sink.write(b"gh").expect("writes on after the patch");
    assert_eq!(
        held(&dir),
        ["sonuc.bin.yaziliyor"],
        "written beside its place"
    );
    sink.finish().expect("put in place");
    assert_eq!(std::fs::read(&path).expect("reads"), b"aXYdefgh");
    assert_eq!(held(&dir), ["sonuc.bin"]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_sink_dropped_unfinished_leaves_nothing() {
    // A run that breaks or is stopped while writing drops its sink.
    let dir = folder("dropped");
    let path = dir.join("sonuc.bin");
    let mut sink: Box<dyn Sink> = Box::new(staged::create(&path).expect("opens"));
    sink.write(&[7; 4096]).expect("writes");
    drop(sink);
    assert!(held(&dir).is_empty(), "{:?}", held(&dir));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_file_that_cannot_be_put_in_place_leaves_no_piece_and_the_place_as_it_was() {
    let dir = folder("taken");
    let place = taken_place(&dir, "sonuc.tif");
    let mut sink: Box<dyn Sink> = Box::new(staged::create(&place).expect("opens"));
    sink.write(&[1; 1024]).expect("writes");
    let why = sink.finish().expect_err("a folder is there");
    assert!(
        why.contains("sonuc.tif") && why.contains("yazılamadı"),
        "{why}"
    );
    assert_eq!(held(&dir), ["sonuc.tif"], "no piece beside it");
    left_as_it_was(&place);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A tiled GeoTIFF of one level, as a raster tool writes it.
fn tiled() -> Vec<u8> {
    let (w, h) = (300, 200);
    let image = Image {
        width: w,
        height: h,
        bands: 1,
        sample: RasterSample::U8,
        alpha: false,
        geo: Some(Geo {
            affine: [487000.0, 1.0, 0.0, 4420000.0, 0.0, -1.0],
            epsg: Some(5256),
            geographic: false,
        }),
        nodata: Some(0.0),
    };
    let (mut writer, head) = Writer::new(vec![image], 1).expect("a writer");
    let mut file = head;
    for ty in 0..h.div_ceil(TILE) {
        for tx in 0..w.div_ceil(TILE) {
            let s = Samples::filled(
                RasterSample::U8,
                (TILE * TILE) as usize,
                f64::from(tx + ty + 1),
            );
            file.extend(writer.tile(0, tx, ty, &s).expect("a tile"));
        }
    }
    let (tail, header) = writer.finish().expect("finished");
    file.extend(tail);
    file[..header.len()].copy_from_slice(&header);
    file
}

#[test]
fn a_cog_is_laid_out_in_place_or_leaves_neither_piece() {
    let dir = folder("cog");
    let bytes = tiled();
    let path = dir.join("yuzey.tif");
    let mut sink: Box<dyn Sink> = Box::new(staged::create(&path).expect("opens"));
    sink.write(&bytes).expect("writes");
    sink.finish_cog().expect("laid out");
    let written = std::fs::read(&path).expect("reads");
    assert!(kentos_formats::raster::cog::declared(
        &written[..written.len().min(1024)]
    ));
    assert_eq!(held(&dir), ["yuzey.tif"]);
    // Its place taken: neither the written piece nor the laid out one stays.
    let place = taken_place(&dir, "egim.tif");
    let mut sink: Box<dyn Sink> = Box::new(staged::create(&place).expect("opens"));
    sink.write(&bytes).expect("writes");
    let why = sink.finish_cog().expect_err("a folder is there");
    assert!(why.contains("egim.tif"), "{why}");
    assert_eq!(held(&dir), ["egim.tif", "yuzey.tif"]);
    left_as_it_was(&place);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A file's bytes read as a host reads a file.
struct Held(Vec<u8>);

impl ReadAt for Held {
    fn size(&self) -> u64 {
        self.0.len() as u64
    }

    fn read(&self, offset: u64, len: u64) -> Result<Vec<u8>, String> {
        let (at, end) = (offset as usize, (offset + len) as usize);
        self.0
            .get(at..end)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| format!("{at}..{end}: past the file's end"))
    }
}

#[test]
fn a_copc_that_cannot_be_put_in_place_leaves_no_piece_nor_bins() {
    let las = std::fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../../fixtures/pointcloud/v1/files/tile-a.las"),
    )
    .expect("the cloud");
    let dir = folder("copc");
    let path = dir.join("bulut.copc.laz");
    copc_file(Box::new(Held(las.clone())), &path, &mut |_| false).expect("an index");
    assert_eq!(held(&dir), ["bulut.copc.laz"]);
    let place = taken_place(&dir, "baska.copc.laz");
    let why =
        copc_file(Box::new(Held(las)), &place, &mut |_| false).expect_err("a folder is there");
    assert!(why.contains("baska.copc.laz"), "{why}");
    assert_eq!(
        held(&dir),
        ["baska.copc.laz", "bulut.copc.laz"],
        "no piece, no bins"
    );
    left_as_it_was(&place);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Whether this process writes where the system says it may not (a superuser's): a folder's rights test nothing then.
#[cfg(unix)]
fn rights_hold(dir: &Path) -> bool {
    std::fs::write(dir.join("deneme"), b"").is_err()
}

#[cfg(unix)]
#[test]
fn a_folder_that_takes_no_file_is_said_and_nothing_is_written() {
    use std::os::unix::fs::PermissionsExt;
    let dir = folder("locked");
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).expect("locked");
    if rights_hold(&dir) {
        let why = staged::create(&dir.join("sonuc.tif"))
            .err()
            .expect("the folder takes no file");
        assert!(
            why.contains("Klasörün yazılabilir olduğunu denetleyin"),
            "{why}"
        );
        assert!(held(&dir).is_empty());
    }
    std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).expect("opened");
    let _ = std::fs::remove_dir_all(&dir);
}
