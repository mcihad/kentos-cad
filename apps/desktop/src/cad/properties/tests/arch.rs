//! Öznitelikler's tests: architectural objects' lengths and units, virtual separators, Kot marks, object
//! isolation, text heights, the selection limit, walls, rooms and slabs.

use super::*;

/// Physical fields share the tool grammar; paper mm, percentages and counts do not.
#[test]
fn cad_element_lengths_keep_physical_units_and_nonlength_fields() {
    use super::super::ArchField as A;
    use kentos_commands::{DrawingUnit, Workspace};
    use serde_json::json;
    for unit in [DrawingUnit::Mm, DrawingUnit::Cm, DrawingUnit::M, DrawingUnit::In, DrawingUnit::Ft] {
        let mut app = objects();
        let doc = &mut app.document.as_mut().unwrap().model;
        let mut settings = doc.settings().clone();
        settings.workspace = Some(Workspace::Cad);
        settings.srid = 0;
        settings.drawing_unit = Some(unit);
        doc.set_settings(settings);
        let column = add(&mut app, serde_json::from_value(json!({
            "kind":"column", "id":0,"layerId":"parsel","attrs":{},
            "at":{"x":0.,"y":0.},"width":0.5,"depth":0.6
        })).unwrap());
        let stair = add(&mut app, serde_json::from_value(json!({
            "kind":"stair", "id":0,"layerId":"parsel","attrs":{},
            "pts":[{"x":0.,"y":0.},{"x":6.,"y":0.}],
            "width":1.,"risers":18,"riserHeight":0.17,"tread":0.28
        })).unwrap());
        let ramp = add(&mut app, serde_json::from_value(json!({
            "kind":"ramp", "id":0,"layerId":"parsel","attrs":{},
            "pts":[{"x":0.,"y":0.},{"x":10.,"y":0.}],"width":1.,"rise":0.5
        })).unwrap());
        let grid = add(&mut app, serde_json::from_value(json!({
            "kind":"grid", "id":0,"layerId":"parsel","attrs":{},
            "pts":[{"x":0.,"y":0.},{"x":10.,"y":0.}],"name":"A"
        })).unwrap());
        let snapshot = |app: &App| app.document.as_ref().unwrap().model.to_snapshot_v2();
        for (slot, field, key, metric, expected, imperial, inch_value) in [
            (column,A::Width,"width","40 cm",0.4,"16 in",0.4064),
            (column,A::Depth,"depth","60,5 cm",0.605,"24 in",0.6096),
            (stair,A::Width,"width","120 cm",1.2,"4 ft",1.2192),
            (stair,A::RiserHeight,"riserHeight","18 cm",0.18,"7 in",0.1778),
            (stair,A::Tread,"tread","30 cm",0.3,"12 in",0.3048),
            (stair,A::Landing,"landing","150 cm",1.5,"5 ft",1.524),
            (ramp,A::Width,"width","150 cm",1.5,"5 ft",1.524),
            (ramp,A::Rise,"rise","80 cm",0.8,"3 ft",0.9144),
            (ramp,A::Landing,"landing","150 cm",1.5,"5 ft",1.524),
            (ramp,A::Base,"base","-50 cm",-0.5,"-2 ft",-0.6096),
        ] {
            let before = snapshot(&app);
            for (text, want) in [(metric,expected),(imperial,inch_value)] {
                event(&mut app, Event::Commit(Field::Arch(Slot(slot),field),text.into()));
                let got = serde_json::to_value(entity(&app,slot)).unwrap()[key].as_f64().unwrap();
                assert!((got-want).abs()<1e-10,"{unit:?}/{key}/{text}: {got}; {:?}",warned(&app));
                assert!(undo_label(&mut app).is_some());
                assert_eq!(snapshot(&app),before,"undo: {key}/{text}");
            }
            for text in ["40 cm junk","1,2,3","NaN","40 apples"] {
                event(&mut app, Event::Commit(Field::Arch(Slot(slot),field),text.into()));
                assert_eq!(snapshot(&app),before,"invalid: {key}/{text}");
                assert!(warned(&app).is_some());
            }
            if matches!(field,A::Depth|A::Landing|A::Base) {
                event(&mut app,Event::Commit(Field::Arch(Slot(slot),field),metric.into()));
                let set = snapshot(&app);
                event(&mut app,Event::Commit(Field::Arch(Slot(slot),field),String::new()));
                assert!(serde_json::to_value(entity(&app,slot)).unwrap().get(key).is_none());
                assert!(undo_label(&mut app).is_some());
                assert_eq!(snapshot(&app),set);
                assert!(undo_label(&mut app).is_some());
                assert_eq!(snapshot(&app),before);
            } else {
                event(&mut app,Event::Commit(Field::Arch(Slot(slot),field),String::new()));
                assert_eq!(snapshot(&app),before,"required empty: {key}");
            }
        }
        for (slot,field,key,text,want) in [
            (column,A::Turn,"rotation","90",std::f64::consts::FRAC_PI_2),
            (stair,A::Risers,"risers","17",17.),
            (ramp,A::Slope,"slope","8",8.),
            (grid,A::BubbleSize,"bubbleSize","10",10.),
        ] {
            let before=snapshot(&app);
            event(&mut app,Event::Commit(Field::Arch(Slot(slot),field),text.into()));
            let got=serde_json::to_value(entity(&app,slot)).unwrap()[key].as_f64().unwrap();
            assert!((got-want).abs()<1e-10,"nonlength {key}");
            assert!(undo_label(&mut app).is_some());
            assert_eq!(snapshot(&app),before);
        }
        // Negative base is legal; negative physical size remains an atomic domain refusal.
        for (slot,field) in [(column,A::Width),(column,A::Depth),(stair,A::RiserHeight),(ramp,A::Landing)] {
            let before=snapshot(&app);
            event(&mut app,Event::Commit(Field::Arch(Slot(slot),field),"-1 m".into()));
            assert_eq!(snapshot(&app),before);
        }
        let before=snapshot(&app);
        let f=kentos_interaction::Format::of(app.document.as_ref().unwrap().model.settings());
        event(&mut app,Event::Commit(Field::Arch(Slot(column),A::Width),"2".into()));
        assert_eq!(serde_json::to_value(entity(&app,column)).unwrap()["width"].as_f64(),Some(f.to_metres(2.)));
        assert!(undo_label(&mut app).is_some());
        assert_eq!(snapshot(&app),before);
        let doc=&mut app.document.as_mut().unwrap().model;
        let mut settings=doc.settings().clone();settings.workspace=Some(Workspace::Gis);doc.set_settings(settings);
        let before=snapshot(&app);
        event(&mut app,Event::Commit(Field::Arch(Slot(column),A::Width),"2 cm".into()));
        assert_eq!(serde_json::to_value(entity(&app,column)).unwrap()["width"].as_f64(),Some(f.to_metres(2.)));
        assert!(undo_label(&mut app).is_some());
        assert_eq!(snapshot(&app),before);
    }
}

/// Mahal ayırıcı is a building's (docs/adr/0802): KentOS Harita's and KentOS GIS's polylines have no such row,
/// KentOS Mimari's and kentos-cad's have.
#[test]
fn a_maps_polyline_has_no_space_separator_row() {
    for (program, shown) in [
        (Some(kentos_launch::Module::Map), false),
        (Some(kentos_launch::Module::Gis), false),
        (Some(kentos_launch::Module::Architecture), true),
        (None, true),
    ] {
        let mut app = objects();
        app.program = program;
        let doc = &mut app.document.as_mut().unwrap().model;
        let mut settings = doc.settings().clone();
        settings.workspace = Some(kentos_commands::Workspace::Cad);
        doc.set_settings(settings);
        let id = path_z(&mut app, None);
        select(&mut app, &[id]);
        assert_eq!(has_row(&app, "Geometri", "Mahal ayırıcı"), shown, "{program:?}");
    }
}

#[test]
fn virtual_separator_property_role_phase_lock_and_single_undo_use_the_semantic_command() {
    let mut app = objects();
    let doc = &mut app.document.as_mut().unwrap().model;
    let mut settings = doc.settings().clone();
    settings.workspace = Some(kentos_commands::Workspace::Cad);
    doc.set_settings(settings);
    let id = path_z(&mut app, None);
    select(&mut app, &[id]);
    assert_eq!(value(&app, "Geometri", "Mahal ayırıcı"), "Hayır");
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(&mut app, Event::SpaceSeparator(Slot(id), true));
    assert_eq!(value(&app, "Geometri", "Mahal ayırıcı"), "Evet");
    assert!(!row_editable(&app, "Geometri", "Genişlik"));
    assert!(!row_editable(&app, "Geometri", "Kot"));
    assert!(has_row(&app, "Geometri", "Kat") && has_row(&app, "Geometri", "Tadilat durumu"));
    assert_eq!(undo_label(&mut app).as_deref(), Some("Mahal ayırıcı"));
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
    event(&mut app, Event::SpaceSeparator(Slot(id), true));
    let role = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(
        &mut app,
        Event::Arch(
            Slot(id),
            serde_json::from_value(serde_json::json!({"kind":"set","phase":"existing"})).unwrap(),
        ),
    );
    let Entity::Polyline(p) = entity(&app, id) else {
        panic!()
    };
    assert_eq!(
        p.space_separator.unwrap().phase,
        Some(kentos_commands::ArchPhase::Existing)
    );
    assert_eq!(undo_label(&mut app).as_deref(), Some("Mahal ayırıcı"));
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), role);
    app.document
        .as_mut()
        .unwrap()
        .model
        .set_layer_locked("cizim", true);
    let locked = app.document.as_ref().unwrap().model.to_snapshot_v2();
    assert!(!row_editable(&app, "Geometri", "Mahal ayırıcı"));
    event(&mut app, Event::SpaceSeparator(Slot(id), false));
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        locked
    );
}

/// A Kot's rows (docs/adr/0721 §2): Kot, Kat ▾ (Yok or a level of the project) and, while bound, Fark; the value
/// read only while the level gives it. Each change one step “Kotun katı”.
#[test]
fn a_kots_rows_bind_it_to_a_level_and_set_its_offset() {
    use kentos_commands::DimensionStyle::Elevation;
    let mut app = objects();
    crate::files_testing::make_cad(&mut app);
    {
        let doc = &mut app.document.as_mut().expect("open").model;
        let mut s = doc.settings().clone();
        s.arch = Some(
            serde_json::from_value(serde_json::json!({"levels": [
                {"id": "zemin", "name": "Zemin kat", "elevation": 0.0},
                {"id": "kat-1", "name": "1. kat", "elevation": 3.25}
            ]}))
            .unwrap(),
        );
        doc.set_settings(s);
    }
    let kot = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            za: Some(2.0),
            offset: 0.0,
            ..dimension(Elevation, [0.0, 0.0], [2.0, 2.0])
        }),
    );
    select(&mut app, &[kot]);
    assert_eq!(value(&app, "Geometri", "Kot"), "2.000 m");
    assert!(row_editable(&app, "Geometri", "Kot"));
    assert_eq!(value(&app, "Geometri", "Kat"), "Yok (kendi kotu)");
    assert!(!has_row(&app, "Geometri", "Fark"));
    // Its own value typed in centimetres.
    event(&mut app, Event::Commit(Field::DimensionZa(vec![Slot(kot)]), "250 cm".into()));
    assert_eq!(dimension_of(&app, kot).za, Some(2.5));
    // Kat: 1. kat; the value is the level's, read only.
    event(
        &mut app,
        Event::DimensionEdit(
            vec![Slot(kot)],
            DimensionOp::Level(Some(kentos_commands::DimensionLevel { id: "kat-1".into(), offset: None })),
        ),
    );
    assert_eq!(dimension_of(&app, kot).za, Some(3.25));
    assert_eq!(value(&app, "Geometri", "Kat"), "1. kat");
    assert_eq!(value(&app, "Geometri", "Kot"), "3.250 m");
    assert!(!row_editable(&app, "Geometri", "Kot"), "the level gives it");
    assert_eq!(value(&app, "Geometri", "Fark"), "0.000 m");
    // Fark −2 cm: a wet room's floor.
    event(&mut app, Event::Commit(Field::DimensionLevelOffset(vec![Slot(kot)]), "-2 cm".into()));
    assert_eq!(dimension_of(&app, kot).za, Some(3.25 + -0.02));
    assert_eq!(value(&app, "Geometri", "Fark"), "-0.020 m");
    // The level raised in the project's settings: the Kot follows.
    {
        let doc = &mut app.document.as_mut().expect("open").model;
        let mut s = doc.settings().clone();
        s.arch.as_mut().unwrap().levels[1].elevation = 6.5;
        doc.set_settings(s);
    }
    assert_eq!(value(&app, "Geometri", "Kot"), "6.480 m");
    assert_eq!(undo_label(&mut app).as_deref(), Some("Kotun katı"));
    assert_eq!(dimension_of(&app, kot).level.as_deref().and_then(|l| l.offset), None);
    // Yok: free again, its value kept as it is.
    event(&mut app, Event::DimensionEdit(vec![Slot(kot)], DimensionOp::Level(None)));
    let d = dimension_of(&app, kot);
    assert_eq!((d.za, d.level.is_none()), (Some(6.5), true));
    assert_eq!(value(&app, "Geometri", "Kat"), "Yok (kendi kotu)");
}

#[test]
fn object_isolation_updates_drawing_picking_and_output_without_layer_or_history_changes() {
    use serde_json::json;
    let mut app = objects();
    crate::files_testing::make_cad(&mut app);
    let model = &mut app.document.as_mut().unwrap().model;
    let all: Vec<_> = model.entities().map(|e| Slot(e.base().id)).collect();
    let before = model.to_snapshot_v2();
    let undo = model.undo_labels();
    app.selection.set([Slot(1)]);
    app.object_isolation(false);
    app.follow_document();
    let model = &mut app.document.as_mut().unwrap().model;
    assert!(model.objects_isolated());
    assert_eq!(model.to_snapshot_v2(), before);
    assert_eq!(model.undo_labels(), undo);
    for slot in &all {
        assert_eq!(app.spatial.store().view_shown(f64::from(slot.0)), *slot == Slot(1), "{slot:?}");
    }
    let (_, looks) = app.spatial.level_looks();
    assert_eq!(looks.hidden.len(), all.len()-1, "the styled scene uses the same hidden objects");
    let q = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(model), "cad.entities.select",
        kentos_commands::CallOp::Execute, json!({"region":{"kind":"all"}})).unwrap();
    assert_eq!(q["output"]["uids"].as_array().unwrap().len(), 1);
    let mut output = kentos_interaction::Spatial::new();
    output.ignore_object_isolation();
    output.reload(model);
    assert!(output.level_looks().1.hidden.is_empty(), "temporary editor state must not reach plotted labels or geometry");
    assert!(all.iter().all(|s| output.store().view_shown(f64::from(s.0))));
    app.object_isolation(true);
    app.follow_document();
    assert!(app.spatial.level_looks().1.hidden.is_empty());
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before);
    assert!(all.iter().all(|s| app.spatial.store().view_shown(f64::from(s.0))));
}

#[test]
fn single_cad_text_height_uses_the_same_physical_units_and_atomic_validation_as_bulk() {
    let mut app = objects();
    crate::files_testing::make_cad(&mut app);
    let slot = scoped_text(&mut app, "cizim", 0.5);
    app.selection.set([slot]);
    for (input, metres) in [("77 cm",0.77),("250 mm",0.25),("1 ft",0.3048)] {
        event(&mut app, Event::Commit(Field::TextHeight(slot), input.into()));
        let Entity::Text(t) = app.document.as_ref().unwrap().model.get(slot).unwrap() else { panic!("text"); };
        assert!((t.height-metres).abs()<1e-12,"{input}");
    }
    let before=app.document.as_ref().unwrap().model.to_snapshot_v2();
    for bad in ["", "0", "-1 cm", "77 garbage", "NaN"] {
        event(&mut app, Event::Commit(Field::TextHeight(slot), bad.into()));
        assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before,"{bad}");
    }
}

/// docs/adr/1027: past the limit (AutoCAD's PROPOBJLIMIT) Öznitelikler shows the selection's size and kinds and one
/// note, no values and no editors; at the limit it shows the values as before.
#[test]
fn a_selection_past_the_limit_shows_its_size_and_no_editors() {
    let mut app = crate::files_testing::app_with_drawing();
    let mut o = crate::tools_scenes::Objects::new();
    for k in 0..(super::super::PROPERTIES_LIMIT as u32 + 1) {
        let (x, y) = (f64::from(k % 300), f64::from(k / 300));
        o.line("cizim", [x, y], [x + 0.5, y]);
    }
    crate::tools_scenes::open(&mut app, o);
    let all: Vec<u32> = (1..=super::super::PROPERTIES_LIMIT as u32 + 1).collect();
    select(&mut app, &all);
    let sections = rows(&app);
    assert_eq!(sections.len(), 1, "{:?}", sections.iter().map(|(t, _)| t).collect::<Vec<_>>());
    assert_eq!(sections[0].0, "Büyük seçim");
    assert!(!editable(&app), "no editors past the limit");
    let doc = app.document.as_ref().expect("open");
    let panel = app.properties_panel(doc);
    assert_eq!(
        (panel.total, panel.editable),
        (
            super::super::PROPERTIES_LIMIT + 1,
            super::super::PROPERTIES_LIMIT + 1
        ),
        "none locked"
    );
    // At the limit: the values and their editors, as before.
    select(&mut app, &all[..super::super::PROPERTIES_LIMIT]);
    assert!(editable(&app), "values and editors at the limit");
    assert!(rows(&app).iter().all(|(t, _)| t != "Büyük seçim"));
}

/// A building element's own section comes first, named for it, its main rows at its top (docs/adr/8005, plan S18): a
/// wall's Kalınlık was under the eight rows every object has and its own joins.
#[test]
fn a_walls_own_section_comes_first_with_its_main_rows_on_top() {
    let (mut app, wall, _) = wall_dimension_set();
    app.selection.set([wall]);
    let sections = rows(&app);
    let titles: Vec<&str> = sections.iter().map(|(t, _)| t.as_str()).collect();
    assert_eq!(titles[0], "Duvar", "{titles:?}");
    assert!(titles.iter().position(|t| *t == "Genel").expect("Genel") > 0, "{titles:?}");
    let labels: Vec<&str> = sections[0].1.iter().filter(|r| !r.note).map(|r| r.label.as_ref()).collect();
    assert_eq!(labels[0], "Kalınlık", "{labels:?}");
    let at = |l: &str| labels.iter().position(|x| *x == l).unwrap_or_else(|| panic!("{l} in {labels:?}"));
    assert!(at("Stil") < at("Sabit kalan çizgi"), "{labels:?}");
    assert!(at("Hiza") < at("Öteleme"), "{labels:?}");
    // An object that is no building element keeps Genel first.
    let line = sections.len();
    assert!(line > 1);
}

/// A wall without a height of its own (docs/adr/8040 §1): its Yükseklik says where its height comes from and the
/// number: its storey's, else under the slab over it; a slab of the storey above drawn, 15 cm thick, it is 2.850.
#[test]
fn a_wall_s_height_row_says_it_stops_under_the_slab_over_it() {
    use kentos_commands::Workspace;
    use serde_json::json;
    let mut app = objects();
    let doc = &mut app.document.as_mut().unwrap().model;
    let mut settings = doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.arch = Some(
        serde_json::from_value(json!({"levels": [
            {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
            {"id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0}
        ]}))
        .unwrap(),
    );
    doc.set_settings(settings);
    let wall = add(
        &mut app,
        serde_json::from_value(json!({
            "kind":"wall", "id":0, "layerId":"parsel", "attrs":{},
            "pts":[{"x":0.,"y":2.},{"x":6.,"y":2.}], "width":0.2, "arch":{"level":"zemin"}
        }))
        .unwrap(),
    );
    select(&mut app, &[wall]);
    // The number in its cell, where it comes from in the note under it (docs/adr/0687).
    let height = |app: &App| noted(app, "Yükseklik");
    assert_eq!(
        height(&app),
        ("3.000".into(), Some("Katın yüksekliği".into()))
    );
    add(
        &mut app,
        serde_json::from_value(json!({
            "kind":"slab", "id":0, "layerId":"parsel", "attrs":{},
            "parts":[{"outer":{"pts":[{"x":-1.,"y":-1.},{"x":8.,"y":-1.},{"x":8.,"y":5.},{"x":-1.,"y":5.}]}}],
            "thickness":0.15, "arch":{"level":"k1"}
        }))
        .unwrap(),
    );
    assert_eq!(
        height(&app),
        ("2.850".into(), Some("Üst döşemenin altına kadar".into()))
    );
}

/// A row's value and the note under it, if one follows.
pub(super) fn noted(app: &App, label: &str) -> (String, Option<String>) {
    let all: Vec<_> = rows(app).into_iter().flat_map(|(_, rows)| rows).collect();
    let k = all
        .iter()
        .position(|r| r.label == label)
        .unwrap_or_else(|| panic!("{label}"));
    (
        all[k].value.to_string(),
        all.get(k + 1)
            .filter(|r| r.note)
            .map(|r| r.value.to_string()),
    )
}

/// A room's floor, asma tavan and inside height (docs/adr/8040 §3): under the 1. kat's Ara kat slab the ground storey's
/// room is 2.76 to the slab's plaster; 2 cm lower, 2.78; an asma tavan at 2.50 is its inside height, and what is typed
/// as its inside height then moves the asma tavan; taken away, the slab again.
#[test]
fn a_room_s_floor_asma_tavan_and_inside_height_are_its_rows() {
    use kentos_commands::Workspace;
    use serde_json::json;
    let mut app = objects();
    let doc = &mut app.document.as_mut().unwrap().model;
    let mut settings = doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.arch = Some(
        serde_json::from_value(json!({"levels": [
            {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
            {"id": "k1", "name": "1. kat", "elevation": 3.0, "height": 3.0}
        ]}))
        .unwrap(),
    );
    doc.set_settings(settings);
    add(
        &mut app,
        serde_json::from_value(json!({
            "kind":"slab", "id":0, "layerId":"parsel", "attrs":{},
            "parts":[{"outer":{"pts":[{"x":-1.,"y":-1.},{"x":8.,"y":-1.},{"x":8.,"y":6.},{"x":-1.,"y":6.}]}}],
            "thickness":0.15, "arch":{"level":"k1"},
            "layers": {"over": [{"kind": "finish", "thickness": 0.02}, {"kind": "screed", "thickness": 0.05}],
                       "under": [{"kind": "plaster", "thickness": 0.02}]}
        }))
        .unwrap(),
    );
    let room = add(
        &mut app,
        serde_json::from_value(json!({
            "kind":"space", "id":0, "layerId":"parsel", "attrs":{},
            "parts":[{"outer":{"pts":[{"x":0.,"y":0.},{"x":5.,"y":0.},{"x":5.,"y":4.},{"x":0.,"y":4.}]}}],
            "name":"Salon", "arch":{"level":"zemin"}
        }))
        .unwrap(),
    );
    select(&mut app, &[room]);
    let space = |app: &App| match entity(app, room) {
        Entity::Space(s) => s.space,
        other => panic!("{other:?}"),
    };
    assert_eq!(noted(&app, "Döşeme kot farkı"), ("0.000".into(), None));
    assert_eq!(noted(&app, "Döşeme kotu").0, "0.000");
    assert_eq!(noted(&app, "Asma tavan"), ("Yok".into(), None));
    assert_eq!(
        noted(&app, "İç yükseklik"),
        ("2.760".into(), Some("Üst döşemenin altına kadar".into()))
    );
    assert_eq!(noted(&app, "Hacim").0, "55.20");
    // A wet room's floor 2 cm lower: 2.78 to the same ceiling.
    event(
        &mut app,
        Event::Commit(Field::SpaceFloorOffset(Slot(room)), "-2 cm".into()),
    );
    assert_eq!(space(&app).floor_offset, Some(-0.02));
    assert_eq!(noted(&app, "Döşeme kotu").0, "-0.020");
    assert_eq!(noted(&app, "İç yükseklik").0, "2.780");
    // An asma tavan at 2.50 over the floor: the inside height.
    event(
        &mut app,
        Event::Commit(Field::SpaceSuspended(Slot(room)), "2.5".into()),
    );
    assert_eq!(
        noted(&app, "İç yükseklik"),
        ("2.500".into(), Some("Asma tavana kadar".into()))
    );
    // Its inside height typed moves the asma tavan, the height stays its own.
    event(
        &mut app,
        Event::Commit(Field::SpaceHeight(Slot(room)), "2.6".into()),
    );
    let s = space(&app);
    assert_eq!((s.suspended_ceiling, s.height), (Some(2.6), None));
    // Yok takes the asma tavan away; an empty floor offset gives the storey's floor again.
    event(
        &mut app,
        Event::Commit(Field::SpaceSuspended(Slot(room)), "Yok".into()),
    );
    event(
        &mut app,
        Event::Commit(Field::SpaceFloorOffset(Slot(room)), String::new()),
    );
    let s = space(&app);
    assert_eq!((s.suspended_ceiling, s.floor_offset), (None, None));
    assert_eq!(noted(&app, "İç yükseklik").0, "2.760");
}

/// A slab's layers (docs/adr/8040 §2): Katmanlar picks a build-up whole (Ara kat döşemesi: kaplama 2, şap 5 over the
/// concrete, sıva 2 under), each layer's thickness is typed in its row, 0 takes it away; Toplam kalınlık adds them.
#[test]
fn a_slab_s_layers_are_picked_whole_and_typed_one_by_one() {
    use kentos_commands::{SlabLayerKind, Workspace};
    use serde_json::json;
    let mut app = objects();
    let doc = &mut app.document.as_mut().unwrap().model;
    let mut settings = doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    doc.set_settings(settings);
    let slab = add(
        &mut app,
        serde_json::from_value(json!({
            "kind":"slab", "id":0, "layerId":"parsel", "attrs":{},
            "parts":[{"outer":{"pts":[{"x":0.,"y":0.},{"x":8.,"y":0.},{"x":8.,"y":5.},{"x":0.,"y":5.}]}}],
            "thickness":0.15
        }))
        .unwrap(),
    );
    select(&mut app, &[slab]);
    let value = |app: &App, label: &str| -> Option<String> {
        rows(app)
            .into_iter()
            .flat_map(|(_, rows)| rows)
            .find(|r| r.label == label)
            .map(|r| r.value.to_string())
    };
    let layers = |app: &App| match entity(app, slab) {
        Entity::Slab(s) => s.slab.layers,
        other => panic!("{other:?}"),
    };
    assert_eq!(value(&app, "Katmanlar").as_deref(), Some("Yalnız taşıyıcı"));
    assert_eq!(value(&app, "Toplam kalınlık"), None);
    // Ara kat döşemesi picked: its message, one step.
    let pick = rows(&app)
        .into_iter()
        .flat_map(|(_, rows)| rows)
        .find(|r| r.label == "Katmanlar")
        .and_then(|r| match r.editor {
            Some(Editor::Select { items, .. }) => items.into_iter().find_map(|c| match c {
                Choice::Pick { label, message, .. } if label == "Ara kat döşemesi" => {
                    Some(message)
                }
                _ => None,
            }),
            _ => None,
        })
        .expect("Ara kat döşemesi");
    let _ = app.update(pick);
    let l = layers(&app);
    assert_eq!(
        l.over
            .iter()
            .map(|x| (x.kind, x.thickness))
            .collect::<Vec<_>>(),
        [(SlabLayerKind::Finish, 0.02), (SlabLayerKind::Screed, 0.05)]
    );
    assert_eq!(l.under.len(), 1);
    assert_eq!(
        value(&app, "Katmanlar").as_deref(),
        Some("Ara kat döşemesi")
    );
    assert_eq!(value(&app, "Şap").as_deref(), Some("0.050"));
    assert_eq!(value(&app, "Toplam kalınlık").as_deref(), Some("0.240"));
    // The screed typed 4 cm: no build-up's name, each layer said.
    event(
        &mut app,
        Event::Commit(Field::SlabLayer(Slot(slab), true, 1), "4 cm".into()),
    );
    assert_eq!(layers(&app).over[1].thickness, 0.04);
    assert_eq!(
        value(&app, "Katmanlar").as_deref(),
        Some("Kaplama 0.020 · Şap 0.040 · Taşıyıcı 0.150 · Sıva 0.020")
    );
    // The finish typed 0: taken away.
    event(
        &mut app,
        Event::Commit(Field::SlabLayer(Slot(slab), true, 0), "0".into()),
    );
    assert_eq!(
        layers(&app).over.iter().map(|x| x.kind).collect::<Vec<_>>(),
        [SlabLayerKind::Screed]
    );
}
