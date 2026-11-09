//! Mevzuat denetimi (docs/adr/3033, 3034; ADV-23, M33–M34): Planlı Alanlar İmar Yönetmeliği's rules measured on the
//! drawing, each finding saying what it measured, the limit, the article and the gazette it stands in.
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
//! - **Merdivenler (31, docs/adr/3034):** a common stair's flight and landings at least 1.20 m in a dwelling building,
//!   1.50 m in another; a stair inside a dwelling unit 1.00 m. A stair is a unit's when the room it stands in is; outside
//!   every room of its level it is the building's outdoor steps, whose flight 31/1-a does not measure. The riser at most
//!   0.16 m, 0.18 m where the building has an elevator; 2a+b 0.60–0.64 m, the tread at least 0.27 m, a curved stair's
//!   at least 0.10 m at its narrow end. A common stair has railings on both sides (31/1-ç), any other of more than one
//!   step at least one (38/1).
//! - **Rampalar (30/12, 38/1):** the slope by the rise: 8 % up to 0.15 m, 7 % to 0.50 m, 6 % to 1.00 m, 5 % above;
//!   railings. A ramp 2.75 m wide or wider is a vehicle ramp (Otopark Yönetmeliği), not read.
//! - **Pencereler (38/1):** a window above the ground storey whose sill is below 0.90 m asks for a 1.10 m railing.
//! - **Işıklık ve hava bacası (32/2, 32/3):** a void named Işıklık (Aydınlık) at least 1.50 m and 4.50 m² in a building
//!   of one to six storeys, 2.00 m and 9.00 m² in a taller one; a void named Hava bacası 0.60 × 0.60 m.
//! - **Asansör (34, as amended 1.7.2026):** a building of three storeys (its basements holding a unit, a common area or
//!   an annex counted) leaves an elevator's place, one of four or more has one; a single dwelling unit's is not asked.
//!   An elevator is a space named Asansör; its net size at least the cabin's (narrow side 1.20 m, 1.80 m²), the landing
//!   its door opens to at least 1.20 m before a sliding door, 1.50 m before a hinged one.
//!
//! **Yerel yönetmelik** (`ArchSettings::local_rules`): the municipality's values are read over these where they differ,
//! as PAİY 69/3 applies them. Sivas İmar Yönetmeliği (10.11.2022): common stairs 1.30 m and their intermediate landings
//! 1.30 m (31/1-a); an elevator from three storeys, a single dwelling of three storeys or fewer aside (34/1); storeys at
//! most 3.60 m in a dwelling area (28/1-c, no 4.00 m); an air shaft's narrow side 0.30 m and 0.36 m² (32/3); toilet and
//! balcony doors 0.80 m (39/1-c); a light well by the storeys it serves (32/2-a). A stricter value binds and the other
//! layer's is named beside it; a looser one a drawing keeps while missing the national value is a conflict
//! (`regulation_conflict`) the administration decides.
//!
//! The limits are the regulation's own where a plan says nothing more; a finding is the drawing's measure, not an
//! approval, and a plan's notes come first.

use std::collections::{BTreeMap, BTreeSet, HashMap};

use kentos_commands::{
    ArchAuditFinding, ArchLevel, Entity, LocalRules, OpeningForm, OpeningOperation, ProgramUnit,
    Railings, SpaceFields, UnitKind, codes,
};
use kentos_domain::space_follow::SpaceSearch;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::geom::arch;
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region::inside_area;
use serde_json::json;

/// The rule set's name: the regulation and its last amendment read (docs/adr/3033, 3034).
pub const RULE_SET: &str = "PAİY (3.7.2017/30113; son değişiklik 1.7.2026/33297)";

/// Sivas İmar Yönetmeliği's gazette (docs/adr/3034).
pub const SIVAS_GAZETTE: &str = "10.11.2022/32009";

/// The regulation's own gazette, for an article never amended.
const ORIGINAL: &str = "3.7.2017/30113";

/// Whose rule an article is (docs/adr/3034).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    /// Planlı Alanlar İmar Yönetmeliği.
    Paiy,
    /// Sivas Belediyesi İmar Yönetmeliği.
    Sivas,
}

impl Source {
    /// Its name ending a message: “PAİY”, “Sivas”.
    pub fn short(self) -> &'static str {
        match self {
            Source::Paiy => "PAİY",
            Source::Sivas => "Sivas",
        }
    }

    /// Its name in a finding's rule: “PAİY”, “Sivas İmar Yön.”.
    pub fn named(self) -> &'static str {
        match self {
            Source::Paiy => "PAİY",
            Source::Sivas => "Sivas İmar Yön.",
        }
    }
}

/// An article of a regulation and the gazette it stands in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Article {
    pub source: Source,
    pub article: &'static str,
    pub gazette: &'static str,
}

impl Article {
    /// Planlı Alanlar İmar Yönetmeliği's article.
    pub const fn paiy(article: &'static str, gazette: &'static str) -> Self {
        Self {
            source: Source::Paiy,
            article,
            gazette,
        }
    }

    /// Sivas İmar Yönetmeliği's article.
    pub const fn sivas(article: &'static str) -> Self {
        Self {
            source: Source::Sivas,
            article,
            gazette: SIVAS_GAZETTE,
        }
    }

    /// As a finding's `rule` names it: “PAİY 29/1 · RG 3.7.2017/30113”, “Sivas İmar Yön. 31/1-a · RG 10.11.2022/32009”.
    pub fn named(self) -> String {
        format!(
            "{} {} · RG {}",
            self.source.named(),
            self.article,
            self.gazette
        )
    }

    /// As a finding's message ends: “PAİY 29/1”; an amended article with its amendment's date, “PAİY 39/1-c
    /// (13.8.2025)”: the rule a drawing of last year kept may be a new one.
    pub fn short(self) -> String {
        match self.gazette.strip_suffix(" değişikliğiyle") {
            Some(g) => format!(
                "{} {} ({})",
                self.source.short(),
                self.article,
                g.split('/').next().unwrap_or(g)
            ),
            None => format!("{} {}", self.source.short(), self.article),
        }
    }
}

pub const ROOMS_ARTICLE: Article = Article::paiy("29/1", ORIGINAL);
pub const CORRIDOR_ARTICLE: Article = Article::paiy("29/3", ORIGINAL);
pub const STOREY_ARTICLE: Article = Article::paiy("28/1-c", "12.8.2023/32277 değişikliğiyle");
pub const DOOR_HEIGHT_ARTICLE: Article = Article::paiy("39/1-a", "13.8.2025/32985 değişikliğiyle");
pub const DOOR_ENTRANCE_ARTICLE: Article =
    Article::paiy("39/1-b", "13.8.2025/32985 değişikliğiyle");
pub const DOOR_WIDTH_ARTICLE: Article = Article::paiy("39/1-c", "13.8.2025/32985 değişikliğiyle");
pub const RAMP_ARTICLE: Article = Article::paiy("30/12", ORIGINAL);
pub const STAIR_WIDTH_ARTICLE: Article = Article::paiy("31/1-a", ORIGINAL);
pub const STAIR_RAILING_ARTICLE: Article = Article::paiy("31/1-ç", ORIGINAL);
pub const RISER_ARTICLE: Article = Article::paiy("31/2-a", ORIGINAL);
pub const STEP_ARTICLE: Article = Article::paiy("31/2-b", ORIGINAL);
pub const WINDER_ARTICLE: Article = Article::paiy("31/2-c", ORIGINAL);
pub const LIGHTWELL_ARTICLE: Article = Article::paiy("32/2-a", ORIGINAL);
pub const TALL_LIGHTWELL_ARTICLE: Article = Article::paiy("32/2-b", ORIGINAL);
pub const AIR_SHAFT_ARTICLE: Article = Article::paiy("32/3", ORIGINAL);
pub const ELEVATOR_ARTICLE: Article = Article::paiy("34/1", "1.7.2026/33297 değişikliğiyle");
pub const CABIN_ARTICLE: Article = Article::paiy("34/2", ORIGINAL);
pub const RAILING_ARTICLE: Article = Article::paiy("38/1", ORIGINAL);
pub const SIVAS_STOREY_ARTICLE: Article = Article::sivas("28/1-c");
pub const SIVAS_STAIR_ARTICLE: Article = Article::sivas("31/1-a");
pub const SIVAS_LIGHTWELL_ARTICLE: Article = Article::sivas("32/2-a");
pub const SIVAS_AIR_SHAFT_ARTICLE: Article = Article::sivas("32/3");
pub const SIVAS_ELEVATOR_ARTICLE: Article = Article::sivas("34/1");
pub const SIVAS_DOOR_ARTICLE: Article = Article::sivas("39/1-c");

/// The codes the group finds, in their order.
pub const CODES: [&str; 20] = [
    codes::PAIY_ROOM_MISSING,
    codes::PAIY_ROOM_AREA_BELOW,
    codes::PAIY_ROOM_WIDTH_BELOW,
    codes::PAIY_CORRIDOR_NARROW,
    codes::PAIY_STOREY_HEIGHT_OVER,
    codes::PAIY_DOOR_HEIGHT_BELOW,
    codes::PAIY_DOOR_WIDTH_BELOW,
    codes::PAIY_STAIR_WIDTH_NARROW,
    codes::PAIY_STAIR_LANDING_SHORT,
    codes::PAIY_STAIR_RISER_HIGH,
    codes::PAIY_STAIR_STEP,
    codes::PAIY_RAILING_MISSING,
    codes::PAIY_RAMP_STEEP,
    codes::PAIY_WINDOW_RAILING,
    codes::PAIY_LIGHTWELL_SMALL,
    codes::PAIY_AIR_SHAFT_SMALL,
    codes::PAIY_ELEVATOR_MISSING,
    codes::PAIY_ELEVATOR_SMALL,
    codes::PAIY_ELEVATOR_LANDING_NARROW,
    codes::REGULATION_CONFLICT,
];

/// A fix a finding offers (docs/adr/3034): `cad.arch.edit`'s operation on its first object (a stair, a ramp) and the
/// words its button says.
#[derive(Clone, Debug, PartialEq)]
pub struct Fix {
    pub label: String,
    pub operation: serde_json::Value,
}

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

/// A void's or a shaft's kind by its name or use (docs/adr/3034).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ShaftKind {
    /// Işıklık, aydınlık (32/2).
    LightWell,
    /// Hava bacası, havalandırma bacası (32/3).
    AirShaft,
    /// Asansör kuyusu (34/2); its lobby (Asansör holü, önü) is not.
    Elevator,
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

/// A name's and a use's words, folded.
fn words(name: Option<&str>, usage: Option<&str>) -> Vec<String> {
    [name, usage]
        .into_iter()
        .flatten()
        .flat_map(|t| {
            fold(t)
                .split(|c: char| !c.is_alphanumeric())
                .filter(|w| !w.is_empty())
                .map(str::to_owned)
                .collect::<Vec<_>>()
        })
        .collect()
}

/// Whether a word begins with `w`.
fn has(words: &[String], w: &str) -> bool {
    words.iter().any(|x| x.starts_with(w))
}

/// Whether a word is `w`.
fn is(words: &[String], w: &str) -> bool {
    words.iter().any(|x| x == w)
}

/// A room's kind by its name or use: the wet rooms first (an Ebeveyn banyosu is a bathroom), then the kitchen, the
/// passages, the living room and the bedrooms; none for another (a store, a study, a balcony …).
pub fn kind_of(name: Option<&str>, usage: Option<&str>) -> Option<RoomKind> {
    let texts: Vec<String> = [name, usage].into_iter().flatten().map(fold).collect();
    let w = words(name, usage);
    let has = |p: &str| has(&w, p);
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
        || texts.iter().any(|t| t == "ODA" || t.starts_with("ODA "))
    {
        Some(RoomKind::Bedroom)
    } else {
        None
    }
}

/// A shaft's kind by its name or use: an elevator's shaft (not its lobby), a light well, an air shaft.
pub fn shaft_of(name: Option<&str>, usage: Option<&str>) -> Option<ShaftKind> {
    let w = words(name, usage);
    let has = |p: &str| has(&w, p);
    if has("ASANSOR")
        && !(has("HOL") || has("ONU") || has("LOBI") || has("SAHANLIK") || has("BEKLEME"))
    {
        Some(ShaftKind::Elevator)
    } else if has("ISIKLIK") || has("AYDINLIK") {
        Some(ShaftKind::LightWell)
    } else if (has("HAVA") && has("BACA")) || has("HAVALANDIRMA") {
        Some(ShaftKind::AirShaft)
    } else {
        None
    }
}

/// A technical room 34/1 does not count a basement by: a boiler room, a heat centre, an electricity room, a water tank,
/// a generator, a climate plant (ısı merkezi, kazan dairesi …).
fn technical(name: Option<&str>, usage: Option<&str>) -> bool {
    let w = words(name, usage);
    [
        "KAZAN",
        "ELEKTRIK",
        "JENERATOR",
        "IKLIMLENDIRME",
        "TEKNIK",
        "HIDROFOR",
        "TRAFO",
        "MAKINE",
        "POMPA",
    ]
    .iter()
    .any(|p| has(&w, p))
        || is(&w, "ISI")
        || (is(&w, "SU") && has(&w, "DEPO"))
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

/// Metres, two places: “1.20 m”.
fn metres(v: f64) -> String {
    format!("{} m", m2(v))
}

/// Square metres, two places: “4.50 m²”.
fn square(v: f64) -> String {
    format!("{} m²", m2(v))
}

/// A riser, three places when the third says something: “0.175 m”, “0.17 m”.
fn riser(v: f64) -> String {
    let t = format!("{v:.3}");
    match t.strip_suffix('0') {
        Some(s) => format!("{s} m"),
        None => format!("{t} m"),
    }
}

/// Which way a limit binds a measure.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Bound {
    AtLeast,
    AtMost,
}

impl Bound {
    fn keeps(self, v: f64, limit: f64) -> bool {
        match self {
            Bound::AtLeast => v + 1e-6 >= limit,
            Bound::AtMost => v <= limit + 1e-6,
        }
    }

    /// Whether `a` asks more than `b`.
    fn stricter(self, a: f64, b: f64) -> bool {
        match self {
            Bound::AtLeast => a > b + 1e-9,
            Bound::AtMost => a < b - 1e-9,
        }
    }

    fn sign(self) -> &'static str {
        match self {
            Bound::AtLeast => "<",
            Bound::AtMost => ">",
        }
    }

    fn words(self) -> &'static str {
        match self {
            Bound::AtLeast => "en az",
            Bound::AtMost => "en çok",
        }
    }
}

/// A limit and the article it stands in.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Limit {
    pub value: f64,
    pub article: Article,
}

/// A measure read against the national limit and the local regulation's (docs/adr/3034).
#[derive(Clone, Copy, Debug, PartialEq)]
enum Verdict {
    Kept,
    /// Missed: the limit that binds (the stricter where both are missed, the local where only it is) and the other
    /// layer's where it differs.
    Broken(Limit, Option<Limit>),
    /// Kept by the local regulation, not by the national: PAİY 69/3 applies the local one, the national asks more, and
    /// the administration decides.
    Conflict {
        local: Limit,
        national: Limit,
    },
}

fn judge(v: f64, bound: Bound, national: Limit, local: Option<Limit>) -> Verdict {
    let keeps = |l: &Limit| bound.keeps(v, l.value);
    let Some(local) = local else {
        return if keeps(&national) {
            Verdict::Kept
        } else {
            Verdict::Broken(national, None)
        };
    };
    match (keeps(&national), keeps(&local)) {
        (true, true) => Verdict::Kept,
        (true, false) => Verdict::Broken(local, Some(national)),
        (false, true) => Verdict::Conflict { local, national },
        (false, false) if (local.value - national.value).abs() < 1e-9 => {
            Verdict::Broken(national, None)
        }
        (false, false) if bound.stricter(local.value, national.value) => {
            Verdict::Broken(local, Some(national))
        }
        (false, false) => Verdict::Broken(national, Some(local)),
    }
}

/// A measure's finding as its words say it.
struct Said {
    code: &'static str,
    message: String,
    repair: String,
    rule: String,
    /// The limit that binds; none for a conflict.
    binding: Option<Limit>,
}

/// One measure read (docs/adr/3034): `subject` and the measure, its limit, `qual` after them and the article; the other
/// layer's limit beside it where it differs (“… < 1.30 m — Sivas 31/1-a; PAİY 31/1-a: 1.20 m”); a conflict under
/// `regulation_conflict` with both and who decides.
#[allow(clippy::too_many_arguments)]
fn said(
    code: &'static str,
    subject: &str,
    v: f64,
    bound: Bound,
    national: Limit,
    local: Option<Limit>,
    unit: fn(f64) -> String,
    qual: &str,
    repair: &str,
) -> Option<Said> {
    match judge(v, bound, national, local) {
        Verdict::Kept => None,
        Verdict::Broken(limit, other) => Some(Said {
            code,
            message: format!(
                "{subject} {} {} {}{qual} — {}{}",
                unit(v),
                bound.sign(),
                unit(limit.value),
                limit.article.short(),
                other.map_or_else(String::new, |o| format!(
                    "; {}: {}",
                    o.article.short(),
                    unit(o.value)
                ))
            ),
            repair: repair.to_owned(),
            rule: limit.article.named(),
            binding: Some(limit),
        }),
        Verdict::Conflict { local, national } => Some(Said {
            code: codes::REGULATION_CONFLICT,
            message: format!(
                "{subject} {}{qual}: {} {} {} ister, uyuyor; {} {} {} ister, uymuyor — çelişkide idare karar verir",
                unit(v),
                local.article.short(),
                bound.words(),
                unit(local.value),
                national.article.short(),
                bound.words(),
                unit(national.value)
            ),
            repair: format!(
                "İdareye sorun (PAİY 69/3: plan aksini demiyorsa yerel yönetmelik uygulanır); {} değerine ({}) uymak \
                 çelişkiyi ortadan kaldırır.",
                national.article.short(),
                unit(national.value)
            ),
            rule: format!("{} ↔ {}", local.article.named(), national.article.named()),
            binding: None,
        }),
    }
}

/// One room as the rules read it.
struct Room {
    slot: Slot,
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

/// A space as the rules read it: its level, its program group, its outline and what it is beyond a room.
struct Place {
    slot: Slot,
    level: Option<String>,
    unit: Option<String>,
    areas: Vec<Area>,
    void: bool,
    outdoor: bool,
    shaft: Option<ShaftKind>,
    balcony: bool,
    toilet: bool,
    technical: bool,
    /// A shaft's net narrow side, net area and label point.
    measure: Option<(f64, Option<f64>, V)>,
}

impl Place {
    fn holds(&self, level: &Option<String>, p: V) -> bool {
        &self.level == level && self.areas.iter().any(|a| inside_area(a, p))
    }
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

/// The storeys 34/1 counts (as amended 1.7.2026): the levels at or above ±0.00 and each basement holding a room that is
/// not a technical one; the basements among them apart.
fn storeys(levels: &[&ArchLevel], places: &[Place]) -> (usize, usize) {
    let above = levels.iter().filter(|l| l.elevation >= -1e-6).count();
    let basements = levels
        .iter()
        .filter(|l| {
            l.elevation < -1e-6
                && places.iter().any(|p| {
                    p.level.as_deref() == Some(l.id.as_str())
                        && !p.void
                        && !p.outdoor
                        && !p.technical
                })
        })
        .count();
    (above + basements, basements)
}

/// Where a stair stands (31/1-a): in a common area (or nowhere known), inside a dwelling unit, inside another unit, or
/// outside every room of its level.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StairIn {
    Common,
    Unit,
    Other,
    Outside,
}

/// The regulation group's findings (docs/adr/3033, 3034), each made by `finding(code, message, repair, slots, at, rule,
/// fix)`.
pub fn findings(
    doc: &Document,
    finding: impl Fn(&str, String, &str, Vec<Slot>, V, String, Option<Fix>) -> ArchAuditFinding,
) -> Vec<ArchAuditFinding> {
    let program = doc.program();
    let sivas = doc.settings().arch.as_ref().and_then(|a| a.local_rules) == Some(LocalRules::Sivas);
    let local = |value: f64, article: Article| sivas.then_some(Limit { value, article });
    let mut search = SpaceSearch::new(doc);
    let mut rooms: Vec<Room> = Vec::new();
    let mut places: Vec<Place> = Vec::new();
    for e in doc.entities() {
        let Entity::Space(s) = e else { continue };
        let x = &s.space;
        let slot = Slot(s.base.id);
        let outdoor = x
            .usage
            .as_deref()
            .and_then(|u| program.brief(u))
            .is_some_and(|b| b.outdoor);
        let (name, usage) = (x.name.as_deref(), x.usage.as_deref());
        let kind = kind_of(name, usage);
        let shaft = shaft_of(name, usage);
        let measure = (shaft.is_some() && x.broken.is_none()).then(|| {
            let parts = net_parts(&mut search, x);
            let (width, pole) = crate::program_check::width_of(&parts);
            let at = pole
                .or_else(|| {
                    kentos_geometry_core::geom::space::label_at(
                        &kentos_domain::arch_values::core_areas(&x.parts),
                    )
                })
                .unwrap_or(V::new(0.0, 0.0));
            (width, search.net_area(x), at)
        });
        places.push(Place {
            slot,
            level: e.own_level().map(str::to_owned),
            unit: x.unit.clone(),
            areas: kentos_domain::arch_values::core_areas(&x.parts),
            void: x.void,
            outdoor,
            shaft,
            balcony: has(&words(name, usage), "BALKON"),
            toilet: !x.void && kind == Some(RoomKind::Toilet),
            technical: technical(name, usage),
            measure,
        });
        if x.void || outdoor || x.broken.is_some() || shaft.is_some() {
            continue;
        }
        let Some(kind) = kind else {
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
    let mut push = |s: Said, slots: Vec<Slot>, at: V, fix: Option<Fix>| {
        out.push(finding(
            s.code, s.message, &s.repair, slots, at, s.rule, fix,
        ));
    };
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
            if let Some(s) = said(
                codes::PAIY_CORRIDOR_NARROW,
                &format!("{} net eni", named(r)),
                r.width,
                Bound::AtLeast,
                Limit {
                    value: least_width,
                    article: CORRIDOR_ARTICLE,
                },
                None,
                metres,
                "",
                "Holü ya da koridoru en az 1,20 m genişletin.",
            ) {
                push(s, vec![r.slot], r.at, None);
            }
            continue;
        }
        if let (Some(a), Some(least)) = (r.area, least_area)
            && let Some(s) = said(
                codes::PAIY_ROOM_AREA_BELOW,
                &format!("{} net alanı", named(r)),
                a,
                Bound::AtLeast,
                Limit {
                    value: least,
                    article: ROOMS_ARTICLE,
                },
                None,
                square,
                "",
                "Piyesi büyütün ya da kullanımını değiştirin.",
            )
        {
            push(s, vec![r.slot], r.at, None);
        }
        if r.width > 0.0
            && let Some(s) = said(
                codes::PAIY_ROOM_WIDTH_BELOW,
                &format!("{} dar kenarı", named(r)),
                r.width,
                Bound::AtLeast,
                Limit {
                    value: least_width,
                    article: ROOMS_ARTICLE,
                },
                None,
                metres,
                "",
                "Piyesin dar kenarını genişletin.",
            )
        {
            push(s, vec![r.slot], r.at, None);
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
        let has_rooms = places
            .iter()
            .any(|p| p.unit.as_deref() == Some(u.id.as_str()));
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
            let slots: Vec<Slot> = places
                .iter()
                .filter(|p| p.unit.as_deref() == Some(u.id.as_str()))
                .map(|p| p.slot)
                .collect();
            let at = mine.first().map_or(V::new(0.0, 0.0), |r| r.at);
            push(
                Said {
                    code: codes::PAIY_ROOM_MISSING,
                    message: format!(
                        "“{}” bölümünde {} yok — {}",
                        u.number,
                        missing
                            .iter()
                            .map(|m| m.to_lowercase())
                            .collect::<Vec<_>>()
                            .join(", "),
                        ROOMS_ARTICLE.short()
                    ),
                    repair: "Eksik piyesi çizin ya da mahalin adını veya kullanımını düzeltin."
                        .into(),
                    rule: ROOMS_ARTICLE.named(),
                    binding: None,
                },
                slots,
                at,
                None,
            );
        }
    }
    let mut levels: Vec<&ArchLevel> = doc
        .settings()
        .arch
        .as_ref()
        .map(|a| a.levels.iter().collect())
        .unwrap_or_default();
    levels.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
    // Storey heights (28/1-c) in a dwelling area, when the project is plainly a dwelling's; Sivas's 3.60 m without the
    // national 4.00 m of a low building.
    let dwelling_project = program.units.iter().any(|u| {
        u.kind == UnitKind::Unit && u.nature.as_deref().is_some_and(|n| dwelling(Some(n)))
    }) || rooms.iter().any(|r| r.kind == RoomKind::Bedroom);
    if dwelling_project {
        let above = levels.iter().filter(|l| l.elevation >= -1e-6).count();
        let most = if above <= 3 { 4.00 } else { 3.60 };
        for (i, l) in levels.iter().enumerate() {
            let next = levels
                .get(i + 1)
                .map(|n| n.elevation - l.elevation)
                .filter(|h| *h > 0.0);
            let Some(h) = l.height.or(next) else { continue };
            if let Some(s) = said(
                codes::PAIY_STOREY_HEIGHT_OVER,
                &format!("“{}” kat yüksekliği", l.name),
                h,
                Bound::AtMost,
                Limit {
                    value: most,
                    article: STOREY_ARTICLE,
                },
                local(3.60, SIVAS_STOREY_ARTICLE),
                metres,
                &format!(" (konut bölgesi, {above} kat)"),
                "Kat yüksekliğini düşürün (Proje ayarları › Mimari › Katlar) ya da planın kendi notuna bakın.",
            ) {
                let at = rooms
                    .iter()
                    .find(|r| r.level.as_deref() == Some(l.id.as_str()))
                    .map_or(V::new(0.0, 0.0), |r| r.at);
                push(s, Vec::new(), at, None);
            }
        }
    }
    // The building as 31, 32 and 34 read it.
    let (storeys, basements) = storeys(&levels, &places);
    let has_elevator = places.iter().any(|p| p.shaft == Some(ShaftKind::Elevator));
    let units: Vec<&ProgramUnit> = program
        .units
        .iter()
        .filter(|u| u.kind == UnitKind::Unit)
        .collect();
    let dwelling_building = units.is_empty() || units.iter().any(|u| dwelling(u.nature.as_deref()));
    let single_dwelling = units.len() == 1 && dwelling(units[0].nature.as_deref());
    let place_at =
        |level: &Option<String>, p: V| places.iter().find(|x| !x.void && x.holds(level, p));
    // A unit's owner: an annex's unit, else itself.
    let owner = |id: &str| {
        let u = program.unit(id)?;
        match u.kind {
            UnitKind::Annex => u
                .annex_of
                .as_deref()
                .and_then(|o| program.unit(o))
                .or(Some(u)),
            _ => Some(u),
        }
    };
    // Doors (39/1): their height and their net clear width by what they enter; with Sivas a toilet's or a balcony's
    // door 0.80 m (39/1-c). The landing an elevator's door opens to (34/2).
    let shared = program.units.iter().any(|u| u.kind == UnitKind::Common) || units.len() > 1;
    let kind_of_unit = |unit: &Option<String>| {
        unit.as_deref()
            .and_then(|u| program.unit(u))
            .map(|u| (u.kind, u.annex_of.clone()))
    };
    let mut landing_width: HashMap<Slot, f64> = HashMap::new();
    for e in doc.entities() {
        let Entity::Opening(o) = e else { continue };
        let f = &o.opening;
        let slot = Slot(o.base.id);
        let Some((c, left, right)) = crate::opening_codes::sides(f) else {
            continue;
        };
        let level = crate::opening_types::level_of(doc, f);
        if f.form == OpeningForm::Window {
            // A window above the ground storey whose sill is below 0.90 m (38/1).
            let elevation = level
                .as_deref()
                .and_then(|id| levels.iter().find(|l| l.id == id))
                .map(|l| l.elevation);
            let sill = f.sill.unwrap_or(0.0);
            if elevation.is_some_and(|z| z > 1e-6) && sill + 1e-6 < 0.90 {
                push(
                    Said {
                        code: codes::PAIY_WINDOW_RAILING,
                        message: format!(
                            "Pencerenin denizliği {} < 0.90 m: 1.10 m korkuluk gerekir — {}",
                            metres(sill),
                            RAILING_ARTICLE.short()
                        ),
                        repair: "Denizliği 0,90 m'ye çıkarın ya da pencereye 1,10 m korkuluk çizin; korkuluk çizildiyse \
                                 bulguyu istisna olarak işaretleyin."
                            .into(),
                        rule: RAILING_ARTICLE.named(),
                        binding: None,
                    },
                    vec![slot],
                    c,
                    None,
                );
            }
            continue;
        }
        if f.form != OpeningForm::Door {
            continue;
        }
        // Its height: its head over its sill (none without a head).
        if let Some(h) = f.head.map(|head| head - f.sill.unwrap_or(0.0))
            && h + 1e-6 < 2.10
        {
            push(
                Said {
                    code: codes::PAIY_DOOR_HEIGHT_BELOW,
                    message: format!(
                        "Kapı yüksekliği {} m < 2.10 m — {}",
                        m2(h),
                        DOOR_HEIGHT_ARTICLE.short()
                    ),
                    repair: "Kapının yüksekliğini en az 2,10 m yapın.".into(),
                    rule: DOOR_HEIGHT_ARTICLE.named(),
                    binding: None,
                },
                vec![slot],
                c,
                None,
            );
        }
        let (l, r) = (
            places.iter().find(|p| p.holds(&level, left)),
            places.iter().find(|p| p.holds(&level, right)),
        );
        // The landing before an elevator's door (34/2): 1.20 m before a sliding door, 1.50 m before a hinged one.
        let lift = |p: Option<&Place>| p.is_some_and(|p| p.shaft == Some(ShaftKind::Elevator));
        let before = match (l, r) {
            (Some(a), Some(b)) if lift(Some(a)) && !lift(Some(b)) => Some(b),
            (Some(a), Some(b)) if lift(Some(b)) && !lift(Some(a)) => Some(a),
            _ => None,
        };
        if let Some(b) = before {
            let width = *landing_width
                .entry(b.slot)
                .or_insert_with(|| match doc.get(b.slot) {
                    Some(Entity::Space(s)) => {
                        crate::program_check::width_of(&net_parts(&mut search, &s.space)).0
                    }
                    _ => 0.0,
                });
            let sliding = matches!(
                f.operation,
                OpeningOperation::Slide | OpeningOperation::Pocket
            );
            let (least, how) = if sliding {
                (1.20, " (sürme kapı)")
            } else {
                (1.50, " (dışa açılan kapı)")
            };
            if width > 0.0
                && let Some(s) = said(
                    codes::PAIY_ELEVATOR_LANDING_NARROW,
                    "Asansör kapısının açıldığı sahanlığın net eni",
                    width,
                    Bound::AtLeast,
                    Limit {
                        value: least,
                        article: CABIN_ARTICLE,
                    },
                    None,
                    metres,
                    how,
                    "Asansörün önündeki sahanlığı genişletin ya da sürme kapılı asansör seçin.",
                )
            {
                push(s, vec![b.slot, slot], c, None);
            }
        }
        let Some(clear) = f.sizes().clear else {
            continue;
        };
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
            (Some(a), Some(b)) if entrance(&a.unit, &b.unit) => {
                (1.00, "Bağımsız bölüm giriş kapısının", DOOR_WIDTH_ARTICLE)
            }
            _ => (0.90, "Kapının", DOOR_WIDTH_ARTICLE),
        };
        // Sivas 39/1-c: a toilet's or a balcony's door may be 0.80 m.
        let toilet = [l, r].iter().flatten().any(|p| p.toilet);
        let balcony = [l, r].iter().flatten().any(|p| p.balcony);
        let (what, local_door) = match (sivas && least == 0.90, toilet, balcony) {
            (true, true, _) => ("Tuvalet kapısının", local(0.80, SIVAS_DOOR_ARTICLE)),
            (true, false, true) => ("Balkon kapısının", local(0.80, SIVAS_DOOR_ARTICLE)),
            _ => (what, None),
        };
        if let Some(s) = said(
            codes::PAIY_DOOR_WIDTH_BELOW,
            &format!("{what} net genişliği"),
            clear,
            Bound::AtLeast,
            Limit {
                value: least,
                article,
            },
            local_door,
            metres,
            "",
            "Kapıyı genişletin ya da daha geniş bir kapı tipi seçin.",
        ) {
            push(s, vec![slot], c, None);
        }
    }
    // Stairs (31; 38/1).
    let mut first_stair: Option<V> = None;
    for e in doc.entities() {
        let Entity::Stair(st) = e else { continue };
        let x = &st.stair;
        let slot = Slot(st.base.id);
        let shape = kentos_formats::arch_elements::core_stair(x);
        let Shape::Stair { pts, stair } = &shape else {
            continue;
        };
        let walk = arch::Walk::curved(pts, stair.bulge);
        let at = pts.first().copied().unwrap_or(V::new(0.0, 0.0));
        first_stair.get_or_insert(at);
        let level = e.own_level().map(str::to_owned);
        let mid = walk.point(walk.length() / 2.0).0;
        let room_on_level = places.iter().any(|p| p.level == level && !p.void);
        let place = match place_at(&level, mid) {
            Some(p) => match p.unit.as_deref().and_then(|u| owner(u)) {
                Some(u) if u.kind == UnitKind::Unit && dwelling(u.nature.as_deref()) => {
                    StairIn::Unit
                }
                Some(u) if u.kind == UnitKind::Unit => StairIn::Other,
                _ => StairIn::Common,
            },
            None if room_on_level => StairIn::Outside,
            None => StairIn::Common,
        };
        // Its flight and landing (31/1-a).
        let limits = match place {
            StairIn::Common if dwelling_building => Some(("Ortak merdivenin", 1.20, 1.30, "")),
            StairIn::Common => Some(("Ortak merdivenin", 1.50, 1.50, " (konut dışı yapı)")),
            StairIn::Unit => Some(("Bağımsız bölüm içindeki merdivenin", 1.00, 1.00, "")),
            StairIn::Other | StairIn::Outside => None,
        };
        if let Some((whose, national, sivas_value, qual)) = limits {
            if let Some(s) = said(
                codes::PAIY_STAIR_WIDTH_NARROW,
                &format!("{whose} kolu"),
                x.width,
                Bound::AtLeast,
                Limit {
                    value: national,
                    article: STAIR_WIDTH_ARTICLE,
                },
                local(sivas_value, SIVAS_STAIR_ARTICLE),
                metres,
                qual,
                "Merdiveni genişletin.",
            ) {
                let fix = s.binding.map(|b| Fix {
                    label: format!("Genişliği {} m yap", m2(b.value)),
                    operation: json!({"kind": "set", "width": b.value}),
                });
                push(s, vec![slot], at, fix);
            }
            if let Some(&(a, b)) = arch::stair_landings(&walk, stair).first()
                && let Some(s) = said(
                    codes::PAIY_STAIR_LANDING_SHORT,
                    &format!("{whose} ara sahanlığı"),
                    stair.landing(),
                    Bound::AtLeast,
                    Limit {
                        value: national,
                        article: STAIR_WIDTH_ARTICLE,
                    },
                    local(sivas_value, SIVAS_STAIR_ARTICLE),
                    metres,
                    qual,
                    "Ara sahanlığı derinleştirin.",
                )
            {
                let fix = s.binding.map(|b| Fix {
                    label: format!("Sahanlığı {} m yap", m2(b.value)),
                    operation: json!({"kind": "set", "landing": b.value}),
                });
                push(s, vec![slot], walk.point((a + b) / 2.0).0, fix);
            }
        }
        // Its steps (31/2).
        let (most, how) = if has_elevator {
            (0.18, " (asansörlü bina)")
        } else {
            (0.16, " (asansörsüz bina)")
        };
        if let Some(s) = said(
            codes::PAIY_STAIR_RISER_HIGH,
            "Merdivenin rıhtı",
            x.riser_height,
            Bound::AtMost,
            Limit {
                value: most,
                article: RISER_ARTICLE,
            },
            None,
            riser,
            how,
            "Rıht sayısını artırın; merdiven uzar.",
        ) {
            push(s, vec![slot], at, None);
        }
        let step = 2.0 * x.riser_height + x.tread;
        if !(0.60 - 1e-6..=0.64 + 1e-6).contains(&step) {
            push(
                Said {
                    code: codes::PAIY_STAIR_STEP,
                    message: format!(
                        "Merdivenin 2a+b'si {}; 0.60–0.64 m olmalı — {}",
                        riser(step),
                        STEP_ARTICLE.short()
                    ),
                    repair: "Rıhtı ya da basamağı değiştirin.".into(),
                    rule: STEP_ARTICLE.named(),
                    binding: None,
                },
                vec![slot],
                at,
                None,
            );
        }
        if let Some(s) = said(
            codes::PAIY_STAIR_STEP,
            "Merdivenin basamağı",
            x.tread,
            Bound::AtLeast,
            Limit {
                value: 0.27,
                article: STEP_ARTICLE,
            },
            None,
            riser,
            "",
            "Basamağı derinleştirin; merdiven uzar.",
        ) {
            push(s, vec![slot], at, None);
        }
        if let Some(inner) = arch::stair_measure(pts, stair).inner_tread
            && let Some(s) = said(
                codes::PAIY_STAIR_STEP,
                "Döner merdivenin basamağı dar uçta",
                inner,
                Bound::AtLeast,
                Limit {
                    value: 0.10,
                    article: WINDER_ARTICLE,
                },
                None,
                riser,
                "",
                "Yarıçapı büyütün ya da merdiveni daraltın.",
            )
        {
            push(s, vec![slot], at, None);
        }
        // Its railings: a common stair's on both sides (31/1-ç), another's of more than one step at least one (38/1).
        let both = Fix {
            label: "İki yana korkuluk koy".into(),
            operation: json!({"kind": "set", "railings": "both"}),
        };
        let common = place == StairIn::Common;
        let railing = if common && x.railings != Some(Railings::Both) {
            let missing = match x.railings {
                Some(Railings::Left) => "Ortak merdivenin sağ yanında korkuluk yok",
                Some(Railings::Right) => "Ortak merdivenin sol yanında korkuluk yok",
                _ => "Ortak merdivenin iki yanında da korkuluk yok",
            };
            Some((
                missing,
                STAIR_RAILING_ARTICLE,
                "İki yanına korkuluk ve küpeşte koyun.",
            ))
        } else if !common && x.risers > 1 && x.railings.is_none() {
            Some((
                "Merdivende korkuluk yok",
                RAILING_ARTICLE,
                "Merdivene korkuluk koyun.",
            ))
        } else {
            None
        };
        if let Some((missing, article, repair)) = railing {
            push(
                Said {
                    code: codes::PAIY_RAILING_MISSING,
                    message: format!("{missing} — {}", article.short()),
                    repair: repair.into(),
                    rule: article.named(),
                    binding: None,
                },
                vec![slot],
                at,
                Some(both),
            );
        }
    }
    // Ramps (30/12, 38/1); a vehicle ramp's is the parking regulation's.
    for e in doc.entities() {
        let Entity::Ramp(r) = e else { continue };
        if r.ramp.width >= 2.75 - 1e-6 {
            continue;
        }
        let slot = Slot(r.base.id);
        let shape = kentos_formats::arch_elements::core_ramp(&r.ramp);
        let Shape::Ramp { pts, ramp } = &shape else {
            continue;
        };
        let walk = arch::Walk::of(pts);
        let run = arch::ramp_run(&walk, ramp);
        let rise = r.ramp.rise.abs();
        let at = pts.first().copied().unwrap_or(V::new(0.0, 0.0));
        if rise <= 1e-9 {
            continue;
        }
        if run > 1e-9 {
            let slope = rise / run * 100.0;
            let most = if rise <= 0.15 + 1e-9 {
                8.0
            } else if rise <= 0.50 + 1e-9 {
                7.0
            } else if rise <= 1.00 + 1e-9 {
                6.0
            } else {
                5.0
            };
            if slope > most + 1e-9 {
                push(
                    Said {
                        code: codes::PAIY_RAMP_STEEP,
                        message: format!(
                            "Rampanın eğimi %{:.2} > %{most} ({} kot farkı) — {}",
                            slope,
                            metres(rise),
                            RAMP_ARTICLE.short()
                        ),
                        repair: format!(
                            "Rampayı uzatın ya da kot farkını azaltın; eğim en çok %{most} olmalı."
                        ),
                        rule: RAMP_ARTICLE.named(),
                        binding: None,
                    },
                    vec![slot],
                    at,
                    None,
                );
            }
        }
        if r.ramp.railings.is_none() {
            push(
                Said {
                    code: codes::PAIY_RAILING_MISSING,
                    message: format!("Rampada korkuluk yok — {}", RAILING_ARTICLE.short()),
                    repair: "Rampaya korkuluk koyun; 30/13 kesintisiz 0,90 ve 0,70 m yükseklikte iki düzey küpeşte ister."
                        .into(),
                    rule: RAILING_ARTICLE.named(),
                    binding: None,
                },
                vec![slot],
                at,
                Some(Fix {
                    label: "İki yana korkuluk koy".into(),
                    operation: json!({"kind": "set", "railings": "both"}),
                }),
            );
        }
    }
    // Light wells and air shafts (32/2, 32/3); Sivas's air shaft 0.30 m and 0.36 m², its light well by the storeys it
    // serves.
    for p in &places {
        let (Some(kind), Some((width, area, at))) = (p.shaft, p.measure) else {
            continue;
        };
        match kind {
            ShaftKind::LightWell => {
                let served = places
                    .iter()
                    .filter(|q| {
                        q.shaft == Some(ShaftKind::LightWell)
                            && q.areas.iter().any(|a| inside_area(a, at))
                    })
                    .map(|q| q.level.clone())
                    .collect::<BTreeSet<_>>()
                    .len()
                    .max(1);
                let (n_width, n_area, n_article) = if storeys >= 7 {
                    (2.00, 9.00, TALL_LIGHTWELL_ARTICLE)
                } else {
                    (1.50, 4.50, LIGHTWELL_ARTICLE)
                };
                let (l_width, l_area) = if served >= 7 {
                    (2.00, 9.00)
                } else {
                    (1.50, 4.50)
                };
                let building = storeys.max(1);
                let qual = if sivas {
                    format!(" ({building} katlı bina, ışıklık {served} katta)")
                } else {
                    format!(" ({building} katlı bina)")
                };
                if let Some(s) = said(
                    codes::PAIY_LIGHTWELL_SMALL,
                    "Işıklığın dar kenarı",
                    width,
                    Bound::AtLeast,
                    Limit {
                        value: n_width,
                        article: n_article,
                    },
                    local(l_width, SIVAS_LIGHTWELL_ARTICLE),
                    metres,
                    &qual,
                    "Işıklığı genişletin.",
                ) {
                    push(s, vec![p.slot], at, None);
                }
                if let Some(a) = area
                    && let Some(s) = said(
                        codes::PAIY_LIGHTWELL_SMALL,
                        "Işıklığın net alanı",
                        a,
                        Bound::AtLeast,
                        Limit {
                            value: n_area,
                            article: n_article,
                        },
                        local(l_area, SIVAS_LIGHTWELL_ARTICLE),
                        square,
                        &qual,
                        "Işıklığı büyütün.",
                    )
                {
                    push(s, vec![p.slot], at, None);
                }
            }
            ShaftKind::AirShaft => {
                if let Some(s) = said(
                    codes::PAIY_AIR_SHAFT_SMALL,
                    "Hava bacasının dar kenarı",
                    width,
                    Bound::AtLeast,
                    Limit {
                        value: 0.60,
                        article: AIR_SHAFT_ARTICLE,
                    },
                    local(0.30, SIVAS_AIR_SHAFT_ARTICLE),
                    metres,
                    "",
                    "Hava bacasını genişletin.",
                ) {
                    push(s, vec![p.slot], at, None);
                }
                if let Some(a) = area
                    && let Some(s) = said(
                        codes::PAIY_AIR_SHAFT_SMALL,
                        "Hava bacasının net alanı",
                        a,
                        Bound::AtLeast,
                        Limit {
                            value: 0.36,
                            article: AIR_SHAFT_ARTICLE,
                        },
                        local(0.36, SIVAS_AIR_SHAFT_ARTICLE),
                        square,
                        "",
                        "Hava bacasını büyütün.",
                    )
                {
                    push(s, vec![p.slot], at, None);
                }
            }
            ShaftKind::Elevator => {}
        }
    }
    // Elevators (34/1, 34/2).
    let counted = if basements > 0 {
        format!("bodrum dahil {storeys} kat")
    } else {
        format!("{storeys} kat")
    };
    if !has_elevator && storeys >= 3 {
        let national = match (single_dwelling, storeys >= 4) {
            (true, _) => None,
            (false, true) => Some(true),
            (false, false) => Some(false),
        };
        let local_needs = sivas && !(single_dwelling && storeys <= 3);
        let at = first_stair
            .or_else(|| rooms.first().map(|r| r.at))
            .unwrap_or(V::new(0.0, 0.0));
        let repair = "Asansör kuyusunu çizin (boşluk mahal, adı “Asansör”). Tek bağımsız bölümlü konutsa Bina programında \
                      tek bölümü tanımlayın: o zaman aranmaz.";
        let missing =
            if local_needs {
                let paiy = match national {
                    Some(true) => "4 ve daha fazla katta asansör",
                    Some(false) => "3 katta asansör yeri",
                    None => "tek bağımsız bölümlü konutta aranmaz",
                };
                Some(Said {
                    code: codes::PAIY_ELEVATOR_MISSING,
                    message: format!(
                        "Binada asansör yok ({counted}) — {}; {}: {paiy}",
                        SIVAS_ELEVATOR_ARTICLE.short(),
                        ELEVATOR_ARTICLE.short()
                    ),
                    repair: repair.into(),
                    rule: SIVAS_ELEVATOR_ARTICLE.named(),
                    binding: None,
                })
            } else {
                national.map(|elevator| Said {
                code: codes::PAIY_ELEVATOR_MISSING,
                message: if elevator {
                    format!("Binada asansör yok ({counted}) — {}", ELEVATOR_ARTICLE.short())
                } else {
                    format!(
                        "Binada asansör yeri yok ({counted}; 3 katta asansör yeri bırakılır) — {}",
                        ELEVATOR_ARTICLE.short()
                    )
                },
                repair: repair.into(),
                rule: ELEVATOR_ARTICLE.named(),
                binding: None,
            })
            };
        if let Some(s) = missing {
            push(s, Vec::new(), at, None);
        }
    }
    // Each level's shafts: one keeps the cabin's size; of several, half (rounded down) do.
    let mut shafts: BTreeMap<Option<String>, Vec<&Place>> = BTreeMap::new();
    for p in places
        .iter()
        .filter(|p| p.shaft == Some(ShaftKind::Elevator))
    {
        shafts.entry(p.level.clone()).or_default().push(p);
    }
    for list in shafts.values() {
        let fits = |p: &Place| {
            p.measure
                .is_some_and(|(w, a, _)| w + 1e-6 >= 1.20 && a.is_none_or(|a| a + 1e-6 >= 1.80))
        };
        let asked = if list.len() == 1 { 1 } else { list.len() / 2 };
        if list.iter().filter(|p| fits(p)).count() >= asked {
            continue;
        }
        for p in list.iter().filter(|p| !fits(p)) {
            let Some((width, area, at)) = p.measure else {
                continue;
            };
            let cabin = Limit {
                value: 1.20,
                article: CABIN_ARTICLE,
            };
            let repair = "Asansör kuyusunu büyütün: kabin en az 1,20 m dar kenarlı ve 1,80 m² olmalı (birden çok \
                          asansörde yarısı).";
            if let Some(s) = said(
                codes::PAIY_ELEVATOR_SMALL,
                "Asansör kuyusunun net dar kenarı",
                width,
                Bound::AtLeast,
                cabin,
                None,
                metres,
                " (kabinin en az dar kenarı)",
                repair,
            ) {
                push(s, vec![p.slot], at, None);
            }
            if let Some(a) = area
                && let Some(s) = said(
                    codes::PAIY_ELEVATOR_SMALL,
                    "Asansör kuyusunun net alanı",
                    a,
                    Bound::AtLeast,
                    Limit {
                        value: 1.80,
                        article: CABIN_ARTICLE,
                    },
                    None,
                    square,
                    " (kabinin en az alanı)",
                    repair,
                )
            {
                push(s, vec![p.slot], at, None);
            }
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

    #[test]
    fn shafts_are_known_by_their_names_an_elevator_s_lobby_is_none() {
        assert_eq!(shaft_of(Some("Asansör"), None), Some(ShaftKind::Elevator));
        assert_eq!(
            shaft_of(Some("Asansör kuyusu"), None),
            Some(ShaftKind::Elevator)
        );
        assert_eq!(shaft_of(Some("Asansör holü"), None), None);
        assert_eq!(shaft_of(Some("Asansör önü"), None), None);
        assert_eq!(shaft_of(Some("Işıklık"), None), Some(ShaftKind::LightWell));
        assert_eq!(shaft_of(Some("AYDINLIK"), None), Some(ShaftKind::LightWell));
        assert_eq!(
            shaft_of(Some("Hava bacası"), None),
            Some(ShaftKind::AirShaft)
        );
        assert_eq!(
            shaft_of(None, Some("Havalandırma bacası")),
            Some(ShaftKind::AirShaft)
        );
        assert_eq!(shaft_of(Some("Tesisat şaftı"), None), None);
        assert!(technical(Some("Kazan dairesi"), None));
        assert!(technical(Some("Isı merkezi"), None));
        assert!(technical(Some("Su deposu"), None));
        assert!(
            !technical(Some("Işıklık"), None),
            "a light well is no heat centre"
        );
        assert!(!technical(Some("Sığınak"), None));
    }

    /// The layers (docs/adr/3034): a stricter local value binds with the national beside it; a looser one kept while
    /// the national is missed is a conflict; equal values say the national alone.
    #[test]
    fn a_local_value_binds_when_stricter_and_conflicts_when_looser() {
        let n = |value| Limit {
            value,
            article: STAIR_WIDTH_ARTICLE,
        };
        let l = |value| Limit {
            value,
            article: SIVAS_STAIR_ARTICLE,
        };
        assert_eq!(judge(1.25, Bound::AtLeast, n(1.20), None), Verdict::Kept);
        assert_eq!(
            judge(1.25, Bound::AtLeast, n(1.20), Some(l(1.30))),
            Verdict::Broken(l(1.30), Some(n(1.20)))
        );
        assert_eq!(
            judge(1.10, Bound::AtLeast, n(1.20), Some(l(1.30))),
            Verdict::Broken(l(1.30), Some(n(1.20)))
        );
        assert_eq!(
            judge(0.40, Bound::AtLeast, n(0.60), Some(l(0.30))),
            Verdict::Conflict {
                local: l(0.30),
                national: n(0.60)
            }
        );
        assert_eq!(
            judge(0.20, Bound::AtLeast, n(0.60), Some(l(0.30))),
            Verdict::Broken(n(0.60), Some(l(0.30)))
        );
        assert_eq!(
            judge(0.20, Bound::AtLeast, n(0.36), Some(l(0.36))),
            Verdict::Broken(n(0.36), None)
        );
        assert_eq!(
            judge(3.80, Bound::AtMost, n(4.00), Some(l(3.60))),
            Verdict::Broken(l(3.60), Some(n(4.00)))
        );
        let s = said(
            codes::PAIY_STAIR_WIDTH_NARROW,
            "Ortak merdivenin kolu",
            1.25,
            Bound::AtLeast,
            n(1.20),
            Some(l(1.30)),
            metres,
            "",
            "",
        )
        .expect("said");
        assert_eq!(
            s.message,
            "Ortak merdivenin kolu 1.25 m < 1.30 m — Sivas 31/1-a; PAİY 31/1-a: 1.20 m"
        );
        assert_eq!(s.rule, "Sivas İmar Yön. 31/1-a · RG 10.11.2022/32009");
        assert_eq!(riser(0.175), "0.175 m");
        assert_eq!(riser(0.17), "0.17 m");
    }
}
