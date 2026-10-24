use super::*;

fn source(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/formats/v1")
        .join(name)
}

#[test]
fn cad_sources_open_without_new_project_and_save_only_as_native_files() {
    let dir = scratch("cad-direct-open");
    for name in [
        "dxf-units/mm.dxf",
        "dxf-units/inches.dxf",
        "dwg/mm.dwg",
        "ncz/01-her-tur.ncz",
    ] {
        let path = source(name);
        let original = std::fs::read(&path).expect("source");
        let mut s = Session::open(&path).expect("CAD opens directly");
        let summary = s.summary();
        assert_eq!(summary["settings"]["workspace"], "cad");
        assert!(
            summary["path"].is_null(),
            "foreign source is never the save path"
        );
        assert!(summary["openedCad"]["report"].is_object());
        assert!(!s.document().is_empty());
        assert!(s.undo().is_none(), "opening is the baseline");
        assert_eq!(s.save(None).expect_err("must choose path").code, "no_path");
        assert_eq!(
            s.save(Some(&path))
                .expect_err("must not overwrite source")
                .code,
            "invalid_file_path"
        );
        let target = dir.join(format!(
            "{}.kcad",
            path.file_stem().unwrap().to_string_lossy()
        ));
        let expected = s.document().to_snapshot_v2();
        s.save(Some(&target)).expect("native save");
        let back = Session::open(&target).expect("native reopen");
        assert_eq!(back.document().to_snapshot_v2(), expected);
        assert_eq!(std::fs::read(&path).expect("source unchanged"), original);
    }
    let _ = std::fs::remove_dir_all(dir);
}

/// A CAD drawing asked for without a unit is the wizard's first work, a building in metres at 1:100
/// (docs/adr/0626 §1); a millimetre one is a detail at 1:1.
#[test]
fn local_cad_defaults_to_a_building_in_metres() {
    let choices = kentos_headless::new_drawing(&json!({"workspace":"cad"})).expect("default CAD");
    assert_eq!(choices.drawing_unit, Some(kentos_commands::DrawingUnit::M));
    assert_eq!(choices.plot_scale, 100.0);
    assert_eq!(choices.srid, 0);
    let detail = kentos_headless::new_drawing(&json!({"workspace":"cad", "drawingUnit":"mm"}))
        .expect("a detail");
    assert_eq!(detail.plot_scale, 1.0);
    let survey =
        kentos_headless::new_drawing(&json!({"workspace":"cad", "srid":5256})).expect("map CAD");
    assert_eq!(survey.plot_scale, 1000.0);
    assert_eq!(survey.drawing_unit, None);
}
