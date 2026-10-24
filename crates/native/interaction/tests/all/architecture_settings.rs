use crate::common::{Bench, rel};
use kentos_commands::{DrawingUnit, Entity, Workspace};
fn local(id: &str) -> Bench {
    let mut b = Bench::new(id);
    let mut s = b.doc.settings().clone();
    s.srid = 0;
    s.workspace = Some(Workspace::Cad);
    s.drawing_unit = Some(DrawingUnit::M);
    b.doc.set_settings(s);
    b
}
fn set(b: &mut Bench, k: &str, v: &str) -> bool {
    b.run(|s, cx| s.configure(k, v, cx))
}
#[test]
fn settings_never_edit_the_selection_or_write_a_draft_and_keep_the_first_corner() {
    let mut b = local("wallRectangle");
    b.click(0.0, 0.0);
    b.move_to(4.0, 3.0);
    let before = b.doc.to_snapshot_v2();
    assert!(set(&mut b, "E", "250mm"));
    assert!(set(&mut b, "B", "4m,3m"));
    assert!(set(&mut b, "R", "inner"));
    assert!(!set(&mut b, "E", "-5m"));
    assert!(!set(&mut b, "B", "4m,0m"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert_eq!(b.points(), 1);
    b.click(6.0, 5.0);
    let Entity::Wall(w) = b.newest() else {
        panic!()
    };
    assert_eq!(w.wall.width, 0.25);
    assert_eq!(rel(w.wall.pts[2])[0], 4.0);
    assert_eq!(rel(w.wall.pts[2])[1], 3.0);
}
#[test]
fn wall_configuration_keeps_old_path_and_arc_options_and_fixed_style_width() {
    let mut b = local("wall");
    b.click(0.0, 0.0);
    assert!(set(&mut b, "E", "250mm"));
    assert!(set(&mut b, "H", "Sol"));
    assert!(set(&mut b, "Ö", "100mm"));
    assert_eq!(b.points(), 1);
    b.click(5.0, 0.0);
    b.confirm();
    let Entity::Wall(w) = b.newest() else {
        panic!()
    };
    assert_eq!(w.wall.width, 0.25);
    assert_eq!(w.wall.offset, 0.1);
    let mut b = local("wallRectangle");
    let mut fixed = kentos_commands::builtin_wall_styles().remove(3);
    fixed.id = "01900000-0000-7000-8000-000000000022".into();
    fixed.name = "Sabit 20".into();
    fixed.components[0].thickness = Some(0.2);
    fixed.width = None;
    let mut settings = b.doc.settings().clone();
    settings.wall_styles.push(fixed);
    b.doc.set_settings(settings);
    assert!(set(&mut b, "S", "Sabit 20"));
    let before = b.memory.wall_width;
    assert!(!set(&mut b, "E", "300mm"));
    assert_eq!(b.memory.wall_width, before);
}
#[test]
fn opening_and_space_defaults_are_visible_and_only_apply_to_future_objects() {
    for tool in ["door", "window", "opening"] {
        let mut b = local(tool);
        let before = b.doc.to_snapshot_v2();
        assert!(set(&mut b, "E", "900mm"));
        assert!(set(&mut b, "U", "200mm"));
        assert!(set(&mut b, "O", "center"));
        assert!(!set(&mut b, "E", "0"));
        let fields = b.session.settings(&Default::default());
        assert!(
            fields
                .iter()
                .any(|f| f.key == "E" && f.value.contains("0.900"))
        );
        assert!(
            !fields.iter().any(|f| f.key == "U"),
            "centred placement does not use a corner distance"
        );
        assert_eq!(b.doc.to_snapshot_v2(), before);
    }
    let mut b = local("space");
    assert!(set(&mut b, "N", "Z-01"));
    assert!(set(&mut b, "A", "G"));
    assert!(set(&mut b, "Y", "Net"));
    let fields = b.session.settings(&Default::default());
    assert_eq!(fields.iter().find(|f| f.key == "A").unwrap().value, "G");
    assert_eq!(b.doc.len(), 0);
}
#[test]
fn measured_partition_is_a_preview_then_one_wall_one_undo_and_esc_is_atomic() {
    let mut b = local("wall");
    b.click(0.0, 0.0);
    b.click(8.0, 0.0);
    b.confirm();
    assert!(b.run(|s, cx| s.start("wallPartition") && {
        s.activate(cx);
        true
    }));
    assert!(set(&mut b, "U", "3m"));
    assert!(set(&mut b, "E", "150mm"));
    let before = b.doc.to_snapshot_v2();
    b.click(4.0, 0.0);
    b.move_to(4.0, 3.0);
    let preview = b.session.preview(&Default::default()).unwrap();
    assert!(!preview.areas.is_empty(), "{:?} / {:?}", preview.tag, b.log);
    assert_eq!(b.doc.to_snapshot_v2(), before);
    b.confirm();
    assert_eq!(b.doc.len(), 2);
    let Entity::Wall(w) = b.newest() else {
        panic!()
    };
    let pts = w.wall.pts.iter().copied().map(rel).collect::<Vec<_>>();
    assert!((pts[0][1] - 3.175).abs() < 1e-8);
    assert!((pts[1][1] - 3.175).abs() < 1e-8);
    assert_eq!([pts[0][0], pts[1][0]], [0.0, 8.0]);
    assert_eq!(b.doc.undo().as_deref(), Some("Duvar"));
    assert_eq!(b.doc.len(), 1);
    b.start("wallPartition");
    b.click(4.0, 0.0);
    let before = b.doc.to_snapshot_v2();
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn overlapping_source_candidates_are_visible_and_can_be_changed_before_writing() {
    let mut b = local("wall");
    b.click(0.0, 0.0);
    b.click(8.0, 0.0);
    b.confirm();
    b.confirm();
    b.start("wall");
    b.click(0.0, 0.3);
    b.click(8.0, 0.3);
    b.confirm();
    b.confirm();
    b.start("wallPartition");
    b.click(4.0, 0.15);
    let before = b.doc.to_snapshot_v2();
    let fields = b.session.settings(&Default::default());
    assert_eq!(
        fields.iter().find(|s| s.key == "C").unwrap().choices.len(),
        2
    );
    assert!(set(&mut b, "C", "1"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
    assert_eq!(b.points(), 1);
    assert!(!set(&mut b, "C", "99"));
    assert_eq!(b.doc.to_snapshot_v2(), before);
}

#[test]
fn displayed_single_lengths_and_explicit_units_apply_without_writing_or_losing_the_corner() {
    for unit in [
        DrawingUnit::Mm,
        DrawingUnit::Cm,
        DrawingUnit::M,
        DrawingUnit::In,
        DrawingUnit::Ft,
    ] {
        let mut b = local("wallRectangle");
        let mut settings = b.doc.settings().clone();
        settings.drawing_unit = Some(unit);
        b.doc.set_settings(settings);
        b.click(0., 0.);
        b.move_to(4., 3.);
        let before = b.doc.to_snapshot_v2();
        let shown = b
            .run(|s, cx| s.settings(&cx.format()))
            .into_iter()
            .find(|f| f.key == "E")
            .unwrap()
            .value;
        assert!(set(&mut b, "E", &shown), "{unit:?}: {shown}");
        let (mul, div) = unit.metres();
        assert!((b.memory.wall_width - 0.2).abs() <= 0.0005 * mul / div + 1e-12);
        for (input, expected) in [
            ("250 mm", 0.25),
            ("0.300 m", 0.3),
            ("8 in", 0.2032),
            ("1 ft", 0.3048),
        ] {
            assert!(set(&mut b, "E", input), "{unit:?}: {input}");
            assert!((b.memory.wall_width - expected).abs() < 1e-12);
        }
        assert!(!set(&mut b, "E", "1 2 m"));
        assert!((b.memory.wall_width - 0.3048).abs() < 1e-12);
        assert_eq!(b.doc.to_snapshot_v2(), before);
        assert_eq!(b.points(), 1);
        b.click(4., 3.);
        let Entity::Wall(wall) = b.newest() else {
            panic!("a wall")
        };
        assert!((wall.wall.width - 0.3048).abs() < 1e-12);
        assert_eq!(rel(wall.wall.pts[0]), [0., 0.]);
    }
}
