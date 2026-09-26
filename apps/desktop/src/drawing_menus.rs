//! The right button over the drawing (the web's `ui/shell/viewportMenus.ts`
//! and `ViewportController.onRightDown` / `onRightUp`):
//!
//! - a quick right click is Enter for a running command that takes a
//!   confirm; with no command, or one that takes none (Kaydır, Pencere
//!   yakınlaştır), it opens the idle menu;
//! - held for 300 ms, it opens the command menu while a command runs, the
//!   idle menu otherwise;
//! - Shift and the right button open the one-shot snap menu at once.
//!
//! A one-shot snap makes the next left click snap to its kind only, even
//! with running snaps off (F3). A left press or another command drops it;
//! the command line shows it with a × until then.
//!
//! The web's grip actions (a vertex or an edge under the cursor) wait for
//! grips on the desktop, and its Nokta hesapla submenu for the point
//! calculator (TODOS.md UX-07).

use iced::Point;
use kentos_interaction::SnapKind;
use kentos_ui::icon::Icon;
use kentos_ui::widget::Menu;

use crate::app::{App, Message};
use crate::catalog::{Standing, catalog};

/// The kinds a one-shot snap offers, in the order surveyors reach for them
/// (the web's `SNAP_ORDER`).
const SNAP_ORDER: [SnapKind; 9] = [
    SnapKind::Endpoint,
    SnapKind::Midpoint,
    SnapKind::Intersection,
    SnapKind::Center,
    SnapKind::Perpendicular,
    SnapKind::Tangent,
    SnapKind::Quadrant,
    SnapKind::Node,
    SnapKind::Nearest,
];

/// A snap kind's name (the web's `SNAP_LABEL`).
pub fn snap_label(kind: SnapKind) -> &'static str {
    match kind {
        SnapKind::Endpoint => "Uç nokta",
        SnapKind::Midpoint => "Orta nokta",
        SnapKind::Center => "Merkez",
        SnapKind::Node => "Nokta",
        SnapKind::Quadrant => "Çeyrek",
        SnapKind::Intersection => "Kesişim",
        SnapKind::Perpendicular => "Dik",
        SnapKind::Tangent => "Teğet",
        SnapKind::Nearest => "En yakın",
    }
}

/// The web's icon of a snap kind, drawn like its marker.
fn snap_icon(kind: SnapKind) -> Icon {
    crate::icons::from_web(Some(match kind {
        SnapKind::Endpoint => "snapEndpoint",
        SnapKind::Midpoint => "snapMidpoint",
        SnapKind::Center => "snapCenter",
        SnapKind::Node => "snapNode",
        SnapKind::Quadrant => "snapQuadrant",
        SnapKind::Intersection => "snapIntersection",
        SnapKind::Perpendicular => "snapPerpendicular",
        SnapKind::Tangent => "snapTangent",
        SnapKind::Nearest => "snapNearest",
    }))
}

/// Which menu is open over the drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// No command, or a quick click with one that takes no confirm.
    Idle,
    /// Held while a command runs.
    Command,
    /// Shift and the right button.
    Snap,
}

/// An open menu and where (logical pixels from the drawing area's top-left).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Open {
    pub kind: Kind,
    pub at: Point,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// The menu closed: a command chosen, a click outside, Esc.
    Closed,
    /// A one-shot snap chosen, or dropped (the command line's ×).
    SnapOnce(Option<SnapKind>),
}

impl App {
    /// The right button went down on the drawing; with Shift, the snap menu opens.
    pub(crate) fn right_pressed(&mut self, at: Point) {
        self.drawing_menu = self.modifiers.shift().then_some(Open {
            kind: Kind::Snap,
            at,
        });
    }

    /// The right button has been held (the drawing area times it, viewport.rs):
    /// the command menu while a command runs, the idle menu otherwise.
    pub(crate) fn right_held(&mut self, at: Point) {
        if self.drawing_menu.is_some() {
            return;
        }
        let kind = if self.session.is_running() {
            Kind::Command
        } else {
            Kind::Idle
        };
        self.drawing_menu = Some(Open { kind, at });
    }

    /// The right button came up sooner than the hold.
    pub(crate) fn right_clicked(&mut self, at: Point) {
        if self.drawing_menu.is_some() {
            return;
        }
        self.field = None;
        self.line_focused = false;
        // Enter for a running command that takes a confirm; the idle menu otherwise.
        if self.session.confirms() {
            self.with_tool(|s, cx| s.confirm(cx));
        } else {
            self.drawing_menu = Some(Open {
                kind: Kind::Idle,
                at,
            });
        }
    }

    pub(crate) fn drawing_menu_event(&mut self, event: Event) {
        match event {
            Event::Closed => self.drawing_menu = None,
            Event::SnapOnce(kind) => {
                self.snap_once = kind.map(|kind| (kind, self.session.tool_id()));
            }
        }
    }

    /// The one-shot snap, while the command it was chosen in runs.
    pub(crate) fn snap_once(&self) -> Option<SnapKind> {
        self.snap_once
            .filter(|(_, tool)| *tool == self.session.tool_id())
            .map(|(kind, _)| kind)
    }

    /// The open menu's items.
    pub(crate) fn drawing_menu_items(&self) -> Menu<Message> {
        match self.drawing_menu.map(|open| open.kind) {
            Some(Kind::Snap) => self.snap_menu(Menu::new().header("Tek seferlik kenet (sonraki tık)")),
            Some(Kind::Command) => self.command_menu(),
            Some(Kind::Idle) | None => self.idle_menu(),
        }
    }

    /// A catalog command as a menu item (the web's `commandItem`): its title,
    /// icon and key, its check where it has one; dimmed where it cannot run.
    pub(crate) fn command_item(&self, menu: Menu<Message>, id: &'static str) -> Menu<Message> {
        let Some(command) = catalog().get(id) else {
            return menu;
        };
        let runs = command.standing == Standing::Ported && self.available(id);
        let on_press = runs.then_some(Message::Run(id));
        let menu = match self.checked(id) {
            Some(on) => menu.check(command.title, on, on_press),
            None => menu.item(command.title, on_press).icon(command.icon),
        };
        match command.shortcuts.first() {
            Some(keys) => menu.shortcut(*keys),
            None => menu,
        }
    }

    /// No command running (the web's `idleItems`).
    fn idle_menu(&self) -> Menu<Message> {
        let mut menu = Menu::new();
        if let Some(last) = self.session.last() {
            let title = catalog()
                .get(&format!("tool.{last}"))
                .map_or(last, |command| command.title);
            menu = menu
                .item(format!("Yinele: {title}"), Message::Run("tool.repeat"))
                .shortcut("Enter");
        }
        [
            "-",
            "view.zoomExtents",
            "view.zoomSelection",
            "tool.pan",
            "-",
            "edit.selectAll",
            "edit.deselect",
            "-",
            "tool.move",
            "tool.copy",
            "edit.copy",
            "edit.paste",
            "tool.erase",
            "-",
            "view.coords",
        ]
        .into_iter()
        .fold(menu, |menu, id| match id {
            "-" => menu.separator(),
            id => self.command_item(menu, id),
        })
    }

    /// A command running, the button held (the web's `commandItems`).
    fn command_menu(&self) -> Menu<Message> {
        let prompt = self.session.prompt();
        let mut menu = Menu::new()
            .header(prompt.tool.unwrap_or("Komut"))
            .item("Onayla / bitir", Message::Run("tool.confirm"))
            .icon(Icon::Check)
            .shortcut("Enter")
            .item("İptal", Message::Run("tool.cancel"))
            .icon(Icon::Close)
            .shortcut("Esc")
            .separator();
        for option in prompt
            .options
            .iter()
            .filter(|o| o.key != "Enter" && o.key != "Esc")
        {
            let label = match &option.value {
                Some(value) => format!("{}: {value}", option.label),
                None => option.label.to_owned(),
            };
            menu = menu
                .item(label, Message::PromptOption(option.key))
                .shortcut(option.key);
        }
        let menu = menu
            .separator()
            .submenu("Tek seferlik kenet", self.snap_menu(Menu::new()))
            .icon(crate::icons::from_web(Some("snap")));
        ["draft.snap", "draft.ortho", "draft.polar", "draft.tracking", "-", "view.zoomExtents"]
            .into_iter()
            .fold(menu, |menu, id| match id {
                "-" => menu.separator(),
                id => self.command_item(menu, id),
            })
    }

    /// The one-shot snaps (the web's `snapItems`), after `menu`'s header.
    fn snap_menu(&self, menu: Menu<Message>) -> Menu<Message> {
        let current = self.snap_once();
        SNAP_ORDER
            .into_iter()
            .fold(menu, |menu, kind| {
                menu.check(
                    snap_label(kind),
                    current == Some(kind),
                    Message::DrawingMenu(Event::SnapOnce(Some(kind))),
                )
                .icon(snap_icon(kind))
            })
            .separator()
            .item("Kenet ayarları…", Message::Run("tools.options"))
            .icon(crate::icons::from_web(Some("settings")))
    }
}

#[cfg(test)]
mod tests {
    use iced::Point;
    use kentos_interaction::SnapKind;

    use super::{Event, Kind};
    use crate::app::{App, Message};
    use crate::files_testing::app_with_drawing;
    use crate::viewport;

    fn view(app: &mut App, event: viewport::Event) {
        let _ = app.update(Message::Viewport(event));
    }

    #[test]
    fn a_quick_right_click_is_enter_in_a_command_and_the_idle_menu_without_one() {
        let mut app = app_with_drawing();
        let at = Point::new(300.0, 200.0);
        view(&mut app, viewport::Event::RightPressed(at));
        view(&mut app, viewport::Event::RightClick(at));
        assert_eq!(app.drawing_menu.map(|m| m.kind), Some(Kind::Idle));
        let _ = app.update(Message::DrawingMenu(Event::Closed));
        assert!(app.drawing_menu.is_none());
        // A command that takes a confirm: Enter, no menu.
        let _ = app.update(Message::Run("tool.line"));
        view(&mut app, viewport::Event::RightPressed(at));
        view(&mut app, viewport::Event::RightClick(at));
        assert!(app.drawing_menu.is_none());
        assert!(!app.session.is_running(), "Enter with no point leaves the line tool");
    }

    #[test]
    fn held_it_opens_the_command_menu_and_shift_the_snap_menu() {
        let mut app = app_with_drawing();
        let at = Point::new(300.0, 200.0);
        view(&mut app, viewport::Event::RightPressed(at));
        view(&mut app, viewport::Event::RightHeld(at));
        assert_eq!(app.drawing_menu.map(|m| m.kind), Some(Kind::Idle));
        let _ = app.update(Message::DrawingMenu(Event::Closed));
        let _ = app.update(Message::Run("tool.line"));
        view(&mut app, viewport::Event::RightPressed(at));
        view(&mut app, viewport::Event::RightHeld(at));
        assert_eq!(app.drawing_menu.map(|m| m.kind), Some(Kind::Command));
        assert!(app.session.is_running(), "held, it is no Enter");
        let _ = app.update(Message::DrawingMenu(Event::Closed));
        app.modifiers = iced::keyboard::Modifiers::SHIFT;
        view(&mut app, viewport::Event::RightPressed(at));
        assert_eq!(app.drawing_menu.map(|m| m.kind), Some(Kind::Snap));
    }

    #[test]
    fn a_one_shot_snap_lasts_for_the_next_press_of_its_command() {
        let mut app = app_with_drawing();
        let _ = app.update(Message::Run("tool.line"));
        let _ = app.update(Message::DrawingMenu(Event::SnapOnce(Some(SnapKind::Midpoint))));
        assert_eq!(app.snap_once(), Some(SnapKind::Midpoint));
        view(&mut app, viewport::Event::Pressed(Point::new(300.0, 200.0)));
        assert_eq!(app.snap_once(), None, "a left press drops it");
        let _ = app.update(Message::DrawingMenu(Event::SnapOnce(Some(SnapKind::Center))));
        let _ = app.update(Message::Run("tool.circle"));
        assert_eq!(app.snap_once(), None, "another command drops it");
    }
}

/// Pictures for the owner: the idle menu, the command menu (Çizgi running),
/// the one-shot snap menu and its chip in the command line;
/// `.run/shots/sag-tik-*`.
/// `cargo test -p kentos-desktop drawing_menus::screens -- --ignored --nocapture`
#[cfg(test)]
#[test]
#[ignore = "pictures for the owner, run by hand"]
fn screens() {
    use iced::Size;
    use kentos_ui::snapshot::{Input, Snapshot};

    let out = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../.run/shots");
    std::fs::create_dir_all(&out).expect("a folder for the pictures");
    for (mode, suffix) in [("dark", ""), ("light", "-acik")] {
        for (width, height) in [(1440.0, 900.0), (1100.0, 650.0)] {
            for name in ["bosta", "komut", "kenet", "sonraki-tik"] {
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
                let area = app.viewport.bounds;
                let at = Point::new(area.width * 0.45, area.height * 0.35);
                match name {
                    "bosta" => {
                        // A line drawn first, so Yinele names it.
                        let _ = app.update(Message::Run("tool.line"));
                        let _ = app.update(Message::Run("tool.cancel"));
                        let screen = Point::new(area.x + at.x, area.y + at.y);
                        snapshot.input(&mut app, App::view, &mut update, Input::RightClick(screen));
                    }
                    "komut" => {
                        let _ = app.update(Message::Run("tool.line"));
                        app.right_held(at);
                    }
                    "kenet" => {
                        let _ = app.update(Message::Run("tool.line"));
                        app.modifiers = iced::keyboard::Modifiers::SHIFT;
                        app.right_pressed(at);
                        app.modifiers = iced::keyboard::Modifiers::default();
                    }
                    _ => {
                        let _ = app.update(Message::Run("tool.line"));
                        let _ = app.update(Message::DrawingMenu(Event::SnapOnce(Some(
                            SnapKind::Intersection,
                        ))));
                    }
                }
                snapshot.settle(&mut app, App::view, &mut update);
                let file = out.join(format!("sag-tik-{name}-{width}x{height}{suffix}.png"));
                snapshot
                    .render(app.view(), &app.theme())
                    .save(&file)
                    .expect("writes the picture");
                println!("{}", file.display());
            }
        }
    }
}
