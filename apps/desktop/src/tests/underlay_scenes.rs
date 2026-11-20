//! The pictures of docs/adr/0351: Kırp on the architect's plan (`fixtures/xref/v1/plan.kcad`) with its frame on the
//! screen and inverted with a plotted frame, a PDF page attached as an underlay (`fixtures/pdf/v1/katmanlar.pdf`) with
//! Dış başvuru ekle's Sayfa, Dış başvurular with the underlay's Soldur, Tek renk and Yazdır, and two drawings open in
//! their tabs. `tools_screens` takes them in both themes at 1440×900 and 1100×650:
//!
//! ```text
//! KENTOS_SHOTS_ONLY=kirp,kirp-ters,pdf-ekle,pdf-altlik,pdf-altlik-pencere,belge-sekmeleri cargo test -p kentos-desktop tools_screens -- --ignored --nocapture
//! ```
//!
//! Test code only.

use std::path::PathBuf;

use kentos_commands::CallOp;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::json;

use crate::app::{App, Message};
use crate::tools_screens::Scene;
use crate::wall_scenes::opened;
use crate::xrefs_window::{Event, Purpose};

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../fixtures").join(rel)
}

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::Xrefs(e));
}

fn on_inserts(app: &mut App) {
    let inserts: Vec<f64> = app
        .document
        .as_ref()
        .map(|d| {
            d.model
                .entities()
                .filter(|e| matches!(e, kentos_commands::Entity::Insert(_)))
                .map(|e| f64::from(e.base().id))
                .collect()
        })
        .unwrap_or_default();
    app.zoom_to_objects(&inserts);
}

/// The plan attached in place; its insert's persistent id.
pub(crate) fn plan(app: &mut App) -> String {
    opened(app, crate::tools_scenes::Objects::new(), 50.0);
    app.tab = "insert";
    send(app, Event::Picked(Purpose::Attach, Some(fixture("xref/v1/plan.kcad"))));
    send(app, Event::Name("Plan".to_owned()));
    send(app, Event::InPlace);
    let doc = app.document.as_ref().expect("a drawing");
    let slot = doc
        .model
        .entities()
        .find(|e| matches!(e, kentos_commands::Entity::Insert(_)))
        .map(|e| e.base().id)
        .expect("the insert");
    doc.model.uid(kentos_domain::Slot(slot)).map(|u| u.to_string()).unwrap_or_default()
}

/// Kırp through its command, as the tool writes.
pub(crate) fn clip(app: &mut App, uid: &str, operation: serde_json::Value) {
    if let Some(doc) = app.document.as_mut() {
        let answer = registry::run(
            &mut ExecutionContext::new(&mut doc.model),
            "cad.inserts.clip",
            CallOp::Execute,
            json!({ "uids": [uid], "operation": operation }),
        );
        assert!(answer.as_ref().is_ok_and(|a| a["status"] == "completed"), "{answer:?}");
    }
    app.spatial.reload(&app.document.as_ref().expect("a drawing").model);
}

/// The plan's living room kept, its frame on the screen; Kırp running on the next insert.
fn clipped(app: &mut App) {
    let uid = plan(app);
    clip(
        app,
        &uid,
        json!({ "kind": "set", "boundary": { "kind": "polygon", "points": [
            { "x": -0.5, "y": -0.5 }, { "x": 6.5, "y": -0.5 }, { "x": 6.5, "y": 3.0 }, { "x": 3.0, "y": 5.5 },
            { "x": -0.5, "y": 5.5 }] }, "frame": "show" }),
    );
    on_inserts(app);
    let _ = app.run("tool.xclip");
}

/// The same plan, its middle left out (Ters), the frame plotted.
fn inverted(app: &mut App) {
    let uid = plan(app);
    clip(
        app,
        &uid,
        json!({ "kind": "set", "boundary": { "kind": "rectangle", "a": { "x": 2.0, "y": 1.0 }, "b": { "x": 7.0, "y": 4.0 } },
                "inverted": true, "frame": "plot" }),
    );
    on_inserts(app);
}

/// Dış başvuru ekle with a PDF chosen: its Sayfa.
fn pdf_window(app: &mut App) {
    opened(app, crate::tools_scenes::Objects::new(), 1.0);
    app.tab = "insert";
    send(app, Event::Picked(Purpose::Attach, Some(fixture("pdf/v1/katmanlar.pdf"))));
}

/// The PDF's page attached in place at 1:1 on paper, faded and in one colour.
fn pdf_attached(app: &mut App) {
    pdf_window(app);
    send(app, Event::InPlace);
    let _ = app.run("xref.manager");
    send(app, Event::Choose(0));
    send(app, Event::Fade(40.0));
    send(app, Event::Monochrome(true));
    send(app, Event::Close);
    on_inserts(app);
}

fn pdf_manager(app: &mut App) {
    pdf_attached(app);
    let _ = app.run("xref.manager");
    send(app, Event::Choose(0));
}

/// PDF içe aktar (docs/adr/0352) on the curves page: 1:50, NURBS, the plan shown.
fn pdf_import_window(app: &mut App) {
    opened(app, crate::tools_scenes::Objects::new(), 50.0);
    app.tab = "insert";
    let _ = app.update(Message::Exchange(Box::new(crate::exchange::Event::PdfImport(
        crate::exchange::pdf_import::Event::Picked(Some(fixture("pdf/v1/egriler.pdf"))),
    ))));
    let _ = app.update(Message::Exchange(Box::new(crate::exchange::Event::PdfImport(
        crate::exchange::pdf_import::Event::Curves(kentos_commands::PdfCurveOutput::Nurbs),
    ))));
}

/// The curves page taken in at 1:50.
fn pdf_imported(app: &mut App) {
    pdf_import_window(app);
    let _ = app.update(Message::Exchange(Box::new(crate::exchange::Event::PdfImport(
        crate::exchange::pdf_import::Event::Run,
    ))));
}

/// Two drawings open: the office plan set aside in its tab, the site plan in front.
fn tabs(app: &mut App) {
    plan(app);
    if let Some(doc) = app.document.as_mut() {
        doc.model.set_name("Ofis planı");
    }
    on_inserts(app);
    assert!(app.park_active());
    opened(app, crate::tools_scenes::Objects::new(), 500.0);
    if let Some(doc) = app.document.as_mut() {
        doc.model.set_name("Vaziyet planı");
    }
}

pub(crate) fn scenes() -> Vec<Scene> {
    vec![
        ("kirp", clipped),
        ("kirp-ters", inverted),
        ("pdf-ekle", pdf_window),
        ("pdf-altlik", pdf_attached),
        ("pdf-altlik-pencere", pdf_manager),
        ("belge-sekmeleri", tabs),
        ("pdf-ice-aktar", pdf_import_window),
        ("pdf-ice-aktar-cizim", pdf_imported),
    ]
}

/// UR2-044: nothing selected, Kırp started, and a click on a line of the attached plan: the plan's insert is the one
/// to clip, as a plain click would select it, not “no block or external reference here”.
#[test]
fn kirp_takes_the_reference_a_click_on_its_line_would_select() {
    let mut app = App::boot(None).0;
    let _ = plan(&mut app);
    on_inserts(&mut app);
    let insert = app
        .document
        .as_ref()
        .and_then(|d| d.model.entities().find(|e| matches!(e, kentos_commands::Entity::Insert(_))).map(|e| kentos_domain::Slot(e.base().id)))
        .expect("the insert");
    // A point a plain click selects the insert at: the first on a grid over the view.
    let b = app.viewport.camera.visible_bounds();
    let tol = (b.max_x - b.min_x) / 400.0;
    let at = (0..80)
        .flat_map(|i| (0..80).map(move |j| (i, j)))
        .map(|(i, j)| kentos_interaction::Vec2::new(b.min_x + (b.max_x - b.min_x) * f64::from(i) / 80.0, b.min_y + (b.max_y - b.min_y) * f64::from(j) / 80.0))
        .find(|p| app.spatial.pick(*p, tol) == Some(insert))
        .expect("a point on the plan's lines");
    app.selection.set([]);
    let _ = app.run("tool.xclip");
    crate::tools_scenes::click(&mut app, [at.x - crate::tools_scenes::E, at.y - crate::tools_scenes::N]);
    let said = crate::files_testing::last_said(&app);
    assert!(!said.contains("kilitsiz bir blok ya da dış başvuru yok"), "{said}");
    assert_eq!(app.session.point_count(), 1, "the insert taken: {said}");
}
