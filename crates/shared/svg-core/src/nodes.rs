//! Node editing of a path (`apps/web/src/style/svg/nodeOps.ts`; Inkscape's
//! node tool, and the CAD corner tools): node types (cusp, smooth,
//! symmetric, auto), new nodes in the middle of segments, deleting nodes
//! while keeping the shape, joining and breaking, deleting segments,
//! segments to lines or curves, fillet and chamfer of a corner, aligning
//! and distributing nodes. A node is addressed by its sub-path and index;
//! every function returns new sub-paths and leaves its input alone.

use kentos_geometry_core::jsmath::{
    PI, acos, js_cmp, js_hypot, js_max, js_min, or, sin, stable_sort, tan,
};

use crate::bezier::{
    Cubic, bez, bez_tangent, param_at_distance, segment_count, segment_cubic, segment_is_line,
    split_cubic,
};
use crate::fit::fit_one;
use crate::shape::{PathNode, Pt, SubPath};

/// A node: its sub-path and index (numbers, as the page keeps them).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NodeRef {
    pub sub: f64,
    pub index: f64,
}

kentos_geometry_core::json_struct!(NodeRef { sub, index });

/// A number as an array index (`list[x]` reads nothing for anything else).
pub fn at(x: f64) -> Option<usize> {
    (x >= 0.0 && x.fract() == 0.0 && x < 9.007_199_254_740_992e15).then_some(x as usize)
}

/// `refKey(a) === refKey(b)`: the same node.
fn same_ref(a: NodeRef, b: NodeRef) -> bool {
    a.sub == b.sub && a.index == b.index
}

fn has_ref(refs: &[NodeRef], sub: usize, index: usize) -> bool {
    refs.iter().any(|r| {
        same_ref(
            *r,
            NodeRef {
                sub: sub as f64,
                index: index as f64,
            },
        )
    })
}

fn node_at(subs: &[SubPath], r: NodeRef) -> Option<&PathNode> {
    subs.get(at(r.sub)?)?.nodes.get(at(r.index)?)
}

fn node_at_mut(subs: &mut [SubPath], r: NodeRef) -> Option<&mut PathNode> {
    subs.get_mut(at(r.sub)?)?.nodes.get_mut(at(r.index)?)
}

fn len(x: f64, y: f64) -> f64 {
    js_hypot(x, y)
}

/// Neighbours of node i along its sub-path (None past an open end).
fn neighbours(sp: &SubPath, i: usize) -> (Option<usize>, Option<usize>) {
    let n = sp.nodes.len();
    let prev = if i > 0 {
        Some(i - 1)
    } else if sp.closed && n > 1 {
        Some(n - 1)
    } else {
        None
    };
    let next = if i + 1 < n {
        Some(i + 1)
    } else if sp.closed && n > 1 {
        Some(0)
    } else {
        None
    };
    (prev, next)
}

/// The node's type: stored, or read from its handles (in line → smooth, in line and equal → symmetric).
pub fn node_type_of(sp: &SubPath, i: usize) -> Option<String> {
    let node = sp.nodes.get(i)?;
    if let Some(t) = node.ty.as_ref().filter(|t| !t.is_empty()) {
        return Some(t.clone());
    }
    let cusp = Some("cusp".to_string());
    let (Some(hin), Some(hout)) = (node.in_, node.out) else {
        return cusp;
    };
    let a: Pt = [node.x - hin[0], node.y - hin[1]];
    let b: Pt = [hout[0] - node.x, hout[1] - node.y];
    let la = len(a[0], a[1]);
    let lb = len(b[0], b[1]);
    if la < 1e-12 || lb < 1e-12 {
        return cusp;
    }
    let cross = (a[0] * b[1] - a[1] * b[0]) / (la * lb);
    let dot = (a[0] * b[0] + a[1] * b[1]) / (la * lb);
    if cross.abs() > 0.01 || dot < 0.0 {
        return cusp;
    }
    Some(
        if (la - lb).abs() < 1e-6 * js_max(js_max(la, lb), 1.0) {
            "symmetric"
        } else {
            "smooth"
        }
        .to_string(),
    )
}

/// Auto-smooth handles: along the neighbours' chord, a third of the way to each.
pub fn auto_handles(sp: &SubPath, i: usize) -> (Option<Pt>, Option<Pt>) {
    let node = &sp.nodes[i];
    let (prev, next) = neighbours(sp, i);
    let (prev, next) = (prev.map(|k| &sp.nodes[k]), next.map(|k| &sp.nodes[k]));
    if prev.is_none() && next.is_none() {
        return (None, None);
    }
    let ax = prev.map_or(node.x, |p| p.x);
    let ay = prev.map_or(node.y, |p| p.y);
    let bx = next.map_or(node.x, |p| p.x);
    let by = next.map_or(node.y, |p| p.y);
    let d = len(bx - ax, by - ay);
    if d < 1e-12 {
        return (None, None);
    }
    let ux = (bx - ax) / d;
    let uy = (by - ay) / d;
    let li = prev.map_or(0.0, |p| len(node.x - p.x, node.y - p.y) / 3.0);
    let lo = next.map_or(0.0, |p| len(p.x - node.x, p.y - node.y) / 3.0);
    (
        prev.map(|_| [node.x - ux * li, node.y - uy * li]),
        next.map(|_| [node.x + ux * lo, node.y + uy * lo]),
    )
}

/// Recomputes the handles of auto nodes (after nodes moved).
pub fn refresh_auto(subs: &mut [SubPath]) {
    for sp in subs.iter_mut() {
        for i in 0..sp.nodes.len() {
            if sp.nodes[i].ty.as_deref() != Some("auto") {
                continue;
            }
            let (hin, hout) = auto_handles(sp, i);
            sp.nodes[i].in_ = hin;
            sp.nodes[i].out = hout;
        }
    }
}

/// Sets the type of the chosen nodes, making their handles fit it.
pub fn set_node_type(subs: &[SubPath], refs: &[NodeRef], ty: &str) -> Vec<SubPath> {
    let mut out = subs.to_vec();
    for &r in refs {
        let (Some(s), Some(i)) = (at(r.sub), at(r.index)) else {
            continue;
        };
        let Some(sp) = out.get_mut(s) else { continue };
        if i >= sp.nodes.len() {
            continue;
        }
        sp.nodes[i].ty = Some(ty.to_string());
        if ty == "cusp" {
            continue;
        }
        if ty == "auto" || (sp.nodes[i].in_.is_none() && sp.nodes[i].out.is_none()) {
            let (hin, hout) = auto_handles(sp, i);
            sp.nodes[i].in_ = hin;
            sp.nodes[i].out = hout;
            continue;
        }
        let (prev, next) = neighbours(sp, i);
        let (prev, next) = (
            prev.map(|k| sp.nodes[k].pt()),
            next.map(|k| sp.nodes[k].pt()),
        );
        let node = &mut sp.nodes[i];
        // A missing handle is made opposite the other one, a third of its segment long.
        let in_v: Pt = node
            .in_
            .map_or([0.0, 0.0], |h| [h[0] - node.x, h[1] - node.y]);
        let out_v: Pt = node
            .out
            .map_or([0.0, 0.0], |h| [h[0] - node.x, h[1] - node.y]);
        let mut li = len(in_v[0], in_v[1]);
        let mut lo = len(out_v[0], out_v[1]);
        if node.in_.is_none()
            && let Some(p) = prev
        {
            li = len(node.x - p[0], node.y - p[1]) / 3.0;
        }
        if node.out.is_none()
            && let Some(q) = next
        {
            lo = len(q[0] - node.x, q[1] - node.y) / 3.0;
        }
        // Direction: along out − in (both handles' average direction).
        let mut dx = out_v[0] - in_v[0];
        let mut dy = out_v[1] - in_v[1];
        if len(dx, dy) < 1e-12 {
            dx = next.map_or(node.x, |q| q[0]) - prev.map_or(node.x, |p| p[0]);
            dy = next.map_or(node.y, |q| q[1]) - prev.map_or(node.y, |p| p[1]);
        }
        let d = or(len(dx, dy), 1.0);
        let ux = dx / d;
        let uy = dy / d;
        if ty == "symmetric" {
            li = (li + lo) / 2.0;
            lo = li;
        }
        if prev.is_some() || node.in_.is_some() {
            node.in_ = Some([node.x - ux * li, node.y - uy * li]);
        }
        if next.is_some() || node.out.is_some() {
            node.out = Some([node.x + ux * lo, node.y + uy * lo]);
        }
    }
    refresh_auto(&mut out);
    out
}

fn shift_node(n: &mut PathNode, dx: f64, dy: f64) {
    n.x += dx;
    n.y += dy;
    if let Some(h) = n.in_ {
        n.in_ = Some([h[0] + dx, h[1] + dy]);
    }
    if let Some(h) = n.out {
        n.out = Some([h[0] + dx, h[1] + dy]);
    }
}

/// Moves the chosen nodes (their handles with them); auto nodes follow.
pub fn move_nodes(subs: &[SubPath], refs: &[NodeRef], dx: f64, dy: f64) -> Vec<SubPath> {
    let mut out = subs.to_vec();
    for &r in refs {
        if let Some(n) = node_at_mut(&mut out, r) {
            shift_node(n, dx, dy);
        }
    }
    refresh_auto(&mut out);
    out
}

/// Segments whose both ends are chosen: (sub, segment index).
fn chosen_segments(subs: &[SubPath], refs: &[NodeRef]) -> Vec<(usize, usize)> {
    let mut out = Vec::new();
    for (s, sp) in subs.iter().enumerate() {
        let n = segment_count(sp);
        for i in 0..n {
            if has_ref(refs, s, i) && has_ref(refs, s, (i + 1) % sp.nodes.len()) {
                out.push((s, i));
            }
        }
    }
    out
}

/// A new node in the middle (t = ½) of every segment between two chosen nodes; the new nodes join the choice.
pub fn insert_mid_nodes(subs: &[SubPath], refs: &[NodeRef]) -> (Vec<SubPath>, Vec<NodeRef>) {
    let mut segs = chosen_segments(subs, refs);
    let mut out = subs.to_vec();
    let mut added: Vec<(usize, Vec<usize>)> = Vec::new();
    // From the last segment back, so earlier indices stay valid.
    stable_sort(&mut segs, &mut |a, b| b.0.cmp(&a.0).then(b.1.cmp(&a.1)));
    for (s, i) in segs {
        let sp = &mut out[s];
        let b_idx = (i + 1) % sp.nodes.len();
        let mid = if sp.nodes[i].out.is_none() && sp.nodes[b_idx].in_.is_none() {
            let (a, b) = (&sp.nodes[i], &sp.nodes[b_idx]);
            PathNode::at((a.x + b.x) / 2.0, (a.y + b.y) / 2.0)
        } else {
            let (l, r) = split_cubic(&segment_cubic(sp, i), 0.5);
            sp.nodes[i].out = Some([l[1][0], l[1][1]]);
            sp.nodes[b_idx].in_ = Some([r[2][0], r[2][1]]);
            PathNode {
                in_: Some([l[2][0], l[2][1]]),
                out: Some([r[1][0], r[1][1]]),
                ..PathNode::at(l[3][0], l[3][1])
            }
        };
        sp.nodes.insert(i + 1, mid);
        match added.iter_mut().find(|(k, _)| *k == s) {
            Some((_, list)) => list.push(i),
            None => added.push((s, vec![i])),
        }
    }
    // The choice: old nodes shifted past inserted ones, and the new ones.
    let mut next = Vec::new();
    for s in 0..out.len() {
        let mut ins: Vec<usize> = added
            .iter()
            .find(|(k, _)| *k == s)
            .map(|(_, l)| l.clone())
            .unwrap_or_default();
        ins.sort_unstable();
        for r in refs {
            if r.sub == s as f64 {
                let shift = ins.iter().filter(|&&k| (k as f64) < r.index).count() as f64;
                next.push(NodeRef {
                    sub: s as f64,
                    index: r.index + shift,
                });
            }
        }
        for (j, &k) in ins.iter().enumerate() {
            next.push(NodeRef {
                sub: s as f64,
                index: (k + 1 + j) as f64,
            });
        }
    }
    (out, next)
}

/// Deletes the chosen nodes. With `keep_shape` the curve over each removed
/// run is replaced by one cubic fitted to it (end tangents kept), as
/// Inkscape does; otherwise the neighbours are joined as they are. A
/// sub-path left with too few nodes goes away.
pub fn delete_nodes(subs: &[SubPath], refs: &[NodeRef], keep_shape: bool) -> Vec<SubPath> {
    let mut out = Vec::new();
    for (s, sp) in subs.iter().enumerate() {
        let n = sp.nodes.len();
        let dead: Vec<bool> = (0..n).map(|i| has_ref(refs, s, i)).collect();
        if !dead.iter().any(|&d| d) {
            out.push(sp.clone());
            continue;
        }
        let mut keep: Vec<Option<PathNode>> = sp
            .nodes
            .iter()
            .zip(&dead)
            .map(|(x, &d)| (!d).then(|| x.clone()))
            .collect();
        if keep.iter().flatten().count() < 2 {
            continue;
        }
        if keep_shape {
            // Each run of deleted nodes between two kept ones: one fitted cubic over the old curve.
            for i in 0..n {
                if dead[i] || (!sp.closed && i == n - 1) {
                    continue;
                }
                if !dead[(i + 1) % n] {
                    continue;
                }
                let mut j = (i + 1) % n;
                while dead[j] && (sp.closed || j < n - 1) {
                    j = (j + 1) % n;
                }
                if dead[j] {
                    continue; // an open end was deleted: the run just goes
                }
                let mut pts: Vec<Pt> = Vec::new();
                let mut all_lines = true;
                let mut k = i;
                while k != j {
                    let c = segment_cubic(sp, k);
                    if !segment_is_line(sp, k) {
                        all_lines = false;
                    }
                    for t in (if k == i { 0 } else { 1 })..=16 {
                        pts.push(bez(&c, t as f64 / 16.0));
                    }
                    k = (k + 1) % n;
                }
                if all_lines {
                    if let Some(x) = keep[i].as_mut() {
                        x.out = None;
                    }
                    if let Some(x) = keep[j].as_mut() {
                        x.in_ = None;
                    }
                    continue;
                }
                let t1 = bez_tangent(&segment_cubic(sp, i), 0.0);
                let t2 = bez_tangent(&segment_cubic(sp, (j + n - 1) % n), 1.0);
                if let Some(c) = fit_one(&pts, t1, [-t2[0], -t2[1]]) {
                    if let Some(x) = keep[i].as_mut() {
                        x.out = Some([c[1][0], c[1][1]]);
                    }
                    if let Some(x) = keep[j].as_mut() {
                        x.in_ = Some([c[2][0], c[2][1]]);
                    }
                }
            }
        }
        let mut nodes: Vec<PathNode> = keep.into_iter().flatten().collect();
        if !sp.closed {
            // A deleted end leaves its neighbour as the end, without a handle past it.
            nodes[0].in_ = None;
            let last = nodes.len() - 1;
            nodes[last].out = None;
        }
        out.push(SubPath {
            closed: sp.closed && nodes.len() > 2,
            nodes,
        });
    }
    refresh_auto(&mut out);
    out
}

fn is_end(sp: &SubPath, i: f64) -> bool {
    !sp.closed && (i == 0.0 || i == sp.nodes.len() as f64 - 1.0)
}

/// The sub-path turned so the given end is its last node (`at_end`) or its first.
fn orient(sp: &SubPath, i: f64, at_end: bool) -> SubPath {
    let last = i == sp.nodes.len() as f64 - 1.0;
    if last == at_end {
        return sp.clone();
    }
    SubPath {
        closed: false,
        nodes: sp
            .nodes
            .iter()
            .rev()
            .map(|n| PathNode {
                in_: n.out,
                out: n.in_,
                ..n.clone()
            })
            .collect(),
    }
}

/// Joined sub-paths and the node to choose, or a message.
pub enum Joined {
    Done(Vec<SubPath>, Vec<NodeRef>),
    Error(String),
}

/// Joins two chosen end nodes: into one node at their middle (`merge`) or
/// with a straight segment between them. Ends of one sub-path close it;
/// ends of two sub-paths make one.
pub fn join_ends(subs: &[SubPath], refs: &[NodeRef], merge: bool) -> Joined {
    let ends: Vec<NodeRef> = refs
        .iter()
        .copied()
        .filter(|r| {
            at(r.sub)
                .and_then(|s| subs.get(s))
                .is_some_and(|sp| is_end(sp, r.index))
        })
        .collect();
    if ends.len() != 2 {
        return Joined::Error(
            "Birleştirmek için açık yolların iki uç düğümünü seçin (Shift ile ikincisini ekleyin)."
                .into(),
        );
    }
    let (a, b) = (ends[0], ends[1]);
    let (Some(sa), Some(sb)) = (at(a.sub), at(b.sub)) else {
        return Joined::Error("Aynı düğüm iki kez seçilmiş.".into());
    };
    let mut out = subs.to_vec();
    let mid = |p: &PathNode, q: &PathNode| ((p.x + q.x) / 2.0, (p.y + q.y) / 2.0);
    if sa == sb {
        let sp = &mut out[sa];
        if sp.nodes.len() < 2 || a.index == b.index {
            return Joined::Error("Aynı düğüm iki kez seçilmiş.".into());
        }
        if merge && sp.nodes.len() > 2 {
            let (mx, my) = mid(&sp.nodes[0], &sp.nodes[sp.nodes.len() - 1]);
            if let Some(last) = sp.nodes.pop() {
                let first = &mut sp.nodes[0];
                let (fx, fy) = (first.x, first.y);
                shift_node(first, mx - fx, my - fy);
                if let Some(h) = last.in_ {
                    first.in_ = Some([h[0] + mx - last.x, h[1] + my - last.y]);
                }
                first.ty = None;
            }
        }
        sp.closed = true;
        refresh_auto(&mut out);
        return Joined::Done(
            out,
            vec![NodeRef {
                sub: a.sub,
                index: 0.0,
            }],
        );
    }
    let pa = orient(&out[sa], a.index, true);
    let pb = orient(&out[sb], b.index, false);
    let (Some(a_end), Some(b_start)) = (pa.nodes.last(), pb.nodes.first()) else {
        return Joined::Error("Aynı düğüm iki kez seçilmiş.".into());
    };
    let nodes: Vec<PathNode> = if merge {
        let (mx, my) = mid(a_end, b_start);
        let joined = PathNode {
            in_: a_end
                .in_
                .map(|h| [h[0] + mx - a_end.x, h[1] + my - a_end.y]),
            out: b_start
                .out
                .map(|h| [h[0] + mx - b_start.x, h[1] + my - b_start.y]),
            ..PathNode::at(mx, my)
        };
        let mut v: Vec<PathNode> = pa.nodes[..pa.nodes.len() - 1].to_vec();
        v.push(joined);
        v.extend_from_slice(&pb.nodes[1..]);
        v
    } else {
        let mut v = pa.nodes.clone();
        v.extend_from_slice(&pb.nodes);
        v
    };
    let keep_at = sa.min(sb);
    let drop = sa.max(sb);
    out[keep_at] = SubPath {
        closed: false,
        nodes,
    };
    out.remove(drop);
    let at_node = if merge {
        pa.nodes.len() - 1
    } else {
        pa.nodes.len()
    };
    refresh_auto(&mut out);
    Joined::Done(
        out,
        vec![NodeRef {
            sub: keep_at as f64,
            index: at_node as f64,
        }],
    )
}

/// `array.slice(start, end)` for a count of items: JavaScript's clamping of numbers to indices.
fn slice_range(len: usize, start: f64, end: Option<f64>) -> (usize, usize) {
    let rel = |x: f64| -> usize {
        let x = if x.is_nan() { 0.0 } else { x.trunc() };
        if x < 0.0 {
            js_max(len as f64 + x, 0.0) as usize
        } else {
            js_min(x, len as f64) as usize
        }
    };
    let s = rel(start);
    let e = end.map_or(len, rel);
    (s, e.max(s))
}

/// Breaks the path at the chosen nodes: a closed sub-path opens there, an open one splits in two.
pub fn break_at_nodes(subs: &[SubPath], refs: &[NodeRef]) -> Vec<SubPath> {
    let mut out = Vec::new();
    for (s, sp) in subs.iter().enumerate() {
        let points: Vec<f64> = refs
            .iter()
            .filter(|r| r.sub == s as f64)
            .map(|r| r.index)
            .collect();
        if sp.closed && !points.is_empty() {
            // Open at the first chosen node: it becomes both ends.
            let k = points[0];
            let len = sp.nodes.len();
            let (a0, a1) = slice_range(len, k, None);
            let (b0, b1) = slice_range(len, 0.0, Some(k));
            let mut nodes: Vec<PathNode> = sp.nodes[a0..a1]
                .iter()
                .chain(&sp.nodes[b0..b1])
                .cloned()
                .collect();
            if nodes.is_empty() {
                continue;
            }
            let end = PathNode {
                in_: nodes[0].in_,
                ..PathNode::at(nodes[0].x, nodes[0].y)
            };
            nodes[0].in_ = None;
            nodes.push(end);
            // The other chosen nodes, re-indexed in the opened path.
            let n = len as f64;
            let rest: Vec<f64> = points[1..].iter().map(|&i| (i - k + n) % n).collect();
            out.extend(split_open(
                &SubPath {
                    closed: false,
                    nodes,
                },
                &rest,
            ));
        } else if !sp.closed {
            out.extend(split_open(sp, &points));
        } else {
            out.push(sp.clone());
        }
    }
    out
}

fn split_open(sp: &SubPath, points: &[f64]) -> Vec<SubPath> {
    let last = sp.nodes.len() as f64 - 1.0;
    let mut cuts: Vec<f64> = Vec::new();
    for &i in points {
        // `new Set(at)`: each value once (NaN too).
        if !cuts.iter().any(|&c| c == i || (c.is_nan() && i.is_nan())) {
            cuts.push(i);
        }
    }
    cuts.retain(|&i| i > 0.0 && i < last);
    stable_sort(&mut cuts, &mut |a, b| js_cmp(*a - *b, 0.0));
    if cuts.is_empty() {
        return vec![sp.clone()];
    }
    let mut out = Vec::new();
    let mut from = 0.0;
    cuts.push(last);
    for c in cuts {
        let (s, e) = slice_range(sp.nodes.len(), from, Some(c + 1.0));
        let mut nodes: Vec<PathNode> = sp.nodes[s..e].to_vec();
        if let Some(f) = nodes.first_mut() {
            f.in_ = None;
        }
        if let Some(l) = nodes.last_mut() {
            l.out = None;
        }
        out.push(SubPath {
            closed: false,
            nodes,
        });
        from = c;
    }
    out
}

/// Removes the segments between chosen neighbours: the path opens or splits there.
pub fn delete_segments(subs: &[SubPath], refs: &[NodeRef]) -> Result<Vec<SubPath>, String> {
    let segs = chosen_segments(subs, refs);
    if segs.is_empty() {
        return Err("Silinecek parçanın iki ucundaki düğümleri seçin.".into());
    }
    let mut out = Vec::new();
    for (s, sp) in subs.iter().enumerate() {
        let mine: Vec<usize> = segs
            .iter()
            .filter(|(x, _)| *x == s)
            .map(|&(_, i)| i)
            .collect();
        if mine.is_empty() {
            out.push(sp.clone());
            continue;
        }
        let n = sp.nodes.len();
        // Walk the sub-path from just after a removed segment, starting a new piece after each one.
        let start = if sp.closed { (mine[0] + 1) % n } else { 0 };
        let mut cur: Vec<PathNode> = Vec::new();
        for k in 0..n {
            let i = (start + k) % n;
            cur.push(sp.nodes[i].clone());
            let seg_removed = mine.contains(&i) && (sp.closed || i < n - 1);
            let last_node = !sp.closed && i == n - 1;
            if seg_removed || last_node || (sp.closed && k == n - 1) {
                if cur.len() >= 2 {
                    cur[0].in_ = None;
                    let l = cur.len() - 1;
                    cur[l].out = None;
                    out.push(SubPath {
                        closed: false,
                        nodes: std::mem::take(&mut cur),
                    });
                }
                cur.clear();
            }
        }
    }
    Ok(out)
}

/// The segments between chosen neighbours as straight lines or as (straight-looking) curves ready to bend.
pub fn segments_to(subs: &[SubPath], refs: &[NodeRef], line: bool) -> Vec<SubPath> {
    let mut out = subs.to_vec();
    for (s, i) in chosen_segments(subs, refs) {
        let sp = &mut out[s];
        let j = (i + 1) % sp.nodes.len();
        if line {
            sp.nodes[i].out = None;
            sp.nodes[j].in_ = None;
            for k in [i, j] {
                let t = &mut sp.nodes[k].ty;
                if t.as_deref().is_some_and(|t| !t.is_empty() && t != "cusp") {
                    *t = Some("cusp".into());
                }
            }
        } else if sp.nodes[i].out.is_none() && sp.nodes[j].in_.is_none() {
            let (a, b) = (sp.nodes[i].pt(), sp.nodes[j].pt());
            sp.nodes[i].out = Some([a[0] + (b[0] - a[0]) / 3.0, a[1] + (b[1] - a[1]) / 3.0]);
            sp.nodes[j].in_ = Some([
                a[0] + (2.0 * (b[0] - a[0])) / 3.0,
                a[1] + (2.0 * (b[1] - a[1])) / 3.0,
            ]);
        }
    }
    out
}

// ── Corners ────────────────────────────────────────────────────────────

/// What a corner node offers a fillet or chamfer: its angle and the longest cut along each side.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Corner {
    /// Angle between the two sides (radians, 0..π; π is straight on).
    pub angle: f64,
    /// Unit directions from the node along the incoming and outgoing sides.
    pub back: Pt,
    pub ahead: Pt,
    /// Longest distance the cut may reach along each side (the neighbours' distance).
    pub max: f64,
}

kentos_geometry_core::json_struct!(Corner {
    angle,
    back,
    ahead,
    max
});

pub fn corner_at(sp: &SubPath, i: usize) -> Option<Corner> {
    if segment_count(sp) < 2 {
        return None;
    }
    let n = sp.nodes.len();
    if i >= n || (!sp.closed && (i == 0 || i == n - 1)) {
        return None;
    }
    let in_seg = segment_cubic(sp, (i + n - 1) % n);
    let out_seg = segment_cubic(sp, i);
    let t1 = bez_tangent(&in_seg, 1.0);
    let t2 = bez_tangent(&out_seg, 0.0);
    let back: Pt = [-t1[0], -t1[1]];
    let angle = acos(js_max(-1.0, js_min(1.0, back[0] * t2[0] + back[1] * t2[1])));
    let chord = |c: &Cubic| len(c[3][0] - c[0][0], c[3][1] - c[0][1]);
    Some(Corner {
        angle,
        back,
        ahead: t2,
        max: js_min(chord(&in_seg), chord(&out_seg)),
    })
}

/// Tangent distance (from the corner along each side) of a fillet of radius r.
pub fn fillet_distance(c: &Corner, r: f64) -> f64 {
    r / tan(c.angle / 2.0)
}

/// Radius of the fillet reaching distance d along each side.
pub fn fillet_radius(c: &Corner, d: f64) -> f64 {
    d * tan(c.angle / 2.0)
}

/// Rounds (fillet: `size` is the radius) or cuts (chamfer: `size` is the
/// distance along each side) the corner at the chosen nodes. Sides may be
/// curves; the cut points are where the sides are that far from the
/// corner, and the fillet is the circular arc tangent to both.
pub fn corner_nodes(subs: &[SubPath], refs: &[NodeRef], fillet: bool, size: f64) -> Joined {
    if !(size > 0.0) {
        return Joined::Error(
            if fillet {
                "Yarıçap sıfırdan büyük olmalı."
            } else {
                "Pah boyu sıfırdan büyük olmalı."
            }
            .into(),
        );
    }
    let mut out = subs.to_vec();
    let mut made = Vec::new();
    let mut done = 0;
    let mut problem = String::new();
    // A Map from sub-path to its chosen indices, in the order the sub-paths first come.
    let mut by_sub: Vec<(f64, Vec<f64>)> = Vec::new();
    for r in refs {
        match by_sub.iter_mut().find(|(s, _)| *s == r.sub) {
            Some((_, list)) => list.push(r.index),
            None => by_sub.push((r.sub, vec![r.index])),
        }
    }
    for (s, list) in by_sub {
        let mut unique: Vec<f64> = Vec::new();
        for i in list {
            if !unique.iter().any(|&u| u == i || (u.is_nan() && i.is_nan())) {
                unique.push(i);
            }
        }
        // From the last node back, so earlier indices stay valid.
        stable_sort(&mut unique, &mut |a, b| js_cmp(*b - *a, 0.0));
        for i in unique {
            let Some(sp) = at(s).and_then(|k| out.get_mut(k)) else {
                continue;
            };
            let Some(c) = at(i).and_then(|k| corner_at(sp, k)) else {
                problem = "Açık yolun uç düğümü köşe değildir.".into();
                continue;
            };
            if c.angle > PI - 1e-3 {
                problem = "Düz devam eden bir düğüm köşe değildir.".into();
                continue;
            }
            if c.angle < 1e-3 {
                problem = "Geri dönen bir köşe yuvarlanamaz.".into();
                continue;
            }
            let d = if fillet {
                fillet_distance(&c, size)
            } else {
                size
            };
            match cut_corner(sp, i as usize, d, fillet, &c) {
                Err(e) => {
                    problem = e;
                    continue;
                }
                Ok((nodes, first)) => {
                    sp.nodes = nodes;
                    done += 1;
                    made.push(NodeRef {
                        sub: s,
                        index: first as f64,
                    });
                    made.push(NodeRef {
                        sub: s,
                        index: first as f64 + 1.0,
                    });
                }
            }
        }
    }
    if done == 0 {
        return Joined::Error(if problem.is_empty() {
            "Yuvarlanacak bir köşe düğümü seçin.".into()
        } else {
            problem
        });
    }
    refresh_auto(&mut out);
    Joined::Done(out, made)
}

/// An item of the corner's new node list: a node of the sub-path, or one of the two cut points.
#[derive(Clone, Copy, PartialEq)]
enum Item {
    Node(usize),
    P,
    Q,
}

fn cut_corner(
    sp: &SubPath,
    i: usize,
    d: f64,
    fillet: bool,
    c: &Corner,
) -> Result<(Vec<PathNode>, usize), String> {
    let n = sp.nodes.len();
    let pi = (i + n - 1) % n;
    let in_seg = segment_cubic(sp, pi);
    let out_seg = segment_cubic(sp, i);
    let ta = param_at_distance(&in_seg, d, true);
    let tb = param_at_distance(&out_seg, d, false);
    let (Some(ta), Some(tb)) = (ta, tb) else {
        return Err(if fillet {
            "Yarıçap bu köşe için çok büyük."
        } else {
            "Pah boyu bu köşe için çok büyük."
        }
        .into());
    };
    let in_line = segment_is_line(sp, pi);
    let out_line = segment_is_line(sp, i);
    // Straight sides are cut exactly (no search along them).
    let node = &sp.nodes[i];
    let pa: Pt = [node.x + c.back[0] * d, node.y + c.back[1] * d];
    let pb: Pt = [node.x + c.ahead[0] * d, node.y + c.ahead[1] * d];
    let keep_a: Cubic = if in_line {
        [in_seg[0], in_seg[0], pa, pa]
    } else {
        split_cubic(&in_seg, ta).0
    };
    let keep_b: Cubic = if out_line {
        [pb, pb, out_seg[3], out_seg[3]]
    } else {
        split_cubic(&out_seg, tb).1
    };
    let mut nodes = sp.nodes.clone();
    let next = (i + 1) % n;
    if !in_line {
        nodes[pi].out = Some([keep_a[1][0], keep_a[1][1]]);
    }
    if !out_line {
        nodes[next].in_ = Some([keep_b[2][0], keep_b[2][1]]);
    }
    let mut p = PathNode::at(keep_a[3][0], keep_a[3][1]);
    let mut q = PathNode::at(keep_b[0][0], keep_b[0][1]);
    if !in_line {
        p.in_ = Some([keep_a[2][0], keep_a[2][1]]);
    }
    if !out_line {
        q.out = Some([keep_b[1][0], keep_b[1][1]]);
    }
    if fillet {
        // Tangents at the cut points, toward the corner and away from it; a circular arc's handles.
        let ta = bez_tangent(&keep_a, 1.0);
        let tb = bez_tangent(&keep_b, 0.0);
        let turn = acos(js_max(-1.0, js_min(1.0, ta[0] * tb[0] + ta[1] * tb[1])));
        let chord = len(q.x - p.x, q.y - p.y);
        let r = if turn > 1e-9 {
            chord / (2.0 * sin(turn / 2.0))
        } else {
            0.0
        };
        let k = (4.0 / 3.0) * tan(turn / 4.0) * r;
        p.out = Some([p.x + ta[0] * k, p.y + ta[1] * k]);
        q.in_ = Some([q.x - tb[0] * k, q.y - tb[1] * k]);
    }
    // A cut reaching a neighbour node merges into it.
    let same = |a: &PathNode, b: &PathNode| len(a.x - b.x, a.y - b.y) < 1e-9;
    let mut list: Vec<Item> = (0..n).map(Item::Node).collect();
    list.splice(i..=i, [Item::P, Item::Q]);
    if same(&p, &nodes[pi]) {
        nodes[pi].out = p.out;
        if let Some(k) = list.iter().position(|x| *x == Item::P) {
            list.remove(k);
        }
    }
    let qi = list.iter().position(|x| *x == Item::Q).unwrap_or(0);
    let after = list[(qi + 1) % list.len()];
    if after != Item::Q {
        let after_node = match after {
            Item::Node(k) => nodes[k].clone(),
            Item::P => p.clone(),
            Item::Q => q.clone(),
        };
        if same(&q, &after_node) {
            match after {
                Item::Node(k) => nodes[k].in_ = q.in_,
                Item::P => p.in_ = q.in_,
                Item::Q => {}
            }
            list.remove(qi);
        }
    }
    let first = match list.iter().position(|x| *x == Item::P) {
        Some(k) => k,
        None => list.iter().position(|x| *x == Item::Node(pi)).unwrap_or(0),
    };
    let out = list
        .into_iter()
        .map(|x| match x {
            Item::Node(k) => nodes[k].clone(),
            Item::P => p.clone(),
            Item::Q => q.clone(),
        })
        .collect();
    Ok((out, first))
}

// ── Align and distribute nodes ─────────────────────────────────────────

fn axis_of(n: &PathNode, axis: &str) -> f64 {
    match axis {
        "x" => n.x,
        "y" => n.y,
        _ => f64::NAN,
    }
}

fn shift_on(n: &mut PathNode, axis: &str, d: f64) {
    shift_node(
        n,
        if axis == "x" { d } else { 0.0 },
        if axis == "y" { d } else { 0.0 },
    );
}

/// Moves the chosen nodes onto one line: their smallest, middle or largest x (or y).
pub fn align_nodes(subs: &[SubPath], refs: &[NodeRef], axis: &str, to: &str) -> Vec<SubPath> {
    let vals: Vec<f64> = refs
        .iter()
        .filter_map(|&r| node_at(subs, r))
        .map(|n| axis_of(n, axis))
        .collect();
    if vals.len() < 2 {
        return subs.to_vec();
    }
    let lo = vals.iter().fold(f64::INFINITY, |a, &b| js_min(a, b));
    let hi = vals.iter().fold(f64::NEG_INFINITY, |a, &b| js_max(a, b));
    let target = match to {
        "min" => lo,
        "max" => hi,
        _ => (lo + hi) / 2.0,
    };
    let mut out = subs.to_vec();
    for &r in refs {
        if let Some(n) = node_at_mut(&mut out, r) {
            let d = target - axis_of(n, axis);
            shift_on(n, axis, d);
        }
    }
    refresh_auto(&mut out);
    out
}

/// Spaces the chosen nodes evenly between the outermost ones along x (or y).
pub fn distribute_nodes(subs: &[SubPath], refs: &[NodeRef], axis: &str) -> Vec<SubPath> {
    let mut out = subs.to_vec();
    let mut list: Vec<(usize, usize)> = refs
        .iter()
        .filter_map(|&r| {
            let (s, i) = (at(r.sub)?, at(r.index)?);
            out.get(s)?.nodes.get(i)?;
            Some((s, i))
        })
        .collect();
    if list.len() < 3 {
        return out;
    }
    let value = |out: &[SubPath], (s, i): (usize, usize)| axis_of(&out[s].nodes[i], axis);
    {
        let snapshot = out.clone();
        stable_sort(&mut list, &mut |a, b| {
            js_cmp(value(&snapshot, *a) - value(&snapshot, *b), 0.0)
        });
    }
    let lo = value(&out, list[0]);
    let step = (value(&out, list[list.len() - 1]) - lo) / (list.len() - 1) as f64;
    for (k, &(s, i)) in list.iter().enumerate() {
        let d = lo + step * k as f64 - value(&out, (s, i));
        shift_on(&mut out[s].nodes[i], axis, d);
    }
    refresh_auto(&mut out);
    out
}
