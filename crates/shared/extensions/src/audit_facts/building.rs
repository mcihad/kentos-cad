//! A drawing's building facts: the `building` section of `kentos.auditFacts` 1 (docs/adr/8078). What the regulation's
//! rules count a building by that the architecture section does not carry: every storey of the project with its
//! elevation, the rooms it holds by their program group's kind (an independent unit's, a common area's, an annex's, none)
//! and as a technical or a lived-in room, its elevators; the program's independent units and its dwellings; the
//! municipality's regulation the project reads over the national one. A rule decides what a storey counts as for its
//! article and its version (Planlı Alanlar İmar Yönetmeliği 34/1 before and after 1.7.2026 count a basement apart);
//! the facts say what is drawn, never what it means.
//!
//! Planlı Alanlar İmar Yönetmeliği 28/1-c and 39/1 (docs/adr/8083) read each storey's height floor to floor and its
//! bedrooms, the program's common areas, and each door's net clear width, height and what stands on its two sides; the
//! parcel's plan notes (docs/adr/8084) give a rule the plan's value over the regulations'.
//!
//! An optional section of version 1, as [`super::architecture`]: absent when the project has no storey, so the facts
//! of a drawing that never had one stay byte for byte what they were. The storeys lowest first (by elevation, then
//! id), each elevation to 1 mm.

use serde::{Deserialize, Serialize};

use super::{AREA, LENGTH, round};

/// The building as the regulation's rules count it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct BuildingFacts {
    /// Every storey of the project, lowest first.
    pub storeys: Vec<StoreyHolds>,
    /// The building program's independent units (bağımsız bölüm).
    pub units: u32,
    /// Those of them whose nature is a dwelling's (Daire, Mesken, Konut) or not given.
    pub dwellings: u32,
    /// The building program's common areas (ortak alan groups): with one, or with more than one unit, a building has an
    /// entrance of its own apart from its units' (docs/adr/8083).
    #[serde(default)]
    pub commons: u32,
    /// The municipality's regulation the project reads over the national one (`sivas`: Sivas Belediyesi İmar
    /// Yönetmeliği, RG 10.11.2022/32009); absent: none.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local: Option<String>,
    /// The parcel's plan notes the project writes (docs/adr/8084), a rule reads over the municipality's and the national
    /// regulation (PAİY 69/3), in the project's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub plan_notes: Vec<PlanNoteFacts>,
    /// What the parcel's plan uses its area for, as the project says it (docs/adr/8085): `residential`,
    /// `residentialShops`, `mixed`, `commercial`, `other`; absent: not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub zone: Option<String>,
    /// The building's structural system as the project declares it (docs/adr/8094): `reinforcedConcrete`, `steel`,
    /// `masonry`, `cutStoneMasonry`, `timber`, `composite`; absent: not said.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub structural_system: Option<String>,
    /// The project says the building holds traditional architectural elements (only of cut stone masonry): PAİY 22/1-m
    /// and 28/1-d read such a building of up to three storeys (docs/adr/8094).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub traditional: bool,
    /// The rooms that carry people as the escape rules count them (BYKHY Ek-5/A, docs/adr/8035), by persistent id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rooms: Vec<RoomPeople>,
    /// The doors, by persistent id (docs/adr/8078 §8).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub doors: Vec<DoorFacts>,
    /// The uses Planlı Alanlar İmar Yönetmeliği 22/1 lets out of katlar alanı only up to a limit, as Emsal ve TAKS
    /// tablosu counts them (docs/adr/8079): a unit's balconies, its storage annexes, the caretaker flats, the library.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub limits: Vec<EmsalLimit>,
    /// What the program's buildings are used for, as the project writes it (Kullanım türü), in the program's order.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub uses: Vec<String>,
    /// The stairs with places under them lower than 1.80 m (docs/adr/8080; PAİY 4/1-ğ-3, 31/6), by persistent id.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub under_stairs: Vec<UnderStairFacts>,
    /// Every room the storeys count, with its name and area (docs/adr/8081): what a fire compartment is made of.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub spaces: Vec<SpaceFacts>,
    /// What separates each two rooms of a storey that share a boundary (docs/adr/8081; BYKHY 4/tt, 24).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub separations: Vec<SeparationFacts>,
    /// The openings between storeys (docs/adr/8086): galleries, atriums, stair wells, stairs.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub links: Vec<LinkFacts>,
    /// The shafts and what encloses each (docs/adr/8087; BYKHY 25/3, 41/9, 62/2).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub shafts: Vec<ShaftFacts>,
    /// The coverings (docs/adr/8091; BYKHY 26/2, 26/4): each its IFC type, storey, material and the material's declared
    /// reaction to fire class.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub coverings: Vec<CoveringFacts>,
    /// The project says the building has sprinklers (`settings.arch.sprinklers`): BYKHY Ek-4's notes read it.
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub sprinklers: bool,
    /// The elements BYKHY Ek-3/B gives fire resistance times for, with their declared classes (docs/adr/8082).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub elements: Vec<ElementFacts>,
    /// Bina yüksekliği (BYKHY 4/h): the storeys on ±0.00 and above, as the fire rules measure them, m.
    #[serde(default)]
    pub height: f64,
    /// Bodrum katların derinliği (Ek-3/C's footnote): the lowest basement's floor under ±0.00, m; 0 without one.
    #[serde(default)]
    pub basement_depth: f64,
    /// Yapı yüksekliği (BYKHY 4/bbb): every storey built, basements, mezzanines and attics with them, as the fire rules
    /// measure it, m (docs/adr/8087: BYKHY 25/3's high building and 62/7's elevator doors read it).
    #[serde(default)]
    pub structure_height: f64,
}

/// A shaft and what encloses it (docs/adr/8087): its persistent id, storey and name, `elevator` (an elevator's shaft) or
/// `service` (a service shaft, a flue, an air shaft), the walls and openings met round it with their declared classes and
/// lengths, and the length with neither, m.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ShaftFacts {
    pub uid: String,
    pub level: Option<String>,
    pub name: String,
    pub kind: String,
    pub walls: Vec<SeparatingElement>,
    pub openings: Vec<SeparatingElement>,
    pub open: f64,
}

/// A plan note's value: its measure (`groundStoreyHeight`, `storeyHeight`), the value (m) and where it stands as the
/// architect writes it (“UİP 12.3.2019, plan notu 7”).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct PlanNoteFacts {
    pub key: String,
    pub value: f64,
    pub source: String,
}

/// An element BYKHY Ek-3/B reads: its persistent id, its kind (`column`, `beam`, `wall`, `slab`; `roof`, `terrace`,
/// docs/adr/8087; `ceiling`, a suspended ceiling, docs/adr/8089), its storey, its declared fire class (its own, else its
/// type's), whether it is a column or a beam of the frame, whether a wall is on its storey's outline (docs/adr/8082).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct ElementFacts {
    pub uid: String,
    pub kind: String,
    pub level: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_rating: Option<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub frame: bool,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub exterior: bool,
    /// A wall's declared load-bearing role (docs/adr/8090, Ek-3/B 2); none: not declared.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub load_bearing: Option<bool>,
    /// A roof declared an escape route (docs/adr/8090, Ek-3/B 4-a).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub escape: bool,
}

/// A covering (docs/adr/8091): its persistent id, storey, IFC 4.3 type (`CEILING` a suspended ceiling, `FLOORING` a
/// floor covering …), its material by name and that material's declared reaction to fire class (TS EN 13501-1); none:
/// no material, or none declared.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct CoveringFacts {
    pub uid: String,
    pub level: Option<String>,
    pub predefined: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub material: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reaction: Option<String>,
}

/// A room as a fire compartment counts it: its persistent id, its storey, its name (and number), its area as the
/// regulation's areas measure it (its outer contour, PAİY 4/1-ğ-1), m².
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SpaceFacts {
    pub uid: String,
    pub level: Option<String>,
    pub name: String,
    pub area: f64,
    /// An escape stair's room (named or used as Merdiven, no dwelling unit's own): its enclosure across the storeys is
    /// no storey's compartment (docs/adr/8086).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stair: bool,
}

/// An opening between two storeys (docs/adr/8086; BYKHY 4/g, 24/4, 24/5): a floor slab's hole, a void space that is no
/// shaft, a stair; the rooms it joins on the storey under it and on the storey it opens into, by persistent id; its area
/// (m², 0 for a stair); whether a stair arrives through it (a stair well); the stack it belongs to (openings over one
/// another on consecutive storeys, by the lowest one's id).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct LinkFacts {
    pub uid: String,
    /// `hole`, `void` or `stair`.
    pub kind: String,
    pub below: Option<String>,
    pub above: Option<String>,
    pub rooms_below: Vec<String>,
    pub rooms_above: Vec<String>,
    pub area: f64,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stair_well: bool,
    pub stack: String,
}

/// What separates two rooms of a storey along their shared boundary (docs/adr/8081): the walls, the openings (a door, a
/// window, a gap), each with its declared fire class (its own, else its type's; TS EN 13501-2's words as written) and
/// its length there, and the length with neither (a room separator, a gap), m.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SeparationFacts {
    pub level: Option<String>,
    pub rooms: [String; 2],
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub walls: Vec<SeparatingElement>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub openings: Vec<SeparatingElement>,
    pub open: f64,
}

/// A wall or an opening along a separation: its persistent id, its kind (`wall`, `door`, `window`, `gap`), its declared
/// fire class, its length there, m.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct SeparatingElement {
    pub uid: String,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_rating: Option<String>,
    pub length: f64,
}

/// A stair's places under it lower than 1.80 m over the floor it starts from (docs/adr/8080), from its body as the
/// section draws it: all of them and the spaces among them (where its underside stands over its floor), m², whether its
/// waist is known (a slab it arrives at; without one only the places under treads and landings lower than 1.80 m, taken
/// as solid), and the rooms its spaces lie in.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnderStairFacts {
    pub stair: String,
    pub level: Option<String>,
    pub low: f64,
    pub open: f64,
    pub waist: bool,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rooms: Vec<UnderStairRoom>,
}

/// A room an under-stair space lies in: its persistent id, its name (and number), and the area of the space in it, m².
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct UnderStairRoom {
    pub uid: String,
    pub name: String,
    pub area: f64,
}

/// A use 22/1 lets out of katlar alanı only up to a limit (docs/adr/8079), as Emsal ve TAKS tablosu counts it.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct EmsalLimit {
    /// `balcony` (22/1-l: a unit's open balconies, 20 % of its net area), `storage` (22/1-k: a unit's storage annexes
    /// on a storey under the ground, 50 % of its net area), `caretaker` (22/1-ç: the caretaker flats, 70 m² together),
    /// `library` (22/1-f: the library rooms, 50 m² together) or `traditional` (22/1-m, docs/adr/8094: a traditional cut
    /// stone masonry building's taşlık, dış sofa and eyvan, no area of their own; the building's conditions are the
    /// rule's).
    pub limit: String,
    /// The independent unit whose net area the limit is a share of, by program id (a balcony's, a storage annex's).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unit: Option<String>,
    /// Its rooms, by persistent id.
    pub rooms: Vec<String>,
    /// Their area as katlar alanı counts it (each room's outer contour, PAİY 4/1-ğ-1), m².
    pub area: f64,
    /// What the limit lets out, m².
    pub allowed: f64,
    /// What passed it, counted in, m².
    pub over: f64,
    /// Every room of it is a common area's (a program group of kind `common`).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub common: bool,
}

/// A room's people: its use's square metres a person over its area, rounded up (BYKHY Ek-5/A as Mimari denetim reads
/// it).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct RoomPeople {
    pub uid: String,
    pub level: Option<String>,
    /// Its name (and number).
    pub name: String,
    pub persons: f64,
}

/// A door: the rooms on its two sides, the one its leaf swings into, how it opens, its declared fire class, and whether
/// it opens into an escape stair's room or a fire safety lobby.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct DoorFacts {
    pub uid: String,
    pub level: Option<String>,
    /// The rooms on its two sides by persistent id, the path's left first; none: no room there (outdoors, a wall's
    /// other face with nothing drawn).
    pub between: [Option<String>; 2],
    /// The room its leaf swings into; none for a door that does not swing or swings where no room is.
    pub opens_into: Option<String>,
    /// `swing`, `sliding`, `folding`, `revolving`, `pocket` … as the opening's operation is named.
    pub operation: String,
    /// The fire class the author declares, the door's own else its type's (docs/adr/8068): `EI2 60-S200C5`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fire_rating: Option<String>,
    /// One of its sides is an escape stair's room (a room named or used as Merdiven that is no dwelling unit's own).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stair: bool,
    /// One of its sides is a fire safety lobby (a room named Yangın güvenlik holü).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub lobby: bool,
    /// Its net clear width (net temiz geçiş: with its leaves open, from the open leaf's face to the opposite stop,
    /// docs/adr/0772 §2), m; none where it has none (docs/adr/8083).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear: Option<f64>,
    /// Its height, its head over its sill, m; none without a head.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    /// A door of two or more leaves: each leaf's width from its start, m (docs/adr/8093; PAİY 39/1-b); empty for one.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub leaves: Vec<f64>,
    /// A two-leaf door's net clear width with only its first leaf open, m (docs/adr/8093).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub clear_first: Option<f64>,
    /// What stands on its two sides, in `between`'s order: `unit`, `common`, `annex` or `outdoor` (a room of one of the
    /// program's independent units, common areas, annexes or outdoor areas: a garden, a car park), `room` (a room in no
    /// program group), `void` (a shaft, a gallery); none: nothing drawn there. Planlı Alanlar İmar Yönetmeliği 39/1
    /// reads a door by what it enters.
    #[serde(default)]
    pub sides: [Option<String>; 2],
    /// One of its sides is a toilet (WC, Tuvalet by its name or use).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub toilet: bool,
    /// One of its sides is a balcony (Balkon by its name or use).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub balcony: bool,
    /// One of its sides is an elevator's shaft: an elevator's landing door (docs/adr/8087; BYKHY 62/4, 62/6, 62/7).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub elevator: bool,
    /// One of its sides is a passage: a corridor, a hall, an entrance (Koridor, Hol, Antre, Giriş, Geçit), a lobby or a
    /// landing (Lobi, Sahanlık) or a stair's room (docs/adr/8087: BYKHY 62/4 lets an elevator open onto these alone).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub passage: bool,
}

/// A storey and what stands on it. A room is one that is neither a void (a shaft, a gallery) nor outdoors, counted by
/// its program group's kind and, apart, as a technical or a lived-in room; an elevator shaft is a void and counted with
/// the elevators.
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
#[serde(rename_all = "camelCase")]
pub struct StoreyHolds {
    /// The storey's id (`architecture.levels[].id`).
    pub level: String,
    /// Its name.
    pub name: String,
    /// Its elevation over ±0.00, m.
    pub elevation: f64,
    /// Its height floor to floor (döşeme üst kotundan döşeme üst kotuna): its own, else up to the storey over it, m; none
    /// when neither is known (docs/adr/8083).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<f64>,
    /// Rooms of an independent unit on it.
    pub units: u32,
    /// Rooms of a common area on it that are not technical rooms.
    pub common: u32,
    /// Rooms of an annex (eklenti, a place allocated to an independent unit) on it.
    pub annexes: u32,
    /// Rooms in no program group on it that are not technical rooms.
    pub other: u32,
    /// Technical rooms on it, whatever their group: a heat centre, a boiler room, an electricity room, a water tank, a
    /// climate plant, a generator room, a pump or a transformer room, by their name or use.
    pub technical: u32,
    /// Rooms of a dwelling's living kind on it, whatever their group (a living room, a bedroom, a kitchen, by their name
    /// or use): lived in.
    pub inhabited: u32,
    /// Bedrooms on it (Yatak, Ebeveyn, Çocuk, Oda N by their name or use), whatever their group: a drawing without a
    /// program is a dwelling's by them (docs/adr/8083).
    #[serde(default)]
    pub bedrooms: u32,
    /// Rooms on it of an independent unit whose nature is a dwelling's (Daire, Mesken, Konut) or not given: a storey of
    /// homes, as PAİY 28/1-b reads a mixed area's storeys (docs/adr/8085).
    #[serde(default)]
    pub dwelling_rooms: u32,
    /// It is a mezzanine (its name says Asma kat): part of the ground storey it stands in (PAİY 4/1-d, 28/1-a, b, ç's
    /// “asma katlı zemin kat”; docs/adr/8085).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub mezzanine: bool,
    /// Elevators on it: elevator shafts (a space named Asansör, not its lobby) and Asansör blocks standing on it.
    pub elevators: u32,
    /// The rooms counted on it, by their persistent ids in order: what a finding about the storey names.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub rooms: Vec<String>,
    /// An escape stair's room stands on it: the stair serves it (BYKHY 47/3 counts them).
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub stair: bool,
}

impl BuildingFacts {
    /// These facts in the section's order and rounding. Doing it again changes nothing.
    pub fn normalized(mut self) -> BuildingFacts {
        for s in &mut self.storeys {
            s.elevation = round(s.elevation, LENGTH);
            s.height = s.height.map(|h| round(h, LENGTH));
            s.rooms.sort();
            s.rooms.dedup();
        }
        self.storeys
            .sort_by(|a, b| a.elevation.total_cmp(&b.elevation).then_with(|| a.level.cmp(&b.level)));
        self.local = self.local.filter(|l| !l.is_empty());
        for n in &mut self.plan_notes {
            n.value = round(n.value, LENGTH);
        }
        for r in &mut self.rooms {
            r.persons = round(r.persons, LENGTH);
        }
        self.rooms.sort_by(|a, b| a.uid.cmp(&b.uid));
        for d in &mut self.doors {
            d.clear = d.clear.map(|c| round(c, LENGTH));
            d.height = d.height.map(|h| round(h, LENGTH));
            d.clear_first = d.clear_first.map(|c| round(c, LENGTH));
            for l in &mut d.leaves {
                *l = round(*l, LENGTH);
            }
        }
        self.doors.sort_by(|a, b| a.uid.cmp(&b.uid));
        for l in &mut self.limits {
            l.rooms.sort();
            l.area = round(l.area, AREA);
            l.allowed = round(l.allowed, AREA);
            l.over = round(l.over, AREA);
        }
        self.uses.retain(|u| !u.trim().is_empty());
        for u in &mut self.under_stairs {
            u.low = round(u.low, AREA);
            u.open = round(u.open, AREA);
            for r in &mut u.rooms {
                r.area = round(r.area, AREA);
            }
            u.rooms.sort_by(|a, b| a.uid.cmp(&b.uid));
        }
        self.under_stairs.sort_by(|a, b| a.stair.cmp(&b.stair));
        for x in &mut self.spaces {
            x.area = round(x.area, AREA);
        }
        self.spaces.sort_by(|a, b| a.uid.cmp(&b.uid));
        for x in &mut self.separations {
            x.open = round(x.open, LENGTH);
            for e in x.walls.iter_mut().chain(x.openings.iter_mut()) {
                e.length = round(e.length, LENGTH);
            }
            x.walls.sort_by(|a, b| a.uid.cmp(&b.uid));
            x.openings.sort_by(|a, b| a.uid.cmp(&b.uid));
        }
        self.separations.sort_by(|a, b| a.rooms.cmp(&b.rooms));
        for l in &mut self.links {
            l.area = round(l.area, AREA);
        }
        self.links.sort_by(|a, b| (&a.below, &a.uid, &a.kind).cmp(&(&b.below, &b.uid, &b.kind)));
        self.elements.sort_by(|a, b| a.uid.cmp(&b.uid));
        self.height = round(self.height, LENGTH);
        self.basement_depth = round(self.basement_depth, LENGTH);
        self.structure_height = round(self.structure_height, LENGTH);
        self.coverings.sort_by(|a, b| a.uid.cmp(&b.uid));
        for x in &mut self.shafts {
            x.open = round(x.open, LENGTH);
            for e in x.walls.iter_mut().chain(x.openings.iter_mut()) {
                e.length = round(e.length, LENGTH);
            }
            x.walls.sort_by(|a, b| a.uid.cmp(&b.uid));
            x.openings.sort_by(|a, b| a.uid.cmp(&b.uid));
        }
        self.shafts.sort_by(|a, b| a.uid.cmp(&b.uid));
        self
    }

    /// Whether there is nothing to say: no storey.
    pub fn is_empty(&self) -> bool {
        self.storeys.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn storey(level: &str, elevation: f64) -> StoreyHolds {
        StoreyHolds { level: level.into(), name: level.into(), elevation, ..StoreyHolds::default() }
    }

    #[test]
    fn storeys_are_lowest_first_and_their_elevations_rounded() {
        let facts = BuildingFacts {
            storeys: vec![storey("k1", 3.000_4), storey("b1", -3.000_6), storey("k0", 0.0), storey("a", 0.0)],
            units: 6,
            dwellings: 6,
            local: Some(String::new()),
            ..BuildingFacts::default()
        }
        .normalized();
        let order: Vec<&str> = facts.storeys.iter().map(|s| s.level.as_str()).collect();
        assert_eq!(order, ["b1", "a", "k0", "k1"]);
        assert_eq!(facts.storeys[0].elevation, -3.001);
        assert_eq!(facts.storeys[3].elevation, 3.0);
        assert_eq!(facts.local, None);
        assert_eq!(facts.clone().normalized(), facts);
        let json = serde_json::to_string(&facts).unwrap_or_default();
        assert!(json.starts_with(r#"{"storeys":[{"level":"b1","name":"b1","elevation":-3.001,"units":0"#), "{json}");
        assert!(!json.contains("local"), "{json}");
    }
}
