//! The viewer's measuring (docs/adr/8121): Ölç on the strip turns it on; while it is on a click in the scene gives a
//! point (snapped to a body's corner, an edge's middle, an edge or its face: `kentos_model_view::measure`) instead of
//! choosing an object, and the pointer's snap shows where the next click lands. Uzaklık takes two points (the length,
//! the horizontal length and the components along the axes), Açı three (the angle at the second), Alan one (the area
//! of the flat face clicked). The measurements stay drawn over the scene until removed; the card that lists them folds
//! to its head. The scene's camera and section are the viewer's: a point never lands on what a section cuts away.

use std::collections::HashMap;
use std::fmt;

use kentos_model_view::measure::{self, Distance, Face, Snap};
use kentos_scene3d::math::{self, V3};
use kentos_scene3d::{Exaggeration, Viewport};

use crate::scene::Mark;
use crate::viewer::Viewer;

/// How near (logical pixels) the pointer must be to a corner, an edge's middle or an edge for a point to snap to it.
pub const SNAP_PIXELS: f64 = 10.0;

/// What a click measures.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Tool {
    #[default]
    Distance,
    Angle,
    Area,
}

impl fmt::Display for Tool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Tool::Distance => "Uzaklık",
            Tool::Angle => "Açı",
            Tool::Area => "Alan",
        })
    }
}

impl Tool {
    /// The points a measurement of it takes.
    pub fn points(self) -> usize {
        match self {
            Tool::Distance => 2,
            Tool::Angle => 3,
            Tool::Area => 1,
        }
    }

    /// What the next click does, `given` points in.
    pub fn hint(self, given: usize) -> &'static str {
        match (self, given) {
            (Tool::Distance, 0) => "Başlangıç noktasına tıklayın.",
            (Tool::Distance, _) => "Bitiş noktasına tıklayın.",
            (Tool::Angle, 0) => "Birinci kolun ucuna tıklayın.",
            (Tool::Angle, 1) => "Açının köşesine tıklayın.",
            (Tool::Angle, _) => "İkinci kolun ucuna tıklayın.",
            (Tool::Area, _) => "Düz bir yüze tıklayın: bütünü ölçülür.",
        }
    }
}

/// A measurement done.
#[derive(Debug, Clone, PartialEq)]
pub enum Measured {
    Distance { a: V3, b: V3 },
    Angle { a: V3, at: V3, b: V3 },
    /// A flat face: what it is on (the body's name) and where the value is written.
    Area { face: Face, what: String, at: V3 },
}

/// The measuring's state.
#[derive(Debug, Clone, Default)]
pub struct Measuring {
    /// Whether clicks measure (Ölç on the strip).
    pub on: bool,
    pub tool: Tool,
    /// Whether the card shows only its head.
    pub folded: bool,
    /// The points of the measurement under way.
    pub points: Vec<(V3, Snap)>,
    /// Where the pointer's next click would land.
    pub hover: Option<(V3, Snap)>,
    pub done: Vec<Measured>,
    /// The edges of each body met, by its index in the scene; forgotten when the scene is made again.
    edges: HashMap<usize, Vec<[u32; 2]>>,
}

impl Measuring {
    /// The scene was made again: its bodies' indices are new.
    pub(crate) fn forget_edges(&mut self) {
        self.edges.clear();
    }

    /// Another model: what was measured on the one before means nothing here; the tool and the card stay as they were.
    pub(crate) fn start_again(&mut self) {
        self.points.clear();
        self.hover = None;
        self.done.clear();
        self.edges.clear();
    }
}

impl Viewer {
    /// The point under the pointer at (x, y) of a view `width` × `height`: snapped on the body its ray meets (within
    /// what a section keeps), with that body and triangle; none over the sky.
    pub(crate) fn snapped(&mut self, x: f32, y: f32, width: f32, height: f32) -> Option<(V3, Snap, usize, usize)> {
        let vp = Viewport { width: f64::from(width), height: f64::from(height) };
        let camera = self.camera();
        let planes = self.planes();
        let (x, y) = (f64::from(x), f64::from(y));
        let hit = self.scene().pick_cut(&camera, &vp, &Exaggeration::NONE, x, y, &planes)?;
        let scene = self.scene().clone();
        let source = scene.meshes().get(hit.mesh)?;
        let edges = self
            .measuring
            .edges
            .entry(hit.mesh)
            .or_insert_with(|| measure::edges(&source.vertices, &source.triangles, measure::CREASE_DEGREES));
        let ray = camera.ray(x, y, &vp);
        let (point, snap) = measure::snap(hit.point, &source.vertices, edges, ray, (x, y), SNAP_PIXELS, |p| {
            camera.project(p, &vp).map(|(px, py, _)| (px, py))
        });
        Some((point, snap, hit.mesh, hit.triangle))
    }

    /// A click while measuring: a point given; the measurement done when it has its points (an area at once).
    pub(crate) fn measure_click(&mut self, x: f32, y: f32, width: f32, height: f32) {
        let Some((point, snap, mesh, triangle)) = self.snapped(x, y, width, height) else { return };
        let tool = self.measuring.tool;
        if tool == Tool::Area {
            let scene = self.scene().clone();
            let Some(source) = scene.meshes().get(mesh) else { return };
            let Some(face) = measure::face(&source.vertices, &source.triangles, triangle) else { return };
            let at = centre(&face).unwrap_or(point);
            self.measuring.done.push(Measured::Area { face, what: source.name.clone(), at });
            return;
        }
        let m = &mut self.measuring;
        m.points.push((point, snap));
        if m.points.len() == tool.points() {
            let p: Vec<V3> = m.points.drain(..).map(|(p, _)| p).collect();
            m.done.push(match tool {
                Tool::Angle => Measured::Angle { a: p[0], at: p[1], b: p[2] },
                _ => Measured::Distance { a: p[0], b: p[1] },
            });
        }
    }

    /// The pointer moved over the scene (none: it left it): where its click would land.
    pub(crate) fn measure_hover(&mut self, at: Option<(f32, f32)>, width: f32, height: f32) {
        self.measuring.hover = at.and_then(|(x, y)| self.snapped(x, y, width, height)).map(|(p, s, _, _)| (p, s));
    }

    /// What the scene draws of the measurements: each one's lines, points and value (`write` puts a length, an area or
    /// an angle into words), the one under way and the pointer's snap, with the line from the last point to it.
    pub fn marks(&self, write: &dyn Fn(Value) -> String) -> Vec<Mark> {
        let m = &self.measuring;
        let mut out = Vec::new();
        for d in &m.done {
            match d {
                Measured::Distance { a, b } => {
                    let dist = Distance::between(*a, *b);
                    // The components along the axes, faint, when the line is not along one.
                    let corner = [b[0], b[1], a[2]];
                    if dist.delta[2].abs() > 1e-9 && dist.horizontal > 1e-9 {
                        out.push(Mark::Line { a: *a, b: corner, faint: true });
                        out.push(Mark::Line { a: corner, b: *b, faint: true });
                    }
                    out.push(Mark::Line { a: *a, b: *b, faint: false });
                    out.extend([*a, *b].map(|at| Mark::Point { at, snap: None }));
                    out.push(Mark::Value { at: middle(*a, *b), text: write(Value::Length(dist.length)) });
                }
                Measured::Angle { a, at, b } => {
                    out.push(Mark::Line { a: *at, b: *a, faint: false });
                    out.push(Mark::Line { a: *at, b: *b, faint: false });
                    out.extend([*a, *at, *b].map(|p| Mark::Point { at: p, snap: None }));
                    if let Some(deg) = measure::angle(*a, *at, *b) {
                        out.push(Mark::Value { at: inside(*a, *at, *b), text: write(Value::Angle(deg)) });
                    }
                }
                Measured::Area { face, at, .. } => {
                    out.extend(face.outline.iter().map(|[p, q]| Mark::Line { a: *p, b: *q, faint: false }));
                    out.push(Mark::Value { at: *at, text: write(Value::Area(face.area)) });
                }
            }
        }
        for (p, _) in &m.points {
            out.push(Mark::Point { at: *p, snap: None });
        }
        for w in m.points.windows(2) {
            out.push(Mark::Line { a: w[0].0, b: w[1].0, faint: false });
        }
        if let Some((p, snap)) = m.hover.filter(|_| m.on) {
            if let Some((last, _)) = m.points.last() {
                out.push(Mark::Line { a: *last, b: p, faint: true });
                if m.tool == Tool::Distance {
                    out.push(Mark::Value { at: middle(*last, p), text: write(Value::Length(math::length(math::sub(p, *last)))) });
                }
            }
            out.push(Mark::Point { at: p, snap: Some(snap) });
        }
        out
    }
}

/// A measured value to be written.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Value {
    /// Metres.
    Length(f64),
    /// Square metres.
    Area(f64),
    /// Degrees.
    Angle(f64),
}

fn middle(a: V3, b: V3) -> V3 {
    math::scale(math::add(a, b), 0.5)
}

/// Where an angle's value is written: inside it, along its bisector, a quarter of its shorter arm from its vertex.
fn inside(a: V3, at: V3, b: V3) -> V3 {
    let (u, v) = (math::sub(a, at), math::sub(b, at));
    let (lu, lv) = (math::length(u), math::length(v));
    if lu <= 1e-12 || lv <= 1e-12 {
        return at;
    }
    let bisector = math::add(math::scale(u, 1.0 / lu), math::scale(v, 1.0 / lv));
    let lb = math::length(bisector);
    if lb <= 1e-12 {
        return at;
    }
    math::add(at, math::scale(bisector, 0.25 * lu.min(lv) / lb))
}

/// Where a face's area is written: its outline's middle (the mean of its segments' ends), on its plane.
fn centre(face: &Face) -> Option<V3> {
    let n = face.outline.len();
    (n > 0).then(|| {
        let sum = face.outline.iter().fold([0.0; 3], |acc, [p, q]| math::add(acc, math::add(*p, *q)));
        math::scale(sum, 0.5 / n as f64)
    })
}
