//! Nokta editörü's batch windows (docs/adr/0153 §5; the web's
//! `ui/bottom/PointBatchDialog.ts`): the operation's values over its target
//! rows (named at the top), what it would change (how many points, the first
//! change), Uygula writing it as one undo step named after it (`batch.rs`). A
//! refusal is said and shown, and the window stays for another value. Çift
//! noktaları ayıkla's window counts the groups as its values change;
//! Çiftleri göster shows them in the table.

use std::fmt;

use iced::widget::{Id, column, container, row, space, text, text_input};
use iced::{Center, Color, Element, Fill, Task};
use kentos_contracts::{Entity, LayerNode, LayerNodeType};
use kentos_domain::Slot;
use kentos_geometry_core::ops::point_editor::Keep;
use kentos_geometry_core::tools::point_text::js_trim;
use kentos_interaction::Level;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::typography;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::{Dialog as Frame, Menu, MenuButton, focus_ring, overlay};
use kentos_ui::{label, style};

use super::batch::{self, Kind, Op};
use super::edit::PREFIX;
use super::{Event, layer_name};
use crate::app::{App, Dialog, Message};
use crate::document::Document;
use crate::exchange::words::{self, Kind as Line};
use crate::icons::from_web;

/// The window's text field (Önek, Başlangıç adı): the keyboard goes to it
/// when the window opens.
pub(crate) const FIELD: &str = "point-batch-field";
/// How many of the targets' names the window lists under their count.
const LISTED: usize = 6;

/// An operation's window, while it is open.
#[derive(Debug, Clone)]
pub(crate) struct Window {
    kind: Kind,
    slots: Vec<Slot>,
    header: String,
    /// Önek ekle (else Önek kaldır).
    add: bool,
    /// Önek, Başlangıç adı or Tolerans.
    text: String,
    /// Çift noktaları ayıkla: Aynı ad (else Aynı yer), and the one kept.
    by_name: bool,
    keep: Keep,
    layer: String,
    /// The command's refusal at the last Uygula.
    refused: Option<String>,
}

impl Window {
    fn op(&self) -> Op {
        match self.kind {
            Kind::Rename => Op::Rename {
                add: self.add,
                prefix: self.text.clone(),
            },
            Kind::Number => Op::Number {
                start: self.text.clone(),
            },
            Kind::Layer => Op::Layer {
                layer: self.layer.clone(),
            },
            Kind::Dedupe => Op::Dedupe {
                by_name: self.by_name,
                tolerance: self.text.clone(),
                keep: self.keep,
            },
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum WindowEvent {
    /// Önek ekle (true) or Önek kaldır.
    Mode(bool),
    Text(String),
    Layer(String),
    /// Çift noktaları ayıkla: Aynı ad (true) or Aynı yer; the one kept.
    By(bool),
    Keep(Keep),
    /// Çiftleri göster.
    ShowGroups,
    Apply,
    Close,
}

fn msg(e: WindowEvent) -> Message {
    Message::Points(Event::Window(e))
}

/// Önek ekle or Önek kaldır, as the segmented control shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Mode(bool);

impl fmt::Display for Mode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.0 {
            "Önek ekle"
        } else {
            "Önek kaldır"
        })
    }
}

/// Aynı yer or Aynı ad (`true`), as the segmented control shows them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct By(bool);

impl fmt::Display for By {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(if self.0 { "Aynı ad" } else { "Aynı yer" })
    }
}

/// İlki, Sonuncusu or Ortalaması.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Kept(Keep);

impl fmt::Display for Kept {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            Keep::First => "İlki",
            Keep::Last => "Sonuncusu",
            Keep::Average => "Ortalaması",
        })
    }
}

/// A point's name as the window quotes it, “(adsız)” without one.
fn name_of(model: &kentos_domain::Document, slot: Slot) -> Option<String> {
    match model.get(slot) {
        Some(Entity::Point(p)) => p
            .base
            .label
            .as_deref()
            .map(js_trim)
            .filter(|n| !n.is_empty())
            .map(str::to_owned),
        _ => None,
    }
}

/// A line of the layer field's menu: a group's path, or a layer.
#[derive(Debug, Clone)]
enum LayerLine {
    Header(String),
    Layer {
        id: String,
        name: String,
        color: Color,
        count: usize,
        locked: bool,
    },
}

impl App {
    /// An operation's window over the target rows (İşlemler ▾, a row's menu):
    /// Sıralı numara ver starts from the first row's name when it ends with a
    /// number, Katmana taşı from the active layer.
    pub(crate) fn open_point_batch(&mut self, kind: Kind) -> Task<Message> {
        let Some(doc) = self.document.as_ref() else {
            return Task::none();
        };
        let shown = self.point_rows(doc).shown;
        let (slots, header) = batch::targets(&shown, |s| self.selection.contains(s));
        if slots.is_empty() {
            return Task::none();
        }
        let first = slots
            .first()
            .and_then(|&s| name_of(&doc.model, s))
            .unwrap_or_default();
        let text = match kind {
            Kind::Number if first.as_bytes().last().is_some_and(u8::is_ascii_digit) => first,
            Kind::Number => "1".to_owned(),
            Kind::Dedupe => "0.001".to_owned(),
            _ => String::new(),
        };
        let layer = doc.model.layers().active().to_owned();
        self.points.batch = Some(Window {
            kind,
            slots,
            header,
            add: true,
            text,
            by_name: false,
            keep: Keep::First,
            layer,
            refused: None,
        });
        self.dialog = Some(Dialog::PointBatch);
        if matches!(kind, Kind::Layer | Kind::Dedupe) {
            return Task::none();
        }
        let id = Id::new(FIELD);
        Task::batch([
            iced::widget::operation::focus(id.clone()),
            iced::widget::operation::select_all(id),
        ])
    }

    pub(crate) fn point_batch_event(&mut self, event: WindowEvent) -> Task<Message> {
        let Some(w) = self.points.batch.as_mut() else {
            return Task::none();
        };
        match event {
            WindowEvent::Mode(add) => w.add = add,
            WindowEvent::Text(t) => w.text = t,
            WindowEvent::Layer(layer) => w.layer = layer,
            WindowEvent::By(by_name) => w.by_name = by_name,
            WindowEvent::Keep(keep) => w.keep = keep,
            WindowEvent::Close => {
                self.points.batch = None;
                self.dialog = None;
                return Task::none();
            }
            WindowEvent::ShowGroups => {
                let (slots, op) = (w.slots.clone(), w.op());
                let Some(doc) = self.document.as_ref() else {
                    return Task::none();
                };
                let groups = batch::plan_dedupe(&doc.model, &slots, &op).groups;
                if groups.is_empty() {
                    return Task::none();
                }
                self.points.show_groups(Some(groups));
                self.points.batch = None;
                self.dialog = None;
                return Task::none();
            }
            WindowEvent::Apply => {
                let (slots, op) = (w.slots.clone(), w.op());
                let follow = self.points.follow;
                let Some(doc) = self.document.as_mut() else {
                    return Task::none();
                };
                let plan = batch::plan(&doc.model, &slots, &op);
                let nothing = match &op {
                    Op::Dedupe { .. } => batch::plan_dedupe(&doc.model, &slots, &op)
                        .groups
                        .is_empty(),
                    _ => plan.changes.is_empty(),
                };
                if plan.error.is_some() || nothing {
                    return Task::none();
                }
                let out = batch::run(&mut doc.model, &slots, &op, follow);
                if out.step.is_some() {
                    let mut said = out.said.into_iter();
                    if let Some(done) = said.next() {
                        self.say(Level::Success, done);
                    }
                    for line in said {
                        self.warn(line);
                    }
                    self.points.batch = None;
                    self.dialog = None;
                    // The groups shown are done with.
                    self.points.show_groups(None);
                } else {
                    for line in &out.said {
                        self.warn(line.clone());
                    }
                    if let Some(w) = self.points.batch.as_mut() {
                        w.refused = out.said.first().cloned();
                    }
                }
                return Task::none();
            }
        }
        // A new value: the last refusal was of another one.
        if let Some(w) = self.points.batch.as_mut() {
            w.refused = None;
        }
        Task::none()
    }

    /// The layers as the toolbar's list: groups as headers with their path,
    /// each layer its colour and count; a locked one takes nothing.
    fn batch_layer_lines(&self, doc: &Document) -> Vec<LayerLine> {
        let layers = doc.model.layers();
        let mut out = Vec::new();
        let mut stack: Vec<&LayerNode> = layers.nodes().iter().rev().collect();
        while let Some(node) = stack.pop() {
            match node.kind {
                LayerNodeType::Group => {
                    out.push(LayerLine::Header(crate::properties::layer_path(
                        layers, &node.id,
                    )));
                    stack.extend(node.children.iter().rev());
                }
                LayerNodeType::Layer => out.push(LayerLine::Layer {
                    id: node.id.clone(),
                    name: node.name.clone(),
                    color: self.drawing_color(&node.style.color),
                    count: doc.count_below(node),
                    locked: layers.is_locked(&node.id),
                }),
            }
        }
        out
    }

    /// Katman: the chosen layer's colour and name; a click opens the list.
    fn batch_layer_field(&self, doc: &Document, chosen: &str) -> Element<'_, Message> {
        let lines = self.batch_layer_lines(doc);
        let (name, color) = lines
            .iter()
            .find_map(|l| match l {
                LayerLine::Layer {
                    id, name, color, ..
                } if id == chosen => Some((name.clone(), *color)),
                _ => None,
            })
            .unwrap_or_default();
        let chosen = chosen.to_owned();
        let face = container(
            row![
                kentos_ui::widget::swatch(color),
                text(name).font(typography::ui()).size(typography::body()),
                space::horizontal(),
                icon(Icon::ChevronDown).size(12.0).tone(Tone::Muted),
            ]
            .spacing(8)
            .align_y(Center),
        )
        .padding([5, 8])
        .width(Fill)
        .style(style::container::field_box);
        MenuButton::new(face, move || {
            lines.iter().fold(Menu::new(), |menu, line| match line {
                LayerLine::Header(path) => menu.header(path.clone()),
                LayerLine::Layer {
                    id,
                    name,
                    color,
                    count,
                    locked,
                } => menu
                    .radio(
                        name.clone(),
                        *id == chosen,
                        (!*locked).then(|| msg(WindowEvent::Layer(id.clone()))),
                    )
                    .swatch(*color)
                    .shortcut(count.to_string()),
            })
        })
        .into()
    }

    pub(crate) fn point_batch_view(&self) -> Element<'_, Message> {
        let (Some(w), Some(doc)) = (&self.points.batch, &self.document) else {
            return text("").into();
        };
        let model = &doc.model;
        // The targets: their count and the first names.
        let names: Vec<String> = w
            .slots
            .iter()
            .take(LISTED)
            .map(|&s| name_of(model, s).unwrap_or_else(|| "(adsız)".to_owned()))
            .collect();
        let more = if w.slots.len() > LISTED { " …" } else { "" };
        let chip = container(
            row![
                icon(from_web(Some("pointEditor")))
                    .size(18.0)
                    .tone(Tone::Muted),
                column![
                    label::strong(w.header.clone()),
                    label::caption(format!("{}{more}", names.join(", "))),
                ]
                .spacing(2),
            ]
            .spacing(10)
            .align_y(Center),
        )
        .padding([8, 10])
        .width(Fill)
        .style(style::container::tile);
        let field = |placeholder: &str| {
            focus_ring(
                text_input(placeholder, &w.text)
                    .id(Id::new(FIELD))
                    .on_input(|t| msg(WindowEvent::Text(t)))
                    .on_submit(msg(WindowEvent::Apply))
                    .padding([5, 8])
                    .style(style::field::input),
            )
        };
        let values: Element<'_, Message> = match w.kind {
            Kind::Rename => row![
                words::field(
                    "İşlem",
                    Segmented::new([Mode(true), Mode(false)], Mode(w.add), |m| {
                        msg(WindowEvent::Mode(m.0))
                    }),
                    None
                ),
                container(words::field("Önek", field("ör. P."), None)).width(Fill),
            ]
            .spacing(18)
            .into(),
            Kind::Number => words::field(
                "Başlangıç adı",
                field("ör. 1, P100, 101/1"),
                Some("İlk satır bu adı, her sonraki bir fazlasını alır (Artır).".to_owned()),
            ),
            Kind::Layer => words::field("Katman", self.batch_layer_field(doc, &w.layer), None),
            Kind::Dedupe => {
                let tolerance = focus_ring(
                    text_input("metre", &w.text)
                        .id(Id::new(FIELD))
                        .on_input_maybe((!w.by_name).then_some(|t| msg(WindowEvent::Text(t))))
                        .on_submit(msg(WindowEvent::Apply))
                        .font(typography::mono())
                        .padding([5, 8])
                        .style(style::field::input),
                );
                let kept_hint = if self.points.follow {
                    "Ortalamada tutulan grubun ilk noktasıdır; Bağlı çizgiler izler açık: çizgileri de taşınır."
                } else {
                    "Ortalamada tutulan grubun ilk noktasıdır; Bağlı çizgiler izler kapalı: çizgiler yerinde kalır."
                };
                column![
                    row![
                        words::field(
                            "Ölçüt",
                            Segmented::new([By(false), By(true)], By(w.by_name), |b| {
                                msg(WindowEvent::By(b.0))
                            }),
                            None
                        ),
                        container(words::field(
                            "Tolerans (m)",
                            tolerance,
                            Some(
                                "Aynı yerde: ilk noktasına bu kadar yakın olan gruba katılır."
                                    .to_owned()
                            ),
                        ))
                        .width(Fill),
                    ]
                    .spacing(18),
                    words::field(
                        "Tutulan",
                        Segmented::new(
                            [Kept(Keep::First), Kept(Keep::Last), Kept(Keep::Average)],
                            Kept(w.keep),
                            |k| msg(WindowEvent::Keep(k.0)),
                        ),
                        Some(kept_hint.to_owned()),
                    ),
                ]
                .spacing(12)
                .into()
            }
        };
        let plain = |t: &str| t.strip_prefix(PREFIX).unwrap_or(t).to_owned();
        if w.kind == Kind::Dedupe {
            let plan = batch::plan_dedupe(model, &w.slots, &w.op());
            let mut lines = vec![match &plan.error {
                Some(error) => words::text_line(Line::Warn, plain(error)),
                None => words::text_line(Line::Info, plan.summary.clone().unwrap_or_default()),
            }];
            if let Some(refused) = &w.refused {
                lines.push(words::text_line(Line::Warn, plain(refused)));
            }
            let can = plan.error.is_none() && !plan.groups.is_empty();
            return overlay::modal(
                Frame::new(w.kind.step())
                    .push(column![chip, values, words::summary(lines)].spacing(12))
                    .action(words::secondary(
                        "Çiftleri göster",
                        can.then(|| msg(WindowEvent::ShowGroups)),
                    ))
                    .action(words::secondary("Vazgeç", Some(msg(WindowEvent::Close))))
                    .action(words::primary(
                        "Ayıkla",
                        can.then(|| msg(WindowEvent::Apply)),
                    ))
                    .width(540.0),
                msg(WindowEvent::Close),
            );
        }
        let plan = batch::plan(model, &w.slots, &w.op());
        let mut lines = Vec::new();
        if let Some(error) = &plan.error {
            let kind = if js_trim(&w.text).is_empty() {
                Line::Info
            } else {
                Line::Warn
            };
            lines.push(words::text_line(kind, plain(error)));
        } else if plan.changes.is_empty() {
            lines.push(words::text_line(
                Line::Info,
                if w.kind == Kind::Layer {
                    "Taşınacak nokta yok."
                } else {
                    "Adı değişen nokta yok."
                },
            ));
        } else if w.kind == Kind::Layer {
            lines.push(words::text_line(
                Line::Info,
                format!(
                    "{} nokta “{}” katmanına taşınacak.",
                    plan.changes.len(),
                    layer_name(model, &w.layer)
                ),
            ));
        } else {
            let (slot, to) = &plan.changes[0];
            let from =
                name_of(model, *slot).map_or_else(|| "(adsız)".to_owned(), |n| format!("“{n}”"));
            lines.push(words::text_line(
                Line::Info,
                format!(
                    "{} noktanın adı değişecek; ilki {from} → “{}”.",
                    plan.changes.len(),
                    to.clone().unwrap_or_default()
                ),
            ));
        }
        if let Some(refused) = &w.refused {
            lines.push(words::text_line(Line::Warn, plain(refused)));
        }
        let can = plan.error.is_none() && !plan.changes.is_empty();
        overlay::modal(
            Frame::new(w.kind.step())
                .push(column![chip, values, words::summary(lines)].spacing(12))
                .action(words::secondary("Vazgeç", Some(msg(WindowEvent::Close))))
                .action(words::primary(
                    "Uygula",
                    can.then(|| msg(WindowEvent::Apply)),
                ))
                .width(480.0),
            msg(WindowEvent::Close),
        )
    }
}
