//! How a run ended, read into İşlemler' answer and the host's note (run.rs): the same on the desktop and headless.
//! Moved here from the desktop's tests with the code (docs/adr/8057 §70).

use kentos_processing::external::Failure;
use serde_json::json;

use super::{Concluded, Note};

fn concluded(canceled: bool) -> Concluded<'static> {
    Concluded {
        ext: "tr.ornek.imar-denetim",
        part: "processing:taks",
        label: "Parsel TAKS hesabı",
        timeout: 600,
        canceled,
        took: 0.4,
        output: "ara sonuç\n",
        memory: Some(52 * 1_048_576),
    }
}

/// How a run ended is read into İşlemler' answer and the host's note, the same on the desktop and headless.
#[test]
fn how_a_run_ended_is_read_into_islemler_and_the_note() {
    use kentos_python_env::process::Ended;
    // Its answer: İşlemler gets the result, the note a line.
    let (answer, note) = concluded(false).conclude(Ended::Answered(json!({
        "type": "done", "ok": true, "result": { "kind": "outputs", "outputs": { "asan": 1 } }
    })));
    assert_eq!(
        answer,
        Ok(json!({ "kind": "outputs", "outputs": { "asan": 1 } }))
    );
    assert!(
        matches!(&note, Some(Note::Worked { code: "run_finished", message, .. })
            if message == "Bitti (0,4 sn; süreç belleği 52 MB)."),
        "{note:?}"
    );
    // Its code failed: said with its exception, its traceback and what it printed in the detail.
    let (answer, note) = concluded(false).conclude(Ended::Answered(json!({
        "type": "done", "ok": false, "exception": "ZeroDivisionError", "message": "division by zero",
        "error": "Traceback …\n  File \"logic.py\", line 9, in taks\nZeroDivisionError: division by zero"
    })));
    assert_eq!(
        answer,
        Err(Failure::Failed(
            "“Parsel TAKS hesabı” hata verdi: ZeroDivisionError: division by zero".into()
        ))
    );
    let Some(Note::Failed {
        code,
        detail,
        crash,
        title,
        ..
    }) = note
    else {
        panic!("{note:?}");
    };
    assert_eq!(
        (code, crash, title.as_str()),
        ("run_failed", false, "Parsel TAKS hesabı")
    );
    let detail = detail.unwrap();
    assert!(
        detail.starts_with("ara sonuç") && detail.contains("logic.py"),
        "{detail}"
    );
    // Durdur: answered, the run is cancelled; not asked, it stopped by itself.
    let stopped = json!({ "type": "done", "ok": false, "stopped": true, "stoppedAfterMs": 6 });
    let (_, note) = concluded(true).conclude(Ended::Answered(stopped.clone()));
    assert!(
        matches!(&note, Some(Note::Worked { code: "run_stopped", message, .. })
            if message == "Durdur'dan 6 ms sonra durdu; çizim değişmedi."),
        "{note:?}"
    );
    assert_eq!(
        concluded(true).conclude(Ended::Answered(stopped.clone())).0,
        Err(Failure::Stopped)
    );
    assert!(
        matches!(concluded(false).conclude(Ended::Answered(stopped)).0, Err(Failure::Failed(w)) if w.contains("kendiliğinden"))
    );
    // Not answered: ended, cancelled, said.
    let (answer, note) = concluded(true).conclude(Ended::Killed { timed_out: false });
    assert_eq!(answer, Err(Failure::Stopped));
    assert!(matches!(
        note,
        Some(Note::Worked {
            code: "run_killed",
            ..
        })
    ));
    // Its time over: a failure.
    let (answer, _) = concluded(false).conclude(Ended::Killed { timed_out: true });
    assert!(matches!(answer, Err(Failure::Failed(w)) if w.contains("600 sn içinde bitmedi")));
    // A crash: for the watchdog.
    let (_, note) =
        concluded(false).conclude(Ended::Crashed("çıkış: 3\nSegmentation fault".into()));
    assert!(matches!(
        note,
        Some(Note::Failed {
            code: "runner_crashed",
            crash: true,
            ..
        })
    ));
    // Not started, refused, unavailable: said as they are.
    let (answer, _) = concluded(false).conclude(Ended::NotStarted(
        "Bu eklentinin Python ortamı hazırlanmadı".into(),
    ));
    assert!(matches!(answer, Err(Failure::Failed(w)) if w.contains("ortamı hazırlanmadı")));
    let (_, note) = concluded(false).conclude(Ended::Answered(
        json!({ "ok": false, "refused": true, "message": "işaretsiz" }),
    ));
    assert!(matches!(
        note,
        Some(Note::Failed {
            code: "handler_refused",
            ..
        })
    ));
    let (_, note) = concluded(false).conclude(Ended::Answered(
        json!({ "ok": false, "unavailable": true, "message": "yok" }),
    ));
    assert!(matches!(
        note,
        Some(Note::Failed {
            code: "run_unavailable",
            ..
        })
    ));
    assert_eq!(
        concluded(true).conclude(Ended::StoppedBefore),
        (Err(Failure::Stopped), None)
    );
}
