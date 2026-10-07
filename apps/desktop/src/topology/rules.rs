//! Topoloji kuralları (docs/adr/0202 §6; the web's
//! `ui/topology/TopologyRulesDialog.ts`): the tolerance and the rules in a
//! table, each its layer, kind, other layer and value, and how many
//! exceptions it has. Kural ekle puts a rule on the active layer; Sil takes
//! the chosen one, its exceptions with it. Lengths are typed in the
//! project's unit, angles in its angle unit. The first problem is said under
//! the table and Kaydet waits for it. Kaydet writes the project's setting
//! (not an undo step, as the layer states).

use iced::widget::{Column, button, container, row, text, text_input};
use iced::{Center, Element, Fill, Length, Task};
use kentos_contracts::{
    TOPOLOGY_TOLERANCE, TopologyRule, TopologyRuleKind, TopologySettings, TopologyValue,
};
use kentos_geometry_core::ops::topology_rules::Kind;
use kentos_interaction::{Format, Level};
use kentos_ui::icon::icon;
use kentos_ui::theme::typography;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind as Say};
use crate::icons::from_web;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const RULES_TITLE: &str = "Topoloji kuralları";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const ADD: &str = "Kural ekle";
const REMOVE: &str = "Sil";

/// The table's columns and their widths (logical pixels at the default size).
const HEADS: [(&str, f32); 5] = [
    ("Katman", 190.0),
    ("Kural", 260.0),
    ("Öbür katman", 190.0),
    ("Değer", 130.0),
    ("İstisna", 60.0),
];

/// A rule as the window edits it: its value as typed (empty: the kind's default).
#[derive(Debug, Clone, PartialEq)]
pub struct RuleRow {
    pub id: String,
    pub kind: TopologyRuleKind,
    pub layer: String,
    pub other: String,
    pub value: String,
}

/// A decimal as typed: a point or a comma.
fn decimal(t: &str) -> Option<f64> {
    let t = t.trim().replace(',', ".");
    let ok = !t.is_empty()
        && t.trim_start_matches(['+', '-'])
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.')
        && t.matches('.').count() <= 1
        && t.chars().any(|c| c.is_ascii_digit());
    ok.then(|| t.parse().ok()).flatten()
}

/// A value as the window shows it: twelve significant digits (no noise of the unit's conversion).
fn shown(v: f64) -> String {
    let s = format!("{:.12e}", v);
    let n: f64 = s.parse().unwrap_or(v);
    kentos_project::new_project::js_number(n)
}

/// One of the angle unit, in radians.
fn angle_radians(format: &Format) -> f64 {
    format.angle_from_typed(1.0)
}

/// A rule as the window shows it.
pub fn row_of(r: &TopologyRule, format: &Format) -> RuleRow {
    let value = match (r.value, r.kind.value()) {
        (Some(v), Some(TopologyValue::Length)) => shown(format.from_metres(v)),
        (Some(v), Some(TopologyValue::Angle)) => shown(v / angle_radians(format)),
        _ => String::new(),
    };
    RuleRow {
        id: r.id.clone(),
        kind: r.kind,
        layer: r.layer.clone(),
        other: r.other.clone().unwrap_or_default(),
        value,
    }
}

/// The rule a row writes: another layer only between layers, a value only where the kind takes one.
pub fn rule_of(r: &RuleRow, format: &Format) -> TopologyRule {
    let value = match (r.kind.value(), decimal(&r.value)) {
        (Some(TopologyValue::Length), Some(v)) => Some(format.to_metres(v)),
        (Some(TopologyValue::Angle), Some(v)) => Some(v * angle_radians(format)),
        _ => None,
    };
    TopologyRule {
        id: r.id.clone(),
        kind: r.kind,
        layer: r.layer.clone(),
        other: (r.kind.between() && !r.other.is_empty()).then(|| r.other.clone()),
        value,
    }
}

/// What is wrong with the window's values, in the order a reader meets them.
pub fn rules_problem(
    tolerance: &str,
    rows: &[RuleRow],
    format: &Format,
    layer_name: impl Fn(&str) -> Option<String>,
) -> Option<String> {
    let t = decimal(tolerance).map(|t| format.to_metres(t));
    if !t.is_some_and(TopologySettings::tolerance_holds) {
        return Some("Tolerans 0,000001 m ile 1 m arasında bir sayı olmalı.".to_owned());
    }
    for (i, r) in rows.iter().enumerate() {
        let n = format!("{}. kural", i + 1);
        if layer_name(&r.layer).is_none() {
            return Some(format!("{n}: katman seçin."));
        }
        if r.kind.between() && layer_name(&r.other).is_none() {
            return Some(format!("{n}: öbür katmanı seçin."));
        }
        if r.kind.between() && r.other == r.layer {
            return Some(format!("{n}: öbür katman kuralın kendi katmanı olamaz."));
        }
        if let Some(kind) = r.kind.value()
            && !r.value.trim().is_empty()
        {
            let si = decimal(&r.value).map(|v| match kind {
                TopologyValue::Length => format.to_metres(v),
                TopologyValue::Angle => v * angle_radians(format),
            });
            if !si.is_some_and(|v| TopologySettings::value_holds(kind, v)) {
                return Some(match kind {
                    TopologyValue::Length => {
                        format!("{n}: değer sıfırdan büyük bir uzunluk olmalı.")
                    }
                    TopologyValue::Angle => {
                        format!("{n}: açı sıfırdan büyük, dik açıdan küçük olmalı.")
                    }
                });
            }
        }
    }
    None
}

/// The window: the tolerance and the rows as typed, the settings it opened
/// with, the chosen row and what it last said.
#[derive(Debug, Clone)]
pub struct Window {
    tolerance: String,
    rows: Vec<RuleRow>,
    before: Option<TopologySettings>,
    chosen: Option<usize>,
    said: Option<(Say, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Tolerance(String),
    /// A row's layer, kind or other layer chosen (by its place in its list).
    Layer(usize, usize),
    Kind(usize, usize),
    Other(usize, usize),
    Value(usize, String),
    Choose(usize),
    Add,
    Remove,
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::TopologyRules(event)
}

impl App {
    /// The layers the window lists, by id with their paths.
    fn rule_layers(&self) -> Vec<(String, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let layers = doc.model.layers();
        layers
            .leaves()
            .iter()
            .map(|n| (n.id.clone(), layers.path(&n.id)))
            .collect()
    }

    /// The settings the window would write.
    fn rules_settings(&self, w: &Window) -> Option<TopologySettings> {
        let format = self.format();
        let rules: Vec<TopologyRule> = w.rows.iter().map(|r| rule_of(r, &format)).collect();
        let t = decimal(&w.tolerance)
            .map(|t| format.to_metres(t))
            .unwrap_or(TOPOLOGY_TOLERANCE);
        let exceptions = w
            .before
            .as_ref()
            .map(|b| b.exceptions.clone())
            .unwrap_or_default()
            .into_iter()
            .filter(|x| rules.iter().any(|r| r.id == x.rule))
            .collect();
        TopologySettings {
            tolerance: ((t - TOPOLOGY_TOLERANCE).abs() > 1e-12).then_some(t),
            rules,
            exceptions,
        }
        .sanitized()
    }

    fn rules_changed(&self, w: &Window) -> bool {
        self.rules_settings(w) != w.before
    }

    /// `topology.rules`: the window.
    pub(crate) fn open_topology_rules(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let format = Format::of(doc.settings());
        let before = doc.model.settings().topology.clone();
        let tolerance = shown(
            format.from_metres(
                before
                    .as_ref()
                    .map_or(TOPOLOGY_TOLERANCE, |t| t.tolerance()),
            ),
        );
        let rows: Vec<RuleRow> = before
            .as_ref()
            .map(|t| t.rules.iter().map(|r| row_of(r, &format)).collect())
            .unwrap_or_default();
        self.topology_rules = Some(Window {
            tolerance,
            chosen: (!rows.is_empty()).then_some(0),
            rows,
            before,
            said: None,
        });
        self.dialog = Some(Dialog::TopologyRules);
        Task::none()
    }

    pub(crate) fn topology_rules_event(&mut self, event: Event) -> Task<Message> {
        let layers = self.rule_layers();
        let active = self
            .document
            .as_ref()
            .map(|d| d.model.layers().active().to_owned())
            .unwrap_or_default();
        let Some(w) = self.topology_rules.as_mut() else {
            return Task::none();
        };
        w.said = None;
        match event {
            Event::Tolerance(t) => w.tolerance = t,
            Event::Layer(i, k) => {
                if let (Some(r), Some((id, _))) = (w.rows.get_mut(i), layers.get(k)) {
                    r.layer = id.clone();
                }
                w.chosen = Some(i);
            }
            Event::Other(i, k) => {
                if let (Some(r), Some((id, _))) = (w.rows.get_mut(i), layers.get(k)) {
                    r.other = id.clone();
                }
                w.chosen = Some(i);
            }
            Event::Kind(i, k) => {
                if let (Some(r), Some(kind)) = (w.rows.get_mut(i), TopologyRuleKind::ALL.get(k)) {
                    r.kind = *kind;
                    if !kind.between() {
                        r.other.clear();
                    }
                    if kind.value().is_none() {
                        r.value.clear();
                    }
                }
                w.chosen = Some(i);
            }
            Event::Value(i, t) => {
                if let Some(r) = w.rows.get_mut(i) {
                    r.value = t;
                }
                w.chosen = Some(i);
            }
            Event::Choose(i) => w.chosen = Some(i),
            Event::Add => {
                let layer = if layers.iter().any(|(id, _)| *id == active) {
                    active
                } else {
                    layers.first().map(|(id, _)| id.clone()).unwrap_or_default()
                };
                let id = (1..)
                    .map(|n| format!("kural-{n}"))
                    .find(|id| !w.rows.iter().any(|r| r.id == *id))
                    .unwrap_or_default();
                w.rows.push(RuleRow {
                    id,
                    kind: TopologyRuleKind::MustNotOverlap,
                    layer,
                    other: String::new(),
                    value: String::new(),
                });
                w.chosen = Some(w.rows.len() - 1);
            }
            Event::Remove => {
                if let Some(i) = w.chosen.filter(|&i| i < w.rows.len()) {
                    w.rows.remove(i);
                    w.chosen = (!w.rows.is_empty()).then(|| i.min(w.rows.len() - 1));
                }
            }
            Event::Save => return self.save_topology_rules(),
            Event::Cancel => {
                self.topology_rules = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    /// Kaydet: the project's setting; says how many rules, or why not.
    fn save_topology_rules(&mut self) -> Task<Message> {
        let Some(w) = self.topology_rules.clone() else {
            return Task::none();
        };
        let format = self.format();
        let layers = self.rule_layers();
        let name = |id: &str| layers.iter().find(|(l, _)| l == id).map(|(_, p)| p.clone());
        if let Some(p) = rules_problem(&w.tolerance, &w.rows, &format, name) {
            if let Some(w) = self.topology_rules.as_mut() {
                w.said = Some((Say::Error, p));
            }
            return Task::none();
        }
        let next = self.rules_settings(&w);
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let mut settings = doc.model.settings().clone();
        settings.topology = next;
        doc.model.set_settings(settings);
        self.topology_rules = None;
        self.dialog = None;
        self.say(
            Level::Info,
            format!("Topoloji kuralları kaydedildi: {} kural.", w.rows.len()),
        );
        Task::none()
    }

    pub(crate) fn topology_rules_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(_)) = (&self.topology_rules, &self.document) else {
            return text("").into();
        };
        let format = self.format();
        let layers = self.rule_layers();
        let width = |j: usize| Length::Fixed(typography::scaled(HEADS[j].1));
        let head = HEADS
            .iter()
            .enumerate()
            .fold(row![].spacing(6), |r, (j, (h, _))| {
                r.push(container(label::caption(*h)).width(width(j)))
            });
        let kinds: Vec<Kind> = TopologyRuleKind::ALL
            .iter()
            .filter_map(|k| Kind::of_key(k.key()))
            .collect();
        let exceptions_of = |id: &str| {
            w.before
                .as_ref()
                .map_or(0, |b| b.exceptions.iter().filter(|x| x.rule == id).count())
        };
        let mut rows = Column::new().spacing(4);
        for (i, r) in w.rows.iter().enumerate() {
            let choices = || layers.iter().map(|(_, p)| Choice::new(p.clone()));
            let at = |id: &str| layers.iter().position(|(l, _)| l == id);
            let layer = Select::new(choices(), at(&r.layer), move |k| msg(Event::Layer(i, k)));
            let kind = Select::new(
                kinds.iter().map(|k| Choice::new(k.label())),
                TopologyRuleKind::ALL.iter().position(|k| *k == r.kind),
                move |k| msg(Event::Kind(i, k)),
            );
            let other: Element<'_, Message> = if r.kind.between() {
                Select::new(choices(), at(&r.other), move |k| msg(Event::Other(i, k))).into()
            } else {
                container(label::muted("—")).padding([4, 8]).into()
            };
            let unit = match r.kind.value() {
                Some(TopologyValue::Length) => format.length_unit_label(),
                Some(TopologyValue::Angle) => format.angle_unit_label(),
                None => "",
            };
            let default = r
                .kind
                .default_value()
                .zip(r.kind.value())
                .map(|(v, k)| match k {
                    TopologyValue::Length => shown(format.from_metres(v)),
                    TopologyValue::Angle => shown(v / angle_radians(&format)),
                })
                .unwrap_or_default();
            let field = text_input(&default, &r.value)
                .size(typography::body())
                .padding([3, 6])
                .align_x(iced::alignment::Horizontal::Right)
                .style(style::field::input);
            let field = if r.kind.value().is_some() {
                field.on_input(move |t| msg(Event::Value(i, t)))
            } else {
                field
            };
            let line = row![
                container(layer).width(width(0)),
                container(kind).width(width(1)),
                container(other).width(width(2)),
                container(row![field, label::caption(unit)].spacing(6).align_y(Center))
                    .width(width(3)),
                container(label::body(exceptions_of(&r.id).to_string())).width(width(4)),
            ]
            .spacing(6)
            .align_y(Center);
            rows = rows.push(
                button(line)
                    .on_press(msg(Event::Choose(i)))
                    .padding([2, 4])
                    .style(style::button::table_row(w.chosen == Some(i), false)),
            );
        }
        if w.rows.is_empty() {
            rows =
                rows.push(container(label::muted("Kural yok: Kural ekle ile ekleyin.")).padding(8));
        }
        let table = container(
            Column::new()
                .push(container(head).padding([4, 8]))
                .push(kentos_ui::widget::horizontal_divider())
                .push(
                    iced::widget::scrollable(rows).height(Length::Fixed(typography::scaled(260.0))),
                ),
        )
        .style(style::container::field_box)
        .width(Fill);
        let tool = |glyph: &str, words: &'static str, on: Option<Message>| {
            button(
                row![icon(from_web(Some(glyph))).size(14.0), label::body(words)]
                    .spacing(6)
                    .align_y(Center),
            )
            .style(style::button::secondary)
            .padding([4, 10])
            .on_press_maybe(on)
        };
        let actions = row![
            tool("plus", ADD, Some(msg(Event::Add))),
            tool("erase", REMOVE, w.chosen.map(|_| msg(Event::Remove))),
        ]
        .spacing(8)
        .align_y(Center);
        let name = |id: &str| layers.iter().find(|(l, _)| l == id).map(|(_, p)| p.clone());
        let problem = rules_problem(&w.tolerance, &w.rows, &format, name);
        let n = w.rows.len();
        let line = match (&w.said, &problem) {
            (Some((kind, words)), _) => words::text_line(*kind, words.clone()),
            (None, Some(p)) => words::text_line(Say::Error, p.clone()),
            (None, None) if n == 0 => {
                words::text_line(Say::Ok, "Kural yok: topoloji denetlenmez.".to_owned())
            }
            (None, None) => words::text_line(
                Say::Ok,
                format!("{n} kural; denetim alt panelin Topoloji sekmesinde."),
            ),
        };
        let tolerance = Column::new()
            .spacing(4)
            .push(label::caption("Tolerans"))
            .push(
                row![
                    container(
                        text_input("", &w.tolerance)
                            .on_input(|t| msg(Event::Tolerance(t)))
                            .size(typography::body())
                            .padding([3, 6])
                            .align_x(iced::alignment::Horizontal::Right)
                            .style(style::field::input)
                    )
                    .width(Length::Fixed(typography::scaled(110.0))),
                    label::caption(format.length_unit_label()),
                ]
                .spacing(6)
                .align_y(Center),
            )
            .push(label::muted(
                "Bu uzaklıktaki iki yer aynı yerdir; bundan dar çakışma ve boşluk sayılmaz.",
            ));
        let ready = problem.is_none() && self.rules_changed(w);
        overlay::modal(
            Frame::new(RULES_TITLE)
                .push(
                    Column::new()
                        .spacing(10)
                        .push(tolerance)
                        .push(table)
                        .push(actions)
                        .push(words::summary(vec![line])),
                )
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(SAVE, ready.then(|| msg(Event::Save))))
                .width(900.0),
            msg(Event::Cancel),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step): the
    /// tolerance, the chosen row's value, a row by its number, the buttons.
    pub(crate) fn topology_rules_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.topology_rules else {
            return Err(format!("{RULES_TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Fill("Tolerans", t) => Some(msg(Event::Tolerance(t.to_owned()))),
            Control::Fill("Değer", t) => {
                let i = w.chosen.ok_or("seçili kural yok")?;
                Some(msg(Event::Value(i, t.to_owned())))
            }
            Control::Row(n) if n >= 1 && n <= w.rows.len() => Some(msg(Event::Choose(n - 1))),
            Control::Press(SAVE) => self.rules_changed(w).then(|| msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(REMOVE) => w.chosen.map(|_| msg(Event::Remove)),
            other => return Err(format!("“{RULES_TITLE}” penceresinde {other} yok")),
        })
    }
}
