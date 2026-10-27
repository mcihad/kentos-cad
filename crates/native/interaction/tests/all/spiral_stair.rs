//! Döner merdiven (docs/adr/0709): its centre, its walking line's start, then the pointer going round the centre turns
//! it; with Yay rıhtlara göre it ends at its last riser however far the pointer went, one undo takes it back. The way
//! round is the pointer's (clockwise too), a turn may be typed in degrees with Yay tıklanan yerde, too small a radius
//! says why and writes nothing, and Merdiven's Biçim switches to it.

use crate::common::{Bench, E, N};
use kentos_commands::{DrawingUnit, Entity, Workspace};

fn bench(tool: &str) -> Bench {
    let mut b = Bench::new(tool);
    let mut settings = b.doc.settings().clone();
    settings.workspace = Some(Workspace::Cad);
    settings.srid = 0;
    settings.drawing_unit = Some(DrawingUnit::M);
    settings.angle_unit = kentos_commands::AngleUnit::Deg;
    b.doc.set_settings(settings);
    b.start(tool);
    b
}

fn stair(b: &Bench) -> kentos_commands::StairFields {
    match b.newest() {
        Entity::Stair(s) => s.stair.clone(),
        other => panic!("a stair, not {other:?}"),
    }
}

/// The pointer round the centre (0, 0) at 1.5 m from `from` to `to` degrees, a step at a time.
fn round(b: &mut Bench, from: f64, to: f64) {
    let steps = 12;
    for k in 0..=steps {
        let a = (from + (to - from) * f64::from(k) / f64::from(steps)).to_radians();
        b.move_to(1.5 * a.cos(), 1.5 * a.sin());
    }
}

#[test]
fn the_pointer_turns_it_and_it_ends_at_its_last_riser() {
    let mut b = bench("stairCurved");
    b.click(0., 0.);
    b.click(1.5, 0.);
    round(&mut b, 0.0, 120.0);
    let before = b.doc.to_snapshot_v2();
    let preview = b
        .session
        .preview(&Default::default())
        .expect("the plan drawn");
    assert!(
        preview.strokes.len() > 18,
        "the risers drawn: {}",
        preview.strokes.len()
    );
    let tag = preview.tag.expect("the tag");
    assert!(
        tag.lines.iter().any(|l| l.contains("iç kenarda basamak")),
        "{:?}",
        tag.lines
    );
    assert_eq!(b.doc.to_snapshot_v2(), before, "the preview writes nothing");
    b.click(
        1.5 * 120f64.to_radians().cos(),
        1.5 * 120f64.to_radians().sin(),
    );
    let s = stair(&b);
    assert_eq!(s.risers, 18, "3.00 m under 0.175 m risers");
    let sweep: f64 = 17.0 * 0.28 / 1.5;
    let b0 = s.bulges.as_ref().expect("curved")[0];
    assert!(
        (b0 - (sweep / 4.0).tan()).abs() < 1e-9,
        "ends at its last riser: {b0}"
    );
    let end = s.pts[1];
    assert!(
        (end.x - (E + 1.5 * sweep.cos())).abs() < 1e-9
            && (end.y - (N + 1.5 * sweep.sin())).abs() < 1e-9,
        "{end:?}"
    );
    assert!(
        b.session
            .preview(&Default::default())
            .is_none_or(|p| p.strokes.is_empty()),
        "ready for the next"
    );
    assert_eq!(b.doc.undo().as_deref(), Some("Merdiven"));
    assert!(!b.doc.entities().any(|e| matches!(e, Entity::Stair(_))));
}

#[test]
fn the_other_way_round_turns_it_clockwise() {
    let mut b = bench("stairCurved");
    b.click(0., 0.);
    b.click(1.5, 0.);
    round(&mut b, 0.0, -60.0);
    b.click(
        1.5 * (-60f64).to_radians().cos(),
        1.5 * (-60f64).to_radians().sin(),
    );
    let s = stair(&b);
    assert!(s.bulges.as_ref().expect("curved")[0] < 0.0, "clockwise");
}

#[test]
fn a_typed_turn_with_the_clicked_end_goes_past_a_half_turn() {
    let mut b = bench("stairCurved");
    assert!(b.run(|s, cx| s.configure("U", "clicked", cx)));
    b.click(0., 0.);
    b.click(1.5, 0.);
    assert!(b.run(|s, cx| s.input("270", cx)));
    let s = stair(&b);
    let b0 = s.bulges.as_ref().expect("curved")[0];
    assert!(
        (b0 - (270f64.to_radians() / 4.0).tan()).abs() < 1e-9,
        "three quarters round: {b0}"
    );
    // Going round by the pointer past a half turn keeps its way too.
    b.click(0., 0.);
    b.click(1.5, 0.);
    round(&mut b, 0.0, 300.0);
    b.click(
        1.5 * 300f64.to_radians().cos(),
        1.5 * 300f64.to_radians().sin(),
    );
    let b1 = stair(&b).bulges.expect("curved")[0];
    assert!(
        (b1 - (300f64.to_radians() / 4.0).tan()).abs() < 1e-6,
        "300° counter-clockwise, not 60° back: {b1}"
    );
}

#[test]
fn too_small_a_radius_says_why_and_writes_nothing() {
    let mut b = bench("stairCurved");
    b.click(0., 0.);
    b.click(0.4, 0.);
    round(&mut b, 0.0, 90.0);
    let tag = b
        .session
        .preview(&Default::default())
        .and_then(|p| p.tag)
        .expect("the tag");
    assert!(
        tag.lines.iter().any(|l| l.contains("yarıçapı")),
        "{:?}",
        tag.lines
    );
    b.click(0., 1.5);
    assert!(!b.doc.entities().any(|e| matches!(e, Entity::Stair(_))));
}

#[test]
fn merdiven_s_shape_switches_to_curved() {
    let mut b = bench("stair");
    assert!(b.run(|s, cx| s.configure("B", "curved", cx)));
    let prompt = b.session.prompt();
    assert!(prompt.step.contains("merkezine"), "{}", prompt.step);
    assert_eq!(prompt.tool, Some("Döner merdiven"));
    b.click(0., 0.);
    b.click(1.5, 0.);
    round(&mut b, 0.0, 90.0);
    b.click(0., 1.5);
    assert!(stair(&b).bulges.is_some(), "a curved stair from Merdiven");
}
