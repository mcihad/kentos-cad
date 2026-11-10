//! The approved shapes of the objects whose geometry changed since their item's approval, over the drawing
//! (plan/map-cad.md MAP-01, docs/adr/4480). Harita işi's Değişenler selects the changed objects (their shapes now drawn
//! as the selection) and draws each geometry-changed one's shape as approved, dashed in the warning colour, beside it:
//! ArcGIS Pro's change display puts an edited feature's earlier state on the map beside its current one. The shapes are
//! drawn while the selection is the one Değişenler made; any other selection takes them away.
//!
//! An approved object comes as `map.work.query` gives it (its snapshot form as approved); its outline is the geometry
//! core's edges of it, arcs as short chords (a picture's, not a measure), a point a small ring.

use std::cell::Cell;

use iced::widget::canvas::{self, LineDash, Path, Stroke};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Theme, mouse};
use kentos_commands::Entity;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_interaction::Vec2;
use kentos_render_wgpu::Camera;

use crate::app::Message;

/// The approved shapes in the world (polylines and points) and the selection's version they go with.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Approved {
    pub(crate) lines: Vec<Vec<[f64; 2]>>,
    pub(crate) points: Vec<[f64; 2]>,
    pub(crate) objects: usize,
    pub(crate) version: u64,
}

/// An arc's chords: none longer than a thousandth of its radius off it (a picture's tolerance, not a measure's).
fn arc_line(c: kentos_geometry_core::Vec2, r: f64, a0: f64, sweep: f64) -> Vec<[f64; 2]> {
    kentos_geometry_core::tessellate::arc_points(c, r, a0, sweep, (r * 1e-3).max(1e-6)).into_iter().map(|p| [p.x, p.y]).collect()
}

impl Approved {
    /// The outlines of `shapes` (objects as approved), for the selection's version `version`.
    pub(crate) fn of(shapes: &[Entity], version: u64) -> Self {
        let mut out = Approved { objects: shapes.len(), version, ..Approved::default() };
        for e in shapes {
            if let Entity::Point(p) = e {
                out.points.push([p.p.x, p.p.y]);
                continue;
            }
            let shape = kentos_native_application::geometry::shape(e);
            for edge in kentos_geometry_core::ops::edges::entity_edges(&shape) {
                out.lines.push(match edge {
                    Edge::Seg { a, b } => vec![[a.x, a.y], [b.x, b.y]],
                    Edge::Arc { c, r, a0, sweep } => arc_line(c, r, a0, sweep),
                });
            }
        }
        out
    }
}

/// The layer while `approved` goes with the selection's version `version`; none otherwise.
pub(crate) fn layer<'a>(approved: Option<&'a Approved>, version: u64, camera: &Camera, color: Color) -> Option<Element<'a, Message>> {
    let approved = approved.filter(|a| a.version == version && a.objects > 0)?;
    Some(canvas::Canvas::new(Layer { approved, camera: *camera, color }).width(Fill).height(Fill).into())
}

struct Layer<'a> {
    approved: &'a Approved,
    camera: Camera,
    color: Color,
}

/// The picture of a view.
#[derive(Default)]
struct State {
    cache: canvas::Cache,
    key: Cell<u64>,
}

impl Layer<'_> {
    /// What the picture depends on: the shapes, the view and the colour.
    fn key(&self, bounds: Rectangle) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        (self.approved.version, self.approved.objects, self.approved.lines.len()).hash(&mut h);
        for p in [Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)] {
            for c in self.camera.world_to_screen(p) {
                c.to_bits().hash(&mut h);
            }
        }
        for c in [bounds.width, bounds.height, self.color.r, self.color.g, self.color.b, self.color.a] {
            c.to_bits().hash(&mut h);
        }
        h.finish()
    }
}

impl canvas::Program<Message> for Layer<'_> {
    type State = State;

    fn draw(&self, state: &State, renderer: &Renderer, _theme: &Theme, bounds: Rectangle, _cursor: mouse::Cursor) -> Vec<canvas::Geometry> {
        let key = self.key(bounds);
        if state.key.get() != key {
            state.cache.clear();
            state.key.set(key);
        }
        vec![state.cache.draw(renderer, bounds.size(), |frame| {
            let screen = |p: [f64; 2]| {
                let s = self.camera.world_to_screen(Vec2::new(p[0], p[1]));
                Point::new(s[0] as f32, s[1] as f32)
            };
            let dashed = Stroke {
                line_dash: LineDash { segments: &[7.0, 4.0], offset: 0 },
                ..Stroke::default().with_width(1.8).with_color(self.color)
            };
            let lines = Path::new(|b| {
                for line in &self.approved.lines {
                    let mut pts = line.iter().map(|p| screen(*p));
                    if let Some(first) = pts.next() {
                        b.move_to(first);
                        for p in pts {
                            b.line_to(p);
                        }
                    }
                }
            });
            frame.stroke(&lines, dashed);
            for p in &self.approved.points {
                let ring = Path::circle(screen(*p), 6.0);
                frame.stroke(&ring, Stroke::default().with_width(1.8).with_color(self.color));
            }
        })]
    }
}
