//! An alignment checked against the user's design rules (docs/adr/0922 §1; ROAD-ALIGN-11, ROAD-ALIGN-15): Civil 3D's
//! line, curve, spiral and tangent-intersection checks, Bentley Civil Designer's curve pairs. Every limit is the
//! user's (the design standard's); KentOS gives none, a rule not given is not checked. The pairs are each two curves
//! one after another: reverse (turning the other way) or the same way, with the tangent between them (ST to TS);
//! joined when it is shorter than [`super::chain::MEET`].

use super::chain::{Alignment, MEET, PiInput, wrap};

/// The rules, each the least or most allowed; none, not checked.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct AlignmentRules {
    pub least_radius: Option<f64>,
    /// A curve's circular part.
    pub least_arc: Option<f64>,
    pub least_spiral: Option<f64>,
    /// A tangent between two curves (the ends of the alignment are not).
    pub least_tangent: Option<f64>,
    /// The tangent between two curves turning opposite ways (a joined pair, none, is 0).
    pub least_reverse: Option<f64>,
    /// The tangent between two curves turning the same way (broken-back); a joined pair is compound, not this.
    pub least_broken_back: Option<f64>,
    /// The larger radius over the smaller of two joined curves.
    pub most_radius_ratio: Option<f64>,
    /// Report a PI where the direction turns with no curve.
    pub kinks: bool,
}

/// Which rule a finding breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Radius,
    Arc,
    Spiral,
    Tangent,
    Reverse,
    BrokenBack,
    RadiusRatio,
    Kink,
}

/// A rule broken: by which PIs, over which stations, the value found and the limit.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub rule: Rule,
    pub pis: Vec<String>,
    pub from: f64,
    pub to: f64,
    pub value: f64,
    pub limit: f64,
}

/// Two curves one after another: their PIs, whether they turn opposite ways, the tangent between them and where.
#[derive(Clone, Debug, PartialEq)]
pub struct Pair {
    pub between: [String; 2],
    pub reverse: bool,
    pub tangent: f64,
    pub from: f64,
    pub to: f64,
}

/// The pairs of `al`'s curves, in station order.
pub fn pairs(al: &Alignment) -> Vec<Pair> {
    al.curves
        .windows(2)
        .map(|w| {
            let (a, b) = (&w[0], &w[1]);
            Pair {
                between: [a.pi.clone(), b.pi.clone()],
                reverse: (a.delta > 0.0) != (b.delta > 0.0),
                tangent: (b.stations[0] - a.stations[3]).max(0.0),
                from: a.stations[3],
                to: b.stations[0],
            }
        })
        .collect()
}

/// The findings of `rules` on `al` (built from `pis`), in station order then rule order.
pub fn check(al: &Alignment, pis: &[PiInput], rules: &AlignmentRules) -> Vec<Finding> {
    let mut out = Vec::new();
    let less = |v: f64, limit: Option<f64>| limit.filter(|l| v < *l);
    for c in &al.curves {
        if let Some(l) = less(c.radius, rules.least_radius) {
            out.push(Finding { rule: Rule::Radius, pis: vec![c.pi.clone()], from: c.stations[0], to: c.stations[3], value: c.radius, limit: l });
        }
        if let Some(l) = less(c.arc, rules.least_arc) {
            out.push(Finding { rule: Rule::Arc, pis: vec![c.pi.clone()], from: c.stations[1], to: c.stations[2], value: c.arc, limit: l });
        }
        for (length, from, to) in [(c.spiral_in, c.stations[0], c.stations[1]), (c.spiral_out, c.stations[2], c.stations[3])] {
            if length > 0.0
                && let Some(l) = less(length, rules.least_spiral)
            {
                out.push(Finding { rule: Rule::Spiral, pis: vec![c.pi.clone()], from, to, value: length, limit: l });
            }
        }
    }
    for p in pairs(al) {
        let joined = p.tangent < MEET;
        let pis = p.between.to_vec();
        if !joined && let Some(l) = less(p.tangent, rules.least_tangent) {
            out.push(Finding { rule: Rule::Tangent, pis: pis.clone(), from: p.from, to: p.to, value: p.tangent, limit: l });
        }
        if p.reverse && let Some(l) = less(p.tangent, rules.least_reverse) {
            out.push(Finding { rule: Rule::Reverse, pis: pis.clone(), from: p.from, to: p.to, value: p.tangent, limit: l });
        }
        if !p.reverse && !joined && let Some(l) = less(p.tangent, rules.least_broken_back) {
            out.push(Finding { rule: Rule::BrokenBack, pis: pis.clone(), from: p.from, to: p.to, value: p.tangent, limit: l });
        }
        if joined && let Some(limit) = rules.most_radius_ratio {
            let (a, b) = (al.curves.iter().find(|c| c.pi == p.between[0]), al.curves.iter().find(|c| c.pi == p.between[1]));
            if let (Some(a), Some(b)) = (a, b) {
                let ratio = a.radius.max(b.radius) / a.radius.min(b.radius);
                if ratio > limit {
                    out.push(Finding { rule: Rule::RadiusRatio, pis, from: a.stations[0], to: b.stations[3], value: ratio, limit });
                }
            }
        }
    }
    if rules.kinks {
        for i in 1..pis.len().saturating_sub(1) {
            let p = &pis[i];
            if p.radius > 0.0 {
                continue;
            }
            let (a, b) = (&pis[i - 1], &pis[i + 1]);
            let turn = wrap((b.y - p.y).atan2(b.x - p.x) - (p.y - a.y).atan2(p.x - a.x));
            if turn == 0.0 {
                continue;
            }
            let at = al
                .elements
                .iter()
                .find(|e| (e.start[0] - p.x).hypot(e.start[1] - p.y) < MEET)
                .map_or(f64::NAN, |e| e.station);
            out.push(Finding { rule: Rule::Kink, pis: vec![p.id.clone()], from: at, to: at, value: turn.abs().to_degrees(), limit: 0.0 });
        }
    }
    out.sort_by(|a, b| a.from.total_cmp(&b.from).then((a.rule as u8).cmp(&(b.rule as u8))));
    out
}
