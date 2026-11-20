//! A run's new layer in a GIS project (docs/adr/1070): the layer type that takes what the run writes to it and the
//! fields of the values its objects carry, so that the result keeps the rules of the layers it was made from.

use std::collections::BTreeMap;

use kentos_commands::Entity;
use kentos_commands::LayerGeometry;
use kentos_commands::fields::LayerField;
use kentos_domain::Document;

/// The order a layer type is tried in: closed curves alone (a disc's buffer) are areas before lines.
const TRIED: [LayerGeometry; 4] = [
    LayerGeometry::Point,
    LayerGeometry::Area,
    LayerGeometry::Line,
    LayerGeometry::Annotation,
];

/// Whether the drawing is a GIS project: only its layers are typed (a CAD layer holds any kind).
pub fn is_gis(doc: &Document) -> bool {
    doc.settings().workspace == Some(kentos_commands::Workspace::Gis)
}

/// The type of the layer `id` from the objects the run adds to it: the first that takes them all; none when the run
/// adds nothing there or no one type takes everything (the layer stays Karışık).
pub fn geometry_of(id: &str, added: &[Entity]) -> Option<LayerGeometry> {
    let kinds: Vec<&str> = added
        .iter()
        .filter(|e| e.base().layer_id == id)
        .map(Entity::kind)
        .collect();
    if kinds.is_empty() {
        return None;
    }
    TRIED
        .into_iter()
        .find(|g| kinds.iter().all(|k| g.admits(k)))
}

/// Why the run cannot write what it adds to the existing layers it names: the first object of a kind its layer's
/// type does not take; none when every one fits (a layer the run makes is typed by what it holds).
pub fn misfit(doc: &Document, added: &[Entity]) -> Option<String> {
    added.iter().find_map(|e| {
        let node = doc.layers().get(&e.base().layer_id)?;
        let holds = node.geometry?;
        (!holds.admits(e.kind())).then(|| {
            format!(
                "“{}” bir {}; işlemin yazdığı {} bu katmana yazılmaz, hiçbir şey yazılmadı. Çıktıyı yeni bir katmana ya da türüne uyan bir katmana yazın.",
                node.name,
                holds.layer_words(),
                kentos_commands::kind_words(e.kind())
            )
        })
    })
}

/// The fields of the layer `id` from the values its new objects carry: a source layer's field of that name as it is
/// (its type, shown name, range and other rules), unique only while the run's values do not repeat, in the order the
/// source layers list them; then the tool's own (`declared`) of the values it wrote. A value of neither stays a plain
/// value, as before.
pub fn fields_of(
    doc: &Document,
    id: &str,
    added: &[Entity],
    sources: &[String],
    declared: &[LayerField],
) -> Vec<LayerField> {
    let mut values: BTreeMap<&str, Vec<&str>> = BTreeMap::new();
    for e in added.iter().filter(|e| e.base().layer_id == id) {
        for (k, v) in &e.base().attrs {
            values.entry(k.as_str()).or_default().push(v.as_str());
        }
    }
    let mut out: Vec<LayerField> = Vec::new();
    let taken = |out: &[LayerField], name: &str| out.iter().any(|f| f.name == name);
    let sourced = sources
        .iter()
        .filter_map(|s| doc.layers().get(s))
        .flat_map(|node| node.fields.iter());
    for field in sourced.chain(declared) {
        let Some(written) = values.get(field.name.as_str()) else {
            continue;
        };
        if taken(&out, &field.name) {
            continue;
        }
        let mut field = field.clone();
        if field.unique && repeats(written) {
            field.unique = false;
        }
        out.push(field);
    }
    out
}

/// Whether a value comes twice among `values` (empty ones aside: unique leaves empties alone).
fn repeats(values: &[&str]) -> bool {
    let mut seen = std::collections::HashSet::new();
    values
        .iter()
        .filter(|v| !v.trim().is_empty())
        .any(|v| !seen.insert(v.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repeats_leaves_empties_alone() {
        assert!(!repeats(&["ANK-01", "", "", "ANK-02"]));
        assert!(repeats(&["ANK-01", "ANK-01 "]));
    }
}
