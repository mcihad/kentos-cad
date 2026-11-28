//! Mahal renk planı (docs/adr/8176): each room's colour by the project's colour plan (`ArchSettings::color_plan`) and
//! its legend's rows. A value's colour is the same on every storey and in the legend, and does not move when rooms are
//! added: the groups a name reads as have their own colours, and the values a scheme takes as written (a use no group
//! reads, an independent unit) take the palette's in a fixed order (the uses' folded names sorted, the program's
//! units in their order).
//!
//! - **Kullanım:** by the room's use when one is written, else by its name: a void or a shaft, a technical room, then
//!   the room's kind as the regulations read names (`room_names`): the wet rooms, the kitchen, the passages and
//!   stairs, the living room, the bedrooms; an outdoor room (balkon, teras, bahçe). A use none of them reads is its
//!   own row, as written; a room with neither, Diğer.
//! - **Bağımsız bölüm:** the program's unit (an annex its unit's); the common areas one row, the outdoor areas one;
//!   a room in no unit is not filled.
//!
//! The colours are light: a fill under the plan's lines and texts, printed.

use std::collections::BTreeMap;

use kentos_contracts::{BuildingProgram, ColorScheme, Entity, SpaceFields, UnitKind};

use crate::Document;
use crate::room_names::{self, RoomKind};

/// The groups a room's name or use reads as, in the legend's order, and their colours.
const GROUPS: [(&str, &str); 9] = [
    ("Oturma", "#FDB462"),
    ("Yatma", "#BEBADA"),
    ("Mutfak", "#FB8072"),
    ("Islak hacim", "#80B1D3"),
    ("Dolaşım", "#FFFFB3"),
    ("Teknik hacim", "#BC80BD"),
    ("Açık alan", "#B3DE69"),
    ("Şaft ve boşluk", "#D9D9D9"),
    ("Diğer", "#FCCDE5"),
];
const LIVING: usize = 0;
const BEDROOM: usize = 1;
const KITCHEN: usize = 2;
const WET: usize = 3;
const PASSAGE: usize = 4;
const TECHNICAL: usize = 5;
const OUTDOOR: usize = 6;
const SHAFT: usize = 7;
const OTHER: usize = 8;

/// The colours of the values a scheme takes as written, in turn.
const PALETTE: [&str; 10] = [
    "#8DD3C7", "#FFED6F", "#BEBADA", "#FB8072", "#80B1D3", "#FDB462", "#CCEBC5", "#FCCDE5",
    "#BC80BD", "#E5C494",
];
/// Bağımsız bölüm's common areas and outdoor areas.
const COMMON: &str = "#D9D9D9";
const OUTDOOR_AREA: &str = "#B3DE69";

/// The words an outdoor room is called by.
const OUTDOOR_WORDS: [&str; 5] = ["BALKON", "TERAS", "BAHCE", "AVLU", "VERANDA"];

/// A room's place in a colour plan: its legend row's order and words, and its colour (none: not filled).
#[derive(Clone, Debug, PartialEq)]
pub struct Category {
    pub order: (u8, usize),
    pub label: String,
    pub color: Option<&'static str>,
}

/// A legend row: its words and colour, its rooms and their area, m² (as drawn).
#[derive(Clone, Debug, PartialEq)]
pub struct Row {
    pub label: String,
    pub color: Option<&'static str>,
    pub rooms: usize,
    pub area: f64,
}

/// The group a room reads as: by its use when one is written (the use is what the room is for: “Oda 7” used as an
/// archive is no bedroom), else by its name.
fn group(s: &SpaceFields) -> Option<usize> {
    let (name, usage) = match written_use(s) {
        Some(u) => (None, Some(u)),
        None => (s.name.as_deref(), None),
    };
    if s.void || room_names::shaft(name, usage) {
        return Some(SHAFT);
    }
    if room_names::technical(name, usage) {
        return Some(TECHNICAL);
    }
    match room_names::kind_of(name, usage) {
        Some(RoomKind::Toilet | RoomKind::Bath) => return Some(WET),
        Some(RoomKind::Kitchen) => return Some(KITCHEN),
        Some(RoomKind::Corridor) => return Some(PASSAGE),
        Some(RoomKind::Living) => return Some(LIVING),
        Some(RoomKind::Bedroom) => return Some(BEDROOM),
        None => {}
    }
    if room_names::passage(name, usage) || room_names::stair_name(name, usage) {
        return Some(PASSAGE);
    }
    let words = room_names::words(name, usage);
    OUTDOOR_WORDS
        .iter()
        .any(|w| room_names::has(&words, w))
        .then_some(OUTDOOR)
}

/// A use as written, when one is (its words trimmed).
fn written_use(s: &SpaceFields) -> Option<&str> {
    s.usage.as_deref().map(str::trim).filter(|u| !u.is_empty())
}

/// What a scheme needs of the whole drawing: the uses no group reads, folded, each its place in their sorted order and
/// its words as first written (“Arşiv” and “arşiv ” one row).
struct Plan<'a> {
    scheme: ColorScheme,
    program: &'a BuildingProgram,
    uses: BTreeMap<String, (usize, String)>,
}

impl<'a> Plan<'a> {
    fn of(doc: &'a Document, scheme: ColorScheme, program: &'a BuildingProgram) -> Self {
        let mut uses: BTreeMap<String, (usize, String)> = BTreeMap::new();
        if scheme == ColorScheme::Usage {
            for e in doc.entities() {
                if let Entity::Space(s) = e
                    && group(&s.space).is_none()
                    && let Some(u) = written_use(&s.space)
                {
                    uses.entry(room_names::fold(u))
                        .or_insert_with(|| (0, u.to_owned()));
                }
            }
            for (i, v) in uses.values_mut().enumerate() {
                v.0 = i;
            }
        }
        Plan {
            scheme,
            program,
            uses,
        }
    }

    fn category(&self, s: &SpaceFields) -> Category {
        match self.scheme {
            ColorScheme::Usage => {
                if let Some(g) = group(s) {
                    return Category {
                        order: (0, g),
                        label: GROUPS[g].0.to_owned(),
                        color: Some(GROUPS[g].1),
                    };
                }
                match written_use(s) {
                    Some(u) => {
                        let (i, label) = self
                            .uses
                            .get(&room_names::fold(u))
                            .cloned()
                            .unwrap_or_else(|| (0, u.to_owned()));
                        Category {
                            order: (1, i),
                            label,
                            color: Some(PALETTE[i % PALETTE.len()]),
                        }
                    }
                    None => Category {
                        order: (2, 0),
                        label: GROUPS[OTHER].0.to_owned(),
                        color: Some(GROUPS[OTHER].1),
                    },
                }
            }
            ColorScheme::Unit => {
                let program = self.program;
                let unit =
                    s.unit
                        .as_deref()
                        .and_then(|id| program.unit(id))
                        .map(|u| match u.kind {
                            UnitKind::Annex => u
                                .annex_of
                                .as_deref()
                                .and_then(|o| program.unit(o))
                                .unwrap_or(u),
                            _ => u,
                        });
                match unit {
                    Some(u) if u.kind == UnitKind::Common => Category {
                        order: (1, 0),
                        label: "Ortak alanlar".to_owned(),
                        color: Some(COMMON),
                    },
                    Some(u) if u.kind == UnitKind::Outdoor => Category {
                        order: (2, 0),
                        label: "Dış alanlar".to_owned(),
                        color: Some(OUTDOOR_AREA),
                    },
                    Some(u) => {
                        let i = program
                            .units
                            .iter()
                            .filter(|x| x.kind == UnitKind::Unit)
                            .position(|x| x.id == u.id)
                            .unwrap_or(0);
                        let label = match u.name.as_deref().map(str::trim).filter(|n| !n.is_empty())
                        {
                            Some(name) => format!("{} — {name}", u.number),
                            None => u.number.clone(),
                        };
                        Category {
                            order: (0, i),
                            label,
                            color: Some(PALETTE[i % PALETTE.len()]),
                        }
                    }
                    None => Category {
                        order: (3, 0),
                        label: "Bölüme atanmamış".to_owned(),
                        color: None,
                    },
                }
            }
        }
    }
}

impl Document {
    /// The colour plan's scheme, when the project has one.
    pub fn color_plan(&self) -> Option<ColorScheme> {
        self.settings().arch.as_ref()?.color_plan.map(|c| c.scheme)
    }

    /// Each room's fill (its object's id, its colour) by the colour plan; none without one.
    pub fn space_fills(&self) -> Vec<(u32, &'static str)> {
        let Some(scheme) = self.color_plan() else {
            return Vec::new();
        };
        let program = self.settings().arch.as_ref().map(|a| &a.program);
        let empty = BuildingProgram::default();
        let plan = Plan::of(self, scheme, program.unwrap_or(&empty));
        self.entities()
            .filter_map(|e| match e {
                Entity::Space(s) => plan.category(&s.space).color.map(|c| (e.base().id, c)),
                _ => None,
            })
            .collect()
    }

    /// The colour plan's legend of the rooms on `level` (every room: none), its rows in their order; none without a
    /// plan.
    pub fn color_legend(&self, level: Option<&str>) -> Vec<Row> {
        let Some(scheme) = self.color_plan() else {
            return Vec::new();
        };
        let program = self.settings().arch.as_ref().map(|a| &a.program);
        let empty = BuildingProgram::default();
        let plan = Plan::of(self, scheme, program.unwrap_or(&empty));
        let mut rows: BTreeMap<((u8, usize), String), Row> = BTreeMap::new();
        for e in self.entities() {
            let Entity::Space(s) = e else { continue };
            if level.is_some() && s.space.arch.level.as_deref() != level {
                continue;
            }
            let c = plan.category(&s.space);
            let area = kentos_geometry_core::geom::space::area(&crate::arch_values::core_areas(
                &s.space.parts,
            ));
            let row = rows
                .entry((c.order, c.label.clone()))
                .or_insert_with(|| Row {
                    label: c.label.clone(),
                    color: c.color,
                    rooms: 0,
                    area: 0.0,
                });
            row.rooms += 1;
            row.area += area;
        }
        rows.into_values().collect()
    }
}

impl Document {
    /// The layers a colour plan's fills cover (docs/adr/8176 §3.4): a room's fill is drawn where its layer is among the
    /// layers, so each layer holding rooms that lies over a layer holding anything but rooms and slabs (an opening, a
    /// stair, a tag, a grid) covers that layer's lines inside its rooms: each such room layer's name with the names of
    /// those under it, in the tree's order.
    pub fn covered_layers(&self) -> Vec<(String, Vec<String>)> {
        let leaves = self.layers().leaves();
        let mut rooms = vec![false; leaves.len()];
        let mut other = vec![false; leaves.len()];
        let place: std::collections::HashMap<&str, usize> = leaves
            .iter()
            .enumerate()
            .map(|(i, l)| (l.id.as_str(), i))
            .collect();
        for e in self.entities() {
            let Some(&i) = place.get(e.base().layer_id.as_str()) else {
                continue;
            };
            match e {
                Entity::Space(_) => rooms[i] = true,
                Entity::Slab(_) => {}
                _ => other[i] = true,
            }
        }
        (0..leaves.len())
            .filter(|&i| rooms[i] && self.layers().is_visible(&leaves[i].id))
            .filter_map(|i| {
                let under: Vec<String> = (i + 1..leaves.len())
                    .filter(|&j| other[j] && self.layers().is_visible(&leaves[j].id))
                    .map(|j| leaves[j].name.clone())
                    .collect();
                (!under.is_empty()).then(|| (leaves[i].name.clone(), under))
            })
            .collect()
    }
}
