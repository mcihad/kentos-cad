//! Douglas–Peucker in cell space (docs/adr/0234 §5): of a path, the points
//! farther than ε from the chord of their stretch stay, the ends always.
//! The distance is |(x₁ − x₀)(yᵢ − y₀) − (y₁ − y₀)(xᵢ − x₀)| / √((x₁ − x₀)² +
//! (y₁ − y₀)²) in float64 in this order (|pᵢ − p₀| when the ends meet); of
//! equally far points the first. A closed path is first cut at the point
//! farthest from its start. ε = 0 drops only points in line.

/// The points of `pts` that stay (the ends included).
pub fn douglas_peucker(pts: &[[f64; 2]], eps: f64) -> Vec<[f64; 2]> {
    if pts.len() < 3 {
        return pts.to_vec();
    }
    let mut keep = vec![false; pts.len()];
    keep[0] = true;
    keep[pts.len() - 1] = true;
    mark(pts, eps, &mut keep, 0, pts.len() - 1);
    pts.iter()
        .zip(&keep)
        .filter_map(|(p, &k)| k.then_some(*p))
        .collect()
}

/// A closed path (its first point repeated at its end): cut at the point
/// farthest from the start (squared distances; of equal ones the first),
/// both halves simplified.
pub fn douglas_peucker_closed(pts: &[[f64; 2]], eps: f64) -> Vec<[f64; 2]> {
    let n = pts.len();
    if n < 4 {
        return pts.to_vec();
    }
    let [x0, y0] = pts[0];
    let mut far = 1;
    let mut best = -1.0;
    for (k, &[x, y]) in pts.iter().enumerate().take(n - 1).skip(1) {
        let d = (x - x0) * (x - x0) + (y - y0) * (y - y0);
        if d > best {
            best = d;
            far = k;
        }
    }
    let mut keep = vec![false; n];
    keep[0] = true;
    keep[far] = true;
    keep[n - 1] = true;
    mark(pts, eps, &mut keep, 0, far);
    mark(pts, eps, &mut keep, far, n - 1);
    pts.iter()
        .zip(&keep)
        .filter_map(|(p, &k)| k.then_some(*p))
        .collect()
}

/// Marks the points of `pts[a..=b]` that stay.
fn mark(pts: &[[f64; 2]], eps: f64, keep: &mut [bool], a: usize, b: usize) {
    let mut stack = vec![(a, b)];
    while let Some((a, b)) = stack.pop() {
        if b <= a + 1 {
            continue;
        }
        let [x0, y0] = pts[a];
        let [x1, y1] = pts[b];
        let (dx, dy) = (x1 - x0, y1 - y0);
        let len = (dx * dx + dy * dy).sqrt();
        let mut best = -1.0;
        let mut at = a;
        for (i, &[x, y]) in pts.iter().enumerate().take(b).skip(a + 1) {
            let d = if len == 0.0 {
                ((x - x0) * (x - x0) + (y - y0) * (y - y0)).sqrt()
            } else {
                (dx * (y - y0) - dy * (x - x0)).abs() / len
            };
            if d > best {
                best = d;
                at = i;
            }
        }
        if best > eps {
            keep[at] = true;
            stack.push((a, at));
            stack.push((at, b));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_staircase_and_its_corner() {
        // A staircase of half cells within one cell of its chord: only its ends stay.
        let stair: Vec<[f64; 2]> = (0..9)
            .map(|k| [0.5 + f64::from(k), 0.5 + f64::from(k / 2)])
            .collect();
        assert_eq!(douglas_peucker(&stair, 1.0), vec![[0.5, 0.5], [8.5, 4.5]]);
        // ε = 0 keeps every turn, drops points in line.
        let line = [[0.5, 0.5], [1.5, 0.5], [2.5, 0.5], [2.5, 1.5]];
        assert_eq!(
            douglas_peucker(&line, 0.0),
            vec![[0.5, 0.5], [2.5, 0.5], [2.5, 1.5]]
        );
    }

    #[test]
    fn a_closed_square() {
        let ring = [
            [0.0, 0.0],
            [0.0, 1.0],
            [0.0, 2.0],
            [2.0, 2.0],
            [2.0, 0.0],
            [0.0, 0.0],
        ];
        // Cut at (2, 2), the farthest from the start; the corners stay, (0, 1) goes.
        assert_eq!(
            douglas_peucker_closed(&ring, 0.5),
            vec![[0.0, 0.0], [0.0, 2.0], [2.0, 2.0], [2.0, 0.0], [0.0, 0.0]]
        );
    }
}
