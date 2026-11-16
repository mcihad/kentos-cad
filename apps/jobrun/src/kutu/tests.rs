use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use super::*;

/// The job runner's binary, whose box mode the tests start: built once for the test process (`cargo build -p
/// kentos-jobrun`: one built before the sources changed would answer for the old ones), found in the nearest folder
/// above the test's own program that holds it (`target/<profile>`).
pub(crate) fn job_runner() -> PathBuf {
    static BUILT: std::sync::Once = std::sync::Once::new();
    let me = std::env::current_exe().expect("the test's own program");
    let release = me
        .ancestors()
        .any(|dir| dir.file_name().is_some_and(|n| n == "release"));
    BUILT.call_once(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let mut build = Command::new(std::env::var("CARGO").unwrap_or_else(|_| "cargo".into()));
        build
            .args(["build", "-q", "-p", "kentos-jobrun"])
            .current_dir(root)
            .stdin(Stdio::null());
        if release {
            build.arg("--release");
        }
        let status = build.status().expect("cargo builds kentos-jobrun");
        assert!(status.success(), "cargo build -p kentos-jobrun failed");
    });
    me.ancestors()
        .skip(1)
        .map(|dir| dir.join("kentos-jobrun"))
        .find(|exe| exe.is_file())
        .unwrap_or_else(|| panic!("{} üstündeki klasörlerde kentos-jobrun yok", me.display()))
}

/// The environment a prepared interpreter lives in, on disk: its folder, `pyvenv.cfg` naming a base installation, a
/// `.pth` adding an editable SDK's folder, and another folder a dependency's `.pth` names, which stays shut.
#[test]
fn the_box_allows_the_environment_the_package_and_the_scratch_only() {
    let root = std::env::temp_dir().join(format!("kentos-kutu-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let env = root.join("env");
    let base = root.join("base");
    let sdk = root.join("sdk");
    let elsewhere = root.join("elsewhere");
    let job = root.join("job");
    let site = env.join("lib/python3.14/site-packages");
    for d in [
        &env.join("bin"),
        &site,
        &base.join("bin"),
        &base.join("lib"),
        &sdk.join("kentos"),
        &elsewhere,
        &job.join("extension"),
        &job.join("pyc"),
        &job.join(SCRATCH),
    ] {
        std::fs::create_dir_all(d).unwrap();
    }
    std::fs::write(env.join("bin/python"), b"").unwrap();
    std::fs::write(
        env.join("pyvenv.cfg"),
        format!("home = {}\nversion = 3.14.4\n", base.join("bin").display()),
    )
    .unwrap();
    std::fs::write(sdk.join("kentos/__init__.py"), b"").unwrap();
    std::fs::write(
        site.join("kentos.pth"),
        format!("{}\nimport sys\n/yok/boyle/bir/yer\n", sdk.display()),
    )
    .unwrap();
    std::fs::write(
        site.join("bagimlilik.pth"),
        format!("{}\n", elsewhere.display()),
    )
    .unwrap();
    std::fs::write(job.join("snapshot.kcad"), b"drawing").unwrap();
    let python = env.join("bin/python");
    let allowed = allowed(&job, &python, &job.join("extension"));
    for wanted in [&job.join("extension"), &env, &base.join("lib"), &sdk] {
        assert!(allowed.read.contains(wanted), "{wanted:?} in {allowed:?}");
    }
    assert!(
        !allowed
            .read
            .iter()
            .any(|p| p == &job || p.ends_with("snapshot.kcad")),
        "the drawing is not the rule's: {allowed:?}"
    );
    assert!(!allowed.read.contains(&PathBuf::from("/yok/boyle/bir/yer")));
    assert!(
        !allowed.read.contains(&elsewhere),
        "a dependency's .pth opens nothing: {allowed:?}"
    );
    assert_eq!(
        &allowed.write[..2],
        [job.join("pyc"), job.join(SCRATCH)].as_slice()
    );
    // The box mode's command line: the paths, the scratch, then the interpreter and its own arguments.
    let (program, args) = wrap(
        Path::new("/srv/kentos-jobrun"),
        &allowed,
        &job.join(SCRATCH),
        &python,
        &["-I".into(), "-m".into(), "kentos.ext_host".into()],
    );
    assert_eq!(program, PathBuf::from("/srv/kentos-jobrun"));
    assert_eq!(args[0], ARG);
    let split = args
        .iter()
        .position(|a| a == "--")
        .expect("the interpreter after --");
    assert_eq!(
        &args[split + 1..split + 3],
        [python.clone().into_os_string(), "-I".into()]
    );
    assert!(
        args.windows(2)
            .any(|w| w[0] == "--tmp" && w[1] == job.join(SCRATCH).as_os_str())
    );
    let _ = std::fs::remove_dir_all(&root);
}

/// The box mode refuses words it does not know and a missing interpreter, saying so with its mark.
#[test]
fn the_box_mode_refuses_what_it_does_not_understand() {
    assert_eq!(enter(["--bilinmeyen".into()].into_iter()), 126);
    assert_eq!(
        enter(["--tmp".into(), "/tmp".into(), "--".into()].into_iter()),
        126
    );
}
