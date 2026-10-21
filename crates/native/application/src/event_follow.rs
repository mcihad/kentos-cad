//! Events kept on their routes (docs/adr/0569 §3): run by `call::recorded` after a command's write, in the same step,
//! so what a write does to a route or an event and the places it makes the events take are one undo step (the
//! document's other followers: constraints, subnetwork names).
//!
//! A link of the project's (`settings.linearEvents`) is followed when the write touched objects of its event layer or
//! of one of its route layers (added, removed or changed one): every event object of the link's kind is read from its
//! own attributes (its route, its measures, its offset), placed on its route as the route is now by the core's own
//! rule (`lrs_events`, the one Olay kaymasını denetle uses) and moved there when it lies further from that place than
//! the link's tolerance. A write is never refused for it: an event that cannot be placed (its route is gone, a measure
//! is not a number or lies outside the route, a line event would need a bent path, the layer is locked) stays where
//! it is and the answer carries `events_dirty`. The `cad.linearref.define` command itself touches no object and is
//! not followed.

use kentos_commands::{CommandWarning, Entity, EventLink, EventLinkKind, EventSide, codes};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::ops::lrs::events_run;

use crate::lrs_events::{self, Rules};

/// What following one link did: the events moved and why the others stay.
struct Followed {
    moves: Vec<(Slot, Entity)>,
    /// Events that could not be placed, with the first reason said once.
    left: usize,
    first: Option<String>,
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

/// The link's events placed on its routes, the drawing as it is now; writes nothing.
fn place(doc: &Document, link: &EventLink) -> Followed {
    let line = link.kind == EventLinkKind::Line;
    let route_objects: Vec<&Entity> = link
        .routes
        .iter()
        .flat_map(|layer| doc.by_layer(layer))
        .collect();
    let routes = lrs_events::routes_by_id(&route_objects, &link.route_field);
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
    // The events of the link's kind, with the slot each lives in.
    let mut checked: Vec<(Slot, &Entity)> = Vec::new();
    let mut inputs = Vec::new();
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
        let reading = match lrs_events::read(e, &rules) {
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
    for link in links {
        if !link.layers().any(|l| touched.iter().any(|t| t == l)) {
            continue;
        }
        if doc.layers().get(&link.events).is_none() {
            continue;
        }
        let mut followed = place(doc, &link);
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
