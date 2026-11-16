//! An object's materials by the names the file gives them (`IfcRelAssociatesMaterial`): a material's own name, a layer
//! set's, a usage's set's, or its members' (layers, constituents, profiles, a list). The viewer's details and the
//! written file's facts read them.

use ifc_lite_core::{AttributeValue, EntityDecoder, SchemaVersion};

use super::graph::named;
use super::{Model, ReadError, guarded, header};

impl Model {
    /// The materials of object `object` (an index into [`Model::objects`]), each association's names; `bytes` are the
    /// bytes the model was read from.
    pub fn materials(&self, bytes: &[u8], object: usize) -> Result<Vec<String>, ReadError> {
        let Some(o) = self.objects.get(object) else { return Ok(Vec::new()) };
        let schema = header::version(self.summary.family);
        guarded(|| {
            let mut decoder = EntityDecoder::with_arc_index(bytes, self.index.clone());
            Ok(self
                .relations_of(o.step)
                .iter()
                .map(|&r| &self.relations[r])
                .filter(|r| schema.is_subtype_of(&r.class, "IfcRelAssociatesMaterial") && r.related.contains(&o.step))
                .filter_map(|r| material_name(&mut decoder, schema, r.relating?, 0))
                .collect())
        })
    }
}

/// A material record's name: its own, its layer set's, a usage's set's, or its members' names.
fn material_name(decoder: &mut EntityDecoder, schema: SchemaVersion, step: u32, depth: usize) -> Option<String> {
    if depth > 3 {
        return None;
    }
    let e = decoder.decode_by_id(step).ok()?;
    let text = |name: &str| named(schema, &e, name).and_then(AttributeValue::as_string).filter(|s| !s.trim().is_empty());
    if let Some(n) = text("Name").or_else(|| text("LayerSetName")) {
        return Some(n.to_owned());
    }
    for usage in ["ForLayerSet", "ForProfileSet"] {
        if let Some(set) = named(schema, &e, usage).and_then(AttributeValue::as_entity_ref) {
            return material_name(decoder, schema, set, depth + 1);
        }
    }
    let members: Vec<u32> = ["Materials", "MaterialLayers", "MaterialConstituents", "MaterialProfiles"]
        .iter()
        .find_map(|n| named(schema, &e, n).and_then(AttributeValue::as_list))
        .map(|l| l.iter().filter_map(AttributeValue::as_entity_ref).collect())
        .unwrap_or_default();
    let names: Vec<String> = members
        .into_iter()
        .filter_map(|m| {
            let part = decoder.decode_by_id(m).ok()?;
            match named(schema, &part, "Material").and_then(AttributeValue::as_entity_ref) {
                Some(material) => material_name(decoder, schema, material, depth + 1),
                None => named(schema, &part, "Name").and_then(AttributeValue::as_string).map(str::to_owned),
            }
        })
        .collect();
    (!names.is_empty()).then(|| names.join(", "))
}
