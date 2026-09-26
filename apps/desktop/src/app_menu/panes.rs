//! What the application menu shows: the left column's rows with their
//! lines, and the right side's Bu çizim, İçe aktar and Dışa aktar (the
//! cloud's is in cloud.rs).

use iced::widget::{Column, Row, button, column, container, row, scrollable, space, text};
use iced::{Center, Element, Fill};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::widget::app_menu::{Action, AppMenu, Entry, Item};
use kentos_ui::widget::badge;
use kentos_ui::{label, style};

use super::{EXPORTS, Event, Focus, Format, IMPORTS, NAV, Nav, Pane, RECENT, TILES, Target, event};
use crate::app::{App, Message};
use crate::catalog::catalog;

impl App {
    /// The menu over the window, when it is open.
    pub(crate) fn app_menu_view(&self) -> Option<Element<'_, Message>> {
        let s = self.app_menu.as_ref()?;
        let mut menu = AppMenu::new(event(Event::Dismiss))
            .width(780.0)
            .header(self.menu_header());
        for (i, nav) in NAV.iter().enumerate() {
            let current = s.focus == Some(Focus::Nav(i));
            let entry = Entry::new(nav.icon, nav.label, self.nav_line(nav)).current(current);
            let entry = match nav.target {
                Target::Command(id) => {
                    let entry = entry
                        .on_hover(event(Event::Show(Pane::Overview)))
                        .on_press_maybe(self.menu_runs(id).then(|| event(Event::Run(id))));
                    match catalog().get(id).and_then(|c| c.shortcuts.first()) {
                        Some(chord) => entry.shortcut(*chord),
                        None => entry,
                    }
                }
                Target::Pane(pane) => entry
                    .on_hover(event(Event::Show(pane)))
                    .on_press(event(Event::Show(pane)))
                    .submenu(s.pane == pane),
            };
            menu = menu.entry(entry);
        }
        let focus = match s.focus {
            Some(Focus::Pane(i)) => Some(i),
            _ => None,
        };
        let detail = match s.pane {
            Pane::Overview => self.overview_pane(focus),
            Pane::Import => self.formats_pane(
                "İçe aktar",
                "Dosyadaki nesneler tek geri alma adımıyla eklenir.",
                &IMPORTS,
                focus,
            ),
            Pane::Export => self.formats_pane(
                "Dışa aktar",
                "Çizim kaydedilmiş sayılmaz; dışa aktarma çizime dokunmaz.",
                &EXPORTS,
                focus,
            ),
            Pane::Cloud => self.cloud_pane(s, focus),
        };
        let footer = [
            ("tools.options", "Uygulama ayarları", Icon::Properties),
            ("help.shortcuts", "Kısayollar", Icon::Help),
            ("help.about", "Hakkında", Icon::Info),
        ];
        let mut menu = menu
            .detail(scrollable(detail).height(Fill))
            .note("KentOS CAD");
        for (id, title, glyph) in footer {
            menu = menu.action(Action::secondary(glyph, title, event(Event::Run(id))));
        }
        Some(menu.into())
    }

    /// The mark, the product's name and the drawing's (• when unsaved).
    fn menu_header(&self) -> Element<'_, Message> {
        let doc = self
            .document
            .as_ref()
            .map_or("Açık çizim yok".to_owned(), |d| {
                format!("{}{}", d.name(), if d.dirty() { " •" } else { "" })
            });
        row![
            container(kentos_ui::widget::ribbon::logo_mark())
                .center(28)
                .style(style::container::accent),
            column![
                row![
                    text("KentOS").font(kentos_ui::theme::typography::ui_strong()),
                    label::muted(" CAD"),
                ],
                label::caption(doc),
            ]
            .spacing(1),
        ]
        .spacing(10)
        .align_y(Center)
        .into()
    }

    /// The line under a row of the left column: what it does, or where it stands.
    fn nav_line(&self, nav: &Nav) -> String {
        if let Target::Command(id) = nav.target
            && let Some(why) = self.menu_why(id)
        {
            return why.to_owned();
        }
        match nav.label {
            "Yeni" => "Mod, koordinat sistemi ve ölçek".to_owned(),
            "Aç" => ".kcad çizim dosyası".to_owned(),
            "Kaydet" => self.save_where(),
            "Farklı kaydet" => "Yeni bir .kcad dosyasına".to_owned(),
            "İçe aktar" => "DXF, NCN, NCZ, SHP, GeoJSON".to_owned(),
            "Dışa aktar" => "DXF, NCN, GeoJSON, PDF".to_owned(),
            "Bulut" => self.cloud_line(),
            "Yazdır ve pafta" => "Pafta düzeni ve çıktı".to_owned(),
            _ => "Birimler, sistem, mod, yazı tipi".to_owned(),
        }
    }

    /// Where Kaydet writes (the web's `saveWhere`).
    pub(super) fn save_where(&self) -> String {
        let Some(doc) = &self.document else {
            return "Açık çizim yok".to_owned();
        };
        match (doc.cloud_source(), &doc.path) {
            (Some(_), _) if doc.is_database() => {
                "Bulut projesi: kendiliğinden kaydediliyor".to_owned()
            }
            (Some(source), _) => format!(
                "Bulut dosya projesi · {}",
                source
                    .revision
                    .as_ref()
                    .map_or("henüz revizyonu yok".to_owned(), |r| format!(
                        "revizyon {}",
                        r.number
                    ))
            ),
            (None, Some(path)) => path.file_name().map_or_else(
                || path.display().to_string(),
                |n| n.to_string_lossy().into_owned(),
            ),
            (None, None) => "İlk kayıtta yer sorulur".to_owned(),
        }
    }

    /// The cloud row's line (the web's `cloudLine`).
    pub(super) fn cloud_line(&self) -> String {
        let Some(me) = &self.cloud.me else {
            return "Giriş yapın, projelerinizi açın ve paylaşın".to_owned();
        };
        if self.cloud.link == crate::cloud::copy::Link::Offline {
            return "Sunucuya ulaşılamıyor".to_owned();
        }
        match self
            .document
            .as_ref()
            .and_then(|d| d.cloud_source().map(|s| (d, s)))
        {
            Some((doc, source)) => format!("{} › {}", source.workspace, doc.name()),
            None => format!("{} olarak giriş yapıldı", me.user.display_name),
        }
    }

    /// Bu çizim: the drawing's card, the quick starts and the recent files.
    fn overview_pane(&self, focus: Option<usize>) -> Element<'_, Message> {
        let mut pane = Column::new().spacing(12).push(head("Bu çizim", None));
        pane = pane.push(self.drawing_card());
        let tiles =
            TILES
                .iter()
                .enumerate()
                .fold(Row::new().spacing(8), |r, (i, (id, glyph, title))| {
                    let face = column![
                        icon(*glyph).size(22.0).tone(Tone::Accent),
                        label::body(*title)
                    ]
                    .spacing(6)
                    .align_x(Center);
                    r.push(
                        button(container(face).center_x(Fill))
                            .on_press_maybe(self.menu_runs(id).then(|| event(Event::Run(id))))
                            .width(Fill)
                            .padding([10, 6])
                            .style(style::button::list_item(focus == Some(i))),
                    )
                });
        pane = pane.push(tiles);
        let recent = self.recent.list();
        if !recent.is_empty() {
            let mut list = Column::new().spacing(1);
            for (k, r) in recent.iter().take(RECENT).enumerate() {
                list = list.push(crate::start::recent_row(
                    r,
                    event(Event::OpenRecent(r.path.clone())),
                    event(Event::Forget(r.path.clone())),
                    focus == Some(TILES.len() + k),
                ));
            }
            pane = pane.push(section("Son dosyalar")).push(list);
        }
        pane.into()
    }

    /// The drawing on screen: its name and place, its chips, its counts and its save state.
    fn drawing_card(&self) -> Element<'_, Message> {
        let Some(doc) = &self.document else {
            return card(
                column![
                    label::heading("Açık çizim yok"),
                    label::caption(
                        "Yeni proje açın, bir çizim dosyası seçin ya da bulut projelerinize bakın."
                    ),
                ]
                .spacing(4),
            );
        };
        let s = doc.settings();
        let place = match (doc.cloud_source(), &doc.path) {
            (Some(source), _) => format!(
                "Bulut · {}{}",
                source.workspace,
                match (&source.revision, doc.is_database()) {
                    (_, true) => String::new(),
                    (Some(r), false) => format!(" · dosya projesi, revizyon {}", r.number),
                    (None, false) => " · dosya projesi, henüz revizyonu yok".to_owned(),
                }
            ),
            (None, Some(path)) => format!(
                "Dosya · {}",
                path.file_name().map_or_else(
                    || path.display().to_string(),
                    |n| n.to_string_lossy().into_owned()
                )
            ),
            (None, None) => "Bir dosyaya bağlı değil; ilk kayıtta yer sorulur".to_owned(),
        };
        let system =
            crate::crs::system(s.srid).map_or(format!("EPSG:{}", s.srid), |c| c.name.clone());
        let mode = crate::catalog::mode_of(s.workspace);
        let font = crate::project::font_label(s.drawing_font);
        let chips = row![
            chip(Icon::Globe, system),
            chip(Icon::Layers, mode.to_owned()),
            chip(Icon::Ruler, crate::project::scale_label(s.plot_scale)),
            chip(Icon::Type, font.to_owned()),
        ]
        .spacing(6)
        .wrap();
        let counts = row![
            label::strong(crate::crs::grouped(doc.entity_count() as f64)),
            label::muted("nesne"),
            space::horizontal().width(12),
            label::strong(doc.layer_count().to_string()),
            label::muted("katman"),
        ]
        .spacing(4)
        .align_y(Center);
        let saved: Element<'_, Message> = if doc.dirty() && !doc.is_database() {
            row![
                text("●").style(style::text::accent),
                label::body("Kaydedilmemiş değişiklikler var").width(Fill),
                button(label::body("Kaydet"))
                    .on_press_maybe(
                        self.menu_runs("file.save")
                            .then(|| event(Event::Run("file.save")))
                    )
                    .padding([3, 12])
                    .style(style::button::primary),
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        } else {
            let words = if doc.is_database() {
                "Değişiklikler buluta kendiliğinden kaydediliyor"
            } else if doc.path.is_some() || doc.cloud_source().is_some() {
                "Tüm değişiklikler kaydedildi"
            } else {
                "Kaydedilmemiş değişiklik yok"
            };
            row![
                icon(Icon::Check).size(14.0).tone(Tone::Success),
                label::caption(words)
            ]
            .spacing(6)
            .align_y(Center)
            .into()
        };
        card(
            column![
                label::heading(doc.name().to_owned()),
                label::caption(place),
                chips,
                counts,
                saved,
            ]
            .spacing(8),
        )
    }

    /// İçe aktar or Dışa aktar: the formats, each with its badge and what it takes or gives.
    fn formats_pane(
        &self,
        title: &'static str,
        lead: &'static str,
        formats: &'static [Format],
        focus: Option<usize>,
    ) -> Element<'_, Message> {
        let rows = formats
            .iter()
            .enumerate()
            .fold(Column::new().spacing(1), |list, (i, f)| {
                let runs = self.menu_runs(f.id);
                let trailing: Element<'_, Message> = match (runs, catalog().get(f.id)) {
                    (true, Some(c)) => {
                        label::mono_caption(c.shortcuts.first().copied().unwrap_or("")).into()
                    }
                    _ => soon("Yakında"),
                };
                list.push(
                    Item::new(f.label)
                        .subtitle(self.menu_why(f.id).unwrap_or(f.detail))
                        .leading(badge(f.badge))
                        .trailing(trailing)
                        .on_press_maybe(runs.then(|| event(Event::Run(f.id))))
                        .current(focus == Some(i)),
                )
            });
        column![head(title, Some(lead)), rows].spacing(12).into()
    }
}

/// A pane's title, with a line under it.
pub(super) fn head<'a>(title: &'a str, lead: Option<&'a str>) -> Element<'a, Message> {
    let mut c = column![label::heading(title)].spacing(2);
    if let Some(lead) = lead {
        c = c.push(label::caption(lead));
    }
    c.into()
}

/// A small heading between parts of a pane.
pub(super) fn section(title: &str) -> Element<'_, Message> {
    label::caption(title).style(style::text::muted).into()
}

/// A framed block of the right side.
pub(super) fn card<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding([10, 12])
        .width(Fill)
        .style(style::container::bordered)
        .into()
}

/// A setting of the drawing, as a chip.
pub(super) fn chip<'a>(glyph: Icon, words: String) -> Element<'a, Message> {
    container(
        row![
            icon(glyph).size(13.0).tone(Tone::Muted),
            label::caption(words)
        ]
        .spacing(5)
        .align_y(Center),
    )
    .padding([2, 8])
    .style(style::container::badge)
    .into()
}

/// The pill of a row that does not run yet, or of the open project.
pub(super) fn soon<'a>(words: &'a str) -> Element<'a, Message> {
    container(label::caption(words))
        .padding([1, 8])
        .style(style::container::badge)
        .into()
}
