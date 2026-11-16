//! KentOS Mimari's building as IFC 4.3 (docs/adr/8047; Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması
//! Hakkında Yönetmelik, RG 5.8.2026/33331, EK-5, EK-6). The application gives the model the sections read ([`Input`]:
//! the walls joined and cut as the plan has them, their openings, the slabs, columns, beams, roofs, railings, stairs and
//! ramps as prisms, flights and sloped faces over ±0.00) and the drawing's objects; the spaces are its rooms. Each object
//! becomes the class EK-5 Tablo 5.1 names for it, its name EK-5 §5.5's `Disiplin-KategoriKodu-Açıklama`
//! ([`crate::naming`]), its mandatory properties EK-6's where the drawing knows them.

use std::collections::BTreeMap;

use kentos_contracts::{
    ArchSettings, BeamRole, Entity, OpeningBuild, OpeningForm, OpeningSide, RoofPartKind, SlabRole,
    SpaceFields, Vec2, WallComponent,
};
use kentos_geometry_core::Vec2 as P;
use kentos_geometry_core::geom::opening::{HoleFrame, Recess};
use kentos_geometry_core::geom::section::{Flight, Height, Sloped, Solid};

use crate::building::{
    Actor, Address, Axis, Building, Class, Element, Grid, Group, Opening, Organization, Place, Pset,
    Qto, Shape, Space, Storey, Value, Zone, flat_rings,
};
use crate::naming::name;

/// An opening through a wall as the section model has it: its outline through the wall (its jambs' face points), its
/// sill and head over ±0.00, its form and its object's persistent id.
#[derive(Clone, Debug, PartialEq)]
pub struct Gap {
    pub ring: Vec<P>,
    /// Its ring's edges' bulges (docs/adr/8071 §1): along a curved wall's faces, arcs; empty: straight.
    pub bulges: Vec<f64>,
    pub sill: f64,
    pub head: f64,
    pub form: OpeningForm,
    pub owner: Option<String>,
}

/// What the application gives: the drawing's title, the file's time and the application's name; the section model's
/// storeys (id, name, floor over ±0.00, height), solids with their objects' persistent ids, and openings; the drawing's
/// objects with their ids (the UUID's bits and its text); a wall style's name by its id; a room's net area as the plan
/// measures it; the project's architecture settings (İmar durumu, Vaziyet planı).
pub struct Input<'a> {
    pub title: &'a str,
    pub stamp: &'a str,
    pub application: &'a str,
    pub levels: Vec<(String, String, f64, Option<f64>)>,
    pub solids: &'a [Solid],
    pub owners: &'a [Option<String>],
    /// Which part of its object each solid is (docs/adr/8070 §3, §7); none: its object itself.
    pub parts: &'a [Option<Part>],
    /// Each solid's footprint with its arcs (docs/adr/8071 §1), for a solid that has any: its rings' points and their
    /// edges' bulges, written as arcs in place of its flattened rings; none, or a short list: straight.
    pub exact: &'a [Option<Vec<(Vec<P>, Vec<f64>)>>],
    pub gaps: Vec<Gap>,
    pub entities: Vec<(u128, String, &'a Entity)>,
    pub wall_style_name: &'a dyn Fn(Option<&str>) -> String,
    /// The project's wall styles: a wall's fire resistance class is its own, else its style's (docs/adr/8068).
    pub wall_styles: &'a [kentos_contracts::WallStyleDef],
    pub net_area: &'a mut dyn FnMut(&SpaceFields) -> Option<f64>,
    pub arch: Option<&'a ArchSettings>,
    pub particulars: Particulars,
    /// The project's system (EK-7 Tablo 7.6–7.7; `crate::crs::projected`), none for a drawing in local coordinates.
    pub crs: Option<crate::crs::Crs>,
    /// Each storey's emsal areas as the ratios count the rooms (docs/adr/8069): the spatial zones' grounds.
    pub emsal: Vec<EmsalArea>,
    /// The building program's units with their rooms (docs/adr/8069): the groups of spaces.
    pub groups: Vec<UnitGroup>,
    /// The blocks that are building elements (docs/adr/8070 §5): furniture, sanitary fixtures, lifts.
    pub fixtures: Vec<Fixture>,
    /// A room's floor and ceiling covering thicknesses as the vertical model holds them (docs/adr/8070 §6), by its
    /// persistent id; a room not here, or a side none, has none.
    pub coverings: BTreeMap<String, (Option<f64>, Option<f64>)>,
}

/// What part of its object a solid of the model is (docs/adr/8070 §3, §7).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    /// A roof's chimney or vent stack, by its index in the roof's `parts`.
    Roof(usize),
    /// A curtain wall's mullion (dikme), by its number on the wall.
    Mullion(usize),
    /// A curtain wall's transom (kayıt), by its number on the wall.
    Transom(usize),
    /// A curtain wall's panel.
    Panel,
}

/// A block placed in the drawing that is a building element (docs/adr/8070 §5): its insert's id bits and persistent id,
/// its block's name, what its card says it is, its storey's level and its outline in plan (the box of what it draws,
/// placed); a lift's rated capacity in persons and kilograms, as its insert gives them.
#[derive(Clone, Debug, PartialEq)]
pub struct Fixture {
    pub id: u128,
    pub uid: String,
    pub name: String,
    pub element: kentos_contracts::BlockElement,
    pub level: Option<String>,
    pub ring: Vec<[f64; 2]>,
    pub people: Option<i64>,
    pub weight: Option<f64>,
}

/// An emsal area of a storey (docs/adr/8069; EK-6 Tablo 6.60): its storey's level id, how its rooms count, its rings
/// (even-odd, the rooms' contours together) and, counted in, the part of its storey's balconies past their units' 20 %
/// (m²; PAİY 22/1-l).
#[derive(Clone, Debug, PartialEq)]
pub struct EmsalArea {
    pub level: Option<String>,
    pub count: kentos_contracts::EmsalCount,
    pub rings: Vec<Vec<[f64; 2]>>,
    pub moved: f64,
}

/// A unit of the building program (docs/adr/8069; EK-6 Tablo 6.68): its id's bits, its number and name, its kind and its
/// rooms (their ids' bits).
#[derive(Clone, Debug, PartialEq)]
pub struct UnitGroup {
    pub id: Option<u128>,
    pub number: String,
    pub name: Option<String>,
    pub kind: kentos_contracts::UnitKind,
    pub rooms: Vec<u128>,
}

/// An emsal zone's state as EK-6 Tablo 6.60 writes it and the clauses it rests on.
fn emsal_state(count: kentos_contracts::EmsalCount, moved: f64) -> (&'static str, &'static str, String) {
    use kentos_contracts::EmsalCount as C;
    match count {
        C::Included => (
            "DAHIL",
            "emsale dahil alan",
            if moved > 0.0 {
                format!(
                    "Planlı Alanlar İmar Yönetmeliği 4/1-şş, 21: katlar alanına dahil; 22/1-l: bağımsız bölüm net alanının %20'sini aşan {moved:.2} m² balkon da dahil"
                )
            } else {
                "Planlı Alanlar İmar Yönetmeliği 4/1-şş, 21: katlar alanına dahil".to_owned()
            },
        ),
        C::Exempt => (
            "HARIC",
            "emsal harici alan",
            "Planlı Alanlar İmar Yönetmeliği 22/1: katlar alanına dahil edilmeyen; 5/8: emsale esas alanın %30'u içinde".to_owned(),
        ),
        C::Outside => (
            "HARIC",
            "emsal harici alan (%30 dışında)",
            "Planlı Alanlar İmar Yönetmeliği 5/8: yangın merdiveni, korunumlu koridor, yangın güvenlik holü; %30 sınırının dışında".to_owned(),
        ),
    }
}

/// Why an object of the drawing's building is not in the file.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Why {
    /// The section model gives it no body (a wall without a height, a degenerate object).
    NoBody,
    /// An opening without its outline through a written wall.
    NoWall,
    /// A room without an area.
    NoArea,
    /// A grid's axes on a storey all in one direction: an `IfcGrid` asks axes in two (docs/adr/8070 §4).
    OneWay,
}

/// An object of the drawing's building the file does not hold: its persistent id, what it is ("duvar", "kapı" …) and
/// why.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Left {
    pub id: String,
    pub what: &'static str,
    pub why: Why,
}

/// The drawing's künye (the permit set's particulars, docs/adr/8037) and the measures its title block reads, for EK-6's
/// and EK-7's words: none of them is a value the drawing does not hold ("—", "000000" are none).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Particulars {
    /// İşin adı.
    pub work: Option<String>,
    /// Proje kodu (EK-1), bina kodu, revizyon no (EK-2).
    pub project_code: Option<String>,
    pub building_code: Option<String>,
    pub revision: Option<String>,
    /// İl, ilçe, mahalle.
    pub province: Option<String>,
    pub district: Option<String>,
    pub neighbourhood: Option<String>,
    /// Ada, parsel, tapu kaydı.
    pub block: Option<String>,
    pub parcel: Option<String>,
    pub title_deed: Option<String>,
    /// Mimar (adı soyadı), unvanı, Mimarlar Odası sicil no.
    pub architect: Option<String>,
    pub architect_title: Option<String>,
    pub registration: Option<String>,
    /// Büronun adı, vergi kimlik no, adresi.
    pub office: Option<String>,
    pub office_tax_number: Option<String>,
    pub office_address: Option<String>,
    /// Kullanım amacı (the building program's), yapı inşaat alanı and taban alanı (İmar durumu's measures), m².
    pub occupancy: Option<String>,
    pub gross_area: Option<f64>,
    pub footprint: Option<f64>,
    /// The gardens' distances, m: ön, yan, arka (each the least of its edges'; Vaziyet planı).
    pub setbacks: [Option<f64>; 3],
    /// The construction action (EK-6 Tablo 6.7 `ConstructionMethod`: yeni yapı, ilave, tadilat), the künye's
    /// building kind (docs/adr/8068).
    pub construction: Option<String>,
}

impl Particulars {
    /// The parcel as the land registry names it: "ada/parsel", or the parcel alone.
    pub fn land_id(&self) -> Option<String> {
        match (&self.block, &self.parcel) {
            (Some(a), Some(p)) => Some(format!("{a}/{p}")),
            (None, Some(p)) => Some(p.clone()),
            _ => None,
        }
    }

    /// The site's and the building's address (EK-6 Tablo 6.55's labels: İl → Town, İlçe → Region; the mahalle a line).
    pub fn address(&self) -> Option<Address> {
        if self.province.is_none() && self.district.is_none() && self.neighbourhood.is_none() {
            return None;
        }
        Some(Address {
            lines: self.neighbourhood.iter().cloned().collect(),
            town: self.province.clone(),
            region: self.district.clone(),
            postal_code: None,
            country: Some("Türkiye".into()),
        })
    }

    /// The architect (EK-7 Tablo 7.2–7.5): their family name the last word of their name, their given name the words
    /// before it; their registration number their identification; their office, when it has a name.
    pub fn architect(&self) -> Option<Actor> {
        let whole = self.architect.as_deref()?;
        let words: Vec<&str> = whole.split_whitespace().collect();
        let (given, family) = match words.split_last() {
            Some((last, rest)) if !rest.is_empty() => {
                (Some(rest.join(" ")), Some((*last).to_owned()))
            }
            Some((last, _)) => (None, Some((*last).to_owned())),
            None => return None,
        };
        let organization = self.office.as_ref().map(|name| Organization {
            identification: self.office_tax_number.clone(),
            name: name.clone(),
            address: self.office_address.as_ref().map(|a| Address {
                lines: vec![a.clone()],
                country: Some("Türkiye".into()),
                ..Address::default()
            }),
        });
        Some(Actor {
            identification: self.registration.clone(),
            family,
            given,
            organization,
            role: "ARCHITECT",
            role_description: self.architect_title.clone(),
        })
    }

    /// The model file's name without its extension (EK-2 Tablo 2.1: `crate::naming::model_file`), or the drawing's
    /// title while the project has no code.
    pub fn file_stem(&self, title: &str) -> String {
        match &self.project_code {
            Some(code) => crate::naming::model_file(
                code,
                self.building_code.as_deref().unwrap_or("00"),
                self.revision.as_deref().unwrap_or("000"),
            ),
            None => title.to_owned(),
        }
    }
}

/// Pset_Address (EK-6 Tablo 6.7, 6.55) of an address.
fn address_pset(a: &Option<Address>) -> Option<Pset> {
    let a = a.as_ref()?;
    let mut props = Vec::new();
    if !a.lines.is_empty() {
        props.push(("AddressLines".to_owned(), Value::Labels(a.lines.clone())));
    }
    if let Some(t) = &a.town {
        props.push(("Town".to_owned(), label(t)));
    }
    if let Some(r) = &a.region {
        props.push(("Region".to_owned(), label(r)));
    }
    Some(Pset {
        name: "Pset_Address".into(),
        props,
    })
}

/// What was written, for the command's answer and the acceptance, and what was not.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Written {
    pub storeys: usize,
    pub walls: usize,
    pub doors: usize,
    pub windows: usize,
    pub openings: usize,
    pub spaces: usize,
    pub slabs: usize,
    pub columns: usize,
    pub beams: usize,
    pub roofs: usize,
    pub railings: usize,
    pub stairs: usize,
    pub ramps: usize,
    pub chimneys: usize,
    /// The blocks written as building elements (docs/adr/8070 §5).
    pub fixtures: usize,
    /// The curtain walls and their frames written (docs/adr/8070 §7).
    pub curtain_walls: usize,
    /// The coverings written (docs/adr/8070 §10).
    pub coverings: usize,
    pub members: usize,
    /// The grids' axes written (docs/adr/8070 §4).
    pub grid_axes: usize,
    pub left: Vec<Left>,
    /// The rooms without a number or a name (Md. 10, EK-3), by their persistent ids.
    pub unnamed: Vec<String>,
}

fn pt(p: P) -> [f64; 2] {
    [p.x, p.y]
}

/// Which side of the line `(n, c)` (`n·p ≥ c`) a polygon keeps (Sutherland–Hodgman against one half-plane).
fn clip(ring: &[P], n: P, c: f64) -> Vec<P> {
    let mut out = Vec::new();
    let k = ring.len();
    for i in 0..k {
        let (a, b) = (ring[i], ring[(i + 1) % k]);
        let (da, db) = (n.x * a.x + n.y * a.y - c, n.x * b.x + n.y * b.y - c);
        if da >= 0.0 {
            out.push(a);
        }
        if (da >= 0.0) != (db >= 0.0) {
            let t = da / (da - db);
            out.push(P::new(a.x + (b.x - a.x) * t, a.y + (b.y - a.y) * t));
        }
    }
    out
}

/// A flight's steps as prisms (docs/adr/8042 §2): each tread's slice of its footprint, from the flight's underside under
/// the slice's far edge to the tread.
fn flight(rings: &[Vec<P>], f: &Flight, floor: f64) -> Vec<Shape> {
    let mut out = Vec::new();
    let Some(ring) = rings.first() else {
        return out;
    };
    let reach = ring.iter().map(|p| f.s(*p)).fold(f64::MIN, f64::max);
    for (i, &(from, top)) in f.steps.iter().enumerate() {
        let to = f.steps.get(i + 1).map_or(reach, |n| n.0);
        if to <= from {
            continue;
        }
        let c0 = f.dir.x * f.start.x + f.dir.y * f.start.y;
        let a = clip(ring, f.dir, c0 + from);
        let b = clip(&a, P::new(-f.dir.x, -f.dir.y), -(c0 + to));
        if b.len() < 3 {
            continue;
        }
        let bottom = f.bottom(to).min(top - 1e-3);
        out.push(Shape::Prism {
            rings: vec![b.into_iter().map(pt).collect()],
            bulges: Vec::new(),
            bottom: bottom - floor,
            top: top - floor,
        });
    }
    out
}

/// A roof's face (docs/adr/8044 §5) as a closed set of faces: its footprint's outer ring under its top plane and
/// `thickness` lower, and its sides.
fn sloped(rings: &[Vec<P>], s: &Sloped, floor: f64) -> Vec<Shape> {
    rings
        .iter()
        .filter(|r| r.len() >= 3)
        .map(|r| {
            let n = r.len();
            let mut vertices: Vec<[f64; 3]> =
                r.iter().map(|p| [p.x, p.y, s.z(*p) - floor]).collect();
            vertices.extend(r.iter().map(|p| [p.x, p.y, s.z(*p) - s.thickness - floor]));
            // Counterclockwise from over it: the top as it is, the bottom reversed, the sides outward.
            let ccw = kentos_geometry_core::geometry::signed_area(r) > 0.0;
            let mut top: Vec<usize> = (0..n).collect();
            let mut bottom: Vec<usize> = (n..2 * n).rev().collect();
            if !ccw {
                top.reverse();
                bottom.reverse();
            }
            let mut faces = vec![top, bottom];
            for i in 0..n {
                let j = (i + 1) % n;
                let side = if ccw {
                    vec![i, i + n, j + n, j]
                } else {
                    vec![i, j, j + n, i + n]
                };
                faces.push(side);
            }
            Shape::Faces { vertices, faces }
        })
        .collect()
}

/// A model solid's shapes over its storey's floor.
/// The drawing's grids (EK-6 Tablo 6.36; docs/adr/8070 §4): its axes by storey (none: shown on every storey, the
/// building's), an axis's lines parallel to the first straight one U, the others V; rectangular when each set is
/// parallel and the two square to each other, else irregular. A storey's axes in one direction only make no grid (IFC
/// 4.3 asks both sets): they are left out, said.
fn grids_of(
    entities: &[(u128, String, &Entity)],
    storey_of: &dyn Fn(Option<&str>) -> usize,
    written: &mut Written,
) -> Vec<Grid> {
    let mut by_level: Vec<(Option<String>, Vec<(u128, &String, &kentos_contracts::GridFields)>)> = Vec::new();
    for (bits, uid, e) in entities {
        let Entity::Grid(g) = e else { continue };
        let level = g.grid.arch.level.clone();
        match by_level.iter_mut().find(|(l, _)| *l == level) {
            Some((_, list)) => list.push((*bits, uid, &g.grid)),
            None => by_level.push((level, vec![(*bits, uid, &g.grid)])),
        }
    }
    let cross = |a: [f64; 2], b: [f64; 2]| a[0] * b[1] - a[1] * b[0];
    let mut out = Vec::new();
    for (level, list) in by_level {
        let axes: Vec<(&String, Axis, Option<[f64; 2]>)> = list
            .iter()
            .filter_map(|(bits, uid, g)| {
                let (a, b) = (g.pts.first()?, g.pts.get(1)?);
                let (dx, dy) = (b.x - a.x, b.y - a.y);
                let len = libm::hypot(dx, dy);
                if len < 1e-9 {
                    return None;
                }
                // An arc's middle: the chord's middle moved by the sagitta, to the chord's right for a positive bulge.
                let bulge = g.bulges.as_ref().and_then(|v| v.first().copied()).unwrap_or(0.0);
                let through = (bulge != 0.0).then(|| {
                    [(a.x + b.x) / 2.0 + dy * bulge / 2.0, (a.y + b.y) / 2.0 - dx * bulge / 2.0]
                });
                Some((
                    *uid,
                    Axis {
                        id: Some(*bits),
                        tag: g.label.trim().to_owned(),
                        from: [a.x, a.y],
                        to: [b.x, b.y],
                        through,
                    },
                    (bulge == 0.0).then_some([dx / len, dy / len]),
                ))
            })
            .collect();
        let first = axes.iter().find_map(|(_, _, d)| *d);
        let parallel = |a: Option<[f64; 2]>, b: Option<[f64; 2]>| {
            a.zip(b).is_some_and(|(a, b)| cross(a, b).abs() < 1e-6)
        };
        let (u, v): (Vec<_>, Vec<_>) = axes.iter().partition(|(_, _, d)| parallel(first, *d));
        if u.is_empty() || v.is_empty() {
            for (uid, ..) in &axes {
                written.left.push(Left {
                    id: (*uid).clone(),
                    what: "aks",
                    why: Why::OneWay,
                });
            }
            continue;
        }
        let across = v.iter().find_map(|(_, _, d)| *d);
        let rectangular = v.iter().all(|(_, _, d)| parallel(across, *d))
            && first
                .zip(across)
                .is_some_and(|(a, b)| (a[0] * b[0] + a[1] * b[1]).abs() < 1e-6);
        written.grid_axes += axes.len();
        let seed = axes.iter().map(|(uid, ..)| uid.as_str()).min().unwrap_or_default();
        out.push(Grid {
            id: Some(crate::step::fnv(seed, 3)),
            name: name("AKS", "Aks"),
            storey: level.as_deref().map(|l| storey_of(Some(l))),
            u: u.into_iter().map(|(_, a, _)| a.clone()).collect(),
            v: v.into_iter().map(|(_, a, _)| a.clone()).collect(),
            predefined: if rectangular { "RECTANGULAR" } else { "IRREGULAR" },
        });
    }
    out
}

/// A piece of a chimney (docs/adr/8070 §3): a flue's hollow in a wall or a roof's stack, its outline in plan between
/// two heights over ±0.00.
struct Piece {
    storey: usize,
    ring: Vec<[f64; 2]>,
    bottom: f64,
    top: f64,
    /// A flue: its object's id bits and persistent id, its opening's index, its wall's material.
    flue: Option<(u128, String, usize, Option<String>)>,
    /// A stack: its roof's id bits and persistent id, its part's index and its kind's words.
    stack: Option<(u128, String, usize, &'static str)>,
}

/// The pieces that are one chimney, each group lowest first, the groups in the order of their first piece: two are one
/// where one's middle (its corners' mean) lies in the other's outline in plan.
fn chimney_groups(pieces: &[Piece]) -> Vec<Vec<usize>> {
    fn middle(r: &[[f64; 2]]) -> [f64; 2] {
        let n = r.len().max(1) as f64;
        [r.iter().map(|p| p[0]).sum::<f64>() / n, r.iter().map(|p| p[1]).sum::<f64>() / n]
    }
    fn within(r: &[[f64; 2]], p: [f64; 2]) -> bool {
        let mut odd = false;
        for i in 0..r.len() {
            let (a, b) = (r[i], r[(i + 1) % r.len()]);
            if (a[1] > p[1]) != (b[1] > p[1]) && p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1]) {
                odd = !odd;
            }
        }
        odd
    }
    fn root(of: &mut [usize], mut i: usize) -> usize {
        while of[i] != i {
            of[i] = of[of[i]];
            i = of[i];
        }
        i
    }
    let n = pieces.len();
    let mut of: Vec<usize> = (0..n).collect();
    for i in 0..n {
        for j in i + 1..n {
            let (a, b) = (&pieces[i].ring, &pieces[j].ring);
            if within(b, middle(a)) || within(a, middle(b)) {
                let (ri, rj) = (root(&mut of, i), root(&mut of, j));
                of[rj.max(ri)] = rj.min(ri);
            }
        }
    }
    let mut groups: Vec<(usize, Vec<usize>)> = Vec::new();
    for i in 0..n {
        let r = root(&mut of, i);
        match groups.iter_mut().find(|(g, _)| *g == r) {
            Some((_, list)) => list.push(i),
            None => groups.push((r, vec![i])),
        }
    }
    groups
        .into_iter()
        .map(|(_, mut list)| {
            list.sort_by(|a, b| pieces[*a].bottom.total_cmp(&pieces[*b].bottom));
            list
        })
        .collect()
}

/// A model solid's shapes over its storey's floor: a prism with its arcs where the model holds them (docs/adr/8071
/// §1), a flight's treads, a roof face's faces.
fn shapes(solid: &Solid, exact: Option<&Option<Vec<(Vec<P>, Vec<f64>)>>>, floor: f64) -> Vec<Shape> {
    let rings: Vec<Vec<P>> = solid
        .rings
        .iter()
        .filter(|r| r.len() >= 3)
        .cloned()
        .collect();
    if rings.is_empty() {
        return Vec::new();
    }
    match &solid.height {
        Height::Prism { bottom, top } => {
            let (rings, bulges) = match exact.and_then(Option::as_ref).filter(|e| !e.is_empty()) {
                Some(e) => (
                    e.iter().map(|(r, _)| r.iter().map(|p| pt(*p)).collect()).collect(),
                    e.iter().map(|(_, b)| b.clone()).collect(),
                ),
                None => (rings.iter().map(|r| r.iter().map(|p| pt(*p)).collect()).collect(), Vec::new()),
            };
            vec![Shape::Prism {
                rings,
                bulges,
                bottom: bottom - floor,
                top: top - floor,
            }]
        }
        Height::Flight(f) => flight(&rings, f, floor),
        Height::Sloped(s) => sloped(&rings, s, floor),
    }
}

fn level_of(e: &Entity) -> Option<&str> {
    match e {
        Entity::Wall(w) => w.wall.arch.level.as_deref(),
        Entity::Slab(s) => s.slab.arch.level.as_deref(),
        Entity::Column(c) => c.column.arch.level.as_deref(),
        Entity::Beam(b) => b.beam.arch.level.as_deref(),
        Entity::Roof(r) => r.roof.arch.level.as_deref(),
        Entity::Railing(r) => r.railing.arch.level.as_deref(),
        Entity::Stair(s) => s.stair.arch.level.as_deref(),
        Entity::Ramp(r) => r.ramp.arch.level.as_deref(),
        Entity::Space(s) => s.space.arch.level.as_deref(),
        _ => None,
    }
}

fn label(s: &str) -> Value {
    Value::Label(s.to_owned())
}

/// A layered wall's material for EK-6's single name: its core's, the layer of the strongest join priority (1, a core;
/// absent 3; 5, a finish), of the ones as strong the one taking what the others leave of the width.
fn core_material(components: &[WallComponent]) -> Option<String> {
    components
        .iter()
        .min_by_key(|c| (c.priority.unwrap_or(3), c.thickness.is_some()))
        .and_then(|c| c.material.clone())
}

/// A door's or a window's body across its wall: the band its frame stands in when its construction gives the frame's
/// depth (`HoleFrame`, the rule the plan draws the frame by: centred, or `frameOffset` from the face it opens to), else
/// the opening through the whole wall, as the plan draws one without. `through` is the opening's outline through the
/// wall; the band is cut from its jambs (`place`: the start jamb on the wall's left and right face, then the end's);
/// `right`: it opens to the wall's right face.
fn filler_ring(
    place: &[Vec2],
    build: Option<&OpeningBuild>,
    right: bool,
    through: &[[f64; 2]],
) -> Vec<[f64; 2]> {
    let band = || -> Option<Vec<[f64; 2]>> {
        let [sl, sr, el, er] = place else {
            return None;
        };
        let t = libm::hypot(sr.x - sl.x, sr.y - sl.y);
        let frame = HoleFrame {
            depth: build?.frame_depth?,
            offset: build?.frame_offset,
            right,
        };
        let (near, far) = frame.across(t)?;
        let at =
            |a: &Vec2, c: &Vec2, d: f64| [a.x + (c.x - a.x) * d / t, a.y + (c.y - a.y) * d / t];
        Some(vec![
            at(sl, sr, near),
            at(el, er, near),
            at(el, er, far),
            at(sl, sr, far),
        ])
    };
    band().unwrap_or_else(|| through.to_vec())
}

/// A material's name as EK-6 asks for it ("Malzeme İsmi_Dayanım"): the drawing's, which knows no strength class.
fn material(m: Option<String>) -> Option<String> {
    m.map(|m| m.trim().to_owned()).filter(|m| !m.is_empty())
}

/// `props` given `key` in the set `set`: added to the set if the element has it, else a set of its own.
fn give(psets: &mut Vec<Pset>, set: &str, key: &str, value: Value) {
    match psets.iter_mut().find(|p| p.name == set) {
        Some(p) => p.props.push((key.to_owned(), value)),
        None => psets.push(Pset { name: set.to_owned(), props: vec![(key.to_owned(), value)] }),
    }
}

/// The common property set EK-6 asks a class's fire resistance class in.
fn common_set(class: Class) -> Option<&'static str> {
    Some(match class {
        Class::Wall => "Pset_WallCommon",
        Class::Slab => "Pset_SlabCommon",
        Class::Column => "Pset_ColumnCommon",
        Class::Beam => "Pset_BeamCommon",
        Class::Roof => "Pset_RoofCommon",
        Class::Railing => "Pset_RailingCommon",
        Class::Stair => "Pset_StairCommon",
        Class::Door => "Pset_DoorCommon",
        Class::Window => "Pset_WindowCommon",
        Class::Covering => "Pset_CoveringCommon",
        Class::Ramp => "Pset_RampCommon",
        Class::CurtainWall => "Pset_CurtainWallCommon",
        Class::Member => "Pset_MemberCommon",
        _ => return None,
    })
}

/// The declarations an object carries (docs/adr/8068): its fire resistance class (its own, else its wall style's or its
/// door or window type's) and a door's or a stair's exit role. Nothing declared is nothing written.
fn declared(e: &Entity, wall_styles: &[kentos_contracts::WallStyleDef], arch: Option<&ArchSettings>) -> (Option<String>, Option<bool>) {
    let fire = match e {
        Entity::Wall(w) => w.wall.arch.fire_rating.as_ref().or_else(|| {
            let id = w.wall.style.as_deref()?;
            wall_styles.iter().find(|s| s.id == id)?.fire_rating.as_ref()
        }),
        Entity::Opening(o) => o.opening.fire_rating.as_ref().or_else(|| {
            let id = o.opening.style.as_deref()?;
            arch?.opening_types.iter().find(|t| t.id == id)?.fire_rating.as_ref()
        }),
        other => other.arch_info().and_then(|a| a.fire_rating.as_ref()),
    };
    let exit = match e {
        Entity::Opening(o) => o.opening.exit.as_ref(),
        Entity::Stair(s) => s.stair.arch.exit.as_ref(),
        _ => None,
    };
    // A word that says nothing is no value: it is missing, never written empty.
    (fire.map(|d| d.value.trim().to_owned()).filter(|v| !v.is_empty()), exit.map(|x| x.exit))
}

/// A material's name as the model writes it: `Ad_Dayanım` when the project's material of that name has a strength
/// (EK-6: “Beton_C25”), else its name.
fn material_named(name: String, arch: Option<&ArchSettings>) -> String {
    let strength = arch.and_then(|a| a.material(&name)).and_then(|m| m.strength.as_deref());
    kentos_contracts::material_label(&name, strength)
}

/// The building to write.
pub fn building(input: Input<'_>) -> (Building, Written) {
    let Input {
        title,
        stamp,
        application,
        levels,
        solids,
        owners,
        parts,
        exact,
        gaps,
        entities,
        wall_style_name,
        wall_styles,
        net_area,
        arch,
        particulars,
        crs,
        emsal,
        groups,
        fixtures,
        coverings,
    } = input;
    let mut written = Written::default();
    // The storeys, by their level id; a drawing without levels has one, Zemin kat at ±0.00.
    let mut levels = levels;
    if levels.is_empty() {
        levels.push((String::new(), "Zemin kat".into(), 0.0, None));
    }
    let storey_of = |level: Option<&str>| -> usize {
        level
            .and_then(|l| levels.iter().position(|(id, ..)| id == l))
            .unwrap_or(0)
    };
    let mut storeys: Vec<Storey> = levels
        .iter()
        .map(|(_, n, floor, height)| Storey {
            id: None,
            name: name("BNK", n),
            long_name: Some(n.clone()),
            elevation: *floor,
            psets: Vec::new(),
            qtos: height
                .map(|h| {
                    vec![Qto {
                        name: "Qto_BuildingStoreyBaseQuantities".into(),
                        quantities: vec![("GrossHeight".into(), Value::Length(h))],
                    }]
                })
                .unwrap_or_default(),
        })
        .collect();
    written.storeys = storeys.len();
    let by_uid: BTreeMap<String, (u128, &Entity)> = entities
        .iter()
        .map(|(bits, u, e)| (u.clone(), (*bits, *e)))
        .collect();
    // The model's solids by their object, in the model's order; a roof's chimney and vent stacks by their part.
    let mut owned: Vec<(String, Vec<usize>)> = Vec::new();
    let mut stacks: Vec<(String, usize, Vec<usize>)> = Vec::new();
    let mut curtains: Vec<(String, Vec<(usize, Part)>)> = Vec::new();
    for (i, owner) in owners.iter().enumerate() {
        let Some(o) = owner else { continue };
        match parts.get(i).copied().flatten() {
            Some(Part::Roof(k)) => {
                match stacks.iter_mut().find(|(r, p, _)| r == o && *p == k) {
                    Some((_, _, list)) => list.push(i),
                    None => stacks.push((o.clone(), k, vec![i])),
                }
                continue;
            }
            Some(part) => {
                match curtains.iter_mut().find(|(w, _)| w == o) {
                    Some((_, list)) => list.push((i, part)),
                    None => curtains.push((o.clone(), vec![(i, part)])),
                }
                continue;
            }
            None => {}
        }
        match owned.iter_mut().find(|(k, _)| k == o) {
            Some((_, list)) => list.push(i),
            None => owned.push((o.clone(), vec![i])),
        }
    }
    let mut elements: Vec<Element> = Vec::new();
    let mut index: BTreeMap<String, usize> = BTreeMap::new();
    for (owner, indices) in &owned {
        let Some(&(id, e)) = by_uid.get(owner) else {
            continue;
        };
        let storey = storey_of(level_of(e));
        let floor = levels[storey].2;
        let mut shapes_of: Vec<Shape> = Vec::new();
        for &i in indices {
            shapes_of.extend(shapes(&solids[i], exact.get(i), floor));
        }
        if shapes_of.is_empty() {
            continue;
        }
        let mut object_type = None;
        // A post or a rail (docs/adr/8070 §8; EK-6 Tablo 6.40): an IfcMember of its type, its name's words.
        let member = match e {
            Entity::Column(c) => c.column.member.as_ref(),
            Entity::Beam(b) => b.beam.member.as_ref(),
            _ => None,
        };
        let mut as_member = member.map(|m| {
            let t = kentos_contracts::Member::TYPES
                .iter()
                .find(|t| **t == m.predefined)
                .copied()
                .unwrap_or("NOTDEFINED");
            (Class::Member, t, m.words())
        });
        if let Some(m) = member {
            object_type = m.object_type();
        }
        // A covering (docs/adr/8070 §10; EK-6 Tablo 6.20): an IfcCovering of its type, its name's words.
        if let Entity::Slab(sl) = e
            && let Some(c) = &sl.slab.covering
        {
            let t = kentos_contracts::Covering::TYPES
                .iter()
                .find(|t| **t == c.predefined)
                .copied()
                .unwrap_or("NOTDEFINED");
            as_member = Some((Class::Covering, t, c.words()));
            object_type = c.object_type();
        }
        let (class, predefined, words): (Class, &'static str, String) = if let Some(found) = as_member {
            found
        } else {
            match e {
                // Its style's name (Dış duvar 25), as the plan's legend and the layer key name it.
                Entity::Wall(w) => (
                    Class::Wall,
                    "STANDARD",
                    wall_style_name(w.wall.style.as_deref()),
                ),
                Entity::Slab(_) => (Class::Slab, "FLOOR", "Döşeme".into()),
                Entity::Column(c) => (
                    Class::Column,
                    "COLUMN",
                    c.column.kind.clone().unwrap_or_else(|| "Kolon".into()),
                ),
                Entity::Beam(b) => match b.beam.role {
                    BeamRole::Lintel => (Class::Beam, "LINTEL", "Lento".into()),
                    BeamRole::BondBeam => (Class::Beam, "BEAM", "Hatıl".into()),
                    _ => (Class::Beam, "BEAM", "Kiriş".into()),
                },
                Entity::Roof(_) => (Class::Roof, "NOTDEFINED", "Çatı".into()),
                Entity::Railing(_) => (Class::Railing, "BALUSTRADE", "Korkuluk".into()),
                // Its type from its walking line's flights and turns (docs/adr/8070 §2).
                Entity::Stair(s) => {
                    let (t, words) = crate::turns::of_stair(&s.stair);
                    object_type = words;
                    (Class::Stair, t, "Merdiven".into())
                }
                Entity::Ramp(r) => {
                    let (t, words) = crate::turns::of_ramp(&r.ramp);
                    object_type = words;
                    (Class::Ramp, t, "Rampa".into())
                }
                _ => continue,
            }
        };
        match class {
            Class::Wall => written.walls += 1,
            Class::Slab => written.slabs += 1,
            Class::Column => written.columns += 1,
            Class::Beam => written.beams += 1,
            Class::Roof => written.roofs += 1,
            Class::Railing => written.railings += 1,
            Class::Stair => written.stairs += 1,
            Class::Ramp => written.ramps += 1,
            Class::Member => written.members += 1,
            Class::Covering => written.coverings += 1,
            _ => {}
        }
        // EK-6's properties the drawing knows: a balcony's or a terrace's slab is outside (Tablo 6.56), a roof is
        // (Tablo 6.52), a column carries (Tablo 6.16: KentOS's column is the frame's, Betonarme unless said).
        let psets = match e {
            Entity::Slab(s) if s.slab.covering.is_none() => vec![Pset {
                name: "Pset_SlabCommon".into(),
                props: vec![(
                    "IsExternal".into(),
                    Value::Bool(s.slab.role != SlabRole::Floor),
                )],
            }],
            // A post carries nothing of the frame's that the drawing knows: no LoadBearing (docs/adr/8070 §8).
            Entity::Column(_) if member.is_none() => vec![Pset {
                name: "Pset_ColumnCommon".into(),
                props: vec![("LoadBearing".into(), Value::Bool(true))],
            }],
            Entity::Roof(_) => vec![Pset {
                name: "Pset_RoofCommon".into(),
                props: vec![("IsExternal".into(), Value::Bool(true))],
            }],
            _ => Vec::new(),
        };
        // Its material (EK-6, `IfcRelAssociatesMaterial`): a column's and a beam's, Betonarme unless said; a slab's;
        // a wall's core's; a stair's and a ramp's the concrete the section draws them in (docs/adr/8070 §2).
        let made_of = material(match e {
            Entity::Stair(_) | Entity::Ramp(_) => Some("Betonarme".into()),
            Entity::Wall(w) => core_material(&w.wall.components),
            Entity::Slab(s) => s.slab.material.clone(),
            Entity::Column(c) => Some(
                c.column
                    .material
                    .clone()
                    .unwrap_or_else(|| "Betonarme".into()),
            ),
            Entity::Beam(b) => Some(
                b.beam
                    .material
                    .clone()
                    .unwrap_or_else(|| "Betonarme".into()),
            ),
            _ => None,
        });
        index.insert(owner.clone(), elements.len());
        elements.push(Element {
            class,
            id: Some(id),
            object: None,
            name: name(class.code(), &words),
            description: None,
            tag: Some(owner.clone()),
            predefined,
            object_type,
            part_of: None,
            storey,
            shapes: shapes_of,
            overall: None,
            material: made_of,
            psets,
            qtos: Vec::new(),
        });
    }
    // The curtain walls (docs/adr/8070 §7; EK-6 Tablo 6.21, 6.40): each an IfcCurtainWall whose own body is its panels
    // (EK-5 has no panel class: K45), and its mullions and transoms each an IfcMember MULLION, its parts.
    for (owner, pieces) in &curtains {
        let Some(&(id, e)) = by_uid.get(owner) else {
            continue;
        };
        let Entity::Wall(w) = e else { continue };
        let Some(grid) = w.wall.curtain.as_ref() else {
            continue;
        };
        let storey = storey_of(level_of(e));
        let floor = levels[storey].2;
        let panels: Vec<Shape> = pieces
            .iter()
            .filter(|(_, p)| *p == Part::Panel)
            .flat_map(|(i, _)| shapes(&solids[*i], exact.get(*i), floor))
            .collect();
        let whole = elements.len();
        index.insert(owner.clone(), whole);
        let words = match w.wall.style.as_deref() {
            Some(style) => wall_style_name(Some(style)),
            None => "Giydirme cephe".to_owned(),
        };
        elements.push(Element {
            class: Class::CurtainWall,
            id: Some(id),
            object: None,
            name: name(Class::CurtainWall.code(), &words),
            description: None,
            tag: Some(owner.clone()),
            predefined: "USERDEFINED",
            object_type: Some("Giydirme cephe".to_owned()),
            part_of: None,
            storey,
            shapes: panels,
            overall: None,
            material: material(grid.panel.clone()),
            psets: Vec::new(),
            qtos: Vec::new(),
        });
        written.curtain_walls += 1;
        for (i, part) in pieces {
            let (k, words) = match part {
                Part::Mullion(k) => (*k as u64, "Dikme"),
                Part::Transom(k) => (1_000_000 + *k as u64, "Kayıt"),
                _ => continue,
            };
            let body = shapes(&solids[*i], exact.get(*i), floor);
            if body.is_empty() {
                continue;
            }
            elements.push(Element {
                class: Class::Member,
                id: Some(crate::step::fnv(owner, k)),
                object: Some(id),
                name: name(Class::Member.code(), words),
                description: None,
                tag: Some(owner.clone()),
                predefined: "MULLION",
                object_type: None,
                part_of: Some(whole),
                storey,
                shapes: body,
                overall: None,
                material: material(grid.frames.clone()),
                psets: Vec::new(),
                qtos: Vec::new(),
            });
            written.members += 1;
        }
    }
    // Pset_WallCommon's IsExternal (EK-6 Tablo 6.66): a wall with a corner on the outline of its storey's walls and
    // columns together. The columns close it: the section model stops the walls at them, and the walls of a framed
    // storey alone are pieces with an outline each, an inner wall's end on it (the section building: all 8 inner
    // walls outside).
    let mut exterior: Vec<bool> = vec![false; elements.len()];
    for s in 0..storeys.len() {
        let mut rings: Vec<Vec<P>> = Vec::new();
        let mut whose: Vec<usize> = Vec::new();
        for (k, el) in elements
            .iter()
            .enumerate()
            .filter(|(_, el)| matches!(el.class, Class::Wall | Class::Column | Class::Member) && el.storey == s)
        {
            for sh in &el.shapes {
                if let Shape::Prism { rings: rs, bulges, .. } = sh {
                    for r in &flat_rings(rs, bulges) {
                        rings.push(r.iter().map(|p| P::new(p[0], p[1])).collect());
                        whose.push(k);
                    }
                }
            }
        }
        for i in kentos_geometry_core::geom::wall::exterior(&rings) {
            let k = whose[i];
            match elements[k].class {
                Class::Wall => exterior[k] = true,
                // A curtain wall is outside where one of its frames is on the outline; a post or a rail where it is
                // (docs/adr/8070 §8).
                Class::Member => match elements[k].part_of {
                    Some(whole) => exterior[whole] = true,
                    None => exterior[k] = true,
                },
                _ => {}
            }
        }
    }
    // A curtain wall's frames are as outside as it is.
    for k in 0..elements.len() {
        if let Some(whole) = elements[k].part_of {
            exterior[k] = exterior[whole];
        }
    }
    for (k, el) in elements.iter_mut().enumerate() {
        let set = match (el.class, el.part_of) {
            (Class::Wall, _) => "Pset_WallCommon",
            (Class::CurtainWall, _) => "Pset_CurtainWallCommon",
            (Class::Member, _) => "Pset_MemberCommon",
            _ => continue,
        };
        el.psets.push(Pset {
            name: set.into(),
            props: vec![("IsExternal".into(), Value::Bool(exterior[k]))],
        });
    }
    // The openings, each with its door or window (EK-6 Tablo 6.25, 6.67).
    let mut openings: Vec<Opening> = Vec::new();
    let mut opened: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    for gap in &gaps {
        let Some(owner) = gap.owner.as_ref() else {
            continue;
        };
        // One opening element per opening: a second outline of it would give its door a second GlobalId.
        if opened.contains(owner.as_str()) {
            continue;
        }
        let Some(&(id, Entity::Opening(o))) = by_uid.get(owner) else {
            continue;
        };
        let Some(&host) = index.get(&o.opening.host) else {
            continue;
        };
        let storey = elements[host].storey;
        let floor = levels[storey].2;
        let ring: Vec<[f64; 2]> = gap.ring.iter().map(|p| pt(*p)).collect();
        let (bottom, top) = (gap.sill - floor, gap.head - floor);
        let mark = o.opening.number.clone();
        let filler = match gap.form {
            OpeningForm::Door | OpeningForm::Window => {
                let door = gap.form == OpeningForm::Door;
                let (class, pset) = if door {
                    written.doors += 1;
                    (Class::Door, "Pset_DoorCommon")
                } else {
                    written.windows += 1;
                    (Class::Window, "Pset_WindowCommon")
                };
                let words = mark
                    .clone()
                    .unwrap_or_else(|| if door { "Kapı" } else { "Pencere" }.into());
                elements.push(Element {
                    class,
                    id: Some(id),
                    object: None,
                    name: name(class.code(), &words),
                    description: None,
                    tag: Some(mark.clone().unwrap_or_else(|| owner.clone())),
                    predefined: if door { "DOOR" } else { "WINDOW" },
                    object_type: None,
                    part_of: None,
                    storey,
                    shapes: vec![Shape::Prism {
                        rings: vec![filler_ring(
                            &o.opening.place,
                            o.opening.build.as_ref(),
                            o.opening.side == OpeningSide::Right,
                            &ring,
                        )],
                        bulges: Vec::new(),
                        bottom,
                        top,
                    }],
                    overall: Some((gap.head - gap.sill, o.opening.width)),
                    material: None,
                    psets: vec![Pset {
                        name: pset.into(),
                        props: vec![(
                            "IsExternal".into(),
                            Value::Bool(exterior.get(host).copied().unwrap_or(false)),
                        )],
                    }],
                    qtos: Vec::new(),
                });
                Some(elements.len() - 1)
            }
            _ => None,
        };
        written.openings += 1;
        opened.insert(owner.as_str());
        openings.push(Opening {
            id: None,
            name: name("ACL", mark.as_deref().unwrap_or("Açıklık")),
            host,
            ring,
            bulges: gap.bulges.clone(),
            bottom,
            top,
            filler,
            predefined: if gap.form == OpeningForm::Niche {
                "RECESS"
            } else {
                "OPENING"
            },
        });
    }
    // Niches and flues (docs/adr/8070 §3): the section model takes them out of their walls as boxes; the model writes a
    // niche as a recess in its wall, and a flue's hollow as an opening through its wall's height that a chimney fills.
    let mut pieces: Vec<Piece> = Vec::new();
    for (bits, uid, e) in &entities {
        let Entity::Opening(o) = e else { continue };
        let f = &o.opening;
        let niche = f.form == OpeningForm::Niche;
        if !niche && f.form != OpeningForm::Flue {
            continue;
        }
        let Some(&host) = index.get(&f.host) else {
            continue;
        };
        let Ok(place) = <[Vec2; 4]>::try_from(f.place.clone()) else {
            continue;
        };
        let Some(ring) = Recess::of(niche, f.side == OpeningSide::Right, f.inset, f.depth)
            .and_then(|r| r.band(place.map(|q| P::new(q.x, q.y))))
        else {
            continue;
        };
        let ring: Vec<[f64; 2]> = ring.into_iter().map(pt).collect();
        let storey = elements[host].storey;
        let Some(wall_top) = elements[host]
            .shapes
            .iter()
            .filter_map(|sh| match sh {
                Shape::Prism { top, .. } => Some(*top),
                Shape::Faces { .. } => None,
            })
            .reduce(f64::max)
        else {
            continue;
        };
        let bottom = f.sill.unwrap_or(0.0);
        let top = f.head.unwrap_or(wall_top);
        if top <= bottom + 1e-9 {
            continue;
        }
        written.openings += 1;
        opened.insert(uid.as_str());
        openings.push(Opening {
            // A niche is its object; a flue's object is its chimney, its hollow an id of its own.
            id: Some(if niche { *bits } else { crate::step::fnv(uid, 1) }),
            name: name("ACL", if niche { "Niş" } else { "Baca boşluğu" }),
            host,
            ring: ring.clone(),
            bulges: Vec::new(),
            bottom,
            top,
            filler: None,
            predefined: if niche { "RECESS" } else { "OPENING" },
        });
        if !niche {
            let floor = levels[storey].2;
            pieces.push(Piece {
                storey,
                ring,
                bottom: floor + bottom,
                top: floor + top,
                flue: Some((*bits, uid.clone(), openings.len() - 1, elements[host].material.clone())),
                stack: None,
            });
        }
    }
    // The elements' base quantities (docs/adr/8071 §2; EK-6's optional rows): from their bodies with their arcs, less the
    // openings and recesses that cut them.
    for (owner, &k) in &index {
        let Some(&(_, e)) = by_uid.get(owner) else {
            continue;
        };
        let cuts: Vec<&Opening> = openings.iter().filter(|o| o.host == k).collect();
        let el = &elements[k];
        let qto = match (el.class, e) {
            (Class::Wall, Entity::Wall(w)) => Some(crate::quantities::wall(&w.wall, &el.shapes, &cuts)),
            (Class::Slab, Entity::Slab(_)) => crate::quantities::slab(&el.shapes, &cuts),
            (Class::Covering, _) => crate::quantities::covering(&el.shapes),
            (Class::Column, Entity::Column(_)) => crate::quantities::column(&el.shapes, &cuts),
            (Class::Beam, Entity::Beam(b)) => Some(crate::quantities::beam(&b.beam, &el.shapes, &cuts)),
            _ => None,
        };
        if let Some(q) = qto {
            elements[k].qtos.push(q);
        }
    }
    // A roof's chimney and vent stacks (docs/adr/8044 §6), from the top of the storey it stands on up to their tops.
    for (roof, k, list) in &stacks {
        let Some(&(bits, e)) = by_uid.get(roof) else {
            continue;
        };
        let Entity::Roof(r) = e else { continue };
        let storey = storey_of(level_of(e));
        let words = match r.roof.parts.get(*k).map(|p| p.kind) {
            Some(RoofPartKind::Vent) => "Hava bacası",
            _ => "Duman bacası",
        };
        for &i in list {
            let solid = &solids[i];
            let (Height::Prism { bottom, top }, Some(ring)) = (&solid.height, solid.rings.first()) else {
                continue;
            };
            pieces.push(Piece {
                storey,
                ring: ring.iter().map(|q| pt(*q)).collect(),
                bottom: *bottom,
                top: *top,
                flue: None,
                stack: Some((bits, roof.clone(), *k, words)),
            });
        }
    }
    // The chimneys (EK-6 Tablo 6.14): a flue and the stack over it, and the flues over one another, are one chimney
    // where one's middle lies in the other's outline in plan; its storey the lowest piece's.
    for members in chimney_groups(&pieces) {
        let low = &pieces[members[0]];
        let (storey, floor) = (low.storey, levels[low.storey].2);
        let flue = members.iter().find_map(|&i| pieces[i].flue.as_ref());
        let stack = members.iter().find_map(|&i| pieces[i].stack.as_ref());
        let words = stack.map_or("Baca", |s| s.3);
        let (id, object, tag) = match (flue, stack) {
            (Some((bits, uid, ..)), _) => (*bits, None, uid.clone()),
            (None, Some((roof_bits, roof, k, _))) => (
                crate::step::fnv(&format!("{roof}/baca"), *k as u64),
                Some(*roof_bits),
                format!("{roof}/{}", k + 1),
            ),
            (None, None) => continue,
        };
        // Its material: the wall its lowest flue is in, else the stack's masonry as the section draws it (K43).
        let made_of = flue
            .and_then(|f| f.3.clone())
            .or_else(|| stack.map(|_| "Tuğla".to_owned()));
        let k = elements.len();
        elements.push(Element {
            class: Class::Chimney,
            id: Some(id),
            object,
            name: name(Class::Chimney.code(), words),
            description: None,
            tag: Some(tag),
            predefined: "USERDEFINED",
            object_type: Some(words.to_owned()),
            part_of: None,
            storey,
            shapes: members
                .iter()
                .map(|&i| Shape::Prism {
                    rings: vec![pieces[i].ring.clone()],
                    bulges: Vec::new(),
                    bottom: pieces[i].bottom - floor,
                    top: pieces[i].top - floor,
                })
                .collect(),
            overall: None,
            material: made_of,
            psets: Vec::new(),
            qtos: Vec::new(),
        });
        written.chimneys += 1;
        for &i in &members {
            if let Some((_, _, o, _)) = pieces[i].flue {
                openings[o].filler = Some(k);
            }
        }
    }
    // The blocks that are building elements (docs/adr/8070 §5; EK-6 Tablo 6.35, 6.53, 6.64): each its class and type
    // (USERDEFINED: its block's name its object type), named for its block, its body a box over its outline from its
    // underside up its height; with no height it has no body. A lift carries its rated capacity.
    for f in &fixtures {
        let class = match f.element.class {
            kentos_contracts::BlockElementClass::Furniture => Class::Furniture,
            kentos_contracts::BlockElementClass::SanitaryTerminal => Class::SanitaryTerminal,
            kentos_contracts::BlockElementClass::TransportElement => Class::TransportElement,
        };
        let predefined = match f.element.predefined.as_deref() {
            Some(t) => f
                .element
                .class
                .types()
                .iter()
                .copied()
                .find(|x| *x == t)
                .unwrap_or("NOTDEFINED"),
            None => "NOTDEFINED",
        };
        let bottom = f.element.elevation.unwrap_or(0.0);
        let shapes = f
            .element
            .height
            .filter(|_| f.ring.len() >= 3)
            .map(|h| Shape::Prism {
                rings: vec![f.ring.clone()],
                bulges: Vec::new(),
                bottom,
                top: bottom + h,
            })
            .into_iter()
            .collect();
        let mut psets = Vec::new();
        if class == Class::TransportElement {
            if let Some(n) = f.people {
                give(&mut psets, "Pset_TransportElementCommon", "CapacityPeople", Value::Count(n));
            }
            if let Some(kg) = f.weight {
                give(&mut psets, "Pset_TransportElementCommon", "CapacityWeight", Value::Mass(kg));
            }
        }
        elements.push(Element {
            class,
            id: Some(f.id),
            object: None,
            name: name(class.code(), &f.name),
            description: None,
            tag: Some(f.uid.clone()),
            predefined,
            object_type: (predefined == "USERDEFINED").then(|| f.name.clone()),
            part_of: None,
            storey: storey_of(f.level.as_deref()),
            shapes,
            overall: None,
            material: None,
            psets,
            qtos: Vec::new(),
        });
        written.fixtures += 1;
    }
    // The spaces (EK-6 Tablo 6.58): their numbers and names, their finishes as the room list's.
    let mut spaces = Vec::new();
    let mut net_floor: Vec<Option<f64>> = vec![None; storeys.len()];
    for (bits, uid, e) in &entities {
        let Entity::Space(s) = e else { continue };
        let f = &s.space;
        let storey = storey_of(f.arch.level.as_deref());
        // Its contour with its arcs (docs/adr/8071 §1): a room along a curved wall is bounded by the arc itself.
        let (rings, bulges): (Vec<Vec<[f64; 2]>>, Vec<Vec<f64>>) = f
            .parts
            .iter()
            .flat_map(|part| std::iter::once(&part.outer).chain(&part.holes))
            .filter(|r| {
                let pts: Vec<P> = r.pts.iter().map(|q| P::new(q.x, q.y)).collect();
                kentos_geometry_core::entity::polygon_ring(&pts, r.bulges.as_deref()).len() >= 3
            })
            .map(|r| (r.pts.iter().map(|q| [q.x, q.y]).collect(), r.bulges.clone().unwrap_or_default()))
            .unzip();
        if rings.is_empty() {
            written.left.push(Left {
                id: uid.clone(),
                what: "mahal",
                why: Why::NoArea,
            });
            continue;
        }
        let bottom = f.floor_offset.unwrap_or(0.0);
        let top = bottom + f.height.or(levels[storey].3).unwrap_or(2.8);
        let blank = |v: &Option<String>| v.as_deref().is_none_or(|t| t.trim().is_empty());
        if blank(&f.number) || blank(&f.name) {
            written.unnamed.push(uid.clone());
        }
        let words = [f.number.as_deref(), f.name.as_deref()]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join(" ");
        let mut props = Vec::new();
        for (key, v) in [
            ("FloorCovering", &f.finishes.floor),
            ("WallCovering", &f.finishes.wall),
            ("CeilingCovering", &f.finishes.ceiling),
        ] {
            if let Some(v) = v.as_deref().filter(|v| !v.trim().is_empty()) {
                props.push((key.to_owned(), label(v)));
            }
        }
        // Their thicknesses (docs/adr/8070 §6, EK-6 Tablo 6.20's first way): the floor slab's finish and the plaster
        // under the slab over, as the vertical model holds them; the walls' finish not yet.
        let (floor_t, ceiling_t) = coverings.get(uid).copied().unwrap_or((None, None));
        if let Some(t) = floor_t {
            props.push(("FloorCoveringThickness".to_owned(), Value::PositiveLength(t)));
        }
        if let Some(t) = ceiling_t {
            props.push(("CeilingCoveringThickness".to_owned(), Value::PositiveLength(t)));
        }
        let psets = if props.is_empty() {
            Vec::new()
        } else {
            vec![Pset {
                name: "Pset_SpaceCoveringRequirements".into(),
                props,
            }]
        };
        let net = net_area(f);
        if let Some(a) = net {
            *net_floor[storey].get_or_insert(0.0) += a;
        }
        let qtos = net
            .map(|a| {
                vec![Qto {
                    name: "Qto_SpaceBaseQuantities".into(),
                    quantities: vec![("NetFloorArea".into(), Value::Area(a))],
                }]
            })
            .unwrap_or_default();
        spaces.push(Space {
            id: Some(*bits),
            name: name("MHL", if words.is_empty() { "Mahal" } else { &words }),
            long_name: f.name.clone(),
            storey,
            rings,
            bulges,
            bottom,
            top,
            predefined: "INTERNAL",
            psets,
            qtos,
        });
    }
    written.spaces = spaces.len();
    for (_, uid, e) in &entities {
        let what = match e {
            Entity::Wall(_) => "duvar",
            Entity::Slab(_) => "döşeme",
            Entity::Column(_) => "kolon",
            Entity::Beam(_) => "kiriş",
            Entity::Roof(_) => "çatı",
            Entity::Railing(_) => "korkuluk",
            Entity::Stair(_) => "merdiven",
            Entity::Ramp(_) => "rampa",
            Entity::Opening(o) => match o.opening.form {
                OpeningForm::Door => "kapı",
                OpeningForm::Window => "pencere",
                _ => "açıklık",
            },
            _ => continue,
        };
        let (held, why) = match e {
            Entity::Opening(_) => (opened.contains(uid.as_str()), Why::NoWall),
            _ => (index.contains_key(uid), Why::NoBody),
        };
        if !held {
            written.left.push(Left {
                id: uid.clone(),
                what,
                why,
            });
        }
    }
    // A storey's and the building's net floor area (EK-6 Tablo 6.8, 6.7): its rooms' net areas summed; the building's
    // height its storeys', only where it is the same for all (Tablo 6.7: "sadece yüksekliğin sabit olduğu durumda").
    for (storey, net) in storeys.iter_mut().zip(&net_floor) {
        if let Some(a) = net {
            match storey
                .qtos
                .iter_mut()
                .find(|q| q.name == "Qto_BuildingStoreyBaseQuantities")
            {
                Some(q) => q.quantities.push(("NetFloorArea".into(), Value::Area(*a))),
                None => storey.qtos.push(Qto {
                    name: "Qto_BuildingStoreyBaseQuantities".into(),
                    quantities: vec![("NetFloorArea".into(), Value::Area(*a))],
                }),
            }
        }
    }
    let mut building_quantities = Vec::new();
    let heights: Vec<Option<f64>> = levels.iter().map(|l| l.3).collect();
    if let Some(Some(h)) = heights.first()
        && heights
            .iter()
            .all(|x| x.is_some_and(|x| (x - h).abs() < 1e-9))
    {
        building_quantities.push(("Height".to_owned(), Value::Length(*h)));
    }
    if net_floor.iter().any(Option::is_some) {
        let total: f64 = net_floor.iter().flatten().sum();
        building_quantities.push(("NetFloorArea".to_owned(), Value::Area(total)));
    }
    // The project, its site and its building (EK-6 Tablo 6.7, 6.55; EK-7), their words the drawing's künye's.
    let k = &particulars;
    let mut site_psets = Vec::new();
    let mut site_qtos = Vec::new();
    if let Some(z) = arch.and_then(|a| a.zoning.as_ref()) {
        let mut props = Vec::new();
        if let Some(far) = z.far {
            props.push(("FloorAreaRatio".to_owned(), Value::Ratio(far)));
        }
        if let Some(c) = z.coverage {
            props.push(("SiteCoverageRatio".to_owned(), Value::Ratio(c)));
        }
        if !props.is_empty() {
            site_psets.push(Pset {
                name: "Pset_SiteCommon".into(),
                props,
            });
        }
        if z.parcel_area > 0.0 {
            site_qtos.push(Qto {
                name: "Qto_SiteBaseQuantities".into(),
                quantities: vec![("GrossArea".into(), Value::Area(z.parcel_area))],
            });
        }
    }
    // Tablo 6.55: the address as the regulation's labels map it (İl → Town, İlçe → Region), the parcel ("ada/parsel")
    // and its title deed, and TREpys_ParselOzellikSeti: the plan's TAKS and KAKS (emsal), the gardens' distances.
    let address = k.address();
    if let Some(p) = address_pset(&address) {
        site_psets.push(p.clone());
    }
    let mut land = Vec::new();
    if let Some(id) = k.land_id() {
        land.push(("LandID".to_owned(), label(&id)));
    }
    if let Some(t) = &k.title_deed {
        land.push(("LandTitleID".to_owned(), label(t)));
    }
    if !land.is_empty() {
        site_psets.push(Pset {
            name: "Pset_LandRegistration".into(),
            props: land,
        });
    }
    let mut parcel = Vec::new();
    if let Some(z) = arch.and_then(|a| a.zoning.as_ref()) {
        if let Some(c) = z.coverage {
            parcel.push(("TAKS".to_owned(), Value::Real(c)));
        }
        if let Some(far) = z.far {
            parcel.push(("KAKS".to_owned(), Value::Real(far)));
        }
    }
    for (key, v) in [
        ("CekmeOn", k.setbacks[0]),
        ("CekmeYan", k.setbacks[1]),
        ("CekmeArka", k.setbacks[2]),
    ] {
        if let Some(v) = v {
            parcel.push((key.to_owned(), Value::Real(v)));
        }
    }
    if !parcel.is_empty() {
        site_psets.push(Pset {
            name: "TREpys_ParselOzellikSeti".into(),
            props: parcel,
        });
    }
    // Tablo 6.7: the building's temporary id (its project code and building code; not permanent until it is
    // registered), its use, its planned gross area and its footprint.
    let mut building_psets = Vec::new();
    if let Some(p) = address_pset(&address) {
        building_psets.push(p);
    }
    let mut common = Vec::new();
    // The künye's 000000 is a project not yet given its code (EK-1): no id of a building.
    if let Some(code) = k
        .project_code
        .as_ref()
        .filter(|c| c.chars().any(|ch| ch != '0'))
    {
        let id = format!("{code}_{}", k.building_code.as_deref().unwrap_or("00"));
        common.push(("BuildingID".to_owned(), Value::Id(id)));
        common.push(("IsPermanentID".to_owned(), Value::Bool(false)));
    }
    if let Some(u) = &k.occupancy {
        common.push(("OccupancyType".to_owned(), label(u)));
    }
    if let Some(a) = k.gross_area {
        common.push(("GrossPlannedArea".to_owned(), Value::Area(a)));
    }
    // The rooms' net areas together (EK-6 Tablo 6.7 `NetPlannedArea`; docs/adr/8068), as the model's spaces carry them.
    let net: f64 = spaces.iter().filter_map(|s| s.qtos.iter().find_map(|q| q.quantities.iter().find_map(|(k, v)| match (k.as_str(), v) {
        ("NetFloorArea", Value::Area(a)) => Some(*a),
        _ => None,
    }))).sum();
    if net > 0.0 {
        common.push(("NetPlannedArea".to_owned(), Value::Area(net)));
    }
    if let Some(c) = &k.construction {
        common.push(("ConstructionMethod".to_owned(), label(c)));
    }
    if let Some(d) = arch.and_then(|a| a.performance.as_ref()).and_then(|p| p.fire_protection_class.as_ref()) {
        common.push(("FireProtectionClass".to_owned(), label(&d.value)));
    }
    common.push((
        "NumberOfStoreys".to_owned(),
        Value::Count(storeys.len() as i64),
    ));
    building_psets.push(Pset {
        name: "Pset_BuildingCommon".into(),
        props: common,
    });
    if let Some(a) = k.footprint {
        building_quantities.push(("FootprintArea".to_owned(), Value::Area(a)));
    }
    if let Some(a) = k.gross_area {
        building_quantities.push(("GrossFloorArea".to_owned(), Value::Area(a)));
    }
    let title = title.to_owned();
    let work = k.work.clone().unwrap_or_else(|| title.clone());
    // EK-7 Tablo 7.1: the project's name is its code (EK-1), its long name the work's.
    let project_name = k.project_code.clone().unwrap_or_else(|| title.clone());
    let actors = k.architect().into_iter().collect();
    // The author's declarations (docs/adr/8068): each element's fire resistance class and exit role, its material
    // named with its strength.
    {
        let at: std::collections::HashMap<u128, usize> =
            elements.iter().enumerate().filter_map(|(i, e)| Some((e.id?, i))).collect();
        for (id, _, e) in &entities {
            let Some(&i) = at.get(id) else { continue };
            let class = elements[i].class;
            let (fire, exit) = declared(e, wall_styles, arch);
            if let (Some(set), Some(f)) = (common_set(class), fire) {
                give(&mut elements[i].psets, set, "FireRating", Value::Label(f));
            }
            if let (Some(set), Some(x)) = (common_set(class), exit)
                && matches!(class, Class::Door | Class::Stair)
            {
                give(&mut elements[i].psets, set, "FireExit", Value::Bool(x));
            }
        }
        for e in &mut elements {
            if let Some(m) = e.material.take() {
                e.material = Some(material_named(m, arch));
            }
        }
    }
    // The emsal zones (docs/adr/8069): each storey's rooms by how they count, between its floor and its rooms' tops.
    let zones: Vec<Zone> = emsal
        .iter()
        .filter(|a| a.rings.iter().any(|r| r.len() >= 3))
        .map(|a| {
            let storey = storey_of(a.level.as_deref());
            let (state, words, description) = emsal_state(a.count, a.moved);
            let storey_name = storeys.get(storey).map_or_else(String::new, |s| s.long_name.clone().unwrap_or_else(|| s.name.clone()));
            let top = spaces
                .iter()
                .filter(|s| s.storey == storey)
                .map(|s| s.top)
                .fold(levels.get(storey).and_then(|l| l.3).unwrap_or(0.0), f64::max);
            Zone {
                id: None,
                name: name("MZN", &format!("Emsal {state} {storey_name}")),
                long_name: Some(format!("{storey_name} {words}")),
                description: Some(description),
                object_type: Some("EMSAL".into()),
                predefined: "USERDEFINED",
                storey,
                rings: a.rings.clone(),
                bottom: 0.0,
                top: if top > 0.0 { top } else { 3.0 },
                psets: vec![Pset {
                    name: "TREpys_EmsalOzellikSeti".into(),
                    props: vec![("EmsalDurumu".into(), Value::Label(state.into()))],
                }],
            }
        })
        .collect();
    // The units as groups of their rooms (EK-6 Tablo 6.68): an independent unit, an annex, a common area.
    let group_list: Vec<Group> = groups
        .iter()
        .map(|g| {
            use kentos_contracts::UnitKind as K;
            let kind = match g.kind {
                K::Unit => "Bağımsız bölüm",
                K::Annex => "Eklenti",
                K::Common => "Ortak alan",
                K::Outdoor => "Dış alan",
            };
            let spaces_of: Vec<usize> = spaces
                .iter()
                .enumerate()
                .filter(|(_, s)| s.id.is_some_and(|id| g.rooms.contains(&id)))
                .map(|(i, _)| i)
                .collect();
            Group {
                id: g.id,
                name: name("ZON", &g.number),
                long_name: Some(g.name.clone().unwrap_or_else(|| format!("{kind} {}", g.number))),
                object_type: Some(kind.into()),
                spaces: spaces_of,
                psets: Vec::new(),
            }
        })
        .filter(|g| !g.spaces.is_empty())
        .collect();
    let grids = grids_of(&entities, &storey_of, &mut written);
    let building = Building {
        view: Default::default(),
        file: k.file_stem(&title),
        stamp: stamp.to_owned(),
        application: application.to_owned(),
        phase: None,
        project: Place {
            name: project_name,
            long_name: Some(work.clone()),
            ..Place::default()
        },
        actors,
        site: Place {
            name: name("SAH", "Parsel"),
            long_name: k.land_id(),
            psets: site_psets,
            qtos: site_qtos,
            address: address.clone(),
            ..Place::default()
        },
        site_elevation: arch.and_then(|a| a.site.as_ref()).and_then(|s| s.datum),
        building: Place {
            name: name("BNA", &work),
            long_name: Some(work),
            psets: building_psets,
            qtos: if building_quantities.is_empty() {
                Vec::new()
            } else {
                vec![Qto {
                    name: "Qto_BuildingBaseQuantities".into(),
                    quantities: building_quantities,
                }]
            },
            address,
            ..Place::default()
        },
        storeys,
        elements,
        openings,
        spaces,
        zones,
        groups: group_list,
        grids,
        crs,
    };
    (building, written)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn v(x: f64, y: f64) -> Vec2 {
        Vec2 { x, y }
    }

    /// A 1.2 m opening in a 0.25 m wall along x, its left face at y 0.125: the start jamb at x 0, the end at 1.2.
    fn place() -> Vec<Vec2> {
        vec![v(0.0, 0.125), v(0.0, -0.125), v(1.2, 0.125), v(1.2, -0.125)]
    }

    fn close(a: &[[f64; 2]], b: &[[f64; 2]]) -> bool {
        a.len() == b.len()
            && a.iter()
                .zip(b)
                .all(|(p, q)| (p[0] - q[0]).abs() < 1e-12 && (p[1] - q[1]).abs() < 1e-12)
    }

    #[test]
    fn a_door_s_or_a_window_s_body_is_its_frame_s_band_across_the_wall() {
        let through = vec![[0.0, 0.125], [1.2, 0.125], [1.2, -0.125], [0.0, -0.125]];
        // No construction, or one without the frame's depth: the opening through the whole wall.
        assert_eq!(filler_ring(&place(), None, false, &through), through);
        let plain = OpeningBuild::default();
        assert_eq!(
            filler_ring(&place(), Some(&plain), false, &through),
            through
        );
        // A 7 cm frame centred: 0.09 to 0.16 from the left face.
        let framed = OpeningBuild {
            frame_depth: Some(0.07),
            ..OpeningBuild::default()
        };
        let centred = filler_ring(&place(), Some(&framed), false, &through);
        assert!(close(
            &centred,
            &[[0.0, 0.035], [1.2, 0.035], [1.2, -0.035], [0.0, -0.035]]
        ));
        // 2 cm from the face it opens to: the left's, then the right's.
        let set = OpeningBuild {
            frame_depth: Some(0.07),
            frame_offset: Some(0.02),
            ..OpeningBuild::default()
        };
        assert!(close(
            &filler_ring(&place(), Some(&set), false, &through),
            &[[0.0, 0.105], [1.2, 0.105], [1.2, 0.035], [0.0, 0.035]]
        ));
        assert!(close(
            &filler_ring(&place(), Some(&set), true, &through),
            &[[0.0, -0.035], [1.2, -0.035], [1.2, -0.105], [0.0, -0.105]]
        ));
        // A frame as deep as the wall fills it.
        let deep = OpeningBuild {
            frame_depth: Some(0.3),
            ..OpeningBuild::default()
        };
        assert_eq!(filler_ring(&place(), Some(&deep), false, &through), through);
    }
}
