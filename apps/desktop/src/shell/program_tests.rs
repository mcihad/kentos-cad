//! Which program this is and the module a drawing belongs to (program.rs).
use std::ffi::OsString;
use std::path::PathBuf;

use kentos_commands::{CadWork, Workspace};
use kentos_launch::Module;

use super::*;
use crate::files_testing::app_with_drawing;

fn os(args: &[&str]) -> Vec<OsString> {
    args.iter().map(OsString::from).collect()
}

#[test]
fn a_project_belongs_by_its_type_and_work() {
    let mut s = crate::files_testing::drawing(0).settings().clone();
    let mut of = |w: Option<Workspace>, work: Option<CadWork>| {
        s.workspace = w;
        s.cad_work = work;
        module_of(&s)
    };
    assert_eq!(of(Some(Workspace::Gis), None), Some(Module::Gis));
    assert_eq!(
        of(Some(Workspace::Cad), Some(CadWork::Architecture)),
        Some(Module::Architecture)
    );
    assert_eq!(
        of(Some(Workspace::Cad), Some(CadWork::Map)),
        Some(Module::Map)
    );
    assert_eq!(
        of(Some(Workspace::Cad), None),
        None,
        "a CAD project not asked its work"
    );
    assert_eq!(of(None, None), None, "a project not asked its type");
    assert_eq!(
        of(Some(Workspace::LegacyHybrid), None),
        None,
        "the former Hibrit"
    );
    // A work on a project that is not CAD is no work.
    assert_eq!(
        of(Some(Workspace::Gis), Some(CadWork::Map)),
        Some(Module::Gis)
    );
    assert_eq!(of(None, Some(CadWork::Map)), None);
    // Every work is one module's, the module's own works its own.
    for w in CadWork::ALL {
        assert!(works_of(module_of_work(w)).contains(&w));
    }
    assert_eq!(works_of(Module::Architecture), [CadWork::Architecture]);
    assert_eq!(works_of(Module::Map), [CadWork::Map]);
    assert_eq!(works_of(Module::Road), [CadWork::Road]);
    assert!(works_of(Module::Gis).is_empty());
    assert_eq!(
        Module::ALL.map(type_of),
        [
            Workspace::Cad,
            Workspace::Cad,
            Workspace::Gis,
            Workspace::Cad
        ]
    );
}

#[test]
fn a_module_program_takes_only_its_own_module_and_one_file() {
    assert_eq!(fixed_launch(Module::Gis, os(&[])), Ok(None));
    assert_eq!(
        fixed_launch(Module::Gis, os(&["/a/ada.kcad"])),
        Ok(Some(PathBuf::from("/a/ada.kcad")))
    );
    assert_eq!(
        fixed_launch(Module::Gis, os(&["--modul", "cbs", "/a/ada.kcad"])),
        Ok(Some(PathBuf::from("/a/ada.kcad")))
    );
    let e =
        fixed_launch(Module::Gis, os(&["--modul=mimari", "x.kcad"])).expect_err("another module");
    assert!(
        e.contains("kentos-gis yalnız KentOS GIS olarak çalışır")
            && e.contains("kentos-mimari")
            && e.contains("--modul mimari"),
        "{e}"
    );
    let e = fixed_launch(Module::Map, os(&["a.kcad", "b.kcad"])).expect_err("two files");
    assert!(e.contains("“b.kcad” fazla"), "{e}");
}

#[test]
fn a_module_shows_its_own_recent_files_and_those_of_no_module() {
    let mut app = app_with_drawing();
    app.recent.add_in(
        std::path::Path::new("/p/bina.kcad"),
        "1".into(),
        Some(Module::Architecture),
    );
    app.recent.add_in(
        std::path::Path::new("/p/pafta.kcad"),
        "2".into(),
        Some(Module::Map),
    );
    app.recent
        .add_in(std::path::Path::new("/p/eski.kcad"), "3".into(), None);
    app.recent.add_in(
        std::path::Path::new("/p/mahalle.kcad"),
        "4".into(),
        Some(Module::Gis),
    );
    let names = |app: &App| {
        app.shown_recent()
            .iter()
            .map(|r| r.name.clone())
            .collect::<Vec<_>>()
    };
    assert_eq!(
        names(&app),
        ["mahalle.kcad", "eski.kcad", "pafta.kcad", "bina.kcad"],
        "kentos-cad shows every one"
    );
    app.program = Some(Module::Map);
    assert_eq!(names(&app), ["eski.kcad", "pafta.kcad"]);
    app.program = Some(Module::Gis);
    assert_eq!(names(&app), ["mahalle.kcad", "eski.kcad"]);
    app.program = Some(Module::Architecture);
    assert_eq!(names(&app), ["eski.kcad", "bina.kcad"]);
}

/// Every CAD work belongs to a module (`module_of_work` never needs its fallback).
#[test]
fn every_cad_work_is_a_modules() {
    for w in [CadWork::Architecture, CadWork::Map, CadWork::Road] {
        assert!(
            Module::of_project(Some(Workspace::Cad.id()), Some(w.id())).is_some(),
            "{w:?} has no module"
        );
    }
}
