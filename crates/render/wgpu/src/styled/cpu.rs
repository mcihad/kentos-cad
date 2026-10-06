//! The styled shaders on the CPU (docs/sheet/design.md §9a): a map frame of
//! a sheet drawn as a picture where its drawing has something the PDF's
//! vectors cannot write (a pattern or picture fill, a picture or text marker,
//! a soft-edged line, a hatch too dense for paths), on the screen and in the
//! PDF alike. Every batch is drawn as its pipeline draws it: the same style
//! block ([`style_block`]) read the same way, the same function per pixel as
//! `stroke.wgsl`, `fill.wgsl`, `marker.wgsl` and `shapes.wgsl` (ported line
//! for line below), the same visibility rules (`in_scale`, `in_view`,
//! `legible`) and the atlas's images from the same CPU drawing
//! ([`raster::image`]) at the same power-of-two steps. So the picture is the
//! drawing area's look and the web's, whose map pictures the same shaders draw.
//!
//! Pixels are premultiplied RGBA blended over in sRGB, as the software
//! renderer blends; the GPU blends in linear light, which differs only at
//! soft edges. A fill's triangles cover with anti-aliased edges where the GPU
//! samples a pixel's centre.

use tiny_skia::{FillRule, Mask, PathBuilder, Pixmap, Transform};

use kentos_native_style::batches::{BatchKind, StyledBatch, StyledLayer};

use super::picture::ImageSource;
use super::raster::{self, Painted};
use super::uniform::{StyleBlock, style_block};

/// Where a layer's numbers land on a picture: `px = m·(x, y) + t` for metres from the layers'
/// origin (north up on the ground, rows down in the picture; a turned map turns `m`), and what a
/// unit is there.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CpuView {
    /// `[m00, m01, m10, m11]`: `px.x = m00·x + m01·y + t.x`, `px.y = m10·x + m11·y + t.y`.
    pub m: [f64; 4],
    pub t: [f64; 2],
    /// Picture pixels per ground metre (the shaders' `frame.pxPerM`).
    pub px_per_m: f64,
    /// Picture pixels per CSS pixel: a `Unit::Px` length (`frame.dpr`).
    pub dpr: f64,
    /// The map's scale denominator: the batches' scale ranges.
    pub scale: f64,
}

impl CpuView {
    fn px(&self, x: f64, y: f64) -> [f64; 2] {
        [
            self.m[0] * x + self.m[1] * y + self.t[0],
            self.m[2] * x + self.m[3] * y + self.t[1],
        ]
    }

    /// Metres from the layers' origin at a picture point.
    fn ground(&self, px: f64, py: f64) -> [f64; 2] {
        let det = self.m[0] * self.m[3] - self.m[1] * self.m[2];
        let (dx, dy) = (px - self.t[0], py - self.t[1]);
        [
            (self.m[3] * dx - self.m[1] * dy) / det,
            (-self.m[2] * dx + self.m[0] * dy) / det,
        ]
    }

    /// A shader's pixel vector (y up, the drawing area's pixels) as a picture's: the map's turn
    /// and the rows running down.
    fn lin(&self, v: [f64; 2]) -> [f64; 2] {
        let k = self.px_per_m.max(1e-12);
        [
            (self.m[0] * v[0] + self.m[1] * v[1]) / k,
            (self.m[2] * v[0] + self.m[3] * v[1]) / k,
        ]
    }

    /// The inverse of [`lin`](Self::lin).
    fn unlin(&self, v: [f64; 2]) -> [f64; 2] {
        let k = self.px_per_m.max(1e-12);
        let (a, b, c, d) = (self.m[0] / k, self.m[1] / k, self.m[2] / k, self.m[3] / k);
        let det = a * d - b * c;
        [(d * v[0] - b * v[1]) / det, (-c * v[0] + a * v[1]) / det]
    }

    /// The ground box the picture covers, metres from the layers' origin.
    fn ground_box(&self, w: u32, h: u32) -> [f64; 4] {
        let corners = [
            self.ground(0.0, 0.0),
            self.ground(f64::from(w), 0.0),
            self.ground(0.0, f64::from(h)),
            self.ground(f64::from(w), f64::from(h)),
        ];
        let lo = |i: usize| corners.iter().map(|c| c[i]).fold(f64::INFINITY, f64::min);
        let hi = |i: usize| {
            corners
                .iter()
                .map(|c| c[i])
                .fold(f64::NEG_INFINITY, f64::max)
        };
        [lo(0), lo(1), hi(0), hi(1)]
    }
}

/// The atlas's largest step (`MAX_STEP`) and its smallest.
const MAX_STEP: u32 = 512;
const MIN_STEP: u32 = 8;

/// The atlas's step for an image shown `px` pixels large (`step_for`).
fn step_for(px: f64) -> u32 {
    let p = px.max(f64::from(MIN_STEP)).min(f64::from(MAX_STEP));
    (2f64.powf(p.log2().ceil()) as u32).clamp(MIN_STEP, MAX_STEP)
}

/// Draws a layer's batches into `pixmap` in their order, as the styled pipelines draw them.
pub fn paint_layer(
    pixmap: &mut Pixmap,
    view: &CpuView,
    layer: &StyledLayer,
    source: &dyn ImageSource,
) {
    let seen = view.ground_box(pixmap.width(), pixmap.height());
    for b in &layer.batches {
        if !b.in_scale(view.scale)
            || !b.in_view(seen, view.px_per_m, view.dpr)
            || !b.legible(view.px_per_m, view.dpr)
        {
            continue;
        }
        let st = Style::of(&style_block(b));
        let data = layer.data.get(b.range.clone()).unwrap_or_default();
        match &b.kind {
            BatchKind::Stroke { .. } => strokes(pixmap, view, b, &st, data),
            BatchKind::Fill { .. } => fill(pixmap, view, b, &st, data, source),
            BatchKind::Marker { .. } => markers(pixmap, view, b, &st, data, source),
        }
    }
}

/// Layers drawn on a transparent picture `width` × `height`: straight RGBA, row by row.
pub fn paint(
    width: u32,
    height: u32,
    view: &CpuView,
    layers: &[&StyledLayer],
    source: &dyn ImageSource,
) -> Option<Vec<u8>> {
    let mut pixmap = Pixmap::new(width, height)?;
    for layer in layers {
        paint_layer(&mut pixmap, view, layer, source);
    }
    Some(
        pixmap
            .pixels()
            .iter()
            .flat_map(|p| {
                let c = p.demultiply();
                [c.red(), c.green(), c.blue(), c.alpha()]
            })
            .collect(),
    )
}

// ── The style block as the shaders read it (`SStyle`) ─────────────────────

type V4 = [f64; 4];

struct Style {
    color: V4,
    stroke: V4,
    dash0: V4,
    dash1: V4,
    rect: V4,
    a: V4,
    b: V4,
    c: V4,
    flags: [u32; 4],
}

impl Style {
    fn of(s: &StyleBlock) -> Style {
        let v = |at: usize| {
            [
                f64::from(s.f[at]),
                f64::from(s.f[at + 1]),
                f64::from(s.f[at + 2]),
                f64::from(s.f[at + 3]),
            ]
        };
        Style {
            color: v(0),
            stroke: v(4),
            dash0: v(8),
            dash1: v(12),
            rect: v(16),
            a: v(20),
            b: v(24),
            c: v(28),
            flags: s.u,
        }
    }

    /// `unitK`: picture pixels per unit of the batch.
    fn unit_k(&self, view: &CpuView) -> f64 {
        if self.flags[0] == 0 {
            view.px_per_m
        } else {
            view.dpr
        }
    }

    /// The eight dash lengths (`dash0`, `dash1`).
    fn dashes(&self) -> [f64; 8] {
        [
            self.dash0[0],
            self.dash0[1],
            self.dash0[2],
            self.dash0[3],
            self.dash1[0],
            self.dash1[1],
            self.dash1[2],
            self.dash1[3],
        ]
    }
}

// ── Helpers of `common.wgsl` ──────────────────────────────────────────────

fn clamp01(x: f64) -> f64 {
    x.clamp(0.0, 1.0)
}

fn fmod(x: f64, y: f64) -> f64 {
    x - y * (x / y).floor()
}

fn fract(x: f64) -> f64 {
    x - x.floor()
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = clamp01((x - e0) / (e1 - e0));
    t * t * (3.0 - 2.0 * t)
}

/// `dashCover`: `s` along the line in picture pixels.
fn dash_cover(d: &[f64; 8], s: f64, k: f64, total_raw: f64, on_share: f64, offset_raw: f64) -> f64 {
    let total = total_raw * k;
    if total <= 0.0 {
        return 1.0;
    }
    // A drawn dash under a pixel before a gap is a dot of one pixel (docs/adr/0186 §3).
    if total < 4.0 {
        let dots: f64 = (0..8)
            .step_by(2)
            .filter(|&i| d[i + 1] > 0.0)
            .map(|i| (1.0 - d[i] * k).max(0.0))
            .sum();
        return (on_share + dots / total).min(1.0);
    }
    let t = fmod(s + offset_raw * k, total);
    let mut acc = 0.0;
    let mut a: f64 = 0.0;
    let mut found = false;
    for (i, raw) in d.iter().enumerate() {
        let l = raw * k;
        if i % 2 == 0 && l < 1.0 && d.get(i + 1).is_some_and(|g| *g > 0.0) {
            let e = (t - (acc + 0.5 * l)).abs();
            a = a.max(clamp01(1.0 - e.min(total - e)));
        }
        if !found && t < acc + l {
            found = true;
            if i % 2 == 0 {
                a = a.max(clamp01((t - acc).min(acc + l - t) + 0.5));
            }
        }
        acc += l;
    }
    a
}

// ── Blending ──────────────────────────────────────────────────────────────

/// A premultiplied colour (0…1) over the pixel at (`x`, `y`).
fn over(pixmap: &mut Pixmap, x: u32, y: u32, c: V4) {
    if c[3] < 0.004 / 255.0 {
        return;
    }
    let w = pixmap.width();
    let i = ((y * w + x) * 4) as usize;
    let data = pixmap.data_mut();
    let inv = 1.0 - c[3].min(1.0);
    let a = (c[3].min(1.0) * 255.0 + f64::from(data[i + 3]) * inv)
        .round()
        .clamp(0.0, 255.0);
    for ch in 0..3 {
        let v = (c[ch].max(0.0) * 255.0 + f64::from(data[i + ch]) * inv).round();
        // Premultiplied: a channel never above the alpha.
        data[i + ch] = v.clamp(0.0, a) as u8;
    }
    data[i + 3] = a as u8;
}

/// A straight colour at coverage `a` as premultiplied.
fn straight(c: V4, a: f64) -> V4 {
    let alpha = c[3] * a;
    [c[0] * alpha, c[1] * alpha, c[2] * alpha, alpha]
}

/// The picture's rows a convex polygon covers, each with its columns (pixel centres inside the
/// polygon's box on that row, give or take a pixel; the per-pixel test decides).
fn rows_of(corners: &[[f64; 2]], w: u32, h: u32, mut each: impl FnMut(u32, u32)) {
    let y0 = corners.iter().map(|c| c[1]).fold(f64::INFINITY, f64::min);
    let y1 = corners
        .iter()
        .map(|c| c[1])
        .fold(f64::NEG_INFINITY, f64::max);
    if !(y0.is_finite() && y1.is_finite()) {
        return;
    }
    let from = (y0.floor().max(0.0)) as u32;
    let to = (y1.ceil().min(f64::from(h))) as u32;
    for y in from..to {
        let yc = f64::from(y) + 0.5;
        // Where the row's centre line crosses the polygon's edges.
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        for i in 0..corners.len() {
            let (a, b) = (corners[i], corners[(i + 1) % corners.len()]);
            let (ya, yb) = (a[1], b[1]);
            if (ya - yc) * (yb - yc) <= 0.0 {
                let x = if (yb - ya).abs() < 1e-12 {
                    lo = lo.min(a[0].min(b[0]));
                    hi = hi.max(a[0].max(b[0]));
                    continue;
                } else {
                    a[0] + (b[0] - a[0]) * (yc - ya) / (yb - ya)
                };
                lo = lo.min(x);
                hi = hi.max(x);
            }
        }
        if lo > hi {
            continue;
        }
        let x0 = (lo - 1.0).floor().max(0.0) as u32;
        let x1 = ((hi + 1.0).ceil().min(f64::from(w))) as u32;
        for x in x0..x1 {
            each(x, y);
        }
    }
}

// ── Strokes (`stroke.wgsl`) ───────────────────────────────────────────────

fn strokes(pixmap: &mut Pixmap, view: &CpuView, b: &StyledBatch, st: &Style, data: &[f32]) {
    let k = st.unit_k(view);
    let half_w = (st.a[0] * k).max(1.0) * 0.5;
    let blur = (st.b[0] * k).max(1.0);
    let cover = |t: f64| {
        let c = clamp01(t / blur + 0.5);
        if blur > 1.0 {
            smoothstep(0.0, 1.0, c)
        } else {
            c
        }
    };
    let cap_cover = |x: f64, y: f64, cap: u32| match cap {
        1 => cover(half_w - x.hypot(y)),
        2 => cover(half_w - y) * cover(half_w - x),
        _ => cover(half_w - y) * cover(-x),
    };
    let dashes = st.dashes();
    let (w, h) = (pixmap.width(), pixmap.height());
    for seg in data.chunks_exact(6) {
        let a = view.px(
            f64::from(seg[0]) + b.origin[0],
            f64::from(seg[1]) + b.origin[1],
        );
        let e = view.px(
            f64::from(seg[2]) + b.origin[0],
            f64::from(seg[3]) + b.origin[1],
        );
        let d = [e[0] - a[0], e[1] - a[1]];
        let len = d[0].hypot(d[1]);
        let dir = if len > 1e-4 {
            [d[0] / len, d[1] / len]
        } else {
            [1.0, 0.0]
        };
        let n = [-dir[1], dir[0]];
        let ext = half_w + 0.5 * blur + 1.0;
        let s0 = f64::from(seg[4]) * view.px_per_m;
        let ends = (f64::from(seg[5]) + 0.5) as u32;
        let at = |along: f64, across: f64| {
            [
                a[0] + dir[0] * along + n[0] * across,
                a[1] + dir[1] * along + n[1] * across,
            ]
        };
        let quad = [
            at(-ext, -ext),
            at(len + ext, -ext),
            at(len + ext, ext),
            at(-ext, ext),
        ];
        let mut lit: Vec<(u32, u32, f64)> = Vec::new();
        rows_of(&quad, w, h, |px, py| {
            let p = [f64::from(px) + 0.5 - a[0], f64::from(py) + 0.5 - a[1]];
            let x = p[0] * dir[0] + p[1] * dir[1];
            let y = (p[0] * n[0] + p[1] * n[1]).abs();
            if x < -ext || x > len + ext || y > ext {
                return;
            }
            let mut cov = if x < 0.0 {
                let cap = if ends & 1 != 0 { st.flags[1] } else { 1 };
                cap_cover(-x, y, cap)
            } else if x > len {
                let cap = if ends & 2 != 0 { st.flags[1] } else { 1 };
                cap_cover(x - len, y, cap)
            } else {
                cover(half_w - y)
            };
            cov *= dash_cover(&dashes, s0 + x, k, st.a[1], st.a[2], st.a[3]);
            if cov >= 0.004 {
                lit.push((px, py, cov));
            }
        });
        for (px, py, cov) in lit {
            over(pixmap, px, py, straight(st.color, cov));
        }
    }
}

// ── Fills (`fill.wgsl`) ───────────────────────────────────────────────────

/// The picture's coverage of a fill's triangles (anti-aliased), and the box it lies in.
fn coverage(
    view: &CpuView,
    b: &StyledBatch,
    data: &[f32],
    w: u32,
    h: u32,
) -> Option<(Mask, [u32; 4])> {
    let mut pb = PathBuilder::new();
    for t in data.chunks_exact(6) {
        let p = |i: usize| {
            let q = view.px(
                f64::from(t[i]) + b.origin[0],
                f64::from(t[i + 1]) + b.origin[1],
            );
            (q[0] as f32, q[1] as f32)
        };
        let (a, c, d) = (p(0), p(2), p(4));
        pb.move_to(a.0, a.1);
        pb.line_to(c.0, c.1);
        pb.line_to(d.0, d.1);
        pb.close();
    }
    let path = pb.finish()?;
    let r = path.bounds();
    let x0 = (r.left().floor().max(0.0)) as u32;
    let y0 = (r.top().floor().max(0.0)) as u32;
    let x1 = (r.right().ceil().min(w as f32)).max(0.0) as u32;
    let y1 = (r.bottom().ceil().min(h as f32)).max(0.0) as u32;
    if x0 >= x1 || y0 >= y1 {
        return None;
    }
    let mut mask = Mask::new(w, h)?;
    mask.fill_path(&path, FillRule::Winding, true, Transform::identity());
    Some((mask, [x0, y0, x1, y1]))
}

fn fill(
    pixmap: &mut Pixmap,
    view: &CpuView,
    b: &StyledBatch,
    st: &Style,
    data: &[f32],
    source: &dyn ImageSource,
) {
    use kentos_native_style::batches::FillPaintBatch;
    let BatchKind::Fill { paint } = &b.kind else {
        return;
    };
    let (w, h) = (pixmap.width(), pixmap.height());
    let Some((mask, [x0, y0, x1, y1])) = coverage(view, b, data, w, h) else {
        return;
    };
    // A picture's pixels (docs/adr/0192 §3); light grey while there are none.
    let picture = match paint {
        FillPaintBatch::Image { image, .. } => Some(source.bitmap(image)),
        _ => None,
    };
    // A tile's image, drawn once at the atlas's step.
    let tile = match paint {
        FillPaintBatch::Tile { image, .. } => {
            match raster::image(image, step_for(b.image_px(view.px_per_m, view.dpr)), source) {
                Some(p) => Some(p),
                None => return,
            }
        }
        _ => None,
    };
    let dashes = st.dashes();
    let screen = st.flags[0] == 1;
    let cover = mask.data();
    for y in y0..y1 {
        for x in x0..x1 {
            let m = cover[(y * w + x) as usize];
            if m == 0 {
                continue;
            }
            let (cx, cy) = (f64::from(x) + 0.5, f64::from(y) + 0.5);
            // `i.world`: metres from the batch's tile; or, for a length in CSS pixels, the
            // picture's pixel there in CSS pixels, y up.
            let (p, k) = if screen {
                ([cx / view.dpr, -cy / view.dpr], view.dpr)
            } else {
                let g = view.ground(cx, cy);
                ([g[0] - b.origin[0], g[1] - b.origin[1]], view.px_per_m)
            };
            let c = match paint {
                FillPaintBatch::Solid { .. } => straight(st.color, 1.0),
                FillPaintBatch::Hatch { .. } => straight(st.color, hatch(st, &dashes, p, k)),
                FillPaintBatch::Gradient { .. } => gradient(st, p),
                FillPaintBatch::Pattern { .. } => pattern(st, p, k, view.dpr),
                FillPaintBatch::Tile { .. } => match &tile {
                    Some(t) => tile_at(st, t, p),
                    None => continue,
                },
                FillPaintBatch::Image { .. } => {
                    picture_at(st, picture.as_ref().and_then(|b| b.as_deref()), p)
                }
            };
            let cov = f64::from(m) / 255.0;
            over(
                pixmap,
                x,
                y,
                [c[0] * cov, c[1] * cov, c[2] * cov, c[3] * cov],
            );
        }
    }
}

/// `hatchFs`: the share of the pixel the hatch inks.
fn hatch(st: &Style, dashes: &[f64; 8], p: [f64; 2], k: f64) -> f64 {
    let dir = [st.a[0], st.a[1]];
    let n = [-dir[1], dir[0]];
    let half_w = (st.a[3] * k).max(1.0) * 0.5;
    let gap = st.a[2] * k;
    let a = if gap < 3.0 {
        // An even tint that fades from the hairline look to the paper's coverage.
        let hair = (2.0 * half_w / gap.max(1e-4)).min(1.0);
        let paper = (st.a[3] / st.a[2].max(1e-9)).min(1.0);
        let t = clamp01((gap - 1.0) / 2.0);
        paper + (hair - paper) * t
    } else {
        let u = p[0] * n[0] + p[1] * n[1] - st.b[0];
        let d = (fract(u / st.a[2] + 0.5) - 0.5).abs() * gap;
        // A family's line `row` starts `row` staggers further along (docs/adr/0186 §3).
        let row = (u / st.a[2] + 0.5).floor();
        clamp01(half_w + 0.5 - d)
            * dash_cover(
                dashes,
                (p[0] * dir[0] + p[1] * dir[1] - row * st.rect[0]) * k,
                k,
                st.b[1],
                st.b[2],
                st.b[3],
            )
    };
    if a < 0.004 { 0.0 } else { a }
}

/// `gradientFs` (docs/adr/0186 §3): the first colour mixed toward the second.
fn gradient(st: &Style, p: [f64; 2]) -> V4 {
    let mut t = if st.flags[1] == 2 {
        let d = ((p[0] - st.b[0]).powi(2) + (p[1] - st.b[1]).powi(2)).sqrt();
        1.0 - clamp01(d / st.b[2].max(1e-9))
    } else {
        let u =
            clamp01((p[0] * st.a[0] + p[1] * st.a[1] - st.a[2]) / (st.a[3] - st.a[2]).max(1e-9));
        if st.flags[1] == 1 {
            1.0 - (2.0 * u - 1.0).abs()
        } else {
            u
        }
    };
    if st.flags[2] == 1 {
        t = 1.0 - t;
    }
    let mix = |i: usize| st.color[i] + (st.stroke[i] - st.color[i]) * t;
    let c = [mix(0), mix(1), mix(2), mix(3)];
    straight(c, 1.0)
}

/// A premultiplied sample of an atlas image (its border included), linear, at texel-space `(u, v)`.
fn sample(img: &Painted, u: f64, v: f64) -> V4 {
    let (w, h) = (img.width + 2 * raster::PAD, img.height + 2 * raster::PAD);
    let (x, y) = (u - 0.5, v - 0.5);
    let (x0, y0) = (x.floor(), y.floor());
    let (fx, fy) = (x - x0, y - y0);
    let at = |xi: f64, yi: f64| -> V4 {
        let xi = (xi.max(0.0) as u32).min(w - 1);
        let yi = (yi.max(0.0) as u32).min(h - 1);
        let i = ((yi * w + xi) * 4) as usize;
        let d = &img.data;
        [
            f64::from(d[i]) / 255.0,
            f64::from(d[i + 1]) / 255.0,
            f64::from(d[i + 2]) / 255.0,
            f64::from(d[i + 3]) / 255.0,
        ]
    };
    let (a, b, c, d) = (
        at(x0, y0),
        at(x0 + 1.0, y0),
        at(x0, y0 + 1.0),
        at(x0 + 1.0, y0 + 1.0),
    );
    let mut out = [0.0; 4];
    for ch in 0..4 {
        let top = a[ch] + (b[ch] - a[ch]) * fx;
        let bottom = c[ch] + (d[ch] - c[ch]) * fx;
        out[ch] = top + (bottom - top) * fy;
    }
    out
}

/// `tileFs`: the tile's colour at `p` (premultiplied), its opacity applied.
fn tile_at(st: &Style, t: &Painted, p: [f64; 2]) -> V4 {
    let r = [st.a[2], st.a[3]];
    let q = [
        r[0] * p[0] + r[1] * p[1] - st.b[0],
        -r[1] * p[0] + r[0] * p[1] - st.b[1],
    ];
    let f = [fract(q[0] / st.a[0]), fract(q[1] / st.a[1])];
    // Half a texel in from the image's edges, as the shader's inset.
    let (iw, ih) = (f64::from(t.width), f64::from(t.height));
    let pad = f64::from(raster::PAD);
    let u = pad + 0.5 + f[0] * (iw - 1.0);
    let v = pad + 0.5 + (1.0 - f[1]) * (ih - 1.0);
    let c = sample(t, u, v);
    let o = st.b[2];
    [c[0] * o, c[1] * o, c[2] * o, c[3] * o]
}

/// `imageFs` (premultiplied): the picture's pixel under `p`, bilinear, at its opacity;
/// light grey for a picture without pixels.
fn picture_at(st: &Style, bitmap: Option<&super::picture::Bitmap>, p: [f64; 2]) -> V4 {
    let o = st.b[2];
    let Some(bm) = bitmap.filter(|b| b.width > 0 && b.height > 0) else {
        let g = 200.0 / 255.0;
        return [g * o, g * o, g * o, o];
    };
    let d = [p[0] - st.a[0], p[1] - st.a[1]];
    let s = (d[0] * st.b[0] + d[1] * st.b[1]) / st.a[2];
    let mut t = (d[1] * st.b[0] - d[0] * st.b[1]) / st.a[3];
    if st.b[3] > 0.5 {
        t = 1.0 - t;
    }
    let (w, h) = (f64::from(bm.width), f64::from(bm.height));
    let u = (s.clamp(0.0, 1.0) * w - 0.5).clamp(0.0, w - 1.0);
    let v = ((1.0 - t).clamp(0.0, 1.0) * h - 0.5).clamp(0.0, h - 1.0);
    let (x0, y0) = (u.floor(), v.floor());
    let (fx, fy) = (u - x0, v - y0);
    let at = |x: f64, y: f64| -> V4 {
        let x = x.clamp(0.0, w - 1.0) as usize;
        let y = y.clamp(0.0, h - 1.0) as usize;
        let i = (y * bm.width as usize + x) * 4;
        let a = f64::from(bm.rgba[i + 3]) / 255.0;
        [
            f64::from(bm.rgba[i]) / 255.0 * a,
            f64::from(bm.rgba[i + 1]) / 255.0 * a,
            f64::from(bm.rgba[i + 2]) / 255.0 * a,
            a,
        ]
    };
    let (c00, c10, c01, c11) = (
        at(x0, y0),
        at(x0 + 1.0, y0),
        at(x0, y0 + 1.0),
        at(x0 + 1.0, y0 + 1.0),
    );
    let mut out = [0.0; 4];
    for k in 0..4 {
        let top = c00[k] + (c10[k] - c00[k]) * fx;
        let bottom = c01[k] + (c11[k] - c01[k]) * fx;
        out[k] = (top + (bottom - top) * fy) * o;
    }
    out
}

/// `hash3`, in float32 as the GPU computes it: the cell's random shift and whether it is drawn.
fn hash3(p: [f32; 2]) -> [f32; 3] {
    let fr = |x: f32| x - x.floor();
    let mut q = [fr(p[0] * 0.1031), fr(p[1] * 0.1030), fr(p[0] * 0.0973)];
    let d = q[0] * (q[1] + 33.33) + q[1] * (q[0] + 33.33) + q[2] * (q[2] + 33.33);
    q = [q[0] + d, q[1] + d, q[2] + d];
    [
        fr((q[0] + q[1]) * q[2]),
        fr((q[0] + q[2]) * q[1]),
        fr((q[1] + q[2]) * q[0]),
    ]
}

/// `patternFs` (premultiplied): one shape on a grid, each cell's shape shifted and kept by its
/// hash; a tint where the cells are too small to draw.
fn pattern(st: &Style, p: [f64; 2], k: f64, dpr: f64) -> V4 {
    let rot = [st.dash0[2], st.dash0[3]];
    let p = [
        rot[0] * p[0] + rot[1] * p[1] - st.dash1[0],
        -rot[1] * p[0] + rot[0] * p[1] - st.dash1[1],
    ];
    let size = [st.dash0[0], st.dash0[1]];
    let shape = st.flags[2];
    let col = if size[0].min(size[1]) * k < 4.0 {
        let mut ink = st.color;
        // As the shader tests it: a colour that is not positive (none, or NaN: unreadable) does not ink.
        let positive = |x: f64| x > 0.0;
        if (!positive(st.color[3]) || is_open(shape)) && positive(st.stroke[3]) {
            ink = st.stroke;
        }
        let a = ink[3] * st.b[1];
        [ink[0] * a, ink[1] * a, ink[2] * a, a]
    } else {
        let mut best = 1e9;
        let mut best_q = [0.0; 2];
        let reach = st.flags[3] as i32;
        let row0 = (p[1] / size[1]).floor();
        for dy in -1..=1i32 {
            if dy.abs() > reach {
                continue;
            }
            let row = row0 + f64::from(dy);
            let sx = if st.flags[1] == 1 && fmod(row, 2.0) > 0.5 {
                0.5 * size[0]
            } else {
                0.0
            };
            let col0 = ((p[0] - sx) / size[0]).floor();
            for dx in -1..=1i32 {
                if dx.abs() > reach {
                    continue;
                }
                let cell = [col0 + f64::from(dx), row];
                let hh = hash3([
                    (cell[0] as f32) + (st.a[3] as f32) * 17.31,
                    (cell[1] as f32) + (st.a[3] as f32) * 7.73,
                ]);
                if f64::from(hh[2]) > st.a[2] {
                    continue;
                }
                let c = [
                    (cell[0] + 0.5) * size[0] + sx + (f64::from(hh[0]) - 0.5) * st.dash1[2],
                    (row + 0.5) * size[1] + (f64::from(hh[1]) - 0.5) * st.dash1[3],
                ];
                let q = [p[0] - c[0] - st.rect[2], p[1] - c[1] - st.rect[3]];
                let mr = [st.a[0], st.a[1]];
                let q = [
                    (mr[0] * q[0] + mr[1] * q[1]) * k,
                    (-mr[1] * q[0] + mr[0] * q[1]) * k,
                ];
                let d = shape_dist(shape, q, [st.rect[0] * k, st.rect[1] * k], st.c);
                if d < best {
                    best = d;
                    best_q = q;
                }
            }
        }
        if best > 1e8 {
            return [0.0; 4];
        }
        let sw = if st.b[0] > 0.0 {
            (st.b[0] * k).max(1.0)
        } else {
            0.0
        };
        shape_color(
            shape,
            best,
            best_q,
            [st.rect[0] * k, st.rect[1] * k],
            st.color,
            st.stroke,
            sw,
            dpr,
        )
    };
    let o = st.b[2];
    [col[0] * o, col[1] * o, col[2] * o, col[3] * o]
}

// ── Shapes (`shapes.wgsl`) ────────────────────────────────────────────────

fn length(p: [f64; 2]) -> f64 {
    p[0].hypot(p[1])
}

fn m_box(p: [f64; 2], b: [f64; 2]) -> f64 {
    (p[0].abs() - b[0]).max(p[1].abs() - b[1])
}

fn sd_seg(p: [f64; 2], a: [f64; 2], b: [f64; 2]) -> f64 {
    let pa = [p[0] - a[0], p[1] - a[1]];
    let ba = [b[0] - a[0], b[1] - a[1]];
    let h = clamp01((pa[0] * ba[0] + pa[1] * ba[1]) / (ba[0] * ba[0] + ba[1] * ba[1]));
    length([pa[0] - ba[0] * h, pa[1] - ba[1] * h])
}

fn m_rhombus(q: [f64; 2], b: [f64; 2]) -> f64 {
    let p = [q[0].abs(), q[1].abs()];
    (p[0] * b[1] + p[1] * b[0] - b[0] * b[1]) / length(b)
}

fn m_ngon(p: [f64; 2], a: f64, n: f64, a0: f64) -> f64 {
    if p[0] * p[0] + p[1] * p[1] < 1e-12 {
        return -a;
    }
    let s = std::f64::consts::TAU / n;
    let mut t = p[1].atan2(p[0]) - a0;
    t -= s * (t / s + 0.5).floor();
    length(p) * t.cos() - a
}

fn sd_star(q: [f64; 2], r: f64, rf: f64) -> f64 {
    let k1 = [0.809_016_994_375, -0.587_785_252_292];
    let k2 = [-0.809_016_994_375, -0.587_785_252_292];
    let mut p = [q[0].abs(), q[1]];
    let d1 = 2.0 * (k1[0] * p[0] + k1[1] * p[1]).max(0.0);
    p = [p[0] - d1 * k1[0], p[1] - d1 * k1[1]];
    let d2 = 2.0 * (k2[0] * p[0] + k2[1] * p[1]).max(0.0);
    p = [p[0] - d2 * k2[0], p[1] - d2 * k2[1]];
    p[0] = p[0].abs();
    p[1] -= r;
    let ba = [rf * -k1[1], rf * k1[0] - 1.0];
    let h = ((p[0] * ba[0] + p[1] * ba[1]) / (ba[0] * ba[0] + ba[1] * ba[1])).clamp(0.0, r);
    let s = p[1] * ba[0] - p[0] * ba[1];
    length([p[0] - ba[0] * h, p[1] - ba[1] * h])
        * if s > 0.0 {
            1.0
        } else if s < 0.0 {
            -1.0
        } else {
            0.0
        }
}

fn sd_gear(p: [f64; 2], r: f64, n: f64, depth: f64) -> f64 {
    let rb = r * (1.0 - depth);
    if p[0] * p[0] + p[1] * p[1] < 1e-12 {
        return -rb;
    }
    let a = std::f64::consts::TAU / n;
    let k = (p[1].atan2(p[0]) / a + 0.5).floor() * a;
    let q = [
        k.cos() * p[0] + k.sin() * p[1],
        -k.sin() * p[0] + k.cos() * p[1],
    ];
    let hw = 0.25 * a * (rb + 0.5 * (r - rb));
    let x0 = rb - 0.5 * (r - rb);
    (length(p) - rb).min(m_box([q[0] - 0.5 * (x0 + r), q[1]], [0.5 * (r - x0), hw]))
}

fn sd_arc(q: [f64; 2], r: f64, ap: f64) -> f64 {
    let p = [q[0].abs(), q[1]];
    let sc = [ap.sin(), ap.cos()];
    if sc[1] * p[0] > sc[0] * p[1] {
        return length([p[0] - sc[0] * r, p[1] - sc[1] * r]);
    }
    (length(p) - r).abs()
}

/// `shapeBase`; `sp`: hole (share of the radius), teeth, opening (radians), tooth depth.
fn shape_base(s: u32, p: [f64; 2], hs: [f64; 2], sp: V4) -> f64 {
    let r = hs[0].min(hs[1]);
    match s {
        0 | 1 => length(p) - r,
        2 => m_box(p, [r, r]),
        3 => m_box(p, hs),
        4 => m_rhombus(p, hs),
        5 => m_ngon(p, r * 0.577_350_27, 3.0, -std::f64::consts::FRAC_PI_2),
        6 => m_ngon(p, r, 5.0, -std::f64::consts::FRAC_PI_2),
        7 => m_ngon(p, r, 6.0, 0.0),
        8 => m_ngon(p, r, 8.0, 0.0),
        9 => sd_star(p, r, 0.4),
        10 => sd_seg(p, [-r, 0.0], [r, 0.0]).min(sd_seg(p, [0.0, -r], [0.0, r])),
        11 => {
            let d = r * std::f64::consts::FRAC_1_SQRT_2;
            sd_seg(p, [-d, -d], [d, d]).min(sd_seg(p, [-d, d], [d, -d]))
        }
        12 => sd_seg(p, [-hs[0], 0.0], [hs[0], 0.0]),
        13 => sd_seg(p, [-hs[0], 0.0], [hs[0], 0.0])
            .min(sd_seg(p, [hs[0] * 0.5, hs[1] * 0.4], [hs[0], 0.0]))
            .min(sd_seg(p, [hs[0] * 0.5, -hs[1] * 0.4], [hs[0], 0.0])),
        14 => {
            let a = [hs[0], 0.0];
            let b = [-hs[0], hs[1] * 0.8];
            let c = [-hs[0], -hs[1] * 0.8];
            let d = sd_seg(p, a, b).min(sd_seg(p, b, c)).min(sd_seg(p, c, a));
            let inside = p[0] >= -hs[0] && p[1].abs() <= (hs[0] - p[0]) * 0.4 * hs[1] / hs[0];
            if inside { -d } else { d }
        }
        15 => {
            sd_seg(p, [-hs[0], hs[1]], [hs[0], 0.0]).min(sd_seg(p, [hs[0], 0.0], [-hs[0], -hs[1]]))
        }
        16 => (length(p) - r).max(-p[1]),
        18 => sd_gear(p, r, sp[1].max(3.0), sp[3]),
        19 => sd_arc(p, r, (sp[2] * 0.5).min(std::f64::consts::PI)),
        _ => (length(p) - r).max((-p[0]).max(-p[1])),
    }
}

fn is_open(s: u32) -> bool {
    matches!(s, 10 | 11 | 12 | 13 | 15 | 19)
}

/// `shapeDist`: the shape's signed distance, its hole cut out.
fn shape_dist(s: u32, p: [f64; 2], hs: [f64; 2], sp: V4) -> f64 {
    let mut d = shape_base(s, p, hs, sp);
    if sp[0] > 0.0 && !is_open(s) {
        d = d.max(sp[0] * hs[0].min(hs[1]) - length(p));
    }
    d
}

/// `shapeColor` (premultiplied): the shape's fill and stroke at distance `d` pixels; `q` the
/// point (a ring's dot).
#[allow(clippy::too_many_arguments)]
fn shape_color(
    shape: u32,
    d: f64,
    q: [f64; 2],
    hs: [f64; 2],
    fill: V4,
    stroke_in: V4,
    sw_in: f64,
    dpr: f64,
) -> V4 {
    let open = is_open(shape);
    let stroke = if stroke_in[3] > 0.0 {
        stroke_in
    } else if open {
        fill
    } else {
        [0.0; 4]
    };
    let sw = if sw_in <= 0.0 && open {
        dpr.max(1.0)
    } else {
        sw_in
    };
    let pre = |c: V4, a: f64| [c[0] * c[3] * a, c[1] * c[3] * a, c[2] * c[3] * a, c[3] * a];
    let mix = |top: V4, under: V4| {
        [
            top[0] + under[0] * (1.0 - top[3]),
            top[1] + under[1] * (1.0 - top[3]),
            top[2] + under[2] * (1.0 - top[3]),
            top[3] + under[3] * (1.0 - top[3]),
        ]
    };
    let mut col = [0.0; 4];
    if !open && fill[3] > 0.0 {
        col = pre(fill, clamp01(0.5 - d));
    }
    if stroke[3] > 0.0 && sw > 0.0 {
        let sa = clamp01(sw * 0.5 + 0.5 - d.abs());
        col = mix(pre(stroke, sa), col);
    }
    if shape == 1 && stroke[3] > 0.0 {
        let dt = clamp01((1.2 * dpr).max(hs[0] * 0.16) + 0.5 - length(q));
        col = mix(pre(stroke, dt), col);
    }
    col
}

// ── Markers (`marker.wgsl`) ───────────────────────────────────────────────

fn markers(
    pixmap: &mut Pixmap,
    view: &CpuView,
    b: &StyledBatch,
    st: &Style,
    data: &[f32],
    source: &dyn ImageSource,
) {
    let image_look = st.flags[1] == 1;
    // A picture or text marker's image, once a batch at the atlas's step, and its proportions.
    let (img, aspect) = if image_look {
        let Some(image) = b.image() else {
            return;
        };
        let Some(p) = raster::image(image, step_for(b.image_px(view.px_per_m, view.dpr)), source)
        else {
            return;
        };
        let aspect = f64::from(p.height) / f64::from(p.width.max(1));
        (Some(p), aspect)
    } else {
        (None, st.b[0])
    };
    let k = st.unit_k(view);
    let fit = st.flags[3];
    let (w, h) = (pixmap.width(), pixmap.height());
    for m in data.chunks_exact(5) {
        let mut mw = f64::from(m[3]) * k;
        let mut mh = f64::from(m[4]) * k;
        if fit == 1 {
            mh = mw * aspect;
        } else if fit == 2 {
            mw = mh / aspect.max(1e-6);
        } else if mh <= 0.0 {
            mh = mw;
        }
        let sw = if !image_look && st.b[1] > 0.0 {
            (st.b[1] * k).max(1.0)
        } else {
            0.0
        };
        let pad = sw * 0.5 + 1.5;
        let mut bx = [mw, mh];
        if !image_look && st.flags[2] == 5 {
            bx[1] = mh.max(mw.min(mh) * 1.154_700_5);
        }
        let hs = [mw * 0.5, mh * 0.5];
        // Where the shader puts the quad: the point, then the anchor and offset turned with it.
        let at = view.px(f64::from(m[0]) + b.origin[0], f64::from(m[1]) + b.origin[1]);
        let (sa, ca) = f64::from(m[2]).sin_cos();
        let shift = [-st.a[2] * mw + st.a[0] * k, -st.a[3] * mh + st.a[1] * k];
        let to_px = |l: [f64; 2]| {
            let q = [l[0] + shift[0], l[1] + shift[1]];
            let r = [ca * q[0] - sa * q[1], sa * q[0] + ca * q[1]];
            let d = view.lin(r);
            [at[0] + d[0], at[1] + d[1]]
        };
        let ex = [bx[0] * 0.5 + pad, bx[1] * 0.5 + pad];
        let quad = [
            to_px([-ex[0], -ex[1]]),
            to_px([ex[0], -ex[1]]),
            to_px([ex[0], ex[1]]),
            to_px([-ex[0], ex[1]]),
        ];
        let mut lit: Vec<(u32, u32, V4)> = Vec::new();
        rows_of(&quad, w, h, |px, py| {
            // The pixel in the marker's own frame (pixels, y up, from the quad's middle).
            let d = view.unlin([f64::from(px) + 0.5 - at[0], f64::from(py) + 0.5 - at[1]]);
            let r = [ca * d[0] + sa * d[1], -sa * d[0] + ca * d[1]];
            let local = [r[0] - shift[0], r[1] - shift[1]];
            if local[0].abs() > ex[0] || local[1].abs() > ex[1] {
                return;
            }
            let col = match &img {
                Some(p) => {
                    let t = [
                        local[0] / (2.0 * hs[0]) + 0.5,
                        local[1] / (2.0 * hs[1]) + 0.5,
                    ];
                    if t[0] < 0.0 || t[0] > 1.0 || t[1] < 0.0 || t[1] > 1.0 {
                        return;
                    }
                    let pad = f64::from(raster::PAD);
                    sample(
                        p,
                        pad + t[0] * f64::from(p.width),
                        pad + (1.0 - t[1]) * f64::from(p.height),
                    )
                }
                None => {
                    let shape = st.flags[2];
                    shape_color(
                        shape,
                        shape_dist(shape, local, hs, st.c),
                        local,
                        hs,
                        st.color,
                        st.stroke,
                        sw,
                        view.dpr,
                    )
                }
            };
            let o = st.b[2];
            let c = [col[0] * o, col[1] * o, col[2] * o, col[3] * o];
            if c[3] >= 0.004 {
                lit.push((px, py, c));
            }
        });
        for (px, py, c) in lit {
            over(pixmap, px, py, c);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use kentos_native_style::batches::{
        AtlasImage, BatchKind, Cap, FillPaintBatch, MarkerLook, StyledBatch, StyledLayer, TileMark,
        Unit,
    };

    use super::*;
    use crate::styled::picture::{NoImages, Picture, TextOutline};

    /// A picture 40 × 40 of a 40 m square of ground at 1 m a pixel, north up: (x, y) metres land on
    /// (x, 40 − y).
    fn view() -> CpuView {
        CpuView {
            m: [1.0, 0.0, 0.0, -1.0],
            t: [0.0, 40.0],
            px_per_m: 1.0,
            dpr: 1.0,
            scale: 1000.0,
        }
    }

    fn batch(kind: BatchKind, range: std::ops::Range<usize>) -> StyledBatch {
        StyledBatch {
            range,
            kind,
            level: 0.0,
            key: 0,
            bounds: [-100.0, -100.0, 100.0, 100.0],
            origin: [0.0, 0.0],
            reach: 0.0,
            reach_unit: Unit::World,
            min_scale: None,
            max_scale: None,
        }
    }

    /// The square (10, 10)–(30, 30) as two triangles.
    const SQUARE: [f32; 12] = [
        10.0, 10.0, 30.0, 10.0, 30.0, 30.0, 10.0, 10.0, 30.0, 30.0, 10.0, 30.0,
    ];

    fn shot(layer: &StyledLayer) -> Vec<u8> {
        paint(40, 40, &view(), &[layer], &NoImages).expect("a picture")
    }

    fn px(img: &[u8], x: usize, y: usize) -> [u8; 4] {
        let i = (y * 40 + x) * 4;
        [img[i], img[i + 1], img[i + 2], img[i + 3]]
    }

    #[test]
    fn a_solid_fill_covers_its_triangles() {
        let layer = StyledLayer {
            data: SQUARE.to_vec(),
            batches: vec![batch(
                BatchKind::Fill {
                    paint: FillPaintBatch::Solid {
                        color: [0.2, 0.4, 0.8, 1.0],
                    },
                },
                0..12,
            )],
        };
        let img = shot(&layer);
        assert_eq!(px(&img, 20, 20), [51, 102, 204, 255]);
        assert_eq!(px(&img, 5, 5)[3], 0, "outside");
        // Ground y up, rows down: the square's top edge is at row 40 − 30 = 10.
        assert_eq!(px(&img, 20, 9)[3], 0);
        assert_eq!(px(&img, 20, 10)[3], 255);
    }

    /// A pattern of discs 2 m wide every 5 m: a disc's centre inked, between them clear; cells
    /// too small to draw (under 4 pixels) are an even tint of the share they ink.
    #[test]
    fn a_pattern_draws_its_shape_on_its_grid_or_a_tint() {
        let pattern = |size: f64| FillPaintBatch::Pattern {
            shape: "circle".into(),
            fill: Some([0.0, 0.5, 0.0, 1.0]),
            stroke: None,
            stroke_width: 0.0,
            half: [1.0, 1.0],
            mark_offset: [0.0, 0.0],
            mark_rotation: 0.0,
            params: [0.0; 4],
            size: [size, size],
            stagger: false,
            angle: 0.0,
            offset: [0.0, 0.0],
            jitter: 0.0,
            coverage: 1.0,
            seed: 0.0,
            tint: 0.25,
            opacity: 1.0,
            unit: Unit::World,
        };
        let layer = StyledLayer {
            data: SQUARE.to_vec(),
            batches: vec![batch(
                BatchKind::Fill {
                    paint: pattern(5.0),
                },
                0..12,
            )],
        };
        let img = shot(&layer);
        // Cells of 5 m from the batch's origin: one centred at (12.5, 12.5), row 40 − 12.5.
        assert_eq!(px(&img, 12, 27), [0, 128, 0, 255], "a disc's middle");
        assert_eq!(px(&img, 15, 24)[3], 0, "between the discs");
        let layer = StyledLayer {
            data: SQUARE.to_vec(),
            batches: vec![batch(
                BatchKind::Fill {
                    paint: pattern(2.0),
                },
                0..12,
            )],
        };
        let img = shot(&layer);
        let tint = px(&img, 20, 20);
        assert!(
            tint[3] == 64 && (126..=130).contains(&tint[1]) && tint[0] == 0,
            "a tint of the share it inks: {tint:?}"
        );
    }

    /// A hatch of 2 m lines every 4 m, along east: inked on the lines only.
    #[test]
    fn a_hatch_inks_its_lines() {
        let layer = StyledLayer {
            data: SQUARE.to_vec(),
            batches: vec![batch(
                BatchKind::Fill {
                    paint: FillPaintBatch::Hatch {
                        color: [0.0, 0.0, 0.0, 1.0],
                        angle: 0.0,
                        spacing: 4.0,
                        width: 2.0,
                        offset: 0.0,
                        dash: None,
                        dash_offset: 0.0,
                        stagger: 0.0,
                        unit: Unit::World,
                    },
                },
                0..12,
            )],
        };
        let img = shot(&layer);
        // Lines along x where y is a whole number of 4 m: y = 12, 16, 20 … (rows 28, 24, 20 …).
        let inked: Vec<u8> = (11..30).map(|y| px(&img, 20, 40 - y)[3]).collect();
        assert!(
            inked.iter().any(|a| *a > 200) && inked.contains(&0),
            "{inked:?}"
        );
    }

    /// A staggered family's dashes (docs/adr/0186 §3): 2 m drawn, 6 m apart, each line's 4 m
    /// further along than the one under it; a dot (0 long) inks a pixel.
    #[test]
    fn a_staggered_hatch_moves_its_dashes_line_by_line() {
        let hatch = |dash: Vec<f64>| StyledLayer {
            data: SQUARE.to_vec(),
            batches: vec![batch(
                BatchKind::Fill {
                    paint: FillPaintBatch::Hatch {
                        color: [0.0, 0.0, 0.0, 1.0],
                        angle: 0.0,
                        spacing: 4.0,
                        width: 2.0,
                        offset: 0.0,
                        dash: Some(dash),
                        dash_offset: 0.0,
                        stagger: 4.0,
                        unit: Unit::World,
                    },
                },
                0..12,
            )],
        };
        let img = shot(&hatch(vec![2.0, 6.0]));
        // Line y = 16 (row 4) draws x 16…18, 24…26 (the 4th line, 16 m along); line y = 20 (row 5) 20…22, 28…30.
        let row = |y: usize| -> Vec<usize> {
            (10..30).filter(|&x| px(&img, x, 40 - y)[3] > 128).collect()
        };
        assert_eq!(row(16), [16, 17, 24, 25]);
        assert_eq!(row(20), [12, 13, 20, 21, 28, 29]);
        let dots = shot(&hatch(vec![0.0, 4.0]));
        let inked = (10..30).filter(|&x| px(&dots, x, 40 - 16)[3] > 0).count();
        assert!((4..=10).contains(&inked), "dots: {inked}");
    }

    /// A linear gradient from black on the west to white on the east; a sphere's middle is the second colour.
    #[test]
    fn a_gradient_runs_from_its_first_colour_to_its_second() {
        let gradient = |shape: u32| StyledLayer {
            data: SQUARE.to_vec(),
            batches: vec![batch(
                BatchKind::Fill {
                    paint: FillPaintBatch::Gradient {
                        color: [0.0, 0.0, 0.0, 1.0],
                        color2: [1.0, 1.0, 1.0, 1.0],
                        shape,
                        inverted: false,
                        dir: 0.0,
                        from: 10.0,
                        to: 30.0,
                        centre: [20.0, 20.0],
                        radius: 200.0_f64.sqrt(),
                    },
                },
                0..12,
            )],
        };
        let img = shot(&gradient(0));
        let (west, middle, east) = (
            px(&img, 10, 20)[0],
            px(&img, 20, 20)[0],
            px(&img, 29, 20)[0],
        );
        assert!(
            west < 20 && (110..=150).contains(&middle) && east > 235,
            "{west} {middle} {east}"
        );
        let sphere = shot(&gradient(2));
        assert!(px(&sphere, 20, 20)[0] > 240 && px(&sphere, 10, 29)[0] < 30);
    }

    /// A soft-edged line fades over its blur, a hard one does not.
    #[test]
    fn a_soft_line_fades_over_its_blur() {
        let line = |blur: f64| StyledLayer {
            // From (5, 20) to (35, 20), a path of its own (both ends capped).
            data: vec![5.0, 20.0, 35.0, 20.0, 0.0, 3.0],
            batches: vec![batch(
                BatchKind::Stroke {
                    color: [1.0, 0.0, 0.0, 1.0],
                    width: 2.0,
                    unit: Unit::World,
                    dash: None,
                    dash_offset: 0.0,
                    cap: Cap::Butt,
                    blur,
                },
                0..6,
            )],
        };
        let hard = shot(&line(0.0));
        let soft = shot(&line(6.0));
        assert_eq!(px(&hard, 20, 20)[3], 255);
        assert_eq!(px(&hard, 20, 23)[3], 0, "a hard edge");
        assert!(px(&soft, 20, 20)[3] > 100, "{:?}", px(&soft, 20, 20));
        let (near, far) = (px(&soft, 20, 22)[3], px(&soft, 20, 23)[3]);
        assert!(near > far && far > 0, "a fading edge: {near} {far}");
        // A butt cap: nothing before the line's start.
        assert_eq!(px(&hard, 3, 20)[3], 0);
    }

    /// A shape marker: its distance field filled at the point, turned and anchored.
    #[test]
    fn a_shape_marker_is_its_distance_field() {
        let layer = StyledLayer {
            // At (20, 20), 10 m wide.
            data: vec![20.0, 20.0, 0.0, 10.0, 0.0],
            batches: vec![batch(
                BatchKind::Marker {
                    unit: Unit::World,
                    look: MarkerLook::Shape {
                        shape: "square".into(),
                        fill: Some([1.0, 0.5, 0.0, 1.0]),
                        stroke: None,
                        stroke_width: 0.0,
                        params: [0.0; 4],
                    },
                    offset: [0.0, 0.0],
                    anchor: [0.0, 0.0],
                    opacity: 1.0,
                    extent: [10.0, 10.0],
                },
                0..5,
            )],
        };
        let img = shot(&layer);
        assert_eq!(px(&img, 20, 20), [255, 128, 0, 255]);
        assert_eq!(px(&img, 16, 16)[3], 255, "inside the square");
        assert_eq!(px(&img, 27, 20)[3], 0, "past its side");
    }

    /// An image marker samples the atlas's image (a 2 × 1 picture: red left, blue right), north up.
    #[test]
    fn a_picture_marker_draws_its_image() {
        struct TwoColours;
        impl ImageSource for TwoColours {
            fn picture(&self, _: &AtlasImage) -> Option<Arc<Picture>> {
                Some(Arc::new(Picture::Bitmap {
                    width: 2,
                    height: 1,
                    rgba: vec![255, 0, 0, 255, 0, 0, 255, 255],
                }))
            }
            fn text(&self, _: &str, _: Option<&str>, _: f64, _: bool) -> Option<Arc<TextOutline>> {
                None
            }
        }
        let layer = StyledLayer {
            data: vec![20.0, 20.0, 0.0, 20.0, 10.0],
            batches: vec![batch(
                BatchKind::Marker {
                    unit: Unit::World,
                    look: MarkerLook::Image {
                        image: AtlasImage::Raster {
                            key: "iki".into(),
                            url: String::new(),
                            width: 2.0,
                            height: 1.0,
                        },
                        fit_height: false,
                    },
                    offset: [0.0, 0.0],
                    anchor: [0.0, 0.0],
                    opacity: 1.0,
                    extent: [20.0, 10.0],
                },
                0..5,
            )],
        };
        let img = paint(40, 40, &view(), &[&layer], &TwoColours).unwrap();
        let (left, right) = (px(&img, 13, 20), px(&img, 27, 20));
        assert!(left[0] > 200 && left[2] < 50, "{left:?}");
        assert!(right[2] > 200 && right[0] < 50, "{right:?}");
        assert_eq!(px(&img, 5, 20)[3], 0, "outside the picture");
    }

    /// A picture fill repeats the atlas's tile (here discs on a grid, drawn without images).
    #[test]
    fn a_picture_fill_repeats_its_tile() {
        let disc = TileMark {
            look: MarkerLook::Shape {
                shape: "circle".into(),
                fill: Some([0.0, 0.0, 1.0, 1.0]),
                stroke: None,
                stroke_width: 0.0,
                params: [0.0; 4],
            },
            w: 0.5,
            h: 0.5,
            offset: [0.0, 0.0],
            rotation: 0.0,
        };
        let layer = StyledLayer {
            data: SQUARE.to_vec(),
            batches: vec![batch(
                BatchKind::Fill {
                    paint: FillPaintBatch::Tile {
                        image: AtlasImage::Tile {
                            key: "disk".into(),
                            aspect: 1.0,
                            stagger: false,
                            draw: vec![disc],
                        },
                        size: [8.0, 8.0],
                        angle: 0.0,
                        offset: [0.0, 0.0],
                        opacity: 1.0,
                        unit: Unit::World,
                    },
                },
                0..12,
            )],
        };
        let img = shot(&layer);
        // Tiles of 8 m from the origin: one centred at (12, 12) → row 28; its corner clear.
        let middle = px(&img, 12, 28);
        assert!(middle[2] > 150 && middle[3] > 150, "{middle:?}");
        assert!(px(&img, 16, 24)[3] < 60, "{:?}", px(&img, 16, 24));
    }

    /// A batch out of its scale range, or out of the picture, is not drawn (the GPU's checks).
    #[test]
    fn what_the_gpu_would_not_draw_is_not_drawn() {
        let mut b = batch(
            BatchKind::Fill {
                paint: FillPaintBatch::Solid {
                    color: [0.0, 0.0, 0.0, 1.0],
                },
            },
            0..12,
        );
        b.max_scale = Some(500.0);
        let layer = StyledLayer {
            data: SQUARE.to_vec(),
            batches: vec![b],
        };
        assert_eq!(px(&shot(&layer), 20, 20)[3], 0, "only up to 1:500");
    }

    #[test]
    fn the_steps_are_the_atlas_s() {
        assert_eq!(step_for(3.0), 8);
        assert_eq!(step_for(100.0), 128);
        assert_eq!(step_for(5000.0), 512);
    }
}
