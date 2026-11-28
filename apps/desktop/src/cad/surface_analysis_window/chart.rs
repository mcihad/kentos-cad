//! Yüzey analizi's Grafik tab (surface_analysis_window.rs): the plan drawn from a reading (the ranges coloured, the
//! difference's bars, the check points and spikes), a range's colour swatch for the tables, and the fit of the plan
//! into the canvas.

use super::*;

pub(super) struct Swatch {
    pub(super) k: usize,
    pub(super) n: usize,
}

impl canvas::Program<Message> for Swatch {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        frame.fill_rectangle(
            Point::ORIGIN,
            bounds.size(),
            ramp(&Tokens::of(theme), self.k, self.n),
        );
        vec![frame.into_geometry()]
    }
}

/// The plan: the ranges' parts coloured (Analiz), the difference's ranges as bars (Fark), the check points coloured by
/// their difference and the spikes (Kalite).
pub(super) struct Plan<'a> {
    pub(super) read: &'a Read,
    pub(super) measure: usize,
    /// Yoğunluk's least: a cell with fewer points is sparse.
    pub(super) least: Option<u32>,
}

impl canvas::Program<Message> for Plan<'_> {
    type State = ();

    fn draw(
        &self,
        _: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let text = |frame: &mut canvas::Frame, s: String, p: Point| {
            frame.fill_text(Text {
                content: s,
                position: p,
                color: tk.muted,
                size: Pixels(11.0),
                ..Text::default()
            })
        };
        match self.read {
            Read::Analysis(a) => {
                let Some(b) = a
                    .elevations
                    .as_ref()
                    .or(a.slopes.as_ref())
                    .or(a.aspects.as_ref())
                else {
                    text(
                        &mut frame,
                        "Aralık sorulmadı.".into(),
                        Point::new(10.0, 10.0),
                    );
                    return vec![frame.into_geometry()];
                };
                let pts: Vec<[f64; 2]> = b
                    .bands
                    .iter()
                    .flat_map(|x| x.parts.iter().flatten().copied())
                    .collect();
                let Some(fit) = Fit::of(&pts, bounds) else {
                    return vec![frame.into_geometry()];
                };
                let n = b.bands.len();
                for (k, band) in b.bands.iter().enumerate() {
                    let c = ramp(&tk, k, n);
                    for part in &band.parts {
                        let shape = Path::new(|p| {
                            p.move_to(fit.at(part[0]));
                            for q in &part[1..] {
                                p.line_to(fit.at(*q));
                            }
                            p.close();
                        });
                        frame.fill(&shape, c);
                    }
                }
                text(
                    &mut frame,
                    ["Kot aralıkları", "Eğim aralıkları", "Bakı aralıkları"][self.measure].into(),
                    Point::new(10.0, 6.0),
                );
            }
            Read::Compare(c) => {
                // The material zones (docs/adr/3540) without ranges: each zone's and the rest's adjusted cut and fill.
                if c.bands.is_empty() && !c.zones.is_empty() {
                    let mut rows: Vec<(String, f64, f64)> = c
                        .zones
                        .iter()
                        .map(|z| (z.name.clone(), z.adjusted.cut, z.adjusted.fill))
                        .collect();
                    if let Some(r) = &c.rest {
                        rows.push(("Bölge dışı".to_owned(), r.adjusted.cut, r.adjusted.fill));
                    }
                    let most = rows
                        .iter()
                        .flat_map(|r| [r.1, r.2])
                        .fold(0.0_f64, f64::max)
                        .max(1e-9);
                    let h = ((bounds.height - 40.0) / rows.len() as f32).min(56.0);
                    text(
                        &mut frame,
                        "Malzemeye göre katsayılı kazı ve dolgu".into(),
                        Point::new(10.0, 6.0),
                    );
                    for (k, (name, cut, fill)) in rows.iter().enumerate() {
                        let y = 28.0 + k as f32 * h;
                        text(
                            &mut frame,
                            name.clone(),
                            Point::new(10.0, y + h / 2.0 - 9.0),
                        );
                        let bar = ((h - 10.0) / 2.0).max(2.0);
                        for (j, (v, color, word)) in
                            [(*cut, tk.danger, "kazı"), (*fill, tk.success, "dolgu")]
                                .into_iter()
                                .enumerate()
                        {
                            let w = ((v / most) as f32) * (bounds.width - 290.0).max(10.0);
                            let top = y + 2.0 + j as f32 * (bar + 2.0);
                            frame.fill_rectangle(
                                Point::new(130.0, top),
                                iced::Size::new(w.max(1.0), bar),
                                color,
                            );
                            text(
                                &mut frame,
                                format!("{word} {v:.2} m³"),
                                Point::new(136.0 + w, top + bar / 2.0 - 6.0),
                            );
                        }
                    }
                    return vec![frame.into_geometry()];
                }
                // The sections (docs/adr/4340) along the axis: the fill's area up, the cut's down.
                if !c.sections.is_empty() {
                    let last = c.sections.last().map_or(0.0, |s| s.station).max(1e-9);
                    let most = c
                        .sections
                        .iter()
                        .flat_map(|s| [s.cut_area, s.fill_area])
                        .fold(0.0_f64, f64::max)
                        .max(1e-9);
                    text(
                        &mut frame,
                        "Kesit alanları boyunca: dolgu yukarı, kazı aşağı".into(),
                        Point::new(10.0, 6.0),
                    );
                    let (left, right) = (70.0_f32, (bounds.width - 24.0).max(80.0));
                    let half = ((bounds.height - 70.0) / 2.0).max(8.0);
                    let mid = 30.0 + half;
                    frame.stroke(
                        &Path::line(Point::new(left, mid), Point::new(right, mid)),
                        Stroke::default().with_width(1.0).with_color(tk.muted),
                    );
                    let bar = ((right - left) / c.sections.len() as f32 * 0.6).clamp(2.0, 16.0);
                    for s in &c.sections {
                        let x = left + (s.station / last) as f32 * (right - left);
                        let (up, down) = (
                            (s.fill_area / most) as f32 * half,
                            (s.cut_area / most) as f32 * half,
                        );
                        if up > 0.0 {
                            frame.fill_rectangle(
                                Point::new(x - bar / 2.0, mid - up),
                                iced::Size::new(bar, up),
                                tk.success,
                            );
                        }
                        if down > 0.0 {
                            frame.fill_rectangle(
                                Point::new(x - bar / 2.0, mid),
                                iced::Size::new(bar, down),
                                tk.danger,
                            );
                        }
                    }
                    text(
                        &mut frame,
                        format!("{most:.1} m²"),
                        Point::new(10.0, mid - half),
                    );
                    text(
                        &mut frame,
                        format!("{most:.1} m²"),
                        Point::new(10.0, mid + half - 12.0),
                    );
                    text(
                        &mut frame,
                        "0 m".into(),
                        Point::new(left - 8.0, mid + half + 8.0),
                    );
                    text(
                        &mut frame,
                        format!("{last:.0} m"),
                        Point::new(right - 40.0, mid + half + 8.0),
                    );
                    return vec![frame.into_geometry()];
                }
                if c.bands.is_empty() {
                    let words = if c.cell.is_some() {
                        "Izgara yönteminde fark aralığı yok: hacimler ve kesin TIN'e göre yöntem farkı İçerik'te."
                    } else {
                        "Fark aralığı yazılmadı: hacimler İçerik'te."
                    };
                    text(&mut frame, words.into(), Point::new(10.0, 10.0));
                    return vec![frame.into_geometry()];
                }
                let most = c
                    .bands
                    .iter()
                    .map(|b| b.area)
                    .fold(0.0_f64, f64::max)
                    .max(1e-9);
                let h = (bounds.height - 40.0) / c.bands.len() as f32;
                for (k, b) in c.bands.iter().enumerate() {
                    let y = 20.0 + k as f32 * h;
                    let w = ((b.area / most) as f32) * (bounds.width - 220.0);
                    let color = if b.to <= 0.0 {
                        tk.danger
                    } else if b.from >= 0.0 {
                        tk.success
                    } else {
                        tk.warning
                    };
                    frame.fill_rectangle(
                        Point::new(140.0, y + 3.0),
                        iced::Size::new(w.max(1.0), (h - 6.0).max(2.0)),
                        color,
                    );
                    text(
                        &mut frame,
                        format!("{:+.2} … {:+.2} m", b.from, b.to),
                        Point::new(10.0, y + h / 2.0 - 7.0),
                    );
                    text(
                        &mut frame,
                        format!("{:.1} m²", b.area),
                        Point::new(146.0 + w, y + h / 2.0 - 7.0),
                    );
                }
            }
            Read::Coverage(_, pic) => {
                let pts: Vec<[f64; 2]> = pic
                    .triangles
                    .iter()
                    .flat_map(|t| t.0)
                    .chain(pic.rings.iter().flatten().copied())
                    .collect();
                let Some(fit) = Fit::of(&pts, bounds) else {
                    text(
                        &mut frame,
                        "Gösterilecek üçgen yok.".into(),
                        Point::new(10.0, 10.0),
                    );
                    return vec![frame.into_geometry()];
                };
                for (t, estimated) in &pic.triangles {
                    let shape = Path::new(|p| {
                        p.move_to(fit.at(t[0]));
                        p.line_to(fit.at(t[1]));
                        p.line_to(fit.at(t[2]));
                        p.close();
                    });
                    let color = if *estimated { tk.warning } else { tk.success };
                    frame.fill(&shape, Color { a: 0.35, ..color });
                    frame.stroke(
                        &shape,
                        Stroke::default()
                            .with_width(0.6)
                            .with_color(Color { a: 0.6, ..color }),
                    );
                }
                for ring in &pic.rings {
                    let shape = Path::new(|p| {
                        p.move_to(fit.at(ring[0]));
                        for q in &ring[1..] {
                            p.line_to(fit.at(*q));
                        }
                        p.close();
                    });
                    frame.stroke(
                        &shape,
                        Stroke::default().with_width(2.0).with_color(tk.accent),
                    );
                }
                text(
                    &mut frame,
                    "Yeşil ölçülen, turuncu tahmin edilen; çizgi proje sınırı".into(),
                    Point::new(10.0, 6.0),
                );
            }
            Read::Density(d, pic) => {
                let (w, h) = (pic.columns as f64 * pic.cell, pic.rows as f64 * pic.cell);
                let corners = [pic.origin, [pic.origin[0] + w, pic.origin[1] + h]];
                let pts: Vec<[f64; 2]> = corners
                    .into_iter()
                    .chain(pic.rings.iter().flatten().copied())
                    .collect();
                let Some(fit) = Fit::of(&pts, bounds) else {
                    text(
                        &mut frame,
                        "Gösterilecek hücre yok.".into(),
                        Point::new(10.0, 10.0),
                    );
                    return vec![frame.into_geometry()];
                };
                let most = d.most.max(1);
                for (k, n) in pic.counts.iter().enumerate() {
                    let Some(n) = *n else { continue };
                    let (i, j) = (k % pic.columns, k / pic.columns);
                    let x = pic.origin[0] + i as f64 * pic.cell;
                    let y = pic.origin[1] + j as f64 * pic.cell;
                    let (a, b) = (fit.at([x, y + pic.cell]), fit.at([x + pic.cell, y]));
                    let size = iced::Size::new((b.x - a.x).max(1.0), (b.y - a.y).max(1.0));
                    let sparse = d.sparse.is_some() && self.least.is_some_and(|l| n < l);
                    let color = if n == 0 {
                        Color {
                            a: 0.45,
                            ..tk.danger
                        }
                    } else if sparse {
                        Color {
                            a: 0.5,
                            ..tk.warning
                        }
                    } else {
                        Color {
                            a: 0.2 + 0.6 * (n as f32 / most as f32),
                            ..tk.success
                        }
                    };
                    frame.fill_rectangle(a, size, color);
                    if size.width > 14.0 && size.height > 12.0 {
                        frame.fill_text(Text {
                            content: n.to_string(),
                            position: Point::new(a.x + size.width / 2.0, a.y + size.height / 2.0),
                            color: tk.text,
                            size: Pixels(10.0),
                            align_x: iced::widget::text::Alignment::Center,
                            align_y: iced::alignment::Vertical::Center,
                            ..Text::default()
                        });
                    }
                }
                for ring in &pic.rings {
                    let shape = Path::new(|p| {
                        p.move_to(fit.at(ring[0]));
                        for q in &ring[1..] {
                            p.line_to(fit.at(*q));
                        }
                        p.close();
                    });
                    frame.stroke(
                        &shape,
                        Stroke::default().with_width(2.0).with_color(tk.accent),
                    );
                }
                text(
                    &mut frame,
                    "Hücrelerde nokta sayısı: kırmızı boş, turuncu seyrek; çizgi proje sınırı"
                        .into(),
                    Point::new(10.0, 6.0),
                );
            }
            Read::Quality(q) => {
                let mut pts: Vec<[f64; 2]> = q.spikes.iter().map(|s| [s.at[0], s.at[1]]).collect();
                if let Some(c) = &q.checkpoints {
                    pts.extend(c.points.iter().map(|p| p.at));
                }
                pts.extend(q.near.iter().map(|p| [p.a[0], p.a[1]]));
                let Some(fit) = Fit::of(&pts, bounds) else {
                    text(
                        &mut frame,
                        "Gösterilecek nokta yok.".into(),
                        Point::new(10.0, 10.0),
                    );
                    return vec![frame.into_geometry()];
                };
                if let Some(c) = &q.checkpoints {
                    let most = c
                        .points
                        .iter()
                        .filter_map(|p| p.difference)
                        .map(f64::abs)
                        .fold(0.0_f64, f64::max)
                        .max(1e-9);
                    for p in &c.points {
                        let at = fit.at(p.at);
                        match p.difference {
                            Some(d) => {
                                let color = if d >= 0.0 { tk.success } else { tk.danger };
                                let r = (3.0 + 7.0 * (d.abs() / most)) as f32;
                                frame.fill(&Path::circle(at, r), Color { a: 0.55, ..color });
                                // Far from its class (docs/adr/0945): a ring, so it is seen before it is counted.
                                if p.far {
                                    frame.stroke(
                                        &Path::circle(at, r + 4.0),
                                        Stroke::default().with_width(2.0).with_color(tk.warning),
                                    );
                                }
                                text(
                                    &mut frame,
                                    if p.far {
                                        format!("{d:+.3} uzak")
                                    } else {
                                        format!("{d:+.3}")
                                    },
                                    at + iced::Vector::new(r + 6.0, -6.0),
                                );
                            }
                            None => frame.stroke(
                                &Path::circle(at, 4.0),
                                Stroke::default().with_width(1.2).with_color(tk.muted),
                            ),
                        }
                    }
                }
                for s in &q.spikes {
                    let at = fit.at([s.at[0], s.at[1]]);
                    let up = if s.difference >= 0.0 { -1.0 } else { 1.0 };
                    let tri = Path::new(|p| {
                        p.move_to(at + iced::Vector::new(0.0, 7.0 * up));
                        p.line_to(at + iced::Vector::new(-6.0, -5.0 * up));
                        p.line_to(at + iced::Vector::new(6.0, -5.0 * up));
                        p.close();
                    });
                    frame.stroke(
                        &tri,
                        Stroke::default().with_width(1.5).with_color(tk.warning),
                    );
                }
                for p in &q.near {
                    frame.stroke(
                        &Path::circle(fit.at([p.a[0], p.a[1]]), 8.0),
                        Stroke::default().with_width(1.5).with_color(tk.accent),
                    );
                }
            }
        }
        vec![frame.into_geometry()]
    }
}

/// Plan places fitted into the picture.
struct Fit {
    x0: f64,
    y1: f64,
    scale: f64,
    ox: f64,
    oy: f64,
}

impl Fit {
    fn of(pts: &[[f64; 2]], bounds: Rectangle) -> Option<Self> {
        if pts.is_empty() {
            return None;
        }
        let (mut x0, mut y0, mut x1, mut y1) = (
            f64::INFINITY,
            f64::INFINITY,
            f64::NEG_INFINITY,
            f64::NEG_INFINITY,
        );
        for p in pts {
            x0 = x0.min(p[0]);
            y0 = y0.min(p[1]);
            x1 = x1.max(p[0]);
            y1 = y1.max(p[1]);
        }
        let pad = 26.0_f64;
        let (w, h) = (
            f64::from(bounds.width) - 2.0 * pad,
            f64::from(bounds.height) - 2.0 * pad,
        );
        let span = (x1 - x0).max(y1 - y0).max(1e-6);
        let scale = (w / (x1 - x0).max(span * 0.05)).min(h / (y1 - y0).max(span * 0.05));
        Some(Self {
            x0,
            y1,
            scale,
            ox: pad + (w - (x1 - x0) * scale) / 2.0,
            oy: pad + (h - (y1 - y0) * scale) / 2.0,
        })
    }

    fn at(&self, p: [f64; 2]) -> Point {
        Point::new(
            (self.ox + (p[0] - self.x0) * self.scale) as f32,
            (self.oy + (self.y1 - p[1]) * self.scale) as f32,
        )
    }
}
