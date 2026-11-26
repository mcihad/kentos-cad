//! The style library's items a drawing refers to (docs/adr/9317): what a save and a print take along into the project
//! from Kitaplığım, and what Ön denetim looks for. Its objects' and its blocks' objects' own symbols, its layers' and its
//! layer states' renderers (`{"ref": id}` anywhere in them), and what its own library's items draw with (a template's
//! symbol, a symbol's drawings and markers).

use std::collections::BTreeSet;

use kentos_commands::LayerNode;
use kentos_domain::Document;
use kentos_style_library::library::{assets_of_symbol, refs_in};
use serde_json::Value;

/// The library ids `doc` refers to, each once.
pub fn library_refs(doc: &Document) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let blocks = doc.blocks().iter().flat_map(|b| b.entities.iter());
    for e in doc.entities().chain(blocks) {
        if let Some(id) = e.base().symbol.as_deref().filter(|id| !id.is_empty()) {
            out.insert(id.to_owned());
        }
    }
    fn layers(nodes: &[LayerNode], out: &mut BTreeSet<String>) {
        for n in nodes {
            if let Some(r) = &n.style.renderer {
                out.extend(refs_in(r));
            }
            layers(&n.children, out);
        }
    }
    layers(doc.layers().nodes(), &mut out);
    for n in doc.settings().layer_states.iter().flat_map(|s| &s.nodes) {
        if let Some(r) = n.style.as_ref().and_then(|s| s.renderer.as_ref()) {
            out.extend(refs_in(r));
        }
    }
    for item in &doc.styles().items {
        if let Some(id) = item.pointer("/template/symbol").and_then(Value::as_str) {
            out.insert(id.to_owned());
        }
        if let Some(symbol) = item.get("symbol") {
            out.extend(refs_in(symbol));
            out.extend(assets_of_symbol(symbol));
        }
    }
    out
}

/// The library ids a layer draws with: its renderer's and its objects' own.
pub fn layer_refs(doc: &Document, layer: &LayerNode) -> BTreeSet<String> {
    let mut out: BTreeSet<String> = layer.style.renderer.as_ref().map(refs_in).unwrap_or_default().into_iter().collect();
    for e in doc.by_layer(&layer.id) {
        if let Some(id) = e.base().symbol.as_deref().filter(|id| !id.is_empty()) {
            out.insert(id.to_owned());
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    /// docs/adr/9317: a layer's renderer, a layer state's, an object's own symbol (a block's too) and the project
    /// library's items' needs are what a drawing refers to.
    #[test]
    fn a_drawing_refers_to_its_layers_objects_and_library_items() {
        let fx: Value = serde_json::from_str(include_str!("../../../../../fixtures/sheet/v1/native-preflight.json"))
            .expect("the fixture");
        let mut raw = fx["model"].clone();
        let style = |renderer: Value| json!({ "color": "fg", "lineType": "continuous", "lineWeight": 0.25, "renderer": renderer });
        raw["layers"] = json!([{ "id": "a", "name": "A", "type": "layer", "visible": true, "locked": false, "expanded": true,
            "style": style(json!({ "type": "categorized", "field": "t", "classes": [
                { "value": "x", "label": "x", "symbols": { "fill": { "ref": "u-sinif" } } }] })), "children": [] }]);
        raw["activeLayer"] = json!("a");
        raw["settings"]["layerStates"] = json!([{ "id": "d", "name": "Durum", "nodes": [{ "node": "a", "visible": true,
            "style": style(json!({ "type": "single", "symbols": { "fill": { "ref": "u-durum" } } })) }] }]);
        raw["entities"] = json!([{ "kind": "point", "id": 1, "layerId": "a", "attrs": {}, "p": { "x": 0, "y": 0 }, "symbol": "u-nesne" }]);
        raw["blocks"] = json!([{ "id": "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d6101", "name": "B", "base": { "x": 0, "y": 0 }, "units": "m",
            "entities": [{ "kind": "point", "id": 1, "layerId": "", "attrs": {}, "p": { "x": 0, "y": 0 }, "symbol": "u-blok" }] }]);
        raw["styles"] = json!({ "items": [
            { "kind": "template", "id": "p-t", "name": "Şablon", "path": [], "template": { "kind": "point", "symbol": "u-sablon" } },
            { "kind": "symbol", "id": "p-s", "name": "S", "path": [], "symbol": { "type": "marker", "layers": [
                { "id": "v", "type": "svg", "asset": "u-cizim", "size": 4 }] } }], "categories": [] });
        let doc = Document::from_snapshot(serde_json::from_value(raw).expect("a snapshot")).expect("opens");
        let refs: Vec<String> = library_refs(&doc).into_iter().collect();
        assert_eq!(refs, ["u-blok", "u-cizim", "u-durum", "u-nesne", "u-sablon", "u-sinif"]);
        let layer = doc.layers().nodes()[0].clone();
        let mine: Vec<String> = layer_refs(&doc, &layer).into_iter().collect();
        assert_eq!(mine, ["u-nesne", "u-sinif"]);
    }
}
