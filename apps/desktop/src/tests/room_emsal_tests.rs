//! A room's own emsal state (docs/adr/8125) on the section building (fixtures/arch/v1/section-building.kcad):
//! Öznitelikler says how it counts and why, its basis typed makes the state its own, Emsal artışı (DİĞER) is chosen
//! with it, Kendiliğinden takes it away; each one undo step.
//!
//! ```text
//! RUST_MIN_STACK=67108864 cargo test -p kentos-desktop --lib room_emsal_tests::screens -- --ignored --nocapture
//! ```

use kentos_commands::{EmsalCount, Entity, SpaceChange, SpaceEmsal, SpacesOperation};

use crate::app::{App, Message};
use crate::properties::{Event, Field};

fn app() -> App {
    let (mut app, _) = App::boot(None);
    let snapshot = kentos_kcad::decode(include_bytes!("../../../../fixtures/arch/v1/section-building.kcad"))
        .expect("decodes");
    let doc = crate::document::Document::from_v2(snapshot, None).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

fn row(app: &App, label: &str) -> (String, String) {
    let doc = app.document.as_ref().expect("open");
    let rows: Vec<crate::properties::Row> = app.properties_panel(doc).sections.into_iter().flat_map(|s| s.rows).collect();
    let at = rows.iter().position(|r| r.label == label).unwrap_or_else(|| panic!("no {label} row"));
    (rows[at].value.clone(), rows.get(at + 1).map(|r| r.value.clone()).unwrap_or_default())
}

fn emsal(app: &App, slot: kentos_domain::Slot) -> Option<SpaceEmsal> {
    match app.document.as_ref().expect("open").model.get(slot) {
        Some(Entity::Space(s)) => s.space.emsal.clone(),
        _ => None,
    }
}

#[test]
fn a_room_s_emsal_state_is_chosen_with_its_basis() {
    let mut app = app();
    let doc = &app.document.as_ref().unwrap().model;
    let room = doc
        .entities_with_uids()
        .find_map(|(u, e)| matches!(e, Entity::Space(s) if !s.space.void).then(|| doc.slot_of(u).unwrap()))
        .expect("a room");
    app.selection.set(vec![room]);
    let (value, note) = row(&app, "Emsal durumu");
    assert_eq!(value, "Kendiliğinden (Emsale dahil)");
    assert!(note.starts_with("Kendiliğinden: kullanımından"), "{note}");
    // An increase without its basis: refused, nothing written.
    let set = |c: SpaceChange| Message::Properties(Event::Space(room, Box::new(SpacesOperation::Set(c))));
    let bonus = |basis: Option<&str>| SpaceChange {
        emsal: Some(Some(SpaceEmsal { count: EmsalCount::Bonus, basis: basis.map(str::to_owned) })),
        ..Default::default()
    };
    let _ = app.update(set(bonus(None)));
    assert!(emsal(&app, room).is_none(), "an increase needs its basis");
    // The basis typed: the state it counts as now becomes its own, with it.
    let _ = app.update(Message::Properties(Event::Commit(Field::SpaceEmsalBasis(room), "Sivas İmar Yönetmeliği 22/k".into())));
    assert_eq!(
        emsal(&app, room),
        Some(SpaceEmsal { count: EmsalCount::Included, basis: Some("Sivas İmar Yönetmeliği 22/k".into()) })
    );
    // Then Emsal artışı (DİĞER), the basis kept.
    let _ = app.update(set(bonus(Some("Sivas İmar Yönetmeliği 22/k"))));
    assert_eq!(emsal(&app, room).map(|e| e.count), Some(EmsalCount::Bonus));
    assert_eq!(row(&app, "Emsal durumu").0, "Emsal artışı (DİĞER)");
    assert_eq!(row(&app, "Emsal dayanağı").0, "Sivas İmar Yönetmeliği 22/k");
    let doc = &app.document.as_ref().unwrap().model;
    assert!(doc.undo_labels().len() >= 2);
    assert!(doc.zoning_rooms().iter().any(|r| r.count == EmsalCount::Bonus), "the room counts as an increase");
    // Kendiliğinden: its words' again.
    let _ = app.update(set(SpaceChange { emsal: Some(None), ..Default::default() }));
    assert!(emsal(&app, room).is_none());
}

/// Öznitelikler of a room counted as an emsal increase with its basis, light and dark; `.run/gui-evidence/b8-8125/`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::Snapshot;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/gui-evidence/b8-8125");
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
        let doc = &app.document.as_ref().unwrap().model;
        let room = doc
            .entities_with_uids()
            .find_map(|(u, e)| matches!(e, Entity::Space(s) if !s.space.void).then(|| doc.slot_of(u).unwrap()))
            .expect("a room");
        app.selection.set(vec![room]);
        let _ = app.update(Message::Properties(Event::Space(
            room,
            Box::new(SpacesOperation::Set(SpaceChange {
                emsal: Some(Some(SpaceEmsal {
                    count: EmsalCount::Bonus,
                    basis: Some("Sivas İmar Yönetmeliği 22/k".into()),
                })),
                ..Default::default()
            })),
        )));
        snapshot.settle(&mut app, App::view, &mut update);
        let file = out.join(format!("1-mahal-emsal-artisi-{name}.png"));
        snapshot.render(app.view(), &app.theme()).save(&file).expect("writes the picture");
        println!("{}", file.display());
    }
}
