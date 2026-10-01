//! Köşelere nokta (docs/adr/0152 §5) through the session, over the native
//! document: selection first, the points shown with their names, Ad and Kod
//! Nokta's, the points written in one step on the active layer and Nokta's
//! Ad moved on; as the web's `apps/web/src/tools/vertexPointsTool.test.ts`
//! walks it, the same cases moved to (E, N). Values worked out by hand from
//! the rules; the core itself is checked against the independent reference
//! in the core's `tests/vertex_points.rs`.

mod common;

use common::{Bench, E, N, rel};
use kentos_contracts::{Entity, EntityBase, PathEntity, PointEntity, Vec2};
use kentos_domain::Slot;
use kentos_interaction::{Level, MarkerShape, ViewChange};

fn bench() -> Bench {
    let mut b = Bench::on(include_str!(
        "../../../../fixtures/interaction/v1/empty.kcad"
    ));
    b.draft.snap = false;
    b
}

fn pt(x: f64, y: f64) -> Vec2 {
    Vec2 { x: E + x, y: N + y }
}

fn add(b: &mut Bench, e: Entity) -> Slot {
    b.doc.add(e).expect("a slot")
}

fn point(b: &mut Bench, at: [f64; 2], label: Option<&str>) -> Slot {
    add(
        b,
        Entity::Point(PointEntity {
            base: EntityBase {
                label: label.map(str::to_owned),
                ..common::base("cizim")
            },
            p: pt(at[0], at[1]),
            z: None,
        }),
    )
}

fn area(b: &mut Bench, pts: &[[f64; 2]], zs: Option<Vec<Option<f64>>>) -> Slot {
    add(
        b,
        Entity::Polygon(PathEntity {
            base: common::base("cizim"),
            pts: pts.iter().map(|p| pt(p[0], p[1])).collect(),
            bulges: None,
            holes: None,
            zs,
            parts: None,
        }),
    )
}

/// Two 20 × 15 parcels side by side, the left one's corners with elevations, and a point on a shared corner.
fn parcels(b: &mut Bench) -> [Slot; 2] {
    let left = area(
        b,
        &[[0.0, 0.0], [20.0, 0.0], [20.0, 15.0], [0.0, 15.0]],
        Some(vec![Some(100.0), Some(100.5), None, Some(101.0)]),
    );
    let right = area(
        b,
        &[[20.0, 0.0], [40.0, 0.0], [40.0, 15.0], [20.0, 15.0]],
        None,
    );
    point(b, [20.0, 15.0], Some("7"));
    [left, right]
}

fn points(b: &Bench) -> Vec<PointEntity> {
    b.doc
        .entities()
        .filter_map(|e| match e {
            Entity::Point(p) => Some(p.clone()),
            _ => None,
        })
        .collect()
}

type Seen = (
    [f64; 2],
    Option<String>,
    Option<String>,
    Option<f64>,
    String,
);

/// A point as the tests compare it: where (from (E, N)), its name, its `Kod`, its elevation, its layer.
fn seen(p: &PointEntity) -> Seen {
    (
        rel(p.p),
        p.base.label.clone(),
        p.base.attrs.get("Kod").cloned(),
        p.z,
        p.base.layer_id.clone(),
    )
}

fn named(at: [f64; 2], name: &str, code: Option<&str>, z: Option<f64>) -> Seen {
    (
        at,
        Some(name.to_owned()),
        code.map(str::to_owned),
        z,
        "cizim".to_owned(),
    )
}

#[test]
fn puts_a_named_point_at_every_corner_once_past_the_one_a_point_holds() {
    let mut b = bench();
    let [left, right] = parcels(&mut b);
    b.selection.set(vec![left, right]);
    b.memory.point_name = kentos_interaction::Name::new("101").expect("a name");
    b.memory.point_code = kentos_interaction::Name::new("SN").expect("a code");
    b.start("vertexPoints");
    assert_eq!(
        b.session.prompt().text(),
        "Köşelere nokta: 5 nokta; 1 köşede zaten nokta var [Ad (A): 101 / Kod (K): SN / Uygula (Enter)]"
    );
    assert_eq!(b.options(), ["A", "K", "Enter"]);
    // A click places nothing.
    b.click(50.0, 50.0);
    assert_eq!(points(&b).len(), 1);
    b.move_to(50.0, 20.0);
    let preview = b.run(|s, cx| s.preview(&cx.format())).expect("a preview");
    assert_eq!(
        preview
            .labels
            .iter()
            .map(|l| l.text.as_str())
            .collect::<Vec<_>>(),
        ["101", "102", "103", "104", "105"]
    );
    assert_eq!(
        preview
            .markers
            .iter()
            .map(|m| (
                rel(Vec2 {
                    x: m.at.x,
                    y: m.at.y
                }),
                m.shape
            ))
            .collect::<Vec<_>>(),
        [
            ([0.0, 0.0], MarkerShape::Ring(6.0)),
            ([20.0, 0.0], MarkerShape::Ring(6.0)),
            ([0.0, 15.0], MarkerShape::Ring(6.0)),
            ([40.0, 0.0], MarkerShape::Ring(6.0)),
            ([40.0, 15.0], MarkerShape::Ring(6.0)),
        ]
    );
    assert_eq!(
        preview.tag.map(|t| t.lines),
        Some(vec![
            "5 nokta".to_owned(),
            "1 köşede zaten nokta var".to_owned(),
            "Enter: uygula".to_owned()
        ])
    );
    b.confirm();
    assert_eq!(
        points(&b)[1..].iter().map(seen).collect::<Vec<_>>(),
        [
            named([0.0, 0.0], "101", Some("SN"), Some(100.0)),
            named([20.0, 0.0], "102", Some("SN"), Some(100.5)),
            named([0.0, 15.0], "103", Some("SN"), Some(101.0)),
            named([40.0, 0.0], "104", Some("SN"), None),
            named([40.0, 15.0], "105", Some("SN"), None),
        ]
    );
    assert_eq!(
        b.last_text(),
        Some("Köşelere nokta: 5 nokta eklendi (101 – 105); 1 köşede zaten nokta vardı.")
    );
    assert_eq!(b.last_level(), Some(Level::Success));
    // Nokta goes on past the last name.
    assert_eq!(b.memory.point_name.as_str(), "106");
    assert_eq!(b.session.tool_id(), "select");
    assert_eq!(b.doc.undo().as_deref(), Some("Köşelere nokta"));
    assert_eq!(points(&b).len(), 1);
}

#[test]
fn picks_first_when_nothing_is_selected_and_takes_ad_and_kod_as_nokta_does() {
    let mut b = bench();
    let [left, _] = parcels(&mut b);
    b.start("vertexPoints");
    assert_eq!(
        b.session.prompt().text(),
        "Köşelere nokta: nesnelere tıklayın ya da pencereyle seçin, bitince sağ tıklayın (0 seçili)"
    );
    // A click on the left parcel's bottom edge.
    b.click(10.0, 0.0);
    assert_eq!(b.selected(), [left.0]);
    b.confirm();
    assert_eq!(
        b.session.prompt().text(),
        "Köşelere nokta: 3 nokta; 1 köşede zaten nokta var [Ad (A): — / Kod (K): — / Uygula (Enter)]"
    );
    assert!(b.type_text("a"));
    assert_eq!(b.session.prompt().text(), "Köşelere nokta: değeri yazın");
    assert!(matches!(b.views.last(), Some(ViewChange::Text(_))));
    b.run(|s, cx| s.text_typed(Some("P9"), cx));
    assert_eq!(
        b.session.prompt().text(),
        "Köşelere nokta: 3 nokta; 1 köşede zaten nokta var [Ad (A): P9 / Kod (K): — / Uygula (Enter)]"
    );
    b.confirm();
    assert_eq!(
        points(&b)[1..]
            .iter()
            .map(|p| (p.base.label.clone(), p.base.attrs.is_empty()))
            .collect::<Vec<_>>(),
        [
            (Some("P9".to_owned()), true),
            (Some("P10".to_owned()), true),
            (Some("P11".to_owned()), true),
        ]
    );
    assert_eq!(
        b.last_text(),
        Some("Köşelere nokta: 3 nokta eklendi (P9 – P11); 1 köşede zaten nokta vardı.")
    );
    assert_eq!(b.memory.point_name.as_str(), "P12");
}

#[test]
fn writes_unnamed_points_without_a_name_and_leaves_noktas_empty_ad() {
    let mut b = bench();
    let line = b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    b.selection.set(vec![line]);
    b.start("vertexPoints");
    b.confirm();
    assert_eq!(
        points(&b)
            .iter()
            .map(|p| (rel(p.p), p.base.label.clone()))
            .collect::<Vec<_>>(),
        [([0.0, 0.0], None), ([10.0, 0.0], None)]
    );
    assert_eq!(b.last_text(), Some("Köşelere nokta: 2 nokta eklendi."));
    assert_eq!(b.memory.point_name.as_str(), "");
}

#[test]
fn says_when_every_corner_has_a_point_already_and_when_the_selection_has_no_corners() {
    let mut b = bench();
    let line = b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    point(&mut b, [0.0, 0.0], None);
    point(&mut b, [10.0, 0.0], None);
    b.selection.set(vec![line]);
    b.start("vertexPoints");
    assert_eq!(
        b.session.prompt().text(),
        "Köşelere nokta: yazılacak nokta yok; 2 köşede zaten nokta var [Ad (A): — / Kod (K): — / Uygula (Enter)]"
    );
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Köşelere nokta: yazılacak nokta yok; 2 köşede zaten nokta var.")
    );
    assert_eq!(points(&b).len(), 2);
    assert_eq!(b.session.tool_id(), "select");
    let circle = b.add_circle("cizim", [0.0, 0.0], 5.0);
    b.selection.set(vec![circle]);
    b.start("vertexPoints");
    assert_eq!(
        b.last_text(),
        Some("Köşelere nokta: seçimde çizgi, çoklu çizgi ya da alan yok.")
    );
    assert_eq!(b.session.tool_id(), "select");
}

#[test]
fn is_refused_on_a_locked_layer_and_stays() {
    let mut b = bench();
    let line = b.add_line("cizim", [0.0, 0.0], [10.0, 0.0]);
    b.selection.set(vec![line]);
    b.doc.toggle_layer_locked("cizim");
    b.start("vertexPoints");
    b.confirm();
    assert!(points(&b).is_empty());
    assert!(b.last_text().is_some_and(|t| t.contains("kilitli")));
    assert_eq!(b.session.tool_id(), "vertexPoints");
}
