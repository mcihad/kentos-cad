//! Mahal ayırıcı is a building's (docs/adr/0802): KentOS Harita's and KentOS GIS's polylines have no such row, KentOS
//! Mimari's and kentos-cad's have — each program's module registered. Moved from the core's properties tests: it spans
//! the modules (docs/adr/9199 §4.11).

use std::collections::BTreeMap;

use kentos_commands::{DocumentSnapshotV1, Entity, EntityBase, PathEntity, Vec2 as Wire};

use crate::app::{App, Message};
use crate::document::Document;

/// The selection traces' drawing (fixtures/interaction/v1/objects.kcad), open in an area of 800 × 600.
fn objects() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = DocumentSnapshotV1::from_json(include_str!(
        "../../../../fixtures/interaction/v1/objects.kcad"
    ))
    .expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// An open polyline on `cizim` through three vertices 10 m apart.
fn polyline(app: &mut App) -> u32 {
    let wire = |x: f64, y: f64| Wire {
        x: 487000.0 + x,
        y: 4420000.0 + y,
    };
    let base = EntityBase {
        look: Default::default(),
        id: 0,
        layer_id: "cizim".to_owned(),
        color: None,
        attrs: BTreeMap::new(),
        label: None,
        symbol: None,
        line_weight: None,
        label_pins: Vec::new(),
        attachments: Vec::new(),
    };
    let e = Entity::Polyline(PathEntity {
        space_separator: None,
        ms: None,
        base,
        pts: vec![wire(0.0, 40.0), wire(10.0, 40.0), wire(20.0, 40.0)],
        bulges: None,
        holes: None,
        zs: None,
        parts: None,
        closed: false,
        widths: None,
    });
    app.document
        .as_mut()
        .expect("open")
        .model
        .add(e)
        .expect("a slot")
        .0
}

/// Whether Öznitelikler's section has the row.
fn has_row(app: &App, section: &str, label: &str) -> bool {
    let doc = app.document.as_ref().expect("open");
    app.properties_panel(doc)
        .sections
        .into_iter()
        .find(|s| s.title == section)
        .is_some_and(|s| s.rows.iter().any(|r| r.label == label))
}

#[test]
fn a_maps_polyline_has_no_space_separator_row() {
    for (program, shown) in [
        (Some(kentos_launch::Module::Map), false),
        (Some(kentos_launch::Module::Gis), false),
        (Some(kentos_launch::Module::Architecture), true),
        (None, true),
    ] {
        let mut app = objects();
        app.program = program;
        let doc = &mut app.document.as_mut().unwrap().model;
        let mut settings = doc.settings().clone();
        settings.workspace = Some(kentos_commands::Workspace::Cad);
        doc.set_settings(settings);
        let id = polyline(&mut app);
        app.selection.set([kentos_domain::Slot(id)]);
        assert_eq!(
            has_row(&app, "Geometri", "Mahal ayırıcı"),
            shown,
            "{program:?}"
        );
    }
}
