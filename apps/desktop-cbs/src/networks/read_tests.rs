//! A network read again object by object is the whole read (docs/adr/0641 §3): on the network traces' drawing
//! (fixtures/interaction/v1/networks.kcad), many random edits to the streets, the pipes, the valves and the depot, each
//! read of the changed objects put together with the last input is the input a whole read gives, and the graph rebuilt
//! with the items it says is the graph a whole build gives.

use std::collections::HashSet;

use kentos_commands::{DocumentSnapshotV1, Entity, Vec2};
use kentos_domain::{Changes, Document, Slot};
use kentos_geometry_core::ops::network::Graph;
use kentos_interaction::Spatial;
use kentos_processing::network::rules_of;

use super::read::{Inputs, assemble, read_changed, read_whole, readable_in_part};

const DRAWING: &str = include_str!("../../../../fixtures/interaction/v1/networks.kcad");

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        (self.0.wrapping_mul(0x2545_f491_4f6c_dd1d) >> 11) as f64 / (1u64 << 53) as f64
    }

    fn below(&mut self, n: usize) -> usize {
        ((self.next() * n as f64) as usize).min(n - 1)
    }
}

fn shifted(e: &Entity, dx: f64, dy: f64, all: bool) -> Entity {
    let mut e = e.clone();
    let by = |p: &mut Vec2| {
        p.x += dx;
        p.y += dy;
    };
    match &mut e {
        Entity::Point(p) => by(&mut p.p),
        Entity::Polyline(l) if all => l.pts.iter_mut().for_each(by),
        // One end only: the street comes off its crossing or onto another.
        Entity::Polyline(l) => {
            if let Some(p) = l.pts.last_mut() {
                by(p);
            }
        }
        _ => {}
    }
    e
}

/// One write: an object of the drawing moved (whole or one end), an attribute changed, a copy drawn, one deleted, one
/// moved to another layer.
fn write(doc: &mut Document, r: &mut Rng) {
    let all: Vec<(Slot, Entity)> = doc
        .entities()
        .map(|e| (Slot(e.base().id), e.clone()))
        .collect();
    if all.is_empty() {
        return;
    }
    let (slot, e) = &all[r.below(all.len())];
    match r.below(7) {
        0 | 1 => {
            let d = [0.0, 0.004, 3.0, -40.0][r.below(4)];
            doc.update(*slot, shifted(e, d, d * 0.5, r.next() < 0.5));
        }
        2 => {
            let mut e = e.clone();
            let (field, values): (&str, &[&str]) = match r.below(3) {
                0 => ("hiz", &["30", "50", "", "x"]),
                1 => ("durum", &["kapalı", "açık", ""]),
                _ => ("yon", &["FT", "TF", "N", ""]),
            };
            e.base_mut()
                .attrs
                .insert(field.into(), values[r.below(values.len())].into());
            doc.update(*slot, e);
        }
        3 => {
            let _ = doc.add(shifted(e, 7.0, 3.0, true));
        }
        4 => {
            doc.remove(&[*slot]);
        }
        _ => {
            let mut e = e.clone();
            e.base_mut().layer_id = ["yol", "su", "vana", "cizim"][r.below(4)].into();
            doc.update(*slot, e);
        }
    }
}

#[test]
fn a_read_of_the_changed_objects_is_the_whole_read_and_builds_the_same_graph() {
    let snapshot = DocumentSnapshotV1::from_json(DRAWING).expect("the drawing reads");
    let defs = snapshot.settings.networks.clone();
    let mut read_again = 0;
    for def in &defs {
        assert!(readable_in_part(def), "{}", def.name);
        let rules = rules_of(def).expect("rules");
        for seed in 1..=12u64 {
            let mut doc = Document::from_snapshot(snapshot.clone()).expect("opens");
            let mut spatial = Spatial::of(&doc);
            let mut r = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15));
            let (whole, mut members) = read_whole(def, &doc, spatial.store());
            assert!(!whole.edges.is_empty(), "{}: kenarlar okundu", def.name);
            let mut inputs = Inputs {
                edges: whole.edges,
                junctions: whole.junctions,
                turns: whole.turns,
            };
            let (mut graph, _) = Graph::build_kept(
                rules.clone(),
                &inputs.edges,
                &inputs.junctions,
                &inputs.turns,
            );
            let mut mark = doc.change_mark();
            for step in 0..20 {
                for _ in 0..1 + r.below(3) {
                    write(&mut doc, &mut r);
                }
                spatial.sync(&doc);
                let Changes::Slots(slots) = doc.changes_since(mark) else {
                    panic!("the journal names the slots");
                };
                let changed: HashSet<u32> = slots.iter().map(|s| s.0).collect();
                let (part, now) = read_changed(
                    def,
                    &doc,
                    spatial.store(),
                    std::mem::take(&mut members),
                    &changed,
                )
                .expect("read in part");
                let (whole, whole_members) = read_whole(def, &doc, spatial.store());
                let at = format!("{}, tohum {seed}, adım {step}", def.name);
                assert_eq!(now, whole_members, "{at}: üyeler");
                assert_eq!(part.skipped, whole.skipped, "{at}: alınmayanlar");
                assert_eq!(part.problems, whole.problems, "{at}: sözler");
                read_again += part.edges.len() + part.junctions.len();
                let a = assemble(inputs, part);
                assert_eq!(a.inputs.edges, whole.edges, "{at}: kenarlar");
                assert_eq!(a.inputs.junctions, whole.junctions, "{at}: düğümler");
                assert_eq!(a.inputs.turns, whole.turns, "{at}: dönüşler");
                let (built, built_report) =
                    Graph::build_with(rules.clone(), &whole.edges, &whole.junctions, &whole.turns);
                let (again, report, _) = graph.rebuild_known(
                    rules.clone(),
                    &a.inputs.edges,
                    &a.inputs.junctions,
                    &a.inputs.turns,
                    a.edge_was,
                    a.junction_was,
                );
                assert_eq!(report, built_report, "{at}: rapor");
                assert_eq!(again.nodes, built.nodes, "{at}: düğüm yerleri");
                assert_eq!(again.pieces, built.pieces, "{at}: parçalar");
                assert_eq!(again.geom, built.geom, "{at}: geometri");
                assert_eq!(again.edges, built.edges, "{at}: kenarlar");
                assert_eq!(again.junctions, built.junctions, "{at}: düğüm noktaları");
                assert_eq!(
                    (&again.fw, &again.bw),
                    (&built.fw, &built.bw),
                    "{at}: maliyetler"
                );
                graph = again;
                inputs = a.inputs;
                members = now;
                mark = doc.change_mark();
            }
        }
    }
    assert!(read_again > 100, "yeniden okunan {read_again}");
}

/// What reading a city's network takes on the interface's thread, whole and after a street moved, and what putting
/// the input together and rebuilding takes on the network thread (docs/adr/0641 §4): the grid city of
/// `network::timing` (225 × 225 crossings, 100 800 streets) as a drawing. Release, by hand:
/// `cargo test --release -p kentos-desktop --lib networks::read_tests::timing -- --ignored --nocapture`;
/// `KENTOS_PERF_GRID` another grid.
#[test]
#[ignore = "timings, run by hand in release"]
fn timing() {
    use std::time::Instant;
    let n: usize = std::env::var("KENTOS_PERF_GRID")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(225);
    let step = 50.0;
    let mut entities = Vec::new();
    let at = |i: usize, j: usize| serde_json::json!({"x": i as f64 * step, "y": j as f64 * step});
    for i in 0..n {
        for j in 0..n {
            for (a, b) in [((i, j), (i + 1, j)), ((i, j), (i, j + 1))] {
                if b.0 >= n || b.1 >= n {
                    continue;
                }
                let id = entities.len() + 1;
                let speed = ["30", "50", "70"][id % 3];
                entities.push(
                    serde_json::json!({"kind":"polyline","id":id,"layerId":"yol",
                    "attrs":{"hiz": speed},"pts":[at(a.0, a.1), at(b.0, b.1)]}),
                );
            }
        }
    }
    let streets = entities.len();
    let snapshot = serde_json::json!({"format":"kentos.document","version":1,"name":"Şehir",
        "settings":{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":1000,
            "networks":[{"id":"yollar","name":"Yollar","kind":"road","edges":[{"layer":"yol"}],"connect":"ends",
                "tolerance":0.01,"direction":{"kind":"both"},"costs":[{"name":"Süre","kind":"speed","field":"hiz","speed":50}]}]},
        "origin":{"x":0,"y":0},"layers":[{"id":"yol","name":"Yol","type":"layer","visible":true,"locked":false,
            "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"yol","entities":entities,"styles":{"items":[],"categories":[]}});
    let snapshot = DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads");
    let def = snapshot.settings.networks[0].clone();
    let rules = rules_of(&def).expect("rules");
    let mut doc = Document::from_snapshot(snapshot).expect("opens");
    let mut spatial = Spatial::of(&doc);
    let row = |what: &str, ms: f64| println!("{what:<58} {ms:>9.3} ms");
    let t = Instant::now();
    let (whole, members) = read_whole(&def, &doc, spatial.store());
    assert_eq!(whole.edges.len(), streets);
    row(
        &format!(
            "baştan okuma ({streets} sokak, {} kenar, {} alınmadı, {:?})",
            whole.edges.len(),
            whole.skipped.len(),
            whole.problems
        ),
        t.elapsed().as_secs_f64() * 1e3,
    );
    let inputs = Inputs {
        edges: whole.edges,
        junctions: whole.junctions,
        turns: whole.turns,
    };
    let (mut graph, _) = Graph::build_kept(
        rules.clone(),
        &inputs.edges,
        &inputs.junctions,
        &inputs.turns,
    );
    let mark = doc.change_mark();
    // A street in the middle: one end moved three metres.
    let slot = Slot((streets / 2) as u32);
    let moved = shifted(doc.get(slot).expect("street"), 3.0, 2.0, false);
    doc.update(slot, moved);
    let t = Instant::now();
    spatial.sync(&doc);
    row(
        "deponun izlemesi (bir sokak)",
        t.elapsed().as_secs_f64() * 1e3,
    );
    let Changes::Slots(slots) = doc.changes_since(mark) else {
        panic!("the journal names the slot");
    };
    let changed: HashSet<u32> = slots.iter().map(|s| s.0).collect();
    let t = Instant::now();
    let (part, _) = read_changed(&def, &doc, spatial.store(), members, &changed).expect("in part");
    row("yalnız değişenin okunması", t.elapsed().as_secs_f64() * 1e3);
    let t = Instant::now();
    let (whole, _) = read_whole(&def, &doc, spatial.store());
    row(
        "karşılaştırma için baştan okuma",
        t.elapsed().as_secs_f64() * 1e3,
    );
    let t = Instant::now();
    let a = assemble(inputs, part);
    row(
        "ağın iş parçacığında girdinin birleştirilmesi",
        t.elapsed().as_secs_f64() * 1e3,
    );
    assert_eq!(a.inputs.edges, whole.edges);
    let t = Instant::now();
    let (rebuilt, _, how) = graph.rebuild_known(
        rules,
        &a.inputs.edges,
        &a.inputs.junctions,
        &a.inputs.turns,
        a.edge_was,
        a.junction_was,
    );
    row(
        &format!(
            "yeniden kurma ({} parça, {} kenar yeniden kesildi{})",
            rebuilt.pieces.len(),
            how.recut,
            if how.anew { ", baştan" } else { "" }
        ),
        t.elapsed().as_secs_f64() * 1e3,
    );
}
