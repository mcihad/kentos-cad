//! Why two places of a network have no way between them (docs/adr/0570,
//! 0575): the way, every edge both ways and nothing closed, that the fewest
//! blockers shut (the shortest by length of those), and what on it shuts the
//! question out: each edge
//! the question's cost cannot travel the way it goes, with the reason the
//! edge says (the closed edges' expression, the direction field's closed
//! value, one way the other way, the vehicle's profile, the day, the hour,
//! no value of the cost), each barrier on it, and each node it passes that
//! shuts it (docs/adr/0577): a closed junction, a forbidden turn (a turn
//! layer's line, a U-turn the rules forbid), a device whose terminal paths do
//! not join the two edges. No such way: the places are on parts of the
//! network that do not meet. The independent reference is
//! `scripts/fixtures/network_blocked_cases.py`.

use std::cmp::Ordering;
use std::collections::BinaryHeap;

use super::access::Closing;
use super::graph::{Graph, Location};

/// Why an edge is not travelled the way a way goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Why {
    /// The closed edges' expression.
    Expression,
    /// The direction field's closed value.
    Shut,
    /// One way, the other way.
    OneWay,
    /// The vehicle's profile (docs/adr/0567 §1).
    Profile,
    /// A closure on the question's day (docs/adr/0567 §2, 0568).
    Date,
    /// A closure's hours at the question's moment (docs/adr/0568 §2).
    Hours,
    /// No value of the question's cost.
    Cost,
}

impl Why {
    pub fn name(self) -> &'static str {
        match self {
            Why::Expression => "expression",
            Why::Shut => "shut",
            Why::OneWay => "oneWay",
            Why::Profile => "profile",
            Why::Date => "date",
            Why::Hours => "hours",
            Why::Cost => "cost",
        }
    }

    pub fn words(self) -> &'static str {
        match self {
            Why::Expression => "kapalı kenarlar ifadesiyle kapalı",
            Why::Shut => "yön alanı kapalı",
            Why::OneWay => "tek yön, ters yönde",
            Why::Profile => "araç profiline kapalı",
            Why::Date => "o gün kapalı",
            Why::Hours => "o saatte kapalı",
            Why::Cost => "maliyetin değeri yok",
        }
    }
}

/// What shuts a way: an edge (its object) and why, a barrier (its index among the question's), a closed junction (its
/// object), a forbidden turn (from the edge, to the edge) or a device (its junction, from the edge, to the edge).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Blocker {
    Edge(f64, Why),
    Barrier(usize),
    Junction(f64),
    Turn(f64, f64),
    Device(f64, f64, f64),
}

/// Why two places have no way between them.
#[derive(Clone, Debug, PartialEq)]
pub enum Blocked {
    /// They are on parts of the network that do not meet.
    Apart,
    /// The shortest way with nothing closed is shut by these, in its order (an edge once).
    By(Vec<Blocker>),
    /// Every edge, barrier and node of that way is open: something the search does not see cuts it (a level, a
    /// stop on no piece).
    Other,
}

/// The most blockers said in words; the rest are counted.
const SAID: usize = 5;

impl Blocked {
    pub fn words(&self) -> String {
        match self {
            Blocked::Apart => "duraklar ağın birbirine bağlanmayan parçalarında".into(),
            Blocked::Other => {
                "en az kapatanı olan yolun kenarları, engelleri ve düğümleri açık; yolu bir seviye ya da ağın dışında kalan durak kesiyor"
                    .into()
            }
            Blocked::By(list) => {
                let said: Vec<String> = list
                    .iter()
                    .take(SAID)
                    .map(|b| match *b {
                        Blocker::Edge(id, why) => format!("kenar {} ({})", show_id(id), why.words()),
                        Blocker::Barrier(i) => format!("{}. engel", i + 1),
                        Blocker::Junction(id) => format!("düğüm {} (kapalı)", show_id(id)),
                        Blocker::Turn(from, to) => {
                            format!("kenar {} → kenar {} dönüşü (yasak)", show_id(from), show_id(to))
                        }
                        Blocker::Device(id, from, to) => format!(
                            "düğüm {} (cihaz kenar {} → kenar {} geçirmiyor)",
                            show_id(id),
                            show_id(from),
                            show_id(to)
                        ),
                    })
                    .collect();
                let more = list.len().saturating_sub(SAID);
                let tail = if more > 0 { format!(" ve {more} tane daha") } else { String::new() };
                format!("yolu açmak için en az kalkması gerekenler: {}{tail}", said.join(", "))
            }
        }
    }

    /// As the JSON answers write it: `{kind, edges?, barriers?}`.
    pub fn json(&self) -> String {
        match self {
            Blocked::Apart => r#"{"kind":"apart"}"#.into(),
            Blocked::Other => r#"{"kind":"other"}"#.into(),
            Blocked::By(list) => {
                let edges: Vec<String> = list
                    .iter()
                    .filter_map(|b| match *b {
                        Blocker::Edge(id, why) => Some(format!(
                            r#"{{"id":{},"why":"{}"}}"#,
                            show_id(id),
                            why.name()
                        )),
                        _ => None,
                    })
                    .collect();
                let barriers: Vec<String> = list
                    .iter()
                    .filter_map(|b| match *b {
                        Blocker::Barrier(i) => Some(i.to_string()),
                        _ => None,
                    })
                    .collect();
                let junctions: Vec<String> = list
                    .iter()
                    .filter_map(|b| match *b {
                        Blocker::Junction(id) => {
                            Some(format!(r#"{{"id":{},"why":"closed"}}"#, show_id(id)))
                        }
                        Blocker::Device(id, from, to) => Some(format!(
                            r#"{{"id":{},"why":"device","from":{},"to":{}}}"#,
                            show_id(id),
                            show_id(from),
                            show_id(to)
                        )),
                        _ => None,
                    })
                    .collect();
                let turns: Vec<String> = list
                    .iter()
                    .filter_map(|b| match *b {
                        Blocker::Turn(from, to) => Some(format!(
                            r#"{{"from":{},"to":{}}}"#,
                            show_id(from),
                            show_id(to)
                        )),
                        _ => None,
                    })
                    .collect();
                // The junctions and turns only when there are some (the ADR 0570 answers stay as they were).
                let mut out = format!(
                    r#"{{"kind":"by","edges":[{}],"barriers":[{}]"#,
                    edges.join(","),
                    barriers.join(",")
                );
                if !junctions.is_empty() {
                    out.push_str(&format!(r#","junctions":[{}]"#, junctions.join(",")));
                }
                if !turns.is_empty() {
                    out.push_str(&format!(r#","turns":[{}]"#, turns.join(",")));
                }
                out.push('}');
                out
            }
        }
    }
}

fn show_id(id: f64) -> String {
    crate::api::json::to_string(&id)
}

/// An arc of the search with its blockers and length so far (the fewest blockers first, then the shortest; on a tie
/// the lower arc).
#[derive(Clone, Copy, PartialEq)]
struct Item(u32, f64, u32);

impl Eq for Item {}

impl Ord for Item {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.cmp(&self.0)
            .then_with(|| o.1.total_cmp(&self.1))
            .then_with(|| o.2.cmp(&self.2))
    }
}

impl PartialOrd for Item {
    fn partial_cmp(&self, o: &Self) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}

/// Why `a` and `b` have no way with cost `c` (`barriers` the question's): see the module.
pub fn blocked(g: &Graph, a: &Location, b: &Location, c: usize, barriers: &[Location]) -> Blocked {
    // The way as legs: its spans (piece, metres along it set off at, metres arrived at) and the arcs they go along.
    let legs: Vec<(Span, u32)> = match fewest(g, a, b, c, barriers) {
        Some(s) => s,
        None => return Blocked::Apart,
    };
    let mut list: Vec<Blocker> = Vec::new();
    for (i, &((p, s0, s1), arc)) in legs.iter().enumerate() {
        // The node passed before this leg, when both legs go something.
        if i > 0 {
            let ((_, q0, q1), prev) = legs[i - 1];
            if q0 != q1
                && s0 != s1
                && let Some(b) = node_blocker(g, prev, arc, c)
                && !list.contains(&b)
            {
                list.push(b);
            }
        }
        let forward = s1 >= s0;
        let (lo, hi) = if forward { (s0, s1) } else { (s1, s0) };
        let mut cut: Vec<(f64, usize)> = barriers
            .iter()
            .enumerate()
            .filter(|(_, l)| l.piece == p && lo <= l.offset && l.offset <= hi)
            .map(|(i, l)| {
                (
                    if forward {
                        l.offset - lo
                    } else {
                        hi - l.offset
                    },
                    i,
                )
            })
            .collect();
        cut.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
        // A stop at a piece's end goes nothing of it: its closing does not shut the way.
        if s1 != s0 && g.cost(p, c, forward).is_infinite() {
            let e = g.pieces[p as usize].edge as usize;
            let meta = &g.edges[e];
            let why = if meta.closed {
                Why::Expression
            } else {
                match g.closing_of(e) {
                    Closing::Profile => Why::Profile,
                    Closing::Date => Why::Date,
                    Closing::Hours => Why::Hours,
                    Closing::Open if meta.dir == super::rules::Dir::Closed => Why::Shut,
                    Closing::Open if !meta.dir.allows(forward) => Why::OneWay,
                    Closing::Open => Why::Cost,
                }
            };
            if !list
                .iter()
                .any(|b| matches!(b, Blocker::Edge(id, _) if *id == meta.id))
            {
                list.push(Blocker::Edge(meta.id, why));
            }
        }
        for (_, i) in cut {
            if !list.contains(&Blocker::Barrier(i)) {
                list.push(Blocker::Barrier(i));
            }
        }
    }
    if list.is_empty() {
        Blocked::Other
    } else {
        Blocked::By(list)
    }
}

/// What shuts the way at the node between arcs `prev` and `next` (docs/adr/0577): a closed junction, a device whose
/// terminal paths do not join them, or a forbidden turn; none when the way passes.
fn node_blocker(g: &Graph, prev: u32, next: u32, c: usize) -> Option<Blocker> {
    let node = g.arc_head[prev as usize];
    let edge_id = |arc: u32| g.edges[g.pieces[(arc / 2) as usize].edge as usize].id;
    let junction = || {
        g.junctions
            .iter()
            .find(|j| j.node == Some(node))
            .map(|j| j.id)
    };
    if g.kinds.get(node as usize).is_some_and(|k| k.closed) {
        return junction().map(Blocker::Junction);
    }
    if g.turn(prev, next, node, c).is_some() {
        return None;
    }
    if !g.crosses(node, prev, next)
        && let Some(id) = junction()
    {
        return Some(Blocker::Device(id, edge_id(prev), edge_id(next)));
    }
    Some(Blocker::Turn(edge_id(prev), edge_id(next)))
}

/// A span of a way: the piece, metres along it set off at, metres arrived at.
type Span = (u32, f64, f64);

/// The best way's end so far: its blockers, its length, the arc it arrived by (none: along `a`'s piece), its last span
/// and the arc that span goes along.
type Best = (u32, f64, Option<u32>, Span, u32);

/// The way from `a` to `b`, every piece both ways and no node closed, that the fewest blockers shut, the shortest by
/// length of those (docs/adr/0575): its spans; none when they do not meet. The search is over arcs (a piece the way
/// it is gone), so that the pieces of one edge gone one after another count its closing once; each barrier on a piece
/// gone counts once.
fn fewest(
    g: &Graph,
    a: &Location,
    b: &Location,
    c: usize,
    barriers: &[Location],
) -> Option<Vec<(Span, u32)>> {
    let shut = |arc: u32| g.cost(arc / 2, c, arc.is_multiple_of(2)).is_infinite();
    let edge = |arc: u32| g.pieces[(arc / 2) as usize].edge;
    // The barriers on piece `p` between `lo` and `hi` metres along it.
    let on = |p: u32, lo: f64, hi: f64| -> u32 {
        barriers
            .iter()
            .filter(|l| l.piece == p && lo <= l.offset && l.offset <= hi)
            .count() as u32
    };
    // The blockers stepping on `arc` after `prev` adds: its edge's closing unless `prev` was another piece of that
    // edge, closed too, or nothing of the piece is gone (a stop at its end); and the barriers on the part of its piece
    // gone (`lo`..`hi`).
    let step = |prev: Option<u32>, arc: u32, lo: f64, hi: f64| -> u32 {
        let counted_before = |p: u32| p / 2 != arc / 2 && edge(p) == edge(arc) && shut(p);
        let closing = hi > lo && shut(arc) && !prev.is_some_and(counted_before);
        u32::from(closing) + on(arc / 2, lo, hi)
    };
    // Passing the node from `prev` onto `next` (docs/adr/0577): a closed junction, a forbidden turn or a device that
    // does not join them shut the way once.
    let pass = |prev: u32, next: u32| -> u32 {
        let node = g.arc_head[prev as usize];
        let closed = g.kinds.get(node as usize).is_some_and(|k| k.closed);
        u32::from(closed || g.turn(prev, next, node, c).is_none())
    };
    let pa = g.pieces[a.piece as usize];
    let pb = g.pieces[b.piece as usize];
    // On one piece the way along it is a candidate too.
    let mut best: Option<Best> = None;
    let mut offer = |k: u32, d: f64, last: Option<u32>, span: Span, arc: u32| {
        let better = match &best {
            None => true,
            Some((bk, bd, ..)) => k < *bk || (k == *bk && d < *bd),
        };
        if better {
            best = Some((k, d, last, span, arc));
        }
    };
    if a.piece == b.piece {
        let forward = b.offset >= a.offset;
        let arc = a.piece * 2 + u32::from(!forward);
        let (lo, hi) = if forward {
            (a.offset, b.offset)
        } else {
            (b.offset, a.offset)
        };
        offer(
            step(None, arc, lo, hi),
            hi - lo,
            None,
            (a.piece, a.offset, b.offset),
            arc,
        );
    }
    let arcs = g.pieces.len() * 2;
    let mut label = vec![(u32::MAX, f64::INFINITY); arcs];
    let mut came: Vec<Option<u32>> = vec![None; arcs];
    let mut heap = BinaryHeap::new();
    // The first arcs: from `a` along its piece to either end.
    for (arc, lo, hi, d) in [
        (a.piece * 2, a.offset, pa.len, pa.len - a.offset),
        (a.piece * 2 + 1, 0.0, a.offset, a.offset),
    ] {
        let k = step(None, arc, lo, hi);
        if (k, d) < label[arc as usize] {
            label[arc as usize] = (k, d);
            heap.push(Item(k, d, arc));
        }
    }
    while let Some(Item(k, d, arc)) = heap.pop() {
        if (k, d) > label[arc as usize] {
            continue;
        }
        let head = g.arc_head[arc as usize];
        // Whether this arc went something: a stop at a node passes it by no turn.
        let went = came[arc as usize].is_some() || d > 0.0;
        // The way ends on `b`'s piece from the node it reached.
        if head == pb.from || head == pb.to {
            let forward = head == pb.from;
            let end = b.piece * 2 + u32::from(!forward);
            let (lo, hi, len) = if forward {
                (0.0, b.offset, b.offset)
            } else {
                (b.offset, pb.len, pb.len - b.offset)
            };
            let span = (b.piece, if forward { 0.0 } else { pb.len }, b.offset);
            let at = if went && len > 0.0 { pass(arc, end) } else { 0 };
            offer(
                k + at + step(Some(arc), end, lo, hi),
                d + len,
                Some(arc),
                span,
                end,
            );
        }
        for &next in g.at_node(head) {
            let p = g.pieces[(next / 2) as usize];
            let at = if went { pass(arc, next) } else { 0 };
            let nk = k + at + step(Some(arc), next, 0.0, p.len);
            let nd = d + p.len;
            if (nk, nd) < label[next as usize] {
                label[next as usize] = (nk, nd);
                came[next as usize] = Some(arc);
                heap.push(Item(nk, nd, next));
            }
        }
    }
    let (_, _, last, end_span, end_arc) = best?;
    let Some(last) = last else {
        return Some(vec![(end_span, end_arc)]);
    };
    let mut spans = vec![(end_span, end_arc)];
    let mut arc = last;
    loop {
        let p = arc / 2;
        let piece = &g.pieces[p as usize];
        let forward = arc.is_multiple_of(2);
        match came[arc as usize] {
            Some(prev) => {
                let span = if forward {
                    (p, 0.0, piece.len)
                } else {
                    (p, piece.len, 0.0)
                };
                spans.push((span, arc));
                arc = prev;
            }
            None => {
                // The first arc: from `a` to the end of its piece.
                spans.push(((a.piece, a.offset, if forward { pa.len } else { 0.0 }), arc));
                break;
            }
        }
    }
    spans.reverse();
    Some(spans)
}
