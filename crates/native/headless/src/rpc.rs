//! A drawing's requests by name (docs/adr/0132): what a Python script in the
//! desktop's console asks of the drawing open there, answered here so the
//! desktop and the headless session answer alike. Each request is a method
//! and its JSON parameters; the answer is JSON, or the host's refusal.
//!
//! - `run` `{command, op, input, version?}`: a catalog command (`crate::dispatch`);
//! - `summary`: the drawing as a whole (name, revision, dirty, objects,
//!   settings, active layer, undo and redo);
//! - `layers`: the layer tree;
//! - `entities` `{layer?, kinds?, bbox?, after?, limit?}`: a page of objects;
//!   `bbox` is `[min_x, min_y, max_x, max_y]`;
//! - `entity` `{uid}`, `measure` `{uid}`: one object, its measures.
//!
//! - `call` `{command, op?, input, version?, requestId?, expected?}`: a
//!   command by the protocol (docs/adr/0253 §4): its `CallAnswer`, the
//!   summary and the plan's id with it;
//! - `journal`: the drawing's journal (`kentos.replay/1`); `replay`
//!   `{journal}`: runs a journal's commands again, its report;
//! - `start` `{…call}`, `job` `{id}`, `wait` `{id}`, `cancel` `{id}`: a
//!   command as a job (docs/adr/0253 §9), when the host keeps jobs ([`call_with`]).
//!
//! Undo, redo, groups and files belong to the host (the desktop keeps its
//! own history and its own Save); they are not requests here.

use std::collections::BTreeMap;

use kentos_contracts::{Bounds, CallEnvelope, JobStatus, ReplayJournal};
use kentos_domain::Document;
use kentos_native_application::jobs::Job;
use kentos_native_application::replay::{journal_of, replay_with};
use serde_json::{Value, json};

use crate::HeadlessError;
use crate::dispatch::{self, Op, call_envelope};
use crate::file_commands;
use crate::query::{self, Filter};

/// The jobs a host keeps (docs/adr/0253 §9), by id.
#[derive(Default)]
pub struct Jobs {
    list: BTreeMap<String, Job>,
    given: u64,
}

impl Jobs {
    /// Starts `envelope` as a job on a reading copy of `doc`; its status.
    pub fn start(&mut self, doc: &Document, envelope: CallEnvelope) -> JobStatus {
        self.given += 1;
        let id = format!("is-{}", self.given);
        let job = if file_commands::is_file_command(&envelope.command) {
            let (command, request) = (envelope.command.clone(), envelope.request_id.clone());
            Job::start_with(
                id.clone(),
                command,
                request,
                doc,
                file_commands::job_work(envelope),
            )
        } else {
            Job::start(id.clone(), doc, envelope)
        };
        let status = job.peek();
        self.list.insert(id, job);
        status
    }

    fn get(&mut self, id: &str) -> Result<&mut Job, HeadlessError> {
        self.list
            .get_mut(id)
            .ok_or_else(|| HeadlessError::new("unknown_job", format!("“{id}” kimlikli iş yok.")))
    }

    /// The job as it is now (its result written to `doc` once computed).
    pub fn status(&mut self, doc: &mut Document, id: &str) -> Result<JobStatus, HeadlessError> {
        Ok(self.get(id)?.status(doc))
    }

    /// Waits for the job and writes it to `doc`.
    pub fn wait(&mut self, doc: &mut Document, id: &str) -> Result<JobStatus, HeadlessError> {
        Ok(self.get(id)?.wait(doc))
    }

    /// Asks the job to stop; refused once it was written (`already_committed`).
    pub fn cancel(&mut self, doc: &mut Document, id: &str) -> Result<JobStatus, HeadlessError> {
        let job = self.get(id)?;
        let status = job.status(doc);
        if status.phase == kentos_contracts::JobPhase::Done {
            return Err(HeadlessError::new(
                "already_committed",
                "İş yazıldıktan sonra durdurulmak istendi; yazılan kalır. Geri almak için Geri al'ı kullanın.",
            ));
        }
        job.cancel();
        Ok(job.status(doc))
    }
}

/// Answers `method` with `params` on `doc`; the job requests are refused
/// (a host that keeps jobs answers with [`call_with`]).
pub fn call(doc: &mut Document, method: &str, params: &Value) -> Result<Value, HeadlessError> {
    call_with(doc, None, method, params)
}

/// Answers `method` with `params` on `doc`, the host's jobs in `jobs`.
pub fn call_with(
    doc: &mut Document,
    jobs: Option<&mut Jobs>,
    method: &str,
    params: &Value,
) -> Result<Value, HeadlessError> {
    match method {
        "call" => to_value(&call_envelope(doc, None, envelope(params)?)),
        "journal" => to_value(&journal_of(doc)),
        "replay" => {
            let journal: ReplayJournal =
                serde_json::from_value(params.get("journal").cloned().unwrap_or(Value::Null))
                    .map_err(|e| {
                        invalid(&format!("journal bir kentos.replay/1 kaydı olmalı: {e}"))
                    })?;
            to_value(&replay_with(
                doc,
                None,
                &journal,
                &mut crate::file_commands::replay_row,
            ))
        }
        "start" | "job" | "wait" | "cancel" => {
            let jobs = jobs.ok_or_else(|| {
                HeadlessError::new(
                    "unknown_method",
                    "Bu bağlantı iş tutmuyor; komutu call ile çağırın.",
                )
            })?;
            match method {
                "start" => to_value(&jobs.start(doc, envelope(params)?)),
                "job" => to_value(&jobs.status(doc, text(params, "id")?)?),
                "wait" => to_value(&jobs.wait(doc, text(params, "id")?)?),
                _ => to_value(&jobs.cancel(doc, text(params, "id")?)?),
            }
        }
        _ => plain(doc, method, params),
    }
}

/// A call's envelope from the request's parameters.
pub fn envelope(params: &Value) -> Result<CallEnvelope, HeadlessError> {
    serde_json::from_value(params.clone()).map_err(|e| {
        invalid(&format!(
            "çağrı okunamadı: {e}. command, op, input, version, requestId ve expected alanlarını denetleyin."
        ))
    })
}

fn plain(doc: &mut Document, method: &str, params: &Value) -> Result<Value, HeadlessError> {
    match method {
        "run" => {
            let command = text(params, "command")?;
            let op = Op::parse(text(params, "op")?)?;
            let version = match params.get("version") {
                None | Some(Value::Null) => None,
                Some(v) => Some(
                    v.as_u64()
                        .and_then(|v| u32::try_from(v).ok())
                        .ok_or_else(|| invalid("version bir sürüm numarası olmalı"))?,
                ),
            };
            let input = params.get("input").cloned().unwrap_or(Value::Null);
            dispatch::run(doc, command, version, op, input)
        }
        "summary" => Ok(summary(doc)),
        "layers" => to_value(doc.layers().nodes()),
        // The block definitions in the drawing's order (docs/adr/0144).
        "blocks" => to_value(&doc.blocks().iter().map(|b| &**b).collect::<Vec<_>>()),
        // The associative arrays in the order they were made (docs/adr/0300).
        "arrays" => to_value(&doc.arrays().iter().map(|a| &**a).collect::<Vec<_>>()),
        "entities" => {
            let filter = Filter {
                layer: optional_text(params, "layer")?,
                kinds: match params.get("kinds") {
                    None | Some(Value::Null) => None,
                    Some(Value::Array(items)) => Some(
                        items
                            .iter()
                            .map(|k| k.as_str().map(str::to_owned))
                            .collect::<Option<Vec<_>>>()
                            .ok_or_else(|| invalid("kinds bir metin listesi olmalı"))?,
                    ),
                    Some(_) => return Err(invalid("kinds bir metin listesi olmalı")),
                },
                bbox: match params.get("bbox") {
                    None | Some(Value::Null) => None,
                    Some(v) => Some(bounds(v)?),
                },
                after: optional_text(params, "after")?,
                limit: match params.get("limit") {
                    None | Some(Value::Null) => None,
                    Some(v) => Some(
                        v.as_u64()
                            .and_then(|v| usize::try_from(v).ok())
                            .ok_or_else(|| invalid("limit bir sayı olmalı"))?,
                    ),
                },
            };
            to_value(&query::page(doc, &filter)?)
        }
        "entity" => {
            let uid = text(params, "uid")?;
            let slot = query::slot_of(doc, uid)?;
            let entity = doc.get(slot).ok_or_else(|| {
                HeadlessError::new(
                    "unknown_object",
                    format!("{uid} kimlikli nesne bu çizimde yok."),
                )
            })?;
            Ok(json!({ "uid": uid, "stamp": doc.stamp(slot), "entity": entity }))
        }
        "measure" => {
            let method = query::Method::named(optional_text(params, "method")?.as_deref())?;
            to_value(&query::measure_by(doc, text(params, "uid")?, method)?)
        }
        "measureMany" => {
            let method = query::Method::named(optional_text(params, "method")?.as_deref())?;
            let uids: Vec<String> = params
                .get("uids")
                .and_then(Value::as_array)
                .ok_or_else(|| invalid("uids gerekli: nesne kimliklerinin listesi."))?
                .iter()
                .map(|u| u.as_str().map(str::to_owned))
                .collect::<Option<_>>()
                .ok_or_else(|| invalid("uids'in her öğesi bir nesne kimliği (metin) olmalı."))?;
            to_value(&query::measure_many(doc, &uids, method)?)
        }
        other => Err(HeadlessError::new(
            "unknown_method",
            format!(
                "“{other}” bir istek değil: run, call, summary, layers, entities, entity, measure, measureMany, journal, replay, start, job, wait ya da cancel."
            ),
        )),
    }
}

/// The drawing as a whole, without its objects: what `summary` answers.
pub fn summary(doc: &Document) -> Value {
    json!({
        "name": doc.name(),
        "revision": doc.revision().to_string(),
        "dirty": doc.is_dirty(),
        "objects": doc.len(),
        "settings": doc.settings(),
        "units": crate::query::units(doc.settings()),
        // The axes and angles the values and the windows speak (docs/adr/0256 §5).
        "conventions": crate::query::conventions(doc.settings()),
        // Where the view opens and the drawing's anchor, in the project's plane.
        "homeView": doc.home_view(),
        "origin": doc.origin(),
        "activeLayer": doc.layers().active(),
        "legacy": false,
        "canUndo": doc.can_undo(),
        "canRedo": doc.can_redo(),
        // The sheet book's sheets (docs/adr/0252 §2); null for a book the core cannot read.
        "sheets": kentos_native_application::sheets_edit::sheets_in(doc),
        // The project's style library, each item's id, kind, name and path (docs/adr/0314 §3).
        "library": kentos_native_application::library_edit::listed(doc),
        // The project's parts' state stamps (docs/adr/0253 §2): what a call expects of them.
        "stamps": {
            "layers": doc.layers_stamp(),
            "settings": doc.settings_stamp(),
            "styles": doc.styles_stamp(),
            "sheets": doc.sheets_stamp(),
        },
    })
}

fn to_value<T: serde::Serialize + ?Sized>(value: &T) -> Result<Value, HeadlessError> {
    serde_json::to_value(value).map_err(|e| {
        HeadlessError::new("unwritable_result", format!("yanıt JSON'a yazılamadı: {e}"))
    })
}

fn invalid(message: &str) -> HeadlessError {
    HeadlessError::new("invalid_input", message)
}

fn text<'a>(params: &'a Value, name: &str) -> Result<&'a str, HeadlessError> {
    params.get(name).and_then(Value::as_str).ok_or_else(|| {
        HeadlessError::new("invalid_input", format!("{name} gerekli, metin olarak."))
    })
}

fn optional_text(params: &Value, name: &str) -> Result<Option<String>, HeadlessError> {
    match params.get(name) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(_) => Err(HeadlessError::new(
            "invalid_input",
            format!("{name} metin olmalı."),
        )),
    }
}

fn bounds(v: &Value) -> Result<Bounds, HeadlessError> {
    let n: Option<Vec<f64>> = v
        .as_array()
        .map(|items| items.iter().filter_map(Value::as_f64).collect());
    match n.as_deref() {
        Some(&[min_x, min_y, max_x, max_y]) => Ok(Bounds {
            min_x,
            min_y,
            max_x,
            max_y,
        }),
        _ => Err(invalid(
            "bbox dört sayı olmalı: [min_x, min_y, max_x, max_y]",
        )),
    }
}
