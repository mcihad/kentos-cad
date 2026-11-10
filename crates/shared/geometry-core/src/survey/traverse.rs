//! Poligon hesabı: a traverse from a known point oriented on a known back
//! point, through measured angles and distances, to new points; closed on a
//! known end point (bağlı, or back on itself: kapalı) or left open (açık).
//!
//! Each angle (kırılma açısı) is measured at a station clockwise from the
//! previous point (the back point at the start) to the next one, so a leg's
//! bearing is the reverse of the previous leg's plus the angle. When the end
//! is oriented on a fore point too, the bearing it gives is compared with the
//! known one: the angular misclosure (computed − known) is taken off in equal
//! parts at every angle. The coordinate misclosure (sum of the increments −
//! the known difference from start to end) is then taken off every leg in
//! proportion to its length (the compass rule, "pusula kuralı").
//!
//! With the project's grid (its system and mean ellipsoidal height,
//! docs/adr/0171 §4) the lengths are measured on the ground: the traverse
//! is computed with them, each leg's scale and height factor are taken on
//! its provisional line and it is computed again with the lengths on the
//! grid, twice; the adjustment is the last's.
//!
//! Heights (docs/adr/4100), trigonometric heighting as İTÜ's Topografya
//! (M. Z. Coşkun, Trigonometrik nivelman) gives it: a leg's height difference
//! is S·cot z + i − t, S its horizontal length as measured, z the zenith angle
//! at its station, i the instrument's and t the target's height; with a
//! refraction coefficient k the earth's curvature and refraction (1 − k)·S²/(2R)
//! is added as a field book's reduction adds it (docs/adr/0169 §3: R the earth's
//! mean radius, 6 371 000 m). Closed on
//! a known height, the misclosure (sum − known difference) is taken off the
//! legs in proportion to their lengths. With the geoid's undulations each
//! leg's height factor is then taken at its own ellipsoidal height (the mean
//! of its ends' H + N) instead of the project's mean.
//!
//! Reciprocal zenith angles (docs/adr/4500): a leg sighted back from its far end too (the zenith angle there to its
//! station, that set-up's instrument height i′ and the target's height t′ at the station) has the mean of its two
//! one-way differences as its height difference, ((S·cot z + i − t) − (S·cot z′ + i′ − t′))/2: the curvature and
//! refraction term, the same both ways with one k, falls out. When both sights run between the same two points (each
//! end's instrument at the height of the target the other end sighted there: i = t′, t = i′), the pair gives its
//! refraction coefficient, Baselga, García-Asenjo and Garrigues' (J. Surv. Eng. 2014, eq. 11): k = ½·sin z + ½·sin z′ +
//! (2R/s)·cos((z + z′)/2), s the chord between the sight's ends (S / sin z) and R the earth's mean radius.

use super::{Unit, bearing, distinct, finite, positive, signed};
use crate::api::Op;
use crate::crs::ground::{Grid, grid_factor};
use crate::jsmath::{PI, cos, js_hypot, sin};
use crate::op;
use crate::vec2::Vec2;

#[derive(Clone, Debug, PartialEq)]
pub struct TraverseInput {
    pub unit: String,
    /// The known start point and the known point it is oriented on.
    pub start: Vec2,
    pub back: Vec2,
    /// The known end point (bağlı or kapalı), or none (açık).
    pub end: Option<Vec2>,
    /// The known point the end is oriented on: gives the angular misclosure.
    pub fore: Option<Vec2>,
    /// Angles at the start, at every new point with a leg after it and, with a fore point, at the end.
    pub angles: Vec<f64>,
    /// Horizontal lengths of the legs, in order.
    pub distances: Vec<f64>,
    /// The project's grid: the lengths are on the ground and are taken to
    /// it (docs/adr/0171 §4); none: they are the grid's already.
    pub grid: Option<Grid>,
    /// The points' heights by trigonometric heighting (docs/adr/4100); none: a traverse in plan only.
    pub heights: Option<TraverseHeights>,
}

crate::json_struct!(TraverseInput {
    unit,
    start,
    back,
    end,
    fore,
    angles,
    distances,
    grid,
    heights
});

/// What a traverse's heights are computed from (docs/adr/4100): the start's height and, closed on a known height, the
/// end's (kapalı: the start's again); at each leg's station the zenith angle to its next point (in the unit), the
/// instrument's height and the target's height there (m); the refraction coefficient k (the project's; none: the
/// earth's curvature and refraction are not taken in); the geoid's undulations N (m) at the start, at each point after
/// it and at the end, so that each leg's height factor is taken at its own ellipsoidal height (none: at the project's
/// mean).
#[derive(Clone, Debug, PartialEq)]
pub struct TraverseHeights {
    pub start: f64,
    pub end: Option<f64>,
    pub zeniths: Vec<f64>,
    pub instruments: Vec<f64>,
    pub targets: Vec<f64>,
    pub refraction: Option<f64>,
    pub undulations: Option<Vec<f64>>,
    /// Each leg's reciprocal sight (docs/adr/4500), none for a leg not sighted back; none: no leg was.
    pub back: Option<Vec<Option<TraverseBackSight>>>,
}

crate::json_struct!(TraverseHeights {
    start,
    end,
    zeniths,
    instruments,
    targets,
    refraction,
    undulations,
    back
});

/// A leg sighted back from its far end (docs/adr/4500): the zenith angle there to the leg's station (in the unit), the
/// instrument's height at the far end and the target's height at the station (m).
#[derive(Clone, Debug, PartialEq)]
pub struct TraverseBackSight {
    pub zenith: f64,
    pub instrument: f64,
    pub target: f64,
}

crate::json_struct!(TraverseBackSight { zenith, instrument, target });

#[derive(Clone, Debug, PartialEq)]
pub struct TraverseLeg {
    /// Adjusted bearing, in the unit.
    pub bearing: f64,
    pub distance: f64,
    /// Adjusted coordinate increments (ΔY east, ΔX north).
    pub dy: f64,
    pub dx: f64,
    /// Corrections the adjustment added to the increments.
    pub vy: f64,
    pub vx: f64,
    /// With the project's grid: the length as measured (on the ground) and
    /// the leg's scale and height factor; `distance` is then the grid's.
    pub ground: Option<f64>,
    pub scale: Option<f64>,
    pub height_factor: Option<f64>,
    /// With heights (docs/adr/4100): the adjusted height difference, the correction the height adjustment added, and
    /// the ellipsoidal height its height factor was taken at (its own, with the undulations).
    pub dh: Option<f64>,
    pub vh: Option<f64>,
    pub height: Option<f64>,
    /// Sighted back between the same two points (docs/adr/4500): the refraction coefficient its reciprocal zenith
    /// angles give.
    pub refraction: Option<f64>,
}

crate::json_struct!(out TraverseLeg {
    bearing,
    distance,
    dy,
    dx,
    vy,
    vx,
    ground,
    scale,
    height_factor => "heightFactor",
    dh,
    vh,
    height,
    refraction
});

#[derive(Clone, Debug, PartialEq)]
pub struct TraverseResult {
    /// The new points, adjusted, in order (the end point is not repeated).
    pub points: Vec<Vec2>,
    pub legs: Vec<TraverseLeg>,
    /// Angular misclosure (computed − known) and the correction added to each angle, in the unit.
    pub angle_misclosure: Option<f64>,
    pub angle_correction: Option<f64>,
    /// Coordinate misclosure (sum of increments − known difference) in Y and X, and its length.
    pub fy: Option<f64>,
    pub fx: Option<f64>,
    pub linear_misclosure: Option<f64>,
    /// Total length of the legs.
    pub length: f64,
    /// With heights (docs/adr/4100): the new points' heights, adjusted, in `points`' order; the height misclosure (sum
    /// of the height differences − the known difference) when the end's height is known.
    pub heights: Option<Vec<f64>>,
    pub height_misclosure: Option<f64>,
}

crate::json_struct!(out TraverseResult {
    points,
    legs,
    angle_misclosure => "angleMisclosure",
    angle_correction => "angleCorrection",
    fy,
    fx,
    linear_misclosure => "linearMisclosure",
    length,
    heights,
    height_misclosure => "heightMisclosure"
});

/// A traverse's heights worked out (docs/adr/4100): each leg's adjusted height difference and its correction, every
/// point's height (the start's, each point after it), the misclosure, each leg's own ellipsoidal height.
struct Heights {
    dh: Vec<f64>,
    vh: Vec<Option<f64>>,
    at: Vec<f64>,
    misclosure: Option<f64>,
    own: Option<Vec<f64>>,
    /// Each leg's refraction coefficient from its reciprocal zenith angles (docs/adr/4500).
    refraction: Vec<Option<f64>>,
}

/// The heights of `input` (its legs' horizontal lengths as measured).
fn heights_of(input: &TraverseInput, h: &TraverseHeights) -> Result<Heights, String> {
    let unit = Unit::parse(&input.unit)?;
    let legs = input.distances.len();
    for (list, what) in [(&h.zeniths, "başucu açısı"), (&h.instruments, "alet yüksekliği"), (&h.targets, "işaret yüksekliği")] {
        if list.len() != legs {
            return Err(format!("{legs} kenarlı bu poligonda {legs} {what} olmalı, {} verildi.", list.len()));
        }
    }
    finite(h.start, "Başlangıç noktasının kotu")?;
    if let Some(e) = h.end {
        finite(e, "Bitiş noktasının kotu")?;
    }
    for i in 0..legs {
        let z = unit.rad(finite(h.zeniths[i], &format!("{}. başucu açısı", i + 1))?);
        if !(z > 0.0 && z < PI) {
            return Err(format!(
                "{}. başucu açısı 0 ile {} arasında olmalı (yatay nişan {}).",
                i + 1,
                if unit == Unit::GRAD { "200 grad" } else { "180°" },
                if unit == Unit::GRAD { "100 grad" } else { "90°" }
            ));
        }
        finite(h.instruments[i], &format!("{}. alet yüksekliği", i + 1))?;
        finite(h.targets[i], &format!("{}. işaret yüksekliği", i + 1))?;
    }
    if let Some(k) = h.refraction {
        finite(k, "Kırılma katsayısı")?;
    }
    // docs/adr/4500: a back sight a leg, each finite, its zenith angle between straight up and straight down.
    if let Some(back) = &h.back {
        if back.len() != legs {
            return Err(format!("{legs} kenarlı bu poligonda geri nişanlar {legs} kenar için olmalı, {} verildi.", back.len()));
        }
        for (i, b) in back.iter().enumerate() {
            let Some(b) = b else { continue };
            let z = unit.rad(finite(b.zenith, &format!("{}. kenarın geri başucu açısı", i + 1))?);
            if !(z > 0.0 && z < PI) {
                return Err(format!(
                    "{}. kenarın geri başucu açısı 0 ile {} arasında olmalı (yatay nişan {}).",
                    i + 1,
                    if unit == Unit::GRAD { "200 grad" } else { "180°" },
                    if unit == Unit::GRAD { "100 grad" } else { "90°" }
                ));
            }
            finite(b.instrument, &format!("{}. kenarın geri nişandaki alet yüksekliği", i + 1))?;
            finite(b.target, &format!("{}. kenarın geri nişandaki işaret yüksekliği", i + 1))?;
        }
    }
    if let Some(n) = &h.undulations {
        if n.len() != legs + 1 {
            return Err(format!(
                "Geoit ayrımları başlangıçta, her yeni noktada ve bitişte: {} olmalı, {} verildi.",
                legs + 1,
                n.len()
            ));
        }
        for (i, v) in n.iter().enumerate() {
            finite(*v, &format!("{}. geoit ayrımı", i + 1))?;
        }
    }
    // Each leg's height difference as measured, the curvature and refraction taken in with k; sighted back
    // (docs/adr/4500), the mean of its two one-way differences, which leaves the curvature and refraction out, and the
    // pair's refraction coefficient.
    let mut raw = Vec::with_capacity(legs);
    let mut refraction = Vec::with_capacity(legs);
    for (i, &s) in input.distances.iter().enumerate() {
        let z = unit.rad(h.zeniths[i]);
        let forward = s * cos(z) / sin(z) + h.instruments[i] - h.targets[i];
        match h.back.as_ref().and_then(|b| b[i].as_ref()) {
            Some(b) => {
                let zb = unit.rad(b.zenith);
                let backward = s * cos(zb) / sin(zb) + b.instrument - b.target;
                raw.push((forward - backward) / 2.0);
                // The pair's k only for sights between the same two points (the formula's chord).
                let same = b.target == h.instruments[i] && b.instrument == h.targets[i];
                let chord = s / sin(z);
                let r = super::fieldbook::EARTH_RADIUS;
                refraction.push(same.then(|| sin(z) / 2.0 + sin(zb) / 2.0 + 2.0 * r / chord * cos((z + zb) / 2.0)));
            }
            None => {
                let mut d = forward;
                if let Some(k) = h.refraction {
                    d += super::fieldbook::curvature(s, k);
                }
                raw.push(d);
                refraction.push(None);
            }
        }
    }
    // Closed on a known height: the misclosure taken off in proportion to the legs' lengths.
    let length: f64 = input.distances.iter().sum();
    let misclosure = h.end.map(|e| raw.iter().sum::<f64>() - (e - h.start));
    let mut dh = Vec::with_capacity(legs);
    let mut vh = Vec::with_capacity(legs);
    let mut heights = vec![h.start];
    for (i, d) in raw.iter().enumerate() {
        let v = misclosure.map(|f| -f * input.distances[i] / length);
        let adjusted = d + v.unwrap_or(0.0);
        dh.push(adjusted);
        vh.push(v);
        let last = heights[heights.len() - 1];
        heights.push(last + adjusted);
    }
    // Each leg's own ellipsoidal height: the mean of its ends' H + N.
    let own = h.undulations.as_ref().map(|n| {
        (0..legs).map(|i| ((heights[i] + n[i]) + (heights[i + 1] + n[i + 1])) / 2.0).collect()
    });
    Ok(Heights { dh, vh, at: heights, misclosure, own, refraction })
}

/// `result` with the heights `h` written into it: its legs' height differences and corrections, its new points'
/// heights (the end's, a known point, not among them) and the misclosure.
fn with_heights(mut result: TraverseResult, h: Heights, end_known: bool) -> TraverseResult {
    for (i, leg) in result.legs.iter_mut().enumerate() {
        leg.dh = Some(h.dh[i]);
        leg.vh = h.vh[i];
        leg.refraction = h.refraction[i];
    }
    let mut points = h.at[1..].to_vec();
    if end_known {
        points.pop();
    }
    result.heights = Some(points);
    result.height_misclosure = h.misclosure;
    result
}

/// Computes and adjusts a traverse; with the project's grid, three times:
/// on the lengths as measured, then twice on the lengths taken to the grid
/// with the legs' factors on the previous one's lines (the first's are off
/// by the reduction itself, a few decimetres in a kilometre; the second's
/// by micrometres).
pub fn traverse(input: &TraverseInput) -> Result<TraverseResult, String> {
    let end_known = input.end.is_some();
    let Some(grid) = &input.grid else {
        let result = adjusted(input)?;
        return match &input.heights {
            Some(h) => {
                let heights = heights_of(input, h)?;
                Ok(with_heights(result, heights, end_known))
            }
            None => Ok(result),
        };
    };
    let mut result = adjusted(input)?;
    // Docs/adr/4100: the heights on the lengths as measured; with the undulations each leg's factor at its own
    // ellipsoidal height.
    let heights = match &input.heights {
        Some(h) => Some(heights_of(input, h)?),
        None => None,
    };
    let own = heights.as_ref().and_then(|h| h.own.clone());
    let mut factors = Vec::new();
    for _ in 0..2 {
        let mut at = input.start;
        factors.clear();
        for (i, leg) in result.legs.iter().enumerate() {
            let next = Vec2::new(at.x + leg.dy, at.y + leg.dx);
            let factor = match &own {
                Some(own) => grid_factor(&Grid { system: grid.system.clone(), height: own[i] }, at, next)?,
                None => grid_factor(grid, at, next)?,
            };
            factors.push(factor);
            at = next;
        }
        result = adjusted(&TraverseInput {
            distances: input
                .distances
                .iter()
                .zip(&factors)
                .map(|(d, f)| d * f.combined())
                .collect(),
            grid: None,
            heights: None,
            ..input.clone()
        })?;
    }
    for (i, ((leg, f), ground)) in result.legs.iter_mut().zip(&factors).zip(&input.distances).enumerate() {
        leg.ground = Some(*ground);
        leg.scale = Some(f.scale);
        leg.height_factor = Some(f.height_factor);
        leg.height = own.as_ref().map(|o| o[i]);
    }
    Ok(match heights {
        Some(h) => with_heights(result, h, end_known),
        None => result,
    })
}

/// The traverse with the lengths as given.
fn adjusted(input: &TraverseInput) -> Result<TraverseResult, String> {
    let unit = Unit::parse(&input.unit)?;
    let legs = input.distances.len();
    if legs == 0 {
        return Err("Poligonda en az bir kenar olmalı.".into());
    }
    if input.fore.is_some() && input.end.is_none() {
        return Err("Bitiş yöneltme noktası verildiyse bitiş noktası da verilmeli.".into());
    }
    let want = legs + usize::from(input.fore.is_some());
    if input.angles.len() != want {
        return Err(format!(
            "{legs} kenarlı bu poligonda {want} kırılma açısı olmalı, {} verildi.",
            input.angles.len()
        ));
    }
    distinct(
        input.start,
        input.back,
        "Başlangıç noktası ile başlangıçta bakılan nokta",
    )?;
    if let (Some(e), Some(f)) = (input.end, input.fore) {
        distinct(e, f, "Bitiş noktası ile bitişte bakılan nokta")?;
    }
    for (i, &s) in input.distances.iter().enumerate() {
        if !(finite(s, &format!("{}. kenar", i + 1))? > 0.0) {
            return Err(format!(
                "{}. kenarın uzunluğu sıfırdan büyük olmalı.",
                i + 1
            ));
        }
    }
    for (i, &b) in input.angles.iter().enumerate() {
        finite(b, &format!("{}. açı", i + 1))?;
    }

    // Bearings of the legs from the angles as measured.
    let mut t = Vec::with_capacity(legs);
    let mut back = bearing(input.start, input.back);
    for i in 0..legs {
        let leg = positive(back + unit.rad(input.angles[i]));
        t.push(leg);
        back = leg + PI;
    }

    // Angular closure on the end's fore point: the correction goes into every angle alike,
    // so the i-th leg's bearing takes i + 1 of them.
    let mut angle_misclosure = None;
    let mut correction = 0.0;
    if let (Some(end), Some(fore)) = (input.end, input.fore) {
        let computed = back + unit.rad(input.angles[legs]);
        let f = signed(computed - bearing(end, fore));
        correction = -f / input.angles.len() as f64;
        angle_misclosure = Some(unit.of(f));
    }
    for (i, leg) in t.iter_mut().enumerate() {
        *leg = positive(*leg + correction * (i + 1) as f64);
    }

    let length: f64 = input.distances.iter().sum();
    let raw: Vec<(f64, f64)> = t
        .iter()
        .zip(&input.distances)
        .map(|(&b, &s)| (s * sin(b), s * cos(b)))
        .collect();

    // Coordinate closure on the end point, shared out in proportion to the legs' lengths.
    let (mut fy, mut fx) = (None, None);
    if let Some(end) = input.end {
        let sy: f64 = raw.iter().map(|r| r.0).sum();
        let sx: f64 = raw.iter().map(|r| r.1).sum();
        fy = Some(sy - (end.x - input.start.x));
        fx = Some(sx - (end.y - input.start.y));
    }
    let mut out = Vec::with_capacity(legs);
    let mut points = Vec::with_capacity(legs);
    let mut at = input.start;
    for (i, &(dy, dx)) in raw.iter().enumerate() {
        let s = input.distances[i];
        let vy = fy.map_or(0.0, |f| -f * s / length);
        let vx = fx.map_or(0.0, |f| -f * s / length);
        at = Vec2::new(at.x + dy + vy, at.y + dx + vx);
        out.push(TraverseLeg {
            bearing: unit.of(t[i]),
            distance: s,
            dy: dy + vy,
            dx: dx + vx,
            vy,
            vx,
            ground: None,
            scale: None,
            height_factor: None,
            dh: None,
            vh: None,
            height: None,
            refraction: None,
        });
        points.push(at);
    }
    // A closed traverse ends on its known point: that one is not new.
    if input.end.is_some() {
        points.pop();
    }
    Ok(TraverseResult {
        points,
        legs: out,
        angle_misclosure,
        angle_correction: angle_misclosure.map(|_| unit.of(correction)),
        linear_misclosure: fy.zip(fx).map(|(y, x)| js_hypot(y, x)),
        fy,
        fx,
        length,
        heights: None,
        height_misclosure: None,
    })
}

/// Whether a traverse's misclosures are above the project's tolerances
/// (docs/adr/0169 §3); none where there is no misclosure or no tolerance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Closure {
    pub angle_over: Option<bool>,
    pub coord_over: Option<bool>,
}

crate::json_struct!(out Closure {
    angle_over => "angleOver",
    coord_over => "coordOver"
});

/// The angular misclosure (in `unit`) against `angle` (radians), the linear
/// (fs) against `coord` (metres); one equal to its tolerance is within it.
pub fn closure(
    unit: Unit,
    angle_misclosure: Option<f64>,
    linear_misclosure: Option<f64>,
    angle: Option<f64>,
    coord: Option<f64>,
) -> Closure {
    Closure {
        angle_over: angle_misclosure
            .zip(angle)
            .map(|(f, t)| f.abs() > unit.of(t)),
        coord_over: linear_misclosure.zip(coord).map(|(fs, t)| fs > t),
    }
}

pub(crate) const OP: Op = op!("surveyTraverse", |input: TraverseInput| traverse(&input));

/// [`closure`] in the named unit (`"grad"` or `"deg"`).
fn closure_named(
    unit: &str,
    angle_misclosure: Option<f64>,
    linear_misclosure: Option<f64>,
    angle: Option<f64>,
    coord: Option<f64>,
) -> Result<Closure, String> {
    Ok(closure(
        Unit::parse(unit)?,
        angle_misclosure,
        linear_misclosure,
        angle,
        coord,
    ))
}

pub(crate) const CLOSURE: Op = op!(
    "surveyTraverseClosure",
    |unit: String,
     angle_misclosure: Option<f64>,
     linear_misclosure: Option<f64>,
     angle: Option<f64>,
     coord: Option<f64>| {
        closure_named(&unit, angle_misclosure, linear_misclosure, angle, coord)
    }
);
