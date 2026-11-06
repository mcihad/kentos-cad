//! A culvert across a road (plan/road.md Y11.2, docs/adr/0909 §2; ROAD-DRAIN-01, 02, 11): a pipe or a box barrel at a
//! station, turned from the cross by its skew, its ends at the fill's toes (plus what is added beyond), its flow line
//! falling toward the side the water runs to. Distances along the barrel are signed: the cross's offset (right
//! positive) over the skew's cosine. The sizes are the project's; nothing here is a standard's number.

/// A barrel's cross: pipes side by side (`spacing` the clear gap between them) or a box of cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Shape {
    Pipe { diameter: f64, thickness: f64, count: u32, spacing: f64 },
    Box { width: f64, height: f64, thickness: f64, slab: f64, cells: u32 },
}

impl Shape {
    /// The barrel's top above its flow line: the inner height and the wall or slab over it.
    pub fn top(&self) -> f64 {
        match *self {
            Shape::Pipe { diameter, thickness, .. } => diameter + thickness,
            Shape::Box { height, slab, .. } => height + slab,
        }
    }

    /// Its outer width across the barrel.
    pub fn width(&self) -> f64 {
        match *self {
            Shape::Pipe { diameter, thickness, count, spacing } => {
                f64::from(count) * (diameter + 2.0 * thickness) + f64::from(count.saturating_sub(1)) * spacing
            }
            Shape::Box { width, thickness, cells, .. } => f64::from(cells) * width + f64::from(cells + 1) * thickness,
        }
    }

    /// Its concrete per metre of barrel: the pipes' rings, or the box's outer section less its cells.
    pub fn concrete(&self) -> f64 {
        match *self {
            Shape::Pipe { diameter, thickness, count, .. } => {
                let (r, o) = (0.5 * diameter, 0.5 * diameter + thickness);
                f64::from(count) * std::f64::consts::PI * (o * o - r * r)
            }
            Shape::Box { width, height, slab, cells, .. } => {
                self.width() * (height + 2.0 * slab) - f64::from(cells) * width * height
            }
        }
    }

    /// Pipes run side by side: their total length per metre of barrel (a box has none).
    pub fn pipes(&self) -> u32 {
        match *self {
            Shape::Pipe { count, .. } => count,
            Shape::Box { .. } => 0,
        }
    }
}

/// A barrel along its line: its ends' distances (left, right), its length, and its flow line's elevation at each end.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Barrel {
    pub ends: [f64; 2],
    pub length: f64,
    pub inverts: [f64; 2],
}

/// The barrel between the cross's toes (`toes`: the left and right offsets), turned `skew` degrees, `extend` beyond
/// each, its flow line `invert` under the axis falling `slope` (a grade) toward the right (`flow_right`) or the left.
pub fn barrel(toes: [f64; 2], skew: f64, extend: f64, invert: f64, slope: f64, flow_right: bool) -> Barrel {
    let c = skew.to_radians().cos();
    let ends = [toes[0] / c - extend, toes[1] / c + extend];
    let fall = slope * if flow_right { 1.0 } else { -1.0 };
    Barrel { ends, length: ends[1] - ends[0], inverts: ends.map(|d| invert - fall * d) }
}

/// The least cover over the barrel under the road's top (`top`: the cross's points (offset, elevation) between its
/// hinges, in offset order, linear between) and its offset: the top less the barrel's outer top there. The difference
/// is linear between the top's points, so its least is at one of them.
pub fn cover(top: &[[f64; 2]], skew: f64, invert: f64, slope: f64, flow_right: bool, shape: &Shape) -> Option<(f64, f64)> {
    let c = skew.to_radians().cos();
    let fall = slope * if flow_right { 1.0 } else { -1.0 };
    top.iter()
        .map(|p| (p[1] - (invert - fall * p[0] / c + shape.top()), p[0]))
        .min_by(|a, b| a.0.total_cmp(&b.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_skewed_barrel_and_its_cover() {
        let b = barrel([-12.0, 15.0], 60.0, 1.0, 100.0, 0.02, true);
        assert!((b.ends[0] + 25.0).abs() < 1e-9 && (b.ends[1] - 31.0).abs() < 1e-9, "{:?}", b.ends);
        assert!((b.length - 56.0).abs() < 1e-9);
        assert!((b.inverts[0] - 100.5).abs() < 1e-9 && (b.inverts[1] - 99.38).abs() < 1e-9);
        let pipe = Shape::Pipe { diameter: 1.0, thickness: 0.1, count: 2, spacing: 0.5 };
        assert!((pipe.width() - 2.9).abs() < 1e-12);
        let top = [[-5.0, 104.0], [0.0, 104.1], [5.0, 104.0]];
        // At +5: 104 − (100 − 0.02·10 + 1.1) = 3.1; at −5: 104 − (100.2 + 1.1) = 2.7.
        let (least, at) = cover(&top, 60.0, 100.0, 0.02, true, &pipe).expect("a cover");
        assert!((least - 2.7).abs() < 1e-9 && at == -5.0, "{least} {at}");
        let boxed = Shape::Box { width: 2.0, height: 1.5, thickness: 0.25, slab: 0.3, cells: 2 };
        assert!((boxed.width() - 4.75).abs() < 1e-12);
        assert!((boxed.concrete() - (4.75 * 2.1 - 6.0)).abs() < 1e-12);
    }
}
