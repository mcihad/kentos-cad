//! Why two places of a network have no way between them (docs/adr/0570):
//! the shortest way by length that every edge would give, both ways and
//! with nothing closed, and what on it shuts the question out: each edge
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
                "en kısa yolun kenarları açık; yolu bir dönüş yasağı, kapalı düğüm ya da cihazın terminal yolları kesiyor"
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
                format!("en kısa yolu kapatanlar: {}{tail}", said.join(", "))
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
                        Blocker::Edge(id, why) => Some(format!(r#"{{"id":{},"why":"{}"}}"#, show_id(id), why.name())),
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

/// A node of the relaxed search and its length so far (the least first; on a tie the lower node).
#[derive(Clone, Copy, PartialEq)]
struct Item(f64, u32);

impl Eq for Item {}

impl Ord for Item {
    fn cmp(&self, o: &Self) -> Ordering {
        o.0.total_cmp(&self.0).then_with(|| o.1.cmp(&self.1))
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
    let spans: Vec<(u32, f64, f64)> = match relaxed(g, a, b) {
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
            .map(|(i, l)| (if forward { l.offset - lo } else { hi - l.offset }, i))
            .collect();
        cut.sort_by(|x, y| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1)));
        if g.cost(p, c, forward).is_infinite() {
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
            if !list.iter().any(|b| matches!(b, Blocker::Edge(id, _) if *id == meta.id)) {
                list.push(Blocker::Edge(meta.id, why));
            }
        }
        for (_, i) in cut {
            if !list.contains(&Blocker::Barrier(i)) {
                list.push(Blocker::Barrier(i));
            }
        }
    }
    if list.is_empty() { Blocked::Other } else { Blocked::By(list) }
}

/// The shortest way from `a` to `b` by length, every piece both ways and no node closed: its spans; none when they
/// do not meet.
fn relaxed(g: &Graph, a: &Location, b: &Location) -> Option<Vec<(u32, f64, f64)>> {
    if a.piece == b.piece {
        return Some(vec![(a.piece, a.offset, b.offset)]);
    }
    let n = g.nodes.len();
    let mut dist = vec![f64::INFINITY; n];
    // How each node was reached: the arc (`piece * 2` the way it is drawn, `+ 1` against it); none for a seed.
    let mut came: Vec<Option<u32>> = vec![None; n];
    let mut heap = BinaryHeap::new();
    let pa = &g.pieces[a.piece as usize];
    for (node, d) in [(pa.from, a.offset), (pa.to, pa.len - a.offset)] {
        if d < dist[node as usize] {
            dist[node as usize] = d;
            heap.push(Item(d, node));
        }
    }
    let pb = g.pieces[b.piece as usize];
    let (end_from, end_to) = (pb.from, pb.to);
    while let Some(Item(d, node)) = heap.pop() {
        if d > dist[node as usize] {
            continue;
        }
        for &arc in g.at_node(node) {
            let p = (arc / 2) as usize;
            let head = g.arc_head[arc as usize];
            let nd = d + g.pieces[p].len;
            if nd < dist[head as usize] {
                dist[head as usize] = nd;
                came[head as usize] = Some(arc);
                heap.push(Item(nd, head));
            }
        }
    }
    let via_from = dist[end_from as usize] + b.offset;
    let via_to = dist[end_to as usize] + (pb.len - b.offset);
    if !via_from.is_finite() && !via_to.is_finite() {
        return None;
    }
    let (last, end_span) = if via_from <= via_to {
        (end_from, (b.piece, 0.0, b.offset))
    } else {
        (end_to, (b.piece, pb.len, b.offset))
    };
    let mut spans = vec![end_span];
    let mut node = last;
    while let Some(arc) = came[node as usize] {
        let p = arc / 2;
        let piece = &g.pieces[p as usize];
        let forward = arc % 2 == 0;
        spans.push(if forward { (p, 0.0, piece.len) } else { (p, piece.len, 0.0) });
        node = if forward { piece.from } else { piece.to };
    }
    // The first span: from `a` to the node the way left the piece by.
    spans.push(if node == pa.from { (a.piece, a.offset, 0.0) } else { (a.piece, a.offset, pa.len) });
    spans.reverse();
    Some(spans)
}
