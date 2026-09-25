//! Öznitelik tablosu: yuvanın alt alanında, aktif katmanın kayıtları.
//!
//! ArcGIS'teki öznitelik tablosu gibi: araç çubuğunda katman, arama, filtre
//! ve seçim eylemleri; başlığa tıklayarak sıralama; satıra tıklayarak seçim
//! (Shift aralık seçer, Ctrl satırı seçime ekler ya da çıkarır).

use iced::widget::text::Wrapping;
use iced::widget::{column, pick_list, row, space};
use iced::{Center, Element};

use kentos_ui::attribute::{FieldKind, text};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::spatial::{Feature, Layer, Tool};
use kentos_ui::style;
use kentos_ui::theme::typography;
use kentos_ui::widget::table::{self, Table};
use kentos_ui::widget::{EmptyState, Toolbar};

use super::LayerChoice;
use crate::app::Showcase;
use crate::message::{Message, QueryPurpose};
use crate::table::{Column, TableView};

/// Araç çubuğunda gösterilen filtre açıklamasının en fazla uzunluğu.
const FILTER_SUMMARY: usize = 64;

impl Showcase {
    /// Tablonun başlıktaki özeti: kayıt ve seçim sayısı.
    pub(super) fn table_meta(&self) -> String {
        let index = self.active_layer;

        let Some(layer) = self.layers.get(index) else {
            return String::new();
        };

        let rows = self.table_rows().len();
        let total = layer.features.len();
        let selected = self.selection.count_in(index);

        let mut meta = if rows == total {
            format!("{total} kayıt")
        } else {
            format!("{rows} / {total} kayıt")
        };

        if selected > 0 {
            meta.push_str(&format!(", {selected} seçili"));
        }

        meta
    }

    /// Yuvadaki öznitelik tablosu: araç çubuğu ve aktif katmanın kayıtları.
    pub(super) fn attribute_table(&self) -> Element<'_, Message> {
        let index = self.active_layer;

        let (Some(layer), Some(view)) = (self.layers.get(index), self.active_table()) else {
            return space::vertical().height(0).into();
        };

        let rows = self.table_rows();

        let columns = Column::all(layer);

        let headers: Vec<table::Column<'_, Message>> = columns
            .iter()
            .map(|&column| {
                let header = table::Column::new(column.title(layer))
                    .width(column.width(&self.layers, layer))
                    .sortable(view.order_of(column), Message::TableSort(column));

                if column.is_numeric(layer) {
                    header.align_right()
                } else {
                    header
                }
            })
            .collect();

        let primary = self.selection.primary();
        let reveal = primary.and_then(|primary| rows.iter().position(|row| *row == primary));
        let count = rows.len();

        // Satırlar sanaldır: yalnızca görünenler kurulur. Haritada seçilen
        // öğenin satırı görünür yapılır.
        let row = move |index: usize| {
            let reference = rows[index];

            match layer.feature(reference.id) {
                Some(feature) => table::Row::new(
                    columns
                        .iter()
                        .map(|&column| self.cell(layer, feature, column)),
                )
                .selected(self.selection.contains(&reference))
                .current(primary == Some(reference))
                .on_press(Message::TableRowPressed(reference))
                .menu(move |_| self.row_menu(reference)),
                None => table::Row::new([]),
            }
        };

        // Satır yokken tablonun yerinde nedeni ve yapılabilecek iş yazar.
        // Tablo model alanıyla aynı genişliktedir; dar tabloların başlığı ve
        // satır vurgusu da uçtan uca uzanır.
        let table: Element<'_, Message> = if count == 0 {
            empty_table(layer, view, index == crate::app::DRAWING_LAYER)
        } else {
            Table::new(headers)
                .virtualized(count, row)
                .reveal(reveal)
                .horizontal()
                .min_width(self.viewport.size.width)
                .height(iced::Fill)
                .into()
        };

        column![self.table_toolbar(layer, view), table].into()
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
                .font(typography::ui())
                .text_size(typography::body())
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
        // Hücreler tek satırdır: çok satırlı metnin ilk satırı gösterilir.
        let content = text::first_line(&column.text(&self.layers, layer, feature)).into_owned();

        if content.is_empty() {
            return label::caption("—").into();
        }

        if let Column::Field(field) = column
            && let Some(FieldKind::Object { .. }) = layer.schema.get(field).map(|field| &field.kind)
        {
            return row![
                icon(Icon::Link).size(12.0).tone(Tone::Muted),
                label::body(content).wrapping(Wrapping::None)
            ]
            .spacing(4)
            .align_y(Center)
            .into();
        }

        // Hücreler tek satırdır; sütuna sığmayan metin kırpılır.
        match column {
            Column::Id => label::mono_caption(content).wrapping(Wrapping::None).into(),
            _ if column.is_monospaced(layer) => {
                label::mono(content).wrapping(Wrapping::None).into()
            }
            _ => label::body(content).wrapping(Wrapping::None).into(),
        }
    }
}

/// Satır yokken nedeni ve ne yapılabileceği.
fn empty_table<'a>(layer: &'a Layer, view: &'a TableView, drawings: bool) -> Element<'a, Message> {
    let search = view.search.trim();

    let state = if !view.filter.is_empty() {
        EmptyState::new(Icon::Filter, "Filtreye uyan kayıt yok")
            .description(format!(
                "{}: {}",
                layer.name,
                shorten(&view.filter.describe(&layer.schema), FILTER_SUMMARY)
            ))
            .primary("Filtreyi kaldır", Message::FilterCleared)
            .secondary(
                "Filtreyi düzenle",
                Message::QueryOpened(QueryPurpose::Filter),
            )
    } else if !search.is_empty() {
        EmptyState::new(Icon::Search, format!("\"{search}\" için kayıt yok"))
            .description(
                "Arama bütün alanlarda, büyük küçük harf ve Türkçe karakter ayırmadan yapılır.",
            )
            .primary("Aramayı temizle", Message::TableSearch(String::new()))
    } else if view.selected_only {
        EmptyState::new(Icon::Select, "Bu katmanda seçili kayıt yok")
            .description("Haritada ya da tabloda kayıt seçin ya da bütün kayıtları gösterin.")
            .primary("Bütün kayıtlar", Message::TableSelectedOnly)
    } else if drawings {
        EmptyState::new(Icon::Polyline, "Henüz çizim yok")
            .description(
                "Şeritteki çizim araçlarıyla ya da komut kutusundan (CIZGI, ALAN) çizin; \
                 çizimler bu tabloya eklenir.",
            )
            .primary("Çizgi çiz", Message::ToolSelected(Tool::Line))
    } else {
        EmptyState::new(Icon::Table, "Bu katmanda kayıt yok")
    };

    state.into()
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
