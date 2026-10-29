//! The agents on the link (docs/adr/0137): what a program asks of the
//! drawing open here, answered on it by the headless host's `rpc`, as the
//! console's scripts are; each write is its own undo step and a line of the
//! command history (“Ajan: …”). While console code runs, or a drawing opens,
//! agents wait.
//!
//! With more than one drawing open (doc_tabs.rs, docs/adr/0351 §3) a call names its drawing by `document`, the
//! drawing's session as `hello` gives it: the drawing in front changing does not move a program to another drawing,
//! and a call on a drawing set aside is answered on it, in its tab. A drawing closed is `document_closed`. A call
//! without `document` is the drawing in front's, as before.

use iced::Task;
use serde_json::{Value, json};

use super::link::{self, Heard, Link};
use super::{Event, Kind};
use crate::app::{App, Message};

/// A refusal: its code and message.
fn refused(code: &str, message: &str) -> (String, String) {
    (code.to_owned(), message.to_owned())
}

impl App {
    /// The Python tab's link button: opens the link, or closes it.
    pub(crate) fn agents_toggle(&mut self) -> Task<Message> {
        if self.python.link.take().is_some() {
            self.python.agents.clear();
            self.python.push(Kind::Note, "Ajan bağlantısı kapatıldı.");
            return super::view::follow();
        }
        let path = link::default_path();
        match Link::open(&path, |heard| Message::Python(Event::Agent(heard))) {
            Ok((open, task)) => {
                self.python.push(
                    Kind::Note,
                    format!(
                        "Ajan bağlantısı açık: {}. Bu kullanıcının programları (MCP'de desktop.attach) açık çizimi okuyup komutlarla yazabilir; her yazma bir geri alma adımıdır.",
                        path.display()
                    ),
                );
                self.python.link = Some(open);
                Task::batch([task, super::view::follow()])
            }
            Err(e) => {
                self.python.push(Kind::Error, e);
                super::view::follow()
            }
        }
    }

    pub(crate) fn agents_heard(&mut self, heard: Heard) {
        match heard {
            Heard::Connected(id, answers) => {
                self.python.agents.insert(id, answers);
                self.python.push(Kind::Note, "Bir ajan bağlandı.");
            }
            Heard::Closed(id) => {
                if self.python.agents.remove(&id).is_some() {
                    self.python.push(Kind::Note, "Ajan ayrıldı.");
                }
            }
            Heard::Failed(e) => {
                self.python.link = None;
                self.python.agents.clear();
                self.python
                    .push(Kind::Error, format!("Ajan bağlantısı: {e}"));
            }
            Heard::Request(id, request) => {
                let answer = match self.agents_answer(&request) {
                    Ok(result) => json!({"id": request.get("id"), "ok": true, "result": result}),
                    Err((code, message)) => {
                        json!({"id": request.get("id"), "ok": false, "code": code, "message": message})
                    }
                };
                if let Some(to) = self.python.agents.get(&id) {
                    to.send(&answer);
                }
            }
        }
    }

    pub(crate) fn agents_answer(&mut self, request: &Value) -> Result<Value, (String, String)> {
        if self.python.link.is_none() {
            return Err(refused("closed", "Ajan bağlantısı kapatıldı."));
        }
        let Some(method) = request.get("method").and_then(Value::as_str) else {
            return Err(refused(
                "invalid_request",
                "İstek bir yöntem (method) adı taşımıyor.",
            ));
        };
        if self.python.running.is_some() || self.opening.is_some() {
            return Err(refused(
                "busy",
                "Masaüstü şu an meşgul (konsolda kod çalışıyor ya da çizim açılıyor): biraz sonra yeniden deneyin.",
            ));
        }
        let mut params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        // The drawing it names, taken off the parameters the host reads.
        let named = params
            .as_object_mut()
            .and_then(|m| m.remove("document"))
            .filter(|v| !v.is_null());
        let named = match named {
            None => None,
            Some(v) => Some(v.as_u64().or_else(|| v.as_str().and_then(|t| t.parse().ok())).ok_or_else(|| {
                refused("invalid_request", "document, hello'nun verdiği çizim numarasıdır (document).")
            })?),
        };
        let front = self.document.as_ref().map(|d| d.session);
        let documents: Vec<Value> = self
            .doc_tabs
            .parked
            .iter()
            .map(|p| &p.document)
            .chain(self.document.as_ref())
            .map(|d| {
                json!({
                    "document": d.session,
                    "name": d.model.name(),
                    "path": d.path.as_ref().map(|p| p.display().to_string()),
                    "front": Some(d.session) == front,
                })
            })
            .collect();
        if matches!(method, "select" | "selection") {
            let (doc, selected) = if named.is_none() || named == front {
                (self.document.as_ref().map(|d| &d.model), &mut self.selection)
            } else if let Some(parked) = self.doc_tabs.parked.iter_mut().find(|p| Some(p.document.session) == named) {
                (Some(&parked.document.model), &mut parked.selection)
            } else {
                return Err(refused("document_closed", "İstenen çizim kapatıldı ya da hiç açılmadı."));
            };
            let doc = doc.ok_or_else(|| refused("no_document", "Masaüstünde açık çizim yok."))?;
            return super::selection::answer(doc, selected, method, &params).map_err(|e| (e.code.to_owned(), e.message));
        }
        let doc = match named {
            None => self.document.as_mut(),
            Some(s) if Some(s) == front => self.document.as_mut(),
            Some(s) => self.doc_tabs.find_mut(s),
        };
        let Some(doc) = doc else {
            return Err(match named {
                Some(s) => refused(
                    "document_closed",
                    &format!("{s} numaralı çizim kapatıldı ya da hiç açılmadı; hello açık çizimleri sayar."),
                ),
                None => refused("no_document", "Masaüstünde açık çizim yok."),
            });
        };
        let in_front = Some(doc.session) == front;
        match method {
            "hello" => Ok(json!({
                "desktop": env!("CARGO_PKG_VERSION"),
                "document": doc.session,
                "name": doc.model.name(),
                "path": doc.path.as_ref().map(|p| p.display().to_string()),
                "revision": doc.model.revision().to_string(),
                "objects": doc.model.len(),
                "documents": documents,
            })),
            "run" | "summary" | "layers" | "entities" | "entity" | "measure" | "call"
            | "journal" | "replay" | "start" | "job" | "wait" | "cancel" => {
                // The desktop does not wait for a job on its interface thread: `wait` says how it is.
                let method = if method == "wait" { "job" } else { method };
                // An agent reads no value of a field the drawing hides (docs/adr/0738 §1).
                let mut answer = kentos_headless::rpc::call_for_agent(
                    &mut doc.model,
                    Some(&mut self.python.jobs),
                    doc.path.as_deref(),
                    method,
                    &params,
                )
                .map_err(|e| (e.code.to_owned(), e.message))?;
                if method == "summary"
                    && let Some(fields) = answer.as_object_mut()
                {
                    fields.insert("document".into(), json!(doc.session));
                    if let Some(path) = &doc.path {
                        fields.insert("path".into(), json!(path.display().to_string()));
                    }
                    fields.insert("legacy".into(), json!(doc.legacy));
                }
                let execute = params
                    .get("op")
                    .and_then(Value::as_str)
                    .is_none_or(|o| o == "execute");
                let wrote = match method {
                    "run" => {
                        params.get("op").and_then(Value::as_str) == Some("execute")
                            && answer.get("status").and_then(Value::as_str) == Some("completed")
                    }
                    "call" => {
                        execute
                            && answer.get("status").and_then(Value::as_str) == Some("completed")
                            && answer.get("replayed").and_then(Value::as_bool) != Some(true)
                    }
                    "start" => true,
                    _ => false,
                };
                if wrote && !in_front {
                    // Its drawing waits in its tab: the store on screen is another drawing's.
                    let command = params.get("command").and_then(Value::as_str).unwrap_or("?");
                    self.output(format!("Ajan: {command} (arka sekmedeki çizimde)"));
                } else if wrote {
                    let command = params.get("command").and_then(Value::as_str).unwrap_or("?");
                    self.output(format!("Ajan: {command}"));
                } else if method == "replay" {
                    self.output("Ajan: kayıt yeniden oynatıldı".to_owned());
                }
                Ok(answer)
            }
            other => Err(refused(
                "unknown_method",
                &format!(
                    "“{other}” masaüstünde yanıtlanmaz: hello, run, call, summary, layers, entities, entity, measure, select, selection, journal, replay, start, job, wait ya da cancel."
                ),
            )),
        }
    }
}
