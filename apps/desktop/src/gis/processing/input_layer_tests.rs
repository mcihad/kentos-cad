//! GIS-014 (docs/adr/1080): with the park's area layer active and a tree layer beside it, En yakın tesis opens with
//! both its point inputs on the trees, each changeable; a tool of any kind keeps the active layer.

use serde_json::json;

use crate::app::{App, Message};
use crate::tools_scenes::{Objects, layer, open_edited};

fn park() -> App {
    let (mut app, _) = App::boot(None);
    let mut o = Objects::new();
    o.path("tampon", &[[0.0, 0.0], [40.0, 0.0], [40.0, 30.0], [0.0, 30.0]], true);
    for x in [5.0, 20.0, 35.0] {
        o.point("agac", [x, 10.0], None);
    }
    open_edited(&mut app, o, |d| {
        d["settings"]["workspace"] = json!("gis");
        let mut tampon = layer("tampon", "Park koruma 10 m", "#3E63DD", 0.35);
        tampon["geometry"] = json!("area");
        let mut agac = layer("agac", "Ağaçlar", "#30A46C", 0.25);
        agac["geometry"] = json!("point");
        d["layers"] = json!([tampon, agac]);
        d["activeLayer"] = json!("tampon");
    });
    app
}

fn input_layer(app: &App, name: &str) -> String {
    let w = app.gis.processing.dialog.as_ref().expect("the tool's window");
    w.values[name]["layerId"].as_str().unwrap_or("").to_owned()
}

#[test]
fn point_inputs_open_on_the_point_layer_not_the_active_area() {
    let mut app = park();
    let _ = app.update(Message::Run("processing.run.network.closestFacility"));
    assert_eq!((input_layer(&app, "incidents"), input_layer(&app, "facilities")), ("agac".into(), "agac".into()));
    if std::env::var_os("KENTOS_SHOTS").is_some() {
        let mut shot = crate::files_testing::pictured(iced::Size::new(1440., 900.));
        shot.settle(&mut app, App::view, &mut |app: &mut App, m| drop(app.update(m)));
        crate::files_testing::evidence(&mut shot, &app, "gis-014", "en-yakin-tesis-agaclar");
    }
    let _ = app.update(Message::Run("processing.run.geometry.buffer"));
    assert_eq!(input_layer(&app, "input"), "tampon", "a tool of any kind keeps the active layer");
}
