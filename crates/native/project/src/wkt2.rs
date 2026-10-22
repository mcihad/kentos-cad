//! The registry's systems as WKT2 (ISO 19162:2019) and as a layer's source system record (docs/adr/0583 §1). The
//! text is EPSG's definition written from the registry's numbers, with EPSG's own axis order and its code; PROJ reads
//! it as the code's system (scripts/fixtures/layer_crs_cases.py checks it point by point).

use kentos_contracts::LayerSourceCrs;

use crate::crs::{System, system};

const DEGREE: &str = "ANGLEUNIT[\"degree\",0.0174532925199433]";
const METRE: &str = "LENGTHUNIT[\"metre\",1]";

/// A datum's name and its ellipsoid: name, semi-major axis, inverse flattening.
fn datum(name: &str) -> Option<(&'static str, &'static str, &'static str, f64, f64)> {
    Some(match name {
        "TUREF" => (
            "TUREF",
            "Turkish National Reference Frame",
            "GRS 1980",
            6_378_137.0,
            298.257222101,
        ),
        "ED50" => (
            "ED50",
            "European Datum 1950",
            "International 1924",
            6_378_388.0,
            297.0,
        ),
        "WGS84" => (
            "WGS 84",
            "World Geodetic System 1984",
            "WGS 84",
            6_378_137.0,
            298.257223563,
        ),
        _ => return None,
    })
}

/// A number as WKT writes it: the shortest text that reads back as it.
fn num(v: f64) -> String {
    if v == v.trunc() && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

fn geog(datum_name: &str, ellipsoid: &str, a: f64, rf: f64) -> String {
    format!(
        "DATUM[\"{datum_name}\",ELLIPSOID[\"{ellipsoid}\",{},{},{METRE}]],PRIMEM[\"Greenwich\",0,{DEGREE}]",
        num(a),
        num(rf)
    )
}

/// A system of the registry as WKT2; none for the local system and a datum not known.
pub fn wkt2(s: &System) -> Option<String> {
    if s.is_local() {
        return None;
    }
    let (base, datum_name, ellipsoid, a, rf) = datum(&s.datum)?;
    let datum_text = geog(datum_name, ellipsoid, a, rf);
    let id = format!("ID[\"EPSG\",{}]", s.srid);
    if s.kind == "geographic" {
        return Some(format!(
            "GEOGCRS[\"{}\",{datum_text},CS[ellipsoidal,2],AXIS[\"geodetic latitude (Lat)\",north,ORDER[1],{DEGREE}],AXIS[\"geodetic longitude (Lon)\",east,ORDER[2],{DEGREE}],{id}]",
            s.name
        ));
    }
    let (method, params) = match s.projection.as_deref()? {
        "Transverse Mercator" | "UTM" => (
            "METHOD[\"Transverse Mercator\",ID[\"EPSG\",9807]]".to_owned(),
            format!(
                "PARAMETER[\"Latitude of natural origin\",0,{DEGREE},ID[\"EPSG\",8801]],PARAMETER[\"Longitude of natural origin\",{},{DEGREE},ID[\"EPSG\",8802]],PARAMETER[\"Scale factor at natural origin\",{},SCALEUNIT[\"unity\",1],ID[\"EPSG\",8805]],PARAMETER[\"False easting\",{},{METRE},ID[\"EPSG\",8806]],PARAMETER[\"False northing\",{},{METRE},ID[\"EPSG\",8807]]",
                num(s.central_meridian?),
                num(s.scale_factor?),
                num(s.false_easting.unwrap_or(0.0)),
                num(s.false_northing.unwrap_or(0.0))
            ),
        ),
        "Pseudo-Mercator" => (
            "METHOD[\"Popular Visualisation Pseudo Mercator\",ID[\"EPSG\",1024]]".to_owned(),
            format!(
                "PARAMETER[\"Latitude of natural origin\",0,{DEGREE},ID[\"EPSG\",8801]],PARAMETER[\"Longitude of natural origin\",0,{DEGREE},ID[\"EPSG\",8802]],PARAMETER[\"False easting\",0,{METRE},ID[\"EPSG\",8806]],PARAMETER[\"False northing\",0,{METRE},ID[\"EPSG\",8807]]"
            ),
        ),
        _ => return None,
    };
    let axes = if s.axis_order == "ne" {
        format!(
            "AXIS[\"northing (X)\",north,ORDER[1],{METRE}],AXIS[\"easting (Y)\",east,ORDER[2],{METRE}]"
        )
    } else {
        format!(
            "AXIS[\"easting (X)\",east,ORDER[1],{METRE}],AXIS[\"northing (Y)\",north,ORDER[2],{METRE}]"
        )
    };
    let conversion = s
        .name
        .rsplit_once(" / ")
        .map_or(s.name.as_str(), |(_, c)| c);
    Some(format!(
        "PROJCRS[\"{}\",BASEGEOGCRS[\"{base}\",{datum_text}],CONVERSION[\"{conversion}\",{method},{params}],CS[Cartesian,2],{axes},{id}]",
        s.name
    ))
}

/// The record of a layer whose objects came from the registry's system `srid`; none for the local system and a code
/// the registry does not know (nothing is guessed).
pub fn source_crs(srid: u32) -> Option<LayerSourceCrs> {
    let s = system(srid)?;
    Some(LayerSourceCrs {
        srid: Some(s.srid),
        name: s.name.clone(),
        wkt: wkt2(s)?,
        axis_order: if s.axis_order.is_empty() {
            "en".to_owned()
        } else {
            s.axis_order.clone()
        },
        unit: s.unit.clone(),
        area: s.area.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_registry_system_but_the_local_one_has_its_record() {
        for s in crate::crs::systems() {
            let r = source_crs(s.srid);
            assert_eq!(r.is_some(), !s.is_local(), "{}", s.name);
            if let Some(r) = r {
                assert_eq!(kentos_contracts::layer_crs_problem(&r), None, "{}", s.name);
                assert!(
                    r.wkt.ends_with(&format!("ID[\"EPSG\",{}]]", s.srid)),
                    "{}",
                    r.wkt
                );
            }
        }
    }
}
