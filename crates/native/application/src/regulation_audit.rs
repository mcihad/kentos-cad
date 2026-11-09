//! Mevzuat denetimi (docs/adr/3033; ADV-23, M33): Planlı Alanlar İmar Yönetmeliği's rules measured on the drawing, each
//! finding saying what it measured, the limit, the article and the gazette it stands in.
//!
//! - **Piyesler (29/1):** each dwelling has a living room (narrow side 3.00 m, net 12.0 m²), a bedroom (2.50 m, 9.00
//!   m²), a kitchen (1.50 m, 3.30 m²), a bathroom (1.50 m, 3.00 m²) and a toilet (1.00 m, 1.20 m²). A room is known by
//!   its name or use (Salon, Yatak odası, Mutfak, Banyo, WC …); its net area is its inner faces' (whatever method it is
//!   measured by), its narrow side the diameter of the largest circle inside it (the program check's net width). A
//!   dwelling is a building program's independent unit whose nature is a dwelling's (Daire, Mesken, Konut) or not given;
//!   it lacking one of the five is said (29/1), a bathroom standing for the toilet in one of three rooms or fewer (29/2).
//!   Without a program the rooms are measured, no dwelling counted.
//! - **Hol ve koridor (29/3):** net width at least 1.20 m.
//! - **Kat yüksekliği (28/1-c):** in a dwelling area at most 3.60 m floor to floor, 4.00 m in a building of three
//!   storeys or fewer above its basements; read only when the project is plainly a dwelling's (a dwelling unit or a
//!   bedroom).
//! - **Kapılar (39/1, 13.8.2025):** at least 2.10 m high; their net clear width at least 1.50 m at a building's entrance,
//!   1.00 m at an independent unit's, 0.90 m elsewhere. A door with no room on one side is an entrance: the building's
//!   when it has a common area or more than one unit, else its only unit's; a door between a unit's room and a common
//!   area is that unit's entrance.
//!
//! The limits are the regulation's own where a plan says nothing more; a finding is the drawing's measure, not an
//! approval, and a plan's notes come first.

use kentos_commands::{ArchAuditFinding, Entity, OpeningForm, SpaceFields, UnitKind, codes};
use kentos_domain::Document;
use kentos_domain::space_follow::SpaceSearch;
use kentos_geometry_core::Vec2 as V;

/// The rule set's name: the regulation and its last amendment read (docs/adr/3033).
pub const RULE_SET: &str = "PAİY (3.7.2017/30113; son değişiklik 14.1.2026/33137)";

/// An article of the regulation and the gazette it stands in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Article {
    pub article: &'static str,
    pub gazette: &'static str,
}

impl Article {
    /// As a finding's `rule` names it: “PAİY 29/1 · RG 3.7.2017/30113”.
    pub fn named(self) -> String {
        format!("PAİY {} · RG {}", self.article, self.gazette)
    }

    /// As a finding's message ends: “PAİY 29/1”; an amended article with its amendment's date, “PAİY 39/1-c
    /// (13.8.2025)”: the rule a drawing of last year kept may be a new one.
    pub fn short(self) -> String {
        match self.gazette.strip_suffix(" değişikliğiyle") {
            Some(g) => format!(
                "PAİY {} ({})",
                self.article,
                g.split('/').next().unwrap_or(g)
            ),
            None => format!("PAİY {}", self.article),
        }
    }
}

pub const ROOMS_ARTICLE: Article = Article {
    article: "29/1",
    gazette: "3.7.2017/30113",
};
pub const CORRIDOR_ARTICLE: Article = Article {
    article: "29/3",
    gazette: "3.7.2017/30113",
};
pub const STOREY_ARTICLE: Article = Article {
    article: "28/1-c",
    gazette: "12.8.2023/32277 değişikliğiyle",
};
pub const DOOR_HEIGHT_ARTICLE: Article = Article {
    article: "39/1-a",
    gazette: "13.8.2025/32985 değişikliğiyle",
};
pub const DOOR_ENTRANCE_ARTICLE: Article = Article {
    article: "39/1-b",
    gazette: "13.8.2025/32985 değişikliğiyle",
};
pub const DOOR_WIDTH_ARTICLE: Article = Article {
    article: "39/1-c",
    gazette: "13.8.2025/32985 değişikliğiyle",
};

/// The codes the group finds, in their order.
pub const CODES: [&str; 7] = [
    codes::PAIY_ROOM_MISSING,
    codes::PAIY_ROOM_AREA_BELOW,
    codes::PAIY_ROOM_WIDTH_BELOW,
    codes::PAIY_CORRIDOR_NARROW,
    codes::PAIY_STOREY_HEIGHT_OVER,
    codes::PAIY_DOOR_HEIGHT_BELOW,
    codes::PAIY_DOOR_WIDTH_BELOW,
];

/// A room's kind as the regulation names it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RoomKind {
    Living,
    Bedroom,
    Kitchen,
    Bath,
    Toilet,
    Corridor,
}

impl RoomKind {
    /// Its name in a finding.
    pub fn label(self) -> &'static str {
        match self {
            RoomKind::Living => "Oturma odası",
            RoomKind::Bedroom => "Yatak odası",
            RoomKind::Kitchen => "Mutfak",
            RoomKind::Bath => "Banyo",
            RoomKind::Toilet => "Tuvalet",
            RoomKind::Corridor => "Hol",
        }
    }

    /// 29/1's narrow side and net area, m and m²; a corridor's 29/3 width and no area.
    pub fn limits(self) -> (f64, Option<f64>) {
        match self {
            RoomKind::Living => (3.00, Some(12.0)),
            RoomKind::Bedroom => (2.50, Some(9.00)),
            RoomKind::Kitchen => (1.50, Some(3.30)),
            RoomKind::Bath => (1.50, Some(3.00)),
            RoomKind::Toilet => (1.00, Some(1.20)),
            RoomKind::Corridor => (1.20, None),
        }
    }
}

/// Upper-case ASCII, Turkish letters folded (İ, ı → I; Ş → S …).
fn fold(t: &str) -> String {
    t.chars()
        .map(|c| match c {
            'ç' | 'Ç' => 'C',
            'ğ' | 'Ğ' => 'G',
            'ı' | 'I' | 'İ' | 'i' => 'I',
            'ö' | 'Ö' => 'O',
            'ş' | 'Ş' => 'S',
            'ü' | 'Ü' => 'U',
            c => c.to_ascii_uppercase(),
        })
        .collect()
}

/// A room's kind by its name or use: the wet rooms first (an Ebeveyn banyosu is a bathroom), then the kitchen, the
/// passages, the living room and the bedrooms; none for another (a store, a study, a balcony …).
pub fn kind_of(name: Option<&str>, usage: Option<&str>) -> Option<RoomKind> {
    let words: Vec<String> = [name, usage].into_iter().flatten().map(fold).collect();
    let has = |w: &str| {
        words.iter().any(|t| {
            t.split(|c: char| !c.is_alphanumeric())
                .any(|x| x.starts_with(w))
        })
    };
    if has("WC") || has("TUVALET") {
        Some(RoomKind::Toilet)
    } else if has("BANYO") || has("DUS") || has("YIKANMA") {
        Some(RoomKind::Bath)
    } else if has("MUTFAK") {
        Some(RoomKind::Kitchen)
    } else if has("HOL") || has("KORIDOR") || has("ANTRE") {
        Some(RoomKind::Corridor)
    } else if has("SALON") || has("OTURMA") {
        Some(RoomKind::Living)
    } else if has("YATAK")
        || has("EBEVEYN")
        || has("COCUK")
        || words.iter().any(|t| t == "ODA" || t.starts_with("ODA "))
    {
        Some(RoomKind::Bedroom)
    } else {
        None
    }
}

/// Whether a unit's nature is a dwelling's (or not given).
fn dwelling(nature: Option<&str>) -> bool {
    nature.is_none_or(|n| {
        let n = fold(n);
        ["DAIRE", "MESKEN", "KONUT", "EV"]
            .iter()
            .any(|w| n.contains(w))
    })
}

fn m2(v: f64) -> String {
    format!("{v:.2}")
}

/// One room as the rules read it.
struct Room {
    slot: kentos_domain::Slot,
    kind: RoomKind,
    /// Its own name (“Yatak odası 1”), else none: the kind's label says it.
    name: Option<String>,
    unit: Option<String>,
    level: Option<String>,
    number: Option<String>,
    area: Option<f64>,
    width: f64,
    at: V,
}

/// A room's net outline: its inner faces found again from its seed, else its own.
fn net_parts(search: &mut SpaceSearch<'_>, s: &SpaceFields) -> Vec<kentos_commands::SpacePart> {
    if s.method != kentos_commands::SpaceMethod::Net && s.seed.is_some() {
        let mut net = s.clone();
        net.method = kentos_commands::SpaceMethod::Net;
        let found = search.found_preserving_legacy(&net);
        if found.broken.is_none() && !found.parts.is_empty() {
            return found.parts;
        }
    }
    s.parts.clone()
}

/// The regulation group's findings (docs/adr/3033), each made by `finding(code, message, repair, slots, at, article)`.
pub fn findings(
    doc: &Document,
    finding: impl Fn(&str, String, &str, Vec<kentos_domain::Slot>, V, Article) -> ArchAuditFinding,
) -> Vec<ArchAuditFinding> {
    let program = doc.program();
    let mut search = SpaceSearch::new(doc);
    let mut rooms: Vec<Room> = Vec::new();
    let mut spaces: Vec<(
        kentos_domain::Slot,
        Option<String>,
        Option<String>,
        Vec<kentos_geometry_core::geom::arrangement::Area>,
    )> = Vec::new();
    for e in doc.entities() {
        let Entity::Space(s) = e else { continue };
        let x = &s.space;
        let slot = kentos_domain::Slot(s.base.id);
        let outdoor = x
            .usage
            .as_deref()
            .and_then(|u| program.brief(u))
            .is_some_and(|b| b.outdoor);
        spaces.push((
            slot,
            e.own_level().map(str::to_owned),
            x.unit.clone(),
            kentos_domain::arch_values::core_areas(&x.parts),
        ));
        if x.void || outdoor || x.broken.is_some() {
            continue;
        }
        let Some(kind) = kind_of(x.name.as_deref(), x.usage.as_deref()) else {
            continue;
        };
        // A room of a unit that is not a dwelling (a shop, an office) is not 29/1's.
        let unit = x.unit.clone().filter(|u| program.unit(u).is_some());
        if let Some(u) = unit.as_deref().and_then(|u| program.unit(u))
            && (u.kind == UnitKind::Unit || u.kind == UnitKind::Annex)
            && !dwelling(u.nature.as_deref())
        {
            continue;
        }
        let parts = net_parts(&mut search, x);
        let (width, pole) = crate::program_check::width_of(&parts);
        let area = search.net_area(x);
        let at = pole
            .or_else(|| {
                kentos_geometry_core::geom::space::label_at(
                    &kentos_domain::arch_values::core_areas(&x.parts),
                )
            })
            .unwrap_or(V::new(0.0, 0.0));
        rooms.push(Room {
            slot,
            kind,
            name: x.name.clone().filter(|n| !n.trim().is_empty()),
            unit,
            level: e.own_level().map(str::to_owned),
            number: x.number.clone().filter(|n| !n.trim().is_empty()),
            area,
            width,
            at,
        });
    }
    let mut out = Vec::new();
    // A room by its own name and number, as the drawing writes them; its kind's label without a name.
    let named = |r: &Room| {
        let name = r.name.clone().unwrap_or_else(|| r.kind.label().to_owned());
        match &r.number {
            Some(n) => format!("{name} ({n})"),
            None => name,
        }
    };
    for r in &rooms {
        let (least_width, least_area) = r.kind.limits();
        if r.kind == RoomKind::Corridor {
            if r.width + 1e-6 < least_width {
                out.push(finding(
                    codes::PAIY_CORRIDOR_NARROW,
                    format!(
                        "{} net eni {} m < {} m — {}",
                        named(r),
                        m2(r.width),
                        m2(least_width),
                        CORRIDOR_ARTICLE.short()
                    ),
                    "Holü ya da koridoru en az 1,20 m genişletin.",
                    vec![r.slot],
                    r.at,
                    CORRIDOR_ARTICLE,
                ));
            }
            continue;
        }
        if let (Some(a), Some(least)) = (r.area, least_area)
            && a + 1e-6 < least
        {
            out.push(finding(
                codes::PAIY_ROOM_AREA_BELOW,
                format!(
                    "{} net alanı {} m² < {} m² — {}",
                    named(r),
                    m2(a),
                    m2(least),
                    ROOMS_ARTICLE.short()
                ),
                "Piyesi büyütün ya da kullanımını değiştirin.",
                vec![r.slot],
                r.at,
                ROOMS_ARTICLE,
            ));
        }
        if r.width > 0.0 && r.width + 1e-6 < least_width {
            out.push(finding(
                codes::PAIY_ROOM_WIDTH_BELOW,
                format!(
                    "{} dar kenarı {} m < {} m — {}",
                    named(r),
                    m2(r.width),
                    m2(least_width),
                    ROOMS_ARTICLE.short()
                ),
                "Piyesin dar kenarını genişletin.",
                vec![r.slot],
                r.at,
                ROOMS_ARTICLE,
            ));
        }
    }
    // Each dwelling's rooms (29/1, 29/2): a living room, a bedroom, a kitchen, a bathroom and a toilet; in three rooms or
    // fewer a bathroom stands for the toilet.
    for u in program
        .units
        .iter()
        .filter(|u| u.kind == UnitKind::Unit && dwelling(u.nature.as_deref()))
    {
        let mine: Vec<&Room> = rooms
            .iter()
            .filter(|r| {
                r.unit.as_deref() == Some(u.id.as_str())
                    || r.unit
                        .as_deref()
                        .and_then(|g| program.unit(g))
                        .is_some_and(|g| g.annex_of.as_deref() == Some(u.id.as_str()))
            })
            .collect();
        let has_rooms = spaces
            .iter()
            .any(|(_, _, unit, _)| unit.as_deref() == Some(u.id.as_str()));
        if !has_rooms {
            continue;
        }
        let count = |k: RoomKind| mine.iter().filter(|r| r.kind == k).count();
        let rooms_n = count(RoomKind::Living) + count(RoomKind::Bedroom);
        let mut missing: Vec<&str> = Vec::new();
        for k in [
            RoomKind::Living,
            RoomKind::Bedroom,
            RoomKind::Kitchen,
            RoomKind::Bath,
        ] {
            if count(k) == 0 {
                missing.push(k.label());
            }
        }
        if count(RoomKind::Toilet) == 0 && !(rooms_n <= 3 && count(RoomKind::Bath) > 0) {
            missing.push(RoomKind::Toilet.label());
        }
        if !missing.is_empty() {
            let slots: Vec<kentos_domain::Slot> = spaces
                .iter()
                .filter(|(_, _, unit, _)| unit.as_deref() == Some(u.id.as_str()))
                .map(|(s, ..)| *s)
                .collect();
            let at = mine.first().map_or(V::new(0.0, 0.0), |r| r.at);
            out.push(finding(
                codes::PAIY_ROOM_MISSING,
                format!(
                    "“{}” bölümünde {} yok — {}",
                    u.number,
                    missing
                        .iter()
                        .map(|m| m.to_lowercase())
                        .collect::<Vec<_>>()
                        .join(", "),
                    ROOMS_ARTICLE.short()
                ),
                "Eksik piyesi çizin ya da mahalin adını veya kullanımını düzeltin.",
                slots,
                at,
                ROOMS_ARTICLE,
            ));
        }
    }
    // Storey heights (28/1-c) in a dwelling area, when the project is plainly a dwelling's.
    let dwelling_project = program.units.iter().any(|u| {
        u.kind == UnitKind::Unit && u.nature.as_deref().is_some_and(|n| dwelling(Some(n)))
    }) || rooms.iter().any(|r| r.kind == RoomKind::Bedroom);
    if dwelling_project && let Some(arch) = doc.settings().arch.as_ref() {
        let mut levels: Vec<&kentos_commands::ArchLevel> = arch.levels.iter().collect();
        levels.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
        let above = levels.iter().filter(|l| l.elevation >= -1e-6).count();
        let most = if above <= 3 { 4.00 } else { 3.60 };
        for (i, l) in levels.iter().enumerate() {
            let next = levels
                .get(i + 1)
                .map(|n| n.elevation - l.elevation)
                .filter(|h| *h > 0.0);
            let Some(h) = l.height.or(next) else { continue };
            if h > most + 1e-6 {
                let at = rooms
                    .iter()
                    .find(|r| r.level.as_deref() == Some(l.id.as_str()))
                    .map_or(V::new(0.0, 0.0), |r| r.at);
                out.push(finding(
                    codes::PAIY_STOREY_HEIGHT_OVER,
                    format!(
                        "“{}” kat yüksekliği {} m > {} m (konut bölgesi, {} kat) — {}",
                        l.name,
                        m2(h),
                        m2(most),
                        above,
                        STOREY_ARTICLE.short()
                    ),
                    "Kat yüksekliğini düşürün (Proje ayarları › Mimari › Katlar) ya da planın kendi notuna bakın.",
                    Vec::new(),
                    at,
                    STOREY_ARTICLE,
                ));
            }
        }
    }
    // Doors (39/1): their height and their net clear width by what they enter.
    let shared = program.units.iter().any(|u| u.kind == UnitKind::Common)
        || program
            .units
            .iter()
            .filter(|u| u.kind == UnitKind::Unit)
            .count()
            > 1;
    let room_at = |level: &Option<String>, p: V| {
        spaces.iter().find(|(_, l, _, areas)| {
            l == level
                && areas
                    .iter()
                    .any(|a| kentos_geometry_core::geom::region::inside_area(a, p))
        })
    };
    let kind_of_unit = |unit: &Option<String>| {
        unit.as_deref()
            .and_then(|u| program.unit(u))
            .map(|u| (u.kind, u.annex_of.clone()))
    };
    for e in doc.entities() {
        let Entity::Opening(o) = e else { continue };
        let f = &o.opening;
        if f.form != OpeningForm::Door {
            continue;
        }
        let slot = kentos_domain::Slot(o.base.id);
        let Some((c, left, right)) = crate::opening_codes::sides(f) else {
            continue;
        };
        // Its height: its head over its sill (none without a head).
        if let Some(h) = f.head.map(|head| head - f.sill.unwrap_or(0.0))
            && h + 1e-6 < 2.10
        {
            out.push(finding(
                codes::PAIY_DOOR_HEIGHT_BELOW,
                format!(
                    "Kapı yüksekliği {} m < 2.10 m — {}",
                    m2(h),
                    DOOR_HEIGHT_ARTICLE.short()
                ),
                "Kapının yüksekliğini en az 2,10 m yapın.",
                vec![slot],
                c,
                DOOR_HEIGHT_ARTICLE,
            ));
        }
        let Some(clear) = f.sizes().clear else {
            continue;
        };
        let level = crate::opening_types::level_of(doc, f);
        let (l, r) = (room_at(&level, left), room_at(&level, right));
        // What it enters: the building, a unit or another room.
        let entrance = |a: &Option<String>, b: &Option<String>| {
            let (ka, kb) = (kind_of_unit(a), kind_of_unit(b));
            let unit_side =
                |k: &Option<(UnitKind, Option<String>)>| matches!(k, Some((UnitKind::Unit, _)));
            let common_side =
                |k: &Option<(UnitKind, Option<String>)>| matches!(k, Some((UnitKind::Common, _)));
            (unit_side(&ka) && common_side(&kb)) || (unit_side(&kb) && common_side(&ka))
        };
        let (least, what, article) = match (l, r) {
            (None, None) => continue,
            (Some(_), None) | (None, Some(_)) if shared => {
                (1.50, "Bina giriş kapısının", DOOR_ENTRANCE_ARTICLE)
            }
            (Some(_), None) | (None, Some(_)) => {
                (1.00, "Bağımsız bölüm giriş kapısının", DOOR_WIDTH_ARTICLE)
            }
            (Some(a), Some(b)) if entrance(&a.2, &b.2) => {
                (1.00, "Bağımsız bölüm giriş kapısının", DOOR_WIDTH_ARTICLE)
            }
            _ => (0.90, "Kapının", DOOR_WIDTH_ARTICLE),
        };
        if clear + 1e-6 < least {
            out.push(finding(
                codes::PAIY_DOOR_WIDTH_BELOW,
                format!(
                    "{what} net genişliği {} m < {} m — {}",
                    m2(clear),
                    m2(least),
                    article.short()
                ),
                "Kapıyı genişletin ya da daha geniş bir kapı tipi seçin.",
                vec![slot],
                c,
                article,
            ));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rooms_are_known_by_their_names_wet_rooms_first() {
        assert_eq!(kind_of(Some("Ebeveyn banyosu"), None), Some(RoomKind::Bath));
        assert_eq!(kind_of(Some("WC"), None), Some(RoomKind::Toilet));
        assert_eq!(kind_of(Some("Salon"), None), Some(RoomKind::Living));
        assert_eq!(
            kind_of(Some("Yatak odası 2"), None),
            Some(RoomKind::Bedroom)
        );
        assert_eq!(kind_of(Some("Oda 1"), None), Some(RoomKind::Bedroom));
        assert_eq!(kind_of(Some("Çocuk odası"), None), Some(RoomKind::Bedroom));
        assert_eq!(kind_of(Some("Giriş holü"), None), Some(RoomKind::Corridor));
        assert_eq!(kind_of(Some("MUTFAK"), None), Some(RoomKind::Kitchen));
        assert_eq!(kind_of(Some("Depo"), None), None);
        assert_eq!(
            kind_of(Some("Çalışma odası"), None),
            None,
            "a study is no bedroom"
        );
        assert_eq!(kind_of(None, Some("Duş")), Some(RoomKind::Bath));
    }
}
