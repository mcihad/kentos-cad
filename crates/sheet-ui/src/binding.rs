//! ƒ: a property's value from an expression (docs/sheet/design.md §7; the
//! web's `ui/sheet/ExpressionDialog.ts`, word for word): what may be bound
//! and its unit are the core's (`bindable_properties`), the expression is
//! compiled by the core as it is typed (`kentos_sheet::expr::check`, the
//! same as the web's `checkExpression`: kentos-expression, no language of
//! its own here), and the names it may read are listed to put in with a
//! click: the ready ones (@pafta_adi, @olcek, @tarih …), the sheet's and the
//! project's variables, and an atlas object's fields. Kaydet is one
//! `SetItemProps` of the item's bindings; “Bağı kaldır” takes the binding
//! off and the property keeps its own value.
//!
//! The desktop's İfade oluşturucu (apps/desktop/src/expression) is not
//! used: its names are a layer's fields and the expression library's own
//! variables, and its check would call a sheet's `@pafta_adi` unknown.

use iced::widget::{button, column, container, row};
use iced::{Center, Element, Fill};
use kentos_sheet::bind::{BindType, Bindable, bind_type, bindable_properties};
use kentos_sheet::model::{Binding, ItemId};
use kentos_sheet::ops::{Op, SetItemProps};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::{Dialog, InputField};
use serde_json::json;

use crate::designer::{Designer, Dialog as Window, Effect};
use crate::message::Message;

/// The ready names every expression of a sheet may read.
pub const READY: [&str; 10] = [
    "@proje_adi",
    "@pafta_adi",
    "@sayfa",
    "@sayfa_sayisi",
    "@olcek",
    "@olcek_payda",
    "@kagit",
    "@tarih",
    "@kullanici",
    "@koordinat_sistemi",
];

#[derive(Clone, Debug, PartialEq)]
pub struct BindingDialog {
    pub item: ItemId,
    pub item_name: String,
    pub property: Bindable,
    pub text: String,
    /// The core's word on the text: none while it is empty; Ok when it compiles.
    pub check: Option<Result<(), String>>,
    /// The property is bound now (“Bağı kaldır” is offered).
    pub bound: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BindingMessage {
    /// ƒ pressed on an item's property.
    Open {
        item: ItemId,
        property: String,
    },
    Text(String),
    /// A name put in with a click.
    Insert(String),
    Save,
    Unbind,
}

fn checked(text: &str) -> Option<Result<(), String>> {
    let src = text.trim();
    (!src.is_empty()).then(|| kentos_sheet::expr::check(src))
}

/// The properties of an item's kind that may be bound, in the core's order.
pub fn bindables_of(kind: &kentos_sheet::kinds::ItemKind) -> Vec<Bindable> {
    bindable_properties()
        .into_iter()
        .filter(|b| bind_type(&b.property, kind).is_some())
        .collect()
}

impl Designer {
    pub(crate) fn binding_update(&mut self, m: BindingMessage) -> Vec<Effect> {
        match m {
            BindingMessage::Open { item, property } => {
                let Some(it) = self.book.item(&item) else {
                    return Vec::new();
                };
                let Some(p) = bindables_of(&it.kind)
                    .into_iter()
                    .find(|b| b.property == property)
                else {
                    return Vec::new();
                };
                let bound = it.bindings.iter().find(|b| b.property == property);
                let text = bound.map(|b| b.expression.clone()).unwrap_or_default();
                self.dialog = Some(Window::Binding(BindingDialog {
                    item: item.clone(),
                    item_name: it.name.clone(),
                    property: p,
                    check: checked(&text),
                    text,
                    bound: bound.is_some(),
                }));
            }
            BindingMessage::Text(t) => {
                if let Some(Window::Binding(d)) = &mut self.dialog {
                    d.check = checked(&t);
                    d.text = t;
                }
            }
            BindingMessage::Insert(name) => {
                if let Some(Window::Binding(d)) = &mut self.dialog {
                    d.text.push_str(&name);
                    d.check = checked(&d.text);
                }
            }
            BindingMessage::Save | BindingMessage::Unbind => {
                let save = m == BindingMessage::Save;
                let Some(Window::Binding(d)) = &self.dialog else {
                    return Vec::new();
                };
                if save && !matches!(d.check, Some(Ok(()))) {
                    return Vec::new();
                }
                let Some(it) = self.book.item(&d.item) else {
                    return Vec::new();
                };
                let mut bindings: Vec<Binding> = it
                    .bindings
                    .iter()
                    .filter(|b| b.property != d.property.property)
                    .cloned()
                    .collect();
                let label = if save {
                    bindings.push(Binding {
                        property: d.property.property.clone(),
                        expression: d.text.trim().to_owned(),
                    });
                    format!("Veriye bağla: {}", d.property.label)
                } else {
                    format!("Bağı kaldır: {}", d.property.label)
                };
                let op = Op::SetItemProps(SetItemProps {
                    id: d.item.clone(),
                    patch: json!({ "bindings": bindings }),
                });
                if self.commit_as(vec![op], Some(&label)) {
                    self.dialog = None;
                }
            }
        }
        Vec::new()
    }

    pub(crate) fn binding_view<'a>(&'a self, d: &'a BindingDialog) -> Element<'a, Message> {
        let bm = Message::Binding;
        let unit = if d.property.unit.is_empty() {
            String::new()
        } else {
            format!(" ({})", d.property.unit)
        };
        let kind = match d.property.value {
            BindType::Number => "sayı",
            BindType::Bool => "evet/hayır",
            BindType::Text => "metin",
        };
        let hint = format!(
            "Değer bu ifadeden gelir ({kind}{}). Pafta her çizildiğinde yeniden hesaplanır; hesaplanamazsa ön denetim söyler.",
            if d.property.unit.is_empty() {
                String::new()
            } else {
                format!(", {}", d.property.unit)
            }
        );
        let status: Element<'a, Message> = match &d.check {
            None => label::caption("Bir ifade yazın; aşağıdaki adlara tıklayınca yazılır.")
                .style(style::text::muted)
                .into(),
            Some(Ok(())) => row![
                icon(Icon::Success).size(14.0),
                label::caption("İfade geçerli.")
            ]
            .spacing(6)
            .align_y(Center)
            .into(),
            Some(Err(why)) => row![
                icon(Icon::Error).size(14.0),
                label::caption(format!("{why} (expression_error)")).style(style::text::danger)
            ]
            .spacing(6)
            .align_y(Center)
            .into(),
        };
        let chip = |name: String| {
            button(label::caption(name.clone()))
                .on_press(bm(BindingMessage::Insert(name)))
                .padding([2, 8])
                .style(style::button::secondary)
        };
        let mut ready = row![].spacing(4);
        for r in READY {
            ready = ready.push(chip(r.to_owned()));
        }
        let mut body = column![
            label::muted(hint),
            InputField::new("", &d.text)
                .on_input(move |t| bm(BindingMessage::Text(t)))
                .on_submit(bm(BindingMessage::Save))
                .width(Fill),
            status,
            label::strong("Hazır değişkenler"),
            ready.wrap(),
        ]
        .spacing(8);
        let sheet = self.open_sheet();
        let vars: Vec<(String, String)> = sheet
            .iter()
            .flat_map(|s| s.variables.iter())
            .chain(self.book.variables.iter())
            .map(|v| (v.name.clone(), v.label.clone()))
            .collect();
        if !vars.is_empty() {
            let mut list = row![].spacing(4);
            for (name, _) in vars {
                list = list.push(chip(format!("@{name}")));
            }
            body = body
                .push(label::strong("Paftanın ve projenin değişkenleri"))
                .push(list.wrap());
        }
        if let Some(layer) = sheet
            .and_then(|s| s.atlas.as_ref())
            .map(|a| a.layer.clone())
        {
            body = body.push(label::caption(format!(
                "Atlas sayfasında “{layer}” katmanının nesnesinin alanları da okunur: alan adını yalın ya da köşeli parantezle yazın ([Ada])."
            )).style(style::text::muted));
        }
        let mut w = Dialog::new(format!("ƒ {}{unit}: {}", d.property.label, d.item_name))
            .push(container(body).width(Fill));
        if d.bound {
            w = w.aside(
                button(label::body("Bağı kaldır"))
                    .on_press(bm(BindingMessage::Unbind))
                    .padding([6, 14])
                    .style(style::button::danger),
            );
        }
        w.action(
            button(label::body("Vazgeç"))
                .on_press(Message::CloseDialog)
                .padding([6, 14])
                .style(style::button::secondary),
        )
        .action(
            button(label::body("Kaydet"))
                .on_press_maybe(matches!(d.check, Some(Ok(()))).then(|| bm(BindingMessage::Save)))
                .padding([6, 16])
                .style(style::button::primary),
        )
        .width(560.0)
        .into()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_core_checks_a_binding_as_it_is_typed() {
        assert_eq!(super::checked("  "), None);
        assert_eq!(super::checked("@olcek_payda / 1000"), Some(Ok(())));
        assert!(matches!(super::checked("1 +"), Some(Err(_))));
    }
}
