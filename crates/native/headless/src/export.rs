//! A drawing as a DXF file (docs/adr/0252): the objects by layer in the
//! tree's order and the writer's input as the desktop's DXF dışa aktar
//! builds it — layers with their groups, a dimension without its own text
//! with the value its look writes, the blocks the objects place, a local
//! project's unit, the project's text and dimension styles — so a script, an
//! agent and the window write the same file (`kentos_formats::dxf::write`).

use std::collections::BTreeMap;

use kentos_contracts::{AngleUnit, DxfWriteInput, DxfWriteLayer, Entity, LayerNodeType};
use kentos_domain::Document;
use kentos_project::format::Format;

/// Objects by layer: the tree's order, then layers the tree does not have
/// (their objects go to DXF layer 0), empty ones left out.
pub fn by_layer<'a>(
    doc: &Document,
    objects: impl IntoIterator<Item = &'a Entity>,
) -> Vec<(String, Vec<&'a Entity>)> {
    let mut out: Vec<(String, Vec<&Entity>)> = doc
        .layers()
        .leaves()
        .iter()
        .map(|l| (l.id.clone(), Vec::new()))
        .collect();
    let mut at: BTreeMap<String, usize> = out
        .iter()
        .enumerate()
        .map(|(i, (id, _))| (id.clone(), i))
        .collect();
    for e in objects {
        let id = &e.base().layer_id;
        match at.get(id) {
            Some(&i) => out[i].1.push(e),
            None => {
                at.insert(id.clone(), out.len());
                out.push((id.clone(), vec![e]));
            }
        }
    }
    out.retain(|(_, list)| !list.is_empty());
    out
}

/// A layer as the writer receives it (the groups above it, outermost first).
fn write_layer(doc: &Document, id: &str) -> Option<DxfWriteLayer> {
    let layers = doc.layers();
    let l = layers.get(id)?;
    if l.kind != LayerNodeType::Layer {
        return None;
    }
    let mut path = Vec::new();
    let mut up = layers.parent(id);
    while let Some(group) = up {
        path.insert(0, group.name.clone());
        up = layers.parent(&group.id);
    }
    Some(DxfWriteLayer {
        plot: (!layers.printed(id)).then_some(false),
        id: id.to_owned(),
        name: l.name.clone(),
        path,
        color: l.style.color.clone(),
        visible: layers.is_visible(id),
        locked: layers.is_locked(id),
        line_type: l.style.line_type,
        line_weight: l.style.line_weight,
    })
}

/// The writer's input for `chosen` (from [`by_layer`]).
pub fn dxf_input(doc: &Document, chosen: &[(String, Vec<&Entity>)]) -> DxfWriteInput {
    let entities: Vec<Entity> = chosen
        .iter()
        .flat_map(|(_, list)| list.iter().map(|e| (*e).clone()))
        .collect();
    let settings = doc.settings();
    let format = Format::of(settings);
    // What a dimension without its own text shows (project units): its
    // block draws it, the dimension keeps no text.
    let mut dimension_values = BTreeMap::new();
    for e in &entities {
        if let Entity::Dimension(d) = e
            && d.text.as_deref().is_none_or(str::is_empty)
            && let Some(l) = kentos_formats::dxf::dimension_layout(d)
        {
            dimension_values.insert(
                d.base.id,
                format.dimension_in(l.prefix, l.unit, l.value, &d.look),
            );
        }
    }
    DxfWriteInput {
        entities,
        layers: chosen
            .iter()
            .filter_map(|(id, _)| write_layer(doc, id))
            .collect(),
        scale: settings.plot_scale,
        length_decimals: settings.length_decimals,
        grads: settings.angle_unit == AngleUnit::Grad,
        dimension_values,
        // The writer takes those the objects place (docs/adr/0144 §5).
        blocks: doc.blocks().iter().map(|b| (**b).clone()).collect(),
        // A local project's drawing goes out in its unit (docs/adr/0165 §2).
        unit: (!settings.has_system()).then(|| settings.unit()),
        // The project's styles as STYLE and DIMSTYLE records (docs/adr/0183 §7).
        text_styles: settings.text_styles.clone(),
        dimension_styles: settings.dimension_styles.clone(),
    }
}
