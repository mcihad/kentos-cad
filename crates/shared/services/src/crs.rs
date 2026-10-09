//! Coordinate system names as services write them, and the order their axes
//! go in (docs/adr/0208 §5, §6): `EPSG:5254`, `urn:ogc:def:crs:EPSG::5254`,
//! `urn:ogc:def:crs:EPSG:6.18.3:5254`, `http://www.opengis.net/def/crs/EPSG/0/5254`,
//! OGC's CRS84 (longitude first, whatever EPSG says of 4326) and ArcGIS's
//! Web Mercator codes (102100, 102113, 900913). The axis orders are the CRS
//! registry's (`fixtures/crs/v1/registry.json`, checked against PROJ).

use std::sync::OnceLock;

/// The order a system's coordinates are written in.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Axis {
    /// East then north.
    En,
    /// North then east (TUREF's and ED50's TM zones).
    Ne,
    /// Latitude then longitude (EPSG's geographic systems).
    LatLon,
}

/// A system named by a service: its EPSG code, and whether the name says
/// longitude first (CRS84) whatever the code's own order is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CrsName {
    pub srid: u32,
    pub lon_lat: bool,
}

/// The system a service's name means; none for a name it does not read.
pub fn crs_name(name: &str) -> Option<CrsName> {
    let n = name.trim();
    let upper = n.to_ascii_uppercase();
    if upper == "CRS:84"
        || upper == "OGC:CRS84"
        || upper.ends_with("/OGC/1.3/CRS84")
        || upper.ends_with("/OGC/0/CRS84")
        || upper == "URN:OGC:DEF:CRS:OGC:1.3:CRS84"
        || upper == "URN:OGC:DEF:CRS:OGC::CRS84"
    {
        return Some(CrsName {
            srid: 4326,
            lon_lat: true,
        });
    }
    let code = if let Some(rest) = upper.strip_prefix("EPSG:") {
        rest
    } else if upper.starts_with("URN:OGC:DEF:CRS:EPSG:")
        || upper.starts_with("URN:X-OGC:DEF:CRS:EPSG:")
    {
        // urn:ogc:def:crs:EPSG:<version>:<code>, the version may be empty.
        upper.rsplit(':').next()?
    } else if let Some(i) = upper.find("/DEF/CRS/EPSG/") {
        // http://www.opengis.net/def/crs/EPSG/0/5254
        upper[i + 14..].rsplit('/').next()?
    } else if upper.chars().all(|c| c.is_ascii_digit()) && !upper.is_empty() {
        // An ArcGIS wkid.
        &upper
    } else {
        return None;
    };
    let srid: u32 = code.trim().parse().ok()?;
    let srid = match srid {
        102100 | 102113 | 900913 | 3785 => 3857,
        other => other,
    };
    (srid > 0).then_some(CrsName {
        srid,
        lon_lat: false,
    })
}

/// A system of the registry as these rules read it.
#[derive(Clone, Debug)]
struct Known {
    srid: u32,
    axis: Axis,
    degrees: bool,
}

fn registry() -> &'static [Known] {
    static KNOWN: OnceLock<Vec<Known>> = OnceLock::new();
    KNOWN.get_or_init(|| {
        let text = include_str!("../../../../fixtures/crs/v1/registry.json");
        let value: serde_json::Value = serde_json::from_str(text).unwrap_or_default();
        value["systems"]
            .as_array()
            .map(|list| {
                list.iter()
                    .filter_map(|s| {
                        let srid = u32::try_from(s["srid"].as_u64()?).ok()?;
                        let axis = match s["axisOrder"].as_str()? {
                            "ne" => Axis::Ne,
                            "latlon" => Axis::LatLon,
                            _ => Axis::En,
                        };
                        Some(Known {
                            srid,
                            axis,
                            degrees: s["unit"].as_str() == Some("degree"),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default()
    })
}

/// Whether KentOS knows the system (its registry has it, the local one aside).
pub fn known(srid: u32) -> bool {
    srid != 0 && registry().iter().any(|k| k.srid == srid)
}

/// The order EPSG gives the system's axes; east first for a system the
/// registry does not have.
pub fn axis_order(srid: u32) -> Axis {
    registry()
        .iter()
        .find(|k| k.srid == srid)
        .map_or(Axis::En, |k| k.axis)
}

/// Whether the system's coordinates are degrees.
pub fn degrees(srid: u32) -> bool {
    registry().iter().any(|k| k.srid == srid && k.degrees)
}

/// A point written in `name`'s order, as east and north (longitude and
/// latitude): `(a, b)` is the order the document wrote.
pub fn to_east_north(name: CrsName, a: f64, b: f64) -> (f64, f64) {
    if name.lon_lat {
        return (a, b);
    }
    match axis_order(name.srid) {
        Axis::En => (a, b),
        Axis::Ne | Axis::LatLon => (b, a),
    }
}

/// East and north written in the order `srid`'s axes go in when the
/// request follows the system's order (WMS 1.3.0, WFS 1.1 and 2.0, WMTS).
pub fn in_axis_order(srid: u32, east: f64, north: f64) -> (f64, f64) {
    match axis_order(srid) {
        Axis::En => (east, north),
        Axis::Ne | Axis::LatLon => (north, east),
    }
}

/// The name a request writes: `EPSG:<code>`.
pub fn epsg(srid: u32) -> String {
    format!("EPSG:{srid}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_are_read_in_every_form() {
        let tm30 = Some(CrsName {
            srid: 5254,
            lon_lat: false,
        });
        assert_eq!(crs_name("EPSG:5254"), tm30);
        assert_eq!(crs_name("urn:ogc:def:crs:EPSG::5254"), tm30);
        assert_eq!(crs_name("urn:ogc:def:crs:EPSG:6.18.3:5254"), tm30);
        assert_eq!(crs_name("http://www.opengis.net/def/crs/EPSG/0/5254"), tm30);
        assert_eq!(crs_name("102100").map(|c| c.srid), Some(3857));
        assert_eq!(
            crs_name("http://www.opengis.net/def/crs/OGC/1.3/CRS84"),
            Some(CrsName {
                srid: 4326,
                lon_lat: true
            })
        );
        assert_eq!(crs_name("CRS:84").map(|c| c.lon_lat), Some(true));
        assert_eq!(crs_name("EPSG:x"), None);
        assert_eq!(crs_name("AUTO:42001"), None);
    }

    #[test]
    fn axes_go_in_the_registrys_order() {
        assert_eq!(axis_order(5254), Axis::Ne);
        assert_eq!(axis_order(2320), Axis::Ne);
        assert_eq!(axis_order(32636), Axis::En);
        assert_eq!(axis_order(4326), Axis::LatLon);
        assert_eq!(axis_order(3857), Axis::En);
        assert!(known(5254) && !known(0) && !known(1234));
        assert!(degrees(4326) && !degrees(5254));
        assert_eq!(
            in_axis_order(5254, 500_000.0, 4_400_000.0),
            (4_400_000.0, 500_000.0)
        );
        let crs84 = crs_name("CRS:84").unwrap();
        assert_eq!(to_east_north(crs84, 32.8, 39.9), (32.8, 39.9));
        let wgs = crs_name("EPSG:4326").unwrap();
        assert_eq!(to_east_north(wgs, 39.9, 32.8), (32.8, 39.9));
    }
}
