//! B7's remainder (docs/adr/8124) on the section building (fixtures/arch/v1/section-building.kcad): Öznitelikler says
//! what the drawing reads (a wall outside on its storey's outline, a column of the frame carrying) and why, and the
//! object's own declaration changes it; a fire class's basis is one line; the door and window type's class and the
//! building's fire protection class are written from their windows; the audit suggests the exits and its fix writes them.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib declared_readings_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::{Entity, OpeningForm};
use crate::arch::ArchState;

use crate::app::{App, Message};
use crate::project::settings::Section;
use crate::project::{SettingsEvent, settings_message};
use crate::properties::{ArchField, Event, Field};

fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/section-building.kcad"))
        .expect("decodes");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn rows(app: &App) -> Vec<crate::properties::Row> {
    let doc = app.document.as_ref().expect("open");
    app.properties_panel(doc).sections.into_iter().flat_map(|s| s.rows).collect()
}

/// A row's value and the note under it.
fn row(app: &App, label: &str) -> (String, String) {
    let rows = rows(app);
    let at = rows.iter().position(|r| r.label == label).unwrap_or_else(|| panic!("no {label} row"));
    (rows[at].value.clone(), rows.get(at + 1).map(|r| r.value.clone()).unwrap_or_default())
}

fn first(app: &App, kind: fn(&Entity) -> bool) -> kentos_domain::Slot {
    let doc = &app.document.as_ref().expect("the building").model;
    doc.entities_with_uids().find_map(|(u, e)| kind(e).then(|| doc.slot_of(u).expect("its slot"))).expect("one")
}

fn entity(app: &App, slot: kentos_domain::Slot) -> Entity {
    app.document.as_ref().expect("the building").model.get(slot).expect("the object").clone()
}

fn outer_wall(e: &Entity) -> bool {
    matches!(e, Entity::Wall(w) if w.wall.pts.first().is_some_and(|p| p.y == 0.0)
        && w.wall.pts.last().is_some_and(|p| p.y == 0.0))
}

#[test]
fn ozniteliklerde_the_drawing_s_reading_is_said_and_changed_on_the_object() {
    let mut app = app();
    let wall = first(&app, outer_wall);
    app.selection.set(vec![wall]);
    let (value, note) = row(&app, "Dışarıda");
    assert_eq!(value, "Kendiliğinden (Evet)");
    assert_eq!(note, "Kendiliğinden: katın dış çizgisinde olduğu için dışarıda.");
    // Declared inside: the model writes the declaration; the note says the drawing reads otherwise.
    let _ = app.update(Message::Properties(Event::External(wall, Some(false))));
    let Entity::Wall(w) = entity(&app, wall) else { panic!("the wall") };
    assert_eq!(w.wall.arch.external.as_ref().map(|x| x.external), Some(false));
    assert_eq!(app.document.as_ref().unwrap().model.undo_labels().first().map(String::as_str), Some("Yangın ve performans"));
    let (value, note) = row(&app, "Dışarıda");
    assert_eq!(value, "Hayır");
    assert_eq!(note, "Beyan geçer; çizimde dışarıda.");
    // Kendiliğinden takes the declaration away.
    let _ = app.update(Message::Properties(Event::External(wall, None)));
    let Entity::Wall(w) = entity(&app, wall) else { panic!("the wall") };
    assert!(w.wall.arch.external.is_none());
    // A window in it: as its wall, not changed on it.
    let window = first(&app, |e| matches!(e, Entity::Opening(o) if o.opening.form == OpeningForm::Window));
    app.selection.set(vec![window]);
    let (value, note) = row(&app, "Dışarıda");
    assert_eq!(value, "Evet");
    assert!(note.contains("duvarı dışarıda olduğu için"), "{note}");
    // A column of the frame carries unless declared.
    let column = first(&app, |e| matches!(e, Entity::Column(c) if c.column.member.is_none()));
    app.selection.set(vec![column]);
    let (value, note) = row(&app, "Taşıyıcı");
    assert_eq!((value.as_str(), note.as_str()), ("Kendiliğinden (Evet)", "Kendiliğinden: çerçevenin kolonu olduğu için taşıyıcı."));
    let _ = app.update(Message::Properties(Event::LoadBearing(column, Some(false))));
    let Entity::Column(c) = entity(&app, column) else { panic!("the column") };
    assert_eq!(c.column.arch.load_bearing.as_ref().map(|b| b.bearing), Some(false));
    // A floor slab: inside as a floor.
    let slab = first(&app, |e| matches!(e, Entity::Slab(s) if s.slab.covering.is_none()));
    app.selection.set(vec![slab]);
    assert!(row(&app, "Dışarıda").1.contains("döşemesi olduğu için"));
}

#[test]
fn a_class_s_basis_is_one_line_and_stays_when_the_class_changes() {
    let mut app = app();
    let wall = first(&app, outer_wall);
    app.selection.set(vec![wall]);
    let commit = |app: &mut App, field: ArchField, text: &str| {
        let _ = app.update(Message::Properties(Event::Commit(Field::Arch(wall, field), text.into())));
    };
    commit(&mut app, ArchField::FireRating, "REI 90");
    assert_eq!(row(&app, "Dayanak belgesi").0, "", "the author's statement");
    commit(&mut app, ArchField::FireBasis, "Test raporu 41 (2026-02-28)");
    let basis = |app: &App| match entity(app, wall) {
        Entity::Wall(w) => w.wall.arch.fire_rating.clone(),
        _ => None,
    };
    assert_eq!(
        basis(&app).map(|d| (d.value, d.basis)),
        Some(("REI 90".into(), kentos_commands::DeclaredBasis::Document {
            name: "Test raporu 41".into(),
            date: Some("2026-02-28".into())
        }))
    );
    assert_eq!(row(&app, "Dayanak belgesi").0, "Test raporu 41 (2026-02-28)");
    // The class changed: its basis kept.
    commit(&mut app, ArchField::FireRating, "REI 120");
    assert_eq!(basis(&app).map(|d| (d.value, d.basis.words())), Some(("REI 120".into(), "Test raporu 41 (2026-02-28)".into())));
    // A day that is none: refused, nothing written.
    let before = app.document.as_ref().unwrap().model.undo_labels().len();
    commit(&mut app, ArchField::FireBasis, "Rapor (2026-02-30)");
    assert_eq!(app.document.as_ref().unwrap().model.undo_labels().len(), before);
    assert_eq!(basis(&app).map(|d| d.basis.words()), Some("Test raporu 41 (2026-02-28)".into()));
}

/// The Kapı ve pencere tipleri window open with a door type chosen: the project's, else one made from KentOS's first door.
fn door_type(app: &mut App) -> (usize, String) {
    use crate::opening_types_window::Event as Types;
    let _ = app.update(Message::Run("arch.openingTypes"));
    let find = |app: &App| {
        app.modules.arch().opening_types.as_ref().expect("the window").uses.iter().position(|u| u.kind.form == OpeningForm::Door)
    };
    if find(app).is_none() {
        let _ = app.update(Message::from(crate::arch::Event::OpeningTypes(Types::New(Some(OpeningForm::Door)))));
    }
    let at = find(app).expect("a door type");
    let id = app.modules.arch().opening_types.as_ref().unwrap().uses[at].kind.id.clone();
    let _ = app.update(Message::from(crate::arch::Event::OpeningTypes(Types::Choose(at))));
    (at, id)
}

#[test]
fn the_door_type_s_class_is_written_from_its_window_with_its_basis() {
    use crate::opening_types_window::Event as Types;
    let mut app = app();
    let (_, id) = door_type(&mut app);
    let types = |e| Message::from(crate::arch::Event::OpeningTypes(e));
    let _ = app.update(types(Types::Fire("EI 60".into())));
    let _ = app.update(types(Types::FireBasis("Ürün performans beyanı (2026-01-05)".into())));
    assert!(app.modules.arch().opening_types.as_ref().unwrap().declared.is_none());
    let _ = app.update(types(Types::Apply));
    let doc = &app.document.as_ref().unwrap().model;
    let t = doc.settings().arch.as_ref().unwrap().opening_types.iter().find(|t| t.id == id).expect("the type");
    let d = t.fire_rating.clone().expect("its class");
    assert_eq!((d.value.as_str(), d.basis.words().as_str()), ("EI 60", "Ürün performans beyanı (2026-01-05)"));
    assert_eq!(doc.undo_labels().first().map(String::as_str), Some("Yangın ve performans"));
    // A basis whose day is none is said before Uygula.
    let _ = app.update(types(Types::FireBasis("Rapor (2026-13-01)".into())));
    assert!(app.modules.arch().opening_types.as_ref().unwrap().declared.as_deref().is_some_and(|w| w.contains("bir gün değil")));
}

#[test]
fn the_building_s_fire_protection_class_is_saved_with_the_project_s_settings() {
    use crate::project::arch::ZoningField;
    let mut app = app();
    let _ = app.update(Message::Run("file.settings"));
    let _ = app.update(settings_message(SettingsEvent::Section(Section::Arch)));
    let _ = app.update(settings_message(SettingsEvent::ArchZoning(ZoningField::FireClass, "Konut, düşük tehlike".into())));
    let _ = app.update(settings_message(SettingsEvent::ArchZoning(ZoningField::FireBasis, "Yangın raporu (2026-11-18)".into())));
    let _ = app.update(settings_message(SettingsEvent::Save));
    assert!(app.project.is_none(), "saved");
    let doc = &app.document.as_ref().unwrap().model;
    let class = doc
        .settings()
        .arch
        .as_ref()
        .and_then(|a| a.performance.as_ref())
        .and_then(|p| p.fire_protection_class.clone())
        .expect("the class");
    assert_eq!((class.value.as_str(), class.basis.words().as_str()), ("Konut, düşük tehlike", "Yangın raporu (2026-11-18)"));
}

/// The audit's exit suggestion: one finding naming the candidates, its fix writing them all; then none is left.
#[test]
fn the_audit_suggests_the_exits_and_its_fix_writes_them() {
    let mut app = app();
    let suggestion = |app: &mut App| {
        let _ = app.update(Message::Run("arch.audit"));
        app.arch_audit_standards_settle();
        app.modules.arch()
            .arch_audit
            .audited
            .as_ref()
            .expect("checked")
            .findings
            .iter()
            .filter(|f| f.code.starts_with("digital_exit_"))
            .cloned()
            .collect::<Vec<_>>()
    };
    let found = suggestion(&mut app);
    let s = found.iter().find(|f| f.code == "digital_exit_suggestion").expect("the suggestion");
    let fix = s.fix.clone().expect("its fix");
    assert_eq!(fix.step.command, "cad.arch.performance");
    assert_eq!(fix.step.input["operation"]["kind"], "exits");
    let doc = &mut app.document.as_mut().unwrap().model;
    let answer = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(doc),
        &fix.step.command,
        kentos_commands::CallOp::Execute,
        fix.step.input.clone(),
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{answer}");
    let left = suggestion(&mut app);
    assert!(left.is_empty(), "{left:#?}");
}

/// The B7 remainder in the window, light and dark: Öznitelikler of an outer wall declared inside, of a column, the door
/// type's class, Yangın güvenliği in Proje ayarları, the audit's exit suggestion; `.run/gui-evidence/b7-8124/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    use crate::opening_types_window::Event as Types;
    use crate::project::arch::ZoningField;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/b7-8124");
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
        // `scroll`: down a page to its end, then this many steps up (the settings' Yangın güvenliği).
        let mut shot = |app: &mut App, file: &str, scroll: Option<f32>| {
            snapshot.settle(app, App::view, &mut update);
            if let Some(up) = scroll {
                let at = iced::Point::new(size.width * 0.55, size.height * 0.5);
                snapshot.input(app, App::view, &mut update, Input::Scroll(at, -60.0));
                snapshot.settle(app, App::view, &mut update);
                snapshot.input(app, App::view, &mut update, Input::Scroll(at, up));
                snapshot.settle(app, App::view, &mut update);
            }
            let file = out.join(format!("{file}-{name}.png"));
            snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
            println!("{}", file.display());
        };
        // Öznitelikler: an outer wall with its class from a test report, declared inside.
        let wall = first(&app, outer_wall);
        app.selection.set(vec![wall]);
        let _ = app.update(Message::Properties(Event::Commit(Field::Arch(wall, ArchField::FireRating), "REI 90".into())));
        let _ = app.update(Message::Properties(Event::Commit(
            Field::Arch(wall, ArchField::FireBasis),
            "Test raporu 41 (2026-02-28)".into(),
        )));
        let _ = app.update(Message::Properties(Event::External(wall, Some(false))));
        shot(&mut app, "1-duvar-disarida", None);
        // A column of the frame: carries, said why.
        let column = first(&app, |e| matches!(e, Entity::Column(c) if c.column.member.is_none()));
        app.selection.set(vec![column]);
        shot(&mut app, "2-kolon-tasiyici", None);
        app.selection.set(Vec::new());
        // Kapı ve pencere tipleri: the door type's class and basis typed.
        door_type(&mut app);
        let types = |e| Message::from(crate::arch::Event::OpeningTypes(e));
        let _ = app.update(types(Types::Fire("EI 60".into())));
        let _ = app.update(types(Types::FireBasis("Ürün performans beyanı (2026-01-05)".into())));
        shot(&mut app, "3-kapi-tipi-yangin", None);
        let _ = app.update(types(Types::Apply));
        app.dialog = None;
        app.modules.arch_mut().opening_types = None;
        // Proje ayarları › Mimari › Yangın güvenliği.
        let _ = app.update(Message::Run("file.settings"));
        let _ = app.update(settings_message(SettingsEvent::Section(Section::Arch)));
        let _ = app.update(settings_message(SettingsEvent::ArchZoning(ZoningField::FireClass, "Konut, düşük tehlike".into())));
        let _ = app.update(settings_message(SettingsEvent::ArchZoning(
            ZoningField::FireBasis,
            "Yangın raporu (2026-11-18)".into(),
        )));
        shot(&mut app, "4-yangin-koruma-sinifi", Some(3.0));
        let _ = app.update(settings_message(SettingsEvent::Save));
        // Denetim: the exit suggestion chosen.
        let _ = app.update(Message::Run("arch.audit"));
        app.arch_audit_standards_settle();
        app.modules.arch_mut().arch_audit.group = Some(kentos_commands::ArchAuditCheck::Regulation);
        let at = app.modules.arch().arch_audit.audited.as_ref().expect("checked").findings.iter()
            .position(|f| f.code == "digital_exit_suggestion").expect("the suggestion");
        app.modules.arch_mut().arch_audit.selected = Some(at);
        shot(&mut app, "5-denetim-acil-cikis-onerisi", None);
    }
}
