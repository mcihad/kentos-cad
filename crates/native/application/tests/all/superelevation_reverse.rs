//! An alignment turned round carries its superelevation schedules (plan/road.md Y58, docs/adr/0964; ROAD-DEVER-03):
//! the same physical road read from the other end. Checked with the core's own schedules: every edge has the same
//! slope and the same widening at the same place before and after, the old left at s being the new right at L − s.

use kentos_commands::{AlignmentsEdit, AlignmentsEditOperation, CommandResult};
use kentos_native_application::{ExecutionContext, road_alignments_edit, road_superelevations};

/// A breakpoint list read at `s` (linear between, flat past the ends); `normal` where it is empty.
fn along(points: &[[f64; 2]], s: f64, normal: f64) -> f64 {
    match points {
        [] => normal,
        [only] => only[1],
        _ => {
            if s <= points[0][0] {
                return points[0][1];
            }
            for w in points.windows(2) {
                if s <= w[1][0] {
                    let t = (s - w[0][0]) / (w[1][0] - w[0][0]);
                    return w[0][1] + t * (w[1][1] - w[0][1]);
                }
            }
            points[points.len() - 1][1]
        }
    }
}

#[test]
fn a_reversed_alignment_keeps_every_edge_s_slope() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/road.alignments.edit.json")).expect("cases");
    let case = cases["cases"].as_array().expect("cases").iter().find(|c| c["name"] == "Ters çevrilir: dever çizelgeleri de döner").expect("the case");
    let snapshot: kentos_commands::DocumentSnapshotV2 = serde_json::from_value(case["setup"].clone()).expect("setup");
    let mut doc = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    let before = road_superelevations::made_superelevations(&doc);
    let length = kentos_native_application::road_alignments::made_alignments(&doc)
        .into_iter()
        .find(|(d, _)| d.id == "g-1")
        .and_then(|(_, b)| b.ok())
        .map(|b| b.core.length)
        .expect("the alignment builds");
    let input = AlignmentsEdit {
        alignment: "g-1".into(),
        operation: AlignmentsEditOperation::Reverse,
        name: None,
        pis: None,
        stationing: None,
        offset: None,
        expected_revision: None,
    };
    let mut cx = ExecutionContext::new(&mut doc);
    let CommandResult::Completed { output, .. } = road_alignments_edit::execute(&mut cx, input) else { panic!("reversed") };
    assert_eq!(output.superelevations, ["d-1", "d-2"]);
    let after = road_superelevations::made_superelevations(&doc);
    for ((d0, r0), (d1, r1)) in before.iter().zip(&after) {
        if d0.alignment != "g-1" {
            assert_eq!(d0, d1, "a schedule on another alignment is left alone");
            continue;
        }
        assert!(!r1.stale, "{} is bound to the new revision", d1.id);
        assert!(r0.problem.is_none() && r1.problem.is_none(), "{:?} {:?}", r0.problem, r1.problem);
        let mut worst: f64 = 0.0;
        for k in 0..=1000 {
            let s = length * f64::from(k) / 1000.0;
            let pairs = [
                (along(&r0.left, s, r0.normal_left), along(&r1.right, length - s, r1.normal_right)),
                (along(&r0.right, s, r0.normal_right), along(&r1.left, length - s, r1.normal_left)),
                (along(&r0.left_widening, s, 0.0), along(&r1.right_widening, length - s, 0.0)),
                (along(&r0.right_widening, s, 0.0), along(&r1.left_widening, length - s, 0.0)),
            ];
            for (a, b) in pairs {
                worst = worst.max((a - b).abs());
            }
        }
        assert!(worst < 1e-9, "{}: an edge's slope moved by {worst}", d0.id);
    }
}
