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
/// The front drawing's view as an agent and the console read it (`view`; 14 November 2026, the owner: "ölçek birçok
/// yerde işimize yarayabilir, her yerden erişilebilir olmalı"): its scale 1:N as the status bar shows it
/// (`kentos_geometry_core::scale`), its pixels a metre, the box it shows and its middle in the project's plane, its
/// twist and the drawing's own scale.
pub(crate) fn view_answer(camera: &kentos_render_wgpu::camera::Camera, plot_scale: f64) -> Value {
    let b = camera.visible_bounds();
    let middle = camera.screen_to_world(camera.width / 2.0, camera.height / 2.0);
    json!({
        "scale": camera.screen_scale(),
        "pxPerMetre": camera.scale,
        "window": { "minX": b.min_x, "minY": b.min_y, "maxX": b.max_x, "maxY": b.max_y },
        "center": { "x": middle.x, "y": middle.y },
        "twist": camera.twist.to_degrees(),
        "plotScale": plot_scale,
    })
}

fn refused(code: &str, message: &str) -> (String, String) {
    (code.to_owned(), message.to_owned())
}

/// Whose workshop a link's request is, as the review and the notices say it: an agent on the link (MCP's), or Asistan.
fn who_agent(who: &str) -> &'static str {
    if who == "Asistan" {
        "Asistan"
    } else {
        "bağlı ajan"
    }
}

impl App {
    /// Where this program's link opens (docs/adr/0802): its own socket in the user's runtime folder, beside the
    /// other programs'.
    pub(crate) fn agents_path(&self) -> std::path::PathBuf {
        match &self.python.link_folder {
            Some(folder) => link::path_in(folder, self.program),
            None => link::default_path(self.program),
        }
    }

    /// The Python tab's link button: opens the link, or closes it.
    pub(crate) fn agents_toggle(&mut self) -> Task<Message> {
        if self.python.link.take().is_some() {
            self.python.agents.clear();
            self.python.push(Kind::Note, "Ajan bağlantısı kapatıldı.");
            return super::view::follow();
        }
        let path = self.agents_path();
        match Link::open(&path, |heard| Message::Python(Event::Agent(heard))) {
            Ok((open, task)) => {
                // A module's program says whose drawing it is and how an agent names this link.
                let whose = self.program.map_or_else(String::new, |m| {
                    format!(" {}", crate::program::in_program(m))
                });
                self.python.push(
                    Kind::Note,
                    format!(
                        "Ajan bağlantısı açık: {}. Bu kullanıcının programları (MCP'de {}){whose} açık çizimi okuyup komutlarla yazabilir; her yazma bir geri alma adımıdır.",
                        path.display(),
                        link::attach_words(self.program)
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

    /// Why the product commands a request runs are not this program's (docs/adr/8056 §1): `run`'s and a job's
    /// command, a call's envelope's, each row of a journal replayed, each step of a change set; none for a
    /// request that runs no command.
    fn offered_to_agent(&self, method: &str, params: &Value) -> Option<String> {
        let mut commands: Vec<&str> = match method {
            "run" | "call" | "start" => params
                .get("command")
                .and_then(Value::as_str)
                .into_iter()
                .collect(),
            "replay" => params
                .pointer("/journal/calls")
                .and_then(Value::as_array)
                .map(|calls| {
                    calls
                        .iter()
                        .filter_map(|c| c.get("command").and_then(Value::as_str))
                        .collect()
                })
                .unwrap_or_default(),
            _ => Vec::new(),
        };
        // A change set's steps are commands of their own: a step is not let in by the set (docs/adr/8056 §2).
        if commands.first() == Some(&kentos_commands::CAD_CHANGESET_APPLY)
            && let Some(steps) = params.pointer("/input/steps").and_then(Value::as_array)
        {
            commands.extend(
                steps
                    .iter()
                    .filter_map(|s| s.get("command").and_then(Value::as_str)),
            );
        }
        commands
            .into_iter()
            .find_map(|command| self.product_not_offered(command))
    }

    pub(crate) fn agents_answer(&mut self, request: &Value) -> Result<Value, (String, String)> {
        if self.python.link.is_none() {
            return Err(refused("closed", "Ajan bağlantısı kapatıldı."));
        }
        self.agent_request(request, "Ajan")
    }

    /// An agent's request on the drawings open here, whoever asks it: an agent on the link, or the desktop's own
    /// assistant (docs/adr/8056 §4), which needs no link. The same rules either way: the program's offer, the fields
    /// the drawing hides, a write one undo step said in the log under `who` (“Ajan: …”, “Asistan: …”).
    pub(crate) fn agent_request(
        &mut self,
        request: &Value,
        who: &str,
    ) -> Result<Value, (String, String)> {
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
        if let Some(writer) = self.extension_writing() {
            return Err(refused(
                "busy",
                &format!("Masaüstü şu an meşgul: {writer} çizime yazıyor. Biraz sonra yeniden deneyin."),
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
            Some(v) => Some(
                v.as_u64()
                    .or_else(|| v.as_str().and_then(|t| t.parse().ok()))
                    .ok_or_else(|| {
                        refused(
                            "invalid_request",
                            "document, hello'nun verdiği çizim numarasıdır (document).",
                        )
                    })?,
            ),
        };
        // A module's program runs only the product commands it offers (docs/adr/8056 §1), as its interface does.
        if let Some(why) = self.offered_to_agent(method, &params) {
            return Err(refused("not_offered", &why));
        }
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
        if method == "view" {
            // A drawing in a tab behind has no view on the screen: its camera waits with it.
            if named.is_some() && named != front {
                let behind = self
                    .doc_tabs
                    .parked
                    .iter()
                    .any(|p| Some(p.document.session) == named);
                return Err(if behind {
                    refused(
                        "not_in_front",
                        "Bu çizim arka sekmede; görünümü öne getirildiğinde okunur.",
                    )
                } else {
                    refused(
                        "document_closed",
                        "İstenen çizim kapatıldı ya da hiç açılmadı.",
                    )
                });
            }
            let doc = self
                .document
                .as_ref()
                .ok_or_else(|| refused("no_document", "Masaüstünde açık çizim yok."))?;
            return Ok(view_answer(
                &self.viewport.camera,
                doc.model.settings().plot_scale,
            ));
        }
        // An agent's extension workshop on the link (extensions/workshop.rs, docs/adr/8057 §30): its package opened
        // for the user's review (Kur is the user's), the changes of what the user installed from its folder loaded,
        // and what the user sees of it.
        match method {
            "review_workshop" | "reload_extension" => {
                let path = |key: &str| {
                    params
                        .get(key)
                        .and_then(Value::as_str)
                        .map(std::path::PathBuf::from)
                        .filter(|p| p.is_absolute())
                        .ok_or_else(|| {
                            refused(
                                "invalid_request",
                                &format!("{key} gerekli: mutlak bir yol."),
                            )
                        })
                };
                let answer = if method == "review_workshop" {
                    let (package, folder) = (path("package")?, path("folder")?);
                    self.agent_review_workshop(&package, &folder, who_agent(who))
                } else {
                    let id = params.get("id").and_then(Value::as_str).ok_or_else(|| {
                        refused("invalid_request", "id gerekli: eklentinin kimliği.")
                    })?;
                    let folder = path("folder")?;
                    self.ext_workshop_reload_from(id, &folder, who_agent(who))
                };
                return answer.map_err(|why| refused("invalid_input", &why));
            }
            "preview_extension" => {
                let id = params
                    .get("id")
                    .and_then(Value::as_str)
                    .ok_or_else(|| refused("invalid_request", "id gerekli: eklentinin kimliği."))?;
                return self
                    .ext_preview(id)
                    .map_err(|why| refused("invalid_input", &why));
            }
            _ => {}
        }
        // An extension's audit rules (extensions/audit_agents.rs, docs/adr/8057 §27): a run reads the front drawing.
        match method {
            "audits" => return Ok(self.ext_audit_rules()),
            "audit_report" => {
                let offset = params.get("offset").and_then(Value::as_u64).unwrap_or(0);
                return Ok(self.ext_audit_report(usize::try_from(offset).unwrap_or(usize::MAX)));
            }
            "audit" | "audit_fixed" => {
                if named.is_some() && named != front {
                    return Err(refused(
                        "not_in_front",
                        "Eklenti denetimi öndeki çizimde çalışır: çizimi öne getirin.",
                    ));
                }
                let rule = params.get("rule").and_then(Value::as_str).ok_or_else(|| {
                    refused(
                        "invalid_request",
                        "rule gerekli: audit.list_providers'ın komutu.",
                    )
                })?;
                let answer = if method == "audit" {
                    self.ext_audit_start_rule(rule)
                } else {
                    let finding = params.get("finding").and_then(Value::as_u64).unwrap_or(0);
                    self.ext_audit_fixed(rule, usize::try_from(finding).unwrap_or(0))
                };
                return answer.map_err(|(code, why)| (code.to_owned(), why));
            }
            _ => {}
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
                let before = doc.model.revision();
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
                // Whether it ran a call's execution to its end: a call's answer says it; a job's, by the op it was
                // started with (kept until it ends).
                let job = answer.get("id").and_then(Value::as_str).map(str::to_owned);
                let executed = match method {
                    "call" => {
                        execute
                            && answer.get("status").and_then(Value::as_str) == Some("completed")
                            && answer.get("replayed").and_then(Value::as_bool) != Some(true)
                    }
                    "start" | "job" | "cancel" => job.as_deref().is_some_and(|id| {
                        let phase = answer.get("phase").and_then(Value::as_str).unwrap_or("");
                        let going =
                            matches!(phase, "queued" | "computing" | "staged" | "committing");
                        if method == "start" && execute && going {
                            self.python.executing.insert(id.to_owned());
                        }
                        let executed = phase == "done"
                            && (self.python.executing.contains(id)
                                || (method == "start" && execute));
                        if !going {
                            self.python.executing.remove(id);
                        }
                        executed
                    }),
                    _ => false,
                };
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
                    // A job writes when it is asked after its computation ended (core/jobs.rs), not when it starts:
                    // a plan's job writes nothing, a refused one neither. The drawing's revision says it.
                    "job" => doc.model.revision() != before,
                    _ => false,
                };
                // A job's command is in its status, not in the request (its id).
                let command = params
                    .get("command")
                    .or_else(|| answer.get("command"))
                    .and_then(Value::as_str)
                    .unwrap_or("?")
                    .to_owned();
                // A processing run's chosen objects (its tool's selection: Vitrin's and İmar's TAKS choose the parcels
                // over the limit) are chosen on the screen once it executed, as the İşlemler window keeps them, a run
                // that only chooses too; a call's in its answer, a job's in its status's.
                if executed
                    && in_front
                    && matches!(
                        command.as_str(),
                        "cad.processing.run" | "cad.processing.batch"
                    )
                    && let Some(chosen) = answer
                        .pointer("/output/selected")
                        .or_else(|| answer.pointer("/answer/output/selected"))
                        .and_then(Value::as_array)
                    && !chosen.is_empty()
                {
                    let slots: Vec<_> = chosen
                        .iter()
                        .filter_map(Value::as_str)
                        .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                        .filter_map(|u| doc.model.slot_of(u))
                        .collect();
                    self.selection.set(slots);
                }
                if wrote && !in_front {
                    // Its drawing waits in its tab: the store on screen is another drawing's.
                    self.output(format!("{who}: {command} (arka sekmedeki çizimde)"));
                } else if wrote {
                    self.output(format!("{who}: {command}"));
                } else if method == "replay" {
                    self.output(format!("{who}: kayıt yeniden oynatıldı"));
                }
                Ok(answer)
            }
            other => Err(refused(
                "unknown_method",
                &format!(
                    "“{other}” masaüstünde yanıtlanmaz: hello, run, call, summary, layers, entities, entity, measure, select, selection, view, journal, replay, start, job, wait ya da cancel."
                ),
            )),
        }
    }
}
