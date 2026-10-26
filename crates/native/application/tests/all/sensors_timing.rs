//! Sensor observations and the time of a question, timed (docs/adr/0646 §8): 1 000 meters' 100 000 readings taken in
//! on an empty sensor layer, 10 000 more of them again with the last policy (each ends the version there), each meter's
//! latest of the 110 000 records; a layer's digest and a pinned historical question of 100 000 parcels; an export's
//! time taking half of them off a copy.
//!
//! ```text
//! cargo test --release -p kentos-native-application --test all sensors_timing -- --ignored --nocapture
//! ```

use std::time::Instant;

use kentos_commands::{
    DocumentSnapshotV1, DuplicatePolicy, LatePolicy, Observation, QueryTime, SensorsIngest,
    SensorsLatest, TimeQuery,
};
use kentos_domain::Document;
use kentos_native_application::{
    ExecutionContext, sensors_ingest, sensors_latest, sources, time_export, time_query,
};
use serde_json::json;

const METERS: usize = 1_000;
const READINGS: usize = 100;
const PARCELS: usize = 100_000;

fn drawing() -> Document {
    let entities: Vec<serde_json::Value> = (0..PARCELS)
        .map(|i| {
            let (x, y) = (500_000.0 + (i % 1000) as f64 * 20.0, 4_400_000.0 + (i / 1000) as f64 * 20.0);
            let year = 2000 + (i % 25);
            json!({"kind": "polygon", "id": i + 1, "layerId": "parsel",
                   "attrs": {"no": format!("{i}"), "baslangic": format!("{year}-01-01"),
                             "bitis": format!("{}-01-01", year + 5)},
                   "pts": [{"x": x, "y": y}, {"x": x + 20.0, "y": y}, {"x": x + 20.0, "y": y + 20.0}, {"x": x, "y": y + 20.0}]})
        })
        .collect();
    let layer = |id: &str, time: serde_json::Value| {
        json!({"id": id, "name": id, "type": "layer", "visible": true, "locked": false, "expanded": true,
               "style": {"color": "ink", "lineType": "continuous", "lineWeight": 0.25}, "children": [], "time": time})
    };
    let snapshot = json!({
        "format": "kentos.document", "version": 1, "name": "Kent",
        "settings": {"srid": 5256, "lengthDecimals": 3, "areaDecimals": 2, "areaUnit": "m2", "angleUnit": "grad",
                     "plotScale": 1000},
        "origin": {"x": 500000, "y": 4400000},
        "layers": [
            layer("parsel", json!({"start": "baslangic", "end": "bitis", "key": "no"})),
            layer("sayac", json!({"start": "an", "key": "sayac", "recordedStart": "alim", "recordedEnd": "alim_bitis"})),
        ],
        "activeLayer": "parsel", "entities": entities, "styles": {"items": [], "categories": []}
    });
    Document::from_snapshot(
        DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("a drawing"),
    )
    .expect("opens")
}

fn readings(from: usize, n: usize, value: f64) -> Vec<Observation> {
    (0..METERS)
        .flat_map(|m| {
            (from..from + n).map(move |k| Observation {
                asset: format!("S-{m:04}"),
                at: format!("2024-06-01T{:02}:{:02}:00Z", k / 60, k % 60),
                value: Some(json!(value + k as f64 / 10.0)),
                quality: None,
            })
        })
        .collect()
}

fn ingest(observations: Vec<Observation>, duplicates: DuplicatePolicy) -> SensorsIngest {
    SensorsIngest {
        layer: "sayac".into(),
        value_field: "debi".into(),
        quality_field: None,
        observations: Some(observations),
        file: None,
        url: None,
        asset: None,
        pages: None,
        received_at: Some("2024-06-01T12:00:00Z".into()),
        duplicates,
        late: LatePolicy::Accept,
        late_field: None,
        expected_revision: None,
    }
}

#[test]
#[ignore = "a measurement, run by hand"]
fn sensors_timing() {
    let mut doc = drawing();
    let t = Instant::now();
    let r = sensors_ingest::execute(
        &mut ExecutionContext::new(&mut doc),
        ingest(readings(0, READINGS, 4.0), DuplicatePolicy::First),
    );
    let first = t.elapsed();
    assert!(matches!(
        r,
        kentos_commands::CommandResult::Completed { .. }
    ));
    let t = Instant::now();
    let r = sensors_ingest::execute(
        &mut ExecutionContext::new(&mut doc),
        ingest(readings(READINGS - 10, 10, 5.0), DuplicatePolicy::Last),
    );
    let again = t.elapsed();
    let kentos_commands::CommandResult::Completed { output, .. } = r else {
        panic!("{r:?}");
    };
    assert_eq!(output.replaced.len(), METERS * 10);
    let t = Instant::now();
    let r = sensors_latest::execute(
        &mut ExecutionContext::new(&mut doc),
        SensorsLatest {
            layer: "sayac".into(),
            value_field: "debi".into(),
            quality_field: None,
            at: Some("2024-06-01T12:00:00Z".into()),
            known_at: None,
            stale_after: Some(600.0),
            expected_assets: Vec::new(),
        },
    );
    let latest = t.elapsed();
    let kentos_commands::CommandResult::Completed { output, .. } = r else {
        panic!("{r:?}");
    };
    assert_eq!(output.assets.len(), METERS);
    let t = Instant::now();
    let stamp = sources::layer_digest(&doc, "parsel");
    let digest = t.elapsed();
    let t = Instant::now();
    let r = time_query::execute(
        &mut ExecutionContext::new(&mut doc),
        TimeQuery {
            layer: "parsel".into(),
            at: Some("2015-06-01".into()),
            from: None,
            to: None,
            known_at: None,
            revision: None,
            expected_stamp: Some(stamp),
        },
    );
    let pinned = t.elapsed();
    assert!(matches!(
        r,
        kentos_commands::CommandResult::Completed { .. }
    ));
    let t = Instant::now();
    let (kept, left) = time_export::filtered(
        &doc,
        &["parsel".into()],
        Some(&QueryTime {
            at: Some("2015-06-01".into()),
            ..QueryTime::default()
        }),
    )
    .expect("filters");
    let export = t.elapsed();
    assert!(left > 0 && kept.len() < doc.len());
    println!(
        "{} okuma alındı {:.0} ms; {} yinelenen sonuncusuyla {:.0} ms; {} varlığın son gözlemi {:.1} ms",
        METERS * READINGS,
        first.as_secs_f64() * 1e3,
        METERS * 10,
        again.as_secs_f64() * 1e3,
        METERS,
        latest.as_secs_f64() * 1e3
    );
    println!(
        "{PARCELS} parselin özeti {:.0} ms; özetle sabitlenmiş soru {:.0} ms; dışa aktarmanın zamanı ({left} nesne dışarıda) {:.0} ms",
        digest.as_secs_f64() * 1e3,
        pinned.as_secs_f64() * 1e3,
        export.as_secs_f64() * 1e3
    );
}
