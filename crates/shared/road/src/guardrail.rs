//! Guardrail runs along a corridor (plan/road.md Y11.1, docs/adr/0909 §1; ROAD-JUNC-12). A side of a cross needs a
//! barrier where its fill is high and steep: the fill height is the hinge's elevation above the daylight point's, the
//! slope its run per that rise (1:n). The rule is the project's own table (a standard's, with its source), never a
//! number of KentOS's: a barrier is warranted where any row holds (the fill at least `height`, and steeper than 1:`slope`
//! when the row gives one). Each run of warranted crosses is lengthened before and after (the approach end against the
//! traffic of that side: on the right, traffic runs with the stations; on the left, against them) and runs closer than
//! `gap` are joined (Roadside Design Guide's practice of not leaving short gaps between barriers).

/// A side of a cross as the rule reads it: its station, and its fill height and slope (1:n); none where the side is
/// in cut, open or not solved.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Sample {
    pub station: f64,
    pub fill: Option<(f64, f64)>,
}

/// A row of the rule: a barrier where the fill is at least `height` high and, when `slope` is given, steeper than
/// 1:`slope` (its run per rise at most `slope`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Warrant {
    pub height: f64,
    pub slope: Option<f64>,
}

/// The rule: its rows; the lengths added before and after a run on the right (the approach end first); the least gap
/// left between two runs.
#[derive(Clone, Debug, PartialEq)]
pub struct Rule {
    pub warrants: Vec<Warrant>,
    pub approach: f64,
    pub leaving: f64,
    pub gap: f64,
}

impl Rule {
    /// Whether a fill of `height` and run per rise `slope` needs a barrier.
    pub fn warrants(&self, height: f64, slope: f64) -> bool {
        self.warrants.iter().any(|w| height >= w.height && w.slope.is_none_or(|s| slope <= s))
    }
}

/// A guardrail run on a side: where it starts and ends, where the barrier is warranted within it, and the highest
/// fill and steepest slope it guards.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Run {
    pub right: bool,
    pub from: f64,
    pub to: f64,
    pub warranted: [f64; 2],
    pub height: f64,
    pub slope: f64,
}

/// A side's runs, `samples` in station order, held within `range`.
pub fn side_runs(samples: &[Sample], rule: &Rule, right: bool, range: [f64; 2]) -> Vec<Run> {
    let (before, after) = if right { (rule.approach, rule.leaving) } else { (rule.leaving, rule.approach) };
    let mut out: Vec<Run> = Vec::new();
    let mut open: Option<Run> = None;
    for s in samples {
        let hit = s.fill.filter(|&(h, n)| rule.warrants(h, n));
        match (hit, open.as_mut()) {
            (Some((h, n)), Some(r)) => {
                r.warranted[1] = s.station;
                r.height = r.height.max(h);
                r.slope = r.slope.min(n);
            }
            (Some((h, n)), None) => {
                open = Some(Run { right, from: 0.0, to: 0.0, warranted: [s.station; 2], height: h, slope: n });
            }
            (None, _) => out.extend(open.take()),
        }
    }
    out.extend(open);
    let mut joined: Vec<Run> = Vec::new();
    for mut r in out {
        r.from = (r.warranted[0] - before).max(range[0]);
        r.to = (r.warranted[1] + after).min(range[1]);
        match joined.last_mut() {
            Some(last) if r.from - last.to < rule.gap => {
                last.to = last.to.max(r.to);
                last.warranted[1] = r.warranted[1];
                last.height = last.height.max(r.height);
                last.slope = last.slope.min(r.slope);
            }
            _ => joined.push(r),
        }
    }
    joined
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rule() -> Rule {
        Rule {
            warrants: vec![Warrant { height: 3.0, slope: Some(3.0) }, Warrant { height: 6.0, slope: None }],
            approach: 20.0,
            leaving: 10.0,
            gap: 30.0,
        }
    }

    fn at(station: f64, h: f64, n: f64) -> Sample {
        Sample { station, fill: Some((h, n)) }
    }

    #[test]
    fn runs_are_lengthened_and_joined() {
        let s = [
            at(0.0, 1.0, 2.0),
            at(20.0, 3.5, 2.0),
            at(40.0, 4.0, 2.0),
            at(60.0, 4.0, 4.0),
            Sample { station: 80.0, fill: None },
            at(100.0, 7.0, 4.0),
            at(120.0, 1.0, 1.5),
            at(200.0, 3.0, 3.0),
        ];
        let right = side_runs(&s, &rule(), true, [0.0, 210.0]);
        // 20–40 lengthened to 0–50, 100 to 80–110 (joined: 30 apart is not less than 30), 200 to 180–210.
        let got: Vec<_> = right.iter().map(|r| (r.from, r.to, r.warranted)).collect();
        assert_eq!(got, [(0.0, 50.0, [20.0, 40.0]), (80.0, 110.0, [100.0, 100.0]), (180.0, 210.0, [200.0, 200.0])]);
        let left = side_runs(&s, &rule(), false, [0.0, 210.0]);
        // Before 10, after 20: 10–60, 90–120 (gap 30, kept apart), 190–210.
        let got: Vec<_> = left.iter().map(|r| (r.from, r.to)).collect();
        assert_eq!(got, [(10.0, 60.0), (90.0, 120.0), (190.0, 210.0)]);
        let joined = side_runs(&s, &Rule { gap: 31.0, ..rule() }, false, [0.0, 210.0]);
        assert_eq!(joined.len(), 2);
        assert_eq!((joined[0].from, joined[0].to, joined[0].height, joined[0].slope), (10.0, 120.0, 7.0, 2.0));
    }
}
