//! The pictures of docs/adr/1120 (SHX unifont and Big Font text in line types): lines, an arc and a circle whose
//! line types write “ŞU” in the test unifont and Big Font codes beside it (fixtures/shx/v1), a type whose text has
//! a character the font lacks (a visible loss, never a gap), and Çizgi tipleri showing them. `tools_screens` takes
//! them:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=shx-yazi-cizim,shx-yazi-yakin,shx-yazi-pencere,shx-yazi-kayip cargo test -p kentos-desktop tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use crate::app::{App, Message};
use crate::line_types_window::Event;
use crate::tools_scenes::{E, N, Objects, layer, open_edited};
use crate::tools_screens::Scene;
use serde_json::json;

fn resources() -> serde_json::Value {
    let item = |file: &str, bytes: &[u8]| {
        kentos_commands::shape_resource::item(file, bytes).expect("a test SHX resource")
    };
    json!([
        item("kentos-uni.shx", include_bytes!("../../../fixtures/shx/v1/kentos-uni.shx")),
        item("kentos-big.shx", include_bytes!("../../../fixtures/shx/v1/kentos-big.shx")),
    ])
}

fn text_type(name: &str, text: &str, style: &str, gap: f64) -> serde_json::Value {
    json!({"name": name, "unit": "drawing", "elements": [
        {"length": 3.0}, {"length": -0.5},
        {"length": 0.0, "text": {"text": text, "style": style, "scale": 1.2, "x": 0.3, "y": -0.6}},
        {"length": -gap}]})
}

fn drawing(app: &mut App) {
    let mut o = Objects::new();
    o.line("uni", [0.0, 16.0], [44.0, 16.0]);
    o.line("big", [0.0, 9.0], [44.0, 9.0]);
    o.line("kayip", [0.0, 2.0], [44.0, 2.0]);
    o.arc("uni", [58.0, 9.0], 7.0, 0.0, std::f64::consts::PI);
    o.circle("big", [58.0, 9.0], 4.0);
    open_edited(app, o, |d| {
        let pattern = |id: &str, name: &str, color: &str, pattern: &str| {
            let mut l = layer(id, name, color, 0.25);
            l["style"]["lineType"] = json!("dashed");
            l["style"]["linePattern"] = json!(pattern);
            l
        };
        d["layers"] = json!([
            pattern("uni", "UNIFONT", "#3E63DD", "SU_UNI"),
            pattern("big", "BIGFONT", "#E5484D", "SU_BIG"),
            pattern("kayip", "KAYIP", "#30A46C", "SU_KAYIP"),
        ]);
        d["activeLayer"] = json!("uni");
        d["settings"]["drawingUnit"] = json!("m");
        d["settings"]["textStyles"] = json!([
            {"id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1120", "name": "Uni", "font": "barlow",
             "fontFile": "kentos-uni.shx", "bigFontFile": "kentos-big.shx"}
        ]);
        d["settings"]["lineTypes"] = json!([
            text_type("SU_UNI", "ŞU", "Uni", 3.5),
            text_type("SU_BIG", "Ş\\M+18140\\M+18141\\M+18142U", "Uni", 7.0),
            text_type("SU_KAYIP", "ŞQU", "Uni", 3.5),
        ]);
        d["styles"]["items"] = resources();
    });
    crate::files_testing::make_cad(app);
}

fn fit(app: &mut App, b: [f64; 4]) {
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: E + b[0], min_y: N + b[1], max_x: E + b[2], max_y: N + b[3] },
        24.0,
    );
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("shx-yazi-cizim", |app| {
            drawing(app);
            fit(app, [-2.0, -1.0, 67.0, 19.0]);
        }),
        ("shx-yazi-yakin", |app| {
            drawing(app);
            fit(app, [2.0, 7.0, 16.0, 18.0]);
        }),
        ("shx-yazi-pencere", |app| {
            drawing(app);
            let _ = app.update(Message::Run("lineTypes.manage"));
            let _ = app.update(Message::LineTypes(Event::Choose("SU_BIG".into())));
        }),
        ("shx-yazi-kayip", |app| {
            drawing(app);
            let _ = app.update(Message::Run("lineTypes.manage"));
            let _ = app.update(Message::LineTypes(Event::Choose("SU_KAYIP".into())));
        }),
    ]
}
