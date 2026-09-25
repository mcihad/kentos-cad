//! Uygulama menüsü: dosya komutları, son kullanılan çizimler ve dışa aktarma.

use iced::Element;

use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::app_menu::{Action, AppMenu, Entry, Item, Pane};
use kentos_ui::widget::badge;

use crate::app::Showcase;
use crate::message::{AppCommand, EXPORT_FORMATS, Message, RECENT_DRAWINGS};

impl Showcase {
    pub(super) fn app_menu(&self) -> Element<'_, Message> {
        let expanded = self.app_menu_hover.filter(|command| command.has_submenu());

        let menu = AppCommand::ALL.into_iter().fold(
            AppMenu::new(Message::AppMenuToggled),
            |menu, command| {
                let entry = Entry::new(command.icon(), command.label(), command.description())
                    .on_press(Message::AppCommandPressed(command))
                    .on_hover(Message::AppMenuHovered(command));

                menu.entry(if command.has_submenu() {
                    entry.submenu(expanded == Some(command))
                } else {
                    entry
                })
            },
        );

        let detail = match expanded {
            Some(AppCommand::Export) => export_pane(),
            _ => recent_pane(),
        };

        let theme_label = if self.mode.is_dark() {
            "Aydınlık tema"
        } else {
            "Koyu tema"
        };

        menu.detail(detail)
            .note("KentOS CAD 0.1")
            .action(Action::secondary(
                Icon::Contrast,
                theme_label,
                Message::ToggleTheme,
            ))
            .action(Action::primary(
                Icon::Power,
                "KentOS CAD'den çık",
                Message::Quit,
            ))
            .into()
    }
}

fn recent_pane<'a>() -> Element<'a, Message> {
    RECENT_DRAWINGS
        .iter()
        .enumerate()
        .fold(
            Pane::new("Son kullanılanlar", "Bir çizimi açmak için tıklayın."),
            |pane, (index, (name, folder, when))| {
                let current = index == 0;
                let when = label::caption(*when);

                pane.push(
                    Item::new(*name)
                        .subtitle(*folder)
                        .leading(icon(Icon::Document).size(20.0).tone(if current {
                            Tone::Accent
                        } else {
                            Tone::Muted
                        }))
                        .trailing(if current {
                            when.style(style::text::accent)
                        } else {
                            when
                        })
                        .on_press(Message::RecentPressed(index)),
                )
            },
        )
        .into()
}

fn export_pane<'a>() -> Element<'a, Message> {
    EXPORT_FORMATS
        .iter()
        .fold(
            Pane::new(
                "Dışa aktar",
                "Çizimin bir kopyasını başka bir biçimde kaydedin.",
            ),
            |pane, (format, description)| {
                pane.push(
                    Item::new(format!("{format} olarak dışa aktar"))
                        .subtitle(*description)
                        .leading(badge(format))
                        .on_press(Message::ExportPressed(format)),
                )
            },
        )
        .into()
}
