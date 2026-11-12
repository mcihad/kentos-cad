//! The pictures of docs/adr/0304 (Özellik kopyala by its command) in a CAD project: three parcels, the first with its
//! own colour, weight and dashes; a title text in a bold monospaced face beside two plain notes; two dimensions, one
//! with open arrows, three decimals and a suffix. The parcel picked as the source, the tag beside the cursor and the
//! prompt's switches; Katman and Renk turned off; the title's face, height and width given to the notes and the
//! dimension's look to the other. `tools_screens` takes them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=ozellik-kopyala-kaynak,ozellik-kopyala-anahtar,ozellik-kopyala-yazi,ozellik-kopyala-olcu cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::json;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, click, hover, open, run, typed};
use crate::tools_screens::Scene;

/// Parcels (1–3), the title (4) and two notes (5, 6), two dimensions (7, 8).
fn ground() -> Objects {
    let mut o = Objects::new();
    let source = o.path(
        "parsel",
        &[[0.0, 0.0], [12.0, 0.0], [12.0, 9.0], [0.0, 9.0]],
        true,
    );
    o.fields(
        source,
        json!({ "color": "#E5484D", "lineWeight": 0.7, "lineType": "dashed", "lineTypeScale": 2 }),
    );
    o.path(
        "cizim",
        &[[15.0, 0.0], [27.0, 0.0], [27.0, 9.0], [15.0, 9.0]],
        true,
    );
    o.path(
        "cizim",
        &[[30.0, 0.0], [42.0, 0.0], [42.0, 9.0], [30.0, 9.0]],
        true,
    );
    let title = o.text("cizim", [0.0, 13.0], "ADA 101", 2.0, 0.0);
    o.fields(
        title,
        json!({ "font": "plex-mono", "bold": true, "widthFactor": 0.85 }),
    );
    o.text("cizim", [15.0, 13.0], "Parsel 2", 1.0, 0.0);
    o.text("cizim", [30.0, 13.0], "Parsel 3", 1.0, 0.0);
    o.dimension(
        "cizim",
        Some("aligned"),
        [[0.0, -1.0], [12.0, -1.0]],
        None,
        -2.0,
        json!({ "height": 1.0, "arrow": "open", "decimals": 3, "suffix": " m", "mask": true }),
    );
    o.dimension(
        "cizim",
        Some("aligned"),
        [[15.0, -1.0], [27.0, -1.0]],
        None,
        -2.0,
        json!({ "height": 0.6 }),
    );
    o
}

fn drawn(app: &mut App) {
    open(app, ground());
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 2.0,
            min_y: N - 6.0,
            max_x: E + 44.0,
            max_y: N + 17.0,
        },
        24.0,
    );
}

/// The parcel picked: its outline in the snap colour, the tag beside the cursor over the second parcel.
fn source(app: &mut App) {
    drawn(app);
    run(app, "tool.matchProperties");
    click(app, [0.0, 4.0]);
    hover(app, [21.0, 4.5]);
}

/// Katman and Renk turned off, then the second parcel clicked: dashes and weight alone.
fn switches(app: &mut App) {
    source(app);
    typed(app, "K");
    typed(app, "R");
    click(app, [15.0, 4.0]);
    hover(app, [36.0, 4.5]);
}

/// The title's face, height and width given to both notes by a window.
fn texts(app: &mut App) {
    drawn(app);
    run(app, "tool.matchProperties");
    click(app, [3.0, 13.8]);
    click(app, [17.0, 13.4]);
    click(app, [32.0, 13.4]);
    hover(app, [40.0, 16.0]);
}

/// The dimension's look given to the other.
fn dimensions(app: &mut App) {
    drawn(app);
    run(app, "tool.matchProperties");
    click(app, [6.0, -3.0]);
    click(app, [21.0, -3.0]);
    hover(app, [40.0, -4.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("ozellik-kopyala-kaynak", source),
        ("ozellik-kopyala-anahtar", switches),
        ("ozellik-kopyala-yazi", texts),
        ("ozellik-kopyala-olcu", dimensions),
    ]
}
