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

use super::{Unit, bearing, distinct, finite, positive, signed};
use crate::api::Op;
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
}

crate::json_struct!(TraverseInput {
    unit,
    start,
    back,
    end,
    fore,
    angles,
    distances
});

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
}

crate::json_struct!(out TraverseLeg { bearing, distance, dy, dx, vy, vx });

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
}

crate::json_struct!(out TraverseResult {
    points,
    legs,
    angle_misclosure => "angleMisclosure",
    angle_correction => "angleCorrection",
    fy,
    fx,
    linear_misclosure => "linearMisclosure",
    length
});

/// Computes and adjusts a traverse.
pub fn traverse(input: &TraverseInput) -> Result<TraverseResult, String> {
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
        "Başlangıç noktası ile yöneltme noktası",
    )?;
    if let (Some(e), Some(f)) = (input.end, input.fore) {
        distinct(e, f, "Bitiş noktası ile bitiş yöneltme noktası")?;
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
    })
}

pub(crate) const OP: Op = op!("surveyTraverse", |input: TraverseInput| traverse(&input));
