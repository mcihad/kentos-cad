//! A curve's superelevation as its PI data writes it (docs/adr/0983), against values worked out without KentOS: KGM's
//! own sample sheet (KGM 2019 şartname EK-10, curve S1) and rows whose crossings are fractions by hand.

use kentos_road::alignment::{Alignment, PiInput, build};
use kentos_road::superelevation::{Crossfall, CurveRule, Row, from_curves, from_rows};
use kentos_road::superelevation_pi::curve_super;

const WITHIN: f64 = 1e-6;

fn pi(id: &str, x: f64, y: f64, radius: f64, spiral: f64) -> PiInput {
    PiInput { id: id.into(), x, y, radius, spiral_in: spiral, spiral_out: spiral, ove: false }
}

fn close(a: f64, b: f64, what: &str) {
    assert!((a - b).abs() < WITHIN, "{what}: {a} vs {b}");
}

/// KGM's S1 on EK-10: R = 600, A = 27.45958 grad, clothoids of 76.8 m; turning left (`sign` 1) or right (−1).
fn kgm_s1(sign: f64) -> Alignment {
    let delta = sign * 27.45958 * std::f64::consts::PI / 200.0;
    build(&[
        pi("a", 0.0, 0.0, 0.0, 0.0),
        pi("s1", 1000.0, 0.0, 600.0, 76.8),
        pi("b", 1000.0 + 1000.0 * delta.cos(), 1000.0 * delta.sin(), 0.0, 0.0),
    ])
    .expect("the alignment builds")
}

#[test]
fn kgms_sample_curve_reads_as_kgm_wrote_it() {
    // EK-10's box: S=6.00, Lr=76.800-76.800, Lt=25.600-25.600, La=25.600-25.600, Gn=0.000. The runoff is the clothoid
    // (on_tangent 0), the runout 76.8 · 2 / 6 = 25.6 is on the tangent before TS.
    for sign in [1.0, -1.0] {
        let al = kgm_s1(sign);
        let normal = Crossfall { left: -0.02, right: -0.02 };
        let rule = CurveRule { pi: "s1".into(), rate: 0.06, runoff: 76.8, on_tangent: 0.0, widening: 0.0 };
        let (s, _) = from_curves(&al, normal, &[rule]).expect("the schedule builds");
        let got = curve_super(&s, normal, &al.curves[0]);
        close(got.rate.expect("banked"), 0.06, "S");
        close(got.widening, 0.0, "Gn");
        for (end, what) in [(got.entry, "entry"), (got.exit, "exit")] {
            let e = end.unwrap_or_else(|| panic!("{what}"));
            close(e.runoff, 76.8, "Lr");
            close(e.runout, 25.6, "Lt");
            close(e.on_tangent, 25.6, "La");
        }
    }
}

/// A right turn of 90° at (500, 0), R = 100, no clothoids: TS at 400, ST at 400 + 50π.
fn quarter() -> Alignment {
    build(&[pi("a", 0.0, 0.0, 0.0, 0.0), pi("p", 500.0, 0.0, 100.0, 0.0), pi("b", 500.0, -500.0, 0.0, 0.0)])
        .expect("the alignment builds")
}

#[test]
fn rows_read_by_their_crossings() {
    let al = quarter();
    let c = &al.curves[0];
    close(c.stations[0], 400.0, "TS");
    close(c.stations[3], 400.0 + 50.0 * std::f64::consts::PI, "ST");
    // Turning right the left side is outside: −2 % to +5 % over 70 m each way, the inside −2 % to −5 %.
    let normal = Crossfall { left: -0.02, right: -0.02 };
    let rows = [
        Row { station: 330.0, left: -0.02, right: -0.02 },
        Row { station: 400.0, left: 0.05, right: -0.05 },
        Row { station: 500.0, left: 0.05, right: -0.05 },
        Row { station: 570.0, left: -0.02, right: -0.02 },
    ];
    let s = from_rows(&rows).expect("rows");
    let got = curve_super(&s, normal, c);
    close(got.rate.expect("banked"), 0.05, "S");
    // In: level at 330 + 70 · 2/7 = 350, so Lt = 20, Lr = 400 − 350 = 50, La = 400 − 330 = 70.
    let e = got.entry.expect("entry");
    close(e.runout, 20.0, "Lt in");
    close(e.runoff, 50.0, "Lr in");
    close(e.on_tangent, 70.0, "La in");
    // Out: level at 500 + 70 · 5/7 = 550, so Lr = 50, Lt = 20; NC at 570, ST at 400 + 50π: La = 170 − 50π.
    let x = got.exit.expect("exit");
    close(x.runoff, 50.0, "Lr out");
    close(x.runout, 20.0, "Lt out");
    close(x.on_tangent, 170.0 - 50.0 * std::f64::consts::PI, "La out");
}

#[test]
fn a_transition_on_the_curve_leaves_no_tangent_part() {
    // The rise starts 10 m past TS: La is 0, not negative.
    let al = quarter();
    let normal = Crossfall { left: -0.02, right: -0.02 };
    let rows = [
        Row { station: 410.0, left: -0.02, right: -0.02 },
        Row { station: 445.0, left: 0.05, right: -0.05 },
        Row { station: 510.0, left: 0.05, right: -0.05 },
    ];
    let got = curve_super(&from_rows(&rows).unwrap(), normal, &al.curves[0]);
    let e = got.entry.expect("entry");
    close(e.on_tangent, 0.0, "La");
    close(e.runout, 10.0, "Lt: 35 · 2/7");
    close(e.runoff, 25.0, "Lr: 35 · 5/7");
    // The rows end banked: the way out never comes back to the normal crown.
    assert!(got.exit.is_none());
}

#[test]
fn an_unbanked_curve_has_no_rate_and_no_transition() {
    let al = quarter();
    let normal = Crossfall { left: -0.02, right: -0.02 };
    let got = curve_super(&from_rows(&[]).unwrap(), normal, &al.curves[0]);
    assert_eq!((got.rate, got.entry, got.exit), (None, None, None));
    close(got.widening, 0.0, "Gn");
}

#[test]
fn a_schedule_that_starts_inside_the_transition_has_no_entry() {
    let al = quarter();
    let normal = Crossfall { left: -0.02, right: -0.02 };
    let rows = [
        Row { station: 380.0, left: 0.01, right: -0.02 },
        Row { station: 400.0, left: 0.05, right: -0.05 },
        Row { station: 500.0, left: 0.05, right: -0.05 },
        Row { station: 570.0, left: -0.02, right: -0.02 },
    ];
    let got = curve_super(&from_rows(&rows).unwrap(), normal, &al.curves[0]);
    assert!(got.entry.is_none(), "it never reaches level going back");
    assert!(got.exit.is_some());
}

#[test]
fn the_widening_is_the_insides() {
    // Turning right the inside is the right side.
    let al = quarter();
    let normal = Crossfall { left: -0.02, right: -0.02 };
    let rule = CurveRule { pi: "p".into(), rate: 0.05, runoff: 50.0, on_tangent: 0.6, widening: 0.8 };
    let (s, _) = from_curves(&al, normal, &[rule]).expect("the schedule builds");
    let got = curve_super(&s, normal, &al.curves[0]);
    close(got.widening, 0.8, "Gn");
    // The rule's runoff, its runout 50 · 2/5 = 20 and 0.6 of the runoff before TS: La = 20 + 30 = 50.
    let e = got.entry.expect("entry");
    close(e.runoff, 50.0, "Lr");
    close(e.runout, 20.0, "Lt");
    close(e.on_tangent, 50.0, "La");
}
