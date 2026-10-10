//! Etiketler's style form (docs/adr/0212 §4; the web's `ui/labels/labelForm.ts`):
//! one label style's fields in five tabs, Metin, Yerleşim, Biçim, Sığdırma and
//! Öncelik. Each control shows the style's value (absent: the engine's default,
//! said in its placeholder) and writes a change; an emptied number takes its
//! field away. What is typed stays in the field until it reads; one that does
//! not read (or is out of its range) is marked and not written.

use std::collections::{HashMap, HashSet};
use std::fmt;

use iced::widget::{Column, Row, button, column, container, row, text_editor, text_input};
use iced::{Center, Element, Fill, Theme};
use kentos_contracts::{
    AreaLabelMode, CalloutKind, LabelAbbreviate, LabelAlign, LabelBackground, LabelCallout,
    LabelHalo, LabelOverlap, LabelPlacement, LabelPosition, LabelShadow, LabelShape, LabelStack,
    LabelStyle, LabelWord, LineLabelMode, PointLabelMode, StackMode,
};
use kentos_ui::icon::icon;
use kentos_ui::theme::{Tokens, metrics, typography};
use kentos_ui::widget::color::{ColorPicker, parse_hex, to_hex};
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use super::{Event, Field, msg};
use crate::app::Message;
use crate::exchange::words;
use crate::icons::from_web;

/// The style's five tabs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Text,
    Place,
    Look,
    Fit,
    Order,
}

/// The tabs in their order, with their words (the web's `LABEL_TABS`).
pub const TABS: [(Tab, &str); 5] = [
    (Tab::Text, "Metin"),
    (Tab::Place, "Yerleşim"),
    (Tab::Look, "Biçim"),
    (Tab::Fit, "Sığdırma"),
    (Tab::Order, "Öncelik"),
];

/// A field that is typed in: what is typed shows until it reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    /// The selected class's name and condition (the window's).
    Name,
    When,
    /// The obstacle's weight (the window's).
    Weight,
    Text,
    Template,
    Size,
    Grow,
    MaxSize,
    Distance,
    Repeat,
    MaxAngle,
    HaloWidth,
    Padding,
    ShadowDx,
    ShadowDy,
    ShadowOpacity,
    CalloutWidth,
    CalloutMin,
    StackChars,
    StackAt,
    Shrink,
    Priority,
    Duplicates,
    MinFeature,
    MinScale,
    MaxScale,
}

/// A colour of the style.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Paint {
    Text,
    Halo,
    Fill,
    Stroke,
    Callout,
}

/// A change made with a control that is not typed in.
#[derive(Clone, Debug)]
pub enum Edit {
    /// Metnin kaynağı: İfade (true) or Şablon.
    Expression(bool),
    Bold(bool),
    Italic(bool),
    /// A colour; none: the default the control names.
    Paint(Paint, Option<String>),
    Align(LabelAlign),
    Point(PointLabelMode),
    Line(LineLabelMode),
    Area(AreaLabelMode),
    Position(LabelPosition),
    Curved(bool),
    MergeLines(bool),
    Inside(bool),
    Outside(bool),
    Background(Option<LabelShape>),
    Shadow(bool),
    Callout(Option<CalloutKind>),
    Stack(Option<StackMode>),
    /// Kısaltma: none, gerekirse (false) or her zaman (true).
    Abbreviate(Option<bool>),
    Overlap(LabelOverlap),
}

/// What the form shows: the style, what is typed, what does not read and the dictionary's editor.
pub struct Host<'a> {
    pub style: &'a LabelStyle,
    pub typed: &'a HashMap<Key, String>,
    pub invalid: &'a HashSet<Key>,
    pub dictionary: &'a text_editor::Content,
}

/// A change of a control applied to the style (the web's patches, field for field).
pub fn apply(s: &mut LabelStyle, edit: Edit) {
    match edit {
        Edit::Expression(on) => {
            s.text = if on {
                Some(s.text.take().unwrap_or_else(|| "$etiket".to_owned()))
            } else {
                None
            };
        }
        Edit::Bold(on) => s.weight = on.then_some(600),
        Edit::Italic(on) => s.italic = on.then_some(true),
        Edit::Paint(paint, c) => match paint {
            Paint::Text => s.color = c,
            Paint::Halo => {
                s.halo = Some(LabelHalo {
                    width: s.halo.as_ref().map_or(1.5, |h| h.width),
                    color: c,
                });
            }
            Paint::Fill => {
                if let Some(b) = &mut s.background {
                    b.fill = c;
                }
            }
            Paint::Stroke => {
                if let Some(b) = &mut s.background {
                    b.stroke = c;
                }
            }
            Paint::Callout => {
                if let Some(k) = &mut s.callout {
                    k.color = c;
                }
            }
        },
        Edit::Align(a) => s.align = (a != LabelAlign::Center).then_some(a),
        Edit::Point(m) => s.point = Some(m),
        Edit::Line(m) => s.line = Some(m),
        Edit::Area(m) => s.area = Some(m),
        Edit::Position(p) => s.position = Some(p),
        Edit::Curved(on) => s.curved = on.then_some(true),
        Edit::MergeLines(on) => s.merge_lines = on.then_some(true),
        Edit::Inside(on) => s.inside = on.then_some(true),
        Edit::Outside(on) => s.outside = on.then_some(true),
        Edit::Background(shape) => {
            s.background = shape.map(|shape| match s.background.take() {
                Some(b) => LabelBackground { shape, ..b },
                None => LabelBackground {
                    shape,
                    fill: Some("paper".to_owned()),
                    stroke: None,
                    padding: None,
                },
            });
        }
        Edit::Shadow(on) => {
            s.shadow = on.then(|| LabelShadow {
                dx: 1.0,
                dy: -1.0,
                color: None,
                opacity: Some(0.5),
            });
        }
        Edit::Callout(kind) => {
            s.callout = kind.map(|kind| match s.callout.take() {
                Some(c) => LabelCallout { kind, ..c },
                None => LabelCallout {
                    kind,
                    color: None,
                    width: None,
                    min_length: None,
                },
            });
        }
        Edit::Stack(mode) => {
            s.stack = mode.map(|mode| match s.stack.take() {
                Some(k) => LabelStack { mode, ..k },
                None => LabelStack {
                    mode,
                    chars: 12,
                    at: None,
                },
            });
        }
        Edit::Abbreviate(always) => {
            s.abbreviate = always.map(|always| {
                let words = s
                    .abbreviate
                    .take()
                    .map(|a| a.words)
                    .filter(|w| !w.is_empty())
                    .unwrap_or_else(|| {
                        vec![LabelWord {
                            word: "Caddesi".to_owned(),
                            short: "Cd.".to_owned(),
                        }]
                    });
                LabelAbbreviate {
                    always: always.then_some(true),
                    words,
                }
            });
        }
        Edit::Overlap(o) => s.overlap = (o != LabelOverlap::Never).then_some(o),
    }
}

/// A number as the field shows it.
fn shown(v: f64) -> String {
    format!("{v}")
}

/// A typed number read: empty is none; one that is no number, or out of `[lo, hi]`, does not read.
fn read(text: &str, lo: f64, hi: f64) -> Result<Option<f64>, ()> {
    let t = text.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    match t.parse::<f64>() {
        Ok(v) if v.is_finite() && (lo..=hi).contains(&v) => Ok(Some(v)),
        _ => Err(()),
    }
}

/// A typed field's text as the style holds it.
pub fn value_text(s: &LabelStyle, key: Key) -> String {
    let n = |v: Option<f64>| v.map(shown).unwrap_or_default();
    match key {
        Key::Name | Key::When | Key::Weight => String::new(),
        Key::Text => s.text.clone().unwrap_or_default(),
        Key::Template => s.template.clone().unwrap_or_default(),
        Key::Size => shown(s.size),
        Key::Grow => n(s.grow),
        Key::MaxSize => n(s.max_size),
        Key::Distance => n(s.distance),
        Key::Repeat => n(s.repeat),
        Key::MaxAngle => n(s.max_angle),
        Key::HaloWidth => n(s.halo.as_ref().map(|h| h.width)),
        Key::Padding => n(s.background.as_ref().and_then(|b| b.padding)),
        Key::ShadowDx => n(s.shadow.as_ref().map(|h| h.dx)),
        Key::ShadowDy => n(s.shadow.as_ref().map(|h| h.dy)),
        Key::ShadowOpacity => n(s.shadow.as_ref().and_then(|h| h.opacity)),
        Key::CalloutWidth => n(s.callout.as_ref().and_then(|c| c.width)),
        Key::CalloutMin => n(s.callout.as_ref().and_then(|c| c.min_length)),
        Key::StackChars => s
            .stack
            .as_ref()
            .map(|k| k.chars.to_string())
            .unwrap_or_default(),
        Key::StackAt => s
            .stack
            .as_ref()
            .and_then(|k| k.at.clone())
            .unwrap_or_default(),
        Key::Shrink => n(s.shrink.map(|k| (k * 100.0).round())),
        Key::Priority => s.priority.map(|p| p.to_string()).unwrap_or_default(),
        Key::Duplicates => n(s.duplicates),
        Key::MinFeature => n(s.min_feature_px),
        Key::MinScale => n(s.min_scale),
        Key::MaxScale => n(s.max_scale),
    }
}

/// A typed field written to the style; whether it read (one that did not is marked, the style kept).
pub fn typed(s: &mut LabelStyle, key: Key, text: &str) -> bool {
    let number = |lo: f64, hi: f64| read(text, lo, hi);
    let words = || (!text.trim().is_empty()).then(|| text.to_owned());
    let result: Result<(), ()> = (|| {
        match key {
            Key::Name | Key::When | Key::Weight => {}
            Key::Text => s.text = Some(words().unwrap_or_default()),
            Key::Template => s.template = words(),
            Key::Size => s.size = number(1.0, 200.0)?.unwrap_or(10.0),
            Key::Grow => s.grow = number(0.0, 1000.0)?,
            Key::MaxSize => s.max_size = number(1.0, 200.0)?,
            Key::Distance => s.distance = number(0.0, 500.0)?,
            Key::Repeat => s.repeat = number(20.0, 100_000.0)?,
            Key::MaxAngle => s.max_angle = number(5.0, 90.0)?,
            Key::HaloWidth => {
                let color = s.halo.as_ref().and_then(|h| h.color.clone());
                s.halo = number(0.0, 10.0)?.map(|width| LabelHalo { width, color });
            }
            Key::Padding => {
                let v = number(0.0, 50.0)?;
                if let Some(b) = &mut s.background {
                    b.padding = v;
                }
            }
            Key::ShadowDx | Key::ShadowDy => {
                let v = number(-50.0, 50.0)?.unwrap_or(0.0);
                if let Some(h) = &mut s.shadow {
                    if key == Key::ShadowDx {
                        h.dx = v;
                    } else {
                        h.dy = v;
                    }
                }
            }
            Key::ShadowOpacity => {
                let v = number(0.0, 1.0)?;
                if let Some(h) = &mut s.shadow {
                    h.opacity = v;
                }
            }
            Key::CalloutWidth => {
                let v = number(0.1, 10.0)?;
                if let Some(c) = &mut s.callout {
                    c.width = v;
                }
            }
            Key::CalloutMin => {
                let v = number(0.0, 1000.0)?;
                if let Some(c) = &mut s.callout {
                    c.min_length = v;
                }
            }
            Key::StackChars => {
                let v = number(2.0, 500.0)?;
                if let Some(k) = &mut s.stack {
                    // A whole number of letters.
                    k.chars = v.map_or(12, |v| v.round() as u32);
                }
            }
            Key::StackAt => {
                let v = words();
                if let Some(k) = &mut s.stack {
                    k.at = v;
                }
            }
            Key::Shrink => {
                s.shrink = number(50.0, 100.0)?
                    .filter(|v| *v < 100.0)
                    .map(|v| v / 100.0);
            }
            Key::Priority => s.priority = number(0.0, 10.0)?.map(|v| v.round() as u8),
            Key::Duplicates => s.duplicates = number(1.0, 10_000.0)?,
            Key::MinFeature => s.min_feature_px = number(0.0, 100_000.0)?,
            Key::MinScale => s.min_scale = number(0.0, 1e9)?,
            Key::MaxScale => s.max_scale = number(0.0, 1e9)?,
        }
        Ok(())
    })();
    result.is_ok()
}

/// The dictionary as the window writes it: a word and its short form a line, `Caddesi = Cd.`.
pub fn dictionary_text(s: &LabelStyle) -> String {
    s.abbreviate
        .as_ref()
        .map(|a| {
            a.words
                .iter()
                .map(|w| format!("{} = {}", w.word, w.short))
                .collect::<Vec<_>>()
                .join("\n")
        })
        .unwrap_or_default()
}

/// A dictionary's lines read back; a line without “=” is left out (the web's `readDictionary`).
pub fn read_dictionary(text: &str) -> Vec<LabelWord> {
    text.split('\n')
        .filter_map(|line| {
            let (word, short) = line.split_once('=')?;
            let (word, short) = (word.trim(), short.trim());
            (!word.is_empty() || !short.is_empty()).then(|| LabelWord {
                word: word.to_owned(),
                short: short.to_owned(),
            })
        })
        .collect()
}

/// The dictionary's text written to the style: the words it reads (none read: the dictionary stays
/// empty and the window says so, so the row does not go away while it is typed).
pub fn dictionary(s: &mut LabelStyle, text: &str) {
    if let Some(a) = &mut s.abbreviate {
        a.words = read_dictionary(text);
    }
}

/// An option of a segmented choice: its index and its words.
#[derive(Clone, Copy, PartialEq)]
struct Opt(usize, &'static str);

impl fmt::Display for Opt {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.1)
    }
}

/// A segmented choice of `options`; the chosen one's index goes to `on`.
pub(super) fn choice<'a>(
    options: &[&'static str],
    chosen: usize,
    on: impl Fn(usize) -> Message + 'a,
) -> Segmented<'a, Message> {
    let all: Vec<Opt> = options.iter().enumerate().map(|(i, w)| Opt(i, w)).collect();
    let selected = all.get(chosen).copied().unwrap_or(all[0]);
    Segmented::new(all, selected, move |o| on(o.0))
}

/// A row: its name, its control and a hint under it (the web's `.lbl-row`).
pub(super) fn line<'a>(
    name: &'a str,
    name_width: f32,
    control: impl Into<Element<'a, Message>>,
    hint: Option<&'a str>,
) -> Element<'a, Message> {
    let mut right = Column::new().spacing(2).width(Fill).push(control.into());
    if let Some(hint) = hint {
        right = right.push(label::caption(hint));
    }
    row![
        container(label::muted(name))
            .width(typography::scaled(name_width))
            .height(metrics::control())
            .align_y(Center),
        right,
    ]
    .spacing(12)
    .into()
}

/// The row's name column (the web's 118 px).
const NAME: f32 = 118.0;

fn row_of<'a>(
    name: &'a str,
    control: impl Into<Element<'a, Message>>,
    hint: Option<&'a str>,
) -> Element<'a, Message> {
    line(name, NAME, control, hint)
}

impl<'a> Host<'a> {
    /// A typed field: what is typed, else the style's value; marked when it does not read.
    pub(super) fn input(
        &self,
        key: Key,
        placeholder: &str,
        width: Option<f32>,
    ) -> Element<'a, Message> {
        let text = self
            .typed
            .get(&key)
            .cloned()
            .unwrap_or_else(|| value_text(self.style, key));
        let input = text_input(placeholder, &text)
            .on_input(move |t| msg(Event::Type(key, t)))
            .padding([5, 8])
            .size(typography::body())
            .style(style::field::validated(self.invalid.contains(&key)));
        let input = match width {
            Some(w) => input.width(typography::scaled(w)),
            None => input.width(Fill),
        };
        kentos_ui::widget::focus_ring(input).into()
    }

    /// A number field with its unit.
    fn number(&self, key: Key, placeholder: &str, unit: Option<&'a str>) -> Element<'a, Message> {
        let mut r =
            Row::new()
                .spacing(4)
                .align_y(Center)
                .push(self.input(key, placeholder, Some(72.0)));
        if let Some(unit) = unit {
            r = r.push(label::caption(unit));
        }
        r.into()
    }
}

fn check<'a>(on: bool, words: &'a str, edit: impl Fn(bool) -> Edit) -> Element<'a, Message> {
    words::check(on, words, Some(msg(Event::Edit(edit(!on)))))
}

/// The theme's colours a label may name, with their words (the web's `THEME`).
const THEME: [(&str, &str); 4] = [
    ("fg", "Ana yazı"),
    ("fg-dim", "Soluk"),
    ("paper", "Kâğıt"),
    ("ink", "Mürekkep"),
];
/// The colour a custom choice starts from.
const CUSTOM: &str = "#E5484D";

/// A colour: the default (`none` names it), a theme's or a custom one (`#RRGGBB`, the picker beside).
fn colour<'a>(paint: Paint, value: Option<&str>, none: &'a str) -> Element<'a, Message> {
    let custom = value.is_some_and(|v| v.starts_with('#'));
    let mut choices = vec![Choice::new(none)];
    choices.extend(THEME.iter().map(|(_, name)| Choice::new(*name)));
    choices.push(Choice::new("Özel…"));
    let chosen = match value {
        None => Some(0),
        Some(_) if custom => Some(THEME.len() + 1),
        Some(v) => THEME.iter().position(|(t, _)| *t == v).map(|i| i + 1),
    };
    let kept = value.map(str::to_owned);
    let select = Select::new(choices, chosen, move |i| {
        let c = match i {
            0 => None,
            i if i <= THEME.len() => Some(THEME[i - 1].0.to_owned()),
            _ => Some(
                kept.clone()
                    .filter(|v| v.starts_with('#'))
                    .unwrap_or_else(|| CUSTOM.to_owned()),
            ),
        };
        msg(Event::Edit(Edit::Paint(paint, c)))
    });
    let mut r = Row::new()
        .spacing(4)
        .align_y(Center)
        .push(container(select).width(typography::scaled(132.0)));
    if let Some(c) = value.filter(|_| custom).and_then(parse_hex) {
        let size = typography::scaled(22.0);
        let face = container(iced::widget::space())
            .width(size)
            .height(size)
            .style(move |t: &Theme| container::Style {
                background: Some(iced::Background::Color(c)),
                border: iced::Border {
                    color: Tokens::of(t).muted,
                    width: 1.0,
                    radius: kentos_ui::theme::shape::radius(3.0).into(),
                },
                ..container::Style::default()
            });
        let picker = ColorPicker::new(c, move |c| {
            msg(Event::Edit(Edit::Paint(paint, Some(to_hex(c)))))
        })
        .anchor(face);
        r = r.push(picker);
    }
    r.into()
}

/// The old placement's modes when the style names none (docs/adr/0212 §3.3).
fn modes(s: &LabelStyle) -> (PointLabelMode, LineLabelMode, AreaLabelMode) {
    let (point, line, area) = match s.placement {
        LabelPlacement::Center => (
            PointLabelMode::Center,
            LineLabelMode::Horizontal,
            AreaLabelMode::Horizontal,
        ),
        LabelPlacement::Corner => (
            PointLabelMode::Center,
            LineLabelMode::Horizontal,
            AreaLabelMode::Corner,
        ),
        LabelPlacement::Beside => (
            PointLabelMode::Around,
            LineLabelMode::Horizontal,
            AreaLabelMode::Horizontal,
        ),
        LabelPlacement::Along => (
            PointLabelMode::Around,
            LineLabelMode::Parallel,
            AreaLabelMode::Perimeter,
        ),
    };
    (
        s.point.unwrap_or(point),
        s.line.unwrap_or(line),
        s.area.unwrap_or(area),
    )
}

/// Metin: the text (a template or an expression), its size, weight, slant, colour and alignment.
fn text_tab<'a>(h: &Host<'a>) -> Vec<Element<'a, Message>> {
    let s = h.style;
    let expression = s.text.is_some();
    let source = choice(&["Şablon", "İfade"], usize::from(expression), |i| {
        msg(Event::Edit(Edit::Expression(i == 1)))
    });
    let words_row = if expression {
        let builder = kentos_ui::widget::tip(
            button(icon(from_web(Some("expression"))).size(16.0))
                .style(style::button::ghost)
                .padding(4)
                .on_press(msg(Event::Builder(Field::Text))),
            kentos_ui::widget::Tip::new("İfade oluşturucu…"),
            iced::widget::tooltip::Position::Top,
        );
        row_of(
            "İfade",
            row![h.input(Key::Text, "Ada || '/' || Parsel", None), builder]
                .spacing(4)
                .align_y(Center),
            Some("İfadeyle seç’in dili; $etiket nesnenin etiketi. $sıra ve $ölçek kullanılamaz."),
        )
    } else {
        row_of(
            "Şablon",
            h.input(Key::Template, "{label}", None),
            Some("{label}: nesnenin etiketi; {Ad} gibi öznitelikler de yazılır."),
        )
    };
    let align = match s.align.unwrap_or(LabelAlign::Center) {
        LabelAlign::Left => 0,
        LabelAlign::Center => 1,
        LabelAlign::Right => 2,
    };
    vec![
        row_of("Kaynak", source, None),
        words_row,
        row_of("Boy", h.number(Key::Size, "", Some("px")), None),
        row_of(
            "Büyüme",
            h.number(Key::Grow, "0", None),
            Some("Yakınlaştıkça her px/m için boya eklenen."),
        ),
        row_of(
            "En büyük boy",
            h.number(Key::MaxSize, "sınırsız", Some("px")),
            None,
        ),
        row_of(
            "Yazı",
            row![
                check(s.weight.unwrap_or(500) >= 600, "Kalın", Edit::Bold),
                check(s.italic == Some(true), "Eğik", Edit::Italic),
            ]
            .spacing(16),
            None,
        ),
        row_of(
            "Renk",
            colour(Paint::Text, s.color.as_deref(), "Etiket rengi"),
            None,
        ),
        row_of(
            "Hiza",
            choice(&["Sol", "Orta", "Sağ"], align, |i| {
                msg(Event::Edit(Edit::Align(
                    [LabelAlign::Left, LabelAlign::Center, LabelAlign::Right][i],
                )))
            }),
            Some("Birden çok satırlı (yığılmış) etiketin satırları."),
        ),
    ]
}

const POINTS: [PointLabelMode; 2] = [PointLabelMode::Around, PointLabelMode::Center];
const LINES: [LineLabelMode; 4] = [
    LineLabelMode::Parallel,
    LineLabelMode::Curved,
    LineLabelMode::Horizontal,
    LineLabelMode::Contour,
];
const AREAS: [AreaLabelMode; 6] = [
    AreaLabelMode::Horizontal,
    AreaLabelMode::Free,
    AreaLabelMode::Perimeter,
    AreaLabelMode::Boundary,
    AreaLabelMode::Parcel,
    AreaLabelMode::Corner,
];
const POSITIONS: [LabelPosition; 4] = [
    LabelPosition::On,
    LabelPosition::Above,
    LabelPosition::Below,
    LabelPosition::Sides,
];

fn index_of<T: PartialEq>(all: &[T], v: &T) -> usize {
    all.iter().position(|x| x == v).unwrap_or(0)
}

/// Yerleşim: where points', lines' and areas' labels go, their side, distance, repetition and the line's bends.
fn place_tab<'a>(h: &Host<'a>) -> Vec<Element<'a, Message>> {
    let s = h.style;
    let (point, line_mode, area) = modes(s);
    let position = s.position.unwrap_or(if area == AreaLabelMode::Boundary {
        LabelPosition::Above
    } else {
        LabelPosition::On
    });
    vec![
        row_of(
            "Nokta",
            choice(
                &["Çevresinde", "Üstünde"],
                index_of(&POINTS, &point),
                |i| msg(Event::Edit(Edit::Point(POINTS[i]))),
            ),
            Some("Çevresinde: sağ üst, sol üst … sekiz yerden boş olan."),
        ),
        row_of(
            "Çizgi",
            choice(
                &["Paralel", "Kıvrık", "Yatay", "Eş yükselti"],
                index_of(&LINES, &line_mode),
                |i| msg(Event::Edit(Edit::Line(LINES[i]))),
            ),
            Some("Eş yükselti: kıvrık, üstü yokuş yukarı, kot çoklu çizginin ilk kotu."),
        ),
        row_of(
            "Alan",
            choice(
                &["Yatay", "Eğik", "Çevre", "Sınır", "Parsel", "Köşe"],
                index_of(&AREAS, &area),
                |i| msg(Event::Edit(Edit::Area(AREAS[i]))),
            ),
            Some(
                "Parsel: yalnız içine; sığmazsa yığar, kısaltır, küçültür. Sınır: kenar boyunca içeride.",
            ),
        ),
        row_of(
            "Konum",
            choice(
                &["Üstünde", "Üstte", "Altta", "İki yanda"],
                index_of(&POSITIONS, &position),
                |i| msg(Event::Edit(Edit::Position(POSITIONS[i]))),
            ),
            Some("Alanın çevresinde üstte içeri, altta dışarı demektir."),
        ),
        row_of("Uzaklık", h.number(Key::Distance, "2", Some("px")), None),
        row_of(
            "Yineleme",
            h.number(Key::Repeat, "yok", Some("px")),
            Some("Uzun çizgide her bu kadar pikselde bir etiket."),
        ),
        row_of("Harf açısı", h.number(Key::MaxAngle, "25", Some("°")), None),
        row_of(
            "Seçenekler",
            column![
                row![
                    container(check(
                        s.curved == Some(true),
                        "Çevre ve sınırda kıvrık",
                        Edit::Curved
                    ))
                    .width(Fill),
                    container(check(
                        s.merge_lines == Some(true),
                        "Bağlı çizgileri birleştir",
                        Edit::MergeLines
                    ))
                    .width(Fill),
                ]
                .spacing(16),
                row![
                    container(check(
                        s.inside == Some(true),
                        "Yalnız içine sığarsa",
                        Edit::Inside
                    ))
                    .width(Fill),
                    container(check(
                        s.outside == Some(true),
                        "Sığmazsa dışarıda",
                        Edit::Outside
                    ))
                    .width(Fill),
                ]
                .spacing(16),
            ]
            .spacing(4),
            None,
        ),
    ]
}

const SHAPES: [LabelShape; 3] = [LabelShape::Rect, LabelShape::Round, LabelShape::Ellipse];
const CALLOUTS: [CalloutKind; 2] = [CalloutKind::Straight, CalloutKind::Manhattan];

/// A choice of “Yok” and a few kinds: the chosen kind, or none.
fn optional_index<T: PartialEq>(all: &[T], v: Option<&T>) -> usize {
    v.map_or(0, |v| index_of(all, v) + 1)
}

/// Biçim: the halo, the background, the shadow and the callout.
fn look_tab<'a>(h: &Host<'a>) -> Vec<Element<'a, Message>> {
    let s = h.style;
    let mut rows = vec![
        row_of(
            "Hale",
            row![
                h.number(Key::HaloWidth, "1.5", Some("px")),
                colour(
                    Paint::Halo,
                    s.halo.as_ref().and_then(|h| h.color.as_deref()),
                    "Zemin rengi"
                ),
            ]
            .spacing(10)
            .align_y(Center),
            None,
        ),
        row_of(
            "Zemin",
            choice(
                &["Yok", "Dikdörtgen", "Yuvarlak", "Elips"],
                optional_index(&SHAPES, s.background.as_ref().map(|b| &b.shape)),
                |i| {
                    msg(Event::Edit(Edit::Background(
                        i.checked_sub(1).map(|i| SHAPES[i]),
                    )))
                },
            ),
            None,
        ),
    ];
    if let Some(bg) = &s.background {
        rows.push(row_of(
            "Zeminin rengi",
            row![
                colour(Paint::Fill, bg.fill.as_deref(), "Dolgusuz"),
                colour(Paint::Stroke, bg.stroke.as_deref(), "Çizgisiz"),
                h.number(Key::Padding, "2", Some("px")),
            ]
            .spacing(10)
            .align_y(Center),
            None,
        ));
    }
    let mut shadow = Row::new().spacing(10).align_y(Center).push(check(
        s.shadow.is_some(),
        "Gölge",
        Edit::Shadow,
    ));
    if s.shadow.is_some() {
        shadow = shadow
            .push(h.number(Key::ShadowDx, "", Some("px")))
            .push(h.number(Key::ShadowDy, "", Some("px")))
            .push(h.number(Key::ShadowOpacity, "0.5", None));
    }
    rows.push(row_of("Gölge", shadow, None));
    rows.push(row_of(
        "Çağrı çizgisi",
        choice(
            &["Yok", "Düz", "Dik açılı"],
            optional_index(&CALLOUTS, s.callout.as_ref().map(|c| &c.kind)),
            |i| {
                msg(Event::Edit(Edit::Callout(
                    i.checked_sub(1).map(|i| CALLOUTS[i]),
                )))
            },
        ),
        Some("Etiketi nesnesinden uzakta kalınca (dışarıda, elle taşınmış) nesneye bağlar."),
    ));
    if let Some(callout) = &s.callout {
        rows.push(row_of(
            "Çizginin görünüşü",
            row![
                colour(Paint::Callout, callout.color.as_deref(), "Yazının rengi"),
                h.number(Key::CalloutWidth, "1", Some("px")),
                h.number(Key::CalloutMin, "6", Some("px")),
            ]
            .spacing(10)
            .align_y(Center),
            None,
        ));
    }
    rows
}

const STACKS: [StackMode; 2] = [StackMode::IfNeeded, StackMode::Always];

/// Sığdırma: stacking, abbreviating and shrinking a label that does not fit.
fn fit_tab<'a>(h: &Host<'a>) -> Vec<Element<'a, Message>> {
    let s = h.style;
    let mut rows = vec![row_of(
        "Yığma",
        choice(
            &["Yok", "Gerekirse", "Her zaman"],
            optional_index(&STACKS, s.stack.as_ref().map(|k| &k.mode)),
            |i| {
                msg(Event::Edit(Edit::Stack(
                    i.checked_sub(1).map(|i| STACKS[i]),
                )))
            },
        ),
        Some("Uzun etiket satırlara bölünür."),
    )];
    if s.stack.is_some() {
        rows.push(row_of(
            "Satır",
            row![
                h.number(Key::StackChars, "", Some("harf")),
                h.input(Key::StackAt, "boşluk", Some(120.0)),
            ]
            .spacing(10)
            .align_y(Center),
            None,
        ));
    }
    let abbreviate = match &s.abbreviate {
        None => 0,
        Some(a) if a.always == Some(true) => 2,
        Some(_) => 1,
    };
    rows.push(row_of(
        "Kısaltma",
        choice(&["Yok", "Gerekirse", "Her zaman"], abbreviate, |i| {
            msg(Event::Edit(Edit::Abbreviate(match i {
                0 => None,
                1 => Some(false),
                _ => Some(true),
            })))
        }),
        None,
    ));
    if s.abbreviate.is_some() {
        rows.push(row_of(
            "Sözlük",
            text_editor(h.dictionary)
                .placeholder("Caddesi = Cd.\nSokak = Sk.")
                .on_action(|a| msg(Event::Dictionary(a)))
                .height(typography::scaled(84.0))
                .padding([5, 8])
                .size(typography::body()),
            Some("Her satır bir sözcük: “Sözcük = Kısa”. Yalnız tam sözcükler kısalır."),
        ));
    }
    rows.push(row_of(
        "Küçültme",
        h.number(Key::Shrink, "100", Some("%")),
        Some("Sığmayan etiket bu orana kadar adım adım küçülür."),
    ));
    rows
}

const OVERLAPS: [LabelOverlap; 3] = [
    LabelOverlap::Never,
    LabelOverlap::IfNeeded,
    LabelOverlap::Always,
];

/// Öncelik: priority, overlap, duplicates and when a class is drawn at all.
fn order_tab<'a>(h: &Host<'a>) -> Vec<Element<'a, Message>> {
    let s = h.style;
    vec![
        row_of(
            "Öncelik",
            h.number(Key::Priority, "5", None),
            Some(
                "0–10: yüksek olan önce yerleşir; engelin ağırlığından küçük olan engeli örtemez.",
            ),
        ),
        row_of(
            "Çakışma",
            choice(
                &["Çakışmasın", "Gerekirse", "Her zaman"],
                index_of(&OVERLAPS, &s.overlap.unwrap_or(LabelOverlap::Never)),
                |i| msg(Event::Edit(Edit::Overlap(OVERLAPS[i]))),
            ),
            Some("Gerekirse: yer bulamazsa öbür etiketlerin üstüne yazılır."),
        ),
        row_of(
            "Yinelenenler",
            h.number(Key::Duplicates, "ayıklanmaz", Some("px")),
            Some("Aynı metin bu uzaklıktan yakınsa yazılmaz."),
        ),
        row_of(
            "En küçük nesne",
            h.number(Key::MinFeature, "0", Some("px")),
            Some("Ekranda bundan küçük nesnenin etiketi yazılmaz."),
        ),
        row_of(
            "Ölçek aralığı",
            row![
                h.number(Key::MinScale, "sınırsız", Some("px/m")),
                h.number(Key::MaxScale, "sınırsız", Some("px/m")),
            ]
            .spacing(10)
            .align_y(Center),
            Some("Etiket yalnız bu yakınlıklar arasında yazılır."),
        ),
    ]
}

/// A tab's rows.
pub fn rows<'a>(tab: Tab, h: &Host<'a>) -> Vec<Element<'a, Message>> {
    match tab {
        Tab::Text => text_tab(h),
        Tab::Place => place_tab(h),
        Tab::Look => look_tab(h),
        Tab::Fit => fit_tab(h),
        Tab::Order => order_tab(h),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn style() -> LabelStyle {
        LabelStyle {
            placement: LabelPlacement::Center,
            size: 10.0,
            ..LabelStyle::default()
        }
    }

    #[test]
    fn numbers_read_with_a_comma_and_in_their_range() {
        let mut s = style();
        assert!(typed(&mut s, Key::HaloWidth, "2,5"));
        assert_eq!(s.halo.as_ref().map(|h| h.width), Some(2.5));
        assert!(!typed(&mut s, Key::HaloWidth, "11"));
        assert_eq!(s.halo.as_ref().map(|h| h.width), Some(2.5));
        assert!(typed(&mut s, Key::HaloWidth, ""));
        assert!(s.halo.is_none());
        assert!(typed(&mut s, Key::Size, ""));
        assert_eq!(s.size, 10.0);
        assert!(typed(&mut s, Key::Shrink, "80"));
        assert_eq!(s.shrink, Some(0.8));
        assert!(typed(&mut s, Key::Shrink, "100"));
        assert_eq!(s.shrink, None);
        assert!(typed(&mut s, Key::Priority, "7.4"));
        assert_eq!(s.priority, Some(7));
    }

    #[test]
    fn the_dictionary_reads_as_the_web_s() {
        let words = read_dictionary("Caddesi = Cd.\nyalnız\n Sokak=Sk. \n=");
        assert_eq!(
            words,
            vec![
                LabelWord {
                    word: "Caddesi".into(),
                    short: "Cd.".into()
                },
                LabelWord {
                    word: "Sokak".into(),
                    short: "Sk.".into()
                },
            ]
        );
        let mut s = style();
        apply(&mut s, Edit::Abbreviate(Some(true)));
        assert_eq!(dictionary_text(&s), "Caddesi = Cd.");
        assert_eq!(s.abbreviate.as_ref().and_then(|a| a.always), Some(true));
    }

    #[test]
    fn choices_write_what_the_web_writes() {
        let mut s = style();
        apply(&mut s, Edit::Background(Some(LabelShape::Round)));
        assert_eq!(
            s.background.as_ref().and_then(|b| b.fill.as_deref()),
            Some("paper")
        );
        apply(&mut s, Edit::Paint(Paint::Halo, Some("#E5484D".into())));
        assert_eq!(s.halo.as_ref().map(|h| h.width), Some(1.5));
        apply(&mut s, Edit::Expression(true));
        assert_eq!(s.text.as_deref(), Some("$etiket"));
        apply(&mut s, Edit::Align(LabelAlign::Center));
        assert_eq!(s.align, None);
        apply(&mut s, Edit::Overlap(LabelOverlap::Never));
        assert_eq!(s.overlap, None);
        apply(&mut s, Edit::Stack(Some(StackMode::IfNeeded)));
        assert_eq!(s.stack.as_ref().map(|k| k.chars), Some(12));
    }
}
