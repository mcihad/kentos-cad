//! The store's own tests: its hashing, layers, picking and the rest of what store/mod.rs keeps (moved out of it to keep
//! the file under the size limit, CLAUDE.md §8).

use std::hash::{BuildHasher, BuildHasherDefault};

/// Whole-number ids (their `f64` bits end in zeros) and orders in turn
/// fall into many buckets, the hash's low bits: as a random hash would.
#[test]
fn ids_and_orders_spread_over_the_buckets() {
    let hasher = BuildHasherDefault::<super::IdHasher>::default();
    let mask = (1u64 << 16) - 1;
    for keys in [
        (0..65_536u64)
            .map(|n| (n as f64).to_bits())
            .collect::<Vec<_>>(),
        (0..65_536u64).collect(),
    ] {
        let buckets: std::collections::HashSet<u64> =
            keys.iter().map(|k| hasher.hash_one(k) & mask).collect();
        // A random hash fills about 63 % of them.
        assert!(buckets.len() > 40_000, "{} buckets", buckets.len());
    }
}

use super::*;

fn line(id: f64, layer: &str, x: f64) -> String {
    format!(
        r#"{{"id":{id},"layerId":"{layer}","attrs":{{"Ada":"1"}},"kind":"line","a":{{"x":{x},"y":0}},"b":{{"x":{x},"y":10}}}}"#
    )
}

/// docs/adr/0210 §6: with the slider's window, a temporal layer's objects it does not show are neither
/// picked nor near nor in a window's selection; a timeless one always is; without a window nothing is left out.
#[test]
fn the_time_window_leaves_out_what_it_does_not_show() {
    use crate::Vec2;
    use crate::time::{Mode, Time, Window};
    let mut s = Store::new();
    s.put_json(&format!(
        "[{},{},{}]",
        line(1.0, "a", 0.0),
        line(2.0, "a", 5.0),
        line(3.0, "a", 10.0)
    ))
    .unwrap();
    let t = |a: f64, b: f64| {
        Some(Time {
            s: a,
            e: b,
            mode: Mode::Range,
        })
    };
    s.set_times([(1.0, t(0.0, 10.0)), (2.0, t(10.0, 20.0))]);
    let near = |s: &Store, x: f64| {
        s.near(Vec2::new(x, 5.0), 0.5)
            .iter()
            .map(|it| it.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(near(&s, 0.0), [1.0]);
    s.set_time_window(Some(Window::Instant(15.0)));
    assert!(near(&s, 0.0).is_empty());
    assert_eq!(near(&s, 5.0), [2.0]);
    assert_eq!(near(&s, 10.0), [3.0], "timeless");
    assert_eq!(s.time_mask(&[1.0, 2.0, 3.0]), [0, 1, 1]);
    s.set_times([(2.0, None)]);
    assert_eq!(s.time_mask(&[1.0, 2.0]), [0, 1]);
    s.set_time_window(None);
    assert_eq!(near(&s, 0.0), [1.0]);
}

#[test]
fn the_day_known_leaves_out_the_versions_not_known_then() {
    // docs/adr/0537: a version recorded on 10 and superseded on 20; one never recorded; one of a layer that
    // records nothing.
    use crate::Vec2;
    let mut s = Store::new();
    s.put_json(&format!(
        "[{},{},{}]",
        line(1.0, "a", 0.0),
        line(2.0, "a", 5.0),
        line(3.0, "a", 10.0)
    ))
    .unwrap();
    s.set_recorded([(1.0, Some((10.0, 20.0))), (2.0, None)], []);
    let near = |s: &Store, x: f64| s.near(Vec2::new(x, 5.0), 0.5).len();
    assert_eq!((near(&s, 0.0), near(&s, 5.0), near(&s, 10.0)), (1, 1, 1), "without a day all show");
    s.set_known_at(Some(15.0));
    assert_eq!(s.time_mask(&[1.0, 2.0, 3.0]), [1, 0, 1]);
    s.set_known_at(Some(20.0));
    assert_eq!(s.time_mask(&[1.0, 2.0, 3.0]), [0, 0, 1], "superseded on the day itself");
    s.set_known_at(Some(9.0));
    assert_eq!(s.time_mask(&[1.0]), [0], "not yet recorded");
    s.set_recorded([], [1.0, 2.0]);
    assert_eq!(s.time_mask(&[1.0, 2.0]), [1, 1], "of no recording layer");
}

#[test]
fn the_extent_is_the_union_of_the_boxes() {
    let mut s = Store::new();
    assert_eq!(s.extent(None), None);
    s.put_json(&format!(
        "[{},{},{}]",
        line(1.0, "a", 3.0),
        line(2.0, "a", -4.0),
        r#"{"id":3,"layerId":"a","attrs":{},"kind":"circle","c":{"x":50,"y":5},"r":2}"#
    ))
    .unwrap();
    let all = s.extent(None).unwrap();
    assert_eq!(
        (all.min_x, all.min_y, all.max_x, all.max_y),
        (-4.0, 0.0, 52.0, 10.0)
    );
    let some = s.extent(Some(&[1.0, 2.0, 99.0])).unwrap();
    assert_eq!((some.min_x, some.max_x), (-4.0, 3.0));
    assert_eq!(s.extent(Some(&[99.0])), None);
}

#[test]
fn keeps_the_documents_order_through_puts_and_removes() {
    let mut s = Store::new();
    let batch: Vec<String> = (1..=5)
        .map(|i| line(f64::from(i), "a", f64::from(i)))
        .collect();
    s.put_json(&format!("[{}]", batch.join(","))).unwrap();
    assert_eq!(s.ids(), [1.0, 2.0, 3.0, 4.0, 5.0]);
    // A known id keeps its place; a removed one that comes back (undo)
    // takes its place again (docs/adr/0020); a new one goes last.
    s.put_json(&format!("[{}]", line(2.0, "a", 20.0))).unwrap();
    s.remove(&[3.0]);
    s.put_json(&format!(
        "[{},{}]",
        line(3.0, "a", 3.0),
        line(6.0, "a", 6.0)
    ))
    .unwrap();
    assert_eq!(s.ids(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);
    assert_eq!(s.get(2.0).map(|it| it.bounds.min_x), Some(20.0));
    assert_eq!(s.len(), 6);
    s.clear();
    assert!(s.is_empty() && s.ids().is_empty());
}

#[test]
fn returning_ids_take_their_places_through_compaction() {
    let mut s = Store::new();
    let all: Vec<String> = (1..=600)
        .map(|i| line(f64::from(i), "a", f64::from(i)))
        .collect();
    s.put_json(&format!("[{}]", all.join(","))).unwrap();
    // Remove two thirds (every id not divisible by 3), enough to compact the order.
    let gone: Vec<f64> = (1..=600).filter(|i| i % 3 != 0).map(f64::from).collect();
    s.remove(&gone);
    assert_eq!(s.len(), 200);
    s.put_json(&format!("[{}]", line(601.0, "a", 601.0)))
        .unwrap();
    // They come back in reverse, as an undo puts them: each takes its own place.
    let back: Vec<String> = gone.iter().rev().map(|&i| line(i, "a", i)).collect();
    s.put_json(&format!("[{}]", back.join(","))).unwrap();
    let want: Vec<f64> = (1..=601).map(f64::from).collect();
    assert_eq!(s.ids(), want);
    // Queries answer in the same order.
    let near: Vec<f64> = s
        .candidates(&Bounds {
            min_x: 9.5,
            min_y: 0.0,
            max_x: 12.5,
            max_y: 10.0,
        })
        .iter()
        .map(|it| it.id)
        .collect();
    assert_eq!(near, [10.0, 11.0, 12.0]);
}

#[test]
fn candidates_are_the_same_before_and_after_a_rebuild() {
    let mut s = Store::new();
    let batch: Vec<String> = (0..600)
        .map(|i| line(f64::from(i), "a", f64::from(i) * 3.0))
        .collect();
    s.put_json(&format!("[{}]", batch.join(","))).unwrap();
    assert!(s.tree.is_some(), "a big batch builds the tree");
    s.put_json(&format!("[{}]", line(7.0, "a", 1000.0)))
        .unwrap();
    let q = Bounds {
        min_x: 995.0,
        min_y: -1.0,
        max_x: 1005.0,
        max_y: 1.0,
    };
    let before: Vec<f64> = s.candidates(&q).iter().map(|it| it.id).collect();
    s.rebuild();
    let after: Vec<f64> = s.candidates(&q).iter().map(|it| it.id).collect();
    assert_eq!(before, after);
    assert!(before.contains(&7.0) && before.contains(&333.0) && !before.contains(&100.0));
    let nan = Bounds {
        min_x: f64::NAN,
        ..q
    };
    assert_eq!(s.candidates(&nan).len(), 600);
}

#[test]
fn typed_puts_and_layers_are_the_json_ones() {
    let lines: Vec<String> = (1..=600)
        .map(|i| {
            line(
                f64::from(i),
                if i % 2 == 0 { "a" } else { "b" },
                f64::from(i),
            )
        })
        .collect();
    let mut json = Store::new();
    json.put_json(&format!("[{}]", lines.join(","))).unwrap();
    json.set_layers_json(
        r#"[{"id":"a","visible":false,"locked":true,"pickInterior":false},{"id":"b","visible":true,"locked":false,"pickInterior":true}]"#,
    )
    .unwrap();
    let mut typed = Store::new();
    let n = typed.put_many((1..=600).map(|i| {
        let x = f64::from(i);
        let layer = if i % 2 == 0 { "a" } else { "b" };
        let shape = Shape::Line {
            a: crate::Vec2::new(x, 0.0),
            b: crate::Vec2::new(x, 10.0),
        };
        (x, layer, false, shape)
    }));
    assert_eq!(n, 600);
    assert!(typed.tree.is_some(), "a big batch builds the tree");
    let hidden = LayerFlags {
        visible: false,
        locked: true,
        pick_interior: false,
        snap: u32::MAX,
    };
    typed.set_layers([("a", hidden), ("b", UNLISTED)]);
    assert_eq!(typed.ids(), json.ids());
    for id in [1.0, 2.0, 599.0, 600.0] {
        let (t, j) = (typed.get(id).unwrap(), json.get(id).unwrap());
        assert_eq!((&t.shape, t.bounds), (&j.shape, j.bounds));
        assert_eq!(typed.flags(t), json.flags(j));
    }
    let r = Bounds {
        min_x: 0.0,
        min_y: -1.0,
        max_x: 50.5,
        max_y: 11.0,
    };
    assert_eq!(typed.in_rect(&r, false), json.in_rect(&r, false));
    // A table that does not read leaves the one in place.
    assert!(
        json.set_layers_json(r#"[{"id":"a","visible":true}]"#)
            .is_err()
    );
    assert_eq!(json.flags(json.get(2.0).unwrap()), hidden);
}

#[test]
fn reads_layers_and_rejects_bad_input() {
    let mut s = Store::new();
    s.put_json(&format!("[{}]", line(1.0, "gizli", 0.0)))
        .unwrap();
    s.set_layers_json(r#"[{"id":"gizli","visible":false,"locked":true,"pickInterior":false}]"#)
        .unwrap();
    let it = s.get(1.0).unwrap();
    assert_eq!(
        s.flags(it),
        LayerFlags {
            visible: false,
            locked: true,
            pick_interior: false,
            snap: u32::MAX,
        }
    );
    s.set_layers_json("[]").unwrap();
    assert_eq!(s.flags(s.get(1.0).unwrap()), UNLISTED);
    assert!(
        s.put_json(r#"[{"kind":"point","p":{"x":0,"y":0}}]"#)
            .is_err()
    );
    assert!(s.set_layers_json(r#"[{"id":"a"}]"#).is_err());
}
