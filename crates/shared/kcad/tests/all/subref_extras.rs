//! A reference along an edge and inside a block in the project file (docs/adr/0263 §4): document schema 51 is
//! written only when a dimension's reference has one; it comes back as written; the reader's errors name their
//! places; the typed columns carry both.

use kentos_kcad::Code;
use kentos_kcad::contracts::{DocumentSnapshotV2, Entity, SubPlace};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says.
fn schema(file: &[u8]) -> u8 {
    let payload = &file[36..file.len() - 32];
    let key = b"\x67version";
    let at = payload
        .windows(key.len())
        .position(|w| w == key)
        .expect("a version key")
        + key.len();
    if payload[at] == 0x18 {
        payload[at + 1]
    } else {
        payload[at]
    }
}

fn refs(doc: &mut DocumentSnapshotV2) -> impl Iterator<Item = &mut kentos_kcad::contracts::SubRef> {
    doc.entities.iter_mut().flat_map(|e| match e {
        Entity::Dimension(d) => d
            .refs
            .as_mut()
            .map(|r| [r.a.as_mut(), r.b.as_mut(), r.c.as_mut()])
            .into_iter()
            .flatten()
            .flatten()
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    })
}

#[test]
fn the_sample_reads_and_is_schema_51() {
    let data = read("subref-extras.kcad");
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert_eq!(doc, content("subref-extras.json"));
    assert_eq!(schema(&data), 51);
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    let mut doc = doc;
    let all: Vec<_> = refs(&mut doc).map(|r| (r.place, r.path.clone())).collect();
    assert!(
        all.iter()
            .any(|(p, _)| matches!(p, SubPlace::Along { t, .. } if *t == 0.25))
    );
    assert!(all.iter().any(|(_, path)| path == &vec![1, 0]));
}

#[test]
fn without_along_or_a_path_the_file_is_older_and_keeps_its_bytes() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let mut doc = content("subref-extras.json");
    for r in refs(&mut doc) {
        r.path.clear();
        if let SubPlace::Along {
            part, ring, index, ..
        } = r.place
        {
            r.place = SubPlace::Mid { part, ring, index };
        }
    }
    assert!(of(&doc) < 51);
    let mut doc = content("subref-extras.json");
    for r in refs(&mut doc) {
        if let SubPlace::Along {
            part, ring, index, ..
        } = r.place
        {
            r.place = SubPlace::Mid { part, ring, index };
        }
    }
    assert_eq!(of(&doc), 51, "a path alone is schema 51");
}

#[test]
fn the_typed_columns_carry_along_and_paths() {
    let doc = content("subref-extras.json");
    let columns = kentos_kcad::columns::pack(doc.entities.clone(), doc.uids.clone());
    let (back, uids) = kentos_kcad::columns::unpack(&columns).expect("back");
    assert_eq!(back, doc.entities);
    assert_eq!(uids, doc.uids);
}

#[test]
fn the_readers_errors_name_their_places() {
    for (file, code, place) in [
        (
            "broken/dimension-refs-along-in-schema-50.kcad",
            Code::UnknownField,
            "place/t",
        ),
        (
            "broken/dimension-refs-path-in-schema-50.kcad",
            Code::UnknownField,
            "path",
        ),
        (
            "broken/dimension-refs-along-t-above-one.kcad",
            Code::BadValue,
            "t'si",
        ),
        (
            "broken/dimension-refs-path-empty.kcad",
            Code::BadValue,
            "blok yolu",
        ),
        (
            "broken/dimension-refs-path-deep.kcad",
            Code::BadValue,
            "Blok yolu 17",
        ),
        ("broken/schema-version-87.kcad", Code::SchemaVersion, ""),
    ] {
        let err = kentos_kcad::decode(&read(file)).expect_err(file);
        assert_eq!(err.code, code, "{file}: {err}");
        assert!(
            err.message.contains(place) || err.to_string().contains(place),
            "{file}: {err}"
        );
    }
}
