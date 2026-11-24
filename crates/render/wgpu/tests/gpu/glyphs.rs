//! The drawing's glyphs on a real GPU (docs/adr/9212 §4, stage 1): a text turned and haloed at the survey grid's
//! coordinates against the same outlines drawn whole on the CPU; a text of pixels keeps its size as the view zooms
//! while a world text grows; pans of under a pixel move the text by the pan where float32 alone could not; fields on
//! many pages draw as on one.
use super::{FORMAT, Gpu};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::entity::TextPlace;
use kentos_geometry_core::text::Font;
use kentos_geometry_core::text::shaping::{OutlineOp, placed};
use kentos_render_wgpu::Rgba8;
use kentos_render_wgpu::glyphs::{GlyphAtlas, GlyphGpu, GlyphPlace, GlyphQuad, GlyphText, GlyphUnit};
use kentos_render_wgpu::styled::StyledFrame;

const E: f64 = 487_012.346;
const N: f64 = 4_420_187.521;
const W: u32 = 320;
const H: u32 = 200;
const BG: [u8; 4] = [128, 128, 128, 255];
const INK: Rgba8 = Rgba8([20, 40, 160, 255]);
const HALO: Rgba8 = Rgba8([255, 255, 255, 255]);

fn layout(text: &str, height: f64, rotation: f64) -> placed::Layout {
    let place = TextPlace {
        p: Vec2::new(0.0, 0.0),
        text,
        height,
        rotation,
        align: None,
        width_factor: None,
        box_width: None,
        line_spacing: None,
        runs: &[],
        flow: None,
        font: None,
        bold: false,
        italic: false,
        lean: 0.0,
        path: None,
    };
    placed::layout(&place, Font::DEFAULT, false).expect("a layout")
}

fn world(at: [f64; 2], layout: &placed::Layout, halo_px: f32) -> GlyphText {
    GlyphText {
        at,
        unit: GlyphUnit::World,
        color: INK,
        halo: HALO,
        halo_px,
        glyphs: layout.glyphs.iter().map(GlyphPlace::of).collect(),
        under: Vec::new(),
        over: Vec::new(),
    }
}

/// The view `scale` px a metre about `center`, one device pixel a logical one.
fn view(center: [f64; 2], scale: f64) -> StyledFrame {
    StyledFrame {
        center,
        origin: [0.0, 0.0],
        scale,
        dpr: 1.0,
        size_px: [W as f32, H as f32],
        scale_denominator: 1.0,
        twist: 0.0,
        map: None,
    }
}

/// The middle of the layout's ink, from its anchor.
fn middle(layout: &placed::Layout) -> [f64; 2] {
    let ink = layout.ink.expect("ink");
    [(ink[0] + ink[2]) / 2.0, (ink[1] + ink[3]) / 2.0]
}

/// The texts drawn over the grey through `frame`, read back as RGBA.
fn draw(gpu: &Gpu, glyphs: &mut GlyphGpu, texts: &[GlyphText], frame: &StyledFrame) -> Vec<u8> {
    draw_grown(gpu, glyphs, texts, frame, 1.0)
}

/// As [`draw`], the pixel texts grown `grow` times.
fn draw_grown(gpu: &Gpu, glyphs: &mut GlyphGpu, texts: &[GlyphText], frame: &StyledFrame, grow: f32) -> Vec<u8> {
    let (device, queue) = (&gpu.device, &gpu.queue);
    let mut layer = glyphs.layer(device, queue, texts, None);
    assert!(!layer.pending, "every field made without a deadline");
    glyphs.ensure(device, 1);
    glyphs.prepare(device, queue, &mut layer, frame, grow);
    let texture = device.create_texture(&wgpu::TextureDescriptor {
        label: Some("glyphs target"),
        size: wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: FORMAT,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let target = texture.create_view(&wgpu::TextureViewDescriptor::default());
    let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor { label: None });
    {
        let [r, g, b, a] = BG.map(|v| f64::from(v) / 255.0);
        let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: None,
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: &target,
                depth_slice: None,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color { r, g, b, a }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });
        glyphs.draw(&mut pass, &layer, 1);
    }
    let row = (W * 4).div_ceil(wgpu::COPY_BYTES_PER_ROW_ALIGNMENT) * wgpu::COPY_BYTES_PER_ROW_ALIGNMENT;
    let buffer = device.create_buffer(&wgpu::BufferDescriptor {
        label: None,
        size: u64::from(row * H),
        usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });
    encoder.copy_texture_to_buffer(
        texture.as_image_copy(),
        wgpu::TexelCopyBufferInfo {
            buffer: &buffer,
            layout: wgpu::TexelCopyBufferLayout { offset: 0, bytes_per_row: Some(row), rows_per_image: None },
        },
        wgpu::Extent3d { width: W, height: H, depth_or_array_layers: 1 },
    );
    let index = queue.submit([encoder.finish()]);
    let slice = buffer.slice(..);
    slice.map_async(wgpu::MapMode::Read, |_| {});
    device
        .poll(wgpu::PollType::Wait { submission_index: Some(index), timeout: None })
        .expect("the frame finishes");
    let data = slice.get_mapped_range();
    (0..H).flat_map(|y| data[(y * row) as usize..(y * row + W * 4) as usize].to_vec()).collect()
}

/// The same outlines filled whole on the CPU, the halo a round-joined stroke under them.
fn reference(layout: &placed::Layout, at: [f64; 2], frame: &StyledFrame, halo_px: f32) -> Vec<u8> {
    let px = |p: [f64; 2]| {
        let x = (at[0] + p[0] - frame.center[0]) * frame.scale + f64::from(W) / 2.0;
        let y = f64::from(H) / 2.0 - (at[1] + p[1] - frame.center[1]) * frame.scale;
        (x as f32, y as f32)
    };
    let mut path = tiny_skia::PathBuilder::new();
    for g in &layout.glyphs {
        for op in g.outline().expect("an outline") {
            match op {
                OutlineOp::Move(p) => {
                    let (x, y) = px(p);
                    path.move_to(x, y);
                }
                OutlineOp::Line(p) => {
                    let (x, y) = px(p);
                    path.line_to(x, y);
                }
                OutlineOp::Quad(c, p) => {
                    let ((cx, cy), (x, y)) = (px(c), px(p));
                    path.quad_to(cx, cy, x, y);
                }
                OutlineOp::Cubic(a, b, p) => {
                    let ((ax, ay), (bx, by), (x, y)) = (px(a), px(b), px(p));
                    path.cubic_to(ax, ay, bx, by, x, y);
                }
                OutlineOp::Close => path.close(),
            }
        }
    }
    let path = path.finish().expect("a path");
    let mut picture = tiny_skia::Pixmap::new(W, H).expect("a picture");
    picture.fill(tiny_skia::Color::from_rgba8(BG[0], BG[1], BG[2], BG[3]));
    let paint = |c: Rgba8| {
        let mut p = tiny_skia::Paint::default();
        p.set_color_rgba8(c.0[0], c.0[1], c.0[2], c.0[3]);
        p.anti_alias = true;
        p
    };
    if halo_px > 0.0 {
        let stroke = tiny_skia::Stroke {
            width: 2.0 * halo_px,
            line_join: tiny_skia::LineJoin::Round,
            ..tiny_skia::Stroke::default()
        };
        picture.stroke_path(&path, &paint(HALO), &stroke, tiny_skia::Transform::identity(), None);
    }
    picture.fill_path(&path, &paint(INK), tiny_skia::FillRule::Winding, tiny_skia::Transform::identity(), None);
    picture.data().to_vec()
}

/// The mean difference of the colour channels, the pixels a channel of which is off by more than a quarter, and the
/// pixels of ink in `b`.
fn compare(a: &[u8], b: &[u8]) -> (f64, usize, usize) {
    let mut sum = 0u64;
    let (mut gross, mut ink) = (0, 0);
    for (p, q) in a.as_chunks::<4>().0.iter().zip(b.as_chunks::<4>().0) {
        let d = (0..3).map(|i| p[i].abs_diff(q[i])).collect::<Vec<_>>();
        sum += d.iter().map(|&v| u64::from(v)).sum::<u64>();
        if d.iter().any(|&v| v > 64) {
            gross += 1;
        }
        if (0..3).all(|i| q[i].abs_diff(INK.0[i]) < 40) {
            ink += 1;
        }
    }
    (sum as f64 / (a.len() / 4 * 3) as f64, gross, ink)
}

/// Pixels away from the grey, how far in all (the text's area, its edges' share counted), and their middle weighted by
/// how far.
fn inked(rgba: &[u8]) -> (usize, f64, [f64; 2]) {
    let (mut count, mut weight, mut sx, mut sy) = (0, 0.0, 0.0, 0.0);
    for (i, p) in rgba.as_chunks::<4>().0.iter().enumerate() {
        let w = (0..3).map(|c| f64::from(p[c].abs_diff(BG[c]))).sum::<f64>();
        if w > 0.0 {
            if w > 60.0 {
                count += 1;
            }
            let (x, y) = ((i as u32 % W) as f64 + 0.5, (i as u32 / W) as f64 + 0.5);
            weight += w;
            sx += w * x;
            sy += w * y;
        }
    }
    (count, weight, [sx / weight, sy / weight])
}

/// A picture for the owner: `.run/gui-evidence/render/rm13/<name>.png`.
fn keep(rgba: &[u8], name: &str) {
    let out = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join(format!("../../../.run/gui-evidence/render/rm13/{name}.png"));
    let _ = std::fs::create_dir_all(out.parent().expect("a folder"));
    let file = std::io::BufWriter::new(std::fs::File::create(&out).expect("a file"));
    let mut png = png::Encoder::new(file, W, H);
    png.set_color(png::ColorType::Rgba);
    png.set_depth(png::BitDepth::Eight);
    png.write_header().expect("a header").write_image_data(rgba).expect("writes the picture");
}

/// docs/adr/9212 §4 stage 1: a text turned 30° with a halo of 1.5 px, at the survey grid's numbers, against its
/// outlines drawn whole.
pub fn a_turned_haloed_text_matches_its_outlines_drawn_whole(gpu: &Gpu) {
    let mut glyphs = GlyphGpu::new(&gpu.device, FORMAT);
    let text = layout("Kentos Ağ 17", 2.0, 30.0);
    let at = [E, N];
    let m = middle(&text);
    let frame = view([E + m[0], N + m[1]], 20.0);
    let ours = draw(gpu, &mut glyphs, &[world(at, &text, 1.5)], &frame);
    let theirs = reference(&text, at, &frame, 1.5);
    let (mean, gross, ink) = compare(&ours, &theirs);
    let halo = |rgba: &[u8]| rgba.as_chunks::<4>().0.iter().filter(|p| p[0] > 230 && p[1] > 230 && p[2] > 230).count();
    let (halo_ours, halo_theirs) = (halo(&ours), halo(&theirs));
    eprintln!(
        "glyphs: mean difference {mean:.2}, {gross} pixels off by a quarter, {ink} ink pixels, halo {halo_ours} / {halo_theirs}"
    );
    keep(&ours, "dondurulmus-haleli-gpu");
    keep(&theirs, "dondurulmus-haleli-referans");
    assert!(ink > 800, "the text is in view: {ink} ink pixels");
    assert!(mean < 1.5, "mean difference {mean:.2}");
    assert!((gross as f64) < 0.03 * ink as f64, "{gross} pixels off by a quarter of {ink}");
    assert!(halo_ours.abs_diff(halo_theirs) * 5 < halo_theirs, "halo {halo_ours} / {halo_theirs}");
}

/// A text of pixels drawn as large as a world text at one scale stays that large at twice the scale, as the world
/// text grows four times in area.
pub fn a_text_of_pixels_keeps_its_size_as_a_world_text_grows(gpu: &Gpu) {
    let mut glyphs = GlyphGpu::new(&gpu.device, FORMAT);
    let text = layout("Ölçek 1:500", 1.0, 0.0);
    let m = middle(&text);
    let at = [E - m[0], N - m[1]];
    let pixels = GlyphText {
        unit: GlyphUnit::Px { turns: false },
        glyphs: text
            .glyphs
            .iter()
            .map(|g| {
                let mut p = GlyphPlace::of(g);
                p.transform = p.transform.map(|v| v * 20.0);
                p
            })
            .collect(),
        ..world(at, &text, 0.0)
    };
    let world = world(at, &text, 0.0);
    let near = view([E, N], 20.0);
    let (w20, p20) = (
        draw(gpu, &mut glyphs, std::slice::from_ref(&world), &near),
        draw(gpu, &mut glyphs, std::slice::from_ref(&pixels), &near),
    );
    let (mean, _, _) = compare(&w20, &p20);
    assert!(mean < 0.2, "the same at the scale they were matched at: {mean:.3}");
    let nearer = view([E, N], 40.0);
    let (w40, p40) = (
        draw(gpu, &mut glyphs, std::slice::from_ref(&world), &nearer),
        draw(gpu, &mut glyphs, std::slice::from_ref(&pixels), &nearer),
    );
    let ((_, a, _), (_, b, _), (_, c, _)) = (inked(&w20), inked(&w40), inked(&p40));
    let (grown, kept) = (b / a, c / a);
    eprintln!("glyphs: the world text's ink ×{grown:.3}, the pixel text's ×{kept:.3} at twice the scale");
    assert!((grown - 4.0).abs() < 0.2, "the world text grows four times in area: ×{grown:.3}");
    assert!((kept - 1.0).abs() < 0.03, "the pixel text keeps its size: ×{kept:.3}");
    // Grown with the view as a picture of it would (a gesture before the texts are placed again), it is the world
    // text at the new scale.
    let picture = draw_grown(gpu, &mut glyphs, std::slice::from_ref(&pixels), &nearer, 2.0);
    let (mean, _, _) = compare(&w40, &picture);
    eprintln!("glyphs: the pixel text grown twice against the world text at twice the scale: {mean:.3}");
    assert!(mean < 0.2, "{mean:.3}");
}

/// Pans of a quarter, a half and a whole pixel at the survey grid's numbers, a centimetre text at 2000 px a metre:
/// there the layer's float32 step is 2 mm, 3.9 px, so only the low parts can move the text by less.
pub fn sub_pixel_pans_move_the_text_by_the_pan(gpu: &Gpu) {
    const SCALE: f64 = 2000.0;
    let mut glyphs = GlyphGpu::new(&gpu.device, FORMAT);
    let text = layout("Parsel 1234/5", 0.015, 12.0);
    let m = middle(&text);
    let at = [E - m[0], N - m[1]];
    let texts = [world(at, &text, 0.0)];
    let (_, _, before) = inked(&draw(gpu, &mut glyphs, &texts, &view([E, N], SCALE)));
    for pan in [0.25, 0.5, 1.0] {
        let step = pan / SCALE;
        let (_, _, after) = inked(&draw(gpu, &mut glyphs, &texts, &view([E + step, N + step], SCALE)));
        let moved = [after[0] - before[0], after[1] - before[1]];
        eprintln!("glyphs: a pan of {pan} px moved the ink by {:.3}, {:.3} px", moved[0], moved[1]);
        // The camera east and north: the text goes left and down the screen (y down), within a twentieth of a pixel
        // (the field's own sampling as the letters cross the pixel grid).
        assert!((moved[0] + pan).abs() < 0.05 && (moved[1] - pan).abs() < 0.05, "{pan}: {moved:?}");
    }
}

/// Flat quads with a text (docs/adr/9212 §7): a mask turned 30° under its letters covers its parallelogram's area,
/// its edges smoothed over a pixel; the letters are drawn over it and an underline over them.
pub fn masks_and_underlines_draw_under_and_over_the_letters(gpu: &Gpu) {
    let mut glyphs = GlyphGpu::new(&gpu.device, FORMAT);
    let (w, h) = (120.0f64, 40.0f64);
    let (s, c) = 30f64.to_radians().sin_cos();
    // The unit square onto a 120 × 40 px box turned 30°, its middle on the anchor (pixels, y up).
    let mask = GlyphQuad {
        transform: [w * c, w * s, -h * s, h * c, -(w * c - h * s) / 2.0, -(w * s + h * c) / 2.0],
        color: Rgba8([255, 0, 0, 255]),
        triangle: false,
    };
    let alone = GlyphText {
        unit: GlyphUnit::Px { turns: false },
        glyphs: Vec::new(),
        under: vec![mask],
        ..world([E, N], &layout("O", 1.0, 0.0), 0.0)
    };
    let frame = view([E, N], 20.0);
    let rgba = draw(gpu, &mut glyphs, std::slice::from_ref(&alone), &frame);
    // How much of each pixel the mask covers: over the grey, red less green is 255 a whole pixel.
    let share = |p: &[u8; 4]| ((f64::from(p[0]) - f64::from(p[1])) / 255.0).clamp(0.0, 1.0);
    let pixels = rgba.as_chunks::<4>().0;
    let area: f64 = pixels.iter().map(share).sum();
    let edge = pixels.iter().filter(|p| (0.15..0.85).contains(&share(p))).count();
    eprintln!("glyphs: a mask of {} px² covers {area:.0}, {edge} pixels of its edge partly", w * h);
    assert!((area - w * h).abs() < 0.01 * w * h, "{area}");
    assert!(edge > 200, "smoothed edges: {edge}");
    // The letters over the mask, the underline over them.
    let text = layout("OO", 1.2, 0.0);
    let m = middle(&text);
    let ink_at = |rgba: &[u8], colour: [u8; 3]| {
        rgba.as_chunks::<4>().0.iter().filter(|p| (0..3).all(|i| p[i].abs_diff(colour[i]) < 30)).count()
    };
    let lettered = GlyphText {
        under: vec![GlyphQuad {
            transform: [8.0, 0.0, 0.0, 2.0, -4.0 - m[0], -1.0 - m[1]],
            color: Rgba8([255, 0, 0, 255]),
        triangle: false,
    }],
        over: vec![GlyphQuad {
            transform: [8.0, 0.0, 0.0, 0.1, -4.0 - m[0], -0.05 - m[1]],
            color: Rgba8([0, 200, 0, 255]),
        triangle: false,
    }],
        ..world([E, N], &text, 0.0)
    };
    let rgba = draw(gpu, &mut glyphs, std::slice::from_ref(&lettered), &frame);
    let ink_colour = [INK.0[0], INK.0[1], INK.0[2]];
    let letters = GlyphText { under: Vec::new(), over: Vec::new(), ..lettered.clone() };
    let underline = GlyphText { glyphs: Vec::new(), under: Vec::new(), ..lettered.clone() };
    let (alone, bar) = (
        draw(gpu, &mut glyphs, std::slice::from_ref(&letters), &frame),
        draw(gpu, &mut glyphs, std::slice::from_ref(&underline), &frame),
    );
    let (ink, ink_alone) = (ink_at(&rgba, ink_colour), ink_at(&alone, ink_colour));
    let (green, green_alone) = (ink_at(&rgba, [0, 200, 0]), ink_at(&bar, [0, 200, 0]));
    eprintln!("glyphs: ink {ink} over the mask, {ink_alone} alone; underline {green} over the letters, {green_alone} alone");
    // The letters whole over the mask (the underline takes its own row off them), the underline whole over them.
    assert!(ink_alone > 50 && ink + 40 >= ink_alone, "{ink} of {ink_alone}");
    assert!(green_alone > 100 && green == green_alone, "{green} of {green_alone}");
    keep(&rgba, "maske-alt-cizgi-gpu");
}

/// Rings as their triangles with their outline just inside, and lines as segments (docs/adr/9212 §9): a hexagon, a
/// concave L and a line 2 px wide cover their areas, their edges smoothed.
pub fn rings_and_lines_cover_their_areas(gpu: &Gpu) {
    let mut glyphs = GlyphGpu::new(&gpu.device, FORMAT);
    let red = Rgba8([255, 0, 0, 255]);
    let frame = view([E, N], 20.0);
    let cover = |shapes: Vec<GlyphQuad>, glyphs: &mut GlyphGpu| {
        let text = GlyphText {
            unit: GlyphUnit::Px { turns: false },
            glyphs: Vec::new(),
            under: shapes,
            ..world([E, N], &layout("O", 1.0, 0.0), 0.0)
        };
        let rgba = draw(gpu, glyphs, std::slice::from_ref(&text), &frame);
        let shares: Vec<f64> = rgba
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| ((f64::from(p[0]) - f64::from(p[1])) / 255.0).clamp(0.0, 1.0))
            .collect();
        let edge = shares.iter().filter(|s| (0.15..0.85).contains(*s)).count();
        (shares.iter().sum::<f64>(), edge, rgba)
    };
    let hexagon: Vec<[f64; 2]> = (0..6)
        .map(|i| {
            let a = f64::from(i) * std::f64::consts::PI / 3.0 + 0.2;
            [50.0 * a.cos(), 50.0 * a.sin()]
        })
        .collect();
    let (area, edge, _) = cover(GlyphQuad::ring(&hexagon, 1.0, red), &mut glyphs);
    let exact = 1.5 * 3f64.sqrt() * 50.0 * 50.0;
    eprintln!("glyphs: a hexagon of {exact:.0} px² covers {area:.0}, {edge} pixels of its edge partly");
    // The triangles fill to the edge; the outline smooths its outer half pixel (about a pixel of the edge in three).
    assert!((area - exact).abs() < 0.01 * exact && edge > 80, "{area} {edge}");
    // A concave L, clockwise: 80 × 80 less its upper right 50 × 50.
    let l = [[-40.0, -40.0], [-40.0, 40.0], [-10.0, 40.0], [-10.0, -10.0], [40.0, -10.0], [40.0, -40.0]];
    let (area, _, _) = cover(GlyphQuad::ring(&l, 1.0, red), &mut glyphs);
    eprintln!("glyphs: a concave L of 3900 px² covers {area:.0}");
    assert!((area - 3900.0).abs() < 0.015 * 3900.0, "{area}");
    let line = GlyphQuad::segment([-60.0, -70.0], [60.0, -50.0], 2.0, red).expect("a segment");
    let (area, _, rgba) = cover(vec![line], &mut glyphs);
    let exact = 2.0 * 120f64.hypot(20.0);
    eprintln!("glyphs: a line of {exact:.0} px² covers {area:.0}");
    assert!((area - exact).abs() < 0.03 * exact, "{area}");
    keep(&rgba, "cizgi-gpu");
}

/// Fields on pages of 160 px (a dozen and more of them, the texture made again as they come) draw as on one page.
pub fn fields_on_many_pages_draw_as_on_one(gpu: &Gpu) {
    let lines = ["ABCDEFGHIJKLM", "NOPQRSTUVWXYZ", "abcdefghijklm", "nopqrstuvwxyz", "0123456789 ğüşıöç", "ĞÜŞİÖÇ &@%"];
    let texts: Vec<GlyphText> = lines
        .iter()
        .enumerate()
        .map(|(i, line)| world([E - 7.0, N + 4.0 - 1.6 * i as f64], &layout(line, 0.9, 0.0), 1.0))
        .collect();
    let frame = view([E, N], 20.0);
    let mut one = GlyphGpu::new(&gpu.device, FORMAT);
    let mut many = GlyphGpu::with_atlas(&gpu.device, FORMAT, GlyphAtlas::new(160));
    let a = draw(gpu, &mut one, &texts, &frame);
    let b = draw(gpu, &mut many, &texts, &frame);
    let pages = many.atlas().page_count();
    let worst = a.iter().zip(&b).map(|(p, q)| p.abs_diff(*q)).max().unwrap_or(0);
    let (mean, _, _) = compare(&a, &b);
    let (count, _, _) = inked(&a);
    eprintln!("glyphs: {pages} pages; the most a channel differs {worst}, mean {mean:.4}; {count} pixels of text");
    keep(&b, "cok-sayfa-gpu");
    assert_eq!(one.atlas().page_count(), 1);
    assert!(pages >= 12, "{pages} pages");
    assert!(count > 3000, "the lines are in view: {count}");
    assert!(worst <= 2 && mean < 0.05, "{worst} {mean}");
}
