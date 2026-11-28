//! Emsalin 22. madde sınırları (docs/adr/8069, 8079): the uses Planlı Alanlar İmar Yönetmeliği 22/1 lets out of katlar
//! alanı only up to a limit — an independent unit's open balconies (22/1-l) and its storage annexes under the ground
//! (22/1-k), the caretaker flats (22/1-ç), the library (22/1-f), the shelter past its needed area (22/1-g,
//! docs/adr/8149), the residents' common rooms under the ground (22/1-j, docs/adr/8147) — what passes each limit, and
//! the storeys under the ground; and a traditional cut stone masonry building's taşlık, dış sofa and eyvan (22/1-m,
//! docs/adr/8094), let out only in such a building. The rooms are [`super::Room`]s, as the ratios read them.

use kentos_contracts::{ArchLevel, BuildingProgram, EmsalCount};

use super::Room;
use crate::room_names;

/// The limits of 22/1 that let a use out of katlar alanı only so far (docs/adr/8069, 8079).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoningLimit {
    /// 22/1-l: an independent unit's open balconies, up to 20 % of its net area.
    Balcony,
    /// 22/1-k: an independent unit's storage annexes on a storey under the ground, up to 50 % of its net area.
    Storage,
    /// 22/1-ç: the caretaker flats, 70 m² together.
    Caretaker,
    /// 22/1-f: the library rooms, 50 m² together.
    Library,
    /// 22/1-g (docs/adr/8149): the shelter rooms, up to the building's needed shelter area (Sığınak Yönetmeliği 8/a's
    /// net square metre a person, none where 7 asks no shelter) in their own gross by their net.
    Shelter,
    /// 22/1-m (docs/adr/8094): a traditional cut stone masonry building's taşlık, dış sofa and eyvan; no area of their
    /// own (the 30 % of 5/8 holds them with the others), the building's storeys and the rooms' place are the rule's.
    Traditional,
    /// 22/1-j (docs/adr/8147): the residents' common management and meeting rooms, sport and social rooms on a storey
    /// under the ground, together at most 5 % of katlar alanı and 1000 m².
    Amenity,
}

impl ZoningLimit {
    /// Its name in the facts (`kentos_extensions::audit_facts::building::EmsalLimit`).
    pub fn name(self) -> &'static str {
        match self {
            ZoningLimit::Balcony => "balcony",
            ZoningLimit::Storage => "storage",
            ZoningLimit::Caretaker => "caretaker",
            ZoningLimit::Library => "library",
            ZoningLimit::Shelter => "shelter",
            ZoningLimit::Traditional => "traditional",
            ZoningLimit::Amenity => "amenity",
        }
    }
}

/// A limited use counted out (docs/adr/8079): the limit, the independent unit whose net area it is a share of (a
/// balcony's, a storage annex's), its rooms by their persistent ids in the drawing's order, their area as katlar alanı
/// counts it, what the limit lets out and what passed it, counted in (m²), and whether every room of it is a common
/// area's (a program group of kind common).
#[derive(Clone, Debug, PartialEq)]
pub struct ZoningLimitUse {
    pub limit: ZoningLimit,
    pub unit: Option<String>,
    pub rooms: Vec<String>,
    pub area: f64,
    pub allowed: f64,
    pub over: f64,
    pub common: bool,
}

/// The share of an independent unit's net area its open balconies may take out of katlar alanı (PAİY 22/1-l, as amended
/// RG 11/7/2021-31538).
pub const BALCONY_SHARE: f64 = 0.20;

/// The share of an independent unit's net area its storage annexes on a storey under the ground may take out of katlar
/// alanı (PAİY 22/1-k, as amended RG 25/7/2019-30842).
pub const STORAGE_SHARE: f64 = 0.50;

/// The caretaker flats' area together katlar alanı leaves out, m² (PAİY 22/1-ç, as amended RG 25/7/2019-30842: "kapıcı
/// dairelerinin toplam 70 m²'si"; the text it replaced read each flat's 75 m², docs/adr/8079 §1).
pub const CARETAKER_AREA: f64 = 70.0;

/// The library rooms' area together katlar alanı leaves out, m² (PAİY 22/1-f, as amended RG 23/1/2021-31373).
pub const LIBRARY_AREA: f64 = 50.0;

/// The share of katlar alanı the residents' common rooms under the ground may take out of it together (PAİY 22/1-j, as
/// amended RG 25/7/2019-30842: "toplamda emsal hesabına konu alanın % 5'ini").
pub const AMENITY_SHARE: f64 = 0.05;

/// The most those rooms may take out together, m² (PAİY 22/1-j: "… ve 1000 m2'yi geçmeyen").
pub const AMENITY_AREA: f64 = 1000.0;

/// What 22/1-j lets out of `floors`, the katlar alanı it is a share of (docs/adr/8147 K376): 5 % of it, at most
/// 1000 m².
pub fn amenity_allowed(floors: f64) -> f64 {
    (AMENITY_SHARE * floors).min(AMENITY_AREA)
}

/// Whether a room or its group says `word` (folded, a word's beginning: Kapıcı dairesi, Depolar).
pub(super) fn names(word: &str, room: &[String], group: &[String]) -> bool {
    room_names::has(room, word) || room_names::has(group, word)
}

/// Whether `name` or `usage` says a balcony, Turkish letters folded.
pub(super) fn says_balcony(name: Option<&str>, usage: Option<&str>) -> bool {
    [name, usage].into_iter().flatten().any(|t| {
        t.chars()
            .map(|c| match c {
                'ı' | 'I' | 'İ' | 'i' => 'i',
                c => c.to_lowercase().next().unwrap_or(c),
            })
            .collect::<String>()
            .contains("balkon")
    })
}

/// Whether a room's words say a traditional building's taşlık, dış sofa or eyvan (PAİY 22/1-m, docs/adr/8094): a word
/// beginning with Taşlık or Eyvan, or the words Dış and Sofa (a sofa within is a hall, not 22/1-m's).
pub(super) fn says_traditional(words: &[String]) -> bool {
    room_names::has(words, "TASLIK")
        || room_names::has(words, "EYVAN")
        || (room_names::is(words, "DIS") && room_names::has(words, "SOFA"))
}

/// Whether a room's words or its group's say 22/1-j's rooms (docs/adr/8147): "yönetim ve toplantı odaları ile spor ve
/// sosyal mekânlar", a word beginning with Yönetim, Toplantı, Spor or Sosyal.
pub(super) fn says_amenity(room: &[String], group: &[String]) -> bool {
    ["YONETIM", "TOPLANTI", "SPOR", "SOSYAL"]
        .into_iter()
        .any(|w| names(w, room, group))
}

/// The storeys under the ground, by their ids (22/1-k: "bütün cepheleri tamamen toprağın altında kalan bodrum katlar";
/// docs/adr/8079): a storey whose top (its floor's elevation and its height, else the floor of the next storey up) is
/// at or below its building's ±0.00. The model's graded ground lies at ±0.00: sections draw it there, and the model
/// holds no other ground (EK-4 4.30, docs/adr/8042 §4). A storey whose top is not known (no height, none above) is not.
pub(super) fn buried_levels(arch: &kentos_contracts::ArchSettings) -> Vec<String> {
    let levels: &[ArchLevel] = &arch.levels;
    levels
        .iter()
        .filter(|l| {
            let top = l.height.map(|h| l.elevation + h).or_else(|| {
                levels
                    .iter()
                    .map(|o| o.elevation)
                    .filter(|e| *e > l.elevation)
                    .reduce(f64::min)
            });
            let zero = arch
                .program
                .building_of_level(&l.id)
                .and_then(|b| b.zero)
                .unwrap_or(0.0);
            top.is_some_and(|t| t <= zero + 1e-6)
        })
        .map(|l| l.id.clone())
        .collect()
}

/// The limits of 22/1 on the rooms counted out: each room's part past its limit (counted in), the limit it is under, and
/// each limited use.
///
/// - l: an independent unit's open balconies up to 20 % of its net area (its own indoor rooms counted in, 4/1-ğ-3:
///   balconies, terraces, annexes and common areas not in it); a balcony of no unit, or of a common area, keeps its
///   count;
/// - k: a unit's storage annexes under the ground up to 50 % of its net area;
/// - ç: the caretaker flats 70 m² together; f: the library rooms 50 m² together;
/// - g: the shelter rooms up to the building's needed shelter area `shelter_need` (net, m²; docs/adr/8149 K384), in
///   their gross by their net: the need times their gross over their net; none: the need not known (no unit in the
///   program), no limit;
/// - m: a traditional cut stone masonry building's taşlık, dış sofa and eyvan, all of them (no area of their own);
/// - j: the residents' common rooms under the ground, together 5 % of katlar alanı and at most 1000 m² (docs/adr/8147
///   K376): katlar alanı as the rooms counted in and the other limits' excess make it, last; neither its own excess nor
///   5/8's past the 30 % (both rest on it) is in it.
///
/// What passes a limit counts in, shared among its rooms by their areas; a room is under the first limit that reads it.
pub(super) fn apply(
    rooms: &[Room],
    gross: &[f64],
    uids: &[String],
    program: &BuildingProgram,
    shelter_need: Option<f64>,
) -> (Vec<f64>, Vec<Option<ZoningLimit>>, Vec<ZoningLimitUse>) {
    let mut moved = vec![0.0; rooms.len()];
    let mut limit_of: Vec<Option<ZoningLimit>> = vec![None; rooms.len()];
    let mut limits: Vec<ZoningLimitUse> = Vec::new();
    let net_of = |unit: &str| -> f64 {
        rooms
            .iter()
            .filter(|r| {
                r.unit.as_deref() == Some(unit)
                    && !r.void
                    && !r.outdoor
                    && r.count == EmsalCount::Included
            })
            .map(|r| r.net)
            .sum()
    };
    let units: Vec<&str> = program
        .units
        .iter()
        .filter(|u| u.kind == kentos_contracts::UnitKind::Unit)
        .map(|u| u.id.as_str())
        .collect();
    // Each limit, the unit it is a share of and what it lets out; none: all of its rooms (22/1-m).
    let mut asks: Vec<(ZoningLimit, Option<&str>, Option<f64>)> = Vec::new();
    for limit in [ZoningLimit::Balcony, ZoningLimit::Storage] {
        for &u in &units {
            let share = if limit == ZoningLimit::Balcony {
                BALCONY_SHARE
            } else {
                STORAGE_SHARE
            };
            asks.push((limit, Some(u), Some(share * net_of(u))));
        }
    }
    asks.push((ZoningLimit::Caretaker, None, Some(CARETAKER_AREA)));
    asks.push((ZoningLimit::Library, None, Some(LIBRARY_AREA)));
    if shelter_need.is_some() {
        asks.push((ZoningLimit::Shelter, None, None));
    }
    asks.push((ZoningLimit::Traditional, None, None));
    asks.push((ZoningLimit::Amenity, None, None));
    for (limit, unit, allowed) in asks {
        let members: Vec<usize> = (0..rooms.len())
            .filter(|&i| {
                let r = &rooms[i];
                limit_of[i].is_none()
                    && !r.void
                    && r.count == EmsalCount::Exempt
                    && match limit {
                        ZoningLimit::Balcony => r.balcony && r.unit.as_deref() == unit,
                        ZoningLimit::Storage => r.storage_of.as_deref() == unit,
                        ZoningLimit::Caretaker => r.caretaker,
                        ZoningLimit::Library => r.library,
                        ZoningLimit::Shelter => r.shelter,
                        ZoningLimit::Traditional => r.traditional,
                        ZoningLimit::Amenity => r.amenity,
                    }
            })
            .collect();
        if members.is_empty() {
            continue;
        }
        let area: f64 = members.iter().map(|&i| gross[i]).sum();
        let allowed = match limit {
            // Katlar alanı so far: the rooms counted in and what the other limits moved in.
            ZoningLimit::Amenity => amenity_allowed(
                rooms
                    .iter()
                    .zip(gross)
                    .filter(|(r, _)| !r.void && r.count == EmsalCount::Included)
                    .map(|(_, g)| g)
                    .sum::<f64>()
                    + moved.iter().sum::<f64>(),
            ),
            // The needed net area in the rooms' gross: their walls' share with it.
            ZoningLimit::Shelter => {
                let need = shelter_need.unwrap_or(0.0);
                let net: f64 = members.iter().map(|&i| rooms[i].net).sum();
                if net > 0.0 {
                    (need * area / net).min(area)
                } else {
                    need.min(area)
                }
            }
            _ => allowed.unwrap_or(area),
        };
        let over = (area - allowed).max(0.0);
        for &i in &members {
            limit_of[i] = Some(limit);
            if over > 0.0 && area > 0.0 {
                moved[i] = gross[i] * over / area;
            }
        }
        let common = members.iter().all(|&i| {
            rooms[i]
                .unit
                .as_deref()
                .and_then(|u| program.unit(u))
                .is_some_and(|g| g.kind == kentos_contracts::UnitKind::Common)
        });
        limits.push(ZoningLimitUse {
            limit,
            unit: unit.map(str::to_owned),
            rooms: members.iter().map(|&i| uids[i].clone()).collect(),
            area,
            allowed,
            over,
            common,
        });
    }
    (moved, limit_of, limits)
}

#[cfg(test)]
mod tests {
    use super::amenity_allowed;

    /// 22/1-j's limit worked by hand (docs/adr/8147): 5 % of 2400 m² is 120 m²; of 20000 m² it is 1000 m², the bent's
    /// own; of 30000 m² it would be 1500 m², the 1000 m² holds.
    #[test]
    fn the_common_rooms_limit_is_5_percent_of_katlar_alani_and_at_most_1000_m2() {
        for (floors, allowed) in [
            (2400.0, 120.0),
            (20000.0, 1000.0),
            (30000.0, 1000.0),
            (0.0, 0.0),
        ] {
            assert!(
                (amenity_allowed(floors) - allowed).abs() < 1e-9,
                "{floors}: {}",
                amenity_allowed(floors)
            );
        }
    }
}
