//! Öznitelikler over the selection traces' drawing (fixtures/interaction/v1/objects.kcad):
//! what it shows for no object, one and several, and what its edits write.
//! Expected values are worked out by hand from the drawing.

use std::collections::BTreeMap;

use iced::keyboard::Modifiers;
use kentos_contracts::{
    DimensionEntity, Entity, EntityBase, HatchEntity, HatchPattern, HatchPatternType, TextEntity,
    Vec2 as Wire,
};
use kentos_domain::Slot;

use super::{Choice, Editor, Event, Field, Row, Summary, web_number};
use crate::app::{App, Message};
use crate::selecting::tests::{click, objects};

const E: f64 = 487000.0;
const N: f64 = 4420000.0;

fn rows(app: &App) -> Vec<(String, Vec<Row>)> {
    let doc = app.document.as_ref().expect("open");
    app.properties_panel(doc)
        .sections
        .into_iter()
        .map(|s| (s.title.to_owned(), s.rows))
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
    assert!(matches!(
        app.history.last(),
        Some(kentos_ui::widget::command_line::Entry::Warning(t))
            if t == "“Gizli katman” katmanı gizli; taşınan nesneler görünmeyecek."
    ));
    assert_eq!(app.selection.ids(), [Slot(5)]);
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
