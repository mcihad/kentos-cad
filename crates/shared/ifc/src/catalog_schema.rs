//! The rule catalog compared with IFC 4.3 (ISO 16739-1:2024, `IFC4X3_ADD2`; plan/ifc-uyum.md §3, IFC-01; docs/adr/8062
//! §5): every class, attribute, property set and property, quantity set and quantity, and predefined type the annexes
//! print, against the schema and its official property and quantity set templates (data/ifc4x3-sozluk.json, written by
//! scripts/mevzuat/ifc_sozluk.py from the published schema and templates; docs/adr/8062 §7). The schema is above a table's
//! spelling: a name it does not have is a finding, and each finding is settled by a decision (data/kararlar-2026.json)
//! or it stops the catalog's test. Nothing the schema lacks is made up.
//!
//! The national sets (`TREpys_…`, EK-6 §6.5) are not the schema's; their spellings are compared among themselves.

use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

use kentos_digital_project::decision::Decision;
use serde::Deserialize;

use crate::catalog::{Catalog, Kind};

/// A class of the dictionary: its supertypes (nearest first), its attributes in order (name, type, optional) and its
/// predefined types' enumeration.
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct ClassDef {
    pub supertypes: Vec<String>,
    #[serde(rename = "abstract")]
    pub is_abstract: bool,
    pub attributes: Vec<(String, String, bool)>,
    pub predefined: Option<String>,
}

/// A property set or quantity set template: the classes it applies to and its items (name, template kind, measure,
/// enumerated values).
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub struct SetDef {
    pub kind: String,
    pub applies: Vec<String>,
    pub items: Vec<serde_json::Value>,
}

impl SetDef {
    /// An item's name, template kind (P_SINGLEVALUE, Q_LENGTH …) and measure.
    pub fn item(&self, name: &str) -> Option<(String, Option<String>)> {
        self.items.iter().find_map(|i| {
            let a = i.as_array()?;
            (a.first()?.as_str()? == name).then(|| {
                (
                    a.get(1).and_then(|k| k.as_str()).unwrap_or("").to_owned(),
                    a.get(2).and_then(|m| m.as_str()).map(str::to_owned),
                )
            })
        })
    }

    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.items.iter().filter_map(|i| {
            i.as_array()
                .and_then(|a| a.first())
                .and_then(|n| n.as_str())
        })
    }
}

/// The IFC 4.3 dictionary.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Dictionary {
    pub format: u32,
    pub schema: String,
    pub classes: BTreeMap<String, ClassDef>,
    pub enums: BTreeMap<String, Vec<String>>,
    pub sets: BTreeMap<String, SetDef>,
    pub set_names: Vec<String>,
}

/// The dictionary's text.
pub const DICTIONARY_JSON: &str = include_str!("../data/ifc4x3-sozluk.json");

/// The dictionary, read once (a test reads it whole); unreadable, it is empty and every name a finding.
pub fn dictionary() -> &'static Dictionary {
    static D: OnceLock<Dictionary> = OnceLock::new();
    D.get_or_init(|| {
        serde_json::from_str(DICTIONARY_JSON).unwrap_or(Dictionary {
            format: 0,
            schema: String::new(),
            classes: BTreeMap::new(),
            enums: BTreeMap::new(),
            sets: BTreeMap::new(),
            set_names: Vec::new(),
        })
    })
}

/// A quantity template's kind's measure (`Q_LENGTH` → `IfcLengthMeasure`): what its value is written with.
fn quantity_measure(kind: &str) -> Option<&'static str> {
    Some(match kind {
        "Q_LENGTH" => "IfcLengthMeasure",
        "Q_AREA" => "IfcAreaMeasure",
        "Q_VOLUME" => "IfcVolumeMeasure",
        "Q_COUNT" => "IfcCountMeasure",
        "Q_WEIGHT" => "IfcMassMeasure",
        "Q_TIME" => "IfcTimeMeasure",
        _ => return None,
    })
}

/// The standard properties and quantities a building to write gives another type than IFC 4.3's template asks, or
/// that the template does not have at all (docs/adr/8074 §5, 8075: the profile's IDS asks the template's data type; an
/// independent checker found the land registration's identifiers written as labels, the form's 20th question the
/// building's `FootprintArea` where the template spells `FootPrintArea`): the set, the item, the type written and the
/// template's (empty: no such item), each once. The national sets (`TREpys_…`) are not the schema's and have no
/// template.
pub fn mistyped(b: &crate::building::Building) -> Vec<(String, String, &'static str, String)> {
    use crate::building::{Pset, Qto};
    let d = dictionary();
    let mut out = Vec::new();
    let mut psets: Vec<&Pset> = Vec::new();
    let mut qtos: Vec<&Qto> = Vec::new();
    for p in [&b.project, &b.site, &b.building] {
        psets.extend(&p.psets);
        qtos.extend(&p.qtos);
    }
    for s in &b.storeys {
        psets.extend(&s.psets);
        qtos.extend(&s.qtos);
    }
    for e in &b.elements {
        psets.extend(&e.psets);
        qtos.extend(&e.qtos);
    }
    for s in &b.spaces {
        psets.extend(&s.psets);
        qtos.extend(&s.qtos);
    }
    for z in &b.zones {
        psets.extend(&z.psets);
    }
    for g in &b.groups {
        psets.extend(&g.psets);
    }
    let sets = psets
        .iter()
        .map(|p| (&p.name, &p.props))
        .chain(qtos.iter().map(|q| (&q.name, &q.quantities)));
    for (set, items) in sets {
        let Some(def) = d.sets.get(set.as_str()) else {
            continue;
        };
        for (item, value) in items {
            let Some((kind, measure)) = def.item(item) else {
                out.push((set.clone(), item.clone(), value.ifc_type(), String::new()));
                continue;
            };
            let Some(want) = measure.or_else(|| quantity_measure(&kind).map(str::to_owned)) else {
                continue;
            };
            if !want.eq_ignore_ascii_case(value.ifc_type()) {
                out.push((set.clone(), item.clone(), value.ifc_type(), want));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

/// A difference between the catalog and the schema.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Finding {
    /// A class the schema does not have (Tablo 6.92's title spells IfcDistributionSystem otherwise).
    UnknownClass {
        class: String,
        rows: Vec<String>,
        nearest: Option<String>,
    },
    /// An attribute the class does not have (Tablo 6.37's IfcGroup.Tag).
    UnknownAttribute {
        class: String,
        name: String,
        rows: Vec<String>,
    },
    /// A property or quantity set the templates do not have.
    UnknownSet {
        set: String,
        rows: Vec<String>,
        nearest: Option<String>,
    },
    /// A property or quantity its set does not have.
    UnknownItem {
        set: String,
        name: String,
        rows: Vec<String>,
        nearest: Option<String>,
    },
    /// A set whose template does not apply to the class (nor to its type or a supertype).
    NotApplicable {
        set: String,
        class: String,
        rows: Vec<String>,
    },
    /// A predefined type printed otherwise than the enumeration spells it (“ELEMENTED WALL”, ELEMENTEDWALL).
    PredefinedSpelling {
        class: String,
        printed: String,
        schema: String,
        tables: Vec<String>,
    },
    /// A predefined type the class's enumeration does not have.
    UnknownPredefined {
        class: String,
        printed: String,
        tables: Vec<String>,
    },
    /// The table's data type (for information) is not the kind of the schema's type.
    DataType {
        row: String,
        printed: String,
        schema: String,
    },
    /// A national set spelled otherwise than its other rows spell it.
    NationalSpelling {
        set: String,
        rows: Vec<String>,
        nearest: String,
    },
}

impl Finding {
    /// A stable key a decision settles it by.
    pub fn key(&self) -> String {
        match self {
            Finding::UnknownClass { class, .. } => format!("class:{class}"),
            Finding::UnknownAttribute { class, name, .. } => format!("attribute:{class}.{name}"),
            Finding::UnknownSet { set, .. } => format!("set:{set}"),
            Finding::UnknownItem { set, name, .. } => format!("item:{set}.{name}"),
            Finding::NotApplicable { set, class, .. } => format!("applies:{set}:{class}"),
            Finding::PredefinedSpelling { class, printed, .. } => {
                format!("predefined-spelling:{class}:{printed}")
            }
            Finding::UnknownPredefined { class, printed, .. } => {
                format!("predefined:{class}:{printed}")
            }
            Finding::DataType { row, .. } => format!("datatype:{row}"),
            Finding::NationalSpelling { set, .. } => format!("national:{set}"),
        }
    }

    /// What the rules window says of it.
    pub fn message(&self) -> String {
        let near = |n: &Option<String>| {
            n.as_ref()
                .map(|n| format!("; şemadaki en yakın ad {n}"))
                .unwrap_or_default()
        };
        match self {
            Finding::UnknownClass { class, nearest, .. } => {
                format!("IFC 4.3'te {class} sınıfı yok{}", near(nearest))
            }
            Finding::UnknownAttribute { class, name, .. } => {
                format!("IFC 4.3'te {class} sınıfının {name} özniteliği yok")
            }
            Finding::UnknownSet { set, nearest, .. } => {
                format!("IFC 4.3 şablonlarında {set} seti yok{}", near(nearest))
            }
            Finding::UnknownItem {
                set, name, nearest, ..
            } => {
                format!(
                    "IFC 4.3 şablonunda {set} setinin {name} özelliği yok{}",
                    near(nearest)
                )
            }
            Finding::NotApplicable { set, class, .. } => {
                format!("{set} şablonu {class} sınıfına uygulanmıyor")
            }
            Finding::PredefinedSpelling {
                class,
                printed,
                schema,
                ..
            } => {
                format!("{class} ön tanımlı tipi ekte “{printed}”, şemada {schema}")
            }
            Finding::UnknownPredefined { class, printed, .. } => {
                format!("{class} sınıfının ön tanımlı tipleri arasında “{printed}” yok")
            }
            Finding::DataType {
                printed, schema, ..
            } => {
                format!("Ekteki veri tipi “{printed}”, şemadaki tür {schema}")
            }
            Finding::NationalSpelling { set, nearest, .. } => {
                format!("Ulusal set {set} diye yazılmış; öbür satırlar {nearest} yazıyor")
            }
        }
    }

    /// The catalog rows it is about.
    pub fn rows(&self) -> Vec<String> {
        match self {
            Finding::UnknownClass { rows, .. }
            | Finding::UnknownSet { rows, .. }
            | Finding::NationalSpelling { rows, .. }
            | Finding::UnknownAttribute { rows, .. }
            | Finding::UnknownItem { rows, .. }
            | Finding::NotApplicable { rows, .. } => rows.clone(),
            Finding::DataType { row, .. } => vec![row.clone()],
            Finding::PredefinedSpelling { tables, .. }
            | Finding::UnknownPredefined { tables, .. } => {
                tables.iter().map(|t| format!("T{t}")).collect()
            }
        }
    }

    /// Another finding of the same key: its rows joined to these.
    fn absorb(&mut self, other: Finding) {
        let more = other.rows();
        match self {
            Finding::UnknownClass { rows, .. }
            | Finding::UnknownSet { rows, .. }
            | Finding::NationalSpelling { rows, .. }
            | Finding::UnknownAttribute { rows, .. }
            | Finding::UnknownItem { rows, .. }
            | Finding::NotApplicable { rows, .. } => rows.extend(more),
            Finding::PredefinedSpelling { tables, .. }
            | Finding::UnknownPredefined { tables, .. } => tables.extend(
                more.into_iter()
                    .map(|t| t.trim_start_matches('T').to_owned()),
            ),
            Finding::DataType { .. } => {}
        }
    }
}

/// The findings, one per key in their first place, each with all its rows.
fn merged(findings: Vec<Finding>) -> Vec<Finding> {
    let mut out: Vec<Finding> = Vec::new();
    let mut at: BTreeMap<String, usize> = BTreeMap::new();
    for f in findings {
        match at.get(&f.key()) {
            Some(&i) => out[i].absorb(f),
            None => {
                at.insert(f.key(), out.len());
                out.push(f);
            }
        }
    }
    out
}

/// The edit distance of two names (letters, not bytes).
fn distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    for (i, ca) in a.iter().enumerate() {
        let mut cur = vec![i + 1; b.len() + 1];
        for (j, cb) in b.iter().enumerate() {
            cur[j + 1] = (prev[j] + usize::from(ca != cb))
                .min(prev[j + 1] + 1)
                .min(cur[j] + 1);
        }
        prev = cur;
    }
    prev[b.len()]
}

/// The name among `names` nearest to `name`, when it is near: at most three letters off and less than a quarter of it.
fn nearest<'a>(name: &str, names: impl Iterator<Item = &'a str>) -> Option<String> {
    names
        .map(|n| (distance(name, n), n))
        .filter(|(d, _)| *d > 0 && *d <= 3 && *d * 4 < name.chars().count())
        .min_by_key(|(d, n)| (*d, n.to_owned()))
        .map(|(_, n)| n.to_owned())
}

/// What kind of value a type holds: text, number, truth, or another (an entity, a list).
fn kind_of(schema_type: &str) -> &'static str {
    match schema_type {
        "IfcBoolean" | "IfcLogical" => "bool",
        "IfcLabel"
        | "IfcText"
        | "IfcIdentifier"
        | "IfcGloballyUniqueId"
        | "IfcURIReference"
        | "IfcDate"
        | "IfcDateTime"
        | "IfcTime" => "text",
        t if t.starts_with("Q_") => "number",
        t if t.ends_with("Enum") || t == "P_ENUMERATEDVALUE" => "text",
        t if t.ends_with("Measure")
            || t == "IfcInteger"
            || t == "IfcReal"
            || t == "IfcPositiveInteger" =>
        {
            "number"
        }
        _ => "other",
    }
}

fn printed_kinds(datatype: &str) -> &'static [&'static str] {
    match datatype.trim() {
        "Metin" => &["text"],
        "Sayı" => &["number"],
        "Mantıksal" => &["bool"],
        "Sayı veya Metin" => &["text", "number"],
        _ => &["text", "number", "bool"],
    }
}

/// Whether a template's applicable entity names the class, its type, or one of its supertypes.
fn applies(set: &SetDef, class: &str, def: Option<&ClassDef>) -> bool {
    set.applies.iter().any(|e| {
        let entity = e.split('/').next().unwrap_or(e).trim();
        entity == class
            || entity.strip_suffix("Type") == Some(class)
            || def.is_some_and(|d| d.supertypes.iter().any(|s| s == entity))
    })
}

/// Every difference between the catalog and the dictionary, in the catalog's order.
pub fn compare(c: &Catalog, d: &Dictionary) -> Vec<Finding> {
    let mut out = Vec::new();
    // Classes: EK-5's and the tables'.
    let mut unknown: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for x in &c.classes {
        if !d.classes.contains_key(&x.class) {
            unknown
                .entry(x.class.clone())
                .or_default()
                .push(x.id.clone());
        }
    }
    for t in &c.tables {
        if !d.classes.contains_key(&t.class) {
            unknown
                .entry(t.class.clone())
                .or_default()
                .push(t.id.clone());
        }
    }
    for (class, rows) in unknown {
        let near = nearest(&class, d.classes.keys().map(String::as_str));
        out.push(Finding::UnknownClass {
            class,
            rows,
            nearest: near,
        });
    }
    // Predefined types.
    for t in &c.tables {
        let Some(enum_name) = d.classes.get(&t.class).and_then(|x| x.predefined.as_ref()) else {
            continue;
        };
        let items = d.enums.get(enum_name).cloned().unwrap_or_default();
        for printed in &t.predefined {
            if items.iter().any(|i| i == printed) {
                continue;
            }
            // The schema joins a type's words with “_” or nothing (FLAT_ROOF, ELEMENTEDWALL, LAYBY); the tables
            // print spaces and hyphens. Letters and digits alone are compared.
            let canon = |s: &str| {
                s.chars()
                    .filter(|ch| ch.is_alphanumeric())
                    .collect::<String>()
                    .to_uppercase()
            };
            let squeezed = canon(printed);
            match items.iter().find(|i| canon(i) == squeezed) {
                Some(schema) => out.push(Finding::PredefinedSpelling {
                    class: t.class.clone(),
                    printed: printed.clone(),
                    schema: schema.clone(),
                    tables: vec![t.table.clone()],
                }),
                None => out.push(Finding::UnknownPredefined {
                    class: t.class.clone(),
                    printed: printed.clone(),
                    tables: vec![t.table.clone()],
                }),
            }
        }
    }
    // Rows.
    let mut missing_sets: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut national: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for r in &c.requirements {
        let def = d.classes.get(&r.class);
        match r.kind {
            Kind::Attribute => {
                let Some(def) = def else { continue };
                match def.attributes.iter().find(|a| a.0 == r.name) {
                    None => out.push(Finding::UnknownAttribute {
                        class: r.class.clone(),
                        name: r.name.clone(),
                        rows: vec![r.id.clone()],
                    }),
                    Some(a) => {
                        let k = kind_of(&a.1);
                        if k != "other" && !printed_kinds(&r.datatype).contains(&k) {
                            out.push(Finding::DataType {
                                row: r.id.clone(),
                                printed: r.datatype.clone(),
                                schema: a.1.clone(),
                            });
                        }
                    }
                }
            }
            Kind::Property | Kind::Quantity => {
                let Some(set) = r.set.as_deref() else {
                    continue;
                };
                if set.starts_with("TREpys_") {
                    national
                        .entry(set.to_owned())
                        .or_default()
                        .push(r.id.clone());
                    continue;
                }
                let Some(sd) = d.sets.get(set) else {
                    missing_sets
                        .entry(set.to_owned())
                        .or_default()
                        .push(r.id.clone());
                    continue;
                };
                match sd.item(&r.name) {
                    None => out.push(Finding::UnknownItem {
                        set: set.to_owned(),
                        name: r.name.clone(),
                        rows: vec![r.id.clone()],
                        nearest: nearest(&r.name, sd.names()),
                    }),
                    Some((template, measure)) => {
                        let ty = if template == "P_ENUMERATEDVALUE" || template.starts_with("Q_") {
                            template
                        } else {
                            measure.unwrap_or_default()
                        };
                        let k = kind_of(&ty);
                        if k != "other" && !printed_kinds(&r.datatype).contains(&k) {
                            out.push(Finding::DataType {
                                row: r.id.clone(),
                                printed: r.datatype.clone(),
                                schema: ty,
                            });
                        }
                    }
                }
                if !applies(sd, &r.class, def) && d.classes.contains_key(&r.class) {
                    out.push(Finding::NotApplicable {
                        set: set.to_owned(),
                        class: r.class.clone(),
                        rows: vec![r.id.clone()],
                    });
                }
            }
            Kind::Material | Kind::Relation => {
                let parts: Vec<&str> = r.path.as_deref().unwrap_or("").split('>').collect();
                for part in &parts {
                    if part.starts_with("Ifc") && !d.classes.contains_key(*part) {
                        out.push(Finding::UnknownClass {
                            class: (*part).to_owned(),
                            rows: vec![r.id.clone()],
                            nearest: nearest(part, d.classes.keys().map(String::as_str)),
                        });
                    }
                }
                // A material's attribute is its path's last class's (IfcMaterial.Name, IfcMaterialLayer.LayerThickness).
                if r.kind == Kind::Material
                    && let Some(last) = parts.last().and_then(|p| d.classes.get(*p))
                    && !last.attributes.iter().any(|a| a.0 == r.name)
                {
                    out.push(Finding::UnknownAttribute {
                        class: parts.last().copied().unwrap_or_default().to_owned(),
                        name: r.name.clone(),
                        rows: vec![r.id.clone()],
                    });
                }
            }
        }
    }
    for (set, rows) in missing_sets {
        let near = nearest(&set, d.set_names.iter().map(String::as_str));
        out.push(Finding::UnknownSet {
            set,
            rows,
            nearest: near,
        });
    }
    // A national set used by few rows and near a more used one is a misspelling of it.
    let counts: Vec<(String, usize)> = national.iter().map(|(s, r)| (s.clone(), r.len())).collect();
    for (set, rows) in &national {
        let more = counts
            .iter()
            .filter(|(o, n)| o != set && *n > rows.len() && distance(o, set) <= 2)
            .min_by_key(|(o, _)| distance(o, set));
        if let Some((near, _)) = more {
            out.push(Finding::NationalSpelling {
                set: set.clone(),
                rows: rows.clone(),
                nearest: near.clone(),
            });
        }
    }
    merged(out)
}

/// The findings no decision settles: what a new or changed catalog row brought.
pub fn unsettled<'a>(findings: &'a [Finding], decisions: &[Decision]) -> Vec<&'a Finding> {
    findings
        .iter()
        .filter(|f| {
            let key = f.key();
            !decisions.iter().any(|d| d.settles_key(&key))
        })
        .collect()
}

/// The decisions' keys no finding has any more: a decision left after its finding went.
pub fn stale(findings: &[Finding], decisions: &[Decision]) -> Vec<String> {
    let keys: BTreeSet<String> = findings.iter().map(Finding::key).collect();
    decisions
        .iter()
        .flat_map(|d| d.settles.iter())
        .filter(|s| match s.strip_suffix('*') {
            Some(prefix) => !keys.iter().any(|k| k.starts_with(prefix)),
            None => !keys.contains(*s),
        })
        .cloned()
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::catalog::{self, Requirement};

    #[test]
    fn the_dictionary_reads_whole() {
        let d: Dictionary = serde_json::from_str(DICTIONARY_JSON).expect("the dictionary reads");
        assert_eq!(d.schema, "IFC4X3_ADD2");
        assert!(
            d.classes["IfcWall"]
                .attributes
                .iter()
                .any(|a| a.0 == "PredefinedType")
        );
        assert!(d.sets["Pset_WallCommon"].item("FireRating").is_some());
    }

    #[test]
    fn every_finding_is_settled_and_every_decision_still_finds_its_finding() {
        let findings = compare(catalog::base(), dictionary());
        let open = unsettled(&findings, catalog::decisions());
        assert!(
            open.is_empty(),
            "findings no decision settles:\n{}",
            open.iter()
                .map(|f| format!("{}  {}", f.key(), f.message()))
                .collect::<Vec<_>>()
                .join("\n")
        );
        assert_eq!(stale(&findings, catalog::decisions()), Vec::<String>::new());
    }

    #[test]
    fn a_misspelled_property_is_found() {
        let mut c = catalog::base().clone();
        let fire: &mut Requirement = c
            .requirements
            .iter_mut()
            .find(|r| r.id == "T6.66/Pset_WallCommon.FireRating")
            .expect("the wall's fire rating");
        fire.name = "FireRatng".into();
        let findings = compare(&c, dictionary());
        let open = unsettled(&findings, catalog::decisions());
        assert_eq!(open.len(), 1, "{open:?}");
        assert_eq!(open[0].key(), "item:Pset_WallCommon.FireRatng");
        assert_eq!(
            open[0].message(),
            "IFC 4.3 şablonunda Pset_WallCommon setinin FireRatng özelliği yok; şemadaki en yakın ad FireRating"
        );
    }

    #[test]
    fn the_source_s_own_misspellings_are_found() {
        let keys: Vec<String> = compare(catalog::base(), dictionary())
            .iter()
            .map(Finding::key)
            .collect();
        for k in [
            "item:Pset_CoolingTowerTypeCommon.NumberofCells",
            "class:IfcDistrubitionSystem",
            "attribute:IfcGroup.Tag",
            "national:TREpys_PozOzellikScti",
            "predefined-spelling:IfcWall:ELEMENTED WALL",
        ] {
            assert!(keys.iter().any(|x| x == k), "{k} not found");
        }
        assert_eq!(
            nearest(
                "Qto_BuildingBaseQuantity",
                ["Qto_BuildingBaseQuantities", "Qto_Other"].into_iter()
            )
            .as_deref(),
            Some("Qto_BuildingBaseQuantities")
        );
    }

    /// A label where the template asks an identifier is found, and an item the template does not have; the
    /// identifier and a national set are not.
    #[test]
    fn a_property_of_another_type_than_its_template_s_is_found() {
        use crate::building::{Building, Place, Pset, Value};
        let site = |id: Value| Place {
            psets: vec![
                Pset {
                    name: "Pset_LandRegistration".into(),
                    props: vec![("LandID".into(), id)],
                },
                Pset {
                    name: "TREpys_ParselOzellikSeti".into(),
                    props: vec![("TAKS".into(), Value::Label("0.4".into()))],
                },
                Pset {
                    name: "Pset_LandRegistration".into(),
                    props: vec![("LandTitleID".into(), Value::Id("12/345".into()))],
                },
            ],
            ..Place::default()
        };
        let b = Building {
            site: site(Value::Label("1234/5".into())),
            ..Building::default()
        };
        assert_eq!(
            mistyped(&b),
            [(
                "Pset_LandRegistration".to_owned(),
                "LandID".to_owned(),
                "IfcLabel",
                "IfcIdentifier".to_owned()
            )]
        );
        let b = Building {
            site: site(Value::Id("1234/5".into())),
            ..Building::default()
        };
        assert_eq!(mistyped(&b), Vec::new());
        let mut b = b;
        b.building.qtos.push(crate::building::Qto {
            name: "Qto_BuildingBaseQuantities".into(),
            quantities: vec![
                ("FootprintArea".into(), Value::Area(120.0)),
                ("FootPrintArea".into(), Value::Area(120.0)),
            ],
        });
        assert_eq!(
            mistyped(&b),
            [(
                "Qto_BuildingBaseQuantities".to_owned(),
                "FootprintArea".to_owned(),
                "IfcAreaMeasure",
                String::new()
            )]
        );
    }
}
