//! A pattern tile's SVG marks as a program the tile shader draws them from (plan/render.md RM-11, docs/adr/9239).
//!
//! A pattern fill whose marks are SVG pictures (BÖHHBÜY's olive grove, marsh, rose garden and five more of its 30
//! patterns) is drawn from one atlas tile: its marks rasterized into a bitmap at most [`TILE_WIDE_PX`] wide
//! (`styled/atlas.rs`), repeated over the area by the tile shader (`tileFs`). Close up the tile is magnified and the
//! marks' edges turn soft and stepped, as a marker's SVG did before docs/adr/9208.
//!
//! The program holds the tile's marks as their triangles: each mark's SVG triangulated by colour (`svg_marks::mesh`,
//! fills ear-clipped, strokes outlined) and laid in a grid of bins over its box, every bin listing the triangles that
//! meet it. The renderer puts it in the layer's long-pattern page and draws the area with `tileVectorFs` where the
//! bitmap would be magnified: for each pixel, at the drawing's four sample points, the marks whose cells reach it, the
//! point turned into the mark's SVG and looked up in its bin. The area's own triangles bound what is drawn, as they
//! bound the bitmap, so a mark crossing the area's edge is cut exactly, with nothing kept per mark.
//!
//! Each mark sits where `raster::tile_image` draws it: its middle in the cell (a staggered tile's second row
//! half-shifted) moved by its offset, its picture fitted in its box (`raster::fit`), turned by its own turn
//! counter-clockwise on the ground. [`covers`] is the shader's reading on the CPU, the tests' twin.
//!
//! Numbers, all `f32` in the page (whole numbers exact):
//!
//! | At | What |
//! |---|---|
//! | 0 | the places' count P |
//! | 1 + 8 p | a place: its middle's x (tile widths), y (tile heights) and y (tile widths); ground tile widths an SVG pixel; the cosine and sine of the mark's turn; where its mark starts; how far it reaches (tile widths) |
//! | a mark | its SVG's width and height; its triangles' box (x0, y0, x1, y1, SVG pixels, y down); its bins across and up; its colours' count C; then each colour's r, g, b, a and where its bins, list and triangles start |
//! | bins | nx·ny + 1 starts into the list, row by row from the box's top left |
//! | list | the triangles' numbers |
//! | triangles | six numbers each, SVG pixels, every one turning so that its inside is on the left |
//!
//! Places come mark by mark, in the tile's order: a later mark draws over an earlier, a later colour over an earlier.

use crate::batches::{AtlasImage, MarkerLook, TileMark};
use crate::svg_marks::{VECTORS_FROM_PX, mesh};

/// The most numbers a program takes: one page of the layer's long patterns (`styled/dashes.rs`, 256 KiB). A tile
/// whose marks need more keeps its bitmap.
pub const MOST_NUMBERS: usize = 65_536;
/// The widest tile the atlas draws, device pixels (`styled/atlas.rs`, `MAX_STEP`): a tile shown wider is magnified.
pub const TILE_WIDE_PX: f64 = 512.0;
/// The highest (`raster::MAX_SIDE`): a staggered tile is two rows high.
pub const TILE_HIGH_PX: f64 = crate::raster::MAX_SIDE as f64;
/// Numbers a place takes.
pub const PLACE: usize = 8;
/// Numbers a mark's head takes before its colours, and a colour's.
const MARK_HEAD: usize = 9;
const COLOUR: usize = 7;
/// Bins across a mark's box: about the square root of its triangles, within these.
const FEWEST_BINS: usize = 4;
const MOST_BINS: usize = 48;

/// The marks of a tile drawn by its program: all of them SVG pictures with a size (a text or raster mark keeps the
/// tile's bitmap, which holds every mark in one picture).
fn svg_marks(draw: &[TileMark]) -> Option<Vec<(&TileMark, &str, f64, f64)>> {
    let marks: Vec<_> = draw
        .iter()
        .map(|m| match &m.look {
            MarkerLook::Image {
                image:
                    AtlasImage::Svg {
                        svg, width, height, ..
                    },
                fit_height: false,
            } => Some((m, svg.as_str(), *width, *height)),
            _ => None,
        })
        .collect::<Option<Vec<_>>>()?;
    let marks: Vec<_> = marks
        .into_iter()
        .filter(|(m, ..)| m.w > 0.0 && m.w.is_finite())
        .collect();
    (!marks.is_empty()).then_some(marks)
}

/// The pixels a metre at which a tile's marks draw from its program rather than its bitmap, as a scale `s`: the
/// program draws where `s × px_per_m > VECTORS_FROM_PX` (device pixels). That is where a mark grows as wide as a
/// marker's symbol does before its vectors take over (docs/adr/9208), or where the tile grows past what the atlas
/// draws (`size`: the tile's width and height on the ground), whichever comes first. None: the tile has no program.
pub fn switch(image: &AtlasImage, size: [f64; 2]) -> Option<f64> {
    let AtlasImage::Tile { draw, .. } = image else {
        return None;
    };
    let marks = svg_marks(draw)?;
    let widest = marks
        .iter()
        .map(|(m, ..)| m.w * size[0])
        .fold(0.0, f64::max);
    Some(
        widest
            .max(size[0] * VECTORS_FROM_PX / TILE_WIDE_PX)
            .max(size[1] * VECTORS_FROM_PX / TILE_HIGH_PX),
    )
}

/// The program of a pattern tile whose marks are all SVG pictures; none for another tile, a picture that does not
/// read, or one too large for a page.
pub fn program(image: &AtlasImage) -> Option<Vec<f64>> {
    let AtlasImage::Tile {
        draw,
        stagger,
        aspect,
        ..
    } = image
    else {
        return None;
    };
    let marks = svg_marks(draw)?;
    // The rows' places in a tile, as `raster::tile_image` draws them (rows down there, up on the ground).
    let centres: &[[f64; 2]] = if *stagger {
        &[[0.5, 0.25], [0.0, 0.75]]
    } else {
        &[[0.5, 0.5]]
    };
    let places = marks.len() * centres.len();
    let mut out = vec![places as f64];
    out.resize(1 + places * PLACE, 0.0);
    let mut p = 0;
    for (m, svg, width, height) in marks {
        let meshes = mesh(svg, width, height)?;
        let (sw, sh) = (meshes.width, meshes.height);
        // `raster::draw_mark`: the box `w` tile widths wide, its height the picture's own proportion; the picture
        // fitted in it, centred (`raster::fit`): tile widths an SVG pixel.
        let bw = m.w;
        let bh = bw * (height / width.max(1.0));
        let k = (bw / sw).min(bh / sh);
        // Its triangles, each turning with its inside on the left (SVG pixels, y down), and their box.
        let mut fills: Vec<([f64; 4], Vec<[f64; 6]>)> = Vec::new();
        let mut bbox = [f64::MAX, f64::MAX, f64::MIN, f64::MIN];
        for (rgba, flat) in &meshes.fills {
            let mut tris = Vec::with_capacity(flat.len() / 6);
            for t in flat.chunks(6) {
                let mut t: [f64; 6] = [t[0], t[1], t[2], t[3], t[4], t[5]];
                let cross = (t[2] - t[0]) * (t[5] - t[1]) - (t[4] - t[0]) * (t[3] - t[1]);
                // A triangle with no area (or not a number) holds nothing.
                if cross == 0.0 || cross.is_nan() {
                    continue;
                }
                if cross < 0.0 {
                    t.swap(2, 4);
                    t.swap(3, 5);
                }
                for q in t.chunks(2) {
                    bbox = [
                        bbox[0].min(q[0]),
                        bbox[1].min(q[1]),
                        bbox[2].max(q[0]),
                        bbox[3].max(q[1]),
                    ];
                }
                tris.push(t);
            }
            if !tris.is_empty() {
                fills.push((*rgba, tris));
            }
        }
        if fills.is_empty() {
            return None;
        }
        // How far the mark reaches from its middle, in tile widths, any way it turns.
        let reach = k * [
            (bbox[0], bbox[1]),
            (bbox[2], bbox[1]),
            (bbox[2], bbox[3]),
            (bbox[0], bbox[3]),
        ]
        .iter()
        .map(|&(x, y)| (x - sw / 2.0).hypot(sh / 2.0 - y))
        .fold(0.0, f64::max);
        let total: usize = fills.iter().map(|(_, t)| t.len()).sum();
        let (bx, by) = ((bbox[2] - bbox[0]).max(1e-9), (bbox[3] - bbox[1]).max(1e-9));
        let n = ((total as f64).sqrt().round() as usize).clamp(FEWEST_BINS, MOST_BINS) as f64;
        let nx = ((n * (bx / by).sqrt()).round() as usize).clamp(1, MOST_BINS);
        let ny = ((n * (by / bx).sqrt()).round() as usize).clamp(1, MOST_BINS);
        // The mark: its head, its colours' heads, then each colour's bins, list and triangles.
        let start = out.len();
        out.extend([
            sw,
            sh,
            bbox[0],
            bbox[1],
            bbox[2],
            bbox[3],
            nx as f64,
            ny as f64,
            fills.len() as f64,
        ]);
        let heads = out.len();
        out.resize(heads + fills.len() * COLOUR, 0.0);
        for (c, (rgba, tris)) in fills.iter().enumerate() {
            let mut lists: Vec<Vec<u32>> = vec![Vec::new(); nx * ny];
            let cell = |x: f64, y: f64| -> [f64; 4] {
                let (x0, y0) = (bbox[0] + bx * x / nx as f64, bbox[1] + by * y / ny as f64);
                [
                    x0,
                    y0,
                    bbox[0] + bx * (x + 1.0) / nx as f64,
                    bbox[1] + by * (y + 1.0) / ny as f64,
                ]
            };
            for (n, t) in tris.iter().enumerate() {
                let (tx0, tx1) = (t[0].min(t[2]).min(t[4]), t[0].max(t[2]).max(t[4]));
                let (ty0, ty1) = (t[1].min(t[3]).min(t[5]), t[1].max(t[3]).max(t[5]));
                let col = |v: f64, lo: f64, span: f64, count: usize| {
                    (((v - lo) / span * count as f64).floor().max(0.0) as usize).min(count - 1)
                };
                for y in col(ty0, bbox[1], by, ny)..=col(ty1, bbox[1], by, ny) {
                    for x in col(tx0, bbox[0], bx, nx)..=col(tx1, bbox[0], bx, nx) {
                        if meets(t, cell(x as f64, y as f64)) {
                            lists[y * nx + x].push(n as u32);
                        }
                    }
                }
            }
            let bins = out.len();
            let mut at = 0usize;
            for l in &lists {
                out.push(at as f64);
                at += l.len();
            }
            out.push(at as f64);
            let list = out.len();
            out.extend(lists.iter().flatten().map(|&n| f64::from(n)));
            let triangles = out.len();
            out.extend(tris.iter().flatten());
            out[heads + c * COLOUR..heads + (c + 1) * COLOUR].copy_from_slice(&[
                rgba[0],
                rgba[1],
                rgba[2],
                rgba[3],
                bins as f64,
                list as f64,
                triangles as f64,
            ]);
        }
        let (sin, cos) = m.rotation.sin_cos();
        for [cx, cy] in centres {
            let at = 1 + p * PLACE;
            out[at..at + PLACE].copy_from_slice(&[
                cx + m.offset[0],
                1.0 - cy,
                m.offset[1],
                k,
                cos,
                sin,
                start as f64,
                reach,
            ]);
            p += 1;
        }
        if out.len() > MOST_NUMBERS {
            return None;
        }
    }
    // A tile's height in its widths (`aspect`) is not needed: the shader takes the tile's ground size from the batch.
    let _ = aspect;
    Some(out)
}

/// Whether a triangle (its inside on the left, SVG pixels) meets a box `[x0, y0, x1, y1]`, edges included.
fn meets(t: &[f64; 6], b: [f64; 4]) -> bool {
    let tri = [(t[0], t[1]), (t[2], t[3]), (t[4], t[5])];
    let quad = [(b[0], b[1]), (b[2], b[1]), (b[2], b[3]), (b[0], b[3])];
    for poly in [&tri[..], &quad[..]] {
        for i in 0..poly.len() {
            let (p, q) = (poly[i], poly[(i + 1) % poly.len()]);
            let axis = (p.1 - q.1, q.0 - p.0);
            let span = |s: &[(f64, f64)]| {
                s.iter().fold((f64::MAX, f64::MIN), |(lo, hi), v| {
                    let d = v.0 * axis.0 + v.1 * axis.1;
                    (lo.min(d), hi.max(d))
                })
            };
            let ((a0, a1), (b0, b1)) = (span(&tri), span(&quad));
            if a1 < b0 || b1 < a0 {
                return false;
            }
        }
    }
    true
}

/// The shader's reading on the CPU (`tileVectorFs` at one sample point): the colour of the last mark colour covering
/// the point `t` of the tile's frame (ground metres, turned back by the tile's angle and shifted by its offset), for a
/// tile `size` metres wide and high. None where no mark is.
pub fn covers(program: &[f64], size: [f64; 2], t: [f64; 2]) -> Option<[f64; 4]> {
    let at = |i: usize| program[i];
    let places = at(0) as usize;
    let mut found = None;
    for p in 0..places {
        let base = 1 + p * PLACE;
        let (px, py) = (
            at(base) * size[0],
            at(base + 1) * size[1] + at(base + 2) * size[0],
        );
        let k = at(base + 3) * size[0];
        let (cos, sin) = (at(base + 4), at(base + 5));
        let mark = at(base + 6) as usize;
        let reach = at(base + 7) * size[0];
        let i0 = ((t[0] - px - reach) / size[0]).ceil() as i64;
        let i1 = ((t[0] - px + reach) / size[0]).floor() as i64;
        let j0 = ((t[1] - py - reach) / size[1]).ceil() as i64;
        let j1 = ((t[1] - py + reach) / size[1]).floor() as i64;
        for j in j0..=j1 {
            for i in i0..=i1 {
                let d = [
                    t[0] - (i as f64 * size[0] + px),
                    t[1] - (j as f64 * size[1] + py),
                ];
                // Into the mark: turned back, then in SVG pixels from its top left (y down).
                let local = [
                    (cos * d[0] + sin * d[1]) / k,
                    (-sin * d[0] + cos * d[1]) / k,
                ];
                let (sw, sh) = (at(mark), at(mark + 1));
                let q = [local[0] + sw / 2.0, sh / 2.0 - local[1]];
                if let Some(c) = inside(program, mark, q) {
                    found = Some(c);
                }
            }
        }
    }
    found
}

/// The last colour of the mark at `mark` whose triangles hold the SVG point `q`.
fn inside(program: &[f64], mark: usize, q: [f64; 2]) -> Option<[f64; 4]> {
    let at = |i: usize| program[i];
    let [x0, y0, x1, y1] = [at(mark + 2), at(mark + 3), at(mark + 4), at(mark + 5)];
    if q[0] < x0 || q[0] > x1 || q[1] < y0 || q[1] > y1 {
        return None;
    }
    let (nx, ny) = (at(mark + 6) as usize, at(mark + 7) as usize);
    let bx = ((((q[0] - x0) / (x1 - x0).max(1e-9)) * nx as f64)
        .floor()
        .max(0.0) as usize)
        .min(nx - 1);
    let by = ((((q[1] - y0) / (y1 - y0).max(1e-9)) * ny as f64)
        .floor()
        .max(0.0) as usize)
        .min(ny - 1);
    let bin = by * nx + bx;
    let mut found = None;
    for c in 0..at(mark + 8) as usize {
        let head = mark + MARK_HEAD + c * COLOUR;
        let (bins, list, tris) = (
            at(head + 4) as usize,
            at(head + 5) as usize,
            at(head + 6) as usize,
        );
        for n in at(bins + bin) as usize..at(bins + bin + 1) as usize {
            let t = tris + at(list + n) as usize * 6;
            let side = |a: usize, b: usize| {
                (at(b) - at(a)) * (q[1] - at(a + 1)) - (at(b + 1) - at(a + 1)) * (q[0] - at(a))
            };
            if side(t, t + 2) >= 0.0 && side(t + 2, t + 4) >= 0.0 && side(t + 4, t) >= 0.0 {
                found = Some([at(head), at(head + 1), at(head + 2), at(head + 3)]);
                break;
            }
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use super::*;
    use crate::picture::{ImageSource, Picture, TextOutline};
    use crate::raster::{PAD, tile_image};

    /// The SVGs as the desktop's image source reads them (`svg::picture`).
    struct Svgs;

    impl ImageSource for Svgs {
        fn picture(&self, image: &AtlasImage) -> Option<Arc<Picture>> {
            match image {
                AtlasImage::Svg { svg, .. } => {
                    crate::svg::picture(svg, &|_, _, _, _| None).map(Arc::new)
                }
                _ => None,
            }
        }

        fn text(&self, _: &str, _: Option<&str>, _: f64, _: bool) -> Option<Arc<TextOutline>> {
            None
        }
    }

    /// An L of two bars in a 20 × 10 picture's top left: no turn or mirror of it looks the same.
    const ELL: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="10"><path d="M1 1 H9 V3 H3 V8 H1 Z" fill="#336699"/></svg>"##;
    /// A round-capped stroke, as BÖHHBÜY's pictograms are drawn.
    const SPRIG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 100 100" width="100" height="100"><path fill="none" stroke="#000000" stroke-width="5.56" stroke-linecap="round" stroke-linejoin="round" d="M50 91.67L50 36.11M50 36.11Q22.22 34.72 25 8.33Q47.22 9.72 50 36.11Z"/></svg>"##;

    fn mark(svg: &str, size: [f64; 2], w: f64, offset: [f64; 2], rotation: f64) -> TileMark {
        TileMark {
            look: MarkerLook::Image {
                image: AtlasImage::Svg {
                    key: "s".into(),
                    svg: svg.into(),
                    width: size[0],
                    height: size[1],
                },
                fit_height: false,
            },
            // As decoding writes an SVG mark: its height its size (`batches::tile_image`).
            w,
            h: w,
            offset,
            rotation,
        }
    }

    fn tile(draw: Vec<TileMark>, aspect: f64, stagger: bool) -> AtlasImage {
        AtlasImage::Tile {
            key: "t".into(),
            aspect,
            stagger,
            draw,
        }
    }

    /// The bitmap `raster::tile_image` draws for the tile, against the program read at each pixel's middle: pixels
    /// clearly inked (more than 90 %) and clearly clear (under 10 %) that the program reads the other way.
    fn disagreements(image: &AtlasImage, size: [f64; 2], step: u32) -> (usize, usize) {
        let AtlasImage::Tile {
            aspect,
            stagger,
            draw,
            ..
        } = image
        else {
            panic!("a tile");
        };
        let program = program(image).expect("a program");
        let painted = tile_image(*aspect, *stagger, draw, step, &Svgs).expect("the bitmap draws");
        let (w, h) = (painted.width as usize, painted.height as usize);
        let row = w + 2 * PAD as usize;
        let (mut wrong, mut inked) = (0, 0);
        for y in 0..h {
            for x in 0..w {
                let a = painted.data[((y + PAD as usize) * row + x + PAD as usize) * 4 + 3];
                // The pixel's middle in the tile's frame: across from its left, up from its foot (the image's rows
                // run down, `tileFs` reads them up).
                let t = [
                    (x as f64 + 0.5) / w as f64 * size[0],
                    (1.0 - (y as f64 + 0.5) / h as f64) * size[1],
                ];
                let read = covers(&program, size, t).is_some();
                if a > 230 {
                    inked += 1;
                    wrong += usize::from(!read);
                } else if a < 25 {
                    wrong += usize::from(read);
                }
            }
        }
        (wrong, inked)
    }

    #[test]
    fn the_program_draws_the_marks_where_the_tile_bitmap_has_them() {
        // A staggered tile 10 × 7 (two rows: 14 high), its L shifted and turned, wider than its cell's half so that
        // its copies cross the tile's edges.
        let size = [10.0, 14.0];
        let image = tile(
            vec![mark(ELL, [20.0, 10.0], 0.55, [0.08, -0.05], 0.4)],
            1.4,
            true,
        );
        let (wrong, inked) = disagreements(&image, size, 256);
        assert!(inked > 2_000, "the L is inked: {inked} px");
        assert!(
            wrong <= inked / 500,
            "{wrong} of {inked} px read the other way"
        );
        // Unstaggered, unturned, a round-capped sprig.
        let image = tile(
            vec![mark(SPRIG, [100.0, 100.0], 0.36, [0.0, 0.0], 0.0)],
            1.0,
            false,
        );
        let (wrong, inked) = disagreements(&image, [10.0, 10.0], 512);
        assert!(inked > 1_000, "the sprig is inked: {inked} px");
        assert!(
            wrong <= inked / 500,
            "{wrong} of {inked} px read the other way"
        );
    }

    #[test]
    fn a_later_mark_and_colour_draw_over_an_earlier() {
        let two = r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="10"><rect width="10" height="10" fill="#ff0000"/><rect x="2" y="2" width="6" height="6" fill="#0000ff"/></svg>"##;
        let image = tile(
            vec![mark(two, [10.0, 10.0], 0.5, [0.0, 0.0], 0.0)],
            1.0,
            false,
        );
        let program = program(&image).expect("a program");
        // The cell's middle is the mark's: blue over red; near its edge red; outside nothing.
        assert_eq!(
            covers(&program, [10.0, 10.0], [5.0, 5.0]),
            Some([0.0, 0.0, 1.0, 1.0])
        );
        assert_eq!(
            covers(&program, [10.0, 10.0], [2.8, 5.0]),
            Some([1.0, 0.0, 0.0, 1.0])
        );
        assert_eq!(covers(&program, [10.0, 10.0], [1.0, 1.0]), None);
        // A whole number of tiles away, the same.
        assert_eq!(
            covers(&program, [10.0, 10.0], [-35.0, 125.0]),
            Some([0.0, 0.0, 1.0, 1.0])
        );
    }

    #[test]
    fn the_program_takes_over_where_a_mark_or_the_tile_outgrows_its_bitmap() {
        // BÖHHBÜY's olive grove: a 3.6 mm sprig in a staggered 10 mm tile, 1:1000. The sprig reaches 128 px first.
        let olive = tile(
            vec![mark(SPRIG, [100.0, 100.0], 0.36, [0.0, 0.0], 0.0)],
            2.0,
            true,
        );
        let near = |a: Option<f64>, b: f64| a.is_some_and(|a| (a - b).abs() < 1e-12);
        assert!(
            near(switch(&olive, [10.0, 20.0]), 3.6),
            "{:?}",
            switch(&olive, [10.0, 20.0])
        );
        // A small mark in a wide tile: the tile reaches the atlas's 512 px first.
        let sparse = tile(
            vec![mark(SPRIG, [100.0, 100.0], 0.05, [0.0, 0.0], 0.0)],
            1.0,
            false,
        );
        assert!(
            near(switch(&sparse, [40.0, 40.0]), 10.0),
            "{:?}",
            switch(&sparse, [40.0, 40.0])
        );
        // A text mark keeps the tile's bitmap: one picture holds all its marks.
        let mut text = mark(SPRIG, [100.0, 100.0], 0.36, [0.0, 0.0], 0.0);
        text.look = MarkerLook::Image {
            image: AtlasImage::Text {
                key: "x".into(),
                text: "x".into(),
                font: None,
                weight: 400.0,
                italic: false,
                color: "#000000".into(),
                halo: None,
            },
            fit_height: true,
        };
        let mixed = tile(
            vec![mark(SPRIG, [100.0, 100.0], 0.36, [0.0, 0.0], 0.0), text],
            1.0,
            false,
        );
        assert_eq!(
            (switch(&mixed, [10.0, 10.0]), program(&mixed)),
            (None, None)
        );
        // A picture that does not read keeps it too.
        let broken = tile(
            vec![mark("<svg", [10.0, 10.0], 0.5, [0.0, 0.0], 0.0)],
            1.0,
            false,
        );
        assert_eq!(program(&broken), None);
    }
}
