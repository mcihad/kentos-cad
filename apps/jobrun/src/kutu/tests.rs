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
    // The box mode's command line: the paths, the scratch, then the interpreter and its own arguments; the filter
    // unless told otherwise.
    let interpreter_args: [std::ffi::OsString; 3] =
        ["-I".into(), "-m".into(), "kentos.ext_host".into()];
    let wrapped = |filter, abi| {
        wrap(
            Path::new("/srv/kentos-jobrun"),
            &allowed,
            filter,
            abi,
            &job.join(SCRATCH),
            &python,
            &interpreter_args,
        )
    };
    let (program, args) = wrapped(true, None);
    assert_eq!(program, PathBuf::from("/srv/kentos-jobrun"));
    assert_eq!(args[0], ARG);
    assert!(!args.iter().any(|a| a == NO_FILTER || a == ABI_WORD));
    assert_eq!(wrapped(false, None).1[..2], [ARG, NO_FILTER]);
    assert_eq!(wrapped(true, Some(1)).1[..3], [ARG, ABI_WORD, "1"]);
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

/// The box mode refuses words it does not know, a Landlock ABI it does not know and a missing interpreter, saying so
/// with its mark, before it restricts anything.
#[test]
fn the_box_mode_refuses_what_it_does_not_understand() {
    assert_eq!(enter(["--bilinmeyen".into()].into_iter()), 126);
    assert_eq!(
        enter(["--tmp".into(), "/tmp".into(), "--".into()].into_iter()),
        126
    );
    for abi in ["0", "10", "bir"] {
        assert_eq!(
            enter([ABI_WORD.into(), abi.into(), "--".into(), "/bin/true".into()].into_iter()),
            126,
            "{abi}"
        );
    }
}

/// The plan (docs/adr/8057 §63, §65): the whole box only with Landlock and the filter, at any Landlock ABI from 1. Its
/// words are the same at every ABI but for who keeps the signals: what an older kernel leaves open, the filter closes.
/// A box that is not whole runs only under `report`: in Landlock alone where the filter is missing, its words saying
/// what that ABI leaves open; outside the box where Landlock is missing.
#[test]
fn a_box_that_is_not_whole_runs_only_under_report_and_says_what_is_open() {
    let at = |abi| Landlock { abi };
    let no_filter = || Err::<(), _>("seccomp süzgeci yok".to_owned());
    let no_landlock = || Err::<Landlock, _>("Landlock yok".to_owned());
    for policy in [Policy::Require, Policy::Report] {
        for abi in [1, 2, 3, 4, 5, 6, 9] {
            let Ok(Plan::Whole { words, abi: None }) = plan(Ok(at(abi)), Ok(()), policy) else {
                panic!("Landlock {abi} and the filter: the whole box");
            };
            assert!(
                words.starts_with(
                    "dosyalar kapalı: içerik, yol adıyla kısaltma, izin, sahip, zaman, xattr; ağ kapalı (soket \
                     açılamaz): TCP, UDP, yol adlı ve soyut UNIX soketleri"
                ),
                "{words}"
            );
            let signals = if abi >= 6 {
                "öteki süreçler kapalı: sinyaller Landlock'la,"
            } else {
                "öteki süreçler kapalı: sinyaller süzgeçle (Landlock ABI 6 yok),"
            };
            assert!(words.contains(signals), "{abi}: {words}");
        }
    }
    assert_eq!(
        plan(Ok(at(8)), no_filter(), Policy::Require),
        Err("seccomp süzgeci yok".to_owned())
    );
    let alone = |abi| match plan(Ok(at(abi)), no_filter(), Policy::Report) {
        Ok(Plan::LandlockOnly(open)) => open,
        other => panic!("report keeps Landlock {abi}, not {other:?}"),
    };
    assert_eq!(
        alone(6),
        "seccomp süzgeci yok; kapalı: dosyaların içeriği, kısaltma, TCP, sinyaller ve soyut UNIX soketleri; açık: yol \
         adlı UNIX soketleri (ABI 9 yok), UDP ve öteki soket türleri, io_uring, dosyaların izni, sahibi, zamanı ve \
         xattr'ı, öteki süreçlerin kaynak sınırı, önceliği ve zamanlaması"
    );
    let one = alone(1);
    assert!(
        one.contains(
            "kapalı: dosyaların içeriği; açık: yol adıyla kısaltma (ABI 3 yok), TCP (ABI 4 yok), sinyaller ve soyut \
             UNIX soketleri (ABI 6 yok), yol adlı UNIX soketleri (ABI 9 yok)"
        ),
        "{one}"
    );
    assert_eq!(
        plan(no_landlock(), no_filter(), Policy::Require),
        Err("Landlock yok; seccomp süzgeci yok".to_owned())
    );
    assert_eq!(
        plan(no_landlock(), Ok(()), Policy::Report),
        Ok(Plan::Outside("Landlock yok".to_owned()))
    );
    assert_eq!(
        Plan::Outside("Landlock yok".into()).note().as_deref(),
        Some(" (kutu kapatılamadı: Landlock yok)")
    );
    let whole = Plan::Whole {
        words: String::new(),
        abi: None,
    };
    assert_eq!(whole.note(), None);
}
