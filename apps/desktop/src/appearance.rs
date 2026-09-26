//! Görünüm (the showcase's interface groups, on the desktop): the theme
//! with the accent colour, the drawing's background, the interface and the
//! monospaced typefaces and the text size (docs/adr/0051). Each choice is a
//! preference kept in the settings file (`appearance.*`) and applies at
//! once; the ribbon's Görünüm tab shows them after the web's panels, and
//! they fold into one button each when the window is narrow.

use iced::widget::{button, column, container, row, space, text, tooltip};
use iced::{Border, Center, Color, Element, Fill, Font, Task, Theme};
use kentos_ui::icon::Icon;
use kentos_ui::theme::typography::{self, Family, Mono, Typography};
use kentos_ui::theme::{Accent, Mode, Tokens};
use kentos_ui::widget::ribbon::{self, Button as Tool, Group};
use kentos_ui::widget::{Menu, Tip, tip};
use kentos_ui::{label, style};
use serde_json::Value;

use crate::app::{App, Message};
use crate::viewport::Canvas;

/// The drawing area's background, whatever the interface's theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Backdrop {
    /// The theme's own: slate, paper, night or black.
    #[default]
    Theme,
    Slate,
    Black,
    Paper,
}

impl Backdrop {
    pub const ALL: [Backdrop; 4] = [
        Backdrop::Theme,
        Backdrop::Slate,
        Backdrop::Black,
        Backdrop::Paper,
    ];

    /// The setting's value.
    pub fn key(self) -> &'static str {
        match self {
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

    pub fn name(self) -> &'static str {
        match self {
            Backdrop::Theme => "Temaya uy",
            Backdrop::Slate => "Arduvaz",
            Backdrop::Black => "Siyah",
            Backdrop::Paper => "Kâğıt",
        }
    }

    fn note(self) -> &'static str {
        match self {
            Backdrop::Theme => {
                "Temaya uyar: koyu temada arduvaz, aydınlıkta kâğıt, gecede kısık gece zemini, yüksek karşıtlıkta siyah."
            }
            Backdrop::Slate => "Koyu gri-mavi model alanı; uzun çalışmada göz yormaz.",
            Backdrop::Black => "Klasik AutoCAD: saf siyah zemin, parlak çizgiler.",
            Backdrop::Paper => "Beyaza yakın zemin; çıktıya en yakın görünüm.",
        }
    }
}

/// The setting's values of the typefaces.
fn family_key(family: Family) -> &'static str {
    match family {
        Family::IbmPlexSans => "plex",
        Family::Inter => "inter",
        Family::PlusJakartaSans => "jakarta",
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

fn theme_note(mode: Mode) -> &'static str {
    match mode {
        Mode::Dark => "CAD programlarının grafit arayüzü; uzun çalışmada göz yormaz.",
        Mode::Light => {
            "Kâğıt zeminli aydınlık arayüz; aydınlık ortamda ve çıktıya yakın çalışırken."
        }
        Mode::Night => {
            "Çok koyu, az parlak arayüz ve kısık çizim zemini; karanlık odada ekran parlamaz."
        }
        Mode::HighContrast => {
            "Siyah zemin, beyaz yazı ve parlak kenarlar; yazılar ve vurgu en az 7:1 karşıtlıkta."
        }
    }
}

fn family_note(family: Family) -> &'static str {
    match family {
        Family::IbmPlexSans => {
            "Mühendislik çizgili, dar ve sakin bir grotesk; ekrana çok metin sığar."
        }
        Family::Inter => "Ekran için çizilmiş; x yüksekliği büyük, küçük boyutta en okunaklısı.",
        Family::PlusJakartaSans => "Geometrik, açık ve yumuşak hatlı; ferah bir görünüm.",
    }
}

/// A step of the text size.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Step {
    Larger,
    Smaller,
    Default,
}

#[derive(Debug, Clone)]
pub enum Event {
    Theme(Mode),
    Accent(Accent),
    Backdrop(Backdrop),
    Family(Family),
    Mono(Mono),
    Size(Step),
}

fn event(e: Event) -> Message {
    Message::Appearance(e)
}

impl App {
    /// A choice from the Görünüm tab: kept as a preference, applied at once.
    pub(crate) fn appearance_event(&mut self, e: Event) -> Task<Message> {
        let (key, value, said) = match e {
            Event::Theme(mode) => (
                "appearance.theme",
                Value::from(mode_key(mode)),
                format!("Tema: {}.", mode.label()),
            ),
            Event::Accent(accent) => (
                "appearance.accentColor",
                Value::from(accent.key()),
                format!("Vurgu rengi: {}.", accent.name()),
            ),
            Event::Backdrop(backdrop) => (
                "appearance.drawingBackground",
                Value::from(backdrop.key()),
                format!("Çizim zemini: {}.", backdrop.name()),
            ),
            Event::Family(family) => (
                "appearance.typeface",
                Value::from(family_key(family)),
                format!("Yazı tipi: {}.", family.name()),
            ),
            Event::Mono(mono) => (
                "appearance.monoTypeface",
                Value::from(mono_key(mono)),
                format!("Eş aralıklı yazı: {}.", mono.name()),
            ),
            Event::Size(step) => {
                let size = match step {
                    Step::Larger => self.typography.size + 1.0,
                    Step::Smaller => self.typography.size - 1.0,
                    Step::Default => Typography::DEFAULT.size,
                }
                .clamp(*Typography::SIZES.start(), *Typography::SIZES.end());
                (
                    "appearance.textSize",
                    Value::from(size as i64),
                    format!("Yazı boyutu: {size} piksel."),
                )
            }
        };
        let refused = self.settings.choose(&[(key, value)]);
        self.apply_settings();
        if refused.is_empty() {
            self.output(said);
        } else {
            self.warn("Görünüm tercihi kaydedilemedi; Uygulama ayarları → Görünüm'den deneyin.");
        }
        Task::none()
    }

    /// Reads the look from the settings: theme, accent, typefaces, size, background.
    pub(crate) fn apply_appearance(&mut self) {
        let s = &self.settings;
        let word = |key: &str| s.effective(key).as_str().unwrap_or_default().to_owned();
        self.mode = match word("appearance.theme").as_str() {
            "light" => Mode::Light,
            "night" => Mode::Night,
            "highContrast" => Mode::HighContrast,
            _ => Mode::Dark,
        };
        self.accent = Accent::parse(&word("appearance.accentColor")).unwrap_or_default();
        self.backdrop = Backdrop::parse(&word("appearance.drawingBackground"));
        let family = Family::ALL
            .into_iter()
            .find(|f| family_key(*f) == word("appearance.typeface"))
            .unwrap_or_default();
        let mono = Mono::ALL
            .into_iter()
            .find(|m| mono_key(*m) == word("appearance.monoTypeface"))
            .unwrap_or_default();
        let size = s.number("appearance.textSize") as f32;
        let typography = Typography { family, mono, size }.clamped();
        // The typography is the whole interface's (KentOS UI keeps it globally):
        // it is set when it changes, not by every app that reads the defaults.
        if typography != self.typography {
            self.typography = typography;
            typography::set(typography);
        }
    }

    /// What the drawing area is drawn on.
    pub(crate) fn canvas(&self) -> Canvas {
        match self.backdrop {
            Backdrop::Theme => Canvas::from(self.mode),
            Backdrop::Slate => Canvas::Slate,
            Backdrop::Black => Canvas::Black,
            Backdrop::Paper => Canvas::Paper,
        }
    }

    /// The Görünüm tab's groups after the web's panels.
    pub(crate) fn appearance_groups(&self) -> Vec<Group<'static, Message>> {
        vec![
            self.theme_group(),
            self.backdrop_group(),
            self.typeface_group(),
            self.mono_group(),
            self.size_group(),
        ]
    }

    fn theme_group(&self) -> Group<'static, Message> {
        let (mode, accent) = (self.mode, self.accent);
        let view = move || -> Element<'static, Message> {
            let tile = |m: Mode| theme_tile(m, accent, m == mode);
            let themes = column![
                row![tile(Mode::Dark), tile(Mode::Light)].spacing(2),
                row![tile(Mode::Night), tile(Mode::HighContrast)].spacing(2),
                label::caption(match mode {
                    Mode::HighContrast => "Karşıtlık",
                    other => other.label(),
                }),
            ]
            .spacing(2)
            .width(typography::scaled(72.0))
            .align_x(Center);
            let chips = |presets: &[Accent]| {
                presets.iter().fold(row![].spacing(1), |row, &a| {
                    row.push(accent_chip(a, mode, a == accent))
                })
            };
            let custom = Tool::small(
                Icon::Drop,
                match accent {
                    Accent::Custom(_) => accent.name(),
                    _ => "Özel renk…".to_owned(),
                },
            )
            .on(matches!(accent, Accent::Custom(_)))
            .on_press(Message::Run("tools.options"))
            .tip(
                Tip::new("Özel vurgu rengi")
                    .body("Uygulama ayarları → Görünüm'de #RRGGBB yazın; renk temanın zemininde okunur kalacak kadar ayarlanır."),
            );
            row![
                themes,
                column![
                    chips(&Accent::PRESETS[..4]),
                    chips(&Accent::PRESETS[4..]),
                    Element::from(custom),
                ]
                .spacing(1),
            ]
            .spacing(8)
            .into()
        };
        let menu = move || {
            let menu = Mode::ALL
                .into_iter()
                .fold(Menu::new().header("Tema"), |menu, m| {
                    menu.check(m.label(), m == mode, event(Event::Theme(m)))
                })
                .separator()
                .header("Vurgu rengi");
            Accent::PRESETS
                .into_iter()
                .fold(menu, |menu, a| {
                    menu.check(a.name(), a == accent, event(Event::Accent(a)))
                })
                .item("Özel renk…", Message::Run("tools.options"))
        };
        Group::new("Tema").icon(Icon::Contrast).custom(
            typography::scaled(72.0) + 8.0 + chips_width(),
            view,
            menu,
        )
    }

    fn backdrop_group(&self) -> Group<'static, Message> {
        let current = self.backdrop;
        let view = move || -> Element<'static, Message> {
            let tile = |b: Backdrop| backdrop_tile(b, b == current);
            column![
                row![tile(Backdrop::Theme), tile(Backdrop::Slate)].spacing(2),
                row![tile(Backdrop::Black), tile(Backdrop::Paper)].spacing(2),
                label::caption(current.name()),
            ]
            .spacing(2)
            .width(typography::scaled(72.0))
            .align_x(Center)
            .into()
        };
        let menu = move || {
            Backdrop::ALL
                .into_iter()
                .fold(Menu::new().header("Çizim zemini"), |menu, b| {
                    menu.check(b.name(), b == current, event(Event::Backdrop(b)))
                })
        };
        Group::new("Çizim zemini")
            .icon(Icon::Layout)
            .custom(typography::scaled(72.0), view, menu)
    }

    fn typeface_group(&self) -> Group<'static, Message> {
        let current = self.typography;
        let view = move || -> Element<'static, Message> {
            Family::ALL
                .into_iter()
                .fold(row![].spacing(2), |row, family| {
                    let sample = Typography { family, ..current };
                    row.push(typeface_tile(
                        "Aa",
                        family.name(),
                        family_note(family),
                        sample.ui(),
                        current.family == family,
                        event(Event::Family(family)),
                    ))
                })
                .into()
        };
        let menu = move || {
            Family::ALL
                .into_iter()
                .fold(Menu::new().header("Yazı tipi"), |menu, family| {
                    menu.check(
                        family.name(),
                        current.family == family,
                        event(Event::Family(family)),
                    )
                })
        };
        Group::new("Yazı tipi")
            .icon(Icon::Type)
            .custom(tiles_width(Family::ALL.len()), view, menu)
    }

    fn mono_group(&self) -> Group<'static, Message> {
        let current = self.typography;
        let view = move || -> Element<'static, Message> {
            Mono::ALL
                .into_iter()
                .fold(row![].spacing(2), |row, mono| {
                    let sample = Typography { mono, ..current };
                    row.push(typeface_tile(
                        "41°",
                        mono.name(),
                        "Koordinatların, ölçülerin ve komut satırının yazısı.",
                        sample.mono(),
                        current.mono == mono,
                        event(Event::Mono(mono)),
                    ))
                })
                .into()
        };
        let menu = move || {
            Mono::ALL
                .into_iter()
                .fold(Menu::new().header("Eş aralıklı"), |menu, mono| {
                    menu.check(mono.name(), current.mono == mono, event(Event::Mono(mono)))
                })
        };
        Group::new("Eş aralıklı")
            .icon(Icon::Hash)
            .custom(tiles_width(Mono::ALL.len()), view, menu)
    }

    fn size_group(&self) -> Group<'static, Message> {
        let size = self.typography.size;
        let (smallest, largest) = (*Typography::SIZES.start(), *Typography::SIZES.end());
        let larger = (size < largest).then(|| event(Event::Size(Step::Larger)));
        let smaller = (size > smallest).then(|| event(Event::Size(Step::Smaller)));
        let default = (size != Typography::DEFAULT.size).then(|| event(Event::Size(Step::Default)));
        let (l, s, d) = (larger.clone(), smaller.clone(), default.clone());
        let view = move || -> Element<'static, Message> {
            let value = container(
                column![
                    text(format!("{size}"))
                        .font(typography::ui_strong())
                        .size(typography::scaled(22.0)),
                    label::caption("piksel"),
                ]
                .spacing(1)
                .align_x(Center),
            )
            .center_x(typography::scaled(48.0))
            .center_y(ribbon::content_height());
            let steps = column![
                Element::from(
                    Tool::small(Icon::Plus, "Büyüt")
                        .on_press_maybe(l.clone())
                        .tip(Tip::new("Yazıyı büyüt").body(format!("En çok {largest} piksel.")))
                ),
                Element::from(
                    Tool::small(Icon::Minus, "Küçült")
                        .on_press_maybe(s.clone())
                        .tip(Tip::new("Yazıyı küçült").body(format!("En az {smallest} piksel.")))
                ),
                Element::from(
                    Tool::small(Icon::Undo, "Varsayılan")
                        .on_press_maybe(d.clone())
                        .tip(
                            Tip::new("Varsayılan boyut")
                                .body(format!("{} piksel", Typography::DEFAULT.size)),
                        )
                ),
            ]
            .spacing(1);
            row![value, steps].spacing(4).into()
        };
        let menu = move || {
            Menu::new()
                .header(format!("Yazı boyutu: {size} piksel"))
                .item("Büyüt", larger.clone())
                .item("Küçült", smaller.clone())
                .item("Varsayılan", default.clone())
        };
        let steps_width =
            4.0 + 16.0 + 6.0 + typography::text_width("Varsayılan", typography::caption()) + 7.0;
        Group::new("Yazı boyutu").icon(Icon::Type).custom(
            typography::scaled(48.0) + 4.0 + steps_width,
            view,
            menu,
        )
    }
}

/// The accent chips' column: four to a row.
fn chips_width() -> f32 {
    4.0 * (14.0 + 4.0 + 2.0) + 3.0
}

/// Typeface tiles side by side.
fn tiles_width(count: usize) -> f32 {
    typography::scaled(78.0) * count as f32 + 2.0 * (count.saturating_sub(1)) as f32
}

/// A theme's preview: its window, surface, text and accent.
fn theme_tile(mode: Mode, accent: Accent, selected: bool) -> Element<'static, Message> {
    let tokens = mode.tokens(accent);
    let block = |color: Color, width: f32, height: f32, radius: f32| {
        container(space::horizontal())
            .width(width)
            .height(height)
            .style(move |_: &Theme| container::Style {
                background: Some(color.into()),
                border: iced::border::rounded(radius),
                ..container::Style::default()
            })
    };
    let body = container(
        row![
            block(tokens.text, 10.0, 2.0, 1.0),
            space::horizontal(),
            block(tokens.accent, 5.0, 5.0, 2.5),
        ]
        .align_y(Center),
    )
    .padding([0, 3])
    .width(Fill)
    .center_y(11)
    .style(move |_: &Theme| container::Style {
        background: Some(tokens.surface.into()),
        ..container::Style::default()
    });
    let preview = container(column![block(tokens.window, 26.0, 4.0, 0.0), body])
        .width(28)
        .padding(1)
        .style(move |_: &Theme| container::Style {
            background: Some(tokens.window.into()),
            border: Border {
                color: tokens.border,
                width: 1.0,
                radius: 2.0.into(),
            },
            ..container::Style::default()
        });
    tip(
        button(preview)
            .on_press(event(Event::Theme(mode)))
            .padding(2)
            .style(style::button::swatch(selected)),
        Tip::new(mode.label()).body(theme_note(mode)),
        tooltip::Position::Bottom,
    )
}

/// An accent colour: a round swatch; the chosen one ringed.
fn accent_chip(accent: Accent, mode: Mode, selected: bool) -> Element<'static, Message> {
    let color = accent.color(mode);
    let chip = container(space::horizontal())
        .width(14)
        .height(14)
        .style(move |_: &Theme| container::Style {
            background: Some(color.into()),
            border: Border {
                color: Color::BLACK.scale_alpha(0.3),
                width: 1.0,
                radius: 7.0.into(),
            },
            ..container::Style::default()
        });
    tip(
        button(chip)
            .on_press(event(Event::Accent(accent)))
            .padding(2)
            .style(move |theme, status| {
                let mut style = style::button::swatch(selected)(theme, status);
                style.border.radius = 10.0.into();
                style
            }),
        Tip::new(accent.name()),
        tooltip::Position::Bottom,
    )
}

/// A drawing background: its colour; “Temaya uy” half slate, half paper.
fn backdrop_tile(backdrop: Backdrop, selected: bool) -> Element<'static, Message> {
    let fill = |canvas: Canvas, width: f32| {
        let c = crate::viewport::palette(canvas).background.0;
        let color = Color::from_rgb8(c[0], c[1], c[2]);
        container(space::horizontal())
            .width(width)
            .height(16)
            .style(move |_: &Theme| container::Style {
                background: Some(color.into()),
                ..container::Style::default()
            })
    };
    let preview: Element<'static, Message> = match backdrop {
        Backdrop::Theme => row![fill(Canvas::Slate, 13.0), fill(Canvas::Paper, 13.0)].into(),
        Backdrop::Slate => fill(Canvas::Slate, 26.0).into(),
        Backdrop::Black => fill(Canvas::Black, 26.0).into(),
        Backdrop::Paper => fill(Canvas::Paper, 26.0).into(),
    };
    let framed = container(preview)
        .padding(1)
        .style(|theme: &Theme| container::Style {
            border: Border {
                color: Tokens::of(theme).border,
                width: 1.0,
                radius: 2.0.into(),
            },
            ..container::Style::default()
        });
    tip(
        button(framed)
            .on_press(event(Event::Backdrop(backdrop)))
            .padding(2)
            .style(style::button::swatch(selected)),
        Tip::new(backdrop.name()).body(backdrop.note()),
        tooltip::Position::Bottom,
    )
}

/// A typeface: a sample in its own letters, its name in the interface's.
fn typeface_tile(
    sample: &'static str,
    name: &'static str,
    note: &'static str,
    font: Font,
    active: bool,
    on_press: Message,
) -> Element<'static, Message> {
    let tile = button(
        column![
            text(sample)
                .font(font)
                .size(typography::scaled(22.0))
                .line_height(1.0),
            label::caption(name)
                .style(style::text::default)
                .align_x(Center)
                .width(Fill),
        ]
        .spacing(5)
        .align_x(Center)
        .width(Fill),
    )
    .on_press(on_press)
    .width(typography::scaled(78.0))
    .height(ribbon::content_height())
    .padding([7, 4])
    .style(style::button::ribbon(if active {
        style::button::Ribbon::On
    } else {
        style::button::Ribbon::Idle
    }));
    tip(tile, Tip::new(name).body(note), tooltip::Position::Bottom)
}

#[cfg(test)]
pub(crate) mod tests {
    use std::sync::Mutex;

    use kentos_ui::theme::typography::{self, Family, Typography};
    use kentos_ui::theme::{Accent, Mode};
    use serde_json::Value;

    use super::{Backdrop, Event, Step};
    use crate::app::{App, Message};
    use crate::files_testing::app_with_drawing;
    use crate::viewport::Canvas;

    /// The interface's typography is KentOS UI's, one for the whole process:
    /// tests that change it or measure text take this lock.
    pub(crate) static TYPOGRAPHY: Mutex<()> = Mutex::new(());

    fn look(app: &mut App, e: Event) {
        let _ = app.update(Message::Appearance(e));
    }

    fn setting(app: &App, key: &str) -> Value {
        app.settings.effective(key)
    }

    #[test]
    fn choices_are_kept_as_preferences_and_applied_at_once() {
        let _typography = TYPOGRAPHY.lock();
        let mut app = app_with_drawing();
        look(&mut app, Event::Theme(Mode::Night));
        assert_eq!(app.mode, Mode::Night);
        assert_eq!(setting(&app, "appearance.theme"), "night");
        assert_eq!(app.canvas(), Canvas::Night, "the drawing follows the theme");
        look(&mut app, Event::Backdrop(Backdrop::Black));
        assert_eq!(app.canvas(), Canvas::Black, "whatever the theme");
        assert_eq!(setting(&app, "appearance.drawingBackground"), "black");
        look(&mut app, Event::Accent(Accent::Violet));
        assert_eq!(app.accent, Accent::Violet);
        assert_eq!(setting(&app, "appearance.accentColor"), "mor");
        look(&mut app, Event::Family(Family::Inter));
        assert_eq!(app.typography.family, Family::Inter);
        assert_eq!(setting(&app, "appearance.typeface"), "inter");
        look(&mut app, Event::Size(Step::Larger));
        assert_eq!(app.typography.size, 14.0);
        assert_eq!(setting(&app, "appearance.textSize"), 14);
        look(&mut app, Event::Size(Step::Default));
        assert_eq!(app.typography.size, Typography::DEFAULT.size);
        for _ in 0..10 {
            look(&mut app, Event::Size(Step::Smaller));
        }
        assert_eq!(app.typography.size, 11.0, "no smaller than 11");
        look(&mut app, Event::Size(Step::Default));
        look(&mut app, Event::Family(Family::IbmPlexSans));
        assert_eq!(typography::current(), Typography::DEFAULT);
    }

    #[test]
    fn the_theme_commands_know_the_four_themes() {
        let mut app = app_with_drawing();
        look(&mut app, Event::Theme(Mode::HighContrast));
        let _ = app.update(Message::Run("view.theme.toggle"));
        assert_eq!(app.mode, Mode::Light, "every dark theme turns light");
        let _ = app.update(Message::Run("view.theme.toggle"));
        assert_eq!(app.mode, Mode::Dark);
        assert_eq!(app.checked("view.theme.dark"), Some(true));
    }

    #[test]
    fn a_custom_accent_is_read_and_a_broken_one_falls_back() {
        let mut app = app_with_drawing();
        let _ = app
            .settings
            .choose(&[("appearance.accentColor", Value::from("#ff8800"))]);
        app.apply_settings();
        assert_eq!(app.accent, Accent::Custom(0xff8800));
        let _ = app
            .settings
            .choose(&[("appearance.accentColor", Value::from("bozuk"))]);
        app.apply_settings();
        assert_eq!(app.accent, Accent::default());
    }
}
