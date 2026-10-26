use crate::common::Bench;
use kentos_commands::{Entity, OpeningAnchor, OpeningHand, OpeningSide};
use kentos_domain::Slot;

fn set(b: &mut Bench, k: &str, v: &str) -> bool {
    b.run(|s, cx| s.configure(k, v, cx))
}
fn wall(tool: &str) -> Bench {
    let mut b = Bench::new("wall");
    b.click(0.0, 0.0);
    b.click(10.0, 0.0);
    b.confirm();
    b.confirm();
    b.start(tool);
    b
}
#[test]
fn visible_end_and_hinge_reference_are_previewed_then_written_through_the_same_command() {
    let mut b = wall("door");
    let before = b.doc.to_snapshot_v2();
    for (key, value) in [
        ("E", "1 m"),
        ("U", "2 m"),
        ("F", "end"),
        ("R", "hinge"),
        ("M", "start"),
    ] {
        assert!(set(&mut b, key, value));
    }
    b.move_to(4., 0.3);
    let p = b.session.preview(&Default::default()).unwrap();
    assert!(
        p.strokes.iter().any(|s| s.dash.is_some()),
        "highlight the chosen host"
    );
    let text = p.tag.unwrap().lines.join(" ");
    assert!(
        text.contains("Baş") && text.contains("Son") && text.contains("Menteşe"),
        "{text}"
    );
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(4., 0.3);
    let Entity::Opening(o) = b.newest() else {
        panic!()
    };
    assert_eq!(o.opening.at, 8.5);
    assert_eq!(o.opening.width, 1.0);
    assert_eq!(o.opening.anchor, OpeningAnchor::Center);
    assert_eq!(
        b.doc.journal().last().unwrap().command.as_deref(),
        Some("cad.openings.create")
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Kapı"));
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
}
#[test]
fn two_jamb_method_keeps_a_host_and_first_point_on_invalid_second_click_and_cancel_writes_nothing()
{
    let mut b = wall("window");
    assert!(set(&mut b, "B", "two"));
    assert!(
        !b.session
            .settings(&Default::default())
            .iter()
            .any(|f| matches!(f.key, "E" | "U" | "F" | "R"))
    );
    assert!(
        !b.options()
            .iter()
            .any(|k| matches!(*k, "E" | "U" | "F" | "R"))
    );
    let before = b.doc.to_snapshot_v2();
    b.click(2., 0.2);
    assert_eq!(b.points(), 1);
    b.move_to(3.2, 0.2);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    let text = b
        .session
        .preview(&Default::default())
        .unwrap()
        .tag
        .unwrap()
        .lines
        .join(" ");
    assert!(text.contains("1.200"), "{text}");
    b.click(2., 0.2);
    assert_eq!(b.points(), 1);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(b.undo_step());
    assert_eq!(b.points(), 0);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(2., 0.2);
    b.click(3.2, 0.2);
    assert_eq!(b.points(), 0);
    let Entity::Opening(o) = b.newest() else {
        panic!()
    };
    assert!((o.opening.width - 1.2).abs() < 1e-9);
    assert!((o.opening.at - 2.6).abs() < 1e-9);
    assert_eq!(b.doc.len(), 2);
    assert_eq!(b.doc.undo().as_deref(), Some("Pencere"));
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
}
#[test]
fn sampling_takes_only_type_and_width_and_cannot_convert_a_door_tool_to_a_window() {
    let mut b = wall("door");
    assert!(set(&mut b, "E", "1.1 m"));
    assert!(set(&mut b, "M", "end"));
    assert!(set(&mut b, "Y", "Sağ"));
    b.click(2., 0.2);
    let source = b.doc.uid(Slot(b.newest().base().id)).unwrap().to_string();
    b.start("door");
    assert!(set(&mut b, "B", "copy"));
    let before = b.doc.to_snapshot_v2();
    assert!(set(&mut b, "A", &source));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.click(5., 0.2);
    let Entity::Opening(o) = b.newest() else {
        panic!()
    };
    assert_eq!(o.opening.width, 1.1);
    // Sampling is explicitly type + width, not a hidden copy of host, hand or side.
    assert_eq!(o.opening.hand, OpeningHand::Start);
    assert_eq!(o.opening.side, OpeningSide::Left);
    b.start("window");
    let before = b.doc.to_snapshot_v2();
    assert!(!set(&mut b, "A", &source));
    assert!(!set(&mut b, "T", "kentos-kapi-90"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn typed_second_point_and_finish_after_one_point_do_not_leave_partial_openings() {
    let mut b = wall("opening");
    assert!(set(&mut b, "B", "two"));
    let before = b.doc.to_snapshot_v2();
    b.click(2., 0.2);
    b.confirm();
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert_eq!(b.points(), 0);
    b.start("opening");
    assert!(set(&mut b, "B", "two"));
    b.click(2., 0.2);
    assert!(b.type_text("@1.2,0"));
    let Entity::Opening(o) = b.newest() else {
        panic!()
    };
    assert!((o.opening.width - 1.2).abs() < 1e-9);
}

#[test]
fn a_new_precision_opening_can_be_moved_by_its_existing_centre_grip_and_undone() {
    let mut b = wall("door");
    for (key, value) in [("E", "1 m"), ("U", "2 m"), ("F", "end"), ("R", "hinge")] {
        assert!(set(&mut b, key, value));
    }
    b.click(4.0, 0.3);
    let slot = Slot(b.newest().base().id);
    let uid = b.doc.uid(slot).unwrap().to_string();
    let before = b.doc.to_snapshot_v2();
    b.confirm();
    b.selection.set([slot]);
    b.click(8.5, 0.0);
    assert!(b.session.grip_active());
    b.move_to(7.5, 0.0);
    assert_eq!(b.doc.to_snapshot_v2(), before, "grip preview is read-only");
    b.click(7.5, 0.0);
    assert!(!b.session.grip_active());
    let Entity::Opening(o) = b.doc.get(slot).unwrap() else {
        panic!("opening")
    };
    assert_eq!(b.doc.uid(slot).unwrap().to_string(), uid);
    assert_eq!((o.opening.at, o.opening.width), (7.5, 1.0));
    assert_eq!(
        b.doc.journal().last().unwrap().command.as_deref(),
        Some("cad.openings.edit")
    );
    assert!(o.opening.broken.is_none());
    assert!(b.doc.wall_line_of(&o.opening.host).is_some());
    assert!(b.doc.undo().is_some());
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
    assert!(b.doc.redo().is_some());
    let Entity::Opening(o) = b.doc.get(slot).unwrap() else {
        panic!("opening")
    };
    assert_eq!(o.opening.at, 7.5);
}

#[test]
fn an_opening_grip_refuses_endpoint_overflow_and_can_copy_without_moving_the_source() {
    let mut b = wall("door");
    assert!(set(&mut b, "E", "1 m"));
    b.click(8.5, 0.2);
    let slot = Slot(b.newest().base().id);
    b.confirm();
    b.selection.set([slot]);
    let before = b.doc.to_snapshot_v2();
    b.click(8.5, 0.0);
    b.move_to(9.8, 0.0);
    b.click(9.8, 0.0);
    assert_eq!(
        b.doc.to_snapshot_v2(),
        before,
        "off-wall placement writes nothing"
    );
    assert!(b.last_text().unwrap().contains("duvar"));
    b.click(8.5, 0.0);
    assert!(b.type_text("K"));
    b.move_to(5.0, 0.0);
    assert_eq!(
        b.doc.to_snapshot_v2(),
        before,
        "copy preview writes nothing"
    );
    b.click(5.0, 0.0);
    assert!(b.session.grip_active(), "copy stays hot");
    assert_eq!(b.doc.len(), 3);
    let Entity::Opening(source) = b.doc.get(slot).unwrap() else {
        panic!("opening")
    };
    assert_eq!(source.opening.at, 8.5);
    let Entity::Opening(copy) = b.newest() else {
        panic!("opening")
    };
    assert_eq!(copy.opening.at, 5.0);
    assert!(copy.opening.broken.is_none());
    assert!(b.doc.undo().is_some());
    assert_eq!(b.doc.to_snapshot_v2().entities, before.entities);
}

#[test]
fn a_mixed_hot_grip_set_cannot_silently_skip_the_opening_and_move_only_its_wall() {
    let mut b = wall("door");
    assert!(set(&mut b, "E", "1 m"));
    b.click(8.5, 0.2);
    let opening = Slot(b.newest().base().id);
    b.confirm();
    b.selection.set([Slot(1), opening]);
    for (x, y) in [(0.0, 0.0), (8.5, 0.0)] {
        b.shift = true;
        b.move_to(x, y);
        b.click(x, y);
        b.shift = false;
    }
    assert_eq!(b.session.hot_grips().len(), 2);
    let before = b.doc.to_snapshot_v2();
    b.move_to(8.5, 0.0);
    b.click(8.5, 0.0);
    b.move_to(7.5, 0.0);
    b.click(7.5, 0.0);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert!(b.last_text().unwrap().contains("hiçbir nesne"));
}

#[test]
fn the_visible_placement_mode_matches_the_distance_and_zero_can_place_a_flush_jamb() {
    let mut b = wall("door");
    assert!(set(&mut b, "E", "1m"));
    assert!(set(&mut b, "O", "distance"));
    assert!(set(&mut b, "F", "start"));
    assert!(set(&mut b, "U", "0 m"));
    let fields = b.session.settings(&Default::default());
    assert_eq!(
        fields.iter().find(|f| f.key == "O").unwrap().value,
        "Uçtan uzaklık"
    );
    b.click(4., 0.2);
    let Entity::Opening(o) = b.newest() else {
        panic!()
    };
    assert_eq!(o.opening.at, 0.5);
    assert!(set(&mut b, "O", "free"));
    let fields = b.session.settings(&Default::default());
    assert_eq!(
        fields.iter().find(|f| f.key == "O").unwrap().value,
        "Serbest"
    );
    assert!(b.type_text("U"));
    assert!(b.type_text("2m"));
    assert!(b.type_text("U"));
    assert!(b.type_text("0"));
    let fields = b.session.settings(&Default::default());
    assert_eq!(
        fields.iter().find(|f| f.key == "O").unwrap().value,
        "Serbest"
    );
}

#[test]
fn centering_suppresses_a_remembered_corner_distance_and_the_legacy_toggle_restores_it() {
    let mut b = wall("door");
    assert!(set(&mut b, "E", "1m"));
    assert!(set(&mut b, "U", "2m"));
    assert!(set(&mut b, "O", "center"));
    b.click(4., 0.2);
    let Entity::Opening(o) = b.newest() else {
        panic!()
    };
    assert_eq!(o.opening.at, 5.0);
    assert!(b.doc.undo().is_some());
    assert!(b.type_text("O"));
    b.click(4., 0.2);
    let Entity::Opening(o) = b.newest() else {
        panic!()
    };
    assert_eq!(o.opening.at, 2.5);
}
