//! A BIM permit package (docs/adr/8075; plan/bim-plan.md BIM-14; Dijital Proje Yön. Md. 9/1, 15): beside the PDF/A
//! sheets, in the same stage and from the same pinned drawing, the building's IFC model (EK-2's name) and the model
//! quality control form's record. The model is read back from the stage — the delivered file, not the drawing — and the
//! standards package's rule (`dijital_proje.bim`) decides on it, on the drawing's facts with the package's files, and on
//! the author's answers. Each finding it leaves is an error issue, which stops the publication before the folder is
//! exposed (the publication's own rule). Planning writes nothing and reads the same bytes from memory.

use std::path::Path;

use kentos_commands::{
    BimPackage, BimPackaged, FormRow, PackageResult, PublishSheets, SheetPublishIssue,
    SheetPublishMode, SheetPublishScope, SheetPublishedFile, encoding::sha256_hex,
};
use kentos_domain::Document;
use serde_json::{Value, json};

use crate::publish::issue;

/// The form's record in the folder.
pub(crate) const FORM: &str = "kalite-kontrol-formu.json";
/// The standards package and its rule.
const PACKAGE: &str = "dijital-proje";
const RULE: &str = "dijital_proje.bim";

/// What a BIM package adds to a publication: its report, its delivered file (the model, written to the stage when
/// there is one) and its part of `yayin.json`. The form's record is KentOS's own, beside `yayin.json`: no delivered
/// file, so not under EK-2's name rule nor in the delivery list; `yayin.json` names it with its digest.
pub(crate) struct Packed {
    pub report: BimPackaged,
    pub files: Vec<SheetPublishedFile>,
    pub manifest: Value,
}

/// The package's part for `input`'s `bim`, its files written into `stage` (none: a plan); `files` and `scope` are the
/// sheets' as published, `date` the publication's. A request the package cannot meet is an error issue and no part.
#[allow(clippy::too_many_arguments)]
pub(crate) fn package(
    doc: &Document,
    input: &PublishSheets,
    bim: &BimPackage,
    stage: Option<&Path>,
    files: &[SheetPublishedFile],
    scope: &[SheetPublishScope],
    date: &str,
    issues: &mut Vec<SheetPublishIssue>,
) -> Option<Packed> {
    if !input.pdfa || input.mode != SheetPublishMode::Separate {
        return stop(
            issues,
            "bim_needs_pdfa",
            "BIM ruhsat paketi PDF/A ve pafta başına ayrı dosya ister (Dijital Proje Yön. Md. 4/2, 8).".into(),
            "PDF/A'yı ve ayrı PDF'leri seçin.",
        );
    }
    let application = format!("KentOS Mimari {}", env!("CARGO_PKG_VERSION"));
    let Some(planned) = kentos_native_application::ifc_facts::plan(
        doc,
        bim.building.as_deref(),
        &format!("{date}T00:00:00"),
        &application,
    ) else {
        let why = match &bim.building {
            Some(id) => format!("“{id}” binası bina programında yok."),
            None => "Çizimde duvar yok: BIM paketi binanın modelini ister.".into(),
        };
        return stop(
            issues,
            "bim_no_building",
            why,
            "Binayı çizin ya da programdaki bir binayı seçin.",
        );
    };
    let name = planned.file_name();
    if files.iter().any(|f| f.name == name || f.name == FORM) {
        return stop(
            issues,
            "bim_name_taken",
            format!("“{name}” ya da “{FORM}” bir paftanın dosya adı."),
            "Paftaların adlandırmasını değiştirin.",
        );
    }
    let text = planned.text();
    // The delivered file is the one read: written to the stage, then read back from there.
    let bytes = match stage {
        Some(dir) => {
            let path = dir.join(&name);
            if let Err(e) = crate::files::write_bytes(text.as_bytes(), &path) {
                return stop(
                    issues,
                    "file_unwritable",
                    format!("“{name}” yazılamadı: {}", e.message),
                    "Klasörü denetleyin.",
                );
            }
            match std::fs::read(&path) {
                Ok(b) => b,
                Err(e) => {
                    return stop(
                        issues,
                        "file_unwritable",
                        format!("“{name}” geri okunamadı: {e}"),
                        "Klasörü denetleyin.",
                    );
                }
            }
        }
        None => text.into_bytes(),
    };
    let ifc = match planned.facts_of(&bytes, false) {
        Ok(f) => f,
        Err(why) => return stop(issues, "bim_unreadable", why, "Çizimi bildirin."),
    };
    let cizim = json!({
        "cad": serde_json::to_value(kentos_processing::audit_facts::cad::measure(doc)).unwrap_or(Value::Null),
        "package": {
            "pdfa": input.pdfa,
            "mode": "separate",
            "files": files.iter().map(|f| json!({"name": f.name, "sheets": f.sheets})).collect::<Vec<_>>(),
            "sheets": scope.iter().map(|s| json!({
                "id": s.sheet, "number": s.number, "name": s.name, "revision": s.revision, "paper": s.paper,
            })).collect::<Vec<_>>(),
        },
    });
    // The drawing's answers (docs/adr/8077), a question's answer given to the publication over its own.
    let mut given: Vec<kentos_commands::FormAnswer> =
        doc.form_answers().iter().filter(|a| !bim.answers.iter().any(|b| b.no == a.no)).cloned().collect();
    given.extend(bim.answers.iter().cloned());
    given.sort_by_key(|a| a.no);
    let answers = serde_json::to_value(&given).unwrap_or(Value::Null);
    let decided =
        match kentos_standards::call(PACKAGE, RULE, "denetle", vec![cizim, ifc.clone(), answers]) {
            Ok(v) => v,
            Err(why) => {
                return stop(
                    issues,
                    "bim_rule_unavailable",
                    why,
                    "Ayarlar › Python'dan bir Python seçin.",
                );
            }
        };
    for b in decided["bulgular"].as_array().into_iter().flatten() {
        let text = |k: &str| b[k].as_str().unwrap_or_default().to_owned();
        issues.push(issue(
            &name,
            &text("code"),
            text("message"),
            true,
            text("repair"),
        ));
    }
    let results: Vec<PackageResult> = decided["sonuclar"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| PackageResult {
            key: r["key"].as_str().unwrap_or_default().to_owned(),
            holds: r["holds"].as_bool(),
            words: r["words"].as_str().unwrap_or_default().to_owned(),
        })
        .collect();
    let rows: Vec<FormRow> = decided["satirlar"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| FormRow {
            no: r["no"]
                .as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .unwrap_or(0),
            title: r["baslik"].as_str().unwrap_or_default().to_owned(),
            answer: r["cevap"].as_str().unwrap_or_default().to_owned(),
            evidence: r["kanit"].as_str().unwrap_or_default().to_owned(),
            note: r["not"].as_str().unwrap_or_default().to_owned(),
            basis: r["gerekce"].as_str().unwrap_or_default().to_owned(),
        })
        .collect();
    let model = SheetPublishedFile {
        name: name.clone(),
        sheets: Vec::new(),
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
    };
    let versions = json!({
        "kentos": env!("CARGO_PKG_VERSION"),
        "catalog": ifc["catalog"],
        "standards": format!("{PACKAGE} {}", kentos_standards::version(PACKAGE).unwrap_or_default()),
        "schema": ifc["file"]["schema"],
    });
    let record = json!({
        "format": "kentos.ek9Form",
        "version": 1,
        "table": "9.1",
        "model": {"name": model.name, "bytes": model.bytes, "sha256": model.sha256},
        "rows": rows,
        "results": results,
        "answers": given,
        "versions": versions,
    });
    let record_bytes = serde_json::to_vec_pretty(&record).unwrap_or_default();
    if let Some(dir) = stage
        && let Err(e) = crate::files::write_bytes(&record_bytes, &dir.join(FORM))
    {
        return stop(
            issues,
            "file_unwritable",
            format!("“{FORM}” yazılamadı: {}", e.message),
            "Klasörü denetleyin.",
        );
    }
    let report = BimPackaged {
        model: name,
        form: FORM.into(),
        results,
        rows,
    };
    let manifest = json!({
        "model": report.model,
        "form": {"name": report.form, "bytes": record_bytes.len(), "sha256": sha256_hex(&record_bytes)},
        "results": report.results,
        "versions": versions,
    });
    Some(Packed { report, files: vec![model], manifest })
}

/// A request the package cannot meet: an error issue, and no part.
fn stop(
    issues: &mut Vec<SheetPublishIssue>,
    code: &str,
    message: String,
    fix: &str,
) -> Option<Packed> {
    issues.push(issue("model", code, message, true, fix.to_owned()));
    None
}
