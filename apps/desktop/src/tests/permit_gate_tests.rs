//! Ruhsat yayınının kapısı (docs/adr/8065; plan/bim-plan.md BIM-04): on the section building (fixtures/arch/v1/
//! section-building.kcad, written without KentOS code), a permit set's Paftaları toplu yayınla keeps Yayınla shut while a
//! section is stale or a model layer holds free drawing, says each with its object, and opens once they are mended; the
//! Mimari audit lists the same and its Düzelt draws the section again.

use crate::app::{App, Message};
use crate::exchange::Window;
use crate::exchange::permit_set::Event as Permit;
use kentos_commands::Entity;
use crate::arch::ArchState;

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

fn mark(app: &App, letter: &str) -> String {
    let doc = &app.document.as_ref().expect("the building").model;
    doc.entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::ViewMark(v) if v.mark.mark == letter => Some(u.to_string()),
            _ => None,
        })
        .expect("the mark")
}

/// The entrance door GK01 10 cm higher: the section B-B cuts it (docs/adr/8042 §5's own case).
fn raise_the_door(app: &mut App) {
    let doc = &mut app.document.as_mut().expect("the building").model;
    let (slot, mut door) = doc
        .entities_with_uids()
        .find_map(|(u, e)| match e {
            Entity::Opening(o) if o.opening.number.as_deref() == Some("GK01") => Some((doc.slot_of(u).unwrap(), o.clone())),
            _ => None,
        })
        .expect("the entrance door");
    door.opening.head = Some(door.opening.head.unwrap_or(2.1) + 0.1);
    doc.update_many(vec![(slot, Entity::Opening(door))], "Kapı");
}

/// The gate's findings for the drawing as it is, asked now; none when it is open.
fn gate(app: &mut App) -> Vec<String> {
    app.permit_gate_settle();
    let generation = app.document.as_ref().expect("the building").model.generation();
    match &app.exchange {
        Some(Window::SheetPublish(s)) => {
            let g = s.permit.as_ref().expect("a permit publication");
            assert_eq!(g.failed, None);
            assert_eq!(g.open(generation), g.findings.is_empty());
            g.findings.iter().map(|f| f.message.clone()).collect()
        }
        _ => panic!("Paftaları toplu yayınla is open"),
    }
}

/// The building's wall layer, by its id.
fn wall_layer(app: &App) -> String {
    let doc = &app.document.as_ref().expect("the building").model;
    doc.entities().find_map(|e| matches!(e, Entity::Wall(_)).then(|| e.base().layer_id.clone())).expect("a wall")
}

#[test]
fn the_permit_publication_waits_for_a_stale_section_and_free_drawing_on_a_wall_layer() {
    let mut app = app();
    let b = mark(&app, "B");
    run(&mut app, "cad.sections.edit", serde_json::json!({"operation": {"kind": "derive", "mark": b, "at": {"x": 40.0, "y": 0.0}}}));
    let _ = app.update(Message::Run("arch.permitSet"));
    let _ = app.permit_set_event(Permit::Make);
    assert!(matches!(&app.exchange, Some(Window::SheetPublish(s)) if s.permit.is_some()), "the set's publication");
    assert_eq!(gate(&mut app), Vec::<String>::new(), "the section current, nothing free on the model's layers");
    // A door changed: the section is stale and the gate shut.
    raise_the_door(&mut app);
    let shut = gate(&mut app);
    assert!(shut.len() == 1 && shut[0].starts_with("B-B kesiti eski"), "{shut:?}");
    // Yeniden çiz opens it again.
    run(&mut app, "cad.sections.edit", serde_json::json!({"operation": {"kind": "refresh", "mark": b}}));
    assert_eq!(gate(&mut app), Vec::<String>::new());
    // A free line on the wall layer: said with its count; the facts name the line.
    let layer = wall_layer(&app);
    let made = run(&mut app, "cad.entities.create", serde_json::json!({"layerId": layer, "objects": [{"geometry": {"kind": "line", "a": {"x": 0.0, "y": 0.0}, "b": {"x": 3.0, "y": 0.0}}}]}));
    let shut = gate(&mut app);
    assert!(shut.len() == 1 && shut[0].contains("1 serbest çizim var"), "{shut:?}");
    let facts = app.arch_audit_standards_facts().expect("facts");
    let line = facts["cad"]["layers"].as_array().unwrap().iter().find(|l| l["id"] == layer.as_str()).unwrap()["firstFree"].clone();
    assert_eq!(line.as_array().map(Vec::len), Some(1), "{made}");
    run(&mut app, "cad.entities.delete", serde_json::json!({"uids": line}));
    assert_eq!(gate(&mut app), Vec::<String>::new());
}

#[test]
fn the_audit_lists_a_stale_section_and_its_fix_draws_it_again() {
    let mut app = app();
    let b = mark(&app, "B");
    run(&mut app, "cad.sections.edit", serde_json::json!({"operation": {"kind": "derive", "mark": b, "at": {"x": 40.0, "y": 0.0}}}));
    raise_the_door(&mut app);
    let _ = app.update(Message::Run("arch.audit"));
    app.arch_audit_standards_settle();
    let at = {
        let a = app.modules.arch().arch_audit.audited.as_ref().expect("checked");
        a.findings.iter().position(|f| f.code == "digital_derived_stale").expect("the stale section")
    };
    let f = &app.modules.arch().arch_audit.audited.as_ref().unwrap().findings[at];
    assert!(f.message.contains("— Dijital Proje Yön. 9"), "{}", f.message);
    assert!(f.rule.as_deref().is_some_and(|r| r.contains("EK-9 Tablo 9.1 satır 21 · PDF s. 239")));
    app.modules.arch_mut().arch_audit.selected = Some(at);
    let _ = app.update(Message::from(crate::arch::Event::ArchAudit(crate::arch_audit::Event::Fix)));
    app.arch_audit_standards_settle();
    let left = app.modules.arch().arch_audit.audited.as_ref().unwrap().findings.iter().any(|f| f.code == "digital_derived_stale");
    assert!(!left, "drawn again");
}

/// The publication's own check let finish, as the window's ticks do.
fn finish_the_check(app: &mut App) {
    for _ in 0..600 {
        if !app.sheet_publish_running() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
        let _ = app.sheet_publish_event(crate::exchange::sheet_publish::Event::Tick);
    }
    panic!("the check did not finish");
}

/// Paftaları toplu yayınla with the permit gate shut (a stale section, a free line on the wall layer) and open again,
/// light and dark; `.run/gui-evidence/dijital-turetim-8065/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib permit_gate_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;
    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/dijital-turetim-8065");
    std::fs::create_dir_all(&out).expect("a folder");
    for (theme, size, name) in [("light", Size::new(1600.0, 1000.0), "acik"), ("dark", Size::new(1440.0, 900.0), "koyu")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let b = mark(&app, "B");
        run(&mut app, "cad.sections.edit", serde_json::json!({"operation": {"kind": "derive", "mark": b, "at": {"x": 40.0, "y": 0.0}}}));
        let _ = app.update(Message::Run("view.zoomExtents"));
        let _ = app.update(Message::Run("arch.permitSet"));
        let _ = app.permit_set_event(Permit::Make);
        finish_the_check(&mut app);
        raise_the_door(&mut app);
        let layer = wall_layer(&app);
        run(&mut app, "cad.entities.create", serde_json::json!({"layerId": layer, "objects": [{"geometry": {"kind": "line", "a": {"x": 0.0, "y": -2.0}, "b": {"x": 6.0, "y": -2.0}}}]}));
        let _ = app.sheet_publish_event(crate::exchange::sheet_publish::Event::Plan);
        finish_the_check(&mut app);
        app.permit_gate_settle();
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        let mut shot = |snapshot: &mut Snapshot, app: &mut App, file: String| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(file);
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes");
            println!("{}", file.display());
        };
        // The window's end: the gate under the check's report.
        let middle = iced::Point::new(size.width / 2.0, size.height / 2.0);
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.input(&mut app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(middle, -60.0));
        shot(&mut snapshot, &mut app, format!("1-kapi-kapali-{name}.png"));
        // Mended: the section drawn again, the line gone.
        run(&mut app, "cad.sections.edit", serde_json::json!({"operation": {"kind": "refresh", "mark": b}}));
        let facts = app.arch_audit_standards_facts().expect("facts");
        let line = facts["cad"]["layers"].as_array().unwrap().iter().find(|l| l["id"] == layer.as_str()).unwrap()["firstFree"].clone();
        run(&mut app, "cad.entities.delete", serde_json::json!({"uids": line}));
        let _ = app.sheet_publish_event(crate::exchange::sheet_publish::Event::Plan);
        finish_the_check(&mut app);
        app.permit_gate_settle();
        let mut update = |app: &mut App, m: Message| {
            let _ = app.update(m);
        };
        snapshot.input(&mut app, App::view, &mut update, kentos_ui::snapshot::Input::Scroll(middle, -60.0));
        shot(&mut snapshot, &mut app, format!("2-kapi-acik-{name}.png"));
    }
}
