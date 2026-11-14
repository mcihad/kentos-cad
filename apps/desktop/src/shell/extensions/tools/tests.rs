//! The extensions' İşlemler tools on the desktop (tools.rs): listed from the registry and their descriptors, their
//! ribbon button opening their window, a descriptor that does not fit left out and said once; how a run ended read
//! into İşlemler' answer and the extension's log. The process itself: kentos-python-env's process tests; the tool's
//! answer checked whole: kentos-processing's tests/external.rs; the whole way in a real window: docs/adr/8057 §7.

use std::path::Path;

use kentos_extensions::archive::Limits;
use kentos_processing::external::Failure;
use kentos_python_env::process::Ended;
use serde_json::json;

use kentos_launch::Module;

use super::super::ribbon::Action;
use super::super::{Event, Extensions};
use super::{Concluded, Note};
use crate::app::App;

const IMAR: &str = "tr.ornek.imar-denetim";
const TAKS: &str = "ext.tr.ornek.imar-denetim.taks";

fn temp(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("kentos-eklenti-arac-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// İmar's package, its files changed by `edit`.
fn imar(edit: impl Fn(&mut std::collections::BTreeMap<String, Vec<u8>>)) -> Vec<u8> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/extensions/v1/ornek-imar");
    let mut files = kentos_extension_host::store::read_files(&root).unwrap();
    edit(&mut files);
    kentos_extensions::pack(&files, &Limits::default())
        .unwrap_or_else(|p| panic!("{p:#?}"))
        .bytes
}

/// A GIS window with a drawing, İmar installed (everything granted), Python on and its environment ready.
fn with_imar(name: &str, bytes: Vec<u8>) -> App {
    use crate::python_setup::PythonEdit;
    use kentos_python_env::extension::{self, Ready};
    use kentos_python_env::python::Interpreter;
    use kentos_python_env::uv::UvStatus;

    let root = temp(name);
    let python_root = temp(&format!("{name}-python"));
    let mut gis = crate::files_testing::app_with_drawing();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&root, false).0;
    gis.extensions_changed();
    gis.python_setup.root = Some(python_root.clone());
    gis.open_extensions();
    let _ = gis.extensions_event(Event::Picked(Some(("imar.kek".to_owned(), bytes))));
    let _ = gis.extensions_event(Event::Install);
    assert!(gis.extensions.review.is_none(), "{:?}", gis.extensions.note);
    let _ = gis.python_edit(PythonEdit::Enable(true));
    let _ = gis.python_edit(PythonEdit::Detected(UvStatus::Found {
        path: "/k/tools/uv".into(),
        version: "0.11.26".into(),
        bundled: true,
    }));
    let python = Interpreter {
        version: "3.12.13".into(),
        path: python_root.join("yorumlayici/cpython-3.12/bin/python3.12"),
    };
    let _ = gis.python_edit(PythonEdit::Found(Ok((Some(python.clone()), 1))));
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(IMAR))
        .unwrap();
    let key = extension::key_for(&package, "uv.lock", IMAR, "1.1.0", &python, &[]).unwrap();
    let dir = extension::place(&python_root, &key);
    std::fs::create_dir_all(&dir).unwrap();
    let ready = Ready {
        format: "kentos.extension-env".into(),
        key,
        python: "/bin/sh".into(),
        packages: Vec::new(),
        sdk: String::new(),
        made_at: 1,
    };
    std::fs::write(
        dir.join(extension::RECORD),
        serde_json::to_string(&ready).unwrap(),
    )
    .unwrap();
    gis.extensions_changed();
    gis
}

#[test]
fn an_extensions_tool_is_listed_in_islemler_and_its_button_opens_its_window() {
    let mut gis = with_imar("listelenir", imar(|_| {}));
    let registry = &gis.gis.processing.registry;
    let tool = registry.get(TAKS).expect("İmar's TAKS tool in İşlemler");
    assert_eq!(tool.label, "Parsel TAKS hesabı");
    assert_eq!(registry.category_path(&tool.category), "Eklentiler");
    let names: Vec<&str> = tool.parameters.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names, ["parseller", "yapilar", "taks_siniri", "cikti"]);
    assert!(
        tool.help
            .as_deref()
            .unwrap()
            .contains("ayrı süreç bir korumalı alan değildir")
    );
    assert_eq!(registry.external().len(), 1);
    // The toolbox's tree has it under Eklentiler.
    let tree = registry.tree(&|_| true);
    let extensions = tree
        .iter()
        .find(|n| n.category.id == kentos_processing::external::CATEGORY)
        .expect("the Eklentiler branch");
    assert_eq!(extensions.tools[0].id, TAKS);
    // İmar's tab: its second button opens the tool's window.
    let button = gis
        .extension_tabs()
        .find(|t| t.extension == IMAR)
        .expect("İmar's tab")
        .panels[0]
        .items[1]
        .clone();
    assert_eq!(button.label, "TAKS");
    assert!(
        matches!(button.action, Action::Python(ref e, ref k, ref l) if e == IMAR && k == "processing" && l == "taks"),
        "{:?}",
        button.action
    );
    let _ = gis.extensions_event(Event::Run(IMAR.into(), "processing".into(), "taks".into()));
    let window = gis.gis.processing.dialog.as_ref().expect("its window");
    assert_eq!(window.tool.id, TAKS);
    // Turned off: gone from İşlemler.
    let _ = gis.extensions_event(Event::Disable(IMAR.into()));
    assert!(
        gis.gis.processing.registry.get(TAKS).is_none(),
        "an extension turned off adds no tool"
    );
}

#[test]
fn a_descriptor_that_does_not_fit_is_said_once_and_its_tool_left_out() {
    let bytes = imar(|files| {
        let mut d: serde_json::Value =
            serde_json::from_slice(&files["tools/taks.tool.json"]).unwrap();
        d["parameters"][2]["type"] = json!("rasterValues");
        files.insert(
            "tools/taks.tool.json".into(),
            serde_json::to_vec_pretty(&d).unwrap(),
        );
    });
    let mut gis = with_imar("uymaz", bytes);
    assert!(gis.gis.processing.registry.get(TAKS).is_none());
    gis.extensions_changed();
    let log = gis
        .extensions
        .registry
        .as_ref()
        .unwrap()
        .log_of(IMAR)
        .iter()
        .filter(|r| r.code == "tool_descriptor")
        .collect::<Vec<_>>();
    assert_eq!(log.len(), 1, "said once a session: {log:?}");
    assert!(
        log[0].message.contains("“rasterValues” türü"),
        "{}",
        log[0].message
    );
}

fn concluded(canceled: bool) -> Concluded<'static> {
    Concluded {
        ext: IMAR,
        part: "processing:taks",
        label: "Parsel TAKS hesabı",
        timeout: 600,
        canceled,
        took: 0.4,
        output: "ara sonuç\n",
        memory: Some(52 * 1_048_576),
    }
}

#[test]
fn how_a_run_ended_is_read_into_islemler_and_the_log() {
    // Its answer: İşlemler gets the result, the log a line.
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
    // Not answered: ended, cancelled, said in the log.
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

#[test]
fn an_extensions_expression_function_is_known_to_islemler_by_name() {
    let gis = with_imar("ifade", imar(|_| {}));
    let defs = kentos_processing::external::expressions();
    let def = defs
        .iter()
        .find(|d| d.name == "ext.tr.ornek.imar-denetim.ada-parsel")
        .expect("İmar's function");
    assert!(
        def.description.contains("Ada/parsel yazımı"),
        "{}",
        def.description
    );
    // The window's check of an İşlemler expression knows it; an unknown one is refused.
    let tool = gis
        .gis
        .processing
        .registry
        .get("attributes.calculate")
        .unwrap();
    let mut values = serde_json::Map::new();
    values.insert("input".into(), json!({ "scope": "all" }));
    values.insert("field".into(), json!("etiket"));
    values.insert(
        "value".into(),
        json!("ext.tr.ornek.imar-denetim.ada-parsel(Ada, Parsel)"),
    );
    values.insert("empty".into(), json!("keep"));
    values.insert("label".into(), json!(false));
    let doc = gis.document.as_ref().unwrap();
    let issues = kentos_processing::parameters::validate_values(&tool, &values, doc.model.layers());
    assert!(issues.is_empty(), "{issues:?}");
}

#[test]
fn a_labels_call_never_waits_and_its_rows_are_asked_in_the_background() {
    use kentos_expression::external::{Arg, ExternalCalls};
    let gis = with_imar("etiket", imar(|_| {}));
    let labels = std::sync::Arc::clone(&gis.extensions.tools.labels);
    let before = kentos_native_application::label_texts::answers_generation();
    let calls = super::LabelCalls(std::sync::Arc::clone(&labels));
    let row = vec![Arg::Text("101".into()), Arg::Text("1".into())];
    let started = std::time::Instant::now();
    // Its environment's Python is a stand-in (/bin/sh): the answer is empty, and kept.
    let first = calls.call("ext.tr.ornek.imar-denetim.ada-parsel", &[row.clone()]);
    assert!(
        started.elapsed() < std::time::Duration::from_millis(200),
        "it did not wait"
    );
    assert_eq!(first, [Arg::Null]);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while labels.waiting() && !labels.take_fresh() && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    assert!(kentos_native_application::label_texts::answers_generation() > before);
    // Asked once: the row is kept (empty) and not asked again.
    let again = calls.call("ext.tr.ornek.imar-denetim.ada-parsel", &[row]);
    assert_eq!(again, [Arg::Null]);
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert!(!labels.waiting() || labels.take_fresh(), "no second ask");
}
