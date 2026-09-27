//! The tools and models that ship with KentOS (the web's `builtin/`).

pub mod calculate_field;
pub mod edge_lengths;
pub mod models;
pub mod numbering;
pub mod select_by_expression;
pub mod vertex_numbering;

use crate::model::Model;
use crate::types::Tool;

/// The built-in tools, in the web's order (`BUILTIN_TOOLS`).
pub fn tools() -> Vec<Tool> {
    vec![
        vertex_numbering::tool(),
        edge_lengths::tool(),
        calculate_field::tool(),
        select_by_expression::tool(),
    ]
}

/// The built-in models (`BUILTIN_MODELS`).
pub fn models() -> Vec<Model> {
    vec![models::parcel_sheet()]
}
