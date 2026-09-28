//! The drawings and the steps of the pictures `tools_screens` takes of the
//! tools of docs/adr/0140 phase 1: each scene opens a small drawing that
//! shows its tool well, drives the real app the way a person would (the
//! tool from the ribbon's command, a selection, a typed value, the pointer)
//! and stops where the picture is to be taken: mid-use with the preview, or
//! after the confirm. Test code only.

use iced::Point;
use kentos_render_wgpu::Vec2;
use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::document::Document;
use crate::tools_screens::Scene;
use crate::viewport::Event;

const EMPTY: &str = include_str!("../../../fixtures/interaction/v1/empty.kcad");
const E: f64 = 487_000.0;
const N: f64 = 4_420_000.0;

fn xy(p: [f64; 2]) -> Value {
    json!({ "x": E + p[0], "y": N + p[1] })
}

/// One object of a scene: its kind's fields and its layer.
struct Objects(Vec<Value>);

impl Objects {
    fn new() -> Self {
        Self(Vec::new())
    }

    fn push(&mut self, layer: &str, mut fields: Value) -> u32 {
        let id = self.0.len() as u32 + 1;
        fields["id"] = json!(id);
        fields["layerId"] = json!(layer);
        fields["attrs"] = json!({});
        self.0.push(fields);
        id
    }

    fn line(&mut self, layer: &str, a: [f64; 2], b: [f64; 2]) -> u32 {
        self.push(layer, json!({ "kind": "line", "a": xy(a), "b": xy(b) }))
    }

    fn path(&mut self, layer: &str, pts: &[[f64; 2]], closed: bool) -> u32 {
        let kind = if closed { "polygon" } else { "polyline" };
        let pts: Vec<Value> = pts.iter().map(|p| xy(*p)).collect();
        self.push(layer, json!({ "kind": kind, "pts": pts }))
    }

    fn arc(&mut self, layer: &str, c: [f64; 2], r: f64, a0: f64, a1: f64) -> u32 {
        self.push(
            layer,
            json!({ "kind": "arc", "c": xy(c), "r": r, "a0": a0, "a1": a1 }),
        )
    }

    /// A field every object may have besides its kind's: its own colour, line weight.
    fn styled(&mut self, id: u32, color: Option<&str>, weight: Option<f64>) {
        let e = &mut self.0[id as usize - 1];
        if let Some(color) = color {
            e["color"] = json!(color);
        }
        if let Some(weight) = weight {
            e["lineWeight"] = json!(weight);
        }
    }
}

fn layer(id: &str, name: &str, color: &str, weight: f64) -> Value {
    json!({
        "id": id, "name": name, "type": "layer", "visible": true, "locked": false,
        "expanded": true,
        "style": { "color": color, "lineType": "continuous", "lineWeight": weight },
        "children": []
    })
}

/// Opens a new drawing of `objects` on the layers Çizim, Yol and Parsel, as
/// the app would open a file: the view fits it.
fn open(app: &mut App, objects: Objects) {
    let mut d: Value = serde_json::from_str(EMPTY).expect("the empty drawing reads");
    d["layers"] = json!([
        layer("cizim", "Çizim", "fg", 0.25),
        layer("yol", "Yol", "#E5484D", 0.5),
        layer("parsel", "Parsel", "#3E63DD", 0.35),
    ]);
    d["entities"] = Value::Array(objects.0);
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(&d.to_string()).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
}

/// Where a point (east and north from the drawing's origin) is on the drawing area.
fn at(app: &App, p: [f64; 2]) -> Point {
    let [x, y] = app
        .viewport
        .camera
        .world_to_screen(Vec2::new(E + p[0], N + p[1]));
    Point::new(x as f32, y as f32)
}

fn hover(app: &mut App, p: [f64; 2]) {
    let point = at(app, p);
    let _ = app.update(Message::Viewport(Event::Moved(point)));
}

fn click(app: &mut App, p: [f64; 2]) {
    let point = at(app, p);
    let _ = app.update(Message::Viewport(Event::Moved(point)));
    let _ = app.update(Message::Viewport(Event::Pressed(point)));
    let _ = app.update(Message::Viewport(Event::Released(point)));
}

fn run(app: &mut App, id: &'static str) {
    let _ = app.update(Message::Run(id));
}

fn method(app: &mut App, id: &'static str, option: &'static str) {
    let _ = app.update(Message::RunMethod {
        id,
        option,
        label: "",
    });
}

fn typed(app: &mut App, text: &str) {
    let _ = app.submit_line(text);
}

fn select_all(app: &mut App) {
    run(app, "edit.selectAll");
}

// ── The drawings ────────────────────────────────────────────────────────────

/// Parcels with corners: an L-shaped one, a road with three turns, a
/// triangle and a small square that a 2 m radius does not fit.
fn corners() -> Objects {
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[
            [0.0, 0.0],
            [24.0, 0.0],
            [24.0, 10.0],
            [14.0, 10.0],
            [14.0, 20.0],
            [0.0, 20.0],
        ],
        true,
    );
    o.path(
        "yol",
        &[
            [30.0, 2.0],
            [42.0, 2.0],
            [42.0, 12.0],
            [54.0, 12.0],
            [54.0, 22.0],
        ],
        false,
    );
    o.path("parsel", &[[30.0, 26.0], [44.0, 26.0], [37.0, 37.0]], true);
    o.path(
        "cizim",
        &[[48.0, 27.0], [51.0, 27.0], [51.0, 30.0], [48.0, 30.0]],
        true,
    );
    o
}

/// A little road network: lines that cross, an arc and a long road to cut.
fn roads() -> Objects {
    let mut o = Objects::new();
    for y in [4.0, 16.0, 28.0] {
        o.line("cizim", [0.0, y], [56.0, y]);
    }
    for x in [10.0, 30.0, 50.0] {
        o.line("cizim", [x, 0.0], [x, 32.0]);
    }
    o.line("yol", [2.0, 30.0], [54.0, 2.0]);
    o.arc("parsel", [20.0, 16.0], 9.0, 0.0, std::f64::consts::PI);
    o
}

/// One long road, and two short lines apart from it.
fn long_road() -> Objects {
    let mut o = Objects::new();
    o.path(
        "yol",
        &[
            [0.0, 6.0],
            [14.0, 6.0],
            [24.0, 16.0],
            [40.0, 16.0],
            [52.0, 26.0],
        ],
        false,
    );
    o.line("cizim", [4.0, 26.0], [40.0, 26.0]);
    o.line("cizim", [4.0, 34.0], [40.0, 34.0]);
    o
}

/// A parcel, a road and a line, each with a direction to turn.
fn directions() -> Objects {
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[
            [0.0, 0.0],
            [22.0, 0.0],
            [22.0, 12.0],
            [11.0, 18.0],
            [0.0, 12.0],
        ],
        true,
    );
    o.path(
        "yol",
        &[[28.0, 2.0], [38.0, 2.0], [46.0, 10.0], [46.0, 20.0]],
        false,
    );
    o.line("cizim", [28.0, 28.0], [52.0, 28.0]);
    o
}

/// A surveyed road with a vertex every metre and a millimetre of noise, and
/// a parcel with extra vertices along its straight sides.
fn dense() -> Objects {
    let mut o = Objects::new();
    let noise = |i: usize| ((i * 7919) % 13) as f64 * 0.004 - 0.024;
    let road: Vec<[f64; 2]> = (0..=28)
        .map(|i| {
            let x = i as f64 * 2.0;
            [x, 24.0 + 4.0 * (x / 9.0).sin() + noise(i)]
        })
        .collect();
    o.path("cizim", &road, false);
    let mut parcel = Vec::new();
    for i in 0..6 {
        parcel.push([i as f64 * 3.0, noise(i)]);
    }
    for i in 0..4 {
        parcel.push([18.0 + noise(i + 3), i as f64 * 3.0]);
    }
    for i in 0..6 {
        parcel.push([18.0 - i as f64 * 3.0, 12.0 + noise(i + 5)]);
    }
    parcel.push([0.0, 12.0]);
    for i in 1..4 {
        parcel.push([noise(i + 2), 12.0 - i as f64 * 3.0]);
    }
    o.path("parsel", &parcel, true);
    o
}

/// Four parcels with their shared sides drawn twice, a line with no length,
/// a road that repeats its vertices and a copy of a whole parcel.
fn untidy() -> Objects {
    let mut o = Objects::new();
    for i in 0..3 {
        let x = i as f64 * 18.0;
        o.path(
            "parsel",
            &[[x, 0.0], [x + 18.0, 0.0], [x + 18.0, 14.0], [x, 14.0]],
            true,
        );
    }
    // Shared sides again, as lines.
    o.line("cizim", [18.0, 0.0], [18.0, 14.0]);
    o.line("cizim", [18.0, 0.0], [18.0, 14.0]);
    o.line("cizim", [36.0, 0.0], [36.0, 14.0]);
    o.line("cizim", [36.0, 0.0], [36.0, 14.0]);
    // A copy of the first parcel, on top of it.
    o.path(
        "parsel",
        &[[0.0, 0.0], [18.0, 0.0], [18.0, 14.0], [0.0, 14.0]],
        true,
    );
    // Empty: no length.
    o.line("cizim", [8.0, 20.0], [8.0, 20.0]);
    o.line("cizim", [30.0, 22.0], [30.0, 22.0]);
    // A road with repeated vertices.
    o.path(
        "yol",
        &[
            [0.0, 26.0],
            [12.0, 26.0],
            [12.0, 26.0],
            [24.0, 32.0],
            [24.0, 32.0],
            [24.0, 32.0],
            [40.0, 32.0],
            [54.0, 26.0],
        ],
        false,
    );
    o
}

/// A heavy red source line, and thin targets on another layer.
fn styles() -> Objects {
    let mut o = Objects::new();
    let source = o.line("yol", [0.0, 30.0], [26.0, 30.0]);
    o.styled(source, Some("#E5484D"), Some(1.4));
    for y in [4.0, 12.0, 20.0] {
        o.line("cizim", [4.0, y], [26.0, y]);
    }
    o.path(
        "cizim",
        &[[34.0, 4.0], [50.0, 4.0], [50.0, 18.0], [34.0, 18.0]],
        true,
    );
    o.path("cizim", &[[34.0, 24.0], [42.0, 30.0], [52.0, 26.0]], false);
    o
}

// ── The scenes ──────────────────────────────────────────────────────────────

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("yuvarla-onizleme", |app| {
            open(app, corners());
            select_all(app);
            run(app, "tool.filletAll");
            typed(app, "2");
            hover(app, [28.0, 14.0]);
        }),
        ("yuvarla-sonuc", |app| {
            open(app, corners());
            select_all(app);
            run(app, "tool.filletAll");
            typed(app, "2");
            run(app, "tool.confirm");
        }),
        ("pah-onizleme", |app| {
            open(app, corners());
            select_all(app);
            run(app, "tool.chamferAll");
            typed(app, "1.5,2.5");
            hover(app, [28.0, 14.0]);
        }),
        ("pah-sonuc", |app| {
            open(app, corners());
            select_all(app);
            run(app, "tool.chamferAll");
            typed(app, "1.5,2.5");
            run(app, "tool.confirm");
        }),
        ("parcala-kesisim-onizleme", |app| {
            open(app, roads());
            select_all(app);
            run(app, "tool.split");
            hover(app, [58.0, 34.0]);
        }),
        ("parcala-kesisim-sonuc", |app| {
            open(app, roads());
            select_all(app);
            run(app, "tool.split");
            run(app, "tool.confirm");
        }),
        ("parcala-esit-onizleme", |app| {
            open(app, long_road());
            method(app, "tool.split", "E");
            typed(app, "6");
            click(app, [19.0, 11.0]);
            hover(app, [30.0, 22.0]);
        }),
        ("parcala-esit-sonuc", |app| {
            open(app, long_road());
            method(app, "tool.split", "E");
            click(app, [19.0, 11.0]);
            typed(app, "6");
        }),
        ("parcala-uzunluk-onizleme", |app| {
            open(app, long_road());
            method(app, "tool.split", "U");
            typed(app, "9");
            click(app, [38.0, 26.0]);
            hover(app, [30.0, 22.0]);
        }),
        ("parcala-uzunluk-sonuc", |app| {
            open(app, long_road());
            method(app, "tool.split", "U");
            click(app, [38.0, 26.0]);
            typed(app, "9");
        }),
        ("yon-onizleme", |app| {
            open(app, directions());
            select_all(app);
            run(app, "tool.reverse");
            hover(app, [28.0, 36.0]);
        }),
        ("yon-sonuc", |app| {
            open(app, directions());
            select_all(app);
            run(app, "tool.reverse");
            run(app, "tool.confirm");
        }),
        ("sadelestir-onizleme", |app| {
            open(app, dense());
            select_all(app);
            run(app, "tool.simplify");
            typed(app, "0.03");
            hover(app, [30.0, 12.0]);
        }),
        ("sadelestir-sonuc", |app| {
            open(app, dense());
            select_all(app);
            run(app, "tool.simplify");
            typed(app, "0.03");
            run(app, "tool.confirm");
        }),
        ("temizle-onizleme", |app| {
            open(app, untidy());
            run(app, "tool.cleanup");
            hover(app, [58.0, 34.0]);
        }),
        ("temizle-sonuc", |app| {
            open(app, untidy());
            run(app, "tool.cleanup");
            run(app, "tool.confirm");
        }),
        ("ozellik-kaynak", |app| {
            open(app, styles());
            run(app, "tool.matchProperties");
            click(app, [13.0, 30.0]);
            hover(app, [30.0, 14.0]);
        }),
        ("ozellik-sonuc", |app| {
            open(app, styles());
            run(app, "tool.matchProperties");
            click(app, [13.0, 30.0]);
            click(app, [15.0, 4.0]);
            click(app, [15.0, 12.0]);
            click(app, [50.0, 11.0]);
            hover(app, [30.0, 14.0]);
        }),
    ]
}
