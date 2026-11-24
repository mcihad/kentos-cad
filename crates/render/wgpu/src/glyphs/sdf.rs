//! A glyph's signed distance field (docs/adr/9212 §3): its outline at [`EM_PX`] an em, flattened into segments; for each
//! pixel's middle the distance to the nearest segment, signed by the nonzero rule (inside positive); [`SPREAD_PX`]
//! either side of the edge mapped onto a byte, 128 the edge. The shader turns it back into a distance in screen pixels
//! for its edge and its halo, so one field serves every size.

use ttf_parser::{Face, GlyphId, OutlineBuilder};

/// Pixels an em of the field.
pub const EM_PX: f32 = 64.0;
/// Pixels of distance either side of the edge the byte holds: a halo up to this wide at [`EM_PX`].
pub const SPREAD_PX: f32 = 8.0;
/// Chords a curve is flattened into: at [`EM_PX`] a glyph's curve strays under a tenth of a pixel from them.
const CURVE_STEPS: usize = 12;

/// A glyph's field: `width` × `height` bytes, rows from the top.
#[derive(Clone, Debug, PartialEq)]
pub struct GlyphField {
    pub width: u32,
    pub height: u32,
    /// The field's left and bottom edges from the glyph's origin, pixels at [`EM_PX`], y up.
    pub left: f32,
    pub bottom: f32,
    pub data: Vec<u8>,
}

/// The outline's segments, `[ax, ay, bx, by]` in pixels at [`EM_PX`], y up.
struct Segments {
    scale: f32,
    segments: Vec<[f32; 4]>,
    start: [f32; 2],
    last: [f32; 2],
}

impl Segments {
    fn to(&mut self, p: [f32; 2]) {
        if p != self.last {
            self.segments.push([self.last[0], self.last[1], p[0], p[1]]);
        }
        self.last = p;
    }
    fn at(&self, x: f32, y: f32) -> [f32; 2] {
        [x * self.scale, y * self.scale]
    }
}

impl OutlineBuilder for Segments {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close();
        self.start = self.at(x, y);
        self.last = self.start;
    }
    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.at(x, y);
        self.to(p);
    }
    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (a, c, b) = (self.last, self.at(x1, y1), self.at(x, y));
        for k in 1..=CURVE_STEPS {
            let t = k as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            self.to([u * u * a[0] + 2.0 * u * t * c[0] + t * t * b[0], u * u * a[1] + 2.0 * u * t * c[1] + t * t * b[1]]);
        }
    }
    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, c1, c2, b) = (self.last, self.at(x1, y1), self.at(x2, y2), self.at(x, y));
        for k in 1..=CURVE_STEPS {
            let t = k as f32 / CURVE_STEPS as f32;
            let u = 1.0 - t;
            let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            self.to([
                w0 * a[0] + w1 * c1[0] + w2 * c2[0] + w3 * b[0],
                w0 * a[1] + w1 * c1[1] + w2 * c2[1] + w3 * b[1],
            ]);
        }
    }
    fn close(&mut self) {
        let start = self.start;
        self.to(start);
    }
}

/// The field of `glyph` in `face`; none for a glyph without an outline (a space).
pub fn field(face: &Face<'_>, glyph: GlyphId) -> Option<GlyphField> {
    let mut s = Segments {
        scale: EM_PX / f32::from(face.units_per_em().max(1)),
        segments: Vec::new(),
        start: [0.0; 2],
        last: [0.0; 2],
    };
    face.outline_glyph(glyph, &mut s)?;
    s.close();
    if s.segments.is_empty() {
        return None;
    }
    let (mut x0, mut y0, mut x1, mut y1) = (f32::MAX, f32::MAX, f32::MIN, f32::MIN);
    for g in &s.segments {
        x0 = x0.min(g[0]).min(g[2]);
        y0 = y0.min(g[1]).min(g[3]);
        x1 = x1.max(g[0]).max(g[2]);
        y1 = y1.max(g[1]).max(g[3]);
    }
    let pad = SPREAD_PX + 1.0;
    let (left, bottom) = ((x0 - pad).floor(), (y0 - pad).floor());
    let (right, top) = ((x1 + pad).ceil(), (y1 + pad).ceil());
    let (width, height) = ((right - left) as u32, (top - bottom) as u32);
    let (w, h) = (width as usize, height as usize);
    // Each pixel's squared distance to the nearest segment, a segment tried only on the pixels within the spread of
    // its box: a pixel farther from every segment is a byte at the end of the range either way.
    let reach = SPREAD_PX + 1.0;
    let mut near = vec![reach * reach; w * h];
    for g in &s.segments {
        let c0 = (g[0].min(g[2]) - reach - left).floor().max(0.0) as usize;
        let c1 = ((g[0].max(g[2]) + reach - left).ceil() as usize).min(w);
        let r0 = (top - g[1].max(g[3]) - reach).floor().max(0.0) as usize;
        let r1 = ((top - g[1].min(g[3]) + reach).ceil() as usize).min(h);
        for row in r0..r1 {
            let y = top - row as f32 - 0.5;
            let line = &mut near[row * w..(row + 1) * w];
            for (col, n) in line.iter_mut().enumerate().take(c1).skip(c0) {
                let d = distance2(g, left + col as f32 + 0.5, y);
                if d < *n {
                    *n = d;
                }
            }
        }
    }
    // The sign row by row (nonzero rule): where the outline crosses the row's middle, and the winding of a ray to the
    // right of each pixel, the crossings it has passed taken off.
    let mut data = Vec::with_capacity(w * h);
    let mut crossings: Vec<(f32, i32)> = Vec::new();
    for row in 0..h {
        let y = top - row as f32 - 0.5;
        crossings.clear();
        crossings.extend(s.segments.iter().filter_map(|g| crossing(g, y)));
        crossings.sort_by(|a, b| a.0.total_cmp(&b.0));
        let mut winding: i32 = crossings.iter().map(|c| c.1).sum();
        let mut passed = 0;
        for col in 0..w {
            let x = left + col as f32 + 0.5;
            while passed < crossings.len() && crossings[passed].0 <= x {
                winding -= crossings[passed].1;
                passed += 1;
            }
            let d = near[row * w + col].sqrt() * if winding != 0 { 1.0 } else { -1.0 };
            data.push(((0.5 + d / (2.0 * SPREAD_PX)).clamp(0.0, 1.0) * 255.0).round() as u8);
        }
    }
    Some(GlyphField { width, height, left, bottom, data })
}

/// Squared distance from (x, y) to the segment.
fn distance2(g: &[f32; 4], x: f32, y: f32) -> f32 {
    let (dx, dy) = (g[2] - g[0], g[3] - g[1]);
    let (px, py) = (x - g[0], y - g[1]);
    let len2 = dx * dx + dy * dy;
    let t = if len2 > 0.0 { ((px * dx + py * dy) / len2).clamp(0.0, 1.0) } else { 0.0 };
    let (ex, ey) = (px - t * dx, py - t * dy);
    ex * ex + ey * ey
}

/// Where the segment crosses the row at height `y`, and how: upward +1, downward −1 (a ray to the right of a point
/// left of it counts it); none when it does not cross. A segment's lower end is on the row, its upper end above it.
fn crossing(g: &[f32; 4], y: f32) -> Option<(f32, i32)> {
    let (ax, ay, bx, by) = (g[0], g[1], g[2], g[3]);
    let dir = if ay <= y && by > y {
        1
    } else if by <= y && ay > y {
        -1
    } else {
        return None;
    };
    Some((ax + (y - ay) * (bx - ax) / (by - ay), dir))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn face() -> Face<'static> {
        // Arimo 400, the drawing's sans face.
        let (i, _) = kentos_geometry_core::text::font_data::DRAWING_KEYS
            .iter()
            .enumerate()
            .find(|(_, k)| k.0 == "arimo" && k.1 == 400 && !k.2)
            .expect("Arimo 400");
        Face::parse(kentos_geometry_core::text::font_data::DRAWING_FACES[i], 0).expect("a face")
    }

    fn value(f: &GlyphField, x: f32, y: f32) -> u8 {
        let col = (x - f.left).floor() as u32;
        let row = (f.bottom + f.height as f32 - y).floor() as u32;
        f.data[(row * f.width + col) as usize]
    }

    #[test]
    fn a_stem_is_inside_and_the_field_falls_by_the_spread_away_from_its_edge() {
        let face = face();
        let h = face.glyph_index('H').expect("H");
        let f = field(&face, h).expect("a field");
        // The left stem of an H in Arimo: its left edge where its outline's least x is, the stem a few px wide.
        let stem = f.left + SPREAD_PX + 1.0;
        let mid_y = f.bottom + f.height as f32 / 2.0;
        assert!(value(&f, stem + 2.0, mid_y) > 150, "inside the stem");
        assert!(value(&f, f.left + 0.5, mid_y) < 20, "far outside");
        // A pixel further out from the stem's edge: the byte falls by 255 / (2 × spread) ≈ 16.
        let (a, b) = (value(&f, stem - 1.5, mid_y), value(&f, stem - 2.5, mid_y));
        assert!((i32::from(a) - i32::from(b) - 16).abs() <= 2, "{a} {b}");
    }

    /// Every pixel against every segment, the winding counted per pixel: what the field's bounded search must equal.
    fn whole(face: &Face<'_>, glyph: GlyphId) -> Vec<u8> {
        let mut s = Segments {
            scale: EM_PX / f32::from(face.units_per_em().max(1)),
            segments: Vec::new(),
            start: [0.0; 2],
            last: [0.0; 2],
        };
        face.outline_glyph(glyph, &mut s).expect("an outline");
        s.close();
        let f = field(face, glyph).expect("a field");
        let top = f.bottom + f.height as f32;
        let mut out = Vec::new();
        for row in 0..f.height {
            let y = top - row as f32 - 0.5;
            for col in 0..f.width {
                let x = f.left + col as f32 + 0.5;
                let nearest = s.segments.iter().map(|g| distance2(g, x, y)).fold(f32::MAX, f32::min);
                let winding: i32 = s
                    .segments
                    .iter()
                    .filter_map(|g| crossing(g, y))
                    .filter(|c| c.0 > x)
                    .map(|c| c.1)
                    .sum();
                let d = nearest.sqrt() * if winding != 0 { 1.0 } else { -1.0 };
                out.push(((0.5 + d / (2.0 * SPREAD_PX)).clamp(0.0, 1.0) * 255.0).round() as u8);
            }
        }
        out
    }

    #[test]
    fn the_bounded_search_draws_the_same_field_as_every_pixel_against_every_segment() {
        let face = face();
        for c in ['H', 'o', 'g', 'ğ', 'Ş', '&', '@', '8', 'İ', 'ı'] {
            let g = face.glyph_index(c).expect("in the face");
            assert_eq!(field(&face, g).expect("a field").data, whole(&face, g), "{c}");
        }
    }

    #[test]
    fn a_counter_is_outside_and_a_space_has_no_field() {
        let face = face();
        let o = field(&face, face.glyph_index('o').expect("o")).expect("a field");
        let (cx, cy) = (o.left + o.width as f32 / 2.0, o.bottom + o.height as f32 / 2.0);
        assert!(value(&o, cx, cy) < 100, "the counter of an o is outside");
        assert!(field(&face, face.glyph_index(' ').expect("space")).is_none(), "nothing to draw");
    }
}
