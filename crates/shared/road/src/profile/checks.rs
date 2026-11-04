//! A vertical alignment checked against the user's design rules (docs/adr/0922 §2; ROAD-PROFILE-13): Civil 3D's
//! profile line and curve checks. Every limit is the user's (the design standard's); KentOS gives none, a rule not
//! given is not checked. Grades are ratios; K is L/|A| with A in percent (metres per percent).

use super::vertical::{Pvi, Vertical};

/// The rules, each the least or most allowed; none, not checked.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ProfileRules {
    /// The steepest grade between two PVIs (ratio, either way).
    pub most_grade: Option<f64>,
    /// The flattest grade between two PVIs (ratio, either way): water must run off.
    pub least_grade: Option<f64>,
    pub least_crest_k: Option<f64>,
    pub least_sag_k: Option<f64>,
    /// A vertical curve's horizontal length.
    pub least_curve: Option<f64>,
    /// The straight grade between two vertical curves.
    pub least_grade_length: Option<f64>,
}

/// Which rule a finding breaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rule {
    Grade,
    LeastGrade,
    CrestK,
    SagK,
    Curve,
    GradeLength,
}

/// A rule broken: by which PVIs, over which stations, the value found and the limit.
#[derive(Clone, Debug, PartialEq)]
pub struct Finding {
    pub rule: Rule,
    pub pvis: Vec<String>,
    pub from: f64,
    pub to: f64,
    pub value: f64,
    pub limit: f64,
}

/// The findings of `rules` on `v` (built from `pvis`), in station order then rule order.
pub fn check(v: &Vertical, pvis: &[Pvi], rules: &ProfileRules) -> Vec<Finding> {
    let mut out = Vec::new();
    for w in pvis.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        let g = ((b.elevation - a.elevation) / (b.station - a.station)).abs();
        let ids = vec![a.id.clone(), b.id.clone()];
        if let Some(l) = rules.most_grade.filter(|l| g > *l) {
            out.push(Finding { rule: Rule::Grade, pvis: ids.clone(), from: a.station, to: b.station, value: g, limit: l });
        }
        if let Some(l) = rules.least_grade.filter(|l| g < *l) {
            out.push(Finding { rule: Rule::LeastGrade, pvis: ids, from: a.station, to: b.station, value: g, limit: l });
        }
    }
    for c in &v.curves {
        let (from, to) = (c.start[0], c.end[0]);
        if let Some(k) = c.k {
            let limit = if c.crest { rules.least_crest_k } else { rules.least_sag_k };
            if let Some(l) = limit.filter(|l| k < *l) {
                let rule = if c.crest { Rule::CrestK } else { Rule::SagK };
                out.push(Finding { rule, pvis: vec![c.pvi.clone()], from, to, value: k, limit: l });
            }
        }
        if let Some(l) = rules.least_curve.filter(|l| to - from < *l) {
            out.push(Finding { rule: Rule::Curve, pvis: vec![c.pvi.clone()], from, to, value: to - from, limit: l });
        }
    }
    if let Some(l) = rules.least_grade_length {
        for w in v.curves.windows(2) {
            let (a, b) = (&w[0], &w[1]);
            let length = (b.start[0] - a.end[0]).max(0.0);
            if length < l {
                out.push(Finding {
                    rule: Rule::GradeLength,
                    pvis: vec![a.pvi.clone(), b.pvi.clone()],
                    from: a.end[0],
                    to: b.start[0],
                    value: length,
                    limit: l,
                });
            }
        }
    }
    out.sort_by(|a, b| a.from.total_cmp(&b.from).then((a.rule as u8).cmp(&(b.rule as u8))));
    out
}
