//! An extension's audit rules (`entrypoints.audits`, `@ext.audit`; docs/adr/8057 §17, plan/eklenti.md ADIM 20).
//!
//! Pressed on the ribbon, a rule runs in its extension version's process on the drawing's layers as data
//! (`kentos_processing::external::audit_facts`: what a tool's inputs give, hidden fields masked) and on what KentOS
//! measured in it (`facts.project`, `kentos_processing::audit_facts`, docs/adr/8057 §36: invalid geometries, overlaps,
//! gaps), worked out on a reading copy on the run's own thread before the rule is asked. It has no drawing:
//! the pool refuses its requests, so it reads and answers, never writes. Its findings go to the bottom panel's
//! Eklenti denetimi tab, each with its severity, words and object: Göster selects the object and brings it into view;
//! Düzelt applies the rule's fix, a KentOS command, as one undo step named by the fix, and only for an extension
//! allowed to write (`document.write`): the fix is the extension's words, the press is the user's.
//!
//! The facts, the answer's reading and the provenance are a cloud job's own (docs/adr/8057 §42):
//! `kentos_processing::audit_job` (`facts_of`, `message`, `drawing_stamp`) and `kentos_contracts::audit_job`
//! (`findings_of`, `counts`, `digest_of`), so a rule run here and in a job on the same drawing reports alike.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::{Duration, Instant};

use iced::widget::{Column, button, container, row, scrollable};
use iced::{Center, Element, Fill, Task};
pub(crate) use kentos_commands::audit_job::Severity;
use kentos_commands::audit_job::digest_of;
use kentos_extensions::manifest::Capability;
use kentos_interaction::Level;
use kentos_processing::audit_job;
use kentos_ui::icon::{Tone, icon};
use kentos_ui::{label, style};
use serde_json::{Value, json};

use super::{Event, msg};
use crate::app::{App, Message};
use crate::icons::from_web;
use kentos_ui::widget::Toast;

/// The most objects a rule is given: past it the facts are cut and the rule is told (`facts.cut`); a cloud job's.
pub(crate) const MOST_OBJECTS: usize = audit_job::MOST_FEATURES;

/// The findings the tab lists; the rest are counted under it.
const LISTED: usize = 500;

/// How long a rule may take.
const TIMEOUT: Duration = Duration::from_secs(120);

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
    /// Findings past the most kept (`kentos_contracts::audit_job::MOST_FINDINGS`), counted.
    pub(crate) dropped: usize,
    /// The drawing was larger than [`MOST_OBJECTS`]: the rule saw its first objects.
    pub(crate) cut: bool,
    pub(crate) ms: u64,
    /// Why it gave no findings, when it failed.
    pub(crate) error: Option<String>,
    /// The drawing it read: a fix writes only to it.
    pub(crate) session: u64,
    /// Its provenance (plan ADIM 20's köken): the extension's version and package digest the rule ran from, and the
    /// drawing's revision it read; a report says by which rule's version it was made.
    pub(crate) version: String,
    pub(crate) digest: Option<String>,
    pub(crate) revision: u64,
    /// What the rule read, once it ran, as a cloud job's report says it (docs/adr/8057 §42): the drawing's content
    /// stamp (`drawing_stamp`) and the facts' digest (`digest_of`).
    pub(crate) drawing: Option<String>,
    pub(crate) facts: Option<String>,
}

/// What a rule read, worked out with its facts on the run's thread.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct AuditRead {
    /// The drawing was larger than [`MOST_OBJECTS`].
    pub(crate) cut: bool,
    pub(crate) drawing: String,
    pub(crate) facts: String,
}

/// A rule's run, ended in the background.
#[derive(Clone, Debug)]
pub(crate) struct AuditDone {
    pub(crate) ext: String,
    pub(crate) local: String,
    pub(crate) answer: Result<Value, String>,
    pub(crate) ms: u64,
    /// What it read; none when it stopped before its facts were worked out.
    pub(crate) read: Option<AuditRead>,
}

/// A rule's answer (`done`) read as a cloud job reads it (`kentos_contracts::audit_job::findings_of`): its findings
/// and how many were past the most kept, or why it gave none and its traceback.
pub(crate) fn findings_of(done: &Value) -> Result<(Vec<Finding>, usize), (String, Option<String>)> {
    let (found, dropped) = kentos_commands::audit_job::findings_of(done)?;
    let findings = found
        .into_iter()
        .map(|f| Finding {
            severity: f.severity,
            message: f.message,
            feature: f.feature_id,
            fix: f.fix.map(|x| Fix {
                title: x.title,
                command: x.command,
                input: x.args,
            }),
            fixed: false,
        })
        .collect();
    Ok((findings, usize::try_from(dropped).unwrap_or(usize::MAX)))
}

/// Why a rule's run did not start: a stable code (an agent reads it) and the words.
pub(crate) type Refusal = (&'static str, String);

impl App {
    /// Runs an extension's audit rule on the drawing's layers in the background; the bottom panel's Eklenti
    /// denetimi tab opens on it and its findings come as [`Event::AuditDone`]. Why it did not start is said.
    pub(crate) fn ext_audit_run(&mut self, ext: &str, local: &str) -> Task<Message> {
        let name = self.extension_name(ext);
        match self.ext_audit_start(ext, local) {
            Ok(task) => task,
            Err(("busy", _)) => {
                self.toast(
                    Toast::info(format!("{name}: bir denetim sürüyor"))
                        .body("Bitmesini bekleyin ya da Eklenti denetimi'nde Durdur'a basın."),
                );
                Task::none()
            }
            Err(("unknown_rule", why)) => {
                self.toast(Toast::warning(format!("{name}: denetim bulunamadı")).body(why));
                Task::none()
            }
            Err(("no_document", why)) => {
                self.output(why);
                Task::none()
            }
            Err((_, why)) => {
                self.toast(Toast::warning(format!("{name}: denetlenemedi")).body(why));
                Task::none()
            }
        }
    }

    /// Starts an extension's audit rule on the front drawing: the tab's state set and opened, its run's background
    /// task, or why not. The ribbon's button and an agent's `audit.run` start it alike (audit_agents.rs).
    pub(crate) fn ext_audit_start(
        &mut self,
        ext: &str,
        local: &str,
    ) -> Result<Task<Message>, Refusal> {
        if let Some(running) = self
            .extensions
            .audit
            .as_ref()
            .filter(|a| a.running.is_some())
        {
            return Err((
                "busy",
                format!(
                    "Bir denetim sürüyor ({} › {}): bitince yeniden deneyin.",
                    self.extension_name(&running.ext),
                    running.title
                ),
            ));
        }
        let entry = self
            .extensions
            .registry
            .as_ref()
            .and_then(|r| r.manifest(ext))
            .and_then(|m| m.entrypoints.audits.iter().find(|a| a.id == local).cloned());
        let Some(entry) = entry else {
            return Err((
                "unknown_rule",
                format!("“{local}” bu eklentinin bir denetimi değil."),
            ));
        };
        let Some(doc) = &self.document else {
            return Err(("no_document", "Açık çizim yok.".to_owned()));
        };
        #[cfg(test)]
        let given = self.extensions.test_audit_answer.take();
        #[cfg(not(test))]
        let given: Option<Result<Value, String>> = None;
        type Work = Box<dyn FnOnce(Value, &dyn Fn() -> bool) -> Result<Value, String> + Send>;
        let work: Work = match given {
            Some(answer) => Box::new(move |_, _| answer),
            None => {
                let job = self
                    .pool_job(ext, local, &entry.handler)
                    .map_err(|why| ("not_ready", why))?;
                Box::new(move |invoke, stopped| job.run(invoke, TIMEOUT, stopped))
            }
        };
        // The facts are worked out on the run's thread, from the drawing as it is now (ADR 8057 §36, §42).
        let copy = doc.model.reading_copy();
        let (version, digest) = self.extensions.registry.as_ref().map_or_else(
            || (String::new(), None),
            |r| {
                (
                    r.manifest(ext)
                        .map(|m| m.version.to_string())
                        .unwrap_or_default(),
                    r.digest(ext),
                )
            },
        );
        let revision = doc.model.revision();
        let stop = Arc::new(AtomicBool::new(false));
        self.extensions.audit = Some(AuditState {
            ext: ext.to_owned(),
            local: local.to_owned(),
            title: entry.title.clone(),
            running: Some(Arc::clone(&stop)),
            findings: Vec::new(),
            dropped: 0,
            cut: false,
            ms: 0,
            error: None,
            session: doc.session,
            version,
            digest,
            revision,
            drawing: None,
            facts: None,
        });
        self.bottom_tab = crate::bottom::BottomTab::ExtAudit;
        self.command_expanded = true;
        let (owner, rule) = (ext.to_owned(), local.to_owned());
        let began = Instant::now();
        Ok(crate::python_setup::in_background(
            move || {
                let stopped = || stop.load(Ordering::Relaxed);
                // The layers' objects and what KentOS measured, as a cloud job gives them (a measure that breaks on
                // some drawing leaves the rule the layers, `facts.project` then says it is missing).
                let Some(facts) = audit_job::facts_of(&copy, &stopped) else {
                    return (Err("Durduruldu.".to_owned()), None, 0);
                };
                let read = AuditRead {
                    cut: facts["cut"] == true,
                    drawing: audit_job::drawing_stamp(&copy),
                    facts: digest_of(&facts),
                };
                let answer = work(audit_job::message(facts), &stopped);
                (answer, Some(read), began.elapsed().as_millis() as u64)
            },
            (Err("Denetim cevapsız kaldı.".to_owned()), None, 0),
            move |(answer, read, ms)| {
                msg(Event::AuditDone(Box::new(AuditDone {
                    ext: owner.clone(),
                    local: rule.clone(),
                    answer,
                    ms,
                    read,
                })))
            },
        ))
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
        if let Some(read) = done.read {
            state.cut = read.cut;
            state.drawing = Some(read.drawing);
            state.facts = Some(read.facts);
        }
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
        // An agent waiting for it hears its report (Asistan's call answered now, docs/adr/8057 §27).
        self.assistant_audit_ended(&format!("ext.{}.{}", done.ext, done.local));
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
        self.ext_show_object(
            &uid,
            "Bulgunun nesnesi çizimde yok: silinmiş ya da başka bir çizim açık.",
        )
    }

    /// An object an extension named by its persistent id (a finding's, a panel row's), selected and zoomed to;
    /// `missing` said when the drawing has none of that id.
    pub(crate) fn ext_show_object(&mut self, uid: &str, missing: &str) -> Task<Message> {
        let slot = self.document.as_ref().and_then(|d| {
            kentos_domain::Uuid::parse_str(uid)
                .ok()
                .and_then(|u| d.model.slot_of(u))
        });
        match slot {
            Some(slot) => {
                self.selection.set([slot]);
                self.run("view.zoomSelection")
            }
            None => {
                self.output(missing);
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
        // A stopped extension's findings stay to be read; its fixes do not write (they did, 14 November 2026).
        if self
            .extensions
            .registry
            .as_ref()
            .is_none_or(|r| r.lease(&state.ext).is_none())
        {
            return Some(format!(
                "{} kapalı: düzeltmesi uygulanmaz; Eklentiler'den etkinleştirin.",
                self.extension_name(&state.ext)
            ));
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

    /// The last run's provenance in words (the extension's version, the rule, the package's digest, the drawing's
    /// revision), and what changed since when something did: the extension (its findings are an older rule's) or the
    /// drawing (fixes included).
    pub(crate) fn ext_audit_provenance(&self) -> Option<(String, Option<String>)> {
        let state = self.extensions.audit.as_ref()?;
        let name = self.extension_name(&state.ext);
        let short = state
            .digest
            .as_deref()
            .map(|d| {
                d.trim_start_matches("sha256:")
                    .chars()
                    .take(12)
                    .collect::<String>()
            })
            .map_or_else(String::new, |d| format!(" · paket {d}"));
        let words = format!(
            "Köken: {name} {} · {}{short} · çizim revizyonu {}",
            state.version, state.local, state.revision
        );
        let now = self.extensions.registry.as_ref().map(|r| {
            (
                r.manifest(&state.ext)
                    .map(|m| m.version.to_string())
                    .unwrap_or_default(),
                r.digest(&state.ext),
            )
        });
        let changed = match now {
            Some((version, digest)) if digest != state.digest => Some(if version != state.version {
                format!(
                    "Eklenti denetimden sonra değişti ({} → {version}): bulgular eski kural sürümüyle üretildi; \
                     yeniden denetleyin.",
                    state.version
                )
            } else {
                "Eklentinin paketi denetimden sonra değişti (aynı sürüm): bulgular eski kuralla üretildi; yeniden \
                 denetleyin."
                    .to_owned()
            }),
            _ => self
                .document
                .as_ref()
                .filter(|d| {
                    state.running.is_none()
                        && d.session == state.session
                        && d.model.revision() != state.revision
                })
                .map(|_| {
                    "Çizim denetimden sonra değişti (düzeltmeler dahil): yeniden denetlemek bulguları tazeler."
                        .to_owned()
                }),
        };
        Some((words, changed))
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
        if let Some((words, changed)) = self.ext_audit_provenance() {
            notes = notes.push(label::muted(words));
            if let Some(changed) = changed {
                notes = notes.push(label::caption(changed));
            }
        }
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
            let mut line = row![
                icon(from_web(Some(f.severity.code())))
                    .size(14.0)
                    .tone(tone),
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

/// "2 hata, 1 uyarı" — the findings counted by severity, as a cloud job's summary says them.
pub(crate) fn counts(findings: &[Finding]) -> String {
    kentos_commands::audit_job::counts(findings.iter().map(|f| f.severity))
}
