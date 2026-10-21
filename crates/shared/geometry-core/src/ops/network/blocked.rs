//! Why two places of a network have no way between them (docs/adr/0570,
//! 0575): the way, every edge both ways and nothing closed, that the fewest
//! blockers shut (the shortest by length of those), and what on it shuts the
//! question out: each edge
//! the question's cost cannot travel the way it goes, with the reason the
//! edge says (the closed edges' expression, the direction field's closed
//! value, one way the other way, the vehicle's profile, the day, the hour,
//! no value of the cost), and each barrier on it. No such way: the places are
//! on parts of the network that do not meet. A way whose every edge is open:
//! a turn, a closed junction or a device's terminal paths cut it. The
//! independent reference is `scripts/fixtures/network_blocked_cases.py`.

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

/// What shuts a way: an edge (its object) and why, or a barrier (its index among the question's).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Blocker {
    Edge(f64, Why),
    Barrier(usize),
}

/// Why two places have no way between them.
#[derive(Clone, Debug, PartialEq)]
pub enum Blocked {
    /// They are on parts of the network that do not meet.
    Apart,
    /// The shortest way with nothing closed is shut by these, in its order (an edge once).
    By(Vec<Blocker>),
    /// Every edge and barrier of that way is open: a turn, a closed junction or a device cuts it.
    Other,
}

/// The most blockers said in words; the rest are counted.
const SAID: usize = 5;

impl Blocked {
    pub fn words(&self) -> String {
        match self {
            Blocked::Apart => "duraklar ağın birbirine bağlanmayan parçalarında".into(),
            Blocked::Other => {
                "en az kapatanı olan yolun kenarları açık; yolu bir dönüş yasağı, kapalı düğüm ya da cihazın terminal yolları kesiyor"
                    .into()
            }
            Blocked::By(list) => {
                let said: Vec<String> = list
                    .iter()
                    .take(SAID)
                    .map(|b| match *b {
                        Blocker::Edge(id, why) => format!("kenar {} ({})", show_id(id), why.words()),
                        Blocker::Barrier(i) => format!("{}. engel", i + 1),
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
                        Blocker::Barrier(_) => None,
                    })
                    .collect();
                let barriers: Vec<String> = list
                    .iter()
                    .filter_map(|b| match *b {
                        Blocker::Barrier(i) => Some(i.to_string()),
                        Blocker::Edge(..) => None,
                    })
                    .collect();
                format!(
                    r#"{{"kind":"by","edges":[{}],"barriers":[{}]}}"#,
                    edges.join(","),
                    barriers.join(",")
                )
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
    // The way as spans: (piece, metres along it set off at, metres arrived at).
    let spans: Vec<(u32, f64, f64)> = match fewest(g, a, b, c, barriers) {
        Some(s) => s,
        None => return Blocked::Apart,
    };
    let mut list: Vec<Blocker> = Vec::new();
    for &(p, s0, s1) in &spans {
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

/// A span of a way: the piece, metres along it set off at, metres arrived at.
type Span = (u32, f64, f64);

/// The best way's end so far: its blockers, its length, the arc it arrived by (none: along `a`'s piece) and its last span.
type Best = (u32, f64, Option<u32>, Span);

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
) -> Option<Vec<Span>> {
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
    let pa = g.pieces[a.piece as usize];
    let pb = g.pieces[b.piece as usize];
    // On one piece the way along it is a candidate too.
    let mut best: Option<Best> = None;
    let mut offer = |k: u32, d: f64, last: Option<u32>, span: Span| {
        let better = match &best {
            None => true,
            Some((bk, bd, ..)) => k < *bk || (k == *bk && d < *bd),
        };
        if better {
            best = Some((k, d, last, span));
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
            offer(k + step(Some(arc), end, lo, hi), d + len, Some(arc), span);
        }
        for &next in g.at_node(head) {
            let p = g.pieces[(next / 2) as usize];
            let nk = k + step(Some(arc), next, 0.0, p.len);
            let nd = d + p.len;
            if (nk, nd) < label[next as usize] {
                label[next as usize] = (nk, nd);
                came[next as usize] = Some(arc);
                heap.push(Item(nk, nd, next));
            }
        }
    }
    let (_, _, last, end_span) = best?;
    let Some(last) = last else {
        return Some(vec![end_span]);
    };
    let mut spans = vec![end_span];
    let mut arc = last;
    loop {
        let p = arc / 2;
        let piece = &g.pieces[p as usize];
        let forward = arc.is_multiple_of(2);
        match came[arc as usize] {
            Some(prev) => {
                spans.push(if forward {
                    (p, 0.0, piece.len)
                } else {
                    (p, piece.len, 0.0)
                });
                arc = prev;
            }
            None => {
                // The first arc: from `a` to the end of its piece.
                spans.push((a.piece, a.offset, if forward { pa.len } else { 0.0 }));
                break;
            }
        }
    }
    spans.reverse();
    Some(spans)
}
