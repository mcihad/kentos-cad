//! Drawing health (docs/adr/1005 §1), only when `checks` asks for `health`: candidates for review, never repairs.
//!
//! - `t_junction_short` / `t_junction_overshoot`: a line's free end stops short of, or passes over, the inside of
//!   another object's straight edge by at most `maxGap` along the line's own direction.
//! - `angle_deviation`: a straight edge within `maxAngle` of horizontal or vertical but not on it.
//! - `stray_object`: with ten or more objects, one whose centre lies farther from the box of the 10th–90th
//!   percentile centres than ten times that box's diagonal (at least 1 m).
//! - `block_definition_duplicate`: block definitions of the same content (parts, base and attributes) under
//!   different names.

use super::*;

/// A straight edge of an object.
pub(super) struct Straight {
    uid: Uuid,
    a: Vec2,
    b: Vec2,
    /// A line's own edge: its ends are free ends to test.
    line: bool,
}

/// What the health checks read of the examined objects.
#[derive(Default)]
pub(super) struct Health {
    straights: Vec<Straight>,
    centres: Vec<(Uuid, Vec2)>,
}

const NEAR: f64 = 1e-9;

impl Health {
    pub(super) fn collect(&mut self, e: &Entity, uid: Uuid) {
        let mut pts: Vec<Vec2> = Vec::new();
        match e {
            Entity::Line(l) => {
                self.straights.push(Straight { uid, a: l.a, b: l.b, line: true });
                pts.extend([l.a, l.b]);
            }
            Entity::Polyline(_) | Entity::Polygon(_) => {
                for edge in kentos_geometry_core::ops::edges::entity_edges(&shape(e)) {
                    if let kentos_geometry_core::geom::intersect::Edge::Seg { a, b } = edge {
                        let (a, b) = (Vec2 { x: a.x, y: a.y }, Vec2 { x: b.x, y: b.y });
                        self.straights.push(Straight { uid, a, b, line: false });
                        pts.extend([a, b]);
                    }
                }
            }
            Entity::Circle(c) => pts.push(c.c),
            Entity::Arc(a) => pts.push(a.c),
            Entity::Point(p) => pts.push(p.p),
            Entity::Insert(i) => pts.push(i.p),
            Entity::Text(t) => pts.push(t.p),
            _ => {}
        }
        if !pts.is_empty() {
            let n = pts.len() as f64;
            let c = Vec2 { x: pts.iter().map(|p| p.x).sum::<f64>() / n, y: pts.iter().map(|p| p.y).sum::<f64>() / n };
            self.centres.push((uid, c));
        }
    }
}

fn len(a: Vec2, b: Vec2) -> f64 {
    (b.x - a.x).hypot(b.y - a.y)
}

fn finding(code: &str, severity: InspectionSeverity, uids: Vec<String>, points: Vec<Vec2>, message: String,
    repair: &str, distance: Option<f64>) -> InspectionFinding {
    InspectionFinding {
        group: InspectionGroup::Health,
        severity,
        code: code.into(),
        uids,
        points,
        message,
        repair: repair.into(),
        distance,
    }
}

/// Where the ray from `e` along `d` (a unit vector) meets the inside of `s`: its distance along the ray (negative
/// behind `e`) and the point; none when parallel or outside `s`'s inside.
fn meet(e: Vec2, d: Vec2, s: &Straight) -> Option<(f64, Vec2)> {
    let (ux, uy) = (s.b.x - s.a.x, s.b.y - s.a.y);
    let den = d.x * uy - d.y * ux;
    if den.abs() <= NEAR * len(s.a, s.b) {
        return None;
    }
    let (wx, wy) = (s.a.x - e.x, s.a.y - e.y);
    let t = (wx * uy - wy * ux) / den;
    let u = (wx * d.y - wy * d.x) / den;
    let inside = NEAR / len(s.a, s.b).max(NEAR);
    (u > inside && u < 1.0 - inside).then(|| (t, Vec2 { x: e.x + t * d.x, y: e.y + t * d.y }))
}

/// A free end of line `l`: no other straight edge's end at it.
fn free(end: Vec2, me: usize, all: &[&Straight]) -> bool {
    !all.iter().enumerate().any(|(i, s)| i != me && (len(s.a, end) <= NEAR || len(s.b, end) <= NEAR))
}

pub(super) fn inspect(
    doc: &Document,
    h: &Health,
    max_gap: f64,
    max_angle: f64,
    limit: usize,
    budget: &mut Budget,
    stopped: &mut impl FnMut() -> bool,
    out: &mut DrawingInspected,
) -> Result<(), Stop> {
    // Findings in the drawing's order (the scope is a set of ids): what is listed first is drawn first.
    let place = |uid: &Uuid| doc.slot_of(*uid).map_or(u32::MAX, |s| s.0);
    let mut order: Vec<usize> = (0..h.straights.len()).collect();
    order.sort_by_key(|i| place(&h.straights[*i].uid));
    let straights: Vec<&Straight> = order.iter().map(|i| &h.straights[*i]).collect();
    let mut centres: Vec<&(Uuid, Vec2)> = h.centres.iter().collect();
    centres.sort_by_key(|c| place(&c.0));
    // T junctions.
    'lines: for (i, l) in straights.iter().enumerate().filter(|(_, s)| s.line) {
        let length = len(l.a, l.b);
        if length <= NEAR {
            continue;
        }
        for (end, from, name) in [(l.b, l.a, "end"), (l.a, l.b, "start")] {
            if !free(end, i, &straights) {
                continue;
            }
            let d = Vec2 { x: (end.x - from.x) / length, y: (end.y - from.y) / length };
            let mut best: Option<(f64, Vec2, &&Straight)> = None;
            for s in straights.iter().filter(|s| s.uid != l.uid) {
                if !budget.tick(stopped)? {
                    break 'lines;
                }
                if let Some((t, x)) = meet(end, d, s)
                    && t.abs() > NEAR
                    && t.abs() <= max_gap
                    && -t < length
                    && best.is_none_or(|(b, _, _)| t.abs() < b.abs())
                {
                    best = Some((t, x, s));
                }
            }
            if let Some((t, x, s)) = best {
                let (code, words) = if t > 0.0 {
                    ("t_junction_short", "kenara ulaşmıyor")
                } else {
                    ("t_junction_overshoot", "kenarı geçiyor")
                };
                push(
                    out,
                    finding(
                        code,
                        InspectionSeverity::Warning,
                        vec![l.uid.to_string(), s.uid.to_string()],
                        vec![end, x],
                        format!(
                            "Çizginin {} ucu öbür nesnenin kenarına {:.4} m {words}.",
                            if name == "end" { "bitiş" } else { "başlangıç" },
                            t.abs()
                        ),
                        "Ucu kesişime getirin: Uzat/Buda ya da cad.drawing.repair (ölçülü ve kısıtlı nesneye dokunmaz).",
                        Some(t.abs()),
                    ),
                    limit,
                );
            }
        }
    }
    // Angle deviations.
    let tolerance = max_angle.to_radians();
    if tolerance > 0.0 {
        for s in &straights {
            if !budget.tick(stopped)? {
                break;
            }
            let length = len(s.a, s.b);
            if length <= NEAR {
                continue;
            }
            let angle = (s.b.y - s.a.y).atan2(s.b.x - s.a.x);
            let quarter = std::f64::consts::FRAC_PI_2;
            let deviation = (angle - (angle / quarter).round() * quarter).abs();
            if deviation > 1e-12 && deviation <= tolerance {
                push(
                    out,
                    finding(
                        "angle_deviation",
                        InspectionSeverity::Info,
                        vec![s.uid.to_string()],
                        vec![s.a, s.b],
                        format!(
                            "Kenar yatay ya da düşeyden {:.4}° sapıyor; ucu {:.4} m kayık.",
                            deviation.to_degrees(),
                            length * deviation.sin()
                        ),
                        "Bilerek eğikse bırakın; değilse Döndür ya da Ortogonal ile düzeltin.",
                        Some(length * deviation.sin()),
                    ),
                    limit,
                );
            }
        }
    }
    // Stray objects.
    if centres.len() >= 10 {
        let pick = |mut v: Vec<f64>, q: f64| {
            v.sort_by(f64::total_cmp);
            v[((v.len() - 1) as f64 * q).round() as usize]
        };
        let xs: Vec<f64> = centres.iter().map(|c| c.1.x).collect();
        let ys: Vec<f64> = centres.iter().map(|c| c.1.y).collect();
        let (x0, x1) = (pick(xs.clone(), 0.1), pick(xs, 0.9));
        let (y0, y1) = (pick(ys.clone(), 0.1), pick(ys, 0.9));
        let reach = 10.0 * (x1 - x0).hypot(y1 - y0).max(0.1);
        for (uid, c) in centres.iter().map(|c| (&c.0, &c.1)) {
            let dx = (x0 - c.x).max(c.x - x1).max(0.0);
            let dy = (y0 - c.y).max(c.y - y1).max(0.0);
            let away = dx.hypot(dy);
            if away > reach {
                push(
                    out,
                    finding(
                        "stray_object",
                        InspectionSeverity::Info,
                        vec![uid.to_string()],
                        vec![*c],
                        format!("Nesne çizimin ana bölgesinden {away:.3} m uzakta; yanlışlıkla taşınmış ya da kopyalanmış olabilir."),
                        "Nesneye yakınlaştırıp inceleyin; gereksizse silin.",
                        Some(away),
                    ),
                    limit,
                );
            }
        }
    }
    // Block definitions of the same content.
    let mut seen: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
    for b in doc.blocks() {
        if b.xref.is_some() {
            continue;
        }
        let mut parts: Vec<serde_json::Value> = b
            .entities
            .iter()
            .map(|e| {
                let mut v = serde_json::to_value(e).unwrap_or_default();
                if let Some(o) = v.as_object_mut() {
                    o.remove("id");
                }
                v
            })
            .collect();
        parts.sort_by_key(|v| v.to_string());
        let key = serde_json::json!({ "base": b.base, "entities": parts, "attributes": b.attributes }).to_string();
        seen.entry(key).or_default().push(b.name.clone());
    }
    for names in seen.into_values().filter(|n| n.len() > 1) {
        push(
            out,
            finding(
                "block_definition_duplicate",
                InspectionSeverity::Info,
                Vec::new(),
                Vec::new(),
                format!("Aynı içerikli blok tanımları: {}.", names.join(", ")),
                "Yerleştirmeleri tek tanıma bağlayıp öbürlerini kullanılmayan tanım olarak temizleyin.",
                None,
            ),
            limit,
        );
    }
    Ok(())
}
