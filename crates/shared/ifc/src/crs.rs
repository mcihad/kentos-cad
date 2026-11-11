//! The project's coordinate reference system as Mimarlık ve Mühendislik Projelerinin Dijital Olarak Hazırlanması
//! Hakkında Yönetmelik EK-7 asks it (Tablo 7.6 `IfcProjectedCRS`, Tablo 7.7 `IfcMapConversion`; docs/adr/8047 §8): the
//! system and the conversion of the model's coordinates into it. TUREF's 3° Transverse Mercator zones (EPSG:5253 …
//! 5259, their central meridians 27° … 45°; ITRF96's realisation in Türkiye on the GRS80 ellipsoid) are named in full,
//! another system by its name. Every IFC file KentOS writes (the building's, the road's alignments) says its system
//! through here.

use crate::step::{Step, real, text};

/// A projected system and where the model's origin stands in it.
#[derive(Clone, Debug, PartialEq)]
pub struct Crs {
    /// Its name: an EPSG code ("EPSG:5254"), or a project's own system's name.
    pub name: String,
    pub description: Option<String>,
    /// The datum its plane coordinates stand on (TUREF).
    pub geodetic_datum: Option<String>,
    /// The datum its heights are measured from ("Antalya height (EPSG:5775)", "ITRF96").
    pub vertical_datum: Option<String>,
    pub map_projection: Option<String>,
    /// A TUREF zone's central meridian, degrees ("30").
    pub map_zone: Option<String>,
    /// The model's origin in it: eastings, northings and orthogonal height, metres.
    pub origin: [f64; 3],
}

/// The central meridian of TUREF's 3° zone an EPSG code names, degrees: EPSG:5253 27 … EPSG:5259 45 (EK-7 Tablo 7.6).
pub fn turef_meridian(code: &str) -> Option<u32> {
    let n: u32 = code.strip_prefix("EPSG:")?.parse().ok()?;
    (5253..=5259).contains(&n).then(|| 27 + 3 * (n - 5253))
}

/// The system `name` ("EPSG:5254", or a project's own) with its description, the datum its heights are measured from
/// and the model's origin in it: a TUREF zone with its datum, its projection and its zone (EK-7 Tablo 7.6), another
/// system by its name.
pub fn projected(
    name: &str,
    description: Option<String>,
    vertical_datum: Option<String>,
    origin: [f64; 3],
) -> Crs {
    let meridian = turef_meridian(name);
    Crs {
        name: name.to_owned(),
        description,
        geodetic_datum: meridian.map(|_| "TUREF".to_owned()),
        vertical_datum,
        map_projection: meridian.map(|_| "Transverse Mercator".to_owned()),
        map_zone: meridian.map(|m| m.to_string()),
        origin,
    }
}

/// Writes `crs`: its `IfcProjectedCRS` (its unit the file's metre, `length`) and the `IfcMapConversion` of the
/// representation context `context` into it, with no turn and no scale.
pub(crate) fn write(st: &mut Step, crs: &Crs, context: usize, length: usize) {
    let opt = |s: &Option<String>| s.as_deref().map_or_else(|| "$".to_owned(), text);
    let c = st.add(format!(
        "IFCPROJECTEDCRS({},{},{},{},{},{},#{length})",
        text(&crs.name),
        opt(&crs.description),
        opt(&crs.geodetic_datum),
        opt(&crs.vertical_datum),
        opt(&crs.map_projection),
        opt(&crs.map_zone)
    ));
    st.add(format!(
        "IFCMAPCONVERSION(#{context},#{c},{},{},{},1.,0.,1.)",
        real(crs.origin[0]),
        real(crs.origin[1]),
        real(crs.origin[2])
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turef_s_zones_are_named_in_full_and_another_system_by_its_name() {
        assert_eq!(turef_meridian("EPSG:5253"), Some(27));
        assert_eq!(turef_meridian("EPSG:5254"), Some(30));
        assert_eq!(turef_meridian("EPSG:5259"), Some(45));
        assert_eq!(turef_meridian("EPSG:5260"), None);
        assert_eq!(turef_meridian("EPSG:32636"), None);
        let tm30 = projected(
            "EPSG:5254",
            Some("TUREF / TM30".into()),
            None,
            [0.0, 0.0, 1285.4],
        );
        assert_eq!(
            (
                tm30.geodetic_datum.as_deref(),
                tm30.map_projection.as_deref(),
                tm30.map_zone.as_deref()
            ),
            (Some("TUREF"), Some("Transverse Mercator"), Some("30"))
        );
        let utm = projected("EPSG:32636", None, None, [0.0; 3]);
        assert_eq!(
            (utm.geodetic_datum, utm.map_projection, utm.map_zone),
            (None, None, None)
        );
    }

    #[test]
    fn it_is_written_as_a_projected_system_and_its_conversion() {
        let mut st = Step::new("t".into());
        let length = st.add("IFCSIUNIT(*,.LENGTHUNIT.,$,.METRE.)");
        let model = st.add("IFCGEOMETRICREPRESENTATIONCONTEXT($,'Model',3,1.E-05,$,$)");
        write(
            &mut st,
            &projected(
                "EPSG:5255",
                Some("TUREF / TM33".into()),
                Some("Antalya height (EPSG:5775)".into()),
                [0.0, 0.0, 1285.4],
            ),
            model,
            length,
        );
        let text = st.file("v", "t.ifc", "2026-11-11T12:00:00", "KentOS");
        assert!(text.contains(
            "IFCPROJECTEDCRS('EPSG:5255','TUREF / TM33','TUREF','Antalya height (EPSG:5775)','Transverse Mercator','33',#1)"
        ));
        assert!(text.contains("IFCMAPCONVERSION(#2,#3,0.,0.,1285.4,1.,0.,1.)"));
    }
}
