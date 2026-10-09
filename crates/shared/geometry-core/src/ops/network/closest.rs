//! En yakın tesis ve maliyet matrisi (docs/adr/0209 §7): from each origin
//! the targets it reaches, the cheapest first; of equal costs the first in
//! the targets' list. A search stops once `k` targets are settled (or the
//! cutoff is passed): a city's every incident does not search the whole city.

use super::graph::{Graph, Location};
use super::search::{Path, Query, Searcher};

/// A target an origin reaches and what it costs.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Found {
    pub target: usize,
    pub cost: f64,
}

/// The question: from `origins` to `targets` with cost `cost`, at most `k` each, within `cutoff`; `reverse` reads the
/// costs of reaching the origins from the targets (Tesisten olaya) instead.
#[derive(Clone, Copy, Debug)]
pub struct Nearest<'a> {
    pub origins: &'a [Location],
    pub targets: &'a [Location],
    pub k: Option<usize>,
    pub cutoff: Option<f64>,
    pub cost: usize,
    pub reverse: bool,
    pub barriers: &'a [Location],
}

/// Each origin's targets by cost, and when `paths` the way to each (from the origin, or from the target in a
/// reverse question).
pub fn nearest(
    g: &Graph,
    s: &mut Searcher,
    q: &Nearest,
    paths: bool,
) -> Vec<Vec<(Found, Option<Path>)>> {
    let k = q.k.unwrap_or(q.targets.len()).min(q.targets.len());
    let mut out = Vec::with_capacity(q.origins.len());
    for i in 0..q.origins.len() {
        let t = s.search(
            g,
            &Query {
                reverse: q.reverse,
                targets: q.targets,
                barriers: q.barriers,
                cutoff: q.cutoff,
                enough: Some(k),
                ..Query::from(&q.origins[i..i + 1], q.cost)
            },
        );
        let mut found: Vec<Found> = t
            .settled
            .iter()
            .map(|&j| Found {
                target: j,
                cost: t.place_cost(1 + j),
            })
            .filter(|f| f.cost.is_finite() && q.cutoff.is_none_or(|m| f.cost <= m))
            .collect();
        found.sort_by(|a, b| a.cost.total_cmp(&b.cost).then(a.target.cmp(&b.target)));
        found.truncate(k);
        let row = found
            .into_iter()
            .map(|f| {
                let path = if paths {
                    t.path_to(g, &q.targets[f.target])
                } else {
                    None
                };
                (f, path)
            })
            .collect();
        s.recycle(t);
        out.push(row);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::super::graph::EdgeIn;
    use super::super::rules::Rules;
    use super::*;
    use crate::api::json::Json;
    use crate::geom::intersect::Edge;
    use crate::vec2::Vec2;

    #[test]
    fn each_incident_finds_its_nearest_stations_in_cost_order() {
        let rules = Rules::from_json(
            &Json::parse(r#"{"connect":"ends","tolerance":0.01,"direction":{"kind":"both"}}"#)
                .expect("JSON"),
        )
        .expect("rules");
        // One street along x from 0 to 1000.
        let edges = [EdgeIn {
            id: 1.0,
            part: 0,
            path: vec![Edge::Seg {
                a: Vec2::new(0.0, 0.0),
                b: Vec2::new(1000.0, 0.0),
            }],
            direction: None,
            costs: Vec::new(),
            closed: false,
        }];
        let g = Graph::build(rules, &edges, &[]).0;
        let at = |x: f64| g.locate(Vec2::new(x, 1.0), 5.0).expect("near");
        let stations = [at(100.0), at(900.0), at(450.0)];
        let incidents = [at(500.0), at(980.0)];
        let mut s = Searcher::new();
        let q = Nearest {
            origins: &incidents,
            targets: &stations,
            k: Some(2),
            cutoff: None,
            cost: 0,
            reverse: true,
            barriers: &[],
        };
        let rows = nearest(&g, &mut s, &q, true);
        let ids: Vec<Vec<usize>> = rows
            .iter()
            .map(|r| r.iter().map(|(f, _)| f.target).collect())
            .collect();
        // From 500: 450 at 50, then 100 and 900 both at 400, the first in the list first.
        assert_eq!(ids, vec![vec![2, 0], vec![1, 2]]);
        assert!((rows[0][0].0.cost - 50.0).abs() < 1e-9);
        // Tesisten olaya: the way starts at the station.
        let way = rows[1][0].1.as_ref().expect("a way");
        assert!((g.point_on(way.spans[0].piece, way.spans[0].a).x - 900.0).abs() < 1e-9);
        // Within 60 only the first incident has a station.
        let near = nearest(
            &g,
            &mut s,
            &Nearest {
                cutoff: Some(60.0),
                ..q
            },
            false,
        );
        assert_eq!(near[0].len(), 1);
        assert!(near[1].is_empty());
    }
}
