//! Networks in the project file (docs/adr/0209 §2, docs/specs/kcad-v2.md
//! §6.4.8): document schema 33 is written only for a project with a network;
//! every other drawing keeps its schema and its bytes. Networks come back bit
//! for bit, a field direction's empty lists too; the writer refuses what the
//! reader would; the reader's errors name their places.

use kentos_kcad::Code;
use kentos_kcad::contracts::{
    DocumentSnapshotV2, NetworkConnect, NetworkCost, NetworkCostKind, NetworkDef, NetworkDirection,
    NetworkKind, NetworkLayer,
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

fn roads() -> NetworkDef {
    NetworkDef {
        id: "ag-1".into(),
        name: "Yollar".into(),
        kind: NetworkKind::Road,
        edges: vec![NetworkLayer {
            layer: "0".into(),
            filter: None,
        }],
        junctions: Vec::new(),
        connect: NetworkConnect::Ends,
        tolerance: 0.01,
        direction: NetworkDirection::Both,
        costs: Vec::new(),
        closed: None,
    }
}

fn with(network: NetworkDef) -> DocumentSnapshotV2 {
    let mut doc = content("minimal.json");
    doc.settings.networks = vec![network];
    doc
}

#[test]
fn only_a_project_with_a_network_is_schema_33() {
    let of = |doc: &DocumentSnapshotV2| schema(&kentos_kcad::encode_verified(doc).expect("writes"));
    let data = read("networks.kcad");
    assert_eq!(schema(&data), 33);
    let doc = kentos_kcad::decode(&data).expect("reads");
    assert!(kentos_kcad::encode(&doc).expect("writes") == data);
    assert_eq!(of(&content("minimal.json")), 2);
    assert_eq!(of(&with(roads())), 33);
    // The newest drawings before it keep their schemas.
    assert_eq!(of(&content("services.json")), 32);
    assert_eq!(of(&content("pointclouds.json")), 31);
}

#[test]
fn networks_come_back_bit_for_bit() {
    let doc = content("networks.json");
    let bytes = kentos_kcad::encode_verified(&doc).expect("writes");
    let again = kentos_kcad::decode(&bytes).expect("reads");
    assert_eq!(
        serde_json::to_string(&again).expect("serializes"),
        serde_json::to_string(&doc).expect("serializes")
    );
    let nets = &again.settings.networks;
    assert_eq!(nets.len(), 3);
    assert_eq!(nets[0].cost_names(), vec!["Uzunluk", "Süre", "Ücret"]);
    assert_eq!(nets[1].junctions.len(), 3);
    match &nets[2].direction {
        NetworkDirection::Field {
            forward,
            backward,
            closed,
            ..
        } => {
            assert_eq!(forward, &vec!["ileri".to_string()]);
            assert!(
                backward.is_empty() && closed.is_empty(),
                "empty lists stay empty"
            );
        }
        other => panic!("a field direction: {other:?}"),
    }
}

#[test]
fn the_writer_refuses_what_the_reader_would() {
    let refused = |doc: DocumentSnapshotV2| kentos_kcad::encode(&doc).expect_err("refused");
    let changed = |f: fn(&mut NetworkDef)| {
        let mut n = roads();
        f(&mut n);
        with(n)
    };
    let mut twice = with(roads());
    let mut other = roads();
    other.id = "ag-2".into();
    other.name = "YOLLAR".into();
    twice.settings.networks.push(other);
    for (doc, words) in [
        (changed(|n| n.id = "Ağ 1".into()), "kimliği"),
        (changed(|n| n.tolerance = 20.0), "toleransı"),
        (changed(|n| n.edges.clear()), "kenar katmanı yok"),
        (
            changed(|n| {
                n.costs.push(NetworkCost {
                    name: "uzunluk".into(),
                    kind: NetworkCostKind::Field,
                    field: "u".into(),
                    unit: String::new(),
                    speed: None,
                })
            }),
            "uzunluğunundur",
        ),
        (
            changed(|n| {
                n.direction = NetworkDirection::Field {
                    field: "yon".into(),
                    forward: Vec::new(),
                    backward: Vec::new(),
                    closed: Vec::new(),
                }
            }),
            "değeri yok",
        ),
        (twice, "iki kez var"),
    ] {
        let err = refused(doc);
        assert_eq!(err.code, Code::BadValue, "{err}");
        assert!(err.message.contains("settings/networks"), "{err}");
        assert!(err.message.contains(words), "{err}");
    }
}

#[test]
fn the_readers_errors_name_their_places() {
    let refused = |name: &str| kentos_kcad::decode(&read(name)).expect_err(name);
    for (file, code, place) in [
        (
            "broken/networks-in-schema-32.kcad",
            Code::UnknownField,
            "settings/networks",
        ),
        (
            "broken/network-unknown-field.kcad",
            Code::UnknownField,
            "networks/0/speed",
        ),
        (
            "broken/networks-empty.kcad",
            Code::BadValue,
            "boş ağ listesi",
        ),
        (
            "broken/network-junctions-empty.kcad",
            Code::BadValue,
            "boş düğüm katmanı",
        ),
        (
            "broken/network-direction-list-empty.kcad",
            Code::BadValue,
            "direction/backward",
        ),
        (
            "broken/network-direction-field-on-both.kcad",
            Code::BadValue,
            "yalnız alanla yönde",
        ),
        (
            "broken/network-duplicate-name.kcad",
            Code::BadValue,
            "iki kez var",
        ),
        (
            "broken/network-cost-speed-unit.kcad",
            Code::BadValue,
            "birim yazılmaz",
        ),
        (
            "broken/network-unknown-kind.kcad",
            Code::BadValue,
            "networks/0/kind",
        ),
        ("broken/schema-version-36.kcad", Code::SchemaVersion, ""),
    ] {
        let e = refused(file);
        assert_eq!(e.code, code, "{file}: {e}");
        assert!(e.message.contains(place), "{file}: {e}");
    }
}
