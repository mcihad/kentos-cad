//! Kenar eşleme's view (the web's `EdgematchDialog.ts`): Kaynak, Komşu and
//! Sınır; the search distance, the angle tolerance and the criterion; where
//! the ends meet and how; the links with Kullan and Göster; the summary and
//! the buttons.

use iced::widget::tooltip::Position;
use iced::widget::{button, column, container, row};
use iced::{Element, Fill};
use kentos_domain::Document as Model;
use kentos_ui::icon::icon;
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog, Tip, tip};

use super::words::{
    Counted, MEETS, METHODS, NamedMeet, NamedMethod, describe, meet_hint, method_hint,
};
use super::{COLUMNS, Event, Form, LAYER_KEY, Scope, TITLE, border_slot, edge_event, line_layers};
use crate::app::Message;
use crate::calc::{
    Event as Calc, MAX_HEIGHT, Window, event, footer_button, grid, number_field, summary,
};
use crate::exchange::words;

impl Form {
    pub fn view<'a>(&'a self, model: &'a Model, selected: usize) -> Element<'a, Message> {
        let mut body = column![
            self.sets_row(model, selected),
            self.numbers_row(model),
            row![
                container(words::field(
                    "Buluşma",
                    Segmented::new_with(
                        MEETS,
                        NamedMeet(self.meet),
                        |m| edge_event(Event::Meet(m.0)),
                        |m| m.0 != kentos_geometry_core::ops::edgematch::Meet::Border
                            || self.border.is_some(),
                    ),
                    Some(meet_hint(self.meet).to_owned()),
                ))
                .width(Fill),
                container(words::field(
                    "Yöntem",
                    Segmented::new(METHODS, NamedMethod(self.method), |m| {
                        edge_event(Event::Method(m.0))
                    }),
                    Some(method_hint(self.method).to_owned()),
                ))
                .width(Fill),
            ]
            .spacing(18),
        ]
        .spacing(16);
        let table = grid::view_with(
            Window::Edgematch,
            &COLUMNS,
            self,
            |_, _| String::new(),
            1,
            |r| {
                vec![tip(
                    button(icon(crate::icons::from_web(Some("zoomSelection"))).size(14.0))
                        .on_press(edge_event(Event::Show(r)))
                        .padding(6)
                        .style(style::button::ghost),
                    Tip::new(format!("{}. bağı çizimde göster", r + 1)).body(
                        "Pencere kenara çekilir, bağın iki çizgisi seçilip bağa yakınlaşılır. Tıklayın ya da Enter'a basın: pencere geri gelir.",
                    ),
                    Position::Top,
                )]
            },
        );
        body = body.push(column![label::strong("Bağlar"), table].spacing(8));
        if let Some(summary) = summary(self.summary_lines()) {
            body = body.push(summary);
        }
        let mut dialog = Dialog::new(TITLE)
            .scroll(body)
            .action(footer_button(
                "Raporu kopyala",
                self.found.is_some().then(|| event(Calc::CopyReport)),
                false,
            ))
            .action(footer_button("Kapat", Some(event(Calc::Close)), false))
            .action(footer_button(
                "Uygula",
                (!self.changes.is_empty()).then(|| edge_event(Event::Apply)),
                true,
            ))
            .max_height(MAX_HEIGHT)
            .width(980.0);
        if let Some(status) = &self.status {
            dialog = dialog.aside(label::caption(status.clone()).style(style::text::danger));
        }
        dialog.into()
    }

    /// Kaynak (Seçili or Katman, and its layer), Komşu and Sınır.
    fn sets_row<'a>(&'a self, model: &'a Model, selected: usize) -> Element<'a, Message> {
        let counts = [
            Counted(Scope::Selection, selected),
            Counted(Scope::Layer, 0),
        ];
        let current = counts
            .iter()
            .copied()
            .find(|c| c.0 == self.scope)
            .unwrap_or(counts[1]);
        let scope = Segmented::new_with(
            counts,
            current,
            |c| edge_event(Event::Scope(c.0)),
            |c| c.0 != Scope::Selection || c.1 > 0,
        );
        let layers = line_layers(model);
        let mut line = row![words::field(
            "Kaynak",
            scope,
            Some("Düzeltilecek çizgiler".to_owned())
        )]
        .spacing(18);
        if self.scope == Scope::Layer {
            line = line.push(
                container(words::field(
                    "Kaynak katmanı",
                    layer_select(&layers, self.source.as_deref(), Event::Source),
                    None,
                ))
                .width(Fill),
            );
        }
        line.push(
            container(words::field(
                "Komşu katmanı",
                layer_select(&layers, self.adjacent.as_deref(), Event::Adjacent),
                Some("Komşu paftanın çizgileri".to_owned()),
            ))
            .width(Fill),
        )
        .push(
            container(words::field(
                "Sınır",
                self.border_field(model),
                Some("İsteğe bağlı: pafta kenarı".to_owned()),
            ))
            .width(Fill),
        )
        .into()
    }

    /// Sınır: the object picked, read-only as a field, with Sahneden seç and its removal.
    fn border_field<'a>(&'a self, model: &'a Model) -> Element<'a, Message> {
        let layers = model.layers();
        let picked = self
            .border
            .as_deref()
            .and_then(|uid| border_slot(model, uid))
            .and_then(|s| model.get(s));
        let name = picked.map_or_else(
            || "Seçilmedi".to_owned(),
            |e| describe(e, &layers.path(&e.base().layer_id)),
        );
        let pick = tip(
            button(icon(crate::icons::from_web(Some("target"))).size(14.0))
                .on_press(edge_event(Event::PickBorder))
                .padding(6)
                .style(style::button::ghost),
            Tip::new("Sınırı çizimden seç").body(
                "Sahneden seç: pafta kenarı ya da çerçevesi (çizgi, çoklu çizgi ya da alan).",
            ),
            Position::Top,
        );
        let clear = tip(
            button(icon(crate::icons::from_web(Some("close"))).size(12.0))
                .on_press_maybe(picked.is_some().then(|| edge_event(Event::ClearBorder)))
                .padding(6)
                .style(style::button::ghost),
            Tip::new("Sınırı kaldır"),
            Position::Top,
        );
        row![
            container(label::body(name).style(style::text::muted))
                .padding([5, 8])
                .width(Fill)
                .style(style::container::field_box),
            pick,
            clear,
        ]
        .spacing(6)
        .align_y(iced::Center)
        .into()
    }

    /// The search distance, the angle tolerance and the criterion.
    fn numbers_row<'a>(&'a self, model: &'a Model) -> Element<'a, Message> {
        let number = |title: &'a str,
                      value: &'a str,
                      placeholder: &'a str,
                      hint: &'a str,
                      on: fn(String) -> Event| {
            column![
                number_field(title, value, placeholder, move |t| edge_event(on(t))),
                label::caption(hint),
            ]
            .spacing(4)
        };
        // The criterion: none, the layers' names, or an attribute of the two layers' objects.
        let mut attributes: Vec<String> = Vec::new();
        for e in model.entities() {
            let layer = e.base().layer_id.as_str();
            if Some(layer) == self.source.as_deref() || Some(layer) == self.adjacent.as_deref() {
                for k in e.base().attrs.keys() {
                    if !attributes.contains(k) {
                        attributes.push(k.clone());
                    }
                }
            }
        }
        attributes.sort_by(|a, b| kentos_expression::js::collate::compare_tr(a, b));
        let mut keys = vec![
            (String::new(), "Yok".to_owned()),
            (LAYER_KEY.to_owned(), "Katman adı".to_owned()),
        ];
        keys.extend(attributes.into_iter().map(|k| {
            let words = format!("Öznitelik: {k}");
            (k, words)
        }));
        let chosen = keys.iter().position(|(k, _)| *k == self.key);
        let ids: Vec<String> = keys.iter().map(|(k, _)| k.clone()).collect();
        let criterion = Select::new(
            keys.into_iter().map(|(_, w)| Choice::new(w)),
            chosen.or(Some(0)),
            move |i| edge_event(Event::Key(ids.get(i).cloned().unwrap_or_default())),
        )
        .searchable(false);
        row![
            number(
                "Arama uzaklığı (m)",
                &self.distance,
                "0.5",
                "Uçlar en çok bu kadar aralıklı",
                Event::Distance
            ),
            number(
                "Açı toleransı (°)",
                &self.angle,
                "30",
                "Çizgilerin doğrultusu en çok bu kadar sapar",
                Event::Angle
            ),
            container(words::field(
                "Eşleşme ölçütü",
                criterion,
                Some("Aynı değeri taşıyanlar eşlenir".to_owned())
            ))
            .width(Fill),
        ]
        .spacing(18)
        .into()
    }
}

/// A select over the layers holding line work, with their counts.
fn layer_select<'a>(
    layers: &[(String, String)],
    chosen: Option<&str>,
    on: fn(String) -> Event,
) -> Element<'a, Message> {
    if layers.is_empty() {
        return Select::new(
            [Choice::new("Çizgisi olan katman yok")],
            Some(0),
            move |_| edge_event(on(String::new())),
        )
        .searchable(false)
        .into();
    }
    let ids: Vec<String> = layers.iter().map(|(id, _)| id.clone()).collect();
    let selected = chosen.and_then(|c| ids.iter().position(|id| id == c));
    let choices: Vec<Choice> = layers.iter().map(|(_, w)| Choice::new(w.clone())).collect();
    Select::new(choices, selected, move |i| {
        edge_event(on(ids.get(i).cloned().unwrap_or_default()))
    })
    .searchable(false)
    .into()
}
