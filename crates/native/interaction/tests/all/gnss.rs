//! The GNSS import's points (docs/adr/0169 §6) against
//! fixtures/gnss/v1/import.json (scripts/fixtures/gnss_import_cases.py, from
//! the ADR's rules and PROJ): names, elevations and attributes exactly, the
//! positions within the fixture's tolerance. The web checks the same file
//! (`apps/web/src/model/gnssImport.test.ts`).

use kentos_contracts::Entity;
use kentos_contracts::GnssPoint;
use kentos_interaction::gnss::{Options, entities, place};
use serde_json::Value;

#[test]
fn the_imports_points_are_the_references() {
    let file: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/gnss/v1/import.json"))
            .expect("the cases read");
    assert_eq!(file["format"], "kentos.gnss-import");
    let tolerance = file["tolerance"]["metres"].as_f64().expect("a tolerance");
    for case in file["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("a name");
        let srid = u32::try_from(case["srid"].as_u64().expect("an SRID")).expect("an SRID");
        let system = kentos_project::crs::system(srid).expect("a system of the registry");
        let to = system.transform_system().expect("a transform system");
        let points: Vec<GnssPoint> =
            serde_json::from_value(case["points"].clone()).expect("points");
        let o = &case["options"];
        let options = Options {
            kinds: o["kinds"]
                .as_array()
                .expect("kinds")
                .iter()
                .map(|k| k.as_str().expect("a kind").to_owned())
                .collect(),
            prefix: o["prefix"].as_str().expect("a prefix").to_owned(),
            start: u32::try_from(o["start"].as_u64().expect("a start")).expect("a start"),
        };
        let plan = place(&points, &options, &to, &system.name, &[]);
        assert!(plan.skipped.is_empty(), "{name}: {:?}", plan.skipped);
        let want = case["expect"].as_array().expect("points");
        assert_eq!(plan.placed.len(), want.len(), "{name}");
        for (got, w) in plan.placed.iter().zip(want) {
            assert_eq!(got.name, w["name"].as_str().expect("a name"), "{name}");
            let p = w["p"].as_array().expect("a position");
            let (x, y) = (p[0].as_f64().expect("x"), p[1].as_f64().expect("y"));
            assert!(
                (got.p.x - x).abs() <= tolerance && (got.p.y - y).abs() <= tolerance,
                "{name}: {} {:?} ≠ {x} {y}",
                got.name,
                got.p
            );
            assert_eq!(got.z, w["z"].as_f64(), "{name}: {}", got.name);
            let attrs: std::collections::BTreeMap<String, String> =
                serde_json::from_value(w["attrs"].clone()).expect("attributes");
            assert_eq!(got.attrs, attrs, "{name}: {}", got.name);
        }
        // As the import writes them: points with their names as labels and
        // their elevations, on the source layer the window sends on.
        let made = entities(&plan.placed);
        assert_eq!(made.len(), plan.placed.len());
        for (e, p) in made.iter().zip(&plan.placed) {
            let Entity::Point(pt) = e else {
                panic!("{name}: {} is not a point", p.name)
            };
            assert_eq!(pt.base.label.as_deref(), Some(p.name.as_str()));
            assert_eq!((pt.p.x, pt.p.y, pt.z), (p.p.x, p.p.y, p.z));
            assert_eq!(pt.base.attrs, p.attrs);
            assert_eq!(pt.base.layer_id, "");
        }
    }
}

#[test]
fn a_point_the_system_does_not_reach_is_said_with_why() {
    use kentos_geometry_core::crs::{CustomDatum, Datum, Ellipsoid, System};
    // A project's datum without a way to WGS 84 stands alone (docs/adr/0168
    // §2): no GNSS point reaches its system. The web says the same
    // (`gnssImport.test.ts`).
    let alone = System::Tm {
        datum: Datum::Custom(Box::new(CustomDatum {
            name: "Şantiye datumu".to_owned(),
            ellipsoid: Ellipsoid {
                name: "GRS 1980".to_owned(),
                semi_major: 6_378_137.0,
                inverse_flattening: 298.257_222_101,
            },
            to_wgs84: None,
        })),
        latitude_of_origin: None,
        central_meridian: 30.0,
        scale_factor: 1.0,
        false_easting: 500_000.0,
        false_northing: 0.0,
    };
    let point = |kind: &str, name: Option<&str>, lat: f64, lon: f64, line: u32| GnssPoint {
        kind: kind.to_owned(),
        name: name.map(str::to_owned),
        lat,
        lon,
        height: None,
        geoid: None,
        ellipsoidal: None,
        time: None,
        fix: None,
        satellites: None,
        hdop: None,
        line,
    };
    let options = Options {
        kinds: vec!["wpt".to_owned(), "gga".to_owned()],
        prefix: "G".to_owned(),
        start: 1,
    };
    let plan = place(
        &[
            point("wpt", Some("N1"), 40.75, 29.38, 4),
            point("gga", None, 40.76, 29.39, 7),
        ],
        &options,
        &alone,
        "Şantiye",
        &[],
    );
    assert!(plan.placed.is_empty());
    let said: Vec<(u32, &str)> = plan
        .skipped
        .iter()
        .map(|s| (s.line, s.message.as_str()))
        .collect();
    assert_eq!(
        said,
        [
            (
                4,
                "Satır 4: N1 noktası projenin sistemine çevrilemedi (datumlardan birinin WGS 84'e dönüşümü yok); eklenmedi."
            ),
            (
                7,
                "Satır 7: Adsız noktası projenin sistemine çevrilemedi (datumlardan birinin WGS 84'e dönüşümü yok); eklenmedi."
            ),
        ]
    );
}
