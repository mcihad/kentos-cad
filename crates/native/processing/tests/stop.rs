//! A stop set while a tool runs reaches the tool through its feedback (docs/adr/0551 §4): the runner reads the host's
//! flag as the tool asks `canceled()`, the run ends as canceled and the drawing is as it was. The flag here is set
//! by the tool itself, in the middle of its run, so the answer cannot come from the check before the run starts.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, OnceLock};

use kentos_commands::DocumentSnapshotV1;
use kentos_domain::{Document, Slot};
use kentos_processing::runner::{Outcome, Runner, Status};
use kentos_processing::types::{
    Feedback, OutputDef, OutputKind, Resolved, RunContext, RunResult, Target, Tool, Values,
};
use kentos_processing::{Bounds, Host, Scene};

static FLAG: OnceLock<Arc<AtomicBool>> = OnceLock::new();

/// How many questions the tool asked before it heard the stop (none heard: the loop's end).
static ASKED: OnceLock<std::sync::atomic::AtomicUsize> = OnceLock::new();

fn run(_: &Resolved<'_>, _: &RunContext<'_>, feedback: &mut dyn Feedback) -> RunResult {
    // The stop arrives while the tool runs.
    FLAG.get().expect("a flag").store(true, Ordering::Relaxed);
    let asked = ASKED.get_or_init(Default::default);
    for _ in 0..1000 {
        asked.fetch_add(1, Ordering::Relaxed);
        if feedback.canceled() {
            return RunResult::default();
        }
    }
    RunResult {
        summary: Some("bitti".into()),
        ..RunResult::default()
    }
}

fn tool() -> Tool {
    Tool {
        id: "test.polls".into(),
        label: "Sorular".into(),
        category: "analysis".into(),
        description: "Durdurma bayrağını soran araç.".into(),
        help: None,
        keywords: Vec::new(),
        aliases: Vec::new(),
        icon: None,
        parameters: Vec::new(),
        outputs: vec![OutputDef::new("count", "Sayı", OutputKind::Number)],
        targets: vec![Target::Client],
        validate: None,
        preview: None,
        run: Some(run),
    }
}

struct Bare(Document);

impl Scene for Bare {
    fn doc(&self) -> &Document {
        &self.0
    }

    fn selected(&self) -> Vec<Slot> {
        Vec::new()
    }

    fn visible_bounds(&self) -> Option<Bounds> {
        None
    }
}

impl Host for Bare {
    fn doc_mut(&mut self) -> &mut Document {
        &mut self.0
    }

    fn select(&mut self, _: &[Slot]) {}
}

#[test]
fn a_stop_set_while_the_tool_runs_ends_the_run_as_canceled() {
    let path = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../../fixtures/processing/v1/parcels.kcad");
    let text = std::fs::read_to_string(path).expect("reads");
    let doc = Document::from_snapshot(DocumentSnapshotV1::from_json(&text).expect("reads"))
        .expect("opens");
    let revision = doc.revision();
    let flag = Arc::new(AtomicBool::new(false));
    FLAG.set(Arc::clone(&flag)).expect("set once");
    let mut host = Bare(doc);
    let mut runner = Runner::new().with_stop(flag);
    assert!(!runner.stop_asked());
    let mut log = Vec::new();
    let outcome = runner.run(&mut host, &tool(), &Values::new(), true, &mut log);
    assert!(matches!(
        outcome,
        Outcome::Stopped {
            status: Status::Canceled,
            ..
        }
    ));
    assert!(runner.stop_asked());
    // The tool heard it at its first question after the flag was set.
    assert_eq!(ASKED.get().expect("asked").load(Ordering::Relaxed), 1);
    assert_eq!(host.0.revision(), revision);
    assert!(!host.0.can_undo());
}
