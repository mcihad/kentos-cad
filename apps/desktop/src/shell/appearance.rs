//! The interface's look (docs/adr/0051): the theme with the accent colour,
//! the drawing's background, the interface and the monospaced typefaces and
//! the text size, each a preference kept in the settings file
//! (`appearance.*`, the web's keys too: docs/adr/0126) and applied at once.
//! They are chosen in Uygulama ayarları › Görünüm; the ribbon keeps only the
//! theme's commands (docs/adr/0622 §3).

use kentos_ui::theme::shape::{self, Corners, Shadows, Shape};
use kentos_ui::theme::typography::{self, Family, Mono, Typography};
use kentos_ui::theme::{Accent, Mode};
use serde_json::Value;

use crate::app::App;
use crate::viewport::Canvas;

/// The drawing area's background, whatever the interface's theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backdrop {
    /// The project's type's (docs/adr/0165 §5): a CAD project's dark, slate
    /// on a light theme; a CBS project's the theme's own.
    #[default]
    ByType,
    /// The theme's own: slate, paper, night or black.
    Theme,
    Slate,
    Black,
    Paper,
}

impl Backdrop {
    pub const ALL: [Backdrop; 5] = [
        Backdrop::ByType,
        Backdrop::Theme,
        Backdrop::Slate,
        Backdrop::Black,
        Backdrop::Paper,
    ];

    /// The setting's value.
    pub fn key(self) -> &'static str {
        match self {
            Backdrop::ByType => "mode",
            Backdrop::Theme => "theme",
            Backdrop::Slate => "slate",
            Backdrop::Black => "black",
            Backdrop::Paper => "paper",
        }
    }

    fn parse(key: &str) -> Self {
        Self::ALL
            .into_iter()
            .find(|b| b.key() == key)
            .unwrap_or_default()
    }
}

/// The setting's values of the typefaces (`appearance.uiFont`, the web's UI_FONTS ids).
fn family_key(family: Family) -> &'static str {
    match family {
        Family::IbmPlexSans => "plex",
        Family::Inter => "inter",
        Family::PlusJakartaSans => "jakarta",
        Family::SourceSans3 => "source",
        Family::NotoSans => "noto",
        Family::Roboto => "roboto",
        Family::System => "system",
    }
}

fn mono_key(mono: Mono) -> &'static str {
    match mono {
        Mono::IbmPlexMono => "plexMono",
        Mono::JetBrainsMono => "jetbrains",
    }
}

fn mode_key(mode: Mode) -> &'static str {
    match mode {
        Mode::Dark => "dark",
        Mode::Light => "light",
        Mode::Night => "night",
        Mode::HighContrast => "highContrast",
    }
}

impl App {
    /// A theme chosen (Görünüm › Tema, the theme commands): kept as a
    /// preference, applied at once.
    pub(crate) fn set_theme(&mut self, mode: Mode) {
        let refused = self
            .settings
            .choose(&[("appearance.theme", Value::from(mode_key(mode)))]);
        self.apply_settings();
        if refused.is_empty() {
            self.output(format!("Tema: {}.", mode.label()));
        } else {
            self.warn("Görünüm tercihi kaydedilemedi; Uygulama ayarları → Görünüm'den deneyin.");
        }
    }

    /// Reads the look from the settings: theme, accent, typefaces, size, background.
    pub(crate) fn apply_appearance(&mut self) {
        // A font-sensitive test may already hold this on the same thread;
        // another test's App::boot must not reset its global font mid-layout.
        #[cfg(test)]
        let _typography = tests::TYPOGRAPHY.lock();
        let s = &self.settings;
        let word = |key: &str| s.effective(key).as_str().unwrap_or_default().to_owned();
        self.mode = match word("appearance.theme").as_str() {
            "light" => Mode::Light,
            "night" => Mode::Night,
            "highContrast" => Mode::HighContrast,
            _ => Mode::Dark,
        };
        self.accent = Accent::parse(&word("appearance.accent")).unwrap_or_default();
        self.backdrop = Backdrop::parse(&word("appearance.drawingBackground"));
        let family = Family::ALL
            .into_iter()
            .find(|f| family_key(*f) == word("appearance.uiFont"))
            .unwrap_or_default();
        let mono = Mono::ALL
            .into_iter()
            .find(|m| mono_key(*m) == word("appearance.monoFont"))
            .unwrap_or_default();
        let size = s.number("appearance.textSize") as f32;
        let typography = Typography { family, mono, size }.clamped();
        // The typography is the whole interface's (KentOS UI keeps it globally):
        // it is set when it changes, not by every app that reads the defaults.
        if typography != self.typography {
            self.typography = typography;
            typography::set(typography);
        }
        // So are the corners and the shadows (docs/adr/0127).
        let shape = Shape {
            corners: Corners::parse(&word("appearance.corners")).unwrap_or_default(),
            shadows: Shadows::parse(&word("appearance.shadows")).unwrap_or_default(),
        };
        if shape != self.shape {
            self.shape = shape;
            shape::set(shape);
        }
    }

    /// What the drawing area is drawn on.
    pub(crate) fn canvas(&self) -> Canvas {
        match self.backdrop {
            // A CAD project's drawing is dark whatever the theme (docs/adr/0165 §5).
            Backdrop::ByType
                if self.work_mode() == kentos_commands::Workspace::Cad
                    && self.mode == Mode::Light =>
            {
                Canvas::Slate
            }
            Backdrop::ByType | Backdrop::Theme => Canvas::from(self.mode),
            Backdrop::Slate => Canvas::Slate,
            Backdrop::Black => Canvas::Black,
            Backdrop::Paper => Canvas::Paper,
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use parking_lot::ReentrantMutex;

    use kentos_ui::theme::typography::{self, Family, Typography};
    use kentos_ui::theme::{Accent, Mode};
    use serde_json::Value;

    use super::Backdrop;
    use crate::app::{App, Message};
    use crate::files_testing::app_with_drawing;
    use crate::viewport::Canvas;

    /// The interface's typography is KentOS UI's, one for the whole process:
    /// Tests that measure text hold this for the whole interaction; test-only
    /// apply_appearance also takes it, including nested App boot on this thread.
    /// Do not join a worker that applies appearance while holding this guard.
    pub(crate) static TYPOGRAPHY: ReentrantMutex<()> = ReentrantMutex::new(());

    /// A look's preference chosen as Uygulama ayarları chooses it.
    fn choose(app: &mut App, key: &str, value: Value) {
        assert!(app.settings.choose(&[(key, value)]).is_empty(), "{key}");
        app.apply_settings();
    }

    fn setting(app: &App, key: &str) -> Value {
        app.settings.effective(key)
    }

    #[test]
    fn choices_are_kept_as_preferences_and_applied_at_once() {
        let _typography = TYPOGRAPHY.lock();
        let mut app = app_with_drawing();
        app.set_theme(Mode::Night);
        assert_eq!(app.mode, Mode::Night);
        assert_eq!(setting(&app, "appearance.theme"), "night");
        assert_eq!(app.canvas(), Canvas::Night, "the drawing follows the theme");
        choose(&mut app, "appearance.drawingBackground", Value::from("black"));
        assert_eq!(app.canvas(), Canvas::Black, "whatever the theme");
        // The web's keys and ids (docs/adr/0126).
        choose(&mut app, "appearance.accent", Value::from("violet"));
        assert_eq!(app.accent, Accent::Violet);
        choose(&mut app, "appearance.uiFont", Value::from("inter"));
        assert_eq!(app.typography.family, Family::Inter);
        choose(&mut app, "appearance.textSize", Value::from(14));
        assert_eq!(app.typography.size, 14.0);
        choose(&mut app, "appearance.textSize", Value::from(Typography::DEFAULT.size as i64));
        choose(&mut app, "appearance.uiFont", Value::from("jakarta"));
        assert_eq!(typography::current(), Typography::DEFAULT);
    }

    /// “Türe göre”, the default (docs/adr/0165 §5): a CAD project's drawing is
    /// dark whatever the theme, a CBS project's the theme's own.
    #[test]
    fn the_drawing_ground_follows_the_projects_type() {
        let mut app = app_with_drawing();
        assert_eq!(app.backdrop, Backdrop::ByType);
        app.set_theme(Mode::Light);
        assert_eq!(app.canvas(), Canvas::Paper, "a CBS project: the theme's");
        app.choose_mode("workspace.cad");
        assert_eq!(
            app.canvas(),
            Canvas::Slate,
            "a CAD project: slate on a light theme"
        );
        app.set_theme(Mode::Night);
        assert_eq!(app.canvas(), Canvas::Night, "and a dark theme's own");
        app.set_theme(Mode::Light);
        choose(&mut app, "appearance.drawingBackground", Value::from("theme"));
        assert_eq!(
            app.canvas(),
            Canvas::Paper,
            "Temaya uy: the theme's whatever the type"
        );
    }

    #[test]
    fn the_theme_commands_know_the_four_themes() {
        let mut app = app_with_drawing();
        app.set_theme(Mode::HighContrast);
        let _ = app.update(Message::Run("view.theme.toggle"));
        assert_eq!(app.mode, Mode::Light, "every dark theme turns light");
        let _ = app.update(Message::Run("view.theme.toggle"));
        assert_eq!(app.mode, Mode::Dark);
        assert_eq!(app.checked("view.theme.dark"), Some(true));
    }

    #[test]
    fn a_custom_accent_is_read_and_a_broken_one_is_refused() {
        let mut app = app_with_drawing();
        let _ = app
            .settings
            .choose(&[("appearance.accent", Value::from("#FF8800"))]);
        app.apply_settings();
        assert_eq!(app.accent, Accent::Custom(0xff8800));
        assert_eq!(
            setting(&app, "appearance.accent"),
            "#ff8800",
            "kept in lower case"
        );
        let refused = app
            .settings
            .choose(&[("appearance.accent", Value::from("bozuk"))]);
        assert!(!refused.is_empty(), "the shared rules refuse it");
        app.apply_settings();
        assert_eq!(app.accent, Accent::Custom(0xff8800), "the chosen one stays");
    }

    /// An older settings file of this program (and one of the web's) is read
    /// under today's keys: the look comes back as it was (docs/adr/0126).
    #[test]
    fn an_older_settings_file_keeps_its_look() {
        use kentos_commands::{SettingsFile, settings_schema};
        let text = r##"{"format":"kentos.settings","version":1,"user":{"appearance.accentColor":"turkuaz","appearance.typeface":"inter","appearance.monoTypeface":"jetbrains","appearance.textSize":15,"appearance.theme":"night"}}"##;
        let (file, diagnostics) = SettingsFile::from_json(text, &settings_schema()).expect("reads");
        assert!(diagnostics.is_empty(), "{diagnostics:?}");
        assert_eq!(
            file.user.get("appearance.accent"),
            Some(&Value::from("teal"))
        );
        assert_eq!(
            file.user.get("appearance.uiFont"),
            Some(&Value::from("inter"))
        );
        assert_eq!(
            file.user.get("appearance.monoFont"),
            Some(&Value::from("jetbrains"))
        );
    }
}
