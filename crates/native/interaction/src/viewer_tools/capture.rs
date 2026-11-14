//! An extension's map tool (plan/eklenti.md ADIM 19; the manifest's `entrypoints.mapTools`): the user draws the shape
//! the tool asks for and the host gives it to the extension's code ([`ViewChange::Captured`]). Snaps and typed
//! coordinates work as in any command. A point is one click; a line and an area their corners, Enter ending them; a
//! rectangle two corners; an oriented rectangle with a template (MEBİS's 1,10 × 2,20 m grave) its middle and the
//! direction of its long side (Enter: the long side north), without one a side's two ends and the width; a circle its
//! centre and a point on it. Esc gives none. The tool leaves after one shape; the host starts it again for the next
//! (a placement is the extension's run, one undo step).

use crate::Vec2;
use crate::format::Format;
use crate::prompt::Prompt;
use crate::tool::{Context, Flow, Pointer, Preview, Tool, ViewChange};

/// The tool's id, as the traces would read it.
pub const ID: &str = "extensionMapTool";

/// What the map tool asks the user to draw (the manifest's `capture`).
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Kind {
    Point,
    Line,
    Polygon,
    Rectangle,
    /// Its width and length in metres when the tool places a template; none: drawn whole.
    OrientedRectangle(Option<(f64, f64)>),
    Circle,
}

/// The shape drawn, in the project's plane.
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Point(Vec2),
    Line(Vec<Vec2>),
    /// An area's corners in order, the first not repeated: a polygon, a rectangle's four.
    Polygon(Vec<Vec2>),
    Circle {
        center: Vec2,
        radius: f64,
    },
}

/// Draws one shape for an extension's map tool.
#[derive(Clone, Debug)]
pub struct Capture {
    /// The tool's title (“Tekli mezar”), the prompt's name.
    title: String,
    kind: Kind,
    points: Vec<Vec2>,
    hover: Option<Vec2>,
    done: bool,
}

/// An oriented rectangle of `width` × `length` about `middle`, its long side along `towards − middle` (north when
/// they are one point): its corners counter-clockwise.
pub fn template_rectangle(middle: Vec2, towards: Vec2, width: f64, length: f64) -> Vec<Vec2> {
    let (dx, dy) = (towards.x - middle.x, towards.y - middle.y);
    let d = (dx * dx + dy * dy).sqrt();
    let (ux, uy) = if d > 1e-9 {
        (dx / d, dy / d)
    } else {
        (0.0, 1.0)
    };
    // `u` along the long side, `v` across it (u turned a right angle clockwise).
    let (vx, vy) = (uy, -ux);
    let (hl, hw) = (length / 2.0, width / 2.0);
    let at = |a: f64, b: f64| Vec2::new(middle.x + ux * a + vx * b, middle.y + uy * a + vy * b);
    vec![at(-hl, -hw), at(-hl, hw), at(hl, hw), at(hl, -hw)]
}

/// A rectangle drawn whole: `a` and `b` one side's ends, `c` how far across.
fn three_point_rectangle(a: Vec2, b: Vec2, c: Vec2) -> Vec<Vec2> {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let d = (dx * dx + dy * dy).sqrt();
    if d <= 1e-9 {
        return vec![a, b, c];
    }
    // The other side's offset: c's distance from the line a–b, along its normal.
    let (nx, ny) = (-dy / d, dx / d);
    let across = (c.x - a.x) * nx + (c.y - a.y) * ny;
    let (ox, oy) = (nx * across, ny * across);
    vec![
        a,
        b,
        Vec2::new(b.x + ox, b.y + oy),
        Vec2::new(a.x + ox, a.y + oy),
    ]
}

impl Capture {
    pub fn new(title: impl Into<String>, kind: Kind) -> Self {
        Self {
            title: title.into(),
            kind,
            points: Vec::new(),
            hover: None,
            done: false,
        }
    }

    fn finish(&mut self, shape: Option<Shape>, cx: &mut Context<'_>) {
        if !self.done {
            cx.view_changes.push(ViewChange::Captured(shape));
            self.done = true;
        }
    }

    /// The shape once `p` is given as the next point; none while more are needed.
    fn complete_with(&self, p: Vec2) -> Option<Shape> {
        let pts = &self.points;
        match self.kind {
            Kind::Point => Some(Shape::Point(p)),
            Kind::Rectangle => pts
                .first()
                .map(|&a| Shape::Polygon(vec![a, Vec2::new(p.x, a.y), p, Vec2::new(a.x, p.y)])),
            Kind::OrientedRectangle(Some((w, l))) => pts
                .first()
                .map(|&m| Shape::Polygon(template_rectangle(m, p, w, l))),
            Kind::OrientedRectangle(None) => {
                (pts.len() == 2).then(|| Shape::Polygon(three_point_rectangle(pts[0], pts[1], p)))
            }
            Kind::Circle => pts.first().map(|&c| Shape::Circle {
                center: c,
                radius: ((p.x - c.x).powi(2) + (p.y - c.y).powi(2)).sqrt(),
            }),
            Kind::Line | Kind::Polygon => None,
        }
    }

    fn give(&mut self, p: Vec2, cx: &mut Context<'_>) {
        match self.complete_with(p) {
            Some(shape) => self.finish(Some(shape), cx),
            None => self.points.push(p),
        }
    }

    fn step(&self) -> String {
        let n = self.points.len();
        match (self.kind, n) {
            (Kind::Point, _) => "noktayı gösterin".into(),
            (Kind::Line, 0) => "çizginin ilk noktasını gösterin".into(),
            (Kind::Line, _) => "sonraki noktayı gösterin (Enter: bitir)".into(),
            (Kind::Polygon, 0) => "alanın ilk köşesini gösterin".into(),
            (Kind::Polygon, _) => "sonraki köşeyi gösterin (Enter: kapat)".into(),
            (Kind::Rectangle, 0) => "dikdörtgenin bir köşesini gösterin".into(),
            (Kind::Rectangle, _) => "karşı köşeyi gösterin".into(),
            (Kind::OrientedRectangle(Some(_)), 0) => "yerleştirme noktasını (orta) gösterin".into(),
            (Kind::OrientedRectangle(Some(_)), _) => {
                "uzun kenarın yönünü gösterin (Enter: kuzeye)".into()
            }
            (Kind::OrientedRectangle(None), 0) => "bir kenarın başını gösterin".into(),
            (Kind::OrientedRectangle(None), 1) => "kenarın sonunu gösterin".into(),
            (Kind::OrientedRectangle(None), _) => "genişliği gösterin".into(),
            (Kind::Circle, 0) => "dairenin merkezini gösterin".into(),
            (Kind::Circle, _) => "daire üzerinde bir nokta gösterin".into(),
        }
    }
}

impl Tool for Capture {
    fn accepts_points(&self) -> bool {
        true
    }

    fn accept_point(&mut self, p: Vec2, cx: &mut Context<'_>) -> bool {
        self.give(p, cx);
        true
    }

    fn id(&self) -> &'static str {
        ID
    }

    fn label(&self) -> &'static str {
        "Eklenti aracı"
    }

    /// “Tekli mezar: yerleştirme noktasını (orta) gösterin ya da Y,X yazın [Vazgeç (Esc)]”.
    fn prompt(&self) -> Prompt {
        Prompt::untitled(format!("{}: {} ya da Y,X yazın", self.title, self.step()))
            .option("Vazgeç", "Esc")
    }

    fn point_count(&self) -> usize {
        self.points.len()
    }

    fn pointer_move(&mut self, p: &Pointer, _cx: &mut Context<'_>) {
        self.hover = Some(p.world);
    }

    fn pointer_down(&mut self, p: &Pointer, cx: &mut Context<'_>) {
        self.give(p.world, cx);
    }

    fn input(&mut self, text: &str, cx: &mut Context<'_>) -> bool {
        let Some(p) = cx.typed_point(text, self.points.last().copied(), self.hover) else {
            return false;
        };
        self.give(p, cx);
        true
    }

    /// Enter: a line of two points or an area of three ends; a template placed points its long side north; else
    /// nothing is drawn and the tool leaves.
    fn confirm(&mut self, cx: &mut Context<'_>) -> Flow {
        let pts = self.points.clone();
        let shape = match self.kind {
            Kind::Line if pts.len() >= 2 => Some(Shape::Line(pts)),
            Kind::Polygon if pts.len() >= 3 => Some(Shape::Polygon(pts)),
            Kind::OrientedRectangle(Some((w, l))) if pts.len() == 1 => {
                Some(Shape::Polygon(template_rectangle(pts[0], pts[0], w, l)))
            }
            _ => None,
        };
        self.finish(shape, cx);
        Flow::Exit
    }

    /// The last point given back.
    fn undo_step(&mut self, _cx: &mut Context<'_>) -> bool {
        self.points.pop().is_some()
    }

    /// Esc: nothing drawn; the tool leaves.
    fn cancel(&mut self, cx: &mut Context<'_>) -> bool {
        self.finish(None, cx);
        false
    }

    /// The shape as it would be with the cursor's point: the line or ring so far, the template turned to the cursor.
    fn preview(&self, _format: &Format) -> Preview {
        let Some(h) = self.hover else {
            return Preview::default();
        };
        let mut p = Preview::default();
        match self.complete_with(h) {
            Some(Shape::Polygon(ring)) => p.ring = Some(ring),
            Some(Shape::Circle { center, radius }) => {
                p.ring = Some(
                    (0..48)
                        .map(|i| {
                            let a = f64::from(i) / 48.0 * std::f64::consts::TAU;
                            Vec2::new(center.x + radius * a.cos(), center.y + radius * a.sin())
                        })
                        .collect(),
                );
            }
            Some(Shape::Point(_)) => p.marks.push(h),
            Some(Shape::Line(_)) | None => {
                let mut path = self.points.clone();
                path.push(h);
                if self.kind == Kind::Polygon && path.len() >= 3 {
                    p.ring = Some(path);
                } else {
                    p.path = path;
                }
            }
        }
        p
    }

    fn finished(&self) -> bool {
        self.done
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Vec2, b: Vec2) -> bool {
        (a.x - b.x).abs() < 1e-9 && (a.y - b.y).abs() < 1e-9
    }

    #[test]
    fn a_template_turns_its_long_side_to_the_direction_given() {
        // MEBİS's grave: 1,10 × 2,20 m about (10, 20); the long side towards the east, then north by default.
        let east = template_rectangle(Vec2::new(10.0, 20.0), Vec2::new(30.0, 20.0), 1.1, 2.2);
        let xs: Vec<f64> = east.iter().map(|p| p.x).collect();
        let ys: Vec<f64> = east.iter().map(|p| p.y).collect();
        let span = |v: &[f64]| {
            v.iter().cloned().fold(f64::MIN, f64::max) - v.iter().cloned().fold(f64::MAX, f64::min)
        };
        assert!(
            (span(&xs) - 2.2).abs() < 1e-9 && (span(&ys) - 1.1).abs() < 1e-9,
            "{east:?}"
        );
        let north = template_rectangle(Vec2::new(0.0, 0.0), Vec2::new(0.0, 0.0), 1.1, 2.2);
        assert!(
            close(north[0], Vec2::new(-0.55, -1.1)) && close(north[2], Vec2::new(0.55, 1.1)),
            "{north:?}"
        );
    }

    #[test]
    fn each_kind_is_complete_when_its_points_are_given() {
        let c = Capture::new("Deneme", Kind::Rectangle);
        assert_eq!(
            c.complete_with(Vec2::new(1.0, 1.0)),
            None,
            "one corner first"
        );
        let mut c = Capture::new("Deneme", Kind::Rectangle);
        c.points.push(Vec2::new(0.0, 0.0));
        assert_eq!(
            c.complete_with(Vec2::new(2.0, 1.0)),
            Some(Shape::Polygon(vec![
                Vec2::new(0.0, 0.0),
                Vec2::new(2.0, 0.0),
                Vec2::new(2.0, 1.0),
                Vec2::new(0.0, 1.0)
            ]))
        );
        let mut c = Capture::new("Deneme", Kind::OrientedRectangle(None));
        c.points.extend([Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0)]);
        let Some(Shape::Polygon(r)) = c.complete_with(Vec2::new(1.0, 3.0)) else {
            panic!("three points make it");
        };
        assert!(
            close(r[2], Vec2::new(4.0, 3.0)) && close(r[3], Vec2::new(0.0, 3.0)),
            "{r:?}"
        );
        let mut c = Capture::new("Deneme", Kind::Circle);
        c.points.push(Vec2::new(1.0, 1.0));
        assert_eq!(
            c.complete_with(Vec2::new(4.0, 5.0)),
            Some(Shape::Circle {
                center: Vec2::new(1.0, 1.0),
                radius: 5.0
            })
        );
        assert_eq!(
            Capture::new("Deneme", Kind::Point).complete_with(Vec2::new(7.0, 8.0)),
            Some(Shape::Point(Vec2::new(7.0, 8.0)))
        );
        assert_eq!(
            Capture::new("Deneme", Kind::Line).complete_with(Vec2::new(7.0, 8.0)),
            None
        );
    }
}
