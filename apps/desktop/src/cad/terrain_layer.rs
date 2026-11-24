//! A road project's terrain surfaces over the drawing (plan/road.md Y2.4, docs/adr/0900 §12): each surface's triangles
//! as thin lines in the drawing's ink, its breaklines', boundary's and voids' edges (the constrained ones) in the
//! danger colour, under the drawing's text. OpenRoads draws a terrain's triangles and its breaklines by the terrain's
//! template; KentOS's first look is this one, the same for every surface.
//!
//! The surfaces are made from the drawing by `road.terrains`' rule ([`kentos_native_application::road_terrains`]) once
//! for a revision of the drawing; the picture is drawn once for a view and a revision (a canvas cache), its edges cut
//! to the window, those shorter than two pixels left out (a zoomed-out surface would be a mesh of ink).

use std::cell::{Cell, RefCell};

use iced::widget::canvas::{self, Path, Stroke};
use iced::{Color, Element, Fill, Point, Rectangle, Renderer, Theme, mouse};
use kentos_domain::Document;
use kentos_interaction::Vec2;
use kentos_render_wgpu::Camera;

use crate::app::Message;

/// A surface's edges in the world: its triangles' (each once) and its constrained ones.
#[derive(Debug, Default)]
struct Edges {
    plain: Vec<[[f64; 2]; 2]>,
    constrained: Vec<[[f64; 2]; 2]>,
    /// The triangles edits took out (docs/adr/1560): outlined and faintly filled in the danger colour.
    removed: Vec<[[f64; 2]; 3]>,
}

fn edges_of(surface: &kentos_road::surface::Surface) -> Edges {
    let at = |i: u32| {
        let v = surface.vertices[i as usize];
        [v[0], v[1]]
    };
    let constrained: std::collections::BTreeSet<[u32; 2]> = surface.constrained.iter().copied().collect();
    let mut seen = std::collections::BTreeSet::new();
    let mut out = Edges::default();
    for t in &surface.triangles {
        for (a, b) in [(t[0], t[1]), (t[1], t[2]), (t[2], t[0])] {
            let key = if a < b { [a, b] } else { [b, a] };
            if seen.insert(key) && !constrained.contains(&key) {
                out.plain.push([at(a), at(b)]);
            }
        }
    }
    out.constrained = surface.constrained.iter().map(|[a, b]| [at(*a), at(*b)]).collect();
    out.removed = surface.removed_triangles().into_iter().map(|t| t.map(at)).collect();
    out
}

/// The layer: the drawing whose surfaces it draws, and how.
pub(crate) struct Layer<'a> {
    doc: &'a Document,
    camera: Camera,
    ink: Color,
    danger: Color,
}

/// The surfaces' edges of a revision, and the picture of a view.
#[derive(Default)]
pub(crate) struct State {
    built: RefCell<Option<(u64, Vec<Edges>)>>,
    cache: canvas::Cache,
    key: Cell<u64>,
}

/// The layer of a Harita or Yol project with surfaces (docs/adr/1520); none otherwise.
pub(crate) fn layer<'a>(doc: &'a Document, camera: &Camera, ink: Color, danger: Color) -> Option<Element<'a, Message>> {
    let road = doc.settings().road.as_ref()?;
    if road.surfaces.is_empty() || !kentos_native_application::road_terrains::holds_surfaces(doc.settings().cad_work) {
        return None;
    }
    Some(
        canvas::Canvas::new(Layer {
            doc,
            camera: *camera,
            ink,
            danger,
        })
        .width(Fill)
        .height(Fill)
        .into(),
    )
}

impl Layer<'_> {
    /// What the picture depends on: the drawing's revision, the view and the colours.
    fn key(&self, bounds: Rectangle) -> u64 {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        self.doc.revision().hash(&mut h);
        for p in [Vec2::new(0.0, 0.0), Vec2::new(1.0, 0.0), Vec2::new(0.0, 1.0)] {
            for c in self.camera.world_to_screen(p) {
                c.to_bits().hash(&mut h);
            }
        }
        for c in [bounds.width, bounds.height, self.ink.r, self.ink.a, self.danger.r] {
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
        let stale = state.built.borrow().as_ref().is_none_or(|(r, _)| *r != revision);
        if stale {
            let edges = kentos_native_application::road_terrains::made_surfaces(self.doc)
                .into_iter()
                .filter_map(|(_, _, s)| s.as_ref().map(edges_of))
                .collect();
            *state.built.borrow_mut() = Some((revision, edges));
        }
        let key = self.key(bounds);
        if state.key.get() != key {
            state.cache.clear();
            state.key.set(key);
        }
        let built = state.built.borrow();
        let Some((_, surfaces)) = built.as_ref() else {
            return Vec::new();
        };
        vec![state.cache.draw(renderer, bounds.size(), |frame| {
            let (w, h) = (bounds.width as f64, bounds.height as f64);
            let screen = |p: [f64; 2]| self.camera.world_to_screen(Vec2::new(p[0], p[1]));
            let path = |edges: &[[[f64; 2]; 2]], least: f64| {
                Path::new(|b| {
                    for [a, c] in edges {
                        let (p, q) = (screen(*a), screen(*c));
                        let off = (p[0] < 0.0 && q[0] < 0.0)
                            || (p[0] > w && q[0] > w)
                            || (p[1] < 0.0 && q[1] < 0.0)
                            || (p[1] > h && q[1] > h);
                        if off || (q[0] - p[0]).hypot(q[1] - p[1]) < least {
                            continue;
                        }
                        b.move_to(Point::new(p[0] as f32, p[1] as f32));
                        b.line_to(Point::new(q[0] as f32, q[1] as f32));
                    }
                })
            };
            for s in surfaces {
                frame.stroke(
                    &path(&s.plain, 2.0),
                    Stroke::default().with_width(0.75).with_color(Color { a: 0.45, ..self.ink }),
                );
                frame.stroke(
                    &path(&s.constrained, 0.0),
                    Stroke::default().with_width(1.6).with_color(self.danger),
                );
                for t in &s.removed {
                    let [a, b, c] = t.map(screen);
                    let hole = Path::new(|p| {
                        p.move_to(Point::new(a[0] as f32, a[1] as f32));
                        p.line_to(Point::new(b[0] as f32, b[1] as f32));
                        p.line_to(Point::new(c[0] as f32, c[1] as f32));
                        p.close();
                    });
                    frame.fill(&hole, Color { a: 0.14, ..self.danger });
                    frame.stroke(&hole, Stroke::default().with_width(1.0).with_color(Color { a: 0.7, ..self.danger }));
                }
            }
        })]
    }
}
