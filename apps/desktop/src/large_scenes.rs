//! The pictures of docs/adr/1062 (CAD-ADV-19): the measured mixed drawing (`perf::mixed`, 10 000 objects from seed
//! 19) on the GPU, far (all of it), near (1:200 over its middle cell: texts, a dimension, a dense hatch, the nested
//! blocks) and close (1:20 over the middle cell's nested block: its texts, 0.3 m circles and solid triangles).
//! The far views again without the long strokes' levels (`-duzeysiz`, docs/adr/1064 §4), and the 100 000 objects'
//! drawing far with and without them, for the difference pictures.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650, on the GPU with
//! `KENTOS_SNAPSHOT_BACKEND=wgpu`:
//!
//! ```text
//! KENTOS_SNAPSHOT_BACKEND=wgpu KENTOS_SHOTS_ONLY=buyuk-uzak,buyuk-yakin,buyuk-milimetrik \
//!   cargo test --release -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! KENTOS_SNAPSHOT_BACKEND=wgpu KENTOS_SHOTS_ONLY=buyuk-uzak-duzeysiz,buyuk-100k-uzak,buyuk-100k-uzak-duzeysiz …
//! ```
//!
//! Test code only.

use kentos_render_wgpu::Vec2;

use crate::app::{App, Message};
use crate::document::Document;
use crate::perf::mixed;
use crate::tools_screens::Scene;

const OBJECTS: usize = 10_000;

fn open(app: &mut App) {
    open_of(app, OBJECTS);
}

fn open_of(app: &mut App, objects: usize) {
    let (snapshot, _) = mixed::drawing(objects, mixed::SEED);
    let doc = Document::from_v2(snapshot, None).expect("the drawing opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    // The project's questions (its type, its CAD work) are not the picture.
    app.dialog = None;
}

/// The view over the drawing's middle, `along` metres east and north of it, at 1:`scale`.
fn look(app: &mut App, along: [f64; 2], scale: f64) {
    let e = mixed::extent(OBJECTS);
    app.viewport.camera.center = Vec2::new(
        (e[0] + e[2]) / 2.0 + along[0],
        (e[1] + e[3]) / 2.0 + along[1],
    );
    app.viewport.camera.scale = 1.0 / (scale * 0.000_264_58);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("buyuk-uzak", open),
        ("buyuk-uzak-duzeysiz", |app| {
            app.styles.line_lod = false;
            open(app);
        }),
        ("buyuk-100k-uzak", |app| open_of(app, 100_000)),
        ("buyuk-100k-uzak-duzeysiz", |app| {
            app.styles.line_lod = false;
            open_of(app, 100_000);
        }),
        ("buyuk-yakin", |app| {
            open(app);
            look(app, [0.0, 0.0], 200.0);
        }),
        ("buyuk-milimetrik", |app| {
            open(app);
            // The middle cell's nested block (its insert is the cell's eighth object): its texts, its 0.3 m circles,
            // its solid triangles, at 1:20.
            let side = mixed::side(OBJECTS);
            let cells = OBJECTS.div_ceil(10);
            let cell = ((cells.div_ceil(side) / 2) * side + side / 2).min(cells - 1);
            let id = (cell * 10 + 7 + 1) as u32;
            let at = app
                .document
                .as_ref()
                .and_then(|d| d.model.get(kentos_domain::Slot(id)))
                .and_then(|e| match e {
                    kentos_commands::Entity::Insert(i) => Some(i.p),
                    _ => None,
                })
                .expect("the middle cell's insert");
            app.viewport.camera.center = Vec2::new(at.x + 2.0, at.y + 1.0);
            app.viewport.camera.scale = 1.0 / (20.0 * 0.000_264_58);
        }),
    ]
}
