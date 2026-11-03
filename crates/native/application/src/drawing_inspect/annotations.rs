//! Annotation checks share the public dimension reader; no displayed number is treated as a measurement.
use kentos_commands::{
    DimensionBond, Entity, InspectionFinding, InspectionGroup, InspectionSeverity, Vec2,
};
use kentos_domain::{Document, Uuid};
use kentos_project::format::Format;

fn finding(
    uid: Uuid,
    sources: &[String],
    code: &str,
    severity: InspectionSeverity,
    points: Vec<Vec2>,
    message: String,
    repair: &str,
) -> InspectionFinding {
    let primary = uid.to_string();
    let mut uids = vec![primary.clone()];
    uids.extend(
        sources
            .iter()
            .filter(|u| **u != primary)
            .cloned()
            .collect::<std::collections::BTreeSet<_>>(),
    );
    InspectionFinding {
        group: InspectionGroup::Annotations,
        severity,
        code: code.into(),
        uids,
        points,
        message,
        repair: repair.into(),
        distance: None,
    }
}

/// Bounded by a dimension's references or a leader's 64 arms. Callers retain result/work caps.
pub fn findings(doc: &Document, uid: Uuid, e: &Entity) -> Vec<InspectionFinding> {
    let mut out = Vec::new();
    match e {
        Entity::Dimension(d) => {
            let read = crate::dimensions_query::read(&Format::of(doc.settings()), uid, d);
            let sources: Vec<_> = d
                .refs
                .iter()
                .flat_map(|r| r.each())
                .map(|(_, r)| Uuid::from_bytes(r.uid.0).to_string())
                .collect();
            let missing = sources.iter().any(|u| {
                Uuid::parse_str(u)
                    .ok()
                    .is_none_or(|u| doc.slot_of(u).is_none())
            });
            let broken_points: Vec<_> = d
                .refs
                .iter()
                .flat_map(|r| r.each())
                .filter(|(_, r)| !r.live() || doc.slot_of(Uuid::from_bytes(r.uid.0)).is_none())
                .map(|(name, _)| match name {
                    "b" => d.b,
                    "c" => d.c.unwrap_or(d.a),
                    _ => d.a,
                })
                .collect();
            let free_points: Vec<_> = [("a", Some(d.a)), ("b", Some(d.b)), ("c", d.c)]
                .into_iter()
                .filter(|(name, p)| {
                    p.is_some()
                        && !d
                            .refs
                            .iter()
                            .flat_map(|r| r.each())
                            .any(|(k, _)| k == *name)
                })
                .filter_map(|(_, p)| p)
                .collect();
            let point = if !broken_points.is_empty() {
                broken_points
            } else if read.bond == DimensionBond::Partial {
                free_points
            } else {
                vec![d.a]
            };
            let (code, severity, words) = if missing || read.bond == DimensionBond::Broken {
                (
                    "dimension_broken",
                    InspectionSeverity::Warning,
                    "Ölçünün kaynak bağı kopuk; saklanan geometri güncel kaynak ölçüsü sayılmaz.",
                )
            } else {
                match read.bond {
                    DimensionBond::Partial => (
                        "dimension_partial",
                        InspectionSeverity::Warning,
                        "Ölçü kısmen bağlı; serbest uç kaynak değişimini izlemez.",
                    ),
                    DimensionBond::Free => (
                        "dimension_free",
                        InspectionSeverity::Info,
                        "Ölçü serbest; kaynak değişimini izlemez.",
                    ),
                    _ => ("", InspectionSeverity::Info, ""),
                }
            };
            if !code.is_empty() {
                out.push(finding(uid, &sources, code, severity, point.clone(), words.into(), "Kaynağa git ile referansları inceleyin; Yeniden bağla ile ölçünün uçlarını doğru nesneye kenetleyin."));
            }
            if !read.measured.is_finite() {
                out.push(finding(
                    uid,
                    &sources,
                    "dimension_invalid",
                    InspectionSeverity::Error,
                    point.clone(),
                    "Ölçünün geometrisinden sonlu bir değer hesaplanamıyor.".into(),
                    "Ölçünün kaynak noktalarını ve türünü düzeltin.",
                ));
            } else if read.overridden {
                let actual = match read.unit.as_str() {
                    "length" | "coordinate" => Format::of(doc.settings()).length(read.measured),
                    "angle" => Format::of(doc.settings()).angle(read.measured),
                    _ => format!("{} %", read.measured),
                };
                out.push(finding(uid, &sources, "dimension_override", InspectionSeverity::Warning, point,
                    format!("Ölçü yazısı elle değiştirilmiş. Gerçek: {actual} · Gösterilen: {}", read.shown),
                    "Öznitelikler'de ölçü yazısını kontrol edin; ölçülen değeri göstermek için <> kullanın veya yazı değişikliğini kaldırın."));
            }
        }
        Entity::Leader(l) => {
            let arms = std::iter::once((l.tip.as_ref(), l.pts.first()))
                .chain(l.arms.iter().map(|a| (a.tip.as_ref(), a.pts.first())));
            let mut sources = Vec::new();
            let mut broken = Vec::new();
            let mut free = 0;
            let mut total = 0;
            for (tip, at) in arms {
                total += 1;
                match tip {
                    None => free += 1,
                    Some(r) => {
                        let source = Uuid::from_bytes(r.uid.0);
                        sources.push(source.to_string());
                        if !r.live() || doc.slot_of(source).is_none() {
                            broken.extend(at.copied());
                        }
                    }
                }
            }
            if !broken.is_empty() {
                out.push(finding(uid, &sources, "leader_broken", InspectionSeverity::Warning, broken,
                    "Kılavuzun en az bir kolunun kaynak bağı kopuk; ok ucu son yerinde kalıyor.".into(),
                    "Açıklama → Kılavuz ucunu bağla ile sorunlu kolun ucunu doğru nesneye kenetleyin."));
            }
            if free > 0 {
                out.push(finding(uid, &sources, "leader_free", if free == total { InspectionSeverity::Info } else { InspectionSeverity::Warning }, l.pts.first().copied().into_iter().collect(),
                    format!("Kılavuzun {total} kolundan {free} tanesi serbest; bu uçlar kaynak değişimini izlemez."),
                    "Serbest uçlar tasarım kararı olabilir; gerekiyorsa Açıklama → Kılavuz ucunu bağla kullanın."));
            }
        }
        _ => {}
    }
    out
}
