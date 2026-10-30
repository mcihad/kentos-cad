//! Parametreler (docs/adr/0338 §1; AutoCAD's PARAMETERS manager): the drawing's parameters, each its name, expression and
//! value, an expression changed and the drawing solved with it (Enter or Uygula), a user parameter added or taken
//! away; and the constraints, each its kind, its parameter and whether the drawing meets it, taken away or made a
//! reference (a dimension). A value the constraints cannot meet is refused with nothing written: the constraints in
//! the conflict are marked in the list, each with Kaldır and Başvuru yap, the way out of it. Every write goes through
//! `cad.parameters.edit` or `cad.constraints.edit`: one undo step each.

use std::collections::BTreeMap;

use iced::widget::{Column, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CadParameter, CommandResult, ConstraintEdit, ConstraintsEdit, ParameterEdit, ParameterKind,
    ParametersEdit, rule_label,
};
use kentos_native_application::constraint_solve::{parameter_values, unmet};
use kentos_native_application::{ExecutionContext, constraints_edit, parameters_edit};
use kentos_ui::theme::typography;
use kentos_ui::widget::{Dialog as Frame, Elided};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const PARAMETERS_TITLE: &str = "Parametreler";
const ADD: &str = "Ekle";
const CLOSE: &str = "Kapat";
const APPLY: &str = "Uygula";
const REMOVE: &str = "Sil";
const TAKE_AWAY: &str = "Kaldır";

#[derive(Debug, Clone, Default)]
pub struct Window {
    /// Expressions as typed, by parameter name; a name not here shows the drawing's.
    typed: BTreeMap<String, String>,
    new_name: String,
    new_expression: String,
    /// The constraints in the last refusal's conflict, by id.
    conflict: Vec<String>,
    said: Option<(Kind, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Expression(String, String),
    Apply(String),
    Remove(String),
    NewName(String),
    NewExpression(String),
    Add,
    TakeAway(String),
    Mode(String, bool),
    Close,
}

fn msg(event: Event) -> Message {
    Message::Parameters(event)
}

fn kind_words(kind: ParameterKind) -> &'static str {
    match kind {
        ParameterKind::Length => "uzunluk",
        ParameterKind::Angle => "açı",
        ParameterKind::Number => "sayı",
    }
}

fn value_words(v: f64) -> String {
    let t = format!("{v:.4}");
    let t = t.trim_end_matches('0').trim_end_matches('.');
    if t == "-0" { "0".into() } else { t.into() }
}

impl App {
    /// Parametreler (`constraint.parameters`).
    pub(crate) fn open_parameters(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.parameters_window = Some(Window::default());
        self.dialog = Some(Dialog::Parameters);
    }

    /// What a write said: done, or refused with the conflict's constraints kept to mark.
    fn parameters_said<T>(&mut self, result: CommandResult<T>, done: String) {
        let Some(w) = self.parameters_window.as_mut() else {
            return;
        };
        match result {
            CommandResult::Completed { warnings, .. } => {
                w.conflict.clear();
                w.typed.clear();
                let mut words = done;
                for warning in warnings {
                    words.push_str(&format!(" {}", warning.message));
                }
                w.said = Some((Kind::Ok, words));
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                w.conflict = if error.code == "constraint_conflict" {
                    error.uids.clone()
                } else {
                    Vec::new()
                };
                let words = if w.conflict.is_empty() {
                    error.message
                } else {
                    format!(
                        "{} İşaretli {} kısıttan birini kaldırın ya da başvuru yapın.",
                        error.message,
                        w.conflict.len()
                    )
                };
                w.said = Some((Kind::Warn, words));
            }
            _ => {}
        }
    }

    fn edit_parameters(&mut self, edits: Vec<ParameterEdit>, done: String) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let input = ParametersEdit {
            edits,
            allow_flip: false,
            expected_revision: None,
        };
        let result = parameters_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        self.parameters_said(result, done);
    }

    fn edit_constraints(&mut self, edit: ConstraintEdit, done: String) {
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let input = ConstraintsEdit {
            edit,
            hold: Vec::new(),
            allow_flip: false,
            expected_revision: None,
        };
        let result = constraints_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        self.parameters_said(result, done);
    }

    pub(crate) fn parameters_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.parameters_window.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Expression(name, t) => {
                w.typed.insert(name, t);
            }
            Event::Apply(name) => {
                let Some(text) = w.typed.get(&name).cloned() else {
                    return Task::none();
                };
                let done = format!("“{name}” = {text}: çizim yeni değerle çözüldü.");
                self.edit_parameters(
                    vec![ParameterEdit::Set {
                        name,
                        expression: Some(text),
                        min: None,
                        max: None,
                        description: None,
                    }],
                    done,
                );
            }
            Event::Remove(name) => {
                let done = format!("“{name}” kaldırıldı.");
                self.edit_parameters(vec![ParameterEdit::Remove { name }], done);
            }
            Event::NewName(t) => w.new_name = t,
            Event::NewExpression(t) => w.new_expression = t,
            Event::Add => {
                let (name, expression) = (
                    w.new_name.trim().to_owned(),
                    w.new_expression.trim().to_owned(),
                );
                let done = format!("“{name}” eklendi.");
                self.edit_parameters(
                    vec![ParameterEdit::Add {
                        parameter: CadParameter {
                            name,
                            kind: ParameterKind::Length,
                            expression,
                            min: None,
                            max: None,
                            description: String::new(),
                        },
                    }],
                    done,
                );
                if let Some(w) = self.parameters_window.as_mut()
                    && w.said.as_ref().is_some_and(|(k, _)| matches!(k, Kind::Ok))
                {
                    w.new_name.clear();
                    w.new_expression.clear();
                }
            }
            Event::TakeAway(id) => {
                self.edit_constraints(
                    ConstraintEdit::Remove { ids: vec![id] },
                    "Kısıt kaldırıldı.".into(),
                );
            }
            Event::Mode(id, reference) => {
                let done = if reference {
                    "Ölçü başvuru yapıldı: artık yalnız ölçer."
                } else {
                    "Ölçü yeniden sürücü: değeri çizimi belirler."
                };
                self.edit_constraints(
                    ConstraintEdit::Mode {
                        ids: vec![id],
                        reference,
                    },
                    done.into(),
                );
            }
            Event::Close => {
                self.parameters_window = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn parameters_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.parameters_window, &self.document) else {
            return iced::widget::text("").into();
        };
        let model = &doc.model;
        let set = model.constraints();
        // The two lists share what the window leaves: at 1100 × 650 both still show and Kapat stays on screen.
        let room = self.window_size.height - typography::scaled(420.0);
        let (params_height, list_height) = (
            (room * 0.45).clamp(typography::scaled(64.0), typography::scaled(180.0)),
            (room * 0.55).clamp(typography::scaled(90.0), typography::scaled(200.0)),
        );
        let values = parameter_values(model, set).unwrap_or_default();
        let measured: Vec<&str> = set
            .constraints
            .iter()
            .filter(|c| c.reference)
            .filter_map(|c| c.rule.parameter())
            .collect();
        // The parameters.
        let mut params = Column::new().spacing(1).padding(2);
        if set.parameters.is_empty() {
            params = params.push(
                container(label::caption(
                    "Parametre yok. Bir ölçü kısıtı (Kısıt › Uzaklık, Açı, Yarıçap, Çap) kendi parametresini getirir; aşağıdan bir kullanıcı parametresi de eklenebilir.",
                ))
                .padding(10),
            );
        }
        for p in &set.parameters {
            let name = Elided::new(p.name.clone())
                .size(typography::body())
                .font(typography::ui())
                .width(typography::scaled(110.0));
            let only_measures = measured.contains(&p.name.as_str());
            let typed = w
                .typed
                .get(&p.name)
                .cloned()
                .unwrap_or_else(|| p.expression.clone());
            let field: Element<'_, Message> = if only_measures {
                container(label::caption("yalnız ölçer"))
                    .width(typography::scaled(200.0))
                    .into()
            } else {
                let key = p.name.clone();
                let submit = p.name.clone();
                container(
                    kentos_ui::widget::text_field("ifade", &typed)
                        .on_input(move |t| msg(Event::Expression(key.clone(), t)))
                        .on_submit(msg(Event::Apply(submit))),
                )
                .width(typography::scaled(200.0))
                .into()
            };
            let value = values
                .get(&p.name)
                .map(|v| format!("{} ({})", value_words(*v), kind_words(p.kind)))
                .unwrap_or_else(|| "—".into());
            let changed = w.typed.contains_key(&p.name);
            let face = row![
                name,
                field,
                container(label::caption(value)).width(Fill),
                words::secondary(APPLY, changed.then(|| msg(Event::Apply(p.name.clone())))),
                words::secondary(REMOVE, Some(msg(Event::Remove(p.name.clone())))),
            ]
            .spacing(8)
            .align_y(Center);
            params = params.push(container(face).padding([3, 8]));
        }
        let table = container(iced::widget::scrollable(params).height(params_height))
            .style(style::container::field_box)
            .width(Fill);
        let add = row![
            container(
                kentos_ui::widget::text_field("ad", &w.new_name)
                    .on_input(|t| msg(Event::NewName(t)))
            )
            .width(typography::scaled(110.0)),
            container(
                kentos_ui::widget::text_field(
                    "ifade (3.5, 120cm, genislik / 2)",
                    &w.new_expression
                )
                .on_input(|t| msg(Event::NewExpression(t)))
                .on_submit(msg(Event::Add))
            )
            .width(Fill),
            words::secondary(
                ADD,
                (!w.new_name.trim().is_empty() && !w.new_expression.trim().is_empty())
                    .then(|| msg(Event::Add))
            ),
        ]
        .spacing(8)
        .align_y(Center);
        // The constraints.
        let missed = unmet(model, set, None, None);
        let mut list = Column::new().spacing(1).padding(2);
        if set.constraints.is_empty() {
            list = list
                .push(container(label::caption("Kısıt yok. Kısıt aracıyla ekleyin.")).padding(10));
        }
        for (i, c) in set.constraints.iter().enumerate() {
            let id = c.id.to_string();
            let in_conflict = w.conflict.contains(&id);
            let state = if in_conflict {
                (Kind::Warn, "çelişiyor")
            } else if missed.contains(&i) {
                (Kind::Warn, "tutmuyor")
            } else {
                (Kind::Ok, "tutuyor")
            };
            let mut what = rule_label(&c.rule).to_owned();
            if let Some(name) = c.rule.parameter() {
                what.push_str(&format!(" · {name}"));
            }
            if c.reference {
                what.push_str(" · başvuru");
            }
            let mut face = row![
                container(label::body(format!("{}. {what}", i + 1))).width(Fill),
                words::text_line(state.0, state.1.to_owned()),
            ]
            .spacing(8)
            .align_y(Center);
            if c.rule.parameter().is_some() {
                face = face.push(words::secondary(
                    if c.reference {
                        "Sürücü yap"
                    } else {
                        "Başvuru yap"
                    },
                    Some(msg(Event::Mode(id.clone(), !c.reference))),
                ));
            }
            face = face.push(words::secondary(TAKE_AWAY, Some(msg(Event::TakeAway(id)))));
            list = list.push(
                container(face)
                    .padding([3, 8])
                    .style(style::container::menu_item(in_conflict, true, in_conflict)),
            );
        }
        let constraints = container(iced::widget::scrollable(list).height(list_height))
            .style(style::container::field_box)
            .width(Fill);
        let mut body = Column::new()
            .spacing(12)
            .push(words::field(
                "Parametreler",
                table,
                Some("İfade uzunlukta çizim biriminde, açıda projenin açı biriminde; Enter çizimi yeni değerle çözer.".to_owned()),
            ))
            .push(words::field("Yeni kullanıcı parametresi", add, None))
            .push(words::field(
                "Kısıtlar",
                constraints,
                Some(format!("{} kısıt, {} parametre.", set.constraints.len(), set.parameters.len())),
            ));
        if let Some((kind, said)) = &w.said {
            body = body.push(words::summary(vec![words::text_line(*kind, said.clone())]));
        }
        Frame::new(PARAMETERS_TITLE)
            .push(body)
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(720.0)
            .modal(msg(Event::Close))
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn parameters_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.parameters_window.is_none() {
            return Err(format!("{PARAMETERS_TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            _ => None,
        })
    }
}
