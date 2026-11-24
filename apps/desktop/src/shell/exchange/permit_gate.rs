//! Ruhsat yayınının kapısı (docs/adr/8065; plan/bim-plan.md BIM-04; Md. 9/1, EK-9 Tablo 9.1 satır 21): Paftaları toplu
//! yayınla, Ruhsat takımından açıldığında, paftaların modelden üretildiğini denetletir ve kapı kapalıyken Yayınla'yı
//! açmaz. Karar standart kural paketinin Python kuralıdır (`dijital_proje.ruhsat.denetle`, bim-plan K10): eski kesit ve
//! görünüş, işareti silinmiş kesit, model katmanında serbest çizim, kopmuş etiket ve ölçü (ADR 8065); PDF/A seçilmemiş,
//! bir dosyada birden çok pafta, A4 ve katları dışında kâğıt, EK-2'ye uymayan dosya adı (ADR 8066). Olgular çizimin `cad`
//! bölümü ve denetimin raporundan paket bölümüdür.
//!
//! - **Her Denetle'nin raporu gelince sorulur;** cevap sorulduğu andaki çizimin kuşağını taşır: çizim değiştiyse kapı
//!   yeniden sorulana dek kapalıdır.
//! - **Kural çalışamazsa kapı kapalıdır** ve nedeni yazılır: ruhsat paftasının modelden geldiği denetlenmeden yayın olmaz.
//! - **Bulgunun yeri Denetim'dir:** "Denetim'de göster" pencereyi kapatıp Mimari denetimi açar; düzeltmeler oradadır.

use iced::widget::{button, column, row};
use iced::{Element, Fill, Task};
use kentos_ui::{label, style};
use serde_json::Value;

use super::sheet_publish::Event;
use super::{Event as Exchange, Window, message};
use crate::app::{App, Message};

const PACKAGE: &str = "dijital-proje";
const MODULE: &str = "dijital_proje.ruhsat";
const FUNCTION: &str = "denetle";

/// The gate's state in a permit publication's window.
#[derive(Clone, Debug, Default)]
pub struct Gate {
    /// The drawing's generation the last ask was made on.
    asked_on: Option<u64>,
    /// The answer to it has come.
    answered: bool,
    /// The rule's findings: each closes the gate.
    pub findings: Vec<Shut>,
    /// Why the rule could not run.
    pub failed: Option<String>,
    /// A check was asked: the gate asks the rule when its report comes.
    due: bool,
}

/// A finding that keeps the gate shut.
#[derive(Clone, Debug)]
pub struct Shut {
    pub message: String,
    pub repair: String,
}

impl Gate {
    /// Open for the drawing at `generation`: asked on it, answered, nothing found.
    pub fn open(&self, generation: u64) -> bool {
        self.asked_on == Some(generation) && self.answered && self.failed.is_none() && self.findings.is_empty()
    }
}

fn event(e: Event) -> Message {
    message(Exchange::SheetPublish(e))
}

impl App {
    /// A check asked: the gate shut until its report comes and the rule answers.
    pub(crate) fn permit_gate_wait(&mut self) {
        if let Some(gate) = self.permit_gate_mut() {
            gate.asked_on = None;
            gate.answered = false;
            gate.due = true;
        }
    }

    /// Whether the check's report came and the gate has yet to ask; asking takes it.
    pub(crate) fn permit_gate_due(&mut self) -> bool {
        let ready = matches!(&self.exchange, Some(Window::SheetPublish(s)) if s.planned().is_some());
        match self.permit_gate_mut() {
            Some(gate) if gate.due && ready => {
                gate.due = false;
                true
            }
            _ => false,
        }
    }

    /// The facts the rule reads: the drawing's `cad` section and, from the check's report, the package — the files it
    /// would write, the sheets with their papers, PDF/A or not, one file a sheet or one for all (docs/adr/8066).
    fn permit_gate_facts(&self) -> Option<Value> {
        // The drawing's `cad` section, measured as Mimari denetimi's Dijital proje rules read it.
        let cad = kentos_processing::audit_facts::cad::measure(&self.document.as_ref()?.model);
        let mut facts = serde_json::to_value(
            kentos_extensions::audit_facts::ProjectAuditFacts::new(0.0, Vec::new(), false).with_cad(cad),
        )
        .ok()?;
        if let Some(Window::SheetPublish(s)) = &self.exchange
            && let Some(report) = s.planned()
        {
            facts["package"] = serde_json::json!({
                "pdfa": s.input.pdfa,
                "mode": match s.input.mode {
                    kentos_commands::SheetPublishMode::Separate => "separate",
                    kentos_commands::SheetPublishMode::Single => "single",
                },
                "files": report.files.iter().map(|f| serde_json::json!({"name": f.name, "sheets": f.sheets})).collect::<Vec<_>>(),
                "sheets": report.scope.iter().map(|sc| serde_json::json!({
                    "id": sc.sheet, "number": sc.number, "name": sc.name, "revision": sc.revision, "paper": sc.paper,
                })).collect::<Vec<_>>(),
            });
        }
        Some(facts)
    }

    fn permit_gate_mut(&mut self) -> Option<&mut Gate> {
        match &mut self.exchange {
            Some(Window::SheetPublish(s)) => s.permit.as_mut(),
            _ => None,
        }
    }

    /// The rule asked for the drawing as it is; its answer comes as [`Event::Gate`].
    pub(crate) fn permit_gate_ask(&mut self) -> Task<Message> {
        let generation = self.document.as_ref().map(|d| d.model.generation());
        let facts = self.permit_gate_facts();
        let (Some(generation), Some(facts)) = (generation, facts) else {
            return Task::none();
        };
        let Some(gate) = self.permit_gate_mut() else {
            return Task::none();
        };
        gate.asked_on = Some(generation);
        gate.answered = false;
        Task::perform(
            async move {
                let (sent, got) = iced::futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    let _ = sent.send(crate::standards::call(PACKAGE, MODULE, FUNCTION, vec![facts]));
                });
                got.await.unwrap_or_else(|_| Err("Kuralın süreci cevap vermeden kapandı.".to_owned()))
            },
            move |answer| event(Event::Gate(generation, answer)),
        )
    }

    /// The rule's answer for the drawing at `generation`; an answer to an older ask is dropped.
    pub(crate) fn permit_gate_answer(&mut self, generation: u64, answer: Result<Value, String>) {
        let Some(gate) = self.permit_gate_mut() else {
            return;
        };
        if gate.asked_on != Some(generation) {
            return;
        }
        gate.answered = true;
        match answer {
            Ok(found) => {
                gate.failed = None;
                gate.findings = found
                    .as_array()
                    .into_iter()
                    .flatten()
                    .filter(|f| f.get("severity").and_then(Value::as_str) == Some("error"))
                    .map(|f| Shut {
                        message: f.get("message").and_then(Value::as_str).unwrap_or_default().to_owned(),
                        repair: f.get("repair").and_then(Value::as_str).unwrap_or_default().to_owned(),
                    })
                    .collect();
            }
            Err(why) => {
                gate.findings.clear();
                gate.failed = Some(why);
            }
        }
    }

    /// The rule asked and answered now, on this thread (tests and pictures).
    #[cfg(test)]
    pub(crate) fn permit_gate_settle(&mut self) {
        let generation = self.document.as_ref().map(|d| d.model.generation()).unwrap_or_default();
        let Some(facts) = self.permit_gate_facts() else {
            return;
        };
        if let Some(gate) = self.permit_gate_mut() {
            gate.asked_on = Some(generation);
        }
        let answer = crate::standards::call(PACKAGE, MODULE, FUNCTION, vec![facts]);
        self.permit_gate_answer(generation, answer);
    }

    /// The gate's part of the window: what keeps it shut, or that it is open.
    pub(crate) fn permit_gate_view<'a>(&'a self, gate: &'a Gate, generation: u64) -> Element<'a, Message> {
        let mut out = column![label::body("Ruhsat kapısı: paftalar modelden, paket yönetmelikte (Md. 4/2, 6, 8, 9/1)")
            .font(kentos_ui::theme::typography::ui_strong())]
        .spacing(4);
        if gate.asked_on != Some(generation) || !gate.answered {
            return out.push(label::caption("Paftaların modelden üretildiği denetleniyor…")).into();
        }
        if let Some(why) = &gate.failed {
            return out
                .push(label::body(format!("Denetlenemedi, yayın açılmaz: {why}")).style(style::text::danger))
                .into();
        }
        if gate.findings.is_empty() {
            return out
                .push(label::caption(
                    "Açık: kesitler ve görünüşler güncel, model katmanlarında serbest çizim ve kopmuş bağ yok; dosyalar \
                     PDF/A, her pafta ayrı dosyada, kâğıtlar A4 ve katları, adlar EK-2'ye uygun.",
                ))
                .into();
        }
        for f in &gate.findings {
            out = out
                .push(label::body(format!("Hata · {}", f.message)).style(style::text::danger))
                .push(label::caption(&f.repair));
        }
        out.push(
            row![
                label::caption("Kapı kapalıyken ruhsat yayını yapılmaz; bulgular Denetim'de düzeltilir.").width(Fill),
                button(label::caption("Denetim'de göster"))
                    .padding(kentos_ui::theme::metrics::action())
                    .style(style::button::secondary)
                    .on_press(event(Event::GateShow)),
            ]
            .spacing(8)
            .align_y(iced::Alignment::Center),
        )
        .into()
    }
}
