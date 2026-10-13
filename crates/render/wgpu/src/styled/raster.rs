//! The atlas's images drawn on the CPU with tiny-skia (docs/adr/0090): as
//! the web's atlas draws them with Canvas2D (`render/atlas.ts`,
//! `canvasShapes.ts`), so the two platforms' markers, texts and pattern tiles
//! look the same. Output is premultiplied RGBA with a transparent border, as
//! the texture holds it.

use tiny_skia::{
    FillRule, FilterQuality, Mask, MaskType, Paint, Path, PathBuilder, Pixmap, PixmapPaint,
    Transform,
};

use kentos_native_style::batches::{AtlasImage, MarkerLook, TileMark};

use super::picture::{
    Fill, ImageSource, LineCap, LineJoin, Matrix, Node, Picture, Segment, Stroke, TextOutline,
};

/// Transparent border around every image: linear filtering never reads a neighbour.
pub const PAD: u32 = 2;
/// No side of one image is longer (a long text at a large step is scaled down).
pub const MAX_SIDE: f32 = 1024.0;
/// Height of a text marker's box relative to its font size (`TEXT_BOX`).
const TEXT_BOX: f32 = 1.25;

/// An image as drawn: its size without the border, and the pixels with it.
pub struct Painted {
    pub width: u32,
    pub height: u32,
    /// (width + 2·PAD) × (height + 2·PAD) pixels, premultiplied RGBA.
    pub data: Vec<u8>,
}

fn matrix(m: &Matrix) -> Transform {
    Transform::from_row(m[0], m[1], m[2], m[3], m[4], m[5])
}

/// A path of segments; None when it has nothing to draw.
pub fn path_of(segments: &[Segment]) -> Option<Path> {
    let mut b = PathBuilder::new();
    for s in segments {
        match *s {
            Segment::Move([x, y]) => b.move_to(x, y),
            Segment::Line([x, y]) => b.line_to(x, y),
            Segment::Quad([x1, y1], [x, y]) => b.quad_to(x1, y1, x, y),
            Segment::Cubic([x1, y1], [x2, y2], [x, y]) => b.cubic_to(x1, y1, x2, y2, x, y),
            Segment::Close => b.close(),
        }
    }
    b.finish()
}

fn paint(color: [u8; 4]) -> Paint<'static> {
    let mut p = Paint::default();
    p.set_color_rgba8(color[0], color[1], color[2], color[3]);
    p.anti_alias = true;
    p
}

fn stroke_of(s: &Stroke) -> tiny_skia::Stroke {
    tiny_skia::Stroke {
        width: s.width.max(0.0),
        miter_limit: s.miter.max(1.0),
        line_cap: match s.cap {
            LineCap::Butt => tiny_skia::LineCap::Butt,
            LineCap::Round => tiny_skia::LineCap::Round,
            LineCap::Square => tiny_skia::LineCap::Square,
        },
        line_join: match s.join {
            LineJoin::Miter => tiny_skia::LineJoin::Miter,
            LineJoin::Round => tiny_skia::LineJoin::Round,
            LineJoin::Bevel => tiny_skia::LineJoin::Bevel,
        },
        dash: s
            .dash
            .as_ref()
            .and_then(|(d, offset)| tiny_skia::StrokeDash::new(d.clone(), *offset)),
    }
}

/// Draws a picture's nodes under `base` (the picture's place in the image).
fn draw_nodes(pixmap: &mut Pixmap, nodes: &[Node], base: Transform, mask: Option<&Mask>) {
    for node in nodes {
        match node {
            Node::Path {
                segments,
                transform,
                fill,
                stroke,
            } => {
                let Some(path) = path_of(segments) else {
                    continue;
                };
                let t = base.pre_concat(matrix(transform));
                if let Some(Fill { color, even_odd }) = fill {
                    let rule = if *even_odd {
                        FillRule::EvenOdd
                    } else {
                        FillRule::Winding
                    };
                    pixmap.fill_path(&path, &paint(*color), rule, t, mask);
                }
                if let Some(s) = stroke.as_ref().filter(|s| s.width > 0.0) {
                    pixmap.stroke_path(&path, &paint(s.color), &stroke_of(s), t, mask);
                }
            }
            Node::Group {
                opacity,
                mask: group_mask,
                children,
            } => {
                let plain = *opacity >= 1.0 && group_mask.is_none();
                if plain {
                    draw_nodes(pixmap, children, base, mask);
                    continue;
                }
                // A layer the size of the image: the children, then cut and faded as one.
                let Some(mut layer) = Pixmap::new(pixmap.width(), pixmap.height()) else {
                    continue;
                };
                draw_nodes(&mut layer, children, base, None);
                let own = group_mask.as_ref().and_then(|m| {
                    let mut luminance = Pixmap::new(pixmap.width(), pixmap.height())?;
                    draw_nodes(&mut luminance, m, base, None);
                    Some(Mask::from_pixmap(luminance.as_ref(), MaskType::Luminance))
                });
                if let Some(own) = &own {
                    layer.apply_mask(own);
                }
                let p = PixmapPaint {
                    opacity: opacity.clamp(0.0, 1.0),
                    ..PixmapPaint::default()
                };
                pixmap.draw_pixmap(0, 0, layer.as_ref(), &p, Transform::identity(), mask);
            }
        }
    }
}

/// Where a vector picture's viewBox lands in a `w` × `h` image (`xMidYMid meet`).
fn fit(view: [f32; 4], w: f32, h: f32) -> Transform {
    let [vx, vy, vw, vh] = view;
    if !(vw > 0.0 && vh > 0.0) {
        return Transform::identity();
    }
    let k = (w / vw).min(h / vh);
    let tx = (w - vw * k) / 2.0 - vx * k;
    let ty = (h - vh * k) / 2.0 - vy * k;
    Transform::from_row(k, 0.0, 0.0, k, tx, ty)
}

/// Straight-alpha RGBA pixels as a pixmap (premultiplied).
fn bitmap(width: u32, height: u32, rgba: &[u8]) -> Option<Pixmap> {
    let mut p = Pixmap::new(width, height)?;
    let data = p.data_mut();
    if data.len() != rgba.len() {
        return None;
    }
    for (out, px) in data.chunks_exact_mut(4).zip(rgba.chunks_exact(4)) {
        let a = u32::from(px[3]);
        out[0] = ((u32::from(px[0]) * a + 127) / 255) as u8;
        out[1] = ((u32::from(px[1]) * a + 127) / 255) as u8;
        out[2] = ((u32::from(px[2]) * a + 127) / 255) as u8;
        out[3] = px[3];
    }
    Some(p)
}

/// Draws a picture into `pixmap` filling the box `w` × `h` at `at`.
fn draw_picture(pixmap: &mut Pixmap, picture: &Picture, at: Transform, w: f32, h: f32) {
    match picture {
        Picture::Vector { view, nodes } => {
            draw_nodes(pixmap, nodes, at.pre_concat(fit(*view, w, h)), None);
        }
        Picture::Bitmap {
            width,
            height,
            rgba,
        } => {
            let Some(src) = bitmap(*width, *height, rgba) else {
                return;
            };
            let sx = w / (*width).max(1) as f32;
            let sy = h / (*height).max(1) as f32;
            let p = PixmapPaint {
                quality: FilterQuality::Bicubic,
                ..PixmapPaint::default()
            };
            pixmap.draw_pixmap(0, 0, src.as_ref(), &p, at.pre_scale(sx, sy), None);
        }
    }
}

/// A picture drawn filling `width` × `height` pixels, on `background` or
/// transparent, as straight-alpha RGBA: the SVG editor's PNG export and the
/// pixels it traces (no side limit but the one the caller keeps).
pub fn picture_pixels(
    picture: &Picture,
    width: u32,
    height: u32,
    background: Option<[u8; 4]>,
) -> Option<Vec<u8>> {
    let mut p = Pixmap::new(width.max(1), height.max(1))?;
    if let Some([r, g, b, a]) = background {
        p.fill(tiny_skia::Color::from_rgba8(r, g, b, a));
    }
    draw_picture(
        &mut p,
        picture,
        Transform::identity(),
        width.max(1) as f32,
        height.max(1) as f32,
    );
    let mut out = p.take();
    for px in out.chunks_exact_mut(4) {
        let a = u32::from(px[3]);
        if a > 0 && a < 255 {
            for c in &mut px[..3] {
                *c = ((u32::from(*c) * 255 + a / 2) / a).min(255) as u8;
            }
        }
    }
    Some(out)
}

fn padded(w: u32, h: u32) -> Option<Pixmap> {
    Pixmap::new(w + 2 * PAD, h + 2 * PAD)
}

fn painted(pixmap: Pixmap, width: u32, height: u32) -> Painted {
    Painted {
        width,
        height,
        data: pixmap.take(),
    }
}

/// An atlas image drawn at `step` pixels (`Atlas.make`): a text marker at that height, an SVG or
/// raster picture and a pattern tile at that width. The GPU's atlas and the CPU's map pictures
/// (`cpu.rs`) draw their images here.
pub fn image(image: &AtlasImage, step: u32, source: &dyn ImageSource) -> Option<Painted> {
    match image {
        AtlasImage::Text {
            text,
            font,
            weight,
            italic,
            color,
            halo,
            ..
        } => {
            let outline = source.text(text, font.as_deref(), *weight, *italic)?;
            text_image(&outline, color, halo.as_ref(), step)
        }
        AtlasImage::Svg { width, height, .. } | AtlasImage::Raster { width, height, .. } => {
            let picture = source.picture(image)?;
            picture_image(&picture, *width, *height, step)
        }
        AtlasImage::Tile {
            aspect,
            stagger,
            draw,
            ..
        } => tile_image(*aspect, *stagger, draw, step, source),
    }
}

/// An SVG or raster image at `step` pixels wide (its height follows its proportions).
pub fn picture_image(picture: &Picture, image_w: f64, image_h: f64, step: u32) -> Option<Painted> {
    let step = step as f32;
    let aspect = image_h as f32 / (image_w as f32).max(1e-9);
    let k = (MAX_SIDE / step.max(step * aspect)).min(1.0);
    let w = (step * k).round().max(1.0) as u32;
    let h = (step * aspect * k).round().max(1.0) as u32;
    let mut p = padded(w, h)?;
    draw_picture(
        &mut p,
        picture,
        Transform::from_translate(PAD as f32, PAD as f32),
        w as f32,
        h as f32,
    );
    Some(painted(p, w, h))
}

fn rgba8(hex: &str) -> [u8; 4] {
    let c = kentos_native_style::color::parse_hex(hex, 1.0);
    let v = |x: f64| {
        if x.is_finite() {
            (x.clamp(0.0, 1.0) * 255.0).round() as u8
        } else {
            0
        }
    };
    [v(c[0]), v(c[1]), v(c[2]), v(c[3])]
}

fn rgba_f(c: &[f64; 4]) -> [u8; 4] {
    let v = |x: f64| {
        if x.is_finite() {
            (x.clamp(0.0, 1.0) * 255.0).round() as u8
        } else {
            0
        }
    };
    [v(c[0]), v(c[1]), v(c[2]), v(c[3])]
}

/// Glyph outlines placed at `(x, y)` (baseline) and scaled to `size`.
fn glyph_paths(outline: &TextOutline, size: f32) -> (Vec<Path>, f32) {
    let k = size / outline.size.max(1e-6);
    let paths = outline.glyphs.iter().filter_map(|g| path_of(g)).collect();
    (paths, outline.advance * k)
}

/// A text marker's image at `step` pixels high (`Atlas.make`, kind text):
/// the letters at `step / 1.25`, the baseline at 78 % of the box, a halo
/// stroke under the fill.
pub fn text_image(
    outline: &TextOutline,
    color: &str,
    halo: Option<&(String, f64)>,
    step: u32,
) -> Option<Painted> {
    let step = step as f32;
    let size = step / TEXT_BOX;
    let (paths, advance) = glyph_paths(outline, size);
    let halo_w = halo.map_or(0.0, |(_, w)| *w as f32 * size);
    let tw = advance + 2.0 * halo_w;
    let k = (MAX_SIDE / tw.max(step)).min(1.0);
    let w = (tw * k).ceil().max(1.0) as u32;
    let h = (step * k).ceil().max(1.0) as u32;
    let mut p = padded(w, h)?;
    let s = size / outline.size.max(1e-6);
    let place = Transform::from_translate(PAD as f32, PAD as f32)
        .pre_scale(k, k)
        .pre_translate(halo_w, step * 0.78)
        .pre_scale(s, s);
    if let Some((halo_color, _)) = halo {
        let stroke = tiny_skia::Stroke {
            width: halo_w * 2.0 / s,
            line_join: tiny_skia::LineJoin::Round,
            ..tiny_skia::Stroke::default()
        };
        let hp = paint(rgba8(halo_color));
        for path in &paths {
            p.stroke_path(path, &hp, &stroke, place, None);
        }
    }
    let fp = paint(rgba8(color));
    for path in &paths {
        p.fill_path(path, &fp, FillRule::Winding, place, None);
    }
    Some(painted(p, w, h))
}

// The shapes (the web's `canvasShapes.ts`) are the style crate's: a PDF writes their outlines too.
use kentos_native_style::shapes::{OPEN, arc, shape_path};

/// Draws a shape look centred at the transform's origin (y up) (`drawShape`).
fn draw_shape(p: &mut Pixmap, look: &MarkerLook, w: f32, h: f32, stroke_width: f32, at: Transform) {
    let MarkerLook::Shape {
        shape,
        fill,
        stroke,
        params,
        ..
    } = look
    else {
        return;
    };
    let params = params.map(|x| x as f32);
    let hh = if h != 0.0 { h } else { w } / 2.0;
    let Some(path) = shape_path(shape, w / 2.0, hh, params) else {
        return;
    };
    let open = OPEN.contains(&shape.as_str());
    let stroke_color = stroke.or(if open { *fill } else { None });
    if !open && let Some(fill) = fill {
        p.fill_path(&path, &paint(rgba_f(fill)), FillRule::EvenOdd, at, None);
    }
    if let Some(sc) = stroke_color {
        let s = tiny_skia::Stroke {
            width: stroke_width.max(0.0001),
            line_join: tiny_skia::LineJoin::Miter,
            ..tiny_skia::Stroke::default()
        };
        p.stroke_path(&path, &paint(rgba_f(&sc)), &s, at, None);
    }
    if shape == "ring"
        && let Some(sc) = stroke_color
    {
        let d = stroke_width.max(w.min(if h != 0.0 { h } else { w }) * 0.08);
        let mut b = PathBuilder::new();
        arc(&mut b, d, 0.0, 2.0 * std::f32::consts::PI, true);
        b.close();
        if let Some(dot) = b.finish() {
            p.fill_path(&dot, &paint(rgba_f(&sc)), FillRule::Winding, at, None);
        }
    }
}

/// One mark of a tile at `(cx, cy)` (pixels, y down), `W` the tile width (`drawMark`).
fn draw_mark(
    p: &mut Pixmap,
    m: &TileMark,
    cx: f32,
    cy: f32,
    tile_w: f32,
    source: &dyn ImageSource,
    base: Transform,
) {
    let w = m.w as f32 * tile_w;
    let h = (if m.h != 0.0 { m.h } else { m.w }) as f32 * tile_w;
    // Canvas: translate, rotate(-rotation), scale(1, -1): y up from here.
    let at = base
        .pre_translate(cx, cy)
        .pre_concat(Transform::from_rotate(-(m.rotation as f32).to_degrees()))
        .pre_scale(1.0, -1.0);
    match &m.look {
        look @ MarkerLook::Shape { stroke_width, .. } => {
            draw_shape(p, look, w, h, *stroke_width as f32 * tile_w, at);
        }
        MarkerLook::Image { image, .. } => match image {
            AtlasImage::Text {
                text,
                font,
                weight,
                italic,
                color,
                ..
            } => {
                let Some(outline) = source.text(text, font.as_deref(), *weight, *italic) else {
                    return;
                };
                let (paths, advance) = glyph_paths(&outline, h);
                let s = h / outline.size.max(1e-6);
                // Centred, the em box's middle on the point (`textBaseline = 'middle'`).
                let place = at
                    .pre_scale(1.0, -1.0)
                    .pre_translate(-advance / 2.0, 0.35 * h)
                    .pre_scale(s, s);
                let fp = paint(rgba8(color));
                for path in &paths {
                    p.fill_path(path, &fp, FillRule::Winding, place, None);
                }
            }
            AtlasImage::Svg { width, height, .. } | AtlasImage::Raster { width, height, .. } => {
                let Some(picture) = source.picture(image) else {
                    return;
                };
                let ih = w * (*height as f32 / (*width as f32).max(1.0));
                let place = at.pre_scale(1.0, -1.0).pre_translate(-w / 2.0, -ih / 2.0);
                draw_picture(p, &picture, place, w, ih);
            }
            AtlasImage::Tile { .. } => {}
        },
    }
}

/// A pattern tile `step` pixels wide: each mark at the cell centre (and
/// half-shifted for staggered rows), wrapped at the edges so tiles join (`paintTile`).
pub fn tile_image(
    aspect: f64,
    stagger: bool,
    draw: &[TileMark],
    step: u32,
    source: &dyn ImageSource,
) -> Option<Painted> {
    let w = step;
    let h = ((step as f64 * aspect).round() as f32).clamp(2.0, MAX_SIDE) as u32;
    let mut p = padded(w, h)?;
    let (tw, th) = (w as f32, h as f32);
    let base = Transform::from_translate(PAD as f32, PAD as f32);
    // Clip to the tile: draw into a tile-sized pixmap, then place it inside the border.
    let mut tile = Pixmap::new(w, h)?;
    let centres: &[(f32, f32)] = if stagger {
        &[(0.5, 0.25), (0.0, 0.75)]
    } else {
        &[(0.5, 0.5)]
    };
    for m in draw {
        for (cx, cy) in centres {
            for dx in [-tw, 0.0, tw] {
                for dy in [-th, 0.0, th] {
                    draw_mark(
                        &mut tile,
                        m,
                        cx * tw + dx + m.offset[0] as f32 * tw,
                        cy * th + dy - m.offset[1] as f32 * tw,
                        tw,
                        source,
                        Transform::identity(),
                    );
                }
            }
        }
    }
    p.draw_pixmap(0, 0, tile.as_ref(), &PixmapPaint::default(), base, None);
    Some(painted(p, w, h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::styled::picture::IDENTITY;

    fn coverage(p: &Painted) -> f64 {
        p.data
            .chunks_exact(4)
            .map(|px| f64::from(px[3]))
            .sum::<f64>()
            / 255.0
    }

    #[test]
    fn a_vector_picture_fills_its_box() {
        let square = Picture::Vector {
            view: [0.0, 0.0, 10.0, 10.0],
            nodes: vec![Node::Path {
                segments: vec![
                    Segment::Move([0.0, 0.0]),
                    Segment::Line([10.0, 0.0]),
                    Segment::Line([10.0, 10.0]),
                    Segment::Line([0.0, 10.0]),
                    Segment::Close,
                ],
                transform: IDENTITY,
                fill: Some(Fill {
                    color: [255, 0, 0, 255],
                    even_odd: false,
                }),
                stroke: None,
            }],
        };
        let p = picture_image(&square, 10.0, 10.0, 32).expect("drawn");
        assert_eq!((p.width, p.height), (32, 32));
        assert!((coverage(&p) - 32.0 * 32.0).abs() < 1.0, "{}", coverage(&p));
    }

    #[test]
    fn a_mask_cuts_its_group() {
        let rect = |x: f32, w: f32, color: [u8; 4]| Node::Path {
            segments: vec![
                Segment::Move([x, 0.0]),
                Segment::Line([x + w, 0.0]),
                Segment::Line([x + w, 10.0]),
                Segment::Line([x, 10.0]),
                Segment::Close,
            ],
            transform: IDENTITY,
            fill: Some(Fill {
                color,
                even_odd: false,
            }),
            stroke: None,
        };
        let masked = Picture::Vector {
            view: [0.0, 0.0, 10.0, 10.0],
            nodes: vec![Node::Group {
                opacity: 1.0,
                mask: Some(vec![rect(0.0, 5.0, [255, 255, 255, 255])]),
                children: vec![rect(0.0, 10.0, [0, 0, 0, 255])],
            }],
        };
        let p = picture_image(&masked, 10.0, 10.0, 20).expect("drawn");
        assert!((coverage(&p) - 200.0).abs() < 2.0, "{}", coverage(&p));
    }
}
