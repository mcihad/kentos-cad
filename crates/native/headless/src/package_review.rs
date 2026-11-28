//! Gelen paketi incele (docs/adr/8104; plan/bim-plan.md BIM-18, B20): `file.review.package` reads a package as it came,
//! without the author's drawing, and measures what each file is from its own bytes; the standards package's rule
//! (`dijital_proje.inceleme`) decides on those facts (K10: the checks are Python rules over what Rust measures).
//!
//! - **The package:** a publication's folder (`yayin.json`: its files with bytes and SHA-256, its sheets, the drawing's
//!   revision it was made from, its BIM part) or a lone `.ifc` file. Each file of the folder: listed or not, there or
//!   not, its bytes and digest the list's or not.
//! - **The sheets:** each PDF's PDF/A-2 report (`kentos_formats::pdf::pdfa2_report`), its pages that hold only pictures
//!   (a scanned sheet: images, no line and no text), and the source KentOS writes in it (`kentos.sheet-source/1`: the
//!   drawing's revision and digest).
//! - **The model:** its facts as the file says them (`kentos_ifc::read::file_facts`), or why it did not read; its digest.
//! - **The form:** the author's model quality form record as written.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use kentos_commands::{
    CallOp, CommandResult, PackageReviewed, ReviewFile, ReviewFinding, ReviewFormRow, ReviewLayer,
    ReviewPackage, encoding::sha256_hex,
};
use serde_json::{Value, json};

const PACKAGE: &str = "dijital-proje";
const MODULE: &str = "dijital_proje.inceleme";
const LIST: &str = "yayin.json";
const DELIVERY_LIST: &str = "teslim-listesi.csv";
/// The most bytes a file of a package is read with.
const MOST: u64 = 512 * 1024 * 1024;

fn refused(code: &str, message: String, path: &str) -> CommandResult<Value> {
    crate::file_commands::refused_result(*crate::file_commands::refuse(code, message, path))
}

/// The file's kind by its name.
fn kind(name: &str) -> &'static str {
    let lower = name.to_ascii_lowercase();
    if lower.ends_with(".pdf") {
        "pdf"
    } else if lower.ends_with(".ifc") {
        "ifc"
    } else if lower == crate::publish_bim::FORM {
        "form"
    } else if lower == LIST || lower == DELIVERY_LIST {
        "list"
    } else {
        "other"
    }
}

fn read(path: &Path) -> Option<Vec<u8>> {
    let size = std::fs::metadata(path).ok()?.len();
    if size > MOST {
        return None;
    }
    std::fs::read(path).ok()
}

/// The source KentOS writes in a sheet (its information dictionary's keywords, `kentos.sheet-source/1`): the drawing's
/// revision and digest; none in a PDF KentOS did not write.
fn sheet_source(bytes: &[u8]) -> Option<Value> {
    let file = kentos_formats::pdf::file::PdfFile::open(bytes).ok()?;
    let info = file.field(&file.trailer, "Info");
    let info = info.dict()?;
    let keywords = file.resolve(info.get("Keywords")?);
    let kentos_formats::pdf::object::Obj::Str(raw) = keywords else {
        return None;
    };
    let text = kentos_formats::pdf::content::decode_text_string(&raw);
    let v: Value = serde_json::from_str(&text).ok()?;
    (v["format"] == "kentos.sheet-source/1")
        .then(|| json!({"revision": v["source"]["revision"], "sha256": v["source"]["sha256"]}))
}

/// What a PDF is: its PDF/A report, its pages of pictures only, the source in it.
fn sheet_facts(name: &str, bytes: &[u8]) -> Value {
    let pages = match kentos_formats::pdf::page_count(bytes) {
        Ok(n) => n,
        Err(e) => return json!({"ad": name, "okunmuyor": e.to_string()}),
    };
    let pdfa = match kentos_formats::pdf::pdfa::pdfa2_report(bytes) {
        Ok(r) => json!({
            "part": r.part,
            "conformance": r.conformance,
            "findings": r.findings.iter().map(|f| json!({"clause": f.clause, "message": f.message})).collect::<Vec<_>>(),
        }),
        Err(why) => {
            json!({"part": null, "conformance": null, "findings": [{"clause": "6.1", "message": why}]})
        }
    };
    let scanned: Vec<u32> = (1..=pages)
        .filter(|n| {
            kentos_formats::pdf::read_page(bytes, *n)
                .is_ok_and(|p| p.left.images > 0 && p.paths.is_empty() && p.texts.is_empty())
        })
        .collect();
    json!({"ad": name, "sayfa": pages, "pdfa": pdfa, "taranmis": scanned, "kaynak": sheet_source(bytes)})
}

/// The facts the rule reads, the files as the review read them, the model's name.
#[allow(clippy::result_large_err)]
fn facts(
    input: &ReviewPackage,
) -> Result<(Value, Vec<ReviewFile>, Option<String>), CommandResult<Value>> {
    let text = input.path.trim();
    let path = Path::new(text);
    if text.is_empty() || !path.is_absolute() {
        return Err(refused(
            "invalid_file_path",
            "Paketin tam yolunu verin: klasörü ya da .ifc dosyası.".into(),
            "path",
        ));
    }
    let lone = path.is_file();
    if lone
        && !path
            .extension()
            .is_some_and(|e| e.eq_ignore_ascii_case("ifc"))
    {
        return Err(refused(
            "invalid_file_path",
            format!(
                "“{text}” bir klasör ya da .ifc dosyası değil: paket bir yayın klasörüdür ya da tek bir IFC modeli."
            ),
            "path",
        ));
    }
    if !lone && !path.is_dir() {
        return Err(refused(
            "invalid_file_path",
            format!("“{text}” yok."),
            "path",
        ));
    }
    let (folder, mut names): (PathBuf, Vec<String>) = if lone {
        (
            path.parent().map(Path::to_path_buf).unwrap_or_default(),
            vec![
                path.file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_default(),
            ],
        )
    } else {
        let mut n: Vec<String> = std::fs::read_dir(path)
            .map_err(|e| {
                refused(
                    "invalid_file_path",
                    format!("“{text}” okunamadı: {e}"),
                    "path",
                )
            })?
            .flatten()
            .filter(|e| e.path().is_file())
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| !n.starts_with('.'))
            .collect();
        n.sort();
        (path.to_path_buf(), n)
    };
    // The list.
    let (list, list_error): (Option<Value>, Option<String>) =
        if lone || !names.iter().any(|n| n == LIST) {
            (None, None)
        } else {
            match read(&folder.join(LIST)).map(|b| serde_json::from_slice::<Value>(&b)) {
                Some(Ok(v)) if v["format"] == "kentos.publish/1" => (Some(v), None),
                Some(Ok(_)) => (None, Some("biçimi kentos.publish/1 değil".into())),
                Some(Err(e)) => (None, Some(e.to_string())),
                None => (None, Some("okunamadı".into())),
            }
        };
    let listed: Vec<Value> = list
        .as_ref()
        .and_then(|l| l["files"].as_array().cloned())
        .unwrap_or_default();
    let listed_names: BTreeSet<String> = listed
        .iter()
        .filter_map(|f| f["name"].as_str().map(str::to_owned))
        .collect();
    let model_name: Option<String> = if lone {
        names.first().cloned()
    } else {
        match &list {
            Some(l) => l["bim"]["model"].as_str().map(str::to_owned),
            // No list: the folder's one IFC file, when it has one.
            None => {
                let ifcs: Vec<&String> = names.iter().filter(|n| kind(n) == "ifc").collect();
                (ifcs.len() == 1).then(|| ifcs[0].clone())
            }
        }
    };
    let form_name: Option<String> = list
        .as_ref()
        .and_then(|l| l["bim"]["form"]["name"].as_str().map(str::to_owned));
    for f in &listed_names {
        if !names.contains(f) {
            names.push(f.clone());
        }
    }
    let mut files = Vec::new();
    let mut records = Vec::new();
    let mut sheets = Vec::new();
    let mut model_bytes: Option<Vec<u8>> = None;
    for name in &names {
        let at = folder.join(name);
        let there = at.is_file();
        let entry = listed.iter().find(|f| f["name"] == name.as_str());
        let bytes = if there { read(&at) } else { None };
        let (bytes_ok, sha_ok) = match (entry, &bytes) {
            (Some(e), Some(b)) => (
                Some(e["bytes"].as_u64() == Some(b.len() as u64)),
                Some(
                    e["sha256"]
                        .as_str()
                        .is_some_and(|s| s.eq_ignore_ascii_case(&sha256_hex(b))),
                ),
            ),
            _ => (None, None),
        };
        let k = kind(name);
        records.push(json!({"ad": name, "tur": k, "listede": entry.is_some(), "var": there, "baytTutar": bytes_ok, "ozetTutar": sha_ok}));
        files.push(ReviewFile {
            name: name.clone(),
            kind: k.into(),
            listed: entry.is_some(),
        });
        if let Some(b) = &bytes {
            if k == "pdf" {
                sheets.push(sheet_facts(name, b));
            }
            if model_name.as_deref() == Some(name.as_str()) {
                model_bytes = Some(b.clone());
            }
        }
    }
    let form: Option<Value> = form_name
        .as_ref()
        .and_then(|n| read(&folder.join(n)))
        .and_then(|b| serde_json::from_slice(&b).ok());
    let (ifc, ifc_error, model_sha) = match (&model_name, &model_bytes) {
        (Some(name), Some(b)) => match kentos_ifc::read::file_facts(
            b,
            name,
            &kentos_ifc::read::Limits::default(),
            &|| false,
        ) {
            Ok(f) => (
                serde_json::to_value(f).unwrap_or(Value::Null),
                None,
                Some(sha256_hex(b)),
            ),
            Err(e) => (Value::Null, Some(e.to_string()), Some(sha256_hex(b))),
        },
        // A model the list names and the folder lacks: the package layer says so (review_file_missing).
        _ => (Value::Null, None, None),
    };
    let package = json!({
        "tur": if lone { "dosya" } else { "klasor" },
        "liste": list.is_some(),
        "listeHata": list_error,
        "dosyalar": records,
        "kaynak": list.as_ref().map(|l| json!({"revision": l["source"]["revision"], "sha256": l["source"]["sha256"]})),
        "model": model_name,
        "formAdi": form_name,
    });
    let facts = json!({
        "paket": package,
        "paftalar": sheets,
        "ifc": if ifc.is_null() { Value::Null } else { ifc },
        "ifcHata": ifc_error,
        "modelOzeti": model_sha,
        "form": form,
        "idare": {"system": input.system},
    });
    Ok((facts, files, model_name))
}

#[allow(clippy::result_large_err)]
fn reviewed(input: &ReviewPackage) -> Result<PackageReviewed, CommandResult<Value>> {
    let (facts, files, model) = facts(input)?;
    let decided = kentos_standards::call(PACKAGE, MODULE, "denetle", vec![facts]).map_err(|why| {
        refused(
            "review_rule_unavailable",
            format!("İnceleme kuralı (dijital-proje) çalışmadı: {why}. Ayarlar › Python'dan bir Python seçin."),
            "path",
        )
    })?;
    let text = |v: &Value| v.as_str().unwrap_or_default().to_owned();
    let layers = decided["katmanlar"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|l| ReviewLayer {
            key: text(&l["key"]),
            holds: l["holds"].as_bool(),
            words: text(&l["words"]),
        })
        .collect();
    let findings = decided["bulgular"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|b| ReviewFinding {
            layer: text(&b["katman"]),
            code: text(&b["code"]),
            error: b["error"].as_bool().unwrap_or(true),
            message: text(&b["message"]),
            repair: text(&b["repair"]),
            cite: text(&b["cite"]),
            file: b["file"].as_str().map(str::to_owned),
            objects: b["objects"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|g| g.as_str().map(str::to_owned))
                .collect(),
        })
        .collect();
    let rows = decided["satirlar"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|r| ReviewFormRow {
            no: r["no"]
                .as_u64()
                .and_then(|n| u8::try_from(n).ok())
                .unwrap_or(0),
            title: text(&r["baslik"]),
            author: r["muellif"].as_str().map(str::to_owned),
            kentos: text(&r["kentos"]),
            agree: r["uyusuyor"].as_bool(),
        })
        .collect();
    let table = decided["tablo"].as_str().map(str::to_owned);
    Ok(PackageReviewed {
        path: input.path.trim().to_owned(),
        files,
        model,
        layers,
        findings,
        table,
        rows,
    })
}

/// `file.review.package`: the facts measured and the rule's answer; nothing is written.
pub(crate) fn run(input: &ReviewPackage, op: CallOp) -> CommandResult<Value> {
    if matches!(op, CallOp::Validate) {
        let p = Path::new(input.path.trim());
        return if p.is_absolute() && (p.is_dir() || p.is_file()) {
            CommandResult::Completed {
                output: Value::Null,
                warnings: Vec::new(),
            }
        } else {
            refused(
                "invalid_file_path",
                "Paketin tam yolunu verin: klasörü ya da .ifc dosyası.".into(),
                "path",
            )
        };
    }
    match reviewed(input) {
        Ok(out) => CommandResult::Completed {
            output: serde_json::to_value(out).unwrap_or(Value::Null),
            warnings: Vec::new(),
        },
        Err(refusal) => refusal,
    }
}
