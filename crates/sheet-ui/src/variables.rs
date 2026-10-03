//! Değişkenler (design §7; the web's `VariablesDialog`): the values written
//! `[% @ad %]` in the sheet's texts and title block cells — the sheet's own
//! (ada, parsel, mahalle … a template's questions) and the project's (proje
//! no, idare …), the sheet's found first. One with no value is written ‹ad?›
//! on the paper and the preflight says so; the names the preflight found
//! missing are offered to add. Kaydet is one undo step for both lists.

use iced::widget::{button, column, container, row, scrollable};
use iced::{Center, Element, Fill};
use kentos_sheet::model::{VarKind, VarValue, Variable};
use kentos_sheet::ops::{Op, SaveVariables};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::{Dialog, InputField, Switch};

use crate::designer::{Designer, Dialog as Window, Effect};
use crate::message::Message;

/// Whose variables: the sheet's or the project's.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Scope {
    Sheet,
    Project,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct VariablesDialog {
    pub own: Vec<Variable>,
    pub project: Vec<Variable>,
    /// What is typed in each value field, kept as typed (a number in Turkish).
    pub texts: Vec<(Scope, usize, String)>,
    pub new_own: String,
    pub new_project: String,
    pub problem: Option<String>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VariablesMessage {
    Open,
    Text(Scope, usize, String),
    Bool(Scope, usize, bool),
    NewName(Scope, String),
    Add(Scope),
    /// The names the preflight found without a value, as the sheet's.
    AddMissing,
    Save,
}

pub(crate) fn shown(v: &VarValue) -> String {
    match v {
        VarValue::Null => String::new(),
        VarValue::Bool(b) => if *b { "evet" } else { "hayır" }.to_owned(),
        // As the web's number field: two decimals by the display rule (ADR 0149: a point).
        VarValue::Number(n) => kentos_geometry_core::display::fixed(*n, 2),
        VarValue::Text(t) => t.clone(),
    }
}

/// A typed value of a kind; none for a number that does not read.
pub(crate) fn value_of(kind: VarKind, text: &str) -> Option<VarValue> {
    let t = text.trim();
    if t.is_empty() {
        return Some(VarValue::Null);
    }
    Some(match kind {
        // A comma is a decimal point too (the web's `parseNumber`).
        VarKind::Number => VarValue::Number(
            t.replace(',', ".")
                .parse::<f64>()
                .ok()
                .filter(|v| v.is_finite())?,
        ),
        VarKind::Bool => VarValue::Bool(matches!(
            crate::text::fold(t).as_str(),
            "evet" | "true" | "1"
        )),
        VarKind::Text | VarKind::Date => VarValue::Text(t.to_owned()),
    })
}

/// A name the expressions can read: letters, digits and `_`, not starting with a digit.
fn clean_name(name: &str) -> Option<String> {
    let n = name.trim().trim_start_matches('@');
    let ok = !n.is_empty()
        && n.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        && !n.starts_with(|c: char| c.is_ascii_digit());
    ok.then(|| n.to_owned())
}

impl Designer {
    /// The names the preflight found without a value (`@kurum` of “… “@kurum” değerinin değeri yok …”).
    fn missing_names(&self) -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for f in self.findings.iter().filter(|f| f.code == "value_missing") {
            if let Some(at) = f.message.find('@') {
                let name: String = f.message[at + 1..]
                    .chars()
                    .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                    .collect();
                if !name.is_empty() && !out.contains(&name) {
                    out.push(name);
                }
            }
        }
        out
    }

    pub(crate) fn variables_update(&mut self, m: VariablesMessage) -> Vec<Effect> {
        if m == VariablesMessage::Open {
            let Some(s) = self.open_sheet() else {
                return Vec::new();
            };
            let own = s.variables.clone();
            let project = self.book.variables.clone();
            let texts = own
                .iter()
                .enumerate()
                .map(|(i, v)| (Scope::Sheet, i, shown(&v.value)))
                .chain(
                    project
                        .iter()
                        .enumerate()
                        .map(|(i, v)| (Scope::Project, i, shown(&v.value))),
                )
                .collect();
            self.dialog = Some(Window::Variables(VariablesDialog {
                own,
                project,
                texts,
                ..VariablesDialog::default()
            }));
            return Vec::new();
        }
        let missing = self.missing_names();
        let sheet = self.open.clone();
        let Some(Window::Variables(d)) = &mut self.dialog else {
            return Vec::new();
        };
        match m {
            VariablesMessage::Open => {}
            VariablesMessage::Text(s, i, text) => {
                if let Some(t) = d.texts.iter_mut().find(|(x, j, _)| *x == s && *j == i) {
                    t.2 = text;
                }
            }
            VariablesMessage::Bool(s, i, on) => {
                let vars = if s == Scope::Sheet {
                    &mut d.own
                } else {
                    &mut d.project
                };
                if let Some(v) = vars.get_mut(i) {
                    v.value = VarValue::Bool(on);
                }
            }
            VariablesMessage::NewName(s, text) => match s {
                Scope::Sheet => d.new_own = text,
                Scope::Project => d.new_project = text,
            },
            VariablesMessage::Add(s) => {
                let typed = if s == Scope::Sheet {
                    &d.new_own
                } else {
                    &d.new_project
                };
                match clean_name(typed) {
                    Some(name) if !d.own.iter().chain(d.project.iter()).any(|v| v.name == name) => {
                        let vars = if s == Scope::Sheet {
                            &mut d.own
                        } else {
                            &mut d.project
                        };
                        vars.push(Variable {
                            name: name.clone(),
                            label: name,
                            kind: VarKind::Text,
                            value: VarValue::Null,
                        });
                        let i = vars.len() - 1;
                        d.texts.push((s, i, String::new()));
                        match s {
                            Scope::Sheet => d.new_own.clear(),
                            Scope::Project => d.new_project.clear(),
                        }
                        d.problem = None;
                    }
                    Some(name) => d.problem = Some(format!("“@{name}” zaten var.")),
                    None => {
                        d.problem = Some(
                            "Ad harf, rakam ve _ olmalı; rakamla başlamamalı (metinde @ad).".into(),
                        )
                    }
                }
            }
            VariablesMessage::AddMissing => {
                for name in missing {
                    if !d.own.iter().chain(d.project.iter()).any(|v| v.name == name) {
                        d.own.push(Variable {
                            name: name.clone(),
                            label: name,
                            kind: VarKind::Text,
                            value: VarValue::Null,
                        });
                        d.texts.push((Scope::Sheet, d.own.len() - 1, String::new()));
                    }
                }
            }
            VariablesMessage::Save => {
                let mut own = d.own.clone();
                let mut project = d.project.clone();
                for (s, i, text) in &d.texts {
                    let vars = if *s == Scope::Sheet {
                        &mut own
                    } else {
                        &mut project
                    };
                    if let Some(v) = vars.get_mut(*i)
                        && v.kind != VarKind::Bool
                    {
                        match value_of(v.kind, text) {
                            Some(value) => v.value = value,
                            None => {
                                d.problem = Some(format!("“@{}” bir sayı olmalı.", v.name));
                                return Vec::new();
                            }
                        }
                    }
                }
                let ops = vec![
                    Op::SaveVariables(SaveVariables {
                        sheet,
                        variables: own,
                    }),
                    Op::SaveVariables(SaveVariables {
                        sheet: None,
                        variables: project,
                    }),
                ];
                if self.commit(ops) {
                    self.dialog = None;
                }
            }
        }
        Vec::new()
    }

    pub(crate) fn variables_view<'a>(&'a self, d: &'a VariablesDialog) -> Element<'a, Message> {
        let vm = Message::Variables;
        let empty = [&d.own, &d.project]
            .iter()
            .flat_map(|l| l.iter())
            .filter(|v| v.value.is_null())
            .count();
        let field = |s: Scope, i: usize, v: &'a Variable| -> Element<'a, Message> {
            let title = format!(
                "{} · @{}",
                if v.label.is_empty() {
                    &v.name
                } else {
                    &v.label
                },
                v.name
            );
            let input: Element<'a, Message> = match v.kind {
                VarKind::Bool => Switch::new(matches!(v.value, VarValue::Bool(true)), move |on| {
                    vm(VariablesMessage::Bool(s, i, on))
                })
                .into(),
                _ => {
                    let text = d
                        .texts
                        .iter()
                        .find(|(x, j, _)| *x == s && *j == i)
                        .map_or("", |t| t.2.as_str());
                    InputField::new("değeri yok", text)
                        .on_input(move |t| vm(VariablesMessage::Text(s, i, t)))
                        .width(Fill)
                        .into()
                }
            };
            column![label::caption(title).style(style::text::muted), input]
                .spacing(2)
                .into()
        };
        let list = |s: Scope, vars: &'a [Variable], none: &'static str| -> Element<'a, Message> {
            if vars.is_empty() {
                return label::caption(none).style(style::text::muted).into();
            }
            column(vars.iter().enumerate().map(|(i, v)| field(s, i, v)))
                .spacing(8)
                .into()
        };
        let add = |s: Scope, value: &'a str, what: &'static str| {
            row![
                InputField::new("yeni_ad", value)
                    .on_input(move |t| vm(VariablesMessage::NewName(s, t)))
                    .on_submit(vm(VariablesMessage::Add(s)))
                    .width(Fill),
                button(label::body(what))
                    .on_press(vm(VariablesMessage::Add(s)))
                    .padding([5, 10])
                    .style(style::button::secondary),
            ]
            .spacing(6)
            .align_y(Center)
        };
        let sheet_name = self
            .open_sheet()
            .map_or_else(String::new, |s| s.name.clone());
        let mut body = column![].spacing(10);
        if empty > 0 {
            body = body.push(
                container(label::body(format!(
                    "{empty} değişkenin değeri yok: kâğıda ‹ad?› yazılır ve ön denetim hata verir."
                )))
                .padding(8)
                .width(Fill)
                .style(style::container::bordered),
            );
        }
        let missing = self.missing_names();
        let unknown: Vec<&String> = missing
            .iter()
            .filter(|n| !d.own.iter().chain(d.project.iter()).any(|v| &v.name == *n))
            .collect();
        if !unknown.is_empty() {
            let names = unknown
                .iter()
                .map(|n| format!("@{n}"))
                .collect::<Vec<_>>()
                .join(", ");
            body = body.push(
                row![
                    label::caption(format!("Paftada kullanılıp tanımlanmamış: {names}"))
                        .width(Fill),
                    button(label::body("Paftaya ekle"))
                        .on_press(vm(VariablesMessage::AddMissing))
                        .padding([5, 10])
                        .style(style::button::secondary),
                ]
                .spacing(8)
                .align_y(Center),
            );
        }
        body = body
            .push(label::strong(format!("Paftanın değişkenleri ({sheet_name})")))
            .push(list(Scope::Sheet, &d.own, "Bu paftanın kendi değişkeni yok. Şablondan yapılan paftalar sorularını burada taşır."))
            .push(add(Scope::Sheet, &d.new_own, "Pafta değişkeni ekle"))
            .push(label::strong("Projenin değişkenleri"))
            .push(list(Scope::Project, &d.project, "Projenin değişkeni yok: bütün paftalarda aynı olan değerler (proje no, idare …) için ekleyin."))
            .push(add(Scope::Project, &d.new_project, "Proje değişkeni ekle"))
            .push(
                label::caption("Hazır değişkenler kendiliğinden dolar: @proje_adi, @pafta_adi, @sayfa, @sayfa_sayisi, @olcek, @tarih, @kullanici, @koordinat_sistemi.")
                    .style(style::text::muted),
            );
        if let Some(p) = &d.problem {
            body = body.push(label::body(p.as_str()).style(style::text::danger));
        }
        Dialog::new("Değişkenler")
            .push(scrollable(container(body).padding([0, 8])).height(420))
            .action(
                button(label::body("Vazgeç"))
                    .on_press(Message::CloseDialog)
                    .padding([6, 14])
                    .style(style::button::secondary),
            )
            .action(
                button(label::body("Kaydet"))
                    .on_press(vm(VariablesMessage::Save))
                    .padding([6, 16])
                    .style(style::button::primary),
            )
            .width(540.0)
            .into()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn values_and_names_read_as_typed() {
        // The display rule's point, and a comma as a decimal point too (the web's `parseNumber`).
        assert_eq!(
            value_of(VarKind::Number, "1234.5"),
            Some(VarValue::Number(1234.5))
        );
        assert_eq!(
            value_of(VarKind::Number, "12,5"),
            Some(VarValue::Number(12.5))
        );
        assert_eq!(value_of(VarKind::Number, "on iki"), None);
        assert_eq!(value_of(VarKind::Text, "  "), Some(VarValue::Null));
        assert_eq!(value_of(VarKind::Bool, "Evet"), Some(VarValue::Bool(true)));
        assert_eq!(clean_name("@pafta_no"), Some("pafta_no".into()));
        assert_eq!(clean_name("2ada"), None);
        assert_eq!(clean_name("ada no"), None);
        assert_eq!(shown(&VarValue::Number(12.5)), "12.50");
    }
}
