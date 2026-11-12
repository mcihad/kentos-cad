//! Katmanları çevir (docs/adr/0311 §5; AutoCAD's LAYTRANS): a window of
//! rules, each a name pattern (`A-*`, `duvar*,wall*`), the standard layer it
//! goes to and that layer's colour and weight, with the preview below: the
//! command's plan (`cad.layers.translate`), what goes where with its objects
//! and the layers to add or restyle, read again after every change. Çevir
//! writes it as one undo step “Katmanları çevir” and closes.

use iced::widget::{Column, button, container, row, text};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{CommandResult, LayersTranslate, LayersTranslated, TranslateRule};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, layers_translate};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame, Elided, VirtualList};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const TRANSLATE_TITLE: &str = "Katmanları çevir";
const ADD: &str = "Kural ekle";
const TRANSLATE: &str = "Çevir";
const CANCEL: &str = "Vazgeç";
const ROW: f32 = 26.0;

/// A rule as typed: the pattern, the standard layer, its colour and weight
/// (empty: as the standard layer has them, or its first source).
#[derive(Debug, Clone, Default)]
pub struct RuleRow {
    pub from: String,
    pub to: String,
    pub color: String,
    pub weight: String,
}

#[derive(Debug, Clone)]
pub struct Window {
    pub rules: Vec<RuleRow>,
    /// The preview, read again after every change of the rules (a plan on
    /// a large drawing is not made for every frame).
    preview: Result<LayersTranslated, String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    From(usize, String),
    To(usize, String),
    Color(usize, String),
    Weight(usize, String),
    Add,
    Remove(usize),
    Translate,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::LayerTranslate(event)
}

/// The rules as the command takes them; a row without a pattern or a name is
/// left out, a weight that does not read as a number kept as typed for the
/// refusal (none).
fn input_of(w: &Window) -> Result<LayersTranslate, String> {
    let mut rules = Vec::new();
    for (i, r) in w.rules.iter().enumerate() {
        if r.from.trim().is_empty() && r.to.trim().is_empty() {
            continue;
        }
        let weight = match r.weight.trim() {
            "" => None,
            t => match t.replace(',', ".").parse::<f64>() {
                Ok(v) => Some(v),
                Err(_) => {
                    return Err(format!(
                        "{}. kuralın kalınlığı (“{t}”) bir sayı değil; mm olarak yazın (0,35).",
                        i + 1
                    ));
                }
            },
        };
        rules.push(TranslateRule {
            from: r.from.trim().to_owned(),
            to: r.to.trim().to_owned(),
            color: (!r.color.trim().is_empty()).then(|| r.color.trim().to_owned()),
            line_type: None,
            line_weight: weight,
        });
    }
    Ok(LayersTranslate {
        rules,
        parent: None,
        expected_revision: None,
    })
}

impl App {
    /// Katmanları çevir (`layer.translate`): the window with one empty rule.
    pub(crate) fn open_layer_translate(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.layer_translate = Some(Window {
            rules: vec![RuleRow::default()],
            preview: Err(String::new()),
        });
        self.dialog = Some(Dialog::LayerTranslate);
        self.refresh_translate();
    }

    /// The window's preview read again from the command's plan.
    fn refresh_translate(&mut self) {
        let preview = self.translate_plan();
        if let Some(w) = self.layer_translate.as_mut() {
            w.preview = preview;
        }
    }

    /// The preview: the command's plan, or why it would not write.
    fn translate_plan(&self) -> Result<LayersTranslated, String> {
        let (Some(w), Some(doc)) = (&self.layer_translate, &self.document) else {
            return Err("Açık çizim yok.".to_owned());
        };
        if let Some(locked) = self.tree_locked() {
            return Err(locked.to_owned());
        }
        let input = input_of(w)?;
        if input.rules.is_empty() {
            return Err(
                "Bir kural yazın: Kaynak bir ad örüntüsüdür (A-*, duvar*,wall*), Standart katman çevrilecek katmanın adı."
                    .to_owned(),
            );
        }
        let mut copy = doc.model.reading_copy();
        match layers_translate::plan(&ExecutionContext::new(&mut copy), &input) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err("Çevirme planlanamadı.".to_owned()),
        }
    }

    pub(crate) fn layer_translate_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.layer_translate.as_mut() else {
            return Task::none();
        };
        fn row(w: &mut Window, i: usize) -> Option<&mut RuleRow> {
            w.rules.get_mut(i)
        }
        match event {
            Event::From(i, t) => {
                if let Some(r) = row(w, i) {
                    r.from = t;
                }
            }
            Event::To(i, t) => {
                if let Some(r) = row(w, i) {
                    r.to = t;
                }
            }
            Event::Color(i, t) => {
                if let Some(r) = row(w, i) {
                    r.color = t;
                }
            }
            Event::Weight(i, t) => {
                if let Some(r) = row(w, i) {
                    r.weight = t;
                }
            }
            Event::Add => {
                w.rules.push(RuleRow::default());
                return Task::none();
            }
            Event::Remove(i) => {
                if i < w.rules.len() {
                    w.rules.remove(i);
                }
                if w.rules.is_empty() {
                    w.rules.push(RuleRow::default());
                }
            }
            Event::Translate => {
                if self.translate_layers() {
                    self.layer_translate = None;
                    self.dialog = None;
                }
                return Task::none();
            }
            Event::Cancel => {
                self.layer_translate = None;
                self.dialog = None;
            }
        }
        self.refresh_translate();
        Task::none()
    }

    /// Writes the window's rules; whether it wrote (what it did said).
    pub(crate) fn translate_layers(&mut self) -> bool {
        let input = match self.layer_translate.as_ref().map(input_of) {
            Some(Ok(input)) => input,
            Some(Err(why)) => {
                self.warn(why);
                return false;
            }
            None => return false,
        };
        if let Some(locked) = self.tree_locked() {
            self.warn(locked);
            return false;
        }
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let result = layers_translate::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(done) = self.answered(result) else {
            return false;
        };
        if !done.changed {
            self.say(
                Level::Info,
                "Kurallara uyan katman yok; değişen olmadı.".to_owned(),
            );
            return true;
        }
        self.say(
            Level::Success,
            format!(
                "{} katman çevrildi: {} nesne taşındı{}.",
                done.translations.len(),
                done.objects,
                match done.created.len() {
                    0 => String::new(),
                    n => format!(", {n} standart katman eklendi"),
                }
            ),
        );
        true
    }

    pub(crate) fn layer_translate_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.layer_translate else {
            return text("").into();
        };
        let field =
            |placeholder: &'static str, value: &str, on: fn(usize, String) -> Event, i: usize| {
                kentos_ui::widget::text_field(placeholder, value).on_input(move |t| msg(on(i, t)))
            };
        let mut rules = Column::new().spacing(4);
        rules = rules.push(
            row![
                container(label::caption("Kaynak (ad örüntüsü)")).width(Fill),
                container(label::caption("Standart katman")).width(Fill),
                container(label::caption("Renk")).width(typography::scaled(84.0)),
                container(label::caption("Kalınlık")).width(typography::scaled(64.0)),
                container(text("")).width(typography::scaled(24.0)),
            ]
            .spacing(6),
        );
        for (i, r) in w.rules.iter().enumerate() {
            rules = rules.push(
                row![
                    container(field("A-*, duvar*", &r.from, Event::From, i)).width(Fill),
                    container(field("A-DUVAR", &r.to, Event::To, i)).width(Fill),
                    container(field("#E5484D", &r.color, Event::Color, i))
                        .width(typography::scaled(84.0)),
                    container(field("0,35", &r.weight, Event::Weight, i))
                        .width(typography::scaled(64.0)),
                    button(icon(Icon::Close).size(12.0).tone(Tone::Muted))
                        .on_press(msg(Event::Remove(i)))
                        .padding(kentos_ui::theme::metrics::icon_small(12.0))
                        .style(style::button::ghost),
                ]
                .spacing(6)
                .align_y(Center),
            );
        }
        rules = rules.push(words::secondary(ADD, Some(msg(Event::Add))));
        let (lines, rows, ready) = match &w.preview {
            Err(why) => (
                if why.is_empty() {
                    Vec::new()
                } else {
                    vec![words::text_line(Kind::Info, why.clone())]
                },
                Vec::new(),
                false,
            ),
            Ok(p) if p.translations.is_empty() => (
                vec![words::text_line(
                    Kind::Info,
                    "Kurallara uyan katman yok. Kaynak örüntüsünü değiştirin: * herhangi bir dizi, ? tek karakter."
                        .to_owned(),
                )],
                Vec::new(),
                false,
            ),
            Ok(p) => {
                let mut lines = vec![words::text_line(
                    Kind::Info,
                    format!(
                        "{} katman {} standart katmana çevrilecek: {} nesne taşınır, kaynak katmanlar silinir. Tek adımda geri alınır.",
                        p.translations.len(),
                        {
                            let mut to: Vec<&str> =
                                p.translations.iter().map(|t| t.to.as_str()).collect();
                            to.sort_unstable();
                            to.dedup();
                            to.len()
                        },
                        p.objects
                    ),
                )];
                if !p.created.is_empty() {
                    let names: Vec<String> = p
                        .translations
                        .iter()
                        .filter(|t| p.created.contains(&t.to))
                        .map(|t| t.to_name.clone())
                        .fold(Vec::new(), |mut v, n| {
                            if !v.contains(&n) {
                                v.push(n);
                            }
                            v
                        });
                    lines.push(words::text_line(
                        Kind::Info,
                        format!("Eklenecek katman: {}.", names.join(", ")),
                    ));
                }
                let rows: Vec<(String, String, String, bool)> = p
                    .translations
                    .iter()
                    .map(|t| {
                        (
                            t.name.clone(),
                            t.to_name.clone(),
                            t.objects.to_string(),
                            p.created.contains(&t.to),
                        )
                    })
                    .collect();
                (lines, rows, true)
            }
        };
        let head = kentos_ui::widget::table::band(
            row![
                container(kentos_ui::widget::table::title("Kaynak katman")).width(Fill),
                container(kentos_ui::widget::table::title("Standart katman")).width(Fill),
                container(kentos_ui::widget::table::title("Nesne")).width(typography::scaled(56.0)),
            ]
            .spacing(8)
            .padding([0, 8]),
        );
        let list = VirtualList::new(rows.len(), typography::scaled(ROW), move |i| {
            let (from, to, n, new) = &rows[i];
            let to = if *new {
                format!("{to} (yeni)")
            } else {
                to.clone()
            };
            row![
                Elided::new(from.clone())
                    .size(typography::caption())
                    .font(typography::ui())
                    .width(Fill),
                Elided::new(to)
                    .size(typography::caption())
                    .font(typography::ui())
                    .style(|theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(Tokens::of(theme).text),
                    })
                    .width(Fill),
                container(label::caption(n.clone())).width(typography::scaled(56.0)),
            ]
            .spacing(8)
            .padding([0, 8])
            .height(typography::scaled(ROW))
            .align_y(Center)
            .into()
        })
        .height(typography::scaled(180.0));
        let table = container(Column::new().push(head).push(list))
            .style(style::container::field_box)
            .width(Fill);
        let body = Column::new()
            .spacing(12)
            .push(words::field("Kurallar", rules, Some(
                "Her katman adına uyan ilk kuralı alır; adı standart katmanınki olan katman kendisidir.".to_owned(),
            )))
            .push(words::summary(lines))
            .push(words::field("Önizleme", table, None));
        Frame::new(TRANSLATE_TITLE)
            .push(body)
            .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
            .action(words::primary(
                TRANSLATE,
                ready.then(|| msg(Event::Translate)),
            ))
            .width(640.0)
            .modal(msg(Event::Cancel))
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn layer_translate_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.layer_translate.is_none() {
            return Err(format!("{TRANSLATE_TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Fill("Kaynak", t) => Some(msg(Event::From(0, t.to_owned()))),
            Control::Fill("Standart katman", t) => Some(msg(Event::To(0, t.to_owned()))),
            Control::Fill("Renk", t) => Some(msg(Event::Color(0, t.to_owned()))),
            Control::Fill("Kalınlık", t) => Some(msg(Event::Weight(0, t.to_owned()))),
            Control::Press(ADD) => Some(msg(Event::Add)),
            Control::Press(TRANSLATE) => Some(msg(Event::Translate)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            other => return Err(format!("“{TRANSLATE_TITLE}” penceresinde {other} yok")),
        })
    }
}
