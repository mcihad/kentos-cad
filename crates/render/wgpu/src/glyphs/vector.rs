//! A glyph drawn from its outline where its field would be magnified (docs/adr/9212 §3, docs/adr/9240; plan/render.md
//! RM-13 and RM-11).
//!
//! A glyph's field is [`EM_PX`] pixels an em in bytes (`sdf`). Close up its edge is the field's bilinear reading: a
//! sharp corner rounds over a field pixel, and past about sixteen times the byte's steps show as a staircase (a 2 m
//! text's E, 36 times its opening view, `.run/gui-evidence/render/rm13v`). ADR 9212 meant the glyph to be drawn from
//! its outline past [`VECTOR_EM_PX`] device pixels an em; this is that outline, as a program the glyph shader reads:
//!
//! - its contours, curves flattened within [`FLATTEN_EM`], filled by their winding (`svg_marks::fill_nonzero`) and
//!   triangulated; the triangles binned over the glyph's box (`vector_tiles::bin_triangles`);
//! - its contours' edges binned within a halo's reach ([`HALO_EM`], the field's spread), for the halo's distance.
//!
//! The glyph shader (`glyphFs`) reads it at four sample points a pixel, the drawing's 4× MSAA pattern: the glyph pass
//! draws single-sampled over the picture, so the edge is smoothed by the samples, as an area's by MSAA. [`read`] is
//! the reading on the CPU, the tests' twin.
//!
//! Numbers (f32 from the program's start; ems from the glyph's origin, y up):
//!
//! | At | What |
//! |---|---|
//! | 0–3 | its box (x0, y0, x1, y1): the outline's, grown by [`HALO_EM`] |
//! | 4, 5 | bins across and up |
//! | 6, 7, 8 | where the triangles' bins, list and triangles start |
//! | 9, 10, 11 | where the edges' bins, list and edges start |
//! | bins | nx·ny + 1 starts into their list, rows from the box's foot |
//! | triangles | six numbers each, every one turning with its inside on the left |
//! | edges | four numbers each: a contour's piece from one point to the next |

use kentos_geometry_core::vec2::Vec2;
use ttf_parser::{Face, GlyphId, OutlineBuilder};

#[cfg(doc)]
use super::sdf::{EM_PX, SPREAD_PX};

/// Device pixels an em past which a glyph is read from its outline, not its field (docs/adr/9212 §3). Up to here the
/// field is magnified at most 8 times: its byte's steps stay under a quarter pixel.
pub const VECTOR_EM_PX: f32 = 512.0;
/// How far a halo read from the outline reaches past it, ems: 16 px at [`VECTOR_EM_PX`], more as the glyph grows. A
/// text's halo is a few pixels; the field's own reach ([`SPREAD_PX`] at [`EM_PX`], an eighth of an em) would bin every
/// edge into four times as many bins.
pub const HALO_EM: f64 = 1.0 / 32.0;
/// Curves are flattened within this share of an em: a glyph 20 000 px an em strays under a pixel.
const FLATTEN_EM: f64 = 1.0 / 20_000.0;
/// Numbers a program's head takes.
const HEAD: usize = 12;
/// Bins across a glyph's box: about the square root of its triangles, within these.
const FEWEST_BINS: usize = 4;
const MOST_BINS: usize = 32;

/// A glyph's contours in ems, y up.
struct Contours {
    scale: f64,
    rings: Vec<Vec<Vec2>>,
    ring: Vec<Vec2>,
}

impl Contours {
    fn at(&self, x: f32, y: f32) -> Vec2 {
        Vec2::new(f64::from(x) * self.scale, f64::from(y) * self.scale)
    }

    fn last(&self) -> Vec2 {
        self.ring.last().copied().unwrap_or(Vec2::new(0.0, 0.0))
    }

    fn to(&mut self, p: Vec2) {
        if self.ring.last().is_none_or(|l| l.x != p.x || l.y != p.y) {
            self.ring.push(p);
        }
    }

    /// Steps a curve takes to stay within [`FLATTEN_EM`], by its control points' second difference `bend` (ems): a
    /// quadratic's chords stray at most a quarter of it over the steps squared.
    fn steps(bend: f64) -> usize {
        ((bend / (4.0 * FLATTEN_EM)).sqrt().ceil() as usize).clamp(1, 128)
    }
}

impl OutlineBuilder for Contours {
    fn move_to(&mut self, x: f32, y: f32) {
        self.close();
        let p = self.at(x, y);
        self.ring.push(p);
    }

    fn line_to(&mut self, x: f32, y: f32) {
        let p = self.at(x, y);
        self.to(p);
    }

    fn quad_to(&mut self, x1: f32, y1: f32, x: f32, y: f32) {
        let (a, c, b) = (self.last(), self.at(x1, y1), self.at(x, y));
        let n = Contours::steps((a.x - 2.0 * c.x + b.x).hypot(a.y - 2.0 * c.y + b.y));
        for k in 1..=n {
            let t = k as f64 / n as f64;
            let u = 1.0 - t;
            self.to(Vec2::new(
                u * u * a.x + 2.0 * u * t * c.x + t * t * b.x,
                u * u * a.y + 2.0 * u * t * c.y + t * t * b.y,
            ));
        }
    }

    fn curve_to(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, x: f32, y: f32) {
        let (a, c1, c2, b) = (self.last(), self.at(x1, y1), self.at(x2, y2), self.at(x, y));
        // A cubic strays at most three quarters of its larger second difference over the steps squared.
        let bend = (a.x - 2.0 * c1.x + c2.x)
            .hypot(a.y - 2.0 * c1.y + c2.y)
            .max((c1.x - 2.0 * c2.x + b.x).hypot(c1.y - 2.0 * c2.y + b.y));
        let n = Contours::steps(3.0 * bend);
        for k in 1..=n {
            let t = k as f64 / n as f64;
            let u = 1.0 - t;
            let (w0, w1, w2, w3) = (u * u * u, 3.0 * u * u * t, 3.0 * u * t * t, t * t * t);
            self.to(Vec2::new(
                w0 * a.x + w1 * c1.x + w2 * c2.x + w3 * b.x,
                w0 * a.y + w1 * c1.y + w2 * c2.y + w3 * b.y,
            ));
        }
    }

    fn close(&mut self) {
        let mut ring = std::mem::take(&mut self.ring);
        if ring.len() > 1
            && ring
                .first()
                .zip(ring.last())
                .is_some_and(|(f, l)| f.x == l.x && f.y == l.y)
        {
            ring.pop();
        }
        if ring.len() >= 3 {
            self.rings.push(ring);
        }
    }
}

/// The program of `glyph` in `face`; none for a glyph without an outline (a space).
pub fn program(face: &Face<'_>, glyph: GlyphId) -> Option<Vec<f32>> {
    let mut c = Contours {
        scale: 1.0 / f64::from(face.units_per_em().max(1)),
        rings: Vec::new(),
        ring: Vec::new(),
    };
    face.outline_glyph(glyph, &mut c)?;
    c.close();
    if c.rings.is_empty() {
        return None;
    }
    let mut tris: Vec<[f64; 6]> = Vec::new();
    for t in kentos_native_style::svg_marks::fill_nonzero(&c.rings).chunks(6) {
        let mut t = [t[0], t[1], t[2], t[3], t[4], t[5]];
        let cross = (t[2] - t[0]) * (t[5] - t[1]) - (t[4] - t[0]) * (t[3] - t[1]);
        if cross == 0.0 || cross.is_nan() {
            continue;
        }
        if cross < 0.0 {
            t.swap(2, 4);
            t.swap(3, 5);
        }
        tris.push(t);
    }
    if tris.is_empty() {
        return None;
    }
    let edges: Vec<[f64; 4]> = c
        .rings
        .iter()
        .flat_map(|r| {
            (0..r.len()).map(move |i| {
                [
                    r[i].x,
                    r[i].y,
                    r[(i + 1) % r.len()].x,
                    r[(i + 1) % r.len()].y,
                ]
            })
        })
        .collect();
    let mut b = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
    for p in c.rings.iter().flatten() {
        b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
    }
    let b = [
        b[0] - HALO_EM,
        b[1] - HALO_EM,
        b[2] + HALO_EM,
        b[3] + HALO_EM,
    ];
    let (bx, by) = (b[2] - b[0], b[3] - b[1]);
    let n = ((tris.len() as f64).sqrt().round() as usize).clamp(FEWEST_BINS, MOST_BINS) as f64;
    let nx = ((n * (bx / by).sqrt()).round() as usize).clamp(1, MOST_BINS);
    let ny = ((n * (by / bx).sqrt()).round() as usize).clamp(1, MOST_BINS);
    let tri_lists = kentos_native_style::vector_tiles::bin_triangles(&tris, b, nx, ny);
    // An edge in every bin it comes within a halo's reach of.
    let mut edge_lists: Vec<Vec<u32>> = vec![Vec::new(); nx * ny];
    for (k, e) in edges.iter().enumerate() {
        let (ex0, ex1) = (e[0].min(e[2]) - HALO_EM, e[0].max(e[2]) + HALO_EM);
        let (ey0, ey1) = (e[1].min(e[3]) - HALO_EM, e[1].max(e[3]) + HALO_EM);
        let col = |v: f64, lo: f64, span: f64, count: usize| {
            (((v - lo) / span * count as f64).floor().max(0.0) as usize).min(count - 1)
        };
        for y in col(ey0, b[1], by, ny)..=col(ey1, b[1], by, ny) {
            for x in col(ex0, b[0], bx, nx)..=col(ex1, b[0], bx, nx) {
                let cell = [
                    b[0] + bx * x as f64 / nx as f64 - HALO_EM,
                    b[1] + by * y as f64 / ny as f64 - HALO_EM,
                    b[0] + bx * (x + 1) as f64 / nx as f64 + HALO_EM,
                    b[1] + by * (y + 1) as f64 / ny as f64 + HALO_EM,
                ];
                if segment_meets(e, cell) {
                    edge_lists[y * nx + x].push(k as u32);
                }
            }
        }
    }
    let mut out: Vec<f64> = vec![b[0], b[1], b[2], b[3], nx as f64, ny as f64];
    out.resize(HEAD, 0.0);
    let lay = |out: &mut Vec<f64>,
               lists: &[Vec<u32>],
               items: &mut dyn Iterator<Item = f64>,
               at: usize| {
        out[at] = out.len() as f64;
        let mut start = 0usize;
        for l in lists {
            out.push(start as f64);
            start += l.len();
        }
        out.push(start as f64);
        out[at + 1] = out.len() as f64;
        out.extend(lists.iter().flatten().map(|&n| f64::from(n)));
        out[at + 2] = out.len() as f64;
        out.extend(items);
    };
    lay(&mut out, &tri_lists, &mut tris.iter().flatten().copied(), 6);
    lay(
        &mut out,
        &edge_lists,
        &mut edges.iter().flatten().copied(),
        9,
    );
    // Ems a glyph stays within: float32 keeps a millionth of one.
    Some(out.into_iter().map(|v| v as f32).collect())
}

/// Whether the segment `e` meets the box `[x0, y0, x1, y1]`, edges included.
fn segment_meets(e: &[f64; 4], b: [f64; 4]) -> bool {
    let (dx, dy) = (e[2] - e[0], e[3] - e[1]);
    // The segment's own normal, then the box's sides (its bounding box against the box's).
    let n = (-dy, dx);
    let span = [(b[0], b[1]), (b[2], b[1]), (b[2], b[3]), (b[0], b[3])]
        .iter()
        .map(|&(x, y)| (x - e[0]) * n.0 + (y - e[1]) * n.1)
        .fold((f64::MAX, f64::MIN), |(lo, hi), d| (lo.min(d), hi.max(d)));
    if span.0 > 0.0 || span.1 < 0.0 {
        return false;
    }
    e[0].min(e[2]) <= b[2]
        && e[0].max(e[2]) >= b[0]
        && e[1].min(e[3]) <= b[3]
        && e[1].max(e[3]) >= b[1]
}

/// The program's reading on the CPU (`glyphFs` at one sample point, the tests' twin): whether the point `q` (ems from
/// the glyph's origin, y up) is inside the glyph, and its distance to the outline when it is not (within the halo's
/// reach; `f64::MAX` beyond, or outside the box).
pub fn read(program: &[f32], q: [f64; 2]) -> (bool, f64) {
    let at = |i: usize| f64::from(program[i]);
    let [x0, y0, x1, y1] = [at(0), at(1), at(2), at(3)];
    if q[0] < x0 || q[0] > x1 || q[1] < y0 || q[1] > y1 {
        return (false, f64::MAX);
    }
    let (nx, ny) = (at(4) as usize, at(5) as usize);
    let bx = ((((q[0] - x0) / (x1 - x0)) * nx as f64).floor().max(0.0) as usize).min(nx - 1);
    let by = ((((q[1] - y0) / (y1 - y0)) * ny as f64).floor().max(0.0) as usize).min(ny - 1);
    let bin = by * nx + bx;
    let (bins, list, tris) = (at(6) as usize, at(7) as usize, at(8) as usize);
    for n in at(bins + bin) as usize..at(bins + bin + 1) as usize {
        let t = tris + at(list + n) as usize * 6;
        let side = |a: usize, b: usize| {
            (at(b) - at(a)) * (q[1] - at(a + 1)) - (at(b + 1) - at(a + 1)) * (q[0] - at(a))
        };
        if side(t, t + 2) >= 0.0 && side(t + 2, t + 4) >= 0.0 && side(t + 4, t) >= 0.0 {
            return (true, 0.0);
        }
    }
    let (bins, list, edges) = (at(9) as usize, at(10) as usize, at(11) as usize);
    let mut d = f64::MAX;
    for n in at(bins + bin) as usize..at(bins + bin + 1) as usize {
        let e = edges + at(list + n) as usize * 4;
        let (ax, ay, bx, by) = (at(e), at(e + 1), at(e + 2), at(e + 3));
        let (vx, vy) = (bx - ax, by - ay);
        let t = (((q[0] - ax) * vx + (q[1] - ay) * vy) / (vx * vx + vy * vy).max(1e-30))
            .clamp(0.0, 1.0);
        d = d.min((q[0] - ax - vx * t).hypot(q[1] - ay - vy * t));
    }
    (false, d)
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_geometry_core::text::shaping::font_bytes;

    fn face() -> Face<'static> {
        let font =
            kentos_geometry_core::text::shaping::select_font("barlow", 400, false).expect("Barlow");
        Face::parse(font_bytes(font).expect("bytes"), 0).expect("a face")
    }

    /// The glyph's contours and their winding at `q`, counted on the contours alone (no triangle, no bin).
    fn winding(rings: &[Vec<Vec2>], q: [f64; 2]) -> i32 {
        let mut w = 0;
        for r in rings {
            for i in 0..r.len() {
                let (a, b) = (r[i], r[(i + 1) % r.len()]);
                let side = (b.x - a.x) * (q[1] - a.y) - (q[0] - a.x) * (b.y - a.y);
                if a.y <= q[1] && b.y > q[1] && side > 0.0 {
                    w += 1;
                } else if a.y > q[1] && b.y <= q[1] && side < 0.0 {
                    w -= 1;
                }
            }
        }
        w
    }

    fn contours(face: &Face<'_>, c: char) -> Vec<Vec<Vec2>> {
        let mut out = Contours {
            scale: 1.0 / f64::from(face.units_per_em()),
            rings: Vec::new(),
            ring: Vec::new(),
        };
        face.outline_glyph(face.glyph_index(c).expect("in the face"), &mut out)
            .expect("an outline");
        out.close();
        out.rings
    }

    /// The program's reading against the contours' own winding over a fine grid of each glyph's box: holes (8, @),
    /// Turkish letters (ğ) and a descender (g) included. Points closer to the outline than a ten-thousandth of an em are
    /// not compared: there a float32 corner may fall either side.
    #[test]
    fn a_glyph_s_program_reads_as_its_contours_wind() {
        let face = face();
        for c in ['E', '8', 'g', 'ğ', '@', 'K'] {
            let program =
                program(&face, face.glyph_index(c).expect("in the face")).expect("a program");
            let rings = contours(&face, c);
            let (mut checked, mut wrong) = (0, 0);
            let [x0, y0, x1, y1] = [program[0], program[1], program[2], program[3]].map(f64::from);
            for j in 0..200 {
                for i in 0..200 {
                    let q = [
                        x0 + (x1 - x0) * (i as f64 + 0.37) / 200.0,
                        y0 + (y1 - y0) * (j as f64 + 0.61) / 200.0,
                    ];
                    let (inside, d) = read(&program, q);
                    let edge = rings
                        .iter()
                        .flat_map(|r| (0..r.len()).map(move |k| (r[k], r[(k + 1) % r.len()])))
                        .map(|(a, b)| {
                            let (vx, vy) = (b.x - a.x, b.y - a.y);
                            let t = (((q[0] - a.x) * vx + (q[1] - a.y) * vy)
                                / (vx * vx + vy * vy).max(1e-30))
                            .clamp(0.0, 1.0);
                            (q[0] - a.x - vx * t).hypot(q[1] - a.y - vy * t)
                        })
                        .fold(f64::MAX, f64::min);
                    if edge < 1e-4 {
                        continue;
                    }
                    checked += 1;
                    wrong += usize::from(inside != (winding(&rings, q) != 0));
                    // Outside within the halo's reach the distance is the outline's.
                    if !inside && edge <= HALO_EM {
                        wrong += usize::from((d - edge).abs() > 1e-5);
                    }
                }
            }
            assert!(checked > 30_000, "{c}: {checked} points");
            assert_eq!(wrong, 0, "{c}: {wrong} of {checked} points read otherwise");
        }
    }

    /// A glyph without an outline has no program.
    #[test]
    fn a_space_has_no_program() {
        let face = face();
        assert_eq!(
            program(&face, face.glyph_index(' ').expect("in the face")),
            None
        );
    }
}
