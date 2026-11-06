//! A line type's zero-length dash (a dot) as AutoCAD plots it: a round dot of the line's weight (ADR 1124). A PDF
//! stroke paints a zero-length dash only with round caps (ISO 32000-1 §8.5.3.2, degenerate subpaths), so a pattern
//! whose dashes keep butt or square ends is written twice: its dashes without the dots, then its dots alone with
//! round caps. Both arrays keep the source's phase: every dash and dot stays where the pattern put it.

/// A pattern's dashes and its dots as two PDF dash arrays with their phases (the PDF's own units).
#[derive(Clone, Debug, PartialEq)]
pub struct Split {
    /// The dashes of positive length, the dots taken out; none when the pattern has only dots.
    pub dashes: Option<(Vec<f64>, f64)>,
    /// The dots alone: `[0, gap, 0, gap, …]`, painted with round caps.
    pub dots: (Vec<f64>, f64),
}

/// `dash` (on, off, …; an odd list repeats as PDF repeats it) at `phase` split into its dashes and dots; none when
/// it has no dot, so the stroke is written as it always was. A dot is an ink element of length zero followed by a
/// gap (the renderers' rule, `dashCover`); negative lengths count as zero, as the PDF writer clamps them.
pub fn split(dash: &[f64], phase: f64) -> Option<Split> {
    let mut d: Vec<f64> = dash.iter().map(|v| if v.is_finite() { v.max(0.0) } else { 0.0 }).collect();
    if d.len() % 2 == 1 {
        d.extend_from_within(..);
    }
    let period: f64 = d.iter().sum();
    if d.is_empty() || !(period > 0.0) || !phase.is_finite() {
        return None;
    }
    let mut runs: Vec<(f64, f64)> = Vec::new();
    let mut dots: Vec<f64> = Vec::new();
    let mut at = 0.0;
    for (i, &l) in d.iter().enumerate() {
        if i % 2 == 0 {
            if l > 0.0 {
                runs.push((at, l));
            } else if d[i + 1] > 0.0 && dots.last() != Some(&at) {
                dots.push(at);
            }
        }
        at += l;
    }
    let first_dot = *dots.first()?;
    let cycle = |from: f64| (phase - from).rem_euclid(period);
    let dashes = runs.first().map(|&(s0, _)| {
        let mut out = Vec::with_capacity(runs.len() * 2);
        for (k, &(s, l)) in runs.iter().enumerate() {
            let next = runs.get(k + 1).map_or(period + s0, |r| r.0);
            out.push(l);
            out.push(next - s - l);
        }
        (out, cycle(s0))
    });
    let mut gaps = Vec::with_capacity(dots.len() * 2);
    for (k, &p) in dots.iter().enumerate() {
        let next = dots.get(k + 1).copied().unwrap_or(period + first_dot);
        gaps.push(0.0);
        gaps.push(next - p);
    }
    Some(Split {
        dashes,
        dots: (gaps, cycle(first_dot)),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether the dash ink of `dash` at `phase` covers `at` along the line.
    fn on(dash: &[f64], phase: f64, at: f64) -> bool {
        let period: f64 = dash.iter().sum();
        let t = (at + phase).rem_euclid(period);
        let mut acc = 0.0;
        let mut on = false;
        for (i, l) in dash.iter().enumerate() {
            on |= i % 2 == 0 && *l > 0.0 && t >= acc && t < acc + l;
            acc += l;
        }
        on
    }

    /// Where the dots of `dash` at `phase` stand over `0..len`.
    fn dots(dash: &[f64], phase: f64, len: f64) -> Vec<f64> {
        let period: f64 = dash.iter().sum();
        let mut out = Vec::new();
        let mut acc = 0.0;
        for (i, l) in dash.iter().enumerate() {
            if i % 2 == 0 && *l == 0.0 {
                let mut x = (acc - phase).rem_euclid(period);
                while x < len {
                    out.push(x);
                    x += period;
                }
            }
            acc += l;
        }
        out.sort_by(f64::total_cmp);
        out
    }

    #[test]
    fn a_pattern_without_a_dot_is_left_alone() {
        assert_eq!(split(&[3., 1.], 0.5), None);
        assert_eq!(split(&[], 0.), None);
        assert_eq!(split(&[0., 0.], 0.), None);
        // A zero-length ink element with no gap after it touches the next dash: no separate dot.
        assert_eq!(split(&[2., 1., 0., 0., 1., 1.], 0.), None);
    }

    #[test]
    fn dashes_and_dots_keep_their_places_at_every_phase() {
        // ACAD_ISO10W100-like dash-dot (ISO 128): 12 dash, 3 gap, a dot, 3 gap.
        let source = [12., 3., 0., 3.];
        for phase in [0., 1.5, 12., 13.25, 15., 17.9, -4., 18. * 7. + 0.3] {
            let s = split(&source, phase).unwrap();
            let (dashes, p) = s.dashes.clone().unwrap();
            assert_eq!(dashes, vec![12., 6.]);
            let (dots_of, q) = s.dots.clone();
            assert_eq!(dots_of, vec![0., 18.]);
            // Samples off the dashes' ends (a sample on an end reads either side by rounding).
            for k in 0..360 {
                let at = f64::from(k) * 0.1 + 0.05;
                assert_eq!(on(&dashes, p, at), on(&source, phase, at), "{phase} {at}");
            }
            let (want, got) = (dots(&source, phase, 72.), dots(&dots_of, q, 72.));
            assert_eq!(want.len(), got.len(), "{phase}");
            assert!(want.iter().zip(&got).all(|(a, b)| (a - b).abs() < 1e-9), "{phase}: {want:?} {got:?}");
        }
    }

    #[test]
    fn only_dots_two_dots_a_leading_dot_and_an_odd_list() {
        let s = split(&[0., 2.], 0.5).unwrap();
        assert_eq!(s.dashes, None);
        assert_eq!(s.dots, (vec![0., 2.], 0.5));
        // Two dots and a dash: the dots' gaps run round the period from the first dot.
        let s = split(&[0., 1., 4., 1., 0., 2.], 0.).unwrap();
        assert_eq!(s.dashes, Some((vec![4., 4.], 7.)));
        assert_eq!(s.dots, (vec![0., 6., 0., 2.], 0.));
        // An odd list is the list twice, as PDF reads it: [0 1 3] is dot, 1 off, 3 on, 0 off, 1 on, 3 off.
        let s = split(&[0., 1., 3.], 0.).unwrap();
        assert_eq!(s.dashes, Some((vec![3., 0., 1., 4.], 7.)));
        assert_eq!(s.dots, (vec![0., 8.], 0.));
    }
}
