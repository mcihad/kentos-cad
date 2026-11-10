//! Emsal and TAKS (docs/adr/8030; ADV-22, M30): the drawing's floor area ratio and coverage against the parcel's
//! zoning, as Planlı Alanlar İmar Yönetmeliği reads them.
//!
//! - **Each room's gross** is its outer contour (4/ğ-1's rule, room by room): its outer-face outline less every other
//!   room's axis outline on its level, a shared wall's façade corner halved between them.
//! - **How it counts** is its use's brief's (`SpaceBrief::emsal`), else `emsal_by_use` by its name and use; an outdoor
//!   room (a balcony, a terrace) is out within the 30 % (22/1-l); a void (shaft, gallery) is no room.
//! - **Katlar alanı** (4/şş, 21): the rooms counted in; **emsal harici** (22/1) together at most 30 % of the parcel's
//!   emsal area (parcel × emsal, 5/8), what passes goes in; 5/8's own rooms (fire stairs …) out apart.
//! - **Taban alanı** (4/bbbb): the ground storey's rooms' gross (the storey named Zemin, else the one nearest ±0.00),
//!   5/8's rooms and outdoor rooms out (20/8: fire stairs, open car parks).
//!
//! - **Yapı inşaat alanı** (4/oooo): every room's, balconies, terraces and common areas in, voids out.
//!
//! **Emsal ve TAKS tablosu** (`zoning_table`) writes them as PAİY 57/3 asks the approved project to show them: the
//! parcel, the emsal and the area it allows against the drawing's, the emsal harici allowance, TAKS and the footprint,
//! the construction area, and each storey's area counted in and out.
//!
//! A ratio is the drawing's measure, not an approval; the plan's own notes and the regulation's other limits (the
//! balconies' 20 %, the storage's 50 %, the caretaker's 70 m²) are the next step.

use kentos_contracts::{
    AreaUnit, CellRange, CellStyle, EmsalCount, Entity, SpaceMethod, TableAlign, Zoning,
};
use kentos_geometry_core::geom::arrangement::Area;
use kentos_geometry_core::geom::region;

use crate::Document;
use crate::arch_values::core_areas;
use crate::space_follow::SpaceSearch;

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
    /// Each storey's rooms, in the project's order of its storeys; the rooms on none last.
    pub levels: Vec<ZoningLevel>,
    /// The footprint's outline: the ground storey's rooms it counts, their contours together (the site plan's oturum,
    /// docs/adr/8032).
    pub footprint_shape: Vec<Area>,
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

    /// The rooms counted against `zoning`; with none, a parcel of 0 (its ratios are not numbers).
    fn zoning_measured(&self, zoning: Option<Zoning>) -> ZoningReport {
        let empty = kentos_contracts::ArchSettings::default();
        let arch = self.settings().arch.as_ref().unwrap_or(&empty);
        let program = &arch.program;
        let mut search = SpaceSearch::new(self);
        let mut rooms = Vec::new();
        for e in self.entities() {
            let Entity::Space(s) = e else { continue };
            let x = &s.space;
            let brief = x.usage.as_deref().and_then(|u| program.brief(u));
            let outdoor = brief.is_some_and(|b| b.outdoor);
            let count = match brief.and_then(|b| b.emsal) {
                Some(c) => c,
                None if outdoor => EmsalCount::Exempt,
                None => kentos_contracts::emsal_by_use(x.name.as_deref(), x.usage.as_deref()),
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
            });
        }
        // Each room's contour on its level, a shared wall's façade corner halved between two rooms.
        let mut gross = vec![0.0; rooms.len()];
        let mut shapes: Vec<Vec<Area>> = vec![Vec::new(); rooms.len()];
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
                    }
                }
            }
        }
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
        for (r, g) in rooms.iter().zip(&gross) {
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
                    out.exempt += g;
                    level.exempt += g;
                }
                EmsalCount::Outside => {
                    out.outside += g;
                    level.outside += g;
                }
            }
        }
        out.levels.sort_by_key(|l| order(&l.level));
        out.cap = out.far_limit.map(|f| 0.30 * f * parcel);
        out.over = out.cap.map_or(0.0, |c| (out.exempt - c).max(0.0));
        out.floors = out.included + out.over;
        // The ground storey: named Zemin, else the one nearest ±0.00; with no storeys, every room.
        let ground = {
            // Turkish İ lower-cases to i and a dot; the dot is dropped for the comparison.
            let fold = |t: &str| t.to_lowercase().replace('\u{307}', "");
            arch.levels
                .iter()
                .find(|l| fold(&l.name).contains("zemin"))
                .or_else(|| {
                    arch.levels
                        .iter()
                        .min_by(|a, b| a.elevation.abs().total_cmp(&b.elevation.abs()))
                })
                .map(|l| l.id.clone())
        };
        let counted = |r: &Room| {
            !r.void
                && !r.outdoor
                && r.count != EmsalCount::Outside
                && (ground.is_none() || r.level == ground)
        };
        out.footprint = rooms
            .iter()
            .zip(&gross)
            .filter(|(r, _)| counted(r))
            .map(|(_, g)| g)
            .sum();
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
        out
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
