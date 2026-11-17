use super::*;

const LINUX: Target = Target {
    os: Os::Linux,
    machine: Machine::X86_64,
    python: (3, 12),
};
const LINUX_ARM: Target = Target {
    os: Os::Linux,
    machine: Machine::Aarch64,
    python: (3, 12),
};
const WINDOWS: Target = Target {
    os: Os::Windows,
    machine: Machine::X86_64,
    python: (3, 12),
};
const WINDOWS_ARM: Target = Target {
    os: Os::Windows,
    machine: Machine::Aarch64,
    python: (3, 12),
};
const MAC: Target = Target {
    os: Os::Macos,
    machine: Machine::Aarch64,
    python: (3, 12),
};
const MAC_INTEL: Target = Target {
    os: Os::Macos,
    machine: Machine::X86_64,
    python: (3, 12),
};

/// A wheel fits by its Python, ABI and platform tags as `packaging` lists them for CPython 3.12.
#[test]
fn a_wheel_fits_by_its_tags() {
    let manylinux = "numpy-2.5.4-cp312-cp312-manylinux_2_27_x86_64.manylinux_2_28_x86_64.whl";
    assert!(fits(manylinux, &LINUX));
    assert!(!fits(manylinux, &LINUX_ARM));
    assert!(!fits(manylinux, &WINDOWS));
    let newer = Target {
        python: (3, 13),
        ..LINUX
    };
    assert!(!fits(manylinux, &newer), "another CPython's ABI");
    assert!(
        !fits("numpy-2.5.4-cp312-cp312-musllinux_1_2_x86_64.whl", &LINUX),
        "musl"
    );
    assert!(fits("x-1.0-cp312-cp312-linux_x86_64.whl", &LINUX));
    assert!(fits(
        "x-1.0-cp312-cp312-manylinux2014_aarch64.whl",
        &LINUX_ARM
    ));
    assert!(fits("x-1.0-cp312-cp312-win_amd64.whl", &WINDOWS));
    assert!(!fits("x-1.0-cp312-cp312-win_amd64.whl", &WINDOWS_ARM));
    assert!(fits("x-1.0-cp312-cp312-win_arm64.whl", &WINDOWS_ARM));
    assert!(!fits("x-1.0-cp312-cp312-win32.whl", &WINDOWS), "32-bit");
    assert!(fits("x-1.0-cp312-cp312-macosx_11_0_arm64.whl", &MAC));
    assert!(!fits("x-1.0-cp312-cp312-macosx_11_0_arm64.whl", &MAC_INTEL));
    assert!(fits("x-1.0-cp312-cp312-macosx_10_9_universal2.whl", &MAC));
    assert!(fits(
        "x-1.0-cp312-cp312-macosx_10_9_universal2.whl",
        &MAC_INTEL
    ));
    assert!(fits(
        "x-1.0-cp312-cp312-macosx_10_13_x86_64.whl",
        &MAC_INTEL
    ));
    // Pure Python, the stable ABI, a build tag, compressed sets.
    for any in [
        "saf-1.0-py3-none-any.whl",
        "saf-1.0-py2.py3-none-any.whl",
        "saf-1.0-py311-none-any.whl",
        "saf-1.0-cp312-none-any.whl",
        "saf-1.0-1-py3-none-any.whl",
    ] {
        for t in [LINUX, LINUX_ARM, WINDOWS, WINDOWS_ARM, MAC, MAC_INTEL] {
            assert!(fits(any, &t), "{any} {t:?}");
        }
    }
    assert!(fits("k-1.0-cp39-abi3-win_amd64.whl", &WINDOWS));
    assert!(
        !fits("k-1.0-cp313-abi3-win_amd64.whl", &WINDOWS),
        "3.13 and later"
    );
    for not in [
        "k-1.0-cp312-cp312t-win_amd64.whl",
        "k-1.0-pp310-pypy310_pp73-win_amd64.whl",
        "k-1.0-py2-none-any.whl",
        "k-1.0-py313-none-any.whl",
        "k-1.0-cp311-none-any.whl",
        "k-1.0.tar.gz",
        "bozuk.whl",
    ] {
        assert!(!fits(not, &WINDOWS), "{not}");
    }
}

/// Markers as PEP 508 reads them, for each computer; one this reader cannot read holds.
#[test]
fn markers_are_read_for_each_computer() {
    let (linux, windows, mac) = (Env::of(&LINUX), Env::of(&WINDOWS), Env::of(&MAC));
    assert!(windows.holds("sys_platform == 'win32'"));
    assert!(!linux.holds("sys_platform == \"win32\""));
    assert!(windows.holds("platform_machine == 'AMD64' and sys_platform == 'win32'"));
    assert!(
        !Env::of(&WINDOWS_ARM).holds("platform_machine == 'AMD64' and sys_platform == 'win32'")
    );
    assert!(mac.holds("platform_system == 'Darwin' and platform_machine == 'arm64'"));
    assert!(linux.holds("os_name == 'posix'") && windows.holds("os_name == 'nt'"));
    assert!(linux.holds("python_full_version >= '3.11'"));
    assert!(!linux.holds("python_version < '3.11'"));
    assert!(
        linux.holds("'3.11' <= python_version"),
        "the variable on the right"
    );
    assert!(linux.holds("python_version == '3.12.*'"));
    assert!(linux.holds("sys_platform != 'darwin' or platform_machine == 'arm64'"));
    assert!(!linux.holds(
        "(sys_platform == 'win32' or sys_platform == 'darwin') and python_version >= '3.10'"
    ));
    assert!(mac.holds(
        "(sys_platform == 'win32' or sys_platform == 'darwin') and python_version >= '3.10'"
    ));
    assert!(!linux.holds("implementation_name == 'pypy'"));
    assert!(linux.holds("'linux' in sys_platform"));
    assert!(linux.holds("sys_platform not in 'win32 cygwin'"));
    assert!(!windows.holds("sys_platform not in 'win32 cygwin'"));
    // Not known: holds.
    for unknown in [
        "platform_release >= '5'",
        "extra == 'grafik'",
        "!!!",
        "sys_platform ==",
        "(sys_platform == 'win32'",
    ] {
        assert!(linux.holds(unknown), "{unknown}");
    }
}

/// A lock with the cases: a package with wheels everywhere it is asked, one only for Windows x86_64 and no source
/// (asked only on Windows), one with only a source, an optional feature's, a repository's, an extra a dependency asks
/// for, and a fork by Python.
const LOCK: &str = r#"version = 1
revision = 3
requires-python = ">=3.10"

[[package]]
name = "ornek"
version = "1.0.0"
source = { virtual = "." }
dependencies = [
    { name = "numpy" },
    { name = "pywin32", marker = "sys_platform == 'win32'" },
    { name = "tasli" },
    { name = "saf" },
    { name = "depodan" },
    { name = "istekler", extra = ["socks"] },
    { name = "catal", version = "2.0", source = { registry = "https://pypi.org/simple" }, marker = "python_full_version >= '3.12'" },
    { name = "catal", version = "1.0", source = { registry = "https://pypi.org/simple" }, marker = "python_full_version < '3.12'" },
]

[package.optional-dependencies]
grafik = [
    { name = "cizim" },
]

[[package]]
name = "numpy"
version = "2.5.4"
source = { registry = "https://pypi.org/simple" }
sdist = { url = "https://files.pythonhosted.org/packages/numpy-2.5.4.tar.gz", hash = "sha256:00" }
wheels = [
    { url = "https://files.pythonhosted.org/packages/numpy-2.5.4-cp312-cp312-manylinux_2_27_x86_64.manylinux_2_28_x86_64.whl", hash = "sha256:01" },
    { url = "https://files.pythonhosted.org/packages/numpy-2.5.4-cp312-cp312-win_amd64.whl", hash = "sha256:02" },
    { url = "https://files.pythonhosted.org/packages/numpy-2.5.4-cp312-cp312-macosx_11_0_arm64.whl", hash = "sha256:03" },
]

[[package]]
name = "pywin32"
version = "311"
source = { registry = "https://pypi.org/simple" }
wheels = [
    { url = "https://files.pythonhosted.org/packages/pywin32-311-cp312-cp312-win_amd64.whl", hash = "sha256:04" },
]

[[package]]
name = "tasli"
version = "0.3"
source = { registry = "https://pypi.org/simple" }
sdist = { url = "https://files.pythonhosted.org/packages/tasli-0.3.tar.gz", hash = "sha256:05" }

[[package]]
name = "saf"
version = "1.1"
source = { registry = "https://pypi.org/simple" }
wheels = [
    { url = "https://files.pythonhosted.org/packages/saf-1.1-py3-none-any.whl", hash = "sha256:06" },
]

[[package]]
name = "depodan"
version = "0.1"
source = { git = "https://depo.ornek.bel.tr/depodan?rev=v0.1#abc" }

[[package]]
name = "istekler"
version = "2.0"
source = { registry = "https://pypi.org/simple" }
wheels = [
    { url = "https://files.pythonhosted.org/packages/istekler-2.0-py3-none-any.whl", hash = "sha256:07" },
]

[package.optional-dependencies]
socks = [
    { name = "corap" },
]

[[package]]
name = "corap"
version = "1.7"
source = { registry = "https://pypi.org/simple" }
sdist = { url = "https://files.pythonhosted.org/packages/corap-1.7.tar.gz", hash = "sha256:08" }

[[package]]
name = "catal"
version = "1.0"
source = { registry = "https://pypi.org/simple" }
resolution-markers = ["python_full_version < '3.12'"]
sdist = { url = "https://files.pythonhosted.org/packages/catal-1.0.tar.gz", hash = "sha256:09" }

[[package]]
name = "catal"
version = "2.0"
source = { registry = "https://pypi.org/simple" }
resolution-markers = ["python_full_version >= '3.12'"]
wheels = [
    { url = "https://files.pythonhosted.org/packages/catal-2.0-py3-none-any.whl", hash = "sha256:10" },
]

[[package]]
name = "cizim"
version = "4.0"
source = { registry = "https://pypi.org/simple" }
sdist = { url = "https://files.pythonhosted.org/packages/cizim-4.0.tar.gz", hash = "sha256:11" }
wheels = [
    { url = "https://files.pythonhosted.org/packages/cizim-4.0-cp312-cp312-manylinux_2_17_x86_64.whl", hash = "sha256:12" },
]
"#;

fn needs(found: &[Unwheeled], name: &str) -> Vec<(Target, Need)> {
    found
        .iter()
        .find(|u| u.name == name)
        .map(|u| u.needs.clone())
        .unwrap_or_default()
}

/// The packages each computer installs, followed through markers, extras and forks; what each needs where no wheel
/// fits.
#[test]
fn the_packages_without_a_wheel_are_found_where_they_install() {
    let all = [LINUX, LINUX_ARM, WINDOWS, WINDOWS_ARM, MAC, MAC_INTEL];
    let found = unwheeled(LOCK, "ornek", &all);
    let names: Vec<&str> = found.iter().map(|u| u.name.as_str()).collect();
    assert_eq!(
        names,
        ["cizim", "corap", "depodan", "numpy", "pywin32", "tasli"],
        "the wheeled ones, and the project, not listed"
    );
    // Wheels for three computers, a source for the rest.
    assert_eq!(
        needs(&found, "numpy"),
        [
            (LINUX_ARM, Need::Source),
            (WINDOWS_ARM, Need::Source),
            (MAC_INTEL, Need::Source)
        ]
    );
    // Asked only on Windows; no source: Windows on ARM cannot install it.
    assert_eq!(needs(&found, "pywin32"), [(WINDOWS_ARM, Need::Nothing)]);
    let everywhere = |need| all.iter().map(|t| (*t, need)).collect::<Vec<_>>();
    assert_eq!(needs(&found, "tasli"), everywhere(Need::Source));
    assert_eq!(
        needs(&found, "depodan"),
        everywhere(Need::Source),
        "a repository's"
    );
    assert_eq!(
        needs(&found, "corap"),
        everywhere(Need::Source),
        "the extra a dependency asks for"
    );
    let cizim = &found.iter().find(|u| u.name == "cizim").unwrap();
    assert_eq!(cizim.version, "4.0");
    assert_eq!(
        cizim.targets(Need::Source),
        [LINUX_ARM, WINDOWS, WINDOWS_ARM, MAC, MAC_INTEL],
        "an optional feature's"
    );
    assert!(cizim.targets(Need::Nothing).is_empty());
    // On 3.11 the fork without a wheel is installed, and numpy's wheels are 3.12's.
    let old = Target {
        python: (3, 11),
        ..LINUX
    };
    let found = unwheeled(LOCK, "ornek", &[old]);
    assert_eq!(needs(&found, "catal"), [(old, Need::Source)]);
    assert_eq!(needs(&found, "numpy"), [(old, Need::Source)]);
    // A lock that does not read, or without the project: nothing said.
    assert!(unwheeled("[[package]", "ornek", &all).is_empty());
    assert!(unwheeled(LOCK, "baska", &all).is_empty());
}

/// A computer the lock does not support is not judged: its `supported-markers`, its `requires-python`.
#[test]
fn a_computer_the_lock_does_not_support_is_not_judged() {
    let narrowed = LOCK.replacen(
        "requires-python = \">=3.10\"",
        "requires-python = \">=3.10\"\nsupported-markers = [\"platform_machine == 'x86_64' and sys_platform == 'linux'\"]",
        1,
    );
    let found = unwheeled(&narrowed, "ornek", &[LINUX, LINUX_ARM, WINDOWS_ARM]);
    assert!(
        found
            .iter()
            .all(|u| u.needs.iter().all(|(t, _)| *t == LINUX)),
        "{found:?}"
    );
    assert_eq!(needs(&found, "tasli"), [(LINUX, Need::Source)]);
    let newer = LOCK.replacen(">=3.10", ">=3.13", 1);
    assert!(
        unwheeled(&newer, "ornek", &[LINUX]).is_empty(),
        "3.12 is not taken"
    );
}

/// The computers a package says it runs on, with the Python it gets there; said by system.
#[test]
fn the_declared_computers_and_their_words() {
    let all = Target::declared(&[], None);
    assert_eq!(all.len(), 6);
    assert!(all.iter().all(|t| t.python == (3, 12)));
    let windows = Target::declared(&[Os::Windows], Some(">=3.12, <3.14"));
    assert_eq!(windows, [WINDOWS, WINDOWS_ARM]);
    assert_eq!(
        Target::declared(&[Os::Linux], Some(">=3.13"))[0].python,
        (3, 13)
    );
    assert_eq!(
        Target::declared(&[Os::Linux], Some("<3.11"))[0].python,
        (3, 10)
    );
    assert_eq!(
        targets_words(&[WINDOWS, LINUX_ARM, WINDOWS_ARM]),
        "Windows (x86_64, aarch64), Linux (aarch64)"
    );
}

/// The computers and the Python here are KentOS's: the Python environment's own list.
#[test]
fn the_computers_are_kentos_python_environments() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../native/python-env/compat.json");
    let compat: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let listed: BTreeSet<(String, String)> = compat["platforms"]
        .as_array()
        .unwrap()
        .iter()
        .map(|p| {
            (
                p["os"].as_str().unwrap().to_owned(),
                p["arch"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let ours: BTreeSet<(String, String)> = PLATFORMS
        .iter()
        .map(|(os, m)| (os.id().to_owned(), m.id().to_owned()))
        .collect();
    assert_eq!(ours, listed);
    assert_eq!(compat["python"]["install"], "3.12");
    assert_eq!(compat["python"]["range"], ">=3.10,<3.15");
}

/// The demo extension's lock is narrowed to the three computers it names and has their wheels: nothing to say.
#[test]
fn the_demo_extensions_lock_needs_no_build() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../samples/eklenti-vitrini/eklenti");
    let lock = std::fs::read_to_string(root.join("uv.lock")).unwrap();
    let found = unwheeled(
        &lock,
        "vitrin",
        &Target::declared(&[], Some(">=3.12,<3.14")),
    );
    assert!(found.is_empty(), "{found:?}");
}

/// The packages a computer must not build: every locked one but those the review listed as built there (§98).
#[test]
fn the_packages_not_to_build_are_the_wheeled_ones() {
    let python = crate::manifest::PythonProject {
        project_file: "pyproject.toml".into(),
        lock_file: "uv.lock".into(),
        optional_features: Default::default(),
    };
    let files: BTreeMap<String, Vec<u8>> = [
        (
            "pyproject.toml".to_owned(),
            b"[project]\nname = \"ornek\"\nversion = \"1.0.0\"\nrequires-python = \">=3.10\"\n"
                .to_vec(),
        ),
        ("uv.lock".to_owned(), LOCK.as_bytes().to_vec()),
    ]
    .into();
    let linux = wheeled_on(&python, &files, LINUX);
    // Built on Linux x86_64: tasli, depodan, corap (sources only); the others' wheels fit there, they do not build.
    for not in ["numpy", "saf", "istekler", "cizim", "pywin32", "catal"] {
        assert!(linux.contains(&not.to_owned()), "{not}: {linux:?}");
    }
    for built in ["tasli", "depodan", "corap"] {
        assert!(!linux.contains(&built.to_owned()), "{built}: {linux:?}");
    }
    assert!(!linux.contains(&"ornek".to_owned()), "the project itself");
    let arm = wheeled_on(&python, &files, LINUX_ARM);
    assert!(!arm.contains(&"numpy".to_owned()), "built there: {arm:?}");
    let missing = crate::manifest::PythonProject {
        lock_file: "yok.lock".into(),
        ..python.clone()
    };
    assert!(
        wheeled_on(&missing, &files, LINUX).is_empty(),
        "a lock that is not there"
    );
}

/// What the organisation's wheel folder must hold for one computer (docs/adr/8057 §123): each package the lock installs
/// there with the chosen optional features, and the lock's wheels of it that fit with their hashes; a package with only
/// a source or from a repository has none.
#[test]
fn the_wheels_a_lock_names_for_a_computer_with_their_hashes() {
    let project = crate::manifest::PythonProject {
        project_file: "pyproject.toml".into(),
        lock_file: "uv.lock".into(),
        optional_features: BTreeMap::new(),
    };
    let files = BTreeMap::from([
        (
            "pyproject.toml".to_owned(),
            b"[project]\nname = \"ornek\"\n".to_vec(),
        ),
        ("uv.lock".to_owned(), LOCK.as_bytes().to_vec()),
    ]);
    let said = |target: &Target, extras: &[String]| -> Vec<String> {
        locked_wheels(&project, &files, target, extras)
            .expect("reads")
            .iter()
            .map(|w| {
                let files: Vec<String> = w.files.iter().map(|(f, h)| format!("{f} {h}")).collect();
                format!("{} {}: {}", w.name, w.version, files.join(", "))
            })
            .collect()
    };
    assert_eq!(
        said(&LINUX, &[]),
        [
            "catal 2.0: catal-2.0-py3-none-any.whl sha256:10",
            "corap 1.7: ",
            "depodan 0.1: ",
            "istekler 2.0: istekler-2.0-py3-none-any.whl sha256:07",
            "numpy 2.5.4: numpy-2.5.4-cp312-cp312-manylinux_2_27_x86_64.manylinux_2_28_x86_64.whl sha256:01",
            "saf 1.1: saf-1.1-py3-none-any.whl sha256:06",
            "tasli 0.3: "
        ],
        "the project and Windows' package left out, no optional feature chosen"
    );
    let windows = said(&WINDOWS, &["grafik".to_owned()]);
    assert!(
        windows
            .contains(&"pywin32 311: pywin32-311-cp312-cp312-win_amd64.whl sha256:04".to_owned())
    );
    assert!(
        windows.contains(&"cizim 4.0: ".to_owned()),
        "the feature chosen; no wheel for Windows"
    );
    assert!(
        windows
            .contains(&"numpy 2.5.4: numpy-2.5.4-cp312-cp312-win_amd64.whl sha256:02".to_owned())
    );
}
