//! What the elevations are and the coordinates' epoch in the project file (docs/adr/0519 §1, docs/specs/kcad-v2.md
//! §6.1): document schema 86 is written only when a project names one, every other drawing keeps its schema and its
//! bytes; they come back as written; the writer refuses what the reader would.

use kentos_kcad::Code;
use kentos_kcad::contracts::{DocumentSnapshotV2, GeoidModel, HeightKind, HeightReference};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says (a byte 0x18 and the value).
fn schema(file: &[u8]) -> u8 {
    let payload = &file[36..file.len() - 32];
    let key = b"\x67version";
    let at = payload
        .windows(key.len())
        .position(|w| w == key)
        .expect("a version key")
        + key.len();
    match payload[at] {
        0x18 => payload[at + 1],
        small => small,
    }
}

#[test]
fn a_project_with_heights_is_schema_86_and_one_without_stays_as_it_was() {
    let doc = content("heights.json");
    let written = kentos_kcad::encode(&doc).expect("writes");
    assert_eq!(schema(&written), 86);
    assert!(written == read("heights.kcad"), "the reference's bytes");
    let back = kentos_kcad::decode(&written).expect("reads");
    assert_eq!(back.settings.heights, doc.settings.heights);
    assert_eq!(back.settings.coordinate_epoch, Some(2023.5));
    // Without them: the external references' schema 83 of the same drawing.
    let mut plain = doc.clone();
    plain.settings.heights = None;
    plain.settings.coordinate_epoch = None;
    assert_eq!(schema(&kentos_kcad::encode(&plain).expect("writes")), 83);
}

#[test]
fn the_writer_refuses_what_the_reader_refuses() {
    let refused = |change: &dyn Fn(&mut DocumentSnapshotV2)| {
        let mut doc = content("heights.json");
        change(&mut doc);
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    let e = refused(&|d| d.settings.heights.as_mut().expect("heights").kind = HeightKind::Unknown);
    assert_eq!(e.code, Code::BadValue);
    let e = refused(&|d| d.settings.heights.as_mut().expect("heights").vertical_srid = Some(5703));
    assert!(e.message.contains("EPSG:5703"), "{e}");
    let e = refused(&|d| d.settings.coordinate_epoch = Some(1850.0));
    assert!(e.message.contains("1900"), "{e}");
    let e = refused(&|d| {
        d.settings.srid = 0;
        d.settings.coordinate_epoch = None;
        d.settings.heights = Some(HeightReference {
            kind: HeightKind::Ellipsoidal,
            vertical_srid: None,
            geoid: None,
        });
    });
    assert!(e.message.contains("koordinat sistemi"), "{e}");
    let e = refused(&|d| {
        d.settings.heights = Some(HeightReference {
            kind: HeightKind::Orthometric,
            vertical_srid: None,
            geoid: Some(GeoidModel {
                name: "TG-20".into(),
                sha256: Some("ABC".into()),
            }),
        })
    });
    assert!(e.message.contains("SHA-256"), "{e}");
    // An orthometric height of a local project is kept.
    let mut doc = content("heights.json");
    doc.settings.srid = 0;
    doc.settings.coordinate_epoch = None;
    let back = kentos_kcad::decode(&kentos_kcad::encode(&doc).expect("writes")).expect("reads");
    assert_eq!(back.settings.heights, doc.settings.heights);
}
