//! Whether a layer can be a network's edges (gis-usage-report GIS-013, docs/adr/1079): a line layer can, an empty one
//! too (a field team's template, drafted before its first line); an area, point or annotation layer cannot; a layer of
//! any kind can when it holds lines, its other objects left out as the network says. The window proposes a fitting
//! layer for a new network and Kaydet refuses edges that cannot be any, naming the layer and why.

use kentos_commands::LayerGeometry;
use kentos_domain::Document;

/// The kinds a network takes as edges.
pub(crate) const EDGE_KINDS: [&str; 3] = ["line", "polyline", "arc"];

/// How a layer fits as a network's edges.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum Fit {
    /// It holds lines.
    Lines,
    /// It holds nothing yet, of a type that takes lines (or any kind): a template.
    Empty,
    /// It cannot give an edge, and why.
    No(String),
}

/// How the layer `id` fits.
pub(crate) fn fit(doc: &Document, id: &str) -> Fit {
    let Some(node) = doc.layers().get(id) else {
        return Fit::No(format!("“{id}” katmanı çizimde yok."));
    };
    if let Some(g) = node.geometry.filter(|g| *g != LayerGeometry::Line) {
        return Fit::No(format!(
            "“{}” bir {}; ağın kenarları çizgi ister. Bir çizgi katmanı seçin.",
            node.name,
            g.layer_words()
        ));
    }
    let mut lines = 0usize;
    let mut other = 0usize;
    for (_, e) in doc.by_layer_with_uids(id) {
        if EDGE_KINDS.contains(&e.kind()) {
            lines += 1;
        } else {
            other += 1;
        }
    }
    match (lines, other) {
        (0, 0) => Fit::Empty,
        (0, n) => Fit::No(format!(
            "“{}” katmanında çizgi yok ({n} nesnenin hiçbiri çizgi, çoklu çizgi ya da yay değil); ağın kenarı olamaz. Bir çizgi katmanı seçin.",
            node.name
        )),
        _ => Fit::Lines,
    }
}

/// The layer a new network's edges are proposed on: the active one when it holds lines, else the first line layer
/// that does, else the first that holds lines, else the first empty line layer; none when no layer fits.
pub(crate) fn proposed(doc: &Document, active: &str) -> Option<String> {
    if fit(doc, active) == Fit::Lines {
        return Some(active.to_owned());
    }
    let leaves = doc.layers().leaves();
    let typed = |n: &kentos_commands::LayerNode| n.geometry == Some(LayerGeometry::Line);
    leaves
        .iter()
        .find(|n| typed(n) && fit(doc, &n.id) == Fit::Lines)
        .or_else(|| leaves.iter().find(|n| fit(doc, &n.id) == Fit::Lines))
        .or_else(|| {
            leaves
                .iter()
                .find(|n| typed(n) && fit(doc, &n.id) == Fit::Empty)
        })
        .map(|n| n.id.clone())
}
