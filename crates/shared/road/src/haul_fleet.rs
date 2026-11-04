//! A haul fleet on the road (docs/adr/0936, plan/road.md Y32; ROAD-HAUL-10): how long a truck takes to carry a load
//! along the road's own heights and come back, the energy it spends, how a haul plan's flows split into hauls, and
//! how many loads a fleet of trucks takes from one loader in an hour when the times vary. The numbers (the truck's
//! masses, power, efficiency, traction, speeds and braking, the road's rolling resistance, the fixed times) are the
//! user's; KentOS fills in none.
//!
//! **The run.** Along the road, at the slope length u, the truck of mass m feels the rolling resistance and the grade
//! `R = m·g·(c_r·cos θ + sin θ)`, and pulls with `F(v) = min(η·P/v, μ·D·m·g·cos θ)`: the engine's power reaching the
//! wheels, no more than the driven wheels' grip. Newton's second law in the distance, `m·v·dv/du = F(v) − R`, is
//! solved for `w = v²` by Heun's method in steps of at most 0,25 m, shorter where v² changes by more than 2 %; the speed is held under the top speed (and the
//! downhill speed where the grade pushes the truck), and under the braking curve `v² = 2·b·(L − u)` that stops it at
//! the end. Each step's time is `2h/(v₀ + v₁)`, exact for a constant acceleration (so the start from rest, where 1/v
//! is unbounded, is integrated exactly). The force the wheels deliver over a step is what the change of speed and the
//! resistance ask, `m·(w₁ − w₀)/(2h) + R`, never under zero (braking is not the engine's): its sum is the work at the
//! wheels. Where it is zero the engine idles. A truck whose grip is less than the grade's resistance stalls there.
//!
//! **The hauls.** On a line, a haul plan's segment flows (forward positive) make each place a source or a sink: the
//! forward flows are swept left to right and the backward ones right to left, a place's surplus queued, a place's
//! demand taken from the front of the queue (first in, first out: loads do not overtake one another). The moment of
//! the hauls is the plan's.
//!
//! **The loader.** N trucks share one loader. A truck is away for a time with mean `a` (travel, dumping, spotting) and
//! is loaded in a time with mean `s`. Deterministically the fleet makes `min(N/(a + s), 1/s)` loads per unit time and
//! the match factor is `N·s/(a + s)`. With the times exponential the loader is the finite-source queue (the machine
//! repair model): `p₀ = [Σₙ N!/(N − n)!·(s/a)ⁿ]⁻¹`, the loader busy `1 − p₀` of the time, the fleet making `(1 − p₀)/s`
//! loads, a truck waiting `N/X − a − s` per cycle at the loader. The truth lies between the two: real times vary
//! less than exponentially. Both are given; neither is called the production.

/// Standard gravity (m/s²), as the other physics of the road (docs/adr/0931).
pub const G: f64 = 9.80665;

/// The longest step of the run (m along the slope), and the most a step changes v² (per one).
const STEP: f64 = 0.25;
const CHANGE: f64 = 0.02;

/// A truck: its masses empty and loaded (kg), its engine's power (W) and the share of it reaching the wheels, its
/// top speed and its speed down a grade that pushes it (m/s), the grip of its driven wheels (the friction coefficient
/// and the share of its weight on them), and the deceleration it stops with (m/s²).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Truck {
    pub empty: f64,
    pub gross: f64,
    pub power: f64,
    pub efficiency: f64,
    pub top: f64,
    pub downhill: Option<f64>,
    pub traction: f64,
    pub drive: f64,
    pub brake: f64,
}

/// A run along the road: its time (s), the slope length (m), the work at the wheels (J), the time the engine does
/// not pull (s), the highest speed (m/s), and the speed along the way every `sample` metres of the route's
/// horizontal distance (`[distance, m/s]`; the start and the end too).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Run {
    pub time: f64,
    pub length: f64,
    pub work: f64,
    pub idle: f64,
    pub fastest: f64,
    pub speeds: Vec<[f64; 2]>,
}

/// Why a run does not end: the truck stalls at this horizontal distance, its grip less than the grade's resistance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Stall {
    pub at: f64,
}

/// The truck of mass `mass` (kg) driven from rest along `path` (`[horizontal distance from the start, height]`,
/// increasing distances) to a stop at its end on a road of rolling resistance `rolling` (per one); `sample` (m) the
/// spacing of the speeds kept (none: none kept).
pub fn run(path: &[[f64; 2]], mass: f64, truck: &Truck, rolling: f64, sample: Option<f64>) -> Result<Run, Stall> {
    let pieces: Vec<(f64, f64, f64, f64)> = path
        .windows(2)
        .filter(|p| p[1][0] > p[0][0])
        .map(|p| {
            let (dx, dz) = (p[1][0] - p[0][0], p[1][1] - p[0][1]);
            let l = dx.hypot(dz);
            (p[0][0], l, dx / l, dz / l)
        })
        .collect();
    let total: f64 = pieces.iter().map(|p| p.1).sum();
    let mut out = Run { length: total, ..Run::default() };
    let keep = |out: &mut Run, x: f64, v: f64| out.speeds.push([x, v]);
    if sample.is_some() {
        keep(&mut out, path.first().map_or(0.0, |p| p[0]), 0.0);
    }
    let mut next_sample = path.first().map_or(0.0, |p| p[0]) + sample.unwrap_or(f64::INFINITY);
    let (mut u, mut w) = (0.0f64, 0.0f64);
    for &(x0, l, cos, sin) in &pieces {
        let resist = mass * G * (rolling * cos + sin);
        let grip = truck.traction * truck.drive * mass * G * cos;
        let pull = |w: f64| {
            let v = w.max(0.0).sqrt();
            if v > 0.0 { (truck.efficiency * truck.power / v).min(grip) } else { grip }
        };
        // d(v²)/du = 2·(F − R)/m.
        let slope = |w: f64| 2.0 * (pull(w) - resist) / mass;
        let cap = match truck.downhill {
            Some(d) if resist < 0.0 => truck.top.min(d),
            _ => truck.top,
        };
        // Steps of at most 0,25 m, shorter where v² changes fast (the start, the grip giving way to the power):
        // each step changes v² by about 2 % at most. Fixed 0,25 m steps put the underpowered climb 77 ms (5·10⁻⁴)
        // late against its closed form; these 0,5 ms.
        let mut along = 0.0;
        while along < l - 1e-12 {
            let k1 = slope(w);
            let mut h = STEP.min(l - along);
            if k1 != 0.0 {
                h = h.min((CHANGE * w.max(0.01) / k1.abs()).max(1e-4));
            }
            let k2 = slope((w + h * k1).max(0.0));
            let free = w + 0.5 * h * (k1 + k2);
            let u1 = u + h;
            if free < 0.0 && total - u1 > 1e-9 {
                return Err(Stall { at: x0 + (along + 0.5 * h) * cos });
            }
            let w1 = free.min(cap * cap).min(2.0 * truck.brake * (total - u1).max(0.0)).max(0.0);
            let (v0, v1) = (w.sqrt(), w1.sqrt());
            let dt = if v0 + v1 > 0.0 { 2.0 * h / (v0 + v1) } else { 0.0 };
            let force = mass * (w1 - w) / (2.0 * h) + resist;
            if force > 1e-9 * mass {
                out.work += force * h;
            } else {
                out.idle += dt;
            }
            out.time += dt;
            out.fastest = out.fastest.max(v1);
            u = u1;
            w = w1;
            along += h;
            let x = x0 + along * cos;
            if sample.is_some() && x >= next_sample - 1e-9 {
                keep(&mut out, x, v1);
                next_sample += sample.unwrap_or(f64::INFINITY);
            }
        }
    }
    if let (Some(_), Some(last)) = (sample, path.last())
        && out.speeds.last().is_none_or(|s| (s[0] - last[0]).abs() > 1e-9)
    {
        keep(&mut out, last[0], w.sqrt());
    }
    Ok(out)
}

/// The path the other way: a run back from the end to the start (distances measured from the end).
pub fn reversed(path: &[[f64; 2]]) -> Vec<[f64; 2]> {
    let end = path.last().map_or(0.0, |p| p[0]);
    path.iter().rev().map(|p| [end - p[0], p[1]]).collect()
}

/// A haul: from a station to a station, the volume carried (m³).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Haul {
    pub from: f64,
    pub to: f64,
    pub volume: f64,
}

/// A haul plan's segments (`from`, `to` stations, the flow, forward positive; in station order, neighbouring) split
/// into hauls, first in, first out; the forward hauls first, by their start, then the backward ones.
pub fn hauls(segments: &[(f64, f64, f64)]) -> Vec<Haul> {
    const TINY: f64 = 1e-9;
    let mut out = Vec::new();
    for forward in [true, false] {
        // The places in the sweep's order and the flow into and out of each along it.
        let n = segments.len();
        let flow = |i: usize| if forward { segments[i].2.max(0.0) } else { (-segments[i].2).max(0.0) };
        let mut queue: std::collections::VecDeque<(f64, f64)> = std::collections::VecDeque::new();
        for step in 0..=n {
            // Place `step` along the sweep: forward the segments' starts then the last end; backward the reverse.
            let (station, inflow, outflow) = if forward {
                let station = if step < n { segments[step].0 } else { segments[n - 1].1 };
                (station, if step > 0 { flow(step - 1) } else { 0.0 }, if step < n { flow(step) } else { 0.0 })
            } else {
                let i = n - step; // the place's index from the left
                let station = if i < n { segments[i].0 } else { segments[n - 1].1 };
                (station, if i < n { flow(i) } else { 0.0 }, if i > 0 { flow(i - 1) } else { 0.0 })
            };
            if outflow > inflow + TINY {
                queue.push_back((station, outflow - inflow));
            } else if inflow > outflow + TINY {
                let mut need = inflow - outflow;
                while need > TINY {
                    let Some(front) = queue.front_mut() else { break };
                    let take = front.1.min(need);
                    out.push(Haul { from: front.0, to: station, volume: take });
                    front.1 -= take;
                    need -= take;
                    if front.1 <= TINY {
                        queue.pop_front();
                    }
                }
            }
        }
    }
    out
}

/// N trucks at one loader: the match factor, the loads per second deterministically and as the finite-source queue,
/// the loader's busy share in the queue, and a truck's wait at the loader per cycle in the queue (s).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Loader {
    pub match_factor: f64,
    pub steady: f64,
    pub queued: f64,
    pub busy: f64,
    pub wait: f64,
}

/// `trucks` trucks away from the loader `away` seconds on average, loaded in `service` seconds on average.
pub fn loader(trucks: u32, away: f64, service: f64) -> Loader {
    let n = f64::from(trucks);
    let ratio = service / away;
    // Σ N!/(N − k)!·ratioᵏ, its terms built one from the last.
    let (mut term, mut sum) = (1.0, 1.0);
    for k in 0..trucks {
        term *= f64::from(trucks - k) * ratio;
        sum += term;
    }
    let busy = 1.0 - 1.0 / sum;
    let queued = busy / service;
    Loader {
        match_factor: n * service / (away + service),
        steady: (n / (away + service)).min(1.0 / service),
        queued,
        busy,
        wait: if queued > 0.0 { (n / queued - away - service).max(0.0) } else { 0.0 },
    }
}
