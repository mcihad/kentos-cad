//! A drawing's architecture facts: the `architecture` section of `kentos.auditFacts` 1 (docs/adr/8057 §39). The
//! storeys, every room with its net area and its outline to the walls' axes, the building program's groups with
//! Planlı Alanlar İmar Yönetmeliği article 4's areas, and the escape routes Binaların Yangından Korunması Hakkında
//! Yönetmelik measures: as `kentos_domain` works them out (`paiy_areas`, `escape`), for a rule's Python to judge.
//!
//! An optional section of version 1, not a version 2: a reader of version 1 ignores a field it does not know, and a
//! drawing without rooms gives no section at all, so the facts of a drawing without architecture stay byte for byte
//! what they were. The rules are the layers' ([`super`]): areas to 0.0001 m², lengths, places, elevations and heights
//! to 1 mm; the storeys by elevation, then id, and the storeys' exits and a group's storeys in that order (the rooms on
//! no storey first, a storey the project does not name last, by id); the rooms and the doors by persistent id; the
//! groups in the program's order; a walk's points as walked. A list holds at most [`super::MOST_ITEMS`] items, the first
//! in its order; past it the facts say `cut`.

use serde::{Deserialize, Serialize};

use super::{AREA, LENGTH, MOST_ITEMS, place, round};

/// The drawing's architecture.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct ArchitectureFacts {
    /// The project's storeys, lowest first.
    pub levels: Vec<Storey>,
    /// Each storey holding rooms and how many storey exits it has (BYKHY 4/u): an escape stair's room, an open
    /// stair's foot or head, a door to the outside on a storey at or below ±0.00.
    pub exits: Vec<StoreyExits>,
    /// Every room (space), a void and an outdoor one too.
    pub rooms: Vec<RoomFacts>,
    /// The building program's groups that have rooms, with article 4's areas, in the program's order.
    pub units: Vec<UnitFacts>,
    /// Each dwelling unit's door: the walk from its outer side to the nearest storey exit (BYKHY 48/4).
    pub doors: Vec<DoorEscape>,
    /// The building has an automatic sprinkler system, as the project's settings say (`.kcad` 226): the fire
    /// regulation's sprinklered distances are the ones read (BYKHY 48/2, Ek-5/B; docs/adr/8057 §53).
    #[serde(default)]
    pub sprinklers: bool,
}

/// A storey of the project.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct Storey {
    pub id: String,
    pub name: String,
    /// Its floor's elevation, m.
    pub elevation: f64,
    /// Its height, m; none when the project gives none.
    pub height: Option<f64>,
}

/// A storey's exits counted.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct StoreyExits {
    /// The storey's id; none for the rooms on no storey.
    pub level: Option<String>,
    pub count: u64,
}

/// A room.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct RoomFacts {
    /// Its persistent id.
    pub uid: String,
    pub number: Option<String>,
    pub name: Option<String>,
    /// Its usage, as the program's briefs name it.
    pub usage: Option<String>,
    /// Its kind as the regulations read rooms, by its name or usage (KentOS Mimari's reading: Salon, Yatak odası,
    /// Mutfak, Banyo, WC, Hol …); none for another room (a store, a study, a balcony).
    #[serde(default)]
    pub kind: Option<RoomKind>,
    /// The use its name or usage names as BYKHY Ek-5/B's rows do (Otopark, Depo, Lokanta …), as Mimari denetim's
    /// fire rules read a room outside a unit (docs/adr/8057 §82); none when they name none, or a dwelling's.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupancy: Option<Occupancy>,
    /// Its storey's id.
    pub level: Option<String>,
    /// Its program group's id.
    pub unit: Option<String>,
    /// A void: a shaft, a lift shaft, an opening in the floor.
    pub void: bool,
    /// Its usage's brief is an outdoor one (a balcony, a terrace).
    pub outdoor: bool,
    /// Its boundary could not be found round its seed: its areas are its drawn rings', and it has no walks.
    pub broken: bool,
    /// An escape stair's room (named or used so, no dwelling unit's own): reaching it is reaching the storey's exit.
    pub stair: bool,
    /// Its net area, m²: between the walls' inner faces, as article 4 measures it.
    pub net: f64,
    /// Its outline to the walls' axes, m²: what most uses' person loads read (BYKHY Ek-5/A).
    pub axis: f64,
    /// Its net width, m: the diameter of the largest circle inside its net outline, the regulations' narrow side
    /// (dar kenar, PAİY 29/1 and 29/3); none for a broken room.
    #[serde(default)]
    pub narrow: Option<f64>,
    /// Its walks; none for a void, an outdoor or a broken room and an escape stair's room.
    pub escape: Option<RoomEscape>,
}

/// A room's kind, as Planlı Alanlar İmar Yönetmeliği 29 names the rooms of a dwelling (the passages too).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum RoomKind {
    /// Oturma odası (Salon).
    Living,
    /// Yatak odası (Ebeveyn, Çocuk, Oda).
    Bedroom,
    /// Mutfak veya yemek pişirme yeri.
    Kitchen,
    /// Banyo veya yıkanma yeri (Duş).
    Bath,
    /// Tuvalet (WC).
    Toilet,
    /// Hol, koridor, antre.
    Corridor,
}

/// A room's walks, from its farthest point: 40 cm in front of its walls (BYKHY 32/6).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RoomEscape {
    /// To the nearest storey exit (BYKHY 32/4); none when it reaches none.
    pub exit: Option<EscapeWalk>,
    /// Whether its storey has an exit it may walk to: through its own unit's rooms and the common ones (33/6-b).
    pub storey_exits: bool,
    /// A dwelling unit's room: to its unit's door, through its own rooms (48/2); none for another room.
    pub entrance: Option<EscapeWalk>,
    /// Whether its unit has a door on its storey (a dwelling unit's room only).
    pub has_entrance: bool,
    /// Its doors and gaps without a sill: its exits for BYKHY 39/2.
    pub passages: u64,
}

/// A walk, along the points it turns at.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct EscapeWalk {
    /// Its length, m.
    pub length: f64,
    /// A walk to an exit: the part it shares with the walk to the nearest other exit, m (tek yön; a dead end's); all
    /// of it with one exit. None for a walk to a unit's door.
    pub one_way: Option<f64>,
    /// A walk to an exit: how many exits its start reaches. None for a walk to a unit's door.
    pub exits: Option<u64>,
    /// Its points, from its start to its end.
    pub path: Vec<[f64; 2]>,
}

/// What a program group is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum GroupKind {
    /// Bağımsız bölüm: a flat, a shop, an office.
    Unit,
    /// Ortak alan: a shared corridor, the stairs, a shaft.
    Common,
    /// Dış alan: a garden, a car park.
    Outdoor,
    /// Eklenti: a place allocated to an independent unit (its `annexOf`).
    Annex,
}

/// A unit's use as BYKHY Ek-5/B's rows name it, as KentOS Mimari reads it from the unit's nature (Daire, Büro,
/// Dükkân, Otel …): the row of its escape distances.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub enum Occupancy {
    /// Yüksek tehlikeli yerler.
    HighHazard,
    /// Endüstri amaçlı yapılar.
    Industry,
    /// Yurtlar, yatakhaneler.
    Dormitory,
    /// Mağazalar, dükkânlar, marketler.
    Shop,
    /// Büro binaları.
    Office,
    /// Otoparklar ve depolar.
    Parking,
    /// Okul ve eğitim yapıları.
    School,
    /// Toplanma amaçlı binalar.
    Assembly,
    /// Hastaneler, huzurevleri.
    Hospital,
    /// Oteller, pansiyonlar.
    Hotel,
    /// Apartmanlar.
    Dwelling,
}

/// A program group's article 4 areas (Planlı Alanlar İmar Yönetmeliği 4/ğ), m².
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnitFacts {
    pub id: String,
    /// Its number, as the project writes it.
    pub number: String,
    pub name: Option<String>,
    pub kind: GroupKind,
    /// What the deed calls it (Daire, Dükkân, Büro, Depo).
    pub nature: Option<String>,
    /// An annex's independent unit, by its id.
    pub annex_of: Option<String>,
    /// An independent unit that is a dwelling (BYKHY 48 reads its rooms).
    pub dwelling: bool,
    /// Its use as BYKHY Ek-5/B's rows name it, read from its nature (an annex's from its unit's; docs/adr/8057 §79);
    /// none for a common or outdoor group and for a nature that names no use.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub occupancy: Option<Occupancy>,
    /// BB brüt alanı (4/ğ-1): its rooms' outer contour, a wall it shares cut at its axis.
    pub gross: f64,
    /// BB net alanı (4/ğ-3): its rooms' net areas, voids and outdoor rooms out.
    pub net: f64,
    /// Its annexes' gross and net areas summed (an independent unit's; 0 for another group).
    pub annex_gross: f64,
    pub annex_net: f64,
    /// BB toplam brüt alanı (4/ğ-4): its gross and its annexes'.
    pub total: f64,
    /// Its part of the common areas' gross, by the program's rule; none when the program shares nothing.
    pub share: Option<f64>,
    /// BB genel brüt alanı (4/ğ-2): its total and its part; none without a part.
    pub general: Option<f64>,
    /// Each storey's part of its gross and net.
    pub levels: Vec<UnitLevel>,
}

/// A group's areas on one storey, m².
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct UnitLevel {
    pub level: Option<String>,
    pub gross: f64,
    pub net: f64,
}

/// A dwelling unit's door and its walk to the nearest storey exit.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct DoorEscape {
    /// The door's persistent id.
    pub uid: String,
    /// Its unit's id.
    pub unit: String,
    pub level: Option<String>,
    /// The door's middle.
    pub at: [f64; 2],
    /// From its outer side; none when it opens into an escape stair's room or reaches no exit.
    pub exit: Option<EscapeWalk>,
    /// It opens into an escape stair's room: it stands at its storey's exit (no walk, none needed).
    #[serde(default, rename = "intoStair")]
    pub into_stair: bool,
}

impl ArchitectureFacts {
    /// These facts in the module's order, rounding and bounds, and whether a list was cut. Doing it again changes
    /// nothing.
    pub fn normalized(mut self) -> (ArchitectureFacts, bool) {
        for s in &mut self.levels {
            s.elevation = round(s.elevation, LENGTH);
            s.height = s.height.map(|h| round(h, LENGTH));
        }
        self.levels.sort_by(|a, b| {
            a.elevation
                .total_cmp(&b.elevation)
                .then_with(|| a.id.cmp(&b.id))
        });
        let levels = std::mem::take(&mut self.levels);
        let rank = |level: &Option<String>| match level {
            None => (0, None),
            Some(id) => (
                levels
                    .iter()
                    .position(|l| &l.id == id)
                    .map_or(levels.len() + 1, |i| i + 1),
                Some(id.clone()),
            ),
        };
        self.exits.sort_by_key(|e| rank(&e.level));
        for r in &mut self.rooms {
            r.net = round(r.net, AREA);
            r.axis = round(r.axis, AREA);
            r.narrow = r.narrow.map(|w| round(w, LENGTH));
            if let Some(e) = &mut r.escape {
                e.exit = e.exit.take().map(EscapeWalk::normalized);
                e.entrance = e.entrance.take().map(EscapeWalk::normalized);
            }
        }
        self.rooms.sort_by(|a, b| a.uid.cmp(&b.uid));
        for u in &mut self.units {
            for a in [
                &mut u.gross,
                &mut u.net,
                &mut u.annex_gross,
                &mut u.annex_net,
                &mut u.total,
            ] {
                *a = round(*a, AREA);
            }
            u.share = u.share.map(|s| round(s, AREA));
            u.general = u.general.map(|g| round(g, AREA));
            for l in &mut u.levels {
                l.gross = round(l.gross, AREA);
                l.net = round(l.net, AREA);
            }
            u.levels.sort_by_key(|l| rank(&l.level));
        }
        self.levels = levels;
        for d in &mut self.doors {
            d.at = place(d.at);
            d.exit = d.exit.take().map(EscapeWalk::normalized);
        }
        self.doors.sort_by(|a, b| {
            a.uid
                .cmp(&b.uid)
                .then_with(|| a.unit.cmp(&b.unit))
                .then_with(|| a.level.cmp(&b.level))
        });
        let cut = self.rooms.len() > MOST_ITEMS || self.doors.len() > MOST_ITEMS;
        self.rooms.truncate(MOST_ITEMS);
        self.doors.truncate(MOST_ITEMS);
        (self, cut)
    }
}

impl EscapeWalk {
    fn normalized(mut self) -> EscapeWalk {
        self.length = round(self.length, LENGTH);
        self.one_way = self.one_way.map(|w| round(w, LENGTH));
        for p in &mut self.path {
            *p = place(*p);
        }
        self
    }
}

#[cfg(test)]
mod tests;
