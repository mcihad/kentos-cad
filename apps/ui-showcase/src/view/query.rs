//! "Öznitelikle seç" ve "Tabloyu filtrele" pencereleri.
//!
//! ```text
//! ┌ Öznitelikle seç ─────────────────────────────── SORGU ┐
//! │ Katman    [Şehirler                              ▾]   │
//! │ Yöntem    [Yeni seçim | Seçime ekle | Çıkar | İçinde] │
//! │ [Bölge ▾] [= ▾] [Marmara                   ▾] ×       │
//! │ + Koşul ekle                                          │
//! │ ┌ Bölge = "Marmara"                                ┐  │
//! │ └ 2 / 16 kayıt eşleşiyor                           ┘  │
//! │                                   [Vazgeç] [Uygula]   │
//! ```

use iced::widget::{button, column, container, pick_list, row, space};
use iced::{Center, Element, Fill};

use kentos_ui::attribute::{Field, Query};
use kentos_ui::label;
use kentos_ui::spatial::SelectionMode;
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::{Dialog, QueryBuilder, Segmented, overlay};

use super::LayerChoice;
use crate::app::{QueryDialog, Showcase};
use crate::command::{self, Command};
use crate::message::{Message, QueryPurpose};

/// Alan adlarının sütun genişliği.
const KEY_WIDTH: f32 = 64.0;

impl Showcase {
    pub(super) fn query_dialog<'a>(&'a self, dialog: &'a QueryDialog) -> Element<'a, Message> {
        let Some(layer) = self.layers.get(dialog.layer) else {
            return space::vertical().height(0).into();
        };

        let schema = &layer.schema;
        let problem = problem(&dialog.query, schema);
        let total = layer.features.len();
        let matches = layer
            .features
            .iter()
            .filter(|feature| dialog.query.matches(schema, &feature.values))
            .count();

        let (title, command) = match dialog.purpose {
            QueryPurpose::Select => ("Öznitelikle seç", Command::SelectByAttributes),
            QueryPurpose::Filter => ("Tabloyu filtrele", Command::Filter),
        };

        let target: Element<'a, Message> = match dialog.purpose {
            QueryPurpose::Select => {
                let choices = self.layer_choices();
                let current = choices.get(dialog.layer).cloned();

                pick_list(choices, current, |choice: LayerChoice<'_>| {
                    Message::QueryLayerSelected(choice.index)
                })
                .font(typography::ui())
                .text_size(typography::body())
                .padding([3, 8])
                .width(Fill)
                .style(style::field::pick_list)
                .menu_style(style::field::menu)
                .into()
            }
            QueryPurpose::Filter => label::body(layer.name.as_str()).into(),
        };

        let mut body = column![setting("Katman", target)].spacing(10);

        if dialog.purpose == QueryPurpose::Select {
            body = body.push(setting(
                "Yöntem",
                Segmented::new(SelectionMode::ALL, dialog.mode, Message::QueryModeSelected)
                    .width(Fill)
                    .into(),
            ));
        }

        let expression = if dialog.query.is_empty() {
            "Koşul yok".to_owned()
        } else {
            dialog.query.describe(schema)
        };

        let outcome = match (problem, dialog.purpose) {
            (Some(problem), _) => problem.to_owned(),
            (None, QueryPurpose::Select) => format!("{matches} / {total} kayıt eşleşiyor"),
            (None, QueryPurpose::Filter) => format!("Tabloda {matches} / {total} kayıt görünecek"),
        };

        let preview = container(
            column![
                label::mono_caption(expression).style(style::text::default),
                label::caption(outcome),
            ]
            .spacing(4),
        )
        .padding([8, 10])
        .width(Fill)
        .style(style::container::field);

        let body = body
            .push(QueryBuilder::new(
                schema,
                &dialog.query,
                Message::QueryEdited,
            ))
            .push(preview);

        let apply = match dialog.purpose {
            QueryPurpose::Filter if dialog.query.is_empty() => "Filtreyi kaldır",
            QueryPurpose::Filter => "Filtrele",
            QueryPurpose::Select => "Seç",
        };

        let dialog = Dialog::new(title)
            .hint(command::name(command))
            .width(640.0)
            .push(body)
            .action(
                button(label::body("Vazgeç"))
                    .on_press(Message::QueryClosed)
                    .padding([5, 16])
                    .style(style::button::secondary),
            )
            .action(
                button(label::body(apply))
                    .on_press_maybe(problem.is_none().then_some(Message::QueryApplied))
                    .padding([5, 16])
                    .style(style::button::primary),
            );

        overlay::modal(dialog, Message::QueryClosed)
    }
}

/// Sorgu uygulanamıyorsa nedeni: değeri çözümlenemeyen ya da boş koşul.
pub(super) fn problem(query: &Query, schema: &[Field]) -> Option<&'static str> {
    let errors = query.errors(schema);

    if errors.is_empty() {
        return None;
    }

    let invalid = errors.iter().any(|(index, _)| {
        query
            .conditions
            .get(*index)
            .is_some_and(|condition| !condition.value.trim().is_empty())
    });

    Some(if invalid {
        "Hatalı koşulları düzeltin."
    } else {
        "Değeri boş koşulları tamamlayın."
    })
}

/// Pencerenin ayar satırı: solda ad, sağda denetim.
fn setting<'a>(name: &'a str, control: Element<'a, Message>) -> Element<'a, Message> {
    row![
        label::muted(name).width(typography::scaled(KEY_WIDTH)),
        control
    ]
    .spacing(12)
    .align_y(Center)
    .into()
}
