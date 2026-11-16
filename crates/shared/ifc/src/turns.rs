//! A stair's and a ramp's predefined type (docs/adr/8070 §2; EK-6 Tablo 6.49 `IfcRampTypeEnum`, Tablo 6.61
//! `IfcStairTypeEnum`) from how its walking line goes: how many flights it has and how far it turns at each stop
//! between two (a landing, or a balanced stair's winders), in quarter turns. A line the enumeration has no item for
//! (three straight flights, a turn that is not a right angle) is `USERDEFINED`, its object type saying what it is: a
//! type is never guessed, and `NOTDEFINED` is no type (EK-6).

use kentos_contracts::{RampFields, StairFields};
use kentos_geometry_core::Vec2 as P;
use kentos_geometry_core::geom::arch::{
    RampSpec, StairSpec, Walk, ramp_landings, stair_landings, winder_turns,
};

/// Closer than this (radians, one degree) to a multiple of a right angle, a turn is that many quarter turns.
const SQUARE: f64 = core::f64::consts::PI / 180.0;
/// Two walking distances closer than this are one, metres.
const EPS: f64 = 1e-9;

/// A stop between two flights: how far the line turns there in quarter turns (+ to its left) and whether its treads
/// wind round the turn (a balanced stair's) instead of standing on a landing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Stop {
    pub quarters: i32,
    pub winding: bool,
}

/// How a walking line goes: its flights and the stops between them, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Course {
    pub flights: usize,
    pub stops: Vec<Stop>,
}

/// The course of the line through `pts` with its stops over `stops` (each a stretch of walking distance and whether it
/// winds), in order; overlapping stops are one. None where the line turns by other than right angles, or turns where
/// it has no stop (its flight would turn).
pub fn course(pts: &[P], stops: &[((f64, f64), bool)]) -> Option<Course> {
    let n = pts.len();
    if n < 2 {
        return None;
    }
    let walk = Walk::of(pts);
    let length = walk.length();
    let mut merged: Vec<((f64, f64), bool)> = Vec::new();
    for &((a, b), winding) in stops {
        match merged.last_mut() {
            Some(((_, end), w)) if a < *end + EPS => {
                *end = end.max(b);
                *w |= winding;
            }
            _ => merged.push(((a, b), winding)),
        }
    }
    let mut quarters = vec![0i32; merged.len()];
    for k in 1..n - 1 {
        let u = (pts[k].x - pts[k - 1].x, pts[k].y - pts[k - 1].y);
        let v = (pts[k + 1].x - pts[k].x, pts[k + 1].y - pts[k].y);
        let turn = libm::atan2(u.0 * v.1 - u.1 * v.0, u.0 * v.0 + u.1 * v.1);
        let q = libm::round(turn / core::f64::consts::FRAC_PI_2);
        if (turn - q * core::f64::consts::FRAC_PI_2).abs() > SQUARE {
            return None;
        }
        let q = q as i32;
        let at = walk.at[k];
        match merged
            .iter()
            .position(|((a, b), _)| at >= a - EPS && at <= b + EPS)
        {
            Some(i) => quarters[i] += q,
            None if q != 0 => return None,
            None => {}
        }
    }
    let mut flights = 0;
    let mut start = 0.0;
    for ((a, b), _) in &merged {
        if a - start > EPS {
            flights += 1;
        }
        start = *b;
    }
    if length - start > EPS {
        flights += 1;
    }
    Some(Course {
        flights,
        stops: merged
            .iter()
            .zip(quarters)
            .map(|((_, winding), quarters)| Stop {
                quarters,
                winding: *winding,
            })
            .collect(),
    })
}

/// A ramp's `IfcRampTypeEnum` item for its course; none where the enumeration has none.
pub fn ramp_type(c: &Course) -> Option<&'static str> {
    let q: Vec<i32> = c.stops.iter().map(|s| s.quarters).collect();
    Some(match (c.flights, q.as_slice()) {
        (1, []) => "STRAIGHT_RUN_RAMP",
        (2, [0]) => "TWO_STRAIGHT_RUN_RAMP",
        (2, [1 | -1]) => "QUARTER_TURN_RAMP",
        (2, [2 | -2]) => "HALF_TURN_RAMP",
        (3, [a, b]) if a.abs() == 1 && a == b => "TWO_QUARTER_TURN_RAMP",
        _ => return None,
    })
}

/// A stair's `IfcStairTypeEnum` item for its course; none where the enumeration has none.
pub fn stair_type(c: &Course) -> Option<&'static str> {
    let quarters: Vec<i32> = c.stops.iter().map(|s| s.quarters).collect();
    // The same quarter turn at every stop, as many as `n`.
    let quarter_turns = |n: usize| quarters.len() == n && quarters.iter().all(|q| q.abs() == 1 && *q == quarters[0]);
    if !c.stops.is_empty() && c.stops.iter().all(|s| s.winding) {
        // Winders are steps of the one flight they turn (IFC: "a stair making a 90° turn, consisting of one flight
        // with quarter winders"): the flights between them are not counted.
        return Some(match quarters.as_slice() {
            [1 | -1] => "QUARTER_WINDING_STAIR",
            [2 | -2] => "HALF_WINDING_STAIR",
            _ if quarter_turns(2) => "TWO_QUARTER_WINDING_STAIR",
            _ if quarter_turns(3) => "THREE_QUARTER_WINDING_STAIR",
            _ => return None,
        });
    }
    if c.stops.iter().any(|s| s.winding) {
        return None;
    }
    Some(match (c.flights, quarters.as_slice()) {
        (1, []) => "STRAIGHT_RUN_STAIR",
        (2, [0]) => "TWO_STRAIGHT_RUN_STAIR",
        (2, [1 | -1]) => "QUARTER_TURN_STAIR",
        (2, [2 | -2]) => "HALF_TURN_STAIR",
        (3, _) if quarter_turns(2) => "TWO_QUARTER_TURN_STAIR",
        (4, _) if quarter_turns(3) => "THREE_QUARTER_TURN_STAIR",
        _ => return None,
    })
}

/// What a line of no item is, for `USERDEFINED`'s object type: “3 kollu, 2 sahanlıklı rampa”, “dik açı dışında dönen
/// merdiven”.
fn words(c: Option<&Course>, what: &str) -> String {
    match c {
        None => format!("dik açı dışında dönen {what}"),
        Some(c) => {
            let winding = c.stops.iter().filter(|s| s.winding).count();
            let landings = c.stops.len() - winding;
            let mut out = format!("{} kollu", c.flights);
            if landings > 0 {
                out.push_str(&format!(", {landings} sahanlıklı"));
            }
            if winding > 0 {
                out.push_str(&format!(", {winding} balansmanlı dönüşlü"));
            }
            format!("{out} {what}")
        }
    }
}

fn points(pts: &[kentos_contracts::Vec2]) -> Vec<P> {
    pts.iter().map(|p| P::new(p.x, p.y)).collect()
}

/// A ramp's predefined type and, when it is `USERDEFINED`, its object type.
pub fn of_ramp(r: &RampFields) -> (&'static str, Option<String>) {
    let pts = points(&r.pts);
    let spec = RampSpec {
        width: r.width,
        rise: r.rise,
        slope: r.slope,
        base: r.base,
        landing: r.landing,
        railings: None,
    };
    let walk = Walk::of(&pts);
    let stops: Vec<((f64, f64), bool)> = ramp_landings(&walk, &spec)
        .into_iter()
        .map(|l| (l, false))
        .collect();
    let c = course(&pts, &stops);
    match c.as_ref().and_then(ramp_type) {
        Some(t) => (t, None),
        None => ("USERDEFINED", Some(words(c.as_ref(), "rampa"))),
    }
}

/// A stair's predefined type and, when it is `USERDEFINED`, its object type. A curved stair is `SPIRAL_STAIR` round its
/// centre post, else `CURVED_RUN_STAIR` (one curved flight).
pub fn of_stair(s: &StairFields) -> (&'static str, Option<String>) {
    let bulge = s.bulge();
    if bulge != 0.0 {
        return (
            if s.post.is_some() {
                "SPIRAL_STAIR"
            } else {
                "CURVED_RUN_STAIR"
            },
            None,
        );
    }
    let pts = points(&s.pts);
    let spec = StairSpec {
        bulge,
        width: s.width,
        risers: f64::from(s.risers),
        riser_height: s.riser_height,
        tread: s.tread,
        landing: s.landing,
        cut: None,
        numbering: false,
        railings: None,
        post: s.post,
        upper: false,
        winders: s.winders.map(f64::from),
    };
    let walk = Walk::of(&pts);
    let mut stops: Vec<((f64, f64), bool)> = stair_landings(&walk, &spec)
        .into_iter()
        .map(|l| (l, false))
        .collect();
    stops.extend(
        winder_turns(&walk, &spec)
            .iter()
            .map(|t| ((t.corner - t.reach, t.corner + t.reach), true)),
    );
    stops.sort_by(|a, b| a.0.0.total_cmp(&b.0.0));
    let c = course(&pts, &stops);
    match c.as_ref().and_then(stair_type) {
        Some(t) => (t, None),
        None => ("USERDEFINED", Some(words(c.as_ref(), "merdiven"))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: f64, y: f64) -> P {
        P::new(x, y)
    }

    #[test]
    fn a_straight_line_is_one_flight() {
        let c = course(&[p(0.0, 0.0), p(6.0, 0.0)], &[]).unwrap();
        assert_eq!(c, Course { flights: 1, stops: vec![] });
        assert_eq!(ramp_type(&c), Some("STRAIGHT_RUN_RAMP"));
        assert_eq!(stair_type(&c), Some("STRAIGHT_RUN_STAIR"));
    }

    #[test]
    fn a_corner_with_its_landing_is_a_quarter_turn_and_a_u_a_half_or_two_quarters() {
        // L: 6 m east, 4 m north, a 1.5 m landing round the corner.
        let l = course(&[p(0.0, 0.0), p(6.0, 0.0), p(6.0, 4.0)], &[((5.25, 6.75), false)]).unwrap();
        assert_eq!(l, Course { flights: 2, stops: vec![Stop { quarters: 1, winding: false }] });
        assert_eq!(ramp_type(&l), Some("QUARTER_TURN_RAMP"));
        // U whose landings touch: one stop turning twice to the left.
        let u = course(
            &[p(0.0, 0.0), p(6.0, 0.0), p(6.0, 1.5), p(0.0, 1.5)],
            &[((5.25, 6.75), false), ((6.75, 8.25), false)],
        )
        .unwrap();
        assert_eq!(u, Course { flights: 2, stops: vec![Stop { quarters: 2, winding: false }] });
        assert_eq!(ramp_type(&u), Some("HALF_TURN_RAMP"));
        assert_eq!(stair_type(&u), Some("HALF_TURN_STAIR"));
        // U with a sloped middle: two quarter landings and three flights.
        let wide = course(
            &[p(0.0, 0.0), p(6.0, 0.0), p(6.0, 3.0), p(0.0, 3.0)],
            &[((5.25, 6.75), false), ((8.25, 9.75), false)],
        )
        .unwrap();
        assert_eq!(wide.flights, 3);
        assert_eq!(ramp_type(&wide), Some("TWO_QUARTER_TURN_RAMP"));
    }

    #[test]
    fn a_landing_on_a_straight_line_makes_two_straight_runs_and_an_odd_angle_has_no_item() {
        let two = course(&[p(0.0, 0.0), p(9.0, 0.0), p(18.0, 0.0)], &[((8.25, 9.75), false)]).unwrap();
        assert_eq!(ramp_type(&two), Some("TWO_STRAIGHT_RUN_RAMP"));
        assert_eq!(stair_type(&two), Some("TWO_STRAIGHT_RUN_STAIR"));
        assert_eq!(course(&[p(0.0, 0.0), p(6.0, 0.0), p(9.0, 3.0)], &[((5.0, 7.0), false)]), None);
        let three = course(
            &[p(0.0, 0.0), p(9.0, 0.0), p(18.0, 0.0), p(27.0, 0.0)],
            &[((8.25, 9.75), false), ((17.25, 18.75), false)],
        )
        .unwrap();
        assert_eq!(ramp_type(&three), None);
        assert_eq!(words(Some(&three), "rampa"), "3 kollu, 2 sahanlıklı rampa");
    }

    #[test]
    fn winders_name_the_winding_items() {
        let c = course(&[p(0.0, 0.0), p(3.0, 0.0), p(3.0, 3.0)], &[((2.0, 4.0), true)]).unwrap();
        assert_eq!(stair_type(&c), Some("QUARTER_WINDING_STAIR"));
        let right = course(
            &[p(0.0, 0.0), p(3.0, 0.0), p(3.0, -3.0), p(0.0, -3.0)],
            &[((2.0, 4.0), true), ((5.0, 7.0), true)],
        )
        .unwrap();
        assert_eq!(right.stops[0].quarters, -1);
        assert_eq!(stair_type(&right), Some("TWO_QUARTER_WINDING_STAIR"));
        // Winders reaching both ends: still one flight turning a quarter (IFC counts winders as its steps).
        let whole = course(&[p(0.0, 0.0), p(1.0, 0.0), p(1.0, 1.0)], &[((0.0, 2.0), true)]).unwrap();
        assert_eq!(whole.flights, 0);
        assert_eq!(stair_type(&whole), Some("QUARTER_WINDING_STAIR"));
        // A landing and winders on one stair: no item.
        let mixed = course(
            &[p(0.0, 0.0), p(3.0, 0.0), p(3.0, 3.0), p(0.0, 3.0)],
            &[((2.0, 4.0), true), ((5.25, 6.75), false)],
        )
        .unwrap();
        assert_eq!(stair_type(&mixed), None);
    }
}
