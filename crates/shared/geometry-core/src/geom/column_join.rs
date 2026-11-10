//! A column in the walls it stands in (docs/adr/3041 §2).
//!
//! A wall on the column's level is cut where the column stands: its lines, caps, recesses and closures keep their parts
//! outside the column, its layers' bands and its body lose what the column covers. A brick wall stops at the column's
//! face and its hatch no longer runs under the column.
//!
//! Where the column meets a wall's layer of its own material (a reinforced concrete column and a shear wall's
//! concrete), the two are one section, as ISO 128-3 (7.2) hatches the parts of one item alike: the column takes that
//! layer's hatch and its outline is not drawn where it lies inside the layer. Between different materials the line
//! stays.
//!
//! A round column meets the walls as a 64-sided ring (its fill's, [`crate::geom::arch`]): at 1:50 its sides stray from
//! the circle by a hundredth of a millimetre on paper.

use crate::geom::arch::{ColumnSpec, column_center, column_corners};
use crate::geom::arrangement::{Area, Ring};
use crate::geom::intersect::line_line;
use crate::geom::wall::{Body, ComponentFill, Detail, Drawn, Line, Spec, Stroke};
use crate::geometry::{Bounds, point_in_polygon};
use crate::jsmath::{TAU, cos, js_hypot, js_max, js_min, sin};
use crate::vec2::Vec2;

/// Lengths and parameters closer than this are the same.
const EPS: f64 = 1e-9;

/// A column as the walls on its level meet it.
#[derive(Clone, Debug, PartialEq)]
pub struct Foot {
    /// Its outline counter-clockwise: a rectangle's or an L's corners, a round column's 64 points.
    pub ring: Vec<Vec2>,
    pub bounds: Bounds,
    /// Its level's id; a wall cuts by it on the same level, or both on none.
    pub level: Option<String>,
    /// Its material, folded (Turkish letters plain, lower case): Betonarme unless it names another.
    pub material: String,
}

/// What material names are compared as: trimmed and folded, so "BETONARME" and "Betonarme" are one material.
pub fn material_key(name: &str) -> String {
    crate::ops::geocode::fold::fold(name.trim())
}

impl Foot {
    /// The column at `at`; none for one that cannot be drawn (a size or a point not finite, an L's legs too thick).
    pub fn of(at: Vec2, spec: &ColumnSpec) -> Option<Foot> {
        crate::geom::arch::column_area(at, spec)?;
        let ring = if spec.round {
            let c = column_center(at, spec);
            let r = spec.width / 2.0;
            (0..64)
                .map(|k| {
                    let a = f64::from(k) * TAU / 64.0;
                    Vec2::new(c.x + r * cos(a), c.y + r * sin(a))
                })
                .collect()
        } else {
            column_corners(at, spec)
        };
        let bounds = bounds_of(&ring);
        Some(Foot {
            ring,
            bounds,
            level: spec.level.clone(),
            material: material_key(spec.material_name()),
        })
    }

    /// Its footprint as an area.
    pub fn area(&self) -> Area {
        Area {
            outer: Ring {
                pts: self.ring.clone(),
                bulges: None,
            },
            holes: Vec::new(),
        }
    }
}

fn bounds_of(pts: &[Vec2]) -> Bounds {
    let mut b = Bounds {
        min_x: f64::INFINITY,
        min_y: f64::INFINITY,
        max_x: f64::NEG_INFINITY,
        max_y: f64::NEG_INFINITY,
    };
    for p in pts {
        b.min_x = js_min(b.min_x, p.x);
        b.min_y = js_min(b.min_y, p.y);
        b.max_x = js_max(b.max_x, p.x);
        b.max_y = js_max(b.max_y, p.y);
    }
    b
}

/// Whether two boxes overlap, `grow` added round the first.
pub fn boxes_meet(a: &Bounds, b: &Bounds, grow: f64) -> bool {
    a.min_x - grow <= b.max_x
        && b.min_x <= a.max_x + grow
        && a.min_y - grow <= b.max_y
        && b.min_y <= a.max_y + grow
}

/// The distance from `p` to the segment a→b.
fn to_segment(p: Vec2, a: Vec2, b: Vec2) -> f64 {
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    let t = if l2 > 0.0 {
        (((p.x - a.x) * dx + (p.y - a.y) * dy) / l2).clamp(0.0, 1.0)
    } else {
        0.0
    };
    js_hypot(p.x - (a.x + t * dx), p.y - (a.y + t * dy))
}

/// Whether `p` is inside the ring and not on it.
fn strictly_inside(p: Vec2, ring: &[Vec2]) -> bool {
    let n = ring.len();
    if n < 3 {
        return false;
    }
    if (0..n).any(|i| to_segment(p, ring[i], ring[(i + 1) % n]) <= EPS) {
        return false;
    }
    point_in_polygon(p, ring)
}

/// Whether `p` is inside a body (its ring less its holes) and on none of its rings.
fn inside_body(p: Vec2, body: &Body) -> bool {
    strictly_inside(p, &body.0)
        && body.1.as_ref().is_none_or(|holes| {
            holes
                .iter()
                .all(|h| !strictly_inside(p, h) && !on_ring(p, h))
        })
}

fn on_ring(p: Vec2, ring: &[Vec2]) -> bool {
    let n = ring.len();
    (0..n).any(|i| to_segment(p, ring[i], ring[(i + 1) % n]) <= EPS)
}

/// The parameters along a→b, inside (0, 1), where it crosses or touches the ring's sides, a side lying along it
/// giving its ends.
fn crossings(a: Vec2, b: Vec2, ring: &[Vec2], out: &mut Vec<f64>) {
    let n = ring.len();
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let l2 = dx * dx + dy * dy;
    if l2 <= 0.0 {
        return;
    }
    let mut take = |t: f64| {
        if t > EPS && t < 1.0 - EPS {
            out.push(t);
        }
    };
    for i in 0..n {
        let (p, q) = (ring[i], ring[(i + 1) % n]);
        match line_line(a, b, p, q) {
            Some(h) if h.u >= -EPS && h.u <= 1.0 + EPS => take(h.t),
            Some(_) => {}
            None => {
                // Parallel: along it, its ends are where the parts change.
                let l = l2.sqrt();
                let off = |s: Vec2| ((s.x - a.x) * dy - (s.y - a.y) * dx).abs() / l;
                if off(p) <= EPS && off(q) <= EPS {
                    for s in [p, q] {
                        take(((s.x - a.x) * dx + (s.y - a.y) * dy) / l2);
                    }
                }
            }
        }
    }
}

fn lerp(a: Vec2, b: Vec2, t: f64) -> Vec2 {
    Vec2::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t)
}

/// The parts of the path `pts` (closed: back to its first point) whose points are not taken by `inside`, split where
/// `cross` says its edges cross the takers' outlines; none when no part of it is taken.
fn parts_outside(
    pts: &[Vec2],
    closed: bool,
    inside: impl Fn(Vec2) -> bool,
    cross: impl Fn(Vec2, Vec2, &mut Vec<f64>),
) -> Option<Vec<Vec<Vec2>>> {
    if pts.len() < 2 {
        return None;
    }
    let mut path = pts.to_vec();
    if closed {
        path.push(pts[0]);
    }
    let mut parts: Vec<Vec<Vec2>> = Vec::new();
    let mut cur: Vec<Vec2> = Vec::new();
    let mut taken = false;
    for w in path.windows(2) {
        let (a, b) = (w[0], w[1]);
        let mut ts = vec![0.0, 1.0];
        cross(a, b, &mut ts);
        ts.sort_by(f64::total_cmp);
        ts.dedup_by(|x, y| (*x - *y).abs() <= EPS);
        for t in ts.windows(2) {
            let (t0, t1) = (t[0], t[1]);
            let (p0, p1) = (lerp(a, b, t0), lerp(a, b, t1));
            if inside(lerp(a, b, (t0 + t1) / 2.0)) {
                taken = true;
                if cur.len() >= 2 {
                    parts.push(std::mem::take(&mut cur));
                }
                cur.clear();
            } else {
                if cur.last() != Some(&p0) {
                    if cur.len() >= 2 {
                        parts.push(std::mem::take(&mut cur));
                    }
                    cur = vec![p0];
                }
                cur.push(p1);
            }
        }
    }
    if cur.len() >= 2 {
        parts.push(cur);
    }
    if !taken {
        return None;
    }
    // A closed path's last part runs on into its first through the start.
    if closed && parts.len() >= 2 && parts[0].first() == parts.last().and_then(|p| p.last()) {
        let first = parts.remove(0);
        if let Some(last) = parts.last_mut() {
            last.extend_from_slice(&first[1..]);
        }
    }
    Some(parts)
}

/// A line's points as drawn: its arcs as chords.
fn line_points(line: &Line, closed: bool) -> Vec<Vec2> {
    if line.bulges.iter().any(|b| *b != 0.0) {
        let mut pts = line.outline(closed);
        // The outline of a closed line repeats its first point last.
        if closed && pts.len() > 1 && pts.first() == pts.last() {
            pts.pop();
        }
        pts
    } else {
        line.pts.clone()
    }
}

/// The parts of a stroke outside the feet.
fn cut_stroke(s: &Stroke, feet: &[&Foot]) -> Option<Vec<Stroke>> {
    let pts = line_points(&s.line, s.closed);
    let parts = parts_outside(
        &pts,
        s.closed,
        |p| feet.iter().any(|f| strictly_inside(p, &f.ring)),
        |a, b, out| feet.iter().for_each(|f| crossings(a, b, &f.ring, out)),
    )?;
    Some(
        parts
            .into_iter()
            .map(|pts| Stroke {
                line: Line {
                    bulges: vec![0.0; pts.len()],
                    pts,
                },
                closed: false,
            })
            .collect(),
    )
}

/// What of a body is outside the feet; none when they leave it as it is.
fn cut_body(body: &Body, feet: &[&Foot]) -> Option<Vec<Body>> {
    let b = bounds_of(&body.0);
    let near: Vec<Area> = feet
        .iter()
        .filter(|f| boxes_meet(&f.bounds, &b, EPS))
        .map(|f| f.area())
        .collect();
    if near.is_empty() {
        return None;
    }
    let area = Area {
        outer: Ring {
            pts: body.0.clone(),
            bulges: None,
        },
        holes: body
            .1
            .iter()
            .flatten()
            .map(|h| Ring {
                pts: h.clone(),
                bulges: None,
            })
            .collect(),
    };
    let left = crate::geom::region::subtract_areas(std::slice::from_ref(&area), &near);
    // Nothing covered: the body as it is (the overlay may have reordered its points).
    if left.len() == 1 && (area_of(&left[0]) - area_of(&area)).abs() <= 1e-12 {
        return None;
    }
    Some(
        left.into_iter()
            .map(|a| {
                let holes: Vec<Vec<Vec2>> = a.holes.into_iter().map(|h| h.pts).collect();
                (a.outer.pts, (!holes.is_empty()).then_some(holes))
            })
            .collect(),
    )
}

fn ring_area(pts: &[Vec2]) -> f64 {
    let n = pts.len();
    (0..n)
        .map(|i| {
            let (p, q) = (pts[i], pts[(i + 1) % n]);
            p.x * q.y - q.x * p.y
        })
        .sum::<f64>()
        / 2.0
}

fn area_of(a: &Area) -> f64 {
    ring_area(&a.outer.pts).abs() - a.holes.iter().map(|h| ring_area(&h.pts).abs()).sum::<f64>()
}

/// `d` cut by the columns `feet` standing in it (docs/adr/3041 §2); none when none of them reaches it.
pub fn cut(d: &Drawn, feet: &[&Foot]) -> Option<Drawn> {
    if feet.is_empty() {
        return None;
    }
    let mut out = d.clone();
    let mut changed = false;
    let mut strokes = |list: &mut Vec<Stroke>| {
        let mut next = Vec::with_capacity(list.len());
        for s in list.iter() {
            match cut_stroke(s, feet) {
                Some(parts) => {
                    changed = true;
                    next.extend(parts);
                }
                None => next.push(s.clone()),
            }
        }
        *list = next;
    };
    for line in &mut out.lines {
        strokes(line);
    }
    strokes(&mut out.recesses);
    let mut closures = Vec::with_capacity(out.closures.len());
    for (k, line) in &out.closures {
        let s = Stroke {
            line: line.clone(),
            closed: false,
        };
        match cut_stroke(&s, feet) {
            Some(parts) => {
                changed = true;
                closures.extend(parts.into_iter().map(|p| (*k, p.line)));
            }
            None => closures.push((*k, line.clone())),
        }
    }
    out.closures = closures;
    let mut caps = Vec::with_capacity(out.caps.len());
    for &(a, b) in &out.caps {
        match parts_outside(
            &[a, b],
            false,
            |p| feet.iter().any(|f| strictly_inside(p, &f.ring)),
            |s, e, o| feet.iter().for_each(|f| crossings(s, e, &f.ring, o)),
        ) {
            Some(parts) => {
                changed = true;
                caps.extend(
                    parts
                        .into_iter()
                        .filter_map(|p| Some((*p.first()?, *p.last()?))),
                );
            }
            None => caps.push((a, b)),
        }
    }
    out.caps = caps;
    let mut bodies = |list: &mut Vec<Body>| {
        let mut next = Vec::with_capacity(list.len());
        for body in list.iter() {
            match cut_body(body, feet) {
                Some(left) => {
                    changed = true;
                    next.extend(left);
                }
                None => next.push(body.clone()),
            }
        }
        *list = next;
    };
    for band in &mut out.bands {
        bodies(band);
    }
    bodies(&mut out.fill);
    changed.then_some(out)
}

/// The look a column takes from the walls it stands in (docs/adr/3041 §2): the layer of its material it meets.
#[derive(Clone, Debug, PartialEq)]
pub struct Joined {
    /// The wall's schematic fill (its `fill`), drawn over the column in the schematic view as over the wall.
    pub fill: Option<String>,
    /// The layer's hatch, the least detail it is drawn at, and its wall's first direction, degrees (the hatch turns
    /// with it).
    pub hatch: Option<(ComponentFill, Detail, f64)>,
    /// What is drawn of its outline: the parts not inside a layer of its material.
    pub outline: Vec<Vec<Vec2>>,
}

/// One wall as a column meets it: its drawing as its joins left it, before the columns cut it; its fields; its first
/// direction, degrees.
pub struct WallAt<'a> {
    pub drawn: &'a Drawn,
    pub spec: &'a Spec,
    pub turn: f64,
}

/// Whether the column covers some of the body.
fn covers(foot: &Foot, body: &Body) -> bool {
    if !boxes_meet(&foot.bounds, &bounds_of(&body.0), EPS) {
        return false;
    }
    let both = crate::geom::region::intersect_areas(&[
        foot.area(),
        Area {
            outer: Ring {
                pts: body.0.clone(),
                bulges: None,
            },
            holes: body
                .1
                .iter()
                .flatten()
                .map(|h| Ring {
                    pts: h.clone(),
                    bulges: None,
                })
                .collect(),
        },
    ]);
    both.iter().map(area_of).sum::<f64>() > 1e-10
}

/// A look found: the wall's fill and its layer's hatch, least detail and turn ([`Joined`]'s).
type Look = (Option<String>, Option<(ComponentFill, Detail, f64)>);

/// What column `foot` takes from the walls `walls` on its level (docs/adr/3041 §2); none when it meets no layer of its
/// material. The first such layer in the walls' order gives its hatch and fill.
pub fn joined(foot: &Foot, walls: &[WallAt<'_>]) -> Option<Joined> {
    let mut look: Option<Look> = None;
    let mut regions: Vec<&Body> = Vec::new();
    for w in walls {
        if w.drawn.bands.len() != w.spec.components.len() {
            continue;
        }
        for (k, c) in w.spec.components.iter().enumerate() {
            let Some(m) = c.material.as_deref() else {
                continue;
            };
            if material_key(m) != foot.material {
                continue;
            }
            for body in &w.drawn.bands[k] {
                if covers(foot, body) {
                    regions.push(body);
                    if look.is_none() {
                        look = Some((
                            w.spec.fill.clone(),
                            c.hatch.clone().map(|h| (h, c.detail, w.turn)),
                        ));
                    }
                }
            }
        }
    }
    let (fill, hatch) = look?;
    let outline = parts_outside(
        &foot.ring,
        true,
        |p| regions.iter().any(|b| inside_body(p, b)),
        |a, b, out| {
            for body in &regions {
                crossings(a, b, &body.0, out);
                for h in body.1.iter().flatten() {
                    crossings(a, b, h, out);
                }
            }
        },
    )
    .unwrap_or_else(|| {
        let mut whole = foot.ring.clone();
        whole.push(foot.ring[0]);
        vec![whole]
    });
    Some(Joined {
        fill,
        hatch,
        outline,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geom::wall::{Component, drawn};

    fn column(x: f64, y: f64, material: Option<&str>) -> Foot {
        Foot::of(
            Vec2::new(x, y),
            &ColumnSpec {
                width: 0.4,
                depth: Some(0.4),
                material: material.map(str::to_owned),
                ..Default::default()
            },
        )
        .expect("a column")
    }

    fn wall(components: Vec<Component>, width: f64) -> Spec {
        Spec {
            width,
            components,
            ..Default::default()
        }
    }

    fn layer(material: &str, pattern: &str) -> Component {
        Component {
            material: Some(material.into()),
            hatch: Some(ComponentFill {
                pattern: pattern.into(),
                scale: 1.0,
                angle: 0.0,
                color: None,
            }),
            ..Default::default()
        }
    }

    fn length(parts: &[Vec<Vec2>]) -> f64 {
        parts
            .iter()
            .flat_map(|p| p.windows(2))
            .map(|w| js_hypot(w[1].x - w[0].x, w[1].y - w[0].y))
            .sum()
    }

    /// A 20 cm brick wall from (0, 0) to (4, 0) into a 40 × 40 column at (4, 0): its faces stop at the column's face
    /// (x 3.8), its cap there goes, its band loses the column's part; the column keeps its whole outline.
    #[test]
    fn a_wall_stops_at_the_column_s_face() {
        let pts = [Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0)];
        let spec = wall(vec![layer("Tuğla", "ANSI31")], 0.2);
        let d = drawn(&pts, None, &spec).unwrap();
        let foot = column(4.0, 0.0, None);
        let c = cut(&d, &[&foot]).expect("the column reaches it");
        let faces: Vec<f64> = c.lines.iter().map(|l| l[0].line.pts[1].x).collect();
        assert!(
            faces.len() == 2 && faces.iter().all(|x| (x - 3.8).abs() < 1e-12),
            "both faces stop at its face: {faces:?}"
        );
        assert_eq!(c.caps.len(), 1, "the cap in the column goes: {:?}", c.caps);
        let band: f64 = c.bands[0].iter().map(|b| ring_area(&b.0).abs()).sum();
        assert!((band - 3.8 * 0.2).abs() < 1e-9, "{band}");
        // Of another material: no look, the line between stays.
        let at = [WallAt {
            drawn: &d,
            spec: &spec,
            turn: 0.0,
        }];
        assert_eq!(joined(&foot, &at), None);
        // Out of reach: nothing changes.
        assert_eq!(cut(&d, &[&column(9.0, 0.0, None)]), None);
    }

    /// A 25 cm concrete wall into the same column: one section. The column takes the wall's solid fill and hatch; the
    /// side of its outline across the wall's end (x 3.8, from −0.125 to 0.125) is not drawn: 1.6 − 0.25 = 1.35 m
    /// drawn of its 1.6 m.
    #[test]
    fn a_column_and_a_wall_of_its_material_are_one_section() {
        let pts = [Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0)];
        let mut spec = wall(vec![layer("BETONARME", "SOLID")], 0.25);
        spec.fill = Some("#3C3C3C".into());
        let d = drawn(&pts, None, &spec).unwrap();
        let foot = column(4.0, 0.0, Some("Betonarme"));
        let at = [WallAt {
            drawn: &d,
            spec: &spec,
            turn: 0.0,
        }];
        let j = joined(&foot, &at).expect("one material");
        assert_eq!(j.fill.as_deref(), Some("#3C3C3C"));
        assert_eq!(
            j.hatch.as_ref().map(|h| h.0.pattern.as_str()),
            Some("SOLID")
        );
        assert!((length(&j.outline) - 1.35).abs() < 1e-9, "{:?}", j.outline);
        // A steel column in the same wall: no look.
        assert_eq!(joined(&column(4.0, 0.0, Some("Çelik")), &at), None);
    }

    /// A column in the middle of a wall splits it: two faces each side, its band in two.
    #[test]
    fn a_column_in_a_wall_s_middle_splits_it() {
        let pts = [Vec2::new(0.0, 0.0), Vec2::new(4.0, 0.0)];
        let spec = wall(vec![layer("Tuğla", "ANSI31")], 0.2);
        let d = drawn(&pts, None, &spec).unwrap();
        let c = cut(&d, &[&column(2.0, 0.0, None)]).unwrap();
        for l in &c.lines {
            let ends: Vec<(f64, f64)> = l
                .iter()
                .map(|s| (s.line.pts[0].x, s.line.pts[s.line.pts.len() - 1].x))
                .collect();
            assert_eq!(ends.len(), 2, "{ends:?}");
            assert!(
                ends.iter().any(|e| (e.1 - 1.8).abs() < 1e-12)
                    && ends.iter().any(|e| (e.0 - 2.2).abs() < 1e-12),
                "{ends:?}"
            );
        }
        assert_eq!(c.bands[0].len(), 2);
        assert_eq!(c.caps.len(), 2, "its own two caps stay");
    }
}
