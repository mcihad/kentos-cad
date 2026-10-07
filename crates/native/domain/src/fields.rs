//! A layer's fields read from its objects (docs/adr/0199 §3): what the
//! Alanlar window's Verilerden al offers, the keys in the natural order
//! (docs/adr/0153 §6) the geometry core gives. The rules of a kind are the
//! contract's (`kentos_contracts::infer_fields`); the web composes the same
//! (`inferFields`, apps/web/src/model/layerFields.ts).

use kentos_contracts::{LayerField, infer_fields};
use kentos_geometry_core::text::natural::natural_cmp;

use crate::document::Document;

/// The fields Verilerden al gives `rows` of attributes: one per key, the
/// first kind every value that is not blank takes, in the natural order.
pub fn fields_from_rows(rows: &[Vec<(String, String)>]) -> Vec<LayerField> {
    infer_fields(rows.iter().map(Vec::as_slice), natural_cmp)
}

impl Document {
    /// The fields Verilerden al gives a layer: its objects' attributes
    /// read ([`fields_from_rows`]); none for an unknown layer or one without
    /// objects.
    pub fn fields_from_data(&self, layer: &str) -> Vec<LayerField> {
        let rows: Vec<Vec<(String, String)>> = self
            .by_layer(layer)
            .map(|e| {
                e.base()
                    .attrs
                    .iter()
                    .map(|(k, v)| (k.clone(), v.clone()))
                    .collect()
            })
            .collect();
        fields_from_rows(&rows)
    }
}
