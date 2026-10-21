//! A speed cost's time profile (docs/adr/0521 §4): each edge's speed through
//! the day, a step from each period's start, and how long a stretch takes
//! when it is entered (or left) at a time of day. A vehicle on an edge goes
//! at the speed of the moment: a stretch crossing a period's end goes on at
//! the next period's speed, and where the speed is zero (a closed edge) it
//! waits until the edge opens; the day repeats. So arriving later never
//! means leaving earlier (first in, first out), and the earliest arrival is
//! found by a search's labels exactly as a fixed cost's (docs/adr/0521 §5).
//! Times are minutes; the day's are 0–1440, a search's past it go on into
//! the next day (and in a backward search before 0 into the one before).

use super::rules::PeriodRule;
use crate::jsmath::js_floor;

/// Minutes in a day.
pub const DAY: f64 = 1440.0;

/// Each edge's speed steps through the day: `steps[start[e]..start[e + 1]]`, each `(minute it starts, metres a
/// minute)`, the first at 0; none at all for an edge closed the whole day.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Clock {
    start: Vec<u32>,
    steps: Vec<(f64, f64)>,
}

impl Clock {
    /// The clock of a speed cost: `base[e]` each edge's own speed (km/h), `values[e][k]` what its field gives for
    /// period `k` when the period reads a field (none: no value). Also the edges whose period values were not read.
    pub fn new(
        periods: &[PeriodRule],
        base: &[f64],
        values: &[Vec<Option<String>>],
        unread: &mut Vec<usize>,
    ) -> Clock {
        let mut clock = Clock {
            start: Vec::with_capacity(base.len() + 1),
            steps: Vec::new(),
        };
        let mpm = |kmh: f64| kmh * 1000.0 / 60.0;
        for (e, &kmh) in base.iter().enumerate() {
            clock.start.push(clock.steps.len() as u32);
            let mut day: Vec<(f64, f64)> = Vec::with_capacity(2 * periods.len() + 1);
            let mut at = 0.0;
            let mut bad = false;
            for (k, p) in periods.iter().enumerate() {
                if p.from > at {
                    day.push((at, mpm(kmh)));
                }
                let speed = match p.factor {
                    Some(f) => kmh * f,
                    None => match values.get(e).and_then(|v| v.get(k)).and_then(Option::as_deref) {
                        None => kmh,
                        Some(t) if t.trim().is_empty() => kmh,
                        Some(t) => match super::rules::number(t).filter(|v| *v >= 0.0) {
                            Some(v) => v,
                            None => {
                                bad = true;
                                kmh
                            }
                        },
                    },
                };
                day.push((p.from, mpm(speed)));
                at = p.to;
            }
            if at < DAY {
                day.push((at, mpm(kmh)));
            }
            if bad {
                unread.push(e);
            }
            // A speed that does not change is one step (and the stretch's time one division).
            day.dedup_by(|b, a| b.1 == a.1);
            // An edge closed all day is travelled by nothing with this cost.
            if day.iter().any(|s| s.1 > 0.0) {
                clock.steps.extend(day);
            }
        }
        clock.start.push(clock.steps.len() as u32);
        clock
    }

    /// The clock of a vehicle whose top speed is `kmh` (docs/adr/0567 §1): every step's speed at most it.
    pub fn capped(&self, kmh: f64) -> Clock {
        let top = kmh * 1000.0 / 60.0;
        let mut clock = Clock {
            start: Vec::with_capacity(self.start.len()),
            steps: Vec::with_capacity(self.steps.len()),
        };
        for e in 0..self.start.len().saturating_sub(1) {
            clock.start.push(clock.steps.len() as u32);
            let mut day: Vec<(f64, f64)> = self
                .day_of(e as u32)
                .iter()
                .map(|&(t, s)| (t, if s > top { top } else { s }))
                .collect();
            day.dedup_by(|b, a| b.1 == a.1);
            clock.steps.extend(day);
        }
        clock.start.push(clock.steps.len() as u32);
        clock
    }

    fn day_of(&self, edge: u32) -> &[(f64, f64)] {
        let e = edge as usize;
        &self.steps[self.start[e] as usize..self.start[e + 1] as usize]
    }

    /// Minutes to go `d` metres along `edge` setting off at minute `t`; infinite when the edge is closed all day.
    pub fn forward(&self, edge: u32, t: f64, d: f64) -> f64 {
        forward(self.day_of(edge), t, d)
    }

    /// Minutes to go `d` metres along `edge` arriving at minute `t`; infinite when the edge is closed all day.
    pub fn backward(&self, edge: u32, t: f64, d: f64) -> f64 {
        backward(self.day_of(edge), t, d)
    }

    /// Metres gone along `edge` in `dt` minutes setting off at minute `t`.
    pub fn reach_forward(&self, edge: u32, t: f64, dt: f64) -> f64 {
        reach_forward(self.day_of(edge), t, dt)
    }

    /// Metres gone along `edge` in the `dt` minutes before arriving at minute `t`.
    pub fn reach_backward(&self, edge: u32, t: f64, dt: f64) -> f64 {
        reach_backward(self.day_of(edge), t, dt)
    }
}

/// The step `t` falls in: its index and the day it is in (minutes at the day's start).
fn step_at(day: &[(f64, f64)], t: f64) -> (usize, f64) {
    let base = js_floor(t / DAY) * DAY;
    let m = t - base;
    let k = day.iter().rposition(|s| s.0 <= m).unwrap_or(0);
    (k, base)
}

/// The step `t` falls in going back: the one ending at or after it (a time at a step's start is the step before's).
fn step_before(day: &[(f64, f64)], t: f64) -> (usize, f64) {
    let base = js_floor(t / DAY) * DAY;
    let m = t - base;
    if m == 0.0 {
        return (day.len() - 1, base - DAY);
    }
    let k = day.iter().rposition(|s| s.0 < m).unwrap_or(0);
    (k, base)
}

fn step_end(day: &[(f64, f64)], k: usize) -> f64 {
    day.get(k + 1).map_or(DAY, |s| s.0)
}

/// Minutes to go `d` metres setting off at `t` through a day of speed steps.
pub fn forward(day: &[(f64, f64)], t0: f64, d: f64) -> f64 {
    if d <= 0.0 {
        return 0.0;
    }
    if day.is_empty() || !(t0.is_finite() && d.is_finite()) {
        return f64::INFINITY;
    }
    if day.len() == 1 {
        return d / day[0].1;
    }
    let (mut k, mut base) = step_at(day, t0);
    let (mut t, mut left) = (t0, d);
    loop {
        let end = base + step_end(day, k);
        let v = day[k].1;
        if v > 0.0 {
            let need = left / v;
            if t + need <= end {
                return t + need - t0;
            }
            left -= v * (end - t);
        }
        t = end;
        k += 1;
        if k == day.len() {
            k = 0;
            base += DAY;
        }
    }
}

/// Minutes to go `d` metres arriving at `t`.
pub fn backward(day: &[(f64, f64)], t1: f64, d: f64) -> f64 {
    if d <= 0.0 {
        return 0.0;
    }
    if day.is_empty() || !(t1.is_finite() && d.is_finite()) {
        return f64::INFINITY;
    }
    if day.len() == 1 {
        return d / day[0].1;
    }
    let (mut k, mut base) = step_before(day, t1);
    let (mut t, mut left) = (t1, d);
    loop {
        let begin = base + day[k].0;
        let v = day[k].1;
        if v > 0.0 {
            let need = left / v;
            if t - need >= begin {
                return t1 - (t - need);
            }
            left -= v * (t - begin);
        }
        t = begin;
        if k == 0 {
            k = day.len() - 1;
            base -= DAY;
        } else {
            k -= 1;
        }
    }
}

/// Metres gone in `dt` minutes setting off at `t`.
pub fn reach_forward(day: &[(f64, f64)], t0: f64, dt: f64) -> f64 {
    if dt <= 0.0 || day.is_empty() || !t0.is_finite() {
        return 0.0;
    }
    if !dt.is_finite() {
        return f64::INFINITY;
    }
    let (mut k, mut base) = step_at(day, t0);
    let (mut t, stop, mut gone) = (t0, t0 + dt, 0.0);
    loop {
        let end = base + step_end(day, k);
        let v = day[k].1;
        if end >= stop {
            return gone + v * (stop - t);
        }
        gone += v * (end - t);
        t = end;
        k += 1;
        if k == day.len() {
            k = 0;
            base += DAY;
        }
    }
}

/// Metres gone in the `dt` minutes before arriving at `t`.
pub fn reach_backward(day: &[(f64, f64)], t1: f64, dt: f64) -> f64 {
    if dt <= 0.0 || day.is_empty() || !t1.is_finite() {
        return 0.0;
    }
    if !dt.is_finite() {
        return f64::INFINITY;
    }
    let (mut k, mut base) = step_before(day, t1);
    let (mut t, stop, mut gone) = (t1, t1 - dt, 0.0);
    loop {
        let begin = base + day[k].0;
        let v = day[k].1;
        if begin <= stop {
            return gone + v * (t - stop);
        }
        gone += v * (t - begin);
        t = begin;
        if k == 0 {
            k = day.len() - 1;
            base -= DAY;
        } else {
            k -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 60 km/h (1000 m a minute) but 30 km/h from 08:00 to 09:00 and closed from 12:00 to 12:30.
    fn day() -> Vec<(f64, f64)> {
        vec![
            (0.0, 1000.0),
            (480.0, 500.0),
            (540.0, 1000.0),
            (720.0, 0.0),
            (750.0, 1000.0),
        ]
    }

    #[test]
    fn a_stretch_goes_at_the_speed_of_the_moment_and_waits_where_it_is_closed() {
        let d = day();
        // 3 km from 07:58: 2 km in 2 minutes at full speed, then 1 km at half: 4 minutes in all.
        assert_eq!(forward(&d, 478.0, 3000.0), 4.0);
        // The same arriving at 08:04: back 2 km at half speed past 08:00, then 1 km at full.
        assert_eq!(backward(&d, 484.0, 3000.0), 5.0);
        // From 11:59: a minute, the closure's 30 minutes, then the rest.
        assert_eq!(forward(&d, 719.0, 2000.0), 32.0);
        assert_eq!(reach_forward(&d, 719.0, 32.0), 2000.0);
        assert_eq!(reach_backward(&d, 751.0, 32.0), 2000.0);
        // Past midnight into the next day.
        assert_eq!(forward(&d, 1439.0, 2000.0), 2.0);
        assert_eq!(backward(&d, 1441.0, 2000.0), 2.0);
        assert_eq!(backward(&d, -1.0, 1000.0), 1.0);
        assert_eq!(forward(&[], 0.0, 1.0), f64::INFINITY);
    }
}
