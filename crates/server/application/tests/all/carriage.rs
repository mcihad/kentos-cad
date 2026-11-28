//! Every object of the project file's samples crosses the stored feature and comes back as it was
//! (plan/cad.md `CAD-DATA-22`, docs/adr/0265): each valid sample of `fixtures/kcad/v2` (`expected.json`'s
//! drawings written by hand) holds the kinds and fields the schemas added one by one (elevations, parts,
//! blocks, look, NURBS, references along an edge and inside a block …); each object is stored as the
//! server stores it (`to_stored`: the PostGIS geometry or the CAD definition) and read back (`from_stored`),
//! and must equal the object (its id is the browser's, 0; a negative zero is zero). No database is needed.

use kentos_application::cad::{from_stored, to_stored, to_stored_block};
use kentos_contracts::{DocumentSnapshotV2, Entity};
use kentos_formats::blocks::Placing;
use serde_json::Value;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

/// The object as the server keeps it: no id, no negative zero.
fn kept(e: &Entity) -> Value {
    let mut v = serde_json::to_value(e).expect("an object");
    fn zero(v: &mut Value) {
        match v {
            Value::Number(n) if n.as_f64() == Some(0.0) => *v = Value::from(0.0),
            Value::Array(a) => a.iter_mut().for_each(zero),
            Value::Object(m) => m.values_mut().for_each(zero),
            _ => {}
        }
    }
    zero(&mut v);
    v["id"] = Value::from(0);
    v
}

#[test]
fn every_sample_object_crosses_the_stored_feature_and_back() {
    let expected: Value = serde_json::from_str(
        &std::fs::read_to_string(format!("{DIR}expected.json")).expect("expected.json"),
    )
    .expect("JSON");
    let (mut objects, mut files, mut kinds) = (0, 0, std::collections::BTreeSet::new());
    let (mut failures, mut refused) = (Vec::new(), 0);
    for case in expected["files"].as_array().expect("files") {
        let Some(content) = case["content"].as_str() else {
            continue;
        };
        let text = std::fs::read_to_string(format!("{DIR}{content}")).expect("the content");
        let doc: DocumentSnapshotV2 = serde_json::from_str(&text).expect("a drawing");
        let srid = doc.settings.srid;
        let blocks = Placing::new(&doc.blocks);
        files += 1;
        for b in &doc.blocks {
            match to_stored_block(b) {
                Ok((back, _))
                    if serde_json::to_value(&back).ok() == serde_json::to_value(b).ok() => {}
                Ok(_) => failures.push(format!("{content} › blok {}: değişti", b.name)),
                Err(why) => failures.push(format!("{content} › blok {}: {why}", b.name)),
            }
        }
        for e in &doc.entities {
            objects += 1;
            let kind = serde_json::to_value(e).expect("JSON")["kind"]
                .as_str()
                .unwrap_or("")
                .to_string();
            kinds.insert(kind.clone());
            let back = to_stored(e, srid, &blocks).and_then(|s| from_stored(&s));
            // A coordinate past ±10⁹ m (the codec's own f64::MAX sample) is garbage to the server: refused, not mangled.
            if content == "drawing.json" && back.as_ref().is_err_and(|w| w.contains("çok büyük"))
            {
                refused += 1;
                continue;
            }
            match back {
                Ok(b) if kept(&b) == kept(e) => {}
                Ok(b) => failures.push(format!("{content} › {kind}: geri gelen {}", kept(&b))),
                Err(why) => failures.push(format!("{content} › {kind}: {why}")),
            }
        }
    }
    assert!(
        failures.is_empty(),
        "{} nesne taşınamadı:\n{}",
        failures.len(),
        failures.join("\n")
    );
    assert_eq!(refused, 1, "the one garbage sample is refused");
    assert!(
        files >= 50 && objects >= 200,
        "{files} dosya, {objects} nesne"
    );
    assert!(kinds.len() >= 20, "{kinds:?}");
}
