use super::*;
use crate::files_testing::{app_with_drawing, make_cad};

const TYPES: &str = include_str!("../../../../fixtures/commands/v1/cad.openingtypes.edit.json");

/// The shared cases' corridor: twenty K01 doors, the sixth keeping its own frame; K02 and P01 beside.
fn app() -> App {
    let mut app = app_with_drawing();
    let file: serde_json::Value = serde_json::from_str(TYPES).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(file["setup"].clone()).unwrap();
    let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(&mut app);
    app
}

fn k01(app: &App) -> OpeningType {
    app.document
        .as_ref()
        .unwrap()
        .model
        .settings()
        .arch
        .as_ref()
        .unwrap()
        .opening_type("K01")
        .unwrap()
        .clone()
}

#[test]
fn the_window_counts_each_type_and_previews_a_change_before_writing() {
    let mut app = app();
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let _ = app.update(Message::Run("arch.openingTypes"));
    assert_eq!(app.dialog, Some(Dialog::OpeningTypes));
    let w = app.opening_types.as_ref().unwrap();
    let k = w.uses.iter().find(|u| u.kind.id == "k01").unwrap();
    assert_eq!((k.count, k.exceptions), (20, 1));
    let i = w.uses.iter().position(|u| u.kind.id == "k01").unwrap();
    let _ = app.update(Message::OpeningTypes(Event::Choose(i)));
    // Nothing typed: nothing to apply.
    assert!(app.opening_type_change().is_none());
    let _ = app.update(Message::OpeningTypes(Event::Field(
        OpeningTypeField::Frame,
        "0.06".into(),
    )));
    let w = app.opening_types.as_ref().unwrap();
    let Some(Ok(done)) = &w.preview else {
        panic!("{:?}", w.preview);
    };
    // Every door takes the new version; the exception keeps its own frame and is counted among those kept.
    assert_eq!(done.report.applied.len(), 20);
    assert_eq!(done.report.kept.len(), 1);
    assert_eq!(done.report.kept[0].fields, vec![OpeningTypeField::Frame]);
    // The preview wrote nothing.
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}

#[test]
fn apply_writes_one_undo_step_and_esc_writes_nothing() {
    let mut app = app();
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let _ = app.update(Message::Run("arch.openingTypes"));
    let i = app
        .opening_types
        .as_ref()
        .unwrap()
        .uses
        .iter()
        .position(|u| u.kind.id == "k01")
        .unwrap();
    let _ = app.update(Message::OpeningTypes(Event::Choose(i)));
    let _ = app.update(Message::OpeningTypes(Event::Field(
        OpeningTypeField::Casing,
        "0.07".into(),
    )));
    let _ = app.update(Message::OpeningTypes(Event::Close));
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before,
        "closing writes nothing"
    );
    let _ = app.update(Message::Run("arch.openingTypes"));
    let _ = app.update(Message::OpeningTypes(Event::Choose(i)));
    let _ = app.update(Message::OpeningTypes(Event::Field(
        OpeningTypeField::Casing,
        "0.07".into(),
    )));
    let _ = app.update(Message::OpeningTypes(Event::Apply));
    let t = k01(&app);
    assert_eq!(t.version, 2);
    assert_eq!(t.build.as_ref().and_then(|b| b.casing), Some(0.07));
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(k01(&app).version, 1);
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before,
        "one undo step"
    );
}

#[test]
fn a_typed_value_that_reads_as_nothing_is_said() {
    let mut app = app();
    let _ = app.update(Message::Run("arch.openingTypes"));
    let _ = app.update(Message::OpeningTypes(Event::Field(
        OpeningTypeField::Frame,
        "beş".into(),
    )));
    assert!(matches!(
        app.opening_types.as_ref().unwrap().preview,
        Some(Err(_))
    ));
}

#[test]
fn same_type_selects_every_door_of_the_selected_ones_type() {
    let mut app = app();
    let doc = app.document.as_ref().unwrap();
    let slot = (0..200u32)
        .map(kentos_domain::Slot)
        .find(|&s| matches!(doc.model.get(s), Some(Entity::Opening(o)) if o.opening.style.as_deref() == Some("k01")))
        .unwrap();
    app.selection.set([slot]);
    let _ = app.update(Message::Run("arch.openingSameType"));
    assert_eq!(app.selection.len(), 20);
}

#[test]
fn detach_and_reset_run_on_the_selection_in_one_step() {
    let mut app = app();
    let doc = app.document.as_ref().unwrap();
    let (slot, uid) = (0..200u32)
        .map(kentos_domain::Slot)
        .find_map(|s| match doc.model.get(s) {
            Some(Entity::Opening(o)) if !o.opening.overrides.is_empty() => {
                Some((s, doc.model.uid(s).unwrap()))
            }
            _ => None,
        })
        .unwrap();
    app.selection.set([slot]);
    let _ = app.update(Message::Run("arch.openingReset"));
    let model = &app.document.as_ref().unwrap().model;
    let Some(Entity::Opening(o)) = model.get(model.slot_of(uid).unwrap()) else {
        panic!("the opening");
    };
    assert!(o.opening.overrides.is_empty());
    let _ = app.update(Message::Run("arch.openingDetach"));
    let model = &app.document.as_ref().unwrap().model;
    let Some(Entity::Opening(o)) = model.get(model.slot_of(uid).unwrap()) else {
        panic!("the opening");
    };
    assert!(o.opening.style.is_none() && o.opening.type_version.is_none());
}

/// The special doors' cases (docs/adr/0777): D01 revolving, CG1 pocket, KT1 folding, each with its door.
const SPECIAL: &str = include_str!("../../../../fixtures/commands/v1/cad.openings.create.special.json");

#[test]
fn a_types_operation_shows_its_own_fields_and_a_switch_reads_evet() {
    let mut app = app_with_drawing();
    let file: serde_json::Value = serde_json::from_str(SPECIAL).unwrap();
    let snapshot: kentos_commands::DocumentSnapshotV2 =
        serde_json::from_value(file["setup"].clone()).unwrap();
    let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    make_cad(&mut app);
    let _ = app.update(Message::Run("arch.openingTypes"));
    let fields = |app: &App| -> Vec<OpeningTypeField> {
        app.opening_types.as_ref().unwrap().typed.iter().map(|(f, _)| *f).collect()
    };
    let i = app.opening_types.as_ref().unwrap().uses.iter().position(|u| u.kind.id == "cg1").unwrap();
    let _ = app.update(Message::OpeningTypes(Event::Choose(i)));
    let typed = fields(&app);
    assert!(typed.contains(&OpeningTypeField::Lip), "{typed:?}");
    assert!(!typed.contains(&OpeningTypeField::Drum), "a pocket door's lip, no drum");
    // Turned to folding: both sides, no lip; the panels folding both ways typed and planned.
    let _ = app.update(Message::OpeningTypes(Event::Operation(OpeningOperation::Folding)));
    let typed = fields(&app);
    assert!(typed.contains(&OpeningTypeField::BothSides) && !typed.contains(&OpeningTypeField::Lip), "{typed:?}");
    let _ = app.update(Message::OpeningTypes(Event::Field(OpeningTypeField::Leaves, "4".into())));
    let _ = app.update(Message::OpeningTypes(Event::Field(OpeningTypeField::BothSides, "Evet".into())));
    let w = app.opening_types.as_ref().unwrap();
    let Some(Ok(done)) = &w.preview else {
        panic!("{:?}", w.preview);
    };
    let cg = done.types.iter().find(|t| t.id == "cg1").unwrap();
    assert_eq!(cg.operation, OpeningOperation::Folding);
    assert_eq!(cg.build.as_ref().and_then(|b| b.both_sides), Some(true));
    assert_eq!(
        cg.build.as_ref().and_then(|b| b.lip),
        None,
        "a pocket's lip goes with the pocket"
    );
}

/// A type's frame and leaves and its U value (M38, docs/adr/8038): typed in the window, kept on the type; a window
/// above TS 825:2024's 1,8 W/m²K is Mevzuat's finding, at or below it none.
#[test]
fn a_window_type_says_its_frame_and_u_value_and_ts_825_judges_it() {
    let mut app = app();
    let _ = app.update(Message::Run("arch.openingTypes"));
    let i = {
        let w = app.opening_types.as_ref().unwrap();
        w.uses.iter().position(|u| u.kind.id == "p01").unwrap()
    };
    let _ = app.update(Message::OpeningTypes(Event::Choose(i)));
    // An unreadable U value is said, not written.
    let _ = app.update(Message::OpeningTypes(Event::UValue("iki".into())));
    assert!(
        matches!(app.opening_type_change(), Some(Err(ref why)) if why.starts_with("U değeri:")),
        "{:?}",
        app.opening_type_change()
    );
    let _ = app.update(Message::OpeningTypes(Event::Material(
        "PVC, 3 odacıklı; çift camlı".into(),
    )));
    let _ = app.update(Message::OpeningTypes(Event::UValue("2,4".into())));
    let _ = app.update(Message::OpeningTypes(Event::Apply));
    let p01 = |app: &App| {
        app.document
            .as_ref()
            .unwrap()
            .model
            .settings()
            .arch
            .as_ref()
            .unwrap()
            .opening_type("P01")
            .unwrap()
            .clone()
    };
    let t = p01(&app);
    assert_eq!(
        (t.material.as_deref(), t.u_value),
        (Some("PVC, 3 odacıklı; çift camlı"), Some(2.4))
    );
    let findings = |app: &mut App| -> Vec<(String, String)> {
        let doc = &mut app.document.as_mut().unwrap().model;
        let answer = kentos_native_application::registry::run(
            &mut kentos_native_application::ExecutionContext::new(doc),
            "cad.arch.audit",
            kentos_commands::CallOp::Execute,
            serde_json::json!({ "checks": ["regulation"] }),
        )
        .unwrap();
        answer["output"]["findings"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|f| f["code"] == "window_u_high")
            .map(|f| {
                (
                    f["message"].as_str().unwrap_or_default().to_owned(),
                    f["rule"].as_str().unwrap_or_default().to_owned(),
                )
            })
            .collect()
    };
    let said = findings(&mut app);
    assert_eq!(
        said,
        [(
            "P01 (1 pencere): U 2,4 W/m²K > 1,8 W/m²K — TS 825 tavsiye edilen U değerleri (pencere 1,8 W/m²K)"
                .to_owned(),
            "TS 825:2024 tavsiye edilen U değerleri (pencere 1,8 W/m²K) · RG 20.2.2025/32819, Tebliğ MHGM-2025/1"
                .to_owned()
        )]
    );
    // At TS 825's value: no finding; the window shows the type's own again.
    let _ = app.update(Message::OpeningTypes(Event::UValue("1,8".into())));
    let _ = app.update(Message::OpeningTypes(Event::Apply));
    assert_eq!(p01(&app).u_value, Some(1.8));
    assert!(findings(&mut app).is_empty());
    let _ = app.update(Message::OpeningTypes(Event::Choose(i)));
    let w = app.opening_types.as_ref().unwrap();
    assert_eq!(
        (w.material.as_str(), w.u_value.as_str()),
        ("PVC, 3 odacıklı; çift camlı", "1,8")
    );
}

/// The chosen type's name and form.
fn chosen(app: &App) -> (String, OpeningForm) {
    let w = app.opening_types.as_ref().unwrap();
    let u = &w.uses[w.chosen];
    (u.kind.name.clone(), u.kind.form)
}

/// Yeni tip ▾'s item with these words, as a trace picks it.
fn pick_new(app: &mut App, item: &str) {
    let message = app
        .opening_types_control(Control::Pick(NEW, item))
        .expect("an item of Yeni tip ▾")
        .expect("a message");
    let _ = app.update(message);
}

/// The form of a new type was nowhere to choose (an owner's report: “yeni tip dediğimde hep aynı tip geliyor”): Yeni
/// tip copied the chosen type, a door while the list starts with one. Yeni tip ▾ asks for the form first; a copy of a
/// copy is numbered, never “… kopyası kopyası”.
#[test]
fn a_new_type_is_the_form_picked_or_a_copy_of_the_chosen_one() {
    let mut app = app();
    let _ = app.update(Message::Run("arch.openingTypes"));
    let i = app
        .opening_types
        .as_ref()
        .unwrap()
        .uses
        .iter()
        .position(|u| u.kind.id == "k01")
        .unwrap();
    let _ = app.update(Message::OpeningTypes(Event::Choose(i)));
    // A window with a door chosen: KentOS's first window's values, its own name, chosen in the list.
    pick_new(&mut app, "Pencere");
    assert_eq!(
        chosen(&app),
        ("Yeni pencere tipi".to_owned(), OpeningForm::Window)
    );
    // Said without “tipi tipi”.
    assert_eq!(
        crate::files_testing::last_said(&app),
        "Tip eklendi: Yeni pencere tipi (pencere)."
    );
    let made = app
        .document
        .as_ref()
        .unwrap()
        .model
        .settings()
        .arch
        .as_ref()
        .unwrap()
        .opening_type("Yeni pencere tipi")
        .unwrap()
        .clone();
    let first = kentos_commands::builtin_opening_types()
        .into_iter()
        .find(|t| t.form == OpeningForm::Window)
        .unwrap();
    assert_eq!(
        (made.width, made.sill, made.head),
        (first.width, first.sill, first.head)
    );
    // The chosen type's copy keeps its form; a copy of the copy is numbered.
    pick_new(&mut app, COPY);
    assert_eq!(
        chosen(&app),
        ("Yeni pencere tipi kopyası".to_owned(), OpeningForm::Window)
    );
    pick_new(&mut app, COPY);
    assert_eq!(
        chosen(&app),
        (
            "Yeni pencere tipi kopyası 2".to_owned(),
            OpeningForm::Window
        )
    );
    // Each form of the menu.
    for (item, form, name) in [
        ("Kapı", OpeningForm::Door, "Yeni kapı tipi"),
        ("Boşluk", OpeningForm::Opening, "Yeni boşluk tipi"),
        ("Niş", OpeningForm::Niche, "Yeni niş tipi"),
        ("Baca", OpeningForm::Flue, "Yeni baca tipi"),
        ("Pencere", OpeningForm::Window, "Yeni pencere tipi 2"),
    ] {
        pick_new(&mut app, item);
        assert_eq!(chosen(&app), (name.to_owned(), form), "{item}");
    }
    // An item the menu does not have is said, nothing made.
    assert!(
        app.opening_types_control(Control::Pick(NEW, "Kapak"))
            .is_err()
    );
    // The names the copies take.
    let t = |name: &str| OpeningType {
        name: name.into(),
        ..made.clone()
    };
    assert_eq!(new_name(None, Some(&t("İç kapı 90"))), "İç kapı 90 kopyası");
    assert_eq!(
        new_name(None, Some(&t("İç kapı 90 kopyası"))),
        "İç kapı 90 kopyası"
    );
    assert_eq!(
        new_name(None, Some(&t("İç kapı 90 kopyası 3"))),
        "İç kapı 90 kopyası"
    );
    assert_eq!(
        new_name(None, Some(&t("Kopyası olmayan"))),
        "Kopyası olmayan kopyası"
    );
    assert_eq!(new_name(Some(OpeningForm::Niche), None), "Yeni niş tipi");
}
