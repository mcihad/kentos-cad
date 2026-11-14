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
fn vitrin_package() -> Vec<u8> {
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
            "Yapı ara",
            "Yapı yerleştir"
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
    assert_eq!(
        values["yapilar"]["layerId"], "yapi",
        "its other input as it was"
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
    use super::audits::{AuditDone, AuditState, Severity, findings_of};
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
    });
    gis.ext_audit_done(AuditDone {
        ext: VITRIN.into(),
        local: "yapi-no".into(),
        answer: Ok(done),
        ms: 12,
    });
    let state = gis.extensions.audit.as_ref().unwrap();
    assert!(state.running.is_none() && state.error.is_none());
    assert_eq!(state.findings.len(), 2);
    let _ = gis.ext_audit_tab();
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
        gis.extensions
            .panel
            .as_ref()
            .map(|p| p.definition.title.as_str()),
        Some("Yapı ara")
    );
    assert!(
        gis.docks.contains(crate::app::Panel::Extension),
        "shown in the dock"
    );
    let _ = gis.extensions_event(Event::Panel(PanelEvent::Text("sorgu".into(), "Y-0".into())));
    let _ = gis.extensions_event(Event::Panel(PanelEvent::Check("numarasiz".into(), true)));
    let panel = gis.extensions.panel.as_ref().unwrap();
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
    let panel = gis.extensions.panel.as_ref().unwrap();
    assert_eq!(panel.said, Some((true, "Ortam hazırlanmadı.".to_owned())));
    assert!(panel.running.is_none() && gis.extensions.panel_call.is_none());
    // A run's answer: its rows into the table, its words under the panel; a panel since closed takes none.
    let origin = PanelOrigin {
        ext: "tr.ornek.vitrin".into(),
        panel: "yapi-paneli".into(),
        button: "ara".into(),
        table: Some("sonuc".into()),
    };
    gis.ext_panel_answer(
        &origin,
        Ok(serde_json::json!({ "rows": [{ "no": "Y-01", "alan (m²)": 120.0 }], "columns": ["no", "alan (m²)"],
                               "message": "1 yapı bulundu." })),
    );
    let panel = gis.extensions.panel.as_ref().unwrap();
    assert_eq!(panel.tables["sonuc"].1.len(), 1);
    assert_eq!(panel.said, Some((false, "1 yapı bulundu.".to_owned())));
    let _ = gis.extension_panel_view();
    let _ = gis.extensions_event(Event::Panel(PanelEvent::Close));
    assert!(gis.extensions.panel.is_none());
    gis.ext_panel_answer(&origin, Err("geç".into()));
    assert!(gis.extensions.panel.is_none(), "a late answer goes nowhere");
    let _ = std::fs::remove_dir_all(&dir);
}

/// docs/adr/8057 §24: an item's `when` is checked as the window is drawn: MEBİS's grave tool waits for its layers
/// (`layerSchema: mezar_parsel`) and says so; once a layer of that id is in the drawing it can be pressed.
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
