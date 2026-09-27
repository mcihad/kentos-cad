//! The SVG editor's drawing tools (the web's `svgDrawTool.ts`): shapes
//! dragged out (rectangle, ellipse, regular polygon or star, text), and the
//! polyline and Bézier pen drafts clicked node by node (the pen's drag gives
//! a node symmetric handles; a click on the first node closes, Enter or a
//! double click ends it).

use iced::widget::canvas::{Frame, Path, Stroke};
use iced::{Color, Point, Size};
use kentos_geometry_core::api::json::Json;
use kentos_svg_core::model::{regular_polygon, rotate_about, transform_shape};
use kentos_svg_core::shape::{Obj, PathNode, Pt, SubPath};

use super::ToolId;
use super::doc::shape_id;
use super::paint::{View, subs_path};
use super::state::SvgEditor;

/// The polyline or pen path being drawn.
#[derive(Clone, Debug, Default)]
pub struct DrawTool {
    pub draft: Vec<PathNode>,
    pub cursor: Option<Pt>,
    /// Shift was held at the last pointer event (square, circle).
    pub shift: bool,
}

impl DrawTool {
    pub fn drafting(&self) -> bool {
        !self.draft.is_empty()
    }

    /// The draft's last node (perpendicular and tangent snaps start there).
    pub fn last(&self) -> Option<Pt> {
        self.draft.last().map(PathNode::pt)
    }

    pub fn cancel(&mut self) -> bool {
        if self.draft.is_empty() {
            return false;
        }
        self.draft.clear();
        self.cursor = None;
        true
    }

    /// Dragging out of the pen's new node gives it symmetric handles.
    pub fn pen_drag(&mut self, q: Pt, zoom: f64) {
        if let Some(n) = self.draft.last_mut()
            && (q[0] - n.x).hypot(q[1] - n.y) > 2.0 / zoom
        {
            // `in` before `out`: the order the core writes nodes in.
            n.in_ = Some([2.0 * n.x - q[0], 2.0 * n.y - q[1]]);
            n.out = Some(q);
        }
    }

    /// The double click's second press added a node on the last one: it goes before the end.
    pub fn drop_last(&mut self) {
        if self.draft.len() > 1 {
            self.draft.pop();
        }
    }
}

/// The name of a shape the tool places, for the history.
fn label(tool: ToolId) -> &'static str {
    match tool {
        ToolId::Rect => "Dikdörtgen",
        ToolId::Ellipse => "Elips",
        ToolId::Polygon => "Çokgen",
        ToolId::Text => "Yazı",
        _ => "Şekil",
    }
}

/// A new shape's fields in the web's order: `{ id, fill, stroke, strokeWidth, kind, … }`.
fn base(width: f64) -> Obj {
    let mut o = Obj::default();
    o.set("id", Json::Str(shape_id()));
    o.set_text("fill", "fill");
    o.set_text("stroke", "none");
    o.set_num("strokeWidth", (width / 50.0).max(1.0));
    o
}

impl SvgEditor {
    /// A click with the polyline or pen tool: a new node, or the end on the first node; true when it closed.
    pub fn draft_click(&mut self, q: Pt) -> bool {
        let zoom = self.camera.zoom;
        if let Some(first) = self.draw.draft.first()
            && self.draw.draft.len() > 2
            && (first.x - q[0]).hypot(first.y - q[1]) < 8.0 / zoom
        {
            self.finish_draft(true);
            return true;
        }
        self.draw.draft.push(PathNode::at(q[0], q[1]));
        false
    }

    /// Ends the polyline or pen path (Enter, a double click, a click on its first node).
    pub fn finish_draft(&mut self, closed: bool) {
        let nodes = std::mem::take(&mut self.draw.draft);
        self.draw.cursor = None;
        if nodes.len() >= 2 {
            let mut s = Obj::default();
            let id = shape_id();
            s.set("id", Json::Str(id.clone()));
            s.set_text("kind", "path");
            s.set_subs(&[SubPath { closed, nodes }]);
            s.set_text("fill", if closed { "fill" } else { "none" });
            s.set_text("stroke", if closed { "none" } else { "fill" });
            s.set_num("strokeWidth", (self.doc.width / 25.0).max(1.0));
            self.begin();
            self.doc.shapes.push(s);
            self.commit(if self.tool == ToolId::Pen {
                "Kalem"
            } else {
                "Çizgi"
            });
            self.select(vec![id]);
            self.snapper.reset();
        }
        self.touch();
    }

    /// The shape a drag from p0 to p1 makes with the current tool (Alt: from the centre).
    pub fn shape_from_drag(&self, p0: Pt, p1: Pt, from_centre: bool) -> Option<Obj> {
        let mut s = base(self.doc.width);
        let tool = self.tool;
        if tool == ToolId::Text {
            s.set_text("kind", "text");
            s.set_num("x", p0[0]);
            s.set_num("y", p0[1]);
            s.set_text("text", "Aa");
            s.set_num("size", self.doc.height / 5.0);
            s.set_num("weight", 700.0);
            s.set_text("font", "sans");
            s.set_text("anchor", "start");
            return Some(s);
        }
        let mut dx = p1[0] - p0[0];
        let mut dy = p1[1] - p0[1];
        if dx.hypot(dy) < 0.5 {
            return None;
        }
        if tool == ToolId::Polygon {
            let r = dx.hypot(dy);
            let sp = regular_polygon(
                p0[0],
                p0[1],
                r,
                self.options.sides.max(3.0),
                self.options.star.then_some(r * 0.45),
            );
            s.set_text("kind", "path");
            s.set_subs(&[sp]);
            // The first corner points at the pointer.
            let turn = dy.atan2(dx) + std::f64::consts::FRAC_PI_2;
            return transform_shape(&s, &rotate_about(turn.to_degrees(), p0[0], p0[1]))
                .ok()
                .flatten();
        }
        if self.draw.shift {
            let k = dx.abs().max(dy.abs());
            dx = if dx == 0.0 { 1.0 } else { dx.signum() } * k;
            dy = if dy == 0.0 { 1.0 } else { dy.signum() } * k;
        }
        let x0 = if from_centre {
            p0[0] - dx.abs()
        } else {
            p0[0].min(p0[0] + dx)
        };
        let y0 = if from_centre {
            p0[1] - dy.abs()
        } else {
            p0[1].min(p0[1] + dy)
        };
        let w = if from_centre { 2.0 * dx.abs() } else { dx.abs() };
        let h = if from_centre { 2.0 * dy.abs() } else { dy.abs() };
        if tool == ToolId::Rect {
            s.set_text("kind", "rect");
            s.set_num("x", x0);
            s.set_num("y", y0);
            s.set_num("w", w);
            s.set_num("h", h);
        } else {
            s.set_text("kind", "ellipse");
            s.set_num("cx", x0 + w / 2.0);
            s.set_num("cy", y0 + h / 2.0);
            s.set_num("rx", w / 2.0);
            s.set_num("ry", h / 2.0);
        }
        Some(s)
    }

    /// A finished drag: the shape goes in as one undo step and is chosen (back to Seç, except for text).
    pub fn place(&mut self, p0: Pt, p1: Pt, from_centre: bool) {
        let Some(s) = self.shape_from_drag(p0, p1, from_centre) else {
            return;
        };
        let id = super::doc::id_of(&s).to_owned();
        self.begin();
        self.doc.shapes.push(s);
        self.commit(label(self.tool));
        self.select(vec![id]);
        self.snapper.reset();
        if self.tool != ToolId::Text {
            self.set_tool(ToolId::Select);
        }
    }
}

/// The draft in screen space: its path, the rubber band to the pointer and its nodes.
pub fn draw_draft(frame: &mut Frame, ed: &SvgEditor, view: &View, accent: Color, panel: Color) {
    let d = &ed.draw;
    if d.draft.is_empty() {
        return;
    }
    let line = Stroke::default().with_color(accent).with_width(1.4);
    frame.stroke(
        &subs_path(
            &[SubPath {
                closed: false,
                nodes: d.draft.clone(),
            }],
            view,
        ),
        line,
    );
    if let (Some(c), Some(last)) = (d.cursor, d.draft.last()) {
        frame.stroke(
            &Path::line(view.point(last.pt()), view.point(c)),
            line,
        );
    }
    for n in &d.draft {
        let s = view.point(n.pt());
        let r = Path::rectangle(Point::new(s.x - 3.0, s.y - 3.0), Size::new(6.0, 6.0));
        frame.fill(&r, panel);
        frame.stroke(&r, Stroke::default().with_color(accent).with_width(1.2));
    }
}
