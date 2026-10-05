//! Öznitelikler over the selection traces' drawing (fixtures/interaction/v1/objects.kcad):
//! what it shows for no object, one and several, and what its edits write.
//! Expected values are worked out by hand from the drawing.

use std::collections::BTreeMap;

use iced::keyboard::Modifiers;
use kentos_contracts::{
    DimensionEntity, Entity, EntityBase, HatchEntity, HatchPattern, HatchPatternType, LineEntity,
    TextEntity, Vec2 as Wire,
};
use kentos_domain::Slot;

use super::{Choice, Editor, Event, Field, Row, Spot, Summary, web_number};
use crate::app::{App, Message};
use crate::selecting::tests::{click, objects};

const E: f64 = 487000.0;
const N: f64 = 4420000.0;

fn rows(app: &App) -> Vec<(String, Vec<Row>)> {
    let doc = app.document.as_ref().expect("open");
    app.properties_panel(doc)
        .sections
        .into_iter()
        .map(|s| (s.title.into_owned(), s.rows))
        .collect()
}

/// A row's value by its section and label.
fn value(app: &App, section: &str, label: &str) -> String {
    rows(app)
        .into_iter()
        .find(|(title, _)| title == section)
        .and_then(|(_, rows)| rows.into_iter().find(|r| r.label == label))
        .map(|r| {
            let unit = r.unit.map(|u| format!(" {u}")).unwrap_or_default();
            format!("{}{unit}", r.value)
        })
        .unwrap_or_else(|| panic!("{section} / {label}"))
}

/// Whether any row can be edited.
fn editable(app: &App) -> bool {
    rows(app)
        .iter()
        .any(|(_, rows)| rows.iter().any(|r| r.editor.is_some()))
}

fn select(app: &mut App, slots: &[u32]) {
    app.selection
        .set(slots.iter().map(|s| Slot(*s)).collect::<Vec<_>>());
}

fn event(app: &mut App, event: Event) {
    let _ = app.update(Message::Properties(event));
}

fn entity(app: &App, slot: u32) -> Entity {
    app.document
        .as_ref()
        .and_then(|d| d.model.get(Slot(slot)))
        .cloned()
        .expect("the object")
}

fn undo_label(app: &mut App) -> Option<String> {
    app.document.as_mut().and_then(|d| d.model.undo())
}

fn base(layer: &str) -> EntityBase {
    EntityBase {
        id: 0,
        layer_id: layer.to_owned(),
        color: None,
        attrs: BTreeMap::new(),
        label: None,
        symbol: None,
        line_weight: None,
    }
}

fn add(app: &mut App, e: Entity) -> u32 {
    app.document
        .as_mut()
        .expect("open")
        .model
        .add(e)
        .expect("a slot")
        .0
}

#[test]
fn nothing_selected_shows_the_drawing() {
    let app = objects();
    let doc = app.document.as_ref().expect("open");
    let panel = app.properties_panel(doc);
    assert!(matches!(panel.summary, Summary::Empty));
    assert_eq!(panel.meta, None);
    assert_eq!(value(&app, "Çizim", "Dosya"), "Etkileşim izi: nesneler");
    assert_eq!(value(&app, "Çizim", "SRID"), "EPSG:5256");
    assert_eq!(value(&app, "Çizim", "Çizim ölçeği"), "1:1000");
    assert_eq!(value(&app, "Çizim", "Nesne sayısı"), "7");
    assert_eq!(value(&app, "Çizim", "Etkin katman"), "Çizim");
    assert!(!editable(&app));
}

#[test]
fn one_parcel_its_kind_layer_measures_and_attributes() {
    let mut app = objects();
    click(&mut app, -18.0, 9.0);
    let doc = app.document.as_ref().expect("open");
    let panel = app.properties_panel(doc);
    assert_eq!(panel.meta.as_deref(), Some("#4"));
    let Summary::One { kind, label, layer } = panel.summary else {
        panic!("one object");
    };
    assert_eq!((kind, label), ("Kapalı alan", None));
    assert_eq!(layer, Some(("#E5484D".to_owned(), "Parsel".to_owned())));
    assert_eq!(value(&app, "Genel", "Tür"), "Kapalı alan");
    assert_eq!(value(&app, "Genel", "Renk"), "Katmana göre");
    assert_eq!(value(&app, "Genel", "Sembol"), "Katman stiline göre");
    // 12 × 10 m.
    assert_eq!(value(&app, "Geometri", "Köşe sayısı"), "4");
    assert_eq!(value(&app, "Geometri", "Çevre"), "44.000 m");
    assert_eq!(value(&app, "Geometri", "Alan"), "120.00 m²");
    assert_eq!(value(&app, "Öznitelik bilgileri", "Ada"), "104");
    assert_eq!(value(&app, "Öznitelik bilgileri", "Parsel"), "7");
    // Katman ▾ lists every layer by its path; the locked one cannot be chosen.
    let genel = rows(&app).remove(0).1;
    let Some(Editor::Select { text, items, .. }) = &genel[1].editor else {
        panic!("Katman ▾");
    };
    assert_eq!(text, "Parsel");
    let picks: Vec<(String, bool, bool)> = items
        .iter()
        .filter_map(|c| match c {
            Choice::Pick {
                label,
                chosen,
                enabled,
                ..
            } => Some((label.clone(), *chosen, *enabled)),
            _ => None,
        })
        .collect();
    assert_eq!(
        picks,
        [
            ("Çizim".to_owned(), false, true),
            ("Parsel".to_owned(), true, true),
            ("Nokta".to_owned(), false, true),
            ("Kilitli katman".to_owned(), false, false),
            ("Gizli katman".to_owned(), false, true),
        ]
    );
}

#[test]
fn layer_and_colour_are_one_step_each_and_a_hidden_layer_is_said() {
    let mut app = objects();
    select(&mut app, &[4]);
    event(&mut app, Event::Layer(vec![Slot(4)], "nokta".to_owned()));
    assert_eq!(entity(&app, 4).base().layer_id, "nokta");
    event(
        &mut app,
        Event::Color(vec![Slot(4)], Some("#E5484D".to_owned())),
    );
    assert_eq!(value(&app, "Genel", "Renk"), "Kırmızı");
    assert_eq!(undo_label(&mut app).as_deref(), Some("Renk değiştir"));
    assert_eq!(undo_label(&mut app).as_deref(), Some("Katman değiştir"));
    // The point to the hidden layer: moved, said, still selected.
    select(&mut app, &[5]);
    event(&mut app, Event::Layer(vec![Slot(5)], "gizli".to_owned()));
    assert_eq!(entity(&app, 5).base().layer_id, "gizli");
    assert_eq!(
        warned(&app).as_deref(),
        Some("“Gizli katman” katmanı gizli; taşınan nesneler görünmeyecek.")
    );
    assert_eq!(app.selection.ids(), [Slot(5)]);
}

#[test]
fn a_line_shows_its_own_weight_and_kalinlik_sets_it_or_gives_it_back_to_the_layer() {
    // docs/adr/0139: an imported weight (1.20 mm) is shown, and listed among the ordinary ones.
    let mut app = objects();
    let mut own = base("nokta");
    own.line_weight = Some(1.2);
    let line = add(
        &mut app,
        Entity::Line(LineEntity {
            base: own,
            a: Wire { x: E, y: N },
            b: Wire { x: E + 10.0, y: N },
            za: None,
            zb: None,
        }),
    );
    select(&mut app, &[line]);
    assert_eq!(value(&app, "Genel", "Kalınlık"), "1.20 mm");
    let genel = rows(&app).remove(0).1;
    let Some(Editor::Select { items, .. }) = &genel
        .iter()
        .find(|r| r.label == "Kalınlık")
        .expect("Kalınlık ▾")
        .editor
    else {
        panic!("Kalınlık ▾");
    };
    let picks: Vec<(String, bool)> = items
        .iter()
        .filter_map(|c| match c {
            Choice::Pick { label, chosen, .. } => Some((label.clone(), *chosen)),
            _ => None,
        })
        .collect();
    assert_eq!(picks[0], ("Katmana göre".to_owned(), false));
    assert!(picks.contains(&("1.20 mm".to_owned(), true)), "{picks:?}");
    // Kalınlık ▾ → 0.35 mm, then Katmana göre: each its own step.
    event(&mut app, Event::Weight(vec![Slot(line)], Some(0.35)));
    assert_eq!(entity(&app, line).base().line_weight, Some(0.35));
    assert_eq!(value(&app, "Genel", "Kalınlık"), "0.35 mm");
    event(&mut app, Event::Weight(vec![Slot(line)], None));
    assert_eq!(entity(&app, line).base().line_weight, None);
    assert_eq!(value(&app, "Genel", "Kalınlık"), "Katmana göre");
    assert_eq!(undo_label(&mut app).as_deref(), Some("Kalınlık değiştir"));
    assert_eq!(entity(&app, line).base().line_weight, Some(0.35));
    // A point is not drawn with a line: it shows no weight.
    select(&mut app, &[5]);
    assert!(rows(&app)[0].1.iter().all(|r| r.label != "Kalınlık"));
}

#[test]
fn a_point_is_moved_by_its_typed_coordinates_read_as_the_web_reads_them() {
    let mut app = objects();
    select(&mut app, &[5]);
    event(
        &mut app,
        Event::Commit(Field::PointX(Slot(5)), "487021,5".into()),
    );
    let Entity::Point(p) = entity(&app, 5) else {
        panic!("a point");
    };
    assert_eq!(p.p.x, E + 21.5);
    // What is not a number changes nothing; “12abc” is 12, as parseFloat reads it.
    event(
        &mut app,
        Event::Commit(Field::PointY(Slot(5)), "yok".into()),
    );
    event(
        &mut app,
        Event::Commit(Field::PointY(Slot(5)), "4419990abc".into()),
    );
    let Entity::Point(p) = entity(&app, 5) else {
        panic!("a point");
    };
    assert_eq!(p.p.y, N - 10.0);
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
}

/// A local project's drawing unit (docs/adr/0165 §2): rows read in it, typed
/// values are taken in it (a trailing unit too), the drawing keeps metres.
#[test]
fn a_local_project_reads_and_takes_its_unit() {
    let mut app = objects();
    {
        let doc = app.document.as_mut().expect("open");
        let settings = kentos_contracts::ProjectSettings {
            srid: 0,
            drawing_unit: Some(kentos_contracts::DrawingUnit::Mm),
            ..doc.model.settings().clone()
        };
        doc.model.set_settings(settings);
    }
    select(&mut app, &[5]);
    event(
        &mut app,
        Event::Commit(Field::PointX(Slot(5)), "1500".into()),
    );
    let Entity::Point(p) = entity(&app, 5) else {
        panic!("a point");
    };
    assert_eq!(p.p.x, 1.5);
    assert_eq!(value(&app, "Geometri", "Y (sağa)"), "1500.000 mm");
    let line = line_z(&mut app, Some(0.1), None);
    select(&mut app, &[line]);
    assert_eq!(value(&app, "Geometri", "Kot (başlangıç)"), "100.000 mm");
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(line), Spot::End), "250 mm".into()),
    );
    let Entity::Line(l) = entity(&app, line) else {
        panic!("a line");
    };
    assert_eq!(l.zb, Some(0.25));
}

#[test]
fn an_attribute_is_written_as_typed_and_the_parcel_number_follows() {
    let mut app = objects();
    select(&mut app, &[4]);
    // The drawn number equals the attribute: it follows the new value.
    if let Some(doc) = app.document.as_mut() {
        let mut e = doc.model.get(Slot(4)).cloned().expect("the parcel");
        e.base_mut().label = Some("7".to_owned());
        doc.model.update(Slot(4), e);
    }
    event(
        &mut app,
        Event::Commit(Field::Attribute(Slot(4), "Parsel".into()), " 8".into()),
    );
    let e = entity(&app, 4);
    assert_eq!(e.base().attrs.get("Parsel").map(String::as_str), Some(" 8"));
    assert_eq!(e.base().label.as_deref(), Some(" 8"));
    // An empty value is taken; Ada is not the label.
    event(
        &mut app,
        Event::Commit(Field::Attribute(Slot(4), "Ada".into()), String::new()),
    );
    let e = entity(&app, 4);
    assert_eq!(e.base().attrs.get("Ada").map(String::as_str), Some(""));
    assert_eq!(e.base().label.as_deref(), Some(" 8"));
}

#[test]
fn texts_dimensions_and_hatches_take_what_the_web_takes() {
    let mut app = objects();
    let text = add(
        &mut app,
        Entity::Text(TextEntity {
            base: base("cizim"),
            p: Wire { x: E, y: N },
            text: "Ada".to_owned(),
            height: 2.5,
            rotation: 0.0,
            align: None,
            width_factor: None,
            mask: false,
            label_of: None,
            label_scale: None,
        }),
    );
    let s = Slot(text);
    for (field, typed) in [
        (Field::Text(s), "  Ada 5  "),
        (Field::Text(s), "   "),
        (Field::TextHeight(s), "0"),
        (Field::TextHeight(s), "3,5"),
        (Field::TextAngle(s), "-90"),
    ] {
        event(&mut app, Event::Commit(field, typed.to_owned()));
    }
    let Entity::Text(t) = entity(&app, text) else {
        panic!("a text");
    };
    assert_eq!(
        (t.text.as_str(), t.height, t.rotation),
        ("Ada 5", 3.5, 270.0)
    );

    let dim = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            base: base("cizim"),
            a: Wire { x: E, y: N },
            b: Wire { x: E + 10.0, y: N },
            offset: 2.0,
            height: 2.5,
            text: Some("özel".to_owned()),
            style: None,
            angle: None,
            c: None,
            mask: false,
            za: None,
            zb: None,
        }),
    );
    let s = Slot(dim);
    for (field, typed) in [
        (Field::DimensionText(s), "  "),
        (Field::DimensionHeight(s), "-1"),
        (Field::DimensionOffset(s), "-3"),
    ] {
        event(&mut app, Event::Commit(field, typed.to_owned()));
    }
    let Entity::Dimension(d) = entity(&app, dim) else {
        panic!("a dimension");
    };
    assert_eq!((d.text, d.height, d.offset), (None, 2.5, -3.0));
    select(&mut app, &[dim]);
    assert_eq!(value(&app, "Geometri", "Ölçülen uzunluk"), "10.000 m");
    assert_eq!(value(&app, "Geometri", "Ötelenme"), "-3.000 m");

    let hatch = add(
        &mut app,
        Entity::Hatch(HatchEntity {
            base: base("cizim"),
            ring: vec![
                Wire { x: E, y: N },
                Wire { x: E + 4.0, y: N },
                Wire {
                    x: E + 4.0,
                    y: N + 5.0,
                },
            ],
            holes: None,
            pattern: HatchPattern {
                kind: HatchPatternType::Lines,
                angle: 45.0,
                spacing: 3.0,
            },
        }),
    );
    let s = Slot(hatch);
    event(&mut app, Event::Pattern(s, HatchPatternType::Cross));
    event(&mut app, Event::Commit(Field::HatchSpacing(s), "0".into()));
    event(&mut app, Event::Commit(Field::HatchAngle(s), "30".into()));
    let Entity::Hatch(h) = entity(&app, hatch) else {
        panic!("a hatch");
    };
    assert_eq!(
        (h.pattern.kind, h.pattern.angle, h.pattern.spacing),
        (HatchPatternType::Cross, 30.0, 3.0)
    );
    select(&mut app, &[hatch]);
    assert_eq!(value(&app, "Geometri", "Desen"), "Çapraz");
    assert_eq!(value(&app, "Geometri", "Açı"), "30.00 °");
    assert_eq!(value(&app, "Geometri", "Alan"), "10.00 m²");
}

/// The last line said in the command history, when it is a warning.
/// A block made of `slot`'s object from (E, N): with `replace`, the insert that took its place.
fn block_of(
    app: &mut App,
    slot: u32,
    name: &str,
    replace: bool,
) -> (kentos_contracts::BlockId, Option<u32>) {
    let model = &mut app.document.as_mut().expect("open").model;
    let uid = model.uid(Slot(slot)).expect("an object").to_string();
    let input = kentos_contracts::BlocksDefine {
        name: name.to_owned(),
        base: Wire { x: E, y: N },
        uids: vec![uid],
        description: None,
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

fn block_of_name(app: &App, name: &str) -> kentos_contracts::BlockId {
    let model = &app.document.as_ref().expect("open").model;
    model
        .blocks()
        .iter()
        .find(|b| b.name == name)
        .expect("the block")
        .id
}

fn warned(app: &App) -> Option<String> {
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
    // A line on the locked layer: nothing is edited, the values still named.
    assert_eq!(
        value(&app, "Ortak özellikler", "Katman"),
        "Kilitli katman içeriyor"
    );
    assert_eq!(value(&app, "Ortak özellikler", "Renk"), "Katmana göre");
    assert!(!editable(&app));
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

// ── docs/adr/0142: the elevation rows ───────────────────────────────────────

fn wire(x: f64, y: f64) -> Wire {
    Wire { x: E + x, y: N + y }
}

/// A line on `cizim` from (0, 30) to (40, 30) with the elevations of its ends.
fn line_z(app: &mut App, za: Option<f64>, zb: Option<f64>) -> u32 {
    add(
        app,
        Entity::Line(LineEntity {
            base: base("cizim"),
            a: wire(0.0, 30.0),
            b: wire(40.0, 30.0),
            za,
            zb,
        }),
    )
}

/// An open polyline on `cizim` through three vertices 10 m apart.
fn path_z(app: &mut App, zs: Option<Vec<Option<f64>>>) -> u32 {
    add(
        app,
        Entity::Polyline(kentos_contracts::PathEntity {
            base: base("cizim"),
            pts: vec![wire(0.0, 40.0), wire(10.0, 40.0), wire(20.0, 40.0)],
            bulges: None,
            holes: None,
            zs,
            parts: None,
        }),
    )
}

/// A 20 × 10 m area on `parsel` with a 4 × 2 m hole.
fn area_z(app: &mut App, zs: Option<Vec<Option<f64>>>, hole: Option<Vec<Option<f64>>>) -> u32 {
    add(
        app,
        Entity::Polygon(kentos_contracts::PathEntity {
            base: base("parsel"),
            pts: vec![
                wire(0.0, 50.0),
                wire(20.0, 50.0),
                wire(20.0, 60.0),
                wire(0.0, 60.0),
            ],
            bulges: None,
            holes: Some(vec![kentos_contracts::RingGeometry {
                pts: vec![
                    wire(4.0, 53.0),
                    wire(8.0, 53.0),
                    wire(8.0, 55.0),
                    wire(4.0, 55.0),
                ],
                bulges: None,
                zs: hole,
            }]),
            zs,
            parts: None,
        }),
    )
}

fn some(v: &[f64]) -> Option<Vec<Option<f64>>> {
    Some(v.iter().copied().map(Some).collect())
}

/// Whether a row of the section is there.
fn has_row(app: &App, section: &str, label: &str) -> bool {
    rows(app)
        .into_iter()
        .find(|(title, _)| title == section)
        .is_some_and(|(_, rows)| rows.iter().any(|r| r.label == label))
}

/// The note under a row, when one follows it: a row with no name of its own.
fn note_after(app: &App, section: &str, label: &str) -> Option<String> {
    let (_, rows) = rows(app).into_iter().find(|(title, _)| title == section)?;
    let at = rows.iter().position(|r| r.label == label)?;
    rows.get(at + 1)
        .filter(|r| r.note && r.label.is_empty() && r.editor.is_none())
        .map(|r| r.value.clone())
}

fn row_editable(app: &App, section: &str, label: &str) -> bool {
    rows(app)
        .into_iter()
        .find(|(title, _)| title == section)
        .and_then(|(_, rows)| rows.into_iter().find(|r| r.label == label))
        .is_some_and(|r| r.editor.is_some())
}

/// 40 m in plan rising 9 m is 41 m in space (a 9-40-41 triangle).
#[test]
fn a_line_shows_the_elevation_of_each_end_and_its_length_in_space() {
    let mut app = objects();
    let line = line_z(&mut app, Some(100.0), Some(109.0));
    select(&mut app, &[line]);
    assert_eq!(value(&app, "Geometri", "Kot (başlangıç)"), "100.000 m");
    assert_eq!(value(&app, "Geometri", "Kot (bitiş)"), "109.000 m");
    assert_eq!(value(&app, "Geometri", "Uzunluk"), "40.000 m");
    assert_eq!(value(&app, "Geometri", "3B uzunluk"), "41.000 m");
    // The order of the rows: each end's elevation after its coordinates, then the lengths.
    let labels: Vec<String> = rows(&app)[1]
        .1
        .iter()
        .map(|r| r.label.to_string())
        .collect();
    assert_eq!(
        labels,
        [
            "Başlangıç Y",
            "Başlangıç X",
            "Kot (başlangıç)",
            "Bitiş Y",
            "Bitiş X",
            "Kot (bitiş)",
            "Uzunluk",
            "3B uzunluk",
            "Semt"
        ]
    );
    // An end without one says so, and there is no length in space.
    let half = line_z(&mut app, Some(100.0), None);
    select(&mut app, &[half]);
    assert_eq!(value(&app, "Geometri", "Kot (başlangıç)"), "100.000 m");
    assert_eq!(value(&app, "Geometri", "Kot (bitiş)"), "kot yok");
    assert!(!has_row(&app, "Geometri", "3B uzunluk"));
    let bare = line_z(&mut app, None, None);
    select(&mut app, &[bare]);
    assert_eq!(value(&app, "Geometri", "Kot (başlangıç)"), "kot yok");
    assert!(!has_row(&app, "Geometri", "3B uzunluk"));
}

#[test]
fn a_polyline_shows_its_elevations_as_a_value_a_range_or_none() {
    let mut app = objects();
    let same = path_z(&mut app, some(&[5.0, 5.0, 5.0]));
    let range = path_z(&mut app, some(&[98.5, 101.25, 105.25]));
    let partial = path_z(&mut app, Some(vec![Some(105.25), None, Some(98.5)]));
    let none = path_z(&mut app, None);
    for (slot, text, note) in [
        (same, "5.000 m", None),
        (range, "98.500–105.250 m", None),
        // The range of those that have one; the note is a line under it, the column too narrow for both.
        (partial, "98.500–105.250 m", Some("(bazı köşeler kotsuz)")),
        (none, "kot yok", None),
    ] {
        select(&mut app, &[slot]);
        assert_eq!(value(&app, "Geometri", "Kot"), text, "#{slot}");
        assert_eq!(
            note_after(&app, "Geometri", "Kot").as_deref(),
            note,
            "#{slot}"
        );
    }
    // The elevation of every vertex, the length in space when every one has it.
    select(&mut app, &[range]);
    assert_eq!(value(&app, "Geometri", "Uzunluk"), "20.000 m");
    // 10 m east rising 2.75, then 10 m east rising 4: √(100 + 7.5625) + √(116).
    let want = (100.0f64 + 2.75 * 2.75).sqrt() + (100.0f64 + 16.0).sqrt();
    assert_eq!(
        value(&app, "Geometri", "3B uzunluk"),
        format!("{want:.3} m")
    );
    select(&mut app, &[partial]);
    assert!(!has_row(&app, "Geometri", "3B uzunluk"));
}

#[test]
fn an_area_says_its_perimeter_in_space_holes_included() {
    let mut app = objects();
    // Flat at 7 m: the plan perimeter is 60 m and the hole's 12 m.
    let flat = area_z(&mut app, some(&[7.0; 4]), some(&[7.0; 4]));
    select(&mut app, &[flat]);
    assert_eq!(value(&app, "Geometri", "Kot"), "7.000 m");
    assert_eq!(value(&app, "Geometri", "Çevre"), "72.000 m");
    assert_eq!(value(&app, "Geometri", "3B çevre"), "72.000 m");
    // A rise of 3 m along the two long sides.
    let sloped = area_z(&mut app, some(&[10.0, 13.0, 13.0, 10.0]), some(&[10.0; 4]));
    select(&mut app, &[sloped]);
    assert_eq!(value(&app, "Geometri", "Kot"), "10.000–13.000 m");
    // The ring: two sides of √(20² + 3²) and two flat ones of 10; the hole is flat, 12 m.
    let want = 2.0 * (400.0f64 + 9.0).sqrt() + 20.0 + 12.0;
    assert_eq!(value(&app, "Geometri", "3B çevre"), format!("{want:.3} m"));
    // One vertex of the hole without an elevation: the range of those that have one, and none in space.
    let missing = area_z(&mut app, some(&[7.0; 4]), None);
    select(&mut app, &[missing]);
    assert_eq!(value(&app, "Geometri", "Kot"), "7.000 m");
    assert_eq!(
        note_after(&app, "Geometri", "Kot").as_deref(),
        Some("(bazı köşeler kotsuz)")
    );
    assert!(!has_row(&app, "Geometri", "3B çevre"));
    // A polygon without any: no row of it.
    let bare = area_z(&mut app, None, None);
    select(&mut app, &[bare]);
    assert_eq!(value(&app, "Geometri", "Kot"), "kot yok");
    assert!(!has_row(&app, "Geometri", "3B çevre"));
}

#[test]
fn a_point_keeps_its_row_as_it_was() {
    let mut app = objects();
    let spot = add(
        &mut app,
        Entity::Point(kentos_contracts::PointEntity {
            base: base("nokta"),
            p: wire(1.0, 2.0),
            z: Some(12.5),
            parts: None,
        }),
    );
    select(&mut app, &[spot]);
    assert_eq!(value(&app, "Geometri", "Z (kot)"), "12.500 m");
    assert!(!has_row(&app, "Geometri", "Kot"));
    assert!(!row_editable(&app, "Geometri", "Z (kot)"));
}

/// A number sets the end, an empty text clears it: through Kot ver, one step each.
#[test]
fn a_line_s_cells_set_or_clear_one_end_in_a_step_named_kot_ver() {
    let mut app = objects();
    let line = line_z(&mut app, Some(100.0), Some(109.0));
    select(&mut app, &[line]);
    assert!(row_editable(&app, "Geometri", "Kot (başlangıç)"));
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(line), Spot::End), "112,5".into()),
    );
    let Entity::Line(l) = entity(&app, line) else {
        panic!("a line");
    };
    assert_eq!((l.za, l.zb), (Some(100.0), Some(112.5)));
    assert_eq!(value(&app, "Geometri", "Kot (bitiş)"), "112.500 m");
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(line), Spot::Start), String::new()),
    );
    let Entity::Line(l) = entity(&app, line) else {
        panic!("a line");
    };
    assert_eq!((l.za, l.zb), (None, Some(112.5)));
    assert_eq!(value(&app, "Geometri", "Kot (başlangıç)"), "kot yok");
    // 0 is an elevation, and a negative one too; a trailing m is read as the unit.
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(line), Spot::Start), "-3 m".into()),
    );
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(line), Spot::End), "0".into()),
    );
    let Entity::Line(l) = entity(&app, line) else {
        panic!("a line");
    };
    assert_eq!((l.za, l.zb), (Some(-3.0), Some(0.0)));
    assert_eq!(undo_label(&mut app).as_deref(), Some("Kot ver"));
    assert_eq!(undo_label(&mut app).as_deref(), Some("Kot ver"));
    assert_eq!(undo_label(&mut app).as_deref(), Some("Kot ver"));
    assert_eq!(undo_label(&mut app).as_deref(), Some("Kot ver"));
    let Entity::Line(l) = entity(&app, line) else {
        panic!("a line");
    };
    assert_eq!((l.za, l.zb), (Some(100.0), Some(109.0)));
}

#[test]
fn what_is_not_a_number_or_changes_nothing_writes_nothing() {
    let mut app = objects();
    let line = line_z(&mut app, Some(100.0), Some(109.0));
    let none = path_z(&mut app, None);
    select(&mut app, &[line]);
    let revision = app.document.as_ref().expect("open").model.revision();
    for typed in ["kot yok", "yüz", "NaN", "Infinity", "12abc", "1,2,3"] {
        event(
            &mut app,
            Event::Commit(Field::Elevation(Slot(line), Spot::End), typed.to_owned()),
        );
    }
    // The elevation it has: no step. Clearing what has none: no step either.
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(line), Spot::End), "109".into()),
    );
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(none), Spot::All), "   ".into()),
    );
    assert_eq!(
        app.document.as_ref().expect("open").model.revision(),
        revision
    );
    let Entity::Line(l) = entity(&app, line) else {
        panic!("a line");
    };
    assert_eq!((l.za, l.zb), (Some(100.0), Some(109.0)));
}

#[test]
fn a_polyline_or_area_cell_sets_every_vertex_holes_included_or_clears_them_all() {
    let mut app = objects();
    let path = path_z(&mut app, Some(vec![Some(1.0), None, Some(3.0)]));
    let area = area_z(&mut app, some(&[1.0, 2.0, 3.0, 4.0]), some(&[5.0; 4]));
    select(&mut app, &[path]);
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(path), Spot::All), "100.5".into()),
    );
    let Entity::Polyline(p) = entity(&app, path) else {
        panic!("a polyline");
    };
    assert_eq!(p.zs, some(&[100.5; 3]));
    assert_eq!(value(&app, "Geometri", "Kot"), "100.500 m");
    select(&mut app, &[area]);
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(area), Spot::All), "42".into()),
    );
    let Entity::Polygon(p) = entity(&app, area) else {
        panic!("an area");
    };
    assert_eq!(p.zs, some(&[42.0; 4]));
    assert_eq!(
        p.holes.as_ref().and_then(|h| h[0].zs.clone()),
        some(&[42.0; 4]),
        "the hole's too"
    );
    // Empty: none of them keeps one, and the row says so.
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(area), Spot::All), String::new()),
    );
    let Entity::Polygon(p) = entity(&app, area) else {
        panic!("an area");
    };
    assert_eq!(p.zs, None);
    assert_eq!(p.holes.as_ref().and_then(|h| h[0].zs.clone()), None);
    assert_eq!(value(&app, "Geometri", "Kot"), "kot yok");
    assert_eq!(undo_label(&mut app).as_deref(), Some("Kot ver"));
    assert_eq!(undo_label(&mut app).as_deref(), Some("Kot ver"));
    assert_eq!(undo_label(&mut app).as_deref(), Some("Kot ver"));
}

#[test]
fn an_object_on_a_locked_layer_names_its_elevations_without_a_cell_to_edit() {
    let mut app = objects();
    let line = add(
        &mut app,
        Entity::Line(LineEntity {
            base: base("kilitli"),
            a: wire(0.0, 30.0),
            b: wire(40.0, 30.0),
            za: Some(100.0),
            zb: Some(109.0),
        }),
    );
    select(&mut app, &[line]);
    assert_eq!(value(&app, "Geometri", "Kot (bitiş)"), "109.000 m");
    assert_eq!(value(&app, "Geometri", "3B uzunluk"), "41.000 m");
    assert!(!row_editable(&app, "Geometri", "Kot (başlangıç)"));
    assert!(!row_editable(&app, "Geometri", "Kot (bitiş)"));
}

/// Several objects: a Kot row among what they share, over every vertex of them all as one list
/// (the web's row): the value when every vertex has it, `kot yok` when none has one, else Çeşitli.
#[test]
fn several_objects_show_their_elevation_only_when_every_vertex_has_the_same() {
    let mut app = objects();
    let a = path_z(&mut app, some(&[5.0; 3]));
    let b = path_z(&mut app, some(&[5.0; 3]));
    let c = area_z(&mut app, some(&[5.0; 4]), some(&[5.0; 4]));
    let d = path_z(&mut app, some(&[6.0; 3]));
    let line = line_z(&mut app, Some(5.0), Some(5.0));
    let spot = add(
        &mut app,
        Entity::Point(kentos_contracts::PointEntity {
            base: base("nokta"),
            p: wire(1.0, 2.0),
            z: Some(5.0),
            parts: None,
        }),
    );
    select(&mut app, &[a, b, c, line, spot]);
    assert_eq!(value(&app, "Ortak özellikler", "Kot"), "5.000 m");
    assert!(row_editable(&app, "Ortak özellikler", "Kot"));
    // Another elevation among them: not the same.
    select(&mut app, &[a, b, d]);
    assert_eq!(value(&app, "Ortak özellikler", "Kot"), "Çeşitli");
    // The same range on two objects is a range, not a value: Çeşitli.
    let r1 = path_z(&mut app, some(&[1.0, 2.0, 3.0]));
    let r2 = path_z(&mut app, some(&[1.0, 2.0, 3.0]));
    select(&mut app, &[r1, r2]);
    assert_eq!(value(&app, "Ortak özellikler", "Kot"), "Çeşitli");
    // A vertex without one among vertices that have it: Çeşitli too, and a point without a z is one.
    let bare_spot = add(
        &mut app,
        Entity::Point(kentos_contracts::PointEntity {
            base: base("nokta"),
            p: wire(2.0, 2.0),
            z: None,
            parts: None,
        }),
    );
    select(&mut app, &[a, bare_spot]);
    assert_eq!(value(&app, "Ortak özellikler", "Kot"), "Çeşitli");
    // None of them has one: `kot yok`, and it is the same for all.
    let bare = path_z(&mut app, None);
    let bare2 = path_z(&mut app, None);
    select(&mut app, &[bare, bare2]);
    assert_eq!(value(&app, "Ortak özellikler", "Kot"), "kot yok");
    // Objects that take no elevation add nothing, and with only them there is no row.
    let circle = |app: &mut App, x: f64| {
        add(
            app,
            Entity::Circle(kentos_contracts::CircleEntity {
                base: base("cizim"),
                c: wire(x, 70.0),
                r: 1.0,
            }),
        )
    };
    let (c1, c2) = (circle(&mut app, 0.0), circle(&mut app, 5.0));
    select(&mut app, &[c1, c2]);
    assert!(!has_row(&app, "Ortak özellikler", "Kot"));
    select(&mut app, &[a, b, c1]);
    assert_eq!(value(&app, "Ortak özellikler", "Kot"), "5.000 m");
    // The drawing's own line 2 and point 5 have none.
    select(&mut app, &[5, 2]);
    assert_eq!(value(&app, "Ortak özellikler", "Kot"), "kot yok");
}

#[test]
fn a_cell_of_several_objects_sets_every_vertex_of_each_of_them_in_one_step() {
    let mut app = objects();
    let path = path_z(&mut app, Some(vec![Some(1.0), None, Some(3.0)]));
    let area = area_z(&mut app, some(&[1.0, 2.0, 3.0, 4.0]), None);
    let line = line_z(&mut app, Some(10.0), Some(20.0));
    let spot = add(
        &mut app,
        Entity::Point(kentos_contracts::PointEntity {
            base: base("nokta"),
            p: wire(1.0, 2.0),
            z: None,
            parts: None,
        }),
    );
    select(&mut app, &[path, area, line, spot]);
    assert_eq!(value(&app, "Ortak özellikler", "Kot"), "Çeşitli");
    event(
        &mut app,
        Event::Commit(
            Field::Elevations(vec![Slot(path), Slot(area), Slot(line), Slot(spot)]),
            "250".into(),
        ),
    );
    assert_eq!(value(&app, "Ortak özellikler", "Kot"), "250.000 m");
    let Entity::Line(l) = entity(&app, line) else {
        panic!("a line");
    };
    assert_eq!((l.za, l.zb), (Some(250.0), Some(250.0)));
    let Entity::Point(p) = entity(&app, spot) else {
        panic!("a point");
    };
    assert_eq!(p.z, Some(250.0), "a point takes its z");
    // One step for all three.
    assert_eq!(undo_label(&mut app).as_deref(), Some("Kot ver"));
    let Entity::Line(l) = entity(&app, line) else {
        panic!("a line");
    };
    assert_eq!((l.za, l.zb), (Some(10.0), Some(20.0)));
    let Entity::Polyline(p) = entity(&app, path) else {
        panic!("a polyline");
    };
    assert_eq!(p.zs, Some(vec![Some(1.0), None, Some(3.0)]));
    let Entity::Point(p) = entity(&app, spot) else {
        panic!("a point");
    };
    assert_eq!(p.z, None);
}

#[test]
fn a_locked_layer_among_several_takes_the_row_s_cell_away() {
    let mut app = objects();
    let open = path_z(&mut app, some(&[5.0; 3]));
    let locked = add(
        &mut app,
        Entity::Line(LineEntity {
            base: base("kilitli"),
            a: wire(0.0, 30.0),
            b: wire(40.0, 30.0),
            za: Some(5.0),
            zb: Some(5.0),
        }),
    );
    select(&mut app, &[open, locked]);
    assert_eq!(value(&app, "Ortak özellikler", "Kot"), "5.000 m");
    assert!(!row_editable(&app, "Ortak özellikler", "Kot"));
}

/// Pictures of Öznitelikler for the owner, over the sample drawing: nothing
/// selected, the parcel, several objects, the dimension, the text being
/// edited, Katman ▾ open.
/// Not run by default: `cargo test -p kentos-desktop properties::tests::screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size};
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for (name, picked) in [
                ("bos", &[][..]),
                ("parsel", &[4][..]),
                ("coklu", &[4, 5, 2][..]),
                ("olcu", &[12][..]),
                ("yazi", &[11][..]),
                ("katman", &[4][..]),
            ] {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                select(&mut app, picked);
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let panel = width - app.docks.size(kentos_ui::widget::docking::Side::Right);
                // The panel sits 125 px higher in the smaller window (read off the pictures).
                let lift = if height > 800.0 { 0.0 } else { -125.0 };
                let at = |x: f32, y: f32| Point::new(panel + x, y + lift);
                match name {
                    // Genel closed; the text's Metin cell, edited.
                    "yazi" => {
                        let _ = app.update(Message::Properties(Event::Toggle("general")));
                        snapshot.settle(&mut app, App::view, &mut update);
                        snapshot.input(
                            &mut app,
                            App::view,
                            &mut update,
                            Input::Click(at(250.0, YAZI_Y)),
                        );
                        snapshot.input(&mut app, App::view, &mut update, Input::Type(" 2".into()));
                    }
                    "katman" => snapshot.input(
                        &mut app,
                        App::view,
                        &mut update,
                        Input::Click(at(250.0, KATMAN_Y)),
                    ),
                    _ => {}
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("oznitelik-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

/// Where the text's Metin row (Genel closed) and the parcel's Katman row are
/// in the 1440 × 900 window (read off the pictures).
const YAZI_Y: f32 = 620.0;
const KATMAN_Y: f32 = 618.0;

/// A multi-part area (docs/adr/0143): its corners, holes, perimeter and area
/// are every part's, and a row says how many parts it has. Parcel 4
/// (12 × 10 m) gets a 10 × 10 m part with a 2 × 2 m hole.
#[test]
fn a_multi_part_area_counts_every_part() {
    let mut app = objects();
    click(&mut app, -18.0, 9.0);
    let doc = app.document.as_mut().expect("open");
    let Some(Entity::Polygon(mut p)) = doc.model.get(Slot(4)).cloned() else {
        panic!("the parcel");
    };
    let at = |x: f64, y: f64| Wire { x: E + x, y: N + y };
    p.parts = Some(vec![kentos_contracts::AreaPart {
        pts: vec![at(30.0, 0.0), at(40.0, 0.0), at(40.0, 10.0), at(30.0, 10.0)],
        bulges: None,
        holes: Some(vec![kentos_contracts::RingGeometry {
            pts: vec![at(34.0, 4.0), at(34.0, 6.0), at(36.0, 6.0), at(36.0, 4.0)],
            bulges: None,
            zs: None,
        }]),
        zs: None,
    }]);
    assert!(doc.model.update(Slot(4), Entity::Polygon(p)));
    assert_eq!(value(&app, "Geometri", "Köşe sayısı"), "8");
    assert_eq!(value(&app, "Geometri", "Parça sayısı"), "2");
    assert_eq!(value(&app, "Geometri", "Ada (delik)"), "1");
    // 44 + 40 + 8 m; 120 + 100 − 4 m².
    assert_eq!(value(&app, "Geometri", "Çevre"), "92.000 m");
    assert_eq!(value(&app, "Geometri", "Alan"), "216.00 m²");
}

/// A multi-part polyline counts every part and a multi-point object its
/// points (docs/adr/0174 §6): Köşe sayısı and Uzunluk are every part's, with
/// Parça sayısı; Nokta sayısı and one Kot row for every point.
#[test]
fn multi_part_lines_and_points_count_every_part() {
    let mut app = objects();
    let at = |x: f64, y: f64| Wire { x: E + x, y: N + y };
    let doc = app.document.as_mut().expect("open");
    let Some(Entity::Line(line)) = doc.model.get(Slot(1)).cloned() else {
        panic!("the line");
    };
    let road = Entity::Polyline(kentos_contracts::PathEntity {
        base: line.base.clone(),
        pts: vec![at(-24.0, -12.0), at(-8.0, -12.0)],
        bulges: None,
        holes: None,
        zs: None,
        parts: Some(vec![kentos_contracts::AreaPart {
            pts: vec![at(0.0, -20.0), at(0.0, -10.0), at(5.0, -10.0)],
            bulges: None,
            holes: None,
            zs: None,
        }]),
    });
    assert!(doc.model.update(Slot(1), road));
    let Some(Entity::Point(mut marks)) = doc.model.get(Slot(5)).cloned() else {
        panic!("the point");
    };
    marks.z = Some(100.0);
    marks.parts = Some(vec![
        kentos_contracts::PointPart {
            p: at(25.0, -8.0),
            z: Some(100.0),
        },
        kentos_contracts::PointPart {
            p: at(30.0, -8.0),
            z: Some(102.5),
        },
    ]);
    assert!(doc.model.update(Slot(5), Entity::Point(marks)));
    select(&mut app, &[1]);
    assert_eq!(value(&app, "Geometri", "Köşe sayısı"), "5");
    assert_eq!(value(&app, "Geometri", "Parça sayısı"), "2");
    // 16 + 10 + 5 m.
    assert_eq!(value(&app, "Geometri", "Uzunluk"), "31.000 m");
    select(&mut app, &[5]);
    assert_eq!(value(&app, "Geometri", "Nokta sayısı"), "3");
    assert_eq!(value(&app, "Geometri", "Kot"), "100.000–102.500 m");
    // Typed into Kot, every point takes it.
    event(
        &mut app,
        Event::Commit(Field::Elevation(Slot(5), Spot::All), "99".into()),
    );
    let Entity::Point(p) = entity(&app, 5) else {
        panic!("a point");
    };
    assert_eq!(p.z, Some(99.0));
    assert!(p.parts.iter().flatten().all(|q| q.z == Some(99.0)));
}

/// The KCAD v2 blocks fixture (fixtures/kcad/v2/blocks.kcad): Rögar's attributes
/// are NO (default "R-1") and KOT (no default); insert 1 has both, insert 2 none.
fn blocks_drawing() -> App {
    let (mut app, _) = App::boot(None);
    let path = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/kcad/v2/blocks.kcad"
    );
    let doc = crate::document::Document::read(std::path::Path::new(path)).expect("opens");
    let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
    app
}

/// A section's labels in order.
fn labels(app: &App, section: &str) -> Vec<String> {
    rows(app)
        .into_iter()
        .find(|(title, _)| title == section)
        .map(|(_, rows)| rows.into_iter().map(|r| r.label.into_owned()).collect())
        .unwrap_or_default()
}

/// An insert's block attributes (docs/adr/0144 §7): the definition's tags in its
/// order with what the insert shows, its own value else the default; they are
/// not repeated among its other attributes. A cell writes the insert's value.
#[test]
fn an_insert_shows_its_block_attributes_and_a_cell_writes_its_value() {
    let mut app = blocks_drawing();
    select(&mut app, &[1]);
    assert_eq!(labels(&app, "Blok öznitelikleri"), ["NO", "KOT"]);
    assert_eq!(value(&app, "Blok öznitelikleri", "NO"), "R-12");
    assert_eq!(value(&app, "Blok öznitelikleri", "KOT"), "101.35");
    assert!(labels(&app, "Öznitelik bilgileri").is_empty());
    // Without values: the default, and nothing for KOT.
    select(&mut app, &[2]);
    assert_eq!(value(&app, "Blok öznitelikleri", "NO"), "R-1");
    assert_eq!(value(&app, "Blok öznitelikleri", "KOT"), "");
    event(
        &mut app,
        Event::Commit(Field::Attribute(Slot(2), "NO".into()), "R-7".into()),
    );
    assert_eq!(
        entity(&app, 2).base().attrs.get("NO").map(String::as_str),
        Some("R-7")
    );
    assert_eq!(value(&app, "Blok öznitelikleri", "NO"), "R-7");
    // An empty value gives the default back.
    event(
        &mut app,
        Event::Commit(Field::Attribute(Slot(2), "NO".into()), String::new()),
    );
    assert_eq!(value(&app, "Blok öznitelikleri", "NO"), "R-1");
    // An attribute that is not the block's stays among the others.
    event(
        &mut app,
        Event::Commit(Field::Attribute(Slot(1), "Malzeme".into()), "Beton".into()),
    );
    select(&mut app, &[1]);
    assert_eq!(labels(&app, "Öznitelik bilgileri"), ["Malzeme"]);
    // The lighting pole has none: no section.
    select(&mut app, &[3]);
    assert!(labels(&app, "Blok öznitelikleri").is_empty());
}

/// Pictures of an insert's Blok öznitelikleri (docs/adr/0144 §7) over the
/// block attributes trace's drawing (fixtures/interaction/v1/block-attributes.kcad),
/// Genel and Geometri closed: the west manhole's own values, the middle one's
/// default, its NO cell typed into (the default wiped) and written with Enter.
/// `.run/shots/oznitelik-blok-*`.
/// Not run by default: `cargo test -p kentos-desktop properties::tests::block_attribute_screens -- --ignored --nocapture`.
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn block_attribute_screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    let drawing = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../fixtures/interaction/v1/block-attributes.kcad"
    );
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for (name, slot) in [
                ("kendi", 3),
                ("varsayilan", 4),
                ("yaziliyor", 4),
                ("yazildi", 4),
            ] {
                let (mut app, _) = App::boot(None);
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let doc =
                    crate::document::Document::read(std::path::Path::new(drawing)).expect("opens");
                let _ = app.update(Message::Opened(Some(Ok(Box::new(doc)))));
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                app.viewport.camera.center = kentos_interaction::Vec2::new(487015.0, 4420008.5);
                app.viewport.camera.scale = 1.0 / 0.03;
                select(&mut app, &[slot]);
                let _ = app.update(Message::Properties(Event::Toggle("general")));
                let _ = app.update(Message::Properties(Event::Toggle("geometry")));
                snapshot.settle(&mut app, App::view, &mut update);
                if name == "yaziliyor" || name == "yazildi" {
                    // The NO cell (read off the pictures); it opens with the default, R-?.
                    let at = if height > 800.0 {
                        iced::Point::new(1330.0, 709.0)
                    } else {
                        iced::Point::new(1000.0, 583.0)
                    };
                    snapshot.input(&mut app, App::view, &mut update, Input::Click(at));
                    for _ in 0..3 {
                        let back = Input::Key(iced::keyboard::key::Named::Backspace);
                        snapshot.input(&mut app, App::view, &mut update, back);
                    }
                    snapshot.input(&mut app, App::view, &mut update, Input::Type("R-7".into()));
                }
                if name == "yazildi" {
                    let enter = Input::Key(iced::keyboard::key::Named::Enter);
                    snapshot.input(&mut app, App::view, &mut update, enter);
                    let e = entity(&app, 4);
                    assert_eq!(e.base().attrs.get("NO").map(String::as_str), Some("R-7"));
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!(
                    "oznitelik-blok-{name}-{width}x{height}{suffix}.png"
                ));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}

/// Two texts for the text rows (docs/adr/0145 §6): one plain, 30° turned;
/// one centred, narrowed and masked.
fn two_texts(app: &mut App) -> (u32, u32) {
    let text = |p: Wire, text: &str, rotation: f64| TextEntity {
        base: base("cizim"),
        p,
        text: text.to_owned(),
        height: 2.0,
        rotation,
        align: None,
        width_factor: None,
        mask: false,
        label_of: None,
        label_scale: None,
    };
    let a = add(
        app,
        Entity::Text(text(
            Wire {
                x: E + 10.0,
                y: N + 20.0,
            },
            "Ada 101",
            30.0,
        )),
    );
    let b = add(
        app,
        Entity::Text(TextEntity {
            align: Some(kentos_contracts::TextAlign::MiddleCenter),
            width_factor: Some(0.8),
            mask: true,
            ..text(
                Wire {
                    x: E + 10.0,
                    y: N + 10.0,
                },
                "Ada 102",
                0.0,
            )
        }),
    );
    (a, b)
}

fn text_of(app: &App, slot: u32) -> TextEntity {
    match entity(app, slot) {
        Entity::Text(t) => t,
        other => panic!("a text: {other:?}"),
    }
}

/// A text's box as it is drawn (the drawing's Barlow), to a micrometre.
fn text_box(_app: &App, t: &TextEntity) -> Vec<[i64; 2]> {
    let font = kentos_geometry_core::text::Font::from_id("barlow");
    let place = kentos_geometry_core::entity::TextPlace {
        p: kentos_geometry_core::vec2::Vec2::new(t.p.x, t.p.y),
        text: &t.text,
        height: t.height,
        rotation: t.rotation,
        align: t
            .align
            .and_then(|a| kentos_geometry_core::text::TextAlign::from_name(a.name())),
        width_factor: t.width_factor,
    };
    place
        .outline(font)
        .iter()
        .map(|q| [(q.x * 1e6).round() as i64, (q.y * 1e6).round() as i64])
        .collect()
}

/// Öznitelikler's Hiza, Genişlik çarpanı and Zemin (docs/adr/0145 §6): one
/// text's values, the twelve points with their pictures; the web's are
/// apps/web/src/ui/properties/textRows.test.ts.
#[test]
fn a_texts_rows_show_its_alignment_width_factor_and_mask() {
    let mut app = objects();
    let (_, b) = two_texts(&mut app);
    select(&mut app, &[b]);
    assert_eq!(value(&app, "Geometri", "Hiza"), "Orta");
    assert_eq!(value(&app, "Geometri", "Genişlik çarpanı"), "0.8");
    assert_eq!(value(&app, "Geometri", "Zemin"), "Açık");
    let rows = rows(&app);
    let hiza = rows
        .iter()
        .flat_map(|(_, rows)| rows)
        .find(|r| r.label == "Hiza")
        .expect("Hiza");
    let Some(Editor::Select { icon, items, .. }) = &hiza.editor else {
        panic!("a drop-down");
    };
    assert_eq!(*icon, Some("textAlignMiddleCenter"));
    let picks: Vec<(String, bool, Option<&str>)> = items
        .iter()
        .filter_map(|c| match c {
            Choice::Pick {
                label,
                chosen,
                icon,
                ..
            } => Some((label.clone(), *chosen, *icon)),
            _ => None,
        })
        .collect();
    assert_eq!(picks.len(), 12);
    assert_eq!(
        picks[3],
        ("Sol orta".to_owned(), false, Some("textAlignMiddleLeft"))
    );
    assert_eq!(
        picks[4],
        ("Orta".to_owned(), true, Some("textAlignMiddleCenter"))
    );
}

/// Over a selection the texts' rows say Çeşitli where they differ; a new
/// alignment keeps each where it is, in one step.
#[test]
fn a_new_alignment_keeps_each_text_where_it_is() {
    let mut app = objects();
    let (a, b) = two_texts(&mut app);
    select(&mut app, &[a, b]);
    for label in ["Hiza", "Genişlik çarpanı", "Zemin"] {
        assert_eq!(value(&app, "Yazı", label), "Çeşitli");
    }
    let before = [
        text_box(&app, &text_of(&app, a)),
        text_box(&app, &text_of(&app, b)),
    ];
    let (pa, pb) = (text_of(&app, a).p, text_of(&app, b).p);
    event(
        &mut app,
        Event::TextAlign(
            vec![Slot(a), Slot(b)],
            Some(kentos_contracts::TextAlign::TopRight),
        ),
    );
    let (ta, tb) = (text_of(&app, a), text_of(&app, b));
    assert_eq!(
        (ta.align, tb.align),
        (
            Some(kentos_contracts::TextAlign::TopRight),
            Some(kentos_contracts::TextAlign::TopRight)
        )
    );
    assert_eq!([text_box(&app, &ta), text_box(&app, &tb)], before);
    assert_ne!(ta.p, pa);
    assert_eq!(value(&app, "Yazı", "Hiza"), "Sağ üst");
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
    let (ta, tb) = (text_of(&app, a), text_of(&app, b));
    assert_eq!(
        (ta.p, tb.p, ta.align, tb.align),
        (
            pa,
            pb,
            None,
            Some(kentos_contracts::TextAlign::MiddleCenter)
        )
    );
    // The left of the baseline takes the field away; the text still stays.
    let before = text_box(&app, &tb);
    event(&mut app, Event::TextAlign(vec![Slot(b)], None));
    let tb = text_of(&app, b);
    assert_eq!((tb.align, text_box(&app, &tb)), (None, before));
}

/// Genişlik çarpanı about each text's point, 1 as no field, a refusal said;
/// Zemin on and off, nothing written when nothing changes.
#[test]
fn width_factor_and_mask_are_written_to_the_texts() {
    let mut app = objects();
    let (a, b) = two_texts(&mut app);
    let (sa, sb) = (Slot(a), Slot(b));
    event(
        &mut app,
        Event::Commit(Field::TextWidth(vec![sa, sb]), "1,5".to_owned()),
    );
    let pb = text_of(&app, b).p;
    assert_eq!(
        (
            text_of(&app, a).width_factor,
            text_of(&app, b).width_factor,
            text_of(&app, b).p
        ),
        (Some(1.5), Some(1.5), pb)
    );
    event(
        &mut app,
        Event::Commit(Field::TextWidth(vec![sa]), "1".to_owned()),
    );
    assert_eq!(text_of(&app, a).width_factor, None);
    event(
        &mut app,
        Event::Commit(Field::TextWidth(vec![sb]), "0".to_owned()),
    );
    assert_eq!(text_of(&app, b).width_factor, Some(1.5));
    assert_eq!(
        warned(&app).as_deref(),
        Some(
            "Yazının genişlik çarpanı 0'dan büyük, en çok 100 olmalı; 0 verildi. Çarpanı bu aralıkta verin ya da alanı kaldırın (1)."
        )
    );
    // Zemin: b has one already, so turning it on writes nothing.
    let revision = |app: &App| app.document.as_ref().map_or(0, |d| d.model.revision());
    let before = revision(&app);
    event(&mut app, Event::TextMask(vec![sb], true));
    assert_eq!(revision(&app), before);
    event(&mut app, Event::TextMask(vec![sa, sb], true));
    assert_eq!((text_of(&app, a).mask, text_of(&app, b).mask), (true, true));
    event(&mut app, Event::TextMask(vec![sa, sb], false));
    assert_eq!(
        (text_of(&app, a).mask, text_of(&app, b).mask),
        (false, false)
    );
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
    assert_eq!((text_of(&app, a).mask, text_of(&app, b).mask), (true, true));
}

/// Two leaders: a filled arrow with its note, an open one masked, on Çizim.
fn two_leaders(app: &mut App) -> (u32, u32) {
    let leader = |x: f64, text: &str, arrow, mask| kentos_contracts::LeaderEntity {
        base: base("cizim"),
        pts: vec![
            Wire {
                x: 487000.0 + x,
                y: 4420020.0,
            },
            Wire {
                x: 487004.0 + x,
                y: 4420023.0,
            },
        ],
        text: Some(text.to_owned()),
        height: 2.0,
        rotation: 0.0,
        arrow,
        mask,
    };
    let a = add(app, Entity::Leader(leader(0.0, "Mevcut bina", None, false)));
    let b = add(
        app,
        Entity::Leader(leader(
            20.0,
            "Ø150 PVC",
            Some(kentos_contracts::LeaderArrow::Open),
            true,
        )),
    );
    (a, b)
}

fn leader_of(app: &App, slot: u32) -> kentos_contracts::LeaderEntity {
    match entity(app, slot) {
        Entity::Leader(l) => l,
        other => panic!("a leader: {other:?}"),
    }
}

/// A leader's rows (docs/adr/0146 §7): its note, height, turn, arrowhead and
/// mask under Geometri; over a selection, Kılavuz with Çeşitli where they
/// differ, every change one step “Değiştir”, nothing written when nothing
/// changes, an emptied note the arrow alone, a height not over 0 not taken.
#[test]
fn a_leaders_rows_write_its_note_height_turn_arrowhead_and_mask() {
    let mut app = objects();
    let (a, b) = two_leaders(&mut app);
    select(&mut app, &[a]);
    assert_eq!(value(&app, "Geometri", "Not"), "Mevcut bina");
    assert_eq!(value(&app, "Geometri", "Yükseklik"), "2.000 m");
    assert_eq!(value(&app, "Geometri", "Dönüş"), "0.00 °");
    assert_eq!(value(&app, "Geometri", "Ok"), "Dolu");
    assert_eq!(value(&app, "Geometri", "Zemin"), "Kapalı");
    assert_eq!(value(&app, "Geometri", "Köşe sayısı"), "2");
    assert_eq!(value(&app, "Geometri", "Uzunluk"), "5.000 m");
    select(&mut app, &[a, b]);
    for label in ["Not", "Ok", "Zemin"] {
        assert_eq!(value(&app, "Kılavuz", label), "Çeşitli");
    }
    assert_eq!(value(&app, "Kılavuz", "Yükseklik"), "2.000 m");
    let (sa, sb) = (Slot(a), Slot(b));
    event(
        &mut app,
        Event::LeaderArrow(vec![sa, sb], Some(kentos_contracts::LeaderArrow::Dot)),
    );
    assert_eq!(
        (leader_of(&app, a).arrow, leader_of(&app, b).arrow),
        (
            Some(kentos_contracts::LeaderArrow::Dot),
            Some(kentos_contracts::LeaderArrow::Dot)
        )
    );
    assert_eq!(value(&app, "Kılavuz", "Ok"), "Nokta");
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
    // Zemin on: b has it already, a alone is written.
    event(&mut app, Event::LeaderMask(vec![sa, sb], true));
    assert_eq!((leader_of(&app, a).mask, leader_of(&app, b).mask), (true, true));
    event(
        &mut app,
        Event::Commit(Field::LeaderTurn(vec![sa, sb]), "370".to_owned()),
    );
    assert_eq!(leader_of(&app, b).rotation, 10.0);
    let revision = |app: &App| app.document.as_ref().map_or(0, |d| d.model.revision());
    let before = revision(&app);
    event(
        &mut app,
        Event::Commit(Field::LeaderHeight(vec![sa]), "0".to_owned()),
    );
    assert_eq!(revision(&app), before, "a height not over 0 is not taken");
    event(
        &mut app,
        Event::Commit(Field::LeaderHeight(vec![sa]), "3,5".to_owned()),
    );
    assert_eq!(leader_of(&app, a).height, 3.5);
    event(
        &mut app,
        Event::Commit(Field::LeaderNote(vec![sb]), "  ".to_owned()),
    );
    assert_eq!(leader_of(&app, b).text, None, "the arrow alone");
    event(
        &mut app,
        Event::Commit(Field::LeaderNote(vec![sb]), " Ø200 PVC ".to_owned()),
    );
    assert_eq!(leader_of(&app, b).text.as_deref(), Some("Ø200 PVC"));
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
    assert_eq!(leader_of(&app, b).text, None);
}

/// A dimension of `style` from (E + a, N) to (E + b, N), on Çizim.
fn dimension(style: kentos_contracts::DimensionStyle, a: [f64; 2], b: [f64; 2]) -> DimensionEntity {
    DimensionEntity {
        base: base("cizim"),
        a: Wire { x: E + a[0], y: N + a[1] },
        b: Wire { x: E + b[0], y: N + b[1] },
        offset: 3.0,
        height: 2.5,
        text: None,
        style: Some(style),
        angle: None,
        c: None,
        mask: false,
        za: None,
        zb: None,
    }
}

fn dimension_of(app: &App, slot: u32) -> DimensionEntity {
    match entity(app, slot) {
        Entity::Dimension(d) => d,
        other => panic!("a dimension: {other:?}"),
    }
}

/// A dimension's rows (docs/adr/0147 §7): Zemin; an ordinate's Koordinat; a
/// slope's two elevations; an arc length's radius and angle, shown only.
/// Over a selection, Ölçü with Çeşitli where they differ, the kind's rows
/// only when all are of it; every change one step “Değiştir”, those that
/// have the value left out. The web's `dimensionRows.test.ts`.
#[test]
fn a_dimensions_rows_write_its_mask_axis_and_elevations() {
    use kentos_contracts::DimensionStyle::{ArcLength, Ordinate, Slope};
    let mut app = objects();
    // A quarter arc of radius 10 about (E, N + 40), east to north, 3 m out.
    let arc = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            c: Some(Wire { x: E, y: N + 40.0 }),
            ..dimension(ArcLength, [10.0, 40.0], [0.0, 50.0])
        }),
    );
    select(&mut app, &[arc]);
    assert_eq!(value(&app, "Geometri", "Zemin"), "Kapalı");
    assert_eq!(value(&app, "Geometri", "Yarıçap"), "10.000 m");
    assert_eq!(value(&app, "Geometri", "Açı"), "100.0000 g");
    // Two ordinates, the Y's and the X's.
    let oy = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            angle: Some(0.0),
            ..dimension(Ordinate, [0.0, 60.0], [5.0, 70.0])
        }),
    );
    let ox = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            angle: Some(90.0),
            mask: true,
            ..dimension(Ordinate, [0.0, 80.0], [-10.0, 85.0])
        }),
    );
    select(&mut app, &[oy, ox]);
    assert_eq!(value(&app, "Ölçü", "Zemin"), "Çeşitli");
    assert_eq!(value(&app, "Ölçü", "Koordinat"), "Çeşitli");
    let revision = |app: &App| app.document.as_ref().map_or(0, |d| d.model.revision());
    event(&mut app, Event::DimensionAxis(vec![Slot(oy), Slot(ox)], 90.0));
    assert_eq!(
        (dimension_of(&app, oy).angle, dimension_of(&app, ox).angle),
        (Some(90.0), Some(90.0))
    );
    assert_eq!(value(&app, "Ölçü", "Koordinat"), "X");
    let before = revision(&app);
    event(&mut app, Event::DimensionAxis(vec![Slot(oy), Slot(ox)], 90.0));
    assert_eq!(revision(&app), before, "nothing written when both have it");
    // Zemin on: the X's has it already, the Y's alone is written.
    event(&mut app, Event::DimensionMask(vec![Slot(oy), Slot(ox)], true));
    assert!(dimension_of(&app, oy).mask && dimension_of(&app, ox).mask);
    assert_eq!(undo_label(&mut app).as_deref(), Some("Değiştir"));
    assert!(!dimension_of(&app, oy).mask);
    // Two slopes: one first elevation, two second ones.
    let s1 = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            za: Some(100.0),
            zb: Some(99.0),
            ..dimension(Slope, [0.0, 100.0], [40.0, 100.0])
        }),
    );
    let s2 = add(
        &mut app,
        Entity::Dimension(DimensionEntity {
            za: Some(100.0),
            zb: Some(98.0),
            ..dimension(Slope, [0.0, 110.0], [40.0, 110.0])
        }),
    );
    select(&mut app, &[s1, s2]);
    assert_eq!(value(&app, "Ölçü", "Birinci kot"), "100.000 m");
    assert_eq!(value(&app, "Ölçü", "İkinci kot"), "Çeşitli");
    event(
        &mut app,
        Event::Commit(Field::DimensionZb(vec![Slot(s1), Slot(s2)]), "97,5".to_owned()),
    );
    assert_eq!(
        (dimension_of(&app, s1).zb, dimension_of(&app, s2).zb),
        (Some(97.5), Some(97.5))
    );
    assert_eq!(value(&app, "Ölçü", "İkinci kot"), "97.500 m");
    let before = revision(&app);
    event(
        &mut app,
        Event::Commit(Field::DimensionZa(vec![Slot(s1)]), "kot".to_owned()),
    );
    assert_eq!(revision(&app), before, "not a number: nothing written");
    // An ordinate and a slope: Zemin only; beside another object, counted.
    select(&mut app, &[oy, s1]);
    let titles: Vec<String> = rows(&app).into_iter().map(|(t, _)| t).collect();
    assert!(titles.contains(&"Ölçü".to_owned()), "{titles:?}");
    let section = rows(&app).into_iter().find(|(t, _)| t == "Ölçü").expect("Ölçü");
    let labels: Vec<String> = section.1.iter().map(|r| r.label.to_string()).collect();
    assert_eq!(labels, ["Zemin"]);
    select(&mut app, &[oy, 1]);
    assert_eq!(value(&app, "Ölçüler (1)", "Zemin"), "Kapalı");
}
