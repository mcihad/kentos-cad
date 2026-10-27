//! A wall's straight edge moved square to itself, and what is joined to it following (docs/adr/0710): Kaydır and the
//! temporary distances of a selected wall. One rule for the desktop, the headless server and the shared cases.
//!
//! - **Gaps:** on each side of the edge, the nearest straight edge of a wall on the same level (the wall's own other
//!   edges too) that runs parallel to it (within 0.001°) and overlaps it along its line: the clear distance from the
//!   edge's face on that side to that edge's facing face, square to the edge, measured at the middle of the overlap.
//!   Equal distances (within [`TIE`]) go to the wall given first, then to its lower edge.
//! - **Kaydır:** the edge's line moved `by` to its left (negative: to its right). A vertex between two edges slides
//!   along the other edge. A path end slides along the one wall whose end meets it, that end following to the same
//!   point (a corner); along the wall it stands in (a tee: that wall stays), or along the line of two walls meeting
//!   it in one line; square to the edge when nothing joins it (or its join is `none`). The ends of the walls standing
//!   in the edge (tees into it) slide along their own last edge with it. Refused, nothing moving: an arc where a
//!   vertex slides, an adjacent edge along the same line, a wall end meeting it in one line, more walls at an end,
//!   another wall's end at a vertex between two edges, or an edge that would shrink to nothing or turn round.

use crate::geom::wall::{JoinMode, left_face};
use crate::geom::wall_join::{INLINE_DEGREES, JOIN_TOLERANCE, WallIn};
use crate::jsmath::{PI, js_hypot, js_max, js_min, sin};
use crate::vec2::Vec2;

/// Slack, metres.
pub const TIE: f64 = 1e-9;
/// Two directions this near are parallel: the sine of 0.001°.
pub const PARALLEL: f64 = 1.745_329_251_994_33e-5;

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn left(u: Vec2) -> Vec2 {
    Vec2::new(-u.y, u.x)
}

fn along(p: Vec2, d: Vec2, t: f64) -> Vec2 {
    Vec2::new(p.x + d.x * t, p.y + d.y * t)
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    js_hypot(b.x - a.x, b.y - a.y)
}

fn unit(a: Vec2, b: Vec2) -> Option<Vec2> {
    let l = dist(a, b);
    (l > 0.0).then(|| Vec2::new((b.x - a.x) / l, (b.y - a.y) / l))
}

/// The sine of [`INLINE_DEGREES`]: two ends this near one line meet inline.
fn inline() -> f64 {
    sin(INLINE_DEGREES * PI / 180.0)
}

/// A wall's level: walls join and face each other only on the same one (or both on none), as the joins say.
fn level<'a>(w: &WallIn<'a>) -> Option<&'a str> {
    match w.spec.arch.as_ref()?.get("level") {
        crate::api::json::Json::Str(id) => Some(id.as_str()),
        _ => None,
    }
}

/// The number of edges of a wall's path: a closed one's last runs back to its first point.
pub fn edge_count(w: &WallIn<'_>) -> usize {
    let n = w.pts.len();
    if w.spec.closed {
        n
    } else {
        n.saturating_sub(1)
    }
}

fn bulge(w: &WallIn<'_>, i: usize) -> f64 {
    w.bulges.and_then(|b| b.get(i)).copied().unwrap_or(0.0)
}

/// Edge `k`'s two points, when it is a straight edge of a length.
fn edge(w: &WallIn<'_>, k: usize) -> Option<(Vec2, Vec2)> {
    if k >= edge_count(w) || bulge(w, k) != 0.0 {
        return None;
    }
    let (a, b) = (w.pts[k], w.pts[(k + 1) % w.pts.len()]);
    (dist(a, b) > 0.0).then_some((a, b))
}

/// The straight edge `k` of wall `w` and its frame: its start, its direction, its left, its length.
fn frame(walls: &[WallIn<'_>], w: usize, k: usize) -> Result<(Vec2, Vec2, Vec2, f64), String> {
    let me = walls.get(w).ok_or_else(|| "Duvar yok.".to_owned())?;
    let count = edge_count(me);
    if k >= count {
        return Err(match count {
            0 => "Duvarın kenarı yok.".to_owned(),
            1 => format!("Duvarın {k}. kenarı yok; tek kenarı 0."),
            _ => format!(
                "Duvarın {k}. kenarı yok; kenarları 0 ile {} arasında.",
                count - 1
            ),
        });
    }
    let (a, b) = edge(me, k).ok_or_else(|| {
        "Duvarın bu kenarı yay; geçici ölçü ve kaydırma düz kenardadır.".to_owned()
    })?;
    let l = dist(a, b);
    let u = Vec2::new((b.x - a.x) / l, (b.y - a.y) / l);
    Ok((a, u, left(u), l))
}

/// A gap on one side of an edge.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Gap {
    /// The wall measured to (its index in the walls given), its edge, and whether its left face is the one facing.
    pub wall: usize,
    pub edge: usize,
    pub left_face: bool,
    /// The clear distance, metres.
    pub distance: f64,
    /// Where it is measured: on the edge's face, and on the other's facing face.
    pub from: Vec2,
    pub to: Vec2,
}

fn take(slot: &mut Option<Gap>, g: Gap) {
    match slot {
        Some(b) if b.distance <= g.distance + TIE => {}
        _ => *slot = Some(g),
    }
}

/// The gaps of wall `w`'s edge `k`, on its left and on its right; why there are none (no such edge, an arc).
pub fn gaps(walls: &[WallIn<'_>], w: usize, k: usize) -> Result<[Option<Gap>; 2], String> {
    let (a, u, n, l) = frame(walls, w, k)?;
    let me = &walls[w];
    let lf = left_face(me.spec);
    let rf = lf - me.spec.width;
    let lvl = level(me);
    let mut best: [Option<Gap>; 2] = [None, None];
    for (j, other) in walls.iter().enumerate() {
        if level(other) != lvl {
            continue;
        }
        for e in 0..edge_count(other) {
            if j == w && e == k {
                continue;
            }
            let Some((c, d)) = edge(other, e) else {
                continue;
            };
            let Some(v) = unit(c, d) else { continue };
            if cross(u, v).abs() > PARALLEL {
                continue;
            }
            let (tc, td) = (dot(sub(c, a), u), dot(sub(d, a), u));
            let lo = js_max(0.0, js_min(tc, td));
            let hi = js_min(l, js_max(tc, td));
            if hi - lo <= TIE {
                continue;
            }
            // Its faces as offsets to the edge's left: its left face is to its own left, which is the edge's left
            // when they run the same way.
            let sgn = if dot(left(v), n) > 0.0 { 1.0 } else { -1.0 };
            let sd = dot(sub(c, a), n);
            let olf = left_face(other.spec);
            let f_left = sd + sgn * olf;
            let f_right = sd + sgn * (olf - other.spec.width);
            let (near, near_left) = if f_left <= f_right {
                (f_left, true)
            } else {
                (f_right, false)
            };
            let (far, far_left) = if f_left >= f_right {
                (f_left, true)
            } else {
                (f_right, false)
            };
            let base = along(a, u, (lo + hi) / 2.0);
            if near >= lf - TIE {
                take(
                    &mut best[0],
                    Gap {
                        wall: j,
                        edge: e,
                        left_face: near_left,
                        distance: near - lf,
                        from: along(base, n, lf),
                        to: along(base, n, near),
                    },
                );
            }
            if far <= rf + TIE {
                take(
                    &mut best[1],
                    Gap {
                        wall: j,
                        edge: e,
                        left_face: far_left,
                        distance: rf - far,
                        from: along(base, n, rf),
                        to: along(base, n, far),
                    },
                );
            }
        }
    }
    Ok(best)
}

/// A wall's path after a Kaydır: its index in the walls given and its points.
#[derive(Clone, Debug, PartialEq)]
pub struct Moved {
    pub wall: usize,
    pub pts: Vec<Vec2>,
}

/// An end of another open wall: its wall, its vertex and its last edge's direction (from the end inward).
struct EndAt {
    wall: usize,
    vertex: usize,
    dir: Vec2,
}

/// The ends of the open walls other than `w` on its level within the joins' tolerance of `p`, their joins not `none`.
fn ends_at(walls: &[WallIn<'_>], w: usize, p: Vec2) -> Result<Vec<EndAt>, String> {
    let lvl = level(&walls[w]);
    let mut out = Vec::new();
    for (j, other) in walls.iter().enumerate() {
        let n = other.pts.len();
        if j == w || level(other) != lvl || other.spec.closed || n < 2 {
            continue;
        }
        for (end, vertex, seg) in [(0, 0, 0), (1, n - 1, n - 2)] {
            if other.spec.joins[end] == JoinMode::None
                || dist(other.pts[vertex], p) > JOIN_TOLERANCE
            {
                continue;
            }
            if bulge(other, seg) != 0.0 {
                return Err(
                    "Kaydırılan duvara bağlı bir duvarın ucu yay; o duvarı düz kenarla bitirin."
                        .into(),
                );
            }
            let inward = if end == 0 {
                (other.pts[0], other.pts[1])
            } else {
                (other.pts[n - 1], other.pts[n - 2])
            };
            let Some(dir) = unit(inward.0, inward.1) else {
                continue;
            };
            out.push(EndAt {
                wall: j,
                vertex,
                dir,
            });
        }
    }
    Ok(out)
}

/// The direction of a straight edge of another wall on `w`'s level whose body `p` stands in: the wall an end tees into.
fn host_at(walls: &[WallIn<'_>], w: usize, p: Vec2) -> Option<Vec2> {
    let lvl = level(&walls[w]);
    for (j, other) in walls.iter().enumerate() {
        if j == w || level(other) != lvl {
            continue;
        }
        let lf = left_face(other.spec);
        let rf = lf - other.spec.width;
        for e in 0..edge_count(other) {
            let Some((c, d)) = edge(other, e) else {
                continue;
            };
            let Some(v) = unit(c, d) else { continue };
            let t = dot(sub(p, c), v);
            let o = dot(sub(p, c), left(v));
            if t >= -JOIN_TOLERANCE
                && t <= dist(c, d) + JOIN_TOLERANCE
                && o >= rf - JOIN_TOLERANCE
                && o <= lf + JOIN_TOLERANCE
            {
                return Some(v);
            }
        }
    }
    None
}

/// Slides `p` along `d` until it is `by` further to the edge's left `n`; none when `d` runs along the edge.
fn slide(p: Vec2, d: Vec2, n: Vec2, by: f64) -> Option<Vec2> {
    let dn = dot(d, n);
    (dn.abs() > PARALLEL).then(|| along(p, d, by / dn))
}

/// Where a path end of the moved edge goes, and the other wall's end that goes with it (a corner).
fn end_goes(
    walls: &[WallIn<'_>],
    w: usize,
    end: usize,
    p: Vec2,
    u: Vec2,
    n: Vec2,
    by: f64,
) -> Result<(Vec2, Option<(usize, usize)>), String> {
    if walls[w].spec.joins[end] == JoinMode::None {
        return Ok((along(p, n, by), None));
    }
    let meeting = ends_at(walls, w, p)?;
    let stuck = || {
        "Kaydırılan duvarın ucundaki duvar onunla aynı doğrultuda; uç kaydırılamıyor.".to_owned()
    };
    match meeting.as_slice() {
        [] => match host_at(walls, w, p) {
            Some(h) => slide(p, h, n, by).map(|q| (q, None)).ok_or_else(stuck),
            None => Ok((along(p, n, by), None)),
        },
        [one] => {
            if cross(u, one.dir).abs() <= inline() {
                return Err("Kaydırılan duvar ucunda aynı doğrultudaki bir duvara bağlı (uç uca); önce ikisini ayırın ya da birlikte taşıyın.".into());
            }
            let q = slide(p, one.dir, n, by).ok_or_else(stuck)?;
            Ok((q, Some((one.wall, one.vertex))))
        }
        many => {
            // Two meeting in one line make the line this end stands on; they stay.
            for (i, x) in many.iter().enumerate() {
                for y in &many[i + 1..] {
                    if cross(x.dir, y.dir).abs() <= inline() && dot(x.dir, y.dir) < 0.0 {
                        return slide(p, x.dir, n, by).map(|q| (q, None)).ok_or_else(stuck);
                    }
                }
            }
            Err("Kaydırılan duvarın ucunda birden çok duvar birleşiyor; uç kaydırılamıyor.".into())
        }
    }
}

/// Wall `w`'s edge `k` moved `by` to its left (negative: to its right), and the walls joined to it following; each
/// wall that changed once, `w` first. Why not: the edge, a join that cannot follow, or an edge lost.
pub fn shift(walls: &[WallIn<'_>], w: usize, k: usize, by: f64) -> Result<Vec<Moved>, String> {
    if !by.is_finite() {
        return Err("Kaydırma sonlu bir uzunluk olmalı.".into());
    }
    let (a, u, n, l) = frame(walls, w, k)?;
    let me = &walls[w];
    let np = me.pts.len();
    let count = edge_count(me);
    let mut moved: Vec<Moved> = vec![Moved {
        wall: w,
        pts: me.pts.to_vec(),
    }];
    let set = |moved: &mut Vec<Moved>, wall: usize, vertex: usize, p: Vec2| match moved
        .iter_mut()
        .find(|m| m.wall == wall)
    {
        Some(m) => m.pts[vertex] = p,
        None => {
            let mut pts = walls[wall].pts.to_vec();
            pts[vertex] = p;
            moved.push(Moved { wall, pts });
        }
    };
    let b = me.pts[(k + 1) % np];
    for (vertex, first) in [(k, true), ((k + 1) % np, false)] {
        let p = me.pts[vertex];
        let path_end = !me.spec.closed && (vertex == 0 || vertex == np - 1);
        if path_end {
            let (q, other) = end_goes(walls, w, usize::from(vertex != 0), p, u, n, by)?;
            set(&mut moved, w, vertex, q);
            if let Some((j, i)) = other {
                set(&mut moved, j, i, q);
            }
            continue;
        }
        let side = if first {
            (k + count - 1) % count
        } else {
            (k + 1) % count
        };
        let (c, d) = edge(me, side).ok_or_else(|| {
            "Kaydırılan kenarın bitişiğindeki kenar yay; yalnız düz kenarlar arasında kaydırılır.".to_owned()
        })?;
        let v = unit(c, d).ok_or_else(|| "Bitişik kenarın boyu yok.".to_owned())?;
        if cross(u, v).abs() <= PARALLEL {
            return Err(
                "Kaydırılan kenarın bitişiğindeki kenar aynı doğrultuda; o köşeyi kaldırın.".into(),
            );
        }
        if !ends_at(walls, w, p)?.is_empty() {
            return Err(
                "Kaydırılan kenarın köşesine başka bir duvarın ucu bağlı; kaydırılamıyor.".into(),
            );
        }
        let q = slide(p, v, n, by).ok_or_else(|| "Bitişik kenar aynı doğrultuda.".to_owned())?;
        set(&mut moved, w, vertex, q);
    }
    // The walls standing in the edge (tees into it) go with it.
    let lf = left_face(me.spec);
    let rf = lf - me.spec.width;
    let lvl = level(me);
    for (j, other) in walls.iter().enumerate() {
        let on = other.pts.len();
        if j == w || level(other) != lvl || other.spec.closed || on < 2 {
            continue;
        }
        for (end, vertex, seg) in [(0, 0, 0), (1, on - 1, on - 2)] {
            let e = other.pts[vertex];
            if other.spec.joins[end] == JoinMode::None
                || dist(e, a) <= JOIN_TOLERANCE
                || dist(e, b) <= JOIN_TOLERANCE
            {
                continue;
            }
            let t = dot(sub(e, a), u);
            let o = dot(sub(e, a), n);
            if t < -JOIN_TOLERANCE
                || t > l + JOIN_TOLERANCE
                || o < rf - JOIN_TOLERANCE
                || o > lf + JOIN_TOLERANCE
            {
                continue;
            }
            if bulge(other, seg) != 0.0 {
                return Err(
                    "Kaydırılan duvara saplanan bir duvarın ucu yay; o duvarı düz kenarla bitirin."
                        .into(),
                );
            }
            let inward = if end == 0 {
                (other.pts[0], other.pts[1])
            } else {
                (other.pts[on - 1], other.pts[on - 2])
            };
            let Some(dir) = unit(inward.0, inward.1) else {
                continue;
            };
            if let Some(q) = slide(e, dir, n, by) {
                set(&mut moved, j, vertex, q);
            }
        }
    }
    // Every edge keeps a length and its way.
    for m in &moved {
        let old = walls[m.wall].pts;
        let ne = edge_count(&walls[m.wall]);
        for s in 0..ne {
            let (i, j) = (s, (s + 1) % old.len());
            if old[i] == m.pts[i] && old[j] == m.pts[j] {
                continue;
            }
            let (p0, p1) = (m.pts[i], m.pts[j]);
            let (o0, o1) = (old[i], old[j]);
            let new_dir = sub(p1, p0);
            if dist(p0, p1) <= JOIN_TOLERANCE || dot(new_dir, sub(o1, o0)) <= 0.0 {
                return Err(format!(
                    "Bu kaydırma bir duvar kenarını yok ediyor ya da ters çeviriyor ({:.3} m); daha küçük bir değer yazın.",
                    by.abs()
                ));
            }
        }
    }
    Ok(moved)
}
