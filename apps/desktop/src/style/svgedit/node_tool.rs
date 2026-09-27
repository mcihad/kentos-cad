//! The node tool of the SVG editor (the web's `svgNodeTool.ts`, Inkscape's,
//! with the CAD corner tools): click a node to choose it (Shift adds), drag
//! a box round nodes, drag the chosen nodes together (snapping the grabbed
//! one), drag a handle (smooth nodes keep them in line, symmetric ones
//! equal, auto ones turn smooth; Alt frees it). A click on a segment chooses
//! its two ends, a double click adds a node there, a double click on a node
//! switches cusp and smooth. In the corner modes a corner under the pointer
//! gets a ring: press on it and pull along a side, the fillet radius or
//! chamfer length follows, and the release applies it.

use iced::widget::canvas::{Frame, LineDash, Path, Stroke};
use iced::{Color, Point, Size};
use kentos_svg_core::bezier::{nearest_on_cubic, segment_count, segment_cubic, split_cubic};
use kentos_svg_core::nodes::{
    Corner, Joined, NodeRef, corner_at, corner_nodes, fillet_radius, move_nodes, node_type_of,
    refresh_auto,
};
use kentos_svg_core::shape::{Obj, PathNode, Pt, SubPath};

use super::doc::id_of;
use super::hit::Hit;
use super::measure::fmt_num;
use super::paint::{View, subs_path};
use super::snap::{Hang, SnapOpts, tag};
use super::state::SvgEditor;

/// Pixels round a segment that still take a click.
const HIT_PX: f64 = 7.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CornerMode {
    Fillet,
    Chamfer,
}

/// A node marker's part.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Node,
    In,
    Out,
}

#[derive(Clone, Debug)]
enum NodeOp {
    Nodes {
        grab: (usize, usize),
        p0: Pt,
        orig: Vec<SubPath>,
        moved: bool,
    },
    Handle {
        at: (usize, usize),
        part: Part,
        p0: Pt,
        orig: Vec<SubPath>,
        moved: bool,
    },
    Box {
        p0: Pt,
        p1: Pt,
        add: bool,
    },
    Corner {
        at: (usize, usize),
        corner: Corner,
        orig: Vec<SubPath>,
        size: f64,
    },
}

#[derive(Clone, Debug)]
pub struct NodeTool {
    op: Option<NodeOp>,
    for_shape: Option<String>,
    chosen: Vec<(usize, usize)>,
    /// Corner mode (fillet or chamfer by dragging).
    pub mode: Option<CornerMode>,
    hover: Option<(usize, usize)>,
    preview: Option<Vec<SubPath>>,
}

impl Default for NodeTool {
    fn default() -> NodeTool {
        NodeTool {
            op: None,
            for_shape: None,
            chosen: Vec::new(),
            mode: None,
            hover: None,
            preview: None,
        }
    }
}

pub fn node_ref((sub, index): (usize, usize)) -> NodeRef {
    NodeRef {
        sub: sub as f64,
        index: index as f64,
    }
}

fn refs(list: &[(usize, usize)]) -> Vec<NodeRef> {
    list.iter().copied().map(node_ref).collect()
}

fn from_ref(r: &NodeRef) -> Option<(usize, usize)> {
    Some((
        kentos_svg_core::nodes::at(r.sub)?,
        kentos_svg_core::nodes::at(r.index)?,
    ))
}

impl NodeTool {
    pub fn busy(&self) -> bool {
        self.op.is_some()
    }

    pub fn set_mode(&mut self, mode: Option<CornerMode>) {
        self.mode = mode;
        self.hover = None;
        self.preview = None;
    }

    /// The node markers' hit (nodes over their handles), screen point `s`.
    pub fn marker_at(&self, ed: &SvgEditor, s: [f64; 2]) -> Option<Hit> {
        let shape = ed.node_shape()?;
        let subs = shape.subs().ok()?;
        let (show_all, handle_of) = handles_shown(&subs, &ed.node_selected());
        let near = |p: Pt, r: f64| {
            let q = ed.camera.to_screen(p);
            (q[0] - s[0]).hypot(q[1] - s[1]) <= r
        };
        for (si, sp) in subs.iter().enumerate() {
            for (ni, n) in sp.nodes.iter().enumerate() {
                if near(n.pt(), 6.5) {
                    return Some(Hit::Node(si, ni, Part::Node));
                }
            }
        }
        for (si, sp) in subs.iter().enumerate() {
            for (ni, n) in sp.nodes.iter().enumerate() {
                for (part, h) in [(Part::In, n.in_), (Part::Out, n.out)] {
                    let Some(h) = h else { continue };
                    if (show_all || handle_of.contains(&(si, ni, part))) && near(h, 5.5) {
                        return Some(Hit::Node(si, ni, part));
                    }
                }
            }
        }
        None
    }
}

/// Which handles show: all of them on small paths, else those of the chosen
/// nodes and of the segments beside them.
fn handles_shown(subs: &[SubPath], chosen: &[(usize, usize)]) -> (bool, Vec<(usize, usize, Part)>) {
    let total: usize = subs.iter().map(|sp| sp.nodes.len()).sum();
    let show_all = total <= 24;
    let mut of = Vec::new();
    if !show_all {
        for &(si, ni) in chosen {
            let Some(sp) = subs.get(si) else { continue };
            let len = sp.nodes.len();
            if len == 0 {
                continue;
            }
            of.push((si, ni, Part::In));
            of.push((si, ni, Part::Out));
            if ni > 0 || sp.closed {
                of.push((si, (ni + len - 1) % len, Part::Out));
            }
            if ni + 1 < len || sp.closed {
                of.push((si, (ni + 1) % len, Part::In));
            }
        }
    }
    (show_all, of)
}

impl SvgEditor {
    /// The path being edited.
    pub fn node_shape(&self) -> Option<&Obj> {
        let s = self.doc.shape(self.node_edit.as_deref()?)?;
        (s.kind() == "path").then_some(s)
    }

    fn node_subs(&self) -> Option<Vec<SubPath>> {
        self.node_shape()?.subs().ok()
    }

    /// The chosen nodes of the edited path (none after switching paths).
    pub fn node_selected(&self) -> Vec<(usize, usize)> {
        let Some(s) = self.node_shape() else {
            return Vec::new();
        };
        if self.nodes.for_shape.as_deref() != Some(id_of(s)) {
            return Vec::new();
        }
        let subs = s.subs().unwrap_or_default();
        self.nodes
            .chosen
            .iter()
            .copied()
            .filter(|&(si, ni)| subs.get(si).is_some_and(|sp| ni < sp.nodes.len()))
            .collect()
    }

    pub fn node_refs(&self) -> Vec<NodeRef> {
        refs(&self.node_selected())
    }

    pub fn set_node_selected(&mut self, list: Vec<(usize, usize)>) {
        self.nodes.for_shape = self.node_shape().map(|s| id_of(s).to_owned());
        self.nodes.chosen = list;
        self.touch();
    }

    fn set_subs_of_edited(&mut self, subs: &[SubPath]) {
        let Some(id) = self.node_edit.clone() else {
            return;
        };
        if let Some(s) = self.doc.shapes.iter_mut().find(|s| id_of(s) == id) {
            s.set_subs(subs);
        }
        self.touch();
    }

    /// Corner mode on or off, with its hint.
    pub fn set_corner_mode(&mut self, mode: Option<CornerMode>) {
        self.nodes.set_mode(mode);
        if let Some(m) = mode {
            self.say(format!(
                "{}: köşe düğümüne basıp bir kenar boyunca çekin, bırakınca uygulanır; yazılı değer sağdaki alandan. Esc bitirir.",
                if m == CornerMode::Fillet {
                    "Köşe yuvarla"
                } else {
                    "Pah kır"
                }
            ));
        }
        self.touch();
    }

    /// Replaces the edited path's sub-paths as one undo step (the node panel's operations).
    pub fn node_apply(&mut self, label: &str, subs: Vec<SubPath>, chosen: Vec<(usize, usize)>) {
        let Some(id) = self.node_edit.clone() else {
            return;
        };
        self.begin();
        self.set_subs_of_edited(&subs);
        // A path with no nodes left goes away.
        if !subs.iter().any(|sp| !sp.nodes.is_empty()) {
            self.doc.shapes.retain(|s| id_of(s) != id);
            self.commit(label);
            self.edit_nodes(None);
            return;
        }
        self.set_node_selected(chosen);
        self.commit(label);
        self.snapper.reset();
    }

    // ── Pointer ──────────────────────────────────────────────────────────

    pub fn node_down(&mut self, shift: bool, p: Pt, hit: &Hit) -> bool {
        let Some(subs) = self.node_subs() else {
            return false;
        };
        let sid = self.node_edit.clone().unwrap_or_default();
        if self.nodes.mode.is_some() {
            let Some(at) = self.corner_near(p) else {
                return true;
            };
            let Some(c) = subs.get(at.0).and_then(|sp| corner_at(sp, at.1)) else {
                return true;
            };
            if !self.node_selected().contains(&at) {
                self.set_node_selected(vec![at]);
            }
            self.nodes.op = Some(NodeOp::Corner {
                at,
                corner: c,
                orig: subs,
                size: 0.0,
            });
            return true;
        }
        if let Hit::Node(si, ni, part) = hit {
            let at = (*si, *ni);
            if *part == Part::Node {
                let sel = self.node_selected();
                let on = sel.contains(&at);
                if shift {
                    let next = if on {
                        sel.into_iter().filter(|r| *r != at).collect()
                    } else {
                        let mut v = sel;
                        v.push(at);
                        v
                    };
                    self.set_node_selected(next);
                    return true;
                }
                if !on {
                    self.set_node_selected(vec![at]);
                }
                self.begin();
                self.nodes.op = Some(NodeOp::Nodes {
                    grab: at,
                    p0: p,
                    orig: subs,
                    moved: false,
                });
            } else {
                self.begin();
                self.nodes.op = Some(NodeOp::Handle {
                    at,
                    part: *part,
                    p0: p,
                    orig: subs,
                    moved: false,
                });
            }
            self.touch();
            return true;
        }
        if let Some((sub, seg, _)) = self.segment_at(p) {
            // A segment: its two ends are chosen (Shift adds them).
            let len = subs[sub].nodes.len();
            let ends = [(sub, seg), (sub, (seg + 1) % len)];
            let next = if shift {
                let mut v = self.node_selected();
                for e in ends {
                    if !v.contains(&e) {
                        v.push(e);
                    }
                }
                v
            } else {
                ends.to_vec()
            };
            self.set_node_selected(next);
            return true;
        }
        if let Hit::Shape(id) = hit
            && *id != sid
            && let Some(other) = self.doc.shape(id)
            && !other.is("locked")
        {
            // Another shape: a path is edited instead; anything else is selected.
            if other.kind() == "path" {
                self.edit_nodes(Some(id.clone()));
            } else {
                let id = id.clone();
                self.edit_nodes(None);
                self.select(vec![id]);
            }
            return true;
        }
        self.nodes.op = Some(NodeOp::Box {
            p0: p,
            p1: p,
            add: shift,
        });
        true
    }

    pub fn node_move(&mut self, alt: bool, p: Pt) -> bool {
        if self.node_shape().is_none() {
            return false;
        }
        let Some(op) = self.nodes.op.clone() else {
            if self.nodes.mode.is_some() {
                let h = self.corner_near(p);
                if h != self.nodes.hover {
                    self.nodes.hover = h;
                    self.touch();
                }
                return true;
            }
            return false;
        };
        let zoom = self.camera.zoom;
        let sid = self.node_edit.clone().unwrap_or_default();
        match op {
            NodeOp::Nodes {
                grab,
                p0,
                orig,
                moved,
            } => {
                let Some(o) = orig
                    .get(grab.0)
                    .and_then(|sp| sp.nodes.get(grab.1))
                    .cloned()
                else {
                    return true;
                };
                if !moved && (p[0] - p0[0]).hypot(p[1] - p0[1]) * zoom < 3.0 {
                    return true;
                }
                if let Some(NodeOp::Nodes { moved, .. }) = &mut self.nodes.op {
                    *moved = true;
                }
                let sel = self.node_refs();
                let q = self.snap(
                    [o.x + p[0] - p0[0], o.y + p[1] - p0[1]],
                    &SnapOpts {
                        nodes: Some((sid, sel.clone())),
                        ..SnapOpts::default()
                    },
                );
                let subs = move_nodes(&orig, &sel, q[0] - o.x, q[1] - o.y);
                self.set_subs_of_edited(&subs);
                true
            }
            NodeOp::Handle {
                at,
                part,
                p0,
                orig,
                moved,
            } => {
                // As with nodes: a click on a handle does not snap it anywhere.
                if !moved && (p[0] - p0[0]).hypot(p[1] - p0[1]) * zoom < 3.0 {
                    return true;
                }
                if let Some(NodeOp::Handle { moved, .. }) = &mut self.nodes.op {
                    *moved = true;
                }
                let q = self.snap(
                    p,
                    &SnapOpts {
                        nodes: Some((sid, vec![node_ref(at)])),
                        no_grid: true,
                        ..SnapOpts::default()
                    },
                );
                let subs = drag_handle(&orig, at, part, q, alt);
                self.set_subs_of_edited(&subs);
                true
            }
            NodeOp::Box { p0, add, .. } => {
                self.nodes.op = Some(NodeOp::Box { p0, p1: p, add });
                self.touch();
                true
            }
            NodeOp::Corner {
                at, corner, orig, ..
            } => {
                let Some(n) = orig.get(at.0).and_then(|sp| sp.nodes.get(at.1)) else {
                    return true;
                };
                let v = [p[0] - n.x, p[1] - n.y];
                let along = (v[0] * corner.back[0] + v[1] * corner.back[1])
                    .max(v[0] * corner.ahead[0] + v[1] * corner.ahead[1]);
                let d = along.min(corner.max).max(0.0);
                let fillet = self.nodes.mode == Some(CornerMode::Fillet);
                let size = nice_round(
                    if fillet { fillet_radius(&corner, d) } else { d },
                    2.0 / zoom,
                );
                let sel = self.node_refs();
                let r = (size > 0.0).then(|| corner_nodes(&orig, &sel, fillet, size));
                self.nodes.preview = match &r {
                    Some(Joined::Done(subs, _)) => Some(subs.clone()),
                    _ => None,
                };
                match &r {
                    Some(Joined::Error(e)) => self.warn(e.clone()),
                    _ => {
                        let n = sel.len();
                        self.say(format!(
                            "{} {}{}",
                            if fillet { "Yarıçap" } else { "Pah" },
                            fmt_num(size, 3),
                            if n > 1 {
                                format!(" ({n} köşe)")
                            } else {
                                String::new()
                            }
                        ));
                    }
                }
                if let Some(NodeOp::Corner { size: s, .. }) = &mut self.nodes.op {
                    *s = size;
                }
                self.touch();
                true
            }
        }
    }

    pub fn node_up(&mut self) -> bool {
        let Some(op) = self.nodes.op.take() else {
            return false;
        };
        match op {
            NodeOp::Nodes { moved, .. } => {
                self.commit(if moved { "Düğümü taşı" } else { "" });
                self.snapper.reset();
            }
            NodeOp::Handle { moved, .. } => {
                self.commit(if moved { "Kolu taşı" } else { "" });
                self.snapper.reset();
            }
            NodeOp::Box { p0, p1, add } => {
                let Some(subs) = self.node_subs() else {
                    return true;
                };
                let (x0, x1) = (p0[0].min(p1[0]), p0[0].max(p1[0]));
                let (y0, y1) = (p0[1].min(p1[1]), p0[1].max(p1[1]));
                let mut inside = Vec::new();
                for (si, sp) in subs.iter().enumerate() {
                    for (ni, n) in sp.nodes.iter().enumerate() {
                        if n.x >= x0 && n.x <= x1 && n.y >= y0 && n.y <= y1 {
                            inside.push((si, ni));
                        }
                    }
                }
                let mut next = if add {
                    self.node_selected()
                } else {
                    Vec::new()
                };
                for r in inside {
                    if !next.contains(&r) {
                        next.push(r);
                    }
                }
                self.set_node_selected(next);
            }
            NodeOp::Corner { orig, size, .. } => {
                self.nodes.preview = None;
                if !(size > 0.0) || self.node_shape().is_none() {
                    self.touch();
                    return true;
                }
                let fillet = self.nodes.mode == Some(CornerMode::Fillet);
                match corner_nodes(&orig, &self.node_refs(), fillet, size) {
                    Joined::Error(e) => {
                        self.warn(e);
                        self.touch();
                    }
                    Joined::Done(subs, chosen) => {
                        let chosen = chosen.iter().filter_map(from_ref).collect();
                        self.node_apply(
                            if fillet { "Köşe yuvarla" } else { "Pah kır" },
                            subs,
                            chosen,
                        );
                        self.say(format!(
                            "{} {} uygulandı. Sonraki köşeye basın ya da Esc.",
                            if fillet { "Yarıçap" } else { "Pah" },
                            fmt_num(size, 3)
                        ));
                    }
                }
            }
        }
        true
    }

    /// Double click: on a node, cusp ↔ smooth; on a segment, a new node there.
    pub fn node_dbl(&mut self, p: Pt, hit: &Hit) -> bool {
        let Some(mut subs) = self.node_subs() else {
            return false;
        };
        if let Hit::Node(si, ni, Part::Node) = hit {
            let (si, ni) = (*si, *ni);
            let Some(sp) = subs.get(si).filter(|sp| ni < sp.nodes.len()) else {
                return false;
            };
            let len = sp.nodes.len();
            let n = sp.nodes[ni].clone();
            let new = if node_type_of(sp, ni).as_deref() == Some("cusp") {
                // Handles along the neighbours' chord, a sixth of it each way.
                let prev = &sp.nodes[(ni + len - 1) % len];
                let next = &sp.nodes[(ni + 1) % len];
                let dx = (next.x - prev.x) / 6.0;
                let dy = (next.y - prev.y) / 6.0;
                PathNode {
                    x: n.x,
                    y: n.y,
                    in_: Some([n.x - dx, n.y - dy]),
                    out: Some([n.x + dx, n.y + dy]),
                    ty: Some("smooth".to_owned()),
                }
            } else {
                PathNode {
                    in_: None,
                    out: None,
                    ty: Some("cusp".to_owned()),
                    ..n
                }
            };
            subs[si].nodes[ni] = new;
            self.node_apply("Düğüm türü", subs, vec![(si, ni)]);
            return true;
        }
        let Some((sub, seg, t)) = self.segment_at(p) else {
            return false;
        };
        let sp = &mut subs[sub];
        let len = sp.nodes.len();
        let bi = (seg + 1) % len;
        let mid = if sp.nodes[seg].out.is_none() && sp.nodes[bi].in_.is_none() {
            let (a, b) = (&sp.nodes[seg], &sp.nodes[bi]);
            PathNode::at(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
        } else {
            let (l, r) = split_cubic(&segment_cubic(sp, seg), t);
            sp.nodes[seg].out = Some([l[1][0], l[1][1]]);
            sp.nodes[bi].in_ = Some([r[2][0], r[2][1]]);
            PathNode {
                x: l[3][0],
                y: l[3][1],
                in_: Some([l[2][0], l[2][1]]),
                out: Some([r[1][0], r[1][1]]),
                ty: None,
            }
        };
        sp.nodes.insert(seg + 1, mid);
        refresh_auto(&mut subs);
        self.node_apply("Düğüm ekle", subs, vec![(sub, seg + 1)]);
        true
    }

    /// Esc: a drag in progress goes back, then the corner mode ends, then the node choice.
    pub fn node_cancel(&mut self) -> bool {
        if let Some(op) = self.nodes.op.take() {
            match op {
                NodeOp::Nodes { orig, .. }
                | NodeOp::Handle { orig, .. }
                | NodeOp::Corner { orig, .. } => {
                    self.set_subs_of_edited(&orig);
                }
                NodeOp::Box { .. } => {}
            }
            self.nodes.preview = None;
            self.commit("");
            return true;
        }
        if self.nodes.mode.is_some() {
            self.nodes.set_mode(None);
            self.status("", false);
            self.touch();
            return true;
        }
        if !self.node_selected().is_empty() {
            self.set_node_selected(Vec::new());
            return true;
        }
        false
    }

    // ── Hits ─────────────────────────────────────────────────────────────

    /// The segment under the pointer and where along it (not at its ends).
    fn segment_at(&self, p: Pt) -> Option<(usize, usize, f64)> {
        let subs = self.node_subs()?;
        let tol = HIT_PX / self.camera.zoom;
        let mut best: Option<(usize, usize, f64, f64)> = None;
        for (si, sp) in subs.iter().enumerate() {
            for i in 0..segment_count(sp) {
                let c = segment_cubic(sp, i);
                let xs = [c[0][0], c[1][0], c[2][0], c[3][0]];
                let ys = [c[0][1], c[1][1], c[2][1], c[3][1]];
                let (min_x, max_x) = (
                    xs.iter().copied().fold(f64::INFINITY, f64::min) - tol,
                    xs.iter().copied().fold(f64::NEG_INFINITY, f64::max) + tol,
                );
                let (min_y, max_y) = (
                    ys.iter().copied().fold(f64::INFINITY, f64::min) - tol,
                    ys.iter().copied().fold(f64::NEG_INFINITY, f64::max) + tol,
                );
                if p[0] < min_x || p[0] > max_x || p[1] < min_y || p[1] > max_y {
                    continue;
                }
                let h = nearest_on_cubic(&c, p);
                if h.d <= tol && best.is_none_or(|b| h.d < b.3) {
                    best = Some((si, i, h.t, h.d));
                }
            }
        }
        let (si, i, t, _) = best?;
        (t > 1e-6 && t < 1.0 - 1e-6).then_some((si, i, t))
    }

    /// The corner node within reach of the pointer (corner modes).
    fn corner_near(&self, p: Pt) -> Option<(usize, usize)> {
        let subs = self.node_subs()?;
        let mut best_d = 12.0 / self.camera.zoom;
        let mut best = None;
        for (si, sp) in subs.iter().enumerate() {
            for (ni, n) in sp.nodes.iter().enumerate() {
                let d = (n.x - p[0]).hypot(n.y - p[1]);
                if d > best_d {
                    continue;
                }
                let Some(c) = corner_at(sp, ni) else { continue };
                if c.angle > std::f64::consts::PI - 1e-3 || c.angle < 1e-3 {
                    continue;
                }
                best_d = d;
                best = Some((si, ni));
            }
        }
        best
    }
}

/// A handle dragged to q, the other one kept as the node's type says.
fn drag_handle(
    orig: &[SubPath],
    at: (usize, usize),
    part: Part,
    q: Pt,
    free: bool,
) -> Vec<SubPath> {
    let mut subs = orig.to_vec();
    let Some(ty) = orig.get(at.0).map(|sp| node_type_of(sp, at.1)) else {
        return subs;
    };
    let Some(n) = subs.get_mut(at.0).and_then(|sp| sp.nodes.get_mut(at.1)) else {
        return subs;
    };
    match part {
        Part::In => n.in_ = Some(q),
        _ => n.out = Some(q),
    }
    let other = if part == Part::In { n.out } else { n.in_ };
    if free {
        n.ty = Some("cusp".to_owned());
        refresh_auto(&mut subs);
        return subs;
    }
    let ty = ty.unwrap_or_default();
    if ty == "auto" {
        n.ty = Some("smooth".to_owned());
    }
    let Some(oh) = other else {
        refresh_auto(&mut subs);
        return subs;
    };
    if ty == "cusp" {
        refresh_auto(&mut subs);
        return subs;
    }
    let dx = q[0] - n.x;
    let dy = q[1] - n.y;
    let l = {
        let l = dx.hypot(dy);
        if l == 0.0 { 1.0 } else { l }
    };
    let len = if ty == "symmetric" {
        l
    } else {
        (oh[0] - n.x).hypot(oh[1] - n.y)
    };
    let mirrored = [n.x - dx / l * len, n.y - dy / l * len];
    match part {
        Part::In => n.out = Some(mirrored),
        _ => n.in_ = Some(mirrored),
    }
    refresh_auto(&mut subs);
    subs
}

/// A size rounded to a 1-2-5 step near `unit` (what a pixel is worth), so dragging gives tidy values.
pub fn nice_round(v: f64, unit: f64) -> f64 {
    let e = 10f64.powf(unit.log10().floor());
    let step = if unit / e < 2.0 {
        e
    } else if unit / e < 5.0 {
        2.0 * e
    } else {
        5.0 * e
    };
    kentos_native_style::classify::js_round(v / step) * step
}

/// Colours of the node tool.
pub struct NodeColors {
    pub accent: Color,
    pub panel: Color,
    pub text: Color,
}

/// The edited path's nodes and handles, the box being dragged, a corner's ring.
pub fn draw_nodes(frame: &mut Frame, ed: &SvgEditor, view: &View, c: &NodeColors) {
    let Some(shape) = ed.node_shape() else {
        return;
    };
    let Ok(subs) = shape.subs() else {
        return;
    };
    if let Some(preview) = &ed.nodes.preview {
        frame.stroke(
            &subs_path(preview, view),
            Stroke {
                line_dash: LineDash {
                    segments: &[5.0, 3.0],
                    offset: 0,
                },
                ..Stroke::default().with_color(c.accent).with_width(1.4)
            },
        );
    }
    let chosen = ed.node_selected();
    let (show_all, handle_of) = handles_shown(&subs, &chosen);
    let hline = Stroke::default()
        .with_color(Color { a: 0.7, ..c.accent })
        .with_width(1.0);
    for (si, sp) in subs.iter().enumerate() {
        for (ni, n) in sp.nodes.iter().enumerate() {
            let p = view.point(n.pt());
            for (part, h) in [(Part::In, n.in_), (Part::Out, n.out)] {
                let Some(h) = h else { continue };
                if !(show_all || handle_of.contains(&(si, ni, part))) {
                    continue;
                }
                let q = view.point(h);
                frame.stroke(&Path::line(p, q), hline);
                let dot = Path::circle(q, 4.0);
                frame.fill(&dot, c.accent);
                frame.stroke(&dot, Stroke::default().with_color(c.panel).with_width(1.0));
            }
        }
    }
    // Nodes over handles: cusp a diamond, smooth and symmetric a square, auto a circle; chosen ones filled.
    let edge = Stroke::default().with_color(c.accent).with_width(1.2);
    for (si, sp) in subs.iter().enumerate() {
        for (ni, n) in sp.nodes.iter().enumerate() {
            let Point { x, y } = view.point(n.pt());
            let on = chosen.contains(&(si, ni));
            let mark = match node_type_of(sp, ni).as_deref() {
                Some("cusp") => Path::new(|b| {
                    b.move_to(Point::new(x, y - 5.5));
                    b.line_to(Point::new(x + 5.5, y));
                    b.line_to(Point::new(x, y + 5.5));
                    b.line_to(Point::new(x - 5.5, y));
                    b.close();
                }),
                Some("auto") => Path::circle(Point::new(x, y), 4.5),
                _ => Path::rectangle(Point::new(x - 4.0, y - 4.0), Size::new(8.0, 8.0)),
            };
            frame.fill(&mark, if on { c.accent } else { c.panel });
            frame.stroke(&mark, edge);
        }
    }
    if let Some(NodeOp::Box { p0, p1, .. }) = &ed.nodes.op {
        let (a, b) = (view.point(*p0), view.point(*p1));
        let r = Path::rectangle(
            Point::new(a.x.min(b.x), a.y.min(b.y)),
            Size::new((a.x - b.x).abs(), (a.y - b.y).abs()),
        );
        frame.fill(
            &r,
            Color {
                a: 0.12,
                ..c.accent
            },
        );
        frame.stroke(&r, Stroke::default().with_color(c.accent).with_width(1.0));
    }
    let ring = match &ed.nodes.op {
        Some(NodeOp::Corner { at, .. }) => Some(*at),
        _ => ed.nodes.hover,
    };
    if let (Some(mode), Some((si, ni))) = (ed.nodes.mode, ring)
        && let Some(n) = subs.get(si).and_then(|sp| sp.nodes.get(ni))
    {
        let p = view.point(n.pt());
        frame.stroke(
            &Path::circle(p, 11.0),
            Stroke::default().with_color(c.accent).with_width(1.6),
        );
        if let Some(NodeOp::Corner { size, .. }) = &ed.nodes.op
            && *size > 0.0
        {
            let text = format!(
                "{}{}",
                if mode == CornerMode::Fillet { "R " } else { "" },
                fmt_num(*size, 3)
            );
            tag(
                frame,
                &text,
                Point::new(p.x + 14.0, p.y - 12.0),
                Hang::Above,
                c.text,
                c.panel,
                false,
            );
        }
    }
}
