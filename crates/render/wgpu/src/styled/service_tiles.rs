//! Map services' tiles on the GPU (docs/adr/0208 §3, §4, §9): a layer drawn
//! from a service shows, each frame, the tiles of the level whose pixel is
//! nearest the screen's (`geom::tiles`), the view's centre first. A picture
//! tile goes into the raster atlas in 256-pixel slots (a 512-pixel tile
//! takes four) and is drawn as a mesh: its box divided n × n in the
//! service's system, each node moved into the project's (one cell when the
//! two systems are the same). A tile the host has not made yet shows the
//! part of the nearest coarser tile the atlas holds. A vector tile is a
//! styled layer of its own, uploaded once and drawn where the service's
//! batch is drawn.
//!
//! The host answers [`super::ImageSource::service_view`],
//! [`super::ImageSource::service_tile`] and
//! [`super::ImageSource::service_vector`]; what it does not have yet it
//! starts making, and asks for another frame when done.

use std::collections::HashMap;
use std::sync::Arc;

use kentos_geometry_core::geom::tiles::{Grid, Shown, TileRef, cells_of, mesh, shown};

use super::gpu::StyledLayerPart;
use super::picture::ImageSource;
use super::raster_tiles::{RasterAtlas, RasterQuads, SLOT, TileId, UPLOADS_PER_FRAME};

/// The side of a slot's picture: a tile's 256 pixels (`SLOT` with its apron).
pub const PIECE: u32 = SLOT - 2;

/// A transformation between the project's system and a service's: east and
/// north in, east and north out; none where the point has no place there.
pub type Transform = Arc<dyn Fn(f64, f64) -> Option<(f64, f64)> + Send + Sync>;

/// A service as the pass draws it, made by the host once its tiles are known.
pub struct ServiceView {
    /// Its tiles' own: the service and the system it is drawn in, hashed.
    pub id: u64,
    pub grid: Grid,
    pub min_level: u32,
    pub max_level: u32,
    /// Vector tiles ([`super::ImageSource::service_vector`]) rather than pictures.
    pub vector: bool,
    /// The project's system is the grid's: the tiles are drawn as they are.
    pub same_system: bool,
    pub to_grid: Transform,
    pub to_project: Transform,
    /// Metres a grid unit is about (a degree's 111 km on a geographic grid),
    /// for how finely a tile's mesh is divided.
    pub metres_per_unit: f64,
    /// Slots across and down a picture tile takes (1 for 256 pixels, 2 for 512).
    pub across: u32,
    pub down: u32,
}

/// A picture tile: `across` × `down` slots, row by row, each `SLOT` × `SLOT`
/// premultiplied RGBA (its apron repeats its edge).
pub struct ServiceImage {
    pub across: u32,
    pub down: u32,
    pub slots: Vec<Arc<Vec<u8>>>,
}

/// What the host has of a tile.
#[derive(Clone)]
pub enum ServiceTile {
    Image(Arc<ServiceImage>),
    /// Done, and nothing to draw (the server has no tile there).
    Empty,
}

/// A vector tile: its batches as a styled layer, in the drawing's tiles
/// (docs/adr/0157), its id unique to this tile and this build.
pub struct VectorTile {
    pub part: Arc<StyledLayerPart>,
}

/// A service layer's paint as a batch carries it.
#[derive(Clone, Debug, PartialEq)]
pub struct ServicePaint {
    pub service: String,
}

/// The side of the drawing's position tiles, metres (docs/adr/0157; the
/// style core's `TILE`).
const POSITION_TILE: f64 = 65536.0;

/// The tile of the drawing's position grid the view's `center` (metres from
/// the layers' origin) is in: a service's vertices are relative to it.
pub fn center_tile(center: [f64; 2]) -> [f64; 2] {
    let t = |v: f64| {
        let i = (v / POSITION_TILE).round();
        if i.is_finite() {
            i * POSITION_TILE
        } else {
            0.0
        }
    };
    [t(center[0]), t(center[1])]
}

/// The zoom a vector tile's style is read at (MapLibre's: 512-pixel tiles)
/// for `units_per_px` grid units a screen pixel.
pub fn display_zoom(service: &ServiceView, units_per_px: f64) -> f64 {
    let Some(m) = service.grid.matrices.first() else {
        return 0.0;
    };
    let z = (m.resolution * f64::from(m.tile_w) / 512.0 / units_per_px).log2();
    if z.is_finite() {
        z.clamp(0.0, 24.0)
    } else {
        0.0
    }
}

/// A mesh's place: the service view, the tile and the cells across.
type MeshKey = (u64, TileRef, u32);

/// A mesh's nodes and the frame that last drew it.
type HeldMesh = (Arc<Vec<f64>>, u64);

/// The meshes of tiles drawn lately, by service, tile and cells: worked out
/// once while the view moves over them.
#[derive(Default)]
pub struct Meshes {
    held: HashMap<MeshKey, HeldMesh>,
    frame: u64,
}

/// Meshes kept at most.
const MESHES: usize = 4096;

impl Meshes {
    pub(crate) fn begin_frame(&mut self) {
        self.frame += 1;
        if self.held.len() > MESHES {
            let keep = self.frame.saturating_sub(4);
            self.held.retain(|_, (_, at)| *at >= keep);
        }
    }

    fn get(&mut self, view: &ServiceView, t: TileRef, n: u32) -> Arc<Vec<f64>> {
        let frame = self.frame;
        let entry = self.held.entry((view.id, t, n)).or_insert_with(|| {
            let mut nodes = Vec::new();
            let to_project = view.to_project.clone();
            mesh(&view.grid, t, n, |x, y| to_project(x, y), &mut nodes);
            (Arc::new(nodes), frame)
        });
        entry.1 = frame;
        entry.0.clone()
    }
}

/// The view in a service's grid, and the level and tiles it draws (the geometry core's).
pub type InView = Shown;

/// What `view` shows of `service` in the project's box `box_world` (east and
/// north, `px_per_unit` logical pixels a unit): the level and its tiles in
/// view, nearest the centre first (`geom::tiles::shown`, the web's too).
pub fn in_view(service: &ServiceView, box_world: [f64; 4], px_per_unit: f64) -> Option<InView> {
    let to_grid = service.to_grid.clone();
    shown(
        &service.grid,
        service.min_level,
        service.max_level,
        box_world,
        px_per_unit,
        |x, y| to_grid(x, y),
    )
}

/// How finely a tile is divided: a multiple of its slots across and down.
fn cells(service: &ServiceView, t: TileRef) -> u32 {
    cells_of(
        &service.grid,
        t.level,
        service.same_system,
        service.metres_per_unit,
        service.across.max(service.down),
    )
}

impl RasterQuads {
    /// A service's picture tiles in view (`box_world` east and north in the
    /// project's system), as quads relative to `origin` (east and north of the
    /// batch's origin in the world), appended; the vertex range they take.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add_service(
        &mut self,
        paint: &ServicePaint,
        service: &ServiceView,
        origin: [f64; 2],
        box_world: [f64; 4],
        px_per_unit: f64,
        atlas: &mut RasterAtlas,
        meshes: &mut Meshes,
        queue: &wgpu::Queue,
        source: &dyn ImageSource,
        uploads: &mut usize,
    ) -> std::ops::Range<u32> {
        let start = (self.data.len() / 4) as u32;
        let Some(seen) = in_view(service, box_world, px_per_unit) else {
            return start..start;
        };
        let (across, down) = (service.across.max(1), service.down.max(1));
        for &t in &seen.tiles {
            let n = cells(service, t);
            // Each slot of the tile: held in the atlas, or put there now.
            let mut slots: Vec<Option<[f32; 4]>> = Vec::new();
            let mut whole = true;
            match source.service_tile(&paint.service, t) {
                Some(ServiceTile::Empty) => continue,
                Some(ServiceTile::Image(img)) => {
                    for sy in 0..down {
                        for sx in 0..across {
                            let id = TileId {
                                raster: service.id,
                                level: t.level,
                                tx: t.col * u64::from(across) + u64::from(sx),
                                ty: t.row * u64::from(down) + u64::from(sy),
                            };
                            let slot = match atlas.held.get(&id).copied() {
                                Some(slot) => Some(slot),
                                None if *uploads < UPLOADS_PER_FRAME => {
                                    *uploads += 1;
                                    img.slots
                                        .get((sy * across + sx) as usize)
                                        .and_then(|rgba| atlas.put(queue, id, rgba))
                                }
                                None => {
                                    self.waiting = true;
                                    None
                                }
                            };
                            whole &= slot.is_some();
                            slots.push(slot.map(|s| {
                                atlas.touch(s);
                                atlas.uv(s, 0.0, 0.0, f64::from(PIECE), f64::from(PIECE))
                            }));
                        }
                    }
                }
                None => {
                    self.waiting = true;
                    whole = false;
                }
            }
            let nodes = meshes.get(service, t, n);
            if whole && !slots.is_empty() {
                self.cells(&nodes, n, origin, |fu, fv| {
                    // The slot the cell's middle is in, and the cell's place in it.
                    let sx = ((fu * f64::from(across)).floor() as u32).min(across - 1);
                    let sy = ((fv * f64::from(down)).floor() as u32).min(down - 1);
                    let rect = slots[(sy * across + sx) as usize]?;
                    Some((
                        rect,
                        f64::from(sx) / f64::from(across),
                        f64::from(sy) / f64::from(down),
                        f64::from(across),
                        f64::from(down),
                    ))
                });
                continue;
            }
            // The part of the nearest coarser tile the atlas holds.
            let mut child = t;
            let mut part = [0.0, 0.0, 1.0, 1.0];
            for _ in 0..4 {
                let Some((parent, quarter)) = service.grid.parent(child) else {
                    break;
                };
                // The tile's part of the parent: the quarter's part of the part before.
                let (w, h) = (quarter[2] - quarter[0], quarter[3] - quarter[1]);
                part = [
                    quarter[0] + part[0] * w,
                    quarter[1] + part[1] * h,
                    quarter[0] + part[2] * w,
                    quarter[1] + part[3] * h,
                ];
                child = parent;
                let held = |sx: u32, sy: u32| {
                    atlas
                        .held
                        .get(&TileId {
                            raster: service.id,
                            level: parent.level,
                            tx: parent.col * u64::from(across) + u64::from(sx),
                            ty: parent.row * u64::from(down) + u64::from(sy),
                        })
                        .copied()
                };
                let mut rects = Vec::with_capacity((across * down) as usize);
                let mut all = true;
                for sy in 0..down {
                    for sx in 0..across {
                        let r = held(sx, sy);
                        all &= r.is_some();
                        rects.push(r);
                    }
                }
                // Only the slots the part reaches need to be held.
                let reached = |fu: f64, fv: f64| {
                    let sx = ((fu * f64::from(across)).floor() as u32).min(across - 1);
                    let sy = ((fv * f64::from(down)).floor() as u32).min(down - 1);
                    (sx, sy)
                };
                let (a, b) = reached(part[0], part[1]);
                let (c, d) = reached(part[2] - 1e-9, part[3] - 1e-9);
                let needed = (b..=d)
                    .all(|sy| (a..=c).all(|sx| rects[(sy * across + sx) as usize].is_some()));
                if !(all || needed) {
                    continue;
                }
                for r in rects.iter().flatten() {
                    atlas.touch(*r);
                }
                let rects: Vec<Option<[f32; 4]>> = rects
                    .iter()
                    .map(|r| r.map(|s| atlas.uv(s, 0.0, 0.0, f64::from(PIECE), f64::from(PIECE))))
                    .collect();
                let part = part;
                self.cells(&nodes, n, origin, |fu, fv| {
                    let (pu, pv) = (
                        part[0] + fu * (part[2] - part[0]),
                        part[1] + fv * (part[3] - part[1]),
                    );
                    let sx = ((pu * f64::from(across)).floor() as u32).min(across - 1);
                    let sy = ((pv * f64::from(down)).floor() as u32).min(down - 1);
                    let rect = rects[(sy * across + sx) as usize]?;
                    // The cell's fractions mapped through the part into the slot:
                    // (part₀ + f·w − sx/across)·across = (f − (sx/across − part₀)/w)·(w·across).
                    let (w, h) = (part[2] - part[0], part[3] - part[1]);
                    Some((
                        rect,
                        (f64::from(sx) / f64::from(across) - part[0]) / w,
                        (f64::from(sy) / f64::from(down) - part[1]) / h,
                        f64::from(across) * w,
                        f64::from(down) * h,
                    ))
                });
                break;
            }
        }
        start..(self.data.len() / 4) as u32
    }

    /// A tile's mesh cells as triangles: `slot(fu, fv)` gives, for the
    /// fractions of a cell's middle, its slot's rectangle and how the tile's
    /// fractions map into it: `u = (f − du) · ku` within the rectangle.
    fn cells(
        &mut self,
        nodes: &[f64],
        n: u32,
        origin: [f64; 2],
        mut slot: impl FnMut(f64, f64) -> Option<([f32; 4], f64, f64, f64, f64)>,
    ) {
        let side = (n + 1) as usize;
        if nodes.len() < side * side * 2 {
            return;
        }
        let at = |i: u32, j: u32| {
            let k = (j as usize * side + i as usize) * 2;
            (nodes[k], nodes[k + 1])
        };
        let nf = f64::from(n);
        for j in 0..n {
            for i in 0..n {
                let corners = [at(i, j), at(i + 1, j), at(i + 1, j + 1), at(i, j + 1)];
                if corners
                    .iter()
                    .any(|(x, y)| !x.is_finite() || !y.is_finite())
                {
                    continue;
                }
                let (mu, mv) = ((f64::from(i) + 0.5) / nf, (f64::from(j) + 0.5) / nf);
                let Some((rect, du, dv, ku, kv)) = slot(mu, mv) else {
                    continue;
                };
                let uv = |fi: u32, fj: u32| {
                    let fu = (f64::from(fi) / nf - du) * ku;
                    let fv = (f64::from(fj) / nf - dv) * kv;
                    [
                        rect[0] + (rect[2] - rect[0]) * fu as f32,
                        rect[1] + (rect[3] - rect[1]) * fv as f32,
                    ]
                };
                let p = |k: usize| {
                    [
                        (corners[k].0 - origin[0]) as f32,
                        (corners[k].1 - origin[1]) as f32,
                    ]
                };
                let q = [
                    (p(0), uv(i, j)),
                    (p(1), uv(i + 1, j)),
                    (p(2), uv(i + 1, j + 1)),
                    (p(3), uv(i, j + 1)),
                ];
                for k in [0, 1, 2, 0, 2, 3] {
                    let (pos, t) = q[k];
                    self.data.extend_from_slice(&[pos[0], pos[1], t[0], t[1]]);
                }
            }
        }
    }
}
