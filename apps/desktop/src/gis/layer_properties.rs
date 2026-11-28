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
//!
//! The second round's T19–T21 (docs/adr/9356):
//!
//! - **One way on:** Kaynak copies its file or address (Yolu kopyala); Stil, Alanlar and Formlar open their windows on
//!   this layer (Stil ayarlarını aç, Alanları aç, Formları aç). The summary stays for reading.
//! - **A theme's colour by its name:** “fg” reads “Tema ön planı” with the colour it is now, said to follow the theme;
//!   a colour of its own by its hex.
//! - **Keys:** the list has the keyboard when the window opens, drawn with a ring: ↑ ↓ Home End choose a section; Tab
//!   (Shift+Tab) goes from the list to the section's action and Kapat; Enter presses what has the ring; Esc closes and
//!   gives the layer tree the keyboard on the layer.

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
    const ALL: [Section; 6] = [
        Section::General,
        Section::Source,
        Section::Crs,
        Section::Style,
        Section::Fields,
        Section::Forms,
    ];

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

/// What has the keyboard's ring (T21).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Focus {
    #[default]
    List,
    Action,
    Close,
}

/// The window: the layer, the section shown and what has the keyboard.
#[derive(Debug, Clone)]
pub struct Window {
    pub layer: String,
    pub section: Section,
    pub focus: Focus,
}

/// A section's one way on (T19).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
    CopyPath,
    Style,
    Fields,
    Forms,
}

impl Action {
    fn words(self) -> &'static str {
        match self {
            Action::CopyPath => "Yolu kopyala",
            Action::Style => "Stil ayarlarını aç",
            Action::Fields => "Alanları aç",
            Action::Forms => "Formları aç",
        }
    }

    fn glyph(self) -> &'static str {
        match self {
            Action::CopyPath => "copy",
            Action::Style => "layerStyle",
            Action::Fields => "layerFields",
            Action::Forms => "formEdit",
        }
    }
}

/// A key the window takes (T21).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Home,
    End,
    Tab(bool),
    Enter,
}

#[derive(Debug, Clone)]
pub enum Event {
    Open(String),
    Show(Section),
    Act(Action),
    Key(Key),
    /// Esc or Kapat: `keys`, from the keyboard (the layer tree gets it on the layer).
    Close,
    CloseByKey,
}

fn msg(e: Event) -> Message {
    Message::Gis(gis::Event::LayerProperties(e))
}

/// The menu's message for `layer`.
pub(crate) fn open(layer: String) -> Message {
    msg(Event::Open(layer))
}

/// A key pressed while the window is open (input.rs).
pub(crate) fn key(k: Key) -> Message {
    msg(Event::Key(k))
}

/// Esc while the window is open (input.rs): closed, the layer tree given the keyboard on the layer.
pub(crate) fn close_by_key() -> Message {
    msg(Event::CloseByKey)
}

/// A theme's colour by its name in a layer's style (T20): the drawing's own words; none for a colour of its own.
fn theme_color_words(token: &str) -> Option<&'static str> {
    match token {
        "fg" => Some("Tema ön planı"),
        "fg-dim" => Some("Tema ön planı, soluk"),
        "ink" => Some("Çizim mürekkebi"),
        "paper" => Some("Kâğıt (arka plan)"),
        _ => None,
    }
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
            Some((who, host)) => {
                format!("{scheme}://{}@{host}", who.split(':').next().unwrap_or(who))
            }
            None => address.to_owned(),
        },
        None => address.strip_prefix("gpkg:").unwrap_or(address).to_owned(),
    }
}

impl App {
    pub(crate) fn layer_properties_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Open(layer) => {
                self.gis.layer_properties = Some(Window {
                    layer,
                    section: Section::General,
                    focus: Focus::List,
                });
                self.dialog = Some(Dialog::Gis(gis::GisDialogKind::LayerProperties));
            }
            Event::Show(section) => {
                if let Some(w) = self.gis.layer_properties.as_mut() {
                    w.section = section;
                    w.focus = Focus::List;
                }
            }
            Event::Act(action) => return self.layer_properties_act(action),
            Event::Key(k) => return self.layer_properties_key(k),
            Event::Close => self.layer_properties_close(false),
            Event::CloseByKey => self.layer_properties_close(true),
        }
        Task::none()
    }

    /// The window closed; from the keyboard, the layer tree gets it on the layer (T21).
    fn layer_properties_close(&mut self, keys: bool) {
        let layer = self.gis.layer_properties.take().map(|w| w.layer);
        if self.dialog == Some(Dialog::Gis(gis::GisDialogKind::LayerProperties)) {
            self.dialog = None;
        }
        if keys && let Some(id) = layer {
            self.selected_layer = Some(id.clone());
            self.layers_follow = false;
            self.layer_reveal = Some(id);
            self.layers_keyboard = true;
        }
    }

    /// The section's way on (T19): the path copied, or the layer's own window opened on it (this one closes).
    fn layer_properties_act(&mut self, action: Action) -> Task<Message> {
        let Some(id) = self.gis.layer_properties.as_ref().map(|w| w.layer.clone()) else {
            return Task::none();
        };
        if action == Action::CopyPath {
            let Some(path) = self.layer_properties_path(&id) else {
                return Task::none();
            };
            self.say(
                kentos_interaction::Level::Success,
                format!("Yol panoya kopyalandı: {path}"),
            );
            return iced::clipboard::write(path);
        }
        self.layer_properties_close(false);
        match action {
            Action::Style => {
                self.open_layer_style(Some(id));
                Task::none()
            }
            Action::Fields => self.open_layer_fields(&id),
            Action::Forms => {
                self.open_forms_for_layer(&id);
                Task::none()
            }
            Action::CopyPath => Task::none(),
        }
    }

    /// The keys (T21): ↑ ↓ Home End the sections, Tab the ring round the list, the action and Kapat, Enter what has it.
    fn layer_properties_key(&mut self, k: Key) -> Task<Message> {
        let Some(w) = self.gis.layer_properties.as_ref() else {
            return Task::none();
        };
        let action = self.layer_properties_action(w.section, &w.layer);
        let (section, focus) = (w.section, w.focus);
        let at = Section::ALL.iter().position(|s| *s == section).unwrap_or(0);
        let last = Section::ALL.len() - 1;
        let to = |i: usize| Section::ALL[i.min(last)];
        let ring = |f: Focus, back: bool| {
            let order: Vec<Focus> = [Focus::List, Focus::Action, Focus::Close]
                .into_iter()
                .filter(|f| *f != Focus::Action || action.is_some())
                .collect();
            let i = order.iter().position(|o| *o == f).unwrap_or(0);
            let n = order.len();
            order[if back { (i + n - 1) % n } else { (i + 1) % n }]
        };
        match k {
            Key::Up | Key::Down | Key::Home | Key::End => {
                let next = match k {
                    Key::Up => to(at.saturating_sub(1)),
                    Key::Down => to(at + 1),
                    Key::Home => Section::ALL[0],
                    _ => Section::ALL[last],
                };
                if let Some(w) = self.gis.layer_properties.as_mut() {
                    w.section = next;
                    w.focus = Focus::List;
                }
            }
            Key::Tab(back) => {
                if let Some(w) = self.gis.layer_properties.as_mut() {
                    w.focus = ring(focus, back);
                }
            }
            Key::Enter => match focus {
                Focus::Action => {
                    return action.map_or_else(Task::none, |a| self.layer_properties_act(a));
                }
                Focus::Close => self.layer_properties_close(true),
                Focus::List => {}
            },
        }
        Task::none()
    }

    /// A section's one way on for `layer` (T19); none where it has none.
    fn layer_properties_action(&self, section: Section, layer: &str) -> Option<Action> {
        match section {
            Section::Source => self.layer_properties_path(layer).map(|_| Action::CopyPath),
            Section::Style => Some(Action::Style),
            Section::Fields => Some(Action::Fields),
            Section::Forms => Some(Action::Forms),
            Section::General | Section::Crs => None,
        }
    }

    /// The layer's file or address as Kaynak shows it (no password); none for a layer of the project file.
    fn layer_properties_path(&self, layer: &str) -> Option<String> {
        let doc = self.document.as_ref()?;
        let node = doc.model.layers().get(layer)?;
        if let Some(d) = &node.data {
            return Some(shown_address(&d.address));
        }
        if let Some(s) = &node.service {
            return Some(shown_address(&s.url));
        }
        doc.model
            .settings()
            .sources
            .iter()
            .find(|s| s.layer == layer)
            .map(|s| shown_address(&s.path))
    }

    pub(crate) fn layer_properties_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.gis.layer_properties, &self.document) else {
            return text("").into();
        };
        let Some(node) = doc.model.layers().get(&w.layer) else {
            return Frame::new(TITLE)
                .push(label::muted("Katman artık projede yok."))
                .action(crate::exchange::words::secondary(
                    "Kapat",
                    Some(msg(Event::Close)),
                ))
                .modal(msg(Event::Close));
        };
        let kind = self.layer_kind(node);
        // The head: the layer, its kind and where its data is.
        let chip = |words: String| {
            container(label::caption(words))
                .padding([1, 8])
                .style(style::container::badge)
        };
        let mut chips = row![chip(kind_words(kind).to_owned())]
            .spacing(6)
            .align_y(Center);
        if let Some(d) = &node.data {
            chips = chips.push(chip(if d.driver == "gpkg" {
                "GeoPackage".to_owned()
            } else {
                "PostgreSQL".to_owned()
            }));
            if d.read_only {
                chips = chips.push(chip("Salt okunur".to_owned()));
            }
        }
        if doc.model.editing(&node.id) {
            chips = chips.push(chip("Düzenleniyor".to_owned()));
        }
        let head = row![
            container(
                icon(from_web(Some(Section::General.glyph())))
                    .size(22.0)
                    .tone(Tone::Accent)
            )
            .padding([2, 4]),
            column![label::title(node.name.clone()), chips].spacing(6),
        ]
        .spacing(12)
        .align_y(Center);
        // The keyboard's ring (T21): round the chosen section while the list has the keys, else round the action or
        // Kapat.
        let ring = |on: bool| {
            move |theme: &iced::Theme| {
                let t = kentos_ui::theme::Tokens::of(theme);
                style::container::outline(
                    if on {
                        t.accent
                    } else {
                        iced::Color::TRANSPARENT
                    },
                    2.0,
                )(theme)
            }
        };
        let list = Section::ALL
            .iter()
            .fold(Column::new().spacing(2).padding([6, 6]), |list, s| {
                let chosen = w.section == *s;
                list.push(
                    container(
                        button(
                            row![
                                icon(from_web(Some(s.glyph()))).size(15.0).tone(Tone::Muted),
                                label::body(s.title())
                            ]
                            .spacing(10)
                            .align_y(Center),
                        )
                        .width(Fill)
                        .padding([7, 10])
                        .style(style::button::list_item(chosen))
                        .on_press(msg(Event::Show(*s))),
                    )
                    .padding(1)
                    .style(ring(chosen && w.focus == Focus::List)),
                )
            });
        let list = column![
            list,
            space::vertical(),
            container(
                label::caption("↑ ↓ bölüm · Tab eylem ve Kapat · Enter basar · Esc kapatır")
                    .style(style::text::muted)
                    .wrapping(iced::widget::text::Wrapping::Word),
            )
            .padding([4, 10]),
        ]
        .height(Fill);
        let details: Element<'_, Message> = match w.section {
            Section::General => self.properties_general(node, kind),
            Section::Source => self.properties_source(node, kind),
            Section::Crs => self.properties_crs(node),
            Section::Style => self.properties_style(node),
            Section::Fields => self.properties_fields(node),
            Section::Forms => self.properties_forms(node),
        };
        // The section's one way on, beside its title (T19).
        let mut title = row![container(label::strong(w.section.title())).width(Fill)]
            .spacing(8)
            .align_y(Center);
        if let Some(a) = self.layer_properties_action(w.section, &w.layer) {
            title = title.push(
                container(
                    button(
                        row![
                            icon(from_web(Some(a.glyph()))).size(14.0),
                            label::caption(a.words())
                        ]
                        .spacing(6)
                        .align_y(Center),
                    )
                    .padding(kentos_ui::theme::metrics::action_small())
                    .style(style::button::secondary)
                    .on_press(msg(Event::Act(a))),
                )
                .padding(1)
                .style(ring(w.focus == Focus::Action)),
            );
        }
        let details = column![title, details].spacing(12).padding([10, 18]);
        let close = container(crate::exchange::words::secondary(
            "Kapat",
            Some(msg(Event::Close)),
        ))
        .padding(1)
        .style(ring(w.focus == Focus::Close));
        Frame::new(TITLE)
            .push(
                column![
                    head,
                    ListDetails::new(list, details)
                        .list_width(200.0)
                        .height(380.0)
                ]
                .spacing(16),
            )
            .status(
                label::caption(
                    "Bilgi içindir; her bölümün eylemi kendi penceresini bu katmanda açar.",
                )
                .style(style::text::muted),
            )
            .action(close)
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
        } else if matches!(
            kind,
            LayerKind::Drawing | LayerKind::Annotation | LayerKind::Bound
        ) {
            pairs = pairs.push(
                name("Nesne sayısı"),
                value(doc.model.by_layer(&node.id).count().to_string()),
            );
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
                .push(
                    name("Bağlantı"),
                    value(if d.driver == "gpkg" {
                        "GeoPackage dosyası"
                    } else {
                        "PostgreSQL / PostGIS veritabanı"
                    }),
                )
                .push(
                    name(if d.driver == "gpkg" {
                        "Dosya"
                    } else {
                        "Veritabanı"
                    }),
                    value(shown_address(&d.address)),
                )
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
                .push(
                    name("Yazma"),
                    value(if d.read_only {
                        "Salt okunur"
                    } else {
                        "Düzenleme oturumuyla yazılır"
                    }),
                );
            if let Some(rel) = &d.relative {
                pairs = pairs.push(name("Projeye göre yolu"), value(rel.clone()));
            }
            return pairs.into();
        }
        if let Some(s) = &node.service {
            pairs = pairs
                .push(
                    name("Bağlantı"),
                    value(format!("Harita servisi · {}", s.kind.label())),
                )
                .push(name("Adres"), value(shown_address(&s.url)));
            if !s.layers.is_empty() {
                pairs = pairs.push(name("Servis katmanları"), value(s.layers.join(", ")));
            }
            return pairs.into();
        }
        let link = self.document.as_ref().and_then(|d| {
            d.model
                .settings()
                .sources
                .iter()
                .find(|s| s.layer == node.id)
                .cloned()
        });
        if let Some(link) = link {
            return pairs
                .push(
                    name("Bağlantı"),
                    value("Bağlı tablo: nesneler projede, tabloya yazılır"),
                )
                .push(name("Kaynak"), value(shown_address(&link.path)))
                .push(name("Tablo"), value(link.table.clone()))
                .into();
        }
        pairs
            .push(
                name("Bağlantı"),
                value(match kind {
                    LayerKind::Annotation => {
                        "Proje dosyası: yazılar, ölçüler ve paftalar projeyle saklanır"
                    }
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
                let table = g
                    .srid
                    .map_or_else(|| "Tabloda yazılı değil".to_owned(), srid_title);
                let carried = match g.srid {
                    Some(srid) if srid == settings.srid => "Aynı sistem: satırlar olduğu gibi çizilir".to_owned(),
                    Some(_) => "Satırlar çizilirken projenin sistemine dönüştürülür; kaynaktaki koordinatlar değişmez".to_owned(),
                    None => "Tablonun sistemi bilinmiyor: satırlar projenin sisteminde sayılır".to_owned(),
                };
                pairs = pairs
                    .push(name("Tablonun sistemi"), value(table))
                    .push(name("Çizim"), value(carried));
            }
            None if node.data.is_some() => {
                pairs = pairs.push(name("Tablonun sistemi"), value("Geometrisiz tablo"));
            }
            None => {
                pairs = pairs.push(
                    name("Katmanın sistemi"),
                    value("Projenin sistemi (nesneler projeyle saklanır)"),
                );
            }
        }
        pairs.into()
    }

    fn properties_style<'a>(&self, node: &LayerNode) -> Element<'a, Message> {
        let s = &node.style;
        let kind = s
            .renderer
            .as_ref()
            .and_then(|r| r.get("type"))
            .and_then(|t| t.as_str());
        // A theme's colour by its name with the colour it is now, a colour of its own by its hex (T20): the swatch is the
        // drawing's own colour on this canvas.
        let palette = crate::viewport::palette(self.canvas());
        let swatch = |c: &str| {
            let resolved = palette.resolve(c);
            let color = resolved.map_or_else(|| crate::view::hex_color(c), crate::view::rgba_color);
            let hex = resolved.map_or_else(
                || c.to_uppercase(),
                |r| format!("#{:02X}{:02X}{:02X}", r.0[0], r.0[1], r.0[2]),
            );
            let words = match theme_color_words(c) {
                Some(name) => format!("{name} ({hex}, temaya göre değişir)"),
                None => hex,
            };
            row![
                container(space().width(14).height(14)).style(style::container::swatch(color)),
                label::body(words).wrapping(iced::widget::text::Wrapping::WordOrGlyph)
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
            .push(
                name("Etiketler"),
                value(if s.labels_on() { "Açık" } else { "Kapalı" }),
            )
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
                LayerFieldKind::Text => f
                    .length
                    .map_or_else(|| "Metin".to_owned(), |n| format!("Metin ({n})")),
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
                        container(label::caption(kind).style(style::text::muted))
                            .width(Length::FillPortion(2)),
                        container(label::caption(marks.join(" · ")).style(style::text::accent))
                            .width(Length::FillPortion(3)),
                    ]
                    .spacing(10)
                    .align_y(Center),
                )
                .padding([5, 8]),
            );
        }
        column![
            label::caption(format!("{} alan", node.fields.len())).style(style::text::muted),
            list
        ]
        .spacing(6)
        .into()
    }

    fn properties_forms<'a>(&self, node: &LayerNode) -> Element<'a, Message> {
        let forms: Vec<&kentos_commands::ProjectForm> = self
            .project_forms()
            .iter()
            .filter(|f| f.layer == node.id)
            .collect();
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
    kentos_project::crs::system(srid)
        .map_or_else(|| format!("EPSG:{srid}"), kentos_project::crs::title)
}

/// The project's system: its own definition's name, else by its SRID.
fn system_title(settings: &kentos_commands::ProjectSettings) -> String {
    match settings
        .custom_crs
        .as_ref()
        .filter(|_| settings.srid == kentos_project::crs::LOCAL_SRID)
    {
        Some(d) => kentos_project::systems::definition_title(d),
        None => srid_title(settings.srid),
    }
}

/// A pair's name, muted.
fn name<'a>(words: impl Into<String>) -> Element<'a, Message> {
    label::caption(words.into())
        .style(style::text::muted)
        .into()
}

/// A pair's value, wrapping.
fn value<'a>(words: impl Into<String>) -> Element<'a, Message> {
    label::body(words.into())
        .wrapping(iced::widget::text::Wrapping::WordOrGlyph)
        .into()
}

#[cfg(test)]
#[path = "layer_properties_tests.rs"]
mod layer_properties_tests;
