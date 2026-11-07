//! The project's coordinate facts in words (plan/map-cad.md MAP-18, docs/adr/1680): the datum, the ellipsoid, the
//! projection with its central meridian and scale factor, what the heights are, the coordinates' epoch, the second
//! system and the EPSG code. A sheet's texts read them from here (`@datum`, `@elipsoit`, `@projeksiyon`,
//! `@kot_sistemi`, `@epok`, `@ikinci_sistem`, `@epsg_kodu`), so a title block says what the project is and never what
//! someone typed once (the plan: “datum/epok ve kalite notları aynı proje bilgisinden gelsin”). The project's
//! expressions (`crate::variables`) keep the built-ins they share with the web's reference; these are the sheets'.
//!
//! A registry system gives every one; a project's own definition gives its name (`@koordinat_sistemi`) only, the rest
//! none (not guessed). A project without a system has none.

use kentos_contracts::{HeightKind, ProjectSettings};

use crate::crs::{LOCAL_SRID, datum_label, system};
use crate::systems;

/// The facts' variable names and what each holds, in the order the builder lists them. Not reserved: a project's or a
/// sheet's own variable of a name comes first, so a drawing that has one stays as it was.
pub const FACT_VARIABLES: [(&str, &str); 7] = [
    ("datum", "Koordinat sisteminin datumu (kayıtlı sistemlerde)"),
    ("elipsoit", "Datumun elipsoidi (kayıtlı sistemlerde)"),
    ("projeksiyon", "Projeksiyon, dilim orta meridyeni ve ölçek katsayısı (kayıtlı sistemlerde)"),
    ("kot_sistemi", "Kotların türü, düşey sistemi ve geoit modeli"),
    ("epok", "Koordinatların epoku (ondalık yıl)"),
    ("ikinci_sistem", "Projenin ikinci koordinat sisteminin adı"),
    ("epsg_kodu", "Koordinat sisteminin EPSG kodu yazıyla (EPSG:5256)"),
];

/// The facts; an empty text or none where the project does not say it.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct CoordinateFacts {
    pub datum: String,
    pub ellipsoid: String,
    pub projection: String,
    pub heights: String,
    pub epoch: Option<f64>,
    pub second: String,
    /// “EPSG:5256” for a registry system.
    pub epsg: String,
}

/// A number as a sheet writes it: no trailing zeros, a decimal comma (36 → “36”, 0.9996 → “0,9996”).
fn plain(v: f64) -> String {
    let t = format!("{v:.10}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    t.replace('.', ",")
}

/// The project's coordinate facts.
pub fn coordinate_facts(settings: &ProjectSettings) -> CoordinateFacts {
    let mut out = CoordinateFacts::default();
    if let Some(s) = (settings.srid != LOCAL_SRID).then(|| system(settings.srid)).flatten() {
        out.datum = datum_label(&s.datum).to_owned();
        out.ellipsoid = s.ellipsoid.clone().unwrap_or_default();
        out.epsg = format!("EPSG:{}", s.srid);
        if let Some(p) = &s.projection {
            let mut words = vec![p.clone()];
            if let Some(m) = s.central_meridian {
                words.push(format!("DOM {}°", plain(m)));
            }
            if let Some(k) = s.scale_factor {
                words.push(format!("ölçek katsayısı {}", plain(k)));
            }
            out.projection = words.join(", ");
        }
    }
    if let Some(h) = &settings.heights {
        let kind = match h.kind {
            HeightKind::Ellipsoidal => Some("Elipsoit yüksekliği"),
            HeightKind::Orthometric => Some("Ortometrik yükseklik"),
            HeightKind::Unknown => None,
        };
        if let Some(kind) = kind {
            let mut more = Vec::new();
            if let Some(v) = h.vertical() {
                more.push(v.name.to_owned());
            }
            if let Some(g) = &h.geoid {
                more.push(format!("geoit {}", g.name));
            }
            out.heights = if more.is_empty() { kind.to_owned() } else { format!("{kind} ({})", more.join("; ")) };
        }
    }
    out.epoch = settings.coordinate_epoch.filter(|_| settings.has_system());
    out.second = systems::second(settings).map(|n| n.name).unwrap_or_default();
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_contracts::{GeoidModel, HeightReference};

    fn settings(srid: u32) -> ProjectSettings {
        serde_json::from_value(serde_json::json!({
            "srid": srid, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "deg",
            "plotScale": 1000.0
        }))
        .expect("settings")
    }

    /// TUREF / TM36 (EPSG:5256) by EPSG's own record: the TUREF datum (ITRF96), GRS 1980, Transverse Mercator with
    /// its natural origin at 36° E and a scale factor of 1.
    #[test]
    fn turef_tm36_by_its_epsg_record() {
        let f = coordinate_facts(&settings(5256));
        assert_eq!(f.datum, "TUREF (ITRF96)");
        // EPSG's ellipsoid 7019, “GRS 1980”, as the registry names it.
        assert_eq!(f.ellipsoid, "GRS80");
        assert_eq!(f.projection, "Transverse Mercator, DOM 36°, ölçek katsayısı 1");
        assert_eq!(f.epsg, "EPSG:5256");
        assert_eq!((f.heights.as_str(), f.epoch, f.second.as_str()), ("", None, ""));
    }

    /// ED50 / UTM zone 36N (EPSG:23036): central meridian 33° E, scale factor 0.9996, International 1924.
    #[test]
    fn ed50_utm_36_by_its_epsg_record() {
        let f = coordinate_facts(&settings(23036));
        assert_eq!(f.datum, "ED50");
        assert!(f.ellipsoid.contains("1924") || f.ellipsoid.contains("International"), "{}", f.ellipsoid);
        assert!(f.projection.ends_with("DOM 33°, ölçek katsayısı 0,9996"), "{}", f.projection);
    }

    #[test]
    fn heights_epoch_and_the_second_system() {
        let mut s = settings(5256);
        s.heights = Some(HeightReference {
            kind: HeightKind::Orthometric,
            vertical_srid: None,
            geoid: Some(GeoidModel { name: "TG-20".into(), sha256: None }),
        });
        s.coordinate_epoch = Some(2005.0);
        s.second_srid = Some(23036);
        let f = coordinate_facts(&s);
        assert_eq!(f.heights, "Ortometrik yükseklik (geoit TG-20)");
        assert_eq!(f.epoch, Some(2005.0));
        assert!(f.second.contains("ED50"), "{}", f.second);
    }

    #[test]
    fn a_local_project_says_none() {
        assert_eq!(coordinate_facts(&settings(LOCAL_SRID)), CoordinateFacts::default());
    }
}
