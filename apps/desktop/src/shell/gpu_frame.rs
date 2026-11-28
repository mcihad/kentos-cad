//! The drawing area's overlays drawn by its renderer (plan/render.md RM-14, docs/adr/9214): a frame that takes what an
//! Iced canvas frame takes from them (paths stroked, dashed and filled, rectangles, texts, a saved and turned transform)
//! and hands it to the area's glyph layer (`kentos_render_wgpu::glyphs`) as segments, rings and glyphs. A layer draws
//! through the `Paint` trait, onto a canvas frame or onto this one, with the same code.
//!
//! What it draws is anchored on the map (a layer of the drawing: it moves and grows with the camera as a picture of it
//! would, until drawn again) or on the screen (grid north, the scale bar, the axes: they stay where they are).

use iced::widget::canvas::path::lyon_path::{self, iterator::PathIterator};
use iced::widget::canvas::{self, Fill, LineJoin, Path, Stroke, Text};
use iced::{Color, Point, Size, Vector};
use kentos_render_wgpu::glyphs::{GlyphQuad, GlyphText, GlyphUnit};
use kentos_render_wgpu::{Camera, Rgba8};

/// The slots of the drawing area's overlays, each holding what its layer last drew, for the area's renderer to draw
/// over the picture in the stack's order (the drawing's own labels and the screen's marks are the labels' `Spots`').
#[derive(Clone, Debug, Default)]
pub struct GpuLayers {
    pub terrain: crate::labels::GlyphSlot,
    pub alignment: crate::labels::GlyphSlot,
    pub approved: crate::labels::GlyphSlot,
    pub services: crate::labels::GlyphSlot,
    pub selection: crate::labels::GlyphSlot,
    pub edits: crate::labels::GlyphSlot,
}

/// A layer's picture for the renderer: drawn by `paint` on the map of `view` unless `slot` holds what was drawn for
/// `key` already. Its canvas draws nothing. The key goes with the slot, not the canvas: a layer hidden (its slot
/// emptied) and shown again for the same picture is drawn again.
pub fn publish(
    slot: &crate::labels::GlyphSlot,
    key: u64,
    view: Camera,
    size: Size,
    paint: impl FnOnce(&mut GpuFrame),
) {
    if slot.holds(key) {
        return;
    }
    let mut frame = GpuFrame::on_map(view, size);
    paint(&mut frame);
    slot.put_keyed(key, frame.finish(), view.map_px());
}

/// How far a curve strays from the chords it is drawn with, pixels.
const FLATNESS: f32 = 0.2;
/// A corner turning less than this (radians) leaves a gap under a tenth of the stroke's half width: no join is drawn.
const JOIN_TURN: f64 = 0.2;
/// The longest miter as the canvas's tessellator draws one (its default limit, the miter's length over the stroke's
/// width); past it a bevel.
const MITER_LIMIT: f64 = 4.0;

/// What fills the gap two pieces of a stroke leave on the outer side of their corner at `b`, as the stroke's join asks:
/// a miter as long as the canvas draws one (else a bevel), a bevel, or a round fan. `hw`: half the stroke's width.
/// Without it a selection's 2.5 px outline lost a square of a pixel and more at each right-angled corner.
fn join(
    a: [f64; 2],
    b: [f64; 2],
    c: [f64; 2],
    hw: f64,
    how: LineJoin,
    color: Rgba8,
    out: &mut Vec<GlyphQuad>,
) {
    let unit = |p: [f64; 2], q: [f64; 2]| {
        let (dx, dy) = (q[0] - p[0], q[1] - p[1]);
        let l = dx.hypot(dy);
        (l > 0.0 && l.is_finite()).then(|| [dx / l, dy / l])
    };
    let (Some(d1), Some(d2)) = (unit(a, b), unit(b, c)) else {
        return;
    };
    let turn = (d1[0] * d2[1] - d1[1] * d2[0]).atan2(d1[0] * d2[0] + d1[1] * d2[1]);
    if turn.abs() < JOIN_TURN {
        return;
    }
    // The outer side: right of a left turn (y up), left of a right turn. Its normals turn as the pieces do.
    let s = -turn.signum() * hw;
    let (n1, n2) = ([-d1[1] * s, d1[0] * s], [-d2[1] * s, d2[0] * s]);
    let (p1, p2) = ([b[0] + n1[0], b[1] + n1[1]], [b[0] + n2[0], b[1] + n2[1]]);
    // The miter reaches hw / cos(turn / 2) from the corner: its length over the width is 1 / cos(turn / 2).
    let half = (turn / 2.0).cos();
    match how {
        LineJoin::Miter if half > 0.0 && 1.0 / half <= MITER_LIMIT => {
            let m = [n1[0] + n2[0], n1[1] + n2[1]];
            let l = m[0].hypot(m[1]);
            let tip = [b[0] + m[0] / l * hw / half, b[1] + m[1] / l * hw / half];
            out.push(GlyphQuad::triangle(b, p1, tip, color));
            out.push(GlyphQuad::triangle(b, tip, p2, color));
        }
        LineJoin::Round => {
            let steps = (turn.abs() / (std::f64::consts::PI / 8.0)).ceil().max(1.0) as usize;
            let start = n1[1].atan2(n1[0]);
            let mut prev = p1;
            for i in 1..=steps {
                let angle = start + turn * i as f64 / steps as f64;
                let q = [b[0] + hw * angle.cos(), b[1] + hw * angle.sin()];
                out.push(GlyphQuad::triangle(b, prev, q, color));
                prev = q;
            }
        }
        _ => out.push(GlyphQuad::triangle(b, p1, p2, color)),
    }
}

/// What a layer draws with: a canvas frame or the area's renderer.
pub trait Paint {
    fn size(&self) -> Size;
    fn stroke<'a>(&mut self, path: &Path, stroke: impl Into<Stroke<'a>>);
    fn fill(&mut self, path: &Path, fill: impl Into<Fill>);
    fn fill_rectangle(&mut self, top_left: Point, size: Size, fill: impl Into<Fill>);
    fn fill_text(&mut self, text: impl Into<Text>);
    fn translate(&mut self, by: Vector);
    fn rotate(&mut self, angle: f32);
    fn with_save(&mut self, f: impl FnOnce(&mut Self));
    /// What `f` draws is anchored on the map of `view`, not the screen: a mark of a place on the drawing among the
    /// screen's marks (its 0,0) moves with the drawing while the view moves. A canvas frame draws it as it is.
    fn on_map_of(&mut self, _view: &Camera, f: impl FnOnce(&mut Self)) {
        f(self);
    }
}

impl Paint for canvas::Frame {
    fn size(&self) -> Size {
        canvas::Frame::size(self)
    }
    fn stroke<'a>(&mut self, path: &Path, stroke: impl Into<Stroke<'a>>) {
        canvas::Frame::stroke(self, path, stroke);
    }
    fn fill(&mut self, path: &Path, fill: impl Into<Fill>) {
        canvas::Frame::fill(self, path, fill);
    }
    fn fill_rectangle(&mut self, top_left: Point, size: Size, fill: impl Into<Fill>) {
        canvas::Frame::fill_rectangle(self, top_left, size, fill);
    }
    fn fill_text(&mut self, text: impl Into<Text>) {
        canvas::Frame::fill_text(self, text);
    }
    fn translate(&mut self, by: Vector) {
        canvas::Frame::translate(self, by);
    }
    fn rotate(&mut self, angle: f32) {
        canvas::Frame::rotate(self, angle);
    }
    fn with_save(&mut self, f: impl FnOnce(&mut Self)) {
        canvas::Frame::with_save(self, f);
    }
}

/// Where a frame's drawing is anchored.
#[derive(Clone, Copy, Debug)]
enum Anchor {
    /// On the map of this view (its screen's pixels are the frame's).
    Map(Camera),
    /// On the screen.
    Screen,
}

/// A frame drawn by the area's renderer.
pub struct GpuFrame {
    size: Size,
    anchor: Anchor,
    /// The current transform, as Iced's: x' = a x + c y + e, y' = b x + d y + f.
    at: [f32; 6],
    saved: Vec<[f32; 6]>,
    texts: Vec<GlyphText>,
}

fn rgba(c: Color) -> Rgba8 {
    let byte = |v: f32| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
    Rgba8([byte(c.r), byte(c.g), byte(c.b), byte(c.a)])
}

fn solid(style: canvas::Style) -> Option<Color> {
    match style {
        canvas::Style::Solid(c) => Some(c),
        // A gradient's first stop: no overlay of the area paints one.
        canvas::Style::Gradient(_) => None,
    }
}

impl GpuFrame {
    /// A frame for a layer of the drawing in `view`, `size` its area.
    pub fn on_map(view: Camera, size: Size) -> GpuFrame {
        GpuFrame {
            size,
            anchor: Anchor::Map(view),
            at: [1.0, 0.0, 0.0, 1.0, 0.0, 0.0],
            saved: Vec::new(),
            texts: Vec::new(),
        }
    }

    /// A frame for the screen's own marks.
    pub fn on_screen(size: Size) -> GpuFrame {
        GpuFrame {
            anchor: Anchor::Screen,
            ..GpuFrame::on_map(Camera::default(), size)
        }
    }

    pub fn finish(self) -> Vec<GlyphText> {
        self.texts
    }

    fn point(&self, p: lyon_path::math::Point) -> Point {
        let [a, b, c, d, e, f] = self.at;
        Point::new(a * p.x + c * p.y + e, b * p.x + d * p.y + f)
    }

    /// A text anchored at the screen's `p`, its shapes and glyphs from there in pixels, y up.
    fn anchored(&self, p: Point, color: Color) -> GlyphText {
        let (at, unit) = match self.anchor {
            // The frame's points are the area's own (its transform applied, a twisted view's turn among it): they do
            // not turn again.
            Anchor::Map(view) => {
                let m = view.screen_to_map(f64::from(p.x), f64::from(p.y));
                ([m.x, m.y], GlyphUnit::Px { turns: false })
            }
            Anchor::Screen => ([f64::from(p.x), f64::from(p.y)], GlyphUnit::Screen),
        };
        GlyphText {
            at,
            unit,
            color: rgba(color),
            halo: Rgba8([0; 4]),
            halo_px: 0.0,
            glyphs: Vec::new(),
            under: Vec::new(),
            over: Vec::new(),
        }
    }

    /// The path's figures on the screen: each a polyline, and whether it is closed.
    fn figures(&self, path: &Path) -> Vec<(Vec<Point>, bool)> {
        let mut out: Vec<(Vec<Point>, bool)> = Vec::new();
        for event in path.raw().iter().flattened(FLATNESS) {
            match event {
                lyon_path::Event::Begin { at } => out.push((vec![self.point(at)], false)),
                lyon_path::Event::Line { to, .. } => {
                    if let Some((pts, _)) = out.last_mut() {
                        pts.push(self.point(to));
                    }
                }
                lyon_path::Event::End { close, .. } => {
                    if let Some((_, closed)) = out.last_mut() {
                        *closed = close;
                    }
                }
                _ => {}
            }
        }
        out
    }
}

/// Twice a ring's signed area (counter-clockwise positive, y up).
fn twice_area(ring: &[[f64; 2]]) -> f64 {
    (0..ring.len())
        .map(|i| {
            let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
            a[0] * b[1] - b[0] * a[1]
        })
        .sum()
}

/// Whether `p` is inside `ring` (even-odd).
fn inside(ring: &[[f64; 2]], p: [f64; 2]) -> bool {
    let mut odd = false;
    for i in 0..ring.len() {
        let (a, b) = (ring[i], ring[(i + 1) % ring.len()]);
        if (a[1] > p[1]) != (b[1] > p[1])
            && p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
        {
            odd = !odd;
        }
    }
    odd
}

/// A path's rings as polygons, each an outer ring and its holes, as a nonzero fill reads them: rings turning as the
/// largest one does are outer, the others holes of the smallest outer ring they lie in (a letter's counters, a terrain's
/// gaps; a letter's separate parts are separate outers).
fn polygons(rings: Vec<Vec<[f64; 2]>>) -> Vec<Vec<Vec<[f64; 2]>>> {
    let Some(largest) = rings
        .iter()
        .map(|r| twice_area(r))
        .max_by(|a, b| a.abs().total_cmp(&b.abs()))
    else {
        return Vec::new();
    };
    let (outers, holes): (Vec<_>, Vec<_>) = rings
        .into_iter()
        .partition(|r| (twice_area(r) >= 0.0) == (largest >= 0.0));
    let mut out: Vec<Vec<Vec<[f64; 2]>>> = outers.into_iter().map(|o| vec![o]).collect();
    for hole in holes {
        let Some(&p) = hole.first() else { continue };
        let home = out
            .iter_mut()
            .filter(|polygon| inside(&polygon[0], p))
            .min_by(|a, b| twice_area(&a[0]).abs().total_cmp(&twice_area(&b[0]).abs()));
        if let Some(polygon) = home {
            polygon.push(hole);
        }
    }
    out
}

/// `pts` cut by a dash pattern (`on`, `off`, … pixels from `offset`'s place in it): its drawn pieces.
fn dashed(pts: &[Point], pattern: &[f32], offset: usize) -> Vec<Vec<Point>> {
    let total: f32 = pattern.iter().sum();
    if pattern.is_empty() || total <= 0.0 {
        return vec![pts.to_vec()];
    }
    let mut out: Vec<Vec<Point>> = Vec::new();
    let (mut index, mut left) = (offset % pattern.len(), pattern[offset % pattern.len()]);
    let mut on = index % 2 == 0;
    let mut piece: Vec<Point> = pts.first().copied().filter(|_| on).into_iter().collect();
    for w in pts.windows(2) {
        let (a, b) = (w[0], w[1]);
        let length = a.distance(b);
        let mut along = 0.0;
        while length - along > left {
            along += left;
            let p = Point::new(
                a.x + (b.x - a.x) * along / length,
                a.y + (b.y - a.y) * along / length,
            );
            if on {
                piece.push(p);
                out.push(std::mem::take(&mut piece));
            } else {
                piece = vec![p];
            }
            on = !on;
            index = (index + 1) % pattern.len();
            left = pattern[index];
        }
        left -= length - along;
        if on {
            piece.push(b);
        }
    }
    if on && piece.len() > 1 {
        out.push(piece);
    }
    out
}

impl Paint for GpuFrame {
    fn size(&self) -> Size {
        self.size
    }

    fn stroke<'a>(&mut self, path: &Path, stroke: impl Into<Stroke<'a>>) {
        let stroke = stroke.into();
        let Some(color) = solid(stroke.style) else {
            return;
        };
        let width = f64::from(stroke.width.max(0.1));
        // One text for the call, anchored at its first point: a picture of its pieces grows about any of its points.
        let figures = self.figures(path);
        let Some(first) = figures.first().and_then(|(pts, _)| pts.first().copied()) else {
            return;
        };
        let at = |p: &Point| [f64::from(p.x - first.x), -f64::from(p.y - first.y)];
        let mut text = self.anchored(first, color);
        let (hw, ink) = (width / 2.0, rgba(color));
        for (mut pts, closed) in figures {
            pts.dedup();
            if closed && let Some(&start) = pts.first() {
                pts.push(start);
            }
            let whole = stroke.line_dash.segments.is_empty();
            for piece in dashed(&pts, stroke.line_dash.segments, stroke.line_dash.offset) {
                let piece: Vec<[f64; 2]> = piece.iter().map(at).collect();
                text.under.extend(
                    piece
                        .windows(2)
                        .filter_map(|w| GlyphQuad::segment(w[0], w[1], width, ink)),
                );
                // The corners' joins; a closed figure drawn whole turns at its start too.
                for w in piece.windows(3) {
                    join(w[0], w[1], w[2], hw, stroke.line_join, ink, &mut text.under);
                }
                if closed && whole && piece.len() > 3 {
                    let n = piece.len();
                    join(
                        piece[n - 2],
                        piece[0],
                        piece[1],
                        hw,
                        stroke.line_join,
                        ink,
                        &mut text.under,
                    );
                }
            }
        }
        if !text.under.is_empty() {
            self.texts.push(text);
        }
    }

    fn fill(&mut self, path: &Path, fill: impl Into<Fill>) {
        let fill = fill.into();
        let Some(color) = solid(fill.style) else {
            return;
        };
        let figures = self.figures(path);
        let Some(first) = figures.first().and_then(|(pts, _)| pts.first().copied()) else {
            return;
        };
        let at = |p: &Point| [f64::from(p.x - first.x), -f64::from(p.y - first.y)];
        let rings: Vec<Vec<[f64; 2]>> = figures
            .iter()
            .filter(|(pts, _)| pts.len() >= 3)
            .map(|(pts, _)| pts.iter().map(at).collect())
            .collect();
        let mut text = self.anchored(first, color);
        for polygon in polygons(rings) {
            text.under
                .extend(GlyphQuad::polygon(&polygon, 1.0, rgba(color)));
        }
        self.texts.push(text);
    }

    fn fill_rectangle(&mut self, top_left: Point, size: Size, fill: impl Into<Fill>) {
        self.fill(&Path::rectangle(top_left, size), fill);
    }

    fn fill_text(&mut self, text: impl Into<Text>) {
        let text = text.into();
        if text.content.trim().is_empty() {
            return;
        }
        let Some(run) = crate::labels::text_run(&text.content, text.font) else {
            // A face the glyph layer does not hold (the system's own): its letters' outlines, filled. No text is lost.
            text.draw_with(|path, color| self.fill(&path, color));
            return;
        };
        let size = f64::from(text.size.0);
        let line = f64::from(text.line_height.to_absolute(text.size).0);
        let width = f64::from(run.width) * size;
        let along = match text.align_x {
            iced::advanced::text::Alignment::Center => -width / 2.0,
            iced::advanced::text::Alignment::Right => -width,
            _ => 0.0,
        };
        // The baseline under the line box's top: its leading shared over and under, as the text system lays a line.
        let baseline = f64::from(run.baseline) * size + (line - size) / 2.0;
        let top = match text.align_y {
            iced::alignment::Vertical::Top => 0.0,
            iced::alignment::Vertical::Center => -line / 2.0,
            iced::alignment::Vertical::Bottom => -line,
        };
        // The frame's transform turns and scales the letters with it (y down on the screen, up for the glyphs).
        let [a, b, c, d, _, _] = self.at.map(f64::from);
        let out = |x: f64, y: f64| [a * x + c * y, -(b * x + d * y)];
        let origin = self.point(lyon_path::math::point(text.position.x, text.position.y));
        let mut glyphs = self.anchored(origin, text.color);
        let (ex, ey) = (out(size, 0.0), out(0.0, -size));
        glyphs.glyphs = run
            .glyphs
            .iter()
            .map(|(key, [x, y])| {
                let p = out(
                    along + size * f64::from(*x),
                    top + baseline - size * f64::from(*y),
                );
                kentos_render_wgpu::glyphs::GlyphPlace {
                    key: *key,
                    transform: [ex[0], ex[1], ey[0], ey[1], p[0], p[1]],
                }
            })
            .collect();
        self.texts.push(glyphs);
    }

    fn translate(&mut self, by: Vector) {
        let [a, b, c, d, e, f] = self.at;
        self.at = [a, b, c, d, e + a * by.x + c * by.y, f + b * by.x + d * by.y];
    }

    fn rotate(&mut self, angle: f32) {
        let (s, k) = angle.sin_cos();
        let [a, b, c, d, e, f] = self.at;
        self.at = [
            a * k + c * s,
            b * k + d * s,
            -a * s + c * k,
            -b * s + d * k,
            e,
            f,
        ];
    }

    fn with_save(&mut self, f: impl FnOnce(&mut Self)) {
        self.saved.push(self.at);
        f(self);
        if let Some(at) = self.saved.pop() {
            self.at = at;
        }
    }

    fn on_map_of(&mut self, view: &Camera, f: impl FnOnce(&mut Self)) {
        let anchor = std::mem::replace(&mut self.anchor, Anchor::Map(*view));
        f(self);
        self.anchor = anchor;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A dash pattern cuts a line into its drawn pieces, the pattern going on round a corner.
    #[test]
    fn a_dash_pattern_cuts_a_polyline_into_its_pieces() {
        let line = [
            Point::new(0.0, 0.0),
            Point::new(20.0, 0.0),
            Point::new(20.0, 10.0),
        ];
        let pieces = dashed(&line, &[9.0, 4.0], 0);
        let lengths: Vec<f32> = pieces
            .iter()
            .map(|p| p.windows(2).map(|w| w[0].distance(w[1])).sum::<f32>())
            .collect();
        // 9 on, 4 off, 9 on (7 along, 2 down), 4 off, the last 4 on.
        assert_eq!(lengths.len(), 3, "{lengths:?}");
        assert!(
            (lengths[0] - 9.0).abs() < 1e-4
                && (lengths[1] - 9.0).abs() < 1e-4
                && (lengths[2] - 4.0).abs() < 1e-4,
            "{lengths:?}"
        );
        assert_eq!(pieces[1].len(), 3, "the second piece turns the corner");
    }

    /// A stroked and a filled path, and a text, go anchored on the map where the canvas draws them.
    #[test]
    fn paths_and_texts_go_to_the_glyph_layer() {
        crate::drawing_fonts::load();
        let mut view = Camera::default();
        view.set_size(400.0, 300.0);
        let mut frame = GpuFrame::on_map(view, Size::new(400.0, 300.0));
        frame.stroke(
            &Path::line(Point::new(10.0, 10.0), Point::new(50.0, 10.0)),
            Stroke::default().with_width(2.0),
        );
        frame.fill(&Path::circle(Point::new(100.0, 100.0), 6.0), Color::BLACK);
        let font = crate::drawing_fonts::font(kentos_commands::DrawingFont::Arimo, 400, false);
        frame.fill_text(Text {
            content: "km 0+100".into(),
            position: Point::new(200.0, 150.0),
            font,
            ..Text::default()
        });
        let texts = frame.finish();
        assert_eq!(texts.len(), 3);
        assert_eq!(texts[0].under.len(), 1, "one segment");
        assert!(
            texts[1].under.iter().any(|q| q.triangle),
            "the circle's fan"
        );
        let run = crate::labels::text_run("km 0+100", font).expect("a drawing face's run");
        assert_eq!(
            texts[2].glyphs.len(),
            run.glyphs.len(),
            "every letter placed"
        );
        assert!(texts[2].glyphs.len() >= 7);
        let at = view.screen_to_map(10.0, 10.0);
        assert_eq!(texts[0].at, [at.x, at.y]);
    }

    /// A closed square's corners are joined as its stroke asks: a miter fills each corner whole (two triangles), a
    /// bevel cuts it (one), a round join fans; a straight run of points needs none.
    #[test]
    fn a_strokes_corners_are_joined_as_it_asks() {
        let square = Path::rectangle(Point::new(10.0, 10.0), Size::new(40.0, 40.0));
        let quads = |join: LineJoin| {
            let mut frame = GpuFrame::on_screen(Size::new(100.0, 100.0));
            frame.stroke(
                &square,
                Stroke {
                    line_join: join,
                    ..Stroke::default().with_width(2.5)
                },
            );
            let texts = frame.finish();
            let under = &texts[0].under;
            (
                under.iter().filter(|q| !q.triangle).count(),
                under.iter().filter(|q| q.triangle).count(),
            )
        };
        assert_eq!(quads(LineJoin::Miter), (4, 8));
        assert_eq!(quads(LineJoin::Bevel), (4, 4));
        let (sides, fans) = quads(LineJoin::Round);
        assert_eq!(sides, 4);
        assert!(fans >= 8, "{fans}");
        // A miter's tip is the outer corner of the stroke: 1.25 px out each way from the square's corner.
        let mut frame = GpuFrame::on_screen(Size::new(100.0, 100.0));
        frame.stroke(&square, Stroke::default().with_width(2.5));
        let texts = frame.finish();
        let tips: Vec<[f64; 2]> = texts[0]
            .under
            .iter()
            .filter(|q| q.triangle)
            .map(|q| {
                [
                    q.transform[4] + q.transform[0],
                    q.transform[5] + q.transform[1],
                ]
            })
            .collect();
        // The anchor is the square's first corner (10, 10); y up from it. The corner opposite is (50, 50) on screen.
        assert!(
            tips.iter()
                .any(|t| (t[0] - 41.25).abs() < 1e-9 && (t[1] + 41.25).abs() < 1e-9),
            "{tips:?}"
        );
        let mut frame = GpuFrame::on_screen(Size::new(100.0, 100.0));
        let straight = Path::new(|p| {
            p.move_to(Point::new(0.0, 0.0));
            p.line_to(Point::new(10.0, 0.0));
            p.line_to(Point::new(20.0, 0.1));
        });
        frame.stroke(&straight, Stroke::default().with_width(2.0));
        assert!(
            frame.finish()[0].under.iter().all(|q| !q.triangle),
            "a turn of a hundredth: no join"
        );
    }

    /// The screen's marks are lettered in the interface's face: the glyph layer takes it (none of them is lost), and
    /// they stay on the screen.
    #[test]
    fn the_marks_letters_in_the_interface_face_go_as_glyphs() {
        kentos_ui::theme::typography::load();
        let mut frame = GpuFrame::on_screen(Size::new(400.0, 300.0));
        frame.fill_text(Text {
            content: "100 m".into(),
            position: Point::new(300.0, 280.0),
            font: kentos_ui::theme::typography::ui_strong(),
            ..Text::default()
        });
        let texts = frame.finish();
        assert_eq!(texts.len(), 1);
        assert!(texts[0].glyphs.len() >= 4, "{}", texts[0].glyphs.len());
        assert!(
            texts[0]
                .glyphs
                .iter()
                .all(|g| g.key.font.0 >= kentos_render_wgpu::glyphs::EXTRA_FACES),
            "an interface face"
        );
        assert_eq!(texts[0].unit, GlyphUnit::Screen);
        assert_eq!(texts[0].at, [300.0, 280.0]);
    }

    /// The 0,0 mark among the screen's marks moves with the drawing: anchored on the map, the others on the screen.
    #[test]
    fn a_mark_of_a_place_goes_on_the_map_among_the_screens() {
        let mut view = Camera::default();
        view.set_size(400.0, 300.0);
        let mut frame = GpuFrame::on_screen(Size::new(400.0, 300.0));
        let dot = Path::circle(Point::new(20.0, 20.0), 3.0);
        frame.fill(&dot, Color::BLACK);
        frame.on_map_of(&view, |f| f.fill(&dot, Color::BLACK));
        frame.fill(&dot, Color::BLACK);
        let units: Vec<GlyphUnit> = frame.finish().iter().map(|t| t.unit).collect();
        assert_eq!(
            units,
            [
                GlyphUnit::Screen,
                GlyphUnit::Px { turns: false },
                GlyphUnit::Screen
            ]
        );
    }
}
