//! The snap kinds one by one (docs/adr/0163 §5–§6), as the web's
//! (`ui/statusbar/snapMenu.ts`, `SNAP_KIND_COMMANDS` in `app/commands.ts`):
//!
//! - each kind's command `draft.snap.<kind>` turns its `snap.<kind>` setting
//!   (a preference, kept), `draft.snap.self` turns Çizilmekte olan nesneye;
//! - the Kenet cell's right-click menu: the kinds each a tick, Çizilmekte
//!   olan nesneye, Karelaj aralığı's usual spacings (choosing one turns
//!   Karelaj on; Farklı aralık… opens the settings) and Kenet ayarları…;
//! - out of the snap's scale range the cell is idle, its tip says the view's
//!   scale first.

use kentos_interaction::screen_scale;
use kentos_ui::widget::Menu;
use serde_json::Value;

use crate::app::{App, Message};
use crate::catalog::catalog;
use crate::settings_sections::Section;

/// The kinds' commands and the settings they turn, in the menu's order:
/// the one-shot menu's, then the additions (the web's `KINDS`).
const KINDS: [(&str, &str); 12] = [
    ("draft.snap.endpoint", "snap.endpoint"),
    ("draft.snap.midpoint", "snap.midpoint"),
    ("draft.snap.intersection", "snap.intersection"),
    ("draft.snap.center", "snap.center"),
    ("draft.snap.perpendicular", "snap.perpendicular"),
    ("draft.snap.tangent", "snap.tangent"),
    ("draft.snap.node", "snap.node"),
    ("draft.snap.nearest", "snap.nearest"),
    ("draft.snap.centroid", "snap.centroid"),
    ("draft.snap.extension", "snap.extension"),
    ("draft.snap.parallel", "snap.parallel"),
    ("draft.snap.grid", "snap.grid"),
];

/// Çizilmekte olan nesneye's command and setting (§3).
const SELF: (&str, &str) = ("draft.snap.self", "snap.self");

/// Karelaj's usual spacings, metres: the same east and north (the web's `GRID_SPACINGS`).
pub(crate) const GRID_SPACINGS: [f64; 10] =
    [0.1, 0.25, 0.5, 1.0, 2.0, 5.0, 10.0, 20.0, 50.0, 100.0];

/// What the Kenet cell's menu asks for besides commands.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Event {
    /// Karelaj at this spacing, east and north, and on.
    Spacing(f64),
    /// The settings window on Kenetleme (Farklı aralık…, Kenet ayarları…).
    Settings,
}

/// The setting a snap command turns, when `id` is one.
pub(crate) fn setting(id: &str) -> Option<&'static str> {
    KINDS
        .iter()
        .chain([&SELF])
        .find(|(command, _)| *command == id)
        .map(|(_, key)| *key)
}

/// A spacing as set, not to the drawing's decimals: “0.25 m”, “1 m” (the web's `spacing`).
pub(crate) fn spacing(v: f64) -> String {
    format!("{v} m")
}

impl App {
    pub(crate) fn snap_event(&mut self, event: Event) {
        match event {
            Event::Spacing(v) => self.choose_grid_spacing(v),
            Event::Settings => self.open_settings_at(Section::Snap),
        }
    }

    /// A snap kind's command, or Çizilmekte olan nesneye's: its setting turned.
    pub(crate) fn toggle_snap_setting(&mut self, id: &str) -> bool {
        let Some(key) = setting(id) else {
            return false;
        };
        let name = catalog().get(id).map_or(id, |c| c.title);
        self.toggle_session(key, name);
        true
    }

    /// Karelaj at this spacing, east and north, and on (the menu's spacings).
    fn choose_grid_spacing(&mut self, v: f64) {
        let _ = self.settings.choose(&[
            ("snap.gridEast", Value::from(v)),
            ("snap.gridNorth", Value::from(v)),
            ("snap.grid", Value::Bool(true)),
        ]);
        self.apply_settings();
        self.output(format!("Karelaj açık: {} × {}", spacing(v), spacing(v)));
    }

    /// The view's screen scale when it lies out of the snap's range (§5), else none.
    pub(crate) fn snap_out_of_range(&self) -> Option<f64> {
        let n = screen_scale(1.0 / self.viewport.camera.scale);
        (!self.draft.snap_in_range(n)).then_some(n)
    }

    /// What Kenet's tip says first while the view is out of the snap's scale range.
    pub(crate) fn snap_note(&self) -> Option<String> {
        self.snap_out_of_range().map(|n| {
            format!(
                "Ölçek aralığının dışında (1:{}): kenet bu ölçekte çalışmaz.",
                crate::view::thousands(n)
            )
        })
    }

    /// The Kenet cell's right-click menu (the web's `snapMenu`).
    pub(crate) fn snap_cell_menu(&self) -> Menu<Message> {
        let run = |id: &'static str| catalog().get(id).map(|_| Message::Run(id));
        let mut menu = Menu::new().header("Kenet türleri");
        for (id, _) in KINDS {
            let Some(command) = catalog().get(id) else {
                continue;
            };
            menu = menu
                .check(command.short, self.checked(id).unwrap_or(false), run(id))
                .icon(command.icon);
        }
        menu = menu.separator().check(
            "Çizilmekte olan nesneye",
            self.checked(SELF.0).unwrap_or(false),
            run(SELF.0),
        );
        let (east, north) = (
            self.settings.number("snap.gridEast"),
            self.settings.number("snap.gridNorth"),
        );
        let mut spacings = Menu::new();
        for v in GRID_SPACINGS {
            spacings = spacings.radio(
                spacing(v),
                east == v && north == v,
                Message::Snap(Event::Spacing(v)),
            );
        }
        let settings = crate::icons::from_web(Some("settings"));
        let spacings = spacings
            .separator()
            .item("Farklı aralık…", Message::Snap(Event::Settings))
            .icon(settings);
        menu.submenu("Karelaj aralığı", spacings)
            .icon(crate::icons::from_web(Some("snapGrid")))
            .hint(format!("{} × {}", spacing(east), spacing(north)))
            .item("Kenet ayarları…", Message::Snap(Event::Settings))
            .icon(settings)
    }
}

#[cfg(test)]
mod tests {
    use kentos_interaction::SnapKind;

    use super::*;
    use crate::app::Dialog;

    fn app() -> App {
        App::boot(None).0
    }

    fn on(app: &App, kind: SnapKind) -> bool {
        app.draft.snap_kinds & kind.bit() != 0
    }

    #[test]
    fn a_kinds_command_turns_its_setting_and_the_cell_shows_it() {
        let mut app = app();
        for kind in [
            SnapKind::Centroid,
            SnapKind::Extension,
            SnapKind::Parallel,
            SnapKind::Grid,
        ] {
            assert!(!on(&app, kind), "{kind:?} is off at first");
        }
        assert_eq!(app.checked("draft.snap.grid"), Some(false));
        let _ = app.update(Message::Run("draft.snap.grid"));
        assert_eq!(app.checked("draft.snap.grid"), Some(true));
        assert!(on(&app, SnapKind::Grid));
        assert!(app.settings.bool("snap.grid"));
        // Uç nokta takes Çeyrek with it, as its setting does.
        let _ = app.update(Message::Run("draft.snap.endpoint"));
        assert!(!on(&app, SnapKind::Endpoint) && !on(&app, SnapKind::Quadrant));
        assert!(app.draft.snap_self);
        let _ = app.update(Message::Run("draft.snap.self"));
        assert!(!app.draft.snap_self);
        assert_eq!(app.checked("draft.snap.self"), Some(false));
    }

    #[test]
    fn the_menu_lists_the_kinds_ticked_and_karelaj_s_spacing() {
        let app = app();
        let menu = format!("{:?}", app.snap_cell_menu());
        for name in [
            "Uç nokta",
            "Ağırlık merkezi",
            "Uzantı",
            "Paralel",
            "Karelaj",
            "Çizilmekte olan nesneye",
            "Karelaj aralığı",
            "1 m × 1 m",
            "0.25 m",
            "Farklı aralık…",
            "Kenet ayarları…",
        ] {
            assert!(menu.contains(name), "{name} is on the menu");
        }
    }

    #[test]
    fn a_spacing_sets_both_and_turns_karelaj_on() {
        let mut app = app();
        let _ = app.update(Message::Snap(Event::Spacing(0.25)));
        assert_eq!(app.draft.snap_grid, [0.25, 0.25]);
        assert!(on(&app, SnapKind::Grid));
        let _ = app.update(Message::Snap(Event::Settings));
        assert_eq!(app.dialog, Some(Dialog::Settings));
        assert_eq!(
            app.settings_draft.as_ref().map(|d| d.section),
            Some(Section::Snap)
        );
    }

    #[test]
    fn out_of_the_scale_range_the_cell_says_the_scale() {
        let mut app = app();
        // 0.125 m a pixel: 1:472.
        app.viewport.camera.scale = 8.0;
        assert_eq!(app.snap_out_of_range(), None);
        assert_eq!(app.snap_note(), None);
        let _ = app.settings.choose(&[("snap.scaleMax", Value::from(400))]);
        app.apply_settings();
        assert_eq!(app.snap_out_of_range(), Some(472.0));
        assert_eq!(
            app.snap_note().as_deref(),
            Some("Ölçek aralığının dışında (1:472): kenet bu ölçekte çalışmaz.")
        );
        app.viewport.camera.scale = 1.0 / (0.00026458 * 400.0);
        assert_eq!(app.snap_out_of_range(), None);
    }
}
