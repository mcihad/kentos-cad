//! The reading face against another reader (plan/ifc-uyum.md IFC-07 "yazılan her örnek geri okununca aynı nesne
//! çizgesini verir"): every file KentOS writes, read by IfcOpenShell into fixtures/ifc/v1/okuma/
//! (scripts/fixtures/ifc_read_cases.py), gives the same header, records, unit, map conversion, objects (class, names,
//! predefined type, shape, parent, container, type) and the same own property and quantity sets, value for value.

use serde_json::Value as Json;

use super::tests::{read_ok, samples};
use super::*;

/// IfcOpenShell's reading of each sample, in [`samples`]'s order.
const REFERENCES: [&str; 13] = [
    include_str!("../../../../../fixtures/ifc/v1/okuma/kesit-binasi.json"),
    include_str!("../../../../../fixtures/ifc/v1/okuma/kesit-binasi-kunye.json"),
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
];

pub(super) fn reference(i: usize) -> Json {
    serde_json::from_str(REFERENCES[i]).expect("the reference reads")
}

fn text(v: &Json) -> Option<String> {
    v.as_str().map(str::to_owned)
}

#[test]
fn the_header_records_unit_and_map_conversion_are_the_other_reader_s() {
    for (i, (name, bytes)) in samples().enumerate() {
        let r = reference(i);
        let m = read_ok(bytes);
        let s = &m.summary;
        assert_eq!(Some(s.schema.as_str()), r["schema"].as_str(), "{name}");
        assert_eq!(s.header.name, text(&r["header"]["name"]), "{name}");
        assert_eq!(s.header.time_stamp, text(&r["header"]["timeStamp"]), "{name}");
        assert_eq!(s.header.preprocessor, text(&r["header"]["preprocessor"]), "{name}");
        assert_eq!(s.header.originating_system, text(&r["header"]["originatingSystem"]), "{name}");
        assert_eq!(Some(s.records as u64), r["records"].as_u64(), "{name}");
        assert_eq!(s.units.length.as_ref().map(|u| u.to_si), r["lengthToSi"].as_f64(), "{name}");
        match (&s.location, r["location"].as_object()) {
            (None, None) => {}
            (Some(l), Some(o)) => {
                assert_eq!(Some(l.eastings), o["eastings"].as_f64(), "{name}");
                assert_eq!(Some(l.northings), o["northings"].as_f64(), "{name}");
                assert_eq!(Some(l.orthogonal_height), o["orthogonalHeight"].as_f64(), "{name}");
                assert_eq!(l.x_axis_abscissa, o["xAxisAbscissa"].as_f64(), "{name}");
                assert_eq!(l.x_axis_ordinate, o["xAxisOrdinate"].as_f64(), "{name}");
                assert_eq!(l.scale, o["scale"].as_f64(), "{name}");
                assert_eq!(l.crs, text(&o["crs"]), "{name}");
                let factors = o.get("factors").and_then(Json::as_array).map(|f| {
                    let v: Vec<f64> = f.iter().filter_map(Json::as_f64).collect();
                    [v[0], v[1], v[2]]
                });
                assert_eq!(l.factors, factors, "{name}");
            }
            (ours, theirs) => panic!("{name}: map conversion {ours:?} against {theirs:?}"),
        }
    }
}

#[test]
fn every_object_and_its_place_in_the_tree_are_the_other_reader_s() {
    for (i, (name, bytes)) in samples().enumerate() {
        let r = reference(i);
        let theirs = r["objects"].as_object().expect("objects");
        let m = read_ok(bytes);
        assert_eq!(m.objects.len(), theirs.len(), "{name}: as many objects");
        let gid = |at: Option<usize>| at.map(|k| m.objects[k].global_id.clone());
        for o in &m.objects {
            let t = theirs.get(&o.global_id).unwrap_or_else(|| panic!("{name}: {} not in the other reading", o.global_id));
            let what = format!("{name}: {} #{}", o.class, o.step);
            assert_eq!(Some(u64::from(o.step)), t["step"].as_u64(), "{what}");
            assert_eq!(Some(o.class.as_str()), t["class"].as_str(), "{what}");
            assert_eq!(o.name, text(&t["name"]), "{what}");
            assert_eq!(o.description, text(&t["description"]), "{what}");
            assert_eq!(o.object_type, text(&t["objectType"]), "{what}");
            assert_eq!(o.long_name, text(&t["longName"]), "{what}");
            assert_eq!(o.tag, text(&t["tag"]), "{what}");
            assert_eq!(o.predefined, text(&t["predefined"]), "{what}");
            assert_eq!(Some(o.has_shape), t["hasShape"].as_bool(), "{what}");
            assert_eq!(gid(o.parent), text(&t["parent"]), "{what}: parent");
            assert_eq!(gid(o.container), text(&t["container"]), "{what}: container");
            assert_eq!(gid(o.type_object), text(&t["type"]), "{what}: type");
        }
    }
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

#[test]
fn an_object_s_own_sets_are_the_other_reader_s_value_for_value() {
    let mut compared = 0;
    for (i, (name, bytes)) in samples().enumerate() {
        let r = reference(i);
        let m = read_ok(bytes);
        for (global_id, sets) in r["sets"].as_object().expect("sets") {
            let at = m.objects.iter().position(|o| &o.global_id == global_id).expect("the object");
            let ours: serde_json::Map<String, Json> = m
                .property_sets(bytes, at)
                .expect("its sets")
                .into_iter()
                .filter(|s| !s.from_type && s.kind != SetKind::Other)
                .map(|s| {
                    let values: serde_json::Map<String, Json> =
                        s.values.iter().map(|v| (v.name.clone(), as_json(&v.value))).collect();
                    (s.name, Json::Object(values))
                })
                .collect();
            assert_eq!(&Json::Object(ours), sets, "{name}: {} {global_id}", m.objects[at].class);
            compared += 1;
        }
    }
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
