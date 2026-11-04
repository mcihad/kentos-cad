//! A haul plan (docs/adr/0924; ROAD-HAUL-02, ROAD-HAUL-07): the earthwork moved along the road at the least haul
//! moment (m³·m), borrow and waste sites taking what they must up to their capacity. The road is a line: each stretch
//! of the mass ordinate puts its net (cut more than fill, a supply; fill more than cut, a demand) at its middle, a site
//! at its station; the earth flows between neighbouring places, a cubic metre costing the distance it goes. Every cut
//! must be placed and every fill met; a borrow site may give and a waste site take up to its capacity. The least
//! cost flow is found exactly by successive shortest paths (Bellman–Ford on the residual network): placing a supply or
//! meeting a demand earns more than any haul costs, so a plan that can place everything does; among those, the moment
//! is least. On a line the segments' flows follow from what each place gives or takes, so they are the plan's.

/// A site: its station, its capacity (m³) and whether it gives (borrow) or takes (waste).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PlanSite {
    pub station: f64,
    pub capacity: f64,
    pub borrow: bool,
}

/// A haul plan.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Plan {
    /// Whether everything is placed: the cut left over fits the waste sites, the fill missing the borrow sites.
    pub feasible: bool,
    /// The cut more than the fill, and the fill more than the cut, over all stretches (m³).
    pub cut: f64,
    pub fill: f64,
    /// Each site's use (m³), in the order given.
    pub used: Vec<f64>,
    /// The segments between neighbouring places: from, to (stations) and the flow (m³; forward positive).
    pub segments: Vec<(f64, f64, f64)>,
    /// Σ |flow| × length (m³·m).
    pub moment: f64,
    /// What moves: the cut and the borrow given (m³).
    pub moved: f64,
    /// Where infeasible: the fill no borrow site can meet, and the cut no waste site can take (m³).
    pub shortage: f64,
    pub surplus: f64,
}

struct Edge {
    to: usize,
    cap: f64,
    cost: f64,
    flow: f64,
}

struct Network {
    edges: Vec<Edge>,
    out: Vec<Vec<usize>>,
}

impl Network {
    fn new(n: usize) -> Self {
        Self { edges: Vec::new(), out: vec![Vec::new(); n] }
    }

    fn add(&mut self, a: usize, b: usize, cap: f64, cost: f64) -> usize {
        let k = self.edges.len();
        self.edges.push(Edge { to: b, cap, cost, flow: 0.0 });
        self.edges.push(Edge { to: a, cap: 0.0, cost: -cost, flow: 0.0 });
        self.out[a].push(k);
        self.out[b].push(k + 1);
        k
    }

    fn left(&self, k: usize) -> f64 {
        self.edges[k].cap - self.edges[k].flow
    }

    /// Successive shortest paths from `s` to `t` while a path costs less than 0.
    fn least_cost(&mut self, s: usize, t: usize, tiny: f64) {
        let n = self.out.len();
        loop {
            let mut dist = vec![f64::INFINITY; n];
            let mut via: Vec<Option<usize>> = vec![None; n];
            dist[s] = 0.0;
            for _ in 0..n {
                let mut changed = false;
                for u in 0..n {
                    if !dist[u].is_finite() {
                        continue;
                    }
                    for &k in &self.out[u] {
                        if self.left(k) > tiny {
                            let v = self.edges[k].to;
                            let d = dist[u] + self.edges[k].cost;
                            if d < dist[v] - 1e-12 * d.abs().max(1.0) {
                                dist[v] = d;
                                via[v] = Some(k);
                                changed = true;
                            }
                        }
                    }
                }
                if !changed {
                    break;
                }
            }
            if dist[t] >= 0.0 {
                return;
            }
            let mut push = f64::INFINITY;
            let mut v = t;
            while let Some(k) = via[v] {
                push = push.min(self.left(k));
                v = self.edges[k ^ 1].to;
            }
            if push <= tiny {
                return;
            }
            let mut v = t;
            while let Some(k) = via[v] {
                self.edges[k].flow += push;
                self.edges[k ^ 1].flow -= push;
                v = self.edges[k ^ 1].to;
            }
        }
    }
}

/// The plan for the mass ordinate `points` (station, m³; a stretch's net is its end's ordinate less its start's) and
/// the sites.
pub fn plan(points: &[[f64; 2]], sites: &[PlanSite]) -> Plan {
    let mut places: Vec<(f64, f64)> = Vec::new();
    for w in points.windows(2) {
        let net = w[1][1] - w[0][1];
        if net != 0.0 && w[1][0] > w[0][0] {
            places.push((0.5 * (w[0][0] + w[1][0]), net));
        }
    }
    let cut: f64 = places.iter().filter(|p| p.1 > 0.0).map(|p| p.1).sum();
    let fill: f64 = places.iter().filter(|p| p.1 < 0.0).map(|p| -p.1).sum();
    let gives: f64 = sites.iter().filter(|s| s.borrow).map(|s| s.capacity).sum();
    let takes: f64 = sites.iter().filter(|s| !s.borrow).map(|s| s.capacity).sum();
    let tiny = 1e-9 * (cut + fill + gives + takes).max(1.0);
    let shortage = (fill - cut - gives).max(0.0);
    let surplus = (cut - fill - takes).max(0.0);
    let mut out = Plan { cut, fill, used: vec![0.0; sites.len()], ..Plan::default() };
    if shortage > tiny || surplus > tiny {
        out.shortage = shortage;
        out.surplus = surplus;
        return out;
    }
    out.feasible = true;
    let mut stations: Vec<f64> = places.iter().map(|p| p.0).chain(sites.iter().map(|s| s.station)).collect();
    stations.sort_by(f64::total_cmp);
    stations.dedup();
    let at = |s: f64| stations.partition_point(|x| *x < s);
    let n = stations.len();
    let (s, t) = (n, n + 1);
    let mut net = Network::new(n + 2);
    let span = stations.last().copied().unwrap_or(0.0) - stations.first().copied().unwrap_or(0.0);
    let earn = -(2.0 * span + 1.0);
    let mut forward = Vec::with_capacity(n.saturating_sub(1));
    for i in 0..n.saturating_sub(1) {
        let length = stations[i + 1] - stations[i];
        let f = net.add(i, i + 1, f64::INFINITY, length);
        let b = net.add(i + 1, i, f64::INFINITY, length);
        forward.push((f, b));
    }
    for (station, v) in &places {
        if *v > 0.0 {
            net.add(s, at(*station), *v, earn);
        } else {
            net.add(at(*station), t, -v, earn);
        }
    }
    let site_edges: Vec<usize> = sites
        .iter()
        .map(|p| if p.borrow { net.add(s, at(p.station), p.capacity, 0.0) } else { net.add(at(p.station), t, p.capacity, 0.0) })
        .collect();
    net.least_cost(s, t, tiny);
    for (j, k) in site_edges.iter().enumerate() {
        out.used[j] = net.edges[*k].flow;
    }
    for (i, (f, b)) in forward.iter().enumerate() {
        let flow = net.edges[*f].flow - net.edges[*b].flow;
        let length = stations[i + 1] - stations[i];
        out.moment += flow.abs() * length;
        out.segments.push((stations[i], stations[i + 1], flow));
    }
    out.moved = cut + sites.iter().zip(&out.used).filter(|(p, _)| p.borrow).map(|(_, u)| u).sum::<f64>();
    out
}
