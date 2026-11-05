//! The pictures of docs/adr/1000 (review notes, taking revision differences): Farkları al over a drawing file's
//! revision with two differences ticked; the split view after the take; İnceleme notları with a stale note bound to
//! a moved parcel, an answered one and its reply; the Açıklama tab's İşaretleme group with İnceleme notları.
//! `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=inceleme-farklari-al,inceleme-farklari-alindi,inceleme-notlari,inceleme-serit cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use kentos_native_application::{ExecutionContext, registry};

use crate::app::{App, Message};
use crate::document::Document;
use crate::split_view::Event;
use crate::tools_screens::Scene;

fn path(name: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/compare/v1/revisions").join(name)
}

fn command(app: &mut App, name: &str, input: serde_json::Value) {
    let doc = app.document.as_mut().expect("open");
    let answer =
        registry::run(&mut ExecutionContext::new(&mut doc.model), name, kentos_commands::CallOp::Execute, input)
            .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
}

fn uid(app: &App, slot: u32) -> String {
    app.document.as_ref().expect("open").model.uid(kentos_domain::Slot(slot)).expect("an object").to_string()
}

/// The parcel map and, in the other view, the file with parcel 102 five metres back, parcel 103 gone and a new line.
fn compared(app: &mut App) {
    let open = Document::read(&path("ayni.kcad")).expect("reads");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(open)))));
    app.tab = "map";
    let _ = app.update(Message::Run("view.zoomExtents"));
    let _ = app.update(Message::Run("view.split"));
    app.revision_from_file(0, &path("tasindi.kcad"));
    let _ = app.update(Message::Run("view.zoomExtents"));
}

fn ticked(app: &mut App) {
    compared(app);
    let _ = app.update(Message::Split(Event::Take(crate::split_revision::TakeEvent::Open(0))));
    let rows: Vec<_> = {
        let r = app.split.as_ref().and_then(|s| s.others[0].revision.as_ref()).expect("a revision");
        r.take_rows(&app.document.as_ref().expect("open").model).into_iter().map(|row| row.uid).collect()
    };
    // The changed parcel and the line only the file has.
    for u in rows.iter().take(2) {
        let _ = app.update(Message::Split(Event::Take(crate::split_revision::TakeEvent::Toggle(0, *u))));
    }
}

fn notes(app: &mut App) {
    let open = Document::read(&path("ayni.kcad")).expect("reads");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(open)))));
    app.tab = "annotate";
    let _ = app.update(Message::Run("view.zoomExtents"));
    let (p102, p101) = (uid(app, 2), uid(app, 1));
    command(
        app,
        "cad.review.notes.edit",
        serde_json::json!({ "change": { "kind": "add", "text": "102 parselinin batı sınırı tapu planıyla uyuşmuyor; 5 m kaydırılmış olabilir.",
            "anchors": [p102], "revision": "B", "author": "Ayşe", "createdAt": "2026-11-05T17:40:00Z" } }),
    );
    command(
        app,
        "cad.review.notes.edit",
        serde_json::json!({ "change": { "kind": "add", "text": "101 parselinin malik bilgisi eksik.", "anchors": [p101],
            "author": "Ayşe", "origin": "ai" } }),
    );
    let second = app.document.as_ref().expect("open").model.settings().review_notes[1].id.clone();
    command(
        app,
        "cad.review.notes.edit",
        serde_json::json!({ "change": { "kind": "reply", "id": second, "text": "Malik B olarak işlendi.", "author": "Cihad", "status": "answered" } }),
    );
    command(
        app,
        "cad.entities.transform",
        serde_json::json!({ "uids": [p102], "transform": { "kind": "move", "dx": 5.0, "dy": 0.0 } }),
    );
    let _ = app.update(Message::Run("review.notes"));
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("inceleme-farklari-al", ticked),
        ("inceleme-farklari-alindi", |app| {
            ticked(app);
            let _ = app.update(Message::Split(Event::Take(crate::split_revision::TakeEvent::Take(0))));
        }),
        ("inceleme-notlari", notes),
        ("inceleme-serit", |app| {
            notes(app);
            let _ = app.update(Message::DialogClosed);
            let _ = app.update(Message::RibbonTab("annotate"));
        }),
    ]
}

/// docs/adr/1002: the shared cases' drawing (two bound office standards, a differing GAZ and A-NOT) with
/// Standart kaynaklarını kur open, GAZ to be added under a new name and A-NOT replaced in place.
fn install(app: &mut App) {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/commands/v1/cad.standards.install.json"
    ))
    .expect("the cases");
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&cases["setup"].to_string()).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::files_testing::make_cad(app);
    let _ = app.update(Message::StandardInstall(crate::standard_install::Event::Open));
    use crate::standard_install::{Choice, Event as I};
    use kentos_commands::{StandardInstallAction as A, StandardResourceKind as K};
    let _ = app.update(Message::StandardInstall(I::Pick(K::LineType, "GAZ".into(), Choice(A::RenameIncoming))));
    let _ = app.update(Message::StandardInstall(I::Pick(K::TextStyle, "A-NOT".into(), Choice(A::Replace))));
}

pub(crate) fn install_scenes() -> Vec<Scene> {
    vec![
        ("standart-kaynak-kur", install),
        ("standart-kaynak-kuruldu", |app| {
            install(app);
            let _ = app.update(Message::StandardInstall(crate::standard_install::Event::Install));
        }),
    ]
}

/// docs/adr/1004: the CAD-ADV-20 acceptance detail (fixtures/acceptance/v1/detail.json) run through the desktop's
/// commands up to the 10 m width, with Parametreler open beside the plate and its associative dimension.
fn detail(app: &mut App) {
    let scenario: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/acceptance/v1/detail.json")).expect("the scenario");
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&scenario["setup"].to_string()).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::files_testing::make_cad(app);
    let mut names = std::collections::BTreeMap::new();
    for (i, n) in scenario["names"].as_array().expect("names").iter().enumerate() {
        names.insert(n.as_str().unwrap().to_owned(), uid(app, i as u32 + 1));
    }
    fn bind(v: &serde_json::Value, names: &std::collections::BTreeMap<String, String>) -> serde_json::Value {
        match v {
            serde_json::Value::String(s) if s.starts_with('$') && names.contains_key(&s[1..]) => names[&s[1..]].clone().into(),
            serde_json::Value::Array(a) => a.iter().map(|x| bind(x, names)).collect(),
            serde_json::Value::Object(m) => m.iter().map(|(k, x)| (k.clone(), bind(x, names))).collect(),
            other => other.clone(),
        }
    }
    for step in scenario["steps"].as_array().expect("steps").iter().take(3) {
        let doc = app.document.as_mut().expect("open");
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            step["command"].as_str().expect("a command"),
            kentos_commands::CallOp::Execute,
            bind(&step["input"], &names),
        )
        .expect("runs");
        assert_eq!(answer["status"], "completed", "{answer}");
    }
    app.viewport.camera.fit(
        &kentos_render_wgpu::Bounds { min_x: -1.5, min_y: -2.0, max_x: 11.5, max_y: 5.0 },
        24.0,
    );
    let _ = app.update(Message::Run("constraint.parameters"));
}

pub(crate) fn detail_scenes() -> Vec<Scene> {
    vec![("kabul-detay-parametre", detail)]
}

/// docs/adr/1005: the health cases' drawing with CAD denetimi's Çizim sağlığı on: the short and over-running T
/// junctions, the 0.38° edge, the far point and the two block definitions of one content; the first finding chosen.
fn health(app: &mut App) {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../fixtures/commands/v1/cad.drawing.repair.json")).expect("the cases");
    let snapshot =
        kentos_commands::DocumentSnapshotV1::from_json(&cases["setup"].to_string()).expect("the drawing reads");
    let doc = Document::new(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::files_testing::make_cad(app);
    app.viewport.camera.fit(&kentos_render_wgpu::Bounds { min_x: -1.0, min_y: -1.0, max_x: 23.0, max_y: 6.0 }, 24.0);
    use crate::cad_inspection::Event as C;
    let task = app.update(Message::Run("cad.inspect"));
    crate::files_testing::drive(app, task);
    for e in [C::Boundaries(false), C::Hatches(false), C::Annotations(false), C::Health(true)] {
        let _ = app.update(Message::CadInspection(e));
    }
    let task = app.update(Message::CadInspection(C::Run));
    crate::files_testing::drive(app, task);
    let _ = app.update(Message::CadInspection(C::Show(0)));
}

pub(crate) fn health_scenes() -> Vec<Scene> {
    vec![
        ("saglik-bulgular", health),
        ("saglik-onarildi", |app| {
            health(app);
            let task = app.update(Message::CadInspection(crate::cad_inspection::Event::Repair));
            crate::files_testing::drive(app, task);
        }),
    ]
}
