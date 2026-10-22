//! Bounded entity/property/handle samples for CAD losses, with real source lines.
use super::lexer::Lexer;
use crate::watch::{STOPPED, Watch};
use kentos_contracts::{ImportReport, ReportDetail, ReportItem, ReportSeverity};
use std::collections::{BTreeMap, BTreeSet};

fn property(item: &ReportItem) -> &'static str {
    match item.what.as_str() {
        "Birim" | "Blok birimi" => "units",
        "Kod sayfası" => "encoding",
        "Yazı stili (STYLE)" => "font",
        "Ölçü stili (DIMSTYLE)" => "dimensionStyle",
        "Kâğıt uzayı nesnesi" => "paperSpace",
        "Dış başvuru (XREF)" => "xref",
        "Kısıt" => "constraints",
        "Z kotu" | "Kot" => "elevation",
        "Genişlikli çoklu çizgi" | "Çoklu çizgi genişliği" => "width",
        _ => "entity",
    }
}

pub(super) fn enrich(
    report: &mut ImportReport,
    bytes: &[u8],
    watch: &mut dyn Watch,
) -> Result<(), String> {
    // Only the already bounded report samples need identities. No per-entity
    // index is allocated for a million-object file.
    let wanted: BTreeSet<u32> = report
        .skipped
        .iter()
        .chain(&report.notes)
        .flat_map(|r| r.lines.iter().copied())
        .collect();
    let mut sources = BTreeMap::new();
    if !wanted.is_empty() {
        let mut lex = Lexer::new(bytes);
        let mut current: Option<(u32, ReportDetail)> = None;
        let mut count = 0u64;
        while let Some(p) = lex.next()? {
            count += 1;
            if count.is_multiple_of(2048) && !watch.step(950, 1000) {
                return Err(STOPPED.into());
            }
            if p.code == 0 {
                if let Some((line, detail)) = current.take() {
                    sources.insert(line, detail);
                }
                if wanted.contains(&p.line) {
                    current = Some((
                        p.line,
                        ReportDetail {
                            entity_type: p.text().trim().to_uppercase(),
                            source_line: Some(p.line),
                            ..Default::default()
                        },
                    ));
                }
            } else if matches!(p.code, 5 | 105)
                && let Some((_, detail)) = &mut current
            {
                let id = p.text().trim();
                if !id.is_empty() && id.bytes().all(|b| b.is_ascii_hexdigit()) {
                    detail.source_id = Some(id.to_uppercase());
                }
            }
        }
        if let Some((line, detail)) = current {
            sources.insert(line, detail);
        }
    }
    for (items, severity) in [
        (&mut report.skipped, ReportSeverity::Error),
        (&mut report.notes, ReportSeverity::Warning),
    ] {
        for item in items {
            item.severity = Some(if item.count == 0 {
                ReportSeverity::Info
            } else {
                severity
            });
            let property = item.details.iter().find(|d| d.property.starts_with("polyline.loss.")).map(|d| d.property.clone()).unwrap_or_else(|| property(item).into());
            let existing = item.details.clone();
            let details: Vec<_> = item
                .lines
                .iter()
                .take(5)
                .filter_map(|line| sources.get(line).cloned())
                .map(|mut d| {
                    d.property = property.clone();
                    d
                })
                .collect();
            item.details = if details.is_empty() { existing } else { details };
        }
    }
    Ok(())
}
