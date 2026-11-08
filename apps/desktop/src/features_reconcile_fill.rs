//! Adayları uzlaştır's Alan gir (features_reconcile_window.rs, plan/map-cad.md MAP-04, docs/adr/2540): a group whose
//! required field is missing or whose value its class layer's field rules refuse asks for those fields in a form under
//! the table. A field whose domain has a list chooses from it (its code goes in), the others are typed; the rule's
//! sentence stands beside a refused one. Alanları ekle puts the values in the decisions of the group's candidates (over
//! a decision already there) and the plan reads them again; the command checks them, the window does not.

use iced::widget::{container, row};
use iced::{Center, Element};
use kentos_commands::{CandidateDecision, ReconciledGroup};
use kentos_ui::label;
use kentos_ui::widget::Form;

use crate::app::{App, Message};
use crate::exchange::words;
use crate::features_reconcile_window::{Event, PROFILE, msg};
use crate::style::fields;

pub const FILL_APPLY: &str = "Alanları ekle";
pub const FILL_CANCEL: &str = "Vazgeç";

/// A class field of the profile by name, with its list when its domain has one: its label and the (code, label) values.
fn class_field(class: &str, name: &str) -> Option<(String, Vec<(String, String)>)> {
    let profile = kentos_commands::builtin_profile(PROFILE)?;
    let field = profile.classes.iter().find(|c| c.id == class)?.fields.iter().find(|f| f.name == name)?.clone();
    let values = field
        .domain
        .as_ref()
        .and_then(|d| profile.domains.iter().find(|x| x.id == *d))
        .map(|d| d.values.iter().flatten().map(|v| (v.code.clone(), v.label.clone())).collect())
        .unwrap_or_default();
    Some((field.alias.clone().filter(|a| !a.is_empty()).unwrap_or(field.name.clone()), values))
}

/// A field as the window names it: its alias in the profile, else its name.
pub(crate) fn field_title(class: &str, name: &str) -> String {
    class_field(class, name).map_or_else(|| name.to_owned(), |(title, _)| title)
}

/// A class as the window names it: its title in the profile, else its id.
pub(crate) fn class_title(class: &str) -> String {
    kentos_commands::builtin_profile(PROFILE)
        .and_then(|p| p.classes.into_iter().find(|c| c.id == class).map(|c| c.title))
        .unwrap_or_else(|| class.to_owned())
}

/// A group's fields Alan gir asks for: the missing ones, then those the rules refuse, each once.
pub(crate) fn fill_fields(g: &ReconciledGroup) -> Vec<String> {
    let mut out: Vec<String> = g.missing.clone();
    for t in &g.invalid {
        if !out.contains(&t.field) {
            out.push(t.field.clone());
        }
    }
    out
}

impl App {
    /// Alanları ekle: the typed values in the decisions of the group's candidates, the plan again.
    pub(crate) fn reconcile_fill_apply(&mut self) {
        let Some(g) = self.features_reconcile.fill else { return };
        let typed: std::collections::BTreeMap<String, String> = self
            .features_reconcile
            .typed
            .iter()
            .filter(|(_, v)| !v.trim().is_empty())
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect();
        if typed.is_empty() {
            self.warn("Alan gir: bir değer yazın ya da seçin.");
            return;
        }
        let Some(Ok(answer)) = &self.features_reconcile.answer else { return };
        let Some(group) = answer.groups.get(g) else { return };
        let uids = group.candidates.clone();
        let s = &mut self.features_reconcile;
        for uid in uids {
            match s.decisions.iter_mut().find(|d| d.candidate == uid) {
                Some(d) => d.fields.extend(typed.clone()),
                None => s.decisions.push(CandidateDecision {
                    candidate: uid,
                    fields: typed.clone(),
                    ..CandidateDecision::default()
                }),
            }
        }
        s.fill = None;
        s.typed.clear();
        self.reconcile_run();
    }

    /// The form of the group Alan gir opened, with its heading: none while no group is open.
    pub(crate) fn reconcile_fill_view(&self) -> Option<(Element<'_, Message>, Element<'_, Message>)> {
        let s = &self.features_reconcile;
        let (Some(g), Some(Ok(a))) = (s.fill, &s.answer) else { return None };
        let group = a.groups.get(g)?;
        let class = group.class.clone().unwrap_or_default();
        let mut fill = Form::new().label_width(130.0);
        for name in fill_fields(group) {
            let (title, values) = class_field(&class, &name).unwrap_or((name.clone(), Vec::new()));
            let typed = s.typed.get(&name).cloned().unwrap_or_default();
            let why = group.invalid.iter().find(|t| t.field == name).map(|t| t.message.clone());
            let field = name.clone();
            let input: Element<'_, Message> = if values.is_empty() {
                container(fields::input(None, &title, &typed, false, false).on_input(move |v| msg(Event::Typed(field.clone(), v))))
                    .width(240)
                    .into()
            } else {
                let mut choices = vec![(String::new(), "Seçin".to_owned())];
                choices.extend(values);
                container(fields::select_owned(choices, &typed, move |v| msg(Event::Typed(field.clone(), v)))).width(240).into()
            };
            let line = match why {
                Some(w) => row![input, label::caption(w)].spacing(8).align_y(Center),
                None => row![input].spacing(8).align_y(Center),
            };
            fill = fill.field(title, line);
        }
        fill = fill.field(
            "",
            row![
                words::secondary(FILL_APPLY, Some(msg(Event::FillApply))),
                words::ghost(FILL_CANCEL, Some(msg(Event::FillCancel)))
            ]
            .spacing(8),
        );
        let head = label::caption(format!("Alan gir: {} grubunun alanları", class_title(&class))).into();
        Some((head, fill.into()))
    }
}
