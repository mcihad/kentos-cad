//! The reading face against another reader (plan/ifc-uyum.md IFC-07 "yazılan her örnek geri okununca aynı nesne
//! çizgesini verir"): every sample (the files KentOS writes, two another program writes, real-world ones from
//! fixtures/ifc/v1/disaridan/), read by IfcOpenShell into fixtures/ifc/v1/okuma/ (scripts/fixtures/ifc_read_cases.py),
//! gives the same header, records, unit, map conversion, objects (class, names, predefined type, shape, parent,
//! container, type) and the same own property and quantity sets, value for value. The checks are functions so the
//! samples downloaded for a measurement go through the same (outside_tests.rs).

use std::fmt::Debug;

use serde_json::Value as Json;

use super::tests::{read_ok, samples};
use super::*;

/// IfcOpenShell's reading of each sample, in [`samples`]'s order.
const REFERENCES: [&str; 21] = [
    include_str!("../../../../../fixtures/ifc/v1/okuma/kesit-binasi.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/kesit-binasi-kunye.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/kesit-binasi-tam.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/blok-a.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/blok-b.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/cati-alti-mahal.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/egimli-doseme.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/egri-bina.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/egri-bina-referans.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/katmanli-cati.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/yerel-sistem.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/yol.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/ifcopenshell-ifc4.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/ifcopenshell-ifc4x3.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/bsi-ifc4x3-building-architecture.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/bsi-ifc4x3-infra-road.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/bsi-ifc4x3-infra-rail.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/bsi-ifc4-wall-with-opening-and-window.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/bsi-ifc4-basin-tessellation.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/bsi-ifc4-column-straight-rectangle-tessellation.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/bsi-ifc2x3-building-architecture.json"),
];

pub(super) fn reference(i: usize) -> Json {
    serde_json::from_str(REFERENCES[i]).expect("the reference reads")
}

fn text(v: &Json) -> Option<String> {
    v.as_str().map(str::to_owned)
}

fn same<T: PartialEq + Debug>(out: &mut Vec<String>, what: &str, ours: T, theirs: T) {
    if ours != theirs {
        out.push(format!("{what}: ours {ours:?}, the other reader's {theirs:?}"));
    }
}

/// Where our reading `m` differs from the other reader's `r`: the summary, then every object and its place.
pub(super) fn graph_differences(m: &Model, r: &Json) -> Vec<String> {
    let mut out = Vec::new();
    let s = &m.summary;
    same(&mut out, "schema", Some(s.schema.clone()), text(&r["schema"]));
    same(&mut out, "name", s.header.name.clone(), text(&r["header"]["name"]));
    same(&mut out, "time stamp", s.header.time_stamp.clone(), text(&r["header"]["timeStamp"]));
    same(&mut out, "preprocessor", s.header.preprocessor.clone(), text(&r["header"]["preprocessor"]));
    same(&mut out, "originating system", s.header.originating_system.clone(), text(&r["header"]["originatingSystem"]));
    same(&mut out, "records", Some(s.records as u64), r["records"].as_u64());
    same(&mut out, "length unit", s.units.length.as_ref().map(|u| u.to_si), r["lengthToSi"].as_f64());
    match (&s.location, r["location"].as_object()) {
        (None, None) => {}
        (Some(l), Some(o)) => {
            same(&mut out, "eastings", Some(l.eastings), o["eastings"].as_f64());
            same(&mut out, "northings", Some(l.northings), o["northings"].as_f64());
            same(&mut out, "orthogonal height", Some(l.orthogonal_height), o["orthogonalHeight"].as_f64());
            same(&mut out, "x axis abscissa", l.x_axis_abscissa, o["xAxisAbscissa"].as_f64());
            same(&mut out, "x axis ordinate", l.x_axis_ordinate, o["xAxisOrdinate"].as_f64());
            same(&mut out, "scale", l.scale, o["scale"].as_f64());
            same(&mut out, "system", l.crs.clone(), text(&o["crs"]));
            let factors = o.get("factors").and_then(Json::as_array).map(|f| {
                let v: Vec<f64> = f.iter().filter_map(Json::as_f64).collect();
                [v[0], v[1], v[2]]
            });
            same(&mut out, "factors", l.factors, factors);
        }
        (ours, theirs) => out.push(format!("map conversion: ours {ours:?}, the other reader's {theirs:?}")),
    }
    let Some(theirs) = r["objects"].as_object() else {
        out.push("the other reader lists no objects".to_owned());
        return out;
    };
    same(&mut out, "objects", m.objects.len(), theirs.len());
    let gid = |at: Option<usize>| at.map(|k| m.objects[k].global_id.clone());
    for o in &m.objects {
        let Some(t) = theirs.get(&o.global_id) else {
            out.push(format!("{} #{} {}: not in the other reading", o.class, o.step, o.global_id));
            continue;
        };
        let what = format!("{} #{}", o.class, o.step);
        same(&mut out, &format!("{what} step"), Some(u64::from(o.step)), t["step"].as_u64());
        same(&mut out, &format!("{what} class"), Some(o.class.clone()), text(&t["class"]));
        same(&mut out, &format!("{what} name"), o.name.clone(), text(&t["name"]));
        same(&mut out, &format!("{what} description"), o.description.clone(), text(&t["description"]));
        same(&mut out, &format!("{what} object type"), o.object_type.clone(), text(&t["objectType"]));
        same(&mut out, &format!("{what} long name"), o.long_name.clone(), text(&t["longName"]));
        same(&mut out, &format!("{what} tag"), o.tag.clone(), text(&t["tag"]));
        same(&mut out, &format!("{what} predefined"), o.predefined.clone(), text(&t["predefined"]));
        same(&mut out, &format!("{what} shape"), Some(o.has_shape), t["hasShape"].as_bool());
        same(&mut out, &format!("{what} parent"), gid(o.parent), text(&t["parent"]));
        same(&mut out, &format!("{what} container"), gid(o.container), text(&t["container"]));
        same(&mut out, &format!("{what} type"), gid(o.type_object), text(&t["type"]));
    }
    out
}

/// A value of ours as the other reader gives it in Python: a logical as a boolean, a number as a number.
fn as_json(v: &ValueKind) -> Json {
    match v {
        ValueKind::Text(s) => Json::from(s.clone()),
        ValueKind::Number { value, .. } => Json::from(*value),
        ValueKind::Integer { value, .. } => Json::from(*value),
        ValueKind::Logical(Some(b)) => Json::from(*b),
        ValueKind::Logical(None) | ValueKind::Empty => Json::Null,
        ValueKind::Enumeration(items) => Json::from(items.clone()),
        ValueKind::List(items) => Json::from(items.iter().map(as_json).collect::<Vec<_>>()),
        ValueKind::Bounded { lower, upper } => Json::from(vec![as_json(lower), as_json(upper)]),
        ValueKind::Reference(r) => Json::from(*r),
    }
}

/// Where the own sets of the objects the other reader lists sets of differ, value for value; and how many objects.
pub(super) fn set_differences(m: &Model, bytes: &[u8], r: &Json) -> (usize, Vec<String>) {
    let (mut compared, mut out) = (0, Vec::new());
    for (global_id, sets) in r["sets"].as_object().into_iter().flatten() {
        let Some(at) = m.objects.iter().position(|o| &o.global_id == global_id) else {
            out.push(format!("{global_id}: no such object"));
            continue;
        };
        let ours: serde_json::Map<String, Json> = match m.property_sets(bytes, at) {
            Ok(list) => list
                .into_iter()
                .filter(|s| !s.from_type && s.kind != SetKind::Other)
                .map(|s| {
                    let values: serde_json::Map<String, Json> =
                        s.values.iter().map(|v| (v.name.clone(), as_json(&v.value))).collect();
                    (s.name, Json::Object(values))
                })
                .collect(),
            Err(e) => {
                out.push(format!("{global_id}: {e}"));
                continue;
            }
        };
        compared += 1;
        if &Json::Object(ours.clone()) != sets {
            out.push(format!("{} {global_id}: sets {ours:?}, the other reader's {sets:?}", m.objects[at].class));
        }
    }
    (compared, out)
}

#[test]
fn every_sample_reads_as_the_other_reader_reads_it() {
    let mut differences = Vec::new();
    for (i, (name, bytes)) in samples().enumerate() {
        let m = read_ok(bytes);
        differences.extend(graph_differences(&m, &reference(i)).into_iter().map(|d| format!("{name}: {d}")));
    }
    assert!(differences.is_empty(), "{} differences:\n{}", differences.len(), differences.join("\n"));
}

#[test]
fn an_object_s_own_sets_are_the_other_reader_s_value_for_value() {
    let (mut compared, mut differences) = (0, Vec::new());
    for (i, (name, bytes)) in samples().enumerate() {
        let m = read_ok(bytes);
        let (n, d) = set_differences(&m, bytes, &reference(i));
        compared += n;
        differences.extend(d.into_iter().map(|d| format!("{name}: {d}")));
    }
    assert!(differences.is_empty(), "{}", differences.join("\n"));
    assert!(compared > 50, "{compared} objects' sets compared");
}

/// The engine's IFC 4.3 table (its classes, their parents, their attributes in order) against the published schema
/// (plan/ifc-uyum.md IFC-07 "Şema sözlüğü"): the dictionary data/ifc4x3-sozluk.json holds every class KentOS writes or
/// checks, read from IFC4X3_ADD2 (docs/adr/8062 §7). A difference is listed, never passed over.
#[test]
fn the_engine_s_schema_table_is_the_published_ifc_4_3_for_every_class_kentos_uses() {
    let schema = ifc_lite_core::SchemaVersion::Ifc4x3;
    let mut differences = Vec::new();
    let d = crate::catalog_schema::dictionary();
    assert!(d.classes.len() > 100, "the dictionary reads");
    for (class, def) in &d.classes {
        let Some(info) = schema.entity_info(class) else {
            differences.push(format!("{class}: the engine has no such class"));
            continue;
        };
        let parent = def.supertypes.first().map(String::as_str);
        if info.parent.map(str::to_ascii_uppercase) != parent.map(str::to_ascii_uppercase) {
            differences.push(format!("{class}: parent {:?}, the schema's {parent:?}", info.parent));
        }
        if info.is_abstract != def.is_abstract {
            differences.push(format!("{class}: abstract {}, the schema's {}", info.is_abstract, def.is_abstract));
        }
        let ours: Vec<&str> = schema.attribute_names(class).unwrap_or(&[]).to_vec();
        let theirs: Vec<&str> = def.attributes.iter().map(|a| a.0.as_str()).collect();
        if ours != theirs {
            differences.push(format!("{class}: attributes {ours:?}, the schema's {theirs:?}"));
        }
    }
    assert!(differences.is_empty(), "{} differences:\n{}", differences.len(), differences.join("\n"));
}
