//! The catalog's selected project (docs/adr/0086; the web's
//! catalogDetails.ts): its name and favourite, chips, the Bilgiler and
//! Geçmiş tabs, what describes it and its facts as the web writes them, and
//! its actions below, which stay in view. A copy of this device, selected in
//! “Bu cihazdaki projeler”, says what the copy is.

use iced::widget::text::Wrapping;
use iced::widget::{Column, Row, Space, button, column, container, row, scrollable};
use iced::{Background, Border, Center, Element, Fill, Length, Theme};
use kentos_contracts::{AccessSource, AreaUnit, ProjectState, ProjectSummary};
use kentos_interaction::Format;
use kentos_ui::icon::{Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Tip, horizontal_divider, tip};
use kentos_ui::{label, style};

use super::catalog_view::{Chip, NOT_YET, chip, cloud, muted, small_button, web};
use crate::app::{App, Message};
use crate::cloud::catalog::{Catalog, Details, List, Tab, place_of};
use crate::cloud::catalog_actions::Act;
use crate::cloud::local_time::{Zone, day, when};
use crate::cloud::plan::{self, DetailAction};
use crate::cloud::{Event, catalog_forms, share, words};

impl App {
    /// The selected project (the web's `renderDetails`): its name and
    /// favourite, chips, tabs, what describes it and its facts; its actions
    /// stay in view below.
    pub(super) fn catalog_pane<'a>(&'a self, c: &'a Catalog) -> Element<'a, Message> {
        if c.list == List::Device {
            return self.kept_pane(c);
        }
        let Some(p) = c.picked() else {
            return container(
                label::caption(
                    "Bilgilerini görmek ve üzerinde işlem yapmak için listeden bir proje seçin.",
                )
                .style(style::text::muted),
            )
            .padding([14, 16])
            .into();
        };
        let open = self.is_open_project(&p.id);
        let plan = plan::detail_plan(p, open);
        let held = self.cloud.opening.is_some() || c.busy();
        let mut body = Column::new().spacing(10).padding([14, 16]);
        let mut head = column![
            label::heading(p.name.as_str())
                .font(typography::ui_strong())
                .wrapping(Wrapping::WordOrGlyph)
        ]
        .spacing(4);
        if let Some(favorite) = plan.favorite {
            let on = p.favorite;
            head = head.push(tip(
                button(
                    row![
                        icon(web(if on { "starOn" } else { "star" }))
                            .size(14.0)
                            .tone(if on { Tone::Accent } else { Tone::Muted }),
                        label::caption(favorite),
                    ]
                    .spacing(6)
                    .align_y(Center),
                )
                .on_press_maybe((!held).then(|| cloud(Event::CatalogAct(Act::Favorite))))
                .padding([3, 6])
                .style(style::button::ghost),
                Tip::new("Favorileriniz yalnız size görünür."),
                iced::widget::tooltip::Position::Bottom,
            ));
        }
        body = body.push(head);
        let chips = plan
            .chips
            .iter()
            .enumerate()
            .fold(Row::new().spacing(6), |r, (i, t)| {
                let kind = if i == 0 {
                    Chip::Kind
                } else if *t == "Şu anda açık" {
                    Chip::Open
                } else if p.state == ProjectState::Trashed {
                    Chip::Trashed
                } else {
                    Chip::Plain
                };
                r.push(chip(*t, kind))
            });
        body = body.push(chips.wrap());
        if let Some(tabs) = plan.tabs {
            let tab = |caption: &'static str, which: Tab| {
                let on = c.tab == which;
                column![
                    button(label::caption(caption).font(typography::ui_strong()))
                        .on_press(cloud(Event::CatalogTab(which)))
                        .padding([5, 10])
                        .style(move |theme: &Theme, status| {
                            let t = Tokens::of(theme);
                            button::Style {
                                background: None,
                                text_color: if on || matches!(status, button::Status::Hovered) {
                                    t.text
                                } else {
                                    t.muted
                                },
                                ..button::Style::default()
                            }
                        }),
                    container(Space::new().height(2))
                        .width(Fill)
                        .style(move |theme: &Theme| container::Style {
                            background: on.then(|| Background::Color(Tokens::of(theme).accent)),
                            ..container::Style::default()
                        }),
                ]
                .width(Length::Shrink)
            };
            body = body.push(
                column![
                    row![tab(tabs[0], Tab::Info), tab(tabs[1], Tab::History)].spacing(2),
                    horizontal_divider(),
                ]
                .spacing(0),
            );
        }
        if plan.tabs.is_some() && c.tab == Tab::History {
            for part in self.history_tab(p) {
                body = body.push(part);
            }
        } else {
            body = body.push(if p.description.is_empty() {
                muted("Açıklama yok.").width(Fill)
            } else {
                label::caption(p.description.as_str()).width(Fill)
            });
            if !p.tags.is_empty() {
                let tags = p.tags.iter().fold(Row::new().spacing(6), |r, t| {
                    r.push(
                        container(label::caption(t.as_str()).style(style::text::muted))
                            .padding([1, 8])
                            .style(|theme: &Theme| container::Style {
                                background: Some(Background::Color(Tokens::of(theme).selection())),
                                border: Border {
                                    radius: 2.0.into(),
                                    ..Border::default()
                                },
                                ..container::Style::default()
                            }),
                    )
                });
                body = body.push(tags.wrap());
            }
            body = body.push(horizontal_divider());
            // The terms' column is as wide as its widest term (the web's `auto` column).
            let terms = plan
                .facts
                .iter()
                .map(|t| typography::text_width(t, typography::caption()))
                .fold(0.0, f32::max)
                .ceil()
                + 2.0;
            let mut facts = Column::new().spacing(5);
            for term in &plan.facts {
                facts = facts.push(
                    row![
                        container(muted(*term)).width(Length::Fixed(terms)),
                        container(self.fact(term, p, &c.details)).width(Fill),
                    ]
                    .spacing(12),
                );
            }
            body = body.push(facts);
        }
        let actions = plan.actions.iter().fold(Row::new().spacing(6), |r, a| {
            // What the desktop cannot do yet says so; what the account may not do, which right it needs.
            let act = match a.id {
                DetailAction::Download => Some(cloud(Event::CatalogAct(Act::Download))),
                DetailAction::Archive => Some(cloud(Event::CatalogAct(Act::Archive))),
                DetailAction::Unarchive => Some(cloud(Event::CatalogAct(Act::Unarchive))),
                DetailAction::Trash => Some(cloud(Event::CatalogAct(Act::Trash))),
                DetailAction::Purge => Some(cloud(Event::CatalogAct(Act::Purge))),
                DetailAction::Share => Some(share::msg(share::Event::Open)),
                DetailAction::Edit => Some(catalog_forms::msg(catalog_forms::Event::Edit)),
                DetailAction::Duplicate => {
                    Some(catalog_forms::msg(catalog_forms::Event::Duplicate))
                }
                DetailAction::Convert => Some(catalog_forms::msg(catalog_forms::Event::Convert)),
            };
            let why = a
                .why
                .clone()
                .or_else(|| act.is_none().then(|| NOT_YET.to_owned()));
            let on = act.filter(|_| why.is_none() && !held);
            let b = small_button(a.label, web(a.icon), on, a.danger);
            r.push(match why {
                Some(why) => tip(b, Tip::new(why), iced::widget::tooltip::Position::Top),
                None => b.into(),
            })
        });
        column![
            scrollable(body)
                .direction(style::field::body_scrollbar())
                .height(Fill),
            horizontal_divider(),
            container(actions.wrap().vertical_spacing(6)).padding([10, 16]),
        ]
        .height(Fill)
        .into()
    }

    /// A fact's value (the web's `value(term)`).
    fn fact<'a>(&self, term: &str, p: &ProjectSummary, d: &Details) -> Element<'a, Message> {
        let zone = Zone::system();
        let plain = |t: String| -> Element<'a, Message> { label::caption(t).into() };
        // What the server counts: while asked, when it failed, or not asked (the trash).
        let waiting = |d: &Details| -> Option<Element<'a, Message>> {
            match d {
                Details::Loading => Some(muted("Hesaplanıyor…").into()),
                Details::Failed(_) => Some(muted("Okunamadı").into()),
                Details::None => Some(plain("—".to_owned())),
                Details::Database(_) | Details::File { .. } => None,
            }
        };
        match term {
            "Çalışma alanı" => plain(place_of(self, p)),
            "Sahibi" => {
                if p.owner_name.is_empty() {
                    muted("görünmüyor").into()
                } else {
                    plain(p.owner_name.clone())
                }
            }
            "Rolünüz" => plain(format!(
                "{}{}",
                words::role(p.access.role),
                if p.access.via == AccessSource::Policy {
                    " (kurum politikası)"
                } else {
                    ""
                }
            )),
            "Koordinat sistemi" => plain(match crate::document::crs_name(p.srid) {
                Some(name) => format!("{name} (EPSG:{})", p.srid),
                None => format!("EPSG:{}", p.srid),
            }),
            "Alan birimi" => plain(
                match p.area_unit {
                    AreaUnit::M2 => "m²",
                    AreaUnit::Donum => "dönüm",
                    AreaUnit::Ha => "hektar",
                }
                .to_owned(),
            ),
            "Nesne" => waiting(d).unwrap_or_else(|| match d {
                Details::Database(x) => plain(format!(
                    "{} nesne, {} katman",
                    words::grouped(&x.feature_count),
                    x.layer_count
                )),
                Details::File {
                    revision: Some(r),
                    objects,
                    ..
                } => plain(match objects {
                    Some(n) => format!("{} nesne (revizyon {r})", words::grouped(n)),
                    None => format!("Nesne sayısı bilinmiyor (revizyon {r})"),
                }),
                _ => muted("Henüz kaydedilmiş revizyon yok").into(),
            }),
            "Kapsam" => waiting(d).unwrap_or_else(|| match d {
                Details::Database(x) => match &x.bounds {
                    None => muted("Geometrili nesne yok").into(),
                    Some(b) => {
                        let f = self
                            .document
                            .as_ref()
                            .map_or_else(Format::default, |doc| Format::of(doc.settings()));
                        column![
                            label::caption(format!("Y {}–{}", f.coord(b.min_x), f.coord(b.max_x)))
                                .wrapping(Wrapping::None),
                            label::caption(format!("X {}–{}", f.coord(b.min_y), f.coord(b.max_y)))
                                .wrapping(Wrapping::None),
                        ]
                        .into()
                    }
                },
                _ => muted("Dosya projesinde hesaplanmaz").into(),
            }),
            "Oluşturan" => plain(format!(
                "{}, {}",
                if p.creator_name.is_empty() {
                    "görünmüyor"
                } else {
                    p.creator_name.as_str()
                },
                when(&p.created_at, zone)
            )),
            "Son değişiklik" => plain(when(&p.updated_at, zone)),
            "Revizyon" => plain(p.data_revision.clone()),
            "Saklama" => plain(words::storage_title(p.storage).to_owned()),
            "Arşivlenme" => plain(
                p.archived_at
                    .as_deref()
                    .map_or_else(String::new, |t| when(t, zone)),
            ),
            "Çöpe taşınma" => plain(p.trashed_at.as_deref().map_or_else(String::new, |t| {
                match p.trashed_by_name.as_deref().filter(|n| !n.is_empty()) {
                    Some(n) => format!("{}, {n}", when(t, zone)),
                    None => when(t, zone),
                }
            })),
            _ => plain(
                p.purge_after
                    .as_deref()
                    .and_then(|t| day(t, zone))
                    .unwrap_or_else(|| "Elle silinene kadar kalır".to_owned()),
            ),
        }
    }

    /// A copy of this device, selected: what the copy is.
    fn kept_pane<'a>(&'a self, c: &'a Catalog) -> Element<'a, Message> {
        let Some(k) = c.picked_kept() else {
            return container(
                label::caption(
                    "Bilgilerini görmek ve açmak için listeden bu cihazdaki bir projeyi seçin.",
                )
                .style(style::text::muted),
            )
            .padding([14, 16])
            .into();
        };
        let info = &k.info;
        let terms = ["Çalışma alanı", "Rolünüz", "Saklama", "Bu cihazda", "Durum"]
            .iter()
            .map(|t| typography::text_width(t, typography::caption()))
            .fold(0.0, f32::max)
            .ceil()
            + 2.0;
        let row_of = |term: &'static str, value: String| {
            row![
                container(muted(term)).width(Length::Fixed(terms)),
                label::caption(value).width(Fill),
            ]
            .spacing(12)
        };
        let mut facts = column![
            row_of(
                "Çalışma alanı",
                words::workspace(info.tenant_kind, &info.tenant_name, true)
            ),
            row_of("Rolünüz", words::role(info.access.role).to_owned()),
            row_of("Saklama", words::storage_title(info.storage).to_owned()),
            row_of("Bu cihazda", words::ago_ms(k.saved_at)),
        ]
        .spacing(5);
        if let Some(ended) = k.ended {
            facts = facts.push(row_of("Durum", words::ended(ended).to_owned()));
        }
        let mut chips = Row::new()
            .spacing(6)
            .push(chip(words::storage_badge(info.storage), Chip::Kind));
        if self.is_open_project(&info.id) {
            chips = chips.push(chip("Şu anda açık", Chip::Open));
        }
        column![
            label::heading(info.name.as_str())
                .font(typography::ui_strong())
                .wrapping(Wrapping::WordOrGlyph),
            chips.wrap(),
            muted("Bu cihazdaki kopya bağlantı olmadan açılır; değişiklikler bağlantı gelince gönderilir.")
                .width(Fill),
            horizontal_divider(),
            facts,
        ]
        .spacing(10)
        .padding([14, 16])
        .into()
    }
}
