//! A sheet's display list as a page's content stream: every primitive the
//! core draws as a vector (paths, rectangles, arcs as Bézier curves), text
//! in the embedded faces at the core's positions, pictures as images,
//! clips (turned ones too) and group opacity as the graphics state's alpha.
//!
//! The page's user space is the PDF's: points, y up. The core's paper is
//! micrometres, y down; `x` and `y` below turn one into the other.

use std::collections::BTreeMap;

use pdf_writer::types::{LineCapStyle, LineJoinStyle, TextRenderingMode};
use pdf_writer::{Content, Name, Ref, Str};

use super::fonts::Fonts;
use crate::display::{ArcSeg, ClipPrim, ImagePrim, Seg, TextPrim};
use crate::style::{LineCap, LineJoin, Stroke};
use crate::units::{Mdeg, RectUm, Um, corners};

/// Points per micrometre.
pub(super) const PT: f64 = 72.0 / 25_400.0;

/// A colour of the core (`#rgb`, `#rrggbb`, `#rrggbbaa`): RGB and alpha, 0–1; none for anything
/// else (“none”).
pub(super) fn color(s: &str) -> Option<([f32; 3], f32)> {
    let h = s.strip_prefix('#')?;
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    let (rgb, a) = match h.len() {
        3 => {
            let d = |i: usize| {
                u8::from_str_radix(h.get(i..i + 1)?, 16)
                    .ok()
                    .map(|v| v * 17)
            };
            ([d(0)?, d(1)?, d(2)?], 255)
        }
        6 => ([byte(0)?, byte(2)?, byte(4)?], 255),
        8 => ([byte(0)?, byte(2)?, byte(4)?], byte(6)?),
        _ => return None,
    };
    Some((rgb.map(|v| f32::from(v) / 255.0), f32::from(a) / 255.0))
}

/// The page being written: its content and the resources it names.
pub(super) struct Page<'a> {
    pub c: Content,
    /// The page's height, points.
    h: f64,
    pub fonts: &'a Fonts,
    pub used_fonts: BTreeMap<String, Ref>,
    pub xobjects: BTreeMap<String, Ref>,
    /// Graphics states by name: fill and stroke alpha.
    pub gstates: BTreeMap<String, (f32, f32)>,
    /// Optional content groups by name.
    pub properties: BTreeMap<String, Ref>,
    /// The groups' opacity in force.
    alpha: f32,
    /// The graphics state's alpha as last set, saved with `q`.
    set: (f32, f32),
    saved: Vec<(f32, f32, f32)>,
}

impl<'a> Page<'a> {
    pub fn new(height_um: Um, fonts: &'a Fonts) -> Self {
        Page {
            c: Content::new(),
            h: f64::from(height_um) * PT,
            fonts,
            used_fonts: BTreeMap::new(),
            xobjects: BTreeMap::new(),
            gstates: BTreeMap::new(),
            properties: BTreeMap::new(),
            alpha: 1.0,
            set: (1.0, 1.0),
            saved: Vec::new(),
        }
    }

    /// A paper point (µm, y down) on the page (points, y up).
    pub fn xy(&self, p: [f64; 2]) -> (f32, f32) {
        ((p[0] * PT) as f32, (self.h - p[1] * PT) as f32)
    }

    pub fn save(&mut self) {
        self.c.save_state();
        self.saved.push((self.alpha, self.set.0, self.set.1));
    }

    pub fn restore(&mut self) {
        self.c.restore_state();
        if let Some((a, f, s)) = self.saved.pop() {
            self.alpha = a;
            self.set = (f, s);
        }
    }

    /// A group's opacity on everything painted until its `restore`.
    pub fn group(&mut self, opacity: u8) {
        self.save();
        self.alpha *= f32::from(opacity.min(100)) / 100.0;
    }

    /// The graphics state's alpha for the next painting (the groups' times the colours').
    pub fn alpha_for(&mut self, fill: f32, stroke: f32) {
        let want = (self.alpha * fill, self.alpha * stroke);
        if (want.0 - self.set.0).abs() < 1e-4 && (want.1 - self.set.1).abs() < 1e-4 {
            return;
        }
        let name = format!(
            "A{}_{}",
            (want.0 * 1000.0).round() as u32,
            (want.1 * 1000.0).round() as u32
        );
        self.c.set_parameters(Name(name.as_bytes()));
        self.gstates.insert(name, want);
        self.set = want;
    }

    // ── Paths ────────────────────────────────────────────────────────────

    pub fn move_to(&mut self, p: [f64; 2]) {
        let (x, y) = self.xy(p);
        self.c.move_to(x, y);
    }

    pub fn line_to(&mut self, p: [f64; 2]) {
        let (x, y) = self.xy(p);
        self.c.line_to(x, y);
    }

    fn cubic(&mut self, a: [f64; 2], b: [f64; 2], c: [f64; 2]) {
        let (x1, y1) = self.xy(a);
        let (x2, y2) = self.xy(b);
        let (x3, y3) = self.xy(c);
        self.c.cubic_to(x1, y1, x2, y2, x3, y3);
    }

    /// A polygon (closed) or a polyline of paper points.
    pub fn poly(&mut self, pts: &[[f64; 2]], closed: bool) {
        let Some(first) = pts.first() else {
            return;
        };
        self.move_to(*first);
        for p in &pts[1..] {
            self.line_to(*p);
        }
        if closed {
            self.c.close_path();
        }
    }

    /// The core's path: moves, lines, arcs of ellipses (as Bézier curves) and closes.
    pub fn path(&mut self, segs: &[Seg]) {
        let mut open = false;
        for s in segs {
            match s {
                Seg::M(p) => {
                    self.move_to(f(*p));
                    open = true;
                }
                Seg::L(p) => {
                    if open {
                        self.line_to(f(*p));
                    } else {
                        self.move_to(f(*p));
                        open = true;
                    }
                }
                Seg::A(a) => {
                    self.arc(a, open);
                    open = true;
                }
                Seg::Z => {
                    self.c.close_path();
                }
            }
        }
    }

    /// An arc: a line to its start first (a move when the path has no point yet), then Bézier
    /// curves of at most a quarter turn each.
    fn arc(&mut self, a: &ArcSeg, open: bool) {
        let (rx, ry) = (f64::from(a.radius[0]), f64::from(a.radius[1]));
        let c = f(a.center);
        let rot = rad(a.rotation);
        let (sr, cr) = (libm::sin(rot), libm::cos(rot));
        // A point and the derivative of the ellipse at angle t (clockwise on the paper).
        let at = |t: f64| {
            let (s, k) = (libm::sin(t), libm::cos(t));
            let (x, y) = (rx * k, ry * s);
            [c[0] + x * cr - y * sr, c[1] + x * sr + y * cr]
        };
        let d = |t: f64| {
            let (s, k) = (libm::sin(t), libm::cos(t));
            let (x, y) = (-rx * s, ry * k);
            [x * cr - y * sr, x * sr + y * cr]
        };
        let start = rad(a.start);
        let sweep = rad(a.sweep);
        let p0 = at(start);
        if open {
            self.line_to(p0);
        } else {
            self.move_to(p0);
        }
        if sweep == 0.0 || (rx <= 0.0 && ry <= 0.0) {
            return;
        }
        let n = libm::ceil(sweep.abs() / (core::f64::consts::FRAC_PI_2 + 1e-9)).max(1.0) as usize;
        let step = sweep / n as f64;
        let k = 4.0 / 3.0 * libm::tan(step / 4.0);
        for i in 0..n {
            let t0 = start + step * i as f64;
            let t1 = t0 + step;
            let (a0, a1) = (at(t0), at(t1));
            let (d0, d1) = (d(t0), d(t1));
            self.cubic(
                [a0[0] + k * d0[0], a0[1] + k * d0[1]],
                [a1[0] - k * d1[0], a1[1] - k * d1[1]],
                a1,
            );
        }
    }

    /// A rectangle turned about its centre, its corners rounded by `radius`.
    pub fn rect(&mut self, r: &RectUm, rotation: Mdeg, radius: Um) {
        let radius = f64::from(radius)
            .min(f64::from(r.width) / 2.0)
            .min(f64::from(r.height) / 2.0)
            .max(0.0);
        if radius <= 0.0 {
            let cs = corners(r, rotation);
            self.poly(&cs, true);
            return;
        }
        let c = r.center();
        let (w2, h2) = (f64::from(r.width) / 2.0, f64::from(r.height) / 2.0);
        let rot = rad(rotation);
        let (s, k) = (libm::sin(rot), libm::cos(rot));
        let turn = |x: f64, y: f64| [c[0] + x * k - y * s, c[1] + x * s + y * k];
        // Corner arcs as quarter circles, clockwise from the top edge.
        let q = 4.0 / 3.0 * (core::f64::consts::SQRT_2 - 1.0) * radius;
        self.move_to(turn(-w2 + radius, -h2));
        self.line_to(turn(w2 - radius, -h2));
        self.cubic(
            turn(w2 - radius + q, -h2),
            turn(w2, -h2 + radius - q),
            turn(w2, -h2 + radius),
        );
        self.line_to(turn(w2, h2 - radius));
        self.cubic(
            turn(w2, h2 - radius + q),
            turn(w2 - radius + q, h2),
            turn(w2 - radius, h2),
        );
        self.line_to(turn(-w2 + radius, h2));
        self.cubic(
            turn(-w2 + radius - q, h2),
            turn(-w2, h2 - radius + q),
            turn(-w2, h2 - radius),
        );
        self.line_to(turn(-w2, -h2 + radius));
        self.cubic(
            turn(-w2, -h2 + radius - q),
            turn(-w2 + radius - q, -h2),
            turn(-w2 + radius, -h2),
        );
        self.c.close_path();
    }

    // ── Painting ─────────────────────────────────────────────────────────

    /// The path just built filled and/or stroked; dropped when neither paints.
    pub fn paint(&mut self, fill: Option<&str>, stroke: Option<&Stroke>, even_odd: bool) {
        let fill = fill.and_then(color);
        let line = stroke.and_then(|s| color(&s.color).map(|c| (s, c)));
        self.alpha_for(
            fill.map_or(1.0, |(_, a)| a),
            line.map_or(1.0, |(_, (_, a))| a),
        );
        if let Some((rgb, _)) = fill {
            self.c.set_fill_rgb(rgb[0], rgb[1], rgb[2]);
        }
        if let Some((s, (rgb, _))) = line {
            self.c.set_stroke_rgb(rgb[0], rgb[1], rgb[2]);
            self.line_style(
                f64::from(s.width) * PT,
                &s.dash
                    .iter()
                    .map(|d| f64::from(*d) * PT)
                    .collect::<Vec<_>>(),
                s.cap,
                s.join,
            );
        }
        match (fill.is_some(), line.is_some(), even_odd) {
            (true, true, false) => self.c.fill_nonzero_and_stroke(),
            (true, true, true) => self.c.fill_even_odd_and_stroke(),
            (true, false, false) => self.c.fill_nonzero(),
            (true, false, true) => self.c.fill_even_odd(),
            (false, true, _) => self.c.stroke(),
            (false, false, _) => self.c.end_path(),
        };
    }

    /// A line's width, dashes (points), ends and corners.
    pub fn line_style(&mut self, width: f64, dash: &[f64], cap: LineCap, join: LineJoin) {
        self.c.set_line_width(width.max(0.0) as f32);
        self.c.set_line_cap(match cap {
            LineCap::Butt => LineCapStyle::ButtCap,
            LineCap::Round => LineCapStyle::RoundCap,
            LineCap::Square => LineCapStyle::ProjectingSquareCap,
        });
        self.c.set_line_join(match join {
            LineJoin::Miter => LineJoinStyle::MiterJoin,
            LineJoin::Round => LineJoinStyle::RoundJoin,
            LineJoin::Bevel => LineJoinStyle::BevelJoin,
        });
        let dash: Vec<f32> = dash.iter().map(|d| d.max(0.0) as f32).collect();
        if dash.iter().any(|d| *d > 0.0) {
            self.c.set_dash_pattern(dash, 0.0);
        } else {
            self.c.set_dash_pattern([], 0.0);
        }
    }

    // ── Clips ────────────────────────────────────────────────────────────

    /// A clip until its `restore`: the rectangle turned about its centre.
    pub fn clip(&mut self, c: &ClipPrim) {
        self.save();
        self.rect(&c.rect, c.rotation, 0);
        self.c.clip_nonzero();
        self.c.end_path();
    }

    // ── Text ─────────────────────────────────────────────────────────────

    /// A line of the core's text from the start of its baseline, turned clockwise about it.
    pub fn text(&mut self, t: &TextPrim) {
        let rotation = rad(t.rotation);
        self.text_at(
            &t.text,
            (&t.font, t.weight, t.italic),
            f64::from(t.size) * PT,
            f(t.at),
            rotation,
            &t.color,
            t.halo.as_deref(),
        );
    }

    /// Text in a face at a paper point (µm), `size` in points, `turn` clockwise on the paper (radians).
    #[allow(clippy::too_many_arguments)]
    pub fn text_at(
        &mut self,
        text: &str,
        face: (&str, u16, bool),
        size: f64,
        at: [f64; 2],
        turn: f64,
        fill: &str,
        halo: Option<&str>,
    ) {
        let fonts = self.fonts;
        let Some((rgb, alpha)) = color(fill) else {
            return;
        };
        if size <= 0.0 || text.is_empty() {
            return;
        }
        // The pieces its faces draw (a letter the face lacks in another face, never a box).
        let runs = fonts.runs(face.0, face.1, face.2, text);
        if runs.is_empty() {
            return;
        }
        for (font, _) in &runs {
            self.used_fonts.insert(font.name.clone(), font.font);
        }
        let (x, y) = self.xy(at);
        let (s, k) = (libm::sin(turn) as f32, libm::cos(turn) as f32);
        // Clockwise on the paper is clockwise on the page: the baseline (cos, −sin).
        let matrix = [k, -s, s, k, x, y];
        let show = |c: &mut Content, mode: TextRenderingMode| {
            c.begin_text();
            c.set_text_matrix(matrix);
            c.set_text_rendering_mode(mode);
            // Each piece goes on where the one before it ended.
            for (font, codes) in &runs {
                c.set_font(Name(font.name.as_bytes()), size as f32);
                c.show(Str(codes));
            }
            c.end_text();
        };
        if let Some((hrgb, halpha)) = halo.and_then(color) {
            self.alpha_for(1.0, halpha);
            self.c.set_stroke_rgb(hrgb[0], hrgb[1], hrgb[2]);
            self.c.set_line_width((size / 5.0) as f32);
            self.c.set_line_join(LineJoinStyle::RoundJoin);
            self.c.set_dash_pattern([], 0.0);
            show(&mut self.c, TextRenderingMode::Stroke);
        }
        self.alpha_for(alpha, 1.0);
        self.c.set_fill_rgb(rgb[0], rgb[1], rgb[2]);
        show(&mut self.c, TextRenderingMode::Fill);
    }

    // ── Pictures ─────────────────────────────────────────────────────────

    /// A picture (its image object named `name`) in its rectangle, turned about its centre;
    /// without one, the missing picture's light box.
    pub fn image(&mut self, img: &ImagePrim, image: Option<(&str, Ref)>) {
        let opacity = f32::from(img.opacity.min(100)) / 100.0;
        match image {
            Some((name, r)) => {
                self.xobjects.insert(name.to_owned(), r);
                self.save();
                self.alpha_for(opacity, opacity);
                self.place(&img.rect, img.rotation);
                self.c.x_object(Name(name.as_bytes()));
                self.restore();
            }
            None => {
                self.rect(&img.rect, img.rotation, 0);
                self.paint(Some("#f2f2f2"), Some(&Stroke::solid("#b4b4b4", 180)), false);
            }
        }
    }

    /// The unit square onto a rectangle turned about its centre (an image's space).
    pub fn place(&mut self, r: &RectUm, rotation: Mdeg) {
        let cs = corners(r, rotation);
        // Image space: (0,0) is the bottom left (the paper's bottom-left corner), (1,1) the top right.
        let (bl, br, tl) = (self.xy(cs[3]), self.xy(cs[2]), self.xy(cs[0]));
        self.c.transform([
            br.0 - bl.0,
            br.1 - bl.1,
            tl.0 - bl.0,
            tl.1 - bl.1,
            bl.0,
            bl.1,
        ]);
    }
}

/// A point of the core (µm) as floats.
pub(super) fn f(p: [i32; 2]) -> [f64; 2] {
    [f64::from(p[0]), f64::from(p[1])]
}

/// Thousandths of a degree as radians.
pub(super) fn rad(a: Mdeg) -> f64 {
    f64::from(a) / 1000.0 * core::f64::consts::PI / 180.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_read_as_the_core_writes_them() {
        assert_eq!(color("#ff0000"), Some(([1.0, 0.0, 0.0], 1.0)));
        assert_eq!(color("#fff"), Some(([1.0, 1.0, 1.0], 1.0)));
        assert_eq!(
            color("#00000080").map(|(_, a)| (a * 255.0).round()),
            Some(128.0)
        );
        assert_eq!(color("none"), None);
        assert_eq!(color("#12"), None);
    }

    /// The paper's corner (0, 0) is the page's top left; a micrometre is 72/25 400 of a point.
    #[test]
    fn the_paper_is_the_page() {
        let fonts = Fonts::default();
        let page = Page::new(297_000, &fonts);
        let (x, y) = page.xy([420_000.0, 0.0]);
        assert!(
            (x - 1190.551).abs() < 1e-3 && (y - 841.8898).abs() < 1e-3,
            "{x} {y}"
        );
        assert_eq!(page.xy([0.0, 297_000.0]), (0.0, 0.0));
    }
}
