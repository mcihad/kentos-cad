//! ArcGIS REST documents (docs/adr/0208 §7): a MapServer's, an
//! ImageServer's or a FeatureServer's description (its system, its tile
//! cache as a tile grid, its layers, its extent, its credits) and a layer's
//! (its geometry, fields and the most records a query gives).

use kentos_contracts::TileMatrix;
use serde::Serialize;
use serde_json::Value;

use super::json;
use crate::crs::crs_name;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcgisLayer {
    pub id: u32,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent: Option<u32>,
    pub visible: bool,
    /// None for a group layer.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcgisService {
    pub title: String,
    /// The service's system (102100 read as 3857); none when unknown.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srid: Option<u32>,
    /// Its tile cache's levels, coarsest first, when it has one.
    pub tiles: Vec<TileMatrix>,
    /// The tiles' image format (`PNG32`, `JPEG`, `MIXED`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tile_format: Option<String>,
    /// Whether `export` draws any view (a MapServer that is not only cached).
    pub exports: bool,
    pub layers: Vec<ArcgisLayer>,
    /// The full extent, east and north in `srid`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extent: Option<[f64; 4]>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub copyright: Option<String>,
}

fn wkid(sr: &Value) -> Option<u32> {
    let code = sr["latestWkid"].as_u64().or_else(|| sr["wkid"].as_u64())?;
    crs_name(&code.to_string()).map(|n| n.srid)
}

/// A service's description.
pub fn service(text: &str) -> Result<ArcgisService, String> {
    let v = json(text)?;
    if let Some(m) = v["error"]["message"].as_str() {
        return Err(format!("ArcGIS bir hata bildirdi: {m}"));
    }
    if !(v["layers"].is_array() || v["tileInfo"].is_object() || v["spatialReference"].is_object()) {
        return Err("Belge bir ArcGIS REST servis tanımı değil.".to_owned());
    }
    let srid = wkid(&v["spatialReference"]).or_else(|| wkid(&v["tileInfo"]["spatialReference"]));
    let mut tiles = Vec::new();
    let info = &v["tileInfo"];
    let cached = v["singleFusedMapCache"]
        .as_bool()
        .unwrap_or(info.is_object());
    if cached && info.is_object() {
        let (x0, y0) = (info["origin"]["x"].as_f64(), info["origin"]["y"].as_f64());
        let (cols, rows) = (info["cols"].as_u64(), info["rows"].as_u64());
        // The extent bounds the matrices (ArcGIS has no matrix size of its own).
        let full = &v["fullExtent"];
        let xmax = full["xmax"].as_f64();
        let ymin = full["ymin"].as_f64();
        if let (Some(x0), Some(y0), Some(cols), Some(rows), Some(lods)) =
            (x0, y0, cols, rows, info["lods"].as_array())
        {
            for lod in lods {
                let (Some(level), Some(res)) = (lod["level"].as_u64(), lod["resolution"].as_f64())
                else {
                    continue;
                };
                let span_x = res * cols as f64;
                let span_y = res * rows as f64;
                let across =
                    xmax.map_or(1u64 << 20, |e| ((e - x0) / span_x).ceil().max(1.0) as u64);
                let down = ymin.map_or(1u64 << 20, |s| ((y0 - s) / span_y).ceil().max(1.0) as u64);
                tiles.push(TileMatrix {
                    id: level.to_string(),
                    resolution: res,
                    x0,
                    y0,
                    tile_width: u32::try_from(cols).unwrap_or(256),
                    tile_height: u32::try_from(rows).unwrap_or(256),
                    matrix_width: across,
                    matrix_height: down,
                });
            }
        }
    }
    let layers = v["layers"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|l| {
                    Some(ArcgisLayer {
                        id: u32::try_from(l["id"].as_u64()?).ok()?,
                        name: l["name"].as_str().unwrap_or("").to_owned(),
                        parent: l["parentLayerId"]
                            .as_i64()
                            .and_then(|p| u32::try_from(p).ok()),
                        visible: l["defaultVisibility"].as_bool().unwrap_or(true),
                        geometry: l["geometryType"].as_str().map(str::to_owned),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    let full = &v["fullExtent"];
    let extent = match (
        full["xmin"].as_f64(),
        full["ymin"].as_f64(),
        full["xmax"].as_f64(),
        full["ymax"].as_f64(),
    ) {
        (Some(a), Some(b), Some(c), Some(d)) => Some([a, b, c, d]),
        _ => None,
    };
    let capabilities = v["capabilities"].as_str().unwrap_or("");
    Ok(ArcgisService {
        title: v["documentInfo"]["Title"]
            .as_str()
            .filter(|t| !t.is_empty())
            .or_else(|| v["mapName"].as_str())
            .or_else(|| v["name"].as_str())
            .unwrap_or("")
            .to_owned(),
        srid,
        tile_format: info["format"].as_str().map(str::to_owned),
        exports: !cached || capabilities.contains("Map"),
        tiles,
        layers,
        extent,
        copyright: v["copyrightText"]
            .as_str()
            .filter(|t| !t.trim().is_empty())
            .map(str::to_owned),
    })
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArcgisLayerInfo {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub geometry: Option<String>,
    pub fields: Vec<(String, String)>,
    /// The most records one query gives.
    pub max_records: u64,
    pub geojson: bool,
}

/// A layer's description (`…/MapServer/3?f=json`).
pub fn layer(text: &str) -> Result<ArcgisLayerInfo, String> {
    let v = json(text)?;
    if let Some(m) = v["error"]["message"].as_str() {
        return Err(format!("ArcGIS bir hata bildirdi: {m}"));
    }
    Ok(ArcgisLayerInfo {
        name: v["name"].as_str().unwrap_or("").to_owned(),
        geometry: v["geometryType"].as_str().map(str::to_owned),
        fields: v["fields"]
            .as_array()
            .map(|f| {
                f.iter()
                    .filter_map(|x| {
                        Some((
                            x["name"].as_str()?.to_owned(),
                            x["type"].as_str().unwrap_or("").to_owned(),
                        ))
                    })
                    .collect()
            })
            .unwrap_or_default(),
        max_records: v["maxRecordCount"]
            .as_u64()
            .unwrap_or(1000)
            .clamp(1, 100_000),
        geojson: v["supportedQueryFormats"]
            .as_str()
            .is_some_and(|f| f.to_ascii_lowercase().contains("geojson")),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_cached_mapserver() {
        let s = service(
            r#"{"mapName":"Layers","documentInfo":{"Title":"World Imagery"},"spatialReference":{"wkid":102100,"latestWkid":3857},
            "singleFusedMapCache":true,"capabilities":"Map,Tilemap,Query,Data",
            "tileInfo":{"rows":256,"cols":256,"format":"JPEG","origin":{"x":-20037508.342787,"y":20037508.342787},
              "lods":[{"level":0,"resolution":156543.03392800014,"scale":5.91657527591555E8},{"level":1,"resolution":78271.51696399994,"scale":2.95828763795777E8}]},
            "fullExtent":{"xmin":-20037507.067161843,"ymin":-19971868.880408604,"xmax":20037507.067161843,"ymax":19971868.88040863},
            "copyrightText":"Esri, Maxar","layers":[{"id":0,"name":"World Imagery","defaultVisibility":true}]}"#,
        )
        .expect("reads");
        assert_eq!(s.title, "World Imagery");
        assert_eq!(s.srid, Some(3857));
        assert_eq!(s.tiles.len(), 2);
        assert_eq!((s.tiles[0].matrix_width, s.tiles[1].matrix_width), (1, 2));
        assert_eq!(s.copyright.as_deref(), Some("Esri, Maxar"));
        assert!(s.exports);
        assert!(
            service(r#"{"error":{"code":499,"message":"Token Required"}}"#)
                .unwrap_err()
                .contains("Token Required")
        );
        let l = layer(r#"{"name":"Parsel","geometryType":"esriGeometryPolygon","maxRecordCount":2000,"supportedQueryFormats":"JSON, geoJSON","fields":[{"name":"ADA","type":"esriFieldTypeString"}]}"#).unwrap();
        assert!(l.geojson);
        assert_eq!(l.max_records, 2000);
        assert_eq!(l.fields[0].0, "ADA");
    }
}
