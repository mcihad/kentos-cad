//! Öznitelikler's tests: inserts, what the commands refuse, locked layers, several objects, numbers as
//! typed, explicit units, niches, flues, doors and an opening's phase.

use super::*;

/// The last line said in the command history, when it is a warning.
/// A block made of `slot`'s object from (E, N): with `replace`, the insert that took its place.
pub(super) fn block_of(
    app: &mut App,
    slot: u32,
    name: &str,
    replace: bool,
) -> (kentos_commands::BlockId, Option<u32>) {
    let model = &mut app.document.as_mut().expect("open").model;
    let uid = model.uid(Slot(slot)).expect("an object").to_string();
    let input = kentos_commands::BlocksDefine {
        name: name.to_owned(),
        base: Wire { x: E, y: N },
        uids: vec![uid],
        description: None,
        units: None,
        replace: replace.then_some(true),
        layer_id: replace.then(|| "cizim".to_owned()),
        expected_revision: None,
    };
    let (out, _) = kentos_interaction::block_define::define(model, input).expect("defined");
    (out.block, out.id)
}

/// An insert's rows take its block, place, scale (above zero), turn (degrees,
/// within a turn) and mirroring, each a step of its own (docs/adr/0144 §6).
#[test]
fn an_insert_takes_its_block_place_scale_turn_and_mirroring() {
    let mut app = objects();
    let (_, insert) = block_of(&mut app, 1, "Direk", true);
    let (rogar, _) = block_of(&mut app, 2, "Rögar", false);
    let slot = insert.expect("an insert");
    let s = Slot(slot);
    select(&mut app, &[slot]);
    assert!(editable(&app));
    for (field, typed) in [
        (Field::InsertX(s), "487010,5"),
        (Field::InsertY(s), "4420020"),
        (Field::InsertScale(s), "0"),
        (Field::InsertScale(s), "2"),
        (Field::InsertTurn(s), "-90"),
    ] {
        event(&mut app, Event::Commit(field, typed.to_owned()));
    }
    event(&mut app, Event::InsertMirror(s, true));
    event(&mut app, Event::InsertBlock(s, rogar));
    let Entity::Insert(i) = entity(&app, slot) else {
        panic!("an insert");
    };
    let quarter = std::f64::consts::FRAC_PI_2;
    assert_eq!(
        (i.p.x, i.p.y, i.scale, i.rotation, i.mirror, i.block),
        (487010.5, 4420020.0, 2.0, 3.0 * quarter, true, rogar)
    );
    assert_eq!(value(&app, "Geometri", "Blok"), "Rögar");
    assert_eq!(value(&app, "Geometri", "Dönüş"), "270.0000 °");
    assert_eq!(value(&app, "Geometri", "Aynalı"), "Evet");
    // Each is its own step: the block goes back first.
    assert!(undo_label(&mut app).is_some());
    let Entity::Insert(i) = entity(&app, slot) else {
        panic!("an insert");
    };
    assert_eq!(i.block, block_of_name(&app, "Direk"));
}

pub(super) fn block_of_name(app: &App, name: &str) -> kentos_commands::BlockId {
    let model = &app.document.as_ref().expect("open").model;
    model
        .blocks()
        .iter()
        .find(|b| b.name == name)
        .expect("the block")
        .id
}

pub(super) fn warned(app: &App) -> Option<String> {
    app.log
        .last()
        .filter(|l| l.level == kentos_interaction::Level::Warn)
        .map(|l| l.text.clone())
}

/// The panel writes through the product commands (docs/adr/0066): what they
/// refuse is said and nothing is written.
#[test]
fn what_the_commands_refuse_is_said_and_not_written() {
    let mut app = objects();
    select(&mut app, &[4]);
    // Its layer locked while the panel still offers the row: `layer_locked`.
    let layer = entity(&app, 4).base().layer_id.clone();
    let name = {
        let doc = app.document.as_mut().expect("open");
        doc.model.toggle_layer_locked(&layer);
        doc.model
            .layers()
            .get(&layer)
            .expect("its layer")
            .name
            .clone()
    };
    let before = entity(&app, 4);
    event(
        &mut app,
        Event::Commit(Field::Attribute(Slot(4), "Parsel".into()), "9".into()),
    );
    assert_eq!(entity(&app, 4), before);
    assert_eq!(
        warned(&app).as_deref(),
        Some(
            format!(
                "“{name}” katmanı kilitli; üzerindeki nesne düzenlenemez. Kilidi Katmanlar panelinden açın."
            )
            .as_str()
        )
    );
    // A text of U+0085 alone, which JavaScript's trim leaves: `empty_text`.
    let text = add(
        &mut app,
        Entity::Text(TextEntity {
            annotative: None,
            base: base("cizim"),
            p: Wire { x: E, y: N },
            text: "Park".to_owned(),
            height: 2.5,
            rotation: 0.0,
            align: None,
            width_factor: None,
            mask: false,
            label_of: None,
            label_scale: None,
            paragraph: Default::default(),
            face: Default::default(),
            path: None,
        }),
    );
    event(
        &mut app,
        Event::Commit(Field::Text(Slot(text)), "\u{85}".into()),
    );
    let Entity::Text(t) = entity(&app, text) else {
        panic!("a text");
    };
    assert_eq!(t.text, "Park");
    assert_eq!(
        warned(&app).as_deref(),
        Some(
            "Yazının metni boş olamaz; yalnız boşluktan oluşan metin de boştur. Yazıya bir metin verin."
        )
    );
}

#[test]
fn an_object_on_a_locked_layer_is_named_not_edited() {
    let mut app = objects();
    select(&mut app, &[6]);
    assert_eq!(value(&app, "Genel", "Katman"), "Kilitli katman (kilitli)");
    assert_eq!(value(&app, "Genel", "Renk"), "Katmana göre");
    assert!(!editable(&app));
}

#[test]
fn several_objects_what_they_share_and_their_totals() {
    let mut app = objects();
    click(&mut app, -18.0, 9.0);
    let _ = app.update(Message::Modifiers(Modifiers::SHIFT));
    click(&mut app, -16.0, -12.25);
    click(&mut app, 10.0, -16.25);
    let _ = app.update(Message::Modifiers(Modifiers::empty()));
    let doc = app.document.as_ref().expect("open");
    let panel = app.properties_panel(doc);
    assert_eq!(panel.meta.as_deref(), Some("3 nesne"));
    let Summary::Many { count, kinds } = panel.summary else {
        panic!("several");
    };
    assert_eq!((count, kinds.as_str()), (3, "1 kapalı alan, 2 çizgi"));
    // A line on the locked layer: the others stay editable, the note says it is skipped (docs/adr/0286).
    assert_eq!(value(&app, "Ortak özellikler", "Katman"), "Çeşitli");
    assert_eq!(value(&app, "Ortak özellikler", "Renk"), "Katmana göre");
    assert!(editable(&app));
    let notes: Vec<String> = rows(&app)
        .into_iter()
        .flat_map(|(_, rows)| rows)
        .filter(|r| r.note && r.warn)
        .map(|r| r.value)
        .collect();
    assert_eq!(notes, ["1 nesne kilitli katmanda: atlanır."]);
    // Lines of 16 and 12 m; the parcel's 120 m².
    assert_eq!(value(&app, "Toplamlar", "Toplam uzunluk"), "28.000 m");
    assert_eq!(value(&app, "Toplamlar", "Toplam alan"), "120.00 m²");
}

#[test]
fn a_section_closes_and_stays_closed() {
    let mut app = objects();
    event(&mut app, Event::Toggle("geometry"));
    assert!(app.props_closed.contains("geometry"));
    event(&mut app, Event::Toggle("geometry"));
    assert!(app.props_closed.is_empty());
}

#[test]
fn numbers_are_read_as_parsefloat_reads_them() {
    let same = |text: &str, want: f64| {
        let have = web_number(text);
        assert!(have == want, "{text}: {have}");
    };
    same("12abc", 12.0);
    same("3,5", 3.5);
    same(" -1e3x", -1000.0);
    same(".5", 0.5);
    same("5.", 5.0);
    same("1,2,3", 1.2);
    same("1e", 1.0);
    same("1e+", 1.0);
    same("+7", 7.0);
    same("Infinity", f64::INFINITY);
    for nothing in ["abc", "", "-", ".", "e5"] {
        assert!(web_number(nothing).is_nan(), "{nothing}");
    }
}

#[test]
fn cad_room_height_respects_explicit_units_and_invalid_suffixes_are_atomic() {
    use kentos_commands::{DrawingUnit, SpaceEntity, SpaceFields, SpacePart, SpaceRing, Workspace};
    let setup = |unit| {
        let mut app = objects();
        let doc = &mut app.document.as_mut().unwrap().model;
        let mut settings = doc.settings().clone();
        settings.workspace = Some(Workspace::Cad);
        settings.srid = 0;
        settings.drawing_unit = Some(unit);
        doc.set_settings(settings);
        let id = add(&mut app, Entity::Space(SpaceEntity {
            base: base("parsel"),
            space: SpaceFields { parts: vec![SpacePart {
                outer: SpaceRing {pts:vec![Wire{x:0.,y:0.},Wire{x:4.,y:0.},Wire{x:4.,y:3.},Wire{x:0.,y:3.}],..Default::default()},
                ..Default::default()
            }], ..Default::default() },
        }));
        (app, id)
    };
    for unit in [DrawingUnit::Mm, DrawingUnit::Cm, DrawingUnit::M, DrawingUnit::In, DrawingUnit::Ft] {
        for (text, expected) in [("3 m",3.),("300 cm",3.),("3000 mm",3.),("3,5 m",3.5),("12 in",0.3048),("1 ft",0.3048),("5'6\"",1.6764)] {
            let (mut app, id) = setup(unit);
            let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
            event(&mut app, Event::Commit(Field::SpaceHeight(Slot(id)), text.into()));
            let Entity::Space(s) = entity(&app,id) else {panic!()};
            assert!((s.space.height.unwrap()-expected).abs()<1e-10,"{unit:?}/{text}: {:?}",s.space.height);
            assert_eq!(undo_label(&mut app).as_deref(),Some("Mahali düzenle"));
            assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before);
        }
    }
    let (mut app,id)=setup(DrawingUnit::Mm);
    event(&mut app,Event::Commit(Field::SpaceHeight(Slot(id)),"3000".into()));
    let Entity::Space(s)=entity(&app,id)else{panic!()};
    assert_eq!(s.space.height,Some(3.));
    let before=app.document.as_ref().unwrap().model.to_snapshot_v2();
    for text in ["3 m junk","3 apples","1,2,3","Infinity","-1 m","0 m"] {
        event(&mut app,Event::Commit(Field::SpaceHeight(Slot(id)),text.into()));
        assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before,"{text} changed height");
    }
    event(&mut app,Event::Commit(Field::SpaceHeight(Slot(id)),String::new()));
    let Entity::Space(s)=entity(&app,id)else{panic!()};
    assert_eq!(s.space.height,None);
    app.document.as_mut().unwrap().model.undo().unwrap();
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before);
}

// ── docs/adr/0142: the elevation rows ───────────────────────────────────────

#[test]
fn cad_wall_and_opening_lengths_use_explicit_units_and_one_undo() {
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
        let wall = add(&mut app, serde_json::from_value(json!({
            "kind":"wall", "id":0, "layerId":"parsel", "attrs":{},
            "pts":[{"x":0.,"y":0.},{"x":20.,"y":0.}], "width":0.2
        })).unwrap());
        let host = app.document.as_ref().unwrap().model.uid(Slot(wall)).unwrap().to_string();
        let opening = add(&mut app, serde_json::from_value(json!({
            "kind":"opening", "id":0, "layerId":"parsel", "attrs":{},
            "host":host, "at":5., "width":0.8, "place":[]
        })).unwrap());
        let w=Slot(wall); let o=Slot(opening);
        for (field, slot, key, cases) in [
            (Field::WallWidth(w),wall,"width",vec![("25 cm",0.25),("0,25 m",0.25),("10 in",0.254)]),
            (Field::WallOffset(w),wall,"offset",vec![("-5 cm",-0.05),("1 cm",0.01),("2 in",0.0508)]),
            (Field::WallHeight(w),wall,"height",vec![("3 m",3.),("300 cm",3.),("10 ft",3.048)]),
            (Field::OpeningWidth(o),opening,"width",vec![("90 cm",0.9),("0,9 m",0.9),("3 ft",0.9144)]),
            (Field::OpeningAt(o),opening,"at",vec![("2 m",2.),("200 cm",2.),("6 ft",1.8288)]),
            (Field::OpeningFrame(o),opening,"frame",vec![("5 cm",0.05),("0,05 m",0.05),("2 in",0.0508)]),
            (Field::OpeningSill(o),opening,"sill",vec![("90 cm",0.9),("0,9 m",0.9),("3 ft",0.9144)]),
            (Field::OpeningHead(o),opening,"head",vec![("210 cm",2.1),("2,1 m",2.1),("7 ft",2.1336)]),
        ] {
            for (text,expected) in cases {
                let before=app.document.as_ref().unwrap().model.to_snapshot_v2();
                event(&mut app,Event::Commit(field.clone(),text.into()));
                let e=serde_json::to_value(entity(&app,slot)).unwrap();
                let got=e[key].as_f64().unwrap_or(f64::NAN);
                assert!((got-expected).abs()<1e-10,"{unit:?}/{key}/{text}: {got}; {:?}",warned(&app));
                assert!(undo_label(&mut app).is_some(),"missing undo: {key}/{text}");
                assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before,"undo: {key}/{text}");
            }
            for text in ["3 apples","3 m junk","1,2,3","Infinity"] {
                let before=app.document.as_ref().unwrap().model.to_snapshot_v2();
                event(&mut app,Event::Commit(field.clone(),text.into()));
                assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before,"invalid {key}/{text}");
                assert!(warned(&app).is_some(),"missing invalid-unit explanation");
            }
        }
        // Bare values still use the drawing's selected unit. A zero offset is
        // valid, and optional heights/frame/sill/head can still be cleared.
        let f=kentos_interaction::Format::of(app.document.as_ref().unwrap().model.settings());
        let bare=match unit {DrawingUnit::Mm=>"250",DrawingUnit::Cm=>"25",DrawingUnit::M=>"0.25",DrawingUnit::In=>"10",DrawingUnit::Ft=>"1"};
        let expected=f.to_metres(bare.parse().unwrap());
        let before=app.document.as_ref().unwrap().model.to_snapshot_v2();
        event(&mut app,Event::Commit(Field::WallWidth(w),bare.into()));
        let Entity::Wall(e)=entity(&app,wall)else{panic!()};
        assert!((e.wall.width-expected).abs()<1e-10);
        assert!(undo_label(&mut app).is_some());
        assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before);
        for (field,slot,key,text) in [
            (Field::WallHeight(w),wall,"height","3 m"),
            (Field::OpeningFrame(o),opening,"frame","5 cm"),
            (Field::OpeningSill(o),opening,"sill","90 cm"),
            (Field::OpeningHead(o),opening,"head","210 cm"),
        ] {
            event(&mut app,Event::Commit(field.clone(),text.into()));
            let set=app.document.as_ref().unwrap().model.to_snapshot_v2();
            event(&mut app,Event::Commit(field,String::new()));
            assert!(serde_json::to_value(entity(&app,slot)).unwrap().get(key).is_none());
            assert!(undo_label(&mut app).is_some());
            assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),set);
            assert!(undo_label(&mut app).is_some());
            assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before);
        }
        event(&mut app,Event::Commit(Field::WallOffset(w),"-5 cm".into()));
        event(&mut app,Event::Commit(Field::WallOffset(w),"0 m".into()));
        let Entity::Wall(e)=entity(&app,wall)else{panic!()};
        assert_eq!(e.wall.offset,0.);
        assert!(undo_label(&mut app).is_some());
        assert!(undo_label(&mut app).is_some());
        assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before);
        for (field,text) in [(Field::WallWidth(w),"-1 m"),(Field::WallHeight(w),"-1 m"),
            (Field::OpeningWidth(o),"-1 m"),(Field::OpeningAt(o),"100 m"),(Field::OpeningFrame(o),"-1 m")] {
            event(&mut app,Event::Commit(field,text.into()));
            assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before,"range: {text}");
        }
        // Angles retain their old degree grammar even in a millimetre drawing.
        event(&mut app,Event::Commit(Field::OpeningSwing(o),"45".into()));
        let Entity::Opening(e)=entity(&app,opening)else{panic!()};
        assert_eq!(e.opening.swing,Some(45.));
        assert!(undo_label(&mut app).is_some());
        assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(),before);
        // GIS keeps its legacy number-prefix interpretation and unit selection.
        let doc=&mut app.document.as_mut().unwrap().model;
        let mut settings=doc.settings().clone();settings.workspace=Some(Workspace::Gis);doc.set_settings(settings);
        let f=kentos_interaction::Format::of(app.document.as_ref().unwrap().model.settings());
        event(&mut app,Event::Commit(Field::WallWidth(w),"25 cm".into()));
        let Entity::Wall(e)=entity(&app,wall)else{panic!()};
        assert_eq!(e.wall.width,f.to_metres(25.));
    }
}

// ── docs/adr/0719: a niche's and a flue's rows ─────────────────────────────

#[test]
fn a_niches_depth_and_a_flues_inset_are_written_from_the_rows_one_step_each() {
    use kentos_commands::Workspace;
    use serde_json::json;
    let mut app = objects();
    // A CAD drawing: its length cells read explicit units (docs/adr/0647).
    let doc = &mut app.document.as_mut().unwrap().model;
    let mut settings = doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    doc.set_settings(settings);
    let wall = add(&mut app, serde_json::from_value(json!({
        "kind":"wall", "id":0, "layerId":"parsel", "attrs":{},
        "pts":[{"x":0.,"y":0.},{"x":20.,"y":0.}], "width":0.3
    })).unwrap());
    let host = app.document.as_ref().unwrap().model.uid(Slot(wall)).unwrap().to_string();
    let niche = add(&mut app, serde_json::from_value(json!({
        "kind":"opening", "id":0, "layerId":"parsel", "attrs":{},
        "host":host, "at":5., "width":0.6, "form":"niche", "depth":0.1, "place":[]
    })).unwrap());
    let flue = add(&mut app, serde_json::from_value(json!({
        "kind":"opening", "id":0, "layerId":"parsel", "attrs":{},
        "host":host, "at":9., "width":0.2, "form":"flue", "depth":0.2, "place":[]
    })).unwrap());
    let fields = |app: &App, slot: u32| match entity(app, slot) {
        Entity::Opening(o) => o.opening,
        _ => panic!("an opening"),
    };
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(&mut app, Event::Commit(Field::OpeningDepth(Slot(niche)), "15 cm".into()));
    assert_eq!(fields(&app, niche).depth, Some(0.15), "{:?} {:?}", warned(&app), fields(&app, niche));
    assert!(undo_label(&mut app).is_some());
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before);
    // Too deep for the wall: refused, said, nothing written.
    event(&mut app, Event::Commit(Field::OpeningDepth(Slot(niche)), "30 cm".into()));
    assert_eq!(fields(&app, niche).depth, Some(0.1));
    assert!(warned(&app).is_some_and(|w| w.contains("Nişin derinliği")), "{:?}", warned(&app));
    event(&mut app, Event::Commit(Field::OpeningInset(Slot(flue)), "3 cm".into()));
    assert_eq!(fields(&app, flue).inset, Some(0.03));
    event(&mut app, Event::Commit(Field::OpeningInset(Slot(flue)), String::new()));
    assert_eq!(fields(&app, flue).inset, None, "empty: centred");
    assert!(undo_label(&mut app).is_some());
    assert!(undo_label(&mut app).is_some());
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before);
}

// ── docs/adr/0777: a special door's rows ─────────────────────────────

#[test]
fn a_revolving_doors_drum_and_a_pocket_doors_lip_are_written_from_the_rows() {
    use kentos_commands::{OpeningOperation, OpeningTypeField, Workspace};
    use serde_json::json;
    let mut app = objects();
    let doc = &mut app.document.as_mut().unwrap().model;
    let mut settings = doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    doc.set_settings(settings);
    let wall = add(&mut app, serde_json::from_value(json!({
        "kind":"wall", "id":0, "layerId":"parsel", "attrs":{},
        "pts":[{"x":0.,"y":0.},{"x":20.,"y":0.}], "width":0.3
    })).unwrap());
    let host = app.document.as_ref().unwrap().model.uid(Slot(wall)).unwrap().to_string();
    let door = add(&mut app, serde_json::from_value(json!({
        "kind":"opening", "id":0, "layerId":"parsel", "attrs":{},
        "host":host, "at":5., "width":2.4, "form":"door", "leaves":4, "operation":"revolving", "place":[]
    })).unwrap());
    let fields = |app: &App| match entity(app, door) {
        Entity::Opening(o) => o.opening,
        _ => panic!("an opening"),
    };
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    event(&mut app, Event::Commit(Field::OpeningBuild(Slot(door), OpeningTypeField::Drum), "180 cm".into()));
    assert_eq!(fields(&app).build.and_then(|b| b.drum), Some(1.8), "{:?}", warned(&app));
    // Wider than its light: refused, said, nothing written.
    event(&mut app, Event::Commit(Field::OpeningBuild(Slot(door), OpeningTypeField::Drum), "3 m".into()));
    assert_eq!(fields(&app).build.and_then(|b| b.drum), Some(1.8));
    assert!(warned(&app).is_some_and(|w| w.contains("tamburu")), "{:?}", warned(&app));
    event(&mut app, Event::Commit(Field::OpeningBuild(Slot(door), OpeningTypeField::Drum), String::new()));
    assert_eq!(fields(&app).build, None, "empty: the light");
    assert!(undo_label(&mut app).is_some());
    assert!(undo_label(&mut app).is_some());
    assert_eq!(app.document.as_ref().unwrap().model.to_snapshot_v2(), before);
    // A pocket door now: its lip.
    let set = kentos_commands::OpeningChange {
        operation: Some(OpeningOperation::Pocket),
        leaves: Some(1),
        width: Some(0.9),
        ..Default::default()
    };
    event(&mut app, Event::Opening(Slot(door), kentos_commands::OpeningsOperation::Set(set)));
    assert_eq!(fields(&app).operation, OpeningOperation::Pocket, "{:?}", warned(&app));
    event(&mut app, Event::Commit(Field::OpeningBuild(Slot(door), OpeningTypeField::Lip), "3 cm".into()));
    assert_eq!(fields(&app).build.and_then(|b| b.lip), Some(0.03), "{:?}", warned(&app));
}

// ── docs/adr/8039: an opening's own phase ─────────────────────────────

/// A door's or window's own phase (docs/adr/8039 §1): Tadilat durumu ▾ says its wall's while it has none (“Duvarınki
/// (Mevcut)”), picked writes its own through `cad.openings.edit` in one step (İptal: its hole closed up, Mimarlar
/// Odası's word), and Duvarınki gives it its wall's again.
#[test]
fn an_openings_renovation_row_writes_its_own_phase() {
    use kentos_commands::{ArchPhase, Workspace};
    use serde_json::json;
    let mut app = objects();
    let doc = &mut app.document.as_mut().unwrap().model;
    let mut settings = doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    doc.set_settings(settings);
    let wall = add(
        &mut app,
        serde_json::from_value(json!({
            "kind":"wall", "id":0, "layerId":"parsel", "attrs":{},
            "pts":[{"x":0.,"y":0.},{"x":20.,"y":0.}], "width":0.3, "arch":{"phase":"existing"}
        }))
        .unwrap(),
    );
    let host = app
        .document
        .as_ref()
        .unwrap()
        .model
        .uid(Slot(wall))
        .unwrap()
        .to_string();
    let window = add(
        &mut app,
        serde_json::from_value(json!({
            "kind":"opening", "id":0, "layerId":"parsel", "attrs":{},
            "host":host, "at":5., "width":1.2, "form":"window", "sill":0.9, "head":2.2, "place":[]
        }))
        .unwrap(),
    );
    select(&mut app, &[window]);
    // The row, its value and its choices' words with what each sends.
    let row = |app: &App| -> (String, Vec<(String, bool, Message)>) {
        let r = rows(app)
            .into_iter()
            .flat_map(|(_, rows)| rows)
            .find(|r| r.label == "Tadilat durumu")
            .expect("Tadilat durumu");
        let Some(Editor::Select { items, .. }) = r.editor else {
            panic!("a choice")
        };
        let picks = items
            .into_iter()
            .filter_map(|c| match c {
                Choice::Pick {
                    label,
                    chosen,
                    message,
                    ..
                } => Some((label, chosen, message)),
                _ => None,
            })
            .collect();
        (r.value.to_string(), picks)
    };
    let phase = |app: &App| match entity(app, window) {
        Entity::Opening(o) => o.opening.phase,
        _ => panic!("an opening"),
    };
    let (shown, picks) = row(&app);
    assert_eq!(shown, "Duvarınki (Mevcut)");
    let words: Vec<&str> = picks.iter().map(|p| p.0.as_str()).collect();
    assert_eq!(
        words,
        ["Duvarınki (Mevcut)", "Mevcut", "İptal", "İlave", "Geçici"]
    );
    assert!(picks[0].1, "its wall's chosen");
    let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
    // İptal: its own phase, one step.
    let _ = app.update(picks[2].2.clone());
    assert_eq!(phase(&app), Some(ArchPhase::Demolish), "{:?}", warned(&app));
    let (shown, picks) = row(&app);
    assert_eq!(shown, "İptal");
    assert!(picks[2].1);
    // Duvarınki: its wall's again.
    let _ = app.update(picks[0].2.clone());
    assert_eq!(phase(&app), None);
    assert_eq!(row(&app).0, "Duvarınki (Mevcut)");
    assert!(undo_label(&mut app).is_some());
    assert!(undo_label(&mut app).is_some());
    assert_eq!(
        app.document.as_ref().unwrap().model.to_snapshot_v2(),
        before
    );
}
