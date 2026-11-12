//! Öznitelikler's tests: elevations (Kot, docs/adr/0142) of lines, polylines, areas and points, alone
//! and several together, and their pictures.

use super::*;

pub(super) fn wire(x: f64, y: f64) -> Wire {
    Wire { x: E + x, y: N + y }
}

/// A line on `cizim` from (0, 30) to (40, 30) with the elevations of its ends.
pub(super) fn line_z(app: &mut App, za: Option<f64>, zb: Option<f64>) -> u32 {
    add(
        app,
        Entity::Line(LineEntity {
            ma: None,
            mb: None,
            base: base("cizim"),
            a: wire(0.0, 30.0),
            b: wire(40.0, 30.0),
            za,
            zb,
        }),
    )
}

/// An open polyline on `cizim` through three vertices 10 m apart.
pub(super) fn path_z(app: &mut App, zs: Option<Vec<Option<f64>>>) -> u32 {
    add(
        app,
        Entity::Polyline(kentos_commands::PathEntity {
            space_separator: None,
            ms: None,
            base: base("cizim"),
            pts: vec![wire(0.0, 40.0), wire(10.0, 40.0), wire(20.0, 40.0)],
            bulges: None,
            holes: None,
            zs,
            parts: None,
            closed: false,
            widths: None,
        }),
    )
}

/// A 20 × 10 m area on `parsel` with a 4 × 2 m hole.
pub(super) fn area_z(
    app: &mut App,
    zs: Option<Vec<Option<f64>>>,
    hole: Option<Vec<Option<f64>>>,
) -> u32 {
    add(
        app,
        Entity::Polygon(kentos_commands::PathEntity {
            space_separator: None,
            ms: None,
            base: base("parsel"),
            pts: vec![
                wire(0.0, 50.0),
                wire(20.0, 50.0),
                wire(20.0, 60.0),
                wire(0.0, 60.0),
            ],
            bulges: None,
            holes: Some(vec![kentos_commands::RingGeometry {
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
            closed: false,
            widths: None,
        }),
    )
}

pub(super) fn some(v: &[f64]) -> Option<Vec<Option<f64>>> {
    Some(v.iter().copied().map(Some).collect())
}

/// Whether a row of the section is there.
pub(super) fn has_row(app: &App, section: &str, label: &str) -> bool {
    rows(app)
        .into_iter()
        .find(|(title, _)| title == section)
        .is_some_and(|(_, rows)| rows.iter().any(|r| r.label == label))
}

/// The note under a row, when one follows it: a row with no name of its own.
pub(super) fn note_after(app: &App, section: &str, label: &str) -> Option<String> {
    let (_, rows) = rows(app).into_iter().find(|(title, _)| title == section)?;
    let at = rows.iter().position(|r| r.label == label)?;
    rows.get(at + 1)
        .filter(|r| r.note && r.label.is_empty() && r.editor.is_none())
        .map(|r| r.value.clone())
}

pub(super) fn row_editable(app: &App, section: &str, label: &str) -> bool {
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
        Entity::Point(kentos_commands::PointEntity {
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
            ma: None,
            mb: None,
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
        Entity::Point(kentos_commands::PointEntity {
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
        Entity::Point(kentos_commands::PointEntity {
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
            Entity::Circle(kentos_commands::CircleEntity {
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
        Entity::Point(kentos_commands::PointEntity {
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

/// A locked layer among several no longer takes the row's cell away (docs/adr/0286): the cell writes the open
/// objects only; the locked line keeps its elevation.
#[test]
fn a_locked_layer_among_several_leaves_the_cell_to_the_others() {
    let mut app = objects();
    let open = path_z(&mut app, some(&[5.0; 3]));
    let locked = add(
        &mut app,
        Entity::Line(LineEntity {
            ma: None,
            mb: None,
            base: base("kilitli"),
            a: wire(0.0, 30.0),
            b: wire(40.0, 30.0),
            za: Some(5.0),
            zb: Some(5.0),
        }),
    );
    select(&mut app, &[open, locked]);
    assert_eq!(value(&app, "Ortak özellikler", "Kot"), "5.000 m");
    assert!(row_editable(&app, "Ortak özellikler", "Kot"));
    event(
        &mut app,
        Event::Commit(Field::Elevations(vec![Slot(open)]), "7".to_owned()),
    );
    let Entity::Line(l) = entity(&app, locked) else {
        panic!("a line");
    };
    assert_eq!(l.za, Some(5.0));
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
