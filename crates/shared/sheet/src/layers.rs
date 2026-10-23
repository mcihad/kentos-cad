//! Effective viewport layer visibility and appearance shared by rendering and model queries.
use std::collections::HashMap;
use kentos_contracts::{LayerNode, LayerNodeType};

pub fn map_layers<'a>(
    nodes: &'a [LayerNode],
    states: &[kentos_contracts::LayerState],
    layers: &crate::kinds::MapLayers,
    overrides: &[crate::kinds::LayerOverride],
) -> Vec<std::borrow::Cow<'a, LayerNode>> {
    use std::borrow::Cow;
    let state = match layers {
        crate::kinds::MapLayers::Theme(t) => states.iter().find(|s| s.name.trim() == t.name.trim()),
        _ => None,
    };
    let kept: HashMap<&str, &kentos_contracts::LayerStateNode> = state
        .map(|s| s.nodes.iter().map(|n| (n.node.as_str(), n)).collect())
        .unwrap_or_default();
    struct Walk<'s> {
        kept: HashMap<&'s str, &'s kentos_contracts::LayerStateNode>,
        overrides: &'s [crate::kinds::LayerOverride],
    }
    fn walk<'a>(w: &Walk<'_>, nodes: &'a [LayerNode], on: bool, out: &mut Vec<Cow<'a, LayerNode>>) {
        for node in nodes {
            let k = w.kept.get(node.id.as_str());
            let visible = k.map_or(node.visible, |k| k.visible);
            let frozen = k.and_then(|k| k.frozen).or(node.frozen) == Some(true);
            let o = w.overrides.iter().find(|o| o.layer == node.id);
            let plot = o
                .and_then(|o| o.plot)
                .or_else(|| k.and_then(|k| k.plot))
                .or(node.plot)
                != Some(false);
            let on = on && visible && plot && !frozen && !o.is_some_and(|o| o.frozen);
            match node.kind {
                LayerNodeType::Group => walk(w, &node.children, on, out),
                LayerNodeType::Layer if on => {
                    let mut style = k
                        .and_then(|k| k.style.clone())
                        .unwrap_or_else(|| node.style.clone());
                    if let Some(o) = o {
                        if let Some(c) = &o.color {
                            style.color.clone_from(c);
                        }
                        if let Some(t) = o
                            .line_type
                            .as_deref()
                            .and_then(|t| serde_json::from_value(serde_json::Value::from(t)).ok())
                        {
                            style.line_type = t;
                            style.line_pattern = None;
                        }
                        if let Some(p) = &o.line_pattern {
                            style.line_pattern = Some(p.clone());
                        }
                        if let Some(w) = o.line_weight {
                            style.line_weight = f64::from(w) / 1000.0;
                        }
                    }
                    if style == node.style {
                        out.push(Cow::Borrowed(node));
                    } else {
                        let mut copy = node.clone();
                        copy.style = style;
                        out.push(Cow::Owned(copy));
                    }
                }
                LayerNodeType::Layer => {}
            }
        }
    }
    let mut out = Vec::new();
    walk(&Walk { kept, overrides }, nodes, true, &mut out);
    out.reverse();
    if let crate::kinds::MapLayers::List(list) = layers {
        out.retain(|n| list.layers.contains(&n.id));
    }
    out
}
