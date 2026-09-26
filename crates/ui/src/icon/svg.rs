//! Web uygulamasının 20×20'lik çizgi ikonları (apps/web/src/ui/icons.ts),
//! SVG metinlerinden çizilir: o setin kullandığı alt küme. `path` (M, L, H,
//! V, C, S, Q, T, A, Z; büyük ve küçük harf), köşesi yuvarlatılabilen `rect`,
//! `circle`, `rotate()` ile döndürülebilen `ellipse`; `fill="currentColor"`
//! ve `fill-opacity`, `stroke="none"`, `stroke-width`, `stroke-dasharray`,
//! `stroke-linecap`, `fill-rule`. Web çizgiyi yazı rengiyle, 1,4 birim
//! kalınlıkta, yuvarlak uç ve birleşimle çizer; burada da öyle. Çözülemeyen
//! öğe atlanır, hiçbir girdi panik yaptırmaz.

use std::f32::consts::{FRAC_PI_2, PI, TAU};

use iced::widget::canvas::fill::Rule;
use iced::widget::canvas::{Fill, Frame, LineCap, LineDash, LineJoin, Path, Stroke, Style};
use iced::{Color, Point};

/// The web set's grid and stroke (`viewBox="0 0 20 20"`, `stroke-width="1.4"`).
const GRID: f32 = 20.0;
const STROKE: f32 = 1.4;

/// One step of an outline, in grid units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Step {
    Move(Point),
    Line(Point),
    Cubic(Point, Point, Point),
    Close,
}

/// How an element is painted.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Paint {
    pub stroke: bool,
    /// Fill opacity; none: not filled.
    pub fill: Option<f32>,
    /// Stroke width in grid units, when the element gives its own.
    pub width: Option<f32>,
    pub dash: Vec<f32>,
    pub cap: LineCapKind,
    pub even_odd: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum LineCapKind {
    Round,
    Butt,
    Square,
}

/// An element of the icon: its outline and paint.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Shape {
    pub steps: Vec<Step>,
    pub paint: Paint,
}

/// Draws `markup` filling the frame. `weight` fixes the stroke in pixels
/// (the web's large ribbon icons: `vector-effect: non-scaling-stroke`).
pub(super) fn draw(frame: &mut Frame, markup: &str, color: Color, weight: Option<f32>) {
    let size = frame.size();
    let unit = size.width.min(size.height) / GRID;
    for shape in parse(markup) {
        let path = Path::new(|b| {
            for step in &shape.steps {
                match *step {
                    Step::Move(p) => b.move_to(scaled(p, unit)),
                    Step::Line(p) => b.line_to(scaled(p, unit)),
                    Step::Cubic(c1, c2, p) => {
                        b.bezier_curve_to(scaled(c1, unit), scaled(c2, unit), scaled(p, unit));
                    }
                    Step::Close => b.close(),
                }
            }
        });
        let paint = &shape.paint;
        if let Some(opacity) = paint.fill {
            frame.fill(
                &path,
                Fill {
                    style: Style::Solid(Color {
                        a: color.a * opacity,
                        ..color
                    }),
                    rule: if paint.even_odd {
                        Rule::EvenOdd
                    } else {
                        Rule::NonZero
                    },
                },
            );
        }
        if paint.stroke {
            // A fixed weight keeps its pixels, dashes too (non-scaling stroke).
            let (width, dash_unit) = match weight {
                Some(px) => (paint.width.unwrap_or(px), 1.0),
                None => ((paint.width.unwrap_or(STROKE) * unit).max(1.1), unit),
            };
            let segments: Vec<f32> = paint.dash.iter().map(|d| d * dash_unit).collect();
            frame.stroke(
                &path,
                Stroke {
                    style: Style::Solid(color),
                    width,
                    line_cap: match paint.cap {
                        LineCapKind::Round => LineCap::Round,
                        LineCapKind::Butt => LineCap::Butt,
                        LineCapKind::Square => LineCap::Square,
                    },
                    line_join: LineJoin::Round,
                    line_dash: LineDash {
                        segments: &segments,
                        offset: 0,
                    },
                },
            );
        }
    }
}

fn scaled(p: Point, unit: f32) -> Point {
    Point::new(p.x * unit, p.y * unit)
}

/// The elements of `markup`, in order; those it cannot read are left out.
pub(super) fn parse(markup: &str) -> Vec<Shape> {
    let mut shapes = Vec::new();
    let mut rest = markup;
    while let Some(open) = rest.find('<') {
        rest = &rest[open + 1..];
        let end = rest.find('>').unwrap_or(rest.len());
        let tag = &rest[..end];
        rest = &rest[end.min(rest.len())..];
        if tag.starts_with('/') {
            continue;
        }
        let name_end = tag
            .find(|c: char| c.is_whitespace() || c == '/')
            .unwrap_or(tag.len());
        let attrs = Attrs::read(&tag[name_end..]);
        let steps = match &tag[..name_end] {
            "path" => attrs.get("d").map(path_steps),
            "rect" => rect(&attrs),
            "circle" => attrs.num("r").map(|r| {
                ellipse(
                    attrs.num("cx").unwrap_or(0.0),
                    attrs.num("cy").unwrap_or(0.0),
                    r,
                    r,
                    0.0,
                )
            }),
            "ellipse" => match (attrs.num("rx"), attrs.num("ry")) {
                (Some(rx), Some(ry)) => Some(ellipse(
                    attrs.num("cx").unwrap_or(0.0),
                    attrs.num("cy").unwrap_or(0.0),
                    rx,
                    ry,
                    rotation(attrs.get("transform")),
                )),
                _ => None,
            },
            _ => None,
        };
        let Some(steps) = steps.filter(|s| !s.is_empty()) else {
            continue;
        };
        shapes.push(Shape {
            steps,
            paint: attrs.paint(),
        });
    }
    shapes
}

/// An element's attributes as written (`name="value"`).
struct Attrs<'a>(Vec<(&'a str, &'a str)>);

impl<'a> Attrs<'a> {
    fn read(mut text: &'a str) -> Self {
        let mut pairs = Vec::new();
        while let Some(eq) = text.find("=\"") {
            let name = text[..eq].trim();
            let value_start = eq + 2;
            let Some(len) = text[value_start..].find('"') else {
                break;
            };
            pairs.push((name, &text[value_start..value_start + len]));
            text = &text[value_start + len + 1..];
        }
        Self(pairs)
    }

    fn get(&self, name: &str) -> Option<&'a str> {
        self.0.iter().find(|(n, _)| *n == name).map(|(_, v)| *v)
    }

    fn num(&self, name: &str) -> Option<f32> {
        self.get(name)?
            .trim()
            .parse()
            .ok()
            .filter(|v: &f32| v.is_finite())
    }

    fn paint(&self) -> Paint {
        let filled = self.get("fill").is_some_and(|f| f != "none");
        Paint {
            stroke: self.get("stroke") != Some("none"),
            fill: filled.then(|| self.num("fill-opacity").unwrap_or(1.0).clamp(0.0, 1.0)),
            width: self.num("stroke-width").filter(|w| *w > 0.0),
            dash: self
                .get("stroke-dasharray")
                .map(|d| numbers(d).into_iter().filter(|v| *v >= 0.0).collect())
                .unwrap_or_default(),
            cap: match self.get("stroke-linecap") {
                Some("butt") => LineCapKind::Butt,
                Some("square") => LineCapKind::Square,
                _ => LineCapKind::Round,
            },
            even_odd: self.get("fill-rule") == Some("evenodd"),
        }
    }
}

/// The numbers of a list (`"2 1.8"`, `"-28 10 10"`).
fn numbers(text: &str) -> Vec<f32> {
    let mut t = Tokens::new(text);
    std::iter::from_fn(|| t.number()).collect()
}

/// `rotate(a cx cy)`'s angle in radians (the set rotates about the shape's centre).
fn rotation(transform: Option<&str>) -> f32 {
    transform
        .and_then(|t| t.trim().strip_prefix("rotate("))
        .and_then(|t| t.strip_suffix(')'))
        .and_then(|t| numbers(t).first().copied())
        .map_or(0.0, f32::to_radians)
}

fn rect(attrs: &Attrs<'_>) -> Option<Vec<Step>> {
    let (x, y) = (attrs.num("x").unwrap_or(0.0), attrs.num("y").unwrap_or(0.0));
    let (w, h) = (attrs.num("width")?, attrs.num("height")?);
    if w <= 0.0 || h <= 0.0 {
        return None;
    }
    let r = attrs
        .num("rx")
        .or_else(|| attrs.num("ry"))
        .unwrap_or(0.0)
        .clamp(0.0, w.min(h) / 2.0);
    if r == 0.0 {
        return Some(vec![
            Step::Move(Point::new(x, y)),
            Step::Line(Point::new(x + w, y)),
            Step::Line(Point::new(x + w, y + h)),
            Step::Line(Point::new(x, y + h)),
            Step::Close,
        ]);
    }
    let mut steps = vec![Step::Move(Point::new(x + r, y))];
    let corner = |steps: &mut Vec<Step>, cx: f32, cy: f32, from: f32| {
        arc_steps(steps, Point::new(cx, cy), r, r, 0.0, from, FRAC_PI_2);
    };
    steps.push(Step::Line(Point::new(x + w - r, y)));
    corner(&mut steps, x + w - r, y + r, -FRAC_PI_2);
    steps.push(Step::Line(Point::new(x + w, y + h - r)));
    corner(&mut steps, x + w - r, y + h - r, 0.0);
    steps.push(Step::Line(Point::new(x + r, y + h)));
    corner(&mut steps, x + r, y + h - r, FRAC_PI_2);
    steps.push(Step::Line(Point::new(x, y + r)));
    corner(&mut steps, x + r, y + r, PI);
    steps.push(Step::Close);
    Some(steps)
}

/// A whole ellipse of radii `rx`, `ry` turned by `rotation`.
fn ellipse(cx: f32, cy: f32, rx: f32, ry: f32, rotation: f32) -> Vec<Step> {
    if rx <= 0.0 || ry <= 0.0 {
        return Vec::new();
    }
    let centre = Point::new(cx, cy);
    let mut steps = vec![Step::Move(on_ellipse(centre, rx, ry, rotation, 0.0))];
    arc_steps(&mut steps, centre, rx, ry, rotation, 0.0, TAU);
    steps.push(Step::Close);
    steps
}

fn on_ellipse(c: Point, rx: f32, ry: f32, rotation: f32, angle: f32) -> Point {
    let (sin_r, cos_r) = rotation.sin_cos();
    let (x, y) = (rx * angle.cos(), ry * angle.sin());
    Point::new(c.x + x * cos_r - y * sin_r, c.y + x * sin_r + y * cos_r)
}

/// Cubic Béziers along an elliptical arc from `start` through `sweep`
/// radians, at most a quarter turn each (the curve is continued from the
/// arc's start, which the caller has reached).
fn arc_steps(
    steps: &mut Vec<Step>,
    c: Point,
    rx: f32,
    ry: f32,
    rotation: f32,
    start: f32,
    sweep: f32,
) {
    let parts = ((sweep.abs() / FRAC_PI_2).ceil() as usize).clamp(1, 16);
    let delta = sweep / parts as f32;
    let k = 4.0 / 3.0 * (delta / 4.0).tan();
    let (sin_r, cos_r) = rotation.sin_cos();
    let map = |x: f32, y: f32| Point::new(c.x + x * cos_r - y * sin_r, c.y + x * sin_r + y * cos_r);
    let mut a = start;
    for _ in 0..parts {
        let b = a + delta;
        let (sa, ca) = a.sin_cos();
        let (sb, cb) = b.sin_cos();
        let c1 = map(rx * (ca - k * sa), ry * (sa + k * ca));
        let c2 = map(rx * (cb + k * sb), ry * (sb - k * cb));
        let end = map(rx * cb, ry * sb);
        steps.push(Step::Cubic(c1, c2, end));
        a = b;
    }
}

/// An SVG path's data as steps in absolute units.
fn path_steps(d: &str) -> Vec<Step> {
    let mut t = Tokens::new(d);
    let mut steps = Vec::new();
    let mut cur = Point::ORIGIN;
    let mut start = Point::ORIGIN;
    // The last control point, for the smooth curves (S after C, T after Q).
    let mut last_cubic: Option<Point> = None;
    let mut last_quad: Option<Point> = None;
    let mut command = None;
    let mut started = false;
    loop {
        let cmd = match t.command() {
            Some(c) => c,
            None if t.at_number() => match command {
                // More pairs after a move are lines.
                Some(b'M') => b'L',
                Some(b'm') => b'l',
                Some(c) => c,
                None => break,
            },
            None => break,
        };
        command = Some(cmd);
        let relative = cmd.is_ascii_lowercase();
        let base = if relative { cur } else { Point::ORIGIN };
        let at = |x: f32, y: f32| Point::new(base.x + x, base.y + y);
        let (mut next_cubic, mut next_quad) = (None, None);
        match cmd.to_ascii_uppercase() {
            b'M' => {
                let Some((x, y)) = t.pair() else { break };
                cur = at(x, y);
                start = cur;
                steps.push(Step::Move(cur));
                started = true;
            }
            b'Z' => {
                if started {
                    steps.push(Step::Close);
                }
                cur = start;
            }
            _ if !started => break,
            b'L' => {
                let Some((x, y)) = t.pair() else { break };
                cur = at(x, y);
                steps.push(Step::Line(cur));
            }
            b'H' => {
                let Some(x) = t.number() else { break };
                cur = Point::new(if relative { cur.x + x } else { x }, cur.y);
                steps.push(Step::Line(cur));
            }
            b'V' => {
                let Some(y) = t.number() else { break };
                cur = Point::new(cur.x, if relative { cur.y + y } else { y });
                steps.push(Step::Line(cur));
            }
            b'C' => {
                let (Some((x1, y1)), Some((x2, y2)), Some((x, y))) = (t.pair(), t.pair(), t.pair())
                else {
                    break;
                };
                let (c1, c2, end) = (at(x1, y1), at(x2, y2), at(x, y));
                steps.push(Step::Cubic(c1, c2, end));
                next_cubic = Some(c2);
                cur = end;
            }
            b'S' => {
                let (Some((x2, y2)), Some((x, y))) = (t.pair(), t.pair()) else {
                    break;
                };
                let c1 =
                    last_cubic.map_or(cur, |c| Point::new(2.0 * cur.x - c.x, 2.0 * cur.y - c.y));
                let (c2, end) = (at(x2, y2), at(x, y));
                steps.push(Step::Cubic(c1, c2, end));
                next_cubic = Some(c2);
                cur = end;
            }
            b'Q' | b'T' => {
                let control = if cmd.eq_ignore_ascii_case(&b'Q') {
                    let Some((x1, y1)) = t.pair() else { break };
                    at(x1, y1)
                } else {
                    last_quad.map_or(cur, |c| Point::new(2.0 * cur.x - c.x, 2.0 * cur.y - c.y))
                };
                let Some((x, y)) = t.pair() else { break };
                let end = at(x, y);
                // A quadratic as the cubic it is.
                let c1 = Point::new(
                    cur.x + 2.0 / 3.0 * (control.x - cur.x),
                    cur.y + 2.0 / 3.0 * (control.y - cur.y),
                );
                let c2 = Point::new(
                    end.x + 2.0 / 3.0 * (control.x - end.x),
                    end.y + 2.0 / 3.0 * (control.y - end.y),
                );
                steps.push(Step::Cubic(c1, c2, end));
                next_quad = Some(control);
                cur = end;
            }
            b'A' => {
                let (Some(rx), Some(ry), Some(angle), Some(large), Some(sweep), Some((x, y))) = (
                    t.number(),
                    t.number(),
                    t.number(),
                    t.flag(),
                    t.flag(),
                    t.pair(),
                ) else {
                    break;
                };
                let end = at(x, y);
                svg_arc(
                    &mut steps,
                    cur,
                    end,
                    rx,
                    ry,
                    angle.to_radians(),
                    large,
                    sweep,
                );
                cur = end;
            }
            _ => break,
        }
        last_cubic = next_cubic;
        last_quad = next_quad;
    }
    steps
}

/// An SVG arc from `from` to `to` (endpoint form) as Béziers, by the
/// centre form of SVG 1.1 appendix F.6.5; radii too small are scaled up
/// (F.6.6), a zero radius is a line.
#[allow(clippy::too_many_arguments)]
fn svg_arc(
    steps: &mut Vec<Step>,
    from: Point,
    to: Point,
    rx: f32,
    ry: f32,
    phi: f32,
    large: bool,
    sweep: bool,
) {
    if from == to {
        return;
    }
    let (mut rx, mut ry) = (rx.abs(), ry.abs());
    if rx == 0.0 || ry == 0.0 {
        steps.push(Step::Line(to));
        return;
    }
    let (sin_p, cos_p) = phi.sin_cos();
    let dx = (from.x - to.x) / 2.0;
    let dy = (from.y - to.y) / 2.0;
    let x1 = cos_p * dx + sin_p * dy;
    let y1 = -sin_p * dx + cos_p * dy;
    let lambda = (x1 * x1) / (rx * rx) + (y1 * y1) / (ry * ry);
    if lambda > 1.0 {
        let s = lambda.sqrt();
        rx *= s;
        ry *= s;
    }
    let num = rx * rx * ry * ry - rx * rx * y1 * y1 - ry * ry * x1 * x1;
    let den = rx * rx * y1 * y1 + ry * ry * x1 * x1;
    let mut coef = if den == 0.0 {
        0.0
    } else {
        (num / den).max(0.0).sqrt()
    };
    if large == sweep {
        coef = -coef;
    }
    let cx1 = coef * rx * y1 / ry;
    let cy1 = -coef * ry * x1 / rx;
    let centre = Point::new(
        cos_p * cx1 - sin_p * cy1 + (from.x + to.x) / 2.0,
        sin_p * cx1 + cos_p * cy1 + (from.y + to.y) / 2.0,
    );
    let angle = |ux: f32, uy: f32, vx: f32, vy: f32| {
        let a = (ux * vy - uy * vx).atan2(ux * vx + uy * vy);
        if a.is_finite() { a } else { 0.0 }
    };
    let ux = (x1 - cx1) / rx;
    let uy = (y1 - cy1) / ry;
    let vx = (-x1 - cx1) / rx;
    let vy = (-y1 - cy1) / ry;
    let theta = angle(1.0, 0.0, ux, uy);
    let mut delta = angle(ux, uy, vx, vy) % TAU;
    if !sweep && delta > 0.0 {
        delta -= TAU;
    } else if sweep && delta < 0.0 {
        delta += TAU;
    }
    arc_steps(steps, centre, rx, ry, phi, theta, delta);
    // Land exactly on the end point.
    if let Some(Step::Cubic(_, _, end)) = steps.last_mut() {
        *end = to;
    }
}

/// Path data read token by token: commands, numbers as SVG writes them
/// (`1.2.5` is two numbers, `-1-2` too) and single-digit arc flags.
struct Tokens<'a> {
    s: &'a [u8],
    i: usize,
}

impl<'a> Tokens<'a> {
    fn new(s: &'a str) -> Self {
        Self {
            s: s.as_bytes(),
            i: 0,
        }
    }

    fn skip(&mut self) {
        while self
            .s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_whitespace() || *c == b',')
        {
            self.i += 1;
        }
    }

    fn at_number(&mut self) -> bool {
        self.skip();
        self.s
            .get(self.i)
            .is_some_and(|c| c.is_ascii_digit() || matches!(c, b'-' | b'+' | b'.'))
    }

    fn command(&mut self) -> Option<u8> {
        self.skip();
        let c = *self.s.get(self.i)?;
        (c.is_ascii_alphabetic() && c != b'e' && c != b'E').then(|| {
            self.i += 1;
            c
        })
    }

    fn number(&mut self) -> Option<f32> {
        if !self.at_number() {
            return None;
        }
        let begin = self.i;
        if matches!(self.s.get(self.i), Some(b'-' | b'+')) {
            self.i += 1;
        }
        let mut dot = false;
        let mut digits = false;
        while let Some(&c) = self.s.get(self.i) {
            if c.is_ascii_digit() {
                digits = true;
            } else if c == b'.' && !dot {
                dot = true;
            } else {
                break;
            }
            self.i += 1;
        }
        if digits && matches!(self.s.get(self.i), Some(b'e' | b'E')) {
            let mark = self.i;
            self.i += 1;
            if matches!(self.s.get(self.i), Some(b'-' | b'+')) {
                self.i += 1;
            }
            if self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                while self.s.get(self.i).is_some_and(u8::is_ascii_digit) {
                    self.i += 1;
                }
            } else {
                self.i = mark;
            }
        }
        if !digits {
            self.i = begin + 1;
            return None;
        }
        std::str::from_utf8(&self.s[begin..self.i])
            .ok()?
            .parse::<f32>()
            .ok()
            .filter(|v| v.is_finite())
    }

    fn pair(&mut self) -> Option<(f32, f32)> {
        Some((self.number()?, self.number()?))
    }

    fn flag(&mut self) -> Option<bool> {
        self.skip();
        let c = *self.s.get(self.i)?;
        matches!(c, b'0' | b'1').then(|| {
            self.i += 1;
            c == b'1'
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Point, b: Point) -> bool {
        (a.x - b.x).abs() < 1e-4 && (a.y - b.y).abs() < 1e-4
    }

    #[test]
    fn numbers_are_read_as_svg_writes_them() {
        let mut t = Tokens::new("1.2.5-3,4e1 .5-.5");
        let read: Vec<f32> = std::iter::from_fn(|| t.number()).collect();
        assert_eq!(read, [1.2, 0.5, -3.0, 40.0, 0.5, -0.5]);
        // Arc flags need no separator: “a1 1 0 011 1”.
        let steps = path_steps("M0 0a1 1 0 011 1");
        assert!(
            matches!(steps.last(), Some(Step::Cubic(_, _, p)) if close(*p, Point::new(1.0, 1.0)))
        );
    }

    #[test]
    fn relative_and_implicit_commands_follow_the_pen() {
        let steps = path_steps("m2 3 4 0h2v-1l1 1zM10 10H12");
        assert_eq!(
            steps,
            [
                Step::Move(Point::new(2.0, 3.0)),
                Step::Line(Point::new(6.0, 3.0)),
                Step::Line(Point::new(8.0, 3.0)),
                Step::Line(Point::new(8.0, 2.0)),
                Step::Line(Point::new(9.0, 3.0)),
                Step::Close,
                Step::Move(Point::new(10.0, 10.0)),
                Step::Line(Point::new(12.0, 10.0)),
            ]
        );
    }

    #[test]
    fn an_arc_ends_where_it_is_told_and_bows_the_right_way() {
        let end = |steps: &[Step]| match steps.last() {
            Some(Step::Cubic(_, _, p)) => Some(*p),
            _ => None,
        };
        // The web's “pan” finger tip: a half circle of radius 1.2 up and over.
        let steps = path_steps("M7.2 4.7a1.2 1.2 0 0 1 2.4 0");
        assert!(
            end(&steps).is_some_and(|p| close(p, Point::new(9.6, 4.7))),
            "{steps:?}"
        );
        // Sweep 1 from left to right, y down: over the top (smaller y).
        let Some(Step::Cubic(c1, _, _)) = steps.get(1).copied() else {
            panic!("an arc: {steps:?}")
        };
        assert!(c1.y < 4.7, "{c1:?}");
        // Radii too small are scaled up: it still reaches the end.
        let steps = path_steps("M0 0A0.1 0.1 0 0 1 4 0");
        assert!(
            end(&steps).is_some_and(|p| close(p, Point::new(4.0, 0.0))),
            "{steps:?}"
        );
    }

    #[test]
    fn the_webs_elements_and_paint_are_read() {
        let shapes = parse(concat!(
            r#"<rect x="3.5" y="3.5" width="5" height="5"/>"#,
            r#"<rect x="3" y="8.5" width="4" height="3" rx=".5" fill="currentColor" fill-opacity=".3"/>"#,
            r#"<rect x="14" y="8.5" width="3" height="3" fill="currentColor" stroke="none"/>"#,
            r#"<circle cx="10" cy="10" r="6.5"/>"#,
            r#"<ellipse cx="10" cy="10" rx="7.6" ry="4.3" transform="rotate(-28 10 10)"/>"#,
            r#"<path d="M10 2.5v15" stroke-dasharray="2 1.8"/>"#,
            r#"<unknown/><path d=""/>"#,
        ));
        assert_eq!(shapes.len(), 6);
        assert!(shapes[0].paint.stroke && shapes[0].paint.fill.is_none());
        assert_eq!(shapes[1].paint.fill, Some(0.3));
        assert!(
            shapes[1].steps.iter().any(|s| matches!(s, Step::Cubic(..))),
            "rounded corners"
        );
        assert!(
            !shapes[2].paint.stroke && shapes[2].paint.fill == Some(1.0),
            "a grip"
        );
        assert_eq!(shapes[5].paint.dash, [2.0, 1.8]);
        // The turned ellipse starts on its major axis, turned 28° up.
        let Step::Move(p) = shapes[4].steps[0] else {
            panic!("a start")
        };
        assert!(close(
            p,
            Point::new(
                10.0 + 7.6 * (-28f32).to_radians().cos(),
                10.0 + 7.6 * (-28f32).to_radians().sin()
            )
        ));
    }

    #[test]
    fn broken_markup_draws_what_it_can_and_never_panics() {
        for text in [
            "",
            "<",
            "<path",
            r#"<path d="M"/>"#,
            r#"<path d="L1 2"/>"#,
            r#"<path d="M1 2 C"/>"#,
            r#"<path d="M1e 2"/>"#,
            r#"<rect width="-1" height="2"/>"#,
            r#"<circle r="NaN"/>"#,
            r#"<path d="M0 0A0 0 0 0 1 1 1"/>"#,
            r#"<path d="M0 0a1 1 0 2 1 1 1"/>"#,
            "<path d=\"M0 0\u{1F600}\"/>",
        ] {
            let _ = parse(text);
        }
        let shapes = parse(r#"<path d="M0 0L1 1 X 5 5"/>"#);
        assert_eq!(shapes[0].steps.len(), 2, "stops at the unknown command");
    }
}
