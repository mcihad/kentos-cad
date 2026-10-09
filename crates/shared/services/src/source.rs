//! A service layer's tiles (docs/adr/0208 §3–§9): its grid, the system the
//! grid is in, whether its tiles are pictures or vector tiles, its levels,
//! and the address of each tile. WMS and an ArcGIS `export` are asked on a
//! virtual grid of 512-pixel tiles; with `dynamic`, one picture of the view.

use kentos_contracts::{ServiceKind, ServiceLayer, TileGrid};
use kentos_geometry_core::geom::tiles::{Grid, Matrix, TileRef};

use crate::request::{arcgis, wms, wmts};
use crate::template;

/// The side of a WMS's and an `export`'s virtual tiles.
pub const VIRTUAL_TILE: u32 = 512;

/// What a service layer's tiles are.
#[derive(Clone, Debug, PartialEq)]
pub struct TileSource {
    pub grid: Grid,
    /// The grid's system.
    pub srid: u32,
    pub vector: bool,
    pub min_level: u32,
    pub max_level: u32,
    /// The template for vector tiles resolved from a style or a TileJSON
    /// (a vector layer's own address when it is a template).
    pub template: Option<String>,
}

/// A grid read from a service (WMTS, OGC, ArcGIS) as the tile math takes it.
pub fn grid_of(g: &TileGrid) -> Grid {
    Grid::from_matrices(
        g.matrices
            .iter()
            .map(|m| Matrix {
                resolution: m.resolution,
                x0: m.x0,
                y0: m.y0,
                tile_w: m.tile_width,
                tile_h: m.tile_height,
                cols: m.matrix_width,
                rows: m.matrix_height,
            })
            .collect(),
    )
}

/// Whether a format names vector tiles.
pub fn is_vector_format(format: &str) -> bool {
    let f = format.to_ascii_lowercase();
    f.contains("mapbox-vector-tile")
        || f.contains("mvt")
        || f.contains("pbf")
        || f == "application/x-protobuf"
}

/// A service layer's tiles; none for one asked view by view (`dynamic`), or
/// for a vector layer whose template is still to be read from its style or
/// TileJSON (`resolved` gives it).
pub fn tiles(s: &ServiceLayer, resolved: Option<&str>) -> Option<TileSource> {
    let side = s.tile_side();
    let zoom = |max: u32| (s.min_zoom.unwrap_or(0), s.max_zoom.unwrap_or(max).min(max));
    match s.kind {
        ServiceKind::Xyz | ServiceKind::Google => {
            let (lo, hi) = zoom(22);
            Some(TileSource {
                grid: Grid::web_mercator(side, hi),
                srid: 3857,
                vector: false,
                min_level: lo,
                max_level: hi,
                template: None,
            })
        }
        ServiceKind::Vector => {
            let template = match resolved {
                Some(t) => t.to_owned(),
                None if s.url.contains("{z}") => s.url.clone(),
                None => return None,
            };
            let (lo, hi) = zoom(14);
            // A vector tile covers 512 screen pixels at its zoom, as MapLibre draws it.
            Some(TileSource {
                grid: Grid::web_mercator(s.tile_size.unwrap_or(512), hi),
                srid: 3857,
                vector: true,
                min_level: lo,
                max_level: hi,
                template: Some(template),
            })
        }
        ServiceKind::Wmts | ServiceKind::OgcTiles => {
            let g = s.grid.as_ref()?;
            let grid = grid_of(g);
            let last = grid.matrices.len().saturating_sub(1) as u32;
            Some(TileSource {
                grid,
                srid: g.srid,
                vector: s.format.as_deref().is_some_and(is_vector_format),
                min_level: s.min_zoom.unwrap_or(0).min(last),
                max_level: s.max_zoom.unwrap_or(last).min(last),
                template: None,
            })
        }
        ServiceKind::Arcgis if s.grid.is_some() && !s.dynamic => {
            let g = s.grid.as_ref()?;
            let grid = grid_of(g);
            let last = grid.matrices.len().saturating_sub(1) as u32;
            Some(TileSource {
                grid,
                srid: g.srid,
                vector: false,
                min_level: s.min_zoom.unwrap_or(0).min(last),
                max_level: s.max_zoom.unwrap_or(last).min(last),
                template: None,
            })
        }
        ServiceKind::Wms | ServiceKind::Arcgis => {
            if s.dynamic {
                return None;
            }
            let srid = s.srid?;
            let grid = Grid::virtual_for(crate::crs::degrees(srid), VIRTUAL_TILE);
            let last = grid.matrices.len().saturating_sub(1) as u32;
            Some(TileSource {
                grid,
                srid,
                vector: false,
                min_level: s.min_zoom.unwrap_or(0),
                max_level: s.max_zoom.unwrap_or(last).min(last),
                template: None,
            })
        }
    }
}

/// The address of tile `t` of `src` (without a connection's proof, which
/// `auth::apply` adds): XYZ's and a vector layer's template, WMTS's REST
/// template or KVP, OGC's template, ArcGIS's tile or `export`, a WMS
/// GetMap; Google's needs its session (`google::tile_url`).
pub fn tile_url(s: &ServiceLayer, src: &TileSource, t: TileRef, hidpi: bool) -> Option<String> {
    let matrix = src.grid.matrices.get(t.level as usize)?;
    let id = || {
        s.grid
            .as_ref()
            .and_then(|g| g.matrices.get(t.level as usize))
            .map_or_else(|| t.level.to_string(), |m| m.id.clone())
    };
    let dims: Vec<(String, String)> = s
        .params
        .iter()
        .map(|p| (p.name.clone(), p.value.clone()))
        .collect();
    match s.kind {
        ServiceKind::Xyz => Some(template::xyz(&s.url, t, &s.subdomains, s.y_flip, hidpi)),
        ServiceKind::Vector => Some(template::xyz(
            src.template.as_deref()?,
            t,
            &s.subdomains,
            s.y_flip,
            false,
        )),
        ServiceKind::Google => None,
        ServiceKind::Wmts => {
            let layer = s.layers.first()?;
            let style = s.style.as_deref().unwrap_or("default");
            let matrix_set = s.matrix_set.clone().unwrap_or_default();
            match &s.template {
                Some(tpl) => Some(template::wmts(
                    tpl,
                    &matrix_set,
                    &id(),
                    t.row,
                    t.col,
                    style,
                    &dims,
                )),
                None => Some(wmts::get_tile(
                    &s.url,
                    layer,
                    style,
                    s.format.as_deref().unwrap_or("image/png"),
                    &matrix_set,
                    &id(),
                    t.row,
                    t.col,
                    &dims,
                )),
            }
        }
        ServiceKind::OgcTiles => Some(template::ogc(s.template.as_deref()?, &id(), t.row, t.col)),
        ServiceKind::Arcgis if s.grid.is_some() && !s.dynamic => Some(arcgis::tile(
            &s.url,
            id().parse().unwrap_or(t.level),
            t.row,
            t.col,
        )),
        ServiceKind::Arcgis => {
            let b = matrix.bounds(t.col, t.row);
            Some(arcgis::export(
                &s.url,
                b,
                src.srid,
                matrix.tile_w,
                matrix.tile_h,
                s.transparent,
                &s.layers,
            ))
        }
        ServiceKind::Wms => Some(wms::get_map(&wms::Map {
            base: &s.url,
            version: s.version.as_deref().unwrap_or("1.3.0"),
            layers: &s.layers,
            styles: s.style.as_deref().unwrap_or(""),
            srid: src.srid,
            bbox: matrix.bounds(t.col, t.row),
            width: matrix.tile_w,
            height: matrix.tile_h,
            format: s.format.as_deref().unwrap_or("image/png"),
            transparent: s.transparent,
            params: &dims,
        })),
    }
}

/// The address of one picture of the view `bbox` (east and north in the
/// layer's system) drawn `width` × `height` pixels: a `dynamic` WMS or
/// `export`.
pub fn view_url(s: &ServiceLayer, bbox: [f64; 4], width: u32, height: u32) -> Option<String> {
    let srid = s.srid?;
    let dims: Vec<(String, String)> = s
        .params
        .iter()
        .map(|p| (p.name.clone(), p.value.clone()))
        .collect();
    match s.kind {
        ServiceKind::Wms => Some(wms::get_map(&wms::Map {
            base: &s.url,
            version: s.version.as_deref().unwrap_or("1.3.0"),
            layers: &s.layers,
            styles: s.style.as_deref().unwrap_or(""),
            srid,
            bbox,
            width,
            height,
            format: s.format.as_deref().unwrap_or("image/png"),
            transparent: s.transparent,
            params: &dims,
        })),
        ServiceKind::Arcgis => Some(arcgis::export(
            &s.url,
            bbox,
            srid,
            width,
            height,
            s.transparent,
            &s.layers,
        )),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::ServiceParam;

    fn base(kind: ServiceKind, url: &str) -> ServiceLayer {
        ServiceLayer {
            kind,
            url: url.into(),
            layers: Vec::new(),
            style: None,
            format: None,
            srid: None,
            grid: None,
            matrix_set: None,
            template: None,
            tile_size: None,
            min_zoom: None,
            max_zoom: None,
            subdomains: Vec::new(),
            y_flip: false,
            transparent: false,
            version: None,
            params: Vec::new(),
            dynamic: false,
            attribution: None,
            opacity: None,
            connection: None,
            preset: None,
            bbox: None,
        }
    }

    #[test]
    fn xyz_and_wms_tiles() {
        let osm = ServiceLayer {
            max_zoom: Some(19),
            ..base(
                ServiceKind::Xyz,
                "https://tile.openstreetmap.org/{z}/{x}/{y}.png",
            )
        };
        let src = tiles(&osm, None).expect("tiles");
        assert_eq!(
            (src.srid, src.max_level, src.grid.matrices.len()),
            (3857, 19, 20)
        );
        let t = TileRef {
            level: 15,
            col: 19_458,
            row: 12_412,
        };
        assert_eq!(
            tile_url(&osm, &src, t, false).as_deref(),
            Some("https://tile.openstreetmap.org/15/19458/12412.png")
        );
        let w = ServiceLayer {
            layers: vec!["imar:plan".into()],
            srid: Some(5254),
            version: Some("1.3.0".into()),
            transparent: true,
            params: vec![ServiceParam {
                name: "TIME".into(),
                value: "2024".into(),
            }],
            ..base(ServiceKind::Wms, "https://x/wms")
        };
        let src = tiles(&w, None).expect("tiles");
        assert_eq!(src.grid.matrices[0].tile_w, 512);
        let url = tile_url(
            &w,
            &src,
            TileRef {
                level: 10,
                col: 511,
                row: 511,
            },
            false,
        )
        .unwrap();
        // Level 10's 512-pixel tile 511, 511 ends at the grid's centre: (0, 0) east and north,
        // written north first for EPSG:5254.
        assert!(
            url.contains(
                "&CRS=EPSG:5254&BBOX=0,-39135.75848201024,39135.75848201024,0&WIDTH=512&HEIGHT=512"
            ),
            "{url}"
        );
        assert!(url.ends_with("&TRANSPARENT=TRUE&TIME=2024"));
        assert!(
            tiles(
                &ServiceLayer {
                    dynamic: true,
                    ..w.clone()
                },
                None
            )
            .is_none()
        );
        assert!(
            view_url(&w, [0.0, 0.0, 1.0, 1.0], 100, 100)
                .unwrap()
                .contains("WIDTH=100")
        );
    }
}
