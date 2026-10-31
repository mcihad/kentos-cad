use crate::common::Bench;
use kentos_commands::{DrawingUnit, Entity, Workspace};
use kentos_domain::Slot;

struct CloseView;
impl kentos_interaction::View for CloseView {
    fn to_screen(&self, p: kentos_interaction::Vec2) -> [f64; 2] {
        [
            (p.x - crate::common::E) * 1000.,
            -(p.y - crate::common::N) * 1000.,
        ]
    }
    fn world_length(&self, px: f64) -> f64 {
        px / 1000.
    }
    fn visible(&self) -> kentos_geometry_core::geometry::Bounds {
        kentos_geometry_core::geometry::Bounds {
            min_x: crate::common::E - 1.,
            min_y: crate::common::N - 1.,
            max_x: crate::common::E + 21.,
            max_y: crate::common::N + 1.,
        }
    }
}
static CLOSE: CloseView = CloseView;

fn source() -> (Bench, Slot) {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.start("wall");
    b.click(0., 0.);
    b.click(20., 0.);
    b.confirm();
    b.confirm();
    let wall = Slot(b.newest().base().id);
    b.selection.set([wall]);
    b.start("wallDimensions");
    (b, wall)
}

fn configure(b: &mut Bench, key: &str, value: &str) -> bool {
    b.run(|session, cx| session.configure(key, value, cx))
}

#[test]
fn visible_settings_and_live_text_preview_do_not_write_and_one_confirmation_places_the_set() {
    let (mut b, _) = source();
    let before = b.doc.to_snapshot_v2();
    for (key, value) in [("R", "9 mm"), ("B", "centers"), ("D", "60 cm")] {
        assert!(configure(&mut b, key, value));
    }
    let settings = b.session.settings(&Default::default());
    for key in ["C", "L", "S", "F", "A", "B", "T", "D", "R"] {
        assert!(settings.iter().any(|setting| setting.key == key));
    }
    assert!(settings.iter().any(|setting| setting.note && !setting.warn));
    // A wall without openings: its openings row is its whole length, written once (docs/adr/0695's 27 October
    // addendum), not again as the overall row.
    let preview = b.session.preview(&Default::default()).unwrap();
    assert!(!preview.strokes.is_empty());
    assert_eq!(preview.texts.len(), 1);
    assert!(preview.texts.iter().all(|text| !text.text.is_empty()));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.confirm();
    assert!(!b.session.is_running());
    assert_eq!(b.doc.dimension_sets().len(), 1);
    assert_eq!(b.doc.dimension_sets()[0].members.len(), 1);
    assert_eq!(b.doc.dimension_sets()[0].gap_mm, 9.);
    assert_eq!(b.selection.len(), 1);
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
    assert!(b.doc.dimension_sets().is_empty());
}

#[test]
fn pointer_placement_needs_one_click_after_source_choice_and_escape_writes_nothing() {
    let (mut b, _) = source();
    let before = b.doc.to_snapshot_v2();
    b.move_to(10., -2.);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.run(|session, cx| session.cancel(cx));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.selection.clear();
    b.start("wallDimensions");
    b.click(10., 0.);
    assert_eq!(b.points(), 1);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(10., -2.);
    assert!(!b.session.is_running());
    assert_eq!(b.doc.dimension_sets().len(), 1);
    assert_eq!(
        b.doc.dimension_sets()[0].face,
        kentos_commands::DimensionSetFace::Right
    );
}

#[test]
fn invalid_paper_units_make_no_partial_dimensions() {
    let (mut b, _) = source();
    let before = b.doc.to_snapshot_v2();
    for value in ["10 cm", "1 m", "0 mm", "-4 mm", "1001", "NaN"] {
        assert!(!configure(&mut b, "R", value));
    }
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

/// The source wall made a closed 20 × 10 room, counter-clockwise: its left face is inside.
fn room() -> (Bench, Slot) {
    let (mut b, wall) = source();
    b.run(|session, cx| session.cancel(cx));
    let mut source = b.doc.get(wall).unwrap().clone();
    let Entity::Wall(w) = &mut source else {
        panic!()
    };
    let a = w.wall.pts[0];
    let z = w.wall.pts[1];
    w.wall.pts.extend([
        kentos_commands::Vec2 {
            x: z.x,
            y: z.y + 10.,
        },
        kentos_commands::Vec2 {
            x: a.x,
            y: a.y + 10.,
        },
    ]);
    w.wall.closed = true;
    b.doc.update_many(vec![(wall, source)], "closed wall");
    b.selection.clear();
    (b, wall)
}

/// docs/adr/0700: on a closed wall the edge under the pointer is measured, its outer face by the pointer's side.
#[test]
fn a_closed_wall_measures_the_edge_under_the_pointer() {
    let (mut b, _) = room();
    b.start("wallDimensions");
    let before = b.doc.to_snapshot_v2();
    b.click(20., 5.);
    assert_eq!(b.points(), 1, "the wall is chosen");
    let settings = b.session.settings(&Default::default());
    assert!(
        settings
            .iter()
            .any(|s| s.key == "F" && s.choices.iter().any(|c| c.label == "Dış yüz"))
    );
    assert!(settings.iter().any(|s| s.key == "N"));
    b.move_to(22., 5.);
    assert_eq!(b.doc.to_snapshot_v2(), before, "the preview writes nothing");
    b.click(22., 5.);
    assert!(!b.session.is_running());
    let set = &b.doc.dimension_sets()[0];
    assert_eq!(set.edge, Some(1));
    assert_eq!(set.face, kentos_commands::DimensionSetFace::Right);
    assert_eq!(set.members.len(), 1);
    b.doc.undo().unwrap();
    assert!(b.doc.dimension_sets().is_empty());
}

/// Kenarlar → Bütün kenarlar: a chain on each edge, on the outer face, one confirmation and one undo step.
#[test]
fn every_edge_of_a_closed_wall_in_one_step() {
    let (mut b, wall) = room();
    b.selection.set([wall]);
    b.start("wallDimensions");
    assert!(configure(&mut b, "N", "all"));
    // Inside the room, the rows still go out from the outer faces.
    b.move_to(10., 1.);
    let preview = b.session.preview(&Default::default()).unwrap();
    assert_eq!(preview.texts.len(), 4, "four edges, one dimension each: no openings, the overall row not repeated");
    assert!(
        preview
            .tag
            .as_ref()
            .is_some_and(|tag| tag.lines[0].contains("4 kenar"))
    );
    let before = b.doc.to_snapshot_v2();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.confirm();
    assert!(!b.session.is_running());
    let sets = b.doc.dimension_sets();
    assert_eq!(sets.len(), 4);
    assert_eq!(
        sets.iter().map(|s| s.edge).collect::<Vec<_>>(),
        [Some(0), Some(1), Some(2), Some(3)]
    );
    assert!(
        sets.iter()
            .all(|s| s.face == kentos_commands::DimensionSetFace::Right)
    );
    assert_eq!(b.selection.len(), 4);
    b.doc.undo().unwrap();
    assert!(b.doc.dimension_sets().is_empty());
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
}

#[test]
fn source_revision_change_requires_a_fresh_review_before_writing() {
    let (mut b, wall) = source();
    let mut entity = b.doc.get(wall).unwrap().clone();
    entity.base_mut().label = Some("yeniden bak".into());
    b.doc.update_many(vec![(wall, entity)], "another edit");
    let changed = b.doc.to_snapshot_v2();
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2(), changed);
    b.confirm();
    assert_eq!(b.doc.dimension_sets().len(), 1);
}

#[test]
fn opening_create_previews_added_and_removed_measurements_without_writing() {
    let (mut b, _) = source();
    b.confirm();
    b.selection.clear();
    b.start("door");
    assert!(configure(&mut b, "E", "1 m"));
    let before = b.doc.to_snapshot_v2();
    b.move_to(5., 0.2);
    let preview = b.session.preview(&Default::default()).unwrap();
    assert!(preview.texts.len() >= 3, "new intervals must be visible");
    assert!(
        preview
            .strokes
            .iter()
            .any(|stroke| stroke.tone == kentos_interaction::Tone::Danger),
        "the replaced interval must be marked"
    );
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(5., 0.2);
    assert_eq!(b.doc.dimension_sets()[0].members.len(), 4);
    b.confirm();
    b.doc.undo().unwrap();
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
}

#[test]
fn a_close_zoom_accepts_the_unhatched_wall_body_far_from_its_edges() {
    let (mut b, wall) = source();
    b.run(|session, cx| session.cancel(cx));
    b.selection.clear();
    b.start("wallDimensions");
    let at = Bench::pointer(10., 0.003);
    b.spatial.sync(&b.doc);
    // The body's centre is far beyond the faces' aperture; the store picks the wall inside its body (docs/adr/0776
    // §3), not by an edge.
    assert!(b.spatial.hits(at.raw, 0.005).contains(&wall));
    assert!(
        !b.spatial
            .store()
            .hit_edge(at.raw, 0.005)
            .iter()
            .any(|(id, _)| *id as u32 == wall.0),
        "body centre is far beyond the edge aperture"
    );
    let before = b.doc.to_snapshot_v2();
    b.run(|session, cx| {
        cx.view = &CLOSE;
        session.pointer_move(&at, cx);
    });
    b.run(|session, cx| {
        cx.view = &CLOSE;
        session.pointer_down(&at, cx);
    });
    assert_eq!(b.points(), 1, "the first body click chooses the wall");
    assert!(
        !b.session
            .preview(&Default::default())
            .unwrap()
            .texts
            .is_empty()
    );
    assert_eq!(b.doc.to_snapshot_v2(), before, "source choice is read-only");
    b.confirm();
    assert_eq!(
        b.doc.dimension_sets()[0].wall.0,
        b.doc.uid(wall).unwrap().into_bytes()
    );
}

#[test]
fn overlapping_unhatched_bodies_choose_the_nearest_physical_wall_centre() {
    let (mut b, first) = source();
    b.run(|session, cx| session.cancel(cx));
    let mut second = b.doc.get(first).unwrap().clone();
    second.base_mut().id = 0;
    let Entity::Wall(wall) = &mut second else {
        panic!()
    };
    // The second path is deliberately offset: distance must be measured to the actual body.
    wall.wall.offset = 0.04;
    let second = b.doc.add_many(vec![second], "overlapping source").unwrap()[0];
    b.selection.clear();
    b.start("wallDimensions");
    let at = Bench::pointer(10., 0.03);
    let before = b.doc.to_snapshot_v2();
    b.run(|session, cx| {
        cx.view = &CLOSE;
        session.pointer_down(&at, cx);
    });
    assert_eq!(b.points(), 1);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.confirm();
    assert_eq!(
        b.doc.dimension_sets()[0].wall.0,
        b.doc.uid(second).unwrap().into_bytes()
    );
}

#[test]
fn entered_distance_survives_pointer_travel_and_one_confirm_or_canvas_click() {
    for click in [false, true] {
        let (mut b, _) = source();
        let before = b.doc.to_snapshot_v2();
        assert!(configure(&mut b, "D", "1 m"));
        assert!(configure(&mut b, "R", "10 mm"));
        let fixed = b.session.preview(&Default::default()).unwrap();
        assert!(
            b.session
                .settings(&Default::default())
                .iter()
                .any(|setting| setting.key == "P" && setting.value == "Sabit uzaklık")
        );
        assert_eq!(
            b.doc.to_snapshot_v2(),
            before,
            "settings never place the dimensions"
        );
        // The pointer crosses the wall on its way to an unrelated menu position.
        b.move_to(-5., -3.);
        assert_eq!(
            b.session.preview(&Default::default()).unwrap(),
            fixed,
            "numeric placement stays authoritative"
        );
        assert_eq!(b.doc.to_snapshot_v2(), before);
        if click {
            b.click(-5., -3.);
        } else {
            b.confirm();
        }
        assert!(!b.session.is_running(), "one placement action finishes");
        let set = &b.doc.dimension_sets()[0];
        assert!(
            (b.doc
                .dimension_set_row_offset(set, kentos_commands::DimensionSetRow::Openings)
                .unwrap()
                - 1.)
                .abs()
                < 1e-8
        );
        assert_eq!(set.gap_mm, 10.);
        assert_eq!(set.face, kentos_commands::DimensionSetFace::Left);
    }
}

#[test]
fn cursor_mode_restores_pointer_placement_without_writing_a_draft() {
    let (mut b, _) = source();
    let before = b.doc.to_snapshot_v2();
    assert!(configure(&mut b, "D", "1 m"));
    let fixed = b
        .session
        .preview(&Default::default())
        .unwrap()
        .tag
        .unwrap()
        .at;
    b.move_to(10., -3.);
    assert_eq!(
        b.session
            .preview(&Default::default())
            .unwrap()
            .tag
            .unwrap()
            .at,
        fixed
    );
    assert!(configure(&mut b, "P", "cursor"));
    b.move_to(10., -3.);
    let moved = b
        .session
        .preview(&Default::default())
        .unwrap()
        .tag
        .unwrap()
        .at;
    assert!((moved.y - (crate::common::N - 3.)).abs() < 1e-10);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.confirm();
    assert_eq!(
        b.doc.dimension_sets()[0].face,
        kentos_commands::DimensionSetFace::Right
    );
    assert_eq!(b.doc.dimension_sets()[0].at.y, moved.y);
}

#[test]
fn changing_source_resets_fixed_placement_and_bare_keyboard_distance_still_places_once() {
    let (mut b, _) = source();
    assert!(configure(&mut b, "D", "1 m"));
    assert!(b.type_text("K"));
    assert!(
        b.session
            .settings(&Default::default())
            .iter()
            .any(|setting| setting.key == "P" && setting.value == "İmleçle")
    );
    b.click(10., 0.);
    assert!(configure(&mut b, "D", "1 m"));
    assert!(b.undo_step());
    assert_eq!(b.points(), 0);
    assert!(
        b.session
            .settings(&Default::default())
            .iter()
            .any(|setting| setting.key == "P" && setting.value == "İmleçle")
    );
    b.click(10., 0.);
    assert!(b.type_text("1 m"));
    assert!(!b.session.is_running());
    assert_eq!(b.doc.dimension_sets().len(), 1);
    let set = &b.doc.dimension_sets()[0];
    assert!(
        (b.doc
            .dimension_set_row_offset(set, kentos_commands::DimensionSetRow::Openings)
            .unwrap()
            - 1.)
            .abs()
            < 1e-8
    );
}

/// docs/adr/0729: a 10 × 8 house of four 20 cm walls drawn corner to corner at 1:50, a partition at x = 4, doors on
/// its south wall at 1–2 m and 6–7 m; the walls' slots south, east, north, west, partition.
fn house() -> (Bench, [Slot; 5]) {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    settings.plot_scale = 50.0;
    b.doc.set_settings(settings);
    b.session.exit();
    let mut slots = Vec::new();
    for (a, z) in [((0., 0.), (10., 0.)), ((10., 0.), (10., 8.)), ((10., 8.), (0., 8.)), ((0., 8.), (0., 0.)), ((4., 0.), (4., 8.))] {
        b.start("wall");
        b.click(a.0, a.1);
        b.click(z.0, z.1);
        b.confirm();
        b.confirm();
        slots.push(Slot(b.newest().base().id));
    }
    for x in [1.5, 6.5] {
        b.start("door");
        assert!(configure(&mut b, "E", "1 m"));
        b.click(x, 0.05);
        b.session.exit();
    }
    (b, [slots[0], slots[1], slots[2], slots[3], slots[4]])
}

fn tag(b: &Bench) -> Vec<String> {
    b.session
        .preview(&Default::default())
        .and_then(|p| p.tag)
        .map(|t| t.lines)
        .unwrap_or_default()
}

/// Cephe: the wall clicked, measured with the walls at its corners: their outer faces close the rows, the door
/// between; one confirmation writes the recipe, one undo takes it back.
#[test]
fn a_facade_of_the_clicked_wall_runs_to_its_corner_walls_outer_faces() {
    let (mut b, [south, ..]) = house();
    b.selection.clear();
    b.start("wallDimensions");
    assert!(configure(&mut b, "M", "facade"));
    let before = b.doc.to_snapshot_v2();
    b.click(5., 0.);
    assert_eq!(b.points(), 1, "the façade is chosen");
    b.move_to(5., -1.5);
    assert_eq!(b.doc.to_snapshot_v2(), before, "the preview writes nothing");
    let lines = tag(&b);
    assert!(lines[0].starts_with("6 ölçü"), "{lines:?}");
    assert!(lines.iter().any(|l| l.starts_with("Açıklıklar: 5 ölçü · 4 kaynak")), "{lines:?}");
    let values: Vec<String> = b.session.preview(&Default::default()).unwrap().texts.iter().map(|t| t.text.clone()).collect();
    assert_eq!(values, ["1.100", "1.000", "4.000", "1.000", "3.100", "10.200"], "outer face to outer face");
    b.click(5., -1.5);
    assert!(!b.session.is_running());
    let set = &b.doc.dimension_sets()[0];
    let span = set.span.as_ref().expect("a façade's recipe");
    assert_eq!(set.wall.0, b.doc.uid(south).unwrap().into_bytes());
    assert!(span.walls.is_empty() && !span.space);
    assert_eq!(set.members.len(), 6);
    b.doc.undo().unwrap();
    assert!(b.doc.dimension_sets().is_empty());
}

/// Walls selected before the tool make a façade; Dört yan writes four recipes in one step.
#[test]
fn selected_walls_make_a_facade_and_four_sides_go_in_one_step() {
    let (mut b, [s, e, n, w, _]) = house();
    b.selection.set([s, e, n, w]);
    b.start("wallDimensions");
    let settings = b.session.settings(&Default::default());
    assert!(settings.iter().any(|r| r.key == "M" && r.value == "Cephe"), "several walls are a façade");
    for key in ["O", "Y", "N", "W", "G"] {
        assert!(settings.iter().any(|r| r.key == key), "{key}");
    }
    assert!(configure(&mut b, "N", "all"));
    let lines = tag(&b);
    assert!(lines[0].contains("dört yan"), "{lines:?}");
    let before = b.doc.to_snapshot_v2();
    b.confirm();
    assert!(!b.session.is_running());
    let sets = b.doc.dimension_sets();
    assert_eq!(sets.len(), 4);
    assert!(sets.iter().all(|s| s.span.as_ref().is_some_and(|p| p.walls.len() == 3)));
    b.doc.undo().unwrap();
    assert!(b.doc.dimension_sets().is_empty());
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
}

/// Duvarlar sırası: the walls' thicknesses are measured; a value wider than its interval is said and drawn in the
/// danger tone, never hidden (docs/adr/0729 §6).
#[test]
fn values_that_do_not_fit_are_moved_aside_or_counted_and_drawn_in_the_danger_tone() {
    let (mut b, [south, ..]) = house();
    b.selection.set([south]);
    b.start("wallDimensions");
    assert!(configure(&mut b, "M", "facade"));
    assert!(configure(&mut b, "W", "yes"));
    b.move_to(5., -1.5);
    // Sığmayan yazı → Kaydır, the new recipe's (docs/adr/0730 §2): the thin walls' values go aside, none in danger.
    let lines = tag(&b);
    assert!(lines.iter().any(|l| l == "3 ölçünün yazısı yana alındı"), "{lines:?}");
    let settings = b.session.settings(&Default::default());
    assert!(!settings.iter().any(|r| r.key == "U"), "nothing left that does not fit");
    let preview = b.session.preview(&Default::default()).unwrap();
    assert!(!preview.strokes.iter().any(|s| s.tone == kentos_interaction::Tone::Danger));
    // Yerinde: they stay where they are, said and drawn in the danger tone (docs/adr/0729 §6).
    assert!(configure(&mut b, "I", "keep"));
    let settings = b.session.settings(&Default::default());
    let notice = settings.iter().find(|r| r.key == "U").expect("the tight values are said");
    assert!(notice.value.starts_with("3 ölçünün"), "{}", notice.value);
    let preview = b.session.preview(&Default::default()).unwrap();
    assert!(preview.strokes.iter().any(|s| s.tone == kentos_interaction::Tone::Danger));
    assert!(tag(&b).iter().any(|l| l.starts_with("Duvarlar: 5 ölçü")), "{:?}", tag(&b));
    // Pafta ölçekleri, typed in any order: read back ascending, each sheet said; one confirmation writes them.
    assert!(configure(&mut b, "I", "shift"));
    assert!(configure(&mut b, "Ö", "1:200 100"));
    assert!(!configure(&mut b, "Ö", "0.5"), "a scale below 1 is refused");
    let settings = b.session.settings(&Default::default());
    let scales = settings.iter().find(|r| r.key == "Ö").unwrap();
    assert_eq!(scales.value, "1:100, 1:200");
    let sheets = settings.iter().find(|r| r.key == "V").expect("each sheet said");
    assert!(sheets.value.starts_with("1:100: "), "{}", sheets.value);
    assert!(tag(&b).iter().any(|l| l.starts_with("1:200: ")), "{:?}", tag(&b));
    b.confirm();
    let set = &b.doc.dimension_sets()[0];
    assert_eq!(set.scales, [100.0, 200.0]);
    assert_eq!(set.tight, kentos_commands::DimensionSetTight::Shift);
    // The next façade starts with the same choices.
    assert_eq!(b.memory.wall_dimensions_scales[..2], [100.0, 200.0]);
}

/// Mahal: a click inside a room measures its outline along the direction; the room's door on its edge facing the
/// rows is in the openings row.
#[test]
fn a_space_clicked_is_measured_with_its_door() {
    let (mut b, _) = house();
    b.start("space");
    b.click(2., 4.);
    b.session.exit();
    assert!(matches!(b.newest(), Entity::Space(_)));
    b.selection.clear();
    b.start("wallDimensions");
    assert!(configure(&mut b, "M", "space"));
    b.click(2., 4.);
    assert_eq!(b.points(), 1, "the room is chosen");
    // Along its longest edge (north) first; across it, along the south edge with the door.
    assert_eq!(tag(&b)[0], "1 ölçü · tek adım");
    assert!(configure(&mut b, "O", "across"));
    b.move_to(2., -1.5);
    let settings = b.session.settings(&Default::default());
    assert!(settings.iter().any(|r| r.key == "C" && r.value.starts_with("Mahal")));
    assert!(!settings.iter().any(|r| r.key == "Y"), "a room has no core to choose");
    b.confirm();
    let set = &b.doc.dimension_sets()[0];
    assert!(set.span.as_ref().is_some_and(|s| s.space));
    let lengths: Vec<f64> = set
        .members
        .iter()
        .map(|m| {
            let Some(Entity::Dimension(d)) =
                b.doc.slot_of(kentos_domain::Uuid::from_bytes(m.uid.0)).and_then(|s| b.doc.get(s))
            else {
                panic!("a dimension");
            };
            ((d.b.x - d.a.x).abs() * 1e6).round() / 1e6
        })
        .collect();
    assert_eq!(lengths, [0.9, 1.0, 1.9, 3.8], "west to east: the door in the room's south edge, its net length");
}

/// The source wall with a dimension set on it and a 1 m door at 5 m: the set's four members.
fn measured_door() -> (Bench, Slot, Slot) {
    let (mut b, wall) = source();
    b.confirm();
    b.selection.clear();
    b.start("door");
    assert!(configure(&mut b, "E", "1 m"));
    b.click(5., 0.2);
    b.confirm();
    let door = b
        .doc
        .entities()
        .filter(|e| matches!(e, Entity::Opening(_)))
        .last()
        .map(|e| Slot(e.base().id))
        .expect("the door");
    assert_eq!(b.doc.dimension_sets()[0].members.len(), 4);
    (b, wall, door)
}

fn texts(b: &Bench) -> Vec<String> {
    b.session
        .preview(&Default::default())
        .map(|p| p.texts.into_iter().map(|t| t.text).collect())
        .unwrap_or_default()
}

/// docs/adr/0695's open gate: the door dragged by its centre grip shows the dimensions that follow it, as they would
/// be, before anything is written.
#[test]
fn an_opening_grip_previews_the_dimensions_that_follow_it() {
    let (mut b, _, door) = measured_door();
    b.selection.set([door]);
    b.click(5., 0.);
    assert!(b.session.grip_active());
    let before = b.doc.to_snapshot_v2();
    b.move_to(8., 0.);
    let shown = texts(&b);
    for value in ["7.500", "1.000", "11.500"] {
        assert!(shown.iter().any(|t| t == value), "{value} in {shown:?}");
    }
    assert_eq!(b.doc.to_snapshot_v2(), before, "the preview writes nothing");
    b.click(8., 0.);
    assert!(!b.session.grip_active());
    assert_eq!(b.doc.dimension_sets()[0].members.len(), 4);
}

/// A wall's end dragged by its grip: its dimension set's members follow in the preview.
#[test]
fn a_wall_grip_previews_the_dimensions_that_follow_it() {
    let (mut b, wall, _) = measured_door();
    b.selection.set([wall]);
    b.click(20., 0.);
    assert!(b.session.grip_active());
    let before = b.doc.to_snapshot_v2();
    b.move_to(22., 0.);
    let shown = texts(&b);
    for value in ["16.500", "22.000"] {
        assert!(shown.iter().any(|t| t == value), "{value} in {shown:?}");
    }
    assert_eq!(b.doc.to_snapshot_v2(), before, "the preview writes nothing");
}

/// A row whose values, each rounded from its own measurement, add up to other than the overall reads (docs/adr/0730 §3):
/// said in the preview and in a notice, never spread over the parts.
#[test]
fn a_row_whose_values_do_not_add_up_to_the_overall_is_said() {
    let mut b = Bench::new("wall");
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(settings);
    b.start("wall");
    b.click(0., 0.);
    b.click(3.0012, 0.);
    b.confirm();
    b.confirm();
    let wall = Slot(b.newest().base().id);
    let host = b.doc.uid(wall).unwrap().to_string();
    let layer = b.newest().base().layer_id.clone();
    let made = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut b.doc),
        "cad.openings.create",
        kentos_commands::CallOp::Execute,
        serde_json::json!({"layerId": layer, "openings": [{"host": host, "at": 1.5006, "width": 1.0004}]}),
    )
    .unwrap();
    assert_eq!(made["status"], "completed", "{made}");
    b.selection.set([wall]);
    b.start("wallDimensions");
    b.move_to(1.5, 1.5);
    let lines = tag(&b);
    assert!(
        lines.iter().any(|l| l == "Yuvarlama · Açıklıklar: parçalar 3.000 · toplam 3.001"),
        "{lines:?}"
    );
    let settings = b.session.settings(&Default::default());
    let notice = settings.iter().find(|r| r.key == "Z").expect("the rounding said");
    assert!(notice.value.starts_with("Açıklıklar: parçalar 3.000 · toplam 3.001."), "{}", notice.value);
}
