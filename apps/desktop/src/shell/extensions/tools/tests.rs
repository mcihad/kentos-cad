//! The extensions' İşlemler tools on the desktop (tools.rs): listed from the registry and their descriptors, their
//! ribbon button opening their window, a descriptor that does not fit left out and said once. How a run ended, read
//! into İşlemler' answer and the note the log is written from, and the run itself: kentos-extension-tools' tests (the
//! desktop and the headless hosts share them, docs/adr/8057 §70); the process: kentos-python-env's process tests; the
//! tool's answer checked whole: kentos-processing's tests/external.rs; the whole way in a real window: §7.

use std::path::Path;

use kentos_extensions::archive::Limits;
use serde_json::json;

use kentos_launch::Module;

use super::super::ribbon::Action;
use super::super::{Event, Extensions};
use crate::app::App;

const IMAR: &str = "tr.ornek.imar-denetim";
pub(crate) const TAKS: &str = "ext.tr.ornek.imar-denetim.taks";

fn temp(name: &str) -> std::path::PathBuf {
    let dir =
        std::env::temp_dir().join(format!("kentos-eklenti-arac-{name}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// İmar's package, its files changed by `edit`.
pub(crate) fn imar(edit: impl Fn(&mut std::collections::BTreeMap<String, Vec<u8>>)) -> Vec<u8> {
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/extensions/v1/ornek-imar");
    let mut files = kentos_extension_host::store::read_files(&root).unwrap();
    edit(&mut files);
    kentos_extensions::pack(&files, &Limits::default())
        .unwrap_or_else(|p| panic!("{p:#?}"))
        .bytes
}

/// A GIS window with a drawing, İmar installed (everything granted), Python on and its environment ready.
pub(crate) fn with_imar(name: &str, bytes: Vec<u8>) -> App {
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
fn a_descriptor_that_does_not_fit_is_refused_before_kur() {
    // The desktop's own tool check runs on reviewing (plan/eklenti.md ADIM 19, "tanımın kurulumda tam denetimi"): a
    // parameter type the tools do not take is said there, with the descriptor, instead of a log line after "kuruldu".
    let bytes = imar(|files| {
        let mut d: serde_json::Value =
            serde_json::from_slice(&files["tools/taks.tool.json"]).unwrap();
        d["parameters"][2]["type"] = json!("rasterValues");
        files.insert(
            "tools/taks.tool.json".into(),
            serde_json::to_vec_pretty(&d).unwrap(),
        );
    });
    let root = temp("uymaz");
    let (mut gis, _) = App::boot(None);
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&root, false).0;
    gis.open_extensions();
    let _ = gis.extensions_event(Event::Picked(Some(("imar.kek".to_owned(), bytes))));
    let review = gis.extensions.review.as_ref().expect("reviewed");
    let found: Vec<&str> = review
        .review
        .inspection
        .problems
        .iter()
        .filter(|p| p.code == "tool_descriptor")
        .map(|p| p.message.as_str())
        .collect();
    assert!(
        found.len() == 1 && found[0].contains("“rasterValues” türü"),
        "{found:?}"
    );
    assert!(!review.review.installable);
}

#[test]
fn an_extensions_expression_function_is_known_to_islemler_by_name() {
    let mut gis = with_imar("ifade", imar(|_| {}));
    // The functions are the process's, as the window that changed its extensions last left them: another test's window
    // may have just written its own; this one writes İmar's again and reads them at once.
    gis.extensions_changed();
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
fn a_labels_call_never_waits_and_an_answer_not_given_is_not_kept() {
    use kentos_expression::external::{Arg, ExternalCalls};
    let mut gis = with_imar("etiket", imar(|_| {}));
    let labels = std::sync::Arc::clone(&gis.extensions.tools.labels);
    let calls = super::LabelCalls(std::sync::Arc::clone(&labels));
    let row = vec![Arg::Text("101".into()), Arg::Text("1".into())];
    let started = std::time::Instant::now();
    // Its environment's Python is a stand-in (/bin/sh) that never says it is ready: asked in the background.
    let first = calls.call(
        "ext.tr.ornek.imar-denetim.ada-parsel",
        std::slice::from_ref(&row),
    );
    assert!(
        started.elapsed() < std::time::Duration::from_millis(200),
        "it did not wait"
    );
    assert_eq!(first, [Arg::Null]);
    let until = std::time::Instant::now() + std::time::Duration::from_secs(30);
    while labels.waiting() && std::time::Instant::now() < until {
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
    // The process did not start: nothing kept (asked again once ready), no label worked out again in a loop, and the
    // failure said once in the extension's log.
    assert!(!labels.take_fresh(), "nothing kept, no redraw");
    gis.ext_tool_notes();
    let said = gis
        .extensions
        .registry
        .as_ref()
        .unwrap()
        .log_of(IMAR)
        .iter()
        .any(|r| r.code == "expression_failed" && r.message.contains("çalıştırılamadı"));
    assert!(said, "said in the log");
}
