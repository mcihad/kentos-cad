//! The survey settings' a priori standard deviations in the project file
//! (docs/adr/0203 §1, docs/specs/kcad-v2.md §6.1, §6.4.2): document schema
//! 28 is written only when a project names one, every other drawing keeps
//! its schema and its bytes; they come back as written; the writer refuses
//! what the reader would.

use kentos_kcad::Code;
use kentos_kcad::contracts::{DocumentSnapshotV2, SurveySettings};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says (from 24 on, a byte 0x18 and the value).
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
fn a_project_with_sigmas_is_schema_28_and_one_without_stays_as_it_was() {
    let doc = content("survey-sigmas.json");
    let written = kentos_kcad::encode(&doc).expect("writes");
    assert_eq!(schema(&written), 28);
    assert!(
        written == read("survey-sigmas.kcad"),
        "the reference's bytes"
    );
    let back = kentos_kcad::decode(&written).expect("reads");
    assert_eq!(back.settings.survey, doc.settings.survey);
    // The same project without them: the ground's schema 16, its bytes.
    let plain = content("survey-ground.json");
    let written = kentos_kcad::encode(&plain).expect("writes");
    assert_eq!(schema(&written), 16);
    assert!(written == read("survey-ground.kcad"));
}

#[test]
fn the_writer_refuses_what_the_reader_refuses() {
    let refused = |s: SurveySettings| {
        let mut doc = content("minimal.json");
        doc.settings.survey = Some(s);
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    let e = refused(SurveySettings {
        sigma_distance: Some(0.0),
        ..Default::default()
    });
    assert_eq!(e.code, Code::BadValue);
    assert!(
        e.message
            .contains("kenarın sabit payının önsel doğruluğu 0"),
        "{e}"
    );
    let e = refused(SurveySettings {
        sigma_ppm: Some(-1.0),
        ..Default::default()
    });
    assert!(
        e.message
            .contains("kenarın ppm payı -1; sıfırdan küçük olamaz"),
        "{e}"
    );
    // Nothing per million and no centering are parts a project may name.
    let mut doc = content("minimal.json");
    doc.settings.survey = Some(SurveySettings {
        sigma_ppm: Some(0.0),
        sigma_centering: Some(0.0),
        ..Default::default()
    });
    let written = kentos_kcad::encode(&doc).expect("writes");
    assert_eq!(schema(&written), 28);
}
