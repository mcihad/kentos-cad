//! GIS-013 (docs/adr/1079): Ağlar proposes a layer that holds lines for a new network's edges, not the active area
//! layer; Kaydet refuses a network whose edges are an area layer's, or a layer of any kind holding no line, and says
//! which; an empty line layer is saved as a template, a mixed layer holding lines as edges.

use serde_json::json;

use super::Event as Networks;
use super::window::Event;
use crate::app::{App, Message};
use crate::gis;
use crate::tools_scenes::{Objects, layer, open_edited};

fn window(app: &mut App, e: Event) {
    let task = app.update(Message::Gis(gis::Event::Networks(Networks::Window(e))));
    crate::files_testing::drive(app, task);
}

/// A park's buffer (active), a walkway, an empty route template, a layer of any kind with a line and a point, one
/// with only an area.
fn park() -> App {
    let (mut app, _) = App::boot(None);
    let mut o = Objects::new();
    o.path("tampon", &[[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [0.0, 30.0]], true);
    o.path("yol", &[[-5.0, -5.0], [45.0, -5.0], [45.0, 35.0]], false);
    o.path("karma", &[[0.0, 40.0], [20.0, 40.0]], false);
    o.point("karma", [10.0, 45.0], None);
    o.path("eski", &[[50.0, 0.0], [60.0, 0.0], [60.0, 10.0]], true);
    open_edited(&mut app, o, |d| {
        d["settings"]["workspace"] = json!("gis");
        let typed = |id: &str, name: &str, g: &str| {
            let mut n = layer(id, name, "#3E63DD", 0.35);
            n["geometry"] = json!(g);
            n
        };
        d["layers"] = json!([
            typed("tampon", "Park koruma 10 m", "area"),
            typed("yol", "Yürüyüş yolları", "line"),
            typed("rota", "Bakım rotaları", "line"),
            layer("karma", "Karma", "#888888", 0.25),
            layer("eski", "Eski alanlar", "#888888", 0.25),
        ]);
        d["activeLayer"] = json!("tampon");
    });
    let task = app.update(Message::Run("network.manage"));
    crate::files_testing::drive(&mut app, task);
    app
}

fn edge_layer(app: &App) -> String {
    let w = app.gis.networks.window.as_ref().expect("Ağlar");
    w.form().expect("a network").edges.first().map(|e| e.layer.clone()).unwrap_or_default()
}

/// The place of `id` in the window's layer list (the tree's leaves).
fn place(app: &App, id: &str) -> usize {
    let doc = &app.document.as_ref().expect("open").model;
    doc.layers().leaves().iter().position(|n| n.id == id).expect("listed")
}

fn saved(app: &App) -> Vec<String> {
    let doc = &app.document.as_ref().expect("open").model;
    doc.settings().networks.iter().map(|n| n.name.clone()).collect()
}

#[test]
fn a_new_network_proposes_the_walkway_not_the_active_area() {
    let mut app = park();
    window(&mut app, Event::Add);
    assert_eq!(edge_layer(&app), "yol");
}

#[test]
fn an_area_layer_or_a_layer_without_lines_is_refused_with_its_reason() {
    for (id, words) in [("tampon", "bir alan katmanı"), ("eski", "katmanında çizgi yok")] {
        let mut app = park();
        window(&mut app, Event::Add);
        let k = place(&app, id);
        window(&mut app, Event::EdgeLayer(0, k));
        window(&mut app, Event::Save);
        let said = app.gis.networks.window.as_ref().and_then(|w| w.said_text()).unwrap_or("").to_owned();
        assert!(said.contains(words) && said.contains("kaydedilmedi"), "{id}: {said}");
        assert!(saved(&app).is_empty(), "{id}: nothing written");
        if id == "tampon" {
            let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
            shot.settle(&mut app, App::view, &mut |app: &mut App, m| drop(app.update(m)));
            crate::files_testing::evidence(&mut shot, &app, "gis-013", "alan-katmani-kenar-olamaz");
        }
    }
}

#[test]
fn an_empty_line_template_and_a_mixed_layer_with_lines_are_saved() {
    for id in ["rota", "karma"] {
        let mut app = park();
        window(&mut app, Event::Add);
        let k = place(&app, id);
        window(&mut app, Event::EdgeLayer(0, k));
        window(&mut app, Event::Save);
        assert_eq!(saved(&app).len(), 1, "{id}: {:?}", app.gis.networks.window.as_ref().and_then(|w| w.said_text()));
    }
}
