//! A network's turns and levels and a speed cost's time profile in the project file (docs/adr/0521,
//! docs/specs/kcad-v2.md §6.4.8): document schema 87 is written only when a network has one, every other drawing keeps
//! its schema and its bytes; they come back as written; the writer refuses what the reader would.

use kentos_kcad::Code;
use kentos_kcad::contracts::{DocumentSnapshotV2, NetworkPeriod};

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
fn a_network_with_turns_is_schema_87_and_one_without_stays_as_it_was() {
    let doc = content("network-turns.json");
    let written = kentos_kcad::encode(&doc).expect("writes");
    assert_eq!(schema(&written), 87);
    assert!(
        written == read("network-turns.kcad"),
        "the reference's bytes"
    );
    let back = kentos_kcad::decode(&written).expect("reads");
    assert_eq!(back.settings.networks, doc.settings.networks);
    // Without them: the networks' own drawing, byte for byte.
    let mut plain = doc.clone();
    for n in &mut plain.settings.networks {
        n.turns = None;
        n.levels = None;
        for c in &mut n.costs {
            c.periods.clear();
        }
    }
    assert!(kentos_kcad::encode(&plain).expect("writes") == read("networks.kcad"));
}

#[test]
fn the_writer_refuses_what_the_reader_refuses() {
    let refused = |change: &dyn Fn(&mut DocumentSnapshotV2)| {
        let mut doc = content("network-turns.json");
        change(&mut doc);
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    let e = refused(&|d| {
        d.settings.networks[0].costs[0].periods.push(NetworkPeriod {
            from: 1000,
            to: 1100,
            factor: Some(0.5),
            field: None,
        })
    });
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("örtüşüyor"), "{e}");
    let e = refused(&|d| d.settings.networks[0].turns.as_mut().expect("turns").left = 5000.0);
    assert!(e.message.contains("sola dönüş"), "{e}");
    let e = refused(&|d| {
        d.settings.networks[0]
            .levels
            .as_mut()
            .expect("levels")
            .start = String::new()
    });
    assert!(e.message.contains("seviye alanı"), "{e}");
}
