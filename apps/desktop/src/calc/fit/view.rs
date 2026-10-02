//! Vektör oturtma's view (the web's `FitDialog.ts`): the kind, Adla eşle,
//! the control points' table with its picks, the summary, what Uygula takes,
//! and the buttons.

use iced::widget::tooltip::Position;
use iced::widget::{button, column, container, row};
use iced::{Element, Fill};
use kentos_domain::Document as Model;
use kentos_interaction::Format;
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, Tip, tip};

use super::words::{Counted, KINDS, Named, kind_hint};
use super::{COLUMNS, Event, Form, Scope, Side, TITLE, fit_event};
use crate::app::Message;
use crate::calc::{Event as Calc, MAX_HEIGHT, Window, event, footer_button, grid, summary};
use crate::exchange::words;

impl Form {
    pub fn view<'a>(
        &'a self,
        model: &'a Model,
        format: &Format,
        selected: usize,
    ) -> Element<'a, Message> {
        let kind = words::field(
            "Dönüşüm",
            Segmented::new(KINDS, Named(self.kind), |k| fit_event(Event::Kind(k.0))),
            Some(kind_hint(self.kind).to_owned()),
        );
        let matching = row![
            container(words::field(
                "Kaynak noktaları",
                self.named_select(self.source.as_deref(), Event::Source),
                Some("Çizimdeki adlı noktalar".to_owned()),
            ))
            .width(Fill),
            container(words::field(
                "Hedef noktaları",
                self.named_select(self.target.as_deref(), Event::Target),
                Some("Oturtulacakları sistemdeki adlı noktalar".to_owned()),
            ))
            .width(Fill),
            words::field(
                "Adla eşle",
                words::secondary(
                    "Eşle",
                    (self.named.len() >= 2).then(|| fit_event(Event::Match)),
                ),
                Some("Aynı adlılar çift olur".to_owned()),
            ),
        ]
        .spacing(18);
        let table = grid::view_with(
            Window::Fit,
            &COLUMNS,
            self,
            |_, _| String::new(),
            2,
            |r| {
                vec![
                    row_pick(
                        r,
                        Side::Source,
                        "target",
                        format!("{}. satırın kaynağını çizimden seç", r + 1),
                        "Çizimdeki yerini gösterin; bir noktaya kenetlenirse adı da gelir.",
                    ),
                    row_pick(
                        r,
                        Side::Target,
                        "pin",
                        format!("{}. satırın hedefini çizimden seç", r + 1),
                        "Ülke sistemindeki noktası çizimdeyse onu gösterin.",
                    ),
                ]
            },
        );
        let mut body = column![
            kind,
            matching,
            column![label::strong("Kontrol noktaları"), table].spacing(8),
        ]
        .spacing(16);
        if let Some(summary) = summary(self.summary_lines(format)) {
            body = body.push(summary);
        }
        body = body.push(self.apply_row(model, selected));
        let can_apply = self.fit().is_some() && self.has_targets(model, selected);
        let mut dialog = Dialog::new(TITLE)
            .scroll(body)
            .action(footer_button(
                "Raporu kopyala",
                Some(event(Calc::CopyReport)),
                false,
            ))
            .action(footer_button("Kapat", Some(event(Calc::Close)), false))
            .action(footer_button(
                "Uygula",
                can_apply.then(|| fit_event(Event::Apply)),
                true,
            ))
            .max_height(MAX_HEIGHT)
            .width(980.0);
        if let Some(status) = &self.status {
            dialog = dialog.aside(label::caption(status.clone()).style(style::text::danger));
        }
        dialog.into()
    }

    /// A layer select over the layers holding named points, with their counts.
    fn named_select<'a>(
        &self,
        chosen: Option<&str>,
        on: fn(String) -> Event,
    ) -> Element<'a, Message> {
        if self.named.is_empty() {
            return Select::new([Choice::new("Adlı nokta yok")], Some(0), move |_| {
                fit_event(on(String::new()))
            })
            .searchable(false)
            .into();
        }
        let ids: Vec<String> = self.named.iter().map(|n| n.0.clone()).collect();
        let selected = chosen.and_then(|c| ids.iter().position(|id| id == c));
        let choices = self
            .named
            .iter()
            .map(|(_, path, n)| Choice::new(format!("{path} ({n})")));
        Select::new(choices, selected, move |i| {
            fit_event(on(ids.get(i).cloned().unwrap_or_default()))
        })
        .searchable(false)
        .into()
    }

    /// Uygula's objects (Seçili, Katman, Tümü), the layer when a layer's, and Kopya.
    fn apply_row<'a>(&'a self, model: &Model, selected: usize) -> Element<'a, Message> {
        let counts = [
            Counted(Scope::Selection, selected),
            Counted(Scope::Layer, 0),
            Counted(Scope::All, model.len()),
        ];
        let current = counts
            .iter()
            .copied()
            .find(|c| c.0 == self.scope)
            .unwrap_or(counts[2]);
        let scope = Segmented::new_with(
            counts,
            current,
            |c| fit_event(Event::Scope(c.0)),
            |c| c.0 != Scope::Selection || c.1 > 0,
        );
        let mut line = row![words::field("Uygulanacak nesneler", scope, None)].spacing(18);
        if self.scope == Scope::Layer {
            let layers = model.layers();
            let leaves: Vec<(String, String)> = layers
                .leaves()
                .into_iter()
                .map(|n| (n.id.clone(), layers.path(&n.id)))
                .collect();
            let selected = self
                .layer
                .as_deref()
                .and_then(|l| leaves.iter().position(|(id, _)| id == l));
            let choices: Vec<Choice> = leaves
                .iter()
                .map(|(_, path)| Choice::new(path.clone()))
                .collect();
            let ids: Vec<String> = leaves.into_iter().map(|(id, _)| id).collect();
            let select = Select::new(choices, selected, move |i| {
                fit_event(Event::Layer(ids.get(i).cloned().unwrap_or_default()))
            })
            .searchable(false);
            line = line.push(container(words::field("Katman", select, None)).width(Fill));
        }
        line.push(words::field(
            "Kopya",
            words::check(
                self.copy,
                "Kopya olarak: asıllar yerinde kalır",
                Some(fit_event(Event::Copy(!self.copy))),
            ),
            None,
        ))
        .into()
    }
}

/// A row's button that shows its source or target on the drawing (the web's `actions`).
fn row_pick<'a>(
    r: usize,
    side: Side,
    glyph: &str,
    title: String,
    body: &'static str,
) -> Element<'a, Message> {
    tip(
        button(icon(crate::icons::from_web(Some(glyph))).size(14.0))
            .on_press(fit_event(Event::Pick(r, side)))
            .padding(6)
            .style(style::button::ghost),
        Tip::new(title).body(body),
        Position::Top,
    )
}
