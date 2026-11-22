//! A layer's style classes in Katmanlar (docs/adr/9105 §2–§4): rows under the layer, a class switched off from its box
//! without an undo step, the counts after the names, Bu sınıfın nesnelerini seç, Öznitelik tablosunda göster and Renk
//! ▸ on a drawing layer, and a data layer's counts and selection read from its GeoPackage.

use kentos_native_application::ExecutionContext;
use serde_json::json;

use crate::files_testing::app_with_drawing;
use crate::layer_classes::Event;

fn renderer() -> serde_json::Value {
    let mark = |c: &str| json!({ "marker": { "type": "marker", "layers": [{ "id": "m", "type": "shape", "shape": "circle", "fill": c }] } });
    json!({
        "type": "categorized", "expr": "kullanim",
        "categories": [
            { "value": "Konut", "label": "Konut", "symbols": mark("#d9c27a") },
            { "value": "Park", "label": "Park", "symbols": mark("#2b7a3d") }
        ],
        "other": mark("#888888")
    })
}

fn style(app: &mut crate::app::App, layer: &str, renderer: serde_json::Value) {
    let doc = app.document.as_mut().expect("a drawing");
    let result = kentos_native_application::layers_renderer::execute(
        &mut ExecutionContext::new(&mut doc.model),
        kentos_commands::LayersRenderer { layer: layer.into(), renderer: Some(renderer), expected_revision: None },
    );
    assert!(matches!(result, kentos_commands::CommandResult::Completed { .. }), "{result:?}");
}

fn points(app: &mut crate::app::App, layer: &str, uses: &[&str]) {
    let objects: Vec<serde_json::Value> = uses
        .iter()
        .enumerate()
        .map(|(i, u)| json!({ "geometry": { "kind": "point", "p": { "x": 10.0 * i as f64, "y": 0.0 } }, "attrs": { "kullanim": u } }))
        .collect();
    let input = serde_json::from_value(json!({ "layerId": layer, "objects": objects })).expect("an input");
    let doc = app.document.as_mut().expect("a drawing");
    let result = kentos_native_application::create::execute(&mut ExecutionContext::new(&mut doc.model), input);
    assert!(matches!(result, kentos_commands::CommandResult::Completed { .. }), "{result:?}");
}

fn layer_node<'a>(app: &'a crate::app::App, id: &str) -> &'a kentos_commands::LayerNode {
    app.document.as_ref().expect("a drawing").model.layers().get(id).expect("the layer")
}

#[test]
fn a_drawing_layers_classes_are_rows_under_it_switched_counted_selected_and_coloured() {
    let mut app = app_with_drawing();
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    style(&mut app, &layer, renderer());
    // The drawing comes with objects of its own on the layer: they count too.
    let before = app
        .document
        .as_ref()
        .expect("a drawing")
        .model
        .by_layer(&layer)
        .filter(|e| kentos_native_application::style_classes::drawn_by_style(e))
        .count() as u64;
    points(&mut app, &layer, &["Konut", "Park", "Konut", "Sanayi"]);

    // The rows: the layer, then its two categories and Diğer değerler.
    let doc = app.document.as_ref().expect("a drawing");
    let rows = crate::layer_tree::open_rows(doc.layers(), &app.layer_search_state());
    let at = rows.iter().position(|r| r.node.id == layer && r.class.is_none()).expect("the layer's row");
    let classes: Vec<Option<usize>> = rows[at + 1..].iter().take_while(|r| r.node.id == layer).map(|r| r.class).collect();
    assert_eq!(classes, [Some(0), Some(1), Some(2)]);
    assert_eq!(rows[at + 1].depth, rows[at].depth + 1);

    // A class's box: switched off in the renderer, an edit without an undo step.
    let could_undo = app.document.as_ref().expect("a drawing").model.can_undo();
    let _ = app.layer_classes_event(Event::Toggle(layer.clone(), "c0".into()));
    let r = layer_node(&app, &layer).style.renderer.clone().expect("a renderer");
    assert_eq!(r["categories"][0]["enabled"], json!(false));
    assert_eq!(app.document.as_ref().expect("a drawing").model.can_undo(), could_undo, "no undo step");
    let _ = app.layer_classes_event(Event::Invert(layer.clone()));
    let r = layer_node(&app, &layer).style.renderer.clone().expect("a renderer");
    assert!(r["categories"][0].get("enabled").is_none());
    assert_eq!(r["categories"][1]["enabled"], json!(false));
    assert_eq!(r["otherEnabled"], json!(false));
    let _ = app.layer_classes_event(Event::All(layer.clone(), true));

    // Nesne sayısını göster: the counts after the names, worked out here for a drawing layer.
    let _ = app.layer_classes_event(Event::Counts(layer.clone()));
    assert_eq!(layer_node(&app, &layer).class_counts, Some(true));
    let _ = app.layer_classes_tasks();
    assert_eq!(app.class_total(layer_node(&app, &layer)), Some(before + 4));

    // Bu sınıfın nesnelerini seç: the two Konut points.
    let _ = app.layer_classes_event(Event::Select(layer.clone(), 0));
    assert_eq!(app.selection.len(), 2);

    // Öznitelik tablosunda göster: the table on the layer, its filter the class's condition.
    let _ = app.layer_classes_event(Event::Table(layer.clone(), 1));
    assert_eq!(app.gis.features.layer.as_deref(), Some(layer.as_str()));
    assert_eq!(app.gis.features.filter, "kullanim = 'Park'");

    // Renk ▸: the class's symbol in that colour, a style change Geri al takes back.
    let _ = app.layer_classes_event(Event::Color(layer.clone(), 1, "#e06c75".into()));
    let r = layer_node(&app, &layer).style.renderer.clone().expect("a renderer");
    assert_eq!(r["categories"][1]["symbols"]["marker"]["layers"][0]["fill"], json!("#e06c75"));
    assert!(app.document.as_ref().expect("a drawing").model.can_undo());
}

#[test]
fn a_data_layers_classes_are_counted_and_selected_from_its_source() {
    crate::data_layers::register();
    let path = crate::data_layers_tests::parcels("siniflar", 40);
    let address = path.display().to_string();
    let mut app = app_with_drawing();
    app.add_data_layers(vec![crate::data_layers::describe(&address, "parseller").expect("described")])
        .expect("added");
    let id = crate::data_layers_tests::node(&app, "parseller").expect("the layer").id.clone();
    style(&mut app, &id, renderer());
    let _ = app.layer_classes_event(Event::Counts(id.clone()));
    // Asked off the thread; here read and handed back.
    let _ = app.layer_classes_tasks();
    let key = app.class_count_key(&id).expect("asked");
    let node = layer_node(&app, &id).clone();
    let settings = app.document.as_ref().expect("a drawing").model.settings().clone();
    let answer = kentos_native_application::layer_classes::data_counts(&node, &settings, &|| false);
    let (counts, total) = answer.clone().expect("counted");
    assert_eq!(counts, [Ok(20), Ok(20), Ok(0)]);
    assert_eq!(total, 40);
    let _ = app.layer_classes_event(Event::Counted(id.clone(), key, answer));
    assert_eq!(app.class_total(layer_node(&app, &id)), Some(40));

    // Bu sınıfın nesnelerini seç: the Park rows' keys, selected in the table.
    let keys = kentos_native_application::layer_classes::data_class_keys(&node, &settings, 1, 10_000, &|| false);
    assert_eq!(keys.as_ref().map(|(k, more)| (k.len(), *more)), Ok((20, false)));
    // A row selected before is let go: the class's rows are the selection.
    app.gis.features.data.select_rows(&id, vec![1]);
    let _ = app.layer_classes_event(Event::Keys(id.clone(), "Park".into(), keys));
    assert_eq!(app.gis.features.data.selected.len(), 20);
    assert!(!app.gis.features.data.selected.contains(&1), "the Konut row selected before is let go");
    // At most the rows asked for, and there were more.
    let few = kentos_native_application::layer_classes::data_class_keys(&node, &settings, 0, 5, &|| false);
    assert_eq!(few.map(|(k, more)| (k.len(), more)), Ok((5, true)));
    let _ = std::fs::remove_file(&path);
}

/// A class's row pressed twice opens its symbol in the symbol designer over Katman stili (docs/adr/9110 §3); one press
/// only chooses the layer's row, and a press on the layer's row and then on its class is not a double one.
#[test]
fn a_class_rows_double_click_opens_its_symbol_in_the_designer() {
    use crate::style::designer::Target;
    use crate::style::layer_style::SetAt;
    let mut app = app_with_drawing();
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    style(&mut app, &layer, renderer());
    let _ = app.layer_classes_event(Event::Pressed(layer.clone(), 1));
    assert_eq!(app.selected_layer.as_deref(), Some(layer.as_str()));
    assert!(app.styles.layer_style.is_none(), "one press only chooses the row");
    let _ = app.update(crate::app::Message::LayerSelected(layer.clone()));
    let _ = app.layer_classes_event(Event::Pressed(layer.clone(), 1));
    assert!(app.styles.layer_style.is_none(), "the layer's row, then its class: two single presses");
    let _ = app.layer_classes_event(Event::Pressed(layer.clone(), 1));
    assert!(app.styles.layer_style.is_some(), "Katman stili on the layer");
    let designer = app.styles.designer.as_ref().expect("the designer");
    match &designer.target {
        Target::Slot { at, title, .. } => {
            assert_eq!(*at, SetAt::Category(1));
            assert_eq!(title, "Park");
        }
        other => panic!("{other:?}"),
    }
}

/// A drawing layer's object of a class switched off is not labelled, as a data layer's (docs/adr/9110 §4): the labels
/// follow what the style draws.
#[test]
fn a_drawing_layers_objects_of_a_class_switched_off_are_not_labelled() {
    let mut app = app_with_drawing();
    let layer = app.document.as_ref().expect("a drawing").model.layers().active().to_owned();
    let mut r = renderer();
    r["categories"][0]["enabled"] = json!(false);
    style(&mut app, &layer, r);
    points(&mut app, &layer, &["Konut", "Park", "Sanayi"]);
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let mut look = doc.layers().get(&layer).expect("the layer").style.clone();
    look.label = Some(kentos_commands::LabelStyle { template: Some("{kullanim}".into()), ..Default::default() });
    doc.set_layer_style(&layer, look, "Etiket");
    let spatial = kentos_interaction::Spatial::of(doc);
    let texts = |use_: &str| -> Vec<String> {
        let slot = doc
            .entities()
            .find(|e| e.base().attrs.get("kullanim").is_some_and(|v| v == use_))
            .map(|e| kentos_domain::Slot(e.base().id))
            .expect("the object");
        spatial.label_texts(slot)
    };
    assert!(texts("Konut").is_empty(), "Konut is switched off: not drawn, not labelled");
    assert_eq!(texts("Park"), ["Park"]);
    assert_eq!(texts("Sanayi"), ["Sanayi"], "Diğer değerler draws it");
}
