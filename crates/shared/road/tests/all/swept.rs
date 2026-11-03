//! A design vehicle's swept path (src/swept.rs, docs/adr/0915): a straight drive, the circle's steady state
//! √(R² − L²) (and √(R² − L₁² − L₂²) with a trailer on the rear axle), the tractrix after a right-angle corner in closed
//! form, the clearance to a curb, and the refusals.

use kentos_road::swept::{self, SweptError, Trailer, Vehicle};

fn car(wheelbase: f64) -> Vehicle {
    Vehicle { wheelbase, front: 1.0, rear: 1.5, width: 2.5, trailer: None }
}

fn circle(r: f64, turns: f64, n: usize) -> Vec<[f64; 2]> {
    let m = (n as f64 * turns) as usize;
    (0..=m)
        .map(|k| {
            let a = std::f64::consts::TAU * k as f64 / n as f64;
            [r * a.sin(), r - r * a.cos()]
        })
        .collect()
}

#[test]
fn a_straight_drive_keeps_the_rear_on_the_path() {
    let s = swept::drive(&car(6.0), &[[0.0, 0.0], [50.0, 0.0]], 0.5).expect("drives");
    assert!(s.offtracking < 1e-12, "{}", s.offtracking);
    let last = s.samples.last().expect("samples");
    assert!((last.rear[0] - 44.0).abs() < 1e-9 && last.rear[1].abs() < 1e-12, "{last:?}");
    assert!((last.distance - 50.0).abs() < 1e-12);
    // The body: 1 m ahead of the front axle, 1.5 m behind the rear, 2.5 m wide.
    assert_eq!(last.bodies[0], [[51.0, 1.25], [51.0, -1.25], [42.5, -1.25], [42.5, 1.25]]);
}

#[test]
fn on_a_circle_the_rear_settles_inside() {
    // Front on R = 20 for three turns: the rear's radius tends to √(R² − L²).
    let s = swept::drive(&car(6.0), &circle(20.0, 3.0, 7200), 0.05).expect("drives");
    let last = s.samples.last().expect("samples");
    let r = last.rear[0].hypot(last.rear[1] - 20.0);
    assert!((r - (400.0_f64 - 36.0).sqrt()).abs() < 1e-3, "{r}");
    assert!((s.offtracking - (20.0 - 364.0_f64.sqrt())).abs() < 1e-3, "{}", s.offtracking);
    let t = Vehicle {
        trailer: Some(Trailer { kingpin: 0.0, wheelbase: 8.0, front: 1.0, rear: 1.0, width: 2.5 }),
        ..car(6.0)
    };
    let s = swept::drive(&t, &circle(20.0, 4.0, 7200), 0.05).expect("drives");
    let last = s.samples.last().expect("samples");
    let ta = last.trailer.expect("a trailer");
    let rt = ta[0].hypot(ta[1] - 20.0);
    assert!((rt - (400.0_f64 - 36.0 - 64.0).sqrt()).abs() < 2e-3, "{rt}");
    assert_eq!(last.bodies.len(), 2);
}

#[test]
fn after_a_right_angle_the_rear_draws_a_tractrix() {
    // Up the y axis to the origin, then east: at the corner the bar is at 90° to the new heading, so after s metres
    // tan(φ/2) = e^(−s/L) and the rear is the front less L (cos φ, sin φ).
    let l = 6.0;
    let s = swept::drive(&car(l), &[[0.0, -10.0], [0.0, 0.0], [5.0, 0.0]], 0.1).expect("drives");
    let last = s.samples.last().expect("samples");
    let phi = 2.0 * (-5.0_f64 / l).exp().atan();
    let want = [5.0 - l * phi.cos(), -l * phi.sin()];
    assert!((last.rear[0] - want[0]).abs() < 1e-9 && (last.rear[1] - want[1]).abs() < 1e-9, "{:?} {want:?}", last.rear);
}

#[test]
fn the_clearance_to_a_curb() {
    let s = swept::drive(&car(6.0), &[[0.0, 0.0], [30.0, 0.0]], 0.5).expect("drives");
    let (d, _) = swept::clearance(&s, &[vec![[-20.0, 3.0], [40.0, 3.0]]]).expect("a clearance");
    assert!((d - 1.75).abs() < 1e-12, "{d}");
    let (d, _) = swept::clearance(&s, &[vec![[10.0, -5.0], [10.0, 5.0]]]).expect("a clearance");
    assert_eq!(d, 0.0);
    // A curb post inside the body meets it.
    let (d, _) = swept::clearance(&s, &[vec![[20.0, 0.1], [20.0, 0.2]]]).expect("a clearance");
    assert_eq!(d, 0.0);
}

#[test]
fn the_refusals() {
    assert_eq!(swept::drive(&car(0.0), &[[0.0, 0.0], [1.0, 0.0]], 0.5).err(), Some(SweptError::Vehicle));
    assert_eq!(swept::drive(&Vehicle { rear: -1.0, ..car(6.0) }, &[[0.0, 0.0], [1.0, 0.0]], 0.5).err(), Some(SweptError::Vehicle));
    assert_eq!(swept::drive(&car(6.0), &[[0.0, 0.0], [0.0, 0.0]], 0.5).err(), Some(SweptError::Path));
    assert_eq!(swept::drive(&car(6.0), &[[0.0, 0.0], [1.0, 0.0]], 0.0).err(), Some(SweptError::Path));
}
