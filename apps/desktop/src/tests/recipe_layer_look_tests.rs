//! Tariften üret's added layer with the recipe layer's transparency, blending mode, type and Yazdır (İş grafiği,
//! project_graph_window.rs; docs/adr/4440), on the shared cases' road project of `road.project.produce`: the plan says
//! what the layer takes before anything is written, Üret adds it so and the window says it again; a recipe whose point
//! layer would take the zero line's lines is refused in the plan, Üret does not run. And its pictures.

use serde_json::Value;

use crate::app::{App, Message};
use crate::project_graph_window::{Event as G, plan_lines, produced_lines};
use crate::project_graph_window_tests::other_project;

fn recipe(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/recipes/v1").join(name)
}

/// The layer definition the recipe file keeps for its first drawing.
fn definition(name: &str) -> Value {
    let recipe: Value = serde_json::from_str(&std::fs::read_to_string(recipe(name)).expect("the recipe")).expect("JSON");
    recipe["nodes"].as_array().expect("nodes").iter().find_map(|n| n.get("recipe")).expect("a drawing")["layer"].clone()
}

/// The plan says the layer to be added with what it takes beyond its style; nothing is written before Üret. Üret adds
/// it opacity 0.6 in the multiply mode, a line layer not printed, as the recipe keeps them; the window says it again.
#[test]
fn tariften_uret_adds_the_layer_with_its_transparency_type_and_print() {
    let def = definition("saydam-katmanli.json");
    let mut b = other_project();
    let _ = b.update(Message::ProjectGraph(G::Open));
    let revision = b.document.as_ref().map(|d| d.model.revision());
    let _ = b.update(Message::ProjectGraph(G::Picked(Some(recipe("saydam-katmanli.json")))));
    let lines = plan_lines(b.project_graph.recipe.as_ref().expect("the recipe shown"));
    let planned = "“Kazı sınırı” katmanı bu projede yok; tarifteki stiliyle eklenecek. Tarifteki gibi saydamlığı %40, \
                   karışım kipi Çoğalt, türü Çizgi ve yazdırılmaz.";
    assert!(lines.iter().any(|(_, l)| l == planned), "{lines:?}");
    assert_eq!(b.document.as_ref().map(|d| d.model.revision()), revision, "nothing written before Üret");
    let _ = b.update(Message::ProjectGraph(G::Confirm));
    let out = b.project_graph.produced.clone().expect("produced");
    let done = "“Kazı sınırı” katmanı tarifteki stiliyle eklendi. Tarifteki gibi saydamlığı %40, karışım kipi Çoğalt, \
                türü Çizgi ve yazdırılmaz.";
    let said = produced_lines(&out, &b.project_graph.looks);
    assert!(said.iter().any(|(_, l)| l == done), "{said:?}");
    let model = &b.document.as_ref().expect("a drawing").model;
    let layers = model.layers();
    let added = layers.get(&out.layers[0].to).expect("added");
    assert_eq!(serde_json::to_value(&added.compose).expect("JSON"), def["compose"]);
    assert_eq!(serde_json::to_value(added.geometry).expect("JSON"), def["geometry"]);
    assert_eq!(added.plot, Some(false));
    // Düz under Tümsek: one closed line; Teras under Set: two open lines (road_terrain_zeroline_cases.py's).
    assert_eq!(model.by_layer(&added.id).count(), 3, "the zero lines' polylines on the line layer");
}

/// A recipe whose point layer would take the zero line's lines: the plan says why, and Üret does not run.
#[test]
fn a_point_layer_for_lines_is_refused_in_the_plan() {
    let mut b = other_project();
    let _ = b.update(Message::ProjectGraph(G::Open));
    let revision = b.document.as_ref().map(|d| d.model.revision());
    let _ = b.update(Message::ProjectGraph(G::Picked(Some(recipe("nokta-katmanli.json")))));
    let r = b.project_graph.recipe.as_ref().expect("the recipe shown");
    let why = r.plan.as_ref().expect_err("refused");
    assert!(why.contains("“Kazı noktaları” bir nokta katmanı; çoklu çizgi bu katmana yazılmaz."), "{why}");
    let _ = b.update(Message::ProjectGraph(G::Confirm));
    assert_eq!(b.document.as_ref().map(|d| d.model.revision()), revision, "nothing written");
    let names: Vec<String> =
        b.document.as_ref().expect("a drawing").model.layers().leaves().iter().map(|n| n.name.clone()).collect();
    assert!(!names.iter().any(|n| n == "Kazı noktaları"), "{names:?}");
}

/// `.run/gui-evidence/tarif-katman-gorunusu-4440/`: the picked recipe before Üret (the layer to be added with its
/// transparency, type and Yazdır), after Üret, and the point layer refused in the plan.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib recipe_layer_look_tests::look_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn look_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tarif-katman-gorunusu-4440");
    std::fs::create_dir_all(&out).expect("a folder");
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
        for (k, (file, done, what)) in [
            ("saydam-katmanli.json", false, "plan"),
            ("saydam-katmanli.json", true, "uretildi"),
            ("nokta-katmanli.json", false, "nokta-ret"),
        ]
        .into_iter()
        .enumerate()
        {
            let _ = app.update(Message::ProjectGraph(G::Picked(Some(recipe(file)))));
            if done {
                let _ = app.update(Message::ProjectGraph(G::Confirm));
            }
            snapshot.settle(&mut app, App::view, &mut update);
            let shot = out.join(format!("{}-{what}-{name}.png", k + 1));
            snapshot.render(app.view(), &app.theme()).save(&shot).expect("writes");
            println!("{}", shot.display());
            if !done {
                let _ = app.update(Message::ProjectGraph(G::Cancel));
            }
        }
    }
}
