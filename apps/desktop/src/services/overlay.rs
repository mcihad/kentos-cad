//! The map services over the drawing (docs/adr/0208 §3, §8, §9): the vector
//! tiles' labels where their styles place them (`kentos_services::labels`:
//! the later style layers first, a label that would cover another dropped,
//! a line's label letter by letter along it), drawn under the drawing's own
//! text; and the credits of the services shown, each once, in a strip at
//! the drawing area's bottom right under the scale bar. The strip is a
//! button: it opens a card listing the credits with their links.
//!
//! The labels a view shows are placed again only when the view, the
//! drawing, the colours or the services' tiles change (`Kept`); the picture
//! is kept while they stay.

use std::cell::{Cell, RefCell};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::rc::Rc;
use std::sync::Arc;

use iced::alignment::Vertical;
use iced::widget::canvas::{self, Frame, LineJoin, Stroke};
use iced::widget::text;
use iced::{Color, Element, Fill, Font, Pixels, Point, Rectangle, Renderer, Theme, Vector, mouse};
use kentos_contracts::{DrawingFont, ServiceKind, ServiceLayer};
use kentos_domain::Document;
use kentos_render_wgpu::Camera;
use kentos_render_wgpu::styled::service_tiles::{display_zoom, in_view};
use kentos_services::attribution::{self, Credit};
use kentos_services::labels::{self as rule, Candidate, Screen};

use super::hub::CreditView;
use crate::app::Message;
use crate::labels::Colors;

/// The services the drawing shows now, bottom first: each its key (the
/// hub's) and its layer's service.
pub fn shown(doc: &Document) -> Vec<(String, &ServiceLayer)> {
    let settings = doc.settings();
    crate::style::scene::shown_service_layers(doc.layers().nodes())
        .into_iter()
        .map(|service| {
            let connection = service
                .connection
                .as_ref()
                .and_then(|c| settings.connections.iter().find(|k| &k.id == c));
            (super::key_of(service, connection), service)
        })
        .collect()
}

/// A label as the overlay draws it: its lines upright or its letters along a line.
struct Drawn {
    lines: Vec<rule::Line>,
    glyphs: Vec<rule::Glyph>,
    size: f32,
    font: Font,
    bold: bool,
    color: Color,
    halo: Option<(Color, f32)>,
}

/// What a view shows of the services: its labels and its credits.
#[derive(Default)]
pub struct Marks {
    labels: Vec<Drawn>,
    pub credits: Vec<Credit>,
}

impl Marks {
    /// The strip's words: the credits joined, each once.
    pub fn strip(&self) -> String {
        attribution::joined(self.credits.iter().map(|c| c.text.as_str())).join(" | ")
    }
}

/// The marks last worked out and what they rest on: a frame whose view,
/// drawing, colours and tiles stay takes them from here.
#[derive(Default)]
pub struct Kept(RefCell<Option<(u64, Rc<Marks>)>>);

impl Kept {
    /// The marks for `doc` in `camera`'s view (`key` what they rest on).
    pub fn marks(&self, doc: &Document, camera: &Camera, colors: &Colors, key: u64) -> Rc<Marks> {
        if let Some((known, marks)) = self.0.borrow().as_ref()
            && *known == key
        {
            return marks.clone();
        }
        let marks = Rc::new(work_out(doc, camera, colors));
        *self.0.borrow_mut() = Some((key, marks.clone()));
        marks
    }
}

/// What the marks of a frame rest on.
pub fn key_of_view(doc: &Document, camera: &Camera, colors: &Colors) -> u64 {
    let mut h = DefaultHasher::new();
    (
        camera.center.x.to_bits(),
        camera.center.y.to_bits(),
        camera.scale.to_bits(),
        camera.width.to_bits(),
        camera.height.to_bits(),
        doc.generation(),
        super::hub().changes(),
    )
        .hash(&mut h);
    format!("{colors:?}").hash(&mut h);
    h.finish()
}

fn color(c: [f64; 4], opacity: f64) -> Color {
    Color::from_rgba(
        c[0] as f32,
        c[1] as f32,
        c[2] as f32,
        (c[3] * opacity).clamp(0.0, 1.0) as f32,
    )
}

/// Degrees of a Web Mercator point (Google's tiles').
fn degrees(x: f64, y: f64) -> (f64, f64) {
    const R: f64 = 6_378_137.0;
    let lon = (x / R).to_degrees();
    let lat = (2.0 * (y / R).exp().atan() - std::f64::consts::FRAC_PI_2).to_degrees();
    (lon, lat)
}

fn work_out(doc: &Document, camera: &Camera, colors: &Colors) -> Marks {
    let hub = super::hub();
    let view = camera.visible_bounds();
    let box_world = [view.min_x, view.min_y, view.max_x, view.max_y];
    let mut held: Vec<Arc<Vec<Candidate>>> = Vec::new();
    let mut credits: Vec<Credit> = Vec::new();
    // Top first: an upper service's credits and labels before a lower one's.
    for (key, service) in shown(doc).into_iter().rev() {
        let sv = hub.view(&key);
        let seen = sv
            .as_ref()
            .and_then(|sv| in_view(sv, box_world, camera.scale));
        if let Some(c) = hub.attribution(&key) {
            credits.push(c);
        }
        if service.kind == ServiceKind::Google
            && let Some(seen) = &seen
        {
            let [x1, y1, x2, y2] = seen.view.bbox;
            let (west, south) = degrees(x1, y1);
            let (east, north) = degrees(x2, y2);
            let view = CreditView::new(seen.level as u32, south, west, north, east);
            if let Some(text) = hub.google_credit(&key, view) {
                credits.push(Credit {
                    text,
                    links: vec![(
                        "Google Haritalar'ın koşulları".into(),
                        "https://www.google.com/intl/tr_tr/help/terms_maps/".into(),
                    )],
                });
            }
        }
        if let (Some(sv), Some(seen)) = (&sv, &seen)
            && sv.vector
        {
            let zoom = display_zoom(sv, seen.view.units_per_px);
            held.extend(hub.labels(&key, &seen.tiles, zoom));
        }
    }
    let cands: Vec<&Candidate> = held.iter().flat_map(|t| t.iter()).collect();
    let screen = Screen {
        x0: view.min_x,
        y0: view.max_y,
        px_per_unit: camera.scale,
        width: camera.width,
        height: camera.height,
    };
    let mut placed = Vec::new();
    rule::place(&cands, &screen, &mut placed);
    let labels = placed
        .into_iter()
        .filter_map(|p| {
            let c = cands.get(p.candidate)?;
            let s = &c.style;
            let weight = if s.bold { 700 } else { 400 };
            Some(Drawn {
                lines: p.lines,
                glyphs: p.glyphs,
                size: s.size as f32,
                font: crate::drawing_fonts::font(DrawingFont::Arimo, weight, false),
                bold: s.bold,
                color: colors.shown(color(s.color, s.opacity)),
                halo: s
                    .halo
                    .filter(|(c, w)| c[3] > 0.0 && *w > 0.0)
                    .map(|(c, w)| (colors.shown(color(c, s.opacity)), (w as f32).min(3.0))),
            })
        })
        .collect();
    Marks { labels, credits }
}

/// The labels' layer over the drawing area, under the drawing's own text;
/// none while no vector service shows.
pub fn layer<'a>(marks: Rc<Marks>, key: u64) -> Option<Element<'a, Message>> {
    if marks.labels.is_empty() {
        return None;
    }
    Some(
        canvas::Canvas::new(Labels { marks, key })
            .width(Fill)
            .height(Fill)
            .into(),
    )
}

struct Labels {
    marks: Rc<Marks>,
    key: u64,
}

#[derive(Default)]
struct State {
    cache: canvas::Cache,
    key: Cell<u64>,
}

impl canvas::Program<Message> for Labels {
    type State = State;

    fn draw(
        &self,
        state: &State,
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        if state.key.get() != self.key {
            state.cache.clear();
            state.key.set(self.key);
        }
        vec![state.cache.draw(renderer, bounds.size(), |frame| {
            for d in &self.marks.labels {
                paint(frame, d);
            }
        })]
    }
}

/// A label: a point's lines through the glyph cache over eight copies in
/// its halo's colour; a line's letters as outlines turned along it, the
/// halo stroked under them.
fn paint(frame: &mut Frame, d: &Drawn) {
    for line in &d.lines {
        let top = crate::labels::baseline_in(&line.text, d.font, d.size);
        let text = |position: Point, color: Color| canvas::Text {
            content: line.text.clone(),
            position,
            max_width: f32::INFINITY,
            color,
            size: Pixels(d.size),
            line_height: text::LineHeight::Relative(1.0),
            font: d.font,
            align_x: text::Alignment::Left,
            align_y: Vertical::Top,
            shaping: text::Shaping::Advanced,
        };
        let at = Point::new(line.x as f32, line.y as f32 - top);
        if let Some((halo, w)) = d.halo {
            let w = w.min(2.0);
            for i in 0..8 {
                let a = i as f32 * std::f32::consts::FRAC_PI_4;
                frame.fill_text(text(at + Vector::new(a.cos() * w, a.sin() * w), halo));
            }
        }
        frame.fill_text(text(at, d.color));
    }
    let mut buf = [0u8; 4];
    for g in &d.glyphs {
        let ch: &str = g.ch.encode_utf8(&mut buf);
        if ch.trim().is_empty() {
            continue;
        }
        let paths = crate::labels::text_outlines(ch, d.font, d.size);
        let half = rule_width(ch, d.size, d.bold) / 2.0;
        frame.with_save(|frame| {
            frame.translate(Vector::new(g.x as f32, g.y as f32));
            frame.rotate(g.angle as f32);
            frame.translate(Vector::new(-half, d.size * 0.35));
            if let Some((halo, w)) = d.halo {
                let stroke = Stroke {
                    width: w * 2.0,
                    line_join: LineJoin::Round,
                    ..Stroke::default()
                }
                .with_color(halo);
                for p in paths.iter() {
                    frame.stroke(p, stroke);
                }
            }
            for p in paths.iter() {
                frame.fill(p, d.color);
            }
        });
    }
}

/// A letter's width as the placement measured it (the core's Arimo tables).
fn rule_width(ch: &str, size: f32, bold: bool) -> f32 {
    kentos_geometry_core::text::width_em_in(
        ch,
        kentos_geometry_core::text::Font::from_id("arimo"),
        bold,
    ) as f32
        * size
}
