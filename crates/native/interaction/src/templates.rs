//! Drawing with an object template (docs/adr/0176 §3): the layer a template
//! draws on and what every object drawn with it takes. The template itself is
//! the style library's (`kentos_native_style::object_template`); the host
//! reads it, finds or opens its layer here, makes it active, takes the
//! template's colour and weight into the draft and starts its tool with the
//! [`Stamp`] in the tools' context. The web's rules are
//! `apps/web/src/model/objectTemplate.ts`'s `templateLayer`; both pass
//! `fixtures/style/v1/template-layers.json`.

use std::collections::BTreeMap;

use kentos_contracts::{LayerNode, LayerNodeType, LineType};
use kentos_domain::{Document, NewLayer, Refusal, labels};

/// What every object drawn with a template takes besides its geometry: its
/// symbol (a library symbol's id), its attributes and its label.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Stamp {
    pub symbol: Option<String>,
    pub attrs: BTreeMap<String, String>,
    pub label: Option<String>,
}

/// The layer a template's objects go on: found by its name, opened under
/// its path with this look when the drawing lacks it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct TemplateLayer {
    /// The groups above it, from the top; empty: at the top.
    pub path: Vec<String>,
    pub name: String,
    /// Its look when it is opened; absent parts are a new layer's.
    pub color: Option<String>,
    pub line_type: Option<LineType>,
    pub line_weight: Option<f64>,
}

/// Where a template draws in a drawing's layer tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayerAnswer {
    /// This layer, by its id.
    Found(String),
    /// This node, by its id, is locked (by itself or a group above it): the
    /// template does not start.
    Locked(String),
    /// The drawing lacks it: it is opened under `parent` (a group's id; none:
    /// at the top), inside the groups `create` names, made in this order.
    Open {
        parent: Option<String>,
        create: Vec<String>,
    },
}

/// Where a template with `layer` draws in the tree `nodes` (the top level),
/// `locked` saying whether a node is locked by itself or a group above it.
/// The layer is found by its name: of several, the one under the template's
/// path, else the first in the tree. Without one, the path's groups are
/// followed as far as the tree has them (the first group of each name), and
/// the rest are opened. Names are compared without the white space around them.
pub fn find_layer(
    nodes: &[LayerNode],
    layer: &TemplateLayer,
    locked: impl Fn(&str) -> bool,
) -> LayerAnswer {
    let name = layer.name.trim();
    let path: Vec<&str> = layer.path.iter().map(|g| g.trim()).collect();
    let mut found: Vec<(&LayerNode, Vec<&str>)> = Vec::new();
    collect(nodes, &mut Vec::new(), name, &mut found);
    if !found.is_empty() {
        let chosen = found
            .iter()
            .find(|(_, groups)| *groups == path)
            .unwrap_or(&found[0])
            .0;
        return if locked(&chosen.id) {
            LayerAnswer::Locked(chosen.id.clone())
        } else {
            LayerAnswer::Found(chosen.id.clone())
        };
    }
    let mut level = nodes;
    let mut parent: Option<&LayerNode> = None;
    let mut depth = 0;
    for group in &path {
        let Some(next) = level
            .iter()
            .find(|n| n.kind == LayerNodeType::Group && n.name.trim() == *group)
        else {
            break;
        };
        parent = Some(next);
        level = &next.children;
        depth += 1;
    }
    if let Some(group) = parent
        && locked(&group.id)
    {
        return LayerAnswer::Locked(group.id.clone());
    }
    LayerAnswer::Open {
        parent: parent.map(|g| g.id.clone()),
        create: path[depth..].iter().map(|g| (*g).to_owned()).collect(),
    }
}

/// The layers named `name` under `nodes`, in the tree's order, each with the
/// names of the groups above it.
fn collect<'a>(
    nodes: &'a [LayerNode],
    groups: &mut Vec<&'a str>,
    name: &str,
    out: &mut Vec<(&'a LayerNode, Vec<&'a str>)>,
) {
    for node in nodes {
        match node.kind {
            LayerNodeType::Layer if node.name.trim() == name => {
                out.push((node, groups.clone()));
            }
            LayerNodeType::Group => {
                groups.push(node.name.trim());
                collect(&node.children, groups, name, out);
                groups.pop();
            }
            LayerNodeType::Layer => {}
        }
    }
}

/// Opens a template's layer where [`find_layer`] said, inside the groups it
/// names, with the template's look, and makes it active: one undo step
/// “Katman ekle” (the user sees where the objects will go). Returns the new
/// layer's id; a refusal changes nothing.
pub fn open_layer(
    doc: &mut Document,
    layer: &TemplateLayer,
    parent: Option<&str>,
    create: &[String],
) -> Result<String, Refusal> {
    let group = doc.begin_group(labels::LAYER_ADD);
    let opened = (|| {
        let mut under = parent.map(str::to_owned);
        for name in create {
            under = Some(doc.add_layer(NewLayer::group(name.clone()), under.as_deref(), false)?);
        }
        let mut new = NewLayer::layer(layer.name.trim());
        if let Some(color) = &layer.color {
            new.style.color = color.clone();
        }
        if let Some(line_type) = layer.line_type {
            new.style.line_type = line_type;
        }
        if let Some(weight) = layer.line_weight {
            new.style.line_weight = weight;
        }
        doc.add_layer(new, under.as_deref(), true)
    })();
    match opened {
        Ok(id) => {
            doc.end_group(group);
            Ok(id)
        }
        Err(refusal) => {
            doc.cancel_group(group);
            Err(refusal)
        }
    }
}

/// What is said when a template's layer, or the group it would be opened
/// in, is locked: the node by its name, the template by its.
pub fn locked_text(node: &LayerNode, template: &str) -> String {
    let what = match node.kind {
        LayerNodeType::Group => "grubu",
        LayerNodeType::Layer => "katmanı",
    };
    format!(
        "“{}” {what} kilitli; “{template}” şablonu bu katmana çizer. Kilidi Katmanlar panelinden açın.",
        node.name
    )
}
