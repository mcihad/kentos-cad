//! Topolojik temizlik (docs/adr/0148 §9) through the session, over the
//! native document: the scope taken when it starts, the finding shown
//! first, the tolerance typed and the works turned on and off, one step
//! written; as the web's `apps/web/src/tools/topologyTool.test.ts` walks it,
//! the same cases moved to (E, N). Values worked out by hand from the rules
//! of §4–§6; the core itself is checked against the independent reference
//! in the core's `tests/topology.rs`.

use crate::common;

use common::{Bench, E, N, rel};
use kentos_contracts::{
    ArcEntity, Entity, EntityGeometry, LineEntity, PathEntity, PointEntity, RingGeometry,
    TextEntity, Vec2,
};
use kentos_domain::Slot;
use kentos_interaction::Level;
use kentos_interaction::topology::{geometry_of, object};

/// The traces' empty drawing with a road layer and a locked one.
const DRAWING: &str = r##"{"format":"kentos.document","version":1,"name":"Topolojik temizlik","settings":{"srid":5256,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,"workspace":"hybrid","drawingFont":"barlow"},"origin":{"x":487000,"y":4420000},"layers":[{"id":"cizim","name":"Çizim","type":"layer","visible":true,"locked":false,"expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.25},"children":[]},{"id":"yol","name":"Yol","type":"layer","visible":true,"locked":false,"expanded":true,"style":{"color":"#E5484D","lineType":"continuous","lineWeight":0.5},"children":[]},{"id":"kilitli","name":"Kilitli","type":"layer","visible":true,"locked":true,"expanded":true,"style":{"color":"#8E8E93","lineType":"continuous","lineWeight":0.25},"children":[]}],"activeLayer":"cizim","entities":[],"styles":{"items":[],"categories":[]}}"##;

const OPTIONS: &str = "[Tolerans (T): 0.010 m / Uçlar (U): açık / Köşeler (K): kapalı / Uzat (Z): açık / Buda (B): açık / Uygula (Enter)]";

fn bench() -> Bench {
    let mut b = Bench::on(DRAWING);
    b.draft.snap = false;
    b
}

fn pt(x: f64, y: f64) -> Vec2 {
    Vec2 { x: E + x, y: N + y }
}

fn add(b: &mut Bench, e: Entity) -> Slot {
    b.doc.add(e).expect("a slot")
}

fn line(
    b: &mut Bench,
    layer: &str,
    a: [f64; 2],
    bb: [f64; 2],
    za: Option<f64>,
    zb: Option<f64>,
) -> Slot {
    add(
        b,
        Entity::Line(LineEntity {
            base: common::base(layer),
            a: pt(a[0], a[1]),
            b: pt(bb[0], bb[1]),
            za,
            zb,
        }),
    )
}

fn line_of(b: &Bench, slot: Slot) -> &LineEntity {
    match b.doc.get(slot) {
        Some(Entity::Line(l)) => l,
        other => panic!("a line at {slot:?}: {other:?}"),
    }
}

fn near(p: Vec2, want: [f64; 2]) -> bool {
    let [x, y] = rel(p);
    (x - want[0]).abs() < 1e-9 && (y - want[1]).abs() < 1e-9
}

#[test]
fn joins_the_ends_within_the_tolerance_on_the_whole_drawing_in_one_step() {
    let mut b = bench();
    let first = line(&mut b, "cizim", [0.0, 0.0], [10.0, 0.0], None, None);
    let second = line(&mut b, "cizim", [10.006, 0.004], [20.0, 0.0], None, None);
    add(
        &mut b,
        Entity::Point(PointEntity {
            base: common::base("cizim"),
            p: pt(20.0, 10.0),
            z: Some(105.0),
            parts: None,
        }),
    );
    let third = line(
        &mut b,
        "cizim",
        [20.0, 0.0],
        [20.004, 9.997],
        Some(100.0),
        Some(104.0),
    );
    b.start("topology");
    assert_eq!(
        b.last_text(),
        Some(
            "Topolojik temizlik: 2 uç birleşir; en büyük kayma 0.007 m. Enter ile uygulayın (bütün çizim: 3 nesne, 1 dayanak)."
        )
    );
    assert_eq!(
        b.session.prompt().text(),
        format!("Topolojik temizlik: 2 uç birleşir; en büyük kayma 0.007 m {OPTIONS}")
    );
    assert_eq!(b.options(), ["T", "U", "K", "Z", "B", "Enter"]);
    assert!(!b.session.tracks(), "no snapping");
    // Shown, not written: a ring where each end lands, the old outlines dashed under the new.
    assert!(near(line_of(&b, second).a, [10.006, 0.004]));
    b.move_to(5.0, 5.0);
    let preview = b.run(|s, cx| s.preview(&cx.format())).expect("a preview");
    // The finding beside the cursor.
    assert_eq!(
        preview.tag.map(|t| t.lines),
        Some(vec![
            "2 uç birleşir".to_owned(),
            "en büyük kayma 0.007 m".to_owned(),
            "Enter: uygula".to_owned()
        ])
    );
    // A ring where each end lands, a small one where it was.
    assert_eq!(preview.markers.len(), 4);
    assert!(preview.markers.iter().any(|m| near(
        Vec2 {
            x: m.at.x,
            y: m.at.y
        },
        [10.006, 0.004]
    )));
    assert!(preview.markers.iter().any(|m| near(
        Vec2 {
            x: m.at.x,
            y: m.at.y
        },
        [10.0, 0.0]
    )));
    assert!(preview.markers.iter().any(|m| near(
        Vec2 {
            x: m.at.x,
            y: m.at.y
        },
        [20.0, 10.0]
    )));
    // The two old outlines and the two moves, from where each end was.
    assert_eq!(
        preview.strokes.iter().filter(|s| s.dash.is_some()).count(),
        4
    );
    b.confirm();
    // The second line's start goes to the first's end (the drawing's order breaks the tie); the third's end to the point.
    let l = line_of(&b, second);
    assert!(near(l.a, [10.0, 0.0]) && near(l.b, [20.0, 0.0]), "{l:?}");
    assert_eq!((l.za, l.zb), (None, None));
    let l = line_of(&b, third);
    assert!(near(l.a, [20.0, 0.0]) && near(l.b, [20.0, 10.0]), "{l:?}");
    assert_eq!((l.za, l.zb), (Some(100.0), Some(105.0)));
    assert!(near(line_of(&b, first).b, [10.0, 0.0]));
    assert_eq!(
        b.last_text(),
        Some("Topolojik temizlik: 2 uç birleşti; 2 nesne değişti, en büyük kayma 0.007 m.")
    );
    assert_eq!(b.last_level(), Some(Level::Success));
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.undo().as_deref(), Some("Topolojik temizlik"));
    assert_eq!(line_of(&b, third).zb, Some(104.0));
}

#[test]
fn takes_a_typed_tolerance_turns_the_works_and_extends_and_trims() {
    let mut b = bench();
    line(&mut b, "cizim", [0.0, 0.0], [20.0, 0.0], None, None);
    let short = line(&mut b, "cizim", [5.0, 10.0], [5.0, 0.03], None, None);
    let past = line(&mut b, "cizim", [10.0, 10.0], [10.0, -0.02], None, None);
    let far = line(&mut b, "cizim", [15.0, 10.0], [15.0, 0.2], None, None);
    b.start("topology");
    assert_eq!(
        b.last_text(),
        Some(
            "Topolojik temizlik: 0.010 m toleransla düzeltilecek bir şey yok; daha büyük bir tolerans yazın (bütün çizim: 4 nesne, 0 dayanak)."
        )
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(
        b.session.prompt().text(),
        format!("Topolojik temizlik: düzeltilecek bir şey yok {OPTIONS}")
    );
    assert!(b.type_text("0.05"));
    // Each new finding is said: the history and the status bar keep the current one.
    assert_eq!(
        b.last_text(),
        Some("Topolojik temizlik: 1 uç uzar, 1 uç kısalır; en büyük kayma 0.030 m.")
    );
    let with =
        |works: &str| format!("[Tolerans (T): 0.050 m / Uçlar (U): {works} / Uygula (Enter)]");
    assert_eq!(
        b.session.prompt().text(),
        format!(
            "Topolojik temizlik: 1 uç uzar, 1 uç kısalır; en büyük kayma 0.030 m {}",
            with("açık / Köşeler (K): kapalı / Uzat (Z): açık / Buda (B): açık")
        )
    );
    // Uzat off: the end that stops short goes onto the nearest line instead.
    assert!(b.type_text("z"));
    assert_eq!(
        b.session.prompt().text(),
        format!(
            "Topolojik temizlik: 1 uç kısalır, 1 uç kenara taşınır; en büyük kayma 0.030 m {}",
            with("açık / Köşeler (K): kapalı / Uzat (Z): kapalı / Buda (B): açık")
        )
    );
    b.type_text("U");
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Topolojik temizlik: 1 uç kısalır; en büyük kayma 0.020 m [")
    );
    b.type_text("B");
    assert_eq!(
        b.session.prompt().text(),
        format!(
            "Topolojik temizlik: düzeltilecek bir şey yok {}",
            with("kapalı / Köşeler (K): kapalı / Uzat (Z): kapalı / Buda (B): kapalı")
        )
    );
    assert_eq!(
        b.last_text(),
        Some("Topolojik temizlik: 0.050 m toleransla düzeltilecek bir şey yok.")
    );
    let said = b.log.len();
    // T asks for the tolerance; Esc goes back, keeping it, and the finding is not said again.
    assert!(b.type_text("T"));
    assert_eq!(
        b.session.prompt().text(),
        "Topolojik temizlik: toleransı yazın, metre (Enter: 0.050 m)"
    );
    assert_eq!(b.options(), Vec::<&str>::new());
    assert!(b.run(|s, cx| s.cancel(cx)), "Esc steps back");
    assert_eq!(b.log.len(), said);
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Topolojik temizlik: düzeltilecek bir şey yok [Tolerans (T): 0.050 m")
    );
    // Below a micrometre: said, the tolerance kept; not a number: not taken.
    assert!(b.type_text("0"));
    assert_eq!(b.last_text(), Some("Tolerans en az 0.000001 m olmalı."));
    assert!(!b.type_text("abc"));
    assert_eq!(b.memory.topology_tolerance, 0.05);
    for k in ["U", "Z", "B"] {
        b.type_text(k);
    }
    b.confirm();
    let l = line_of(&b, short);
    assert!(near(l.a, [5.0, 10.0]) && near(l.b, [5.0, 0.0]), "{l:?}");
    let l = line_of(&b, past);
    assert!(near(l.a, [10.0, 10.0]) && near(l.b, [10.0, 0.0]), "{l:?}");
    assert!(near(line_of(&b, far).b, [15.0, 0.2]));
    assert_eq!(
        b.last_text(),
        Some(
            "Topolojik temizlik: 1 uç uzadı, 1 uç kısaldı; 2 nesne değişti, en büyük kayma 0.030 m."
        )
    );
    // The tolerance is kept for as long as the app lives.
    b.start("topology");
    assert!(b.session.prompt().text().contains("Tolerans (T): 0.050 m"));
}

#[test]
fn with_a_selection_corrects_only_it_the_rest_and_the_locked_are_supports() {
    let mut b = bench();
    let first = line(&mut b, "cizim", [0.0, 0.0], [10.0, 0.0], None, None);
    let second = line(&mut b, "cizim", [10.004, 0.0], [20.0, 0.0], None, None);
    let locked = line(&mut b, "kilitli", [20.003, 0.0], [30.0, 0.0], None, None);
    let text = add(
        &mut b,
        Entity::Text(TextEntity {
            base: common::base("cizim"),
            p: pt(5.0, 5.0),
            text: "Ada 101".into(),
            height: 1.0,
            rotation: 0.0,
            align: None,
            width_factor: None,
            mask: false,
            label_of: None,
            label_scale: None,
            paragraph: Default::default(),
            face: Default::default(),
            path: None,
        }),
    );
    b.selection.set([second, locked, text]);
    let before = b.log.len();
    b.start("topology");
    assert_eq!(
        b.said(before),
        [
            (
                Level::Warn,
                "1 nesne kilitli katmanda olduğu için düzeltilmez; dayanak olarak kalır."
            ),
            (
                Level::Warn,
                "1 nesne topolojik temizliğe katılmaz: yazı, ölçü, tarama, blok, kılavuz ve yardımcı çizgiler girmez."
            ),
            (
                Level::Info,
                "Topolojik temizlik: 2 uç birleşir; en büyük kayma 0.004 m. Enter ile uygulayın (seçili 1 nesne, 2 dayanak)."
            ),
        ]
    );
    b.confirm();
    let l = line_of(&b, second);
    assert!(near(l.a, [10.0, 0.0]) && near(l.b, [20.003, 0.0]), "{l:?}");
    assert!(near(line_of(&b, first).b, [10.0, 0.0]));
    assert!(near(line_of(&b, locked).a, [20.003, 0.0]));
}

#[test]
fn leaves_a_hidden_layer_out_as_a_support_too() {
    let mut b = bench();
    line(&mut b, "cizim", [0.0, 0.0], [10.0, 0.0], None, None);
    let hidden = line(&mut b, "yol", [10.004, 0.0], [20.0, 0.0], None, None);
    b.doc.set_layer_visible("yol", false);
    b.start("topology");
    assert_eq!(
        b.last_text(),
        Some(
            "Topolojik temizlik: 0.010 m toleransla düzeltilecek bir şey yok; daha büyük bir tolerans yazın (bütün çizim: 1 nesne, 0 dayanak)."
        )
    );
    // Nothing to write: said, and the tool leaves.
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Topolojik temizlik: düzeltilecek bir şey yok; hiçbir şey değişmedi.")
    );
    assert_eq!(b.session.tool_id(), "select");
    assert!(near(line_of(&b, hidden).a, [10.004, 0.0]));
}

#[test]
fn with_nothing_it_corrects_says_so_and_leaves() {
    let mut b = bench();
    b.add_point_z("cizim", [0.0, 0.0], 1.0);
    b.add_circle("cizim", [5.0, 5.0], 2.0);
    b.start("topology");
    assert_eq!(
        b.last_text(),
        Some("Topolojik temizlik: düzeltilecek çizgi, çoklu çizgi, yay ya da alan yok.")
    );
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn moves_an_arcs_end_keeping_its_angle() {
    let mut b = bench();
    line(&mut b, "cizim", [0.0, 0.0], [10.0, 0.0], None, None);
    let pi = std::f64::consts::PI;
    let arc = b.add_arc("cizim", [10.006, 5.0], 5.0, -pi / 2.0, 0.0);
    b.start("topology");
    b.confirm();
    let Some(Entity::Arc(ArcEntity { c, r, a0, a1, .. })) = b.doc.get(arc) else {
        panic!("an arc");
    };
    let on = |a: f64| Vec2 {
        x: c.x + r * a.cos(),
        y: c.y + r * a.sin(),
    };
    // Its start is on the line's end now, its end where it was, and it still turns a quarter.
    assert!(near(on(*a0), [10.0, 0.0]), "{:?}", rel(on(*a0)));
    assert!(near(on(*a1), [15.006, 5.0]), "{:?}", rel(on(*a1)));
    assert!(((a1 - a0).rem_euclid(2.0 * pi) - pi / 2.0).abs() < 1e-12);
}

#[test]
fn joins_the_vertices_of_areas_only_with_koseler_the_holes_kept() {
    let mut b = bench();
    b.add_path(
        "cizim",
        &[[0.0, 0.0], [10.0, 0.0], [10.0, 10.0], [0.0, 10.0]],
        true,
    );
    let hole = RingGeometry {
        pts: vec![pt(12.0, 2.0), pt(18.0, 2.0), pt(18.0, 8.0), pt(12.0, 8.0)],
        bulges: None,
        zs: None,
    };
    let right = add(
        &mut b,
        Entity::Polygon(PathEntity {
            base: common::base("cizim"),
            pts: vec![
                pt(10.004, 0.0),
                pt(20.0, 0.0),
                pt(20.0, 10.0),
                pt(10.003, 10.0),
            ],
            bulges: None,
            holes: Some(vec![hole.clone()]),
            zs: None,
            parts: None,
        }),
    );
    b.start("topology");
    assert!(
        b.session
            .prompt()
            .text()
            .starts_with("Topolojik temizlik: düzeltilecek bir şey yok [")
    );
    b.type_text("K");
    assert_eq!(
        b.session.prompt().text(),
        "Topolojik temizlik: 2 köşe birleşir; en büyük kayma 0.004 m [Tolerans (T): 0.010 m / Uçlar (U): açık / Köşeler (K): açık / Uzat (Z): açık / Buda (B): açık / Uygula (Enter)]"
    );
    b.confirm();
    let pts = b.path_pts(right);
    let want = [[10.0, 0.0], [20.0, 0.0], [20.0, 10.0], [10.0, 10.0]];
    assert!(
        pts.iter()
            .zip(want)
            .all(|(p, q)| (p[0] - q[0]).abs() < 1e-9 && (p[1] - q[1]).abs() < 1e-9),
        "{pts:?}"
    );
    let Some(Entity::Polygon(area)) = b.doc.get(right) else {
        panic!("an area");
    };
    assert_eq!(area.holes, Some(vec![hole]));
}

#[test]
fn the_objects_it_takes_are_paths_with_their_elevations_and_come_back_the_same() {
    let mut b = bench();
    let slot = line(&mut b, "cizim", [0.0, 0.0], [1.0, 0.0], Some(5.0), None);
    let o = object(b.doc.get(slot).unwrap(), false).unwrap();
    assert_eq!((o.kind.as_str(), o.fixed), ("line", false));
    assert_eq!(o.paths[0].zs, [Some(5.0), None]);
    assert_eq!(o.paths[0].bulges, None);
    // Bulges one a vertex, the missing last one straight.
    let path = add(
        &mut b,
        Entity::Polyline(PathEntity {
            base: common::base("cizim"),
            pts: vec![pt(0.0, 0.0), pt(1.0, 0.0), pt(2.0, 1.0)],
            bulges: Some(vec![0.5, 0.0]),
            holes: None,
            zs: Some(vec![Some(1.0), None, Some(3.0)]),
            parts: None,
        }),
    );
    let e = b.doc.get(path).unwrap();
    let o = object(e, true).unwrap();
    assert_eq!(o.paths[0].bulges, Some(vec![0.5, 0.0, 0.0]));
    let Some(EntityGeometry::Polyline { bulges, zs, .. }) = geometry_of(e, &o.paths) else {
        panic!("a polyline");
    };
    assert_eq!(bulges, Some(vec![0.5, 0.0, 0.0]));
    assert_eq!(zs, Some(vec![Some(1.0), None, Some(3.0)]));
    // An area ring after ring: its holes, then each other part's ring and holes.
    let area = add(
        &mut b,
        Entity::Polygon(PathEntity {
            base: common::base("cizim"),
            pts: vec![pt(0.0, 0.0), pt(4.0, 0.0), pt(4.0, 4.0)],
            bulges: None,
            holes: Some(vec![RingGeometry {
                pts: vec![pt(1.0, 1.0), pt(2.0, 1.0), pt(2.0, 2.0)],
                bulges: None,
                zs: None,
            }]),
            zs: None,
            parts: Some(vec![kentos_contracts::AreaPart {
                pts: vec![pt(10.0, 0.0), pt(14.0, 0.0), pt(14.0, 4.0)],
                bulges: None,
                holes: Some(vec![RingGeometry {
                    pts: vec![pt(11.0, 1.0), pt(12.0, 1.0), pt(12.0, 2.0)],
                    bulges: None,
                    zs: Some(vec![Some(7.0), Some(8.0), Some(9.0)]),
                }]),
                zs: None,
            }]),
        }),
    );
    let e = b.doc.get(area).unwrap();
    let o = object(e, false).unwrap();
    assert_eq!(o.kind, "area");
    let firsts: Vec<[f64; 2]> = o
        .paths
        .iter()
        .map(|p| [p.pts[0].x - E, p.pts[0].y - N])
        .collect();
    assert_eq!(firsts, [[0.0, 0.0], [1.0, 1.0], [10.0, 0.0], [11.0, 1.0]]);
    let Some(EntityGeometry::Polygon { holes, parts, .. }) = geometry_of(e, &o.paths) else {
        panic!("an area");
    };
    assert_eq!(holes.unwrap()[0].zs, Some(vec![None, None, None]));
    let parts = parts.unwrap();
    assert_eq!(
        parts[0].holes.as_ref().unwrap()[0].zs,
        Some(vec![Some(7.0), Some(8.0), Some(9.0)])
    );
    // Points always fixed; circles edges only; texts no part.
    let point = b.add_point_z("cizim", [3.0, 4.0], 9.0);
    let o = object(b.doc.get(point).unwrap(), false).unwrap();
    assert_eq!(
        (o.kind.as_str(), o.fixed, o.paths[0].zs.clone()),
        ("point", true, vec![Some(9.0)])
    );
    let circle = b.add_circle("cizim", [0.0, 0.0], 2.0);
    let o = object(b.doc.get(circle).unwrap(), false).unwrap();
    assert_eq!((o.kind.as_str(), o.fixed), ("edges", true));
    assert_eq!(o.paths[0].bulges, Some(vec![1.0, 1.0]));
    assert!(near(o.paths[0].pts[0].into_contract(), [2.0, 0.0]));
}

/// The core's point as the contract's, for `near`.
trait IntoContract {
    fn into_contract(self) -> Vec2;
}

impl IntoContract for kentos_interaction::Vec2 {
    fn into_contract(self) -> Vec2 {
        Vec2 {
            x: self.x,
            y: self.y,
        }
    }
}
