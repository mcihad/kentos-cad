//! The second pass: every record's class decided once per class against the file's own schema (its attribute names
//! and inheritance), the objects, relations and placements decoded, every reference checked against the records
//! there are; then the tree (parts, containment, types), the inverse index, the roots, the cycles, the GlobalIds, the
//! units and the map conversion. What cannot be decoded is a finding with its line; the rest is still read.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::Arc;

use ifc_lite_core::{AttributeValue, DecodedEntity, EntityDecoder, EntityScanner, SchemaVersion};

use super::checks::{cycles, global_ids, placement_cycles};
use super::scan::{CANCEL_EVERY, Lines, Scanned, references};
use super::units::{location, units};
use super::{Family, Finding, Header, Limits, Model, Object, ReadError, Relation, Severity, Summary, header};

/// The most findings of one kind kept with their lines; the rest are counted.
const KEEP: usize = 100;

/// What the reading face does with a relation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Role {
    Parts,
    Contains,
    Types,
    Voids,
    /// `IfcRelReferencedInSpatialStructure`: a structure that references an element or a zone without containing it.
    References,
    Other,
}

/// What a class's records are to the reading face, decided once per class.
#[derive(Debug, Clone)]
enum Kind {
    /// `outside`: not a product nor the project (a type, a group, an actor…), never in the spatial tree.
    Object { class: String, at: [Option<usize>; 8], outside: bool },
    Relation { class: String, relating: Option<usize>, related: Vec<usize>, role: Role },
    Placement { rel_to: Option<usize> },
    Project,
    MapConversion,
    Other,
    Unknown,
}

/// The object attributes the face reads, in this order: GlobalId, Name, Description, ObjectType (ElementType on a
/// type), LongName, Tag, PredefinedType, Representation.
const OBJECT_ATTRS: [&str; 8] =
    ["GlobalId", "Name", "Description", "ObjectType", "LongName", "Tag", "PredefinedType", "Representation"];

fn kind_of(schema: SchemaVersion, keyword: &str) -> Kind {
    if schema.entity_info(keyword).is_none() {
        return Kind::Unknown;
    }
    // The schema's table spells classes in capitals; the engine's type table gives the EXPRESS spelling (IfcWall).
    let class = ifc_lite_core::IfcType::from_str(keyword).name().to_owned();
    let names = schema.attribute_names(keyword).unwrap_or(&[]);
    let at = |name: &str| names.iter().position(|a| a.eq_ignore_ascii_case(name));
    let sub = |parent: &str| schema.is_subtype_of(keyword, parent);
    if keyword.eq_ignore_ascii_case("IfcProject") {
        // The project is an object too; its units are read from it after the pass.
        return object(class, &at, false).unwrap_or(Kind::Project);
    }
    if sub("IfcObjectDefinition") {
        // The spatial tree holds the project and products; types, groups, actors, processes, resources and controls
        // are assigned, not placed.
        return object(class, &at, !(sub("IfcProduct") || sub("IfcContext"))).unwrap_or(Kind::Other);
    }
    if sub("IfcRelationship") {
        let related = names.iter().enumerate().filter(|(_, a)| a.starts_with("Related")).map(|(i, _)| i).collect();
        let role = if sub("IfcRelAggregates") || sub("IfcRelNests") {
            Role::Parts
        } else if sub("IfcRelContainedInSpatialStructure") {
            Role::Contains
        } else if sub("IfcRelDefinesByType") {
            Role::Types
        } else if sub("IfcRelVoidsElement") {
            Role::Voids
        } else if sub("IfcRelReferencedInSpatialStructure") {
            Role::References
        } else {
            Role::Other
        };
        return Kind::Relation {
            class,
            relating: names.iter().position(|a| a.starts_with("Relating")),
            related,
            role,
        };
    }
    if sub("IfcObjectPlacement") {
        return Kind::Placement { rel_to: at("PlacementRelTo") };
    }
    if sub("IfcMapConversion") || keyword.eq_ignore_ascii_case("IfcMapConversion") {
        return Kind::MapConversion;
    }
    Kind::Other
}

fn object(class: String, at: &dyn Fn(&str) -> Option<usize>, outside: bool) -> Option<Kind> {
    let mut i = OBJECT_ATTRS.map(at);
    if i[3].is_none() {
        // A type object's ObjectType is its ElementType.
        i[3] = at("ElementType");
    }
    i[0]?;
    Some(Kind::Object { class, at: i, outside })
}

/// The second pass and what follows it.
#[allow(clippy::too_many_arguments)]
pub(super) fn build(
    bytes: &[u8],
    limits: &Limits,
    cancel: &dyn Fn() -> bool,
    header: Header,
    schema: String,
    family: Family,
    scanned: Scanned,
) -> Result<Model, ReadError> {
    let version = header::version(family);
    let Scanned { index, mut findings } = scanned;
    let index = Arc::new(index);
    let mut decoder = EntityDecoder::with_arc_index(bytes, index.clone());
    let mut classes_seen: HashMap<String, usize> = HashMap::new();
    let mut kinds: Vec<Kind> = Vec::new();
    let mut objects: Vec<Object> = Vec::new();
    let mut is_type: Vec<bool> = Vec::new();
    let mut relations: Vec<Relation> = Vec::new();
    let mut roles: Vec<Role> = Vec::new();
    let mut line_of: HashMap<u32, usize> = HashMap::new();
    let mut placements: HashMap<u32, u32> = HashMap::new();
    let (mut project, mut conversions) = (None, Vec::new());
    let mut unknown: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let (mut missing, mut unreadable) = (Counted::default(), Counted::default());
    let mut scanner = EntityScanner::new(bytes);
    let mut lines = Lines::new();
    let mut n = 0usize;
    while let Some((id, keyword, start, end)) = scanner.next_entity() {
        n += 1;
        if n.is_multiple_of(CANCEL_EVERY) && cancel() {
            return Err(ReadError::Cancelled);
        }
        let line = lines.at(bytes, start);
        let end = end.min(bytes.len());
        references(&bytes[start..end], |r| {
            if !index.contains_key(&r) {
                missing.add(&mut findings, || Finding {
                    code: "reference_missing",
                    severity: Severity::Error,
                    step: Some(id),
                    line: Some(line),
                    message: format!(
                        "{line}. satırdaki #{id} kaydı dosyada olmayan #{r} kaydına başvuruyor; o başvuru boş sayıldı. Dosya eksik yazılmış olabilir."
                    ),
                });
            }
        });
        let upper = keyword.to_ascii_uppercase();
        let k = match classes_seen.get(&upper) {
            Some(&k) => k,
            None => {
                kinds.push(kind_of(version, &upper));
                classes_seen.insert(upper.clone(), kinds.len() - 1);
                kinds.len() - 1
            }
        };
        if matches!(kinds[k], Kind::Other) {
            continue;
        }
        if matches!(kinds[k], Kind::Unknown) {
            unknown.entry(upper).or_insert((0, line)).0 += 1;
            continue;
        }
        let e = match decoder.decode_at_uncached(start, end) {
            Ok(e) => e,
            Err(error) => {
                unreadable.add(&mut findings, || Finding {
                    code: "record_unreadable",
                    severity: Severity::Error,
                    step: Some(id),
                    line: Some(line),
                    message: format!(
                        "{line}. satırdaki #{id} kaydı okunamadı ({error}); bu kayıt yokmuş gibi sürdürüldü. Dosyayı yazan programla yeniden dışa aktarın."
                    ),
                });
                continue;
            }
        };
        match &kinds[k] {
            Kind::Object { class, at, outside } => {
                let text = |i: Option<usize>| i.and_then(|i| e.get(i)).and_then(AttributeValue::as_string).map(str::to_owned);
                if class == "IfcProject" {
                    project = Some(id);
                }
                line_of.insert(id, line);
                is_type.push(*outside);
                objects.push(Object {
                    step: id,
                    global_id: text(at[0]).unwrap_or_default(),
                    class: class.clone(),
                    name: text(at[1]),
                    description: text(at[2]),
                    object_type: text(at[3]),
                    long_name: text(at[4]),
                    tag: text(at[5]),
                    predefined: at[6].and_then(|i| e.get(i)).and_then(AttributeValue::as_enum).map(str::to_owned),
                    parent: None,
                    container: None,
                    type_object: None,
                    children: Vec::new(),
                    has_shape: at[7].and_then(|i| e.get(i)).is_some_and(|v| v.as_entity_ref().is_some()),
                });
            }
            Kind::Relation { class, relating, related, role } => {
                let mut out = Vec::new();
                for &i in related {
                    match e.get(i) {
                        Some(v) if v.as_entity_ref().is_some() => out.extend(v.as_entity_ref()),
                        Some(v) => out.extend(v.as_list().unwrap_or(&[]).iter().filter_map(AttributeValue::as_entity_ref)),
                        None => {}
                    }
                }
                line_of.insert(id, line);
                roles.push(*role);
                relations.push(Relation {
                    step: id,
                    class: class.clone(),
                    relating: relating.and_then(|i| e.get(i)).and_then(AttributeValue::as_entity_ref),
                    related: out,
                });
            }
            Kind::Placement { rel_to } => {
                line_of.insert(id, line);
                if let Some(to) = rel_to.and_then(|i| e.get(i)).and_then(AttributeValue::as_entity_ref) {
                    placements.insert(id, to);
                }
            }
            Kind::Project => project = Some(id),
            Kind::MapConversion => conversions.push(id),
            Kind::Other | Kind::Unknown => {}
        }
    }
    missing.close(&mut findings, "reference_missing_more", "kayıt olmayan kayıtlara başvuruyor");
    unreadable.close(&mut findings, "record_unreadable_more", "kayıt okunamadı");
    for (keyword, (count, line)) in &unknown {
        findings.push(Finding {
            code: "class_unknown",
            severity: Severity::Warning,
            step: None,
            line: Some(*line),
            message: format!(
                "{keyword} sınıfı dosyanın şemasında ({schema}) yok; {count} kayıt atlandı (ilki {line}. satırda). Dosya şemasından başka bir sürümle yazılmış olabilir."
            ),
        });
    }
    let by_step: HashMap<u32, usize> = objects.iter().enumerate().map(|(i, o)| (o.step, i)).collect();
    let voiding = tree(&mut objects, &relations, &roles, &by_step, &line_of, &mut findings);
    // An element or a zone a structure references (and openings voiding a host) is in the tree through that, not a
    // root of it.
    let inverse = inverse(&relations);
    let roots = (0..objects.len())
        .filter(|&i| {
            let o = &objects[i];
            o.parent.is_none() && o.container.is_none() && !is_type[i] && !voiding.contains(&o.step)
        })
        .collect();
    cycles(&objects, &line_of, &mut findings);
    placement_cycles(&placements, limits, &line_of, &mut findings);
    global_ids(&objects, &line_of, &mut findings);
    let mut classes = BTreeMap::new();
    for o in &objects {
        *classes.entry(o.class.clone()).or_insert(0) += 1;
    }
    let units = project.map(|p| units(&mut decoder, version, p)).unwrap_or_default();
    let location = conversions.first().and_then(|&c| location(&mut decoder, version, c));
    if conversions.len() > 1 {
        findings.push(Finding {
            code: "map_conversion_many",
            severity: Severity::Warning,
            step: conversions.get(1).copied(),
            line: conversions.get(1).and_then(|c| line_of.get(c)).copied(),
            message: format!("Dosyada {} harita dönüşümü var; ilki gösterildi.", conversions.len()),
        });
    }
    Ok(Model {
        summary: Summary { schema, family, header, records: index.len(), classes, units, location },
        objects,
        relations,
        roots,
        findings,
        by_step,
        inverse,
        index,
    })
}

/// Findings of one kind: the first [`KEEP`] with their lines, then how many more.
#[derive(Default)]
pub(super) struct Counted {
    kept: usize,
    more: usize,
}

impl Counted {
    pub(super) fn add(&mut self, findings: &mut Vec<Finding>, f: impl FnOnce() -> Finding) {
        if self.kept < KEEP {
            self.kept += 1;
            findings.push(f());
        } else {
            self.more += 1;
        }
    }

    pub(super) fn close(self, findings: &mut Vec<Finding>, code: &'static str, what: &str) {
        if self.more > 0 {
            findings.push(Finding {
                code,
                severity: Severity::Error,
                step: None,
                line: None,
                message: format!("Bunlardan başka {} {what}.", self.more),
            });
        }
    }
}

/// Parts, containment and types onto the objects; returns the openings that void a host (not roots of the tree).
fn tree(
    objects: &mut [Object],
    relations: &[Relation],
    roles: &[Role],
    by_step: &HashMap<u32, usize>,
    line_of: &HashMap<u32, usize>,
    findings: &mut Vec<Finding>,
) -> HashSet<u32> {
    let mut voiding = HashSet::new();
    // Parts first, then what is contained, so a structure's children list its parts before its elements.
    for pass in [Role::Parts, Role::Contains, Role::Types, Role::Voids, Role::References] {
        for (r, role) in relations.iter().zip(roles) {
            if *role != pass {
                continue;
            }
            if matches!(pass, Role::Voids | Role::References) {
                voiding.extend(r.related.iter().copied());
                continue;
            }
            let Some(&up) = r.relating.as_ref().and_then(|s| by_step.get(s)) else { continue };
            for s in &r.related {
                let Some(&c) = by_step.get(s) else { continue };
                let slot = match pass {
                    Role::Parts => &mut objects[c].parent,
                    Role::Contains => &mut objects[c].container,
                    _ => &mut objects[c].type_object,
                };
                match *slot {
                    Some(was) if was != up && pass != Role::Types => {
                        let o = &objects[c];
                        findings.push(Finding {
                            code: if pass == Role::Parts { "parent_twice" } else { "contained_twice" },
                            severity: Severity::Warning,
                            step: Some(r.step),
                            line: line_of.get(&r.step).copied(),
                            message: format!(
                                "{} #{} ({}) iki yerde: {} kaydı onu ikinci kez bağlıyor; ilki gösterildi. Bir nesne tek bir üst yapıda bulunur.",
                                o.class,
                                o.step,
                                o.name.as_deref().unwrap_or("adsız"),
                                r.class
                            ),
                        });
                    }
                    Some(_) => {}
                    None => {
                        *slot = Some(up);
                        if pass != Role::Types {
                            objects[up].children.push(c);
                        }
                    }
                }
            }
        }
    }
    voiding
}

/// Each record's relations, on both sides.
fn inverse(relations: &[Relation]) -> HashMap<u32, Vec<usize>> {
    let mut out: HashMap<u32, Vec<usize>> = HashMap::new();
    for (i, r) in relations.iter().enumerate() {
        for s in r.relating.iter().chain(&r.related) {
            let list = out.entry(*s).or_default();
            if list.last() != Some(&i) {
                list.push(i);
            }
        }
    }
    out
}

/// An attribute of a decoded record by its name in the file's schema; none when it is unset.
pub(super) fn named<'e>(schema: SchemaVersion, e: &'e DecodedEntity, name: &str) -> Option<&'e AttributeValue> {
    let names = schema.attribute_names(e.ifc_type.name())?;
    let i = names.iter().position(|a| a.eq_ignore_ascii_case(name))?;
    e.get(i).filter(|v| !v.is_null())
}

