//! The drawing's grids as IFC 4.3 grids (EK-6 Tablo 6.36; docs/adr/8070 §4, docs/adr/8169). Each storey's axes (none:
//! the building's, shown on every storey) make a grid of each radial system among them and one of the rest.
//!
//! A radial system is the arcs about one centre and the straight axes running out from it (IFC 4.3 `IfcGridTypeEnum`
//! RADIAL: the U axes meet at the centre and turn counter-clockwise, the V axes are concentric arcs): U the straight
//! ones counter-clockwise from the first after the widest gap round the centre, V the arcs from the centre out. Of
//! the rest, the lines parallel to the first straight one are U, the others V; rectangular when each set is parallel
//! and the two square to each other, else irregular. Axes in one direction only make no grid (IFC 4.3 asks both
//! sets): they are left out, said.

use kentos_contracts::{Entity, GridFields};

use crate::arch::{Left, Why, Written};
use crate::building::{Axis, Grid};
use crate::naming::name;

/// How near two points must be to be one, metres (two arcs' centres, a straight axis's line and a centre): the
/// project's spatial tolerance (docs/specs/numeric-policy.md §5).
const SAME_POINT: f64 = 0.001;

/// An axis as the drawing has it: its persistent id, its axis, its direction when straight, its centre and radius when
/// an arc.
struct Drawn<'a> {
    uid: &'a str,
    axis: Axis,
    direction: Option<[f64; 2]>,
    circle: Option<([f64; 2], f64)>,
}

/// The drawing's grids, storey by storey; what is left out is said in `written`.
pub(crate) fn grids_of(
    entities: &[(u128, String, &Entity)],
    storey_of: &dyn Fn(Option<&str>) -> usize,
    written: &mut Written,
) -> Vec<Grid> {
    let mut by_level: Vec<(Option<String>, Vec<(u128, &String, &GridFields)>)> = Vec::new();
    for (bits, uid, e) in entities {
        let Entity::Grid(g) = e else { continue };
        let level = g.grid.arch.level.clone();
        match by_level.iter_mut().find(|(l, _)| *l == level) {
            Some((_, list)) => list.push((*bits, uid, &g.grid)),
            None => by_level.push((level, vec![(*bits, uid, &g.grid)])),
        }
    }
    let mut out = Vec::new();
    for (level, list) in by_level {
        let drawn: Vec<Drawn> = list.iter().filter_map(|(bits, uid, g)| drawn(*bits, uid, g)).collect();
        let storey = level.as_deref().map(|l| storey_of(Some(l)));
        let (grids, left) = grids(&drawn, storey);
        written.grid_axes += grids.iter().map(|g| g.u.len() + g.v.len()).sum::<usize>();
        for uid in left {
            written.left.push(Left { id: uid.to_owned(), what: "aks", why: Why::OneWay });
        }
        out.extend(grids);
    }
    out
}

/// A grid object's axis: its line, or its arc through its middle with the arc's circle; none when its ends meet.
fn drawn<'a>(bits: u128, uid: &'a str, g: &GridFields) -> Option<Drawn<'a>> {
    let (a, b) = (g.pts.first()?, g.pts.get(1)?);
    let (dx, dy) = (b.x - a.x, b.y - a.y);
    let len = crate::math::hypot(dx, dy);
    if len < 1e-9 {
        return None;
    }
    let bulge = g.bulges.as_ref().and_then(|v| v.first().copied()).unwrap_or(0.0);
    let middle = [(a.x + b.x) / 2.0, (a.y + b.y) / 2.0];
    // An arc's middle: the chord's middle moved by the sagitta, to the chord's right for a positive bulge; its centre
    // on the chord's other side, (1 − b²)/(4b) of the chord's normal (dy, −dx) from the chord's middle.
    let through = (bulge != 0.0).then(|| [middle[0] + dy * bulge / 2.0, middle[1] - dx * bulge / 2.0]);
    let circle = (bulge != 0.0).then(|| {
        let k = (1.0 - bulge * bulge) / (4.0 * bulge);
        let c = [middle[0] - dy * k, middle[1] + dx * k];
        (c, crate::math::hypot(a.x - c[0], a.y - c[1]))
    });
    Some(Drawn {
        uid,
        axis: Axis { id: Some(bits), tag: g.label.trim().to_owned(), from: [a.x, a.y], to: [b.x, b.y], through },
        direction: (bulge == 0.0).then_some([dx / len, dy / len]),
        circle,
    })
}

/// A storey's grids: its radial systems' (in the order of their first arcs), after the rest's; and the axes left out.
fn grids<'a>(drawn: &[Drawn<'a>], storey: Option<usize>) -> (Vec<Grid>, Vec<&'a str>) {
    let mut taken = vec![false; drawn.len()];
    let mut radial = Vec::new();
    // The arcs about one centre: the first one's, the others within the tolerance of it.
    let mut centres: Vec<([f64; 2], Vec<usize>)> = Vec::new();
    for (k, d) in drawn.iter().enumerate() {
        let Some((c, _)) = d.circle else { continue };
        match centres.iter_mut().find(|(o, _)| crate::math::hypot(o[0] - c[0], o[1] - c[1]) <= SAME_POINT) {
            Some((_, arcs)) => arcs.push(k),
            None => centres.push((c, vec![k])),
        }
    }
    for (c, mut arcs) in centres {
        let mut rays: Vec<(f64, usize)> = (0..drawn.len())
            .filter(|&k| !taken[k])
            .filter_map(|k| out_from(&drawn[k], c).map(|angle| (angle, k)))
            .collect();
        if rays.is_empty() {
            continue;
        }
        // Counter-clockwise, from the first after the widest gap between neighbours round the centre.
        rays.sort_by(|a, b| a.0.total_cmp(&b.0));
        let n = rays.len();
        let gap = |i: usize| {
            if i == 0 { rays[0].0 + std::f64::consts::TAU - rays[n - 1].0 } else { rays[i].0 - rays[i - 1].0 }
        };
        let first = (0..n).fold(0, |best, i| if gap(i) > gap(best) { i } else { best });
        rays.rotate_left(first);
        let radius = |k: usize| drawn[k].circle.map_or(0.0, |(_, r)| r);
        arcs.sort_by(|&a, &b| radius(a).total_cmp(&radius(b)));
        let members: Vec<usize> = rays.iter().map(|&(_, k)| k).chain(arcs.iter().copied()).collect();
        for &k in &members {
            taken[k] = true;
        }
        radial.push(Grid {
            id: Some(crate::step::fnv(seed(drawn, &members), 3)),
            name: name("AKS", "Eğrisel aks"),
            storey,
            u: rays.iter().map(|&(_, k)| drawn[k].axis.clone()).collect(),
            v: arcs.iter().map(|&k| drawn[k].axis.clone()).collect(),
            predefined: "RADIAL",
        });
    }
    let rest: Vec<usize> = (0..drawn.len()).filter(|&k| !taken[k]).collect();
    let mut out = Vec::new();
    let mut left = Vec::new();
    if !rest.is_empty() {
        let cross = |a: [f64; 2], b: [f64; 2]| a[0] * b[1] - a[1] * b[0];
        let parallel =
            |a: Option<[f64; 2]>, b: Option<[f64; 2]>| a.zip(b).is_some_and(|(a, b)| cross(a, b).abs() < 1e-6);
        let first = rest.iter().find_map(|&k| drawn[k].direction);
        let (u, v): (Vec<usize>, Vec<usize>) = rest.iter().partition(|&&k| parallel(first, drawn[k].direction));
        if u.is_empty() || v.is_empty() {
            left.extend(rest.iter().map(|&k| drawn[k].uid));
        } else {
            let across = v.iter().find_map(|&k| drawn[k].direction);
            let rectangular = v.iter().all(|&k| parallel(across, drawn[k].direction))
                && first.zip(across).is_some_and(|(a, b)| (a[0] * b[0] + a[1] * b[1]).abs() < 1e-6);
            out.push(Grid {
                id: Some(crate::step::fnv(seed(drawn, &rest), 3)),
                name: name("AKS", "Aks"),
                storey,
                u: u.iter().map(|&k| drawn[k].axis.clone()).collect(),
                v: v.iter().map(|&k| drawn[k].axis.clone()).collect(),
                predefined: if rectangular { "RECTANGULAR" } else { "IRREGULAR" },
            });
        }
    }
    out.extend(radial);
    (out, left)
}

/// A grid's GlobalId seed: the least persistent id among its axes'.
fn seed<'a>(drawn: &[Drawn<'a>], members: &[usize]) -> &'a str {
    members.iter().map(|&k| drawn[k].uid).min().unwrap_or_default()
}

/// The angle (radians, counter-clockwise from east) a straight axis runs out from `c` at, when its line passes through
/// `c` and `c` is at or past one of its ends; none otherwise.
fn out_from(d: &Drawn, c: [f64; 2]) -> Option<f64> {
    let u = d.direction?;
    let (p, q) = (d.axis.from, d.axis.to);
    let w = [c[0] - p[0], c[1] - p[1]];
    let len = crate::math::hypot(q[0] - p[0], q[1] - p[1]);
    let across = (u[0] * w[1] - u[1] * w[0]).abs();
    let along = u[0] * w[0] + u[1] * w[1];
    if across > SAME_POINT || (along > SAME_POINT && along < len - SAME_POINT) {
        return None;
    }
    let far = if along <= SAME_POINT { q } else { p };
    Some(crate::math::atan2(far[1] - c[1], far[0] - c[0]))
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A straight axis from `a` to `b`, or an arc of `bulge`.
    fn axis(uid: &'static str, tag: &str, a: [f64; 2], b: [f64; 2], bulge: f64) -> Drawn<'static> {
        let g = GridFields {
            pts: vec![kentos_contracts::Vec2 { x: a[0], y: a[1] }, kentos_contracts::Vec2 { x: b[0], y: b[1] }],
            bulges: (bulge != 0.0).then(|| vec![bulge, 0.0]),
            label: tag.to_owned(),
            ..GridFields::default()
        };
        drawn(1, uid, &g).expect("an axis")
    }

    /// A fan about `c`: radial axes at `angles` (degrees) from 2.5 m to 9.5 m out, arcs of radius 4 and 8 from −70° to
    /// 70°, counter-clockwise when `ccw`, else drawn the other way.
    fn fan(c: [f64; 2], angles: &[f64], ccw: bool) -> Vec<Drawn<'static>> {
        let at = |r: f64, deg: f64| {
            let t = deg.to_radians();
            [c[0] + r * crate::math::cos(t), c[1] + r * crate::math::sin(t)]
        };
        let tags = ["4", "5", "6", "7", "8"];
        let uids = ["r4", "r5", "r6", "r7", "r8"];
        let mut out: Vec<Drawn> =
            angles.iter().enumerate().map(|(i, &d)| axis(uids[i], tags[i], at(2.5, d), at(9.5, d), 0.0)).collect();
        // A 140° arc's bulge: tan(140°/4); clockwise when drawn from its end back.
        let bulge = libm::tan(140f64.to_radians() / 4.0);
        for (uid, tag, r) in [("aF", "F", 8.0), ("aE", "E", 4.0)] {
            let (s, e) = (at(r, -70.0), at(r, 70.0));
            out.push(if ccw { axis(uid, tag, s, e, bulge) } else { axis(uid, tag, e, s, -bulge) });
        }
        out
    }

    fn tags(list: &[Axis]) -> Vec<&str> {
        list.iter().map(|a| a.tag.as_str()).collect()
    }

    #[test]
    fn a_fan_of_axes_about_one_centre_is_a_radial_grid() {
        // Drawn clockwise, from 60° down: still U counter-clockwise from −60°, V from the centre out.
        let c = [512_345.678, 4_412_345.678];
        let mut drawn = fan(c, &[60.0, 30.0, 0.0, -30.0, -60.0], false);
        // The building's own axes beside it: one through the centre (it passes it, so it is not the fan's), one across.
        drawn.push(axis("g2", "2", [c[0] - 14.0, c[1]], [c[0] + 2.0, c[1]], 0.0));
        drawn.push(axis("gD", "D", [c[0], c[1] - 6.5], [c[0], c[1] + 6.5], 0.0));
        let (made, left) = grids(&drawn, None);
        assert!(left.is_empty());
        assert_eq!(made.len(), 2);
        let (rest, fan) = (&made[0], &made[1]);
        assert_eq!((rest.predefined, tags(&rest.u), tags(&rest.v)), ("RECTANGULAR", vec!["2"], vec!["D"]));
        assert_eq!(fan.predefined, "RADIAL");
        assert_eq!(fan.name, name("AKS", "Eğrisel aks"));
        assert_eq!(tags(&fan.u), ["8", "7", "6", "5", "4"]);
        assert_eq!(tags(&fan.v), ["E", "F"]);
        // Each arc's centre is the fan's, its middle on it at 0°.
        for (axis, r) in fan.v.iter().zip([4.0, 8.0]) {
            let m = axis.through.expect("an arc");
            assert!((m[0] - (c[0] + r)).abs() < 1e-6 && (m[1] - c[1]).abs() < 1e-6, "{m:?}");
        }
        // The two grids' GlobalIds are their own axes' least ids'.
        assert_eq!((rest.id, fan.id), (Some(crate::step::fnv("g2", 3)), Some(crate::step::fnv("aE", 3))));
    }

    #[test]
    fn a_fan_across_east_starts_after_its_widest_gap() {
        // From 300° on round to 60°: counter-clockwise the first is the one at 300° (−60°).
        let drawn = fan([0.0, 0.0], &[0.0, 60.0, 300.0, 330.0, 30.0], true);
        let (made, _) = grids(&drawn, Some(1));
        assert_eq!(made.len(), 1);
        assert_eq!((made[0].storey, tags(&made[0].u)), (Some(1), vec!["6", "7", "4", "8", "5"]));
    }

    #[test]
    fn arcs_with_no_axis_out_of_their_centre_or_off_it_stay_with_the_rest() {
        // A straight axis 2 mm off the centre's line is not the fan's; the arcs have no straight axis of their own.
        let c = [10.0, 10.0];
        let mut drawn: Vec<Drawn> = fan(c, &[0.0], true).into_iter().skip(1).collect();
        drawn.push(axis("x", "X", [c[0] + 2.5, c[1] + 0.002], [c[0] + 9.5, c[1] + 0.002], 0.0));
        let (made, left) = grids(&drawn, None);
        assert!(left.is_empty());
        assert_eq!(made.len(), 1);
        assert_eq!((made[0].predefined, tags(&made[0].u), tags(&made[0].v)), ("IRREGULAR", vec!["X"], vec!["F", "E"]));
        // One way only: left out, said.
        let one = vec![axis("a", "A", [0.0, 0.0], [0.0, 5.0], 0.0), axis("b", "B", [3.0, 0.0], [3.0, 5.0], 0.0)];
        let (made, left) = grids(&one, None);
        assert!(made.is_empty());
        assert_eq!(left, ["a", "b"]);
    }
}
