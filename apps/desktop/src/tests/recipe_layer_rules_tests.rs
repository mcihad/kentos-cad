//! Tariften üret's added layer with the recipe layer's value combinations, record rules, indexes and forms, and the
//! choice that gives a layer found here the recipe's fields it lacks (İş grafiği, project_graph_window.rs;
//! docs/adr/5240), on the shared cases' road project of `road.project.produce`. And their pictures.

use crate::road;
use serde_json::Value;

use crate::app::{App, Message};
use crate::project_graph_window::{Event as G, plan_lines};
use crate::project_graph_window_tests::other_project;

fn recipe(name: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/recipes/v1").join(name)
}

fn definition(name: &str) -> Value {
    let recipe: Value = serde_json::from_str(&std::fs::read_to_string(recipe(name)).expect("the recipe")).expect("JSON");
    recipe["nodes"].as_array().expect("nodes").iter().find_map(|n| n.get("recipe")).expect("a drawing")["layer"].clone()
}

/// The plan says the combination, rule, index and form the layer takes; Üret adds them, the form as a draft editing it.
#[test]
fn the_added_layer_takes_its_combination_rule_index_and_form() {
    let def = definition("tanimli-katmanli.json");
    let mut b = other_project();
    let _ = b.update(Message::Road(road::Event::ProjectGraph(G::Open)));
    let _ = b.update(Message::Road(road::Event::ProjectGraph(G::Picked(Some(
        recipe("tanimli-katmanli.json"),
    )))));
    let lines = plan_lines(
        b.road
            .project_graph
            .recipe
            .as_ref()
            .expect("the recipe shown"),
    );
    let more = "1 değer bileşimi, 1 kayıt kuralı ve 1 dizin de tarifteki gibi. “Sınır formu” formu taslak olarak.";
    assert!(lines.iter().any(|(_, l)| l.contains(more)), "{lines:?}");
    let _ = b.update(Message::Road(road::Event::ProjectGraph(G::Confirm)));
    let out = b.road.project_graph.produced.clone().expect("produced");
    let model = &b.document.as_ref().expect("a drawing").model;
    let layers = model.layers();
    let added = layers.get(&out.layers[0].to).expect("added");
    assert_eq!(serde_json::to_value(&added.contingents).expect("JSON"), def["contingents"]);
    assert_eq!(serde_json::to_value(&added.rules).expect("JSON"), def["rules"]);
    assert_eq!(serde_json::to_value(&added.indexes).expect("JSON"), def["indexes"]);
    let forms = &model.settings().forms;
    assert_eq!(forms.len(), 1);
    assert_eq!((forms[0].layer.as_str(), forms[0].releases.len(), forms[0].enforce), (out.layers[0].to.as_str(), 0, false));
}

/// Bulunan katman: the choice shows for a recipe whose layers hold fields; checked, the plan and Üret give the layer of
/// the name here the recipe's fields it lacks, after its own.
#[test]
fn add_fields_gives_the_found_layer_what_it_lacks() {
    let mut b = other_project();
    let _ = b.update(Message::Road(road::Event::ProjectGraph(G::Open)));
    let _ = b.update(Message::Road(road::Event::ProjectGraph(G::Picked(Some(
        recipe("tanimli-adli.json"),
    )))));
    let before = b
        .road
        .project_graph
        .recipe
        .as_ref()
        .expect("shown")
        .warnings
        .clone();
    assert!(
        before.iter().any(|w| w.contains("kendi alanlarını korur")),
        "{before:?}"
    );
    let _ = b.update(Message::Road(road::Event::ProjectGraph(G::AddFields(true))));
    let r = b.road.project_graph.recipe.as_ref().expect("shown");
    assert!(r.add_fields);
    assert!(
        r.warnings
            .iter()
            .any(|w| w.contains("katmanının alanlarının sonuna eklenecek")),
        "{:?}",
        r.warnings
    );
    let _ = b.update(Message::Road(road::Event::ProjectGraph(G::Confirm)));
    let model = &b.document.as_ref().expect("a drawing").model;
    let names: Vec<String> = model
        .layers()
        .get("sifir")
        .expect("sifir")
        .fields
        .iter()
        .map(|f| f.name.clone())
        .collect();
    assert_eq!(names, ["Ad", "Tür", "Derinlik", "Zemin"]);
    assert!(model.settings().forms.is_empty());
}

/// `.run/gui-evidence/tarif-katman-tanimlari-5240/`: the plan of the layer added with its combination, rule, index and
/// form, and the found layer's choice checked.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib recipe_layer_rules_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tarif-katman-tanimlari-5240");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1440.0, 900.0), "acik-genis"), ("dark", Size::new(1100.0, 760.0), "koyu-dar")] {
        for (file, add, what) in [("tanimli-katmanli.json", false, "eklenen"), ("tanimli-adli.json", true, "eksik-alanlar")] {
            let mut app = other_project();
            let _ = app.settings.choose(&[("appearance.theme", Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::WindowResized(size));
            let _ = app.update(Message::Run("view.zoomExtents"));
            let _ = app.update(Message::Road(road::Event::ProjectGraph(G::Open)));
            let _ = app.update(Message::Road(road::Event::ProjectGraph(G::Picked(Some(
                recipe(file),
            )))));
            if add {
                let _ = app.update(Message::Road(road::Event::ProjectGraph(G::AddFields(true))));
            }
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            let mut update = |app: &mut App, m: Message| {
                let _ = app.update(m);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let shot = out.join(format!("{what}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&shot).expect("writes");
            println!("{}", shot.display());
        }
    }
}
