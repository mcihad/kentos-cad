//! A dense parcel and road map through the label engine (docs/adr/0533): how many labels are placed, left
//! unplaced, overlapping, repeated; and how many labels move when the view moves a little, without and with the
//! last placing remembered (`Prior`). The map is built here, from a seeded generator; the numbers are the ADR's
//! acceptance example. `-- --ignored --nocapture` prints them.

use std::fmt::Write as _;

use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::label_audit::{AuditedLabel, audit};
use kentos_geometry_core::store::labels::LABEL_STRIDE;
use kentos_geometry_core::store::placing::{
    LABEL_PLACED, ObjectLabels, PlaceOptions, Prior, Shown,
};

/// The generator: a small linear congruential sequence, so the map is the same everywhere.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> f64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        ((self.0 >> 33) as f64) / f64::from(1u32 << 31)
    }
}

/// The map's origin (metres) and extent, and the scale it is read at (px per metre).
const X0: f64 = 487_000.0;
const Y0: f64 = 4_420_000.0;
const COLS: usize = 10;
const ROWS: usize = 7;
const BLOCK: f64 = 100.0;
const STREET: f64 = 12.0;
pub const SCALE: f64 = 1.6;

/// The dense map: 70 blocks of a dozen parcels each (a fifth of them too narrow for their number), a building in
/// most of them, the streets between them named, in several pieces each, and survey points at the corners.
pub struct DenseMap {
    pub store: Store,
    pub parcels: usize,
    pub streets: usize,
}

pub fn dense_map() -> DenseMap {
    let mut rng = Lcg(20_261_019);
    let mut objects = String::from("[");
    let mut texts: Vec<(f64, String)> = Vec::new();
    let mut id = 0.0;
    let mut add = |objects: &mut String, body: String| {
        id += 1.0;
        if id > 1.0 {
            objects.push(',');
        }
        let _ = write!(objects, r#"{{"id":{id},{body}}}"#);
        id
    };
    let (mut parcels, mut streets) = (0, 0);
    for bx in 0..COLS {
        for by in 0..ROWS {
            let (x, y) = (
                X0 + bx as f64 * BLOCK + STREET / 2.0,
                Y0 + by as f64 * BLOCK + STREET / 2.0,
            );
            let side = BLOCK - STREET;
            // Two rows of six parcels across the block, their widths drawn: some narrow.
            for row in 0..2 {
                let (y0, y1) = (
                    y + row as f64 * side / 2.0,
                    y + (row + 1) as f64 * side / 2.0,
                );
                let mut widths: Vec<f64> = (0..6)
                    .map(|_| {
                        if rng.next() < 0.2 {
                            5.0 + rng.next() * 4.0
                        } else {
                            10.0 + rng.next() * 8.0
                        }
                    })
                    .collect();
                let total: f64 = widths.iter().sum();
                widths.iter_mut().for_each(|w| *w *= side / total);
                let mut cx = x;
                for (k, w) in widths.iter().enumerate() {
                    let pid = add(
                        &mut objects,
                        format!(
                            r#""layerId":"parsel","kind":"polygon","pts":[{{"x":{cx},"y":{y0}}},{{"x":{},"y":{y0}}},{{"x":{},"y":{y1}}},{{"x":{cx},"y":{y1}}}]"#,
                            cx + w,
                            cx + w
                        ),
                    );
                    texts.push((pid, format!("{}/{}", 100 + bx * ROWS + by, row * 6 + k + 1)));
                    parcels += 1;
                    // A building in most parcels: an obstacle for the labels, not labelled itself.
                    if rng.next() < 0.7 && *w > 9.0 {
                        let m = 2.0;
                        add(
                            &mut objects,
                            format!(
                                r#""layerId":"bina","kind":"polygon","pts":[{{"x":{},"y":{}}},{{"x":{},"y":{}}},{{"x":{},"y":{}}},{{"x":{},"y":{}}}]"#,
                                cx + m,
                                y0 + m,
                                cx + w - m,
                                y0 + m,
                                cx + w - m,
                                y1 - m,
                                cx + m,
                                y1 - m
                            ),
                        );
                    }
                    cx += w;
                }
            }
            // A survey point at the block's corner.
            let pid = add(
                &mut objects,
                format!(r#""layerId":"nokta","kind":"point","p":{{"x":{x},"y":{y}}}"#),
            );
            texts.push((pid, format!("P.{}", 1000 + bx * ROWS + by)));
        }
    }
    // The streets: each runs the map's whole length in three pieces with a slight bend, one name along it.
    let names = [
        "Atatürk Cd.",
        "Gül Sk.",
        "İnönü Blv.",
        "Çınar Sk.",
        "Mimar Sinan Cd.",
        "Lale Sk.",
        "Fatih Cd.",
        "Kavak Sk.",
    ];
    for r in 0..=ROWS {
        let y = Y0 + r as f64 * BLOCK;
        for piece in 0..3 {
            let (a, b) = (
                X0 + piece as f64 * COLS as f64 * BLOCK / 3.0,
                X0 + (piece + 1) as f64 * COLS as f64 * BLOCK / 3.0,
            );
            let bend = (rng.next() - 0.5) * 6.0;
            let pid = add(
                &mut objects,
                format!(
                    r#""layerId":"yol","kind":"polyline","pts":[{{"x":{a},"y":{y}}},{{"x":{},"y":{}}},{{"x":{b},"y":{y}}}]"#,
                    (a + b) / 2.0,
                    y + bend
                ),
            );
            texts.push((pid, names[r % names.len()].to_owned()));
        }
        streets += 1;
    }
    for c in 0..=COLS {
        let x = X0 + c as f64 * BLOCK;
        for piece in 0..3 {
            let (a, b) = (
                Y0 + piece as f64 * ROWS as f64 * BLOCK / 3.0,
                Y0 + (piece + 1) as f64 * ROWS as f64 * BLOCK / 3.0,
            );
            let bend = (rng.next() - 0.5) * 6.0;
            let pid = add(
                &mut objects,
                format!(
                    r#""layerId":"yol","kind":"polyline","pts":[{{"x":{x},"y":{a}}},{{"x":{},"y":{}}},{{"x":{x},"y":{b}}}]"#,
                    x + bend,
                    (a + b) / 2.0
                ),
            );
            texts.push((pid, names[(c + 3) % names.len()].to_owned()));
        }
        streets += 1;
    }
    // Two quarters (mahalle) each larger than the view with its margin, named at the pole of the part in view, and
    // a stream across the map named once along the part of it in view (no repeat).
    for (qx, name) in [(-150.0, "Cumhuriyet Mah."), (450.0, "Yeni Mah.")] {
        let (a, b) = (X0 + qx, X0 + qx + 650.0);
        let (c, d) = (Y0 - 50.0, Y0 + ROWS as f64 * BLOCK + 50.0);
        let pid = add(
            &mut objects,
            format!(
                r#""layerId":"mahalle","kind":"polygon","pts":[{{"x":{a},"y":{c}}},{{"x":{b},"y":{c}}},{{"x":{b},"y":{d}}},{{"x":{a},"y":{d}}}]"#
            ),
        );
        texts.push((pid, name.to_owned()));
    }
    let mut pts = String::new();
    for k in 0..=40 {
        let t = f64::from(k) / 40.0;
        let _ = write!(
            pts,
            r#"{}{{"x":{},"y":{}}}"#,
            if k > 0 { "," } else { "" },
            X0 - 100.0 + t * 1200.0,
            Y0 + 330.0 + 90.0 * kentos_geometry_core::jsmath::sin(t * 9.0) + t * 60.0
        );
    }
    let pid = add(
        &mut objects,
        format!(r#""layerId":"dere","kind":"polyline","pts":[{pts}]"#),
    );
    texts.push((pid, "Kırmızı Dere".to_owned()));
    objects.push(']');
    let mut store = Store::new();
    store.put_json(&objects).expect("the objects");
    store
        .set_label_layers_json(
            r#"[
              {"id":"parsel","rank":2,"label":{"placement":"center","size":8,"area":"parcel","inside":true,"minFeaturePx":12}},
              {"id":"bina","rank":3,"labels":{"mode":"off","obstacle":{"weight":4}}},
              {"id":"yol","rank":0,"label":{"placement":"along","size":9,"line":"parallel","repeat":420,"mergeLines":true,"duplicates":160,"priority":8}},
              {"id":"nokta","rank":1,"point":4,"label":{"placement":"beside","size":8,"point":"around"}},
              {"id":"mahalle","rank":5,"label":{"placement":"center","size":16,"area":"free","priority":2}},
              {"id":"dere","rank":4,"label":{"placement":"along","size":10,"line":"curved","priority":6}}
            ]"#,
        )
        .expect("the layers");
    for (id, text) in &texts {
        store.set_object_labels(
            *id,
            ObjectLabels {
                texts: vec![(0, text.clone())],
                z: f64::NAN,
            },
        );
    }
    DenseMap {
        store,
        parcels,
        streets,
    }
}

/// The view `dx`, `dy` px from the base one, 800 × 560 px (500 × 350 m).
pub fn window(dx: f64, dy: f64) -> Bounds {
    let (x, y) = (X0 + 150.0 + dx / SCALE, Y0 + 120.0 + dy / SCALE);
    Bounds {
        min_x: x,
        min_y: y,
        max_x: x + 500.0,
        max_y: y + 350.0,
    }
}

pub fn options() -> PlaceOptions {
    PlaceOptions {
        unplaced: true,
        hidden: false,
    }
}

/// Counts of an audit.
#[derive(Debug, Default, PartialEq)]
pub struct Counts {
    pub placed: usize,
    pub unplaced: usize,
    pub forced: usize,
    pub collisions: usize,
    pub repeats: usize,
}

pub fn counts(labels: &[AuditedLabel]) -> Counts {
    Counts {
        placed: labels.iter().filter(|l| l.placed()).count(),
        unplaced: labels.iter().filter(|l| l.unplaced()).count(),
        forced: labels.iter().filter(|l| l.forced()).count(),
        collisions: labels.iter().filter(|l| l.collides).count(),
        repeats: labels.iter().filter(|l| l.repeats).count(),
    }
}

/// The placed labels' middles, ground, with their object and class.
fn middles(shown: &Shown) -> Vec<(f64, u16, [f64; 2])> {
    shown
        .records
        .chunks_exact(LABEL_STRIDE)
        .filter(|r| r[1] == LABEL_PLACED && (r[8] as u32 & 6) == 0)
        .map(|r| (r[0], r[7] as u16, [r[2], r[3]]))
        .collect()
}

/// How many of the labels of `base` still in sight after the view moved to `now` are where they were (within
/// 1.5 px), moved, or gone: (common, moved, gone).
pub fn drift(
    base: &Shown,
    now: &Shown,
    view_now: &Bounds,
    view_base: &Bounds,
) -> (usize, usize, usize) {
    let (c, m, g, _) = drift_of(base, now, view_now, view_base);
    (c, m, g)
}

/// [`drift`] with the labels that moved: object, class and how far, px.
pub fn drift_of(
    base: &Shown,
    now: &Shown,
    view_now: &Bounds,
    view_base: &Bounds,
) -> (usize, usize, usize, Vec<(f64, u16, f64)>) {
    let mut who = Vec::new();
    let after = middles(now);
    let (mut common, mut moved, mut gone) = (0, 0, 0);
    // Only labels well inside both views count: the ones at an edge leave or enter by the move itself.
    let edge = 60.0 / SCALE;
    let inside = |p: [f64; 2], v: &Bounds| {
        p[0] > v.min_x + edge
            && p[0] < v.max_x - edge
            && p[1] > v.min_y + edge
            && p[1] < v.max_y - edge
    };
    for (id, class, p) in middles(base) {
        if !inside(p, view_now) || !inside(p, view_base) {
            continue;
        }
        let best = after
            .iter()
            .filter(|(i, c, _)| *i == id && *c == class)
            .map(|(_, _, q)| {
                let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
                (dx * dx + dy * dy).sqrt() * SCALE
            })
            .fold(f64::INFINITY, |least, d| if d < least { d } else { least });
        if best.is_infinite() {
            gone += 1;
        } else if best > 1.5 {
            moved += 1;
            who.push((id, class, best));
            common += 1;
        } else {
            common += 1;
        }
    }
    (common, moved, gone, who)
}

fn place(map: &DenseMap, view: &Bounds, prior: Option<&Prior>) -> (Shown, Prior) {
    map.store
        .place_labels_after(view, SCALE, &[], options(), prior)
}

/// The acceptance example: the map's labels, counted. No label crosses another and none is forced over one
/// (no class here may overlap); the narrow parcels have no room for their numbers and are left out, said so.
#[test]
fn the_dense_map_is_placed_without_collisions() {
    let map = dense_map();
    assert_eq!(map.parcels, 840);
    assert_eq!(map.streets, 19);
    let view = window(0.0, 0.0);
    let (shown, _) = place(&map, &view, None);
    let ids = |id: f64| u32::from(id < 5_000.0);
    let labels = audit(&shown, SCALE, &ids);
    let c = counts(&labels);
    println!(
        "{} parsel, {} sokak: yerleşen {}, yerleşemeyen {}, zorla {}, örtüşen {}, yinelenen {}",
        map.parcels, map.streets, c.placed, c.unplaced, c.forced, c.collisions, c.repeats
    );
    assert!(c.placed > 300, "{c:?}");
    assert_eq!(c.collisions, 0, "{c:?}");
    assert_eq!(c.forced, 0, "{c:?}");
    assert!(c.unplaced > 0, "the narrow parcels have no room: {c:?}");
}

/// A pan of a few pixels moves the labels with the ground: over 24 pans of up to 40 px, with the last placing
/// remembered no label still in sight moves (one is kept where it was while it fits and is free), where a fresh
/// placing re-chooses the place of the long strips' names (an area's pole is anywhere along its ridge).
#[test]
fn a_small_pan_keeps_the_labels_where_they_were() {
    let map = dense_map();
    let base_view = window(0.0, 0.0);
    let (base, prior) = place(&map, &base_view, None);
    let mut rng = Lcg(77);
    let (mut fresh_moved, mut kept_moved, mut common_all, mut worst) = (0, 0, 0, 0);
    for _ in 0..24 {
        let (dx, dy) = ((rng.next() - 0.5) * 80.0, (rng.next() - 0.5) * 80.0);
        let view = window(dx, dy);
        let (fresh, _) = place(&map, &view, None);
        let (kept, _) = place(&map, &view, Some(&prior));
        let (common, moved, _) = drift(&base, &fresh, &view, &base_view);
        let (_, moved2, _) = drift(&base, &kept, &view, &base_view);
        fresh_moved += moved;
        kept_moved += moved2;
        common_all += common;
        worst = worst.max(moved);
    }
    println!(
        "24 kaydırma, ortak {common_all}: yeni yerleşimde kayan {fresh_moved} (bir kaydırmada en çok {worst}), önceki korunarak {kept_moved}"
    );
    assert!(common_all > 3000);
    assert_eq!(kept_moved, 0, "{kept_moved} of {common_all} moved");
    assert!(fresh_moved > kept_moved, "the pans moved nothing to keep");
}

/// docs/adr/0636 §8: a long pan, each step from the last placing: an area's name at its pole (the quarters) and a
/// name along a curve (the stream) move only when the view leaves no room where they were; well inside two views in a
/// row they stay. `-- --nocapture` prints the counts.
#[test]
fn a_long_pan_moves_area_and_curved_names_only_when_they_must() {
    let map = dense_map();
    // The quarters' and the stream's labels: the last objects put (two quarters, then the stream).
    let ids: Vec<f64> = {
        let n = map.store.len() as f64;
        vec![n - 2.0, n - 1.0, n]
    };
    let mut view = window(-200.0, 0.0);
    let (mut last, mut prior) = place(&map, &view, None);
    let (mut seen, mut jumps, mut fresh_jumps) = (0, 0, 0);
    for _ in 0..30 {
        let next = Bounds {
            min_x: view.min_x + 40.0 / SCALE,
            max_x: view.max_x + 40.0 / SCALE,
            min_y: view.min_y + 6.0 / SCALE,
            max_y: view.max_y + 6.0 / SCALE,
        };
        let (now, p) = place(&map, &next, Some(&prior));
        let (fresh, _) = place(&map, &next, None);
        let (_, _, _, moved) = drift_of(&last, &now, &next, &view);
        let (_, _, _, fresh_moved) = drift_of(&last, &fresh, &next, &view);
        let ours =
            |list: &[(f64, u16, f64)]| list.iter().filter(|(id, _, _)| ids.contains(id)).count();
        seen += middles(&last)
            .iter()
            .filter(|(id, _, _)| ids.contains(id))
            .count();
        jumps += ours(&moved);
        fresh_jumps += ours(&fresh_moved);
        (last, prior, view) = (now, p, next);
    }
    println!(
        "uzun kaydırma: alan ve eğri adları {seen} kez görüldü; önceki korunarak sıçrayan {jumps}, yeni yerleşimde {fresh_jumps}"
    );
    assert!(seen > 30, "the names were not in sight");
    assert_eq!(
        jumps, 0,
        "an area's or a curve's name jumped while it had room"
    );
}
