//! Otopark yerlerinin mevzuat denetimi (docs/adr/8045 §6): the library's Otopark yeri blocks (`Document::parking_stalls`)
//! read by Otopark Yönetmeliği. A finding says the measure, the limit, the article and its gazette.
//!
//! - **Ölçü (5/1-h-8, 9):** "Birim otopark alanlarının uzun kenarı en az 4,90 metre, kısa kenarı ise engelliler için en
//!   az 3,50 metre diğerleri için en az 2,40 metre"; a van's place 3.00 × 6.00 m (h-8). `parking_space_small`, fixed by
//!   its En and Boy (`cad.blocks.parameters`) when the larger place runs into no other place of its storey: a place in a
//!   row grown into its neighbour is no fix, its row moves.
//! - **Manevra (4/1-ç):** "Binek otoları için birim park alanı, manevra alanı dâhil en az 20 m²": a room named Otopark
//!   or Garaj holding more places than its net area over 20 m², `parking_area_short`.
//! - **Engelli (4/1-ı):** a public building's places (`ParkingReport::public`), one in every twenty and one at least
//!   for the disabled: `parking_disabled_short`. No fix: a wider place moves its row.
//! - **Şarj (5/1-j):** twenty required places or more, 5 % of them and one at least for electric cars:
//!   `parking_charging_short`, fixed by a car's place made Şarjlı.
//!
//! Without a place drawn only Otopark hesabı's area (docs/adr/8036) is read: nothing here is said.

use kentos_commands::{Entity, codes};
use kentos_domain::provisions::{STALL_KIND, STALL_LENGTH, STALL_WIDTH, Stall, StallKind};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;
use serde_json::{Map, Value, json};

use crate::block_values::{PARAMETERS, button_metres, least_value};
use crate::regulation_audit::{Article, Bound, Fix, Limit, Said, Source, metres, said};

/// Otopark Yönetmeliği 5/1-h-9: a place's sides.
pub const SIZE_ARTICLE: Article = Article {
    source: Source::Otopark,
    article: "5/1-h-9",
    gazette: "22.2.2018/30340",
};
/// 5/1-h-8: a van's place.
pub const VAN_ARTICLE: Article = Article {
    source: Source::Otopark,
    article: "5/1-h-8",
    gazette: "22.2.2018/30340",
};
/// 4/1-ç: a car's 20 m², its manoeuvre in.
pub const AREA_ARTICLE: Article = Article {
    source: Source::Otopark,
    article: "4/1-ç",
    gazette: "22.2.2018/30340",
};
/// 4/1-ı: the disabled's places.
pub const DISABLED_ARTICLE: Article = Article {
    source: Source::Otopark,
    article: "4/1-ı",
    gazette: "22.2.2018/30340",
};
/// 5/1-j: the electric cars' places.
pub const CHARGING_ARTICLE: Article = Article {
    source: Source::Otopark,
    article: "5/1-j",
    gazette: "25.3.2021/31434 değişikliğiyle",
};

/// A place in a sentence's possessive: “Engelli otopark yerinin”.
fn whose(kind: StallKind) -> &'static str {
    match kind {
        StallKind::Car => "Otopark yerinin",
        StallKind::Disabled => "Engelli otopark yerinin",
        StallKind::Charging => "Şarjlı otopark yerinin",
        StallKind::Van => "Kamyonet otopark yerinin",
    }
}

/// Whether two convex outlines overlap (separating axes; a touch within a millimetre is none).
fn overlap(a: &[V; 4], b: &[V; 4]) -> bool {
    for poly in [a, b] {
        for k in 0..4 {
            let (p, q) = (poly[k], poly[(k + 1) % 4]);
            let (nx, ny) = (p.y - q.y, q.x - p.x);
            let len = nx.hypot(ny);
            if len == 0.0 {
                continue;
            }
            let span = |r: &[V; 4]| {
                r.iter()
                    .map(|v| (v.x * nx + v.y * ny) / len)
                    .fold((f64::MAX, f64::MIN), |(lo, hi), d| (lo.min(d), hi.max(d)))
            };
            let ((a0, a1), (b0, b1)) = (span(a), span(b));
            if a1 <= b0 + 1e-3 || b1 <= a0 + 1e-3 {
                return false;
            }
        }
    }
    true
}

/// The fix that gives a place its kind's least sides: its En and Boy raised, each the least its rule allows; none when
/// the larger place would run into another place of its storey.
fn size_fix(doc: &Document, s: &Stall, others: &[Stall]) -> Option<Fix> {
    let (short, long) = s.kind.least();
    let mut values = Map::new();
    let (mut w, mut l) = (s.width, s.length);
    if w + 1e-6 < short {
        let v = least_value(doc, s.slot, STALL_WIDTH, short / s.scale.0)?;
        values.insert(STALL_WIDTH.into(), json!(v));
        w = v * s.scale.0;
    }
    if l + 1e-6 < long {
        let v = least_value(doc, s.slot, STALL_LENGTH, long / s.scale.1)?;
        values.insert(STALL_LENGTH.into(), json!(v));
        l = v * s.scale.1;
    }
    let grown = s.outline(w, l);
    if others.iter().any(|o| {
        o.slot != s.slot && o.level == s.level && overlap(&grown, &o.outline(o.width, o.length))
    }) {
        return None;
    }
    (!values.is_empty()).then(|| Fix {
        label: format!("Yeri {} × {} m yap", button_metres(w), button_metres(l)),
        operation: Value::Object(values),
        command: Some(PARAMETERS),
    })
}

/// The parking places' findings (docs/adr/8045 §6), each said through `push(said, slots, at, fix)`.
pub(crate) fn findings(doc: &Document, push: &mut impl FnMut(Said, Vec<Slot>, V, Option<Fix>)) {
    let report = doc.parking_report();
    if report.stalls.is_empty() {
        return;
    }
    let at = |s: &Stall| V::new(s.at.x, s.at.y);
    // Each place's sides (5/1-h-8, 9): En across, Boy along, the place read as drawn.
    for s in &report.stalls {
        let (short, long) = s.kind.least();
        let article = if s.kind == StallKind::Van {
            VAN_ARTICLE
        } else {
            SIZE_ARTICLE
        };
        let repair = "Yeri büyütün: binek 2,40 × 4,90 m, engelli 3,50 × 4,90 m, kamyonet 3,00 × 6,00 m (manevra alanı \
                      hariç); Ölçü tablosundan seçilebilir. Sıradaki yer büyüyünce komşusuna biner: sırayı kaydırın.";
        let fix = size_fix(doc, s, &report.stalls);
        for (side, v, least) in [("kısa", s.width, short), ("uzun", s.length, long)] {
            if let Some(found) = said(
                codes::PARKING_SPACE_SMALL,
                &format!("{} {side} kenarı", whose(s.kind)),
                v,
                Bound::AtLeast,
                Limit {
                    value: least,
                    article,
                },
                None,
                metres,
                "",
                repair,
            ) {
                push(found, vec![s.slot], at(s), fix.clone());
            }
        }
    }
    // A garage's places against its area (4/1-ç).
    for (room, area, inside) in doc.parking_rooms() {
        let fits = (area / kentos_domain::provisions::PARKING_UNIT + 1e-9).floor() as usize;
        if inside.len() <= fits {
            continue;
        }
        let (name, label) = match doc.get(room) {
            Some(Entity::Space(s)) => (
                s.space
                    .name
                    .clone()
                    .filter(|n| !n.trim().is_empty())
                    .unwrap_or_else(|| "Otopark".into()),
                kentos_geometry_core::geom::space::label_at(
                    &kentos_domain::arch_values::core_areas(&s.space.parts),
                )
                .unwrap_or(V::new(0.0, 0.0)),
            ),
            _ => ("Otopark".into(), V::new(0.0, 0.0)),
        };
        let mut slots = vec![room];
        slots.extend(inside.iter().copied());
        push(
            Said {
                code: codes::PARKING_AREA_SHORT,
                message: format!(
                    "{name}: {} otopark yeri, {} ancak {fits} yere yeter (araç başına manevra dahil 20 m²) — {}",
                    inside.len(),
                    crate::regulation_audit::square(area),
                    AREA_ARTICLE.short()
                ),
                repair: "Otopark mahalini büyütün ya da yer sayısını azaltın: binek başına manevra alanı dahil 20 m² \
                         gerekir."
                    .into(),
                rule: AREA_ARTICLE.named(),
                binding: None,
            },
            slots,
            label,
            None,
        );
    }
    // A public building's places for the disabled (4/1-ı).
    let disabled = report.drawn(StallKind::Disabled);
    if report.disabled > disabled
        && let Some(first) = report.stalls.first()
    {
        push(
            Said {
                code: codes::PARKING_DISABLED_SHORT,
                message: format!(
                    "Engelli otopark yeri {disabled} < {} ({} yerde her 20'de 1, en az 1; umumi bina) — {}",
                    report.disabled,
                    report.stalls.len(),
                    DISABLED_ARTICLE.short()
                ),
                repair: "Giriş-çıkışa ve asansöre en yakın yerlerden birini engelli yeri yapın (Ölçü › Engelli 350 × \
                         490); engelli yeri 3,50 m geniştir."
                    .into(),
                rule: DISABLED_ARTICLE.named(),
                binding: None,
            },
            vec![first.slot],
            at(first),
            None,
        );
    }
    // The electric cars' places (5/1-j).
    let charging = report.drawn(StallKind::Charging);
    if report.charging > charging {
        let car = report.stalls.iter().find(|s| s.kind == StallKind::Car);
        let place = car.or(report.stalls.first());
        if let Some(p) = place {
            push(
                Said {
                    code: codes::PARKING_CHARGING_SHORT,
                    message: format!(
                        "Elektrikli araç şarj yeri {charging} < {} ({} zorunlu yerin %5'i, en az 1) — {}",
                        report.charging,
                        report.required,
                        CHARGING_ARTICLE.short()
                    ),
                    repair: "Yerlerden birinin Tür'ünü Şarjlı yapın: şarj ünitesi dahil elektrikli araca uygun \
                             düzenlenir."
                        .into(),
                    rule: CHARGING_ARTICLE.named(),
                    binding: None,
                },
                vec![p.slot],
                at(p),
                car.map(|_| Fix {
                    label: "Bu yeri şarjlı yap".into(),
                    operation: json!({ STALL_KIND: "Şarjlı" }),
                    command: Some(PARAMETERS),
                }),
            );
        }
    }
}
