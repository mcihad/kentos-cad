//! Uzaktan algılama in the desktop's window (docs/adr/0242) over the
//! valley's synthetic satellite image (fixtures/interaction/v1/remote.kcad):
//! Spektral indis writes its NDVI right above the image's layer, undone
//! whole; Denetimli sınıflandırma gives its five classes in their names'
//! natural order; Doğruluk analizi its matrix without touching the drawing;
//! Bantlara ayır one raster a band.

use kentos_contracts::Entity;
use serde_json::{Value, json};

use super::RunStatus;
use crate::app::App;
use crate::remote_scenes::{accuracy, layer, ndvi, open_remote, supervised};

fn run(app: &mut App) -> RunStatus {
    super::surface_tests::run(app)
}

fn valley() -> App {
    let (mut app, _) = App::boot(None);
    open_remote(&mut app);
    app
}

fn window(app: &mut App, tool: &'static str, values: &[(&str, Value)]) {
    crate::stats_scenes::window(app, tool, values);
}

fn layer_at(app: &App, name: &str) -> (String, usize) {
    let layers = app.document.as_ref().expect("open").model.layers();
    let id = layers
        .leaves()
        .into_iter()
        .find(|l| l.name == name)
        .map(|l| l.id.clone())
        .unwrap_or_else(|| panic!("{name}"));
    let (_, at) = layers.place_of(&id).expect("placed");
    (id, at)
}

#[test]
fn the_ndvi_goes_right_above_the_image_and_back_whole() {
    let mut app = valley();
    window(&mut app, "processing.run.remote.index", &ndvi());
    let text = match run(&mut app) {
        RunStatus::Ok { text, undo, .. } => {
            assert!(undo);
            text
        }
        s => panic!("{s:?}"),
    };
    assert!(
        text.starts_with("480 × 324 hücrelik raster; “") && text.contains("” yazıldı. NDVI -0."),
        "{text}"
    );
    let (_, at) = layer_at(&app, "Spektral indis");
    let (_, image) = layer_at(&app, "Görüntü (2024)");
    assert_eq!(at + 1, image);
    let doc = &mut app.document.as_mut().expect("open").model;
    assert_eq!(doc.undo().as_deref(), Some("Spektral indis"));
    assert!(
        doc.layers()
            .leaves()
            .iter()
            .all(|l| l.name != "Spektral indis")
    );
}

#[test]
fn the_classes_come_in_their_names_order_and_the_matrix_leaves_the_drawing() {
    let mut app = valley();
    window(&mut app, "processing.run.remote.supervised", &supervised());
    match run(&mut app) {
        RunStatus::Ok { text, table, .. } => {
            assert!(text.contains(" 5 sınıf, "), "{text}");
            let t = table.expect("the table");
            assert_eq!(
                t.columns,
                [
                    "Sınıf",
                    "Değer",
                    "Eğitim hücresi",
                    "Hücre sayısı",
                    "Alan (m²)"
                ]
            );
            let names: Vec<&str> = t.rows.iter().map(|r| r[0].as_str()).collect();
            assert_eq!(names, ["Mera", "Orman", "Su", "Tarım", "Yerleşim"]);
        }
        s => panic!("{s:?}"),
    }
    let _ = app.update(crate::app::Message::Processing(super::Event::Close));
    let before = app
        .document
        .as_ref()
        .expect("open")
        .model
        .entities()
        .count();
    window(&mut app, "processing.run.remote.accuracy", &accuracy());
    match run(&mut app) {
        RunStatus::Ok {
            text, undo, table, ..
        } => {
            assert!(!undo);
            assert!(
                text.starts_with("Genel doğruluk %") && text.contains(", kappa 0."),
                "{text}"
            );
            let t = table.expect("the matrix");
            assert_eq!(t.columns[0], "Sınıflandırılan \\ Referans");
            assert_eq!(
                t.rows.last().map(|r| r[0].as_str()),
                Some("Üretici doğruluğu (%)")
            );
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
fn every_band_has_its_raster() {
    let mut app = valley();
    let out = crate::files_testing::scratch("ua-bantlar").join("bant.tif");
    window(
        &mut app,
        "processing.run.remote.split",
        &[
            ("input", layer("goruntu")),
            ("output", json!(out.to_string_lossy())),
        ],
    );
    match run(&mut app) {
        RunStatus::Ok { text, .. } => {
            assert!(
                text.starts_with("4 bant ayrı rasterlere yazıldı: “"),
                "{text}"
            );
            assert!(text.ends_with("bant-b4.tif” (480 × 324 hücre)."), "{text}");
        }
        s => panic!("{s:?}"),
    }
    let doc = &app.document.as_ref().expect("open").model;
    let (id, _) = layer_at(&app, "Bantlar");
    let bands: Vec<_> = doc
        .entities()
        .filter(|e| e.base().layer_id == id)
        .filter_map(|e| match e {
            Entity::Raster(r) => Some(r.raster.bands),
            _ => None,
        })
        .collect();
    assert_eq!(bands, [1, 1, 1, 1]);
}
