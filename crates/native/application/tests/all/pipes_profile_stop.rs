//! A pipe run's profile as a job's part (docs/adr/4860): its pipes are told one at a time, so a job stopped before them
//! draws nothing and leaves the drawing as it was, and a job that ran shows its progress at the run's last pipe. The
//! network needs the processing tools, which this binary registers (the library's own tests have none).

use kentos_commands::{CommandResult, DocumentSnapshotV2, PipesProfile};
use kentos_native_application::ExecutionContext;
use kentos_native_application::jobs::Progress;
use serde_json::Value;

fn project() -> (kentos_domain::Document, PipesProfile) {
    kentos_processing::command::register();
    let cases: Value =
        serde_json::from_str(include_str!("../../../../../fixtures/commands/v1/cad.pipes.profile.json")).expect("cases");
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("a snapshot");
    let doc = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    let input = serde_json::from_value(cases["cases"][0]["steps"][0]["input"].clone()).expect("an input");
    (doc, input)
}

#[test]
fn a_stopped_job_draws_no_pipe_profile() {
    let (mut doc, input) = project();
    let before = doc.to_snapshot_v2();
    let job = Progress::default();
    job.stop();
    let r = kentos_native_application::pipes_profile::execute(&mut ExecutionContext::new(&mut doc).in_job(&job), input);
    assert!(matches!(r, CommandResult::Cancelled), "{r:?}");
    assert_eq!(doc.to_snapshot_v2(), before);
}

#[test]
fn a_job_s_progress_is_told_pipe_by_pipe() {
    let (mut doc, input) = project();
    let job = Progress::default();
    let r = kentos_native_application::pipes_profile::execute(&mut ExecutionContext::new(&mut doc).in_job(&job), input);
    let CommandResult::Completed { output, .. } = r else { panic!("{r:?}") };
    // The case's run has six pipes: the last told is the sixth's start, 5 of 6.
    assert_eq!(output.pipes.len(), 6);
    let told = job.fraction().expect("told");
    assert!((told - 5.0 / 6.0).abs() < 1e-12, "{told}");
}
