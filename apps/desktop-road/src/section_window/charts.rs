//! The section chart and the plan beside the editors, with the lines clipped to them.

use super::*;

/// The part of a line (offsets increasing) between offsets `a` and `b`, its ends on it.
pub(super) fn clip_line(line: &[[f64; 2]], a: f64, b: f64) -> Vec<[f64; 2]> {
    let at = |o: f64| {
        let i = line.partition_point(|p| p[0] < o).clamp(1, line.len().max(2) - 1);
        let (p, q) = (line[i - 1], line[i]);
        if q[0] == p[0] { [o, q[1]] } else { [o, p[1] + (q[1] - p[1]) * (o - p[0]) / (q[0] - p[0])] }
    };
    if line.len() < 2 {
        return Vec::new();
    }
    let mut out = vec![at(a)];
    out.extend(line.iter().copied().filter(|p| p[0] > a && p[0] < b));
    out.push(at(b));
    out
}

impl canvas::Program<Message> for Chart<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        // What is drawn: the ground as (offset, z or none, gap), the design line, the daylight points.
        let (ground, design, daylight): Drawn = match (self.row, &self.sample) {
            (Some(r), _) => (
                r.ground.iter().map(|g| (g.offset, g.z, g.gap)).collect(),
                road_sections_draw::design_line(r),
                r.left.iter().chain(r.right.iter()).filter_map(|s| s.daylight).collect(),
            ),
            (None, Some((g, s))) => (
                g.iter().map(|p| (p.offset, p.z, p.gap)).collect(),
                s.left.points.iter().rev().chain(s.right.points.iter().skip(1)).copied().collect(),
                [s.left.daylight, s.right.daylight].into_iter().flatten().collect(),
            ),
            _ => (Vec::new(), Vec::new(), Vec::new()),
        };
        let word = |frame: &mut canvas::Frame, s: String, at: Point, color: Color, centred: bool| {
            frame.fill_text(Text {
                content: s,
                position: at,
                color,
                size: Pixels(typography::caption()),
                font: typography::ui(),
                align_x: if centred { iced::widget::text::Alignment::Center } else { iced::widget::text::Alignment::Default },
                ..Text::default()
            });
        };
        let offsets: Vec<f64> = ground.iter().map(|g| g.0).chain(design.iter().map(|p| p[0])).collect();
        let zs: Vec<f64> = ground.iter().filter_map(|g| g.1).chain(design.iter().map(|p| p[1])).chain(self.row.and_then(|r| r.axis)).collect();
        let (Some(o0), Some(o1), Some(z0), Some(z1)) = (
            offsets.iter().copied().reduce(f64::min),
            offsets.iter().copied().reduce(f64::max),
            zs.iter().copied().reduce(f64::min),
            zs.iter().copied().reduce(f64::max),
        ) else {
            word(&mut frame, "Takımın yüzeyi ve km'leri verilince enkesit burada çizilir.".into(), Point::new(16.0, 20.0), t.muted, false);
            return vec![frame.into_geometry()];
        };
        let pad = ((z1 - z0) * 0.1).max(0.5);
        let (z0, z1) = (z0 - pad, z1 + pad);
        let plot = Rectangle { x: 54.0, y: 10.0, width: bounds.width - 70.0, height: bounds.height - 34.0 };
        let sx = plot.width as f64 / (o1 - o0);
        let sy = match self.exaggeration {
            Some(e) => (sx * e).min(plot.height as f64 / (z1 - z0)),
            None => plot.height as f64 / (z1 - z0),
        };
        // The heights centred in the plot when they do not fill it.
        let lift = 0.5 * (plot.height as f64 - (z1 - z0) * sy);
        let x = |o: f64| plot.x + ((o - o0) * sx) as f32;
        let y = |z: f64| plot.y + plot.height - (lift + (z - z0) * sy) as f32;
        let thin = |c: Color| Stroke::default().with_color(c).with_width(1.0);
        let (z0, z1) = (z0 - lift / sy, z1 + lift / sy);
        // The grid from the pixels (docs/adr/0937 §3): heights at least 45 px apart, offsets 70 px. Zoomed in, the chart
        // is laid out larger and finer lines come in; the labels stay on the part shown, on a backdrop where they move in.
        let shown = crate::picture::Shown::of(bounds.size());
        let left = shown.left(4.0);
        let base = shown.bottom(plot.y + plot.height + 6.0, typography::caption());
        let backdrop = |moved: bool| moved.then_some(t.field.scale_alpha(0.9));
        let step = crate::picture::step(45.0 / sy);
        let places = crate::picture::decimals(step);
        let mut z = (z0 / step).ceil() * step;
        while z <= z1 {
            if shown.has_y(y(z)) {
                frame.stroke(&Path::line(Point::new(x(o0), y(z)), Point::new(x(o1), y(z))), thin(t.border));
            }
            z += step;
        }
        let ostep = crate::picture::step(70.0 / sx);
        let oplaces = crate::picture::decimals(ostep);
        let mut o = (o0 / ostep).ceil() * ostep;
        while o <= o1 + 1e-9 {
            if shown.has_x(x(o)) {
                frame.stroke(&Path::line(Point::new(x(o), plot.y), Point::new(x(o), plot.y + plot.height)), thin(t.border));
            }
            o += ostep;
        }
        let labels = |frame: &mut canvas::Frame| {
            let mut z = (z0 / step).ceil() * step;
            while z <= z1 {
                // Not into the offsets' row at the bottom.
                if shown.has_y(y(z)) && y(z) < base - 8.0 {
                    crate::picture::label(frame, comma(z, places), Point::new(left, y(z) - 7.0), t.muted, false, backdrop(left > 4.5));
                }
                z += step;
            }
            let mut o = (o0 / ostep).ceil() * ostep;
            while o <= o1 + 1e-9 {
                if shown.has_x(x(o)) {
                    let words = if oplaces == 0 { format!("{}", o.round() + 0.0) } else { comma(o + 0.0, oplaces) };
                    crate::picture::label(frame, words, Point::new(x(o), base), t.muted, true, backdrop(base < plot.y + plot.height + 5.5));
                }
                o += ostep;
            }
        };
        // The axis.
        if o0 <= 0.0 && o1 >= 0.0 {
            frame.stroke(
                &Path::line(Point::new(x(0.0), plot.y), Point::new(x(0.0), plot.y + plot.height)),
                Stroke { line_dash: LineDash { segments: &[5.0, 4.0], offset: 0 }, ..thin(t.accent) },
            );
        }
        // Docs/adr/1034: the swath's stretches off the surface, faint, so a design over them shows it has no ground.
        if let Some(r) = self.row {
            for m in &r.missing {
                let (a, b) = (x(m[0].max(o0)), x(m[1].min(o1)));
                if b > a + 0.5 {
                    frame.fill_rectangle(Point::new(a, plot.y), iced::Size::new(b - a, plot.height), t.warning.scale_alpha(0.12));
                    if b - a > 60.0 {
                        word(&mut frame, "arazi yok".into(), Point::new(a + 6.0, plot.y + 4.0), t.warning, false);
                    }
                }
            }
        }
        let terrain = Path::new(|b| {
            let mut open = false;
            for (o, z, gap) in &ground {
                match z {
                    Some(z) if open && !gap => b.line_to(Point::new(x(*o), y(*z))),
                    Some(z) => {
                        b.move_to(Point::new(x(*o), y(*z)));
                        open = true;
                    }
                    None => open = false,
                }
            }
        });
        frame.stroke(&terrain, Stroke::default().with_color(t.text).with_width(1.4));
        // The pavement (docs/adr/0975 §3): each layer a band between the line above it (the finish grade for the first)
        // and its bottom, toned from dark to light; the formation dashed.
        let bottoms: Option<&Vec<Vec<[f64; 2]>>> = self.row.and_then(|r| r.pavement.as_ref()).map(|p| &p.bottoms).or(self.layers.as_ref());
        if let Some(bottoms) = bottoms {
            let n = bottoms.len().max(1) as f32;
            let mut above: Vec<[f64; 2]> = Vec::new();
            for (k, b) in bottoms.iter().enumerate() {
                let (Some(first), Some(last)) = (b.first(), b.last()) else { continue };
                let top = if k == 0 { clip_line(&design, first[0], last[0]) } else { above.clone() };
                let band = Path::new(|p| {
                    for (i, q) in top.iter().enumerate() {
                        let at = Point::new(x(q[0]), y(q[1]));
                        if i == 0 { p.move_to(at) } else { p.line_to(at) }
                    }
                    for q in b.iter().rev() {
                        p.line_to(Point::new(x(q[0]), y(q[1])));
                    }
                    p.close();
                });
                let tone = 0.55 - 0.4 * (k as f32) / n;
                frame.fill(&band, Color { a: tone, ..t.muted });
                frame.stroke(&band, thin(Color { a: 0.8, ..t.muted }));
                above = b.clone();
            }
            if let Some(f) = bottoms.last() {
                let formation = Path::new(|p| {
                    for (i, q) in f.iter().enumerate() {
                        let at = Point::new(x(q[0]), y(q[1]));
                        if i == 0 { p.move_to(at) } else { p.line_to(at) }
                    }
                });
                frame.stroke(&formation, Stroke { line_dash: LineDash { segments: &[6.0, 3.0], offset: 0 }, ..Stroke::default().with_color(t.accent).with_width(1.8) });
            }
        }
        if design.len() > 1 {
            let line = Path::new(|b| {
                for (i, p) in design.iter().enumerate() {
                    let at = Point::new(x(p[0]), y(p[1]));
                    if i == 0 { b.move_to(at) } else { b.line_to(at) }
                }
            });
            frame.stroke(&line, Stroke::default().with_color(t.danger).with_width(2.0));
        }
        for d in &daylight {
            frame.fill(&Path::circle(Point::new(x(d[0]), y(d[1])), 3.5), t.danger);
        }
        if let Some(a) = self.row.and_then(|r| r.axis) {
            word(&mut frame, format!("Eksen {}", comma(a, 2)), Point::new(x(0.0) + 6.0, y(a) - 20.0), t.danger, false);
        }
        // The axes' labels last, over the drawing where the view has moved in.
        labels(&mut frame);
        vec![frame.into_geometry()]
    }
}

/// Whether two segments cross (not at an end shared by both).
pub(super) fn segments_cross(a: [[f64; 2]; 2], b: [[f64; 2]; 2]) -> bool {
    let o = |p: [f64; 2], q: [f64; 2], r: [f64; 2]| (q[0] - p[0]) * (r[1] - p[1]) - (q[1] - p[1]) * (r[0] - p[0]);
    let (d1, d2) = (o(b[0], b[1], a[0]), o(b[0], b[1], a[1]));
    let (d3, d4) = (o(a[0], a[1], b[0]), o(a[0], a[1], b[1]));
    let tiny = 1e-9 * (1.0 + a[0][0].abs().max(a[0][1].abs()));
    d1.abs() > tiny && d2.abs() > tiny && d3.abs() > tiny && d4.abs() > tiny && (d1 > 0.0) != (d2 > 0.0) && (d3 > 0.0) != (d4 > 0.0)
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let t = Tokens::of(theme);
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        if self.rows.is_empty() {
            return vec![frame.into_geometry()];
        }
        let pts = self.rows.iter().flat_map(|r| r.ends);
        let (mut lo, mut hi) = ([f64::INFINITY; 2], [f64::NEG_INFINITY; 2]);
        for p in pts {
            lo = [lo[0].min(p[0]), lo[1].min(p[1])];
            hi = [hi[0].max(p[0]), hi[1].max(p[1])];
        }
        let pad = 10.0f32;
        let span = (hi[0] - lo[0]).max(hi[1] - lo[1]).max(1e-6);
        let scale = f64::from((bounds.width - 2.0 * pad).min(bounds.height - 2.0 * pad)) / span;
        let (cx, cy) = (0.5 * (lo[0] + hi[0]), 0.5 * (lo[1] + hi[1]));
        let at = |p: [f64; 2]| {
            Point::new(
                bounds.width / 2.0 + ((p[0] - cx) * scale) as f32,
                bounds.height / 2.0 - ((p[1] - cy) * scale) as f32,
            )
        };
        let meets: Vec<bool> = (0..self.rows.len())
            .map(|i| {
                (0..self.rows.len()).any(|j| j != i && segments_cross(self.rows[i].ends, self.rows[j].ends))
            })
            .collect();
        for (i, r) in self.rows.iter().enumerate() {
            let color = if meets[i] { t.danger } else if i == self.at { t.accent } else { t.muted };
            let width = if i == self.at { 2.0 } else { 1.0 };
            frame.stroke(&Path::line(at(r.ends[0]), at(r.ends[1])), Stroke::default().with_color(color).with_width(width));
        }
        vec![frame.into_geometry()]
    }
}
