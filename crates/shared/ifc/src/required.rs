//! What the model does not carry of EK-6's and EK-7's mandatory rows (docs/adr/8068; plan/ifc-uyum.md IFC-02,
//! plan/bim-plan.md BIM-07): each mandatory row of the catalog (`crate::catalog`) for a class the model writes, held to
//! the objects the writer made (`crate::building::Building`): an attribute, a property of a set, a quantity, the
//! material's name. A row is carried when every object of its class has it, with a value. These are facts: whether a
//! gap is a finding, and how it is said, is the standards package's rule (bim-plan K10); EK-9's 9th question reads
//! them too ([`crate::quality`]).
//!
//! - **Objects:** the elements by their class's entity, the spaces (`IfcSpace`), the emsal zones (`IfcSpatialZone`), the
//!   units' groups (`IfcZone`), the storeys, the building, the site and the project; a class the model does not write is asked nothing (EK-6 Tablo 6.1's note: a class's fields are
//!   asked when it is in the building).
//! - **What counts:** a predefined type `NOTDEFINED` is no type (IFC-02: a roof's and a stair's type given); an empty
//!   word is no value. A relation row (`IfcRelAssignsToGroup>IfcSystem`) is not read here: it is not a field.

use crate::building::{Building, Pset, Qto, Value};
use crate::catalog::{self, Kind, Requirement};
use kentos_digital_project::Level;

/// A mandatory row some objects of its class do not carry.
#[derive(Clone, Debug, PartialEq)]
pub struct Gap {
    /// The class, as EK-6 writes it (`IfcWall`).
    pub class: String,
    /// The row's IFC path (`Pset_WallCommon.FireRating`, `PredefinedType`, `IfcMaterial.Name`).
    pub path: String,
    /// Its Turkish name (`Yangın Dayanım Sınıfı`).
    pub tr: String,
    /// Its table and page (`Tablo 6.66`, 59).
    pub table: String,
    pub page: u32,
    /// The objects without it, by their KentOS ids (none: one made of the file's seed: the project, the site …).
    pub missing: Vec<Option<u128>>,
    /// How many objects of the class the model has.
    pub of: usize,
}

/// One object as the rows read it, whatever it comes from: the model the writer makes (here) or a file read
/// (`crate::read::facts`). Its properties are (set, name, whether it holds a value), its quantities (set, name).
pub struct Carrier<'a> {
    pub id: Option<u128>,
    pub name: &'a str,
    pub long_name: Option<&'a str>,
    pub description: Option<&'a str>,
    pub object_type: Option<&'a str>,
    pub predefined: Option<&'a str>,
    pub tag: Option<&'a str>,
    pub material: Option<&'a str>,
    pub props: Vec<(&'a str, &'a str, bool)>,
    /// Its quantities: (set, name, its number when it has one).
    pub quantities: Vec<(&'a str, &'a str, Option<f64>)>,
    pub elevation: bool,
}

/// A mandatory row some objects do not carry: the row, those objects (indices into the list asked about) and how many
/// of its class there are.
pub struct Missing {
    pub requirement: &'static Requirement,
    pub objects: Vec<usize>,
    pub of: usize,
}

fn word(s: Option<&str>) -> bool {
    s.is_some_and(|s| !s.trim().is_empty())
}

fn valued(v: &Value) -> bool {
    match v {
        Value::Label(s) | Value::Text(s) | Value::Id(s) => !s.trim().is_empty(),
        Value::Labels(l) => l.iter().any(|s| !s.trim().is_empty()),
        _ => true,
    }
}

impl Carrier<'_> {
    fn carries(&self, r: &Requirement) -> Option<bool> {
        Some(match r.kind {
            Kind::Property => {
                let set = r.set.as_deref()?;
                self.props.iter().any(|(s, n, v)| *s == set && *n == r.name && *v)
            }
            Kind::Quantity => {
                let set = r.set.as_deref()?;
                self.quantities.iter().any(|(s, n, _)| *s == set && *n == r.name)
            }
            Kind::Material => word(self.material),
            Kind::Attribute => match r.name.trim_start_matches("Attribute:").trim() {
                "Name" => word(Some(self.name)),
                "LongName" => word(self.long_name),
                "Description" => word(self.description),
                "ObjectType" => word(self.object_type),
                "PredefinedType" => self.predefined.is_some_and(|p| !p.is_empty() && p != "NOTDEFINED"),
                "Tag" => word(self.tag),
                // An axis's tag (EK-6 Tablo 6.36): what its object's tag holds.
                "AxisTag" => word(self.tag),
                "RefElevation" => self.elevation,
                // A field this measure does not read (an object's place, its context's north): not judged here.
                _ => return None,
            },
            Kind::Relation => return None,
        })
    }
}

/// A model object's sets as the rows read them.
fn props(psets: &[Pset]) -> Vec<(&str, &str, bool)> {
    psets.iter().flat_map(|p| p.props.iter().map(move |(k, v)| (p.name.as_str(), k.as_str(), valued(v)))).collect()
}

fn quantities(qtos: &[Qto]) -> Vec<(&str, &str, Option<f64>)> {
    qtos.iter()
        .flat_map(|q| q.quantities.iter().map(move |(k, v)| (q.name.as_str(), k.as_str(), number(v))))
        .collect()
}

/// A measure's number.
fn number(v: &Value) -> Option<f64> {
    match v {
        Value::Length(x)
        | Value::PositiveLength(x)
        | Value::Area(x)
        | Value::Volume(x)
        | Value::Real(x)
        | Value::PositiveRatio(x) => Some(*x),
        Value::Count(n) => Some(*n as f64),
        _ => None,
    }
}

/// The building's heights asked "sadece yüksekliğin sabit olduğu durumda" (EK-6 Tablo 6.7; IFC 4.3 "only provided if
/// there is a constant height"; docs/adr/8076 §4): `Height` by the storeys' `GrossHeight`, `EavesHeight` by their
/// `NetHeight`.
const CONSTANT: [(&str, &str); 2] = [("Height", "GrossHeight"), ("EavesHeight", "NetHeight")];

/// Whether the storeys' own heights of `name` differ: then the building has no constant height, and its row is not
/// asked. Storeys without one say nothing either way.
fn not_constant(objects: &[(String, Carrier<'_>)], name: &str) -> bool {
    let known: Vec<f64> = objects
        .iter()
        .filter(|(c, _)| c.eq_ignore_ascii_case("IfcBuildingStorey"))
        .filter_map(|(_, o)| {
            o.quantities.iter().find(|(s, n, _)| *s == "Qto_BuildingStoreyBaseQuantities" && *n == name).and_then(|q| q.2)
        })
        .collect();
    known.iter().any(|h| (h - known[0]).abs() > 1e-9)
}

/// The objects of the model by their class, as EK-6 names it.
fn objects(b: &Building) -> Vec<(String, Carrier<'_>)> {
    let mut out = Vec::new();
    for e in &b.elements {
        let class = entity_class(e.class.entity());
        out.push((
            class,
            Carrier {
                id: e.id,
                name: &e.name,
                long_name: None,
                description: e.description.as_deref(),
                object_type: e.object_type.as_deref(),
                predefined: Some(e.predefined),
                tag: e.tag.as_deref(),
                material: e.material.as_deref(),
                props: props(&e.psets),
                quantities: quantities(&e.qtos),
                elevation: false,
            },
        ));
    }
    for s in &b.spaces {
        out.push((
            "IfcSpace".to_owned(),
            Carrier {
                id: s.id,
                name: &s.name,
                long_name: s.long_name.as_deref(),
                description: None,
                object_type: None,
                predefined: Some(s.predefined),
                tag: None,
                material: None,
                props: props(&s.psets),
                quantities: quantities(&s.qtos),
                elevation: false,
            },
        ));
    }
    for s in &b.storeys {
        out.push((
            "IfcBuildingStorey".to_owned(),
            Carrier {
                id: s.id,
                name: &s.name,
                long_name: s.long_name.as_deref(),
                description: None,
                object_type: None,
                predefined: None,
                tag: None,
                material: None,
                props: props(&s.psets),
                quantities: quantities(&s.qtos),
                elevation: true,
            },
        ));
    }
    for z in &b.zones {
        out.push((
            "IfcSpatialZone".to_owned(),
            Carrier {
                id: z.id,
                name: &z.name,
                long_name: z.long_name.as_deref(),
                description: z.description.as_deref(),
                object_type: z.object_type.as_deref(),
                predefined: Some(z.predefined),
                tag: None,
                material: None,
                props: props(&z.psets),
                quantities: Vec::new(),
                elevation: false,
            },
        ));
    }
    for g in &b.groups {
        out.push((
            "IfcZone".to_owned(),
            Carrier {
                id: g.id,
                name: &g.name,
                long_name: g.long_name.as_deref(),
                description: None,
                object_type: g.object_type.as_deref(),
                predefined: None,
                tag: None,
                material: None,
                props: props(&g.psets),
                quantities: Vec::new(),
                elevation: false,
            },
        ));
    }
    // A grid's axes (EK-6 Tablo 6.36): each its tag; its id its grid line's, an axis has no GlobalId of its own.
    for a in b.grids.iter().flat_map(|g| g.u.iter().chain(&g.v)) {
        out.push((
            "IfcGridAxis".to_owned(),
            Carrier {
                id: a.id,
                name: &a.tag,
                long_name: None,
                description: None,
                object_type: None,
                predefined: None,
                tag: Some(&a.tag),
                material: None,
                props: Vec::new(),
                quantities: Vec::new(),
                elevation: false,
            },
        ));
    }
    for (class, p, elevation) in [
        ("IfcProject", &b.project, false),
        ("IfcSite", &b.site, b.site_elevation.is_some()),
        ("IfcBuilding", &b.building, false),
    ] {
        out.push((
            class.to_owned(),
            Carrier {
                id: p.id,
                name: &p.name,
                long_name: p.long_name.as_deref(),
                description: p.description.as_deref(),
                object_type: p.object_type.as_deref(),
                predefined: None,
                tag: None,
                material: None,
                props: props(&p.psets),
                quantities: quantities(&p.qtos),
                elevation,
            },
        ));
    }
    out
}

/// `IFCWALL` as EK-6 writes it: `IfcWall` (the catalog's own spelling, case aside).
pub fn entity_class(entity: &str) -> String {
    let folded = entity.to_lowercase();
    catalog::base()
        .tables
        .iter()
        .find(|t| t.class.to_lowercase() == folded)
        .map_or_else(|| entity.to_owned(), |t| t.class.clone())
}

/// The mandatory rows some of `objects` (each with its class, as EK-6 names it) do not carry, in the catalog's order;
/// a class none of them is of is asked nothing.
pub fn missing(objects: &[(String, Carrier<'_>)]) -> Vec<Missing> {
    let mut out = Vec::new();
    for r in catalog::base().requirements.iter().filter(|r| r.level == Level::Mandatory) {
        let of: Vec<usize> = (0..objects.len()).filter(|&i| objects[i].0.eq_ignore_ascii_case(&r.class)).collect();
        if of.is_empty() {
            continue;
        }
        if r.kind == Kind::Quantity
            && r.class.eq_ignore_ascii_case("IfcBuilding")
            && r.set.as_deref() == Some("Qto_BuildingBaseQuantities")
            && CONSTANT.iter().any(|(row, storey)| r.name == *row && not_constant(objects, storey))
        {
            continue;
        }
        let mut lacking = Vec::new();
        let mut judged = false;
        for &i in &of {
            match objects[i].1.carries(r) {
                Some(true) => judged = true,
                Some(false) => {
                    judged = true;
                    lacking.push(i);
                }
                None => {}
            }
        }
        if judged && !lacking.is_empty() {
            out.push(Missing { requirement: r, objects: lacking, of: of.len() });
        }
    }
    out
}

/// The mandatory rows some objects of the model do not carry, in the catalog's order.
pub fn gaps(b: &Building) -> Vec<Gap> {
    let objects = objects(b);
    missing(&objects)
        .into_iter()
        .map(|m| {
            let r = m.requirement;
            Gap {
                class: r.class.clone(),
                path: r.path(),
                tr: r.tr.clone(),
                table: r.table.clone(),
                page: r.page,
                missing: m.objects.iter().map(|&i| objects[i].1.id).collect(),
                of: m.of,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::building::{Class, Element, Shape};

    fn wall(id: u128, fire: Option<&str>) -> Element {
        let mut psets = vec![Pset { name: "Pset_WallCommon".into(), props: vec![("IsExternal".into(), Value::Bool(true))] }];
        if let Some(f) = fire {
            psets[0].props.push(("FireRating".into(), Value::Label(f.into())));
        }
        Element {
            class: Class::Wall,
            id: Some(id),
            object: None,
            name: "MM-DVR-Duvar".into(),
            description: None,
            tag: None,
            predefined: "STANDARD",
            object_type: None,
            part_of: None,
            storey: 0,
            shapes: vec![Shape::Prism { rings: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 0.2], [0.0, 0.2]]], bulges: vec![], bottom: 0.0, top: 3.0 }],
            overall: None,
            material: Some("Tuğla".into()),
            psets,
            qtos: Vec::new(),
        }
    }

    #[test]
    fn a_wall_without_its_fire_rating_is_a_gap_with_its_id() {
        let mut b = Building { elements: vec![wall(1, Some("REI 120")), wall(2, None)], ..Building::default() };
        let fire = |b: &Building| gaps(b).into_iter().find(|g| g.path == "Pset_WallCommon.FireRating");
        let g = fire(&b).expect("a gap");
        assert_eq!((g.class.as_str(), g.missing.as_slice(), g.of), ("IfcWall", &[Some(2)][..], 2));
        assert_eq!(g.tr, "Yangın Dayanım Sınıfı");
        b.elements[1] = wall(2, Some("REI 60"));
        assert!(fire(&b).is_none());
        // An empty word is no value.
        b.elements[1] = wall(2, Some(" "));
        assert!(fire(&b).is_some());
    }

    #[test]
    fn a_type_not_defined_is_no_type_and_a_class_not_written_is_asked_nothing() {
        let mut stair = wall(3, Some("REI 60"));
        stair.class = Class::Stair;
        stair.predefined = "NOTDEFINED";
        stair.psets = vec![Pset { name: "Pset_StairCommon".into(), props: vec![("FireRating".into(), Value::Label("REI 60".into()))] }];
        let b = Building { elements: vec![stair], ..Building::default() };
        let gaps = gaps(&b);
        assert!(gaps.iter().any(|g| g.class == "IfcStair" && g.path == "PredefinedType"), "{gaps:?}");
        assert!(gaps.iter().any(|g| g.class == "IfcStair" && g.path == "Pset_StairCommon.FireExit"));
        assert!(!gaps.iter().any(|g| g.class == "IfcWall"), "no wall in the model");
    }

    /// A storey with its own gross and net heights (docs/adr/8076 §4).
    fn storey(gross: Option<f64>, net: Option<f64>) -> crate::building::Storey {
        let quantities: Vec<(String, Value)> = [("GrossHeight", gross), ("NetHeight", net)]
            .into_iter()
            .filter_map(|(k, v)| Some((k.to_owned(), Value::Length(v?))))
            .collect();
        crate::building::Storey {
            id: None,
            name: "MM-BNK-Kat".into(),
            long_name: None,
            elevation: 0.0,
            psets: Vec::new(),
            qtos: vec![Qto { name: "Qto_BuildingStoreyBaseQuantities".into(), quantities }],
        }
    }

    #[test]
    fn the_building_s_heights_are_asked_only_where_its_storeys_have_one_height() {
        let asked = |b: &Building, name: &str| {
            gaps(b).iter().any(|g| g.class == "IfcBuilding" && g.path == format!("Qto_BuildingBaseQuantities.{name}"))
        };
        // Storeys 3.06 and 2.90 high, nets 2.91 and 2.75 (a ground slab thicker than the floor over it): no constant
        // height, so neither row is asked (Tablo 6.7: "sadece yüksekliğin sabit olduğu durumda").
        let mut b = Building { storeys: vec![storey(Some(3.06), Some(2.91)), storey(Some(2.9), Some(2.75))], ..Building::default() };
        assert!(!asked(&b, "Height") && !asked(&b, "EavesHeight"));
        // The same height on both: asked, and missing.
        b.storeys = vec![storey(Some(3.0), Some(2.85)), storey(Some(3.0), Some(2.85))];
        assert!(asked(&b, "Height") && asked(&b, "EavesHeight"));
        // A storey whose height is not known says nothing either way: the one known asks.
        b.storeys = vec![storey(Some(3.0), None), storey(None, None)];
        assert!(asked(&b, "Height") && asked(&b, "EavesHeight"));
        // Written, nothing is missing.
        b.building.qtos = vec![Qto {
            name: "Qto_BuildingBaseQuantities".into(),
            quantities: vec![("Height".into(), Value::Length(3.0)), ("EavesHeight".into(), Value::Length(2.85))],
        }];
        assert!(!asked(&b, "Height") && !asked(&b, "EavesHeight"));
    }
}
