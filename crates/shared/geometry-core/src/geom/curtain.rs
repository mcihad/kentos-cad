//! A curtain wall's grid (docs/adr/8070 §7): each straight stretch of its line divided into as many equal bays as leave
//! none wider than its largest spacing, its height into as many equal rows likewise; a frame on every line of the grid
//! (its ends, its foot and its top among them), a panel in each cell. The plan draws it, the section model builds its
//! frames and panels and the IFC model writes them from these places alone.

/// The places of a grid's lines along a stretch `length` long: as many equal bays as leave none longer than `most`, its
/// two ends among them; one bay for a stretch not longer than `most`. `[0, length]` when either is not above 0.
pub fn divisions(length: f64, most: f64) -> Vec<f64> {
    if !(length > 0.0 && length.is_finite() && most > 0.0 && most.is_finite()) {
        return vec![0.0, crate::jsmath::js_max(length, 0.0)];
    }
    // A stretch that is a whole number of spacings is that many bays: the ratio's last bit does not add one.
    let n = crate::jsmath::js_max(libm::ceil(length / most - 1e-9), 1.0) as usize;
    (0..=n).map(|k| length * k as f64 / n as f64).collect()
}

/// A frame's span along its line at the grid line `k` of `lines` (the places of [`divisions`]), `face` wide: centred on
/// an inner line, inside the stretch at its ends.
pub fn frame_span(lines: &[f64], k: usize, face: f64) -> (f64, f64) {
    let last = lines.len().saturating_sub(1);
    let at = lines[k];
    if k == 0 {
        (at, at + face)
    } else if k == last {
        (at - face, at)
    } else {
        (at - face / 2.0, at + face / 2.0)
    }
}

/// A cell's span along its line between the frames on the grid lines `k` and `k + 1`: from one frame's face to the
/// other's.
pub fn cell_span(lines: &[f64], k: usize, face: f64) -> (f64, f64) {
    (frame_span(lines, k, face).1, frame_span(lines, k + 1, face).0)
}

/// The curtain grid a wall's spec carries (its `curtain` JSON): its largest mullion spacing and its frame's face; none
/// for a wall that is not a curtain wall or a grid that cannot be read.
pub fn grid_of(curtain: Option<&crate::api::json::Json>) -> Option<(f64, f64, f64)> {
    use crate::api::json::Json;
    let c = curtain?;
    let num = |k: &str| match c.get(k) {
        Json::Num(n) if n.is_finite() => Some(*n),
        _ => None,
    };
    Some((num("spacing")?, num("rows")?, num("frame").unwrap_or(FRAME)))
}

/// A frame's face when the grid gives none, metres (the contract's `CurtainGrid::FRAME`).
pub const FRAME: f64 = 0.05;

/// A door's, a window's or a bare opening's place in a curtain wall (docs/adr/8129): along its base line from `s0` to
/// `s1`, up from `sill` to `head` over the wall's foot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Fill {
    pub s0: f64,
    pub s1: f64,
    pub sill: f64,
    pub head: f64,
}

/// The cells of a curtain wall an opening fills in place of their panels (docs/adr/8129): the wall's base line `pts`
/// (straight stretches, `closed` its last back to its first), `height` high over its foot, its `grid` (the largest
/// spacing, the largest row, the frame's face); the opening along `span` of its base line, `rise` its sill and head over
/// the foot. Along: the bays of the stretch its middle is on whose cells' middles it covers, else the bay its middle is
/// in; up likewise the rows, but one that stands on the floor (`floor`: a door, or one without a sill) from the foot
/// row up to the row its head is in (its head never lower than asked: on a grid line it stops there). Its sides and
/// its head and sill at their frames' faces; one on the floor has no frame under it, its sill the foot. None when its
/// middle is on no stretch or the cells have no room.
pub fn fill(
    pts: &[crate::vec2::Vec2],
    closed: bool,
    (spacing, rows, face): (f64, f64, f64),
    height: f64,
    (s0, s1): (f64, f64),
    (sill, head): (f64, f64),
    floor: bool,
) -> Option<Fill> {
    const EPS: f64 = 1e-9;
    let n = pts.len();
    let stretches = if closed { n } else { n.saturating_sub(1) };
    let middle = (s0 + s1) / 2.0;
    // Its cells in `lines` from `lo` to `hi`: those whose middles lie in `[lo, hi]`, else the one `at` is in; with
    // `from_first` from the first cell up to the one `hi` is in (the last that begins below it).
    let chosen = |lines: &[f64], lo: f64, hi: f64, at: f64, from_first: bool| -> Option<(usize, usize)> {
        let cells = lines.len().checked_sub(1)?;
        if from_first {
            return Some((0, (0..cells).rfind(|&k| lines[k] < hi - EPS).unwrap_or(0)));
        }
        let mid = |k: usize| {
            let (a, b) = cell_span(lines, k, face);
            (a + b) / 2.0
        };
        let inside: Vec<usize> = (0..cells).filter(|&k| mid(k) >= lo - EPS && mid(k) <= hi + EPS).collect();
        match (inside.first(), inside.last()) {
            (Some(&a), Some(&b)) => Some((a, b)),
            _ => (0..cells).find(|&k| at >= lines[k] - EPS && at <= lines[k + 1] + EPS).map(|k| (k, k)),
        }
    };
    let mut start = 0.0;
    for i in 0..stretches {
        let (a, b) = (pts[i], pts[(i + 1) % n]);
        let length = crate::jsmath::js_hypot(b.x - a.x, b.y - a.y);
        if middle >= start - EPS && middle <= start + length + EPS && length > 2.0 * face {
            let bays = divisions(length, spacing);
            let (k0, k1) = chosen(&bays, s0 - start, s1 - start, middle - start, false)?;
            let ups = divisions(height, rows);
            let (r0, r1) = chosen(&ups, sill, head, (sill + head) / 2.0, floor)?;
            let (x0, x1) = (cell_span(&bays, k0, face).0, cell_span(&bays, k1, face).1);
            let (z0, z1) = (cell_span(&ups, r0, face).0, cell_span(&ups, r1, face).1);
            if x1 - x0 <= EPS || z1 - z0 <= EPS {
                return None;
            }
            return Some(Fill { s0: start + x0, s1: start + x1, sill: if floor { 0.0 } else { z0 }, head: z1 });
        }
        start += length;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_stretch_is_divided_into_equal_bays_none_past_the_spacing() {
        // 6 m at 1.5 m: four bays exactly; 6.1 m: five of 1.22 m.
        assert_eq!(divisions(6.0, 1.5), vec![0.0, 1.5, 3.0, 4.5, 6.0]);
        let d = divisions(6.1, 1.5);
        assert_eq!(d.len(), 6);
        assert!((d[1] - 1.22).abs() < 1e-12);
        // Shorter than the spacing: one bay.
        assert_eq!(divisions(1.0, 1.5), vec![0.0, 1.0]);
        assert_eq!(divisions(0.0, 1.5), vec![0.0, 0.0]);
    }

    /// A 6 m curtain wall at 1.50 m and 1.20 m, 3 m high, frames 5 cm (docs/adr/8070 §7's test wall): four bays of
    /// 1.50 (cells 0.05–1.475, 1.525–2.975, 3.025–4.475, 4.525–5.95), three rows of 1.00 (cells 0.05–0.975,
    /// 1.025–1.975, 2.025–2.95). Worked by hand.
    #[test]
    fn an_opening_fills_the_cells_it_covers() {
        use crate::vec2::Vec2;
        let wall = [Vec2::new(0.0, 0.0), Vec2::new(6.0, 0.0)];
        let grid = (1.5, 1.2, 0.05);
        let near = |f: Option<Fill>, want: (f64, f64, f64, f64)| {
            let f = f.expect("fills");
            assert!(
                (f.s0 - want.0).abs() < 1e-9
                    && (f.s1 - want.1).abs() < 1e-9
                    && (f.sill - want.2).abs() < 1e-9
                    && (f.head - want.3).abs() < 1e-9,
                "{f:?} != {want:?}"
            );
        };
        // A 0.90 m door round 2.25, its head 2.10: the second bay; from the floor up to the row 2.10 is in (the third,
        // 2.00–3.00): its head that row's top, 2.95. Asked 2.00, on a grid line: the second row's top, 1.975.
        near(fill(&wall, false, grid, 3.0, (1.8, 2.7), (0.0, 2.1), true), (1.525, 2.975, 0.0, 2.95));
        near(fill(&wall, false, grid, 3.0, (1.8, 2.7), (0.0, 2.0), true), (1.525, 2.975, 0.0, 1.975));
        // A 2 m window round 3.00 from 0.90 to 2.10: the middles 2.25 and 3.75, two bays; the middle row.
        near(fill(&wall, false, grid, 3.0, (2.0, 4.0), (0.9, 2.1), false), (1.525, 4.475, 1.025, 1.975));
        // A small one covering no cell's middle across: the row its middle (1.30) is in.
        near(fill(&wall, false, grid, 3.0, (0.2, 0.8), (1.2, 1.4), false), (0.05, 1.475, 1.025, 1.975));
        // On an L's second stretch (3 m north from (6, 0)): two bays of 1.50 there, from 6.00 along the line.
        let l = [Vec2::new(0.0, 0.0), Vec2::new(6.0, 0.0), Vec2::new(6.0, 3.0)];
        near(fill(&l, false, grid, 3.0, (7.9, 8.6), (0.0, 2.1), true), (7.525, 8.95, 0.0, 2.95));
        // Off the wall: none.
        assert_eq!(fill(&wall, false, grid, 3.0, (7.0, 8.0), (0.0, 2.1), true), None);
    }

    #[test]
    fn frames_sit_inside_the_ends_and_on_the_inner_lines() {
        let lines = divisions(3.0, 1.5);
        assert_eq!(frame_span(&lines, 0, 0.05), (0.0, 0.05));
        assert_eq!(frame_span(&lines, 1, 0.05), (1.475, 1.525));
        assert_eq!(frame_span(&lines, 2, 0.05), (2.95, 3.0));
        assert_eq!(cell_span(&lines, 0, 0.05), (0.05, 1.475));
        assert_eq!(cell_span(&lines, 1, 0.05), (1.525, 2.95));
    }
}
