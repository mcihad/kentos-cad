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

/// docs/adr/1100 §6: the library cases' drawing (two office standards naming blocks of `kentos-ofis` and sheet
/// templates, a KAPI of the door's first release placed three times, sheets made from an older template revision)
/// with Standart kaynaklarını kur open: KAPI to be replaced by the second release, the mimari template's approved
/// revision to be replaced, the references no library holds shown as warnings.
fn install_library(app: &mut App) {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../fixtures/commands/v1/cad.standards.install.library.json"
    ))
    .expect("the cases");
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(cases["setup"].clone()).expect("the drawing reads");
    let doc = Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::files_testing::make_cad(app);
    let _ = app.update(Message::StandardInstall(crate::standard_install::Event::Open));
    use crate::standard_install::{Choice, Event as I};
    use kentos_commands::{StandardInstallAction as A, StandardResourceKind as K};
    let _ = app.update(Message::StandardInstall(I::Pick(K::Block, "KAPI".into(), Choice(A::Replace))));
    let _ = app.update(Message::StandardInstall(I::Pick(K::SheetTemplate, "sys:cad-mimari".into(), Choice(A::Replace))));
}

pub(crate) fn install_scenes() -> Vec<Scene> {
    vec![
        ("standart-kaynak-kur", install),
        ("standart-kaynak-kuruldu", |app| {
            install(app);
            let _ = app.update(Message::StandardInstall(crate::standard_install::Event::Install));
        }),
        ("standart-kitaplik-kur", install_library),
        ("standart-kitaplik-kuruldu", |app| {
            install_library(app);
            let _ = app.update(Message::StandardInstall(crate::standard_install::Event::Install));
            let _ = app.update(Message::DialogClosed);
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
        // docs/adr/1012: the 0.38° edge chosen (Eksene döndür), then turned; the two definitions of one content merged.
        ("saglik-eksen", |app| choose(app, "angle_deviation")),
        ("saglik-eksen-onarildi", |app| repaired(app, "angle_deviation")),
        ("saglik-birlestir", |app| choose(app, "block_definition_duplicate")),
        ("saglik-birlestirildi", |app| repaired(app, "block_definition_duplicate")),
    ]
}

/// The health scene with the first finding of `code` chosen.
fn choose(app: &mut App, code: &str) {
    health(app);
    let at = app
        .cad_inspection
        .result()
        .and_then(|r| r.findings.iter().position(|f| f.code == code))
        .expect("the finding");
    let _ = app.update(Message::CadInspection(crate::cad_inspection::Event::Show(at)));
}

/// [`choose`], then Onar on it: the check runs again and says the finding is gone.
fn repaired(app: &mut App, code: &str) {
    choose(app, code);
    let task = app.update(Message::CadInspection(crate::cad_inspection::Event::Repair));
    crate::files_testing::drive(app, task);
    assert!(
        app.cad_inspection.result().is_some_and(|r| !r.findings.iter().any(|f| f.code == code)),
        "{code} is repaired"
    );
}

/// docs/adr/1100 §5: a wall layer the bound office standard defines, a line with its own colour beside a plain one,
/// two sheet viewports drawing the layer otherwise (one magenta, one 0.7 mm): Öznitelikler's “Bu değer nereden
/// geliyor?” for the selected line.
fn value_origin(app: &mut App, slot: u32) {
    let package = serde_json::json!({"format": "kentos.drawing-standard", "version": 2, "id": "ofis-b",
        "revision": "2026.2", "standard": {"name": "Ofis B", "layers": [{"name": "A-DUVAR", "color": "#0000ff",
            "lineType": "continuous", "lineWeight": 0.5}]}, "resources": {}});
    let style = serde_json::json!({"color": "#0000ff", "lineType": "continuous", "lineWeight": 0.5});
    let sheet = |id: &str, over: serde_json::Value| serde_json::json!({"id": id, "name": id.to_uppercase(),
        "page": {"paper": "a3", "orientation": "landscape", "size": {"width": 420000, "height": 297000},
                 "margins": {"top": 10000, "right": 10000, "bottom": 10000, "left": 20000}},
        "items": [{"id": format!("{id}-plan"), "name": "Plan",
                   "frame": {"left": 30000, "top": 20000, "width": 300000, "height": 200000},
                   "kind": {"type": "map", "view": {"type": "fixed", "scale": 50}, "layerOverrides": [over]}}]});
    let snapshot = serde_json::json!({
        "format": "kentos.document", "version": 2, "name": "Değerin kökeni",
        "settings": {"srid": 0, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 50.0, "workspace": "cad", "standardDocs": [{"text": package.to_string()}]},
        "origin": {"x": 0.0, "y": 0.0},
        "layers": [{"id": "duvar", "name": "A-DUVAR", "type": "layer", "visible": true, "locked": false,
                    "expanded": true, "style": style, "children": []}],
        "activeLayer": "duvar",
        "entities": [
            {"kind": "line", "id": 1, "layerId": "duvar", "attrs": {}, "color": "#00ff00",
             "a": {"x": 0.0, "y": 0.0}, "b": {"x": 8.0, "y": 0.0}},
            {"kind": "line", "id": 2, "layerId": "duvar", "attrs": {}, "a": {"x": 0.0, "y": 3.0}, "b": {"x": 8.0, "y": 3.0}},
            {"kind": "line", "id": 3, "layerId": "duvar", "attrs": {}, "color": "#00ff00",
             "a": {"x": 0.0, "y": 6.0}, "b": {"x": 8.0, "y": 6.0}}],
        "uids": ["0192f5a1-4444-7000-8000-000000000001", "0192f5a1-4444-7000-8000-000000000002",
                 "0192f5a1-4444-7000-8000-000000000003"],
        "styles": {"items": [], "categories": []},
        "sheets": {"book": {"schema": "kentos.sheet/1", "sheets": [
            sheet("a-101", serde_json::json!({"layer": "duvar", "color": "#ff00ff"})),
            sheet("a-102", serde_json::json!({"layer": "duvar", "lineWeight": 700}))], "masters": [], "assets": []},
            "pictures": []}
    });
    let doc = Document::from_v2(serde_json::from_value(snapshot).expect("the drawing"), None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    crate::files_testing::make_cad(app);
    crate::elevation_scenes::select(app, &[kentos_domain::Slot(slot)]);
    crate::elevation_scenes::panel_room(app);
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle("geometry")));
    let _ = app.update(Message::Properties(crate::properties::Event::Toggle("modelAppearance")));
}

pub(crate) fn origin_scenes() -> Vec<Scene> {
    vec![
        ("deger-kokeni-katman", |app| value_origin(app, 2)),
        // docs/adr/1015: siyah.ctb's pen 1 given to the layer and applied to sheet a-101: Baskıda says black, 0.50 mm.
        ("deger-kokeni-baski", |app| {
            value_origin(app, 2);
            let base = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/commands/v1/plotstyles");
            let doc = app.document.as_mut().expect("a drawing");
            let answer = kentos_native_application::registry::run(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                "cad.plotstyles.import",
                kentos_commands::CallOp::Execute,
                serde_json::json!({"path": "siyah.ctb", "base": base.display().to_string(),
                    "assignments": [{"target": {"type": "modelLayer", "id": "duvar"}, "key": {"type": "aci", "index": 1}}],
                    "sheets": ["a-101"]}),
            )
            .expect("runs");
            assert_eq!(answer["status"], "completed", "{answer}");
            crate::elevation_scenes::select(app, &[kentos_domain::Slot(2)]);
        }),
        ("deger-kokeni-nesne", |app| value_origin(app, 1)),
    ]
}
