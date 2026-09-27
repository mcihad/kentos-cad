//! Nokta hesapla on the desktop (docs/adr/0083): the web's `calcMenu.ts`,
//! the command line's alias and chip (`CommandLine.ts`) and the strip's
//! button (`CommandBar.ts`). While a command waits for a point, one of
//! kentos_interaction's six constructions runs over it
//! (`kentos_interaction::point_calc`). It starts from:
//!
//! - its alias typed on the command line (YAN, KKES, DKES, HAT, AM, ORTA);
//! - Nokta hesapla ▸ in the drawing's command menu (the right button held);
//! - the command line's Nokta hesabı chip while the strip over the drawing
//!   is off, or the strip's button while it is on.

use iced::Task;
use kentos_interaction::point_calc::{
    CALC_KINDS, CalcDef, CalcKind, MENU_HEADER, NOT_NOW, PointCalc,
};
use kentos_ui::widget::Menu;

use crate::app::{App, Message};

/// The chip's and the strip button's name (the web's).
pub const CHIP: &str = "Nokta hesabı";

/// The calculator's rows (the web's `calcMenuItems`): its heading, then each
/// construction with its icon, what it computes and its alias.
pub fn calc_menu() -> Menu<Message> {
    CALC_KINDS
        .iter()
        .fold(Menu::new().header(MENU_HEADER), |menu, def| {
            menu.item(def.label, Message::PointCalc(def.kind))
                .icon(crate::icons::from_web(Some(def.icon)))
                .detail(def.description)
                .shortcut(def.alias)
        })
}

impl App {
    /// Starts a construction over the running command, or says why not
    /// when no command waits for a point (the web's `startPointCalc`).
    pub(crate) fn start_point_calc(&mut self, kind: CalcKind) -> Task<Message> {
        if !self.session.can_calc_point() {
            self.warn(NOT_NOW);
            return Task::none();
        }
        self.field = None;
        let def = CalcDef::of(kind);
        self.with_tool(|s, cx| s.nest(Box::new(PointCalc::new(kind)), def.started(), cx));
        Task::none()
    }
}

/// Every text of a view and where it is laid out (tests and pictures).
#[cfg(test)]
pub(crate) fn texts(
    snapshot: &mut kentos_ui::snapshot::Snapshot,
    app: &App,
) -> Vec<(String, iced::Rectangle)> {
    use std::sync::{Arc, Mutex};

    use iced::advanced::widget::{Id, Operation};

    struct Texts(Arc<Mutex<Vec<(String, iced::Rectangle)>>>);

    impl Operation for Texts {
        fn traverse(&mut self, operate: &mut dyn FnMut(&mut dyn Operation)) {
            operate(self);
        }

        fn text(&mut self, _id: Option<&Id>, bounds: iced::Rectangle, text: &str) {
            if let Ok(mut found) = self.0.lock() {
                found.push((text.to_owned(), bounds));
            }
        }
    }

    let found = Arc::new(Mutex::new(Vec::new()));
    snapshot.operate(app.view(), Box::new(Texts(found.clone())));
    found.lock().map(|t| t.clone()).unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use iced::Size;
    use iced::keyboard::key::Named;
    use kentos_interaction::point_calc::CalcKind;
    use kentos_ui::snapshot::{Input, Snapshot};
    use crate::app::{App, Message};
    use crate::files_testing::app_with_drawing;

    fn said(app: &crate::app::App) -> Vec<String> {
        app.log.lines().map(|l| l.text.clone()).collect()
    }

    /// Only over a command waiting for a point; then the command waits
    /// under it, and Esc brings it back.
    #[test]
    fn the_calculator_runs_over_a_command_waiting_for_a_point() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::PointCalc(CalcKind::Mid));
        assert_eq!(
            said(&app).last().map(String::as_str),
            Some("Nokta hesabı, nokta bekleyen bir komut sırasında kullanılır.")
        );
        let _ = app.update(Message::Run("tool.line"));
        let _ = app.update(Message::PointCalc(CalcKind::Mid));
        assert!(app.session.nested());
        assert_eq!(app.session.tool_id(), "line", "the command stays");
        assert_eq!(
            app.session.prompt().text(),
            "İki nokta ortası: birinci nokta gösterin"
        );
        assert_eq!(
            said(&app).last().map(String::as_str),
            Some("Nokta hesabı: İki nokta ortası")
        );
        // Not over itself.
        assert!(!app.session.can_calc_point());
        let _ = app.update(Message::Run("tool.cancel"));
        assert!(!app.session.nested());
        assert_eq!(app.session.prompt().text(), "Çizgi: ilk noktayı belirtin");
    }

    /// The command line's chip opens the calculator's menu above it, and a
    /// row of the menu starts that construction.
    #[test]
    fn the_command_line_chip_opens_the_menu_and_its_rows_start_the_calculator() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run("tool.line"));
        let mut update = |app: &mut App, message| {
            let _ = app.update(message);
        };
        let mut snapshot = Snapshot::software(Size::new(1440.0, 900.0)).expect("a renderer");
        snapshot.settle(&mut app, App::view, &mut update);
        let find = |texts: &[(String, iced::Rectangle)], text: &str| {
            texts
                .iter()
                .find(|(t, _)| t == text)
                .map(|(_, b)| *b)
                .unwrap_or_else(|| panic!("“{text}” is shown: {texts:?}"))
        };
        let chip = find(&super::texts(&mut snapshot, &app), super::CHIP);
        snapshot.input(
            &mut app,
            App::view,
            &mut update,
            Input::Click(chip.center()),
        );
        snapshot.settle(&mut app, App::view, &mut update);
        // The menu is an overlay, out of the operation's reach: its keys choose
        // the second construction (the first row is its heading).
        for key in [Named::ArrowDown, Named::ArrowDown, Named::Enter] {
            snapshot.input(&mut app, App::view, &mut update, Input::Key(key));
        }
        assert!(app.session.nested(), "{:?}", app.session.prompt());
        assert_eq!(
            app.session.prompt().text(),
            "Kenar kesişimi: birinci nokta (A) gösterin"
        );
    }
}

/// The calculator over Çizgi, for the owner: Kenar kesişimi with its two
/// solutions marked, Yan nokta with its readings beside the cursor, the
/// command menu with Nokta hesapla open, and the command line's chip. Not
/// run by default:
/// `cargo test -p kentos-desktop point_calc::screens -- --ignored --nocapture`.
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::{Point, Size};
    use kentos_interaction::Vec2;
    use kentos_ui::snapshot::{Input, Snapshot};

    use crate::viewport::Event;

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["kkes", "yan", "menu", "cip", "serit"] {
                let mut app = crate::files_testing::app_with_drawing();
                let _ = app
                    .settings
                    .choose(&[("appearance.theme", serde_json::Value::from(mode))]);
                app.apply_settings();
                let mut snapshot = Snapshot::new(Size::new(width, height)).expect("a renderer");
                let mut update = |app: &mut App, message| {
                    let _ = app.update(message);
                };
                snapshot.settle(&mut app, App::view, &mut update);
                let camera = app.viewport.camera;
                // Screen points, from the drawing area's centre, in pixels.
                let at = |dx: f64, dy: f64| {
                    let [x, y] = [camera.width / 2.0 + dx, camera.height / 2.0 + dy];
                    (Point::new(x as f32, y as f32), camera.screen_to_world(x, y))
                };
                let click = |app: &mut App, (p, _): (Point, Vec2)| {
                    for event in [Event::Moved(p), Event::Pressed(p), Event::Released(p)] {
                        let _ = app.update(Message::Viewport(event));
                    }
                };
                // The strip over the drawing, on: its own button.
                app.command_bar = name == "serit";
                let _ = app.update(Message::Run("tool.line"));
                click(&mut app, at(-260.0, 120.0));
                match name {
                    "kkes" => {
                        let _ = app.update(Message::PointCalc(CalcKind::Distances));
                        let a = at(-60.0, 40.0);
                        let b = at(60.0, 40.0);
                        click(&mut app, a);
                        click(&mut app, b);
                        // 10 m from each at this scale: two solutions.
                        let d = (b.1.x - a.1.x) * 0.8;
                        let text = format!("{d:.3},{d:.3}");
                        app.with_tool(|s, cx| s.input(&text, cx));
                        let _ = app.update(Message::Viewport(Event::Moved(at(40.0, -60.0).0)));
                    }
                    "yan" => {
                        let _ = app.update(Message::PointCalc(CalcKind::Side));
                        click(&mut app, at(-120.0, 60.0));
                        click(&mut app, at(80.0, 20.0));
                        let _ = app.update(Message::Viewport(Event::Moved(at(40.0, -50.0).0)));
                    }
                    "menu" => {
                        let (p, _) = at(-40.0, -140.0);
                        app.right_held(p);
                        snapshot.settle(&mut app, App::view, &mut update);
                        // Onto Nokta hesapla: the header, Onayla, İptal and a line above it.
                        let area = app.viewport.bounds;
                        let row = 4.0 + 26.0 + 28.0 + 28.0 + 9.0 + 14.0;
                        let over = Point::new(area.x + p.x + 2.0 + 60.0, area.y + p.y + 2.0 + row);
                        snapshot.input(&mut app, App::view, &mut update, Input::Move(over));
                    }
                    "serit" => {
                        let _ = app.update(Message::Viewport(Event::Moved(at(40.0, -60.0).0)));
                    }
                    _ => {
                        let _ = app.update(Message::Viewport(Event::Moved(at(40.0, -60.0).0)));
                        snapshot.settle(&mut app, App::view, &mut update);
                        // The chip clicked: its menu opens above it.
                        let chip = texts(&mut snapshot, &app)
                            .into_iter()
                            .find(|(t, _)| t == CHIP)
                            .map(|(_, b)| b)
                            .expect("the chip");
                        snapshot.input(
                            &mut app,
                            App::view,
                            &mut update,
                            Input::Click(chip.center()),
                        );
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("nokta-hesapla-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
