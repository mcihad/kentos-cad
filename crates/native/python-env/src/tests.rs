//! ADIM 18A's first slice (plan/eklenti.md): the manifest read, uv found where a desktop session would miss it, its
//! version checked, and uv run without a shell, within its time and output, with none of the environment's uv
//! settings. A fake uv (a small shell script) stands in for uv: nothing touches the user's own uv, Python or network.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};
#[cfg(unix)]
use std::time::Duration;

use crate::compat::{self, Compat, Platform};
use crate::python::{self, Cause, InstallError, Interpreter, Progress};
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
