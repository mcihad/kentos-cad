//! A network's terminals and subnetworks in the project file (docs/adr/0557, docs/specs/kcad-v2.md §6.4.8): document
//! schema 93 is written only when a network has one, every other drawing keeps its schema and its bytes; they come back
//! as written; the writer refuses what the reader would.

use kentos_kcad::Code;
use kentos_kcad::contracts::{DocumentSnapshotV2, SubnetworkReach};

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
fn a_network_with_terminals_is_schema_93_and_one_without_stays_as_it_was() {
    let doc = content("network-terminals.json");
    let written = kentos_kcad::encode(&doc).expect("writes");
    assert_eq!(schema(&written), 93);
    assert!(
        written == read("network-terminals.kcad"),
        "the reference's bytes"
    );
    let back = kentos_kcad::decode(&written).expect("reads");
    assert_eq!(back.settings.networks, doc.settings.networks);
    // Without them: the networks' own drawing, byte for byte.
    let mut plain = doc.clone();
    for n in &mut plain.settings.networks {
        n.terminals = None;
        n.subnetworks = None;
    }
    assert!(kentos_kcad::encode(&plain).expect("writes") == read("networks.kcad"));
    // Subnetworks alone are schema 93 too.
    let mut alone = doc.clone();
    for n in &mut alone.settings.networks {
        n.terminals = None;
    }
    assert_eq!(schema(&kentos_kcad::encode(&alone).expect("writes")), 93);
}

#[test]
fn the_writer_refuses_what_the_reader_refuses() {
    let refused = |change: &dyn Fn(&mut DocumentSnapshotV2)| {
        let mut doc = content("network-terminals.json");
        change(&mut doc);
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    let e = refused(&|d| {
        d.settings.networks[1]
            .terminals
            .as_mut()
            .expect("terminals")
            .devices[0]
            .paths[0] = vec!["giris".into(), "yok".into()]
    });
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("cihazın terminali değil"), "{e}");
    let e = refused(&|d| {
        d.settings.networks[1]
            .terminals
            .as_mut()
            .expect("terminals")
            .end = "uc_bas".into()
    });
    assert!(e.message.contains("aynı"), "{e}");
    let e = refused(&|d| {
        let s = d.settings.networks[1]
            .subnetworks
            .as_mut()
            .expect("subnetworks");
        s.attribute = "Ad".into();
        s.reach = SubnetworkReach::Connected;
    });
    assert!(e.message.contains("üstüne"), "{e}");
}
