//! A cancellable, output-free print inspection shared by the UI, typed Python and agents.
use kentos_commands::{PreflightSheets, SheetsPreflighted, SheetPublishIssue, CallOp, CommandResult};
use kentos_domain::Document;
use kentos_native_application::jobs::Progress;
use crate::file_commands::{Refused, refuse};
fn stop(progress: Option<&Progress>) -> Result<(), Refused> {
    if progress.is_some_and(Progress::stopped) {
        Err(refuse(
            "cancelled",
            "Baskı denetimi iptal edildi.",
            "sheets",
        ))
    } else {
        Ok(())
    }
}
pub(crate) fn work(
    doc: &Document,
    input: &PreflightSheets,
    progress: Option<&Progress>,
) -> Result<SheetsPreflighted, Refused> {
    stop(progress)?;
    let kept = doc.sheets().ok_or_else(|| {
        refuse(
            "no_sheets",
            "Denetlenecek pafta yok; önce bir pafta ekleyin.",
            "sheets",
        )
    })?;
    let (mut book, pictures) = kentos_sheet::project::read_project_sheets(kept)
        .map_err(|e| refuse("book_unreadable", e.message, "sheets"))?;
    let chosen = input
        .sheets
        .clone()
        .unwrap_or_else(|| book.sheets.iter().map(|s| s.id.clone()).collect());
    if chosen.is_empty() {
        return Err(refuse("no_sheets", "En az bir pafta seçin.", "sheets"));
    }
    let mut seen = std::collections::BTreeSet::new();
    for id in &chosen {
        if book.sheet(id).is_none() {
            return Err(refuse(
                "unknown_sheet",
                format!("“{id}” paftası yok."),
                "sheets",
            ));
        }
        if !seen.insert(id) {
            return Err(refuse(
                "invalid_input",
                "Bir pafta iki kez seçilemez.",
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
    let mut copy = doc.reading_copy();
    copy.set_sheets(
        kentos_sheet::project::project_sheets(&book, |h| {
            pictures
                .iter()
                .find(|(sha, _)| sha == h)
                .map(|(_, b)| b.clone())
        })
        .map_err(|e| refuse("book_unreadable", e.message, "sheets"))?,
    )
    .map_err(|e| refuse("book_unreadable", e, "sheets"))?;
    let user = kentos_plot::book::current_user();
    let date = kentos_plot::book::today();
    let render = kentos_plot::book::base_inputs(&copy, &user, &date);
    let mut issues = Vec::new();
    for (n, id) in chosen.iter().enumerate() {
        stop(progress)?;
        if let Some(p) = progress {
            p.set(n as u64, chosen.len() as u64);
        }
        let findings =
            kentos_plot::preflight::inspect(&copy, &book, id, &render, input.drawing.as_deref())
                .map_err(|e| refuse(&e.code, e.message, "sheets"))?;
        for f in findings {
            issues.push(SheetPublishIssue {
                sheet: id.clone(),
                code: f.code.clone(),
                message: f.message,
                fix: f.fix,
                error: f.severity == kentos_sheet::preflight::Severity::Error
                    || f.code == "map_unplaced",
            });
        }
        // Resolve live tables/labels, embed fonts and decode every used asset through the real writer too.
        if !issues.iter().any(|i| i.sheet == *id && i.error) {
            match kentos_plot::book::book_pdf_controlled(
                &copy,
                std::slice::from_ref(id),
                &user,
                &date,
                input.drawing.as_deref(),
                None,
                progress
                    .map(|p| p.span(n as u64, 1, chosen.len() as u64))
                    .as_ref(),
            ) {
                Ok(pdf) => {
                    for (_, f) in pdf.sheet_findings {
                        if !issues
                            .iter()
                            .any(|i| i.sheet == *id && i.code == f.code && i.message == f.message)
                        {
                            issues.push(SheetPublishIssue {
                                sheet: id.clone(),
                                code: f.code.clone(),
                                message: f.message,
                                fix: f.fix,
                                error: f.severity == kentos_sheet::preflight::Severity::Error
                                    || f.code == "map_unplaced",
                            });
                        }
                    }
                }
                Err(e) => {
                    if e.code == "cancelled" {
                        return Err(refuse(&e.code, e.message, "sheets"));
                    }
                    issues.push(SheetPublishIssue {
                        sheet: id.clone(),
                        code: e.code,
                        message: e.message,
                        fix: "Paftanın kaynaklarını düzeltip yeniden denetleyin.".into(),
                        error: true,
                    });
                }
            }
        }
    }
    stop(progress)?;
    if let Some(p) = progress {
        p.set(chosen.len() as u64, chosen.len() as u64);
    }
    Ok(SheetsPreflighted {
        source: crate::publish::pin(doc)?,
        sheets: chosen,
        ready: !issues.iter().any(|i| i.error),
        issues,
    })
}
pub(crate) fn run(
    doc: &Document,
    input: &PreflightSheets,
    op: CallOp,
    progress: Option<&Progress>,
) -> Result<CommandResult<serde_json::Value>, String> {
    match work(doc, input, progress) {
        Ok(report) => Ok(CommandResult::Completed {
            output: if matches!(op, CallOp::Validate) {
                serde_json::Value::Null
            } else {
                serde_json::to_value(report).map_err(|e| e.to_string())?
            },
            warnings: vec![],
        }),
        Err(e) => Ok(CommandResult::Failed { error: *e }),
    }
}
