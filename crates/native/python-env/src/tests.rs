//! ADIM 18A's first slice (plan/eklenti.md): the manifest read, uv found where a desktop session would miss it, its
//! version checked, and uv run without a shell, within its time and output, with none of the environment's uv
//! settings. A fake uv (a small shell script) stands in for uv: nothing touches the user's own uv, Python or network.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::Duration;

use crate::compat::{self, Compat, Platform};
use crate::settings::PythonSettings;
use crate::uv::{self, UvStatus};

/// A folder of its own, removed afterwards.
struct Temp(PathBuf);

impl Temp {
    fn new(name: &str) -> Temp {
        let dir = std::env::temp_dir().join(format!("kentos-python-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        Temp(dir)
    }
}

impl Drop for Temp {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

/// A script that runs as uv: `body` after `#!/bin/sh`.
#[cfg(unix)]
fn fake_uv(dir: &Path, body: &str) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).unwrap();
    let file = dir.join("uv");
    std::fs::write(&file, format!("#!/bin/sh\n{body}\n")).unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    file
}

fn linux() -> Platform {
    Platform {
        os: "linux".into(),
        arch: "x86_64".into(),
    }
}

#[test]
fn the_built_in_manifest_reads_and_a_broken_one_refuses_everything() {
    let c = compat::compat();
    assert_eq!(c.uv.min, "0.9.0");
    assert!(c.supports(&linux()));
    assert!(c.uv_new_enough("0.11.26") && c.uv_new_enough("0.9.0") && !c.uv_new_enough("0.8.24"));
    assert!(c.install_commands().iter().all(|cmd| !cmd.is_empty()));
    // Windows: the PowerShell installer, and WinGet for computers that refuse `-ExecutionPolicy ByPass`.
    let windows = c.install_commands_for("windows");
    assert!(windows[0].contains("install.ps1") && windows[0].starts_with("powershell"));
    assert!(
        windows
            .iter()
            .any(|cmd| cmd.starts_with("winget install") && cmd.contains("astral-sh.uv"))
    );
    assert!(c.supports(&Platform {
        os: "windows".into(),
        arch: "x86_64".into()
    }));
    assert!(Compat::read("{}").is_err());
    let newer =
        include_str!("../compat.json").replace("\"schemaVersion\": 1", "\"schemaVersion\": 2");
    assert!(Compat::read(&newer).unwrap_err().contains("sürüm 1'i okur"));
    assert_eq!(
        compat::triple("0.11.26 (x86_64-unknown-linux-gnu)"),
        Some((0, 11, 26))
    );
    assert_eq!(compat::triple("1.2"), Some((1, 2, 0)));
    assert_eq!(compat::triple("yok"), None);
}

/// A program started from the desktop menu does not read the shell's profile: uv in `~/.local/bin` is found without it
/// on `PATH`; a chosen path is looked at alone.
#[test]
fn uv_is_found_where_a_desktop_sessions_path_would_miss_it() {
    let home = Path::new("/ev/kullanici");
    let looked = uv::candidates(None, Some(OsStr::new("/usr/bin:/bin")), Some(home), None);
    assert_eq!(
        looked,
        [
            PathBuf::from("/usr/bin/uv"),
            PathBuf::from("/bin/uv"),
            home.join(".local/bin/uv"),
            home.join(".cargo/bin/uv"),
        ]
    );
    let chosen = Path::new("/opt/araclar/uv");
    assert_eq!(
        uv::candidates(Some(chosen), Some(OsStr::new("/usr/bin")), Some(home), None),
        [chosen.to_path_buf()]
    );
    let none = uv::detect_in(compat::compat(), &linux(), &[PathBuf::from("/yok/uv")]);
    assert!(
        matches!(&none, UvStatus::NotFound { looked } if looked.len() == 1),
        "{none:?}"
    );
    assert!(none.words().contains("resmî komutu kendiniz çalıştırın"));
    // Windows: `uv.exe`; the standalone installer's `%USERPROFILE%\.local\bin`, WinGet's links, Scoop's shims.
    let user = Path::new("C:/Users/ayse");
    let local = Path::new("C:/Users/ayse/AppData/Local");
    let looked = uv::candidates_for("windows", None, None, Some(user), Some(local));
    assert_eq!(
        looked,
        [
            user.join(".local/bin/uv.exe"),
            user.join(".cargo/bin/uv.exe"),
            local.join("Microsoft/WinGet/Links/uv.exe"),
            user.join("scoop/shims/uv.exe"),
        ]
    );
    let elsewhere = Platform {
        os: "freebsd".into(),
        arch: "x86_64".into(),
    };
    assert!(matches!(
        uv::detect_in(compat::compat(), &elsewhere, &[]),
        UvStatus::Unsupported { .. }
    ));
}

#[cfg(unix)]
#[test]
fn uvs_version_is_asked_and_checked() {
    let temp = Temp::new("surum");
    let new = fake_uv(
        &temp.0.join("yeni"),
        "echo 'uv 0.11.26 (x86_64-unknown-linux-gnu)'",
    );
    let found = uv::detect_in(
        compat::compat(),
        &linux(),
        &[PathBuf::from("/yok/uv"), new.clone()],
    );
    assert_eq!(
        found,
        UvStatus::Found {
            path: new,
            version: "0.11.26".into()
        }
    );
    assert!(found.ok());
    let old = fake_uv(&temp.0.join("eski"), "echo 'uv 0.5.1'");
    let too_old = uv::detect_in(compat::compat(), &linux(), &[old]);
    assert!(
        matches!(&too_old, UvStatus::TooOld { min, .. } if min == "0.9.0"),
        "{too_old:?}"
    );
    assert!(too_old.words().contains("uv self update"));
    let silent = fake_uv(&temp.0.join("sessiz"), "echo 'merhaba'; exit 3");
    let not = uv::detect_in(compat::compat(), &linux(), &[silent]);
    assert!(
        matches!(&not, UvStatus::NotRunnable { why, .. } if why.contains("merhaba")),
        "{not:?}"
    );
    // A file that is not a program.
    let plain = temp.0.join("duz").join("uv");
    std::fs::create_dir_all(plain.parent().unwrap()).unwrap();
    std::fs::write(&plain, "uv değil").unwrap();
    assert!(matches!(
        uv::detect_in(compat::compat(), &linux(), &[plain]),
        UvStatus::NotRunnable { .. }
    ));
}

/// uv runs with its arguments, not a shell's words; without the environment's uv and Python settings; within its time
/// (ended after) and its output (cut after).
#[cfg(unix)]
#[test]
fn uv_runs_without_a_shell_within_its_time_and_output() {
    let temp = Temp::new("calistir");
    let echo = fake_uv(
        &temp.0.join("yanki"),
        "printf '%s|' \"$@\"; echo; echo \"ek=${KENTOS_EK:-yok}\"",
    );
    // What steers uv or Python is taken out of its environment (setting one here would race the other tests).
    for name in [
        "UV_INDEX_URL",
        "UV_CACHE_DIR",
        "VIRTUAL_ENV",
        "PYTHONPATH",
        "PYTHONHOME",
        "CONDA_PREFIX",
    ] {
        assert!(uv::steers(OsStr::new(name)), "{name}");
    }
    assert!(!uv::steers(OsStr::new("PATH")) && !uv::steers(OsStr::new("HOME")));
    let ran = uv::run(
        &echo,
        &[
            OsStr::new("sync"),
            OsStr::new("--project"),
            OsStr::new("a b; rm -rf /"),
        ],
        None,
        &[("KENTOS_EK", OsStr::new("geldi"))],
        Duration::from_secs(10),
    )
    .unwrap();
    assert_eq!(ran.status, Some(0));
    assert!(
        ran.stdout.starts_with("sync|--project|a b; rm -rf /|"),
        "one argument, not shell words: {}",
        ran.stdout
    );
    assert!(ran.stdout.contains("ek=geldi"), "{}", ran.stdout);
    let slow = fake_uv(&temp.0.join("yavas"), "sleep 30");
    let started = std::time::Instant::now();
    let ran = uv::run(&slow, &[], None, &[], Duration::from_millis(300)).unwrap();
    assert!(ran.timed_out && ran.status.is_none());
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "ended at its limit"
    );
    let loud = fake_uv(
        &temp.0.join("gurultu"),
        "head -c 600000 /dev/zero | tr '\\0' 'x'",
    );
    let ran = uv::run(&loud, &[], None, &[], Duration::from_secs(10)).unwrap();
    assert!(
        ran.cut && ran.stdout.len() == uv::MAX_OUTPUT,
        "{}",
        ran.stdout.len()
    );
}

#[test]
fn the_choice_is_off_until_the_user_turns_it_on_and_is_kept() {
    let temp = Temp::new("ayar");
    assert_eq!(PythonSettings::load(&temp.0), PythonSettings::default());
    assert!(!PythonSettings::load(&temp.0).enabled, "off by default");
    let chosen = PythonSettings {
        enabled: true,
        uv_path: Some("/opt/araclar/uv".into()),
    };
    chosen.save(&temp.0).unwrap();
    assert_eq!(PythonSettings::load(&temp.0), chosen);
    std::fs::write(temp.0.join(crate::settings::FILE), "{ bozuk").unwrap();
    assert_eq!(
        PythonSettings::load(&temp.0),
        PythonSettings::default(),
        "unreadable: off"
    );
}
