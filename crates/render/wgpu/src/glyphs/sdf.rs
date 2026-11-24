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
    let mut data = Vec::with_capacity((width * height) as usize);
    for row in 0..height {
        let y = top - row as f32 - 0.5;
        for col in 0..width {
            let x = left + col as f32 + 0.5;
            let mut nearest = f32::MAX;
            let mut winding = 0i32;
            for g in &s.segments {
                nearest = nearest.min(distance2(g, x, y));
                winding += crossing(g, x, y);
            }
            let d = nearest.sqrt() * if winding != 0 { 1.0 } else { -1.0 };
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

/// The segment's share of the winding number about (x, y): a ray to the right crossed upward +1, downward −1.
fn crossing(g: &[f32; 4], x: f32, y: f32) -> i32 {
    let (ax, ay, bx, by) = (g[0], g[1], g[2], g[3]);
    let side = (bx - ax) * (y - ay) - (x - ax) * (by - ay);
    if ay <= y && by > y && side > 0.0 {
        1
    } else if by <= y && ay > y && side < 0.0 {
        -1
    } else {
        0
    }
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

    #[test]
    fn a_counter_is_outside_and_a_space_has_no_field() {
        let face = face();
        let o = field(&face, face.glyph_index('o').expect("o")).expect("a field");
        let (cx, cy) = (o.left + o.width as f32 / 2.0, o.bottom + o.height as f32 / 2.0);
        assert!(value(&o, cx, cy) < 100, "the counter of an o is outside");
        assert!(field(&face, face.glyph_index(' ').expect("space")).is_none(), "nothing to draw");
    }
}
