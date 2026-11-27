//! Öznitelik tablosu's bar on a data layer (docs/adr/9166): the same tools as a drawing layer's table, in the same
//! order, so the two read alike. What a data layer's table can do works: Seçime yakınlaş, Sütunlar, Dışa aktar (the
//! layer written to a table or a file, as Katmanlar's Dışa aktar), İncele (İstatistik and Grafik read the source's rows)
//! and Alanlar…. What it cannot is off, its tip saying why and what to do instead: Form (a row's form opens with Formla
//! düzenle), Alan hesapla, Kopyala, Kayıt ekle, Çoğalt and Yapıştır. Ara and İfade süzgeci ask with Enter: a source is
//! not asked at every letter (data.rs).

use iced::widget::{container, row, space};
use iced::{Center, Element, Length};
use kentos_ui::icon::Icon;
use kentos_ui::widget::Menu;
use kentos_ui::widget::select::{Choice, Select};

use super::outgoing::{Analyse, ColumnsEvent, words as out_words};
use super::view::{bar_button, bar_gap, bar_menu};
use super::{Event, Rows, Show, texts};
use crate::app::{App, Message};
use crate::icons::from_web;

fn msg(e: Event) -> Message {
    Message::Gis(crate::gis::Event::Features(e))
}

/// Why a tool is off on a data layer's table, and what to do instead.
pub(crate) mod off {
    pub const FORM: &str = "Seçili satırın formu: katmanın formu yoksa alanlarından kurulan varsayılan form açılır.";
    pub const CALCULATE: &str =
        "Veri katmanında alan hesabı yok: düzenleme oturumunda hücreye çift tıklayıp değeri yazın.";
    pub const COPY: &str = "Veri katmanının satırları kaynaktan sayfa sayfa okunur, panoya kopyalanmaz. Satırları bir dosyaya yazmak için Dışa aktar'ı kullanın.";
    pub const ADD_RECORD: &str = "Veri katmanına yeni satır, düzenleme oturumunda çizim araçlarıyla eklenir.";
    pub const DUPLICATE: &str =
        "Veri katmanında satır çoğaltılmaz; yeni satırı düzenleme oturumunda çizim araçlarıyla ekleyin.";
    pub const PASTE: &str =
        "Veri katmanına panodan satır yapıştırılmaz; yeni satırı düzenleme oturumunda çizim araçlarıyla ekleyin.";
    pub const EXPORT: &str = "Katmanı bir dosyaya ya da veri kaynağına yazar; Katmanlar'daki Dışa aktar ile aynı.";
}

impl App {
    /// The table's layer's edit session beside its name (the second round's T9, docs/adr/9347): Düzenleniyor or
    /// Düzenlemeye kapalı; with more than one layer edited, Düzenlenen katmanlar ▾ lists them, each one press away
    /// (made the active layer and the table's).
    fn edit_state_badge<'a>(&self, layer: &str) -> Element<'a, Message> {
        let Some(doc) = self.document.as_ref().map(|d| &d.model) else {
            return space::horizontal().width(0).into();
        };
        let edited = doc.edited_layers();
        let editing = doc.editing(layer);
        let words = if editing { "Düzenleniyor" } else { "Düzenlemeye kapalı" };
        let tone = if editing { kentos_ui::style::text::accent } else { kentos_ui::style::text::muted };
        if edited.len() < 2 {
            return kentos_ui::label::caption(words).style(tone).into();
        }
        let named: Vec<(String, String)> =
            edited.iter().map(|l| (l.clone(), self.data_layer_name(l))).collect();
        let current = layer.to_owned();
        bar_menu(
            from_web(Some("edit")),
            Some(format!("{words} · düzenlenen {} katman", edited.len())),
            "Düzenlenen katmanlar".to_owned(),
            "Düzenlemesi açık katmanlar: birine basmak onu etkin katman ve tablonun katmanı yapar.",
            move || {
                let mut menu = Menu::new().header("Düzenlenen katmanlar");
                for (id, name) in &named {
                    menu = menu.check(name.clone(), *id == current, Some(msg(Event::EditedLayer(id.clone()))));
                }
                menu
            },
        )
    }

    /// The bar over a data layer's table (see the module).
    pub(super) fn data_layer_bar<'a>(&self, rows: &Rows) -> Element<'a, Message> {
        let panel = &self.gis.features;
        let layer = rows.layer.clone().unwrap_or_default();
        // Asked with Enter: a million rows are not searched at every letter (data.rs).
        let apply = msg(Event::Data(super::data::DataEvent::Apply));
        let search = kentos_ui::widget::text_field(super::data::words::SEARCH, &panel.search)
            .on_input(|t| msg(Event::Search(t)))
            .on_submit(apply.clone());
        let show = Select::new(
            Show::ALL.iter().map(|s| Choice::new(s.label())),
            Show::ALL.iter().position(|s| *s == panel.show),
            |i| msg(Event::Show(Show::ALL[i])),
        )
        .prefix(texts::SHOW);
        let filter = kentos_ui::widget::text_field(super::data::words::FILTER, &panel.filter)
            .on_input(|t| msg(Event::Filter(t)))
            .on_submit(apply);
        // Tablo | Form as a drawing layer's: Form says how a row's form opens here.
        let pane = kentos_ui::widget::segmented::Segmented::new(
            [super::Pane::Table, super::Pane::Form],
            super::Pane::Table,
            |p| msg(Event::Pane(p)),
        )
        .hints(["Tablonun satırları", off::FORM])
        .compact();
        // Süzülen katman | Kaynağın tümü, while the layer has a Katman süzgeci (docs/adr/9348).
        let filtered = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer))
            .and_then(|n| n.filter.as_ref())
            .and_then(|f| f.expression.as_ref())
            .is_some_and(|e| !e.trim().is_empty());
        let scope: Element<'a, Message> = if filtered {
            let all = panel.data.source_all;
            let select: Element<'a, Message> = Select::new(
                [Choice::new("Süzülen katman"), Choice::new("Kaynağın tümü")],
                Some(usize::from(all)),
                |i| msg(Event::SourceAll(i == 1)),
            )
            .prefix("Kapsam")
            .into();
            container(select).width(Length::Fixed(220.0)).into()
        } else {
            space::horizontal().width(0).into()
        };
        let filters = row![
            pane,
            container(self.layer_select(rows)).width(Length::Fixed(250.0)),
            self.edit_state_badge(&layer),
            scope,
            container(search).width(Length::Fixed(170.0)),
            container(show).width(Length::Fixed(150.0)),
            container(filter).width(Length::Fixed(240.0)),
        ]
        .spacing(8)
        .align_y(Center);
        // Sütunlar ▾: the source's fields, each shown or hidden in the table.
        let hidden = self.hidden_columns(rows.layer.as_deref());
        let named: Vec<String> = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer))
            .map(|n| n.fields.iter().map(|f| f.key().to_owned()).collect())
            .unwrap_or_default();
        let hidden_count = named.iter().filter(|k| hidden.contains(*k)).count();
        let columns_menu = bar_menu(
            from_web(Some("columns")),
            (hidden_count > 0).then(|| hidden_count.to_string()),
            if hidden_count == 0 {
                out_words::COLUMNS.to_owned()
            } else {
                format!("{} ({hidden_count} gizli)", out_words::COLUMNS)
            },
            out_words::COLUMNS_HINT,
            move || {
                let mut menu = Menu::new().header(out_words::COLUMNS_HEADER);
                for key in &named {
                    menu = menu
                        .check(key.clone(), !hidden.contains(key), Some(msg(Event::Columns(ColumnsEvent::Toggle(key.clone())))))
                        .stay();
                }
                menu.separator().item(out_words::SHOW_ALL, Some(msg(Event::Columns(ColumnsEvent::ShowAll))))
            },
        );
        // Dışa aktar ▾: the layer to a file or a source, the table's selected rows only too.
        let selected = self.layer_has_selection(&layer);
        let open = {
            let layer = layer.clone();
            move |only: bool| {
                Message::Gis(crate::gis::Event::LayerExport(crate::layer_export::Event::Open(layer.clone(), only)))
            }
        };
        let export = bar_menu(from_web(Some("export")), None, texts::EXPORT.to_owned(), off::EXPORT, move || {
            Menu::new()
                .item("Farklı kaydet…", Some(open(false)))
                .icon(from_web(Some("saveAs")))
                .item("Seçilenleri farklı kaydet…", selected.then(|| open(true)))
                .icon(from_web(Some("select")))
        });
        // İncele ▾: İstatistik and Grafik read the source's rows (docs/adr/9130).
        let statistics = self.shows_bottom_tab(crate::bottom::BottomTab::Statistics);
        let chart = self.shows_bottom_tab(crate::bottom::BottomTab::Chart);
        let analyse = bar_menu(
            from_web(Some("chartBar")),
            None,
            out_words::ANALYSE.to_owned(),
            out_words::ANALYSE_HINT,
            move || {
                let mut menu = Menu::new();
                if statistics {
                    menu = menu
                        .item(out_words::STATISTICS, Some(msg(Event::Analyse(Analyse::Statistics))))
                        .icon(from_web(Some("chartHistogram")));
                }
                if chart {
                    menu = menu
                        .item(out_words::CHART, Some(msg(Event::Analyse(Analyse::Chart))))
                        .icon(from_web(Some("chartPie")));
                }
                menu
            },
        );
        // ⋯: what a data layer's table offers of a drawing layer's, the rest off with why.
        let more = bar_menu(Icon::More, None, texts::MORE.to_owned(), texts::MORE_HINT, || {
            Menu::new()
                .item(texts::ADD_RECORD, None)
                .icon(from_web(Some("tableRowBelow")))
                .detail(off::ADD_RECORD)
                .item(texts::DUPLICATE, None)
                .icon(from_web(Some("copy")))
                .detail(off::DUPLICATE)
                .item(texts::PASTE, None)
                .icon(from_web(Some("paste")))
                .detail(off::PASTE)
                .separator()
                .item(texts::FIELDS, Some(msg(Event::Fields)))
                .icon(from_web(Some("layerFields")))
                .detail(texts::FIELDS_HINT)
        });
        let zoom_on = (!panel.data.selected.is_empty()).then(|| msg(Event::Data(super::data::DataEvent::Zoom)));
        let actions = row![
            bar_button("zoomSelection", texts::ZOOM, texts::ZOOM_HINT, zoom_on),
            columns_menu,
            bar_gap(),
            bar_button("fieldCalc", texts::CALCULATE, off::CALCULATE, None),
            bar_button("tableGridAll", out_words::COPY, off::COPY, None),
            export,
            analyse,
            bar_gap(),
            more,
        ]
        .spacing(2)
        .align_y(Center);
        // The layer drawn on, when the table shows another (docs/adr/9129): both are read, neither is taken for the other.
        let mut bar = row![filters, space::horizontal(), actions].spacing(8).align_y(Center);
        // Said with what each is for (docs/adr/9326): the table's selection is its layer's rows; the drawing tools draw
        // on the target.
        if let Some(t) = self.table_target_words(rows) {
            let table = rows.layer.as_deref().and_then(|id| {
                self.document.as_ref().and_then(|d| d.model.layers().get(id).map(|n| n.name.clone()))
            });
            let body = match table {
                Some(name) => format!(
                    "Tablo “{name}” katmanının satırlarını gösterir; seçtiğiniz satırlar onundur, Öznitelikler ve Koordinat \
                     listesi onları okur. Çizim araçları ise etkin katmana çizer."
                ),
                None => "Çizim araçları etkin katmana çizer.".to_owned(),
            };
            bar = bar.push(kentos_ui::widget::tip(
                kentos_ui::label::caption(t).style(kentos_ui::style::text::muted),
                kentos_ui::widget::Tip::new("Tablo ve çizim hedefi").body(body),
                iced::widget::tooltip::Position::Top,
            ));
        }
        container(bar.wrap().vertical_spacing(6)).padding([4, 8]).into()
    }
}

#[cfg(test)]
#[path = "data_bar_tests.rs"]
mod data_bar_tests;
