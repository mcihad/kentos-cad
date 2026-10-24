//! The separate rectangular-wall tool keeps the old rectangle/free-wall
//! interaction intact. These tests give user input, inspect the semantic
//! wall and its undo, and never take expected geometry from a tool run.

use crate::common::{Bench, rel};
use kentos_commands::{DrawingUnit, Entity, WallJustify, Workspace};

fn wall(b: &Bench) -> &kentos_commands::WallEntity {
    let Entity::Wall(w) = b.newest() else {
        panic!("a semantic wall")
    };
    w
}

fn local(unit: DrawingUnit) -> Bench {
    let mut b = Bench::new("wallRectangle");
    let mut settings = b.doc.settings().clone();
    settings.srid = 0;
    settings.workspace = Some(Workspace::Cad);
    settings.drawing_unit = Some(unit);
    b.doc.set_settings(settings);
    b
}

#[test]
fn the_plan_previews_a_real_wall_without_writing_and_one_click_finishes_it() {
    let mut b = local(DrawingUnit::M);
    let revision = b.doc.revision();
    b.click(0.0, 0.0);
    b.move_to(4.0, 3.0);
    let preview = b.session.preview(&Default::default()).expect("a preview");
    assert!(
        !preview.areas.is_empty(),
        "actual wall body, not just a reference box"
    );
    assert!(preview.tag.as_ref().unwrap().lines[0].contains("İç net"));
    assert_eq!(b.doc.len(), 0);
    assert_eq!(b.doc.revision(), revision);
    b.click(4.0, 3.0);
    assert_eq!(b.doc.len(), 1);
    assert_eq!(b.points(), 0);
    let w = wall(&b);
    assert!(w.wall.closed);
    assert_eq!(w.wall.justify, WallJustify::Left);
    assert_eq!(w.wall.width, 0.2);
    assert_eq!(
        w.wall.pts.iter().copied().map(rel).collect::<Vec<_>>(),
        [[0.0, 0.0], [4.0, 0.0], [4.0, 3.0], [0.0, 3.0]]
    );
    assert!(b.undo_step());
    assert_eq!(b.doc.len(), 0, "one undo removes the complete wall");
    assert!(b.doc.redo().is_some());
    assert_eq!(b.doc.len(), 1);
}

#[test]
fn reference_choices_do_not_change_the_free_walls_justification_or_offset() {
    let mut b = local(DrawingUnit::M);
    b.memory.wall_justify = WallJustify::Right;
    b.memory.wall_offset = 0.37;
    let choices = b.session.option_choices("R");
    assert_eq!(
        choices.iter().map(|c| c.label.as_str()).collect::<Vec<_>>(),
        ["İç net", "Dış ölçü", "Eksen"]
    );
    for (reference, justify) in [
        ("outer", WallJustify::Right),
        ("center", WallJustify::Center),
    ] {
        assert!(b.run(|s, cx| s.choose_option("R", reference, cx)));
        b.click(0.0, 0.0);
        b.click(4.0, 3.0);
        assert_eq!(wall(&b).wall.justify, justify);
        assert_eq!(wall(&b).wall.offset, 0.0);
    }
    assert_eq!(b.memory.wall_justify, WallJustify::Right);
    assert_eq!(b.memory.wall_offset, 0.37);
}

#[test]
fn dimensions_use_the_projects_unit_and_have_an_explicit_return_to_corners() {
    let mut b = local(DrawingUnit::Mm);
    assert!(b.type_text("B"));
    assert!(b.type_text("4000,3000"));
    b.click(0.0, 0.0);
    b.click(-8.0, 5.0);
    let pts: Vec<_> = wall(&b).wall.pts.iter().copied().map(rel).collect();
    assert_eq!(pts, [[0.0, 0.0], [0.0, 3.0], [-4.0, 3.0], [-4.0, 0.0]]);
    assert!(b.options().contains(&"K"));
    assert!(b.type_text("K"));
    b.click(6.0, 0.0);
    b.click(8.0, 1.0);
    assert_eq!(
        wall(&b)
            .wall
            .pts
            .iter()
            .copied()
            .map(rel)
            .collect::<Vec<_>>(),
        [[6.0, 0.0], [8.0, 0.0], [8.0, 1.0], [6.0, 1.0]]
    );
}

#[test]
fn rotation_is_local_to_this_tool_and_bad_size_keeps_asking() {
    let mut b = local(DrawingUnit::M);
    b.memory.rect_rotation = 0.7;
    assert!(b.type_text("D"));
    assert!(b.type_text("30"));
    assert!(b.type_text("B"));
    assert!(b.type_text("-4,3"));
    assert!(b.session.prompt().text().contains("uzunluk ve genişliği"));
    assert!(b.type_text("4m,3m"));
    b.click(0.0, 0.0);
    b.click(4.0, 4.0);
    assert_eq!(b.memory.rect_rotation, 0.7);
    let p = wall(&b)
        .wall
        .pts
        .iter()
        .copied()
        .map(rel)
        .collect::<Vec<_>>();
    assert!((p[1][0] - 4.0 * 3.0f64.sqrt() / 2.0).abs() < 1e-8);
    assert!((p[1][1] - 2.0).abs() < 1e-8);
}

#[test]
fn enter_and_escape_never_supply_an_unfinished_second_corner() {
    let mut b = local(DrawingUnit::M);
    b.click(0.0, 0.0);
    b.move_to(4.0, 3.0);
    b.confirm();
    assert_eq!(b.doc.len(), 0);
    assert_eq!(b.session.tool_id(), "select");
    b.start("wallRectangle");
    b.click(0.0, 0.0);
    assert!(b.type_text("B"));
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(b.points(), 1, "Escape leaves the parameter question first");
    b.run(|s, cx| s.cancel(cx));
    assert_eq!(b.doc.len(), 0);
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn fixed_style_width_is_visible_and_does_not_silently_take_an_ignored_width() {
    let mut b = local(DrawingUnit::M);
    // Built-in styles have an elastic component. Make a genuinely fixed
    // one in this test document, without changing any product style.
    let mut style = kentos_commands::builtin_wall_styles().remove(3);
    style.id = "01900000-0000-7000-8000-000000000021".into();
    style.name = "Sabit beton 20".into();
    style.components[0].thickness = Some(0.2);
    style.width = None;
    let mut settings = b.doc.settings().clone();
    settings.wall_styles.push(style);
    b.doc.set_settings(settings);
    assert!(b.type_text("S"));
    assert!(b.type_text("Sabit beton 20"));
    let remembered = b.memory.wall_width;
    assert!(b.type_text("E"));
    assert!(b.last_text().unwrap().contains("kalınlığı sabittir"));
    assert_eq!(b.memory.wall_width, remembered);
    b.click(0.0, 0.0);
    b.click(4.0, 3.0);
    assert_eq!(wall(&b).wall.width, 0.2);
}

#[test]
fn a_builtin_elastic_style_keeps_its_width_editable() {
    let mut b = local(DrawingUnit::M);
    assert!(b.type_text("S"));
    assert!(b.type_text("Dış duvar 25"));
    assert!(b.type_text("E"));
    assert!(b.session.prompt().text().contains("kalınlığını yazın"));
    assert!(b.type_text("0.35m"));
    b.click(0.0, 0.0);
    b.click(4.0, 3.0);
    assert_eq!(wall(&b).wall.width, 0.35);
}

#[test]
fn imperial_lengths_in_a_metre_project_reach_the_semantic_wall() {
    let mut b = local(DrawingUnit::M);
    assert!(b.type_text("E"));
    assert!(b.type_text("8\""));
    assert!(b.type_text("B"));
    assert!(b.type_text("12',10'"));
    b.click(0.0, 0.0);
    b.click(6.0, 5.0);
    let w = wall(&b);
    assert!((w.wall.width - 0.2032).abs() < 1e-12);
    let p: Vec<_> = w.wall.pts.iter().copied().map(rel).collect();
    assert!((p[1][0] - 3.6576).abs() < 1e-8);
    assert!((p[1][1]).abs() < 1e-8);
    assert!((p[2][0] - 3.6576).abs() < 1e-8);
    assert!((p[2][1] - 3.048).abs() < 1e-8);
}

#[test]
fn changing_reference_size_and_rotation_after_the_first_corner_only_replans() {
    let mut b = local(DrawingUnit::M);
    let revision = b.doc.revision();
    b.click(0.0, 0.0);
    b.move_to(4.0, 3.0);
    assert!(
        !b.session
            .preview(&Default::default())
            .unwrap()
            .areas
            .is_empty()
    );
    assert!(b.run(|s, cx| s.choose_option("R", "outer", cx)));
    for value in ["B", "5m,4m", "D", "30"] {
        assert!(b.type_text(value), "{value}");
        assert_eq!(b.doc.len(), 0);
        assert_eq!(b.doc.revision(), revision, "{value} must only replan");
        assert_eq!(b.points(), 1, "the first corner stays selected");
    }
    b.move_to(6.0, 5.0);
    let preview = b.session.preview(&Default::default()).unwrap();
    assert!(!preview.areas.is_empty());
    assert_eq!(preview.tag.unwrap().lines[0], "Boy referansı: Dış ölçü");
    assert_eq!(b.doc.len(), 0);
    b.click(6.0, 5.0);
    assert_eq!(b.doc.len(), 1);
    assert_eq!(wall(&b).wall.justify, WallJustify::Right);
    let p: Vec<_> = wall(&b).wall.pts.iter().copied().map(rel).collect();
    assert!((p[1][0] - 5.0 * 3.0f64.sqrt() / 2.0).abs() < 1e-8);
    assert!((p[1][1] - 2.5).abs() < 1e-8);
}

#[test]
fn numeric_preview_keeps_length_width_order_in_every_quadrant() {
    let mut b = local(DrawingUnit::M);
    assert!(b.type_text("B"));
    assert!(b.type_text("4m,3m"));
    b.click(0.0, 0.0);
    for toward in [[5.0, 4.0], [-5.0, 4.0], [-5.0, -4.0], [5.0, -4.0]] {
        b.move_to(toward[0], toward[1]);
        let preview = b.session.preview(&Default::default()).unwrap();
        assert!(!preview.areas.is_empty());
        assert_eq!(
            preview.tag.unwrap().lines[1],
            "4.000 × 3.000 m",
            "length and width stay in the entered order toward {toward:?}"
        );
        assert_eq!(b.doc.len(), 0);
    }
}

#[test]
fn a_direct_tool_call_cannot_put_architectural_walls_on_a_typed_gis_layer() {
    for geometry in kentos_commands::LAYER_GEOMETRIES {
        let mut b = local(DrawingUnit::M);
        let mut settings = b.doc.settings().clone();
        settings.workspace = Some(Workspace::Gis);
        b.doc.set_settings(settings);
        let layer = b.doc.layers().active().to_owned();
        b.doc
            .set_layer_geometry(&layer, Some(geometry), "Katman türü")
            .unwrap();
        let before = b.doc.to_snapshot_v2();
        b.click(0.0, 0.0);
        b.click(4.0, 3.0);
        assert_eq!(b.doc.to_snapshot_v2(), before, "{geometry:?}");
        let refused = b.doc.refused_calls().last().expect("the refused write");
        assert_eq!(refused.command, "cad.walls.rectangle");
        assert_eq!(
            refused.error.as_ref().unwrap().code,
            "layer_geometry_mismatch",
            "{geometry:?}"
        );
    }
}
