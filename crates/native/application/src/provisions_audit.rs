//! Otopark ve sığınak (docs/adr/8036; ADV-22, M36): the parking and the shelter regulations' needs said in Mimari
//! denetim's Mevzuat group beside their tables (`kentos_domain::provisions`): fewer places than Otopark Yönetmeliği
//! Ek-1 asks, a smaller shelter than Sığınak Yönetmeliği 8/a asks, a shelter over 100 m² with one exit (8/c), a shelter
//! lower than 2.40 m inside (8/a).

use kentos_commands::{Entity, codes};
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;

use crate::regulation_audit::{Article, Said, Source, m2};

pub const PARKING_ARTICLE: Article = Article {
    source: Source::Otopark,
    article: "8, Ek-1",
    gazette: "16.6.2022/31868 değişikliğiyle",
};
pub const SHELTER_ARTICLE: Article = Article {
    source: Source::Siginak,
    article: "8/a",
    gazette: "7.11.2025/33070 değişikliğiyle",
};
pub const SHELTER_EXIT_ARTICLE: Article = Article {
    source: Source::Siginak,
    article: "8/c",
    gazette: "7.11.2025/33070 değişikliğiyle",
};

/// The codes the provisions find, in their order.
pub const CODES: [&str; 4] = [
    codes::PARKING_SHORT,
    codes::SHELTER_SHORT,
    codes::SHELTER_EXIT_COUNT,
    codes::SHELTER_HEIGHT_LOW,
];

/// A room's name as a finding says it, and its label point.
fn room(doc: &Document, slot: Slot) -> (String, V) {
    match doc.get(slot) {
        Some(Entity::Space(s)) => {
            let name = s
                .space
                .name
                .clone()
                .filter(|n| !n.trim().is_empty())
                .unwrap_or_else(|| "Mahal".into());
            let at = kentos_geometry_core::geom::space::label_at(
                &kentos_domain::arch_values::core_areas(&s.space.parts),
            )
            .unwrap_or(V::new(0.0, 0.0));
            (name, at)
        }
        _ => ("Mahal".into(), V::new(0.0, 0.0)),
    }
}

/// The provisions' findings (docs/adr/8036), each said through `push(said, slots, at)`.
pub(crate) fn findings(doc: &Document, push: &mut dyn FnMut(Said, Vec<Slot>, V)) {
    let parking = doc.parking_report();
    if parking.required > 0 && parking.short() {
        let at = parking
            .places
            .first()
            .map_or(V::new(0.0, 0.0), |s| room(doc, *s).1);
        push(
            Said {
                code: codes::PARKING_SHORT,
                message: format!(
                    "Otopark: gereken {} yer, çizimde {} ({} m² ÷ 20 m²) — {}",
                    parking.required,
                    parking.available,
                    m2(parking.area),
                    PARKING_ARTICLE.short()
                ),
                repair: "Otopark mahalini büyütün (adı Otopark ya da Garaj) ya da ihtiyacı Otopark Yönetmeliği 4/1-e, f'ye \
                         göre başka yerde karşılayın; ayrıntı Metraj ▾ › Otopark hesabı'nda."
                    .into(),
                rule: PARKING_ARTICLE.named(),
                binding: None,
            },
            parking.places.clone(),
            at,
        );
    }
    let shelter = doc.shelter_report();
    if shelter.short() {
        let at = shelter
            .shelters
            .first()
            .map_or(V::new(0.0, 0.0), |(s, _)| room(doc, *s).1);
        push(
            Said {
                code: codes::SHELTER_SHORT,
                message: format!(
                    "Sığınak: {} kişi için en az {} m² net, çizimde {} m² — {}",
                    shelter.persons,
                    m2(shelter.area_needed),
                    m2(shelter.area),
                    SHELTER_ARTICLE.short()
                ),
                repair: "Sığınak mahalini büyütün ya da yeni bir sığınak çizin (adı Sığınak); ayrıntı Metraj ▾ › Sığınak \
                         hesabı'nda."
                    .into(),
                rule: SHELTER_ARTICLE.named(),
                binding: None,
            },
            shelter.shelters.iter().map(|(s, _)| *s).collect(),
            at,
        );
    }
    if shelter.shelters.is_empty() {
        return;
    }
    let escape = kentos_domain::escape::escape(doc);
    for (slot, area) in &shelter.shelters {
        let (name, at) = room(doc, *slot);
        // Over 100 m² two exits (8/c).
        let exits = escape
            .rooms
            .iter()
            .find(|e| e.room == *slot)
            .map_or(0, |e| e.passages);
        if *area > 100.0 + 1e-6 && exits < 2 {
            push(
                Said {
                    code: codes::SHELTER_EXIT_COUNT,
                    message: format!(
                        "{name} {} m² > 100 m²: 2 çıkış gerekir, {exits} var — {}",
                        m2(*area),
                        SHELTER_EXIT_ARTICLE.short()
                    ),
                    repair:
                        "Sığınağa ikinci bir çıkış açın (demir kapılı, en az bir dik açı dönüşlü)."
                            .into(),
                    rule: SHELTER_EXIT_ARTICLE.named(),
                    binding: None,
                },
                vec![*slot],
                at,
            );
        }
        // Its inside height: its finished floor to its ceiling (docs/adr/8040 §3); without one, its storey's floor to
        // floor (more than the net inside height).
        let Some(Entity::Space(s)) = doc.get(*slot) else {
            continue;
        };
        if let Some(h) = doc.room_height(&s.space).map(|h| h.inside())
            && h + 1e-6 < 2.40
        {
            push(
                Said {
                    code: codes::SHELTER_HEIGHT_LOW,
                    message: format!(
                        "{name} iç yüksekliği {} m < 2.40 m — {}",
                        m2(h),
                        SHELTER_ARTICLE.short()
                    ),
                    repair: "Sığınağın net iç yüksekliğini en az 2,40 m yapın.".into(),
                    rule: SHELTER_ARTICLE.named(),
                    binding: None,
                },
                vec![*slot],
                at,
            );
        }
    }
}
