//! The installed extensions in the agents' common catalog (catalog_agents.rs, docs/adr/8057 §109, PY-AI-01): the
//! link's `extensions` says each with its capabilities asked and given, the Python it needs and whether that is
//! ready, its parts and how an agent uses each; `catalog.search` finds them by words and `catalog.describe` describes
//! one, through Asistan's door as through the link's.

use kentos_headless::agent_tools::{self, Guard};
use serde_json::{Value, json};

use super::demo::vitrin_package;
use super::*;

const VITRIN: &str = "tr.ornek.vitrin";

#[test]
fn an_installed_extension_is_in_the_agents_catalog() {
    let root = temp("katalog-vitrin");
    let mut gis = app(&root, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let listed = gis.ext_catalog();
    let vitrin = listed["extensions"]
        .as_array()
        .and_then(|l| l.iter().find(|e| e["id"] == VITRIN))
        .unwrap_or_else(|| panic!("{listed:#}"))
        .clone();
    // What it needs: the Python its project declares, as written.
    assert_eq!(
        vitrin["python"]["requirements"],
        json!(["shapely>=2.0,<2.1", "numpy>=2.0,<3"])
    );
    // What it was written for, judged here (docs/adr/8057 §113): it runs on this KentOS.
    let compat = &vitrin["compatibility"];
    assert_eq!(compat["fitsHere"], true, "{compat:#}");
    assert_eq!(
        compat["sdkProtocol"]["here"],
        kentos_extensions::SDK_PROTOCOL
    );
    assert!(
        compat["runtimes"]
            .as_array()
            .is_some_and(|r| r.iter().any(|x| x == "desktop")),
        "{compat:#}"
    );
    // Its capabilities, as asked and as given (the manager's install gives everything).
    let caps = vitrin["capabilities"].as_array().expect("capabilities");
    assert!(
        caps.iter()
            .any(|c| c["id"] == "processing.provider" && c["granted"] == true)
    );
    assert!(
        caps.iter()
            .any(|c| c["id"] == "audit.provider" && c["title"] == "Denetim ekler")
    );
    assert!(
        caps.iter()
            .all(|c| c["reason"].as_str().is_some_and(|r| !r.is_empty()))
    );
    // Its parts with how an agent uses each; a tab and a menu item are the interface's own.
    let parts = vitrin["contributions"].as_array().expect("parts");
    let kind_of = |id: &str| {
        parts
            .iter()
            .find(|p| p["id"] == id)
            .map(|p| (p["kind"].clone(), p["use"].clone()))
    };
    let (kind, used) = kind_of("ext.tr.ornek.vitrin.cekme").expect("Çekme");
    assert_eq!(kind, "tool");
    assert!(used.as_str().is_some_and(|u| u.contains("processing.run")));
    assert_eq!(
        kind_of("ext.tr.ornek.vitrin.yapi-no").map(|k| k.0),
        Some(json!("rule"))
    );
    assert!(parts.iter().any(|p| p["kind"] == "panel"));
    assert!(parts.iter().all(|p| p["kind"] != "ribbonTab"));
    // Through Asistan's door: found by its words, described by its id.
    let mut guard = Guard::new();
    let mut host = crate::assistant::host::AppHost(&mut gis);
    let found = agent_tools::call(
        &mut host,
        &mut guard,
        "catalog.search",
        &json!({ "query": "vitrin", "kinds": ["extension"] }),
    )
    .expect("found");
    let others: Vec<&Value> = found["others"].as_array().expect("others").iter().collect();
    assert_eq!(others[0]["id"], VITRIN, "{found:#}");
    assert_eq!(others[0]["kind"], "extension");
    let described = agent_tools::call(
        &mut host,
        &mut guard,
        "catalog.describe",
        &json!({ "extension": VITRIN }),
    )
    .expect("described");
    assert_eq!(described["python"], vitrin["python"]);
    assert!(
        described["note"]
            .as_str()
            .is_some_and(|n| n.contains("veridir"))
    );
    let unknown = agent_tools::call(
        &mut host,
        &mut guard,
        "catalog.describe",
        &json!({ "extension": "tr.ornek.yok" }),
    );
    assert_eq!(
        unknown.err().map(|e| e.code),
        Some("unknown_extension".into())
    );
    let _ = std::fs::remove_dir_all(&root);
}
