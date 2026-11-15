//! Eklentiler's Python (docs/adr/8057 §5–§7): the buttons' reasons by the user's Python, the personal and the
//! extension environments, a command's own process through the broker, its writes as one undo step, its bytecode.

use super::*;

/// Python ve eklentiler (plan/eklenti.md ADIM 18A, P01/P02): off by default, nothing looked for; an extension's Python
/// button says why it does not run: Python off, uv not ready, or the runner still to come; the choice is written at
/// once; turned off, what was found is dropped.
#[test]
fn an_extensions_python_buttons_say_why_by_the_users_python() {
    use crate::python_setup::{OwnPython, PythonEdit};
    use kentos_python_env::python::{Cause, InstallError, Interpreter, Progress};
    use kentos_python_env::uv::UvStatus;
    use std::sync::Arc;

    let root = temp("python");
    let python_root = temp("python-ayar");
    let mut gis = app(&root, Some(Module::Gis));
    gis.python_setup.root = Some(python_root.clone());
    // Without its dependency on another example extension: MEBİS alone runs.
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] = serde_json::json!([]);
        }),
    );
    let why = |app: &App| -> String {
        let tab = app
            .extension_tabs()
            .find(|t| t.extension == "tr.ornek.mebis")
            .expect("MEBİS's tab");
        match &tab.panels[0].items[0].action {
            Action::Unavailable(why) => why.clone(),
            other => panic!("{other:?}"),
        }
    };
    assert!(!gis.python_setup.settings.enabled, "off by default");
    assert!(gis.python_setup.uv.is_none(), "nothing looked for");
    assert!(
        why(&gis).contains("Python kapalı: Uygulama ayarları › Python ve eklentiler"),
        "{}",
        why(&gis)
    );
    // Turned on: written at once, uv looked for in the background.
    let _ = gis.python_edit(PythonEdit::Enable(true));
    assert!(gis.python_setup.looking);
    assert!(
        kentos_python_env::settings::PythonSettings::load(&python_root).enabled,
        "written at once"
    );
    assert!(why(&gis).contains("uv aranıyor"));
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::NotFound {
        looked: Vec::new(),
        own: false,
    }));
    assert!(why(&gis).contains("uv hazır değil"), "{}", why(&gis));
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::Found {
        path: "/ev/.local/bin/uv".into(),
        version: "0.11.26".into(),
        bundled: false,
    }));
    // uv ready: KentOS's own Python looked for in the background.
    assert_eq!(gis.python_setup.python, Some(OwnPython::Looking));
    assert!(
        why(&gis).contains("KentOS'un Python'u aranıyor"),
        "{}",
        why(&gis)
    );
    let _ = gis.python_edit(PythonEdit::Found(Ok((None, 0))));
    assert_eq!(gis.python_setup.python, Some(OwnPython::Missing));
    assert!(
        why(&gis).contains("KentOS'un Python'u kurulu değil"),
        "{}",
        why(&gis)
    );
    let _ = gis.python_section();
    // Installed only when the user presses the button (the consent); Durdur stops it, and what ended it is said.
    assert!(
        gis.python_setup.installing.is_none(),
        "nothing installed unasked"
    );
    let _ = gis.python_edit(PythonEdit::Install);
    let stop = gis.python_setup.installing.clone().expect("installing");
    assert!(why(&gis).contains("kuruluyor"), "{}", why(&gis));
    let _ = gis.python_edit(PythonEdit::Install);
    assert!(
        Arc::ptr_eq(&stop, gis.python_setup.installing.as_ref().unwrap()),
        "one install at a time"
    );
    let _ = gis.python_section();
    // How far it is, as the bar says it: uv's size and the bytes so far, then unpacking.
    assert_eq!(gis.python_setup.progress, Some(Progress::Starting));
    let at = Progress::Downloading {
        done: 13_002_342,
        total: Some(34_183_578),
    };
    let _ = gis.python_edit(PythonEdit::Progressed(at));
    assert_eq!(gis.python_setup.progress, Some(at));
    assert_eq!(
        crate::python_setup::progress_words(at),
        "İndiriliyor: 12,4 MB / 32,6 MB · %38"
    );
    assert_eq!(
        crate::python_setup::progress_words(Progress::Downloading {
            done: 3_418_399,
            total: Some(34_183_578)
        }),
        "İndiriliyor: 3,3 MB / 32,6 MB · %10"
    );
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::Progressed(Progress::Unpacking));
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::StopInstall);
    assert!(stop.load(std::sync::atomic::Ordering::Relaxed), "Durdur");
    let toasts = gis.toasts.len();
    let _ = gis.python_edit(PythonEdit::Installed(Err(InstallError::Cancelled)));
    assert!(gis.python_setup.installing.is_none() && gis.python_setup.progress.is_none());
    // Said by a notice; nothing stays in the section, where the button waits again.
    assert_eq!(gis.toasts.len(), toasts + 1);
    assert!(
        gis.toasts
            .iter()
            .any(|(_, toast, _)| toast.title() == "Python kurulumu durduruldu")
    );
    assert!(gis.python_setup.note.is_none());
    let _ = gis.python_edit(PythonEdit::Install);
    let _ = gis.python_edit(PythonEdit::Installed(Err(InstallError::Failed {
        cause: Cause::Network,
        said: "dns error".into(),
    })));
    // A failure: in the section, and a notice that waits to be closed.
    assert!(
        matches!(&gis.python_setup.note, Some((_, said)) if said.contains("internete ulaşılamadı")),
        "{:?}",
        gis.python_setup.note
    );
    assert!(
        gis.toasts
            .iter()
            .any(|(_, toast, _)| toast.title() == "Python kurulamadı")
    );
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::Install);
    let _ = gis.python_edit(PythonEdit::Installed(Ok(())));
    assert!(gis.python_setup.note.is_none());
    assert!(
        gis.toasts
            .iter()
            .any(|(_, toast, _)| toast.title() == "Python 3.12 kuruldu")
    );
    assert_eq!(
        gis.python_setup.python,
        Some(OwnPython::Looking),
        "looked for again"
    );
    let installed = Interpreter {
        version: "3.12.13".into(),
        path: python_root.join("yorumlayici/cpython-3.12/bin/python3.12"),
    };
    let _ = gis.python_edit(PythonEdit::Found(Ok((
        Some(installed.clone()),
        102_640_000,
    ))));
    // KentOS's Python ready: the button now says what the extension's own environment needs (environment.rs).
    assert!(
        why(&gis).contains("Bu eklentinin Python ortamı hazırlanmadı"),
        "{}",
        why(&gis)
    );
    let _ = gis.python_section();
    // Removed from the same place: Kaldır asks first, Vazgeç keeps it, Evet, kaldır removes it and says the room freed.
    let _ = gis.python_edit(PythonEdit::Remove);
    assert!(gis.python_setup.confirm_remove, "asks first");
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::RemoveCancelled);
    assert!(!gis.python_setup.confirm_remove && !gis.python_setup.removing);
    let _ = gis.python_edit(PythonEdit::Remove);
    let _ = gis.python_edit(PythonEdit::RemoveConfirmed);
    assert!(gis.python_setup.removing && !gis.python_setup.confirm_remove);
    assert!(why(&gis).contains("kaldırılıyor"), "{}", why(&gis));
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::Install);
    assert!(gis.python_setup.installing.is_none(), "not while removing");
    let _ = gis.python_edit(PythonEdit::Removed(Ok(102_640_000)));
    assert!(!gis.python_setup.removing);
    assert!(
        gis.toasts
            .iter()
            .any(|(_, toast, _)| toast.title() == "Python kaldırıldı"),
        "a notice"
    );
    assert_eq!(crate::python_setup::size_words(102_640_000), "98 MB");
    let _ = gis.python_edit(PythonEdit::Found(Ok((None, 0))));
    assert_eq!(gis.python_setup.python, Some(OwnPython::Missing));
    // A removal that fails says why, in the section and by a notice.
    let _ = gis.python_edit(PythonEdit::Found(Ok((Some(installed), 102_640_000))));
    let _ = gis.python_edit(PythonEdit::Remove);
    let _ = gis.python_edit(PythonEdit::RemoveConfirmed);
    let _ = gis.python_edit(PythonEdit::Removed(Err(
        "Python kaldırılamadı: dosyaları kullanılıyor.".into(),
    )));
    assert!(
        matches!(&gis.python_setup.note, Some((_, said)) if said.contains("dosyaları kullanılıyor")),
        "{:?}",
        gis.python_setup.note
    );
    assert!(
        gis.toasts
            .iter()
            .any(|(_, toast, _)| toast.title() == "Python kaldırılamadı")
    );
    // A path by hand, kept.
    let _ = gis.python_edit(PythonEdit::UvTyped("/opt/araclar/uv".into()));
    let _ = gis.python_edit(PythonEdit::UvApply);
    assert_eq!(
        kentos_python_env::settings::PythonSettings::load(&python_root)
            .uv_path
            .as_deref(),
        Some("/opt/araclar/uv")
    );
    // Off again: what was found is dropped; nothing looked for; an install under way is stopped.
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::Found {
        path: "/ev/.local/bin/uv".into(),
        version: "0.11.26".into(),
        bundled: false,
    }));
    let _ = gis.python_edit(PythonEdit::Install);
    let stop = gis.python_setup.installing.clone().expect("installing");
    let _ = gis.python_edit(PythonEdit::Enable(false));
    assert!(
        stop.load(std::sync::atomic::Ordering::Relaxed),
        "stopped when turned off"
    );
    assert!(gis.python_setup.uv.is_none() && !gis.python_setup.looking);
    assert!(gis.python_setup.python.is_none());
    assert!(why(&gis).contains("Python kapalı"));
    // The settings window's section draws.
    gis.open_settings_at(crate::settings_sections::Section::Python);
    let _ = gis.settings_dialog();
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&python_root);
}

/// Kişisel ortam (plan/eklenti.md ADIM 18A, P04): on KentOS's Python, made on the user's press; a package typed as an
/// option never reaches uv; a change is previewed (asked for, dependencies) and made only when agreed, with how far it
/// is and Durdur; packages and the environment are removed from the same place, the environment asking first; each
/// outcome a notice (the owner, 13 November 2026).
#[test]
fn the_personal_environment_is_made_previewed_changed_and_removed() {
    use crate::python_personal::{Busy, OwnEnv, PersonalEdit};
    use crate::python_setup::{OwnPython, PythonEdit};
    use kentos_python_env::personal::{Change, Outdated, Package, Personal, Preview};
    use kentos_python_env::python::{InstallError, Interpreter, Progress};
    use kentos_python_env::uv::UvStatus;

    let root = temp("kisisel");
    let python_root = temp("kisisel-ayar");
    let mut gis = app(&root, Some(Module::Gis));
    gis.python_setup.root = Some(python_root.clone());
    let edit = |app: &mut App, e: PersonalEdit| {
        let _ = app.python_edit(PythonEdit::Personal(e));
    };
    let titles = |app: &App| -> Vec<String> {
        app.toasts
            .iter()
            .map(|(_, toast, _)| toast.title().to_owned())
            .collect()
    };
    let _ = gis.python_edit(PythonEdit::Enable(true));
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::Found {
        path: "/k/tools/uv".into(),
        version: "0.11.26".into(),
        bundled: true,
    }));
    assert!(
        gis.python_setup.personal.env.is_none(),
        "not before KentOS's Python"
    );
    let python = Interpreter {
        version: "3.12.13".into(),
        path: python_root.join("yorumlayici/cpython-3.12/bin/python3.12"),
    };
    let _ = gis.python_edit(PythonEdit::Found(Ok((Some(python), 97_885_768))));
    assert!(matches!(
        gis.python_setup.python,
        Some(OwnPython::Ready(..))
    ));
    assert_eq!(gis.python_setup.personal.env, Some(OwnEnv::Looking));
    edit(&mut gis, PersonalEdit::Found(Ok((None, 4096))));
    assert_eq!(gis.python_setup.personal.env, Some(OwnEnv::Missing));
    let _ = gis.python_section();
    // Made on the user's press.
    edit(&mut gis, PersonalEdit::Create);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Creating));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Created(Ok(())));
    assert!(titles(&gis).contains(&"Kişisel ortam oluşturuldu".to_owned()));
    let made = Personal {
        python: python_root.join("kisisel/.venv/bin/python"),
        size: 362_000_000,
        sdk: Some("0.1.0".into()),
        packages: Vec::new(),
    };
    edit(
        &mut gis,
        PersonalEdit::Found(Ok((Some(made.clone()), 4096))),
    );
    assert!(matches!(
        &gis.python_setup.personal.env,
        Some(OwnEnv::Ready(_))
    ));
    let _ = gis.python_section();
    // An option typed as a package never reaches uv.
    edit(
        &mut gis,
        PersonalEdit::Typed("--index-url=https://ornek.invalid".into()),
    );
    edit(&mut gis, PersonalEdit::PreviewAdd);
    assert!(gis.python_setup.personal.busy.is_none(), "uv not asked");
    assert!(gis.python_setup.personal.note.is_some());
    // Previewed: what is asked for, what comes with it.
    edit(&mut gis, PersonalEdit::Typed("pandas".into()));
    edit(&mut gis, PersonalEdit::PreviewAdd);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Previewing));
    let _ = gis.python_section();
    let package = |name: &str, version: &str, direct| Package {
        name: name.into(),
        version: version.into(),
        direct,
    };
    let adding = Preview {
        change: Change::Add(vec!["pandas".into()]),
        added: vec![
            package("pandas", "3.1.1", true),
            package("numpy", "2.5.4", false),
            package("python-dateutil", "4.0.0", false),
            package("tzdata", "2026.6", false),
        ],
        removed: Vec::new(),
        changed: Vec::new(),
        direct: vec!["pandas".into()],
    };
    assert_eq!(
        crate::python_personal::preview_words(&adding),
        [
            "Kurulacak: pandas 3.1.1.",
            "Bağımlılık olarak gelecek (3): numpy 2.5.4, python-dateutil 4.0.0, tzdata 2026.6."
        ]
    );
    edit(&mut gis, PersonalEdit::Previewed(Ok(adding.clone())));
    assert!(gis.python_setup.personal.preview.is_some());
    let _ = gis.python_section();
    // Agreed, then Durdur: the environment went back.
    edit(&mut gis, PersonalEdit::Apply);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Applying));
    let stop = gis.python_setup.personal.stop.clone().expect("applying");
    edit(
        &mut gis,
        PersonalEdit::Progressed(Progress::Downloading {
            done: 12_058_624,
            total: Some(28_835_840),
        }),
    );
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Stop);
    assert!(stop.load(std::sync::atomic::Ordering::Relaxed));
    edit(
        &mut gis,
        PersonalEdit::Applied(Err(InstallError::Cancelled)),
    );
    assert!(titles(&gis).contains(&"Paket kurulumu durduruldu".to_owned()));
    assert!(
        gis.python_setup.personal.busy.is_none() && gis.python_setup.personal.preview.is_none()
    );
    // Again, to the end.
    edit(&mut gis, PersonalEdit::Previewed(Ok(adding)));
    edit(&mut gis, PersonalEdit::Apply);
    edit(&mut gis, PersonalEdit::Applied(Ok(())));
    assert!(titles(&gis).contains(&"pandas 3.1.1 kuruldu".to_owned()));
    assert!(gis.python_setup.personal.typed.is_empty());
    let with_pandas = Personal {
        packages: vec![
            package("pandas", "3.1.1", true),
            package("numpy", "2.5.4", false),
            package("python-dateutil", "4.0.0", false),
            package("tzdata", "2026.6", false),
        ],
        ..made
    };
    edit(
        &mut gis,
        PersonalEdit::Found(Ok((Some(with_pandas), 30_000_000))),
    );
    let _ = gis.python_section();
    // A package removed from the same place, previewed too.
    edit(&mut gis, PersonalEdit::PreviewRemove("pandas".into()));
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Previewing));
    let removing = Preview {
        change: Change::Remove(vec!["pandas".into()]),
        added: Vec::new(),
        removed: vec![
            package("pandas", "3.1.1", true),
            package("numpy", "2.5.4", false),
            package("python-dateutil", "4.0.0", false),
            package("tzdata", "2026.6", false),
        ],
        changed: Vec::new(),
        direct: Vec::new(),
    };
    edit(&mut gis, PersonalEdit::Previewed(Ok(removing)));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Apply);
    edit(&mut gis, PersonalEdit::Applied(Ok(())));
    assert!(titles(&gis).contains(&"pandas kaldırıldı".to_owned()));
    // Picked from the recommended list, several at once, previewed together.
    edit(&mut gis, PersonalEdit::Typed("koordinat".into()));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Typed(String::new()));
    edit(&mut gis, PersonalEdit::ShowAll(true));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Pick("shapely".into(), true));
    edit(&mut gis, PersonalEdit::Pick("pyproj".into(), true));
    edit(&mut gis, PersonalEdit::Pick("numpy".into(), true));
    edit(&mut gis, PersonalEdit::Pick("numpy".into(), false));
    assert_eq!(
        gis.python_setup
            .personal
            .picked
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
        ["pyproj", "shapely"]
    );
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::PreviewPicked);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Previewing));
    edit(
        &mut gis,
        PersonalEdit::Previewed(Ok(Preview {
            change: Change::Add(vec!["pyproj".into(), "shapely".into()]),
            added: vec![
                package("pyproj", "3.7.2", true),
                package("shapely", "2.1.2", true),
                package("certifi", "2026.10.5", false),
            ],
            removed: Vec::new(),
            changed: Vec::new(),
            direct: vec!["pyproj".into(), "shapely".into()],
        })),
    );
    edit(&mut gis, PersonalEdit::Apply);
    edit(&mut gis, PersonalEdit::Applied(Ok(())));
    assert!(titles(&gis).contains(&"pyproj 3.7.2, shapely 2.1.2 kuruldu".to_owned()));
    assert!(
        gis.python_setup.personal.picked.is_empty(),
        "ticks cleared once made"
    );
    // A name not among the recommended ones is asked for as it is.
    edit(&mut gis, PersonalEdit::Typed("pyogrio-yok-boyle".into()));
    let _ = gis.python_section();
    // Updates: asked of uv; one or all previewed as an upgrade.
    let with_old = Personal {
        packages: vec![
            package("pandas", "2.3.3", true),
            package("numpy", "2.5.4", false),
        ],
        python: python_root.join("kisisel/.venv/bin/python"),
        size: 400_000_000,
        sdk: Some("0.1.0".into()),
    };
    edit(&mut gis, PersonalEdit::Found(Ok((Some(with_old), 1))));
    edit(&mut gis, PersonalEdit::CheckUpdates);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Checking));
    edit(
        &mut gis,
        PersonalEdit::Checked(Ok(vec![
            Outdated {
                name: "pandas".into(),
                version: "2.3.3".into(),
                latest: "3.1.1".into(),
            },
            Outdated {
                name: "numpy".into(),
                version: "2.5.4".into(),
                latest: "2.6.0".into(),
            },
        ])),
    );
    assert!(titles(&gis).contains(&"2 paketin yeni sürümü var".to_owned()));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Upgrade(None));
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Previewing));
    edit(
        &mut gis,
        PersonalEdit::Previewed(Ok(Preview {
            change: Change::Upgrade {
                raise: vec![("pandas".into(), "3.1.1".into())],
                others: vec!["numpy".into()],
            },
            added: Vec::new(),
            removed: Vec::new(),
            changed: vec![
                ("numpy".into(), "2.5.4".into(), "2.6.0".into()),
                ("pandas".into(), "2.3.3".into(), "3.1.1".into()),
            ],
            direct: vec!["pandas".into()],
        })),
    );
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Apply);
    edit(&mut gis, PersonalEdit::Applied(Ok(())));
    assert!(titles(&gis).contains(&"Paketler güncellendi".to_owned()));
    assert!(
        gis.python_setup.personal.outdated.is_none(),
        "asked again when wanted"
    );
    // The dependency tree, shown and hidden.
    edit(&mut gis, PersonalEdit::ToggleTree);
    assert_eq!(gis.python_setup.personal.tree.as_deref(), Some(""));
    edit(
        &mut gis,
        PersonalEdit::Tree(Ok("kentos-kisisel v0\n└── pandas v3.1.1\n".into())),
    );
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::ToggleTree);
    assert!(gis.python_setup.personal.tree.is_none());
    edit(&mut gis, PersonalEdit::Tree(Ok("geç gelen".into())));
    assert!(
        gis.python_setup.personal.tree.is_none(),
        "hidden meanwhile: stays hidden"
    );
    // A failure is said in the part and by a notice that waits.
    edit(&mut gis, PersonalEdit::Typed("numpy".into()));
    edit(&mut gis, PersonalEdit::PreviewAdd);
    edit(
        &mut gis,
        PersonalEdit::Previewed(Err("Paket dizinine ulaşılamadı.".into())),
    );
    assert_eq!(
        gis.python_setup.personal.note.as_deref(),
        Some("Paket dizinine ulaşılamadı.")
    );
    // The cache, apart.
    edit(&mut gis, PersonalEdit::CleanCache);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Cleaning));
    edit(&mut gis, PersonalEdit::Cleaned(Ok(())));
    assert!(titles(&gis).contains(&"Paket önbelleği temizlendi".to_owned()));
    // KentOS's Python's removal says the environment goes with it.
    let _ = gis.python_edit(PythonEdit::Remove);
    let _ = gis.python_section();
    let _ = gis.python_edit(PythonEdit::RemoveCancelled);
    // The environment removed, asked first.
    edit(&mut gis, PersonalEdit::Remove);
    assert!(gis.python_setup.personal.confirm_remove);
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::RemoveCancelled);
    assert!(!gis.python_setup.personal.confirm_remove);
    edit(&mut gis, PersonalEdit::Remove);
    edit(&mut gis, PersonalEdit::RemoveConfirmed);
    assert_eq!(gis.python_setup.personal.busy, Some(Busy::Removing));
    let _ = gis.python_section();
    edit(&mut gis, PersonalEdit::Removed(Ok(())));
    assert!(titles(&gis).contains(&"Kişisel ortam kaldırıldı".to_owned()));
    // Python off: the environment is let go of.
    let _ = gis.python_edit(PythonEdit::Enable(false));
    assert!(gis.python_setup.personal.env.is_none());
    gis.open_settings_at(crate::settings_sections::Section::Python);
    let _ = gis.settings_dialog();
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&python_root);
}

/// Bir eklentinin Python ortamı (plan/eklenti.md ADIM 18A, P05, P07): in the extension's details, once KentOS's Python
/// is ready: looked for when chosen; what preparing would install previewed from its lock, its optional feature ticked
/// into it; made with a bar and Durdur, each outcome a notice; the ribbon's button says, for that extension, whether
/// its environment is ready (files on disk); removed from the same place, and with the extension.
#[test]
fn an_extensions_python_environment_is_previewed_prepared_and_removed() {
    use super::environment::{EnvBusy, EnvEvent, EnvStatus};
    use crate::python_setup::{OwnPython, PythonEdit};
    use kentos_python_env::extension::{self, Listed, Ready};
    use kentos_python_env::python::{InstallError, Interpreter, Progress};
    use kentos_python_env::uv::UvStatus;

    const MEBIS: &str = "tr.ornek.mebis";
    let root = temp("ortam");
    let python_root = temp("ortam-python");
    let mut gis = app(&root, Some(Module::Gis));
    gis.python_setup.root = Some(python_root.clone());
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] = serde_json::json!([]);
        }),
    );
    let env = |app: &mut App, e: EnvEvent| {
        let _ = app.extensions_event(Event::Env(e));
    };
    let titles = |app: &App| -> Vec<String> {
        app.toasts
            .iter()
            .map(|(_, toast, _)| toast.title().to_owned())
            .collect()
    };
    let why = |app: &App| -> String {
        let tab = app
            .extension_tabs()
            .find(|t| t.extension == MEBIS)
            .expect("MEBİS's tab");
        match &tab.panels[0].items[0].action {
            Action::Unavailable(why) => why.clone(),
            other => panic!("{other:?}"),
        }
    };
    // Before KentOS's Python: the section says what comes first; nothing looked for.
    let _ = gis.extensions_event(Event::Choose(MEBIS.into()));
    assert!(!gis.extensions.envs.contains_key(MEBIS));
    let _ = gis.extensions_view();
    // KentOS's Python ready.
    let _ = gis.python_edit(PythonEdit::Enable(true));
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::Found {
        path: "/k/tools/uv".into(),
        version: "0.11.26".into(),
        bundled: true,
    }));
    let python = Interpreter {
        version: "3.12.13".into(),
        path: python_root.join("yorumlayici/cpython-3.12/bin/python3.12"),
    };
    let _ = gis.python_edit(PythonEdit::Found(Ok((Some(python.clone()), 97_885_768))));
    assert!(matches!(
        gis.python_setup.python,
        Some(OwnPython::Ready(..))
    ));
    assert!(
        why(&gis).contains("Bu eklentinin Python ortamı hazırlanmadı: Eklentiler ›"),
        "{}",
        why(&gis)
    );
    // Looked for when the window opens on it, without a click: only a click looked, and until one the section said
    // "Ortam aranıyor…".
    gis.open_extensions();
    assert_eq!(gis.extensions.look_wanted.as_deref(), Some(MEBIS));
    let _ = gis.ext_look_tasks();
    assert_eq!(gis.extensions.look_wanted, None);
    assert_eq!(gis.extensions.envs[MEBIS].status, Some(EnvStatus::Looking));
    gis.extensions.envs.remove(MEBIS);
    // Looked for when chosen.
    let _ = gis.extensions_event(Event::Choose(MEBIS.into()));
    assert_eq!(gis.extensions.envs[MEBIS].status, Some(EnvStatus::Looking));
    env(&mut gis, EnvEvent::Found(MEBIS.into(), Ok(None)));
    assert_eq!(gis.extensions.envs[MEBIS].status, Some(EnvStatus::Missing));
    let _ = gis.extensions_view();
    // Its optional feature ticked; previewed from the lock.
    env(
        &mut gis,
        EnvEvent::Extra(MEBIS.into(), "charts".into(), true),
    );
    assert!(gis.extensions.envs[MEBIS].extras.contains("charts"));
    env(&mut gis, EnvEvent::Preview(MEBIS.into()));
    assert_eq!(gis.extensions.envs[MEBIS].busy, Some(EnvBusy::Previewing));
    let listed = vec![
        Listed {
            name: "numpy".into(),
            version: "2.5.4".into(),
        },
        Listed {
            name: "shapely".into(),
            version: "2.2.0".into(),
        },
    ];
    assert_eq!(
        super::environment::listed_words(&listed),
        "Kurulacak paketler (2): numpy 2.5.4, shapely 2.2.0."
    );
    env(
        &mut gis,
        EnvEvent::Previewed(MEBIS.into(), Ok(listed.clone())),
    );
    assert!(gis.extensions.envs[MEBIS].preview.is_some());
    let _ = gis.extensions_view();
    // Prepared on the press, with a bar; Durdur stops it.
    env(&mut gis, EnvEvent::Prepare(MEBIS.into()));
    assert_eq!(gis.extensions.envs[MEBIS].busy, Some(EnvBusy::Preparing));
    let stop = gis.extensions.envs[MEBIS].stop.clone().expect("preparing");
    env(
        &mut gis,
        EnvEvent::Progressed(
            MEBIS.into(),
            Progress::Downloading {
                done: 9_000_000,
                total: Some(17_000_000),
            },
        ),
    );
    let _ = gis.extensions_view();
    env(&mut gis, EnvEvent::Stop(MEBIS.into()));
    assert!(stop.load(std::sync::atomic::Ordering::Relaxed));
    env(
        &mut gis,
        EnvEvent::Prepared(MEBIS.into(), Err(InstallError::Cancelled)),
    );
    assert!(
        titles(&gis)
            .iter()
            .any(|t| t.ends_with("ortam hazırlama durduruldu"))
    );
    // Ready: made on disk as the module makes it, so the ribbon's words follow (files only, offline).
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(MEBIS))
        .unwrap();
    let key = extension::key_for(
        &package,
        "uv.lock",
        MEBIS,
        "1.0.0",
        &python,
        &["charts".into()],
    )
    .unwrap();
    let dir = extension::place(&python_root, &key);
    std::fs::create_dir_all(dir.join(".venv/bin")).unwrap();
    std::fs::write(dir.join(".venv/bin/python"), b"").unwrap();
    let ready = Ready {
        format: "kentos.extension-env".into(),
        key: key.clone(),
        python: dir.join(".venv/bin/python"),
        packages: listed,
        sdk: String::new(),
        made_at: 1,
    };
    std::fs::write(
        dir.join(extension::RECORD),
        serde_json::to_string(&ready).unwrap(),
    )
    .unwrap();
    env(&mut gis, EnvEvent::Prepare(MEBIS.into()));
    env(
        &mut gis,
        EnvEvent::Prepared(MEBIS.into(), Ok(Box::new(ready.clone()))),
    );
    assert!(
        titles(&gis)
            .iter()
            .any(|t| t.ends_with("Python ortamı hazır"))
    );
    // Its command now runs (ADIM 19: an_extensions_command_runs_in_its_own_process_through_the_broker).
    let action = gis
        .extension_tabs()
        .find(|t| t.extension == MEBIS)
        .expect("MEBİS's tab")
        .panels[0]
        .items[0]
        .action
        .clone();
    assert!(
        matches!(action, Action::Python(ref e, ref k, ref l) if e == MEBIS && k == "command" && l == "init-schema"),
        "{action:?}"
    );
    env(
        &mut gis,
        EnvEvent::Found(MEBIS.into(), Ok(Some(Box::new(ready)))),
    );
    assert!(matches!(
        gis.extensions.envs[MEBIS].status,
        Some(EnvStatus::Ready(_))
    ));
    let _ = gis.extensions_view();
    // A failure is said in the section and by a notice that waits.
    env(&mut gis, EnvEvent::Prepare(MEBIS.into()));
    env(
        &mut gis,
        EnvEvent::Prepared(
            MEBIS.into(),
            Err(InstallError::Failed {
                cause: kentos_python_env::python::Cause::Network,
                said: "dns error".into(),
            }),
        ),
    );
    assert!(
        titles(&gis)
            .iter()
            .any(|t| t.ends_with("Python ortamı hazırlanamadı"))
    );
    assert!(
        gis.extensions.envs[MEBIS]
            .note
            .as_deref()
            .is_some_and(|n| n.contains("internete ulaşılamadı"))
    );
    // Removed with the extension.
    assert!(dir.exists());
    let _ = gis.extensions_event(Event::Remove(MEBIS.into()));
    assert!(!dir.exists(), "its environments go with it");
    assert!(!gis.extensions.envs.contains_key(MEBIS));
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&python_root);
}

/// Bir eklentinin komutu çalışırken (plan/eklenti.md ADIM 19): its ribbon button runs once its environment is ready;
/// its process is started with a hello and the call; the broker answers its reads on the open drawing and refuses
/// what it was not granted (İmar asked to read, not to write) and what is the user's (undo); its report comes back
/// as a notice and the messages; a failure is logged with its traceback; Durdur asks the code, then ends a process
/// that does not answer; a process that ends by itself is a crash. The process's lines are given here as it would
/// send them (its own side: python/tests/test_ext_host.py).
#[cfg(unix)]
#[test]
fn an_extensions_command_runs_in_its_own_process_through_the_broker() {
    use super::runner::row_words;
    use crate::python::host::Said;
    use crate::python_setup::PythonEdit;
    use kentos_python_env::extension::{self, Ready};
    use kentos_python_env::python::Interpreter;
    use kentos_python_env::uv::UvStatus;
    use serde_json::json;

    const IMAR: &str = "tr.ornek.imar-denetim";
    let root = temp("yurutucu");
    let python_root = temp("yurutucu-python");
    let mut gis = crate::files_testing::app_with_drawing();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&root, false).0;
    gis.extensions_changed();
    gis.python_setup.root = Some(python_root.clone());
    install(&mut gis, "imar.kek", package("ornek-imar", |_| {}));
    let _ = gis.python_edit(PythonEdit::Enable(true));
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::Found {
        path: "/k/tools/uv".into(),
        version: "0.11.26".into(),
        bundled: true,
    }));
    let python = Interpreter {
        version: "3.12.13".into(),
        path: python_root.join("yorumlayici/cpython-3.12/bin/python3.12"),
    };
    let _ = gis.python_edit(PythonEdit::Found(Ok((Some(python.clone()), 1))));
    let button = |app: &App| {
        app.extension_tabs()
            .find(|t| t.extension == IMAR)
            .expect("İmar's tab")
            .panels[0]
            .items[0]
            .action
            .clone()
    };
    assert!(
        matches!(button(&gis), Action::Unavailable(_)),
        "no environment yet"
    );
    // Its environment ready (a record on disk; its Python anything that starts: its lines are given here).
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(IMAR))
        .unwrap();
    let key = extension::key_for(&package, "uv.lock", IMAR, "1.1.0", &python, &[]).unwrap();
    let dir = extension::place(&python_root, &key);
    std::fs::create_dir_all(&dir).unwrap();
    let ready = Ready {
        format: "kentos.extension-env".into(),
        key,
        python: "/bin/sh".into(),
        packages: Vec::new(),
        sdk: String::new(),
        made_at: 1,
    };
    std::fs::write(
        dir.join(extension::RECORD),
        serde_json::to_string(&ready).unwrap(),
    )
    .unwrap();
    gis.extensions_changed();
    assert!(
        matches!(button(&gis), Action::Python(ref e, ref k, ref l) if e == IMAR && k == "command" && l == "check-setbacks"),
        "{:?}",
        button(&gis)
    );
    // Pressed: the environment found in the background, then the process started and the call sent.
    let launch = super::runner::Launch::for_test(
        "/bin/sh".into(),
        package.clone(),
        "imar.logic:check_setbacks",
        "Yaklaşma mesafelerini denetle",
        false,
    );
    let _ = gis.extensions_event(Event::Launch(
        IMAR.into(),
        "command".into(),
        "check-setbacks".into(),
        Ok(launch.clone()),
    ));
    let generation = gis.extensions.runners[IMAR].generation;
    let run = gis.extensions.runners[IMAR]
        .run
        .as_ref()
        .expect("running")
        .id;
    let say = |app: &mut App, m: serde_json::Value| {
        let _ = app.extensions_event(Event::Runner(IMAR.into(), generation, Said::Message(m)));
    };
    let titles = |app: &App| -> Vec<String> {
        app.toasts
            .iter()
            .map(|(_, toast, _)| toast.title().to_owned())
            .collect()
    };
    assert!(
        titles(&gis).iter().any(|t| t.ends_with("çalışıyor")),
        "{:?}",
        titles(&gis)
    );
    say(&mut gis, json!({"type": "ready", "protocol": 1}));
    assert!(gis.extensions.runners[IMAR].ready);
    // The broker: reads answered on the open drawing; a write not granted, and the user's undo, refused.
    let summary = gis.ext_answer_for_test(IMAR, "summary", &json!({}));
    assert!(summary.is_ok(), "{summary:?}");
    let layers = gis.ext_answer_for_test(IMAR, "layers", &json!({}));
    assert!(layers.is_ok(), "{layers:?}");
    let write = gis
        .ext_answer_for_test(
            IMAR,
            "run",
            &json!({"command": "cad.polygon.create", "op": "execute", "input": {}}),
        )
        .unwrap_err();
    assert_eq!(write.code, "capability");
    assert!(write.message.contains("document.write"));
    let undo = gis
        .ext_answer_for_test(IMAR, "undo", &json!({}))
        .unwrap_err();
    assert_eq!(undo.code, "not_for_extensions");
    // Its report: a notice and the rows in the messages.
    say(
        &mut gis,
        json!({"type": "progress", "id": run, "done": 1, "total": 3}),
    );
    say(
        &mut gis,
        json!({"type": "log", "id": run, "level": "info", "text": "3 yapı denetlendi"}),
    );
    say(
        &mut gis,
        json!({"type": "done", "id": run, "ok": true, "result": {
            "kind": "report", "title": "Yaklaşma mesafesi denetimi", "message": "3 yapı denetlendi; 1 yapı sınırın içinde.",
            "rows": [{"yapı": "B1", "mesafe_m": 2.0}]
        }}),
    );
    assert!(gis.extensions.runners[IMAR].run.is_none());
    assert!(
        titles(&gis)
            .iter()
            .any(|t| t.ends_with("Yaklaşma mesafesi denetimi")),
        "{:?}",
        titles(&gis)
    );
    assert_eq!(
        gis.extensions
            .result
            .as_ref()
            .map(|(e, t, _)| (e.as_str(), t.as_str())),
        Some((IMAR, "Yaklaşma mesafesi denetimi"))
    );
    // Its columns in the order it gave them (a JSON object's keys come sorted).
    assert_eq!(
        row_words(
            &["yapı", "mesafe_m"],
            &json!({"yapı": "B1", "mesafe_m": 2.0, "not": "x"})
        ),
        "yapı: B1 · mesafe_m: 2.0 · not: x"
    );
    let logged = |app: &App, code: &str| {
        app.extensions
            .registry
            .as_ref()
            .unwrap()
            .log_of(IMAR)
            .iter()
            .any(|r| r.code == code)
    };
    assert!(logged(&gis, "run_started") && logged(&gis, "run_log") && logged(&gis, "run_done"));
    // A failure: logged with its traceback, said by a notice naming the extension.
    let _ = gis.extensions_event(Event::Launch(
        IMAR.into(),
        "command".into(),
        "check-setbacks".into(),
        Ok(launch.clone()),
    ));
    let run = gis.extensions.runners[IMAR].run.as_ref().unwrap().id;
    say(
        &mut gis,
        json!({"type": "done", "id": run, "ok": false, "exception": "ZeroDivisionError", "message": "division by zero",
               "error": "Traceback (most recent call last):\n  File \"logic.py\", line 3\nZeroDivisionError: division by zero"}),
    );
    let failed = gis
        .extensions
        .registry
        .as_ref()
        .unwrap()
        .log_of(IMAR)
        .iter()
        .rev()
        .find(|r| r.code == "run_failed")
        .cloned()
        .expect("logged");
    assert!(failed.message.contains("ZeroDivisionError"));
    assert!(
        failed
            .detail
            .as_deref()
            .is_some_and(|d| d.contains("Traceback"))
    );
    // Said where its start was said, with the way to its log.
    assert!(
        titles(&gis)
            .iter()
            .any(|t| t.ends_with("“Yaklaşma mesafelerini denetle” hata verdi")),
        "{:?}",
        titles(&gis)
    );
    // The messages' line points at where it failed, not at the traceback's first line.
    assert!(
        gis.log
            .lines()
            .any(|l| l.text.contains("[logic.py:3]") && l.text.contains("ZeroDivisionError")),
        "{:?}",
        gis.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>()
    );
    // Ayrıntı: the manager on it, its environment looked for as a click on its row would.
    let _ = gis.extensions_event(Event::Show(IMAR.into()));
    assert_eq!(gis.extensions.chosen.as_deref(), Some(IMAR));
    assert!(gis.extensions.envs.contains_key(IMAR), "looked for");
    // Unused for a while: its process closed (D6); the next call starts a new one.
    let generation = gis.extensions.runners[IMAR].generation;
    let _ = gis.extensions_event(Event::RunnerIdle(IMAR.into(), generation));
    assert!(
        gis.extensions.runners.contains_key(IMAR),
        "not unused long enough"
    );
    let long_ago = std::time::Instant::now().checked_sub(kentos_python_env::process::IDLE);
    gis.extensions.runners.get_mut(IMAR).unwrap().ended = long_ago;
    let _ = gis.extensions_event(Event::RunnerIdle(IMAR.into(), generation));
    assert!(
        !gis.extensions.runners.contains_key(IMAR),
        "closed when unused"
    );
    assert!(logged(&gis, "runner_idle"));
    // Durdur: asked of the code; a process that does not answer is ended.
    let _ = gis.extensions_event(Event::Launch(
        IMAR.into(),
        "command".into(),
        "check-setbacks".into(),
        Ok(launch.clone()),
    ));
    let run = gis.extensions.runners[IMAR].run.as_ref().unwrap().id;
    let _ = gis.extensions_event(Event::StopRun(IMAR.into()));
    assert!(
        gis.extensions.runners[IMAR]
            .run
            .as_ref()
            .unwrap()
            .stopping
            .is_some()
    );
    let _ = gis.extensions_event(Event::RunOverdue(IMAR.into(), run));
    assert!(
        !gis.extensions.runners.contains_key(IMAR),
        "its process ended"
    );
    assert!(titles(&gis).iter().any(|t| t.ends_with("durduruldu")));
    // A process that ends by itself during a run: a crash, logged.
    let _ = gis.extensions_event(Event::Launch(
        IMAR.into(),
        "command".into(),
        "check-setbacks".into(),
        Ok(launch),
    ));
    let generation = gis.extensions.runners[IMAR].generation;
    let _ = gis.extensions_event(Event::Runner(IMAR.into(), generation, Said::Closed));
    assert!(!gis.extensions.runners.contains_key(IMAR));
    assert!(logged(&gis, "runner_crashed"));
    let _ = std::fs::remove_dir_all(&root);
    let _ = std::fs::remove_dir_all(&python_root);
}

/// docs/adr/8057 §29: a running command's `dialog` opens a KentOS window with a panel's fields and is answered when the
/// user is: Tamam's values by id (a wrong number said by its label, the window open), İptal's none; a dialog with a
/// button or a command is the call's error at once; Durdur closes it, answered none.
#[cfg(unix)]
#[test]
fn an_extensions_dialog_asks_the_user_and_answers_its_code() {
    use super::panels::PanelEvent;
    use crate::python::host::Said;
    use serde_json::json;

    const MEBIS: &str = "tr.ornek.mebis";
    let root = temp("pencere");
    let mut gis = crate::files_testing::app_with_drawing();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&root, false).0;
    gis.extensions_changed();
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] = json!([]);
        }),
    );
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(MEBIS))
        .unwrap();
    // Its "Python" keeps what KentOS writes to it: the replies are read back from there.
    let heard = root.join("duyulan.jsonl");
    let python = root.join("python");
    std::fs::write(
        &python,
        format!("#!/bin/sh\nexec cat >> '{}'\n", heard.display()),
    )
    .unwrap();
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&python, std::fs::Permissions::from_mode(0o755)).unwrap();
    }
    let launch = super::runner::Launch::for_test(
        python,
        package,
        "mebis.logic:init_schema",
        "Numarala",
        false,
    );
    let _ = gis.extensions_event(Event::Launch(
        MEBIS.into(),
        "command".into(),
        "init-schema".into(),
        Ok(launch),
    ));
    let generation = gis.extensions.runners[MEBIS].generation;
    let say = |app: &mut App, m: serde_json::Value| {
        let _ = app.extensions_event(Event::Runner(MEBIS.into(), generation, Said::Message(m)));
    };
    // The reply to call `id`, once the process has it.
    let reply = |id: u64| -> serde_json::Value {
        for _ in 0..200 {
            let text = std::fs::read_to_string(&heard).unwrap_or_default();
            if let Some(found) = text
                .lines()
                .filter_map(|l| serde_json::from_str::<serde_json::Value>(l).ok())
                .find(|m| m["type"] == "reply" && m["id"] == id)
            {
                return found;
            }
            std::thread::sleep(std::time::Duration::from_millis(20));
        }
        panic!("no reply to {id}");
    };
    say(&mut gis, json!({"type": "ready", "protocol": 1}));
    let body = json!([
        { "type": "text", "text": "Yapılar bu önekle numaralanır." },
        { "type": "textField", "id": "onek", "label": "Önek", "value": "Y-" },
        { "type": "numberField", "id": "ilk", "label": "İlk numara", "value": 1, "min": 1 }
    ]);
    say(
        &mut gis,
        json!({"type": "call", "id": 5, "method": "dialog",
               "params": {"title": "Yapıları numarala", "body": body, "ok": "Numarala"}}),
    );
    let dialog = gis.extensions.dialog.as_ref().expect("the window is open");
    assert_eq!(
        (dialog.ok.as_str(), dialog.cancel.as_str()),
        ("Numarala", "İptal")
    );
    assert!(gis.ext_dialog_view().is_some());
    // A wrong number: said by its label, the window stays, nothing answered.
    let _ = gis.extensions_event(Event::Dialog(PanelEvent::Text("onek".into(), "B-".into())));
    let _ = gis.extensions_event(Event::Dialog(PanelEvent::Text("ilk".into(), "beş".into())));
    let _ = gis.extensions_event(Event::DialogAnswer(true));
    let said = gis
        .extensions
        .dialog
        .as_ref()
        .and_then(|d| d.panel.said.clone());
    assert_eq!(
        said.map(|(_, w)| w).as_deref(),
        Some("“İlk numara”: “beş” bir sayı değil.")
    );
    let _ = gis.extensions_event(Event::Dialog(PanelEvent::Text("ilk".into(), "5".into())));
    let _ = gis.extensions_event(Event::DialogAnswer(true));
    assert!(gis.extensions.dialog.is_none(), "answered, closed");
    assert_eq!(reply(5)["result"], json!({ "onek": "B-", "ilk": 5.0 }));
    // İptal: none.
    say(
        &mut gis,
        json!({"type": "call", "id": 6, "method": "dialog", "params": {"title": "Emin misiniz?",
                          "body": [{ "type": "text", "text": "Sürsün mü?" }]}}),
    );
    let _ = gis.extensions_event(Event::DialogAnswer(false));
    assert_eq!(reply(6)["result"], serde_json::Value::Null);
    // A button in a dialog: the call's error at once, no window.
    say(
        &mut gis,
        json!({"type": "call", "id": 7, "method": "dialog", "params": {"title": "Yanlış",
                          "body": [{ "type": "button", "id": "b", "label": "B", "command": "x" }]}}),
    );
    assert!(gis.extensions.dialog.is_none());
    let refused = reply(7);
    assert_eq!(
        (refused["ok"].clone(), refused["code"].clone()),
        (json!(false), json!("invalid_input"))
    );
    assert!(
        refused["message"]
            .as_str()
            .is_some_and(|m| m.contains("pencerede olamaz")),
        "{refused}"
    );
    // Durdur while it asks: closed, answered none.
    say(
        &mut gis,
        json!({"type": "call", "id": 8, "method": "dialog", "params": {"title": "Soru",
                          "body": [{ "type": "checkbox", "id": "evet", "label": "Evet" }]}}),
    );
    assert!(gis.extensions.dialog.is_some());
    let _ = gis.extensions_event(Event::StopRun(MEBIS.into()));
    assert!(gis.extensions.dialog.is_none(), "Durdur closes it");
    assert_eq!(reply(8)["result"], serde_json::Value::Null);
    let _ = std::fs::remove_dir_all(&root);
}

/// Bir eklenti çizime yazarken (ADIM 19): its run is one undo step named after its command, kept when it ends well and
/// taken back when it fails or is stopped; meanwhile the drawing takes no other edit (the user is told once), and
/// neither the console, an agent nor another writing run goes; an ability this KentOS lacks is said calmly.
#[cfg(unix)]
#[test]
fn an_extensions_writes_are_one_undo_step_and_the_drawing_waits_for_them() {
    use crate::python::host::Said;
    use serde_json::json;

    const MEBIS: &str = "tr.ornek.mebis";
    let root = temp("yazan");
    let mut gis = crate::files_testing::app_with_drawing();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&root, false).0;
    gis.extensions_changed();
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] = json!([]);
        }),
    );
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(MEBIS))
        .unwrap();
    let launch = super::runner::Launch::for_test(
        "/bin/sh".into(),
        package,
        "mebis.logic:init_schema",
        "MEBİS katmanlarını kur",
        true,
    );
    let start = |app: &mut App| {
        let _ = app.extensions_event(Event::Launch(
            MEBIS.into(),
            "command".into(),
            "init-schema".into(),
            Ok(launch.clone()),
        ));
        let runner = &app.extensions.runners[MEBIS];
        (runner.generation, runner.run.as_ref().expect("running").id)
    };
    let say = |app: &mut App, generation: u64, m: serde_json::Value| {
        let _ = app.extensions_event(Event::Runner(MEBIS.into(), generation, Said::Message(m)));
    };
    let add_layer = |app: &mut App, name: &str| {
        app.ext_answer_for_test(
            MEBIS,
            "run",
            &json!({"command": "cad.layers.edit", "op": "execute",
                    "input": {"operation": "add", "name": name, "geometry": "area"}}),
        )
    };
    let has_layer = |app: &App, name: &str| {
        app.document
            .as_ref()
            .unwrap()
            .model
            .layers()
            .nodes()
            .iter()
            .any(|n| n.name == name)
    };
    let titles = |app: &App| -> Vec<String> {
        app.toasts
            .iter()
            .map(|(_, toast, _)| toast.title().to_owned())
            .collect()
    };
    // Running: the drawing waits for it, said once; the console and agents wait too.
    let (generation, run) = start(&mut gis);
    assert_eq!(
        gis.extension_writing().as_deref(),
        Some("Mezarlık Bilgi Sistemi (MEBİS): “MEBİS katmanlarını kur”")
    );
    let before = gis.log.lines().count();
    assert!(
        gis.while_extension_writes(&Message::CommandSubmitted)
            .is_some()
    );
    assert!(
        gis.while_extension_writes(&Message::CommandSubmitted)
            .is_some()
    );
    let said: Vec<String> = gis
        .log
        .lines()
        .skip(before)
        .map(|l| l.text.clone())
        .collect();
    assert_eq!(said.len(), 1, "{said:?}");
    assert!(said[0].contains("düzenlenemez"));
    // A command typed meanwhile waits too (found in the real window, 14 November 2026: Çizgi started).
    let tool = gis.session.tool_id().to_owned();
    let _ = gis.update(Message::CommandInput("CIZGI".into()));
    let _ = gis.update(Message::CommandSubmitted);
    assert_eq!(gis.session.tool_id(), tool, "no command starts");
    let busy = gis
        .agent_request(&json!({"method": "summary"}), "Ajan")
        .unwrap_err();
    assert_eq!(busy.0, "busy");
    assert!(busy.1.contains("MEBİS katmanlarını kur"));
    let _ = gis.python_run_titled("print(1)".into(), "deneme.py".into());
    assert!(gis.python.running.is_none(), "the console waits");
    // Its writes: one undo step named after the command, kept when it ends well.
    add_layer(&mut gis, "mezar_ada").expect("written");
    add_layer(&mut gis, "mezar_yeri").expect("written");
    say(
        &mut gis,
        generation,
        json!({"type": "done", "id": run, "ok": true, "result": {"kind": "message", "message": "Kuruldu."}}),
    );
    assert!(gis.extension_writing().is_none());
    assert!(
        gis.while_extension_writes(&Message::CommandSubmitted)
            .is_none()
    );
    assert!(has_layer(&gis, "mezar_ada") && has_layer(&gis, "mezar_yeri"));
    let doc = gis.document.as_mut().unwrap();
    assert_eq!(
        doc.model.undo_labels().first().map(String::as_str),
        Some("MEBİS katmanlarını kur")
    );
    doc.model.undo();
    assert!(
        !has_layer(&gis, "mezar_ada") && !has_layer(&gis, "mezar_yeri"),
        "one step took both back"
    );
    // A failure takes back what it wrote.
    let (generation, run) = start(&mut gis);
    add_layer(&mut gis, "mezar_ada").expect("written");
    say(
        &mut gis,
        generation,
        json!({"type": "done", "id": run, "ok": false, "exception": "KeyError", "message": "x", "error": "Traceback …"}),
    );
    assert!(!has_layer(&gis, "mezar_ada"), "taken back");
    // Durdur, answered by the code: taken back.
    let (generation, run) = start(&mut gis);
    add_layer(&mut gis, "mezar_ada").expect("written");
    let _ = gis.extensions_event(Event::StopRun(MEBIS.into()));
    say(
        &mut gis,
        generation,
        json!({"type": "done", "id": run, "ok": false, "stopped": true}),
    );
    assert!(!has_layer(&gis, "mezar_ada"), "taken back");
    assert!(titles(&gis).iter().any(|t| t.ends_with("durduruldu")));
    // An ability this KentOS lacks: said calmly, logged as a warning; what it wrote taken back.
    let (generation, run) = start(&mut gis);
    add_layer(&mut gis, "mezar_ada").expect("written");
    say(
        &mut gis,
        generation,
        json!({"type": "done", "id": run, "ok": false, "unavailable": true,
               "message": "Katman şeması kurulumu (schema.json) bu KentOS sürümünde yok."}),
    );
    assert!(!has_layer(&gis, "mezar_ada"), "taken back");
    assert!(
        titles(&gis)
            .iter()
            .any(|t| t.ends_with("bu sürümde çalışmaz")),
        "{:?}",
        titles(&gis)
    );
    let unavailable = gis
        .extensions
        .registry
        .as_ref()
        .unwrap()
        .log_of(MEBIS)
        .iter()
        .rev()
        .find(|r| r.code == "run_unavailable")
        .cloned()
        .expect("logged");
    assert!(unavailable.detail.is_none(), "no traceback");
    // Another writing run waits for one that writes.
    let (_generation, _run) = start(&mut gis);
    let before = gis.extensions.run_counter;
    let _ = gis.extensions_event(Event::Launch(
        MEBIS.into(),
        "command".into(),
        "init-schema".into(),
        Ok(launch.clone()),
    ));
    assert_eq!(gis.extensions.run_counter, before, "not started twice");
    let _ = std::fs::remove_dir_all(&root);
}

/// An extension's process writes no bytecode beside its package (whose integrity check refuses added files): its
/// interpreter's arguments send it to the environment's folder (real window, 14 November 2026).
#[test]
fn an_extensions_bytecode_goes_to_its_environment_not_its_package() {
    let args = super::runner::interpreter_args(std::path::Path::new("/k/ortamlar/ab12/pyc"));
    let args: Vec<String> = args
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        args,
        [
            "-I",
            "-X",
            "pycache_prefix=/k/ortamlar/ab12/pyc",
            "-m",
            "kentos.ext_host"
        ]
    );
}
