//! TileJSON 2 and 3 (docs/adr/0208 §9): a tile source's templates, its
//! levels, box, scheme (XYZ or TMS), credits and, for vector tiles, its
//! layers.

use serde::Serialize;

use super::json;
use crate::query::resolve;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TileJson {
    pub tiles: Vec<String>,
    pub min_zoom: u32,
    pub max_zoom: u32,
    /// `[west, south, east, north]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bounds: Option<[f64; 4]>,
    /// Rows from the bottom (`"scheme": "tms"`).
    pub tms: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attribution: Option<String>,
    pub name: String,
    /// Vector tiles' layers.
    pub layers: Vec<String>,
}

/// Reads a TileJSON document fetched from `base` (its templates resolved against it).
pub fn read(text: &str, base: &str) -> Result<TileJson, String> {
    let v = json(text)?;
    let tiles: Vec<String> = v["tiles"]
        .as_array()
        .map(|t| {
            t.iter()
                .filter_map(|x| x.as_str())
                .map(|x| resolve(base, x))
                .collect()
        })
        .unwrap_or_default();
    if tiles.is_empty() {
        return Err("Belge bir TileJSON değil (tiles yok).".to_owned());
    }
    let zoom = |k: &str, d: u32| {
        v[k].as_u64()
            .and_then(|z| u32::try_from(z).ok())
            .unwrap_or(d)
            .min(kentos_contracts::MAX_ZOOM)
    };
    let bounds = v["bounds"].as_array().and_then(|b| {
        let n: Vec<f64> = b.iter().filter_map(serde_json::Value::as_f64).collect();
        (n.len() == 4).then(|| [n[0], n[1], n[2], n[3]])
    });
    Ok(TileJson {
        tiles,
        min_zoom: zoom("minzoom", 0),
        max_zoom: zoom("maxzoom", 22),
        bounds,
        tms: v["scheme"].as_str() == Some("tms"),
        attribution: v["attribution"].as_str().map(str::to_owned),
        name: v["name"].as_str().unwrap_or("").to_owned(),
        layers: v["vector_layers"]
            .as_array()
            .map(|l| {
                l.iter()
                    .filter_map(|x| x["id"].as_str().map(str::to_owned))
                    .collect()
            })
            .unwrap_or_default(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn openfreemaps_planet() {
        let t = read(
            r#"{"tilejson":"3.0.0","tiles":["https://tiles.openfreemap.org/planet/20261004_113936_pt/{z}/{x}/{y}.pbf"],
            "attribution":"<a href=\"https://openfreemap.org\">OpenFreeMap</a>","bounds":[-180,-85.05113,180,85.05113],
            "maxzoom":14,"minzoom":0,"name":"OpenFreeMap","vector_layers":[{"id":"water"},{"id":"transportation"}]}"#,
            "https://tiles.openfreemap.org/planet",
        )
        .expect("reads");
        assert_eq!(t.max_zoom, 14);
        assert_eq!(t.layers, vec!["water", "transportation"]);
        assert!(!t.tms);
        assert!(read("{}", "https://x").is_err());
        let rel = read(
            r#"{"tiles":["t/{z}/{x}/{y}.png"],"scheme":"tms"}"#,
            "https://x/a/tile.json",
        )
        .unwrap();
        assert_eq!(rel.tiles[0], "https://x/a/t/{z}/{x}/{y}.png");
        assert!(rel.tms);
    }
}
