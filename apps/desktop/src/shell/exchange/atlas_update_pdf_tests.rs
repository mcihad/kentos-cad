//! Atlası PDF yap, docs/adr/4400 (plan/map-cad.md MAP-18): with a previous record chosen, Yalnız değişen ve yeni
//! sayfalar writes the update's PDF of the pages the shared case names; the message says which of the atlas's pages it
//! holds and the record beside it says them too. And its picture.

use super::atlas_export::Event as A;
use super::atlas_tests::{app_with_atlas_of, previewed_against_the_record, window};
use super::tests::send;
use super::Event;
use crate::app::{App, Message};
use crate::files_testing::scratch;
use serde_json::Value;

const CASES: &str = include_str!("../../../../../fixtures/atlas/v1/cases.json");

/// The shared case's pages of the update's PDF.
fn written() -> Vec<u64> {
    let file: Value = serde_json::from_str(CASES).expect("the cases");
    let case = file["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with("Güncelleme PDF'i: yalnız")))
        .expect("the update PDF case")
        .clone();
    case["steps"][2]["result"]["output"]["written"].as_array().expect("written").iter().filter_map(Value::as_u64).collect()
}

#[test]
fn yalniz_degisen_sayfalar_writes_the_update_s_pdf() {
    let mut app = app_with_atlas_of("plain");
    let dir = scratch("atlas-guncelleme-pdf");
    previewed_against_the_record(&mut app, &dir);
    assert!(!window(&app).changed_only, "off until asked");
    send(&mut app, Event::Atlas(A::ChangedOnly(true)));
    assert!(window(&app).changed_only);
    send(&mut app, Event::Atlas(A::SavePicked(Some(dir.join("guncelleme.pdf")))));
    assert!(app.exchange.is_none(), "written and closed");
    let want = written();
    let numbers: Vec<String> = want.iter().map(u64::to_string).collect();
    let said = crate::files_testing::last_said(&app);
    assert!(said.contains(&format!("güncelleme, 100 sayfanın {}'i (sayfa {})", want.len(), numbers.join(", "))), "{said}");
    assert!(dir.join("guncelleme.pdf").exists());
    let record: Value =
        serde_json::from_str(&std::fs::read_to_string(dir.join("guncelleme.kaynaklar.json")).expect("its record")).expect("json");
    let got: Vec<u64> = record["written"].as_array().expect("written").iter().filter_map(Value::as_u64).collect();
    assert_eq!(got, want);
    assert_eq!(record["pages"].as_array().map(Vec::len), Some(100));
}

/// `.run/gui-evidence/atlas-guncelleme-pdf-4400/`: the window with the previous record and the box.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib exchange::atlas_update_pdf_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/atlas-guncelleme-pdf-4400");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 700.0), "koyu-dar")] {
        let mut app = app_with_atlas_of("plain");
        let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let dir = scratch(&format!("atlas-guncelleme-pdf-resim-{name}"));
        previewed_against_the_record(&mut app, &dir);
        send(&mut app, Event::Atlas(A::ChangedOnly(true)));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("pencere-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}
