//! A run's drawing (PY-RUN-16; plan/eklenti.md PY-UI-17: "belge değişimi geç geri çağırmayı etkisiz kılar, bir taslak
//! başka belgeye uygulanmaz"; docs/adr/8057 §125). While an extension's command that may write runs, its drawing is
//! held in front as the console holds it: another drawing is not opened in a tab, the drawing is not closed. Were it set
//! aside anyway, its late writes go to no other drawing and its undo step still ends on its own drawing. Before, a
//! drawing set aside mid-run kept the run's undo step open, and the user's later edits there went into it.

use serde_json::json;

use super::*;
use crate::python::host::Said;

const MEBIS: &str = "tr.ornek.mebis";
const TITLE: &str = "MEBİS katmanlarını kur";

/// The app with a drawing and MEBİS installed, its command running; the runner's generation and the run's id.
fn running(root: &std::path::Path) -> (App, u64, u64) {
    let mut gis = crate::files_testing::app_with_drawing();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(root, false).0;
    gis.extensions_changed();
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] = json!([]);
        }),
    );
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(MEBIS))
        .unwrap();
    let launch = super::runner::Launch::for_test(
        "/bin/sh".into(),
        package,
        "mebis.logic:init_schema",
        TITLE,
        true,
    );
    let _ = gis.extensions_event(Event::Launch(
        MEBIS.into(),
        "command".into(),
        "init-schema".into(),
        Ok(launch),
    ));
    let (generation, run) = {
        let runner = &gis.extensions.runners[MEBIS];
        (runner.generation, runner.run.as_ref().expect("running").id)
    };
    (gis, generation, run)
}

fn add_layer(
    app: &mut App,
    name: &str,
) -> Result<serde_json::Value, kentos_headless::HeadlessError> {
    app.ext_answer_for_test(
        MEBIS,
        "run",
        &json!({"command": "cad.layers.edit", "op": "execute",
                "input": {"operation": "add", "name": name, "geometry": "area"}}),
    )
}

fn has_layer(doc: &crate::document::Document, name: &str) -> bool {
    doc.model.layers().nodes().iter().any(|n| n.name == name)
}

fn done(app: &mut App, generation: u64, run: u64) {
    let _ = app.extensions_event(Event::Runner(
        MEBIS.into(),
        generation,
        Said::Message(json!({"type": "done", "id": run, "ok": true,
            "result": {"kind": "message", "message": "Kuruldu."}})),
    ));
}

#[test]
fn a_writing_run_holds_its_drawing_in_front() {
    let root = temp("cizim-tutulur");
    let (mut gis, generation, run) = running(&root);
    let first = gis.document.as_ref().unwrap().session;
    add_layer(&mut gis, "mezar_ada").expect("written");
    // Yeni sekmede aç and Çizimi kapat wait for it, as for the console: said, the drawing in front stays.
    let _ = gis.update(Message::DocTabs(crate::doc_tabs::Event::New));
    let said = crate::files_testing::last_said(&gis);
    assert!(
        said.contains("çalışıyor; bitince ya da Durdur'dan sonra çizim değiştirilebilir"),
        "{said}"
    );
    let _ = gis.update(Message::DocTabs(crate::doc_tabs::Event::Close(0)));
    let doc = gis.document.as_ref().expect("the drawing stays");
    assert_eq!(doc.session, first);
    assert!(gis.doc_tabs.parked.is_empty(), "nothing set aside");
    // It ends: its one undo step on its drawing; the drawing can be set aside again.
    done(&mut gis, generation, run);
    let doc = gis.document.as_ref().unwrap();
    assert!(has_layer(doc, "mezar_ada") && !doc.model.in_group());
    assert_eq!(
        doc.model.undo_labels().first().map(String::as_str),
        Some(TITLE)
    );
    assert_eq!(gis.tabs_held(), None);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_run_whose_drawing_was_set_aside_ends_its_step_there_and_writes_nowhere_else() {
    let root = temp("cizim-sekmede");
    let (mut gis, generation, run) = running(&root);
    let first = gis.document.as_ref().unwrap().session;
    add_layer(&mut gis, "mezar_ada").expect("written");
    // Set aside by a way that does not ask (a run that says it only reads would not hold the tabs).
    gis.extensions
        .runners
        .get_mut(MEBIS)
        .and_then(|r| r.run.as_mut())
        .expect("running")
        .write = false;
    // Yeni sekmede aç's end: the drawing in its tab, another in front.
    assert!(gis.park_active(), "set aside");
    gis.show_document(crate::files_testing::drawing(0));
    let second = gis.document.as_ref().unwrap().session;
    assert_ne!(second, first, "another drawing in front");
    // Its late write goes to no drawing: refused, said.
    gis.extensions
        .runners
        .get_mut(MEBIS)
        .and_then(|r| r.run.as_mut())
        .expect("running")
        .write = true;
    let refused = add_layer(&mut gis, "mezar_yeri").expect_err("not this drawing");
    assert_eq!(refused.code, "no_document");
    assert!(
        refused
            .message
            .contains("Çalıştırıldığı çizim artık açık değil"),
        "{}",
        refused.message
    );
    // It ends: the step closes on its own drawing in its tab; the one in front has nothing of it.
    done(&mut gis, generation, run);
    let front = gis.document.as_ref().unwrap();
    assert!(!has_layer(front, "mezar_ada") && !has_layer(front, "mezar_yeri"));
    assert!(front.model.undo_labels().is_empty());
    let set_aside = gis.doc_tabs.find_mut(first).expect("in its tab");
    assert!(!set_aside.model.in_group(), "its step ended");
    assert!(has_layer(set_aside, "mezar_ada") && !has_layer(set_aside, "mezar_yeri"));
    assert_eq!(
        set_aside.model.undo_labels().first().map(String::as_str),
        Some(TITLE)
    );
    let _ = std::fs::remove_dir_all(&root);
}
