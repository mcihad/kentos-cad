//! Eklentiye ekle (docs/adr/8057 §95): a package of the personal environment added to an extension developed in a
//! working folder, written and locked by uv with the project's own settings, the extension loaded again with it; one
//! already among its dependencies said and not changed.

use kentos_python_env::personal::{Package, Personal};
use kentos_python_env::python::Interpreter;
use kentos_python_env::uv::UvStatus;

use super::*;
use crate::python_personal::PersonalEdit;
use crate::python_setup::PythonEdit;

const VITRIN: &str = "tr.ornek.vitrin";

/// A fake uv, its words logged: `add` writes the requirement into the copy's project and a package more into its
/// lock, as uv would.
#[cfg(unix)]
fn fake_uv(dir: &Path, log: &Path) -> PathBuf {
    use std::os::unix::fs::PermissionsExt;
    std::fs::create_dir_all(dir).unwrap();
    let file = dir.join("uv");
    std::fs::write(
        &file,
        format!(
            r#"#!/bin/sh
printf '%s\n' "$*" >> '{log}'
for a in "$@"; do last="$a"; done
case "$1" in
  add)
    printf '# eklendi: %s\n' "$last" >> pyproject.toml
    printf '\n[[package]]\nname = "tasli"\nversion = "0.3"\nsource = {{ registry = "https://pypi.org/simple" }}\n' >> uv.lock ;;
esac
"#,
            log = log.display(),
        ),
    )
    .unwrap();
    std::fs::set_permissions(&file, std::fs::Permissions::from_mode(0o755)).unwrap();
    file
}

#[cfg(unix)]
#[test]
fn a_personal_package_is_added_to_an_extension_being_developed() {
    let base = temp("eklentiye-ekle");
    let root = base.join("eklentiler");
    std::fs::create_dir_all(&root).unwrap();
    let mut gis = app(&root, Some(Module::Gis));
    // Vitrin's source in a workshop folder, installed from it.
    let folder = base.join("atolye").join(VITRIN);
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/eklenti-vitrini/eklenti");
    for (path, bytes) in kentos_extension_host::store::read_files(&source).unwrap() {
        if path.contains("__pycache__") {
            continue;
        }
        let to = folder.join(&path);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::write(to, bytes).unwrap();
    }
    let files = kentos_extension_host::store::read_files(&folder).unwrap();
    let package = base.join("vitrin.kek");
    std::fs::write(
        &package,
        kentos_extensions::pack(&files, &Limits::default())
            .unwrap()
            .bytes,
    )
    .unwrap();
    gis.review_workshop_package(&package, &folder, "Asistan");
    let _ = gis.extensions_event(Event::Install);
    let developed: Vec<String> = gis
        .developed_extensions()
        .into_iter()
        .map(|d| d.id)
        .collect();
    assert_eq!(developed, [VITRIN]);
    // KentOS's Python ready, uv a fake one; the personal environment holds numpy and tasli.
    let python_root = base.join("python");
    gis.python_setup.root = Some(python_root.clone());
    let log = base.join("uv.log");
    let uv = fake_uv(&base.join("araclar"), &log);
    let _ = gis.python_edit(PythonEdit::Enable(true));
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::Found {
        path: uv,
        version: "0.11.26".into(),
        bundled: true,
    }));
    let python = Interpreter {
        version: "3.12.13".into(),
        path: python_root.join("bin/python3.12"),
    };
    let _ = gis.python_edit(PythonEdit::Found(Ok((Some(python), 0))));
    let package = |name: &str, version: &str| Package {
        name: name.into(),
        version: version.into(),
        direct: true,
    };
    let personal = Personal {
        python: python_root.join("kisisel/.venv/bin/python"),
        size: 0,
        sdk: Some(env!("CARGO_PKG_VERSION").into()),
        packages: vec![package("numpy", "2.5.4"), package("tasli", "0.3")],
    };
    let edit = |app: &mut App, e: PersonalEdit| {
        let task = app.python_edit(PythonEdit::Personal(e));
        crate::files_testing::drive(app, task);
    };
    edit(&mut gis, PersonalEdit::Found(Ok((Some(personal), 0))));
    // numpy is one of Vitrin's: said, nothing run.
    edit(
        &mut gis,
        PersonalEdit::AddTo("numpy".into(), "2.5.4".into()),
    );
    edit(&mut gis, PersonalEdit::AddToConfirm);
    let note = gis.python_setup.personal.note.clone().unwrap_or_default();
    assert!(
        note.contains("zaten “Eklenti Vitrini” eklentisinin bağımlılığı (kilitte 2.5.4)"),
        "{note}"
    );
    assert!(!log.exists(), "uv not run");
    // tasli is not: the chooser drawn, then added, locked and the extension loaded again.
    edit(&mut gis, PersonalEdit::AddTo("tasli".into(), "0.3".into()));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::AddToConfirm);
    let said = std::fs::read_to_string(&log).unwrap();
    assert!(
        said.starts_with("add --no-sync") && said.contains("tasli>=0.3,<0.4"),
        "{said}"
    );
    assert!(!said.contains("--no-config"), "{said}");
    assert!(
        std::fs::read_to_string(folder.join("pyproject.toml"))
            .unwrap()
            .ends_with("# eklendi: tasli>=0.3,<0.4\n")
    );
    assert_eq!(gis.python_setup.personal.add_to, None);
    assert_eq!(gis.python_setup.personal.note, None);
    let toast = gis
        .toasts
        .iter()
        .map(|(_, t, _)| t)
        .find(|t| t.title() == "Eklenti Vitrini: bağımlılık eklendi")
        .expect("the notice");
    let body = toast.description().unwrap_or_default();
    assert!(
        body.contains("“tasli>=0.3,<0.4”")
            && body.contains("yeni: tasli 0.3")
            && body.contains("yeniden yüklendi"),
        "{body}"
    );
    let _ = std::fs::remove_dir_all(&base);
}
