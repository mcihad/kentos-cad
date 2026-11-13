//! Çatı bacası and Çatı penceresi (docs/adr/8044 §6): a click on a roof writes a chimney, a vent or a roof window into
//! its parts through `cad.arch.edit`, one undo step; the roof under the click is the one the view draws whole.

use crate::common::{Bench, E, N};
use kentos_commands::{CallOp, DrawingUnit, Entity, RoofEntity, RoofPartKind, Workspace};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, registry};
use serde_json::{Value, json};

fn run(b: &mut Bench, name: &str, input: Value) -> Value {
    let answer = registry::run(
        &mut ExecutionContext::new(&mut b.doc),
        name,
        CallOp::Execute,
        input,
    )
    .expect("runs");
    assert_eq!(answer["status"], "completed", "{name}: {answer}");
    answer
}

/// Zemin kat (3 m high) and Çatı at +3.00; a 13 × 10 m hip roof at 33 % on Zemin kat, its eaves (−0.5, −0.5) to
/// (12.5, 9.5) at Zemin kat's top (+3.00): its ridge 1.65 m over them, +4.65, from (4.5, 4.5) to (7.5, 4.5). Çatı is
/// the active level: the roof plan.
fn roofed() -> Bench {
    let mut b = Bench::new("line");
    b.session.exit();
    let mut s = b.doc.settings().clone();
    s.srid = 0;
    s.workspace = Some(Workspace::Cad);
    s.drawing_unit = Some(DrawingUnit::M);
    s.arch = Some(
        serde_json::from_value(json!({"levels": [
            {"id": "zemin", "name": "Zemin kat", "elevation": 0.0, "height": 3.0},
            {"id": "cati", "name": "Çatı", "elevation": 3.0}
        ]}))
        .unwrap(),
    );
    b.doc.set_settings(s);
    let p = |x: f64, y: f64| json!({"x": E + x, "y": N + y});
    run(
        &mut b,
        "cad.roofs.create",
        json!({"layerId": "cizim", "roofs": [{
            "pts": [p(-0.5, -0.5), p(12.5, -0.5), p(12.5, 9.5), p(-0.5, 9.5)],
            "slopes": [33.0, 33.0, 33.0, 33.0], "overhang": 0.5, "arch": {"level": "zemin"}
        }]}),
    );
    b.level = Some("cati".into());
    b
}

fn the_roof(b: &Bench) -> RoofEntity {
    b.doc
        .entities()
        .find_map(|e| match e {
            Entity::Roof(r) => Some(r.clone()),
            _ => None,
        })
        .expect("the roof")
}

#[test]
fn a_chimney_goes_on_the_roof_80_cm_over_its_ridge_in_one_undo_step() {
    let mut b = roofed();
    b.start("roofChimney");
    assert_eq!(b.options(), ["T", "E", "D", "Y", "Enter"]);
    b.click(9.0, 6.0);
    let r = the_roof(&b);
    assert_eq!(r.roof.parts.len(), 1);
    let c = &r.roof.parts[0];
    assert_eq!(c.kind, RoofPartKind::Chimney);
    assert_eq!((c.at.x - E, c.at.y - N), (9.0, 6.0));
    assert_eq!((c.width, c.depth), (0.4, 0.4));
    // An upright building's chimney is not turned; its rise the rule's 80 cm (Yangın Yön. 58/2), written.
    assert_eq!(c.rotation, None);
    assert_eq!(c.rise, Some(0.8));
    assert_eq!(b.last_level(), Some(Level::Success));
    assert!(
        b.last_text()
            .is_some_and(|t| t.starts_with("Duman bacası kondu; tepesi +5.45 (mahyanın")),
        "{:?}",
        b.last_text()
    );
    // A second one keeps the first: the parts are written whole.
    b.click(3.0, 3.0);
    assert_eq!(the_roof(&b).roof.parts.len(), 2);
    // One undo step each.
    assert_eq!(b.doc.undo().as_deref(), Some("Mimari nesneyi düzenle"));
    assert_eq!(the_roof(&b).roof.parts.len(), 1);
}

#[test]
fn a_vent_over_60_cm_is_written_and_warned_of_by_paiy_40() {
    let mut b = roofed();
    b.start("roofChimney");
    assert!(b.run(|s, cx| s.configure("T", "vent", cx)));
    assert!(b.run(|s, cx| s.configure("Y", "0.9", cx)));
    b.move_to(9.0, 6.0);
    let preview = b.run(|s, cx| s.preview(&cx.format())).expect("its preview");
    let tag = preview.tag.expect("its tag").lines;
    assert!(tag.iter().any(|l| l.starts_with("PAİY 40/5")), "{tag:?}");
    assert_eq!(preview.tag_tone, kentos_interaction::Tone::Danger);
    b.click(9.0, 6.0);
    let v = &the_roof(&b).roof.parts[0];
    assert_eq!(v.kind, RoofPartKind::Vent);
    assert_eq!(v.rise, Some(0.9));
    // Made a chimney again, its rise is the chimney's rule again.
    assert!(b.run(|s, cx| s.configure("T", "chimney", cx)));
    b.click(3.0, 3.0);
    assert_eq!(the_roof(&b).roof.parts[1].rise, Some(0.8));
}

#[test]
fn a_roof_window_lies_in_its_face_and_one_across_a_hip_is_refused() {
    let mut b = roofed();
    b.start("roofWindow");
    assert_eq!(b.options(), ["E", "Y", "Enter"]);
    // On the south face, 78 × 118 cm down its slope.
    b.click(6.0, 1.5);
    let r = the_roof(&b);
    let w = &r.roof.parts[0];
    assert_eq!(w.kind, RoofPartKind::Window);
    assert_eq!(
        (w.width, w.depth, w.rotation, w.rise),
        (0.78, 1.18, None, None)
    );
    assert!(
        b.last_text().is_some_and(
            |t| t.starts_with("Çatı penceresi kondu:") && t.ends_with("%33 eğimli yüzde.")
        ),
        "{:?}",
        b.last_text()
    );
    // Near the south-west hip it runs across it: refused, nothing written.
    b.click(0.6, 0.4);
    assert_eq!(the_roof(&b).roof.parts.len(), 1);
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert!(
        b.last_text()
            .is_some_and(|t| t.contains("tek bir yüzün içinde kalmalı")),
        "{:?}",
        b.last_text()
    );
    // Larger than Sivas's 1.20 × 1.50 m: written, warned of.
    assert!(b.run(|s, cx| s.configure("Y", "1.6", cx)));
    b.move_to(6.0, 7.5);
    let tag = b
        .run(|s, cx| s.preview(&cx.format()))
        .and_then(|p| p.tag)
        .expect("its tag")
        .lines;
    assert!(
        tag.iter().any(|l| l.starts_with("Sivas İmar Yön. 40")),
        "{tag:?}"
    );
}

#[test]
fn outside_any_roof_or_on_its_own_storeys_plan_nothing_is_put() {
    use kentos_interaction::level_view::LevelView;
    let mut b = roofed();
    b.start("roofChimney");
    b.click(30.0, 30.0);
    assert_eq!(the_roof(&b).roof.parts.len(), 0);
    assert_eq!(
        b.last_text(),
        Some("Burada çatı yok; bacayı çatının saçak çizgisinin içine koyun.")
    );
    // Zemin kat's plan draws the roof by its eaves alone: its parts are put on Çatı's plan or Bütün katlar.
    b.spatial.sync(&b.doc);
    b.spatial
        .set_level_view(&b.doc, LevelView::Active, Some("zemin".into()));
    b.click(9.0, 6.0);
    assert_eq!(the_roof(&b).roof.parts.len(), 0);
    assert_eq!(
        b.last_text(),
        Some(
            "Bu katın planında çatının yalnız saçağı görünüyor; bacaları ve pencereleri “Çatı” planında ya da Bütün \
             katlar'da konur."
        )
    );
}

/// The roof plan (docs/adr/8044 §7): every roof whole, none dashed, its storey's other objects left out; the objects on
/// no level stay.
#[test]
fn the_roof_plan_draws_every_roof_whole_and_no_storey_s_inside() {
    use kentos_interaction::level_view::{LevelView, looks, roof_plan};
    let mut b = roofed();
    // A wall on Zemin kat under the roof, and a line on no level.
    run(
        &mut b,
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [{"pts": [{"x": E, "y": N}, {"x": E + 12.0, "y": N}], "width": 0.25,
                                              "justify": "center", "arch": {"level": "zemin"}}]}),
    );
    let wall = b.newest().base().id;
    let line = b.add_line("cizim", [0.0, -3.0], [5.0, -3.0]).0;
    let roof = the_roof(&b).base.id;
    // On Zemin kat's own plan the roof is dashed; on the roof plan it is whole and the wall is out.
    assert_eq!(
        looks(&b.doc, LevelView::Active, Some("zemin")).roofs,
        vec![roof]
    );
    let plan = roof_plan(&b.doc);
    assert!(plan.roofs.is_empty(), "{plan:?}");
    assert!(plan.hidden.contains(&wall), "{plan:?}");
    assert!(
        !plan.hidden.contains(&roof) && !plan.hidden.contains(&line),
        "{plan:?}"
    );
}

/// İniş borusu (docs/adr/8044 §8): without a gutter nothing, said; with a 15 cm gutter a click 10 cm inside the south
/// eaves puts a Ø100 downpipe under it, its middle at the gutter's middle, 7.5 cm outside the eaves.
#[test]
fn a_downpipe_goes_under_the_gutter_nearest_the_click() {
    let mut b = roofed();
    b.start("roofDownpipe");
    assert_eq!(b.options(), ["Ç", "Enter"]);
    b.click(6.0, -0.4);
    assert_eq!(the_roof(&b).roof.parts.len(), 0);
    assert!(
        b.last_text()
            .is_some_and(|t| t.starts_with("İniş borusu oluğa bağlanır")),
        "{:?}",
        b.last_text()
    );
    let uid = b
        .doc
        .entities_with_uids()
        .find_map(|(u, e)| matches!(e, Entity::Roof(_)).then_some(u))
        .expect("the roof");
    run(
        &mut b,
        "cad.arch.edit",
        json!({"uids": [uid.to_string()], "operation": {"kind": "set", "gutter": 0.15}}),
    );
    b.click(6.0, -0.4);
    let roof = the_roof(&b);
    let p = &roof.roof.parts[0];
    assert_eq!(p.kind, RoofPartKind::Downpipe);
    assert!(
        (p.at.x - E - 6.0).abs() < 1e-9 && (p.at.y - N + 0.575).abs() < 1e-9,
        "{p:?}"
    );
    assert_eq!((p.width, p.depth, p.rise), (0.1, 0.1, None));
    assert_eq!(
        b.last_text(),
        Some("İniş borusu kondu: Ø100, oluğun altında.")
    );
    // Clicked at a corner it comes down the wall, not the air past the building's corner (docs/adr/8044 §8): its
    // middle the overhang, the clips' 3 cm and its radius (0.58 m) in from the eaves' end, 8 cm in from the corner.
    b.click(-0.3, -0.7);
    let roof = the_roof(&b);
    let p = &roof.roof.parts[1];
    assert!(
        (p.at.x - E - 0.08).abs() < 1e-9 && (p.at.y - N + 0.575).abs() < 1e-9,
        "{p:?}"
    );
}

/// UR-101: a second roof window clicked where the first stands is refused and said; one beside it goes in.
#[test]
fn a_roof_window_over_another_is_refused() {
    let mut b = roofed();
    b.start("roofWindow");
    b.click(6.0, 1.5);
    assert_eq!(the_roof(&b).roof.parts.len(), 1);
    b.click(6.0, 1.5);
    assert_eq!(the_roof(&b).roof.parts.len(), 1, "nothing over the first");
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.last_text(), Some("Çatı penceresi başka bir çatı penceresinin üstüne geliyor; yerini kaydırın."));
    b.click(3.5, 1.5);
    assert_eq!(the_roof(&b).roof.parts.len(), 2, "one beside it");
}
