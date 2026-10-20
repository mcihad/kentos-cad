//! What the processing job tests share (docs/adr/0559): the child program they run tools in and a project to run them
//! on.

#![allow(dead_code)]

use std::path::PathBuf;
use std::process::{Command, Stdio};

/// Two neighbouring parcels (fixtures/server-jobs/v1/parcels.kcad, the same drawing as İşlemler's own cases, written
/// as KCAD v2 the way a server's project snapshot is).
pub const PARCELS: &[u8] = include_bytes!("../../../../fixtures/server-jobs/v1/parcels.kcad");

/// A program of a few lines of shell, for the tests that need a child that does something the tool does not (burns
/// the processor until it is killed); written to a folder of its own, executable.
#[cfg(unix)]
pub fn shell_program(body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let dir = std::env::temp_dir().join(format!("kentos-test-program-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join(format!("program-{}.sh", body.len()));
    std::fs::write(&path, format!("#!/bin/sh\n{body}\n")).expect("the program");
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).expect("executable");
    path
}

/// The child program `kentos-jobrun`: `KENTOS_JOBRUN`, else beside the test's own binaries (`target/<profile>`), built
/// first, once for the test process (`cargo build -p kentos-jobrun`: another package's program is not built by this
/// one's tests, and one built before the sources changed would answer for the old ones).
pub fn child_exe() -> PathBuf {
    if let Some(path) = std::env::var_os("KENTOS_JOBRUN") {
        return path.into();
    }
    let me = std::env::current_exe().expect("the test's own program");
    let profile = me
        .parent()
        .and_then(|deps| deps.parent())
        .expect("target/<profile>/deps/<test>")
        .to_path_buf();
    let exe = profile.join("kentos-jobrun");
    static BUILT: std::sync::Once = std::sync::Once::new();
    BUILT.call_once(|| {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut build = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        build
            .args(["build", "-q", "-p", "kentos-jobrun"])
            .current_dir(root)
            .stdin(Stdio::null());
        if profile.file_name().is_some_and(|n| n == "release") {
            build.arg("--release");
        }
        let status = build.status().expect("cargo builds kentos-jobrun");
        assert!(status.success(), "cargo build -p kentos-jobrun failed");
    });
    assert!(exe.is_file(), "{} yok", exe.display());
    exe
}
