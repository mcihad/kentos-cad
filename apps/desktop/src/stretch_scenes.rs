//! The pictures of docs/adr/0297 (Esnet: only the window's vertices, what
//! follows them) in a CAD project: a room (an area) with its associative
//! hatch and two dimensions tied to its corners, and a door's swing (an
//! arc). The room's east wall stretched 2 m east: the room dashed, the hatch
//! and the dimensions that follow it dotted in the snap colour (the plan's
//! `dependents`); what was written with the command's message; the swing's
//! one end moved, its chord height kept. `tools_screens` takes them in the
//! dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=esnet-izleyenler,esnet-yazildi,esnet-yay cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_contracts::{
    DimensionRefs, Entity, EntityId, HatchAssoc, SubPlace, SubRef, Vec2 as Point,
};
use kentos_domain::Slot;
use serde_json::json;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run};
use crate::tools_screens::Scene;

const SOLID: &str = r#"{"type":"solid","angle":0,"spacing":1}"#;

/// The room (1), its hatch (2), the south (3) and east (4) dimensions and the door's swing (5).
fn ground() -> Objects {
    let mut o = Objects::new();
    let room = [[0.0, 0.0], [8.0, 0.0], [8.0, 5.0], [0.0, 5.0]];
    o.path("parsel", &room, true);
    o.hatch(
        "parsel",
        &room,
        serde_json::from_str(SOLID).expect("a pattern"),
    );
    let small = json!({ "height": 0.3 });
    o.dimension(
        "cizim",
        None,
        [[0.0, 0.0], [8.0, 0.0]],
        None,
        -1.2,
        small.clone(),
    );
    o.dimension("cizim", None, [[8.0, 0.0], [8.0, 5.0]], None, -1.2, small);
    o.arc(
        "cizim",
        [12.5, 1.5],
        1.5,
        std::f64::consts::FRAC_PI_2,
        std::f64::consts::PI,
    );
    o
}

/// The hatch tied to the room and the dimensions to its corners, as Tarama and
/// the dimension tools would have tied them.
fn tie(app: &mut App) {
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let room = EntityId(doc.uid(Slot(1)).expect("the room").into_bytes());
    let corner = |index: u32| SubRef {
        uid: room,
        place: SubPlace::Vertex {
            part: 0,
            ring: 0,
            index,
        },
        path: Vec::new(),
        broken: None,
    };
    if let Some(Entity::Hatch(h)) = doc.get(Slot(2)).cloned() {
        let tied = HatchAssoc {
            outer: room,
            islands: Vec::new(),
            cutouts: Vec::new(),
            seed: Point {
                x: E + 4.0,
                y: N + 2.5,
            },
        };
        let _ = doc.update(
            Slot(2),
            Entity::Hatch(kentos_contracts::HatchEntity {
                assoc: Some(tied),
                ..h
            }),
        );
    }
    for (slot, a, b) in [(3, 0, 1), (4, 1, 2)] {
        if let Some(Entity::Dimension(d)) = doc.get(Slot(slot)).cloned() {
            let refs = DimensionRefs {
                a: Some(corner(a)),
                b: Some(corner(b)),
                c: None,
            };
            let _ = doc.update(
                Slot(slot),
                Entity::Dimension(kentos_contracts::DimensionEntity {
                    refs: Some(Box::new(refs)),
                    ..d
                }),
            );
        }
    }
}

/// The drawing in a CAD project, its ties made, Esnet running.
fn opened(app: &mut App) {
    open(app, ground());
    make_cad(app);
    tie(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 3.0,
            min_y: N - 4.0,
            max_x: E + 14.0,
            max_y: N + 8.0,
        },
        24.0,
    );
    run(app, "tool.stretch");
}

/// The window over the room's east wall, the base on its south corner and the
/// cursor 2 m east: the room dashed, the hatch and dimensions that follow dotted.
fn followers(app: &mut App) {
    opened(app);
    click(app, [7.0, -0.5]);
    click(app, [9.0, 5.5]);
    click(app, [8.0, 0.0]);
    hover(app, [10.0, 0.0]);
}

/// What was written: the wider room, its hatch and dimensions followed, the message.
fn written(app: &mut App) {
    followers(app);
    click(app, [10.0, 0.0]);
    hover(app, [12.0, 7.0]);
}

/// The door's swing: its west end moved 0.8 m south, its chord height kept.
fn swing(app: &mut App) {
    opened(app);
    click(app, [10.6, 1.1]);
    click(app, [11.4, 1.9]);
    click(app, [11.0, 1.5]);
    hover(app, [11.0, 0.7]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("esnet-izleyenler", followers),
        ("esnet-yazildi", written),
        ("esnet-yay", swing),
    ]
}
