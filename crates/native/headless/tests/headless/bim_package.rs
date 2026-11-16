//! The BIM permit package (docs/adr/8075; plan/bim-plan.md BIM-14): the section building's permit set published with
//! its IFC model and its model quality control form. The standards package's real rule finds what the model still
//! lacks and the form's questions left to the author, and nothing is published; a stand-in rule that finds nothing
//! (`KENTOS_STANDARDS_DIR`) lets the publication's mechanics be seen whole: the model read back from the folder, the
//! form's record, the delivery list and `yayin.json` naming both.

use std::fs;
use std::path::{Path, PathBuf};

use kentos_commands::{CallEnvelope, CallOp, CommandResult, SheetsPublished, encoding::sha256_hex};
use kentos_domain::Document;
use serde_json::{Value, json};

/// The section building with a künye and its permit set made (Ruhsat takımı, docs/adr/8037).
fn permit_drawing() -> Document {
    let mut snap: kentos_commands::DocumentSnapshotV2 =
        kentos_kcad::decode(include_bytes!("../../../../../fixtures/arch/v1/section-building.kcad")).unwrap();
    // The parcel's İmar durumu (the first sheet's table reads its area), as Mimari › Parsel gives it.
    snap.settings.arch.get_or_insert_with(Default::default).zoning =
        Some(kentos_commands::Zoning { parcel_area: 400.0, far: Some(1.5), coverage: Some(0.4), ..Default::default() });
    // A drawing without a building program says its independent units and its use by hand.
    for (name, label, value) in [("bb_sayisi", "Bağımsız bölüm sayısı", "2"), ("kullanim_amaci", "Kullanım amacı", "Konut")] {
        snap.settings.variables.push(kentos_commands::ProjectVariable {
            name: name.into(),
            label: label.into(),
            kind: kentos_commands::VariableKind::Text,
            value: kentos_commands::VariableValue::Text(value.into()),
        });
    }
    let mut doc = Document::from_snapshot_v2(snap).unwrap();
    let values: Vec<(String, String)> = [
        ("proje_kodu", "260123"),
        ("il", "Sivas"),
        ("ilce", "Merkez"),
        ("mahalle", "Esentepe"),
        ("ada", "1234"),
        ("parsel", "5"),
        ("mimar", "Ayşe Nur Yılmaz"),
        ("sicil_no", "41234"),
        ("buro_adi", "Örnek Mimarlık"),
        ("vergi_no", "1234567890"),
        ("buro_adres", "Atatürk Cad. No:1 Sivas"),
        ("proje_aciklama", "İki katlı konut"),
        ("btb_no", "58-2026-0012"),
        ("yapi_sahibi", "Ayşe Yılmaz"),
        ("tasiyici_sistem", "Betonarme karkas"),
    ]
    .iter()
    .map(|(n, v)| ((*n).to_owned(), (*v).to_owned()))
    .collect();
    let input = kentos_native_application::permit_set::PermitSet {
        phase: kentos_sheet::permit::Phase::Permit,
        values,
        tables: Vec::new(),
        lists: Vec::new(),
        renovation: false,
        building: None,
    };
    kentos_native_application::permit_set::execute(&mut kentos_native_application::ExecutionContext::new(&mut doc), &input)
        .expect("the permit set");
    doc
}

fn folder(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("kentos-bim-paket-{}-{name}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    dir
}

fn publish(doc: &mut Document, path: &Path, op: CallOp, answers: Value) -> kentos_commands::CallAnswer {
    let input = json!({
        "path": path.display().to_string(),
        "mode": "separate",
        "pdfa": true,
        "naming": kentos_sheet::permit::FILE_NAMING,
        "bim": {"answers": answers},
    });
    kentos_headless::file_commands::call(
        doc,
        CallEnvelope {
            command: "file.publish.sheets".into(),
            version: None,
            op,
            request_id: None,
            expected: None,
            relax_constraints: false,
            input,
        },
    )
}

fn report(answer: &kentos_commands::CallAnswer) -> SheetsPublished {
    let CommandResult::Completed { output, .. } = &answer.result else { panic!("{answer:?}") };
    serde_json::from_value(output.clone()).unwrap()
}

/// The author's answers to the questions KentOS leaves to them (the file size, the development level, the area tables).
fn answers() -> Value {
    json!([{"no": 1, "yes": true, "note": "Sınır yayımlanmadı"}, {"no": 7, "yes": true}, {"no": 10, "yes": true}])
}

#[test]
fn the_real_rule_stops_the_package_and_a_passing_one_publishes_it_whole() {
    let mut doc = permit_drawing();
    let path = folder("gercek");
    // The plan: the model read from memory, the form answered, its five results and rows, its findings as errors.
    let planned = report(&publish(&mut doc, &path, CallOp::Plan, json!([])));
    assert!(!planned.ready);
    let bim = planned.bim.as_ref().expect("the package's part");
    assert_eq!(bim.model, "260123_00_MM_XXXX_MD_XXX_NNN_01_000.ifc");
    assert_eq!(bim.rows.len(), 21);
    assert_eq!(bim.results.iter().map(|r| r.key.as_str()).collect::<Vec<_>>(), ["okunuyor", "sema", "profil", "model_pafta", "idare"]);
    assert_eq!(bim.results.iter().map(|r| r.holds).collect::<Vec<_>>(), [Some(true), Some(true), Some(false), Some(true), None]);
    let codes: Vec<&str> = planned.issues.iter().filter(|i| i.error).map(|i| i.code.as_str()).collect();
    assert_eq!(codes.iter().filter(|c| **c == "digital_form_unanswered").count(), 3, "{codes:?}");
    assert!(codes.contains(&"digital_required_field"), "{codes:?}");
    let north = planned.issues.iter().find(|i| i.message.contains("(TrueNorth)")).expect("the context's north");
    assert_eq!(north.sheet, bim.model);
    // With the answers the questions are no finding; the model's missing fields still stop the publication.
    let answered = report(&publish(&mut doc, &path, CallOp::Plan, answers()));
    assert!(!answered.issues.iter().any(|i| i.code == "digital_form_unanswered"));
    let done = publish(&mut doc, &path, CallOp::Execute, answers());
    let CommandResult::Failed { error } = &done.result else { panic!("{done:?}") };
    assert_eq!(error.code, "publish_blocked");
    assert!(error.message.contains("TrueNorth"), "{}", error.message);
    assert!(!path.exists(), "nothing published");
    // Then the mechanics, with a rule that passes (one test: the stand-in is named by the process's environment).
    a_package_the_rule_passes_is_published_whole();
}

fn a_package_the_rule_passes_is_published_whole() {
    // A stand-in standards package whose rule finds nothing and answers every question yes.
    let standards = folder("kural");
    let src = standards.join("dijital-proje/src/dijital_proje");
    fs::create_dir_all(&src).unwrap();
    fs::write(standards.join("dijital-proje/manifest.json"), r#"{"version": "0.0.0-sinama"}"#).unwrap();
    fs::write(src.join("__init__.py"), "").unwrap();
    fs::write(
        src.join("bim.py"),
        "def denetle(cizim, ifc, cevaplar):\n    rows = [{'no': n, 'baslik': 'S', 'cevap': 'Evet', 'kanit': 'geçti', 'not': ''} for n in range(1, 22)]\n    keys = ['okunuyor', 'sema', 'profil', 'model_pafta', 'idare']\n    return {'bulgular': [], 'sonuclar': [{'key': k, 'holds': None if k == 'idare' else True, 'words': k} for k in keys], 'satirlar': rows, 'olgu': ifc['file']}\n",
    )
    .unwrap();
    // SAFETY: no other test of this binary asks a standards rule; this one asks it only after setting the variable.
    unsafe { std::env::set_var("KENTOS_STANDARDS_DIR", &standards) };
    let mut doc = permit_drawing();
    // `KENTOS_BIM_PAKET=<folder>` keeps the published package there, for the independent readers (docs/adr/8075 §6).
    let keep = std::env::var_os("KENTOS_BIM_PAKET").map(PathBuf::from);
    let path = keep.clone().unwrap_or_else(|| folder("gecen"));
    let answer = publish(&mut doc, &path, CallOp::Execute, answers());
    unsafe { std::env::remove_var("KENTOS_STANDARDS_DIR") };
    let published = report(&answer);
    assert!(published.ready, "{:?}", published.issues);
    let bim = published.bim.as_ref().expect("the package's part");
    // The model as delivered: read back from the folder, its digest the delivery's.
    let model = path.join(&bim.model);
    let bytes = fs::read(&model).expect("the model in the folder");
    let listed = published.files.iter().find(|f| f.name == bim.model).expect("the model among the files");
    assert_eq!((listed.bytes, listed.sha256.as_str()), (bytes.len() as u64, sha256_hex(&bytes).as_str()));
    assert!(String::from_utf8_lossy(&bytes).contains("FILE_SCHEMA(('IFC4X3_ADD2'))"));
    // The form's record names the model by its digest and keeps the author's answers.
    let record: Value = serde_json::from_slice(&fs::read(path.join(&bim.form)).expect("the form's record")).unwrap();
    assert_eq!(record["model"]["sha256"], listed.sha256);
    assert_eq!(record["answers"][0]["note"], "Sınır yayımlanmadı");
    assert_eq!(record["rows"].as_array().map(Vec::len), Some(21));
    assert_eq!(record["versions"]["standards"], "dijital-proje 0.0.0-sinama");
    // The delivery list counts the sheets, the model and the form, each once with its digest.
    let list = fs::read_to_string(path.join("teslim-listesi.csv")).expect("the delivery list");
    for f in &published.files {
        assert_eq!(list.matches(&format!(";{};{};", f.name, f.sha256)).count(), 1, "{}", f.name);
    }
    assert!(list.contains(";IFC modeli;"), "{list}");
    // The form's record is KentOS's own, beside yayin.json: not a delivered file, named there with its digest.
    assert!(!published.files.iter().any(|f| f.name == bim.form) && !list.contains(&bim.form), "{list}");
    let manifest: Value = serde_json::from_slice(&fs::read(path.join("yayin.json")).unwrap()).unwrap();
    assert_eq!(manifest["bim"]["model"], bim.model);
    let record_bytes = fs::read(path.join(&bim.form)).unwrap();
    assert_eq!(manifest["bim"]["form"]["sha256"], sha256_hex(&record_bytes));
    if keep.is_none() {
        let _ = fs::remove_dir_all(&path);
    }
    let _ = fs::remove_dir_all(&standards);
}
