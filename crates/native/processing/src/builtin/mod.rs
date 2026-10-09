//! The tools and models that ship with KentOS (the web's `builtin/`).

pub mod calculate_field;
pub mod edge_lengths;
pub mod geometry;
pub mod info_from_enclosing;
pub mod info_from_inside;
pub mod join_by_field;
pub mod models;
pub mod network;
pub mod numbering;
pub mod pointcloud;
pub mod queries;
pub mod select_by_expression;
pub mod select_by_location;
pub mod summary_statistics;
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
        select_by_location::tool(),
        info_from_inside::tool(),
        info_from_enclosing::tool(),
        summary_statistics::tool(),
        join_by_field::tool(),
        geometry::buffer::tool(),
        geometry::clip::tool(),
        geometry::dissolve::tool(),
        geometry::overlay::intersection(),
        geometry::overlay::difference(),
        geometry::overlay::sym_difference(),
        geometry::overlay::union(),
        geometry::validity::validity(),
        geometry::validity::repair_tool(),
        geometry::simplify::tool(),
        geometry::reproject::tool(),
        // Ağ analizi (docs/adr/0209 §7, §10).
        network::closest_facility::tool(),
        network::od_matrix::tool(),
        network::service_areas::tool(),
        // Nokta bulutu (docs/adr/0207 §7): the desktop's; the web's side waits.
        pointcloud::area_stats::tool(),
        pointcloud::thin::tool(),
        pointcloud::ground::tool(),
        pointcloud::classify::tool(),
        pointcloud::clip::tool(),
        pointcloud::merge::tool(),
        pointcloud::tile::tool(),
        pointcloud::rasterize::tool(),
        pointcloud::boundary::tool(),
    ]
}

/// The built-in models (`BUILTIN_MODELS`).
pub fn models() -> Vec<Model> {
    vec![models::parcel_sheet()]
}
