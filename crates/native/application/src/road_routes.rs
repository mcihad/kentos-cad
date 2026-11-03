//! A road project's preliminary routes (plan/road.md Y12.1, docs/adr/0910 §1): the terrain surface built now, its
//! heights at a grid's nodes over the two places (rounded to the micrometre, none off the surface or in an area of the
//! layers to avoid), the least-cost path between the places' nearest nodes (`kentos-road`'s `route`) and the line it
//! makes, simplified within the tolerance. Nothing is kept.

use kentos_commands::{Entity, RouteAsk, RouteFound, RoutePoint};
use kentos_domain::Document;
use kentos_road::route::{Grid, NoRoute, least_cost, rounded, simplified};

use crate::checks::Stop;
use crate::codes;
use crate::road_terrains::{self as terrains, fail};

/// Only a road project has preliminary routes.
pub(crate) fn road_project(doc: &Document) -> Result<(), Stop> {
    if doc.settings().cad_work == Some(kentos_commands::CadWork::Road) {
        Ok(())
    } else {
        Err(fail(codes::NOT_A_ROAD_PROJECT, "Ön güzergâh yalnız yol projesinde (KentOS Yol) olur; bu çizim yol projesi değil.".into(), ""))
    }
}

fn bad(words: String, path: &str) -> Stop {
    fail(codes::INVALID_ROUTE, words, path)
}

/// Whether `p` is inside a ring (even–odd, the vertices joined straight).
fn inside(ring: &[kentos_commands::Vec2], p: [f64; 2]) -> bool {
    let mut odd = false;
    let n = ring.len();
    for k in 0..n {
        let (a, b) = (&ring[k], &ring[(k + n - 1) % n]);
        if (a.y > p[1]) != (b.y > p[1]) && p[0] < (b.x - a.x) * (p[1] - a.y) / (b.y - a.y) + a.x {
            odd = !odd;
        }
    }
    odd
}

/// The areas of the layers to avoid: each an outer ring and its holes.
fn areas(doc: &Document, layers: &[String]) -> Result<Vec<Vec<Vec<kentos_commands::Vec2>>>, Stop> {
    for (i, id) in layers.iter().enumerate() {
        if doc.layers().get(id).is_none() {
            return Err(bad(format!("“{id}” kimlikli katman çizimde yok."), &format!("avoid[{i}]")));
        }
    }
    let mut out = Vec::new();
    for e in doc.entities() {
        let Entity::Polygon(p) = e else { continue };
        if !layers.contains(&p.base.layer_id) {
            continue;
        }
        let ring = |pts: &Vec<kentos_commands::Vec2>, holes: &Option<Vec<kentos_commands::RingGeometry>>| {
            let mut area = vec![pts.clone()];
            area.extend(holes.iter().flatten().map(|h| h.pts.clone()));
            area
        };
        out.push(ring(&p.pts, &p.holes));
        for part in p.parts.iter().flatten() {
            out.push(ring(&part.pts, &part.holes));
        }
    }
    Ok(out)
}

/// The route asked, found now.
pub(crate) fn find(doc: &Document, ask: &RouteAsk) -> Result<RouteFound, Stop> {
    road_project(doc)?;
    let id = crate::road_profiles::surface_id(doc, &ask.surface, "surface")?;
    let finite = |v: f64| v.is_finite();
    for (v, path) in [(ask.from.x, "from"), (ask.from.y, "from"), (ask.to.x, "to"), (ask.to.y, "to")] {
        if !finite(v) {
            return Err(bad("Rotanın başı ve sonu sonlu olmalı.".into(), path));
        }
    }
    if !(finite(ask.cell) && ask.cell >= 0.1 && ask.cell <= 1000.0) {
        return Err(bad("Hücre 0,1 ile 1000 m arası olur.".into(), "cell"));
    }
    if !(finite(ask.margin) && (0.0..=100_000.0).contains(&ask.margin)) {
        return Err(bad("Pay 0 ile 100 000 m arası olur.".into(), "margin"));
    }
    if !(finite(ask.grade) && ask.grade > 0.0 && ask.grade <= 1000.0) {
        return Err(bad("En dik eğim 0'dan büyük, en çok %1000 olur.".into(), "grade"));
    }
    if !(finite(ask.climb) && (0.0..=1000.0).contains(&ask.climb)) {
        return Err(bad("Tırmanmanın ağırlığı 0 ile 1000 arası olur.".into(), "climb"));
    }
    if ask.tolerance.is_some_and(|t| !(finite(t) && (0.0..=1000.0).contains(&t))) {
        return Err(bad("Sadeleştirme toleransı 0 ile 1000 m arası olur.".into(), "tolerance"));
    }
    let (a, b) = ([ask.from.x, ask.from.y], [ask.to.x, ask.to.y]);
    let grid = Grid::over(a, b, ask.cell, ask.margin)
        .ok_or_else(|| bad("Izgara iki milyondan çok düğüm olur; hücreyi büyütün ya da payı küçültün.".into(), "cell"))?;
    let avoid = areas(doc, &ask.avoid)?;
    let Some(def) = terrains::surfaces(doc).iter().find(|s| s.id == id) else {
        return Err(Stop::Cancelled);
    };
    let surface = terrains::make(doc, def)
        .surface
        .map_err(|_| bad(format!("“{}” yüzeyi kurulamıyor.", def.name), "surface"))?;
    let places: Vec<[f64; 2]> = (0..grid.cols * grid.rows).map(|n| grid.place(n)).collect();
    let heights: Vec<Option<f64>> = surface
        .heights(&places)
        .into_iter()
        .zip(&places)
        .map(|(h, p)| {
            let free = !avoid.iter().any(|area| inside(&area[0], *p) && !area[1..].iter().any(|hole| inside(hole, *p)));
            h.z.filter(|_| free).map(rounded)
        })
        .collect();
    let (na, nb) = (grid.nearest(a), grid.nearest(b));
    let found = least_cost(&grid, &heights, na, nb, ask.grade, ask.climb).map_err(|why| match why {
        NoRoute::Blocked { end } => fail(
            codes::ROUTE_BLOCKED,
            format!(
                "Rotanın {} yüzeyin dışında ya da kaçınılacak bir alanda.",
                if end { "sonu" } else { "başı" }
            ),
            if end { "to" } else { "from" },
        ),
        NoRoute::Unreached => fail(
            codes::ROUTE_UNREACHED,
            format!("İki yer arasında %{} eğimi aşmayan yol yok.", ask.grade),
            "grade",
        ),
    })?;
    let mut line: Vec<[f64; 3]> = found
        .nodes
        .iter()
        .map(|&n| {
            let p = grid.place(n);
            [p[0], p[1], heights[n].unwrap_or(0.0)]
        })
        .collect();
    let (za, zb) = (heights[na].unwrap_or(0.0), heights[nb].unwrap_or(0.0));
    if line.len() == 1 {
        line = vec![[a[0], a[1], za], [b[0], b[1], zb]];
    } else {
        let last = line.len() - 1;
        line[0] = [a[0], a[1], za];
        line[last] = [b[0], b[1], zb];
    }
    let kept = simplified(&line, ask.tolerance.unwrap_or(0.0));
    let (mut length, mut climb, mut steepest) = (0.0, 0.0, 0.0_f64);
    for w in kept.windows(2) {
        let (dx, dy) = (w[1][0] - w[0][0], w[1][1] - w[0][1]);
        let l = (dx * dx + dy * dy).sqrt();
        let rise = (w[1][2] - w[0][2]).abs();
        length += l;
        climb += rise;
        if l > 0.0 {
            steepest = steepest.max(rise / l * 100.0);
        }
    }
    Ok(RouteFound {
        points: kept.iter().map(|p| RoutePoint { x: p[0], y: p[1], z: p[2] }).collect(),
        length,
        climb,
        steepest,
        cost: found.cost,
        nodes: (grid.cols * grid.rows) as u32,
        settled: found.settled as u32,
    })
}
