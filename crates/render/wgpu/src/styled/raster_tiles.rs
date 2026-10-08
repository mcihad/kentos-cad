//! Rasters' tiles on the GPU (docs/adr/0204 §5): one atlas page of 258-pixel
//! slots (a tile and a pixel of its neighbours round it, so bilinear
//! sampling shows no seam), the least recently drawn let go first; each
//! frame, each raster batch in view picks the tiles of the level whose
//! pixel is about a device pixel (`geom::raster::level_for`, `visible`, the
//! view's centre first), draws those the atlas holds and, for the others,
//! the part of the nearest coarser tile it holds while the host makes them.
//! The quads go into one reused buffer: a raster is one draw call.
//!
//! The host makes the tiles' colours (the formats core's `Reader::render_tile`
//! on its own threads) and answers [`ImageSource::raster_tile`]; a tile it
//! does not have yet it starts making, and asks for another frame when done.

use std::collections::HashMap;
use std::hash::{Hash, Hasher};

use kentos_geometry_core::geom::raster::{DrawTile, TILE, level_for, level_sizes, visible};

use super::picture::ImageSource;

/// A slot's side: a tile and a pixel round it.
pub const SLOT: u32 = TILE + 2;

/// At most this many tiles go to the GPU in a frame (8.5 MB); the rest the next.
pub const UPLOADS_PER_FRAME: usize = 32;

/// A raster's paint as a batch carries it.
#[derive(Clone, Debug)]
pub struct RasterPaint {
    pub raster: String,
    pub look: String,
    /// Its own: the raster, its look and its pixel's size and turn hashed.
    pub key: u64,
    /// x₀ and y₀ from the batch's tile.
    pub affine: [f64; 6],
    pub width: u32,
    pub height: u32,
    pub nearest: bool,
}

impl RasterPaint {
    pub fn new(
        raster: &str,
        look: &str,
        affine: [f64; 6],
        size: [f64; 2],
        nearest: bool,
    ) -> RasterPaint {
        let mut h = std::collections::hash_map::DefaultHasher::new();
        raster.hash(&mut h);
        look.hash(&mut h);
        // The pixel's size and turn: a shaded relief's slopes are by them.
        for v in &affine[1..3] {
            v.to_bits().hash(&mut h);
        }
        for v in &affine[4..6] {
            v.to_bits().hash(&mut h);
        }
        let side = |v: f64| {
            if v.is_finite() && v >= 1.0 {
                v.min(f64::from(u32::MAX)) as u32
            } else {
                1
            }
        };
        RasterPaint {
            raster: raster.to_owned(),
            look: look.to_owned(),
            key: h.finish(),
            affine,
            width: side(size[0]),
            height: side(size[1]),
            nearest,
        }
    }
}

/// A tile's name in the atlas.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct TileId {
    raster: u64,
    level: u32,
    tx: u32,
    ty: u32,
}

/// The raster atlas: its texture and which tile each slot holds.
pub struct RasterAtlas {
    pub(crate) texture: wgpu::Texture,
    pub(crate) view: wgpu::TextureView,
    pub(crate) linear: wgpu::Sampler,
    pub(crate) nearest: wgpu::Sampler,
    width: u32,
    height: u32,
    held: HashMap<TileId, u32>,
    owner: Vec<Option<TileId>>,
    /// The frame each slot was last drawn in.
    used: Vec<u64>,
    frame: u64,
    /// Tiles placed since the view's key was last taken.
    pub(crate) placed: u64,
}

impl RasterAtlas {
    /// 8192 × 4096 where the device allows (465 slots), else 4096 × 4096 (225).
    pub fn new(device: &wgpu::Device) -> RasterAtlas {
        let wide = device.limits().max_texture_dimension_2d >= 8192;
        let (width, height) = if wide { (8192, 4096) } else { (4096, 4096) };
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("kentos.raster.atlas"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
        let sampler = |filter: wgpu::FilterMode, label: &'static str| {
            device.create_sampler(&wgpu::SamplerDescriptor {
                label: Some(label),
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                mag_filter: filter,
                min_filter: filter,
                ..wgpu::SamplerDescriptor::default()
            })
        };
        let slots = ((width / SLOT) * (height / SLOT)) as usize;
        RasterAtlas {
            texture,
            view,
            linear: sampler(wgpu::FilterMode::Linear, "kentos.raster.linear"),
            nearest: sampler(wgpu::FilterMode::Nearest, "kentos.raster.nearest"),
            width,
            height,
            held: HashMap::new(),
            owner: vec![None; slots],
            used: vec![0; slots],
            frame: 0,
            placed: 0,
        }
    }

    fn per_row(&self) -> u32 {
        self.width / SLOT
    }

    /// A slot's upper left pixel.
    fn origin(&self, slot: u32) -> (u32, u32) {
        let n = self.per_row();
        ((slot % n) * SLOT, (slot / n) * SLOT)
    }

    /// The atlas rectangle (u₀, v₀, u₁, v₁) of a slot's tile pixels
    /// (`x`, `y`) to (`x + w`, `y + h`), its apron left round them.
    fn uv(&self, slot: u32, x: f64, y: f64, w: f64, h: f64) -> [f32; 4] {
        let (ox, oy) = self.origin(slot);
        let (aw, ah) = (f64::from(self.width), f64::from(self.height));
        let u0 = (f64::from(ox) + 1.0 + x) / aw;
        let v0 = (f64::from(oy) + 1.0 + y) / ah;
        [
            u0 as f32,
            v0 as f32,
            (u0 + w / aw) as f32,
            (v0 + h / ah) as f32,
        ]
    }

    /// A slot for a new tile: a free one, else the one drawn longest ago,
    /// never one drawn this frame; none when all are.
    fn free_slot(&mut self) -> Option<u32> {
        if let Some(i) = self.owner.iter().position(Option::is_none) {
            return Some(i as u32);
        }
        let (i, &at) = self.used.iter().enumerate().min_by_key(|(_, at)| **at)?;
        if at == self.frame {
            return None;
        }
        if let Some(old) = self.owner[i].take() {
            self.held.remove(&old);
        }
        Some(i as u32)
    }

    fn touch(&mut self, slot: u32) {
        self.used[slot as usize] = self.frame;
    }

    /// Writes a tile into a slot.
    fn put(&mut self, queue: &wgpu::Queue, id: TileId, rgba: &[u8]) -> Option<u32> {
        if rgba.len() != (SLOT * SLOT * 4) as usize {
            return None;
        }
        let slot = self.free_slot()?;
        let (x, y) = self.origin(slot);
        queue.write_texture(
            wgpu::TexelCopyTextureInfo {
                texture: &self.texture,
                mip_level: 0,
                origin: wgpu::Origin3d { x, y, z: 0 },
                aspect: wgpu::TextureAspect::All,
            },
            rgba,
            wgpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(4 * SLOT),
                rows_per_image: Some(SLOT),
            },
            wgpu::Extent3d {
                width: SLOT,
                height: SLOT,
                depth_or_array_layers: 1,
            },
        );
        self.owner[slot as usize] = Some(id);
        self.held.insert(id, slot);
        self.touch(slot);
        self.placed += 1;
        Some(slot)
    }
}

/// What a view draws of its rasters this frame: the quads of every raster
/// batch, `[x, y, u, v]` six to a tile, in one list; each batch's range.
#[derive(Default)]
pub struct RasterQuads {
    pub(crate) data: Vec<f32>,
    tiles: Vec<DrawTile>,
    /// Whether a tile was asked for and not drawn this frame.
    pub(crate) waiting: bool,
}

impl RasterQuads {
    /// Starts a frame's quads.
    pub(crate) fn clear(&mut self) {
        self.data.clear();
        self.waiting = false;
    }

    /// One raster batch's quads in view (`box_`, metres from the layers'
    /// origin; `origin` its tile's), appended; the vertex range they take.
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn add(
        &mut self,
        paint: &RasterPaint,
        origin: [f64; 2],
        box_: [f64; 4],
        px_per_m: f64,
        atlas: &mut RasterAtlas,
        queue: &wgpu::Queue,
        source: &dyn ImageSource,
        uploads: &mut usize,
    ) -> std::ops::Range<u32> {
        let start = (self.data.len() / 4) as u32;
        // The affine from the layers' origin, as the box is.
        let [x0, a, b, y0, c, d] = paint.affine;
        let affine = [x0 + origin[0], a, b, y0 + origin[1], c, d];
        let sizes = level_sizes(paint.width, paint.height);
        let s = (a * d - b * c).abs().sqrt();
        let level = level_for(s, px_per_m, sizes.len());
        let mut tiles = std::mem::take(&mut self.tiles);
        visible(
            &affine,
            paint.width,
            paint.height,
            &sizes,
            level,
            box_,
            &mut tiles,
        );
        for t in &tiles {
            let id = TileId {
                raster: paint.key,
                level: t.level,
                tx: t.tx,
                ty: t.ty,
            };
            let (tw, th) = (t.uv[0] * f64::from(TILE), t.uv[1] * f64::from(TILE));
            let held = match atlas.held.get(&id).copied() {
                Some(slot) => Some(slot),
                None => match source.raster_tile(
                    &paint.raster,
                    &paint.look,
                    &paint.affine,
                    t.level,
                    t.tx,
                    t.ty,
                ) {
                    Some(rgba) if *uploads < UPLOADS_PER_FRAME => {
                        *uploads += 1;
                        atlas.put(queue, id, &rgba)
                    }
                    Some(_) => {
                        self.waiting = true;
                        None
                    }
                    None => {
                        self.waiting = true;
                        None
                    }
                },
            };
            let rect = match held {
                Some(slot) => {
                    atlas.touch(slot);
                    Some(atlas.uv(slot, 0.0, 0.0, tw, th))
                }
                // The nearest coarser tile the atlas holds, the part over this one.
                None => {
                    let mut found = None;
                    for up in 1..=(sizes.len() as u32 - 1 - t.level.min(sizes.len() as u32 - 1)) {
                        let k = t.level + up;
                        let parent = TileId {
                            raster: paint.key,
                            level: k,
                            tx: t.tx >> up,
                            ty: t.ty >> up,
                        };
                        if let Some(&slot) = atlas.held.get(&parent) {
                            let f = f64::from(1u32 << up);
                            let span = f64::from(TILE) / f;
                            let x = f64::from(t.tx & ((1 << up) - 1)) * span;
                            let y = f64::from(t.ty & ((1 << up) - 1)) * span;
                            atlas.touch(slot);
                            found = Some(atlas.uv(slot, x, y, tw / f, th / f));
                            break;
                        }
                    }
                    found
                }
            };
            let Some([u0, v0, u1, v1]) = rect else {
                continue;
            };
            // Corners: (x₀, y₀) the tile's first pixel, then along the row, then down.
            let [xs, ys] = t.corners;
            let p = |k: usize| [(xs[k] - origin[0]) as f32, (ys[k] - origin[1]) as f32];
            let (p0, p1, p2, p3) = (p(0), p(1), p(2), p(3));
            for (q, u, v) in [
                (p0, u0, v0),
                (p1, u1, v0),
                (p2, u1, v1),
                (p0, u0, v0),
                (p2, u1, v1),
                (p3, u0, v1),
            ] {
                self.data.extend_from_slice(&[q[0], q[1], u, v]);
            }
        }
        self.tiles = tiles;
        start..(self.data.len() / 4) as u32
    }
}

impl RasterAtlas {
    /// A frame begins: what is drawn now is stamped with it.
    pub(crate) fn begin_frame(&mut self) {
        self.frame += 1;
    }
}

/// The raster atlas in group 0 with a frame uniform: sampled linearly and nearest.
pub struct RasterBinds {
    pub(crate) linear: wgpu::BindGroup,
    pub(crate) nearest: wgpu::BindGroup,
}

/// The two groups for a frame uniform's buffer.
pub(crate) fn bindings(
    device: &wgpu::Device,
    layout: &wgpu::BindGroupLayout,
    frame: &wgpu::Buffer,
    atlas: &RasterAtlas,
) -> RasterBinds {
    let group = |sampler: &wgpu::Sampler, label: &'static str| {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some(label),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: frame.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas.view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    };
    RasterBinds {
        linear: group(&atlas.linear, "kentos.raster.linear"),
        nearest: group(&atlas.nearest, "kentos.raster.nearest"),
    }
}
