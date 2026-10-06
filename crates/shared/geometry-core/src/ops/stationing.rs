//! Hat boyunca kilometre, Km yaz (docs/adr/0189): the stations of a route at
//! the multiples of an interval of its km (and its ends), and what is written
//! at each: a tick square to the route, the km as a text square to it and
//! upright, a cross-section line and a point at an offset. The route and its
//! points are Obje üzerinde nokta's (`tools::point_calc::Walk`, docs/adr/0188
//! §1); the km is written by its rule (`km_text`). The independent reference
//! is `scripts/fixtures/stationing_cases.py` (`fixtures/stationing/v1/cases.json`).

use crate::api::Op;
use crate::entity::Entity;
use crate::jsmath::{atan2, cos, js_floor, js_max, js_round, sin};
use crate::op;
use crate::text::TextAlign;
use crate::tools::point_calc::{Walk, km_text};
use crate::vec2::Vec2;

/// The most stations a route takes: more is refused, the interval to be widened.
pub const MOST_STATIONS: usize = 20_000;

/// Why a route gives no stations, in the tool's words.
pub const NO_ROUTE: &str = "Bu nesnenin üzerinde yürünecek tek bir yolu yok: çizgi, çoklu çizgi, yay, daire, elips, eğri ya da tek parçalı alan seçin.";
/// The interval is no length.
pub const BAD_INTERVAL: &str = "Aralık sıfırdan büyük bir uzunluk olmalı.";

/// Where the stations are (docs/adr/0189 §2).
#[derive(Clone, Debug, PartialEq)]
pub struct Rules {
    /// The interval of the km, metres, over 0.
    pub interval: f64,
    /// The km of the route's first point (its last when `reverse`), metres.
    pub start: f64,
    /// Walked from its drawn end.
    pub reverse: bool,
    /// The route's start and end are stations too.
    pub ends: bool,
    /// The ends' decimals (the project's length decimals).
    pub decimals: usize,
}

crate::json_struct!(Rules {
    interval,
    start,
    reverse,
    ends,
    decimals
});

/// A station: how far along the route it is, its km and how that is
/// written, its place and the unit direction the route runs there.
#[derive(Clone, Debug, PartialEq)]
pub struct Station {
    pub s: f64,
    pub km: f64,
    pub text: String,
    pub point: Vec2,
    pub tangent: Vec2,
}

crate::json_struct!(out Station {
    s,
    km,
    text,
    point,
    tangent
});

/// The side of the route a km text is written on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Side {
    Left,
    Right,
}

/// What is written at each station (docs/adr/0189 §3), in world metres.
#[derive(Clone, Debug, PartialEq)]
pub struct Look {
    /// The side of the km text; none: no text.
    pub text: Option<Side>,
    /// The text's height.
    pub height: f64,
    /// The tick's half length, each side of the route; 0: no tick.
    pub tick: f64,
    /// The cross-section's half width, each side; 0: none.
    pub section: f64,
    /// The point's offset, the right of the way positive; none: no point.
    pub point: Option<f64>,
}

impl crate::api::json::FromJson for Look {
    fn from_json(v: &crate::api::json::Json) -> Result<Look, String> {
        use crate::api::json::read_field;
        let text: Option<String> = read_field(v, "text")?;
        let text = match text.as_deref() {
            None => None,
            Some("left") => Some(Side::Left),
            Some("right") => Some(Side::Right),
            Some(other) => {
                return Err(format!(
                    "yazının yanı “left” ya da “right” olmalı, “{other}” değil"
                ));
            }
        };
        Ok(Look {
            text,
            height: read_field(v, "height")?,
            tick: read_field(v, "tick")?,
            section: read_field(v, "section")?,
            point: read_field(v, "point")?,
        })
    }
}

/// A km text: where it is anchored, its turn (degrees counter-clockwise
/// from east), how it hangs on the anchor (none: the left of its baseline)
/// and what it says.
#[derive(Clone, Debug, PartialEq)]
pub struct StationText {
    pub p: Vec2,
    pub rotation: f64,
    pub align: Option<TextAlign>,
    pub text: String,
}

crate::json_struct!(out StationText {
    p,
    rotation,
    align,
    text
});

/// A line with the km of its station (a cross-section, a point's `Km`).
#[derive(Clone, Debug, PartialEq)]
pub struct Marked {
    pub a: Vec2,
    pub b: Vec2,
    pub km: String,
}

crate::json_struct!(out Marked { a, b, km });

/// A point with the km of its station.
#[derive(Clone, Debug, PartialEq)]
pub struct MarkedPoint {
    pub p: Vec2,
    pub km: String,
}

crate::json_struct!(out MarkedPoint { p, km });

/// What Km yaz writes along a route: the stations and the objects at them.
#[derive(Clone, Debug, PartialEq)]
pub struct Stationing {
    pub stations: Vec<Station>,
    pub texts: Vec<StationText>,
    pub ticks: Vec<Marked>,
    pub sections: Vec<Marked>,
    pub points: Vec<MarkedPoint>,
}

crate::json_struct!(out Stationing {
    stations,
    texts,
    ticks,
    sections,
    points
});

/// The decimals an interval is written with: the fewest (up to six) that
/// hold it, 20 m → 0, 12.5 m → 1.
fn interval_decimals(interval: f64) -> usize {
    let mut scale = 1.0;
    for d in 0..=6 {
        let v = interval * scale;
        if (v - js_round(v)).abs() <= 1e-6 * js_max(v.abs(), 1.0) {
            return d;
        }
        scale *= 10.0;
    }
    6
}

/// The stations of `e`'s route (docs/adr/0189 §2): the km multiples of the
/// interval along it, and its ends when asked; a station within 10⁻⁹ m of
/// another is one, and round a closed route the end is the start's station.
/// Refused with the tool's words: no route, no interval, too many stations.
pub fn stations(e: &crate::entity::Shape, rules: &Rules) -> Result<Vec<Station>, String> {
    let Some(walk) = Walk::new(e, rules.reverse) else {
        return Err(NO_ROUTE.to_owned());
    };
    if !(rules.interval > 0.0 && rules.interval.is_finite() && rules.start.is_finite()) {
        return Err(BAD_INTERVAL.to_owned());
    }
    let length = walk.length();
    let (start, interval) = (rules.start, rules.interval);
    let end = start + length;
    // The multiples k·interval in [start, end], k whole, a multiple 10⁻⁹ m outside still in.
    let first = ((start - 1e-9) / interval).ceil();
    let last = js_floor((end + 1e-9) / interval);
    let count = if last >= first {
        last - first + 1.0
    } else {
        0.0
    };
    if count > MOST_STATIONS as f64 {
        return Err(format!(
            "Bu aralıkla {} istasyon olur, en çok {} yazılır; aralığı büyütün.",
            count as u64, MOST_STATIONS
        ));
    }
    let step = interval_decimals(interval);
    let mut at: Vec<(f64, f64, usize)> = Vec::new();
    if rules.ends {
        at.push((0.0, start, rules.decimals));
    }
    let mut k = first;
    while k <= last {
        let km = k * interval;
        let s = (km - start).clamp(0.0, length);
        at.push((s, km, step));
        k += 1.0;
    }
    if rules.ends {
        at.push((length, end, rules.decimals));
    }
    at.sort_by(|a, b| a.0.total_cmp(&b.0));
    // One station where two meet: the multiple's, written with the interval's decimals.
    let mut kept: Vec<(f64, f64, usize)> = Vec::new();
    for (s, km, d) in at {
        match kept.last_mut() {
            Some(last) if (s - last.0).abs() <= 1e-9 => {
                if d == step && last.2 != step {
                    *last = (last.0, km, d);
                }
            }
            _ => kept.push((s, km, d)),
        }
    }
    // Round a closed route its end is its start: one station there, the start's.
    if walk.closed() && kept.len() > 1 && kept[0].0 <= 1e-9 {
        kept.retain(|&(s, _, _)| s < length - 1e-9);
    }
    Ok(kept
        .into_iter()
        .map(|(s, km, d)| {
            let (point, tangent) = walk.frame(s);
            Station {
                s,
                km,
                text: km_text(km, d),
                point,
                tangent,
            }
        })
        .collect())
}

/// The objects at the stations (docs/adr/0189 §3): a tick square to the
/// route, the km text past it on its side, square to the route and upright
/// (past a right angle either way it turns half a turn and hangs on its
/// right), a cross-section, a point at an offset.
pub fn objects(stations: Vec<Station>, look: &Look) -> Stationing {
    let mut out = Stationing {
        stations: Vec::new(),
        texts: Vec::new(),
        ticks: Vec::new(),
        sections: Vec::new(),
        points: Vec::new(),
    };
    for st in &stations {
        let (p, t) = (st.point, st.tangent);
        // The left of the way and a point `d` along it from the station.
        let left = Vec2::new(-t.y, t.x);
        let off = |d: f64| Vec2::new(p.x + left.x * d, p.y + left.y * d);
        if look.tick > 0.0 {
            out.ticks.push(Marked {
                a: off(look.tick),
                b: off(-look.tick),
                km: st.text.clone(),
            });
        }
        if let Some(side) = look.text {
            let way = match side {
                Side::Left => 1.0,
                Side::Right => -1.0,
            };
            let n = Vec2::new(left.x * way, left.y * way);
            let from = js_max(look.tick, 0.0) + look.height / 2.0;
            let a = atan2(n.y, n.x);
            let upright = a > std::f64::consts::FRAC_PI_2 || a < -std::f64::consts::FRAC_PI_2;
            let (turn, align) = if upright {
                (a + std::f64::consts::PI, Some(TextAlign::BaselineRight))
            } else {
                (a, None)
            };
            // Beside the tick's line, not on it: the baseline a quarter of the height off it, on the letters' side.
            let up = Vec2::new(-sin(turn), cos(turn));
            let gap = look.height / 4.0;
            let mut rotation = turn * 180.0 / std::f64::consts::PI;
            if rotation > 180.0 {
                rotation -= 360.0;
            }
            out.texts.push(StationText {
                p: Vec2::new(p.x + n.x * from + up.x * gap, p.y + n.y * from + up.y * gap),
                rotation,
                align,
                text: st.text.clone(),
            });
        }
        if look.section > 0.0 {
            out.sections.push(Marked {
                a: off(look.section),
                b: off(-look.section),
                km: st.text.clone(),
            });
        }
        if let Some(offset) = look.point.filter(|o| o.is_finite()) {
            // The right of the way positive.
            out.points.push(MarkedPoint {
                p: off(-offset),
                km: st.text.clone(),
            });
        }
    }
    out.stations = stations;
    out
}

/// Km yaz's whole answer: the stations of `e`'s route and the objects at them.
pub fn stationing(
    e: &crate::entity::Shape,
    rules: &Rules,
    look: &Look,
) -> Result<Stationing, String> {
    stations(e, rules).map(|s| objects(s, look))
}

/// The op's answer: what is written, or why nothing is.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    pub stationing: Option<Stationing>,
    pub problem: Option<String>,
}

crate::json_struct!(out Answer { stationing, problem });

pub(crate) static OPS: &[Op] =
    &[op!(
        "stationing",
        |e: Entity, rules: Rules, look: Look| match stationing(&e.shape, &rules, &look) {
            Ok(s) => Answer {
                stationing: Some(s),
                problem: None,
            },
            Err(why) => Answer {
                stationing: None,
                problem: Some(why),
            },
        }
    )];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entity::Shape;

    fn line() -> Shape {
        Shape::Line {
            a: Vec2::new(0.0, 0.0),
            b: Vec2::new(45.0, 0.0),
        }
    }

    fn rules(interval: f64, start: f64, ends: bool) -> Rules {
        Rules {
            interval,
            start,
            reverse: false,
            ends,
            decimals: 3,
        }
    }

    #[test]
    fn the_multiples_of_the_interval_and_the_ends() {
        let s = stations(&line(), &rules(20.0, 0.0, true)).unwrap();
        let texts: Vec<&str> = s.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(texts, ["0+000", "0+020", "0+040", "0+045.000"]);
        let s = stations(&line(), &rules(20.0, 15.0, false)).unwrap();
        let at: Vec<f64> = s.iter().map(|s| s.s).collect();
        assert_eq!(at, [5.0, 25.0, 45.0]);
        assert_eq!(interval_decimals(12.5), 1);
        assert!(stations(&line(), &rules(0.0, 0.0, true)).is_err());
        assert!(stations(&line(), &rules(0.001, 0.0, true)).is_err());
    }

    #[test]
    fn a_text_square_to_the_route_reads_upright() {
        let s = stations(&line(), &rules(20.0, 0.0, false)).unwrap();
        let look = Look {
            text: Some(Side::Left),
            height: 2.0,
            tick: 1.0,
            section: 0.0,
            point: Some(3.0),
        };
        let o = objects(s, &look);
        // East: the left is north, the text reads up from 2 m north of the station, half a metre west of the tick's line.
        assert!((o.texts[0].p.x + 0.5).abs() < 1e-12 && (o.texts[0].p.y - 2.0).abs() < 1e-12);
        assert_eq!((o.texts[0].rotation, o.texts[0].align), (90.0, None));
        // The point 3 m to the right: south.
        assert_eq!(o.points[1].p, Vec2::new(20.0, -3.0));
        assert_eq!(o.ticks[2].a, Vec2::new(40.0, 1.0));
    }
}
