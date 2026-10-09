//! The tools and models that ship with KentOS (the web's `builtin/`).

pub mod calculate_field;
pub mod edge_lengths;
pub mod geometry;
pub mod info_from_enclosing;
pub mod info_from_inside;
pub mod interpolation;
pub mod join_by_field;
pub mod models;
pub mod numbering;
pub mod pointcloud;
pub mod queries;
pub mod select_by_expression;
pub mod select_by_location;
pub mod summary_statistics;
pub mod surface;
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
        // Yüzey analizi (docs/adr/0231): both platforms.
        surface::slope(),
        surface::aspect(),
        surface::hillshade(),
        surface::color_relief(),
        surface::curvature(),
        surface::ruggedness(),
        surface::insolation(),
        surface::contours(),
        // İnterpolasyon and Yoğunluk (docs/adr/0232): both platforms.
        interpolation::idw(),
        interpolation::natural_neighbor(),
        interpolation::spline(),
        interpolation::kriging(),
        interpolation::tin(),
        interpolation::kernel_density(),
        interpolation::line_density(),
    ]
}

/// The built-in models (`BUILTIN_MODELS`).
pub fn models() -> Vec<Model> {
    vec![models::parcel_sheet()]
}
