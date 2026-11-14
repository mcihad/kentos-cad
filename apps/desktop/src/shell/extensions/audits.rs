//! An extension's audit rules (`entrypoints.audits`, `@ext.audit`; docs/adr/8057 §17, plan/eklenti.md ADIM 20).
//!
//! Pressed on the ribbon, a rule runs in its extension version's process on the drawing's layers as data
//! (`kentos_processing::external::audit_facts`: what a tool's inputs give, hidden fields masked). It has no drawing:
//! the pool refuses its requests, so it reads and answers, never writes. Its findings go to the bottom panel's
//! Eklenti denetimi tab, each with its severity, words and object: Göster selects the object and brings it into view;
//! Düzelt applies the rule's fix, a KentOS command, as one undo step named by the fix, and only for an extension
//! allowed to write (`document.write`): the fix is the extension's words, the press is the user's.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use iced::widget::{Column, button, container, row, scrollable};
use iced::{Center, Element, Fill, Task};
use kentos_extensions::manifest::Capability;
use kentos_interaction::Level;
use kentos_ui::icon::{Tone, icon};
use kentos_ui::{label, style};
use serde_json::{Value, json};

use super::{Event, msg};
use crate::app::{App, Message};
use crate::icons::from_web;
use kentos_ui::widget::Toast;

/// The most objects a rule is given: past it the facts are cut and the rule is told (`facts.cut`).
pub(crate) const MOST_OBJECTS: usize = 50_000;

/// The most findings kept of a run; the rest are counted.
pub(crate) const MOST_FINDINGS: usize = 10_000;

/// The findings the tab lists; the rest are counted under it.
const LISTED: usize = 500;

/// How long a rule may take.
const TIMEOUT: Duration = Duration::from_secs(120);

/// How much a finding matters, as its rule says.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Severity {
    Error,
    Warning,
    Info,
}

impl Severity {
    fn word(self) -> &'static str {
        match self {
            Severity::Error => "Hata",
            Severity::Warning => "Uyarı",
            Severity::Info => "Bilgi",
        }
    }
}

/// A finding's fix: a KentOS command and its input, applied by the user's press.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Fix {
    pub(crate) title: String,
    pub(crate) command: String,
    pub(crate) input: Value,
}

/// One finding of a rule.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Finding {
    pub(crate) severity: Severity,
    pub(crate) message: String,
    /// The object it is about: its persistent id.
    pub(crate) feature: Option<String>,
    pub(crate) fix: Option<Fix>,
    /// Its fix was applied.
    pub(crate) fixed: bool,
}

/// A rule's run as the tab shows it.
#[derive(Clone, Debug)]
pub(crate) struct AuditState {
    pub(crate) ext: String,
    pub(crate) local: String,
    pub(crate) title: String,
    /// While it runs: what stops it.
    pub(crate) running: Option<Arc<AtomicBool>>,
    pub(crate) findings: Vec<Finding>,
    /// Findings past [`MOST_FINDINGS`], counted.
    pub(crate) dropped: usize,
    /// The drawing was larger than [`MOST_OBJECTS`]: the rule saw its first objects.
    pub(crate) cut: bool,
    pub(crate) ms: u64,
    /// Why it gave no findings, when it failed.
    pub(crate) error: Option<String>,
    /// The drawing it read: a fix writes only to it.
    pub(crate) session: u64,
}

/// A rule's run, ended in the background.
#[derive(Clone, Debug)]
pub(crate) struct AuditDone {
    pub(crate) ext: String,
    pub(crate) local: String,
    pub(crate) answer: Result<Value, String>,
    pub(crate) ms: u64,
}

/// A rule's answer (`done`) read: its findings and how many were past [`MOST_FINDINGS`], or why it gave none and
/// its traceback.
pub(crate) fn findings_of(done: &Value) -> Result<(Vec<Finding>, usize), (String, Option<String>)> {
    if done["ok"] != true {
        let message = done["message"].as_str().unwrap_or("Kural cevap vermedi.");
        let words = match done["exception"].as_str() {
            Some(kind) => format!("{kind}: {message}"),
            None if done["stopped"] == true => "Durduruldu.".to_owned(),
            None => message.to_owned(),
        };
        return Err((words, done["error"].as_str().map(str::to_owned)));
    }
    let result = &done["result"];
    if result["kind"] != "findings" {
        return Err(("Kural bulgu listesi döndürmedi.".to_owned(), None));
    }
    let list = result["findings"].as_array().cloned().unwrap_or_default();
    let dropped = list.len().saturating_sub(MOST_FINDINGS);
    let findings = list
        .iter()
        .take(MOST_FINDINGS)
        .filter_map(|f| {
            let severity = match f["severity"].as_str()? {
                "error" => Severity::Error,
                "warning" => Severity::Warning,
                "info" => Severity::Info,
                _ => return None,
            };
            let message: String = f["message"].as_str()?.chars().take(2000).collect();
            let fix = f.get("fix").and_then(|x| {
                Some(Fix {
                    title: x["title"].as_str()?.chars().take(120).collect(),
                    command: x["command"].as_str()?.to_owned(),
                    input: x.get("args").cloned().unwrap_or_else(|| json!({})),
                })
            });
            Some(Finding {
                severity,
                message,
                feature: f["featureId"].as_str().map(str::to_owned),
                fix,
                fixed: false,
            })
        })
        .collect();
    Ok((findings, dropped))
}

impl App {
    /// Runs an extension's audit rule on the drawing's layers in the background; the bottom panel's Eklenti
    /// denetimi tab opens on it and its findings come as [`Event::AuditDone`].
    pub(crate) fn ext_audit_run(&mut self, ext: &str, local: &str) -> Task<Message> {
        let name = self.extension_name(ext);
        if self
            .extensions
            .audit
            .as_ref()
            .is_some_and(|a| a.running.is_some())
        {
            self.toast(
                Toast::info(format!("{name}: bir denetim sürüyor"))
                    .body("Bitmesini bekleyin ya da Eklenti denetimi'nde Durdur'a basın."),
            );
            return Task::none();
        }
        let entry = self
            .extensions
            .registry
            .as_ref()
            .and_then(|r| r.manifest(ext))
            .and_then(|m| m.entrypoints.audits.iter().find(|a| a.id == local).cloned());
        let Some(entry) = entry else {
            self.toast(
                Toast::warning(format!("{name}: denetim bulunamadı"))
                    .body(format!("“{local}” bu eklentinin bir denetimi değil.")),
            );
            return Task::none();
        };
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let job = match self.pool_job(ext, local, &entry.handler) {
            Ok(job) => job,
            Err(why) => {
                self.toast(Toast::warning(format!("{name}: denetlenemedi")).body(why));
                return Task::none();
            }
        };
        let facts = kentos_processing::external::audit_facts(&doc.model, MOST_OBJECTS);
        let stop = Arc::new(AtomicBool::new(false));
        self.extensions.audit = Some(AuditState {
            ext: ext.to_owned(),
            local: local.to_owned(),
            title: entry.title.clone(),
            running: Some(Arc::clone(&stop)),
            findings: Vec::new(),
            dropped: 0,
            cut: facts["cut"] == true,
            ms: 0,
            error: None,
            session: doc.session,
        });
        self.bottom_tab = crate::bottom::BottomTab::ExtAudit;
        self.command_expanded = true;
        let (owner, rule) = (ext.to_owned(), local.to_owned());
        let began = Instant::now();
        crate::python_setup::in_background(
            move || {
                let answer = job.run(json!({ "kind": "audit", "facts": facts }), TIMEOUT, &|| {
                    stop.load(Ordering::Relaxed)
                });
                (answer, began.elapsed().as_millis() as u64)
            },
            (Err("Denetim cevapsız kaldı.".to_owned()), 0),
            move |(answer, ms)| {
                msg(Event::AuditDone(Box::new(AuditDone {
                    ext: owner.clone(),
                    local: rule.clone(),
                    answer,
                    ms,
                })))
            },
        )
    }

    /// A rule's run ended: its findings in the tab; a failure said with the extension's name and kept in its log.
    pub(crate) fn ext_audit_done(&mut self, done: AuditDone) {
        let Some(state) = self
            .extensions
            .audit
            .as_mut()
            .filter(|a| a.ext == done.ext && a.local == done.local)
        else {
            return;
        };
        let stopped = state
            .running
            .take()
            .is_some_and(|s| s.load(Ordering::Relaxed));
        state.ms = done.ms;
        let title = state.title.clone();
        let part = format!("audit:{}", done.local);
        let name = self.extension_name(&done.ext);
        let read = done
            .answer
            .map_err(|why| (why, None))
            .and_then(|v| findings_of(&v));
        match read {
            Ok((findings, dropped)) => {
                let said = counts(&findings);
                if let Some(state) = self.extensions.audit.as_mut() {
                    state.findings = findings;
                    state.dropped = dropped;
                }
                self.extension_worked(&done.ext, &part, "audit_done", &format!("{title}: {said}"));
                self.toast(Toast::success(format!("{name}: {title}")).body(said));
            }
            Err(_) if stopped => {
                if let Some(state) = self.extensions.audit.as_mut() {
                    state.error = Some("Durduruldu.".to_owned());
                }
                self.toast(Toast::info(format!("{name}: “{title}” durduruldu")));
            }
            Err((words, detail)) => {
                if let Some(state) = self.extensions.audit.as_mut() {
                    state.error = Some(words.clone());
                }
                self.extension_failed(&done.ext, &part, "audit_failed", &words, detail.as_deref());
            }
        }
    }

    /// Durdur: the rule is asked to stop, then its process is ended.
    pub(crate) fn ext_audit_stop(&mut self) {
        if let Some(stop) = self
            .extensions
            .audit
            .as_ref()
            .and_then(|a| a.running.as_ref())
        {
            stop.store(true, Ordering::Relaxed);
        }
    }

    /// Göster: the finding's object selected and brought into view.
    pub(crate) fn ext_audit_show(&mut self, i: usize) -> Task<Message> {
        let Some(uid) = self
            .extensions
            .audit
            .as_ref()
            .and_then(|a| a.findings.get(i))
            .and_then(|f| f.feature.clone())
        else {
            return Task::none();
        };
        let slot = self.document.as_ref().and_then(|d| {
            kentos_domain::Uuid::parse_str(&uid)
                .ok()
                .and_then(|u| d.model.slot_of(u))
        });
        match slot {
            Some(slot) => {
                self.selection.set([slot]);
                self.run("view.zoomSelection")
            }
            None => {
                self.output("Bulgunun nesnesi çizimde yok: silinmiş ya da başka bir çizim açık.");
                Task::none()
            }
        }
    }

    /// Why a finding's fix cannot be applied; none when it can.
    pub(crate) fn ext_audit_fix_refused(&self, i: usize) -> Option<String> {
        let state = self.extensions.audit.as_ref()?;
        let finding = state.findings.get(i)?;
        finding.fix.as_ref()?;
        if finding.fixed {
            return Some("Düzeltildi.".to_owned());
        }
        let writes = self
            .extensions
            .registry
            .as_ref()
            .and_then(|r| r.grant(&state.ext))
            .is_some_and(|g| g.capabilities.contains(&Capability::DocumentWrite));
        if !writes {
            return Some(
                "Düzeltme çizime yazar; bu eklentiye yazma izni (document.write) verilmedi."
                    .to_owned(),
            );
        }
        if self.document.as_ref().map(|d| d.session) != Some(state.session) {
            return Some("Denetlenen çizim artık açık değil.".to_owned());
        }
        None
    }

    /// Düzelt: the rule's fix, a KentOS command, applied as one undo step named by the fix.
    pub(crate) fn ext_audit_fix(&mut self, i: usize) {
        if let Some(why) = self.ext_audit_fix_refused(i) {
            self.output(why);
            return;
        }
        let Some((ext, fix)) = self
            .extensions
            .audit
            .as_ref()
            .and_then(|a| Some((a.ext.clone(), a.findings.get(i)?.fix.clone()?)))
        else {
            return;
        };
        let name = self.extension_name(&ext);
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let group = doc.model.begin_group(&fix.title);
        let answer = kentos_headless::rpc::call_for_agent(
            &mut doc.model,
            None,
            doc.path.as_deref(),
            "run",
            &json!({ "command": fix.command, "input": fix.input, "op": "execute" }),
        );
        // A refusal comes as an answer too: only a completed command is kept.
        let failed = match &answer {
            Ok(v) if v["status"] == "completed" || v.get("status").is_none() => None,
            Ok(v) => Some(
                v["error"]["message"]
                    .as_str()
                    .or_else(|| v["message"].as_str())
                    .unwrap_or("Komut yürütülmedi.")
                    .to_owned(),
            ),
            Err(e) => Some(e.to_string()),
        };
        match failed {
            None => {
                doc.model.end_group(group);
                if let Some(f) = self
                    .extensions
                    .audit
                    .as_mut()
                    .and_then(|a| a.findings.get_mut(i))
                {
                    f.fixed = true;
                }
                self.say(
                    Level::Success,
                    format!("{name}: {} — düzeltildi.", fix.title),
                );
            }
            Some(why) => {
                doc.model.cancel_group(group);
                self.toast(
                    Toast::warning(format!("{name}: düzeltilemedi"))
                        .body(format!("{} ({}): {why}", fix.title, fix.command)),
                );
            }
        }
    }

    /// Eklenti denetimi: the last rule's findings with Göster and Düzelt, or what to do first.
    pub(crate) fn ext_audit_tab(&self) -> Element<'_, Message> {
        let Some(state) = &self.extensions.audit else {
            return container(label::muted(
                "Bir eklentinin denetimini şeritteki düğmesinden çalıştırın: bulguları burada listelenir.",
            ))
            .padding(12)
            .into();
        };
        let name = self.extension_name(&state.ext);
        let status = if state.running.is_some() {
            "Denetleniyor…".to_owned()
        } else if let Some(error) = &state.error {
            format!("Bulgu yok: {error}")
        } else if state.findings.is_empty() {
            format!(
                "Bulgu yok: kural her şeyi uygun buldu ({:.1} sn).",
                state.ms as f64 / 1000.0
            )
        } else {
            format!(
                "{} ({:.1} sn)",
                counts(&state.findings),
                state.ms as f64 / 1000.0
            )
        };
        let action = if state.running.is_some() {
            button(label::body("Durdur"))
                .style(style::button::secondary)
                .on_press(msg(Event::AuditStop))
        } else {
            button(label::body("Yeniden denetle"))
                .style(style::button::secondary)
                .on_press(msg(Event::Run(
                    state.ext.clone(),
                    "audit".to_owned(),
                    state.local.clone(),
                )))
        };
        let head = row![
            icon(from_web(Some("standardsCheck"))).size(16.0),
            label::strong(format!("{name} › {}", state.title)),
            label::muted(status).width(Fill),
            action,
        ]
        .spacing(10)
        .align_y(Center);
        let mut notes = Column::new().spacing(2);
        if state.cut {
            notes = notes.push(label::muted(format!(
                "Çizim büyük: kurala ilk {MOST_OBJECTS} nesne verildi."
            )));
        }
        let hidden = state.findings.len().saturating_sub(LISTED) + state.dropped;
        if hidden > 0 {
            notes = notes.push(label::muted(format!("{hidden} bulgu daha listelenmedi.")));
        }
        let mut list = Column::new().spacing(6);
        for (i, f) in state.findings.iter().take(LISTED).enumerate() {
            let tone = match f.severity {
                Severity::Error => Tone::Danger,
                Severity::Warning => Tone::Warning,
                Severity::Info => Tone::Accent,
            };
            let glyph = match f.severity {
                Severity::Error => "error",
                Severity::Warning => "warning",
                Severity::Info => "info",
            };
            let mut line = row![
                icon(from_web(Some(glyph))).size(14.0).tone(tone),
                label::muted(f.severity.word()),
                label::body(f.message.clone()).width(Fill),
            ]
            .spacing(8)
            .align_y(Center);
            if f.feature.is_some() {
                line = line.push(
                    button(label::body("Göster"))
                        .style(style::button::secondary)
                        .on_press(msg(Event::AuditShow(i))),
                );
            }
            if let Some(fix) = &f.fix {
                let refused = self.ext_audit_fix_refused(i);
                let face = if f.fixed {
                    format!("{} ✓", fix.title)
                } else {
                    format!("Düzelt: {}", fix.title)
                };
                let pressed = button(label::body(face))
                    .style(style::button::secondary)
                    .on_press_maybe(refused.is_none().then(|| msg(Event::AuditFix(i))));
                line = line.push(match refused.filter(|_| !f.fixed) {
                    Some(why) => kentos_ui::widget::tip(
                        pressed,
                        kentos_ui::widget::Tip::new(why),
                        iced::widget::tooltip::Position::Top,
                    ),
                    None => Element::from(pressed),
                });
            }
            list = list.push(line);
        }
        container(
            Column::new().spacing(8).push(head).push(notes).push(
                // The bar beside the rows, not over their buttons.
                scrollable(container(list).padding(iced::Padding::new(0.0).right(14.0)))
                    .direction(style::field::body_scrollbar())
                    .height(Fill),
            ),
        )
        .padding([8, 12])
        .width(Fill)
        .height(Fill)
        .into()
    }
}

/// "2 hata, 1 uyarı" — the findings counted by severity.
fn counts(findings: &[Finding]) -> String {
    let n = |s: Severity| findings.iter().filter(|f| f.severity == s).count();
    let parts: Vec<String> = [
        (n(Severity::Error), "hata"),
        (n(Severity::Warning), "uyarı"),
        (n(Severity::Info), "bilgi"),
    ]
    .into_iter()
    .filter(|(k, _)| *k > 0)
    .map(|(k, w)| format!("{k} {w}"))
    .collect();
    if parts.is_empty() {
        "Bulgu yok".to_owned()
    } else {
        parts.join(", ")
    }
}
