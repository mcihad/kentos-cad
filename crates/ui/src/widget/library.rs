//! Kitaplık tarayıcılarının ortak parçaları (malzeme ve 3B nesne
//! tarayıcısı): arama kutusu, kategori hapları, önizlemeli karo, yüklenirken
//! iskelet karolar, boş durum ve ayrıntı panelinin özellik satırları.

use iced::widget::{Column, Row, button, column, container, space};
use iced::{Center, Element, Fill, Length};

use crate::icon::{Icon, Tone, icon};
use crate::label;
use crate::style;
use crate::theme::{Tokens, metrics, typography};
use crate::widget::InputField;
use crate::widget::assets::DoubleClick;
use crate::widget::elided::Elided;

/// Kategorileri öğelerdeki ilk görünüş sırasıyla toplar.
pub(crate) fn categories<'a>(names: impl IntoIterator<Item = &'a str>) -> Vec<String> {
    let mut categories: Vec<String> = Vec::new();

    for name in names {
        if !categories.iter().any(|category| category == name) {
            categories.push(name.to_owned());
        }
    }

    categories
}

/// Kategori hapları: “Tümü” ve öğelerin kategorileri; sığmayan alt satıra.
pub(crate) fn chips<'a, Message: Clone + 'a>(
    categories: &[String],
    selected: Option<&str>,
    on_category: &(dyn Fn(Option<String>) -> Message + 'a),
) -> Element<'a, Message> {
    let chip = |name: String, active: bool, message: Message| {
        button(
            container(label::caption(name).style(move |theme: &iced::Theme| {
                let t = Tokens::of(theme);

                iced::widget::text::Style {
                    color: Some(if active { t.accent_hover } else { t.muted }),
                }
            }))
            .center_y(Fill),
        )
        .on_press(message)
        .padding([0, 10])
        .height(typography::from_default(24.0))
        .style(style::button::chip(active))
    };

    let mut chips = Row::new().spacing(4).push(chip(
        "Tümü".to_owned(),
        selected.is_none(),
        on_category(None),
    ));

    for category in categories {
        chips = chips.push(chip(
            category.clone(),
            selected == Some(category.as_str()),
            on_category(Some(category.clone())),
        ));
    }

    chips.wrap().vertical_spacing(4).into()
}

/// Önizlemeli karo: önizleme, ad ve altında üçüncül bir satır; tıklamak
/// seçer, çift tıklamak kullanır.
#[allow(clippy::too_many_arguments)]
pub(crate) fn tile<'a, Message: Clone + 'a>(
    preview: Element<'a, Message>,
    preview_side: f32,
    width: f32,
    name: String,
    caption: Option<String>,
    badge: Option<String>,
    selected: bool,
    on_press: Message,
    on_activate: Option<Message>,
) -> Element<'a, Message> {
    let mut stage = iced::widget::Stack::new()
        .push(container(preview).width(preview_side).height(preview_side));

    if let Some(badge) = badge {
        stage = stage.push(
            container(
                container(label::caption(badge).style(style::text::default))
                    .padding([0, 5])
                    .style(style::container::badge),
            )
            .width(preview_side)
            .height(preview_side)
            .align_x(iced::Right)
            .align_y(iced::Top),
        );
    }

    // Uzun ad ve açıklama “…” ile kısalır, ortada durur.
    let mut text = column![
        container(
            Elided::new(name)
                .size(typography::caption())
                .font(typography::ui())
        )
        .center_x(Fill)
    ]
    .spacing(1)
    .width(Fill);

    if let Some(caption) = caption {
        text = text.push(
            container(
                Elided::new(caption)
                    .size(typography::caption())
                    .font(typography::ui())
                    .style(style::text::faint),
            )
            .center_x(Fill),
        );
    }

    let content = column![container(stage).center_x(Fill), text]
        .spacing(6)
        .align_x(Center)
        .width(Fill);

    let item: Element<'a, Message> = button(container(content).clip(true))
        .on_press(on_press)
        .padding([8, 6])
        .width(width)
        .style(style::button::library_tile(selected))
        .into();

    match on_activate {
        Some(message) => DoubleClick {
            content: item,
            message,
        }
        .into(),
        None => item,
    }
}

/// Yüklenirken gösterilen iskelet karolar: önizlemenin ve adın yeri.
pub(crate) fn skeletons<'a, Message: 'a>(
    count: usize,
    preview_side: f32,
    width: f32,
    round: bool,
) -> Element<'a, Message> {
    let block = move |w: f32, h: f32, radius: f32| {
        container(space::horizontal())
            .width(w)
            .height(h)
            .style(move |theme: &iced::Theme| container::Style {
                background: Some(Tokens::of(theme).layer(0.06).into()),
                border: iced::Border {
                    radius: radius.into(),
                    ..iced::Border::default()
                },
                ..container::Style::default()
            })
    };

    let tiles = (0..count).map(|_| {
        container(
            column![
                block(
                    preview_side,
                    preview_side,
                    if round {
                        preview_side / 2.0
                    } else {
                        crate::theme::shape::md()
                    }
                ),
                block(width * 0.62, typography::scaled(8.0), 4.0),
                block(width * 0.4, typography::scaled(7.0), 4.0),
            ]
            .spacing(6)
            .align_x(Center),
        )
        .padding([8, 6])
        .width(width)
        .into()
    });

    Row::with_children(tiles).spacing(8).wrap().into()
}

/// Boş durum: aramaya uyan öğe yoksa ya da kitaplık boşsa.
pub(crate) fn empty<'a, Message: 'a>(message: String) -> Element<'a, Message> {
    container(
        column![
            icon(Icon::Search).size(18.0).tone(Tone::Muted),
            label::muted(message).center(),
        ]
        .spacing(6)
        .align_x(Center),
    )
    .center_x(Fill)
    .padding(24)
    .into()
}

/// Etiketler küçük haplar olarak; sığmayan alt satıra.
pub(crate) fn tags<'a, Message: 'a>(tags: &[String]) -> Element<'a, Message> {
    Row::with_children(tags.iter().map(|tag| {
        container(label::caption(tag.clone()))
            .padding([1, 8])
            .style(|theme: &iced::Theme| {
                let t = Tokens::of(theme);

                container::Style {
                    background: Some(t.layer(0.05).into()),
                    border: iced::Border {
                        color: t.border,
                        width: 1.0,
                        radius: 999.0.into(),
                    },
                    ..container::Style::default()
                }
            })
            .into()
    }))
    .spacing(4)
    .wrap()
    .vertical_spacing(4)
    .into()
}

/// Ayrıntı panelinin altındaki birincil eylem düğmesi (ör. Uygula).
pub(crate) fn action<'a, Message: Clone + 'a>(
    label_text: String,
    message: Message,
) -> Element<'a, Message> {
    button(container(label::strong(label_text)).center(Fill))
        .on_press(message)
        .width(Fill)
        .height(metrics::control())
        .padding(0)
        .style(style::button::primary)
        .into()
}

/// Ayrıntı paneli: kenarlı, kendi zemininde, içerik dikey dizilir.
pub(crate) fn panel<'a, Message: 'a>(
    content: Column<'a, Message>,
    width: Length,
) -> Element<'a, Message> {
    container(content.spacing(10))
        .padding(12)
        .width(width)
        .style(style::container::bordered)
        .into()
}

/// Arama kutusu (InputField): ikonlu, doluyken temizlenir; süzülmüşse
/// sağında kaç öğenin göründüğü yazar.
pub(crate) fn search_field<'a, Message: Clone + 'a>(
    placeholder: &str,
    query: &str,
    on_search: impl Fn(String) -> Message + 'a,
    shown: usize,
    total: usize,
) -> Element<'a, Message> {
    let clear = on_search(String::new());
    let mut field = InputField::new(placeholder, query)
        .on_input(on_search)
        .icon(Icon::Search)
        .clear(clear);

    if !query.trim().is_empty() {
        field = field.suffix(format!("{shown}/{total}"));
    }

    field.into()
}
