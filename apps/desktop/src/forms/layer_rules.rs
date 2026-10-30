//! Kayıt kuralları (docs/adr/0438 §1, `data.layerRules`): a layer's record rules.
//!
//! ```text
//! ┌ Kayıt kuralları ───────────────────────────────────────────────────────────┐
//! │ Katman [Ruhsatlı yapı       ▾]                                             │
//! │ Kurallar            │ Ad [Tarih sırası            ]  ☐ Uyarı                │
//! │ Tarih sırası        │ İfade [Bitiş boş veya Bitiş >= Başlangıç          ]  │
//! │ İnceleme notu · ⚠   │ ✓ Derleniyor · var olan 2 kayıt bozuyor             │
//! │ [Yeni kural]        │ İleti [Bitiş tarihi başlangıçtan önce olamaz.    ]  │
//! │                     │ Gösterildiği alan [Bitiş ▾]                         │
//! │ [Kuralı sil]                                         [Vazgeç] [Kaydet]     │
//! └────────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! The window edits a copy; Kaydet writes it through `cad.layers.rules` in one step and closes. Each
//! rule says whether it compiles and how many of the layer's records break it now (they are not
//! judged anew by the rule; their next write is).

use iced::widget::{Column, button, column, container, row, scrollable, space};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{LayerField, LayerNodeType, LayerRule, LayersRules, layer_rules_problem};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, layers_rules};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::Dialog as Frame;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TITLE: &str = "Kayıt kuralları";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NEW: &str = "Yeni kural";
const REMOVE: &str = "Kuralı sil";

/// The window.
#[derive(Debug, Clone, Default)]
pub struct Window {
    pub layer: String,
    pub rules: Vec<LayerRule>,
    kept: Vec<LayerRule>,
    pub chosen: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Layer(String),
    Choose(usize),
    New,
    Remove,
    Name(String),
    Rule(String),
    Message(String),
    Warning(bool),
    /// The field it is shown on, by key; none: the first field it reads.
    ShownOn(Option<String>),
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::LayerRules(event)
}

/// What the layer's rules are now: per rule, why it does not compile, else how many records break it.
struct Judged {
    problems: Vec<Option<String>>,
    broken: Vec<usize>,
}

impl App {
    /// Kayıt kuralları: the window on the active layer's rules (the table's layer when it shows one).
    pub(crate) fn open_layer_rules(&mut self) -> Task<Message> {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let layers = self.record_rule_layers();
        let wanted = self
            .features
            .layer
            .clone()
            .or_else(|| Some(doc.model.layers().active().to_owned()));
        let Some(layer) = wanted
            .filter(|l| layers.iter().any(|(id, _)| id == l))
            .or_else(|| layers.first().map(|(id, _)| id.clone()))
        else {
            self.warn("Alanı olan katman yok; kayıt kuralları bir katmanın alanlarını okur. Önce Alanlar penceresinden alan ekleyin.");
            return Task::none();
        };
        self.layer_rules = Some(self.layer_rules_window_of(layer));
        self.dialog = Some(Dialog::LayerRules);
        Task::none()
    }

    fn layer_rules_window_of(&self, layer: String) -> Window {
        let rules = self
            .document
            .as_ref()
            .and_then(|d| d.model.layers().get(&layer))
            .map(|n| n.rules.clone())
            .unwrap_or_default();
        Window {
            layer,
            chosen: (!rules.is_empty()).then_some(0),
            kept: rules.clone(),
            rules,
        }
    }

    pub(crate) fn layer_rules_event(&mut self, event: Event) -> Task<Message> {
        if let Event::Layer(id) = &event {
            self.layer_rules = Some(self.layer_rules_window_of(id.clone()));
            return Task::none();
        }
        let Some(w) = self.layer_rules.as_mut() else {
            return Task::none();
        };
        let at = w.chosen.filter(|i| *i < w.rules.len());
        let chosen = at.and_then(|i| w.rules.get_mut(i));
        match event {
            Event::Layer(_) => {}
            Event::Choose(i) => w.chosen = Some(i),
            Event::New => {
                let mut n = w.rules.len() + 1;
                while w.rules.iter().any(|r| r.id == format!("kural-{n}")) {
                    n += 1;
                }
                w.rules.push(LayerRule {
                    id: format!("kural-{n}"),
                    name: format!("Kural {n}"),
                    rule: String::new(),
                    message: String::new(),
                    warning: false,
                    fields: Vec::new(),
                });
                w.chosen = Some(w.rules.len() - 1);
            }
            Event::Remove => {
                if let Some(i) = at {
                    w.rules.remove(i);
                    w.chosen = (!w.rules.is_empty()).then_some(i.min(w.rules.len() - 1));
                }
            }
            Event::Name(text) => {
                if let Some(r) = chosen {
                    r.name = text;
                }
            }
            Event::Rule(text) => {
                if let Some(r) = chosen {
                    r.rule = text;
                }
            }
            Event::Message(text) => {
                if let Some(r) = chosen {
                    r.message = text;
                }
            }
            Event::Warning(on) => {
                if let Some(r) = chosen {
                    r.warning = on;
                }
            }
            Event::ShownOn(key) => {
                if let Some(r) = chosen {
                    r.fields = key.into_iter().collect();
                }
            }
            Event::Cancel => {
                self.layer_rules = None;
                self.dialog = None;
            }
            Event::Save => return self.save_layer_rules(),
        }
        Task::none()
    }

    /// Kaydet: the rules through `cad.layers.rules` in one step; the window closes when they go in.
    fn save_layer_rules(&mut self) -> Task<Message> {
        let Some(w) = self.layer_rules.clone() else {
            return Task::none();
        };
        if w.rules == w.kept {
            self.layer_rules = None;
            self.dialog = None;
            return Task::none();
        }
        let Some(doc) = self.document.as_mut() else {
            return Task::none();
        };
        let result = layers_rules::execute(
            &mut ExecutionContext::new(&mut doc.model),
            LayersRules {
                layer: w.layer.clone(),
                rules: w.rules.clone(),
                expected_revision: None,
            },
        );
        if self.answered(result).is_none() {
            return Task::none();
        }
        self.layer_rules = None;
        self.dialog = None;
        let name = self.form_layer_name(&w.layer);
        let said = if w.rules.is_empty() {
            format!("“{name}” katmanının kayıt kuralları kaldırıldı.")
        } else {
            format!(
                "“{name}” katmanının {} kayıt kuralı kaydedildi; her yazma onları sorar.",
                w.rules.len()
            )
        };
        self.say(Level::Success, said);
        Task::none()
    }

    /// The layers holding objects with fields, by their path.
    fn record_rule_layers(&self) -> Vec<(String, String)> {
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

    /// Each rule compiled over the layer's fields and asked of its records now.
    fn judge_rules(&self, w: &Window, fields: &[LayerField]) -> Judged {
        let Some(doc) = &self.document else {
            return Judged {
                problems: vec![None; w.rules.len()],
                broken: vec![0; w.rules.len()],
            };
        };
        let variables = kentos_project::variables::expression_variables(
            doc.model.name(),
            doc.model.settings(),
            0.0,
            "",
        );
        let unit = kentos_native_application::expression_unit::reading_unit(doc.model.settings());
        let layer_name = self.form_layer_name(&w.layer);
        let records: Vec<(kentos_forms::Attrs, kentos_forms::Object)> = doc
            .model
            .by_layer(&w.layer)
            .map(|e| {
                let object = kentos_forms::object_of(e, &layer_name, f64::from(e.base().id));
                (e.base().attrs.clone(), object)
            })
            .collect();
        let mut problems = Vec::with_capacity(w.rules.len());
        let mut broken = Vec::with_capacity(w.rules.len());
        for r in &w.rules {
            if r.rule.trim().is_empty() {
                problems.push(Some("İfade yazın.".to_owned()));
                broken.push(0);
                continue;
            }
            match kentos_forms::LayerRules::new(std::slice::from_ref(r), fields, &variables, unit) {
                Err((_, why)) => {
                    problems.push(Some(why));
                    broken.push(0);
                }
                Ok(one) => {
                    problems.push(None);
                    broken.push(
                        records
                            .iter()
                            .filter(|(a, o)| !one.broken(fields, None, a, o).is_empty())
                            .count(),
                    );
                }
            }
        }
        Judged { problems, broken }
    }

    pub(crate) fn layer_rules_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.layer_rules else {
            return iced::widget::text("").into();
        };
        let layers = self.record_rule_layers();
        let ids: Vec<String> = layers.iter().map(|(id, _)| id.clone()).collect();
        let layer = Select::new(
            layers.iter().map(|(_, n)| Choice::new(n.clone())),
            ids.iter().position(|id| *id == w.layer),
            move |i| msg(Event::Layer(ids[i].clone())),
        );
        let fields = self.layer_fields_of(&w.layer);
        let judged = self.judge_rules(w, &fields);
        let mut list = Column::new().spacing(1).padding(2);
        for (i, r) in w.rules.iter().enumerate() {
            let state = match (&judged.problems[i], judged.broken[i]) {
                (Some(_), _) => "derlenmiyor".to_owned(),
                (None, 0) => "kayıtlar uyuyor".to_owned(),
                (None, n) => format!("{n} kayıt bozuyor"),
            };
            let face = column![
                label::body(r.name.clone()),
                label::caption(format!(
                    "{} · {state}",
                    if r.warning { "uyarır" } else { "reddeder" }
                ))
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
        if w.rules.is_empty() {
            list = list.push(
                label::caption(
                    "Kural yok. Yeni kural ile katmanın her yazmada tutması gereken bir ifade ekleyin.",
                )
                .style(muted),
            );
        }
        let at = w.chosen.filter(|i| *i < w.rules.len());
        let left = column![
            label::strong("Kurallar"),
            container(scrollable(list).height(typography::scaled(300.0)))
                .style(style::container::field_box)
                .width(Fill),
            words::secondary(NEW, Some(msg(Event::New))),
        ]
        .spacing(8);
        let right: Element<'_, Message> = match at {
            None => container(label::caption(
                "Bir kural seçin ya da yeni bir kural ekleyin.",
            ))
            .padding(12)
            .into(),
            Some(i) => rule_editor(&w.rules[i], &fields, &judged.problems[i], judged.broken[i]),
        };
        let problem = layer_rules_problem(&fields, &w.rules)
            .map(|(_, why)| why)
            .or_else(|| judged.problems.iter().flatten().next().cloned());
        let mut body = column![
            words::field("Katman", container(layer).width(320), None),
            row![container(left).width(250), container(right).width(Fill)].spacing(16),
        ]
        .spacing(14);
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
    pub(crate) fn layer_rules_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.layer_rules.is_none() {
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

/// The chosen rule: its name, its expression and whether it compiles, its message, whether it only
/// warns and the field it is shown on.
fn rule_editor<'a>(
    r: &'a LayerRule,
    fields: &[LayerField],
    problem: &Option<String>,
    broken: usize,
) -> Element<'a, Message> {
    let input = |hint: &'static str, value: &str, on: fn(String) -> Event| {
        kentos_ui::widget::text_field(hint, value).on_input(move |s| msg(on(s)))
    };
    let state = match problem {
        Some(why) => words::text_line(Kind::Error, why.clone()),
        None if broken == 0 => words::text_line(
            Kind::Ok,
            "Derleniyor; katmanın kayıtları kurala uyuyor.".to_owned(),
        ),
        None => words::text_line(
            Kind::Warn,
            format!(
                "Derleniyor; var olan {broken} kayıt kuralı bozuyor (sonraki yazmalarında sorulur)."
            ),
        ),
    };
    let keys: Vec<String> = fields.iter().map(|f| f.key().to_owned()).collect();
    let mut choices = vec![Choice::new("İfadenin okuduğu ilk alan")];
    choices.extend(fields.iter().map(|f| Choice::new(f.label().to_owned())));
    let now = match r.fields.first() {
        None => Some(0),
        Some(k) => keys.iter().position(|x| x == k).map(|i| i + 1),
    };
    let shown = Select::new(choices, now, move |i| {
        msg(Event::ShownOn(i.checked_sub(1).map(|i| keys[i].clone())))
    });
    column![
        row![
            words::field("Ad", container(input("Kuralın adı", &r.name, Event::Name)).width(300), None),
            container(words::check(r.warning, "Yalnız uyarır", Some(msg(Event::Warning(!r.warning)))))
                .padding(iced::Padding {
                    top: 18.0,
                    ..iced::Padding::ZERO
                }),
        ]
        .spacing(16),
        words::field(
            "İfade",
            input("Bitiş boş veya Bitiş >= Başlangıç", &r.rule, Event::Rule),
            Some("Kaydın tutması gereken koşul; alanlar adlarıyla, $alan ve @ değerleri de. Boş alanları “boş” ile karşılayın.".into()),
        ),
        state,
        words::field("İleti", input("Kural bozulunca söylenen", &r.message, Event::Message), None),
        words::field(
            "Gösterildiği alan",
            container(shown).width(300),
            Some("Formda kural bu alanın altında söylenir; red de buraya işaret eder.".into()),
        ),
        label::caption(if r.warning {
            "Uyarı kuralı: bozan yazma yine yazılır, uyarılır."
        } else {
            "Bozan yazma reddedilir: form, tablo, komut, içe aktarma, Python ve ajan."
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
