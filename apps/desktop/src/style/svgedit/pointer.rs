//! What the pointer does on the SVG editor's canvas (the web's `SvgCanvas`
//! pointer handlers): select, move (snapping the selection's corners, centre
//! and nodes), scale by eight handles (Shift keeps the proportions at the
//! corners), turn by the knob above (Shift in 15° steps), a box round shapes,
//! the drawing tools, panning with the middle button or Space, zooming with
//! the wheel about the pointer, and double clicks counted here (a path's
//! nodes open, a draft ends, a guide's window opens).
//!
//! Esc during a move, a scale or a turn puts the shapes back where they
//! were (the web let the drag go on).

use std::time::{Duration, Instant};

use kentos_geometry_core::geometry::Bounds;
use kentos_svg_core::arrange::translate;
use kentos_svg_core::model::{box_to_box, rotate_about, shape_box, shapes_box, transform_shape};
use kentos_svg_core::path::Matrix;
use kentos_svg_core::shape::{Obj, Pt};

use super::ToolId;
use super::doc::{id_of, same};
use super::hit::Hit;
use super::snap::SnapOpts;
use super::stage::{Button, Input};
use super::state::{At, SvgEditor};

/// Screen pixels the pointer travels before a press moves anything.
const DRAG_PX: f64 = 3.0;
/// Two presses this close in time and place are a double click.
const DOUBLE: Duration = Duration::from_millis(450);
const CLICK_SLOP_PX: f64 = 4.0;

#[derive(Clone, Debug)]
pub enum Op {
    Pan {
        x: f64,
        y: f64,
        ox: f64,
        oy: f64,
    },
    Move {
        p0: Pt,
        orig: Vec<Obj>,
        bx: Bounds,
        sources: Vec<Pt>,
        /// A click on one of several chosen shapes chooses only its unit on release.
        narrow: Option<Vec<String>>,
        moved: bool,
    },
    Scale {
        handle: u8,
        p0: Pt,
        orig: Vec<Obj>,
        bx: Bounds,
        moved: bool,
    },
    Rotate {
        p0: Pt,
        orig: Vec<Obj>,
        bx: Bounds,
        moved: bool,
    },
    Marquee {
        p0: Pt,
        p1: Pt,
        add: bool,
    },
    Draw {
        p0: Pt,
        p1: Pt,
    },
    /// The unlocked tracing reference dragged into place (screen start, its start).
    Reference {
        x: f64,
        y: f64,
        rx: f64,
        ry: f64,
    },
}

/// The left press under way (for double clicks).
#[derive(Clone, Debug)]
pub struct Press {
    at: Instant,
    s: [f64; 2],
    hit: Hit,
    travelled: bool,
}

/// The last press that was a click.
#[derive(Clone, Debug)]
pub struct Click {
    at: Instant,
    s: [f64; 2],
}

/// A panel waits for a point on the canvas.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pick {
    /// Dönüştür's rotation centre.
    Rotate,
    /// Dizi's polar centre.
    Polar,
    /// Dizi's mirror line's point.
    Mirror,
}

impl Pick {
    pub fn prompt(self) -> &'static str {
        match self {
            Pick::Rotate => "Döndürme merkezini tuvalde tıklayın (kenetlenir); Esc vazgeçer.",
            Pick::Polar | Pick::Mirror => "Merkezi tuvalde tıklayın (kenetlenir); Esc vazgeçer.",
        }
    }
}

/// Where a moving selection snaps from: its box corners and centre, and the nodes of its paths (at most 64).
fn move_sources(sel: &[Obj], b: &Bounds) -> Vec<Pt> {
    let mut out = vec![
        [b.min_x, b.min_y],
        [b.max_x, b.min_y],
        [b.min_x, b.max_y],
        [b.max_x, b.max_y],
        [(b.min_x + b.max_x) / 2.0, (b.min_y + b.max_y) / 2.0],
    ];
    for s in sel {
        if s.kind() != "path" {
            continue;
        }
        for sp in s.subs().unwrap_or_default() {
            for n in &sp.nodes {
                if out.len() < 69 {
                    out.push([n.x, n.y]);
                }
            }
        }
    }
    out
}

impl SvgEditor {
    /// A panel asks for a point: the next click (snapped) gives it; Esc gives up.
    pub fn pick_point(&mut self, pick: Pick) {
        self.picking = Some(pick);
        self.say(pick.prompt());
        self.touch();
    }

    fn picked(&mut self, pick: Pick, q: Pt) {
        match pick {
            Pick::Rotate => {
                self.ui.transform.point = Some(q);
                self.ui.transform.about = None;
            }
            Pick::Polar => {
                self.ui.array.polar.point = Some(q);
                self.ui.array.polar.at = At::Point;
            }
            Pick::Mirror => {
                self.ui.array.mirror.point = Some(q);
                self.ui.array.mirror.at = At::Point;
            }
        }
        self.marker = Some(q);
        self.status("", false);
    }

    /// The shapes of a group that can be picked (`groupOf`).
    pub fn group_of(&self, id: &str) -> Vec<String> {
        let Some(s) = self.doc.shape(id) else {
            return vec![id.to_owned()];
        };
        if !s.is("group") {
            return vec![id.to_owned()];
        }
        let g = s.get("group").clone();
        self.doc
            .shapes
            .iter()
            .filter(|x| *x.get("group") == g && !x.is("locked"))
            .map(|x| id_of(x).to_owned())
            .collect()
    }

    /// The canvas's input.
    pub fn pointer(&mut self, input: Input) {
        match input {
            Input::Down {
                s,
                button,
                shift,
                space,
            } => self.down(s, button, shift, space),
            Input::Move { s, shift, alt } => self.moved(s, shift, alt),
            Input::Up { alt } => self.up(alt),
            Input::Wheel { s, up } => {
                self.camera.zoom_by(if up { 1.15 } else { 1.0 / 1.15 }, Some(s));
                self.touch();
            }
            Input::Resized(w, h) => {
                self.camera.size = Some((w, h));
                if !self.camera.fitted {
                    self.camera.fitted = true;
                    self.camera.fit(&self.doc, &self.options);
                }
                self.touch();
            }
            Input::Left => {
                if self.measure.hover.take().is_some() {
                    self.touch();
                }
            }
        }
    }

    fn down(&mut self, s: [f64; 2], button: Button, shift: bool, space: bool) {
        self.draw.shift = shift;
        self.snapper.reset();
        let p = self.camera.to_doc(s);
        if button == Button::Middle || (button == Button::Left && space) {
            self.op = Some(Op::Pan {
                x: s[0],
                y: s[1],
                ox: self.camera.ox,
                oy: self.camera.oy,
            });
            return;
        }
        if button != Button::Left {
            return;
        }
        // A press on the canvas closes the guide's window.
        self.rulers.popup = None;
        let hit = self.hit_at(s);
        self.press = Some(Press {
            at: Instant::now(),
            s,
            hit: hit.clone(),
            travelled: false,
        });
        if let Some(pick) = self.picking.take() {
            let q = self.snap(p, &SnapOpts::default());
            self.picked(pick, q);
            self.snapper.hit = None;
            self.touch();
            return;
        }
        if self.ruler_down(p, &hit) {
            return;
        }
        // An unlocked reference is dragged from where no shape lies over it (the web's image).
        if hit == Hit::Nothing
            && let Some(r) = &self.files.reference
            && r.visible
            && !r.spec.locked
            && p[0] >= r.spec.x
            && p[0] <= r.spec.x + r.spec.width
            && p[1] >= r.spec.y
            && p[1] <= r.spec.y + r.spec.height
        {
            self.op = Some(Op::Reference {
                x: s[0],
                y: s[1],
                rx: r.spec.x,
                ry: r.spec.y,
            });
            return;
        }
        let tool = self.tool;
        if tool == ToolId::Measure {
            self.measure_down(p);
            self.touch();
            return;
        }
        if self.node_edit.is_some() && tool == ToolId::Node && self.node_down(shift, p, &hit) {
            self.touch();
            return;
        }
        if tool == ToolId::Select && matches!(hit, Hit::Handle(_) | Hit::Rot) {
            let sel = self.chosen();
            let Some(bx) = shapes_box(&sel).ok().flatten() else {
                return;
            };
            self.begin();
            self.op = Some(match hit {
                Hit::Rot => Op::Rotate {
                    p0: p,
                    orig: sel,
                    bx,
                    moved: false,
                },
                Hit::Handle(h) => Op::Scale {
                    handle: h,
                    p0: p,
                    orig: sel,
                    bx,
                    moved: false,
                },
                _ => return,
            });
            return;
        }
        match tool {
            ToolId::Select | ToolId::Node => {
                let shape = match &hit {
                    Hit::Shape(id) => self.doc.shape(id).filter(|s| !s.is("locked")).cloned(),
                    _ => None,
                };
                if let Some(shape) = shape {
                    let id = id_of(&shape).to_owned();
                    if tool == ToolId::Node && shape.kind() == "path" {
                        self.edit_nodes(Some(id));
                        return;
                    }
                    let members = self.group_of(&id);
                    if shift {
                        let mut next = self.selection.clone();
                        let on = members.iter().all(|m| next.contains(m));
                        for m in &members {
                            if on {
                                next.retain(|x| x != m);
                            } else if !next.contains(m) {
                                next.push(m.clone());
                            }
                        }
                        self.select(next);
                        return;
                    }
                    // A click (no drag) on one of several chosen shapes chooses only it on release (Inkscape's way).
                    let narrow = (self.is_selected(&id) && self.selection.len() > members.len())
                        .then(|| members.clone());
                    if !self.is_selected(&id) {
                        self.select(members);
                    }
                    let sel = self.chosen();
                    if sel.iter().any(|s| s.is("locked")) {
                        return;
                    }
                    let Some(bx) = shapes_box(&sel).ok().flatten() else {
                        return;
                    };
                    self.begin();
                    let sources = move_sources(&sel, &bx);
                    self.op = Some(Op::Move {
                        p0: p,
                        orig: sel,
                        bx,
                        sources,
                        narrow,
                        moved: false,
                    });
                } else {
                    self.op = Some(Op::Marquee {
                        p0: p,
                        p1: p,
                        add: shift,
                    });
                    if !shift {
                        self.select(Vec::new());
                    }
                }
            }
            ToolId::Rect | ToolId::Ellipse | ToolId::Polygon | ToolId::Text => {
                let q = self.snap(p, &SnapOpts::default());
                self.op = Some(Op::Draw { p0: q, p1: q });
            }
            ToolId::Line | ToolId::Pen => {
                let q = self.snap(
                    p,
                    &SnapOpts {
                        from: self.draw.last(),
                        ..SnapOpts::default()
                    },
                );
                if self.draft_click(q) {
                    return;
                }
                self.op = (tool == ToolId::Pen).then_some(Op::Draw { p0: q, p1: q });
            }
            ToolId::Measure => {}
        }
        self.touch();
    }

    /// Whether a move, scale or turn has begun: the pointer has travelled a few pixels since the press.
    fn travelled(&self, p0: Pt, moved: bool, p: Pt) -> bool {
        moved || (p[0] - p0[0]).hypot(p[1] - p0[1]) * self.camera.zoom >= DRAG_PX
    }

    fn moved(&mut self, s: [f64; 2], shift: bool, alt: bool) {
        self.draw.shift = shift;
        if let Some(press) = &mut self.press
            && !press.travelled
            && (s[0] - press.s[0]).hypot(s[1] - press.s[1]) > CLICK_SLOP_PX
        {
            press.travelled = true;
        }
        let p = self.camera.to_doc(s);
        if self.ruler_move(s, p) {
            return;
        }
        if self.tool == ToolId::Measure && self.op.is_none() {
            self.measure_move(p);
            self.touch();
            return;
        }
        if self.node_edit.is_some()
            && self.tool == ToolId::Node
            && self.op.is_none()
            && self.node_move(alt, p)
        {
            return;
        }
        let Some(op) = self.op.clone() else {
            if self.draw.drafting() {
                let q = self.snap(
                    p,
                    &SnapOpts {
                        from: self.draw.last(),
                        ..SnapOpts::default()
                    },
                );
                self.draw.cursor = Some(q);
                self.touch();
            } else if self.picking.is_some() {
                self.snap(p, &SnapOpts::default());
                self.touch();
            }
            return;
        };
        match op {
            Op::Pan { x, y, ox, oy } => {
                self.camera.ox = ox + s[0] - x;
                self.camera.oy = oy + s[1] - y;
                self.touch();
            }
            Op::Reference { x, y, rx, ry } => {
                let k = self.camera.zoom;
                if let Some(r) = &mut self.files.reference {
                    r.spec.x = rx + (s[0] - x) / k;
                    r.spec.y = ry + (s[1] - y) / k;
                }
                self.touch();
            }
            Op::Move {
                p0,
                orig,
                bx,
                sources,
                moved,
                ..
            } => {
                // A click is no move: nothing moves (or snaps to the grid) until the pointer has travelled.
                if !self.travelled(p0, moved, p) {
                    return;
                }
                if let Some(Op::Move { moved, .. }) = &mut self.op {
                    *moved = true;
                }
                let d = [p[0] - p0[0], p[1] - p0[1]];
                let exclude: Vec<String> = orig.iter().map(|s| id_of(s).to_owned()).collect();
                let dd = self.move_snap(&sources, &bx, d, &exclude);
                self.replace(&orig, &translate(dd[0], dd[1]));
            }
            Op::Scale {
                handle: h,
                p0,
                orig,
                bx,
                moved,
            } => {
                if !self.travelled(p0, moved, p) {
                    return;
                }
                if let Some(Op::Scale { moved, .. }) = &mut self.op {
                    *moved = true;
                }
                let exclude: Vec<String> = orig.iter().map(|s| id_of(s).to_owned()).collect();
                let q = self.snap(
                    p,
                    &SnapOpts {
                        exclude,
                        ..SnapOpts::default()
                    },
                );
                let Some(b) = scaled_box(&bx, h, q, shift) else {
                    return;
                };
                self.replace(&orig, &box_to_box(&bx, &b));
            }
            Op::Rotate { p0, orig, bx, moved } => {
                if !self.travelled(p0, moved, p) {
                    return;
                }
                if let Some(Op::Rotate { moved, .. }) = &mut self.op {
                    *moved = true;
                }
                let cx = (bx.min_x + bx.max_x) / 2.0;
                let cy = (bx.min_y + bx.max_y) / 2.0;
                let mut deg = ((p[1] - cy).atan2(p[0] - cx) - (p0[1] - cy).atan2(p0[0] - cx))
                    .to_degrees();
                if shift {
                    deg = kentos_native_style::classify::js_round(deg / 15.0) * 15.0;
                }
                self.replace(&orig, &rotate_about(deg, cx, cy));
                self.say(format!(
                    "Döndürme: {}°",
                    kentos_native_style::classify::js_round(deg)
                ));
            }
            Op::Marquee { p0, add, .. } => {
                self.op = Some(Op::Marquee { p0, p1: p, add });
                self.touch();
            }
            Op::Draw { p0, .. } => {
                let q = self.snap(
                    p,
                    &SnapOpts {
                        from: Some(p0),
                        ..SnapOpts::default()
                    },
                );
                self.op = Some(Op::Draw { p0, p1: q });
                if self.tool == ToolId::Pen {
                    let zoom = self.camera.zoom;
                    self.draw.pen_drag(q, zoom);
                }
                self.touch();
            }
        }
    }

    /// The move's snapped offset: every snap source of the moving shapes is
    /// tried at its new place, the one nearest a target wins; with none near,
    /// the box corner goes to the grid.
    fn move_snap(&mut self, sources: &[Pt], bx: &Bounds, d: Pt, exclude: &[String]) -> Pt {
        let opts = SnapOpts {
            exclude: exclude.to_vec(),
            no_grid: true,
            ..SnapOpts::default()
        };
        let mut best: Option<(Pt, f64, kentos_svg_core::snap::SnapHit)> = None;
        for src in sources {
            let at = [src[0] + d[0], src[1] + d[1]];
            let q = self.snap(at, &opts);
            let Some(hit) = self.snapper.hit.clone() else {
                continue;
            };
            if best.as_ref().is_none_or(|b| hit.d < b.1) {
                best = Some(([q[0] - src[0], q[1] - src[1]], hit.d, hit));
            }
        }
        if let Some((dd, _, hit)) = best {
            self.snapper.hit = Some(hit);
            return dd;
        }
        let corner = self.snap(
            [bx.min_x + d[0], bx.min_y + d[1]],
            &SnapOpts {
                exclude: exclude.to_vec(),
                ..SnapOpts::default()
            },
        );
        [corner[0] - bx.min_x, corner[1] - bx.min_y]
    }

    /// The shapes as they were, under `m`.
    fn replace(&mut self, orig: &[Obj], m: &Matrix) {
        for o in orig {
            let id = id_of(o);
            if let Some(slot) = self.doc.shapes.iter_mut().find(|s| id_of(s) == id) {
                *slot = transform_shape(o, m).ok().flatten().unwrap_or_else(|| o.clone());
            }
        }
        self.touch();
    }

    fn up(&mut self, alt: bool) {
        let press = self.press.take();
        self.release(alt);
        let Some(press) = press.filter(|p| !p.travelled) else {
            self.last_click = None;
            return;
        };
        let double = self.last_click.as_ref().is_some_and(|c| {
            press.at.duration_since(c.at) <= DOUBLE
                && (press.s[0] - c.s[0]).hypot(press.s[1] - c.s[1]) <= CLICK_SLOP_PX
        });
        if double {
            self.last_click = None;
            self.dbl(&press);
        } else {
            self.last_click = Some(Click {
                at: press.at,
                s: press.s,
            });
        }
    }

    fn release(&mut self, alt: bool) {
        let op = self.op.take();
        if self.ruler_up() {
            self.snapper.reset();
            return;
        }
        if self.tool == ToolId::Measure {
            self.measure_up();
            self.touch();
            return;
        }
        if op.is_none() && self.node_edit.is_some() && self.tool == ToolId::Node {
            self.node_up();
            self.snapper.hit = None;
            self.touch();
            return;
        }
        self.snapper.hit = None;
        let Some(op) = op else {
            return;
        };
        match op {
            Op::Move { orig, narrow, .. } => {
                let unchanged = orig
                    .iter()
                    .all(|o| self.doc.shape(id_of(o)).is_none_or(|s| same(s, o)));
                if unchanged {
                    self.commit("");
                    if let Some(n) = narrow {
                        self.select(n);
                    }
                } else {
                    self.commit("Taşı");
                }
            }
            Op::Scale { .. } => self.commit("Boyutlandır"),
            Op::Rotate { .. } => {
                self.commit("Döndür");
                self.status("", false);
            }
            Op::Marquee { p0, p1, add } => {
                let b = Bounds {
                    min_x: p0[0].min(p1[0]),
                    min_y: p0[1].min(p1[1]),
                    max_x: p0[0].max(p1[0]),
                    max_y: p0[1].max(p1[1]),
                };
                if b.max_x - b.min_x > 1e-6 || b.max_y - b.min_y > 1e-6 {
                    let inside: Vec<String> = self
                        .doc
                        .shapes
                        .iter()
                        .filter(|s| !s.is("hidden") && !s.is("locked"))
                        .filter(|s| {
                            shape_box(s).is_ok_and(|sb| {
                                sb.min_x >= b.min_x
                                    && sb.max_x <= b.max_x
                                    && sb.min_y >= b.min_y
                                    && sb.max_y <= b.max_y
                            })
                        })
                        .map(|s| id_of(s).to_owned())
                        .collect();
                    let mut ids = if add { self.selection.clone() } else { Vec::new() };
                    for id in inside {
                        for m in self.group_of(&id) {
                            if !ids.contains(&m) {
                                ids.push(m);
                            }
                        }
                    }
                    self.select(ids);
                }
            }
            Op::Draw { p0, p1 } => {
                if self.tool != ToolId::Pen {
                    self.place(p0, p1, alt);
                }
            }
            Op::Pan { .. } | Op::Reference { .. } => {}
        }
        self.snapper.reset();
        self.touch();
    }

    /// A double click; `press` is the second press (what it pressed, as it was then).
    fn dbl(&mut self, press: &Press) {
        let hit = press.hit.clone();
        if self.ruler_dbl(press.s, &hit) {
            self.touch();
            return;
        }
        if matches!(self.tool, ToolId::Line | ToolId::Pen) {
            self.draw.drop_last();
            self.finish_draft(false);
            return;
        }
        let p = self.camera.to_doc(press.s);
        if self.node_edit.is_some() && self.tool == ToolId::Node && self.node_dbl(p, &hit) {
            return;
        }
        if let Hit::Shape(id) = &hit
            && self
                .doc
                .shape(id)
                .is_some_and(|s| s.kind() == "path" && !s.is("locked"))
        {
            self.select(vec![id.clone()]);
            self.edit_nodes(Some(id.clone()));
            self.say("Düğüm düzenleme: düğüme tık seçer (Shift ekler), boşlukta sürükleyince kutu içindekiler seçilir; parçaya çift tık düğüm ekler, düğüme çift tık köşe/yumuşak yapar. İşlemler sağda, Esc bitirir.");
        }
    }

    /// Esc on the canvas: whatever is half done goes first (`cancel`).
    pub fn cancel(&mut self) -> bool {
        if self.picking.take().is_some() {
            self.say("Nokta seçmekten vazgeçildi.");
            self.snapper.hit = None;
            self.touch();
            return true;
        }
        if let Some(op) = self.op.take() {
            if let Op::Move { orig, .. } | Op::Scale { orig, .. } | Op::Rotate { orig, .. } = &op {
                self.replace(orig, &translate(0.0, 0.0));
                self.commit("");
                self.status("", false);
            }
            self.snapper.hit = None;
            self.touch();
            return true;
        }
        if self.rulers.popup.take().is_some() {
            self.touch();
            return true;
        }
        if self.tool == ToolId::Measure && self.measure_cancel() {
            self.touch();
            return true;
        }
        if self.node_edit.is_some() && self.node_cancel() {
            return true;
        }
        let done = self.draw.cancel();
        if done {
            self.touch();
        }
        done
    }
}

/// The box a scale handle dragged to q gives (Shift keeps a corner's proportions); none when it collapses.
pub fn scaled_box(bx: &Bounds, h: u8, q: Pt, shift: bool) -> Option<Bounds> {
    let mut b = *bx;
    if matches!(h, 0 | 6 | 7) {
        b.min_x = q[0];
    }
    if matches!(h, 2..=4) {
        b.max_x = q[0];
    }
    if matches!(h, 0..=2) {
        b.min_y = q[1];
    }
    if matches!(h, 4..=6) {
        b.max_y = q[1];
    }
    if shift && h % 2 == 0 {
        // Corners keep the proportions.
        let k = ((b.max_x - b.min_x) / (bx.max_x - bx.min_x))
            .max((b.max_y - b.min_y) / (bx.max_y - bx.min_y));
        let w = (bx.max_x - bx.min_x) * k;
        let hh = (bx.max_y - bx.min_y) * k;
        if h == 0 || h == 6 {
            b.min_x = b.max_x - w;
        } else {
            b.max_x = b.min_x + w;
        }
        if h == 0 || h == 2 {
            b.min_y = b.max_y - hh;
        } else {
            b.max_y = b.min_y + hh;
        }
    }
    if (b.max_x - b.min_x).abs() < 1e-6 || (b.max_y - b.min_y).abs() < 1e-6 {
        return None;
    }
    Some(b)
}
