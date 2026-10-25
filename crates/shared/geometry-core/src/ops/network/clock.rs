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
//! A question with timed closures (docs/adr/0568 §3) varies the clock: an
//! edge closed in hours steps through a stretch of days around the
//! question's day (its frame, docs/adr/0640 §1) long enough to hold its
//! closure's first and last days; before the frame its first week repeats,
//! after it its last. So a journey of any length meets each day's closures.

use super::rules::PeriodRule;
use crate::jsmath::js_floor;

/// Minutes in a day.
pub const DAY: f64 = 1440.0;

/// The stretch an edge's steps cover, minutes from the question's day's midnight: `[lead, end)`. Before it the first
/// `cycle` minutes repeat, after it the last (a day: `[0, DAY)` and its days repeat).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Frame {
    pub lead: f64,
    pub end: f64,
    pub cycle: f64,
}

impl Frame {
    /// A day repeated (`period` minutes).
    pub fn repeated(period: f64) -> Frame {
        Frame {
            lead: 0.0,
            end: period,
            cycle: period,
        }
    }

    /// Where minute `t` falls: its place in the frame and the shift from it (`t = place + shift`).
    fn place(self, t: f64) -> (f64, f64) {
        let shift = if t < self.lead + self.cycle {
            js_floor((t - self.lead) / self.cycle) * self.cycle
        } else if t >= self.end - self.cycle {
            js_floor((t - (self.end - self.cycle)) / self.cycle) * self.cycle
        } else {
            0.0
        };
        (t - shift, shift)
    }

    /// The same going back: a minute at a step's start is the step before's (`(lead, end]` with its copies).
    fn place_before(self, t: f64) -> (f64, f64) {
        let shift = if t <= self.lead + self.cycle {
            (-js_floor((self.lead + self.cycle - t) / self.cycle)) * self.cycle
        } else if t <= self.end {
            0.0
        } else {
            (-js_floor((self.end - t) / self.cycle)) * self.cycle
        };
        (t - shift, shift)
    }

    /// Where the copy a place at `shift` is in ends going forward (its next copy starts there).
    fn cap(self, shift: f64) -> f64 {
        if shift < 0.0 { self.lead + self.cycle } else { self.end }
    }

    /// Where the copy a place at `shift` is in begins going back.
    fn cap_before(self, shift: f64) -> f64 {
        if shift > 0.0 { self.end - self.cycle } else { self.lead }
    }
}

/// Each edge's speed steps through the day (or its frame: `frames`): `steps[start[e]..start[e + 1]]`, each `(minute
/// it starts, metres a minute)`, the first at the frame's lead; none at all for an edge closed the whole period.
#[derive(Clone, Debug, PartialEq)]
pub struct Clock {
    start: Vec<u32>,
    steps: Vec<(f64, f64)>,
    period: f64,
    /// Each edge's frame (docs/adr/0640 §1); none: every edge's day `[0, period)` repeated.
    frames: Vec<Frame>,
}

impl Default for Clock {
    fn default() -> Clock {
        Clock {
            start: Vec::new(),
            steps: Vec::new(),
            period: DAY,
            frames: Vec::new(),
        }
    }
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
            period: DAY,
            frames: Vec::new(),
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
            period: self.period,
            frames: self.frames.clone(),
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

    /// The clock of closures in hours (docs/adr/0568 §3, 0640 §1): `days[e]` each edge's steps through a day repeated,
    /// `windows[e]` the minutes it is closed (each `[from, to)` within its frame) and `frames[e]` its frame (none: no
    /// window, the day repeated). Each edge with windows steps through its frame, its day's steps a day after a day.
    pub fn framed(days: &[Vec<(f64, f64)>], windows: &[Vec<(f64, f64)>], frames: &[Option<(f64, f64)>]) -> Clock {
        let mut clock = Clock {
            start: Vec::with_capacity(days.len() + 1),
            steps: Vec::new(),
            period: DAY,
            frames: Vec::with_capacity(days.len()),
        };
        for (e, day) in days.iter().enumerate() {
            clock.start.push(clock.steps.len() as u32);
            let shut = windows.get(e).map_or(&[][..], Vec::as_slice);
            let Some((lead, end)) = frames.get(e).copied().flatten().filter(|_| !shut.is_empty()) else {
                clock.frames.push(Frame::repeated(DAY));
                if day.iter().any(|s| s.1 > 0.0) {
                    clock.steps.extend_from_slice(day);
                }
                continue;
            };
            clock.frames.push(Frame {
                lead,
                end,
                cycle: super::access::WEEK,
            });
            let first = js_floor(lead / DAY) as i64;
            let last = js_floor(end / DAY) as i64;
            let mut cuts: Vec<f64> = (first..last)
                .flat_map(|k| day.iter().map(move |s| s.0 + k as f64 * DAY))
                .chain(shut.iter().flat_map(|w| [w.0, w.1]))
                .filter(|t| lead <= *t && *t < end)
                .collect();
            cuts.sort_by(f64::total_cmp);
            cuts.dedup();
            let speed_at = |t: f64| {
                if shut.iter().any(|w| w.0 <= t && t < w.1) {
                    return 0.0;
                }
                let m = t - js_floor(t / DAY) * DAY;
                day.iter().rev().find(|s| s.0 <= m).map_or(0.0, |s| s.1)
            };
            let mut stretch: Vec<(f64, f64)> = cuts.into_iter().map(|t| (t, speed_at(t))).collect();
            stretch.dedup_by(|b, a| b.1 == a.1);
            if stretch.iter().any(|s| s.1 > 0.0) {
                clock.steps.extend(stretch);
            }
        }
        clock.start.push(clock.steps.len() as u32);
        clock
    }

    /// Edge `edge`'s frame.
    fn frame(&self, edge: u32) -> Frame {
        self.frames
            .get(edge as usize)
            .copied()
            .unwrap_or_else(|| Frame::repeated(self.period))
    }

    /// Edge `edge`'s steps through a day: its own, or the question's day's of a frame's.
    pub fn day_steps(&self, edge: u32) -> Vec<(f64, f64)> {
        let fr = self.frame(edge);
        let steps = self.day_of(edge);
        if fr == Frame::repeated(DAY) {
            return steps.to_vec();
        }
        let at = |t: f64| steps.iter().rev().find(|s| s.0 <= t).map_or(0.0, |s| s.1);
        let mut day: Vec<(f64, f64)> = std::iter::once((0.0, at(0.0)))
            .chain(steps.iter().copied().filter(|s| 0.0 < s.0 && s.0 < DAY))
            .collect();
        day.dedup_by(|b, a| b.1 == a.1);
        day
    }

    fn day_of(&self, edge: u32) -> &[(f64, f64)] {
        let e = edge as usize;
        &self.steps[self.start[e] as usize..self.start[e + 1] as usize]
    }

    /// Minutes to go `d` metres along `edge` setting off at minute `t`; infinite when the edge is closed all day.
    pub fn forward(&self, edge: u32, t: f64, d: f64) -> f64 {
        forward_in(self.day_of(edge), self.frame(edge), t, d)
    }

    /// Minutes to go `d` metres along `edge` arriving at minute `t`; infinite when the edge is closed all day.
    pub fn backward(&self, edge: u32, t: f64, d: f64) -> f64 {
        backward_in(self.day_of(edge), self.frame(edge), t, d)
    }

    /// Metres gone along `edge` in `dt` minutes setting off at minute `t`.
    pub fn reach_forward(&self, edge: u32, t: f64, dt: f64) -> f64 {
        reach_forward_in(self.day_of(edge), self.frame(edge), t, dt)
    }

    /// Metres gone along `edge` in the `dt` minutes before arriving at minute `t`.
    pub fn reach_backward(&self, edge: u32, t: f64, dt: f64) -> f64 {
        reach_backward_in(self.day_of(edge), self.frame(edge), t, dt)
    }
}

/// The step a place `loc` falls in going forward.
fn step_at(steps: &[(f64, f64)], loc: f64) -> usize {
    steps.partition_point(|s| s.0 <= loc).saturating_sub(1)
}

/// The step a place `loc` falls in going back (a place at a step's start is the step before's).
fn step_before(steps: &[(f64, f64)], loc: f64) -> usize {
    steps.partition_point(|s| s.0 < loc).saturating_sub(1)
}

/// Where step `k` ends in the copy a place at `shift` is in (its next step's start, or the copy's end), the shift
/// added.
fn step_end(steps: &[(f64, f64)], fr: Frame, k: usize, shift: f64) -> f64 {
    let cap = fr.cap(shift);
    let next = steps.get(k + 1).map_or(cap, |s| s.0);
    (if next < cap { next } else { cap }) + shift
}

/// Where step `k` begins in the copy a place at `shift` is in, the shift added.
fn step_begin(steps: &[(f64, f64)], fr: Frame, k: usize, shift: f64) -> f64 {
    let cap = fr.cap_before(shift);
    let at = steps[k].0;
    (if at > cap { at } else { cap }) + shift
}

/// Minutes to go `d` metres setting off at `t` through a day of speed steps repeated every `period` minutes.
pub fn forward(day: &[(f64, f64)], period: f64, t0: f64, d: f64) -> f64 {
    forward_in(day, Frame::repeated(period), t0, d)
}

/// Minutes to go `d` metres arriving at `t`.
pub fn backward(day: &[(f64, f64)], period: f64, t1: f64, d: f64) -> f64 {
    backward_in(day, Frame::repeated(period), t1, d)
}

/// Metres gone in `dt` minutes setting off at `t`.
pub fn reach_forward(day: &[(f64, f64)], period: f64, t0: f64, dt: f64) -> f64 {
    reach_forward_in(day, Frame::repeated(period), t0, dt)
}

/// Metres gone in the `dt` minutes before arriving at `t`.
pub fn reach_backward(day: &[(f64, f64)], period: f64, t1: f64, dt: f64) -> f64 {
    reach_backward_in(day, Frame::repeated(period), t1, dt)
}

/// Minutes to go `d` metres setting off at `t0` through steps in frame `fr`.
pub fn forward_in(steps: &[(f64, f64)], fr: Frame, t0: f64, d: f64) -> f64 {
    if d <= 0.0 {
        return 0.0;
    }
    if steps.is_empty() || !(t0.is_finite() && d.is_finite()) {
        return f64::INFINITY;
    }
    if steps.len() == 1 {
        return d / steps[0].1;
    }
    let (mut t, mut left) = (t0, d);
    loop {
        let (loc, shift) = fr.place(t);
        let k = step_at(steps, loc);
        let end = step_end(steps, fr, k, shift);
        let v = steps[k].1;
        if v > 0.0 {
            let need = left / v;
            if t + need <= end {
                return t + need - t0;
            }
            left -= v * (end - t);
        }
        t = end;
    }
}

/// Minutes to go `d` metres arriving at `t1` through steps in frame `fr`.
pub fn backward_in(steps: &[(f64, f64)], fr: Frame, t1: f64, d: f64) -> f64 {
    if d <= 0.0 {
        return 0.0;
    }
    if steps.is_empty() || !(t1.is_finite() && d.is_finite()) {
        return f64::INFINITY;
    }
    if steps.len() == 1 {
        return d / steps[0].1;
    }
    let (mut t, mut left) = (t1, d);
    loop {
        let (loc, shift) = fr.place_before(t);
        let k = step_before(steps, loc);
        let begin = step_begin(steps, fr, k, shift);
        let v = steps[k].1;
        if v > 0.0 {
            let need = left / v;
            if t - need >= begin {
                return t1 - (t - need);
            }
            left -= v * (t - begin);
        }
        t = begin;
    }
}

/// Metres gone in `dt` minutes setting off at `t0` through steps in frame `fr`.
pub fn reach_forward_in(steps: &[(f64, f64)], fr: Frame, t0: f64, dt: f64) -> f64 {
    if dt <= 0.0 || steps.is_empty() || !t0.is_finite() {
        return 0.0;
    }
    if !dt.is_finite() {
        return f64::INFINITY;
    }
    let (mut t, stop, mut gone) = (t0, t0 + dt, 0.0);
    loop {
        let (loc, shift) = fr.place(t);
        let k = step_at(steps, loc);
        let end = step_end(steps, fr, k, shift);
        let v = steps[k].1;
        if end >= stop {
            return gone + v * (stop - t);
        }
        gone += v * (end - t);
        t = end;
    }
}

/// Metres gone in the `dt` minutes before arriving at `t1` through steps in frame `fr`.
pub fn reach_backward_in(steps: &[(f64, f64)], fr: Frame, t1: f64, dt: f64) -> f64 {
    if dt <= 0.0 || steps.is_empty() || !t1.is_finite() {
        return 0.0;
    }
    if !dt.is_finite() {
        return f64::INFINITY;
    }
    let (mut t, stop, mut gone) = (t1, t1 - dt, 0.0);
    loop {
        let (loc, shift) = fr.place_before(t);
        let k = step_before(steps, loc);
        let begin = step_begin(steps, fr, k, shift);
        let v = steps[k].1;
        if begin <= stop {
            return gone + v * (t - stop);
        }
        gone += v * (t - begin);
        t = begin;
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
        assert_eq!(forward(&d, DAY, 478.0, 3000.0), 4.0);
        // The same arriving at 08:04: back 2 km at half speed past 08:00, then 1 km at full.
        assert_eq!(backward(&d, DAY, 484.0, 3000.0), 5.0);
        // From 11:59: a minute, the closure's 30 minutes, then the rest.
        assert_eq!(forward(&d, DAY, 719.0, 2000.0), 32.0);
        assert_eq!(reach_forward(&d, DAY, 719.0, 32.0), 2000.0);
        assert_eq!(reach_backward(&d, DAY, 751.0, 32.0), 2000.0);
        // Past midnight into the next day.
        assert_eq!(forward(&d, DAY, 1439.0, 2000.0), 2.0);
        assert_eq!(backward(&d, DAY, 1441.0, 2000.0), 2.0);
        assert_eq!(backward(&d, DAY, -1.0, 1000.0), 1.0);
        assert_eq!(forward(&[], DAY, 0.0, 1.0), f64::INFINITY);
    }

    #[test]
    fn a_week_clock_waits_out_its_windows_and_goes_round_the_week() {
        let week = super::super::access::WEEK;
        // 1000 m a minute, closed from the second day's 22:00 to the third's 06:00 and the last day's 22:00 on.
        let windows = vec![vec![(2760.0, 3240.0), (9960.0, week), (0.0, 360.0)]];
        let clock = Clock::framed(&[vec![(0.0, 1000.0)]], &windows, &[Some((0.0, week))]);
        // From 21:59 of the second day: a minute, eight hours' wait, a minute.
        assert_eq!(clock.forward(0, 2759.0, 2000.0), 482.0);
        // Arriving at 06:01 of the question's day: back a minute, then the night before (minutes before 0).
        assert_eq!(clock.backward(0, 361.0, 2000.0), 482.0);
        // A day without a window as the day's own clock.
        assert_eq!(clock.forward(0, 600.0, 3000.0), 3.0);
        // Closed the whole week: never.
        let shut = Clock::framed(&[vec![(0.0, 1000.0)]], &[vec![(0.0, week)]], &[Some((0.0, week))]);
        assert_eq!(shut.forward(0, 0.0, 1.0), f64::INFINITY);
    }

    #[test]
    fn a_framed_clock_keeps_the_days_before_and_after_its_closure() {
        // Closed every night 22:00–06:00 from the tenth day on; the frame holds days 0 to 17, then its last week repeats.
        let windows: Vec<(f64, f64)> = (10..17)
            .map(|d| (f64::from(d) * DAY + 1320.0, crate::jsmath::js_min(f64::from(d) * DAY + 1800.0, 17.0 * DAY)))
            .collect();
        let clock = Clock::framed(&[vec![(0.0, 1000.0)]], &[windows], &[Some((0.0, 17.0 * DAY))]);
        let night = |d: f64| d * DAY + 1319.0;
        // The second night is open, the twelfth closed: a minute, eight hours' wait, a minute.
        assert_eq!(clock.forward(0, night(2.0), 2000.0), 2.0);
        assert_eq!(clock.forward(0, night(12.0), 2000.0), 482.0);
        // The fortieth night is in a repeated last week, closed too; so is a week before the frame.
        assert_eq!(clock.forward(0, night(40.0), 2000.0), 482.0);
        assert_eq!(clock.forward(0, night(-5.0), 2000.0), 2.0);
        // Back from 06:01 of day 13, and from day 3.
        assert_eq!(clock.backward(0, 13.0 * DAY + 361.0, 2000.0), 482.0);
        assert_eq!(clock.backward(0, 3.0 * DAY + 361.0, 2000.0), 2.0);
        // Ten days from day 5 meet the nights from the tenth: four whole and two hours of the fourteenth.
        let gone = clock.reach_forward(0, 5.0 * DAY, 10.0 * DAY);
        assert_eq!(gone, 1000.0 * (10.0 * DAY - 4.0 * 480.0 - 120.0));
        // Going as far again ends at the fourteenth night's 22:00.
        assert_eq!(clock.forward(0, 5.0 * DAY, gone), 10.0 * DAY - 120.0);
        assert_eq!(clock.day_steps(0), vec![(0.0, 1000.0)]);
    }
}
