//! Ruhsat takımı's Dijital teslim kapsamı (docs/adr/8063, plan/bim-plan.md BIM-02): the dated scope decision's
//! particulars asked in the window (`kentos_digital_project::scope::inputs`, written into the drawing's variables with
//! the others when the set is made) and the decision as it comes: which obligation — the regulation itself, PDF/A, IFC,
//! PDF/A from the model, e-PYS — starts when for this project, by which article, or what it still needs. The decision
//! is the standards package's Python rule (`dijital_proje.kapsam.karar`, bim-plan K10), the one the audit runs; KentOS
//! hands it the facts — the particulars, the building's yapı inşaat alanı and storeys measured from the drawing
//! (`kentos_processing::audit_facts::permit`) — and the calendar (`kentos_digital_project::scope`). Every edit asks
//! again; an answer to an older edit (its generation) is dropped.

use std::collections::BTreeMap;

use iced::widget::{column, row};
use iced::{Element, Fill, Task};
use kentos_digital_project::scope::{self, Kind};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};
use serde_json::Value;

use super::permit_set::Event;
use super::{Event as Exchange, Window, message};
use crate::app::{App, Message};
use crate::road_dialog::{self, col};

/// The standards package and its function.
const PACKAGE: &str = "dijital-proje";
const MODULE: &str = "dijital_proje.kapsam";
const FUNCTION: &str = "karar";

/// One particular of the decision.
#[derive(Clone, Debug)]
pub struct Field {
    pub name: String,
    pub label: String,
    pub kind: Kind,
    pub choices: Vec<String>,
    pub value: String,
}

/// The section's state: its particulars, the drawing's measures, the decision.
#[derive(Debug)]
pub struct Scope {
    pub fields: Vec<Field>,
    /// The building's yapı inşaat alanı and its storeys above ±0.00, as the drawing measures them: what an empty box
    /// stands for.
    pub area: Option<f64>,
    pub storeys: Option<u32>,
    pub generation: u64,
    pub verdict: Verdict,
    /// The row whose reason is said under the table.
    pub chosen: Option<usize>,
}

/// The decision, as far as it came.
#[derive(Debug)]
pub enum Verdict {
    Asking,
    Given(Given),
    Failed(String),
}

/// The rule's answer.
#[derive(Debug)]
pub struct Given {
    pub rows: Vec<Line>,
    /// The particulars it could not read, each with why and what to write.
    pub problems: Vec<String>,
}

/// One obligation's row.
#[derive(Debug)]
pub struct Line {
    pub title: String,
    pub state: String,
    pub words: String,
    pub from: Option<String>,
    pub until: Option<String>,
    pub articles: Vec<String>,
    pub reason: String,
}

impl Scope {
    /// The particulars as the drawing's variables hold them and the drawing's measures.
    pub fn of(doc: &kentos_domain::Document) -> Scope {
        let have = kentos_processing::audit_facts::permit::particulars(doc);
        let measured = kentos_processing::audit_facts::permit::facts(doc, BTreeMap::new());
        Scope {
            fields: scope::inputs()
                .iter()
                .map(|i| Field {
                    name: i.name.clone(),
                    label: i.label.clone(),
                    kind: i.kind,
                    choices: i.choices.clone(),
                    value: have.get(&i.name).cloned().unwrap_or_default(),
                })
                .collect(),
            area: measured.construction_area,
            storeys: measured.storeys.map(|s| s.above),
            generation: 0,
            verdict: Verdict::Asking,
            chosen: None,
        }
    }

    /// The facts' permit section of what the boxes hold now, with the drawing's measures.
    fn facts(&self, doc: &kentos_domain::Document) -> Value {
        let particulars = self
            .fields
            .iter()
            .filter(|f| !f.value.trim().is_empty())
            .map(|f| (f.name.clone(), f.value.trim().to_owned()))
            .collect();
        serde_json::to_value(kentos_processing::audit_facts::permit::facts(doc, particulars)).unwrap_or(Value::Null)
    }

    /// The boxes' values by name: what the set writes into the drawing's variables.
    pub fn values(&self) -> Vec<(String, String)> {
        self.fields.iter().map(|f| (f.name.clone(), f.value.clone())).collect()
    }

    /// The rule's answer read; a row it does not give whole is left out.
    fn given(answer: &Value) -> Given {
        let text = |v: &Value, k: &str| v.get(k).and_then(Value::as_str).map(str::to_owned);
        let rows = answer
            .get("rows")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|r| {
                Some(Line {
                    title: text(r, "title")?,
                    state: text(r, "state")?,
                    words: text(r, "words")?,
                    from: text(r, "from"),
                    until: text(r, "until"),
                    articles: r
                        .get("articles")
                        .and_then(Value::as_array)
                        .into_iter()
                        .flatten()
                        .filter_map(|a| a.as_str().map(str::to_owned))
                        .collect(),
                    reason: text(r, "reason").unwrap_or_default(),
                })
            })
            .collect();
        let problems = answer
            .get("problems")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|p| text(p, "message"))
            .collect();
        Given { rows, problems }
    }
}

/// `YYYY-AA-GG` as the window writes a day: 1.9.2029.
fn day(iso: &str) -> String {
    let mut parts = iso.split('-');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(y), Some(m), Some(d)) => format!(
            "{}.{}.{y}",
            d.trim_start_matches('0'),
            m.trim_start_matches('0')
        ),
        _ => iso.to_owned(),
    }
}

/// A row's days: “1.3.2029”, “1.3.2029 – 1.9.2032” (until the next obligation takes over), “—”.
fn days(line: &Line) -> String {
    match (&line.from, &line.until) {
        (Some(f), Some(u)) => format!("{} – {}", day(f), day(u)),
        (Some(f), None) => day(f),
        (None, _) => "—".to_owned(),
    }
}

fn event(e: Event) -> Message {
    message(Exchange::PermitSet(e))
}

impl App {
    /// Asks the rule again for what the boxes hold now; its answer comes as [`Event::ScopeGiven`] with this
    /// generation.
    pub(crate) fn permit_scope_ask(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            return Task::none();
        };
        let Some(Window::PermitSet(s)) = &mut self.exchange else {
            return Task::none();
        };
        let scope = &mut s.scope;
        scope.generation += 1;
        scope.verdict = Verdict::Asking;
        let generation = scope.generation;
        let permit = scope.facts(&doc.model);
        let today = kentos_plot::book::today();
        let calendar: Value = serde_json::from_str(scope::CALENDAR_JSON).unwrap_or(Value::Null);
        Task::perform(
            async move {
                let (sent, got) = iced::futures::channel::oneshot::channel();
                std::thread::spawn(move || {
                    let answer = crate::standards::call(
                        PACKAGE,
                        MODULE,
                        FUNCTION,
                        vec![permit, Value::from(today), calendar],
                    );
                    let _ = sent.send(answer);
                });
                got.await
                    .unwrap_or_else(|_| Err("Kuralın süreci cevap vermeden kapandı.".to_owned()))
            },
            move |answer| event(Event::ScopeGiven(generation, answer)),
        )
    }

    /// The section's events; an edit asks again.
    pub(crate) fn permit_scope_event(&mut self, e: Event) -> Task<Message> {
        let Some(Window::PermitSet(s)) = &mut self.exchange else {
            return Task::none();
        };
        let scope = &mut s.scope;
        match e {
            Event::ScopeField(i, v) => {
                if let Some(f) = scope.fields.get_mut(i) {
                    f.value = v;
                }
                self.permit_scope_ask()
            }
            Event::ScopeChoice(i, k) => {
                if let Some(f) = scope.fields.get_mut(i) {
                    f.value = k.and_then(|k| f.choices.get(k).cloned()).unwrap_or_default();
                }
                self.permit_scope_ask()
            }
            Event::ScopeChoose(k) => {
                scope.chosen = Some(k);
                Task::none()
            }
            Event::ScopeGiven(generation, answer) => {
                if generation == scope.generation {
                    scope.verdict = match answer {
                        Ok(answer) => Verdict::Given(Scope::given(&answer)),
                        Err(why) => Verdict::Failed(why),
                    };
                }
                Task::none()
            }
            _ => Task::none(),
        }
    }

    /// The section: its boxes two to a line, then the decision's table and the chosen row's reason.
    pub(crate) fn permit_scope_view<'a>(&'a self, scope: &'a Scope) -> Element<'a, Message> {
        let mut body = column![
            label::body("Dijital teslim kapsamı").font(kentos_ui::theme::typography::ui_strong()),
            label::caption(
                "Bu projede PDF/A, IFC ve e-PYS'nin ne zaman, hangi maddeyle başladığı. Karar yazdıkça yenilenir; \
                 eksik bilgi sonucu değiştirebiliyorsa “Teyit gerekli” yazılır."
            ),
        ]
        .spacing(8);
        let mut pair: Vec<Element<'a, Message>> = Vec::new();
        for (i, f) in scope.fields.iter().enumerate() {
            let input: Element<'a, Message> = match f.kind {
                Kind::Choice => {
                    let at = f.choices.iter().position(|c| *c == f.value);
                    Select::new(f.choices.iter().map(Choice::new), at, move |k| {
                        event(Event::ScopeChoice(i, Some(k)))
                    })
                    .clear(event(Event::ScopeChoice(i, None)))
                    .into()
                }
                _ => {
                    let hint = match f.name.as_str() {
                        "yapi_insaat_alani" => scope.area.map(|a| format!("çizimden {a:.2} m²")),
                        "kat_adedi" => scope.storeys.map(|n| format!("çizimden {n}")),
                        _ => None,
                    };
                    let hint = hint.unwrap_or_else(|| match f.kind {
                        Kind::Date => "GG.AA.YYYY".to_owned(),
                        _ => String::new(),
                    });
                    kentos_ui::widget::text_field(&hint, &f.value)
                        .on_input(move |v| event(Event::ScopeField(i, v)))
                        .into()
                }
            };
            pair.push(column![label::caption(f.label.as_str()), input].spacing(2).width(Fill).into());
            if pair.len() == 2 {
                body = body.push(row(std::mem::take(&mut pair)).spacing(10));
            }
        }
        if !pair.is_empty() {
            pair.push(iced::widget::space().width(Fill).into());
            body = body.push(row(pair).spacing(10));
        }
        match &scope.verdict {
            Verdict::Asking => body = body.push(label::caption("Karar veriliyor…")),
            Verdict::Failed(why) => body = body.push(label::body(why).style(style::text::warning)),
            Verdict::Given(given) => {
                for p in &given.problems {
                    body = body.push(label::caption(p).style(style::text::warning));
                }
                let cols = vec![col("Yükümlülük", 235.0), col("Durum", 120.0), col("Başlangıç", 150.0), col("Madde", 280.0)];
                let rows = given
                    .rows
                    .iter()
                    .map(|l| vec![l.title.clone(), l.words.clone(), days(l), l.articles.join(", ")])
                    .collect();
                let chosen = scope
                    .chosen
                    .or_else(|| given.rows.iter().position(|l| l.state == "confirm"))
                    .or(Some(0));
                body = body.push(road_dialog::sheet_chosen(cols, rows, "Karar yok", 200.0, chosen, |k| {
                    event(Event::ScopeChoose(k))
                }));
                if let Some(l) = chosen.and_then(|k| given.rows.get(k)) {
                    let words = format!("{}: {}", l.title, l.reason);
                    body = body.push(if l.state == "confirm" {
                        label::caption(words).style(style::text::warning)
                    } else {
                        label::caption(words)
                    });
                }
            }
        }
        body.into()
    }
}

#[cfg(test)]
impl App {
    /// The rule asked and answered now, on the test's own thread, for the day `today`: what the window's task does.
    pub(crate) fn permit_scope_settle(&mut self, today: &str) {
        let Some(doc) = &self.document else {
            return;
        };
        let Some(Window::PermitSet(s)) = &mut self.exchange else {
            return;
        };
        let permit = s.scope.facts(&doc.model);
        let calendar: Value = serde_json::from_str(scope::CALENDAR_JSON).unwrap_or(Value::Null);
        let answer = crate::standards::call(PACKAGE, MODULE, FUNCTION, vec![permit, Value::from(today), calendar]);
        s.scope.verdict = match answer {
            Ok(answer) => Verdict::Given(Scope::given(&answer)),
            Err(why) => Verdict::Failed(why),
        };
    }
}
