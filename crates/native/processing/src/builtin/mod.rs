//! The tools and models that ship with KentOS (the web's `builtin/`).

pub mod address;
pub mod calculate_field;
pub mod convert;
pub mod distance;
pub mod edge_lengths;
pub mod generalize;
pub mod generate;
pub mod geometry;
pub mod hydrology;
pub mod info_from_enclosing;
pub mod info_from_inside;
pub mod interpolation;
pub mod join_by_field;
pub mod join_source;
pub mod layers;
pub mod lrs;
pub mod models;
pub mod multidim;
pub mod network;
pub mod numbering;
pub mod pointcloud;
pub mod proximity;
pub mod queries;
pub mod raster_ops;
pub mod raster_vector;
pub mod remote;
pub mod sampling;
pub mod select_by_expression;
pub mod select_by_location;
pub mod stats;
pub mod suitability;
pub mod summary_statistics;
pub mod surface;
pub mod transfer;
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
        transfer::tool(),
        // Yakınlık (docs/adr/0215): both platforms.
        proximity::nearest::tool(),
        proximity::matrix::tool(),
        proximity::hub::tool(),
        proximity::neighbors::tool(),
        proximity::shortest_line::tool(),
        // Katman işlemleri (docs/adr/0217): both platforms.
        layers::split::tool(),
        layers::merge::tool(),
        layers::extract::tool(),
        layers::dedupe::tool(),
        layers::order_by::tool(),
        // Geometri dönüştürme (docs/adr/0218): both platforms.
        convert::points::to_points(),
        convert::points::vertices(),
        convert::points::points_to_path(),
        convert::tables::bearing_line(),
        convert::tables::xy_line(),
        convert::tables::table_ellipse(),
        convert::tables::table_points(),
        convert::paths::substring_tool(),
        convert::paths::polygons_to_lines(),
        convert::paths::lines_to_polygons(),
        convert::paths::densify_tool(),
        generate::triangulation::voronoi_tool(),
        generate::triangulation::delaunay_tool(),
        generate::bounding::bounding_tool(),
        generate::bounding::concave_tool(),
        generate::pole::tool(),
        sampling::grid::tool(),
        sampling::points::regular_tool(),
        sampling::points::random_tool(),
        sampling::along::tool(),
        sampling::subdivide::tool(),
        generalize::smooth::tool(),
        generalize::square::tool(),
        generalize::building::tool(),
        generalize::aggregate::tool(),
        generalize::parts::tool(),
        generalize::eliminate::tool(),
        generalize::coverage::tool(),
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
        // Adres (docs/adr/0229 §11, §12): the desktop's; the web's side waits (docs/adr/0228).
        address::geocode::tool(),
        address::reverse::tool(),
        // Ağ analizi (docs/adr/0209 §7, §10).
        network::closest_facility::tool(),
        network::od_matrix::tool(),
        network::service_areas::tool(),
        // Doğrusal referans (docs/adr/0224 §5): routes, their events and measures.
        lrs::routes::create_routes(),
        lrs::routes::calibrate_routes(),
        lrs::events::route_events(),
        lrs::locate::locate_along(),
        lrs::events::overlay_events(),
        lrs::routes::extract_measures(),
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
        raster_ops::save_cog(),
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
        // Uygunluk analizi (docs/adr/0237): both platforms.
        suitability::fuzzy_membership(),
        suitability::fuzzy_overlay(),
        suitability::weighted_sum(),
        suitability::weighted_overlay(),
        suitability::pairwise(),
        suitability::roc(),
        // Mekânsal istatistik (docs/adr/0238): both platforms.
        stats::tools::mean_center(),
        stats::tools::median_center(),
        stats::tools::standard_distance(),
        stats::tools::directional_distribution(),
        stats::tools::nearest_neighbor(),
        stats::tools::morans(),
        stats::tools::hot_spot(),
        stats::tools::local_moran(),
        stats::tools::dbscan_tool(),
        stats::tools::k_means_tool(),
        // Uzaktan algılama (docs/adr/0242): both platforms.
        remote::tools::composite(),
        remote::tools::split(),
        remote::tools::index(),
        remote::tools::supervised(),
        remote::tools::unsupervised(),
        remote::tools::accuracy(),
        remote::tools::change(),
        remote::tools::pansharpen(),
        // Çok boyutlu veri (docs/adr/0243): both platforms.
        multidim::tools::profile(),
        multidim::tools::series(),
        multidim::tools::calculator(),
    ]
}

/// The built-in models (`BUILTIN_MODELS`).
pub fn models() -> Vec<Model> {
    vec![models::parcel_sheet()]
}
