//! WMTS 1.0 capabilities (docs/adr/0208 §6): layers (formats, styles, the
//! matrix sets they are tiled on with their limits, REST templates,
//! dimensions' defaults), the matrix sets (each matrix's pixel from its scale
//! denominator, its top left corner turned to east and north by its
//! system's axis order) and the KVP GetTile address.

use kentos_contracts::TileMatrix;
use serde::Serialize;

use super::{attr, child, child_text, children, metres_per_unit, pair, text_of, wgs84_box, xml};
use crate::crs::{crs_name, to_east_north};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WmtsCapabilities {
    pub title: String,
    pub layers: Vec<WmtsLayer>,
    pub matrix_sets: Vec<WmtsMatrixSet>,
    /// GetTile's KVP address, when the service has one.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get_tile: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WmtsStyle {
    pub id: String,
    pub title: String,
    pub default: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WmtsLimit {
    pub matrix: String,
    pub min_row: u64,
    pub max_row: u64,
    pub min_col: u64,
    pub max_col: u64,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WmtsLink {
    pub set: String,
    pub limits: Vec<WmtsLimit>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WmtsLayer {
    pub id: String,
    pub title: String,
    #[serde(rename = "abstract", skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wgs84: Option<[f64; 4]>,
    pub formats: Vec<String>,
    pub styles: Vec<WmtsStyle>,
    pub links: Vec<WmtsLink>,
    /// REST templates by format.
    pub templates: Vec<(String, String)>,
    /// Dimensions and their defaults (`Time`, `2024`).
    pub dimensions: Vec<(String, String)>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WmtsMatrixSet {
    pub id: String,
    /// The system's EPSG code; none for one KentOS does not read.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srid: Option<u32>,
    pub crs: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub well_known: Option<String>,
    pub matrices: Vec<TileMatrix>,
}

/// Reads a WMTS capabilities document.
pub fn read(text: &str) -> Result<WmtsCapabilities, String> {
    let doc = xml(text)?;
    let root = doc.root_element();
    if root.tag_name().name() != "Capabilities" {
        return Err(format!(
            "Belge bir WMTS yetenek belgesi değil (kökü {}).",
            root.tag_name().name()
        ));
    }
    let title = child(root, "ServiceIdentification")
        .and_then(|s| child_text(s, "Title"))
        .unwrap_or_default();
    // The KVP GetTile address: an Operation named GetTile whose Get is KVP.
    let get_tile = child(root, "OperationsMetadata").and_then(|m| {
        children(m, "Operation")
            .find(|o| attr(*o, "name") == Some("GetTile"))
            .and_then(|o| {
                o.descendants()
                    .filter(|d| d.is_element() && d.tag_name().name() == "Get")
                    .find(|g| {
                        let kvp = g.descendants().any(|v| {
                            v.is_element()
                                && v.tag_name().name() == "Value"
                                && text_of(v).as_deref() == Some("KVP")
                        });
                        let constrained = g
                            .descendants()
                            .any(|v| v.is_element() && v.tag_name().name() == "Constraint");
                        kvp || !constrained
                    })
                    .and_then(|g| attr(g, "href").map(|h| h.trim().to_owned()))
            })
    });
    let contents =
        child(root, "Contents").ok_or_else(|| "WMTS belgesinde Contents yok.".to_owned())?;
    let layers = children(contents, "Layer")
        .map(|l| WmtsLayer {
            id: child_text(l, "Identifier").unwrap_or_default(),
            title: child_text(l, "Title").unwrap_or_default(),
            summary: child_text(l, "Abstract"),
            wgs84: wgs84_box(l),
            formats: children(l, "Format").filter_map(text_of).collect(),
            styles: children(l, "Style")
                .map(|s| WmtsStyle {
                    id: child_text(s, "Identifier").unwrap_or_default(),
                    title: child_text(s, "Title")
                        .or_else(|| child_text(s, "Identifier"))
                        .unwrap_or_default(),
                    default: attr(s, "isDefault").is_some_and(|v| v == "true" || v == "1"),
                })
                .collect(),
            links: children(l, "TileMatrixSetLink")
                .map(|k| WmtsLink {
                    set: child_text(k, "TileMatrixSet").unwrap_or_default(),
                    limits: child(k, "TileMatrixSetLimits")
                        .map(|lim| {
                            children(lim, "TileMatrixLimits")
                                .filter_map(|m| {
                                    let n = |key: &str| child_text(m, key)?.parse::<u64>().ok();
                                    Some(WmtsLimit {
                                        matrix: child_text(m, "TileMatrix")?,
                                        min_row: n("MinTileRow")?,
                                        max_row: n("MaxTileRow")?,
                                        min_col: n("MinTileCol")?,
                                        max_col: n("MaxTileCol")?,
                                    })
                                })
                                .collect()
                        })
                        .unwrap_or_default(),
                })
                .collect(),
            templates: children(l, "ResourceURL")
                .filter(|r| attr(*r, "resourceType") == Some("tile"))
                .filter_map(|r| {
                    Some((
                        attr(r, "format").unwrap_or("").to_owned(),
                        attr(r, "template")?.trim().to_owned(),
                    ))
                })
                .collect(),
            dimensions: children(l, "Dimension")
                .filter_map(|d| {
                    let id = child_text(d, "Identifier")?;
                    let value = child_text(d, "Default").or_else(|| child_text(d, "Value"))?;
                    Some((id, value))
                })
                .collect(),
        })
        .collect();
    let matrix_sets = children(contents, "TileMatrixSet")
        .map(|s| {
            let crs = child_text(s, "SupportedCRS").unwrap_or_default();
            let name = crs_name(&crs);
            let srid = name.map(|n| n.srid);
            let per = metres_per_unit(srid.unwrap_or(0));
            let matrices = children(s, "TileMatrix")
                .filter_map(|m| {
                    let scale: f64 = child_text(m, "ScaleDenominator")?.parse().ok()?;
                    let (a, b) = pair(&child_text(m, "TopLeftCorner")?)?;
                    let (x0, y0) = match name {
                        Some(n) => to_east_north(n, a, b),
                        None => (a, b),
                    };
                    let n = |key: &str| child_text(m, key)?.parse::<u64>().ok();
                    Some(TileMatrix {
                        id: child_text(m, "Identifier")?,
                        resolution: scale * 0.000_28 / per,
                        x0,
                        y0,
                        tile_width: u32::try_from(n("TileWidth")?).ok()?,
                        tile_height: u32::try_from(n("TileHeight")?).ok()?,
                        matrix_width: n("MatrixWidth")?,
                        matrix_height: n("MatrixHeight")?,
                    })
                })
                .collect();
            WmtsMatrixSet {
                id: child_text(s, "Identifier").unwrap_or_default(),
                srid,
                crs,
                well_known: child_text(s, "WellKnownScaleSet"),
                matrices,
            }
        })
        .collect();
    Ok(WmtsCapabilities {
        title,
        layers,
        matrix_sets,
        get_tile,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<Capabilities xmlns="http://www.opengis.net/wmts/1.0" xmlns:ows="http://www.opengis.net/ows/1.1" xmlns:xlink="http://www.w3.org/1999/xlink" version="1.0.0">
  <ows:ServiceIdentification><ows:Title>Ortofoto</ows:Title></ows:ServiceIdentification>
  <ows:OperationsMetadata>
    <ows:Operation name="GetTile"><ows:DCP><ows:HTTP>
      <ows:Get xlink:href="https://x/wmts?"><ows:Constraint name="GetEncoding"><ows:AllowedValues><ows:Value>KVP</ows:Value></ows:AllowedValues></ows:Constraint></ows:Get>
    </ows:HTTP></ows:DCP></ows:Operation>
  </ows:OperationsMetadata>
  <Contents>
    <Layer>
      <ows:Title>Ortofoto 2024</ows:Title><ows:Identifier>orto</ows:Identifier>
      <ows:WGS84BoundingBox><ows:LowerCorner>26 36</ows:LowerCorner><ows:UpperCorner>45 42</ows:UpperCorner></ows:WGS84BoundingBox>
      <Style isDefault="true"><ows:Identifier>default</ows:Identifier></Style>
      <Format>image/jpeg</Format>
      <Dimension><ows:Identifier>Time</ows:Identifier><Default>2024</Default><Value>2024</Value></Dimension>
      <TileMatrixSetLink><TileMatrixSet>TM30</TileMatrixSet>
        <TileMatrixSetLimits><TileMatrixLimits><TileMatrix>0</TileMatrix><MinTileRow>0</MinTileRow><MaxTileRow>1</MaxTileRow><MinTileCol>0</MinTileCol><MaxTileCol>2</MaxTileCol></TileMatrixLimits></TileMatrixSetLimits>
      </TileMatrixSetLink>
      <TileMatrixSetLink><TileMatrixSet>Geo</TileMatrixSet></TileMatrixSetLink>
      <ResourceURL format="image/jpeg" resourceType="tile" template="https://x/wmts/orto/{Style}/{TileMatrixSet}/{TileMatrix}/{TileRow}/{TileCol}.jpg"/>
    </Layer>
    <TileMatrixSet><ows:Identifier>TM30</ows:Identifier><ows:SupportedCRS>urn:ogc:def:crs:EPSG::5254</ows:SupportedCRS>
      <TileMatrix><ows:Identifier>0</ows:Identifier><ScaleDenominator>1000000</ScaleDenominator><TopLeftCorner>5000000 300000</TopLeftCorner>
        <TileWidth>256</TileWidth><TileHeight>256</TileHeight><MatrixWidth>4</MatrixWidth><MatrixHeight>3</MatrixHeight></TileMatrix>
    </TileMatrixSet>
    <TileMatrixSet><ows:Identifier>Geo</ows:Identifier><ows:SupportedCRS>urn:ogc:def:crs:EPSG::4326</ows:SupportedCRS>
      <TileMatrix><ows:Identifier>0</ows:Identifier><ScaleDenominator>279541132.0143589</ScaleDenominator><TopLeftCorner>90 -180</TopLeftCorner>
        <TileWidth>256</TileWidth><TileHeight>256</TileHeight><MatrixWidth>2</MatrixWidth><MatrixHeight>1</MatrixHeight></TileMatrix>
    </TileMatrixSet>
  </Contents>
</Capabilities>"#;

    #[test]
    fn layers_sets_and_corners_east_first() {
        let c = read(DOC).expect("reads");
        assert_eq!(c.title, "Ortofoto");
        assert_eq!(c.get_tile.as_deref(), Some("https://x/wmts?"));
        let l = &c.layers[0];
        assert_eq!(
            (l.id.as_str(), l.wgs84),
            ("orto", Some([26.0, 36.0, 45.0, 42.0]))
        );
        assert!(l.styles[0].default);
        assert_eq!(l.dimensions, vec![("Time".to_owned(), "2024".to_owned())]);
        assert_eq!(l.links[0].limits[0].max_col, 2);
        assert_eq!(l.templates[0].0, "image/jpeg");
        let tm30 = &c.matrix_sets[0];
        assert_eq!(tm30.srid, Some(5254));
        // North first in the document: east first here; 1:1 000 000 is 280 m a pixel.
        let m = &tm30.matrices[0];
        assert_eq!((m.x0, m.y0), (300_000.0, 5_000_000.0));
        assert!((m.resolution - 280.0).abs() < 1e-9);
        let geo = &c.matrix_sets[1].matrices[0];
        assert_eq!((geo.x0, geo.y0), (-180.0, 90.0));
        // Level 0 of WorldCRS84Quad: 180° over 256 pixels.
        assert!((geo.resolution - 180.0 / 256.0).abs() < 1e-9);
    }
}
