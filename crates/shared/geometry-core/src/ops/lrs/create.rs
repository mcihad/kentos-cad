//! Rota oluştur (docs/adr/0224 §4): the routes made of pieces by their
//! route id, their measures from their length, a field each or a from and to
//! each.

use super::{ON, RoutePart, dist, steps};
use crate::jsmath::{js_max, js_min};
use crate::vec2::Vec2;
use std::collections::HashMap;

/// How a route's measures are made from its pieces.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// The pieces' length (Uzunluk).
    Length,
    /// Each piece's field value as its measured length (Tek alan).
    OneField,
    /// Each piece's from and to measures (İki alan).
    TwoFields,
}

/// The corner of the route's box its measures start nearest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Priority {
    UpperLeft,
    LowerLeft,
    UpperRight,
    LowerRight,
}

words!(Method { Length => "length", OneField => "oneField", TwoFields => "twoFields" });
words!(Priority { UpperLeft => "upperLeft", LowerLeft => "lowerLeft", UpperRight => "upperRight", LowerRight => "lowerRight" });

/// Create's rules.
#[derive(Clone, Debug, PartialEq)]
pub struct CreateRules {
    pub method: Method,
    pub priority: Option<Priority>,
    pub factor: Option<f64>,
    pub start: Option<f64>,
    pub ignore_gaps: Option<bool>,
}

crate::json_struct!(CreateRules {
    method,
    priority,
    factor,
    start,
    ignore_gaps => "ignoreGaps"
});

/// A piece of a route to be: its route id, path, and its field values.
#[derive(Clone, Debug, PartialEq)]
pub struct Piece {
    pub id: String,
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
    pub value: Option<f64>,
    pub from: Option<f64>,
    pub to: Option<f64>,
}

crate::json_struct!(Piece {
    id,
    pts,
    bulges,
    value,
    from,
    to
});

/// Why a piece was left out.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PieceFault {
    /// Its route id is empty.
    NoId,
    /// Tek alan: its value is not a number.
    NoValue,
    /// İki alan: its from or to is not a number.
    NoFromTo,
    /// Fewer than two vertices.
    NoPath,
}

words!(PieceFault { NoId => "noId", NoValue => "noValue", NoFromTo => "noFromTo", NoPath => "noPath" });

#[derive(Clone, Debug, PartialEq)]
pub struct LeftOut {
    pub piece: usize,
    pub why: PieceFault,
}

crate::json_struct!(LeftOut { piece, why });

/// A route made: its id and parts.
#[derive(Clone, Debug, PartialEq)]
pub struct Made {
    pub id: String,
    pub parts: Vec<RoutePart>,
}

crate::json_struct!(Made { id, parts });

#[derive(Clone, Debug, PartialEq)]
pub struct Created {
    pub routes: Vec<Made>,
    pub faults: Vec<LeftOut>,
}

crate::json_struct!(Created { routes, faults });

/// A piece's vertices and bulges, oriented.
#[derive(Clone)]
struct Path {
    pts: Vec<Vec2>,
    bulges: Vec<f64>,
}

impl Path {
    fn reversed(&self) -> Path {
        Path {
            pts: self.pts.iter().rev().copied().collect(),
            bulges: self.bulges.iter().rev().map(|b| -b).collect(),
        }
    }

    /// The length walked to each vertex.
    fn walked(&self) -> Vec<f64> {
        let part = RoutePart {
            pts: self.pts.clone(),
            bulges: Some(self.bulges.clone()),
            ms: Vec::new(),
        };
        let mut out = Vec::with_capacity(self.pts.len());
        out.push(0.0);
        for s in steps(&part) {
            let last = out[out.len() - 1];
            out.push(last + s.len);
        }
        out
    }
}

/// Each route id's route, in the order the ids first come, and the pieces
/// left out with why.
pub fn create(pieces: &[Piece], rules: &CreateRules) -> Created {
    let factor = rules.factor.unwrap_or(1.0);
    let start = rules.start.unwrap_or(0.0);
    let ignore = rules.ignore_gaps.unwrap_or(true);
    let mut order: Vec<&str> = Vec::new();
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut seen: HashMap<&str, usize> = HashMap::new();
    let mut faults = Vec::new();
    for (k, piece) in pieces.iter().enumerate() {
        let why = if crate::tools::point_text::js_trim(&piece.id).is_empty() {
            Some(PieceFault::NoId)
        } else if piece.pts.len() < 2 {
            Some(PieceFault::NoPath)
        } else if rules.method == Method::OneField && piece.value.is_none() {
            Some(PieceFault::NoValue)
        } else if rules.method == Method::TwoFields && (piece.from.is_none() || piece.to.is_none())
        {
            Some(PieceFault::NoFromTo)
        } else {
            None
        };
        if let Some(why) = why {
            faults.push(LeftOut { piece: k, why });
            continue;
        }
        match seen.get(piece.id.as_str()) {
            Some(&g) => groups[g].push(k),
            None => {
                seen.insert(&piece.id, order.len());
                order.push(&piece.id);
                groups.push(vec![k]);
            }
        }
    }
    let routes = order
        .iter()
        .zip(&groups)
        .map(|(id, group)| Made {
            id: (*id).to_owned(),
            parts: if rules.method == Method::TwoFields {
                two_fields(pieces, group)
            } else {
                chained(
                    pieces,
                    group,
                    rules.priority.unwrap_or(Priority::UpperLeft),
                    rules.method,
                    factor,
                    start,
                    ignore,
                )
            },
        })
        .collect();
    Created { routes, faults }
}

fn path_of(piece: &Piece) -> Path {
    Path {
        pts: piece.pts.clone(),
        bulges: (0..piece.pts.len() - 1)
            .map(|i| {
                piece
                    .bulges
                    .as_ref()
                    .and_then(|b| b.get(i))
                    .copied()
                    .unwrap_or(0.0)
            })
            .collect(),
    }
}

fn chained(
    pieces: &[Piece],
    group: &[usize],
    priority: Priority,
    method: Method,
    factor: f64,
    start: f64,
    ignore: bool,
) -> Vec<RoutePart> {
    let all = group.iter().flat_map(|&k| pieces[k].pts.iter());
    let (mut x0, mut y0, mut x1, mut y1) = (
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    );
    for p in all {
        x0 = js_min(x0, p.x);
        y0 = js_min(y0, p.y);
        x1 = js_max(x1, p.x);
        y1 = js_max(y1, p.y);
    }
    let corner = match priority {
        Priority::UpperLeft => Vec2::new(x0, y1),
        Priority::LowerLeft => Vec2::new(x0, y0),
        Priority::UpperRight => Vec2::new(x1, y1),
        Priority::LowerRight => Vec2::new(x1, y0),
    };
    let mut used = vec![false; group.len()];
    // The pieces' ends by their distance from the corner (input order, start
    // before end, on a tie): each chain starts at the first unused.
    let mut queue: Vec<(f64, usize, bool)> = Vec::with_capacity(group.len() * 2);
    // The ends by x, for the joins.
    let mut ends: Vec<(f64, usize, bool)> = Vec::with_capacity(group.len() * 2);
    for (i, &k) in group.iter().enumerate() {
        let pts = &pieces[k].pts;
        for (end, q) in [(false, pts[0]), (true, pts[pts.len() - 1])] {
            queue.push((dist(q, corner), i, end));
            ends.push((q.x, i, end));
        }
    }
    queue.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)).then(a.2.cmp(&b.2)));
    ends.sort_by(|a, b| a.0.total_cmp(&b.0));
    let end_of = |i: usize, end: bool| {
        let pts = &pieces[group[i]].pts;
        if end { pts[pts.len() - 1] } else { pts[0] }
    };
    let mut chains: Vec<Vec<(Path, usize)>> = Vec::new();
    let mut next_start = 0;
    loop {
        while next_start < queue.len() && used[queue[next_start].1] {
            next_start += 1;
        }
        let Some(&(_, i, end)) = queue.get(next_start) else {
            break;
        };
        used[i] = true;
        let path = path_of(&pieces[group[i]]);
        let mut chain = vec![(if end { path.reversed() } else { path }, group[i])];
        loop {
            let tail = {
                let last = &chain[chain.len() - 1].0.pts;
                last[last.len() - 1]
            };
            // The first unused piece in input order with an end on the tail,
            // its start before its end.
            let from = ends.partition_point(|e| e.0 < tail.x - ON);
            let mut next: Option<usize> = None;
            for &(x, j, e) in &ends[from..] {
                if x > tail.x + ON {
                    break;
                }
                if !used[j] && next.is_none_or(|n| j < n) && dist(end_of(j, e), tail) <= ON {
                    next = Some(j);
                }
            }
            let Some(j) = next else {
                break;
            };
            let rev = dist(end_of(j, false), tail) > ON;
            used[j] = true;
            let path = path_of(&pieces[group[j]]);
            chain.push((if rev { path.reversed() } else { path }, group[j]));
        }
        chains.push(chain);
    }
    let mut parts = Vec::with_capacity(chains.len());
    let mut cum = 0.0;
    let mut last_end: Option<Vec2> = None;
    for chain in chains {
        if let Some(e) = last_end
            && !ignore
        {
            cum += dist(e, chain[0].0.pts[0]);
        }
        let mut pts = Vec::new();
        let mut bulges = Vec::new();
        let mut ms = Vec::new();
        for (n, (path, k)) in chain.iter().enumerate() {
            let walked = path.walked();
            let total = walked[walked.len() - 1];
            let scale = match method {
                Method::OneField => pieces[*k].value.unwrap_or(0.0) / total,
                _ => 1.0,
            };
            for (v, w) in walked.iter().enumerate() {
                if n > 0 && v == 0 {
                    continue;
                }
                pts.push(path.pts[v]);
                ms.push(Some(start + factor * (cum + w * scale)));
            }
            bulges.extend_from_slice(&path.bulges);
            cum += total * scale;
        }
        last_end = pts.last().copied();
        parts.push(RoutePart {
            pts,
            bulges: bulges.iter().any(|&b| b != 0.0).then_some(bulges),
            ms,
        });
    }
    parts
}

fn two_fields(pieces: &[Piece], group: &[usize]) -> Vec<RoutePart> {
    let mut ordered: Vec<usize> = group.to_vec();
    ordered.sort_by(|&a, &b| {
        let (fa, fb) = (pieces[a].from.unwrap_or(0.0), pieces[b].from.unwrap_or(0.0));
        fa.total_cmp(&fb).then(a.cmp(&b))
    });
    let mut parts: Vec<RoutePart> = Vec::new();
    for k in ordered {
        let path = path_of(&pieces[k]);
        let walked = path.walked();
        let total = walked[walked.len() - 1];
        let (f, t) = (pieces[k].from.unwrap_or(0.0), pieces[k].to.unwrap_or(0.0));
        let ms: Vec<Option<f64>> = walked
            .iter()
            .map(|w| Some(f + (t - f) * w / total))
            .collect();
        let last = parts
            .last_mut()
            .filter(|q| dist(q.pts[q.pts.len() - 1], path.pts[0]) <= ON);
        if let Some(q) = last {
            q.pts.extend_from_slice(&path.pts[1..]);
            q.bulges
                .get_or_insert_with(Vec::new)
                .extend_from_slice(&path.bulges);
            q.ms.extend_from_slice(&ms[1..]);
        } else {
            parts.push(RoutePart {
                pts: path.pts,
                bulges: Some(path.bulges),
                ms,
            });
        }
    }
    for q in &mut parts {
        if q.bulges
            .as_ref()
            .is_some_and(|b| b.iter().all(|&v| v == 0.0))
        {
            q.bulges = None;
        }
    }
    parts
}
