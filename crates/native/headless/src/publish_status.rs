//! `file.publish.status` v1 (“Yayın durumu”, docs/adr/1090 §6): a delivery folder's last publication against the
//! drawing now, page by page by content digest (without the title context), and a stopped publication's journal.
//! The source pin now and the published one are kept apart. Reads files, writes none.
use std::path::{Path, PathBuf};

use kentos_commands::{
    CallOp, CommandResult, PublishFolderState, PublishStatus, PublishStatusReport,
    PublishStopReason, SheetPublishPin, SheetPublishState, SheetPublishStatus, SheetPublishedFile,
    SheetPublishedPage, StoppedPublication, encoding::sha256_hex,
};
use kentos_domain::Document;
use kentos_native_application::jobs::Progress;
use kentos_sheet::project::read_project_sheets;

use crate::file_commands::Refused;
use crate::publish::{JOURNAL, JOURNAL_FORMAT, digests, issue, journal_dir, refuse};

fn read_json(path: &Path) -> Option<serde_json::Value> {
    std::fs::read(path)
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
}

fn same_source(a: &SheetPublishPin, b: &SheetPublishPin) -> bool {
    a.sha256 == b.sha256
        && a.resources_sha256 == b.resources_sha256
        && a.context_sha256 == b.context_sha256
}

fn work(
    doc: &Document,
    input: &PublishStatus,
    progress: Option<&Progress>,
) -> Result<PublishStatusReport, Refused> {
    let target = PathBuf::from(&input.path);
    if input.path.trim().is_empty() || !target.is_absolute() {
        return Err(refuse(
            "invalid_file_path",
            format!(
                "“{}” mutlak bir klasör yolu değil; teslim klasörünün tam yolunu verin.",
                input.path
            ),
            "path",
        ));
    }
    let manifest =
        read_json(&target.join("yayin.json")).filter(|m| m["format"] == "kentos.publish/1");
    let journal = journal_dir(&target)
        .and_then(|d| read_json(&d.join(JOURNAL)))
        .filter(|j| j["format"] == JOURNAL_FORMAT);
    let book = match doc.sheets() {
        Some(kept) => {
            read_project_sheets(kept)
                .map_err(|e| refuse("book_unreadable", e.message, "sheets"))?
                .0
        }
        None => Default::default(),
    };
    let context = manifest.as_ref().or(journal.as_ref());
    let (user, date) = context.map_or_else(
        || {
            (
                kentos_plot::book::current_user(),
                kentos_plot::book::today(),
            )
        },
        |m| {
            (
                m["user"].as_str().unwrap_or_default().to_owned(),
                m["date"].as_str().unwrap_or_default().to_owned(),
            )
        },
    );
    let mut captured =
        kentos_plot::provenance::Capture::new(doc, &user, &date, input.drawing.as_deref())
            .map_err(|e| refuse(&e.code, e.message, "pin"))?;
    let dpi = context
        .and_then(|m| m["options"]["dpi"].as_u64())
        .and_then(|d| u16::try_from(d).ok());
    captured
        .dpi_override(dpi)
        .map_err(|e| refuse(&e.code, e.message, "dpi"))?;
    let current = captured.pin().clone();
    let mut sheets = Vec::new();
    let mut stale = Vec::new();
    let mut published = None;
    if let Some(m) = &manifest {
        published = serde_json::from_value::<SheetPublishPin>(m["source"].clone()).ok();
        let ids: Vec<String> = serde_json::from_value(m["sheets"].clone()).unwrap_or_default();
        let files: Vec<SheetPublishedFile> =
            serde_json::from_value(m["files"].clone()).unwrap_or_default();
        let pages: Vec<SheetPublishedPage> =
            serde_json::from_value(m["pages"].clone()).unwrap_or_default();
        let geo = m["options"]["geo"].as_bool().unwrap_or(true);
        let layers = m["options"]["layers"].as_bool().unwrap_or(true);
        let present: Vec<String> = ids
            .iter()
            .filter(|id| book.sheet(id).is_some())
            .cloned()
            .collect();
        let now = if present.is_empty() {
            Vec::new()
        } else {
            digests(&captured, &present, geo, layers, progress)?
        };
        // What each page's sources did since the delivery (docs/adr/1320 §2).
        let resources = published
            .as_ref()
            .is_some_and(|p| p.resources_sha256.is_some() && p.resources_sha256 != current.resources_sha256);
        let changes = crate::publish_update::changes(doc, &book, &m["sources"], &present, resources);
        let name_of = |id: &str| {
            m["drawingSet"]
                .as_array()
                .and_then(|a| a.iter().find(|s| s["id"] == id))
                .and_then(|s| s["name"].as_str())
                .unwrap_or(id)
                .to_owned()
        };
        for id in &ids {
            let file = files.iter().find(|f| f.sheets.contains(id));
            let page = pages.iter().find(|p| &p.sheet == id);
            let was = page.map(|p| p.digest.clone()).filter(|d| !d.is_empty());
            let digest = now.iter().find(|(s, _)| s == id).map(|(_, d)| d.clone());
            let mut row = SheetPublishStatus {
                sheet: id.clone(),
                name: book
                    .sheet(id)
                    .map_or_else(|| name_of(id), |s| s.name.clone()),
                state: SheetPublishState::Unknown,
                file: file.map(|f| f.name.clone()),
                published: was.clone(),
                digest: digest.as_ref().and_then(|d| d.as_ref().ok().cloned()),
                issue: None,
                date: page
                    .and_then(|p| p.date.clone())
                    .or_else(|| m["date"].as_str().map(str::to_owned)),
                changes: changes.iter().find(|(s, _)| s == id).map(|(_, c)| c.clone()),
            };
            row.state = if book.sheet(id).is_none() {
                SheetPublishState::Removed
            } else if file.is_none_or(|f| {
                std::fs::read(target.join(&f.name))
                    .ok()
                    .map(|b| sha256_hex(&b))
                    .as_deref()
                    != Some(f.sha256.as_str())
            }) {
                SheetPublishState::FileChanged
            } else if let Some(Err((code, message))) = &digest {
                row.issue = Some(issue(
                    id,
                    code,
                    message.clone(),
                    true,
                    "Paftanın kaynaklarını düzeltip yeniden denetleyin.".into(),
                ));
                SheetPublishState::Failed
            } else if was.is_none() {
                SheetPublishState::Unknown
            } else if row.digest == was {
                SheetPublishState::Current
            } else {
                SheetPublishState::Stale
            };
            if matches!(
                row.state,
                SheetPublishState::Stale
                    | SheetPublishState::FileChanged
                    | SheetPublishState::Failed
            ) {
                stale.push(id.clone());
            }
            sheets.push(row);
        }
        for s in &book.sheets {
            if !ids.contains(&s.id) {
                sheets.push(SheetPublishStatus {
                    sheet: s.id.clone(),
                    name: s.name.clone(),
                    state: SheetPublishState::Unpublished,
                    file: None,
                    published: None,
                    digest: None,
                    issue: None,
                    date: None,
                    changes: None,
                });
            }
        }
    }
    let stopped = journal.as_ref().map(|j| {
        let files: Vec<SheetPublishedFile> =
            serde_json::from_value(j["files"].clone()).unwrap_or_default();
        let chosen: Vec<String> = serde_json::from_value(j["sheets"].clone()).unwrap_or_default();
        let done: Vec<String> = files
            .iter()
            .filter(|f| !f.sha256.is_empty())
            .flat_map(|f| f.sheets.clone())
            .collect();
        let source: SheetPublishPin =
            serde_json::from_value(j["source"].clone()).unwrap_or_default();
        StoppedPublication {
            reason: if j["reason"] == "failed" {
                PublishStopReason::Failed
            } else {
                PublishStopReason::Cancelled
            },
            date: j["date"].as_str().unwrap_or_default().to_owned(),
            resumable: same_source(&source, &current),
            source,
            remaining: chosen
                .iter()
                .filter(|id| !done.contains(id))
                .cloned()
                .collect(),
            sheets: chosen,
            done,
            files,
            issues: serde_json::from_value(j["issues"].clone()).unwrap_or_default(),
        }
    });
    Ok(PublishStatusReport {
        path: input.path.clone(),
        state: if manifest.is_some() {
            PublishFolderState::Published
        } else if stopped.is_some() {
            PublishFolderState::Stopped
        } else {
            PublishFolderState::None
        },
        current,
        published,
        date: manifest
            .as_ref()
            .and_then(|m| m["date"].as_str().map(str::to_owned)),
        sheets,
        stale,
        stopped,
    })
}

pub(crate) fn run(
    doc: &Document,
    input: &PublishStatus,
    op: CallOp,
    progress: Option<&Progress>,
) -> CommandResult<serde_json::Value> {
    if matches!(op, CallOp::Validate) {
        return match Path::new(&input.path).is_absolute() {
            true => CommandResult::Completed {
                output: serde_json::Value::Null,
                warnings: Vec::new(),
            },
            false => crate::file_commands::refused_result(*refuse(
                "invalid_file_path",
                "Teslim klasörünün tam yolunu verin.",
                "path",
            )),
        };
    }
    match work(doc, input, progress) {
        Ok(report) => CommandResult::Completed {
            output: serde_json::to_value(report).unwrap_or_default(),
            warnings: Vec::new(),
        },
        Err(e) if e.code == "cancelled" => CommandResult::Cancelled,
        Err(e) => crate::file_commands::refused_result(*e),
    }
}
