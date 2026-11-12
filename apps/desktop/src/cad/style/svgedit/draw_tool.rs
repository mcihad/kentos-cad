//! The SVG editor's drawing tools (the web's `svgDrawTool.ts`): shapes
//! dragged out (rectangle, ellipse, regular polygon or star, text), and the
//! polyline and Bézier pen drafts clicked node by node (the pen's drag gives
//! a node symmetric handles; a click on the first node closes, Enter or a
//! double click ends it).

use iced::widget::canvas::{Frame, Path, Stroke};
use iced::{Color, Point, Size};
use kentos_geometry_core::api::json::Json;
use kentos_geometry_core::jsmath::{PI, atan2, js_hypot, js_max, js_min};
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

/// A new shape's stroke width: a fiftieth of the canvas (a drawn line's a twenty-fifth), at least 1.
pub fn shape_stroke_width(width: f64) -> f64 {
    js_max(1.0, width / 50.0)
}

pub fn line_stroke_width(width: f64) -> f64 {
    js_max(1.0, width / 25.0)
}

/// What a drag needs besides its two points (the web's `DragSpec`).
#[derive(Clone, Debug, PartialEq)]
pub struct DragSpec {
    pub tool: ToolId,
    pub width: f64,
    pub height: f64,
    pub sides: f64,
    pub star: bool,
    /// Shift: a square or a circle.
    pub shift: bool,
    /// Alt: from the centre.
    pub from_centre: bool,
}

/// The shape a drag from p0 to p1 makes (`shapeFromDrag`); none for a drag too short to draw.
/// Fields in the web's order: `{ id, fill, stroke, strokeWidth, kind, … }`.
pub fn shape_from_drag(spec: &DragSpec, p0: Pt, p1: Pt, id: &str) -> Option<Obj> {
    let mut s = Obj::default();
    s.set("id", Json::Str(id.to_owned()));
    s.set_text("fill", "fill");
    s.set_text("stroke", "none");
    s.set_num("strokeWidth", shape_stroke_width(spec.width));
    if spec.tool == ToolId::Text {
        s.set_text("kind", "text");
        s.set_num("x", p0[0]);
        s.set_num("y", p0[1]);
        s.set_text("text", "Aa");
        s.set_num("size", spec.height / 5.0);
        s.set_num("weight", 700.0);
        s.set_text("font", "sans");
        s.set_text("anchor", "start");
        return Some(s);
    }
    let mut dx = p1[0] - p0[0];
    let mut dy = p1[1] - p0[1];
    if js_hypot(dx, dy) < 0.5 {
        return None;
    }
    if spec.tool == ToolId::Polygon {
        let r = js_hypot(dx, dy);
        let sp = regular_polygon(
            p0[0],
            p0[1],
            r,
            js_max(3.0, spec.sides),
            spec.star.then_some(r * 0.45),
        );
        s.set_text("kind", "path");
        s.set_subs(&[sp]);
        // The first corner points at the pointer.
        let turn = atan2(dy, dx) + PI / 2.0;
        return transform_shape(&s, &rotate_about((turn * 180.0) / PI, p0[0], p0[1]))
            .ok()
            .flatten();
    }
    if spec.shift {
        let k = js_max(dx.abs(), dy.abs());
        // `Math.sign(d || 1)`: zero (either sign) and NaN count as 1.
        let sign = |d: f64| {
            if d == 0.0 || d.is_nan() {
                1.0
            } else {
                d.signum()
            }
        };
        dx = sign(dx) * k;
        dy = sign(dy) * k;
    }
    let (x0, y0, w, h) = if spec.from_centre {
        (
            p0[0] - dx.abs(),
            p0[1] - dy.abs(),
            2.0 * dx.abs(),
            2.0 * dy.abs(),
        )
    } else {
        (
            js_min(p0[0], p0[0] + dx),
            js_min(p0[1], p0[1] + dy),
            dx.abs(),
            dy.abs(),
        )
    };
    if spec.tool == ToolId::Rect {
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
            s.set_num("strokeWidth", line_stroke_width(self.doc.width));
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
        if !matches!(
            self.tool,
            ToolId::Rect | ToolId::Ellipse | ToolId::Polygon | ToolId::Text
        ) {
            return None;
        }
        let spec = DragSpec {
            tool: self.tool,
            width: self.doc.width,
            height: self.doc.height,
            sides: self.options.sides,
            star: self.options.star,
            shift: self.draw.shift,
            from_centre,
        };
        shape_from_drag(&spec, p0, p1, &shape_id())
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
        frame.stroke(&Path::line(view.point(last.pt()), view.point(c)), line);
    }
    for n in &d.draft {
        let s = view.point(n.pt());
        let r = Path::rectangle(Point::new(s.x - 3.0, s.y - 3.0), Size::new(6.0, 6.0));
        frame.fill(&r, panel);
        frame.stroke(&r, Stroke::default().with_color(accent).with_width(1.2));
    }
}
