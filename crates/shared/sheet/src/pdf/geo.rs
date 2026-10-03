//! GeoPDF (design §9a): each placed map frame gets a viewport (`VP`) whose
//! geographic measure (`/Subtype /GEO`) ties four points of the frame, in
//! the viewport's unit square (`LPTS`), to their latitude and longitude
//! (`GPTS`) in the system the host names (`GCS`: its WKT, its EPSG code).
//! ISO 32000-2 §12.10 and the OGC best practice; GDAL, QGIS and Acrobat
//! read coordinates off the map with it.
//!
//! The corners' latitude and longitude come from the core's own inverse of
//! the transverse Mercator projection (`geodesy.rs`) when the system is
//! one; else from the host. They are written with ten decimals of a
//! degree (a hundredth of a millimetre): the PDF writer's numbers are
//! 32-bit floats (a few decimetres at these magnitudes), so the array is
//! set into the finished file in place of a placeholder of the same length.

use serde::{Deserialize, Serialize};
#[cfg(feature = "ts")]
use ts_rs::TS;

use super::content::PT;
use super::maps::Frame;
use crate::display::MapPrim;
use crate::geodesy::{TmParams, tm_inverse};
use crate::units::Um;

/// A transverse Mercator system as the CRS registry (`fixtures/crs/v1/registry.json`,
/// `geo/crs.ts`) holds it: what `tmWkt` writes as WKT 1.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "ts", derive(TS))]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
#[cfg_attr(feature = "ts", ts(export, export_to = "sheet/"))]
pub struct TmCrs {
    /// “TUREF / TM36”.
    pub name: String,
    /// The registry's datum: “TUREF”, “ED50”, “WGS84”.
    pub datum: String,
    /// The registry's ellipsoid: “GRS80”, “International 1924”, “WGS84”.
    pub ellipsoid: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "ts", ts(optional))]
    pub epsg: Option<u32>,
    pub tm: TmParams,
}

/// The WKT 1 (OGC flavour, as GDAL writes it) of a transverse Mercator system.
pub fn tm_wkt(crs: &TmCrs) -> String {
    // The registry writes “WGS84”, the web's crs.ts too; “WGS 84” is read the same.
    let (geog, datum, geog_epsg) = match crs.datum.replace(' ', "").as_str() {
        "TUREF" => (
            "TUREF".to_owned(),
            "Turkish_National_Reference_Frame".to_owned(),
            Some(5252),
        ),
        "ED50" => (
            "ED50".to_owned(),
            "European_Datum_1950".to_owned(),
            Some(4230),
        ),
        "WGS84" => ("WGS 84".to_owned(), "WGS_1984".to_owned(), Some(4326)),
        _ => (crs.datum.clone(), crs.datum.replace(' ', "_"), None),
    };
    let spheroid = match crs.ellipsoid.replace(' ', "").as_str() {
        "GRS80" => "GRS 1980".to_owned(),
        "WGS84" => "WGS 84".to_owned(),
        _ => crs.ellipsoid.clone(),
    };
    let t = &crs.tm;
    let authority = |code: Option<u32>| {
        code.map_or_else(String::new, |c| format!(r#",AUTHORITY["EPSG","{c}"]"#))
    };
    format!(
        concat!(
            r#"PROJCS["{name}",GEOGCS["{geog}",DATUM["{datum}",SPHEROID["{spheroid}",{a},{rf}]],"#,
            r#"PRIMEM["Greenwich",0],UNIT["degree",0.0174532925199433]{geog_auth}],"#,
            r#"PROJECTION["Transverse_Mercator"],PARAMETER["latitude_of_origin",0],"#,
            r#"PARAMETER["central_meridian",{cm}],PARAMETER["scale_factor",{k}],"#,
            r#"PARAMETER["false_easting",{fe}],PARAMETER["false_northing",{fnn}],UNIT["metre",1],"#,
            r#"AXIS["Easting",EAST],AXIS["Northing",NORTH]{auth}]"#
        ),
        name = crs.name.replace('"', "'"),
        geog = geog,
        datum = datum,
        spheroid = spheroid,
        a = t.semi_major,
        rf = t.inverse_flattening,
        geog_auth = authority(geog_epsg),
        cm = t.central_meridian,
        k = t.scale_factor,
        fe = t.false_easting,
        fnn = t.false_northing,
        auth = authority(crs.epsg),
    )
}

/// A map frame's viewport on its page.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Viewport {
    /// The frame's box on the page (points): left, bottom, right, top.
    pub bbox: [f32; 4],
    /// The content box's corners in the viewport's unit square: bottom left first, anticlockwise.
    pub lpts: [[f32; 2]; 4],
    /// Their latitude and longitude, degrees.
    pub gpts: [[f64; 2]; 4],
}

/// The viewport of a placed map frame; none without a place or a way to its latitudes.
pub(super) fn viewport(
    page_height: Um,
    m: &MapPrim,
    tm: Option<&TmParams>,
    host: Option<&[[f64; 2]; 4]>,
) -> Option<Viewport> {
    m.view.center?;
    let frame = Frame::of(m);
    // Top left, top right, bottom right, bottom left on the paper; the host's in the same order.
    let paper = frame.corners();
    let latlon: [[f64; 2]; 4] = match (tm, host) {
        (Some(tm), _) => {
            let mut out = [[0.0; 2]; 4];
            for (o, c) in out.iter_mut().zip(paper) {
                let g = frame.ground(c)?;
                let geo = tm_inverse(tm, g[0], g[1])?;
                *o = [geo.lat, geo.lon];
            }
            out
        }
        (None, Some(h)) => *h,
        (None, None) => return None,
    };
    let h = f64::from(page_height) * PT;
    let page: Vec<[f64; 2]> = paper.iter().map(|p| [p[0] * PT, h - p[1] * PT]).collect();
    let (mut x0, mut y0, mut x1, mut y1) = (f64::MAX, f64::MAX, f64::MIN, f64::MIN);
    for p in &page {
        x0 = x0.min(p[0]);
        y0 = y0.min(p[1]);
        x1 = x1.max(p[0]);
        y1 = y1.max(p[1]);
    }
    if x1 - x0 <= 0.0 || y1 - y0 <= 0.0 {
        return None;
    }
    let unit = |p: [f64; 2]| {
        [
            ((p[0] - x0) / (x1 - x0)) as f32,
            ((p[1] - y0) / (y1 - y0)) as f32,
        ]
    };
    // Bottom left, top left, top right, bottom right (the specification's default order).
    let order = [3usize, 0, 1, 2];
    Some(Viewport {
        bbox: [x0 as f32, y0 as f32, x1 as f32, y1 as f32],
        lpts: order.map(|i| unit(page[i])),
        gpts: order.map(|i| latlon[i]),
    })
}

/// The `GPTS` array's text: ten decimals of a degree, latitude first.
pub(super) fn gpts_text(v: &Viewport) -> String {
    let nums: Vec<String> = v
        .gpts
        .iter()
        .flat_map(|ll| [format!("{:.10}", ll[0]), format!("{:.10}", ll[1])])
        .collect();
    format!("[{}]", nums.join(" "))
}

/// The placeholder a viewport's `GPTS` is written as (a literal string of this length, its
/// parentheses included), replaced in the finished file by the array padded with spaces.
pub(super) fn placeholder(n: usize) -> String {
    let head = format!("KENTOSGPTS{n:06}");
    format!("{head}{}", "x".repeat(PLACEHOLDER_LEN - head.len()))
}

pub(super) const PLACEHOLDER_LEN: usize = 200;

/// The finished file with every placeholder replaced (its length unchanged, so the
/// cross-reference table still holds).
pub(super) fn patch(mut bytes: Vec<u8>, arrays: &[String]) -> Vec<u8> {
    for (n, text) in arrays.iter().enumerate() {
        let needle = format!("({})", placeholder(n));
        let Some(at) = find(&bytes, needle.as_bytes()) else {
            continue;
        };
        let mut with = text.clone().into_bytes();
        with.resize(needle.len(), b' ');
        if with.len() == needle.len() {
            bytes[at..at + needle.len()].copy_from_slice(&with);
        }
    }
    bytes
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tm36() -> TmCrs {
        TmCrs {
            name: "TUREF / TM36".into(),
            datum: "TUREF".into(),
            ellipsoid: "GRS80".into(),
            epsg: Some(5256),
            tm: TmParams {
                central_meridian: 36.0,
                scale_factor: 1.0,
                false_easting: 500_000.0,
                false_northing: 0.0,
                semi_major: 6_378_137.0,
                inverse_flattening: 298.257_222_101,
            },
        }
    }

    #[test]
    fn a_tm_system_s_wkt_names_its_parameters_and_its_code() {
        let w = tm_wkt(&tm36());
        assert!(w.starts_with(r#"PROJCS["TUREF / TM36",GEOGCS["TUREF",DATUM["Turkish_National_Reference_Frame",SPHEROID["GRS 1980",6378137,298.257222101]]"#), "{w}");
        assert!(w.contains(r#"PARAMETER["central_meridian",36]"#));
        assert!(w.contains(r#"PARAMETER["false_easting",500000]"#));
        assert!(
            w.ends_with(r#"AXIS["Northing",NORTH],AUTHORITY["EPSG","5256"]]"#),
            "{w}"
        );
    }

    /// The registry's spellings (“WGS84”) are read as WKT's.
    #[test]
    fn the_registry_s_names_are_read() {
        let mut c = tm36();
        c.name = "WGS 84 / UTM 36N".into();
        c.datum = "WGS84".into();
        c.ellipsoid = "WGS84".into();
        c.epsg = Some(32636);
        c.tm.inverse_flattening = 298.257_223_563;
        let w = tm_wkt(&c);
        assert!(
            w.contains(
                r#"GEOGCS["WGS 84",DATUM["WGS_1984",SPHEROID["WGS 84",6378137,298.257223563]]"#
            ),
            "{w}"
        );
        assert!(w.contains(r#"AUTHORITY["EPSG","4326"]"#));
    }

    #[test]
    fn a_placeholder_is_replaced_in_place() {
        let p = placeholder(3);
        assert_eq!(p.len(), PLACEHOLDER_LEN);
        let file = format!("a /GPTS ({p}) b").into_bytes();
        let out = patch(
            file.clone(),
            &[String::new(), String::new(), String::new(), "[1 2]".into()],
        );
        assert_eq!(out.len(), file.len());
        assert!(
            String::from_utf8(out)
                .unwrap()
                .starts_with("a /GPTS [1 2]   ")
        );
    }
}
