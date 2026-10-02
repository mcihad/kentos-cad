//! Sorgu oluşturucu: "öznitelikle seç" ve tablo filtreleri için koşul
//! düzenleyicisi.
//!
//! ```text
//! [Tüm koşullar (VE) | Herhangi biri (VEYA)]
//! [Bölge      ▾] [=  ▾] [Marmara       ▾] ×
//! [Nüfus      ▾] [>  ▾] [2.000.000       ] ×
//! + Koşul ekle
//! ```
//!
//! Bileşen sorguyu değiştirmez; her değişiklik bir [`Edit`] olarak gelir ve
//! uygulama onu [`Query::apply`] ile uygular. Değer düzenleyicisi alan
//! türüne göre değişir: seçenekli ve evet/hayır alanlarda açılır liste,
//! diğerlerinde metin girişi. Çözümlenemeyen değerler koşulun altında
//! açıklanır; değeri henüz girilmemiş koşul hata sayılmaz ama
//! [`Query::errors`] onu eksik olarak bildirmeye devam eder, böylece
//! uygulama "Uygula" düğmesini kapalı tutabilir.

use std::collections::BTreeMap;
use std::rc::Rc;

use iced::widget::{Column, button, column, container, row, text_input};
use iced::{Center, Element, Fill};

use crate::attribute::query::Edit;
use crate::attribute::{Combinator, Field, FieldKind, Operator, Query};
use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{metrics, typography};
use crate::widget::{Choice, Segmented, Select};

/// Sorgu oluşturucu.
pub struct QueryBuilder<'a, Message> {
    schema: &'a [Field],
    query: &'a Query,
    on_edit: Rc<dyn Fn(Edit) -> Message + 'a>,
}

impl<'a, Message: Clone + 'a> QueryBuilder<'a, Message> {
    pub fn new(
        schema: &'a [Field],
        query: &'a Query,
        on_edit: impl Fn(Edit) -> Message + 'a,
    ) -> Self {
        Self {
            schema,
            query,
            on_edit: Rc::new(on_edit),
        }
    }

    fn emit(&self, edit: Edit) -> Message {
        (self.on_edit)(edit)
    }

    fn value_editor(
        &self,
        index: usize,
        field: &Field,
        value: &str,
        invalid: bool,
    ) -> Element<'a, Message> {
        let on_edit = self.on_edit.clone();

        let choices: Option<Vec<String>> = match &field.kind {
            FieldKind::Choice(options) => Some(options.clone()),
            FieldKind::Bool => Some(vec!["Evet".to_owned(), "Hayır".to_owned()]),
            _ => None,
        };

        match choices {
            Some(choices) => {
                let selected = choices.iter().position(|choice| choice.as_str() == value);
                let names = choices.clone();

                Select::new(
                    choices.into_iter().map(Choice::new),
                    selected,
                    move |chosen| on_edit(Edit::Value(index, names[chosen].clone())),
                )
                .placeholder("Seçin")
                .into()
            }
            None => {
                let placeholder = match field.placeholder() {
                    "" => "Değer",
                    placeholder => placeholder,
                };

                text_input(placeholder, value)
                    .on_input(move |value| on_edit(Edit::Value(index, value)))
                    .font(typography::ui())
                    .size(typography::body())
                    .padding(metrics::padding(metrics::control(), 8.0))
                    .width(Fill)
                    .style(move |theme, status| {
                        let mut style = style::field::input(theme, status);

                        if invalid {
                            style.border.color = crate::theme::Tokens::of(theme).danger;
                        }

                        style
                    })
                    .into()
            }
        }
    }
}

/// Alan seçim listesinin seçeneği.
#[derive(Debug, Clone, PartialEq)]
struct FieldChoice {
    index: usize,
    name: String,
}

impl<'a, Message: Clone + 'a> From<QueryBuilder<'a, Message>> for Element<'a, Message> {
    fn from(builder: QueryBuilder<'a, Message>) -> Self {
        let errors: BTreeMap<usize, String> =
            builder.query.errors(builder.schema).into_iter().collect();

        let fields: Vec<FieldChoice> = builder
            .schema
            .iter()
            .enumerate()
            .map(|(index, field)| FieldChoice {
                index,
                name: field.name.clone(),
            })
            .collect();

        let mut content = Column::new().spacing(8);

        if builder.query.conditions.len() > 1 {
            let on_edit = builder.on_edit.clone();

            content = content.push(Segmented::new(
                Combinator::ALL,
                builder.query.combinator,
                move |combinator| on_edit(Edit::Combinator(combinator)),
            ));
        }

        if builder.query.is_empty() {
            content = content.push(label::muted("Koşul yok: bütün kayıtlar eşleşir."));
        }

        for (index, condition) in builder.query.conditions.iter().enumerate() {
            let Some(field) = builder.schema.get(condition.field) else {
                continue;
            };

            let on_field = builder.on_edit.clone();
            let on_operator = builder.on_edit.clone();

            // Boş değer henüz girilmemiştir; kırmızıyla gösterilmez.
            let error = errors
                .get(&index)
                .filter(|_| !condition.value.trim().is_empty());

            let value: Element<'a, Message> = if condition.operator.needs_value() {
                builder.value_editor(index, field, &condition.value, error.is_some())
            } else {
                container(label::muted("—"))
                    .padding([0, 8])
                    .height(metrics::control())
                    .align_y(Center)
                    .width(Fill)
                    .into()
            };

            let field_targets: Vec<usize> = fields.iter().map(|choice| choice.index).collect();
            let operators = Operator::for_kind(&field.kind).to_vec();
            let operator = operators
                .iter()
                .position(|operator| *operator == condition.operator);

            let line = row![
                container(Select::new(
                    fields
                        .iter()
                        .map(|choice| Choice::new(choice.name.clone()))
                        .collect::<Vec<_>>(),
                    field_targets
                        .iter()
                        .position(|target| *target == condition.field),
                    move |chosen| on_field(Edit::Field(index, field_targets[chosen])),
                ))
                .width(typography::scaled(170.0)),
                container(Select::new(
                    operators
                        .iter()
                        .map(|operator| Choice::new(operator.to_string()))
                        .collect::<Vec<_>>(),
                    operator,
                    move |chosen| on_operator(Edit::Operator(index, operators[chosen])),
                ))
                .width(typography::scaled(120.0)),
                value,
                button(container(icon(Icon::Close).size(12.0)).center_y(Fill))
                    .on_press(builder.emit(Edit::Remove(index)))
                    .padding([0, 7])
                    .height(metrics::control())
                    .style(style::button::subtle),
            ]
            .spacing(6)
            .align_y(Center);

            let mut block = column![line].spacing(3);

            if let Some(error) = error {
                block = block.push(
                    row![
                        icon(Icon::Warning).size(12.0).tone(Tone::Danger),
                        label::caption(error.clone()).style(style::text::danger),
                    ]
                    .spacing(6)
                    .align_y(Center),
                );
            }

            content = content.push(block);
        }

        content
            .push(
                button(
                    row![icon(Icon::Plus), label::body("Koşul ekle")]
                        .spacing(6)
                        .align_y(Center),
                )
                .on_press(builder.emit(Edit::Add))
                .padding([3, 8])
                .style(style::button::flat),
            )
            .into()
    }
}
