//! An extension's tool a form's action or a model's step is pinned to, when what it rests on is taken away
//! (docs/adr/8057 §122, plan/eklenti.md ECO-QA-12): the capability the person did not grant, the Python contributions
//! not chosen, an environment that could not be prepared (offline, a wheel this computer has none of). Each is said as
//! itself. Before, a tool the grant left out read "bu aracı içermiyor", and a failed preparation only "hazırlanmadı".
//! Another version or another build of the pinned extension (a changed source) is `demo`'s.

use super::demo::vitrin_package;
use super::*;
use crate::extensions::PinnedTool;

const TOOL: &str = "ext.tr.ornek.vitrin.taks";
const VITRIN: &str = "tr.ornek.vitrin";

/// Vitrin reviewed, the person's choice made by `choose`, installed.
fn installed_with(gis: &mut App, choose: Event) {
    gis.open_extensions();
    let _ = gis.extensions_event(Event::Picked(Some(("vitrin.kek".into(), vitrin_package()))));
    let _ = gis.extensions_event(choose);
    let _ = gis.extensions_event(Event::Install);
    assert!(gis.extensions.review.is_none(), "{:?}", gis.extensions.note);
}

fn unavailable(state: PinnedTool) -> String {
    match state {
        PinnedTool::Unavailable(why) => why,
        other => panic!("{other:?}"),
    }
}

#[test]
fn a_tool_the_grant_left_out_is_said_as_that_not_as_missing() {
    let dir = temp("araca-izin");
    let mut gis = app(&dir, Some(Module::Gis));
    installed_with(
        &mut gis,
        Event::Grant(Capability::ProcessingProvider, false),
    );
    let why = unavailable(gis.pinned_tool_state(TOOL, None));
    assert!(
        why.contains(
            "bu aracı içeriyor ama araç kurulmadı: Verilmeyen yetenek: “İşlemler'e araç ekler”."
        ),
        "{why}"
    );
    assert!(!why.contains("içermiyor"), "{why}");
    let _ = std::fs::remove_dir_all(&dir);
    // Its Python contributions not chosen: said so.
    let dir = temp("araca-python");
    let mut gis = app(&dir, Some(Module::Gis));
    installed_with(&mut gis, Event::Python(false));
    let why = unavailable(gis.pinned_tool_state(TOOL, None));
    assert!(
        why.contains("bu aracı içeriyor ama araç kurulmadı: Python katkıları kurulumda seçilmedi."),
        "{why}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_pinned_tool_says_why_its_environment_could_not_be_prepared() {
    use super::environment::EnvEvent;
    use crate::python_setup::{OwnPython, PythonEdit};
    use kentos_python_env::python::{Cause, InstallError, Interpreter};
    use kentos_python_env::uv::UvStatus;
    let dir = temp("araca-ortam");
    let python_root = temp("araca-ortam-python");
    let mut gis = app(&dir, Some(Module::Gis));
    gis.python_setup.root = Some(python_root.clone());
    install(&mut gis, "vitrin.kek", vitrin_package());
    // KentOS's Python ready, the extension's environment not prepared yet.
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
    let _ = gis.python_edit(PythonEdit::Found(Ok((Some(python), 97_885_768))));
    assert!(matches!(
        gis.python_setup.python,
        Some(OwnPython::Ready(..))
    ));
    let pending = |gis: &App| match gis.pinned_tool_state(TOOL, None) {
        PinnedTool::Pending(why) => why,
        other => panic!("{other:?}"),
    };
    assert!(
        pending(&gis).starts_with("Bu eklentinin Python ortamı hazırlanmadı:"),
        "{}",
        pending(&gis)
    );
    // Preparing it offline fails: the form's button and the model's step say why.
    let failed = InstallError::Failed {
        cause: Cause::Network,
        said: "Failed to fetch: `https://pypi.org/simple/shapely/`".into(),
    };
    let _ = gis.extensions_event(Event::Env(EnvEvent::Prepared(VITRIN.into(), Err(failed))));
    let why = pending(&gis);
    assert!(
        why.starts_with("Bu eklentinin Python ortamı hazırlanamadı: Paketler indirilemedi: internete ulaşılamadı."),
        "{why}"
    );
    assert!(why.ends_with("› Python ortamı."), "{why}");
    let note = gis.step_pin_note(TOOL, None).expect("an extension's tool");
    assert!(
        note.starts_with("Bekliyor: Bu eklentinin Python ortamı hazırlanamadı"),
        "{note}"
    );
    let _ = std::fs::remove_dir_all(&dir);
    let _ = std::fs::remove_dir_all(&python_root);
}
