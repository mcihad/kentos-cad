//! A curtain wall's grid (docs/adr/8070 §7): each straight stretch of its line divided into as many equal bays as leave
//! none wider than its largest spacing, its height into as many equal rows likewise; a frame on every line of the grid
//! (its ends, its foot and its top among them), a panel in each cell. The plan draws it, the section model builds its
//! frames and panels and the IFC model writes them from these places alone.

/// The places of a grid's lines along a stretch `length` long: as many equal bays as leave none longer than `most`, its
/// two ends among them; one bay for a stretch not longer than `most`. `[0, length]` when either is not above 0.
pub fn divisions(length: f64, most: f64) -> Vec<f64> {
    if !(length > 0.0 && length.is_finite() && most > 0.0 && most.is_finite()) {
        return vec![0.0, length.max(0.0)];
    }
    // A stretch that is a whole number of spacings is that many bays: the ratio's last bit does not add one.
    let n = libm::ceil(length / most - 1e-9).max(1.0) as usize;
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
