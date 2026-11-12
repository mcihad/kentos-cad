//! The pictures of docs/adr/0324: a material list hanging on its tabs with a stacked fraction and a two-column note,
//! the list selected with Öznitelikler's Girinti, Sol girinti, Sekmeler and Sütunlar (yazi-duzeni); the editor's
//! Liste ▾ over the list (yazi-liste); Yazı tipleri with a DXF's SHX and Times styles (yazi-tipleri).
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=yazi-duzeni,yazi-liste,yazi-tipleri cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::Entity;
use kentos_domain::Slot;
use serde_json::json;

use crate::app::App;
use crate::files_testing::make_cad;
use crate::tools_scenes::{E, N, Objects, hover, layer, open_edited, run};
use crate::tools_screens::Scene;

/// A list with tabs and a fraction, and a note in two columns.
fn drawing(app: &mut App) -> (u32, u32) {
    let mut o = Objects::new();
    let list = o.push(
        "yazi",
        json!({ "kind": "text", "p": { "x": E, "y": N + 12.0 }, "height": 1.0, "rotation": 0.0, "align": "topLeft",
                "text": "Malzeme listesi\n•\tKum\t2 m³\n•\tÇakıl 8/16\t3 m³\n•\tÇimento\t1/2 ton",
                "boxWidth": 16.0,
                "runs": [{ "start": 0, "end": 15, "bold": true }, { "start": 35, "end": 39, "script": "stack" },
                         { "start": 55, "end": 58, "script": "stack" }],
                "layout": { "indent": -1.5, "leftIndent": 1.5, "tabs": [1.5, 11.0] } }),
    );
    let note = o.push(
        "yazi",
        json!({ "kind": "text", "p": { "x": E + 20.0, "y": N + 12.0 }, "height": 0.8, "rotation": 0.0, "align": "topLeft",
                "text": "1. Ölçüler metredir.\n2. Kotlar deniz seviyesindendir.\n3. Malzeme TS EN 197'ye uyar.\n4. Beton C30/37.\n5. Donatı B420C.\n6. Pas payı 5 cm.",
                "boxWidth": 9.0,
                "layout": { "indent": -1.2, "leftIndent": 1.2, "columns": { "count": 2, "gap": 1.0, "height": 4.5 } } }),
    );
    open_edited(app, o, |d| {
        d["layers"] = json!([layer("yazi", "Yazı", "#3E63DD", 0.25)]);
        d["activeLayer"] = json!("yazi");
        d["settings"]["plotScale"] = json!(100);
    });
    make_cad(app);
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: E - 2.0,
            min_y: N - 1.0,
            max_x: E + 42.0,
            max_y: N + 15.0,
        },
        24.0,
    );
    (list, note)
}

fn laid(app: &mut App) {
    let (list, _) = drawing(app);
    app.tab = "annotate";
    app.selection.set([Slot(list)]);
    let upper =
        kentos_ui::widget::docking::Slot::Docked(kentos_ui::widget::docking::Side::Right, 0);
    let _ = app.update(crate::app::Message::Dock(
        kentos_ui::widget::docking::Event::Collapsed(upper, true),
    ));
    let _ = app.update(crate::app::Message::Properties(
        crate::properties::Event::Toggle("general"),
    ));
    hover(app, [30.0, 1.0]);
}

fn listed(app: &mut App) {
    let (list, _) = drawing(app);
    let t = match app.document.as_ref().and_then(|d| d.model.get(Slot(list))) {
        Some(Entity::Text(t)) => t.clone(),
        _ => panic!("the list"),
    };
    app.edit_paragraph(Slot(list), &t);
    let _ = app.update(crate::app::Message::Paragraph(
        crate::paragraph_editor::Event::Menu(Some(crate::paragraph_editor::Menu::List)),
    ));
}

fn fonts(app: &mut App) {
    let _ = drawing(app);
    if let Some(doc) = app.document.as_mut() {
        let mut settings = doc.model.settings().clone();
        settings.text_styles = serde_json::from_value(json!([
            { "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0601", "name": "Ada no", "font": "arimo", "fontFile": "arial.ttf" },
            { "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0602", "name": "Pafta adı", "font": "barlow", "fontFile": "romans.shx" },
            { "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0603", "name": "Başlık", "font": "overpass", "fontFile": "times.ttf" },
            { "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d0604", "name": "Not", "font": "barlow" }
        ]))
        .expect("styles");
        doc.model.set_settings(settings);
    }
    run(app, "text.fonts");
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("yazi-duzeni", laid),
        ("yazi-liste", listed),
        ("yazi-tipleri", fonts),
    ]
}
