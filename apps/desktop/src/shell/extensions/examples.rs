//! An extension's examples run on the desktop (docs/adr/8057 §48, §75; PY-TOOL-23): its tools' and its audit rules'.
//! The user's Eklentiler (Örnekleri çalıştır) and an agent's trial (`extension.workshop.trial` with `examples`) start
//! it alike, one at a time, in the trial's place. The run itself is `kentos_extension_tools::examples`, the command
//! line's `kentos kek test` too.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use kentos_extension_tools::examples;
use kentos_ui::widget::Toast;
use serde_json::{Value, json};

use super::audits::Refusal;
use super::trial::{RULE_TIMEOUT, TrialState};
use super::{Event, msg};
use crate::app::{App, Message};

impl App {
    /// The examples of `ext`'s tools started in the background (the trial's place: one at a time); `by_user` says the
    /// user asked (a toast and the log say how it went). The task to run, or why not.
    pub(crate) fn ext_examples_start(
        &mut self,
        ext: &str,
        by_user: bool,
    ) -> Result<(Value, iced::Task<Message>), Refusal> {
        if self
            .extensions
            .trial
            .as_ref()
            .is_some_and(|t| t.running.is_some())
        {
            return Err((
                "busy",
                "Bir deneme koşusu sürüyor: bitince yeniden deneyin.".to_owned(),
            ));
        }
        if let Some(Some(why)) = self.extensions.python_reasons.get(ext) {
            return Err(("not_ready", format!("Eklentinin kodu çalışmıyor: {why}")));
        }
        let stop = Arc::new(AtomicBool::new(false));
        let tools = examples::of(ext);
        let rules = self.ext_example_rules(ext, &stop)?;
        if tools.is_empty() && rules.is_empty() {
            return Err((
                "no_examples",
                format!(
                    "“{ext}” araçlarının ve denetim kurallarının örneği yok (araç tanımında examples, kuralın \
                     examples dosyası)."
                ),
            ));
        }
        let count: usize = tools.iter().map(|t| t.examples.len()).sum::<usize>()
            + rules.iter().map(|r| r.examples.len()).sum::<usize>();
        let version = self
            .extensions
            .registry
            .as_ref()
            .and_then(|r| r.manifest(ext))
            .map(|m| m.version.to_string())
            .unwrap_or_default();
        self.extensions.trial = Some(TrialState {
            ext: ext.to_owned(),
            running: Some(Arc::clone(&stop)),
            report: None,
            by_user,
        });
        let head = json!({ "kind": "examples", "extension": { "id": ext, "version": version } });
        let task = crate::python_setup::in_background_deep(
            move || examples::run(&tools, &rules, head, &stop),
            json!({ "status": "failed", "error": "Örnek koşusu cevapsız kaldı." }),
            |report| msg(Event::TrialDone(Box::new(report))),
        );
        Ok((
            json!({ "status": "running", "extension": ext, "examples": count }),
            task,
        ))
    }

    /// The audit rules of `ext` that give examples (docs/adr/8057 §75), each run in its version's process as Eklenti
    /// denetimi runs it, Durdur (`stop`) asked while it runs; why not, when a rule's examples cannot be read or its
    /// Python cannot run now.
    fn ext_example_rules(
        &self,
        ext: &str,
        stop: &Arc<AtomicBool>,
    ) -> Result<Vec<examples::Rule>, Refusal> {
        let Some(registry) = self.extensions.registry.as_ref() else {
            return Ok(Vec::new());
        };
        let mut rules = Vec::new();
        for (audit, read) in examples::rule_examples(registry, ext) {
            let list = read.map_err(|why| ("invalid_input", why))?;
            let job = self
                .pool_job(ext, &audit.id, &audit.handler)
                .map_err(|why| ("not_ready", why))?;
            let stop = Arc::clone(stop);
            rules.push(examples::Rule {
                id: format!("ext.{ext}.{}", audit.id),
                title: audit.title.clone(),
                examples: list,
                run: Box::new(move |invoke| {
                    job.run(invoke, RULE_TIMEOUT, &|| stop.load(Ordering::Relaxed))
                }),
            });
        }
        Ok(rules)
    }

    /// The user's run of the examples ended: how it went in a toast, each failed example in the extension's log.
    pub(crate) fn ext_examples_said(&mut self, report: &Value) {
        let ext = report["extension"]["id"]
            .as_str()
            .unwrap_or_default()
            .to_owned();
        let name = self.extension_name(&ext);
        let said = report["summary"]
            .as_str()
            .unwrap_or("Örnekler çalıştı.")
            .to_owned();
        let failed: Vec<&Value> = report["examples"]
            .as_array()
            .map(|l| l.iter().filter(|e| e["status"] != "passed").collect())
            .unwrap_or_default();
        for e in &failed {
            let problems: Vec<&str> = e["problems"]
                .as_array()
                .map(|p| p.iter().filter_map(Value::as_str).collect())
                .unwrap_or_default();
            let words = format!(
                "{} › {}: {}",
                e["label"].as_str().unwrap_or_default(),
                e["title"].as_str().unwrap_or_default(),
                problems.join("; ")
            );
            self.extension_failed(&ext, "examples", "example_failed", &words, None);
        }
        if failed.is_empty() {
            // A problem an earlier run left goes: the tools do what their examples say.
            self.extension_worked(&ext, "examples", "examples_passed", &said);
            self.toast(Toast::success(format!("{name}: örnekler")).body(said));
        } else {
            self.toast(Toast::warning(format!("{name}: örnekler")).body(said));
        }
    }
}
