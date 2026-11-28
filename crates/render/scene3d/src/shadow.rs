//! The sun's shadows (docs/adr/8177 §4): the opaque faces drawn once more, from the sun, into a depth map whose
//! orthographic projection is fitted to the box of what casts them as the sun sees it; the faces' pass then tests each
//! sample against it (`sunlit` in `mesh.wgsl`), twenty-five comparisons a pixel, each filtered over its four texels, so
//! that a shadow's edge is soft over about two texels and never stair-stepped.
//!
//! - **Space:** as the faces' pass, relative to the eye in the shown space (the tiles' offsets are the eye's); the
//!   projection is made in f64 and only then made float32.
//! - **Depth:** standard (near the sun 0, far 1), cleared to 1, compared less; a sample is lit where it is no deeper
//!   than the map, less a bias of a texel and moved off its face along its normal by up to two texels, more as the sun
//!   grazes it: without, a face shadows itself in stripes (acne); with too much, a shadow leaves its caster's foot.
//! - **What casts:** the opaque faces the frame draws (a see-through part, glass or a room, casts none); a section's
//!   planes cut the casters as they cut the faces, so what the section takes away casts no shadow either.
//! - **Size:** 4 096 texels a side (2 048 at Hızlı): a 30 m building's shadow is drawn to 7 mm a texel.

use kentos_scene3d::math;

use crate::{DEPTH_FORMAT, Quality};

/// The map's side in texels at `quality`.
pub fn side(quality: Quality) -> u32 {
    if quality == Quality::Fast { 2048 } else { 4096 }
}

/// The light's projection of a frame: the matrix from the eye's space to the map's (x, y in −1…1, depth in 0…1),
/// column-major, and a texel's size in the map's coordinates and in metres, and the depth bias.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Projection {
    pub clip: [f32; 16],
    pub texel_uv: f32,
    pub texel_m: f32,
    pub bias: f32,
}

/// The light's projection of the box `low`–`high` (the shown space, the eye at `eye`) for a sun towards `towards` (unit,
/// shown space) and a map of `side` texels: the box's corners as the sun sees them bound it, padded by a texel, and
/// their shadows on its floor (the plane at `low`'s height): those lie along the rays, deeper than any corner, and a
/// ground's shadow beyond the box's depth would read as lit.
pub fn projection(low: [f64; 3], high: [f64; 3], eye: [f64; 3], towards: [f64; 3], side: u32) -> Projection {
    // The sun's frame: forward from the sun into the scene, right and up across it.
    let forward = math::normalize([-towards[0], -towards[1], -towards[2]]);
    let up = if forward[2].abs() < 0.99 { [0.0, 0.0, 1.0] } else { [0.0, 1.0, 0.0] };
    let right = math::normalize(cross(forward, up));
    let up = cross(right, forward);
    let (mut lo, mut hi) = ([f64::MAX; 3], [f64::MIN; 3]);
    for k in 0..16 {
        let mut corner = [
            if k & 1 == 0 { low[0] } else { high[0] } - eye[0],
            if k & 2 == 0 { low[1] } else { high[1] } - eye[1],
            if k & 4 == 0 { low[2] } else { high[2] } - eye[2],
        ];
        // The corner's shadow on the floor, down the ray (a sun over the horizon).
        if k >= 8 {
            if towards[2] <= 1e-3 {
                continue;
            }
            let down = (corner[2] + eye[2] - low[2]) / towards[2];
            corner = [corner[0] - towards[0] * down, corner[1] - towards[1] * down, corner[2] - towards[2] * down];
        }
        let at = [dot(corner, right), dot(corner, up), dot(corner, forward)];
        for i in 0..3 {
            lo[i] = lo[i].min(at[i]);
            hi[i] = hi[i].max(at[i]);
        }
    }
    let width = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-3);
    let texel = width / f64::from(side.max(1));
    let (half, middle) = (width / 2.0 + texel, [(lo[0] + hi[0]) / 2.0, (lo[1] + hi[1]) / 2.0]);
    let (near, depth) = (lo[2] - texel, (hi[2] - lo[2]) + 2.0 * texel);
    // Rows: x' = (p·right − middle.x) / half, y' likewise, z' = (p·forward − near) / depth.
    let rows = [
        [right[0] / half, right[1] / half, right[2] / half, -middle[0] / half],
        [up[0] / half, up[1] / half, up[2] / half, -middle[1] / half],
        [forward[0] / depth, forward[1] / depth, forward[2] / depth, -near / depth],
        [0.0, 0.0, 0.0, 1.0],
    ];
    let mut clip = [0.0f32; 16];
    for (col, out) in clip.chunks_mut(4).enumerate() {
        for (row, v) in out.iter_mut().enumerate() {
            *v = rows[row][col] as f32;
        }
    }
    let size = 2.0 * half;
    Projection {
        clip,
        texel_uv: (1.0 / f64::from(side.max(1))) as f32,
        texel_m: (size / f64::from(side.max(1))) as f32,
        bias: (size / f64::from(side.max(1)) / depth) as f32,
    }
}

fn cross(a: [f64; 3], b: [f64; 3]) -> [f64; 3] {
    [a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]]
}

fn dot(a: [f64; 3], b: [f64; 3]) -> f64 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

/// The map, the sampler that compares against it, and the pipeline that draws it.
pub(crate) struct Shadows {
    pub(crate) pipeline: wgpu::RenderPipeline,
    pub(crate) layout: wgpu::BindGroupLayout,
    pub(crate) sampler: wgpu::Sampler,
    /// A 1 × 1 map bound while no frame casts shadows: the faces do not read it then (`sun.x` is 0).
    pub(crate) none: wgpu::TextureView,
    /// The map, made the first time a frame casts shadows.
    pub(crate) map: Option<wgpu::TextureView>,
    pub(crate) side: u32,
}

impl Shadows {
    /// The pipeline (the faces' vertex stage moved to the sun's projection; a fragment stage that only leaves out what a
    /// section cuts away) on `module`, its view group `layout` (the view and the styles: the map it draws is not bound
    /// while it draws it).
    pub(crate) fn new(
        device: &wgpu::Device,
        module: &wgpu::ShaderModule,
        tile_layout: &wgpu::BindGroupLayout,
        layout: wgpu::BindGroupLayout,
        quality: Quality,
    ) -> Self {
        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("kentos.scene3d.shadow"),
            bind_group_layouts: &[&layout, tile_layout],
            push_constant_ranges: &[],
        });
        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("kentos.scene3d.shadow"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module,
                entry_point: Some("shadowVs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                buffers: &[crate::vertex_layout()],
            },
            primitive: wgpu::PrimitiveState { cull_mode: None, ..wgpu::PrimitiveState::default() },
            depth_stencil: Some(wgpu::DepthStencilState {
                format: DEPTH_FORMAT,
                depth_write_enabled: true,
                depth_compare: wgpu::CompareFunction::Less,
                stencil: wgpu::StencilState::default(),
                bias: wgpu::DepthBiasState::default(),
            }),
            multisample: wgpu::MultisampleState::default(),
            fragment: Some(wgpu::FragmentState {
                module,
                entry_point: Some("shadowFs"),
                compilation_options: wgpu::PipelineCompilationOptions::default(),
                targets: &[],
            }),
            multiview: None,
            cache: None,
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("kentos.scene3d.shadow"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            compare: Some(wgpu::CompareFunction::LessEqual),
            ..wgpu::SamplerDescriptor::default()
        });
        let none = map_texture(device, 1);
        Shadows {
            pipeline,
            layout,
            sampler,
            none: none.create_view(&wgpu::TextureViewDescriptor::default()),
            map: None,
            side: side(quality),
        }
    }

    /// The map's view, made the first time; whether it was made now (the faces' view group is then made again).
    pub(crate) fn ensure(&mut self, device: &wgpu::Device) -> bool {
        if self.map.is_some() {
            return false;
        }
        self.map = Some(map_texture(device, self.side).create_view(&wgpu::TextureViewDescriptor::default()));
        true
    }

    /// The map the faces read: the map once made, else the one nothing is in the shadow of.
    pub(crate) fn bound(&self) -> &wgpu::TextureView {
        self.map.as_ref().unwrap_or(&self.none)
    }
}

/// A depth map of `side` × `side` texels, drawn into and sampled.
fn map_texture(device: &wgpu::Device, side: u32) -> wgpu::Texture {
    device.create_texture(&wgpu::TextureDescriptor {
        label: Some("kentos.scene3d.shadowMap"),
        size: wgpu::Extent3d { width: side, height: side, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: DEPTH_FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
        view_formats: &[],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn apply(m: &[f32; 16], p: [f64; 3]) -> [f64; 3] {
        let at = |row: usize| {
            f64::from(m[row]) * p[0] + f64::from(m[4 + row]) * p[1] + f64::from(m[8 + row]) * p[2] + f64::from(m[12 + row])
        };
        [at(0), at(1), at(2)]
    }

    #[test]
    fn the_box_fills_the_map_and_its_depth_runs_away_from_the_sun() {
        // A 10 × 10 × 10 m box, the eye 50 m south; the sun straight up.
        let (low, high, eye) = ([0.0, 0.0, 0.0], [10.0, 10.0, 10.0], [5.0, -50.0, 5.0]);
        let p = projection(low, high, eye, [0.0, 0.0, 1.0], 4096);
        let rel = |q: [f64; 3]| [q[0] - eye[0], q[1] - eye[1], q[2] - eye[2]];
        let top = apply(&p.clip, rel([5.0, 5.0, 10.0]));
        let bottom = apply(&p.clip, rel([5.0, 5.0, 0.0]));
        assert!(top[0].abs() < 1e-6 && top[1].abs() < 1e-6, "the middle: {top:?}");
        assert!(top[2] < bottom[2] && top[2] >= 0.0 && bottom[2] <= 1.0, "nearer the sun, shallower: {top:?} {bottom:?}");
        let corner = apply(&p.clip, rel([0.0, 0.0, 0.0]));
        assert!(corner[0].abs() < 1.0 && corner[0].abs() > 0.99, "a corner at the map's edge less a texel: {corner:?}");
        assert!((f64::from(p.texel_m) - 10.0 / 4096.0).abs() < 1e-5, "{}", p.texel_m);
        // A low sun from the south: the box's top is nearer the sun than its foot on the north side.
        let s = projection(low, high, eye, math::normalize([0.0, -1.0, 0.3]), 4096);
        let top_south = apply(&s.clip, rel([5.0, 0.0, 10.0]));
        let foot_north = apply(&s.clip, rel([5.0, 10.0, 0.0]));
        assert!(top_south[2] < foot_north[2], "{top_south:?} {foot_north:?}");
        // The top's shadow on the floor, 33 m north of the box: inside the map, its depth within 0…1.
        let far = apply(&s.clip, rel([5.0, 10.0 + 10.0 / 0.3 * 0.9578, 0.0]));
        assert!(far[2] <= 1.0 && far[2] > foot_north[2] && far[1].abs() <= 1.0, "{far:?}");
    }
}
