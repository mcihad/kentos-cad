//! The pictures of docs/adr/0263 (places along an edge and inside blocks): Yeniden bağla with En yakın on, the
//! cursor on a wall's edge and the prompt naming the place along it; the dimension bound there and to the wall's
//! end, the wall stretched and the dimension followed; a room block with a door inside, Yeniden bağla on the door's
//! end inside the insert (“bitiş (blokta)”); the insert turned and moved and the dimension on the door followed.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=yer-yakin,yer-yakin-izler,yer-blok,yer-blok-izler cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::{BlockDefinition, Entity};
use kentos_domain::Slot;
use serde_json::json;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run};
use crate::tools_screens::Scene;

const ROOM: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4101";
const DOOR: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d4102";

/// A wall (0,0)–(30,0) and a free aligned dimension above it, 2 m out.
fn wall_ground() -> Objects {
    let mut o = Objects::new();
    o.line("cizim", [0.0, 0.0], [30.0, 0.0]);
    o.dimension(
        "cizim",
        None,
        [[0.0, 8.0], [10.0, 8.0]],
        None,
        2.0,
        json!({}),
    );
    o
}

fn fitted(app: &mut App, min: [f64; 2], max: [f64; 2]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E + min[0],
            min_y: N + min[1],
            max_x: E + max[0],
            max_y: N + max[1],
        },
        24.0,
    );
}

/// Yeniden bağla, the dimension picked, the cursor on the wall at 30 % of it with En yakın on.
fn nearest(app: &mut App) {
    open(app, wall_ground());
    make_cad(app);
    fitted(app, [-4.0, -8.0], [34.0, 16.0]);
    run(app, "draft.snap.nearest");
    run(app, "tool.dimensionReassociate");
    click(app, [5.0, 10.0]);
    hover(app, [9.0, 0.0]);
}

/// The dimension bound to the wall at 30 % and to its end; the wall then stretched to 40 m: the dimension follows.
fn nearest_followed(app: &mut App) {
    nearest(app);
    click(app, [9.0, 0.0]);
    click(app, [30.0, 0.0]);
    let doc = app.document.as_mut().expect("a drawing");
    let mut wall = doc.model.get(Slot(1)).expect("the wall").clone();
    if let Entity::Line(l) = &mut wall {
        l.b.x = E + 40.0;
    }
    assert!(doc.model.update(Slot(1), wall));
    fitted(app, [-4.0, -8.0], [44.0, 16.0]);
    app.selection.set([Slot(2)]);
    run(app, "tool.select");
}

/// A room block (a floor and a door inside it), its insert, and a free dimension beside it.
fn block_ground(app: &mut App) {
    let mut o = Objects::new();
    o.dimension(
        "cizim",
        None,
        [[0.0, -6.0], [6.0, -6.0]],
        None,
        -2.0,
        json!({}),
    );
    open(app, o);
    make_cad(app);
    let block = |v| serde_json::from_value::<BlockDefinition>(v).expect("a block");
    let doc = app.document.as_mut().expect("a drawing");
    doc.model
        .add_block(block(json!({"id": DOOR, "name": "Kapı", "base": {"x": 0, "y": 0}, "entities": [
            {"kind":"line","id":1,"layerId":"0","attrs":{},"a":{"x":0,"y":0},"b":{"x":3,"y":0}},
            {"kind":"arc","id":2,"layerId":"0","attrs":{},"c":{"x":0,"y":0},"r":3,"a0":0,"a1":std::f64::consts::FRAC_PI_2}
        ]})))
        .expect("the door");
    doc.model
        .add_block(block(json!({"id": ROOM, "name": "Oda", "base": {"x": 0, "y": 0}, "entities": [
            {"kind":"polygon","id":1,"layerId":"0","attrs":{},"pts":[{"x":0,"y":0},{"x":12,"y":0},{"x":12,"y":9},{"x":0,"y":9}]},
            {"kind":"insert","id":2,"layerId":"0","attrs":{},"block":DOOR,"p":{"x":2,"y":0},"scale":1,"rotation":0}
        ]})))
        .expect("the room");
    // The insert after its blocks, so that the drawing places its pieces: slot 2.
    let insert = serde_json::from_value::<Entity>(
        json!({"kind":"insert","id":2,"layerId":"cizim","attrs":{},
        "block": ROOM, "p": {"x": E, "y": N}, "scale": 1, "rotation": 0}),
    )
    .expect("an insert");
    assert_eq!(doc.model.add(insert).ok(), Some(Slot(2)));
    fitted(app, [-4.0, -12.0], [20.0, 12.0]);
}

/// Yeniden bağla on the door's end inside the room's insert: “Bağlanacak: bitiş (blokta)”.
fn in_block(app: &mut App) {
    block_ground(app);
    run(app, "tool.dimensionReassociate");
    click(app, [3.0, -8.0]);
    click(app, [2.0, 0.0]);
    hover(app, [5.0, 0.0]);
}

/// Both ends bound to the door inside the room; the insert then turned a quarter and moved: the dimension follows.
fn in_block_followed(app: &mut App) {
    in_block(app);
    click(app, [5.0, 0.0]);
    let doc = app.document.as_mut().expect("a drawing");
    let mut ins = doc.model.get(Slot(2)).expect("the insert").clone();
    if let Entity::Insert(i) = &mut ins {
        i.p.x = E + 16.0;
        i.p.y = N + 2.0;
        i.rotation = std::f64::consts::FRAC_PI_2;
    }
    assert!(doc.model.update(Slot(2), ins));
    fitted(app, [-4.0, -12.0], [20.0, 16.0]);
    app.selection.set([Slot(1)]);
    run(app, "tool.select");
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("yer-yakin", nearest),
        ("yer-yakin-izler", nearest_followed),
        ("yer-blok", in_block),
        ("yer-blok-izler", in_block_followed),
    ]
}
