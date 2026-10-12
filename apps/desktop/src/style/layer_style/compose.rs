//! Katman çizimi in the layer style window (docs/adr/0226 §3, §4; the web's
//! `composeSection` in `LayerStyleDialog.ts`): the layer's opacity and
//! blending mode as it meets the drawing, and the mask its labels and markers
//! cut into other layers with. A draft like the renderer's; Uygula and Tamam
//! write it with `cad.layers.compose`, a step of its own “Katman çizimi”.

use iced::widget::{Column, column, container, row, scrollable};
use iced::{Center, Element, Fill};
use kentos_contracts::{BlendMode, LayerCompose, LayerMask, MAX_MASK_SIZE};
use kentos_ui::label;
use kentos_ui::style;
use kentos_ui::widget::tree_view::Check;

use super::{Event, LayerStyleWindow};
use crate::app::Message;
use crate::style::fields;

/// A change of the draft.
#[derive(Clone, Debug)]
pub enum Edit {
    /// Saydamlık as typed (%).
    Transparency(String),
    Blend(BlendMode),
    Labels(bool),
    Markers(bool),
    /// Boy as typed (mm).
    Size(String),
    /// A layer cut into or not.
    Target(String, bool),
}

/// The widths of a number field and of the masked layers' list.
const NUMBER: f32 = 160.0;
const LIST: f32 = 380.0;

fn ev(e: Edit) -> Message {
    super::widgets::ev(Event::Compose(e))
}

/// A number as typed: a comma for the point; none for a blank or unreadable field.
fn parse(text: &str) -> Option<f64> {
    let t = kentos_processing::text::js_trim(text);
    let v = kentos_native_style::classify::js_number(&t.replacen(',', ".", 1));
    (!t.is_empty() && v.is_finite()).then_some(v)
}

impl LayerStyleWindow {
    /// The draft's mask, a new one when it has none.
    fn mask_or_new(&self) -> LayerMask {
        self.compose
            .as_ref()
            .and_then(|c| c.mask.clone())
            .unwrap_or(LayerMask {
                labels: false,
                markers: false,
                size: 1.0,
                targets: Vec::new(),
            })
    }

    /// The draft with `f` applied, kept as the layer keeps it (`LayerCompose::normalized`).
    fn change(&mut self, f: impl FnOnce(&mut LayerCompose)) {
        let mut c = self.compose.clone().unwrap_or_default();
        f(&mut c);
        self.compose = c.normalized();
    }

    fn set_mask(&mut self, f: impl FnOnce(&mut LayerMask)) {
        let mut m = self.mask_or_new();
        f(&mut m);
        let keep = m.labels || m.markers;
        self.change(|c| c.mask = keep.then_some(m));
    }

    pub(super) fn compose_edit(&mut self, e: Edit) {
        match e {
            Edit::Transparency(text) => {
                if let Some(v) = parse(&text) {
                    let opacity = 1.0 - v.clamp(0.0, 100.0) / 100.0;
                    self.change(|c| c.opacity = Some(opacity));
                }
                self.typed.insert("compose.transparency".into(), text);
            }
            Edit::Blend(mode) => self.change(|c| c.blend = Some(mode)),
            Edit::Labels(on) => self.set_mask(|m| m.labels = on),
            Edit::Markers(on) => self.set_mask(|m| m.markers = on),
            Edit::Size(text) => {
                if let Some(v) = parse(&text) {
                    self.set_mask(|m| m.size = v.clamp(0.0, MAX_MASK_SIZE));
                }
                self.typed.insert("compose.size".into(), text);
            }
            Edit::Target(id, on) => self.set_mask(|m| {
                m.targets.retain(|t| *t != id);
                if on {
                    m.targets.push(id);
                }
            }),
        }
    }
}

/// A row of the section: its label at the left, the control beside it, a hint under it.
fn line<'a>(
    label_text: &str,
    control: impl Into<Element<'a, Message>>,
    hint: Option<&str>,
) -> Element<'a, Message> {
    let mut right = column![control.into()].spacing(4).width(Fill);
    if let Some(h) = hint {
        right = right.push(fields::hint(h));
    }
    row![
        container(label::caption(label_text.to_owned()).style(style::text::muted))
            .width(160)
            .padding(iced::Padding {
                top: 5.0,
                ..iced::Padding::ZERO
            }),
        right
    ]
    .spacing(14)
    .into()
}

/// The section, under the kind's panel: `layers` the drawing's other layers (id, name).
pub(super) fn section<'a>(
    window: &LayerStyleWindow,
    layers: Vec<(String, String)>,
) -> Element<'a, Message> {
    let c = window.compose.clone().unwrap_or_default();
    let typed = |k: &str| window.typed.get(k).cloned();
    let transparency = typed("compose.transparency").unwrap_or_else(|| {
        kentos_native_style::classify::rounded((1.0 - c.opacity.unwrap_or(1.0)) * 100.0, 0)
    });
    let modes: Vec<(&'static str, &'static str)> = BlendMode::ALL
        .iter()
        .map(|m| (m.key(), m.label()))
        .collect();
    let blend = c.blend.unwrap_or(BlendMode::Normal);
    let mask = c.mask.clone();
    let (labels, markers) = mask
        .as_ref()
        .map_or((false, false), |m| (m.labels, m.markers));
    let mut rows: Vec<Element<'a, Message>> = vec![
        row![
            kentos_ui::icon::icon(crate::icons::from_web(Some("layerCompose"))).size(16.0),
            label::body("Katman çizimi").font(kentos_ui::theme::typography::ui_strong())
        ]
        .spacing(6)
        .align_y(Center)
        .into(),
        line(
            "Saydamlık",
            container(fields::number(
                iced::widget::Id::from("lsty:compose.transparency"),
                &transparency,
                Some("%"),
                parse(&transparency).is_none_or(|v| !(0.0..=100.0).contains(&v)),
                |t| ev(Edit::Transparency(t)),
                ev(Edit::Transparency(transparency.clone())),
            ))
            .max_width(NUMBER),
            None,
        ),
        line(
            "Karışım kipi",
            container(fields::select(&modes, blend.key(), |k| {
                ev(Edit::Blend(
                    BlendMode::from_key(k).unwrap_or(BlendMode::Normal),
                ))
            }))
            .max_width(260),
            Some(
                "Katmanın renkleri alttaki çizimle bu kurala göre karışır (Çoğalt: gölgeli kabartma renklerin üstünde).",
            ),
        ),
        line(
            "Maske",
            row![
                fields::check(
                    Check::from(labels),
                    "Etiketler",
                    Some(ev(Edit::Labels(!labels)))
                ),
                fields::check(
                    Check::from(markers),
                    "İşaretler",
                    Some(ev(Edit::Markers(!markers)))
                ),
            ]
            .spacing(16),
            Some("Bu katmanın etiketlerinin ve işaretlerinin altında seçilen katmanlar boşalır."),
        ),
    ];
    if let Some(m) = mask.filter(|m| m.labels || m.markers) {
        let size = typed("compose.size")
            .unwrap_or_else(|| kentos_native_style::classify::rounded(m.size, 3));
        rows.push(line(
            "Boy",
            container(fields::number(
                iced::widget::Id::from("lsty:compose.size"),
                &size,
                Some("mm"),
                parse(&size).is_none_or(|v| !(0.0..=MAX_MASK_SIZE).contains(&v)),
                |t| ev(Edit::Size(t)),
                ev(Edit::Size(size.clone())),
            ))
            .max_width(NUMBER),
            Some("Ekranda; köşeler bu yarıçapla yuvarlanır."),
        ));
        let list: Element<'a, Message> = if layers.is_empty() {
            fields::hint("Maskelenecek başka katman yok.")
        } else {
            let checks = layers.into_iter().map(|(id, name)| {
                let on = m.targets.contains(&id);
                fields::check(Check::from(on), name, Some(ev(Edit::Target(id, !on))))
            });
            container(
                scrollable(Column::with_children(checks).spacing(2).width(Fill))
                    .direction(style::field::thin_scrollbar())
                    .height(132),
            )
            .padding([4, 8])
            .max_width(LIST)
            .style(style::container::field_box)
            .into()
        };
        rows.push(line("Maskelenen katmanlar", list, None));
    }
    Column::with_children(rows).spacing(10).width(Fill).into()
}

impl crate::app::App {
    /// The window's title as it shows it: “Katman stili: <layer>”.
    pub(crate) fn layer_style_title(&self) -> String {
        let name = self
            .styles
            .layer_style
            .as_ref()
            .zip(self.document.as_ref())
            .and_then(|(w, d)| d.model.layers().get(&w.layer).map(|n| n.name.clone()))
            .unwrap_or_default();
        format!("Katman stili: {name}")
    }

    /// What a trace's `dialog` step does in Katman stili (fixtures/interaction/README.md): Katman çizimi's
    /// switches and its masked layers by their names (docs/adr/0226 §3, §4), Uygula, Tamam and Vazgeç.
    pub(crate) fn layer_style_control(
        &self,
        control: crate::traces::Control<'_>,
    ) -> Result<Option<Message>, String> {
        use crate::traces::Control;
        let (Some(w), Some(doc)) = (&self.styles.layer_style, &self.document) else {
            return Err("Katman stili penceresi açık değil".to_owned());
        };
        let c = w.compose.clone().unwrap_or_default();
        let mask = c.mask.as_ref();
        Ok(match control {
            Control::Check("Etiketler", on) => {
                (mask.is_some_and(|m| m.labels) != on).then(|| ev(Edit::Labels(on)))
            }
            Control::Check("İşaretler", on) => {
                (mask.is_some_and(|m| m.markers) != on).then(|| ev(Edit::Markers(on)))
            }
            Control::Fill("Karışım kipi", words) => {
                let mode = BlendMode::ALL
                    .into_iter()
                    .find(|m| m.label() == words)
                    .ok_or_else(|| format!("“Karışım kipi” listesinde “{words}” yok"))?;
                Some(ev(Edit::Blend(mode)))
            }
            Control::Press("Uygula") => Some(super::widgets::ev(Event::Apply)),
            Control::Press("Tamam") => Some(super::widgets::ev(Event::Done)),
            Control::Press("Vazgeç") => Some(super::widgets::ev(Event::Close)),
            Control::Fill("Saydamlık", text) => Some(ev(Edit::Transparency(text.to_owned()))),
            Control::Fill("Boy", text) => Some(ev(Edit::Size(text.to_owned()))),
            Control::Check(name, want) => {
                // A masked layer's box, by the layer's name (shown only while the mask has a source).
                let shown = mask.is_some_and(|m| m.labels || m.markers);
                let found = doc
                    .model
                    .layers()
                    .leaves()
                    .into_iter()
                    .find(|n| n.id != w.layer && n.name == name)
                    .filter(|_| shown)
                    .ok_or_else(|| format!("“Katman stili” penceresinde “{name}” kutusu yok"))?;
                let on = mask.is_some_and(|m| m.targets.contains(&found.id));
                (on != want).then(|| ev(Edit::Target(found.id.clone(), want)))
            }
            other => return Err(format!("“Katman stili” penceresinde {other} yok")),
        })
    }
}
