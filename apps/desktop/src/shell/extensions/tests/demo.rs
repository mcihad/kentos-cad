//! The demo extension, Vitrin (samples/eklenti-vitrini), and what it shows (docs/adr/8057 §11–§24): its tab, the
//! form's and the model's pins, another extension's tool, an audit's findings, a blueprint installed, a panel.

use super::*;

#[test]
fn a_tool_the_desktop_would_not_make_is_refused_before_kur() {
    // plan/eklenti.md ADIM 19, "tanımın kurulumda tam denetimi": the package's own check reads the descriptor's format,
    // version and id; a parameter Python cannot name (`class`) passed it, the install said "kuruldu" and the tool was
    // found out only when it was listed, in a log line. The review now asks the desktop's own tool check
    // (`kentos_processing::external::from_descriptor`) and says it before Kur.
    let root =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/extensions/v1/ornek-imar");
    let mut files = kentos_extension_host::store::read_files(&root).unwrap();
    let mut tool: serde_json::Value =
        serde_json::from_slice(&files["tools/taks.tool.json"]).unwrap();
    tool["parameters"][0]["name"] = serde_json::json!("class");
    files.insert(
        "tools/taks.tool.json".into(),
        serde_json::to_vec_pretty(&tool).unwrap(),
    );
    let bytes = kentos_extensions::pack(&files, &Limits::default())
        .unwrap_or_else(|p| panic!("the package's own check passes it: {p:#?}"))
        .bytes;
    let dir = temp("arac-tanimi");
    // Not `app`: its extensions_changed writes the process's functions, which other tests read.
    let (mut gis, _) = App::boot(None);
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&dir, false).0;
    gis.open_extensions();
    let _ = gis.extensions_event(Event::Picked(Some(("imar.kek".to_owned(), bytes.clone()))));
    let review = gis.extensions.review.as_ref().expect("reviewed");
    let problem = review
        .review
        .inspection
        .problems
        .iter()
        .find(|p| p.code == "tool_descriptor")
        .unwrap_or_else(|| panic!("{:#?}", review.review.inspection.problems));
    assert_eq!(problem.file.as_deref(), Some("tools/taks.tool.json"));
    assert!(
        problem.message.contains("“taks” aracı") && problem.message.contains("class"),
        "{}",
        problem.message
    );
    assert!(!review.review.installable, "not installable");
    // Installing the bytes anyway (another way in) is refused the same.
    let registry = gis.extensions.registry.as_mut().expect("a registry");
    let source = kentos_extension_host::store::Source {
        kind: kentos_extension_host::store::SourceKind::File,
        location: "imar.kek".to_owned(),
    };
    let refused = registry
        .install(&bytes, &source, &kentos_extension_host::Choice::default())
        .expect_err("refused");
    assert_eq!(refused[0].code, "tool_descriptor");
}

/// The demo extension's version, as its manifest says (the tests follow its releases).
fn vitrin_version() -> String {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../samples/eklenti-vitrini/eklenti/manifest.json");
    let manifest: serde_json::Value =
        serde_json::from_slice(&std::fs::read(manifest).unwrap()).unwrap();
    manifest["version"].as_str().unwrap().to_owned()
}

/// samples/eklenti-vitrini's extension packed from its source, as `hazirla.sh` packs it.
pub(super) fn vitrin_package() -> Vec<u8> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/eklenti-vitrini/eklenti");
    let files: std::collections::BTreeMap<String, Vec<u8>> =
        kentos_extension_host::store::read_files(&root)
            .unwrap()
            .into_iter()
            .filter(|(path, _)| !path.contains("__pycache__"))
            .collect();
    kentos_extensions::pack(&files, &Limits::default())
        .unwrap_or_else(|p| panic!("{p:#?}"))
        .bytes
}

#[test]
fn the_demo_extension_installs_with_every_contribution_on_its_tab() {
    // samples/eklenti-vitrini: the extension every demo uses (the owner, 14 November 2026). The review asks the
    // desktop's own tool check (its TAKS tool passes), and its tab shows each of its Python contributions.
    let dir = temp("vitrin");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let tab = gis
        .extension_tabs()
        .find(|t| t.extension == "tr.ornek.vitrin")
        .expect("Vitrin's tab");
    let labels: Vec<String> = tab
        .panels
        .iter()
        .flat_map(|p| p.items.iter().map(|i| i.label.clone()))
        .collect();
    assert_eq!(
        labels,
        [
            "Katman özeti",
            "Yapıları numarala",
            "Hata örneği",
            "Katmanları kur",
            "TAKS",
            "Yapı numarası",
            "Yapı tamponu",
            "Çekme mesafesi",
            "Yapı ara",
            "Değer say",
            "Parsel kartı",
            "Yakın yapılar",
            "Alandaki yapılar",
            "Bakım kayıtları",
            "Yapı yerleştir",
            "Parsel çakışması"
        ]
    );
}

#[test]
fn a_forms_action_opens_an_extensions_tool_pinned_and_says_why_when_it_cannot() {
    // docs/adr/8057 §12, FORM-API-13: a form's button on Vitrin's TAKS tool. Missing, its code not runnable yet or
    // turned off, the button is off and says so; the designer pins the extension and says when it drifts.
    use crate::extensions::PinnedTool;
    use crate::form_scenes::{designer, entry, meters_rebodied};
    use crate::forms::{designer as d, entry as en};
    use kentos_commands::{ExtensionPin, FormItem};
    const TOOL: &str = "ext.tr.ornek.vitrin.taks";
    const VITRIN: &str = "tr.ornek.vitrin";
    let dir = temp("form-eylemi");
    let mut gis = app(&dir, Some(Module::Gis));
    meters_rebodied(
        &mut gis,
        serde_json::json!([
            { "type": "field", "id": "ad", "field": "Ad" },
            { "type": "action", "id": "taks", "label": "TAKS hesapla", "tool": TOOL }
        ]),
    );
    let unavailable = |state: PinnedTool| match state {
        PinnedTool::Unavailable(why) => why,
        other => panic!("{other:?}"),
    };
    let stale = ExtensionPin {
        extension: VITRIN.into(),
        version: "0.9.0".into(),
        digest: format!("sha256:{}", "0".repeat(64)),
    };
    // Not installed: which extension, and with a pin which version the form expects.
    let why = unavailable(gis.pinned_tool_state(TOOL, None));
    assert!(
        why.contains("tr.ornek.vitrin eklentisi bu bilgisayarda kurulu değil"),
        "{why}"
    );
    let why = unavailable(gis.pinned_tool_state(TOOL, Some(&stale)));
    assert!(
        why.contains("tr.ornek.vitrin 0.9.0 bu bilgisayarda kurulu değil"),
        "{why}"
    );
    let why = unavailable(gis.pinned_tool_state("vitrin.yok", None));
    assert!(why.contains("kayıtlı bir işlem aracı değil"), "{why}");
    // Installed, its Python not ready (no Python in tests): pending, the press opens nothing.
    install(&mut gis, "vitrin.kek", vitrin_package());
    let pending = gis.pinned_tool_state(TOOL, None);
    assert!(matches!(pending, PinnedTool::Pending(_)), "{pending:?}");
    gis.selection.set([kentos_domain::Slot(1)]);
    crate::tools_scenes::run(&mut gis, "data.formEdit");
    assert!(gis.gis.form_entry.is_some(), "the form");
    entry(&mut gis, en::Event::RunAction("taks".into()));
    assert!(
        gis.gis.processing.dialog.is_none(),
        "pending: nothing opened"
    );
    // Its code can run: the press opens the tool on the record.
    gis.extensions.python_reasons.insert(VITRIN.into(), None);
    assert_eq!(gis.pinned_tool_state(TOOL, None), PinnedTool::Ready);
    entry(&mut gis, en::Event::RunAction("taks".into()));
    assert_eq!(
        gis.gis
            .processing
            .dialog
            .as_ref()
            .map(|d| d.tool.id.as_str()),
        Some(TOOL),
        "the tool's window"
    );
    assert_eq!(gis.selection.ids(), [kentos_domain::Slot(1)]);
    let values = &gis.gis.processing.dialog.as_ref().unwrap().values;
    assert_eq!(
        values["parseller"],
        serde_json::json!({ "scope": "selection" }),
        "the record is the tool's input"
    );
    // Its other input, left at the descriptor's default ("yapi", no layer of this drawing), opens on the first layer
    // holding its kinds (docs/adr/1080 §1): the buildings.
    assert_eq!(
        values["yapilar"]["layerId"], "bina",
        "its other input on a layer holding polygons"
    );
    // The pin: the installed version and digest; another version, another build or none is said.
    let pin = gis.tool_pin_of(TOOL).expect("installed");
    assert_eq!(pin.version, vitrin_version());
    assert!(pin.digest.starts_with("sha256:") && pin.digest.len() == 71);
    assert_eq!(gis.tool_pin_drift(TOOL, Some(&pin)), None);
    let drift = gis
        .tool_pin_drift(TOOL, Some(&stale))
        .expect("another version");
    assert!(
        drift.contains(&format!(
            "0.9.0 ile seçildi, kurulu sürüm {}",
            vitrin_version()
        )),
        "{drift}"
    );
    let rebuilt = ExtensionPin {
        digest: format!("sha256:{}", "1".repeat(64)),
        ..pin.clone()
    };
    let drift = gis
        .tool_pin_drift(TOOL, Some(&rebuilt))
        .expect("another build");
    assert!(drift.contains("içeriği farklı"), "{drift}");
    let drift = gis.tool_pin_drift(TOOL, None).expect("not pinned");
    assert!(drift.contains("sabitlenmemiş"), "{drift}");
    assert_eq!(gis.tool_pin_drift("attributes.calculate", None), None);
    // The designer: the preflight says the missing pin; choosing the tool pins it, the warning goes.
    crate::tools_scenes::run(&mut gis, "data.forms");
    designer(&mut gis, d::Event::SelectItem("taks".into()));
    let fields = gis.layer_fields_of("bina");
    let codes = |gis: &App| -> Vec<String> {
        let body = gis.gis.forms_window.as_ref().unwrap().body.clone().unwrap();
        gis.issues_of(&body, &fields)
            .into_iter()
            .map(|i| i.code)
            .collect()
    };
    assert!(codes(&gis).contains(&kentos_forms::codes::ACTION_PIN.to_owned()));
    designer(&mut gis, d::Event::PinnedTool(TOOL.into()));
    let body = gis.gis.forms_window.as_ref().unwrap().body.clone().unwrap();
    let pinned = body.walk().into_iter().find_map(|(_, i)| match i {
        FormItem::Action { pin, .. } => pin.clone(),
        _ => None,
    });
    assert_eq!(pinned, Some(pin));
    assert!(!codes(&gis).contains(&kentos_forms::codes::ACTION_PIN.to_owned()));
    // Turned off: unavailable with the extension's state; the preflight says the button is off.
    let _ = gis.extensions_event(Event::Disable(VITRIN.into()));
    let why = unavailable(gis.pinned_tool_state(TOOL, None));
    assert!(why.contains("Eklenti Vitrini — Devre dışı"), "{why}");
    assert!(codes(&gis).contains(&kentos_forms::codes::ACTION_UNKNOWN.to_owned()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn the_demo_drawings_form_pins_the_demo_extension_as_it_packs_now() {
    // samples/eklenti-vitrini/vitrin.kcad's Parsel form opens Vitrin's TAKS tool pinned to the package's digest
    // (docs/adr/8057 §12): a package changed without the drawing written again is said here, not in a demo.
    use kentos_commands::FormItem;
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/eklenti-vitrini/vitrin.kcad");
    let snapshot = kentos_kcad::decode(&std::fs::read(path).unwrap()).expect("the demo drawing");
    let form = snapshot
        .settings
        .forms
        .iter()
        .find(|f| f.id == "parsel")
        .expect("the Parsel form");
    let pin = form
        .draft
        .walk()
        .into_iter()
        .find_map(|(_, i)| match i {
            FormItem::Action {
                tool: Some(tool),
                pin,
                ..
            } if tool == "ext.tr.ornek.vitrin.taks" => pin.clone(),
            _ => None,
        })
        .expect("its TAKS button, pinned");
    let dir = temp("vitrin-cizim");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    assert_eq!(
        gis.tool_pin_drift("ext.tr.ornek.vitrin.taks", Some(&pin)),
        None,
        "vitrin.kcad's TAKS button is pinned to another package: run `.run/py/bin/python \
         samples/eklenti-vitrini/vitrin_cizim.py`"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_models_step_on_an_extensions_tool_is_pinned_and_says_where_it_stands() {
    // docs/adr/8057 §13, PY-TOOL-20: Model tasarımcısı adds Vitrin's TAKS pinned to the extension as installed; the
    // step's line says the version (another one too), pending without Python, unavailable once the extension is off.
    use crate::processing::designer::Event as Designer;
    use kentos_commands::ExtensionPin;
    const TOOL: &str = "ext.tr.ornek.vitrin.taks";
    const VITRIN: &str = "tr.ornek.vitrin";
    let dir = temp("model-sabitleme");
    // A drawing open (the designer asks for one), the extensions apart from the user's own.
    let mut gis = crate::processing::tests::app_with_parcels();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&dir, false).0;
    gis.extensions_changed();
    install(&mut gis, "vitrin.kek", vitrin_package());
    let _ = gis.update(Message::Run("processing.newModel"));
    let _ = gis.update(Message::Gis(crate::gis::Event::ModelDesigner(
        Designer::AddTool {
            tool: TOOL.into(),
            at: None,
        },
    )));
    let step = gis
        .gis
        .processing
        .designer
        .as_deref()
        .expect("the designer")
        .draft
        .steps[0]
        .clone();
    let pin = step.pin.clone().expect("pinned");
    assert_eq!(
        (pin.extension.as_str(), pin.version.as_str()),
        (VITRIN, vitrin_version().as_str())
    );
    assert_eq!(
        Some(pin.clone()),
        gis.tool_pin_of(TOOL),
        "the installed package's"
    );
    let note = gis.step_pin_note(TOOL, Some(&pin)).expect("a line");
    assert!(note.starts_with("Bekliyor: "), "{note}");
    gis.extensions.python_reasons.insert(VITRIN.into(), None);
    assert_eq!(
        gis.step_pin_note(TOOL, Some(&pin)).as_deref(),
        Some(format!("Eklenti Vitrini {} ile eklendi.", vitrin_version()).as_str())
    );
    let older = ExtensionPin {
        version: "1.0.0".into(),
        ..pin.clone()
    };
    let note = gis.step_pin_note(TOOL, Some(&older)).expect("a line");
    assert!(
        note.contains(&format!(
            "1.0.0 ile eklendi; kurulu sürüm {}",
            vitrin_version()
        )),
        "{note}"
    );
    assert_eq!(gis.step_pin_note("attributes.calculate", None), None);
    // The inspector draws it.
    gis.gis.processing.designer.as_deref_mut().unwrap().selected = Some(
        kentos_processing::model_edit::NodeRef::Step(step.id.clone()),
    );
    let _ = gis.model_designer_view();
    // Turned off: unavailable with the extension's state.
    let _ = gis.extensions_event(Event::Disable(VITRIN.into()));
    let note = gis.step_pin_note(TOOL, Some(&pin)).expect("a line");
    assert!(
        note.starts_with("Kullanılamaz: Eklenti Vitrini — Devre dışı"),
        "{note}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_extension_runs_another_extensions_tool_only_when_it_depends_on_it() {
    // PY-RUN-15 (docs/adr/8057 §16): through the broker an extension's code may run its own tools and those of the
    // extensions its manifest depends on; another's is refused with what to write.
    let dir = temp("eklentiden-eklentiye");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let run = |tool: &str| {
        serde_json::json!({
            "command": "cad.processing.run",
            "input": { "tool": tool, "parameters": {} },
            "op": "execute",
        })
    };
    let taks = run("ext.tr.ornek.vitrin.taks");
    // MEBİS without the dependency: refused, saying where to declare it.
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] = serde_json::json!([]);
        }),
    );
    let why = gis.calls_refused("tr.ornek.mebis", &taks).expect("refused");
    assert!(
        why.contains("“tr.ornek.vitrin”") && why.contains("dependencies.extensions"),
        "{why}"
    );
    // Its own tools and calls that run no extension's tool pass.
    assert_eq!(gis.calls_refused("tr.ornek.vitrin", &taks), None);
    assert_eq!(
        gis.calls_refused("tr.ornek.mebis", &run("attributes.calculate")),
        None
    );
    let entities = serde_json::json!({ "command": "cad.entities.set", "input": {} });
    assert_eq!(gis.calls_refused("tr.ornek.mebis", &entities), None);
    // With the dependency declared (the registry keeps Vitrin installed, enabled and of a fitting version): it runs.
    let _ = gis.extensions_event(Event::Remove("tr.ornek.mebis".into()));
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] =
                serde_json::json!([{ "id": "tr.ornek.vitrin", "version": ">=1.0" }]);
        }),
    );
    assert_eq!(gis.calls_refused("tr.ornek.mebis", &taks), None);
    let batch = serde_json::json!({
        "command": "cad.processing.batch",
        "input": { "tool": "ext.tr.ornek.vitrin.taks", "rows": [] },
    });
    assert_eq!(gis.calls_refused("tr.ornek.mebis", &batch), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn an_audit_rules_findings_are_listed_shown_and_fixed_as_one_step() {
    // docs/adr/8057 §17 (plan ADIM 20): the rule's answer read into Eklenti denetimi; Göster selects the object, Düzelt
    // writes the rule's fix as one undo step named by it; a fix of an extension not allowed to write is refused.
    use super::audits::{AuditDone, AuditRead, AuditState, Severity, findings_of};
    const VITRIN: &str = "tr.ornek.vitrin";
    let dir = temp("denetim");
    let mut gis = crate::processing::tests::app_with_parcels();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&dir, false).0;
    gis.extensions_changed();
    install(&mut gis, "vitrin.kek", vitrin_package());
    let doc = gis.document.as_ref().unwrap();
    let slot = doc
        .model
        .entities()
        .next()
        .map(|e| kentos_domain::Slot(e.base().id))
        .unwrap();
    let uid = doc.model.uid(slot).unwrap().to_string();
    let session = doc.session;
    let done = serde_json::json!({
        "ok": true,
        "result": { "kind": "findings", "findings": [
            { "severity": "warning", "message": "Yapının numarası yok; sıradaki numara Y-01.", "featureId": uid,
              "fix": { "title": "Y-01 yaz", "command": "cad.entities.set",
                       "args": { "uids": [uid], "operation": "attributes", "attrs": { "no": "Y-01" } } } },
            { "severity": "info", "message": "Çizimin yalnız ilk nesneleri denetlendi." },
            { "severity": "ölümcül", "message": "okunmaz" }
        ] }
    });
    let (findings, dropped) = findings_of(&done).expect("read");
    assert_eq!(
        (findings.len(), dropped),
        (2, 0),
        "an unknown severity is left out"
    );
    assert_eq!(findings[0].severity, Severity::Warning);
    let failed = findings_of(
        &serde_json::json!({ "ok": false, "exception": "TypeError", "message": "liste değil",
                                                 "error": "Traceback…" }),
    );
    assert_eq!(failed.unwrap_err().0, "TypeError: liste değil");
    // The run ends: its findings in the tab.
    gis.extensions.audit = Some(AuditState {
        ext: VITRIN.into(),
        local: "yapi-no".into(),
        title: "Yapı numarası".into(),
        running: Some(std::sync::Arc::new(std::sync::atomic::AtomicBool::new(
            false,
        ))),
        findings: Vec::new(),
        dropped: 0,
        cut: false,
        ms: 0,
        error: None,
        session,
        version: vitrin_version(),
        digest: gis.extensions.registry.as_ref().unwrap().digest(VITRIN),
        revision: gis.document.as_ref().unwrap().model.revision(),
        drawing: None,
        facts: None,
        by: crate::works::By::You,
        started: std::time::Instant::now(),
    });
    let model = &gis.document.as_ref().unwrap().model;
    let facts = kentos_processing::audit_job::facts_of(model, &|| false).expect("the facts");
    let read = AuditRead {
        cut: false,
        drawing: kentos_processing::audit_job::drawing_stamp(model),
        facts: kentos_commands::audit_job::digest_of(&facts),
    };
    gis.ext_audit_done(AuditDone {
        ext: VITRIN.into(),
        local: "yapi-no".into(),
        answer: Ok(done),
        ms: 12,
        read: Some(read.clone()),
    });
    let state = gis.extensions.audit.as_ref().unwrap();
    assert!(state.running.is_none() && state.error.is_none());
    assert_eq!(state.findings.len(), 2);
    // What the rule read, as a cloud job's report says it (docs/adr/8057 §42).
    let report = gis.ext_audit_report(0);
    assert_eq!(
        (report["drawing"].as_str(), report["facts"].as_str()),
        (Some(read.drawing.as_str()), Some(read.facts.as_str()))
    );
    assert!(read.facts.starts_with("sha256:") && read.drawing.len() == 16);
    let _ = gis.ext_audit_tab();
    // Its provenance (plan ADIM 20's köken): the rule's version and package, the drawing's revision; nothing changed.
    let (words, changed) = gis.ext_audit_provenance().unwrap();
    assert!(
        words.starts_with(&format!(
            "Köken: Eklenti Vitrini {} · yapi-no · paket ",
            vitrin_version()
        )) && words.contains(" · çizim revizyonu "),
        "{words}"
    );
    assert_eq!(changed, None);
    // Göster: the object selected.
    let _ = gis.ext_audit_show(0);
    assert_eq!(gis.selection.ids(), [slot]);
    // Düzelt: one undo step named by the fix; undone, the attribute goes.
    assert_eq!(gis.ext_audit_fix_refused(0), None);
    gis.ext_audit_fix(0);
    let attr = |gis: &App| {
        gis.document
            .as_ref()
            .unwrap()
            .model
            .get(slot)
            .unwrap()
            .base()
            .attrs
            .get("no")
            .cloned()
    };
    assert_eq!(attr(&gis).as_deref(), Some("Y-01"));
    assert!(gis.extensions.audit.as_ref().unwrap().findings[0].fixed);
    assert!(
        gis.ext_audit_provenance()
            .and_then(|(_, c)| c)
            .is_some_and(|c| c.starts_with("Çizim denetimden sonra değişti")),
        "a fix changed the drawing: said"
    );
    assert_eq!(gis.ext_audit_fix_refused(0).as_deref(), Some("Düzeltildi."));
    let undone = gis.document.as_mut().unwrap().model.undo();
    assert_eq!(undone.as_deref(), Some("Y-01 yaz"));
    assert_eq!(attr(&gis), None);
    // An extension not allowed to write: its fix is refused with why.
    gis.extensions.audit.as_mut().unwrap().findings[0].fixed = false;
    gis.extensions.audit.as_mut().unwrap().ext = "tr.ornek.yok".into();
    let why = gis.ext_audit_fix_refused(0).expect("refused");
    assert!(why.contains("document.write"), "{why}");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn a_blueprint_installs_with_every_contribution_and_passes_the_desktops_own_checks() {
    // docs/adr/8057 §19: what `extension.blueprint` gives packs, passes the desktop's review (its tool check
    // included) and installs, its tab holding each example.
    use kentos_extensions::blueprint::{Ask, Contribution, blueprint};
    let files = blueprint(&Ask {
        id: "tr.belediye.mezarlik-bilgi".into(),
        name: "Mezarlık bilgi".into(),
        summary: "Mezarlık adalarını ve mezar yerlerini yönetir.".into(),
        vertical: true,
        module: kentos_extensions::manifest::Module::Cbs,
        contributions: Contribution::ALL.to_vec(),
    })
    .unwrap();
    let files: std::collections::BTreeMap<String, Vec<u8>> = files
        .into_iter()
        .map(|(k, v)| (k, v.into_bytes()))
        .collect();
    let bytes = kentos_extensions::pack(&files, &Limits::default())
        .unwrap_or_else(|p| panic!("{p:#?}"))
        .bytes;
    let dir = temp("iskelet");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "iskelet.kek", bytes);
    let tab = gis
        .extension_tabs()
        .find(|t| t.extension == "tr.belediye.mezarlik-bilgi")
        .expect("its tab");
    let labels: Vec<String> = tab
        .panels
        .iter()
        .flat_map(|p| p.items.iter().map(|i| i.label.clone()))
        .collect();
    assert_eq!(
        labels,
        [
            "Katman listesi",
            "Alan özeti",
            "Adsız nesneler",
            "Katmanları kur",
            "Alan çiz",
            "Nesne ara"
        ],
        "with a schema, its command; the map tool draws into its layer (docs/adr/8057 §23)"
    );
    assert!(
        gis.gis
            .processing
            .registry
            .get("ext.tr.belediye.mezarlik-bilgi.alan-ozeti")
            .is_some(),
        "its İşlemler tool is listed"
    );
    // Its portable form, bound to the schema's layer (docs/adr/8057 §112): a form among its parts.
    let catalog = gis.ext_catalog();
    assert!(
        catalog["extensions"][0]["contributions"]
            .as_array()
            .is_some_and(|parts| parts
                .iter()
                .any(|p| p["kind"] == "form" && p["id"] == "ext.tr.belediye.mezarlik-bilgi.alan")),
        "{catalog:#}"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/8057 §24: an extension's panel opens in the dock's Eklenti tab without Python; what is typed is kept; a
/// button whose extension cannot run its code says why in the panel; a run's rows fill the panel's table; closing it
/// leaves nothing.
#[test]
fn an_extensions_panel_opens_in_the_dock_and_its_table_takes_its_commands_rows() {
    use super::panels::{PanelEvent, PanelOrigin};

    let dir = temp("panel");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let tab = gis
        .extension_tabs()
        .find(|t| t.extension == "tr.ornek.vitrin")
        .expect("Vitrin's tab");
    let item = tab
        .panels
        .iter()
        .flat_map(|p| &p.items)
        .find(|i| i.label == "Yapı ara")
        .expect("its panel's button");
    assert!(
        matches!(&item.action, Action::OpenPanel(ext, local) if ext == "tr.ornek.vitrin" && local == "yapi-paneli"),
        "a panel opens without Python: {:?}",
        item.action
    );
    let _ = gis.extensions_event(Event::OpenPanel(
        "tr.ornek.vitrin".into(),
        "yapi-paneli".into(),
    ));
    assert_eq!(
        gis.extensions.panel().map(|p| p.definition.title.as_str()),
        Some("Yapı ara")
    );
    assert!(
        gis.docks.contains(crate::app::Panel::Extension),
        "shown in the dock"
    );
    let _ = gis.extensions_event(Event::Panel(PanelEvent::Text("sorgu".into(), "Y-0".into())));
    let _ = gis.extensions_event(Event::Panel(PanelEvent::Check("numarasiz".into(), true)));
    let panel = gis.extensions.panel().unwrap();
    assert_eq!(
        panel.params().unwrap(),
        serde_json::json!({ "sorgu": "Y-0", "katman": null, "numarasiz": true }),
        "no layer chosen: none"
    );
    // Its environment not prepared here: the button says why in the panel, nothing starts.
    gis.extensions
        .python_reasons
        .insert("tr.ornek.vitrin".into(), Some("Ortam hazırlanmadı.".into()));
    let _ = gis.extensions_event(Event::Panel(PanelEvent::Press("ara".into())));
    let panel = gis.extensions.panel().unwrap();
    assert_eq!(
        panel.said,
        Some((
            crate::exchange::words::Kind::Error,
            "Ortam hazırlanmadı.".to_owned()
        ))
    );
    assert!(panel.running.is_none() && gis.extensions.panel_calls.is_empty());
    // A run's answer: its rows into the table, its words under the panel; a panel since closed takes none.
    let origin = PanelOrigin {
        ext: "tr.ornek.vitrin".into(),
        panel: "yapi-paneli".into(),
        button: "ara".into(),
        table: Some("sonuc".into()),
        instance: gis.extensions.instance_of("tr.ornek.vitrin", "yapi-paneli"),
    };
    gis.ext_panel_answer(
        &origin,
        Ok(serde_json::json!({ "rows": [{ "no": "Y-01", "alan (m²)": 120.0 }], "columns": ["no", "alan (m²)"],
                               "message": "1 yapı bulundu." })),
    );
    let panel = gis.extensions.panel().unwrap();
    assert_eq!(panel.tables["sonuc"].1.len(), 1);
    assert_eq!(
        panel.said,
        Some((
            crate::exchange::words::Kind::Ok,
            "1 yapı bulundu.".to_owned()
        ))
    );
    let _ = gis.extension_panel_view();
    let _ = gis.extensions_event(Event::Panel(PanelEvent::Close(0)));
    assert!(gis.extensions.panel().is_none());
    gis.ext_panel_answer(&origin, Err("geç".into()));
    assert!(
        gis.extensions.panel().is_none(),
        "a late answer goes nowhere"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/8057 §24: Vitrin's Değer say on the demo drawing: its field picker lists the fields of the layer its layer
/// picker holds; its feature picker takes the selected objects of that layer when asked, says why when none fit and
/// keeps what it held; another layer clears both.
#[test]
fn a_panels_pickers_list_the_layers_fields_and_take_the_selection() {
    use super::panels::PanelEvent;

    let dir = temp("secici");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/eklenti-vitrini/vitrin.kcad");
    let snapshot = kentos_kcad::decode(&std::fs::read(path).unwrap()).expect("the demo drawing");
    gis.document = Some(crate::document::Document::from_v2(snapshot, None).expect("opens"));
    let _ = gis.extensions_event(Event::OpenPanel(
        "tr.ornek.vitrin".into(),
        "deger-paneli".into(),
    ));
    let names = |gis: &App, layer: Option<&str>| -> Vec<String> {
        gis.panel_fields(layer, &[])
            .into_iter()
            .map(|(name, _)| name)
            .collect()
    };
    assert_eq!(names(&gis, None), [""], "no layer: none only");
    let say = |gis: &mut App, e: PanelEvent| {
        let _ = gis.extensions_event(Event::Panel(e));
    };
    say(
        &mut gis,
        PanelEvent::Choose("katman".into(), "parsel".into()),
    );
    assert_eq!(names(&gis, Some("parsel")), ["", "ada", "parsel"]);
    assert_eq!(
        gis.panel_fields(Some("parsel"), &["integer".to_owned()])
            .len(),
        1,
        "of a kind it has none of: none only"
    );
    say(&mut gis, PanelEvent::Choose("alan".into(), "ada".into()));
    // Nothing selected: said, nothing taken.
    say(&mut gis, PanelEvent::Pick("nesneler".into()));
    let said = |gis: &App| gis.extensions.panel().unwrap().said.clone();
    assert_eq!(
        said(&gis),
        Some((
            crate::exchange::words::Kind::Error,
            "“Nesneler”: önce çizimde nesne seçin.".to_owned()
        ))
    );
    // Two parcels and a building selected: the parcels taken.
    let doc = gis.document.as_ref().unwrap();
    let slots = |layer: &str| -> Vec<kentos_domain::Slot> {
        doc.model
            .by_layer(layer)
            .map(|e| kentos_domain::Slot(e.base().id))
            .collect()
    };
    let (parcels, buildings) = (slots("parsel"), slots("yapi"));
    let uids: Vec<String> = parcels[..2]
        .iter()
        .map(|&s| doc.model.uid(s).unwrap().to_string())
        .collect();
    gis.selection.set([parcels[0], parcels[1], buildings[0]]);
    say(&mut gis, PanelEvent::Pick("nesneler".into()));
    let panel = gis.extensions.panel().unwrap();
    assert_eq!(
        panel.params().unwrap(),
        serde_json::json!({ "katman": "parsel", "alan": "ada", "nesneler": uids, "vurgula": "-" })
    );
    // Taking them runs its onChange command (Vitrin 1.0.13): Python is off in a test, so the panel says why.
    assert!(
        matches!(&panel.said, Some((_, why)) if why.starts_with("Python kapalı")),
        "{:?}",
        panel.said
    );
    // Only a building: none fits, said with the layer's name; the parcels stay.
    gis.selection.set([buildings[0]]);
    say(&mut gis, PanelEvent::Pick("nesneler".into()));
    assert_eq!(
        said(&gis),
        Some((
            crate::exchange::words::Kind::Error,
            "“Nesneler”: seçili 1 nesneden hiçbiri “Parsel sınırı” katmanından değil.".to_owned()
        ))
    );
    assert_eq!(gis.extensions.panel().unwrap().picks["nesneler"].len(), 2);
    // Another layer: the field and the objects chosen from the one before go.
    say(&mut gis, PanelEvent::Choose("katman".into(), "yapi".into()));
    assert_eq!(
        gis.extensions.panel().unwrap().params().unwrap(),
        serde_json::json!({ "katman": "yapi", "alan": null, "nesneler": [], "vurgula": "-" })
    );
    let _ = gis.extension_panel_view();
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/8057 §24: a command's ctx.progress shows in its running notice (kept and never drawn until 14 November
/// 2026) and, when a panel's button started it, in the panel with Durdur; its message leaves the panel's table as it
/// was.
#[test]
fn a_commands_progress_shows_in_its_notice_and_its_panel() {
    use super::panels::{PanelCall, PanelOrigin};
    use crate::python::host::Said;
    use serde_json::json;

    const VITRIN: &str = "tr.ornek.vitrin";
    assert_eq!(
        super::runner::progress_words(&(2.0, Some(5.0), Some("3. yapı numaralanıyor".into()))),
        (Some(0.4), "%40 · 3. yapı numaralanıyor".to_owned())
    );
    assert_eq!(
        super::runner::progress_words(&(7.0, None, None)),
        (None, "7 adım".to_owned()),
        "no total: its steps, the bar moving"
    );
    let dir = temp("ilerleme");
    let mut gis = crate::files_testing::app_with_drawing();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&dir, false).0;
    gis.extensions_changed();
    install(&mut gis, "vitrin.kek", vitrin_package());
    let _ = gis.extensions_event(Event::OpenPanel(VITRIN.into(), "yapi-paneli".into()));
    let origin = PanelOrigin {
        ext: VITRIN.into(),
        panel: "yapi-paneli".into(),
        button: "numarala".into(),
        table: Some("sonuc".into()),
        instance: gis.extensions.instance_of(VITRIN, "yapi-paneli"),
    };
    // The search's rows, then its Yapıları numarala button pressed (the launch the background step hands over).
    gis.ext_panel_answer(
        &origin,
        Ok(json!({ "rows": [{ "no": "—" }], "columns": ["no"], "message": "1 yapı bulundu." })),
    );
    gis.extensions.panel_mut().unwrap().running = Some("numarala".into());
    gis.extensions.panel_calls.push(PanelCall {
        ext: VITRIN.into(),
        command: "yapilari-numarala".into(),
        params: json!({}),
        origin: origin.clone(),
    });
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(VITRIN))
        .unwrap();
    let launch = super::runner::Launch::for_test(
        "/bin/sh".into(),
        package,
        "vitrin.logic:yapilari_numarala",
        "Yapıları numarala",
        true,
    );
    let _ = gis.extensions_event(Event::Launch(
        VITRIN.into(),
        "command".into(),
        "yapilari-numarala".into(),
        Ok(launch),
    ));
    let runner = &gis.extensions.runners[VITRIN];
    let run = runner.run.as_ref().expect("running");
    assert_eq!(run.origin.as_ref(), Some(&origin), "started from the panel");
    let (generation, id, notice) = (runner.generation, run.id, run.notice.expect("its notice"));
    let _ = gis.extension_panel_view();
    let _ = gis.extensions_event(Event::Runner(
        VITRIN.into(),
        generation,
        Said::Message(json!({ "type": "progress", "id": id, "done": 2, "total": 5,
                             "text": "3. yapı numaralanıyor" })),
    ));
    let toast = gis
        .toasts
        .iter()
        .find(|(t, _, _)| *t == notice)
        .map(|(_, t, _)| t)
        .expect("its notice stays");
    assert_eq!(toast.description(), Some("%40 · 3. yapı numaralanıyor"));
    assert_eq!(toast.progress_value(), Some(Some(0.4)));
    let _ = gis.extension_panel_view();
    // Its message: said under the panel; the search's rows stay.
    let _ = gis.extensions_event(Event::Runner(
        VITRIN.into(),
        generation,
        Said::Message(json!({ "type": "done", "id": id, "ok": true,
                             "result": { "kind": "message", "message": "5 yapı numaralandı." } })),
    ));
    let panel = gis.extensions.panel().unwrap();
    assert_eq!(
        panel.said,
        Some((
            crate::exchange::words::Kind::Ok,
            "5 yapı numaralandı.".to_owned()
        ))
    );
    assert_eq!(
        panel.tables["sonuc"].1.len(),
        1,
        "a message leaves the table"
    );
    assert!(panel.running.is_none());
    // Durdur: said as the user's choice, not in red.
    gis.ext_panel_stopped(&origin);
    assert_eq!(
        gis.extensions.panel().unwrap().said,
        Some((
            crate::exchange::words::Kind::Info,
            "Durduruldu; çizime yazdıkları geri alındı.".to_owned()
        ))
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/8057 §24: a panel opens on the side its manifest asks for: Vitrin's Değer say on the left, Yapı ara on the
/// right beside Katmanlar; the tab KentOS docked follows the next panel's side, the one the user moved stays.
#[test]
fn a_panel_opens_on_its_side_and_a_moved_tab_stays() {
    use crate::app::Panel;
    use kentos_ui::widget::docking::{Event as Dock, Side, Slot, Target};

    let dir = temp("kenar");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let side = |gis: &App| match gis.docks.slot(Panel::Extension) {
        Some(Slot::Docked(side, _)) => Some(side),
        _ => None,
    };
    let open = |gis: &mut App, local: &str| {
        let _ = gis.extensions_event(Event::OpenPanel("tr.ornek.vitrin".into(), local.into()));
    };
    open(&mut gis, "deger-paneli");
    assert_eq!(side(&gis), Some(Side::Left), "Değer say asks for the left");
    open(&mut gis, "yapi-paneli");
    assert_eq!(side(&gis), Some(Side::Right));
    assert_eq!(
        gis.docks
            .slot(Panel::Extension)
            .zip(gis.docks.slot(Panel::Layers))
            .map(|(e, l)| e == l),
        Some(true),
        "beside Katmanlar"
    );
    // The user puts it under the map: it stays there for the next panel.
    gis.docks
        .update(Dock::Moved(Panel::Extension, Target::Edge(Side::Bottom)));
    assert_eq!(side(&gis), Some(Side::Bottom));
    open(&mut gis, "deger-paneli");
    assert_eq!(side(&gis), Some(Side::Bottom), "the user's move is kept");
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/8057 §24: Vitrin's Parsel kartı: its feature picker's command answers a row per parcel, the panel's card
/// shows a card each (the panel has no table: a change's rows go to its card); a message keeps the cards, empty rows
/// clear them.
#[test]
fn a_panels_card_shows_a_card_per_row() {
    use super::panels::PanelOrigin;
    use serde_json::json;

    const VITRIN: &str = "tr.ornek.vitrin";
    let dir = temp("kart");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let _ = gis.extensions_event(Event::OpenPanel(VITRIN.into(), "parsel-paneli".into()));
    assert_eq!(
        gis.extensions.panel().map(|p| p.definition.title.as_str()),
        Some("Parsel kartı")
    );
    let _ = gis.extension_panel_view();
    let origin = PanelOrigin {
        ext: VITRIN.into(),
        panel: "parsel-paneli".into(),
        button: "parseller".into(),
        table: Some("kartlar".into()),
        instance: gis.extensions.instance_of(VITRIN, "parsel-paneli"),
    };
    gis.ext_panel_answer(
        &origin,
        Ok(json!({ "rows": [
            { "uid": "6f1c2a52-0000-4000-8000-000000000001", "parsel": "101/1", "durum": "uygun", "alan (m²)": 400.0,
              "yapı": 2, "taban alanı (m²)": 112.0, "TAKS": 0.28 },
            { "parsel": "101/2", "durum": "TAKS aşıyor", "alan (m²)": 400.0, "yapı": 1, "taban alanı (m²)": 180.0,
              "TAKS": 0.45 }
        ], "message": "2 parselin kartı." })),
    );
    assert_eq!(gis.extensions.panel().unwrap().tables["kartlar"].1.len(), 2);
    let _ = gis.extension_panel_view();
    gis.ext_panel_answer(
        &origin,
        Ok(json!({ "kind": "message", "message": "Açık çizim yok." })),
    );
    assert_eq!(
        gis.extensions.panel().unwrap().tables["kartlar"].1.len(),
        2,
        "a message keeps them"
    );
    gis.ext_panel_answer(
        &origin,
        Ok(json!({ "rows": [], "message": "Parselleri çizimde seçip Seçimi al'a basın." })),
    );
    assert!(
        gis.extensions.panel().unwrap().tables["kartlar"]
            .1
            .is_empty(),
        "empty rows clear them"
    );
    let _ = gis.extension_panel_view();
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/8057 §24: several panels stay open in the Eklenti tab, a tab each, of one extension or more: a panel opened
/// again is shown, what was typed in each kept; a run answers its panel while another is shown, its tab marking it
/// until shown; a second panel's call that finds its extension running says so (its button stayed pressed); closing
/// the panel shown shows the next; a disabled extension's panels go, the others' stay.
#[test]
fn several_panels_stay_open_in_tabs_each_with_its_own() {
    use super::panels::{PanelCall, PanelEvent, PanelOrigin};
    use crate::app::Panel;
    use serde_json::json;

    const VITRIN: &str = "tr.ornek.vitrin";
    const MEBIS: &str = "tr.ornek.mebis";
    let dir = temp("paneller");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m["dependencies"]["extensions"] = json!([]);
        }),
    );
    let open = |gis: &mut App, ext: &str, local: &str| {
        let _ = gis.extensions_event(Event::OpenPanel(ext.into(), local.into()));
    };
    let say = |gis: &mut App, e: PanelEvent| {
        let _ = gis.extensions_event(Event::Panel(e));
    };
    let shown = |gis: &App| gis.extensions.panel().map(|p| p.local.clone());
    let origin = |gis: &App, ext: &str, panel: &str, button: &str| PanelOrigin {
        ext: ext.into(),
        panel: panel.into(),
        button: button.into(),
        table: None,
        instance: gis.extensions.instance_of(ext, panel),
    };
    open(&mut gis, VITRIN, "yapi-paneli");
    say(&mut gis, PanelEvent::Text("sorgu".into(), "Y-0".into()));
    open(&mut gis, VITRIN, "deger-paneli");
    open(&mut gis, MEBIS, "search-dock");
    assert_eq!(
        gis.extensions
            .panels
            .iter()
            .map(|p| p.definition.title.as_str())
            .collect::<Vec<_>>(),
        ["Yapı ara", "Değer say", "Mezar sorgulama"]
    );
    assert_eq!(
        shown(&gis).as_deref(),
        Some("search-dock"),
        "the one opened last"
    );
    let _ = gis.extension_panel_view();
    // Yapı ara opened again: shown, not opened twice, what was typed in it kept.
    open(&mut gis, VITRIN, "yapi-paneli");
    assert_eq!(gis.extensions.panels.len(), 3);
    assert_eq!(gis.extensions.panel().unwrap().texts["sorgu"], "Y-0");
    // Its search answers while MEBİS's panel is shown: its rows go to it, its tab marks it until it is shown.
    say(&mut gis, PanelEvent::Activate(2));
    gis.ext_panel_answer(
        &PanelOrigin {
            table: Some("sonuc".into()),
            ..origin(&gis, VITRIN, "yapi-paneli", "ara")
        },
        Ok(json!({ "rows": [{ "no": "Y-01" }], "columns": ["no"] })),
    );
    let yapi = &gis.extensions.panels[0];
    assert!(yapi.unseen && yapi.tables["sonuc"].1.len() == 1);
    assert_eq!(shown(&gis).as_deref(), Some("search-dock"), "nothing jumps");
    let _ = gis.extension_panel_view();
    say(&mut gis, PanelEvent::Activate(0));
    assert!(!gis.extensions.panels[0].unseen, "seen once shown");
    // Where each was scrolled is its own: shown again, it is given back.
    say(&mut gis, PanelEvent::Scrolled(120.0));
    say(&mut gis, PanelEvent::Activate(1));
    assert_eq!(gis.extensions.panels[1].scroll, 0.0);
    say(&mut gis, PanelEvent::Activate(0));
    assert_eq!(gis.extensions.panels[0].scroll, 120.0);
    // A run of Vitrin's going (Yapıları numarala) when Değer say's call is launched: Değer say says so, not stuck.
    gis.extensions.panels[0].running = Some("numarala".into());
    gis.extensions.panel_calls.push(PanelCall {
        ext: VITRIN.into(),
        command: "yapilari-numarala".into(),
        params: json!({}),
        origin: origin(&gis, VITRIN, "yapi-paneli", "numarala"),
    });
    gis.extensions.panels[1].running = Some("say".into());
    gis.extensions.panel_calls.push(PanelCall {
        ext: VITRIN.into(),
        command: "deger-say".into(),
        params: json!({}),
        origin: origin(&gis, VITRIN, "deger-paneli", "say"),
    });
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(VITRIN))
        .unwrap();
    for (command, handler, title) in [
        (
            "yapilari-numarala",
            "vitrin.logic:yapilari_numarala",
            "Yapıları numarala",
        ),
        ("deger-say", "vitrin.logic:deger_say", "Değer say"),
    ] {
        let launch = super::runner::Launch::for_test(
            "/bin/sh".into(),
            package.clone(),
            handler,
            title,
            false,
        );
        let _ = gis.extensions_event(Event::Launch(
            VITRIN.into(),
            "command".into(),
            command.into(),
            Ok(launch),
        ));
    }
    let run = gis.extensions.runners[VITRIN]
        .run
        .as_ref()
        .expect("running");
    assert_eq!(
        run.origin.as_ref().map(|o| o.panel.as_str()),
        Some("yapi-paneli")
    );
    let deger = &gis.extensions.panels[1];
    assert!(deger.running.is_none(), "its button pressable again");
    assert_eq!(
        deger.said,
        Some((
            crate::exchange::words::Kind::Error,
            "“Yapıları numarala” sürüyor: bitmesini bekleyin ya da Durdur'a basın.".to_owned()
        ))
    );
    assert!(gis.extensions.panel_calls.is_empty());
    let _ = gis.extension_panel_view();
    // Tabs moved: the panel shown stays shown. Closing it shows the next, else the one before.
    say(&mut gis, PanelEvent::Move(0, 2));
    assert_eq!(gis.extensions.active_panel, 2);
    assert_eq!(shown(&gis).as_deref(), Some("yapi-paneli"));
    say(&mut gis, PanelEvent::Activate(0));
    say(&mut gis, PanelEvent::Close(0));
    assert_eq!(shown(&gis).as_deref(), Some("search-dock"), "the next");
    say(&mut gis, PanelEvent::Activate(1));
    say(&mut gis, PanelEvent::Close(1));
    assert_eq!(
        shown(&gis).as_deref(),
        Some("search-dock"),
        "the one before"
    );
    // Vitrin disabled: its panels go, MEBİS's stays, with the dock's tab; MEBİS's too: the tab goes.
    open(&mut gis, VITRIN, "deger-paneli");
    let _ = gis.extensions_event(Event::Disable(VITRIN.into()));
    assert_eq!(shown(&gis).as_deref(), Some("search-dock"));
    assert_eq!(gis.extensions.panels.len(), 1);
    assert!(gis.docks.contains(Panel::Extension));
    let _ = gis.extensions_event(Event::Disable(MEBIS.into()));
    assert!(gis.extensions.panels.is_empty() && !gis.docks.contains(Panel::Extension));
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/8057 §24: a panel opened gets room: its stack takes at least three fifths of its side (it had a third
/// under Öznitelikler in a short window and was used by scrolling); a stack the user made taller keeps its share.
#[test]
fn an_opened_panels_stack_takes_three_fifths_of_its_side() {
    use crate::app::Panel;
    use kentos_ui::widget::docking::{Event as Dock, Slot};

    let dir = temp("pay");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let share = |gis: &App| -> (usize, f32) {
        let Some(Slot::Docked(side, index)) = gis.docks.slot(Panel::Extension) else {
            panic!("docked");
        };
        let stacks = gis.docks.stacks(side);
        let total: f32 = stacks
            .iter()
            .filter(|s| !s.collapsed)
            .map(|s| s.weight)
            .sum();
        (stacks.len(), stacks[index].weight / total)
    };
    let open = |gis: &mut App| {
        let _ = gis.extensions_event(Event::OpenPanel(
            "tr.ornek.vitrin".into(),
            "yapi-paneli".into(),
        ));
    };
    open(&mut gis);
    let (stacks, at) = share(&gis);
    assert!(stacks > 1, "beside Katmanlar, above Öznitelikler");
    assert!((at - 0.6).abs() < 1e-3, "{at}");
    // The user makes it taller (the sash between the stacks): opening a panel again keeps that.
    let Some(Slot::Docked(side, index)) = gis.docks.slot(Panel::Extension) else {
        unreachable!()
    };
    let mut weights: Vec<f32> = gis.docks.stacks(side).iter().map(|s| s.weight).collect();
    let others: f32 = weights
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != index)
        .map(|(_, w)| w)
        .sum();
    weights[index] = 4.0 * others;
    gis.docks.update(Dock::Shared(side, weights));
    open(&mut gis);
    assert!((share(&gis).1 - 0.8).abs() < 1e-3, "kept");
    let _ = std::fs::remove_dir_all(&dir);
}

/// plan/eklenti.md, Referans eklenti: an extension disabled leaves nothing behind: its open panel and its dock tab go,
/// a fix its audit offered is refused (both stayed: the panel's buttons pressable, Düzelt writing for a stopped
/// extension).
#[test]
fn a_disabled_extensions_panel_and_audit_fixes_go_with_it() {
    use super::audits::{AuditState, Finding, Fix, Severity};
    use crate::app::Panel;

    const VITRIN: &str = "tr.ornek.vitrin";
    let dir = temp("hayalet-panel");
    let mut gis = crate::files_testing::app_with_drawing();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&dir, false).0;
    gis.extensions_changed();
    install(&mut gis, "vitrin.kek", vitrin_package());
    let _ = gis.extensions_event(Event::OpenPanel(VITRIN.into(), "yapi-paneli".into()));
    assert!(gis.extensions.panel().is_some() && gis.docks.contains(Panel::Extension));
    let session = gis.document.as_ref().unwrap().session;
    gis.extensions.audit = Some(AuditState {
        ext: VITRIN.into(),
        local: "yapi-no".into(),
        title: "Yapı numarası".into(),
        running: None,
        findings: vec![Finding {
            severity: Severity::Warning,
            message: "Yapı numarasız".into(),
            feature: None,
            fix: Some(Fix {
                title: "Numara ver".into(),
                command: "cad.entities.set".into(),
                input: serde_json::json!({ "uids": [], "operation": "attributes", "attrs": { "no": "Y-99" } }),
            }),
            fixed: false,
        }],
        dropped: 0,
        cut: false,
        ms: 1,
        error: None,
        session,
        version: vitrin_version(),
        digest: None,
        revision: 0,
        drawing: None,
        facts: None,
        by: crate::works::By::You,
        started: std::time::Instant::now(),
    });
    assert_eq!(
        gis.ext_audit_fix_refused(0),
        None,
        "running: its fix applies"
    );
    let _ = gis.extensions_event(Event::Disable(VITRIN.into()));
    assert!(gis.extensions.panel().is_none(), "its panel went with it");
    assert!(gis.extensions.panel_calls.is_empty());
    assert!(!gis.docks.contains(Panel::Extension), "its tab too");
    assert_eq!(
        gis.ext_audit_fix_refused(0).as_deref(),
        Some("Eklenti Vitrini kapalı: düzeltmesi uygulanmaz; Eklentiler'den etkinleştirin."),
        "its findings stay to read, its fixes do not apply"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/8057 §24: an input's onChange runs its command with the panel's values (Vitrin's Değer say recounts when a
/// field is chosen); a change while a command runs waits for it, one rerun; an input without onChange runs nothing.
#[test]
fn an_inputs_change_runs_its_command_and_waits_for_a_running_one() {
    use super::panels::PanelEvent;

    const VITRIN: &str = "tr.ornek.vitrin";
    let dir = temp("degisince");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let path =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/eklenti-vitrini/vitrin.kcad");
    let snapshot = kentos_kcad::decode(&std::fs::read(path).unwrap()).expect("the demo drawing");
    gis.document = Some(crate::document::Document::from_v2(snapshot, None).expect("opens"));
    let _ = gis.extensions_event(Event::OpenPanel(VITRIN.into(), "deger-paneli".into()));
    gis.extensions.python_reasons.insert(VITRIN.into(), None);
    let tried = |gis: &App| {
        gis.toasts
            .iter()
            .filter(|(_, t, _)| t.title().ends_with("çalıştırılamadı"))
            .count()
    };
    let say = |gis: &mut App, e: PanelEvent| {
        let _ = gis.extensions_event(Event::Panel(e));
    };
    // The layer picker names no command: nothing runs.
    say(
        &mut gis,
        PanelEvent::Choose("katman".into(), "parsel".into()),
    );
    assert_eq!(tried(&gis), 0);
    // A command running: the field's change waits, and runs once when it ends.
    gis.extensions.panel_mut().unwrap().running = Some("say".into());
    say(&mut gis, PanelEvent::Choose("alan".into(), "ada".into()));
    assert_eq!(
        gis.extensions.panel().unwrap().rerun.as_deref(),
        Some("alan")
    );
    assert_eq!(tried(&gis), 0, "nothing started meanwhile");
    let _ = gis.ext_panel_rerun();
    assert_eq!(tried(&gis), 0, "still running: it waits");
    gis.extensions.panel_mut().unwrap().running = None;
    let _ = gis.ext_panel_rerun();
    // Its command started (no environment in a test: it says it cannot run, in the panel and a notice).
    assert_eq!(tried(&gis), 1, "the rerun started its command");
    let panel = gis.extensions.panel().unwrap();
    assert!(panel.rerun.is_none() && panel.running.is_none());
    assert!(matches!(
        panel.said,
        Some((crate::exchange::words::Kind::Error, _))
    ));
    // A field chosen with nothing running: its command starts at once.
    say(&mut gis, PanelEvent::Choose("alan".into(), "parsel".into()));
    assert_eq!(tried(&gis), 1, "the same notice, counted");
    assert!(
        gis.toasts
            .iter()
            .any(|(_, t, n)| t.title().ends_with("çalıştırılamadı") && n == 2),
        "tried again"
    );
    // Durdur drops a change that waited: the user stopped it.
    gis.extensions.panel_mut().unwrap().running = Some("say".into());
    say(&mut gis, PanelEvent::Choose("alan".into(), "ada".into()));
    let origin = super::panels::PanelOrigin {
        ext: VITRIN.into(),
        panel: "deger-paneli".into(),
        button: "say".into(),
        table: Some("dagilim".into()),
        instance: gis.extensions.instance_of(VITRIN, "deger-paneli"),
    };
    gis.ext_panel_stopped(&origin);
    assert!(gis.extensions.panel().unwrap().rerun.is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/8057 §24: an item's `when` is checked as the window is drawn: MEBİS's grave tool waits for its layers
/// (`layerSchema: mezar_parsel`) and says so; once a layer of that id is in the drawing it can be pressed. Its İşlemler
/// tool's button reads the tool's own label.
#[test]
fn an_items_when_holds_it_back_until_its_layer_is_there() {
    let root = temp("kosul");
    let mut gis = crate::files_testing::app_with_drawing();
    gis.program = Some(Module::Gis);
    gis.extensions = Extensions::open(&root, false).0;
    gis.extensions_changed();
    // Without its dependency on a base extension the fixtures do not carry: it activates here.
    install(
        &mut gis,
        "mebis.kek",
        package("mebis", |m| {
            m.as_object_mut().unwrap().remove("dependencies");
        }),
    );
    let tab = gis
        .extension_tabs()
        .find(|t| t.extension == "tr.ornek.mebis")
        .expect("MEBİS's tab");
    let grave = tab
        .panels
        .iter()
        .flat_map(|p| &p.items)
        .find(|i| i.label == "Tekli mezar")
        .expect("the grave tool")
        .clone();
    // An İşlemler tool's button reads its descriptor's label, not its id (real window, 14 November 2026).
    let labels: Vec<&str> = tab
        .panels
        .iter()
        .flat_map(|p| &p.items)
        .map(|i| i.label.as_str())
        .collect();
    assert!(labels.contains(&"Ada bazlı doluluk"), "{labels:?}");
    let why = gis.item_when_why(&grave).expect("held back");
    assert!(why.contains("mezar_parsel"), "{why}");
    assert!(gis.item_on(&grave).is_none());
    let doc = gis.document.as_mut().expect("a drawing");
    let added = kentos_native_application::registry::run(
        &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
        "cad.layers.edit",
        kentos_commands::CallOp::Execute,
        serde_json::json!({ "operation": "add", "id": "mezar_parsel", "name": "Mezar parseli" }),
    )
    .expect("a known command");
    assert_eq!(added["status"], "completed", "{added}");
    assert_eq!(gis.item_when_why(&grave), None, "its layer is there");
    let _ = std::fs::remove_dir_all(&root);
}

/// docs/adr/8057 §22, plan/eklenti.md ADIM 21's hot_reload and preview_render: a workshop's extension, reviewed from
/// its folder and installed by the user, takes the agent's changes without the app closing (the same version, a new
/// lease) with the capabilities the user granted; one asking for more goes back to the review; one not installed is
/// refused. The preview tells its tab, panels and tools as the app lays them out.
#[test]
fn a_workshop_extension_reloads_without_closing_and_its_preview_says_what_the_user_sees() {
    use serde_json::{Value, json};

    const VITRIN: &str = "tr.ornek.vitrin";
    let base = temp("atolye");
    let root = base.join("eklentiler");
    std::fs::create_dir_all(&root).unwrap();
    let mut gis = app(&root, Some(Module::Gis));
    let workshop = base.join("eklenti-atolyesi");
    let folder = workshop.join(VITRIN);
    // Vitrin's source, as the agent would have written it to its workshop.
    let source =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/eklenti-vitrini/eklenti");
    for (path, bytes) in kentos_extension_host::store::read_files(&source).unwrap() {
        if path.contains("__pycache__") {
            continue;
        }
        let to = folder.join(&path);
        std::fs::create_dir_all(to.parent().unwrap()).unwrap();
        std::fs::write(to, bytes).unwrap();
    }
    assert!(
        gis.ext_workshop_reload(VITRIN)
            .unwrap_err()
            .contains("kurulu değil"),
        "not installed: the user's review first"
    );
    // The user's review, from the folder; Kur.
    let pack = |folder: &Path| {
        let files = kentos_extension_host::store::read_files(folder).unwrap();
        kentos_extensions::pack(&files, &Limits::default())
            .unwrap_or_else(|p| panic!("{p:#?}"))
            .bytes
    };
    let package = workshop.join("tr.ornek.vitrin-paket.kek");
    std::fs::write(&package, pack(&folder)).unwrap();
    gis.review_workshop_package(&package, &folder, "Asistan");
    let _ = gis.extensions_event(Event::Install);
    assert!(gis.extensions.review.is_none(), "{:?}", gis.extensions.note);
    // The agent renames a panel and reloads: the same version, its new name on the ribbon.
    let edit = |file: &str, change: &dyn Fn(&mut Value)| {
        let path = folder.join(file);
        let mut value: Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
        change(&mut value);
        std::fs::write(&path, serde_json::to_vec_pretty(&value).unwrap()).unwrap();
    };
    edit("manifest.json", &|m| {
        m["entrypoints"]["panels"][0]["title"] = json!("Yapı bul");
    });
    edit("ui/yapi-ara.panel.json", &|p| {
        p["title"] = json!("Yapı bul")
    });
    let said = gis.ext_workshop_reload(VITRIN).expect("reloaded");
    assert_eq!(said["changed"], true, "{said:#}");
    assert_eq!(said["version"], vitrin_version().as_str());
    let labels: Vec<String> = gis
        .extension_tabs()
        .filter(|t| t.extension == VITRIN)
        .flat_map(|t| {
            t.panels
                .iter()
                .flat_map(|p| p.items.iter().map(|i| i.label.clone()))
        })
        .collect();
    assert!(labels.iter().any(|l| l == "Yapı bul"), "{labels:?}");
    assert_eq!(
        gis.ext_workshop_reload(VITRIN).unwrap()["changed"],
        false,
        "nothing changed since: said so"
    );
    // What the user sees, in words.
    let preview = gis.ext_preview(VITRIN).unwrap();
    assert_eq!(preview["source"], "geliştirici klasörü");
    let items: Vec<&Value> = preview["ribbonTab"]["groups"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|g| g["items"].as_array().unwrap())
        .collect();
    let bul = items
        .iter()
        .find(|i| i["label"] == "Yapı bul")
        .expect("its button");
    assert_eq!(bul["opens"]["panel"], "yapi-paneli");
    let panel = &preview["panels"][0];
    assert_eq!(panel["title"], "Yapı bul");
    assert!(
        panel["components"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["type"] == "button" && c["command"] == "yapi-ara"),
        "{panel:#}"
    );
    assert!(
        preview["contributions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|c| c["id"] == "ext.tr.ornek.vitrin.taks"),
        "{preview:#}"
    );
    // A capability the user did not grant: not loaded, back to the review; the installed one as it was.
    edit("manifest.json", &|m| {
        m["requestedCapabilities"].as_array_mut().unwrap().push(json!({
            "id": "datasets.read", "scope": "plugin-owned", "reason": "Kendi veri kümelerini okur."
        }));
        m["entrypoints"]["panels"][0]["title"] = json!("Yapı süz");
    });
    let refused = gis.ext_workshop_reload(VITRIN).unwrap_err();
    assert!(refused.contains("vermediği yetenek"), "{refused}");
    assert!(
        gis.extension_tabs()
            .flat_map(|t| t.panels.iter().flat_map(|p| p.items.iter()))
            .all(|i| i.label != "Yapı süz")
    );
    let _ = std::fs::remove_dir_all(&base);
}

/// docs/adr/8057 §32: a panel's command changes its own panel as it runs, through the broker: a value, a line under the
/// form, the cursor; a value the user typed meanwhile stays; words it said stay when its answer says none of its own.
/// A run not started from a panel has none; a closed panel takes nothing and is no mistake.
#[test]
fn a_panels_command_changes_its_panel_as_it_runs() {
    use super::panels::PanelEvent;
    use crate::python::host::Said;
    use serde_json::json;

    const VITRIN: &str = "tr.ornek.vitrin";
    let dir = temp("panel-oturumu");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let _ = gis.extensions_event(Event::OpenPanel(VITRIN.into(), "yapi-paneli".into()));
    // Ara pressed (Python is off in tests: the state its press leaves): its call waits for its launch.
    gis.extensions.panel_calls.push(super::panels::PanelCall {
        ext: VITRIN.into(),
        command: "yapi-ara".into(),
        params: json!({ "sorgu": "", "katman": null, "numarasiz": false }),
        origin: super::panels::PanelOrigin {
            ext: VITRIN.into(),
            panel: "yapi-paneli".into(),
            button: "ara".into(),
            table: Some("sonuc".into()),
            instance: gis.extensions.instance_of(VITRIN, "yapi-paneli"),
        },
    });
    gis.extensions.panels[0].running = Some("ara".into());
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(VITRIN))
        .unwrap();
    let launch = |handler: &str| {
        super::runner::Launch::for_test(
            "/bin/sh".into(),
            package.clone(),
            handler,
            "Yapı ara",
            false,
        )
    };
    let _ = gis.extensions_event(Event::Launch(
        VITRIN.into(),
        "command".into(),
        "yapi-ara".into(),
        Ok(launch("vitrin.logic:yapi_ara")),
    ));
    let runner = &gis.extensions.runners[VITRIN];
    let (generation, run) = (runner.generation, runner.run.as_ref().expect("running").id);
    let call = |gis: &mut App, id: u64, method: &str, params: serde_json::Value| {
        let _ = gis.extensions_event(Event::Runner(
            VITRIN.into(),
            generation,
            Said::Message(json!({ "type": "call", "id": id, "method": method, "params": params })),
        ));
    };
    // Through the broker: a value, words under the form, the cursor (left to do once the message is handled).
    call(
        &mut gis,
        1,
        "panel.set",
        json!({ "id": "sorgu", "value": "Y-02" }),
    );
    call(
        &mut gis,
        2,
        "panel.set",
        json!({ "id": "numarala", "enabled": false }),
    );
    call(
        &mut gis,
        3,
        "panel.say",
        json!({ "text": "Aranıyor…", "kind": "info" }),
    );
    call(&mut gis, 4, "panel.focus", json!({ "id": "sorgu" }));
    assert!(
        gis.extensions.later.is_empty(),
        "the focus went with the message's task"
    );
    let panel = gis.extensions.panel().unwrap();
    assert_eq!(panel.texts["sorgu"], "Y-02");
    assert!(panel.disabled.contains("numarala"));
    assert_eq!(
        panel.said,
        Some((crate::exchange::words::Kind::Info, "Aranıyor…".to_owned()))
    );
    let _ = gis.extension_panel_view();
    // The user types meanwhile: the code's value does not go over it.
    let _ = gis.extensions_event(Event::Panel(PanelEvent::Text("sorgu".into(), "Y-1".into())));
    let kept = gis.ext_panel_session(
        VITRIN,
        "panel.set",
        &json!({ "id": "sorgu", "value": "Y-03" }),
    );
    assert_eq!(kept.ok(), Some(json!({ "applied": false })));
    assert_eq!(gis.extensions.panel().unwrap().texts["sorgu"], "Y-1");
    // A wrong id is the code's mistake: refused, and said in its log (the code may catch the error and go on).
    let wrong = gis.ext_panel_session(VITRIN, "panel.set", &json!({ "id": "yok", "value": 1 }));
    assert!(
        matches!(&wrong, Err(e) if e.code == "unknown_component"),
        "{wrong:?}"
    );
    call(
        &mut gis,
        5,
        "panel.set",
        json!({ "panel": "yapi-paneli", "id": "yok", "value": 1 }),
    );
    let logged = gis
        .extensions
        .registry
        .as_ref()
        .unwrap()
        .log_of(VITRIN)
        .iter()
        .find(|r| r.code == "panel_refused")
        .cloned()
        .expect("its log line");
    assert_eq!(logged.part.as_deref(), Some("panel:yapi-paneli"));
    assert!(
        logged.message.starts_with("panel.set: “yok” panelde yok"),
        "{}",
        logged.message
    );
    // Its answer has rows and no words: they go to the table, the code's words stay.
    let _ = gis.extensions_event(Event::Runner(
        VITRIN.into(),
        generation,
        Said::Message(json!({ "type": "done", "id": run, "ok": true,
            "result": { "kind": "report", "rows": [{ "no": "Y-02" }], "columns": ["no"] } })),
    ));
    let panel = gis.extensions.panel().unwrap();
    assert_eq!(panel.tables["sonuc"].1.len(), 1);
    assert_eq!(
        panel.said,
        Some((crate::exchange::words::Kind::Info, "Aranıyor…".to_owned())),
        "its words stay"
    );
    assert!(panel.touched.is_empty() && panel.running.is_none());
    // A run not started from a panel: no panel to change.
    let _ = gis.extensions_event(Event::Launch(
        VITRIN.into(),
        "command".into(),
        "yapilari-numarala".into(),
        Ok(launch("vitrin.logic:yapilari_numarala")),
    ));
    let none = gis.ext_panel_session(VITRIN, "panel.say", &json!({ "text": "x" }));
    assert!(matches!(&none, Err(e) if e.code == "no_panel"), "{none:?}");
    // It opens one of its panels (ctx.open_panel): shown in front, held until the run ends; the user's choice there
    // stays the user's. A panel its manifest does not declare is the code's mistake.
    let opened = gis.ext_panel_session(VITRIN, "panel.open", &json!({ "panel": "deger-paneli" }));
    assert_eq!(opened.ok(), Some(json!({ "applied": true })));
    assert_eq!(
        gis.extensions.panel().map(|p| p.local.as_str()),
        Some("deger-paneli")
    );
    assert!(gis.extensions.panel().unwrap().held);
    let options = json!({ "panel": "deger-paneli", "id": "vurgula", "options": ["101", "102"], "value": "101" });
    let set = gis.ext_panel_session(VITRIN, "panel.set", &options);
    assert_eq!(set.ok(), Some(json!({ "applied": true })));
    assert_eq!(gis.extensions.panel().unwrap().choices["vurgula"], "101");
    let _ = gis.extensions_event(Event::Panel(PanelEvent::Choose(
        "vurgula".into(),
        "102".into(),
    )));
    let again = gis.ext_panel_session(
        VITRIN,
        "panel.set",
        &json!({ "panel": "deger-paneli", "id": "vurgula", "value": "101" }),
    );
    assert_eq!(
        again.ok(),
        Some(json!({ "applied": false })),
        "the user's choice stays"
    );
    let unknown = gis.ext_panel_session(VITRIN, "panel.open", &json!({ "panel": "yok-paneli" }));
    assert!(
        matches!(&unknown, Err(e) if e.code == "unknown_panel" && e.message.contains("yapi-paneli, deger-paneli")),
        "{unknown:?}"
    );
    // Its end: the panel is the user's again.
    let run = gis.extensions.runners[VITRIN].run.as_ref().unwrap().id;
    let _ = gis.extensions_event(Event::Runner(
        VITRIN.into(),
        generation,
        Said::Message(json!({ "type": "done", "id": run, "ok": true,
            "result": { "kind": "message", "message": "Bitti." } })),
    ));
    let panel = gis.extensions.panel().unwrap();
    assert!(!panel.held && panel.touched.is_empty());
    let _ = gis.extension_panel_view();
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/8057 §32: a layer menu's command is given the layer it was run on (`ctx.params["layer"]`); the same
/// command from elsewhere is not; a run that did not start leaves nothing for the next.
#[test]
fn a_layer_menus_command_gets_the_layer_it_was_run_on() {
    use serde_json::json;

    const VITRIN: &str = "tr.ornek.vitrin";
    let dir = temp("katman-menusu");
    let mut gis = app(&dir, Some(Module::Gis));
    install(&mut gis, "vitrin.kek", vitrin_package());
    let item = gis
        .extensions
        .menus
        .iter()
        .find(|m| m.item.label == "Değerlerini say")
        .expect("its layer menu item");
    assert_eq!(item.target, kentos_extensions::manifest::MenuTarget::Layer);
    // Python off in tests: the item says why it cannot run (with Python on it is the command's).
    assert!(
        matches!(&item.item.action, Action::Unavailable(why) if why.contains("Python kapalı")),
        "{:?}",
        item.item.action
    );
    let _ = gis.extension_menu(
        kentos_extensions::manifest::MenuTarget::Layer,
        Some("parsel"),
        kentos_ui::widget::Menu::new(),
    );
    // Run from a layer's menu; it did not start (no Python in tests): what it was run on goes with it.
    let _ = gis.extensions_event(Event::RunOn(
        VITRIN.into(),
        "command".into(),
        "katman-degerleri".into(),
        json!({ "layer": "parsel" }),
    ));
    assert!(
        gis.extensions.target_params.is_none(),
        "dropped with its run"
    );
    // Its launch takes it (the state the menu leaves while the environment is found).
    gis.extensions.target_params = Some((
        VITRIN.into(),
        "katman-degerleri".into(),
        json!({ "layer": "parsel" }),
    ));
    let package = gis
        .extensions
        .registry
        .as_ref()
        .and_then(|r| r.package_dir(VITRIN))
        .unwrap();
    let launch = || {
        super::runner::Launch::for_test(
            "/bin/sh".into(),
            package.clone(),
            "vitrin.logic:katman_degerleri",
            "Değerlerini say",
            false,
        )
    };
    let _ = gis.extensions_event(Event::Launch(
        VITRIN.into(),
        "command".into(),
        "katman-degerleri".into(),
        Ok(launch()),
    ));
    let sent = gis.extensions.test_last_invoke.clone().expect("its call");
    assert_eq!(sent["params"], json!({ "layer": "parsel" }), "{sent:#}");
    assert!(gis.extensions.target_params.is_none(), "taken");
    let _ = gis.ext_stop(VITRIN);
    gis.extensions.runners.clear();
    // From elsewhere (its ribbon or command search): no layer.
    let _ = gis.extensions_event(Event::Launch(
        VITRIN.into(),
        "command".into(),
        "katman-degerleri".into(),
        Ok(launch()),
    ));
    let sent = gis.extensions.test_last_invoke.clone().expect("its call");
    assert_eq!(sent["params"], json!({}), "{sent:#}");
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/8057 §46 (PY-RUN-14): Vitrin's Yapı tamponu runs KentOS's built-in Tampon through the broker, with the
/// command's own rights: in a run that may write, the five buildings' buffers land on a new layer in the run's undo
/// step; a run that may only read is refused the tool's write.
#[test]
fn an_extensions_command_runs_a_builtin_tool_with_its_own_rights() {
    use serde_json::json;

    // The desktop registers İşlemler's tools when it starts (lib.rs); a test does it itself.
    kentos_processing::command::register_as("desktop");
    const VITRIN: &str = "tr.ornek.vitrin";
    fn find<'a>(
        nodes: &'a [kentos_commands::LayerNode],
        name: &str,
    ) -> Option<&'a kentos_commands::LayerNode> {
        nodes.iter().find_map(|n| {
            (n.name == name)
                .then_some(n)
                .or_else(|| find(&n.children, name))
        })
    }
    let buffer = json!({ "command": "cad.processing.run", "op": "execute", "input": {
        "tool": "geometry.buffer",
        "parameters": { "input": { "scope": "layer", "layerId": "yapi" }, "distance": 3,
                        "layer": { "newName": "Yapı tamponu 3 m" } } } });
    let open = |name: &str, write: bool| {
        let dir = temp(name);
        let mut gis = app(&dir, Some(Module::Gis));
        install(&mut gis, "vitrin.kek", vitrin_package());
        let path =
            Path::new(env!("CARGO_MANIFEST_DIR")).join("../../samples/eklenti-vitrini/vitrin.kcad");
        let snapshot =
            kentos_kcad::decode(&std::fs::read(path).unwrap()).expect("the demo drawing");
        gis.document = Some(crate::document::Document::from_v2(snapshot, None).expect("opens"));
        let package = gis
            .extensions
            .registry
            .as_ref()
            .and_then(|r| r.package_dir(VITRIN))
            .unwrap();
        let _ = gis.extensions_event(Event::Launch(
            VITRIN.into(),
            "command".into(),
            "yapi-tamponu".into(),
            Ok(super::runner::Launch::for_test(
                "/bin/sh".into(),
                package,
                "vitrin.logic:yapi_tamponu",
                "Yapı tamponu",
                write,
            )),
        ));
        (dir, gis)
    };
    // May write: the buffers on a new layer, in the run's step (named after the command).
    let (dir, mut gis) = open("yerlesik-arac", true);
    let answer = gis
        .ext_answer_for_test(VITRIN, "run", &buffer)
        .expect("the tool ran");
    assert_eq!(answer["status"], "completed", "{answer}");
    let model = &gis.document.as_ref().unwrap().model;
    let layer = find(model.layers().nodes(), "Yapı tamponu 3 m").expect("its new layer");
    assert_eq!(
        model.count(&layer.id),
        5,
        "a buffer for each of the five buildings"
    );
    let _ = std::fs::remove_dir_all(&dir);
    // May only read: the tool's write is refused, nothing is written.
    let (dir, mut gis) = open("yerlesik-arac-okur", false);
    let refused = gis.ext_answer_for_test(VITRIN, "run", &buffer);
    assert!(
        matches!(&refused, Err(e) if e.code == "capability" && e.message.contains("document.write")),
        "{refused:?}"
    );
    let model = &gis.document.as_ref().unwrap().model;
    assert!(find(model.layers().nodes(), "Yapı tamponu 3 m").is_none());
    let _ = std::fs::remove_dir_all(&dir);
}
