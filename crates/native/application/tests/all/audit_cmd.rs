//! Denetle by its meaning over a large drawing (docs/adr/0306 §5): what the plan and the write take for 100 000
//! objects, 10 000 of them linked texts whose objects are gone, 10 000 texts naming a style the project lacks, with 20
//! text styles of which ten are unused. The shared cases are `fixtures/commands/v1/cad.drawing.audit.json`.

use kentos_domain::Document;
use kentos_commands::{AuditPurge, CommandResult, DocumentSnapshotV1, DrawingAudit};
use kentos_native_application::{ExecutionContext, drawing_audit};
use serde_json::json;

const GONE: &str = "0192f5a2-0000-7000-8000-00000000ffff";

fn style(i: u32) -> String {
    format!("0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d{i:04}")
}

fn drawing() -> Document {
    let mut entities = Vec::new();
    for i in 0..100_000u32 {
        let (x, y) = (f64::from(i % 300) * 3.0, f64::from(i / 300) * 3.0);
        let mut e = match i % 10 {
            0 => {
                json!({"kind":"text","p":{"x":x,"y":y},"text":"101","height":1,"rotation":0,"labelOf":GONE,"labelScale":500})
            }
            1 => {
                json!({"kind":"text","p":{"x":x,"y":y},"text":"A","height":1,"rotation":0,"textStyle":style(9999)})
            }
            2 => {
                json!({"kind":"text","p":{"x":x,"y":y},"text":"B","height":1,"rotation":0,"textStyle":style((i / 10) % 10)})
            }
            _ => json!({"kind":"line","a":{"x":x,"y":y},"b":{"x":x + 2.0,"y":y}}),
        };
        e["id"] = json!(i + 1);
        e["layerId"] = json!("a");
        e["attrs"] = json!({});
        entities.push(e);
    }
    let styles: Vec<_> = (0..20)
        .map(|i| json!({"id": style(i), "name": format!("Stil {i}"), "font": "arimo"}))
        .collect();
    let snapshot = json!({"format":"kentos.document","version":1,"name":"Denetle","settings":{"srid":0,
        "lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":100,"textStyles":styles},
        "origin":{"x":0,"y":0},"layers":[{"id":"a","name":"A","type":"layer","visible":true,"locked":false,
        "expanded":true,"style":{"color":"fg","lineType":"continuous","lineWeight":0.18},"children":[]}],
        "activeLayer":"a","entities":entities,"styles":{"items":[],"categories":[]}});
    Document::from_snapshot(DocumentSnapshotV1::from_json(&snapshot.to_string()).expect("reads"))
        .expect("opens")
}

fn all() -> DrawingAudit {
    DrawingAudit {
        fix: true,
        purge: vec![AuditPurge::TextStyles],
        expected_revision: None,
    }
}

/// `cargo test --release -p kentos-native-application --test all audit_cmd::timing -- --ignored --nocapture`.
#[test]
#[ignore = "a timing, run on purpose"]
fn timing() {
    use std::time::Instant;
    let mut doc = drawing();
    let n = doc.len();
    let t = Instant::now();
    let CommandResult::Completed { output: plan, .. } = drawing_audit::plan_in(&doc, &all()) else {
        panic!("not completed")
    };
    let planned = t.elapsed().as_secs_f64() * 1e3;
    let t = Instant::now();
    let CommandResult::Completed { output, .. } =
        drawing_audit::execute(&mut ExecutionContext::new(&mut doc), all())
    else {
        panic!("not completed")
    };
    let written = t.elapsed().as_secs_f64() * 1e3;
    assert_eq!(output.findings.len(), plan.findings.len());
    assert_eq!(output.changed.len(), 20_000);
    let purged = output.unused.iter().filter(|u| u.purged).count();
    assert_eq!(purged, 10);
    println!(
        "{n} nesne: plan {planned:.0} ms, yazma {written:.0} ms; {} bulgu, {} onarılan, {purged} stil",
        output.findings.len(),
        output.changed.len()
    );
}
