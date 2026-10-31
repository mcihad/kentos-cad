//! The design change reads only what it reaches and answers as before, to the bit (docs/adr/0782 §5): on the 2+1
//! apartment copied six times, random edits (walls moved, walls added straight, arced, bridging two flats or round
//! them all, walls removed) and after each, every bound slab's outline found among the wall pieces round its seed is the
//! one found among every wall of its level, so is a random seed's in both modes, and every computed table is as its own
//! working out leaves it (the schedules worked out once for the alike ones).

use kentos_commands::Entity;
use kentos_headless::{Op, Session};
use serde_json::{Value, json};

use super::flat_timing::{copied, drawn};

/// A small, fixed pseudo-random sequence (64-bit LCG): the same edits every run.
struct Dice(u64);

impl Dice {
    fn next(&mut self) -> f64 {
        self.0 = self.0.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1_442_695_040_888_963_407);
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn within(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[((self.next() * items.len() as f64) as usize).min(items.len() - 1)]
    }
}

fn walls(s: &Session) -> Vec<String> {
    s.document()
        .entities_with_uids()
        .filter(|(_, e)| matches!(e, Entity::Wall(_)))
        .map(|(u, _)| u.to_string())
        .collect()
}

/// How many bound slabs read fewer walls than their level's: the pieces at work.
fn checked(s: &Session, dice: &mut Dice, level: Option<&str>, at: &str) -> usize {
    let doc = s.document();
    let mut slabs = 0;
    let mut fewer = 0;
    for e in doc.entities() {
        if let Entity::Slab(x) = e
            && let Some(b) = &x.slab.bound
            && let Some(seed) = b.seed
        {
            let room = b.face == kentos_commands::SlabFace::Room;
            let level = x.slab.arch.level.as_deref();
            assert_eq!(
                doc.slab_from_walls(seed, level, room),
                doc.slab_from_every_wall(seed, level, room),
                "{at}: slab {}",
                x.base.id
            );
            slabs += 1;
            fewer += usize::from(kentos_domain::wall_pieces::walls_round(doc, seed, level).is_some());
        }
        if let Entity::Table(t) = e {
            assert_eq!(doc.table_worked_out(t), *t, "{at}: table {} as its working out leaves it", t.base.id);
        }
    }
    assert!(slabs > 0, "{at}: the flats' slabs");
    for _ in 0..12 {
        let seed = kentos_commands::Vec2 { x: dice.within(-6.0, 90.0), y: dice.within(-6.0, 14.0) };
        for room in [false, true] {
            assert_eq!(
                doc.slab_from_walls(seed, level, room),
                doc.slab_from_every_wall(seed, level, room),
                "{at}: seed {seed:?}, room {room}"
            );
        }
    }
    fewer
}

fn tried(s: &mut Session, command: &str, input: Value) -> bool {
    s.run(command, None, Op::Execute, input).is_ok_and(|a| a["status"] == "completed")
}

#[test]
fn the_pieces_round_a_seed_answer_as_every_wall_does() {
    let (mut s, names) = drawn();
    copied(&mut s, 6);
    let shell = s.entity(&names["shell"]).expect("the shell")["entity"].clone();
    let level = shell["arch"]["level"].as_str().map(str::to_owned);
    let layer = shell["layerId"].as_str().expect("a layer").to_owned();
    let arch = json!({ "level": level });
    let mut dice = Dice(0x0782_2026);
    assert_eq!(checked(&s, &mut dice, level.as_deref(), "drawn"), 6, "each flat's slab reads its own flat");
    let mut done = 0;
    let mut fewer = 0;
    for step in 0..48 {
        let all = walls(&s);
        let roll = dice.next();
        let ok = if roll < 0.45 {
            let uid = dice.pick(&all).clone();
            let by = dice.within(-0.4, 0.4);
            tried(&mut s, "cad.walls.edit", json!({ "uids": [uid], "operation": { "kind": "shift", "by": by } }))
        } else if roll < 0.8 {
            let x = dice.within(-4.0, 88.0);
            let y = dice.within(-4.0, 12.0);
            let kind = if step == 40 { 3 } else { (dice.next() * 3.0) as u32 };
            let wall = match kind {
                // Straight, inside or near a flat.
                0 => json!({ "pts": [{ "x": x, "y": y }, { "x": x + dice.within(-4.0, 4.0), "y": y + dice.within(-4.0, 4.0) }] }),
                // An arc.
                1 => json!({ "pts": [{ "x": x, "y": y }, { "x": x + dice.within(1.0, 5.0), "y": y }],
                    "bulges": [dice.within(-1.5, 1.5), 0.0] }),
                // Across the gap between two flats.
                2 => {
                    let k = (dice.next() * 5.0).floor();
                    json!({ "pts": [{ "x": 14.0 * k + 10.0, "y": y }, { "x": 14.0 * k + 16.0, "y": y }] })
                }
                // Round every flat, once, late.
                _ => json!({ "pts": [{ "x": -8.0, "y": -8.0 }, { "x": 92.0, "y": -8.0 }, { "x": 92.0, "y": 16.0 },
                    { "x": -8.0, "y": 16.0 }], "closed": true }),
            };
            let mut wall = wall;
            wall["arch"] = arch.clone();
            wall["width"] = json!(dice.within(0.1, 0.3));
            tried(&mut s, "cad.walls.create", json!({ "layerId": layer, "walls": [wall] }))
        } else {
            let uid = dice.pick(&all).clone();
            tried(&mut s, "cad.entities.delete", json!({ "uids": [uid] }))
        };
        if ok {
            done += 1;
            fewer += checked(&s, &mut dice, level.as_deref(), &format!("step {step}"));
        }
        if dice.next() < 0.1 {
            s.undo();
            checked(&s, &mut dice, level.as_deref(), &format!("step {step} undone"));
        }
    }
    assert!(done >= 20, "{done} edits written");
    assert!(fewer >= 60, "{fewer} slab readings among fewer walls");
}
