//! ADIM 18A's first slice (plan/eklenti.md): the manifest read, uv found where a desktop session would miss it, its
//! version checked, and uv run without a shell, within its time and output, with none of the environment's uv
//! settings. A fake uv (a small shell script) stands in for uv: nothing touches the user's own uv, Python or network.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::Duration;

use crate::compat::{self, Compat, Platform};
use crate::extension;
use crate::personal::{self, Change, Package};
use crate::python::{self, Cause, InstallError, Interpreter, Progress};
use crate::sdk;
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
    // The uv the installation carries: new enough, and a file with its SHA-256 for every platform listed.
    let b = &c.bundled_uv;
    assert!(c.uv_new_enough(&b.version), "{}", b.version);
    assert!(b.download.ends_with(&format!("/{}/", b.version)));
    assert!(
        b.licenses.len() == 2
            && b.licenses.iter().all(
                |l| l.url.contains(&format!("/{}/LICENSE-", b.version)) && l.sha256.len() == 64
            )
    );
    for p in &c.platforms {
        let os = match p.os.as_str() {
            "windows" => "pc-windows-msvc",
            "macos" => "apple-darwin",
            _ => "unknown-linux-gnu",
        };
        let target = format!("{}-{os}", p.arch);
        let asset = b.targets.get(&target).unwrap_or_else(|| panic!("{target}"));
        assert!(asset.file.starts_with(&format!("uv-{target}.")));
        assert!(
            asset.sha256.len() == 64
                && asset
                    .sha256
                    .chars()
                    .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()),
            "{target}"
        );
    }
    assert_eq!(b.targets.len(), c.platforms.len());
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

/// The installation's own uv is looked at first (the owner's decision, 13 November 2026); then, as a program started
/// from the desktop menu does not read the shell's profile, uv in `~/.local/bin` is found without it on `PATH`; a
/// chosen path is looked at alone.
#[test]
fn uv_is_found_where_a_desktop_sessions_path_would_miss_it() {
    let home = Path::new("/ev/kullanici");
    let own = [
        PathBuf::from("/opt/kentos/tools"),
        PathBuf::from("/opt/kentos"),
    ];
    let looked = uv::candidates(
        None,
        &own,
        Some(OsStr::new("/usr/bin:/bin")),
        Some(home),
        None,
    );
    assert_eq!(
        looked,
        [
            PathBuf::from("/opt/kentos/tools/uv"),
            PathBuf::from("/opt/kentos/uv"),
            PathBuf::from("/usr/bin/uv"),
            PathBuf::from("/bin/uv"),
            home.join(".local/bin/uv"),
            home.join(".cargo/bin/uv"),
        ]
    );
    let chosen = Path::new("/opt/araclar/uv");
    assert_eq!(
        uv::candidates(
            Some(chosen),
            &own,
            Some(OsStr::new("/usr/bin")),
            Some(home),
            None
        ),
        [chosen.to_path_buf()]
    );
    // Not found where the installation keeps it: said so; a chosen path looked at alone is said to be that path.
    let none = uv::detect_in(
        compat::compat(),
        &linux(),
        &own,
        &uv::candidates(None, &own, None, None, None),
    );
    assert!(
        matches!(&none, UvStatus::NotFound { looked, own: true } if looked.len() == 2),
        "{none:?}"
    );
    assert!(none.words().contains("KentOS'un kendi klasörü dahil"));
    assert!(
        none.words()
            .contains("KentOS'la gelen uv silinmiş olabilir")
    );
    assert!(none.words().contains("resmî komutu kendiniz çalıştırıp"));
    assert!(none.offers_install());
    let typed = uv::detect_in(
        compat::compat(),
        &linux(),
        &own,
        &uv::candidates(Some(Path::new("/yok/uv")), &own, None, None, None),
    );
    assert!(
        matches!(&typed, UvStatus::NotFound { own: false, .. }),
        "{typed:?}"
    );
    assert!(
        typed.words().starts_with("uv verilen yolda yok: /yok/uv.")
            && !typed.words().contains("kendi klasörü dahil"),
        "{}",
        typed.words()
    );
    // The way out is the path, not installing uv.
    assert!(!typed.offers_install());
    // Windows: `uv.exe` beside the program first; then the standalone installer's `%USERPROFILE%\.local\bin`,
    // WinGet's links, Scoop's shims.
    let user = Path::new("C:/Users/ayse");
    let local = Path::new("C:/Users/ayse/AppData/Local");
    let program = [PathBuf::from("C:/Program Files/KentOS/tools")];
    let looked = uv::candidates_for("windows", None, &program, None, Some(user), Some(local));
    assert_eq!(
        looked,
        [
            PathBuf::from("C:/Program Files/KentOS/tools/uv.exe"),
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
        uv::detect_in(compat::compat(), &elsewhere, &own, &[]),
        UvStatus::Unsupported { .. }
    ));
    // The installation's own folders: `tools` beside the program; in a macOS app bundle the program's own folder, as
    // Apple asks helper tools to be placed; a program in `/usr/bin` does not make `/usr/bin/uv` KentOS's.
    let app = Path::new("/Applications/KentOS.app/Contents/MacOS");
    assert_eq!(
        uv::bundled_dirs_for("macos", app),
        [app.join("tools"), app.to_path_buf()]
    );
    assert_eq!(
        uv::bundled_dirs_for("linux", Path::new("/usr/bin")),
        [PathBuf::from("/usr/bin/tools")]
    );
    assert_eq!(
        uv::bundled_dirs_for("macos", Path::new("/usr/local/bin")),
        [PathBuf::from("/usr/local/bin/tools")]
    );
    let exe_dir = std::env::current_exe()
        .unwrap()
        .parent()
        .unwrap()
        .to_path_buf();
    assert_eq!(
        uv::bundled_dirs(),
        uv::bundled_dirs_for(std::env::consts::OS, &exe_dir)
    );
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
        &[],
        &[PathBuf::from("/yok/uv"), new.clone()],
    );
    assert_eq!(
        found,
        UvStatus::Found {
            path: new.clone(),
            version: "0.11.26".into(),
            bundled: false,
        }
    );
    assert!(found.ok());
    assert!(found.words().starts_with("uv 0.11.26 bulundu"));
    // The same file in the installation's own folder: said to be KentOS's.
    let own = uv::detect_in(
        compat::compat(),
        &linux(),
        &[temp.0.join("yeni")],
        std::slice::from_ref(&new),
    );
    assert!(
        matches!(&own, UvStatus::Found { bundled: true, .. }),
        "{own:?}"
    );
    assert!(own.words().starts_with("KentOS'la gelen uv 0.11.26 hazır"));
    let old = fake_uv(&temp.0.join("eski"), "echo 'uv 0.5.1'");
    let too_old = uv::detect_in(compat::compat(), &linux(), &[], &[old]);
    assert!(
        matches!(&too_old, UvStatus::TooOld { min, .. } if min == "0.9.0"),
        "{too_old:?}"
    );
    assert!(too_old.words().contains("uv self update") && too_old.offers_install());
    assert!(!found.offers_install());
    let silent = fake_uv(&temp.0.join("sessiz"), "echo 'merhaba'; exit 3");
    let not = uv::detect_in(compat::compat(), &linux(), &[], &[silent]);
    assert!(
        matches!(&not, UvStatus::NotRunnable { why, .. } if why.contains("merhaba")),
        "{not:?}"
    );
    // A file that is not a program.
    let plain = temp.0.join("duz").join("uv");
    std::fs::create_dir_all(plain.parent().unwrap()).unwrap();
    std::fs::write(&plain, "uv değil").unwrap();
    assert!(matches!(
        uv::detect_in(compat::compat(), &linux(), &[], &[plain]),
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

/// A fake uv that writes where it ran, KentOS's two folders it was given and its arguments to `log`, then does `body`.
#[cfg(unix)]
fn logging_uv(dir: &Path, log: &Path, body: &str) -> PathBuf {
    fake_uv(
        dir,
        &format!(
            "printf '%s|%s|%s|%s\\n' \"$(pwd -P)\" \"$UV_PYTHON_INSTALL_DIR\" \"$UV_CACHE_DIR\" \"$*\" >> '{}'\n{body}",
            log.display()
        ),
    )
}

/// KentOS's Python is looked for in KentOS's folder alone, with uv's own list: the newest CPython (not PyPy, not the
/// free-threaded build) in the manifest's range; none is none; uv's refusal is said.
#[cfg(unix)]
#[test]
fn kentos_python_is_found_in_kentos_folder_alone() {
    let temp = Temp::new("yorumlayici");
    let root = temp.0.join("python");
    let log = temp.0.join("kayit.txt");
    let listed = temp.0.join("liste.json");
    std::fs::write(
        &listed,
        r#"[
          {"key":"cpython-3.11.9-linux-x86_64-gnu","version":"3.11.9","path":"/k/py311/bin/python3.11","implementation":"cpython","variant":"default"},
          {"key":"cpython-3.12.13-linux-x86_64-gnu","version":"3.12.13","path":"yorumlayici/cpython-3.12-linux-x86_64-gnu/bin/python3.12","implementation":"cpython","variant":"default","yeniAlan":1},
          {"key":"cpython-3.13.5+freethreaded-linux-x86_64-gnu","version":"3.13.5","path":"/k/py313t/bin/python3.13t","implementation":"cpython","variant":"freethreaded"},
          {"key":"pypy-3.10.16-linux-x86_64-gnu","version":"3.10.16","path":"/k/pypy/bin/pypy3","implementation":"pypy","variant":"default"}
        ]"#,
    )
    .unwrap();
    let fake = logging_uv(
        &temp.0.join("uv"),
        &log,
        &format!("cat '{}'", listed.display()),
    );
    let range = &compat::compat().python.range;
    let found = python::installed(&fake, &root, range).unwrap();
    // uv writes a path under the folder it runs in relative to it: read against KentOS's folder.
    assert_eq!(
        found,
        Some(Interpreter {
            version: "3.12.13".into(),
            path: root.join("yorumlayici/cpython-3.12-linux-x86_64-gnu/bin/python3.12")
        })
    );
    let line = std::fs::read_to_string(&log).unwrap();
    let real_root = std::fs::canonicalize(&root).unwrap();
    assert_eq!(
        line.trim(),
        format!(
            "{}|{}|{}|python list --only-installed --managed-python --no-config --output-format json {range}",
            real_root.display(),
            root.join("yorumlayici").display(),
            root.join("onbellek").display()
        ),
        "run in KentOS's folder, with KentOS's folders for uv"
    );
    let none = logging_uv(&temp.0.join("bos"), &log, "echo '[]'");
    assert_eq!(python::installed(&none, &root, range), Ok(None));
    let refusing = logging_uv(
        &temp.0.join("ret"),
        &log,
        "echo 'error: liste bozuk' >&2; exit 2",
    );
    assert_eq!(
        python::installed(&refusing, &root, range),
        Err("liste bozuk".into())
    );
}

/// Installing asks uv for the manifest's version into KentOS's folder and nothing of the system (no `bin` link, no
/// registry entry, none of the user's uv settings, the system's certificates); the user's Durdur ends it at once.
#[cfg(unix)]
#[test]
fn python_is_installed_into_kentos_folder_and_can_be_stopped() {
    let temp = Temp::new("kurulum");
    let root = temp.0.join("python");
    let log = temp.0.join("kayit.txt");
    // uv as it downloads: says the archive's size, the archive grows in the download folder it was given, then it
    // unpacks (measured on uv 0.11.26).
    let fake = logging_uv(
        &temp.0.join("uv"),
        &log,
        "echo 'Downloading cpython-3.12.13-linux-x86_64-gnu (download) (1.0MiB)' >&2
head -c 524288 /dev/zero > \"$UV_PYTHON_CACHE_DIR/.tmpindir\"
sleep 0.6
head -c 524288 /dev/zero >> \"$UV_PYTHON_CACHE_DIR/.tmpindir\"
sleep 0.6
echo ' Downloaded cpython-3.12.13-linux-x86_64-gnu (download)' >&2
echo 'Extracting cpython-3.12.13-linux-x86_64-gnu (extract) (1.0MiB)' >&2
sleep 0.6",
    );
    let version = &compat::compat().python.install;
    assert_eq!(version, "3.12");
    let stop = std::sync::atomic::AtomicBool::new(false);
    let mut seen = Vec::new();
    assert_eq!(
        python::install(&fake, &root, version, &stop, &mut |p| seen.push(p)),
        Ok(())
    );
    // How far it was, as it changed: the half and the whole of the archive, then unpacking.
    assert_eq!(seen.first(), Some(&Progress::Starting), "{seen:?}");
    let half = Progress::Downloading {
        done: 524_288,
        total: Some(1_048_576),
    };
    assert!(seen.contains(&half), "{seen:?}");
    assert_eq!(half.share(), Some(0.5));
    assert!(
        seen.contains(&Progress::Downloading {
            done: 1_048_576,
            total: Some(1_048_576)
        }),
        "{seen:?}"
    );
    assert_eq!(seen.last(), Some(&Progress::Unpacking), "{seen:?}");
    assert!(!root.join("indirme").exists(), "the download is not kept");
    let line = std::fs::read_to_string(&log).unwrap();
    assert!(
        line.trim_end().ends_with(
            "|python install 3.12 --no-bin --no-registry --no-config --no-progress --system-certs"
        ),
        "{line}"
    );
    assert!(line.contains(&format!("|{}|", root.join("yorumlayici").display())));
    assert!(
        root.join("yorumlayici").is_dir(),
        "its folder is made first"
    );
    // Durdur.
    let slow = fake_uv(&temp.0.join("yavas"), "sleep 30");
    let stop = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let pressed = stop.clone();
    std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        pressed.store(true, std::sync::atomic::Ordering::Relaxed);
    });
    let started = std::time::Instant::now();
    assert_eq!(
        python::install(&slow, &root, version, &stop, &mut |_| {}),
        Err(InstallError::Cancelled)
    );
    assert!(!root.join("indirme").exists(), "nor a stopped one");
    assert!(
        started.elapsed() < Duration::from_secs(10),
        "stopped at once"
    );
    assert!(
        InstallError::Cancelled
            .words(version)
            .contains("durduruldu")
    );
}

/// A failed install says why in a sentence with what to do, and uv's own words beside it: no internet, an untrusted
/// certificate (a network that inspects TLS), a full disk, a folder it may not write, no build for this system.
#[test]
fn a_failed_install_says_why_and_what_to_do() {
    let offline = "error: Failed to install cpython-3.12.13-linux-x86_64-gnu
  Caused by: Failed to download https://github.com/astral-sh/python-build-standalone/releases/download/x.tar.gz
  Caused by: Request failed after 3 retries
  Caused by: error sending request for url (https://github.com/)
  Caused by: client error (Connect)
  Caused by: dns error: failed to lookup address information: Name or service not known";
    assert_eq!(python::cause_of(offline), Cause::Network);
    assert_eq!(
        python::said(offline).unwrap(),
        "Failed to install cpython-3.12.13-linux-x86_64-gnu · dns error: failed to lookup address information: Name \
         or service not known"
    );
    let failed = InstallError::Failed {
        cause: Cause::Network,
        said: python::said(offline).unwrap(),
    };
    let words = failed.words("3.12");
    assert!(
        words.starts_with("Python indirilemedi: internete ulaşılamadı")
            && words.contains("HTTPS_PROXY")
            && words.contains("uv: “Failed to install"),
        "{words}"
    );
    let inspected = "error: Failed to install cpython-3.12.13
  Caused by: Failed to download https://github.com/x
  Caused by: error sending request for url (https://github.com/x)
  Caused by: client error (Connect)
  Caused by: invalid peer certificate: UnknownIssuer";
    assert_eq!(python::cause_of(inspected), Cause::Certificate);
    assert!(
        InstallError::Failed {
            cause: Cause::Certificate,
            said: String::new()
        }
        .words("3.12")
        .contains("kurumun sertifikası")
    );
    for (text, cause) in [
        (
            "error: x\n  Caused by: No space left on device (os error 28)",
            Cause::Disk,
        ),
        (
            "error: x\n  Caused by: There is not enough space on the disk. (os error 112)",
            Cause::Disk,
        ),
        (
            "error: x\n  Caused by: Access is denied. (os error 5)",
            Cause::Folder,
        ),
        (
            "error: x\n  Caused by: Permission denied (os error 13)",
            Cause::Folder,
        ),
        (
            "error: No download found for request: cpython-3.12-freebsd-x86_64",
            Cause::Unavailable,
        ),
        ("error: something else entirely", Cause::Other),
    ] {
        assert_eq!(python::cause_of(text), cause, "{text}");
    }
    assert!(
        InstallError::Failed {
            cause: Cause::Unavailable,
            said: String::new()
        }
        .words("3.12")
        .starts_with("Python 3.12 bu sistem için indirilemiyor.")
    );
    assert_eq!(python::said("only words, no error line"), None);
    assert!(InstallError::TimedOut.words("3.12").contains("20 dakikada"));
}

/// uv's sizes and phases read: the archive's size from its first line, the bytes downloaded beside it, unpacking once
/// it says so; nothing said yet is a start.
#[test]
fn how_far_installing_is_is_read_from_uvs_words() {
    assert_eq!(python::bytes("32.6MiB"), Some(34_183_578));
    assert_eq!(python::bytes("512KiB"), Some(524_288));
    assert_eq!(python::bytes("900B"), Some(900));
    assert_eq!(python::bytes("1.2GiB"), Some(1_288_490_189));
    assert_eq!(python::bytes("çok"), None);
    assert_eq!(python::read_progress("", 0), Progress::Starting);
    let said = "Downloading cpython-3.12.13-linux-x86_64-gnu (download) (32.6MiB)\n";
    let early = python::read_progress(said, 3_418_399);
    assert_eq!(
        early,
        Progress::Downloading {
            done: 3_418_399,
            total: Some(34_183_578)
        }
    );
    assert!((early.share().unwrap() - 0.1).abs() < 0.001);
    // A size uv rounds can be passed by a few bytes: never more than the whole.
    assert_eq!(python::read_progress(said, 34_200_000).share(), Some(1.0));
    // Bytes but no size said: downloading, share unknown.
    let no_size = python::read_progress("", 1000);
    assert_eq!(no_size.share(), None);
    assert!(matches!(no_size, Progress::Downloading { total: None, .. }));
    let unpacking = format!(
        "{said} Downloaded cpython-3.12.13-linux-x86_64-gnu (download)\nExtracting cpython-3.12.13 (extract) (32.6MiB)\n"
    );
    assert_eq!(
        python::read_progress(&unpacking, 34_159_222),
        Progress::Unpacking
    );
}

/// Removed from the same place: uv uninstalls KentOS's Pythons, then their folder and what downloads left go; the
/// bytes freed are said; a link is not counted twice; a Python still in use is said so.
#[cfg(unix)]
#[test]
fn kentos_python_is_removed_with_what_it_left() {
    let temp = Temp::new("kaldir");
    let root = temp.0.join("python");
    let log = temp.0.join("kayit.txt");
    let folder = root.join("yorumlayici");
    std::fs::create_dir_all(folder.join("cpython-3.12.13/bin")).unwrap();
    std::fs::write(folder.join("cpython-3.12.13/bin/python3.12"), [0u8; 1000]).unwrap();
    std::os::unix::fs::symlink(folder.join("cpython-3.12.13"), folder.join("cpython-3.12"))
        .unwrap();
    std::fs::write(folder.join(".lock"), b"").unwrap();
    std::fs::create_dir_all(root.join("indirme/1-2")).unwrap();
    std::fs::write(root.join("indirme/1-2/.tmpx"), [0u8; 500]).unwrap();
    std::fs::write(root.join("ayarlar.json"), b"{}").unwrap();
    std::fs::create_dir_all(root.join("kisisel/.venv/bin")).unwrap();
    std::fs::write(root.join("kisisel/pyproject.toml"), b"").unwrap();
    assert_eq!(
        python::folder_size(&folder),
        1000,
        "the link is not counted twice"
    );
    let fake = logging_uv(&temp.0.join("uv"), &log, "exit 0");
    assert_eq!(python::remove(Some(&fake), &root), Ok(1500));
    assert!(
        std::fs::read_to_string(&log)
            .unwrap()
            .trim_end()
            .ends_with("|python uninstall --all --no-config")
    );
    assert!(!folder.exists() && !root.join("indirme").exists());
    assert!(
        !root.join("kisisel").exists(),
        "the personal environment, which runs on it, goes too"
    );
    assert!(
        root.join("ayarlar.json").exists(),
        "the user's choice stays"
    );
    assert_eq!(
        python::remove(None, &root),
        Ok(0),
        "nothing left: nothing to do"
    );
    // In use (Windows keeps a running Python's files).
    std::fs::create_dir_all(&folder).unwrap();
    let busy = logging_uv(
        &temp.0.join("mesgul"),
        &log,
        "echo 'error: Failed to uninstall cpython-3.12.13' >&2
echo '  Caused by: The process cannot access the file because it is being used by another process. (os error 32)' >&2
exit 2",
    );
    let refused = python::remove(Some(&busy), &root).unwrap_err();
    assert!(
        refused.starts_with("Python kaldırılamadı: dosyaları kullanılıyor")
            && refused.contains("Python konsolunu"),
        "{refused}"
    );
}

/// KentOS's library comes from the installation's wheel as its description says: there, of this program's version,
/// with its SHA-256; anything else is refused with what to do.
#[test]
fn the_kentos_library_wheel_is_found_only_as_described() {
    let temp = Temp::new("sdk");
    let dir = temp.0.join("tools");
    std::fs::create_dir_all(&dir).unwrap();
    let wheel = dir.join("kentos-0.1.0-cp310-abi3-linux_x86_64.whl");
    std::fs::write(&wheel, b"tekerlek").unwrap();
    let sum = sdk::sha256_file(&wheel).unwrap();
    assert_eq!(sum.len(), 64);
    let describe = |version: &str, file: &str, sha256: &str| {
        std::fs::write(
            dir.join(sdk::DESCRIPTION),
            format!(
                r#"{{"format":"kentos.sdk-wheel","version":"{version}","file":"{file}","sha256":"{sha256}"}}"#
            ),
        )
        .unwrap();
    };
    let missing = sdk::find(std::slice::from_ref(&dir), "0.1.0").unwrap_err();
    assert!(missing.contains("bu kurulumda yok"), "{missing}");
    describe("0.1.0", "kentos-0.1.0-cp310-abi3-linux_x86_64.whl", &sum);
    let found = sdk::find(&[temp.0.join("yok"), dir.clone()], "0.1.0").unwrap();
    // Where an installation keeps it: `tools`; in a macOS app bundle its Resources.
    let app = Path::new("/Applications/KentOS.app/Contents/MacOS");
    assert_eq!(
        sdk::dirs_for("macos", app),
        [
            app.join("tools"),
            PathBuf::from("/Applications/KentOS.app/Contents/Resources/python")
        ]
    );
    assert_eq!(
        sdk::dirs_for("windows", Path::new("C:/KentOS")),
        [PathBuf::from("C:/KentOS/tools")]
    );
    assert_eq!(found.path, wheel);
    assert_eq!(found.version, "0.1.0");
    let older = sdk::find(std::slice::from_ref(&dir), "0.2.0").unwrap_err();
    assert!(
        older.contains("0.1.0 sürümünde") && older.contains("yeniden kurun"),
        "{older}"
    );
    describe("0.1.0", "../kentos.whl", &sum);
    assert!(
        sdk::find(std::slice::from_ref(&dir), "0.1.0").is_err(),
        "a path is not a file name"
    );
    describe("0.1.0", "kentos-0.1.0-cp310-abi3-linux_x86_64.whl", &sum);
    std::fs::write(&wheel, b"bozulmus").unwrap();
    let broken = sdk::find(std::slice::from_ref(&dir), "0.1.0").unwrap_err();
    assert!(broken.contains("bozuk"), "{broken}");
}

/// What the user types is a package of the index with an optional version: never one of uv's options (it would read a
/// requirement starting with `-` as its own flag), a path or an address; `kentos` is KentOS's library's name.
#[test]
fn requirements_are_packages_of_the_index_only() {
    assert_eq!(personal::check("numpy"), Ok("numpy".into()));
    assert_eq!(personal::check(" Pandas>=2,<3 "), Ok("pandas".into()));
    assert_eq!(
        personal::check("shapely[vectorized]==2.1"),
        Ok("shapely".into())
    );
    assert_eq!(
        personal::check("Python_Dateutil"),
        Ok("python-dateutil".into())
    );
    for refused in [
        "",
        "--index-url=https://ornek.invalid/simple",
        "-e .",
        "./yerel-paket",
        "C:\\paketler\\x.whl",
        "git+https://ornek.invalid/x.git",
        "numpy @ https://ornek.invalid/numpy.whl",
        "numpy; sys_platform == 'win32'",
        "numpy && rm",
    ] {
        assert!(personal::check(refused).is_err(), "{refused}");
    }
    for own in ["kentos", "Kentos", "kentos>=0.1"] {
        let refused = personal::check(own).unwrap_err();
        assert!(
            refused.contains("KentOS'un kendi Python kitaplığı"),
            "{refused}"
        );
    }
}

/// The project KentOS writes is pinned to KentOS's Python and this system (measured: open to every Python, uv locked
/// two pandas at once); the lock's packages and the ones asked for are read from uv's export and the project.
#[test]
fn the_personal_project_is_pinned_and_read() {
    let written = personal::pyproject("3.12.13", "windows");
    assert!(
        written.contains("requires-python = \"==3.12.*\""),
        "{written}"
    );
    assert!(written.contains("environments = [\"sys_platform == 'win32'\"]"));
    assert!(written.contains("package = false") && written.contains("dependencies = []"));
    assert!(personal::pyproject("3.12.13", "linux").contains("sys_platform == 'linux'"));
    assert!(personal::pyproject("3.13.1", "macos").contains("==3.13.*"));
    assert!(personal::direct_names(&written).is_empty());
    let by_uv = "[project]\nname = \"kentos-kisisel\"\ndependencies = [\n    \"pandas>=3.1.1\",\n    \"Shapely[vectorized]>=2.1\",\n]\n\n[tool.uv]\npackage = false\n";
    assert_eq!(personal::direct_names(by_uv), ["pandas", "shapely"]);
    assert_eq!(
        personal::direct_names(
            "[project]\ndependencies = [\"numpy>=2\"]\n[dependency-groups]\ndev = [\"pytest\"]\n"
        ),
        ["numpy"]
    );
    let export = "numpy==2.5.4 ; sys_platform == 'linux'\npandas==3.1.1 ; sys_platform == 'linux'\nPython_Dateutil==4.0.0\n# yorum\n\n-e .\n";
    let packages = personal::exported(export);
    assert_eq!(packages.len(), 3);
    assert_eq!(packages["python-dateutil"], "4.0.0");
    assert_eq!(packages["numpy"], "2.5.4");
    assert_eq!(
        personal::python_in(Path::new("C:/k/.venv"), "windows"),
        Path::new("C:/k/.venv/Scripts/python.exe")
    );
    assert_eq!(
        personal::python_in(Path::new("/k/.venv"), "linux"),
        Path::new("/k/.venv/bin/python")
    );
}

/// How far a sync is: the sizes uv says it downloads, the bytes being unpacked counted back to downloaded ones (so the
/// bar moves before a package is done), those it finished, then installing; never the whole before uv says so.
#[test]
fn a_syncs_progress_is_read_from_uvs_words() {
    assert_eq!(personal::read_sync_progress("", 0), Progress::Starting);
    let started = "Resolved 5 packages in 0.72ms\nDownloading numpy (16.0MiB)\nDownloading pandas (11.5MiB)\n";
    let numpy = python::bytes("16.0MiB").unwrap();
    let pandas = python::bytes("11.5MiB").unwrap();
    let total = numpy + pandas;
    assert_eq!(
        personal::read_sync_progress(started, 0),
        Progress::Downloading {
            done: 0,
            total: Some(total)
        }
    );
    // 35 MB unpacked so far: about 10 MB downloaded.
    assert_eq!(
        personal::read_sync_progress(started, 35_000_000),
        Progress::Downloading {
            done: 10_000_000,
            total: Some(total)
        }
    );
    // More unpacked than the whole would be: not the whole until uv says so.
    let Progress::Downloading { done, .. } = personal::read_sync_progress(started, 500_000_000)
    else {
        panic!("downloading");
    };
    assert!(done < total);
    let one = format!("{started} Downloaded pandas\n");
    assert_eq!(
        personal::read_sync_progress(&one, 0),
        Progress::Downloading {
            done: pandas,
            total: Some(total)
        }
    );
    let all = format!("{one} Downloaded numpy\nPrepared 4 packages in 1.27s\n");
    assert_eq!(personal::read_sync_progress(&all, 0), Progress::Unpacking);
    // What is being unpacked: the cache's `.tmp…` at its top.
    let temp = Temp::new("acilan");
    std::fs::create_dir_all(temp.0.join(".tmpAbC/numpy")).unwrap();
    std::fs::write(temp.0.join(".tmpAbC/numpy/core.so"), [0u8; 700]).unwrap();
    std::fs::create_dir_all(temp.0.join("archive-v0/x")).unwrap();
    std::fs::write(temp.0.join("archive-v0/x/done.py"), [0u8; 300]).unwrap();
    assert_eq!(personal::under_way(&temp.0), 700);
    assert_eq!(personal::under_way(&temp.0.join("yok")), 0);
}

/// The recommended packages: each a package of the index under its normal name, in a known group, found by Turkish
/// words typed with or without their letters.
#[test]
fn recommended_packages_are_found_in_turkish() {
    use crate::catalog;
    let list = catalog::builtin();
    assert_eq!(list.format, "kentos.python-packages");
    assert!(list.packages.len() >= 25, "{}", list.packages.len());
    let mut names = std::collections::BTreeSet::new();
    for entry in &list.packages {
        assert_eq!(
            personal::check(&entry.name),
            Ok(entry.name.clone()),
            "{}",
            entry.name
        );
        assert!(names.insert(entry.name.clone()), "twice: {}", entry.name);
        assert!(list.groups.contains(&entry.group), "{}", entry.group);
        assert!(!entry.title.is_empty());
    }
    let found =
        |q: &str| -> Vec<String> { list.search(q).iter().map(|e| e.name.clone()).collect() };
    assert_eq!(found("").len(), list.packages.len());
    assert!(found("koordinat").contains(&"pyproj".to_owned()));
    assert!(
        found("SAYISAL").contains(&"numpy".to_owned()),
        "ı without its letter"
    );
    assert!(found("sayısal").contains(&"numpy".to_owned()));
    assert!(found("excel").contains(&"openpyxl".to_owned()));
    assert!(found("nokta bulutu").contains(&"laspy".to_owned()));
    assert!(found("İSTATİSTİK").contains(&"scipy".to_owned()));
    assert!(found("bulunmayan bir şey").is_empty());
    assert_eq!(
        catalog::fold("İstatistik Şehir Çağrı"),
        "istatistik sehir cagri"
    );
    assert_eq!(
        list.find("geopandas").map(|e| e.group.as_str()),
        Some("CBS ve geometri")
    );
}

/// A fake uv for the personal project: `add` writes the project uv would (pandas asked for), `export` gives the lock's
/// packages (the project aside: pandas and its dependencies; the project: an older tzdata), `sync` fails when told to
/// (a file `bozuk` beside it), `pip` installs nothing; each call logged.
#[cfg(unix)]
fn personal_uv(dir: &Path, log: &Path) -> PathBuf {
    let fails = dir.join("bozuk");
    logging_uv(
        dir,
        log,
        &format!(
            r#"case "$1" in
  add) printf '%s\n' '[project]' 'name = "kentos-kisisel"' 'dependencies = [' '    "pandas>=3.1.1",' ']' > pyproject.toml ;;
  export)
    if [ "$(basename "$(pwd -P)")" = "kisisel-onizleme" ]; then
      printf '%s\n' "numpy==2.5.4 ; sys_platform == 'linux'" "pandas==3.1.1 ; sys_platform == 'linux'" "tzdata==2026.6"
    else
      printf '%s\n' "tzdata==2025.1"
    fi ;;
  sync)
    if [ -e '{fails}' ]; then echo 'error: Failed to fetch: numpy' >&2; echo '  Caused by: dns error' >&2; exit 2; fi
    echo 'Downloading numpy (16.0MiB)' >&2; echo ' Downloaded numpy' >&2; echo 'Prepared 3 packages in 1s' >&2 ;;
esac"#,
            fails = fails.display()
        ),
    )
}

/// A change is resolved aside and said package by package (asked for or a dependency, new or a new version); the project
/// is untouched until the user agrees; then the environment follows, KentOS's library is put back; a failed sync puts the
/// previous project back.
#[cfg(unix)]
#[test]
fn a_change_is_previewed_aside_then_applied_or_undone() {
    let temp = Temp::new("kisisel");
    let root = temp.0.join("python");
    let project = root.join("kisisel");
    std::fs::create_dir_all(&project).unwrap();
    let before = personal::pyproject("3.12.13", "linux");
    std::fs::write(project.join("pyproject.toml"), &before).unwrap();
    std::fs::write(project.join("uv.lock"), "version = 1\n").unwrap();
    let log = temp.0.join("kayit.txt");
    let fake = personal_uv(&temp.0.join("uv"), &log);
    let python = Interpreter {
        version: "3.12.13".into(),
        path: "/k/yorumlayici/cpython-3.12/bin/python3.12".into(),
    };
    // A requirement that is an option never reaches uv.
    assert!(
        personal::preview(
            &fake,
            &root,
            &python,
            &Change::Add(vec!["--index-url=x".into()])
        )
        .is_err()
    );
    assert!(!log.exists(), "uv was not run");
    let preview =
        personal::preview(&fake, &root, &python, &Change::Add(vec!["pandas".into()])).unwrap();
    assert_eq!(
        preview.added,
        [
            Package {
                name: "pandas".into(),
                version: "3.1.1".into(),
                direct: true
            },
            Package {
                name: "numpy".into(),
                version: "2.5.4".into(),
                direct: false
            },
        ]
    );
    assert_eq!(
        preview.changed,
        [("tzdata".into(), "2025.1".into(), "2026.6".into())]
    );
    assert!(preview.removed.is_empty() && !preview.is_empty());
    assert_eq!(preview.direct, ["pandas"]);
    let logged = std::fs::read_to_string(&log).unwrap();
    assert!(
        logged.contains("|add pandas --no-sync --no-config --no-python-downloads --system-certs --python /k/yorumlayici/cpython-3.12/bin/python3.12"),
        "{logged}"
    );
    assert_eq!(
        std::fs::read_to_string(project.join("pyproject.toml")).unwrap(),
        before,
        "the project is untouched by a preview"
    );
    // Agreed: the environment follows, with how far it is; KentOS's library goes back in.
    let wheel = sdk::Wheel {
        path: temp.0.join("kentos-0.1.0-cp310-abi3-linux_x86_64.whl"),
        version: "0.1.0".into(),
        sha256: "a".repeat(64),
    };
    let stop = std::sync::atomic::AtomicBool::new(false);
    let mut seen = Vec::new();
    personal::apply(&fake, &root, &python, &wheel, &stop, &mut |p| seen.push(p)).unwrap();
    assert!(
        std::fs::read_to_string(project.join("pyproject.toml"))
            .unwrap()
            .contains("pandas>=3.1.1")
    );
    assert_eq!(seen.last(), Some(&Progress::Unpacking), "{seen:?}");
    assert_eq!(
        std::fs::read_to_string(project.join(".kentos-sdk")).unwrap(),
        "0.1.0"
    );
    // Ready for the console once its Python is there too.
    assert_eq!(personal::ready(&root), None);
    std::fs::create_dir_all(project.join(".venv/bin")).unwrap();
    std::fs::write(project.join(".venv/bin/python"), b"").unwrap();
    assert_eq!(
        personal::ready(&root),
        Some(project.join(".venv/bin/python"))
    );
    let logged = std::fs::read_to_string(&log).unwrap();
    assert!(
        logged.contains("|sync --locked --no-config --no-python-downloads --system-certs --python")
    );
    assert!(logged.contains("|pip install --no-deps --offline --no-config --python"));
    assert!(
        !root.join("kisisel-onizleme").exists() && !project.join("pyproject.toml.onceki").exists()
    );
    // A sync that fails: the previous project comes back.
    let agreed = std::fs::read_to_string(project.join("pyproject.toml")).unwrap();
    std::fs::write(project.join("pyproject.toml"), &before).unwrap();
    personal::preview(&fake, &root, &python, &Change::Add(vec!["pandas".into()])).unwrap();
    std::fs::write(temp.0.join("uv").join("bozuk"), b"").unwrap();
    let failed = personal::apply(&fake, &root, &python, &wheel, &stop, &mut |_| {}).unwrap_err();
    assert!(
        matches!(
            &failed,
            InstallError::Failed {
                cause: Cause::Network,
                ..
            }
        ),
        "{failed:?}"
    );
    assert_eq!(
        std::fs::read_to_string(project.join("pyproject.toml")).unwrap(),
        before,
        "the previous project is back"
    );
    assert_ne!(agreed, before);
    assert!(std::fs::read_to_string(&log).unwrap().contains(
        "|sync --locked --no-config --no-python-downloads --system-certs --offline --python"
    ));
}

/// Updates are uv's (`pip list --outdated`), KentOS's library never among them; a pinned package asked for is raised
/// to the newer version (`add "pandas>=3.1.1"`: re-adding a bare name keeps a pin, measured), a dependency upgraded in
/// the lock; the tree is uv's drawing.
#[cfg(unix)]
#[test]
fn updates_and_the_tree_come_from_uv() {
    use crate::personal::Outdated;
    let temp = Temp::new("guncelle");
    let root = temp.0.join("python");
    let project = root.join("kisisel");
    std::fs::create_dir_all(&project).unwrap();
    std::fs::write(
        project.join("pyproject.toml"),
        personal::pyproject("3.12.13", "linux"),
    )
    .unwrap();
    std::fs::write(project.join("uv.lock"), "version = 1\n").unwrap();
    let log = temp.0.join("kayit.txt");
    let fake = logging_uv(
        &temp.0.join("uv"),
        &log,
        r#"case "$1 $2" in
  "pip list") printf '%s' '[{"name":"Pandas","version":"2.3.3","latest_version":"3.1.1","latest_filetype":"wheel"},{"name":"kentos","version":"0.1.0","latest_version":"9.9.9"},{"name":"six","version":"1.0","latest_version":null}]' ;;
  "tree --frozen") printf 'kentos-kisisel v0\n└── pandas v2.3.3\n' ;;
esac"#,
    );
    assert_eq!(
        personal::outdated(&fake, &root).unwrap(),
        [Outdated {
            name: "pandas".into(),
            version: "2.3.3".into(),
            latest: "3.1.1".into()
        }]
    );
    assert!(std::fs::read_to_string(&log).unwrap().contains(
        "|pip list --outdated --exclude kentos --format json --no-config --system-certs --python"
    ));
    assert_eq!(
        personal::tree(&fake, &root).unwrap(),
        "kentos-kisisel v0\n└── pandas v2.3.3\n"
    );
    let python = Interpreter {
        version: "3.12.13".into(),
        path: "/k/py".into(),
    };
    personal::preview(
        &fake,
        &root,
        &python,
        &Change::Upgrade {
            raise: vec![("pandas".into(), "3.1.1".into())],
            others: vec!["tzdata".into()],
        },
    )
    .unwrap();
    let logged = std::fs::read_to_string(&log).unwrap();
    assert!(
        logged.contains("|add pandas>=3.1.1 --no-sync --no-config"),
        "{logged}"
    );
    assert!(
        logged.contains("|lock --upgrade-package tzdata --no-config"),
        "{logged}"
    );
    // An option dressed as a package name never reaches uv.
    assert!(
        personal::preview(
            &fake,
            &root,
            &python,
            &Change::Upgrade {
                raise: Vec::new(),
                others: vec!["--upgrade".into()],
            },
        )
        .is_err()
    );
}

/// A fake uv for an extension's environment: `sync --dry-run` says what it would install; `sync` makes the venv's
/// Python (a script that answers the import) and says its downloads, or fails when told to (`bozuk` beside it);
/// `pip install`, `pip check` succeed; `pip list` lists shapely.
#[cfg(unix)]
fn extension_uv(dir: &Path, log: &Path) -> PathBuf {
    let fails = dir.join("bozuk");
    logging_uv(
        dir,
        log,
        &format!(
            r#"case "$1 $2" in
  "sync --locked")
    case "$*" in
      *--dry-run*) echo 'Would install 2 packages' >&2; echo ' + numpy==2.5.4' >&2; echo ' + shapely==2.2.0' >&2; exit 0 ;;
    esac
    if [ -e '{fails}' ]; then echo 'error: Failed to download shapely' >&2; echo '  Caused by: dns error' >&2; exit 2; fi
    mkdir -p "$UV_PROJECT_ENVIRONMENT/bin"
    printf '#!/bin/sh\nexit 0\n' > "$UV_PROJECT_ENVIRONMENT/bin/python"
    chmod +x "$UV_PROJECT_ENVIRONMENT/bin/python"
    echo 'Downloading shapely (1.0MiB)' >&2; echo ' Downloaded shapely' >&2; echo 'Prepared 2 packages in 1s' >&2 ;;
  "pip list") printf '%s' '[{{"name":"numpy","version":"2.5.4"}},{{"name":"Shapely","version":"2.2.0"}}]' ;;
esac"#,
            fails = fails.display()
        ),
    )
}

/// An extension version's environment is its own: keyed by the extension, its version, its lock, KentOS's Python,
/// this system and the features chosen, so two extensions asking for different versions of a package never share one
/// (P05); previewed from the lock; made, checked and marked ready last; a half-made one goes; ready is files on disk,
/// used again without uv (P07); unused ones are cleaned, nothing else.
#[cfg(unix)]
#[test]
fn an_extensions_environment_is_its_own_previewed_made_and_used_offline() {
    let temp = Temp::new("eklenti-ortami");
    let root = temp.0.join("python");
    let package = temp.0.join("paket");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(
        package.join("pyproject.toml"),
        "[project]\nname = \"mebis\"\n",
    )
    .unwrap();
    std::fs::write(
        package.join("uv.lock"),
        "version = 1\n[[package]]\nname = \"shapely\"\n",
    )
    .unwrap();
    let python = Interpreter {
        version: "3.12.13".into(),
        path: "/k/py/bin/python3.12".into(),
    };
    let key = extension::key_for(
        &package,
        "uv.lock",
        "tr.ornek.mebis",
        "1.0.0",
        &python,
        &["charts".into(), "charts".into(), " ".into()],
    )
    .unwrap();
    assert_eq!(key.extras, ["charts"]);
    assert_eq!(key.lock.len(), 64);
    assert_eq!(key.platform, extension::platform());
    assert_eq!(key.hash().len(), 16);
    // Another extension, or the same with another lock, another environment.
    let other = extension::Key {
        id: "tr.ornek.imar".into(),
        ..key.clone()
    };
    assert_ne!(key.hash(), other.hash());
    assert_ne!(
        key.hash(),
        extension::Key {
            lock: "0".repeat(64),
            ..key.clone()
        }
        .hash()
    );
    assert_ne!(
        extension::place(&root, &key),
        extension::place(&root, &other)
    );
    let log = temp.0.join("kayit.txt");
    let fake = extension_uv(&temp.0.join("uv"), &log);
    // Previewed: what it would install here, nothing written.
    let would = extension::preview(
        &fake,
        &root,
        &python,
        &package,
        ("pyproject.toml", "uv.lock"),
        &key,
    )
    .unwrap();
    assert_eq!(
        would,
        [
            extension::Listed {
                name: "numpy".into(),
                version: "2.5.4".into()
            },
            extension::Listed {
                name: "shapely".into(),
                version: "2.2.0".into()
            }
        ]
    );
    assert!(extension::ready(&root, &key).is_none());
    let logged = std::fs::read_to_string(&log).unwrap();
    assert!(
        logged.contains("|sync --locked --no-dev --no-install-project --no-config --no-python-downloads --system-certs --dry-run --extra charts --python /k/py/bin/python3.12"),
        "{logged}"
    );
    // Made: synced, the library put in, checked, then the record.
    let wheel = sdk::Wheel {
        path: temp.0.join("kentos-0.1.0-cp310-abi3-linux_x86_64.whl"),
        version: "0.1.0".into(),
        sha256: "a".repeat(64),
    };
    let stop = std::sync::atomic::AtomicBool::new(false);
    let mut seen = Vec::new();
    let made = extension::prepare(
        &fake,
        &root,
        &python,
        &wheel,
        &package,
        ("pyproject.toml", "uv.lock"),
        &key,
        &stop,
        &mut |p| seen.push(p),
    )
    .unwrap();
    assert_eq!(made.packages.len(), 2);
    assert_eq!(made.packages[1].name, "shapely", "normalised");
    assert_eq!(seen.last(), Some(&Progress::Unpacking), "{seen:?}");
    let dir = extension::place(&root, &key);
    assert!(dir.join(extension::RECORD).is_file());
    assert_eq!(made.python, dir.join(".venv/bin/python"));
    assert!(
        dir.join("proje/uv.lock").is_file(),
        "the project's files copied, the package left as it is"
    );
    let logged = std::fs::read_to_string(&log).unwrap();
    for asked in [
        "|sync --locked --no-dev --no-install-project --no-config --no-python-downloads --system-certs --extra charts --python",
        "|pip install --no-deps --offline --no-config --python",
        "|pip check --no-config --python",
        "|pip list --format json --exclude kentos --no-config --python",
    ] {
        assert!(logged.contains(asked), "{asked}\n{logged}");
    }
    // Ready is files on disk: used again without uv (offline, P07).
    assert_eq!(extension::ready(&root, &key), Some(made.clone()));
    let calls = std::fs::read_to_string(&log).unwrap().lines().count();
    let again = extension::prepare(
        &fake,
        &root,
        &python,
        &wheel,
        &package,
        ("pyproject.toml", "uv.lock"),
        &key,
        &stop,
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(again, made);
    assert_eq!(
        std::fs::read_to_string(&log).unwrap().lines().count(),
        calls,
        "uv not asked"
    );
    // A sync that fails leaves nothing half-made.
    std::fs::write(temp.0.join("uv").join("bozuk"), b"").unwrap();
    let failed = extension::prepare(
        &fake,
        &root,
        &python,
        &wheel,
        &package,
        ("pyproject.toml", "uv.lock"),
        &other,
        &stop,
        &mut |_| {},
    )
    .unwrap_err();
    assert!(
        matches!(
            &failed,
            InstallError::Failed {
                cause: Cause::Network,
                ..
            }
        ),
        "{failed:?}"
    );
    assert!(
        !extension::place(&root, &other).exists(),
        "no half-made environment"
    );
    // A lock that names `kentos` is refused before anything runs.
    std::fs::write(package.join("uv.lock"), "[[package]]\nname = \"kentos\"\n").unwrap();
    assert!(extension::shadows_library(
        "[[package]]\nname = \"Kentos\"\n"
    ));
    assert!(!extension::shadows_library(
        "[[package]]\nname = \"kentos-plus\"\n"
    ));
    assert!(
        extension::preview(
            &fake,
            &root,
            &python,
            &package,
            ("pyproject.toml", "uv.lock"),
            &other
        )
        .unwrap_err()
        .contains("KentOS'un kendi kitaplığının")
    );
    // Unused environments are cleaned; folders not named as KentOS names them stay.
    std::fs::create_dir_all(extension::folder(&root).join("0123456789abcdef")).unwrap();
    std::fs::create_dir_all(extension::folder(&root).join("kullanicinin")).unwrap();
    assert_eq!(
        extension::unused(&root, std::slice::from_ref(&key)),
        [extension::folder(&root).join("0123456789abcdef")]
    );
    assert_eq!(extension::clean(&root, std::slice::from_ref(&key)), 1);
    assert!(
        extension::ready(&root, &key).is_some(),
        "the kept one stays"
    );
    assert!(extension::folder(&root).join("kullanicinin").exists());
    // Another KentOS's library: put in offline before a run, the record says which; the same one: nothing run.
    let ready = extension::ready(&root, &key).unwrap();
    assert_eq!(ready.sdk, "a".repeat(64));
    let calls = std::fs::read_to_string(&log).unwrap().lines().count();
    assert_eq!(
        extension::refresh_library(&fake, &root, &ready, &wheel).unwrap(),
        ready
    );
    assert_eq!(
        std::fs::read_to_string(&log).unwrap().lines().count(),
        calls,
        "the same build: uv not asked"
    );
    let newer = sdk::Wheel {
        sha256: "b".repeat(64),
        ..wheel.clone()
    };
    let fresh = extension::refresh_library(&fake, &root, &ready, &newer).unwrap();
    assert_eq!(fresh.sdk, "b".repeat(64));
    assert_eq!(
        extension::ready(&root, &key).unwrap().sdk,
        "b".repeat(64),
        "the record follows"
    );
    assert!(std::fs::read_to_string(&log).unwrap().contains(
        "|pip install --no-deps --offline --no-config --reinstall-package kentos --python"
    ));
    extension::remove(&root, &key).unwrap();
    assert!(extension::ready(&root, &key).is_none());
}
