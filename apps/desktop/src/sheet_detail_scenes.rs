//! The pictures of docs/adr/1110, 1111 and 1112: a sheet viewport's own overrides of an external reference's layers
//! in Dış başvurular and in “Bu değer nereden geliyor?”, the package's resource report, and Detay paftaları with a
//! set of four exact ratios and a table's parts. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=adv14-pencere-ezmesi,adv14-deger-kaynagi,adv14-paket-kaynaklari,adv16-detay-penceresi cargo test -p kentos-desktop --lib tools_screens::screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::path::PathBuf;

use crate::app::{App, Message};
use crate::sheet_details_window::{Event as D, ratios};
use crate::tools_screens::Scene;
use crate::wall_scenes::opened;
use crate::xrefs_window::{Event, Mode, Purpose};

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("adv14-pencere-ezmesi", overrides_window),
        ("adv14-deger-kaynagi", value_origin),
        ("adv14-paket-kaynaklari", package_report),
        ("adv16-detay-penceresi", detail_window),
    ]
}

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures").join(rel)
}

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::Xrefs(e));
}

fn call(app: &mut App, command: &str, input: serde_json::Value) {
    let Some(doc) = app.document.as_mut() else { return };
    let answer = kentos_native_application::call::call(
        &mut doc.model,
        None,
        kentos_commands::CallEnvelope {
            command: command.into(),
            version: Some(1),
            op: kentos_commands::CallOp::Execute,
            request_id: None,
            expected: None,
            relax_constraints: false,
            input,
        },
    );
    assert!(matches!(answer.result, kentos_commands::CommandResult::Completed { .. }), "{command}: {answer:?}");
}

/// A sheet of one fixed map frame named Harita whose own layer overrides are `overrides`.
fn sheet(id: &str, name: &str, item: &str, overrides: serde_json::Value) -> serde_json::Value {
    serde_json::json!({ "export": { "dpi": 300, "fileName": "[% @pafta_adi %]", "format": "png" }, "guides": [], "id": id,
        "items": [{ "bindings": [], "constraints": { "h": "left", "relativeTo": "margins", "v": "top" },
            "frame": { "height": 120000, "left": 20000, "top": 10000, "width": 180000 }, "hidden": false, "id": item,
            "kind": { "clipToAtlasFeature": false, "grids": [], "labelBand": 0, "layers": { "type": "all" },
                "layerOverrides": overrides,
                "overviewFrame": { "cap": "butt", "color": "#d0021b", "dash": [], "join": "miter", "width": 500 },
                "type": "map", "view": { "center": { "x": 5.0, "y": 4.0 }, "rotation": 0, "scale": 100, "type": "fixed" } },
            "locked": false, "name": "Harita", "opacity": 100, "padding": 0, "printable": true, "rotation": 0 }],
        "name": name,
        "page": { "margins": { "bottom": 10000, "left": 20000, "right": 10000, "top": 10000 }, "orientation": "landscape",
            "paper": "a3", "size": { "height": 297000, "width": 420000 } },
        "snapGrid": { "enabled": false, "spacing": 5000, "visible": false }, "variables": [], "variants": [] })
}

/// The architect's plan attached in place; two sheets' viewports draw its walls and doors otherwise: one freezes the
/// walls, the other prints the doors red and thick. Its insert's slot.
fn plan_with_overrides(app: &mut App) -> Option<kentos_domain::Slot> {
    opened(app, crate::tools_scenes::Objects::new(), 50.0);
    app.tab = "insert";
    send(app, Event::Picked(Purpose::Attach, Some(fixture("xref/v1/plan.kcad"))));
    send(app, Event::Name("Plan".into()));
    send(app, Event::Mode(Mode::Attach));
    send(app, Event::InPlace);
    let doc = app.document.as_ref()?;
    let by_name = |name: &str| {
        fn find(nodes: &[kentos_commands::LayerNode], name: &str, inside: bool) -> Option<String> {
            nodes.iter().find_map(|n| {
                let inside = inside || n.name == "Dış başvuru: Plan";
                if inside && n.name == name && n.kind == kentos_commands::LayerNodeType::Layer {
                    Some(n.id.clone())
                } else {
                    find(&n.children, name, inside)
                }
            })
        }
        find(doc.model.layers().nodes(), name, false)
    };
    let (wall, door) = (by_name("Duvar")?, by_name("Kapı")?);
    let insert = doc
        .model
        .entities()
        .find(|e| matches!(e, kentos_commands::Entity::Insert(_)))
        .map(|e| kentos_domain::Slot(e.base().id));
    call(app, "cad.sheets.edit", serde_json::json!({ "ops": [
        { "op": "addSheet", "sheet": sheet("a-101", "A-101 Zemin kat", "plan-a",
            serde_json::json!([{ "layer": wall, "frozen": true }])) },
        { "op": "addSheet", "sheet": sheet("a-102", "A-102 Kapılar", "plan-b",
            serde_json::json!([{ "layer": door, "color": "#E5484D", "lineWeight": 700 }])) }
    ] }));
    if let Some(slot) = insert {
        app.zoom_to_objects(&[f64::from(slot.0)]);
    }
    insert
}

fn overrides_window(app: &mut App) {
    plan_with_overrides(app);
    let _ = app.run("xref.manager");
    let k = app.xrefs.listed.as_ref().and_then(|l| l.iter().position(|x| x.name == "Plan"));
    if let Some(k) = k {
        send(app, Event::Choose(k));
    }
}

fn value_origin(app: &mut App) {
    let Some(slot) = plan_with_overrides(app) else { return };
    crate::elevation_scenes::select(app, &[slot]);
    crate::elevation_scenes::panel_room(app);
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle("geometry")));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle("modelAppearance")));
}

/// The plan with a project line type the walls draw with and one no definition has, packed (planned): the log says
/// the resources and the one missing.
fn package_report(app: &mut App) {
    plan_with_overrides(app);
    if let Some(doc) = app.document.as_mut() {
        let mut settings = doc.model.settings().clone();
        settings.line_types.push(kentos_commands::LineTypeDef {
            name: "GAZ".into(),
            description: "Gaz hattı".into(),
            elements: vec![
                kentos_commands::PatternElement { length: 6.0, text: None, shape: None },
                kentos_commands::PatternElement { length: -2.0, text: None, shape: None },
            ],
            unit: Default::default(),
            source: None,
        });
        doc.model.set_settings(settings);
        let layer = doc.model.layers().active().to_owned();
        if let Ok(e) = serde_json::from_value::<kentos_commands::Entity>(serde_json::json!({
            "kind": "line", "id": 0, "layerId": layer, "attrs": {}, "a": { "x": 0.0, "y": -2.0 }, "b": { "x": 10.0, "y": -2.0 },
            "lineType": "dashed", "linePattern": "KANAL" }))
        {
            let _ = doc.model.add(e);
        }
    }
    let target = std::env::temp_dir().join(format!("kentos-adv14-paket-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&target);
    let Some(input) = app.package_input(&target) else { return };
    let Some(doc) = app.document.as_ref() else { return };
    let done = kentos_headless::package::plan(&doc.model, &input)
        .map(|p| Box::new((p.output.clone(), p.warnings.clone())))
        .map_err(|e| e.message.clone());
    let _ = app.update(Message::Exchange(Box::new(crate::exchange::Event::Package(
        crate::exchange::package::Event::Done(done),
    ))));
}

/// Detay paftaları over the details' drawing: the bolt at 1:1, the plate at 1:5 with its dimensions frozen, the
/// whole at 1:100 and the parts list's two parts at 1:20.
fn detail_window(app: &mut App) {
    let fx: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.sheets.details.json")).expect("the cases");
    let model = kentos_domain::Document::from_snapshot(serde_json::from_value(fx["setup"].clone()).expect("a drawing"))
        .expect("opens");
    let doc = crate::document::Document::from_v2(model.to_snapshot_v2(), None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::files_testing::make_cad(app);
    let at = |paper: f64, model: f64| ratios().iter().position(|r| *r == (paper, model)).unwrap_or(0);
    let add = |app: &mut App, slot: u32, ratio: usize| {
        crate::elevation_scenes::select(app, &[kentos_domain::Slot(slot)]);
        if app.sheet_details.is_none() {
            let _ = app.run("sheet.details");
        } else {
            let _ = app.update(Message::SheetDetails(D::FromSelection));
        }
        let k = app.sheet_details.as_ref().map_or(0, |s| s.details.len().saturating_sub(1));
        let _ = app.update(Message::SheetDetails(D::Ratio(k, ratio)));
    };
    add(app, 1, at(1.0, 1.0));
    let _ = app.update(Message::SheetDetails(D::Name(0, "Bulon".into())));
    add(app, 3, at(1.0, 5.0));
    let _ = app.update(Message::SheetDetails(D::Name(1, "Levha".into())));
    let olcu = app
        .document
        .as_ref()
        .map(|_| {
            let mut ids = Vec::new();
            fn walk(nodes: &[kentos_commands::LayerNode], out: &mut Vec<String>) {
                for n in nodes {
                    if n.kind == kentos_commands::LayerNodeType::Layer {
                        out.push(n.id.clone());
                    }
                    walk(&n.children, out);
                }
            }
            if let Some(d) = app.document.as_ref() {
                walk(d.model.layers().nodes(), &mut ids);
            }
            ids.iter().position(|l| l == "olcu").unwrap_or(0)
        })
        .unwrap_or(0);
    let _ = app.update(Message::SheetDetails(D::Freeze(olcu)));
    add(app, 2, at(1.0, 100.0));
    let _ = app.update(Message::SheetDetails(D::Name(2, "Genel".into())));
    crate::elevation_scenes::select(app, &[kentos_domain::Slot(4)]);
    let _ = app.update(Message::SheetDetails(D::FromTable));
    let _ = app.update(Message::SheetDetails(D::Choose(1)));
}
