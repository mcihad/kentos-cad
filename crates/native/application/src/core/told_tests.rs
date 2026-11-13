//! What every drawing that hears a stop within its own reckoning keeps (docs/adr/4040, 4860), checked on its shared
//! cases' project and first case: told, its ticks rise within one whole, each below it; a stop asked at the fourth tick
//! is heard there, cancelled; as a stopped job's part it draws nothing and the document stays as it was.

use std::cell::{Cell, RefCell};

use kentos_commands::{CommandResult, DocumentSnapshotV2};
use serde_json::Value;

use crate::ExecutionContext;
use crate::checks::Stop;

/// The project of the shared cases `cases` (a command's case file) and its first case's input.
pub(crate) fn project<T: serde::de::DeserializeOwned>(cases: &str) -> (kentos_domain::Document, T) {
    let cases: Value = serde_json::from_str(cases).expect("cases");
    let snapshot: DocumentSnapshotV2 = serde_json::from_value(cases["setup"].clone()).expect("a snapshot");
    let doc = kentos_domain::Document::from_snapshot_v2(snapshot).expect("opens");
    let input = serde_json::from_value(cases["cases"][0]["steps"][0]["input"].clone()).expect("an input");
    (doc, input)
}

/// `read` told: more than four ticks, rising within one whole and each below it; told again with a stop asked at the
/// fourth tick, cancelled there.
pub(crate) fn told_and_stopped<T>(read: impl Fn(&dyn Fn(u64, u64) -> bool) -> Result<T, Stop>) {
    let seen = RefCell::new(Vec::new());
    let whole = read(&|done, total| {
        seen.borrow_mut().push((done, total));
        false
    });
    if let Err(stop) = whole {
        panic!(
            "the whole reckoning is refused: {}",
            crate::checks::words(stop)
        );
    }
    let seen = seen.into_inner();
    assert!(seen.len() > 4, "{} ticks", seen.len());
    assert!(seen.windows(2).all(|w| w[0].0 <= w[1].0 && w[0].1 == w[1].1), "{seen:?}");
    assert!(seen.iter().all(|(done, total)| done < total), "{seen:?}");
    let asked = Cell::new(0);
    let stopped = read(&|_, _| {
        asked.set(asked.get() + 1);
        asked.get() > 3
    });
    assert!(matches!(stopped, Err(Stop::Cancelled)), "a stop is heard");
    assert_eq!(asked.get(), 4);
}

/// `draw` as a stopped job's part (a rebuild's, Tariften üret's): cancelled, the document as it was.
pub(crate) fn stopped_job_draws_nothing<T>(
    doc: &mut kentos_domain::Document,
    draw: impl FnOnce(&mut ExecutionContext<'_>) -> CommandResult<T>,
) {
    let before = doc.to_snapshot_v2();
    let job = crate::jobs::Progress::default();
    job.stop();
    let mut cx = ExecutionContext::new(doc);
    cx.job = Some(&job);
    assert!(matches!(draw(&mut cx), CommandResult::Cancelled));
    assert_eq!(doc.to_snapshot_v2(), before);
}
