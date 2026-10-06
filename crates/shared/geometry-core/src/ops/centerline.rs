//! Orta hat (docs/adr/0190): the axis between two sides (a road's edges, a
//! stream's banks). The second side runs the first's way; matched sides (as
//! many edges, each pair parallel lines or concentric arcs of the same
//! sweep) give the centreline edge by edge, exactly; any other pair is
//! matched by the share of its lengths and sampled. The sides are Obje
//! üzerinde nokta's routes (`tools::point_calc::Walk`, docs/adr/0188 §1).
//! The independent reference is `scripts/fixtures/centerline_cases.py`
//! (`fixtures/centerline/v1/cases.json`).

use crate::api::Op;
use crate::entity::{Entity, Shape};
use crate::geom::intersect::Edge;
use crate::jsmath::{js_hypot, js_max, tan};
use crate::op;
use crate::tools::point_calc::Walk;
use crate::vec2::Vec2;

/// The most points a sampled centreline takes: more is refused, the step to be widened.
pub const MOST_POINTS: f64 = 100_000.0;

/// Why a side gives no centreline, in the tool's words.
pub const NO_ROUTE: &str = "Bu nesnenin üzerinde yürünecek tek bir yolu yok: çizgi, çoklu çizgi, yay, daire, elips, eğri ya da tek parçalı alan seçin.";
pub const CLOSED: &str = "Kapalı yolun orta hattı çizilmez; iki açık kenar seçin.";
pub const BAD_STEP: &str = "Adım sıfırdan büyük bir uzunluk olmalı.";
pub const TOO_MANY: &str = "Bu adımla çok nokta olur; adımı büyütün.";

/// How the centreline was found.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Method {
    /// Edge by edge between matched sides: exact.
    Matched,
    /// Through the middles of the sides matched by the share of their lengths.
    Sampled,
}

/// A centreline: its vertices and, between matched arcs, its bulges.
#[derive(Clone, Debug, PartialEq)]
pub struct Centerline {
    pub method: Method,
    pub pts: Vec<Vec2>,
    pub bulges: Option<Vec<f64>>,
}

impl crate::api::json::ToJson for Centerline {
    fn write_json(&self, out: &mut String) {
        out.push_str("{\"method\":");
        let name = match self.method {
            Method::Matched => "matched",
            Method::Sampled => "sampled",
        };
        crate::api::json::ToJson::write_json(name, out);
        out.push_str(",\"pts\":");
        self.pts.write_json(out);
        if let Some(b) = &self.bulges {
            out.push_str(",\"bulges\":");
            b.write_json(out);
        }
        out.push('}');
    }
}

fn mid(p: Vec2, q: Vec2) -> Vec2 {
    Vec2::new((p.x + q.x) / 2.0, (p.y + q.y) / 2.0)
}

fn dist(p: Vec2, q: Vec2) -> f64 {
    js_hypot(p.x - q.x, p.y - q.y)
}

/// The edge's start and end.
fn ends_of(e: &Edge) -> (Vec2, Vec2) {
    match *e {
        Edge::Seg { a, b } => (a, b),
        Edge::Arc { c, r, a0, sweep } => {
            let at = |t: f64| Vec2::new(c.x + r * crate::jsmath::cos(t), c.y + r * crate::jsmath::sin(t));
            (at(a0), at(a0 + sweep))
        }
    }
}

/// The centreline edge by edge, or none when the sides do not match.
fn matched(a: &[Edge], b: &[Edge]) -> Option<(Vec<Vec2>, Vec<f64>)> {
    if a.len() != b.len() || a.is_empty() {
        return None;
    }
    let mut bulges = Vec::with_capacity(a.len());
    for (ea, eb) in a.iter().zip(b) {
        match (*ea, *eb) {
            (Edge::Seg { a: a0, b: a1 }, Edge::Seg { a: b0, b: b1 }) => {
                let (da, db) = (
                    Vec2::new(a1.x - a0.x, a1.y - a0.y),
                    Vec2::new(b1.x - b0.x, b1.y - b0.y),
                );
                let (la, lb) = (js_hypot(da.x, da.y), js_hypot(db.x, db.y));
                if !(la > 0.0 && lb > 0.0) {
                    return None;
                }
                let cross = (da.x * db.y - da.y * db.x) / (la * lb);
                let dot = (da.x * db.x + da.y * db.y) / (la * lb);
                if cross.abs() > 1e-9 || dot <= 0.0 {
                    return None;
                }
                bulges.push(0.0);
            }
            (
                Edge::Arc {
                    c: ca, sweep: sa, ..
                },
                Edge::Arc {
                    c: cb, sweep: sb, ..
                },
            ) => {
                if dist(ca, cb) > 1e-3 || (sa - sb).abs() > 1e-9 {
                    return None;
                }
                bulges.push(tan(sa / 4.0));
            }
            _ => return None,
        }
    }
    let mut pts = Vec::with_capacity(a.len() + 1);
    for (ea, eb) in a.iter().zip(b) {
        pts.push(mid(ends_of(ea).0, ends_of(eb).0));
    }
    let (Some(la), Some(lb)) = (a.last(), b.last()) else {
        return None;
    };
    pts.push(mid(ends_of(la).1, ends_of(lb).1));
    Some((pts, bulges))
}

/// The centreline of the sides `a` and `b` (docs/adr/0190 §2), sampled
/// every `step` along the longer side when they do not match.
pub fn centerline(a: &Shape, b: &Shape, step: f64) -> Result<Centerline, String> {
    let (Some(wa), Some(forward)) = (Walk::new(a, false), Walk::new(b, false)) else {
        return Err(NO_ROUTE.to_owned());
    };
    if wa.closed() || forward.closed() {
        return Err(CLOSED.to_owned());
    }
    if !(step > 0.0 && step.is_finite()) {
        return Err(BAD_STEP.to_owned());
    }
    let (la, lb0) = (wa.length(), forward.length());
    let (a0, a1) = (wa.frame(0.0).0, wa.frame(la).0);
    let (b0, b1) = (forward.frame(0.0).0, forward.frame(lb0).0);
    // The second side runs the first's way.
    let wb = if dist(a0, b1) + dist(a1, b0) < dist(a0, b0) + dist(a1, b1) {
        match Walk::new(b, true) {
            Some(w) => w,
            None => return Err(NO_ROUTE.to_owned()),
        }
    } else {
        forward
    };
    let lb = wb.length();
    let longest = js_max(la, lb);
    if longest / step > MOST_POINTS {
        return Err(TOO_MANY.to_owned());
    }
    if let (Some(ea), Some(eb)) = (wa.edges(), wb.edges())
        && let Some((pts, bulges)) = matched(ea, eb)
    {
        return Ok(Centerline {
            method: Method::Matched,
            pts,
            bulges: Some(bulges),
        });
    }
    // The shares: the ends, both sides' vertices and every step along the longer side.
    let mut shares = vec![0.0, 1.0];
    shares.extend(wa.vertex_lengths().iter().map(|s| s / la));
    shares.extend(wb.vertex_lengths().iter().map(|s| s / lb));
    let mut k = 1.0;
    while k * step < longest {
        shares.push(k * step / longest);
        k += 1.0;
    }
    shares.sort_by(f64::total_cmp);
    let mut kept: Vec<f64> = Vec::with_capacity(shares.len());
    for t in shares {
        if kept.last().is_some_and(|&last| t - last <= 1e-12) {
            continue;
        }
        kept.push(t);
    }
    let pts = kept
        .into_iter()
        .map(|t| mid(wa.frame(t * la).0, wb.frame(t * lb).0))
        .collect();
    Ok(Centerline {
        method: Method::Sampled,
        pts,
        bulges: None,
    })
}

/// The op's answer: the centreline, or why there is none.
#[derive(Clone, Debug, PartialEq)]
pub struct Answer {
    pub centerline: Option<Centerline>,
    pub problem: Option<String>,
}

crate::json_struct!(out Answer {
    centerline,
    problem
});

pub(crate) static OPS: &[Op] = &[op!("centerline", |a: Entity, b: Entity, step: f64| {
    match centerline(&a.shape, &b.shape, step) {
        Ok(c) => Answer {
            centerline: Some(c),
            problem: None,
        },
        Err(why) => Answer {
            centerline: None,
            problem: Some(why),
        },
    }
})];
