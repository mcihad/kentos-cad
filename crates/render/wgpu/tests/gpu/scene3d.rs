//! The 3D scene on a real GPU (docs/adr/0910 §5): tiles of `kentos-scene3d` drawn by `scene3d` into a target with a
//! reversed depth buffer and read back.
//!
//! - A 1 cm square at TM coordinates (512 345.678 E, 4 401 234.567 N, 1 234.5 m), seen from above and in perspective,
//!   covers the pixels the camera projects its corners to, within one pixel: its tile's origin minus the eye is taken
//!   in f64 (a float32 of the northing alone is 6.7 cm off, nearly seven times the square).
//! - Two 100 m squares 2 cm apart in height, seen from 1.5 km: the upper one covers every pixel (reversed float32
//!   depth; with the usual 1 − near/d the two depths are within one float32 step at that distance).
//! - A square at 1 240 m exaggerated threefold about 1 200 m lands where the camera projects its shown corners.
//! - An upload budget of one content a frame leaves the second content waiting a frame.
//! - A section's plane at TM coordinates cuts a 100 m square where the camera projects the plane's line, within one
//!   pixel; a box keeps only its inside; with no plane nothing is cut.
//! - At four samples a pixel a cut ends at the samples (docs/adr/8131): the column the plane crosses keeps the samples
//!   on its kept side, and a part whose style is not cut (a section's cap) is drawn whole.
//!
//! Also `scene3d_timing`: a 4 million triangle terrain at TM coordinates drawn through the selection, every frame.

use kentos_render_wgpu::scene3d::{Budgets, DEPTH_CLEAR, DepthTarget, Frame, Quality, Scene3dRenderer, Stats, Style};
use kentos_scene3d::{Camera, Content, Exaggeration, Limits, Projection, SourceMesh, Viewport, chunk};

use super::{FORMAT, Gpu};

const E: f64 = 512_345.678;
const N: f64 = 4_401_234.567;

/// A square from (x, y) of side `s` at height `z`, as two counterclockwise triangles.
/// A mesh's vertices and triangles.
type Mesh = (Vec<[f64; 3]>, Vec<[u32; 3]>);

fn square(x: f64, y: f64, s: f64, z: f64) -> Mesh {
    (vec![[x, y, z], [x + s, y, z], [x + s, y + s, z], [x, y + s, z]], vec![[0, 1, 2], [0, 2, 3]])
}

fn contents(meshes: &[(u32, Mesh)], first: u64) -> Vec<Content> {
    let src: Vec<SourceMesh> =
        meshes.iter().map(|(part, (v, t))| SourceMesh { part: *part, vertices: v, triangles: t, smooth: false }).collect();
    chunk(&src, Limits::default(), first).contents
}

const STYLES: [Style; 3] = [
    Style::of([0.9, 0.1, 0.1, 1.0]),
    Style::of([0.1, 0.8, 0.2, 1.0]),
    Style::of([0.1, 0.2, 0.9, 1.0]),
];

/// Draws `draws` through `camera` into a width × height target cleared white, read back as RGBA.
fn draw(gpu: &Gpu, r: &mut Scene3dRenderer, camera: &Camera, ex: Exaggeration, draws: &[&Content], size: (u32, u32)) -> (Vec<u8>, Stats) {
    draw_lit(gpu, r, camera, ex, draws, size, (&STYLES, [0.0, 0.0, 1.0], 1.0))
}

/// As `draw`, cut by the half-spaces `clip`.
fn draw_cut(gpu: &Gpu, r: &mut Scene3dRenderer, camera: &Camera, draws: &[&Content], size: (u32, u32), clip: &[[f64; 4]]) -> Vec<u8> {
    CLIP.with(|c| *c.borrow_mut() = clip.to_vec());
    let out = draw(gpu, r, camera, Exaggeration::NONE, draws, size).0;
    CLIP.with(|c| c.borrow_mut().clear());
    out
}

thread_local! {
    /// The planes the next draw is cut by.
    static CLIP: std::cell::RefCell<Vec<[f64; 4]>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// As `draw`, with the parts' styles, the light and its ambient share.
fn draw_lit(
    gpu: &Gpu,
    r: &mut Scene3dRenderer,
    camera: &Camera,
    ex: Exaggeration,
    draws: &[&Content],
    size: (u32, u32),
    look: (&[Style], [f64; 3], f32),
) -> (Vec<u8>, Stats) {
    let (width, height) = size;
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene3d target"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut depth = None;
    let depth = DepthTarget::ensure(&mut depth, &gpu.device, width, height);
    let clip = CLIP.with(|c| c.borrow().clone());
    let frame = Frame {
        camera,
        exaggeration: ex,
        viewport: Viewport { width: f64::from(width), height: f64::from(height) },
        draws,
        styles: look.0,
        light: look.1,
        ambient: look.2,
        sky: 0.0,
        fill: 0.0,
        clip: &clip,
    };
    let stats = r.prepare(&gpu.device, &gpu.queue, &frame);
    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                view: depth,
                depth_ops: Some(wgpu::Operations { load: wgpu::LoadOp::Clear(DEPTH_CLEAR), store: wgpu::StoreOp::Discard }),
                stencil_ops: None,
            }),
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        r.render(&mut pass);
    }
    let row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row * height),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: None },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    let index = gpu.queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    gpu.device
        .poll(wgpu::PollType::Wait { submission_index: Some(index), timeout: None })
        .expect("the frame finishes");
    let data = slice.get_mapped_range();
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let start = (y * row) as usize;
        out.extend_from_slice(&data[start..start + (width * 4) as usize]);
    }
    (out, stats)
}

/// The box of the pixels not white: (x0, y0, x1, y1), x1 and y1 past the last.
fn covered(px: &[u8], width: u32) -> Option<(u32, u32, u32, u32)> {
    let mut b: Option<(u32, u32, u32, u32)> = None;
    for (i, p) in px.as_chunks::<4>().0.iter().enumerate() {
        if p[0] < 250 || p[1] < 250 || p[2] < 250 {
            let (x, y) = (i as u32 % width, i as u32 / width);
            b = Some(match b {
                None => (x, y, x + 1, y + 1),
                Some((a, c, d, e)) => (a.min(x), c.min(y), d.max(x + 1), e.max(y + 1)),
            });
        }
    }
    b
}

/// The box the camera projects the shown corners to.
fn projected(camera: &Camera, vp: &Viewport, corners: &[[f64; 3]]) -> (f64, f64, f64, f64) {
    let ps: Vec<(f64, f64)> = corners.iter().map(|c| camera.project(*c, vp).map(|p| (p.0, p.1)).expect("in front")).collect();
    let x0 = ps.iter().map(|p| p.0).fold(f64::INFINITY, f64::min);
    let x1 = ps.iter().map(|p| p.0).fold(f64::NEG_INFINITY, f64::max);
    let y0 = ps.iter().map(|p| p.1).fold(f64::INFINITY, f64::min);
    let y1 = ps.iter().map(|p| p.1).fold(f64::NEG_INFINITY, f64::max);
    (x0, y0, x1, y1)
}

fn within_a_pixel(got: (u32, u32, u32, u32), want: (f64, f64, f64, f64), what: &str) {
    let g = [got.0, got.1, got.2, got.3].map(f64::from);
    let w = [want.0, want.1, want.2, want.3];
    for k in 0..4 {
        assert!((g[k] - w[k]).abs() <= 1.0, "{what}: covered {got:?}, projected {want:?}");
    }
}

pub(crate) fn the_3d_scene_draws_precisely(gpu: &mut Gpu) {
    let mut r = Scene3dRenderer::new(&gpu.device, FORMAT);
    let size = (256u32, 256u32);
    let vp = Viewport { width: 256.0, height: 256.0 };
    // The northing alone as float32 is 6.7 cm off: nearly seven times the square.
    assert!(((N as f32) as f64 - N).abs() > 0.06);
    let sq = square(E, N, 0.01, 1234.5);
    let c = contents(&[(0, sq.clone())], 1);
    let draws: Vec<&Content> = c.iter().collect();
    let top = Camera {
        eye: [E + 0.005, N + 0.005, 1244.5],
        target: [E + 0.005, N + 0.005, 1234.5],
        up: [0.0, 0.0, 1.0],
        projection: Projection::Orthographic { height: 0.04, depth: 100.0 },
    };
    let (px, _) = draw(gpu, &mut r, &top, Exaggeration::NONE, &draws, size);
    within_a_pixel(covered(&px, 256).expect("drawn"), projected(&top, &vp, &sq.0), "1 cm from above");
    let slant = Camera::orbit([E + 0.005, N + 0.005, 1234.5], 30.0, 40.0, 0.06, Projection::Perspective { fov_y: 40f64.to_radians(), near: 0.001 });
    let (px, _) = draw(gpu, &mut r, &slant, Exaggeration::NONE, &draws, size);
    within_a_pixel(covered(&px, 256).expect("drawn"), projected(&slant, &vp, &sq.0), "1 cm in perspective");

    // Two squares 2 cm apart seen from 1.5 km: the upper (green) everywhere.
    let lower = square(E - 50.0, N - 50.0, 100.0, 1000.0);
    let upper = square(E - 50.0, N - 50.0, 100.0, 1000.02);
    let c = contents(&[(0, lower), (1, upper)], 10);
    let draws: Vec<&Content> = c.iter().collect();
    let far = Camera::orbit([E, N, 1000.0], 60.0, 45.0, 1500.0, Projection::Perspective { fov_y: 40f64.to_radians(), near: 0.5 });
    let (px, _) = draw(gpu, &mut r, &far, Exaggeration::NONE, &draws, size);
    let (mut green, mut red) = (0, 0);
    for p in px.as_chunks::<4>().0.iter() {
        if p[1] > 150 && p[0] < 80 {
            green += 1;
        } else if p[0] > 150 && p[1] < 80 {
            red += 1;
        }
    }
    assert!(green > 200 && red == 0, "green {green}, red {red}");

    // Exaggerated threefold about 1 200 m: a square at 1 240 m is shown at 1 320 m.
    let ex = Exaggeration { factor: 3.0, datum: 1200.0 };
    let sq = square(E, N, 2.0, 1240.0);
    let c = contents(&[(2, sq.clone())], 20);
    let draws: Vec<&Content> = c.iter().collect();
    let cam = Camera::orbit([E + 1.0, N + 1.0, 1320.0], 200.0, 35.0, 12.0, Projection::Perspective { fov_y: 40f64.to_radians(), near: 0.5 });
    let (px, _) = draw(gpu, &mut r, &cam, ex, &draws, size);
    let shown: Vec<[f64; 3]> = sq.0.iter().map(|p| ex.shown(*p)).collect();
    within_a_pixel(covered(&px, 256).expect("drawn"), projected(&cam, &vp, &shown), "exaggerated");

    // One content a frame: the second waits a frame.
    let mut r = Scene3dRenderer::new(&gpu.device, FORMAT);
    let c = contents(&[(0, square(E, N, 0.01, 1234.5)), (1, square(E + 5000.0, N, 0.01, 1234.5))], 30);
    let draws: Vec<&Content> = c.iter().collect();
    assert_eq!(c.len(), 2, "two tiles 5 km apart");
    r.budgets = Budgets { resident: 1 << 20, upload: 1 };
    let (_, first) = draw(gpu, &mut r, &top, Exaggeration::NONE, &draws, size);
    assert_eq!((first.tiles, first.waiting), (1, 1));
    let (_, second) = draw(gpu, &mut r, &top, Exaggeration::NONE, &draws, size);
    assert_eq!((second.tiles, second.waiting, second.resident_contents), (2, 0, 2));

    // A section: a 100 m square at TM coordinates seen from above, cut along the north–south line 20 m east of its
    // middle (the west side kept), then by a box about its south-west quarter.
    let mut r = Scene3dRenderer::new(&gpu.device, FORMAT);
    let sq = square(E - 50.0, N - 50.0, 100.0, 1234.5);
    let c = contents(&[(0, sq.clone())], 40);
    let draws: Vec<&Content> = c.iter().collect();
    let above = Camera {
        eye: [E, N, 1334.5],
        target: [E, N, 1234.5],
        up: [0.0, 1.0, 0.0],
        projection: Projection::Orthographic { height: 128.0, depth: 400.0 },
    };
    let whole = covered(&draw_cut(gpu, &mut r, &above, &draws, size, &[]), 256).expect("drawn");
    within_a_pixel(whole, projected(&above, &vp, &sq.0), "nothing cut");
    let west = covered(&draw_cut(gpu, &mut r, &above, &draws, size, &[[-1.0, 0.0, 0.0, E + 20.0]]), 256).expect("drawn");
    let kept = [[E - 50.0, N - 50.0, 1234.5], [E + 20.0, N + 50.0, 1234.5]];
    within_a_pixel(west, projected(&above, &vp, &kept), "west of the plane");
    let quarter = [
        [1.0, 0.0, 0.0, -(E - 50.0)],
        [-1.0, 0.0, 0.0, E],
        [0.0, 1.0, 0.0, -(N - 50.0)],
        [0.0, -1.0, 0.0, N],
        [0.0, 0.0, 1.0, -1234.0],
        [0.0, 0.0, -1.0, 1235.0],
    ];
    let boxed = covered(&draw_cut(gpu, &mut r, &above, &draws, size, &quarter), 256).expect("drawn");
    within_a_pixel(boxed, projected(&above, &vp, &[[E - 50.0, N - 50.0, 1234.5], [E, N, 1234.5]]), "inside the box");
}

/// A box of `side` from `at` (each face two triangles with vertices of its own, as a flat body's are).
fn block(at: [f64; 3], side: [f64; 3]) -> Mesh {
    let c = |i: usize| [0, 1, 2].map(|k| at[k] + if (i >> k) & 1 == 0 { 0.0 } else { side[k] });
    let faces = [[0, 1, 3, 2], [4, 6, 7, 5], [0, 4, 5, 1], [2, 3, 7, 6], [0, 2, 6, 4], [1, 5, 7, 3]];
    let (mut v, mut t) = (Vec::new(), Vec::new());
    for f in faces {
        let k = v.len() as u32;
        v.extend(f.map(c));
        t.extend([[k, k + 1, k + 2], [k, k + 2, k + 3]]);
    }
    (v, t)
}

/// Draws `draws` at the renderer's quality into its own targets laid over a width × height target cleared white,
/// read back as RGBA; every face its style's colour (the light's ambient share whole), cut by the section's `clip`.
fn draw_offscreen(
    gpu: &Gpu,
    r: &mut Scene3dRenderer,
    camera: &Camera,
    draws: &[&Content],
    styles: &[Style],
    size: (u32, u32),
    clip: &[[f64; 4]],
) -> Vec<u8> {
    let (width, height) = size;
    let texture = gpu.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("scene3d output"),
        size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let view = texture.create_view(&wgpu::TextureViewDescriptor::default());
    r.ensure_targets(&gpu.device, width, height);
    let frame = Frame {
        camera,
        exaggeration: Exaggeration::NONE,
        viewport: Viewport { width: f64::from(width), height: f64::from(height) },
        draws,
        styles,
        light: [0.0, 0.0, 1.0],
        ambient: 1.0,
        sky: 0.0,
        fill: 0.0,
        clip,
    };
    r.prepare(&gpu.device, &gpu.queue, &frame);
    r.place(&gpu.queue, [0.0, 0.0, 1.0, 1.0]);
    let mut encoder = gpu.device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let _clear = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &view,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations { load: wgpu::LoadOp::Clear(wgpu::Color::WHITE), store: wgpu::StoreOp::Store },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
    }
    r.render_offscreen(&mut encoder, &view, [0.0, 0.0, width as f32, height as f32], [0, 0, width, height]);
    let row = (width * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = gpu.device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row * height),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: None },
        },
        wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
    );
    let index = gpu.queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    gpu.device.poll(wgpu::PollType::Wait { submission_index: Some(index), timeout: None }).expect("the frame finishes");
    let data = slice.get_mapped_range();
    let mut out = Vec::with_capacity((width * height * 4) as usize);
    for y in 0..height {
        let start = (y * row) as usize;
        out.extend_from_slice(&data[start..start + (width * 4) as usize]);
    }
    out
}

/// A picture for the owner: `.run/gui-evidence/3b-kalite-8120/gpu-<name>.png`.
fn keep(px: &[u8], size: (u32, u32), name: &str) {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("../../../.run/gui-evidence/3b-kalite-8120/gpu-{name}.png"));
    let _ = std::fs::create_dir_all(out.parent().expect("a folder"));
    let file = std::io::BufWriter::new(std::fs::File::create(&out).expect("a file"));
    let mut png = png::Encoder::new(file, size.0, size.1);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header().expect("a header").write_image_data(px).expect("writes the picture");
}

/// The qualities (docs/adr/8120) on a 1 m box standing on a 4 m slab, seen from the north-east, every face its style's
/// grey: Hızlı draws only the two greys and the white (no edge, no pixel between them); Dengeli draws the box's edges
/// dark and its outline smooth (pixels between the greys, four samples a pixel); Kaliteli darkens the slab beside the
/// box's foot, not the slab away from it, by the ambient light's occlusion.
pub(crate) fn the_qualities_draw_edges_smooth_outlines_and_occlusion(gpu: &mut Gpu) {
    let size = (256u32, 256u32);
    let vp = Viewport { width: 256.0, height: 256.0 };
    let styles = [Style::of([0.9, 0.9, 0.9, 1.0]), Style::of([0.7, 0.7, 0.7, 1.0])];
    let c = contents(&[(0, block([0.0, 0.0, 0.0], [1.0, 1.0, 1.0])), (1, block([-1.5, -1.5, -0.1], [4.0, 4.0, 0.1]))], 60);
    let draws: Vec<&Content> = c.iter().collect();
    let camera = Camera::orbit([0.5, 0.5, 0.3], 30.0, 32.0, 4.2, Projection::Perspective { fov_y: 40f64.to_radians(), near: 0.05 });
    let at = |p: [f64; 3]| {
        let (x, y, _) = camera.project(p, &vp).expect("in front");
        (x as usize, y as usize)
    };
    let value = |px: &[u8], (x, y): (usize, usize)| px[(y * 256 + x) * 4];
    let (box_grey, slab_grey) = ((0.9f64 * 255.0).round() as u8, (0.7f64 * 255.0).round() as u8);
    // How many pixels are darker than either grey (an edge), and how many lie between the greys and the white.
    let count = |px: &[u8]| {
        let (mut dark, mut between) = (0, 0);
        for p in px.as_chunks::<4>().0.iter() {
            let v = p[0];
            if v < slab_grey.saturating_sub(25) {
                dark += 1;
            } else if v.abs_diff(box_grey) > 2 && v.abs_diff(slab_grey) > 2 && v < 253 {
                between += 1;
            }
        }
        (dark, between)
    };
    let mut fast = Scene3dRenderer::with_quality(&gpu.device, FORMAT, Quality::Fast);
    let px = draw_offscreen(gpu, &mut fast, &camera, &draws, &styles, size, &[]);
    keep(&px, size, "hizli");
    assert_eq!(count(&px), (0, 0), "Hızlı: the faces alone, one sample a pixel");
    let mut balanced = Scene3dRenderer::with_quality(&gpu.device, FORMAT, Quality::Balanced);
    let px = draw_offscreen(gpu, &mut balanced, &camera, &draws, &styles, size, &[]);
    keep(&px, size, "dengeli");
    let (dark, between) = count(&px);
    assert!(dark > 150, "Dengeli: the box's edges drawn ({dark} dark pixels)");
    assert!(between > 100, "Dengeli: the outlines smooth ({between} pixels between)");
    // The slab beside the box's east foot and away from it.
    let (near, far) = (at([1.08, 0.5, 0.0]), at([2.2, -0.9, 0.0]));
    // The GPU writes 178.5 as either of its neighbours.
    let grey = |v: u8| v.abs_diff(slab_grey) <= 1;
    assert!(grey(value(&px, near)) && grey(value(&px, far)), "Dengeli: no occlusion ({}, {})", value(&px, near), value(&px, far));
    let mut high = Scene3dRenderer::with_quality(&gpu.device, FORMAT, Quality::High);
    let px = draw_offscreen(gpu, &mut high, &camera, &draws, &styles, size, &[]);
    keep(&px, size, "kaliteli");
    let (near, far) = (value(&px, near), value(&px, far));
    assert!(grey(far) && near + 12 < far, "Kaliteli: the foot occluded ({near}), the slab away not ({far})");
}

/// A 4 million triangle terrain at TM coordinates through the selection: the frames' CPU and GPU time.
pub(crate) fn timing(gpu: &mut Gpu) {
    use kentos_scene3d::{Exaggeration, Settings, select};
    use std::time::Instant;
    let (nx, ny) = (2000usize, 1000usize);
    let mut vs = Vec::with_capacity((nx + 1) * (ny + 1));
    for j in 0..=ny {
        for i in 0..=nx {
            let (x, y) = (i as f64, j as f64);
            vs.push([E + x, N + y, 1200.0 + 0.02 * x + 6.0 * (y / 90.0).sin()]);
        }
    }
    let mut ts = Vec::with_capacity(nx * ny * 2);
    let w = (nx + 1) as u32;
    for j in 0..ny {
        for i in 0..nx {
            let a = (j * (nx + 1) + i) as u32;
            ts.push([a, a + 1, a + w + 1]);
            ts.push([a, a + w + 1, a + w]);
        }
    }
    let c = chunk(&[SourceMesh { part: 0, vertices: &vs, triangles: &ts, smooth: true }], Limits::default(), 1);
    let mut r = Scene3dRenderer::new(&gpu.device, FORMAT);
    let size = (1920u32, 1080u32);
    let vp = Viewport { width: 1920.0, height: 1080.0 };
    let by_id: std::collections::HashMap<u64, &Content> = c.contents.iter().map(|k| (k.id, k)).collect();
    let mut frames = Vec::new();
    for k in 0..30 {
        let cam = Camera::orbit([E + 200.0 + 50.0 * f64::from(k), N + 500.0, 1220.0], 200.0 + 3.0 * f64::from(k), 30.0, 900.0,
            Projection::Perspective { fov_y: 40f64.to_radians(), near: 0.5 });
        let t = Instant::now();
        let s = select(&c.hierarchy, &cam, &vp, &Exaggeration::NONE, Settings { max_screen_error: 16.0 }, |_| true);
        let mut draws: Vec<&Content> = Vec::new();
        for i in s.draw {
            if let Some(cr) = c.hierarchy.tiles[i as usize].content {
                draws.push(by_id[&cr.id]);
            }
        }
        let cpu = t.elapsed();
        let t = Instant::now();
        let (_, stats) = draw(gpu, &mut r, &cam, Exaggeration::NONE, &draws, size);
        frames.push((cpu.as_secs_f64() * 1e3, t.elapsed().as_secs_f64() * 1e3, stats));
    }
    // The picture of the last view, lit, for the owner: .run/gui-evidence/yol-3b-cizici.png.
    let cam = Camera::orbit([E + 1000.0, N + 380.0, 1225.0], 235.0, 24.0, 820.0, Projection::Perspective { fov_y: 40f64.to_radians(), near: 0.5 });
    let s = select(&c.hierarchy, &cam, &vp, &Exaggeration { factor: 4.0, datum: 1200.0 }, Settings { max_screen_error: 16.0 }, |_| true);
    let draws: Vec<&Content> = s.draw.iter().filter_map(|&i| c.hierarchy.tiles[i as usize].content).map(|cr| by_id[&cr.id]).collect();
    let terrain = [Style::of([0.47, 0.55, 0.33, 1.0])];
    let (px, _) = draw_lit(gpu, &mut r, &cam, Exaggeration { factor: 4.0, datum: 1200.0 }, &draws, size, (&terrain, [-0.45, -0.35, 0.82], 0.35));
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.run/gui-evidence/yol-3b-cizici.png");
    let _ = std::fs::create_dir_all(out.parent().expect("a folder"));
    let file = std::io::BufWriter::new(std::fs::File::create(&out).expect("a file"));
    let mut png = png::Encoder::new(file, size.0, size.1);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header().expect("a header").write_image_data(&px).expect("writes the picture");
    println!("{}", out.display());
    let first = frames[0];
    let rest: Vec<f64> = frames[1..].iter().map(|f| f.1).collect();
    let mut sorted = rest.clone();
    sorted.sort_by(f64::total_cmp);
    println!(
        "ilk kare: seçim {:.3} ms, yükleme ve çizim {:.1} ms ({} karo, {} üçgen, {:.1} MB yüklendi)",
        first.0, first.1, first.2.tiles, first.2.triangles, first.2.uploaded as f64 / 1e6
    );
    println!(
        "sonraki 29 kare (1920×1080, geri okuma dahil): ortanca {:.1} ms, en kötü {:.1} ms, kare başına {} karo, {} üçgen",
        sorted[sorted.len() / 2],
        sorted[sorted.len() - 1],
        frames[29].2.tiles,
        frames[29].2.triangles
    );
}

/// A section's cut at its samples (docs/adr/8131): an 8 m square seen from above, 256 pixels across (3.125 cm a
/// pixel), cut by a north–south plane a quarter of a pixel east of the middle of pixel column 128, the west kept.
/// Hızlı keeps that column whole (its one sample, the pixel's middle, is kept) and the next one not at all; Dengeli
/// keeps three of the column's four samples (the standard pattern's places 3/8, 7/8, 1/8, 5/8 across the pixel, the
/// plane at 3/4), so it is three quarters grey, not a whole pixel past the plane or short of it. A part whose style is
/// not cut (a cap) east of the plane is drawn whole.
pub(crate) fn a_cut_ends_at_its_samples_and_a_cap_is_not_cut(gpu: &mut Gpu) {
    let size = (256u32, 256u32);
    let pixel = 8.0 / 256.0;
    let (cut_at, cap_from) = (0.75 * pixel, 1.0);
    let styles = [Style::of([0.5, 0.5, 0.5, 1.0]), Style { color: [0.2, 0.2, 0.2, 1.0], cut: false }];
    let c = contents(&[(0, square(-2.0, -2.0, 4.0, 0.0)), (1, square(cap_from, -0.5, 0.5, 0.1))], 80);
    let draws: Vec<&Content> = c.iter().collect();
    let camera = Camera {
        eye: [0.0, 0.0, 10.0],
        target: [0.0, 0.0, 0.0],
        up: [0.0, 1.0, 0.0],
        projection: Projection::Orthographic { height: 8.0, depth: 40.0 },
    };
    let west = [[-1.0, 0.0, 0.0, cut_at]];
    let value = |px: &[u8], x: usize, y: usize| px[(y * 256 + x) * 4];
    let grey = (0.5f64 * 255.0).round() as u8;
    let near = |v: u8, want: u8| v.abs_diff(want) <= 2;
    let mut fast = Scene3dRenderer::with_quality(&gpu.device, FORMAT, Quality::Fast);
    let px = draw_offscreen(gpu, &mut fast, &camera, &draws, &styles, size, &west);
    assert!(near(value(&px, 127, 100), grey) && near(value(&px, 128, 100), grey), "Hızlı: the column whose middle is kept is kept whole");
    assert_eq!(value(&px, 129, 100), 255, "Hızlı: the next column cut");
    let mut balanced = Scene3dRenderer::with_quality(&gpu.device, FORMAT, Quality::Balanced);
    let px = draw_offscreen(gpu, &mut balanced, &camera, &draws, &styles, size, &west);
    let three_quarters = (0.75 * 127.5f64 + 0.25 * 255.0).round() as u8;
    assert!(near(value(&px, 127, 100), grey), "Dengeli: west of the plane kept ({})", value(&px, 127, 100));
    assert!(
        near(value(&px, 128, 100), three_quarters),
        "Dengeli: three of the column's four samples kept ({}, want {three_quarters})",
        value(&px, 128, 100)
    );
    assert_eq!(value(&px, 129, 100), 255, "Dengeli: the next column cut");
    // The cap's square, x 1 to 1.5 (pixels 160 to 176), y −0.5 to 0 (rows 128 to 144): east of the plane, drawn.
    let dark = (0.2f64 * 255.0).round() as u8;
    assert!(near(value(&px, 168, 136), dark), "a cap's part is not cut ({})", value(&px, 168, 136));
}
