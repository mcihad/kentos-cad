//! The pictures of docs/adr/0314 (a change's reach and the block library): Değişikliğin etkisi for a wall layer whose
//! objects partly keep values of their own, with a door block that follows it and the layer state that keeps its
//! old look; the same for a line type, opened from Çizgi tipleri; the Bloklar panel with Blokları al and Blokları
//! dışa aktar beside the search. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=etki-katman,etki-cizgitipi,blok-kitapligi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::Value;

use crate::app::{App, Message};
use crate::files_testing::make_cad;
use crate::tools_scenes::run;

/// The cases' drawing's corner (style_impact_cases.py's X, Y).
const X: f64 = 500000.0;
const Y: f64 = 4400000.0;
use crate::tools_screens::Scene;

/// The shared cases' drawing of `cad.styles.impact` (scripts/fixtures/style_impact_cases.py).
fn opened(app: &mut App) {
    let path = format!(
        "{}/../../fixtures/commands/v1/cad.styles.impact.json",
        env!("CARGO_MANIFEST_DIR")
    );
    let file: Value =
        serde_json::from_str(&std::fs::read_to_string(path).expect("the cases")).expect("JSON");
    let snapshot =
        kentos_contracts::DocumentSnapshotV1::from_json(&file["setup"].to_string()).expect("reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: X - 5.0,
            min_y: Y - 10.0,
            max_x: X + 60.0,
            max_y: Y + 80.0,
        },
        24.0,
    );
}

fn layer(app: &mut App) {
    opened(app);
    let _ = app.update(Message::StyleImpact(crate::style_impact::Event::Open(
        kentos_contracts::ImpactTarget::Layer,
        "duvar".into(),
    )));
}

fn line_type(app: &mut App) {
    opened(app);
    run(app, "lineTypes.manage");
    let _ = app.update(Message::LineTypes(crate::line_types_window::Event::Choose(
        "EKSEN".into(),
    )));
    let _ = app.update(Message::StyleImpact(crate::style_impact::Event::Open(
        kentos_contracts::ImpactTarget::LineType,
        "EKSEN".into(),
    )));
}

fn blocks(app: &mut App) {
    opened(app);
    run(app, "block.panel");
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("etki-katman", layer),
        ("etki-cizgitipi", line_type),
        ("blok-kitapligi", blocks),
    ]
}
