//! Başlangıç (the web's `ui/start/StartScreen.ts`): what to do first, as
//! AutoCAD's and Netcad's start pages offer it: a new project, a drawing
//! file, a cloud project, a DXF, the recent files, or on with the drawing on
//! screen. It opens with the program when the preference says so
//! (`appearance.startScreen`), unless a drawing was named on the command
//! line or a crash's work is offered first, and from Dosya → Başlangıç
//! ekranı. The recent files open from here and from the application menu
//! (app_menu.rs) the same way: the drawing on screen is left first.

use std::path::PathBuf;

use iced::widget::{Column, button, column, container, row, scrollable, text};
use iced::{Center, Element, Fill, Length, Task};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::typography;
use kentos_ui::widget::{Dialog, Switch, overlay};
use kentos_ui::{label, style};
use serde_json::Value;

use crate::app::{App, Dialog as Asking, Message, Then};
use crate::catalog::catalog;
use crate::opening::Purpose;
use crate::recent::Recent;

#[derive(Debug, Clone)]
pub enum Event {
    /// One of the actions: the screen closes, then its command runs.
    Run(&'static str),
    /// Çizime devam et.
    Keep,
    /// A recent file.
    Open(PathBuf),
    /// × beside a recent file.
    Forget(PathBuf),
    /// Açılışta göster.
    Show(bool),
}

fn event(e: Event) -> Message {
    Message::Start(e)
}

impl App {
    pub(crate) fn open_start(&mut self) {
        self.dialog = Some(Asking::Start);
    }

    /// The program has just opened without a drawing named: the start
    /// screen, if the preference wants it and nothing else is asked first
    /// (a crash's work waits in the recovery window).
    pub fn start_at_launch(&mut self) {
        if self.dialog.is_none() && self.settings.bool("appearance.startScreen") {
            self.open_start();
        }
    }

    fn close_start(&mut self) {
        if self.dialog == Some(Asking::Start) {
            self.dialog = None;
        }
    }

    pub(crate) fn start_event(&mut self, e: Event) -> Task<Message> {
        match e {
            Event::Run(id) => {
                self.close_start();
                return self.run(id);
            }
            Event::Keep => self.close_start(),
            Event::Open(path) => {
                self.close_start();
                return self.open_recent(path);
            }
            Event::Forget(path) => self.recent.remove(&path),
            Event::Show(on) => {
                let refused = self
                    .settings
                    .choose(&[("appearance.startScreen", Value::Bool(on))]);
                if refused.is_empty() {
                    self.apply_settings();
                } else {
                    self.warn("Başlangıç ekranı tercihi kaydedilemedi; Uygulama ayarları → Görünüm'den deneyin.");
                }
            }
        }
        Task::none()
    }

    /// A recent file, from the start screen or the application menu: the
    /// drawing on screen is left first (its unsaved work asked about), then
    /// the file opens. A file that is gone leaves the list, and says so.
    pub(crate) fn open_recent(&mut self, path: PathBuf) -> Task<Message> {
        if !path.is_file() {
            self.warn(format!(
                "“{}” bulunamadı; taşınmış ya da silinmiş olabilir. Son dosyalardan kaldırıldı.",
                path.display()
            ));
            self.recent.remove(&path);
            return Task::none();
        }
        self.opening_recent = Some(path);
        self.leave(Then::OpenRecent)
    }

    /// The drawing on screen is left: the recent file opens.
    pub(crate) fn open_recent_ready(&mut self) -> Task<Message> {
        match self.opening_recent.take() {
            Some(path) => self.start_opening(path, Purpose::File),
            None => Task::none(),
        }
    }

    /// Puts the drawing's file first in the recent files, with what it holds now.
    pub(crate) fn remember_file(&mut self) {
        let Some(doc) = &self.document else { return };
        let Some(path) = doc.path.clone() else { return };
        let system = crate::crs::system(doc.settings().srid)
            .map_or(format!("EPSG:{}", doc.settings().srid), |s| s.name.clone());
        let info = format!(
            "{} nesne · {system}",
            crate::crs::grouped(doc.entity_count() as f64)
        );
        self.recent.add(&path, info);
    }

    pub(crate) fn start_view(&self) -> Element<'_, Message> {
        let action = |id: &'static str, glyph: Icon, title: &'static str, sub: String| {
            let chord = catalog()
                .get(id)
                .and_then(|c| c.shortcuts.first().copied())
                .unwrap_or("");
            let face = row![
                container(icon(glyph).size(22.0).tone(Tone::Accent))
                    .center(36)
                    .style(style::container::tile),
                column![label::heading(title), label::caption(sub)]
                    .spacing(2)
                    .width(Fill),
                label::mono_caption(chord),
            ]
            .spacing(12)
            .align_y(Center);
            button(face)
                .on_press_maybe(self.available(id).then(|| event(Event::Run(id))))
                .padding([8, 10])
                .width(Fill)
                .style(style::button::list_item(false))
        };
        let cloud_sub = match (&self.cloud.me, self.cloud.link) {
            (Some(me), crate::cloud::copy::Link::Online) => {
                format!("{} olarak giriş yapıldı", me.user.display_name)
            }
            (Some(_), crate::cloud::copy::Link::Offline) => {
                "Sunucuya ulaşılamıyor; bu cihazdaki projeler açılır".to_owned()
            }
            (None, _) => "Kurumunuzun projeleri; önce giriş yapılır".to_owned(),
        };
        let mut actions = Column::new()
            .spacing(4)
            .push(action(
                "file.new",
                Icon::DocumentNew,
                "Yeni proje",
                "Çalışma modu, koordinat sistemi ve ölçekle boş çizim".to_owned(),
            ))
            .push(action(
                "file.open",
                Icon::Open,
                "Dosya aç",
                "Bu bilgisayardaki bir .kcad çizimi".to_owned(),
            ))
            .push(action(
                "cloud.open",
                Icon::Globe,
                "Bulut projesi aç",
                cloud_sub,
            ))
            .push(action(
                "file.import.dxf",
                Icon::Import,
                "DXF içe aktar",
                "AutoCAD ve Netcad çizimleri, katmanlarıyla".to_owned(),
            ));
        if let Some(doc) = &self.document {
            let keep = row![
                container(icon(Icon::Play).size(22.0).tone(Tone::Muted))
                    .center(36)
                    .style(style::container::tile),
                column![
                    label::heading("Çizime devam et"),
                    label::caption(format!(
                        "{} · {} nesne",
                        doc.name(),
                        crate::crs::grouped(doc.entity_count() as f64)
                    )),
                ]
                .spacing(2),
            ]
            .spacing(12)
            .align_y(Center);
            actions = actions.push(
                button(keep)
                    .on_press(event(Event::Keep))
                    .padding([8, 10])
                    .width(Fill)
                    .style(style::button::list_item(false)),
            );
        }
        let brand = row![
            container(kentos_ui::widget::ribbon::logo_mark())
                .center(44)
                .style(style::container::accent),
            column![
                row![
                    text("KentOS ")
                        .font(typography::ui_strong())
                        .size(typography::title()),
                    text("CAD")
                        .size(typography::title())
                        .style(style::text::muted),
                ],
                label::caption("Harita, kadastro ve kent bilgi sistemi çizimi"),
            ]
            .spacing(2),
        ]
        .spacing(12)
        .align_y(Center);

        let mut list = Column::new().spacing(2);
        if self.recent.list().is_empty() {
            list = list.push(label::muted(
                "Henüz dosya yok. Açtığınız ya da kaydettiğiniz çizimler burada görünür; tek tıkla yeniden açılır.",
            ));
        }
        for r in self.recent.list() {
            list = list.push(recent_row(
                r,
                event(Event::Open(r.path.clone())),
                event(Event::Forget(r.path.clone())),
                false,
            ));
        }
        let side = column![
            text("Son dosyalar").font(typography::ui_strong()),
            scrollable(list)
                .direction(style::field::body_scrollbar())
                .height(Length::Fixed(typography::scaled(330.0))),
        ]
        .spacing(8);

        let shown = self.settings.bool("appearance.startScreen");
        let footer = row![
            Switch::new(shown, |on| event(Event::Show(on))).label("Açılışta göster"),
            label::caption("Dosya → Başlangıç ekranı ile yeniden açılır."),
        ]
        .spacing(16)
        .align_y(Center);
        overlay::modal(
            Dialog::new("Başlangıç")
                .push(
                    column![
                        row![
                            container(column![brand, actions].spacing(16))
                                .width(Length::FillPortion(3)),
                            container(side).width(Length::FillPortion(2)),
                        ]
                        .spacing(24),
                        footer,
                    ]
                    .spacing(16),
                )
                .width(880.0),
            event(Event::Keep),
        )
    }
}

/// One recent file: its name, what it held and when; a click opens it
/// again, × forgets it (the web's `recentFileRow`). `current`: the keyboard is on it.
pub(crate) fn recent_row(
    r: &Recent,
    open: Message,
    forget: Message,
    current: bool,
) -> Element<'_, Message> {
    let name = r.name.strip_suffix(".kcad").unwrap_or(&r.name).to_owned();
    let face = button(
        row![
            icon(Icon::Open).size(16.0).tone(Tone::Muted),
            column![
                label::body(name),
                label::caption(format!(
                    "{} · {}",
                    r.info,
                    crate::cloud::words::ago_ms(r.at)
                )),
            ]
            .spacing(1)
            .width(Fill),
        ]
        .spacing(8)
        .align_y(Center),
    )
    .on_press(open)
    .padding([6, 8])
    .width(Fill)
    .style(style::button::list_item(current));
    let forget = button(icon(Icon::Close).size(12.0).tone(Tone::Muted))
        .on_press(forget)
        .padding(6)
        .style(style::button::ghost);
    row![face, forget].spacing(2).align_y(Center).into()
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::Event;
    use crate::app::{App, Dialog, Message};
    use crate::files_testing::app_with_drawing;

    fn start(app: &mut App, e: Event) {
        let _ = app.update(Message::Start(e));
    }

    #[test]
    fn it_opens_at_launch_unless_turned_off_or_a_recovery_copy_waits() {
        let (mut app, _) = App::boot(None);
        app.start_at_launch();
        assert_eq!(
            app.dialog,
            Some(Dialog::Start),
            "on by default, as on the web"
        );
        let (mut app, _) = App::boot(None);
        let _ = app
            .settings
            .choose(&[("appearance.startScreen", Value::Bool(false))]);
        app.start_at_launch();
        assert_eq!(app.dialog, None);
        // A crash's work is offered first; the start screen does not cover it.
        let (mut app, _) = App::boot(None);
        app.dialog = Some(Dialog::Recovery);
        app.start_at_launch();
        assert_eq!(app.dialog, Some(Dialog::Recovery));
    }

    #[test]
    fn its_actions_run_and_it_closes_to_the_drawing() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run("file.start"));
        assert_eq!(app.dialog, Some(Dialog::Start), "Dosya → Başlangıç ekranı");
        start(&mut app, Event::Run("file.new"));
        assert_eq!(app.dialog, Some(Dialog::Project), "Yeni proje's window");
        app.open_start();
        start(&mut app, Event::Keep);
        assert_eq!(app.dialog, None, "Çizime devam et");
        app.open_start();
        let _ = app.update(Message::Key(crate::keys::KeyPress {
            key: iced::keyboard::Key::Named(iced::keyboard::key::Named::Escape),
            physical: iced::keyboard::key::Physical::Unidentified(
                iced::keyboard::key::NativeCode::Unidentified,
            ),
            modifiers: iced::keyboard::Modifiers::default(),
            text: None,
            repeat: false,
        }));
        assert_eq!(app.dialog, None, "Esc");
    }

    #[test]
    fn acilista_goster_is_the_setting() {
        let mut app = app_with_drawing();
        app.open_start();
        start(&mut app, Event::Show(false));
        assert!(!app.settings.bool("appearance.startScreen"));
        assert_eq!(app.dialog, Some(Dialog::Start), "the screen stays open");
        start(&mut app, Event::Show(true));
        assert!(app.settings.bool("appearance.startScreen"));
    }

    #[test]
    fn forgetting_a_file_takes_it_off_the_list() {
        let mut app = app_with_drawing();
        let path = std::path::PathBuf::from("/yok/Ada 12.kcad");
        app.recent.add(&path, "3 nesne · TUREF / TM36".into());
        app.open_start();
        start(&mut app, Event::Forget(path));
        assert!(app.recent.list().is_empty());
        assert_eq!(app.dialog, Some(Dialog::Start));
    }
}
