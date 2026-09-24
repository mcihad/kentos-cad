//! Öznitelik tablosu: model alanının altında, aktif katmanın kayıtları.
//!
//! ArcGIS'teki öznitelik tablosu gibi: araç çubuğunda katman, arama, filtre
//! ve seçim eylemleri; başlığa tıklayarak sıralama; satıra tıklayarak seçim
//! (Shift aralık seçer, Ctrl satırı seçime ekler ya da çıkarır).

use iced::widget::{button, column, pick_list, row, space, tooltip};
use iced::{Center, Element};

use kentos_rc::attribute::FieldKind;
use kentos_rc::icon::{Icon, Tone, icon};
use kentos_rc::label;
use kentos_rc::spatial::{Feature, Layer};
use kentos_rc::style;
use kentos_rc::theme::typography;
use kentos_rc::widget::table::{self, Table};
use kentos_rc::widget::{Panel, Tip, Toolbar, tip};

use super::LayerChoice;
use crate::app::Showcase;
use crate::message::{Message, QueryPurpose};
use crate::table::{Column, TableView};

/// Panelin yüksekliği: başlık, araç çubuğu ve yaklaşık yedi satır.
const HEIGHT: f32 = 252.0;

/// Araç çubuğunda gösterilen filtre açıklamasının en fazla uzunluğu.
const FILTER_SUMMARY: usize = 64;

impl Showcase {
    pub(super) fn attribute_table(&self) -> Element<'_, Message> {
        let index = self.active_layer;

        let (Some(layer), Some(view)) = (self.layers.get(index), self.active_table()) else {
            return space::vertical().height(0).into();
        };

        let rows = self.table_rows();
        let total = layer.features.len();
        let selected = self.selection.count_in(index);

        let mut meta = if rows.len() == total {
            format!("{total} kayıt")
        } else {
            format!("{} / {total} kayıt", rows.len())
        };

        if selected > 0 {
            meta.push_str(&format!(", {selected} seçili"));
        }

        let columns = Column::all(layer);

        let headers = columns.iter().map(|&column| {
            let header = table::Column::new(column.title(layer))
                .width(column.width(&self.layers, layer))
                .sortable(view.order_of(column), Message::TableSort(column));

            if column.is_numeric(layer) {
                header.align_right()
            } else {
                header
            }
        });

        let primary = self.selection.primary();

        let body = rows.iter().filter_map(|&reference| {
            let feature = layer.feature(reference.id)?;

            Some(
                table::Row::new(
                    columns
                        .iter()
                        .map(|&column| self.cell(layer, feature, column)),
                )
                .selected(self.selection.contains(&reference))
                .current(primary == Some(reference))
                .on_press(Message::TableRowPressed(reference)),
            )
        });

        // Tablo model alanıyla aynı genişliktedir; dar tabloların başlığı ve
        // satır vurgusu da uçtan uca uzanır.
        let table = Table::new(headers)
            .extend(body)
            .horizontal()
            .min_width(self.viewport.size.width)
            .height(iced::Fill)
            .empty(empty_message(view, index == crate::app::DRAWING_LAYER));

        let close = tip(
            button(icon(Icon::Close).size(12.0))
                .on_press(Message::TableToggled)
                .padding([1, 4])
                .style(style::button::subtle),
            Tip::new("Tabloyu kapat"),
            tooltip::Position::Left,
        );

        Panel::new(
            "Öznitelik tablosu",
            column![self.table_toolbar(layer, view), table],
        )
        .meta(meta)
        .trailing(close)
        .height(HEIGHT)
        .into()
    }

    fn table_toolbar<'a>(&'a self, layer: &'a Layer, view: &'a TableView) -> Toolbar<'a, Message> {
        let choices = self.layer_choices();
        let current = choices.get(self.active_layer).cloned();
        let has_selection = !self.selection.is_empty();
        let filtered = !view.filter.is_empty();

        let mut toolbar = Toolbar::new()
            .push(
                pick_list(choices, current, |choice: LayerChoice<'_>| {
                    Message::LayerActivated(choice.index)
                })
                .text_size(typography::BODY)
                .padding([2, 8])
                .width(150)
                .style(style::field::pick_list)
                .menu_style(style::field::menu),
            )
            .search(&view.search, "Tabloda ara", Message::TableSearch)
            .separator()
            .toggle(
                Icon::Filter,
                "Filtre",
                filtered,
                Message::QueryOpened(QueryPurpose::Filter),
            );

        if filtered {
            toolbar = toolbar.button(Icon::Close, "Filtreyi kaldır", Some(Message::FilterCleared));
        }

        toolbar = toolbar
            .toggle(
                Icon::Check,
                "Yalnızca seçili",
                view.selected_only,
                Message::TableSelectedOnly,
            )
            .separator()
            .button(
                Icon::SelectAll,
                "Tümünü seç (Ctrl+A)",
                Some(Message::SelectAll),
            )
            .button(
                Icon::InvertSelection,
                "Seçimi tersine çevir",
                Some(Message::InvertSelection),
            )
            .button(
                Icon::ClearSelection,
                "Seçimi kaldır",
                has_selection.then_some(Message::ClearSelection),
            )
            .button(
                Icon::Target,
                "Seçime yakınlaştır",
                has_selection.then_some(Message::FocusSelection),
            )
            .spacer();

        if filtered {
            toolbar = toolbar.push(
                row![
                    icon(Icon::Filter).size(12.0).tone(Tone::Accent),
                    label::mono_caption(shorten(
                        &view.filter.describe(&layer.schema),
                        FILTER_SUMMARY
                    )),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }

        toolbar
    }

    /// Hücre: boş değer tireyle, nesne başvurusu bağlantı ikonuyla, sayı
    /// ve tarihler eş aralıklı yazılır.
    fn cell<'a>(
        &'a self,
        layer: &'a Layer,
        feature: &'a Feature,
        column: Column,
    ) -> Element<'a, Message> {
        let content = column.text(&self.layers, layer, feature);

        if content.is_empty() {
            return label::caption("—").into();
        }

        if let Column::Field(field) = column
            && let Some(FieldKind::Object { .. }) = layer.schema.get(field).map(|field| &field.kind)
        {
            return row![
                icon(Icon::Link).size(12.0).tone(Tone::Muted),
                label::body(content)
            ]
            .spacing(4)
            .align_y(Center)
            .into();
        }

        match column {
            Column::Id => label::mono_caption(content).into(),
            _ if column.is_monospaced(layer) => label::mono(content).into(),
            _ => label::body(content).into(),
        }
    }
}

/// Satır yokken nedeni ve ne yapılabileceği.
fn empty_message(view: &TableView, drawings: bool) -> &'static str {
    if !view.filter.is_empty() || !view.search.trim().is_empty() {
        "Filtreye ya da aramaya uyan kayıt yok."
    } else if view.selected_only {
        "Bu katmanda seçili kayıt yok."
    } else if drawings {
        "Henüz çizim yok. Şeritteki çizim araçlarıyla ekleyebilirsiniz."
    } else {
        "Bu katmanda kayıt yok."
    }
}

/// Uzun metni sonuna üç nokta koyarak kısaltır.
fn shorten(text: &str, limit: usize) -> String {
    if text.chars().count() <= limit {
        text.to_owned()
    } else {
        let mut short: String = text.chars().take(limit - 1).collect();
        short.push('…');
        short
    }
}
