//! A vertical curve's least length from what the driver must see and feel (docs/adr/0935, plan/road.md Y31;
//! ROAD-PROFILE-13). The numbers (speed, reaction time, deceleration, eye, object and headlight heights, the beam's
//! angle, the vertical acceleration, the bisector) are the standard's or the user's; KentOS fills in none.
//!
//! **Stopping sight distance.** The distance run in the reaction time t at the speed V and the braking distance at the
//! deceleration a on the grade G (per one, positive uphill): `S = v·t + v²/(2·(a + g·G))`, v = V/3.6 (AASHTO writes
//! `0.278·V·t + V²/(254·(a/9.81 ± G))`, 254 being 2·9.81·3.6² rounded: 0.1 % apart). AASHTO's t = 2.5 s and a = 3.4 m/s² are what about 90 % of drivers
//! do better than: a percentile, not a mean.
//!
//! **Crest.** A symmetric parabola of length L and grade change A (percent) falls `A·x²/(200·L)` from its tangent at
//! a distance x; the sight line from an eye h₁ high to an object h₂ high touches it, each height at
//! `√(200·L·h/A)` from the touch: `S = √(200·L/A)·(√h₁ + √h₂)`. So for S ≤ L: `L = A·S²/(200·(√h₁ + √h₂)²)`; where
//! the sight runs past the curve (S > L): `L = 2S − 200·(√h₁ + √h₂)²/A`.
//!
//! **Sag, headlights.** At night the road ahead is what the headlights at h light, their beam rising β above the axis:
//! S ≤ L: `L = A·S²/(200·(h + S·tan β))`; S > L: `L = 2S − 200·(h + S·tan β)/A`.
//!
//! **Sag, comfort.** On the parabola the vertical acceleration is `v²/R` with `R = 100·L/A`: `L = A·V²/(1296·a_v)`
//! (V km/h, a_v m/s²; AASHTO's `A·V²/395` is a_v ≈ 0.3).
//!
//! **Bisector.** The curve's middle lies `E = A·L/800` from the PVI: a least E asks `L = 800·E/A`.

/// The stopping sight distance (m) at `speed` (km/h) with the reaction `time` (s) and the deceleration `decel`
/// (m/s²) on the grade `grade` (per one, uphill positive); none where the grade takes the braking away
/// (`a/9.81 + G ≤ 0`) or a value is not positive.
pub fn stopping(speed: f64, time: f64, decel: f64, grade: f64) -> Option<f64> {
    let brake = decel + G * grade;
    let ok = speed > 0.0 && time >= 0.0 && decel > 0.0 && brake > 0.0 && [speed, time, decel, grade].iter().all(|v| v.is_finite());
    let v = speed / 3.6;
    ok.then(|| v * time + v * v / (2.0 * brake))
}

/// Standard gravity (m/s²), as the superelevation's physics (docs/adr/0931): AASHTO's 254 is `2·9.81·3.6²` rounded.
pub const G: f64 = 9.80665;

/// The least crest length (m) for the sight `s` (m) over the grade change `a` (percent, its size) from an eye `eye`
/// to an object `object` (m) high; 0 where the grades need no curve for it.
pub fn crest(a: f64, s: f64, eye: f64, object: f64) -> f64 {
    let a = a.abs();
    if a == 0.0 {
        return 0.0;
    }
    let h = (eye.sqrt() + object.sqrt()).powi(2);
    let within = a * s * s / (200.0 * h);
    if within <= s { (2.0 * s - 200.0 * h / a).max(0.0).min(within) } else { within }
}

/// The least sag length (m) for the headlight sight `s` (m) over the grade change `a` (percent) with the headlights
/// `height` (m) high and the beam `beam` degrees above the axis.
pub fn headlight(a: f64, s: f64, height: f64, beam: f64) -> f64 {
    let a = a.abs();
    if a == 0.0 {
        return 0.0;
    }
    let lift = height + s * beam.to_radians().tan();
    let within = a * s * s / (200.0 * lift);
    if within <= s { (2.0 * s - 200.0 * lift / a).max(0.0).min(within) } else { within }
}

/// The least sag length (m) for the vertical acceleration `accel` (m/s²) at `speed` (km/h): `A·V²/(1296·a)`.
pub fn comfort(a: f64, speed: f64, accel: f64) -> f64 {
    a.abs() * speed * speed / (1296.0 * accel)
}

/// The least length (m) for the bisector (the curve's middle from the PVI) `external` (m): `800·E/A`.
pub fn bisector(a: f64, external: f64) -> f64 {
    if a == 0.0 { 0.0 } else { 800.0 * external / a.abs() }
}
