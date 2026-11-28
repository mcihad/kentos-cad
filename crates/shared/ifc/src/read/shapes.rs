//! The kinds of an object's shape as the file writes them (docs/adr/8133; Hazırlama Yönetmeliği Md. 14, EK-8, EK-9/7):
//! each shape representation's identifier and type (`IfcShapeRepresentation`'s `RepresentationIdentifier`,
//! `RepresentationType`), a mapped one's as the representation it maps, and for a tessellated one whether its face sets
//! say they close. The form's 7th question reads the level of development from them: an element's body a solid, not a
//! bounding box (IFC: "simplistic 3D representation by a bounding box"). Nothing is measured from triangles here.

use ifc_lite_core::{AttributeValue, EntityDecoder, SchemaVersion};

use super::Model;
use super::graph::named;

/// How deep mapped representations are followed (a map of a map …).
const MAPS: usize = 4;

/// One representation of an object's shape: its identifier (`Body`, `Axis`, `Box` …) and type (`SweptSolid`, `Brep`,
/// `BoundingBox` …) as written, a mapped one's those of the representation it maps; for a tessellated one, whether its
/// face sets all say they are closed (`Closed`), none when one does not say.
#[derive(Debug, Clone, PartialEq)]
pub struct ShapeKind {
    pub identifier: Option<String>,
    pub kind: Option<String>,
    pub closed: Option<bool>,
}

impl Model {
    /// The representations of object `object`'s shape (its `Representation`'s `Representations`); none when it has no
    /// shape or a record cannot be read.
    pub(super) fn shape_kinds(
        &self,
        decoder: &mut EntityDecoder,
        schema: SchemaVersion,
        object: usize,
    ) -> Vec<ShapeKind> {
        let Some(o) = self.objects.get(object).filter(|o| o.has_shape) else {
            return Vec::new();
        };
        let Ok(product) = decoder.decode_by_id(o.step) else {
            return Vec::new();
        };
        let Some(shape) =
            named(schema, &product, "Representation").and_then(AttributeValue::as_entity_ref)
        else {
            return Vec::new();
        };
        let Ok(shape) = decoder.decode_by_id(shape) else {
            return Vec::new();
        };
        let reps: Vec<u32> = named(schema, &shape, "Representations")
            .and_then(AttributeValue::as_list)
            .map(|l| l.iter().filter_map(AttributeValue::as_entity_ref).collect())
            .unwrap_or_default();
        reps.into_iter()
            .filter_map(|r| kind_of(decoder, schema, r, 0))
            .collect()
    }
}

/// A shape representation's identifier and type; a mapped one's type that of the representation its first item maps
/// (its identifier its own), a tessellated one's closure from its items.
fn kind_of(
    decoder: &mut EntityDecoder,
    schema: SchemaVersion,
    step: u32,
    depth: usize,
) -> Option<ShapeKind> {
    let e = decoder.decode_by_id(step).ok()?;
    let text = |name: &str| {
        named(schema, &e, name)
            .and_then(AttributeValue::as_string)
            .map(str::to_owned)
    };
    let identifier = text("RepresentationIdentifier");
    let kind = text("RepresentationType");
    let items: Vec<u32> = named(schema, &e, "Items")
        .and_then(AttributeValue::as_list)
        .map(|l| l.iter().filter_map(AttributeValue::as_entity_ref).collect())
        .unwrap_or_default();
    match kind.as_deref() {
        Some("MappedRepresentation") if depth < MAPS => {
            let mapped = items.first().and_then(|&i| {
                let item = decoder.decode_by_id(i).ok()?;
                let source = named(schema, &item, "MappingSource")
                    .and_then(AttributeValue::as_entity_ref)?;
                let map = decoder.decode_by_id(source).ok()?;
                named(schema, &map, "MappedRepresentation").and_then(AttributeValue::as_entity_ref)
            });
            match mapped.and_then(|m| kind_of(decoder, schema, m, depth + 1)) {
                Some(inner) => Some(ShapeKind {
                    identifier,
                    kind: inner.kind,
                    closed: inner.closed,
                }),
                None => Some(ShapeKind {
                    identifier,
                    kind,
                    closed: None,
                }),
            }
        }
        // Closed when every face set says so; open when one says it is not; else not said.
        Some("Tessellation") => {
            let said: Vec<Option<bool>> = items
                .iter()
                .map(|&i| {
                    decoder
                        .decode_by_id(i)
                        .ok()
                        .and_then(|f| named(schema, &f, "Closed").and_then(AttributeValue::as_bool))
                })
                .collect();
            let closed = if said.contains(&Some(false)) {
                Some(false)
            } else if said.is_empty() || said.contains(&None) {
                None
            } else {
                Some(true)
            };
            Some(ShapeKind {
                identifier,
                kind,
                closed,
            })
        }
        _ => Some(ShapeKind {
            identifier,
            kind,
            closed: None,
        }),
    }
}
