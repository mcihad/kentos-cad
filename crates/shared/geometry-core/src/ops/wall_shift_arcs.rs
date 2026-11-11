//! Kaydır on a wall's arc (docs/adr/8049 §4): an arc edge's circle grown or shrunk about its centre by `by` to the arc's
//! left (towards the centre for an arc that turns left, a positive bulge; away for one that turns right), the arc
//! staying an arc of its own circle, its bulge the new sweep's. Its ends go as a straight edge's do, along the curve they
//! slide on: a straight neighbour's line (where it meets the new circle, nearer the old corner), the line of the one
//! wall whose end meets a path end (that end following), the line of the wall a path end tees into, two walls' line
//! when two meet the end in one line; a free end (or a `none` join) along the radius. The walls standing in the arc
//! (tees into it) slide along their last edge to the circle their end stood on, grown or shrunk the same.
//!
//! And a straight edge moved next to an arc (`arc_corner`): the corner goes where the moved line meets the arc's
//! circle, the arc keeping its circle and its far end, its bulge the new sweep's.
//!
//! Refused, nothing moving: a neighbour of the moved arc that is an arc too, a straight neighbour tangent to the arc (a
//! rounded corner: its centre would have to move), another wall's end at a corner, the circle shrinking to nothing, a
//! curve the new circle does not meet, an end meeting it in one line, an arc turning round.

use crate::geom::bulge::{bulge_arc, bulge_of_sweep};
use crate::geom::wall::{JoinMode, left_face};
use crate::geom::wall_join::{JOIN_TOLERANCE, WallIn};
use crate::jsmath::{PI, atan2, js_hypot, js_sign};
use crate::ops::edge_shift_arcs::{Curve, meet, sweep_to};
use crate::ops::wall_shift::{
    Moved, TIE, bulge, edge, edge_count, ends_at, host_at, inline, level,
};
use crate::vec2::Vec2;

fn sub(a: Vec2, b: Vec2) -> Vec2 {
    Vec2::new(a.x - b.x, a.y - b.y)
}

fn dot(a: Vec2, b: Vec2) -> f64 {
    a.x * b.x + a.y * b.y
}

fn cross(a: Vec2, b: Vec2) -> f64 {
    a.x * b.y - a.y * b.x
}

fn dist(a: Vec2, b: Vec2) -> f64 {
    js_hypot(b.x - a.x, b.y - a.y)
}

fn unit(a: Vec2, b: Vec2) -> Option<Vec2> {
    let l = dist(a, b);
    (l > 0.0).then(|| Vec2::new((b.x - a.x) / l, (b.y - a.y) / l))
}

/// The arc's way round at `p` on it: square to the radius, turning as it turns (`turn` ±1).
fn tangent(c: Vec2, p: Vec2, turn: f64) -> Vec2 {
    let d = sub(p, c);
    let l = js_hypot(d.x, d.y);
    Vec2::new(-d.y * turn / l, d.x * turn / l)
}

/// Whether `e` is within the arc from `a` that turns `sweep` about `c`, to the joins' tolerance along it.
fn within(c: Vec2, a: Vec2, sweep: f64, r: f64, e: Vec2) -> bool {
    let (u, v) = (sub(a, c), sub(e, c));
    let mut t = atan2(cross(u, v), dot(u, v)) * js_sign(sweep);
    if t < 0.0 {
        t += 2.0 * PI;
    }
    let slack = JOIN_TOLERANCE / r;
    t <= sweep.abs() + slack || t >= 2.0 * PI - slack
}

fn set(moved: &mut Vec<Moved>, walls: &[WallIn<'_>], wall: usize, vertex: usize, p: Vec2) {
    match moved.iter_mut().find(|m| m.wall == wall) {
        Some(m) => m.pts[vertex] = p,
        None => {
            let mut pts = walls[wall].pts.to_vec();
            pts[vertex] = p;
            moved.push(Moved {
                wall,
                pts,
                bulges: None,
            });
        }
    }
}

fn missed() -> String {
    "Yayın ucu kaydığı doğruyla yeni çemberde buluşmuyor; daha küçük bir değer yazın.".to_owned()
}

/// Wall `w`'s arc edge `k` moved `by` to its left, the walls joined to it following; each wall that changed once, `w`
/// first (its bulges given). Why not, in Turkish.
pub(crate) fn shift_arc(
    walls: &[WallIn<'_>],
    w: usize,
    k: usize,
    by: f64,
) -> Result<Vec<Moved>, String> {
    let me = &walls[w];
    let np = me.pts.len();
    let count = edge_count(me);
    let (a, b) = (me.pts[k], me.pts[(k + 1) % np]);
    let arc = bulge_arc(a, b, bulge(me, k))
        .ok_or_else(|| "Duvarın yay kenarının boyu yok.".to_owned())?;
    let turn = js_sign(arc.sweep);
    let r1 = arc.r - turn * by;
    if !(r1 > TIE) {
        return Err(format!(
            "Yay bu kaydırmada merkezine çöküyor ({:.3} m); daha küçük bir değer yazın.",
            by.abs()
        ));
    }
    let circle = Curve::Circle { c: arc.c, r: r1 };
    let radial = |p: Vec2| {
        Vec2::new(
            arc.c.x + (p.x - arc.c.x) * r1 / arc.r,
            arc.c.y + (p.y - arc.c.y) * r1 / arc.r,
        )
    };
    let on_line = |p: Vec2, dir: Vec2| meet(Curve::Line { p, dir }, circle, p).ok_or_else(missed);
    let mut moved = vec![Moved {
        wall: w,
        pts: me.pts.to_vec(),
        bulges: None,
    }];
    for (vertex, first) in [(k, true), ((k + 1) % np, false)] {
        let p = me.pts[vertex];
        let path_end = !me.spec.closed && (vertex == 0 || vertex == np - 1);
        if path_end {
            let end = usize::from(vertex != 0);
            let (q, other) = if me.spec.joins[end] == JoinMode::None {
                (radial(p), None)
            } else {
                match ends_at(walls, w, p)?.as_slice() {
                    [] => match host_at(walls, w, p) {
                        Some(h) => (on_line(p, h)?, None),
                        None => (radial(p), None),
                    },
                    [one] => {
                        if cross(tangent(arc.c, p, turn), one.dir).abs() <= inline() {
                            return Err("Kaydırılan yay ucunda ona teğet bir duvara bağlı; önce ikisini ayırın ya da birlikte taşıyın.".into());
                        }
                        (on_line(p, one.dir)?, Some((one.wall, one.vertex)))
                    }
                    many => {
                        // Two meeting in one line make the line this end stands on; they stay.
                        let mut line = None;
                        for (i, x) in many.iter().enumerate() {
                            for y in &many[i + 1..] {
                                if cross(x.dir, y.dir).abs() <= inline() && dot(x.dir, y.dir) < 0.0
                                {
                                    line = Some(x.dir);
                                }
                            }
                        }
                        let Some(d) = line else {
                            return Err("Kaydırılan yayın ucunda birden çok duvar birleşiyor; uç kaydırılamıyor.".into());
                        };
                        (on_line(p, d)?, None)
                    }
                }
            };
            set(&mut moved, walls, w, vertex, q);
            if let Some((j, i)) = other {
                set(&mut moved, walls, j, i, q);
            }
            continue;
        }
        let side = if first {
            (k + count - 1) % count
        } else {
            (k + 1) % count
        };
        let (c, d) = edge(me, side).ok_or_else(|| {
            "Kaydırılan yayın bitişiğindeki kenar da yay; yalnız düz komşuları olan yay kaydırılır.".to_owned()
        })?;
        let v = unit(c, d).ok_or_else(|| "Bitişik kenarın boyu yok.".to_owned())?;
        if cross(tangent(arc.c, p, turn), v).abs() <= inline() {
            return Err(
                "Yayın bitişiğindeki kenar ona teğet (yuvarlatılmış köşe); böyle bir yay kaydırılamaz.".into(),
            );
        }
        let others = ends_at(walls, w, p)?;
        if others.iter().any(|e| cross(v, e.dir).abs() > inline()) {
            return Err(
                "Kaydırılan yayın köşesine başka bir duvarın ucu bağlı; kaydırılamıyor.".into(),
            );
        }
        let q = on_line(p, v)?;
        set(&mut moved, walls, w, vertex, q);
        for e in &others {
            set(&mut moved, walls, e.wall, e.vertex, q);
        }
    }
    // The walls standing in the arc (tees into it) go with it, each end keeping its place across the wall.
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
            let rho = dist(e, arc.c);
            let off = (arc.r - rho) * turn;
            if off < rf - JOIN_TOLERANCE
                || off > lf + JOIN_TOLERANCE
                || !within(arc.c, a, arc.sweep, arc.r, e)
            {
                continue;
            }
            if bulge(other, seg) != 0.0 {
                return Err(
                    "Kaydırılan yaya saplanan bir duvarın ucu yay; o duvarı düz kenarla bitirin."
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
            let to = rho - turn * by;
            let q = (to > TIE)
                .then(|| {
                    meet(
                        Curve::Line { p: e, dir },
                        Curve::Circle { c: arc.c, r: to },
                        e,
                    )
                })
                .flatten()
                .ok_or_else(|| {
                    "Yaya saplanan bir duvar yeni yaya ulaşamıyor; daha küçük bir değer yazın."
                        .to_owned()
                })?;
            set(&mut moved, walls, j, vertex, q);
        }
    }
    // The arc's new sweep, the way it turned.
    let (qa, qb) = (moved[0].pts[k], moved[0].pts[(k + 1) % np]);
    let sweep = sweep_to(arc.c, qa, qb, arc.sweep).ok_or_else(|| {
        format!(
            "Bu kaydırma yayı ters çeviriyor ({:.3} m); daha küçük bir değer yazın.",
            by.abs()
        )
    })?;
    let mut bulges = bulges_of(me);
    bulges[k] = bulge_of_sweep(sweep);
    moved[0].bulges = Some(bulges);
    kept_ways(walls, &moved, by)?;
    Ok(moved)
}

/// A straight edge's corner `p` moved to where the edge's line, moved `by` along `n` (its direction `u`), meets the arc
/// next to it (edge `side` of wall `w`): the corner and the arc's new bulge (its circle and far end kept). `first`: the
/// arc comes before the corner along the path.
pub(crate) fn arc_corner(
    me: &WallIn<'_>,
    side: usize,
    first: bool,
    p: Vec2,
    u: Vec2,
    n: Vec2,
    by: f64,
) -> Result<(Vec2, f64), String> {
    let np = me.pts.len();
    let (s0, s1) = (me.pts[side], me.pts[(side + 1) % np]);
    let arc =
        bulge_arc(s0, s1, bulge(me, side)).ok_or_else(|| "Bitişik yayın boyu yok.".to_owned())?;
    if cross(tangent(arc.c, p, js_sign(arc.sweep)), u).abs() <= inline() {
        return Err(
            "Kaydırılan kenarın bitişiğindeki yay ona teğet (yuvarlatılmış köşe); böyle bir kenar kaydırılamaz."
                .into(),
        );
    }
    let line = Curve::Line {
        p: Vec2::new(p.x + n.x * by, p.y + n.y * by),
        dir: u,
    };
    let q = meet(line, Curve::Circle { c: arc.c, r: arc.r }, p).ok_or_else(|| {
        "Kaydırılan kenar bitişiğindeki yayla buluşmuyor; daha küçük bir değer yazın.".to_owned()
    })?;
    let (from, to) = if first { (s0, q) } else { (q, s1) };
    let sweep = sweep_to(arc.c, from, to, arc.sweep).ok_or_else(|| {
        format!(
            "Bu kaydırma bitişiğindeki yayı ters çeviriyor ({:.3} m); daha küçük bir değer yazın.",
            by.abs()
        )
    })?;
    Ok((q, bulge_of_sweep(sweep)))
}

/// A wall's bulges to change, one per point as a wall keeps them (the last of an open path's none's).
pub(crate) fn bulges_of(w: &WallIn<'_>) -> Vec<f64> {
    let mut out = w.bulges.map_or_else(Vec::new, <[f64]>::to_vec);
    if out.len() < w.pts.len() {
        out.resize(w.pts.len(), 0.0);
    }
    out
}

/// Every edge of the walls moved keeps a length and its way (its chord's).
pub(crate) fn kept_ways(walls: &[WallIn<'_>], moved: &[Moved], by: f64) -> Result<(), String> {
    for m in moved {
        let old = walls[m.wall].pts;
        for s in 0..edge_count(&walls[m.wall]) {
            let (i, j) = (s, (s + 1) % old.len());
            if old[i] == m.pts[i] && old[j] == m.pts[j] {
                continue;
            }
            let (p0, p1) = (m.pts[i], m.pts[j]);
            if dist(p0, p1) <= JOIN_TOLERANCE || dot(sub(p1, p0), sub(old[j], old[i])) <= 0.0 {
                return Err(format!(
                    "Bu kaydırma bir duvar kenarını yok ediyor ya da ters çeviriyor ({:.3} m); daha küçük bir değer yazın.",
                    by.abs()
                ));
            }
        }
    }
    Ok(())
}
