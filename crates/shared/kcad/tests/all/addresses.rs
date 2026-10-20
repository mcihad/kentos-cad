//! Address data in the project file (docs/adr/0229 §2, docs/specs/kcad-v2.md
//! §6.4.10): document schema 42 is written only for a project with address
//! data; every other drawing keeps its schema and its bytes. The locators
//! come back bit for bit; the writer refuses what the reader would; the
//! reader's errors name their places.

use kentos_kcad::Code;
use kentos_kcad::contracts::{
    AddressLocator, AreaLevel, AreaSource, DocumentSnapshotV2, DoorSource, StreetSource,
};

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

fn numarataj() -> AddressLocator {
    AddressLocator {
        id: "adres-1".into(),
        name: "Numarataj".into(),
        doors: Some(DoorSource {
            layer: "0".into(),
            street: "YOL".into(),
            door: "KAPI".into(),
            neighbourhood: None,
            district: None,
            province: None,
            postcode: None,
            building: None,
            code: None,
        }),
        streets: None,
        areas: Vec::new(),
        min_score: 85,
        reach: 100.0,
    }
}

fn with(locator: AddressLocator) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    doc.settings.address_locators = vec![locator];
    doc
}

#[test]
fn only_a_project_with_address_data_is_schema_42() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("addresses.kcad");
    assert_eq!(schema(&data), 42);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(of(&content("minimal.json")), 2);
    assert_eq!(of(&with(numarataj())), 42);
    // The newest drawings before it keep their schemas.
    assert_eq!(of(&content("compose.json")), 41);
    assert_eq!(of(&content("attachments.json")), 40);
}

#[test]
fn address_data_comes_back_bit_for_bit() {
    let doc = content("addresses.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let list = &again.settings.address_locators;
    assert_eq!(list.len(), 2);
    assert_eq!(list[0].layers(), vec!["kapi", "yol", "mahalle", "ilce"]);
    assert_eq!(
        list[0]
            .area(AreaLevel::Neighbourhood)
            .and_then(|a| a.district.as_deref()),
        Some("ILCE")
    );
    assert!(list[1].doors.is_none() && list[1].areas.is_empty());
    assert_eq!(list[1].reach, 250.5);
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |doc: DocumentSnapshotV2| kentos_kcad::encode(&doc).expect_err("refused");
    let changed = |f: fn(&mut AddressLocator)| {
        let mut l = numarataj();
        f(&mut l);
        with(l)
    };
    let mut twice = with(numarataj());
    let mut other = numarataj();
    other.id = "adres-2".into();
    other.name = "NUMARATAJ".into();
    twice.settings.address_locators.push(other);
    for (doc, words) in [
        (changed(|l| l.id = "Adres 1".into()), "kimliği"),
        (changed(|l| l.reach = 0.0), "uzaklığı"),
        (changed(|l| l.min_score = 101), "en düşük puanı"),
        (changed(|l| l.doors = None), "kaynağı yok"),
        (
            changed(|l| {
                l.streets = Some(StreetSource {
                    layer: "0".into(),
                    name: "".into(),
                    neighbourhood: None,
                    district: None,
                })
            }),
            "ad alanı boş",
        ),
        (
            changed(|l| {
                let area = AreaSource {
                    layer: "0".into(),
                    level: AreaLevel::Province,
                    name: "AD".into(),
                    district: None,
                };
                l.areas = vec![area.clone(), area];
            }),
            "iki kez var",
        ),
        (twice, "iki kez var"),
    ] {
        let err = refused(doc);
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains("settings/addressLocators"), "{err}");
        assert!(err.message.contains(words), "{err}");
    }
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/addresses-in-schema-41.kcad",
            Code::UnknownField,
            "settings/addressLocators",
        ),
        (
            "broken/address-unknown-field.kcad",
            Code::UnknownField,
            "addressLocators/0/note",
        ),
        (
            "broken/addresses-empty.kcad",
            Code::BadValue,
            "boş adres verisi listesi",
        ),
        (
            "broken/address-areas-empty.kcad",
            Code::BadValue,
            "boş bölge listesi",
        ),
        (
            "broken/address-door-field-empty.kcad",
            Code::BadValue,
            "doors/door",
        ),
        (
            "broken/address-door-without-street.kcad",
            Code::MissingField,
            "street",
        ),
        (
            "broken/address-area-unknown-level.kcad",
            Code::BadValue,
            "areas/0/level",
        ),
        (
            "broken/address-duplicate-name.kcad",
            Code::BadValue,
            "iki kez var",
        ),
        (
            "broken/address-district-area-district.kcad",
            Code::BadValue,
            "ilçe alanı olmaz",
        ),
        ("broken/schema-version-94.kcad", Code::SchemaVersion, ""),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}
