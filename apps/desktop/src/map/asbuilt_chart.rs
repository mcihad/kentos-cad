//! Uygulama kontrolü › İzleme's Grafik (docs/adr/5220; `cad.asbuilt.check`'s tracks): one mark's movement from its
//! reference at every epoch it was measured in, against the epochs' days (or their order when a day is missing), in mm.
//! The part is chosen: horizontal, east, north or height. East, north and height show the line fitted through the dated
//! epochs (its velocity is the answer's, its intercept through the mean of the same epochs, as least squares has it);
//! horizontal and height show the thresholds typed as lines with their bands, the warning's and the alarm's, height's
//! either way. A measurement beyond a threshold is drawn in its level's colour. Nothing is checked again.

use iced::widget::canvas::{self, LineDash, Path, Stroke, Text};
use iced::widget::{column, container, row};
use iced::{Center, Color, Element, Fill, Pixels, Point, Rectangle, Renderer, Theme, mouse};
use kentos_commands::{AsbuiltChecked, AsbuiltState, AsbuiltThresholds, AsbuiltTrack};
use kentos_ui::label;
use kentos_ui::widget::Segmented;
use kentos_ui::theme::Tokens;

use crate::app::Message;

/// The part of the movement drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Part {
    #[default]
    Horizontal,
    East,
    North,
    Up,
}

impl std::fmt::Display for Part {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Part::Horizontal => "Yatay",
            Part::East => "Doğu",
            Part::North => "Kuzey",
            Part::Up => "Kot",
        })
    }
}

/// A day `YYYY-MM-DD` as days since 1970-01-01 (the proleptic Gregorian calendar).
pub fn day_number(text: &str) -> Option<i64> {
    let b = text.trim();
    let (y, rest) = b.split_once('-')?;
    let (m, d) = rest.split_once('-')?;
    let (y, m, d): (i64, i64, i64) = (y.parse().ok()?, m.parse().ok()?, d.parse().ok()?);
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some(era * 146_097 + doe - 719_468)
}

/// A step's value of the part (m); none where the mark was not measured or has no height.
fn value(step: &kentos_commands::AsbuiltStep, part: Part) -> Option<f64> {
    match part {
        Part::Horizontal => Some(step.horizontal),
        Part::East => Some(step.de),
        Part::North => Some(step.dn),
        Part::Up => step.dz,
    }
}

/// The measurements drawn: (epoch, day if known, value in m).
pub fn points(track: &AsbuiltTrack, days: &[Option<i64>], part: Part) -> Vec<(usize, Option<i64>, f64)> {
    track
        .steps
        .iter()
        .enumerate()
        .filter_map(|(k, s)| s.as_ref().and_then(|s| value(s, part)).map(|v| (k, days.get(k).copied().flatten(), v)))
        .collect()
}

/// The fitted line through the dated measurements as (day, value) at its two ends: the answer's velocity, the intercept
/// through their mean (least squares); none for the horizontal part or without a line.
pub fn line(track: &AsbuiltTrack, days: &[Option<i64>], part: Part) -> Option<[(i64, f64); 2]> {
    let fit = track.fit.as_ref()?;
    let v = match part {
        Part::East => fit.east,
        Part::North => fit.north,
        Part::Up => fit.up?,
        Part::Horizontal => return None,
    };
    let dated: Vec<(i64, f64)> = points(track, days, part).into_iter().filter_map(|(_, d, v)| d.map(|d| (d, v))).collect();
    let (first, last) = (dated.first()?.0, dated.last()?.0);
    let years = |d: i64| (d - first) as f64 / 365.25;
    let n = dated.len() as f64;
    let (tm, dm) = (dated.iter().map(|p| years(p.0)).sum::<f64>() / n, dated.iter().map(|p| p.1).sum::<f64>() / n);
    let at = |d: i64| dm + v * (years(d) - tm);
    Some([(first, at(first)), (last, at(last))])
}

/// The thresholds of the part (m): warning and alarm; height's either way.
pub fn limits(t: Option<&AsbuiltThresholds>, part: Part) -> (Option<f64>, Option<f64>) {
    match (t, part) {
        (Some(t), Part::Horizontal) => (t.warning.horizontal, t.alarm.horizontal),
        (Some(t), Part::Up) => (t.warning.vertical, t.alarm.vertical),
        _ => (None, None),
    }
}

/// The chart of one mark.
pub struct Chart<'a> {
    pub track: &'a AsbuiltTrack,
    pub days: &'a [Option<i64>],
    pub part: Part,
    pub thresholds: Option<AsbuiltThresholds>,
}

fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

impl canvas::Program<Message> for Chart<'_> {
    type State = ();

    fn draw(&self, _: &(), renderer: &Renderer, theme: &Theme, bounds: Rectangle, _: mouse::Cursor) -> Vec<canvas::Geometry> {
        let mut frame = canvas::Frame::new(renderer, bounds.size());
        let tk = Tokens::of(theme);
        let size = kentos_ui::theme::typography::scaled(11.0);
        let text = |frame: &mut canvas::Frame, s: String, p: Point, color: Color| {
            frame.fill_text(Text { content: s, position: p, color, size: Pixels(size), ..Text::default() })
        };
        let pts = points(self.track, self.days, self.part);
        if pts.is_empty() {
            text(&mut frame, format!("{} için {} hareketi yok.", self.track.name, self.part), Point::new(10.0, 10.0), tk.muted);
            return vec![frame.into_geometry()];
        }
        // Days when every measurement has one, else the epochs' order.
        let dated = pts.iter().all(|p| p.1.is_some());
        let xs: Vec<f64> = pts.iter().map(|p| if dated { p.1.unwrap_or_default() as f64 } else { p.0 as f64 }).collect();
        let (x0, x1) = (xs[0], xs[xs.len() - 1]);
        let (warning, alarm) = limits(self.thresholds.as_ref(), self.part);
        let both = self.part == Part::Up;
        let mut lo = 0.0_f64.min(pts.iter().map(|p| p.2).fold(f64::INFINITY, f64::min));
        let mut hi = 0.0_f64.max(pts.iter().map(|p| p.2).fold(f64::NEG_INFINITY, f64::max));
        for l in [warning, alarm].into_iter().flatten() {
            hi = hi.max(l * 1.1);
            if both {
                lo = lo.min(-l * 1.1);
            }
        }
        if hi - lo < 1e-6 {
            hi += 0.001;
            lo -= 0.001;
        }
        // Round ticks: a step of 1, 2 or 5 times a power of ten (mm), the range widened to whole steps.
        let step = nice((hi - lo) * 1000.0 / 4.0) / 1000.0;
        let (lo, hi) = ((lo / step).floor() * step, (hi / step).ceil() * step);
        let (left, right, top, bottom) = (64.0_f32, bounds.width - 16.0, 28.0_f32, bounds.height - 34.0);
        let x = |v: f64| if x1 > x0 { left + ((v - x0) / (x1 - x0)) as f32 * (right - left) } else { (left + right) / 2.0 };
        let y = |v: f64| bottom - ((v - lo) / (hi - lo)) as f32 * (bottom - top);
        // The bands: beyond the warning, beyond the alarm (height's either way).
        let band = |frame: &mut canvas::Frame, from: f64, to: f64, color: Color| {
            let (a, b) = (y(from.min(hi)), y(to.min(hi)));
            frame.fill_rectangle(Point::new(left, a.min(b)), iced::Size::new(right - left, (a - b).abs()), color);
        };
        let levels = [(warning, alarm.unwrap_or(hi), tk.warning), (alarm, hi, tk.danger)];
        for (from, to, color) in levels {
            if let Some(from) = from {
                band(&mut frame, from, to.max(from), alpha(color, 0.12));
                if both {
                    band(&mut frame, -to.max(from), -from, alpha(color, 0.12));
                }
            }
        }
        // The axes, nought and four ticks of the value (mm).
        let solid = |w: f32, c: Color| Stroke::default().with_width(w).with_color(c);
        frame.stroke(&Path::line(Point::new(left, top), Point::new(left, bottom)), solid(1.0, tk.border));
        frame.stroke(&Path::line(Point::new(left, bottom), Point::new(right, bottom)), solid(1.0, tk.border));
        frame.stroke(&Path::line(Point::new(left, y(0.0)), Point::new(right, y(0.0))), solid(1.0, tk.faint));
        let ticks = ((hi - lo) / step).round() as i32;
        let decimals = usize::from(step * 1000.0 < 1.0);
        for k in 0..=ticks {
            let v = lo + step * f64::from(k);
            let at = y(v);
            frame.stroke(&Path::line(Point::new(left - 4.0, at), Point::new(left, at)), solid(1.0, tk.border));
            let words = format!("{:.*}", decimals, v * 1000.0).replace('.', ",").replace("-0,0", "0,0");
            text(&mut frame, if words == "-0" { "0".into() } else { words }, Point::new(6.0, at - size * 0.6), tk.muted);
        }
        // The thresholds' lines.
        for (l, color, words) in [(warning, tk.warning, "uyarı"), (alarm, tk.danger, "alarm")] {
            let Some(l) = l else { continue };
            for v in if both { vec![l, -l] } else { vec![l] } {
                frame.stroke(&Path::line(Point::new(left, y(v)), Point::new(right, y(v))), solid(1.5, color));
            }
            text(&mut frame, format!("{words} {:.1} mm", l * 1000.0).replace('.', ","), Point::new(right - 96.0, y(l) - size - 2.0), color);
        }
        // The fitted line, dashed.
        let fitted = line(self.track, self.days, self.part).filter(|_| dated);
        if let Some([a, b]) = fitted {
            frame.stroke(
                &Path::line(Point::new(x(a.0 as f64), y(a.1)), Point::new(x(b.0 as f64), y(b.1))),
                Stroke { line_dash: LineDash { segments: &[6.0, 4.0], offset: 0 }, ..solid(1.5, tk.text) },
            );
        }
        // The measurements, joined; beyond a threshold in its level's colour.
        let at: Vec<Point> = pts.iter().zip(&xs).map(|(p, &xv)| Point::new(x(xv), y(p.2))).collect();
        if at.len() > 1 {
            frame.stroke(
                &Path::new(|p| {
                    p.move_to(at[0]);
                    for q in &at[1..] {
                        p.line_to(*q);
                    }
                }),
                solid(2.0, tk.accent),
            );
        }
        for (p, q) in pts.iter().zip(&at) {
            let beyond = |l: Option<f64>| l.is_some_and(|l| if both { p.2.abs() > l } else { p.2 > l });
            let color = if beyond(alarm) {
                tk.danger
            } else if beyond(warning) {
                tk.warning
            } else {
                tk.accent
            };
            frame.fill(&Path::circle(*q, 4.0), color);
        }
        // The epochs' days (or order) under their measurements, the last one's ending at the chart's right.
        let wide = size * 5.6;
        for (p, q) in pts.iter().zip(&at) {
            let words = match p.1 {
                Some(d) if dated => day_words(d),
                _ => format!("{}. dönem", p.0 + 1),
            };
            let from = (q.x - wide / 2.0).clamp(left - wide / 2.0, right - wide);
            text(&mut frame, words, Point::new(from, bottom + 6.0), tk.muted);
        }
        let title = format!(
            "{} · {} hareket (mm) · — ölçüler{}",
            self.track.name,
            self.part,
            if fitted.is_some() { ", - - doğru" } else { "" }
        );
        text(&mut frame, title, Point::new(left, 6.0), tk.text);
        vec![frame.into_geometry()]
    }
}

/// Grafik's mark: the one chosen, else the first in alarm, else the first.
pub fn chosen<'a>(r: &'a AsbuiltChecked, mark: &str) -> Option<&'a AsbuiltTrack> {
    r.tracks
        .iter()
        .find(|t| t.name == mark)
        .or_else(|| r.tracks.iter().find(|t| t.state == Some(AsbuiltState::Alarm)))
        .or_else(|| r.tracks.first())
}

impl crate::app::App {
    /// Grafik (docs/adr/5220): the mark and the part chosen above the chart; the thresholds as typed.
    pub(crate) fn asbuilt_chart<'a>(&'a self, r: &'a AsbuiltChecked) -> Element<'a, Message> {
        use crate::asbuilt_trend::Change;
        use crate::asbuilt_window::Event;
        let s = &self.asbuilt;
        let Some(track) = chosen(r, &s.trend.mark) else {
            return label::caption("İzlenen nokta yok.").into();
        };
        let names: Vec<(String, String)> = r.tracks.iter().map(|t| (t.name.clone(), t.name.clone())).collect();
        let thresholds = crate::asbuilt_trend::ask(&s.trend).ok().and_then(|(_, t)| t);
        let head = row![
            label::caption("Nokta"),
            container(crate::style::fields::select_owned(names, &track.name, |v| {
                Message::Asbuilt(Event::Trend(Change::Mark(v)))
            }))
            .width(120),
            Segmented::new([Part::Horizontal, Part::East, Part::North, Part::Up], s.trend.part, |v| {
                Message::Asbuilt(Event::Trend(Change::Part(v)))
            })
        ]
        .spacing(8)
        .align_y(Center);
        let chart = Chart { track, days: &s.days, part: s.trend.part, thresholds };
        column![head, iced::widget::canvas(chart).width(Fill).height(kentos_ui::theme::typography::scaled(280.0))]
            .spacing(8)
            .into()
    }
}

/// 1, 2 or 5 times a power of ten, the smallest not below `v` (v > 0).
fn nice(v: f64) -> f64 {
    let p = 10f64.powf(v.max(1e-9).log10().floor());
    [1.0, 2.0, 5.0, 10.0].into_iter().map(|k| k * p).find(|s| *s >= v).unwrap_or(10.0 * p)
}

/// A day number as `GG.AA.YYYY`.
fn day_words(n: i64) -> String {
    let z = n + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{d:02}.{m:02}.{y}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn days_go_both_ways() {
        assert_eq!(day_number("1970-01-01"), Some(0));
        assert_eq!(day_number("2026-10-15").map(day_words).as_deref(), Some("15.10.2026"));
        assert_eq!(day_number("2028-02-29").map(day_words).as_deref(), Some("29.02.2028"));
        assert_eq!(day_number("2026-07-15").zip(day_number("2026-01-15")).map(|(a, b)| a - b), Some(181));
    }
}
