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

/// One object as the rows read it.
struct Object<'a> {
    id: Option<u128>,
    name: &'a str,
    long_name: Option<&'a str>,
    description: Option<&'a str>,
    object_type: Option<&'a str>,
    predefined: Option<&'a str>,
    tag: Option<&'a str>,
    material: Option<&'a str>,
    psets: &'a [Pset],
    qtos: &'a [Qto],
    elevation: bool,
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

impl Object<'_> {
    fn carries(&self, r: &Requirement) -> Option<bool> {
        Some(match r.kind {
            Kind::Property => {
                let set = r.set.as_deref()?;
                self.psets.iter().any(|p| p.name == set && p.props.iter().any(|(k, v)| *k == r.name && valued(v)))
            }
            Kind::Quantity => {
                let set = r.set.as_deref()?;
                self.qtos.iter().any(|q| q.name == set && q.quantities.iter().any(|(k, _)| *k == r.name))
            }
            Kind::Material => word(self.material),
            Kind::Attribute => match r.name.trim_start_matches("Attribute:").trim() {
                "Name" => word(Some(self.name)),
                "LongName" => word(self.long_name),
                "Description" => word(self.description),
                "ObjectType" => word(self.object_type),
                "PredefinedType" => self.predefined.is_some_and(|p| !p.is_empty() && p != "NOTDEFINED"),
                "Tag" => word(self.tag),
                "RefElevation" => self.elevation,
                // A field this measure does not read (an object's place, its context's north): not judged here.
                _ => return None,
            },
            Kind::Relation => return None,
        })
    }
}

/// The objects of the model by their class, as EK-6 names it.
fn objects(b: &Building) -> Vec<(String, Object<'_>)> {
    let mut out = Vec::new();
    for e in &b.elements {
        let class = entity_class(e.class.entity());
        out.push((
            class,
            Object {
                id: e.id,
                name: &e.name,
                long_name: None,
                description: e.description.as_deref(),
                object_type: None,
                predefined: Some(e.predefined),
                tag: e.tag.as_deref(),
                material: e.material.as_deref(),
                psets: &e.psets,
                qtos: &e.qtos,
                elevation: false,
            },
        ));
    }
    for s in &b.spaces {
        out.push((
            "IfcSpace".to_owned(),
            Object {
                id: s.id,
                name: &s.name,
                long_name: s.long_name.as_deref(),
                description: None,
                object_type: None,
                predefined: Some(s.predefined),
                tag: None,
                material: None,
                psets: &s.psets,
                qtos: &s.qtos,
                elevation: false,
            },
        ));
    }
    for s in &b.storeys {
        out.push((
            "IfcBuildingStorey".to_owned(),
            Object {
                id: s.id,
                name: &s.name,
                long_name: s.long_name.as_deref(),
                description: None,
                object_type: None,
                predefined: None,
                tag: None,
                material: None,
                psets: &s.psets,
                qtos: &s.qtos,
                elevation: true,
            },
        ));
    }
    for z in &b.zones {
        out.push((
            "IfcSpatialZone".to_owned(),
            Object {
                id: z.id,
                name: &z.name,
                long_name: z.long_name.as_deref(),
                description: z.description.as_deref(),
                object_type: z.object_type.as_deref(),
                predefined: Some(z.predefined),
                tag: None,
                material: None,
                psets: &z.psets,
                qtos: &[],
                elevation: false,
            },
        ));
    }
    for g in &b.groups {
        out.push((
            "IfcZone".to_owned(),
            Object {
                id: g.id,
                name: &g.name,
                long_name: g.long_name.as_deref(),
                description: None,
                object_type: g.object_type.as_deref(),
                predefined: None,
                tag: None,
                material: None,
                psets: &g.psets,
                qtos: &[],
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
            Object {
                id: p.id,
                name: &p.name,
                long_name: p.long_name.as_deref(),
                description: p.description.as_deref(),
                object_type: p.object_type.as_deref(),
                predefined: None,
                tag: None,
                material: None,
                psets: &p.psets,
                qtos: &p.qtos,
                elevation,
            },
        ));
    }
    out
}

/// `IFCWALL` as EK-6 writes it: `IfcWall` (the catalog's own spelling, case aside).
fn entity_class(entity: &str) -> String {
    let folded = entity.to_lowercase();
    catalog::base()
        .tables
        .iter()
        .find(|t| t.class.to_lowercase() == folded)
        .map_or_else(|| entity.to_owned(), |t| t.class.clone())
}

/// The mandatory rows some objects of the model do not carry, in the catalog's order.
pub fn gaps(b: &Building) -> Vec<Gap> {
    let objects = objects(b);
    let mut out = Vec::new();
    for r in catalog::base().requirements.iter().filter(|r| r.level == Level::Mandatory) {
        let of: Vec<&Object<'_>> = objects.iter().filter(|(c, _)| c.eq_ignore_ascii_case(&r.class)).map(|(_, o)| o).collect();
        if of.is_empty() {
            continue;
        }
        let mut missing = Vec::new();
        let mut judged = false;
        for o in &of {
            match o.carries(r) {
                Some(true) => judged = true,
                Some(false) => {
                    judged = true;
                    missing.push(o.id);
                }
                None => {}
            }
        }
        if judged && !missing.is_empty() {
            out.push(Gap {
                class: r.class.clone(),
                path: r.path(),
                tr: r.tr.clone(),
                table: r.table.clone(),
                page: r.page,
                missing,
                of: of.len(),
            });
        }
    }
    out
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
            name: "MM-DVR-Duvar".into(),
            description: None,
            tag: None,
            predefined: "STANDARD",
            storey: 0,
            shapes: vec![Shape::Prism { rings: vec![vec![[0.0, 0.0], [1.0, 0.0], [1.0, 0.2], [0.0, 0.2]]], bottom: 0.0, top: 3.0 }],
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
}
