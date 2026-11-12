//! Stok hacmi (stockpile_window.rs, docs/adr/3140) on the shared cases' drawing of `road.terrains.stockpile`: the
//! window takes the selected boundary; its answer is the shared cases' for the fitted plane, the lowest corner and a
//! typed height, and for the L; the typed height starts at the corners' lowest; nothing is written; Harita and Yol
//! offer it. And its pictures.

use crate::road;
use kentos_commands::TerrainsStockpiled;
use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Dialog, Message};
use crate::stockpile_window::{Base, Event as E};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../../fixtures/commands/v1/road.terrains.stockpile.json")).expect("cases")
}

fn opened(program: Option<Module>) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The output of the shared case named `name`.
fn output(name: &str) -> Value {
    cases()["cases"].as_array().expect("cases").iter().find(|c| c["name"] == name).expect("the case")["steps"][0]["result"]
        ["output"]
        .clone()
}

fn answer(app: &App) -> TerrainsStockpiled {
    app.road
        .stockpile
        .answer
        .clone()
        .expect("an answer")
        .expect("measured")
        .0
}

fn same(got: &TerrainsStockpiled, want: &Value) {
    for (key, v) in [
        ("cut", got.cut),
        ("fill", got.fill),
        ("total", got.total),
        ("area", got.area),
        ("covered", got.covered),
        ("cutArea", got.cut_area),
        ("fillArea", got.fill_area),
    ] {
        assert!((v - want[key].as_f64().expect("a number")).abs() < 1e-6, "{key}: {v} vs {}", want[key]);
    }
}

/// The pentagon is the first object after the surface's 256 points.
const PENTAGON: Slot = Slot(257);
const ELL: Slot = Slot(258);

#[test]
fn the_volume_is_the_shared_case_s() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    app.selection.set([PENTAGON]);
    let _ = app.update(Message::Run("terrain.stockpile"));
    assert_eq!(
        app.dialog,
        Some(Dialog::Road(road::RoadDialogKind::Stockpile))
    );
    assert_eq!(
        app.road.stockpile.boundary,
        cases()["setup"]["uids"][256].as_str().expect("a uid")
    );
    assert_eq!(app.road.stockpile.base, Base::Fit);
    same(
        &answer(&app),
        &output("Beşgen, köşelere oturan eğimli düzlem (en küçük kareler)"),
    );
    let _ = app.update(Message::Road(road::Event::Stockpile(E::Base(Base::Lowest))));
    same(
        &answer(&app),
        &output("Beşgen, en alçak köşe: yatay taban köşelerin en alçağında, yığın kazı"),
    );
    // A typed height starts at the corners' lowest, to the centimetre.
    let low = answer(&app)
        .corners
        .iter()
        .copied()
        .reduce(f64::min)
        .expect("corners");
    let _ = app.update(Message::Road(road::Event::Stockpile(E::Base(
        Base::Elevation,
    ))));
    assert!(
        (app.road.stockpile.elevation - (low * 100.0).round() / 100.0).abs() < 1e-12,
        "{}",
        app.road.stockpile.elevation
    );
    let _ = app.update(Message::Road(road::Event::Stockpile(E::Elevation(101.5))));
    same(&answer(&app), &output("Beşgen, verilen kot 101,5"));
    let ell = app
        .document
        .as_ref()
        .and_then(|d| d.model.uid(ELL))
        .expect("the L")
        .to_string();
    let _ = app.update(Message::Road(road::Event::Stockpile(E::Boundary(ell))));
    let _ = app.update(Message::Road(road::Event::Stockpile(E::Base(Base::Fit))));
    same(
        &answer(&app),
        &output("L (kapalı çoklu çizgi, iç bükey), eğimli düzlem"),
    );
    assert_eq!(
        app.document.as_ref().map(|d| d.model.revision()),
        revision,
        "nothing written"
    );
    let _ = app.update(Message::Road(road::Event::Stockpile(E::Close)));
    assert_eq!(app.dialog, None);
}

#[test]
fn harita_and_yol_offer_it() {
    for module in [Module::Map, Module::Road] {
        assert!(opened(Some(module)).offers("terrain.stockpile"), "{module:?}");
    }
}

/// `.run/gui-evidence/stok-hacmi-3140/`: the window over the pentagon with the fitted plane and with a typed height,
/// light and wide, dark and narrow; and the Arazi tab where it sits.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib stockpile_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/stok-hacmi-3140");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        app.selection.set([PENTAGON]);
        let _ = app.update(Message::Run("terrain.stockpile"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, stem: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        shot(&mut app, "1-egimli-duzlem");
        let _ = app.update(Message::Road(road::Event::Stockpile(E::Base(
            Base::Elevation,
        ))));
        let _ = app.update(Message::Road(road::Event::Stockpile(E::Elevation(101.5))));
        shot(&mut app, "2-kot");
    }
    let mut app = opened(Some(Module::Map));
    let size = Size::new(1920.0, 1000.0);
    let _ = app.update(Message::WindowResized(size));
    let _ = app.update(Message::Run("view.zoomExtents"));
    let _ = app.update(Message::RibbonTab("terrain"));
    let mut snapshot = Snapshot::new(size).expect("a renderer");
    let mut update = |app: &mut App, m: Message| {
        let _ = app.update(m);
    };
    snapshot.settle(&mut app, App::view, &mut update);
    let file = out.join("3-arazi-sekmesi.png");
    snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
    println!("{}", file.display());
}

/// Üçgenlenmiş (docs/adr/3940): the pentagon's and the L's answers are the shared cases', the base's triangles too; the
/// summary says the base by its triangles, not by a plane.
#[test]
fn the_triangulated_base_is_the_shared_case_s() {
    let mut app = opened(Some(Module::Map));
    app.selection.set([PENTAGON]);
    let _ = app.update(Message::Run("terrain.stockpile"));
    let _ = app.update(Message::Road(road::Event::Stockpile(E::Base(
        Base::Triangulated,
    ))));
    let got = answer(&app);
    let want = output("Beşgen, köşelerden üçgenlenmiş taban: üç üçgen, her biri kendi düzlemi");
    same(&got, &want);
    assert_eq!(got.plane, None);
    let tris: Vec<[[f64; 3]; 3]> = serde_json::from_value(want["triangles"].clone()).expect("triangles");
    assert_eq!(got.triangles.len(), tris.len());
    for (g, w) in got.triangles.iter().zip(&tris) {
        for k in 0..3 {
            assert_eq!([g[k][0], g[k][1]], [w[k][0], w[k][1]], "the same corners in the same order");
            assert!((g[k][2] - w[k][2]).abs() < 1e-9, "{g:?} {w:?}");
        }
    }
    let said = app.stockpile_lines_text();
    assert!(
        said.iter()
            .any(|l| l.starts_with("Taban: köşelerden 3 üçgen, eğimleri %")),
        "{said:?}"
    );
    let ell = app
        .document
        .as_ref()
        .and_then(|d| d.model.uid(ELL))
        .expect("the L")
        .to_string();
    let _ = app.update(Message::Road(road::Event::Stockpile(E::Boundary(ell))));
    same(
        &answer(&app),
        &output("L (iç bükey), üçgenlenmiş taban: girintide üçgen yok"),
    );
}

/// `.run/gui-evidence/stok-ucgenlenmis-3940/`: the window over the pentagon and the L with the triangulated base, light
/// and wide, dark and narrow.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib stockpile_window_tests::screens_triangulated -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens_triangulated() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/stok-ucgenlenmis-3940");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = opened(Some(Module::Map));
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        app.selection.set([PENTAGON]);
        let _ = app.update(Message::Run("terrain.stockpile"));
        let _ = app.update(Message::Road(road::Event::Stockpile(E::Base(
            Base::Triangulated,
        ))));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |app: &mut App, stem: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{stem}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        shot(&mut app, "1-besgen");
        let ell = app
            .document
            .as_ref()
            .and_then(|d| d.model.uid(ELL))
            .expect("the L")
            .to_string();
        let _ = app.update(Message::Road(road::Event::Stockpile(E::Boundary(ell))));
        shot(&mut app, "2-l");
    }
}
