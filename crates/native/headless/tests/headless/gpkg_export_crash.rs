//! The municipal report's 6th item, a part of what is left (docs/adr/9178): a GeoPackage delivery written over a file
//! that is there, killed by a real process death. The test runs itself as a child process that writes versions of a
//! drawing's point layer over and over as `file.export.gpkg` writes them; the parent kills it with SIGKILL at different
//! moments. After each death the file under the name is whole: a GeoPackage that passes SQLite's integrity check and
//! holds the original's points or one version's, all of them, never a part; a leftover temporary file is never the
//! delivery's name.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use kentos_headless::{Op, Session};
use serde_json::json;

const CHILD: &str = "KENTOS_GPKG_EXPORT_CRASH_CHILD";
const VERSIONS: u32 = 3;

fn fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/sheets.kcad")
}

/// How many points version `k` has: the original (0) a handful, each version thousands more than the one before.
fn points(k: u32) -> u32 {
    if k == 0 { 7 } else { 20_000 + 2_000 * k }
}

/// The objects the fixture's active layer holds already: every version's table has them too.
fn already() -> u32 {
    let s = Session::open(&fixture()).expect("the fixture opens");
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    s.document().by_layer(&layer).count() as u32
}

/// Version `k`: the fixture with [`points`]`(k)` points more on its active layer; the session and that layer.
fn version(k: u32) -> (Session, String) {
    let mut s = Session::open(&fixture()).expect("the fixture opens");
    let layer = s.summary()["activeLayer"]
        .as_str()
        .expect("a layer")
        .to_owned();
    let objects: Vec<_> = (0..points(k))
        .map(|i| {
            json!({"geometry": {"kind": "point", "p": {"x": 500000.0 + f64::from(i % 300), "y": 4400000.0 + f64::from(i / 300)}}})
        })
        .collect();
    let answer = s
        .run(
            "cad.entities.create",
            None,
            Op::Execute,
            json!({"layerId": layer, "objects": objects}),
        )
        .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    (s, layer)
}

/// The point layer of `s` written to `target` over what is there, as `file.export.gpkg` writes it.
fn deliver(s: &Session, layer: &str, target: &Path) {
    let input: kentos_commands::ExportGpkg = serde_json::from_value(
        json!({"path": target.display().to_string(), "layers": [layer], "overwrite": true}),
    )
    .expect("an input");
    kentos_headless::file_commands::export_gpkg(s.document(), &input).expect("written");
}

/// The child: writes each version in turn to the target, forever, until it is killed.
#[test]
fn gpkg_export_crash_child() {
    let Ok(target) = std::env::var(CHILD) else {
        return;
    };
    let target = PathBuf::from(target);
    let versions: Vec<(Session, String)> = (1..=VERSIONS).map(version).collect();
    std::fs::write(target.with_extension("ready"), b"1").expect("ready");
    loop {
        for (s, layer) in &versions {
            deliver(s, layer, &target);
        }
    }
}

/// What is under the name: SQLite's integrity check and the rows with a geometry of its one feature table; why not
/// when it is not a GeoPackage of one.
fn read(path: &Path) -> Result<(String, i64), String> {
    let gp = kentos_geopackage::GeoPackage::open(path, false).map_err(|e| e.0)?;
    let c = gp.connection();
    let check: String = c
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(|e| e.to_string())?;
    let table: String = c
        .query_row(
            "SELECT table_name FROM gpkg_contents WHERE data_type = 'features'",
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let n: i64 = c
        .query_row(
            &format!("SELECT count(*) FROM \"{table}\" WHERE geom IS NOT NULL"),
            [],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    Ok((check, n))
}

/// Which whole file is under the name: 0 the original, `k` version `k`; none when it is not a whole one.
fn whole(path: &Path, base: u32) -> Option<u32> {
    let (check, n) = read(path).ok()?;
    if check != "ok" {
        return None;
    }
    (0..=VERSIONS).find(|k| i64::from(base + points(*k)) == n)
}

fn leftovers(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .expect("the folder")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".yaziliyor"))
        .collect()
}

#[test]
fn a_killed_delivery_leaves_a_whole_geopackage_under_the_name() {
    if std::env::var(CHILD).is_ok() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("kentos-gpkg-crash-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let target = dir.join("teslim.gpkg");
    let base = already();
    let (original, layer) = version(0);
    deliver(&original, &layer, &target);
    assert_eq!(
        whole(&target, base),
        Some(0),
        "the original delivery: {:?}",
        read(&target)
    );
    let exe = std::env::current_exe().expect("the test binary");
    let mut seen = std::collections::BTreeSet::new();
    for (round, wait_ms) in [5u64, 120, 260, 410, 590, 780, 990, 1210, 1460, 1720]
        .into_iter()
        .enumerate()
    {
        let ready = target.with_extension("ready");
        let _ = std::fs::remove_file(&ready);
        let mut child = std::process::Command::new(&exe)
            .args([
                "gpkg_export_crash::gpkg_export_crash_child",
                "--exact",
                "--nocapture",
                "--test-threads=1",
            ])
            .env(CHILD, &target)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the child starts");
        let start = Instant::now();
        while !ready.exists() {
            assert!(
                start.elapsed() < Duration::from_secs(180),
                "the child never got ready"
            );
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(wait_ms));
        child.kill().expect("SIGKILL");
        let _ = child.wait();
        let which = whole(&target, base);
        assert!(
            which.is_some(),
            "round {round}: the file under the name is not a whole delivery: {:?}",
            read(&target)
        );
        seen.insert(which);
        for name in leftovers(&dir) {
            assert_ne!(
                dir.join(&name),
                target,
                "a temporary file is never the delivery"
            );
        }
    }
    assert!(
        seen.len() >= 2,
        "the kills landed on more than one delivery: {seen:?}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
