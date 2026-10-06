//! The layer states' and Kullanılmayanları temizle's rules (docs/adr/0177 §4,
//! §5) against the shared cases (fixtures/layers/v1/states.json and
//! purge.json, written by scripts/fixtures/layer_state_cases.py and
//! layer_purge_cases.py from the ADR, not KentOS code). The web reads the same
//! files (apps/web/src/model/layerStates.test.ts, layerPurge.test.ts).

use std::path::PathBuf;

use kentos_domain::contracts::{LayerNode, LayerState, LayerStyle};
use kentos_domain::layer_purge::{
    self, PurgeBlock, PurgeIds, PurgeItem, PurgeKind, PurgeObject, PurgeSource,
};
use kentos_domain::layer_states::{self, LayerStateParts};
use serde_json::Value;

fn read(name: &str) -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/layers/v1")
        .join(name);
    let text = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{path:?}: {e}"));
    serde_json::from_str(&text).unwrap_or_else(|e| panic!("{path:?}: {e}"))
}

fn tree(v: &Value) -> Vec<LayerNode> {
    serde_json::from_value(v.clone()).expect("a layer tree")
}

fn text(v: &Value, key: &str) -> String {
    v[key].as_str().unwrap_or_default().to_owned()
}

#[test]
fn layer_states_keep_and_change_the_tree_as_the_shared_cases_say() {
    let file = read("states.json");
    assert_eq!(file["format"], "kentos.layer-state-cases");
    let captures = file["captures"].as_array().expect("captures");
    assert_eq!(captures.len(), 4);
    for c in captures {
        let parts = LayerStateParts {
            locks: c["locks"] == true,
            styles: c["styles"] == true,
        };
        let state = layer_states::capture(
            &tree(&c["tree"]),
            &text(c, "id"),
            &text(c, "stateName"),
            parts,
        );
        let want: LayerState = serde_json::from_value(c["state"].clone()).expect("a state");
        assert_eq!(state, want, "{}", c["name"]);
        assert_eq!(layer_states::parts_of(&state), parts, "{}", c["name"]);
    }
    let applies = file["applies"].as_array().expect("applies");
    assert!(applies.len() >= 6);
    for c in applies {
        let t = tree(&c["tree"]);
        let state: LayerState = serde_json::from_value(c["state"].clone()).expect("a state");
        let got = layer_states::changes(&t, &state);
        let pairs = |v: &Value| -> Vec<(String, bool)> {
            v.as_array()
                .expect("pairs")
                .iter()
                .map(|p| (p[0].as_str().expect("an id").to_owned(), p[1] == true))
                .collect()
        };
        let want = &c["changes"];
        assert_eq!(got.visible, pairs(&want["visible"]), "{}", c["name"]);
        assert_eq!(got.locked, pairs(&want["locked"]), "{}", c["name"]);
        let styles: Vec<(String, LayerStyle)> = want["styles"]
            .as_array()
            .expect("styles")
            .iter()
            .map(|p| {
                let id = p[0].as_str().expect("an id").to_owned();
                (id, serde_json::from_value(p[1].clone()).expect("a style"))
            })
            .collect();
        assert_eq!(got.styles, styles, "{}", c["name"]);
        assert_eq!(
            Some(got.missing as u64),
            want["missing"].as_u64(),
            "{}",
            c["name"]
        );
        assert_eq!(
            layer_states::matches(&t, &state),
            c["matches"] == true,
            "{}",
            c["name"]
        );
    }
}

fn object(v: &Value) -> PurgeObject {
    PurgeObject {
        layer: text(v, "layer"),
        symbol: v["symbol"].as_str().map(str::to_owned),
        block: v["block"].as_str().map(str::to_owned),
        asset: v["asset"].as_str().map(str::to_owned),
    }
}

fn objects(v: &Value) -> Vec<PurgeObject> {
    v.as_array().expect("objects").iter().map(object).collect()
}

fn ids(v: &Value) -> PurgeIds {
    let mut out = PurgeIds::default();
    for kind in PurgeKind::ALL {
        if let Some(list) = v[kind.key()].as_array() {
            *out.of_mut(kind) = list
                .iter()
                .map(|x| x.as_str().expect("an id").to_owned())
                .collect();
        }
    }
    out
}

#[test]
fn purge_finds_and_removes_as_the_shared_cases_say() {
    let file = read("purge.json");
    assert_eq!(file["format"], "kentos.layer-purge-cases");
    let cases = file["cases"].as_array().expect("cases");
    assert!(cases.len() >= 2);
    for c in cases {
        let doc = &c["document"];
        let t = tree(&doc["tree"]);
        let active = text(doc, "active");
        let src = PurgeSource {
            tree: &t,
            active: &active,
            objects: objects(&doc["objects"]),
            blocks: doc["blocks"]
                .as_array()
                .expect("blocks")
                .iter()
                .map(|b| PurgeBlock {
                    id: text(b, "id"),
                    name: text(b, "name"),
                    objects: objects(&b["objects"]),
                })
                .collect(),
            library: doc["library"]
                .as_array()
                .expect("library")
                .iter()
                .map(|it| PurgeItem {
                    id: text(it, "id"),
                    kind: text(it, "kind"),
                    source: text(it, "source"),
                    name: text(it, "name"),
                    symbol: it.get("symbol").cloned(),
                    template: it.get("template").cloned(),
                })
                .collect(),
        };
        let found = layer_purge::found(&src);
        for kind in PurgeKind::ALL {
            let want: Vec<(String, String, bool)> = c["found"][kind.key()]
                .as_array()
                .expect("found")
                .iter()
                .map(|e| {
                    let words = if kind == PurgeKind::Layers || kind == PurgeKind::Groups {
                        "path"
                    } else {
                        "name"
                    };
                    (text(e, "id"), text(e, words), e["locked"] == true)
                })
                .collect();
            let got: Vec<(String, String, bool)> = found
                .of(kind)
                .iter()
                .map(|f| (f.id.clone(), f.text.clone(), f.locked))
                .collect();
            assert_eq!(got, want, "{}: {}", c["name"], kind.key());
        }
        for a in c["applies"].as_array().expect("applies") {
            let (removed, kept) = layer_purge::removed(&src, &ids(&a["checked"]));
            assert_eq!(removed, ids(&a["removed"]), "{}: {}", c["name"], a["name"]);
            assert_eq!(
                Some(kept as u64),
                a["kept"].as_u64(),
                "{}: {}",
                c["name"],
                a["name"]
            );
        }
    }
}
