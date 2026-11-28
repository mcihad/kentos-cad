//! Son teslim edilen model in Denetim (docs/adr/8116; Dijital Proje Yön. Md. 15, plan/bim-plan.md K4): the declared
//! building's BIM package written as Paftaları toplu yayınla writes one (the model, the form's record, the list naming
//! both with their SHA-256, the drawing under kaynaklar when it carries it) and recorded with Teslim kaydı's command.
//! Denetim names it over the table; asked, it reads the EK-9 form from the delivered file; a wall moved after the
//! delivery is counted; a folder changed after the delivery is said, and the form goes back to the drawing.

use crate::arch::ArchState;
use crate::arch_ext::*;
use std::path::{Path, PathBuf};

use iced::Size;
use kentos_commands::Entity;
use kentos_commands::encoding::sha256_hex;
use kentos_native_application::delivered_model::Drift;
use kentos_ui::snapshot::Snapshot;
use serde_json::json;

use crate::app::{App, Message};
use crate::arch_audit::Event;
use crate::bim_permit_tests::declared_building;
use crate::use_class_tests::{checked, run};

fn send(app: &mut App, e: Event) {
    let _ = app.update(Message::from(crate::arch::Event::ArchAudit(e)));
}

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "kentos-teslim-modeli-{}-{name}",
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("kaynaklar")).expect("a folder");
    dir
}

/// The drawing's BIM package in `dir` as it is now — the model the export writes, the form's record with the author's
/// answers, the list naming both and, when `carried`, the drawing under kaynaklar — recorded as a delivery. The model's
/// file name.
fn deliver(app: &mut App, dir: &Path, carried: bool) -> String {
    let doc = &app.document.as_ref().expect("a drawing").model;
    let planned = kentos_native_application::ifc_facts::plan(
        doc,
        None,
        "2026-11-18T00:00:00",
        "KentOS Mimari 0.1.0",
    )
    .expect("a building");
    let name = planned.file_name();
    let model = planned.text().into_bytes();
    std::fs::write(dir.join(&name), &model).expect("writes");
    let record = serde_json::to_vec_pretty(&json!({
        "format": "kentos.ek9Form", "version": 1, "table": "9.1",
        "model": {"name": name, "bytes": model.len(), "sha256": sha256_hex(&model)},
        "answers": [{"no": 1, "yes": true}, {"no": 7, "yes": true}, {"no": 10, "yes": true}]
    }))
    .expect("json");
    std::fs::write(dir.join("kalite-kontrol-formu.json"), &record).expect("writes");
    let revision = doc.revision().to_string();
    let mut list = json!({
        "format": "kentos.publish/1",
        "source": {"revision": revision, "sha256": "0".repeat(64)},
        "files": [{"name": name, "sheets": [], "bytes": model.len(), "sha256": sha256_hex(&model)}],
        "bim": {"model": name, "form": {"name": "kalite-kontrol-formu.json", "bytes": record.len(), "sha256": sha256_hex(&record)}}
    });
    if carried {
        let kcad = kentos_kcad::encode(&doc.to_snapshot_v2()).expect("encodes");
        std::fs::write(dir.join("kaynaklar/cizim.kcad"), &kcad).expect("writes");
        list["resources"] = json!({"project": "kaynaklar/cizim.kcad", "bytes": kcad.len(), "files": [
            {"path": "kaynaklar/cizim.kcad", "kind": "drawing", "bytes": kcad.len(), "sha256": sha256_hex(&kcad)}
        ]});
    }
    let bytes = serde_json::to_vec_pretty(&list).expect("json");
    std::fs::write(dir.join("yayin.json"), &bytes).expect("writes");
    let path = dir.display().to_string();
    run(
        app,
        "cad.delivery.edit",
        json!({"change": {"kind": "add", "revision": "000",
        "package": {"manifest": sha256_hex(&bytes), "files": 2, "path": path, "source": revision},
        "to": "Örnek Belediyesi", "at": "2026-11-18"}}),
    );
    name
}

/// The Dijital proje package's findings now: their codes, sorted.
fn digital(app: &App) -> Vec<String> {
    let mut codes: Vec<String> = app
        .modules
        .arch()
        .arch_audit
        .standards
        .iter()
        .map(|f| f.code.clone())
        .filter(|c| c.starts_with("digital_"))
        .collect();
    codes.sort();
    codes
}

/// A wall of the drawing, by its persistent id.
fn a_wall(app: &App) -> String {
    let doc = &app.document.as_ref().expect("a drawing").model;
    doc.entities_with_uids()
        .find(|(_, e)| matches!(e, Entity::Wall(_)))
        .map(|(u, _)| u.to_string())
        .expect("a wall")
}

/// The line over the table.
fn line(app: &App) -> crate::arch_audit::DeliveredLine {
    app.modules
        .arch()
        .arch_audit
        .delivered
        .clone()
        .expect("the delivered model's line")
}

#[test]
fn denetim_reads_the_form_from_the_delivered_model_and_counts_what_changed_since() {
    let mut app = declared_building();
    let dir = folder("tasiyan");
    let name = deliver(&mut app, &dir, true);
    // On the drawing: the line names the delivery; nothing changed since.
    checked(&mut app);
    let l = line(&app);
    assert_eq!(
        (l.error.as_deref(), l.used, l.drift),
        (None, false, Some(Drift::default())),
        "{}",
        l.words()
    );
    assert_eq!(
        l.words(),
        format!(
            "EK-9 formu çizimin şimdiki hâlinden. 1. teslimin modeli: {name}, 18.11.2026. Çizimin modeli teslim edilenle aynı."
        )
    );
    assert_eq!(l.switch(false), Some(("Teslim edileni denetle", true)));
    let on_drawing = digital(&app);
    // Asked, the form reads the delivered file: the same model, the same findings; the source's objects from the
    // drawing the package carries.
    send(&mut app, Event::DeliveredModel(true));
    app.arch_audit_standards_settle();
    let l = line(&app);
    assert!(
        l.used && l.expectations && l.error.is_none(),
        "{}",
        l.words()
    );
    assert_eq!(
        l.words(),
        format!(
            "EK-9 formu 1. teslimin modelinden ({name}, 18.11.2026). Çizimin modeli teslim edilenle aynı."
        )
    );
    assert_eq!(l.switch(true), Some(("Şimdiki çizimi denetle", false)));
    assert_eq!(
        digital(&app),
        on_drawing,
        "the delivered model is the drawing's"
    );
    // A wall moved half a metre after the delivery: counted; the form still reads the file as it went.
    let wall = a_wall(&app);
    run(
        &mut app,
        "cad.entities.transform",
        json!({"uids": [wall], "transform": {"kind": "move", "dx": 0.0, "dy": 0.5}}),
    );
    checked(&mut app);
    let l = line(&app);
    let drift = l.drift.expect("compared");
    assert!(l.used && drift.changed >= 1, "{drift:?}: {}", l.words());
    assert!(
        l.words().contains("Teslimden bu yana çizimde ") && l.words().contains(" nesne değişti"),
        "{}",
        l.words()
    );
    // Back to the drawing.
    send(&mut app, Event::DeliveredModel(false));
    app.arch_audit_standards_settle();
    assert!(!line(&app).used);
    let _ = std::fs::remove_dir_all(&dir);
}

/// EK-9/18 is the delivered file's own: the schema and the extension of the file that went, not a reminder to check it.
#[test]
fn the_delivered_file_answers_the_version_question_the_memory_file_leaves_to_the_author() {
    let mut app = declared_building();
    let dir = folder("surum");
    deliver(&mut app, &dir, true);
    let doc = &app.document.as_ref().expect("a drawing").model;
    let row18 = |facts: serde_json::Value| -> String {
        let rows = crate::standards::call(
            "dijital-proje",
            "dijital_proje.form",
            "tablo",
            vec![facts, json!([])],
        )
        .expect("the form answers");
        let rows: Vec<Vec<String>> = serde_json::from_value(rows).expect("rows");
        rows.iter()
            .find(|r| r.first().map(String::as_str) == Some("18"))
            .map(|r| r.join(" | "))
            .expect("row 18")
    };
    let memory = kentos_native_application::ifc_facts::plan(doc, None, "", "KentOS Mimari")
        .expect("a building")
        .facts()
        .expect("facts");
    let kentos_native_application::delivered_model::Last::Found(d) =
        kentos_native_application::delivered_model::last(doc)
    else {
        panic!("the delivered model");
    };
    let delivered = d.read(doc).expect("reads").facts;
    assert!(
        row18(memory).contains("Teslim edilen dosyayı denetleyin"),
        "the memory file leaves it to the author"
    );
    let answered = row18(delivered);
    assert!(answered.contains("IFC4X3_ADD2, .ifc."), "{answered}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_package_without_its_drawing_after_a_change_leaves_the_source_questions_and_a_changed_folder_is_said()
 {
    let mut app = declared_building();
    let dir = folder("tasimayan");
    deliver(&mut app, &dir, false);
    // Unchanged: the drawing now gives the source's objects.
    send(&mut app, Event::DeliveredModel(true));
    checked(&mut app);
    assert!(line(&app).expectations, "{}", line(&app).words());
    // Changed and no carried drawing: the three questions are left to review, and the line says why.
    let wall = a_wall(&app);
    run(
        &mut app,
        "cad.entities.transform",
        json!({"uids": [wall], "transform": {"kind": "move", "dx": 0.5, "dy": 0.0}}),
    );
    checked(&mut app);
    let l = line(&app);
    assert!(l.used && !l.expectations, "{}", l.words());
    assert!(
        l.words().ends_with(
            "Paket kaynak çizimini taşımadığından 8., 17. ve 21. sorular incelemeye kaldı."
        ),
        "{}",
        l.words()
    );
    // The list rewritten after the delivery: said; the form reads the drawing; the button takes it back to it.
    std::fs::write(
        dir.join("yayin.json"),
        b"{\"format\": \"kentos.publish/1\", \"files\": []}",
    )
    .expect("writes");
    checked(&mut app);
    let l = line(&app);
    assert!(
        !l.used
            && l.error
                .as_deref()
                .is_some_and(|w| w.contains("teslimden sonra değişmiş")),
        "{}",
        l.words()
    );
    assert!(
        l.words().starts_with("1. teslimin modeli okunamadı: ")
            && l.words().ends_with("EK-9 formu çizimin şimdiki hâlinden.")
    );
    assert_eq!(l.switch(true), Some(("Şimdiki çizimi denetle", false)));
    assert_eq!(l.switch(false), None);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Pictures for the owner: Denetim with the line on the drawing and on the delivered model after a change (light and
/// dark), in `.run/gui-evidence/teslim-modeli-8116/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    let out = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../.run/gui-evidence/teslim-modeli-8116");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    crate::drawing_fonts::load();
    kentos_ui::theme::motion::set_reduced(true);
    for (theme, suffix) in [("light", "acik"), ("dark", "koyu")] {
        let size = Size::new(1440.0, 900.0);
        fn update(app: &mut App, message: Message) {
            let _ = app.update(message);
        }
        let mut app = declared_building();
        let dir = folder(&format!("resim-{suffix}"));
        deliver(&mut app, &dir, false);
        let _ = app
            .settings
            .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        for (step, delivered, moved) in [
            ("1-cizimden", false, false),
            ("2-teslim-edilenden", true, true),
        ] {
            if moved {
                let wall = a_wall(&app);
                run(
                    &mut app,
                    "cad.entities.transform",
                    json!({"uids": [wall], "transform": {"kind": "move", "dx": 0.0, "dy": 0.5}}),
                );
            }
            send(&mut app, Event::DeliveredModel(delivered));
            checked(&mut app);
            send(
                &mut app,
                Event::Group(Some(kentos_commands::ArchAuditCheck::Regulation)),
            );
            let mut snapshot = Snapshot::new(size).expect("a renderer");
            app.toasts.clear();
            snapshot.settle(&mut app, App::view, &mut update);
            let file = out.join(format!("{step}-{suffix}.png"));
            snapshot
                .render(app.view(), &app.theme())
                .save(&file)
                .expect("writes the picture");
            println!("{}", file.display());
        }
        let _ = std::fs::remove_dir_all(&dir);
    }
}
