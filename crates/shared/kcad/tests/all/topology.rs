//! Topology rules in the project file (docs/adr/0202 §7, docs/specs/kcad-v2.md
//! §6.1, §6.4.5): document schema 27 is written only for a project with
//! topology settings, every other drawing keeps its schema and its bytes;
//! the rules and the exceptions come back as written; the writer refuses
//! what the reader would.

use kentos_kcad::Code;
use kentos_kcad::contracts::{
    DocumentSnapshotV2, EntityId, TopologyException, TopologyRule, TopologyRuleKind,
    TopologySettings, Vec2,
};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says: the unsigned integer after the
/// `version` key (from 24 on, a byte 0x18 and the value).
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

fn rule(id: &str, kind: TopologyRuleKind, other: Option<&str>, value: Option<f64>) -> TopologyRule {
    TopologyRule {
        id: id.into(),
        kind,
        layer: "0".into(),
        other: other.map(str::to_owned),
        value,
    }
}

#[test]
fn a_project_with_topology_rules_is_schema_27_and_one_without_stays_as_it_was() {
    let doc = content("topology.json");
    let written = kentos_kcad::encode(&doc).expect("writes");
    assert_eq!(schema(&written), 27);
    assert!(written == read("topology.kcad"), "the reference's bytes");
    let mut plain = doc.clone();
    plain.settings.topology = None;
    let written = kentos_kcad::encode(&plain).expect("writes");
    assert!(schema(&written) < 27, "{}", schema(&written));
    assert!(
        kentos_kcad::decode(&written)
            .expect("reads")
            .settings
            .topology
            .is_none()
    );
}

#[test]
fn the_rules_and_exceptions_come_back_as_written() {
    let doc = content("topology.json");
    let back = kentos_kcad::decode(&kentos_kcad::encode(&doc).expect("writes")).expect("reads");
    assert_eq!(back.settings.topology, doc.settings.topology);
    let t = back.settings.topology.expect("topology");
    assert_eq!(t.tolerance, Some(0.002));
    assert_eq!(t.rules.len(), 5);
    assert_eq!(t.rules[4].kind, TopologyRuleKind::MustBeCoveredBy);
    assert_eq!(t.rules[4].other.as_deref(), Some("parsel"));
    assert_eq!(t.exceptions[0].objects.len(), 2);
}

#[test]
fn the_writer_refuses_what_the_reader_refuses() {
    let refused = |t: TopologySettings| {
        let mut doc = content("minimal.json");
        doc.settings.topology = Some(t);
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    let e = refused(TopologySettings::default());
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("topoloji ayarı boş"), "{e}");
    assert!(e.message.contains("settings/topology"), "the place: {e}");
    let e = refused(TopologySettings {
        tolerance: Some(2.0),
        ..Default::default()
    });
    assert!(e.message.contains("toleransı 2 m"), "{e}");
    let two = |a, b| TopologySettings {
        rules: vec![a, b],
        ..Default::default()
    };
    let one = |r| TopologySettings {
        rules: vec![r],
        ..Default::default()
    };
    let e = refused(two(
        rule("r", TopologyRuleKind::MustNotOverlap, None, None),
        rule("r", TopologyRuleKind::MustNotHaveGaps, None, None),
    ));
    assert!(e.message.contains("iki kez var"), "{e}");
    let e = refused(one(rule(
        "r",
        TopologyRuleKind::MustBeCoveredBy,
        None,
        None,
    )));
    assert!(e.message.contains("öbür katmanı yok"), "{e}");
    let e = refused(one(rule(
        "r",
        TopologyRuleKind::MustBeCoveredBy,
        Some("0"),
        None,
    )));
    assert!(e.message.contains("kendi katmanı"), "{e}");
    let e = refused(one(rule(
        "r",
        TopologyRuleKind::MustNotOverlap,
        Some("1"),
        None,
    )));
    assert!(e.message.contains("öbür katmanı olmaz"), "{e}");
    let e = refused(one(rule(
        "r",
        TopologyRuleKind::MustNotOverlap,
        None,
        Some(1.0),
    )));
    assert!(e.message.contains("değer almaz"), "{e}");
    let e = refused(one(rule(
        "r",
        TopologyRuleKind::MustNotHaveSmallAngles,
        None,
        Some(2.0),
    )));
    assert!(e.message.contains("dik açıdan küçük"), "{e}");
    let e = refused(TopologySettings {
        rules: vec![rule("r", TopologyRuleKind::MustNotOverlap, None, None)],
        exceptions: vec![TopologyException {
            rule: "q".into(),
            objects: vec![EntityId([1; 16])],
            at: Vec2 { x: 0.0, y: 0.0 },
        }],
        ..Default::default()
    });
    assert!(e.message.contains("istisnanın kuralı “q” yok"), "{e}");
}
