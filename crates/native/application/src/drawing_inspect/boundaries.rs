//! Exact line/circular-arc diagnosis. Markers are results, never document geometry.
use super::*;
use kentos_geometry_core::{
    Vec2 as P,
    geom::intersect::{Edge, closest_on_edge, intersect_edges, point_at},
    geometry::dist,
    ops::edges::entity_edges,
};

pub(super) struct SourceEdge {
    uid: Uuid,
    edge: Edge,
}
pub(super) fn collect(
    e: &Entity,
    uid: Uuid,
    edges: &mut Vec<SourceEdge>,
    out: &mut DrawingInspected,
) {
    match e {
        Entity::Line(_)
        | Entity::Arc(_)
        | Entity::Circle(_)
        | Entity::Polyline(_)
        | Entity::Polygon(_) => {
            // Count borrowed vertices before cloning geometry or expanding its edges.
            let vertices = match e {
                Entity::Polyline(p) | Entity::Polygon(p) => {
                    let ring_count = |n: usize, holes: &Option<Vec<kentos_commands::RingGeometry>>| {
                        holes.iter().flatten().fold(n, |n, h| n.saturating_add(h.pts.len()))
                    };
                    p.parts.iter().flatten().fold(ring_count(p.pts.len(), &p.holes), |n, part| {
                        n.saturating_add(ring_count(part.pts.len(), &part.holes))
                    })
                }
                _ => 2,
            };
            if vertices > EDGE_MAX {
                out.truncated = true;
                out.unsupported.push(uid.to_string());
                return;
            }
            let more = entity_edges(&shape(e));
            if edges.len().saturating_add(more.len()) > EDGE_MAX {
                out.truncated = true;
                out.unsupported.push(uid.to_string());
                return;
            }
            edges.extend(more.into_iter().map(|edge| SourceEdge { uid, edge }));
        }
        Entity::Ellipse(_) | Entity::Spline(_) | Entity::Nurbs(_) | Entity::Insert(_) => {
            out.unsupported.push(uid.to_string())
        }
        _ => {}
    }
}
fn wire(p: P) -> Vec2 {
    Vec2 { x: p.x, y: p.y }
}
fn same(a: &Edge, b: &Edge) -> bool {
    if let (
        Edge::Arc {
            c: a,
            r: ar,
            sweep: as_,
            ..
        },
        Edge::Arc {
            c: b,
            r: br,
            sweep: bs,
            ..
        },
    ) = (a, b)
        && dist(*a, *b) <= EPS
        && (ar - br).abs() <= EPS
        && (as_.abs() - std::f64::consts::TAU).abs() < 1e-10
        && (bs.abs() - std::f64::consts::TAU).abs() < 1e-10
    {
        return true;
    }
    let (a0, a1, b0, b1) = (
        point_at(a, 0.),
        point_at(a, 1.),
        point_at(b, 0.),
        point_at(b, 1.),
    );
    ((dist(a0, b0) <= EPS && dist(a1, b1) <= EPS) || (dist(a0, b1) <= EPS && dist(a1, b0) <= EPS))
        && dist(point_at(a, 0.5), point_at(b, 0.5)) <= EPS
}
fn tangent(edge: &Edge, p: P) -> P {
    match edge {
        Edge::Seg { a, b } => P::new(b.x - a.x, b.y - a.y),
        Edge::Arc { c, .. } => P::new(-(p.y - c.y), p.x - c.x),
    }
}
// One diagnostic record, with its source pair and delivery budget kept explicit.
#[allow(clippy::too_many_arguments)]
fn report(
    a: &SourceEdge,
    b: Option<&SourceEdge>,
    code: &str,
    points: Vec<P>,
    message: &str,
    repair: &str,
    distance: Option<f64>,
    out: &mut DrawingInspected,
    limit: usize,
) {
    let mut uids = vec![a.uid.to_string()];
    if let Some(b) = b
        && a.uid != b.uid
    {
        uids.push(b.uid.to_string());
    }
    push(
        out,
        InspectionFinding {
            group: InspectionGroup::Boundaries,
            severity: if code == "boundary_crossing" {
                InspectionSeverity::Info
            } else {
                InspectionSeverity::Warning
            },
            code: code.into(),
            uids,
            points: points.into_iter().map(wire).collect(),
            message: message.into(),
            repair: repair.into(),
            distance,
        },
        limit,
    );
}
pub(super) fn inspect(
    edges: &[SourceEdge],
    gap: f64,
    limit: usize,
    budget: &mut Budget,
    stopped: &mut impl FnMut() -> bool,
    out: &mut DrawingInspected,
) -> Result<(), Stop> {
    let mut connected = vec![[false; 2]; edges.len()];
    for (k, a) in edges.iter().enumerate() {
        if let Edge::Arc { sweep, .. } = a.edge
            && (sweep.abs() - std::f64::consts::TAU).abs() < 1e-10
        {
            connected[k] = [true; 2];
        }
    }
    for (i, a) in edges.iter().enumerate() {
        for (j, b) in edges.iter().enumerate().skip(i + 1) {
            if !budget.tick(stopped)? || out.truncated {
                return Ok(());
            }
            for (k, t) in [(0, 0.), (1, 1.)] {
                if closest_on_edge(&b.edge, point_at(&a.edge, t)).d <= EPS {
                    connected[i][k] = true;
                }
                if closest_on_edge(&a.edge, point_at(&b.edge, t)).d <= EPS {
                    connected[j][k] = true;
                }
            }
            if same(&a.edge, &b.edge) {
                report(
                    a,
                    Some(b),
                    "boundary_duplicate",
                    vec![point_at(&a.edge, 0.), point_at(&a.edge, 1.)],
                    "Aynı yerde yinelenen kenar var; sınır seçimi belirsizleşebilir.",
                    "Kopyaları inceleyin; yineleneni ayrı silme/temizleme planıyla kaldırın.",
                    None,
                    out,
                    limit,
                );
                continue;
            }
            for hit in intersect_edges(&a.edge, &b.edge) {
                let endpoint = |e: &Edge| {
                    dist(hit.p, point_at(e, 0.)) <= EPS || dist(hit.p, point_at(e, 1.)) <= EPS
                };
                if endpoint(&a.edge) && endpoint(&b.edge) {
                    continue;
                }
                let x = tangent(&a.edge, hit.p);
                let y = tangent(&b.edge, hit.p);
                let length = x.x.hypot(x.y) * y.x.hypot(y.y);
                let touch = length > 0. && (x.x * y.y - x.y * y.x).abs() / length < 1e-6;
                let (code, message) = if touch {
                    (
                        "boundary_tangent",
                        "Teğet temas var; iki bölgenin ayrımı incelenmeli.",
                    )
                } else if a.uid == b.uid {
                    (
                        "boundary_self_intersection",
                        "Sınır kendi kendini kesiyor; oluşan bölgeleri inceleyin.",
                    )
                } else {
                    (
                        "boundary_crossing",
                        "Kenarlar kesişiyor; bu bir yüz birleşimi de olabilir.",
                    )
                };
                report(
                    a,
                    Some(b),
                    code,
                    vec![hit.p],
                    message,
                    "Vurgulanan konumu inceleyin. Budama/bölme veya uç düzenlemesini ayrı önizlemeyle uygulayın.",
                    None,
                    out,
                    limit,
                );
            }
        }
    }
    let mut loose = Vec::new();
    for (i, e) in edges.iter().enumerate() {
        for (k, t) in [(0, 0.), (1, 1.)] {
            if !connected[i][k] {
                loose.push((i, point_at(&e.edge, t)));
            }
        }
    }
    let mut paired = BTreeSet::new();
    for (k, (i, p)) in loose.iter().enumerate() {
        let mut nearest = None;
        for (l, (j, q)) in loose.iter().enumerate().skip(k + 1) {
            if !budget.tick(stopped)? || out.truncated {
                return Ok(());
            }
            if i == j {
                continue;
            }
            let d = dist(*p, *q);
            if d > EPS && d <= gap && nearest.as_ref().is_none_or(|(_, _, old)| d < *old) {
                nearest = Some((l, *j, d));
            }
        }
        if let Some((l, j, d)) = nearest {
            paired.insert(k);
            paired.insert(l);
            report(
                &edges[*i],
                Some(&edges[j]),
                "boundary_gap",
                vec![*p, loose[l].1],
                "Birbirine yakın iki açık uç var; kaynak geometri kapalı değil.",
                "Boşluk yalnız raporlandı. Uçları birleştirme/uzatma önizlemesini inceleyin; tarama boşluk toleransı kaynak geometriyi onarmaz.",
                Some(d),
                out,
                limit,
            );
        }
    }
    for (k, (i, p)) in loose.iter().enumerate() {
        if !paired.contains(&k) {
            report(
                &edges[*i],
                None,
                "boundary_open_end",
                vec![*p],
                "Açık uç var; kapalı bölge dışında kalan bir uzantı da olabilir.",
                "Ucu ve komşu sınırları inceleyin; gerekiyorsa birleştirme/uzatma aracını ayrı çalıştırın.",
                None,
                out,
                limit,
            );
        }
    }
    Ok(())
}
