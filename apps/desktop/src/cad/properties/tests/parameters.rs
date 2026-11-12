//! Öznitelikler's tests: block parameters (units, groups, resets, locks, angles and points).

use super::*;

/// CAD-ADV-06: the inspector is a unit-aware view over unchanged canonical block parameters.
pub(super) fn parameter_drawing(unit: kentos_commands::DrawingUnit, unitless: bool) -> (App, Slot) {
    let mut snapshot = kentos_kcad::decode(include_bytes!(
        "../../../../../../fixtures/kcad/v2/dynamic-blocks.kcad"
    ))
    .unwrap();
    snapshot.settings.drawing_unit = Some(unit);
    if unitless {
        snapshot.blocks[0].units = Some(kentos_commands::BlockUnits::Unitless);
    }
    let mut app = crate::files_testing::app_with_drawing();
    let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
    let uid = doc
        .model
        .entities_with_uids()
        .find_map(|(uid, e)| matches!(e, Entity::Insert(_)).then_some(uid))
        .unwrap();
    let slot = doc.model.slot_of(uid).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.selection.set([slot]);
    (app, slot)
}

#[test]
fn block_parameter_mm_input_uses_canonical_metres_rules_and_one_undo() {
    use kentos_commands::DrawingUnit;
    let (mut app, slot) = parameter_drawing(DrawingUnit::Mm, false);
    assert_eq!(value(&app, "Boyut ve konum", "Genişlik"), "900 mm");
    assert!(rows(&app).iter().flat_map(|(_, rows)| rows).any(|r| r.note && r.value == "Tanım varsayılanı · En az 600 mm · En çok 1200 mm · Adım 50 mm"));
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(
        &mut app,
        Event::Commit(Field::Parameter(slot, "Genişlik".into()), "1050".into()),
    );
    assert_eq!(value(&app, "Boyut ve konum", "Genişlik"), "1050 mm");
    assert!(rows(&app).iter().flat_map(|(_, rows)| rows).any(|r| r.note && r.value.starts_with("Örneğe özel · En az 600 mm")));
    let model = &app.document.as_ref().unwrap().model;
    let Entity::Insert(i) = model.get(slot).unwrap() else {
        panic!()
    };
    assert_eq!(
        i.parameters["Genişlik"],
        kentos_commands::ParamValue::Number(1.05)
    );
    let call = model.journal().last().unwrap();
    assert_eq!(call.command.as_deref(), Some("cad.blocks.parameters"));
    assert_eq!(call.input.as_ref().unwrap()["values"]["Genişlik"], 1.05);
    let changed = model.to_snapshot_v2();
    // A real rule failure must be atomic; an invalid suffix must never silently become a number.
    for text in ["2000", "950 bananas", "NaN", ""] {
        event(
            &mut app,
            Event::Commit(Field::Parameter(slot, "Genişlik".into()), text.into()),
        );
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            changed,
            "{text}"
        );
    }
    assert!(undo_label(&mut app).is_some());
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    event(
        &mut app,
        Event::Commit(Field::Parameter(slot, "Genişlik".into()), "105 cm".into()),
    );
    assert_eq!(value(&app, "Boyut ve konum", "Genişlik"), "1050 mm");
}

#[test]
fn a_block_definition_choice_preserves_instance_values_through_the_replacement_command() {
    use kentos_commands::DrawingUnit;
    let (mut app, slot) = parameter_drawing(DrawingUnit::Mm, false);
    let mut snapshot = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let mut target = snapshot.blocks[0].clone();
    let source = target.id;
    target.id =
        serde_json::from_value(serde_json::json!("0192f5a1-2900-7000-8000-0000000000e1")).unwrap();
    target.name = "Düz kapı".into();
    target.dynamic = None;
    let plain = target.id;
    snapshot.blocks.push(target);
    let mut target = snapshot.blocks[0].clone();
    target.id =
        serde_json::from_value(serde_json::json!("0192f5a1-2900-7000-8000-0000000000e2")).unwrap();
    target.name = "Yeni kapı".into();
    let compatible = target.id;
    snapshot.blocks.push(target);
    let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.selection.set([slot]);
    event(
        &mut app,
        Event::Commit(Field::Parameter(slot, "Genişlik".into()), "1050".into()),
    );
    event(
        &mut app,
        Event::Parameter(
            slot,
            "Görünüş".into(),
            kentos_commands::ParamValue::Text("Sade".into()),
        ),
    );
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(&mut app, Event::InsertBlock(slot, plain));
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before,
        "incompatible choice never silently drops overrides"
    );
    event(&mut app, Event::InsertBlock(slot, compatible));
    let doc = app.document.as_ref().unwrap();
    let Entity::Insert(now) = doc.model.get(slot).unwrap() else {
        panic!("insert")
    };
    assert_eq!(now.block, compatible);
    assert_eq!(
        now.parameters["Genişlik"],
        kentos_commands::ParamValue::Number(1.05)
    );
    assert_eq!(
        now.parameters["Görünüş"],
        kentos_commands::ParamValue::Text("Sade".into())
    );
    assert!(
        doc.model
            .entities()
            .filter(|e| e.base().id != slot.0)
            .all(|e| matches!(e,Entity::Insert(i) if i.block==source))
    );
    let call = doc.model.journal().last().unwrap();
    assert_eq!(call.command.as_deref(), Some("cad.blocks.replace"));
    assert_eq!(
        call.input.as_ref().unwrap()["uids"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(kentos_kcad::encode_verified(&doc.model.to_snapshot_v2()).is_ok());
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    event(&mut app, Event::InsertBlock(slot, source));
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before,
        "choosing the same definition is a no-op"
    );
}

#[test]
fn block_parameter_groups_keep_definition_order_and_separate_catalog_appearance_and_computed() {
    use kentos_commands::{BlockParameter, DrawingUnit, NumberRule};
    let (mut app, slot) = parameter_drawing(DrawingUnit::Mm, false);
    let mut snapshot = app.document.as_ref().unwrap().model.to_snapshot_v2();
    snapshot.blocks[0]
        .dynamic
        .as_mut()
        .unwrap()
        .parameters
        .push(BlockParameter::Linear {
            name: "Çift genişlik".into(),
            from: Wire { x: 0., y: 0. },
            to: Wire { x: 1.8, y: 0. },
            rule: NumberRule {
                expression: Some("@Genişlik * 2".into()),
                ..Default::default()
            },
        });
    let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.selection.set([slot]);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let groups = rows(&app);
    let group = |title: &str| {
        groups
            .iter()
            .find(|(t, _)| t == title)
            .unwrap()
            .1
            .iter()
            .filter(|r| !r.note)
            .map(|r| r.label.as_ref())
            .collect::<Vec<_>>()
    };
    assert_eq!(group("Boyut ve konum"), vec!["Genişlik"]);
    assert_eq!(group("Görünüş"), vec!["Yön", "Görünüş"]);
    assert_eq!(group("Katalog"), vec!["Tip"]);
    assert_eq!(group("Hesaplanan"), vec!["Çift genişlik"]);
    assert!(!row_editable(&app, "Hesaplanan", "Çift genişlik"));
    assert_eq!(value(&app, "Blok parametreleri", "Tanım"), "Kapı (dinamik)");
    assert_eq!(value(&app, "Blok parametreleri", "Tanım sürümü"), "1");
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before,
        "display groups do not reorder the definition or write anything"
    );
    event(&mut app, Event::Toggle("blockParamsAppearance"));
    assert!(app.props_closed.contains("blockParamsAppearance"));
    assert!(!app.props_closed.contains("blockParamsDimensions"));
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    event(
        &mut app,
        Event::Commit(Field::Parameter(slot, "Genişlik".into()), "1050".into()),
    );
    assert_eq!(
        value(&app, "Blok parametreleri", "Değerler"),
        "1 değer değişti",
        "derived value and matching lookup are not additional user overrides"
    );
    let sections = rows(&app);
    let reset = sections
        .iter()
        .find(|(s, _)| s == "Blok parametreleri")
        .unwrap()
        .1
        .iter()
        .find(|r| r.label == "Değerler")
        .unwrap();
    let Some(Editor::Select { items, .. }) = &reset.editor else {
        panic!("reset menu")
    };
    assert!(
        items
            .iter()
            .any(|c| matches!(c,Choice::Pick{label,..} if label.starts_with("Genişlik ·")))
    );
    assert!(
        !items
            .iter()
            .any(|c| matches!(c,Choice::Pick{label,..} if label.starts_with("Çift genişlik ·")))
    );
}

#[test]
fn custom_parameter_groups_keep_explicit_member_order_independent_collapse_and_native_editors() {
    use kentos_commands::{BlockParameterGroup, DrawingUnit};
    let (mut app, slot) = parameter_drawing(DrawingUnit::Mm, false);
    let mut snapshot = app.document.as_ref().unwrap().model.to_snapshot_v2();
    snapshot.blocks[0].dynamic.as_mut().unwrap().parameter_groups = vec![
        BlockParameterGroup { name: "Ürün".into(), parameters: vec!["Tip".into(), "Görünüş".into()] },
        BlockParameterGroup { name: "Kanat".into(), parameters: vec!["Yön".into(), "Genişlik".into()] },
    ];
    let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.selection.set([slot]);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    let doc = app.document.as_ref().unwrap();
    let sections = app.properties_panel(doc).sections;
    let custom: Vec<_> = sections.iter().filter(|s| s.id.starts_with("blockParamsGroup:")).collect();
    assert_eq!(custom.iter().map(|s| s.title.as_ref()).collect::<Vec<_>>(), ["Ürün", "Kanat"]);
    assert_eq!(custom[0].rows.iter().filter(|r| !r.note).map(|r| r.label.as_ref()).collect::<Vec<_>>(), ["Tip", "Görünüş"]);
    assert_eq!(custom[1].rows.iter().filter(|r| !r.note).map(|r| r.label.as_ref()).collect::<Vec<_>>(), ["Yön", "Genişlik"]);
    assert_ne!(custom[0].id, custom[1].id);
    let first = custom[0].id.clone();
    let second = custom[1].id.clone();
    event(&mut app, Event::ToggleGroup(first.to_string()));
    assert!(app.props_groups_closed.contains(first.as_ref()));
    assert!(!app.props_groups_closed.contains(second.as_ref()));
    assert_eq!(value(&app, "Kanat", "Genişlik"), "900 mm");
    assert!(row_editable(&app, "Kanat", "Genişlik"));
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before);
}

#[test]
fn automatic_parameter_headings_remain_distinguishable_from_a_custom_group_with_the_same_name() {
    use kentos_commands::{BlockParameterGroup, DrawingUnit};
    let (mut app, slot) = parameter_drawing(DrawingUnit::Mm, false);
    let mut snapshot = app.document.as_ref().unwrap().model.to_snapshot_v2();
    snapshot.blocks[0].dynamic.as_mut().unwrap().parameter_groups = vec![
        BlockParameterGroup { name: "Görünüş".into(), parameters: vec!["Genişlik".into()] },
    ];
    let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.selection.set([slot]);
    assert_eq!(value(&app, "Görünüş", "Genişlik"), "900 mm");
    assert_eq!(value(&app, "Görünüş · Otomatik", "Yön"), "Hayır");
    assert!(rows(&app).iter().any(|(title, _)| title == "Katalog · Otomatik"));
}

#[test]
fn one_parameter_reset_preserves_other_instance_overrides_definition_and_exact_undo() {
    use kentos_commands::{DrawingUnit, ParamValue};
    let (mut app, slot) = parameter_drawing(DrawingUnit::Mm, false);
    event(
        &mut app,
        Event::Commit(Field::Parameter(slot, "Genişlik".into()), "1050".into()),
    );
    event(
        &mut app,
        Event::Parameter(slot, "Görünüş".into(), ParamValue::Text("Sade".into())),
    );
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(&mut app, Event::ParameterReset(slot, "Genişlik".into()));
    assert_eq!(value(&app, "Boyut ve konum", "Genişlik"), "900 mm");
    assert_eq!(value(&app, "Görünüş", "Görünüş"), "Sade");
    assert_eq!(
        value(&app, "Blok parametreleri", "Değerler"),
        "1 değer değişti"
    );
    let model = &app.document.as_ref().unwrap().model;
    assert_eq!(model.to_snapshot_v2().blocks, before.blocks);
    let call = model.journal().last().unwrap();
    assert_eq!(call.command.as_deref(), Some("cad.blocks.parameters"));
    assert_eq!(call.input.as_ref().unwrap()["operation"], "reset");
    assert_eq!(
        call.input.as_ref().unwrap()["names"],
        serde_json::json!(["Genişlik"])
    );
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    // A catalog reset delegates its affected names to the shared command, never resets visibility accidentally.
    event(&mut app, Event::ParameterReset(slot, "Tip".into()));
    assert_eq!(value(&app, "Boyut ve konum", "Genişlik"), "900 mm");
    assert_eq!(value(&app, "Görünüş", "Görünüş"), "Sade");
    let _ = app.update(Message::Run("edit.undo"));
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    event(&mut app, Event::ParameterReset(slot, "Yok".into()));
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before,
        "bad reset is atomic"
    );
}

#[test]
fn locked_parameter_groups_expose_no_reset_or_edit_and_reject_a_stale_menu() {
    use kentos_commands::DrawingUnit;
    let (mut app, slot) = parameter_drawing(DrawingUnit::Mm, false);
    event(
        &mut app,
        Event::Commit(Field::Parameter(slot, "Genişlik".into()), "1050".into()),
    );
    let old = app.properties_key(app.document.as_ref().unwrap());
    let model = &mut app.document.as_mut().unwrap().model;
    model.set_layer_locked("kapilar", true);
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    for (title, rows) in rows(&app) {
        if [
            "Blok parametreleri",
            "Boyut ve konum",
            "Görünüş",
            "Katalog",
            "Hesaplanan",
        ]
        .contains(&title.as_str())
        {
            assert!(rows.iter().all(|r| r.editor.is_none()), "{title}");
        }
    }
    event(&mut app, Event::ParameterReset(slot, "Genişlik".into()));
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    event(
        &mut app,
        Event::Guarded(
            old,
            Box::new(Event::ParameterReset(slot, "Genişlik".into())),
        ),
    );
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}

#[test]
fn block_parameter_unitless_numbers_are_not_reinterpreted_as_metres() {
    use kentos_commands::{DrawingUnit, ParamValue};
    for unit in [
        DrawingUnit::Mm,
        DrawingUnit::Cm,
        DrawingUnit::M,
        DrawingUnit::In,
        DrawingUnit::Ft,
    ] {
        let (mut app, slot) = parameter_drawing(unit, true);
        let label = kentos_interaction::Format::of(app.document.as_ref().unwrap().model.settings())
            .length_unit_label();
        assert_eq!(
            value(&app, "Boyut ve konum", "Genişlik"),
            format!("0.9 {label}")
        );
        event(
            &mut app,
            Event::Commit(Field::Parameter(slot, "Genişlik".into()), "1.05".into()),
        );
        let Entity::Insert(i) = app.document.as_ref().unwrap().model.get(slot).unwrap() else {
            panic!()
        };
        let ParamValue::Number(n) = i.parameters["Genişlik"] else {
            panic!()
        };
        assert!((n - 1.05).abs() < 1e-12, "{unit:?}: {n}");
        assert_eq!(
            value(&app, "Boyut ve konum", "Genişlik"),
            format!("1.05 {label}")
        );
    }
}

#[test]
fn block_parameter_angles_local_points_and_derived_values_follow_project_units() {
    use kentos_commands::{AngleUnit, BlockParameter, DrawingUnit, NumberRule, ParamValue};
    let (mut app, slot) = parameter_drawing(DrawingUnit::Mm, false);
    let mut snapshot = app.document.as_ref().unwrap().model.to_snapshot_v2();
    snapshot.settings.angle_unit = AngleUnit::Grad;
    let d = snapshot.blocks[0].dynamic.as_mut().unwrap();
    d.parameters.extend([
        BlockParameter::Angle {
            name: "Dönüş".into(),
            center: Wire { x: 0., y: 0. },
            base: std::f64::consts::FRAC_PI_2,
            rule: NumberRule::default(),
        },
        BlockParameter::Point {
            name: "Yerel nokta".into(),
            at: Wire { x: 0.125, y: -0.25 },
        },
        BlockParameter::Linear {
            name: "Çift genişlik".into(),
            from: Wire { x: 0., y: 0. },
            to: Wire { x: 1.8, y: 0. },
            rule: NumberRule {
                expression: Some("@Genişlik * 2".into()),
                ..Default::default()
            },
        },
    ]);
    let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app.selection.set([slot]);
    assert_eq!(value(&app, "Boyut ve konum", "Dönüş"), "100.0000 g");
    assert_eq!(
        value(&app, "Boyut ve konum", "Yerel nokta"),
        "125, -250 mm"
    );
    assert_eq!(
        value(&app, "Hesaplanan", "Çift genişlik"),
        "1800 mm"
    );
    assert!(
        rows(&app)
            .iter()
            .flat_map(|(_, r)| r)
            .find(|r| r.label == "Çift genişlik")
            .unwrap()
            .editor
            .is_none()
    );
    assert!(rows(&app).iter().flat_map(|(_, rows)| rows).any(|r| r.note && r.value == "Hesaplanan · @Genişlik * 2"));
    event(
        &mut app,
        Event::Commit(Field::Parameter(slot, "Dönüş".into()), "50".into()),
    );
    event(
        &mut app,
        Event::Commit(
            Field::Parameter(slot, "Yerel nokta".into()),
            "250 mm, -50 cm".into(),
        ),
    );
    assert_eq!(value(&app, "Boyut ve konum", "Dönüş"), "50.0000 g");
    assert_eq!(
        value(&app, "Boyut ve konum", "Yerel nokta"),
        "250, -500 mm"
    );
    let Entity::Insert(i) = app.document.as_ref().unwrap().model.get(slot).unwrap() else {
        panic!()
    };
    let ParamValue::Number(a) = i.parameters["Dönüş"] else {
        panic!()
    };
    assert!((a - std::f64::consts::FRAC_PI_4).abs() < 1e-12);
    assert_eq!(
        i.parameters["Yerel nokta"],
        ParamValue::Point(Wire { x: 0.25, y: -0.5 })
    );
    // Lookup choices remain canonical values; conversion belongs only to the text editor.
    event(
        &mut app,
        Event::Parameter(slot, "Tip".into(), ParamValue::Text("Dar".into())),
    );
    assert_eq!(value(&app, "Boyut ve konum", "Genişlik"), "700 mm");
    assert_eq!(
        value(&app, "Hesaplanan", "Çift genişlik"),
        "1400 mm"
    );
    event(
        &mut app,
        Event::Commit(
            Field::Parameter(slot, "Yerel nokta".into()),
            "1e-8 -2e-8".into(),
        ),
    );
    let shown = rows(&app)
        .into_iter()
        .flat_map(|(_, r)| r)
        .find(|r| r.label == "Yerel nokta")
        .unwrap()
        .value;
    // Values emitted with scientific notation must be accepted again, without scale drift.
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(
        &mut app,
        Event::Commit(Field::Parameter(slot, "Yerel nokta".into()), shown),
    );
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}

#[test]
fn block_parameter_physical_dimensions_stay_independent_of_instance_scale_in_all_units() {
    use kentos_commands::{DrawingUnit, ParamValue};
    for (unit, initial) in [
        (DrawingUnit::Mm, 900.),
        (DrawingUnit::Cm, 90.),
        (DrawingUnit::M, 0.9),
        (DrawingUnit::In, 9000. / 254.),
        (DrawingUnit::Ft, 750. / 254.),
    ] {
        let (mut app, slot) = parameter_drawing(unit, false);
        let model = &app.document.as_ref().unwrap().model;
        let id = model.get(slot).unwrap().base().id;
        let mut snapshot = model.to_snapshot_v2();
        let Entity::Insert(i) = snapshot
            .entities
            .iter_mut()
            .find(|e| e.base().id == id)
            .unwrap()
        else {
            panic!()
        };
        i.scale = 3.;
        let doc = crate::document::Document::from_v2(snapshot, None).unwrap();
        let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
        app.selection.set([slot]);
        let shown = value(&app, "Boyut ve konum", "Genişlik");
        let n: f64 = shown.split_whitespace().next().unwrap().parse().unwrap();
        assert!((n - initial).abs() < 1e-10, "{unit:?}: {shown}");
        event(
            &mut app,
            Event::Commit(Field::Parameter(slot, "Genişlik".into()), "105 cm".into()),
        );
        let Entity::Insert(i) = app.document.as_ref().unwrap().model.get(slot).unwrap() else {
            panic!()
        };
        assert_eq!(i.scale, 3.);
        assert_eq!(i.parameters["Genişlik"], ParamValue::Number(1.05));
    }
}
