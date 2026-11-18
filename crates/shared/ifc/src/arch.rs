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
    /// The voids' parts over their storeys' floor slabs (docs/adr/8071 §5): each an opening of its slab.
    pub slab_cuts: Vec<SlabCut>,
    pub entities: Vec<(u128, String, &'a Entity)>,
    pub wall_style_name: &'a dyn Fn(Option<&str>) -> String,
    /// The project's wall styles: a wall's fire resistance class is its own, else its style's (docs/adr/8068).
    pub wall_styles: &'a [kentos_contracts::WallStyleDef],
    pub net_area: &'a mut dyn FnMut(&SpaceFields) -> Option<f64>,
    pub arch: Option<&'a ArchSettings>,
    pub particulars: Particulars,
    /// The project's system with the model's place in it (EK-7 Tablo 7.6–7.7; `crate::crs`), or why the file says
    /// none (docs/adr/8072 §3): no system, one a map conversion cannot carry, or ±0.00 of unknown height.
    pub crs: Result<crate::crs::Crs, String>,
    /// Each storey's emsal areas as the ratios count the rooms (docs/adr/8069): the spatial zones' grounds.
    pub emsal: Vec<EmsalArea>,
    /// The building program's units with their rooms (docs/adr/8069): the groups of spaces.
    pub groups: Vec<UnitGroup>,
    /// The blocks that are building elements (docs/adr/8070 §5): furniture, sanitary fixtures, lifts.
    pub fixtures: Vec<Fixture>,
    /// A room's floor, ceiling and wall covering thicknesses as the vertical model and its walls' faces hold them
    /// (docs/adr/8070 §6, 8076 §7), by its persistent id; a room not here, or a side none, has none.
    pub coverings: BTreeMap<String, (Option<f64>, Option<f64>, Option<f64>)>,
    /// A room's floor and ceiling as the vertical model finds them (docs/adr/8040 §3, 8071 §7), by its persistent id;
    /// a room not here stands on its own offset up its own height or its storey's.
    pub rooms: BTreeMap<String, RoomLevels>,
    /// The program's building the file holds (docs/adr/8072 §2); none: the project as one building.
    pub building: Option<BuildingOf>,
    /// True north, radians counter-clockwise from the model's y axis (EK-7 Tablo 7.8 `TrueNorth`; docs/adr/8076 §6),
    /// or why the file cannot say it.
    pub north: Result<f64, String>,
}

/// The program's building a file holds (docs/adr/8072 §2): its name, code, use and ±0.00 over the project's, and whether
/// the project has others (its own areas are then written, not the künye's, which are the project's; §4).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BuildingOf {
    /// Its id in the program: its GlobalId's seed.
    pub id: String,
    pub name: String,
    pub code: Option<String>,
    pub usage: Option<String>,
    pub zero: f64,
    pub several: bool,
    /// Its own yapı inşaat alanı and taban alanı (docs/adr/8072 §4): the levels it counts, a shared one only where it is
    /// chosen, and its ground storey's rooms; the buildings' together are the project's. None without the parcel's
    /// zoning (the künye's areas are measured against it).
    pub gross_area: Option<f64>,
    pub footprint: Option<f64>,
}

/// A room's floor and ceiling, metres over ±0.00 (docs/adr/8040 §3): its finished floor; its ceiling (its asma tavan,
/// its height typed, the plastered underside of the slab over it, else its storey's top) and whether it is one (not
/// its storey's top alone) and an asma tavan; with none, the roof over it (docs/adr/8071 §11).
#[derive(Clone, Debug, PartialEq)]
pub struct RoomLevels {
    pub floor: f64,
    pub ceiling: f64,
    pub known: bool,
    pub suspended: bool,
    /// The faces of the roof over it when no slab, asma tavan or height of its own bounds it (docs/adr/8071 §11): each
    /// face's outline in plan and its underside's plane over ±0.00; empty: none.
    pub roof: Vec<crate::under_roof::UnderFace>,
    /// Its construction floor (docs/adr/8076 §3; IFC 4.3's "top surface of the construction floor"): the structure's
    /// top of the level slab under it over ±0.00; none without one.
    pub structure: Option<f64>,
    /// The level slab over it: its structure's top and underside over ±0.00; none without one.
    pub over: Option<(f64, f64)>,
}

/// One room's say in its storey's levels (docs/adr/8076 §3): its area, and the structural slab level under it and its
/// gross and net heights to the slab over it, where known.
struct Vote {
    weight: f64,
    ssl: Option<f64>,
    gross: Option<f64>,
    net: Option<f64>,
}

/// The value more than half of `total`'s weight holds (values within a nanometre are one): IFC 4.3's "significantly
/// more prominent" elevation, as KentOS reads it (docs/adr/8076 §3); none without one.
fn prominent(values: impl Iterator<Item = (Option<f64>, f64)>, total: f64) -> Option<f64> {
    let mut held: Vec<(f64, f64)> = Vec::new();
    for (v, w) in values {
        let Some(v) = v else { continue };
        match held.iter_mut().find(|(x, _)| (*x - v).abs() <= 1e-9) {
            Some(h) => h.1 += w,
            None => held.push((v, w)),
        }
    }
    held.into_iter().find(|(_, w)| *w > total / 2.0).map(|(v, _)| v)
}

/// A void's part over a floor slab (docs/adr/8071 §5): the slab's persistent id, the void's (a space, docs/adr/0708),
/// the outline it takes out of the slab with its arcs.
#[derive(Clone, Debug, PartialEq)]
pub struct SlabCut {
    pub owner: String,
    pub source: String,
    pub ring: Vec<[f64; 2]>,
    pub bulges: Vec<f64>,
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
    /// A stair's or a ramp's railing (docs/adr/8128), its side's (0 left, 1 right as one walks up): not its body but
    /// an `IfcRailing` of its parts.
    Rail(usize),
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
/// (m²; PAİY 22/1-l) and of its other limited uses past their limits (docs/adr/8079): storage annexes past their
/// units' 50 % (22/1-k), caretaker flats past 70 m² together (22/1-ç), library rooms past 50 m² (22/1-f); and, within
/// its rings, a traditional cut stone masonry building's stone walls past 25 cm and 12.50 cm, out apart from the 30 %
/// (5/8, docs/adr/8095).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct EmsalArea {
    pub level: Option<String>,
    pub count: kentos_contracts::EmsalCount,
    pub rings: Vec<Vec<[f64; 2]>>,
    pub moved: f64,
    pub storage: f64,
    pub caretaker: f64,
    pub library: f64,
    pub walls: f64,
    /// Its rooms' own emsal states' bases, each once (docs/adr/8125): an emsal increase's description.
    pub bases: Vec<String>,
    /// The emsal table's area of its storey's rooms counting so, their contours' (docs/adr/8125; EK-9/10); none: not
    /// known.
    pub table: Option<f64>,
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

/// The limited uses' parts an emsal zone names (docs/adr/8079): each clause's words up to its part's area, and how a zone
/// counted in ends them (Turkish vowel harmony: “balkon da”, “eklentisi de”).
fn limited_parts(a: &EmsalArea) -> [(f64, &'static str, &'static str); 4] {
    [
        (a.moved, "22/1-l: bağımsız bölüm net alanının %20'sini aşan {} m² balkon", " da dahil"),
        (a.storage, "22/1-k: bağımsız bölüm net alanının %50'sini aşan {} m² depo eklentisi", " de dahil"),
        (a.caretaker, "22/1-ç: kapıcı dairelerinin toplam 70 m²'sini aşan {} m²", " de dahil"),
        (a.library, "22/1-f: kütüphanenin 50 m²'sini aşan {} m²", " de dahil"),
    ]
}

/// An emsal zone's state as EK-6 Tablo 6.60 writes it and the clauses it rests on. A limited use's part past its limit
/// is named in its storey's zone counted in (the room stays in its own zone, docs/adr/8069 K41), or, on a storey with
/// none, in the zone the room is in, as counted in.
fn emsal_state(a: &EmsalArea) -> (&'static str, &'static str, String) {
    use kentos_contracts::EmsalCount as C;
    let with_parts = |mut words: String, counted_in: bool| {
        for (part, clause, tail) in limited_parts(a) {
            if part > 0.0 {
                words.push_str("; ");
                words.push_str(&clause.replace("{}", &format!("{part:.2}")));
                words.push_str(if counted_in { tail } else { " emsale dahil" });
            }
        }
        // The stone walls' part within the zone's rings is out apart from the 30 % (docs/adr/8095).
        if a.walls > 0.0 {
            words.push_str(&format!(
                "; 5/8: {:.2} m² taşıyıcı yığma taş duvarların kalın kısmı emsal harici, %30 dışında",
                a.walls
            ));
        }
        words
    };
    match a.count {
        C::Included => (
            "DAHIL",
            "emsale dahil alan",
            with_parts("Planlı Alanlar İmar Yönetmeliği 4/1-şş, 21: katlar alanına dahil".to_owned(), true),
        ),
        C::Exempt => (
            "HARIC",
            "emsal harici alan",
            with_parts(
                "Planlı Alanlar İmar Yönetmeliği 22/1: katlar alanına dahil edilmeyen; 5/8: emsale esas alanın %30'u içinde"
                    .to_owned(),
                false,
            ),
        ),
        C::Outside => (
            "HARIC",
            "emsal harici alan (%30 dışında)",
            "Planlı Alanlar İmar Yönetmeliği 5/8: yangın merdiveni, korunumlu koridor, yangın güvenlik holü; %30 sınırının dışında".to_owned(),
        ),
        // EK-6 Tablo 6.60: "DIGER olması durumunda açıklama Description bölümüne girilir" (docs/adr/8125).
        C::Bonus => ("DIGER", "emsal artışı alanı", format!("Emsal artışı: {}", a.bases.join("; "))),
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
    /// Mimar (adı soyadı), soyadı apart (docs/adr/8124), unvanı, Mimarlar Odası sicil no.
    pub architect: Option<String>,
    pub architect_family: Option<String>,
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
    /// The project's description (EK-7 Tablo 7.1 `IfcProject.Description`): the künye's words, none when not given.
    pub description: Option<String>,
    /// The building's mail drop (EK-6 Tablo 6.7 `Pset_Address.PostalBox`): the künye's words, none when not given.
    pub postal_box: Option<String>,
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
            postal_box: self.postal_box.clone(),
            town: self.province.clone(),
            region: self.district.clone(),
            postal_code: None,
            country: Some("Türkiye".into()),
        })
    }

    /// The architect (EK-7 Tablo 7.2–7.5): their family name as the künye gives it apart, their given name their name's
    /// other words (docs/adr/8124: [`Particulars::names`]); their registration number their identification; their
    /// office, when it has a name.
    pub fn architect(&self) -> Option<Actor> {
        let whole = self.architect.as_deref()?;
        if whole.split_whitespace().next().is_none() {
            return None;
        }
        let (given, family) = self.names().unwrap_or((None, None));
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

    /// The architect's given and family names (docs/adr/8124; EK-7 Tablo 7.2, whose note says a family name may stand
    /// first or last): the family name the künye's own field, found at the start or the end of the whole name (case
    /// aside); the given name the words left. None without a family name or when the name does not hold it at either
    /// end: no guess is written.
    pub fn names(&self) -> Option<(Option<String>, Option<String>)> {
        let whole: Vec<&str> = self.architect.as_deref()?.split_whitespace().collect();
        let family: Vec<&str> = self.architect_family.as_deref()?.split_whitespace().collect();
        if family.is_empty() || family.len() > whole.len() {
            return None;
        }
        let same = |a: &[&str], b: &[&str]| a.iter().zip(b).all(|(x, y)| x.to_lowercase() == y.to_lowercase());
        let n = family.len();
        let given = if same(&whole[whole.len() - n..], &family) {
            &whole[..whole.len() - n]
        } else if same(&whole[..n], &family) {
            &whole[n..]
        } else {
            return None;
        };
        let given = (!given.is_empty()).then(|| given.join(" "));
        Some((given, Some(family.join(" "))))
    }

    /// The project's type (EK-7 Tablo 7.1 `Pset_ProjectCommon.ProjectType`, IFC 4.3's `PEnum_ProjectType`) from the
    /// construction action (docs/adr/8074 §3): a new building `NEWBUILD`, an addition or an alteration of a standing one
    /// `MODIFICATION` ("the modification of a facility"); none for another action or none.
    pub fn project_type(&self) -> Option<&'static str> {
        match self.construction.as_deref()? {
            "Yeni yapı" => Some("NEWBUILD"),
            "İlave" | "Tadilat" => Some("MODIFICATION"),
            _ => None,
        }
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
    if let Some(b) = &a.postal_box {
        props.push(("PostalBox".to_owned(), label(b)));
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

/// A balustered railing's balusters as bodies (docs/adr/8128): each upright line of its lines' solid (one without a
/// footprint: the section draws them as lines, ADR 8045 §3) a square bar [`kentos_contracts::BALUSTER_WIDTH`] wide
/// round it, from its foot to its top, over its storey's floor.
fn balusters(solid: &Solid, floor: f64) -> Vec<Shape> {
    if !solid.rings.is_empty() {
        return Vec::new();
    }
    let w = kentos_contracts::BALUSTER_WIDTH / 2.0;
    solid
        .marks
        .iter()
        .filter(|[(a, z0), (b, z1)]| a == b && z1 > z0)
        .map(|[(q, z0), (_, z1)]| Shape::Prism {
            rings: vec![vec![[q.x - w, q.y - w], [q.x + w, q.y - w], [q.x + w, q.y + w], [q.x - w, q.y + w]]],
            bulges: Vec::new(),
            bottom: z0 - floor,
            top: z1 - floor,
        })
        .collect()
}

/// The level an architectural object stands on: its own `arch.level`; none for one without, or another kind.
pub fn level_of(e: &Entity) -> Option<&str> {
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

/// An element's own number as the drawing gives it (docs/adr/8124; EK-6 `Tag`, "a serial or position number"): a
/// column's or a beam's tag (S1, K101); none for one without, whose Tag is then its id.
fn tag_of(e: &Entity) -> Option<String> {
    let own = match e {
        Entity::Column(c) => c.column.tag.as_deref(),
        Entity::Beam(b) => b.beam.tag.as_deref(),
        _ => None,
    };
    own.map(str::trim).filter(|t| !t.is_empty()).map(str::to_owned)
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
        slab_cuts,
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
        rooms,
        building: scope,
        north,
    } = input;
    let (crs, crs_why) = match crs {
        Ok(c) => (Some(c), None),
        Err(why) => (None, Some(why)),
    };
    // Its ±0.00 over the project's (docs/adr/8072 §2): the building stands there, its storeys' elevations over it; the
    // objects keep their places over their storeys (no second offset).
    let zero = scope.as_ref().map_or(0.0, |b| b.zero);
    // The building a derived storey's or zone's key is within (docs/adr/8117): the program's, none for the whole drawing.
    let storey_scope = scope.as_ref().map_or_else(String::new, |b| b.id.clone());
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
    // Its heights are its rooms' (docs/adr/8076 §3), written after them; a level's floor-to-floor height is not one.
    let mut storeys: Vec<Storey> = levels
        .iter()
        .map(|(level, n, floor, _)| Storey {
            // Derived from its building and its level (docs/adr/8117): the same storey keeps its GlobalId whatever else
            // changes; a level two buildings span is a storey of each.
            id: Some(crate::step::derived_id("storey", &format!("{storey_scope}|{level}"))),
            name: name("BNK", n),
            long_name: Some(n.clone()),
            elevation: *floor - zero,
            psets: Vec::new(),
            qtos: Vec::new(),
            unentered: Vec::new(),
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
    let mut rails: Vec<(String, usize, Vec<usize>)> = Vec::new();
    for (i, owner) in owners.iter().enumerate() {
        let Some(o) = owner else { continue };
        match parts.get(i).copied().flatten() {
            // A stair's or a ramp's railing is not its body but a part of it, one each side (docs/adr/8128).
            Some(Part::Rail(side)) => {
                match rails.iter_mut().find(|(r, k, _)| r == o && *k == side) {
                    Some((_, _, list)) => list.push(i),
                    None => rails.push((o.clone(), side, vec![i])),
                }
                continue;
            }
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
        // A balustered railing's balusters are bodies too (docs/adr/8128).
        let barred =
            matches!(e, Entity::Railing(r) if r.railing.infill == kentos_contracts::RailingInfill::Balusters);
        for &i in indices {
            shapes_of.extend(shapes(&solids[i], exact.get(i), floor));
            if barred {
                shapes_of.extend(balusters(&solids[i], floor));
            }
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
            Entity::Slab(s) if s.slab.covering.is_none() => {
                // Its declared exterior role over its role's reading (docs/adr/8124: a floor over open air).
                let outside = s.slab.arch.external.as_ref().map_or(s.slab.role != SlabRole::Floor, |x| x.external);
                let mut props = vec![("IsExternal".into(), Value::Bool(outside))];
                // A sloped slab's pitch (EK-6 Tablo 6.123's PitchAngle; docs/adr/8071 §6), radians.
                if let Some(sl) = s.slab.slope {
                    props.push(("PitchAngle".into(), Value::PlaneAngle(sl.grade().abs().atan())));
                }
                vec![Pset { name: "Pset_SlabCommon".into(), props }]
            }
            // A post carries nothing of the frame's that the drawing knows: no LoadBearing (docs/adr/8070 §8). A column
            // declared not to carry says so (docs/adr/8124).
            Entity::Column(c) if member.is_none() => vec![Pset {
                name: "Pset_ColumnCommon".into(),
                props: vec![(
                    "LoadBearing".into(),
                    Value::Bool(c.column.arch.load_bearing.as_ref().is_none_or(|b| b.bearing)),
                )],
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
            tag: Some(tag_of(e).unwrap_or_else(|| owner.clone())),
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
    // A stair's and a ramp's railings (docs/adr/8128; IFC 4.3 IfcStair, IfcRamp: their railings may be in their
    // aggregation): each side's an IfcRailing BALUSTRADE as a drawn balustered railing is, its body its top rail's
    // pieces and its balusters, a part of its stair or its ramp.
    for (owner, side, pieces) in &rails {
        let (Some(&whole), Some(&(id, _))) = (index.get(owner), by_uid.get(owner)) else {
            continue;
        };
        let (storey, floor) = (elements[whole].storey, levels[elements[whole].storey].2);
        let body: Vec<Shape> = pieces
            .iter()
            .flat_map(|&i| shapes(&solids[i], exact.get(i), floor).into_iter().chain(balusters(&solids[i], floor)))
            .collect();
        if body.is_empty() {
            continue;
        }
        elements.push(Element {
            class: Class::Railing,
            id: Some(crate::step::fnv(owner, 2_000_000 + *side as u64)),
            object: Some(id),
            name: name(Class::Railing.code(), "Korkuluk"),
            description: None,
            tag: Some(owner.clone()),
            predefined: "BALUSTRADE",
            object_type: None,
            part_of: Some(whole),
            storey,
            shapes: body,
            overall: None,
            material: None,
            psets: Vec::new(),
            qtos: Vec::new(),
        });
        written.railings += 1;
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
    // A wall's declared exterior role over the drawing's reading (docs/adr/8124; a curtain wall's too): its doors,
    // windows and frames follow it.
    for (owner, &k) in &index {
        if let Some(&(_, e @ Entity::Wall(_))) = by_uid.get(owner)
            && let Some(x) = e.arch_info().and_then(|a| a.external.as_ref())
            && matches!(elements[k].class, Class::Wall | Class::CurtainWall)
        {
            exterior[k] = x.external;
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
        // Its Tag (docs/adr/8124): its number, else its type's joinery code, else its id.
        let code = o
            .opening
            .style
            .as_deref()
            .and_then(|id| arch.and_then(|a| a.opening_types.iter().find(|t| t.id == id)))
            .and_then(|t| t.code.clone());
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
                    tag: Some(mark.clone().or_else(|| code.clone()).unwrap_or_else(|| owner.clone())),
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
            // Derived from the door or window that makes it (docs/adr/8117).
            id: Some(crate::step::derived_id("opening", owner)),
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
    // A slab's holes and the voids over it (docs/adr/8071 §5): its body is its own outline, each hole drawn in it and
    // each void's part over it an opening of its own (Design Transfer: the reader takes it out; a Reference View: cut).
    for (owner, &k) in &index {
        let Some(&(_, Entity::Slab(sl))) = by_uid.get(owner) else {
            continue;
        };
        if elements[k].class != Class::Slab {
            continue;
        }
        let ring_of = |r: &kentos_contracts::SpaceRing| -> (Vec<[f64; 2]>, Vec<f64>) {
            let n = r.pts.len();
            (
                r.pts.iter().map(|q| [q.x, q.y]).collect(),
                (0..n).map(|i| r.bulges.as_ref().and_then(|b| b.get(i)).copied().unwrap_or(0.0)).collect(),
            )
        };
        let (outer, outer_bulges): (Vec<Vec<[f64; 2]>>, Vec<Vec<f64>>) =
            sl.slab.parts.iter().map(|part| ring_of(&part.outer)).unzip();
        let (mut lo, mut hi) = (f64::MAX, f64::MIN);
        for sh in &mut elements[k].shapes {
            if let Shape::Prism { rings, bulges, bottom, top } = sh {
                *rings = outer.clone();
                *bulges = outer_bulges.clone();
                lo = lo.min(*bottom);
                hi = hi.max(*top);
            }
        }
        if lo >= hi {
            continue;
        }
        let mut n = 0u64;
        for part in &sl.slab.parts {
            for h in &part.holes {
                n += 1;
                let (ring, bulges) = ring_of(h);
                openings.push(Opening {
                    id: Some(crate::step::fnv(owner, 3000 + n)),
                    name: name("ACL", "Döşeme boşluğu"),
                    host: k,
                    ring,
                    bulges,
                    bottom: lo,
                    top: hi,
                    filler: None,
                    predefined: "OPENING",
                });
                written.openings += 1;
            }
        }
        for c in slab_cuts.iter().filter(|c| &c.owner == owner) {
            // Named for its void: its number and its name, as the room list writes them.
            let words = match by_uid.get(&c.source) {
                Some(&(_, Entity::Space(sp))) => [sp.space.number.as_deref(), sp.space.name.as_deref()]
                    .into_iter()
                    .flatten()
                    .collect::<Vec<_>>()
                    .join(" "),
                _ => String::new(),
            };
            openings.push(Opening {
                id: Some(crate::step::fnv(&format!("{}/{}", c.source, owner), 1)),
                name: name("ACL", if words.trim().is_empty() { "Boşluk" } else { &words }),
                host: k,
                ring: c.ring.clone(),
                bulges: c.bulges.clone(),
                bottom: lo,
                top: hi,
                filler: None,
                predefined: "OPENING",
            });
            written.openings += 1;
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
            (Class::Slab, Entity::Slab(sl)) if sl.slab.slope.is_some() => crate::quantities::sloped_slab(&sl.slab),
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
    // Each storey's roof faces its rooms rise to (docs/adr/8071 §11), about the storey: the emsal zones of that storey
    // follow them too (docs/adr/8123).
    let mut storey_roofs: BTreeMap<usize, Vec<crate::under_roof::UnderFace>> = BTreeMap::new();
    let mut net_floor: Vec<Option<f64>> = vec![None; storeys.len()];
    let mut votes: Vec<Vec<Vote>> = (0..storeys.len()).map(|_| Vec::new()).collect();
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
        // From its finished floor to its ceiling (docs/adr/8071 §7): a volume closed by what bounds it.
        let found = rooms.get(uid.as_str()).filter(|r| r.ceiling > r.floor);
        let (bottom, top) = match found {
            Some(r) => (r.floor - levels[storey].2, r.ceiling - levels[storey].2),
            None => {
                let bottom = f.floor_offset.unwrap_or(0.0);
                (bottom, bottom + f.height.or(levels[storey].3).unwrap_or(2.8))
            }
        };
        // Under a roof (docs/adr/8071 §11): its ceiling known no other way, its pieces up to the roof's underside.
        let elevation = levels[storey].2;
        let under = found.filter(|r| !r.known && !r.roof.is_empty()).and_then(|r| {
            let faces: Vec<crate::under_roof::UnderFace> =
                r.roof.iter().map(|(ring, (c, dx, dy))| (ring.clone(), (c - elevation, *dx, *dy))).collect();
            let p = crate::under_roof::pieces(&rings, &bulges, &faces);
            if p.is_some() {
                storey_roofs.entry(storey).or_insert(faces);
            }
            p
        });
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
        // under the slab over, as the vertical model holds them; the finish of the walls' faces it lies along
        // (docs/adr/8076 §7).
        let (floor_t, ceiling_t, wall_t) = coverings.get(uid).copied().unwrap_or((None, None, None));
        if let Some(t) = floor_t {
            props.push(("FloorCoveringThickness".to_owned(), Value::PositiveLength(t)));
        }
        if let Some(t) = wall_t {
            props.push(("WallCoveringThickness".to_owned(), Value::PositiveLength(t)));
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
        // Its quantities from its contour with its arcs and its floor and ceiling (docs/adr/8071 §7; EK-6 Tablo 6.125's
        // optional rows): its height where its ceiling is known, its asma tavan's height where it has one, its gross
        // area and perimeter (its outer rings), its volumes by its height.
        let mut q: Vec<(String, Value)> = Vec::new();
        if let Some(a) = net {
            q.push(("NetFloorArea".into(), Value::Area(a)));
        }
        let height = top - bottom;
        if let Some(r) = found.filter(|r| r.known) {
            q.push(("Height".into(), Value::Length(height)));
            if r.suspended {
                q.push(("FinishCeilingHeight".into(), Value::Length(height)));
            }
        }
        let gross = crate::quantities::area(&rings, &bulges);
        // Its say in its storey's structural slab level and heights, by its area (docs/adr/8076 §3).
        let structure = rooms.get(uid.as_str());
        votes[storey].push(Vote {
            weight: gross,
            ssl: structure.and_then(|r| r.structure),
            gross: structure.and_then(|r| Some(r.over?.0 - r.structure?)),
            net: structure.and_then(|r| Some(r.over?.1 - r.structure?)),
        });
        let perimeter: f64 = crate::quantities::areas(&rings, &bulges)
            .iter()
            .map(|a| {
                let pts: Vec<[f64; 2]> = a.outer.pts.iter().map(|p| [p.x, p.y]).collect();
                crate::quantities::path_length(&pts, &a.outer.bulges.clone().unwrap_or_default(), true)
            })
            .sum();
        q.push(("GrossFloorArea".into(), Value::Area(gross)));
        q.push(("GrossPerimeter".into(), Value::Length(perimeter)));
        if found.is_some_and(|r| r.known) {
            q.push(("GrossVolume".into(), Value::Volume(gross * height)));
            if let Some(a) = net {
                q.push(("NetVolume".into(), Value::Volume(a * height)));
            }
        } else if let Some(p) = &under {
            // Its height not constant: no Height (IFC 4.3: "only if the space has a constant height"); its volume up to
            // the roof, net the same when nothing is taken out of its area (what is, stands at heights not known).
            let v = crate::under_roof::volume(p, bottom);
            q.push(("GrossVolume".into(), Value::Volume(v)));
            if net.is_some_and(|a| (a - gross).abs() <= 1e-9 * gross.max(1.0)) {
                q.push(("NetVolume".into(), Value::Volume(v)));
            }
        }
        let qtos = vec![Qto { name: "Qto_SpaceBaseQuantities".into(), quantities: q }];
        // Its kind (docs/adr/8125; EK-6 Tablo 6.58): a car park PARKING, an outdoor room (its brief's: a balcony, a
        // terrace, an open area) EXTERNAL, else INTERNAL. EK-6 asks the type; IFC 4.3's optional
        // `Pset_SpaceCommon.IsExternal` is not written (K286).
        let outdoor = f
            .usage
            .as_deref()
            .and_then(|u| arch.and_then(|a| a.program.brief(u)))
            .is_some_and(|b| b.outdoor);
        let predefined = if kentos_contracts::is_parking(f.name.as_deref(), f.usage.as_deref()) {
            "PARKING"
        } else if outdoor {
            "EXTERNAL"
        } else {
            "INTERNAL"
        };
        let (top, body) = match &under {
            Some(p) => (crate::under_roof::top(p), crate::under_roof::shapes(p, bottom)),
            None => (top, Vec::new()),
        };
        spaces.push(Space {
            id: Some(*bits),
            name: name("MHL", if words.is_empty() { "Mahal" } else { &words }),
            long_name: f.name.clone(),
            storey,
            rings,
            bulges,
            bottom,
            top,
            predefined,
            psets,
            qtos,
            body,
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
    // Each storey's structural slab level (EK-6 Tablo 6.8 `ElevationOfSSLRelative`, over the building's ±0.00) and its
    // heights (IFC 4.3 `GrossHeight`: from the construction floor's top to the top of the one over it; `NetHeight`: to
    // its underside): what more than half its rooms' area stands on and under (docs/adr/8076 §3). None where the level
    // varies with none prominent (Tablo 6.8: "bu özellik girilmeyebilir").
    let mut storey_heights: Vec<(Option<f64>, Option<f64>)> = Vec::new();
    for (storey_at, (storey, v)) in storeys.iter_mut().zip(&votes).enumerate() {
        let total: f64 = v.iter().map(|x| x.weight).sum();
        let ssl = prominent(v.iter().map(|x| (x.ssl, x.weight)), total);
        let gross = prominent(v.iter().map(|x| (x.gross, x.weight)), total);
        let net = prominent(v.iter().map(|x| (x.net, x.weight)), total);
        // What Tablo 6.8 lets go unentered (docs/adr/8124): a level that varies (two known ones, none prominent); the
        // heights of a storey whose rooms rise to a roof's underside, which are not constant.
        let mut known: Vec<f64> = v.iter().filter_map(|x| x.ssl).collect();
        known.sort_by(f64::total_cmp);
        known.dedup_by(|a, b| (*a - *b).abs() < 1e-9);
        if ssl.is_none() && known.len() > 1 {
            storey.unentered.push(("ElevationOfSSLRelative", "taşıyıcı döşemenin kotu değişiyor, baskın kot yok"));
        }
        if storey_roofs.contains_key(&storey_at) {
            if gross.is_none() {
                storey.unentered.push(("GrossHeight", "mahalleri çatının altına yükseliyor: yükseklik sabit değil"));
            }
            if net.is_none() {
                storey.unentered.push(("NetHeight", "mahalleri çatının altına yükseliyor: yükseklik sabit değil"));
            }
        }
        if let Some(l) = ssl {
            storey.psets.push(Pset {
                name: "Pset_BuildingStoreyCommon".into(),
                props: vec![("ElevationOfSSLRelative".into(), Value::Length(l - zero))],
            });
        }
        let mut q = Vec::new();
        if let Some(h) = gross {
            q.push(("GrossHeight".to_owned(), Value::Length(h)));
        }
        if let Some(h) = net {
            q.push(("NetHeight".to_owned(), Value::Length(h)));
        }
        if !q.is_empty() {
            storey.qtos.push(Qto {
                name: "Qto_BuildingStoreyBaseQuantities".into(),
                quantities: q,
            });
        }
        storey_heights.push((gross, net));
    }
    // A storey's and the building's net floor area (EK-6 Tablo 6.8, 6.7): its rooms' net areas summed; the building's
    // heights its storeys', only where they are the same for all (Tablo 6.7: "sadece yüksekliğin sabit olduğu durumda").
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
    // `Height` the storeys' gross height, `EavesHeight` their net height (Tablo 6.7's text and IFC 4.3's: "bu katın
    // standart net yüksekliği"; docs/adr/8076 §4), each where every storey has the same.
    let constant = |h: Vec<Option<f64>>| -> Option<f64> {
        let first = (*h.first()?)?;
        h.iter().all(|x| x.is_some_and(|x| (x - first).abs() <= 1e-9)).then_some(first)
    };
    if let Some(h) = constant(storey_heights.iter().map(|s| s.0).collect()) {
        building_quantities.push(("Height".to_owned(), Value::Length(h)));
    }
    if let Some(h) = constant(storey_heights.iter().map(|s| s.1).collect()) {
        building_quantities.push(("EavesHeight".to_owned(), Value::Length(h)));
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
        // Both above 0 (IFC 4.3's template: `IfcPositiveRatioMeasure`; docs/adr/8075).
        if let Some(far) = z.far.filter(|v| *v > 0.0) {
            props.push(("FloorAreaRatio".to_owned(), Value::PositiveRatio(far)));
        }
        if let Some(c) = z.coverage.filter(|v| *v > 0.0) {
            props.push(("SiteCoverageRatio".to_owned(), Value::PositiveRatio(c)));
        }
        // The site's planned area (EK-6 Tablo 6.55 `TotalArea`, "arsaya ilişkin toplam planlanan alan"; docs/adr/8076
        // §2): the parcel's, as İmar durumu gives it.
        if z.parcel_area > 0.0 {
            props.push(("TotalArea".to_owned(), Value::Area(z.parcel_area)));
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
    // Both identifiers (IFC 4.3's template: `IfcIdentifier`; docs/adr/8074 §5, found by the profile's IDS).
    let mut land = Vec::new();
    if let Some(id) = k.land_id() {
        land.push(("LandID".to_owned(), Value::Id(id)));
    }
    if let Some(t) = &k.title_deed {
        land.push(("LandTitleID".to_owned(), Value::Id(t.clone())));
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
        let building_code = scope.as_ref().and_then(|b| b.code.as_deref()).or(k.building_code.as_deref());
        let id = format!("{code}_{}", building_code.unwrap_or("00"));
        common.push(("BuildingID".to_owned(), Value::Id(id)));
        common.push(("IsPermanentID".to_owned(), Value::Bool(false)));
    }
    // A building of several: its own use and its own areas (docs/adr/8072 §4), never the künye's, which are the
    // project's.
    let (gross_area, footprint) = match scope.as_ref().filter(|b| b.several) {
        Some(b) => (b.gross_area, b.footprint),
        None => (k.gross_area, k.footprint),
    };
    if let Some(u) = scope.as_ref().and_then(|b| b.usage.as_ref()).or(k.occupancy.as_ref()) {
        common.push(("OccupancyType".to_owned(), label(u)));
    }
    if let Some(a) = gross_area {
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
    if let Some(a) = footprint {
        building_quantities.push(("FootPrintArea".to_owned(), Value::Area(a)));
    }
    if let Some(a) = gross_area {
        building_quantities.push(("GrossFloorArea".to_owned(), Value::Area(a)));
    }
    let title = title.to_owned();
    let work = k.work.clone().unwrap_or_else(|| title.clone());
    // The project's own seed, whichever building a file holds: its code, else its title.
    let project_seed = k.project_code.clone().unwrap_or_else(|| title.clone());
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
            // A wall's declared load-bearing role (docs/adr/8090; EK-6 Tablo 6.133 `Pset_WallCommon.LoadBearing`).
            if let Entity::Wall(w) = e
                && class == Class::Wall
                && let Some(b) = &w.wall.arch.load_bearing
            {
                give(&mut elements[i].psets, "Pset_WallCommon", "LoadBearing", Value::Bool(b.bearing));
            }
        }
        for e in &mut elements {
            if let Some(m) = e.material.take() {
                e.material = Some(material_named(m, arch));
            }
        }
    }
    // The emsal zones (docs/adr/8069): each storey's rooms by how they count, between its floor and its rooms' tops; on a
    // storey whose rooms rise to a roof's underside, cut by the same faces (docs/adr/8123): its tallest room's top would
    // stand out of the roof's slopes as a box.
    let mut zone_keys: Vec<String> = Vec::new();
    let zones: Vec<Zone> = emsal
        .iter()
        .filter(|a| a.rings.iter().any(|r| r.len() >= 3))
        .map(|a| {
            // Derived from the level and how its rooms count (docs/adr/8117).
            let mut key = format!("{storey_scope}|{}|{:?}", a.level.as_deref().unwrap_or(""), a.count);
            while zone_keys.contains(&key) {
                key.push('+');
            }
            zone_keys.push(key.clone());
            let storey = storey_of(a.level.as_deref());
            let (state, words, description) = emsal_state(a);
            let storey_name = storeys.get(storey).map_or_else(String::new, |s| s.long_name.clone().unwrap_or_else(|| s.name.clone()));
            let top = spaces
                .iter()
                .filter(|s| s.storey == storey)
                .map(|s| s.top)
                .fold(levels.get(storey).and_then(|l| l.3).unwrap_or(0.0), f64::max);
            let under = storey_roofs.get(&storey).and_then(|faces| crate::under_roof::pieces(&a.rings, &[], faces));
            let (top, body) = match &under {
                Some(p) => (crate::under_roof::top(p), crate::under_roof::shapes(p, 0.0)),
                None => (top, Vec::new()),
            };
            Zone {
                id: Some(crate::step::derived_id("emsal", &key)),
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
                body,
                table: a.table,
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
    let mut grids = grids_of(&entities, &storey_of, &mut written);
    // In one building's file its axes are its own (docs/adr/8072 §2): the project's axes on no storey stand with each
    // building, at its ±0.00, so each copy takes its building's GlobalId.
    if let Some(b) = &scope {
        for g in &mut grids {
            g.id = g.id.map(|x| crate::step::fnv(&format!("kentos-aks:{}:{x}", b.id), 0));
        }
    }
    // A layered roof's material layer set (docs/adr/8071 §8): top down, each layer its kind's word, its material (the
    // covering's, not given, the roof's covering; another's not given unsaid), its thickness across the roof's face and
    // its role in IFC 4.3's words (`IfcMaterialLayer.Category`): the covering the outer finish, the insulation
    // insulating, the rafters load bearing.
    let layer_sets: Vec<(usize, String, Vec<crate::building::Layer>)> = index
        .iter()
        .filter_map(|(owner, &k)| match by_uid.get(owner) {
            Some(&(_, Entity::Roof(r)))
                if !r.roof.layers.is_empty() && elements.get(k).is_some_and(|e| e.class == Class::Roof) =>
            {
                use kentos_contracts::RoofLayerKind as K;
                let layers = r
                    .roof
                    .layers
                    .iter()
                    .map(|l| crate::building::Layer {
                        name: l.kind.label().to_owned(),
                        material: l.material.clone().or_else(|| {
                            (l.kind == K::Covering).then(|| r.roof.material_name().to_owned())
                        }),
                        thickness: l.thickness,
                        category: match l.kind {
                            K::Covering => Some("Outer finish"),
                            K::Insulation => Some("Insulation"),
                            K::Rafters => Some("LoadBearing"),
                            K::Battens | K::Waterproofing | K::Boarding => None,
                        },
                    })
                    .collect();
                Some((k, "Çatı katmanları".to_owned(), layers))
            }
            _ => None,
        })
        .collect();
    // The walls' layers and their styles' types (docs/adr/8105, IFC-04): each wall its own components as a material
    // layer set, from its left face towards its right, each layer its name, its material (as the model names materials:
    // Ad_Dayanım) and its thickness at the wall's width; each wall style its `IfcWallType`, its set the style's
    // components at the style's own width, the style's walls typed by it. A wall whose set is the style's shares it.
    let mut layer_sets = layer_sets;
    let mut types: Vec<crate::building::TypeObject> = Vec::new();
    // The style each type is of, beside it.
    let mut type_styles: Vec<String> = Vec::new();
    let layers_of = |components: &[WallComponent], width: f64| -> Option<Vec<crate::building::Layer>> {
        let thick = kentos_contracts::component_thicknesses(width, components)?;
        let layers: Vec<crate::building::Layer> = components
            .iter()
            .zip(thick)
            .map(|(c, t)| crate::building::Layer {
                name: c.name.clone(),
                material: c.material.clone().map(|m| material_named(m, arch)),
                thickness: t,
                category: None,
            })
            .collect();
        (!layers.is_empty()).then_some(layers)
    };
    for (owner, &k) in &index {
        let Some(&(_, Entity::Wall(w))) = by_uid.get(owner) else { continue };
        if elements.get(k).is_none_or(|e| e.class != Class::Wall) || layer_sets.iter().any(|(i, ..)| *i == k) {
            continue;
        }
        let Some(layers) = layers_of(&w.wall.components, w.wall.width) else { continue };
        let style = w.wall.style.as_deref();
        let set_name = Some(wall_style_name(style)).filter(|n| !n.trim().is_empty()).unwrap_or_else(|| "Duvar katmanları".into());
        layer_sets.push((k, set_name, layers));
        let Some(id) = style else { continue };
        match type_styles.iter().position(|s| s == id) {
            Some(at) => types[at].members.push(k),
            None => {
                type_styles.push(id.to_owned());
                let def = wall_styles
                    .iter()
                    .find(|d| d.id == id)
                    .cloned()
                    .or_else(|| kentos_contracts::builtin_wall_styles().into_iter().find(|d| d.id == id));
                let layer_set = def
                    .as_ref()
                    .and_then(|d| layers_of(&d.components, d.new_width()))
                    .map(|l| (wall_style_name(Some(id)), l));
                let seed = match &scope {
                    Some(b) => format!("kentos-duvar-tipi:{}:{id}", b.id),
                    None => format!("kentos-duvar-tipi:{id}"),
                };
                types.push(crate::building::TypeObject {
                    entity: "IFCWALLTYPE",
                    id: crate::step::fnv(&seed, 0),
                    name: wall_style_name(Some(id)),
                    predefined: "STANDARD",
                    layer_set,
                    members: vec![k],
                });
            }
        }
    }
    // The element types (docs/adr/8106, BIM-12): each project type of a column, beam, slab, roof, railing or stair is an
    // IFC type of the class its values make (a post's IfcMemberType, a covering's IfcCoveringType), its elements of
    // that class typed by it — one an exception made another class stays untyped. Its predefined type theirs when they
    // agree (NOTDEFINED else, and for USERDEFINED, whose word the type would need); its layer set theirs when they
    // share one.
    let element_types = arch.map_or(&[][..], |a| a.element_types.as_slice());
    for t in element_types {
        use kentos_contracts::ElementTypeKind as K;
        let class = match t.kind {
            K::Column if t.values.contains_key("member") => Class::Member,
            K::Column => Class::Column,
            K::Beam if t.values.contains_key("member") => Class::Member,
            K::Beam => Class::Beam,
            K::Slab if t.values.contains_key("covering") => Class::Covering,
            K::Slab => Class::Slab,
            K::Roof => Class::Roof,
            K::Railing => Class::Railing,
            K::Stair => Class::Stair,
        };
        let entity = match class {
            Class::Column => "IFCCOLUMNTYPE",
            Class::Member => "IFCMEMBERTYPE",
            Class::Beam => "IFCBEAMTYPE",
            Class::Covering => "IFCCOVERINGTYPE",
            Class::Slab => "IFCSLABTYPE",
            Class::Roof => "IFCROOFTYPE",
            Class::Railing => "IFCRAILINGTYPE",
            _ => "IFCSTAIRTYPE",
        };
        let mut members: Vec<usize> = index
            .iter()
            .filter(|&(owner, &k)| {
                by_uid.get(owner).is_some_and(|&(_, e)| {
                    e.arch_info().and_then(|a| a.element_type.as_ref()).is_some_and(|l| l.id == t.id)
                }) && elements.get(k).is_some_and(|e| e.class == class)
            })
            .map(|(_, &k)| k)
            .collect();
        if members.is_empty() {
            continue;
        }
        members.sort_unstable();
        members.dedup();
        let first = elements[members[0]].predefined;
        let predefined = if first != "USERDEFINED" && members.iter().all(|&k| elements[k].predefined == first) {
            first
        } else {
            "NOTDEFINED"
        };
        let set_of = |k: usize| layer_sets.iter().find(|(i, ..)| *i == k).map(|(_, n, l)| (n.clone(), l.clone()));
        let layer_set = set_of(members[0]).filter(|s| members.iter().all(|&k| set_of(k).as_ref() == Some(s)));
        let seed = match &scope {
            Some(b) => format!("kentos-eleman-tipi:{}:{}", b.id, t.id),
            None => format!("kentos-eleman-tipi:{}", t.id),
        };
        types.push(crate::building::TypeObject {
            entity,
            id: crate::step::fnv(&seed, 0),
            name: t.name.clone(),
            predefined,
            layer_set,
            members,
        });
    }
    for t in &mut types {
        t.members.sort_unstable();
    }
    // Each room's coverings (docs/adr/8127; IFC 4.3 `IfcRelCoversSpaces`): a covering bound to a room (its outline
    // the room's, docs/adr/0771 §4) is that room's; one drawn by hand is the room on its storey its inside point stands
    // in; one in no room covers none.
    let covers: Vec<(usize, Vec<usize>)> = {
        use kentos_geometry_core::geom::arrangement::{Area, Ring};
        let area_of = |rings: &[Vec<[f64; 2]>], bulges: &[Vec<f64>]| -> Vec<Area> {
            let ring = |i: usize| Ring {
                pts: rings[i].iter().map(|p| kentos_geometry_core::Vec2 { x: p[0], y: p[1] }).collect(),
                bulges: bulges.get(i).filter(|b| b.iter().any(|x| *x != 0.0)).cloned(),
            };
            if rings.is_empty() {
                return Vec::new();
            }
            vec![Area { outer: ring(0), holes: (1..rings.len()).map(ring).collect() }]
        };
        let rooms: Vec<(usize, Vec<Area>)> = spaces.iter().enumerate().map(|(i, sp)| (i, area_of(&sp.rings, &sp.bulges))).collect();
        let mut out: Vec<(usize, Vec<usize>)> = Vec::new();
        for (k, el) in elements.iter().enumerate().filter(|(_, el)| el.class == Class::Covering) {
            let Some(id) = el.id else { continue };
            let bound = by_uid.values().find(|(bits, _)| *bits == id).and_then(|(_, e)| match e {
                Entity::Slab(s) => s
                    .slab
                    .bound
                    .as_ref()
                    .filter(|b| b.source == kentos_contracts::SlabSourceKind::Space)
                    .and_then(|b| b.space.clone()),
                _ => None,
            });
            let room = match bound {
                Some(uid) => kentos_contracts::EntityId::parse(&uid)
                    .map(|u| u128::from_be_bytes(u.0))
                    .and_then(|bits| spaces.iter().position(|sp| sp.id == Some(bits))),
                None => {
                    // Its inside point, from its body's first outline.
                    let at = el.shapes.iter().find_map(|sh| match sh {
                        Shape::Prism { rings, bulges, .. } => {
                            kentos_geometry_core::geom::space::label_at(&area_of(rings, bulges))
                        }
                        _ => None,
                    });
                    at.and_then(|p| {
                        rooms.iter().find(|(i, areas)| {
                            spaces[*i].storey == el.storey
                                && areas.iter().any(|a| kentos_geometry_core::geom::region::inside_area(a, p))
                        })
                    })
                    .map(|(i, _)| *i)
                }
            };
            if let Some(r) = room {
                match out.iter_mut().find(|(s, _)| *s == r) {
                    Some((_, list)) => list.push(k),
                    None => out.push((r, vec![k])),
                }
            }
        }
        out
    };
    let building = Building {
        view: Default::default(),
        // One building's file (docs/adr/8072 §2): its stem its own (EK-2: its code), so its own GlobalIds; the project's
        // and the site's the same in every building's file, from the project's seed.
        file: match &scope {
            Some(b) => {
                let mut own = k.clone();
                own.building_code = b.code.clone().or(own.building_code);
                // Without a project code the stem is the drawing's name: the building's code (else its id) after it.
                match own.project_code {
                    Some(_) => own.file_stem(&title),
                    None => format!("{title}_{}", b.code.clone().unwrap_or_else(|| b.id.clone())),
                }
            }
            None => k.file_stem(&title),
        },
        stamp: stamp.to_owned(),
        application: application.to_owned(),
        phase: None,
        project: Place {
            id: Some(scope.as_ref().map_or_else(
                || crate::step::derived_id("project", &project_seed),
                |_| crate::step::fnv(&format!("kentos-proje:{project_seed}"), 0),
            )),
            name: project_name,
            long_name: Some(work.clone()),
            // EK-7 Tablo 7.1 (docs/adr/8074 §3): the künye's description and the construction action's project type.
            description: k.description.clone(),
            psets: k
                .project_type()
                .map(|t| Pset {
                    name: "Pset_ProjectCommon".into(),
                    props: vec![("ProjectType".into(), Value::Label(t.into()))],
                })
                .into_iter()
                .collect(),
            ..Place::default()
        },
        actors,
        site: Place {
            id: Some(scope.as_ref().map_or_else(
                || crate::step::derived_id("site", &project_seed),
                |_| crate::step::fnv(&format!("kentos-saha:{project_seed}"), 0),
            )),
            name: name("SAH", "Parsel"),
            long_name: k.land_id(),
            // IFC 4.3's IfcSite has no predefined type: its type is its ObjectType (EK-6 Tablo 6.55, case 2). KentOS's
            // site is the project's parcel (docs/adr/8076 §1), its name and sets the parcel's.
            object_type: Some("Parsel".into()),
            psets: site_psets,
            qtos: site_qtos,
            address: address.clone(),
            ..Place::default()
        },
        site_elevation: arch.and_then(|a| a.site.as_ref()).and_then(|s| s.datum),
        building: Place {
            // One of the program's buildings its own GlobalId, the same from file to file (docs/adr/8072 §2); the site and
            // the project the same in every building's file.
            id: Some(scope.as_ref().map_or_else(
                || crate::step::derived_id("building", &project_seed),
                |b| crate::step::fnv(&format!("kentos-bina:{project_seed}:{}", b.id), 0),
            )),
            name: name("BNA", scope.as_ref().map_or(&work, |b| &b.name)),
            long_name: Some(scope.as_ref().map_or(work.clone(), |b| b.name.clone())),
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
        // The project's survey points are the application's to give (docs/adr/8124): the drawing's entities do not hold
        // them.
        surveys: Vec::new(),
        covers,
        crs,
        crs_why,
        // The y axis turned by the angle: a unit vector in the model's plane.
        true_north: north.as_ref().ok().map(|a| [-crate::math::sin(*a), crate::math::cos(*a)]),
        north_why: north.err(),
        layer_sets,
        types,
        zero,
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

    /// The family name the künye gives apart, at the end or the start of the whole name (docs/adr/8124); no guess.
    #[test]
    fn the_architect_s_names_come_from_the_family_name_given_apart() {
        let k = |whole: &str, family: Option<&str>| Particulars {
            architect: Some(whole.to_owned()),
            architect_family: family.map(str::to_owned),
            ..Particulars::default()
        };
        let names = |p: Particulars| p.names().map(|(g, f)| (g.unwrap_or_default(), f.unwrap_or_default()));
        assert_eq!(names(k("Ayşe Nur Yılmaz", Some("Yılmaz"))), Some(("Ayşe Nur".into(), "Yılmaz".into())));
        // Two words of family name, and one written first.
        assert_eq!(names(k("Ali Rıza Kara Demir", Some("Kara Demir"))), Some(("Ali Rıza".into(), "Kara Demir".into())));
        assert_eq!(names(k("Yılmaz Ayşe", Some("yılmaz"))), Some(("Ayşe".into(), "yılmaz".into())));
        // None given, or one the name does not hold at either end: nothing written for either name.
        assert_eq!(names(k("Ayşe Nur Yılmaz", None)), None);
        assert_eq!(names(k("Ayşe Nur Yılmaz", Some("Nur"))), None);
        let none = k("Ayşe Nur Yılmaz", None).architect().expect("an actor");
        assert_eq!((none.given, none.family), (None, None));
        let both = k("Ayşe Nur Yılmaz", Some("Yılmaz")).architect().expect("an actor");
        assert_eq!((both.given.as_deref(), both.family.as_deref()), (Some("Ayşe Nur"), Some("Yılmaz")));
    }
}
