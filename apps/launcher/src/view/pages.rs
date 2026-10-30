//! Son dosyalar, Güncellemeler, İndirmeler and Ayarlar (docs/adr/0800 §5,
//! §6). The update and download pages say plainly that there is no update
//! service yet and list what is installed; nothing there pretends to work.

use std::path::Path;

use iced::widget::tooltip::Position;
use iced::widget::{Column, button, column, container, row, rule, scrollable};
use iced::{Alignment, Element, Fill};
use kentos_launch::Module;
use kentos_ui::attribute::text as words;
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::theme::typography::Typography;
use kentos_ui::theme::{metrics, typography};
use kentos_ui::widget::{EmptyState, Segmented, Switch, Tip, tip};

use crate::app::{Launcher, Message};
use crate::cards::Status;
use crate::glyphs::Glyph;
use crate::recent::Row;
use crate::resolve::Found;
use crate::settings::{Settings, ThemeChoice};
use crate::tones::Tone;
use crate::view::style::{self, px};
use crate::view::{app_tile, tile};

/// A module's tile on the recent page.
fn module_tile<'a>(module: Option<Module>, side: f32) -> Element<'a, Message> {
    match module {
        Some(Module::Architecture) => tile(Glyph::Mimari, Tone::Purple, side),
        Some(Module::Map) => tile(Glyph::Harita, Tone::Blue, side),
        Some(Module::Gis) => tile(Glyph::Cbs, Tone::Green, side),
        None => tile(Glyph::Klasor, Tone::Gray, side),
    }
}

/// A row's frame: a panel across the page.
fn panel<'a>(content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    container(content)
        .padding([px(6.0), px(6.0)])
        .width(Fill)
        .style(style::card(false))
        .into()
}

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
    let mut list = Column::new().spacing(px(2.0));
    for (index, row) in rows {
        list = list.push(recent_row(launcher, index, row));
    }
    // In a column: the scroll area would stretch the panel to its own height.
    scrollable(column![panel(list)])
        .direction(kentos_ui::style::field::body_scrollbar())
        .height(Fill)
        .into()
}

fn recent_row<'a>(launcher: &'a Launcher, index: usize, row: &'a Row) -> Element<'a, Message> {
    let side = px(36.0);
    let module = row.module.map_or("KentOS", Module::title);
    // A file no longer there is dimmed: its words in the disabled tone, its tile grey.
    let dim = row.missing;
    let faded = |theme: &iced::Theme| iced::widget::text::Style {
        color: Some(kentos_ui::theme::Tokens::of(theme).disabled()),
    };
    let mut name = row![if dim {
        label::strong(row.name.as_str()).style(faded)
    } else {
        label::strong(row.name.as_str())
    }]
    .spacing(px(8.0))
    .align_y(Alignment::Center);
    if dim {
        name = name.push(
            container(
                label::caption("Bulunamadı")
                    .font(typography::ui_strong())
                    .style(kentos_ui::style::text::warning),
            )
            .padding([0.0, px(6.0)]),
        );
    }
    let info = if row.info.is_empty() {
        module.to_owned()
    } else {
        format!("{module} · {}", row.info)
    };
    let caption = |words: String| {
        if dim {
            label::caption(words).style(faded)
        } else {
            label::caption(words)
        }
    };
    let mark = if dim {
        tile(
            match row.module {
                Some(Module::Architecture) => Glyph::Mimari,
                Some(Module::Map) => Glyph::Harita,
                Some(Module::Gis) => Glyph::Cbs,
                None => Glyph::Klasor,
            },
            Tone::Gray,
            side,
        )
    } else {
        module_tile(row.module, side)
    };
    let content = row![
        mark,
        column![name, caption(row.folder.clone())]
            .spacing(px(2.0))
            .width(Fill),
        column![caption(row.ago.clone()), caption(info)]
            .spacing(px(2.0))
            .align_x(Alignment::End),
    ]
    .spacing(px(12.0))
    .align_y(Alignment::Center);
    let spec = launcher.recent_spec(row);
    let mut button = button(content)
        .width(Fill)
        .padding([px(8.0), px(10.0)])
        .style(style::nav(false));
    if !dim && spec.is_ok() {
        button = button.on_press(Message::OpenRecent(index));
    }
    let hint = if dim {
        Tip::new("Dosya yerinde yok").body(format!(
            "{} taşınmış ya da silinmiş olabilir; KentOS'ta Aç ile yeni yerinden açın.",
            row.path.display()
        ))
    } else {
        match spec {
            Ok(spec) => {
                Tip::new(format!("{module} ile aç")).detail(command(&spec.program, &spec.args))
            }
            Err(why) => Tip::new("Açılamaz").body(why),
        }
    };
    tip(button, hint, Position::Bottom)
}

/// The command line a tip shows: the program's name and its arguments.
fn command(program: &Path, args: &[std::ffi::OsString]) -> String {
    let name = program.file_name().map_or_else(
        || program.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    );
    let mut line = name;
    for arg in args {
        let arg = arg.to_string_lossy();
        if arg.contains(' ') {
            line.push_str(&format!(" \"{arg}\""));
        } else {
            line.push(' ');
            line.push_str(&arg);
        }
    }
    line
}

/// The installed apps, their versions and where they run from (both
/// Güncellemeler and İndirmeler list them).
fn installed(launcher: &Launcher) -> Element<'_, Message> {
    let mut list = Column::new().spacing(px(2.0));
    for app in &launcher.registry.apps {
        let status = launcher.status(app);
        let (word, place) = match launcher.found.get(&app.id) {
            Some(Found::Program(spec)) => (status.word(), spec.program.display().to_string()),
            Some(Found::Url(url)) => (status.word(), url.clone()),
            Some(Found::Missing(_)) => (status.word(), "Programı bulunamadı".to_owned()),
            Some(Found::Soon) | None => (status.word(), "Henüz çıkmadı".to_owned()),
        };
        let version = app
            .version
            .as_deref()
            .map_or_else(|| "—".to_owned(), |v| format!("v{v}"));
        let tone = move |theme: &iced::Theme| {
            let t = kentos_ui::theme::Tokens::of(theme);
            iced::widget::text::Style {
                color: Some(match status {
                    Status::Installed => t.success,
                    Status::Running(_) => t.accent_hover,
                    Status::Missing => t.warning,
                    Status::Web => t.info,
                    Status::Soon => t.faint,
                }),
            }
        };
        list = list.push(
            container(
                row![
                    app_tile(launcher, app, px(32.0)),
                    column![label::strong(app.name.as_str()), label::caption(place)]
                        .spacing(px(2.0))
                        .width(Fill),
                    label::body(version).width(px(90.0)),
                    label::caption(word)
                        .font(typography::ui_strong())
                        .style(tone)
                        .width(px(110.0)),
                ]
                .spacing(px(12.0))
                .align_y(Alignment::Center),
            )
            .padding([px(8.0), px(10.0)]),
        );
    }
    panel(list)
}

pub fn updates(launcher: &Launcher) -> Element<'_, Message> {
    scrollable(
        column![
            container(
                EmptyState::new(Icon::Retry, "Güncelleme denetimi henüz yok").description(
                    "KentOS Launcher şimdilik kurulu sürümleri yalnız gösterir: güncellemeleri denetleyen bir hizmet yok. Yeni bir sürümü kurulum paketiyle kurun.",
                ),
            )
            .width(Fill)
            .padding([px(8.0), 0.0]),
            installed(launcher),
        ]
        .spacing(px(style::GAP)),
    )
    .direction(kentos_ui::style::field::body_scrollbar())
    .height(Fill)
    .into()
}

pub fn downloads(launcher: &Launcher) -> Element<'_, Message> {
    scrollable(
        column![
            container(
                EmptyState::new(Icon::Import, "İndirme yok").description(
                    "Launcher bir şey indirmez; uygulamalar kurulum paketleriyle gelir. Kurulu olanlar aşağıda.",
                ),
            )
            .width(Fill)
            .padding([px(8.0), 0.0]),
            installed(launcher),
        ]
        .spacing(px(style::GAP)),
    )
    .direction(kentos_ui::style::field::body_scrollbar())
    .height(Fill)
    .into()
}

/// A group of the settings: its title before a hairline, then its rows.
fn group<'a>(title: &'a str, content: impl Into<Element<'a, Message>>) -> Element<'a, Message> {
    column![
        row![
            label::strong(title),
            container(rule::horizontal(1).style(kentos_ui::style::field::hairline)).width(Fill),
        ]
        .spacing(px(10.0))
        .align_y(Alignment::Center),
        content.into(),
    ]
    .spacing(px(12.0))
    .into()
}

/// A setting's row: its name and note on the left, its control on the right.
fn setting<'a>(
    name: &'a str,
    note: String,
    control: impl Into<Element<'a, Message>>,
) -> Element<'a, Message> {
    row![
        column![label::body(name), label::caption(note)]
            .spacing(px(2.0))
            .width(Fill),
        control.into(),
    ]
    .spacing(px(16.0))
    .align_y(Alignment::Center)
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
            "{} piksel · {name}. Bütün yazılar ve ölçüler onunla büyür.",
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
    let look = column![
        setting("Tema", theme_note, theme),
        setting("Yazı boyutu", size_note, sizes),
    ]
    .spacing(px(14.0));

    let starting = setting(
        "Uygulama başlayınca küçült",
        "Bir uygulama başlayınca launcher görev çubuğuna iner.".to_owned(),
        Switch::new(current.minimize_on_launch, Message::MinimizeOnLaunch),
    );

    let folders = column![
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
    ]
    .spacing(px(14.0));

    let about = column![
        label::body(format!("KentOS Launcher {}", env!("CARGO_PKG_VERSION"))),
        label::caption(format!(
            "Uygulamalar {} biçimindeki manifestlerden okunur (docs/launcher/README.md).",
            crate::manifest::FORMAT
        )),
    ]
    .spacing(px(4.0));

    scrollable(column![
        container(
            column![
                group("Görünüş", look),
                group("Başlatma", starting),
                group("Uygulama klasörleri", folders),
                group("Hakkında", about),
            ]
            .spacing(px(24.0)),
        )
        .padding([px(18.0), px(20.0)])
        .width(Fill)
        .max_width(px(900.0))
        .style(style::card(false))
    ])
    .direction(kentos_ui::style::field::body_scrollbar())
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
