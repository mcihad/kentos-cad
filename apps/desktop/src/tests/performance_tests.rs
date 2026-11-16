//! Yangın ve performans (docs/adr/8068; plan/bim-plan.md BIM-07): on the section building (fixtures/arch/v1/
//! section-building.kcad) the model's mandatory fields are the Mimari audit's findings with their objects; a wall
//! style's fire resistance class reaches its walls, taken away they are in the finding again; the IFC file writes what is
//! declared and never an empty word; EK-9's 9th question is read from the model.

use kentos_commands::Entity;

use crate::app::{App, Message};

fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/section-building.kcad"))
        .expect("decodes");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn run(app: &mut App, command: &str, input: serde_json::Value) -> serde_json::Value {
    let doc = &mut app.document.as_mut().expect("the building").model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        command,
        kentos_commands::CallOp::Execute,
        input.clone(),
    )
    .unwrap_or_else(|e| panic!("{command} {input}: {e:?}"));
    assert_eq!(answer["status"], "completed", "{command} {input}: {answer}");
    answer
}

/// The audit's mandatory-field findings, by the path each names.
fn required(app: &mut App) -> Vec<(String, Vec<String>)> {
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    app.arch
        .arch_audit
        .audited
        .as_ref()
        .expect("checked")
        .findings
        .iter()
        .filter(|f| f.code == "digital_required_field")
        .map(|f| (f.message.clone(), f.uids.clone()))
        .collect()
}

fn walls(app: &App, style: Option<&str>) -> Vec<String> {
    let doc = &app.document.as_ref().expect("the building").model;
    let mut out: Vec<String> = doc
        .entities_with_uids()
        .filter(|(_, e)| matches!(e, Entity::Wall(w) if style.is_none_or(|s| w.wall.style.as_deref() == Some(s))))
        .map(|(u, _)| u.to_string())
        .collect();
    out.sort();
    out
}

fn ifc(app: &mut App) -> String {
    let dir = std::env::temp_dir().join(format!("kentos-yangin-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("a folder");
    let path = dir.join("bina.ifc");
    run(app, "cad.arch.ifc", serde_json::json!({"path": path.to_string_lossy(), "overwrite": true}));
    let text = std::fs::read_to_string(&path).expect("the file");
    let _ = std::fs::remove_dir_all(&dir);
    text
}

#[test]
fn a_wall_style_s_class_reaches_its_walls_and_taken_away_they_are_in_the_finding() {
    let mut app = app();
    // As drawn: no wall declares a class, every wall is in the finding.
    let before = required(&mut app);
    let fire = |found: &[(String, Vec<String>)]| {
        found.iter().find(|(m, _)| m.contains("Pset_WallCommon.FireRating")).map(|(_, u)| {
            let mut u = u.clone();
            u.sort();
            u
        })
    };
    assert_eq!(fire(&before), Some(walls(&app, None)), "{before:?}");
    // Every style used by a wall given REI 120: no wall misses its class, the file carries it.
    let doc = &app.document.as_ref().expect("the building").model;
    let mut styles: Vec<String> = doc
        .entities()
        .filter_map(|e| match e {
            Entity::Wall(w) => w.wall.style.clone(),
            _ => None,
        })
        .collect();
    styles.sort();
    styles.dedup();
    assert!(!styles.is_empty(), "the walls have styles");
    for s in &styles {
        run(
            &mut app,
            "cad.arch.performance",
            serde_json::json!({"operation": {"kind": "wallStyle", "style": s, "fireRating": {"value": "REI 120", "basis": {"kind": "author"}}}}),
        );
    }
    assert_eq!(fire(&required(&mut app)), None);
    let text = ifc(&mut app);
    assert!(text.contains("'FireRating'") && text.contains("IFCLABEL('REI 120')"), "the class written");
    // One style's class taken away: its walls are in the finding again, nothing empty is written.
    let first = &styles[0];
    run(
        &mut app,
        "cad.arch.performance",
        serde_json::json!({"operation": {"kind": "wallStyle", "style": first, "fireRating": null}}),
    );
    assert_eq!(fire(&required(&mut app)), Some(walls(&app, Some(first))));
    let text = ifc(&mut app);
    assert!(!text.contains("IFCLABEL('')"), "no empty word in the file");
    // A door's own class and exit role, from a test report.
    let door = {
        let doc = &app.document.as_ref().expect("the building").model;
        doc.entities_with_uids()
            .find_map(|(u, e)| matches!(e, Entity::Opening(o) if o.opening.form == kentos_commands::OpeningForm::Door).then(|| u.to_string()))
            .expect("a door")
    };
    run(
        &mut app,
        "cad.arch.performance",
        serde_json::json!({"operation": {"kind": "objects", "uids": [door],
            "fireRating": {"value": "EI 60", "basis": {"kind": "document", "name": "Test raporu 2026/41", "date": "2026-11-16"}},
            "exit": {"exit": true, "basis": {"kind": "author"}}}}),
    );
    let text = ifc(&mut app);
    assert!(text.contains("IFCLABEL('EI 60')") && text.contains("'FireExit'"), "the door's class and exit role");
}

#[test]
fn ek9_s_9th_question_is_read_from_the_model() {
    let app = app();
    let row9 = |app: &App| {
        let rows = kentos_native_application::arch_ifc::form_rows(&app.document.as_ref().expect("the building").model);
        rows.into_iter().find(|r| r.first().is_some_and(|c| c == "9")).expect("row 9").join(" | ")
    };
    let before = row9(&app);
    assert!(before.contains("Hayır") && before.contains("zorunlu alan eksik"), "{before}");
    assert!(before.contains("IfcBeam Yangın Dayanım Sınıfı (8/8)"), "{before}");
}
