//! Konumlu fotoğraflar' points (docs/adr/0225 §3) against
//! fixtures/photos/v1/import.json (scripts/fixtures/photo_import_cases.py,
//! from the ADR's rules and PROJ): names, attributes, attachments and the
//! photos left out exactly, the positions within the fixture's tolerance.
//! The web checks the same file (`apps/web/src/model/photoImport.test.ts`).

use kentos_commands::{Entity, PhotoExif};
use kentos_interaction::photos::{Photo, entities, place};
use serde_json::Value;

#[test]
fn the_imports_points_are_the_references() {
    let file: Value = serde_json::from_str(include_str!(
        "../../../../../fixtures/photos/v1/import.json"
    ))
    .expect("the cases read");
    assert_eq!(file["format"], "kentos.photo-import");
    let tolerance = file["tolerance"]["metres"].as_f64().expect("a tolerance");
    for case in file["cases"].as_array().expect("cases") {
        let name = case["name"].as_str().expect("a name");
        let srid = u32::try_from(case["srid"].as_u64().expect("an SRID")).expect("an SRID");
        let system = kentos_project::crs::system(srid).expect("a system of the registry");
        let to = system.transform_system().expect("a transform system");
        let photos: Vec<Photo> = case["photos"]
            .as_array()
            .expect("photos")
            .iter()
            .map(|p| Photo {
                name: p["name"].as_str().expect("a name").to_owned(),
                exif: serde_json::from_value::<PhotoExif>(p["exif"].clone()).expect("EXIF"),
                asset: p["asset"].as_str().map(str::to_owned),
                file: p["file"].as_str().map(str::to_owned),
            })
            .collect();
        let plan = place(&photos, &to, &system.name, &[]);
        let skipped: Vec<(u64, String)> = plan
            .skipped
            .iter()
            .map(|s| (u64::from(s.line), s.message.clone()))
            .collect();
        let want_skipped: Vec<(u64, String)> = case["expect"]["skipped"]
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
        assert_eq!(skipped, want_skipped, "{name}");
        let want = case["expect"]["placed"].as_array().expect("placed");
        assert_eq!(plan.placed.len(), want.len(), "{name}");
        for (got, w) in plan.placed.iter().zip(want) {
            assert_eq!(u64::from(got.index), w["index"].as_u64().expect("an index"));
            assert_eq!(got.name, w["name"].as_str().expect("a name"), "{name}");
            let p = w["p"].as_array().expect("a position");
            let (x, y) = (p[0].as_f64().expect("x"), p[1].as_f64().expect("y"));
            assert!(
                (got.p.x - x).abs() <= tolerance && (got.p.y - y).abs() <= tolerance,
                "{name}: {} {:?} ≠ {x} {y}",
                got.name,
                got.p
            );
            let attrs: std::collections::BTreeMap<String, String> =
                serde_json::from_value(w["attrs"].clone()).expect("attributes");
            assert_eq!(got.attrs, attrs, "{name}: {}", got.name);
            assert_eq!(
                serde_json::to_value(&got.attachment).expect("JSON"),
                w["attachment"],
                "{name}: {}",
                got.name
            );
        }
        for (e, p) in entities(&plan.placed).iter().zip(&plan.placed) {
            let Entity::Point(pt) = e else {
                panic!("{name}: {} is not a point", p.name)
            };
            assert_eq!(pt.base.label.as_deref(), Some(p.name.as_str()));
            assert_eq!(pt.base.attachments, std::slice::from_ref(&p.attachment));
            assert_eq!((pt.base.layer_id.as_str(), pt.z), ("", None));
        }
    }
}
