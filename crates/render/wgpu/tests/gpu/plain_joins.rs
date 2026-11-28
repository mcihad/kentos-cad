//! A seen-through plain line where its segments meet (docs/adr/9242). A path's corners and the pieces a long segment is
//! cut into (`precision::MAX_SEGMENT_LENGTH`, 128 m) each draw their own side of the joint's bisector, so the line is
//! as dark there as along it. Every end was round and two ends overlapped at a joint: the grid's 4 % and 9 % lines
//! showed a dark dot where its pieces met, a seen-through underlay's plan one at every corner.
//!
//! A pixel whose middle lies within a hundredth of a pixel of a joint is drawn by both sides, as the styled lines' are
//! (docs/adr/9226): the rasteriser places each quad's corners to a 256th of a pixel, so the two sides' distances to
//! the joint differ by some thousandths, and a narrower band would leave pixels out.
use kentos_render_wgpu::scene::build_lines;
use kentos_render_wgpu::{Camera, RenderSettings, Rgba8, ScenePart, Vec2};

use super::Gpu;

/// A case's lines (world units), the camera's scale and the pixel columns its joints fall on the middles of.
type Case = (Vec<[[f64; 2]; 2]>, f64, &'static [u32]);

/// The union of the segments' capsules at a pixel's middle (device pixels), as the shader covers it: its half width
/// less the distance to the nearest segment, a pixel's ramp.
fn union_cover(segments: &[[[f64; 2]; 2]], half: f64, p: [f64; 2]) -> f64 {
    let d = segments
        .iter()
        .map(|&[a, b]| {
            let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
            let t =
                (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / (dx * dx + dy * dy)).clamp(0.0, 1.0);
            (p[0] - a[0] - t * dx).hypot(p[1] - a[1] - t * dy)
        })
        .fold(f64::INFINITY, f64::min);
    (half - d + 0.5).clamp(0.0, 1.0)
}

pub(super) fn seen_through_joints_are_drawn_once(gpu: &mut Gpu) {
    let keep = gpu.keep;
    gpu.keep = true;
    let size = 160u32;
    let mut settings = RenderSettings::new(Rgba8::rgb(0, 0, 0));
    settings.line_width = 12.0;
    let red = Rgba8([255, 0, 0, 0x80]);
    let alpha = f64::from(red.0[3]) / 255.0;
    // A zigzag's corners turning 126.9° and back, a sharper one (138.5°) at its end, at a pixel a metre. A line 2 km
    // long at a pixel four metres, cut into 16 pieces of 125 m: five joints in view, none within a hundredth of a
    // pixel of a pixel's middle (-62.575, -31.325, -0.075, 31.175, 62.425 px). The same line from -1000 m: its joints at
    // -62.5 and 62.5 px, on pixels' middles (columns 17 and 142).
    let zigzag = [
        [-66., -30.],
        [-36., 30.],
        [-6., -30.],
        [24., 30.],
        [54., -30.],
        [70., 30.],
    ];
    let cases: [Case; 3] = [
        (zigzag.windows(2).map(|w| [w[0], w[1]]).collect(), 1.0, &[]),
        (vec![[[-1000.3, 7.], [999.7, 7.]]], 0.25, &[]),
        (vec![[[-1000., 7.], [1000., 7.]]], 0.25, &[17, 142]),
    ];
    let half = f64::from(size) / 2.0;
    for samples in [1, 4] {
        settings.samples = samples;
        for (k, (lines, scale, on_middles)) in cases.iter().enumerate() {
            let camera = Camera {
                width: f64::from(size),
                height: f64::from(size),
                scale: *scale,
                ..Default::default()
            };
            let pairs: Vec<[Vec2; 2]> = lines
                .iter()
                .map(|[p, q]| [Vec2::new(p[0], p[1]), Vec2::new(q[0], q[1])])
                .collect();
            let part = build_lines(Vec2::new(0.0, 0.0), &[(&pairs, red)]);
            // Each pixel's ink as the union of the capsules gives it: the line drawn once everywhere.
            let on_screen: Vec<[[f64; 2]; 2]> = lines
                .iter()
                .map(|[p, q]| [[p[0] * scale, p[1] * scale], [q[0] * scale, q[1] * scale]])
                .collect();
            let want: Vec<f64> = (0..size * size)
                .map(|i| {
                    let (x, y) = (
                        f64::from(i % size) + 0.5 - half,
                        half - (f64::from(i / size) + 0.5),
                    );
                    union_cover(&on_screen, 6.0, [x, y])
                })
                .collect();
            // The most a pixel's red is off the union, in levels of 255, and where: off the columns given, and on them.
            let mut off = |part: &ScenePart| -> [(f64, u32); 2] {
                let pixels = gpu.draw(&[part], &camera, &settings, size, size);
                let mut most = [(0.0f64, 0u32); 2];
                for (i, (p, w)) in pixels.chunks(4).zip(&want).enumerate() {
                    let d = (f64::from(p[0]) - 255.0 * alpha * w).abs();
                    let side = usize::from(on_middles.contains(&(i as u32 % size)));
                    if d > most[side].0 {
                        most[side] = (d, i as u32);
                    }
                }
                most
            };
            let [(most, at), (middles, _)] = off(&part);
            eprintln!(
                "plain joins case {k}, {samples} samples: most {most:.1} off at ({}, {}); on joints at pixels' middles {middles:.1}",
                at % size,
                at / size
            );
            assert!(
                most <= 3.0,
                "case {k}, {samples} samples: a pixel {most} off the union at ({}, {})",
                at % size,
                at / size
            );
            // A joint on pixels' middles: that column drawn by both sides, twice the ink at most (255 a (1 - a): 64).
            assert!(
                middles <= 64.0 + 3.0,
                "case {k}: {middles} off on the joints' columns"
            );
            // Every end round, as before the joins: twice the ink where two ends overlap.
            let mut old = part.clone();
            old.id = u64::MAX - (k as u64 + 10 * u64::from(samples));
            for s in &mut old.segments {
                s.joins = 0;
            }
            let [(most, _), _] = off(&old);
            assert!(
                most > 40.0,
                "case {k}: round ends overlap at the joints: {most}"
            );
        }
    }
    gpu.keep = keep;
}
