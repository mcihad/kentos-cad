//! OGC API documents (docs/adr/0208 §6, §10): a landing page's links, the
//! collections (id, title, box, systems, their items and tiles), a tile set
//! (its matrix set, its tile template, map or vector), a tile matrix set
//! (OGC 2D Tile Matrix Set 2.0, and 1.0's JSON) with each matrix's corner
//! turned to east and north.

use kentos_contracts::TileMatrix;
use serde::Serialize;
use serde_json::Value;

use super::{json, metres_per_unit};
use crate::crs::{crs_name, to_east_north};
use crate::query::resolve;

/// A link of a document.
#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Link {
    pub rel: String,
    pub href: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub media: Option<String>,
}

fn links(v: &Value, base: &str) -> Vec<Link> {
    v["links"]
        .as_array()
        .map(|list| {
            list.iter()
                .filter_map(|l| {
                    Some(Link {
                        rel: l["rel"].as_str()?.to_owned(),
                        href: resolve(base, l["href"].as_str()?),
                        media: l["type"].as_str().map(str::to_owned),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

/// The first link of `rel` (its IANA or OGC URI form), preferring JSON.
pub fn link<'a>(list: &'a [Link], rels: &[&str]) -> Option<&'a Link> {
    let wanted = |l: &&Link| {
        rels.iter().any(|r| {
            l.rel == *r
                || l.rel.ends_with(&format!("/rel/{r}"))
                || l.rel.ends_with(&format!("/{r}"))
        })
    };
    list.iter()
        .filter(wanted)
        .find(|l| l.media.as_deref().is_some_and(|m| m.contains("json")))
        .or_else(|| list.iter().find(wanted))
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Landing {
    pub title: String,
    pub links: Vec<Link>,
}

/// A landing page.
pub fn landing(text: &str, base: &str) -> Result<Landing, String> {
    let v = json(text)?;
    if !v["links"].is_array() {
        return Err("Belge bir OGC API açılış sayfası değil (links yok).".to_owned());
    }
    Ok(Landing {
        title: v["title"].as_str().unwrap_or("").to_owned(),
        links: links(&v, base),
    })
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Collection {
    pub id: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// `[west, south, east, north]`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wgs84: Option<[f64; 4]>,
    /// The systems its items may be asked in (Part 2), the storage's first.
    pub srids: Vec<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<String>,
    /// Its tile sets' list (map or vector).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tiles: Option<String>,
    pub links: Vec<Link>,
}

/// A `/collections` document.
pub fn collections(text: &str, base: &str) -> Result<Vec<Collection>, String> {
    let v = json(text)?;
    let list = v["collections"]
        .as_array()
        .ok_or_else(|| "Belge bir OGC API koleksiyon listesi değil.".to_owned())?;
    Ok(list.iter().filter_map(|c| collection(c, base)).collect())
}

fn collection(c: &Value, base: &str) -> Option<Collection> {
    let id = c["id"].as_str()?.to_owned();
    let all = links(c, base);
    let mut srids = Vec::new();
    for name in std::iter::once(&c["storageCrs"]).chain(c["crs"].as_array().into_iter().flatten()) {
        if let Some(n) = name.as_str().and_then(crs_name)
            && !srids.contains(&n.srid)
        {
            srids.push(n.srid);
        }
    }
    let bbox = &c["extent"]["spatial"]["bbox"][0];
    let wgs84 = bbox.as_array().and_then(|b| {
        let n: Vec<f64> = b.iter().filter_map(Value::as_f64).collect();
        match n.len() {
            4 => Some(super::ordered([n[0], n[1], n[2], n[3]])),
            6 => Some(super::ordered([n[0], n[1], n[3], n[4]])),
            _ => None,
        }
    });
    Some(Collection {
        title: c["title"].as_str().unwrap_or(&id).to_owned(),
        description: c["description"].as_str().map(str::to_owned),
        wgs84,
        srids,
        items: link(&all, &["items"]).map(|l| l.href.clone()),
        tiles: link(&all, &["tilesets-map", "tilesets-vector", "tiles"]).map(|l| l.href.clone()),
        links: all,
        id,
    })
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TileSet {
    pub title: String,
    /// `map` or `vector`.
    pub data_type: String,
    /// The tile template (`{tileMatrix}/{tileRow}/{tileCol}`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template: Option<String>,
    /// The tile set's own document.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub href: Option<String>,
    /// Its matrix set's URI or id (`WebMercatorQuad`).
    pub matrix_set: String,
    /// Its matrix set's document, when linked.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matrix_set_href: Option<String>,
}

fn tile_set(t: &Value, base: &str) -> TileSet {
    let all = links(t, base);
    let uri = t["tileMatrixSetURI"]
        .as_str()
        .or_else(|| t["tileMatrixSetId"].as_str())
        .or_else(|| t["tileMatrixSet"].as_str())
        .unwrap_or("")
        .to_owned();
    TileSet {
        title: t["title"].as_str().unwrap_or("").to_owned(),
        data_type: t["dataType"].as_str().unwrap_or("map").to_owned(),
        template: link(&all, &["item"]).map(|l| l.href.clone()),
        href: link(&all, &["self"]).map(|l| l.href.clone()),
        matrix_set_href: link(&all, &["tiling-scheme", "tilingScheme"]).map(|l| l.href.clone()),
        matrix_set: uri,
    }
}

/// A tile sets' list (`/tiles`, `/collections/{id}/map/tiles`).
pub fn tile_sets(text: &str, base: &str) -> Result<Vec<TileSet>, String> {
    let v = json(text)?;
    let list = v["tilesets"]
        .as_array()
        .ok_or_else(|| "Belge bir OGC API karo kümesi listesi değil.".to_owned())?;
    Ok(list.iter().map(|t| tile_set(t, base)).collect())
}

/// One tile set's document.
pub fn tile_set_doc(text: &str, base: &str) -> Result<TileSet, String> {
    json(text).map(|v| tile_set(&v, base))
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MatrixSet {
    pub id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub srid: Option<u32>,
    pub matrices: Vec<TileMatrix>,
}

/// A tile matrix set document: 2.0's `tileMatrices` (`cellSize`,
/// `pointOfOrigin`, `cornerOfOrigin`) or 1.0's `tileMatrix`
/// (`scaleDenominator`, `topLeftCorner`).
pub fn matrix_set(text: &str) -> Result<MatrixSet, String> {
    let v = json(text)?;
    let crs_text = v["crs"]
        .as_str()
        .or_else(|| v["crs"]["uri"].as_str())
        .or_else(|| v["supportedCRS"].as_str())
        .unwrap_or("");
    let name = crs_name(crs_text);
    let srid = name.map(|n| n.srid);
    let per = metres_per_unit(srid.unwrap_or(0));
    let list = v["tileMatrices"]
        .as_array()
        .or_else(|| v["tileMatrix"].as_array())
        .ok_or_else(|| "Belge bir karo matris kümesi değil.".to_owned())?;
    let matrices = list
        .iter()
        .filter_map(|m| {
            let id = m["id"]
                .as_str()
                .or_else(|| m["identifier"].as_str())?
                .to_owned();
            let resolution = m["cellSize"]
                .as_f64()
                .or_else(|| m["scaleDenominator"].as_f64().map(|s| s * 0.000_28 / per))?;
            let corner = m["pointOfOrigin"]
                .as_array()
                .or_else(|| m["topLeftCorner"].as_array())?;
            let (a, b) = (corner.first()?.as_f64()?, corner.get(1)?.as_f64()?);
            let (x0, y0) = match name {
                Some(n) => to_east_north(n, a, b),
                None => (a, b),
            };
            if m["cornerOfOrigin"].as_str().is_some_and(|c| c != "topLeft") {
                return None;
            }
            Some(TileMatrix {
                id,
                resolution,
                x0,
                y0,
                tile_width: u32::try_from(m["tileWidth"].as_u64()?).ok()?,
                tile_height: u32::try_from(m["tileHeight"].as_u64()?).ok()?,
                matrix_width: m["matrixWidth"].as_u64()?,
                matrix_height: m["matrixHeight"].as_u64()?,
            })
        })
        .collect();
    Ok(MatrixSet {
        id: v["id"]
            .as_str()
            .or_else(|| v["identifier"].as_str())
            .unwrap_or("")
            .to_owned(),
        srid,
        matrices,
    })
}

/// Whether a matrix set's URI or id is Web Mercator's square, which every
/// OGC API server uses and KentOS knows without asking.
pub fn is_web_mercator_quad(uri: &str) -> bool {
    uri.ends_with("WebMercatorQuad") || uri.ends_with("GoogleMapsCompatible")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn collections_tile_sets_and_a_matrix_set() {
        let base = "https://x/ogc/collections";
        let c = collections(
            r#"{"collections":[{"id":"parsel","title":"Parseller","storageCrs":"http://www.opengis.net/def/crs/EPSG/0/5254",
              "crs":["http://www.opengis.net/def/crs/OGC/1.3/CRS84","http://www.opengis.net/def/crs/EPSG/0/5254"],
              "extent":{"spatial":{"bbox":[[29,40,31,41]]}},
              "links":[{"rel":"items","type":"application/geo+json","href":"parsel/items"},
                       {"rel":"http://www.opengis.net/def/rel/ogc/1.0/tilesets-vector","type":"application/json","href":"/ogc/collections/parsel/tiles"}]}]}"#,
            base,
        )
        .expect("reads");
        let p = &c[0];
        assert_eq!(p.srids, vec![5254, 4326]);
        assert_eq!(p.wgs84, Some([29.0, 40.0, 31.0, 41.0]));
        assert_eq!(p.items.as_deref(), Some("https://x/ogc/parsel/items"));
        assert_eq!(
            p.tiles.as_deref(),
            Some("https://x/ogc/collections/parsel/tiles")
        );
        let sets = tile_sets(
            r#"{"tilesets":[{"title":"WebMercatorQuad","dataType":"vector","tileMatrixSetURI":"http://www.opengis.net/def/tilematrixset/OGC/1.0/WebMercatorQuad",
              "links":[{"rel":"item","type":"application/vnd.mapbox-vector-tile","href":"https://x/ogc/tiles/WebMercatorQuad/{tileMatrix}/{tileRow}/{tileCol}?f=mvt"}]}]}"#,
            base,
        )
        .expect("reads");
        assert_eq!(sets[0].data_type, "vector");
        assert!(is_web_mercator_quad(&sets[0].matrix_set));
        assert!(
            sets[0]
                .template
                .as_deref()
                .unwrap()
                .contains("{tileMatrix}")
        );
        let m = matrix_set(
            r#"{"id":"TM30","crs":"http://www.opengis.net/def/crs/EPSG/0/5254","tileMatrices":[
              {"id":"0","scaleDenominator":1000000,"cellSize":280,"cornerOfOrigin":"topLeft","pointOfOrigin":[5000000,300000],
               "tileWidth":256,"tileHeight":256,"matrixWidth":4,"matrixHeight":3}]}"#,
        )
        .expect("reads");
        assert_eq!(m.srid, Some(5254));
        assert_eq!(
            (m.matrices[0].x0, m.matrices[0].y0),
            (300_000.0, 5_000_000.0)
        );
        assert!(collections("{}", base).is_err());
    }
}
