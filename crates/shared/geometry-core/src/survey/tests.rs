//! Survey computations against the points they were measured from: angles
//! and distances are made from known points, the computation must give the
//! points back; errors put into the measurements must come out as the
//! misclosures. The independent decimal reference is in
//! fixtures/geometry/v1/reference-calls.json (tests/calls.rs).

use super::intersection::{forward_intersection, resection};
use super::polar::{PolarInput, Shot, StakeoutInput, polar_survey, stakeout};
use super::traverse::{TraverseInput, traverse};
use super::*;

struct Rng(u64);
impl Rng {
    fn next(&mut self) -> f64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 >> 11) as f64 / (1u64 << 53) as f64
    }
    fn range(&mut self, a: f64, b: f64) -> f64 {
        a + (b - a) * self.next()
    }
}

const TM: Vec2 = Vec2 {
    x: 486_512.34,
    y: 4_420_187.52,
};

fn near(a: Vec2, b: Vec2, tol: f64) -> bool {
    distance(a, b) <= tol
}

/// Clockwise angle at `p` from `u` to `v`, in the unit.
fn angle(unit: Unit, p: Vec2, u: Vec2, v: Vec2) -> f64 {
    unit.of(positive(bearing(p, v) - bearing(p, u)))
}

/// A walk of `n` legs from TM, turning a little each time.
fn walk(rng: &mut Rng, n: usize) -> Vec<Vec2> {
    let mut pts = vec![TM];
    let mut t = rng.range(0.0, TAU);
    for _ in 0..n {
        t += rng.range(-1.2, 1.2);
        let s = rng.range(40.0, 400.0);
        pts.push(from_bearing(*pts.last().unwrap_or(&TM), t, s));
    }
    pts
}

/// The measurements of a traverse through `pts` (start, new points, end), oriented on `back` and `fore`.
fn measure(unit: Unit, pts: &[Vec2], back: Vec2, fore: Option<Vec2>) -> (Vec<f64>, Vec<f64>) {
    let mut angles = Vec::new();
    for i in 0..pts.len() - 1 {
        let prev = if i == 0 { back } else { pts[i - 1] };
        angles.push(angle(unit, pts[i], prev, pts[i + 1]));
    }
    if let Some(f) = fore {
        let n = pts.len();
        angles.push(angle(unit, pts[n - 1], pts[n - 2], f));
    }
    let dists = pts.windows(2).map(|w| distance(w[0], w[1])).collect();
    (angles, dists)
}

#[test]
fn a_connected_traverse_gives_back_its_points() {
    let mut rng = Rng(0x5eed_5001);
    for round in 0..500 {
        let unit = if round % 2 == 0 {
            Unit::GRAD
        } else {
            Unit::DEG
        };
        let pts = walk(&mut rng, 2 + round % 9);
        let back = from_bearing(TM, rng.range(0.0, TAU), 300.0);
        let end = *pts.last().unwrap();
        let fore = from_bearing(end, rng.range(0.0, TAU), 250.0);
        let (angles, distances) = measure(unit, &pts, back, Some(fore));
        let r = traverse(&TraverseInput {
            unit: if round % 2 == 0 { "grad" } else { "deg" }.into(),
            start: TM,
            back,
            end: Some(end),
            fore: Some(fore),
            angles,
            distances,
        })
        .unwrap();
        assert_eq!(r.points.len(), pts.len() - 2);
        for (got, want) in r.points.iter().zip(&pts[1..]) {
            assert!(near(*got, *want, 1e-7), "round {round}: {got:?} ≠ {want:?}");
        }
        assert!(
            r.angle_misclosure.unwrap().abs() < 1e-9 && r.linear_misclosure.unwrap() < 1e-7,
            "{r:?}"
        );
    }
}

#[test]
fn misclosures_are_the_errors_put_in() {
    let unit = Unit::GRAD;
    let mut rng = Rng(0x5eed_5002);
    let pts = walk(&mut rng, 6);
    let back = from_bearing(TM, 1.0, 300.0);
    let end = pts[6];
    let fore = from_bearing(end, 2.0, 250.0);
    let (mut angles, mut distances) = measure(unit, &pts, back, Some(fore));
    // 0.0030 grad too much at the third station: the misclosure is +0.0030, each of 7 angles loses 0.0030 / 7.
    angles[2] += 0.003;
    let input = |angles: Vec<f64>, distances: Vec<f64>| TraverseInput {
        unit: "grad".into(),
        start: TM,
        back,
        end: Some(end),
        fore: Some(fore),
        angles,
        distances,
    };
    let r = traverse(&input(angles.clone(), distances.clone())).unwrap();
    assert!((r.angle_misclosure.unwrap() - 0.003).abs() < 1e-9, "{r:?}");
    assert!((r.angle_correction.unwrap() + 0.003 / 7.0).abs() < 1e-12);
    // After the angular adjustment the coordinates close exactly: the linear misclosure is what an
    // angle error leaves once spread, which the compass rule takes off (the end comes out exact).
    let last = r.points.iter().fold(TM, |_, p| *p);
    let leg = r.legs.last().unwrap();
    assert!(near(Vec2::new(last.x + leg.dy, last.y + leg.dx), end, 1e-7));
    // A distance 0.05 m too long on a leg along the X axis: fx grows by about that.
    let (angles, _) = measure(unit, &pts, back, Some(fore));
    let k = 3;
    distances[k] += 0.05;
    let r = traverse(&input(angles, distances.clone())).unwrap();
    let b = bearing(pts[k], pts[k + 1]);
    assert!((r.fy.unwrap() - 0.05 * crate::jsmath::sin(b)).abs() < 1e-9);
    assert!((r.fx.unwrap() - 0.05 * crate::jsmath::cos(b)).abs() < 1e-9);
    assert!((r.linear_misclosure.unwrap() - 0.05).abs() < 1e-9);
    // The corrections share it out by length and sum to the misclosure's opposite.
    let vy: f64 = r.legs.iter().map(|l| l.vy).sum();
    assert!((vy + r.fy.unwrap()).abs() < 1e-12);
    let total: f64 = distances.iter().sum();
    assert!((r.legs[0].vy + r.fy.unwrap() * distances[0] / total).abs() < 1e-15);
}

#[test]
fn closed_and_open_traverses() {
    let unit = Unit::DEG;
    // A square of 100 m walked counter-clockwise from TM, oriented on a point due west.
    let sq = [
        TM,
        Vec2::new(TM.x + 100.0, TM.y),
        Vec2::new(TM.x + 100.0, TM.y + 100.0),
        Vec2::new(TM.x, TM.y + 100.0),
        TM,
    ];
    let back = Vec2::new(TM.x - 50.0, TM.y);
    let (angles, distances) = measure(unit, &sq, back, Some(back));
    // Walked counter-clockwise, the clockwise angle from back to fore is the interior one.
    assert_eq!(angles, vec![180.0, 90.0, 90.0, 90.0, 270.0]);
    let r = traverse(&TraverseInput {
        unit: "deg".into(),
        start: TM,
        back,
        end: Some(TM),
        fore: Some(back),
        angles,
        distances,
    })
    .unwrap();
    assert_eq!(r.points.len(), 3);
    for (got, want) in r.points.iter().zip(&sq[1..4]) {
        assert!(near(*got, *want, 1e-9), "{got:?} ≠ {want:?}");
    }
    assert!((r.length - 400.0).abs() < 1e-12);
    // Open: no closure, every point new, nothing adjusted.
    let (angles, distances) = measure(unit, &sq[..4], back, None);
    let r = traverse(&TraverseInput {
        unit: "deg".into(),
        start: TM,
        back,
        end: None,
        fore: None,
        angles,
        distances,
    })
    .unwrap();
    assert_eq!(r.points.len(), 3);
    assert!(near(r.points[2], sq[3], 1e-9));
    assert!(r.angle_misclosure.is_none() && r.fy.is_none());
    // Connected without the end's orientation: coordinates close, angles are not compared.
    let (angles, distances) = measure(unit, &sq[..4], back, None);
    let r = traverse(&TraverseInput {
        unit: "deg".into(),
        start: TM,
        back,
        end: Some(sq[3]),
        fore: None,
        angles: angles[..3].to_vec(),
        distances,
    })
    .unwrap();
    assert_eq!(r.points.len(), 2);
    assert!(r.angle_misclosure.is_none() && r.linear_misclosure.unwrap() < 1e-9);
}

#[test]
fn traverse_input_is_checked() {
    let t = |angles: Vec<f64>, distances: Vec<f64>, fore: Option<Vec2>, end: Option<Vec2>| {
        traverse(&TraverseInput {
            unit: "grad".into(),
            start: TM,
            back: Vec2::new(TM.x, TM.y + 10.0),
            end,
            fore,
            angles,
            distances,
        })
    };
    assert!(
        t(vec![], vec![], None, None)
            .unwrap_err()
            .contains("en az bir kenar")
    );
    assert!(
        t(vec![100.0], vec![10.0, 20.0], None, None)
            .unwrap_err()
            .contains("2 kırılma açısı")
    );
    assert!(
        t(vec![100.0], vec![0.0], None, None)
            .unwrap_err()
            .contains("sıfırdan büyük")
    );
    assert!(
        t(vec![100.0, 1.0], vec![10.0], Some(TM), None)
            .unwrap_err()
            .contains("bitiş noktası")
    );
    assert!(
        t(vec![f64::NAN], vec![10.0], None, None)
            .unwrap_err()
            .contains("sayı değil")
    );
    assert!(
        traverse(&TraverseInput {
            unit: "rad".into(),
            start: TM,
            back: TM,
            end: None,
            fore: None,
            angles: vec![1.0],
            distances: vec![1.0]
        })
        .unwrap_err()
        .contains("birimi")
    );
    assert!(
        traverse(&TraverseInput {
            unit: "grad".into(),
            start: TM,
            back: TM,
            end: None,
            fore: None,
            angles: vec![1.0],
            distances: vec![1.0]
        })
        .unwrap_err()
        .contains("aynı yerde")
    );
}

#[test]
fn polar_survey_and_stakeout_are_inverse() {
    let mut rng = Rng(0x5eed_5003);
    for round in 0..300 {
        let unit = if round % 2 == 0 {
            Unit::GRAD
        } else {
            Unit::DEG
        };
        let name = if round % 2 == 0 { "grad" } else { "deg" };
        let station = Vec2::new(
            TM.x + rng.range(-500.0, 500.0),
            TM.y + rng.range(-500.0, 500.0),
        );
        let back = from_bearing(station, rng.range(0.0, TAU), rng.range(50.0, 800.0));
        let targets: Vec<Vec2> = (0..8)
            .map(|_| from_bearing(station, rng.range(0.0, TAU), rng.range(1.0, 600.0)))
            .collect();
        let s = stakeout(&StakeoutInput {
            unit: name.into(),
            station,
            back: Some(back),
            targets: targets.clone(),
        })
        .unwrap();
        // Readings: the instrument's zero lies anywhere; the back point reads `zero`.
        let zero = unit.of(rng.range(0.0, TAU));
        let (sz, ih) = (812.345, 1.55);
        let shots: Vec<Shot> = s
            .iter()
            .enumerate()
            .map(|(k, st)| {
                let reading = zero + st.angle.unwrap();
                if k % 2 == 0 {
                    Shot {
                        reading,
                        distance: st.distance,
                        zenith: None,
                        target_height: None,
                    }
                } else {
                    // A slope sight 3 grad (or degrees) off level; the point lies dz below the station mark.
                    let z = unit.rad(if unit == Unit::GRAD { 103.0 } else { 93.0 });
                    let slope = st.distance / crate::jsmath::sin(z);
                    Shot {
                        reading,
                        distance: slope,
                        zenith: Some(unit.of(z)),
                        target_height: Some(1.7),
                    }
                }
            })
            .collect();
        let pts = polar_survey(&PolarInput {
            unit: name.into(),
            station,
            back,
            back_reading: zero,
            station_z: Some(sz),
            instrument_height: Some(ih),
            shots: shots.clone(),
        })
        .unwrap();
        for (k, (got, want)) in pts.iter().zip(&targets).enumerate() {
            assert!(
                near(got.p, *want, 1e-7),
                "round {round} shot {k}: {got:?} ≠ {want:?}"
            );
            assert!(
                (got.bearing - s[k].bearing).abs() < 1e-9
                    || (got.bearing - s[k].bearing).abs() > unit.of(TAU) - 1e-9
            );
            if k % 2 == 0 {
                assert!(got.z.is_none());
            } else {
                let dz = shots[k].distance * crate::jsmath::cos(unit.rad(shots[k].zenith.unwrap()))
                    + ih
                    - 1.7;
                assert!((got.z.unwrap() - (sz + dz)).abs() < 1e-9);
                assert!(dz < 0.0);
            }
        }
    }
    // Without a back point there is no turning angle.
    let s = stakeout(&StakeoutInput {
        unit: "grad".into(),
        station: TM,
        back: None,
        targets: vec![Vec2::new(TM.x + 10.0, TM.y)],
    })
    .unwrap();
    assert!(
        s[0].angle.is_none()
            && (s[0].bearing - 100.0).abs() < 1e-12
            && (s[0].distance - 10.0).abs() < 1e-12
    );
}

#[test]
fn forward_intersection_finds_the_point() {
    let mut rng = Rng(0x5eed_5004);
    for _ in 0..500 {
        let a = Vec2::new(
            TM.x + rng.range(-300.0, 300.0),
            TM.y + rng.range(-300.0, 300.0),
        );
        let b = from_bearing(a, rng.range(0.0, TAU), rng.range(20.0, 500.0));
        // P right of A → B.
        let t = bearing(a, b) + rng.range(0.05, PI - 0.05);
        let p = from_bearing(a, t, rng.range(20.0, 500.0));
        let (alpha, beta) = (angle(Unit::GRAD, a, b, p), angle(Unit::GRAD, b, p, a));
        let got = forward_intersection(Unit::GRAD, a, b, alpha, beta).unwrap();
        assert!(near(got, p, 1e-6), "{got:?} ≠ {p:?}");
    }
    assert!(
        forward_intersection(Unit::GRAD, TM, Vec2::new(TM.x + 1.0, TM.y), 150.0, 60.0).is_err()
    );
    assert!(forward_intersection(Unit::GRAD, TM, TM, 50.0, 60.0).is_err());
}

#[test]
fn resection_finds_the_point_and_refuses_the_danger_circle() {
    let mut rng = Rng(0x5eed_5005);
    let mut met = 0;
    for _ in 0..2000 {
        let p = Vec2::new(
            TM.x + rng.range(-300.0, 300.0),
            TM.y + rng.range(-300.0, 300.0),
        );
        // A, B, C seen from P left to right (clockwise), spread over less than a turn.
        let t0 = rng.range(0.0, TAU);
        let (t1, t2) = (t0 + rng.range(0.2, 2.0), 0.0);
        let t2 = t1 + rng.range(0.2, 2.0) + t2;
        let a = from_bearing(p, t0, rng.range(50.0, 900.0));
        let b = from_bearing(p, t1, rng.range(50.0, 900.0));
        let c = from_bearing(p, t2, rng.range(50.0, 900.0));
        let (alpha, beta) = (angle(Unit::GRAD, p, a, b), angle(Unit::GRAD, p, b, c));
        let r = resection(Unit::GRAD, a, b, c, alpha, beta).unwrap();
        // Near the danger circle the answer is weak: keep to well-conditioned ones for the bound.
        if r.strength > 0.05 {
            assert!(
                near(r.p, p, 1e-6),
                "{:?} ≠ {p:?} (strength {})",
                r.p,
                r.strength
            );
            met += 1;
        }
    }
    assert!(met > 1000, "{met}");
    // P on the circle through A, B, C: refused.
    let (a, b, c) = (
        from_bearing(TM, 0.3, 100.0),
        from_bearing(TM, 1.5, 100.0),
        from_bearing(TM, 2.4, 100.0),
    );
    let p = from_bearing(TM, 4.0, 100.0);
    let (alpha, beta) = (angle(Unit::GRAD, p, a, b), angle(Unit::GRAD, p, b, c));
    let e = resection(Unit::GRAD, a, b, c, alpha, beta);
    assert!(
        e.as_ref().is_err_and(|e| e.contains("tehlike dairesi"))
            || e.as_ref().is_ok_and(|r| r.strength < 1e-6),
        "{e:?}"
    );
}
