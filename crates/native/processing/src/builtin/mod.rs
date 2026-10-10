//! The tools and models that ship with KentOS (the web's `builtin/`).

pub mod calculate_field;
pub mod distance;
pub mod edge_lengths;
pub mod geometry;
pub mod hydrology;
pub mod info_from_enclosing;
pub mod info_from_inside;
pub mod interpolation;
pub mod join_by_field;
pub mod models;
pub mod network;
pub mod numbering;
pub mod pointcloud;
pub mod proximity;
pub mod queries;
pub mod raster_ops;
pub mod raster_vector;
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
        // Yakınlık (docs/adr/0215): both platforms.
        proximity::nearest::tool(),
        proximity::matrix::tool(),
        proximity::hub::tool(),
        proximity::neighbors::tool(),
        proximity::shortest_line::tool(),
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
        // Raster işlemleri and Raster istatistiği (docs/adr/0233): both platforms.
        raster_ops::calculator(),
        raster_ops::reclassify(),
        raster_ops::clip_by_mask(),
        raster_ops::mosaic(),
        raster_ops::resample(),
        raster_ops::zonal_statistics(),
        raster_ops::histogram(),
        raster_ops::focal_statistics(),
        raster_ops::cell_statistics(),
        // Raster ve vektör and Taranmış harita (docs/adr/0234): both platforms.
        raster_vector::rasterize(),
        raster_vector::to_polygons(),
        raster_vector::to_lines(),
        raster_vector::to_points(),
        raster_vector::capture_line(),
        raster_vector::close_area(),
        raster_vector::contour_elevations_tool(),
        // Hidroloji (docs/adr/0235): both platforms.
        hydrology::fill(),
        hydrology::flow_direction(),
        hydrology::accumulation(),
        hydrology::wetness(),
        hydrology::pour_point(),
        hydrology::watershed(),
        hydrology::basins(),
        hydrology::streams(),
        // Uzaklık ve maliyet (docs/adr/0236): both platforms.
        distance::euclidean(),
        distance::cost(),
        distance::path(),
        distance::corridor(),
    ]
}

/// The built-in models (`BUILTIN_MODELS`).
pub fn models() -> Vec<Model> {
    vec![models::parcel_sheet()]
}
