//! The pictures of docs/adr/0322 §4–§5: the Yazı stilleri window with Dışa aktar… and Al… (stil-dosya); parcel
//! numbers in the Ada no style, one set to another typeface by hand and the style changed since, the hand-set one
//! selected with Öznitelikler's Stilden ayrılan row (stil-gecersiz). `tools_screens` takes them in both themes at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=stil-dosya,stil-gecersiz cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_contracts::{CommandResult, EntitiesEdit, StylesEdit};
use kentos_domain::Slot;
use kentos_native_application::ExecutionContext;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited};
use crate::tools_screens::Scene;

const ADA: &str = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0101";

fn drawing(app: &mut App) {
    let mut o = Objects::new();
    for (i, x) in [0.0, 8.0, 16.0, 24.0].into_iter().enumerate() {
        o.path(
            "parsel",
            &[[x, 0.0], [x + 7.0, 0.0], [x + 7.0, 9.0], [x, 9.0]],
            true,
        );
        let t = o.text("yazi", [x + 2.6, 4.0], &format!("{}", 101 + i), 1.75, 0.0);
        o.fields(
            t,
            serde_json::json!({ "textStyle": ADA, "font": "arimo", "bold": true }),
        );
    }
    open_edited(app, o, |d| {
        d["layers"] = serde_json::json!([
            layer("parsel", "Parsel", "#3E63DD", 0.35),
            layer("yazi", "Yazı", "#E5484D", 0.18),
        ]);
        d["settings"]["plotScale"] = serde_json::json!(500);
        d["settings"]["textStyles"] = serde_json::json!([
            { "id": ADA, "name": "Ada no", "font": "arimo", "bold": true, "height": 3.5 },
            { "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0102", "name": "Yol adı", "font": "overpass", "italic": true },
        ]);
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 2.0,
            min_y: N - 3.0,
            max_x: E + 33.0,
            max_y: N + 12.0,
        },
        24.0,
    );
}

fn window(app: &mut App) {
    drawing(app);
    app.open_annotation_styles(crate::annotation_styles::Kind::Text);
}

/// The second number's typeface set to Overpass by hand, then the style's to Overpass and to Barlow: the hand-set one
/// keeps Overpass; it is selected.
fn overridden(app: &mut App) {
    drawing(app);
    let doc = &mut app.document.as_mut().expect("a drawing").model;
    let second = doc.uid(Slot(4)).expect("the second number").to_string();
    let text = match doc.get(Slot(4)) {
        Some(e) => serde_json::to_value(e).expect("json"),
        None => panic!("the second number"),
    };
    let mut geometry = text.clone();
    for k in ["id", "layerId", "attrs"] {
        geometry.as_object_mut().expect("an object").remove(k);
    }
    geometry["font"] = serde_json::json!("overpass");
    let input: EntitiesEdit = serde_json::from_value(serde_json::json!({
        "operation": "properties",
        "changes": [{ "kind": "update", "uid": second, "geometry": geometry }],
    }))
    .expect("an edit");
    assert!(matches!(
        kentos_native_application::edit::execute(&mut ExecutionContext::new(doc), input),
        CommandResult::Completed { .. }
    ));
    for font in ["overpass", "barlow"] {
        let input: StylesEdit = serde_json::from_value(serde_json::json!({
            "kind": "text", "operation": "update",
            "text": { "id": ADA, "name": "Ada no", "font": font, "bold": true, "height": 3.5 },
        }))
        .expect("an update");
        assert!(matches!(
            kentos_native_application::styles_edit::execute(&mut ExecutionContext::new(doc), input),
            CommandResult::Completed { .. }
        ));
    }
    app.selection.set([Slot(4)]);
    hover(app, [32.0, 11.0]);
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![("stil-dosya", window), ("stil-gecersiz", overridden)]
}
