//! Strict, opt-in closed contours for cad.walls.convert. Existing Each/Pairs
//! retain their direction and partial-source policies. Shared geometry owns
//! analytic rings, orientation, validity and offsets.
use crate::{
    checks::{Stop, error},
    codes, geometry,
};
use kentos_commands::{Entity, WallFields};
use kentos_domain::Document;
use kentos_geometry_core::{
    entity::Shape,
    geom::{
        arrangement::{Area, Ring, TOL},
        bulge::bulge_path_edges,
        intersect::{closest_on_edge, intersect_edges, point_at},
        region::{orient_ring, ring_area, ring_edges},
        wall,
    },
    ops::{areas::areas_of_entity, geoprocess::validity},
};

pub(super) fn invalid(why: &str, path: &str) -> Stop {
    Stop::failed(error(codes::INVALID_WALL, why.into(), Some(path.into())))
}
fn polygon(a: &Area) -> Shape {
    Shape::Polygon {
        pts: a.outer.pts.clone(),
        bulges: a.outer.bulges.clone(),
        holes: (!a.holes.is_empty()).then(|| a.holes.clone()),
        parts: None,
    }
}
fn valid(a: &Area, path: &str) -> Result<(), Stop> {
    if std::iter::once(&a.outer)
        .chain(a.holes.iter())
        .any(|r| !ring_area(r).is_finite() || ring_area(r) == 0.0)
    {
        return Err(invalid(
            "Konturun sonlu ve sıfırdan farklı bir alanı olmalı.",
            path,
        ));
    }
    if let Some(p) = validity::problems(&polygon(a)).first() {
        return Err(invalid(
            &format!(
                "Kontur duvara çevrilemedi: {}. Kaynağı düzeltin; hiçbir duvar yazılmadı.",
                p.kind.text()
            ),
            path,
        ));
    }
    let rings: Vec<_> = std::iter::once(&a.outer)
        .chain(a.holes.iter())
        .map(ring_edges)
        .collect();
    // The general area checker permits a boundary touch. A room wall needs
    // distinct boundaries: even touching jamb/face rings leave no clear room.
    for (i, first) in rings.iter().enumerate() {
        for second in &rings[i + 1..] {
            if first
                .iter()
                .any(|a| second.iter().any(|b| !intersect_edges(a, b).is_empty()))
            {
                return Err(invalid(
                    "Konturun dış sınırı ve delikleri birbirine değmemeli veya kesişmemeli; hiçbir duvar yazılmadı.",
                    path,
                ));
            }
        }
    }
    Ok(())
}
fn oriented(r: &Ring, ccw: bool) -> Ring {
    let mut result = orient_ring(r, ccw);
    // Keep the user's first vertex even when reversing the ring and its arcs.
    if let Some(first) = r.pts.first()
        && let Some(i) = result.pts.iter().position(|p| p == first)
    {
        result.pts.rotate_left(i);
        if let Some(b) = &mut result.bulges {
            b.rotate_left(i);
        }
    }
    result
}
pub(super) fn areas(e: &Entity, path: &str) -> Result<Vec<Area>, Stop> {
    let supported = matches!(e, Entity::Polygon(_) | Entity::Circle(_))
        || matches!(e, Entity::Polyline(p) if p.closed && p.parts.is_none());
    if !supported {
        return Err(invalid(
            "Kapalı kontur için kapalı çoklu çizgi, alan veya daire verin; açık ya da başka türde kaynak varsa hiçbir duvar yazılmaz.",
            path,
        ));
    }
    if let Entity::Polygon(p) | Entity::Polyline(p) = e {
        let nonzero =
            |zs: &Option<Vec<Option<f64>>>| zs.iter().flatten().flatten().any(|z| *z != 0.0);
        let has_elevation = nonzero(&p.zs)
            || p.holes.iter().flatten().any(|h| nonzero(&h.zs))
            || p.parts.iter().flatten().any(|part| {
                nonzero(&part.zs) || part.holes.iter().flatten().any(|h| nonzero(&h.zs))
            });
        if has_elevation || p.ms.iter().flatten().any(Option::is_some) {
            return Err(invalid(
                "Kot veya M değeri taşıyan kontur bu 2B dönüşümde desteklenmiyor; kaynak korunarak işlem durduruldu.",
                path,
            ));
        }
    }
    let mut areas = areas_of_entity(&geometry::shape(e));
    if areas.iter().map(|a| 1 + a.holes.len()).sum::<usize>() > kentos_commands::MAX_WALLS {
        return Err(invalid("Konturlar en çok 10000 duvar üretebilir.", path));
    }
    if areas.is_empty() {
        return Err(invalid("Konturun kapalı, geçerli bir alanı yok.", path));
    }
    for a in &mut areas {
        for r in std::iter::once(&a.outer).chain(a.holes.iter()) {
            let pts = r
                .pts
                .iter()
                .map(|p| kentos_commands::Vec2 { x: p.x, y: p.y })
                .collect::<Vec<_>>();
            crate::walls::path_numbers(&pts, r.bulges.as_deref(), true, path)?;
        }
        valid(a, path)?;
        a.outer = oriented(&a.outer, true);
        for h in &mut a.holes {
            *h = oriented(h, false);
        }
    }
    Ok(areas)
}
/// Room-facing line (left of travel). Reject offset inversions instead of
/// accepting a small ring that has folded back through the original room.
pub(super) fn room_face(w: &WallFields, path: &str) -> Result<Ring, Stop> {
    let pts = crate::walls::core_points(&w.pts);
    let spec = kentos_formats::walls::spec(w);
    let offset = wall::left_face(&spec);
    let source = Ring {
        pts: pts.clone(),
        bulges: w.bulges.clone(),
    };
    let edges = bulge_path_edges(&pts, w.bulges.as_deref(), true);
    let mut room = None;
    for d in wall::boundaries(&spec) {
        let line =
            wall::line_at(&pts, w.bulges.as_deref(), true, d).map_err(|e| invalid(&e, path))?;
        let face = Ring {
            pts: line.pts,
            bulges: Some(line.bulges),
        };
        if !ring_area(&face).is_finite() || ring_area(&face) * ring_area(&source) <= 0.0 {
            return Err(invalid(
                "Duvar kalınlığı konturu kapatıyor; kalınlığı azaltın veya referansı değiştirin.",
                path,
            ));
        }
        for edge in ring_edges(&face) {
            let p = point_at(&edge, 0.5);
            let nearest = edges
                .iter()
                .map(|e| closest_on_edge(e, p).d)
                .fold(f64::INFINITY, f64::min);
            if nearest + TOL < d.abs() {
                return Err(invalid(
                    "Duvar kalınlığı konturu kapatıyor; kalınlığı azaltın veya referansı değiştirin.",
                    path,
                ));
            }
        }
        valid(
            &Area {
                outer: face.clone(),
                holes: Vec::new(),
            },
            path,
        )?;
        if d == offset {
            room = Some(face);
        }
    }
    room.ok_or_else(|| invalid("Konturun oda yüzü üretilemedi.", path))
}
pub(super) fn room_valid(faces: &[Ring], path: &str) -> Result<(), Stop> {
    let Some(outer) = faces.first() else {
        return Err(invalid("Kontur boş.", path));
    };
    valid(
        &Area {
            outer: outer.clone(),
            holes: faces[1..].to_vec(),
        },
        path,
    )
}
fn same_ring(a: &Ring, b: &Ring) -> bool {
    let (a, b) = (oriented(a, true), oriented(b, true));
    let n = a.pts.len();
    if n != b.pts.len() || n == 0 {
        return false;
    }
    let bulge = |r: &Ring, i: usize| {
        r.bulges
            .as_ref()
            .and_then(|b| b.get(i))
            .copied()
            .unwrap_or(0.0)
    };
    (0..n).any(|shift| {
        (0..n).all(|i| {
            a.pts[i] == b.pts[(i + shift) % n] && bulge(&a, i) == bulge(&b, (i + shift) % n)
        })
    })
}
fn ring(w: &WallFields) -> Ring {
    Ring {
        pts: crate::walls::core_points(&w.pts),
        bulges: w.bulges.clone(),
    }
}
pub(super) fn not_duplicate(
    doc: &Document,
    w: &WallFields,
    layer: &str,
    path: &str,
) -> Result<(), Stop> {
    let wanted = ring(w);
    let matches = |other: &WallFields| other.closed && same_ring(&wanted, &ring(other));
    if doc.entities().any(
        |e| matches!(e,Entity::Wall(other) if other.base.layer_id==layer && (matches(&other.wall) || kentos_geometry_core::geom::wall_network::paths_overlap(&wanted, &crate::walls::core_points(&other.wall.pts), other.wall.bulges.as_deref(), other.wall.closed))),
    )
    {
        return Err(invalid(
            "Aynı katmanda bu konturdan zaten duvar var; yinelenen duvar üretilmedi. Bütün işlem durduruldu.",
            path,
        ));
    }
    Ok(())
}

/// Validate the target before presenting a plan; a typed GIS layer cannot
/// silently receive architectural objects, even when it held the contour.
pub(super) fn target(doc: &Document, layer: &str, path: &str) -> Result<(), Stop> {
    if let Some(node) = doc.layers().get(layer)
        && let Some(holds) = node.geometry
        && !holds.admits("wall")
    {
        return Err(Stop::failed(error(
            kentos_commands::codes::LAYER_GEOMETRY_MISMATCH,
            format!(
                "“{}” bir {}; duvar bu katmana yazılmaz. Duvarlar için Karışık türde bir hedef katman seçin.",
                node.name,
                holds.layer_words()
            ),
            Some(path.into()),
        )));
    }
    Ok(())
}

#[path = "wall_contour_network.rs"]
mod network;
pub(super) use network::merge;
