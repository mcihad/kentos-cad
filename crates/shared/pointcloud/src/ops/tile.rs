//! Karola (docs/adr/0207 §7): tiles of `size` metres with their corners on
//! the size's multiples; a point on a tile's edge belongs to the tile on its
//! right and above it. A tile's file is named after the source and its lower
//! left corner: `<ad>_<x>_<y>`.

/// The tile of (`x`, `y`): its column and row on the size's multiples.
#[inline]
pub fn tile_of(x: f64, y: f64, size: f64) -> (i64, i64) {
    ((x / size).floor() as i64, (y / size).floor() as i64)
}

/// A tile's lower left corner.
pub fn corner(t: (i64, i64), size: f64) -> [f64; 2] {
    // + 0.0: a −0 corner is written 0.
    [t.0 as f64 * size + 0.0, t.1 as f64 * size + 0.0]
}

/// A tile's file name without its extension: `<stem>_<x>_<y>`, the corner's
/// numbers as JavaScript writes them (no trailing zeros).
pub fn name(stem: &str, t: (i64, i64), size: f64) -> String {
    let [x, y] = corner(t, size);
    format!("{stem}_{}_{}", js(x), js(y))
}

/// A number as JavaScript's `String(n)` writes it, for the values a tile's corner takes.
fn js(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tiles_and_their_names() {
        assert_eq!(tile_of(100.0, 49.999, 50.0), (2, 0));
        assert_eq!(tile_of(-0.5, 0.0, 50.0), (-1, 0));
        assert_eq!(name("koy", (9752, 88406), 50.0), "koy_487600_4420300");
        assert_eq!(name("a", (-1, 0), 2.5), "a_-2.5_0");
        assert_eq!(name("a", (0, 0), 2.5), "a_0_0");
    }
}
