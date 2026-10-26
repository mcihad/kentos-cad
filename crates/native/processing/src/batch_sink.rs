//! `cad.processing.batch` with a sink (docs/adr/0593 §1, plan/gis.md CBS-24): the rows run one after another on a
//! copy of the drawing, as `cad.processing.run` runs one, and what they made goes to the GeoPackage or the PostGIS
//! database in one write: a table for each layer the rows made (rows giving one output layer fill one table, as they
//! fill one layer on the drawing), each row's table outputs beside them with `sink.tables`. The drawing does not change. The batch's own inputs read from their
//! source are read once into the copy; a row's own are read for that row into layers of its own (docs/adr/0593 §2).
//! A row that stops is taken back alone with `onError: continue`, as on the drawing; a stop anywhere writes nothing.

use std::collections::{BTreeMap, BTreeSet};
use std::time::Instant;

use kentos_commands::{
    BatchOnError, BatchRowFailed, BatchRowRan, CommandError, CommandWarning, ProcessingBatch,
    ProcessingBatched, ProcessingRun, RunInput, RunSink, codes,
};
use kentos_domain::{Document, Slot};
use kentos_native_application::geopackage::TextTable;
use kentos_native_application::processing_host::{BatchEnd, BatchFlow};

use crate::command::{
    Digests, Halt, Lineage, Sunk, batch_record, in_row, now_ms, partial_inputs, ready, record_sink,
    request_value, row_label, row_run, run_on, sink_target, text_tables, write_sink,
};
use crate::registry::Registry;
use crate::source_inputs::{Staged, given, said, stage_inputs};
use crate::types::Tool;

/// The batch's own values as one run: what its source inputs are checked and read by.
pub(crate) fn common_run(input: &ProcessingBatch) -> ProcessingRun {
    ProcessingRun {
        environment: input.environment.clone(),
        tool: input.tool.clone(),
        parameters: input.parameters.clone(),
        selection: input.selection.clone(),
        view: input.view,
        sink: input.sink.clone(),
        drawing: input.drawing.clone(),
    }
}

/// A row that ran on the copy: its answer, the objects it made, its table outputs.
struct Made {
    index: usize,
    label: String,
    added: Vec<Slot>,
    texts: Vec<TextTable>,
}

/// The batch on a copy of `doc`, its results written to `sink` (only when `write`: a plan measures them on a passing
/// copy and asks a database only for its names).
pub(crate) fn batch_to_sink(
    doc: &Document,
    input: &ProcessingBatch,
    sink: &RunSink,
    tool: &Tool,
    write: bool,
    flow: &dyn BatchFlow,
) -> Result<BatchEnd, Box<CommandError>> {
    let registry = Registry::builtin();
    let clock = (now_ms(), Instant::now());
    let target = sink_target(sink)?;
    let total = input.rows.len() as u64;
    let stop = flow.stop_flag();
    let stopped = || flow.stopped();
    let mut copy = doc.reading_copy();
    // The batch's own inputs read from their source, once for every row.
    let (common, mut read) = match stage_inputs(&mut copy, &common_run(input), "", &stopped) {
        Ok(s) => s,
        Err(None) => return Ok(BatchEnd::Cancelled),
        Err(Some(e)) => return Err(e),
    };
    let base = ProcessingBatch {
        parameters: common.parameters,
        ..input.clone()
    };
    let output_labels: BTreeMap<String, String> = tool
        .outputs
        .iter()
        .map(|o| (o.name.clone(), o.label.clone()))
        .collect();
    let group = copy.begin_group("Toplu");
    let mut ran_rows: Vec<BatchRowRan> = Vec::new();
    let mut failed: Vec<BatchRowFailed> = Vec::new();
    let mut made: Vec<Made> = Vec::new();
    let mut warnings: Vec<CommandWarning> = Vec::new();
    let mut digests = Digests::new();
    for (i, row) in input.rows.iter().enumerate() {
        if flow.stopped() {
            return Ok(BatchEnd::Cancelled);
        }
        flow.progress(i as u64, total);
        let label = row_label(i, row);
        let at = copy.group_savepoint();
        let ran = (|| {
            let mut run = row_run(&base, row);
            run.sink = Some(sink.clone());
            // The row's own source inputs, into layers the row's number keeps apart.
            let (run, own) = match stage_inputs(&mut copy, &run, &format!("-{}", i + 1), &stopped) {
                Ok(s) => s,
                Err(None) => return Err(Halt::Cancelled),
                Err(Some(e)) => return Err(Halt::Refused(e)),
            };
            let ready = ready(&registry, &copy, &run).map_err(Halt::Refused)?;
            let whole: Vec<&str> = read.iter().chain(&own).map(|s| s.layer.as_str()).collect();
            for w in partial_inputs(&copy, &ready, &whole) {
                if !warnings.iter().any(|x| x.message == w.message) {
                    warnings.push(w);
                }
            }
            let (mut ran, added) = run_on(&mut copy, ready, &registry, &mut digests, stop.clone())?;
            let staged: Vec<&Staged> = read.iter().chain(&own).collect();
            for s in staged {
                said(&mut ran, std::slice::from_ref(s));
            }
            let mut given_run = row_run(input, row);
            given_run.sink = Some(sink.clone());
            given(&mut ran.record.parameters, &given_run);
            Ok((ran, added, own))
        })();
        match ran {
            Ok((mut ran, added, own)) => {
                // A table output is the row's: its name says which (`rows[1].summary`).
                let texts = if sink.tables {
                    let mut t = text_tables(&ran.outputs, &output_labels);
                    for x in &mut t {
                        x.output = format!("rows[{i}].{}", x.output);
                    }
                    t
                } else {
                    Vec::new()
                };
                ran.step = None;
                ran.revision = doc.revision().to_string();
                ran.record.document_revision = doc.revision().to_string();
                made.push(Made {
                    index: i,
                    label: label.clone(),
                    added,
                    texts,
                });
                read.extend(own);
                ran_rows.push(BatchRowRan {
                    index: i as u32,
                    label,
                    ran,
                });
            }
            Err(Halt::Cancelled) => return Ok(BatchEnd::Cancelled),
            Err(Halt::Refused(e)) if input.on_error == BatchOnError::Continue => {
                if let Some(at) = at {
                    copy.rollback_to(at);
                }
                let e = in_row(e, i, row);
                failed.push(BatchRowFailed {
                    index: i as u32,
                    label,
                    code: e.code.clone(),
                    message: e.message.clone(),
                });
            }
            Err(Halt::Refused(e)) => {
                return Err(Box::new(CommandError {
                    code: codes::BATCH_ROW_FAILED.into(),
                    message: format!(
                        "{label} çalışmadı: {} Toplu işin hiçbir sonucu yazılmadı.",
                        e.message
                    ),
                    path: Some(format!("rows[{i}]")),
                    ..CommandError::default()
                }));
            }
        }
    }
    copy.end_group(group);
    flow.progress(total, total);
    let mut record = batch_record(
        &registry,
        input,
        tool,
        &ran_rows,
        &failed,
        doc.revision().to_string(),
        doc.settings().srid,
        clock,
    );
    if !failed.is_empty() {
        warnings.push(rows_failed(&failed, ran_rows.is_empty()));
    }
    if ran_rows.is_empty() {
        // Every row stopped: nothing to write, the file is as it was.
        return Ok(BatchEnd::Ran(
            ProcessingBatched {
                tool: input.tool.clone(),
                summary: format!(
                    "Hiçbir satır çalışmadı; {} satır geri alındı, sonuç yazılmadı.",
                    failed.len()
                ),
                rows: ran_rows,
                failed,
                step: None,
                sink: None,
                sink_report: None,
                record,
                revision: doc.revision().to_string(),
            },
            warnings,
        ));
    }
    // The rows that made each table; two rows' table outputs of one name take the rows' labels.
    let rows_of = name_tables(&copy, &mut made);
    let added: Vec<Slot> = made.iter().flat_map(|m| m.added.iter().copied()).collect();
    let texts: Vec<TextTable> = made.into_iter().flat_map(|m| m.texts).collect();
    let mut inputs: Vec<RunInput> = Vec::new();
    for r in &ran_rows {
        for i in &r.ran.inputs {
            if !inputs.iter().any(|x| x.layer == i.layer) {
                inputs.push(i.clone());
            }
        }
    }
    let lineage = Lineage {
        tool: (&tool.id, &tool.label),
        request: request_value(input),
        inputs: &inputs,
        drawing: doc.revision().to_string(),
        run: &record,
    };
    match write_sink(&mut copy, &added, &texts, &target, write, &lineage, flow)? {
        Sunk::Cancelled => Ok(BatchEnd::Cancelled),
        Sunk::Done(done) => {
            let (exported, report, written) = *done;
            warnings.extend(written);
            record.sink = Some(record_sink(&target, &exported, &report, &rows_of));
            let objects: usize = ran_rows.iter().map(|r| r.ran.added.len()).sum();
            let mut summary = format!(
                "{} satır çalıştı: {objects} nesne {} {} tabloya yazıldı.",
                ran_rows.len(),
                place_words(&exported.path),
                exported.tables.len()
            );
            if !failed.is_empty() {
                let names: Vec<&str> = failed.iter().map(|f| f.label.as_str()).collect();
                summary.push_str(&format!(
                    " {} satır çalışmadı ve geri alındı ({}).",
                    failed.len(),
                    names.join(", ")
                ));
            }
            Ok(BatchEnd::Ran(
                ProcessingBatched {
                    tool: input.tool.clone(),
                    summary,
                    rows: ran_rows,
                    failed,
                    step: None,
                    sink: Some(exported),
                    sink_report: Some(report),
                    record,
                    revision: doc.revision().to_string(),
                },
                warnings,
            ))
        }
    }
}

/// The warning of the rows that stopped and were taken back (as on the drawing, docs/adr/0551 §1).
fn rows_failed(failed: &[BatchRowFailed], none_ran: bool) -> CommandWarning {
    let names: Vec<&str> = failed.iter().map(|f| f.label.as_str()).collect();
    let rest = if none_ran {
        "hiçbir satır yazılmadı"
    } else {
        "öteki satırlar yazıldı"
    };
    CommandWarning {
        code: codes::BATCH_ROWS_FAILED.into(),
        message: format!(
            "{} satır çalışıp durdu ve tek tek geri alındı ({}); {rest}.",
            failed.len(),
            names.join(", ")
        ),
        path: Some("rows".into()),
    }
}

/// The rows that made each table, by the layer's id and the output's name: a layer is one table whichever rows wrote
/// to it (rows giving the same output layer write to one layer, as on the drawing), and is the row's only when one row
/// made it; a table output is each row's own, its name taking the row's label when two rows give the same one
/// (“Özet (Kuzey)”).
fn name_tables(copy: &Document, made: &mut [Made]) -> BTreeMap<String, String> {
    let mut writers: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    for m in made.iter() {
        for s in &m.added {
            if let Some(e) = copy.get(*s) {
                writers
                    .entry(e.base().layer_id.clone())
                    .or_default()
                    .insert(m.index);
            }
        }
    }
    let mut rows_of: BTreeMap<String, String> = writers
        .into_iter()
        .filter(|(_, rows)| rows.len() == 1)
        .filter_map(|(id, rows)| Some((id, format!("rows[{}]", rows.first()?))))
        .collect();
    let mut names: BTreeMap<String, usize> = BTreeMap::new();
    for t in made.iter().flat_map(|m| &m.texts) {
        *names.entry(t.label.to_lowercase()).or_default() += 1;
    }
    for m in made.iter_mut() {
        let (index, label) = (m.index, m.label.clone());
        for t in &mut m.texts {
            if names[&t.label.to_lowercase()] > 1 {
                t.label = format!("{} ({label})", t.label);
            }
            rows_of.insert(t.output.clone(), format!("rows[{index}]"));
        }
    }
    rows_of
}

/// Where the tables went, as the summary says it: a file's name, a database's host and name.
fn place_words(path: &str) -> String {
    if kentos_commands::is_database(path) {
        let at = path.split_once("://").map_or(path, |(_, r)| r);
        let at = at.rsplit_once('@').map_or(at, |(_, h)| h);
        format!("{at} veritabanına")
    } else {
        let name = std::path::Path::new(path)
            .file_name()
            .map_or_else(|| path.to_owned(), |n| n.to_string_lossy().into_owned());
        format!("{name} dosyasına")
    }
}
