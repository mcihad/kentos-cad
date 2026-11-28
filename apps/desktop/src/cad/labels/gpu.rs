//! The drawing's text on the GPU (docs/adr/9212 §4, stages 2 to 4): every text the labels paint goes to the drawing
//! area's renderer as glyphs, flat quads, rings and lines (`kentos_render_wgpu::glyphs`) instead of onto the canvas:
//! text objects, a block's texts, multi-line texts with their masks, dimension values, a table's cells with their fills,
//! the label engine's labels with their frames and callouts, SHX texts' strokes, Unicode-profile texts, the bands of the
//! texts too small to read and a text being written. Placing and layout stay the canvas's own, to the glyph: a piece is
//! shaped as the canvas shapes it (`shaped`: Iced's text at [`REFERENCE`]) and each glyph's face is found among the
//! drawing's faces by its bytes. A piece with a glyph from a face that is not the drawing's (the text system fell back to
//! another for a letter they lack) stays on the canvas, as do the screen's own marks (grid north, the scale bar, the
//! axes: RM-14).
//!
//! The canvas draws its picture in the frame's draw, before the scene is prepared (Iced draws the widgets, then
//! prepares and renders their primitives): a painting leaves its texts in the [`GlyphSlot`] and the scene draws them
//! in the same frame. A text is anchored on the map where the canvas puts it and placed from there in pixels, turning
//! with the view as the canvas's twisted picture turns, and growing with a zoom as the canvas's moved picture grows
//! until the labels are placed again.

use super::*;
use std::sync::Arc;

use kentos_render_wgpu::glyphs::{GlyphPlace, GlyphQuad, GlyphText, GlyphUnit};

use crate::glyph_layer::Run;

/// A line's width on the canvas (`Ink::stroke_line`'s), logical px.
const LINE: f64 = 1.2;

/// A painting's texts for the GPU, and whether the spot being painted may send its pieces there.
pub(super) struct Sink {
    slot: GlyphSlot,
    texts: RefCell<Vec<GlyphText>>,
    open: Cell<bool>,
}

impl Sink {
    pub(super) fn new(slot: GlyphSlot) -> Sink {
        Sink {
            slot,
            texts: RefCell::new(Vec::new()),
            open: Cell::new(false),
        }
    }

    /// A painting starts: everything it draws but the screen's marks may go to the GPU (since stage 4).
    pub(super) fn begin(&self) {
        self.texts.borrow_mut().clear();
        self.open.set(true);
    }

    /// The painting is done: its texts to the renderer, placed at `placed_px` (the view's map pixels a metre).
    pub(super) fn publish(&self, placed_px: f64) {
        self.slot
            .put(std::mem::take(&mut *self.texts.borrow_mut()), placed_px);
    }

    /// The piece as glyphs with its mask and underline, when it may go: true when it went (or has nothing to draw).
    fn take(&self, piece: &Piece<'_>, halo: Color, view: &Camera) -> bool {
        if !self.open.get() || piece.size < SMALLEST {
            return false;
        }
        let blank = piece.text.trim().is_empty();
        if blank && piece.underline <= 0.0 {
            return true;
        }
        let run = if blank {
            None
        } else {
            run(piece.text, piece.font)
        };
        if !blank && run.is_none() {
            return false;
        }
        let at = view.screen_to_map(f64::from(piece.at.x), f64::from(piece.at.y));
        let pen = Pen::of(piece, run.as_deref());
        let mut under = Vec::new();
        if piece.mask > 0.0 {
            under.push(GlyphQuad {
                transform: pen.mask(piece),
                color: rgba(Color { a: 1.0, ..halo }),
                triangle: false,
            });
        }
        let mut over = Vec::new();
        if piece.underline > 0.0 {
            let [halo_bar, bar] = pen.underline(piece);
            if halo.a > 0.0 {
                over.push(GlyphQuad {
                    transform: halo_bar,
                    color: rgba(halo),
                    triangle: false,
                });
            }
            over.push(GlyphQuad {
                transform: bar,
                color: rgba(piece.color),
                triangle: false,
            });
        }
        self.texts.borrow_mut().push(GlyphText {
            at: [at.x, at.y],
            unit: GlyphUnit::Px { turns: true },
            color: rgba(piece.color),
            halo: rgba(halo),
            halo_px: if halo.a > 0.0 { HALO } else { 0.0 },
            glyphs: run.map(|r| pen.glyphs(&r)).unwrap_or_default(),
            under,
            over,
        });
        true
    }

    /// A quadrilateral of the screen filled (a multi-line text's mask, a cell's fill), when its spot may send it and
    /// it is a parallelogram (as those are): true when it went.
    fn quad(&self, quad: [Point; 4], color: Color, view: &Camera) -> bool {
        if !self.open.get() {
            return false;
        }
        let [p0, p1, p2, p3] = quad.map(|p| [f64::from(p.x), f64::from(p.y)]);
        let (e1, e2) = (
            [p1[0] - p0[0], p1[1] - p0[1]],
            [p3[0] - p0[0], p3[1] - p0[1]],
        );
        if (p0[0] + e1[0] + e2[0] - p2[0]).abs() > 0.5
            || (p0[1] + e1[1] + e2[1] - p2[1]).abs() > 0.5
        {
            return false;
        }
        let at = view.screen_to_map(p0[0], p0[1]);
        self.texts.borrow_mut().push(GlyphText {
            at: [at.x, at.y],
            unit: GlyphUnit::Px { turns: true },
            color: rgba(color),
            halo: Rgba8([0; 4]),
            halo_px: 0.0,
            glyphs: Vec::new(),
            under: vec![GlyphQuad {
                transform: [e1[0], -e1[1], e2[0], -e2[1], 0.0, 0.0],
                color: rgba(color),
                triangle: false,
            }],
            over: Vec::new(),
        });
        true
    }
}

impl Sink {
    /// A text of no glyphs anchored at the screen's `p` (the view's), its shapes in pixels from it, y up.
    fn shapes(&self, p: Point, view: &Camera, under: Vec<GlyphQuad>) {
        if under.is_empty() {
            return;
        }
        let at = view.screen_to_map(f64::from(p.x), f64::from(p.y));
        self.texts.borrow_mut().push(GlyphText {
            at: [at.x, at.y],
            unit: GlyphUnit::Px { turns: true },
            color: Rgba8([0; 4]),
            halo: Rgba8([0; 4]),
            halo_px: 0.0,
            glyphs: Vec::new(),
            under,
            over: Vec::new(),
        });
    }

    /// A line of the screen `width` px wide (a frame's outline, a callout, an SHX letter's stroke): true when it went.
    fn stroke(&self, pts: &[Point], closed: bool, color: Color, width: f64, view: &Camera) -> bool {
        if !self.open.get() {
            return false;
        }
        let Some(&first) = pts.first() else {
            return true;
        };
        let at = |p: &Point| [f64::from(p.x - first.x), -f64::from(p.y - first.y)];
        let color = rgba(color);
        let ends = pts.len() + usize::from(closed && pts.len() > 2);
        let quads = (1..ends)
            .filter_map(|i| {
                GlyphQuad::segment(at(&pts[i - 1]), at(&pts[i % pts.len()]), width, color)
            })
            .collect();
        self.shapes(first, view, quads);
        true
    }

    /// A ring of the screen filled (a round frame, an SHX text's mask, an arrowhead): true when it went.
    fn ring(&self, ring: &[Point], color: Color, view: &Camera) -> bool {
        if !self.open.get() {
            return false;
        }
        let Some(&first) = ring.first() else {
            return true;
        };
        let pts: Vec<[f64; 2]> = ring
            .iter()
            .map(|p| [f64::from(p.x - first.x), -f64::from(p.y - first.y)])
            .collect();
        self.shapes(first, view, GlyphQuad::ring(&pts, 1.0, rgba(color)));
        true
    }

    /// A text of the Unicode profile (docs/adr/0914) from its own glyph plan: its mask, its glyphs run by run in their
    /// colours, its rules. A plan that failed keeps its mark on the canvas.
    fn unicode(&self, piece: &unicode::Piece<'_>, view: &Camera) -> bool {
        if !self.open.get() {
            return false;
        }
        let Ok(plan) = &piece.text.plan else {
            return false;
        };
        let [x, y] = piece.camera.world_to_screen(piece.at);
        let at = view.screen_to_map(x, y);
        let k = piece.camera.scale;
        // The plan's model offsets from the text's point, as the canvas puts them on the screen (y up).
        let scaled = |m: [f64; 6]| m.map(|v| v * k);
        let mut texts: Vec<GlyphText> = Vec::new();
        for g in &plan.glyphs {
            let color = rgba(unicode::color(piece, g.glyph.scalars.start));
            let place = GlyphPlace {
                transform: scaled(GlyphPlace::of(g).transform),
                ..GlyphPlace::of(g)
            };
            match texts.last_mut() {
                Some(t) if t.color == color => t.glyphs.push(place),
                _ => texts.push(GlyphText {
                    at: [at.x, at.y],
                    unit: GlyphUnit::Px { turns: true },
                    color,
                    halo: Rgba8([0; 4]),
                    halo_px: 0.0,
                    glyphs: vec![place],
                    under: Vec::new(),
                    over: Vec::new(),
                }),
            }
        }
        if texts.is_empty() {
            texts.push(GlyphText {
                at: [at.x, at.y],
                unit: GlyphUnit::Px { turns: true },
                color: rgba(piece.color),
                halo: Rgba8([0; 4]),
                halo_px: 0.0,
                glyphs: Vec::new(),
                under: Vec::new(),
                over: Vec::new(),
            });
        }
        let corner = |p: [f64; 2]| [p[0] * k, p[1] * k];
        let quad = |c: [[f64; 2]; 4], color: Rgba8| {
            let [a, b, _, d] = c.map(corner);
            GlyphQuad::spanned(
                a,
                [b[0] - a[0], b[1] - a[1]],
                [d[0] - a[0], d[1] - a[1]],
                color,
            )
        };
        if let Some(mask) = piece.text.mask_outline() {
            texts[0].under.push(quad(mask, rgba(piece.halo)));
        }
        let last = texts.len() - 1;
        for rule in &plan.rules {
            let color = rgba(piece.runs.get(rule.run).copied().unwrap_or(piece.color));
            texts[last].over.push(quad(rule.corners, color));
        }
        self.texts.borrow_mut().extend(texts);
        true
    }

    /// The bands of the texts too small to read (docs/adr/0626 §6, 0786 §4), as the canvas strokes them: true when
    /// they went.
    pub(super) fn bands(&self, looks: &[(Color, f32, Vec<[Point; 2]>)], view: &Camera) -> bool {
        if !self.open.get() {
            return false;
        }
        for (color, thick, segments) in looks {
            for [a, b] in segments {
                self.stroke(&[*a, *b], false, *color, f64::from(*thick), view);
            }
        }
        true
    }
}

fn rgba(c: Color) -> Rgba8 {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgba8([byte(c.r), byte(c.g), byte(c.b), byte(c.a)])
}

/// The canvas with the GPU beside it: a piece its spot may send to the GPU goes there, everything else onto the
/// canvas. Counted as the canvas is (`Counted`): a part whose words went to the GPU did not draw bands alone.
pub(super) struct Split<'f, 's> {
    pub(super) frame: &'f mut Frame,
    pub(super) sink: Option<&'s Sink>,
    /// The view the pieces are placed on (a twisted view's upright one).
    pub(super) view: &'s Camera,
}

impl Ink for Split<'_, '_> {
    fn size(&self) -> Size {
        Frame::size(self.frame)
    }

    fn piece(&mut self, piece: &Piece<'_>, halo: Color, of: Option<Slot>) {
        if self
            .sink
            .is_some_and(|sink| sink.take(piece, halo, self.view))
        {
            return;
        }
        <Frame as Ink>::piece(self.frame, piece, halo, of);
    }

    fn shaped(&mut self, piece: &unicode::Piece<'_>) {
        if self.sink.is_some_and(|sink| sink.unicode(piece, self.view)) {
            return;
        }
        <Frame as Ink>::shaped(self.frame, piece);
    }

    fn fill_quad(&mut self, quad: [Point; 4], color: Color, of: Option<Slot>) {
        if self.sink.is_some_and(|sink| {
            sink.quad(quad, color, self.view) || sink.ring(&quad, color, self.view)
        }) {
            return;
        }
        <Frame as Ink>::fill_quad(self.frame, quad, color, of);
    }

    fn stroke_segments(&mut self, segments: &[[Point; 2]], color: Color, width: f32) {
        if let Some(sink) = self.sink
            && sink.bands(&[(color, width, segments.to_vec())], self.view)
        {
            return;
        }
        <Frame as Ink>::stroke_segments(self.frame, segments, color, width);
    }

    fn marks(
        &mut self,
        camera: &Camera,
        colors: &Colors,
        axes: kentos_interaction::Axes,
        ucs: Option<f64>,
        lift: f32,
        per_unit: f64,
    ) {
        <Frame as Ink>::marks(self.frame, camera, colors, axes, ucs, lift, per_unit);
    }

    fn fill_ring(&mut self, ring: &[Point], color: Color, of: Option<Slot>) {
        // A placed label's rectangle (its background, its shadow) as a quad, any other ring as its triangles.
        if let (Some(sink), &[a, b, c, d]) = (self.sink, ring)
            && sink.quad([a, b, c, d], color, self.view)
        {
            return;
        }
        if self
            .sink
            .is_some_and(|sink| sink.ring(ring, color, self.view))
        {
            return;
        }
        <Frame as Ink>::fill_ring(self.frame, ring, color, of);
    }

    fn stroke_line(&mut self, pts: &[Point], closed: bool, color: Color, of: Option<Slot>) {
        if self
            .sink
            .is_some_and(|sink| sink.stroke(pts, closed, color, LINE, self.view))
        {
            return;
        }
        <Frame as Ink>::stroke_line(self.frame, pts, closed, color, of);
    }
}

/// `text` in `font` as glyphs of the drawing's faces; none when a glyph is from another face, or it is more than a line.
pub(super) fn run(text: &str, font: Font) -> Option<Arc<Run>> {
    crate::glyph_layer::text_run(text, font)
}

/// A piece's own frame as the canvas draws it (labels/turned.rs): y down from its anchor, its letters from where the
/// anchor puts the text; onto the screen's pixels from the anchor, y up. Its letters lean (by the lean over the width
/// factor: they lean before they are narrowed) and are narrowed by the width factor; its mask leans with them and is
/// not narrowed; its underline is narrowed and does not lean; all turn by its angle.
pub(super) struct Pen {
    size: f64,
    narrow: f64,
    lean: f64,
    along: f64,
    down: f64,
    sin: f64,
    cos: f64,
}

impl Pen {
    pub(super) fn of(piece: &Piece<'_>, run: Option<&Run>) -> Pen {
        let s = f64::from(piece.size);
        let width = run.map_or(0.0, |r| f64::from(r.width));
        let (sin, cos) = kentos_num::sin_cos(f64::from(piece.angle));
        Pen {
            size: s,
            narrow: f64::from(piece.width_factor),
            lean: f64::from(piece.lean),
            along: match piece.anchor {
                Anchor::LeftBaseline => 0.0,
                Anchor::CenterBaseline | Anchor::CenterMiddle => -width * s / 2.0,
            },
            // The baseline from the anchor, down: a text centred halfway up has its line's top half its size over it.
            down: match piece.anchor {
                Anchor::LeftBaseline | Anchor::CenterBaseline => 0.0,
                Anchor::CenterMiddle => -s / 2.0 + run.map_or(0.0, |r| f64::from(r.baseline)) * s,
            },
            sin,
            cos,
        }
    }

    /// A vector of the frame (y down), leant by `lean` and narrowed by `narrow`, on the screen (y up).
    fn out(&self, x: f64, y: f64, narrow: f64, lean: f64) -> [f64; 2] {
        let x = (x - lean * y) * narrow;
        [self.cos * x - self.sin * y, -(self.sin * x + self.cos * y)]
    }

    /// The rectangle `x`, `y` (its top left in the frame), `w` × `h` as the unit square's place.
    fn rect(&self, [x, y, w, h]: [f64; 4], narrow: f64, lean: f64) -> [f64; 6] {
        let (o, ex, ey) = (
            self.out(x, y, narrow, lean),
            self.out(w, 0.0, narrow, lean),
            self.out(0.0, h, narrow, lean),
        );
        [ex[0], ex[1], ey[0], ey[1], o[0], o[1]]
    }

    /// The glyphs' places: each glyph's em square from its pen.
    pub(super) fn glyphs(&self, run: &Run) -> Vec<GlyphPlace> {
        let (s, lean) = (self.size, self.lean / self.narrow);
        let (ex, ey) = (
            self.out(s, 0.0, self.narrow, lean),
            self.out(0.0, -s, self.narrow, lean),
        );
        run.glyphs
            .iter()
            .map(|(key, [x, y])| {
                let p = self.out(
                    self.along + s * f64::from(*x),
                    self.down - s * f64::from(*y),
                    self.narrow,
                    lean,
                );
                GlyphPlace {
                    key: *key,
                    transform: [ex[0], ex[1], ey[0], ey[1], p[0], p[1]],
                }
            })
            .collect()
    }

    /// The mask (docs/adr/0145): the text's box a tenth of its height wider all round, 1.15 of it over the anchor and
    /// 0.23 under; a centred value's (docs/adr/0147) wider on its sides only.
    pub(super) fn mask(&self, piece: &Piece<'_>) -> [f64; 6] {
        let m = self.size * 0.1;
        let v = if piece.anchor == Anchor::CenterBaseline {
            0.0
        } else {
            m
        };
        let rect = [
            self.along - m,
            -self.size * 1.15 - v,
            f64::from(piece.mask) + 2.0 * m,
            self.size * 1.38 + 2.0 * v,
        ];
        self.rect(rect, 1.0, self.lean)
    }

    /// The underline (docs/adr/0182 §3): 0.12 of the size under the baseline, 0.06 thick (a pixel at least), and its
    /// halo HALO round it.
    pub(super) fn underline(&self, piece: &Piece<'_>) -> [[f64; 6]; 2] {
        let (top, thick) = (self.size * 0.12, (self.size * 0.06).max(1.0));
        let (long, h) = (f64::from(piece.underline), f64::from(HALO));
        [
            self.rect(
                [
                    self.along - h,
                    self.down + top - h,
                    long + 2.0 * h,
                    thick + 2.0 * h,
                ],
                self.narrow,
                0.0,
            ),
            self.rect([self.along, self.down + top, long, thick], self.narrow, 0.0),
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn piece<'t>(text: &'t str, anchor: Anchor, angle: f32) -> Piece<'t> {
        Piece {
            text,
            at: Point::new(100.0, 50.0),
            angle,
            size: 20.0,
            font: drawing_fonts::font(DrawingFont::Barlow, 400, false),
            anchor,
            color: Color::BLACK,
            width_factor: 1.0,
            mask: 0.0,
            underline: 0.0,
            lean: 0.0,
        }
    }

    /// A text in a drawing's face is shaped into that face's glyphs, as wide as the canvas measures it.
    #[test]
    fn a_text_is_the_drawing_faces_glyphs_as_wide_as_measured() {
        drawing_fonts::load();
        let font = drawing_fonts::font(DrawingFont::Barlow, 400, false);
        let run = run("Kentos Ağ", font).expect("drawn on the GPU");
        let barlow =
            kentos_geometry_core::text::shaping::select_font("barlow", 400, false).expect("Barlow");
        assert_eq!(run.glyphs.len(), 9);
        assert!(
            run.glyphs.iter().all(|(k, _)| k.font == barlow),
            "{:?}",
            run.glyphs
        );
        assert!((run.width * REFERENCE - measure("Kentos Ağ", font).width).abs() < 1e-3);
        assert!(
            run.glyphs.windows(2).all(|w| w[1].1[0] > w[0].1[0]),
            "left to right"
        );
        assert!(
            run.glyphs.iter().all(|(_, p)| p[1].abs() < 1e-6),
            "on the baseline"
        );
        // A second time from the kept runs.
        assert!(Arc::ptr_eq(
            &run,
            &super::run("Kentos Ağ", font).expect("kept")
        ));
    }

    /// A piece's glyphs where its anchor puts them: from its left end on the baseline, round its middle, and turned.
    #[test]
    fn a_piece_places_its_glyphs_from_its_anchor_as_the_canvas_does() {
        drawing_fonts::load();
        let left = piece("Ada 12", Anchor::LeftBaseline, 0.0);
        let run = run(left.text, left.font).expect("drawn on the GPU");
        let width = f64::from(run.width) * 20.0;
        let places = Pen::of(&left, Some(&*run)).glyphs(&run);
        let first = places[0].transform;
        assert_eq!(
            first[..4],
            [20.0, 0.0, 0.0, 20.0],
            "an em of 20 px, upright, y up"
        );
        assert!(first[4].abs() < 1e-6 && first[5].abs() < 1e-6, "{first:?}");
        let mid = piece("Ada 12", Anchor::CenterBaseline, 0.0);
        let centred = Pen::of(&mid, Some(&*run)).glyphs(&run);
        assert!(
            (centred[0].transform[4] + width / 2.0).abs() < 1e-4,
            "half its width left"
        );
        // Turned a quarter to the left on the screen (a world rotation of 90°: the angle is the screen's, clockwise):
        // its letters go up the screen, their tops to the left.
        let up = piece("Ada 12", Anchor::LeftBaseline, -std::f32::consts::FRAC_PI_2);
        let turned = Pen::of(&up, Some(&*run)).glyphs(&run);
        let [a, b, c, d, _, _] = turned[0].transform;
        // (The angle is a float32's: a quarter turn leaves a millionth of a pixel.)
        assert!(
            a.abs() < 1e-5 && (b - 20.0).abs() < 1e-5,
            "x along up: {a} {b}"
        );
        assert!(
            (c + 20.0).abs() < 1e-5 && d.abs() < 1e-5,
            "y up to the left: {c} {d}"
        );
        let last = turned.last().expect("glyphs").transform;
        assert!(
            last[4].abs() < 1e-4 && last[5] > width * 0.7,
            "the last letter up the screen: {last:?}"
        );
    }

    /// Lines and rings of the screen go as segments and triangles: a polyline's segments (one more closed), a ring's
    /// fan with its outline, anchored where they start.
    #[test]
    fn lines_and_rings_go_as_segments_and_triangles() {
        let sink = Sink::new(GlyphSlot::default());
        let view = Camera::default();
        sink.begin();
        let pts = [
            Point::new(10.0, 10.0),
            Point::new(40.0, 10.0),
            Point::new(40.0, 30.0),
        ];
        assert!(sink.stroke(&pts, false, Color::BLACK, LINE, &view));
        assert!(sink.stroke(&pts, true, Color::BLACK, LINE, &view));
        let pentagon: Vec<Point> = (0..5)
            .map(|i| {
                let a = i as f32 * std::f32::consts::TAU / 5.0;
                Point::new(100.0 + 20.0 * a.cos(), 100.0 + 20.0 * a.sin())
            })
            .collect();
        assert!(sink.ring(&pentagon, Color::BLACK, &view));
        sink.publish(1.0);
        let set = sink.slot.get();
        let counts: Vec<(usize, usize)> = set
            .texts
            .iter()
            .map(|t| (t.under.len(), t.under.iter().filter(|q| q.triangle).count()))
            .collect();
        assert_eq!(counts, [(2, 0), (3, 0), (3 + 5, 3)]);
        // The first segment from the anchor (the polyline's first point) along x, 1.2 px wide, y up.
        let first = set.texts[0].under[0].transform;
        assert!(
            (first[0] - 30.0).abs() < 1e-9
                && (first[3] - LINE).abs() < 1e-9
                && (first[5] + LINE / 2.0).abs() < 1e-9,
            "{first:?}"
        );
    }

    /// A piece goes with its mask under its letters and its underline over them; a quad that is no parallelogram, and
    /// every piece and quad of a spot that may not send them, stay on the canvas.
    #[test]
    fn a_piece_goes_with_its_mask_and_underline_and_other_spots_stay_on_the_canvas() {
        drawing_fonts::load();
        let sink = Sink::new(GlyphSlot::default());
        let view = Camera::default();
        let plain = piece("Ada", Anchor::LeftBaseline, 0.0);
        assert!(
            !sink.take(&plain, Color::WHITE, &view),
            "not open before a text's spot"
        );
        sink.open.set(true);
        assert!(sink.take(&plain, Color::WHITE, &view));
        assert!(sink.take(
            &Piece {
                mask: 30.0,
                underline: 30.0,
                ..plain
            },
            Color::WHITE,
            &view
        ));
        // A fraction's bar: an underline under a blank.
        assert!(sink.take(
            &Piece {
                text: " ",
                underline: 12.0,
                ..plain
            },
            Color::WHITE,
            &view
        ));
        let square = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(10.0, 5.0),
            Point::new(0.0, 5.0),
        ];
        assert!(sink.quad(square, Color::WHITE, &view));
        let kite = [
            Point::new(0.0, 0.0),
            Point::new(10.0, 0.0),
            Point::new(14.0, 9.0),
            Point::new(0.0, 5.0),
        ];
        assert!(!sink.quad(kite, Color::WHITE, &view), "no parallelogram");
        sink.publish(1.0);
        let set = sink.slot.get();
        assert!(set.version > 0, "a new version");
        assert_eq!(set.texts.len(), 4);
        let [plain, masked, bar, fill] =
            [&set.texts[0], &set.texts[1], &set.texts[2], &set.texts[3]];
        assert_eq!(
            (plain.under.len(), plain.over.len(), plain.halo_px),
            (0, 0, HALO)
        );
        // The mask: 30 px and a tenth of the 20 px size each side, 1.38 of the size and a tenth over and under each,
        // its top left 1.15 and a tenth of the size over the baseline (y up from the anchor).
        let mask = masked.under[0].transform;
        assert!(
            (mask[0] - 34.0).abs() < 1e-9 && (mask[3] + 31.6).abs() < 1e-9,
            "{mask:?}"
        );
        assert!(
            (mask[4] + 2.0).abs() < 1e-9 && (mask[5] - 25.0).abs() < 1e-9,
            "{mask:?}"
        );
        // The underline's halo, then the bar: 2.4 px under the baseline, 1.2 thick.
        assert_eq!(masked.over.len(), 2);
        let line = masked.over[1].transform;
        assert!(
            (line[0] - 30.0).abs() < 1e-9
                && (line[3] + 1.2).abs() < 1e-9
                && (line[5] + 2.4).abs() < 1e-9,
            "{line:?}"
        );
        assert!(
            bar.glyphs.is_empty() && bar.over.len() == 2,
            "a blank's underline alone"
        );
        assert_eq!(
            fill.under[0].transform,
            [10.0, 0.0, 0.0, -5.0, 0.0, 0.0],
            "the screen's quad, y up"
        );
        // A painting with nothing for the GPU after one with texts empties the set; another such does not count. The
        // versions count over every slot of the program (`GlyphSlot::put_keyed`), so another test's painting moves
        // them too: only this slot's own steps are compared.
        let filled = set.version;
        sink.publish(1.0);
        let emptied = sink.slot.get();
        assert!(
            emptied.version > filled && emptied.texts.is_empty(),
            "an empty painting after texts is a new, empty version"
        );
        sink.publish(1.0);
        assert_eq!(
            sink.slot.get().version,
            emptied.version,
            "another empty painting does not count"
        );
    }
}
