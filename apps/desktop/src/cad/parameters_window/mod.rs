//! Parametreler (docs/adr/0338 §1; AutoCAD's PARAMETERS manager): the drawing's parameters, each its name, expression and
//! value, an expression changed and the drawing solved with it (Enter or Uygula), a user parameter added or taken
//! away; and the constraints, each its kind, its parameter and whether the drawing meets it, taken away or made a
//! reference (a dimension). A value the constraints cannot meet is refused with nothing written: the constraints in
//! the conflict are marked in the list, each with Kaldır and Başvuru yap, the way out of it. Every write goes through
//! `cad.parameters.edit` or `cad.constraints.edit`: one undo step each.

mod analysis;

use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::Rc;

use analysis::Analysis;
use iced::widget::{Column, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CadParameter, CommandResult, ConstraintEdit, ConstraintsEdit, ParameterEdit, ParameterKind,
    ParametersEdit, rule_label,
};
use kentos_interaction::constraint_marks::parameter_value_words;
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

type KeptAnalysis = Option<((u64, u64, u64), Rc<Analysis>)>;

#[derive(Debug, Clone, Default)]
pub struct Window {
    /// Expressions as typed, by parameter name; a name not here shows the drawing's.
    typed: BTreeMap<String, String>,
    session: u64,
    analysis: RefCell<KeptAnalysis>,
    inspected: Option<String>,
    rename_to: String,
    new_kind: Option<ParameterKind>,
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
    Inspect(String),
    RenameTo(String),
    Rename,
    ShowParameter(String),
    ShowConstraint(String),
    ShowFree,
    NewKind(&'static str),
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

impl App {
    /// Parametreler (`constraint.parameters`).
    pub(crate) fn open_parameters(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let session = self.document.as_ref().map_or(0, |d| d.session);
        if self
            .parameters_window
            .as_ref()
            .is_none_or(|w| w.session != session)
        {
            self.parameters_window = Some(Window {
                session,
                ..Window::default()
            });
        }
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
            Event::Inspect(name) => {
                w.rename_to = name.clone();
                w.inspected = Some(name);
            }
            Event::RenameTo(t) => w.rename_to = t,
            Event::Rename => {
                let Some(name) = w.inspected.clone() else {
                    return Task::none();
                };
                let to = w.rename_to.trim().to_owned();
                self.edit_parameters(
                    vec![ParameterEdit::Rename {
                        name,
                        to: to.clone(),
                    }],
                    format!("Parametrenin adı “{to}” oldu; bağlı ifadeler güncellendi."),
                );
                if let Some(w) = self.parameters_window.as_mut()
                    && w.said.as_ref().is_some_and(|(k, _)| matches!(k, Kind::Ok))
                {
                    w.inspected = Some(to);
                }
            }
            Event::NewKind(kind) => {
                w.new_kind = Some(match kind {
                    "Açı" => ParameterKind::Angle,
                    "Sayı" => ParameterKind::Number,
                    _ => ParameterKind::Length,
                })
            }
            Event::ShowParameter(name) => {
                let Some(doc) = self.document.as_ref() else {
                    return Task::none();
                };
                let names = self
                    .parameters_analysis()
                    .map(|r| r.dependencies.affected(&name))
                    .unwrap_or_default();
                let ids = doc
                    .model
                    .constraints()
                    .constraints
                    .iter()
                    .filter(|c| c.rule.parameter().is_some_and(|p| names.contains(p)))
                    .flat_map(|c| c.rule.elements())
                    .map(|e| e.uid)
                    .collect::<Vec<_>>();
                self.parameters_show(ids);
            }
            Event::ShowConstraint(id) => {
                let ids = self
                    .document
                    .as_ref()
                    .and_then(|doc| {
                        doc.model
                            .constraints()
                            .constraints
                            .iter()
                            .find(|c| c.id.to_string() == id)
                    })
                    .map(|c| c.rule.elements().into_iter().map(|e| e.uid).collect())
                    .unwrap_or_default();
                self.parameters_show(ids);
            }
            Event::ShowFree => {
                let ids = self
                    .parameters_analysis()
                    .and_then(|r| r.listed.as_ref().map(|r| r.diagnosis.free.clone()))
                    .unwrap_or_default();
                self.parameters_show(ids);
            }
            Event::NewName(t) => w.new_name = t,
            Event::NewExpression(t) => w.new_expression = t,
            Event::Add => {
                let (name, expression) = (
                    w.new_name.trim().to_owned(),
                    w.new_expression.trim().to_owned(),
                );
                let done = format!("“{name}” eklendi.");
                let kind = w.new_kind.unwrap_or(ParameterKind::Length);
                self.edit_parameters(
                    vec![ParameterEdit::Add {
                        parameter: CadParameter {
                            name,
                            kind,
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

    fn parameters_analysis(&self) -> Option<Rc<Analysis>> {
        let (w, doc) = (self.parameters_window.as_ref()?, self.document.as_ref()?);
        let key = (doc.session, doc.model.revision(), doc.model.generation());
        if let Some((was, readout)) = w.analysis.borrow().as_ref()
            && *was == key
        {
            return Some(readout.clone());
        }
        let readout = Rc::new(Analysis::read(&doc.model));
        *w.analysis.borrow_mut() = Some((key, readout.clone()));
        Some(readout)
    }

    fn parameters_show(&mut self, ids: Vec<kentos_commands::EntityId>) {
        let Some(doc) = self.document.as_ref() else {
            return;
        };
        let slots = ids
            .into_iter()
            .filter_map(|u| doc.model.slot_of(kentos_domain::Uuid::from_bytes(u.0)))
            .collect::<Vec<_>>();
        if slots.is_empty() {
            return;
        }
        self.selection.set(slots);
        self.zoom_selection();
        // Keep expression drafts for reopening this same drawing's window.
        self.dialog = None;
        self.output(
            "İlgili geometri seçildi. Parametreler penceresini yeniden açarak devam edebilirsiniz.",
        );
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
        let Some(readout) = self.parameters_analysis() else {
            return iced::widget::text("").into();
        };
        let values = &readout.values;
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
            let name = iced::widget::button(
                Elided::new(p.name.clone())
                    .size(typography::body())
                    .font(typography::ui()),
            )
            .on_press(msg(Event::Inspect(p.name.clone())))
            .style(style::button::row(w.inspected.as_ref() == Some(&p.name)))
            .padding([5, 6])
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
                .map(|v| {
                    let value = parameter_value_words(*v, p.kind, model.settings());
                    if only_measures {
                        format!("({value})")
                    } else {
                        value
                    }
                })
                .unwrap_or_else(|| "—".into());
            let changed = w.typed.contains_key(&p.name);
            let face = row![
                name,
                field,
                container(label::caption(value)).width(Fill),
                if changed {
                    words::secondary(APPLY, Some(msg(Event::Apply(p.name.clone()))))
                } else {
                    container(iced::widget::space())
                        .width(typography::scaled(64.0))
                        .into()
                },
            ]
            .spacing(8)
            .align_y(Center);
            params = params.push(container(face).padding([3, 8]));
        }
        let table = container(iced::widget::scrollable(params).height(params_height))
            .style(style::container::field_box)
            .width(Fill);
        let kinds = ["Uzunluk", "Açı", "Sayı"];
        let chosen_kind = match w.new_kind.unwrap_or(ParameterKind::Length) {
            ParameterKind::Length => "Uzunluk",
            ParameterKind::Angle => "Açı",
            ParameterKind::Number => "Sayı",
        };
        let add = row![
            iced::widget::pick_list(kinds, Some(chosen_kind), |k| msg(Event::NewKind(k)))
                .width(typography::scaled(100.0)),
            container(
                kentos_ui::widget::text_field("ad", &w.new_name)
                    .on_input(|t| msg(Event::NewName(t)))
            )
            .width(typography::scaled(110.0)),
            container(
                kentos_ui::widget::text_field(
                    match w.new_kind.unwrap_or(ParameterKind::Length) {
                        ParameterKind::Length => "ifade (3.5, 120cm, genislik / 2)",
                        ParameterKind::Angle => "ifade (30deg, 90deg, donus / 2)",
                        ParameterKind::Number => "ifade (3, adet + 1)",
                    },
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
        let diagnosis = readout.listed.as_ref().map(|r| &r.diagnosis);
        let mut list = Column::new().spacing(1).padding(2);
        if set.constraints.is_empty() {
            list = list
                .push(container(label::caption("Kısıt yok. Kısıt aracıyla ekleyin.")).padding(10));
        }
        for (i, c) in set.constraints.iter().enumerate() {
            let id = c.id.to_string();
            let in_conflict = w.conflict.contains(&id);
            let state = if in_conflict || diagnosis.is_some_and(|d| d.conflicting.contains(&c.id)) {
                (Kind::Warn, "çelişiyor")
            } else if diagnosis.is_some_and(|d| d.broken.contains(&c.id)) {
                (Kind::Warn, "bağ kopuk")
            } else if diagnosis.is_some_and(|d| d.singular.contains(&c.id)) {
                (Kind::Warn, "geometri geçersiz")
            } else if diagnosis.is_some_and(|d| d.redundant.contains(&c.id)) {
                (Kind::Warn, "tekrarlıyor")
            } else if let Some(r) = readout
                .listed
                .as_ref()
                .and_then(|r| r.constraints.iter().find(|r| r.constraint.id == c.id))
            {
                if r.holds {
                    (Kind::Ok, "tutuyor")
                } else {
                    (Kind::Warn, "tutmuyor")
                }
            } else {
                (Kind::Warn, "teşhis yok")
            };
            let mut what = rule_label(&c.rule).to_owned();
            if let Some(name) = c.rule.parameter() {
                what.push_str(&format!(" · {name}"));
            }
            if c.reference {
                what.push_str(" · başvuru");
            }
            let mut face = row![
                iced::widget::button(label::body(format!("{}. {what}", i + 1)))
                    .on_press(msg(Event::ShowConstraint(id.clone())))
                    .style(style::button::flat)
                    .width(Fill),
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
        let mut body = Column::new().spacing(12).push(
            row![
                container(label::body(readout.summary())).width(Fill),
                words::secondary(
                    "Serbest geometriyi göster",
                    diagnosis
                        .filter(|d| !d.free.is_empty())
                        .map(|_| msg(Event::ShowFree))
                )
            ]
            .spacing(8)
            .align_y(Center),
        );
        if readout.nurbs_endpoints {
            body = body.push(label::caption(
                "NURBS için yalnız uç kısıtları hesaplanır. İç kontrol noktaları bu serbestlik sayısına dahil değildir; düzenlenebilir kalır.",
            ));
        }
        if readout.block_anchors {
            body = body.push(label::caption(
                "Blokların yalnız yerleştirme noktaları hesaplanır. Dönme açısı, ayna, ölçek ve iç parçalar korunur; bu serbestlik sayısına dahil değildir.",
            ));
        }
        if !readout.unit_issues.is_empty() {
            body = body.push(words::text_line(Kind::Warn, format!(
                "{} eski ifadede birim uyuşmazlığı var. Değerler korundu; ayrıntı için parametre adını seçin.", readout.unit_issues.len()
            )));
        }
        if let Some((kind, said)) = &w.said {
            body = body.push(words::summary(vec![words::text_line(*kind, said.clone())]));
        }
        body = body .push(words::field(
                "Parametreler",
                table,
                Some("Adı seçerek bağımlılıkları inceleyin. Birimsiz yazılan ölçü proje birimindedir; farklı türler toplanamaz. pi yarım turdur; birimsiz π için pi / 1rad. Enter uygular.".to_owned()),
            ))
            ;
        if let Some(p) = w.inspected.as_ref().and_then(|name| set.parameter(name)) {
            let defining = readout
                .dependencies
                .defining(&p.name)
                .into_iter()
                .collect::<Vec<_>>();
            let affected = readout.dependencies.affected(&p.name);
            let dependents = affected
                .iter()
                .filter(|n| **n != p.name)
                .cloned()
                .collect::<Vec<_>>();
            let count = set
                .constraints
                .iter()
                .filter(|c| c.rule.parameter().is_some_and(|n| affected.contains(n)))
                .count();
            let what = if measured.contains(&p.name.as_str()) {
                "Başvuru · yalnız ölçer"
            } else if set
                .constraints
                .iter()
                .any(|c| c.rule.parameter() == Some(p.name.as_str()))
            {
                "Sürücü · geometriyi korur"
            } else {
                "Kullanıcı parametresi"
            };
            let mut detail = iced::widget::column![
                label::body(format!("{} · {what}", p.name)),
                label::caption(format!(
                    "Tanımlayan: {} · Etkilediği: {} · {count} ölçü bağı",
                    if defining.is_empty() {
                        "sabit".into()
                    } else {
                        defining.join(", ")
                    },
                    if dependents.is_empty() {
                        "başka parametre yok".into()
                    } else {
                        dependents.join(", ")
                    }
                )),
                row![
                    container(
                        kentos_ui::widget::text_field("yeni ad", &w.rename_to)
                            .on_input(|t| msg(Event::RenameTo(t)))
                            .on_submit(msg(Event::Rename))
                    )
                    .width(Fill),
                    words::secondary(
                        "Yeniden adlandır",
                        (!w.rename_to.trim().is_empty() && w.rename_to.trim() != p.name)
                            .then(|| msg(Event::Rename))
                    ),
                    words::secondary(REMOVE, Some(msg(Event::Remove(p.name.clone())))),
                    words::secondary(
                        "Geometriyi göster",
                        (count > 0).then(|| msg(Event::ShowParameter(p.name.clone())))
                    )
                ]
                .spacing(8)
                .align_y(Center)
            ]
            .spacing(8);
            if let Some(issue) = readout.unit_issues.get(&p.name) {
                detail = detail.push(words::text_line(
                    Kind::Warn,
                    format!(
                        "Eski değer korundu. {issue} Bu ifadeyi düzenlerken birimleri düzeltin."
                    ),
                ));
            }
            body = body.push(
                container(detail)
                    .padding(10)
                    .style(style::container::field_box),
            );
        }
        body = body
            .push(words::field("Yeni kullanıcı parametresi", add, None))
            .push(words::field(
                "Kısıtlar",
                constraints,
                Some(format!(
                    "{} kısıt, {} parametre. Kısıt adına tıklayarak geometriyi gösterin.",
                    set.constraints.len(),
                    set.parameters.len()
                )),
            ));
        Frame::new(PARAMETERS_TITLE)
            .scroll(body)
            .max_height(700.0)
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

#[cfg(test)]
mod tests {
    use super::*;
    use kentos_commands::{ConstraintsQuery, ParameterKind};
    use kentos_native_application::constraints_query;

    fn app() -> App {
        let (mut app, _) = App::boot(None);
        crate::constraint_scenes::drawn(&mut app);
        crate::constraint_scenes::rectangle(&mut app);
        app.open_parameters();
        app
    }
    fn event(app: &mut App, e: Event) {
        let _ = app.update(Message::Parameters(e));
    }
    #[test]
    fn query_evidence_is_shared_across_typing_and_refreshed_after_write_and_undo() {
        let mut app = app();
        let first = app.parameters_analysis().unwrap();
        let doc = &app.document.as_ref().unwrap().model;
        let before = doc.to_snapshot_v2();
        let CommandResult::Completed { output, .. } =
            constraints_query::read(doc, &ConstraintsQuery::default())
        else {
            panic!("query")
        };
        assert_eq!(first.listed.as_ref().unwrap().diagnosis, output.diagnosis);
        event(&mut app, Event::Expression("yukseklik".into(), "9".into()));
        assert!(Rc::ptr_eq(&first, &app.parameters_analysis().unwrap()));
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        event(&mut app, Event::Apply("yukseklik".into()));
        let after = app.parameters_analysis().unwrap();
        assert!(!Rc::ptr_eq(&first, &after));
        assert_eq!(after.values["yukseklik"], 9.);
        app.document.as_mut().unwrap().model.undo();
        let undo = app.parameters_analysis().unwrap();
        assert!(!Rc::ptr_eq(&after, &undo));
        assert_eq!(undo.values["yukseklik"], 8.);
    }
    #[test]
    fn parameter_inspection_rename_and_navigation_keep_drafts_without_writing_geometry() {
        let mut app = app();
        event(&mut app, Event::NewName("yarisi".into()));
        event(&mut app, Event::NewExpression("yukseklik / 2".into()));
        event(&mut app, Event::Add);
        event(&mut app, Event::Inspect("yukseklik".into()));
        event(&mut app, Event::RenameTo("kat_yuksekligi".into()));
        event(&mut app, Event::Rename);
        let model = &app.document.as_ref().unwrap().model;
        assert_eq!(
            model.constraints().parameter("yarisi").unwrap().expression,
            "kat_yuksekligi / 2"
        );
        assert!(
            model
                .constraints()
                .constraints
                .iter()
                .any(|c| c.rule.parameter() == Some("kat_yuksekligi"))
        );
        let before = model.to_snapshot_v2();
        event(
            &mut app,
            Event::Expression("yarisi".into(), "kat_yuksekligi / 3".into()),
        );
        event(&mut app, Event::ShowParameter("kat_yuksekligi".into()));
        assert!(app.dialog.is_none());
        assert!(!app.selection.is_empty());
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        app.open_parameters();
        assert_eq!(
            app.parameters_window.as_ref().unwrap().typed["yarisi"],
            "kat_yuksekligi / 3"
        );
    }
    #[test]
    fn angle_and_number_user_parameters_use_the_existing_command_kind() {
        let mut app = app();
        for (label, name, expr, kind) in [
            ("Açı", "donus", "30deg", ParameterKind::Angle),
            ("Sayı", "adet", "3", ParameterKind::Number),
        ] {
            event(&mut app, Event::NewKind(label));
            event(&mut app, Event::NewName(name.into()));
            event(&mut app, Event::NewExpression(expr.into()));
            event(&mut app, Event::Add);
            assert_eq!(
                app.document
                    .as_ref()
                    .unwrap()
                    .model
                    .constraints()
                    .parameter(name)
                    .unwrap()
                    .kind,
                kind
            );
        }
    }
    #[test]
    fn a_bad_expression_keeps_the_last_valid_document_and_the_correction_draft() {
        let mut app = app();
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        event(
            &mut app,
            Event::Expression("yukseklik".into(), "1 / 0".into()),
        );
        event(&mut app, Event::Apply("yukseklik".into()));
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        assert_eq!(
            app.parameters_window.as_ref().unwrap().typed["yukseklik"],
            "1 / 0"
        );
        assert!(matches!(
            app.parameters_window
                .as_ref()
                .unwrap()
                .said
                .as_ref()
                .unwrap()
                .0,
            Kind::Warn
        ));
    }
}
