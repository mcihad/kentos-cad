//! Mekânsal istatistik in the desktop's window (docs/adr/0238) over the
//! district (fixtures/interaction/v1/spatial-stats.kcad): Yön dağılımı
//! writes an ellipse a kind of accident, undone whole; Moran I gives its
//! table without touching the drawing; Sıcak nokta writes every block's copy
//! in its class's colour; DBSCAN numbers the accidents' clusters.

use kentos_contracts::Entity;
use serde_json::json;

use super::RunStatus;
use crate::app::App;
use crate::stats_scenes::{clusters, ellipses, hot, moran, open_stats, window};

fn run(app: &mut App) -> RunStatus {
    super::surface_tests::run(app)
}

fn district() -> App {
    let (mut app, _) = App::boot(None);
    open_stats(&mut app);
    app
}

fn on_layer<'a>(app: &'a App, name: &str) -> Vec<&'a Entity> {
    let doc = &app.document.as_ref().expect("open").model;
    let id = doc
        .layers()
        .leaves()
        .into_iter()
        .find(|l| l.name == name)
        .map(|l| l.id.clone())
        .unwrap_or_else(|| panic!("{name}"));
    doc.entities().filter(|e| e.base().layer_id == id).collect()
}

#[test]
fn the_ellipses_come_a_kind_and_go_back_whole() {
    let mut app = district();
    window(
        &mut app,
        "processing.run.stats.directionalDistribution",
        &ellipses(),
    );
    match run(&mut app) {
        RunStatus::Ok { text, undo, .. } => {
            assert!(undo);
            assert_eq!(text, "3 grubun yön dağılımı elipsi yazıldı (155 nesne).");
        }
        s => panic!("{s:?}"),
    }
    let made = on_layer(&app, "Yön dağılımı");
    let kinds: Vec<&str> = made
        .iter()
        .map(|e| e.base().attrs["Grup"].as_str())
        .collect();
    assert_eq!(kinds, ["Araç", "Bisiklet", "Yaya"]);
    assert!(made.iter().all(|e| matches!(e, Entity::Ellipse(_))));
    let doc = &mut app.document.as_mut().expect("open").model;
    assert_eq!(doc.undo().as_deref(), Some("Yön dağılımı"));
    assert!(
        doc.layers()
            .leaves()
            .iter()
            .all(|l| l.name != "Yön dağılımı")
    );
}

#[test]
fn morans_table_leaves_the_drawing_as_it_was() {
    let mut app = district();
    let before = app
        .document
        .as_ref()
        .expect("open")
        .model
        .entities()
        .count();
    window(&mut app, "processing.run.stats.moransI", &moran());
    match run(&mut app) {
        RunStatus::Ok {
            text, undo, table, ..
        } => {
            assert!(!undo);
            assert!(
                text.starts_with("Moran I 0.") && text.ends_with("kümelenmiş."),
                "{text}"
            );
            let t = table.expect("the table");
            assert_eq!(t.columns, ["Ölçü", "Değer"]);
            assert_eq!(t.rows[0], ["Nesne sayısı", "36"]);
            assert_eq!(t.rows[6], ["Desen", "Kümelenmiş"]);
        }
        s => panic!("{s:?}"),
    }
    assert_eq!(
        app.document
            .as_ref()
            .expect("open")
            .model
            .entities()
            .count(),
        before
    );
}

#[test]
fn every_block_is_copied_in_its_class_colour() {
    let mut app = district();
    window(&mut app, "processing.run.stats.hotSpot", &hot());
    match run(&mut app) {
        RunStatus::Ok { text, undo, .. } => {
            assert!(undo);
            assert!(text.starts_with("36 nesne yazıldı: "), "{text}");
        }
        s => panic!("{s:?}"),
    }
    let copies = on_layer(&app, "Sıcak noktalar");
    assert_eq!(copies.len(), 36);
    let palette = [
        "#2166AC", "#67A9CF", "#D1E5F0", "#D9D9D9", "#FDDBC7", "#EF8A62", "#B2182B",
    ];
    for e in &copies {
        let b = e.base();
        let class: i32 = b.attrs["Güven sınıfı"].parse().expect("a class");
        assert_eq!(b.color.as_deref(), Some(palette[(class + 3) as usize]));
        assert!(b.attrs.contains_key("Ada") && b.attrs.contains_key("z puanı"));
    }
    assert!(copies.iter().any(|e| e.base().attrs["Güven sınıfı"] == "3"));
}

#[test]
fn the_accidents_clusters_are_numbered() {
    let mut app = district();
    window(&mut app, "processing.run.stats.dbscan", &clusters());
    let text = match run(&mut app) {
        RunStatus::Ok { text, .. } => text,
        s => panic!("{s:?}"),
    };
    assert!(
        text.starts_with("155 nesne ") && text.contains(" kümeye ayrıldı; "),
        "{text}"
    );
    let copies = on_layer(&app, "Kümeler (DBSCAN)");
    assert_eq!(copies.len(), 155);
    let noise = copies
        .iter()
        .filter(|e| e.base().attrs["Küme"] == "0")
        .count();
    assert!(noise > 0 && noise < 60, "{noise}");
    assert!(
        copies
            .iter()
            .filter(|e| e.base().attrs["Küme"] == "0")
            .all(|e| e.base().color.as_deref() == Some("#BDBDBD"))
    );
    // The window keeps its values for the next run.
    let w = app.processing.dialog.as_ref().expect("the window");
    assert_eq!(w.values["radius"], json!(30));
}
