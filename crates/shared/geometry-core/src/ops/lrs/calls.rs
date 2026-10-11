//! The Doğrusal referans tools' runs (docs/adr/0224 §4): the desktop's tools
//! call the functions, the web (`model/ops/lrs.ts`) the operations with
//! whole lists, one call a run.

use super::{
    Along, CalibrateRules, Calibrated, CreateRules, Event, Fault, Mark, Piece, Route, RouteIndex,
    RoutePart, Walk, calibrate, create, locate_area, overlay, summary,
};
use crate::api::Op;
use crate::entity::Shape;
use crate::geom::bulge::bulge_arc;
use crate::geometry::Bounds;
use crate::jsmath::{PI, cos, js_max, js_min, sin};
use crate::op;
use crate::ops::areas::areas_of_entity;
use crate::vec2::Vec2;

/// A route calibrated (its parts) or why not, and which marks it used.
#[derive(Clone, Debug, PartialEq)]
pub struct CalibrateOut {
    pub parts: Option<Vec<RoutePart>>,
    pub used: Vec<bool>,
    pub fault: Option<Fault>,
}

crate::json_struct!(CalibrateOut { parts, used, fault });

/// Each route calibrated by its own marks (`marks[i]` route `i`'s).
pub fn calibrate_run(
    routes: &[Route],
    marks: &[Vec<Mark>],
    rules: &CalibrateRules,
) -> Vec<CalibrateOut> {
    routes
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let own = marks.get(i).map(Vec::as_slice).unwrap_or(&[]);
            match calibrate(r, own, rules) {
                Ok(Calibrated { parts, used }) => CalibrateOut {
                    parts: Some(parts),
                    used,
                    fault: None,
                },
                Err((fault, used)) => CalibrateOut {
                    parts: None,
                    used,
                    fault: Some(fault),
                },
            }
        })
        .collect()
}

/// An event to place: its route by its place in the list (none: the table
/// names no route there is), a point's measure or a line's from and to, and
/// its offset (+ right of the measure direction).
#[derive(Clone, Debug, PartialEq)]
pub struct EventIn {
    pub route: Option<usize>,
    pub m: Option<f64>,
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub offset: Option<f64>,
}

crate::json_struct!(EventIn {
    route,
    m,
    from,
    to,
    offset
});

/// An event placed: a point, a line's parts, or why not.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct EventOut {
    pub p: Option<Vec2>,
    pub parts: Option<Vec<RoutePart>>,
    pub fault: Option<Fault>,
}

crate::json_struct!(EventOut { p, parts, fault });

/// Olayları rotada göster: each event on its route, every route made ready
/// once, when an event first names it.
pub fn events_run(routes: &[Route], events: &[EventIn]) -> Vec<EventOut> {
    let mut walks: Vec<Option<Walk>> = (0..routes.len()).map(|_| None).collect();
    events
        .iter()
        .map(|e| {
            let Some(r) = e.route.filter(|&r| r < routes.len()) else {
                return EventOut {
                    fault: Some(Fault::NoRoute),
                    ..EventOut::default()
                };
            };
            let walk = walks[r].get_or_insert_with(|| Walk::new(&routes[r]));
            let offset = e.offset.unwrap_or(0.0);
            let placed = match (e.m, e.from, e.to) {
                (Some(m), _, _) => walk.point_at(m, offset).map(|p| EventOut {
                    p: Some(p),
                    ..EventOut::default()
                }),
                (None, Some(from), Some(to)) => {
                    walk.stretch(from, to, offset).map(|parts| EventOut {
                        parts: Some(parts),
                        ..EventOut::default()
                    })
                }
                _ => Err(Fault::NoMeasure),
            };
            placed.unwrap_or_else(|fault| EventOut {
                fault: Some(fault),
                ..EventOut::default()
            })
        })
        .collect()
}

/// An object located along a route: a point's measure and distance, a
/// line's or an area's piece's from and to (a line's farthest vertex's
/// distance), the route by its place in the list.
#[derive(Clone, Debug, PartialEq)]
pub struct Located {
    pub route: usize,
    pub m: Option<f64>,
    pub from: Option<f64>,
    pub to: Option<f64>,
    pub distance: Option<f64>,
}

crate::json_struct!(Located {
    route,
    m,
    from,
    to,
    distance
});

/// The vertices a line object is located by: a line's ends, a polyline's
/// vertices part after part, an arc's ends and middle.
fn line_points(s: &Shape) -> Option<Vec<Vec2>> {
    Some(match s {
        Shape::Line { a, b } => vec![*a, *b],
        Shape::Polyline { pts, parts, .. } => pts
            .iter()
            .copied()
            .chain(parts.iter().flatten().flat_map(|p| p.pts.iter().copied()))
            .collect(),
        Shape::Arc { c, r, a0, a1 } => {
            let sweep = if a1 >= a0 {
                a1 - a0
            } else {
                a1 - a0 + 2.0 * PI
            };
            [0.0, 0.5, 1.0]
                .iter()
                .map(|f| {
                    let t = a0 + sweep * f;
                    Vec2::new(c.x + r * cos(t), c.y + r * sin(t))
                })
                .collect()
        }
        _ => return None,
    })
}

fn bounds_of(pts: &[Vec2]) -> Bounds {
    pts.iter().fold(
        Bounds {
            min_x: f64::INFINITY,
            min_y: f64::INFINITY,
            max_x: f64::NEG_INFINITY,
            max_y: f64::NEG_INFINITY,
        },
        |b, p| Bounds {
            min_x: js_min(b.min_x, p.x),
            min_y: js_min(b.min_y, p.y),
            max_x: js_max(b.max_x, p.x),
            max_y: js_max(b.max_y, p.y),
        },
    )
}

/// Rota boyunca konumla: each object's places along the routes within
/// `radius` (a point by its closest place, a line when every vertex is
/// within the radius, an area by the routes' pieces inside it); only the
/// nearest route unless `all` (an area: every route through it).
pub fn locate_run(
    routes: &[Route],
    features: &[Shape],
    radius: f64,
    all: bool,
) -> Vec<Vec<Located>> {
    let index = RouteIndex::new(routes);
    let mut hits = Vec::new();
    features
        .iter()
        .map(|s| {
            if let Shape::Point { p, .. } = s {
                return index
                    .near(*p, radius, all, &mut hits)
                    .into_iter()
                    .map(|n| Located {
                        route: n.route,
                        m: n.place.m,
                        from: None,
                        to: None,
                        distance: Some(n.place.distance),
                    })
                    .collect();
            }
            if let Some(pts) = line_points(s) {
                let mut rows: Vec<(usize, Along)> = index
                    .routes_near(&bounds_of(&pts), radius, &mut hits)
                    .into_iter()
                    .filter_map(|r| Some((r, index.walk(r).locate_line(&pts, radius)?)))
                    .collect();
                if !all {
                    rows.sort_by(|a, b| a.1.distance.total_cmp(&b.1.distance).then(a.0.cmp(&b.0)));
                    rows.truncate(1);
                }
                return rows
                    .into_iter()
                    .map(|(r, a)| Located {
                        route: r,
                        m: None,
                        from: Some(a.from),
                        to: Some(a.to),
                        distance: Some(a.distance),
                    })
                    .collect();
            }
            let areas = areas_of_entity(s);
            if areas.is_empty() {
                return Vec::new();
            }
            let corners: Vec<Vec2> = areas
                .iter()
                .flat_map(|a| arc_corners(&a.outer.pts, a.outer.bulges.as_deref()))
                .collect();
            index
                .routes_near(&bounds_of(&corners), 0.0, &mut hits)
                .into_iter()
                .flat_map(|r| {
                    locate_area(index.walk(r), &areas)
                        .into_iter()
                        .map(move |q| Located {
                            route: r,
                            m: None,
                            from: q.from,
                            to: q.to,
                            distance: None,
                        })
                })
                .collect()
        })
        .collect()
}

/// A ring's vertices and its arcs' extreme points: the box of these holds the ring.
fn arc_corners(pts: &[Vec2], bulges: Option<&[f64]>) -> Vec<Vec2> {
    let mut out = pts.to_vec();
    for i in 0..pts.len() {
        let b = bulges.and_then(|v| v.get(i)).copied().unwrap_or(0.0);
        if b == 0.0 {
            continue;
        }
        if let Some(arc) = bulge_arc(pts[i], pts[(i + 1) % pts.len()], b) {
            // The whole circle's box: a little more than the arc's, never less.
            out.push(Vec2::new(arc.c.x - arc.r, arc.c.y - arc.r));
            out.push(Vec2::new(arc.c.x + arc.r, arc.c.y + arc.r));
        }
    }
    out
}

pub(crate) static OPS: &[Op] = &[
    op!(
        "lrsCreate",
        |pieces: Vec<Piece>, rules: CreateRules| create(&pieces, &rules)
    ),
    op!(
        "lrsCalibrate",
        |routes: Vec<Route>, marks: Vec<Vec<Mark>>, rules: CalibrateRules| {
            calibrate_run(&routes, &marks, &rules)
        }
    ),
    op!("lrsEvents", |routes: Vec<Route>, events: Vec<EventIn>| {
        events_run(&routes, &events)
    }),
    op!("lrsLocate", |routes: Vec<Route>,
                      features: Vec<Shape>,
                      radius: f64,
                      all: bool| {
        locate_run(&routes, &features, radius, all)
    }),
    op!("lrsOverlay", |a: Vec<Event>, b: Vec<Event>, union: bool| {
        overlay(&a, &b, union)
    }),
    op!("lrsSummaries", |routes: Vec<Route>| routes
        .iter()
        .map(summary)
        .collect::<Vec<_>>()),
];
