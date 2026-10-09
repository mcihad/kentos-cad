//! The requests of each kind (docs/adr/0208 §5–§11): what a host sends to a
//! WMS, a WMTS, a WFS, an OGC API, an ArcGIS REST service; a box's
//! coordinates in the order the system and the version want them.

use crate::crs::{epsg, in_axis_order};
use crate::query::{join, with_params};

/// A request a host sends: GET unless it has a body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Request {
    pub url: String,
    pub headers: Vec<(String, String)>,
    /// A POST's body and its media type.
    pub body: Option<(String, String)>,
}

impl Request {
    pub fn get(url: impl Into<String>) -> Request {
        Request {
            url: url.into(),
            headers: Vec::new(),
            body: None,
        }
    }

    /// A POST of a form's fields (`application/x-www-form-urlencoded`).
    pub fn form(url: impl Into<String>, fields: &[(&str, &str)]) -> Request {
        let body = fields
            .iter()
            .map(|(k, v)| format!("{}={}", crate::query::encode(k), crate::query::encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        Request {
            url: url.into(),
            headers: Vec::new(),
            body: Some((body, "application/x-www-form-urlencoded".into())),
        }
    }

    /// A POST of JSON.
    pub fn json(url: impl Into<String>, body: String) -> Request {
        Request {
            url: url.into(),
            headers: Vec::new(),
            body: Some((body, "application/json".into())),
        }
    }
}

fn numbers(list: &[f64]) -> String {
    list.iter()
        .map(|v| crate::query::number(*v))
        .collect::<Vec<_>>()
        .join(",")
}

/// A box `[x₁, y₁, x₂, y₂]` (east and north) in the order the system's
/// axes go when `follows` (WMS 1.3.0, WFS 1.1 and 2.0), else east first.
fn bbox_in(srid: u32, b: [f64; 4], follows: bool) -> [f64; 4] {
    if !follows {
        return b;
    }
    let (a1, b1) = in_axis_order(srid, b[0], b[1]);
    let (a2, b2) = in_axis_order(srid, b[2], b[3]);
    [a1, b1, a2, b2]
}

/// The OGC name of a system as WFS 1.1 and 2.0 and OGC API want it (its axes
/// in EPSG's order): `urn:ogc:def:crs:EPSG::<code>`.
pub fn urn(srid: u32) -> String {
    format!("urn:ogc:def:crs:EPSG::{srid}")
}

/// OGC API's URI of a system: `http://www.opengis.net/def/crs/EPSG/0/<code>`;
/// CRS84 for longitude and latitude.
pub fn ogc_uri(srid: u32, crs84: bool) -> String {
    if crs84 {
        "http://www.opengis.net/def/crs/OGC/1.3/CRS84".to_owned()
    } else {
        format!("http://www.opengis.net/def/crs/EPSG/0/{srid}")
    }
}

/// WMS (docs/adr/0208 §5).
pub mod wms {
    use super::*;

    pub fn capabilities(base: &str, version: Option<&str>) -> String {
        let mut p = vec![("SERVICE", "WMS"), ("REQUEST", "GetCapabilities")];
        if let Some(v) = version {
            p.push(("VERSION", v));
        }
        with_params(base, &p)
    }

    /// What a GetMap and a GetFeatureInfo have in common.
    pub struct Map<'a> {
        pub base: &'a str,
        /// `1.3.0` or `1.1.1`.
        pub version: &'a str,
        pub layers: &'a [String],
        /// Comma-separated styles, one a layer (empty: each layer's default).
        pub styles: &'a str,
        pub srid: u32,
        /// East and north, `[x₁, y₁, x₂, y₂]`.
        pub bbox: [f64; 4],
        pub width: u32,
        pub height: u32,
        pub format: &'a str,
        pub transparent: bool,
        pub params: &'a [(String, String)],
    }

    fn common(m: &Map<'_>, request: &'static str) -> Vec<(String, String)> {
        let modern = m.version != "1.1.1";
        let styles = if m.styles.is_empty() {
            ",".repeat(m.layers.len().saturating_sub(1))
        } else {
            m.styles.to_owned()
        };
        let mut p: Vec<(String, String)> = vec![
            ("SERVICE".into(), "WMS".into()),
            ("VERSION".into(), m.version.into()),
            ("REQUEST".into(), request.into()),
            ("LAYERS".into(), m.layers.join(",")),
            ("STYLES".into(), styles),
            (if modern { "CRS" } else { "SRS" }.into(), epsg(m.srid)),
            ("BBOX".into(), numbers(&bbox_in(m.srid, m.bbox, modern))),
            ("WIDTH".into(), m.width.to_string()),
            ("HEIGHT".into(), m.height.to_string()),
            ("FORMAT".into(), m.format.into()),
        ];
        if m.transparent {
            p.push(("TRANSPARENT".into(), "TRUE".into()));
        }
        for (k, v) in m.params {
            p.push((k.clone(), v.clone()));
        }
        p
    }

    fn url(base: &str, p: &[(String, String)]) -> String {
        let pairs: Vec<(&str, &str)> = p.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        with_params(base, &pairs)
    }

    pub fn get_map(m: &Map<'_>) -> String {
        url(m.base, &common(m, "GetMap"))
    }

    /// GetFeatureInfo at pixel (i, j) of the map `m` describes.
    pub fn get_feature_info(m: &Map<'_>, i: u32, j: u32, info_format: &str, count: u32) -> String {
        let modern = m.version != "1.1.1";
        let mut p = common(m, "GetFeatureInfo");
        p.push(("QUERY_LAYERS".into(), m.layers.join(",")));
        p.push(("INFO_FORMAT".into(), info_format.into()));
        p.push(("FEATURE_COUNT".into(), count.to_string()));
        p.push((if modern { "I" } else { "X" }.into(), i.to_string()));
        p.push((if modern { "J" } else { "Y" }.into(), j.to_string()));
        url(m.base, &p)
    }
}

/// WMTS (docs/adr/0208 §6).
pub mod wmts {
    use super::*;

    pub fn capabilities(base: &str) -> String {
        with_params(
            base,
            &[
                ("SERVICE", "WMTS"),
                ("REQUEST", "GetCapabilities"),
                ("VERSION", "1.0.0"),
            ],
        )
    }

    /// A KVP GetTile.
    #[allow(clippy::too_many_arguments)]
    pub fn get_tile(
        base: &str,
        layer: &str,
        style: &str,
        format: &str,
        matrix_set: &str,
        matrix: &str,
        row: u64,
        col: u64,
        dimensions: &[(String, String)],
    ) -> String {
        let (r, c) = (row.to_string(), col.to_string());
        let mut p: Vec<(&str, &str)> = vec![
            ("SERVICE", "WMTS"),
            ("REQUEST", "GetTile"),
            ("VERSION", "1.0.0"),
            ("LAYER", layer),
            ("STYLE", style),
            ("FORMAT", format),
            ("TILEMATRIXSET", matrix_set),
            ("TILEMATRIX", matrix),
            ("TILEROW", &r),
            ("TILECOL", &c),
        ];
        for (k, v) in dimensions {
            p.push((k, v));
        }
        with_params(base, &p)
    }
}

/// WFS (docs/adr/0208 §10).
pub mod wfs {
    use super::*;

    pub fn capabilities(base: &str) -> String {
        with_params(base, &[("SERVICE", "WFS"), ("REQUEST", "GetCapabilities")])
    }

    /// A GetFeature page.
    pub struct GetFeature<'a> {
        pub base: &'a str,
        /// `2.0.0`, `1.1.0` or `1.0.0`.
        pub version: &'a str,
        pub type_name: &'a str,
        pub srid: u32,
        /// East and north in `srid`; none: everything.
        pub bbox: Option<[f64; 4]>,
        /// A CQL filter (GeoServer's `CQL_FILTER`): the box is then checked
        /// by the host, as a server takes one or the other.
        pub filter: Option<&'a str>,
        pub count: Option<u64>,
        pub start: Option<u64>,
        pub output_format: Option<&'a str>,
        /// The system named `EPSG:n` (east first, as servers write GeoJSON)
        /// rather than by its URN (its axes in EPSG's order, for GML).
        pub short_srs: bool,
    }

    pub fn get_feature(g: &GetFeature<'_>) -> String {
        let v2 = g.version.starts_with('2');
        let v10 = g.version == "1.0.0";
        let short = v10 || g.short_srs;
        let srs = if short { epsg(g.srid) } else { urn(g.srid) };
        let mut p: Vec<(String, String)> = vec![
            ("SERVICE".into(), "WFS".into()),
            ("VERSION".into(), g.version.into()),
            ("REQUEST".into(), "GetFeature".into()),
            (
                if v2 { "TYPENAMES" } else { "TYPENAME" }.into(),
                g.type_name.into(),
            ),
            ("SRSNAME".into(), srs.clone()),
        ];
        match (g.filter, g.bbox) {
            (Some(f), _) => p.push(("CQL_FILTER".into(), f.into())),
            (None, Some(b)) => {
                let b = bbox_in(g.srid, b, !short);
                let mut text = numbers(&b);
                text.push(',');
                text.push_str(&srs);
                p.push(("BBOX".into(), text));
            }
            (None, None) => {}
        }
        if let Some(n) = g.count {
            p.push((
                if v2 { "COUNT" } else { "MAXFEATURES" }.into(),
                n.to_string(),
            ));
        }
        if let (Some(s), true) = (g.start, v2) {
            p.push(("STARTINDEX".into(), s.to_string()));
        }
        if let Some(f) = g.output_format {
            p.push(("OUTPUTFORMAT".into(), f.into()));
        }
        let pairs: Vec<(&str, &str)> = p.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        with_params(g.base, &pairs)
    }
}

/// OGC API Features (docs/adr/0208 §10).
pub mod features {
    use super::*;

    /// A collection's `items` page: the box in CRS84 unless `bbox_srid` is
    /// given (Part 2); the answer in `crs` when given.
    pub fn items(
        items_url: &str,
        bbox: Option<[f64; 4]>,
        bbox_srid: Option<u32>,
        crs: Option<u32>,
        limit: u64,
        filter: Option<&str>,
    ) -> String {
        let mut p: Vec<(String, String)> = vec![("limit".into(), limit.to_string())];
        if let Some(b) = bbox {
            p.push(("bbox".into(), numbers(&b)));
            if let Some(s) = bbox_srid {
                p.push(("bbox-crs".into(), ogc_uri(s, false)));
            }
        }
        if let Some(s) = crs {
            p.push(("crs".into(), ogc_uri(s, false)));
        }
        if let Some(f) = filter {
            p.push(("filter".into(), f.into()));
            p.push(("filter-lang".into(), "cql2-text".into()));
        }
        let pairs: Vec<(&str, &str)> = p.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        with_params(items_url, &pairs)
    }
}

/// ArcGIS REST (docs/adr/0208 §7).
pub mod arcgis {
    use super::*;

    /// A service's or a layer's description.
    pub fn info(url: &str) -> String {
        with_params(url, &[("f", "json")])
    }

    /// A cached service's tile.
    pub fn tile(url: &str, level: u32, row: u64, col: u64) -> String {
        join(url, &format!("tile/{level}/{row}/{col}"))
    }

    /// A MapServer's `export` of the box `bbox` (east and north in `srid`).
    pub fn export(
        url: &str,
        bbox: [f64; 4],
        srid: u32,
        width: u32,
        height: u32,
        transparent: bool,
        layers: &[String],
    ) -> String {
        let (sr, size) = (srid.to_string(), format!("{width},{height}"));
        let shown = format!("show:{}", layers.join(","));
        let b = numbers(&bbox);
        let mut p: Vec<(&str, &str)> = vec![
            ("bbox", &b),
            ("bboxSR", &sr),
            ("imageSR", &sr),
            ("size", &size),
            ("format", "png32"),
            ("transparent", if transparent { "true" } else { "false" }),
            ("dpi", "96"),
            ("f", "image"),
        ];
        if !layers.is_empty() {
            p.push(("layers", &shown));
        }
        with_params(&join(url, "export"), &p)
    }

    /// A layer's `query` page in GeoJSON.
    pub fn query(
        layer_url: &str,
        filter: Option<&str>,
        bbox: Option<[f64; 4]>,
        srid: u32,
        offset: u64,
        count: u64,
    ) -> String {
        let (sr, off, n) = (srid.to_string(), offset.to_string(), count.to_string());
        let b = bbox.map(|b| numbers(&b));
        let mut p: Vec<(&str, &str)> = vec![
            ("where", filter.unwrap_or("1=1")),
            ("outFields", "*"),
            ("outSR", &sr),
            ("f", "geojson"),
            ("resultOffset", &off),
            ("resultRecordCount", &n),
        ];
        if let Some(b) = &b {
            p.extend([
                ("geometry", b.as_str()),
                ("geometryType", "esriGeometryEnvelope"),
                ("inSR", &sr),
                ("spatialRel", "esriSpatialRelIntersects"),
            ]);
        }
        with_params(&join(layer_url, "query"), &p)
    }

    /// `identify` at a point of a map of the box `bbox` drawn `width` ×
    /// `height` pixels (all east and north in `srid`).
    #[allow(clippy::too_many_arguments)]
    pub fn identify(
        url: &str,
        x: f64,
        y: f64,
        srid: u32,
        bbox: [f64; 4],
        width: u32,
        height: u32,
        layers: &[String],
    ) -> String {
        let (sr, at, extent) = (srid.to_string(), numbers(&[x, y]), numbers(&bbox));
        let display = format!("{width},{height},96");
        let shown = if layers.is_empty() {
            "visible".to_owned()
        } else {
            format!("visible:{}", layers.join(","))
        };
        with_params(
            &join(url, "identify"),
            &[
                ("geometry", &at),
                ("geometryType", "esriGeometryPoint"),
                ("sr", &sr),
                ("layers", &shown),
                ("tolerance", "5"),
                ("mapExtent", &extent),
                ("imageDisplay", &display),
                ("returnGeometry", "false"),
                ("f", "json"),
            ],
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_getmap_writes_the_box_in_the_systems_order() {
        let layers = vec!["imar:plan".to_owned(), "imar:yol".to_owned()];
        let m = wms::Map {
            base: "https://kbs.ornek.bel.tr/geoserver/wms",
            version: "1.3.0",
            layers: &layers,
            styles: "",
            srid: 5254,
            bbox: [500_000.0, 4_400_000.0, 500_512.0, 4_400_512.0],
            width: 512,
            height: 512,
            format: "image/png",
            transparent: true,
            params: &[],
        };
        assert_eq!(
            wms::get_map(&m),
            "https://kbs.ornek.bel.tr/geoserver/wms?SERVICE=WMS&VERSION=1.3.0&REQUEST=GetMap&LAYERS=imar:plan,imar:yol&STYLES=,&CRS=EPSG:5254&BBOX=4400000,500000,4400512,500512&WIDTH=512&HEIGHT=512&FORMAT=image/png&TRANSPARENT=TRUE"
        );
        // 1.1.1 writes east first and names SRS.
        let old = wms::Map {
            version: "1.1.1",
            ..m
        };
        assert!(wms::get_map(&old).contains("&SRS=EPSG:5254&BBOX=500000,4400000,500512,4400512&"));
        assert!(wms::get_feature_info(&old, 10, 20, "text/plain", 5).ends_with("&X=10&Y=20"));
    }

    #[test]
    fn a_getfeature_and_its_pages() {
        let g = wfs::GetFeature {
            base: "https://x/wfs",
            version: "2.0.0",
            type_name: "kadastro:parsel",
            srid: 5254,
            bbox: Some([1.0, 2.0, 3.0, 4.0]),
            filter: None,
            count: Some(1000),
            start: Some(2000),
            output_format: Some("application/json"),
            short_srs: false,
        };
        assert_eq!(
            wfs::get_feature(&g),
            "https://x/wfs?SERVICE=WFS&VERSION=2.0.0&REQUEST=GetFeature&TYPENAMES=kadastro:parsel&SRSNAME=urn:ogc:def:crs:EPSG::5254&BBOX=2,1,4,3,urn:ogc:def:crs:EPSG::5254&COUNT=1000&STARTINDEX=2000&OUTPUTFORMAT=application/json"
        );
        let old = wfs::GetFeature {
            version: "1.0.0",
            ..g
        };
        assert!(
            wfs::get_feature(&old)
                .contains("&SRSNAME=EPSG:5254&BBOX=1,2,3,4,EPSG:5254&MAXFEATURES=1000")
        );
        // GeoJSON asked for: the short name, east first.
        let short = wfs::GetFeature {
            short_srs: true,
            ..g
        };
        assert!(
            wfs::get_feature(&short)
                .contains("&SRSNAME=EPSG:5254&BBOX=1,2,3,4,EPSG:5254&COUNT=1000&STARTINDEX=2000")
        );
    }

    #[test]
    fn arcgis_requests() {
        let url = "https://x/arcgis/rest/services/Imar/MapServer";
        assert_eq!(
            arcgis::tile(url, 7, 40, 50),
            "https://x/arcgis/rest/services/Imar/MapServer/tile/7/40/50"
        );
        assert!(arcgis::export(url, [0.0, 1.0, 2.0, 3.0], 5254, 512, 256, true, &["0".into(), "2".into()])
            .starts_with("https://x/arcgis/rest/services/Imar/MapServer/export?bbox=0,1,2,3&bboxSR=5254&imageSR=5254&size=512,256&format=png32&transparent=true"));
        assert!(
            arcgis::query(&format!("{url}/0"), None, None, 5254, 0, 2000)
                .contains("where=1%3D1&outFields=*&outSR=5254&f=geojson")
        );
    }
}
