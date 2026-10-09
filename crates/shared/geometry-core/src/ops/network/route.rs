//! En kısa yol (docs/adr/0209 §5): the stops in their order, each pair's
//! shortest way a leg; or, Sırayı iyileştir, the order of the stops that
//! costs least (the first fixed, or the first and the last), exactly by
//! Held–Karp over the stops' costs, up to [`REORDER_LIMIT`] stops; of equal
//! totals the order first in the stops' own order (lexicographically).

use super::graph::{Graph, Location};
use super::search::{Path, Query, Searcher, Span, spans_cost};

/// The most stops whose order is bettered.
pub const REORDER_LIMIT: usize = 12;

/// Whether and how the stops' order is bettered.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reorder {
    /// As given.
    None,
    /// The first stays first.
    KeepFirst,
    /// The first stays first and the last last.
    KeepFirstLast,
}

/// A route: the stops' order visited, each leg, the spans in one row and the route's cost.
#[derive(Clone, Debug, PartialEq)]
pub struct Route {
    pub order: Vec<usize>,
    pub legs: Vec<Path>,
    pub spans: Vec<Span>,
    pub cost: f64,
}

impl Route {
    /// Its cost with cost `c` (none when a span cannot be travelled with it).
    pub fn total(&self, g: &Graph, c: usize) -> Option<f64> {
        Some(spans_cost(g, &self.spans, c)).filter(|x| x.is_finite())
    }
}

/// Why a route is not made.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RouteError {
    /// Fewer than two stops.
    TooFew,
    /// More stops than [`REORDER_LIMIT`] to reorder.
    TooMany,
    /// No way from stop `.0` to stop `.1` (indices into the stops).
    Unreachable(usize, usize),
    /// Every order has a leg without a way.
    NoOrder,
}

/// The stops' costs to each other with cost `c`: `m[i][j]` from `i` to `j`; infinite where there is no way.
pub fn stop_costs(
    g: &Graph,
    s: &mut Searcher,
    stops: &[Location],
    barriers: &[Location],
    c: usize,
) -> Vec<Vec<f64>> {
    let mut m = vec![vec![f64::INFINITY; stops.len()]; stops.len()];
    for (i, row) in m.iter_mut().enumerate() {
        let t = s.search(
            g,
            &Query {
                targets: stops,
                barriers,
                enough: Some(stops.len()),
                ..Query::from(&stops[i..i + 1], c)
            },
        );
        for (j, v) in row.iter_mut().enumerate() {
            *v = if i == j { 0.0 } else { t.place_cost(1 + j) };
        }
        s.recycle(t);
    }
    m
}

/// The order of the stops costing least by `m` (Held–Karp), `first` fixed first and `last` last when given; of equal
/// totals the lexicographically first. None when every order has a leg without a way.
pub fn best_order(m: &[Vec<f64>], first: usize, last: Option<usize>) -> Option<Vec<usize>> {
    let n = m.len();
    let free: Vec<usize> = (0..n).filter(|&i| i != first && Some(i) != last).collect();
    let k = free.len();
    let full = (1usize << k) - 1;
    // f[mask][i]: the least cost from free stop i, having visited `mask` (i in it), through the rest (and to `last`).
    let mut f = vec![vec![f64::INFINITY; k.max(1)]; 1 << k];
    for mask in (1..=full).rev() {
        for i in 0..k {
            if mask & (1 << i) == 0 {
                continue;
            }
            f[mask][i] = if mask == full {
                last.map_or(0.0, |l| m[free[i]][l])
            } else {
                let mut best = f64::INFINITY;
                for j in 0..k {
                    if mask & (1 << j) == 0 {
                        let v = m[free[i]][free[j]] + f[mask | (1 << j)][j];
                        if v < best {
                            best = v;
                        }
                    }
                }
                best
            };
        }
    }
    let start = |j: usize| m[first][free[j]] + f[1 << j][j];
    if k == 0 {
        let order = std::iter::once(first).chain(last).collect::<Vec<_>>();
        return last
            .is_none_or(|l| m[first][l].is_finite())
            .then_some(order);
    }
    let total = (0..k)
        .map(start)
        .fold(f64::INFINITY, |a, b| if b < a { b } else { a });
    if !total.is_finite() {
        return None;
    }
    // Forward: each time the lowest stop that keeps the least total.
    let mut order = vec![first];
    let mut mask = 0usize;
    let mut at: Option<usize> = None;
    let mut want = total;
    for _ in 0..k {
        let mut chosen = None;
        for j in 0..k {
            if mask & (1 << j) != 0 {
                continue;
            }
            let v = match at {
                None => start(j),
                Some(i) => m[free[i]][free[j]] + f[mask | (1 << j)][j],
            };
            if v == want && chosen.is_none_or(|c: usize| free[j] < free[c]) {
                chosen = Some(j);
            }
        }
        let j = chosen?;
        mask |= 1 << j;
        want = f[mask][j];
        at = Some(j);
        order.push(free[j]);
    }
    order.extend(last);
    Some(order)
}

/// The route through `stops` with cost `c`, not passing `barriers`, its order bettered by `reorder`.
pub fn route(
    g: &Graph,
    s: &mut Searcher,
    stops: &[Location],
    barriers: &[Location],
    c: usize,
    reorder: Reorder,
) -> Result<Route, RouteError> {
    if stops.len() < 2 {
        return Err(RouteError::TooFew);
    }
    let order: Vec<usize> = match reorder {
        Reorder::None => (0..stops.len()).collect(),
        _ if stops.len() > REORDER_LIMIT => return Err(RouteError::TooMany),
        Reorder::KeepFirst | Reorder::KeepFirstLast => {
            let m = stop_costs(g, s, stops, barriers, c);
            let last = (reorder == Reorder::KeepFirstLast).then_some(stops.len() - 1);
            best_order(&m, 0, last).ok_or(RouteError::NoOrder)?
        }
    };
    let mut legs = Vec::with_capacity(order.len() - 1);
    for w in order.windows(2) {
        let (a, b) = (w[0], w[1]);
        let t = s.search(
            g,
            &Query {
                targets: &stops[b..b + 1],
                barriers,
                enough: Some(1),
                ..Query::from(&stops[a..a + 1], c)
            },
        );
        let leg = t.path_to(g, &stops[b]);
        s.recycle(t);
        legs.push(leg.ok_or(RouteError::Unreachable(a, b))?);
    }
    let spans: Vec<Span> = legs.iter().flat_map(|l| l.spans.iter().copied()).collect();
    let cost = legs.iter().map(|l| l.cost).sum();
    Ok(Route {
        order,
        legs,
        spans,
        cost,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn held_karp_takes_the_cheapest_order_and_of_equal_ones_the_first() {
        // Four stops on a line at 0, 10, 20, 30: from 0, visiting all, the order 0 1 2 3 costs 30.
        let at = [0.0, 10.0, 20.0, 30.0];
        let m: Vec<Vec<f64>> = at
            .iter()
            .map(|a: &f64| at.iter().map(|b| (a - b).abs()).collect())
            .collect();
        assert_eq!(best_order(&m, 0, None), Some(vec![0, 1, 2, 3]));
        assert_eq!(
            best_order(&m, 0, Some(1)),
            Some(vec![0, 2, 3, 1]),
            "to 1 last: 20 + 10 + 20"
        );
        // A square's corners: two orders cost the same (around either way); the first in the stops' order wins.
        let sq = [(0.0, 0.0), (1.0, 0.0), (1.0, 1.0), (0.0, 1.0)];
        let d = |a: (f64, f64), b: (f64, f64)| (a.0 - b.0).abs() + (a.1 - b.1).abs();
        let m: Vec<Vec<f64>> = sq
            .iter()
            .map(|a| sq.iter().map(|b| d(*a, *b)).collect())
            .collect();
        assert_eq!(best_order(&m, 0, Some(0)), Some(vec![0, 1, 2, 3, 0]));
        // An unreachable stop leaves no order.
        let mut cut = vec![vec![1.0; 3]; 3];
        cut[0][2] = f64::INFINITY;
        cut[1][2] = f64::INFINITY;
        assert_eq!(best_order(&cut, 0, None), None);
    }
}
