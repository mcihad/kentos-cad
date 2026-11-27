//! Katman özellikleri (the owner's word of 27 November 2026, docs/adr/9350): a CBS layer's summary in one window, for
//! reading. The bottom of the layer's right-click menu opens it. Its head names the layer, its kind and where its data
//! is; its sections, listed on the left as a layer's properties are elsewhere, say:
//!
//! - **Genel:** the group path, visibility, lock, the active layer, the edit session and what waits for the source.
//! - **Kaynak:** the connection (GeoPackage, PostgreSQL, a map service, the project file), its address without a
//!   password, the table, its key and geometry column, its rows as the source counts them, read only or not.
//! - **Koordinat sistemi:** the table's system beside the project's, and whether the rows are carried across.
//! - **Stil:** how it is drawn (one symbol, categories, classes, rules …), its colours, labels and filter.
//! - **Alanlar:** each field with its alias, kind and rules.
//! - **Formlar:** the layer's forms with their releases.
//!
//! Nothing here writes: every value is read from the drawing and, for the row count, from the source.

use iced::widget::{Column, button, column, container, row, space, text};
use iced::{Center, Element, Fill, Length, Task};
use kentos_commands::{LayerFieldKind, LayerNode};
use kentos_ui::icon::{Tone, icon};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::{ListDetails, Pairs};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::gis;
use crate::icons::from_web;
use crate::layer_entries::LayerKind;

/// The window's title.
pub const TITLE: &str = "Katman özellikleri";

/// A section of the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    Source,
    Crs,
    Style,
    Fields,
    Forms,
}

impl Section {
    const ALL: [Section; 6] =
        [Section::General, Section::Source, Section::Crs, Section::Style, Section::Fields, Section::Forms];

    fn title(self) -> &'static str {
        match self {
            Section::General => "Genel",
            Section::Source => "Kaynak",
            Section::Crs => "Koordinat sistemi",
            Section::Style => "Stil",
            Section::Fields => "Alanlar",
            Section::Forms => "Formlar",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Section::General => "info",
            Section::Source => "sourceLinked",
            Section::Crs => "crs",
            Section::Style => "layerStyle",
            Section::Fields => "layerFields",
            Section::Forms => "formEdit",
        }
    }
}

/// The window: the layer and the section shown.
#[derive(Debug, Clone)]
pub struct Window {
    pub layer: String,
    pub section: Section,
}

#[derive(Debug, Clone)]
pub enum Event {
    Open(String),
    Show(Section),
    Close,
}

fn msg(e: Event) -> Message {
    Message::Gis(gis::Event::LayerProperties(e))
}

/// The menu's message for `layer`.
pub(crate) fn open(layer: String) -> Message {
    msg(Event::Open(layer))
}

/// A layer's kind in words.
fn kind_words(kind: LayerKind) -> &'static str {
    match kind {
        LayerKind::Drawing => "Çizim katmanı",
        LayerKind::Annotation => "Açıklama katmanı",
        LayerKind::Data => "Veri katmanı",
        LayerKind::Bound => "Bağlı katman",
        LayerKind::Service => "Servis katmanı",
        LayerKind::Raster => "Raster katmanı",
        LayerKind::PointCloud => "Nokta bulutu",
        LayerKind::Group => "Grup",
    }
}

/// A renderer's kind in words (the style's `renderer.type`).
fn renderer_words(kind: Option<&str>) -> &'static str {
    match kind {
        None | Some("single") => "Tek sembol",
        Some("categorized") => "Kategorilere göre",
        Some("graduated") => "Sınıflara göre",
        Some("rules") => "Kurallara göre",
        Some("unclassed") => "Sürekli renk",
        Some("proportional") => "Orantılı sembol",
        Some("bivariate") => "İki değişkenli",
        Some("dotDensity") => "Nokta yoğunluğu",
        Some("chart") => "Grafik",
        Some("heatmap") => "Isı haritası",
        Some("cluster") => "Kümeleme",
        Some("displacement") => "Yer değiştirme",
        Some("inverted") => "Ters çokgen",
        Some(_) => "Özel",
    }
}

/// An address said without a password.
fn shown_address(address: &str) -> String {
    match address.split_once("://") {
        Some((scheme, rest)) => match rest.split_once('@') {
            Some((who, host)) => format!("{scheme}://{}@{host}", who.split(':').next().unwrap_or(who)),
            None => address.to_owned(),
        },
        None => address.strip_prefix("gpkg:").unwrap_or(address).to_owned(),
    }
}

impl App {
    pub(crate) fn layer_properties_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open(layer) => {
                self.gis.layer_properties = Some(Window { layer, section: Section::General });
                self.dialog = Some(Dialog::Gis(gis::GisDialogKind::LayerProperties));
            }
            Event::Show(section) => {
                if let Some(w) = self.gis.layer_properties.as_mut() {
                    w.section = section;
                }
            }
            Event::Close => {
                self.gis.layer_properties = None;
                if self.dialog == Some(Dialog::Gis(gis::GisDialogKind::LayerProperties)) {
                    self.dialog = None;
                }
            }
        }
        Task::none()
    }

    pub(crate) fn layer_properties_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.gis.layer_properties, &self.document) else {
            return text("").into();
        };
        let Some(node) = doc.model.layers().get(&w.layer) else {
            return Frame::new(TITLE)
                .push(label::muted("Katman artık projede yok."))
                .action(crate::exchange::words::secondary("Kapat", Some(msg(Event::Close))))
                .modal(msg(Event::Close));
        };
        let kind = self.layer_kind(node);
        // The head: the layer, its kind and where its data is.
        let chip = |words: String| container(label::caption(words)).padding([1, 8]).style(style::container::badge);
        let mut chips = row![chip(kind_words(kind).to_owned())].spacing(6).align_y(Center);
        if let Some(d) = &node.data {
            chips = chips.push(chip(if d.driver == "gpkg" { "GeoPackage".to_owned() } else { "PostgreSQL".to_owned() }));
            if d.read_only {
                chips = chips.push(chip("Salt okunur".to_owned()));
            }
        }
        if doc.model.editing(&node.id) {
            chips = chips.push(chip("Düzenleniyor".to_owned()));
        }
        let head = row![
            container(icon(from_web(Some(Section::General.glyph()))).size(22.0).tone(Tone::Accent)).padding([2, 4]),
            column![label::title(node.name.clone()), chips].spacing(6),
        ]
        .spacing(12)
        .align_y(Center);
        let list = Section::ALL.iter().fold(Column::new().spacing(2).padding([6, 6]), |list, s| {
            list.push(
                button(
                    row![icon(from_web(Some(s.glyph()))).size(15.0).tone(Tone::Muted), label::body(s.title())]
                        .spacing(10)
                        .align_y(Center),
                )
                .width(Fill)
                .padding([7, 10])
                .style(style::button::list_item(w.section == *s))
                .on_press(msg(Event::Show(*s))),
            )
        });
        let details: Element<'_, Message> = match w.section {
            Section::General => self.properties_general(node, kind),
            Section::Source => self.properties_source(node, kind),
            Section::Crs => self.properties_crs(node),
            Section::Style => self.properties_style(node),
            Section::Fields => self.properties_fields(node),
            Section::Forms => self.properties_forms(node),
        };
        let details = column![label::strong(w.section.title()), details].spacing(12).padding([10, 18]);
        Frame::new(TITLE)
            .push(column![head, ListDetails::new(list, details).list_width(200.0).height(380.0)].spacing(16))
            .status(label::caption("Bilgi içindir; değiştirmek için katmanın menüsündeki pencereleri kullanın.").style(style::text::muted))
            .action(crate::exchange::words::secondary("Kapat", Some(msg(Event::Close))))
            .width(760.0)
            .modal(msg(Event::Close))
    }

    fn properties_general<'a>(&self, node: &LayerNode, kind: LayerKind) -> Element<'a, Message> {
        let Some(doc) = self.document.as_ref() else {
            return space().into();
        };
        let layers = doc.model.layers();
        let yes = |on: bool| if on { "Evet" } else { "Hayır" };
        let mut pairs = Pairs::new()
            .push(name("Ad"), value(node.name.clone()))
            .push(name("Yeri"), value(layers.path(&node.id)))
            .push(name("Türü"), value(kind_words(kind)))
            .push(name("Görünür"), value(yes(layers.is_visible(&node.id))))
            .push(name("Kilitli"), value(yes(layers.is_locked(&node.id))))
            .push(name("Etkin katman"), value(yes(layers.active() == node.id)));
        if node.data.is_some() {
            let editing = doc.model.editing(&node.id);
            let c = doc.model.data_changes(&node.id);
            let waiting = c.added.len() + c.changed.len() + c.deleted.len();
            pairs = pairs.push(
                name("Düzenleme"),
                value(match (editing, waiting) {
                    (false, _) => "Kapalı".to_owned(),
                    (true, 0) => "Açık; kaynağa yazılmamış kayıt yok".to_owned(),
                    (true, n) => format!("Açık; {n} kayıt kaynağa yazılmadı"),
                }),
            );
        } else if matches!(kind, LayerKind::Drawing | LayerKind::Annotation | LayerKind::Bound) {
            pairs = pairs.push(name("Nesne sayısı"), value(doc.model.by_layer(&node.id).count().to_string()));
        }
        pairs.into()
    }

    fn properties_source<'a>(&self, node: &LayerNode, kind: LayerKind) -> Element<'a, Message> {
        let mut pairs = Pairs::new();
        if let Some(d) = &node.data {
            let rows = match self.source_count_of(&node.id) {
                crate::source_counts::Counted::Ready(n) => crate::view::thousands(n as f64),
                crate::source_counts::Counted::Counting => "sayılıyor…".to_owned(),
                crate::source_counts::Counted::Failed(why) => format!("sayılamadı: {why}"),
            };
            pairs = pairs
                .push(name("Bağlantı"), value(if d.driver == "gpkg" { "GeoPackage dosyası" } else { "PostgreSQL / PostGIS veritabanı" }))
                .push(name(if d.driver == "gpkg" { "Dosya" } else { "Veritabanı" }), value(shown_address(&d.address)))
                .push(name("Tablo"), value(d.table.clone()))
                .push(name("Anahtar sütunu"), value(d.key.clone()))
                .push(
                    name("Geometri"),
                    value(d.geometry.as_ref().map_or_else(
                        || "Yok (geometrisiz tablo)".to_owned(),
                        |g| format!("{} · sütun “{}”", g.type_name, g.column),
                    )),
                )
                .push(name("Satır"), value(rows))
                .push(name("Yazma"), value(if d.read_only { "Salt okunur" } else { "Düzenleme oturumuyla yazılır" }));
            if let Some(rel) = &d.relative {
                pairs = pairs.push(name("Projeye göre yolu"), value(rel.clone()));
            }
            return pairs.into();
        }
        if let Some(s) = &node.service {
            pairs = pairs
                .push(name("Bağlantı"), value(format!("Harita servisi · {}", s.kind.label())))
                .push(name("Adres"), value(shown_address(&s.url)));
            if !s.layers.is_empty() {
                pairs = pairs.push(name("Servis katmanları"), value(s.layers.join(", ")));
            }
            return pairs.into();
        }
        let link = self.document.as_ref().and_then(|d| d.model.settings().sources.iter().find(|s| s.layer == node.id).cloned());
        if let Some(link) = link {
            return pairs
                .push(name("Bağlantı"), value("Bağlı tablo: nesneler projede, tabloya yazılır"))
                .push(name("Kaynak"), value(shown_address(&link.path)))
                .push(name("Tablo"), value(link.table.clone()))
                .into();
        }
        pairs
            .push(
                name("Bağlantı"),
                value(match kind {
                    LayerKind::Annotation => "Proje dosyası: yazılar, ölçüler ve paftalar projeyle saklanır",
                    _ => "Proje dosyası: nesneler projeyle saklanır",
                }),
            )
            .into()
    }

    fn properties_crs<'a>(&self, node: &LayerNode) -> Element<'a, Message> {
        let Some(doc) = self.document.as_ref() else {
            return space().into();
        };
        let settings = doc.model.settings();
        let project = system_title(settings);
        let mut pairs = Pairs::new().push(name("Projenin sistemi"), value(project));
        match node.data.as_ref().and_then(|d| d.geometry.as_ref()) {
            Some(g) => {
                let table = g.srid.map_or_else(|| "Tabloda yazılı değil".to_owned(), srid_title);
                let carried = match g.srid {
                    Some(srid) if srid == settings.srid => "Aynı sistem: satırlar olduğu gibi çizilir".to_owned(),
                    Some(_) => "Satırlar çizilirken projenin sistemine dönüştürülür; kaynaktaki koordinatlar değişmez".to_owned(),
                    None => "Tablonun sistemi bilinmiyor: satırlar projenin sisteminde sayılır".to_owned(),
                };
                pairs = pairs.push(name("Tablonun sistemi"), value(table)).push(name("Çizim"), value(carried));
            }
            None if node.data.is_some() => {
                pairs = pairs.push(name("Tablonun sistemi"), value("Geometrisiz tablo"));
            }
            None => {
                pairs = pairs.push(name("Katmanın sistemi"), value("Projenin sistemi (nesneler projeyle saklanır)"));
            }
        }
        pairs.into()
    }

    fn properties_style<'a>(&self, node: &LayerNode) -> Element<'a, Message> {
        let s = &node.style;
        let kind = s.renderer.as_ref().and_then(|r| r.get("type")).and_then(|t| t.as_str());
        let swatch = |hex: &str| {
            let color = crate::view::hex_color(hex);
            row![
                container(space().width(14).height(14)).style(style::container::swatch(color)),
                label::body(hex.to_owned())
            ]
            .spacing(8)
            .align_y(Center)
        };
        let mut pairs = Pairs::new().push(name("Çizim biçimi"), value(renderer_words(kind)));
        pairs = pairs.push(name("Renk"), swatch(&s.color));
        if let Some(fill) = &s.fill {
            pairs = pairs.push(name("Dolgu"), swatch(fill));
        }
        pairs = pairs
            .push(name("Etiketler"), value(if s.labels_on() { "Açık" } else { "Kapalı" }))
            .push(
                name("Süzgeç"),
                value(
                    node.filter
                        .as_ref()
                        .and_then(|f| f.expression.clone())
                        .filter(|e| !e.trim().is_empty())
                        .unwrap_or_else(|| "Yok: bütün kayıtlar çizilir".to_owned()),
                ),
            );
        pairs.into()
    }

    fn properties_fields<'a>(&self, node: &LayerNode) -> Element<'a, Message> {
        if node.fields.is_empty() {
            return label::muted("Alan yok: katmanın öznitelikleri serbest.").into();
        }
        let mut list = Column::new().spacing(2);
        for f in &node.fields {
            let mut marks: Vec<String> = Vec::new();
            if f.required {
                marks.push("zorunlu".into());
            }
            if f.unique {
                marks.push("tekil".into());
            }
            if let Some(v) = f.values.as_ref().filter(|v| !v.is_empty()) {
                marks.push(format!("{} değerli liste", v.len()));
            }
            if f.min.is_some() || f.max.is_some() {
                marks.push("aralık".into());
            }
            let kind = match f.kind {
                LayerFieldKind::Text => f.length.map_or_else(|| "Metin".to_owned(), |n| format!("Metin ({n})")),
                k => k.label().to_owned(),
            };
            list = list.push(
                container(
                    row![
                        {
                            let mut names = column![label::body(f.name.clone())].spacing(1);
                            if let Some(alias) = f.alias.clone().filter(|a| !a.trim().is_empty()) {
                                names = names.push(label::caption(alias).style(style::text::muted));
                            }
                            names.width(Length::FillPortion(3))
                        },
                        container(label::caption(kind).style(style::text::muted)).width(Length::FillPortion(2)),
                        container(label::caption(marks.join(" · ")).style(style::text::accent)).width(Length::FillPortion(3)),
                    ]
                    .spacing(10)
                    .align_y(Center),
                )
                .padding([5, 8]),
            );
        }
        column![label::caption(format!("{} alan", node.fields.len())).style(style::text::muted), list]
            .spacing(6)
            .into()
    }

    fn properties_forms<'a>(&self, node: &LayerNode) -> Element<'a, Message> {
        let forms: Vec<&kentos_commands::ProjectForm> = self.project_forms().iter().filter(|f| f.layer == node.id).collect();
        if forms.is_empty() {
            return label::muted(
                "Katmanın formu yok. Kayıtlar alanlarından kurulan varsayılan formla açılır; Veri › Formlar'da form yapılır.",
            )
            .into();
        }
        let mut pairs = Pairs::new();
        for f in forms {
            let state = if f.withdrawn {
                "Yayından kaldırıldı".to_owned()
            } else {
                match f.releases.last() {
                    Some(r) => format!("{}. yayın", r.version),
                    None => "Taslak (yayınlanmadı)".to_owned(),
                }
            };
            pairs = pairs.push(name(f.name.clone()), value(state));
        }
        pairs.into()
    }
}

/// A system by its SRID as a sentence names it (the shared core's words): “TUREF / TM36 (EPSG:5256)”.
fn srid_title(srid: u32) -> String {
    kentos_project::crs::system(srid).map_or_else(|| format!("EPSG:{srid}"), kentos_project::crs::title)
}

/// The project's system: its own definition's name, else by its SRID.
fn system_title(settings: &kentos_commands::ProjectSettings) -> String {
    match settings.custom_crs.as_ref().filter(|_| settings.srid == kentos_project::crs::LOCAL_SRID) {
        Some(d) => kentos_project::systems::definition_title(d),
        None => srid_title(settings.srid),
    }
}

/// A pair's name, muted.
fn name<'a>(words: impl Into<String>) -> Element<'a, Message> {
    label::caption(words.into()).style(style::text::muted).into()
}

/// A pair's value, wrapping.
fn value<'a>(words: impl Into<String>) -> Element<'a, Message> {
    label::body(words.into()).wrapping(iced::widget::text::Wrapping::WordOrGlyph).into()
}

#[cfg(test)]
#[path = "layer_properties_tests.rs"]
mod layer_properties_tests;
