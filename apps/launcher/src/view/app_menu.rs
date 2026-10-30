//! A plate's menu (⋯, docs/adr/0800 §4): what else an app can do, each thing
//! that cannot be done off with its reason.

use iced::widget::container;
use iced::{Element, Length};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::widget::{Menu, MenuButton};

use crate::app::{Launcher, Message};
use crate::cards::Status;
use crate::manifest::{Launch, Manifest, State};
use crate::view::style::px;

/// The plate's menu (⋯): quiet until the plate is the one under the pointer.
pub fn more<'a>(
    launcher: &'a Launcher,
    app: &'a Manifest,
    side: f32,
    hot: bool,
) -> Element<'a, Message> {
    let id = app.id.clone();
    let status = launcher.status(app);
    let users = app.origin.is_users();
    let placed = launcher.locations.get(&app.id).is_some();
    let locatable = matches!(app.launch, Some(Launch::Program { .. } | Launch::Kentos(_)));
    let opens_files =
        status.starts() && !app.files.is_empty() && !matches!(app.launch, Some(Launch::Url(_)));
    let from_file = app.origin.base().is_some();
    let soon = app.state == State::Soon;
    let tone = if hot { Tone::Text } else { Tone::Muted };
    MenuButton::new(
        container(icon(Icon::More).size(px(16.0)).tone(tone))
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
