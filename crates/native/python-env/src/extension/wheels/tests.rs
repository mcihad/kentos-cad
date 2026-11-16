use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;

use sha2::{Digest, Sha256};

use super::*;
use crate::extension::{self, Listed};
use crate::python::Cause;

struct Temp(PathBuf);

impl Temp {
    fn new(name: &str) -> Temp {
        let dir =
            std::env::temp_dir().join(format!("kentos-tekerlek-{name}-{}", std::process::id()));
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

/// A fake uv for a server's environment, its words logged: `export` writes the lock's hashed list where it is told;
/// `venv` makes the venv's Python (a script that answers the import); `pip install` of the list fails as uv does for a
/// dependency with only a source distribution when told to (`bozuk` beside it); `pip list` lists the wheel.
#[cfg(unix)]
fn server_uv(dir: &Path, log: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).unwrap();
    let fails = dir.join("bozuk");
    let file = dir.join("uv");
    std::fs::write(
        &file,
        format!(
            r#"#!/bin/sh
printf '%s\n' "$*" >> '{log}'
case "$1 $2" in
  "export --locked")
    prev=""; out=""
    for a in "$@"; do if [ "$prev" = "--output-file" ]; then out="$a"; fi; prev="$a"; done
    printf 'kucuk==1.0 \\\n    --hash=sha256:{hash}\n' > "$out" ;;
  "venv --no-config")
    for a in "$@"; do last="$a"; done
    mkdir -p "$last/bin"; printf '#!/bin/sh\nexit 0\n' > "$last/bin/python"; chmod +x "$last/bin/python" ;;
  "pip install")
    case "$*" in
      *" -r "*)
        if [ -e '{fails}' ]; then
          printf '  x No solution found when resolving dependencies:\n' >&2
          printf '  -> Because tasli==1.0 has no usable wheels and you require tasli==1.0, we\n' >&2
          printf '      can conclude that your requirements are unsatisfiable.\n' >&2
          exit 1
        fi ;;
    esac ;;
  "pip list") printf '%s' '[{{"name":"kucuk","version":"1.0"}}]' ;;
esac
"#,
            log = log.display(),
            hash = "f".repeat(64),
            fails = fails.display(),
        ),
    )
    .unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    file
}

fn package(dir: &Path) -> PathBuf {
    let package = dir.join("paket");
    std::fs::create_dir_all(&package).unwrap();
    std::fs::write(
        package.join("pyproject.toml"),
        "[project]\nname = \"denetim\"\ndependencies = [\"kucuk==1.0\"]\n",
    )
    .unwrap();
    std::fs::write(
        package.join("uv.lock"),
        "version = 1\n[[package]]\nname = \"kucuk\"\n",
    )
    .unwrap();
    package
}

/// A server's environment: the desktop's key and folder for an empty digest, the release's own for its digest; the
/// lock's list exported offline, a venv of KentOS's Python, the list's wheels only, each against its hash, offline
/// from the folder (or from the index where one is allowed), then the library, the check and the record. Ready again:
/// nothing run. A dependency without a wheel: said in Turkish, the package named, nothing half-made left.
#[cfg(unix)]
#[test]
fn a_server_s_environment_takes_the_lock_s_wheels_only_each_against_its_hash() {
    let temp = Temp::new("ortam");
    let root = temp.0.join("python");
    let package = package(&temp.0);
    let python = Interpreter {
        version: "3.14.5".into(),
        path: "/usr/bin/python3.14".into(),
    };
    let desktop = extension::key_for(
        &package,
        "uv.lock",
        "tr.ornek.denetim",
        "1.0.0",
        &python,
        &[],
    )
    .unwrap();
    // The desktop's key and folder are as they were: the digest is a server's only.
    let before: String = Sha256::digest(
        format!(
            "tr.ornek.denetim\n1.0.0\n{}\n3.14.5\n{}\n",
            desktop.lock,
            extension::platform()
        )
        .as_bytes(),
    )
    .iter()
    .take(8)
    .map(|b| format!("{b:02x}"))
    .collect();
    assert_eq!(desktop.hash(), before);
    assert!(!serde_json::to_string(&desktop).unwrap().contains("digest"));
    let key = Key {
        digest: format!("sha256:{}", "1".repeat(64)),
        ..desktop.clone()
    };
    assert_ne!(key.hash(), desktop.hash(), "the release's own");
    let log = temp.0.join("kayit.txt");
    let fake = server_uv(&temp.0.join("uv"), &log);
    let wheel = Wheel {
        path: temp.0.join("kentos-0.1.0-cp310-abi3-linux_x86_64.whl"),
        version: "0.1.0".into(),
        sha256: "a".repeat(64),
    };
    let folder = temp.0.join("tekerlekler");
    let offline = Wheels {
        folder: Some(folder.clone()),
        index: None,
    };
    let stop = AtomicBool::new(false);
    let mut seen = Vec::new();
    let made = prepare_wheels(
        &fake,
        &root,
        &python,
        &wheel,
        &package,
        ("pyproject.toml", "uv.lock"),
        &key,
        &offline,
        &stop,
        &mut |p| seen.push(p),
    )
    .unwrap();
    let dir = extension::place(&root, &key);
    assert_eq!(made.python, dir.join(".venv/bin/python"));
    assert_eq!(made.key, key);
    assert_eq!(
        made.packages,
        [Listed {
            name: "kucuk".into(),
            version: "1.0".into()
        }]
    );
    assert_eq!(extension::ready(&root, &key), Some(made.clone()));
    assert_eq!(seen, [Progress::Starting, Progress::Unpacking]);
    let logged = std::fs::read_to_string(&log).unwrap();
    let list = dir.join("proje").join(REQUIREMENTS);
    for asked in [
        format!(
            "export --locked --no-dev --no-emit-project --format requirements-txt --no-config --offline \
             --output-file {}",
            list.display()
        ),
        format!(
            "venv --no-config --no-python-downloads --offline --python /usr/bin/python3.14 {}",
            dir.join(".venv").display()
        ),
        format!(
            "pip install --no-config --require-hashes --no-build --no-deps --system-certs --offline --no-index \
             --find-links {} --python {} -r {}",
            folder.display(),
            made.python.display(),
            list.display()
        ),
        "pip install --no-deps --offline --no-config --python".to_owned(),
        "pip check --no-config --python".to_owned(),
    ] {
        assert!(logged.contains(&asked), "{asked}\n{logged}");
    }
    // Ready: used as it is, uv not asked.
    let calls = logged.lines().count();
    let again = prepare_wheels(
        &fake,
        &root,
        &python,
        &wheel,
        &package,
        ("pyproject.toml", "uv.lock"),
        &key,
        &offline,
        &stop,
        &mut |_| {},
    )
    .unwrap();
    assert_eq!(again, made);
    assert_eq!(
        std::fs::read_to_string(&log).unwrap().lines().count(),
        calls
    );
    // An index, where the administrator allows the network: online for this step alone, the folder still looked in.
    let online = Wheels {
        folder: Some(folder.clone()),
        index: Some("https://paketler.kurum.example/simple".into()),
    };
    let args: Vec<String> = install_args(Path::new("/p/bin/python"), Path::new("/l.txt"), &online)
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert!(
        args.windows(2)
            .any(|w| w == ["--index-url", "https://paketler.kurum.example/simple"])
    );
    assert!(!args.iter().any(|a| a == "--offline" || a == "--no-index"));
    // A dependency with only a source distribution: refused, named, nothing half-made.
    std::fs::write(temp.0.join("uv").join("bozuk"), b"").unwrap();
    let other = Key {
        version: "1.1.0".into(),
        ..key.clone()
    };
    let failed = prepare_wheels(
        &fake,
        &root,
        &python,
        &wheel,
        &package,
        ("pyproject.toml", "uv.lock"),
        &other,
        &offline,
        &stop,
        &mut |_| {},
    )
    .unwrap_err();
    let InstallError::Failed {
        cause: Cause::Other,
        said,
    } = &failed
    else {
        panic!("{failed:?}");
    };
    assert!(
        said.contains("“tasli” 1.0") && said.contains("tekerleği yok"),
        "{said}"
    );
    assert!(
        !extension::place(&root, &other).exists(),
        "no half-made environment"
    );
    // Nowhere to take wheels from: said before anything runs.
    let nowhere = prepare_wheels(
        &fake,
        &root,
        &python,
        &wheel,
        &package,
        ("pyproject.toml", "uv.lock"),
        &other,
        &Wheels::default(),
        &stop,
        &mut |_| {},
    )
    .unwrap_err();
    assert!(
        format!("{nowhere:?}").contains("Tekerleklerin nereden"),
        "{nowhere:?}"
    );
    // A version's environments removed, whatever their key.
    assert_eq!(
        extension::remove_version(&root, "tr.ornek.denetim", "1.0.0"),
        std::slice::from_ref(&dir)
    );
    assert!(extension::ready(&root, &key).is_none());
}

/// uv's words (0.11), as a server's administrator reads them: the package named, what to do.
#[test]
fn uv_s_words_for_wheels_become_turkish_with_the_package_named() {
    let folder = Wheels {
        folder: Some(PathBuf::from("/srv/tekerlekler")),
        index: None,
    };
    let said = |text: &str| wheel_words(text, &folder);
    let source_only = said(
        "  × No solution found when resolving dependencies:\n  ╰─▶ Because tasli==1.0 has no usable wheels and you \
         require tasli==1.0, we\n      can conclude that your requirements are unsatisfiable.\n\nhint: Wheels are \
         required for `tasli` because building from source is disabled for all packages (i.e., with `--no-build`)",
    );
    assert!(
        source_only.starts_with("“tasli” 1.0: bu sunucunun sistemi"),
        "{source_only}"
    );
    // A universal lock's requirement, its markers in braces across lines.
    let marked = said(
        "      (platform_machine == 'AMD64' and sys_platform == 'win32')}==1.0 has\n      no usable wheels and you \
         require tasli{(platform_machine == 'arm64'\n      and sys_platform == 'darwin') or (platform_machine == \
         'x86_64'\n      and sys_platform == 'linux')}==1.0, we can conclude that your requirements\n      are \
         unsatisfiable.\n\nhint: Wheels are required for `tasli` because building from source is disabled",
    );
    assert!(marked.starts_with("“tasli”"), "{marked}");
    let whole = said(
        "  ╰─▶ Because tasli{(platform_machine == 'arm64' and sys_platform == 'darwin') or (platform_machine == \
         'x86_64' and sys_platform == 'linux')}==1.0 has no usable wheels and you require tasli==1.0.",
    );
    assert!(
        whole.starts_with("“tasli” 1.0: bu sunucunun sistemi"),
        "{whole}"
    );
    let other_python = said(
        "  ╰─▶ Because only kucuk==1.0 is available and kucuk==1.0 has no wheels with a matching Python ABI tag (e.g., \
         `cp314`), we can conclude that your requirements are unsatisfiable.",
    );
    assert!(
        other_python.starts_with("“kucuk” 1.0: bu sunucunun sistemi"),
        "{other_python}"
    );
    let missing = said(
        "  ╰─▶ Because yok-paket was not found in the provided package locations and\n      you require \
         yok-paket==2.0, we can conclude that your requirements are\n      unsatisfiable.",
    );
    assert_eq!(
        missing,
        "“yok-paket” tekerlek klasöründe (/srv/tekerlekler) yok: kilitteki sürümün tekerleğini oraya koyun."
    );
    let other_file = said(
        "  × Failed to download `kucuk==1.0`\n  ╰─▶ Hash mismatch for `kucuk==1.0`\n\n      Expected:",
    );
    assert!(
        other_file.starts_with("“kucuk” 1.0: bulunan dosya kilitteki özetle"),
        "{other_file}"
    );
    let no_hash = said(
        "error: In `--require-hashes` mode, all requirements must have a hash, but none were provided for: kucuk==1.0",
    );
    assert!(
        no_hash.starts_with("Eklentinin kilidi “kucuk” 1.0 için özet"),
        "{no_hash}"
    );
    let stale = said(
        "error: The lockfile at `uv.lock` needs to be updated, but `--locked` was provided. To update the lockfile, \
         run `uv lock`.",
    );
    assert!(
        stale.starts_with("Eklentinin kilidi projesiyle uyuşmuyor"),
        "{stale}"
    );
}
