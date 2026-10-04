//! The project's second coordinate system on the desktop (docs/adr/0167
//! §1–§2, §5), as the web's (`ui/statusbar/secondMenu.ts`, the status bar's
//! `status__second`):
//!
//! - the status bar's cell: the system's short name and the cursor's values
//!   in it; its tip says how sure they are; a click opens İkinci sistem;
//! - İkinci sistem: Yok and the registry's systems but the project's own,
//!   grouped by datum, and how geographic values are written
//!   (`display.geographic`);
//! - the coordinate system cell's right-click menu: Koordinat sistemi… and
//!   İkinci sistem ▸.
//!
//! Choosing is a project setting, an edit of the drawing as Proje
//! ayarları's, never a transformation of its coordinates.

use iced::Element;
use iced::widget::row;
use kentos_interaction::second::{Notation, Second};
use kentos_interaction::{Format, Level, Vec2};
use kentos_ui::label;
use kentos_ui::widget::{Menu, MenuButton, Tip, tip};
use serde_json::Value;

use crate::app::{App, Message};
use crate::crs;

/// What İkinci sistem asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Event {
    /// The second system chosen; none: Yok.
    Choose(Option<u32>),
    /// How a geographic second system's values are written.
    Notation(Notation),
}

fn message(event: Event) -> Message {
    Message::SecondCrs(event)
}

/// The systems a project in `project` may take as its second, grouped by
/// datum as the registry lists them: every one but the local and its own.
pub(crate) fn choices(project: u32) -> Vec<(&'static str, Vec<&'static crs::System>)> {
    let mut groups: Vec<(&'static str, Vec<&'static crs::System>)> = Vec::new();
    for s in crs::systems() {
        if s.is_local() || s.srid == project {
            continue;
        }
        let datum = crs::datum_label(&s.datum);
        match groups.iter_mut().find(|(d, _)| *d == datum) {
            Some((_, list)) => list.push(s),
            None => groups.push((datum, vec![s])),
        }
    }
    groups
}

impl App {
    pub(crate) fn second_crs_event(&mut self, event: Event) {
        match event {
            Event::Choose(srid) => self.choose_second(srid),
            Event::Notation(notation) => {
                let value = match notation {
                    Notation::Dms => "dms",
                    Notation::Dd => "dd",
                };
                let _ = self
                    .settings
                    .choose(&[("display.geographic", Value::from(value))]);
                self.apply_settings();
            }
        }
    }

    /// The project's second system set, or taken away (Yok).
    fn choose_second(&mut self, srid: Option<u32>) {
        let Some(doc) = &mut self.document else {
            return;
        };
        let mut settings = doc.settings().clone();
        if settings.second_srid == srid {
            return;
        }
        settings.second_srid = srid;
        doc.model.set_settings(settings);
        self.say(
            Level::Success,
            match srid {
                None => "İkinci koordinat sistemi kaldırıldı.".to_owned(),
                Some(srid) => format!(
                    "İkinci koordinat sistemi: {}. Çizim dönüştürülmedi.",
                    crs::title_of(srid)
                ),
            },
        );
    }

    /// İkinci sistem: Yok, a submenu of systems for each datum, then Coğrafi
    /// değerler (the web's `secondMenu`).
    pub(crate) fn second_menu(&self) -> Menu<Message> {
        let Some(doc) = &self.document else {
            return Menu::new();
        };
        let settings = doc.settings();
        if settings.srid == crs::LOCAL_SRID {
            return Menu::new().item("Yerel projenin ikinci sistemi olmaz", None);
        }
        let current = settings.second();
        let mut menu = Menu::new().header("İkinci koordinat sistemi").radio(
            "Yok",
            current.is_none(),
            message(Event::Choose(None)),
        );
        // A datum each, the one chosen beside its name: the list stays short, Coğrafi
        // değerler in view.
        for (datum, systems) in choices(settings.srid) {
            let chosen = systems
                .iter()
                .find(|s| current == Some(s.srid))
                .map(|s| s.name.clone());
            let list = systems.iter().fold(Menu::new(), |list, s| {
                list.radio(
                    s.name.clone(),
                    current == Some(s.srid),
                    message(Event::Choose(Some(s.srid))),
                )
                .hint(format!("EPSG:{}", s.srid))
            });
            menu = menu.submenu(datum, list);
            if let Some(name) = chosen {
                menu = menu.hint(name);
            }
        }
        let notation = self.draft.geographic;
        menu.separator()
            .header("Coğrafi değerler")
            .radio(
                "Derece, dakika, saniye",
                notation == Notation::Dms,
                message(Event::Notation(Notation::Dms)),
            )
            .hint("40°45′12.3456″K")
            .radio(
                "Ondalık derece",
                notation == Notation::Dd,
                message(Event::Notation(Notation::Dd)),
            )
            .hint("40.7534293°K")
    }

    /// The coordinate system cell's right-click menu: Koordinat sistemi…
    /// and İkinci sistem ▸.
    pub(crate) fn crs_cell_menu(&self) -> Menu<Message> {
        let set = crate::catalog::catalog().get("crs.set");
        let mut menu = Menu::new().item(
            set.map_or("Koordinat sistemi…", |c| c.title),
            Message::Run("crs.set"),
        );
        if let Some(c) = set {
            menu = menu.icon(c.icon);
        }
        menu.submenu("İkinci sistem", self.second_menu())
            .icon(crate::icons::from_web(Some("crsSecond")))
    }

    /// The second system's values at `cursor`, as the status bar writes
    /// them: “Y 414154.869   X 4540584.350”, “40°59′38.0581″K
    /// 28°58′45.7302″D”; dashes without a cursor or where the system does
    /// not reach.
    pub(crate) fn second_values(&self, second: &Second, cursor: Option<Vec2>) -> String {
        let Some(doc) = &self.document else {
            return String::new();
        };
        let f = Format::of(doc.settings());
        let point = cursor.and_then(|p| second.point(p));
        match point {
            Some(t) => {
                let [a, b] = second.values(t.point, &f, self.draft.geographic);
                if second.geographic() {
                    format!("{}  {}", a.1, b.1)
                } else {
                    format!("{} {}   {} {}", a.0, a.1, b.0, b.1)
                }
            }
            None if second.geographic() => "—  —".to_owned(),
            None => format!("{} —   {} —", f.east_label(), f.north_label()),
        }
    }

    /// The status bar's cell of the second system, when the project has one:
    /// its name and the cursor's values; a click lists İkinci sistem, the
    /// tip says how sure the values are (§5).
    pub(crate) fn second_cell(&self, cursor: Option<Vec2>) -> Option<Element<'_, Message>> {
        let second = Second::of(self.document.as_ref()?.settings())?;
        let face = row![
            label::muted(second.short()),
            label::mono(self.second_values(&second, cursor)),
        ]
        .spacing(8)
        .align_y(iced::Center);
        let sure = match cursor.map(|p| second.point(p)) {
            Some(Some(t)) => format!("{}.", second.accuracy(&t)),
            Some(None) => "İmleç bu sistemin ulaştığı yerin dışında; değer yazılmadı.".to_owned(),
            None => String::new(),
        };
        let body = format!(
            "{} değerleri, projeninkilerden dönüştürülerek. {sure} Sistemi değiştirmek ya da kaldırmak için tıklayın.",
            crs::title(second.system)
        );
        Some(tip(
            MenuButton::new(iced::widget::container(face).padding([0, 8]), move || {
                self.second_menu()
            }),
            Tip::new(format!("İkinci koordinat sistemi: {}", second.system.name)).body(body),
            iced::widget::tooltip::Position::Top,
        ))
    }

    /// About how wide the status bar's cell is, its separator too: its name
    /// and its values (view.rs `status_width`), the drawing's origin's
    /// values, so it does not change as the cursor comes and goes; none
    /// without a second system.
    pub(crate) fn second_width(&self) -> f32 {
        let Some(doc) = &self.document else {
            return 0.0;
        };
        let Some(second) = Second::of(doc.settings()) else {
            return 0.0;
        };
        let size = kentos_ui::theme::typography::body();
        let origin = doc.model.origin();
        let values = self.second_values(&second, Some(Vec2::new(origin.x, origin.y)));
        // About 0.62 of the type size a letter, the name's and the figures' (measured on
        // the pictures), the gap, the padding and the separator.
        (second.short().chars().count() + values.chars().count()) as f32 * 0.62 * size
            + 8.0
            + 16.0
            + 9.0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::files_testing::{app_with_drawing, find_text};

    #[test]
    fn the_choices_are_every_system_but_the_local_and_the_projects_own() {
        let groups = choices(5254);
        let names: Vec<&str> = groups.iter().map(|(d, _)| *d).collect();
        assert_eq!(names, ["TUREF (ITRF96)", "ED50", "WGS 84"]);
        let all: Vec<u32> = groups
            .iter()
            .flat_map(|(_, list)| list.iter().map(|s| s.srid))
            .collect();
        assert!(!all.contains(&5254) && !all.contains(&0));
        assert_eq!(all.len(), crs::systems().len() - 2);
        assert!(all.contains(&2320) && all.contains(&4326));
    }

    #[test]
    fn choosing_sets_the_project_and_says_so() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::SecondCrs(Event::Choose(Some(2322))));
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.settings().second_srid, Some(2322));
        assert!(crate::files_testing::last_said(&app).contains("ED50 / TM36 (EPSG:2322)"));
        // The status bar's values: ED50 TM36's east and north.
        let second = Second::of(doc.settings()).expect("a second system");
        let text = app.second_values(&second, Some(Vec2::new(486_512.34, 4_420_187.52)));
        assert!(
            text.starts_with("Y 486") && text.contains("   X 4420"),
            "{text}"
        );
        assert_eq!(app.second_values(&second, None), "Y —   X —");
        let _ = app.update(Message::SecondCrs(Event::Notation(Notation::Dd)));
        assert_eq!(app.draft.geographic, Notation::Dd);
        let _ = app.update(Message::SecondCrs(Event::Choose(None)));
        let doc = app.document.as_ref().expect("a drawing");
        assert_eq!(doc.settings().second_srid, None);
        assert!(crate::files_testing::last_said(&app).contains("kaldırıldı"));
    }

    /// The second system in the status bar (ED50 TM36; WGS 84 in degrees,
    /// minutes and seconds), the coordinate system cell's menu with İkinci
    /// sistem open, Koordinat oku's tag and Proje ayarları' field, light and
    /// dark, and the bar at 1100 × 650; `.run/shots/ikinci-sistem-*`:
    ///
    /// ```text
    /// cargo test -p kentos-desktop second_crs::tests::screens -- --ignored --nocapture
    /// ```
    #[test]
    #[ignore = "pictures for the owner, run by hand"]
    fn screens() {
        use iced::{Point, Size};
        use kentos_ui::snapshot::{Input, Snapshot};

        let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
        std::fs::create_dir_all(&out).expect("a folder for the pictures");
        for (theme, w, h) in [
            ("light", 1440.0, 900.0),
            ("dark", 1440.0, 900.0),
            ("dark", 1100.0, 650.0),
        ] {
            let mut app = app_with_drawing();
            let _ = app
                .settings
                .choose(&[("appearance.theme", serde_json::Value::from(theme))]);
            app.apply_settings();
            let _ = app.update(Message::SecondCrs(Event::Choose(Some(2322))));
            let mut snapshot = Snapshot::new(Size::new(w, h)).expect("a renderer");
            let mut update = |app: &mut App, message| {
                let _ = app.update(message);
            };
            snapshot.settle(&mut app, App::view, &mut update);
            let tag = format!("{theme}-{w}");
            let save = |snapshot: &mut Snapshot, app: &App, name: &str| {
                let file = out.join(format!("ikinci-sistem-{name}-{tag}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            };
            let over = Point::new(w * 0.42, h * 0.48);
            snapshot.input(&mut app, App::view, &mut update, Input::Move(over));
            // The message of the choice takes the values' room while it shows.
            app.follow.show_fully();
            save(&mut snapshot, &app, "ileti");
            // Once it is gone, the values.
            app.follow.flash = None;
            snapshot.input(&mut app, App::view, &mut update, Input::Move(over));
            save(&mut snapshot, &app, "durum");
            if w < 1400.0 {
                continue;
            }
            // The coordinate system cell's right-click menu, İkinci sistem open.
            if let Some(cell) = find_text(&mut snapshot, &app, "TUREF / TM36") {
                snapshot.input(
                    &mut app,
                    App::view,
                    &mut update,
                    Input::RightClick(cell.center()),
                );
                // İkinci sistem, the menu's second row (a menu is an overlay, its words are not found).
                let item = Point::new(cell.center().x - 100.0, cell.center().y + 49.0);
                snapshot.input(&mut app, App::view, &mut update, Input::Move(item));
                save(&mut snapshot, &app, "menu");
                snapshot.input(
                    &mut app,
                    App::view,
                    &mut update,
                    Input::Key(iced::keyboard::key::Named::Escape),
                );
                snapshot.input(
                    &mut app,
                    App::view,
                    &mut update,
                    Input::Key(iced::keyboard::key::Named::Escape),
                );
            }
            // Koordinat oku: the reading said, its tag beside the cursor.
            let _ = app.update(Message::Run("crs.query"));
            snapshot.settle(&mut app, App::view, &mut update);
            snapshot.input(&mut app, App::view, &mut update, Input::Click(over));
            snapshot.input(&mut app, App::view, &mut update, Input::Move(over));
            app.follow.flash = None;
            snapshot.settle(&mut app, App::view, &mut update);
            save(&mut snapshot, &app, "koordinat-oku");
            let _ = app.update(Message::Run("tool.cancel"));
            // WGS 84 in degrees, minutes and seconds.
            let _ = app.update(Message::SecondCrs(Event::Choose(Some(4326))));
            app.follow.flash = None;
            snapshot.settle(&mut app, App::view, &mut update);
            snapshot.input(&mut app, App::view, &mut update, Input::Move(over));
            save(&mut snapshot, &app, "wgs84");
            // Proje ayarları' Koordinat sistemi: the field below the system's list.
            let _ = app.update(Message::Run("crs.set"));
            snapshot.settle(&mut app, App::view, &mut update);
            if let Some(list) = find_text(&mut snapshot, &app, "İkinci koordinat sistemi") {
                snapshot.input(
                    &mut app,
                    App::view,
                    &mut update,
                    Input::Scroll(list.center(), -6.0),
                );
            }
            save(&mut snapshot, &app, "proje-ayarlari");
        }
    }
}
