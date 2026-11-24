//! Parsel alan çizelgesi (parcel_report_window.rs, docs/adr/9306) on the shared cases' drawing of `cad.parcels.report`:
//! the parcel layer's report as the shared case gives it, the choices read it again, a selection lists only its areas,
//! CSV kaydet writes the table's words, Tabloyu yerleştir hangs the table from the cursor and a click writes it in one
//! undo step. Harita offers it; Mimari, Yol and CBS do not. And its pictures.

use crate::map;
use crate::map::MapState;
use crate::map_ext::*;
use kentos_commands::Entity;
use kentos_domain::Slot;
use kentos_launch::Module;
use serde_json::Value;

use crate::app::{App, Message};
use crate::parcel_report_window::{Event as R, Names};
use crate::viewport::Event;

fn cases() -> Value {
    serde_json::from_str(include_str!(
        "../../../../fixtures/commands/v1/cad.parcels.report.json"
    ))
    .expect("cases")
}

fn opened(program: Option<Module>) -> App {
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = program;
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// The shared case's output named so.
fn case_output(prefix: &str) -> Value {
    let all = cases();
    let case = all["cases"]
        .as_array()
        .expect("cases")
        .iter()
        .find(|c| c["name"].as_str().is_some_and(|n| n.starts_with(prefix)));
    case.expect("the case")["steps"][0]["result"]["output"].clone()
}

fn tables(app: &App) -> Vec<kentos_commands::TableEntity> {
    app.document
        .as_ref()
        .map(|d| {
            d.model
                .entities()
                .filter_map(|e| {
                    if let Entity::Table(t) = e {
                        Some(t.clone())
                    } else {
                        None
                    }
                })
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn the_parcel_layer_s_report_is_the_shared_case_s_and_the_choices_read_it_again() {
    let mut app = opened(Some(Module::Map));
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::Run("map.parcelReport"));
    assert_eq!(
        app.dialog,
        Some(crate::map::dialog(map::MapDialogKind::ParcelReport))
    );
    let r = app
        .modules
        .map()
        .parcel_report
        .answer
        .clone()
        .expect("an answer")
        .expect("listed");
    let want = case_output("plan: parsel katmanının");
    assert_eq!(
        serde_json::to_value(&r.cells).expect("cells"),
        want["cells"]
    );
    assert_eq!((r.places, r.named, r.numbered), (16, 3, 13));
    let _ = app.update(Message::from(map::Event::ParcelReport(R::Names(
        Names::Numbers,
    ))));
    let _ = app.update(Message::from(map::Event::ParcelReport(R::Prefix(
        "K".into(),
    ))));
    let _ = app.update(Message::from(map::Event::ParcelReport(R::Start(101.0))));
    let r = app
        .modules
        .map()
        .parcel_report
        .answer
        .clone()
        .expect("an answer")
        .expect("listed");
    assert_eq!(
        serde_json::to_value(&r.cells).expect("cells"),
        case_output("hepsine yeni numara: K öneki")["cells"]
    );
    let _ = app.update(Message::from(map::Event::ParcelReport(R::Lengths(false))));
    let r = app
        .modules
        .map()
        .parcel_report
        .answer
        .clone()
        .expect("an answer")
        .expect("listed");
    assert_eq!(
        r.cells[1],
        ["Nokta", "Y (m)", "X (m)"],
        "no edges, no radius"
    );
    assert_eq!(
        app.document.as_ref().map(|d| d.model.revision()),
        revision,
        "nothing written"
    );
}

#[test]
fn a_selection_lists_its_areas_only() {
    let mut app = opened(Some(Module::Map));
    // The arc parcel and a line: the line is no area.
    app.selection.set([Slot(3), Slot(5)]);
    let _ = app.update(Message::Run("map.parcelReport"));
    let r = app
        .modules
        .map()
        .parcel_report
        .answer
        .clone()
        .expect("an answer")
        .expect("listed");
    assert_eq!(r.parcels.len(), 1);
    assert_eq!(r.parcels[0].heading, "101 ada 3 parsel (Merkez)");
    assert_eq!(r.cells[1].last().map(String::as_str), Some("Yarıçap (m)"));
}

#[test]
fn csv_kaydet_writes_the_table_s_words() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::Run("map.parcelReport"));
    let dir = std::env::temp_dir().join(format!("kentos-parsel-cizelge-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("cizelge.csv");
    app.parcel_report_save(file.clone());
    let text = std::fs::read_to_string(&file).expect("written");
    let lines: Vec<&str> = text.lines().collect();
    assert_eq!(
        lines.len(),
        case_output("plan: parsel katmanının")["cells"]
            .as_array()
            .expect("cells")
            .len()
    );
    assert_eq!(lines[1], "Nokta;Y (m);X (m);Kenar (m);Yarıçap (m)");
    assert_eq!(lines[3], "P101;487000.000;4420015.000;20.000;");
    assert!(text.contains("Tapu alanı (m²);;;;301,2"), "{text}");
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn tabloyu_yerlestir_hangs_the_table_and_a_click_writes_it_in_one_step() {
    let mut app = opened(Some(Module::Map));
    let _ = app.update(Message::WindowResized(iced::Size::new(1440.0, 900.0)));
    let _ = app.update(Message::Run("view.zoomExtents"));
    let _ = app.update(Message::Run("map.parcelReport"));
    let words = app
        .modules
        .map()
        .parcel_report
        .answer
        .clone()
        .expect("an answer")
        .expect("listed")
        .cells;
    let _ = app.update(Message::from(map::Event::ParcelReport(R::Place)));
    assert_eq!(app.dialog, None, "the window gives way to the placement");
    assert!(tables(&app).is_empty(), "nothing written before the click");
    let [sx, sy] = app
        .viewport
        .camera
        .world_to_screen(kentos_interaction::Vec2::new(487150.0, 4420150.0));
    let at = iced::Point::new(sx as f32, sy as f32);
    let _ = app.update(Message::Viewport(Event::Moved(at)));
    let _ = app.update(Message::Viewport(Event::Pressed(at)));
    let _ = app.update(Message::Viewport(Event::Released(at)));
    let written = tables(&app);
    assert_eq!(written.len(), 1, "one table");
    assert_eq!(written[0].cells, words, "the window's words");
    assert_eq!(written[0].base.layer_id, "tablo", "the active layer");
    assert_eq!(app.selection.ids().len(), 1, "the table is selected");
    let _ = app.update(Message::Run("edit.undo"));
    assert!(tables(&app).is_empty(), "one undo step");
}

#[test]
fn harita_offers_it_mimari_yol_and_cbs_do_not() {
    for (module, yes) in [
        (Module::Map, true),
        (Module::Gis, false),
        (Module::Road, false),
        (Module::Architecture, false),
    ] {
        assert_eq!(
            opened(Some(module)).offers("map.parcelReport"),
            yes,
            "{module:?}"
        );
    }
}

/// `.run/gui-evidence/parsel-alan-cizelgesi-9306/`: the window on the shared cases' parcels in two themes and sizes,
/// and the placed table on the drawing:
///
/// ```text
/// RUST_MIN_STACK=67108864 KENTOS_SNAPSHOT_BACKEND=wgpu cargo test -p kentos-desktop-map --lib parcel_report_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/parsel-alan-cizelgesi-9306");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [
        ("light", Size::new(1440.0, 900.0), "acik-genis"),
        ("dark", Size::new(1100.0, 650.0), "koyu-dar"),
    ] {
        let mut app = opened(Some(Module::Map));
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        // Harita's Plan tab: the button beside Parsel alan denetimi, behind the window.
        app.tab = "plan";
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("map.parcelReport"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("pencere-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes");
        println!("{}", file.display());
        // The table written beside the parcels, the drawing fitted to both.
        let _ = app.update(Message::from(map::Event::ParcelReport(R::Place)));
        let [sx, sy] = app
            .viewport
            .camera
            .world_to_screen(kentos_interaction::Vec2::new(487075.0, 4420085.0));
        let at = iced::Point::new(sx as f32, sy as f32);
        let _ = app.update(Message::Viewport(Event::Moved(at)));
        // The table hanging from the cursor before the click.
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("yerlestir-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::Viewport(Event::Pressed(at)));
        let _ = app.update(Message::Viewport(Event::Released(at)));
        // The parcels of the table and the table itself fitted to the view, nothing selected.
        app.selection
            .set([Slot(1), Slot(2), Slot(3), Slot(4), Slot(17)]);
        let _ = app.update(Message::Run("view.zoomSelection"));
        let _ = app.update(Message::Run("edit.deselect"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("tablo-{name}.png"));
        snapshot
            .render(app.view(), &app.theme())
            .save(&file)
            .expect("writes");
        println!("{}", file.display());
    }
}
