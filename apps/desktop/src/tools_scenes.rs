//! The drawings and the steps of the pictures `tools_screens` takes of the
//! tools of docs/adr/0140 (phases 1 to 3) and docs/adr/0141: each scene opens a small drawing that
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

/// A hole of an area: its vertices and their elevations (docs/adr/0142).
pub(crate) type Hole<'a> = (&'a [[f64; 2]], &'a [Option<f64>]);

/// A part of a multi-part area: its ring and its holes' rings (docs/adr/0143).
pub(crate) type Part<'a> = (&'a [[f64; 2]], &'a [&'a [[f64; 2]]]);

/// One object of a scene: its kind's fields and its layer.
pub(crate) struct Objects(Vec<Value>);

impl Objects {
    pub(crate) fn new() -> Self {
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

    pub(crate) fn line(&mut self, layer: &str, a: [f64; 2], b: [f64; 2]) -> u32 {
        self.push(layer, json!({ "kind": "line", "a": xy(a), "b": xy(b) }))
    }

    /// A line with the elevations of its two ends (docs/adr/0142).
    pub(crate) fn line_z(
        &mut self,
        layer: &str,
        a: [f64; 2],
        b: [f64; 2],
        z: [Option<f64>; 2],
    ) -> u32 {
        let mut fields = json!({ "kind": "line", "a": xy(a), "b": xy(b) });
        if let Some(za) = z[0] {
            fields["za"] = json!(za);
        }
        if let Some(zb) = z[1] {
            fields["zb"] = json!(zb);
        }
        self.push(layer, fields)
    }

    /// A polyline or an area with an elevation for each vertex, `None` for one
    /// without (docs/adr/0142); an area's holes come with theirs.
    pub(crate) fn path_z(
        &mut self,
        layer: &str,
        pts: &[[f64; 2]],
        closed: bool,
        zs: &[Option<f64>],
        holes: &[Hole<'_>],
    ) -> u32 {
        let kind = if closed { "polygon" } else { "polyline" };
        let pts: Vec<Value> = pts.iter().map(|p| xy(*p)).collect();
        let mut fields = json!({ "kind": kind, "pts": pts, "zs": zs });
        if !holes.is_empty() {
            let rings: Vec<Value> = holes
                .iter()
                .map(|(pts, zs)| {
                    let pts: Vec<Value> = pts.iter().map(|p| xy(*p)).collect();
                    json!({ "pts": pts, "zs": zs })
                })
                .collect();
            fields["holes"] = Value::Array(rings);
        }
        self.push(layer, fields)
    }

    pub(crate) fn path(&mut self, layer: &str, pts: &[[f64; 2]], closed: bool) -> u32 {
        let kind = if closed { "polygon" } else { "polyline" };
        let pts: Vec<Value> = pts.iter().map(|p| xy(*p)).collect();
        self.push(layer, json!({ "kind": kind, "pts": pts }))
    }

    /// An open polyline with a bulge on each segment (an arc where it is not 0).
    pub(crate) fn bulged(&mut self, layer: &str, pts: &[[f64; 2]], bulges: &[f64]) -> u32 {
        let pts: Vec<Value> = pts.iter().map(|p| xy(*p)).collect();
        self.push(
            layer,
            json!({ "kind": "polyline", "pts": pts, "bulges": bulges }),
        )
    }

    pub(crate) fn arc(&mut self, layer: &str, c: [f64; 2], r: f64, a0: f64, a1: f64) -> u32 {
        self.push(
            layer,
            json!({ "kind": "arc", "c": xy(c), "r": r, "a0": a0, "a1": a1 }),
        )
    }

    pub(crate) fn point(&mut self, layer: &str, at: [f64; 2], z: Option<f64>) -> u32 {
        let mut fields = json!({ "kind": "point", "p": xy(at) });
        if let Some(z) = z {
            fields["z"] = json!(z);
        }
        self.push(layer, fields)
    }

    pub(crate) fn circle(&mut self, layer: &str, c: [f64; 2], r: f64) -> u32 {
        self.push(layer, json!({ "kind": "circle", "c": xy(c), "r": r }))
    }

    /// A dimension of `style` (none: aligned) between `a` and `b`, `offset`
    /// out, 2.5 m high, with the centre `c` of the kinds that have one; `more`
    /// gives its other fields (angle, mask, za, zb; docs/adr/0147).
    pub(crate) fn dimension(
        &mut self,
        layer: &str,
        style: Option<&str>,
        [a, b]: [[f64; 2]; 2],
        c: Option<[f64; 2]>,
        offset: f64,
        more: Value,
    ) -> u32 {
        let mut fields =
            json!({ "kind": "dimension", "a": xy(a), "b": xy(b), "offset": offset, "height": 2.5 });
        if let Some(style) = style {
            fields["style"] = json!(style);
        }
        if let Some(c) = c {
            fields["c"] = xy(c);
        }
        if let Value::Object(more) = more {
            for (key, value) in more {
                fields[key] = value;
            }
        }
        self.push(layer, fields)
    }

    /// An area of several parts, each its ring and holes, the first the
    /// area's own (docs/adr/0143).
    pub(crate) fn parts(&mut self, layer: &str, parts: &[Part<'_>]) -> u32 {
        let ring = |pts: &[[f64; 2]]| Value::Array(pts.iter().map(|p| xy(*p)).collect());
        let part = |(pts, holes): &Part<'_>| {
            let mut fields = json!({ "pts": ring(pts) });
            if !holes.is_empty() {
                let holes: Vec<Value> = holes.iter().map(|h| json!({ "pts": ring(h) })).collect();
                fields["holes"] = Value::Array(holes);
            }
            fields
        };
        let mut fields = part(&parts[0]);
        fields["kind"] = json!("polygon");
        fields["parts"] = Value::Array(parts[1..].iter().map(part).collect());
        self.push(layer, fields)
    }

    /// A text: its start, words, height in metres and turn in degrees, and
    /// its alignment, width factor and mask when it has them (docs/adr/0145).
    pub(crate) fn text(
        &mut self,
        layer: &str,
        at: [f64; 2],
        words: &str,
        height: f64,
        turn: f64,
    ) -> u32 {
        self.push(
            layer,
            json!({ "kind": "text", "p": xy(at), "text": words, "height": height, "rotation": turn }),
        )
    }

    /// A text's extras (docs/adr/0145): its alignment's name, width factor and mask.
    pub(crate) fn text_extras(
        &mut self,
        id: u32,
        align: Option<&str>,
        width_factor: Option<f64>,
        mask: bool,
    ) {
        let e = &mut self.0[id as usize - 1];
        if let Some(align) = align {
            e["align"] = json!(align);
        }
        if let Some(factor) = width_factor {
            e["widthFactor"] = json!(factor);
        }
        if mask {
            e["mask"] = json!(true);
        }
    }

    /// An object's attributes and label.
    pub(crate) fn data(&mut self, id: u32, attrs: &[(&str, &str)], label: Option<&str>) {
        let e = &mut self.0[id as usize - 1];
        for (k, v) in attrs {
            e["attrs"][*k] = json!(v);
        }
        if let Some(label) = label {
            e["label"] = json!(label);
        }
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
pub(crate) fn open(app: &mut App, objects: Objects) {
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
pub(crate) fn at(app: &App, p: [f64; 2]) -> Point {
    let [x, y] = app
        .viewport
        .camera
        .world_to_screen(Vec2::new(E + p[0], N + p[1]));
    Point::new(x as f32, y as f32)
}

pub(crate) fn hover(app: &mut App, p: [f64; 2]) {
    let point = at(app, p);
    let _ = app.update(Message::Viewport(Event::Moved(point)));
}

pub(crate) fn click(app: &mut App, p: [f64; 2]) {
    let point = at(app, p);
    let _ = app.update(Message::Viewport(Event::Moved(point)));
    let _ = app.update(Message::Viewport(Event::Pressed(point)));
    let _ = app.update(Message::Viewport(Event::Released(point)));
}

pub(crate) fn run(app: &mut App, id: &'static str) {
    let _ = app.update(Message::Run(id));
}

pub(crate) fn method(app: &mut App, id: &'static str, option: &'static str) {
    let _ = app.update(Message::RunMethod {
        id,
        option,
        label: "",
    });
}

pub(crate) fn typed(app: &mut App, text: &str) {
    let _ = app.submit_line(text);
}

/// The message log emptied, so that what the next step says is all it shows.
pub(crate) fn forget(app: &mut App) {
    let _ = app.update(Message::HistoryCleared);
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

/// A road, a parcel and open ground: room for pie slices.
fn slice_ground() -> Objects {
    let mut o = Objects::new();
    o.path(
        "yol",
        &[[0.0, 4.0], [22.0, 2.0], [44.0, 8.0], [64.0, 6.0]],
        false,
    );
    o.path(
        "parsel",
        &[[4.0, 18.0], [28.0, 18.0], [28.0, 38.0], [4.0, 38.0]],
        true,
    );
    o
}

/// A long road with a parcel and a line to place points between.
fn between_ground() -> Objects {
    let mut o = Objects::new();
    o.line("yol", [4.0, 8.0], [54.0, 26.0]);
    o.path(
        "parsel",
        &[[4.0, 14.0], [22.0, 14.0], [22.0, 30.0], [4.0, 30.0]],
        true,
    );
    o.line("cizim", [30.0, 4.0], [58.0, 4.0]);
    o
}

/// Two known points and the ground around them, for distances, bearings and lines.
fn meeting_ground() -> Objects {
    let mut o = Objects::new();
    o.point("parsel", [8.0, 14.0], None);
    o.point("parsel", [44.0, 14.0], None);
    o.line("yol", [0.0, -2.0], [52.0, -2.0]);
    o.path(
        "cizim",
        &[[0.0, 34.0], [20.0, 36.0], [40.0, 34.0], [52.0, 30.0]],
        false,
    );
    o.line("cizim", [6.0, 10.0], [20.0, 16.0]);
    o.line("cizim", [10.0, 34.0], [20.0, 26.0]);
    o
}

/// A parcel with four corners: an angle to measure at each.
fn corners_ground() -> Objects {
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[[6.0, 6.0], [46.0, 6.0], [38.0, 30.0], [14.0, 26.0]],
        true,
    );
    o.line("yol", [0.0, 0.0], [54.0, 0.0]);
    o
}

/// Survey points along a wall, one of them with its elevation.
fn points_ground() -> Objects {
    let mut o = Objects::new();
    for (x, z) in [
        (2.0, None),
        (12.0, Some(24.6)),
        (25.0, None),
        (37.0, None),
        (54.0, Some(21.3)),
    ] {
        o.point("parsel", [x, 8.0], z);
    }
    o.line("cizim", [2.0, 8.0], [54.0, 8.0]);
    o.path("yol", &[[0.0, -6.0], [30.0, -4.0], [58.0, -6.0]], false);
    // The far side of the block: room above for the dimensions.
    o.path("parsel", &[[0.0, 44.0], [30.0, 46.0], [58.0, 44.0]], false);
    o
}

/// Four roads across two streets that reach beyond them.
fn fence_roads() -> Objects {
    let mut o = Objects::new();
    for x in [8.0, 22.0, 36.0, 50.0] {
        o.line("cizim", [x, 0.0], [x, 32.0]);
    }
    for y in [8.0, 24.0] {
        o.line("yol", [0.0, y], [58.0, y]);
    }
    o
}

/// A wall and lines that stop short of it, each a different length.
fn short_lines() -> Objects {
    let mut o = Objects::new();
    o.line("yol", [54.0, 0.0], [54.0, 36.0]);
    for (i, reach) in [30.0, 42.0, 36.0, 26.0, 40.0, 46.0].into_iter().enumerate() {
        let y = 3.0 + i as f64 * 6.0;
        o.line("cizim", [4.0, y], [reach + 4.0, y]);
    }
    o
}

/// A road with a corner and a parcel, to be offset to both sides.
fn kerbs() -> Objects {
    let mut o = Objects::new();
    o.path(
        "yol",
        &[[2.0, 14.0], [24.0, 14.0], [36.0, 26.0], [58.0, 26.0]],
        false,
    );
    o.path(
        "parsel",
        &[[6.0, 30.0], [20.0, 30.0], [20.0, 40.0], [6.0, 40.0]],
        true,
    );
    o.circle("cizim", [44.0, 10.0], 5.0);
    o
}

/// An arched road and a car outline at its start, to be laid along it.
fn arch() -> Objects {
    let mut o = Objects::new();
    o.arc("yol", [30.0, -20.0], 45.0, 1.1, 2.05);
    let (sx, sy) = (30.0 + 45.0 * 1.1f64.cos(), -20.0 + 45.0 * 1.1f64.sin());
    let car = [
        [-2.0, -1.0],
        [1.0, -1.0],
        [2.6, 0.0],
        [1.0, 1.0],
        [-2.0, 1.0],
    ];
    let pts: Vec<[f64; 2]> = car.iter().map(|p| [sx + p[0], sy + p[1]]).collect();
    o.path("parsel", &pts, true);
    o.line("cizim", [0.0, 0.0], [60.0, 0.0]);
    o
}

// ── The scenes ──────────────────────────────────────────────────────────────

fn first_phase() -> Vec<Scene> {
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

/// Every scene: phase 1's tools, then phase 2's, then phase 3's, then docs/adr/0141's.
pub(crate) fn scenes() -> Vec<Scene> {
    let mut all = first_phase();
    all.extend(second_phase());
    all.extend(third_phase());
    all.extend(query_scenes());
    all
}

/// The car outline of [`arch`] selected: it starts where the road ends, so a click there would
/// take the road.
fn choose_car(app: &mut App) {
    app.selection.set([kentos_domain::Slot(2)]);
}

/// Daire dilimi, Ara nokta, Kesişim noktası, Açı ölç, Koordinat oku, Zincir ve Baz ölçü.
fn second_phase() -> Vec<Scene> {
    vec![
        ("dilim-onizleme", |app| {
            open(app, slice_ground());
            run(app, "tool.sector");
            click(app, [44.0, 22.0]);
            click(app, [58.0, 22.0]);
            hover(app, [37.0, 34.1]);
        }),
        ("dilim-yazilan-onizleme", |app| {
            open(app, slice_ground());
            run(app, "tool.sector");
            click(app, [40.0, 22.0]);
            typed(app, "16");
            typed(app, "20");
            hover(app, [30.0, 38.0]);
        }),
        ("dilim-sonuc", |app| {
            open(app, slice_ground());
            run(app, "tool.sector");
            click(app, [44.0, 22.0]);
            click(app, [58.0, 22.0]);
            forget(app);
            click(app, [37.0, 34.1]);
            hover(app, [16.0, 10.0]);
        }),
        ("aranokta-esit-onizleme", |app| {
            open(app, between_ground());
            run(app, "tool.pointsBetween");
            click(app, [4.0, 8.0]);
            click(app, [54.0, 26.0]);
            hover(app, [30.0, 34.0]);
        }),
        ("aranokta-esit-sonuc", |app| {
            open(app, between_ground());
            run(app, "tool.pointsBetween");
            click(app, [4.0, 8.0]);
            click(app, [54.0, 26.0]);
            forget(app);
            typed(app, "6");
            hover(app, [30.0, 34.0]);
        }),
        ("aranokta-uzaklik-onizleme", |app| {
            open(app, between_ground());
            method(app, "tool.pointsBetween", "U");
            click(app, [4.0, 8.0]);
            click(app, [54.0, 26.0]);
            typed(app, "8, 20, 33");
            // Taken back, the kept distances are what the next pair shows.
            run(app, "edit.undo");
            click(app, [4.0, 8.0]);
            click(app, [54.0, 26.0]);
            hover(app, [30.0, 34.0]);
        }),
        ("aranokta-oran-sonuc", |app| {
            open(app, between_ground());
            method(app, "tool.pointsBetween", "O");
            click(app, [4.0, 8.0]);
            click(app, [54.0, 26.0]);
            forget(app);
            typed(app, "0.15, 0.4, 0.75, 0.9");
            hover(app, [30.0, 34.0]);
        }),
        ("kesisim-uzaklik-secim", |app| {
            open(app, meeting_ground());
            run(app, "tool.intersectPoint");
            click(app, [8.0, 14.0]);
            typed(app, "22");
            click(app, [44.0, 14.0]);
            typed(app, "24");
            hover(app, [26.0, 25.0]);
        }),
        ("kesisim-uzaklik-sonuc", |app| {
            open(app, meeting_ground());
            run(app, "tool.intersectPoint");
            click(app, [8.0, 14.0]);
            typed(app, "22");
            click(app, [44.0, 14.0]);
            typed(app, "24");
            forget(app);
            click(app, [24.7, 28.3]);
            hover(app, [40.0, 30.0]);
        }),
        ("kesisim-dogrultu-onizleme", |app| {
            open(app, meeting_ground());
            method(app, "tool.intersectPoint", "D");
            click(app, [8.0, 14.0]);
            typed(app, "60");
            click(app, [44.0, 14.0]);
            hover(app, [33.0, 21.0]);
        }),
        ("kesisim-dogrultu-sonuc", |app| {
            open(app, meeting_ground());
            method(app, "tool.intersectPoint", "D");
            click(app, [8.0, 14.0]);
            typed(app, "60");
            click(app, [44.0, 14.0]);
            forget(app);
            typed(app, "340");
            hover(app, [40.0, 30.0]);
        }),
        ("kesisim-dogru-onizleme", |app| {
            open(app, meeting_ground());
            method(app, "tool.intersectPoint", "L");
            click(app, [6.0, 10.0]);
            click(app, [20.0, 16.0]);
            click(app, [10.0, 34.0]);
            hover(app, [20.0, 26.0]);
        }),
        ("kesisim-dogru-sonuc", |app| {
            open(app, meeting_ground());
            method(app, "tool.intersectPoint", "L");
            click(app, [6.0, 10.0]);
            click(app, [20.0, 16.0]);
            click(app, [10.0, 34.0]);
            forget(app);
            click(app, [20.0, 26.0]);
            hover(app, [40.0, 30.0]);
        }),
        ("aci-onizleme", |app| {
            open(app, corners_ground());
            run(app, "tool.measureAngle");
            click(app, [46.0, 6.0]);
            click(app, [6.0, 6.0]);
            hover(app, [40.0, 26.0]);
        }),
        ("aci-sonuc", |app| {
            open(app, corners_ground());
            run(app, "tool.measureAngle");
            click(app, [46.0, 6.0]);
            click(app, [6.0, 6.0]);
            forget(app);
            click(app, [38.0, 30.0]);
            hover(app, [26.0, 16.0]);
        }),
        ("koordinat-oku", |app| {
            open(app, points_ground());
            run(app, "crs.query");
            click(app, [12.0, 8.0]);
            click(app, [30.0, 20.0]);
            click(app, [54.0, 8.0]);
            hover(app, [40.0, 26.0]);
        }),
        ("zincir-onizleme", |app| {
            open(app, points_ground());
            first_dimension(app);
            run(app, "tool.dimContinue");
            hover(app, [25.0, 12.0]);
        }),
        ("zincir-sonuc", |app| {
            open(app, points_ground());
            first_dimension(app);
            run(app, "tool.dimContinue");
            click(app, [25.0, 8.0]);
            click(app, [37.0, 8.0]);
            forget(app);
            click(app, [54.0, 8.0]);
            hover(app, [30.0, 24.0]);
        }),
        ("baz-onizleme", |app| {
            open(app, points_ground());
            first_dimension(app);
            run(app, "tool.dimBaseline");
            click(app, [25.0, 8.0]);
            hover(app, [37.0, 12.0]);
        }),
        ("baz-sonuc", |app| {
            open(app, points_ground());
            first_dimension(app);
            run(app, "tool.dimBaseline");
            click(app, [25.0, 8.0]);
            click(app, [37.0, 8.0]);
            forget(app);
            click(app, [54.0, 8.0]);
            hover(app, [30.0, 40.0]);
        }),
    ]
}

/// The first dimension of a run, drawn with Ölçülendirme: (2, 8) to (12, 8), its line 6 m above.
fn first_dimension(app: &mut App) {
    run(app, "tool.dimension");
    click(app, [2.0, 8.0]);
    click(app, [12.0, 8.0]);
    click(app, [7.0, 14.0]);
    run(app, "tool.cancel");
}

/// Buda and Uzat's fence, Ötele's two sides and deleted source, Yol boyunca dizi.
fn third_phase() -> Vec<Scene> {
    vec![
        ("cit-buda-onizleme", |app| {
            open(app, fence_roads());
            method(app, "tool.trim", "C");
            click(app, [0.0, 16.0]);
            click(app, [30.0, 16.0]);
            hover(app, [56.0, 16.0]);
        }),
        ("cit-buda-sonuc", |app| {
            open(app, fence_roads());
            method(app, "tool.trim", "C");
            click(app, [0.0, 16.0]);
            click(app, [56.0, 16.0]);
            forget(app);
            run(app, "tool.confirm");
            hover(app, [28.0, 28.0]);
        }),
        ("cit-uzat-onizleme", |app| {
            open(app, short_lines());
            method(app, "tool.extend", "C");
            click(app, [28.0, -2.0]);
            hover(app, [28.0, 38.0]);
        }),
        ("cit-uzat-sonuc", |app| {
            open(app, short_lines());
            method(app, "tool.extend", "C");
            click(app, [28.0, -2.0]);
            click(app, [28.0, 38.0]);
            forget(app);
            run(app, "tool.confirm");
            hover(app, [40.0, 30.0]);
        }),
        ("otele-istem", |app| {
            open(app, kerbs());
            run(app, "tool.offset");
        }),
        ("otele-iki-yana-onizleme", |app| {
            open(app, kerbs());
            run(app, "tool.offset");
            typed(app, "I");
            typed(app, "3");
            click(app, [13.0, 14.0]);
            hover(app, [13.0, 19.0]);
        }),
        ("otele-iki-yana-sonuc", |app| {
            open(app, kerbs());
            run(app, "tool.offset");
            typed(app, "I");
            typed(app, "3");
            click(app, [13.0, 14.0]);
            forget(app);
            click(app, [13.0, 19.0]);
            hover(app, [40.0, 40.0]);
        }),
        ("otele-kaynak-sil-sonuc", |app| {
            open(app, kerbs());
            run(app, "tool.offset");
            typed(app, "I");
            typed(app, "S");
            typed(app, "3");
            click(app, [13.0, 14.0]);
            forget(app);
            click(app, [13.0, 19.0]);
            hover(app, [40.0, 40.0]);
        }),
        ("dizi-yol-onizleme", |app| {
            open(app, arch());
            choose_car(app);
            run(app, "tool.arrayPath");
            click(app, [28.7, 25.0]);
            typed(app, "7");
            hover(app, [30.0, 8.0]);
        }),
        ("dizi-yol-aralik-onizleme", |app| {
            open(app, arch());
            choose_car(app);
            run(app, "tool.arrayPath");
            click(app, [28.7, 25.0]);
            typed(app, "A");
            typed(app, "9");
            hover(app, [30.0, 8.0]);
        }),
        ("dizi-yol-sonuc", |app| {
            open(app, arch());
            choose_car(app);
            run(app, "tool.arrayPath");
            click(app, [28.7, 25.0]);
            typed(app, "7");
            forget(app);
            run(app, "tool.confirm");
            hover(app, [30.0, 8.0]);
        }),
    ]
}

// ── docs/adr/0141: the drawings ─────────────────────────────────────────────

/// Nine parcels in a block, and two objects that fell far away from them.
pub(crate) fn far_ground() -> Objects {
    let mut o = Objects::new();
    for i in 0..9 {
        let (x, y) = ((i % 3) as f64 * 22.0, (i / 3) as f64 * 18.0);
        o.path(
            "parsel",
            &[[x, y], [x + 20.0, y], [x + 20.0, y + 16.0], [x, y + 16.0]],
            true,
        );
    }
    // Brought in with a wrong coordinate system: half a kilometre from the block.
    o.point("yol", [-520.0, -380.0], None);
    o.path(
        "yol",
        &[
            [520.0, 560.0],
            [536.0, 560.0],
            [536.0, 576.0],
            [520.0, 576.0],
        ],
        true,
    );
    o
}

/// A parcel with a building drawn inside it, its neighbour, and a road: Mesafe ölç's corners.
pub(crate) fn survey_ground() -> Objects {
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[[6.0, 6.0], [46.0, 6.0], [38.0, 30.0], [14.0, 26.0]],
        true,
    );
    o.line("yol", [0.0, 0.0], [54.0, 0.0]);
    o.path(
        "cizim",
        &[[20.0, 12.0], [30.0, 12.0], [30.0, 18.0], [20.0, 18.0]],
        true,
    );
    o
}

/// Two neighbouring parcels, the first with a building inside it (an island).
pub(crate) fn islands_ground() -> Objects {
    let mut o = Objects::new();
    o.path(
        "parsel",
        &[[0.0, 0.0], [24.0, 0.0], [24.0, 20.0], [0.0, 20.0]],
        true,
    );
    o.path(
        "parsel",
        &[[24.0, 0.0], [44.0, 0.0], [44.0, 20.0], [24.0, 20.0]],
        true,
    );
    o.path(
        "cizim",
        &[[8.0, 6.0], [14.0, 6.0], [14.0, 12.0], [8.0, 12.0]],
        true,
    );
    o
}

/// A road along a slope with a parcel at one side and a line at the other.
pub(crate) fn station_ground() -> Objects {
    let mut o = Objects::new();
    o.line("yol", [4.0, 8.0], [54.0, 26.0]);
    o.path(
        "parsel",
        &[[4.0, 14.0], [22.0, 14.0], [22.0, 30.0], [4.0, 30.0]],
        true,
    );
    o.line("cizim", [30.0, 4.0], [58.0, 4.0]);
    o.point("cizim", [40.0, 32.0], None);
    o
}

/// A parcel in a block in a district, one closed object round the other.
pub(crate) fn nested_ground() -> Objects {
    let mut o = Objects::new();
    let square = |x0: f64, y0: f64, x1: f64, y1: f64| [[x0, y0], [x1, y0], [x1, y1], [x0, y1]];
    o.path("parsel", &square(0.0, 0.0, 60.0, 40.0), true);
    o.path("yol", &square(8.0, 6.0, 44.0, 34.0), true);
    o.path("cizim", &square(16.0, 12.0, 30.0, 26.0), true);
    o
}

// ── docs/adr/0141: the scenes ───────────────────────────────────────────────

/// Kapsam denetimi, Mesafe ölç's fixed first point, İçine tıkla and Alan olarak çiz, Dik ayak ölç.
fn query_scenes() -> Vec<Scene> {
    vec![
        ("kapsam-sonuc", |app| {
            open(app, far_ground());
            run(app, "view.extentCheck");
            // Everything on screen, the strays selected among the parcels.
            run(app, "view.zoomExtents");
        }),
        ("kapsam-temiz", |app| {
            open(app, survey_ground());
            run(app, "view.extentCheck");
        }),
        ("mesafe-sabit-onizleme", |app| {
            open(app, survey_ground());
            run(app, "tool.measure");
            typed(app, "s");
            forget(app);
            click(app, [6.0, 6.0]);
            click(app, [46.0, 6.0]);
            click(app, [38.0, 30.0]);
            click(app, [14.0, 26.0]);
            hover(app, [52.0, 20.0]);
        }),
        ("mesafe-zincir-onizleme", |app| {
            open(app, survey_ground());
            run(app, "tool.measure");
            click(app, [6.0, 6.0]);
            click(app, [46.0, 6.0]);
            hover(app, [38.0, 30.0]);
        }),
        ("alan-icine-tikla", |app| {
            open(app, islands_ground());
            run(app, "tool.area");
            typed(app, "i");
            hover(app, [4.0, 16.0]);
        }),
        ("alan-icine-tikla-sonuc", |app| {
            open(app, islands_ground());
            run(app, "tool.area");
            typed(app, "i");
            click(app, [4.0, 16.0]);
            hover(app, [4.0, 16.0]);
        }),
        ("alan-olarak-ciz-sonuc", |app| {
            open(app, islands_ground());
            run(app, "tool.area");
            typed(app, "i");
            forget(app);
            click(app, [4.0, 16.0]);
            typed(app, "a");
            hover(app, [34.0, 10.0]);
        }),
        ("dik-ayak-cizgi", |app| {
            open(app, station_ground());
            run(app, "tool.stationOffset");
            click(app, [4.0, 8.0]);
            hover(app, [40.0, 22.0]);
        }),
        ("dik-ayak-onizleme", |app| {
            open(app, station_ground());
            run(app, "tool.stationOffset");
            click(app, [4.0, 8.0]);
            click(app, [54.0, 26.0]);
            hover(app, [46.0, 12.0]);
        }),
        ("dik-ayak-sonuc", |app| {
            open(app, station_ground());
            run(app, "tool.stationOffset");
            click(app, [4.0, 8.0]);
            click(app, [54.0, 26.0]);
            forget(app);
            click(app, [22.0, 14.0]);
            click(app, [22.0, 30.0]);
            hover(app, [46.0, 12.0]);
        }),
        ("cit-sec-onizleme", |app| {
            open(app, fence_roads());
            run(app, "tool.selectFence");
            click(app, [4.0, 30.0]);
            click(app, [30.0, 12.0]);
            hover(app, [56.0, 20.0]);
        }),
        ("cit-sec-sonuc", |app| {
            open(app, fence_roads());
            run(app, "tool.selectFence");
            click(app, [4.0, 30.0]);
            click(app, [30.0, 12.0]);
            click(app, [56.0, 20.0]);
            forget(app);
            run(app, "tool.confirm");
            hover(app, [28.0, 34.0]);
        }),
        ("daire-sec-kesisen-onizleme", |app| {
            open(app, corners());
            run(app, "tool.selectCircle");
            typed(app, "k");
            click(app, [26.0, 18.0]);
            hover(app, [40.0, 26.0]);
        }),
        ("daire-sec-sonuc", |app| {
            open(app, corners());
            run(app, "tool.selectCircle");
            typed(app, "k");
            click(app, [26.0, 18.0]);
            forget(app);
            click(app, [40.0, 26.0]);
            hover(app, [30.0, 40.0]);
        }),
        ("daire-sec-icinde-sonuc", |app| {
            open(app, corners());
            run(app, "tool.selectCircle");
            click(app, [37.0, 30.0]);
            forget(app);
            click(app, [37.0, 41.0]);
            hover(app, [4.0, 40.0]);
        }),
        ("iceren-alan-onizleme", |app| {
            open(app, nested_ground());
            run(app, "tool.selectContaining");
            hover(app, [22.0, 19.0]);
        }),
        ("iceren-alan-2-3", |app| {
            open(app, nested_ground());
            run(app, "tool.selectContaining");
            click(app, [22.0, 19.0]);
            click(app, [22.0, 19.0]);
        }),
    ]
}
