//! The project's sheet book in the project file (docs/adr/0251,
//! docs/specs/kcad-v2.md §6.12): document schema 44 is written only for a
//! project with a book; every other drawing keeps its schema and its bytes.
//! The book and its pictures come back bit for bit; the writer refuses what
//! the reader would; the reader's errors name their places.

use kentos_kcad::Code;
use kentos_kcad::contracts::encoding::sha256_hex;
use kentos_kcad::contracts::{DocumentSnapshotV2, ProjectSheets, SheetPicture};
use serde_json::json;

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

fn picture(bytes: &[u8]) -> SheetPicture {
    SheetPicture {
        sha256: sha256_hex(bytes),
        bytes: bytes.to_vec(),
    }
}

fn with(sheets: ProjectSheets) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    doc.sheets = Some(sheets);
    doc
}

fn book() -> serde_json::Value {
    json!({ "schema": "kentos.sheet/1", "sheets": [], "variables": [{ "name": "ada", "value": 101 }] })
}

#[test]
fn only_a_project_with_a_sheet_book_is_schema_44() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("sheets.kcad");
    assert_eq!(schema(&data), 44);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(of(&content("minimal.json")), 2);
    let bare = with(ProjectSheets {
        book: book(),
        pictures: Vec::new(),
    });
    assert_eq!(of(&bare), 44);
    // The newest drawings before it keep their schemas.
    assert_eq!(of(&content("dimension-refs.json")), 43);
    assert_eq!(of(&content("addresses.json")), 42);
}

#[test]
fn the_book_and_its_pictures_come_back_bit_for_bit() {
    let large: Vec<u8> = (0..300_000u32).map(|i| (i * 7 % 251) as u8).collect();
    let doc = with(ProjectSheets {
        book: book(),
        pictures: vec![picture(b"\x89PNG kentos"), picture(&large)],
    });
    let data = kentos_kcad::encode_verified(&doc).expect("writes");
    let back = kentos_kcad::decode(&data).expect("reads");
    assert_eq!(back.sheets, doc.sheets);
    // An integer of the book stays an integer, a float a float (docs/specs/kcad-v2.md §6.7).
    let mut floats = book();
    floats["variables"][0]["value"] = json!(101.0);
    let other = with(ProjectSheets {
        book: floats,
        pictures: Vec::new(),
    });
    let back = kentos_kcad::decode(&kentos_kcad::encode(&other).expect("writes")).expect("reads");
    assert_eq!(
        back.sheets.expect("a book").book["variables"][0]["value"],
        json!(101.0)
    );
    assert_ne!(
        kentos_kcad::encode(&other).expect("writes"),
        kentos_kcad::encode(&with(ProjectSheets {
            book: book(),
            pictures: Vec::new()
        }))
        .expect("writes")
    );
    // The sample's book is the sheet core's own (fixtures/sheet/v1/kpafta/valid/buro-paftasi.kpafta).
    let sample = content("sheets.json").sheets.expect("a book");
    assert_eq!(sample.book["schema"], "kentos.sheet/1");
    assert_eq!(sample.pictures.len(), 1);
    assert_eq!(
        sample.pictures[0].sha256,
        sha256_hex(&sample.pictures[0].bytes)
    );
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refuse = |sheets: ProjectSheets, place: &str| {
        let err = kentos_kcad::encode(&with(sheets)).expect_err(place);
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains(place), "{place}: {err}");
    };
    refuse(
        ProjectSheets {
            book: json!(["kentos.sheet/1"]),
            pictures: Vec::new(),
        },
        "sheets/book",
    );
    let mut wrong = picture(b"logo");
    wrong.bytes = b"baska".to_vec();
    refuse(
        ProjectSheets {
            book: book(),
            pictures: vec![picture(b"a"), wrong],
        },
        "sheets/pictures/1/sha256",
    );
    refuse(
        ProjectSheets {
            book: book(),
            pictures: vec![picture(b"a"), picture(b"a")],
        },
        "sheets/pictures/1/sha256",
    );
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/sheets-in-schema-43.kcad",
            Code::UnknownField,
            "document/sheets",
        ),
        (
            "broken/sheets-book-not-a-map.kcad",
            Code::WrongType,
            "sheets/book",
        ),
        (
            "broken/sheets-book-missing.kcad",
            Code::MissingField,
            "sheets/book",
        ),
        (
            "broken/sheets-unknown-field.kcad",
            Code::UnknownField,
            "sheets/assets",
        ),
        (
            "broken/sheets-pictures-empty.kcad",
            Code::BadValue,
            "boş resim listesi",
        ),
        (
            "broken/sheets-picture-digest.kcad",
            Code::BadValue,
            "SHA-256'sına uymuyor",
        ),
        (
            "broken/sheets-picture-twice.kcad",
            Code::BadValue,
            "iki kez",
        ),
        (
            "broken/sheets-picture-sha256-length.kcad",
            Code::BadValue,
            "pictures/0/sha256",
        ),
        (
            "broken/sheets-picture-unknown-field.kcad",
            Code::UnknownField,
            "pictures/0/name",
        ),
        ("broken/schema-version-52.kcad", Code::SchemaVersion, ""),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}
