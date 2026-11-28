//! Kısayollar' form of the chosen command (docs/adr/9308): its keys with their ×, Tuş ekle, where the keys come from,
//! KentOS's names and the user's, Yeni ad, the banner of a key or a name another command has (Taşı, Vazgeç), the
//! programs that lack the command, Varsayılana dön.

use iced::widget::{Row, button, column, container, row, space};
use iced::{Center, Element};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::widget::{Banner, Form};
use kentos_ui::{label, style};

use super::{KeysEdit, KeysPage, NAME, Pending, message, programs_without, shown};
use crate::app::{App, Message};
use crate::catalog::{Command, catalog};
use crate::keymap::{Keymap, Source};

impl App {
    /// The chosen command's form.
    pub(super) fn keys_details<'a>(
        &self,
        map: &Keymap,
        page: &'a KeysPage,
        command: &'static Command,
    ) -> Element<'a, Message> {
        let keys: Vec<String> = map
            .keys_of(command)
            .iter()
            .map(|k| (*k).to_owned())
            .collect();
        let user_keys = map.keys_source(command.id) == Source::User;
        let names: Vec<String> = map
            .names_of(command.id)
            .iter()
            .map(|n| (*n).to_owned())
            .collect();
        let mut key_row = Row::new().spacing(6).align_y(Center);
        for key in &keys {
            key_row = key_row.push(removable(
                shown(key),
                message(KeysEdit::RemoveKey(key.clone())),
            ));
        }
        if keys.is_empty() {
            key_row = key_row.push(label::caption("Tuşu yok").style(style::text::muted));
        }
        let record = if page.recording {
            button(label::body("Tuşa basın… (Esc vazgeçer)"))
                .on_press(message(KeysEdit::StopRecording))
                .style(style::button::primary)
        } else {
            button(label::body("Tuş ekle"))
                .on_press(message(KeysEdit::Record))
                .style(style::button::secondary)
        };
        let key_row = key_row.push(record.padding(kentos_ui::theme::metrics::action()));
        let defaults: Vec<String> = command.shortcuts.iter().map(|k| shown(k)).collect();
        let lost: Vec<String> = map
            .lost_keys(command)
            .into_iter()
            .map(|(k, owner)| {
                format!(
                    "“{}” {}",
                    shown(k),
                    catalog().get(owner).map_or(owner, |c| c.title)
                )
            })
            .collect();
        let key_help = match (user_keys, defaults.is_empty()) {
            (true, true) => "Tuşları sizin verdiğiniz; KentOS'ta tuşu yoktu.".to_owned(),
            (true, false) => format!(
                "Tuşları sizin verdiğiniz; KentOS'un verdiği: {}.",
                defaults.join(", ")
            ),
            (false, _) if !lost.is_empty() => {
                format!(
                    "KentOS'un verdiği tuşlardan sizin başka komuta verdikleriniz: {}.",
                    lost.join(", ")
                )
            }
            (false, true) => "KentOS bu komuta tuş vermemiş; Tuş ekle ile verin.".to_owned(),
            (false, false) => "KentOS'un verdiği tuşlar.".to_owned(),
        };
        let mut form = Form::new()
            .label_width(110.0)
            .section("Kısayol")
            .field("Tuşlar", key_row)
            .help(key_help);
        if let Some(Pending::Key { chord, from }) = &page.pending {
            form = form.row(self.taken_banner(map, shown(chord), from, true));
        }
        let own_names = if command.aliases.is_empty() {
            "Yok".to_owned()
        } else {
            command.aliases.join(", ")
        };
        let mut name_row = Row::new().spacing(6).align_y(Center);
        for name in &names {
            name_row = name_row.push(removable(
                name.clone(),
                message(KeysEdit::RemoveName(name.clone())),
            ));
        }
        if names.is_empty() {
            name_row = name_row.push(label::caption("Yok").style(style::text::muted));
        }
        let field = kentos_ui::widget::text_field("Yeni ad (ör. ÇZ2)", &page.name)
            .id(NAME)
            .on_input(|t| message(KeysEdit::NameTyped(t)))
            .on_submit(message(KeysEdit::AddName))
            .width(170);
        let add = button(label::body("Ekle"))
            .on_press_maybe((!page.name.trim().is_empty()).then_some(message(KeysEdit::AddName)))
            .padding(kentos_ui::theme::metrics::action())
            .style(style::button::secondary);
        form = form
            .section("Takma ad")
            .field("KentOS'un adları", label::body(own_names))
            .help("Komut satırına yazılır; değiştirilemez.")
            .field("Sizin adlarınız", name_row)
            .field("Yeni ad", row![field, add].spacing(6).align_y(Center))
            .help("Harf ve rakam, harfle başlar. Komut satırına yazıp Enter'a basınca bu komut başlar.");
        if let Some(Pending::Name { name, from }) = &page.pending {
            form = form.row(self.taken_banner(map, name.clone(), from, false));
        }
        if let Some((severity, note)) = &page.note {
            form = form.row(Banner::new(*severity, note.clone()));
        }
        // A command a module does not offer: its keys and names do nothing in that module's program.
        if let Some(places) = programs_without(command.id) {
            form = form.row(
                label::caption(format!(
                    "Bu komut {places} yok: tuşu ve adı oralarda bir şey yapmaz, KentOS CAD'de ve öbür programlarda çalışır."
                ))
                .style(style::text::muted),
            );
        }
        let changed = user_keys || !names.is_empty();
        let reset = button(label::body("Varsayılana dön"))
            .on_press_maybe(changed.then_some(message(KeysEdit::CommandDefaults)))
            .padding(kentos_ui::theme::metrics::action())
            .style(style::button::secondary);
        column![
            label::heading(command.title),
            label::caption(command.description).style(style::text::muted),
            form.row(reset),
        ]
        .spacing(8)
        .padding(iced::Padding {
            right: 12.0,
            ..iced::Padding::ZERO
        })
        .into()
    }

    /// The banner of a key or a name another command has: whose, and Taşı or Vazgeç.
    fn taken_banner<'a>(
        &self,
        map: &Keymap,
        what: String,
        from: &'static str,
        key: bool,
    ) -> Element<'a, Message> {
        let Some(other) = catalog().get(from) else {
            return space::vertical().height(0).into();
        };
        let whose = match (key, map.keys_source(from)) {
            (true, Source::Default) => "KentOS'un verdiği kısayolu",
            (true, Source::User) => "sizin verdiğiniz kısayolu",
            (false, _) => "sizin verdiğiniz adı",
        };
        let here = if self.offers(from) {
            String::new()
        } else {
            format!(
                " {} {} yok; onun olduğu yerde geçerlidir.",
                other.title,
                self.here_words()
            )
        };
        Banner::warning(format!(
            "“{what}” şu an {} komutunun {whose}.{here} Bu komuta taşınırsa {} onu kullanamaz.",
            other.title, other.title
        ))
        .action("Taşı", message(KeysEdit::Move))
        .action("Vazgeç", message(KeysEdit::Keep))
        .into()
    }
}

/// Keys as caps, side by side.
pub(super) fn caps<'a>(keys: Vec<String>) -> Element<'a, Message> {
    keys.into_iter()
        .fold(Row::new().spacing(4), |row, key| {
            row.push(crate::shortcuts::kbd(key))
        })
        .into()
}

/// A key or a name with its × that takes it away.
fn removable<'a>(text: String, remove: Message) -> Element<'a, Message> {
    let close = button(icon(Icon::Close).size(11.0))
        .on_press(remove)
        .padding(2)
        .style(style::button::ghost);
    container(
        row![crate::shortcuts::kbd(text), close]
            .spacing(2)
            .align_y(Center),
    )
    .into()
}
