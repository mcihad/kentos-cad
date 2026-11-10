//! Çatının mevzuat denetimi (docs/adr/8044 §9; M44): each roof measured on the drawing against Planlı Alanlar İmar
//! Yönetmeliği 40 and 42, the fire regulation's 58/2 and, where the project reads it, the local regulation (Sivas İmar
//! Yönetmeliği 40/5, 42/5). A finding says the measure, the limit, the article and its gazette.
//!
//! - **Mahya (PAİY 40/4; Sivas 40/5-g):** the ridge's top over the saçak seviyesi (PAİY 4/ppp: "son kat tavan döşemesi
//!   üst kotu", the top of the storey the roof stands on, its kot farkı counted) at most 5.00 m; Sivas 4.00 m.
//! - **Eğim (Sivas 40/5-g):** "çatı yüzeyi en fazla % 40 eğimle yapılabilir": each sloping edge's; offered: the
//!   steeper ones at 40.
//! - **Saçak (PAİY 42/1):** a roof drawn from its walls (its overhang known) overhangs them at most 1.00 m.
//! - **Kar siperliği (Sivas 42/5):** "saçak uçlarında kar siperliklerinin yapılması mecburidir": a sloping roof has
//!   one; offered: 30 cm in from the eaves, as the Çatı tool gives it.
//! - **Hava bacası (PAİY 40/5):** at most 0.60 m over the roof's planes, at its highest corner as the tool sets it.
//! - **Duman bacası (BYKHY 58/2):** at least 0.80 m over the ridge ("bina mahyasının en az 0.8 m üzerine").
//! - **Çatı penceresi (Sivas 40/5-e):** at most 1.20 × 1.50 m (en/boy: along the eaves, down the slope in the roof's
//!   plane); its outline at least 1 m from the building's facade lines (the walls' outer faces the roof was drawn from,
//!   its eaves its overhang in; drawn by its corners, its eaves) and 2 m from another window, measured on the roof.
//!
//! A flat roof has no ridge, slope, snow guard or window rule. A finding is the drawing's measure, not an approval.

use kentos_commands::{Entity, LocalRules, RoofEntity, RoofPartKind, codes};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::roof as core;
use serde_json::json;

use crate::regulation_audit::{Article, Bound, Fix, Limit, Said, Source, metres, said};

/// The regulation's own gazette (PAİY 40 and 42 are as first published).
const PAIY: &str = "3.7.2017/30113";

pub const RIDGE_ARTICLE: Article = Article::paiy("40/4", PAIY);
pub const VENT_ARTICLE: Article = Article::paiy("40/5", PAIY);
pub const EAVES_ARTICLE: Article = Article::paiy("42/1", PAIY);
pub const CHIMNEY_ARTICLE: Article = Article {
    source: Source::Bykhy,
    article: "58/2",
    gazette: "19.12.2007/26735",
};
pub const SIVAS_RIDGE_ARTICLE: Article = Article::sivas("40/5-g");
pub const SIVAS_WINDOW_SIZE_ARTICLE: Article = Article::sivas("40/5-e-2");
pub const SIVAS_WINDOW_PLACE_ARTICLE: Article = Article::sivas("40/5-e-3");
pub const SIVAS_SNOW_GUARD_ARTICLE: Article = Article::sivas("42/5");

/// The codes the roofs' rules find, in their order.
pub const CODES: [&str; 8] = [
    codes::PAIY_RIDGE_HIGH,
    codes::PAIY_EAVES_WIDE,
    codes::PAIY_ROOF_VENT_HIGH,
    codes::BYKHY_CHIMNEY_LOW,
    codes::LOCAL_ROOF_SLOPE_STEEP,
    codes::LOCAL_SNOW_GUARD_MISSING,
    codes::LOCAL_ROOF_WINDOW_LARGE,
    codes::LOCAL_ROOF_WINDOW_CLOSE,
];

/// PAİY 40/4: the ridge's height over the saçak seviyesi, metres; Sivas 40/5-g's.
pub const RIDGE_MAX: f64 = 5.0;
pub const SIVAS_RIDGE_MAX: f64 = 4.0;
/// Sivas 40/5-g: a roof's slope at most, per cent.
pub const SIVAS_SLOPE_MAX: f64 = 40.0;
/// PAİY 42/1: the eaves' width at most, metres.
pub const EAVES_MAX: f64 = 1.0;
/// PAİY 40/5: a vent over the roof's planes at most, metres.
pub const VENT_MAX: f64 = 0.6;
/// BYKHY 58/2: a chimney over the ridge at least, metres.
pub const CHIMNEY_MIN: f64 = 0.8;
/// Sivas 40/5-e-2: a roof window's width (en) and length (boy) at most, metres.
pub const SIVAS_WINDOW_MAX: (f64, f64) = (1.2, 1.5);
/// Sivas 40/5-e-3: a roof window from the facade lines and from another window at least, metres.
pub const SIVAS_WINDOW_FACADE: f64 = 1.0;
pub const SIVAS_WINDOW_APART: f64 = 2.0;

/// A slope as a finding writes it: “%45”, “%42,5”.
fn per_cent(v: f64) -> String {
    core::slope_words(v)
}

/// The roofs' findings (docs/adr/8044 §9), each said through `push(said, slots, at, fix)`: the roof is the first slot,
/// a fix `cad.arch.edit`'s operation on it.
pub(crate) fn findings(doc: &Document, push: &mut dyn FnMut(Said, Vec<Slot>, V, Option<Fix>)) {
    let local = doc.settings().arch.as_ref().and_then(|a| a.local_rules) == Some(LocalRules::Sivas);
    for e in doc.entities() {
        let Entity::Roof(r) = e else { continue };
        let slot = Slot(e.base().id);
        let Shape::Roof { pts, roof: spec } = crate::geometry::roof_shape(r, doc.settings()) else {
            continue;
        };
        if pts.len() < 3 {
            continue;
        }
        let middle = V::new(
            pts.iter().map(|p| p.x).sum::<f64>() / pts.len() as f64,
            pts.iter().map(|p| p.y).sum::<f64>() / pts.len() as f64,
        );
        let sloping = spec.slopes.iter().any(Option::is_some);
        roof_rules(r, &pts, &spec, local, sloping, slot, middle, push);
        part_rules(r, &pts, &spec, local, slot, push);
    }
}

/// The roof's own rules: its ridge, its eaves, its slope and its snow guard.
#[allow(clippy::too_many_arguments)]
fn roof_rules(
    r: &RoofEntity,
    pts: &[V],
    spec: &core::RoofSpec,
    local: bool,
    sloping: bool,
    slot: Slot,
    middle: V,
    push: &mut dyn FnMut(Said, Vec<Slot>, V, Option<Fix>),
) {
    // Mahya (PAİY 40/4; Sivas 40/5-g): over the saçak seviyesi, the roof's kot farkı counted.
    if sloping && let Ok(rise) = core::ridge_rise(pts, spec) {
        let height = rise + r.roof.offset;
        if let Some(s) = said(
            codes::PAIY_RIDGE_HIGH,
            "Mahya yüksekliği (saçak seviyesinden)",
            height,
            Bound::AtMost,
            Limit {
                value: RIDGE_MAX,
                article: RIDGE_ARTICLE,
            },
            local.then_some(Limit {
                value: SIVAS_RIDGE_MAX,
                article: SIVAS_RIDGE_ARTICLE,
            }),
            metres,
            "",
            "Çatının eğimini azaltın ya da çatıyı kot farkı olmadan oturtun.",
        ) {
            push(s, vec![slot], middle, None);
        }
    }
    // Saçak (PAİY 42/1): known only when the roof was drawn from its walls.
    if let Some(o) = r.roof.overhang
        && let Some(s) = said(
            codes::PAIY_EAVES_WIDE,
            "Saçak genişliği",
            o,
            Bound::AtMost,
            Limit {
                value: EAVES_MAX,
                article: EAVES_ARTICLE,
            },
            None,
            metres,
            "",
            "Saçağı daraltın: çatıyı Çatı aracıyla en çok 1,00 m saçakla yeniden çizin.",
        )
    {
        push(s, vec![slot], middle, None);
    }
    if !local || !sloping {
        return;
    }
    // Eğim (Sivas 40/5-g).
    let steepest = r
        .roof
        .slopes
        .iter()
        .flatten()
        .copied()
        .fold(0.0_f64, f64::max);
    if steepest > SIVAS_SLOPE_MAX + 1e-9 {
        let slopes: Vec<Option<f64>> = r
            .roof
            .slopes
            .iter()
            .map(|s| s.map(|v| v.min(SIVAS_SLOPE_MAX)))
            .collect();
        push(
            Said {
                code: codes::LOCAL_ROOF_SLOPE_STEEP,
                message: format!(
                    "Çatının eğimi {} > {} — {}",
                    per_cent(steepest),
                    per_cent(SIVAS_SLOPE_MAX),
                    SIVAS_RIDGE_ARTICLE.short()
                ),
                repair: "Çatının eğimini %40'a indirin.".into(),
                rule: SIVAS_RIDGE_ARTICLE.named(),
                binding: None,
            },
            vec![slot],
            middle,
            Some(Fix {
                label: "Eğimi %40'a indir".into(),
                operation: json!({"kind": "set", "slopes": slopes}),
            }),
        );
    }
    // Kar siperliği (Sivas 42/5).
    if r.roof.snow_guard.is_none() {
        push(
            Said {
                code: codes::LOCAL_SNOW_GUARD_MISSING,
                message: format!(
                    "Eğimli çatının saçağında kar siperliği yok — {}",
                    SIVAS_SNOW_GUARD_ARTICLE.short()
                ),
                repair: "Çatıya kar siperliği verin (Çatı aracında Kar siperliği, saçaktan 30 cm içeride).".into(),
                rule: SIVAS_SNOW_GUARD_ARTICLE.named(),
                binding: None,
            },
            vec![slot],
            middle,
            Some(Fix {
                label: "Kar siperliği ver (saçaktan 30 cm)".into(),
                operation: json!({"kind": "set", "snowGuard": kentos_commands::DEFAULT_SNOW_GUARD}),
            }),
        );
    }
}

/// The rules of the parts standing through the roof or set in it: chimneys, vents and windows.
fn part_rules(
    r: &RoofEntity,
    pts: &[V],
    spec: &core::RoofSpec,
    local: bool,
    slot: Slot,
    push: &mut dyn FnMut(Said, Vec<Slot>, V, Option<Fix>),
) {
    // The parts with one of them raised to `rise`: `cad.arch.edit`'s operation.
    let raised = |i: usize, rise: f64| {
        let mut parts = r.roof.parts.clone();
        parts[i].rise = Some(rise);
        json!({"kind": "set", "parts": parts})
    };
    for (i, part) in r.roof.parts.iter().enumerate() {
        let at = V::new(part.at.x, part.at.y);
        let found = match part.kind {
            RoofPartKind::Vent => said(
                codes::PAIY_ROOF_VENT_HIGH,
                "Hava bacasının çatı örtüsünü aşan yüksekliği",
                part.rise_m().unwrap_or(0.0),
                Bound::AtMost,
                Limit {
                    value: VENT_MAX,
                    article: VENT_ARTICLE,
                },
                None,
                metres,
                "",
                "Hava bacasını çatı örtüsünün en çok 0,60 m üstünde bitirin.",
            )
            .map(|s| {
                let fix = Fix {
                    label: "Hava bacasını 0,60 m'ye indir".into(),
                    operation: raised(i, VENT_MAX),
                };
                (s, Some(fix))
            }),
            RoofPartKind::Chimney => said(
                codes::BYKHY_CHIMNEY_LOW,
                "Duman bacasının mahyayı aşan yüksekliği",
                part.rise_m().unwrap_or(0.0),
                Bound::AtLeast,
                Limit {
                    value: CHIMNEY_MIN,
                    article: CHIMNEY_ARTICLE,
                },
                None,
                metres,
                "",
                "Bacayı mahyanın en az 0,80 m üstüne çıkarın.",
            )
            .map(|s| {
                let fix = Fix {
                    label: "Bacayı mahyanın 0,80 m üstüne çıkar".into(),
                    operation: raised(i, CHIMNEY_MIN),
                };
                (s, Some(fix))
            }),
            RoofPartKind::Window if local => window_size(part.width, part.depth).map(|s| (s, None)),
            RoofPartKind::Window | RoofPartKind::Downpipe => None,
        };
        if let Some((s, fix)) = found {
            push(s, vec![slot], at, fix);
        }
    }
    if !local {
        return;
    }
    // Çatı pencereleri: from the facade lines and from each other (Sivas 40/5-e-3).
    let overhang = r.roof.overhang.unwrap_or(0.0);
    let windows: Vec<(V, Vec<[f64; 3]>, Vec<V>)> = spec
        .parts
        .iter()
        .filter(|p| p.kind == core::PartKind::Window)
        .filter_map(|p| {
            let ring = core::part_ring(pts, spec, p)?;
            let lifted = ring
                .iter()
                .map(|q| {
                    [
                        q.x,
                        q.y,
                        core::height_at(pts, spec, *q).unwrap_or(spec.base),
                    ]
                })
                .collect();
            Some((p.at, lifted, ring))
        })
        .collect();
    for (at, _, ring) in &windows {
        let facade = ring
            .iter()
            .map(|q| inside(pts, *q))
            .fold(f64::INFINITY, f64::min)
            - overhang;
        if facade < SIVAS_WINDOW_FACADE - 1e-6 {
            push(
                Said {
                    code: codes::LOCAL_ROOF_WINDOW_CLOSE,
                    message: format!(
                        "Çatı penceresinin bina cephe hattına uzaklığı {} < {} — {}",
                        metres(facade.max(0.0)),
                        metres(SIVAS_WINDOW_FACADE),
                        SIVAS_WINDOW_PLACE_ARTICLE.short()
                    ),
                    repair: "Pencereyi cephe hattından en az 1 m içeri alın.".into(),
                    rule: SIVAS_WINDOW_PLACE_ARTICLE.named(),
                    binding: None,
                },
                vec![slot],
                *at,
                None,
            );
        }
    }
    for (i, (at, a, _)) in windows.iter().enumerate() {
        for (_, b, _) in windows.iter().skip(i + 1) {
            let apart = ring_distance(a, b);
            if apart < SIVAS_WINDOW_APART - 1e-6 {
                push(
                    Said {
                        code: codes::LOCAL_ROOF_WINDOW_CLOSE,
                        message: format!(
                            "İki çatı penceresi arası {} < {} — {}",
                            metres(apart),
                            metres(SIVAS_WINDOW_APART),
                            SIVAS_WINDOW_PLACE_ARTICLE.short()
                        ),
                        repair: "Pencereleri çatının üstünde en az 2 m aralıkla yerleştirin."
                            .into(),
                        rule: SIVAS_WINDOW_PLACE_ARTICLE.named(),
                        binding: None,
                    },
                    vec![slot],
                    *at,
                    None,
                );
            }
        }
    }
}

/// A roof window's size against Sivas 40/5-e-2: its width along the eaves (en) and its length down the slope in the
/// roof's plane (boy).
fn window_size(width: f64, length: f64) -> Option<Said> {
    let (w, l) = SIVAS_WINDOW_MAX;
    (width > w + 1e-6 || length > l + 1e-6).then(|| Said {
        code: codes::LOCAL_ROOF_WINDOW_LARGE,
        message: format!(
            "Çatı penceresi {} × {} m, en çok {} × {} m (en/boy) — {}",
            crate::regulation_audit::m2(width),
            crate::regulation_audit::m2(length),
            crate::regulation_audit::m2(w),
            crate::regulation_audit::m2(l),
            SIVAS_WINDOW_SIZE_ARTICLE.short()
        ),
        repair: "Pencereyi küçültün (Çatı penceresi aracında En ve Boy).".into(),
        rule: SIVAS_WINDOW_SIZE_ARTICLE.named(),
        binding: None,
    })
}

/// How far in from the eaves line `pts` (convex, either way round) the plan point `q` is: its nearest edge's line.
fn inside(pts: &[V], q: V) -> f64 {
    let n = pts.len();
    let area: f64 = (0..n)
        .map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            a.x * b.y - b.x * a.y
        })
        .sum();
    let turn = if area >= 0.0 { 1.0 } else { -1.0 };
    (0..n)
        .filter_map(|i| {
            let (a, b) = (pts[i], pts[(i + 1) % n]);
            let l = (b.x - a.x).hypot(b.y - a.y);
            (l > 1e-12).then(|| turn * ((b.x - a.x) * (q.y - a.y) - (b.y - a.y) * (q.x - a.x)) / l)
        })
        .fold(f64::INFINITY, f64::min)
}

/// The shortest distance between two closed rings in space: their edges' nearest points.
fn ring_distance(a: &[[f64; 3]], b: &[[f64; 3]]) -> f64 {
    let edges = |r: &[[f64; 3]]| -> Vec<([f64; 3], [f64; 3])> {
        (0..r.len()).map(|i| (r[i], r[(i + 1) % r.len()])).collect()
    };
    let (ea, eb) = (edges(a), edges(b));
    ea.iter()
        .flat_map(|(p, q)| {
            eb.iter()
                .map(move |(s, t)| segment_distance(*p, *q, *s, *t))
        })
        .fold(f64::INFINITY, f64::min)
}

/// The shortest distance between the segments `p`→`q` and `s`→`t` in space (Ericson, Real-Time Collision Detection
/// 5.1.9: the closest points of two segments).
fn segment_distance(p: [f64; 3], q: [f64; 3], s: [f64; 3], t: [f64; 3]) -> f64 {
    let sub = |a: [f64; 3], b: [f64; 3]| [a[0] - b[0], a[1] - b[1], a[2] - b[2]];
    let dot = |a: [f64; 3], b: [f64; 3]| a[0] * b[0] + a[1] * b[1] + a[2] * b[2];
    let at = |o: [f64; 3], d: [f64; 3], k: f64| [o[0] + d[0] * k, o[1] + d[1] * k, o[2] + d[2] * k];
    let (d1, d2, r) = (sub(q, p), sub(t, s), sub(p, s));
    let (a, e, f) = (dot(d1, d1), dot(d2, d2), dot(d2, r));
    let (k1, k2) = if a <= 1e-18 && e <= 1e-18 {
        (0.0, 0.0)
    } else if a <= 1e-18 {
        (0.0, (f / e).clamp(0.0, 1.0))
    } else {
        let c = dot(d1, r);
        if e <= 1e-18 {
            ((-c / a).clamp(0.0, 1.0), 0.0)
        } else {
            let b = dot(d1, d2);
            let denom = a * e - b * b;
            let mut k1 = if denom > 1e-18 {
                ((b * f - c * e) / denom).clamp(0.0, 1.0)
            } else {
                0.0
            };
            let mut k2 = (b * k1 + f) / e;
            if k2 < 0.0 {
                k2 = 0.0;
                k1 = (-c / a).clamp(0.0, 1.0);
            } else if k2 > 1.0 {
                k2 = 1.0;
                k1 = ((b - c) / a).clamp(0.0, 1.0);
            }
            (k1, k2)
        }
    };
    let gap = sub(at(p, d1, k1), at(s, d2, k2));
    dot(gap, gap).sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn segments_apart_in_space() {
        // Parallel, 2 m apart; crossing at a height of 1 m; end to end.
        let d = segment_distance(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [0.0, 2.0, 0.0],
            [1.0, 2.0, 0.0],
        );
        assert!((d - 2.0).abs() < 1e-12);
        let d = segment_distance(
            [0.0, 0.0, 0.0],
            [2.0, 0.0, 0.0],
            [1.0, -1.0, 1.0],
            [1.0, 1.0, 1.0],
        );
        assert!((d - 1.0).abs() < 1e-12);
        let d = segment_distance(
            [0.0, 0.0, 0.0],
            [1.0, 0.0, 0.0],
            [4.0, 0.0, 0.0],
            [5.0, 0.0, 0.0],
        );
        assert!((d - 3.0).abs() < 1e-12);
    }

    #[test]
    fn a_point_in_from_the_eaves() {
        let rect = [
            V::new(0.0, 0.0),
            V::new(12.0, 0.0),
            V::new(12.0, 9.0),
            V::new(0.0, 9.0),
        ];
        assert!((inside(&rect, V::new(3.0, 1.5)) - 1.5).abs() < 1e-12);
        let clockwise: Vec<V> = rect.iter().rev().copied().collect();
        assert!((inside(&clockwise, V::new(11.0, 4.0)) - 1.0).abs() < 1e-12);
    }

    #[test]
    fn a_window_larger_than_sivas_allows() {
        assert!(window_size(1.2, 1.5).is_none());
        let s = window_size(1.4, 1.6).expect("too large");
        assert_eq!(s.code, codes::LOCAL_ROOF_WINDOW_LARGE);
        assert_eq!(
            s.message,
            "Çatı penceresi 1.40 × 1.60 m, en çok 1.20 × 1.50 m (en/boy) — Sivas 40/5-e-2"
        );
    }
}
