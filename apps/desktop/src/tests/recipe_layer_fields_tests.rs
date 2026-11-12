//! Tariften üret's added layer with the recipe layer's fields, the value lists they link and its subtypes (İş grafiği,
//! project_graph_window.rs; docs/adr/4860), on the shared cases' road project of `road.project.produce`: the plan says
//! what the layer takes before anything is written, Üret adds it so (the list too) and the window says it again; a
//! layer of the name here keeps its own fields and the plan says what of the recipe's it lacks. And its pictures.

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

/// The plan says the layer to be added with its fields and the list; nothing is written before Üret. Üret adds the
/// layer with the recipe's fields and subtypes and the project's list Sınır türü; the window says it again.
#[test]
fn tariften_uret_adds_the_layer_with_its_fields_list_and_subtypes() {
    let def = definition("alanli-katmanli.json");
    let mut b = other_project();
    let _ = b.update(Message::ProjectGraph(G::Open));
    let revision = b.document.as_ref().map(|d| d.model.revision());
    let _ = b.update(Message::ProjectGraph(G::Picked(Some(recipe("alanli-katmanli.json")))));
    let lines = plan_lines(b.project_graph.recipe.as_ref().expect("the recipe shown"));
    let planned = "“Kazı sınırı” katmanı bu projede yok; tarifteki stiliyle eklenecek. Alanları (3) ve alt türleri \
                   tarifteki gibi. “Sınır türü” değer listesi de eklenecek.";
    assert!(lines.iter().any(|(_, l)| l == planned), "{lines:?}");
    assert_eq!(b.document.as_ref().map(|d| d.model.revision()), revision, "nothing written before Üret");
    let _ = b.update(Message::ProjectGraph(G::Confirm));
    let out = b.project_graph.produced.clone().expect("produced");
    let done = "“Kazı sınırı” katmanı tarifteki stiliyle eklendi. Alanları (3) ve alt türleri tarifteki gibi. “Sınır \
                türü” değer listesi de eklendi.";
    let said = produced_lines(&out, &b.project_graph.looks);
    assert!(said.iter().any(|(_, l)| l == done), "{said:?}");
    let model = &b.document.as_ref().expect("a drawing").model;
    let layers = model.layers();
    let added = layers.get(&out.layers[0].to).expect("added");
    assert_eq!(serde_json::to_value(&added.fields).expect("JSON"), def["fields"]);
    assert_eq!(serde_json::to_value(&added.subtypes).expect("JSON"), def["subtypes"]);
    assert_eq!(serde_json::to_value(&model.settings().domains).expect("JSON"), def["domains"]);
}

/// A layer of the name here keeps its own fields: the plan says the recipe's fields it lacks.
#[test]
fn a_layer_of_the_name_here_keeps_its_fields_and_the_plan_says_the_lack() {
    let mut b = other_project();
    let _ = b.update(Message::ProjectGraph(G::Open));
    let _ = b.update(Message::ProjectGraph(G::Picked(Some(recipe("alanli-adli.json")))));
    let lines = plan_lines(b.project_graph.recipe.as_ref().expect("the recipe shown"));
    let lack = "“Sıfır hattı” katmanı bu projede bulundu ve kendi alanlarını korur: tarifin “Ad”, “Tür”, “Derinlik” \
                alanları onda yok.";
    assert!(lines.iter().any(|(_, l)| l.starts_with(lack)), "{lines:?}");
}

/// `.run/gui-evidence/tarif-katman-alanlari-4860/`: the picked recipe before Üret (the layer to be added with its fields
/// and list), after Üret, and a layer of the name here that lacks the recipe's fields.
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib recipe_layer_fields_tests::fields_screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn fields_screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/tarif-katman-alanlari-4860");
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
            ("alanli-katmanli.json", false, "plan"),
            ("alanli-katmanli.json", true, "uretildi"),
            ("alanli-adli.json", false, "adiyla-bulunan"),
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
