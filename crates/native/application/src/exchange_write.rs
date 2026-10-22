//! Layers and block definitions of another drawing written into this one (docs/adr/0193, 0199 §7, 0309): what
//! Başka çizimden al, Katman olarak ekle, Dosyadan blok ekle and Yapıştır write after the exchange's rules
//! (`kentos_domain::exchange`) made the drawing they bring in.

use kentos_commands::{BlockDefinition, Entity, LayerNode, LayerNodeType};
use kentos_domain::NewLayer;

/// What went into the drawing as one undo step.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct Counts {
    pub layers: usize,
    pub blocks: usize,
    pub redefined: usize,
}

/// The layers of `nodes` the drawing has not, added under `parent`, and the
/// looks of those it has, in the open group (the web's `putLayersAndBlocks`).
pub fn put_nodes(
    model: &mut kentos_domain::Document,
    nodes: &[LayerNode],
    parent: Option<&str>,
    counts: &mut Counts,
    label: &str,
) -> Result<(), String> {
    for n in nodes {
        match model.layers().get(&n.id).map(|m| m.style.clone()) {
            None => {
                let new = NewLayer {
                    frozen: None,
                    plot: None,
                    id: Some(n.id.clone()),
                    name: n.name.clone(),
                    kind: n.kind,
                    visible: n.visible,
                    locked: n.locked,
                    style: n.style.clone(),
                    snap: n.snap.clone(),
                    fields: n.fields.clone(),
                    service: n.service.clone(),
                    feed: n.feed.clone(),
                    // Its time setting comes along; a scenario's links name the other drawing's
                    // layers, so the taken nodes are plain ones (docs/adr/0210 §2).
                    time: n.time.clone(),
                    scenario: None,
                    replaces: None,
                    // Its actions and card name only its own objects' values (docs/adr/0225 §4).
                    actions: n.actions.clone(),
                    card: n.card.clone(),
                    compose: n.compose.clone(),
                    filter: None,
                    // Where its objects came from stays with them (docs/adr/0583 §1).
                    source_crs: n.source_crs.clone(),
                };
                model.add_layer(new, parent, false).map_err(|r| r.0)?;
                counts.layers += 1;
            }
            Some(style) => {
                if style != n.style {
                    model.set_layer_style(&n.id, n.style.clone(), label);
                }
            }
        }
        let inside = if n.kind == LayerNodeType::Group {
            Some(n.id.as_str())
        } else {
            parent
        };
        put_nodes(model, &n.children, inside, counts, label)?;
    }
    Ok(())
}

/// Whether `nodes` would add a node to the drawing's tree or change a node's look.
pub fn changes_tree(model: &kentos_domain::Document, nodes: &[LayerNode]) -> bool {
    nodes.iter().any(|n| match model.layers().get(&n.id) {
        None => true,
        Some(m) => m.style != n.style || changes_tree(model, &n.children),
    })
}

/// The blocks of `blocks` the drawing has not, and the met ones redefined,
/// each once the blocks it places are there (the web's `putLayersAndBlocks`).
pub fn put_blocks(
    model: &mut kentos_domain::Document,
    blocks: &[BlockDefinition],
    counts: &mut Counts,
) -> Result<(), String> {
    let mut pending: Vec<BlockDefinition> = blocks.to_vec();
    let mut guard = pending.len() + 1;
    while !pending.is_empty() && guard > 0 {
        guard -= 1;
        let mut i = 0;
        while i < pending.len() {
            let b = &pending[i];
            // A definition may place only blocks the drawing has (docs/adr/0144): a redefinition waits too.
            let ready = b.entities.iter().all(|e| match e {
                Entity::Insert(ins) => ins.block == b.id || model.block(ins.block).is_some(),
                _ => true,
            });
            if !ready {
                i += 1;
                continue;
            }
            let b = pending.remove(i);
            if model.block(b.id).is_some() {
                if model.update_block(b).map_err(|r| r.0)? {
                    counts.redefined += 1;
                }
            } else {
                model.add_block(b).map_err(|r| r.0)?;
                counts.blocks += 1;
            }
        }
    }
    Ok(())
}
