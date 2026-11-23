//! A drawing of a few objects opened as a file would be (the core's tools_scenes.rs, its part these tests use): the
//! objects on their layers, the layers Çizim, Yol and Parsel unless the snapshot is edited.

use iced::Point;
use kentos_render_wgpu::Vec2;
use kentos_ui::snapshot::{Input, Snapshot};
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::files_testing::{find_text, find_texts};
use crate::viewport::Event;

const EMPTY: &str = include_str!("../../../../fixtures/interaction/v1/empty.kcad");
/// Where the drawings lie: a TM zone's coordinates (Y, X).
pub const E: f64 = 487_000.0;
pub const N: f64 = 4_420_000.0;

fn xy(p: [f64; 2]) -> Value {
    json!({ "x": E + p[0], "y": N + p[1] })
}

/// The objects of a drawing: each its kind's fields and its layer.
pub struct Objects(Vec<Value>);

impl Objects {
    pub fn new() -> Self {
        Self(Vec::new())
    }

    pub fn push(&mut self, layer: &str, mut fields: Value) -> u32 {
        let id = self.0.len() as u32 + 1;
        fields["id"] = json!(id);
        fields["layerId"] = json!(layer);
        fields["attrs"] = json!({});
        self.0.push(fields);
        id
    }

    pub fn line(&mut self, layer: &str, a: [f64; 2], b: [f64; 2]) -> u32 {
        self.push(layer, json!({ "kind": "line", "a": xy(a), "b": xy(b) }))
    }

    pub fn path(&mut self, layer: &str, pts: &[[f64; 2]], closed: bool) -> u32 {
        let kind = if closed { "polygon" } else { "polyline" };
        let pts: Vec<Value> = pts.iter().map(|p| xy(*p)).collect();
        self.push(layer, json!({ "kind": kind, "pts": pts }))
    }

    pub fn point(&mut self, layer: &str, at: [f64; 2], z: Option<f64>) -> u32 {
        let mut fields = json!({ "kind": "point", "p": xy(at) });
        if let Some(z) = z {
            fields["z"] = json!(z);
        }
        self.push(layer, fields)
    }

    /// An object's attributes and its label.
    pub fn data(&mut self, id: u32, attrs: &[(&str, &str)], label: Option<&str>) {
        let e = &mut self.0[id as usize - 1];
        for (k, v) in attrs {
            e["attrs"][*k] = json!(v);
        }
        if let Some(label) = label {
            e["label"] = json!(label);
        }
    }

    pub fn text(&mut self, layer: &str, at: [f64; 2], words: &str, height: f64, turn: f64) -> u32 {
        self.push(layer, json!({ "kind": "text", "p": xy(at), "text": words, "height": height, "rotation": turn }))
    }
}

pub fn layer(id: &str, name: &str, color: &str, weight: f64) -> Value {
    json!({
        "id": id, "name": name, "type": "layer", "visible": true, "locked": false,
        "expanded": true,
        "style": { "color": color, "lineType": "continuous", "lineWeight": weight },
        "children": []
    })
}

/// The drawing of `objects` opened with its snapshot edited first (its settings, layers or blocks).
pub fn open_edited(app: &mut App, objects: Objects, edit: impl FnOnce(&mut Value)) {
    let mut d: Value = serde_json::from_str(EMPTY).expect("the empty drawing reads");
    d["layers"] = json!([
        layer("cizim", "Çizim", "fg", 0.25),
        layer("yol", "Yol", "#E5484D", 0.5),
        layer("parsel", "Parsel", "#3E63DD", 0.35),
    ]);
    d["entities"] = Value::Array(objects.0);
    edit(&mut d);
    let snapshot = kentos_commands::DocumentSnapshotV1::from_json(&d.to_string()).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
}

/// Where a point (east and north from the drawing's origin) is on the drawing area.
pub fn at(app: &App, p: [f64; 2]) -> Point {
    let [x, y] = app.viewport.camera.world_to_screen(Vec2::new(E + p[0], N + p[1]));
    Point::new(x as f32, y as f32)
}

pub fn hover(app: &mut App, p: [f64; 2]) {
    let point = at(app, p);
    let _ = app.update(Message::Viewport(Event::Moved(point)));
}

pub fn click(app: &mut App, p: [f64; 2]) {
    let point = at(app, p);
    let _ = app.update(Message::Viewport(Event::Moved(point)));
    let _ = app.update(Message::Viewport(Event::Pressed(point)));
    let _ = app.update(Message::Viewport(Event::Released(point)));
}

pub fn run(app: &mut App, id: &'static str) {
    let _ = app.update(Message::Run(id));
}

/// Where `caption` is on the window (a docked one first), or beside it where its arrow is.
pub fn caption_at(snapshot: &mut Snapshot, app: &App, caption: &str, arrow: bool) -> Point {
    let docked = find_texts(snapshot, app, caption).into_iter().find(|at| at.x > 900.0 && at.y > 150.0);
    let at = docked
        .or_else(|| find_text(snapshot, app, caption))
        .unwrap_or_else(|| panic!("{caption} is on screen"));
    if arrow { Point::new(at.x + at.width + 10.0, at.center_y()) } else { at.center() }
}

/// `caption` clicked (or right-clicked) on the window, or its arrow.
pub fn press_caption(snapshot: &mut Snapshot, app: &mut App, caption: &str, right: bool, arrow: bool) {
    let at = caption_at(snapshot, app, caption, arrow);
    let mut update = |app: &mut App, message: Message| {
        let _ = app.update(message);
    };
    let input = if right { Input::RightClick(at) } else { Input::Click(at) };
    snapshot.input(app, App::view, &mut update, input);
}
