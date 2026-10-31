//! docs/adr/0772's timings in release: a type with 1 000 doors changed (its plan, the preview, and its write), the
//! openings queried, and the door checks over them with a column at every tenth door.
//!
//! ```text
//! cargo test --release -p kentos-native-application --test all opening_types_timing -- --ignored --nocapture
//! ```

use std::time::Instant;

use kentos_commands::*;
use kentos_domain::Document;
use kentos_native_application::{
    ExecutionContext, arch_audit, columns_create, opening_types_edit, openings_create,
    openings_query, walls_create,
};
use serde_json::{Value, json};

const TYPES: &str = include_str!("../../../../../fixtures/commands/v1/cad.openingtypes.edit.json");

fn done<T: std::fmt::Debug>(r: CommandResult<T>) -> T {
    match r {
        CommandResult::Completed { output, .. } => output,
        other => panic!("{other:?}"),
    }
}

fn drawing(doors: usize) -> Document {
    let file: Value = serde_json::from_str(TYPES).unwrap();
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(file["setup"].clone()).unwrap();
    let mut doc = Document::from_snapshot_v2(snapshot).unwrap();
    let length = 2.5 * doors as f64 + 5.0;
    let made = done(walls_create::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({"layerId":"duvar","walls":[
            {"pts":[{"x":0,"y":-50},{"x":length,"y":-50}],"width":0.2}]}))
        .unwrap(),
    ));
    let host = made.uids[0].clone();
    let openings: Vec<Value> = (0..doors)
        .map(|k| json!({"host": host, "at": 2.5 + 2.5 * k as f64, "type": "k01"}))
        .collect();
    done(openings_create::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({"layerId":"kapi","openings":openings})).unwrap(),
    ));
    let columns: Vec<Value> = (0..doors / 10)
        .map(|k| json!({"at":{"x": 2.5 + 25.0 * k as f64 + 0.3, "y": -49.3}, "width": 0.4}))
        .collect();
    done(columns_create::execute(
        &mut ExecutionContext::new(&mut doc),
        serde_json::from_value(json!({"layerId":"duvar","columns":columns})).unwrap(),
    ));
    doc
}

fn best<T>(n: usize, mut f: impl FnMut() -> T) -> f64 {
    (0..n)
        .map(|_| {
            let t = Instant::now();
            std::hint::black_box(f());
            t.elapsed().as_secs_f64() * 1000.0
        })
        .fold(f64::MAX, f64::min)
}

#[test]
#[ignore]
fn opening_types_timing() {
    let doc = drawing(1000);
    let update: OpeningTypesEdit = serde_json::from_value(json!({
        "operation": {"kind":"update","type":"K01","change":{"frame":0.06,"build":{"casing":0.07}}}
    }))
    .unwrap();
    let plan = best(5, || opening_types_edit::plan_in(&doc, &update));
    let followers = match opening_types_edit::plan_in(&doc, &update) {
        CommandResult::Completed { output, .. } => output.report.applied.len(),
        other => panic!("{other:?}"),
    };
    let write = best(3, || {
        let mut copy = doc.clone();
        opening_types_edit::execute(&mut ExecutionContext::new(&mut copy), update.clone())
    });
    let query = best(5, || {
        openings_query::query(
            &doc,
            &OpeningsQuery {
                kind: Some("k01".into()),
                ..Default::default()
            },
        )
    });
    let rows = match openings_query::query(
        &doc,
        &OpeningsQuery {
            kind: Some("k01".into()),
            ..Default::default()
        },
    ) {
        CommandResult::Completed { output, .. } => output.openings.len(),
        other => panic!("{other:?}"),
    };
    assert_eq!(rows, 1020);
    let audit: ArchAudit = serde_json::from_value(json!({"checks":["doors"]})).unwrap();
    let mut copy = doc.clone();
    let doors = best(3, || {
        arch_audit::plan(&ExecutionContext::new(&mut copy), &audit)
    });
    println!(
        "1 000 kapı ({followers} izleyen): plan {plan:.1} ms, yazma (kopya dahil) {write:.1} ms, sorgu {query:.1} ms, Kapılar denetimi {doors:.1} ms"
    );
    assert!(followers >= 1000);
    assert!(plan < 50.0 && write < 100.0 && query < 20.0 && doors < 100.0);
}
