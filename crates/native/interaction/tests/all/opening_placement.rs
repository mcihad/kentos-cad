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
