//! The pictures of docs/adr/0541 (data profiles): the reference municipality project (fixtures/profiles/v1) opened with
//! its relation links; Veri profili with its findings, one pressed (its parcel selected and shown); the same window after
//! Uygula (what Uygula added gone, the data's own findings left); the Veri tab with its Standart panel. `tools_screens`
//! takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=profil-bulgular,profil-uygulandi,profil-serit cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use serde_json::{Value, json};

use crate::app::{App, Message};
use crate::tools_scenes::{E, N, hover, run};
use crate::tools_screens::Scene;
use kentos_native_application::{ExecutionContext, set};

const PROJECT: &str = include_str!("../../../fixtures/profiles/v1/belediye.json");
const LINKS: &str = include_str!("../../../fixtures/profiles/v1/links.json");

/// The reference project opened, its links written, the view on its district.
fn opened(app: &mut App) {
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(PROJECT).expect("the project reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let links: Value = serde_json::from_str(LINKS).expect("the links read");
    let Some(doc) = app.document.as_mut() else {
        return;
    };
    let uid_of = |model: &kentos_domain::Document, id: u64| {
        model
            .entities_with_uids()
            .find(|(_, e)| u64::from(e.base().id) == id)
            .map(|(u, _)| u.to_string())
    };
    for l in links["links"].as_array().into_iter().flatten() {
        let (Some(child), Some(parent)) = (
            uid_of(&doc.model, l["child"].as_u64().unwrap_or(0)),
            uid_of(&doc.model, l["parent"].as_u64().unwrap_or(0)),
        ) else {
            continue;
        };
        let field = l["field"].as_str().unwrap_or_default();
        let input = serde_json::from_value(json!({
            "uids": [child], "operation": "attributes", "attrs": { field: parent }
        }))
        .expect("the input reads");
        let _ = set::execute(&mut ExecutionContext::new(&mut doc.model), input);
    }
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds {
            min_x: 500_000.0 - 10.0,
            min_y: 4_400_000.0 - 10.0,
            max_x: 500_000.0 + 210.0,
            max_y: 4_400_000.0 + 130.0,
        },
        24.0,
    );
    hover(app, [500_230.0 - E, 4_399_980.0 - N]);
}

/// The findings, the area difference pressed: its parcel selected and its whole message under the table.
fn findings(app: &mut App) {
    opened(app);
    run(app, "profile.window");
    let at = app.data_profile_finding("areaDifference").unwrap_or(0);
    let _ = app.update(Message::DataProfile(crate::data_profile::Event::Pick(at)));
}

/// After Uygula: the plan's layers, field, lists, relation and address data added; the data's own findings left.
fn applied(app: &mut App) {
    opened(app);
    run(app, "profile.window");
    let _ = app.update(Message::DataProfile(crate::data_profile::Event::Apply));
}

/// The Veri tab with its Standart panel.
fn ribbon(app: &mut App) {
    opened(app);
    let _ = app.update(Message::RibbonTab("data"));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("profil-bulgular", findings),
        ("profil-uygulandi", applied),
        ("profil-serit", ribbon),
    ]
}
