//! Destek matrisi (`file.exchange.matrix` v1; plan/map-cad.md MAP-17, docs/adr/1880): the drawing's round trip through
//! each exchange format asked, as `file.exchange.check` makes it (`exchange_check::run`), and what each format kept,
//! approximated, converted, added or lost, aspect by aspect: the plan's support matrix “nesne ve özellik düzeyinde”,
//! for the drawing open rather than for a sample. A cell is read from its format's check by the rules below; the checks
//! themselves come with the answer. Nothing is written; each trip's temporary folder is removed by the check.
//!
//! - `objects`: objects not written (lost; the kinds and their counts);
//! - `kinds`: a kind read back as another (converted; `circle → polygon`);
//! - `curves`: curves read back as chords (approximated);
//! - `z`: heights lost (lost), else added (added);
//! - `m`: measures lost (lost);
//! - `attributes`: attributes lost (lost), else changed (converted), else added (added); the names;
//! - `style`: an object's own colour, weight or symbol lost (lost);
//! - `identity`: persistent ids lost (lost);
//! - `position`: vertices farther than the tolerance from the original's geometry (approximated; the greatest offset);
//! - `extra`: objects read back that match none (added).

use kentos_commands::{
    CallOp, CommandError, CommandResult, EXCHANGE_ASPECTS, ExchangeCell, ExchangeCheck, ExchangeChecked, ExchangeFormat,
    ExchangeMatrix, ExchangeMatrixed, ExchangeRow, ExchangeState, codes,
};
use kentos_domain::Document;
use serde_json::Value;

/// A loss's total count and its details, joined.
fn losses(r: &ExchangeChecked, kind: &str) -> (u32, Option<String>) {
    let found: Vec<_> = r.losses.iter().filter(|l| l.kind == kind).collect();
    let count = found.iter().map(|l| l.count).sum();
    let details: Vec<&str> = found.iter().filter_map(|l| l.detail.as_deref()).collect();
    (count, (!details.is_empty()).then(|| details.join(", ")))
}

fn cell(format: ExchangeFormat, state: ExchangeState, count: u32, detail: Option<String>) -> ExchangeCell {
    ExchangeCell { format, state, count, detail }
}

/// The first of the losses asked that happened, as its state; kept when none did.
fn first(r: &ExchangeChecked, asked: &[(&str, ExchangeState)]) -> ExchangeCell {
    for (kind, state) in asked {
        let (count, detail) = losses(r, kind);
        if count > 0 {
            return cell(r.format, *state, count, detail);
        }
    }
    cell(r.format, ExchangeState::Kept, 0, None)
}

/// A format's cell for an aspect, by the rules in this file's head.
pub fn cell_of(r: &ExchangeChecked, aspect: &str) -> ExchangeCell {
    use ExchangeState as S;
    match aspect {
        "objects" => {
            let count: u32 = r.not_written.iter().map(|k| k.count).sum();
            if count == 0 {
                cell(r.format, S::Kept, 0, None)
            } else {
                let detail = r.not_written.iter().map(|k| format!("{} {}", k.kind, k.count)).collect::<Vec<_>>().join(", ");
                cell(r.format, S::Lost, count, Some(detail))
            }
        }
        "kinds" => first(r, &[("kindChanged", S::Converted)]),
        "curves" => first(r, &[("chorded", S::Approximated)]),
        "z" => first(r, &[("zLost", S::Lost), ("zAdded", S::Added)]),
        "m" => first(r, &[("mLost", S::Lost)]),
        "attributes" => first(r, &[("attrLost", S::Lost), ("attrChanged", S::Converted), ("attrAdded", S::Added)]),
        "style" => first(r, &[("styleLost", S::Lost)]),
        "identity" => first(r, &[("identityLost", S::Lost)]),
        "position" => {
            let offset = r.numeric.offset.map(|o| format!("{o}"));
            if r.numeric.above > 0 {
                cell(r.format, S::Approximated, r.numeric.above, offset)
            } else {
                cell(r.format, S::Kept, 0, offset)
            }
        }
        _ => first(r, &[("extra", S::Added)]),
    }
}

/// The rows of the checks, in [`EXCHANGE_ASPECTS`]' order.
pub fn rows(checks: &[ExchangeChecked]) -> Vec<ExchangeRow> {
    EXCHANGE_ASPECTS
        .iter()
        .map(|a| ExchangeRow { aspect: (*a).to_owned(), cells: checks.iter().map(|r| cell_of(r, a)).collect() })
        .collect()
}

pub fn run(doc: &Document, input: &ExchangeMatrix, op: CallOp) -> CommandResult<Value> {
    let mut formats: Vec<ExchangeFormat> = Vec::new();
    for f in if input.formats.is_empty() { ExchangeFormat::ALL.to_vec() } else { input.formats.clone() } {
        if !formats.contains(&f) {
            formats.push(f);
        }
    }
    let ask = |format: ExchangeFormat| ExchangeCheck { format, layers: input.layers.clone(), tolerance: input.tolerance };
    // The question's own refusals once, as the check says them.
    if let Some(&f) = formats.first() {
        match crate::exchange_check::run(doc, &ask(f), CallOp::Validate) {
            CommandResult::Completed { .. } => {}
            other => return other,
        }
    }
    if op == CallOp::Validate {
        return CommandResult::Completed { output: Value::Null, warnings: Vec::new() };
    }
    let mut checks: Vec<ExchangeChecked> = Vec::new();
    for (i, &f) in formats.iter().enumerate() {
        match crate::exchange_check::run(doc, &ask(f), CallOp::Execute) {
            CommandResult::Completed { output, .. } => match serde_json::from_value::<ExchangeChecked>(output) {
                Ok(r) => checks.push(r),
                Err(e) => {
                    return CommandResult::Failed {
                        error: CommandError {
                            code: codes::EXCHANGE_UNREADABLE.into(),
                            message: format!("{} denetiminin cevabı okunamadı: {e}", f.title()),
                            path: Some(format!("formats[{i}]")),
                            ..CommandError::default()
                        },
                    };
                }
            },
            CommandResult::Failed { mut error } => {
                // The check's refusal, its path under the format's.
                error.path = Some(match error.path.take() {
                    Some(p) if !p.is_empty() => format!("formats[{i}].{p}"),
                    _ => format!("formats[{i}]"),
                });
                return CommandResult::Failed { error };
            }
            other => return other,
        }
    }
    let tolerance = checks.first().map_or(input.tolerance.unwrap_or(kentos_commands::EXCHANGE_TOLERANCE), |c| c.tolerance);
    let output = ExchangeMatrixed { tolerance, rows: rows(&checks), checks };
    CommandResult::Completed { output: serde_json::to_value(output).unwrap_or(Value::Null), warnings: Vec::new() }
}

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::DocumentSnapshotV2;

    fn drawing() -> Document {
        let text = include_str!("../../../../fixtures/exchange/v1/devir.json");
        let snapshot: DocumentSnapshotV2 = serde_json::from_str(text).expect("the drawing");
        Document::from_snapshot_v2(snapshot).expect("opens")
    }

    fn matrix(input: &ExchangeMatrix) -> ExchangeMatrixed {
        match run(&drawing(), input, CallOp::Execute) {
            CommandResult::Completed { output, .. } => serde_json::from_value(output).expect("the answer"),
            other => panic!("{other:?}"),
        }
    }

    fn state(m: &ExchangeMatrixed, aspect: &str, format: ExchangeFormat) -> ExchangeState {
        let row = m.rows.iter().find(|r| r.aspect == aspect).expect("the row");
        row.cells.iter().find(|c| c.format == format).expect("the cell").state
    }

    /// The sample drawing's matrix is ADR 1660's table: every format leaves the text out but DXF, turns the circle
    /// into another kind, GeoParquet and Shapefile chord the curves, GML and GeoParquet and DXF lose M, only GeoPackage
    /// and GeoParquet keep the ids.
    #[test]
    fn the_sample_s_matrix_is_adr_1660_s_table() {
        use ExchangeFormat as F;
        use ExchangeState as S;
        let m = matrix(&ExchangeMatrix::default());
        assert_eq!(m.checks.len(), 6);
        assert_eq!(m.rows.iter().map(|r| r.aspect.as_str()).collect::<Vec<_>>(), EXCHANGE_ASPECTS);
        for f in [F::Gpkg, F::Parquet, F::Fgb, F::Gml, F::Shp] {
            assert_eq!(state(&m, "objects", f), S::Lost, "{f:?}");
        }
        assert_eq!(state(&m, "objects", F::Dxf), S::Kept);
        for f in F::ALL {
            assert_eq!(state(&m, "kinds", f), S::Converted, "{f:?}");
        }
        for (f, chorded) in [(F::Gpkg, false), (F::Parquet, true), (F::Fgb, false), (F::Gml, false), (F::Shp, true), (F::Dxf, false)] {
            assert_eq!(state(&m, "curves", f) == S::Approximated, chorded, "{f:?}");
        }
        for (f, lost) in [(F::Gpkg, false), (F::Parquet, true), (F::Fgb, false), (F::Gml, true), (F::Shp, false), (F::Dxf, true)] {
            assert_eq!(state(&m, "m", f) == S::Lost, lost, "{f:?}");
        }
        for (f, lost) in [(F::Gpkg, false), (F::Parquet, false), (F::Fgb, true), (F::Gml, true), (F::Shp, true), (F::Dxf, true)] {
            assert_eq!(state(&m, "identity", f) == S::Lost, lost, "{f:?}");
        }
        // Within the tolerance everywhere: the vertices kept.
        for f in F::ALL {
            assert_eq!(state(&m, "position", f), S::Kept, "{f:?}");
        }
    }

    /// Formats asked are tried once each, in the order asked; the check's refusal comes with the format's path.
    #[test]
    fn formats_asked_once_and_refusals_under_their_path() {
        use ExchangeFormat as F;
        let m = matrix(&ExchangeMatrix { formats: vec![F::Dxf, F::Gpkg, F::Dxf], ..Default::default() });
        assert_eq!(m.checks.iter().map(|c| c.format).collect::<Vec<_>>(), [F::Dxf, F::Gpkg]);
        let bad = ExchangeMatrix { tolerance: Some(0.0), ..Default::default() };
        let CommandResult::Failed { error } = run(&drawing(), &bad, CallOp::Execute) else { panic!("refused") };
        assert_eq!(error.code, "invalid_tolerance");
        let gone = ExchangeMatrix { layers: vec!["yok".into()], ..Default::default() };
        let CommandResult::Failed { error } = run(&drawing(), &gone, CallOp::Execute) else { panic!("refused") };
        assert_eq!((error.code.as_str(), error.path.as_deref()), ("layer_not_found", Some("layers[0]")));
    }
}
