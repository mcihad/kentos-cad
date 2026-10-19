//! A STAC catalog's root and its collections read (docs/adr/0528 §3): what the desktop's search window lists
//! before a search: the catalog's name, whether it searches items (a `search` link or the `item-search`
//! conformance class) and takes the query and filter extensions, and each collection's id, title, license and
//! extent. The requests are plain GETs (`<endpoint>`, `<endpoint>/collections`).

use serde_json::Value;

/// A catalog's root as it describes itself.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CatalogInfo {
    pub id: Option<String>,
    pub title: Option<String>,
    pub description: Option<String>,
    pub stac_version: Option<String>,
    /// The root links to a search or says it conforms to item search.
    pub searchable: bool,
    /// It says it conforms to the query extension (`eo:cloud_cover` is asked of the server).
    pub query: bool,
    /// It says it conforms to a filter class (CQL2).
    pub filter: bool,
}

/// A collection as a catalog lists it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CollectionInfo {
    pub id: String,
    pub title: Option<String>,
    pub license: Option<String>,
    /// The first spatial extent `[west, south, east, north]`.
    pub bbox: Option<[f64; 4]>,
    /// The first temporal interval's start and end (an open end is none).
    pub start: Option<String>,
    pub end: Option<String>,
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

/// The request address of the catalog's root and of its collections list.
pub fn root_url(endpoint_url: &str) -> String {
    endpoint_url.trim_end_matches('/').to_owned()
}

pub fn collections_url(endpoint_url: &str) -> String {
    format!("{}/collections", endpoint_url.trim_end_matches('/'))
}

/// The root document read.
pub fn read_catalog(bytes: &[u8]) -> Result<CatalogInfo, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|e| format!("JSON okunamadı: {e}"))?;
    if !v.is_object() {
        return Err("Kök belge bir nesne değil.".into());
    }
    let conforms: Vec<String> = v
        .get("conformsTo")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default();
    let has = |part: &str| conforms.iter().any(|c| c.contains(part));
    let links_search = v.get("links").and_then(Value::as_array).is_some_and(|l| {
        l.iter()
            .any(|x| x.get("rel").and_then(Value::as_str) == Some("search"))
    });
    Ok(CatalogInfo {
        id: text(&v, "id"),
        title: text(&v, "title"),
        description: text(&v, "description"),
        stac_version: text(&v, "stac_version"),
        searchable: links_search || has("item-search"),
        query: has("item-search#query"),
        filter: has("item-search#filter") || has("/cql2-"),
    })
}

/// The collections document read (collections without an id are skipped).
pub fn read_collections(bytes: &[u8]) -> Result<Vec<CollectionInfo>, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|e| format!("JSON okunamadı: {e}"))?;
    let list = v
        .get("collections")
        .and_then(Value::as_array)
        .ok_or("Cevapta koleksiyon listesi (collections) yok.")?;
    let mut out = Vec::new();
    for c in list {
        let Some(id) = text(c, "id") else { continue };
        let extent = c.get("extent");
        let bbox = extent
            .and_then(|e| e.get("spatial"))
            .and_then(|s| s.get("bbox"))
            .and_then(Value::as_array)
            .and_then(|b| b.first())
            .and_then(Value::as_array)
            .and_then(|a| {
                let n: Vec<f64> = a.iter().filter_map(Value::as_f64).collect();
                (n.len() >= 4).then(|| [n[0], n[1], n[2], n[3]])
            });
        let interval = extent
            .and_then(|e| e.get("temporal"))
            .and_then(|s| s.get("interval"))
            .and_then(Value::as_array)
            .and_then(|b| b.first())
            .and_then(Value::as_array);
        let at = |i: usize| {
            interval
                .and_then(|a| a.get(i))
                .and_then(Value::as_str)
                .map(str::to_owned)
        };
        out.push(CollectionInfo {
            id,
            title: text(c, "title"),
            license: text(c, "license"),
            bbox,
            start: at(0),
            end: at(1),
        });
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_root_and_its_collections_are_read() {
        let root = br#"{"stac_version":"1.0.0","type":"Catalog","id":"earth-search","title":"Earth Search","description":"d",
            "links":[{"rel":"search","href":"https://x/v1/search"}],
            "conformsTo":["https://api.stacspec.org/v1.0.0/item-search","https://api.stacspec.org/v1.0.0/item-search#query"]}"#;
        let c = read_catalog(root).unwrap();
        assert_eq!(c.title.as_deref(), Some("Earth Search"));
        assert!(c.searchable && c.query && !c.filter);
        assert!(read_catalog(b"[]").is_err());
        let cols = br#"{"collections":[
            {"id":"sentinel-2-l2a","title":"Sentinel-2 Level-2A","license":"proprietary","extent":{"spatial":{"bbox":[[-180,-90,180,90]]},"temporal":{"interval":[["2015-06-27T10:25:31Z",null]]}}},
            {"title":"no id"}]}"#;
        let l = read_collections(cols).unwrap();
        assert_eq!(l.len(), 1);
        assert_eq!(l[0].bbox, Some([-180.0, -90.0, 180.0, 90.0]));
        assert_eq!(l[0].start.as_deref(), Some("2015-06-27T10:25:31Z"));
        assert_eq!(l[0].end, None);
        assert!(read_collections(b"{}").is_err());
        assert_eq!(collections_url("https://x/v1/"), "https://x/v1/collections");
    }
}
