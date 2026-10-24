//! The GNSS import's points and lines (docs/adr/0169 §6, 0222 §6) against
//! fixtures/gnss/v1/import.json (scripts/fixtures/gnss_import_cases.py, from
//! the ADR's rules and PROJ): names, elevations and attributes exactly, the
//! positions within the fixture's tolerance. The web checks the same file
//! (`apps/web/src/model/gnssImport.test.ts`).

use kentos_commands::Entity;
use kentos_commands::{GnssLine, GnssPoint};
use kentos_interaction::gnss::{Options, entities, line_entities, place, place_all};
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
            lines: o["lines"].as_bool().expect("lines"),
        };
        let lines: Vec<GnssLine> = serde_json::from_value(case["lines"].clone()).expect("lines");
        let plan = place_all(&points, &lines, &options, &to, &system.name, &[]);
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
        // The routes and tracks (docs/adr/0222 §6).
        let want = &case["expectLines"];
        let said: Vec<(u64, String)> = want["skipped"]
            .as_array()
            .expect("skipped")
            .iter()
            .map(|s| {
                (
                    s["line"].as_u64().expect("a line"),
                    s["message"].as_str().expect("a text").to_owned(),
                )
            })
            .collect();
        let got_said: Vec<(u64, String)> = plan
            .lines_skipped
            .iter()
            .map(|s| (u64::from(s.line), s.message.clone()))
            .collect();
        assert_eq!(got_said, said, "{name}");
        let want = want["placed"].as_array().expect("lines");
        assert_eq!(plan.lines.len(), want.len(), "{name}");
        for (got, w) in plan.lines.iter().zip(want) {
            assert_eq!(
                u64::from(got.line),
                w["line"].as_u64().expect("a line"),
                "{name}"
            );
            assert_eq!(got.name.as_deref(), w["name"].as_str(), "{name}");
            let attrs: std::collections::BTreeMap<String, String> =
                serde_json::from_value(w["attrs"].clone()).expect("attributes");
            assert_eq!(got.attrs, attrs, "{name}");
            let parts = w["parts"].as_array().expect("parts");
            assert_eq!(got.parts.len(), parts.len(), "{name}");
            for (run, part) in got.parts.iter().zip(parts) {
                let part = part.as_array().expect("a part");
                assert_eq!(run.len(), part.len(), "{name}");
                for ((p, z), v) in run.iter().zip(part) {
                    let q = v["p"].as_array().expect("a position");
                    let (x, y) = (q[0].as_f64().expect("x"), q[1].as_f64().expect("y"));
                    assert!(
                        (p.x - x).abs() <= tolerance && (p.y - y).abs() <= tolerance,
                        "{name}"
                    );
                    assert_eq!(*z, v["z"].as_f64(), "{name}");
                }
            }
        }
        let made = line_entities(&plan.lines);
        assert_eq!(made.len(), plan.lines.len());
        for (e, l) in made.iter().zip(&plan.lines) {
            let Entity::Polyline(p) = e else {
                panic!("{name}: a line is not a polyline")
            };
            assert_eq!(p.base.label, l.name);
            assert_eq!(p.pts.len(), l.parts[0].len());
            assert_eq!(p.parts.as_ref().map_or(0, Vec::len), l.parts.len() - 1);
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
        lines: true,
    };
    let points = [
        point("wpt", Some("N1"), 40.75, 29.38, 4),
        point("gga", None, 40.76, 29.39, 7),
    ];
    let track = GnssLine {
        kind: "nmea".to_owned(),
        name: None,
        parts: vec![vec![0, 1]],
        line: 4,
    };
    let plan = place(&points, &options, &alone, "Şantiye", &[]);
    let all = place_all(&points, &[track], &options, &alone, "Şantiye", &[]);
    assert!(all.lines.is_empty());
    assert_eq!(
        all.lines_skipped
            .iter()
            .map(|s| s.message.as_str())
            .collect::<Vec<_>>(),
        [
            "Satır 4: NMEA kaydı çizgisi projenin sistemine çevrilemedi (datumlardan birinin WGS 84'e dönüşümü yok); eklenmedi."
        ]
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

/// What the elevations become as the project says its heights are (docs/adr/0519 §6): not said, the WGS 84
/// ellipsoidal height; ellipsoidal, the height moved with the datum's shift (to ED50 here: PROJ's ED50 to WGS 84
/// (30) reversed in 3D, WGS 84 to ED50 / TM33 promoted to 3D, gives 1000.0000193491578 m); orthometric, none, said.
#[test]
fn the_elevations_follow_what_the_project_says_its_heights_are() {
    use kentos_commands::HeightKind;
    use kentos_interaction::gnss::place_all_in;
    let point = GnssPoint {
        kind: "wpt".into(),
        name: Some("A".into()),
        lat: 39.932_422_407_409_625,
        lon: 32.859_363_516_103_47,
        height: None,
        geoid: None,
        ellipsoidal: Some(1_040.340_680_593_624_7),
        time: None,
        fix: None,
        satellites: None,
        hdop: None,
        line: 1,
    };
    let system = kentos_project::crs::system(2321).expect("ED50 / TM33");
    let to = system.transform_system().expect("a transform system");
    let options = Options {
        kinds: vec!["wpt".into()],
        prefix: "G".into(),
        start: 1,
        lines: false,
    };
    // A geoid of 30 m everywhere over Ankara (docs/adr/0631 §5).
    let geoid = kentos_geometry_core::crs::geoid::NodeGrid::new(
        32.0,
        39.0,
        1.0,
        1.0,
        2,
        2,
        1,
        vec![30.0; 4],
        None,
    )
    .expect("a grid");
    let z_with = |heights, geoid| {
        let plan = place_all_in(
            std::slice::from_ref(&point),
            &[],
            &options,
            &to,
            &system.name,
            &[],
            heights,
            geoid,
        );
        (plan.placed[0].z, plan.heights)
    };
    let z = |heights| z_with(heights, None);
    assert_eq!(z(None), (Some(1_040.340_680_593_624_7), None));
    let (moved, note) = z(Some(HeightKind::Ellipsoidal));
    assert!(
        (moved.expect("a height") - 1_000.000_019_349_157_8).abs() < 1e-6,
        "{moved:?}"
    );
    assert!(note.expect("said").contains("datumuna kaydırılarak"));
    let (none, note) = z(Some(HeightKind::Orthometric));
    assert_eq!(none, None);
    assert!(note.expect("said").contains("Izgaralar kitaplığında yok"));
    // With the project's geoid here: H = h − N, the height first moved to ED50's ellipsoid.
    let (ortho, note) = z_with(Some(HeightKind::Orthometric), Some(("TG-20", &geoid)));
    assert!(
        (ortho.expect("a height") - 970.000_019_349_157_8).abs() < 1e-6,
        "{ortho:?}"
    );
    assert!(note.expect("said").contains("TG-20 geoit modeliyle"));
}
