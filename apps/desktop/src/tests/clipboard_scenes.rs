//! The pictures of docs/adr/0309 (the clipboard by its commands): Taban noktasıyla kopyala on a parcel's corner; a
//! copy of parcels, a building and its name from a drawing whose layers are in a Kadastro group, pasted into another
//! drawing (the ghost, then the layers and the text style it brought, said in the command line); and the same copy
//! pasted as one block. `tools_screens` takes them in the dark and the light theme at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=pano-tabanla,pano-baska-cizim,pano-katmanlar,pano-blok cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_domain::Slot;
use serde_json::json;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, forget, hover, layer, open, open_edited, run, typed};
use crate::tools_screens::Scene;

const STYLE: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0001";

/// The source: two parcels, a building and its name on layers of a Kadastro group, the name in the style Başlık.
fn source(app: &mut App) {
    let mut o = Objects::new();
    o.path(
        "k-parsel",
        &[[0.0, 0.0], [18.0, 0.0], [18.0, 14.0], [0.0, 14.0]],
        true,
    );
    o.path(
        "k-parsel",
        &[[18.0, 0.0], [34.0, 0.0], [34.0, 14.0], [18.0, 14.0]],
        true,
    );
    o.path(
        "k-bina",
        &[[4.0, 4.0], [12.0, 4.0], [12.0, 10.0], [4.0, 10.0]],
        true,
    );
    let name = o.text("k-bina", [5.0, 6.5], "101/5", 1.2, 0.0);
    o.fields(name, json!({ "textStyle": STYLE }));
    open_edited(app, o, |d| {
        let mut group = layer("kadastro", "Kadastro", "fg", 0.25);
        group["type"] = json!("group");
        group["children"] = json!([
            layer("k-parsel", "Parsel", "#3E63DD", 0.35),
            layer("k-bina", "Bina", "#E5484D", 0.5),
        ]);
        d["layers"] = json!([layer("cizim", "Çizim", "fg", 0.25), group]);
        d["settings"]["textStyles"] =
            json!([{ "id": STYLE, "name": "Başlık", "font": "arimo", "bold": true }]);
    });
    make_cad(app);
    fit(app);
}

fn fit(app: &mut App) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 4.0,
            min_y: N - 4.0,
            max_x: E + 70.0,
            max_y: N + 20.0,
        },
        24.0,
    );
}

/// Taban noktasıyla kopyala: the parcels and the building selected, the base point on the first parcel's corner.
fn based(app: &mut App) {
    source(app);
    app.selection.set([Slot(1), Slot(2), Slot(3), Slot(4)]);
    run(app, "tool.copyBase");
    hover(app, [0.05, 0.05]);
}

/// The copy taken and another drawing opened: Yapıştır's ghost follows the cursor by the base point.
fn elsewhere(app: &mut App) {
    source(app);
    app.selection.set([Slot(1), Slot(2), Slot(3), Slot(4)]);
    run(app, "edit.copy");
    let mut o = Objects::new();
    o.line("yol", [-4.0, -2.0], [70.0, -2.0]);
    o.line("yol", [-4.0, 18.0], [70.0, 18.0]);
    open(app, o);
    make_cad(app);
    fit(app);
    forget(app);
    app.command_expanded = true;
    run(app, "edit.paste");
    hover(app, [30.0, 2.0]);
}

/// Clicked: the objects pasted, with the group, its layers and the style said in the command line.
fn pasted(app: &mut App) {
    elsewhere(app);
    typed(app, &format!("{},{}", E + 30.0, N + 2.0));
    hover(app, [66.0, -3.0]);
}

/// Blok olarak yapıştır: one block “Pano”, placed and selected.
fn as_block(app: &mut App) {
    elsewhere(app);
    run(app, "tool.cancel");
    forget(app);
    run(app, "edit.pasteBlock");
    typed(app, &format!("{},{}", E + 30.0, N + 2.0));
    hover(app, [66.0, -3.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("pano-tabanla", based),
        ("pano-baska-cizim", elsewhere),
        ("pano-katmanlar", pasted),
        ("pano-blok", as_block),
    ]
}
