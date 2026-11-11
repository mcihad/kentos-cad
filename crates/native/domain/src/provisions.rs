//! Otopark ve sığınak hesapları (docs/adr/8036; ADV-22, M36): what the building's independent units ask by the parking
//! and the shelter regulations, beside what the drawing gives.
//!
//! - **Otopark (Otopark Yönetmeliği 8, Ek-1):** a dwelling by its gross area (BB brüt, 8/3): under 80 m² one place for
//!   three, 80–120 m² one for two, 120–180 m² one, over 180 m² two (Ek-1 as amended RG 25.3.2021/31434, its annex
//!   read through the ministry's notice of 15.3.2021); a shop, a store or a bank one for each 50 m², at least one each
//!   (RG 16.6.2022/31868); an office one for each 50 m². A unit of another nature is listed as not read. The sum is
//!   rounded up (8/4). The drawing's places: the rooms named Otopark or Garaj, their net area over a car's 20 m², its
//!   manoeuvre in (4/1-ç). With twenty places or more, 5 % of them (one at least) for electric cars (5/1-j).
//! - **Otopark yerleri (docs/adr/8045 §6):** where the library's Otopark yeri blocks are drawn, they are the drawing's
//!   places (4/1-ç: "Park yerleri … vaziyet ya da kat planında gösterilir"), each of its Tür: Binek, Engelli, Şarjlı,
//!   Kamyonet. In a public building (PAİY 4/jjjj: büro, iş hanı, çarşı …; read as units of an office's or a shop's use
//!   and no dwelling) one in every twenty, one at least, for the disabled (4/1-ı; a fraction rounded up, as 8/4).
//! - **Sığınak (Sığınak Yönetmeliği 7, 8 as amended RG 7.11.2025/33070):** a dwelling's people by its rooms (its living
//!   rooms and bedrooms): one room two, two rooms three, three or more four; another unit's its gross area over 20. A
//!   net square metre for each (8/a). Not asked in a building of only dwellings with fewer than ten units (7/a), in a
//!   building of other uses under 1500 m² (7/b), in a mixed one with fewer than ten dwellings under 1500 m² (7/c). The
//!   drawing's shelter: the rooms named Sığınak, their net area. A toilet and a washbasin each for women and men for
//!   every hundred people, one more past fifty; two exits and a kitchen niche of 2.30 m² in a shelter over 100 m².
//!
//! The gross area is the regulation areas' (docs/adr/8029); the area a shop, an office or a shelter counts by is read
//! as that gross too (the floor area ratio's own area is the zoning table's, docs/adr/8030). A table is the drawing's
//! measure, not an approval; a municipality's parking plan may ask more (8/2).

use kentos_contracts::{CellRange, CellStyle, Entity, TableAlign, UnitKind};

use crate::Document;
use crate::identity::Slot;
use crate::room_names::{RoomKind, dwelling_nature, has, kind_of, words};
use crate::space_follow::SpaceSearch;
use crate::zoning::{OVER_COLOR, ZoningTable};

/// Otopark Yönetmeliği 4/1-ç: a car's parking unit, its manoeuvre in, m².
pub const PARKING_UNIT: f64 = 20.0;

/// A parking place's kind (docs/adr/8045 §6): the Otopark yeri block's Tür.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum StallKind {
    Car,
    Disabled,
    Charging,
    Van,
}

impl StallKind {
    /// Its least short and long sides, m: Otopark Yönetmeliği 5/1-h-8 (a van 3.00 × 6.00) and 5/1-h-9 (the long side
    /// 4.90, the short 3.50 for the disabled, else 2.40).
    pub fn least(self) -> (f64, f64) {
        match self {
            StallKind::Car | StallKind::Charging => (2.40, 4.90),
            StallKind::Disabled => (3.50, 4.90),
            StallKind::Van => (3.00, 6.00),
        }
    }

    /// The kind by its block's Tür state (Turkish case folded); a car's for another.
    pub fn of(state: &str) -> Self {
        let w = words(Some(state), None);
        if has(&w, "ENGELLI") {
            StallKind::Disabled
        } else if has(&w, "SARJ") {
            StallKind::Charging
        } else if has(&w, "KAMYONET") || has(&w, "PANELVAN") {
            StallKind::Van
        } else {
            StallKind::Car
        }
    }
}

/// A parking place drawn: an Otopark yeri block's insert, its sizes in metres.
#[derive(Clone, Debug, PartialEq)]
pub struct Stall {
    pub slot: Slot,
    pub level: Option<String>,
    /// Its middle in the drawing.
    pub at: kentos_contracts::Vec2,
    pub kind: StallKind,
    /// En, along the block's x.
    pub width: f64,
    /// Boy, along its y.
    pub length: f64,
    /// The insert's factors along x and y.
    pub scale: (f64, f64),
    /// Where the insert puts its block's points: the place's outline is its block's (0, 0)–(En, Boy).
    pub placement: kentos_geometry_core::geom::affine::Affine,
}

impl Stall {
    /// Its outline in the drawing were its En and Boy `width` and `length` metres: its block's (0, 0)–(En, Boy) as the
    /// insert places it, counter-clockwise from its front left corner.
    pub fn outline(&self, width: f64, length: f64) -> [kentos_geometry_core::Vec2; 4] {
        use kentos_geometry_core::Vec2 as V;
        let (w, l) = (
            width / self.scale.0.max(1e-12),
            length / self.scale.1.max(1e-12),
        );
        let at = |x: f64, y: f64| {
            kentos_geometry_core::geom::affine::apply(&self.placement, V::new(x, y))
        };
        [at(0.0, 0.0), at(w, 0.0), at(w, l), at(0.0, l)]
    }
}

/// The Otopark yeri block's parameters (docs/adr/8045 §6).
pub const STALL_WIDTH: &str = "En";
pub const STALL_LENGTH: &str = "Boy";
pub const STALL_KIND: &str = "Tür";

/// Whether a block is a parking place's: its card's category under Otopark.
pub fn is_stall(def: &kentos_contracts::BlockDefinition) -> bool {
    def.card
        .as_ref()
        .and_then(|c| c.category.as_deref())
        .and_then(|c| c.split('/').next())
        .is_some_and(|c| c.trim() == "Otopark")
}

/// An independent unit's Ek-1 row.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParkingUse {
    Dwelling,
    Shop,
    Office,
}

/// A unit's parking need.
#[derive(Clone, Debug, PartialEq)]
pub struct ParkingRow {
    pub unit: String,
    pub number: String,
    pub nature: Option<String>,
    pub use_: Option<ParkingUse>,
    /// Its gross area, m².
    pub base: f64,
    /// Ek-1's words: “80–120 m²: 2 daireye 1”.
    pub ratio: &'static str,
    pub need: f64,
}

/// The building's parking (docs/adr/8036).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ParkingReport {
    pub rows: Vec<ParkingRow>,
    pub need: f64,
    /// The need rounded up (8/4).
    pub required: u32,
    /// The rooms named Otopark or Garaj, their net area, m².
    pub area: f64,
    pub places: Vec<Slot>,
    /// The drawing's places: the parking places drawn (docs/adr/8045 §6), else the rooms' area over 20 m².
    pub available: u32,
    /// The places for electric cars 5/1-j asks.
    pub charging: u32,
    /// The parking places drawn: the Otopark yeri blocks.
    pub stalls: Vec<Stall>,
    /// A public building's (PAİY 4/jjjj): its units of an office's or a shop's use, none a dwelling.
    pub public: bool,
    /// The places 4/1-ı keeps for the disabled in a public building's drawn places: one in twenty, one at least.
    pub disabled: u32,
}

impl ParkingReport {
    pub fn short(&self) -> bool {
        self.required > self.available
    }

    /// The drawn places of a kind.
    pub fn drawn(&self, kind: StallKind) -> u32 {
        self.stalls.iter().filter(|s| s.kind == kind).count() as u32
    }
}

/// A unit's shelter people.
#[derive(Clone, Debug, PartialEq)]
pub struct ShelterRow {
    pub unit: String,
    pub number: String,
    pub nature: Option<String>,
    /// What they are counted by: “3 oda”, “120.00 m² ÷ 20”.
    pub base: String,
    pub persons: u32,
}

/// The building's shelter (docs/adr/8036).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShelterReport {
    pub rows: Vec<ShelterRow>,
    pub persons: u32,
    /// A net square metre for each (8/a), m².
    pub area_needed: f64,
    /// The rooms named Sığınak and their net areas, m².
    pub shelters: Vec<(Slot, f64)>,
    pub area: f64,
    /// Why none is asked (7/a–c); none when one is.
    pub exempt: Option<&'static str>,
    /// Toilets and washbasins, each for women and for men (8/a).
    pub toilets: u32,
}

impl ShelterReport {
    pub fn short(&self) -> bool {
        self.exempt.is_none() && self.persons > 0 && self.area + 1e-6 < self.area_needed
    }
}

/// A unit's Ek-1 row by its nature: a dwelling (or no nature given), a shop (dükkân, mağaza, banka, market), an office
/// (büro, ofis); none for another.
pub fn parking_use(nature: Option<&str>) -> Option<ParkingUse> {
    if dwelling_nature(nature) {
        return Some(ParkingUse::Dwelling);
    }
    let w = words(nature, None);
    if ["DUKKAN", "MAGAZA", "BANKA", "MARKET"]
        .iter()
        .any(|p| has(&w, p))
    {
        Some(ParkingUse::Shop)
    } else if ["BURO", "OFIS"].iter().any(|p| has(&w, p)) {
        Some(ParkingUse::Office)
    } else {
        None
    }
}

/// Ek-1 (RG 25.3.2021/31434): a dwelling's places by its gross area.
pub fn dwelling_parking(gross: f64) -> (f64, &'static str) {
    if gross < 80.0 {
        (1.0 / 3.0, "80 m²'den küçük: 3 daireye 1")
    } else if gross <= 120.0 {
        (0.5, "80–120 m²: 2 daireye 1")
    } else if gross <= 180.0 {
        (1.0, "120–180 m²: daireye 1")
    } else {
        (2.0, "180 m²'den büyük: daireye 2")
    }
}

/// Sığınak Yönetmeliği 8/a: a dwelling's people by its rooms.
pub fn dwelling_persons(rooms: usize) -> u32 {
    match rooms {
        0 | 1 => 2,
        2 => 3,
        _ => 4,
    }
}

/// Toilets and washbasins each for women and for men: one for each hundred people, one more past fifty (8/a).
pub fn shelter_toilets(persons: u32) -> u32 {
    persons / 100 + u32::from(persons % 100 > 50)
}

fn named(name: Option<&str>, usage: Option<&str>, prefixes: &[&str]) -> bool {
    let w = words(name, usage);
    prefixes.iter().any(|p| has(&w, p))
}

impl Document {
    /// The rooms whose name or use begins with one of `prefixes` (not voids): each with its net area.
    fn rooms_named(&self, prefixes: &[&str]) -> Vec<(Slot, f64)> {
        let mut search = SpaceSearch::new(self);
        self.entities()
            .filter_map(|e| {
                let Entity::Space(s) = e else { return None };
                let x = &s.space;
                if x.void
                    || x.broken.is_some()
                    || !named(x.name.as_deref(), x.usage.as_deref(), prefixes)
                {
                    return None;
                }
                let area = search.net_area(x).unwrap_or_else(|| {
                    kentos_geometry_core::geom::space::area(&crate::arch_values::core_areas(
                        &x.parts,
                    ))
                });
                Some((Slot(s.base.id), area))
            })
            .collect()
    }

    /// The parking places drawn (docs/adr/8045 §6): every insert of a block of the Otopark category, its En and Boy (the
    /// insert's values, else their defaults) times its factors, its Tür its kind.
    pub fn parking_stalls(&self) -> Vec<Stall> {
        use kentos_contracts::block_dynamic::ParamValue;
        self.entities()
            .filter_map(|e| {
                let Entity::Insert(i) = e else { return None };
                let def = self.blocks().iter().find(|b| b.id == i.block)?;
                if !is_stall(def) {
                    return None;
                }
                let d = def.dynamic.as_ref()?;
                let value = |name: &str| {
                    let p = d.parameters.iter().find(|p| p.name() == name)?;
                    i.parameters
                        .get(name)
                        .cloned()
                        .or_else(|| d.type_values.get(name).cloned())
                        .or_else(|| p.default_value())
                };
                let number = |name: &str| match value(name) {
                    Some(ParamValue::Number(x)) if x.is_finite() => Some(x),
                    _ => None,
                };
                let (sx, sy) = (i.scale.abs(), i.y_scale.unwrap_or(i.scale).abs());
                let placement = kentos_geometry_core::block::insert_matrix_xy(
                    kentos_geometry_core::Vec2::new(def.base.x, def.base.y),
                    kentos_geometry_core::Vec2::new(i.p.x, i.p.y),
                    (i.scale, i.y_scale.unwrap_or(i.scale)),
                    i.rotation,
                    i.mirror,
                );
                let kind = match value(STALL_KIND) {
                    Some(ParamValue::Text(t)) => StallKind::of(&t),
                    _ => StallKind::Car,
                };
                Some(Stall {
                    slot: Slot(i.base.id),
                    level: i.arch.level.clone(),
                    at: self.fixture_middle(i),
                    kind,
                    width: number(STALL_WIDTH)? * sx,
                    length: number(STALL_LENGTH)? * sy,
                    scale: (sx, sy),
                    placement,
                })
            })
            .collect()
    }

    /// The rooms named Otopark or Garaj (docs/adr/8036): each with its net area and the parking places drawn in it
    /// (on its storey, their middles inside it).
    pub fn parking_rooms(&self) -> Vec<(Slot, f64, Vec<Slot>)> {
        let stalls = self.parking_stalls();
        self.rooms_named(&["OTOPARK", "GARAJ"])
            .into_iter()
            .map(|(slot, area)| {
                let inside = match self.get(slot) {
                    Some(Entity::Space(s)) => {
                        let areas = crate::arch_values::core_areas(&s.space.parts);
                        stalls
                            .iter()
                            .filter(|t| {
                                t.level == s.space.arch.level
                                    && areas.iter().any(|a| {
                                        kentos_geometry_core::geom::region::inside_area(
                                            a,
                                            kentos_geometry_core::Vec2::new(t.at.x, t.at.y),
                                        )
                                    })
                            })
                            .map(|t| t.slot)
                            .collect()
                    }
                    _ => Vec::new(),
                };
                (slot, area, inside)
            })
            .collect()
    }

    /// Otopark hesabı (docs/adr/8036): the units' need by Ek-1 against the drawing's places.
    pub fn parking_report(&self) -> ParkingReport {
        let program = self.program();
        let areas = self.paiy_areas();
        let mut rows = Vec::new();
        for u in program.units.iter().filter(|u| u.kind == UnitKind::Unit) {
            let base = areas
                .iter()
                .find(|(id, _)| *id == u.id)
                .map_or(0.0, |(_, g)| g.gross);
            if base <= 0.0 {
                continue;
            }
            let use_ = parking_use(u.nature.as_deref());
            let (need, ratio) = match use_ {
                Some(ParkingUse::Dwelling) => dwelling_parking(base),
                Some(ParkingUse::Shop) => {
                    ((base / 50.0).max(1.0), "50 m²'ye 1, bölüm başına en az 1")
                }
                Some(ParkingUse::Office) => (base / 50.0, "50 m²'ye 1"),
                None => (0.0, "Ek-1'in bu satırı okunmuyor"),
            };
            rows.push(ParkingRow {
                unit: u.id.clone(),
                number: u.number.clone(),
                nature: u.nature.clone(),
                use_,
                base,
                ratio,
                need,
            });
        }
        let need: f64 = rows.iter().map(|r| r.need).sum();
        let required = (need - 1e-9).ceil().max(0.0) as u32;
        let rooms = self.rooms_named(&["OTOPARK", "GARAJ"]);
        // An empty sum is −0 in floating point: written as 0.
        let area: f64 = rooms.iter().map(|(_, a)| a).sum::<f64>() + 0.0;
        let stalls = self.parking_stalls();
        // The places drawn are the drawing's places (docs/adr/8045 §6); without them the rooms' area tells.
        let available = if stalls.is_empty() {
            (area / PARKING_UNIT + 1e-9).floor() as u32
        } else {
            stalls.len() as u32
        };
        let charging = if required >= 20 {
            ((required as f64 * 0.05) - 1e-9).ceil().max(1.0) as u32
        } else {
            0
        };
        let public = !rows.is_empty()
            && rows.iter().all(|r| r.use_ != Some(ParkingUse::Dwelling))
            && rows
                .iter()
                .any(|r| matches!(r.use_, Some(ParkingUse::Shop | ParkingUse::Office)));
        let disabled = if public && !stalls.is_empty() {
            stalls.len().div_ceil(20).max(1) as u32
        } else {
            0
        };
        ParkingReport {
            rows,
            need,
            required,
            area,
            places: rooms.into_iter().map(|(s, _)| s).collect(),
            available,
            charging,
            stalls,
            public,
            disabled,
        }
    }

    /// Sığınak hesabı (docs/adr/8036): the units' people by Sığınak Yönetmeliği 8/a against the drawing's shelter.
    pub fn shelter_report(&self) -> ShelterReport {
        let program = self.program();
        let areas = self.paiy_areas();
        let mut rows = Vec::new();
        let (mut dwellings, mut other_area, mut total_area) = (0usize, 0.0, 0.0);
        for u in program.units.iter().filter(|u| u.kind == UnitKind::Unit) {
            let gross = areas
                .iter()
                .find(|(id, _)| *id == u.id)
                .map_or(0.0, |(_, g)| g.gross);
            total_area += gross;
            if dwelling_nature(u.nature.as_deref()) {
                dwellings += 1;
                let rooms = self
                    .entities()
                    .filter(|e| {
                        matches!(e, Entity::Space(s) if s.space.unit.as_deref() == Some(u.id.as_str())
                            && !s.space.void
                            && matches!(kind_of(s.space.name.as_deref(), s.space.usage.as_deref()),
                                Some(RoomKind::Living | RoomKind::Bedroom)))
                    })
                    .count();
                rows.push(ShelterRow {
                    unit: u.id.clone(),
                    number: u.number.clone(),
                    nature: u.nature.clone(),
                    base: format!("{} oda", rooms.max(1)),
                    persons: dwelling_persons(rooms),
                });
            } else if gross > 0.0 {
                other_area += gross;
                rows.push(ShelterRow {
                    unit: u.id.clone(),
                    number: u.number.clone(),
                    nature: u.nature.clone(),
                    base: format!(
                        "{} m² ÷ 20",
                        crate::text_fields::number_text(gross, 1.0, 2, false)
                    ),
                    persons: (gross / 20.0 - 1e-9).ceil().max(0.0) as u32,
                });
            }
        }
        let others = rows.len() - dwellings;
        let exempt = if rows.is_empty() {
            None
        } else if others == 0 && dwellings < 10 {
            Some("10'dan az bağımsız bölümlü, yalnız konut kullanımlı yapı (7/a)")
        } else if dwellings == 0 && other_area < 1500.0 {
            Some("emsale konu alanı 1500 m²'den az konut dışı yapı (7/b)")
        } else if dwellings > 0 && others > 0 && dwellings < 10 && total_area < 1500.0 {
            Some("10'dan az konutlu, emsale konu alanı 1500 m²'den az karma yapı (7/c)")
        } else {
            None
        };
        let persons: u32 = rows.iter().map(|r| r.persons).sum();
        let shelters = self.rooms_named(&["SIGINAK"]);
        let area = shelters.iter().map(|(_, a)| a).sum::<f64>() + 0.0;
        ShelterReport {
            rows,
            persons,
            area_needed: persons as f64,
            shelters,
            area,
            exempt,
            toilets: shelter_toilets(persons),
        }
    }

    /// Otopark hesabı's rows (docs/adr/8036): each unit's need, the total rounded up, the drawing's places (in the
    /// warning colour when fewer) and the electric cars' places.
    pub fn parking_table(&self) -> ZoningTable {
        let r = self.parking_report();
        let area = |v: f64| self.arch_value_text(&crate::arch_values::ArchValue::Area(v), false);
        let number = |v: f64| crate::text_fields::number_text(v, 1.0, 2, false);
        let mut rows: Vec<Vec<String>> = vec![vec![
            "Otopark hesabı".into(),
            "Brüt alan".into(),
            "Ek-1".into(),
            "Gereken".into(),
        ]];
        let mut styles = Vec::new();
        for row in &r.rows {
            let who = match &row.nature {
                Some(n) => format!("“{}” {n}", row.number),
                None => format!("“{}”", row.number),
            };
            rows.push(vec![
                who,
                area(row.base),
                row.ratio.into(),
                if row.use_.is_some() {
                    number(row.need)
                } else {
                    "—".into()
                },
            ]);
        }
        if r.rows.is_empty() {
            rows.push(vec![
                "Bina programında bölüm yok".into(),
                String::new(),
                String::new(),
                "—".into(),
            ]);
        }
        rows.push(vec![
            "Toplam (kesir bir üste, 8/4)".into(),
            String::new(),
            String::new(),
            r.required.to_string(),
        ]);
        styles.push(style(rows.len() - 1, 0, 4, true, false));
        // The places drawn (docs/adr/8045 §6), else the rooms' area over 20 m².
        if r.stalls.is_empty() {
            rows.push(vec![
                "Çizimdeki otopark".into(),
                area(r.area),
                "20 m²'ye 1 (4/1-ç)".into(),
                r.available.to_string(),
            ]);
        } else {
            rows.push(vec![
                "Çizilen otopark yeri".into(),
                if r.area > 0.0 {
                    area(r.area)
                } else {
                    String::new()
                },
                "Otopark yeri blokları (4/1-ç)".into(),
                r.available.to_string(),
            ]);
        }
        if r.short() {
            styles.push(style(rows.len() - 1, 3, 1, false, true));
        }
        if r.disabled > 0 {
            let drawn = r.drawn(StallKind::Disabled);
            rows.push(vec![
                format!("Engelli park yeri (çizimde {drawn})"),
                String::new(),
                "20 yerde 1, en az 1 (4/1-ı)".into(),
                r.disabled.to_string(),
            ]);
            if drawn < r.disabled {
                styles.push(style(rows.len() - 1, 0, 4, false, true));
            }
        }
        if r.charging > 0 {
            let drawn = r.drawn(StallKind::Charging);
            rows.push(vec![
                if r.stalls.is_empty() {
                    "Elektrikli araç şarj yeri".into()
                } else {
                    format!("Elektrikli araç şarj yeri (çizimde {drawn})")
                },
                String::new(),
                "%5, en az 1 (5/1-j)".into(),
                r.charging.to_string(),
            ]);
            if !r.stalls.is_empty() && drawn < r.charging {
                styles.push(style(rows.len() - 1, 0, 4, false, true));
            }
        }
        ZoningTable {
            rows,
            aligns: vec![
                TableAlign::Left,
                TableAlign::Right,
                TableAlign::Left,
                TableAlign::Right,
            ],
            styles,
        }
    }

    /// Sığınak hesabı's rows (docs/adr/8036): each unit's people, the total and the net area it asks, the drawing's
    /// shelter (in the warning colour when smaller), the toilets and the exits, or why none is asked.
    pub fn shelter_table(&self) -> ZoningTable {
        let r = self.shelter_report();
        let area = |v: f64| self.arch_value_text(&crate::arch_values::ArchValue::Area(v), false);
        let mut rows: Vec<Vec<String>> = vec![vec![
            "Sığınak hesabı".into(),
            "Esas".into(),
            "Kişi".into(),
            "Net alan".into(),
        ]];
        let mut styles = Vec::new();
        for row in &r.rows {
            let who = match &row.nature {
                Some(n) => format!("“{}” {n}", row.number),
                None => format!("“{}”", row.number),
            };
            rows.push(vec![
                who,
                row.base.clone(),
                row.persons.to_string(),
                String::new(),
            ]);
        }
        if r.rows.is_empty() {
            rows.push(vec![
                "Bina programında bölüm yok".into(),
                String::new(),
                "—".into(),
                String::new(),
            ]);
        }
        rows.push(vec![
            "Toplam (kişi başına 1 m², 8/a)".into(),
            String::new(),
            r.persons.to_string(),
            area(r.area_needed),
        ]);
        styles.push(style(rows.len() - 1, 0, 4, true, false));
        rows.push(vec![
            "Çizimdeki sığınak".into(),
            format!("{} mahal", r.shelters.len()),
            String::new(),
            area(r.area),
        ]);
        if r.short() {
            styles.push(style(rows.len() - 1, 3, 1, false, true));
        }
        if let Some(why) = r.exempt {
            rows.push(vec![
                "Sığınak aranmaz".into(),
                why.into(),
                String::new(),
                String::new(),
            ]);
        } else {
            rows.push(vec![
                "Tuvalet ve lavabo (100 kişiye)".into(),
                String::new(),
                format!("kadın {} · erkek {}", r.toilets, r.toilets),
                String::new(),
            ]);
            rows.push(vec![
                "Çıkış".into(),
                "100 m²'yi aşan sığınakta 2 (8/c)".into(),
                String::new(),
                if r.area > 100.0 { "2" } else { "1" }.into(),
            ]);
        }
        ZoningTable {
            rows,
            aligns: vec![
                TableAlign::Left,
                TableAlign::Left,
                TableAlign::Right,
                TableAlign::Right,
            ],
            styles,
        }
    }
}

fn style(row: usize, col: u32, cols: u32, bold: bool, over: bool) -> CellStyle {
    CellStyle {
        range: CellRange {
            row: row as u32,
            col,
            rows: 1,
            cols,
        },
        align: None,
        bold: bold.then_some(true),
        color: over.then(|| OVER_COLOR.to_owned()),
        fill: None,
        decimals: None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ek_1_and_the_shelter_people_by_their_numbers() {
        assert_eq!(dwelling_parking(75.0).0, 1.0 / 3.0);
        assert_eq!(dwelling_parking(80.0).0, 0.5);
        assert_eq!(dwelling_parking(120.0).0, 0.5);
        assert_eq!(dwelling_parking(150.0).0, 1.0);
        assert_eq!(dwelling_parking(181.0).0, 2.0);
        assert_eq!(parking_use(Some("Daire")), Some(ParkingUse::Dwelling));
        assert_eq!(parking_use(Some("Dükkân")), Some(ParkingUse::Shop));
        assert_eq!(parking_use(Some("Banka şubesi")), Some(ParkingUse::Shop));
        assert_eq!(parking_use(Some("Büro")), Some(ParkingUse::Office));
        assert_eq!(parking_use(Some("Otel")), None);
        assert_eq!(dwelling_persons(1), 2);
        assert_eq!(dwelling_persons(2), 3);
        assert_eq!(dwelling_persons(5), 4);
        assert_eq!(shelter_toilets(40), 0);
        assert_eq!(shelter_toilets(151), 2);
        assert_eq!(shelter_toilets(250), 2);
    }
}
