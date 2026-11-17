use std::path::PathBuf;

use super::*;

/// A folder of its own under the system's temporary folder, gone after the test.
struct Temp(PathBuf);

impl Temp {
    fn new(name: &str) -> Temp {
        let dir =
            std::env::temp_dir().join(format!("kentos-bagimlilik-{name}-{}", std::process::id()));
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

/// A fake uv: its words and the files it was given logged; `add` writes the requirement into the copy's project and
/// a new lock, or, with `bozuk` beside it, spoils the copy's project and fails as uv does when nothing resolves.
#[cfg(unix)]
fn fake_uv(dir: &Path, log: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    let fails = dir.join("bozuk");
    let file = dir.join("uv");
    std::fs::write(
        &file,
        format!(
            r#"#!/bin/sh
printf '%s\n' "$*" >> '{log}'
ls >> '{log}'
for a in "$@"; do last="$a"; done
if [ -e '{fails}' ]; then
  printf 'bozuldu\n' >> pyproject.toml
  printf '  x No solution found when resolving dependencies for split (python_full_version >= '"'"'3.13'"'"'):\n' >&2
  printf '  -> Because only numpy<2.5 is available and deneme depends on numpy>=2.5.4, we can conclude that\n' >&2
  printf '     your requirements are unsatisfiable.\n' >&2
  printf 'error: No solution found when resolving dependencies\n' >&2
  exit 1
fi
printf '# eklendi: %s\n' "$last" >> pyproject.toml
printf 'version = 1\n# yeni kilit\n' > uv.lock
"#,
            log = log.display(),
            fails = fails.display(),
        ),
    )
    .unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    file
}

/// The requirement is the tried version and its series; uv adds it on a copy, the project's own settings holding (no
/// `--no-config`), and the project's files are replaced only when the lock resolved.
#[cfg(unix)]
#[test]
fn a_package_is_added_to_an_extension_s_project_only_when_it_locks() {
    assert_eq!(requirement_for("numpy", "2.5.4"), "numpy>=2.5.4,<3");
    assert_eq!(requirement_for("fiona", "0.13.1"), "fiona>=0.13.1,<0.14");
    assert_eq!(requirement_for("tzdata", "2025.2"), "tzdata>=2025.2,<2026");
    assert_eq!(requirement_for("x", "1.0rc1"), "x>=1.0rc1,<2");
    assert_eq!(requirement_for("x", "dev"), "x>=dev");
    let temp = Temp::new("ekle");
    let (root, project, tools) = (
        temp.0.join("python"),
        temp.0.join("eklenti"),
        temp.0.join("araclar"),
    );
    for dir in [&root, &project, &tools] {
        std::fs::create_dir_all(dir).unwrap();
    }
    let log = temp.0.join("uv.log");
    let uv_path = fake_uv(&tools, &log);
    let pyproject = "[project]\nname = \"deneme\"\nversion = \"0.1.0\"\ndependencies = []\n";
    std::fs::write(project.join("pyproject.toml"), pyproject).unwrap();
    std::fs::write(project.join("uv.lock"), "version = 1\n# eski kilit\n").unwrap();
    std::fs::write(project.join("uv.toml"), "exclude-newer = \"7 days\"\n").unwrap();
    let interpreter = Interpreter {
        version: "3.12.13".to_owned(),
        path: PathBuf::from("/usr/bin/python3.12"),
    };
    add_dependency(&uv_path, &root, &interpreter, &project, "numpy>=2.5.4,<3").unwrap();
    let said = std::fs::read_to_string(&log).unwrap();
    assert!(
        said.contains(concat!(
            "add --no-sync --no-python-downloads --system-certs --python /usr/bin/python3.12 ",
            "numpy>=2.5.4,<3"
        )),
        "{said}"
    );
    assert!(
        !said.contains("--no-config"),
        "the project's settings hold: {said}"
    );
    assert!(
        said.contains("uv.toml"),
        "its settings file copied too: {said}"
    );
    assert!(
        std::fs::read_to_string(project.join("pyproject.toml"))
            .unwrap()
            .ends_with("# eklendi: numpy>=2.5.4,<3\n")
    );
    assert_eq!(
        std::fs::read_to_string(project.join("uv.lock")).unwrap(),
        "version = 1\n# yeni kilit\n"
    );
    assert!(!root.join(STAGING).exists(), "the copy is gone");
    // Nothing resolves: the project stays as it was, and why is said.
    std::fs::write(tools.join("bozuk"), "").unwrap();
    let before = std::fs::read_to_string(project.join("pyproject.toml")).unwrap();
    let why =
        add_dependency(&uv_path, &root, &interpreter, &project, "numpy>=2.5.4,<3").unwrap_err();
    assert!(
        why.starts_with("Bu sürüm eklentinin Python aralığı ve sistemleriyle")
            && why.contains("Eklentinin dosyaları değişmedi"),
        "{why}"
    );
    assert_eq!(
        std::fs::read_to_string(project.join("pyproject.toml")).unwrap(),
        before
    );
    assert_eq!(
        std::fs::read_to_string(project.join("uv.lock")).unwrap(),
        "version = 1\n# yeni kilit\n"
    );
    // A folder without a project is said before uv runs.
    let empty = temp.0.join("bos");
    std::fs::create_dir_all(&empty).unwrap();
    assert!(
        add_dependency(&uv_path, &root, &interpreter, &empty, "numpy")
            .unwrap_err()
            .contains("pyproject.toml yok")
    );
}
