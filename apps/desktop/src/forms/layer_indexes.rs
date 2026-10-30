//! Dizinler (docs/adr/0758, `data.layerIndexes`): a layer's indexes and keys.
//!
//! ```text
//! ┌ Dizinler ──────────────────────────────────────────────────────────────────┐
//! │ Katman [Parsel                ▾]                                           │
//! │ Dizinler            │ Ad [Ada ve parsel              ]  ☑ Tekil (anahtar)  │
//! │ Ada ve parsel       │ Alanlar  (Ada) (Parsel) (Mahalle) (Alan)             │
//! │  anahtar · uyuyor   │  1  Ada                                   ˄ ˅ ×      │
//! │ Mahalle             │  2  Parsel                                ˄ ˅ ×      │
//! │  aranır             │ ☐ Boş değerler aynı sayılır                          │
//! │ [Yeni dizin]        │ ✓ Katmanın kayıtları anahtara uyuyor.                │
//! │ [Dizini sil]                                         [Vazgeç] [Kaydet]     │
//! └────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy; Kaydet writes it through `cad.layers.indexes` in one step and closes. A unique index says
//! whether the layer's records already keep it: one they break is refused when written.

use iced::widget::{Column, Row, button, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    LayerField, LayerFieldKind, LayerIndex, LayerNodeType, LayersIndexes, layer_indexes_problem,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, layers_indexes};
use kentos_ui::icon::icon;
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::icons::from_web;
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Dizinler";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni dizin";
const REMOVE: &str = "Dizini sil";

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    pub layer: String,
    pub indexes: Vec<LayerIndex>,
    kept: Vec<LayerIndex>,
    pub chosen: Option<usize>,
    /// What the unique indexes found in the records, for the drawing's revision and the indexes it was found for: the
    /// view is built every frame, the records are read once.
    judged: std::cell::RefCell<Option<Judged>>,
}

/// The drawing's revision and the indexes the broken records were found for, with what was found.
type Judged = (u64, Vec<LayerIndex>, Vec<Broken>);

#[derive(Debug, Clone)]
pub enum Event {
    Layer(String),
    Choose(usize),
    New,
    Remove,
    Name(String),
    Unique(bool),
    NullsEqual(bool),
    /// A field put in (at the end) or taken out, by key.
    ToggleField(String),
    /// A field of the index moved up (`true`) or down.
    Move(usize, bool),
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::LayerIndexes(event)
}

/// A field an index takes: one holding bytes is not indexed.
fn indexable(f: &LayerField) -> bool {
    f.kind != LayerFieldKind::Binary
}

/// What a unique index finds in the layer's records now: the values two or more of them hold, and how many do.
type Broken = Option<(String, usize)>;

impl App {
    /// Dizinler: the window on the active layer's indexes (the table's layer when it shows one).
    pub(crate) fn open_layer_indexes(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let layers = self.index_layers();
        let wanted = self
            .features
            .layer
            .clone()
            .or_else(|| Some(doc.model.layers().active().to_owned()));
        let Some(layer) = wanted
            .filter(|l| layers.iter().any(|(id, _)| id == l))
            .or_else(|| layers.first().map(|(id, _)| id.clone()))
        else {
            self.warn("Alanı olan katman yok; dizin bir katmanın alanlarını sıralar. Önce Alanlar penceresinden alan ekleyin.");
            return Task::none();
        };
        self.layer_indexes = Some(self.layer_indexes_window_of(layer));
        self.dialog = Some(Dialog::LayerIndexes);
        Task::none()
    }

    fn layer_indexes_window_of(&self, layer: String) -> Window {
        let indexes = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer))
            .map(|n| n.indexes.clone())
            .unwrap_or_default();
        Window {
            layer,
            chosen: (!indexes.is_empty()).then_some(0),
            kept: indexes.clone(),
            indexes,
            judged: std::cell::RefCell::new(None),
        }
    }

    pub(crate) fn layer_indexes_event(&mut self, event: Event) -> Task<Message> {
        if let Event::Layer(id) = &event {
            self.layer_indexes = Some(self.layer_indexes_window_of(id.clone()));
            return Task::none();
        }
        let Some(w) = self.layer_indexes.as_mut() else {
            return Task::none();
        };
        let at = w.chosen.filter(|i| *i < w.indexes.len());
        let chosen = at.and_then(|i| w.indexes.get_mut(i));
        match event {
            Event::Layer(_) => {}
            Event::Choose(i) => w.chosen = Some(i),
            Event::New => {
                let mut n = w.indexes.len() + 1;
                let taken = |name: &str, all: &[LayerIndex]| {
                    all.iter()
                        .any(|x| kentos_commands::fold(&x.name) == kentos_commands::fold(name))
                };
                while taken(&format!("Dizin {n}"), &w.indexes) {
                    n += 1;
                }
                w.indexes.push(LayerIndex {
                    name: format!("Dizin {n}"),
                    fields: Vec::new(),
                    unique: false,
                    nulls_equal: false,
                });
                w.chosen = Some(w.indexes.len() - 1);
            }
            Event::Remove => {
                if let Some(i) = at {
                    w.indexes.remove(i);
                    w.chosen = (!w.indexes.is_empty()).then_some(i.min(w.indexes.len() - 1));
                }
            }
            Event::Name(text) => {
                if let Some(x) = chosen {
                    x.name = text;
                }
            }
            Event::Unique(on) => {
                if let Some(x) = chosen {
                    x.unique = on;
                    x.nulls_equal &= on;
                }
            }
            Event::NullsEqual(on) => {
                if let Some(x) = chosen {
                    x.nulls_equal = on && x.unique;
                }
            }
            Event::ToggleField(key) => {
                if let Some(x) = chosen {
                    match x.fields.iter().position(|k| *k == key) {
                        Some(i) => {
                            x.fields.remove(i);
                        }
                        None => x.fields.push(key),
                    }
                }
            }
            Event::Move(i, up) => {
                if let Some(x) = chosen {
                    let j = if up { i.checked_sub(1) } else { Some(i + 1) };
                    if let Some(j) = j.filter(|j| *j < x.fields.len())
                        && i < x.fields.len()
                    {
                        x.fields.swap(i, j);
                    }
                }
            }
            Event::Cancel => {
                self.layer_indexes = None;
                self.dialog = None;
            }
            Event::Save => return self.save_layer_indexes(),
        }
        Task::none()
    }

    /// Kaydet: the indexes through `cad.layers.indexes` in one step; the window closes when they go in.
    fn save_layer_indexes(&mut self) -> Task<Message> {
        let Some(w) = self.layer_indexes.clone() else {
            return Task::none();
        };
        if w.indexes == w.kept {
            self.layer_indexes = None;
            self.dialog = None;
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = layers_indexes::execute(
            &mut ExecutionContext::new(&mut doc.model),
            LayersIndexes {
                layer: w.layer.clone(),
                indexes: w.indexes.clone(),
                expected_revision: None,
            },
        );
        if self.answered(result).is_none() {
            return Task::none();
        }
        self.layer_indexes = None;
        self.dialog = None;
        let name = self.form_layer_name(&w.layer);
        let keys = w.indexes.iter().filter(|x| x.unique).count();
        let said = if w.indexes.is_empty() {
            format!("“{name}” katmanının dizinleri kaldırıldı.")
        } else if keys == 0 {
            format!("“{name}” katmanının {} dizini kaydedildi.", w.indexes.len())
        } else {
            format!(
                "“{name}” katmanının {} dizini kaydedildi; {keys} tekil dizini her yazma sorar.",
                w.indexes.len()
            )
        };
        self.say(Level::Success, said);
        Task::none()
    }

    /// The layers holding objects with fields, by their path.
    fn index_layers(&self) -> Vec<(String, String)> {
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
                if n.kind == LayerNodeType::Layer && n.service.is_none() && !n.fields.is_empty() {
                    out.push((n.id.clone(), path.clone()));
                }
                walk(&n.children, &path, out);
            }
        }
        walk(doc.model.layers().nodes(), "", &mut out);
        out
    }

    /// Each unique index asked of the layer's records now: the first values two or more hold, and how many do.
    fn judge_indexes(&self, w: &Window, fields: &[LayerField]) -> Vec<Broken> {
        let node = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&w.layer).map(|n| (d, n)));
        let revision = node.map_or(0, |(d, _)| d.model.revision());
        if let Some((at, of, judged)) = w.judged.borrow().as_ref()
            && *at == revision
            && *of == w.indexes
        {
            return judged.clone();
        }
        let judged: Vec<Broken> = w
            .indexes
            .iter()
            .map(|x| {
                let (doc, node) = node?;
                if !x.unique || x.fields.is_empty() {
                    return None;
                }
                let (values, uids) = layers_indexes::duplicates(&doc.model, node, x)?;
                let cols: Vec<&LayerField> = x
                    .fields
                    .iter()
                    .filter_map(|k| fields.iter().find(|f| f.key() == k))
                    .collect();
                Some((layers_indexes::said_values(&cols, &values), uids.len()))
            })
            .collect();
        *w.judged.borrow_mut() = Some((revision, w.indexes.clone(), judged.clone()));
        judged
    }

    pub(crate) fn layer_indexes_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.layer_indexes else {
            return iced::widget::text("").into();
        };
        let layers = self.index_layers();
        let ids: Vec<String> = layers.iter().map(|(id, _)| id.clone()).collect();
        let layer = Select::new(
            layers.iter().map(|(_, n)| Choice::new(n.clone())),
            ids.iter().position(|id| *id == w.layer),
            move |i| msg(Event::Layer(ids[i].clone())),
        );
        let fields = self.layer_fields_of(&w.layer);
        let judged = self.judge_indexes(w, &fields);
        let mut list = Column::new().spacing(1).padding(2);
        for (i, x) in w.indexes.iter().enumerate() {
            let what = match (x.unique, &judged[i]) {
                (false, _) => "aranır".to_owned(),
                (true, None) => "anahtar · kayıtlar uyuyor".to_owned(),
                (true, Some((_, n))) => format!("anahtar · {n} kayıt bozuyor"),
            };
            let names: Vec<String> = x
                .fields
                .iter()
                .map(|k| {
                    fields
                        .iter()
                        .find(|f| f.key() == k)
                        .map_or(k.clone(), |f| f.label().to_owned())
                })
                .collect();
            let face = column![
                label::body(x.name.clone()),
                label::caption(if names.is_empty() {
                    format!("alan yok · {what}")
                } else {
                    format!("{} · {what}", names.join(", "))
                })
                .style(muted),
            ]
            .spacing(1);
            list = list.push(
                button(face)
                    .on_press(msg(Event::Choose(i)))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(w.chosen == Some(i))),
            );
        }
        if w.indexes.is_empty() {
            list = list.push(
                label::caption(
                    "Dizin yok. Yeni dizin ile kayıtların arandığı alanları ya da katmanın anahtarını ekleyin.",
                )
                .style(muted),
            );
        }
        let at = w.chosen.filter(|i| *i < w.indexes.len());
        let left = column![
            label::strong("Dizinler"),
            container(scrollable(list).height(typography::scaled(300.0)))
                .style(style::container::field_box)
                .width(Fill),
            words::secondary(NEW, Some(msg(Event::New))),
        ]
        .spacing(8);
        let right: Element<'_, Message> = match at {
            None => container(label::caption(
                "Bir dizin seçin ya da yeni bir dizin ekleyin.",
            ))
            .padding(12)
            .into(),
            Some(i) => index_editor(&w.indexes[i], &fields, &judged[i]),
        };
        let problem = layer_indexes_problem(&fields, &w.indexes)
            .map(|(_, why)| why)
            .or_else(|| {
                judged.iter().zip(&w.indexes).find_map(|(b, x)| {
                    b.as_ref().map(|(values, n)| {
                        format!(
                            "“{}” dizini tekil olamaz: {values} {n} kayıtta var. Kayıtları düzeltin ya da dizini tekil yapmayın.",
                            x.name
                        )
                    })
                })
            });
        let bound = self.document.as_ref().is_some_and(|d| {
            d.model
                .settings()
                .sources
                .iter()
                .any(|s| s.layer == w.layer)
        });
        let mut body = column![
            words::field("Katman", container(layer).width(320), None),
            row![container(left).width(260), container(right).width(Fill)].spacing(16),
        ]
        .spacing(14);
        if bound && !w.indexes.is_empty() {
            body = body.push(words::text_line(
                Kind::Info,
                "Katman bir tabloya bağlı: dizinleri KentOS denetler, tabloda kurulmaz.".to_owned(),
            ));
        }
        if let Some(why) = &problem {
            body = body.push(words::text_line(Kind::Warn, why.clone()));
        }
        body = body.push(
            row![
                words::ghost(REMOVE, at.map(|_| msg(Event::Remove))),
                space::horizontal().width(Fill),
                words::secondary(CANCEL, Some(msg(Event::Cancel))),
                words::primary(SAVE, problem.is_none().then(|| msg(Event::Save))),
            ]
            .spacing(8)
            .align_y(Center),
        );
        Frame::new(TITLE)
            .push(body)
            .width(900.0)
            .modal(msg(Event::Cancel))
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn layer_indexes_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.layer_indexes.is_none() {
            return Err(format!("{TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The chosen index: its name, whether it is the layer's key, its fields in order and whether empty values are the
/// same.
fn index_editor<'a>(
    x: &'a LayerIndex,
    fields: &[LayerField],
    broken: &Broken,
) -> Element<'a, Message> {
    let name =
        kentos_ui::widget::text_field("Dizinin adı", &x.name).on_input(|s| msg(Event::Name(s)));
    let mut chips = Row::new().spacing(4);
    for f in fields.iter().filter(|f| indexable(f)) {
        let on = x.fields.iter().any(|k| k == f.key());
        chips = chips.push(
            button(label::caption(f.label().to_owned()))
                .on_press(msg(Event::ToggleField(f.key().to_owned())))
                .padding([2, 8])
                .style(style::button::chip(on)),
        );
    }
    let chips = chips.wrap().vertical_spacing(4);
    let small = |glyph: &'static str, tip: &'static str, on: Option<Message>| {
        kentos_ui::widget::tip(
            button(icon(from_web(Some(glyph))).size(14.0))
                .on_press_maybe(on)
                .padding(kentos_ui::theme::metrics::icon_small(14.0))
                .style(style::button::ghost),
            kentos_ui::widget::Tip::new(tip),
            iced::widget::tooltip::Position::Top,
        )
    };
    let mut order = Column::new().spacing(2);
    for (i, k) in x.fields.iter().enumerate() {
        let shown = fields
            .iter()
            .find(|f| f.key() == k)
            .map_or(k.clone(), |f| f.label().to_owned());
        order = order.push(
            row![
                container(label::caption(format!("{}", i + 1)).style(muted)).width(20),
                container(label::body(shown)).width(Fill),
                small(
                    "chevronUp",
                    "Yukarı",
                    (i > 0).then(|| msg(Event::Move(i, true)))
                ),
                small(
                    "chevronDown",
                    "Aşağı",
                    (i + 1 < x.fields.len()).then(|| msg(Event::Move(i, false)))
                ),
                small(
                    "close",
                    "Dizinden çıkar",
                    Some(msg(Event::ToggleField(k.clone())))
                ),
            ]
            .spacing(4)
            .align_y(Center),
        );
    }
    if x.fields.is_empty() {
        order =
            order.push(label::caption("Yukarıdan dizinin alanlarını sırayla seçin.").style(muted));
    }
    let state = match (x.unique, broken) {
        (false, _) => words::text_line(
            Kind::Info,
            "Kayıtlar bu alanlarla aranır; GeoPackage'a yazılan tablo dizini alır.".to_owned(),
        ),
        (true, None) => {
            words::text_line(Kind::Ok, "Katmanın kayıtları anahtara uyuyor.".to_owned())
        }
        (true, Some((values, n))) => words::text_line(
            Kind::Error,
            format!("{values} {n} kayıtta var; önce kayıtları düzeltin."),
        ),
    };
    column![
        row![
            words::field("Ad", container(name).width(300), None),
            container(words::check(
                x.unique,
                "Tekil (katmanın anahtarı)",
                Some(msg(Event::Unique(!x.unique)))
            ))
            .padding(iced::Padding {
                top: 18.0,
                ..iced::Padding::ZERO
            }),
        ]
        .spacing(16),
        words::field(
            "Alanlar",
            chips,
            Some("Seçilen alanlar seçildiği sırayla dizine girer; sırayı oklarla değiştirin.".into()),
        ),
        container(order)
            .padding(8)
            .style(style::container::field_box)
            .width(Fill),
        words::check(
            x.nulls_equal,
            "Boş değerler aynı sayılır",
            x.unique.then(|| msg(Event::NullsEqual(!x.nulls_equal)))
        ),
        state,
        label::caption(if x.unique {
            "Anahtarın alanlarında başka bir kaydın değerlerini veren yazma reddedilir: form, tablo, komut, içe aktarma, Python ve ajan. Boş değer, “Boş değerler aynı sayılır” seçili değilse karşılaştırılmaz."
        } else {
            "Dizin kayıtları sınırlamaz; tekil yapılırsa katmanın anahtarı olur."
        })
        .style(muted),
    ]
    .spacing(10)
    .into()
}

fn muted(theme: &iced::Theme) -> iced::widget::text::Style {
    iced::widget::text::Style {
        color: Some(Tokens::of(theme).muted),
    }
}
