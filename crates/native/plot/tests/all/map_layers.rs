//! A map's own layers (docs/adr/0314 §1): a layer state by name in place of the drawing's visibility, freezing,
//! printing and kept looks, then the map's overrides; the drawing's layers stay as they are.

use kentos_contracts::{LayerNode, LayerState};
use kentos_plot::layers::map_layers;
use kentos_sheet::kinds::{Empty, LayerOverride, MapLayers, ThemeRef};
use serde_json::json;

fn layer(id: &str, visible: bool, color: &str) -> LayerNode {
    serde_json::from_value(json!({
        "id": id, "name": id, "type": "layer", "visible": visible, "locked": false, "expanded": true,
        "style": { "color": color, "lineType": "continuous", "lineWeight": 0.25 }, "children": []
    }))
    .expect("a layer")
}

fn tree() -> Vec<LayerNode> {
    vec![
        layer("duvar", true, "#111111"),
        layer("aks", false, "#E5484D"),
        layer("mobilya", true, "#8A94A6"),
    ]
}

fn state() -> LayerState {
    serde_json::from_value(json!({
        "id": "s1", "name": "Sunum",
        "nodes": [
            { "node": "duvar", "visible": true, "style": { "color": "#000000", "lineType": "continuous", "lineWeight": 0.5 } },
            { "node": "aks", "visible": true },
            { "node": "mobilya", "visible": false }
        ]
    }))
    .expect("a state")
}

fn shown(
    nodes: &[LayerNode],
    layers: &MapLayers,
    overrides: &[LayerOverride],
) -> Vec<(String, String, f64)> {
    map_layers(nodes, &[state()], layers, overrides)
        .into_iter()
        .map(|n| (n.id.clone(), n.style.color.clone(), n.style.line_weight))
        .collect()
}

fn over(layer: &str) -> LayerOverride {
    LayerOverride {
        layer: layer.into(),
        frozen: false,
        plot: None,
        color: None,
        line_type: None,
        line_pattern: None,
        line_weight: None,
    }
}

#[test]
fn without_a_state_or_overrides_the_map_prints_the_drawing_s_layers() {
    let nodes = tree();
    let all = MapLayers::All(Empty {});
    // Bottom first: the tree's last layer first; the hidden axis left out.
    assert_eq!(
        shown(&nodes, &all, &[]),
        [
            ("mobilya".into(), "#8A94A6".into(), 0.25),
            ("duvar".into(), "#111111".into(), 0.25)
        ]
    );
}

#[test]
fn a_state_shows_and_hides_and_gives_the_looks_it_kept() {
    let nodes = tree();
    let theme = MapLayers::Theme(ThemeRef {
        name: " Sunum ".into(),
    });
    assert_eq!(
        shown(&nodes, &theme, &[]),
        [
            ("aks".into(), "#E5484D".into(), 0.25),
            ("duvar".into(), "#000000".into(), 0.5)
        ]
    );
    // A state the project does not have: the drawing's own.
    let missing = MapLayers::Theme(ThemeRef { name: "Yok".into() });
    assert_eq!(shown(&nodes, &missing, &[]).len(), 2);
    // The drawing stays as it is.
    assert_eq!(nodes, tree());
}

#[test]
fn overrides_freeze_print_and_draw_a_layer_otherwise_in_this_map_only() {
    let nodes = tree();
    let all = MapLayers::All(Empty {});
    let frozen = LayerOverride {
        frozen: true,
        ..over("duvar")
    };
    let red = LayerOverride {
        color: Some("#E5484D".into()),
        line_weight: Some(700),
        line_pattern: Some("EKSEN".into()),
        ..over("mobilya")
    };
    let out = map_layers(&nodes, &[], &all, &[frozen, red]);
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].id, "mobilya");
    assert_eq!(out[0].style.color, "#E5484D");
    assert_eq!(out[0].style.line_weight, 0.7);
    assert_eq!(out[0].style.line_pattern.as_deref(), Some("EKSEN"));
    // A layer not printed in the drawing is printed in a map that says so; a built-in line type clears a pattern.
    let mut quiet = tree();
    quiet[0].plot = Some(false);
    quiet[0].style.line_pattern = Some("SINIR".into());
    let printed = LayerOverride {
        plot: Some(true),
        line_type: Some("dashed".into()),
        ..over("duvar")
    };
    let out = map_layers(&quiet, &[], &all, &[printed]);
    assert_eq!(
        out.iter().map(|n| n.id.as_str()).collect::<Vec<_>>(),
        ["mobilya", "duvar"]
    );
    assert_eq!(out[1].style.line_pattern, None);
    assert_eq!(out[1].style.line_type, kentos_contracts::LineType::Dashed);
    assert_eq!(quiet[0].style.line_pattern.as_deref(), Some("SINIR"));
}
