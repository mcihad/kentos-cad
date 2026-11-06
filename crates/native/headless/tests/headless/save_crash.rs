//! CAD-ADV-18 (docs/adr/1006): a save killed by a real process death. The test runs itself as a child process that
//! saves large versions of a drawing with sheets over and over through the headless host's save; the parent kills it
//! with SIGKILL at different moments. After each death the file under the name is, byte for byte, the original or one
//! of the versions the child wrote — never a part of one — and it opens with its sheets and objects; a leftover
//! temporary file is never the drawing's name. A save to a folder that refuses writing keeps the drawing's state.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use kentos_headless::{Op, Session};
use serde_json::json;

const CHILD: &str = "KENTOS_SAVE_CRASH_CHILD";
const VERSIONS: u32 = 3;
const POINTS: u32 = 30_000;

fn sheets_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../../fixtures/kcad/v2/sheets.kcad")
}

/// Version `k` of the drawing: the fixture with POINTS points whose attribute `V` is `k`.
fn version(k: u32) -> Session {
    let mut s = Session::open(&sheets_fixture()).expect("the fixture opens");
    let layer = s.summary()["activeLayer"].as_str().expect("a layer").to_owned();
    let objects: Vec<_> = (0..POINTS)
        .map(|i| json!({"geometry": {"kind": "point", "p": {"x": 500000.0 + f64::from(i % 300), "y": 4400000.0 + f64::from(i / 300)}},
            "attrs": {"V": k.to_string()}}))
        .collect();
    let answer = s
        .run("cad.entities.create", None, Op::Execute, json!({"layerId": layer, "objects": objects}))
        .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    s
}

/// The child: writes each version in turn to the target, forever, until it is killed.
#[test]
fn save_crash_child() {
    let Ok(target) = std::env::var(CHILD) else {
        return;
    };
    let target = PathBuf::from(target);
    let mut sessions: Vec<Session> = (1..=VERSIONS).map(version).collect();
    // Each version as this save writes it, beside the target: the parent's reference (the objects' ids are this
    // process's own).
    for (k, s) in sessions.iter_mut().enumerate() {
        let path = target.with_file_name(format!("surum-{}.kcad", k + 1));
        s.save(Some(&path)).expect("saves");
    }
    std::fs::write(target.with_extension("ready"), b"1").expect("ready");
    loop {
        for s in &mut sessions {
            s.save(Some(&target)).expect("saves");
        }
    }
}

fn leftovers(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .expect("the folder")
        .filter_map(|e| e.ok()?.file_name().into_string().ok())
        .filter(|n| n.ends_with(".yaziliyor"))
        .collect()
}

#[test]
fn a_killed_save_leaves_a_whole_file_under_the_name() {
    if std::env::var(CHILD).is_ok() {
        return;
    }
    let dir = std::env::temp_dir().join(format!("kentos-save-crash-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let target = dir.join("pafta.kcad");
    let original = std::fs::read(sheets_fixture()).expect("the fixture");
    std::fs::write(&target, &original).expect("the original");
    let original_known = original.clone();
    let exe = std::env::current_exe().expect("the test binary");
    let mut deaths = 0;
    let mut versions_seen = std::collections::BTreeSet::new();
    for (round, wait_ms) in [3u64, 130, 300, 470, 660, 860, 1070, 1290, 1530, 1800].into_iter().enumerate() {
        let ready = target.with_extension("ready");
        let _ = std::fs::remove_file(&ready);
        let mut child = std::process::Command::new(&exe)
            .args(["save_crash::save_crash_child", "--exact", "--nocapture", "--test-threads=1"])
            .env(CHILD, &target)
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("the child starts");
        let start = Instant::now();
        while !ready.exists() {
            assert!(start.elapsed() < Duration::from_secs(120), "the child never got ready");
            std::thread::sleep(Duration::from_millis(5));
        }
        std::thread::sleep(Duration::from_millis(wait_ms));
        child.kill().expect("SIGKILL");
        let _ = child.wait();
        deaths += 1;
        // The child's own versions, written whole before it started on the target.
        let mut known = vec![original_known.clone()];
        for k in 1..=VERSIONS {
            known.push(std::fs::read(dir.join(format!("surum-{k}.kcad"))).expect("the child's version"));
        }
        let bytes = std::fs::read(&target).expect("the name still has a file");
        let which = known.iter().position(|k| *k == bytes);
        assert!(which.is_some(), "round {round}: the file under the name is not a whole version ({} bytes)", bytes.len());
        versions_seen.insert(which.unwrap());
        let opened = Session::open(&target).expect("the file opens");
        let snapshot = opened.document().to_snapshot_v2();
        assert!(snapshot.sheets.is_some(), "round {round}: the sheets are kept");
        for name in leftovers(&dir) {
            assert_ne!(dir.join(&name), target, "a temporary file is never the drawing");
        }
    }
    assert_eq!(deaths, 10);
    assert!(versions_seen.len() >= 2, "the kills landed on more than one version: {versions_seen:?}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_save_the_disk_refuses_keeps_the_drawing_unsaved_and_its_undo() {
    let dir = std::env::temp_dir().join(format!("kentos-save-refused-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let target = dir.join("pafta.kcad");
    let original = std::fs::read(sheets_fixture()).expect("the fixture");
    std::fs::write(&target, &original).expect("the original");
    let mut s = Session::open(&target).expect("opens");
    let layer = s.summary()["activeLayer"].as_str().unwrap().to_owned();
    let answer = s
        .run("cad.point.create", None, Op::Execute, json!({"layerId": layer, "p": {"x": 500001.0, "y": 4400001.0}}))
        .expect("runs");
    assert_eq!(answer["status"], "completed");
    let before = s.summary();
    let mut perms = std::fs::metadata(&dir).unwrap().permissions();
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o555);
    std::fs::set_permissions(&dir, perms.clone()).unwrap();
    let refused = s.save(None);
    std::os::unix::fs::PermissionsExt::set_mode(&mut perms, 0o755);
    std::fs::set_permissions(&dir, perms).unwrap();
    let error = refused.expect_err("the folder refuses writing");
    assert_eq!(error.code, "file_unwritable");
    assert_eq!(s.summary(), before, "the drawing is as it was: unsaved, the same revision");
    assert_eq!(std::fs::read(&target).unwrap(), original, "the previous file is untouched");
    assert!(leftovers(&dir).is_empty(), "no temporary file is left");
    assert!(s.undo().is_some(), "the undo chain is whole");
    let _ = std::fs::remove_dir_all(&dir);
}

/// A real full disk (docs/adr/1006 ek): a 1 MB volume mounted from a disk image (macOS `hdiutil`, no privileges)
/// holds the drawing; a save of a version too large for the space left fails with the disk's own ENOSPC, the previous
/// file is byte for byte as it was, no temporary file is left on the volume, the drawing stays unsaved and its undo
/// chain whole.
#[cfg(target_os = "macos")]
#[test]
fn a_save_onto_a_full_disk_keeps_the_previous_file_and_the_drawing() {
    struct Mounted(PathBuf);
    impl Drop for Mounted {
        fn drop(&mut self) {
            let _ = std::process::Command::new("hdiutil").args(["detach", "-force"]).arg(&self.0).output();
        }
    }
    let dir = std::env::temp_dir().join(format!("kentos-save-full-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("a folder");
    let image = dir.join("dolu.dmg");
    let made = std::process::Command::new("hdiutil")
        .args(["create", "-size", "1m", "-fs", "HFS+", "-volname", "KDOLU", "-type", "UDIF"])
        .arg(&image)
        .output()
        .expect("hdiutil runs");
    assert!(made.status.success(), "{}", String::from_utf8_lossy(&made.stderr));
    let mount = dir.join("mnt");
    let attached = std::process::Command::new("hdiutil")
        .args(["attach", "-nobrowse", "-mountpoint"])
        .arg(&mount)
        .arg(&image)
        .output()
        .expect("hdiutil runs");
    assert!(attached.status.success(), "{}", String::from_utf8_lossy(&attached.stderr));
    let _mounted = Mounted(mount.clone());
    let target = mount.join("pafta.kcad");
    let original = std::fs::read(sheets_fixture()).expect("the fixture");
    std::fs::write(&target, &original).expect("the original fits");
    let mut s = Session::open(&target).expect("opens");
    let layer = s.summary()["activeLayer"].as_str().expect("a layer").to_owned();
    // Far more than the volume's space left: 30 000 points with their attributes.
    let objects: Vec<_> = (0..POINTS)
        .map(|i| json!({"geometry": {"kind": "point", "p": {"x": 500000.0 + f64::from(i % 300), "y": 4400000.0 + f64::from(i / 300)}},
            "attrs": {"V": "dolu"}}))
        .collect();
    let answer = s
        .run("cad.entities.create", None, Op::Execute, json!({"layerId": layer, "objects": objects}))
        .expect("runs");
    assert_eq!(answer["status"], "completed");
    let before = s.summary();
    let error = s.save(None).expect_err("the disk is full");
    assert_eq!(error.code, "file_unwritable", "{}", error.message);
    assert_eq!(std::fs::read(&target).expect("the file"), original, "the previous file is untouched");
    assert!(leftovers(&mount).is_empty(), "no temporary file is left: {:?}", leftovers(&mount));
    assert_eq!(s.summary(), before, "the drawing is as it was: unsaved, the same revision");
    assert!(s.undo().is_some(), "the undo chain is whole");
    drop(_mounted);
    let _ = std::fs::remove_dir_all(&dir);
}
