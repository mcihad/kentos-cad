//! The pictures of docs/adr/0346 (the work template and the phase view): Mimari şablonu on an empty CAD drawing, its
//! layers and the message; the room of docs/adr/0342 with its outer wall existing, its inner wall to be demolished and a
//! new partition, in Tümü, Yıkım planı and Yeni durum. `tools_screens` takes them in both themes at 1440×900 and
//! 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=mimari-sablon,evre-tumu,evre-yikim,evre-yeni cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_contracts::{ArchPhase, Entity, WallChange, WallsEdit, WallsOperation};
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

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("mimari-sablon", template),
        ("evre-tumu", all),
        ("evre-yikim", demolition),
        ("evre-yeni", proposed),
    ]
}
