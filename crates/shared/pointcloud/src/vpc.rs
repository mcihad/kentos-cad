//! Virtual point clouds (docs/adr/0207 §8): QGIS's and PDAL wrench's `.vpc`, a
//! STAC ItemCollection (a GeoJSON FeatureCollection of STAC Items), each Item a
//! file: `assets.data.href` (relative to the `.vpc`'s folder), `pc:count`,
//! `proj:bbox` (3D in the file's system), `proj:epsg` or `proj:wkt2`. What a
//! `.vpc` leaves out is read from the member's own header.

use serde_json::{Value, json};

use crate::{PcError, Result};

/// The most members a `.vpc` may list.
pub const MAX_MEMBERS: usize = 4096;
/// The most bytes a `.vpc` may have.
pub const MAX_BYTES: usize = 64 * 1024 * 1024;

/// A member of a virtual cloud as the `.vpc` lists it.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    /// As written (relative to the `.vpc`, or absolute, or an address).
    pub href: String,
    pub count: Option<u64>,
    /// `[x₁, y₁, z₁, x₂, y₂, z₂]` in the file's system.
    pub bounds: Option<[f64; 6]>,
    pub epsg: Option<u32>,
}

/// The format a member's name says: `copc`, `laz`, `las` or `xyz`.
pub fn format_of(href: &str) -> &'static str {
    let lower = href.to_ascii_lowercase();
    let path = lower.split(['?', '#']).next().unwrap_or("");
    if path.ends_with(".copc.laz") {
        "copc"
    } else if path.ends_with(".laz") {
        "laz"
    } else if path.ends_with(".las") {
        "las"
    } else {
        "xyz"
    }
}

/// The members of a `.vpc`'s text.
pub fn read(text: &str) -> Result<Vec<Member>> {
    if text.len() > MAX_BYTES {
        return Err(PcError::new("Sanal bulut dosyası 64 MB'tan büyük."));
    }
    let v: Value = serde_json::from_str(text)
        .map_err(|e| PcError::new(format!("Sanal bulut dosyası JSON değil: {e}.")))?;
    if v.get("type").and_then(Value::as_str) != Some("FeatureCollection") {
        return Err(PcError::new(
            "Sanal bulut dosyası bir STAC ItemCollection (FeatureCollection) değil.",
        ));
    }
    let features = v
        .get("features")
        .and_then(Value::as_array)
        .ok_or_else(|| PcError::new("Sanal bulut dosyasında üye (features) yok."))?;
    if features.len() > MAX_MEMBERS {
        return Err(PcError::new(format!(
            "Sanal bulutun en çok {MAX_MEMBERS} üyesi olabilir; dosya {} diyor.",
            features.len()
        )));
    }
    let mut out = Vec::with_capacity(features.len());
    for (i, f) in features.iter().enumerate() {
        let href = f
            .pointer("/assets/data/href")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                PcError::new(format!("{}. üyenin dosyası (assets.data.href) yok.", i + 1))
            })?;
        let p = f.get("properties").unwrap_or(&Value::Null);
        let count = p.get("pc:count").and_then(Value::as_u64);
        let bounds = p.get("proj:bbox").and_then(Value::as_array).and_then(|b| {
            let n: Option<Vec<f64>> = b.iter().map(Value::as_f64).collect();
            match n?.as_slice() {
                [x1, y1, z1, x2, y2, z2] => Some([*x1, *y1, *z1, *x2, *y2, *z2]),
                [x1, y1, x2, y2] => Some([*x1, *y1, 0.0, *x2, *y2, 0.0]),
                _ => None,
            }
        });
        let epsg = p
            .get("proj:epsg")
            .and_then(Value::as_u64)
            .and_then(|e| u32::try_from(e).ok())
            .or_else(|| {
                p.get("proj:wkt2")
                    .and_then(Value::as_str)
                    .and_then(|w| crate::crs::from_wkt(w).epsg)
            });
        out.push(Member {
            href: href.to_owned(),
            count,
            bounds,
            epsg,
        });
    }
    if out.is_empty() {
        return Err(PcError::new("Sanal bulut dosyasında üye yok."));
    }
    Ok(out)
}

/// A member to write: its href, points, bounds and format.
#[derive(Clone, Debug, PartialEq)]
pub struct Item {
    pub href: String,
    pub count: u64,
    pub bounds: [f64; 6],
    /// The plan's corners in WGS 84 (longitude, latitude), when the system is known.
    pub wgs84: Option<[[f64; 2]; 4]>,
}

/// A `.vpc`'s text for these members in the system `epsg` (`wkt` too, when given).
pub fn write(items: &[Item], epsg: Option<u32>, wkt: Option<&str>) -> String {
    let features: Vec<Value> = items
        .iter()
        .enumerate()
        .map(|(i, m)| {
            let id = std::path::Path::new(&m.href).file_name().map_or_else(
                || format!("uye-{}", i + 1),
                |n| n.to_string_lossy().into_owned(),
            );
            let (geometry, bbox) = match &m.wgs84 {
                Some(c) => {
                    let lons = c.iter().map(|p| p[0]);
                    let lats = c.iter().map(|p| p[1]);
                    let (w, e) = lons.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
                        (a.min(v), b.max(v))
                    });
                    let (s, n) = lats.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), v| {
                        (a.min(v), b.max(v))
                    });
                    let ring: Vec<Value> = c
                        .iter()
                        .chain(std::iter::once(&c[0]))
                        .map(|p| json!([p[0], p[1]]))
                        .collect();
                    (
                        json!({ "type": "Polygon", "coordinates": [ring] }),
                        Some(json!([w, s, m.bounds[2], e, n, m.bounds[5]])),
                    )
                }
                None => (Value::Null, None),
            };
            let mut props = serde_json::Map::new();
            props.insert("datetime".into(), json!("1970-01-01T00:00:00Z"));
            props.insert("pc:count".into(), json!(m.count));
            props.insert("pc:type".into(), json!("lidar"));
            props.insert("pc:encoding".into(), json!("?"));
            props.insert("pc:schemas".into(), json!([]));
            props.insert("proj:bbox".into(), json!(m.bounds));
            if let Some(e) = epsg {
                props.insert("proj:epsg".into(), json!(e));
            }
            if let Some(w) = wkt {
                props.insert("proj:wkt2".into(), json!(w));
            }
            let mut f = serde_json::Map::new();
            f.insert("type".into(), json!("Feature"));
            f.insert("stac_version".into(), json!("1.0.0"));
            f.insert(
                "stac_extensions".into(),
                json!([
                    "https://stac-extensions.github.io/pointcloud/v1.0.0/schema.json",
                    "https://stac-extensions.github.io/projection/v1.1.0/schema.json"
                ]),
            );
            f.insert("id".into(), json!(id));
            f.insert("geometry".into(), geometry);
            if let Some(b) = bbox {
                f.insert("bbox".into(), b);
            }
            f.insert("properties".into(), Value::Object(props));
            f.insert("links".into(), json!([]));
            f.insert(
                "assets".into(),
                json!({ "data": { "href": m.href, "roles": ["data"] } }),
            );
            Value::Object(f)
        })
        .collect();
    let doc = json!({ "type": "FeatureCollection", "features": features });
    serde_json::to_string_pretty(&doc).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_vpc_round_trips() {
        let items = vec![
            Item {
                href: "./a.copc.laz".into(),
                count: 10,
                bounds: [0.0, 0.0, 1.0, 10.0, 10.0, 2.0],
                wgs84: Some([[30.0, 39.0], [30.1, 39.0], [30.1, 39.1], [30.0, 39.1]]),
            },
            Item {
                href: "b.laz".into(),
                count: 5,
                bounds: [10.0, 0.0, 1.0, 20.0, 10.0, 3.0],
                wgs84: None,
            },
        ];
        let text = write(&items, Some(5254), None);
        let back = read(&text).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!(back[0].href, "./a.copc.laz");
        assert_eq!(back[0].count, Some(10));
        assert_eq!(back[1].bounds, Some([10.0, 0.0, 1.0, 20.0, 10.0, 3.0]));
        assert_eq!(back[0].epsg, Some(5254));
        assert_eq!(format_of("./a.copc.laz"), "copc");
        assert_eq!(format_of("x/B.LAZ?sig=1"), "laz");
        assert!(read("{}").is_err());
        assert!(read(r#"{"type":"FeatureCollection","features":[{}]}"#).is_err());
    }
}
