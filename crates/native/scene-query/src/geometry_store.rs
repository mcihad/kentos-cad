//! The shared geometry store of a document, for the commands that ask which
//! objects are shown where (Buda's and Uzat's boundaries, docs/adr/0250)
//! on a host that keeps none: the headless server, Python and MCP. It holds
//! what the desktop's store holds for the question: every object with its
//! layer, the blocks' definitions, the layers' visibility and lock, and the
//! layer filters' marks (docs/adr/0211). A time slice is a host's session
//! (docs/adr/0210): without one every time is shown. The desktop gives its
//! own store, kept in step with the document (`ExecutionContext::with_store`).

use kentos_commands::{Entity, LayerNode};
use kentos_domain::{Document, LayerTree};
use kentos_geometry_core::entity::Shape;
use kentos_geometry_core::store::{LayerFlags, Store};

use crate::geometry::{drawing_font, shape};
use crate::layer_filter::left_out;

/// An object as the store takes it: its slot, its layer, whether it has a
/// label of its own, and its shape (the desktop's `Spatial` puts the same).
pub fn record(entity: &Entity) -> (f64, &str, bool, Shape) {
    let base = entity.base();
    let label = base.label.as_deref().is_some_and(|l| !l.is_empty());
    (
        f64::from(base.id),
        base.layer_id.as_str(),
        label,
        shape(entity),
    )
}

/// The walls the openings of `put` are in, by the store's ids (docs/adr/0341 §2): an opening that is broken or whose
/// wall the document lacks is in none. The store takes them after it puts the openings ([`Store::set_hosts`]).
pub fn opening_hosts<'a>(
    doc: &Document,
    put: impl IntoIterator<Item = &'a Entity>,
) -> Vec<(f64, Option<f64>)> {
    put.into_iter()
        .filter_map(|e| match e {
            Entity::Opening(o) => {
                let host = (o.opening.broken.is_none())
                    .then(|| kentos_domain::Uuid::parse_str(&o.opening.host).ok())
                    .flatten()
                    .and_then(|u| doc.slot_of(u))
                    .filter(|s| matches!(doc.get(*s), Some(Entity::Wall(_))))
                    .map(|s| f64::from(s.0));
                Some((f64::from(o.base.id), host))
            }
            _ => None,
        })
        .collect()
}

/// Every node of the layer tree with its visibility and lock resolved
/// through the groups above it, in tree order, with the canonical per-layer snap mask.
fn layer_rows(tree: &LayerTree) -> Vec<(String, LayerFlags)> {
    fn walk(nodes: &[LayerNode], tree: &LayerTree, out: &mut Vec<(String, LayerFlags)>) {
        for node in nodes {
            out.push((
                node.id.clone(),
                LayerFlags {
                    visible: tree.is_visible(&node.id),
                    locked: tree.is_locked(&node.id),
                    pick_interior: node.style.pick_interior != Some(false),
                    snap: node.snap.as_ref().map_or(u32::MAX,|s|kentos_geometry_core::store::snap::layer_mask(s.off,s.kinds.as_deref().unwrap_or_default())),
                },
            ));
            walk(&node.children, tree, out);
        }
    }
    let mut out = Vec::new();
    walk(tree.nodes(), tree, &mut out);
    out
}

/// The store of `doc` as it is now.
pub fn store_of(doc: &Document) -> Store {
    store_for_viewport(doc, None)
}

pub fn store_for_viewport(
    doc: &Document,
    scope: Option<&kentos_sheet::model_scope::Viewport>,
) -> Store {
    let mut store = Store::new();
    store.set_font(drawing_font(doc.settings().drawing_font));
    store.set_blocks(crate::blocks::drawing_blocks(doc));
    store.set_wall_view(scope.map_or_else(
        || crate::walls::view_of(doc.settings()),
        |v| crate::walls::view_at(doc.settings(), v.denominator),
    ));
    // A tagged space's tag is its label (docs/adr/0344's 27 October addendum): sent before the objects.
    store.set_tagged_spaces(
        &doc.tagged_spaces()
            .into_iter()
            .map(|s| f64::from(s.0))
            .collect::<Vec<_>>(),
    );
    store
        .put_many(doc.entities().filter_map(|e| {
            crate::viewport_scope::record_at(e, scope.map(|s| s.annotation_scale), doc.settings())
        }));
    store.set_hosts(opening_hosts(doc, doc.entities()));
    // The masks are picked by their boundary only (docs/adr/0310 §4).
    store.set_masks(
        doc.entities()
            .filter(|e| e.base().look.wipeout.is_some())
            .map(|e| (f64::from(e.base().id), true)),
    );
    let mut rows = layer_rows(doc.layers());
    if let Some(scope) = scope {
        let visible = crate::viewport_scope::visible_layers(doc, scope);
        for (id, flags) in &mut rows {
            flags.visible = visible.contains(id);
        }
    }
    store.set_layers(rows.iter().map(|(id, flags)| (id.as_str(), *flags)));
    let out = left_out(doc)
        .into_iter()
        .map(|s| f64::from(s.0))
        .collect::<Vec<_>>();
    if !out.is_empty() {
        store.set_filtered(out.into_iter().map(|s| (s, true)));
    }
    // The design options not shown (docs/adr/0788 §4): neither drawn, picked nor snapped to; their walls join
    // nothing and their openings cut nothing. A paper map's phase view hides walls and openings so too
    // (docs/adr/8039 §2).
    let options: Vec<f64> = doc
        .option_out()
        .into_iter()
        .map(|s| f64::from(s.0))
        .collect();
    let mut out = options.clone();
    if scope.is_some() {
        let view = doc
            .settings()
            .arch
            .as_ref()
            .and_then(|a| a.phase_view)
            .unwrap_or_default();
        out.extend(crate::viewport_scope::phase_out(doc, view));
    }
    if !out.is_empty() {
        store.set_option_out(out);
    }
    let mut hidden = if scope.is_some() {
        crate::viewport_scope::hidden_entities(doc)
    } else {
        options
    };
    hidden.extend(
        doc.entities()
            .filter(|e| !doc.isolation_shown(kentos_domain::Slot(e.base().id)))
            .map(|e| f64::from(e.base().id)),
    );
    store.set_phase_hidden(hidden);
    if let Some(scope) = scope {
        crate::viewport_scope::apply_boundary(&mut store, scope);
    }
    store
}
