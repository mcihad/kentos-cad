//! CAD-IO-15: source integrity is the default. An impossible 2D conversion
//! fails before the host touches a drawing or publishes a file.
use super::preflight::ReadError;
use kentos_contracts::{DxfWriteInput, ExportReport, ReportItem};

pub const LOSS: &str = kentos_contracts::codes::POLYLINE_GEOMETRY_LOSS;

pub fn refusal<'a>(mut items: impl Iterator<Item = &'a ReportItem>) -> Option<ReadError> {
    let item = items.find(|r| {
        r.details
            .iter()
            .any(|d| d.property.starts_with("polyline.loss."))
    })?;
    let detail = item
        .details
        .iter()
        .find(|d| d.property.starts_with("polyline.loss."))?;
    let source = detail
        .source_id
        .as_ref()
        .map(|s| format!(" Kaynak kimliği: {s}."))
        .unwrap_or_default();
    let line = detail
        .source_line
        .map(|n| format!(" Satır: {n}."))
        .unwrap_or_default();
    Some(ReadError {
        code: LOSS,
        message: format!(
            "Çoklu çizgi kayıpsız aktarılamıyor ({}): {}.{source}{line} Kaynağı düzeltin, blokları patlatmadan aktarın veya rapordaki kaybı kabul ediyorsanız allowPolylineLoss=true seçin.",
            detail.property,
            item.reason.trim_end_matches('.')
        ),
    })
}

/// The checked writer used by product commands. `write` remains the low-level
/// compatibility primitive returning its loss report to callers.
pub fn write_checked(input: &DxfWriteInput) -> Result<(Vec<u8>, ExportReport), ReadError> {
    let (bytes, report) = super::write(input);
    if !input.allow_polyline_loss
        && let Some(error) = refusal(report.notes.iter().chain(&report.skipped))
    {
        return Err(error);
    }
    Ok((bytes, report))
}
