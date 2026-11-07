//! Sınır grafiği (parcel_boundaries_window.rs, docs/adr/2000) on the shared cases' drawings of `cad.parcels.boundaries`:
//! opened from Parsel alan denetimi on the parcel layer, it counts the corners and boundaries as the case does; on the
//! block with problems it lists the T junction, the split corners and the overlap, and Sorunluları seç selects their
//! parcels; nothing is written to the drawing. And its pictures.

use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::parcel_boundaries_window::{Event as B, Show, rows};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.parcels.boundaries.json")).expect("cases")
}

/// The drawing of the case named `name` (none: the file's own), open in Harita.
fn opened(name: Option<&str>) -> App {
    let c = cases();
    let setup = match name {
        Some(n) => c["cases"].as_array().expect("cases").iter().find(|x| x["name"] == n).expect("the case")["setup"].clone(),
        None => c["setup"].clone(),
    };
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(setup).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

const PROBLEMS: &str = "sorunlu ada: T birleşimi, 0,4 µm ayrık köşeler, bir yanda iki parsel";

#[test]
fn the_clean_block_s_graph_from_the_parcel_check() {
    let mut app = opened(None);
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("parcels.check"));
    let _ = app.update(Message::ParcelBoundaries(B::Open));
    assert_eq!(app.dialog, Some(Dialog::ParcelBoundaries));
    let g = app.parcel_boundaries.answer.clone().expect("an answer").expect("read");
    let want = &cases()["cases"][0]["steps"][0]["result"]["output"];
    assert_eq!(g.corners.len(), want["corners"].as_array().map_or(0, Vec::len));
    assert_eq!(g.boundaries.len(), want["boundaries"].as_array().map_or(0, Vec::len));
    assert_eq!((Value::from(g.shared), Value::from(g.outer)), (want["shared"].clone(), want["outer"].clone()));
    assert!(g.problems.is_empty());
    let first = &rows(&g, Show::Boundaries, &app.format())[0];
    assert_eq!(first[0], "S1");
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
}

#[test]
fn the_problems_are_listed_and_their_parcels_selected() {
    let mut app = opened(Some(PROBLEMS));
    let _ = app.update(Message::Run("parcels.check"));
    let _ = app.update(Message::ParcelBoundaries(B::Open));
    let g = app.parcel_boundaries.answer.clone().expect("an answer").expect("read");
    let kinds: Vec<&str> = g.problems.iter().map(|p| p.kind.as_str()).collect();
    assert_eq!(kinds, ["missingVertex", "missingVertex", "missingVertex", "splitCorner", "splitCorner", "overlap"]);
    let _ = app.update(Message::ParcelBoundaries(B::Show(Show::Problems)));
    let shown = rows(&g, Show::Problems, &app.format());
    assert_eq!(shown[0][0], "Komşuda eksik köşe");
    let _ = app.update(Message::ParcelBoundaries(B::Select));
    assert_eq!(app.dialog, None);
    assert_eq!(app.selection.ids().len(), 7, "every parcel of the block is in a problem");
}

/// `.run/gui-evidence/sinir-grafigi-2000/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib parcel_boundaries_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/sinir-grafigi-2000");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (case, show, file) in [(None, Show::Boundaries, "1-sinirlar"), (Some(PROBLEMS), Show::Problems, "2-sorunlar")] {
            let mut app = opened(case);
            let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Run("parcels.check"));
            let _ = app.update(Message::ParcelBoundaries(B::Open));
            let _ = app.update(Message::ParcelBoundaries(B::Show(show)));
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let path = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&path).expect("writes");
            println!("{}", path.display());
        }
    }
}
