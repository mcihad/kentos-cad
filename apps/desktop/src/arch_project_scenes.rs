//! The pictures of docs/adr/0346 (the work template and the phase view): Mimari şablonu on an empty CAD drawing, its
//! layers and the message; the room of docs/adr/0342 with its outer wall existing, its inner wall to be demolished and a
//! new partition, in Tümü, Yıkım planı and Yeni durum. And of docs/adr/0347: Pafta seti over the room with the template's
//! sheets (their frames on the room, a section mark on the sections' frame), alone and with a wall and the mark
//! selected; the room's DXF read back into an empty drawing, a wall selected. `tools_screens` takes them in both themes
//! at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=mimari-sablon,evre-tumu,evre-yikim,evre-yeni,pafta-seti,pafta-seti-etkilenen,dxf-mimari cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_commands::{ArchPhase, Entity, WallChange, WallsEdit, WallsOperation};
use kentos_domain::Slot;

use crate::app::App;
use crate::tools_scenes::{hover, run};
use crate::tools_screens::Scene;
use crate::wall_scenes::{fit, opened};

/// Mimari şablonu on an empty CAD drawing, on the Mimari tab.
fn template(app: &mut App) {
    opened(app, crate::tools_scenes::Objects::new(), 100.0);
    run(app, "arch.template");
    app.tab = "arch";
}

/// The walls' persistent ids and their styles' ids.
fn walls(app: &App) -> Vec<(String, String)> {
    let Some(doc) = app.document.as_ref() else {
        return Vec::new();
    };
    doc.model
        .entities()
        .filter_map(|e| match e {
            Entity::Wall(w) => doc
                .model
                .uid(Slot(e.base().id))
                .map(|u| (w.wall.style.clone().unwrap_or_default(), u.to_string())),
            _ => None,
        })
        .collect()
}

fn phase(app: &mut App, uid: &str, p: ArchPhase) {
    let Some(doc) = app.document.as_mut() else {
        return;
    };
    let _ = kentos_native_application::walls_edit::execute(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        WallsEdit {
            uids: vec![uid.to_owned()],
            operation: WallsOperation::Set(WallChange {
                phase: Some(Some(p)),
                ..Default::default()
            }),
            expected_revision: None,
        },
    );
}

/// The room: its outer wall existing, the inner wall to be demolished, a new partition in the left room.
fn phased(app: &mut App) {
    crate::space_scenes::spaces(app);
    fit(app, 7.0, 5.0);
    // The partition across the living room, through the command (no snap moves its ends).
    if let Some(doc) = app.document.as_mut() {
        let at = |x: f64, y: f64| {
            serde_json::json!({"x": crate::tools_scenes::E + x, "y": crate::tools_scenes::N + y})
        };
        let layer = doc.model.layers().active().to_owned();
        if let Ok(input) = serde_json::from_value(serde_json::json!({
            "layerId": layer,
            "walls": [{"pts": [at(0.0, 2.6), at(4.2, 2.6)], "style": "kentos-bolme-10"}]
        })) {
            let _ = kentos_native_application::walls_create::execute(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                input,
            );
        }
    }
    // By their styles: the outer walls existing, the inner one to go, the partition new.
    for (style, uid) in walls(app) {
        let p = if style.contains("dis") {
            ArchPhase::Existing
        } else if style.contains("bolme") {
            ArchPhase::New
        } else {
            ArchPhase::Demolish
        };
        phase(app, &uid, p);
    }
    app.tab = "arch";
    hover(app, [9.0, -1.0]);
}

fn all(app: &mut App) {
    phased(app);
    run(app, "arch.phase.all");
}

fn demolition(app: &mut App) {
    phased(app);
    run(app, "arch.phase.demolition");
}

fn proposed(app: &mut App) {
    phased(app);
    run(app, "arch.phase.proposed");
}

/// A sheet's map frame looking at `(x, y)` of the room at `scale`, through `cad.sheets.edit`.
fn look_at(app: &mut App, item: &str, x: f64, y: f64, scale: u32) {
    let Some(doc) = app.document.as_mut() else {
        return;
    };
    let kind = serde_json::json!({"clipToAtlasFeature": false, "grids": [], "labelBand": 0, "layers": {"type": "all"},
        "overviewFrame": {"cap": "butt", "color": "#d0021b", "dash": [], "join": "miter", "width": 500},
        "type": "map", "view": {"center": {"x": crate::tools_scenes::E + x, "y": crate::tools_scenes::N + y},
        "rotation": 0, "scale": scale, "type": "fixed"}});
    if let Ok(input) = serde_json::from_value(serde_json::json!({
        "ops": [{"op": "setItemProps", "id": item, "patch": {"kind": kind}}]
    })) {
        let _ = kentos_native_application::sheets_edit::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            input,
        );
    }
}

/// The room with the template's sheets: the plan and the sections on the room, the details on the kitchen, a section
/// mark through the room pointing at the sections' frame.
fn sheet_set(app: &mut App) {
    crate::space_scenes::spaces(app);
    fit(app, 7.0, 5.0);
    run(app, "arch.template");
    look_at(app, "a-101-harita", 3.5, 2.5, 50);
    look_at(app, "a-301-harita", 3.5, 2.5, 50);
    look_at(app, "a-501-harita", 5.6, 1.0, 20);
    if let Some(doc) = app.document.as_mut() {
        let layer = doc.model.layers().active().to_owned();
        let at = |x: f64, y: f64| kentos_commands::Vec2 {
            x: crate::tools_scenes::E + x,
            y: crate::tools_scenes::N + y,
        };
        let _ = kentos_native_application::viewmarks_create::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            kentos_commands::ViewMarksCreate {
                layer_id: layer,
                marks: vec![kentos_commands::NewViewMark {
                    form: kentos_commands::ViewMarkForm::Section,
                    points: vec![at(-1.6, 2.2), at(8.6, 2.2)],
                    radius: None,
                    at: None,
                    angle: None,
                    flip: false,
                    mark: "A".into(),
                    target: Some(kentos_commands::ViewTarget::sheet("a-301", "a-301-harita")),
                    size: None,
                }],
                color: None,
                line_weight: None,
                expected_revision: None,
            },
        );
    }
    app.tab = "arch";
}

fn set_alone(app: &mut App) {
    sheet_set(app);
    app.selection.set([]);
    run(app, "arch.sheetSet");
}

/// The kitchen's wall and the section mark selected: the sheets they affect.
fn set_affected(app: &mut App) {
    sheet_set(app);
    let chosen: Vec<Slot> = app
        .document
        .as_ref()
        .map(|d| {
            let wall = walls(app).into_iter().find(|(s, _)| !s.contains("dis")).map(|(_, u)| u);
            d.model
                .entities()
                .filter(|e| match e {
                    Entity::ViewMark(_) => true,
                    Entity::Wall(_) => d.model.uid(Slot(e.base().id)).map(|u| u.to_string()) == wall,
                    _ => false,
                })
                .map(|e| Slot(e.base().id))
                .collect()
        })
        .unwrap_or_default();
    app.selection.set(chosen);
    run(app, "arch.sheetSet");
}

/// The room written to DXF (`fixtures/formats/v1/dxf-arch/mimari.dxf`) read into an empty drawing: its walls,
/// door, space, grids, column, stair, ramp, tag and mark are themselves again; a wall selected.
fn dxf_back(app: &mut App) {
    opened(app, crate::tools_scenes::Objects::new(), 50.0);
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/formats/v1/dxf-arch/mimari.dxf");
    if let Some(doc) = app.document.as_mut()
        && let Ok(input) = serde_json::from_value(serde_json::json!({ "path": path.display().to_string() }))
    {
        let _ = kentos_native_application::file_import::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            input,
        );
    }
    // The room (its walls and space): the space's words at their size, not grown over the others (docs/adr/0205 §5).
    let room: Vec<f64> = app
        .document
        .as_ref()
        .map(|d| {
            d.model
                .entities()
                .filter(|e| matches!(e, Entity::Wall(_) | Entity::Space(_)))
                .map(|e| f64::from(e.base().id))
                .collect()
        })
        .unwrap_or_default();
    app.zoom_to_objects(&room);
    let wall = app.document.as_ref().and_then(|d| {
        d.model
            .entities()
            .find(|e| matches!(e, Entity::Wall(_)))
            .map(|e| Slot(e.base().id))
    });
    if let Some(w) = wall {
        app.selection.set([w]);
    }
    app.tab = "arch";
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("mimari-sablon", template),
        ("evre-tumu", all),
        ("evre-yikim", demolition),
        ("evre-yeni", proposed),
        ("pafta-seti", set_alone),
        ("pafta-seti-etkilenen", set_affected),
        ("dxf-mimari", dxf_back),
    ]
}
