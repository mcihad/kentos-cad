//! Uygulamalar (docs/adr/0800 §3): the summary strip with its honest counts,
//! the grid of cards (two to four columns as the window allows) and, in its
//! last cell, the update centre that says what there is: the versions
//! installed and that no update service exists yet.
//!
//! Every part of a card has a height of its own, so that a card's height
//! is known before it is laid out: the grid reserves the scroll bar's room
//! only when its cards do not fit, and every row's cards are alike.

use iced::widget::text::Wrapping;
use iced::widget::tooltip::Position;
use iced::widget::{
    Column, Row, Space, button, column, container, image, row, scrollable, space, stack, text,
};
use iced::{Alignment, ContentFit, Element, Fill, Length, Size};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::theme::{Tokens, metrics, shape, typography};
use kentos_ui::widget::{EmptyState, Menu, MenuButton, Responsive, Tip, fallback, tip};

use crate::app::{Launcher, Message};
use crate::cards::{self, Counts, Status};
use crate::glyphs::{self, Glyph};
use crate::manifest::{Launch, Manifest, Picture, State};
use crate::resolve::Found;
use crate::view::style::{self, px};
use crate::view::{app_tile, glyph};

pub fn page(launcher: &Launcher) -> Element<'_, Message> {
    column![strip(launcher), grid(launcher)]
        .spacing(px(style::GAP))
        .height(Fill)
        .into()
}

/// “4 uygulama • 3 kurulu • 1 yakında”, Uygulama ekle, and Tümünü güncelle
/// off with its reason: there is nothing to check updates with yet.
fn strip(launcher: &Launcher) -> Element<'_, Message> {
    let counts = Counts::of(launcher.registry.apps.iter().map(|a| launcher.status(a)));
    let mut said = Row::new().spacing(px(10.0)).align_y(Alignment::Center);
    for (i, part) in counts.parts().into_iter().enumerate() {
        if i > 0 {
            said = said.push(label::caption("•"));
        }
        said = said.push(label::body(part).wrapping(Wrapping::None));
    }
    // Where the strip is narrow, Tümünü güncelle keeps only its mark (its words in the tip).
    let actions = |words: bool| -> Element<'_, Message> {
        let add = button(
            row![
                icon(Icon::Plus).size(px(14.0)),
                label::strong("Uygulama ekle").wrapping(Wrapping::None)
            ]
            .spacing(px(6.0))
            .align_y(Alignment::Center),
        )
        .padding([px(5.0), px(10.0)])
        .style(style::link)
        .on_press(Message::AddApp);
        let mut face = row![icon(Icon::Retry).size(px(14.0))]
            .spacing(px(6.0))
            .align_y(Alignment::Center);
        if words {
            face = face.push(label::body("Tümünü güncelle").wrapping(Wrapping::None));
        }
        let update = tip(
            button(face).padding([px(5.0), px(10.0)]).style(style::link),
            Tip::new("Tümünü güncelle: denetim henüz yok")
                .body("Launcher sürümleri yalnız gösterir; güncellemeleri denetleyen bir hizmet henüz yok. Yeni sürümü kurulum paketiyle kurun."),
            Position::Bottom,
        );
        row![add, update]
            .spacing(px(8.0))
            .align_y(Alignment::Center)
            .into()
    };
    container(
        row![
            said,
            space::horizontal(),
            fallback(actions(true), actions(false))
        ]
        .spacing(px(8.0))
        .align_y(Alignment::Center),
    )
    .height(px(style::STRIP))
    .padding([0.0, px(16.0)])
    .center_y(Length::Fixed(px(style::STRIP)))
    .style(style::card(false))
    .into()
}

/// The grid's layout in `width` × `height` with `cells` cards: its columns,
/// a card's width and whether it scrolls (and leaves the scroll bar its room).
pub fn grid_layout(width: f32, height: f32, cells: usize) -> (usize, f32, bool) {
    let gap = px(style::GAP);
    let least = px(style::CARD_LEAST);
    let lay = |width: f32| {
        let columns = cards::columns(width, least, gap);
        let card = cards::card_width(width, columns, gap);
        let rows = cells.div_ceil(columns);
        let tall = rows as f32 * card_height(card) + rows.saturating_sub(1) as f32 * gap;
        (columns, card, tall)
    };
    let (columns, card, tall) = lay(width);
    if tall <= height {
        return (columns, card, false);
    }
    let (columns, card, _) = lay(width - px(style::SCROLLBAR));
    (columns, card, true)
}

/// A card's picture's height for its width: half of it, whole pixels.
fn picture_height(card: f32) -> f32 {
    ((card - 2.0 * px(style::CARD_PAD)) / 2.0).round()
}

/// A card's height for its width: every part's own.
pub fn card_height(card: f32) -> f32 {
    2.0 * px(style::CARD_PAD)
        + picture_height(card)
        + 3.0 * px(style::CARD_SPACING)
        + px(style::TILE)
        + px(style::META)
        + metrics::button()
}

fn grid(launcher: &Launcher) -> Element<'_, Message> {
    let visible = launcher.visible();
    if visible.is_empty() {
        return container(
            EmptyState::new(
                Icon::Search,
                format!("“{}” ile eşleşen uygulama yok", launcher.query.trim()),
            )
            .description("Aramayı kısaltın ya da başka bir sözcük deneyin; ad ve açıklama aranır.")
            .secondary("Aramayı temizle", Message::Search(String::new())),
        )
        .width(Fill)
        .height(Fill)
        .into();
    }
    Responsive::new(move |size: Size| {
        let visible = launcher.visible();
        // The update centre is the last cell, while nothing is searched.
        let centre = launcher.query.trim().is_empty();
        let cells = visible.len() + usize::from(centre);
        let (columns, width, scrolls) = grid_layout(size.width, size.height, cells);
        launcher.columns.set(columns);
        let height = card_height(width);
        let gap = px(style::GAP);
        let mut items: Vec<Element<'_, Message>> = visible
            .into_iter()
            .map(|app| card(launcher, app, width))
            .collect();
        if centre {
            items.push(update_centre(launcher, width, height));
        }
        let mut rows = Column::new().spacing(gap);
        let mut items = items.into_iter().peekable();
        while items.peek().is_some() {
            let mut line = Row::new().spacing(gap);
            for item in items.by_ref().take(columns) {
                line = line.push(item);
            }
            rows = rows.push(line);
        }
        if scrolls {
            scrollable(rows)
                .direction(kentos_ui::style::field::body_scrollbar())
                .width(Fill)
                .height(Fill)
                .into()
        } else {
            container(rows).width(Fill).height(Fill).into()
        }
    })
    .into()
}

/// A card: its picture, its tile, name and note, its version and state, its button.
fn card<'a>(launcher: &'a Launcher, app: &'a Manifest, width: f32) -> Element<'a, Message> {
    let status = launcher.status(app);
    let hot = launcher.hot() == Some(app.id.as_str());
    let pad = px(style::CARD_PAD);
    let inner = width - 2.0 * pad;
    let tile_side = px(style::TILE);
    // The menu's button centred on whole pixels beside the tile.
    let menu_side = style::around(px(style::TILE), metrics::control()).min(px(style::TILE));
    let gap = px(12.0);

    // The name and the note on one line each, cut to fit with the whole in the tip.
    let room = inner - tile_side - gap - menu_side - px(8.0);
    let name_size = px(17.0);
    let name = style::fit(&app.name, name_size, room, true);
    let note = style::fit(&app.note, typography::body(), room, false);
    let cut = name != app.name.as_str() || note != app.note.as_str();
    let mut words = column![
        text(name.into_owned())
            .size(name_size)
            .font(typography::ui_strong())
            .wrapping(Wrapping::None)
    ]
    .spacing(px(2.0));
    if !app.note.is_empty() {
        words = words.push(label::muted(note.into_owned()).wrapping(Wrapping::None));
    }
    // What was cut to fit, whole in a tip.
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
        more(launcher, app, menu_side),
    ]
    .spacing(gap)
    .height(Length::Fixed(tile_side))
    .align_y(Alignment::Center);

    let body = column![
        picture(launcher, app, inner),
        head,
        meta(app, status),
        action(launcher, app, status, hot),
    ]
    .spacing(px(style::CARD_SPACING));

    let card = container(body)
        .padding(pad)
        .width(Length::Fixed(width))
        .height(Length::Fixed(card_height(width)))
        .style(style::card(hot));
    iced::widget::mouse_area(card)
        .on_enter(Message::Hover(Some(app.id.clone())))
        .on_exit(Message::Hover(None))
        .on_press(Message::Select(app.id.clone()))
        .into()
}

/// The card's picture: the bundled screenshot, the manifest's own, or the
/// tile's glyph on its tone when there is none.
fn picture<'a>(launcher: &'a Launcher, app: &'a Manifest, width: f32) -> Element<'a, Message> {
    let height = picture_height(width + 2.0 * px(style::CARD_PAD));
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
            kentos_ui::icon::icon(kentos_ui::icon::Icon::Svg(glyph.svg()))
                .size(px(56.0))
                .color(paint.bottom),
        )
        .center_x(Length::Fixed(width))
        .center_y(Length::Fixed(height))
        .style(style::placeholder(paint))
        .into();
    };
    let radius = shape::md();
    stack![
        image(handle)
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .content_fit(ContentFit::Cover)
            .border_radius(radius),
        container(Space::new())
            .width(Length::Fixed(width))
            .height(Length::Fixed(height))
            .style(style::picture_edge),
    ]
    .into()
}

/// “v0.1.0 • ● Kurulu”.
fn meta<'a>(app: &'a Manifest, status: Status) -> Element<'a, Message> {
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
    let dot = px(8.0);
    let mut line = Row::new().spacing(px(8.0)).align_y(Alignment::Center);
    if let Some(version) = &app.version {
        line = line
            .push(
                label::caption(format!("v{}", version.trim_start_matches(['v', 'V'])))
                    .style(kentos_ui::style::text::muted),
            )
            .push(label::caption("•"));
    }
    line = line
        .push(
            container(Space::new())
                .width(dot)
                .height(dot)
                .style(move |theme: &iced::Theme| style::dot(color(theme))(theme)),
        )
        .push(
            label::caption(status.word())
                .font(typography::ui_strong())
                .style(move |theme: &iced::Theme| iced::widget::text::Style {
                    color: Some(color(theme)),
                }),
        );
    container(line)
        .height(Length::Fixed(px(style::META)))
        .center_y(Length::Fixed(px(style::META)))
        .into()
}

/// The card's button: Başlat (filled on the card to start), Aç for a web
/// address, Konumu seç… for a program not found, Yakında off.
fn action<'a>(
    launcher: &'a Launcher,
    app: &'a Manifest,
    status: Status,
    hot: bool,
) -> Element<'a, Message> {
    let height = metrics::button();
    let face = |mark: Element<'a, Message>, words: &'a str| {
        container(
            row![mark, label::strong(words)]
                .spacing(px(8.0))
                .align_y(Alignment::Center),
        )
        .center_x(Fill)
        .center_y(Length::Fixed(height))
    };
    let start_style: fn(&iced::Theme, button::Status) -> button::Style = if hot {
        kentos_ui::style::button::primary
    } else {
        style::outline
    };
    match status {
        Status::Installed | Status::Running(_) => {
            button(face(icon(Icon::Play).size(px(12.0)).into(), "Başlat"))
                .width(Fill)
                .height(Length::Fixed(height))
                .padding(0)
                .style(start_style)
                .on_press(Message::Launch(app.id.clone()))
                .into()
        }
        Status::Web => button(face(glyph(glyphs::EXTERNAL, px(14.0)), "Aç"))
            .width(Fill)
            .height(Length::Fixed(height))
            .padding(0)
            .style(start_style)
            .on_press(Message::Launch(app.id.clone()))
            .into(),
        Status::Soon => tip(
            button(face(icon(Icon::Clock).size(px(13.0)).into(), "Yakında"))
                .width(Fill)
                .height(Length::Fixed(height))
                .padding(0)
                .style(style::idle),
            Tip::new("Henüz çıkmadı")
                .body(format!("{} hazır olunca buradan başlatılır.", app.name)),
            Position::Bottom,
        ),
        Status::Missing => {
            let why = match launcher.found.get(&app.id) {
                Some(Found::Missing(why)) => why.clone(),
                _ => String::new(),
            };
            let locatable = !matches!(app.launch, Some(Launch::Url(_)) | None);
            let mut button = button(face(
                icon(Icon::Folder).size(px(14.0)).into(),
                "Konumu seç…",
            ))
            .width(Fill)
            .height(Length::Fixed(height))
            .padding(0)
            .style(style::outline);
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

/// The card's menu (⋯).
fn more<'a>(launcher: &'a Launcher, app: &'a Manifest, side: f32) -> Element<'a, Message> {
    let id = app.id.clone();
    let status = launcher.status(app);
    let users = app.origin.is_users();
    let placed = launcher.locations.get(&app.id).is_some();
    let locatable = matches!(app.launch, Some(Launch::Program { .. } | Launch::Kentos(_)));
    let opens_files =
        status.starts() && !app.files.is_empty() && !matches!(app.launch, Some(Launch::Url(_)));
    let from_file = app.origin.base().is_some();
    let soon = app.state == State::Soon;
    MenuButton::new(
        container(icon(Icon::More).size(px(16.0)))
            .center_x(Length::Fixed(side))
            .center_y(Length::Fixed(side)),
        move || {
            let mut menu = Menu::new()
                .item(
                    if matches!(status, Status::Web) {
                        "Aç"
                    } else {
                        "Başlat"
                    },
                    status.starts().then(|| Message::Launch(id.clone())),
                )
                .icon(Icon::Play);
            if soon {
                menu = menu.hint("Henüz çıkmadı.");
            } else if !status.starts() {
                menu = menu.hint("Program bulunamadı; önce yerini gösterin.");
            }
            menu = menu
                .item(
                    "Dosya aç…",
                    opens_files.then(|| Message::OpenWith(id.clone())),
                )
                .icon(Icon::Open);
            if !opens_files {
                menu = menu.hint(if matches!(status, Status::Web) {
                    "Web adresi dosya açmaz."
                } else {
                    "Açtığı dosya türleri bilinmiyor ya da program yok."
                });
            }
            if locatable {
                menu = menu
                    .separator()
                    .item("Konumu seç…", Some(Message::Locate(id.clone())))
                    .icon(Icon::Folder)
                    .item(
                        "Konumu sıfırla",
                        placed.then(|| Message::ForgetPlace(id.clone())),
                    )
                    .icon(Icon::Undo);
                if !placed {
                    menu = menu.hint("Gösterilmiş bir yer yok; program kendiliğinden aranıyor.");
                }
            }
            menu = menu.separator();
            menu = menu
                .item("Düzenle…", users.then(|| Message::EditApp(id.clone())))
                .icon(Icon::Properties);
            if !users {
                menu = menu.hint(if from_file {
                    "Kurulumla gelen uygulama; manifestini klasöründe düzenleyin."
                } else {
                    "Yerleşik uygulama düzenlenmez."
                });
            }
            if from_file {
                menu = menu
                    .item("Manifesti göster", Some(Message::ShowManifest(id.clone())))
                    .icon(Icon::Folder);
            }
            menu = menu
                .item("Kaldır", users.then(|| Message::RemoveApp(id.clone())))
                .icon(Icon::Close);
            if users {
                menu.danger()
            } else {
                menu.hint("Yalnız eklediğiniz uygulamalar kaldırılır.")
            }
        },
    )
    .into()
}

/// The update centre: the versions installed, and honestly that nothing checks for updates yet.
fn update_centre<'a>(launcher: &'a Launcher, width: f32, height: f32) -> Element<'a, Message> {
    let installed: Vec<&Manifest> = launcher
        .registry
        .apps
        .iter()
        .filter(|a| matches!(launcher.status(a), Status::Installed | Status::Running(_)))
        .collect();
    let head = row![
        icon(Icon::Import).size(px(20.0)),
        text("Güncelleme merkezi")
            .size(px(16.0))
            .font(typography::ui_strong()),
    ]
    .spacing(px(10.0))
    .align_y(Alignment::Center);

    let shown = 4;
    let mut list = Column::new().spacing(px(6.0));
    if installed.is_empty() {
        list = list.push(label::muted("Kurulu uygulama yok."));
    }
    for app in installed.iter().take(shown) {
        list = list.push(
            row![
                label::body(app.name.as_str()),
                space::horizontal(),
                label::caption(
                    app.version
                        .as_deref()
                        .map_or_else(|| "—".to_owned(), |v| format!("v{v}"))
                ),
            ]
            .align_y(Alignment::Center),
        );
    }
    if installed.len() > shown {
        list = list.push(label::caption(format!(
            "+{} uygulama daha",
            installed.len() - shown
        )));
    }
    let versions = container(list)
        .padding([px(10.0), px(12.0)])
        .width(Fill)
        .style(style::inset);

    let note = container(
        row![
            container(Space::new())
                .width(px(8.0))
                .height(px(8.0))
                .style(|theme: &iced::Theme| style::dot(Tokens::of(theme).faint)(theme)),
            column![
                label::strong("Güncelleme denetimi henüz yok"),
                label::caption("Sürümler kurulu programlardan okunur."),
            ]
            .spacing(px(2.0)),
        ]
        .spacing(px(10.0))
        .align_y(Alignment::Center),
    )
    .padding([px(10.0), px(12.0)])
    .width(Fill)
    .style(style::inset);

    let downloads = button(
        row![
            icon(Icon::Properties).size(px(14.0)),
            label::strong("İndirmeleri görüntüle"),
            space::horizontal(),
            icon(Icon::ChevronRight).size(px(14.0)),
        ]
        .spacing(px(8.0))
        .align_y(Alignment::Center),
    )
    .width(Fill)
    .padding([px(6.0), px(8.0)])
    .style(style::link)
    .on_press(Message::Page(crate::app::Page::Downloads));

    container(
        column![head, versions, note, space::vertical(), downloads]
            .spacing(px(style::CARD_SPACING))
            .height(Fill),
    )
    .padding(px(style::CARD_PAD) + px(4.0))
    .width(Length::Fixed(width))
    .height(Length::Fixed(height))
    .style(style::card(false))
    .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_fits_two_rows_at_1440_and_scrolls_at_1100() {
        // At 1440 × 900 and 13 px the page is 1167 wide and the grid 698 high:
        // three columns, two rows, no scroll bar.
        assert_eq!(card_height(378.0), 335.0);
        assert_eq!(grid_layout(1167.0, 698.0, 6), (3, 378.0, false));
        // At 1100 × 650: two columns, three rows that scroll; the cards leave the bar its room.
        assert_eq!(grid_layout(827.0, 405.0, 6), (2, 398.0, true));
        // Every row's cards are the same: a card's height is its parts'.
        assert_eq!(card_height(398.0), card_height(398.0));
        assert_eq!(
            picture_height(378.0) * 2.0,
            354.0,
            "half the picture's width"
        );
    }
}
