//! An app on its plate (docs/adr/0803): no box around it, a picture that is the
//! thing itself and a caption under it, as a map book shows its sheets. Under
//! the pointer a faint ground gathers the two; only the one to start has the
//! accent's button. The apps still to come are smaller and paler.
//!
//! Every part has a height of its own, so that a plate's height is known
//! before it is laid out (the page says from it whether it has to scroll).

use iced::widget::text::Wrapping;
use iced::widget::tooltip::Position;
use iced::widget::{Space, button, column, container, image, row, space, stack, text};
use iced::{Alignment, ContentFit, Element, Fill, Length};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::theme::{Mode, Tokens, metrics, shape, typography};
use kentos_ui::widget::{Tip, tip};

use crate::app::{Launcher, Message};
use crate::cards::Status;
use crate::glyphs::{self, Glyph};
use crate::manifest::{Launch, Manifest, Picture};
use crate::resolve::Found;
use crate::view::app_menu::more;
use crate::view::style::{self, px};
use crate::view::{app_tile, glyph};

/// Between a plate's picture and its name, and between its name and its last line.
const UNDER_PICTURE: f32 = 12.0;
const UNDER_NAME: f32 = 10.0;
/// How much of a coming app's picture shows: paper shows through more kindly than graphite.
const PALE_DARK: f32 = 0.62;
const PALE_LIGHT: f32 = 0.78;

/// A picture's height for a plate of `cell` pixels: 16:9, whole pixels.
fn picture_height(cell: f32) -> f32 {
    ((cell - 2.0 * px(style::INSET)) * 9.0 / 16.0).round()
}

/// A plate's height for its width: every part's own.
pub fn plate_height(cell: f32) -> f32 {
    2.0 * px(style::INSET)
        + picture_height(cell)
        + px(UNDER_PICTURE)
        + px(style::TILE)
        + px(UNDER_NAME)
        + metrics::control()
}

/// A coming app's height.
pub fn coming_height() -> f32 {
    2.0 * px(style::INSET) + px(style::THUMB_HEIGHT)
}

/// A plate: its picture, its tile, name and note, its state and its button.
pub fn plate<'a>(launcher: &'a Launcher, app: &'a Manifest, cell: f32) -> Element<'a, Message> {
    let status = launcher.status(app);
    let hot = launcher.hot() == Some(app.id.as_str());
    let hovered = launcher.hovered.as_deref() == Some(app.id.as_str());
    let selected = launcher.selected.as_deref() == Some(app.id.as_str());
    let inset = px(style::INSET);
    let inner = cell - 2.0 * inset;
    let tile_side = px(style::TILE);
    let menu_side = metrics::control();
    let gap = px(10.0);

    // The name and the note on one line each, cut to fit with the whole in the tip.
    let room = inner - tile_side - gap - menu_side - px(4.0);
    let name_size = px(15.0);
    let name = style::fit(&app.name, name_size, room, true);
    let note = style::fit(&app.note, typography::body(), room, false);
    let cut = name != app.name.as_str() || note != app.note.as_str();
    let mut words = column![
        text(name.into_owned())
            .size(name_size)
            .font(typography::ui_strong())
            .wrapping(Wrapping::None)
    ]
    .spacing(px(1.0));
    if !app.note.is_empty() {
        words = words.push(label::muted(note.into_owned()).wrapping(Wrapping::None));
    }
    let words: Element<'_, Message> = if cut {
        tip(
            words,
            Tip::new(app.name.clone()).body(app.note.clone()),
            Position::Bottom,
        )
    } else {
        words.into()
    };
    let head = row![
        app_tile(launcher, app, tile_side),
        container(words).width(Fill),
        more(launcher, app, menu_side, hot),
    ]
    .spacing(gap)
    .height(Length::Fixed(tile_side))
    .align_y(Alignment::Center);

    let foot = row![
        state(app, status),
        space::horizontal(),
        action(launcher, app, status, hot)
    ]
    .height(Length::Fixed(metrics::control()))
    .align_y(Alignment::Center);

    let body = column![
        picture(
            launcher,
            app,
            inner,
            picture_height(cell),
            style::plate_radius(),
            1.0
        ),
        Space::new().height(px(UNDER_PICTURE)),
        head,
        Space::new().height(px(UNDER_NAME)),
        foot,
    ];
    let ground = container(body)
        .padding(inset)
        .width(Length::Fixed(cell))
        .height(Length::Fixed(plate_height(cell)))
        .style(style::plate(hovered, selected));
    let mut area = iced::widget::mouse_area(ground)
        .on_enter(Message::Hover(Some(app.id.clone())))
        .on_exit(Message::Hover(None))
        .on_press(Message::Select(app.id.clone()));
    if status.starts() {
        area = area.on_double_click(Message::Launch(app.id.clone()));
    }
    area.into()
}

/// An app still to come: its picture small and pale, its name and note beside
/// it; the section's head says what it is, so nothing here repeats it.
pub fn coming<'a>(launcher: &'a Launcher, app: &'a Manifest, cell: f32) -> Element<'a, Message> {
    let hovered = launcher.hovered.as_deref() == Some(app.id.as_str());
    let selected = launcher.selected.as_deref() == Some(app.id.as_str());
    let inset = px(style::INSET);
    let thumb = px(style::THUMB_WIDTH);
    let gap = px(12.0);
    // A manifest of a file can still be edited or shown: its menu stays.
    let from_file = app.origin.base().is_some();
    let menu_side = metrics::control();
    let room = cell - 2.0 * inset - thumb - gap - if from_file { menu_side + gap } else { 0.0 };
    let name = style::fit(&app.name, typography::body(), room, true);
    let note = style::fit(&app.note, typography::caption(), room, false);
    let mut line = row![
        picture(
            launcher,
            app,
            thumb,
            px(style::THUMB_HEIGHT),
            shape::md(),
            if launcher.mode() == Mode::Light {
                PALE_LIGHT
            } else {
                PALE_DARK
            },
        ),
        column![
            label::strong(name.into_owned()).wrapping(Wrapping::None),
            label::caption(note.into_owned())
                .style(style::soft)
                .wrapping(Wrapping::None),
        ]
        .spacing(px(2.0))
        .width(Fill),
    ]
    .spacing(gap)
    .align_y(Alignment::Center);
    if from_file {
        line = line.push(more(
            launcher,
            app,
            menu_side,
            launcher.hot() == Some(app.id.as_str()),
        ));
    }
    let ground = container(line)
        .padding(inset)
        .width(Length::Fixed(cell))
        .height(Length::Fixed(coming_height()))
        .style(style::plate(hovered, selected));
    tip(
        iced::widget::mouse_area(ground)
            .on_enter(Message::Hover(Some(app.id.clone())))
            .on_exit(Message::Hover(None))
            .on_press(Message::Select(app.id.clone())),
        Tip::new(app.name.clone()).body(format!(
            "{}. Henüz çıkmadı; hazır olunca buradan başlatılır.",
            app.note
        )),
        Position::Top,
    )
}

/// A picture of `width` × `height`: the bundled one, the manifest's own, or
/// the tile's glyph on its tone when there is none.
fn picture<'a>(
    launcher: &'a Launcher,
    app: &'a Manifest,
    width: f32,
    height: f32,
    radius: f32,
    opacity: f32,
) -> Element<'a, Message> {
    let handle = match &app.picture {
        Some(Picture::Builtin(shot)) => launcher.pictures.ready(*shot).cloned(),
        Some(Picture::File(path)) => launcher.pictures.ready_file(path).cloned(),
        None => None,
    };
    let Some(handle) = handle else {
        let glyph = match &app.icon {
            crate::manifest::Icon::Glyph(g) => *g,
            crate::manifest::Icon::File(_) => Glyph::default(),
        };
        let paint = app.tone.paint();
        return container(
            icon(Icon::Svg(glyph.svg()))
                .size((height * 0.3).round())
                .color(paint.top),
        )
        .center_x(Length::Fixed(width))
        .center_y(Length::Fixed(height))
        .style(style::placeholder(paint, radius))
        .into();
    };
    stack![
        image(handle)
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .content_fit(ContentFit::Cover)
            .border_radius(radius)
            .opacity(opacity),
        container(Space::new())
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .style(style::picture_edge(radius)),
    ]
    .into()
}

/// “● Kurulu  v0.1.0”: the dot carries the state's colour; the word is quiet
/// unless something is going on (running, not found).
fn state<'a>(app: &'a Manifest, status: Status) -> Element<'a, Message> {
    let color = move |theme: &iced::Theme| {
        let t = Tokens::of(theme);
        match status {
            Status::Installed => t.success,
            Status::Running(_) => t.accent_hover,
            Status::Soon => t.faint,
            Status::Missing => t.warning,
            Status::Web => t.info,
        }
    };
    let dot = px(6.0);
    let word = label::caption(status.word());
    let word = match status {
        Status::Running(_) | Status::Missing => {
            word.font(typography::ui_strong())
                .style(move |theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(color(theme)),
                })
        }
        _ => word.style(kentos_ui::style::text::muted),
    };
    let mut line = row![
        container(Space::new())
            .width(dot)
            .height(dot)
            .style(move |theme: &iced::Theme| style::dot(color(theme))(theme)),
        container(word).padding(iced::Padding {
            right: px(4.0),
            ..iced::Padding::ZERO
        }),
    ]
    .spacing(px(6.0))
    .align_y(Alignment::Center);
    if let Some(version) = &app.version {
        line = line.push(
            label::caption(format!("v{}", version.trim_start_matches(['v', 'V'])))
                .style(style::soft),
        );
    }
    line.into()
}

/// The plate's button: Başlat (the accent's on the plate to start), Aç for a
/// web address, Konumu seç… for a program not found.
fn action<'a>(
    launcher: &'a Launcher,
    app: &'a Manifest,
    status: Status,
    hot: bool,
) -> Element<'a, Message> {
    let height = metrics::control();
    let face = |mark: Element<'a, Message>, words: &'a str| {
        container(
            row![mark, label::strong(words).wrapping(Wrapping::None)]
                .spacing(px(6.0))
                .align_y(Alignment::Center),
        )
        .center_y(Length::Fixed(height))
    };
    let start_style: fn(&iced::Theme, button::Status) -> button::Style = if hot {
        kentos_ui::style::button::primary
    } else {
        style::quiet
    };
    let shaped = |content: iced::widget::Container<'a, Message>| {
        button(content)
            .height(Length::Fixed(height))
            .padding([0.0, px(11.0)])
    };
    match status {
        // A coming app has no plate, and nothing to start.
        Status::Soon => Space::new().into(),
        Status::Installed | Status::Running(_) => {
            let mark = icon(Icon::Play).size(px(10.0)).into();
            shaped(face(mark, "Başlat"))
                .style(start_style)
                .on_press(Message::Launch(app.id.clone()))
                .into()
        }
        Status::Web => shaped(face(glyph(glyphs::EXTERNAL, px(13.0)), "Aç"))
            .style(start_style)
            .on_press(Message::Launch(app.id.clone()))
            .into(),
        Status::Missing => {
            let why = match launcher.found.get(&app.id) {
                Some(Found::Missing(why)) => why.clone(),
                _ => String::new(),
            };
            let locatable = !matches!(app.launch, Some(Launch::Url(_)) | None);
            let mut button = shaped(face(
                icon(Icon::Folder).size(px(13.0)).into(),
                "Konumu seç…",
            ))
            .style(style::quiet);
            if locatable {
                button = button.on_press(Message::Locate(app.id.clone()));
            }
            tip(
                button,
                Tip::new("Program bulunamadı").body(why),
                Position::Bottom,
            )
        }
    }
}
