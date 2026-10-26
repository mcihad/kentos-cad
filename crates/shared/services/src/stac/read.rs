//! A STAC API's answers read (docs/adr/0528 §3): an item collection page (`FeatureCollection` of items, its
//! `numberMatched`, its `next` link), an item (STAC 1.0 and 1.1: `eo:bands` and `raster:bands`, or the unified
//! `bands`; `proj:epsg` or `proj:code`, `proj:shape`, `proj:transform` of the asset, else of the item) and its
//! assets, as the contract's types. Whatever the catalog leaves out stays absent; a value of another type than
//! the specification's is left out too (a catalog's mistake is not a reason to lose its items). An asset's address
//! is kept without a signature (`href::stable`); assets are listed by key.

use kentos_contracts::{
    StacAssetInfo, StacBand, StacItemInfo, StacMethod, StacNext, StacRasterBand,
};
use serde_json::Value;

use super::{endpoint, href};

/// A page of a search as the server answered it.
#[derive(Clone, Debug, PartialEq)]
pub struct Page {
    pub items: Vec<StacItemInfo>,
    /// The items the server sent.
    pub returned: u32,
    /// `numberMatched` (or the older `context.matched`).
    pub matched: Option<u64>,
    pub next: Option<StacNext>,
}

fn text(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_owned)
}

fn number(v: &Value, key: &str) -> Option<f64> {
    v.get(key).and_then(Value::as_f64).filter(|x| x.is_finite())
}

fn texts(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

fn numbers<const N: usize>(v: &Value) -> Option<[f64; N]> {
    let a = v.as_array()?;
    if a.len() < N {
        return None;
    }
    let mut out = [0.0; N];
    for (i, slot) in out.iter_mut().enumerate() {
        *slot = a[i].as_f64().filter(|x| x.is_finite())?;
    }
    Some(out)
}

/// `EPSG:32637` (any case) as 32637.
fn code_epsg(code: &str) -> Option<u32> {
    let (auth, n) = code.trim().split_once(':')?;
    if !auth.eq_ignore_ascii_case("epsg") {
        return None;
    }
    n.parse().ok().filter(|v| *v > 0)
}

/// The EPSG code `proj:epsg` or `proj:code` of an object holding the projection fields.
fn epsg_of(v: &Value) -> Option<u32> {
    v.get("proj:epsg")
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .filter(|n| *n > 0)
        .or_else(|| {
            v.get("proj:code")
                .and_then(Value::as_str)
                .and_then(code_epsg)
        })
}

fn shape_of(v: &Value) -> Option<[u32; 2]> {
    let a = v.get("proj:shape")?.as_array()?;
    if a.len() != 2 {
        return None;
    }
    let rows = u32::try_from(a[0].as_u64()?).ok()?;
    let cols = u32::try_from(a[1].as_u64()?).ok()?;
    (rows > 0 && cols > 0).then_some([rows, cols])
}

/// The first six numbers of `proj:transform` (a 9-number one ends in 0, 0, 1).
fn transform_of(v: &Value) -> Option<[f64; 6]> {
    numbers::<6>(v.get("proj:transform")?)
}

fn band_of(v: &Value) -> StacBand {
    let pick = |a: &str, b: &str| text(v, a).or_else(|| text(v, b));
    StacBand {
        name: text(v, "name"),
        common_name: pick("common_name", "eo:common_name"),
        center_wavelength: number(v, "center_wavelength")
            .or_else(|| number(v, "eo:center_wavelength")),
    }
}

fn raster_band_of(v: &Value) -> StacRasterBand {
    StacRasterBand {
        data_type: text(v, "data_type"),
        nodata: number(v, "nodata"),
        scale: number(v, "scale"),
        offset: number(v, "offset"),
        spatial_resolution: number(v, "spatial_resolution"),
        unit: text(v, "unit"),
    }
}

fn list(v: &Value, key: &str) -> Vec<Value> {
    v.get(key)
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

/// An asset read; the item's projection fields are taken when the asset has none.
fn read_asset(key: &str, v: &Value, props: &Value) -> Option<StacAssetInfo> {
    let raw = text(v, "href")?;
    let (stable, signed) = href::stable(&raw);
    let unified = list(v, "bands");
    let eo = list(v, "eo:bands");
    let rb = list(v, "raster:bands");
    // STAC 1.1 folds both extensions into `bands`; the older keys win when both are there.
    let bands: Vec<StacBand> = if eo.is_empty() {
        unified.iter().map(band_of).collect()
    } else {
        eo.iter().map(band_of).collect()
    };
    let raster_bands: Vec<StacRasterBand> = if rb.is_empty() {
        unified
            .iter()
            .filter(|b| {
                b.get("data_type").is_some()
                    || b.get("scale").is_some()
                    || b.get("nodata").is_some()
            })
            .map(raster_band_of)
            .collect()
    } else {
        rb.iter().map(raster_band_of).collect()
    };
    Some(StacAssetInfo {
        key: key.to_owned(),
        href: stable,
        signed,
        title: text(v, "title"),
        media_type: text(v, "type"),
        roles: texts(v, "roles"),
        bands,
        raster_bands,
        epsg: epsg_of(v).or_else(|| epsg_of(props)),
        shape: shape_of(v).or_else(|| shape_of(props)),
        transform: transform_of(v).or_else(|| transform_of(props)),
        region: storage_of(v, props).0,
        requester_pays: storage_of(v, props).1,
    })
}

/// The storage extension's region and requester pays (docs/adr/0644 §1): v1's `storage:region` and
/// `storage:requester_pays` on the asset, else on the item; v2's first of the asset's `storage:refs` among the item's
/// `storage:schemes`.
fn storage_of(v: &Value, props: &Value) -> (Option<String>, bool) {
    let v1 = |o: &Value| {
        (
            text(o, "storage:region"),
            o.get("storage:requester_pays").and_then(Value::as_bool),
        )
    };
    let (r, rp) = v1(v);
    let (pr, prp) = v1(props);
    let (mut sr, mut srp) = (None, None);
    if let (Some(refs), Some(schemes)) = (
        v.get("storage:refs").and_then(Value::as_array),
        props.get("storage:schemes").and_then(Value::as_object),
    ) {
        for key in refs.iter().filter_map(Value::as_str) {
            if let Some(scheme) = schemes.get(key).filter(|s| s.is_object()) {
                sr = text(scheme, "region");
                srp = scheme.get("requester_pays").and_then(Value::as_bool);
                break;
            }
        }
    }
    (r.or(pr).or(sr), rp.or(prp).or(srp).unwrap_or(false))
}

/// An item of the catalog at `catalog` (its endpoint, normalized) read; none-id items cannot be read.
pub fn read_item(catalog: &str, v: &Value) -> Result<StacItemInfo, String> {
    let id = text(v, "id").ok_or("Öğenin kimliği (id) yok.")?;
    let empty = Value::Object(Default::default());
    let props = v.get("properties").unwrap_or(&empty);
    let mut assets: Vec<StacAssetInfo> = Vec::new();
    if let Some(map) = v.get("assets").and_then(Value::as_object) {
        let mut keys: Vec<&String> = map.keys().collect();
        keys.sort();
        for key in keys {
            if let Some(a) = read_asset(key, &map[key], props) {
                assets.push(a);
            }
        }
    }
    let links = list(v, "links");
    let link = |rel: &str| {
        links
            .iter()
            .find(|l| l.get("rel").and_then(Value::as_str) == Some(rel))
            .and_then(|l| text(l, "href"))
    };
    let datetime = text(props, "datetime").or_else(|| text(props, "start_datetime"));
    let end = text(props, "end_datetime");
    Ok(StacItemInfo {
        catalog: catalog.to_owned(),
        collection: text(v, "collection"),
        id,
        datetime,
        end_datetime: end,
        bbox: v.get("bbox").and_then(numbers::<4>),
        cloud_cover: number(props, "eo:cloud_cover"),
        platform: text(props, "platform"),
        instruments: texts(props, "instruments"),
        gsd: number(props, "gsd"),
        epsg: epsg_of(props),
        license: text(props, "license"),
        license_href: link("license").map(|h| href::stable(&h).0),
        self_href: link("self").map(|h| href::stable(&h).0),
        assets,
    })
}

/// The `next` link of a page, its address made absolute against the endpoint.
fn next_of(endpoint_url: &str, links: &[Value]) -> Option<StacNext> {
    let l = links
        .iter()
        .find(|l| l.get("rel").and_then(Value::as_str) == Some("next"))?;
    let href = endpoint::resolve(endpoint_url, &text(l, "href")?)?;
    let method = match text(l, "method")
        .as_deref()
        .map(str::to_ascii_uppercase)
        .as_deref()
    {
        Some("POST") => StacMethod::Post,
        _ => StacMethod::Get,
    };
    Some(StacNext {
        method,
        href,
        body: l.get("body").filter(|b| b.is_object()).cloned(),
        merge: l.get("merge").and_then(Value::as_bool).unwrap_or(false),
    })
}

/// A search answer read: its items (those that cannot be read are skipped), counts and `next` link.
pub fn read_page(endpoint_url: &str, bytes: &[u8]) -> Result<Page, String> {
    let v: Value = serde_json::from_slice(bytes).map_err(|e| format!("JSON okunamadı: {e}"))?;
    let features = v
        .get("features")
        .and_then(Value::as_array)
        .ok_or("Cevapta öğe listesi (features) yok.")?;
    let items: Vec<StacItemInfo> = features
        .iter()
        .filter_map(|f| read_item(endpoint_url, f).ok())
        .collect();
    let matched = v.get("numberMatched").and_then(Value::as_u64).or_else(|| {
        v.get("context")
            .and_then(|c| c.get("matched"))
            .and_then(Value::as_u64)
    });
    Ok(Page {
        items,
        returned: u32::try_from(features.len()).unwrap_or(u32::MAX),
        matched,
        next: next_of(endpoint_url, &list(&v, "links")),
    })
}

/// The items within `max` percent of cloud (an item without a cloud cover stays) and how many were left out.
pub fn within_cloud(items: Vec<StacItemInfo>, max: Option<f64>) -> (Vec<StacItemInfo>, u32) {
    let Some(max) = max else {
        return (items, 0);
    };
    let before = items.len();
    let kept: Vec<StacItemInfo> = items
        .into_iter()
        .filter(|i| i.cloud_cover.is_none_or(|c| c <= max))
        .collect();
    let dropped = u32::try_from(before - kept.len()).unwrap_or(u32::MAX);
    (kept, dropped)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn an_item_keeps_the_assets_by_key_without_signatures() {
        let v = json!({
            "id": "i1", "collection": "c",
            "bbox": [1.0, 2.0, 3.0, 4.0],
            "properties": { "datetime": "2024-07-13T08:29:15.388000Z", "eo:cloud_cover": 3.5, "proj:code": "EPSG:32637", "platform": "sentinel-2a", "instruments": ["msi"] },
            "links": [
                { "rel": "self", "href": "https://x/v1/collections/c/items/i1" },
                { "rel": "license", "href": "https://x/legal?sig=abc" }
            ],
            "assets": {
                "z": { "href": "https://b/z.tif?sig=abc&se=1", "roles": ["data"], "proj:shape": [2, 3], "proj:transform": [10, 0, 100, 0, -10, 200, 0, 0, 1],
                       "raster:bands": [{ "data_type": "uint16", "nodata": 0, "scale": 0.0001, "offset": -0.1 }],
                       "eo:bands": [{ "name": "B04", "common_name": "red", "center_wavelength": 0.665 }] },
                "a": { "href": "s3://b/a.tif" },
                "nohref": { "title": "x" }
            }
        });
        let i = read_item("https://x/v1", &v).unwrap();
        assert_eq!(i.epsg, Some(32637));
        assert_eq!(i.license_href.as_deref(), Some("https://x/legal"));
        assert_eq!(
            i.assets.iter().map(|a| a.key.as_str()).collect::<Vec<_>>(),
            ["a", "z"]
        );
        let z = &i.assets[1];
        assert!(z.signed);
        assert_eq!(z.href, "https://b/z.tif");
        assert_eq!(z.epsg, Some(32637));
        assert_eq!(z.shape, Some([2, 3]));
        assert_eq!(z.transform, Some([10.0, 0.0, 100.0, 0.0, -10.0, 200.0]));
        assert_eq!(z.raster_bands[0].scale, Some(0.0001));
        assert_eq!(z.bands[0].common_name.as_deref(), Some("red"));
        assert!(!i.assets[0].signed);
    }

    #[test]
    fn the_unified_bands_of_stac_1_1_are_read() {
        let v = json!({
            "id": "i", "properties": { "datetime": null, "start_datetime": "2024-01-01T00:00:00Z", "end_datetime": "2024-01-31T00:00:00Z", "proj:epsg": 4326 },
            "assets": { "r": { "href": "https://h/r.tif", "bands": [{ "name": "r", "eo:common_name": "red", "data_type": "int16", "nodata": -9999, "scale": 0.5 }] } }
        });
        let i = read_item("https://x", &v).unwrap();
        assert_eq!(i.datetime.as_deref(), Some("2024-01-01T00:00:00Z"));
        assert_eq!(i.end_datetime.as_deref(), Some("2024-01-31T00:00:00Z"));
        let a = &i.assets[0];
        assert_eq!(a.bands[0].common_name.as_deref(), Some("red"));
        assert_eq!(a.raster_bands[0].data_type.as_deref(), Some("int16"));
        assert_eq!(a.raster_bands[0].nodata, Some(-9999.0));
        assert!(read_item("https://x", &json!({ "properties": {} })).is_err());
    }

    #[test]
    fn a_page_has_its_counts_and_next_link() {
        let page = json!({
            "type": "FeatureCollection", "numberMatched": 4, "numberReturned": 2,
            "features": [{ "id": "a", "assets": {} }, { "assets": {} }],
            "links": [{ "rel": "next", "method": "POST", "href": "/v1/search", "body": { "next": "t" }, "merge": false }]
        });
        let p = read_page("https://x/v1", page.to_string().as_bytes()).unwrap();
        assert_eq!(p.returned, 2);
        assert_eq!(p.items.len(), 1);
        assert_eq!(p.matched, Some(4));
        let n = p.next.unwrap();
        assert_eq!(n.method, StacMethod::Post);
        assert_eq!(n.href, "https://x/v1/search");
        assert!(read_page("https://x", b"[]").is_err());
        assert!(read_page("https://x", b"{").is_err());
    }

    #[test]
    fn cloud_cover_is_checked_again() {
        let mk = |id: &str, c: Option<f64>| StacItemInfo {
            catalog: "https://x".into(),
            collection: None,
            id: id.into(),
            datetime: None,
            end_datetime: None,
            bbox: None,
            cloud_cover: c,
            platform: None,
            instruments: vec![],
            gsd: None,
            epsg: None,
            license: None,
            license_href: None,
            self_href: None,
            assets: vec![],
        };
        let (kept, dropped) = within_cloud(
            vec![
                mk("a", Some(10.0)),
                mk("b", Some(20.0)),
                mk("c", Some(20.5)),
                mk("d", None),
            ],
            Some(20.0),
        );
        assert_eq!(
            kept.iter().map(|i| i.id.as_str()).collect::<Vec<_>>(),
            ["a", "b", "d"]
        );
        assert_eq!(dropped, 1);
    }
}
