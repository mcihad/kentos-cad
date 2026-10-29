//! The pictures of docs/adr/0541, 0558 and 0735 (data profiles): the reference municipality project
//! (fixtures/profiles/v1) opened with its relation links; Veri profili with its findings, one pressed (its parcel selected
//! and shown); the same window after Uygula (what Uygula added gone, the data's own findings left); the Veri tab with its
//! Standart panel; an institution's profile brought in (İçe aktar…, kept inside the drawing beside a folder's profile); an
//! institution's project read through its mapping; the park project's data migration planned and done (its losses said);
//! the drawing's own profile taken out of it, asked first and done. `tools_screens` takes them in both themes at
//! 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=profil-bulgular,profil-uygulandi,profil-serit,profil-ice-aktar,profil-esleme,profil-goc,profil-goc-yapildi,profil-cizimden-kaldir,profil-kaldirildi cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
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
const SIVAS: &str = include_str!("../../../fixtures/profiles/v1/sivas.json");
const SIVAS_LINKS: &str = include_str!("../../../fixtures/profiles/v1/sivas-links.json");
const PARK: &str = include_str!("../../../fixtures/profiles/v1/park-r1.json");
const SIVAS_MAPPING: &str =
    include_str!("../../../fixtures/profiles/v1/files/sivas-bel.eslem.json");
const PARK_PROFILE: &str =
    include_str!("../../../fixtures/profiles/v1/files/park-bahce.profil.json");
const PARK_MAPPING: &str =
    include_str!("../../../fixtures/profiles/v1/files/park-bahce-kurum.eslem.json");
const PEYZAJ: &str = include_str!("../../../fixtures/profiles/v1/files/peyzaj.profil.json");

/// The reference project opened, its links written, the view on its district.
fn opened(app: &mut App) {
    opened_from(app, PROJECT, LINKS, 210.0, 130.0);
}

/// A project opened with its links written, the view on its `width` × `height` metres from the origin; with a folder
/// of the project's own profiles beside its file (`files`: name → text) when there are some.
fn opened_from(app: &mut App, project: &str, links: &str, width: f64, height: f64) {
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(project).expect("the project reads");
    let doc = crate::document::Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    let links: Value = serde_json::from_str(links).expect("the links read");
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
            max_x: 500_000.0 + width,
            max_y: 4_400_000.0 + height,
        },
        24.0,
    );
    hover(app, [500_230.0 - E, 4_399_980.0 - N]);
}

/// The open drawing given a file's place in a folder of its own, with the project's profiles beside it (`files`: the
/// name in the `profiller` folder → its text); the folder is made empty first.
fn beside(app: &mut App, name: &str, files: &[(&str, &str)]) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("kentos-profil-sahne-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("profiller")).expect("the folder");
    for (file, text) in files {
        std::fs::write(dir.join("profiller").join(file), text).expect("the file");
    }
    if let Some(doc) = app.document.as_mut() {
        doc.path = Some(dir.join("cizim.kcad"));
    }
    dir
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

/// An institution's profile brought in: the file chosen, kept inside the drawing (Nereye's default), the profile chosen
/// in the window; the folder beside the drawing keeps another, listed as the folder's.
fn imported(app: &mut App) {
    opened(app);
    let dir = beside(app, "ice-aktar", &[("peyzaj.profil.json", PEYZAJ)]);
    run(app, "profile.window");
    let file = dir.join("park-bahce.json");
    std::fs::write(&file, PARK_PROFILE).expect("the file");
    let _ = app.update(Message::DataProfile(crate::data_profile::Event::Imported(
        Some(file),
    )));
}

/// An institution's project read through its mapping: its own layer, field and code names, the findings in them.
fn through_mapping(app: &mut App) {
    opened_from(app, SIVAS, SIVAS_LINKS, 210.0, 130.0);
    beside(app, "esleme", &[("sivas-bel.eslem.json", SIVAS_MAPPING)]);
    run(app, "profile.window");
    let _ = app.update(Message::DataProfile(crate::data_profile::Event::Mapping(1)));
    let at = app.data_profile_finding("badValue").unwrap_or(0);
    let _ = app.update(Message::DataProfile(crate::data_profile::Event::Pick(at)));
}

/// The park project at release 1 with its profile (release 3) beside it: the window's migration part with its plan.
fn park(app: &mut App) {
    opened_from(app, PARK, r#"{ "links": [] }"#, 90.0, 30.0);
    beside(
        app,
        "goc",
        &[
            ("park-bahce.profil.json", PARK_PROFILE),
            ("park-bahce-kurum.eslem.json", PARK_MAPPING),
        ],
    );
    run(app, "profile.window");
    let _ = app.update(Message::DataProfile(crate::data_profile::Event::Profile(1)));
}

/// The data moved: what it did said, the values that did not fit listed.
fn migrated(app: &mut App) {
    park(app);
    let _ = app.update(Message::DataProfile(crate::data_profile::Event::Migrate));
}

/// The park project carrying its profile and mapping inside the drawing (brought in with Nereye's default): taking the
/// profile out asked first.
fn removing(app: &mut App) {
    opened_from(app, PARK, r#"{ "links": [] }"#, 90.0, 30.0);
    let dir = beside(app, "kaldir", &[]);
    run(app, "profile.window");
    for (name, text) in [("park-bahce.json", PARK_PROFILE), ("park-esleme.json", PARK_MAPPING)] {
        let file = dir.join(name);
        std::fs::write(&file, text).expect("the file");
        let _ = app.update(Message::DataProfile(crate::data_profile::Event::Imported(
            Some(file),
        )));
    }
    let _ = app.update(Message::DataProfile(crate::data_profile::Event::Remove(
        kentos_commands::ProfileFileKind::Profile,
    )));
}

/// The profile taken out, and the drawing's mapping of it with it: KentOS's profile chosen again.
fn removed(app: &mut App) {
    removing(app);
    let _ = app.update(Message::DataProfile(crate::data_profile::Event::Removed(
        true,
    )));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("profil-bulgular", findings),
        ("profil-uygulandi", applied),
        ("profil-serit", ribbon),
        ("profil-ice-aktar", imported),
        ("profil-esleme", through_mapping),
        ("profil-goc", park),
        ("profil-goc-yapildi", migrated),
        ("profil-cizimden-kaldir", removing),
        ("profil-kaldirildi", removed),
    ]
}
