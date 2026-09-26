//! The browser's typed boundary and the watched read and write (docs/adr/0030;
//! TODOS.md FILE-15, FILE-20): the fixtures cross the columns to the bytes the
//! codec writes from the drawing itself; a read reports the project before its
//! objects and the objects as they come, and stops without a drawing when the
//! watcher says so; a write stops without bytes.

use kentos_kcad::contracts::{DocumentSnapshotV2, Entity, EntityBase, EntityId, PointEntity, Vec2};
use kentos_kcad::{Code, Columns, Quiet, Step};
use serde_json::Value;

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The valid KCAD v2 fixtures, by expected.json.
fn valid() -> Vec<String> {
    let expected: Value = serde_json::from_slice(&read("expected.json")).expect("expected.json");
    expected["files"]
        .as_array()
        .expect("files")
        .iter()
        .filter(|c| c.get("error").is_none() && c["sniff"] == "kcad")
        .map(|c| c["file"].as_str().expect("file").to_owned())
        .collect()
}

#[test]
fn every_valid_fixture_crosses_the_typed_boundary_to_the_codecs_own_bytes() {
    let files = valid();
    assert!(files.len() >= 6, "{files:?}");
    for file in files {
        let doc = kentos_kcad::decode(&read(&file)).expect("reads");
        let want = kentos_kcad::encode(&doc).expect("writes");
        let (head, cols) = kentos_kcad::split(doc).expect("splits");
        let (bytes, back) = kentos_kcad::encode_columns(&head, &cols, &mut Quiet)
            .unwrap_or_else(|e| panic!("{file}: {e}"));
        assert!(bytes == want, "{file}");
        assert_eq!(back, head, "{file}");
    }
}

/// A drawing of `n` points on the minimal fixture's one layer.
fn points(n: usize) -> DocumentSnapshotV2 {
    let mut doc = kentos_kcad::decode(&read("minimal.kcad")).expect("reads");
    doc.entities = (0..n)
        .map(|i| {
            Entity::Point(PointEntity {
                base: EntityBase {
                    id: i as u32 + 1,
                    layer_id: "0".into(),
                    color: None,
                    attrs: Default::default(),
                    label: None,
                    symbol: None,
                },
                p: Vec2 {
                    x: 500_000.0 + i as f64,
                    y: -0.0,
                },
                z: None,
            })
        })
        .collect();
    doc.uids = (0..n)
        .map(|i| {
            let mut id = [0x11u8; 16];
            id[8..].copy_from_slice(&(i as u64).to_be_bytes());
            EntityId(id)
        })
        .collect();
    doc
}

/// What a step says, the integrity check's many steps as one.
fn say(s: Step<'_>) -> String {
    match s {
        Step::Checking { .. } => "checking".to_owned(),
        Step::Project {
            name,
            layers,
            objects,
        } => format!("project {name} {layers} {objects}"),
        Step::Reading { done, total } => format!("reading {done}/{total}"),
        Step::Writing { done, total } => format!("writing {done}/{total}"),
        Step::Verifying => "verifying".to_owned(),
    }
}

#[test]
fn a_verified_write_reports_its_objects_then_reads_them_back() {
    let doc = points(5000);
    let mut seen: Vec<String> = Vec::new();
    let mut watch = |s: Step<'_>| {
        let text = say(s);
        if seen.last() != Some(&text) {
            seen.push(text);
        }
        true
    };
    let bytes = kentos_kcad::encode_verified_watched(&doc, &mut watch).expect("writes");
    assert_eq!(
        seen,
        [
            "writing 0/5000",
            "writing 2048/5000",
            "writing 4096/5000",
            "writing 5000/5000",
            "verifying",
            "checking",
            "project Boş 1 5000",
            "reading 0/5000",
            "reading 2048/5000",
            "reading 4096/5000",
            "reading 5000/5000",
        ]
    );
    assert!(bytes == kentos_kcad::encode(&doc).expect("writes"));
}

#[test]
fn a_read_reports_the_project_before_its_objects_and_stops_when_asked() {
    let bytes = kentos_kcad::encode(&points(3000)).expect("writes");
    let mut seen: Vec<String> = Vec::new();
    let mut watch = |s: Step<'_>| {
        if !matches!(s, Step::Checking { .. }) {
            seen.push(say(s));
        }
        true
    };
    kentos_kcad::decode_watched(&bytes, &mut watch).expect("reads");
    assert_eq!(
        seen,
        [
            "project Boş 1 3000",
            "reading 0/3000",
            "reading 2048/3000",
            "reading 3000/3000"
        ]
    );

    // Stopped at any step: no drawing, and the code says why.
    for stop_at in 0..6 {
        let mut n = 0;
        let mut watch = |_: Step<'_>| {
            n += 1;
            n <= stop_at
        };
        let e = kentos_kcad::decode_watched(&bytes, &mut watch).expect_err("stopped");
        assert_eq!(e.code, Code::Cancelled, "{stop_at}: {e}");
    }
}

#[test]
fn a_write_stopped_at_any_step_gives_no_bytes() {
    let doc = points(3000);
    for stop_at in 0..10 {
        let mut n = 0;
        let mut watch = |_: Step<'_>| {
            n += 1;
            n <= stop_at
        };
        let e = kentos_kcad::encode_verified_watched(&doc, &mut watch).expect_err("stopped");
        assert_eq!(e.code, Code::Cancelled, "{stop_at}");
    }
}

#[test]
fn columns_that_do_not_read_back_as_sent_are_refused_as_unverified() {
    // Two attributes whose order differs in UTF-8 (the contract's, the file's) and in UTF-16 (what a
    // page sorting JavaScript strings would send): ～ U+FF5E and 😀 U+1F600.
    let mut doc = points(1);
    if let Entity::Point(p) = &mut doc.entities[0] {
        p.base.attrs.insert("\u{ff5e}".into(), "a".into());
        p.base.attrs.insert("\u{1f600}".into(), "b".into());
    }
    let (head, mut cols) = kentos_kcad::split(doc).expect("splits");
    let mut texts: Vec<Vec<u16>> = Vec::new();
    let mut at = 0;
    for &n in &cols.text_lengths {
        texts.push(cols.text[at..at + n as usize].to_vec());
        at += n as usize;
    }
    let as_text = |t: &[Vec<u16>]| -> Vec<String> {
        t.iter()
            .map(|u| String::from_utf16(u).expect("text"))
            .collect()
    };
    assert_eq!(as_text(&texts), ["0", "\u{ff5e}", "a", "\u{1f600}", "b"]);
    // The pairs in UTF-16 order: the columns hold together and unpack, but the file holds the
    // attributes in its own order, so the bytes do not read back to what was sent and never
    // leave the codec (docs/adr/0030).
    texts.swap(1, 3);
    texts.swap(2, 4);
    cols.text = texts.concat();
    cols.text_lengths = texts.iter().map(|t| t.len() as u32).collect();
    let e = kentos_kcad::encode_columns(&head, &cols, &mut Quiet).expect_err("refused");
    assert_eq!(e.code, Code::VerifyFailed, "{e}");
    assert!(e.message.contains("nesne 1 (point)"), "{e}");
}

#[test]
fn columns_the_codec_cannot_take_are_refused_with_the_reason() {
    let (head, cols) = kentos_kcad::split(points(3)).expect("splits");
    let refuse = |head: &str, cols: &Columns| {
        kentos_kcad::encode_columns(head, cols, &mut Quiet).expect_err("refused")
    };
    assert_eq!(refuse("{\"format\":", &cols).code, Code::BadColumns);
    let mut short = cols.clone();
    short.ints.truncate(4);
    assert_eq!(refuse(&head, &short).code, Code::BadColumns);
    // A number the file cannot hold: the encoder's own refusal, with its place.
    let mut nan = cols.clone();
    nan.floats[2] = f64::NAN;
    let e = refuse(&head, &nan);
    assert_eq!(e.code, Code::NonFinite);
    assert!(e.message.contains("entities/1"), "{e}");
    // A lone UTF-16 surrogate (JavaScript strings may hold one, UTF-8 cannot): refused, not replaced.
    let mut lone = cols;
    lone.text[0] = 0xdc00;
    assert_eq!(refuse(&head, &lone).code, Code::InvalidUtf8);
}
