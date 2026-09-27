//! The catalog window as the web draws it (CatalogDialog.ts, catalogRows.ts,
//! catalogDetails.ts, catalog.css; docs/adr/0086): the lists on the left,
//! the chosen list in the middle with its search, type and order, the
//! selected project on the right with its facts and every action (an action
//! the account may not take stays visible, off, and its tip says which
//! right it needs), and the lifecycle's questions over it. The texts come
//! from the plan (plan.rs); here they are drawn.

use iced::widget::text::Wrapping;
use iced::widget::{
    Column, Row, Space, button, column, container, mouse_area, row, scrollable, stack, text,
    text_input,
};
use iced::{Background, Border, Center, Element, Fill, Length, Theme, mouse};
use kentos_contracts::{CatalogView, ProjectSummary};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Choice, Select};
use kentos_ui::widget::{
    Confirm, Dialog, Severity, Tip, horizontal_divider, overlay, progress, tip,
};
use kentos_ui::{label, style};

use crate::app::{App, Message};
use crate::cloud::catalog::{Catalog, List, empty_text, place_of};
use crate::cloud::catalog_actions::Act;
use crate::cloud::local_time::Zone;
use crate::cloud::plan::{self, PROJECT_TYPES};
use crate::cloud::view::{primary, secondary};
use crate::cloud::{Event, words};

pub(super) fn cloud(event: Event) -> Message {
    crate::cloud::msg(event)
}

/// The window (the web's: 1040 wide, 700 tall at most) and its columns.
const WIDTH: f32 = 1040.0;
const HEIGHT: f32 = 700.0;
const NAV: f32 = 196.0;
const PANE: f32 = 320.0;
const SELECT: f32 = 132.0;

/// Why an action the desktop does not have yet is off.
pub(super) const NOT_YET: &str = "Masaüstüne henüz taşınmadı; web uygulamasında yapılabilir.";

/// A web icon by name, drawn as the web draws it.
pub(super) fn web(name: &str) -> Icon {
    crate::icons::from_web(Some(name))
}

/// A small label (the web's `catalog-chip`): the type, a state, the open project.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) enum Chip {
    Kind,
    Open,
    Trashed,
    Plain,
}

pub(super) fn chip<'a>(text: impl Into<String>, kind: Chip) -> Element<'a, Message> {
    container(
        label::caption(text.into())
            .size(typography::caption() - 1.0)
            .wrapping(Wrapping::None)
            .style(move |theme: &Theme| {
                let t = Tokens::of(theme);
                iced::widget::text::Style {
                    color: Some(match kind {
                        Chip::Open => t.accent_hover,
                        Chip::Trashed => t.danger,
                        Chip::Kind | Chip::Plain => t.muted,
                    }),
                }
            }),
    )
    .padding([0, 7])
    .style(move |theme: &Theme| {
        let t = Tokens::of(theme);
        container::Style {
            background: (kind == Chip::Kind).then_some(Background::Color(t.header)),
            border: Border {
                color: match kind {
                    Chip::Open => t.accent,
                    Chip::Trashed => t.danger,
                    Chip::Kind | Chip::Plain => t.border,
                },
                width: 1.0,
                radius: 999.0.into(),
            },
            ..container::Style::default()
        }
    })
    .into()
}

/// A small button of the pane (the web's `btn--small`, `btn--danger`).
pub(super) fn small_button<'a>(
    caption: &'a str,
    glyph: Icon,
    on: Option<Message>,
    danger: bool,
) -> iced::widget::Button<'a, Message> {
    button(
        row![
            icon(glyph)
                .size(14.0)
                .tone(if danger { Tone::Danger } else { Tone::Muted }),
            label::caption(caption).wrapping(Wrapping::None),
        ]
        .spacing(6)
        .align_y(Center),
    )
    .on_press_maybe(on)
    .padding([4, 9])
    .style(move |theme: &Theme, status| {
        let mut s = style::button::secondary(theme, status);
        if danger && status != button::Status::Disabled {
            let t = Tokens::of(theme);
            s.text_color = t.danger;
            if matches!(status, button::Status::Hovered | button::Status::Pressed) {
                s.border.color = t.danger;
            }
        }
        s
    })
}

/// Text in the muted colour.
pub(super) fn muted<'a>(content: impl Into<String>) -> iced::widget::Text<'a> {
    label::caption(content.into()).style(style::text::muted)
}

impl App {
    /// “Bulut projeleri”: the lists, the chosen one, the selected project,
    /// at the web's size (1040 × 700 at most, 90 % of the window's height),
    /// kept inside a smaller window with a margin.
    pub(crate) fn catalog_view(&self) -> Element<'_, Message> {
        if self.cloud.catalog.is_none() {
            return text("").into();
        }
        let window = overlay::blocking(iced::widget::responsive(move |room| {
            let width = typography::from_default(WIDTH).min(room.width - 60.0);
            let height = typography::from_default(HEIGHT).min(room.height * 0.9);
            container(self.catalog_window(width, height))
                .center(Fill)
                .into()
        }));
        // A question or a form over the window (the web's stacked dialogs).
        match self
            .catalog_question_view()
            .or_else(|| self.history_overlay())
        {
            Some(over) => stack![window, over].into(),
            None => window,
        }
    }

    fn catalog_window(&self, width: f32, height: f32) -> Element<'_, Message> {
        let Some(c) = &self.cloud.catalog else {
            return text("").into();
        };
        let body = row![
            container(self.catalog_nav(c))
                .width(Length::Fixed(typography::from_default(NAV)))
                .height(Fill)
                .padding(iced::Padding {
                    top: 12.0,
                    right: 10.0,
                    bottom: 12.0,
                    left: 0.0,
                })
                .style(style::container::header),
            container(self.catalog_main(c))
                .width(Fill)
                .height(Fill)
                .padding([14, 16]),
            container(self.catalog_pane(c))
                .width(Length::Fixed(typography::from_default(PANE)))
                .height(Fill)
                .style(style::container::header),
        ]
        .height(Fill);
        let opening = self.cloud.opening.is_some();
        let cancel = if opening {
            cloud(Event::OpenCancel)
        } else {
            cloud(Event::Close)
        };
        let mut dialog = Dialog::new("Bulut projeleri").push(body);
        if c.list == List::Device {
            let can = c.picked_kept().is_some() && !opening && !c.busy();
            dialog = dialog.action(secondary(
                "Bu cihazdan kaldır",
                can.then(|| cloud(Event::CatalogRemove)),
            ));
        }
        let (caption, enabled, why) = self.catalog_primary(c);
        let main = primary(
            caption,
            (enabled && !opening && !c.busy()).then(|| cloud(Event::CatalogOpen)),
        );
        let main: Element<'_, Message> = if why.is_empty() {
            main
        } else {
            tip(main, Tip::new(why), iced::widget::tooltip::Position::Top)
        };
        dialog
            .action(secondary("Vazgeç", Some(cancel)))
            .action(main)
            .width(typography::unscaled(width))
            .max_height(typography::unscaled(height))
            .into()
    }

    /// The main button: Aç (archived: read-only), Geri yükle in the trash.
    fn catalog_primary(&self, c: &Catalog) -> (&'static str, bool, String) {
        match &c.list {
            List::Device => {
                let k = c.picked_kept();
                let read_only = k.is_some_and(|k| k.ended.is_some());
                (
                    if read_only { "Salt okunur aç" } else { "Aç" },
                    k.is_some(),
                    if k.is_none() {
                        "Önce listeden bir proje seçin.".to_owned()
                    } else {
                        String::new()
                    },
                )
            }
            List::View(v) => {
                let p = plan::primary_plan(*v, c.picked());
                (p.label, p.enabled, p.why)
            }
        }
    }

    /// The lists: the server's, then this device's copies.
    fn catalog_nav<'a>(&'a self, c: &'a Catalog) -> Element<'a, Message> {
        let online = self.cloud.me.is_some();
        let held = self.cloud.opening.is_some() || c.busy();
        // The web's `catalog-nav__item`: 34 px tall, 10 px in from both sides,
        // the icon and the name 10 px apart and centred on the row; the chosen
        // list in the pressed shade, its name strong, its icon in the accent,
        // and its bar on the column's edge, 8 px short of the row's ends.
        let height = typography::from_default(34.0);
        let item = |caption: &'static str, glyph: Icon, list: List, enabled: bool| {
            let selected = c.list == list;
            let bar = container(container(Space::new().width(3).height(Fill)).style(
                move |theme: &Theme| container::Style {
                    background: selected.then(|| Background::Color(Tokens::of(theme).accent)),
                    border: Border {
                        radius: iced::border::Radius::default().right(2.0),
                        ..Border::default()
                    },
                    ..container::Style::default()
                },
            ))
            .height(Length::Fixed(height))
            .padding([8, 0]);
            let face = container(
                row![
                    icon(glyph)
                        .size(16.0)
                        .tone(if selected { Tone::Accent } else { Tone::Muted }),
                    label::body(caption)
                        .wrapping(Wrapping::None)
                        .font(if selected {
                            typography::ui_strong()
                        } else {
                            typography::ui()
                        })
                        .style(move |theme: &Theme| {
                            let t = Tokens::of(theme);
                            iced::widget::text::Style {
                                color: Some(if selected { t.text } else { t.muted }),
                            }
                        }),
                ]
                .spacing(10)
                .align_y(Center),
            )
            .height(Fill)
            .center_y(Fill);
            let button = button(face)
                .on_press_maybe((enabled && !held).then(|| cloud(Event::CatalogView(list))))
                .width(Fill)
                .height(Length::Fixed(height))
                .padding([0, 10])
                .style(move |theme: &Theme, status| {
                    let t = Tokens::of(theme);
                    let background = match (selected, status) {
                        (true, _) => Some(t.layer(0.095)),
                        (false, button::Status::Hovered | button::Status::Pressed) => {
                            Some(t.layer(0.055))
                        }
                        _ => None,
                    };
                    button::Style {
                        background: background.map(Background::Color),
                        text_color: if status == button::Status::Disabled {
                            t.disabled()
                        } else {
                            t.text
                        },
                        border: Border {
                            radius: 6.0.into(),
                            ..Border::default()
                        },
                        ..button::Style::default()
                    }
                });
            row![bar, Space::new().width(7), button].align_y(Center)
        };
        let icons = [
            (CatalogView::Recent, "history"),
            (CatalogView::Favorites, "star"),
            (CatalogView::Mine, "folder"),
            (CatalogView::Organization, "layers"),
            (CatalogView::Shared, "share"),
            (CatalogView::Archived, "archive"),
            (CatalogView::Trash, "trash"),
        ];
        let mut nav = Column::new().spacing(2);
        for view in words::VIEWS {
            // The archived and the trash apart from the working lists.
            if view == CatalogView::Archived {
                nav = nav.push(Space::new().height(10));
            }
            let glyph = icons
                .iter()
                .find(|(v, _)| *v == view)
                .map_or(Icon::Button, |(_, n)| web(n));
            nav = nav.push(item(
                words::view(view).label,
                glyph,
                List::View(view),
                online,
            ));
        }
        nav = nav.push(Space::new().height(14)).push(item(
            "Bu cihazdaki projeler",
            web("save"),
            List::Device,
            true,
        ));
        if !online {
            nav = nav.push(Space::new().height(8)).push(
                column![
                    muted("Sunucudaki listeler için giriş yapın.").width(Fill),
                    button(label::body("Buluta giriş…"))
                        .on_press(Message::Run("cloud.signIn"))
                        .padding([5, 10])
                        .style(style::button::secondary),
                ]
                .spacing(6)
                .padding(iced::padding::left(20)),
            );
        }
        scrollable(nav)
            .direction(style::field::body_scrollbar())
            .height(Fill)
            .into()
    }

    /// The chosen list: its head, search, type and order, the rows, and
    /// under them more, an open's or a download's progress and the line.
    fn catalog_main<'a>(&'a self, c: &'a Catalog) -> Element<'a, Message> {
        let orgs = self.organizations();
        let held = self.cloud.opening.is_some() || c.busy();
        let (title, note) = match &c.list {
            List::View(CatalogView::Trash) => (
                words::view(CatalogView::Trash).label.to_owned(),
                plan::trash_note(c.retention),
            ),
            List::View(v) => (
                words::view(*v).label.to_owned(),
                words::view(*v).note.to_owned(),
            ),
            List::Device => (
                "Bu cihazdaki projeler".to_owned(),
                "Bu bilgisayarda kopyası tutulan projeler bağlantı olmadan da açılır; değişiklikler bağlantı gelince gönderilir.".to_owned(),
            ),
        };
        let count = match &c.list {
            List::Device => format!("{} proje", c.device.len()),
            _ if c.total > 0 => format!("{} proje", c.total),
            _ => String::new(),
        };
        let head = row![
            label::heading(title).font(typography::ui_strong()),
            Space::new().width(Fill),
            muted(count),
        ]
        .align_y(iced::Alignment::End);
        let search = container(
            row![
                icon(Icon::Search).size(14.0).tone(Tone::Muted),
                text_input("Ara: ad, açıklama, etiket", &c.search)
                    .on_input_maybe((!held).then_some(|t| cloud(Event::CatalogSearch(t))))
                    .size(typography::body())
                    .padding([4, 0])
                    .style(style::field::bare_input),
            ]
            .spacing(6)
            .align_y(Center),
        )
        .padding([0, 8])
        .width(Fill)
        .style(style::container::field_box);
        let mut tools = Row::new().spacing(8).align_y(Center).push(search);
        if let List::View(view) = c.list {
            // “Tüm türler”, then each type in the web's order.
            let kinds = std::iter::once(Choice::new("Tüm türler")).chain(
                PROJECT_TYPES
                    .iter()
                    .map(|t| Choice::new(plan::type_label(*t))),
            );
            let chosen = c
                .kind
                .and_then(|k| PROJECT_TYPES.iter().position(|t| *t == k))
                .map_or(0, |i| i + 1);
            let kind = Select::new(kinds, Some(chosen), |i| {
                cloud(Event::CatalogKind(
                    i.checked_sub(1).map(|i| PROJECT_TYPES[i]),
                ))
            })
            .searchable(false);
            let sorts = words::view(view).sorts;
            let sort = Select::new(
                sorts.iter().map(|s| Choice::new(plan::sort_label(*s))),
                sorts.iter().position(|s| *s == c.sort),
                move |i| cloud(Event::CatalogSort(sorts[i])),
            )
            .searchable(false);
            tools = tools
                .push(container(kind).width(Length::Fixed(typography::from_default(SELECT))))
                .push(container(sort).width(Length::Fixed(typography::from_default(SELECT))));
        }
        let mut main = column![head, tools].spacing(8).width(Fill);
        if c.list == List::View(CatalogView::Organization) && !orgs.is_empty() {
            let chosen = orgs.iter().position(|(id, _)| Some(id) == c.org.as_ref());
            let field: Element<'_, Message> = if orgs.len() < 2 {
                // One organisation: nothing to choose.
                container(
                    label::body(orgs[0].1.clone())
                        .wrapping(Wrapping::None)
                        .style(style::text::muted),
                )
                .padding([5, 8])
                .width(Fill)
                .style(style::container::field_box)
                .into()
            } else {
                let ids: Vec<String> = orgs.iter().map(|(id, _)| id.clone()).collect();
                Select::new(
                    orgs.iter().map(|(_, name)| Choice::new(name.clone())),
                    chosen,
                    move |i| cloud(Event::CatalogOrg(ids[i].clone())),
                )
                .into()
            };
            main = main.push(row![muted("Kurum"), field].spacing(8).align_y(Center));
        }
        if !note.is_empty() {
            main = main.push(label::caption(note).style(style::text::muted));
        }
        main = main.push(
            container(self.catalog_rows(c, orgs.len()))
                .width(Fill)
                .height(Fill)
                .style(style::container::bordered),
        );
        if c.list != List::Device && c.next.is_some() {
            let caption = if c.loading() {
                "Yükleniyor…".to_owned()
            } else {
                format!("Daha fazla göster ({} / {})", c.projects.len(), c.total)
            };
            main = main.push(
                container(
                    button(label::caption(caption))
                        .on_press_maybe((!c.loading() && !held).then(|| cloud(Event::CatalogMore)))
                        .padding([4, 12])
                        .style(style::button::secondary),
                )
                .width(Fill)
                .align_x(Center),
            );
        }
        if let Some(o) = &self.cloud.opening {
            main = main.push(
                column![
                    label::caption(format!("“{}” açılıyor", o.name)),
                    progress::bar(o.fraction),
                    muted(o.stage.clone()),
                ]
                .spacing(4),
            );
        } else if let Some((said, fraction)) = c.acting.as_ref().and_then(|a| a.progress.clone()) {
            main = main.push(column![progress::bar(Some(fraction)), muted(said)].spacing(4));
        } else if let Some(said) = &c.status {
            let error = said.error;
            main = main.push(
                label::caption(said.text.clone()).style(move |theme: &Theme| {
                    let t = Tokens::of(theme);
                    iced::widget::text::Style {
                        color: Some(if error { t.danger } else { t.muted }),
                    }
                }),
            );
        }
        main.into()
    }

    /// The rows, or what the list says without them.
    fn catalog_rows<'a>(&'a self, c: &'a Catalog, orgs: usize) -> Element<'a, Message> {
        if let Some(error) = &c.error {
            return container(
                column![
                    label::caption(error.clone()).style(style::text::danger),
                    secondary("Yeniden dene", Some(cloud(Event::CatalogRetry))),
                ]
                .spacing(8),
            )
            .padding(12)
            .into();
        }
        let rows: Vec<Element<'a, Message>> = match &c.list {
            List::Device => c.device.iter().map(|k| self.kept_row(k, c)).collect(),
            List::View(view) => c
                .projects
                .iter()
                .map(|p| self.catalog_row(p, *view, c))
                .collect(),
        };
        if rows.is_empty() {
            return container(label::caption(empty_text(c, orgs)).style(style::text::muted))
                .padding(12)
                .width(Fill)
                .into();
        }
        let mut list = Column::new();
        let last = rows.len() - 1;
        for (i, r) in rows.into_iter().enumerate() {
            list = list.push(r);
            if i < last {
                list = list.push(horizontal_divider());
            }
        }
        scrollable(list)
            .direction(style::field::body_scrollbar())
            .height(Fill)
            .into()
    }

    /// A row's frame: selected (the web's accent-soft with its bar), hovered,
    /// a click selects and a double click does the main action.
    fn row_frame<'a>(
        &'a self,
        body: impl Into<Element<'a, Message>>,
        id: &str,
        c: &Catalog,
    ) -> Element<'a, Message> {
        let selected = c.picked.as_deref() == Some(id);
        let hovered = c.hovered.as_deref() == Some(id);
        let bar = container(Space::new().width(2).height(Fill)).style(move |theme: &Theme| {
            container::Style {
                background: selected.then(|| Background::Color(Tokens::of(theme).accent)),
                ..container::Style::default()
            }
        });
        let framed = container(
            row![bar, container(body).padding([7, 12]).width(Fill)].height(Length::Shrink),
        )
        .width(Fill)
        .style(move |theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: if selected {
                    Some(Background::Color(t.selection()))
                } else if hovered {
                    Some(Background::Color(t.layer(0.05)))
                } else {
                    None
                },
                ..container::Style::default()
            }
        });
        let id = id.to_owned();
        mouse_area(framed)
            .on_press(cloud(Event::CatalogPick(id.clone())))
            .on_double_click(cloud(Event::CatalogOpen))
            .on_enter(cloud(Event::CatalogHover(Some(id))))
            .on_exit(cloud(Event::CatalogHover(None)))
            .interaction(mouse::Interaction::Pointer)
            .into()
    }

    /// One project of a server's list (the web's `catalogRow`).
    fn catalog_row<'a>(
        &'a self,
        p: &'a ProjectSummary,
        view: CatalogView,
        c: &Catalog,
    ) -> Element<'a, Message> {
        let open = self.is_open_project(&p.id);
        let place = place_of(self, p);
        let plan = plan::row_plan(p, view, c.sort, open, &place, Zone::system());
        let mut name = Row::new()
            .spacing(6)
            .align_y(Center)
            .push(label::body(p.name.as_str()).wrapping(Wrapping::None));
        for m in &plan.marks {
            name = name.push(match *m {
                "Favorilerinizde" => tip(
                    icon(web("starOn")).size(14.0).tone(Tone::Accent),
                    Tip::new("Favorilerinizde"),
                    iced::widget::tooltip::Position::Top,
                ),
                "Açık" => chip("Açık", Chip::Open),
                other => chip(other, Chip::Plain),
            });
        }
        let sub: Element<'_, Message> = if view == CatalogView::Trash {
            muted(plan.sub[0].clone()).wrapping(Wrapping::None).into()
        } else {
            row![
                chip(plan.sub[0].clone(), Chip::Kind),
                muted(plan.sub.get(1).cloned().unwrap_or_default()).wrapping(Wrapping::None),
            ]
            .spacing(8)
            .align_y(Center)
            .into()
        };
        let mut side = Column::new().spacing(3).align_x(iced::Alignment::End);
        let time = plan.side.last().cloned().unwrap_or_default();
        if view == CatalogView::Shared
            && let Some(role) = plan.side.first()
        {
            side = side.push(tip(
                label::caption(role.clone())
                    .wrapping(Wrapping::None)
                    .style(style::text::muted),
                Tip::new("Bu projedeki rolünüz"),
                iced::widget::tooltip::Position::Left,
            ));
        }
        if view == CatalogView::Trash && plan.side.len() > 1 {
            side = side.push(
                label::caption(plan.side[0].clone())
                    .wrapping(Wrapping::None)
                    .style(|theme: &Theme| iced::widget::text::Style {
                        color: Some(Tokens::of(theme).warning),
                    }),
            );
        }
        side = side.push(muted(time).wrapping(Wrapping::None));
        let body = row![column![name, sub].spacing(3).width(Fill).clip(true), side,]
            .spacing(12)
            .align_y(Center);
        self.row_frame(body, &p.id, c)
    }

    /// A project of this device's copies: its name, where it is, how it is
    /// kept, the account's role, and when the copy was last written.
    fn kept_row<'a>(&'a self, k: &'a kentos_cloud::Kept, c: &Catalog) -> Element<'a, Message> {
        let info = &k.info;
        let open = self.is_open_project(&info.id);
        let place = words::workspace(info.tenant_kind, &info.tenant_name, true);
        let mut name = row![label::body(info.name.as_str()).wrapping(Wrapping::None)]
            .spacing(6)
            .align_y(Center);
        if open {
            name = name.push(chip("Açık", Chip::Open));
        }
        if let Some(ended) = k.ended {
            name = name.push(chip(words::ended(ended), Chip::Plain));
        }
        let body = row![
            column![
                name,
                row![
                    chip(words::storage_badge(info.storage), Chip::Kind),
                    muted(place).wrapping(Wrapping::None),
                ]
                .spacing(8)
                .align_y(Center),
            ]
            .spacing(3)
            .width(Fill)
            .clip(true),
            column![
                label::caption(words::role(info.access.role))
                    .wrapping(Wrapping::None)
                    .style(style::text::muted),
                muted(format!("Bu cihazda: {}", words::ago_ms(k.saved_at)))
                    .wrapping(Wrapping::None),
            ]
            .spacing(3)
            .align_x(iced::Alignment::End),
        ]
        .spacing(12)
        .align_y(Center);
        self.row_frame(body, &info.id, c)
    }

    /// The lifecycle's question over the window (the web's `askRemove` and
    /// `confirmDialog`): the removing answer marked, the safe one focused.
    fn catalog_question_view(&self) -> Option<Element<'_, Message>> {
        let (act, q) = self.catalog_question()?;
        let confirm = Confirm::new(
            q.title,
            cloud(Event::CatalogAnswer(true)),
            cloud(Event::CatalogAnswer(false)),
        )
        .message(q.message)
        .detail(
            q.details
                .iter()
                .map(|d| format!("• {d}"))
                .collect::<Vec<_>>()
                .join("\n"),
        )
        .confirm(q.action);
        let confirm = if act == Act::Archive {
            confirm.severity(Severity::Info)
        } else {
            confirm.destructive()
        };
        Some(overlay::modal(confirm, cloud(Event::CatalogAnswer(false))))
    }
}
