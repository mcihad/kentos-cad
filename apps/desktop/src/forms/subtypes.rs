//! Alt türler (docs/adr/0422 §5, `data.subtypes`): a layer's subtypes.
//!
//! ```text
//! ┌ Alt türler ────────────────────────────────────────────────────────────────┐
//! │ Katman [Bina              ▾]   Alt tür alanı [Tür               ▾]        │
//! │ Alt türler            │ Ad [Sanayi binası            ]                      │
//! │ ☑ Konut               │ Alan        Zorunlu  Varsayılan   Sunulan değerler │
//! │ ☑ Sanayi binası  ◀    │ Malzeme       ☑      [Çelik ▾]   (Betonarme)(Çelik)│
//! │ ☐ Depo                │ Not           ☑      [        ]                    │
//! │                                                     [Vazgeç] [Kaydet]      │
//! └────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy; Kaydet writes it through `cad.layers.subtypes` in one step and closes.

use iced::widget::{Column, Row, button, column, container, row, scrollable, space, text_input};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    LayerField, LayerNodeType, LayerSubtypes, LayersSubtypes, Subtype, SubtypeField,
    subtypes_problem,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, layers_subtypes};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Dialog as Frame, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Alt türler";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NONE: &str = "Yok";

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    /// The layer whose subtypes these are.
    pub layer: String,
    /// The subtype field's key; none: the layer has no subtypes.
    pub field: Option<String>,
    /// The subtypes being edited, in the subtype field's list order.
    pub types: Vec<Subtype>,
    kept: Option<LayerSubtypes>,
    /// The chosen subtype's code.
    pub chosen: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Layer(String),
    /// The subtype field, by key; none takes the subtypes away.
    Field(Option<String>),
    /// A code of the subtype field's list made a subtype, or no longer one.
    Toggle(String),
    Choose(String),
    Label(String),
    /// A field's rule in the chosen subtype, by the field's key.
    Required(String, bool),
    Default(String, Option<String>),
    /// A code of a listed field offered or no longer offered.
    Offer(String, String),
    /// Every value of a listed field offered again.
    OfferAll(String),
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::Subtypes(event)
}

/// The rule of `key` in `t`, made when it has none.
fn rule_mut<'a>(t: &'a mut Subtype, key: &str) -> &'a mut SubtypeField {
    if let Some(i) = t.fields.iter().position(|r| r.field == key) {
        return &mut t.fields[i];
    }
    t.fields.push(SubtypeField {
        field: key.to_owned(),
        values: None,
        default: None,
        required: false,
    });
    t.fields.last_mut().expect("just pushed")
}

/// `t` without the rules that say nothing.
fn prune(t: &mut Subtype) {
    t.fields
        .retain(|r| r.values.is_some() || r.default.is_some() || r.required);
}

impl Window {
    /// The subtypes the window would write.
    pub(crate) fn subtypes(&self) -> Option<LayerSubtypes> {
        let field = self.field.clone()?;
        if self.types.is_empty() {
            return None;
        }
        Some(LayerSubtypes {
            field,
            types: self.types.clone(),
        })
    }
}

impl App {
    /// Alt türler: the window on the active layer's subtypes (the table's layer when it shows one).
    pub(crate) fn open_subtypes(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let layers = self.subtype_layers();
        let wanted = self
            .features
            .layer
            .clone()
            .or_else(|| Some(doc.model.layers().active().to_owned()));
        let Some(layer) = wanted
            .filter(|l| layers.iter().any(|(id, _)| id == l))
            .or_else(|| layers.first().map(|(id, _)| id.clone()))
        else {
            self.warn(
                "Değer listeli alanı olan katman yok; alt türler katmanın değer listeli bir alanıyla kurulur. Önce Alanlar penceresinde bir liste alanı ekleyin.",
            );
            return Task::none();
        };
        self.subtypes = Some(self.subtypes_window_of(layer));
        self.dialog = Some(Dialog::Subtypes);
        Task::none()
    }

    fn subtypes_window_of(&self, layer: String) -> Window {
        let kept = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer))
            .and_then(|n| n.subtypes.clone());
        Window {
            layer,
            field: kept.as_ref().map(|s| s.field.clone()),
            types: kept.as_ref().map(|s| s.types.clone()).unwrap_or_default(),
            chosen: kept
                .as_ref()
                .and_then(|s| s.types.first().map(|t| t.code.clone())),
            kept,
        }
    }

    pub(crate) fn subtypes_event(&mut self, event: Event) -> Task<Message> {
        if let Event::Layer(id) = &event {
            self.subtypes = Some(self.subtypes_window_of(id.clone()));
            return Task::none();
        }
        let fields = self
            .subtypes
            .as_ref()
            .map(|w| self.layer_fields_of(&w.layer))
            .unwrap_or_default();
        let Some(w) = self.subtypes.as_mut() else {
            return Task::none();
        };
        let order: Vec<String> = w
            .field
            .as_ref()
            .and_then(|k| fields.iter().find(|f| f.key() == k))
            .and_then(|f| f.values.as_ref())
            .map(|v| v.iter().map(|c| c.code.clone()).collect())
            .unwrap_or_default();
        let chosen = w.chosen.clone();
        let current = chosen
            .as_ref()
            .and_then(|c| w.types.iter_mut().find(|t| t.code == *c));
        match event {
            Event::Layer(_) => {}
            Event::Field(key) => {
                if w.field != key {
                    w.field = key;
                    w.types.clear();
                    w.chosen = None;
                }
            }
            Event::Toggle(code) => {
                if let Some(i) = w.types.iter().position(|t| t.code == code) {
                    w.types.remove(i);
                    if w.chosen.as_deref() == Some(code.as_str()) {
                        w.chosen = w.types.first().map(|t| t.code.clone());
                    }
                } else {
                    w.types.push(Subtype {
                        code: code.clone(),
                        label: String::new(),
                        fields: Vec::new(),
                    });
                    // The subtypes in the list's order.
                    w.types
                        .sort_by_key(|t| order.iter().position(|c| *c == t.code));
                    w.chosen = Some(code);
                }
            }
            Event::Choose(code) => {
                if w.types.iter().any(|t| t.code == code) {
                    w.chosen = Some(code);
                }
            }
            Event::Label(text) => {
                if let Some(t) = current {
                    t.label = text;
                }
            }
            Event::Required(key, on) => {
                if let Some(t) = current {
                    rule_mut(t, &key).required = on;
                    prune(t);
                }
            }
            Event::Default(key, value) => {
                if let Some(t) = current {
                    rule_mut(t, &key).default = value.filter(|v| !v.is_empty());
                    prune(t);
                }
            }
            Event::Offer(key, code) => {
                if let Some(t) = current {
                    let all: Vec<String> = fields
                        .iter()
                        .find(|f| f.key() == key)
                        .and_then(|f| f.values.as_ref())
                        .map(|v| v.iter().map(|c| c.code.clone()).collect())
                        .unwrap_or_default();
                    let r = rule_mut(t, &key);
                    let mut now = r.values.clone().unwrap_or_else(|| all.clone());
                    if let Some(i) = now.iter().position(|c| *c == code) {
                        now.remove(i);
                    } else {
                        now.push(code);
                    }
                    now.sort_by_key(|c| all.iter().position(|a| a == c));
                    r.values = (now != all).then_some(now);
                    // A default the subtype no longer offers goes with it.
                    if let (Some(d), Some(v)) = (&r.default, &r.values)
                        && !v.contains(d)
                    {
                        r.default = None;
                    }
                    prune(t);
                }
            }
            Event::OfferAll(key) => {
                if let Some(t) = current {
                    rule_mut(t, &key).values = None;
                    prune(t);
                }
            }
            Event::Cancel => {
                self.subtypes = None;
                self.dialog = None;
            }
            Event::Save => return self.save_subtypes(),
        }
        Task::none()
    }

    /// Kaydet: the subtypes through `cad.layers.subtypes` in one step; the window closes when they go in.
    fn save_subtypes(&mut self) -> Task<Message> {
        let Some(w) = self.subtypes.clone() else {
            return Task::none();
        };
        let next = w.subtypes();
        if next == w.kept {
            self.subtypes = None;
            self.dialog = None;
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = layers_subtypes::execute(
            &mut ExecutionContext::new(&mut doc.model),
            LayersSubtypes {
                layer: w.layer.clone(),
                subtypes: next.clone(),
                expected_revision: None,
            },
        );
        if self.answered(result).is_none() {
            return Task::none();
        }
        self.subtypes = None;
        self.dialog = None;
        let name = self.form_layer_name(&w.layer);
        let said = match next {
            Some(s) => format!("“{name}” katmanının {} alt türü kaydedildi.", s.types.len()),
            None => format!("“{name}” katmanının alt türleri kaldırıldı."),
        };
        self.say(Level::Success, said);
        Task::none()
    }

    /// The layers with a single-valued listed field, by their path.
    fn subtype_layers(&self) -> Vec<(String, String)> {
        let Some(doc) = &self.document else {
            return Vec::new();
        };
        let mut out = Vec::new();
        fn walk(nodes: &[kentos_commands::LayerNode], at: &str, out: &mut Vec<(String, String)>) {
            for n in nodes {
                let path = if at.is_empty() {
                    n.name.clone()
                } else {
                    format!("{at} / {}", n.name)
                };
                if n.kind == LayerNodeType::Layer
                    && n.service.is_none()
                    && n.fields.iter().any(sorts)
                {
                    out.push((n.id.clone(), path.clone()));
                }
                walk(&n.children, &path, out);
            }
        }
        walk(doc.model.layers().nodes(), "", &mut out);
        out
    }

    pub(crate) fn subtypes_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.subtypes else {
            return iced::widget::text("").into();
        };
        let layers = self.subtype_layers();
        let ids: Vec<String> = layers.iter().map(|(id, _)| id.clone()).collect();
        let layer = Select::new(
            layers.iter().map(|(_, n)| Choice::new(n.clone())),
            ids.iter().position(|id| *id == w.layer),
            move |i| msg(Event::Layer(ids[i].clone())),
        );
        let fields = self.layer_fields_of(&w.layer);
        let sorters: Vec<&LayerField> = fields.iter().filter(|f| sorts(f)).collect();
        let keys: Vec<String> = sorters.iter().map(|f| f.key().to_owned()).collect();
        let mut choices = vec![Choice::new(NONE)];
        choices.extend(sorters.iter().map(|f| Choice::new(f.label().to_owned())));
        let at = match &w.field {
            None => Some(0),
            Some(k) => keys.iter().position(|x| x == k).map(|i| i + 1),
        };
        let sorter_pick = Select::new(choices, at, move |i| {
            msg(Event::Field(i.checked_sub(1).map(|i| keys[i].clone())))
        });
        let top = row![
            words::field("Katman", container(layer).width(280), None),
            words::field(
                "Alt tür alanı",
                container(sorter_pick).width(240),
                Some("Kayıtları türlerine ayıran değer listeli alan.".into()),
            ),
        ]
        .spacing(24);
        let sorter = w
            .field
            .as_ref()
            .and_then(|k| fields.iter().find(|f| f.key() == k));
        let body: Element<'_, Message> = match sorter {
            None => container(label::caption(
                "Alt tür yok. Bir alt tür alanı seçin: listesinin her değeri bir alt tür olabilir, her alt tür öbür alanların sunduğu değerleri, varsayılanlarını ve zorunluluğunu kendi kayıtları için daraltır.",
            ).style(muted))
            .padding(8)
            .into(),
            Some(sorter) => self.subtypes_body(w, sorter, &fields),
        };
        let problem = w
            .subtypes()
            .and_then(|s| subtypes_problem(&fields, &s).map(|(_, why)| why))
            .or_else(|| {
                (w.field.is_some() && w.types.is_empty()).then(|| {
                    "Alt tür olarak en az bir değer seçin ya da alt tür alanını Yok yapın."
                        .to_owned()
                })
            });
        let mut col = column![top, body].spacing(14);
        if let Some(why) = &problem {
            col = col.push(words::text_line(Kind::Warn, why.clone()));
        }
        col = col.push(
            row![
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, problem.is_none().then(|| msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center),
        );
        overlay::modal(Frame::new(TITLE).push(col).width(900.0), msg(Event::Cancel))
    }

    fn subtypes_body<'a>(
        &'a self,
        w: &'a Window,
        sorter: &LayerField,
        fields: &[LayerField],
    ) -> Element<'a, Message> {
        let mut list = Column::new().spacing(1).padding(2);
        for c in sorter.values.iter().flatten().filter(|c| !c.retired) {
            let made = w.types.iter().find(|t| t.code == c.code);
            let name = made
                .map(|t| t.label.clone())
                .filter(|l| !l.is_empty())
                .unwrap_or_else(|| c.label.clone());
            let rules = made.map_or(0, |t| t.fields.len());
            let face = row![
                words::check(made.is_some(), "", Some(msg(Event::Toggle(c.code.clone())))),
                column![
                    label::body(name),
                    label::caption(if made.is_some() {
                        format!("{} · {rules} alan kuralı", c.code)
                    } else {
                        format!("{} · alt tür değil", c.code)
                    })
                    .style(muted),
                ]
                .spacing(1)
                .width(Fill),
            ]
            .spacing(6)
            .align_y(Center);
            list = list.push(
                button(face)
                    .on_press(if made.is_some() {
                        msg(Event::Choose(c.code.clone()))
                    } else {
                        msg(Event::Toggle(c.code.clone()))
                    })
                    .padding([3, 6])
                    .width(Fill)
                    .style(style::button::row(
                        made.is_some() && w.chosen.as_deref() == Some(c.code.as_str()),
                    )),
            );
        }
        let left = column![
            label::strong("Alt türler"),
            container(scrollable(list).height(typography::scaled(300.0)))
                .style(style::container::field_box)
                .width(Fill),
            label::caption(
                "İşaretli değerler alt türdür; birine basınca kuralları sağda düzenlenir."
            )
            .style(muted),
        ]
        .spacing(8);
        let chosen = w
            .chosen
            .as_ref()
            .and_then(|c| w.types.iter().find(|t| t.code == *c));
        let right: Element<'_, Message> = match chosen {
            None => container(label::caption(
                "Listeden bir değeri alt tür yapın ve seçin: o türün kayıtları için alanların kuralları burada.",
            ))
            .padding(12)
            .into(),
            Some(t) => self.subtype_editor(sorter, t, fields),
        };
        row![container(left).width(260), container(right).width(Fill)]
            .spacing(16)
            .into()
    }

    fn subtype_editor<'a>(
        &'a self,
        sorter: &LayerField,
        t: &'a Subtype,
        fields: &[LayerField],
    ) -> Element<'a, Message> {
        let listed_label = sorter
            .values
            .iter()
            .flatten()
            .find(|c| c.code == t.code)
            .map_or(t.code.clone(), |c| c.label.clone());
        let name = text_input(&listed_label, &t.label)
            .size(typography::body())
            .on_input(|s| msg(Event::Label(s)))
            .padding([5, 8])
            .style(style::field::input);
        let mut rows = Column::new().spacing(6);
        rows = rows.push(
            row![
                container(label::caption("Alan").style(muted)).width(140),
                container(label::caption("Zorunlu").style(muted)).width(64),
                container(label::caption("Varsayılan").style(muted)).width(170),
                label::caption("Sunulan değerler").style(muted),
            ]
            .spacing(8),
        );
        for f in fields.iter().filter(|f| f.key() != sorter.key()) {
            rows = rows.push(field_line(t, f));
        }
        column![
            words::field(
                "Ad",
                container(focus_ring(name)).width(320),
                Some(format!("Boş bırakılırsa listedeki adı: {listed_label}.")),
            ),
            container(scrollable(rows).height(typography::scaled(280.0)))
                .padding([4, 0])
                .width(Fill),
            label::caption(
                "Bu türün kayıtları yalnız sunulan değerleri alır; tür değişince eski değerler kalır, formda ve uyarıyla söylenir.",
            )
            .style(muted),
        ]
        .spacing(10)
        .into()
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn subtypes_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        if self.subtypes.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// Whether `f` can sort a layer's records: a single-valued field with a value list.
fn sorts(f: &LayerField) -> bool {
    f.values.as_ref().is_some_and(|v| !v.is_empty()) && !f.holds_many()
}

/// One field's rule in the subtype: Zorunlu, Varsayılan and, for a listed field, the values offered.
fn field_line<'a>(t: &'a Subtype, f: &LayerField) -> Element<'a, Message> {
    let key = f.key().to_owned();
    let rule = t.fields.iter().find(|r| r.field == key);
    let required = rule.is_some_and(|r| r.required);
    let k = key.clone();
    let check = words::check(
        required || f.required,
        "",
        (!f.required).then(|| msg(Event::Required(k, !required))),
    );
    let default: Element<'a, Message> = match &f.values {
        Some(list) if !list.is_empty() => {
            let offered: Vec<(String, String)> = list
                .iter()
                .filter(|c| !c.retired)
                .filter(|c| {
                    rule.and_then(|r| r.values.as_ref())
                        .is_none_or(|v| v.contains(&c.code))
                })
                .map(|c| (c.code.clone(), c.label.clone()))
                .collect();
            let mut choices = vec![Choice::new("—")];
            choices.extend(offered.iter().map(|(_, l)| Choice::new(l.clone())));
            let at = match rule.and_then(|r| r.default.as_ref()) {
                None => Some(0),
                Some(d) => offered.iter().position(|(c, _)| c == d).map(|i| i + 1),
            };
            let codes: Vec<String> = offered.iter().map(|(c, _)| c.clone()).collect();
            let k = key.clone();
            Select::new(choices, at, move |i| {
                msg(Event::Default(
                    k.clone(),
                    i.checked_sub(1).map(|i| codes[i].clone()),
                ))
            })
            .into()
        }
        _ => {
            let k = key.clone();
            text_input(
                f.default.as_deref().unwrap_or(""),
                rule.and_then(|r| r.default.as_deref()).unwrap_or(""),
            )
            .size(typography::body())
            .on_input(move |s| msg(Event::Default(k.clone(), Some(s))))
            .padding([4, 8])
            .style(style::field::input)
            .into()
        }
    };
    let offers: Element<'a, Message> = match &f.values {
        Some(list) if !list.is_empty() => {
            let narrowed = rule.and_then(|r| r.values.as_ref());
            let mut chips = Row::new().spacing(4);
            for c in list.iter().filter(|c| !c.retired) {
                let on = narrowed.is_none_or(|v| v.contains(&c.code));
                chips = chips.push(
                    button(label::caption(c.label.clone()))
                        .on_press(msg(Event::Offer(key.clone(), c.code.clone())))
                        .padding([2, 8])
                        .style(style::button::chip(on)),
                );
            }
            let all = button(label::caption("Tümü"))
                .on_press_maybe(
                    narrowed
                        .is_some()
                        .then(|| msg(Event::OfferAll(key.clone()))),
                )
                .padding([2, 8])
                .style(style::button::ghost);
            row![chips.wrap(), all].spacing(6).align_y(Center).into()
        }
        _ => label::caption("—").style(muted).into(),
    };
    row![
        container(label::body(f.label().to_owned())).width(140),
        container(check).width(64),
        container(default).width(170),
        container(offers).width(Fill),
    ]
    .spacing(8)
    .align_y(Center)
    .into()
}

fn muted(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}
