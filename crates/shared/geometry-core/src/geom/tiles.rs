//! Map services' tile grids (docs/adr/0208 §3): a grid's matrices (each its
//! pixel's size in the grid system's units, its top left corner as east and
//! north, its tiles' size and how many it has), the level whose pixel is
//! nearest the screen's, the tiles a view meets (the view's centre first),
//! a tile's parent and its part of it, and the mesh a tile is drawn with in
//! the project's system (its corners' grid divided n × n, each node moved by
//! the caller's transformation). Both platforms draw with these, so they ask
//! for the same tiles and draw them in the same place.

use crate::jsmath::{js_max, js_min, log};

/// Web Mercator's half side (EPSG:3857), metres: πR with R the WGS 84 semi-major axis.
pub const MERCATOR_HALF: f64 = 20_037_508.342_789_244;
/// The most tiles a view asks for at once.
pub const MAX_VISIBLE: usize = 400;
/// The finest level a grid made here has.
pub const MAX_LEVEL: u32 = 30;

/// One matrix of a grid.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Matrix {
    /// A pixel's side in the grid system's units.
    pub resolution: f64,
    /// The top left corner, east and north.
    pub x0: f64,
    pub y0: f64,
    pub tile_w: u32,
    pub tile_h: u32,
    /// Tiles across and down.
    pub cols: u64,
    pub rows: u64,
}

impl Matrix {
    /// A tile's side across and down, grid units.
    pub fn span(&self) -> (f64, f64) {
        (
            self.resolution * f64::from(self.tile_w),
            self.resolution * f64::from(self.tile_h),
        )
    }

    /// A tile's box, `[x₁, y₁, x₂, y₂]`: counted in tiles from the corner
    /// and scaled once, so a halving grid's edges at the origin are exact.
    pub fn bounds(&self, col: u64, row: u64) -> [f64; 4] {
        let (w, h) = self.span();
        let (cx, cy) = (self.x0 / w + col as f64, self.y0 / h - row as f64);
        [cx * w, (cy - 1.0) * h, (cx + 1.0) * w, cy * h]
    }
}

/// A tile grid: its matrices, coarsest first.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub matrices: Vec<Matrix>,
    /// Each matrix halves the one before from the same corner with tiles of
    /// the same size: a tile's parent is (col / 2, row / 2) a level up.
    pub quadtree: bool,
}

/// A tile of a grid.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TileRef {
    pub level: u32,
    pub col: u64,
    pub row: u64,
}

impl Grid {
    /// Matrices `0..=max` of a square world from `corner` with level 0's
    /// pixel `res0` and `across` tiles across at level 0 (`down` down).
    fn halving(tile: u32, res0: f64, corner: (f64, f64), across: u64, down: u64, max: u32) -> Grid {
        let max = max.min(MAX_LEVEL);
        let matrices = (0..=max)
            .map(|z| Matrix {
                resolution: res0 / (1u64 << z) as f64,
                x0: corner.0,
                y0: corner.1,
                tile_w: tile,
                tile_h: tile,
                cols: across << z,
                rows: down << z,
            })
            .collect();
        Grid {
            matrices,
            quadtree: true,
        }
    }

    /// Web Mercator's square (`WebMercatorQuad`, XYZ): level z has 2ᶻ × 2ᶻ
    /// tiles of `tile` pixels.
    pub fn web_mercator(tile: u32, max: u32) -> Grid {
        let res0 = 2.0 * MERCATOR_HALF / f64::from(tile.max(1));
        Grid::halving(
            tile.max(1),
            res0,
            (-MERCATOR_HALF, MERCATOR_HALF),
            1,
            1,
            max,
        )
    }

    /// Longitude and latitude, two tiles across and one down at level 0
    /// (`WorldCRS84Quad`), degrees.
    pub fn geographic(tile: u32, max: u32) -> Grid {
        let res0 = 180.0 / f64::from(tile.max(1));
        Grid::halving(tile.max(1), res0, (-180.0, 90.0), 2, 1, max)
    }

    /// The grid a WMS or an ArcGIS `export` is asked on (docs/adr/0208 §5):
    /// Web Mercator's square of resolutions for a system in metres, the
    /// geographic one for a system in degrees, so the same view asks for the
    /// same tiles in every session.
    pub fn virtual_for(degrees: bool, tile: u32) -> Grid {
        if degrees {
            Grid::geographic(tile, MAX_LEVEL)
        } else {
            Grid::web_mercator(tile, MAX_LEVEL)
        }
    }

    /// A service's matrices as read (`kentos_contracts::TileGrid`): a
    /// quadtree when each halves the one before.
    pub fn from_matrices(matrices: Vec<Matrix>) -> Grid {
        let quadtree = !matrices.is_empty()
            && matrices.windows(2).all(|w| {
                let (a, b) = (&w[0], &w[1]);
                let half = b.resolution * 2.0;
                (half - a.resolution).abs() <= a.resolution * 1e-9
                    && (a.x0 - b.x0).abs() <= a.resolution * 1e-6
                    && (a.y0 - b.y0).abs() <= a.resolution * 1e-6
                    && a.tile_w == b.tile_w
                    && a.tile_h == b.tile_h
            });
        Grid { matrices, quadtree }
    }

    /// The level whose pixel is nearest `units_per_px` (log scale; on a tie
    /// the finer), within `min..=max` (the service's levels); none for an
    /// empty grid or a pixel that is not a positive number.
    pub fn level_for(&self, units_per_px: f64, min: u32, max: u32) -> Option<usize> {
        if self.matrices.is_empty() || !(units_per_px.is_finite() && units_per_px > 0.0) {
            return None;
        }
        let last = self.matrices.len() - 1;
        let lo = (min as usize).min(last);
        let hi = (max as usize).clamp(lo, last);
        let want = log(units_per_px);
        let mut best = lo;
        let mut gap = f64::INFINITY;
        for (i, m) in self.matrices.iter().enumerate().take(hi + 1).skip(lo) {
            let d = (log(m.resolution) - want).abs();
            if d <= gap {
                best = i;
                gap = d;
            }
        }
        Some(best)
    }

    /// A tile's parent a level up and the part of it the tile is
    /// (`[u₀, v₀, u₁, v₁]`, fractions from the parent's top left); none at
    /// level 0 or in a grid that is not a quadtree.
    pub fn parent(&self, t: TileRef) -> Option<(TileRef, [f64; 4])> {
        if !self.quadtree || t.level == 0 {
            return None;
        }
        let p = TileRef {
            level: t.level - 1,
            col: t.col / 2,
            row: t.row / 2,
        };
        let u = (t.col % 2) as f64 * 0.5;
        let v = (t.row % 2) as f64 * 0.5;
        Some((p, [u, v, u + 0.5, v + 0.5]))
    }
}

/// The tiles of `level` that the box `bbox` (grid units, `[x₁, y₁, x₂,
/// y₂]`) meets, the nearest `center` first, at most [`MAX_VISIBLE`] of them,
/// into `out` (cleared first).
pub fn visible(
    grid: &Grid,
    level: usize,
    bbox: [f64; 4],
    center: [f64; 2],
    out: &mut Vec<TileRef>,
) {
    out.clear();
    let Some(m) = grid.matrices.get(level) else {
        return;
    };
    let (w, h) = m.span();
    if !(w > 0.0 && h > 0.0 && bbox.iter().all(|v| v.is_finite())) {
        return;
    }
    let col = |x: f64| ((x - m.x0) / w).floor();
    let row = |y: f64| ((m.y0 - y) / h).floor();
    let clamp = |v: f64, n: u64| v.clamp(0.0, (n.max(1) - 1) as f64) as u64;
    let (c1, c2) = (col(bbox[0]), col(bbox[2]));
    let (r1, r2) = (row(bbox[3]), row(bbox[1]));
    if c2 < 0.0 || r2 < 0.0 || c1 > (m.cols as f64 - 1.0) || r1 > (m.rows as f64 - 1.0) {
        return;
    }
    let (c1, c2, r1, r2) = (
        clamp(c1, m.cols),
        clamp(c2, m.cols),
        clamp(r1, m.rows),
        clamp(r2, m.rows),
    );
    let count = (c2 - c1 + 1).saturating_mul(r2 - r1 + 1);
    // The tiles round the centre: a ring at a time is enough when there are too many.
    let (cc, cr) = (
        clamp(col(center[0]), m.cols).clamp(c1, c2),
        clamp(row(center[1]), m.rows).clamp(r1, r2),
    );
    let level = level as u32;
    if count as usize <= MAX_VISIBLE {
        for r in r1..=r2 {
            for c in c1..=c2 {
                out.push(TileRef {
                    level,
                    col: c,
                    row: r,
                });
            }
        }
    } else {
        let mut ring = 0u64;
        while out.len() < MAX_VISIBLE {
            let (a, b) = (cc.saturating_sub(ring), cr.saturating_sub(ring));
            let (e, f) = ((cc + ring).min(c2), (cr + ring).min(r2));
            let mut any = false;
            for r in b.max(r1)..=f {
                for c in a.max(c1)..=e {
                    let edge = c == cc.saturating_sub(ring)
                        || c == cc + ring
                        || r == cr.saturating_sub(ring)
                        || r == cr + ring;
                    if edge && out.len() < MAX_VISIBLE {
                        out.push(TileRef {
                            level,
                            col: c,
                            row: r,
                        });
                        any = true;
                    }
                }
            }
            if !any && a <= c1 && b <= r1 && e >= c2 && f >= r2 {
                break;
            }
            ring += 1;
        }
    }
    // Nearest the centre first (squared distance of the tiles' middles; then row, column).
    let mid = |t: &TileRef| {
        let b = m.bounds(t.col, t.row);
        let (dx, dy) = (
            (b[0] + b[2]) / 2.0 - center[0],
            (b[1] + b[3]) / 2.0 - center[1],
        );
        dx * dx + dy * dy
    };
    out.sort_by(|a, b| {
        mid(a)
            .total_cmp(&mid(b))
            .then((a.row, a.col).cmp(&(b.row, b.col)))
    });
}

/// A view in a grid's system: the box the view's 5 × 5 points fall in, its
/// centre, and a screen pixel's side at the centre (the mean of a step
/// east and a step north).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ViewInGrid {
    pub bbox: [f64; 4],
    pub center: [f64; 2],
    pub units_per_px: f64,
}

/// The view `view` (`[x₁, y₁, x₂, y₂]` in the project's system, which has
/// `px_per_unit` screen pixels a unit) in a grid's system by `to_grid`; none
/// when its centre or every sample has no place there.
pub fn view_in(
    view: [f64; 4],
    px_per_unit: f64,
    mut to_grid: impl FnMut(f64, f64) -> Option<(f64, f64)>,
) -> Option<ViewInGrid> {
    if !(px_per_unit.is_finite() && px_per_unit > 0.0 && view.iter().all(|v| v.is_finite())) {
        return None;
    }
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    let mut any = false;
    for j in 0..5 {
        for i in 0..5 {
            let x = view[0] + (view[2] - view[0]) * f64::from(i) / 4.0;
            let y = view[1] + (view[3] - view[1]) * f64::from(j) / 4.0;
            if let Some((gx, gy)) = to_grid(x, y).filter(|(a, b)| a.is_finite() && b.is_finite()) {
                any = true;
                b[0] = js_min(b[0], gx);
                b[1] = js_min(b[1], gy);
                b[2] = js_max(b[2], gx);
                b[3] = js_max(b[3], gy);
            }
        }
    }
    if !any {
        return None;
    }
    let (cx, cy) = ((view[0] + view[2]) / 2.0, (view[1] + view[3]) / 2.0);
    let step = 1.0 / px_per_unit;
    let c = to_grid(cx, cy)?;
    let e = to_grid(cx + step, cy)?;
    let n = to_grid(cx, cy + step)?;
    let d = |p: (f64, f64)| crate::jsmath::js_hypot(p.0 - c.0, p.1 - c.1);
    let units_per_px = (d(e) + d(n)) / 2.0;
    (units_per_px.is_finite() && units_per_px > 0.0).then_some(ViewInGrid {
        bbox: b,
        center: [c.0, c.1],
        units_per_px,
    })
}

/// What a view shows of a grid: the view in the grid's system, the level
/// whose pixel is nearest the screen's and its tiles, nearest the centre first.
#[derive(Clone, Debug, PartialEq)]
pub struct Shown {
    pub view: ViewInGrid,
    pub level: usize,
    pub tiles: Vec<TileRef>,
}

/// What `view` (`[x₁, y₁, x₂, y₂]` in the project's system, `px_per_unit`
/// screen pixels a unit) shows of `grid` within its levels `min..=max`,
/// `to_grid` taking a point of the project's system into the grid's (both
/// apps' drawing passes, docs/adr/0208 §3).
pub fn shown(
    grid: &Grid,
    min: u32,
    max: u32,
    view: [f64; 4],
    px_per_unit: f64,
    to_grid: impl FnMut(f64, f64) -> Option<(f64, f64)>,
) -> Option<Shown> {
    let v = view_in(view, px_per_unit, to_grid)?;
    let level = grid.level_for(v.units_per_px, min, max)?;
    let mut tiles = Vec::new();
    visible(grid, level, v.bbox, v.center, &mut tiles);
    Some(Shown {
        view: v,
        level,
        tiles,
    })
}

/// How finely a tile at `level` is divided: [`mesh_cells`] of its side
/// (`metres_per_unit` metres a grid unit), a multiple of the atlas slots it
/// takes across or down (`slots`), so each cell is in one slot.
pub fn cells_of(
    grid: &Grid,
    level: u32,
    same_system: bool,
    metres_per_unit: f64,
    slots: u32,
) -> u32 {
    let side = grid
        .matrices
        .get(level as usize)
        .map_or(0.0, |m| m.span().0 * metres_per_unit);
    let n = mesh_cells(same_system, side);
    let k = slots.max(1);
    n.max(k).div_ceil(k) * k
}

/// How finely a tile's mesh is divided: one cell when the systems are the
/// same, else by how long the tile's side is (`metres`): 16 above 100 km, 8
/// above 10 km, 4 below.
pub fn mesh_cells(same_system: bool, metres: f64) -> u32 {
    if same_system {
        1
    } else if metres > 100_000.0 {
        16
    } else if metres > 10_000.0 {
        8
    } else {
        4
    }
}

/// A tile's mesh: its box's (n + 1) × (n + 1) nodes, row by row from the
/// top left, moved by `to_project` into `out` (cleared first) as x, y pairs;
/// a node with no place there is NaN (the cells round it are not drawn).
pub fn mesh(
    grid: &Grid,
    t: TileRef,
    n: u32,
    mut to_project: impl FnMut(f64, f64) -> Option<(f64, f64)>,
    out: &mut Vec<f64>,
) {
    out.clear();
    let Some(m) = grid.matrices.get(t.level as usize) else {
        return;
    };
    let n = n.max(1);
    let [x1, y1, x2, y2] = m.bounds(t.col, t.row);
    out.reserve(((n + 1) * (n + 1) * 2) as usize);
    for j in 0..=n {
        let y = y2 - (y2 - y1) * f64::from(j) / f64::from(n);
        for i in 0..=n {
            let x = x1 + (x2 - x1) * f64::from(i) / f64::from(n);
            match to_project(x, y).filter(|(a, b)| a.is_finite() && b.is_finite()) {
                Some((px, py)) => {
                    out.push(px);
                    out.push(py);
                }
                None => {
                    out.push(f64::NAN);
                    out.push(f64::NAN);
                }
            }
        }
    }
}

/// A tile's place in the project's system through its mesh: the nodes
/// [`mesh`] gives, a point of the tile (`u`, `v` from its top left, 0 to 1)
/// mapped bilinearly in the cell it falls in. A vector tile's vertices go
/// this way (docs/adr/0208 §9), as a picture tile's pixels are drawn on the
/// same mesh: one transformation a node instead of one a vertex, within a
/// millimetre of the exact place at a city's levels (the drawing's own
/// tolerance of a tile is its extent's step, half a metre at level 14).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MeshMap {
    n: u32,
    nodes: Vec<f64>,
}

impl MeshMap {
    /// Tile `t`'s mesh of `n` × `n` cells, its nodes moved by `to_project`.
    pub fn new(
        grid: &Grid,
        t: TileRef,
        n: u32,
        to_project: impl FnMut(f64, f64) -> Option<(f64, f64)>,
    ) -> MeshMap {
        let n = n.max(1);
        let mut nodes = Vec::new();
        mesh(grid, t, n, to_project, &mut nodes);
        MeshMap { n, nodes }
    }

    fn node(&self, i: u32, j: u32) -> (f64, f64) {
        let k = ((j * (self.n + 1) + i) * 2) as usize;
        (self.nodes[k], self.nodes[k + 1])
    }

    /// The point (`u`, `v`) of the tile in the project's system; none for a
    /// tile the mesh has no nodes of, or a cell with a node without a place.
    pub fn at(&self, u: f64, v: f64) -> Option<(f64, f64)> {
        let n = self.n;
        if self.nodes.len() != ((n + 1) * (n + 1) * 2) as usize || !(u.is_finite() && v.is_finite())
        {
            return None;
        }
        // A vertex may lie past the tile (its buffer): the edge cells reach out linearly.
        let (fu, fv) = (u * f64::from(n), v * f64::from(n));
        let i = (js_max(fu.floor(), 0.0) as u32).min(n - 1);
        let j = (js_max(fv.floor(), 0.0) as u32).min(n - 1);
        let (s, r) = (fu - f64::from(i), fv - f64::from(j));
        let (a, b, c, d) = (
            self.node(i, j),
            self.node(i + 1, j),
            self.node(i, j + 1),
            self.node(i + 1, j + 1),
        );
        let x =
            a.0 * (1.0 - s) * (1.0 - r) + b.0 * s * (1.0 - r) + c.0 * (1.0 - s) * r + d.0 * s * r;
        let y =
            a.1 * (1.0 - s) * (1.0 - r) + b.1 * s * (1.0 - r) + c.1 * (1.0 - s) * r + d.1 * s * r;
        (x.is_finite() && y.is_finite()).then_some((x, y))
    }
}

/// A tile's quadkey (Bing's `{quadkey}`): a digit a level, 0–3 from the
/// column's and the row's bits; empty at level 0.
pub fn quadkey(t: TileRef) -> String {
    let mut out = String::with_capacity(t.level as usize);
    for i in (1..=t.level).rev() {
        let mask = 1u64 << (i - 1);
        let mut digit = b'0';
        if t.col & mask != 0 {
            digit += 1;
        }
        if t.row & mask != 0 {
            digit += 2;
        }
        out.push(char::from(digit));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_mesh_maps_its_nodes_exactly_and_an_affine_map_everywhere() {
        let g = Grid::web_mercator(256, 19);
        let t = TileRef {
            level: 2,
            col: 1,
            row: 2,
        };
        // An affine map is a mesh's own: every point exact, past the tile too.
        let affine = |x: f64, y: f64| Some((2.0 * x + 0.5 * y + 10.0, -x + 3.0 * y - 4.0));
        let m = MeshMap::new(&g, t, 4, affine);
        let [x1, y1, x2, y2] = g.matrices[2].bounds(1, 2);
        for (u, v) in [
            (0.0, 0.0),
            (1.0, 1.0),
            (0.3, 0.7),
            (0.5, 0.5),
            (-0.05, 1.02),
        ] {
            let (x, y) = (x1 + (x2 - x1) * u, y2 - (y2 - y1) * v);
            let want = affine(x, y).expect("a place");
            let got = m.at(u, v).expect("a place");
            assert!(
                (got.0 - want.0).abs() < 1e-6 && (got.1 - want.1).abs() < 1e-6,
                "{u} {v}: {got:?} {want:?}"
            );
        }
        // A node without a place leaves its cells without one.
        let holed = MeshMap::new(&g, t, 2, |x, y| {
            (x > x1 + (x2 - x1) * 0.9).then_some((x, y))
        });
        assert!(holed.at(0.1, 0.1).is_none());
        assert!(MeshMap::default().at(0.5, 0.5).is_none());
    }

    #[test]
    fn web_mercators_levels_halve_and_pick_the_nearest() {
        let g = Grid::web_mercator(256, 19);
        assert_eq!(g.matrices.len(), 20);
        assert!(g.quadtree);
        assert!((g.matrices[0].resolution - 156_543.033_928_041).abs() < 1e-6);
        assert_eq!(g.matrices[3].cols, 8);
        // 1 m a pixel lies between level 17 (1.19 m) and 18 (0.60 m): 17 is nearer in log scale.
        assert_eq!(g.level_for(1.0, 0, 19), Some(17));
        assert_eq!(g.level_for(0.6, 0, 19), Some(18));
        assert_eq!(g.level_for(0.01, 0, 19), Some(19));
        assert_eq!(g.level_for(1e9, 2, 19), Some(2));
        assert_eq!(g.level_for(0.0, 0, 19), None);
    }

    #[test]
    fn the_tiles_a_box_meets_nearest_the_centre_first() {
        let g = Grid::web_mercator(256, 19);
        let mut out = Vec::new();
        // Level 1: four tiles; a box in the north-east quarter meets one.
        visible(&g, 1, [1.0, 1.0, 2.0, 2.0], [1.5, 1.5], &mut out);
        assert_eq!(
            out,
            vec![TileRef {
                level: 1,
                col: 1,
                row: 0
            }]
        );
        // The whole world at level 2: 16, the four round the centre first.
        let w = MERCATOR_HALF;
        visible(&g, 2, [-w, -w, w, w], [0.0, 0.0], &mut out);
        assert_eq!(out.len(), 16);
        let first: Vec<(u64, u64)> = out.iter().take(4).map(|t| (t.col, t.row)).collect();
        assert_eq!(first, vec![(1, 1), (2, 1), (1, 2), (2, 2)]);
        // Too many: at most MAX_VISIBLE, round the centre.
        visible(&g, 12, [-w, -w, w, w], [0.0, 0.0], &mut out);
        assert_eq!(out.len(), MAX_VISIBLE);
        assert_eq!((out[0].col, out[0].row), (2047, 2047));
        // Outside the world: none.
        visible(&g, 3, [3.0 * w, 0.0, 4.0 * w, 1.0], [0.0, 0.0], &mut out);
        assert!(out.is_empty());
    }

    #[test]
    fn a_tiles_parent_and_its_quarter() {
        let g = Grid::web_mercator(256, 19);
        let (p, uv) = g
            .parent(TileRef {
                level: 5,
                col: 11,
                row: 6,
            })
            .expect("a parent");
        assert_eq!(
            p,
            TileRef {
                level: 4,
                col: 5,
                row: 3
            }
        );
        assert_eq!(uv, [0.5, 0.0, 1.0, 0.5]);
        assert_eq!(
            g.parent(TileRef {
                level: 0,
                col: 0,
                row: 0
            }),
            None
        );
    }

    #[test]
    fn quadkeys_as_bing_writes_them() {
        assert_eq!(
            quadkey(TileRef {
                level: 3,
                col: 3,
                row: 5
            }),
            "213"
        );
        assert_eq!(
            quadkey(TileRef {
                level: 0,
                col: 0,
                row: 0
            }),
            ""
        );
    }

    #[test]
    fn a_view_in_another_system_and_a_mesh() {
        // The identity: the box, the centre and the pixel are the view's.
        let v = view_in([0.0, 0.0, 100.0, 50.0], 2.0, |x, y| Some((x, y))).expect("a view");
        assert_eq!(v.bbox, [0.0, 0.0, 100.0, 50.0]);
        assert_eq!(v.center, [50.0, 25.0]);
        assert!((v.units_per_px - 0.5).abs() < 1e-12);
        let g = Grid::web_mercator(256, 19);
        let mut out = Vec::new();
        mesh(
            &g,
            TileRef {
                level: 1,
                col: 0,
                row: 0,
            },
            2,
            |x, y| Some((x, y)),
            &mut out,
        );
        assert_eq!(out.len(), 18);
        assert_eq!((out[0], out[1]), (-MERCATOR_HALF, MERCATOR_HALF));
        assert_eq!((out[16], out[17]), (0.0, 0.0));
        // A node with no place is NaN.
        mesh(
            &g,
            TileRef {
                level: 1,
                col: 0,
                row: 0,
            },
            1,
            |x, _| (x > -1.0).then_some((x, 0.0)),
            &mut out,
        );
        assert!(out[0].is_nan());
        assert_eq!(out[2], 0.0);
    }
}
