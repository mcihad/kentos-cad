//! Servis bilgisi's answers as rows (docs/adr/0208 §11): a WMS
//! GetFeatureInfo in JSON or GeoJSON, GML, plain text (GeoServer's and
//! MapServer's forms) or HTML, and ArcGIS's `identify`; each found object a
//! row of its layer's name and its fields in order.

use kentos_contracts::{ServiceKind, ServiceLayer};
use serde::Serialize;

use crate::request::{self, Request};

/// Half the side of the small map asked around the point, pixels: the
/// answer does not depend on the view's size.
const HALF: u32 = 50;
/// The info formats a WMS is asked in, in turn, until one is answered.
pub const FORMATS: [&str; 3] = ["application/json", "text/plain", "application/vnd.ogc.gml"];

/// The requests that ask `s` about the point (`x`, `y`) (east and north in
/// the service's system `srid`, `units` its units a screen pixel), each with
/// the media type its answer is read as: a WMS's GetFeatureInfo in each of
/// [`FORMATS`], an ArcGIS service's `identify`; none for the other kinds.
pub fn requests(s: &ServiceLayer, srid: u32, x: f64, y: f64, units: f64) -> Vec<(String, Request)> {
    let side = 2 * HALF + 1;
    let half = units * f64::from(HALF) + units / 2.0;
    let bbox = [x - half, y - half, x + half, y + half];
    match s.kind {
        ServiceKind::Wms => {
            let dims: Vec<(String, String)> = s
                .params
                .iter()
                .map(|q| (q.name.clone(), q.value.clone()))
                .collect();
            FORMATS
                .iter()
                .map(|f| {
                    let map = request::wms::Map {
                        base: &s.url,
                        version: s.version.as_deref().unwrap_or("1.3.0"),
                        layers: &s.layers,
                        styles: s.style.as_deref().unwrap_or(""),
                        srid,
                        bbox,
                        width: side,
                        height: side,
                        format: s.format.as_deref().unwrap_or("image/png"),
                        transparent: s.transparent,
                        params: &dims,
                    };
                    (
                        (*f).to_owned(),
                        Request::get(request::wms::get_feature_info(&map, HALF, HALF, f, 10)),
                    )
                })
                .collect()
        }
        ServiceKind::Arcgis => vec![(
            "application/json".to_owned(),
            Request::get(request::arcgis::identify(
                &s.url, x, y, srid, bbox, side, side, &s.layers,
            )),
        )],
        _ => Vec::new(),
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct InfoRow {
    pub layer: String,
    pub fields: Vec<(String, String)>,
}

fn value_text(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::Null => String::new(),
        serde_json::Value::String(s) => s.clone(),
        other => other.to_string(),
    }
}

fn object_fields(o: &serde_json::Map<String, serde_json::Value>) -> Vec<(String, String)> {
    o.iter().map(|(k, v)| (k.clone(), value_text(v))).collect()
}

/// The rows of an answer of media type `media`.
pub fn read(media: &str, body: &str) -> Vec<InfoRow> {
    let m = media.to_ascii_lowercase();
    let trimmed = body.trim_start();
    if (m.contains("json") || trimmed.starts_with('{'))
        && let Ok(v) = serde_json::from_str::<serde_json::Value>(body)
    {
        return from_json(&v);
    }
    if (m.contains("gml") || m.contains("xml") || trimmed.starts_with("<?xml"))
        && let Some(rows) = from_gml(body)
    {
        return rows;
    }
    if m.contains("html") || trimmed.to_ascii_lowercase().starts_with("<html") {
        let text = crate::attribution::credit(body).text;
        return if text.is_empty() {
            Vec::new()
        } else {
            vec![InfoRow {
                layer: String::new(),
                fields: vec![("Bilgi".to_owned(), text)],
            }]
        };
    }
    from_text(body)
}

fn from_json(v: &serde_json::Value) -> Vec<InfoRow> {
    // ArcGIS identify: results with layerName and attributes.
    if let Some(results) = v["results"].as_array() {
        return results
            .iter()
            .map(|r| InfoRow {
                layer: r["layerName"].as_str().unwrap_or("").to_owned(),
                fields: r["attributes"]
                    .as_object()
                    .map(object_fields)
                    .unwrap_or_default(),
            })
            .collect();
    }
    // GeoJSON: a collection's features (GeoServer names the type in the id: `parsel.12`).
    let features: Vec<&serde_json::Value> = match v["features"].as_array() {
        Some(f) => f.iter().collect(),
        None if v["type"] == "Feature" => vec![v],
        None => Vec::new(),
    };
    features
        .into_iter()
        .map(|f| InfoRow {
            layer: f["id"]
                .as_str()
                .and_then(|id| id.rsplit_once('.').map(|(t, _)| t.to_owned()))
                .unwrap_or_default(),
            fields: f["properties"]
                .as_object()
                .map(object_fields)
                .unwrap_or_default(),
        })
        .collect()
}

/// GML's feature members: each a member's simple children (not its geometry).
fn from_gml(body: &str) -> Option<Vec<InfoRow>> {
    let doc = crate::caps::xml(body).ok()?;
    let mut rows = Vec::new();
    for m in doc.descendants().filter(|n| {
        n.is_element()
            && matches!(
                n.tag_name().name(),
                "featureMember" | "member" | "featureMembers"
            )
    }) {
        for f in m.children().filter(|c| c.is_element()) {
            let fields: Vec<(String, String)> = f
                .children()
                .filter(|c| c.is_element() && !c.children().any(|g| g.is_element()))
                .map(|c| {
                    (
                        c.tag_name().name().to_owned(),
                        crate::caps::text_of(c).unwrap_or_default(),
                    )
                })
                .collect();
            if !fields.is_empty() {
                rows.push(InfoRow {
                    layer: f.tag_name().name().to_owned(),
                    fields,
                });
            }
        }
    }
    Some(rows)
}

/// Plain text: GeoServer's `Results for FeatureType 'x':` blocks of
/// `name = value` lines, MapServer's `Layer 'x'` and `Feature n:` blocks;
/// anything else one row of its text.
fn from_text(body: &str) -> Vec<InfoRow> {
    let mut rows: Vec<InfoRow> = Vec::new();
    let mut layer = String::new();
    let mut current: Option<InfoRow> = None;
    let quoted = |line: &str| {
        let a = line.find('\'')?;
        let b = line.rfind('\'')?;
        (b > a).then(|| line[a + 1..b].to_owned())
    };
    for line in body.lines() {
        let t = line.trim();
        if t.starts_with("Results for FeatureType") || t.starts_with("Layer '") {
            if let Some(c) = current.take() {
                rows.push(c);
            }
            layer = quoted(t).unwrap_or_default();
        } else if t.starts_with("Feature ") && t.ends_with(':')
            || t.chars().all(|c| c == '-') && !t.is_empty()
        {
            if let Some(c) = current.take() {
                rows.push(c);
            }
        } else if let Some((k, v)) = t.split_once(" = ").or_else(|| t.split_once('=')) {
            let row = current.get_or_insert_with(|| InfoRow {
                layer: layer.clone(),
                fields: Vec::new(),
            });
            row.fields
                .push((k.trim().to_owned(), v.trim().trim_matches('\'').to_owned()));
        }
    }
    if let Some(c) = current.take() {
        rows.push(c);
    }
    if rows.is_empty() && !body.trim().is_empty() && !body.contains("no features were found") {
        rows.push(InfoRow {
            layer: String::new(),
            fields: vec![("Bilgi".to_owned(), body.trim().to_owned())],
        });
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn answers_in_every_form() {
        let j = read(
            "application/json",
            r#"{"type":"FeatureCollection","features":[{"type":"Feature","id":"parsel.12","properties":{"ADA":"104","PARSEL":7,"ALAN":512.4}}]}"#,
        );
        assert_eq!(j[0].layer, "parsel");
        // serde_json keeps a map's keys sorted.
        assert_eq!(j[0].fields[2], ("PARSEL".to_owned(), "7".to_owned()));
        let a = read(
            "application/json",
            r#"{"results":[{"layerName":"İmar","attributes":{"FONK":"Konut"}}]}"#,
        );
        assert_eq!(a[0].layer, "İmar");
        let t = read(
            "text/plain",
            "Results for FeatureType 'kadastro:parsel':\n--------------------------------------------\nada = 104\nparsel = 7\n--------------------------------------------\n",
        );
        assert_eq!(t.len(), 1);
        assert_eq!(t[0].layer, "kadastro:parsel");
        assert_eq!(
            t[0].fields,
            vec![
                ("ada".to_owned(), "104".to_owned()),
                ("parsel".to_owned(), "7".to_owned())
            ]
        );
        let g = read(
            "application/vnd.ogc.gml",
            r#"<?xml version="1.0"?><wfs:FeatureCollection xmlns:wfs="w" xmlns:gml="g" xmlns:k="k"><gml:featureMember><k:parsel><k:ada>104</k:ada><k:geom><gml:Point><gml:pos>1 2</gml:pos></gml:Point></k:geom></k:parsel></gml:featureMember></wfs:FeatureCollection>"#,
        );
        assert_eq!(g[0].layer, "parsel");
        assert_eq!(g[0].fields, vec![("ada".to_owned(), "104".to_owned())]);
        let h = read(
            "text/html",
            "<html><body><table><tr><td>ADA</td><td>104</td></tr></table></body></html>",
        );
        assert_eq!(h[0].fields[0].1, "ADA 104");
        assert!(read("text/plain", "no features were found\n").is_empty());
    }
}
