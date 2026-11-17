//! Emsal and TAKS (docs/adr/8030; ADV-22, M30): the drawing's floor area ratio and coverage against the parcel's
//! zoning, as Planlı Alanlar İmar Yönetmeliği reads them.
//!
//! - **Each room's gross** is its outer contour (4/ğ-1's rule, room by room): its outer-face outline less every other
//!   room's axis outline on its level, a shared wall's façade corner halved between them.
//! - **How it counts** is its use's brief's (`SpaceBrief::emsal`), else `emsal_by_use` by its name and use; an outdoor
//!   room (a balcony, a terrace) is out within the 30 % (22/1-l); a void (shaft, gallery) is no room.
//! - **Katlar alanı** (4/şş, 21): the rooms counted in; **emsal harici** (22/1) together at most 30 % of the parcel's
//!   emsal area (parcel × emsal, 5/8), what passes goes in; 5/8's own rooms (fire stairs …) out apart.
//! - **Taban alanı** (4/bbbb: "yapıların … parseldeki izdüşümünün kapladığı alan"): the ground storey's rooms' gross
//!   (the storey named Zemin, else the one nearest ±0.00), 5/8's rooms and outdoor rooms out (20/8: fire stairs, open car
//!   parks). With the program's buildings (docs/adr/8072 §4), each building's ground storey (named Zemin among the
//!   levels it counts, else the one nearest its own ±0.00); with several, the area of their rooms' contours together.
//!
//! - **Yapı inşaat alanı** (4/oooo): every room's, balconies, terraces and common areas in, voids out.
//!
//! **Emsal ve TAKS tablosu** (`zoning_table`) writes them as PAİY 57/3 asks the approved project to show them: the
//! parcel, the emsal and the area it allows against the drawing's, the emsal harici allowance, TAKS and the footprint,
//! the construction area, and each storey's area counted in and out.
//!
//! - **Balconies** (22/1-l, docs/adr/8069): an independent unit's open balconies (an outdoor room named or used as a
//!   balcony, of a unit) are out up to 20 % of its net area; what passes counts in.
//! - **The stair hall** (22/1-l, docs/adr/8079): a room named or used as a stair (not the room under one) that is no
//!   independent unit's own is out within the 30 %, its landings with it.
//! - **Caretaker flats** (22/1-ç, docs/adr/8079): a room named or used as Kapıcı, or of a program group named or of
//!   the nature Kapıcı, is out; together at most 70 m², what passes counts in.
//! - **Storage annexes** (22/1-k): a room of an annex group of a unit, named or used as a store or of the nature Depo,
//!   on a storey under the ground (its top at or below its building's ±0.00, where the model's graded ground lies) is
//!   out up to 50 % of its unit's net area; what passes counts in. Elsewhere a store counts in.
//! - **The library** (22/1-f): the rooms named or used as Kütüphane together at most 50 m²; what passes counts in.
//! - **A traditional cut stone masonry building's thick stone walls** (5/8, 20/8-j, docs/adr/8095; `masonry`): the
//!   load-bearing stone walls' parts past 25 cm (exterior) and 12.50 cm (interior) within each room's contour are out
//!   apart from the 30 %, the exterior ones' out of taban alanı too.
//! - **A traditional cut stone masonry building's taşlık, dış sofa and eyvan** (22/1-m, docs/adr/8094): where the
//!   project declares the building so (`ArchSettings::traditional_cut_stone`), a room named or used as Taşlık, Dış sofa
//!   or Eyvan is out within the 30 %; the building's storeys (up to three, the ground one with them) and the room's
//!   place (the garden's, by steps) are the standards package's. Its balconies are 22/1-l's 20 % (22/1-m's is the same).
//!
//! Each limit's excess is shared among its rooms by their areas (the artığı rastgele son mahale verilmez, as for the
//! balconies). The conditions on the building as a whole (whether a caretaker flat is obligatory, 44/1; the library's
//! forty units) are the standards package's (catalog/standards/mimari-mevzuat, bim-plan K10), on the limits' facts.
//!
//! A ratio is the drawing's measure, not an approval; the plan's own notes and 22/1-j's basement rooms are the next
//! step.

use kentos_contracts::{
    ArchLevel, AreaUnit, BuildingProgram, CellRange, CellStyle, EmsalCount, Entity, SpaceMethod, TableAlign, Zoning,
};
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region;

use crate::Document;
use crate::arch_values::core_areas;
use crate::room_names;
use crate::space_follow::SpaceSearch;

mod limits;
mod masonry;

use limits::{buried_levels, names, says_balcony, says_traditional};
pub use limits::{BALCONY_SHARE, CARETAKER_AREA, LIBRARY_AREA, STORAGE_SHARE, ZoningLimit, ZoningLimitUse};
pub use masonry::{EXTERIOR_KEPT, INTERIOR_KEPT};

/// The drawing's floor area ratio and coverage against the parcel's zoning, m² and ratios.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ZoningReport {
    pub parcel: f64,
    pub far_limit: Option<f64>,
    pub coverage_limit: Option<f64>,
    /// The rooms counted in: katlar alanı before the exempt rooms' excess.
    pub included: f64,
    /// Emsal harici within the 30 %.
    pub exempt: f64,
    /// Emsal harici apart from the 30 % (5/8's own).
    pub outside: f64,
    /// 30 % of the parcel's emsal area; none without an emsal.
    pub cap: Option<f64>,
    /// The exempt rooms' area past the cap, counted in.
    pub over: f64,
    /// Katlar alanı: the rooms counted in and the excess.
    pub floors: f64,
    /// Taban alanı.
    pub footprint: f64,
    /// Each ground storey the footprint counts (docs/adr/8072 §4) with its rooms' gross there, in the buildings' order.
    pub grounds: Vec<(String, f64)>,
    /// Each storey's rooms, in the project's order of its storeys; the rooms on none last.
    pub levels: Vec<ZoningLevel>,
    /// The footprint's outline: the ground storey's rooms it counts, their contours together (the site plan's oturum,
    /// docs/adr/8032).
    pub footprint_shape: Vec<Area>,
    /// The independent units' balconies past 20 % of their net areas (22/1-l, docs/adr/8069), counted in `included`.
    pub balcony_over: f64,
    /// The caretaker flats past 70 m² together (22/1-ç, docs/adr/8079), counted in `included`.
    pub caretaker_over: f64,
    /// The units' storage annexes under the ground past 50 % of their net areas (22/1-k), counted in `included`.
    pub storage_over: f64,
    /// The library rooms past 50 m² together (22/1-f), counted in `included`.
    pub library_over: f64,
    /// Each limited use counted out (docs/adr/8079): a unit's balconies, its storage annexes, the caretaker flats, the
    /// library; in that order, a unit's by the program's order of its units.
    pub limits: Vec<ZoningLimitUse>,
    /// Each room as the ratios count it (docs/adr/8069: the model's emsal zones are made of them), in the drawing's
    /// order; voids left out.
    pub rooms: Vec<ZoningRoom>,
    /// A traditional cut stone masonry building's load-bearing stone walls' parts past 25 cm (exterior) and 12.50 cm
    /// (interior), out of katlar alanı apart from the 30 % (5/8, docs/adr/8095): counted in `outside`.
    pub masonry: f64,
    /// Of them, the exterior walls' on the ground storeys the footprint counts, out of taban alanı (20/8-j).
    pub masonry_footprint: f64,
    /// The load-bearing stone walls thicker than 12.50 cm of such a building, by persistent id.
    pub masonry_walls: Vec<String>,
    /// The walls thicker than 12.50 cm of such a building with no load-bearing declaration, by persistent id: their
    /// part counts in.
    pub masonry_undeclared: Vec<String>,
}

/// A room as the ratios count it: its persistent id, its storey, how it counts, its contour's area and shape, the limit
/// of 22/1 it is counted out under, and the part of it past that limit, moved into katlar alanı.
#[derive(Clone, Debug, PartialEq)]
pub struct ZoningRoom {
    pub uid: String,
    pub level: Option<String>,
    pub count: EmsalCount,
    pub gross: f64,
    pub moved: f64,
    pub limit: Option<ZoningLimit>,
    pub shape: Vec<Area>,
    /// The stone walls' part within its contour, out of katlar alanı apart from the 30 % (docs/adr/8095).
    pub walls: f64,
}


/// One storey's rooms as the ratios count them, m².
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ZoningLevel {
    /// The storey's id; none: the rooms on no storey.
    pub level: Option<String>,
    pub included: f64,
    pub exempt: f64,
    pub outside: f64,
}

impl ZoningLevel {
    /// The storey's construction area: every room's.
    pub fn all(&self) -> f64 {
        self.included + self.exempt + self.outside
    }
}

/// Emsal ve TAKS tablosu's cells (docs/adr/8030): the rows, each column's alignment and the bold and warning cells.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ZoningTable {
    pub rows: Vec<Vec<String>>,
    pub aligns: Vec<TableAlign>,
    pub styles: Vec<CellStyle>,
}

/// The colour of a value past its limit in Emsal ve TAKS tablosu.
pub const OVER_COLOR: &str = "#d32f2f";

impl ZoningReport {
    /// One of the program's buildings' share (docs/adr/8072 §4): its yapı inşaat alanı (the storeys it counts, a shared
    /// one only where it is chosen) and its taban alanı (its ground storey's rooms); the buildings' together are the
    /// project's, a shared basement once. `levels` are the project's.
    pub fn of_building(&self, program: &BuildingProgram, levels: &[ArchLevel], building: &str) -> (f64, f64) {
        let mine = program.counted_levels(building);
        let construction = self
            .levels
            .iter()
            .filter(|l| l.level.as_deref().is_some_and(|id| mine.contains(&id)))
            .map(ZoningLevel::all)
            .sum();
        let ground = program.ground_of(building, levels);
        let footprint = self
            .grounds
            .iter()
            .find(|(g, _)| Some(g) == ground.as_ref())
            .map_or(0.0, |(_, a)| *a);
        (construction, footprint)
    }

    /// Emsal: the floors' area over the parcel's.
    pub fn far(&self) -> f64 {
        self.floors / self.parcel
    }

    /// TAKS: the footprint over the parcel's.
    pub fn coverage(&self) -> f64 {
        self.footprint / self.parcel
    }

    /// Whether the floors pass the plan's emsal.
    pub fn far_over(&self) -> bool {
        self.far_limit.is_some_and(|l| self.far() > l + 1e-9)
    }

    /// Whether the footprint passes the plan's TAKS.
    pub fn coverage_over(&self) -> bool {
        self.coverage_limit
            .is_some_and(|l| self.coverage() > l + 1e-9)
    }

    /// Yapı inşaat alanı (4/oooo): every room's.
    pub fn construction(&self) -> f64 {
        self.levels.iter().map(ZoningLevel::all).sum()
    }
}

/// One room as the ratios read it.
struct Room {
    level: Option<String>,
    void: bool,
    count: EmsalCount,
    outdoor: bool,
    gross: Vec<Area>,
    axis: Vec<Area>,
    /// Its independent unit, by the program's unit id (docs/adr/8069).
    unit: Option<String>,
    /// An outdoor room that is a balcony by its name or use (22/1-l), not a terrace.
    balcony: bool,
    /// An indoor room of a caretaker flat (22/1-ç): its name or use says Kapıcı, or its group's name or nature does.
    caretaker: bool,
    /// An indoor library room (22/1-f): its name or use says Kütüphane.
    library: bool,
    /// A traditional cut stone masonry building's taşlık, dış sofa or eyvan (22/1-m, docs/adr/8094).
    traditional: bool,
    /// An indoor storage annex's room on a storey under the ground (22/1-k): the independent unit its annex group is
    /// allocated to.
    storage_of: Option<String>,
    /// Its net area, between its walls' faces (PAİY 4/1-ğ-3).
    net: f64,
    /// Its net outline: the stone walls' mass is its storey's contours less these (docs/adr/8095).
    net_shape: Vec<Area>,
}

fn measured(
    search: &mut SpaceSearch<'_>,
    s: &kentos_contracts::SpaceFields,
    method: SpaceMethod,
) -> Vec<Area> {
    if s.seed.is_some() && s.method != method {
        let mut other = s.clone();
        other.method = method;
        let found = search.found_preserving_legacy(&other);
        if found.broken.is_none() && !found.parts.is_empty() {
            return core_areas(&found.parts);
        }
    }
    core_areas(&s.parts)
}

/// An area's box: x0, y0, x1, y1.
fn bounds(areas: &[Area]) -> Option<[f64; 4]> {
    let mut b = [
        f64::INFINITY,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NEG_INFINITY,
    ];
    for a in areas {
        for r in std::iter::once(&a.outer).chain(&a.holes) {
            for p in &r.pts {
                b = [b[0].min(p.x), b[1].min(p.y), b[2].max(p.x), b[3].max(p.y)];
            }
        }
    }
    b[0].is_finite().then_some(b)
}

impl Document {
    /// The drawing's emsal and TAKS against the parcel's zoning (docs/adr/8030); none without a zoning.
    pub fn zoning_report(&self) -> Option<ZoningReport> {
        let zoning = self.settings().arch.as_ref()?.zoning?;
        Some(self.zoning_measured(Some(zoning)))
    }

    /// The building's yapı inşaat alanı (4/oooo), whether or not the parcel's zoning is given: every room's, as the
    /// report counts it (docs/adr/8063, the dated scope decision's area); none without a room.
    pub fn construction_area(&self) -> Option<f64> {
        self.entities()
            .any(|e| matches!(e, Entity::Space(_)))
            .then(|| self.zoning_measured(None).construction())
    }

    /// The rooms counted against `zoning`; with none, a parcel of 0 (its ratios are not numbers).
    fn zoning_measured(&self, zoning: Option<Zoning>) -> ZoningReport {
        let empty = kentos_contracts::ArchSettings::default();
        let arch = self.settings().arch.as_ref().unwrap_or(&empty);
        let program = &arch.program;
        let buried = buried_levels(arch);
        // A traditional cut stone masonry building's taşlık, dış sofa and eyvan are out (22/1-m, docs/adr/8094).
        let traditional_building = arch.traditional_cut_stone();
        // The places under stairs lower than 1.80 m are out of a unit's net area (4/1-ğ-3, docs/adr/8080).
        let unders = self.under_stairs();
        let mut search = SpaceSearch::new(self);
        let mut rooms = Vec::new();
        let mut uids = Vec::new();
        for (uid, e) in self.entities_with_uids() {
            let Entity::Space(s) = e else { continue };
            uids.push(uid.to_string());
            let x = &s.space;
            let (name, usage) = (x.name.as_deref(), x.usage.as_deref());
            let brief = usage.and_then(|u| program.brief(u));
            let outdoor = brief.is_some_and(|b| b.outdoor);
            // What 22/1 reads a room as (docs/adr/8079): by its own words, else (a room its words count in) by its
            // program group's: a coal room in a storage annex stays a coal room.
            let by_use = kentos_contracts::emsal_by_use(name, usage);
            let group = x.unit.as_deref().and_then(|u| program.unit(u));
            let own = room_names::words(name, usage);
            let theirs = if by_use == EmsalCount::Included {
                group.map_or_else(Vec::new, |g| room_names::words(g.name.as_deref(), g.nature.as_deref()))
            } else {
                Vec::new()
            };
            let indoor = !x.void && !outdoor;
            let caretaker = indoor && names("KAPICI", &own, &theirs);
            let library = indoor && room_names::has(&own, "KUTUPHANE");
            let traditional = traditional_building && !x.void && says_traditional(&own);
            let storage_of = group
                .filter(|g| g.kind == kentos_contracts::UnitKind::Annex)
                .and_then(|g| g.annex_of.clone())
                .filter(|owner| {
                    program.unit(owner).is_some_and(|u| u.kind == kentos_contracts::UnitKind::Unit)
                })
                .filter(|_| {
                    indoor
                        && by_use == EmsalCount::Included
                        && names("DEPO", &own, &theirs)
                        && !room_names::technical(name, usage)
                        && x.arch.level.as_ref().is_some_and(|l| buried.contains(l))
                });
            // A stair hall (22/1-l: "kat ve ara sahanlıkları dâhil açık veya kapalı merdiven evi"): no unit's own.
            let stair_hall = indoor
                && room_names::stair_name(name, usage)
                && group.is_none_or(|g| g.kind != kentos_contracts::UnitKind::Unit);
            let count = match brief.and_then(|b| b.emsal) {
                Some(c) => c,
                None if outdoor => EmsalCount::Exempt,
                None => match by_use {
                    EmsalCount::Included if caretaker || storage_of.is_some() || stair_hall || traditional => {
                        EmsalCount::Exempt
                    }
                    c => c,
                },
            };
            let (net, net_shape) = if x.void {
                (0.0, Vec::new())
            } else {
                let outline = measured(&mut search, x, SpaceMethod::Net);
                let low = crate::under_stair::low_on(&unders, &x.arch.level);
                let all = kentos_geometry_core::geom::space::area(&outline);
                (all - crate::under_stair::taken(&outline, &low).min(all), outline)
            };
            rooms.push(Room {
                level: x.arch.level.clone(),
                void: x.void,
                count,
                outdoor,
                gross: if x.void {
                    Vec::new()
                } else {
                    measured(&mut search, x, SpaceMethod::Gross)
                },
                axis: measured(&mut search, x, SpaceMethod::Centerline),
                unit: x.unit.clone(),
                balcony: outdoor && says_balcony(name, usage),
                caretaker,
                library,
                traditional,
                storage_of,
                net,
                net_shape,
            });
        }
        // Each room's contour on its level, a shared wall's façade corner halved between two rooms.
        let mut gross = vec![0.0; rooms.len()];
        let mut shapes: Vec<Vec<Area>> = vec![Vec::new(); rooms.len()];
        // A traditional cut stone masonry building's stone walls' parts within each room's contour (docs/adr/8095):
        // the exterior walls' and the interior walls', halved where two contours meet as the gross is.
        let stone = if traditional_building { masonry::walls(self) } else { Default::default() };
        let mut outer_part = vec![0.0; rooms.len()];
        let mut inner_part = vec![0.0; rooms.len()];
        let levels: std::collections::BTreeSet<Option<String>> =
            rooms.iter().map(|r| r.level.clone()).collect();
        for level in &levels {
            let here: Vec<usize> = (0..rooms.len())
                .filter(|&i| rooms[i].level == *level)
                .collect();
            let mut cut: Vec<(usize, Vec<Area>, Option<[f64; 4]>)> = Vec::new();
            for &i in &here {
                if rooms[i].void || rooms[i].gross.is_empty() {
                    continue;
                }
                let others: Vec<Area> = here
                    .iter()
                    .filter(|&&j| j != i)
                    .flat_map(|&j| rooms[j].axis.iter().cloned())
                    .collect();
                let own = region::union_areas(&rooms[i].gross);
                let c = if others.is_empty() {
                    own
                } else {
                    region::subtract_areas(&own, &region::union_areas(&others))
                };
                gross[i] = kentos_geometry_core::geom::space::area(&c);
                shapes[i].clone_from(&c);
                let b = bounds(&c);
                cut.push((i, c, b));
            }
            let mut overlaps: Vec<(usize, usize, Vec<Area>)> = Vec::new();
            for a in 0..cut.len() {
                for b in a + 1..cut.len() {
                    let (Some(p), Some(q)) = (cut[a].2, cut[b].2) else {
                        continue;
                    };
                    if p[2] < q[0] || q[2] < p[0] || p[3] < q[1] || q[3] < p[1] {
                        continue;
                    }
                    let both = region::intersect_area_sets(&[cut[a].1.clone(), cut[b].1.clone()]);
                    let o = kentos_geometry_core::geom::space::area(&both);
                    if o > 0.0 {
                        gross[cut[a].0] -= o / 2.0;
                        gross[cut[b].0] -= o / 2.0;
                        overlaps.push((cut[a].0, cut[b].0, both));
                    }
                }
            }
            if let Some(walls) = stone.storeys.get(level) {
                let contours: Vec<Area> = cut.iter().flat_map(|(_, c, _)| c.iter().cloned()).collect();
                let nets: Vec<Area> = here.iter().flat_map(|&i| rooms[i].net_shape.iter().cloned()).collect();
                let (outer, inner) = masonry::parts(walls, &contours, &nets);
                for (part, into) in [(&outer, &mut outer_part), (&inner, &mut inner_part)] {
                    if part.is_empty() {
                        continue;
                    }
                    let within = |a: &[Area]| {
                        kentos_geometry_core::geom::space::area(&region::intersect_area_sets(&[a.to_vec(), part.clone()]))
                    };
                    for (i, c, _) in &cut {
                        into[*i] = within(c);
                    }
                    for (a, b, both) in &overlaps {
                        let o = within(both);
                        into[*a] -= o / 2.0;
                        into[*b] -= o / 2.0;
                    }
                }
            }
        }
        // The stone walls' parts are out apart from the 30 % (5/8, docs/adr/8095): a room counts its contour less its
        // part, in or out.
        let walls_of: Vec<f64> = outer_part.iter().zip(&inner_part).map(|(a, b)| a + b).collect();
        let kept: Vec<f64> = gross.iter().zip(&walls_of).map(|(g, w)| g - w).collect();
        // The limits of 22/1 (docs/adr/8069, 8079): each limited use's part past its limit counted in.
        let (moved, limit_of, used) = limits::apply(&rooms, &kept, &uids, program);
        let parcel = zoning.map_or(0.0, |z| z.parcel_area);
        let mut out = ZoningReport {
            parcel,
            far_limit: zoning.and_then(|z| z.far),
            coverage_limit: zoning.and_then(|z| z.coverage),
            ..Default::default()
        };
        // Each storey's, in the project's order of its storeys, then the rooms on none.
        let order = |l: &Option<String>| match l {
            Some(id) => arch
                .levels
                .iter()
                .position(|x| &x.id == id)
                .unwrap_or(usize::MAX - 1),
            None => usize::MAX,
        };
        for (i, (((r, g), m), limit)) in rooms.iter().zip(&kept).zip(&moved).zip(&limit_of).enumerate() {
            if r.void {
                continue;
            }
            let at = match out.levels.iter().position(|l| l.level == r.level) {
                Some(i) => i,
                None => {
                    out.levels.push(ZoningLevel {
                        level: r.level.clone(),
                        ..Default::default()
                    });
                    out.levels.len() - 1
                }
            };
            let level = &mut out.levels[at];
            match r.count {
                EmsalCount::Included => {
                    out.included += g;
                    level.included += g;
                }
                EmsalCount::Exempt => {
                    // A limited use's part past its limit counts in (22/1-l, -k, -ç, -f).
                    out.exempt += g - m;
                    level.exempt += g - m;
                    out.included += m;
                    level.included += m;
                    match limit {
                        Some(ZoningLimit::Balcony) => out.balcony_over += m,
                        Some(ZoningLimit::Storage) => out.storage_over += m,
                        Some(ZoningLimit::Caretaker) => out.caretaker_over += m,
                        Some(ZoningLimit::Library) => out.library_over += m,
                        Some(ZoningLimit::Traditional) | None => {}
                    }
                }
                EmsalCount::Outside => {}
            }
            // A room out apart from the 30 % (5/8's own) keeps its whole contour there, its walls' part with it.
            let out_apart = if r.count == EmsalCount::Outside { gross[i] } else { walls_of[i] };
            out.outside += out_apart;
            level.outside += out_apart;
            out.masonry += walls_of[i];
        }
        out.masonry_walls = stone.thick;
        out.masonry_undeclared = stone.undeclared;
        out.rooms = rooms
            .iter()
            .enumerate()
            .filter(|(_, r)| !r.void)
            .map(|(i, r)| ZoningRoom {
                uid: uids[i].clone(),
                level: r.level.clone(),
                count: r.count,
                gross: gross[i],
                moved: moved[i],
                limit: limit_of[i],
                shape: shapes[i].clone(),
                walls: walls_of[i],
            })
            .collect();
        out.limits = used;
        out.levels.sort_by_key(|l| order(&l.level));
        out.cap = out.far_limit.map(|f| 0.30 * f * parcel);
        out.over = out.cap.map_or(0.0, |c| (out.exempt - c).max(0.0));
        out.floors = out.included + out.over;
        // The ground storeys: each of the program's buildings' (docs/adr/8072 §4), else the storey named Zemin, else the
        // one nearest ±0.00; with no storeys, every room.
        let mut grounds: Vec<String> = Vec::new();
        for b in &program.buildings {
            if let Some(g) = program.ground_of(&b.id, &arch.levels)
                && !grounds.contains(&g)
            {
                grounds.push(g);
            }
        }
        if grounds.is_empty()
            && let Some(l) = arch
                .levels
                .iter()
                .find(|l| kentos_contracts::is_ground_name(&l.name))
                .or_else(|| arch.levels.iter().min_by(|a, b| a.elevation.abs().total_cmp(&b.elevation.abs())))
        {
            grounds.push(l.id.clone());
        }
        let counted = |r: &Room| {
            !r.void
                && !r.outdoor
                && r.count != EmsalCount::Outside
                && (grounds.is_empty() || r.level.as_ref().is_some_and(|l| grounds.contains(l)))
        };
        // The exterior stone walls' parts are out of taban alanı (20/8-j, docs/adr/8095); the interior walls' are not.
        let footing: Vec<f64> = gross.iter().zip(&outer_part).map(|(g, w)| g - w).collect();
        out.grounds = grounds
            .iter()
            .map(|g| {
                let on: f64 = rooms
                    .iter()
                    .zip(&footing)
                    .filter(|(r, _)| counted(r) && r.level.as_ref() == Some(g))
                    .map(|(_, a)| a)
                    .sum();
                (g.clone(), on)
            })
            .collect();
        out.masonry_footprint = rooms.iter().zip(&outer_part).filter(|(r, _)| counted(r)).map(|(_, w)| w).sum();
        let outline: Vec<Area> = rooms
            .iter()
            .zip(&shapes)
            .filter(|(r, _)| counted(r))
            .flat_map(|(_, s)| s.iter().cloned())
            .collect();
        out.footprint_shape = if outline.is_empty() {
            Vec::new()
        } else {
            region::union_areas(&outline)
        };
        // One ground storey: its rooms' gross, as before; several (blocks, a podium and a tower): the area their
        // contours cover on the parcel together, an overlap once.
        out.footprint = if grounds.len() > 1 {
            out.footprint_shape.iter().map(region::net_area).sum::<f64>() - out.masonry_footprint
        } else {
            rooms.iter().zip(&footing).filter(|(r, _)| counted(r)).map(|(_, g)| g).sum()
        };
        out
    }

    /// The ratios' whole count, with or without the parcel's zoning (a parcel of 0 without one): what the audit facts
    /// read at once (docs/adr/8095).
    pub fn zoning_counted(&self) -> ZoningReport {
        let zoning = self.settings().arch.as_ref().and_then(|a| a.zoning);
        self.zoning_measured(zoning)
    }

    /// Each room as the ratios count it, with or without the parcel's zoning (docs/adr/8069): what the model's emsal zones
    /// are made of.
    pub fn zoning_rooms(&self) -> Vec<ZoningRoom> {
        let zoning = self.settings().arch.as_ref().and_then(|a| a.zoning);
        self.zoning_measured(zoning).rooms
    }

    /// The uses 22/1 lets out of katlar alanı only up to a limit, with or without the parcel's zoning (docs/adr/8079):
    /// what the regulation's rules read the limits' conditions on.
    pub fn zoning_limits(&self) -> Vec<ZoningLimitUse> {
        let zoning = self.settings().arch.as_ref().and_then(|a| a.zoning);
        self.zoning_measured(zoning).limits
    }

    /// The building's footprint as the ratios count it (docs/adr/8032): its area and outline, with or without a zoning.
    pub fn footprint(&self) -> (f64, Vec<Area>) {
        let zoning = self.settings().arch.as_ref().and_then(|a| a.zoning);
        let r = self.zoning_measured(zoning);
        (r.footprint, r.footprint_shape)
    }

    /// Emsal ve TAKS tablosu's cells (docs/adr/8030), as PAİY 57/3 asks the approved project to show them: what the
    /// zoning allows beside what the drawing has and what is left (in the warning colour past a limit), then each
    /// storey's area counted in and out and their total. Areas in the project's area unit and decimals, ratios with
    /// two; “—” for what the zoning does not give (none at all: the storeys' areas alone are numbers).
    pub fn zoning_table(&self) -> ZoningTable {
        let zoning = self.settings().arch.as_ref().and_then(|a| a.zoning);
        let r = self.zoning_measured(zoning);
        let s = self.settings();
        let unit = match s.area_unit {
            AreaUnit::M2 => "m²",
            AreaUnit::Donum => "dönüm",
            AreaUnit::Ha => "ha",
        };
        let area = |v: f64| self.arch_value_text(&crate::arch_values::ArchValue::Area(v), false);
        let ratio = |v: f64| crate::text_fields::number_text(v, 1.0, 2, false);
        let dash = || "—".to_owned();
        let given = r.parcel > 0.0;
        let named = |t: &str| format!("{t} ({unit})");
        let mut rows: Vec<Vec<String>> = Vec::new();
        let mut styles: Vec<CellStyle> = Vec::new();
        let style = |row: usize, col: u32, cols: u32, bold: bool, over: bool| CellStyle {
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
        };
        rows.push(vec![
            "Emsal ve TAKS".into(),
            "İmar durumu".into(),
            "Proje".into(),
            "Kalan".into(),
        ]);
        rows.push(vec![
            named("Parsel alanı"),
            if given { area(r.parcel) } else { dash() },
            String::new(),
            String::new(),
        ]);
        // A limit, the drawing's value and what is left; past the limit, the drawing's value and what is left in the
        // warning colour.
        let mut limited = |rows: &mut Vec<Vec<String>>,
                           words: String,
                           limit: Option<f64>,
                           drawn: Option<f64>,
                           left: bool,
                           write: &dyn Fn(f64) -> String| {
            let over = matches!((limit, drawn), (Some(l), Some(d)) if d > l + 1e-9);
            rows.push(vec![
                words,
                limit.map_or_else(dash, write),
                drawn.map_or_else(dash, write),
                match (left, limit, drawn) {
                    (true, Some(l), Some(d)) => write(l - d),
                    _ => String::new(),
                },
            ]);
            if over {
                styles.push(style(
                    rows.len() - 1,
                    2,
                    if left { 2 } else { 1 },
                    false,
                    true,
                ));
            }
        };
        let far = r.far_limit.filter(|_| given);
        let coverage = r.coverage_limit.filter(|_| given);
        limited(
            &mut rows,
            "Emsal (KAKS)".into(),
            far,
            given.then(|| r.far()),
            false,
            &ratio,
        );
        limited(
            &mut rows,
            named("Emsale konu alan"),
            far.map(|f| f * r.parcel),
            Some(r.floors),
            true,
            &area,
        );
        limited(
            &mut rows,
            named("Emsal harici, %30 içinde"),
            r.cap.filter(|_| given),
            Some(r.exempt),
            true,
            &area,
        );
        // Each limited use's part past its limit, counted into Emsale konu alan (22/1-l, docs/adr/8069; 22/1-k, -ç,
        // -f, docs/adr/8079): a row only when something passed.
        for (over, words) in [
            (r.balcony_over, "Balkonların %20'yi aşan kısmı, emsale eklenen"),
            (r.storage_over, "Depo eklentilerinin %50'yi aşan kısmı, emsale eklenen"),
            (r.caretaker_over, "Kapıcı dairelerinin 70 m²'yi aşan kısmı, emsale eklenen"),
            (r.library_over, "Kütüphanenin 50 m²'yi aşan kısmı, emsale eklenen"),
        ] {
            if over > 0.0 {
                rows.push(vec![named(words), String::new(), area(over), String::new()]);
            }
        }
        if r.over > 0.0 {
            rows.push(vec![
                named("%30'u aşan, emsale eklenen"),
                String::new(),
                area(r.over),
                String::new(),
            ]);
        }
        if r.outside > 0.0 {
            rows.push(vec![
                named("Emsal harici, %30 dışında"),
                String::new(),
                area(r.outside),
                String::new(),
            ]);
        }
        // Of it, a traditional cut stone masonry building's stone walls past 25 cm and 12.50 cm (5/8, docs/adr/8095).
        if r.masonry > 0.0 {
            rows.push(vec![
                named("Taş duvarların kalın kısmı, %30 dışında"),
                String::new(),
                area(r.masonry),
                String::new(),
            ]);
        }
        limited(
            &mut rows,
            "TAKS".into(),
            coverage,
            given.then(|| r.coverage()),
            false,
            &ratio,
        );
        limited(
            &mut rows,
            named("Taban alanı"),
            coverage.map(|c| c * r.parcel),
            Some(r.footprint),
            true,
            &area,
        );
        // The exterior stone walls past 25 cm, out of taban alanı (20/8-j, docs/adr/8095).
        if r.masonry_footprint > 0.0 {
            rows.push(vec![
                named("Dış taş duvarların kalın kısmı, taban alanı dışında"),
                String::new(),
                area(r.masonry_footprint),
                String::new(),
            ]);
        }
        rows.push(vec![
            named("Yapı inşaat alanı"),
            String::new(),
            area(r.construction()),
            String::new(),
        ]);
        // Vaziyet planı (docs/adr/8032): each kind of edge's garden distance beside the footprint's nearest, the trees
        // (23/6) and ±0.00.
        if let Some(site) = self.site_report() {
            // Metres with two decimals, as the regulation writes them (“5.00 metre”), whatever the drawing's unit.
            let length = |v: f64| crate::text_fields::number_text(v, 1.0, 2, false);
            for kind in kentos_contracts::SiteEdgeKind::ALL {
                let mine: Vec<&crate::site::SiteEdgeReport> =
                    site.edges.iter().filter(|e| e.kind == kind).collect();
                if mine.is_empty() {
                    continue;
                }
                let need = mine.iter().map(|e| e.required).fold(0.0, f64::max);
                let drawn = mine
                    .iter()
                    .filter_map(|e| e.drawn)
                    .fold(f64::INFINITY, f64::min);
                let left = mine
                    .iter()
                    .filter_map(|e| e.drawn.map(|d| d - e.required))
                    .fold(f64::INFINITY, f64::min);
                rows.push(vec![
                    format!("{} (m)", kind.label()),
                    length(need),
                    if drawn.is_finite() {
                        length(drawn)
                    } else {
                        String::new()
                    },
                    if left.is_finite() {
                        length(left)
                    } else {
                        String::new()
                    },
                ]);
                if left < -1e-6 {
                    styles.push(style(rows.len() - 1, 2, 2, false, true));
                }
            }
            rows.push(vec![
                "Ağaç (adet, 23/6)".into(),
                site.trees.to_string(),
                String::new(),
                String::new(),
            ]);
            if let Some(z) = site.datum {
                rows.push(vec![
                    "±0,00 kotu (m)".into(),
                    String::new(),
                    length(z),
                    String::new(),
                ]);
            }
        }
        // Each storey's area counted in and out.
        rows.push(vec![
            "Kat".into(),
            named("Emsale dahil"),
            named("Emsal harici"),
            named("Kat alanı"),
        ]);
        styles.push(style(rows.len() - 1, 0, 4, true, false));
        let name = |l: &Option<String>| match l {
            Some(id) => self
                .settings()
                .arch
                .as_ref()
                .and_then(|a| a.levels.iter().find(|x| &x.id == id))
                .map_or_else(|| id.clone(), |x| x.name.clone()),
            None if r.levels.len() == 1 => "Bina".to_owned(),
            None => "Katı verilmemiş".to_owned(),
        };
        for l in &r.levels {
            rows.push(vec![
                name(&l.level),
                area(l.included),
                area(l.exempt + l.outside),
                area(l.all()),
            ]);
        }
        rows.push(vec![
            crate::schedules::SCHEDULE_TOTAL.into(),
            area(r.included),
            area(r.exempt + r.outside),
            area(r.construction()),
        ]);
        styles.push(style(rows.len() - 1, 0, 4, true, false));
        ZoningTable {
            rows,
            aligns: vec![
                TableAlign::Left,
                TableAlign::Right,
                TableAlign::Right,
                TableAlign::Right,
            ],
            styles,
        }
    }
}
