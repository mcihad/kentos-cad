//! A cancellable, output-free print inspection shared by the UI, typed Python and agents.
use crate::file_commands::{Refused, refuse};
use kentos_commands::{
    CallOp, CommandResult, PreflightSheets, SheetPublishIssue, SheetsPreflighted,
};
use kentos_domain::Document;
use kentos_native_application::jobs::Progress;
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
        // An atlas sheet is read page by page in each page's context, as the atlas's PDF draws it (GIS-031): its
        // maps' places and its @atlas_ fields come from the page; what is still missing is said with the page.
        if book.sheet(id).is_some_and(|sh| sh.atlas.is_some()) {
            atlas_issues(
                &copy,
                &book,
                id,
                &render,
                input.drawing.as_deref(),
                &user,
                &date,
                &mut issues,
            )?;
            continue;
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
/// The sheets' preflight on `doc` at once, with no progress to tell (a test's, a script's).
pub fn preflight(doc: &Document, input: &PreflightSheets) -> Result<SheetsPreflighted, Refused> {
    work(doc, input, None)
}

/// The pages most read of an atlas in one preflight; past them the rest are said not read.
const MOST_ATLAS_PAGES: usize = 200;

/// An atlas sheet's issues (GIS-031, docs/adr/1091): its pages planned as the atlas's PDF plans them, each read in its
/// own context; a finding is said once with the pages it is on, an error when it is one on any page.
#[allow(clippy::too_many_arguments)]
fn atlas_issues(
    doc: &Document,
    book: &kentos_sheet::model::SheetBook,
    id: &str,
    render: &kentos_sheet::display::RenderInputs,
    drawing: Option<&str>,
    user: &str,
    date: &str,
    issues: &mut Vec<SheetPublishIssue>,
) -> Result<(), Refused> {
    let asked = kentos_plot::atlas::AtlasInput {
        sheet: id,
        fields: None,
        theme: None,
        time: None,
        crs: None,
        pin: None,
        coverage: kentos_commands::AtlasCoverage::Drawing,
        user,
        date,
    };
    let planned = match kentos_plot::atlas::plan(doc, &asked) {
        Ok(p) => p,
        Err(e) => {
            issues.push(SheetPublishIssue {
                sheet: id.to_owned(),
                code: e.code,
                message: format!("Atlas paftası: {}", e.message),
                fix: "Atlas tanımını Atlası PDF yap penceresinde düzeltin.".into(),
                error: true,
            });
            return Ok(());
        }
    };
    let pages = planned.page_contexts();
    let mut said: Vec<(String, String, String, bool, Vec<String>)> = Vec::new();
    for (name, page) in pages.iter().take(MOST_ATLAS_PAGES) {
        let mut on_page = render.clone();
        on_page.atlas = Some(page.clone());
        let findings = kentos_plot::preflight::inspect(doc, book, id, &on_page, drawing)
            .map_err(|e| refuse(&e.code, e.message, "sheets"))?;
        for f in findings {
            let error =
                f.severity == kentos_sheet::preflight::Severity::Error || f.code == "map_unplaced";
            match said.iter_mut().find(|s| s.0 == f.code && s.1 == f.message) {
                Some(s) => {
                    s.3 |= error;
                    s.4.push(name.clone());
                }
                None => said.push((f.code, f.message, f.fix, error, vec![name.clone()])),
            }
        }
    }
    issues.push(SheetPublishIssue {
        sheet: id.to_owned(),
        code: "atlas_sheet".into(),
        message: format!(
            "Atlas paftası: {} sayfa, her biri kendi nesnesi ve alanlarıyla denetlendi{}.",
            pages.len(),
            if pages.len() > MOST_ATLAS_PAGES {
                format!(" (ilk {MOST_ATLAS_PAGES} sayfa)")
            } else {
                String::new()
            }
        ),
        fix: String::new(),
        error: false,
    });
    for (code, message, fix, error, on) in said {
        let shown: Vec<String> = on.iter().take(3).map(|n| format!("“{n}”")).collect();
        let more = if on.len() > 3 {
            format!(" ve {} sayfa daha", on.len() - 3)
        } else {
            String::new()
        };
        issues.push(SheetPublishIssue {
            sheet: id.to_owned(),
            code,
            message: format!("{message} (sayfa {}{more})", shown.join(", ")),
            fix,
            error,
        });
    }
    Ok(())
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
