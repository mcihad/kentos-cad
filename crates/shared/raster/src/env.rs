//! The analysis environment (docs/adr/0643 §1): what every raster tool of a run shares besides its own settings, as
//! a GIS's environment settings are: the analysis area, the cell size, the raster whose cells the result lines up
//! with, and the value written where a cell has none. A tool works out its result's grid as it always has (the
//! base: its input's, the inputs' common or union grid); the environment then gives the grid the run computes on.
//!
//! - **Area** alone: the base grid's cells the area touches (as Kapsam does in docs/adr/0535), the same lattice; a
//!   cell is worked out from its neighbours in the source as the whole raster's would be.
//! - **Cell size** or **snap raster**: a new lattice. Its cells are square, `cell` on a side (else the snap
//!   raster's), its edges on the snap raster's lines (else the base grid's origin's); it covers the area (else the
//!   base grid) with whole cells. Both grids must be north up.
//! - **NoData**: the value a result cell without a value takes (and the file's nodata tag); a float result's is NaN
//!   otherwise.

use crate::grid::Grid;
use crate::inputs::point_of;

/// How far within a cell a side of the area may lie and still be taken as on the cell's edge.
const EDGE: f64 = 1e-9;

/// A run's analysis environment; every part optional.
#[derive(Clone, Debug, Default, PartialEq, serde::Deserialize, serde::Serialize)]
#[serde(default, rename_all = "camelCase")]
pub struct Environment {
    /// The analysis area: minimum east, north, maximum east, north in the drawing.
    pub area: Option<[f64; 4]>,
    /// The result's cell size (drawing units, a side).
    pub cell: Option<f64>,
    /// The snap raster's place (its affine): the result's cells line up with its.
    pub snap: Option<[f64; 6]>,
    /// The value a result cell without a value takes.
    pub nodata: Option<f64>,
}

impl Environment {
    /// Whether it changes nothing.
    pub fn is_empty(&self) -> bool {
        self.area.is_none() && self.cell.is_none() && self.snap.is_none() && self.nodata.is_none()
    }

    /// Whether it changes the grid's lattice (its cell size or the lines of its cells).
    pub fn regrids(&self) -> bool {
        self.cell.is_some() || self.snap.is_some()
    }

    /// The grid a run computes on, from the tool's own `base` (see the module).
    pub fn grid(&self, base: &Grid) -> Result<Grid, String> {
        if !self.regrids() {
            return match self.area {
                None => Ok(*base),
                Some(area) => window(base, area),
            };
        }
        lattice(base, self)
    }
}

/// The base's extent: least x, least y, largest x, largest y of its corners.
fn extent(g: &Grid) -> [f64; 4] {
    let (w, h) = (f64::from(g.width), f64::from(g.height));
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for (i, j) in [(0.0, 0.0), (w, 0.0), (0.0, h), (w, h)] {
        let (x, y) = point_of(&g.affine, i, j);
        b = [b[0].min(x), b[1].min(y), b[2].max(x), b[3].max(y)];
    }
    b
}

fn checked_area(area: [f64; 4]) -> Result<[f64; 4], String> {
    let [xmin, ymin, xmax, ymax] = area;
    if !area.iter().all(|v| v.is_finite()) || xmin > xmax || ymin > ymax {
        return Err(
            "Analiz alanının köşeleri okunamadı (en küçük doğu, kuzey, en büyük doğu, kuzey)."
                .into(),
        );
    }
    Ok(area)
}

/// The cells of `base` the area touches, on its own lattice; refused when the area misses the raster.
fn window(base: &Grid, area: [f64; 4]) -> Result<Grid, String> {
    let [xmin, ymin, xmax, ymax] = checked_area(area)?;
    let [x0, a, b, y0, c, d] = base.affine;
    let det = a * d - b * c;
    if det == 0.0 || !det.is_finite() {
        return Err("Rasterin yeri (afin dönüşümü) tersinmiyor.".into());
    }
    // Each corner of the area into the base's cell coordinates.
    let (mut umin, mut umax, mut vmin, mut vmax) = (
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
    );
    for (x, y) in [(xmin, ymin), (xmax, ymin), (xmin, ymax), (xmax, ymax)] {
        let (dx, dy) = (x - x0, y - y0);
        let u = (d * dx - b * dy) / det;
        let v = (-c * dx + a * dy) / det;
        umin = umin.min(u);
        umax = umax.max(u);
        vmin = vmin.min(v);
        vmax = vmax.max(v);
    }
    let i0 = ((umin + EDGE).floor()).max(0.0);
    let j0 = ((vmin + EDGE).floor()).max(0.0);
    let i1 = ((umax - EDGE).ceil()).min(f64::from(base.width));
    let j1 = ((vmax - EDGE).ceil()).min(f64::from(base.height));
    if !(i1 > i0 && j1 > j0) {
        return Err("Analiz alanı rasterle kesişmiyor; alanı rasterin üstünde seçin.".into());
    }
    let (ox, oy) = point_of(&base.affine, i0, j0);
    Grid::of([ox, a, b, oy, c, d], (i1 - i0) as u32, (j1 - j0) as u32)
}

/// A new lattice: the environment's cell size and snap lines over the area (else the base's extent).
fn lattice(base: &Grid, env: &Environment) -> Result<Grid, String> {
    let north_up = |g: &[f64; 6]| g[2] == 0.0 && g[4] == 0.0 && g[1] > 0.0 && g[5] < 0.0;
    if !north_up(&base.affine) {
        return Err(
            "Hücre boyu ve hizalama ortamı kuzeyi yukarıda olan rasterle çalışır; dönmüş rasteri önce yeniden örnekleyin."
                .into(),
        );
    }
    if let Some(s) = &env.snap
        && !north_up(s)
    {
        return Err("Hizalama rasterinin kuzeyi yukarıda olmalı.".into());
    }
    let cell = match (env.cell, &env.snap) {
        (Some(c), _) => c,
        (None, Some(s)) => s[1],
        (None, None) => base.affine[1],
    };
    if !(cell.is_finite() && cell > 0.0) {
        return Err("Ortamın hücre boyu sıfırdan büyük bir sayı olmalı.".into());
    }
    // The lines the cells' edges lie on: the snap raster's, else the base's origin.
    let (ox, oy) = match &env.snap {
        Some(s) => (s[0], s[3]),
        None => (base.affine[0], base.affine[3]),
    };
    let whole = extent(base);
    let [xmin, ymin, xmax, ymax] = match env.area {
        Some(a) => {
            let a = checked_area(a)?;
            let cut = [
                a[0].max(whole[0]),
                a[1].max(whole[1]),
                a[2].min(whole[2]),
                a[3].min(whole[3]),
            ];
            if !(cut[2] > cut[0] && cut[3] > cut[1]) {
                return Err(
                    "Analiz alanı rasterle kesişmiyor; alanı rasterin üstünde seçin.".into(),
                );
            }
            cut
        }
        None => whole,
    };
    // Whole cells on the lines, covering the box: left and top edges down to a line, right and bottom up.
    let left = ox + ((xmin - ox) / cell + EDGE).floor() * cell;
    let right = ox + ((xmax - ox) / cell - EDGE).ceil() * cell;
    let top = oy + ((ymax - oy) / cell - EDGE).ceil() * cell;
    let bottom = oy + ((ymin - oy) / cell + EDGE).floor() * cell;
    let w = ((right - left) / cell).round();
    let h = ((top - bottom) / cell).round();
    if !(w >= 1.0 && h >= 1.0) || w > f64::from(u32::MAX) || h > f64::from(u32::MAX) {
        return Err(
            "Ortamın ızgarası kurulamadı: alan hücre boyuna göre boş ya da çok büyük.".into(),
        );
    }
    Grid::of([left, cell, 0.0, top, 0.0, -cell], w as u32, h as u32)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> Grid {
        // 100 × 80 cells of 2 m from (1000, 2000) north up.
        Grid::of([1000.0, 2.0, 0.0, 2000.0, 0.0, -2.0], 100, 80).expect("grid")
    }

    #[test]
    fn an_area_alone_cuts_the_base_on_its_own_lattice() {
        let env = Environment {
            area: Some([1011.0, 1900.5, 1020.0, 1950.0]),
            ..Environment::default()
        };
        let g = env.grid(&base()).expect("window");
        // Columns 5..10 (x 1010..1020), rows 25..50 (y 1950..1900).
        assert_eq!(g.affine, [1010.0, 2.0, 0.0, 1950.0, 0.0, -2.0]);
        assert_eq!((g.width, g.height), (5, 25));
        let off = Environment {
            area: Some([0.0, 0.0, 10.0, 10.0]),
            ..Environment::default()
        };
        assert!(off.grid(&base()).is_err());
    }

    #[test]
    fn a_cell_size_and_a_snap_raster_give_a_new_lattice_over_the_area() {
        let env = Environment {
            area: Some([1011.0, 1900.5, 1020.0, 1950.0]),
            cell: Some(5.0),
            snap: Some([3.0, 1.0, 0.0, 7.0, 0.0, -1.0]),
            nodata: None,
        };
        let g = env.grid(&base()).expect("lattice");
        // Lines at x ≡ 3 (mod 5), y ≡ 7 (mod 5): 1008 … 1023, 1952 … 1897.
        assert_eq!(g.affine, [1008.0, 5.0, 0.0, 1952.0, 0.0, -5.0]);
        assert_eq!((g.width, g.height), (3, 11));
        let empty = Environment::default();
        assert_eq!(empty.grid(&base()).expect("same"), base());
        // The base's own cell and lines: its own grid.
        let own = Environment {
            cell: Some(2.0),
            ..Environment::default()
        };
        assert_eq!(own.grid(&base()).expect("same"), base());
    }
}
