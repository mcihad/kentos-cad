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
    Area(f64),
    Volume(f64),
    Ratio(f64),
    Count(i64),
    Real(f64),
    /// Several short names (`IfcPropertyListValue` of `IfcLabel`s: Pset_Address's AddressLines).
    Labels(Vec<String>),
}

impl Value {
    fn step(&self) -> String {
        match self {
            Value::Bool(b) => format!("IFCBOOLEAN(.{}.)", if *b { "T" } else { "F" }),
            Value::Label(s) => format!("IFCLABEL({})", text(s)),
            Value::Text(s) => format!("IFCTEXT({})", text(s)),
            Value::Id(s) => format!("IFCIDENTIFIER({})", text(s)),
            Value::Length(v) => format!("IFCLENGTHMEASURE({})", real(*v)),
            Value::Area(v) => format!("IFCAREAMEASURE({})", real(*v)),
            Value::Volume(v) => format!("IFCVOLUMEMEASURE({})", real(*v)),
            Value::Ratio(v) => format!("IFCRATIOMEASURE({})", real(*v)),
            Value::Count(v) => format!("IFCCOUNTMEASURE({v})"),
            Value::Real(v) => format!("IFCREAL({})", real(*v)),
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
        bottom: f64,
        top: f64,
    },
    Faces {
        vertices: Vec<[f64; 3]>,
        faces: Vec<Vec<usize>>,
    },
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
        }
    }
}

/// An element of the building.
#[derive(Clone, Debug, PartialEq)]
pub struct Element {
    pub class: Class,
    /// Its KentOS id (a UUID's 128 bits): its GlobalId. None: one made of the file's seed.
    pub id: Option<u128>,
    /// Its name (EK-5 §5.5: discipline, category code, description).
    pub name: String,
    pub description: Option<String>,
    /// Its tag (EK-6: an instance's mark: K1, P3, its KentOS id).
    pub tag: Option<String>,
    /// Its predefined type, as STEP spells the enumeration's item (`STANDARD`, `FLOOR`, `DOOR` …).
    pub predefined: &'static str,
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
    pub bottom: f64,
    pub top: f64,
    /// INTERNAL, EXTERNAL, PARKING …
    pub predefined: &'static str,
    pub psets: Vec<Pset>,
    pub qtos: Vec<Qto>,
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
    pub crs: Option<Crs>,
}

/// Which of `rings` are outer rings and which holes (even-odd: a ring inside an odd number of others is a hole), each
/// hole with the nearest outer ring holding it: (outer, holes).
fn nest(rings: &[Vec<[f64; 2]>]) -> Vec<(usize, Vec<usize>)> {
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

    /// A closed polyline of a ring (its first point again at its end).
    fn curve(&mut self, ring: &[[f64; 2]]) -> usize {
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

    /// A shape's items: one extrusion an outer ring (with its holes), or a face set.
    fn items(&mut self, shape: &Shape) -> Vec<usize> {
        match shape {
            Shape::Prism { rings, bottom, top } => {
                let rings: Vec<&Vec<[f64; 2]>> = rings.iter().filter(|r| r.len() >= 3).collect();
                let owned: Vec<Vec<[f64; 2]>> = rings.iter().map(|r| (*r).clone()).collect();
                let mut out = Vec::new();
                for (outer, holes) in nest(&owned) {
                    let o = self.curve(&owned[outer]);
                    let profile = if holes.is_empty() {
                        self.st
                            .add(format!("IFCARBITRARYCLOSEDPROFILEDEF(.AREA.,$,#{o})"))
                    } else {
                        let inner: Vec<usize> =
                            holes.iter().map(|&h| self.curve(&owned[h])).collect();
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
            "IFCSHAPEREPRESENTATION(#{},'Body','{kind}',({}))",
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
            "IFCPOSTALADDRESS($,$,$,$,{lines},$,{},{},{},{})",
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
    let units = st.add(format!(
        "IFCUNITASSIGNMENT((#{length},#{area},#{volume},#{angle}))"
    ));
    let origin = st.add("IFCCARTESIANPOINT((0.,0.,0.))");
    let axis_up = st.add("IFCDIRECTION((0.,0.,1.))");
    let axis_x = st.add("IFCDIRECTION((1.,0.,0.))");
    let world = st.add(format!(
        "IFCAXIS2PLACEMENT3D(#{origin},#{axis_up},#{axis_x})"
    ));
    let model = st.add(format!(
        "IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,#{world},$)"
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
    // Those who act on it (EK-7): person, organisation, role, assigned to the project.
    for a in &b.actors {
        let person = w.st.add(format!(
            "IFCPERSON({},{},{},$,$,$,$,$)",
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
                    "IFCORGANIZATION({},{},$,$,{addresses})",
                    Writer::opt(&o.identification),
                    text(&o.name)
                ));
                w.st.add(format!("IFCPERSONANDORGANIZATION(#{person},#{org},$)"))
            }
            None => person,
        };
        let role = w.st.add(format!(
            "IFCACTORROLE(.{}.,$,{})",
            a.role,
            Writer::opt(&a.role_description)
        ));
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
    let building_place = w.placement(Some(site_place), 0.0);
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
    for e in &b.elements {
        let Some(&parent) = storey_places.get(e.storey) else {
            products.push(0);
            continue;
        };
        let place = w.placement(Some(parent), 0.0);
        let shape = w.body(&e.shapes).map_or("$".into(), |s| format!("#{s}"));
        let g = w.id(e.id);
        let head = format!(
            "{g},$,{},{},$,#{place},{shape},{}",
            text(&e.name),
            Writer::opt(&e.description),
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
        if let Some(m) = &e.material {
            match materials.iter_mut().find(|(name, _)| name == m) {
                Some((_, list)) => list.push(n),
                None => materials.push((m.clone(), vec![n])),
            }
        }
        contained[e.storey].push(n);
        products.push(n);
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
        let shape = w
            .body(&[Shape::Prism {
                rings: vec![o.ring.clone()],
                bottom: o.bottom,
                top: o.top,
            }])
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
    for s in &b.spaces {
        let Some(&parent) = storey_places.get(s.storey) else {
            continue;
        };
        let place = w.placement(Some(parent), 0.0);
        let shape = w
            .body(&[Shape::Prism {
                rings: s.rings.clone(),
                bottom: s.bottom,
                top: s.top,
            }])
            .map_or("$".into(), |x| format!("#{x}"));
        let g = w.id(s.id);
        let n = w.st.add(format!(
            "IFCSPACE({g},$,{},$,$,#{place},{shape},{},.ELEMENT.,.{}.,$)",
            text(&s.name),
            Writer::opt(&s.long_name),
            s.predefined
        ));
        w.psets(n, &s.psets, &s.qtos);
        // A space is part of its storey (IfcRelAggregates), not contained in it.
        let g = w.st.gid();
        w.st.add(format!(
            "IFCRELAGGREGATES({g},$,$,$,#{},(#{n}))",
            storeys[s.storey]
        ));
    }
    for (i, list) in contained.iter().enumerate().filter(|(_, l)| !l.is_empty()) {
        let g = w.st.gid();
        w.st.add(format!(
            "IFCRELCONTAINEDINSPATIALSTRUCTURE({g},$,$,$,({}),#{})",
            Step::refs(list),
            storeys[i]
        ));
    }
    for (name, list) in &materials {
        let m = w.st.add(format!("IFCMATERIAL({},$,$)", text(name)));
        let g = w.st.gid();
        w.st.add(format!(
            "IFCRELASSOCIATESMATERIAL({g},$,$,$,({}),#{m})",
            Step::refs(list)
        ));
    }
    // Design Transfer, not Reference View: an opening's body is subtracted from its wall's by the reader (IFC 4.3,
    // IfcRelVoidsElement: a 'Body' opening is a Boolean difference); a Reference View holds the walls already cut and
    // its openings as 'Reference' only (docs/adr/8047).
    w.st.file(
        "ViewDefinition [DesignTransferView]",
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

    #[test]
    fn a_wall_with_a_door_is_voided_and_filled() {
        let b = Building {
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
                    name: "MM-DVR-Dis_duvar".into(),
                    description: None,
                    tag: None,
                    predefined: "STANDARD",
                    storey: 0,
                    shapes: vec![Shape::Prism {
                        rings: vec![vec![[0.0, 0.0], [5.0, 0.0], [5.0, 0.2], [0.0, 0.2]]],
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
                    name: "MM-KAP-K1".into(),
                    description: None,
                    tag: Some("K1".into()),
                    predefined: "DOOR",
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
                bottom: 0.0,
                top: 2.1,
                filler: Some(1),
                predefined: "OPENING",
            }],
            ..Building::default()
        };
        let f = write(&b);
        // `KENTOS_IFC_BUILDING_SAMPLE=path` keeps it for IfcOpenShell's validator.
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
}
