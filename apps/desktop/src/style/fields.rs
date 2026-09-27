//! Form controls of the style windows (the web's `ui/style/designerFields.ts`):
//! the symbol designer's property forms, and the SVG editor's. A row is a
//! label over its control, a hint under it; two rows may stand side by side.
//! Numbers, texts, dashes and colours are typed as text and each keystroke
//! reports the value it gives (the preview follows while typing); the text is
//! kept as typed by the window while it differs from the value. A value that
//! may come from each object's data carries an “ƒ” switch.

use std::rc::Rc;

use iced::widget::tooltip::Position;
use iced::widget::{Id, button, column, container, row, space, text_input};
use iced::{Border, Center, Color, Element, Fill, Theme};
use kentos_native_style::StylePalette;
use kentos_native_style::color::resolve;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::color::{ColorPicker, parse_hex, to_hex};
use kentos_ui::widget::tree_view::{Check, check_box};
use kentos_ui::widget::{Choice as SelectChoice, Menu, MenuButton, Select, Tip, tip};

/// A select's choices: the value written and the name shown.
pub type Choice = (&'static str, &'static str);

/// A control's height at the default text size (the web's `--ctl-h`).
const CONTROL: f32 = 26.0;

/// A labelled row: the label on top, the control below and a hint under it (`row`).
pub fn labelled<'a, M: 'a>(
    label: impl Into<String>,
    control: impl Into<Element<'a, M>>,
    hint: Option<&str>,
) -> Element<'a, M> {
    let mut parts = column![
        label::caption(label.into()).style(style::text::muted),
        control.into()
    ]
    .spacing(4)
    .width(Fill);
    if let Some(h) = hint {
        parts = parts.push(self::hint(h));
    }
    parts.into()
}

/// Two controls side by side, sharing the width (`pair`).
pub fn pair<'a, M: 'a>(a: impl Into<Element<'a, M>>, b: impl Into<Element<'a, M>>) -> Element<'a, M> {
    row![
        container(a.into()).width(Fill),
        container(b.into()).width(Fill)
    ]
    .spacing(8)
    .into()
}

/// A hint under a control, small and muted (`sdf__hint`).
pub fn hint<'a, M: 'a>(text: &str) -> Element<'a, M> {
    label::caption(text.to_owned())
        .size(typography::caption() - 1.0)
        .style(style::text::muted)
        .into()
}

/// A group's title (Genel, Dalga).
pub fn group_title<'a, M: 'a>(text: &str) -> Element<'a, M> {
    label::caption(text.to_owned())
        .font(typography::ui_strong())
        .style(style::text::muted)
        .into()
}

/// A text field in the forms' look: `id` lets the window give it the
/// keyboard and find it; `invalid` marks what does not read as a value.
pub fn input<'a, M: Clone + 'a>(
    id: Option<Id>,
    placeholder: &str,
    value: &str,
    mono: bool,
    invalid: bool,
) -> text_input::TextInput<'a, M> {
    let mut field = text_input(placeholder, value)
        .padding([4, 7])
        .font(if mono {
            typography::mono()
        } else {
            typography::ui()
        })
        .size(typography::body())
        .style(style::field::validated(invalid))
        .width(Fill);
    if let Some(id) = id {
        field = field.id(id);
    }
    field
}

/// A unit after a field (mm, °, %, adet).
fn unit<'a, M: 'a>(text: &str) -> Element<'a, M> {
    label::caption(text.to_owned())
        .style(style::text::muted)
        .into()
}

/// A number field and its unit (`numberInput`): what is typed goes to
/// `on_input`; Enter to `on_submit` (the value's own text comes back).
pub fn number<'a, M: Clone + 'a>(
    id: Id,
    text: &str,
    unit_text: Option<&str>,
    invalid: bool,
    on_input: impl Fn(String) -> M + 'a,
    on_submit: M,
) -> Element<'a, M> {
    let field = input(Some(id), "", text, false, invalid)
        .on_input(on_input)
        .on_submit(on_submit);
    match unit_text {
        Some(u) => row![field, unit(u)].spacing(6).align_y(Center).into(),
        None => field.into(),
    }
}

/// A plain text field (`textInput`).
pub fn text<'a, M: Clone + 'a>(
    id: Id,
    value: &str,
    placeholder: &str,
    mono: bool,
    on_input: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    input(Some(id), placeholder, value, mono, false)
        .on_input(on_input)
        .into()
}

/// A drop-down of choices (`select`): the value's choice is shown, the
/// first one when the value is none of them.
pub fn select<'a, M: Clone + 'a>(
    choices: &[Choice],
    value: &str,
    on_select: impl Fn(&'static str) -> M + 'a,
) -> Element<'a, M> {
    let values: Vec<&'static str> = choices.iter().map(|(v, _)| *v).collect();
    let chosen = values.iter().position(|v| *v == value).or(Some(0));
    Select::new(
        choices.iter().map(|(_, l)| SelectChoice::new(*l)),
        chosen,
        move |i| on_select(values.get(i).copied().unwrap_or_default()),
    )
    .into()
}

/// A drop-down of choices made at run time (the library's drawings): the
/// value's choice is shown, the first one when the value is none of them.
pub fn select_owned<'a, M: Clone + 'a>(
    choices: Vec<(String, String)>,
    value: &str,
    on_select: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    let chosen = choices.iter().position(|(v, _)| v == value).or(Some(0));
    let values: Vec<String> = choices.iter().map(|(v, _)| v.clone()).collect();
    Select::new(
        choices.into_iter().map(|(_, l)| SelectChoice::new(l)),
        chosen,
        move |i| on_select(values.get(i).cloned().unwrap_or_default()),
    )
    .into()
}

/// A check box with its words, the words pressable too (`checkbox`).
pub fn check<'a, M: Clone + 'a>(
    state: Check,
    words: impl Into<String>,
    on_toggle: Option<M>,
) -> Element<'a, M> {
    let face = row![check_box(state, on_toggle.clone()), label::body(words.into())]
        .spacing(8)
        .align_y(Center);
    button(face)
        .on_press_maybe(on_toggle)
        .padding([3, 0])
        .style(style::button::ghost)
        .into()
}

/// The “ƒ” switch of a value that may come from each object's data: pressed,
/// the value is an expression.
pub fn fx<'a, M: Clone + 'a>(on: bool, press: M) -> Element<'a, M> {
    let face = container(
        label::body("ƒ")
            .font(typography::ui_strong())
            .style(move |t: &Theme| iced::widget::text::Style {
                color: Some(if on {
                    Tokens::of(t).accent
                } else {
                    Tokens::of(t).muted
                }),
            }),
    )
    .center_x(typography::scaled(CONTROL))
    .center_y(typography::scaled(CONTROL));
    tip(
        button(face)
            .on_press(press)
            .padding(0)
            .style(move |t: &Theme, status| {
                let tokens = Tokens::of(t);
                let hovered = matches!(
                    status,
                    button::Status::Hovered | button::Status::Pressed
                );
                button::Style {
                    background: Some(iced::Background::Color(if on {
                        tokens.accent.scale_alpha(0.14)
                    } else if hovered {
                        tokens.surface_hover
                    } else {
                        Color::TRANSPARENT
                    })),
                    text_color: tokens.text,
                    border: Border {
                        color: if on {
                            tokens.accent.scale_alpha(0.55)
                        } else {
                            tokens.border
                        },
                        width: 1.0,
                        radius: 3.0.into(),
                    },
                    ..button::Style::default()
                }
            }),
        Tip::new(if on {
            "Sabit değere dön"
        } else {
            "Nesnenin özniteliğinden ya da bir ifadeden al"
        }),
        Position::Left,
    )
}

/// The theme's colours a symbol may name instead of a hex value.
pub const TOKENS: [Choice; 4] = [
    ("ink", "Mürekkep (siyah / koyu temada beyaz)"),
    ("paper", "Kâğıt (beyaz / koyu temada zemin)"),
    ("fg", "Ana ön plan"),
    ("fg-dim", "İkincil ön plan"),
];

/// Whether text is a colour a symbol takes: #RRGGBB, #RRGGBBAA or a token.
pub fn is_color(text: &str) -> bool {
    let hex = text.strip_prefix('#').is_some_and(|d| {
        (d.len() == 6 || d.len() == 8) && d.chars().all(|c| c.is_ascii_hexdigit())
    });
    hex || TOKENS.iter().any(|(t, _)| *t == text)
}

/// A colour as the GPU sees it, the theme's tokens resolved.
pub fn resolved(value: &str, palette: &StylePalette) -> Option<Color> {
    parse_hex(resolve(value, palette))
}

/// “No colour”: a square crossed from corner to corner, in the danger colour.
const NONE_MARK: &str = r#"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 16 16"><path d="M2 14 14 2" stroke="currentColor" stroke-width="1.6" stroke-linecap="round" fill="none"/></svg>"#;

/// What a colour field needs besides its value.
pub struct ColorEnv<'p> {
    pub palette: &'p StylePalette,
    /// Colours already in the thing being edited, offered under the picker.
    pub recent: Vec<Color>,
}

/// A colour (`colorInput`): its swatch opens the colour picker; the text
/// takes a hex value or a theme token; ⋯ lists the tokens (and “Yok” when
/// `none` is allowed: no fill, no outline). `on_change` gets the text as
/// typed, or the colour chosen as its hex, or "" for none.
pub fn color<'a, M: Clone + 'a>(
    id: Id,
    label_text: &str,
    value: Option<&str>,
    typed: Option<&str>,
    none: bool,
    env: &ColorEnv<'_>,
    on_change: impl Fn(String) -> M + 'a,
) -> Element<'a, M> {
    let on_change: Rc<dyn Fn(String) -> M + 'a> = Rc::new(on_change);
    let shown = value.and_then(|v| resolved(v, env.palette));
    let size = typography::scaled(CONTROL);
    let face: Element<'a, M> = match shown {
        Some(c) => container(space())
            .width(size)
            .height(size)
            .style(move |t: &Theme| container::Style {
                background: Some(iced::Background::Color(c)),
                border: Border {
                    color: Tokens::of(t).muted,
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..container::Style::default()
            })
            .into(),
        None => container(icon(Icon::Svg(NONE_MARK)).size(size - 4.0).tone(Tone::Danger))
            .center_x(size)
            .center_y(size)
            .style(|t: &Theme| container::Style {
                border: Border {
                    color: Tokens::of(t).muted,
                    width: 1.0,
                    radius: 3.0.into(),
                },
                ..container::Style::default()
            })
            .into(),
    };
    // The picker keeps an alpha only when one is chosen (#RRGGBBAA).
    let pick = on_change.clone();
    let picker = ColorPicker::new(shown.unwrap_or(Color::BLACK), move |c| pick(to_hex(c)))
        .alpha()
        .recent(env.recent.iter().copied())
        .anchor(face);
    let text = typed.map_or_else(|| value.unwrap_or("").to_owned(), str::to_owned);
    let typed_in = on_change.clone();
    let field = input(
        Some(id),
        if none { "yok" } else { "#000000" },
        &text,
        true,
        false,
    )
    .on_input(move |t| typed_in(t));
    let palette = env.palette.clone();
    let menu_change = on_change.clone();
    let menu = move || {
        let mut m = Menu::new();
        for (token, name) in TOKENS {
            m = m
                .item(name, menu_change(token.to_owned()))
                .swatch(resolved(token, &palette).unwrap_or(Color::BLACK));
        }
        if none {
            m = m.separator().item("Yok", menu_change(String::new()));
        }
        m
    };
    let more = tip(
        MenuButton::new(
            container(icon(Icon::More).size(14.0).tone(Tone::Muted))
                .center_x(typography::scaled(CONTROL - 4.0))
                .center_y(size),
            menu,
        ),
        Tip::new(format!("{label_text}: tema renkleri")),
        Position::Left,
    );
    row![Element::from(picker), field, more]
        .spacing(6)
        .align_y(Center)
        .width(Fill)
        .into()
}
