//! Paper canvas strokes: consume phase before clipping, keep the final partial
//! dash, and preserve an ink run through a polyline corner. Iced's LineDash
//! walker visits pattern boundaries only and cannot express a distance phase.
use iced::{Point, Rectangle};
use kentos_geometry_core::{Vec2, labels::geom::clip_path};

const PIECE_LIMIT: usize = 100_000;

/// Screen-space runs. Dense/dotted or excessive recipes return None so the
/// caller uses the existing cached CPU painter, never a truncated vector list.
pub(super) fn runs(
    points: &[Vec2],
    bounds: Rectangle,
    dash: &[f64],
    phase: f64,
) -> Option<Vec<Vec<Point>>> {
    let dash = kentos_native_style::uniform::normalized_dashes(dash);
    let period: f64 = dash.iter().sum();
    if !period.is_finite()
        || period < 4.
        || !phase.is_finite()
        || dash.iter().any(|v| !v.is_finite() || *v <= 0.)
    {
        return None;
    }
    let mut out: Vec<Vec<Point>> = Vec::new();
    let mut travelled = 0.;
    let mut pieces = 0;
    for edge in points.windows(2) {
        let (a, b) = (edge[0], edge[1]);
        let (dx, dy) = (b.x - a.x, b.y - a.y);
        let len = dx.hypot(dy);
        if !len.is_finite() {
            return None;
        }
        if len <= 1e-12 {
            continue;
        }
        let clipped = clip_path(
            &[a, b],
            [
                f64::from(bounds.x),
                f64::from(bounds.y),
                f64::from(bounds.x + bounds.width),
                f64::from(bounds.y + bounds.height),
            ],
        );
        if let Some(piece) = clipped.first() {
            let (first, last) = (piece[0], piece[1]);
            let along = |p: Vec2| ((p.x - a.x) * dx + (p.y - a.y) * dy) / len;
            let start = along(first).clamp(0., len);
            let end = along(last).clamp(start, len);
            let point = |s: f64| {
                let p = if s == start {
                    first
                } else if s == end {
                    last
                } else {
                    Vec2::new(a.x + dx * (s / len), a.y + dy * (s / len))
                };
                Point::new(p.x as f32, p.y as f32)
            };
            // Skip hidden cycles arithmetically; never walk from a remote
            // path origin to the visible part of a tiny viewport.
            let mut cycle = start - (travelled + start + phase).rem_euclid(period);
            while cycle < end {
                let mut at = cycle;
                for (i, &length) in dash.iter().enumerate() {
                    let next = at + length;
                    if i % 2 == 0 && next > start && at < end {
                        pieces += 1;
                        if pieces > PIECE_LIMIT {
                            return None;
                        }
                        let p = point(at.max(start));
                        let q = point(next.min(end));
                        if out.last().and_then(|r| r.last()) == Some(&p) {
                            out.last_mut().unwrap().push(q);
                        } else {
                            out.push(vec![p, q]);
                        }
                    }
                    at = next;
                }
                let next = cycle + period;
                if next <= cycle {
                    return None;
                }
                cycle = next;
            }
        }
        travelled += len;
    }
    if points.first() == points.last()
        && out.len() > 1
        && out.first().and_then(|r| r.first()) == out.last().and_then(|r| r.last())
    {
        let mut joined = out.pop().unwrap();
        joined.extend(out.remove(0).into_iter().skip(1));
        out.insert(0, joined);
    }
    Some(out)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn sample(
        points: &[Point],
        bounds: Rectangle,
        dash: &[f64],
        phase: f64,
    ) -> Option<Vec<Vec<Point>>> {
        runs(
            &points
                .iter()
                .map(|p| Vec2::new(f64::from(p.x), f64::from(p.y)))
                .collect::<Vec<_>>(),
            bounds,
            dash,
            phase,
        )
    }
    fn p(x: f32, y: f32) -> Point {
        Point::new(x, y)
    }
    fn box_() -> Rectangle {
        Rectangle::new(p(-100., -100.), iced::Size::new(200., 200.))
    }
    #[test]
    fn remote_fractional_start_keeps_its_phase_before_float32_conversion() {
        let bounds = Rectangle::new(p(0., -1.), iced::Size::new(40., 2.));
        let r = runs(
            &[Vec2::new(-1_000_000_000.125, 0.), Vec2::new(40., 0.)],
            bounds,
            &[10., 10.],
            0.,
        )
        .unwrap();
        assert_eq!(
            r,
            vec![
                vec![p(0., 0.), p(9.875, 0.)],
                vec![p(19.875, 0.), p(29.875, 0.)],
                vec![p(39.875, 0.), p(40., 0.)]
            ]
        );
    }
    #[test]
    fn retains_partial_final_dash_and_distance_phase() {
        let line = [p(0., 0.), p(50., 0.)];
        let lengths = [1., 0.5, 2., 0.5, 3., 0.5].map(|v| v * 10.);
        let r = sample(&line, box_(), &lengths, 0.).unwrap();
        assert_eq!(
            r,
            vec![
                vec![p(0., 0.), p(10., 0.)],
                vec![p(15., 0.), p(35., 0.)],
                vec![p(40., 0.), p(50., 0.)]
            ]
        );
        assert_eq!(
            sample(&line, box_(), &[10., 10.], 5.).unwrap(),
            vec![
                vec![p(0., 0.), p(5., 0.)],
                vec![p(15., 0.), p(25., 0.)],
                vec![p(35., 0.), p(45., 0.)]
            ]
        );
        assert_eq!(
            sample(&line, box_(), &[10., 10.], -5.).unwrap(),
            vec![
                vec![p(5., 0.), p(15., 0.)],
                vec![p(25., 0.), p(35., 0.)],
                vec![p(45., 0.), p(50., 0.)]
            ]
        );
    }
    #[test]
    fn keeps_corner_and_closed_seam_phase_without_reset_at_clip() {
        let r = sample(&[p(0., 0.), p(5., 0.), p(5., 10.)], box_(), &[10., 10.], 0.).unwrap();
        assert_eq!(r, vec![vec![p(0., 0.), p(5., 0.), p(5., 5.)]]);
        let bounds = Rectangle::new(p(3., -1.), iced::Size::new(20., 2.));
        assert_eq!(
            sample(&[p(0., 0.), p(50., 0.)], bounds, &[10., 10.], 0.).unwrap(),
            vec![vec![p(3., 0.), p(10., 0.)], vec![p(20., 0.), p(23., 0.)]]
        );
        let square = [p(0., 0.), p(10., 0.), p(10., 10.), p(0., 10.), p(0., 0.)];
        let r = sample(&square, box_(), &[15., 5.], 0.).unwrap();
        assert_eq!(
            r,
            vec![
                vec![p(0., 0.), p(10., 0.), p(10., 5.)],
                vec![p(10., 10.), p(0., 10.), p(0., 5.)]
            ]
        );
        assert_eq!(
            sample(&square, box_(), &[15., 10.], 0.).unwrap(),
            vec![vec![
                p(5., 10.),
                p(0., 10.),
                p(0., 0.),
                p(10., 0.),
                p(10., 5.)
            ]]
        );
    }
    #[test]
    fn normalizes_odd_recipes_and_skips_remote_hidden_cycles() {
        let odd = sample(&[p(0., 0.), p(50., 0.)], box_(), &[5., 10., 15.], 0.).unwrap();
        assert_eq!(
            odd,
            vec![
                vec![p(0., 0.), p(5., 0.)],
                vec![p(15., 0.), p(30., 0.)],
                vec![p(35., 0.), p(45., 0.)]
            ]
        );
        let bounds = Rectangle::new(p(0., -1.), iced::Size::new(50., 2.));
        let r = sample(&[p(-1_000_000., 0.), p(50., 0.)], bounds, &[10., 10.], 0.).unwrap();
        assert_eq!(
            r,
            vec![
                vec![p(0., 0.), p(10., 0.)],
                vec![p(20., 0.), p(30., 0.)],
                vec![p(40., 0.), p(50., 0.)]
            ]
        );
        assert!(sample(&[p(0., 0.), p(50., 0.)], box_(), &[0., 10.], 0.).is_none());
        assert!(sample(&[p(0., 0.), p(50., 0.)], box_(), &[1., 1.], 0.).is_none());
    }
}
