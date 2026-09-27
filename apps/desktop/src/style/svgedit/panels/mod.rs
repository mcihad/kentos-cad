//! The SVG editor's right column, in tabs (the web's `svgProps.ts`):
//! Özellikler (with nothing selected the canvas; with a selection its paint,
//! stroke and look, box, geometry, path operations, order and selection
//! helpers; the node tool's box and the polygon tool's settings on top while
//! they are in use), Hizala, Dönüştür and Dizi. The controls shared by the
//! tabs live here: number fields that apply while typing, segmented choices,
//! and icon buttons with their tips.

mod align;
mod array;
mod canvas;
mod nodes;
pub(crate) mod props;
mod style;
mod transform;

use std::fmt::Display;
use std::sync::Arc;

use iced::widget::tooltip::Position;
use iced::widget::{Id, button, column, container, row};
use iced::{Border, Center, Element, Fill, Theme};
use kentos_ui::icon::{Icon, icon};
use kentos_ui::label;
use kentos_ui::style as ui_style;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Segmented, Tip, tip};

use super::state::{SvgEditor, Tab};
use super::{Event, change, ev};
use crate::app::Message;
use crate::style::fields;

/// A choice of a segmented control or a select: its value and its name.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Opt(pub &'static str, pub &'static str);

impl Display for Opt {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.1)
    }
}

/// A segmented control (`liveSeg`): the value's segment lit (none when mixed).
pub fn seg<'a>(
    options: &[Opt],
    value: Option<&str>,
    hints: &[&str],
    on: impl Fn(&'static str) -> Message + 'a,
) -> Element<'a, Message> {
    let chosen = options
        .iter()
        .copied()
        .find(|o| Some(o.0) == value)
        .unwrap_or(Opt("", ""));
    let mut s = Segmented::new(options.iter().copied(), chosen, move |o: Opt| on(o.0)).width(Fill);
    if !hints.is_empty() {
        s = s.hints(hints.iter().copied());
    }
    s.into()
}

/// A number field's step (↑ ↓, ×10 with Shift) and bounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct NumSpec {
    pub step: f64,
    pub min: f64,
    pub max: f64,
}

pub const fn spec(step: f64, min: f64, max: f64) -> NumSpec {
    NumSpec { step, min, max }
}

pub const ANY: NumSpec = spec(1.0, f64::NEG_INFINITY, f64::INFINITY);

/// What a number field does with a value.
pub type Apply = Arc<dyn Fn(&mut SvgEditor, f64) + Send + Sync>;

/// A number field the view made: its bounds and what it applies (for ↑ and ↓).
#[derive(Clone)]
pub struct FieldDef {
    pub spec: NumSpec,
    pub apply: Apply,
}

pub fn field_id(key: &str) -> Id {
    Id::from(format!("svge:{key}"))
}

/// A number as the field writes it: six decimals at most (`fmt`).
pub fn text_of(v: f64) -> String {
    kentos_native_style::classify::rounded(v, 6)
}

/// What a field's text reads as: a comma for the point, nothing for a blank field.
pub fn parse(text: &str) -> Option<f64> {
    let t = kentos_processing::text::js_trim(text);
    if t.is_empty() {
        return None;
    }
    let v = kentos_native_style::classify::js_number(&t.replacen(',', ".", 1));
    v.is_finite().then_some(v)
}

impl SvgEditor {
    /// A number typed into a field: kept as typed, applied when it reads as a value.
    pub fn num_typed(&mut self, key: &str, text: &str, spec: NumSpec, apply: &Apply) {
        self.typed.insert(key.to_owned(), text.to_owned());
        if let Some(v) = parse(text) {
            apply(self, v.clamp(spec.min, spec.max));
        }
    }

    /// ↑ or ↓ in a number field (the field's `keydown`).
    pub fn num_step(&mut self, key: &str, up: bool, shift: bool) -> bool {
        let Some(def) = self.fields.borrow().get(key).cloned() else {
            return false;
        };
        let now = self
            .typed
            .get(key)
            .cloned()
            .or_else(|| self.shown.borrow().get(key).cloned())
            .unwrap_or_default();
        let step = def.spec.step * if shift { 10.0 } else { 1.0 };
        let v = parse(&now).unwrap_or(0.0) + if up { step } else { -step };
        let v = (kentos_native_style::classify::js_round(v * 1e6) / 1e6)
            .clamp(def.spec.min, def.spec.max);
        self.typed.insert(key.to_owned(), text_of(v));
        (def.apply)(self, v);
        true
    }
}

/// A number field with its unit: shows `value` unless something else is typed; each keystroke applies.
pub fn num_bare<'a>(
    ed: &SvgEditor,
    key: &str,
    value: f64,
    unit: Option<&str>,
    spec: NumSpec,
    apply: impl Fn(&mut SvgEditor, f64) + Send + Sync + 'static,
) -> Element<'a, Message> {
    let apply: Apply = Arc::new(apply);
    let shown = text_of(value);
    ed.fields.borrow_mut().insert(
        key.to_owned(),
        FieldDef {
            spec,
            apply: apply.clone(),
        },
    );
    ed.shown.borrow_mut().insert(key.to_owned(), shown.clone());
    let text = ed.typed.get(key).cloned().unwrap_or(shown);
    let invalid = !kentos_processing::text::js_trim(&text).is_empty() && parse(&text).is_none();
    let k = key.to_owned();
    fields::number(
        field_id(key),
        &text,
        unit,
        invalid,
        move |t| {
            let k = k.clone();
            let apply = apply.clone();
            change(move |ed| ed.num_typed(&k, &t, spec, &apply))
        },
        ev(Event::Settle(key.to_owned())),
    )
}

/// A labelled number field (`row(label, numberInput(…))`).
pub fn num<'a>(
    ed: &SvgEditor,
    key: &str,
    label_text: &str,
    value: f64,
    unit: Option<&str>,
    spec: NumSpec,
    apply: impl Fn(&mut SvgEditor, f64) + Send + Sync + 'static,
) -> Element<'a, Message> {
    fields::labelled(label_text, num_bare(ed, key, value, unit, spec, apply), None)
}

/// A check box with its words (`checkbox`).
pub fn check<'a>(on: bool, words: &str, f: impl Fn(&mut SvgEditor, bool) + Send + Sync + 'static) -> Element<'a, Message> {
    let f = Arc::new(f);
    fields::check(
        if on {
            kentos_ui::widget::tree_view::Check::Checked
        } else {
            kentos_ui::widget::tree_view::Check::Unchecked
        },
        words,
        Some(change(move |ed| f(ed, !on))),
    )
}

/// An icon button of the panels' action rows, with its tip; none: dimmed.
pub fn act<'a>(glyph: Icon, tip_text: &str, press: Option<Message>) -> Element<'a, Message> {
    act_sized(glyph, tip_text, press, false, 16.0)
}

/// An icon button that shows whether it is on.
pub fn act_sized<'a>(
    glyph: Icon,
    tip_text: &str,
    press: Option<Message>,
    pressed: bool,
    size: f32,
) -> Element<'a, Message> {
    let side = typography::scaled(size + 12.0);
    let face = container(icon(glyph).size(size)).center_x(side).center_y(side);
    tip(
        button(face)
            .padding(0)
            .style(ui_style::button::tool(pressed))
            .on_press_maybe(press),
        Tip::new(tip_text.to_owned()),
        Position::Top,
    )
}

/// A small text button of the panels.
pub fn small<'a>(words: &str, press: Option<Message>) -> Element<'a, Message> {
    button(label::body(words.to_owned()))
        .padding([3, 10])
        .style(ui_style::button::secondary)
        .on_press_maybe(press)
        .into()
}

/// A row of action buttons that wraps on a narrow column.
pub fn acts<'a>(items: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    iced::widget::Row::with_children(items)
        .spacing(4)
        .wrap()
        .vertical_spacing(4)
        .into()
}

/// A group: a thin rule above, its title, its rows.
pub fn group<'a>(title: &str, rows: Vec<Element<'a, Message>>) -> Element<'a, Message> {
    let mut c = column![fields::group_title(title)].spacing(8);
    for r in rows {
        c = c.push(r);
    }
    container(c)
        .padding(iced::Padding {
            top: 10.0,
            ..iced::Padding::ZERO
        })
        .width(Fill)
        .style(|t: &Theme| container::Style {
            border: Border {
                color: Tokens::of(t).border,
                width: 0.0,
                radius: 0.0.into(),
            },
            ..container::Style::default()
        })
        .into()
}

/// A panel's title with the rule under it (`svgp__title`).
pub fn title<'a>(words: String, extra: Option<Element<'a, Message>>) -> Element<'a, Message> {
    let mut r = row![label::strong(words)].spacing(8).align_y(Center);
    if let Some(e) = extra {
        r = r.push(e);
    }
    column![r, kentos_ui::widget::horizontal_divider()]
        .spacing(8)
        .into()
}

/// The tabs over the column.
fn tabs<'a>(ed: &SvgEditor) -> Element<'a, Message> {
    let tab = |t: Tab, words: &'static str, keys: &'static str| -> Element<'a, Message> {
        let on = ed.ui.tab == t;
        let face = column![
            label::body(words).style(move |th: &Theme| iced::widget::text::Style {
                color: Some(if on {
                    Tokens::of(th).accent
                } else {
                    Tokens::of(th).muted
                }),
            }),
            container(iced::widget::space())
                .height(2)
                .width(Fill)
                .style(move |th: &Theme| container::Style {
                    background: on.then(|| iced::Background::Color(Tokens::of(th).accent)),
                    ..container::Style::default()
                }),
        ]
        .spacing(5)
        .width(iced::Length::Shrink);
        let b = button(face)
            .padding([6, 8])
            .style(ui_style::button::tab)
            .on_press(change(move |ed| {
                ed.ui.tab = t;
                ed.touch();
            }));
        if keys.is_empty() {
            b.into()
        } else {
            tip(b, Tip::new(format!("{words} ({keys})")), Position::Bottom)
        }
    };
    row![
        tab(Tab::Props, "Özellikler", ""),
        tab(Tab::Align, "Hizala", "Ctrl+Shift+A"),
        tab(Tab::Transform, "Dönüştür", "Ctrl+Shift+M"),
        tab(Tab::Array, "Dizi", ""),
    ]
    .spacing(2)
    .into()
}

/// The right column.
pub fn panel<'a>(ed: &'a SvgEditor) -> Element<'a, Message> {
    ed.fields.borrow_mut().clear();
    ed.shown.borrow_mut().clear();
    let count = ed.chosen().len();
    let body = match ed.ui.tab {
        Tab::Align => align::tab(ed, count),
        Tab::Transform => transform::tab(ed, count),
        Tab::Array => array::tab(ed, count),
        Tab::Props => props::tab(ed),
    };
    column![tabs(ed), kentos_ui::widget::horizontal_divider(), body]
        .spacing(10)
        .width(Fill)
        .into()
}
