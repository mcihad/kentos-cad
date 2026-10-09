//! A capture's parts (docs/adr/0234 §7, §8): a cell's colour and how near
//! it is to another, the seed of a line among the cells round a click, the
//! window read round it, and the flood from the seed that tells whether the
//! window must grow.

/// The cells round a click a line's seed is looked for in: Chebyshev 3 (7 × 7).
pub const SEED_REACH: i64 = 3;
/// A window's first side (cells).
pub const FIRST_WINDOW: u64 = 1024;
/// A captured line's most cells: past it the click was not on a line.
pub const MOST_LINE_CELLS: usize = 4_194_304;

/// A cell's colour: its first three value bands, or its one value (then
/// the other two 0).
pub type Colour = [f64; 3];

/// Brightness: 0.299·R + 0.587·G + 0.114·B in this order; a single band's value.
pub fn luminance(c: &Colour, rgb: bool) -> f64 {
    if rgb {
        0.299 * c[0] + 0.587 * c[1] + 0.114 * c[2]
    } else {
        c[0]
    }
}

/// Whether `c` is within `tol` of `target`: (ΔR² + ΔG² + ΔB²) ≤ tol² (tol²
/// in float64), or |Δ| ≤ tol for a single band.
pub fn near(c: &Colour, target: &Colour, tol: f64, rgb: bool) -> bool {
    if rgb {
        let (r, g, b) = (c[0] - target[0], c[1] - target[1], c[2] - target[2]);
        r * r + g * g + b * b <= tol * tol
    } else {
        (c[0] - target[0]).abs() <= tol
    }
}

/// The seed among the cells within Chebyshev [`SEED_REACH`] of `click`
/// that have a colour (`colour(i, j)`): the darkest, then the nearest to
/// the click (Chebyshev), then the first row by row.
pub fn seed(
    click: (i64, i64),
    rgb: bool,
    colour: &dyn Fn(i64, i64) -> Option<Colour>,
) -> Option<((i64, i64), Colour)> {
    let mut best: Option<(f64, i64, (i64, i64), Colour)> = None;
    for j in click.1 - SEED_REACH..=click.1 + SEED_REACH {
        for i in click.0 - SEED_REACH..=click.0 + SEED_REACH {
            let Some(c) = colour(i, j) else {
                continue;
            };
            let l = luminance(&c, rgb);
            let d = (i - click.0).abs().max((j - click.1).abs());
            let better = match &best {
                None => true,
                Some((bl, bd, _, _)) => l < *bl || (l == *bl && d < *bd),
            };
            if better {
                best = Some((l, d, (i, j), c));
            }
        }
    }
    best.map(|(_, _, at, c)| (at, c))
}

/// A window: its first column and row on the raster and its size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub i0: u32,
    pub j0: u32,
    pub w: u32,
    pub h: u32,
}

/// The window of side `side` centred on `at` (its first cell `at − side / 2`),
/// within a raster of `width` × `height`.
pub fn window(at: (i64, i64), side: u64, (width, height): (u32, u32)) -> Window {
    let place = |c: i64, n: u32| -> (u32, u32) {
        let len = side.min(u64::from(n)) as i64;
        let first = (c - len / 2).clamp(0, i64::from(n) - len);
        (first as u32, len as u32)
    };
    let (i0, w) = place(at.0, width);
    let (j0, h) = place(at.1, height);
    Window { i0, j0, w, h }
}

/// A flood's cells (window indices, in the order found), whether it reaches
/// a side of the window that is not the raster's, and whether it reaches
/// the raster's edge.
#[derive(Clone, Debug, PartialEq)]
pub struct Flood {
    pub cells: Vec<u32>,
    pub inner: bool,
    pub edge: bool,
}

/// The cells of `mask` (1: in) joined to `start` by sides (and by corners
/// when `eight`), within window `win` of a raster of `size`; the flood
/// stops past `most` cells (then `cells` holds `most + 1`).
pub fn flood(
    mask: &mut [u8],
    win: Window,
    size: (u32, u32),
    start: (u32, u32),
    eight: bool,
    most: usize,
) -> Flood {
    let (w, h) = (win.w as i64, win.h as i64);
    let mut out = Flood {
        cells: Vec::new(),
        inner: false,
        edge: false,
    };
    let k0 = start.1 as usize * win.w as usize + start.0 as usize;
    if mask[k0] != 1 {
        return out;
    }
    let steps: &[(i64, i64)] = if eight {
        &[
            (0, -1),
            (1, -1),
            (1, 0),
            (1, 1),
            (0, 1),
            (-1, 1),
            (-1, 0),
            (-1, -1),
        ]
    } else {
        &[(0, -1), (1, 0), (0, 1), (-1, 0)]
    };
    mask[k0] = 2;
    let mut stack = vec![k0 as u32];
    while let Some(k) = stack.pop() {
        out.cells.push(k);
        if out.cells.len() > most {
            return out;
        }
        let (i, j) = ((k % win.w) as i64, (k / win.w) as i64);
        let (gi, gj) = (i64::from(win.i0) + i, i64::from(win.j0) + j);
        if gi == 0 || gj == 0 || gi == i64::from(size.0) - 1 || gj == i64::from(size.1) - 1 {
            out.edge = true;
        }
        if (i == 0 && win.i0 > 0)
            || (j == 0 && win.j0 > 0)
            || (i == w - 1 && win.i0 + win.w < size.0)
            || (j == h - 1 && win.j0 + win.h < size.1)
        {
            out.inner = true;
        }
        for &(di, dj) in steps {
            let (ni, nj) = (i + di, j + dj);
            if ni < 0 || nj < 0 || ni >= w || nj >= h {
                continue;
            }
            let n = (nj * w + ni) as usize;
            if mask[n] == 1 {
                mask[n] = 2;
                stack.push(n as u32);
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_darkest_nearest_first() {
        // Two equally dark cells: the nearer to the click wins, then the first row by row.
        let colour = |i: i64, j: i64| -> Option<Colour> {
            match (i, j) {
                (12, 10) | (8, 10) | (10, 7) => Some([20.0, 20.0, 20.0]),
                (i, j) if (7..=13).contains(&i) && (7..=13).contains(&j) => {
                    Some([250.0, 250.0, 250.0])
                }
                _ => None,
            }
        };
        assert_eq!(seed((10, 10), true, &colour).map(|s| s.0), Some((8, 10)));
        assert!(near(&[10.0, 20.0, 30.0], &[13.0, 24.0, 30.0], 5.0, true));
        assert!(!near(&[10.0, 20.0, 30.0], &[13.0, 24.0, 31.0], 5.0, true));
    }

    #[test]
    fn windows_and_floods() {
        assert_eq!(
            window((100, 5), 64, (1000, 40)),
            Window {
                i0: 68,
                j0: 0,
                w: 64,
                h: 40
            }
        );
        assert_eq!(
            window((995, 20), 64, (1000, 40)),
            Window {
                i0: 936,
                j0: 0,
                w: 64,
                h: 40
            }
        );
        // A diagonal line: joined by corners, apart by sides; it reaches the window's inner right side.
        let win = Window {
            i0: 0,
            j0: 0,
            w: 4,
            h: 4,
        };
        let line = [1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1, 0, 0, 0, 0, 1];
        let f = flood(&mut line.clone(), win, (10, 10), (0, 0), true, 100);
        assert_eq!(f.cells.len(), 4);
        assert!(f.inner && f.edge);
        let f = flood(&mut line.clone(), win, (10, 10), (0, 0), false, 100);
        assert_eq!(f.cells.len(), 1);
    }
}
