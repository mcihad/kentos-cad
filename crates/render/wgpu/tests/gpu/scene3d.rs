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
//!
//! Also `scene3d_timing`: a 4 million triangle terrain at TM coordinates drawn through the selection, every frame.

use kentos_render_wgpu::scene3d::{Budgets, DEPTH_CLEAR, DepthTarget, Frame, Scene3dRenderer, Stats, Style};
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
        meshes.iter().map(|(part, (v, t))| SourceMesh { part: *part, vertices: v, triangles: t }).collect();
    chunk(&src, Limits::default(), first).contents
}

const STYLES: [Style; 3] = [
    Style { color: [0.9, 0.1, 0.1, 1.0] },
    Style { color: [0.1, 0.8, 0.2, 1.0] },
    Style { color: [0.1, 0.2, 0.9, 1.0] },
];

/// Draws `draws` through `camera` into a width × height target cleared white, read back as RGBA.
fn draw(gpu: &Gpu, r: &mut Scene3dRenderer, camera: &Camera, ex: Exaggeration, draws: &[&Content], size: (u32, u32)) -> (Vec<u8>, Stats) {
    draw_lit(gpu, r, camera, ex, draws, size, (&STYLES, [0.0, 0.0, 1.0], 1.0))
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
    let frame = Frame {
        camera,
        exaggeration: ex,
        viewport: Viewport { width: f64::from(width), height: f64::from(height) },
        draws,
        styles: look.0,
        light: look.1,
        ambient: look.2,
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
    for (i, p) in px.chunks_exact(4).enumerate() {
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
    for p in px.chunks_exact(4) {
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
    let c = chunk(&[SourceMesh { part: 0, vertices: &vs, triangles: &ts }], Limits::default(), 1);
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
    let terrain = [Style { color: [0.47, 0.55, 0.33, 1.0] }];
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
