//! The points an interpolation or a density takes (docs/adr/0232 §2): the
//! vertices of points, lines, polylines and areas with their elevations or
//! their object's value, equal places joined; a density's points with their
//! weights; a line density's edges. One rule for both platforms: the web
//! hands its objects over as JSON (only their places and elevations), the
//! desktop from the contract (`Source::of`), and the values as the texts of
//! the chosen field, read by `kentos.statistics/1` (ADR 0200 §4).

use kentos_contracts::{AreaPart, Entity, PointPart, RingGeometry, Vec2 as Place};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::intersect::Edge;
use kentos_geometry_core::ops::edges::entity_edges;
use kentos_geometry_core::ops::statistics::read_number;
use kentos_geometry_core::vec2::Vec2;
use serde::Deserialize;

/// The most points a run takes (before equal places are joined).
pub const MOST_POINTS: usize = 10_000_000;

/// An object the points are taken from: its place and elevations as the
/// drawing holds them. Other kinds take no part.
#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase")]
pub enum Source {
    Point {
        p: Place,
        #[serde(default)]
        z: Option<f64>,
        #[serde(default)]
        parts: Option<Vec<PointPart>>,
    },
    Line {
        a: Place,
        b: Place,
        #[serde(default)]
        za: Option<f64>,
        #[serde(default)]
        zb: Option<f64>,
    },
    Polyline(Path),
    Polygon(Path),
    #[serde(other)]
    Other,
}

/// A polyline's or an area's vertices and elevations, with its holes and parts.
#[derive(Clone, Debug, Default, Deserialize)]
pub struct Path {
    pub pts: Vec<Place>,
    #[serde(default)]
    pub zs: Option<Vec<Option<f64>>>,
    #[serde(default)]
    pub holes: Option<Vec<RingGeometry>>,
    #[serde(default)]
    pub parts: Option<Vec<AreaPart>>,
}

impl Source {
    /// The object as the points take it.
    pub fn of(e: &Entity) -> Source {
        let path = |p: &kentos_contracts::PathEntity| Path {
            pts: p.pts.clone(),
            zs: p.zs.clone(),
            holes: p.holes.clone(),
            parts: p.parts.clone(),
        };
        match e {
            Entity::Point(p) => Source::Point {
                p: p.p,
                z: p.z,
                parts: p.parts.clone(),
            },
            Entity::Line(l) => Source::Line {
                a: l.a,
                b: l.b,
                za: l.za,
                zb: l.zb,
            },
            Entity::Polyline(p) => Source::Polyline(path(p)),
            Entity::Polygon(p) => Source::Polygon(path(p)),
            _ => Source::Other,
        }
    }

    /// Each vertex with its elevation, in the object's order: a point and
    /// its parts; a line's ends; a path's ring, its holes, then its parts and theirs.
    fn vertices(&self, mut f: impl FnMut(Place, Option<f64>)) {
        let ring = |pts: &[Place],
                    zs: &Option<Vec<Option<f64>>>,
                    f: &mut dyn FnMut(Place, Option<f64>)| {
            for (k, p) in pts.iter().enumerate() {
                f(*p, zs.as_ref().and_then(|z| z.get(k).copied().flatten()));
            }
        };
        match self {
            Source::Point { p, z, parts } => {
                f(*p, *z);
                for q in parts.iter().flatten() {
                    f(q.p, q.z);
                }
            }
            Source::Line { a, b, za, zb } => {
                f(*a, *za);
                f(*b, *zb);
            }
            Source::Polyline(path) | Source::Polygon(path) => {
                ring(&path.pts, &path.zs, &mut f);
                for h in path.holes.iter().flatten() {
                    ring(&h.pts, &h.zs, &mut f);
                }
                for part in path.parts.iter().flatten() {
                    ring(&part.pts, &part.zs, &mut f);
                    for h in part.holes.iter().flatten() {
                        ring(&h.pts, &h.zs, &mut f);
                    }
                }
            }
            Source::Other => {}
        }
    }

    fn takes_part(&self) -> bool {
        !matches!(self, Source::Other)
    }
}

/// A value's text read as a number (`kentos.statistics/1`), or none.
pub fn number_of(text: Option<&str>) -> Option<f64> {
    let v = read_number(text?)?.to_f64();
    v.is_finite().then_some(v)
}

/// An interpolation's points: distinct places, their values, and where each came from.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Points {
    pub xy: Vec<Vec2>,
    pub v: Vec<f64>,
    /// The object (its place in the input) of each point's first vertex.
    pub object: Vec<u32>,
    /// Vertices joined into a point at the same place (beyond the first).
    pub merged: usize,
    /// Objects whose value is not a number.
    pub unread: usize,
    /// Vertices without an elevation (none when a field gives the values).
    pub no_elevation: usize,
}

impl Points {
    pub fn len(&self) -> usize {
        self.xy.len()
    }

    pub fn is_empty(&self) -> bool {
        self.xy.is_empty()
    }

    /// The points' box: least x, least y, largest x, largest y.
    pub fn bounds(&self) -> Option<[f64; 4]> {
        bounds_of(self.xy.iter().copied())
    }
}

/// The box of some places.
pub fn bounds_of(places: impl Iterator<Item = Vec2>) -> Option<[f64; 4]> {
    let mut b: Option<[f64; 4]> = None;
    for p in places {
        b = Some(match b {
            None => [p.x, p.y, p.x, p.y],
            Some([a, c, d, e]) => [a.min(p.x), c.min(p.y), d.max(p.x), e.max(p.y)],
        });
    }
    b
}

/// The points of `sources` (docs/adr/0232 §2): with `values` (each
/// object's text; the list as long as the sources) every vertex takes its
/// object's number; without, its own elevation. Vertices at the same place
/// become one point, its value their mean in the input's order, its place
/// in the order the first of them came.
pub fn gather(sources: &[Source], values: Option<&[Option<String>]>) -> Result<Points, String> {
    let mut raw: Vec<(Vec2, f64, u32)> = Vec::new();
    let mut unread = 0;
    let mut no_elevation = 0;
    for (i, s) in sources.iter().enumerate() {
        if !s.takes_part() {
            continue;
        }
        let value = match values {
            Some(list) => match number_of(list.get(i).and_then(|v| v.as_deref())) {
                Some(v) => Some(v),
                None => {
                    unread += 1;
                    continue;
                }
            },
            None => None,
        };
        let mut too_many = false;
        s.vertices(|p, z| {
            if raw.len() >= MOST_POINTS {
                too_many = true;
                return;
            }
            let p = Vec2::new(p.x, p.y);
            if !p.x.is_finite() || !p.y.is_finite() {
                return;
            }
            match value.or(z.filter(|z| z.is_finite())) {
                Some(v) => raw.push((p, v, i as u32)),
                None => no_elevation += 1,
            }
        });
        if too_many {
            return Err(format!(
                "Çözümleme en çok {MOST_POINTS} nokta alır; nesnelerin köşeleri daha çok."
            ));
        }
    }
    // Equal places: sorted by place then by order, each run joined.
    let mut order: Vec<u32> = (0..raw.len() as u32).collect();
    order.sort_by(|&a, &b| {
        let (pa, pb) = (raw[a as usize].0, raw[b as usize].0);
        pa.x.total_cmp(&pb.x)
            .then(pa.y.total_cmp(&pb.y))
            .then(a.cmp(&b))
    });
    let mut first_of: Vec<(u32, f64, u32)> = Vec::new(); // (first raw index, mean, count)
    let mut k = 0;
    while k < order.len() {
        let first = order[k];
        let p = raw[first as usize].0;
        let mut sum = 0.0;
        let mut count = 0u32;
        let mut m = k;
        while m < order.len() && raw[order[m] as usize].0 == p {
            sum += raw[order[m] as usize].1;
            count += 1;
            m += 1;
        }
        first_of.push((
            first,
            if count == 1 {
                sum
            } else {
                sum / f64::from(count)
            },
            count,
        ));
        k = m;
    }
    first_of.sort_by_key(|f| f.0);
    let merged = raw.len() - first_of.len();
    Ok(Points {
        xy: first_of.iter().map(|f| raw[f.0 as usize].0).collect(),
        v: first_of.iter().map(|f| f.1).collect(),
        object: first_of.iter().map(|f| raw[f.0 as usize].2).collect(),
        merged,
        unread,
        no_elevation,
    })
}

/// A density's points: places and weights, equal places kept apart.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Weighted {
    pub xy: Vec<Vec2>,
    pub w: Vec<f64>,
    /// Objects whose weight is not a number or below zero.
    pub unread: usize,
}

/// Çekirdek yoğunluğu's points (docs/adr/0232 §2): each point object's
/// point and parts, weighted by its object's number (1 without `weights`).
pub fn gather_weighted(
    sources: &[Source],
    weights: Option<&[Option<String>]>,
) -> Result<Weighted, String> {
    let mut out = Weighted::default();
    for (i, s) in sources.iter().enumerate() {
        let Source::Point { .. } = s else {
            continue;
        };
        let w = match weights {
            Some(list) => match number_of(list.get(i).and_then(|v| v.as_deref())) {
                Some(w) if w >= 0.0 => w,
                _ => {
                    out.unread += 1;
                    continue;
                }
            },
            None => 1.0,
        };
        let mut too_many = false;
        s.vertices(|p, _| {
            if out.xy.len() >= MOST_POINTS {
                too_many = true;
            } else if p.x.is_finite() && p.y.is_finite() {
                out.xy.push(Vec2::new(p.x, p.y));
                out.w.push(w);
            }
        });
        if too_many {
            return Err(format!("Çözümleme en çok {MOST_POINTS} nokta alır."));
        }
    }
    Ok(out)
}

/// Çizgi yoğunluğu's lines: each object's edges (segments and arcs; an
/// ellipse's and a curve's chords within 0.1 mm) and its weight.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Lines {
    pub edges: Vec<Edge>,
    pub w: Vec<f64>,
    pub unread: usize,
}

/// The edges of `shapes`, weighted by their objects' numbers (1 without `weights`).
pub fn gather_lines(shapes: &[Shape], weights: Option<&[Option<String>]>) -> Result<Lines, String> {
    let mut out = Lines::default();
    for (i, s) in shapes.iter().enumerate() {
        if matches!(s, Shape::Point { .. } | Shape::Text { .. }) {
            continue;
        }
        let w = match weights {
            Some(list) => match number_of(list.get(i).and_then(|v| v.as_deref())) {
                Some(w) if w >= 0.0 => w,
                _ => {
                    out.unread += 1;
                    continue;
                }
            },
            None => 1.0,
        };
        for e in entity_edges(s) {
            if out.edges.len() >= MOST_POINTS {
                return Err(format!(
                    "Çözümleme en çok {MOST_POINTS} çizgi parçası alır."
                ));
            }
            out.edges.push(e);
            out.w.push(w);
        }
    }
    Ok(out)
}
