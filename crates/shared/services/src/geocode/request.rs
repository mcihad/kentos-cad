//! A provider's requests (docs/adr/0229 §10): an address to its places and
//! a place (WGS 84) to its address, with their parameters in a fixed order,
//! values percent-encoded as the services' are (`query::encode`),
//! coordinates with 7 decimals and no trailing zeros. A key is not in the
//! address: the host adds `key_param=<key>` last, and the cache's key is the
//! address without it.

use super::providers::{Provider, ProviderKind};
use crate::query::encode;

/// How many candidates a search asks: the window's and a batch's.
pub const SEARCH_LIMIT: u32 = 10;
pub const BATCH_LIMIT: u32 = 5;

/// A search's bias (interactive only): the view's middle and its box, WGS 84.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Bias {
    pub lon: f64,
    pub lat: f64,
    pub west: f64,
    pub south: f64,
    pub east: f64,
    pub north: f64,
}

/// A request a host sends: its address without the key, and the key's parameter when the provider takes one.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GeoRequest {
    pub url: String,
    pub key_param: Option<String>,
}

/// A coordinate as a request writes it: 7 decimals, trailing zeros and the point dropped.
pub fn coord(x: f64) -> String {
    let s = format!("{x:.7}");
    let s = s.trim_end_matches('0').trim_end_matches('.');
    if s == "-0" {
        "0".to_owned()
    } else {
        s.to_owned()
    }
}

fn query(base: &str, params: &[(&str, String)]) -> String {
    let mut out = base.to_owned();
    for (i, (k, v)) in params.iter().enumerate() {
        out.push(if i == 0 { '?' } else { '&' });
        out.push_str(k);
        out.push('=');
        out.push_str(&encode(v));
    }
    out
}

fn key_param(p: &Provider) -> Option<String> {
    p.key_param.clone()
}

/// The request for an address's places.
pub fn forward(p: &Provider, text: &str, limit: u32, bias: Option<&Bias>) -> GeoRequest {
    let text = text.trim().to_owned();
    let mut params: Vec<(&str, String)> = Vec::new();
    let base = match p.kind {
        ProviderKind::Nominatim => {
            params.push(("q", text));
            params.push(("format", "jsonv2".into()));
            params.push(("addressdetails", "1".into()));
            params.push(("limit", limit.to_string()));
            if let Some(c) = &p.country {
                params.push(("countrycodes", c.to_ascii_lowercase()));
            }
            if let Some(b) = bias {
                params.push((
                    "viewbox",
                    format!(
                        "{},{},{},{}",
                        coord(b.west),
                        coord(b.south),
                        coord(b.east),
                        coord(b.north)
                    ),
                ));
            }
            params.push(("accept-language", "tr".into()));
            format!("{}/search", p.url)
        }
        ProviderKind::Photon => {
            params.push(("q", text));
            params.push(("limit", limit.to_string()));
            if let Some(b) = bias {
                params.push(("lat", coord(b.lat)));
                params.push(("lon", coord(b.lon)));
            }
            format!("{}/api", p.url)
        }
        ProviderKind::Pelias => {
            params.push(("text", text));
            params.push(("size", limit.to_string()));
            if let Some(b) = bias {
                params.push(("focus.point.lat", coord(b.lat)));
                params.push(("focus.point.lon", coord(b.lon)));
            }
            if let Some(c) = &p.country {
                params.push(("boundary.country", c.clone()));
            }
            format!("{}/v1/search", p.url)
        }
        ProviderKind::Arcgis => {
            params.push(("SingleLine", text));
            params.push(("f", "json".into()));
            params.push(("outSR", "4326".into()));
            params.push(("outFields", "*".into()));
            params.push(("maxLocations", limit.to_string()));
            if let Some(c) = &p.country {
                params.push(("sourceCountry", c.clone()));
            }
            if let Some(b) = bias {
                params.push(("location", format!("{},{}", coord(b.lon), coord(b.lat))));
            }
            params.push(("langCode", "TR".into()));
            params.push(("forStorage", p.storing.to_string()));
            format!("{}/findAddressCandidates", p.url)
        }
        ProviderKind::Google => {
            params.push(("address", text));
            params.push(("language", "tr".into()));
            if let Some(c) = &p.country {
                params.push(("region", c.to_ascii_lowercase()));
            }
            p.url.clone()
        }
    };
    GeoRequest {
        url: query(&base, &params),
        key_param: key_param(p),
    }
}

/// The request for a place's address.
pub fn reverse(p: &Provider, lon: f64, lat: f64) -> GeoRequest {
    let (x, y) = (coord(lon), coord(lat));
    let (base, params): (String, Vec<(&str, String)>) = match p.kind {
        ProviderKind::Nominatim => (
            format!("{}/reverse", p.url),
            vec![
                ("lat", y),
                ("lon", x),
                ("format", "jsonv2".into()),
                ("addressdetails", "1".into()),
                ("zoom", "18".into()),
                ("accept-language", "tr".into()),
            ],
        ),
        ProviderKind::Photon => (
            format!("{}/reverse", p.url),
            vec![("lat", y), ("lon", x), ("limit", "1".into())],
        ),
        ProviderKind::Pelias => (
            format!("{}/v1/reverse", p.url),
            vec![("point.lat", y), ("point.lon", x), ("size", "1".into())],
        ),
        ProviderKind::Arcgis => (
            format!("{}/reverseGeocode", p.url),
            vec![
                ("location", format!("{x},{y}")),
                ("f", "json".into()),
                ("outSR", "4326".into()),
                ("langCode", "TR".into()),
                ("forStorage", p.storing.to_string()),
            ],
        ),
        ProviderKind::Google => (
            p.url.clone(),
            vec![("latlng", format!("{y},{x}")), ("language", "tr".into())],
        ),
    };
    GeoRequest {
        url: query(&base, &params),
        key_param: key_param(p),
    }
}

/// A batch's columns as the one line a provider is sent (§10): “street door, neighbourhood Mahallesi, district,
/// province, postcode”, empty parts left out, “ Mahallesi” only when the neighbourhood has no MAHALLE or KÖY.
pub fn line_of(
    neighbourhood: &str,
    street: &str,
    door: &str,
    district: &str,
    province: &str,
    postcode: &str,
) -> String {
    use kentos_geometry_core::ops::geocode::locator::Locator;
    let mut parts: Vec<String> = Vec::new();
    let head = format!("{} {}", street.trim(), door.trim());
    if !head.trim().is_empty() {
        parts.push(head.trim().to_owned());
    }
    let n = neighbourhood.trim();
    if !n.is_empty() {
        parts.push(if Locator::tagged_hood(n) {
            n.to_owned()
        } else {
            format!("{n} Mahallesi")
        });
    }
    for x in [district, province, postcode] {
        if !x.trim().is_empty() {
            parts.push(x.trim().to_owned());
        }
    }
    parts.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geocode::providers::Providers;

    #[test]
    fn a_nominatim_search_names_its_parameters_in_order() {
        let p = Providers::default().resolved().remove(0);
        let r = forward(&p, " Atatürk Bulvarı 15, Çankaya ", SEARCH_LIMIT, None);
        assert_eq!(
            r.url,
            "https://nominatim.openstreetmap.org/search?q=Atat%C3%BCrk%20Bulvar%C4%B1%2015,%20%C3%87ankaya&format=jsonv2&addressdetails=1&limit=10&countrycodes=tr&accept-language=tr"
        );
        assert_eq!(r.key_param, None);
        assert_eq!(coord(32.85400000001), "32.854");
        assert_eq!(coord(-0.00000001), "0");
        assert_eq!(
            line_of("Kızılay", "Atatürk Bulvarı", "15", "Çankaya", "Ankara", ""),
            "Atatürk Bulvarı 15, Kızılay Mahallesi, Çankaya, Ankara"
        );
    }
}
