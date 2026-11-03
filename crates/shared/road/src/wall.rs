//! A retaining wall along a corridor's side (plan/road.md Y11.3, docs/adr/0909 §3; ROAD-DRAIN-11): a cantilever wall
//! whose face stands at the road's hinge in place of the slope. At each station its top is the higher of the road's
//! edge and the ground under it, raised by what stands above; its footing's top lies the cover below the lower of the
//! two (Civil 3D's RetainWallTapered measures the wall from the footing's top to the ground or the attachment point the
//! same way). The stem is `top` thick at its top and widens by `batter` per metre down; the footing is `footing` wide
//! and `base` thick. The sizes are the project's; nothing here is a standard's number.

/// The wall's cross as the project gives it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shape {
    /// The stem's thickness at its top.
    pub top: f64,
    /// How much the stem widens per metre down (its back face's batter, m/m).
    pub batter: f64,
    /// The footing's width across the wall.
    pub footing: f64,
    /// The footing's thickness.
    pub base: f64,
    /// How far the footing reaches in front of the face.
    pub toe: f64,
    /// The ground over the footing's top.
    pub cover: f64,
    /// What stands above the higher of the road's edge and the ground (a parapet).
    pub above: f64,
}

/// The wall at one station.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Slice {
    /// The stem's top.
    pub top: f64,
    /// The footing's top.
    pub footing: f64,
    /// The stem's height: its top above the footing's.
    pub height: f64,
    /// The cross's concrete area: the stem's trapezoid and the footing.
    pub area: f64,
    /// Whether the wall holds the road up (fill: the road's edge above the ground) rather than the ground back.
    pub fill: bool,
}

/// The wall at a station whose road edge is `road` and whose ground under the face is `ground`.
pub fn slice(road: f64, ground: f64, shape: &Shape) -> Slice {
    let top = road.max(ground) + shape.above;
    let footing = road.min(ground) - shape.cover;
    let height = top - footing;
    Slice { top, footing, height, area: area(height, shape), fill: road > ground }
}

/// The cross's concrete area of a stem `height` high.
pub fn area(height: f64, shape: &Shape) -> f64 {
    height * (2.0 * shape.top + shape.batter * height) / 2.0 + shape.footing * shape.base
}

/// The wall's totals along its face: `places` its face's plan places and `slices` the wall there, in station order.
/// Its plan length, its stem's face area (each stretch its length times its heights' mean) and its concrete (each
/// stretch its length times its areas' mean).
pub fn totals(places: &[[f64; 2]], slices: &[Slice]) -> (f64, f64, f64) {
    let (mut length, mut face, mut concrete) = (0.0, 0.0, 0.0);
    for (p, s) in places.windows(2).zip(slices.windows(2)) {
        let l = (p[1][0] - p[0][0]).hypot(p[1][1] - p[0][1]);
        length += l;
        face += l * (s[0].height + s[1].height) / 2.0;
        concrete += l * (s[0].area + s[1].area) / 2.0;
    }
    (length, face, concrete)
}

/// The ground at `offset` on a sample line's terrain (its places in offset order): the straight line between the places
/// around it, none off the surface (a stretch that is a gap, or a place without a height).
pub fn ground_at(ground: &[crate::section::GroundPoint], offset: f64) -> Option<f64> {
    for w in ground.windows(2) {
        let (a, b) = (&w[0], &w[1]);
        if a.offset <= offset + 1e-9 && offset <= b.offset + 1e-9 {
            if b.gap {
                return None;
            }
            let (za, zb) = (a.z?, b.z?);
            let span = b.offset - a.offset;
            if span <= 0.0 {
                return Some(za);
            }
            let t = ((offset - a.offset) / span).clamp(0.0, 1.0);
            return Some(za + (zb - za) * t);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    const SHAPE: Shape = Shape { top: 0.3, batter: 0.1, footing: 2.0, base: 0.5, toe: 0.5, cover: 0.6, above: 0.9 };

    #[test]
    fn a_fill_wall_stands_from_below_the_ground_to_above_the_road() {
        let s = slice(105.0, 102.0, &SHAPE);
        assert!(s.fill);
        assert!((s.top - 105.9).abs() < 1e-12 && (s.footing - 101.4).abs() < 1e-12);
        assert!((s.height - 4.5).abs() < 1e-12);
        assert!((s.area - (4.5 * (0.6 + 0.45) / 2.0 + 1.0)).abs() < 1e-12);
        let cut = slice(102.0, 105.0, &SHAPE);
        assert!(!cut.fill && (cut.height - 4.5).abs() < 1e-12);
    }

    #[test]
    fn totals_take_the_means_of_the_stretches() {
        let a = slice(105.0, 102.0, &SHAPE);
        let b = slice(104.0, 102.0, &SHAPE);
        let (l, f, c) = totals(&[[0.0, 0.0], [3.0, 4.0]], &[a, b]);
        assert!((l - 5.0).abs() < 1e-12);
        assert!((f - 5.0 * (a.height + b.height) / 2.0).abs() < 1e-12);
        assert!((c - 5.0 * (a.area + b.area) / 2.0).abs() < 1e-12);
    }
}
