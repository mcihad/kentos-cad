//! A written file's facts for the model quality control form (EK-9 Tablo 9.1; plan/ifc-uyum.md IFC-08,
//! docs/adr/8074 §2): what the file holds, measured from its bytes through the reading face, as JSON
//! (`kentos.ifcFacts` 1). Whether something is a finding is the standards package's rule (`dijital_proje.form`,
//! bim-plan K10); here nothing is judged.
//!
//! - **file:** its name, extension, size, header and the schema it declares;
//! - **units, location:** the project's units and the map conversion as written;
//! - **objects:** every object definition: GlobalId, record, class, attributes, its place in the tree (parent,
//!   container, type and the storey above it, by GlobalId), whether it has a shape, its materials, its own property and
//!   quantity sets and its type's, a storey's elevation;
//! - **records:** EK-7's resource records (persons, organisations, roles, addresses, the projected system, the map
//!   conversion, the representation contexts) with their attributes by the schema's names;
//! - **rows:** EK-7's catalog rows (project, persons, organisations, roles, system), for the project information;
//! - **gaps:** the catalog's mandatory rows some objects of the file do not carry (`crate::required`), by GlobalId;
//! - **classCodes:** EK-5 Tablo 5.1's category code of each class the file holds;
//! - **findings:** the reading face's findings.
//!
//! The bodies' measures (closed, heights, the spaces' footprints and overlaps) are `facts_bodies.rs`'s, with the
//! `geometry` feature. The host adds what only it knows: the objects it expected and what its export left out.

use std::collections::{BTreeMap, BTreeSet};

use ifc_lite_core::{AttributeValue, EntityDecoder, EntityScanner, SchemaVersion};
use serde_json::{Map, Value as Json, json};

use super::graph::named;
use super::{Model, PropertySet, ReadError, SetKind, Unit, ValueKind, guarded, header};
use crate::catalog::{self, Kind};
use crate::required::{self, Carrier};

/// The facts' format and version.
pub const FORMAT: &str = "kentos.ifcFacts";
pub const VERSION: u32 = 1;

/// EK-7's resource classes (Tablo 7.2–7.8) and the address their records point to: records, not objects.
/// EK-7's resource records by their STEP entity, and the class each is listed under: a scaled map conversion (a local
/// system's, docs/adr/8072 §3) is a map conversion, its own class in its record.
const RECORDS: [(&str, &str); 9] = [
    ("IFCPERSON", "IfcPerson"),
    ("IFCPERSONANDORGANIZATION", "IfcPersonAndOrganization"),
    ("IFCORGANIZATION", "IfcOrganization"),
    ("IFCACTORROLE", "IfcActorRole"),
    ("IFCPOSTALADDRESS", "IfcPostalAddress"),
    ("IFCPROJECTEDCRS", "IfcProjectedCRS"),
    ("IFCMAPCONVERSION", "IfcMapConversion"),
    ("IFCMAPCONVERSIONSCALED", "IfcMapConversion"),
    ("IFCGEOMETRICREPRESENTATIONCONTEXT", "IfcGeometricRepresentationContext"),
];

/// The annex rows the form cites (docs/adr/8074 §3) besides EK-9's questions, by their catalog ids: EK-1's project code,
/// EK-2's file name, EK-3's room numbers, EK-5's classes, version and names, EK-7's project and system.
const SOURCES: [&str; 9] = ["M1.1.1", "T2.1", "T3.1", "M5.1", "M5.4", "M5.5", "T7.1", "T7.6", "T7.7"];

impl Model {
    /// The facts of the file read from `bytes`, its name `name` (with its extension).
    pub fn facts(&self, bytes: &[u8], name: &str) -> Result<Json, ReadError> {
        let schema = header::version(self.summary.family);
        let sets: Vec<Vec<PropertySet>> =
            (0..self.objects.len()).map(|i| self.property_sets(bytes, i)).collect::<Result<_, _>>()?;
        let materials: Vec<Vec<String>> =
            (0..self.objects.len()).map(|i| self.materials(bytes, i)).collect::<Result<_, _>>()?;
        guarded(|| {
            let mut decoder = EntityDecoder::with_arc_index(bytes, self.index.clone());
            let elevations = self.elevations(&mut decoder, schema);
            let objects: Vec<Json> =
                (0..self.objects.len()).map(|i| self.object_json(i, &sets[i], &materials[i], &elevations)).collect();
            Ok(json!({
                "format": FORMAT,
                "version": VERSION,
                "file": self.file_json(bytes, name),
                "units": {
                    "length": self.summary.units.length.as_ref().map(written),
                    "area": self.summary.units.area.as_ref().map(written),
                    "volume": self.summary.units.volume.as_ref().map(written),
                    "planeAngle": self.summary.units.plane_angle.as_ref().map(written),
                },
                "location": self.summary.location.as_ref().map(|l| json!({
                    "crs": l.crs,
                    "crsDescription": l.crs_description,
                    "geodeticDatum": l.geodetic_datum,
                    "verticalDatum": l.vertical_datum,
                    "mapProjection": l.map_projection,
                    "mapZone": l.map_zone,
                    "eastings": l.eastings,
                    "northings": l.northings,
                    "orthogonalHeight": l.orthogonal_height,
                    "xAxisAbscissa": l.x_axis_abscissa,
                    "xAxisOrdinate": l.x_axis_ordinate,
                    "scale": l.scale,
                })),
                "objects": objects,
                "records": records(bytes, &mut decoder, schema),
                "rows": project_rows(),
                "gaps": self.gaps(&sets, &materials, &elevations),
                "setChecks": self.set_checks(&sets),
                "profile": { "schema": crate::catalog_schema::dictionary().schema },
                // The rule catalog the gaps are read against (a result names it to be made again).
                "catalog": { "id": catalog::base().profile.id, "version": catalog::base().profile.version },
                "classCodes": self.class_codes(),
                "sources": sources(),
                "findings": self.findings.iter().map(|f| json!({
                    "code": f.code,
                    "severity": match f.severity { super::Severity::Error => "error", super::Severity::Warning => "warning" },
                    "step": f.step,
                    "line": f.line,
                    "message": f.message,
                })).collect::<Vec<_>>(),
            }))
        })
    }

    fn file_json(&self, bytes: &[u8], name: &str) -> Json {
        let h = &self.summary.header;
        let extension = std::path::Path::new(name).extension().map(|e| e.to_string_lossy().to_lowercase());
        json!({
            "name": name,
            "extension": extension,
            "bytes": bytes.len(),
            "schema": self.summary.schema,
            "records": self.summary.records,
            "header": {
                "name": h.name,
                "timeStamp": h.time_stamp,
                "originatingSystem": h.originating_system,
                "preprocessor": h.preprocessor,
                "description": h.description,
                "implementationLevel": h.implementation_level,
                "author": h.author,
                "organization": h.organization,
            },
        })
    }

    /// The storeys' and the site's elevations (`Elevation`, `RefElevation`), by object index.
    fn elevations(&self, decoder: &mut EntityDecoder, schema: SchemaVersion) -> BTreeMap<usize, f64> {
        let mut out = BTreeMap::new();
        for (i, o) in self.objects.iter().enumerate() {
            let attribute = if schema.is_subtype_of(&o.class, "IfcBuildingStorey") {
                "Elevation"
            } else if schema.is_subtype_of(&o.class, "IfcSite") {
                "RefElevation"
            } else {
                continue;
            };
            if let Ok(e) = decoder.decode_by_id(o.step)
                && let Some(v) = named(schema, &e, attribute).and_then(AttributeValue::as_float)
            {
                out.insert(i, v);
            }
        }
        out
    }

    /// The storey above object `i`: the nearest up its parts and containers.
    fn storey_above(&self, i: usize) -> Option<usize> {
        let schema = header::version(self.summary.family);
        let mut at = self.objects.get(i).and_then(|o| o.parent.or(o.container));
        for _ in 0..self.objects.len() {
            let p = at?;
            let o = self.objects.get(p)?;
            if schema.is_subtype_of(&o.class, "IfcBuildingStorey") {
                return Some(p);
            }
            at = o.parent.or(o.container);
        }
        None
    }

    fn object_json(&self, i: usize, sets: &[PropertySet], materials: &[String], elevations: &BTreeMap<usize, f64>) -> Json {
        let o = &self.objects[i];
        let gid = |j: Option<usize>| j.and_then(|j| self.objects.get(j)).map(|x| x.global_id.clone());
        let group = |own: bool, kind: SetKind| -> Map<String, Json> {
            sets.iter()
                .filter(|s| s.from_type != own && s.kind == kind)
                .map(|s| (s.name.clone(), Json::Object(s.values.iter().map(|v| (v.name.clone(), value_json(&v.value))).collect())))
                .collect()
        };
        json!({
            "globalId": o.global_id,
            "step": o.step,
            "class": o.class,
            "name": o.name,
            "description": o.description,
            "objectType": o.object_type,
            "longName": o.long_name,
            "tag": o.tag,
            "predefinedType": o.predefined,
            "parent": gid(o.parent),
            "container": gid(o.container),
            "type": gid(o.type_object),
            "storey": gid(self.storey_above(i)),
            "hasShape": o.has_shape,
            "materials": materials,
            "sets": group(true, SetKind::Properties),
            "quantities": group(true, SetKind::Quantities),
            "typeSets": group(false, SetKind::Properties),
            "typeQuantities": group(false, SetKind::Quantities),
            "elevation": elevations.get(&i),
        })
    }

    /// The catalog's mandatory rows some objects of the file do not carry, their objects by GlobalId.
    fn gaps(&self, sets: &[Vec<PropertySet>], materials: &[Vec<String>], elevations: &BTreeMap<usize, f64>) -> Vec<Json> {
        let carriers: Vec<(String, Carrier<'_>)> = self
            .objects
            .iter()
            .enumerate()
            .map(|(i, o)| {
                let props = sets[i]
                    .iter()
                    .filter(|s| s.kind == SetKind::Properties)
                    .flat_map(|s| s.values.iter().map(move |v| (s.name.as_str(), v.name.as_str(), valued(&v.value))))
                    .collect();
                let quantities = sets[i]
                    .iter()
                    .filter(|s| s.kind == SetKind::Quantities)
                    .flat_map(|s| {
                        s.values.iter().map(move |v| {
                            let n = match v.value {
                                ValueKind::Number { value, .. } => Some(value),
                                ValueKind::Integer { value, .. } => Some(value as f64),
                                _ => None,
                            };
                            (s.name.as_str(), v.name.as_str(), n)
                        })
                    })
                    .collect();
                let carrier = Carrier {
                    id: None,
                    name: o.name.as_deref().unwrap_or(""),
                    long_name: o.long_name.as_deref(),
                    description: o.description.as_deref(),
                    object_type: o.object_type.as_deref(),
                    predefined: o.predefined.as_deref(),
                    tag: o.tag.as_deref(),
                    material: materials[i].first().map(String::as_str),
                    props,
                    quantities,
                    elevation: elevations.contains_key(&i),
                    // A file read says no reason to leave a row out.
                    unentered: Vec::new(),
                };
                (required::entity_class(&o.class), carrier)
            })
            .collect();
        required::missing(&carriers)
            .into_iter()
            .map(|m| {
                let r = m.requirement;
                json!({
                    "id": r.id,
                    "class": r.class,
                    "path": r.path(),
                    "tr": r.tr,
                    "table": r.table,
                    "page": r.page,
                    "missing": m.objects.iter().map(|&i| self.objects[i].global_id.clone()).collect::<Vec<_>>(),
                    "of": m.of,
                })
            })
            .collect()
    }

    /// The standard sets (`Pset_…`, `Qto_…`) the IFC 4.3 dictionary does not have, those given a class they are not for,
    /// and the items they do not define, each with its object; a file of another schema is not compared (its sets are
    /// another dictionary's).
    fn set_checks(&self, sets: &[Vec<PropertySet>]) -> Vec<Json> {
        if self.summary.family != super::Family::Ifc4x3 {
            return Vec::new();
        }
        let d = crate::catalog_schema::dictionary();
        let schema = header::version(self.summary.family);
        let mut out = Vec::new();
        for (i, o) in self.objects.iter().enumerate() {
            for s in &sets[i] {
                if !(s.name.starts_with("Pset_") || s.name.starts_with("Qto_")) {
                    continue;
                }
                let class = match (s.from_type, o.type_object) {
                    (true, Some(t)) => self.objects[t].class.as_str(),
                    _ => o.class.as_str(),
                };
                let mut say = |why: &str, item: Option<&str>| {
                    out.push(json!({ "globalId": o.global_id, "set": s.name, "item": item, "fromType": s.from_type, "class": class, "why": why }));
                };
                match d.sets.get(&s.name) {
                    None => say("unknown_set", None),
                    Some(def) => {
                        if !def.applies.is_empty() && !def.applies.iter().any(|a| schema.is_subtype_of(class, a)) {
                            say("not_for_class", None);
                        }
                        for v in &s.values {
                            if def.item(&v.name).is_none() {
                                say("unknown_item", Some(&v.name));
                            }
                        }
                    }
                }
            }
        }
        out
    }

    /// EK-5 Tablo 5.1's category code of each class the file holds.
    fn class_codes(&self) -> BTreeMap<String, String> {
        let held: BTreeSet<String> = self.objects.iter().map(|o| o.class.to_lowercase()).collect();
        catalog::base()
            .classes
            .iter()
            .filter(|c| held.contains(&c.class.to_lowercase()))
            .map(|c| (c.class.clone(), c.code.clone()))
            .collect()
    }
}

/// A unit as written: its prefix, then its name.
fn written(u: &Unit) -> String {
    format!("{}{}", u.prefix.as_deref().unwrap_or(""), u.name)
}

/// Whether a value counts as given: an empty word is none.
fn valued(v: &ValueKind) -> bool {
    match v {
        ValueKind::Text(s) => !s.trim().is_empty(),
        ValueKind::Enumeration(l) => l.iter().any(|s| !s.trim().is_empty()),
        ValueKind::List(l) => !l.is_empty(),
        ValueKind::Empty => false,
        _ => true,
    }
}

/// A property's value as JSON: a text, a number, a logical (null: unknown), a list, a bounded value's ends, a record.
fn value_json(v: &ValueKind) -> Json {
    match v {
        ValueKind::Text(s) => json!(s),
        ValueKind::Number { value, .. } => json!(value),
        ValueKind::Integer { value, .. } => json!(value),
        ValueKind::Logical(b) => json!(b),
        ValueKind::Enumeration(items) => json!(items),
        ValueKind::List(items) => Json::Array(items.iter().map(value_json).collect()),
        ValueKind::Bounded { lower, upper } => json!({ "lower": value_json(lower), "upper": value_json(upper) }),
        ValueKind::Reference(r) => json!(format!("#{r}")),
        ValueKind::Empty => Json::Null,
    }
}

/// An attribute's value as JSON: a record it names as `#n`, an enumeration's word without its dots.
fn attribute_json(v: &AttributeValue) -> Json {
    match v {
        AttributeValue::String(s) | AttributeValue::Binary(s) => json!(s),
        AttributeValue::Integer(i) => json!(i),
        AttributeValue::Float(f) => json!(f),
        AttributeValue::Enum(e) => json!(e.trim_matches('.')),
        AttributeValue::EntityRef(r) => json!(format!("#{r}")),
        AttributeValue::List(l) => Json::Array(l.iter().map(attribute_json).collect()),
        AttributeValue::Typed { value, .. } => attribute_json(value),
        AttributeValue::Null | AttributeValue::Derived => Json::Null,
    }
}

/// EK-7's resource records by class, each its record and its attributes by the schema's names.
fn records(bytes: &[u8], decoder: &mut EntityDecoder, schema: SchemaVersion) -> BTreeMap<String, Vec<Json>> {
    let mut out: BTreeMap<String, Vec<Json>> = BTreeMap::new();
    let mut scanner = EntityScanner::new(bytes);
    while let Some((id, keyword, _, _)) = scanner.next_entity() {
        let upper = keyword.to_ascii_uppercase();
        let Some((_, listed)) = RECORDS.iter().find(|(k, _)| *k == upper) else {
            continue;
        };
        let Ok(e) = decoder.decode_by_id(id) else { continue };
        let class = e.ifc_type.name().to_owned();
        let names = schema.attribute_names(&class).unwrap_or_default();
        let attributes: Map<String, Json> =
            names.iter().enumerate().map(|(k, n)| ((*n).to_owned(), e.get(k).map_or(Json::Null, attribute_json))).collect();
        out.entry((*listed).to_owned()).or_default().push(json!({ "step": id, "class": class, "attributes": attributes }));
    }
    out
}

/// The cites of [`SOURCES`] and of EK-9 Tablo 9.1's questions, as the rule catalog says them (“EK-7 Tablo 7.6 · PDF s.
/// 227–228”): from the IFC annexes' catalog, else the 2D annexes'.
fn sources() -> Map<String, Json> {
    let ids = SOURCES.iter().map(|s| (*s).to_owned()).chain((1..=21).map(|n| format!("T9.1/{n}")));
    ids.filter_map(|id| {
        let s = catalog::base()
            .row(&id)
            .map(|r| r.source())
            .or_else(|| kentos_digital_project::catalog::base().row(&id).map(|r| r.source()))?;
        Some((id, json!(s.cite())))
    })
    .collect()
}

/// EK-7's catalog rows (Tablo 7.1–7.8): the project's information and system, mandatory or not.
fn project_rows() -> Vec<Json> {
    catalog::base()
        .requirements
        .iter()
        .filter(|r| r.table.starts_with("7."))
        .map(|r| {
            json!({
                "id": r.id,
                "table": r.table,
                "page": r.page,
                "class": r.class,
                "name": r.name,
                "path": r.path(),
                "tr": r.tr,
                "mandatory": r.level == kentos_digital_project::Level::Mandatory,
                "attribute": r.kind == Kind::Attribute,
            })
        })
        .collect()
}
