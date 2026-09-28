//! The agents on the link (docs/adr/0137): what a program asks of the
//! drawing open here, answered on it by the headless host's `rpc`, as the
//! console's scripts are; each write is its own undo step and a line of the
//! command history (“Ajan: …”). While console code runs, or a drawing opens,
//! agents wait.

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

    fn agents_answer(&mut self, request: &Value) -> Result<Value, (String, String)> {
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
        let params = request.get("params").cloned().unwrap_or_else(|| json!({}));
        let Some(doc) = self.document.as_mut() else {
            return Err(refused("no_document", "Masaüstünde açık çizim yok."));
        };
        match method {
            "hello" => Ok(json!({
                "desktop": env!("CARGO_PKG_VERSION"),
                "name": doc.model.name(),
                "path": doc.path.as_ref().map(|p| p.display().to_string()),
                "revision": doc.model.revision().to_string(),
                "objects": doc.model.len(),
            })),
            "run" | "summary" | "layers" | "entities" | "entity" | "measure" => {
                let mut answer = kentos_headless::rpc::call(&mut doc.model, method, &params)
                    .map_err(|e| (e.code.to_owned(), e.message))?;
                if method == "summary"
                    && let Some(fields) = answer.as_object_mut()
                {
                    if let Some(path) = &doc.path {
                        fields.insert("path".into(), json!(path.display().to_string()));
                    }
                    fields.insert("legacy".into(), json!(doc.legacy));
                }
                let wrote = method == "run"
                    && params.get("op").and_then(Value::as_str) == Some("execute")
                    && answer.get("status").and_then(Value::as_str) == Some("completed");
                if wrote {
                    let command = params.get("command").and_then(Value::as_str).unwrap_or("?");
                    self.output(format!("Ajan: {command}"));
                }
                Ok(answer)
            }
            other => Err(refused(
                "unknown_method",
                &format!(
                    "“{other}” masaüstünde yanıtlanmaz: hello, run, summary, layers, entities, entity ya da measure."
                ),
            )),
        }
    }
}
