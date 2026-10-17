//! A turned text as a picture in its own pixels (docs/adr/0055's 17 Ekim eki): its mask, its glyphs' halo, its
//! glyphs and its underline filled by tiny-skia with exact coverage, then laid on the drawing area's pixel grid as
//! an image. Iced's canvas turns a turned text into paths and smooths their edges with four samples only: the
//! letters' edges had four shades and the halo's rim showed beside the ground (the owner's 17 October pictures).
//! Pictures are kept by what they show and where they fall inside a pixel, so a pan finds them again.

use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::Rc;

use iced::advanced::image::{FilterMethod, Handle};
use iced::widget::canvas::path::lyon_path::{self, PathEvent};
use iced::widget::canvas::{self, Frame, Path};
use iced::{Color, Point, Rectangle, Size};

/// Text pictures kept at most, and their bytes; all are let go at once when more are asked (a view at 1:1000 over a
/// parcel sheet turns about 1 300 edge lengths).
const KEPT: usize = 4_096;
const KEPT_BYTES: usize = 64 << 20;
/// The largest text picture made, in device pixels; a text larger than this on screen is drawn as paths.
const LARGEST: f32 = 2_048.0;
/// The largest picture of marks over the view (a selected dimension's bonds), in device pixels.
const LARGEST_MARKS: f32 = 8_192.0;
/// Where a text falls inside a pixel, in steps of a quarter pixel.
const STEPS: f32 = 4.0;

thread_local! {
    /// The window's device pixels per logical pixel, as the drawing area was last drawn.
    static DEVICE: Cell<f32> = const { Cell::new(1.0) };
    /// The labels are drawn turned by the view's twist: they stay paths.
    static TWISTED: Cell<bool> = const { Cell::new(false) };
    static KEPT_PICTURES: RefCell<Kept> = RefCell::new(Kept::default());
    /// The marks' last picture only: they follow the view, a pan makes a new one each frame.
    static LAST_MARKS: RefCell<Option<(Key, Rc<Picture>)>> = const { RefCell::new(None) };
}

#[derive(Default)]
struct Kept {
    pictures: HashMap<Key, Rc<Picture>>,
    bytes: usize,
}

/// The window's device pixels per logical pixel for the pictures to come.
pub(crate) fn set_device_scale(scale: f64) {
    DEVICE.with(|d| d.set(if scale > 0.0 { scale as f32 } else { 1.0 }));
}

/// Whether the pictures to come are turned by the view's twist.
pub(crate) fn set_twisted(twisted: bool) {
    TWISTED.with(|t| t.set(twisted));
}

/// What a text's picture shows and where it falls inside a pixel.
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct Key {
    pub what: String,
    pub numbers: Vec<u32>,
}

/// One part of a text, in the text's frame: its own paths and transform after the turn round the anchor.
pub(crate) enum Part<'a> {
    /// Filled.
    Fill(&'a [Path], Local, Color),
    /// Its coverage grown `radius` (logical) pixels all round: a text's halo.
    Halo(&'a [Path], Local, Color, f32),
    /// Stroked `width` wide with round joins.
    Stroke(&'a [Path], Local, Color, f32),
    /// Stroked `width` wide in dashes and gaps of these lengths.
    Dash(&'a [Path], Local, Color, f32, &'a [f32]),
}

/// A part's transform in the text's frame: its letters' width factor (x only).
#[derive(Clone, Copy)]
pub(crate) struct Local {
    pub width_factor: f32,
}

struct Picture {
    handle: Handle,
    /// Its top left corner from the anchor's pixel, in device pixels.
    left: i32,
    top: i32,
    width: u32,
    height: u32,
}

/// Draws the text at `at` (logical pixels) turned by `angle` radians: its parts in their order, as one picture.
/// False when the text is too large for a picture (the caller draws it as paths).
pub(crate) fn draw(
    frame: &mut Frame,
    key: Key,
    at: Point,
    angle: f32,
    parts: impl FnOnce(&mut dyn FnMut(Part<'_>)),
) -> bool {
    drawn(frame, key, at, angle, false, parts)
}

/// Draws marks laid out on the screen (logical pixels) as one picture. False when it is too large.
pub(crate) fn draw_on_screen(
    frame: &mut Frame,
    key: Key,
    parts: impl FnOnce(&mut dyn FnMut(Part<'_>)),
) -> bool {
    drawn(frame, key, Point::ORIGIN, 0.0, true, parts)
}

fn drawn(
    frame: &mut Frame,
    key: Key,
    at: Point,
    angle: f32,
    marks: bool,
    parts: impl FnOnce(&mut dyn FnMut(Part<'_>)),
) -> bool {
    let twisted = !marks && TWISTED.with(Cell::get);
    if twisted {
        // The view's twist would turn the picture again and sample it anew: such text is drawn as paths.
        return false;
    }
    let largest = if marks { LARGEST_MARKS } else { LARGEST };
    let scale = DEVICE.with(Cell::get);
    let (dx, dy) = (at.x * scale, at.y * scale);
    let (fx, fy) = (
        ((dx - dx.floor()) * STEPS).round() / STEPS,
        ((dy - dy.floor()) * STEPS).round() / STEPS,
    );
    let mut key = key;
    key.numbers
        .extend([angle.to_bits(), scale.to_bits(), fx.to_bits(), fy.to_bits()]);
    let found = if marks {
        LAST_MARKS.with(|m| {
            m.borrow()
                .as_ref()
                .filter(|(k, _)| *k == key)
                .map(|(_, p)| p.clone())
        })
    } else {
        KEPT_PICTURES.with(|k| k.borrow().pictures.get(&key).cloned())
    };
    let picture = match found {
        Some(p) => p,
        None => {
            let Some(p) = paint(scale, angle, fx, fy, largest, parts) else {
                return false;
            };
            let p = Rc::new(p);
            if marks {
                LAST_MARKS.with(|m| *m.borrow_mut() = Some((key, p.clone())));
            } else {
                KEPT_PICTURES.with(|k| {
                    let mut k = k.borrow_mut();
                    let bytes = (p.width * p.height * 4) as usize;
                    if k.pictures.len() >= KEPT || k.bytes + bytes > KEPT_BYTES {
                        k.pictures.clear();
                        k.bytes = 0;
                    }
                    k.bytes += bytes;
                    k.pictures.insert(key, p.clone());
                });
            }
            p
        }
    };
    let (px, py) = (
        dx.floor() as i32 + picture.left,
        dy.floor() as i32 + picture.top,
    );
    frame.draw_image(
        Rectangle::new(
            Point::new(px as f32 / scale, py as f32 / scale),
            Size::new(picture.width as f32 / scale, picture.height as f32 / scale),
        ),
        canvas::Image::new(picture.handle.clone())
            .filter_method(FilterMethod::Nearest)
            .snap(true),
    );
    true
}

/// The picture of the parts, the anchor `fx`, `fy` into its pixel; none when nothing shows or it is too large.
fn paint(
    scale: f32,
    angle: f32,
    fx: f32,
    fy: f32,
    largest: f32,
    parts: impl FnOnce(&mut dyn FnMut(Part<'_>)),
) -> Option<Picture> {
    // The parts as tiny-skia paths in device pixels from the anchor, and how to paint them.
    let mut painted: Vec<Op> = Vec::new();
    let (sin, cos) = angle.sin_cos();
    let mut add = |part: Part<'_>| {
        // What the coverage is drawn from: a fill and a halo share theirs, a stroke or dashes have their own.
        let mut drawn_as = 0u32;
        let (paths, local, color, stroke, grow) = match part {
            Part::Fill(paths, local, color) => (paths, local, color, None, 0.0),
            Part::Halo(paths, local, color, radius) => (paths, local, color, None, radius * scale),
            Part::Stroke(paths, local, color, width) => {
                drawn_as = width.to_bits();
                (paths, local, color, Some((width, None)), 0.0)
            }
            Part::Dash(paths, local, color, width, dash) => {
                drawn_as = !width.to_bits();
                (
                    paths,
                    local,
                    color,
                    Some((width, tiny_skia::StrokeDash::new(dash.to_vec(), 0.0))),
                    0.0,
                )
            }
        };
        if color.a <= 0.0 {
            return;
        }
        // The stroke is laid out in the part's frame and grows with its width factor, as the canvas's did.
        let to_screen = tiny_skia::Transform::from_row(
            local.width_factor * cos * scale,
            local.width_factor * sin * scale,
            -sin * scale,
            cos * scale,
            fx,
            fy,
        );
        let mut out = tiny_skia::PathBuilder::new();
        for path in paths {
            let Some(p) = skia_path(path.raw()) else {
                continue;
            };
            let p = match &stroke {
                Some((width, dash)) => {
                    let s = tiny_skia::Stroke {
                        width: *width,
                        line_join: tiny_skia::LineJoin::Round,
                        line_cap: if dash.is_some() {
                            tiny_skia::LineCap::Butt
                        } else {
                            tiny_skia::LineCap::Round
                        },
                        dash: dash.clone(),
                        ..tiny_skia::Stroke::default()
                    };
                    // Path::stroke leaves the dashes out; they are cut first.
                    let cut = match &s.dash {
                        Some(d) => p.dash(d, 1.0),
                        None => Some(p),
                    };
                    match cut
                        .and_then(|p| p.stroke(&s, 1.0))
                        .and_then(|o| o.transform(to_screen))
                    {
                        Some(o) => o,
                        None => continue,
                    }
                }
                None => match p.transform(to_screen) {
                    Some(o) => o,
                    None => continue,
                },
            };
            out.push_path(&p);
        }
        // One path for the part: one pass of the rasterizer, not one a glyph.
        if let Some(path) = out.finish() {
            painted.push(Op {
                paths: vec![path],
                color,
                grow,
                source: (
                    paths.as_ptr() as usize,
                    paths.len(),
                    local.width_factor.to_bits(),
                    drawn_as,
                ),
            });
        }
    };
    parts(&mut add);
    let mut bounds: Option<(f32, f32, f32, f32)> = None;
    for op in &painted {
        for p in &op.paths {
            let b = p.bounds();
            let g = op.grow;
            let b = (b.left() - g, b.top() - g, b.right() + g, b.bottom() + g);
            bounds = Some(match bounds {
                None => b,
                Some(a) => (a.0.min(b.0), a.1.min(b.1), a.2.max(b.2), a.3.max(b.3)),
            });
        }
    }
    let (l, t, r, b) = bounds?;
    let (left, top) = (l.floor() as i32 - 1, t.floor() as i32 - 1);
    let (width, height) = (
        (r.ceil() as i32 + 1 - left).max(1),
        (b.ceil() as i32 + 1 - top).max(1),
    );
    if width as f32 > largest || height as f32 > largest {
        return None;
    }
    let (w, h) = (width as u32, height as u32);
    let shift = tiny_skia::Transform::from_translate(-left as f32, -top as f32);
    // Each part's coverage once (a halo and its letters share theirs), then the parts laid over each other in
    // their order, premultiplied.
    let mut coverages: Vec<(Source, Vec<u8>)> = Vec::new();
    let mut out = vec![0f32; (w * h * 4) as usize];
    for op in &painted {
        let at = match coverages.iter().position(|(k, _)| *k == op.source) {
            Some(i) => i,
            None => {
                let mut mask = tiny_skia::Mask::new(w, h)?;
                for path in &op.paths {
                    mask.fill_path(path, tiny_skia::FillRule::Winding, true, shift);
                }
                coverages.push((op.source, mask.data().to_vec()));
                coverages.len() - 1
            }
        };
        // A halo: the coverage grown by its radius (its largest value round each pixel), as a stroke round
        // every outline would draw it at a fraction of the cost.
        let grown_coverage;
        let coverage: &[u8] = if op.grow > 0.0 {
            grown_coverage = grown(&coverages[at].1, w as usize, h as usize, op.grow);
            &grown_coverage
        } else {
            &coverages[at].1
        };
        let Color { r, g, b, a } = op.color;
        for (px, &c) in out.chunks_exact_mut(4).zip(coverage) {
            if c == 0 {
                continue;
            }
            let k = f32::from(c) / 255.0 * a;
            let rest = 1.0 - k;
            px[0] = r * k + px[0] * rest;
            px[1] = g * k + px[1] * rest;
            px[2] = b * k + px[2] * rest;
            px[3] = k + px[3] * rest;
        }
    }
    // Where nothing is painted the picture takes the first part's colour, so a smooth sample blends towards
    // it, not towards black.
    let ground = painted.first().map(|op| op.color);
    Some(Picture {
        handle: Handle::from_rgba(w, h, straight(&out, ground)),
        left,
        top,
        width: w,
        height: h,
    })
}

/// A part's paths (where and how many), its width factor and how it is drawn.
type Source = (usize, usize, u32, u32);

/// One part ready to paint: its paths in the picture's device pixels, its colour, and for a halo how far
/// (device pixels) its coverage grows.
struct Op {
    paths: Vec<tiny_skia::Path>,
    color: Color,
    grow: f32,
    /// Its paths, width factor and how they are drawn, to find a coverage drawn already.
    source: Source,
}

/// A coverage grown by `radius` pixels: each pixel the largest coverage within the radius. A radius under 2 is the
/// 3 × 3 square (its corners √2 away), taken as a row and then a column.
fn grown(data: &[u8], w: usize, h: usize, radius: f32) -> Vec<u8> {
    let reach = radius.floor().max(1.0) as isize;
    if radius < 2.0 {
        let mut rows = vec![0u8; data.len()];
        for y in 0..h {
            for x in 0..w {
                let lo = x.saturating_sub(1);
                let hi = (x + 1).min(w - 1);
                rows[y * w + x] = data[y * w + lo..=y * w + hi]
                    .iter()
                    .copied()
                    .max()
                    .unwrap_or(0);
            }
        }
        let mut out = vec![0u8; data.len()];
        for y in 0..h {
            let lo = y.saturating_sub(1);
            let hi = (y + 1).min(h - 1);
            for x in 0..w {
                out[y * w + x] = (lo..=hi).map(|yy| rows[yy * w + x]).max().unwrap_or(0);
            }
        }
        return out;
    }
    let offsets: Vec<(isize, isize)> = (-reach..=reach)
        .flat_map(|dy| (-reach..=reach).map(move |dx| (dx, dy)))
        .filter(|&(dx, dy)| ((dx * dx + dy * dy) as f32) <= radius * radius)
        .collect();
    let mut out = vec![0u8; data.len()];
    for y in 0..h as isize {
        for x in 0..w as isize {
            let mut m = 0u8;
            for &(dx, dy) in &offsets {
                let (xx, yy) = (x + dx, y + dy);
                if xx >= 0 && yy >= 0 && (xx as usize) < w && (yy as usize) < h {
                    m = m.max(data[yy as usize * w + xx as usize]);
                }
            }
            out[y as usize * w + x as usize] = m;
        }
    }
    out
}

/// Premultiplied pixels as Iced's image pipeline takes them: straight alpha in bytes; where nothing is painted,
/// `ground`'s colour.
fn straight(premultiplied: &[f32], ground: Option<Color>) -> Vec<u8> {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    let ground = ground.map_or([0, 0, 0], |c| [byte(c.r), byte(c.g), byte(c.b)]);
    let mut data = Vec::with_capacity(premultiplied.len());
    for px in premultiplied.chunks_exact(4) {
        let a = px[3];
        if a <= 0.0 {
            data.extend_from_slice(&ground);
            data.push(0);
        } else {
            data.extend_from_slice(&[byte(px[0] / a), byte(px[1] / a), byte(px[2] / a), byte(a)]);
        }
    }
    data
}

/// A lyon path as tiny-skia's.
fn skia_path(path: &lyon_path::Path) -> Option<tiny_skia::Path> {
    let mut b = tiny_skia::PathBuilder::new();
    for event in path.iter() {
        match event {
            PathEvent::Begin { at } => b.move_to(at.x, at.y),
            PathEvent::Line { to, .. } => b.line_to(to.x, to.y),
            PathEvent::Quadratic { ctrl, to, .. } => b.quad_to(ctrl.x, ctrl.y, to.x, to.y),
            PathEvent::Cubic {
                ctrl1, ctrl2, to, ..
            } => b.cubic_to(ctrl1.x, ctrl1.y, ctrl2.x, ctrl2.y, to.x, to.y),
            PathEvent::End { close, .. } => {
                if close {
                    b.close();
                }
            }
        }
    }
    b.finish()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn premultiplied_pixels_become_straight() {
        // Half-covered dark grey and nothing.
        let out = straight(
            &[0.106, 0.125, 0.149, 0.5, 0.0, 0.0, 0.0, 0.0],
            Some(Color::from_rgb8(248, 249, 250)),
        );
        assert_eq!(&out[..4], &[54, 64, 76, 128]);
        assert_eq!(&out[4..], &[248, 249, 250, 0]);
    }

    /// `cargo test --release -p kentos-desktop label_picture::tests::timing -- --ignored --nocapture`: the pictures
    /// of 1 300 edge lengths of a parcel sheet at 1:1000, 14 px high and turned, with their halos and without (a
    /// zoom makes them again; a pan finds them kept).
    #[test]
    #[ignore = "a timing, run on purpose"]
    fn timing() {
        use iced::widget::canvas::Text;
        kentos_ui::theme::typography::load();
        let texts: Vec<String> = (0..1300)
            .map(|i| format!("{}.{:03}", 10 + i % 40, i * 7 % 1000))
            .collect();
        let glyphs = |t: &str| {
            let mut out: Vec<Path> = Vec::new();
            Text {
                content: t.to_owned(),
                size: iced::Pixels(14.0),
                ..Text::default()
            }
            .draw_with(|path, _| out.push(path));
            out
        };
        let outlines: Vec<Vec<Path>> = texts.iter().map(|t| glyphs(t)).collect();
        let start = std::time::Instant::now();
        let mut made = 0;
        for (i, g) in outlines.iter().enumerate() {
            let angle = (i as f32 * 0.37).sin();
            if paint(1.0, angle, 0.25, 0.5, LARGEST, |add| {
                add(Part::Halo(
                    g,
                    Local { width_factor: 1.0 },
                    Color::WHITE,
                    1.5,
                ));
                add(Part::Fill(g, Local { width_factor: 1.0 }, Color::BLACK));
            })
            .is_some()
            {
                made += 1;
            }
        }
        let ms = start.elapsed().as_secs_f64() * 1e3;
        let start = std::time::Instant::now();
        for (i, g) in outlines.iter().enumerate() {
            let angle = (i as f32 * 0.37).sin();
            let _ = paint(1.0, angle, 0.25, 0.5, LARGEST, |add| {
                add(Part::Fill(g, Local { width_factor: 1.0 }, Color::BLACK));
            });
        }
        let letters = start.elapsed().as_secs_f64() * 1e3;
        println!(
            "1 300 eğik yazının resmi: haleyle {ms:.1} ms ({made} resim, yazı başına {:.1} µs), halesiz {letters:.1} ms",
            ms * 1e3 / 1300.0
        );
    }

    #[test]
    fn a_stroke_and_a_fill_of_the_same_paths_are_drawn_apart() {
        // A diamond's halo and its fill (a bond's place): white round it, black inside.
        let square = Path::rectangle(Point::new(0.0, 0.0), Size::new(10.0, 10.0));
        let one = std::slice::from_ref(&square);
        let plain = Local { width_factor: 1.0 };
        let p = paint(1.0, 0.0, 0.0, 0.0, LARGEST, |add| {
            add(Part::Stroke(one, plain, Color::WHITE, 4.0));
            add(Part::Fill(one, plain, Color::BLACK));
        })
        .expect("a picture");
        let Handle::Rgba { pixels, .. } = &p.handle else {
            panic!("pixels")
        };
        let at = |x: i32, y: i32| {
            let i = ((y - p.top) as u32 * p.width + (x - p.left) as u32) as usize * 4;
            [pixels[i], pixels[i + 1], pixels[i + 2], pixels[i + 3]]
        };
        assert_eq!(at(5, 5), [0, 0, 0, 255]);
        assert_eq!(at(-1, 5), [255, 255, 255, 255]);
    }

    #[test]
    fn a_turned_square_has_smooth_edges() {
        let square = Path::rectangle(Point::new(0.0, 0.0), Size::new(10.0, 10.0));
        let p = paint(1.0, 0.3, 0.25, 0.5, LARGEST, |add| {
            add(Part::Fill(
                std::slice::from_ref(&square),
                Local { width_factor: 1.0 },
                Color::BLACK,
            ));
        })
        .expect("a picture");
        // Far more than the four samples' five shades along its edges.
        let Handle::Rgba { pixels, .. } = &p.handle else {
            panic!("pixels")
        };
        let shades: std::collections::BTreeSet<u8> = pixels.chunks(4).map(|px| px[3]).collect();
        assert!(shades.len() > 8, "{} shades", shades.len());
    }
}
