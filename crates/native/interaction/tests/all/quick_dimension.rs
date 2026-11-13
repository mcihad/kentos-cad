//! Hızlı ölçü (docs/adr/0147 §7) through the session, over the native
//! document: the selection first, then where the dimensions go or their
//! distance typed, all in one step; as the web's
//! `apps/web/src/tools/quickDimensionTool.test.ts` walks it. Values worked
//! out by hand; what each edge gets is the core's, checked against the
//! independent reference in the core's `tests/dimensions.rs`.

use crate::common;

use common::{Bench, rel};
use kentos_commands::{DimensionEntity, DimensionStyle, Entity};
use kentos_domain::Slot;
use kentos_interaction::Level;

const EMPTY: &str = include_str!("../../../../../fixtures/interaction/v1/empty.kcad");

fn dims(b: &Bench) -> Vec<DimensionEntity> {
    let mut out: Vec<DimensionEntity> = b
        .doc
        .entities()
        .filter_map(|e| match e {
            Entity::Dimension(d) => Some(d.clone()),
            _ => None,
        })
        .collect();
    out.sort_by_key(|d| d.base.id);
    out
}

fn near(have: f64, want: f64) -> bool {
    (have - want).abs() < 1e-9
}

/// Two parcels side by side sharing their 30 m edge, a road edge (10 m
/// straight, then a clockwise quarter arc of 10 m) and a point, nothing running.
fn ground() -> (Bench, [Slot; 4]) {
    let mut b = Bench::on(EMPTY);
    b.draft.snap = false;
    let west = b.add_path(
        "cizim",
        &[[0.0, 0.0], [20.0, 0.0], [20.0, 30.0], [0.0, 30.0]],
        true,
    );
    let east = b.add_path(
        "cizim",
        &[[20.0, 0.0], [40.0, 0.0], [40.0, 30.0], [20.0, 30.0]],
        true,
    );
    let road = b.add_bulged(
        "cizim",
        &[[0.0, -10.0], [10.0, -10.0], [20.0, -20.0]],
        &[0.0, -(std::f64::consts::PI / 8.0).tan()],
    );
    let spot = b.add_point_z("cizim", [50.0, 50.0], 100.0);
    (b, [west, east, road, spot])
}

#[test]
fn every_edge_of_the_selection_at_once_the_shared_one_once_out_of_the_areas() {
    let (mut b, [west, east, road, spot]) = ground();
    b.selection.set([spot, east, west, road]);
    b.start("quickDimension");
    assert_eq!(
        b.session.prompt().text(),
        "Hızlı ölçü: ölçülerin yerini gösterin ya da uzaklık yazın [Zemin (Z): kapalı]"
    );
    assert!(
        !b.session.tracks(),
        "no snapping: the cursor's distance places them"
    );
    // 4 m over the parcels' north edge: the nearest edge.
    b.move_to(10.0, 34.0);
    let strokes = b.run(|s, cx| s.preview(&cx.format()).map_or(0, |p| p.strokes.len()));
    assert!(strokes > 0, "the dimensions previewed");
    b.click(10.0, 34.0);
    let made = dims(&b);
    // West 4 edges, east 3 (their shared edge once), the road's straight part and its arc.
    assert_eq!(made.len(), 9);
    assert_eq!(
        b.last_text(),
        Some(
            "Hızlı ölçü: 9 ölçü eklendi; 1 nesne atlandı (yalnız çizgi, çoklu çizgi ve alan ölçülür); ölçüler kenarlarının köşelerine bağlı. 1 yay uzunluğu ölçüsü nesneye bağlı değil (İlişki: Serbest): yay sonra değişirse kendiliğinden güncellenmez."
        )
    );
    assert_eq!(b.last_level(), Some(Level::Success));
    // The tool is done.
    assert_eq!(b.session.tool_id(), "select");
    // Out of the west parcel (counter-clockwise: right of its edges).
    let first = &made[0];
    assert_eq!((rel(first.a), rel(first.b)), ([0.0, 0.0], [20.0, 0.0]));
    assert!(
        near(first.offset, -4.0) && near(first.height, 2.5),
        "{first:?}"
    );
    assert_eq!(first.style, None);
    // The road: its side is the cursor's, left of its way.
    let (straight, arc) = (&made[7], &made[8]);
    assert_eq!(
        (rel(straight.a), rel(straight.b)),
        ([0.0, -10.0], [10.0, -10.0])
    );
    assert!(near(straight.offset, 4.0), "{straight:?}");
    // The clockwise arc about (10, −20): its ends counter-clockwise, its dimension outwards.
    assert_eq!(arc.style, Some(DimensionStyle::ArcLength));
    assert_eq!((rel(arc.a), rel(arc.b)), ([20.0, -20.0], [10.0, -10.0]));
    assert!(near(arc.offset, 4.0), "{arc:?}");
    let c = rel(arc.c.expect("a centre"));
    assert!(near(c[0], 10.0) && near(c[1], -20.0), "{c:?}");
    // One step, named after the tool (docs/adr/0622 §9).
    assert_eq!(b.doc.undo().as_deref(), Some("Hızlı ölçü"));
    assert!(dims(&b).is_empty());
}

#[test]
fn a_typed_distance_and_zemin() {
    let (mut b, [west, ..]) = ground();
    b.selection.set([west]);
    b.start("quickDimension");
    assert!(b.type_text("Z"));
    assert!(b.session.prompt().text().ends_with("[Zemin (Z): açık]"));
    // Not a distance: refused.
    assert!(!b.type_text("12,5"));
    b.move_to(10.0, 15.0);
    assert!(b.type_text("-2.5"));
    let made = dims(&b);
    assert_eq!(made.len(), 4);
    assert!(made.iter().all(|d| d.offset == -2.5 && d.mask), "{made:?}");
    assert_eq!(b.last_text(), Some("Hızlı ölçü: 4 ölçü eklendi; ölçüler kenarlarının köşelerine bağlı."));
    // Zemin stays for the next run, as Ölçülendirme's.
    assert!(b.memory.dimension_mask);
}

#[test]
fn picks_first_without_a_selection_and_leaves_with_nothing_to_measure() {
    let (mut b, [.., spot]) = ground();
    b.start("quickDimension");
    assert_eq!(
        b.session.prompt().text(),
        "Hızlı ölçü: nesnelere tıklayın ya da pencereyle seçin (0 seçili) [Bitir (Enter)]"
    );
    b.selection.set([spot]);
    b.confirm();
    assert_eq!(
        b.last_text(),
        Some("Seçimde ölçülecek çizgi, çoklu çizgi ya da alan yok.")
    );
    assert_eq!(b.last_level(), Some(Level::Warn));
    assert_eq!(b.session.tool_id(), "select");
    assert!(dims(&b).is_empty());
}

/// UR-020: a quick dimension's ends follow the vertices they measure: the parcel moved, its dimensions move with it
/// in the same step; an arc's length dimension stays as it was laid.
#[test]
fn quick_dimensions_follow_their_object() {
    let (mut b, [west, _, road, _]) = ground();
    b.selection.set([west, road]);
    b.start("quickDimension");
    b.click(10.0, 34.0);
    let made = dims(&b);
    let straight: Vec<&DimensionEntity> = made.iter().filter(|d| d.style.is_none()).collect();
    assert!(!straight.is_empty());
    assert!(
        straight.iter().all(|d| d.refs.as_ref().is_some_and(|r| r.a.is_some() && r.b.is_some())),
        "every straight edge's ends tied"
    );
    assert!(made.iter().filter(|d| d.style == Some(DimensionStyle::ArcLength)).all(|d| d.refs.is_none()));
    let before: Vec<[[f64; 2]; 2]> = straight.iter().map(|d| [rel(d.a), rel(d.b)]).collect();
    let ids: Vec<u32> = straight.iter().map(|d| d.base.id).collect();

    b.selection.set([west]);
    b.start("move");
    b.click(0.0, 0.0);
    assert!(b.type_text("@5,0"));
    let after = dims(&b);
    let mut followed = 0;
    for (id, was) in ids.iter().zip(&before) {
        let d = after.iter().find(|d| d.base.id == *id).expect("still there");
        let moved = [rel(d.a), rel(d.b)];
        let tied_to_west = d.refs.as_ref().and_then(|r| r.a.as_ref()).is_some_and(|r| {
            Some(kentos_commands::EntityId(*b.doc.uid(west).expect("west's id").as_bytes())) == Some(r.uid)
        });
        if tied_to_west {
            assert!(
                near(moved[0][0], was[0][0] + 5.0) && near(moved[1][0], was[1][0] + 5.0),
                "{id}: {was:?} → {moved:?}"
            );
            assert!(near(moved[0][1], was[0][1]) && near(moved[1][1], was[1][1]));
            followed += 1;
        }
    }
    assert_eq!(followed, 4, "the parcel's four edges followed it");
}
