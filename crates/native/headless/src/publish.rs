//! Atomic sheet delivery. Planning writes nothing; publication exposes the whole folder in one rename.
use std::{
    collections::BTreeSet,
    fs,
    path::{Path, PathBuf},
};
use kentos_commands::{
    PublishSheets, SheetPublishMode, SheetPublishPin, SheetPublishedFile, SheetPublishIssue,
    SheetsPublished, CallOp, CommandResult, CommandError, FileEffect, encoding::sha256_hex,
};
use kentos_domain::Document;
use kentos_native_application::jobs::Progress;
use kentos_sheet::{
    SheetBook,
    project::{read_project_sheets, project_sheets},
};
use crate::file_commands::Refused;
const MANIFEST: &str = "yayin.json";
fn refuse(code: &str, message: impl Into<String>, path: &str) -> Refused {
    Box::new(CommandError {
        code: code.into(),
        message: message.into(),
        path: Some(path.into()),
        ..Default::default()
    })
}
fn stopped(progress: Option<&Progress>) -> Result<(), Refused> {
    if progress.is_some_and(Progress::stopped) {
        Err(refuse(
            "cancelled",
            "Yayın iptal edildi; teslim klasörü oluşturulmadı.",
            "path",
        ))
    } else {
        Ok(())
    }
}
/// A deterministic document pin. Linked-resource version pinning extends this at the plot boundary.
pub fn pin(doc: &Document) -> Result<SheetPublishPin, Refused> {
    Ok(SheetPublishPin {
        revision: doc.revision().to_string(),
        sha256: sha256_hex(
            &serde_json::to_vec(&doc.to_snapshot_v2())
                .map_err(|e| refuse("book_unreadable", e.to_string(), "pin"))?,
        ),
    })
}
fn stem(value: &str) -> String {
    let mut s: String = value
        .chars()
        .map(|c| {
            if c.is_control() || "/\\:*?\"<>|".contains(c) {
                '_'
            } else {
                c
            }
        })
        .collect();
    s = s.trim().trim_matches('.').trim_end().to_owned();
    if s.to_ascii_lowercase().ends_with(".pdf") {
        s.truncate(s.len() - 4);
    }
    while s.len() > 200 {
        s.pop();
    }
    let base = s.split('.').next().unwrap_or("").to_ascii_uppercase();
    if matches!(base.as_str(), "CON" | "PRN" | "AUX" | "NUL")
        || (1..=9).any(|i| base == format!("COM{i}") || base == format!("LPT{i}"))
    {
        s.insert(0, '_');
    }
    if s.is_empty() { "pafta".into() } else { s }
}
fn names(
    book: &SheetBook,
    chosen: &[String],
    input: &PublishSheets,
) -> Result<Vec<(String, Vec<String>)>, Refused> {
    if input.mode == SheetPublishMode::Single {
        return Ok(vec![(
            format!("{}.pdf", stem(input.name.as_deref().unwrap_or("paftalar"))),
            chosen.to_vec(),
        )]);
    }
    if input.naming.trim().is_empty()
        || input.naming.len() > 1000
        || input.naming.contains(['/', '\\', '\0'])
    {
        return Err(refuse(
            "invalid_input",
            "Dosya adı şablonu boş olamaz; klasör ayırıcısı içeremez. {index}, {sheet}, {id} kullanın.",
            "naming",
        ));
    }
    let rest = input
        .naming
        .replace("{index}", "")
        .replace("{sheet}", "")
        .replace("{id}", "")
        .replace("{number}", "")
        .replace("{title}", "")
        .replace("{revision}", "")
        .replace("{status}", "")
        .replace("{subset}", "");
    if rest.contains(['{', '}']) {
        return Err(refuse(
            "invalid_input",
            "Dosya adında tanınmayan yer tutucu var; {index}, {sheet}, {id} kullanın.",
            "naming",
        ));
    }
    let mut used = BTreeSet::new();
    let mut out = Vec::new();
    for (n, id) in chosen.iter().enumerate() {
        let sheet = book
            .sheet(id)
            .ok_or_else(|| refuse("unknown_sheet", format!("“{id}” paftası yok."), "sheets"))?;
        let info = sheet.drawing_info.clone().unwrap_or_default();
        let mut rendered = String::new();
        let mut rest = input.naming.as_str();
        while let Some((before, tail)) = rest.split_once('{') {
            rendered.push_str(before);
            let Some((key, after)) = tail.split_once('}') else {
                break;
            };
            rendered.push_str(&match key {
                "index" => format!("{:03}", n + 1),
                "sheet" => sheet.name.clone(),
                "id" => id.clone(),
                "number" => info.number.clone(),
                "title" => {
                    if info.title.is_empty() {
                        sheet.name.clone()
                    } else {
                        info.title.clone()
                    }
                }
                "revision" => info.revision.clone(),
                "status" => info.status.clone(),
                "subset" => info.subset.join("-"),
                _ => String::new(),
            });
            rest = after;
        }
        rendered.push_str(rest);
        let base = stem(&rendered);
        let mut name = format!("{base}.pdf");
        let mut suffix = 2;
        while !used.insert(name.to_lowercase()) {
            name = format!("{base}-{suffix}.pdf");
            suffix += 1;
        }
        out.push((name, vec![id.clone()]));
    }
    Ok(out)
}
struct Stage {
    path: PathBuf,
    committed: bool,
}
impl Drop for Stage {
    fn drop(&mut self) {
        if !self.committed {
            let _ = fs::remove_dir_all(&self.path);
        }
    }
}
fn stage(target: &Path) -> Result<Stage, Refused> {
    let parent = target.parent().ok_or_else(|| {
        refuse(
            "invalid_file_path",
            "Teslim klasörünün üst klasörü yok.",
            "path",
        )
    })?;
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_nanos());
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let path = parent.join(format!(
        ".kentos-yayin-{}-{stamp}-{}",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    fs::create_dir(&path).map_err(|e| {
        refuse(
            "file_unwritable",
            format!("Geçici teslim klasörü açılamadı: {e}"),
            "path",
        )
    })?;
    Ok(Stage {
        path,
        committed: false,
    })
}
fn issue(sheet: &str, code: &str, message: String, error: bool, fix: String) -> SheetPublishIssue {
    SheetPublishIssue {
        sheet: sheet.into(),
        code: code.into(),
        message,
        fix,
        error,
    }
}
/// Shared by direct, job, Python, MCP and desktop publication. No output folder survives an error/cancel.
fn work(
    doc: &Document,
    input: &PublishSheets,
    op: CallOp,
    progress: Option<&Progress>,
) -> Result<(SheetsPublished, Option<FileEffect>), Refused> {
    let target = crate::package::target(&input.path)?;
    stopped(progress)?;
    let source = pin(doc)?;
    if input.pin.as_ref().is_some_and(|p| p != &source) {
        return Err(refuse(
            "pin_mismatch",
            "Çizim önizlemeden sonra değişti; yayını yeniden denetleyin.",
            "pin",
        ));
    }
    let kept = doc.sheets().ok_or_else(|| {
        refuse(
            "no_sheets",
            "Çizimde pafta yok; önce bir pafta oluşturun.",
            "sheets",
        )
    })?;
    let (mut book, pictures) =
        read_project_sheets(kept).map_err(|e| refuse("book_unreadable", e.message, "sheets"))?;
    let chosen = input
        .sheets
        .clone()
        .unwrap_or_else(|| book.sheets.iter().map(|s| s.id.clone()).collect());
    if chosen.is_empty() {
        return Err(refuse(
            "no_sheets",
            "Yayınlanacak en az bir pafta seçin.",
            "sheets",
        ));
    }
    let mut seen = BTreeSet::new();
    for id in &chosen {
        if book.sheet(id).is_none() {
            return Err(refuse(
                "unknown_sheet",
                format!("“{id}” paftası kitapta yok."),
                "sheets",
            ));
        }
        if !seen.insert(id) {
            return Err(refuse(
                "invalid_input",
                format!("“{id}” iki kez seçildi; her pafta bir kez yayınlanır."),
                "sheets",
            ));
        }
    }
    if let Some(dpi) = input.dpi {
        if !(36..=1200).contains(&dpi) {
            return Err(refuse(
                "invalid_input",
                "Çözünürlük 36–1200 dpi olmalı.",
                "dpi",
            ));
        }
        for s in &mut book.sheets {
            s.export.dpi = dpi;
        }
    }
    let mut snapshot = doc.reading_copy();
    snapshot
        .set_sheets(
            project_sheets(&book, |h| {
                pictures
                    .iter()
                    .find(|(hash, _)| hash == h)
                    .map(|(_, b)| b.clone())
            })
            .map_err(|e| refuse("book_unreadable", e.message, "sheets"))?,
        )
        .map_err(|e| refuse("book_unreadable", e, "sheets"))?;
    let planned = names(&book, &chosen, input)?;
    if matches!(op, CallOp::Validate) {
        return Ok((
            SheetsPublished {
                resources: None,
                path: input.path.clone(),
                sheets: chosen,
                files: Vec::new(),
                issues: Vec::new(),
                source,
                ready: false,
                manifest: target.join(MANIFEST).display().to_string(),
                sha256: String::new(),
            },
            None,
        ));
    }
    let mut issues = Vec::new();
    let resource_plan = if input.resources {
        match crate::sheet_resources::Prepared::plan(doc, input, &book, &chosen) {
            Ok(p) => {
                issues.extend(p.issues.clone());
                Some(p)
            }
            Err(e) => {
                issues.push(issue(
                    "",
                    &e.code,
                    e.message,
                    true,
                    "Kaynak paketi sorununu düzeltip yeniden denetleyin.".into(),
                ));
                None
            }
        }
    } else {
        None
    };
    let resources = resource_plan.as_ref().map(|p| p.output.clone());
    let total_work = chosen.len() as u64 + resources.as_ref().map_or(0, |r| r.files.len() as u64);
    let mut staging = if matches!(op, CallOp::Execute) {
        Some(stage(&target)?)
    } else {
        None
    };
    let mut files = Vec::new();
    if let Some(p) = progress {
        p.set(0, total_work);
    }
    let date = kentos_plot::book::today();
    let user = kentos_plot::book::current_user();
    for (n, (name, sheets)) in planned.iter().enumerate() {
        stopped(progress)?;
        let opts = kentos_sheet::pdf::PdfOptions {
            sheets: sheets.clone(),
            geo: input.geo,
            layers: input.layers,
        };
        let page_progress = progress.map(|p| {
            p.span(
                if input.mode == SheetPublishMode::Single {
                    0
                } else {
                    n as u64
                },
                if input.mode == SheetPublishMode::Single {
                    chosen.len() as u64
                } else {
                    1
                },
                total_work,
            )
        });
        match kentos_plot::book::book_pdf_controlled(
            &snapshot,
            sheets,
            &user,
            &date,
            input.drawing.as_deref(),
            Some(opts),
            page_progress.as_ref(),
        ) {
            Ok(pdf) => {
                for (owner, f) in pdf.sheet_findings {
                    issues.push(issue(
                        &owner,
                        &f.code,
                        f.message,
                        f.severity == kentos_sheet::preflight::Severity::Error
                            || f.code == "map_unplaced",
                        f.fix,
                    ));
                }
                for note in pdf.notes {
                    issues.push(issue(
                        &sheets.join(","),
                        "plot_rasterized",
                        note,
                        false,
                        "Çözünürlüğü ve raster kaynaklarını denetleyin.".into(),
                    ));
                }
                stopped(progress)?;
                if let Some(stage) = &staging {
                    let path = stage.path.join(name);
                    if path.exists() {
                        return Err(refuse(
                            "file_exists",
                            "Çıktı adları bu dosya sisteminde çakışıyor; ad şablonuna {index} ekleyin.",
                            "naming",
                        ));
                    }
                    crate::files::write_bytes(&pdf.bytes, &path)
                        .map_err(|e| refuse("file_unwritable", e.message, "path"))?;
                }
                files.push(SheetPublishedFile {
                    name: name.clone(),
                    sheets: sheets.clone(),
                    bytes: pdf.bytes.len() as u64,
                    sha256: sha256_hex(&pdf.bytes),
                });
            }
            Err(e) => {
                let before = issues.len();
                if e.code == "cancelled" {
                    return Err(refuse("cancelled", e.message, "path"));
                }
                // A single combined failure is diagnosed page by page so the whole missing-source report is useful.
                if sheets.len() > 1 {
                    for id in sheets {
                        stopped(progress)?;
                        if let Err(page) = kentos_plot::book::book_pdf_at(
                            &snapshot,
                            std::slice::from_ref(id),
                            &user,
                            &date,
                            input.drawing.as_deref(),
                        ) {
                            issues.push(issue(
                                id,
                                &page.code,
                                page.message,
                                true,
                                "Paftanın kaynaklarını düzeltip yeniden denetleyin.".into(),
                            ));
                        }
                    }
                }
                if !issues[before..].iter().any(|i| i.error) {
                    issues.push(issue(
                        &sheets.join(","),
                        &e.code,
                        e.message,
                        true,
                        "Kaynakları ve ön denetimi düzeltin.".into(),
                    ));
                }
                files.push(SheetPublishedFile {
                    name: name.clone(),
                    sheets: sheets.clone(),
                    bytes: 0,
                    sha256: String::new(),
                });
            }
        }
        if let Some(p) = progress {
            p.set(
                if input.mode == SheetPublishMode::Single {
                    chosen.len() as u64
                } else {
                    (n + 1) as u64
                },
                total_work,
            );
        }
    }
    stopped(progress)?;
    let ready = !issues.iter().any(|i| i.error);
    let bytes=serde_json::to_vec_pretty(&serde_json::json!({"format":"kentos.publish/1","drawingSet":chosen.iter().filter_map(|id| book.sheet(id).map(|s|serde_json::json!({"id":s.id,"name":s.name,"info":s.drawing_info}))).collect::<Vec<_>>(),"source":source,"date":date,"options":{"mode":input.mode,"naming":input.naming,"dpi":input.dpi,"geo":input.geo,"layers":input.layers},"sheets":chosen,"files":files,"resources":resources,"issues":issues})).map_err(|e|refuse("file_unwritable",e.to_string(),"path"))?;
    let report = SheetsPublished {
        resources,
        path: input.path.clone(),
        sheets: chosen,
        files,
        issues,
        source,
        ready,
        manifest: target.join(MANIFEST).display().to_string(),
        sha256: sha256_hex(&bytes),
    };
    if let Some(stage) = &mut staging {
        if !ready {
            return Err(refuse(
                "publish_blocked",
                report
                    .issues
                    .iter()
                    .filter(|i| i.error)
                    .map(|i| format!("{}: {} {}", i.sheet, i.message, i.fix))
                    .collect::<Vec<_>>()
                    .join(" "),
                "sheets",
            ));
        }
        if let Some(resources) = &resource_plan {
            let phase = progress.map(|p| {
                p.span(
                    report.sheets.len() as u64,
                    resources.output.files.len() as u64,
                    total_work,
                )
            });
            resources.write(&stage.path, phase.as_ref())?;
        }
        stopped(progress)?;
        crate::files::write_bytes(&bytes, &stage.path.join(MANIFEST))
            .map_err(|e| refuse("file_unwritable", e.message, "path"))?;
        stopped(progress)?;
        crate::package::target(&input.path)?; // A concurrent writer must not lose its published folder.
        if target.exists() {
            fs::remove_dir(&target)
                .map_err(|e| refuse("file_unwritable", e.to_string(), "path"))?;
        }
        fs::rename(&stage.path, &target)
            .map_err(|e| refuse("file_unwritable", e.to_string(), "path"))?;
        stage.committed = true;
        if let Some(p) = progress {
            p.set(total_work, total_work);
        }
        return Ok((
            report.clone(),
            Some(FileEffect {
                path: report.manifest,
                bytes: bytes.len() as u64,
                sha256: report.sha256,
            }),
        ));
    }
    Ok((report, None))
}
pub(crate) fn run(
    doc: &Document,
    input: &PublishSheets,
    op: CallOp,
    progress: Option<&Progress>,
) -> (CommandResult<serde_json::Value>, Option<FileEffect>) {
    match work(doc, input, op, progress) {
        Ok((report, effect)) => (
            CommandResult::Completed {
                output: if matches!(op, CallOp::Validate) {
                    serde_json::Value::Null
                } else {
                    serde_json::to_value(report).unwrap_or_default()
                },
                warnings: Vec::new(),
            },
            effect,
        ),
        Err(e) if e.code == "cancelled" => (CommandResult::Cancelled, None),
        Err(e) => (crate::file_commands::refused_result(*e), None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cancelled_publication_never_creates_a_target_or_temporary_delivery() {
        let fx: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../fixtures/commands/v1/file.publish.sheets.json"
        ))
        .unwrap();
        let mut doc =
            Document::from_snapshot(serde_json::from_value(fx["setup"].clone()).unwrap()).unwrap();
        let book: SheetBook = serde_json::from_value(fx["book"].clone()).unwrap();
        doc.set_sheets(project_sheets(&book, |_| None).unwrap())
            .unwrap();
        let root =
            std::env::temp_dir().join(format!("kentos-publish-cancel-{}", std::process::id()));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir(&root).unwrap();
        let path = root.join("delivery");
        let input: PublishSheets =
            serde_json::from_value(serde_json::json!({"path":path})).unwrap();
        let progress = Progress::default();
        progress.stop();
        let (result, effect) = run(&doc, &input, CallOp::Execute, Some(&progress));
        assert!(matches!(result, CommandResult::Cancelled));
        assert!(effect.is_none());
        assert_eq!(fs::read_dir(&root).unwrap().count(), 0);
        fs::remove_dir(root).unwrap();
    }
}
