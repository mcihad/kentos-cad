//! A view's text asked near it and object by object (docs/adr/0786): a table's cells near a box are the layout's
//! cells that start in it (and its first), to the bit; the records a memo gives back after random changes are the
//! records read afresh, to the bit.

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::table::table_geom;
use kentos_geometry_core::geometry::Bounds;
use kentos_geometry_core::jsmath::js_max;
use kentos_geometry_core::store::Store;
use kentos_geometry_core::store::label_memo::LabelMemo;
use kentos_geometry_core::store::labels::{LABEL_CELL, LABEL_STRIDE};
use kentos_geometry_core::store::legible::{LABEL_SHOWN_STRIDE, LabelSize};
use kentos_geometry_core::store::placing::PlaceOptions;

/// Knuth's MMIX generator: the same draws everywhere.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> u64 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        self.0 >> 11
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n.max(1)
    }
    fn unit(&mut self) -> f64 {
        (self.next() as f64) / ((1u64 << 53) as f64)
    }
    fn within(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.unit()
    }
}

/// A random table as the store reads it: rows of 0.2–1 m, 2–6 columns, some cells empty, some merged, turned, a
/// heading row, aligned columns and a style over a few.
fn table(d: &mut Dice, id: u64, at: Vec2) -> String {
    let rows = 5 + d.below(400) as usize;
    let cols = 2 + d.below(5) as usize;
    let heights: Vec<String> = (0..rows)
        .map(|_| format!("{}", d.within(0.2, 1.0)))
        .collect();
    let widths: Vec<String> = (0..cols)
        .map(|_| format!("{}", d.within(1.0, 4.0)))
        .collect();
    let mut cells = Vec::new();
    for i in 0..rows {
        let row: Vec<String> = (0..cols)
            .map(|j| match d.below(9) {
                0 => "\"\"".to_owned(),
                1 => format!("\"{}\"", "Salon ".repeat(1 + d.below(4) as usize)),
                _ => format!("\"{i}.{j} {}\"", d.below(10_000)),
            })
            .collect();
        cells.push(format!("[{}]", row.join(",")));
    }
    let mut merges = Vec::new();
    for _ in 0..d.below(4) {
        let (r, c) = (
            d.below(rows as u64 - 1) as usize,
            d.below(cols as u64 - 1) as usize,
        );
        let (rr, cc) = (1 + d.below(6) as usize, 1 + d.below(2) as usize);
        if merges
            .iter()
            .all(|(r0, c0, rr0, cc0): &(usize, usize, usize, usize)| {
                r + rr <= *r0 || *r0 + rr0 <= r || c + cc <= *c0 || *c0 + cc0 <= c
            })
            && r + rr <= rows
            && c + cc <= cols
        {
            merges.push((r, c, rr, cc));
        }
    }
    let merges: Vec<String> = merges
        .iter()
        .map(|(r, c, rr, cc)| format!("{{\"row\":{r},\"col\":{c},\"rows\":{rr},\"cols\":{cc}}}"))
        .collect();
    let rotation = [0.0, 0.0, 30.0, 90.0, 200.0, d.within(-180.0, 180.0)][d.below(6) as usize];
    let aligns: Vec<&str> = (0..cols)
        .map(|_| ["\"left\"", "\"center\"", "\"right\""][d.below(3) as usize])
        .collect();
    format!(
        "{{\"id\":{id},\"layerId\":\"cizelge\",\"kind\":\"table\",\"p\":{{\"x\":{},\"y\":{}}},\"rotation\":{rotation},\
         \"height\":{},\"rows\":[{}],\"columns\":[{}],\"cells\":[{}],\"merges\":[{}],\"aligns\":[{}],\"header\":{},\
         \"styles\":[{{\"row\":{},\"col\":0,\"rows\":3,\"cols\":2,\"align\":\"right\",\"bold\":true}}]}}",
        at.x,
        at.y,
        d.within(0.1, 0.3),
        heights.join(","),
        widths.join(","),
        cells.join(","),
        merges.join(","),
        aligns.join(","),
        d.below(2) == 0,
        d.below(rows as u64),
    )
}

/// A random text.
fn text(d: &mut Dice, id: u64, at: Vec2) -> String {
    format!(
        "{{\"id\":{id},\"layerId\":\"yazi\",\"kind\":\"text\",\"p\":{{\"x\":{},\"y\":{}}},\"text\":\"Mahal {}\",\
         \"height\":{},\"rotation\":{}}}",
        at.x,
        at.y,
        d.below(1000),
        d.within(0.1, 0.5),
        d.within(0.0, 360.0),
    )
}

fn around(d: &mut Dice, near: f64) -> Vec2 {
    Vec2::new(d.within(-near, near), d.within(-near, near))
}

fn boxed(d: &mut Dice, near: f64) -> Bounds {
    let c = around(d, near);
    let (w, h) = (d.within(1.0, 80.0), d.within(1.0, 80.0));
    Bounds {
        min_x: c.x - w / 2.0,
        min_y: c.y - h / 2.0,
        max_x: c.x + w / 2.0,
        max_y: c.y + h / 2.0,
    }
}

/// The layout's cells, those that start in `near` and the first: what `cells_near` gives, to the bit.
#[test]
fn the_cells_near_a_box_are_the_layouts_that_start_in_it() {
    let mut d = Dice(0x0786_2026);
    let mut kept = 0;
    let mut store = Store::new();
    for k in 0..60u64 {
        let at = around(&mut d, 50.0);
        store
            .put_json(&format!("[{}]", table(&mut d, k + 1, at)))
            .unwrap();
        let shape = &store.get((k + 1) as f64).unwrap().shape;
        let t = table_geom(shape).unwrap();
        let font = store.font();
        let all = t.layout(font).cells;
        for _ in 0..12 {
            let near = boxed(&mut d, 120.0);
            let inside = |p: Vec2| {
                p.x >= near.min_x && p.x <= near.max_x && p.y >= near.min_y && p.y <= near.max_y
            };
            let want: Vec<_> = all
                .iter()
                .enumerate()
                .filter(|(i, c)| *i == 0 || inside(c.at))
                .map(|(_, c)| c.clone())
                .collect();
            let got = t.cells_near(font, &near);
            assert_eq!(got, want, "table {k}, box {near:?}");
            kept += got.len().saturating_sub(1);
        }
    }
    // Most boxes hold some of a table's cells, and few all of them.
    assert!(kept > 1_000, "{kept}");
}

/// Every record of a view's text, as numbers to the bit.
fn bits(records: &[f64]) -> Vec<u64> {
    records.iter().map(|x| x.to_bits()).collect()
}

fn ask(
    store: &Store,
    view: &Bounds,
    scale: f64,
    cells: Option<&Bounds>,
    memo: Option<&mut LabelMemo>,
) -> (Vec<u64>, Vec<String>) {
    let (shown, _) = store
        .labels_shown_parts(
            view,
            scale,
            None,
            LabelSize::Print,
            PlaceOptions::default(),
            None,
            None,
            cells,
            memo,
        )
        .flat();
    (bits(&shown.records), shown.texts)
}

/// Random changes (tables and texts put, put again as they were, moved, taken away, new ones; another view, another
/// box of cells, another scale), each followed by a view's ask: the memo's records are a fresh read's; near a box, a
/// table's cells are the whole read's that start in it and its first.
#[test]
fn the_memos_records_are_a_fresh_reads() {
    let mut d = Dice(0x5eed_0786);
    let mut store = Store::new();
    let mut objects: Vec<(u64, String)> = Vec::new();
    for id in 1..=40u64 {
        let at = around(&mut d, 60.0);
        let json = if id % 3 == 0 {
            table(&mut d, id, at)
        } else {
            text(&mut d, id, at)
        };
        objects.push((id, json));
    }
    let all: Vec<&str> = objects.iter().map(|(_, j)| j.as_str()).collect();
    store.put_json(&format!("[{}]", all.join(","))).unwrap();
    let mut memo = LabelMemo::default();
    let mut next = 41u64;
    let (mut view, mut scale) = (boxed(&mut d, 30.0), 40.0);
    let mut taken = 0;
    let mut blocks = 0;
    let mut whole_cells = 0;
    for step in 0..120 {
        match d.below(10) {
            // A text or a table changed: the same id put with another shape.
            0 | 1 => {
                let i = d.below(objects.len() as u64) as usize;
                let id = objects[i].0;
                let at = around(&mut d, 60.0);
                objects[i].1 = if objects[i].1.contains("\"table\"") {
                    table(&mut d, id, at)
                } else {
                    text(&mut d, id, at)
                };
                store.put_json(&format!("[{}]", objects[i].1)).unwrap();
            }
            // Put again as it was (an undo's, another editor's): read again, the same records.
            2 => {
                let i = d.below(objects.len() as u64) as usize;
                store.put_json(&format!("[{}]", objects[i].1)).unwrap();
            }
            // Taken away, a new one.
            3 if objects.len() > 5 => {
                let i = d.below(objects.len() as u64) as usize;
                let (id, _) = objects.remove(i);
                store.remove(&[id as f64]);
            }
            4 => {
                let at = around(&mut d, 60.0);
                let json = if d.below(2) == 0 {
                    table(&mut d, next, at)
                } else {
                    text(&mut d, next, at)
                };
                store.put_json(&format!("[{json}]")).unwrap();
                objects.push((next, json));
                next += 1;
            }
            // A few rows' words of a table changed, the rest as they were: its other blocks are taken as kept.
            7 | 8 => {
                let tables: Vec<usize> = (0..objects.len())
                    .filter(|i| objects[*i].1.contains("\"table\""))
                    .collect();
                if let Some(&i) = tables.get(d.below(tables.len().max(1) as u64) as usize) {
                    for _ in 0..1 + d.below(3) {
                        let row = d.below(400);
                        objects[i].1 = objects[i]
                            .1
                            .replace(&format!("\"{row}."), &format!("\"{row}.x"));
                    }
                    store.put_json(&format!("[{}]", objects[i].1)).unwrap();
                }
            }
            // Another view, another scale.
            5 => view = boxed(&mut d, 30.0),
            6 => scale = d.within(5.0, 200.0),
            _ => {}
        }
        let cells = (step % 4 != 0).then(|| {
            let side = js_max(view.max_x - view.min_x, view.max_y - view.min_y) * 0.55;
            Bounds {
                min_x: view.min_x - side,
                min_y: view.min_y - side,
                max_x: view.max_x + side,
                max_y: view.max_y + side,
            }
        });
        let kept = ask(&store, &view, scale, cells.as_ref(), Some(&mut memo));
        taken += memo.taken().0;
        blocks += memo.blocks_taken();
        let fresh = ask(&store, &view, scale, cells.as_ref(), None);
        assert_eq!(kept, fresh, "step {step}");
        // The whole read, its cells that start in the box and each table's first: the near read.
        if let Some(near) = &cells {
            let (whole, texts) = ask(&store, &view, scale, None, None);
            let mut seen = std::collections::HashSet::new();
            let mut want = Vec::new();
            for r in whole.chunks_exact(LABEL_SHOWN_STRIDE) {
                let f = |i: usize| f64::from_bits(r[i]);
                if f(1) == LABEL_CELL {
                    let inside = f(2) >= near.min_x
                        && f(2) <= near.max_x
                        && f(3) >= near.min_y
                        && f(3) <= near.max_y;
                    if !seen.insert(r[0]) && !inside {
                        whole_cells += 1;
                        continue;
                    }
                }
                want.extend_from_slice(r);
            }
            assert_eq!((want, texts), fresh, "step {step}");
            assert_eq!(LABEL_SHOWN_STRIDE, LABEL_STRIDE + 3);
        }
    }
    // Many objects were taken as kept (a view meets a few of them); the boxes left cells out.
    assert!(taken > 300, "{taken}");
    assert!(blocks > 10, "{blocks}");
    assert!(whole_cells > 1_000, "{whole_cells}");
}
