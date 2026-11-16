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

/// Öznitelikler's rows now, every section's in order.
fn property_rows(app: &App) -> Vec<crate::properties::Row> {
    let doc = app.document.as_ref().expect("open");
    app.properties_panel(doc).sections.into_iter().flat_map(|s| s.rows).collect()
}

/// The first object of a kind, its slot and persistent id.
fn first(app: &App, kind: fn(&Entity) -> bool) -> (kentos_domain::Slot, String) {
    let doc = &app.document.as_ref().expect("the building").model;
    doc.entities_with_uids()
        .find_map(|(u, e)| kind(e).then(|| (doc.slot_of(u).expect("its slot"), u.to_string())))
        .expect("one of the kind")
}

#[test]
fn ozniteliklerde_a_wall_s_class_and_a_door_s_exit_role_are_written_and_taken_away() {
    use crate::properties::{ArchField, Event, Field};
    let mut app = app();
    let (wall, _) = first(&app, |e| matches!(e, Entity::Wall(_)));
    app.selection.set(vec![wall]);
    // The rows: Yangın dayanımı empty, its note “missing”.
    let rows = property_rows(&app);
    let at = rows.iter().position(|r| r.label == "Yangın dayanımı").expect("the class's row");
    assert_eq!(rows[at].value, "");
    assert!(rows[at + 1].value.starts_with("Yok: modelde eksik"), "{}", rows[at + 1].value);
    let _ = app.update(crate::app::Message::Properties(Event::Commit(Field::Arch(wall, ArchField::FireRating), " REI 90 ".into())));
    let doc = &app.document.as_ref().expect("the building").model;
    let Some(Entity::Wall(w)) = doc.get(wall) else { panic!("the wall") };
    assert_eq!(w.wall.arch.fire_rating.as_ref().map(|d| d.value.as_str()), Some("REI 90"), "trimmed, the author's");
    assert_eq!(doc.undo_labels().first().map(String::as_str), Some("Yangın ve performans"));
    // Empty takes it away: missing again, never an empty word.
    let _ = app.update(crate::app::Message::Properties(Event::Commit(Field::Arch(wall, ArchField::FireRating), String::new())));
    let Some(Entity::Wall(w)) = app.document.as_ref().expect("the building").model.get(wall) else { panic!("the wall") };
    assert!(w.wall.arch.fire_rating.is_none());
    // A door: Acil çıkış Evet.
    let (door, _) = first(&app, |e| matches!(e, Entity::Opening(o) if o.opening.form == kentos_commands::OpeningForm::Door));
    app.selection.set(vec![door]);
    assert!(property_rows(&app).iter().any(|r| r.label == "Acil çıkış" && r.value == "Bilinmiyor"));
    let _ = app.update(crate::app::Message::Properties(Event::Exit(door, Some(true))));
    let Some(Entity::Opening(o)) = app.document.as_ref().expect("the building").model.get(door) else { panic!("the door") };
    assert_eq!(o.opening.exit.as_ref().map(|x| x.exit), Some(true));
}

/// Yangın ve performans in the window, light and dark: the Duvar stilleri window with the style's class, Öznitelikler of
/// a wall carrying it from its style and of a door with its exit role, the Denetim tab's missing mandatory fields;
/// `.run/gui-evidence/yangin-performans-8068/`:
///
/// ```text
/// RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib performance_tests::screens -- --ignored --nocapture
/// ```
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    use crate::app::Message;
    use crate::wall_styles::Event as Styles;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/yangin-performans-8068");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (theme, size, name) in [("light", Size::new(1440.0, 1180.0), "acik"), ("dark", Size::new(1280.0, 1100.0), "koyu")] {
        let mut app = app();
        let _ = app.settings.choose(&[("appearance.theme", serde_json::Value::from(theme))]);
        app.apply_settings();
        let _ = app.update(Message::WindowResized(size));
        let _ = app.update(Message::RibbonTab("arch"));
        let mut snapshot = Snapshot::new(size).expect("a renderer");
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut shot = |app: &mut App, file: &str| {
            snapshot.settle(app, App::view, &mut update);
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        // Duvar stilleri: the walls' style given REI 120 (a KentOS style is taken into the project).
        let (wall, _) = first(&app, |e| matches!(e, Entity::Wall(_)));
        let style = match app.document.as_ref().expect("the building").model.get(wall) {
            Some(Entity::Wall(w)) => w.wall.style.clone().expect("a style"),
            _ => unreachable!(),
        };
        let _ = app.update(Message::Run("arch.wallStyles"));
        let _ = app.update(Message::Arch(crate::arch::Event::WallStyles(Styles::Choose(Some(style.clone())))));
        let _ = app.update(Message::Arch(crate::arch::Event::WallStyles(Styles::FireRating("REI 120".into()))));
        shot(&mut app, "1-duvar-stili");
        run(
            &mut app,
            "cad.arch.performance",
            serde_json::json!({"operation": {"kind": "wallStyle", "style": style, "fireRating": {"value": "REI 120", "basis": {"kind": "author"}}}}),
        );
        app.dialog = None;
        app.arch.wall_styles = None;
        // Öznitelikler: the wall carries its style's class.
        app.selection.set(vec![wall]);
        shot(&mut app, "2-duvar-oznitelikler");
        // A door: its own class from a test report and its exit role.
        let (door, uid) = first(&app, |e| matches!(e, Entity::Opening(o) if o.opening.form == kentos_commands::OpeningForm::Door));
        run(
            &mut app,
            "cad.arch.performance",
            serde_json::json!({"operation": {"kind": "objects", "uids": [uid],
                "fireRating": {"value": "EI 60", "basis": {"kind": "document", "name": "Test raporu 2026/41", "date": "2026-11-16"}},
                "exit": {"exit": true, "basis": {"kind": "author"}}}}),
        );
        app.selection.set(vec![door]);
        shot(&mut app, "3-kapi-acil-cikis");
        // Denetim: the model's missing mandatory fields, each with its objects.
        app.selection.set(Vec::new());
        let found = required(&mut app);
        assert!(!found.is_empty());
        app.arch.arch_audit.group = Some(kentos_commands::ArchAuditCheck::Regulation);
        let at = app.arch.arch_audit.audited.as_ref().expect("checked").findings.iter()
            .position(|f| f.code == "digital_required_field" && f.message.contains("Pset_WallCommon.FireRating"))
            .or_else(|| app.arch.arch_audit.audited.as_ref().expect("checked").findings.iter().position(|f| f.code == "digital_required_field"))
            .expect("a mandatory field finding");
        app.arch.arch_audit.selected = Some(at);
        shot(&mut app, "4-denetim-zorunlu-alanlar");
    }
}
