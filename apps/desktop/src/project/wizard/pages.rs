//! The Yeni proje wizard's pages (the web's `ui/settings/wizardSteps.ts`,
//! DESIGN.md §7.10.1, docs/adr/0626 §1): the project's type on cards with their
//! pictures; what a CAD drawing is for (a building, a plan on real coordinates,
//! a detail), and a plan's or a CBS project's province and system, the zone
//! suggested and shown on a strip of Türkiye's longitudes; its name and a summary.

use iced::widget::{
    Column, Id, Row, button, column, container, mouse_area, row, rule, scrollable, space, text,
    text_input,
};
use iced::{Background, Border, Center, Element, Fill, FillPortion, Padding, Theme};
use kentos_interaction::fixed;
use kentos_project::crs::datum_label;
use kentos_project::provinces;
use kentos_project::wizard::{Coords, Kind, WORKS};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::theme::{Tokens, metrics, shape, typography};
use kentos_ui::widget::Banner;
use kentos_ui::{label, style};

use super::{Event, Note, State, art, event, strip};
use crate::app::Message;
use crate::catalog::{Mode, catalog};

/// A page's title and what it is about.
fn head<'a>(title: &'a str, lead: &'a str) -> Element<'a, Message> {
    column![
        text(title)
            .font(typography::ui_strong())
            .size(typography::from_default(19.0)),
        label::muted(lead),
    ]
    .spacing(4)
    .into()
}

/// A titled part of a page.
fn section<'a>(
    name: impl Into<String>,
    content: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    column![
        text(name.into())
            .font(typography::ui_strong())
            .size(typography::caption())
            .style(style::text::muted),
        content.into()
    ]
    .spacing(8)
    .into()
}

/// A page: its head, then its parts 16 pixels apart.
fn page<'a>(head: Element<'a, Message>, parts: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    Column::new()
        .push(head)
        .push(Column::with_children(parts).spacing(16))
        .spacing(14)
        .width(Fill)
        .into()
}

/// A card's look (the web's `.wspick__card`, `.wiz__choice`): the field's
/// ground and a strong edge; the chosen one in the accent.
fn card(on: bool) -> impl Fn(&Theme, button::Status) -> button::Style {
    move |theme, status| {
        let t = Tokens::of(theme);
        let hovered = matches!(status, button::Status::Hovered | button::Status::Pressed);
        let (ground, edge, width) = match (on, hovered) {
            (true, _) => (t.selection(), t.accent, 2.0),
            (false, true) => (t.surface_hover, t.faint, 1.0),
            (false, false) => (t.field, t.border_strong(), 1.0),
        };
        button::Style {
            background: Some(Background::Color(ground)),
            text_color: t.text,
            border: Border {
                color: edge,
                width,
                radius: shape::md().into(),
            },
            ..button::Style::default()
        }
    }
}

/// The round mark at a card's corner: a check in the accent when chosen.
fn check<'a>(on: bool) -> Element<'a, Message> {
    let size = 18.0;
    let mark: Element<'a, Message> = if on {
        icon(Icon::Check).size(11.0).into()
    } else {
        space::horizontal().width(0).into()
    };
    container(mark)
        .center_x(size)
        .center_y(size)
        .style(move |theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                text_color: Some(t.on_accent),
                background: on.then_some(Background::Color(t.accent)),
                border: Border {
                    color: if on { t.accent } else { t.border_strong() },
                    width: 1.0,
                    radius: (size / 2.0).into(),
                },
                ..container::Style::default()
            }
        })
        .into()
}

/// A mark's box (the web's `.wiz__mark`, `.wspick__art`): the panel head's
/// ground, the accent's when chosen.
fn mark_box(on: bool) -> impl Fn(&Theme) -> container::Style {
    move |theme| {
        let t = Tokens::of(theme);
        container::Style {
            text_color: Some(if on { t.on_accent } else { t.muted }),
            background: Some(Background::Color(if on { t.accent } else { t.header })),
            border: Border {
                radius: shape::md().into(),
                ..Border::default()
            },
            ..container::Style::default()
        }
    }
}

/// The accent as text (the web's `--c-accent-text`).
fn accent_text(theme: &Theme) -> text::Style {
    text::Style {
        color: Some(Tokens::of(theme).accent_hover),
    }
}

// ── 1. Proje türü ──────────────────────────────────────────────────────

pub(super) fn kind<'a>(s: &'a State, theme: &Theme) -> Element<'a, Message> {
    let modes = catalog().modes();
    let chosen = s.draft.kind.workspace();
    // A module's program makes its own type only (docs/adr/0801): its card, and where the others are made.
    if let Some(program) = s.limit {
        let own: Vec<Element<'a, Message>> = modes
            .iter()
            .filter(|m| m.ready && s.allows_type(m.id))
            .map(|m| type_card(m, m.id == chosen, theme))
            .collect();
        let elsewhere = match program {
            kentos_launch::Module::Gis => {
                "CAD projeleri KentOS Mimari'de (bina, detay) ve KentOS Harita'da (harita, imar planı) yapılır."
            }
            _ => "CBS projeleri KentOS GIS'te yapılır.",
        };
        return page(
            head(
                "Ne tür bir proje?",
                "Tür; sahneyi, eksenleri ve şeridi belirler. Proje başladıktan sonra değişmez.",
            ),
            vec![
                // As wide as one of the two cards kentos-cad shows.
                Row::with_children(own)
                    .push(space::horizontal().width(Fill))
                    .spacing(10)
                    .into(),
                label::muted(format!("{} {elsewhere}", program_makes(program))).into(),
            ],
        );
    }
    let ready: Vec<Element<'a, Message>> = modes
        .iter()
        .filter(|m| m.ready)
        .map(|m| type_card(m, m.id == chosen, theme))
        .collect();
    let soon: Vec<Element<'a, Message>> =
        modes.iter().filter(|m| !m.ready).map(soon_card).collect();
    let mut parts = vec![Row::with_children(ready).spacing(10).into()];
    if !soon.is_empty() {
        parts.push(
            column![
                row![
                    text("Yakında")
                        .font(typography::ui_strong())
                        .size(typography::caption() - 1.0)
                        .style(style::text::faint),
                    rule::horizontal(1).style(style::field::hairline),
                ]
                .spacing(10)
                .align_y(Center),
                Row::with_children(soon).spacing(10),
            ]
            .spacing(8)
            .into(),
        );
    }
    page(
        head(
            "Ne tür bir proje?",
            "Tür; sahneyi, eksenleri ve şeridi belirler. Proje başladıktan sonra değişmez.",
        ),
        parts,
    )
}

/// What a module's program makes, in a sentence.
fn program_makes(program: kentos_launch::Module) -> String {
    match program {
        kentos_launch::Module::Gis => "KentOS GIS, CBS projeleri yapar.".to_owned(),
        m => {
            let works: Vec<&str> = crate::program::works_of(m)
                .into_iter()
                .map(kentos_commands::CadWork::name)
                .collect();
            format!("{}, {} çizimleri yapar.", m.title(), works.join(" ve "))
        }
    }
}

/// A type that can be chosen: its picture across the card, its name, what
/// it is for and three points; a double click goes on.
fn type_card<'a>(m: &'static Mode, on: bool, theme: &Theme) -> Element<'a, Message> {
    let picture = container(art::picture(m.id, on, theme))
        .width(Fill)
        .height(typography::from_default(118.0))
        .padding([6, 10])
        .style(move |theme: &Theme| {
            let t = Tokens::of(theme);
            container::Style {
                background: Some(Background::Color(if on { t.field } else { t.header })),
                border: Border {
                    radius: shape::md().into(),
                    ..Border::default()
                },
                ..container::Style::default()
            }
        });
    let points = Column::with_children(m.highlights.iter().map(|p| {
        row![
            container(space::horizontal().width(0))
                .width(4)
                .height(4)
                .style(move |theme: &Theme| {
                    let t = Tokens::of(theme);
                    container::Style {
                        background: Some(Background::Color(if on { t.accent } else { t.faint })),
                        border: Border {
                            radius: 1.0.into(),
                            ..Border::default()
                        },
                        ..container::Style::default()
                    }
                }),
            text(*p)
                .size(typography::caption() - 1.0)
                .style(style::text::muted),
        ]
        .spacing(7)
        .align_y(Center)
        .into()
    }))
    .spacing(2);
    let body = column![
        picture,
        row![
            text(m.label)
                .font(typography::ui_strong())
                .size(typography::heading()),
            space::horizontal(),
            check(on),
        ]
        .align_y(Center),
        text(m.title)
            .font(typography::ui_strong())
            .size(typography::caption())
            .style(accent_text),
        text(m.description)
            .size(typography::caption())
            .style(style::text::muted),
        container(points).padding(Padding {
            top: 4.0,
            ..Padding::ZERO
        }),
    ]
    .spacing(4);
    mouse_area(
        button(body)
            .on_press(event(Event::Type(m.id)))
            .padding(Padding {
                top: 12.0,
                right: 12.0,
                bottom: 11.0,
                left: 12.0,
            })
            .width(Fill)
            .style(card(on)),
    )
    .on_double_click(event(Event::Choose(m.id)))
    .into()
}

/// An announced type: dimmed, with “Yakında”; it cannot be chosen.
fn soon_card<'a>(m: &'static Mode) -> Element<'a, Message> {
    let mark = container(icon(crate::modes::mode_icon(m.id)).size(20.0))
        .center_x(typography::from_default(32.0))
        .center_y(typography::from_default(32.0))
        .style(mark_box(false));
    let badge = container(
        text("Yakında")
            .font(typography::ui_strong())
            .size(typography::caption() - 1.0),
    )
    .padding([1, 7])
    .style(style::container::badge);
    container(
        row![
            mark,
            column![
                row![
                    text(m.label)
                        .font(typography::ui_strong())
                        .size(typography::heading())
                        .style(style::text::muted),
                    space::horizontal(),
                    badge,
                ]
                .align_y(Center),
                text(m.title)
                    .size(typography::caption())
                    .style(style::text::faint),
            ]
            .spacing(2)
            .width(Fill),
        ]
        .spacing(10)
        .align_y(Center),
    )
    .padding([9, 12])
    .width(Fill)
    .style(|theme: &Theme| {
        let t = Tokens::of(theme);
        container::Style {
            border: Border {
                color: t.border_strong(),
                width: 1.0,
                radius: shape::md().into(),
            },
            ..container::Style::default()
        }
    })
    .into()
}

// ── 2. Koordinatlar ────────────────────────────────────────────────────

pub(super) fn coords(s: &State) -> Element<'_, Message> {
    let d = &s.draft;
    if d.kind == Kind::Gis {
        return page(
            head(
                "Konum ve koordinat sistemi",
                "İli seçin: proje il merkezinde açılır ve ilin TM dilimi önerilir.",
            ),
            place(s),
        );
    }
    // What the drawing is for (docs/adr/0626 §1): its unit, scale and coordinates come with it; a module's program
    // offers its own works (docs/adr/0801).
    let chosen = d.work();
    let works = Column::with_children(
        WORKS
            .iter()
            .enumerate()
            .filter(|(_, w)| s.allows_work(w.work))
            .map(|(i, w)| {
                choice_card(
                    icon(crate::icons::from_web(Some(w.icon))).size(22.0).into(),
                    w.name,
                    w.note,
                    chosen == Some(w),
                    event(Event::Work(i)),
                )
            }),
    )
    .spacing(10);
    let mut parts = vec![works.into()];
    // A plan is on a real coordinate system: the province and the system chosen here.
    if d.coords == Coords::Real {
        parts.extend(place(s));
    }
    page(
        head(
            "Ne çiziyorsunuz?",
            "Seçiminiz projenin birimini, çizim ölçeğini ve koordinatlarını belirler; hepsi sonradan Proje ayarları’ndan değiştirilebilir.",
        ),
        parts,
    )
}

/// A card to choose one of (the web's `.wiz__choice`): a mark in its box,
/// the name, what it is for and the round mark.
pub(in crate::project) fn choice_card<'a>(
    mark: Element<'a, Message>,
    name: &'a str,
    note: &'a str,
    on: bool,
    message: Message,
) -> Element<'a, Message> {
    let mark = container(mark)
        .padding([0, 6])
        .center_y(typography::from_default(42.0))
        .center_x(typography::from_default(42.0))
        .style(mark_box(on));
    button(
        row![
            mark,
            column![
                text(name)
                    .font(typography::ui_strong())
                    .size(typography::heading()),
                text(note)
                    .size(typography::caption())
                    .style(style::text::muted),
            ]
            .spacing(2)
            .width(Fill),
            check(on),
        ]
        .spacing(12)
        .align_y(Center),
    )
    .on_press(message)
    .padding(12)
    .width(Fill)
    .style(card(on))
    .into()
}

/// The province's search and list, the systems with the suggested one
/// first, and the zone strip.
fn place(s: &State) -> Vec<Element<'_, Message>> {
    let d = &s.draft;
    let list_height = typography::from_default(206.0);
    let search = kentos_ui::widget::focus_ring(
        text_input("İl adı ya da plaka kodu", &s.query)
            .id(Id::new(super::SEARCH))
            .on_input(|q| event(Event::Search(q)))
            .on_submit(event(Event::SearchFirst))
            .padding(metrics::padding(metrics::control(), 8.0))
            .size(typography::body())
            .style(style::field::input),
    );
    let found = provinces::search(&s.query);
    let mut rows = Column::new().spacing(1);
    for p in &found {
        let on = d.province == Some(p.code);
        rows = rows.push(
            button(
                row![
                    container(label::mono_caption(format!("{:02}", p.code)))
                        .width(typography::from_default(36.0)),
                    label::body(p.name.clone()),
                ]
                .spacing(10)
                .align_y(Center),
            )
            .on_press(event(Event::Province(p.code)))
            .padding([4, 8])
            .width(Fill)
            .style(style::button::list_item(on)),
        );
    }
    if found.is_empty() {
        rows = rows.push(container(label::caption("Bu adla il yok.")).padding(10));
    }
    let provinces_list = list(rows, list_height);

    let chosen = d.srid();
    let mut systems = Column::new().spacing(1);
    let mut datum = "";
    for (crs, suggested) in d.system_choices() {
        if !suggested && crs.datum != datum {
            datum = &crs.datum;
            systems = systems.push(
                container(
                    text(datum_label(datum))
                        .font(typography::ui_strong())
                        .size(typography::caption() - 1.0)
                        .style(style::text::faint),
                )
                .padding(Padding {
                    top: 8.0,
                    right: 8.0,
                    bottom: 4.0,
                    left: 8.0,
                }),
            );
        }
        let tag: Element<'_, Message> = if suggested {
            container(
                text("Önerilen")
                    .font(typography::ui_strong())
                    .size(typography::caption() - 1.0),
            )
            .padding([1, 7])
            .style(|theme: &Theme| {
                let t = Tokens::of(theme);
                container::Style {
                    text_color: Some(t.on_accent),
                    background: Some(Background::Color(t.accent)),
                    border: Border {
                        radius: 999.0.into(),
                        ..Border::default()
                    },
                    ..container::Style::default()
                }
            })
            .into()
        } else {
            text(crs.area.clone().unwrap_or_default())
                .size(typography::caption() - 1.0)
                .style(style::text::faint)
                .into()
        };
        systems = systems.push(
            button(
                row![
                    container(label::mono_caption(crs.srid.to_string()))
                        .width(typography::from_default(40.0)),
                    container(label::body(crs.name.clone())).width(Fill),
                    tag,
                ]
                .spacing(10)
                .align_y(Center),
            )
            .on_press(event(Event::System(crs.srid)))
            .padding([4, 8])
            .width(Fill)
            .style(style::button::list_item(crs.srid == chosen)),
        );
    }
    // As tall as the province's search and list beside it.
    let systems_list = list(systems, list_height + 8.0 + metrics::control());

    let p = d.province();
    let strip_title = p.map_or_else(
        || "TM3 dilimleri".to_owned(),
        |p| format!("{}, {}° D", p.name, fixed(p.lon, 2)),
    );
    vec![
        row![
            container(section("İl", column![search, provinces_list].spacing(8)))
                .width(FillPortion(4)),
            container(section("Koordinat sistemi", systems_list)).width(FillPortion(6)),
        ]
        .spacing(16)
        .into(),
        section(
            strip_title,
            container(strip::view(chosen, p))
                .padding(Padding {
                    top: 8.0,
                    right: 10.0,
                    bottom: 4.0,
                    left: 10.0,
                })
                .width(Fill)
                .style(|theme: &Theme| {
                    let t = Tokens::of(theme);
                    container::Style {
                        background: Some(Background::Color(t.header)),
                        border: Border {
                            color: t.border,
                            width: 1.0,
                            radius: shape::md().into(),
                        },
                        ..container::Style::default()
                    }
                }),
        ),
    ]
}

/// A list in its box: the field's ground, a hairline edge, its own scroll.
fn list<'a>(rows: Column<'a, Message>, height: f32) -> Element<'a, Message> {
    container(
        scrollable(container(rows).padding(4))
            .direction(style::field::body_scrollbar())
            .height(height),
    )
    .width(Fill)
    .style(|theme: &Theme| {
        let t = Tokens::of(theme);
        container::Style {
            background: Some(Background::Color(t.field)),
            border: Border {
                color: t.border,
                width: 1.0,
                radius: shape::md().into(),
            },
            ..container::Style::default()
        }
    })
    .into()
}

// ── 3. Proje adı ───────────────────────────────────────────────────────

pub(super) fn details(s: &State, current: Option<Note>) -> Element<'_, Message> {
    let d = &s.draft;
    let name = kentos_ui::widget::focus_ring(
        text_input("Proje adı", &d.name)
            .id(Id::new(super::NAME))
            .on_input(|t| event(Event::Name(t)))
            .on_submit(event(Event::Next))
            .padding(metrics::padding(metrics::control(), 8.0))
            .size(typography::body())
            .width(typography::from_default(420.0))
            .style(style::field::input),
    );
    let summary = Column::with_children(d.summary().into_iter().map(|(k, v)| {
        row![
            container(label::muted(k)).width(typography::from_default(110.0)),
            container(label::body(v)).width(Fill),
        ]
        .spacing(16)
        .into()
    }))
    .spacing(6);
    let mut parts = vec![
        section("Proje adı", name),
        section(
            "Özet",
            container(summary)
                .padding([12, 14])
                .width(Fill)
                .style(|theme: &Theme| {
                    let t = Tokens::of(theme);
                    container::Style {
                        border: Border {
                            color: t.border_strong(),
                            width: 1.0,
                            radius: shape::md().into(),
                        },
                        ..container::Style::default()
                    }
                }),
        ),
    ];
    if let Some(note) = current {
        parts.push(if note.warn {
            Banner::warning(note.text).into()
        } else {
            Banner::info(note.text).into()
        });
    }
    page(
        head(
            "Proje adı",
            "Projeye bir ad verin. Yazı boyları, çizim yazı tipi ve diğer ayrıntılar Proje ayarları’ndadır.",
        ),
        parts,
    )
}
