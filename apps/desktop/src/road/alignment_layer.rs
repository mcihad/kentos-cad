//! A road project's alignments over the drawing (plan/road.md Y3, docs/adr/0901): each alignment's elements by kind
//! (tangents in the drawing's ink, clothoids in the warning colour, curves in the accent), its PI line dashed, its km
//! ticks (every 20 m, every 100 m longer with the km written along the normal, the way Netcad and Civil 3D write a
//! road's stationing), the curves' TS, SC, CS and ST marked; a parallel axis thin in its source's colours.
//!
//! The alignments are built by `road.alignments`' rule ([`kentos_native_application::road_alignments`]) once for a
//! revision of the drawing; the picture is drawn once for a view and a revision (a canvas cache). Ticks closer than six
//! pixels are left out (a zoomed-out road would be a comb), the km words when closer than sixty.

use std::cell::{Cell, RefCell};

use iced::widget::canvas::{self, LineDash, Path, Stroke};
use iced::{Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Theme, mouse};
use kentos_domain::Document;
use kentos_interaction::Vec2;
use kentos_native_application::road_alignments::{Built, made_alignments};
use kentos_render_wgpu::Camera;
use kentos_road::alignment::{Kind, km_text};

use crate::app::Message;

/// What the layer draws in: the drawing's ink and the theme's colours.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Colors {
    pub ink: Color,
    pub accent: Color,
    pub warning: Color,
}

/// An alignment in the world: its pieces by kind, its PI line, its ticks and its curve points.
#[derive(Debug, Default)]
struct Drawn {
    parallel: bool,
    pieces: Vec<(Kind, Vec<[f64; 2]>)>,
    pis: Vec<[f64; 2]>,
    /// Station, place, normal (right), km.
    ticks: Vec<(f64, [f64; 2], [f64; 2], f64)>,
    marks: Vec<(&'static str, [f64; 2])>,
    name: String,
    start: Option<([f64; 2], f64)>,
}

fn drawn(def: &kentos_commands::AlignmentDef, b: &Built) -> Drawn {
    let place = |s: f64| b.place(s).map(|p| ([p.x, p.y], p.direction));
    let mut out = Drawn {
        parallel: def.offset_of.is_some(),
        name: def.name.clone(),
        pis: def.pis.iter().map(|p| [p.x, p.y]).collect(),
        ..Drawn::default()
    };
    for e in &b.core.elements {
        let turn = e.k0.abs().max(e.k1.abs()) * e.length;
        let n = if e.kind == Kind::Tangent { 1 } else { ((turn / 0.01).ceil() as usize).clamp(4, 2000) };
        let pts: Vec<[f64; 2]> = (0..=n)
            .filter_map(|i| place(e.station + e.length * i as f64 / n as f64).map(|(p, _)| p))
            .collect();
        out.pieces.push((e.kind, pts));
    }
    if !out.parallel {
        // Ticks at the round kms of every region, every 20 m.
        let step = 20.0;
        for r in &b.regions {
            let (a, z) = (r.ahead, r.ahead + r.dir * (r.until - r.at));
            let (lo, hi) = (a.min(z), a.max(z));
            let mut k = (lo / step).ceil() * step;
            while k <= hi + 1e-9 {
                let s = r.at + r.dir * (k - r.ahead);
                if (r.at..=r.until).contains(&s)
                    && let Some((p, dir)) = place(s)
                {
                    out.ticks.push((s, p, [dir.sin(), -dir.cos()], k));
                }
                k += step;
            }
        }
        for c in &b.core.curves {
            let names: [&'static str; 4] = if c.spiral_in > 0.0 || c.spiral_out > 0.0 {
                ["TS", "SC", "CS", "ST"]
            } else {
                ["TC", "TC", "CT", "CT"]
            };
            for (i, s) in c.stations.iter().enumerate() {
                if let Some((p, _)) = place(*s)
                    && out.marks.iter().all(|(_, q)| (q[0] - p[0]).hypot(q[1] - p[1]) > 1e-6)
                {
                    out.marks.push((names[i], p));
                }
            }
        }
        out.start = place(0.0);
    }
    out
}

/// The layer: the drawing whose alignments it draws, and how.
pub(crate) struct Layer<'a> {
    doc: &'a Document,
    camera: Camera,
    colors: Colors,
}

/// The alignments of a revision, and the picture of a view.
#[derive(Default)]
pub(crate) struct State {
    built: RefCell<Option<(u64, Vec<Drawn>)>>,
    cache: canvas::Cache,
    key: Cell<u64>,
}

/// The layer of a road project with alignments; none otherwise.
pub(crate) fn layer<'a>(doc: &'a Document, camera: &Camera, colors: Colors) -> Option<Element<'a, Message>> {
    if doc.road_alignments().is_empty() || doc.settings().cad_work != Some(kentos_commands::CadWork::Road) {
        return None;
    }
    Some(
        canvas::Canvas::new(Layer {
            doc,
            camera: *camera,
            colors,
        })
        .width(Fill)
        .height(Fill)
        .into(),
    )
}

impl Layer<'_> {
    fn key(&self, bounds: Rectangle) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.doc.revision().hash(&mut h);
        for p in [Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)] {
            for c in self.camera.world_to_screen(p) {
                c.to_bits().hash(&mut h);
            }
        }
        for c in [bounds.width, bounds.height, self.colors.ink.r, self.colors.accent.g, self.colors.warning.b] {
            c.to_bits().hash(&mut h);
        }
        h.finish()
    }
}

impl canvas::Program<Message> for Layer<'_> {
    type State = State;

    fn draw(
        &self,
        state: &State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let revision = self.doc.revision();
        if state.built.borrow().as_ref().is_none_or(|(r, _)| *r != revision) {
            let all = made_alignments(self.doc)
                .into_iter()
                .filter_map(|(d, b)| b.ok().map(|b| drawn(&d, &b)))
                .collect();
            *state.built.borrow_mut() = Some((revision, all));
        }
        let key = self.key(bounds);
        if state.key.get() != key {
            state.cache.clear();
            state.key.set(key);
        }
        let built = state.built.borrow();
        let Some((_, all)) = built.as_ref() else {
            return Vec::new();
        };
        let c = self.colors;
        vec![state.cache.draw(renderer, bounds.size(), |frame| {
            let screen = |p: [f64; 2]| {
                let [x, y] = self.camera.world_to_screen(Vec2::new(p[0], p[1]));
                Point::new(x as f32, y as f32)
            };
            let line = |pts: &[[f64; 2]]| {
                Path::new(|b| {
                    for (i, p) in pts.iter().enumerate() {
                        if i == 0 {
                            b.move_to(screen(*p));
                        } else {
                            b.line_to(screen(*p));
                        }
                    }
                })
            };
            // Metres per pixel, near enough for leaving out what is too close.
            let o = self.camera.world_to_screen(Vec2::new(0.0, 0.0));
            let one = self.camera.world_to_screen(Vec2::new(1.0, 0.0));
            let px = (one[0] - o[0]).hypot(one[1] - o[1]).max(1e-12);
            for d in all {
                if d.pis.len() > 1 {
                    frame.stroke(
                        &line(&d.pis),
                        Stroke {
                            line_dash: LineDash {
                                segments: &[6.0, 4.0],
                                offset: 0,
                            },
                            ..Stroke::default().with_width(0.8).with_color(Color { a: 0.5, ..c.ink })
                        },
                    );
                    for p in &d.pis {
                        frame.stroke(
                            &Path::circle(screen(*p), 3.0),
                            Stroke::default().with_width(1.0).with_color(Color { a: 0.7, ..c.ink }),
                        );
                    }
                }
                let width = if d.parallel { 1.2 } else { 2.6 };
                for (kind, pts) in &d.pieces {
                    let color = match kind {
                        Kind::Tangent => c.ink,
                        Kind::Spiral => c.warning,
                        Kind::Arc => c.accent,
                    };
                    let color = if d.parallel { Color { a: 0.75, ..color } } else { color };
                    frame.stroke(&line(pts), Stroke::default().with_width(width).with_color(color));
                }
                let tick_gap = 20.0 * px;
                let word_gap = 100.0 * px;
                for (_, p, n, km) in &d.ticks {
                    let major = (km / 100.0).round() * 100.0 == *km;
                    if !major && tick_gap < 6.0 {
                        continue;
                    }
                    if major && word_gap < 6.0 {
                        continue;
                    }
                    let len = if major { 9.0 } else { 5.0 };
                    let at = screen(*p);
                    // The normal on screen: y grows down.
                    let (nx, ny) = (n[0] as f32, -(n[1] as f32));
                    frame.stroke(
                        &Path::line(
                            Point::new(at.x - nx * len, at.y - ny * len),
                            Point::new(at.x + nx * len, at.y + ny * len),
                        ),
                        Stroke::default().with_width(if major { 1.4 } else { 1.0 }).with_color(c.ink),
                    );
                    if major && word_gap >= 60.0 {
                        let words = km_text(*km, 0);
                        frame.with_save(|f| {
                            f.translate(iced::Vector::new(at.x + nx * 13.0, at.y + ny * 13.0));
                            // Along the normal, read left to right.
                            let mut angle = ny.atan2(nx);
                            if !(-std::f32::consts::FRAC_PI_2..=std::f32::consts::FRAC_PI_2).contains(&angle) {
                                angle += std::f32::consts::PI;
                                f.rotate(angle);
                                f.fill_text(canvas::Text {
                                    content: words.clone(),
                                    position: Point::new(0.0, 0.0),
                                    color: c.ink,
                                    size: Pixels(11.0),
                                    align_x: iced::widget::text::Alignment::Right,
                                    align_y: iced::alignment::Vertical::Center,
                                    ..canvas::Text::default()
                                });
                            } else {
                                f.rotate(angle);
                                f.fill_text(canvas::Text {
                                    content: words.clone(),
                                    position: Point::new(0.0, 0.0),
                                    color: c.ink,
                                    size: Pixels(11.0),
                                    align_y: iced::alignment::Vertical::Center,
                                    ..canvas::Text::default()
                                });
                            }
                        });
                    }
                }
                if word_gap >= 30.0 {
                    for (name, p) in &d.marks {
                        let at = screen(*p);
                        frame.fill(&Path::circle(at, 3.2), c.accent);
                        frame.fill_text(canvas::Text {
                            content: (*name).to_owned(),
                            position: Point::new(at.x + 5.0, at.y - 12.0),
                            color: c.accent,
                            size: Pixels(10.0),
                            ..canvas::Text::default()
                        });
                    }
                }
                if let Some((p, dir)) = d.start {
                    // Left of the start, away from the km words on the right.
                    let at = screen(p);
                    let (lx, ly) = (-(dir.sin() as f32), -(dir.cos() as f32));
                    frame.fill_text(canvas::Text {
                        content: d.name.clone(),
                        position: Point::new(at.x + lx * 16.0, at.y + ly * 16.0 - 6.0),
                        color: c.ink,
                        size: Pixels(12.0),
                        ..canvas::Text::default()
                    });
                }
            }
        })]
    }
}
