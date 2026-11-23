//! The municipal report's 10th item, its losses (docs/adr/9173): a QML whose parts KentOS cannot take says each of them
//! on a line of its own after a headline with how many, the warning that only gathered them not said again.

use crate::files_testing::app_with_drawing;

fn qml(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/qml/v1").join(name)
}

#[test]
fn a_lossy_qmls_parts_are_said_one_by_one() {
    let mut app = app_with_drawing();
    let before = app.log.lines().count();
    app.qml_import_from("parsel".into(), Some(qml("kayipli.qml")));
    let said: Vec<String> = app.log.lines().skip(before).map(|l| l.text.clone()).collect();
    let head = said.iter().position(|l| l.starts_with("QML'den alınamayan ")).unwrap_or_else(|| panic!("{said:#?}"));
    let n: usize = said[head]
        .trim_start_matches("QML'den alınamayan ")
        .split(' ')
        .next()
        .and_then(|n| n.parse().ok())
        .unwrap_or_else(|| panic!("{}", said[head]));
    assert!(n >= 1, "{said:#?}");
    for i in 1..=n {
        assert!(said[head + i].starts_with(&format!("{i}. ")), "{said:#?}");
    }
    assert!(!said.iter().any(|l| l.contains("bazı parçaları alınamadı")), "not gathered again: {said:#?}");
}

#[test]
fn the_headline_names_the_file_and_the_way() {
    let mut app = app_with_drawing();
    let before = app.log.lines().count();
    app.say_losses("SLD", true, &["dolgu deseni yazılmadı".into(), "saydamlık yaklaşık".into()], Vec::new());
    let said: Vec<String> = app.log.lines().skip(before).map(|l| l.text.clone()).collect();
    assert_eq!(said, ["SLD'nin söyleyemediği 2 özellik:", "1. dolgu deseni yazılmadı", "2. saydamlık yaklaşık"]);
}

/// The picture (`.run/gui-evidence/kullanim-9173/`): Komut geçmişi after the lossy QML, its losses a line each.
#[test]
#[ignore = "pictures for the owner"]
fn screens() {
    crate::files_testing::usage_shots("9173", "01-qml-kayiplari", || {
        let mut app = app_with_drawing();
        app.qml_import_from("parsel".into(), Some(qml("kayipli.qml")));
        app.command_expanded = true;
        app.bottom_tab = crate::bottom::BottomTab::History;
        app
    });
}
