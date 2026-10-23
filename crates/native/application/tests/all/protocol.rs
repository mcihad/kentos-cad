//! The command protocol (docs/adr/0253): `call::call` over the native
//! document. What the shared cases of each command cannot show: the summary
//! and its stamps, the plan's id and the write that must equal it, the
//! expectations, a request sent again, the errors' objects, the journal, and
//! a command run as a job.

use kentos_commands::{
    CallAnswer, CallEnvelope, CallExpectations, CallKind, CallOp, CallStatus, CommandResult,
    DocumentSnapshotV1, DocumentSnapshotV2, JobPhase,
};
use kentos_domain::{Document, Slot, Uuid};
use kentos_native_application::call::call;
use kentos_native_application::jobs::Job;
use serde_json::{Value, json};

const CASES: &str = include_str!("../../../../../fixtures/commands/v1/cad.changeset.apply.json");
const LINKED: &str = include_str!("../../../../../fixtures/kcad/v2/linked-texts.json");

/// The change set's drawing: line 1 and point 2 on “yapi”, a locked and a hidden layer.
fn drawing() -> Document {
    let fixture: Value = serde_json::from_str(CASES).expect("the cases read");
    let snapshot =
        DocumentSnapshotV1::from_json(&fixture["setup"].to_string()).expect("the drawing reads");
    Document::from_snapshot(snapshot).expect("opens")
}

fn uid(doc: &Document, slot: u32) -> String {
    doc.uid(Slot(slot)).expect("an object").to_string()
}

fn envelope(command: &str, op: CallOp, input: Value) -> CallEnvelope {
    CallEnvelope {
        relax_constraints: false,
        command: command.into(),
        version: None,
        op,
        input,
        request_id: None,
        expected: None,
    }
}

fn run(doc: &mut Document, e: CallEnvelope) -> CallAnswer {
    call(doc, None, e)
}

fn output(answer: &CallAnswer) -> &Value {
    match &answer.result {
        CommandResult::Completed { output, .. } => output,
        other => panic!("not completed: {other:?}"),
    }
}

fn code(answer: &CallAnswer) -> (&'static str, &str) {
    match &answer.result {
        CommandResult::Failed { error } => ("failed", error.code.as_str()),
        CommandResult::Conflict { error } => ("conflict", error.code.as_str()),
        other => panic!("not refused: {other:?}"),
    }
}

fn point(x: f64, y: f64) -> Value {
    json!({ "layerId": "yapi", "p": { "x": x, "y": y } })
}

#[test]
fn a_paper_only_changeset_reports_an_undoable_book_and_actual_revision() {
    use kentos_commands::ProjectPart;
    let mut doc = Document::from_snapshot_v2(
        serde_json::from_str::<DocumentSnapshotV2>(include_str!(
            "../../../../../fixtures/kcad/v2/sheets.json"
        ))
        .unwrap(),
    )
    .unwrap();
    let before = doc.to_snapshot_v2();
    let input = json!({"label":"Pafta işlemi", "steps":[{
        "command":"cad.sheets.edit", "input":{"ops":[{
            "op":"renameSheet", "id":"pafta-1", "name":"Yeni ad"
        }]}
    }]});
    let plan = run(
        &mut doc,
        envelope("cad.changeset.apply", CallOp::Plan, input.clone()),
    );
    assert!(matches!(plan.result, CommandResult::Completed { .. }));
    assert_eq!(
        doc.to_snapshot_v2(),
        before,
        "planning leaves book and pictures untouched"
    );
    let answer = run(
        &mut doc,
        envelope("cad.changeset.apply", CallOp::Execute, input),
    );
    assert_eq!(output(&answer)["revision"], doc.revision().to_string());
    assert_eq!(
        output(&answer)["steps"][0]["output"]["revision"],
        doc.revision().to_string()
    );
    let summary = answer.summary.unwrap();
    assert_eq!(summary.project, vec![ProjectPart::Sheets]);
    assert_eq!(summary.undo.label.as_deref(), Some("Pafta işlemi"));
    assert!(summary.undo.outside.is_empty());
    assert_ne!(summary.sources, summary.stamps);
    let after = doc.sheets().unwrap().clone();
    assert_eq!(after.pictures, before.sheets.as_ref().unwrap().pictures);
    doc.undo();
    assert_eq!(doc.sheets(), before.sheets.as_ref());
    doc.redo();
    assert_eq!(doc.sheets(), Some(&after));
}

fn shift(u: &str, dx: f64) -> Value {
    json!({ "uids": [u], "transform": { "kind": "move", "dx": dx, "dy": 0.0 } })
}

#[test]
fn a_write_answers_with_what_it_touched_and_the_stamps_before_and_after() {
    let mut doc = drawing();
    let line = uid(&doc, 1);
    let before = doc.stamp(Slot(1)).expect("a stamp");
    let base = doc.revision();
    let answer = run(
        &mut doc,
        envelope("cad.entities.transform", CallOp::Execute, shift(&line, 2.0)),
    );
    output(&answer);
    let s = answer.summary.expect("a summary");
    assert_eq!(s.changed, vec![line.clone()]);
    assert!(s.added.is_empty() && s.removed.is_empty() && s.dependents.is_empty());
    assert_eq!((s.counts.changed, s.counts.added), (1, 0));
    assert_eq!(s.sources.objects[&line], before);
    assert_eq!(
        s.stamps.objects[&line],
        doc.stamp(Slot(1)).expect("a stamp")
    );
    assert_ne!(s.sources.objects[&line], s.stamps.objects[&line]);
    assert_eq!(s.sources.revision, Some(base.to_string()));
    assert_eq!(s.stamps.revision, Some(doc.revision().to_string()));
    let b = s.bounds.expect("a box");
    // The line from 0 to 10 moved by 2: the box holds both.
    assert_eq!((b.min_x, b.max_x), (487000.0, 487012.0));
    assert!(s.undo.label.is_some());
    assert!(s.undo.outside.is_empty());
    assert_eq!(s.algorithm.policy, "kentos.numeric/1");
    assert_eq!(s.algorithm.command, "cad.entities.transform");
    // Its journal row.
    let call = answer.call.expect("a row");
    let row = doc.journal_row(call).expect("recorded");
    assert_eq!(row.kind, CallKind::Command);
    assert_eq!(row.command.as_deref(), Some("cad.entities.transform"));
    assert_eq!(row.changed, vec![line]);
    assert_eq!(row.base, base.to_string());
    assert_eq!(row.revision, doc.revision().to_string());
    assert!(row.input.is_some() && row.input_digest.as_ref().is_some_and(|d| d.len() == 32));
    assert_eq!(
        row.context.as_ref().map(|c| c.plane.as_str()),
        Some("world")
    );
    assert!(row.step.is_some());
}

#[test]
fn a_plan_says_what_the_write_will_be_and_the_write_must_equal_it() {
    let mut doc = drawing();
    let line = uid(&doc, 1);
    let revision = doc.revision();
    let planned = run(
        &mut doc,
        envelope("cad.entities.transform", CallOp::Plan, shift(&line, 3.0)),
    );
    output(&planned);
    assert_eq!(doc.revision(), revision, "a plan writes nothing");
    let plan = planned.plan_id.clone().expect("a plan id");
    assert_eq!(plan.len(), 32);
    let planned_summary = planned.summary.clone().expect("a summary");
    assert_eq!(planned_summary.stamps.revision, None);
    // An unrelated write between: the plan still holds.
    let other = run(
        &mut doc,
        envelope(
            "cad.point.create",
            CallOp::Execute,
            point(487050.0, 4420050.0),
        ),
    );
    output(&other);
    let written = run(
        &mut doc,
        CallEnvelope {
            expected: Some(CallExpectations {
                plan: Some(plan.clone()),
                ..CallExpectations::default()
            }),
            ..envelope("cad.entities.transform", CallOp::Execute, shift(&line, 3.0))
        },
    );
    output(&written);
    assert_eq!(written.plan_id.as_deref(), Some(plan.as_str()));
    let s = written.summary.expect("a summary");
    assert_eq!(
        s.stamps.objects, planned_summary.stamps.objects,
        "the plan's stamps are the write's"
    );
    // The line moved again since: the same plan no longer holds, and nothing is written.
    let replanned = run(
        &mut doc,
        envelope("cad.entities.transform", CallOp::Plan, shift(&line, 3.0)),
    );
    let stale_plan = replanned.plan_id.expect("a plan id");
    let moved = run(
        &mut doc,
        envelope("cad.entities.transform", CallOp::Execute, shift(&line, 1.0)),
    );
    output(&moved);
    let at = doc.revision();
    let refused = run(
        &mut doc,
        CallEnvelope {
            expected: Some(CallExpectations {
                plan: Some(stale_plan),
                ..CallExpectations::default()
            }),
            ..envelope("cad.entities.transform", CallOp::Execute, shift(&line, 3.0))
        },
    );
    assert_eq!(code(&refused), ("conflict", "plan_changed"));
    let CommandResult::Conflict { error } = &refused.result else {
        unreachable!()
    };
    assert!(error.retryable);
    assert_eq!(doc.revision(), at, "nothing written");
}

#[test]
fn new_objects_plan_and_write_alike_though_their_ids_are_new() {
    let mut doc = drawing();
    let input = json!({ "steps": [
        { "command": "cad.point.create", "as": "p", "input": point(487020.0, 4420020.0) },
        { "command": "cad.entities.set", "input": { "uids": ["$p"], "color": "#E5484D", "operation": "color" } }
    ] });
    let planned = run(
        &mut doc,
        envelope("cad.changeset.apply", CallOp::Plan, input.clone()),
    );
    let plan = planned.plan_id.expect("a plan id");
    assert_eq!(doc.len(), 2, "a plan writes nothing");
    let written = run(
        &mut doc,
        CallEnvelope {
            expected: Some(CallExpectations {
                plan: Some(plan.clone()),
                ..CallExpectations::default()
            }),
            ..envelope("cad.changeset.apply", CallOp::Execute, input)
        },
    );
    output(&written);
    assert_eq!(written.plan_id, Some(plan));
    assert_eq!(doc.len(), 3);
    assert_eq!(doc.undo().as_deref(), Some("Değişiklik kümesi"));
}

#[test]
fn what_no_longer_holds_writes_nothing() {
    let mut doc = drawing();
    let (line, dot) = (uid(&doc, 1), uid(&doc, 2));
    let stamp = doc.stamp(Slot(1)).expect("a stamp");
    let at = doc.revision();
    let with = |objects: Vec<(&str, &str)>| CallExpectations {
        objects: objects
            .into_iter()
            .map(|(u, s)| (u.to_owned(), s.to_owned()))
            .collect(),
        ..CallExpectations::default()
    };
    // The stamps hold: written.
    let ok = run(
        &mut doc,
        CallEnvelope {
            expected: Some(with(vec![(&line, &stamp)])),
            ..envelope("cad.entities.transform", CallOp::Execute, shift(&line, 1.0))
        },
    );
    output(&ok);
    // The line changed since that stamp, and an object that is not there.
    let gone = Uuid::now_v7().to_string();
    let refused = run(
        &mut doc,
        CallEnvelope {
            expected: Some(with(vec![(&line, &stamp), (&gone, "0011223344556677")])),
            ..envelope("cad.entities.transform", CallOp::Execute, shift(&dot, 1.0))
        },
    );
    assert_eq!(code(&refused), ("conflict", "stale_state"));
    let CommandResult::Conflict { error } = &refused.result else {
        unreachable!()
    };
    let mut want = vec![line.clone(), gone.clone()];
    want.sort();
    assert_eq!(error.uids, want);
    assert!(error.retryable);
    assert!(
        error
            .path
            .as_deref()
            .is_some_and(|p| p.starts_with("expected.objects."))
    );
    // The revision.
    let refused = run(
        &mut doc,
        CallEnvelope {
            expected: Some(CallExpectations {
                revision: Some(at.to_string()),
                ..CallExpectations::default()
            }),
            ..envelope("cad.entities.transform", CallOp::Execute, shift(&dot, 1.0))
        },
    );
    assert_eq!(code(&refused), ("conflict", "revision_conflict"));
    // A stamp that is not one is the call's fault.
    let refused = run(
        &mut doc,
        CallEnvelope {
            expected: Some(with(vec![(&line, "XYZ")])),
            ..envelope("cad.entities.transform", CallOp::Execute, shift(&dot, 1.0))
        },
    );
    assert_eq!(code(&refused), ("failed", "invalid_call"));
}

#[test]
fn a_request_sent_again_is_answered_as_before_and_applied_once() {
    let mut doc = drawing();
    let send = |doc: &mut Document, input: Value| {
        run(
            doc,
            CallEnvelope {
                request_id: Some("ajan:1".into()),
                ..envelope("cad.point.create", CallOp::Execute, input)
            },
        )
    };
    let first = send(&mut doc, point(487020.0, 4420020.0));
    let made = output(&first).clone();
    assert!(!first.replayed);
    let (len, revision) = (doc.len(), doc.revision());
    let again = send(&mut doc, point(487020.0, 4420020.0));
    assert!(again.replayed && !again.undone);
    assert_eq!(output(&again), &made);
    assert_eq!(again.summary, first.summary);
    assert_eq!((doc.len(), doc.revision()), (len, revision), "applied once");
    // The same id for another call is refused.
    let other = send(&mut doc, point(1.0, 1.0));
    assert_eq!(code(&other), ("failed", "request_reused"));
    // Undone since: the answer says so.
    doc.undo();
    let after_undo = send(&mut doc, point(487020.0, 4420020.0));
    assert!(after_undo.replayed && after_undo.undone);
    // The ledger goes with the file.
    let reopened = Document::from_snapshot_v2(
        serde_json::from_value::<DocumentSnapshotV2>(
            serde_json::to_value(doc.to_snapshot_v2()).expect("writes"),
        )
        .expect("reads"),
    )
    .expect("opens");
    let mut reopened = reopened;
    let replayed = send(&mut reopened, point(487020.0, 4420020.0));
    assert!(replayed.replayed && replayed.undone);
    // A request id is only for execute, and by its rule.
    let refused = run(
        &mut doc,
        CallEnvelope {
            request_id: Some("ajan:2".into()),
            ..envelope("cad.point.create", CallOp::Plan, point(1.0, 1.0))
        },
    );
    assert_eq!(code(&refused), ("failed", "invalid_call"));
    let refused = run(
        &mut doc,
        CallEnvelope {
            request_id: Some("boşluk var".into()),
            ..envelope("cad.point.create", CallOp::Execute, point(1.0, 1.0))
        },
    );
    assert_eq!(code(&refused), ("failed", "invalid_call"));
}

#[test]
fn an_error_names_its_objects_and_whether_to_call_again() {
    let mut doc = drawing();
    let line = uid(&doc, 1);
    let missing = Uuid::now_v7().to_string();
    let refused = run(
        &mut doc,
        envelope(
            "cad.entities.delete",
            CallOp::Execute,
            json!({ "uids": [line, missing] }),
        ),
    );
    let CommandResult::Failed { error } = &refused.result else {
        panic!("{refused:?}");
    };
    assert_eq!(error.code, "entity_not_found");
    assert_eq!(error.path.as_deref(), Some("uids[1]"));
    assert_eq!(error.uids, vec![missing]);
    assert!(!error.retryable);
    // A command this host does not run, input that is not its type.
    assert_eq!(
        code(&run(
            &mut doc,
            envelope("cad.ufo.create", CallOp::Execute, json!({}))
        )),
        ("failed", "unknown_command")
    );
    assert_eq!(
        code(&run(
            &mut doc,
            envelope("cad.point.create", CallOp::Execute, json!({ "p": 1 }))
        )),
        ("failed", "invalid_input")
    );
}

#[test]
fn what_changes_with_an_object_is_its_dependent() {
    let snapshot: DocumentSnapshotV2 = serde_json::from_str(LINKED).expect("reads");
    let mut doc = Document::from_snapshot_v2(snapshot).expect("opens");
    let parcel = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1801";
    let text = "0192f5a0-7c3e-7d4a-9b1e-4c2f8a6d1803";
    let answer = run(
        &mut doc,
        envelope(
            "cad.entities.delete",
            CallOp::Execute,
            json!({ "uids": [parcel] }),
        ),
    );
    output(&answer);
    let s = answer.summary.expect("a summary");
    assert!(s.removed.contains(&parcel.to_owned()) && s.removed.contains(&text.to_owned()));
    assert_eq!(
        s.dependents,
        vec![text.to_owned()],
        "the linked text went with its object"
    );
}

#[test]
fn the_journal_says_what_was_written_in_order() {
    let mut doc = drawing();
    let line = uid(&doc, 1);
    output(&run(
        &mut doc,
        envelope("cad.point.create", CallOp::Execute, point(1.0, 1.0)),
    ));
    // An edit made without a command.
    let slot = doc
        .slot_of(Uuid::parse_str(&line).expect("uid"))
        .expect("there");
    doc.remove(&[slot]);
    output(&run(
        &mut doc,
        envelope(
            "cad.layers.set",
            CallOp::Execute,
            json!({ "operation": "visible", "layer": "gizli", "value": true }),
        ),
    ));
    let rows: Vec<_> = doc.journal().cloned().collect();
    assert_eq!(
        rows.iter()
            .map(|r| (r.kind, r.command.clone()))
            .collect::<Vec<_>>(),
        vec![
            (CallKind::Command, Some("cad.point.create".to_owned())),
            (CallKind::Edit, None),
            (CallKind::Command, Some("cad.layers.set".to_owned())),
        ]
    );
    assert_eq!(rows[1].removed, vec![line]);
    assert_eq!(rows[2].step, None, "the flags are no undo step");
    assert_eq!(rows[2].parts.len(), 1);
    doc.undo();
    assert_eq!(
        doc.journal_row(rows[1].seq).map(|r| r.status),
        Some(CallStatus::Undone)
    );
    assert_eq!(
        doc.journal_row(rows[0].seq).map(|r| r.status),
        Some(CallStatus::Done)
    );
}

#[test]
fn a_job_computes_on_a_copy_and_its_write_comes_in_whole() {
    let mut doc = drawing();
    let line = uid(&doc, 1);
    let mut job = Job::start(
        "j1".into(),
        &doc,
        envelope("cad.entities.transform", CallOp::Execute, shift(&line, 5.0)),
    );
    let status = job.wait(&mut doc);
    assert_eq!(status.phase, JobPhase::Done);
    let answer = status.answer.expect("an answer");
    output(&answer);
    assert!(answer.call.is_some());
    assert!(doc.undo().is_some());
    doc.redo();
    // Something else written meanwhile: its objects found by their ids, the rest kept.
    let mut job = Job::start(
        "j2".into(),
        &doc,
        envelope("cad.entities.transform", CallOp::Execute, shift(&line, 1.0)),
    );
    output(&run(
        &mut doc,
        envelope("cad.point.create", CallOp::Execute, point(9.0, 9.0)),
    ));
    let status = job.wait(&mut doc);
    assert_eq!(status.phase, JobPhase::Done, "{:?}", status.answer);
    // The line itself changed meanwhile: nothing written.
    let mut job = Job::start(
        "j3".into(),
        &doc,
        envelope("cad.entities.transform", CallOp::Execute, shift(&line, 1.0)),
    );
    output(&run(
        &mut doc,
        envelope("cad.entities.transform", CallOp::Execute, shift(&line, 2.0)),
    ));
    let at = doc.revision();
    let status = job.wait(&mut doc);
    assert_eq!(status.phase, JobPhase::Conflict);
    let answer = status.answer.expect("an answer");
    assert_eq!(code(&answer), ("conflict", "document_changed"));
    let CommandResult::Conflict { error } = &answer.result else {
        unreachable!()
    };
    assert_eq!(error.uids, vec![line.clone()]);
    assert_eq!(doc.revision(), at);
    // Stopped before it is written: nothing.
    let mut job = Job::start(
        "j4".into(),
        &doc,
        envelope("cad.entities.transform", CallOp::Execute, shift(&line, 1.0)),
    );
    job.cancel();
    let status = job.wait(&mut doc);
    assert_eq!(status.phase, JobPhase::Cancelled);
    assert_eq!(doc.revision(), at);
    // Stopped once written: it stands.
    let mut job = Job::start(
        "j5".into(),
        &doc,
        envelope("cad.entities.transform", CallOp::Execute, shift(&line, 1.0)),
    );
    assert_eq!(job.wait(&mut doc).phase, JobPhase::Done);
    let answer = job.stop_answer().expect("a word");
    assert_eq!(code(&answer), ("failed", "already_committed"));
}

#[test]
fn a_journal_replays_on_another_drawing_with_its_new_ids_mapped() {
    use kentos_commands::ReplayOutcome;
    use kentos_native_application::replay::{journal_of, replay};
    let mut doc = drawing();
    let line = uid(&doc, 1);
    let made = run(
        &mut doc,
        envelope(
            "cad.point.create",
            CallOp::Execute,
            point(487020.0, 4420020.0),
        ),
    );
    let dot = output(&made)["uid"].as_str().expect("a uid").to_owned();
    // A later row names the object an earlier one made.
    output(&run(
        &mut doc,
        envelope(
            "cad.entities.set",
            CallOp::Execute,
            json!({ "uids": [dot], "color": "#E5484D", "operation": "color" }),
        ),
    ));
    output(&run(
        &mut doc,
        envelope("cad.entities.transform", CallOp::Execute, shift(&line, 2.0)),
    ));
    // An edit without a command, and a row undone since.
    doc.set_layer_visible("gizli", true);
    output(&run(
        &mut doc,
        envelope("cad.point.create", CallOp::Execute, point(1.0, 1.0)),
    ));
    doc.undo();
    let journal = journal_of(&doc);
    assert_eq!(journal.format, "kentos.replay/1");
    let text = serde_json::to_string(&journal).expect("writes");
    let journal = serde_json::from_str(&text).expect("reads");
    let mut other = drawing();
    let report = replay(&mut other, None, &journal);
    assert_eq!(
        report.steps.iter().map(|s| s.outcome).collect::<Vec<_>>(),
        vec![
            ReplayOutcome::Same,
            ReplayOutcome::Same,
            ReplayOutcome::Same,
            ReplayOutcome::Skipped,
            ReplayOutcome::Skipped,
        ],
        "{report:?}"
    );
    assert_eq!((report.same, report.skipped, report.failed), (3, 2, 0));
    let new = report.mapped.get(&dot).expect("the point's id mapped");
    let slot = other
        .slot_of(Uuid::parse_str(new).expect("uid"))
        .expect("made there");
    assert_eq!(
        other
            .get(slot)
            .and_then(|e| e.base().color.clone())
            .as_deref(),
        Some("#E5484D")
    );
    // The line is the setup's own: the same id in both drawings, moved alike.
    assert_eq!(other.stamp(Slot(1)), doc.stamp(Slot(1)));
}

/// A stop in each phase a host can see (docs/adr/0253 §9): computing (a
/// command that asks stops, nothing written), staged (computed, not written:
/// the result is dropped), done (written: the stop says so, the write stands).
#[test]
fn a_stop_in_each_phase_does_what_the_phase_says() {
    use kentos_native_application::jobs::{JobOutcome, Progress};
    use std::time::{Duration, Instant};

    fn until(job: &Job, phase: JobPhase) {
        let end = Instant::now() + Duration::from_secs(20);
        while job.peek().phase != phase {
            assert!(
                Instant::now() < end,
                "never {phase:?}: {:?}",
                job.peek().phase
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    let mut doc = drawing();
    let line = uid(&doc, 1);
    let at = doc.revision();
    // Computing: the command asks for the stop and stops.
    let mut job = Job::start_with(
        "hesap".into(),
        "cad.entities.transform".into(),
        None,
        &doc,
        |_copy, progress: &Progress| {
            let end = Instant::now() + Duration::from_secs(20);
            while !progress.stopped() && Instant::now() < end {
                std::thread::sleep(Duration::from_millis(1));
            }
            JobOutcome {
                answer: CallAnswer::of(CommandResult::Cancelled),
                copy: None,
                effect: None,
            }
        },
    );
    until(&job, JobPhase::Computing);
    job.cancel();
    assert_eq!(job.wait(&mut doc).phase, JobPhase::Cancelled);
    assert_eq!(doc.revision(), at);
    // Staged: computed and waiting for the host; the stop drops it.
    let mut job = Job::start(
        "hazir".into(),
        &doc,
        envelope("cad.entities.transform", CallOp::Execute, shift(&line, 1.0)),
    );
    until(&job, JobPhase::Staged);
    assert_eq!(doc.revision(), at, "a staged result is not written");
    job.cancel();
    let status = job.wait(&mut doc);
    assert_eq!(status.phase, JobPhase::Cancelled);
    assert!(matches!(
        status.answer.map(|a| a.result),
        Some(CommandResult::Cancelled)
    ));
    assert_eq!(doc.revision(), at);
    // Done: the stop comes too late, and says so.
    let mut job = Job::start(
        "yazildi".into(),
        &doc,
        envelope("cad.entities.transform", CallOp::Execute, shift(&line, 1.0)),
    );
    assert_eq!(job.wait(&mut doc).phase, JobPhase::Done);
    job.cancel();
    assert_eq!(job.status(&mut doc).phase, JobPhase::Done);
    assert_eq!(
        code(&job.stop_answer().expect("a word")),
        ("failed", "already_committed")
    );
    assert_ne!(doc.revision(), at);
}
