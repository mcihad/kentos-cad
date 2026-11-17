//! A building written as IFC 4.3 (docs/adr/8047; Dijital Proje Yönetmeliği EK-5 §5.4: the classes are IFC 4.3's): an
//! ISO 10303-21 file of the `IFC4X3_ADD2` schema. The spatial structure is a project, its site, its building and the
//! building's storeys (`IfcRelAggregates`); each element stands on its storey (`IfcRelContainedInSpatialStructure`), its
//! body extruded prisms (`IfcExtrudedAreaSolid` of an `IfcArbitraryClosedProfileDef`, or `…WithVoids`) or faces
//! (`IfcPolygonalFaceSet`), placed on its storey. A door or a window fills an opening (`IfcOpeningElement`,
//! `IfcRelFillsElement`) that voids its wall (`IfcRelVoidsElement`). Each object's properties are property sets and its
//! quantities element quantities (`IfcRelDefinesByProperties`), its material an `IfcMaterial`
//! (`IfcRelAssociatesMaterial`). An object's GlobalId is its KentOS id's 128 bits when it has one, so the same object
//! keeps its GlobalId from one export to the next. Lengths in metres, areas in square metres, angles in radians.
//!
//! Nothing here knows KentOS's drawing: the application gives the building as [`Building`].

use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::arc::DEFAULT_STEP;
use kentos_geometry_core::geom::bulge::{bulge_arc, bulge_path_outline, has_bulges, segment_mid};

use crate::step::{Step, guid, real, text};

/// A property's or a quantity's value.
#[derive(Clone, Debug, PartialEq)]
pub enum Value {
    Bool(bool),
    /// A short name (`IfcLabel`).
    Label(String),
    /// A longer text (`IfcText`).
    Text(String),
    /// An identifier (`IfcIdentifier`).
    Id(String),
    Length(f64),
    /// A length above 0 (`IfcPositiveLengthMeasure`: a covering's thickness).
    PositiveLength(f64),
    Area(f64),
    Volume(f64),
    Ratio(f64),
    /// A ratio above 0 (`IfcPositiveRatioMeasure`: the site's floor area ratio and coverage, IFC 4.3's
    /// Pset_SiteCommon).
    PositiveRatio(f64),
    Count(i64),
    Real(f64),
    /// A mass, kilograms (`IfcMassMeasure`: a lift's rated load).
    Mass(f64),
    /// An angle, radians (`IfcPlaneAngleMeasure`: a sloped slab's pitch, docs/adr/8071 §6).
    PlaneAngle(f64),
    /// Several short names (`IfcPropertyListValue` of `IfcLabel`s: Pset_Address's AddressLines).
    Labels(Vec<String>),
}

impl Value {
    /// The IFC type it is written with (`IfcLabel`; a list of labels its items'), as IFC 4.3's templates name a
    /// property's measure.
    pub fn ifc_type(&self) -> &'static str {
        match self {
            Value::Bool(_) => "IfcBoolean",
            Value::Label(_) | Value::Labels(_) => "IfcLabel",
            Value::Text(_) => "IfcText",
            Value::Id(_) => "IfcIdentifier",
            Value::Length(_) => "IfcLengthMeasure",
            Value::PositiveLength(_) => "IfcPositiveLengthMeasure",
            Value::Area(_) => "IfcAreaMeasure",
            Value::Volume(_) => "IfcVolumeMeasure",
            Value::Ratio(_) => "IfcRatioMeasure",
            Value::PositiveRatio(_) => "IfcPositiveRatioMeasure",
            Value::Count(_) => "IfcCountMeasure",
            Value::Real(_) => "IfcReal",
            Value::Mass(_) => "IfcMassMeasure",
            Value::PlaneAngle(_) => "IfcPlaneAngleMeasure",
        }
    }

    fn step(&self) -> String {
        match self {
            Value::Bool(b) => format!("IFCBOOLEAN(.{}.)", if *b { "T" } else { "F" }),
            Value::Label(s) => format!("IFCLABEL({})", text(s)),
            Value::Text(s) => format!("IFCTEXT({})", text(s)),
            Value::Id(s) => format!("IFCIDENTIFIER({})", text(s)),
            Value::Length(v) => format!("IFCLENGTHMEASURE({})", real(*v)),
            Value::PositiveLength(v) => format!("IFCPOSITIVELENGTHMEASURE({})", real(*v)),
            Value::Area(v) => format!("IFCAREAMEASURE({})", real(*v)),
            Value::Volume(v) => format!("IFCVOLUMEMEASURE({})", real(*v)),
            Value::Ratio(v) => format!("IFCRATIOMEASURE({})", real(*v)),
            Value::PositiveRatio(v) => format!("IFCPOSITIVERATIOMEASURE({})", real(*v)),
            Value::Count(v) => format!("IFCCOUNTMEASURE({v})"),
            Value::Real(v) => format!("IFCREAL({})", real(*v)),
            Value::Mass(v) => format!("IFCMASSMEASURE({})", real(*v)),
            Value::PlaneAngle(v) => format!("IFCPLANEANGLEMEASURE({})", real(*v)),
            Value::Labels(list) => format!(
                "({})",
                list.iter()
                    .map(|l| format!("IFCLABEL({})", text(l)))
                    .collect::<Vec<_>>()
                    .join(",")
            ),
        }
    }
}

/// A property set (`Pset_…`): its name and its properties in order.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Pset {
    pub name: String,
    pub props: Vec<(String, Value)>,
}

/// An element quantity set (`Qto_…`): areas, lengths, volumes.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Qto {
    pub name: String,
    pub quantities: Vec<(String, Value)>,
}

/// A body: an extruded prism (its footprint's rings, outer rings and holes alike by the even-odd rule, between two
/// heights over its storey's floor) or a closed set of plane faces (its vertices, each face a ring of their indices).
#[derive(Clone, Debug, PartialEq)]
pub enum Shape {
    Prism {
        rings: Vec<Vec<[f64; 2]>>,
        /// Each ring's edges' bulges, each to its next point (docs/adr/8071 §1): an arc where it is not 0, positive
        /// bowing right of travel; a ring with none, or all 0, is straight. The file writes the arcs as arcs; what
        /// reads a polygon (a mesh, an outline test) takes [`flat_rings`].
        bulges: Vec<Vec<f64>>,
        bottom: f64,
        top: f64,
    },
    Faces {
        vertices: Vec<[f64; 3]>,
        faces: Vec<Vec<usize>>,
    },
}

/// Rings with their arcs flattened at the drawing's step (docs/adr/8071 §1; at most 5° a chord): for what reads a
/// polygon. The measures are the arcs' own, never these.
pub fn flat_rings(rings: &[Vec<[f64; 2]>], bulges: &[Vec<f64>]) -> Vec<Vec<[f64; 2]>> {
    rings
        .iter()
        .enumerate()
        .map(|(i, r)| match bulges.get(i).filter(|b| has_bulges(Some(b))) {
            Some(b) => {
                let pts: Vec<Vec2> = r.iter().map(|p| Vec2::new(p[0], p[1])).collect();
                bulge_path_outline(&pts, Some(b), true, DEFAULT_STEP)
                    .into_iter()
                    .map(|p| [p.x, p.y])
                    .collect()
            }
            None => r.clone(),
        })
        .collect()
}

/// The element classes the building has (EK-5 Tablo 5.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Wall,
    Slab,
    Column,
    Beam,
    Roof,
    Railing,
    Stair,
    Covering,
    Door,
    Window,
    TransportElement,
    Furniture,
    SanitaryTerminal,
    Member,
    Chimney,
    Ramp,
    CurtainWall,
}

impl Class {
    /// Its IFC entity, as STEP spells it.
    pub fn entity(self) -> &'static str {
        match self {
            Class::Wall => "IFCWALL",
            Class::Slab => "IFCSLAB",
            Class::Column => "IFCCOLUMN",
            Class::Beam => "IFCBEAM",
            Class::Roof => "IFCROOF",
            Class::Railing => "IFCRAILING",
            Class::Stair => "IFCSTAIR",
            Class::Covering => "IFCCOVERING",
            Class::Door => "IFCDOOR",
            Class::Window => "IFCWINDOW",
            Class::TransportElement => "IFCTRANSPORTELEMENT",
            Class::Furniture => "IFCFURNITURE",
            Class::SanitaryTerminal => "IFCSANITARYTERMINAL",
            Class::Member => "IFCMEMBER",
            Class::Chimney => "IFCCHIMNEY",
            Class::Ramp => "IFCRAMP",
            Class::CurtainWall => "IFCCURTAINWALL",
        }
    }

    /// Its category code (EK-5 Tablo 5.1), the second field of its name.
    pub fn code(self) -> &'static str {
        match self {
            Class::Wall => "DVR",
            Class::Slab => "DSM",
            Class::Column => "KLN",
            Class::Beam => "KRS",
            Class::Roof => "CAT",
            Class::Railing => "KRK",
            Class::Stair => "MRD",
            Class::Covering => "KPL",
            Class::Door => "KAP",
            Class::Window => "PNC",
            Class::TransportElement => "TSM",
            Class::Furniture => "MBL",
            Class::SanitaryTerminal => "STT",
            Class::Member => "ELM",
            Class::Chimney => "BAC",
            Class::Ramp => "RMP",
            Class::CurtainWall => "GDC",
        }
    }
}

/// An element of the building.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub class: Class,
    /// Its KentOS id (a UUID's 128 bits): its GlobalId. None: one made of the file's seed.
    pub id: Option<u128>,
    /// The KentOS object it shows when that is not its id's (a roof's chimney stack: the roof): what a click on it in
    /// the 3D view selects. Not written to the file.
    pub object: Option<u128>,
    /// Its name (EK-5 §5.5: discipline, category code, description).
    pub name: String,
    pub description: Option<String>,
    /// Its tag (EK-6: an instance's mark: K1, P3, its KentOS id).
    pub tag: Option<String>,
    /// Its predefined type, as STEP spells the enumeration's item (`STANDARD`, `FLOOR`, `DOOR` …).
    pub predefined: &'static str,
    /// Its object type: what a `USERDEFINED` one is (IFC's rule: a user-defined type is said in words).
    pub object_type: Option<String>,
    /// The element it is a part of, by index into [`Building::elements`] (a curtain wall's mullion): written in that
    /// one's `IfcRelAggregates`, not on a storey of its own.
    pub part_of: Option<usize>,
    /// Its storey, by index into [`Building::storeys`].
    pub storey: usize,
    pub shapes: Vec<Shape>,
    /// A door's or a window's overall height and width.
    pub overall: Option<(f64, f64)>,
    /// Its material's name (EK-6: "Malzeme_Dayanım": Beton_C25, Tugla).
    pub material: Option<String>,
    pub psets: Vec<Pset>,
    pub qtos: Vec<Qto>,
}

/// An opening through a wall: the wall's index into [`Building::elements`], its footprint through the wall between two
/// heights over its storey's floor, and the door or window that fills it, by index.
#[derive(Clone, Debug, PartialEq)]
pub struct Opening {
    pub id: Option<u128>,
    pub name: String,
    pub host: usize,
    pub ring: Vec<[f64; 2]>,
    /// Its ring's edges' bulges (docs/adr/8071 §1): an opening through a curved wall runs along its faces' arcs.
    pub bulges: Vec<f64>,
    pub bottom: f64,
    pub top: f64,
    pub filler: Option<usize>,
    /// RECESS for a niche, OPENING through.
    pub predefined: &'static str,
}

/// A space (mahal): its footprint's rings between two heights over its storey's floor.
#[derive(Clone, Debug, PartialEq)]
pub struct Space {
    pub id: Option<u128>,
    /// Its name (EK-5 §5.5.4: its number and its name in the description).
    pub name: String,
    /// Its long name: its name as the drawing writes it.
    pub long_name: Option<String>,
    pub storey: usize,
    pub rings: Vec<Vec<[f64; 2]>>,
    /// Each ring's edges' bulges, as a prism's (docs/adr/8071 §1).
    pub bulges: Vec<Vec<f64>>,
    pub bottom: f64,
    pub top: f64,
    /// INTERNAL, EXTERNAL, PARKING …
    pub predefined: &'static str,
    pub psets: Vec<Pset>,
    pub qtos: Vec<Qto>,
    /// Its body under a roof (docs/adr/8071 §11): its pieces up to the roof's underside, each a closed face set; empty:
    /// its prism from its bottom to its top.
    pub body: Vec<Shape>,
}

impl Space {
    /// Its body as written: its pieces under a roof, else its prism.
    pub fn shapes(&self) -> Vec<Shape> {
        if self.body.is_empty() {
            vec![Shape::Prism { rings: self.rings.clone(), bulges: self.bulges.clone(), bottom: self.bottom, top: self.top }]
        } else {
            self.body.clone()
        }
    }
}

/// A storey: its name, its floor's elevation over ±0.00, its own properties and quantities.
#[derive(Clone, Debug, PartialEq)]
pub struct Storey {
    pub id: Option<u128>,
    pub name: String,
    pub long_name: Option<String>,
    pub elevation: f64,
    pub psets: Vec<Pset>,
    pub qtos: Vec<Qto>,
}

/// A postal address (`IfcPostalAddress`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Address {
    pub lines: Vec<String>,
    /// Its mail drop (EK-6 Tablo 6.7 `PostalBox`, docs/adr/8076 §5).
    pub postal_box: Option<String>,
    pub town: Option<String>,
    pub region: Option<String>,
    pub postal_code: Option<String>,
    pub country: Option<String>,
}

/// An organisation (EK-7 Tablo 7.4): its tax number (`Identification`), its name and its address.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Organization {
    pub identification: Option<String>,
    pub name: String,
    pub address: Option<Address>,
}

/// One who acts on the project (EK-7 Tablo 7.2–7.5): a person (their chamber's registration number as
/// `Identification`, their family and given names), the organisation they act for, and their role (an `IfcRoleEnum`
/// value, ARCHITECT; its description their title, Mimar). Written as an `IfcActor` assigned to the project
/// (`IfcRelAssignsToActor`).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Actor {
    pub identification: Option<String>,
    pub family: Option<String>,
    pub given: Option<String>,
    pub organization: Option<Organization>,
    pub role: &'static str,
    pub role_description: Option<String>,
}

/// A spatial object's words and data: the project, the site or the building.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Place {
    pub id: Option<u128>,
    pub name: String,
    pub long_name: Option<String>,
    pub description: Option<String>,
    pub object_type: Option<String>,
    pub address: Option<Address>,
    pub psets: Vec<Pset>,
    pub qtos: Vec<Qto>,
}

pub use crate::crs::Crs;

/// A building to write.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Building {
    /// The file's name, without its extension.
    pub file: String,
    /// What its bodies are: whole with openings the reader takes out, or already cut (docs/adr/8071 §4).
    pub view: View,
    /// `YYYY-MM-DDThh:mm:ss`.
    pub stamp: String,
    pub application: String,
    /// The project's phase (`IfcProject.Phase`: Ön proje, Uygulama projesi).
    pub phase: Option<String>,
    pub project: Place,
    /// Those who act on the project (EK-7): the architect.
    pub actors: Vec<Actor>,
    pub site: Place,
    /// The site's height over sea level (`RefElevation`).
    pub site_elevation: Option<f64>,
    pub building: Place,
    pub storeys: Vec<Storey>,
    pub elements: Vec<Element>,
    pub openings: Vec<Opening>,
    pub spaces: Vec<Space>,
    /// The spatial zones (EK-6 Tablo 6.60): each storey's emsal areas (docs/adr/8069).
    pub zones: Vec<Zone>,
    /// The groups of spaces (EK-6 Tablo 6.68): an independent unit, an annex, a common area (docs/adr/8069).
    pub groups: Vec<Group>,
    /// The grids (EK-6 Tablo 6.36's axes, docs/adr/8070 §4).
    pub grids: Vec<Grid>,
    /// The layered elements' material layer sets (docs/adr/8071 §8): an element (by its index), its set's name and its
    /// layers top down, in place of a single material.
    pub layer_sets: Vec<(usize, String, Vec<Layer>)>,
    pub crs: Option<Crs>,
    /// Why the model has no system when it has none (docs/adr/8072 §3): the export says it, EK-9's sixth question
    /// answers with it.
    pub crs_why: Option<String>,
    /// True north in the model's plane (EK-7 Tablo 7.8 `TrueNorth`; docs/adr/8076 §6): a unit vector, x and y; none
    /// when neither the project's system nor the drawing's north arrows say it, and why (`north_why`).
    pub true_north: Option<[f64; 2]>,
    pub north_why: Option<String>,
    /// The building's ±0.00 over the site's (the project's ±0.00), metres (docs/adr/8072 §2): its placement there, its
    /// storeys' elevations over it.
    pub zero: f64,
}

/// A layer of an element's material layer set (docs/adr/8071 §8; IFC 4.3 `IfcMaterialLayer`).
#[derive(Clone, Debug, PartialEq)]
pub struct Layer {
    /// What it is (its `Name`).
    pub name: String,
    /// Its material's name; none: not given, which is no air gap, only not said (its `Material`).
    pub material: Option<String>,
    /// How thick it is, metres (its `LayerThickness`).
    pub thickness: f64,
    /// Its role in the set (its `Category`): IFC 4.3's own words, 'LoadBearing', 'Insulation', 'Inner finish' or
    /// 'Outer finish'; none, none of them.
    pub category: Option<&'static str>,
}

/// What the file's bodies are, its header's view definition (docs/adr/8071 §4; IFC 4.3, `IfcRelVoidsElement`): Design
/// Transfer writes each body whole and its openings' bodies as 'Body', which the reader takes out of their hosts; a
/// Reference View writes each body already cut (`quantities::cut`) and its openings as 'Reference', "not subtracted",
/// there "in addition to the hole in the Body shape representation of the voided element".
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum View {
    #[default]
    DesignTransfer,
    Reference,
}

/// A grid (`IfcGrid`): its name, its storey (none: the building's, on every storey), its axes in two directions (U and
/// V: IFC 4.3 asks at least one of each) and its predefined type.
#[derive(Clone, Debug, PartialEq)]
pub struct Grid {
    pub id: Option<u128>,
    pub name: String,
    pub storey: Option<usize>,
    pub u: Vec<Axis>,
    pub v: Vec<Axis>,
    pub predefined: &'static str,
}

/// A grid's axis (`IfcGridAxis`, EK-6 Tablo 6.36): its tag (A, 1) and its line in plan, from `from` to `to`; an arc
/// through `through`, the point halfway along it.
#[derive(Clone, Debug, PartialEq)]
pub struct Axis {
    /// Its KentOS object's id (the grid's): not written, an axis has no GlobalId; what a missing tag names.
    pub id: Option<u128>,
    pub tag: String,
    pub from: [f64; 2],
    pub to: [f64; 2],
    pub through: Option<[f64; 2]>,
}

/// A spatial zone (`IfcSpatialZone`): its footprint's rings between two heights over its storey's floor, its words, its
/// predefined type (`USERDEFINED` with its object type for an emsal zone) and its property sets.
#[derive(Clone, Debug, PartialEq)]
pub struct Zone {
    pub id: Option<u128>,
    pub name: String,
    pub long_name: Option<String>,
    pub description: Option<String>,
    pub object_type: Option<String>,
    pub predefined: &'static str,
    pub storey: usize,
    pub rings: Vec<Vec<[f64; 2]>>,
    pub bottom: f64,
    pub top: f64,
    pub psets: Vec<Pset>,
}

/// A group of spaces (`IfcZone`): its words and its spaces, by index into [`Building::spaces`].
#[derive(Clone, Debug, PartialEq)]
pub struct Group {
    pub id: Option<u128>,
    pub name: String,
    pub long_name: Option<String>,
    pub object_type: Option<String>,
    pub spaces: Vec<usize>,
    pub psets: Vec<Pset>,
}

/// Which of `rings` are outer rings and which holes (even-odd: a ring inside an odd number of others is a hole), each
/// hole with the nearest outer ring holding it: (outer, holes).
pub(crate) fn nest(rings: &[Vec<[f64; 2]>]) -> Vec<(usize, Vec<usize>)> {
    let inside = |p: [f64; 2], r: &[[f64; 2]]| {
        let mut odd = false;
        let n = r.len();
        for i in 0..n {
            let (a, b) = (r[i], r[(i + 1) % n]);
            if (a[1] > p[1]) != (b[1] > p[1])
                && p[0] < a[0] + (p[1] - a[1]) * (b[0] - a[0]) / (b[1] - a[1])
            {
                odd = !odd;
            }
        }
        odd
    };
    // A ring's point: the middle of its first edge, off any other ring's corner.
    let probe = |r: &[[f64; 2]]| {
        let (a, b) = (r[0], r[1 % r.len()]);
        [(a[0] + b[0]) / 2.0, (a[1] + b[1]) / 2.0]
    };
    let holders: Vec<Vec<usize>> = (0..rings.len())
        .map(|i| {
            (0..rings.len())
                .filter(|&j| j != i && rings[i].len() > 1 && inside(probe(&rings[i]), &rings[j]))
                .collect()
        })
        .collect();
    let mut out: Vec<(usize, Vec<usize>)> = (0..rings.len())
        .filter(|&i| holders[i].len().is_multiple_of(2))
        .map(|i| (i, Vec::new()))
        .collect();
    for (i, held) in holders.iter().enumerate() {
        if held.len() % 2 == 1 {
            // The outer ring holding it directly: the one held by as many as it less one.
            if let Some(o) = held.iter().find(|&&j| holders[j].len() + 1 == held.len())
                && let Some(slot) = out.iter_mut().find(|(k, _)| k == o)
            {
                slot.1.push(i);
            }
        }
    }
    out
}

struct Writer {
    st: Step,
    body: usize,
    axis_up: usize,
    axis_x: usize,
    origin: usize,
}

impl Writer {
    fn id(&mut self, id: Option<u128>) -> String {
        match id {
            Some(n) => text(&guid(n)),
            None => self.st.gid(),
        }
    }

    fn opt(s: &Option<String>) -> String {
        s.as_deref().map_or("$".into(), text)
    }

    fn point3(&mut self, p: [f64; 3]) -> usize {
        self.st.add(format!(
            "IFCCARTESIANPOINT(({},{},{}))",
            real(p[0]),
            real(p[1]),
            real(p[2])
        ))
    }

    /// A placement at `z` over its parent's, unturned.
    fn placement(&mut self, parent: Option<usize>, z: f64) -> usize {
        let at = if z == 0.0 {
            self.origin
        } else {
            self.point3([0.0, 0.0, z])
        };
        let axes = self.st.add(format!(
            "IFCAXIS2PLACEMENT3D(#{at},#{},#{})",
            self.axis_up, self.axis_x
        ));
        let parent = parent.map_or("$".into(), |p| format!("#{p}"));
        self.st.add(format!("IFCLOCALPLACEMENT({parent},#{axes})"))
    }

    /// A ring's closed curve: an indexed curve of line and arc segments where an edge is an arc (docs/adr/8071 §1;
    /// `IfcArcIndex`: its start, its middle, its end), else a polyline, its first point again at its end.
    fn curve(&mut self, ring: &[[f64; 2]], bulges: Option<&[f64]>) -> usize {
        let n = ring.len();
        let at = |i: usize| Vec2::new(ring[i % n][0], ring[i % n][1]);
        let arc = |i: usize| {
            bulges
                .and_then(|b| b.get(i).copied())
                .filter(|&b| bulge_arc(at(i), at(i + 1), b).is_some())
        };
        if (0..n).any(|i| arc(i).is_some()) {
            let mut pts: Vec<[f64; 2]> = Vec::new();
            let mut segments: Vec<String> = Vec::new();
            let mut straight: Vec<usize> = Vec::new(); // the run of line indices not yet written
            for i in 0..n {
                let start = pts.len() + 1;
                pts.push(ring[i]);
                let bulge = arc(i);
                let end = if i + 1 == n { 1 } else { start + 1 + usize::from(bulge.is_some()) };
                match bulge {
                    Some(b) => {
                        if straight.len() >= 2 {
                            segments.push(Self::line_index(&straight));
                        }
                        straight.clear();
                        let m = segment_mid(at(i), at(i + 1), b);
                        pts.push([m.x, m.y]);
                        segments.push(format!("IFCARCINDEX(({start},{},{end}))", start + 1));
                    }
                    None => {
                        if straight.is_empty() {
                            straight.push(start);
                        }
                        straight.push(end);
                    }
                }
            }
            if straight.len() >= 2 {
                segments.push(Self::line_index(&straight));
            }
            let coords: Vec<String> = pts.iter().map(|p| format!("({},{})", real(p[0]), real(p[1]))).collect();
            let list = self.st.add(format!("IFCCARTESIANPOINTLIST2D(({}),$)", coords.join(",")));
            return self.st.add(format!("IFCINDEXEDPOLYCURVE(#{list},({}),.F.)", segments.join(",")));
        }
        let mut pts: Vec<usize> = ring
            .iter()
            .map(|p| {
                self.st.add(format!(
                    "IFCCARTESIANPOINT(({},{}))",
                    real(p[0]),
                    real(p[1])
                ))
            })
            .collect();
        pts.push(pts[0]);
        self.st.add(format!("IFCPOLYLINE(({}))", Step::refs(&pts)))
    }

    /// A run of straight edges as one segment of an indexed curve.
    fn line_index(run: &[usize]) -> String {
        let list: Vec<String> = run.iter().map(|i| i.to_string()).collect();
        format!("IFCLINEINDEX(({}))", list.join(","))
    }

    /// A shape's items: one extrusion an outer ring (with its holes), or a face set.
    fn items(&mut self, shape: &Shape) -> Vec<usize> {
        match shape {
            Shape::Prism { rings, bulges, bottom, top } => {
                // Which ring is in which by the arcs' own outline, not their chords; a ring of two points and two
                // arcs (a circle) is a ring.
                let all = flat_rings(rings, bulges);
                let kept: Vec<usize> = (0..rings.len()).filter(|&i| all[i].len() >= 3).collect();
                let owned: Vec<Vec<[f64; 2]>> = kept.iter().map(|&i| rings[i].clone()).collect();
                let arcs: Vec<Vec<f64>> = kept.iter().map(|&i| bulges.get(i).cloned().unwrap_or_default()).collect();
                let flat: Vec<Vec<[f64; 2]>> = kept.iter().map(|&i| all[i].clone()).collect();
                let mut out = Vec::new();
                for (outer, holes) in nest(&flat) {
                    let o = self.curve(&owned[outer], Some(&arcs[outer]));
                    let profile = if holes.is_empty() {
                        self.st
                            .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#{o})"))
                    } else {
                        let inner: Vec<usize> =
                            holes.iter().map(|&h| self.curve(&owned[h], Some(&arcs[h]))).collect();
                        self.st.add(format!(
                            "IFCARBITRARYPROFILEDEFWITHVOIDS(.AREA.,$,#{o},({}))",
                            Step::refs(&inner)
                        ))
                    };
                    let at = self.point3([0.0, 0.0, *bottom]);
                    let position = self.st.add(format!("IFCAXIS2PLACEMENT3D(#{at},$,$)"));
                    let depth = (top - bottom).max(1e-6);
                    out.push(self.st.add(format!(
                        "IFCEXTRUDEDAREASOLID(#{profile},#{position},#{},{})",
                        self.axis_up,
                        real(depth)
                    )));
                }
                out
            }
            Shape::Faces { vertices, faces } => {
                let coords: Vec<String> = vertices
                    .iter()
                    .map(|p| format!("({},{},{})", real(p[0]), real(p[1]), real(p[2])))
                    .collect();
                let list = self
                    .st
                    .add(format!("IFCCARTESIANPOINTLIST3D(({}),$)", coords.join(",")));
                let faces: Vec<usize> = faces
                    .iter()
                    .filter(|f| f.len() >= 3)
                    .map(|f| {
                        let idx: Vec<String> = f.iter().map(|i| (i + 1).to_string()).collect();
                        self.st
                            .add(format!("IFCINDEXEDPOLYGONALFACE(({}))", idx.join(",")))
                    })
                    .collect();
                vec![self.st.add(format!(
                    "IFCPOLYGONALFACESET(#{list},.T.,({}),$)",
                    Step::refs(&faces)
                ))]
            }
        }
    }

    /// A product's body: its shapes' items in one representation.
    fn body(&mut self, shapes: &[Shape]) -> Option<usize> {
        self.shape("Body", shapes)
    }

    /// A product's shape: its shapes' items in one representation named `identifier` ('Body', or an opening's
    /// 'Reference' in a Reference View).
    fn shape(&mut self, identifier: &str, shapes: &[Shape]) -> Option<usize> {
        let mut items = Vec::new();
        let mut faces = false;
        for s in shapes {
            faces |= matches!(s, Shape::Faces { .. });
            items.extend(self.items(s));
        }
        if items.is_empty() {
            return None;
        }
        let kind = if faces { "Tessellation" } else { "SweptSolid" };
        let rep = self.st.add(format!(
            "IFCSHAPEREPRESENTATION(#{},'{identifier}','{kind}',({}))",
            self.body,
            Step::refs(&items)
        ));
        Some(
            self.st
                .add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}))")),
        )
    }

    fn psets(&mut self, object: usize, psets: &[Pset], qtos: &[Qto]) {
        for p in psets.iter().filter(|p| !p.props.is_empty()) {
            let props: Vec<usize> = p
                .props
                .iter()
                .map(|(n, v)| match v {
                    Value::Labels(_) => self.st.add(format!(
                        "IFCPROPERTYLISTVALUE({},$,{},$)",
                        text(n),
                        v.step()
                    )),
                    _ => self.st.add(format!(
                        "IFCPROPERTYSINGLEVALUE({},$,{},$)",
                        text(n),
                        v.step()
                    )),
                })
                .collect();
            let g = self.st.gid();
            let set = self.st.add(format!(
                "IFCPROPERTYSET({g},$,{},$,({}))",
                text(&p.name),
                Step::refs(&props)
            ));
            let g = self.st.gid();
            self.st.add(format!(
                "IFCRELDEFINESBYPROPERTIES({g},$,$,$,(#{object}),#{set})"
            ));
        }
        for q in qtos.iter().filter(|q| !q.quantities.is_empty()) {
            let items: Vec<usize> = q
                .quantities
                .iter()
                .map(|(n, v)| {
                    let body = match v {
                        Value::Area(a) => {
                            format!("IFCQUANTITYAREA({},$,$,{},$)", text(n), real(*a))
                        }
                        Value::Volume(a) => {
                            format!("IFCQUANTITYVOLUME({},$,$,{},$)", text(n), real(*a))
                        }
                        Value::Count(c) => format!("IFCQUANTITYCOUNT({},$,$,{}.,$)", text(n), c),
                        other => {
                            let v = match other {
                                Value::Length(l) | Value::Real(l) | Value::Ratio(l) => *l,
                                _ => 0.0,
                            };
                            format!("IFCQUANTITYLENGTH({},$,$,{},$)", text(n), real(v))
                        }
                    };
                    self.st.add(body)
                })
                .collect();
            let g = self.st.gid();
            let set = self.st.add(format!(
                "IFCELEMENTQUANTITY({g},$,{},$,$,({}))",
                text(&q.name),
                Step::refs(&items)
            ));
            let g = self.st.gid();
            self.st.add(format!(
                "IFCRELDEFINESBYPROPERTIES({g},$,$,$,(#{object}),#{set})"
            ));
        }
    }

    fn address(&mut self, a: &Option<Address>) -> String {
        let Some(a) = a else {
            return "$".into();
        };
        let lines = if a.lines.is_empty() {
            "$".into()
        } else {
            format!(
                "({})",
                a.lines
                    .iter()
                    .map(|l| text(l))
                    .collect::<Vec<_>>()
                    .join(",")
            )
        };
        let n = self.st.add(format!(
            "IFCPOSTALADDRESS($,$,$,$,{lines},{},{},{},{},{})",
            Self::opt(&a.postal_box),
            Self::opt(&a.town),
            Self::opt(&a.region),
            Self::opt(&a.postal_code),
            Self::opt(&a.country)
        ));
        format!("#{n}")
    }
}

/// The file: KentOS's building as IFC 4.3.
pub fn write(b: &Building) -> String {
    let mut st = Step::new(format!("{}|{}", b.file, b.stamp));
    let length = st.add("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)");
    let area = st.add("IFCSIUNIT(*,.AREAUNIT.,$,.SQUARE_METRE.)");
    let volume = st.add("IFCSIUNIT(*,.VOLUMEUNIT.,$,.CUBIC_METRE.)");
    let angle = st.add("IFCSIUNIT(*,.PLANEANGLEUNIT.,$,.RADIAN.)");
    // The kilogram only where a mass is written (a lift's rated load): a file without one stays as it was.
    let massive = b.elements.iter().any(|e| {
        e.psets.iter().any(|p| p.props.iter().any(|(_, v)| matches!(v, Value::Mass(_))))
    });
    let units = if massive {
        let mass = st.add("IFCSIUNIT(*,.MASSUNIT.,.KILO.,.GRAM.)");
        st.add(format!(
            "IFCUNITASSIGNMENT((#{length},#{area},#{volume},#{angle},#{mass}))"
        ))
    } else {
        st.add(format!(
            "IFCUNITASSIGNMENT((#{length},#{area},#{volume},#{angle}))"
        ))
    };
    let origin = st.add("IFCCARTESIANPOINT((0.,0.,0.))");
    let axis_up = st.add("IFCDIRECTION((0.,0.,1.))");
    let axis_x = st.add("IFCDIRECTION((1.,0.,0.))");
    let world = st.add(format!(
        "IFCAXIS2PLACEMENT3D(#{origin},#{axis_up},#{axis_x})"
    ));
    // True north as a direction in the model's plane (EK-7 Tablo 7.8); `$`, IFC's default up the y axis, never
    // written in its place when it is not known.
    let north = match b.true_north {
        Some([x, y]) => format!("#{}", st.add(format!("IFCDIRECTION(({},{}))", crate::step::real(x), crate::step::real(y)))),
        None => "$".to_owned(),
    };
    let model = st.add(format!(
        "IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#{world},{north})"
    ));
    let body = st.add(format!(
        "IFCGEOMETRICREPRESENTATIONSUBCONTEXT('Body','Model',*,*,*,*,#{model},$,.MODEL_VIEW.,$)"
    ));
    let mut w = Writer {
        st,
        body,
        axis_up,
        axis_x,
        origin,
    };
    if let Some(crs) = &b.crs {
        crate::crs::write(&mut w.st, crs, model, length);
    }
    // The project.
    let g = w.id(b.project.id);
    let project = w.st.add(format!(
        "IFCPROJECT({g},$,{},{},{},{},{},(#{model}),#{units})",
        text(&b.project.name),
        Writer::opt(&b.project.description),
        Writer::opt(&b.project.object_type),
        Writer::opt(&b.project.long_name),
        Writer::opt(&b.phase)
    ));
    w.psets(project, &b.project.psets, &b.project.qtos);
    // Those who act on it (EK-7): person, organisation, role, assigned to the project. The role is the person's and
    // the organisation's too: EK-7 Tablo 7.2 and 7.4 make their `Roles` mandatory (docs/adr/8074 §3).
    for a in &b.actors {
        let role = w.st.add(format!(
            "IFCACTORROLE(.{}.,$,{})",
            a.role,
            Writer::opt(&a.role_description)
        ));
        let person = w.st.add(format!(
            "IFCPERSON({},{},{},$,$,$,(#{role}),$)",
            Writer::opt(&a.identification),
            Writer::opt(&a.family),
            Writer::opt(&a.given)
        ));
        let who = match &a.organization {
            Some(o) => {
                let address = w.address(&o.address);
                let addresses = if address == "$" {
                    "$".to_owned()
                } else {
                    format!("({address})")
                };
                let org = w.st.add(format!(
                    "IFCORGANIZATION({},{},$,(#{role}),{addresses})",
                    Writer::opt(&o.identification),
                    text(&o.name)
                ));
                w.st.add(format!("IFCPERSONANDORGANIZATION(#{person},#{org},(#{role}))"))
            }
            None => person,
        };
        let g = w.st.gid();
        let actor = w.st.add(format!("IFCACTOR({g},$,$,$,$,#{who})"));
        let g = w.st.gid();
        w.st.add(format!(
            "IFCRELASSIGNSTOACTOR({g},$,$,$,(#{project}),$,#{actor},#{role})"
        ));
    }
    // The site, the building and the storeys.
    let site_place = w.placement(None, 0.0);
    let g = w.id(b.site.id);
    let site_address = w.address(&b.site.address);
    let site = w.st.add(format!(
        "IFCSITE({g},$,{},{},{},#{site_place},$,{},.ELEMENT.,$,$,{},$,{site_address})",
        text(&b.site.name),
        Writer::opt(&b.site.description),
        Writer::opt(&b.site.object_type),
        Writer::opt(&b.site.long_name),
        b.site_elevation.map_or("$".into(), real)
    ));
    w.psets(site, &b.site.psets, &b.site.qtos);
    let building_place = w.placement(Some(site_place), b.zero);
    let g = w.id(b.building.id);
    let building_address = w.address(&b.building.address);
    let building = w.st.add(format!(
        "IFCBUILDING({g},$,{},{},{},#{building_place},$,{},.ELEMENT.,$,$,{building_address})",
        text(&b.building.name),
        Writer::opt(&b.building.description),
        Writer::opt(&b.building.object_type),
        Writer::opt(&b.building.long_name)
    ));
    w.psets(building, &b.building.psets, &b.building.qtos);
    let mut storeys = Vec::new();
    let mut storey_places = Vec::new();
    for s in &b.storeys {
        let place = w.placement(Some(building_place), s.elevation);
        let g = w.id(s.id);
        let n = w.st.add(format!(
            "IFCBUILDINGSTOREY({g},$,{},$,$,#{place},$,{},.ELEMENT.,{})",
            text(&s.name),
            Writer::opt(&s.long_name),
            real(s.elevation)
        ));
        w.psets(n, &s.psets, &s.qtos);
        storeys.push(n);
        storey_places.push(place);
    }
    let g = w.st.gid();
    w.st.add(format!("IFCRELAGGREGATES({g},$,$,$,#{project},(#{site}))"));
    let g = w.st.gid();
    w.st.add(format!("IFCRELAGGREGATES({g},$,$,$,#{site},(#{building}))"));
    if !storeys.is_empty() {
        let g = w.st.gid();
        w.st.add(format!(
            "IFCRELAGGREGATES({g},$,$,$,#{building},({}))",
            Step::refs(&storeys)
        ));
    }
    // The elements on their storeys.
    let mut contained: Vec<Vec<usize>> = vec![Vec::new(); b.storeys.len()];
    let mut products = Vec::new();
    let mut materials: Vec<(String, Vec<usize>)> = Vec::new();
    // An element's material layer set (docs/adr/8071 §8), by its index.
    let mut sets: Vec<(usize, usize)> = Vec::new();
    for (i, e) in b.elements.iter().enumerate() {
        let Some(&parent) = storey_places.get(e.storey) else {
            products.push(0);
            continue;
        };
        let place = w.placement(Some(parent), 0.0);
        // A Reference View's body is already cut (docs/adr/8071 §4).
        let cut;
        let shapes: &[Shape] = match b.view {
            View::DesignTransfer => &e.shapes,
            View::Reference => {
                let hosted: Vec<&Opening> = b.openings.iter().filter(|o| o.host == i && o.ring.len() >= 2).collect();
                cut = crate::quantities::cut(&e.shapes, &hosted);
                &cut
            }
        };
        let shape = w.body(shapes).map_or("$".into(), |s| format!("#{s}"));
        let g = w.id(e.id);
        let head = format!(
            "{g},$,{},{},{},#{place},{shape},{}",
            text(&e.name),
            Writer::opt(&e.description),
            Writer::opt(&e.object_type),
            Writer::opt(&e.tag)
        );
        let n = match (e.class, e.overall) {
            (Class::Door, overall) => {
                let (h, wd) =
                    overall.map_or(("$".into(), "$".into()), |(h, wd)| (real(h), real(wd)));
                w.st.add(format!("IFCDOOR({head},{h},{wd},.{}.,$,$)", e.predefined))
            }
            (Class::Window, overall) => {
                let (h, wd) =
                    overall.map_or(("$".into(), "$".into()), |(h, wd)| (real(h), real(wd)));
                w.st.add(format!("IFCWINDOW({head},{h},{wd},.{}.,$,$)", e.predefined))
            }
            (c, _) => {
                w.st.add(format!("{}({head},.{}.)", c.entity(), e.predefined))
            }
        };
        w.psets(n, &e.psets, &e.qtos);
        if b.layer_sets.iter().any(|(k, ..)| *k == i) {
            sets.push((i, n));
        } else if let Some(m) = &e.material {
            match materials.iter_mut().find(|(name, _)| name == m) {
                Some((_, list)) => list.push(n),
                None => materials.push((m.clone(), vec![n])),
            }
        }
        // A part is in its whole's aggregation, not on a storey of its own (IFC 4.3: a decomposed element's parts).
        if e.part_of.is_none() {
            contained[e.storey].push(n);
        }
        products.push(n);
    }
    // The wholes and their parts (a curtain wall's mullions and transoms).
    let mut wholes: Vec<(usize, Vec<usize>)> = Vec::new();
    for (k, e) in b.elements.iter().enumerate() {
        let Some(whole) = e.part_of else { continue };
        let (Some(&part), Some(&parent)) = (products.get(k), products.get(whole)) else {
            continue;
        };
        if part == 0 || parent == 0 {
            continue;
        }
        match wholes.iter_mut().find(|(p, _)| *p == parent) {
            Some((_, list)) => list.push(part),
            None => wholes.push((parent, vec![part])),
        }
    }
    for (parent, parts) in &wholes {
        let g = w.st.gid();
        w.st.add(format!("IFCRELAGGREGATES({g},$,$,$,#{parent},({}))", Step::refs(parts)));
    }
    // The openings: each voids its wall, a door or a window fills it.
    for o in &b.openings {
        let (Some(&host), Some(host_el)) = (products.get(o.host), b.elements.get(o.host)) else {
            continue;
        };
        if host == 0 {
            continue;
        }
        let Some(&parent) = storey_places.get(host_el.storey) else {
            continue;
        };
        let place = w.placement(Some(parent), 0.0);
        let identifier = match b.view {
            View::DesignTransfer => "Body",
            View::Reference => "Reference",
        };
        let shape = w
            .shape(
                identifier,
                &[Shape::Prism {
                    rings: vec![o.ring.clone()],
                    bulges: vec![o.bulges.clone()],
                    bottom: o.bottom,
                    top: o.top,
                }],
            )
            .map_or("$".into(), |s| format!("#{s}"));
        let g = w.id(o.id);
        let opening = w.st.add(format!(
            "IFCOPENINGELEMENT({g},$,{},$,$,#{place},{shape},$,.{}.)",
            text(&o.name),
            o.predefined
        ));
        let g = w.st.gid();
        w.st.add(format!("IFCRELVOIDSELEMENT({g},$,$,$,#{host},#{opening})"));
        if let Some(f) = o
            .filler
            .and_then(|f| products.get(f).copied())
            .filter(|f| *f != 0)
        {
            let g = w.st.gid();
            w.st.add(format!("IFCRELFILLSELEMENT({g},$,$,$,#{opening},#{f})"));
        }
    }
    // The spaces on their storeys.
    let mut space_ids: Vec<Option<usize>> = Vec::with_capacity(b.spaces.len());
    for s in &b.spaces {
        let Some(&parent) = storey_places.get(s.storey) else {
            space_ids.push(None);
            continue;
        };
        let place = w.placement(Some(parent), 0.0);
        let shape = w.body(&s.shapes()).map_or("$".into(), |x| format!("#{x}"));
        let g = w.id(s.id);
        let n = w.st.add(format!(
            "IFCSPACE({g},$,{},$,$,#{place},{shape},{},.ELEMENT.,.{}.,$)",
            text(&s.name),
            Writer::opt(&s.long_name),
            s.predefined
        ));
        w.psets(n, &s.psets, &s.qtos);
        space_ids.push(Some(n));
        // A space is part of its storey (IfcRelAggregates), not contained in it.
        let g = w.st.gid();
        w.st.add(format!(
            "IFCRELAGGREGATES({g},$,$,$,#{},(#{n}))",
            storeys[s.storey]
        ));
    }
    // The spatial zones, each referenced in its storey (IfcRelReferencedInSpatialStructure: a zone is no part of the
    // spatial decomposition; IFC 4.3).
    for z in &b.zones {
        let Some(&parent) = storey_places.get(z.storey) else {
            continue;
        };
        let place = w.placement(Some(parent), 0.0);
        let shape = w
            .body(&[Shape::Prism {
                rings: z.rings.clone(),
                bulges: Vec::new(),
                bottom: z.bottom,
                top: z.top,
            }])
            .map_or("$".into(), |x| format!("#{x}"));
        let g = w.id(z.id);
        let n = w.st.add(format!(
            "IFCSPATIALZONE({g},$,{},{},{},#{place},{shape},{},.{}.)",
            text(&z.name),
            Writer::opt(&z.description),
            Writer::opt(&z.object_type),
            Writer::opt(&z.long_name),
            z.predefined
        ));
        w.psets(n, &z.psets, &[]);
        let g = w.st.gid();
        w.st.add(format!(
            "IFCRELREFERENCEDINSPATIALSTRUCTURE({g},$,$,$,(#{n}),#{})",
            storeys[z.storey]
        ));
    }
    // The groups: each its spaces (IfcRelAssignsToGroup); a group with none of its spaces written is left out.
    for gr in &b.groups {
        let members: Vec<usize> = gr.spaces.iter().filter_map(|i| space_ids.get(*i).copied().flatten()).collect();
        if members.is_empty() {
            continue;
        }
        let g = w.id(gr.id);
        let z = w.st.add(format!(
            "IFCZONE({g},$,{},$,{},{})",
            text(&gr.name),
            Writer::opt(&gr.object_type),
            Writer::opt(&gr.long_name)
        ));
        w.psets(z, &gr.psets, &[]);
        let g = w.st.gid();
        w.st.add(format!(
            "IFCRELASSIGNSTOGROUP({g},$,$,$,({}),$,#{z})",
            Step::refs(&members)
        ));
    }
    // The grids (EK-6 Tablo 6.36; docs/adr/8070 §4): each axis its tag and its line, an arc an indexed curve's arc
    // segment; the grid's footprint their lines. A grid on a storey is its storey's, else the building's.
    let mut in_building = Vec::new();
    let mut footprint = None;
    for grid in b.grids.iter().filter(|g| !g.u.is_empty() && !g.v.is_empty()) {
        let context = *footprint.get_or_insert_with(|| {
            w.st.add(format!(
                "IFCGEOMETRICREPRESENTATIONSUBCONTEXT('FootPrint','Model',*,*,*,*,#{model},$,.PLAN_VIEW.,$)"
            ))
        });
        let parent = grid
            .storey
            .and_then(|s| storey_places.get(s).copied())
            .unwrap_or(building_place);
        let place = w.placement(Some(parent), 0.0);
        let mut curves = Vec::new();
        let mut axes = |w: &mut Writer, list: &[Axis]| -> Vec<usize> {
            list.iter()
                .map(|a| {
                    let pts: Vec<[f64; 2]> = match a.through {
                        Some(m) => vec![a.from, m, a.to],
                        None => vec![a.from, a.to],
                    };
                    let coords: Vec<String> = pts
                        .iter()
                        .map(|p| format!("({},{})", real(p[0]), real(p[1])))
                        .collect();
                    let list = w
                        .st
                        .add(format!("IFCCARTESIANPOINTLIST2D(({}),$)", coords.join(",")));
                    let segment = if a.through.is_some() {
                        "IFCARCINDEX((1,2,3))"
                    } else {
                        "IFCLINEINDEX((1,2))"
                    };
                    let curve = w
                        .st
                        .add(format!("IFCINDEXEDPOLYCURVE(#{list},({segment}),.F.)"));
                    curves.push(curve);
                    w.st.add(format!("IFCGRIDAXIS({},#{curve},.T.)", text(&a.tag)))
                })
                .collect()
        };
        let u = axes(&mut w, &grid.u);
        let v = axes(&mut w, &grid.v);
        let set = w
            .st
            .add(format!("IFCGEOMETRICCURVESET(({}))", Step::refs(&curves)));
        let rep = w.st.add(format!(
            "IFCSHAPEREPRESENTATION(#{context},'FootPrint','GeometricCurveSet',(#{set}))"
        ));
        let shape = w.st.add(format!("IFCPRODUCTDEFINITIONSHAPE($,$,(#{rep}))"));
        let g = w.id(grid.id);
        let n = w.st.add(format!(
            "IFCGRID({g},$,{},$,$,#{place},#{shape},({}),({}),$,.{}.)",
            text(&grid.name),
            Step::refs(&u),
            Step::refs(&v),
            grid.predefined
        ));
        match grid.storey {
            Some(s) if s < contained.len() => contained[s].push(n),
            _ => in_building.push(n),
        }
    }
    for (i, list) in contained.iter().enumerate().filter(|(_, l)| !l.is_empty()) {
        let g = w.st.gid();
        w.st.add(format!(
            "IFCRELCONTAINEDINSPATIALSTRUCTURE({g},$,$,$,({}),#{})",
            Step::refs(list),
            storeys[i]
        ));
    }
    if !in_building.is_empty() {
        let g = w.st.gid();
        w.st.add(format!(
            "IFCRELCONTAINEDINSPATIALSTRUCTURE({g},$,$,$,({}),#{building})",
            Step::refs(&in_building)
        ));
    }
    let mut material_ids: Vec<(String, usize)> = Vec::new();
    for (name, list) in &materials {
        let m = w.st.add(format!("IFCMATERIAL({},$,$)", text(name)));
        material_ids.push((name.clone(), m));
        let g = w.st.gid();
        w.st.add(format!(
            "IFCRELASSOCIATESMATERIAL({g},$,$,$,({}),#{m})",
            Step::refs(list)
        ));
    }
    // The layered elements' sets (docs/adr/8071 §8): a material for each name once, a layer for each with its thickness,
    // its name and its role; a layer's material not given stays unsaid.
    for (i, n) in sets {
        let Some((_, name, layers)) = b.layer_sets.iter().find(|(k, ..)| *k == i) else {
            continue;
        };
        let refs: Vec<usize> = layers
            .iter()
            .map(|l| {
                let material = match &l.material {
                    Some(m) => match material_ids.iter().find(|(x, _)| x == m) {
                        Some((_, id)) => format!("#{id}"),
                        None => {
                            let id = w.st.add(format!("IFCMATERIAL({},$,$)", text(m)));
                            material_ids.push((m.clone(), id));
                            format!("#{id}")
                        }
                    },
                    None => "$".to_owned(),
                };
                let category = l.category.map_or_else(|| "$".to_owned(), text);
                w.st.add(format!(
                    "IFCMATERIALLAYER({material},{},$,{},$,{category},$)",
                    real(l.thickness),
                    text(&l.name)
                ))
            })
            .collect();
        let set = w.st.add(format!("IFCMATERIALLAYERSET(({}),{},$)", Step::refs(&refs), text(name)));
        let g = w.st.gid();
        w.st.add(format!("IFCRELASSOCIATESMATERIAL({g},$,$,$,(#{n}),#{set})"));
    }
    // The header says what the bodies are (docs/adr/8047, 8071 §4): Design Transfer, an opening's body subtracted from
    // its wall's by the reader (IFC 4.3, IfcRelVoidsElement: a 'Body' opening is a Boolean difference); a Reference View
    // holds the walls already cut and their openings as 'Reference' only.
    w.st.file(
        match b.view {
            View::DesignTransfer => "ViewDefinition [DesignTransferView]",
            View::Reference => "ViewDefinition [ReferenceView]",
        },
        &format!("{}.ifc", b.file),
        &b.stamp,
        &b.application,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn square(x: f64, y: f64, s: f64) -> Vec<[f64; 2]> {
        vec![[x, y], [x + s, y], [x + s, y + s], [x, y + s]]
    }

    #[test]
    fn rings_nest_outer_and_holes() {
        let rings = vec![
            square(0.0, 0.0, 10.0),
            square(2.0, 2.0, 2.0),
            square(20.0, 0.0, 5.0),
        ];
        let mut n = nest(&rings);
        n.sort();
        assert_eq!(n, vec![(0, vec![1]), (2, vec![])]);
    }

    /// An emsal zone is a spatial zone referenced in its storey with its state; a unit groups its spaces (docs/adr/8069).
    #[test]
    fn an_emsal_zone_and_a_unit_s_group_are_written_with_their_relations() {
        let room = Space {
            id: Some(21),
            name: "MM-MHL-001_Salon".into(),
            long_name: Some("Salon".into()),
            storey: 0,
            rings: vec![square(0.0, 0.0, 4.0)],
            bulges: vec![],
            bottom: 0.0,
            top: 2.8,
            predefined: "INTERNAL",
            psets: vec![],
            qtos: vec![],
            body: Vec::new(),
        };
        let b = Building {
            file: "Ev".into(),
            stamp: "2026-11-16T12:00:00".into(),
            application: "KentOS".into(),
            project: Place { name: "Ev".into(), ..Place::default() },
            site: Place { name: "MM-SAH-Parsel".into(), ..Place::default() },
            building: Place { name: "MM-BNA-Ev".into(), ..Place::default() },
            storeys: vec![Storey {
                id: None,
                name: "MM-BNK-Zemin_kat".into(),
                long_name: Some("Zemin kat".into()),
                elevation: 0.0,
                psets: vec![],
                qtos: vec![],
            }],
            spaces: vec![room],
            zones: vec![Zone {
                id: None,
                name: "MM-MZN-Emsal_DAHIL_Zemin_kat".into(),
                long_name: Some("Zemin kat emsale dahil alan".into()),
                description: Some("Planlı Alanlar İmar Yönetmeliği 4/1-şş, 21".into()),
                object_type: Some("EMSAL".into()),
                predefined: "USERDEFINED",
                storey: 0,
                rings: vec![square(0.0, 0.0, 4.0)],
                bottom: 0.0,
                top: 2.8,
                psets: vec![Pset {
                    name: "TREpys_EmsalOzellikSeti".into(),
                    props: vec![("EmsalDurumu".into(), Value::Label("DAHIL".into()))],
                }],
            }],
            groups: vec![Group {
                id: None,
                name: "MM-ZON-1".into(),
                long_name: Some("Daire 1".into()),
                object_type: Some("Bağımsız bölüm".into()),
                spaces: vec![0],
                psets: vec![],
            }],
            ..Building::default()
        };
        let f = write(&b);
        let line = |prefix: &str| {
            f.lines().find(|l| l.split_once('=').is_some_and(|(_, r)| r.starts_with(prefix))).unwrap_or_else(|| panic!("{prefix}"))
        };
        let zone = line("IFCSPATIALZONE(");
        assert!(zone.contains("'EMSAL'") && zone.contains(".USERDEFINED.") && zone.contains("'Zemin kat emsale dahil alan'"), "{zone}");
        let zone_id = zone.split_once('=').map(|(i, _)| i.trim().to_owned()).expect("an id");
        assert!(line("IFCRELREFERENCEDINSPATIALSTRUCTURE(").contains(&format!("({zone_id})")));
        assert!(f.contains("'TREpys_EmsalOzellikSeti'") && f.contains("IFCLABEL('DAHIL')"));
        let space_id = line("IFCSPACE(").split_once('=').map(|(i, _)| i.trim().to_owned()).expect("an id");
        let group = line("IFCZONE(");
        // Its object type “Bağımsız bölüm”, its Turkish letters in STEP's \\X2\\ escape.
        assert!(group.contains("'MM-ZON-1'") && group.contains("'Daire 1'") && group.contains("\\X2\\011F0131\\X0\\"), "{group}");
        assert!(line("IFCRELASSIGNSTOGROUP(").contains(&format!("({space_id})")));
    }

    /// A curved wall's footprint is written with its arcs (docs/adr/8071 §1): one indexed curve, an arc through each arc's
    /// middle and a line for each straight end, closed on its first point; the profile is that curve.
    #[test]
    fn a_prism_s_arcs_are_written_as_arc_segments() {
        let (r1, r2) = (6.15, 5.85);
        let quarter = (std::f64::consts::PI / 8.0).tan();
        let wall = Element {
            class: Class::Wall,
            id: Some(7),
            object: None,
            name: "MM-DVR-Egri".into(),
            description: None,
            tag: None,
            predefined: "STANDARD",
            object_type: None,
            part_of: None,
            storey: 0,
            shapes: vec![Shape::Prism {
                rings: vec![vec![[r1, 0.0], [0.0, r1], [0.0, r2], [r2, 0.0]]],
                bulges: vec![vec![quarter, 0.0, -quarter, 0.0]],
                bottom: 0.0,
                top: 3.0,
            }],
            overall: None,
            material: None,
            psets: vec![],
            qtos: vec![],
        };
        let b = Building {
            storeys: vec![Storey {
                id: None,
                name: "MM-BNK-Zemin_kat".into(),
                long_name: None,
                elevation: 0.0,
                psets: vec![],
                qtos: vec![],
            }],
            elements: vec![wall],
            ..Building::default()
        };
        let f = write(&b);
        let line = |head: &str| f.lines().find(|l| l.contains(head)).unwrap_or_else(|| panic!("{head}")).to_owned();
        let id = |l: &str| l.split_once('=').map(|(i, _)| i.trim().to_owned()).expect("an id");
        let curve = line("IFCINDEXEDPOLYCURVE(");
        assert!(
            curve.contains("(IFCARCINDEX((1,2,3)),IFCLINEINDEX((3,4)),IFCARCINDEX((4,5,6)),IFCLINEINDEX((6,1))),.F.)"),
            "{curve}"
        );
        let list = line("IFCCARTESIANPOINTLIST2D(");
        assert!(curve.contains(&format!("({},", id(&list))), "{curve}");
        let inside = list.split_once("(((").and_then(|(_, r)| r.split_once("))")).map(|(i, _)| i).expect("its points");
        let pts: Vec<[f64; 2]> = inside
            .split("),(")
            .map(|p| {
                let (x, y) = p.split_once(',').expect("x,y");
                [x.parse().expect("x"), y.parse().expect("y")]
            })
            .collect();
        assert_eq!(pts.len(), 6);
        // Each arc's middle on its circle, at 45°.
        let h = std::f64::consts::FRAC_1_SQRT_2;
        for (p, r) in [(pts[1], r1), (pts[4], r2)] {
            assert!((p[0] - r * h).abs() < 1e-9 && (p[1] - r * h).abs() < 1e-9, "{p:?}");
        }
        assert!(line("IFCARBITRARYCLOSEDPROFILEDEF(").contains(&format!("#{})", id(&curve).trim_start_matches('#'))));
        assert!(!f.contains("IFCPOLYLINE("), "no chords");
    }

    /// A Reference View (docs/adr/8071 §4): its header says so, the wall's body is already cut (the door's band two
    /// pieces, the band over it whole) and the opening's shape is 'Reference', not 'Body'.
    #[test]
    fn a_reference_view_writes_the_wall_cut_and_its_opening_as_reference() {
        let mut b = door_building();
        b.view = View::Reference;
        let f = write(&b);
        assert!(f.contains("FILE_DESCRIPTION(('ViewDefinition [ReferenceView]'),'2;1');"), "{}", &f[..200]);
        let opening = f.lines().find(|l| l.contains("IFCOPENINGELEMENT(")).expect("the opening");
        let shape = opening.split(',').nth(6).expect("its shape").trim_start_matches('#').to_owned();
        let product = f.lines().find(|l| l.starts_with(&format!("#{shape}="))).expect("its product shape");
        let rep = product.split("(#").nth(1).and_then(|r| r.split(')').next()).expect("its representation");
        let rep_line = f.lines().find(|l| l.starts_with(&format!("#{rep}="))).expect("the representation");
        assert!(rep_line.contains(",'Reference','SweptSolid',"), "{rep_line}");
        assert_eq!(f.matches("IFCEXTRUDEDAREASOLID(").count(), 1 + 3, "the opening's and the wall's three bands");
        // Design Transfer: whole, 'Body'.
        let f = write(&door_building());
        assert!(f.contains("ViewDefinition [DesignTransferView]") && !f.contains("'Reference'"));
        assert_eq!(f.matches("IFCEXTRUDEDAREASOLID(").count(), 2);
    }

    /// A roof's layers (docs/adr/8071 §8): one IfcMaterialLayerSet top down in place of its material, each layer its
    /// name, thickness and role; a material written once for two layers of it, a layer without one unsaid.
    #[test]
    fn a_layered_roof_is_one_material_layer_set() {
        let mut b = door_building();
        b.elements[0].class = Class::Roof;
        b.elements[0].predefined = "NOTDEFINED";
        let layer = |name: &str, material: Option<&str>, thickness: f64, category: Option<&'static str>| Layer {
            name: name.into(),
            material: material.map(Into::into),
            thickness,
            category,
        };
        b.layer_sets = vec![(
            0,
            "Çatı katmanları".into(),
            vec![
                layer("Kaplama", Some("Kiremit"), 0.03, Some("Outer finish")),
                layer("Lata", None, 0.03, None),
                layer("Kaplama tahtası", Some("Çam"), 0.025, None),
                layer("Mertek", Some("Çam"), 0.125, Some("LoadBearing")),
            ],
        )];
        let f = write(&b);
        let layers: Vec<&str> = f.lines().filter(|l| l.contains("IFCMATERIALLAYER(")).collect();
        assert_eq!(layers.len(), 4, "{layers:?}");
        assert!(layers[0].contains(",0.03,$,'Kaplama',$,'Outer finish',$)"), "{}", layers[0]);
        assert!(layers[1].contains("IFCMATERIALLAYER($,0.03,$,'Lata',$,$,$)"), "{}", layers[1]);
        assert!(layers[3].contains(",0.125,$,'Mertek',$,'LoadBearing',$)"), "{}", layers[3]);
        assert_eq!(f.matches(&format!("IFCMATERIAL({},$,$)", text("Çam"))).count(), 1, "Çam once");
        let set = f.lines().find(|l| l.contains("IFCMATERIALLAYERSET(")).expect("the set");
        assert!(set.contains(&format!("),{},$)", text("Çatı katmanları"))), "{set}");
        // The roof's own material gives way to the set; the set is the roof's.
        assert!(!f.contains("IFCMATERIAL('Tugla'"));
        let roof = f.lines().find(|l| l.contains("IFCROOF(")).expect("the roof").split('=').next().unwrap().to_owned();
        let set_id = set.split('=').next().unwrap().to_owned();
        assert!(f.lines().any(|l| l.contains("IFCRELASSOCIATESMATERIAL(") && l.contains(&format!("({roof}),{set_id})"))));
    }

    /// A 5 m wall with a 0.9 m door through it, on one storey.
    fn door_building() -> Building {
        Building {
            file: "Ev".into(),
            stamp: "2026-11-11T12:00:00".into(),
            application: "KentOS".into(),
            project: Place {
                name: "Ev".into(),
                ..Place::default()
            },
            site: Place {
                name: "MM-SAH-Parsel".into(),
                ..Place::default()
            },
            building: Place {
                name: "MM-BNA-Ev".into(),
                ..Place::default()
            },
            storeys: vec![Storey {
                id: None,
                name: "MM-BNK-Zemin_kat".into(),
                long_name: Some("Zemin kat".into()),
                elevation: 0.0,
                psets: vec![],
                qtos: vec![],
            }],
            elements: vec![
                Element {
                    class: Class::Wall,
                    id: Some(7),
                    object: None,
                    name: "MM-DVR-Dis_duvar".into(),
                    description: None,
                    tag: None,
                    predefined: "STANDARD",
                    object_type: None,
                    part_of: None,
                    storey: 0,
                    shapes: vec![Shape::Prism {
                        rings: vec![vec![[0.0, 0.0], [5.0, 0.0], [5.0, 0.2], [0.0, 0.2]]],
                        bulges: Vec::new(),
                        bottom: 0.0,
                        top: 3.0,
                    }],
                    overall: None,
                    material: Some("Tugla".into()),
                    psets: vec![Pset {
                        name: "Pset_WallCommon".into(),
                        props: vec![("IsExternal".into(), Value::Bool(true))],
                    }],
                    qtos: vec![],
                },
                Element {
                    class: Class::Door,
                    id: Some(8),
                    object: None,
                    name: "MM-KAP-K1".into(),
                    description: None,
                    tag: Some("K1".into()),
                    predefined: "DOOR",
                    object_type: None,
                    part_of: None,
                    storey: 0,
                    shapes: vec![],
                    overall: Some((2.1, 0.9)),
                    material: None,
                    psets: vec![],
                    qtos: vec![],
                },
            ],
            openings: vec![Opening {
                id: None,
                name: "MM-ACL-K1".into(),
                host: 0,
                ring: vec![[1.0, 0.0], [1.9, 0.0], [1.9, 0.2], [1.0, 0.2]],
                bulges: vec![],
                bottom: 0.0,
                top: 2.1,
                filler: Some(1),
                predefined: "OPENING",
            }],
            ..Building::default()
        }
    }

    #[test]
    fn a_wall_with_a_door_is_voided_and_filled() {
        let b = door_building();
        let f = write(&b);
        // `KENTOS_IFC_BUILDING_SAMPLE=path` keeps it for an independent validator (scripts/fixtures/arch_ifc_check.py).
        if let Ok(path) = std::env::var("KENTOS_IFC_BUILDING_SAMPLE") {
            std::fs::write(path, &f).expect("the sample kept");
        }
        assert!(f.starts_with("ISO-10303-21;") && f.contains("FILE_SCHEMA(('IFC4X3_ADD2'));"));
        assert!(
            f.contains("IFCWALL(") && f.contains("IFCDOOR(") && f.contains(",2.1,0.9,.DOOR.,$,$)")
        );
        assert!(f.contains("IFCRELVOIDSELEMENT(") && f.contains("IFCRELFILLSELEMENT("));
        assert!(
            f.contains("'Pset_WallCommon'")
                && f.contains("IFCBOOLEAN(.T.)")
                && f.contains("IFCMATERIAL('Tugla'")
        );
        // The KentOS id's bits are the GlobalId.
        assert!(f.contains(&text(&guid(7))));
    }

    #[test]
    fn true_north_is_the_model_context_s_direction_and_an_unknown_one_is_not_written() {
        // Sivas on TUREF / TM36: true north 0.0111 rad (0.64°) counter-clockwise from the grid's.
        let b = Building { file: "Ev".into(), true_north: Some([-0.0111, 0.99993839]), ..Building::default() };
        let text = write(&b);
        let line = text.lines().find(|l| l.contains("IFCGEOMETRICREPRESENTATIONCONTEXT(")).expect("the context");
        let north = text.lines().find(|l| l.contains("IFCDIRECTION((-0.0111,0.99993839))")).expect("the direction");
        let id = north.split('=').next().unwrap().trim();
        assert!(line.ends_with(&format!(",{id});")), "{line}");
        // Unknown: IFC's default is up the y axis, which would say what is not known; the field stays unset.
        let b = Building { file: "Ev".into(), ..Building::default() };
        let text = write(&b);
        let line = text.lines().find(|l| l.contains("IFCGEOMETRICREPRESENTATIONCONTEXT(")).expect("the context");
        assert!(line.ends_with(",$);"), "{line}");
    }
}
