//! İş grafiği (project_graph_window.rs, docs/adr/1940) on the shared cases' map project of `road.project.graph`: the
//! zero line of Tümsek over Düz drawn and Pafta 1's map over it; one of Tümsek's heights set. The window, opened from
//! the project tree's panel, counts the stale drawing and the sheet that cannot be delivered, its row names the changed
//! source and the sheet, Güncel olmayanlar filters, Tarifi kaydet writes the graph; nothing is written to the drawing.
//! Drawn again, the chain is up to date. And its pictures.

use serde_json::{Value, json};

use crate::app::{App, Dialog, Message};
use crate::project_graph_window::{Event as G, Show, drawing_rows};

fn cases() -> Value {
    serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.project.graph.json")).expect("cases")
}

/// The cases' map project (with its sheets), the zero line drawn and Tümsek's corner set lower.
fn app() -> App {
    app_drawn().0
}

/// [`app`] and the drawing's revision right after the zero line was drawn (docs/adr/2840).
fn app_drawn() -> (App, String) {
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases()["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(kentos_launch::Module::Map);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::program_scenes::command(&mut app, "road.terrains.zeroline", json!({"base": "Düz", "comparison": "Tümsek", "layerId": "sifir"}));
    let drawn = app.document.as_ref().map(|d| d.model.revision().to_string()).unwrap_or_default();
    let surface = json!({"id": "t-10", "name": "Tümsek", "points": {"layers": ["tumsek"]},
                         "edits": [{"op": "setHeight", "x": 6000.0, "y": 0.0, "z": 119.0}]});
    crate::program_scenes::command(&mut app, "road.terrains.edit", json!({"operation": "set", "surface": surface}));
    (app, drawn)
}

#[test]
fn the_stale_zero_line_and_the_sheet_that_shows_it() {
    let (mut app, drawn) = app_drawn();
    let revision = app.document.as_ref().map(|d| d.model.revision());
    let _ = app.update(Message::ProjectGraph(G::Open));
    assert_eq!(app.dialog, Some(Dialog::ProjectGraph));
    let g = app.project_graph.answer.as_ref().expect("an answer").as_ref().expect("read").clone();
    assert_eq!((g.drawings, g.outdated, g.sheets, g.blocked), (1, 1, 1, 1));
    assert_eq!((g.epsg.as_deref(), g.system.as_deref()), (Some("EPSG:5254"), Some("TUREF / TM30")));
    let rows = drawing_rows(&g, false);
    assert_eq!(
        rows,
        [vec![
            "Sıfır hattı · Sıfır hattı".to_owned(),
            "zeroline v1".to_owned(),
            "Düz, Tümsek (değişti)".to_owned(),
            "1/1".to_owned(),
            // Docs/adr/2840: the revision the zero line's write made, as its record keeps it.
            drawn,
            "Güncel değil".to_owned(),
            "Pafta 1 › Tümsek haritası".to_owned(),
        ]]
    );
    let _ = app.update(Message::ProjectGraph(G::Show(Show::Outdated)));
    assert_eq!(app.project_graph.show, Show::Outdated);
    // Tarifi kaydet: the graph as JSON, every recipe in it.
    let dir = std::env::temp_dir().join(format!("kentos-is-grafigi-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let file = dir.join("is-grafigi.json");
    app.project_graph_save(file.clone());
    let saved: Value = serde_json::from_str(&std::fs::read_to_string(&file).expect("written")).expect("JSON");
    let drawing = saved["nodes"].as_array().expect("nodes").iter().find(|n| n["kind"] == "drawing").expect("the drawing");
    assert_eq!(drawing["recipe"]["input"], json!({"base": "t-5", "comparison": "Tümsek", "layerId": "sifir"}));
    let _ = std::fs::remove_dir_all(dir);
    assert_eq!(app.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
    // Drawn again: the chain is up to date.
    let _ = app.update(Message::ProjectGraph(G::Close));
    crate::program_scenes::command(&mut app, "road.project.rebuild", json!({}));
    let rebuilt = app.document.as_ref().map(|d| d.model.revision().to_string()).unwrap_or_default();
    let _ = app.update(Message::ProjectGraph(G::Open));
    let g = app.project_graph.answer.as_ref().expect("an answer").as_ref().expect("read");
    assert_eq!((g.outdated, g.blocked), (0, 0));
    assert!(drawing_rows(g, true).is_empty(), "nothing out of date");
    assert_eq!(drawing_rows(g, false)[0][4], rebuilt, "drawn again: the rebuild's revision");
}

#[test]
fn a_gis_project_has_no_graph() {
    let mut app = app();
    if let Some(doc) = app.document.as_mut() {
        let mut settings = doc.model.settings().clone();
        settings.cad_work = None;
        settings.road = None;
        doc.model.set_settings(settings);
    }
    let _ = app.update(Message::ProjectGraph(G::Open));
    let answer = app.project_graph.answer.clone().expect("an answer");
    assert!(answer.is_err_and(|why| why.contains("yol ve harita projesinde")), "refused in words");
}

/// `.run/gui-evidence/is-grafigi-1940/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib project_graph_window_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/is-grafigi-1940");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        app.docks.update(kentos_ui::widget::docking::Event::Selected(crate::app::Panel::RoadProject));
        app.docks.close(crate::app::Panel::Properties);
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        // The panel with its İş grafiği… button.
        let _ = app.update(Message::RibbonTab("terrain"));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-panel-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
        let _ = app.update(Message::ProjectGraph(G::Open));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("2-grafik-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// docs/adr/2840: İş grafiği with the Revizyon column, the zero line's record keeping the revision its write made;
/// `.run/gui-evidence/islem-surumu-2840/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib project_graph_window_tests::provenance_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn provenance_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/islem-surumu-2840");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::ProjectGraph(G::Open));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-grafik-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// The recipe of the map project's zero line, saved by Tarifi kaydet… to a scratch folder.
fn saved_recipe(dir: &std::path::Path) -> std::path::PathBuf {
    let mut a = app();
    let _ = a.update(Message::ProjectGraph(G::Open));
    std::fs::create_dir_all(dir).expect("a folder");
    let file = dir.join("tarif.json");
    a.project_graph_save(file.clone());
    file
}

/// The shared cases' Yol project of `road.project.produce` (road_terrain_zeroline_cases.py's: Düz, Tümsek, `sifir`)
/// opened, without drawings.
fn other_project() -> App {
    let cases: Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/road.project.produce.json")).expect("cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("reads");
    let (mut app, _) = App::boot(None);
    app.program = Some(kentos_launch::Module::Road);
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// Tariften üret… with `recipe` picked and drawn (Üret) as it is planned (docs/adr/3640).
fn produce(app: &mut App, recipe: &std::path::Path) {
    let _ = app.update(Message::ProjectGraph(G::Picked(Some(recipe.to_path_buf()))));
    let _ = app.update(Message::ProjectGraph(G::Confirm));
}

/// docs/adr/3240: the map project's recipe (its zero line, Tümsek over Düz) saved, then Tariften üret in another
/// project: the zero line is drawn there as its own drawing, current, with its recipe; the window says it above the
/// table; one undo takes it back; a second time it is left, the project has it.
#[test]
fn tariften_uret_draws_the_recipe_in_another_project() {
    use crate::exchange::words::Kind;
    use crate::project_graph_window::produced_lines;
    let dir = std::env::temp_dir().join(format!("kentos-tariften-uret-{}", std::process::id()));
    let file = saved_recipe(&dir);
    let mut b = other_project();
    let _ = b.update(Message::ProjectGraph(G::Open));
    assert_eq!(b.project_graph.answer.as_ref().and_then(|a| a.as_ref().ok()).map(|g| g.drawings), Some(0));
    produce(&mut b, &file);
    let out = b.project_graph.produced.clone().expect("produced");
    assert_eq!(out.produced.len(), 1);
    assert_eq!((out.produced[0].source.id.as_str(), out.produced[0].source.name.as_str()), ("t-5", "Düz"));
    assert_eq!(
        produced_lines(&out),
        [
            (Kind::Ok, "Tariften üretildi: 1 çizim, 1 nesne: cz-1 (Düz).".to_owned()),
            // docs/adr/3640: the recipe keeps its layer's definition; this project has the layer of that name.
            (Kind::Ok, "“Sıfır hattı” katmanı adıyla bulundu: Sıfır hattı.".to_owned()),
        ]
    );
    let g = b.project_graph.answer.as_ref().expect("an answer").as_ref().expect("read").clone();
    assert_eq!((g.drawings, g.outdated), (1, 0), "drawn here, current");
    assert_eq!(drawing_rows(&g, false)[0][1], "zeroline v1");
    let said = b.log.lines().map(|l| l.text.clone()).collect::<Vec<_>>().join("\n");
    assert!(said.contains("1 çizim tariften üretildi"), "{said}");
    // Once more: the project has it.
    produce(&mut b, &file);
    let again = b.project_graph.produced.clone().expect("answered");
    assert!(again.produced.is_empty());
    assert_eq!(
        produced_lines(&again)[1],
        (Kind::Info, "Tarifin cz-1 çizimi bırakıldı: Projede bu çizim var (aynı komut, ana kaynak ve katman); Yeniden üret ile güncellenir.".to_owned())
    );
    // One undo takes the drawing back.
    let _ = b.update(Message::ProjectGraph(G::Close));
    let _ = b.update(Message::Run("edit.undo"));
    let _ = b.update(Message::ProjectGraph(G::Open));
    assert_eq!(b.project_graph.answer.as_ref().and_then(|a| a.as_ref().ok()).map(|g| g.drawings), Some(0));
    let _ = std::fs::remove_dir_all(dir);
}

/// `.run/gui-evidence/tariften-uret-3240/`: İş grafiği in the other project after Tariften üret…
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib project_graph_window_tests::produce_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn produce_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tariften-uret-3240");
    std::fs::create_dir_all(&out).expect("a folder");
    let recipe = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/recipes/v1/eksik-kaynakli.json");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = other_project();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::ProjectGraph(G::Open));
        produce(&mut app, &recipe);
        let _ = app.update(Message::Run("view.zoomExtents"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-tariften-uret-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
        println!("{}", file.display());
    }
}

/// docs/adr/3640: the picked recipe is shown first: its plan says where each recipe layer goes (here “Kazı sınırı”,
/// which the project lacks, would be added with the recipe's look); nothing is written before Üret. A layer chosen for
/// the recipe layer is planned again and drawn on, and no layer is added; Vazgeç drops a picked recipe.
#[test]
fn tariften_uret_shows_the_recipe_s_layers_before_it_draws() {
    use crate::project_graph_window::plan_lines;
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/recipes/v1");
    let mut b = other_project();
    let _ = b.update(Message::ProjectGraph(G::Open));
    let revision = b.document.as_ref().map(|d| d.model.revision());
    let _ = b.update(Message::ProjectGraph(G::Picked(Some(fixtures.join("yeni-katmanli.json")))));
    let r = b.project_graph.recipe.as_ref().expect("the recipe shown");
    assert_eq!(r.layers, vec![("k-9".to_owned(), "Kazı sınırı".to_owned(), String::new())]);
    let lines = plan_lines(r);
    assert!(lines.iter().any(|(_, l)| l.contains("“Kazı sınırı” katmanı bu projede yok")), "{lines:?}");
    assert!(lines.iter().any(|(_, l)| l == "Üretilecek: 2 çizim: cz-1 (Düz), cz-2 (Teras)."), "{lines:?}");
    assert_eq!(b.document.as_ref().map(|d| d.model.revision()), revision, "nothing written before Üret");
    // Drawn on Drenaj instead: planned again, then drawn, no layer added.
    let _ = b.update(Message::ProjectGraph(G::Layer(0, "drenaj".into())));
    let lines = plan_lines(b.project_graph.recipe.as_ref().expect("still shown"));
    assert!(lines.iter().any(|(_, l)| l == "“Kazı sınırı” katmanının yerine seçilen: Drenaj."), "{lines:?}");
    let _ = b.update(Message::ProjectGraph(G::Confirm));
    assert!(b.project_graph.recipe.is_none());
    let out = b.project_graph.produced.clone().expect("produced");
    assert_eq!((out.produced.len(), out.layers[0].to.as_str()), (2, "drenaj"));
    let names: Vec<String> = b.document.as_ref().expect("a drawing").model.layers().leaves().iter().map(|n| n.name.clone()).collect();
    assert!(!names.iter().any(|n| n == "Kazı sınırı"), "{names:?}");
    // Vazgeç drops a picked recipe.
    let _ = b.update(Message::ProjectGraph(G::Picked(Some(fixtures.join("katman-adli.json")))));
    assert!(b.project_graph.recipe.is_some());
    let _ = b.update(Message::ProjectGraph(G::Cancel));
    assert!(b.project_graph.recipe.is_none());
}

/// `.run/gui-evidence/tarif-katmanlari-3640/`: the picked recipe before Üret (its layer to be added, then found by
/// name), and after.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib project_graph_window_tests::recipe_layer_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn recipe_layer_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tarif-katmanlari-3640");
    std::fs::create_dir_all(&out).expect("a folder");
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/recipes/v1");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = other_project();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::ProjectGraph(G::Open));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        for (k, (file, done)) in [("yeni-katmanli.json", false), ("katman-adli.json", false), ("yeni-katmanli.json", true)].into_iter().enumerate() {
            let _ = app.update(Message::ProjectGraph(G::Picked(Some(fixtures.join(file)))));
            if done {
                let _ = app.update(Message::ProjectGraph(G::Confirm));
            }
            snapshot.settle(&mut app, App::view, &mut update);
            let shot = out.join(format!("{}-{}-{name}.png", k + 1, if done { "uretildi" } else { file.trim_end_matches(".json") }));
            snapshot.render(app.view(), &app.theme()).save(&shot).expect("writes");
            println!("{}", shot.display());
            if !done {
                let _ = app.update(Message::ProjectGraph(G::Cancel));
            }
        }
    }
}

/// docs/adr/4040: a recipe whose layer keeps its groups, whole style and line type. The plan says the layer goes in the
/// project's group Yol (the recipe's YOL, folded), in a Toprak işleri added there, and SINIR is added; nothing is written
/// before Üret. Üret adds them so: the layer under Yol › Toprak işleri with the recipe's style as it is, SINIR among the
/// project's line types as the recipe defines it; the window says what was added.
#[test]
fn tariften_uret_adds_the_recipe_layer_in_its_groups_with_its_style() {
    use crate::project_graph_window::{plan_lines, produced_lines};
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/recipes/v1/grup-stilli.json");
    let recipe: Value = serde_json::from_str(&std::fs::read_to_string(&path).expect("the recipe")).expect("JSON");
    let def = recipe["nodes"].as_array().expect("nodes").iter().find_map(|n| n.get("recipe")).expect("a drawing")["layer"].clone();
    let mut b = other_project();
    let _ = b.update(Message::ProjectGraph(G::Open));
    let revision = b.document.as_ref().map(|d| d.model.revision());
    let _ = b.update(Message::ProjectGraph(G::Picked(Some(path.clone()))));
    let lines = plan_lines(b.project_graph.recipe.as_ref().expect("the recipe shown"));
    let planned = "“Kazı sınırı” katmanı bu projede yok; tarifteki stiliyle “Yol › Toprak işleri” grubuna eklenecek. \
                   “Toprak işleri” grubu ve “SINIR” çizgi türü de eklenecek.";
    assert!(lines.iter().any(|(_, l)| l == planned), "{lines:?}");
    assert_eq!(b.document.as_ref().map(|d| d.model.revision()), revision, "nothing written before Üret");
    let _ = b.update(Message::ProjectGraph(G::Confirm));
    let out = b.project_graph.produced.clone().expect("produced");
    let done = "“Kazı sınırı” katmanı tarifteki stiliyle “Yol › Toprak işleri” grubuna eklendi. “Toprak işleri” grubu ve \
                “SINIR” çizgi türü de eklendi.";
    assert!(produced_lines(&out).iter().any(|(_, l)| l == done), "{:?}", produced_lines(&out));
    let model = &b.document.as_ref().expect("a drawing").model;
    let id = out.layers[0].to.clone();
    let layers = model.layers();
    assert_eq!(serde_json::to_value(&layers.get(&id).expect("added").style).expect("JSON"), def["style"]);
    let parent = layers.place_of(&id).and_then(|(p, _)| p).expect("in a group");
    assert_eq!(layers.get(&parent).map(|n| n.name.as_str()), Some("Toprak işleri"));
    assert_eq!(layers.place_of(&parent).and_then(|(p, _)| p).as_deref(), Some("grup"), "in the project's Yol");
    let sinir = model.settings().line_types.iter().find(|t| t.name == "SINIR").expect("SINIR added");
    assert_eq!(serde_json::to_value(sinir).expect("JSON"), def["lineTypeDef"]);
}

/// `.run/gui-evidence/tarif-grubu-4040/`: the grouped, styled recipe before Üret, and the layer tree after it.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib project_graph_window_tests::recipe_group_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn recipe_group_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tarif-grubu-4040");
    std::fs::create_dir_all(&out).expect("a folder");
    let recipe = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/recipes/v1/grup-stilli.json");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        let mut app = other_project();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::ProjectGraph(G::Open));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let _ = app.update(Message::ProjectGraph(G::Picked(Some(recipe.clone()))));
        snapshot.settle(&mut app, App::view, &mut update);
        let shot = out.join(format!("1-plan-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&shot).expect("writes");
        println!("{}", shot.display());
        let _ = app.update(Message::ProjectGraph(G::Confirm));
        snapshot.settle(&mut app, App::view, &mut update);
        let shot = out.join(format!("2-uretildi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&shot).expect("writes");
        println!("{}", shot.display());
        let _ = app.update(Message::ProjectGraph(G::Close));
        // The layer tree given most of the dock, so the group added shows under Yol.
        let _ = app.update(Message::Dock(kentos_ui::widget::docking::Event::Shared(kentos_ui::widget::docking::Side::Right, vec![0.85, 0.15])));
        snapshot.settle(&mut app, App::view, &mut update);
        let shot = out.join(format!("3-katmanlar-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&shot).expect("writes");
        println!("{}", shot.display());
    }
}
