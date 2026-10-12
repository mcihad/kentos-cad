//! A chart laid out (docs/adr/0227 §5): its drawing list in CSS pixels,
//! the same on both platforms. A primitive carries a role (a theme colour
//! the platform resolves) and, when a click selects through it, its element.
//! Sizes scale with the text (u = the chart's text size / 12): 12u margins,
//! bars at most 24u wide with 4u rounded data ends, 2u lines, 4u markers
//! with a 2u ring of the surface, hairline grids.

use std::f32::consts::PI;

use super::fonts::{fit, width};
use super::model::{Body, BoxItem, Color, Model};
use super::ticks::{Axis, nice};
use crate::ops::statistics::Dec;

/// The room a chart is laid out in, and its text.
#[derive(Clone, Debug, PartialEq)]
pub struct Frame {
    pub width: f32,
    pub height: f32,
    /// The chart's text size in CSS pixels (the interface's `--fs-xs`).
    pub font: f32,
    /// The interface typeface's family (`fonts::width`).
    pub family: String,
}

/// A colour by its role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Surface,
    Grid,
    Axis,
    Ink1,
    Ink2,
    Ink3,
    Series(u8),
    Other,
    Selection,
}

impl Role {
    pub fn word(self) -> String {
        match self {
            Role::Surface => "surface".into(),
            Role::Grid => "grid".into(),
            Role::Axis => "axis".into(),
            Role::Ink1 => "ink1".into(),
            Role::Ink2 => "ink2".into(),
            Role::Ink3 => "ink3".into(),
            Role::Series(k) => format!("series{k}"),
            Role::Other => "other".into(),
            Role::Selection => "selection".into(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Start,
    Middle,
    End,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Baseline {
    Top,
    Middle,
    Bottom,
}

/// A drawing primitive.
#[derive(Clone, Debug, PartialEq)]
pub enum Prim {
    /// A rectangle with rounded top or bottom corners (a bar's data end).
    Rect {
        x: f32,
        y: f32,
        w: f32,
        h: f32,
        r_top: f32,
        r_bottom: f32,
        fill: Role,
        alpha: f32,
        elem: i32,
    },
    Line {
        x1: f32,
        y1: f32,
        x2: f32,
        y2: f32,
        stroke: Role,
        width: f32,
        alpha: f32,
    },
    /// A polyline (`closed`: a ring).
    Poly {
        pts: Vec<f32>,
        closed: bool,
        stroke: Role,
        width: f32,
        alpha: f32,
    },
    Circle {
        x: f32,
        y: f32,
        r: f32,
        fill: Option<Role>,
        stroke: Option<Role>,
        width: f32,
        alpha: f32,
        elem: i32,
    },
    /// A ring's slice: radii and angles clockwise from twelve o'clock (radians).
    Wedge {
        cx: f32,
        cy: f32,
        r0: f32,
        r1: f32,
        a0: f32,
        a1: f32,
        fill: Role,
        alpha: f32,
        elem: i32,
    },
    /// A mean's ×.
    Cross {
        x: f32,
        y: f32,
        size: f32,
        stroke: Role,
        width: f32,
    },
    Text {
        x: f32,
        y: f32,
        text: String,
        size: f32,
        weight: u16,
        align: Align,
        baseline: Baseline,
        color: Role,
    },
    /// Many points from [`Layout::points`]: `count` pairs from pair `start`.
    Points {
        start: u32,
        count: u32,
        r: f32,
        ring: bool,
        fill: Role,
        alpha: f32,
    },
}

/// Where a click selects an element.
#[derive(Clone, Debug, PartialEq)]
pub enum Target {
    Band {
        x0: f32,
        x1: f32,
        y0: f32,
        y1: f32,
        elem: u32,
    },
    Wedge {
        cx: f32,
        cy: f32,
        r0: f32,
        r1: f32,
        a0: f32,
        a1: f32,
        elem: u32,
    },
    Point {
        x: f32,
        y: f32,
        elem: u32,
    },
}

/// A chart's drawing.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub width: f32,
    pub height: f32,
    pub prims: Vec<Prim>,
    /// The bulk points (x, y pairs) and each one's element.
    pub points: Vec<f32>,
    pub point_elems: Vec<u32>,
    pub targets: Vec<Target>,
    /// The plot area: x, y, width, height.
    pub plot: [f32; 4],
    /// The hit distance (12u).
    pub reach: f32,
}

/// An axis mapped to pixels.
#[derive(Clone, Copy, Debug)]
struct Scale {
    d0: f64,
    d1: f64,
    p0: f32,
    p1: f32,
}

impl Scale {
    fn px(&self, v: f64) -> f32 {
        if self.d1 == self.d0 {
            return (self.p0 + self.p1) / 2.0;
        }
        self.p0 + ((v - self.d0) / (self.d1 - self.d0)) as f32 * (self.p1 - self.p0)
    }
}

/// The selection's look: marked elements in the selection's colour, the
/// others faded when any is marked.
#[derive(Clone, Copy)]
struct Looks<'a> {
    marks: &'a [bool],
    any: bool,
}

impl Looks<'_> {
    fn of(&self, elem: usize, role: Role) -> (Role, f32) {
        if !self.any {
            (role, 1.0)
        } else if self.marks.get(elem).copied().unwrap_or(false) {
            (Role::Selection, 1.0)
        } else {
            (role, 0.35)
        }
    }

    fn marked(&self, elem: usize) -> bool {
        self.any && self.marks.get(elem).copied().unwrap_or(false)
    }
}

struct Ctx<'a> {
    out: Layout,
    font: f32,
    family: &'a str,
    u: f32,
}

impl Ctx<'_> {
    fn text(&mut self, x: f32, y: f32, text: &str, align: Align, baseline: Baseline, color: Role) {
        self.text_at(x, y, text, align, baseline, color, self.font, 400);
    }

    #[allow(clippy::too_many_arguments)]
    fn text_at(
        &mut self,
        x: f32,
        y: f32,
        text: &str,
        align: Align,
        baseline: Baseline,
        color: Role,
        size: f32,
        weight: u16,
    ) {
        if text.is_empty() {
            return;
        }
        self.out.prims.push(Prim::Text {
            x,
            y,
            text: text.to_owned(),
            size,
            weight,
            align,
            baseline,
            color,
        });
    }

    fn line(&mut self, x1: f32, y1: f32, x2: f32, y2: f32, stroke: Role, width: f32) {
        self.out.prims.push(Prim::Line {
            x1,
            y1,
            x2,
            y2,
            stroke,
            width,
            alpha: 1.0,
        });
    }

    fn w(&self, text: &str) -> f32 {
        width(text, self.font, self.family)
    }
}

fn color_role(c: Color) -> Role {
    match c {
        Color::Series(k) => Role::Series(k),
        Color::Other => Role::Other,
    }
}

fn f(v: Dec) -> f64 {
    v.to_f64()
}

/// The axis over a range: its ticks within `room` pixels, `per` pixels a tick at most.
fn axis_over(lo: Dec, hi: Dec, room: f32, per: f32, integer: bool, zero: bool) -> Option<Axis> {
    let max = ((room / per).floor() as u32).max(2);
    nice(lo, hi, max, integer, zero)
}

/// A chart's drawing in a frame, its elements marked by `marks` (selected rows).
pub fn lay_out(model: &Model, marks: &[bool], frame: &Frame) -> Layout {
    let font = frame.font.max(6.0);
    let u = font / 12.0;
    let mut cx = Ctx {
        out: Layout {
            width: frame.width,
            height: frame.height,
            reach: 12.0 * u,
            ..Layout::default()
        },
        font,
        family: &frame.family,
        u,
    };
    let looks = Looks {
        marks,
        any: marks.iter().any(|&m| m),
    };
    let pad = 12.0 * u;
    let line_h = 1.4 * font;
    if frame.width < 120.0 * u || frame.height < 80.0 * u {
        cx.text(
            frame.width / 2.0,
            frame.height / 2.0,
            "Grafik için yer yok",
            Align::Middle,
            Baseline::Middle,
            Role::Ink3,
        );
        return cx.out;
    }
    match &model.body {
        Body::Pie { .. } => pie(&mut cx, model, looks, frame, pad, line_h),
        _ => cartesian(&mut cx, model, looks, frame, pad, line_h),
    }
    cx.out
}

/// The legend of a line chart's series: items (key, name) right-aligned on a
/// line, or what fits with “+k”.
fn series_legend(cx: &mut Ctx, names: &[(String, Role)], right: f32, y: f32, room: f32) {
    let u = cx.u;
    let item_w = |cx: &Ctx, n: &str| 16.0 * u + 4.0 * u + cx.w(n);
    let gap = 12.0 * u;
    let mut shown = names.len();
    let total = |cx: &Ctx, k: usize, more: bool| -> f32 {
        let mut t: f32 = names[..k].iter().map(|(n, _)| item_w(cx, n)).sum::<f32>()
            + gap * (k.saturating_sub(1)) as f32;
        if more {
            t += gap + cx.w(&format!("+{}", names.len() - k));
        }
        t
    };
    while shown > 0 && total(cx, shown, shown < names.len()) > room {
        shown -= 1;
    }
    let mut x = right - total(cx, shown, shown < names.len());
    for (name, role) in &names[..shown] {
        cx.out.prims.push(Prim::Line {
            x1: x,
            y1: y,
            x2: x + 16.0 * u,
            y2: y,
            stroke: *role,
            width: 2.0 * u,
            alpha: 1.0,
        });
        x += 20.0 * u;
        cx.text(x, y, name, Align::Start, Baseline::Middle, Role::Ink2);
        x += cx.w(name) + gap;
    }
    if shown < names.len() {
        let more = format!("+{}", names.len() - shown);
        cx.text(x, y, &more, Align::Start, Baseline::Middle, Role::Ink3);
    }
}

#[allow(clippy::too_many_lines)]
fn cartesian(cx: &mut Ctx, model: &Model, looks: Looks, frame: &Frame, pad: f32, line_h: f32) {
    let u = cx.u;
    let font = cx.font;
    // The y axis's name on the first line; a line chart's legend beside it or under it.
    cx.text(
        pad,
        pad,
        &model.y_title.clone(),
        Align::Start,
        Baseline::Top,
        Role::Ink2,
    );
    let mut top = pad + line_h + 10.0 * u;
    if let Body::Lines { series, .. } = &model.body
        && series.len() > 1
    {
        let names: Vec<(String, Role)> = series
            .iter()
            .map(|s| (s.label.clone(), color_role(s.color)))
            .collect();
        let used = cx.w(&model.y_title) + 24.0 * u;
        let room = frame.width - 2.0 * pad - used;
        let need: f32 = names
            .iter()
            .map(|(n, _)| 20.0 * u + cx.w(n) + 12.0 * u)
            .sum();
        if need <= room {
            series_legend(cx, &names, frame.width - pad, pad + line_h / 2.0, room);
        } else {
            series_legend(
                cx,
                &names,
                frame.width - pad,
                top + line_h / 2.0,
                frame.width - 2.0 * pad,
            );
            top += line_h + 4.0 * u;
        }
    }
    let bottom = frame.height - pad - 2.0 * line_h - 10.0 * u;
    cx.text(
        frame.width - pad,
        frame.height - pad,
        &model.x_title.clone(),
        Align::End,
        Baseline::Bottom,
        Role::Ink2,
    );
    if bottom - top < 30.0 * u {
        return;
    }
    // The value axis.
    let (lo, hi, integer, zero) = match &model.body {
        Body::Bars { bars, integer } => {
            let vals = bars.iter().filter_map(|b| b.value);
            let (lo, hi) =
                range_of(vals).unwrap_or((Dec { m: 0, scale: 0 }, Dec { m: 1, scale: 0 }));
            (lo, hi, *integer, true)
        }
        Body::Histogram { bins, .. } => {
            let top = bins.iter().map(|b| b.count).max().unwrap_or(1).max(1);
            (
                Dec { m: 0, scale: 0 },
                Dec {
                    m: top as i128,
                    scale: 0,
                },
                true,
                true,
            )
        }
        Body::Boxes { boxes } => {
            let vals = boxes
                .iter()
                .filter(|b| b.n > 0)
                .flat_map(|b| [b.min, b.max]);
            let (lo, hi) =
                range_of(vals).unwrap_or((Dec { m: 0, scale: 0 }, Dec { m: 1, scale: 0 }));
            (lo, hi, false, false)
        }
        Body::Scatter {
            y_range, y_whole, ..
        } => {
            let (lo, hi) = y_range.unwrap_or((Dec { m: 0, scale: 0 }, Dec { m: 1, scale: 0 }));
            (lo, hi, *y_whole, false)
        }
        Body::Lines {
            y_range, series, ..
        } => {
            let (lo, hi) = y_range.unwrap_or((Dec { m: 0, scale: 0 }, Dec { m: 1, scale: 0 }));
            let whole = series
                .iter()
                .all(|s| s.points.iter().all(|p| p.y.scale == 0));
            (lo, hi, whole, false)
        }
        Body::Pie { .. } => return,
    };
    let Some(y_axis) = axis_over(lo, hi, bottom - top, 2.5 * font, integer, zero) else {
        return;
    };
    let label_w = y_axis
        .labels
        .iter()
        .map(|l| cx.w(l))
        .fold(0.0f32, |a, b| if b > a { b } else { a });
    let left = pad + label_w + 6.0 * u;
    let right = frame.width - pad;
    if right - left < 40.0 * u {
        return;
    }
    let ys = Scale {
        d0: f(y_axis.lo),
        d1: f(y_axis.hi),
        p0: bottom,
        p1: top,
    };
    cx.out.plot = [left, top, right - left, bottom - top];
    // The x axis of the numeric charts.
    // The x axis of the numeric charts; whole numbers step by wholes.
    let x_axis = match &model.body {
        Body::Histogram { min, max, .. } => {
            let whole = min.scale == 0 && max.scale == 0;
            axis_over(*min, *max, right - left, 6.5 * font, whole, false)
        }
        Body::Scatter {
            x_range, x_whole, ..
        } => {
            let (lo, hi) = x_range.unwrap_or((Dec { m: 0, scale: 0 }, Dec { m: 1, scale: 0 }));
            axis_over(lo, hi, right - left, 6.5 * font, *x_whole, false)
        }
        Body::Lines {
            x_range, series, ..
        } => {
            let (lo, hi) = x_range.unwrap_or((Dec { m: 0, scale: 0 }, Dec { m: 1, scale: 0 }));
            let whole = series
                .iter()
                .all(|s| s.points.iter().all(|p| p.x.scale == 0));
            axis_over(lo, hi, right - left, 6.5 * font, whole, false)
        }
        _ => None,
    };
    let xs = x_axis.as_ref().map(|a| Scale {
        d0: f(a.lo),
        d1: f(a.hi),
        p0: left,
        p1: right,
    });
    // The grid: horizontal hairlines at the ticks, vertical ones in a scatter.
    for (t, label) in y_axis.ticks.iter().zip(&y_axis.labels) {
        let y = ys.px(f(*t));
        let role = if t.m == 0 && zero {
            Role::Axis
        } else {
            Role::Grid
        };
        cx.line(left, y, right, y, role, 1.0);
        cx.text(
            left - 6.0 * u,
            y,
            label,
            Align::End,
            Baseline::Middle,
            Role::Ink3,
        );
    }
    if let (Some(a), Some(xs)) = (&x_axis, xs) {
        let scatter = matches!(model.body, Body::Scatter { .. });
        let mut last_end = f32::NEG_INFINITY;
        for (t, label) in a.ticks.iter().zip(&a.labels) {
            let x = xs.px(f(*t));
            if scatter {
                cx.line(x, top, x, bottom, Role::Grid, 1.0);
            }
            let w = cx.w(label);
            // Kept inside the frame; a label that would touch the one before it is left out.
            let start = (x - w / 2.0).clamp(2.0 * u, (frame.width - 2.0 * u - w).max(2.0 * u));
            if start > last_end + 4.0 * u {
                cx.text(
                    start,
                    bottom + 6.0 * u,
                    label,
                    Align::Start,
                    Baseline::Top,
                    Role::Ink3,
                );
                last_end = start + w;
            }
        }
        if !zero {
            cx.line(left, bottom, right, bottom, Role::Axis, 1.0);
        }
    } else if !zero {
        cx.line(left, bottom, right, bottom, Role::Axis, 1.0);
    }
    match &model.body {
        Body::Bars { bars, .. } => {
            let n = bars.len().max(1);
            let band = (right - left) / n as f32;
            let bar_w = (24.0 * u).min(0.8 * band).max(1.0);
            let y0 = ys.px(0.0);
            let labels: Vec<String> = bars.iter().map(|b| b.label.clone()).collect();
            category_labels(cx, &labels, left, band, bottom + 6.0 * u);
            for (i, b) in bars.iter().enumerate() {
                let x0 = left + band * i as f32;
                cx.out.targets.push(Target::Band {
                    x0,
                    x1: x0 + band,
                    y0: top,
                    y1: bottom,
                    elem: i as u32,
                });
                let Some(v) = b.value else { continue };
                let base = if b.other {
                    Role::Other
                } else {
                    Role::Series(0)
                };
                let (role, alpha) = looks.of(i, base);
                let y = ys.px(f(v));
                let (ry, rh) = if y <= y0 { (y, y0 - y) } else { (y0, y - y0) };
                let r = (4.0 * u).min(bar_w / 2.0).min(rh);
                cx.out.prims.push(Prim::Rect {
                    x: x0 + (band - bar_w) / 2.0,
                    y: ry,
                    w: bar_w,
                    h: rh,
                    r_top: if y <= y0 { r } else { 0.0 },
                    r_bottom: if y <= y0 { 0.0 } else { r },
                    fill: role,
                    alpha,
                    elem: i as i32,
                });
            }
        }
        Body::Histogram {
            bins,
            mean,
            median,
            guides,
            ..
        } => {
            let Some(xs) = xs else { return };
            let y0 = ys.px(0.0);
            for (i, b) in bins.iter().enumerate() {
                let (mut x0, mut x1) = (xs.px(b.lo_x), xs.px(b.hi_x));
                if x1 - x0 < 1.0 {
                    let c = (x0 + x1) / 2.0;
                    x0 = c - 12.0 * u;
                    x1 = c + 12.0 * u;
                }
                cx.out.targets.push(Target::Band {
                    x0,
                    x1,
                    y0: top,
                    y1: bottom,
                    elem: i as u32,
                });
                if b.count == 0 {
                    continue;
                }
                let gap = if x1 - x0 >= 3.0 * u { u } else { 0.0 };
                let y = ys.px(b.count as f64);
                let w = x1 - x0 - 2.0 * gap;
                let (role, alpha) = looks.of(i, Role::Series(0));
                cx.out.prims.push(Prim::Rect {
                    x: x0 + gap,
                    y,
                    w,
                    h: y0 - y,
                    r_top: (4.0 * u).min(w / 2.0).min(y0 - y),
                    r_bottom: 0.0,
                    fill: role,
                    alpha,
                    elem: i as i32,
                });
            }
            if *guides {
                let mut last: Option<(f32, f32)> = None;
                for (v, name) in [(*mean, "Ortalama"), (*median, "Ortanca")] {
                    let Some(v) = v else { continue };
                    let x = xs.px(f(v));
                    cx.out.prims.push(Prim::Line {
                        x1: x,
                        y1: top,
                        x2: x,
                        y2: bottom,
                        stroke: Role::Ink2,
                        width: 1.5 * u,
                        alpha: 1.0,
                    });
                    let w = cx.w(name);
                    // Beside the line, to its left when it would leave the plot; under the first when they meet.
                    let (tx, align) = if x + 4.0 * u + w > right {
                        (x - 4.0 * u, Align::End)
                    } else {
                        (x + 4.0 * u, Align::Start)
                    };
                    let mut ty = top + 2.0 * u;
                    if let Some((lx, ly)) = last
                        && (lx - x).abs() < w + 8.0 * u
                    {
                        ty = ly + line_h;
                    }
                    // On a ground of the surface, so it reads over the bars.
                    let bx = if align == Align::Start {
                        tx - 2.0 * u
                    } else {
                        tx - w - 2.0 * u
                    };
                    cx.out.prims.push(Prim::Rect {
                        x: bx,
                        y: ty - u,
                        w: w + 4.0 * u,
                        h: font + 2.0 * u,
                        r_top: 2.0 * u,
                        r_bottom: 2.0 * u,
                        fill: Role::Surface,
                        alpha: 0.85,
                        elem: -1,
                    });
                    cx.text(tx, ty, name, align, Baseline::Top, Role::Ink2);
                    last = Some((x, ty));
                }
            }
        }
        Body::Boxes { boxes } => {
            let n = boxes.len().max(1);
            let band = (right - left) / n as f32;
            if !model.x_title.is_empty() || boxes.len() > 1 {
                let labels: Vec<String> = boxes.iter().map(|b| b.label.clone()).collect();
                category_labels(cx, &labels, left, band, bottom + 6.0 * u);
            }
            for (i, b) in boxes.iter().enumerate() {
                let x0 = left + band * i as f32;
                cx.out.targets.push(Target::Band {
                    x0,
                    x1: x0 + band,
                    y0: top,
                    y1: bottom,
                    elem: i as u32,
                });
                draw_box(
                    cx,
                    b,
                    i,
                    x0 + band / 2.0,
                    (32.0 * u).min(0.6 * band),
                    &ys,
                    looks,
                );
            }
        }
        Body::Scatter { x, y, trend, .. } => {
            let Some(xs) = xs else { return };
            let n = x.len();
            let (r, ring) = if n <= 2000 {
                (4.0 * u, true)
            } else {
                (2.0 * u, false)
            };
            let mut plain: Vec<f32> = Vec::with_capacity(n * 2);
            let mut plain_elems = Vec::with_capacity(n);
            let mut chosen: Vec<f32> = Vec::new();
            let mut chosen_elems = Vec::new();
            for i in 0..n {
                let (px, py) = (xs.px(x[i]), ys.px(y[i]));
                if looks.marked(i) {
                    chosen.extend([px, py]);
                    chosen_elems.push(i as u32);
                } else {
                    plain.extend([px, py]);
                    plain_elems.push(i as u32);
                }
            }
            let count = plain_elems.len() as u32;
            cx.out.points.extend(plain);
            cx.out.point_elems.extend(plain_elems);
            cx.out.prims.push(Prim::Points {
                start: 0,
                count,
                r,
                ring,
                fill: Role::Series(0),
                alpha: if looks.any {
                    0.35
                } else if ring {
                    1.0
                } else {
                    0.8
                },
            });
            if !chosen_elems.is_empty() {
                let start = count;
                let k = chosen_elems.len() as u32;
                cx.out.points.extend(chosen);
                cx.out.point_elems.extend(chosen_elems);
                cx.out.prims.push(Prim::Points {
                    start,
                    count: k,
                    r: r.max(3.0 * u),
                    ring: true,
                    fill: Role::Selection,
                    alpha: 1.0,
                });
            }
            if let Some(t) = trend {
                if let Some((p, q)) = clip_line(t.a, t.b, &xs, &ys, [left, top, right, bottom]) {
                    cx.out.prims.push(Prim::Line {
                        x1: p.0,
                        y1: p.1,
                        x2: q.0,
                        y2: q.1,
                        stroke: Role::Ink2,
                        width: 1.5 * u,
                        alpha: 1.0,
                    });
                }
                cx.text(
                    right - 4.0 * u,
                    top + 4.0 * u,
                    &t.text,
                    Align::End,
                    Baseline::Top,
                    Role::Ink2,
                );
            }
        }
        Body::Lines { series, .. } => {
            let Some(xs) = xs else { return };
            for s in series {
                let role = color_role(s.color);
                let pts: Vec<f32> = s
                    .points
                    .iter()
                    .flat_map(|p| [xs.px(f(p.x)), ys.px(f(p.y))])
                    .collect();
                if pts.len() >= 4 {
                    cx.out.prims.push(Prim::Poly {
                        pts: pts.clone(),
                        closed: false,
                        stroke: role,
                        width: 2.0 * u,
                        alpha: 1.0,
                    });
                }
                let markers = s.points.len() <= 60;
                for (p, xy) in s.points.iter().zip(pts.chunks_exact(2)) {
                    cx.out.targets.push(Target::Point {
                        x: xy[0],
                        y: xy[1],
                        elem: p.element as u32,
                    });
                    let marked = looks.marked(p.element);
                    if markers || marked || s.points.len() == 1 {
                        cx.out.prims.push(Prim::Circle {
                            x: xy[0],
                            y: xy[1],
                            r: 4.0 * u,
                            fill: Some(if marked { Role::Selection } else { role }),
                            stroke: Some(Role::Surface),
                            width: 2.0 * u,
                            alpha: 1.0,
                            elem: p.element as i32,
                        });
                    }
                }
            }
        }
        Body::Pie { .. } => {}
    }
}

/// A box: its whiskers and caps, the box, the median, the mean and the outliers.
fn draw_box(cx: &mut Ctx, b: &BoxItem, elem: usize, x: f32, w: f32, ys: &Scale, looks: Looks) {
    let u = cx.u;
    for (v, e) in &b.outliers {
        cx.out.targets.push(Target::Point {
            x,
            y: ys.px(f(*v)),
            elem: *e as u32,
        });
    }
    if b.n == 0 {
        return;
    }
    let (role, alpha) = looks.of(elem, Role::Series(0));
    let (yl, y1, ym, y3, yh) = (
        ys.px(f(b.low)),
        ys.px(f(b.q1)),
        ys.px(f(b.median)),
        ys.px(f(b.q3)),
        ys.px(f(b.high)),
    );
    let line =
        |cx: &mut Ctx, x1: f32, y1: f32, x2: f32, y2: f32, role: Role, width: f32, alpha: f32| {
            cx.out.prims.push(Prim::Line {
                x1,
                y1,
                x2,
                y2,
                stroke: role,
                width,
                alpha,
            });
        };
    line(cx, x, yl, x, y1, role, 1.5 * u, alpha);
    line(cx, x, y3, x, yh, role, 1.5 * u, alpha);
    line(cx, x - w / 4.0, yl, x + w / 4.0, yl, role, 1.5 * u, alpha);
    line(cx, x - w / 4.0, yh, x + w / 4.0, yh, role, 1.5 * u, alpha);
    cx.out.prims.push(Prim::Rect {
        x: x - w / 2.0,
        y: y3,
        w,
        h: (y1 - y3).max(1.0),
        r_top: 0.0,
        r_bottom: 0.0,
        fill: role,
        alpha: 0.28 * alpha,
        elem: elem as i32,
    });
    cx.out.prims.push(Prim::Poly {
        pts: vec![
            x - w / 2.0,
            y3,
            x + w / 2.0,
            y3,
            x + w / 2.0,
            y1,
            x - w / 2.0,
            y1,
        ],
        closed: true,
        stroke: role,
        width: 1.5 * u,
        alpha,
    });
    line(
        cx,
        x - w / 2.0,
        ym,
        x + w / 2.0,
        ym,
        Role::Ink1,
        2.0 * u,
        alpha,
    );
    if let Some(m) = b.mean {
        cx.out.prims.push(Prim::Cross {
            x,
            y: ys.px(f(m)),
            size: 3.0 * u,
            stroke: Role::Ink2,
            width: 1.5 * u,
        });
    }
    for (v, e) in &b.outliers {
        let marked = looks.marked(*e);
        cx.out.prims.push(Prim::Circle {
            x,
            y: ys.px(f(*v)),
            r: 3.0 * u,
            fill: None,
            stroke: Some(if marked { Role::Selection } else { role }),
            width: 1.5 * u,
            alpha: if marked { 1.0 } else { alpha },
            elem: *e as i32,
        });
    }
}

/// Category names under their bands, shortened to the band; in narrow bands every k-th.
fn category_labels(cx: &mut Ctx, labels: &[String], left: f32, band: f32, y: f32) {
    let u = cx.u;
    let k = if band >= 24.0 * u {
        1
    } else {
        ((28.0 * u) / band).ceil().max(1.0) as usize
    };
    for (i, l) in labels.iter().enumerate() {
        if i % k != 0 {
            continue;
        }
        let room = band * k as f32 - 4.0 * u;
        let text = fit(l, room, cx.font, cx.family);
        cx.text(
            left + band * (i as f32 + 0.5),
            y,
            &text,
            Align::Middle,
            Baseline::Top,
            Role::Ink3,
        );
    }
}

/// The trend line y = a + b·x within the plot, in pixels.
fn clip_line(
    a: f64,
    b: f64,
    xs: &Scale,
    ys: &Scale,
    plot: [f32; 4],
) -> Option<((f32, f32), (f32, f32))> {
    let [left, top, right, bottom] = plot;
    let p = (left, ys.px(a + b * xs.d0));
    let q = (right, ys.px(a + b * xs.d1));
    // Liang–Barsky against the plot's y band (x already spans it).
    let (dy, mut t0, mut t1) = (q.1 - p.1, 0.0f32, 1.0f32);
    for (num, den) in [(p.1 - top, -dy), (bottom - p.1, dy)] {
        if den == 0.0 {
            if num < 0.0 {
                return None;
            }
            continue;
        }
        let t = num / den;
        if den < 0.0 {
            if t > t1 {
                return None;
            }
            if t > t0 {
                t0 = t;
            }
        } else {
            if t < t0 {
                return None;
            }
            if t < t1 {
                t1 = t;
            }
        }
    }
    let at = |t: f32| (p.0 + (q.0 - p.0) * t, p.1 + dy * t);
    Some((at(t0), at(t1)))
}

fn range_of(vals: impl Iterator<Item = Dec>) -> Option<(Dec, Dec)> {
    let mut out: Option<(Dec, Dec)> = None;
    for v in vals {
        out = Some(match out {
            None => (v, v),
            Some((lo, hi)) => (
                if crate::ops::statistics::compare(v, lo).is_lt() {
                    v
                } else {
                    lo
                },
                if crate::ops::statistics::compare(v, hi).is_gt() {
                    v
                } else {
                    hi
                },
            ),
        });
    }
    out
}

fn pie(cx: &mut Ctx, model: &Model, looks: Looks, frame: &Frame, pad: f32, line_h: f32) {
    let u = cx.u;
    let font = cx.font;
    let Body::Pie {
        slices,
        total,
        word,
    } = &model.body
    else {
        return;
    };
    cx.text(
        pad,
        pad,
        &model.y_title.clone(),
        Align::Start,
        Baseline::Top,
        Role::Ink2,
    );
    let top = pad + line_h + 6.0 * u;
    let legend_w = (0.45 * frame.width).min(300.0 * u);
    let plot_w = frame.width - 2.0 * pad - legend_w - 16.0 * u;
    let plot_h = frame.height - top - pad;
    let r1 = (plot_w.min(plot_h) / 2.0 - 2.0 * u).max(0.0);
    let (ccx, ccy) = (pad + plot_w / 2.0, top + plot_h / 2.0);
    cx.out.plot = [pad, top, plot_w, plot_h];
    if slices.is_empty() {
        cx.text(
            ccx,
            ccy,
            "Dilim yok",
            Align::Middle,
            Baseline::Middle,
            Role::Ink3,
        );
        return;
    }
    let total_f = f(*total);
    if r1 >= 20.0 * u && total_f > 0.0 {
        let r0 = 0.58 * r1;
        let mut acc = 0.0f64;
        let mut bounds = Vec::new();
        for (i, s) in slices.iter().enumerate() {
            let a0 = (acc / total_f * 2.0 * std::f64::consts::PI) as f32;
            acc += f(s.value);
            let a1 = if i + 1 == slices.len() {
                2.0 * PI
            } else {
                (acc / total_f * 2.0 * std::f64::consts::PI) as f32
            };
            let (role, alpha) = looks.of(i, color_role(s.color));
            cx.out.prims.push(Prim::Wedge {
                cx: ccx,
                cy: ccy,
                r0,
                r1,
                a0,
                a1,
                fill: role,
                alpha,
                elem: i as i32,
            });
            cx.out.targets.push(Target::Wedge {
                cx: ccx,
                cy: ccy,
                r0: r0 - 4.0 * u,
                r1: r1 + 4.0 * u,
                a0,
                a1,
                elem: i as u32,
            });
            bounds.push(a0);
        }
        // The 2u gaps of the surface between the slices.
        if slices.len() > 1 {
            for a in bounds {
                let (s, c) = (
                    crate::jsmath::sin(f64::from(a)) as f32,
                    crate::jsmath::cos(f64::from(a)) as f32,
                );
                cx.line(
                    ccx + s * (r0 - u),
                    ccy - c * (r0 - u),
                    ccx + s * (r1 + u),
                    ccy - c * (r1 + u),
                    Role::Surface,
                    2.0 * u,
                );
            }
        }
        cx.text_at(
            ccx,
            ccy - 2.0 * u,
            &total.text(),
            Align::Middle,
            Baseline::Bottom,
            Role::Ink1,
            1.4 * font,
            600,
        );
        cx.text(
            ccx,
            ccy + 2.0 * u,
            word,
            Align::Middle,
            Baseline::Top,
            Role::Ink3,
        );
    }
    // The legend: the categories' name above, then a row a slice.
    let lx = frame.width - pad - legend_w;
    let row_h = 1.6 * font;
    cx.text_at(
        lx,
        top,
        &model.x_title.clone(),
        Align::Start,
        Baseline::Top,
        Role::Ink2,
        font,
        500,
    );
    let mut y = top + row_h;
    let rows_fit = (((frame.height - pad) - y) / row_h).floor().max(0.0) as usize;
    let shown = if slices.len() > rows_fit {
        rows_fit.saturating_sub(1)
    } else {
        slices.len()
    };
    for (i, s) in slices.iter().take(shown).enumerate() {
        let (role, alpha) = looks.of(i, color_role(s.color));
        let mid = y + row_h / 2.0;
        cx.out.prims.push(Prim::Rect {
            x: lx,
            y: mid - 5.0 * u,
            w: 10.0 * u,
            h: 10.0 * u,
            r_top: 2.0 * u,
            r_bottom: 2.0 * u,
            fill: role,
            alpha,
            elem: i as i32,
        });
        // The share in a column of its own at the right, the value before it.
        let share = format!("%{}", s.share);
        let value = s.value.text();
        let share_w = cx.w("%100.0");
        cx.text(
            lx + legend_w,
            mid,
            &share,
            Align::End,
            Baseline::Middle,
            Role::Ink2,
        );
        let value_right = lx + legend_w - share_w - 10.0 * u;
        cx.text(
            value_right,
            mid,
            &value,
            Align::End,
            Baseline::Middle,
            Role::Ink2,
        );
        let room = value_right - cx.w(&value) - 10.0 * u - (lx + 16.0 * u);
        let name = fit(&s.label, room, font, cx.family);
        cx.text(
            lx + 16.0 * u,
            mid,
            &name,
            Align::Start,
            Baseline::Middle,
            Role::Ink1,
        );
        cx.out.targets.push(Target::Band {
            x0: lx,
            x1: lx + legend_w,
            y0: y,
            y1: y + row_h,
            elem: i as u32,
        });
        y += row_h;
    }
    if shown < slices.len() {
        let more = format!("+{} dilim", slices.len() - shown);
        cx.text(
            lx + 16.0 * u,
            y + row_h / 2.0,
            &more,
            Align::Start,
            Baseline::Middle,
            Role::Ink3,
        );
    }
}

/// Where element `e` is pressed on a layout: its band's middle, its slice's
/// middle, its point; none when it is not on the chart.
pub fn element_middle(layout: &Layout, e: u32) -> Option<(f32, f32)> {
    layout.targets.iter().find_map(|t| match *t {
        Target::Band {
            x0,
            x1,
            y0,
            y1,
            elem,
        } if elem == e => Some(((x0 + x1) / 2.0, (y0 + y1) / 2.0)),
        Target::Wedge {
            cx,
            cy,
            r0,
            r1,
            a0,
            a1,
            elem,
        } if elem == e => {
            let (a, r) = ((a0 + a1) / 2.0, (r0 + r1) / 2.0);
            Some((cx + r * a.sin(), cy - r * a.cos()))
        }
        Target::Point { x, y, elem } if elem == e => Some((x, y)),
        _ => None,
    })
}

/// The element under a point: a legend row, a slice, a point (12u at most,
/// the nearest; the first among equals) or a band.
pub fn hit(layout: &Layout, x: f32, y: f32) -> Option<u32> {
    let mut best: Option<(f32, u32)> = None;
    let reach2 = layout.reach * layout.reach;
    let mut consider = |px: f32, py: f32, e: u32| {
        let d2 = (px - x) * (px - x) + (py - y) * (py - y);
        if d2 <= reach2 && best.is_none_or(|(b, _)| d2 < b) {
            best = Some((d2, e));
        }
    };
    for t in &layout.targets {
        if let Target::Point { x: px, y: py, elem } = t {
            consider(*px, *py, *elem);
        }
    }
    for (i, xy) in layout.points.chunks_exact(2).enumerate() {
        consider(xy[0], xy[1], layout.point_elems[i]);
    }
    if let Some((_, e)) = best {
        return Some(e);
    }
    for t in &layout.targets {
        match t {
            Target::Wedge {
                cx,
                cy,
                r0,
                r1,
                a0,
                a1,
                elem,
            } => {
                let (dx, dy) = (x - cx, y - cy);
                let r = (dx * dx + dy * dy).sqrt();
                if r < *r0 || r > *r1 {
                    continue;
                }
                // Clockwise from twelve o'clock.
                let mut a = crate::jsmath::atan2(f64::from(dx), f64::from(-dy)) as f32;
                if a < 0.0 {
                    a += 2.0 * PI;
                }
                if a >= *a0 && a < *a1 {
                    return Some(*elem);
                }
            }
            Target::Band {
                x0,
                x1,
                y0,
                y1,
                elem,
            } => {
                if x >= *x0 && x < *x1 && y >= *y0 && y <= *y1 {
                    return Some(*elem);
                }
            }
            Target::Point { .. } => {}
        }
    }
    None
}

/// The elements of the points within a rectangle (Saçılım's dragged box).
pub fn within(layout: &Layout, x0: f32, y0: f32, x1: f32, y1: f32) -> Vec<u32> {
    let (xa, xb) = (x0.min(x1), x0.max(x1));
    let (ya, yb) = (y0.min(y1), y0.max(y1));
    let inside = |x: f32, y: f32| x >= xa && x <= xb && y >= ya && y <= yb;
    let mut out: Vec<u32> = layout
        .points
        .chunks_exact(2)
        .enumerate()
        .filter(|(_, xy)| inside(xy[0], xy[1]))
        .map(|(i, _)| layout.point_elems[i])
        .collect();
    for t in &layout.targets {
        if let Target::Point { x, y, elem } = t
            && inside(*x, *y)
        {
            out.push(*elem);
        }
    }
    out.sort_unstable();
    out.dedup();
    out
}
