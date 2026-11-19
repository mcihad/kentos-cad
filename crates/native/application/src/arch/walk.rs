//! Same-storey design routes. The walking envelope is a disc swept along the centreline.

use kentos_commands::{ArchWalk, ArchWalked, CommandResult, Entity, OpeningForm, Vec2};
use kentos_domain::{Document, Slot, Uuid};
use kentos_geometry_core::Vec2 as Point;
use kentos_geometry_core::geom::arrangement::{Area, Ring};
use kentos_geometry_core::geom::intersect::{Edge, intersect_edges, point_at};
use kentos_geometry_core::geom::region::{inside_area, ring_edges};
use kentos_geometry_core::ops::geoprocess::{buffer, subtract_all, union_all, union_buffers};

use crate::ExecutionContext;
use crate::checks::{Stop, error};

const EDGE_LIMIT: usize = 256;
const NODE_LIMIT: usize = 384;
// The route uses an extra millimetre while tracing the arc buffers.
const MARGIN: f64 = 0.001;

fn point(p: Vec2) -> Point {
    Point::new(p.x, p.y)
}

fn wire(p: Point) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}

fn distance(a: Point, b: Point) -> f64 {
    (a.x - b.x).hypot(a.y - b.y)
}

fn edges(areas: &[Area]) -> Vec<Edge> {
    areas
        .iter()
        .flat_map(|a| std::iter::once(&a.outer).chain(a.holes.iter()))
        .flat_map(ring_edges)
        .collect()
}

fn polygon(pts: Vec<Point>) -> Area {
    Area {
        outer: Ring { pts, bulges: None },
        holes: Vec::new(),
    }
}

fn checked(doc: &Document, input: &ArchWalk) -> Result<(), Stop> {
    for (p, key) in [(input.start, "start"), (input.end, "end")] {
        if !p.x.is_finite() || !p.y.is_finite() {
            return Err(Stop::failed(error(
                "not_finite",
                "Çizimde sonlu bir konum seçin.".into(),
                Some(key.into()),
            )));
        }
    }
    if !input.width.is_finite() || input.width < 0.05 || input.width > 10.0 {
        return Err(Stop::failed(error(
            "invalid_input",
            "Geçiş genişliği 0,05–10 m arasında olmalı.".into(),
            Some("width".into()),
        )));
    }
    if let Some(level) = &input.level
        && !doc
            .settings()
            .arch
            .as_ref()
            .is_some_and(|a| a.levels.iter().any(|l| l.id == *level))
    {
        return Err(Stop::failed(error(
            "level_not_found",
            "Çizimde bulunan bir kat seçin.".into(),
            Some("level".into()),
        )));
    }
    Ok(())
}

pub fn validate(cx: &ExecutionContext<'_>, input: &ArchWalk) -> CommandResult<()> {
    match checked(cx.doc, input) {
        Ok(()) => CommandResult::Completed {
            output: (),
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

pub fn execute(cx: &mut ExecutionContext<'_>, input: ArchWalk) -> CommandResult<ArchWalked> {
    plan(cx, &input)
}

pub fn plan(cx: &ExecutionContext<'_>, input: &ArchWalk) -> CommandResult<ArchWalked> {
    plan_in(cx.doc, input)
}

pub fn plan_in(doc: &Document, input: &ArchWalk) -> CommandResult<ArchWalked> {
    match draw(doc, input) {
        Ok(output) => CommandResult::Completed {
            output,
            warnings: Vec::new(),
        },
        Err(stop) => stop.into(),
    }
}

fn budget() -> Stop {
    Stop::failed(error(
        "invalid_input",
        "Bu katta rota hesabı için çok fazla sınır var. Daha küçük bir çizim kapsamı kullanın."
            .into(),
        None,
    ))
}

fn portals(doc: &Document, room: &[Area], level: Option<&str>) -> Vec<Area> {
    kentos_domain::passages::passages(doc)
        .into_iter()
        .filter_map(|p| {
            if p.level.as_deref() != level {
                return None;
            }
            let slot = doc.slot_of(Uuid::parse_str(&p.uid).ok()?)?;
            let Entity::Opening(o) = doc.get(slot)? else {
                return None;
            };
            if o.opening.broken.is_some() {
                return None;
            }
            let width = match o.opening.form {
                OpeningForm::Door => o.opening.sizes().clear?,
                OpeningForm::Opening => p.width,
                _ => return None,
            };
            let len = distance(p.a, p.b);
            if len < 1e-8 || width <= 0.0 {
                return None;
            }
            let (ux, uy) = ((p.b.x - p.a.x) / len, (p.b.y - p.a.y) / len);
            let at = |along: f64, across: f64| {
                Point::new(
                    (p.a.x + p.b.x) * 0.5 + ux * along - uy * across,
                    (p.a.y + p.b.y) * 0.5 + uy * along + ux * across,
                )
            };
            let half = p.half + 0.015;
            if ![-half, half]
                .into_iter()
                .all(|side| room.iter().any(|a| inside_area(a, at(0.0, side))))
            {
                return None;
            }
            let w = width / 2.0;
            Some(polygon(vec![
                at(-w, -half),
                at(w, -half),
                at(w, half),
                at(-w, half),
            ]))
        })
        .collect()
}

fn walk_objects(doc: &Document, level: Option<&str>, shell: bool) -> Vec<(Slot, Area)> {
    crate::door_audit::obstacles(doc)
        .into_iter()
        .filter(|o| {
            if o.level.as_deref() != level {
                return false;
            }
            match doc.get(o.slot) {
                Some(Entity::Column(_)) => true,
                Some(Entity::Insert(i)) if !shell => doc
                    .block(i.block)
                    .and_then(|b| b.card.as_ref())
                    .and_then(|c| c.category.as_deref())
                    .is_some_and(|c| {
                        matches!(
                            c.split('/').next().map(str::trim),
                            Some("Mobilya" | "Mutfak" | "Islak hacim")
                        )
                    }),
                _ => false,
            }
        })
        .map(|o| {
            (
                o.slot,
                Area {
                    outer: Ring {
                        pts: o.outer,
                        bulges: None,
                    },
                    holes: o
                        .holes
                        .into_iter()
                        .map(|pts| Ring { pts, bulges: None })
                        .collect(),
                },
            )
        })
        .collect()
}

fn draw(doc: &Document, input: &ArchWalk) -> Result<ArchWalked, Stop> {
    checked(doc, input)?;
    let (start, end) = (point(input.start), point(input.end));
    let mut search = kentos_domain::space_follow::SpaceSearch::new(doc);
    let mut areas = Vec::new();
    for e in doc.entities() {
        let Entity::Space(s) = e else { continue };
        if e.own_level() != input.level.as_deref() || s.space.void || s.space.broken.is_some() {
            continue;
        }
        let parts = kentos_domain::escape::net_parts(&mut search, &s.space);
        areas.extend(kentos_domain::arch_values::core_areas(&parts));
    }
    let mut result = ArchWalked {
        points: Vec::new(),
        length: 0.0,
        width: input.width,
        blocked: None,
        obstacle: None,
    };
    for at in [start, end] {
        if !areas.iter().any(|a| inside_area(a, at)) {
            result.blocked = Some(wire(at));
            return Ok(result);
        }
    }
    if edges(&areas).len() > EDGE_LIMIT {
        return Err(budget());
    }
    areas.extend(portals(doc, &areas, input.level.as_deref()));
    let obstacles = walk_objects(doc, input.level.as_deref(), input.shell_only);
    let cutters = obstacles.iter().map(|(_, a)| a.clone()).collect::<Vec<_>>();
    if edges(&areas).len() + edges(&cutters).len() > EDGE_LIMIT {
        return Err(budget());
    }
    let plan = subtract_all(&union_all(&areas), &union_all(&cutters));
    let boundary = edges(&plan);
    let radius = input.width / 2.0;
    let buffered = radius + MARGIN;
    let bands = buffer::union_pieces(&boundary, buffered);
    let clear = subtract_all(&plan, &union_buffers(&bands, &boundary, buffered));
    if let Some(area) = clear
        .iter()
        .find(|a| inside_area(a, start) && inside_area(a, end))
    {
        let path = shortest(area, start, end)?;
        result.length = path.windows(2).map(|w| distance(w[0], w[1])).sum();
        result.points = path.into_iter().map(wire).collect();
        if !result.points.is_empty() {
            return Ok(result);
        }
    }
    let at = clear
        .iter()
        .find(|a| inside_area(a, start))
        .and_then(|a| first_outside(a, &edges(std::slice::from_ref(a)), start, end))
        .unwrap_or(start);
    result.blocked = Some(wire(at));
    result.obstacle = obstacles
        .iter()
        .find(|(_, area)| {
            inside_area(area, at)
                || edges(std::slice::from_ref(area)).iter().any(|e| {
                    let Edge::Seg { a, b } = *e else { return false };
                    kentos_geometry_core::labels::geom::segment_distance2(at, a, b)
                        <= (radius + 2.0 * MARGIN).powi(2)
                })
        })
        .and_then(|(slot, _)| doc.uid(*slot))
        .map(|u| u.to_string());
    Ok(result)
}

fn first_outside(area: &Area, boundary: &[Edge], a: Point, b: Point) -> Option<Point> {
    let line = Edge::Seg { a, b };
    let mut hits = vec![0.0, 1.0];
    hits.extend(
        boundary
            .iter()
            .flat_map(|e| intersect_edges(&line, e))
            .map(|h| h.t.clamp(0.0, 1.0)),
    );
    hits.sort_by(f64::total_cmp);
    hits.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
    hits.windows(2)
        .find(|w| {
            let p = point_at(&line, (w[0] + w[1]) / 2.0);
            !inside_area(area, p)
                && !boundary.iter().any(|e| {
                    let Edge::Seg { a, b } = *e else { return false };
                    kentos_geometry_core::labels::geom::segment_distance2(p, a, b) < 1e-14
                })
        })
        .map(|w| point_at(&line, w[0]))
}

fn shortest(area: &Area, start: Point, finish: Point) -> Result<Vec<Point>, Stop> {
    let boundary = edges(std::slice::from_ref(area));
    if first_outside(area, &boundary, start, finish).is_none() {
        return Ok(vec![start, finish]);
    }
    let mut points = vec![start, finish];
    for ring in std::iter::once(&area.outer).chain(area.holes.iter()) {
        for edge in ring_edges(ring) {
            points.push(point_at(&edge, 0.0));
            if let Edge::Arc { r, sweep, .. } = edge {
                // Nudge curved candidates into the clear area so their chords also lie inside it.
                let count = ((sweep.abs() * (r / MARGIN).sqrt()).ceil() as usize).clamp(2, 48);
                for k in 0..count {
                    let t = (k as f64 + 0.5) / count as f64;
                    let Edge::Arc { c, .. } = edge else {
                        unreachable!()
                    };
                    let p = point_at(&edge, t);
                    for scale in [1.0 - MARGIN / r, 1.0 + MARGIN / r] {
                        let candidate =
                            Point::new(c.x + (p.x - c.x) * scale, c.y + (p.y - c.y) * scale);
                        if inside_area(area, candidate) {
                            points.push(candidate);
                            break;
                        }
                    }
                }
            }
            if points.len() > NODE_LIMIT {
                return Err(budget());
            }
        }
    }
    let mut lengths = vec![f64::INFINITY; points.len()];
    let mut previous = vec![None; points.len()];
    let mut visited = vec![false; points.len()];
    lengths[0] = 0.0;
    while let Some((at, &length)) = lengths
        .iter()
        .enumerate()
        .filter(|(i, d)| !visited[*i] && d.is_finite())
        .min_by(|(_, a), (_, b)| a.total_cmp(b))
    {
        if at == 1 {
            let mut route = vec![finish];
            let mut cursor = at;
            while let Some(prev) = previous[cursor] {
                route.push(points[prev]);
                cursor = prev;
            }
            route.reverse();
            return Ok(route);
        }
        visited[at] = true;
        for next in 0..points.len() {
            let next_length = length + distance(points[at], points[next]);
            if !visited[next]
                && next_length < lengths[next]
                && first_outside(area, &boundary, points[at], points[next]).is_none()
            {
                lengths[next] = next_length;
                previous[next] = Some(at);
            }
        }
    }
    Ok(Vec::new())
}
