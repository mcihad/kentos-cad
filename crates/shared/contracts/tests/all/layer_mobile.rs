//! A layer on the mobile clients (docs/adr/9198 §2.1): a hidden group hides what it holds; else a node's own choice,
//! shown when it has none.

use kentos_contracts::{LayerMobile, LayerNode, layer_on_mobile};
use serde_json::json;

fn node(id: &str, mobile: Option<&str>, children: Vec<serde_json::Value>) -> serde_json::Value {
    let mut n = json!({
        "id": id, "name": id, "type": if children.is_empty() { "layer" } else { "group" },
        "visible": true, "locked": false, "expanded": true,
        "style": { "color": "fg", "lineType": "continuous", "lineWeight": 0.25 }, "children": children
    });
    if let Some(m) = mobile {
        n["mobile"] = json!(m);
    }
    n
}

#[test]
fn a_hidden_group_hides_what_it_holds_on_the_phone() {
    let tree: Vec<LayerNode> = serde_json::from_value(json!([
        node(
            "saha",
            None,
            vec![
                node("parsel", Some("edit"), vec![]),
                node("bina", None, vec![])
            ]
        ),
        node(
            "altlik",
            Some("hide"),
            vec![
                node("ortofoto", None, vec![]),
                node("yol", Some("edit"), vec![])
            ]
        ),
        node("not", Some("hide"), vec![]),
    ]))
    .expect("a layer tree");
    let on = |id: &str| layer_on_mobile(&tree, id);
    assert_eq!(on("saha"), Some(LayerMobile::Show));
    assert_eq!(on("parsel"), Some(LayerMobile::Edit));
    assert_eq!(on("bina"), Some(LayerMobile::Show));
    assert_eq!(on("altlik"), Some(LayerMobile::Hide));
    assert_eq!(on("ortofoto"), Some(LayerMobile::Hide));
    assert_eq!(
        on("yol"),
        Some(LayerMobile::Hide),
        "a hidden group's edit layer is not on the phone"
    );
    assert_eq!(on("not"), Some(LayerMobile::Hide));
    assert_eq!(on("yok"), None);
}
