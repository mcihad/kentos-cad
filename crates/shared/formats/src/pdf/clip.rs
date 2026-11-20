//! A PDF page's clipping (gis-usage-report GIS-053, docs/adr/1110; ISO 32000-1 §8.5.4): the page's crop box first, then
//! every `W` (nonzero) or `W*` (even-odd) path, each kept with the graphics state it was set in, so `Q` gives the outer
//! one back. What is painted is cut by every clip in force when it is painted: a stroked part keeps its pieces inside;
//! a filled part is cut exactly by a convex clip (Sutherland–Hodgman), and otherwise by the plane's area overlay. The
//! points are in the page's device space.

/// A clipping path: its rings and its fill rule.
#[derive(Clone, Debug, PartialEq)]
pub struct Clip {
    pub rings: Vec<Vec<[f64; 2]>>,
    pub even_odd: bool,
}

impl Clip {
    /// The page's box.
    pub fn rect(x0: f64, y0: f64, x1: f64, y1: f64) -> Clip {
        Clip { rings: vec![vec![[x0, y0], [x1, y0], [x1, y1], [x0, y1]]], even_odd: false }
    }

    /// Whether `p` is inside, by the path's rule.
    pub fn contains(&self, p: [f64; 2]) -> bool {
        let mut winding = 0i32;
        let mut crossings = 0u32;
        for ring in &self.rings {
            let n = ring.len();
            for i in 0..n {
                let (a, b) = (ring[i], ring[(i + 1) % n]);
                if (a[1] <= p[1]) != (b[1] <= p[1]) {
                    let t = (p[1] - a[1]) / (b[1] - a[1]);
                    if p[0] < a[0] + t * (b[0] - a[0]) {
                        crossings += 1;
                        winding += if b[1] > a[1] { 1 } else { -1 };
                    }
                }
            }
        }
        if self.even_odd { crossings % 2 == 1 } else { winding != 0 }
    }

    /// The single convex ring the clip is, counter-clockwise; none for any other.
    fn convex(&self) -> Option<Vec<[f64; 2]>> {
        let [ring] = self.rings.as_slice() else { return None };
        let n = ring.len();
        if n < 3 {
            return None;
        }
        let cross = |i: usize| {
            let (a, b, c) = (ring[i], ring[(i + 1) % n], ring[(i + 2) % n]);
            (b[0] - a[0]) * (c[1] - b[1]) - (b[1] - a[1]) * (c[0] - b[0])
        };
        let signs: Vec<f64> = (0..n).map(cross).filter(|c| c.abs() > 1e-12).collect();
        if signs.is_empty() {
            return None;
        }
        let positive = signs[0] > 0.0;
        if !signs.iter().all(|c| (*c > 0.0) == positive) {
            return None;
        }
        let mut r = ring.clone();
        if !positive {
            r.reverse();
        }
        Some(r)
    }

    /// Every parameter in (0, 1) where the segment from `a` to `b` crosses one of the clip's edges.
    fn cuts(&self, a: [f64; 2], b: [f64; 2], out: &mut Vec<f64>) {
        let d = [b[0] - a[0], b[1] - a[1]];
        for ring in &self.rings {
            let n = ring.len();
            for i in 0..n {
                let (p, q) = (ring[i], ring[(i + 1) % n]);
                let e = [q[0] - p[0], q[1] - p[1]];
                let den = d[0] * e[1] - d[1] * e[0];
                if den.abs() < 1e-15 {
                    continue;
                }
                let w = [p[0] - a[0], p[1] - a[1]];
                let t = (w[0] * e[1] - w[1] * e[0]) / den;
                let u = (w[0] * d[1] - w[1] * d[0]) / den;
                if t > 0.0 && t < 1.0 && (0.0..=1.0).contains(&u) {
                    out.push(t);
                }
            }
        }
    }
}

/// Whether `p` is inside every clip.
pub fn inside(clips: &[Clip], p: [f64; 2]) -> bool {
    clips.iter().all(|c| c.contains(p))
}

fn lerp(a: [f64; 2], b: [f64; 2], t: f64) -> [f64; 2] {
    [a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t]
}

/// A stroked part's pieces inside every clip: open runs, the part's own when nothing is cut.
fn stroke_runs(pts: &[[f64; 2]], closed: bool, clips: &[Clip]) -> Vec<Vec<[f64; 2]>> {
    let n = pts.len();
    let edges = if closed { n } else { n.saturating_sub(1) };
    let mut runs: Vec<Vec<[f64; 2]>> = Vec::new();
    let mut run: Vec<[f64; 2]> = Vec::new();
    for i in 0..edges {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let mut ts = vec![0.0, 1.0];
        for c in clips {
            c.cuts(a, b, &mut ts);
        }
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() < 1e-12);
        for w in ts.windows(2) {
            let (p, q) = (lerp(a, b, w[0]), lerp(a, b, w[1]));
            if inside(clips, lerp(a, b, (w[0] + w[1]) / 2.0)) {
                if run.last() != Some(&p) {
                    if run.len() > 1 {
                        runs.push(std::mem::take(&mut run));
                    }
                    run = vec![p];
                }
                run.push(q);
            } else if run.len() > 1 {
                runs.push(std::mem::take(&mut run));
            } else {
                run.clear();
            }
        }
    }
    if run.len() > 1 {
        runs.push(run);
    }
    runs
}

/// A ring cut by a convex counter-clockwise ring (Sutherland–Hodgman).
fn by_convex(subject: &[[f64; 2]], clip: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let mut out = subject.to_vec();
    let n = clip.len();
    for i in 0..n {
        let (a, b) = (clip[i], clip[(i + 1) % n]);
        let side = |p: [f64; 2]| (b[0] - a[0]) * (p[1] - a[1]) - (b[1] - a[1]) * (p[0] - a[0]);
        let input = std::mem::take(&mut out);
        let m = input.len();
        for j in 0..m {
            let (p, q) = (input[j], input[(j + 1) % m]);
            let (sp, sq) = (side(p), side(q));
            if sp >= 0.0 {
                out.push(p);
            }
            if (sp >= 0.0) != (sq >= 0.0) {
                out.push(lerp(p, q, sp / (sp - sq)));
            }
        }
        if out.is_empty() {
            break;
        }
    }
    out
}

/// A filled ring cut by a clip that is not one convex ring: the plane's overlay of the two areas.
fn by_overlay(subject: &[[f64; 2]], clip: &Clip) -> Vec<Vec<[f64; 2]>> {
    use kentos_geometry_core::geom::arrangement::{Area, Ring};
    use kentos_geometry_core::geom::region::intersect_areas;
    use kentos_geometry_core::vec2::Vec2;
    let ring = |pts: &[[f64; 2]]| Ring { pts: pts.iter().map(|p| Vec2::new(p[0], p[1])).collect(), bulges: None };
    let a = Area { outer: ring(subject), holes: Vec::new() };
    let mut out = Vec::new();
    // Each ring of the clip as its own area; an even-odd clip's inner rings as holes of the first.
    let clips: Vec<Area> = if clip.even_odd && clip.rings.len() > 1 {
        vec![Area { outer: ring(&clip.rings[0]), holes: clip.rings[1..].iter().map(|r| ring(r)).collect() }]
    } else {
        clip.rings.iter().map(|r| Area { outer: ring(r), holes: Vec::new() }).collect()
    };
    for c in clips {
        for piece in intersect_areas(&[a.clone(), c]) {
            out.push(piece.outer.pts.iter().map(|p| [p.x, p.y]).collect());
        }
    }
    out
}

/// A painted part cut by the clips: its pieces (each with whether it is closed) and whether it changed; `filled` parts
/// stay areas.
pub fn cut(pts: &[[f64; 2]], closed: bool, filled: bool, clips: &[Clip]) -> (Vec<(Vec<[f64; 2]>, bool)>, bool) {
    if clips.is_empty() || pts.is_empty() {
        return (vec![(pts.to_vec(), closed)], false);
    }
    let all_in = pts.iter().all(|p| inside(clips, *p));
    let n = pts.len();
    let edges = if closed || filled { n } else { n.saturating_sub(1) };
    let crosses = (0..edges).any(|i| {
        let mut ts = Vec::new();
        clips.iter().for_each(|c| c.cuts(pts[i], pts[(i + 1) % n], &mut ts));
        !ts.is_empty()
    });
    if all_in && !crosses {
        return (vec![(pts.to_vec(), closed)], false);
    }
    if !filled || n < 3 {
        let runs = stroke_runs(pts, closed, clips);
        return (runs.into_iter().map(|r| (r, false)).collect(), true);
    }
    // A filled ring: through each clip in turn.
    let mut pieces: Vec<Vec<[f64; 2]>> = vec![pts.to_vec()];
    for c in clips {
        let mut next = Vec::new();
        for p in pieces {
            match c.convex() {
                Some(ring) => {
                    let r = by_convex(&p, &ring);
                    if r.len() >= 3 {
                        next.push(r);
                    }
                }
                None => next.extend(by_overlay(&p, c).into_iter().filter(|r| r.len() >= 3)),
            }
        }
        pieces = next;
    }
    (pieces.into_iter().map(|r| (r, true)).collect(), true)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A line across a frame keeps its piece inside; a frame's two windows keep two pieces; an even-odd frame with a
    /// hole keeps the pieces outside the hole; a square filled over the frame is cut to the frame exactly.
    #[test]
    fn what_is_painted_keeps_inside_the_clips() {
        let frame = Clip::rect(0.0, 0.0, 10.0, 10.0);
        let (pieces, changed) = cut(&[[-5.0, 5.0], [15.0, 5.0]], false, false, std::slice::from_ref(&frame));
        assert!(changed);
        assert_eq!(pieces, vec![(vec![[0.0, 5.0], [10.0, 5.0]], false)]);
        // Two windows.
        let two = Clip { rings: vec![Clip::rect(0.0, 0.0, 4.0, 10.0).rings[0].clone(), Clip::rect(6.0, 0.0, 10.0, 10.0).rings[0].clone()], even_odd: false };
        let (pieces, _) = cut(&[[-5.0, 5.0], [15.0, 5.0]], false, false, std::slice::from_ref(&two));
        assert_eq!(pieces.len(), 2, "{pieces:?}");
        // An even-odd hole.
        let holed = Clip { rings: vec![Clip::rect(0.0, 0.0, 10.0, 10.0).rings[0].clone(), Clip::rect(4.0, 4.0, 6.0, 6.0).rings[0].clone()], even_odd: true };
        let (pieces, _) = cut(&[[-5.0, 5.0], [15.0, 5.0]], false, false, std::slice::from_ref(&holed));
        assert_eq!(pieces, vec![(vec![[0.0, 5.0], [4.0, 5.0]], false), (vec![[6.0, 5.0], [10.0, 5.0]], false)]);
        // Nested clips: the inner one narrows the outer.
        let (pieces, _) = cut(&[[-5.0, 5.0], [15.0, 5.0]], false, false, &[frame.clone(), Clip::rect(2.0, 0.0, 20.0, 10.0)]);
        assert_eq!(pieces, vec![(vec![[2.0, 5.0], [10.0, 5.0]], false)]);
        // A fill over the frame: the frame's square.
        let (pieces, _) = cut(&[[-5.0, -5.0], [15.0, -5.0], [15.0, 15.0], [-5.0, 15.0]], true, true, std::slice::from_ref(&frame));
        assert_eq!(pieces.len(), 1);
        let area: f64 = {
            let r = &pieces[0].0;
            (0..r.len()).map(|i| r[i][0] * r[(i + 1) % r.len()][1] - r[(i + 1) % r.len()][0] * r[i][1]).sum::<f64>() / 2.0
        };
        assert!((area.abs() - 100.0).abs() < 1e-9, "{area}");
        // Inside: unchanged.
        let (pieces, changed) = cut(&[[1.0, 1.0], [2.0, 2.0]], false, false, std::slice::from_ref(&frame));
        assert!(!changed && pieces.len() == 1);
        // Outside: nothing.
        let (pieces, _) = cut(&[[20.0, 20.0], [30.0, 30.0]], false, false, std::slice::from_ref(&frame));
        assert!(pieces.is_empty());
    }
}
