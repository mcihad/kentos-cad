//! Layer states in the project file (docs/adr/0177 §4, docs/specs/kcad-v2.md
//! §6.1, §6.4.3): document schema 19 is written only for a project with
//! layer states, every other drawing keeps its schema and its bytes; the
//! states come back as written, a node's lock and style only where they were
//! kept; the writer refuses what the reader would (an empty or a repeated id
//! or name, a repeated node).

use kentos_kcad::Code;
use kentos_kcad::contracts::{DocumentSnapshotV2, LayerState, LayerStateNode};

const DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/../../../fixtures/kcad/v2/");

fn read(name: &str) -> Vec<u8> {
    std::fs::read(format!("{DIR}{name}")).unwrap_or_else(|e| panic!("{name}: {e}"))
}

fn content(name: &str) -> DocumentSnapshotV2 {
    serde_json::from_slice(&read(name)).unwrap_or_else(|e| panic!("{name}: {e}"))
}

/// The document schema a file's payload says: the byte after the `version` key.
fn schema(file: &[u8]) -> u8 {
    let payload = &file[36..file.len() - 32];
    let key = b"\x67version";
    let at = payload
        .windows(key.len())
        .position(|w| w == key)
        .expect("a version key")
        + key.len();
    payload[at]
}

fn state(id: &str, name: &str, nodes: &[&str]) -> LayerState {
    LayerState {
        id: id.into(),
        name: name.into(),
        nodes: nodes
            .iter()
            .map(|n| LayerStateNode {
                node: (*n).into(),
                visible: true,
                locked: None,
                style: None,
            })
            .collect(),
    }
}

#[test]
fn a_project_with_layer_states_is_schema_19_and_one_without_stays_as_it_was() {
    let doc = content("layer-states.json");
    let written = kentos_kcad::encode(&doc).expect("writes");
    assert_eq!(schema(&written), 19);
    assert!(
        written == read("layer-states.kcad"),
        "the reference's bytes"
    );
    // Without its states the same drawing is the schema it was before them.
    let mut plain = doc.clone();
    plain.settings.layer_states.clear();
    let written = kentos_kcad::encode(&plain).expect("writes");
    assert!(schema(&written) < 19, "{}", schema(&written));
    let back = kentos_kcad::decode(&written).expect("reads");
    assert!(back.settings.layer_states.is_empty());
}

#[test]
fn the_states_come_back_as_written_their_locks_and_styles_only_where_kept() {
    let doc = content("layer-states.json");
    let back = kentos_kcad::decode(&kentos_kcad::encode(&doc).expect("writes")).expect("reads");
    assert_eq!(back.settings.layer_states, doc.settings.layer_states);
    let print = &back.settings.layer_states[1];
    assert_eq!(print.name, "Baskı");
    assert_eq!(print.nodes[1].locked, Some(true));
    assert_eq!(
        print.nodes[2]
            .style
            .as_ref()
            .and_then(|s| s.fill.as_deref()),
        Some("#8C9AAA33")
    );
    // A node the tree no longer has is kept: the state passes over it when applied.
    assert_eq!(print.nodes[3].node, "silinmis");
    assert!(print.nodes[3].locked.is_none() && print.nodes[3].style.is_none());
}

#[test]
fn the_writer_refuses_what_the_reader_refuses() {
    let refused = |states: Vec<LayerState>| {
        let mut doc = content("minimal.json");
        doc.settings.layer_states = states;
        kentos_kcad::encode(&doc).expect_err("refused")
    };
    let e = refused(vec![state("", "Görünüm", &["0"])]);
    assert_eq!(e.code, Code::BadValue);
    assert!(e.message.contains("kimliği boş"), "{e}");
    let e = refused(vec![
        state("a", "Görünüm", &["0"]),
        state("a", "Baskı", &["0"]),
    ]);
    assert!(
        e.message.contains("“a” kimlikli katman durumu iki kez var"),
        "{e}"
    );
    let e = refused(vec![state("a", "  ", &["0"])]);
    assert!(e.message.contains("adı boş"), "{e}");
    let e = refused(vec![
        state("a", "Görünüm", &["0"]),
        state("b", " Görünüm ", &["0"]),
    ]);
    assert!(
        e.message
            .contains("“Görünüm” adlı katman durumu iki kez var"),
        "{e}"
    );
    let e = refused(vec![state("a", "Görünüm", &["0", "0"])]);
    assert!(
        e.message
            .contains("“Görünüm” durumunda “0” düğümü iki kez var"),
        "{e}"
    );
    assert!(e.message.contains("settings/layerStates"), "the place: {e}");
}
