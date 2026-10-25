//! Events kept on their routes (docs/adr/0569 §3): run by `call::recorded` after a command's write, in the same step,
//! so what a write does to a route or an event and the places it makes the events take are one undo step (the
//! document's other followers: constraints, subnetwork names).
//!
//! A link of the project's (`settings.linearEvents`) is followed when the write touched objects of its event layer or
//! of one of its route layers (added, removed or changed one). Only what the write touched is placed again
//! (docs/adr/0641 §2): the event objects it wrote, and the events of every route whose id an object of the route
//! layers held before or after the write. Each is read from its own attributes (its route, its measures, its
//! offset), placed on its route as the route is now by the core's own rule (`lrs_events`, the one Olay kaymasını
//! denetle uses) and moved there when it lies further from that place than the link's tolerance. An event the write
//! did not reach keeps its place: its route is the same object as before, so where it was in place it still is.
//! A write is never refused for it: an event that cannot be placed (its route is gone, a measure is not a number or
//! lies outside the route, a line event would need a bent path, the layer is locked) stays where it is and the answer
//! carries `events_dirty`. The `cad.linearref.define` command itself touches no object and is not followed.

use std::collections::HashSet;

use kentos_commands::{
    CommandWarning, Entity, EventLink, EventLinkKind, EventSide, codes, js_trim,
};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::lrs::events_run;

use crate::lrs_events::{self, Rules};

/// What following one link did: the events moved and why the others stay.
pub(crate) struct Followed {
    pub moves: Vec<(Slot, Entity)>,
    /// Events that could not be placed, with the first reason said once.
    pub left: usize,
    pub first: Option<String>,
}

fn dirty(name: &str, left: usize, first: &str) -> CommandWarning {
    CommandWarning {
        code: codes::EVENTS_DIRTY.into(),
        message: format!(
            "“{name}” bağı olayları rotalarında tutuyor ama bu yazma {left} olayı yerine koyamadı: {first}. Olay kaymasını denetle nedenleri tek tek söyler."
        ),
        path: None,
    }
}

/// Which of a link's events a write reached: the route ids its route layers' objects held before or after it, and the
/// event objects it left on the event layer. `None`: every event (Olay kaymasını denetle's whole check, the tests'
/// full follow).
pub(crate) struct Reached {
    pub routes: HashSet<String>,
    pub events: HashSet<u32>,
}

impl Reached {
    /// What the open step's writes reached of `link`.
    pub(crate) fn of(link: &EventLink, objects: &[kentos_domain::Written]) -> Reached {
        let mut out = Reached {
            routes: HashSet::new(),
            events: HashSet::new(),
        };
        for (before, after) in objects {
            for e in before.iter().chain(after.iter()) {
                if link.routes.iter().any(|l| *l == e.base().layer_id) {
                    let id = js_trim(
                        e.base()
                            .attrs
                            .get(&link.route_field)
                            .map_or("", String::as_str),
                    );
                    if !id.is_empty() {
                        out.routes.insert(id.to_owned());
                    }
                }
            }
            if let Some(e) = after
                && e.base().layer_id == link.events
            {
                out.events.insert(e.base().id);
            }
        }
        out
    }
}

/// The link's events placed on its routes, the drawing as it is now (only those `reached`, or every one); writes
/// nothing.
pub(crate) fn place(doc: &Document, link: &EventLink, reached: Option<&Reached>) -> Followed {
    let line = link.kind == EventLinkKind::Line;
    let rules = Rules {
        route: &link.event_route,
        measures: if line {
            vec![
                link.from.as_deref().unwrap_or_default(),
                link.to.as_deref().unwrap_or_default(),
            ]
        } else {
            vec![link.measure.as_deref().unwrap_or_default()]
        },
        offset: link.offset.as_deref(),
        turn: if link.side == EventSide::Left {
            -1.0
        } else {
            1.0
        },
    };
    let mut out = Followed {
        moves: Vec::new(),
        left: 0,
        first: None,
    };
    let note = |out: &mut Followed, why: String| {
        out.left += 1;
        out.first.get_or_insert(why);
    };
    // The events of the link's kind the write reached, with the slot each lives in and its reading.
    let mut chosen: Vec<(Slot, &Entity, Result<lrs_events::Reading, String>)> = Vec::new();
    for (uid, e) in doc.by_layer_with_uids(&link.events) {
        let fits = match e {
            Entity::Point(_) => !line,
            Entity::Line(_) | Entity::Polyline(_) => line,
            _ => false,
        };
        if !fits {
            continue;
        }
        let Some(slot) = doc.slot_of(uid) else {
            continue;
        };
        let reading = lrs_events::read(e, &rules);
        if let Some(r) = reached
            && !r.events.contains(&e.base().id)
        {
            // An event the write did not touch, on a route it did not touch either: its place is as it was.
            let id = match &reading {
                Ok(read) => read.id.as_str(),
                Err(_) => js_trim(
                    e.base()
                        .attrs
                        .get(&link.event_route)
                        .map_or("", String::as_str),
                ),
            };
            if !r.routes.contains(id) {
                continue;
            }
        }
        chosen.push((slot, e, reading));
    }
    // The routes the chosen events name, only those built (a route's place is the first object of its id).
    let wanted: HashSet<&str> = chosen
        .iter()
        .filter_map(|(_, _, r)| r.as_ref().ok().map(|r| r.id.as_str()))
        .collect();
    let route_objects: Vec<&Entity> = link
        .routes
        .iter()
        .flat_map(|layer| doc.by_layer(layer))
        .filter(|e| {
            reached.is_none()
                || wanted.contains(js_trim(
                    e.base()
                        .attrs
                        .get(&link.route_field)
                        .map_or("", String::as_str),
                ))
        })
        .collect();
    let routes = lrs_events::routes_by_id(&route_objects, &link.route_field);
    let mut checked: Vec<(Slot, &Entity)> = Vec::new();
    let mut inputs = Vec::new();
    for (slot, e, reading) in chosen {
        let reading = match reading {
            Ok(r) => r,
            Err(why) => {
                note(&mut out, format!("#{}: {why}", e.base().id));
                continue;
            }
        };
        let Some(route) = routes.index.get(reading.id.as_str()).copied() else {
            note(
                &mut out,
                format!("#{}: “{}” rotası yok", e.base().id, reading.id),
            );
            continue;
        };
        checked.push((slot, e));
        inputs.push(lrs_events::event_in(&reading, Some(route), line));
    }
    let placed = events_run(&routes.routes, &inputs);
    for ((slot, e), out_of) in checked.into_iter().zip(&placed) {
        if let Some(fault) = &out_of.fault {
            note(
                &mut out,
                format!("#{}: {}", e.base().id, lrs_events::fault_words(fault)),
            );
            continue;
        }
        let Some(drift) = lrs_events::drift_of(e, out_of) else {
            continue;
        };
        if drift <= link.tolerance_m() {
            continue;
        }
        match lrs_events::moved(e, out_of) {
            Some(shape) => out.moves.push((slot, shape)),
            None => note(
                &mut out,
                format!("#{}: çizgi nesnesi kırıklı yola taşınamaz", e.base().id),
            ),
        }
    }
    out
}

/// Follows the open step's write: see the module. Inside no step, or with no link, it does nothing.
pub(crate) fn follow(doc: &mut Document, command: &str) -> Vec<CommandWarning> {
    let mut out = Vec::new();
    if command == kentos_commands::CAD_LINEARREF_DEFINE || !doc.in_step() {
        return out;
    }
    let links: Vec<EventLink> = doc.settings().linear_events.clone();
    if links.is_empty() {
        return out;
    }
    let touched = doc.step_layers();
    let objects = doc.step_objects();
    for link in links {
        if !link.layers().any(|l| touched.iter().any(|t| t == l)) {
            continue;
        }
        if doc.layers().get(&link.events).is_none() {
            continue;
        }
        let reached = Reached::of(&link, &objects);
        let mut followed = place(doc, &link, Some(&reached));
        if !followed.moves.is_empty() && doc.layers().is_locked(&link.events) {
            followed.left += followed.moves.len();
            followed
                .first
                .get_or_insert_with(|| "olay katmanı kilitli".to_owned());
            followed.moves.clear();
        }
        if !followed.moves.is_empty() {
            doc.update_many(followed.moves, "Olayları rotalarında tut");
        }
        if followed.left > 0 {
            let first = followed.first.unwrap_or_default();
            out.push(dirty(&link.name, followed.left, &first));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{DocumentSnapshotV1, Vec2};

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

        fn pick(&mut self, list: &[&str]) -> String {
            list[self.below(list.len())].to_owned()
        }
    }

    /// The shared cases' drawing: two routes, every event in the place its route gives it, two links.
    fn drawing() -> Document {
        let cases: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../fixtures/commands/v1/cad.linearref.follow.json"
        ))
        .expect("cases");
        Document::from_snapshot(
            DocumentSnapshotV1::from_json(&cases["setup"].to_string()).expect("reads"),
        )
        .expect("opens")
    }

    fn shifted(e: &Entity, dx: f64, dy: f64) -> Entity {
        let mut e = e.clone();
        let by = |p: &mut Vec2| {
            p.x += dx;
            p.y += dy;
        };
        match &mut e {
            Entity::Point(p) => by(&mut p.p),
            Entity::Line(l) => {
                by(&mut l.a);
                by(&mut l.b);
            }
            Entity::Polyline(l) => l.pts.iter_mut().for_each(by),
            _ => {}
        }
        e
    }

    /// One user's write: a route or an event moved, an attribute changed, an event or a route drawn, one deleted.
    fn write(doc: &mut Document, r: &mut Rng) {
        let slots: Vec<(Slot, Entity)> = ["yol", "yol-eski", "kaza", "kesim"]
            .iter()
            .flat_map(|l| {
                doc.by_layer_with_uids(l)
                    .map(|(u, e)| (u, e.clone()))
                    .collect::<Vec<_>>()
            })
            .filter_map(|(u, e)| doc.slot_of(u).map(|s| (s, e)))
            .collect();
        let routes = ["D-100", "E-1", "D-9", ""];
        match r.below(6) {
            0 if !slots.is_empty() => {
                let (s, e) = &slots[r.below(slots.len())];
                doc.update(*s, shifted(e, r.next() * 6.0 - 3.0, r.next() * 6.0 - 3.0));
            }
            1 | 2 if !slots.is_empty() => {
                let (s, e) = &slots[r.below(slots.len())];
                let mut e = e.clone();
                let (field, value) = match r.below(4) {
                    0 => ("Yol", r.pick(&routes)),
                    1 => ("Km", r.pick(&["10", "40", "70", "90", "500", "abc"])),
                    2 => ("Bas", r.pick(&["10", "20", "60"])),
                    _ => ("Sapma", r.pick(&["", "2", "-3"])),
                };
                e.base_mut().attrs.insert(field.to_owned(), value);
                doc.update(*s, e);
            }
            3 => {
                let mut e = slots
                    .iter()
                    .find(|(_, e)| matches!(e, Entity::Point(_)) && e.base().layer_id == "kaza")
                    .map(|(_, e)| e.clone())
                    .expect("an event");
                e.base_mut().attrs.insert("Yol".into(), r.pick(&routes));
                e.base_mut()
                    .attrs
                    .insert("Km".into(), r.pick(&["15", "35", "55"]));
                let _ = doc.add(shifted(&e, r.next() * 20.0, 0.0));
            }
            4 => {
                let mut e = slots
                    .iter()
                    .find(|(_, e)| matches!(e, Entity::Line(_)) && e.base().layer_id == "yol")
                    .map(|(_, e)| e.clone())
                    .unwrap_or_else(|| slots[0].1.clone());
                e.base_mut().attrs.insert("Yol".into(), r.pick(&routes));
                let _ = doc.add(shifted(&e, 0.0, r.next() * 40.0));
            }
            _ if !slots.is_empty() => {
                let (s, _) = &slots[r.below(slots.len())];
                doc.remove(&[*s]);
            }
            _ => {}
        }
    }

    /// When every event was in its place or could not be placed, the events a write reached move just as placing
    /// every event would move them (docs/adr/0641 §2): many random writes, each followed.
    #[test]
    fn placing_what_a_write_reached_moves_what_placing_all_moves() {
        let mut moved = 0;
        for seed in 1..=40u64 {
            let mut doc = drawing();
            let mut r = Rng(seed.wrapping_mul(0x9e37_79b9_7f4a_7c15));
            for step in 0..25 {
                let ran = doc.run_step(
                    |d| {
                        write(d, &mut r);
                        let objects = d.step_objects();
                        for link in d.settings().linear_events.clone() {
                            let all = place(d, &link, None);
                            let reached = Reached::of(&link, &objects);
                            let some = place(d, &link, Some(&reached));
                            assert_eq!(
                                all.moves, some.moves,
                                "tohum {seed}, adım {step}, bağ {}",
                                link.id
                            );
                            moved += some.moves.len();
                            if !some.moves.is_empty() {
                                d.update_many(some.moves, "Olayları rotalarında tut");
                            }
                        }
                        Ok::<(), ()>(())
                    },
                    |_, _, _| Ok(None),
                );
                assert!(ran.is_ok());
            }
        }
        assert!(moved > 200, "taşınan {moved}");
    }
}
