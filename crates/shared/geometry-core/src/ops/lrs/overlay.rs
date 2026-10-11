//! Olayları çakıştır (docs/adr/0224 §4): two tables of line events cut
//! where any begins or ends, their pieces either both or one covers.

use std::collections::HashMap;

use crate::jsmath::{js_max, js_min};

/// One side's events of a route: (event, least, most measure).
type Spans = Vec<(usize, f64, f64)>;
/// The events covering the sweep's place: (event, most measure).
type Active = Vec<(usize, f64)>;

/// A line event: its route and measures.
#[derive(Clone, Debug, PartialEq)]
pub struct Event {
    pub route: String,
    pub from: f64,
    pub to: f64,
}

crate::json_struct!(Event { route, from, to });

/// A piece of an overlay: its route, measures and the events of each side
/// covering it (none: a union's piece only the other side covers).
#[derive(Clone, Debug, PartialEq)]
pub struct Overlaid {
    pub route: String,
    pub from: f64,
    pub to: f64,
    pub a: Option<usize>,
    pub b: Option<usize>,
}

crate::json_struct!(Overlaid {
    route,
    from,
    to,
    a,
    b
});

/// Line events of each route cut where any begins or ends: Kesişim the
/// pieces both sides cover, Birleşim those either covers; each pair of
/// events covering a piece one row, neighbouring pieces of one pair one row;
/// in the order the routes first come, then by measure.
pub fn overlay(a: &[Event], b: &[Event], union: bool) -> Vec<Overlaid> {
    // Each route's events of each side, the routes in the order they first come.
    let mut index: HashMap<&str, usize> = HashMap::new();
    let mut sides: Vec<(&str, Spans, Spans)> = Vec::new();
    for (side, events) in [a, b].into_iter().enumerate() {
        for (i, e) in events.iter().enumerate() {
            let r = *index.entry(&e.route).or_insert_with(|| {
                sides.push((&e.route, Vec::new(), Vec::new()));
                sides.len() - 1
            });
            let span = (i, js_min(e.from, e.to), js_max(e.from, e.to));
            if side == 0 {
                sides[r].1.push(span);
            } else {
                sides[r].2.push(span);
            }
        }
    }
    let mut rows: Vec<Overlaid> = Vec::new();
    for (route, ea, eb) in &sides {
        let mut cuts: Vec<f64> = ea
            .iter()
            .chain(eb)
            .flat_map(|&(_, lo, hi)| [lo, hi])
            .collect();
        cuts.sort_by(f64::total_cmp);
        cuts.dedup();
        // Each side's events by where they begin; the covering ones of a piece
        // are those begun at or before it and not ended before its end.
        let by_start = |side: &[(usize, f64, f64)]| {
            let mut v = side.to_vec();
            v.sort_by(|x, y| x.1.total_cmp(&y.1).then(x.0.cmp(&y.0)));
            v
        };
        let (sa, sb) = (by_start(ea), by_start(eb));
        let (mut na, mut nb) = (0, 0);
        let (mut act_a, mut act_b): (Active, Active) = (Vec::new(), Vec::new());
        let start = rows.len();
        // The rows of the previous piece by their pair, which a piece of the
        // same pair goes on.
        let mut open: HashMap<(Option<usize>, Option<usize>), usize> = HashMap::new();
        let mut now: HashMap<(Option<usize>, Option<usize>), usize> = HashMap::new();
        for w in cuts.windows(2) {
            let (lo, hi) = (w[0], w[1]);
            for (sorted, next, act) in [(&sa, &mut na, &mut act_a), (&sb, &mut nb, &mut act_b)] {
                while *next < sorted.len() && sorted[*next].1 <= lo {
                    act.push((sorted[*next].0, sorted[*next].2));
                    *next += 1;
                }
                act.retain(|&(_, y)| hi <= y);
            }
            let cover = |act: &[(usize, f64)]| -> Vec<Option<usize>> {
                let mut c: Vec<Option<usize>> = act.iter().map(|&(i, _)| Some(i)).collect();
                c.sort_unstable();
                if c.is_empty() && union { vec![None] } else { c }
            };
            now.clear();
            for i in cover(&act_a) {
                for j in cover(&act_b) {
                    if i.is_none() && j.is_none() {
                        continue;
                    }
                    let row = match open.get(&(i, j)) {
                        Some(&r) => {
                            rows[r].to = hi;
                            r
                        }
                        None => {
                            rows.push(Overlaid {
                                route: (*route).to_owned(),
                                from: lo,
                                to: hi,
                                a: i,
                                b: j,
                            });
                            rows.len() - 1
                        }
                    };
                    now.insert((i, j), row);
                }
            }
            std::mem::swap(&mut open, &mut now);
        }
        let key = |o: Option<usize>| o.map_or(-1, |v| v as i64);
        rows[start..].sort_by(|x, y| {
            x.from
                .total_cmp(&y.from)
                .then(key(x.a).cmp(&key(y.a)))
                .then(key(x.b).cmp(&key(y.b)))
        });
    }
    rows
}
