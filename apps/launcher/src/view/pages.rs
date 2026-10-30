//! Son dosyalar, Güncellemeler, İndirmeler and Ayarlar (docs/adr/0803, 0800
//! §5, §6): on the sheet with nothing around them, hairlines between their
//! rows. The update and download pages say plainly that there is no update
//! service yet and list what is installed; nothing there pretends to work.

use std::path::Path;

use iced::widget::tooltip::Position;
use iced::widget::{Column, Space, button, column, container, row, scrollable};
use iced::{Alignment, Element, Fill, Length};
use kentos_ui::attribute::text as words;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::theme::metrics;
use kentos_ui::theme::typography::Typography;
use kentos_ui::widget::{EmptyState, Responsive, Segmented, Switch, Tip, tip};

use crate::app::{Launcher, Message, Page};
use crate::recent::Row;
use crate::settings::{Settings, ThemeChoice};
use crate::view::style::{self, px};
use crate::view::tables;

/// The settings' rows are read across this much at most.
const SETTINGS_MOST: f32 = 760.0;

pub fn recent(launcher: &Launcher) -> Element<'_, Message> {
    let query = launcher.file_query.trim();
    let rows: Vec<(usize, &Row)> = launcher
        .recent
        .iter()
        .enumerate()
        .filter(|(_, r)| {
            query.is_empty() || words::contains(&format!("{} {}", r.name, r.folder), query)
        })
        .collect();
    if launcher.recent.is_empty() {
        return EmptyState::new(Icon::Clock, "Son dosya yok")
            .description(
                "KentOS programlarında açılan ve kaydedilen çizimler burada görünür. Bir uygulamayı başlatıp çizim açın.",
            )
            .secondary("Yeniden oku", Message::Reload)
            .into();
    }
    if rows.is_empty() {
        return EmptyState::new(Icon::Search, format!("“{query}” ile eşleşen dosya yok"))
            .description("Dosyanın adı ve klasörü aranır.")
            .secondary("Aramayı temizle", Message::FileSearch(String::new()))
            .into();
    }
    Responsive::new(move |size: iced::Size| {
        let query = launcher.file_query.trim();
        let width = size.width - px(style::SCROLLBAR);
        let rows = launcher
            .recent
            .iter()
            .enumerate()
            .filter(|(_, r)| {
                query.is_empty() || words::contains(&format!("{} {}", r.name, r.folder), query)
            })
            .map(|(index, row)| tables::recent_row(launcher, index, row, width));
        // In a column: the scroll area would stretch the list to its own height.
        scrollable(column![tables::lines(rows)])
            .direction(kentos_ui::style::field::body_scrollbar())
            .style(style::scroller)
            .height(Fill)
            .into()
    })
    .into()
}

/// The installed apps, their versions and where they run from.
fn installed(launcher: &Launcher) -> Element<'_, Message> {
    Responsive::new(move |size: iced::Size| {
        let width = size.width - px(style::SCROLLBAR);
        scrollable(column![tables::lines(
            launcher
                .registry
                .apps
                .iter()
                .map(|app| tables::installed_row(launcher, app, width)),
        )])
        .direction(kentos_ui::style::field::body_scrollbar())
        .style(style::scroller)
        .height(Fill)
        .into()
    })
    .into()
}

pub fn updates(launcher: &Launcher) -> Element<'_, Message> {
    // Off with its reason: there is nothing to check updates with yet.
    let update_all = tip(
        button(
            container(
                row![
                    icon(Icon::Retry).size(px(14.0)),
                    label::body("Tümünü güncelle")
                ]
                .spacing(px(6.0))
                .align_y(Alignment::Center),
            )
            .center_y(Length::Fixed(metrics::control())),
        )
        .height(Length::Fixed(metrics::control()))
        .padding([0.0, px(10.0)])
        .style(style::quiet),
        Tip::new("Tümünü güncelle: denetim henüz yok").body(
            "Launcher sürümleri yalnız gösterir; güncellemeleri denetleyen bir hizmet henüz yok. Yeni sürümü kurulum paketiyle kurun.",
        ),
        Position::Left,
    );
    let said = row![
        column![
            label::strong("Güncelleme denetimi henüz yok"),
            label::muted("Launcher kurulu sürümleri yalnız gösterir. Yeni bir sürümü kurulum paketiyle kurun."),
        ]
        .spacing(px(2.0))
        .width(Fill),
        update_all,
    ]
    .spacing(px(16.0))
    .align_y(Alignment::Center);
    column![
        container(said).padding([0.0, px(style::INSET)]),
        tables::section("Sürümler", None, installed(launcher)),
    ]
    .spacing(px(style::SECTION))
    .height(Fill)
    .into()
}

pub fn downloads(_launcher: &Launcher) -> Element<'_, Message> {
    container(
        EmptyState::new(Icon::Import, "İndirme yok")
            .description("Launcher bir şey indirmez; uygulamalar kurulum paketleriyle gelir.")
            .secondary("Kurulu sürümleri gör", Message::Page(Page::Updates)),
    )
    .width(Fill)
    .height(Fill)
    .into()
}

/// A group of the settings: its title, then its rows with hairlines between them.
fn group<'a>(
    title: &'a str,
    rows: impl IntoIterator<Item = Element<'a, Message>>,
) -> Element<'a, Message> {
    let mut list = Column::new();
    for (i, line) in rows.into_iter().enumerate() {
        if i > 0 {
            list = list.push(tables::rule());
        }
        list = list.push(line);
    }
    tables::section(title, None, list)
}

/// A setting's row: its name and note on the left, its control on the right.
fn setting<'a>(
    name: &'a str,
    note: String,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    container(
        row![
            column![label::body(name), label::caption(note)]
                .spacing(px(2.0))
                .width(Fill),
            control.into(),
        ]
        .spacing(px(16.0))
        .align_y(Alignment::Center),
    )
    .padding([px(10.0), px(style::INSET)])
    .into()
}

/// The text sizes a user can choose (KentOS UI's range).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Size(u8);

impl std::fmt::Display for Size {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub fn settings(launcher: &Launcher) -> Element<'_, Message> {
    let current = launcher.settings;
    let theme = Segmented::new(ThemeChoice::ALL, current.theme, Message::Theme).width(px(260.0));
    let (smallest, largest) = (
        *Typography::SIZES.start() as u8,
        *Typography::SIZES.end() as u8,
    );
    let sizes = Segmented::new(
        (smallest..=largest).map(Size),
        Size(current.text_size as u8),
        |Size(s)| Message::TextSize(s),
    )
    .width(px(360.0));
    let size_note = match Settings::size_name(current.text_size) {
        Some(name) => format!(
            "{} piksel ({name}). Bütün yazılar ve ölçüler onunla büyür.",
            current.text_size
        ),
        None => format!(
            "{} piksel. Bütün yazılar ve ölçüler onunla büyür.",
            current.text_size
        ),
    };
    let theme_note = match current.theme {
        ThemeChoice::System => format!(
            "Sistemin teması izlenir; şimdi {}.",
            if launcher.system_dark {
                "koyu"
            } else {
                "açık"
            }
        ),
        ThemeChoice::Light => "Kâğıt zeminli açık tema.".to_owned(),
        ThemeChoice::Dark => "KentOS'un grafit koyu teması.".to_owned(),
    };
    let look = [
        setting("Tema", theme_note, theme),
        setting("Yazı boyutu", size_note, sizes),
    ];

    let starting = [setting(
        "Uygulama başlayınca küçült",
        "Bir uygulama başlayınca launcher görev çubuğuna iner.".to_owned(),
        Switch::new(current.minimize_on_launch, Message::MinimizeOnLaunch),
    )];

    let folders = [
        folder(
            "Kendi uygulamalarınız",
            "Uygulama ekle buraya yazar; buraya konan manifestler okunur:",
            launcher.folders.user.as_deref(),
            true,
        ),
        folder(
            "Kurulumla gelenler",
            "Launcher'ın yanındaki klasör:",
            launcher.folders.beside.as_deref(),
            false,
        ),
    ];

    let about = [container(
        column![
            label::body(format!("KentOS Launcher {}", env!("CARGO_PKG_VERSION"))),
            label::caption(format!(
                "Uygulamalar {} biçimindeki manifestlerden okunur (docs/launcher/README.md).",
                crate::manifest::FORMAT
            )),
        ]
        .spacing(px(2.0)),
    )
    .padding([px(10.0), px(style::INSET)])
    .into()];

    scrollable(column![
        column![
            group("Görünüş", look),
            group("Başlatma", starting),
            group("Uygulama klasörleri", folders),
            group("Hakkında", about),
            Space::new().height(px(8.0)),
        ]
        .spacing(px(style::SECTION))
        .max_width(px(SETTINGS_MOST))
    ])
    .direction(kentos_ui::style::field::body_scrollbar())
    .style(style::scroller)
    .height(Fill)
    .into()
}

/// An app folder's row: where it is, and Klasörü aç (made when it is the
/// user's and not there yet; off with its reason when an installation's is not there).
fn folder<'a>(
    name: &'a str,
    note: &'a str,
    place: Option<&'a Path>,
    makes: bool,
) -> Element<'a, Message> {
    let shown = place.map_or_else(
        || "Bilinmiyor (HOME yok)".to_owned(),
        |p| p.display().to_string(),
    );
    let exists = place.is_some_and(Path::is_dir);
    let mut open = button(
        row![icon(Icon::Folder).size(px(14.0)), label::body("Klasörü aç")]
            .spacing(px(6.0))
            .align_y(Alignment::Center),
    )
    .padding(metrics::padding(metrics::control(), px(10.0)))
    .style(kentos_ui::style::button::secondary);
    if let Some(place) = place
        && (exists || makes)
    {
        open = open.on_press(Message::OpenFolder(place.to_path_buf()));
    }
    let open: Element<'a, Message> = if place.is_some() && !exists && !makes {
        tip(
            open,
            Tip::new("Klasör yok").body("Kurulum bu klasörü getirmemiş."),
            Position::Left,
        )
    } else if !exists && makes {
        tip(
            open,
            Tip::new("Klasör henüz yok").body("Açınca oluşturulur."),
            Position::Left,
        )
    } else {
        open.into()
    };
    setting(name, format!("{note} {shown}"), open)
}
