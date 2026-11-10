//! Yangın kaçışı (docs/adr/8035; ADV-23, M35): Binaların Yangından Korunması Hakkında Yönetmelik (BYKHY) read on the
//! drawing's escape routes (`kentos_domain::escape`), in Mimari denetim's Mevzuat group.
//!
//! - **Kaçış uzaklığı (32/4, Ek-5/B):** a room that is no dwelling's, from its farthest point (40 cm in, 32/6) to the
//!   nearest storey exit: the one-way part and, with two exits or more, the whole walk against its use's limits.
//! - **Konutlar (48):** inside a dwelling unit, from its farthest point to its door at most 20 m (30 m with a sprinkler
//!   system, 48/2); from the unit's door to the nearest storey exit as Ek-5/B's apartments (48/4). Neither is asked in
//!   a dwelling building of four storeys or fewer with its basements, nor in a single house (48/1). By the structure's
//!   height (all its storeys): above 21.50 m two stairs, one protected; above 30.50 m both protected, one with a fire
//!   lobby or pressurised; above 51.50 m both with fire lobbies and pressurised (48/5).
//! - **Kişi yükü (Ek-5/A) and exits:** a room that is no dwelling's carries its area over its use's square metres a
//!   person (net in the assembly rows 1–4, its axis outline otherwise). A storey's total exit width is its people over
//!   Ek-5/B's people a 50 cm unit width passes, times 0.50 m (33/1): its escape stairs going down (up from a basement),
//!   its doors to the outside on the ground storey; each escape route 1.00 m from 50 people, 1.50 m from 501, 2.00 m
//!   from 2001; a corridor 1.10 m (33/1), every route and stair 1.20 m in a high building (33/2). A room of more than
//!   50 people has two exits, a storey of more than 500 three, of more than 1000 four (39/2).
//! - **Erişim (33/6-b):** a room reaching no exit its storey has, or a dwelling's room not reaching its door.
//!
//! A finding is the drawing's measure, not an approval; protection, fire ratings and lobbies are not drawn and are
//! said, not measured.

use kentos_commands::{Entity, OpeningForm, SpaceFields, SpaceMethod, UnitKind, codes};
use kentos_domain::escape::{ExitRoute, Route};
use kentos_domain::space_follow::SpaceSearch;
use kentos_domain::{Document, Slot};
use kentos_geometry_core::Vec2 as V;
use kentos_geometry_core::geom::region::inside_area;

use crate::regulation_audit::{
    Article, Bound, Limit, Said, Source, has, is, kind_of, metres, said, shaft_of, words,
};

/// The rule set read (docs/adr/8035): the regulation and its last amendment.
pub const RULE_SET: &str = "BYKHY (19.12.2007/26735; son değişiklik 1.7.2025/32943)";

const ORIGINAL: &str = "19.12.2007/26735";
const Y2009: &str = "9.9.2009/27344 değişikliğiyle";
const Y2015: &str = "9.7.2015/29411 değişikliğiyle";

const fn bykhy(article: &'static str, gazette: &'static str) -> Article {
    Article {
        source: Source::Bykhy,
        article,
        gazette,
    }
}

pub const DISTANCE_ARTICLE: Article = bykhy("32/4, Ek-5/B", Y2015);
pub const DEAD_END_ARTICLE: Article = bykhy("4/i, Ek-5/B", Y2015);
pub const DWELLING_ARTICLE: Article = bykhy("48/2", Y2009);
pub const DOOR_ARTICLE: Article = bykhy("48/4, Ek-5/B", Y2015);
pub const ACCESS_ARTICLE: Article = bykhy("33/6-b", ORIGINAL);
pub const WIDTH_ARTICLE: Article = bykhy("33/1", Y2015);
pub const HIGH_ARTICLE: Article = bykhy("33/2", ORIGINAL);
pub const COUNT_ARTICLE: Article = bykhy("39/2", Y2009);
pub const STAIRS_B_ARTICLE: Article = bykhy("48/5-b", Y2009);
pub const STAIRS_C_ARTICLE: Article = bykhy("48/5-c", Y2009);
pub const STAIRS_D_ARTICLE: Article = bykhy("48/5-ç", Y2009);

/// The codes the fire rules find, in their order.
pub const CODES: [&str; 6] = [
    codes::BYKHY_ESCAPE_DISTANCE_OVER,
    codes::BYKHY_ESCAPE_NO_ROUTE,
    codes::BYKHY_EXIT_WIDTH_BELOW,
    codes::BYKHY_EXIT_COUNT_BELOW,
    codes::BYKHY_CORRIDOR_NARROW,
    codes::BYKHY_STAIR_COUNT_BELOW,
];

/// A building's use as Ek-5/B's rows name it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Use {
    HighHazard,
    Industry,
    Dormitory,
    Shop,
    Office,
    Parking,
    School,
    Assembly,
    Hospital,
    Hotel,
    Dwelling,
}

impl Use {
    /// Its row's words in a finding.
    pub fn label(self) -> &'static str {
        match self {
            Use::HighHazard => "yüksek tehlikeli yer",
            Use::Industry => "endüstri yapısı",
            Use::Dormitory => "yurt",
            Use::Shop => "mağaza",
            Use::Office => "büro",
            Use::Parking => "otopark ve depo",
            Use::School => "okul",
            Use::Assembly => "toplanma amaçlı bina",
            Use::Hospital => "hastane",
            Use::Hotel => "otel",
            Use::Dwelling => "apartman",
        }
    }

    /// Ek-5/B: the farthest one-way and two-way walks, m, without a sprinkler system and with one.
    pub fn walks(self) -> [[f64; 2]; 2] {
        match self {
            Use::HighHazard => [[10.0, 20.0], [20.0, 35.0]],
            Use::Industry => [[15.0, 25.0], [30.0, 60.0]],
            Use::Dormitory | Use::Office | Use::School => [[15.0, 30.0], [45.0, 75.0]],
            Use::Shop | Use::Parking | Use::Assembly => [[15.0, 25.0], [45.0, 60.0]],
            Use::Hospital => [[15.0, 25.0], [30.0, 45.0]],
            Use::Hotel => [[15.0, 20.0], [30.0, 45.0]],
            Use::Dwelling => [[15.0, 30.0], [30.0, 75.0]],
        }
    }

    /// Ek-5/B: a dead-end corridor's farthest walk (çıkmaz koridor, 4/i), m, without a sprinkler system and with one.
    pub fn dead_end(self) -> [f64; 2] {
        match self {
            Use::HighHazard => [10.0, 20.0],
            _ => [15.0, 20.0],
        }
    }

    /// Ek-5/B: the people a 50 cm unit width passes: a door to the outside, other doors, escape stairs, ramps and
    /// corridors.
    pub fn per_unit(self) -> [f64; 4] {
        match self {
            Use::HighHazard | Use::Dormitory | Use::Hotel | Use::Dwelling => {
                [50.0, 40.0, 30.0, 50.0]
            }
            Use::Hospital => [30.0, 30.0, 30.0, 30.0],
            _ => [100.0, 80.0, 60.0, 100.0],
        }
    }

    /// The square metres a person of a room nothing names more plainly (Ek-5/A's row of the use's own rooms).
    fn default_load(self) -> f64 {
        match self {
            Use::Shop => 5.0,
            Use::Assembly => 1.5,
            Use::Hospital | Use::Hotel => 20.0,
            Use::Parking => 30.0,
            _ => 10.0,
        }
    }

    /// The stricter of two uses: the shorter two-way walk, then the shorter one-way walk (31/5).
    fn stricter(self, other: Use) -> Use {
        let key = |u: Use| (u.walks()[1][0], u.walks()[0][0], u.per_unit()[2]);
        if key(other) < key(self) { other } else { self }
    }
}

/// A use by its words (a unit's nature, a room's name or use); none for words that name no use.
pub fn use_of(name: Option<&str>, usage: Option<&str>) -> Option<Use> {
    let w = words(name, usage);
    let any = |list: &[&str]| list.iter().any(|p| has(&w, p));
    if any(&["DAIRE", "MESKEN", "KONUT", "APARTMAN"]) || is(&w, "EV") {
        Some(Use::Dwelling)
    } else if any(&[
        "HASTANE",
        "HUZUREVI",
        "KLINIK",
        "POLIKLINIK",
        "BAKIMEVI",
        "SAGLIK",
    ]) {
        Some(Use::Hospital)
    } else if any(&["OTEL", "MOTEL", "PANSIYON"]) {
        Some(Use::Hotel)
    } else if any(&["YURT", "YATAKHANE"]) {
        Some(Use::Dormitory)
    } else if any(&["OKUL", "EGITIM", "DERSLIK", "SINIF", "KRES", "ANAOKUL"]) {
        Some(Use::School)
    } else if any(&["DUKKAN", "MAGAZA", "MARKET", "SATIS"]) {
        Some(Use::Shop)
    } else if any(&["OTOPARK", "GARAJ", "DEPO", "AMBAR"]) {
        Some(Use::Parking)
    } else if any(&["FABRIKA", "IMALAT", "ATOLYE", "URETIM", "SANAYI"]) {
        Some(Use::Industry)
    } else if any(&[
        "LOKANTA",
        "RESTORAN",
        "KAFE",
        "SINEMA",
        "TIYATRO",
        "KONFERANS",
        "DUGUN",
        "CAMI",
        "MESCIT",
        "KONSER",
    ]) {
        Some(Use::Assembly)
    } else if any(&["BURO", "OFIS", "IDARI", "DERNEK"]) {
        Some(Use::Office)
    } else {
        None
    }
}

/// Ek-5/A: a room's square metres a person and whether its net area is read (rows 1–4); none for a room it does not
/// count (a passage, a stair, a wet room, a shaft: 31/6).
pub fn load_of(name: Option<&str>, usage: Option<&str>) -> Option<(f64, bool)> {
    let w = words(name, usage);
    let any = |list: &[&str]| list.iter().any(|p| has(&w, p));
    if kind_of(name, usage).is_some_and(|k| {
        matches!(
            k,
            crate::regulation_audit::RoomKind::Corridor
                | crate::regulation_audit::RoomKind::Toilet
                | crate::regulation_audit::RoomKind::Bath
        )
    }) || shaft_of(name, usage).is_some()
        || any(&["MERDIVEN", "SOYUNMA", "LAVABO"])
    {
        return None;
    }
    let row = if any(&["DANS", "BAR", "GECE"]) {
        (1.0, true)
    } else if any(&[
        "KONFERANS",
        "LOKANTA",
        "RESTORAN",
        "KANTIN",
        "KONSER",
        "SINEMA",
        "TIYATRO",
        "DUGUN",
        "TOPLANTI",
        "SERGI",
        "STUDYO",
    ]) || (has(&w, "BEKLEME") && has(&w, "SALON"))
    {
        (1.5, true)
    } else if any(&["TERMINAL", "YOLCU"]) {
        (3.0, true)
    } else if any(&["DERSLIK", "SINIF", "BILGISAYAR", "SEMINER"]) {
        (1.5, false)
    } else if any(&["RESEPSIYON", "BEKLEME", "ATRIUM", "LOBI", "FUAYE"]) || has(&w, "SPOR") {
        (3.0, false)
    } else if any(&[
        "MARKET", "MAGAZA", "DUKKAN", "SATIS", "GALERI", "MUZE", "ATOLYE", "FITNES", "AEROBIK",
    ]) || (has(&w, "OKUMA") && has(&w, "SALON"))
        || has(&w, "MUAYENE")
    {
        (5.0, false)
    } else if any(&["HASTA", "HEMSIRE", "LABORATUVAR", "ECZANE"])
        || (has(&w, "OTEL") && has(&w, "ODA"))
    {
        (20.0, false)
    } else if any(&["DEPO", "AMBAR", "MAKINE", "KAZAN", "OTOPARK", "GARAJ"]) {
        (30.0, false)
    } else if any(&[
        "OFIS",
        "BURO",
        "DERNEK",
        "KUTUPHANE",
        "PAKETLEME",
        "FABRIKA",
        "URETIM",
        "MUTFAK",
        "CAMASIR",
    ]) || (has(&w, "OGRENCI") && has(&w, "YATAK"))
    {
        (10.0, false)
    } else {
        return Some((0.0, false));
    };
    Some(row)
}

/// A room as the fire rules read it.
struct Room {
    slot: Slot,
    level: Option<String>,
    name: String,
    unit: Option<String>,
    dwelling: bool,
    corridor: bool,
    /// Its use and its people (Ek-5/A); none for a dwelling's room.
    use_: Option<Use>,
    people: f64,
    width: f64,
    at: V,
    areas: Vec<kentos_geometry_core::geom::arrangement::Area>,
}

/// A room's area by its outline at `method`: found again from its seed, else its own when it is measured so.
fn area_by(search: &mut SpaceSearch<'_>, s: &SpaceFields, method: SpaceMethod) -> Option<f64> {
    if s.method == method || s.seed.is_none() {
        return Some(kentos_geometry_core::geom::space::area(
            &kentos_domain::arch_values::core_areas(&s.parts),
        ));
    }
    let mut other = s.clone();
    other.method = method;
    let found = search.found_preserving_legacy(&other);
    found.broken.is_none().then(|| {
        kentos_geometry_core::geom::space::area(&kentos_domain::arch_values::core_areas(
            &found.parts,
        ))
    })
}

fn level_height(levels: &[&kentos_commands::ArchLevel], i: usize) -> f64 {
    let next = levels
        .get(i + 1)
        .map(|n| n.elevation - levels[i].elevation)
        .filter(|h| *h > 0.0);
    levels[i].height.or(next).unwrap_or(3.0)
}

/// A walk the fire rules measured (docs/adr/8035 §2): its points from the start, its length, the measure read against
/// its limit (the whole walk, its one-way part or a corridor's dead end) and whether it breaks it.
#[derive(Clone, Debug, PartialEq)]
pub struct Walk {
    pub level: Option<String>,
    pub path: Vec<V>,
    pub length: f64,
    pub measure: f64,
    /// The limit, m; none where the regulation does not ask it (a small dwelling building, 48/1).
    pub limit: Option<f64>,
    /// What the measure is: “tek yön”, “tek yönlü kısım”, “iki yön”, “çıkmaz koridor”, “daire içi”.
    pub words: &'static str,
    pub over: bool,
}

/// The fire rules' findings (docs/adr/8035), each said through `push(said, slots, at)`.
pub(crate) fn findings(doc: &Document, push: &mut dyn FnMut(Said, Vec<Slot>, V)) {
    assess(doc, push, &mut Vec::new());
}

/// The walks the fire rules measure, every storey's (docs/adr/8035 §2).
pub fn walks(doc: &Document) -> Vec<Walk> {
    let mut out = Vec::new();
    assess(doc, &mut |_, _, _| {}, &mut out);
    out
}

fn assess(doc: &Document, push: &mut dyn FnMut(Said, Vec<Slot>, V), walks: &mut Vec<Walk>) {
    let program = doc.program();
    let arch = doc.settings().arch.clone().unwrap_or_default();
    let sprinklers = arch.sprinklers;
    let col = usize::from(sprinklers);
    let wet = if sprinklers { ", yağmurlamalı" } else { "" };
    let mut levels: Vec<&kentos_commands::ArchLevel> = arch.levels.iter().collect();
    levels.sort_by(|a, b| a.elevation.total_cmp(&b.elevation));
    let mut search = SpaceSearch::new(doc);
    // The units' uses; the building's strictest.
    let units: Vec<&kentos_commands::ProgramUnit> = program
        .units
        .iter()
        .filter(|u| u.kind == UnitKind::Unit)
        .collect();
    let dwellings = units
        .iter()
        .filter(|u| kentos_domain::escape::dwelling_nature(u.nature.as_deref()))
        .count();
    let unit_use = |id: &str| {
        let u = program.unit(id)?;
        let owner = match u.kind {
            UnitKind::Annex => u
                .annex_of
                .as_deref()
                .and_then(|o| program.unit(o))
                .unwrap_or(u),
            _ => u,
        };
        if owner.kind != UnitKind::Unit {
            return None;
        }
        Some(
            if kentos_domain::escape::dwelling_nature(owner.nature.as_deref()) {
                Use::Dwelling
            } else {
                use_of(owner.nature.as_deref(), None).unwrap_or(Use::Office)
            },
        )
    };
    let building_use = units
        .iter()
        .filter_map(|u| unit_use(&u.id))
        .filter(|u| *u != Use::Dwelling)
        .reduce(Use::stricter);
    // The rooms.
    let mut rooms: Vec<Room> = Vec::new();
    for e in doc.entities() {
        let Entity::Space(s) = e else { continue };
        let x = &s.space;
        let outdoor = x
            .usage
            .as_deref()
            .and_then(|u| program.brief(u))
            .is_some_and(|b| b.outdoor);
        if x.void || outdoor || x.broken.is_some() {
            continue;
        }
        let (name, usage) = (x.name.as_deref(), x.usage.as_deref());
        let of_unit = x.unit.as_deref().and_then(unit_use);
        let dwelling = of_unit == Some(Use::Dwelling);
        let use_ = if dwelling {
            None
        } else {
            of_unit
                .or_else(|| use_of(name, usage).filter(|u| *u != Use::Dwelling))
                .or(building_use)
        };
        let parts = crate::regulation_audit::net_parts(&mut search, x);
        let (width, pole) = crate::program_check::width_of(&parts);
        let areas = kentos_domain::arch_values::core_areas(&x.parts);
        let at = pole
            .or_else(|| kentos_geometry_core::geom::space::label_at(&areas))
            .unwrap_or(V::new(0.0, 0.0));
        let people = match (use_, load_of(name, usage)) {
            (Some(u), Some((factor, net))) => {
                let factor = if factor > 0.0 {
                    factor
                } else {
                    u.default_load()
                };
                let area = if net {
                    search.net_area(x)
                } else {
                    area_by(&mut search, x, SpaceMethod::Centerline)
                };
                area.map_or(0.0, |a| (a / factor).ceil())
            }
            _ => 0.0,
        };
        let name_text = x
            .name
            .clone()
            .filter(|n| !n.trim().is_empty())
            .or_else(|| x.usage.clone())
            .unwrap_or_else(|| "Mahal".into());
        rooms.push(Room {
            slot: Slot(s.base.id),
            level: e.own_level().map(str::to_owned),
            name: match &x.number {
                Some(n) if !n.trim().is_empty() => format!("{name_text} ({n})"),
                _ => name_text,
            },
            unit: x.unit.clone(),
            dwelling,
            corridor: kind_of(name, usage) == Some(crate::regulation_audit::RoomKind::Corridor),
            use_,
            people,
            width,
            at,
            areas,
        });
    }
    // A dwelling building: a dwelling unit, or bedrooms without a program (docs/adr/8033).
    let dwelling_building = dwellings > 0
        || (units.is_empty()
            && doc.entities().any(|e| {
                matches!(e, Entity::Space(s)
                    if kind_of(s.space.name.as_deref(), s.space.usage.as_deref())
                        == Some(crate::regulation_audit::RoomKind::Bedroom))
            }));
    // The storeys: the structure's height (all of them, 48/5, 4/bbb) and the building's (those on ±0.00 and above).
    let structure: f64 = (0..levels.len()).map(|i| level_height(&levels, i)).sum();
    let building: f64 = (0..levels.len())
        .filter(|&i| levels[i].elevation >= -1e-6)
        .map(|i| level_height(&levels, i))
        .sum();
    let high = building > 21.50 + 1e-9 || structure > 30.50 + 1e-9;
    let storeys = levels
        .iter()
        .filter(|l| {
            l.elevation >= -1e-6
                || rooms
                    .iter()
                    .any(|r| r.level.as_deref() == Some(l.id.as_str()))
        })
        .count()
        .max(1);
    // 48/1: a dwelling building of four storeys or fewer, or a single house, asks no distance.
    let small_dwelling = dwelling_building && (storeys <= 4 || dwellings == 1);
    let escape = kentos_domain::escape::escape(doc);
    let room_of = |slot: Slot| rooms.iter().find(|r| r.slot == slot);
    let m = metres;
    // A walk against Ek-5/B: the measure, the limit, their words and whether it is broken — the whole walk with one
    // exit, else its one-way part, then the whole against two ways.
    let judge = |e: &ExitRoute, u: Use| -> (f64, f64, &'static str, bool) {
        let [one, two] = u.walks();
        let (one, two) = (one[col], two[col]);
        if e.exits <= 1 {
            (e.route.length, one, "tek yön", e.route.length > one + 1e-6)
        } else if e.one_way > one + 1e-6 {
            (e.one_way, one, "tek yönlü kısım", true)
        } else {
            (e.route.length, two, "iki yön", e.route.length > two + 1e-6)
        }
    };
    // A walk's finding.
    let walk = |who: &str, to: &str, e: &ExitRoute, u: Use, article: Article| -> Option<Said> {
        let (measure, most, words, over) = judge(e, u);
        if !over {
            return None;
        }
        let (message, repair) = match words {
            "tek yönlü kısım" => (
                format!(
                    "{who} {to} yolun tek yönlü kısmı {} > {} ({}{wet}) — {}",
                    m(measure),
                    m(most),
                    u.label(),
                    article.short()
                ),
                "İki çıkışa giden yollar daha erken ayrılmalı: kapıyı ya da koridoru yeniden düzenleyin.",
            ),
            "tek yön" => (
                format!(
                    "{who} {to} {} > {} ({}, tek yön{wet}) — {}",
                    m(measure),
                    m(most),
                    u.label(),
                    article.short()
                ),
                "İkinci bir kaçış yolu (merdiven ya da çıkış) açın ya da yolu kısaltın.",
            ),
            _ => (
                format!(
                    "{who} {to} {} > {} ({}, iki yön{wet}) — {}",
                    m(measure),
                    m(most),
                    u.label(),
                    article.short()
                ),
                "Çıkışı yaklaştırın ya da yeni bir kaçış merdiveni ekleyin.",
            ),
        };
        Some(Said {
            code: codes::BYKHY_ESCAPE_DISTANCE_OVER,
            message,
            repair: repair.into(),
            rule: article.named(),
            binding: None,
        })
    };
    let start = |r: &Route| r.path.first().copied().unwrap_or(V::new(0.0, 0.0));
    // Kaçış uzaklıkları.
    let mut worst: std::collections::BTreeMap<(String, Option<String>), (f64, Slot, Route)> =
        std::collections::BTreeMap::new();
    for e in &escape.rooms {
        let Some(room) = room_of(e.room) else {
            continue;
        };
        if room.dwelling {
            match &e.entrance {
                Some(r) => {
                    let key = (room.unit.clone().unwrap_or_default(), e.level.clone());
                    if worst.get(&key).is_none_or(|w| r.length > w.0) {
                        worst.insert(key, (r.length, room.slot, r.clone()));
                    }
                }
                None if e.has_entrance && !small_dwelling => push(
                    Said {
                        code: codes::BYKHY_ESCAPE_NO_ROUTE,
                        message: format!(
                            "{} daire kapısına ulaşmıyor — {}",
                            room.name,
                            DWELLING_ARTICLE.short()
                        ),
                        repair: "Mahale kapı ya da geçiş açın.".into(),
                        rule: DWELLING_ARTICLE.named(),
                        binding: None,
                    },
                    vec![room.slot],
                    room.at,
                ),
                None => {}
            }
            continue;
        }
        if dwelling_building && room.unit.is_none() || room.use_.is_none() {
            continue;
        }
        let u = room.use_.unwrap_or(Use::Office);
        match &e.exit {
            // A corridor's own walk is a dead end's (4/i): from its farthest point to an exit or to where two ways
            // part.
            Some(x) if room.corridor => {
                let most = u.dead_end()[col];
                let over = x.one_way > most + 1e-6;
                walks.push(Walk {
                    level: e.level.clone(),
                    path: x.route.path.clone(),
                    length: x.route.length,
                    measure: x.one_way,
                    limit: Some(most),
                    words: "çıkmaz koridor",
                    over,
                });
                if over {
                    push(
                        Said {
                            code: codes::BYKHY_ESCAPE_DISTANCE_OVER,
                            message: format!(
                                "{} çıkmaz koridor uzunluğu {} > {} ({}{wet}) — {}",
                                room.name,
                                m(x.one_way),
                                m(most),
                                u.label(),
                                DEAD_END_ARTICLE.short()
                            ),
                            repair: "Koridorun ucuna bir çıkış ya da ikinci bir kaçış merdiveni koyun.".into(),
                            rule: DEAD_END_ARTICLE.named(),
                            binding: None,
                        },
                        vec![room.slot],
                        start(&x.route),
                    );
                }
            }
            Some(x) => {
                let (measure, most, words, over) = judge(x, u);
                walks.push(Walk {
                    level: e.level.clone(),
                    path: x.route.path.clone(),
                    length: x.route.length,
                    measure,
                    limit: Some(most),
                    words,
                    over,
                });
                if let Some(s) = walk(
                    &format!("{} en uzak noktasından", room.name),
                    "kat çıkışına",
                    x,
                    u,
                    DISTANCE_ARTICLE,
                ) {
                    push(s, vec![room.slot], start(&x.route));
                }
            }
            None if e.storey_exits => push(
                Said {
                    code: codes::BYKHY_ESCAPE_NO_ROUTE,
                    message: format!("{} kat çıkışına ulaşmıyor — {}", room.name, ACCESS_ARTICLE.short()),
                    repair: "Mahalden ortak koridora ya da kaçış merdivenine bir kapı açın; başka bir bölümün \
                             mahallerinden geçilerek kaçılmaz."
                        .into(),
                    rule: ACCESS_ARTICLE.named(),
                    binding: None,
                },
                vec![room.slot],
                room.at,
            ),
            None => {}
        }
    }
    let limit = 20.0 + 10.0 * col as f64;
    for ((unit, level), (length, slot, route)) in &worst {
        let over = !small_dwelling && *length > limit + 1e-6;
        walks.push(Walk {
            level: level.clone(),
            path: route.path.clone(),
            length: *length,
            measure: *length,
            limit: (!small_dwelling).then_some(limit),
            words: "daire içi",
            over,
        });
        if over {
            let number = program
                .unit(unit)
                .map_or(unit.as_str(), |u| u.number.as_str());
            push(
                Said {
                    code: codes::BYKHY_ESCAPE_DISTANCE_OVER,
                    message: format!(
                        "“{number}” bölümünde en uzak noktadan daire kapısına {} > {}{} — {}",
                        m(*length),
                        m(limit),
                        if sprinklers { " (yağmurlamalı)" } else { "" },
                        DWELLING_ARTICLE.short()
                    ),
                    repair: "Daire kapısını yaklaştırın ya da daireye ikinci bir çıkış açın."
                        .into(),
                    rule: DWELLING_ARTICLE.named(),
                    binding: None,
                },
                vec![*slot],
                start(route),
            );
        }
    }
    let mut said_units = std::collections::BTreeSet::new();
    for d in &escape.doors {
        let Some(x) = &d.exit else { continue };
        let (measure, most, words, over) = judge(x, Use::Dwelling);
        walks.push(Walk {
            level: d.level.clone(),
            path: x.route.path.clone(),
            length: x.route.length,
            measure,
            limit: (!small_dwelling).then_some(most),
            words,
            over: over && !small_dwelling,
        });
        if small_dwelling {
            continue;
        }
        let number = program
            .unit(&d.unit)
            .map_or(d.unit.as_str(), |u| u.number.as_str());
        if let Some(s) = walk(
            &format!("“{number}” bölümünün kapısından"),
            "kat çıkışına",
            x,
            Use::Dwelling,
            DOOR_ARTICLE,
        ) && said_units.insert((d.unit.clone(), d.level.clone()))
        {
            push(s, vec![d.door], d.at);
        }
    }
    // Kişi yükü, çıkış genişliği ve sayısı (33/1, 39/2).
    let room_holds = |level: &Option<String>, p: V| {
        rooms
            .iter()
            .find(|r| &r.level == level && r.areas.iter().any(|a| inside_area(a, p)))
    };
    for r in rooms.iter().filter(|r| r.people > 50.0 && !r.dwelling) {
        let count = escape
            .rooms
            .iter()
            .find(|e| e.room == r.slot)
            .map_or(0, |e| e.passages);
        if count < 2 {
            push(
                Said {
                    code: codes::BYKHY_EXIT_COUNT_BELOW,
                    message: format!(
                        "{} kişi yükü {}: en az 2 çıkış gerekir, {count} var — {}",
                        r.name,
                        r.people,
                        COUNT_ARTICLE.short()
                    ),
                    repair: "Mahale birbirinden uzak ikinci bir kapı açın.".into(),
                    rule: COUNT_ARTICLE.named(),
                    binding: None,
                },
                vec![r.slot],
                r.at,
            );
        }
    }
    let storey_ids: Vec<Option<String>> = if levels.is_empty() {
        vec![None]
    } else {
        levels.iter().map(|l| Some(l.id.clone())).collect()
    };
    for level in &storey_ids {
        let on: Vec<&Room> = rooms
            .iter()
            .filter(|r| &r.level == level && !r.dwelling)
            .collect();
        let people: f64 = on.iter().map(|r| r.people).sum();
        if people <= 0.0 {
            continue;
        }
        let u = on
            .iter()
            .filter_map(|r| r.use_)
            .reduce(Use::stricter)
            .unwrap_or(Use::Office);
        let i = level
            .as_deref()
            .and_then(|id| levels.iter().position(|l| l.id == id));
        let z = i.map_or(0.0, |i| levels[i].elevation);
        let name = i.map_or("Kat".to_owned(), |i| levels[i].name.clone());
        let at = on.first().map_or(V::new(0.0, 0.0), |r| r.at);
        // Its exits: the doors to the outside on the ground storey, else its escape stairs.
        let mut widths: Vec<(f64, Slot)> = Vec::new();
        let ground = z.abs() <= 0.01 || levels.is_empty();
        if ground {
            for e in doc.entities() {
                let Entity::Opening(o) = e else { continue };
                let f = &o.opening;
                if !matches!(f.form, OpeningForm::Door | OpeningForm::Opening) {
                    continue;
                }
                if crate::opening_types::level_of(doc, f) != *level {
                    continue;
                }
                let Some((_, a, b)) = crate::opening_codes::sides(f) else {
                    continue;
                };
                if room_holds(level, a).is_some() != room_holds(level, b).is_some() {
                    widths.push((f.sizes().clear.unwrap_or(f.width), Slot(o.base.id)));
                }
            }
        } else {
            let above = z > 0.01;
            for e in doc.entities() {
                let Entity::Stair(st) = e else { continue };
                let own = e.own_level().map(str::to_owned);
                let own_i = own
                    .as_deref()
                    .and_then(|id| levels.iter().position(|l| l.id == id));
                let serves = if above {
                    own_i.is_some_and(|k| Some(k + 1) == i)
                } else {
                    own == *level
                };
                let at = st
                    .stair
                    .pts
                    .first()
                    .map(|p| V::new(p.x, p.y))
                    .unwrap_or(V::new(0.0, 0.0));
                let in_dwelling = room_holds(&own, at).is_some_and(|r| r.dwelling);
                if serves && !in_dwelling {
                    widths.push((st.stair.width, Slot(st.base.id)));
                }
            }
        }
        if widths.is_empty() {
            continue;
        }
        let column = if ground { 0 } else { 2 };
        let what = if ground {
            "dışarı çıkış kapılarının toplam net genişliği"
        } else {
            "kaçış merdivenlerinin toplam eni"
        };
        let need = people / u.per_unit()[column] * 0.5;
        let have: f64 = widths.iter().map(|(w, _)| w).sum();
        if have + 1e-6 < need {
            push(
                Said {
                    code: codes::BYKHY_EXIT_WIDTH_BELOW,
                    message: format!(
                        "“{name}” kişi yükü {people} ({}): {what} en az {}, çizimde {} — {}",
                        u.label(),
                        m(need),
                        m(have),
                        WIDTH_ARTICLE.short()
                    ),
                    repair: "Kaçış merdivenini ya da çıkış kapısını genişletin veya yeni bir çıkış ekleyin.".into(),
                    rule: WIDTH_ARTICLE.named(),
                    binding: None,
                },
                widths.iter().map(|(_, s)| *s).collect(),
                at,
            );
        }
        let least = if people > 2000.0 {
            2.0
        } else if people > 500.0 {
            1.5
        } else if people >= 50.0 {
            1.0
        } else {
            0.0
        };
        if let Some((w, slot)) = widths
            .iter()
            .copied()
            .filter(|(w, _)| *w + 1e-6 < least)
            .min_by(|a, b| a.0.total_cmp(&b.0))
        {
            push(
                Said {
                    code: codes::BYKHY_EXIT_WIDTH_BELOW,
                    message: format!(
                        "“{name}” kişi yükü {people}: her kaçış yolu en az {}; {} {} — {}",
                        m(least),
                        if ground {
                            "çıkış kapısı"
                        } else {
                            "merdiven"
                        },
                        m(w),
                        WIDTH_ARTICLE.short()
                    ),
                    repair: "Kaçış yolunu genişletin.".into(),
                    rule: WIDTH_ARTICLE.named(),
                    binding: None,
                },
                vec![slot],
                at,
            );
        }
        let exits = escape.exits.get(level).copied().unwrap_or(0);
        let asked = if people > 1000.0 {
            4
        } else if people > 500.0 {
            3
        } else if people > 50.0 {
            2
        } else {
            0
        };
        if exits < asked {
            push(
                Said {
                    code: codes::BYKHY_EXIT_COUNT_BELOW,
                    message: format!(
                        "“{name}” kişi yükü {people}: en az {asked} çıkış gerekir, {exits} var — {}",
                        COUNT_ARTICLE.short()
                    ),
                    repair: "Birbirinin alternatifi yeni bir kaçış merdiveni ya da çıkış ekleyin."
                        .into(),
                    rule: COUNT_ARTICLE.named(),
                    binding: None,
                },
                Vec::new(),
                at,
            );
        }
    }
    // Koridorlar (33/1) and a high building's stairs (33/2).
    for r in rooms
        .iter()
        .filter(|r| r.corridor && !r.dwelling && r.width > 0.0)
    {
        let (least, article) = if high {
            (1.20, HIGH_ARTICLE)
        } else {
            (1.10, WIDTH_ARTICLE)
        };
        if let Some(s) = said(
            codes::BYKHY_CORRIDOR_NARROW,
            &format!("{} net eni", r.name),
            r.width,
            Bound::AtLeast,
            Limit {
                value: least,
                article,
            },
            None,
            metres,
            if high {
                " (yüksek bina)"
            } else {
                " (kaçış yolu)"
            },
            "Koridoru genişletin.",
        ) {
            push(s, vec![r.slot], r.at);
        }
    }
    if high {
        for e in doc.entities() {
            let Entity::Stair(st) = e else { continue };
            let own = e.own_level().map(str::to_owned);
            let at = st
                .stair
                .pts
                .first()
                .map(|p| V::new(p.x, p.y))
                .unwrap_or(V::new(0.0, 0.0));
            if room_holds(&own, at).is_some_and(|r| r.dwelling) {
                continue;
            }
            if let Some(s) = said(
                codes::BYKHY_EXIT_WIDTH_BELOW,
                "Yüksek binada merdiven kolu",
                st.stair.width,
                Bound::AtLeast,
                Limit {
                    value: 1.20,
                    article: HIGH_ARTICLE,
                },
                None,
                metres,
                &format!(
                    " (bina yüksekliği {}, yapı yüksekliği {})",
                    m(building),
                    m(structure)
                ),
                "Merdiveni genişletin.",
            ) {
                push(s, vec![Slot(st.base.id)], at);
            }
        }
    }
    // Konut merdivenleri (48/5) by the structure's height.
    if dwelling_building && structure > 21.50 + 1e-9 {
        let ground = levels
            .iter()
            .find(|l| l.elevation >= -1e-6)
            .map(|l| l.id.clone());
        let stairs = doc
            .entities()
            .filter(|e| {
                let Entity::Stair(st) = e else { return false };
                let own = e.own_level().map(str::to_owned);
                let at = st
                    .stair
                    .pts
                    .first()
                    .map(|p| V::new(p.x, p.y))
                    .unwrap_or(V::new(0.0, 0.0));
                own == ground && !room_holds(&own, at).is_some_and(|r| r.dwelling)
            })
            .count();
        let (article, asked) = if structure > 51.50 + 1e-9 {
            (
                STAIRS_D_ARTICLE,
                "yangın güvenlik hollü ve basınçlandırılmış en az 2 kaçış merdiveni",
            )
        } else if structure > 30.50 + 1e-9 {
            (
                STAIRS_C_ARTICLE,
                "ikisi de korunumlu, birinde yangın güvenlik holü ya da basınçlandırma olan 2 kaçış merdiveni",
            )
        } else {
            (
                STAIRS_B_ARTICLE,
                "en az 2 merdiven, biri korunumlu ve her daireden ikisine de ulaşılan",
            )
        };
        if stairs < 2 {
            push(
                Said {
                    code: codes::BYKHY_STAIR_COUNT_BELOW,
                    message: format!(
                        "Yapı yüksekliği {}: {asked} gerekir; zemin katta {stairs} ortak merdiven var — {}",
                        m(structure),
                        article.short()
                    ),
                    repair: "İkinci, alternatif konumda bir kaçış merdiveni çizin; korunumu ve hollerini ayrıca \
                             belirtin (çizim yangın dayanımını göstermez)."
                        .into(),
                    rule: article.named(),
                    binding: None,
                },
                Vec::new(),
                rooms.first().map_or(V::new(0.0, 0.0), |r| r.at),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uses_and_loads_are_known_by_their_words() {
        assert_eq!(use_of(Some("Büro"), None), Some(Use::Office));
        assert_eq!(use_of(Some("Dükkân"), None), Some(Use::Shop));
        assert_eq!(use_of(Some("Daire"), None), Some(Use::Dwelling));
        assert_eq!(use_of(Some("Düğün salonu"), None), Some(Use::Assembly));
        assert_eq!(use_of(Some("Yatak odası"), None), None);
        assert_eq!(load_of(Some("Ofis 1"), None), Some((10.0, false)));
        assert_eq!(load_of(Some("Toplantı salonu"), None), Some((1.5, true)));
        assert_eq!(load_of(Some("Derslik"), None), Some((1.5, false)));
        assert_eq!(load_of(Some("Depo"), None), Some((30.0, false)));
        assert_eq!(load_of(Some("Koridor"), None), None);
        assert_eq!(load_of(Some("WC"), None), None);
        assert_eq!(load_of(Some("Merdiven evi"), None), None);
        assert_eq!(
            load_of(Some("Oda 4"), None),
            Some((0.0, false)),
            "a room nothing names: its use's own"
        );
        // Ek-5/B: an office 15/30 one way, 45/75 two ways; 100 · 80 · 60 · 100 people a unit width.
        assert_eq!(Use::Office.walks(), [[15.0, 30.0], [45.0, 75.0]]);
        assert_eq!(Use::Office.per_unit(), [100.0, 80.0, 60.0, 100.0]);
        assert_eq!(Use::Dwelling.walks(), [[15.0, 30.0], [30.0, 75.0]]);
        assert_eq!(Use::Office.stricter(Use::Hotel), Use::Hotel);
    }
}
