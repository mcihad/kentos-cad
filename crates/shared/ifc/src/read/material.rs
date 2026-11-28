//! An object's materials by the names the file gives them (`IfcRelAssociatesMaterial`): a material's own name, a layer
//! set's, a usage's set's, or its members' (layers, constituents, profiles, a list). The viewer's details and the
//! written file's facts read them. A layer set's layers and how a usage lays them (docs/adr/8166) for the details.

use ifc_lite_core::{AttributeValue, EntityDecoder, SchemaVersion};

use super::graph::named;
use super::{Model, ReadError, guarded, header};

impl Model {
    /// The materials of object `object` (an index into [`Model::objects`]), each association's names; `bytes` are the
    /// bytes the model was read from.
    pub fn materials(&self, bytes: &[u8], object: usize) -> Result<Vec<String>, ReadError> {
        let Some(o) = self.objects.get(object) else {
            return Ok(Vec::new());
        };
        let schema = header::version(self.summary.family);
        guarded(|| {
            let mut decoder = EntityDecoder::with_arc_index(bytes, self.index.clone());
            Ok(self
                .relations_of(o.step)
                .iter()
                .map(|&r| &self.relations[r])
                .filter(|r| {
                    schema.is_subtype_of(&r.class, "IfcRelAssociatesMaterial")
                        && r.related.contains(&o.step)
                })
                .filter_map(|r| material_name(&mut decoder, schema, r.relating?, 0))
                .collect())
        })
    }
}

/// A material layer set as the file writes it (docs/adr/8166): how a usage lays it (its direction, its sense and its
/// base line's offset, as written) and its layers in order, each its name, its material's name and its thickness.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LayerList {
    pub usage: Option<(String, String, f64)>,
    pub layers: Vec<(String, Option<String>, f64)>,
}

impl Model {
    /// The material layer set object `object` (an index into [`Model::objects`]) is associated with, directly or
    /// through a usage; none when its material is another kind.
    pub fn layers(&self, bytes: &[u8], object: usize) -> Result<Option<LayerList>, ReadError> {
        let Some(o) = self.objects.get(object) else {
            return Ok(None);
        };
        let schema = header::version(self.summary.family);
        guarded(|| {
            let mut decoder = EntityDecoder::with_arc_index(bytes, self.index.clone());
            let associations: Vec<u32> = self
                .relations_of(o.step)
                .iter()
                .map(|&r| &self.relations[r])
                .filter(|r| {
                    schema.is_subtype_of(&r.class, "IfcRelAssociatesMaterial")
                        && r.related.contains(&o.step)
                })
                .filter_map(|r| r.relating)
                .collect();
            for m in associations {
                let Ok(e) = decoder.decode_by_id(m) else {
                    continue;
                };
                let mut out = LayerList::default();
                let set = match named(schema, &e, "ForLayerSet")
                    .and_then(AttributeValue::as_entity_ref)
                {
                    Some(set) => {
                        let word = |n: &str| {
                            let v = named(schema, &e, n).and_then(AttributeValue::as_enum);
                            v.map(str::to_owned).unwrap_or_default()
                        };
                        let offset = named(schema, &e, "OffsetFromReferenceLine")
                            .and_then(AttributeValue::as_float);
                        let offset = offset.unwrap_or(0.0);
                        out.usage =
                            Some((word("LayerSetDirection"), word("DirectionSense"), offset));
                        set
                    }
                    None => m,
                };
                let Ok(s) = decoder.decode_by_id(set) else {
                    continue;
                };
                let Some(list) =
                    named(schema, &s, "MaterialLayers").and_then(AttributeValue::as_list)
                else {
                    continue;
                };
                let refs: Vec<u32> = list
                    .iter()
                    .filter_map(AttributeValue::as_entity_ref)
                    .collect();
                for l in refs {
                    let Ok(layer) = decoder.decode_by_id(l) else {
                        continue;
                    };
                    let name = named(schema, &layer, "Name").and_then(AttributeValue::as_string);
                    let name = name.unwrap_or("").to_owned();
                    let thickness = named(schema, &layer, "LayerThickness")
                        .and_then(AttributeValue::as_float)
                        .unwrap_or(0.0);
                    let material = named(schema, &layer, "Material")
                        .and_then(AttributeValue::as_entity_ref)
                        .and_then(|x| decoder.decode_by_id(x).ok())
                        .and_then(|x| {
                            named(schema, &x, "Name")
                                .and_then(AttributeValue::as_string)
                                .map(str::to_owned)
                        });
                    out.layers.push((name, material, thickness));
                }
                return Ok(Some(out));
            }
            Ok(None)
        })
    }
}

/// A section of a material profile set as the file writes it (docs/adr/8167): its profile's name, its kind (the
/// profile's class), its measures (a rectangle's XDim and YDim, a circle's Radius; an outline none) and its material.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ProfileLine {
    pub name: Option<String>,
    pub class: String,
    pub measures: Vec<(&'static str, f64)>,
    pub material: Option<String>,
}

impl Model {
    /// The sections of the material profile set object `object` is associated with, directly or through a usage;
    /// none when its material is another kind.
    pub fn profiles(&self, bytes: &[u8], object: usize) -> Result<Vec<ProfileLine>, ReadError> {
        let Some(o) = self.objects.get(object) else {
            return Ok(Vec::new());
        };
        let schema = header::version(self.summary.family);
        guarded(|| {
            let mut decoder = EntityDecoder::with_arc_index(bytes, self.index.clone());
            let associations: Vec<u32> = self
                .relations_of(o.step)
                .iter()
                .map(|&r| &self.relations[r])
                .filter(|r| {
                    schema.is_subtype_of(&r.class, "IfcRelAssociatesMaterial")
                        && r.related.contains(&o.step)
                })
                .filter_map(|r| r.relating)
                .collect();
            let mut out = Vec::new();
            for m in associations {
                let Ok(e) = decoder.decode_by_id(m) else {
                    continue;
                };
                let set = named(schema, &e, "ForProfileSet")
                    .and_then(AttributeValue::as_entity_ref)
                    .unwrap_or(m);
                let Ok(s) = decoder.decode_by_id(set) else {
                    continue;
                };
                let Some(list) =
                    named(schema, &s, "MaterialProfiles").and_then(AttributeValue::as_list)
                else {
                    continue;
                };
                let refs: Vec<u32> = list
                    .iter()
                    .filter_map(AttributeValue::as_entity_ref)
                    .collect();
                for r in refs {
                    let Ok(mp) = decoder.decode_by_id(r) else {
                        continue;
                    };
                    let material = named(schema, &mp, "Material")
                        .and_then(AttributeValue::as_entity_ref)
                        .and_then(|x| decoder.decode_by_id(x).ok())
                        .and_then(|x| {
                            named(schema, &x, "Name")
                                .and_then(AttributeValue::as_string)
                                .map(str::to_owned)
                        });
                    let Some(p) =
                        named(schema, &mp, "Profile").and_then(AttributeValue::as_entity_ref)
                    else {
                        continue;
                    };
                    let Ok(p) = decoder.decode_by_id(p) else {
                        continue;
                    };
                    let class = p.ifc_type.name().to_owned();
                    let mut measures = Vec::new();
                    for key in ["XDim", "YDim", "Radius"] {
                        if let Some(v) = named(schema, &p, key).and_then(AttributeValue::as_float) {
                            measures.push((key, v));
                        }
                    }
                    let name = named(schema, &p, "ProfileName")
                        .and_then(AttributeValue::as_string)
                        .map(str::to_owned);
                    out.push(ProfileLine {
                        name,
                        class,
                        measures,
                        material,
                    });
                }
            }
            Ok(out)
        })
    }
}

/// A material record's name: its own, its layer set's, a usage's set's, or its members' names.
fn material_name(
    decoder: &mut EntityDecoder,
    schema: SchemaVersion,
    step: u32,
    depth: usize,
) -> Option<String> {
    if depth > 3 {
        return None;
    }
    let e = decoder.decode_by_id(step).ok()?;
    let text = |name: &str| {
        named(schema, &e, name)
            .and_then(AttributeValue::as_string)
            .filter(|s| !s.trim().is_empty())
    };
    if let Some(n) = text("Name").or_else(|| text("LayerSetName")) {
        return Some(n.to_owned());
    }
    for usage in ["ForLayerSet", "ForProfileSet"] {
        if let Some(set) = named(schema, &e, usage).and_then(AttributeValue::as_entity_ref) {
            return material_name(decoder, schema, set, depth + 1);
        }
    }
    let members: Vec<u32> = [
        "Materials",
        "MaterialLayers",
        "MaterialConstituents",
        "MaterialProfiles",
    ]
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
                None => named(schema, &part, "Name")
                    .and_then(AttributeValue::as_string)
                    .map(str::to_owned),
            }
        })
        .collect();
    (!names.is_empty()).then(|| names.join(", "))
}
