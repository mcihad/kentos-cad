//! What lies under the pointer on the canvas (on the web the browser's hit
//! test found the element: `data-ruler`, `data-handle`, `data-node`,
//! `data-guide`, `data-id`): the rulers on top, then the node tool's nodes
//! and handles or the selection's handles and rotation knob, a guide, and
//! the front-most shape that is shown and not locked. A shape is hit inside
//! its fill (even where it paints none, as `pointer-events: all`) or on its
//! stroke; a thin one is found within three pixels (the web wanted the
//! exact stroke).

use kentos_svg_core::bezier::{flatten_sub_path_tol, winding_of};
use kentos_svg_core::model::{shape_box, shapes_box};
use kentos_svg_core::shape::{Obj, Pt};

use super::ToolId;
use super::camera::ruler;
use super::doc::id_of;
use super::node_tool::Part;
use super::paint::outline;
use super::state::SvgEditor;

/// A ruler: along the top (h) or down the left (v).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    H,
    V,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Hit {
    Ruler(Axis),
    /// A selection handle, 0 top left clockwise to 7 middle left.
    Handle(u8),
    Rot,
    /// A node tool marker: sub-path, index and which part.
    Node(usize, usize, Part),
    Guide(String),
    Shape(String),
    Nothing,
}

/// Pixels round a handle that still take it.
const HANDLE_PX: f64 = 5.0;
/// A thin stroke or a guide is found this near.
const LINE_PX: f64 = 3.0;
const GUIDE_PX: f64 = 4.5;

fn dist_seg(p: Pt, a: Pt, b: Pt) -> f64 {
    let (dx, dy) = (b[0] - a[0], b[1] - a[1]);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p[0] - a[0]) * dx + (p[1] - a[1]) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    ((p[0] - a[0] - t * dx).powi(2) + (p[1] - a[1] - t * dy).powi(2)).sqrt()
}

/// Whether a shape takes a press at `p` (drawing units); `px` is a pixel in drawing units.
pub fn shape_hit(s: &Obj, p: Pt, px: f64) -> bool {
    if s.kind() == "text" {
        // The text's box, unturned about its anchor.
        let turn = s.opt_num("rotate").unwrap_or(0.0);
        let q = if turn != 0.0 {
            let (x, y) = (s.num("x"), s.num("y"));
            let (sin, cos) = (-turn).to_radians().sin_cos();
            let (dx, dy) = (p[0] - x, p[1] - y);
            [x + dx * cos - dy * sin, y + dx * sin + dy * cos]
        } else {
            p
        };
        let mut flat = s.clone();
        flat.remove("rotate");
        return shape_box(&flat).is_ok_and(|b| {
            q[0] >= b.min_x - px && q[0] <= b.max_x + px && q[1] >= b.min_y - px && q[1] <= b.max_y + px
        });
    }
    let Some(subs) = outline(s) else {
        return false;
    };
    let half = (s.num("strokeWidth") / 2.0).max(0.0);
    let reach = half.max(LINE_PX * px);
    let even_odd = match s.text("fillRule") {
        Some("nonzero") => false,
        Some("evenodd") => true,
        _ => s.kind() == "path",
    };
    let mut winding = 0;
    let mut crossings = 0;
    for sp in &subs {
        let ring = flatten_sub_path_tol(sp, px * 0.25);
        if ring.len() < 2 {
            if let Some(a) = ring.first()
                && dist_seg(p, *a, *a) <= reach
            {
                return true;
            }
            continue;
        }
        let n = ring.len();
        let segs = if sp.closed { n } else { n - 1 };
        for i in 0..segs {
            if dist_seg(p, ring[i], ring[(i + 1) % n]) <= reach {
                return true;
            }
        }
        let w = winding_of(&ring, p);
        winding += w;
        crossings += w.abs();
    }
    if even_odd {
        crossings % 2 == 1
    } else {
        winding != 0
    }
}

impl SvgEditor {
    /// The selection's box in screen space, when the Seç tool shows its handles.
    pub fn handle_box(&self) -> Option<([f64; 2], [f64; 2], bool)> {
        if self.tool != ToolId::Select || self.node_edit.is_some() {
            return None;
        }
        let sel = self.chosen();
        if sel.is_empty() {
            return None;
        }
        let b = shapes_box(&sel).ok().flatten()?;
        let a = self.camera.to_screen([b.min_x, b.min_y]);
        let c = self.camera.to_screen([b.max_x, b.max_y]);
        // A locked shape shows its box but no handles.
        Some((a, c, sel.iter().any(|s| s.is("locked"))))
    }

    /// The eight handles' places (screen), 0 top left clockwise.
    pub fn handles(a: [f64; 2], c: [f64; 2]) -> [[f64; 2]; 8] {
        let mx = (a[0] + c[0]) / 2.0;
        let my = (a[1] + c[1]) / 2.0;
        [
            [a[0], a[1]],
            [mx, a[1]],
            [c[0], a[1]],
            [c[0], my],
            [c[0], c[1]],
            [mx, c[1]],
            [a[0], c[1]],
            [a[0], my],
        ]
    }

    /// What is under the screen point `s`.
    pub fn hit_at(&self, s: [f64; 2]) -> Hit {
        let r = ruler();
        if self.options.rulers {
            if s[1] <= r && s[0] > r {
                return Hit::Ruler(Axis::H);
            }
            if s[0] <= r && s[1] > r {
                return Hit::Ruler(Axis::V);
            }
        }
        if self.tool == ToolId::Node
            && self.node_edit.is_some()
            && let Some(h) = self.nodes.marker_at(self, s)
        {
            return h;
        }
        if let Some((a, c, locked)) = self.handle_box()
            && !locked
        {
            let mx = (a[0] + c[0]) / 2.0;
            if (s[0] - mx).hypot(s[1] - (a[1] - 26.0)) <= HANDLE_PX + 1.0 {
                return Hit::Rot;
            }
            for (i, h) in Self::handles(a, c).iter().enumerate() {
                if (s[0] - h[0]).abs() <= HANDLE_PX && (s[1] - h[1]).abs() <= HANDLE_PX {
                    return Hit::Handle(i as u8);
                }
            }
        }
        for g in &self.doc.guides {
            let [x, y] = self.camera.to_screen([g.x, g.y]);
            let (dy, dx) = g.angle.to_radians().sin_cos();
            // Distance from the endless line through (x, y) along (dx, dy).
            let d = ((s[0] - x) * dy - (s[1] - y) * dx).abs();
            if d <= GUIDE_PX {
                return Hit::Guide(g.id.clone());
            }
        }
        match self.shape_at(self.camera.to_doc(s)) {
            Some(id) => Hit::Shape(id),
            None => Hit::Nothing,
        }
    }

    /// The front-most shape shown and not locked at `p` (drawing units).
    pub fn shape_at(&self, p: Pt) -> Option<String> {
        let px = 1.0 / self.camera.zoom;
        self.doc
            .shapes
            .iter()
            .rev()
            .filter(|s| !s.is("hidden") && !s.is("locked"))
            .find(|s| shape_hit(s, p, px))
            .map(|s| id_of(s).to_owned())
    }
}
