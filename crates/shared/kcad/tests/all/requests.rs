//! The request ledger in the project file (docs/adr/0253 §6,
//! docs/specs/kcad-v2.md §6.13): document schema 45 is written only for a
//! drawing that executed requests; every other drawing keeps its schema and
//! its bytes. The ledger comes back as it was; the writer refuses what the
//! reader would; the reader's errors name their places.

use kentos_kcad::Code;
use kentos_kcad::contracts::{CallStatus, DocumentSnapshotV2, RequestRecord};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says (one byte below 24, else the byte after 0x18).
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

fn request(id: &str) -> RequestRecord {
    RequestRecord {
        request_id: id.into(),
        fingerprint: "0123456789abcdef0123456789abcdef".into(),
        command: "cad.entities.delete".into(),
        version: 1,
        revision: "7".into(),
        status: CallStatus::Done,
        answer: Some("{\"status\":\"completed\"}".into()),
    }
}

fn with(requests: Vec<RequestRecord>) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    doc.requests = requests;
    doc
}

#[test]
fn only_a_drawing_with_requests_is_schema_45() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("requests.kcad");
    assert_eq!(schema(&data), 45);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(doc.requests.len(), 2);
    assert_eq!(doc.requests[1].status, CallStatus::Undone);
    assert_eq!(doc.requests[1].answer, None);
    assert_eq!(of(&content("minimal.json")), 2);
    assert_eq!(of(&with(vec![request("a")])), 45);
    // The newest drawings before it keep their schemas.
    assert_eq!(of(&content("sheets.json")), 44);
    assert_eq!(of(&content("dimension-refs.json")), 43);
}

#[test]
fn the_ledger_comes_back_as_it_was() {
    let mut many: Vec<RequestRecord> = (0..256).map(|i| request(&format!("r:{i}"))).collect();
    many[3].status = CallStatus::Undone;
    many[4].answer = None;
    many[5].answer = Some("ş".repeat(32 * 1024));
    let doc = with(many);
    let data = kentos_kcad::encode_verified(&doc).expect("writes");
    let back = kentos_kcad::decode(&data).expect("reads");
    assert_eq!(back.requests, doc.requests);
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refuse = |requests: Vec<RequestRecord>, place: &str| {
        let err = kentos_kcad::encode(&with(requests)).expect_err(place);
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains(place), "{place}: {err}");
    };
    refuse(vec![request("a"), request("a")], "requests/1/requestId");
    refuse(vec![request("boşluk var")], "requests/0/requestId");
    refuse(
        vec![RequestRecord {
            fingerprint: "ABC".into(),
            ..request("a")
        }],
        "requests/0/fingerprint",
    );
    refuse(
        vec![RequestRecord {
            revision: "x".into(),
            ..request("a")
        }],
        "requests/0/revision",
    );
    refuse(
        vec![RequestRecord {
            answer: Some("x".repeat(64 * 1024 + 1)),
            ..request("a")
        }],
        "requests/0/answer",
    );
    refuse(
        (0..257).map(|i| request(&format!("r{i}"))).collect(),
        "en çok 256",
    );
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/requests-in-schema-44.kcad",
            Code::UnknownField,
            "document/requests",
        ),
        ("broken/requests-empty.kcad", Code::BadValue, "boş istek"),
        (
            "broken/requests-unknown-field.kcad",
            Code::UnknownField,
            "requests/0/note",
        ),
        (
            "broken/requests-missing-field.kcad",
            Code::MissingField,
            "requests/0/fingerprint",
        ),
        (
            "broken/requests-fingerprint-length.kcad",
            Code::BadValue,
            "16 bayt",
        ),
        ("broken/requests-status.kcad", Code::BadValue, "“lost”"),
        (
            "broken/requests-twice.kcad",
            Code::BadValue,
            "requests/1/requestId",
        ),
        (
            "broken/requests-id.kcad",
            Code::BadValue,
            "requests/0/requestId",
        ),
        (
            "broken/requests-revision.kcad",
            Code::BadValue,
            "requests/0/revision",
        ),
        (
            "broken/requests-version-zero.kcad",
            Code::BadValue,
            "requests/0/version",
        ),
        ("broken/schema-version-125.kcad", Code::SchemaVersion, ""),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}
