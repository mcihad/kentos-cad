//! Çatı aracı (docs/adr/8044 §4): a click inside a building's walls writes the roof over them, its eaves the walls' outer
//! faces with the overhang beyond, on the roof's layer (MM-CATI-KIREMIT), its eaves at its storey's top; Beşik slopes the
//! edge nearest the click and the one facing it; the corners drawn are the eaves themselves, the first edge the front.

use crate::common::{Bench, E, N};
use kentos_commands::{CallOp, DrawingUnit, Entity, RoofEntity, Workspace};
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

/// Zemin kat (3 m high) and Çatı at +3.00; on Zemin kat 25 cm walls whose outer faces make a 12 × 9 m outline, the south
/// one in two pieces meeting in line at x 6 (a corner on the outline that does not turn), and a partition inside.
/// Zemin kat is the active level.
fn building() -> Bench {
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
        "cad.walls.create",
        json!({"layerId": "cizim", "walls": [
            {"pts": [p(0.125, 0.125), p(6.0, 0.125)], "width": 0.25, "justify": "center", "arch": {"level": "zemin"}},
            {"pts": [p(6.0, 0.125), p(11.875, 0.125), p(11.875, 8.875), p(0.125, 8.875), p(0.125, 0.125)],
             "width": 0.25, "justify": "center", "arch": {"level": "zemin"}},
            {"pts": [p(5.0, 0.25), p(5.0, 8.75)], "width": 0.1, "justify": "center", "arch": {"level": "zemin"}}
        ]}),
    );
    b.level = Some("zemin".into());
    b
}

fn newest_roof(b: &Bench) -> RoofEntity {
    match b.newest() {
        Entity::Roof(r) => r.clone(),
        e => panic!("a roof: {e:?}"),
    }
}

/// The roof's corners relative to the bench's origin, rounded to the millimetre, in its order.
fn corners(r: &RoofEntity) -> Vec<(f64, f64)> {
    let mm = |v: f64| (v * 1000.0).round() / 1000.0;
    r.roof
        .pts
        .iter()
        .map(|p| (mm(p.x - E), mm(p.y - N)))
        .collect()
}

/// The sides whose edges slope: S, E, N, W by where the edge's middle lies.
fn sloping_sides(r: &RoofEntity) -> Vec<char> {
    let pts = &r.roof.pts;
    let n = pts.len();
    let mut out: Vec<char> = (0..n)
        .filter(|i| r.roof.slopes[*i].is_some())
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let (x, y) = ((a.x + b.x) / 2.0 - E, (a.y + b.y) / 2.0 - N);
            if y < 0.0 {
                'S'
            } else if y > 9.0 {
                'N'
            } else if x < 0.0 {
                'W'
            } else {
                'E'
            }
        })
        .collect();
    out.sort_unstable();
    out
}

#[test]
fn a_click_inside_the_walls_writes_a_hip_roof_over_their_outer_faces() {
    let mut b = building();
    // Katman anahtarları on (docs/adr/8008): the roof on its covering's layer.
    b.memory.layer_keys = true;
    b.start("roof");
    // Its eaves at Zemin kat's top itself (Kot farkı 0), not seated on the walls.
    assert!(b.run(|s, cx| s.configure("F", "0", cx)));
    b.click(6.0, 1.0);
    let r = newest_roof(&b);
    let mut c = corners(&r);
    c.sort_by(|a, b| a.partial_cmp(b).unwrap());
    // The outer faces (0,0)–(12,9), 50 cm out; the corner where the south wall's pieces meet left out.
    assert_eq!(c, [(-0.5, -0.5), (-0.5, 9.5), (12.5, -0.5), (12.5, 9.5)]);
    assert_eq!(r.roof.slopes, vec![Some(33.0); 4]);
    assert_eq!(r.roof.overhang, Some(0.5));
    assert_eq!(r.roof.arch.level.as_deref(), Some("zemin"));
    // 13 × 10 m at 33 %: the ridge 1.65 m over the eaves at Zemin kat's top, +3.00.
    // Over Zemin kat's plan's cut: its eaves dashed there, the whole on Çatı's plan.
    assert_eq!(
        b.last_text(),
        Some(
            "Kırma çatı çizildi; mahya +4.65. Kat planında saçağı kesikli; bütünü “Çatı” planında ve Bütün katlar'da."
        )
    );
    assert_eq!(
        b.doc
            .layers()
            .get(&r.base.layer_id)
            .map(|l| l.name.as_str()),
        Some("MM-CATI-KIREMIT")
    );
}

#[test]
fn a_gable_roof_slopes_the_side_nearest_the_click_and_the_one_facing_it() {
    let mut b = building();
    b.start("roof");
    assert!(b.run(|s, cx| s.configure("T", "gable", cx)));
    assert!(b.run(|s, cx| s.configure("M", "%40", cx)));
    // Near the west wall: the ridge runs north–south between gable ends on the north and south.
    b.click(1.0, 4.5);
    let r = newest_roof(&b);
    assert_eq!(sloping_sides(&r), ['E', 'W']);
    assert_eq!(r.roof.kind_name(), "Beşik çatı");
    assert!(r.roof.slopes.iter().flatten().all(|s| *s == 40.0));
    // Near the south wall: the ridge runs east–west.
    b.click(6.0, 1.0);
    assert_eq!(sloping_sides(&newest_roof(&b)), ['N', 'S']);
}

#[test]
fn corners_drawn_are_the_eaves_and_the_first_edge_is_the_front() {
    let mut b = building();
    b.start("roof");
    assert!(b.run(|s, cx| s.configure("S", "draw", cx)));
    assert!(b.run(|s, cx| s.configure("T", "shed", cx)));
    assert!(b.run(|s, cx| s.configure("M", "10", cx)));
    for (x, y) in [(20.0, 0.0), (28.0, 0.0), (28.0, 6.0), (20.0, 6.0)] {
        b.click(x, y);
    }
    b.confirm();
    let r = newest_roof(&b);
    assert_eq!(
        corners(&r),
        [(20.0, 0.0), (28.0, 0.0), (28.0, 6.0), (20.0, 6.0)]
    );
    // A shed: its water runs off the first edge; the eaves drawn, no overhang said. Seated at its eaves line: its
    // underside on Zemin kat's top there, its eaves its thickness over it.
    assert_eq!(r.roof.slopes, vec![Some(10.0), None, None, None]);
    assert_eq!(r.roof.overhang, None);
    assert!((r.roof.offset - 0.25).abs() < 1e-12);
    assert_eq!(
        b.last_text(),
        Some(
            "Tek eğimli çatı çizildi; mahya +3.85. Kat planında saçağı kesikli; bütünü “Çatı” planında ve Bütün katlar'da."
        )
    );
}

#[test]
fn a_click_outside_any_walls_writes_nothing_and_says_why() {
    let mut b = building();
    b.start("roof");
    let before = b.doc.len();
    b.click(30.0, 30.0);
    assert_eq!(b.doc.len(), before);
    assert_eq!(b.last_level(), Some(kentos_interaction::Level::Warn));
    assert!(
        b.last_text()
            .is_some_and(|t| t.contains("duvar konturunun içinde değil")),
        "{:?}",
        b.last_text()
    );
    // A slope of nothing is refused, the tool keeping its own.
    assert!(!b.run(|s, cx| s.configure("M", "0", cx)));
}

#[test]
fn a_roof_drawn_on_its_own_storey_stands_on_the_walls_under_it() {
    let mut b = building();
    // Çatı active: it has no walls; Binadan finds Zemin kat's under it, the eaves at Çatı's floor, +3.00.
    b.level = Some("cati".into());
    b.start("roof");
    assert!(b.run(|s, cx| s.configure("F", "0", cx)));
    b.click(6.0, 1.0);
    let r = newest_roof(&b);
    assert_eq!(r.roof.arch.level.as_deref(), Some("cati"));
    // Its eaves on Çatı's own floor: its plan is the roof plan; nothing more said.
    assert_eq!(b.last_text(), Some("Kırma çatı çizildi; mahya +4.65."));
}

#[test]
fn its_own_storeys_plan_shows_a_roof_dashed_and_the_storey_above_whole() {
    use kentos_domain::Slot;
    use kentos_geometry_core::entity::Shape;
    use kentos_interaction::level_view::{LevelView, looks};
    let mut b = building();
    b.start("roof");
    b.click(6.0, 1.0);
    let roof = Slot(newest_roof(&b).base.id);
    // Its eaves stand on Zemin kat's top, over that plan's cut: its eaves dashed there, still Zemin kat's own
    // (Mimarlar Odası: "Saçak … nokta nokta işlenir").
    for view in [LevelView::Active, LevelView::Below] {
        let l = looks(&b.doc, view, Some("zemin"));
        assert_eq!(l.roofs, vec![roof.0], "{view:?}");
        assert!(
            l.overhead.is_empty() && !l.hidden.contains(&roof.0),
            "{view:?}: {l:?}"
        );
    }
    // Çatı's floor is its eaves: its plan is the roof plan, the roof whole; Bütün katlar whole too.
    let l = looks(&b.doc, LevelView::Active, Some("cati"));
    assert!(
        l.roofs.is_empty() && !l.hidden.contains(&roof.0) && !l.faded.contains(&roof.0),
        "{l:?}"
    );
    assert!(
        looks(&b.doc, LevelView::All, Some("zemin"))
            .roofs
            .is_empty()
    );

    let dashed = |b: &Bench| {
        b.spatial
            .store()
            .get(f64::from(roof.0))
            .is_some_and(|it| matches!(&it.shape, Shape::Roof { roof, .. } if roof.overhead))
    };
    let pieces = |b: &Bench| {
        b.spatial
            .store()
            .get(f64::from(roof.0))
            .and_then(|it| it.expanded.clone())
            .expect("its pieces")
    };
    b.spatial.sync(&b.doc);
    b.spatial
        .set_level_view(&b.doc, LevelView::Active, Some("zemin".into()));
    assert!(dashed(&b));
    let p = pieces(&b);
    assert_eq!(p.shapes.len(), 1, "its eaves alone: {:?}", p.shapes);
    assert!(
        p.strokes
            .iter()
            .all(|s| s.line_type.as_deref() == Some("dashed")),
        "{:?}",
        p.strokes
    );
    // Picked on its own storey's plan: its eaves on the west, 50 cm out of the wall.
    let at = kentos_interaction::Vec2::new(E - 0.5, N + 4.0);
    assert_eq!(b.spatial.pick(at, 0.2), Some(roof));
    // On Çatı: whole, its hips and arrows back.
    b.spatial
        .set_level_view(&b.doc, LevelView::Active, Some("cati".into()));
    assert!(!dashed(&b));
    assert!(pieces(&b).shapes.len() > 1);
}

#[test]
fn without_an_offset_the_roof_sits_on_the_walls_outer_faces() {
    let mut b = building();
    b.start("roof");
    assert!(b.run(|s, cx| s.configure("M", "40", cx)));
    b.click(6.0, 1.0);
    let r = newest_roof(&b);
    // 25 cm thick, 50 cm out at 40 %: its eaves 5 cm over Zemin kat's top, so its underside meets it at the walls'
    // outer faces (Revit's Truss); 13 × 10 m: the ridge 2.00 m over the eaves.
    assert!((r.roof.offset - 0.05).abs() < 1e-12, "{}", r.roof.offset);
    assert_eq!(
        b.last_text(),
        Some(
            "Kırma çatı çizildi; mahya +5.05. Kat planında saçağı kesikli; bütünü “Çatı” planında ve Bütün katlar'da."
        )
    );
    // A typed offset is kept; an empty one seats it again.
    assert!(b.run(|s, cx| s.configure("F", "-0.1", cx)));
    assert!(b.run(|s, cx| s.configure("F", "", cx)));
    b.click(6.0, 1.0);
    assert!((newest_roof(&b).roof.offset - 0.05).abs() < 1e-12);
}

/// Oluk and Kar siperliği (docs/adr/8044 §8; Sivas İmar Yönetmeliği 42/5): a roof gets both by default, 15 cm and 30 cm;
/// emptied, none; a flat roof has neither.
#[test]
fn a_roof_gets_a_gutter_and_a_snow_guard_unless_emptied() {
    let mut b = building();
    b.start("roof");
    assert!(b.options().contains(&"O") && b.options().contains(&"K"));
    b.click(6.0, 1.0);
    let r = newest_roof(&b);
    assert_eq!((r.roof.gutter, r.roof.snow_guard), (Some(0.15), Some(0.3)));
    assert!(b.run(|s, cx| s.configure("O", "", cx)));
    assert!(b.run(|s, cx| s.configure("K", "yok", cx)));
    b.click(6.0, 1.0);
    let r = newest_roof(&b);
    assert_eq!((r.roof.gutter, r.roof.snow_guard), (None, None));
    // A 60 cm gutter is refused, the tool keeping its own.
    assert!(!b.run(|s, cx| s.configure("O", "0.6", cx)));
    assert!(b.run(|s, cx| s.configure("O", "0.12", cx)));
    assert!(b.run(|s, cx| s.configure("T", "flat", cx)));
    b.click(6.0, 1.0);
    assert_eq!(
        newest_roof(&b).roof.gutter,
        None,
        "a flat roof sheds no water off an eaves edge"
    );
}
