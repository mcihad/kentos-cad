//! Yazı stilleri and Ölçü stilleri (docs/adr/0183 §5; the web's
//! `ui/annotation/StylesDialog.ts`): the project's styles of a kind,
//! Standart first and not edited, each with how many objects follow it. A
//! style chosen shows its form and a preview drawn as the drawing draws it;
//! Yeni copies the chosen one, Sil takes the chosen one away (asking when
//! objects follow it: they keep their look, without the link). The form is a
//! draft: Kaydet writes the table to the project and the objects that follow
//! the styles in one undo step (`kentos_interaction::style_tables`); Vazgeç,
//! Esc and × leave it. A CAD project's window: the commands are hidden in a
//! CBS project's interface.

use std::collections::HashMap;

use iced::widget::canvas::{self, Frame, Path, Stroke};
use iced::widget::{Column, button, container, row, text_input};
use iced::{Center, Color, Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_contracts::{
    DimensionArrow, DimensionStyleDef, DimensionTextPlace, DrawingFont, DrawingUnit,
    STANDARD_STYLE, TextStyleDef, dimension_styles_problem, text_styles_problem,
};
use kentos_interaction::{Format, Level, style_tables};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Choice, Confirm, Dialog as Frame_, Elided, Select, focus_ring, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind as Line};
use crate::traces::Control;

/// The windows' titles, which a trace names them by.
pub const TEXT_TITLE: &str = "Yazı stilleri";
pub const DIMENSION_TITLE: &str = "Ölçü stilleri";
const NEW: &str = "Yeni";
const REMOVE: &str = "Sil";
const SAVE: &str = "Kaydet";
const CANCEL: &str = "Vazgeç";
const NAME: &str = "Ad";
const FONT: &str = "Yazı tipi";
const BOLD: &str = "Kalın";
const ITALIC: &str = "İtalik";
const ARROWS: &str = "Uçlar";
const PLACE: &str = "Değerin yeri";
const UNIT: &str = "Birim";
const PREFIX: &str = "Önek";
const SUFFIX: &str = "Sonek";

/// The number fields: their key (the style's field), label and hint, as the web's.
const TEXT_NUMBERS: [(&str, &str, &str); 3] = [
    ("oblique", "Eğiklik (°)", "−85 ile 85; boş: dik"),
    ("height", "Yükseklik (mm)", "Kâğıtta; boş: aracın"),
    ("widthFactor", "Genişlik çarpanı", "Boş: 1"),
];
const DIMENSION_NUMBERS: [(&str, &str, &str); 6] = [
    ("height", "Değer yüksekliği (mm)", "Kâğıtta"),
    ("arrowSize", "Uç boyu (mm)", "Boş: yüksekliğe göre"),
    ("extOffset", "Uzatma boşluğu (mm)", "Noktadan"),
    ("extBeyond", "Uzatma aşması (mm)", "Çizgiyi"),
    ("textGap", "Değerin yüksekliği (mm)", "Çizgiden"),
    ("decimals", "Basamak", "Boş: projenin"),
];

/// The arrowheads in the menu's order, the tick (none) first.
const ARROW_CHOICES: [(Option<DimensionArrow>, &str); 5] = [
    (None, "Çentik"),
    (Some(DimensionArrow::Closed), "Dolu ok"),
    (Some(DimensionArrow::Open), "Açık ok"),
    (Some(DimensionArrow::Dot), "Nokta"),
    (Some(DimensionArrow::None), "Yok"),
];
/// A dimension's line (docs/adr/0205 §6): the dimension line and its
/// arrowheads, or the extension lines; the value has a colour alone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Part {
    Line,
    Ext,
    Value,
}

const UNIT_CHOICES: [(Option<DrawingUnit>, &str); 4] = [
    (None, "Projenin"),
    (Some(DrawingUnit::M), "m"),
    (Some(DrawingUnit::Cm), "cm"),
    (Some(DrawingUnit::Mm), "mm"),
];

/// Which styles the window holds.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Text,
    Dimension,
}

/// The window: the draft table, the chosen style, the number fields as typed.
#[derive(Debug, Clone)]
pub struct Window {
    kind: Kind,
    text: Vec<TextStyleDef>,
    dimension: Vec<DimensionStyleDef>,
    chosen: Option<String>,
    /// What each number field holds as typed, by its key; refilled when another style is chosen.
    typed: HashMap<&'static str, String>,
    /// How many objects follow each style, as when the window opened.
    usage: HashMap<String, usize>,
    /// Sil asked about the style with this id (objects follow it).
    asking: Option<String>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(Option<String>),
    New,
    Remove,
    RemoveConfirmed,
    RemoveDropped,
    Name(String),
    Font(Option<DrawingFont>),
    Bold(bool),
    Italic(bool),
    Number(&'static str, String),
    Arrow(Option<DimensionArrow>),
    Centre(bool),
    Unit(Option<DrawingUnit>),
    Prefix(String),
    Suffix(String),
    /// A line's colour (`#RRGGBB`; none: the object's), weight on paper (mm;
    /// none: a hairline) and type (none: continuous), docs/adr/0205 §6.
    LineColor(Part, Option<String>),
    LineWeight(Part, Option<f64>),
    LineType(Part, Option<kentos_contracts::LineType>),
    Save,
    Cancel,
}

fn msg(event: Event) -> Message {
    Message::AnnotationStyles(event)
}

/// A number field's text: empty for none, else as JavaScript writes it.
fn shown(n: Option<f64>) -> String {
    n.map(|v| format!("{v}")).unwrap_or_default()
}

/// A number field's value: none when empty, `Err` when it is not a number.
fn number(s: &str) -> Result<Option<f64>, ()> {
    let t = s.trim().replace(',', ".");
    if t.is_empty() {
        return Ok(None);
    }
    t.parse::<f64>()
        .ok()
        .filter(|n| n.is_finite())
        .map(Some)
        .ok_or(())
}

/// A check box as tall as the select beside it, centred in it.
fn boxed<'a>(check: Element<'a, Message>) -> Element<'a, Message> {
    container(check)
        .height(typography::scaled(28.0))
        .align_y(iced::Center)
        .into()
}

/// A name not taken yet: `base`, else `base 2`, `base 3` …
fn free_name<'a>(base: &str, taken: impl Iterator<Item = &'a str> + Clone) -> String {
    let free = |n: &str| !taken.clone().any(|t| t.to_lowercase() == n.to_lowercase());
    if free(base) {
        return base.to_owned();
    }
    (2..)
        .map(|i| format!("{base} {i}"))
        .find(|n| free(n))
        .unwrap_or_default()
}

impl Window {
    /// Whether Sil's question is open.
    pub(crate) fn asking(&self) -> bool {
        self.asking.is_some()
    }

    fn names(&self) -> Vec<(String, String)> {
        match self.kind {
            Kind::Text => self
                .text
                .iter()
                .map(|s| (s.id.clone(), s.name.clone()))
                .collect(),
            Kind::Dimension => self
                .dimension
                .iter()
                .map(|s| (s.id.clone(), s.name.clone()))
                .collect(),
        }
    }

    fn problem(&self) -> Option<String> {
        match self.kind {
            Kind::Text => text_styles_problem(&self.text),
            Kind::Dimension => dimension_styles_problem(&self.dimension),
        }
    }

    fn text_style(&mut self) -> Option<&mut TextStyleDef> {
        let id = self.chosen.clone()?;
        self.text.iter_mut().find(|s| s.id == id)
    }

    fn dimension_style(&mut self) -> Option<&mut DimensionStyleDef> {
        let id = self.chosen.clone()?;
        self.dimension.iter_mut().find(|s| s.id == id)
    }

    /// The number fields refilled from the chosen style.
    fn fill(&mut self) {
        self.typed.clear();
        let id = self.chosen.clone();
        match self.kind {
            Kind::Text => {
                if let Some(s) = self.text.iter().find(|s| Some(&s.id) == id.as_ref()) {
                    self.typed.insert("oblique", shown(s.oblique));
                    self.typed.insert("height", shown(s.height));
                    self.typed.insert("widthFactor", shown(s.width_factor));
                }
            }
            Kind::Dimension => {
                if let Some(s) = self.dimension.iter().find(|s| Some(&s.id) == id.as_ref()) {
                    self.typed.insert("height", shown(Some(s.height)));
                    self.typed.insert("arrowSize", shown(s.arrow_size));
                    self.typed.insert("extOffset", shown(s.ext_offset));
                    self.typed.insert("extBeyond", shown(s.ext_beyond));
                    self.typed.insert("textGap", shown(s.text_gap));
                    self.typed
                        .insert("decimals", shown(s.decimals.map(f64::from)));
                }
            }
        }
    }

    /// A number typed into `key`: kept as typed, the style's value when it is one.
    fn number(&mut self, key: &'static str, typed: String) {
        let value = number(&typed);
        self.typed.insert(key, typed);
        let Ok(v) = value else { return };
        match self.kind {
            Kind::Text => {
                if let Some(s) = self.text_style() {
                    match key {
                        "oblique" => s.oblique = v,
                        "height" => s.height = v,
                        _ => s.width_factor = v,
                    }
                }
            }
            Kind::Dimension => {
                if let Some(s) = self.dimension_style() {
                    match key {
                        // The value's height is never none.
                        "height" => {
                            if let Some(h) = v {
                                s.height = h;
                            }
                        }
                        "arrowSize" => s.arrow_size = v,
                        "extOffset" => s.ext_offset = v,
                        "extBeyond" => s.ext_beyond = v,
                        "textGap" => s.text_gap = v,
                        _ => {
                            s.decimals = v
                                .filter(|d| *d >= 0.0 && d.fract() == 0.0 && *d <= 255.0)
                                .map(|d| d as u32)
                        }
                    }
                }
            }
        }
    }
}

impl App {
    /// The open drawing's styles of a kind, as saved.
    fn saved_styles(&self, kind: Kind) -> (Vec<TextStyleDef>, Vec<DimensionStyleDef>) {
        self.document.as_ref().map_or_else(Default::default, |d| {
            let s = d.model.settings();
            match kind {
                Kind::Text => (s.text_styles.clone(), Vec::new()),
                Kind::Dimension => (Vec::new(), s.dimension_styles.clone()),
            }
        })
    }

    /// The open window's title (a trace names it by it).
    pub(crate) fn annotation_styles_title(&self) -> &'static str {
        match self.annotation_styles.as_ref().map(|w| w.kind) {
            Some(Kind::Dimension) => DIMENSION_TITLE,
            _ => TEXT_TITLE,
        }
    }

    /// Yazı stilleri (`style.textStyles`) or Ölçü stilleri (`style.dimensionStyles`): the window.
    pub(crate) fn open_annotation_styles(&mut self, kind: Kind) {
        let Some(doc) = self.document.as_ref() else {
            self.output("Açık çizim yok.");
            return;
        };
        let usage = style_tables::usage(&doc.model, kind == Kind::Text);
        let (text, dimension) = self.saved_styles(kind);
        let chosen = match kind {
            Kind::Text => text.first().map(|s| s.id.clone()),
            Kind::Dimension => dimension.first().map(|s| s.id.clone()),
        };
        let mut w = Window {
            kind,
            text,
            dimension,
            chosen,
            typed: HashMap::new(),
            usage,
            asking: None,
        };
        w.fill();
        self.annotation_styles = Some(w);
        self.dialog = Some(Dialog::AnnotationStyles);
    }

    fn styles_changed(&self) -> bool {
        self.annotation_styles.as_ref().is_some_and(|w| {
            let (text, dimension) = self.saved_styles(w.kind);
            w.text != text || w.dimension != dimension
        })
    }

    /// Kaydet: the table, then the objects that follow it, in one undo step.
    fn save_annotation_styles(&mut self) {
        let Some(w) = self.annotation_styles.clone() else {
            return;
        };
        if w.problem().is_some() {
            return;
        }
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let saved = match w.kind {
            Kind::Text => style_tables::save_text_styles(&mut doc.model, w.text.clone()),
            Kind::Dimension => {
                style_tables::save_dimension_styles(&mut doc.model, w.dimension.clone())
            }
        };
        let (title, what) = match w.kind {
            Kind::Text => ("Yazı", "yazı"),
            Kind::Dimension => ("Ölçü", "ölçü"),
        };
        for line in saved.said {
            self.warn(line);
        }
        let followed = if saved.changed > 0 {
            format!("; {} {what} stiline uydu", saved.changed)
        } else {
            String::new()
        };
        self.say(
            Level::Success,
            format!("{title} stilleri kaydedildi{followed}."),
        );
        if saved.locked > 0 {
            self.warn(format!(
                "{} {what} kilitli katmanda; eski görünüşüyle kaldı. Kilidi açıp stili yeniden kaydedin.",
                saved.locked
            ));
        }
        self.annotation_styles = None;
        self.dialog = None;
    }

    pub(crate) fn annotation_styles_event(&mut self, event: Event) -> Task<Message> {
        let font = self
            .document
            .as_ref()
            .and_then(|d| d.model.settings().drawing_font)
            .unwrap_or(DrawingFont::Barlow);
        match event {
            Event::Save => {
                self.save_annotation_styles();
                return Task::none();
            }
            Event::Cancel => {
                self.annotation_styles = None;
                self.dialog = None;
                return Task::none();
            }
            _ => {}
        }
        let Some(w) = self.annotation_styles.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Choose(id) => {
                w.chosen = id;
                w.fill();
            }
            Event::New => {
                let id = kentos_domain::Uuid::now_v7().to_string();
                match w.kind {
                    Kind::Text => {
                        let from = w
                            .chosen
                            .as_ref()
                            .and_then(|c| w.text.iter().find(|s| &s.id == c))
                            .cloned();
                        let base = from
                            .as_ref()
                            .map_or("Yazı stili".to_owned(), |s| format!("{} kopyası", s.name));
                        let name = free_name(&base, w.text.iter().map(|s| s.name.as_str()));
                        let mut s = from.unwrap_or(TextStyleDef {
                            id: String::new(),
                            name: String::new(),
                            font,
                            bold: false,
                            italic: false,
                            oblique: None,
                            height: None,
                            width_factor: None,
                            font_file: None,
                        });
                        s.id = id.clone();
                        s.name = name;
                        s.font_file = None;
                        w.text.push(s);
                    }
                    Kind::Dimension => {
                        let from = w
                            .chosen
                            .as_ref()
                            .and_then(|c| w.dimension.iter().find(|s| &s.id == c))
                            .cloned();
                        let base = from.as_ref().map_or("Ölçü stili".to_owned(), |s| {
                            format!("{} kopyası", s.name)
                        });
                        let name = free_name(&base, w.dimension.iter().map(|s| s.name.as_str()));
                        let mut s = from.unwrap_or(DimensionStyleDef {
                            id: String::new(),
                            name: String::new(),
                            height: 2.5,
                            arrow: None,
                            arrow_size: None,
                            ext_offset: None,
                            ext_beyond: None,
                            text_gap: None,
                            text_place: None,
                            decimals: None,
                            unit: None,
                            prefix: None,
                            suffix: None,
                            font: None,
                            ..Default::default()
                        });
                        s.id = id.clone();
                        s.name = name;
                        w.dimension.push(s);
                    }
                }
                w.chosen = Some(id);
                w.fill();
            }
            Event::Remove => {
                if let Some(id) = w.chosen.clone() {
                    if w.usage.get(&id).copied().unwrap_or(0) > 0 {
                        w.asking = Some(id);
                    } else {
                        w.text.retain(|s| s.id != id);
                        w.dimension.retain(|s| s.id != id);
                        w.chosen = None;
                        w.fill();
                    }
                }
            }
            Event::RemoveConfirmed => {
                if let Some(id) = w.asking.take() {
                    w.text.retain(|s| s.id != id);
                    w.dimension.retain(|s| s.id != id);
                    w.chosen = None;
                    w.fill();
                }
            }
            Event::RemoveDropped => w.asking = None,
            Event::Name(name) => {
                if let Some(s) = w.text_style() {
                    s.name = name;
                } else if let Some(s) = w.dimension_style() {
                    s.name = name;
                }
            }
            Event::Font(f) => {
                if let Some(s) = w.text_style() {
                    if let Some(f) = f {
                        s.font = f;
                    }
                } else if let Some(s) = w.dimension_style() {
                    s.font = f;
                }
            }
            Event::Bold(on) => {
                if let Some(s) = w.text_style() {
                    s.bold = on;
                }
            }
            Event::Italic(on) => {
                if let Some(s) = w.text_style() {
                    s.italic = on;
                }
            }
            Event::Number(key, typed) => w.number(key, typed),
            Event::Arrow(a) => {
                if let Some(s) = w.dimension_style() {
                    s.arrow = a;
                }
            }
            Event::Centre(on) => {
                if let Some(s) = w.dimension_style() {
                    s.text_place = on.then_some(DimensionTextPlace::Centre);
                }
            }
            Event::Unit(u) => {
                if let Some(s) = w.dimension_style() {
                    s.unit = u;
                }
            }
            Event::Prefix(t) => {
                if let Some(s) = w.dimension_style() {
                    s.prefix = (!t.is_empty()).then_some(t);
                }
            }
            Event::Suffix(t) => {
                if let Some(s) = w.dimension_style() {
                    s.suffix = (!t.is_empty()).then_some(t);
                }
            }
            Event::LineColor(part, c) => {
                if let Some(s) = w.dimension_style() {
                    match part {
                        Part::Line => s.dim_line_color = c,
                        Part::Ext => s.ext_color = c,
                        Part::Value => s.text_color = c,
                    }
                }
            }
            // The value has a colour alone.
            Event::LineWeight(part, v) => {
                if let Some(s) = w.dimension_style() {
                    match part {
                        Part::Line => s.dim_line_weight = v,
                        Part::Ext => s.ext_weight = v,
                        Part::Value => {}
                    }
                }
            }
            Event::LineType(part, t) => {
                if let Some(s) = w.dimension_style() {
                    match part {
                        Part::Line => s.dim_line_type = t,
                        Part::Ext => s.ext_line_type = t,
                        Part::Value => {}
                    }
                }
            }
            Event::Save | Event::Cancel => {}
        }
        Task::none()
    }

    pub(crate) fn annotation_styles_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.annotation_styles else {
            return iced::widget::text("").into();
        };
        let Some(doc) = self.document.as_ref() else {
            return iced::widget::text("").into();
        };
        let settings = doc.model.settings();
        let project = settings.drawing_font.unwrap_or(DrawingFont::Barlow);
        let title = match w.kind {
            Kind::Text => TEXT_TITLE,
            Kind::Dimension => DIMENSION_TITLE,
        };
        // The list: Standart, then the styles, each with how many objects follow it.
        let mut list = Column::new().spacing(1).padding(2);
        let rows = std::iter::once((None, STANDARD_STYLE.to_owned()))
            .chain(w.names().into_iter().map(|(id, name)| (Some(id), name)));
        for (id, name) in rows {
            let count = id
                .as_ref()
                .map(|id| w.usage.get(id).copied().unwrap_or(0).to_string());
            let face = row![
                Elided::new(name)
                    .size(typography::body())
                    .font(typography::ui())
                    .width(Fill),
                label::caption(count.unwrap_or_default()).style(|theme: &Theme| {
                    iced::widget::text::Style {
                        color: Some(Tokens::of(theme).muted),
                    }
                }),
            ]
            .spacing(6)
            .align_y(Center);
            list = list.push(
                button(face)
                    .on_press(msg(Event::Choose(id.clone())))
                    .padding([4, 8])
                    .width(Fill)
                    .style(style::button::row(w.chosen == id)),
            );
        }
        let list = container(iced::widget::scrollable(list).height(typography::scaled(260.0)))
            .style(style::container::field_box)
            .width(Fill);
        let side = Column::new()
            .spacing(10)
            .width(typography::scaled(220.0))
            .push(words::field(title, list, None))
            .push(
                row![
                    words::secondary(NEW, Some(msg(Event::New))),
                    words::secondary(REMOVE, w.chosen.as_ref().map(|_| msg(Event::Remove))),
                ]
                .spacing(8),
            );
        let form: Element<'_, Message> = match (w.kind, w.chosen.as_ref()) {
            (_, None) => {
                let font = project.label();
                label::body(match w.kind {
                    Kind::Text => format!("Stilsiz yazılar projenin yazı tipiyle ({font}) yazılır; tek satırlı yazı eğiktir. Yazı tipi Proje ayarları'ndadır. Standart düzenlenmez: yeni bir stil için Yeni'ye basın."),
                    Kind::Dimension => format!("Stilsiz ölçüler: uçlarda çentik, 2,5 mm değer, projenin yazı tipi ({font}), birimi ve basamakları. Standart düzenlenmez: yeni bir stil için Yeni'ye basın."),
                })
                .into()
            }
            (Kind::Text, Some(id)) => match w.text.iter().find(|s| &s.id == id) {
                Some(s) => self.text_style_form(w, s),
                None => iced::widget::text("").into(),
            },
            (Kind::Dimension, Some(id)) => match w.dimension.iter().find(|s| &s.id == id) {
                Some(s) => self.dimension_style_form(w, s, project),
                None => iced::widget::text("").into(),
            },
        };
        let sample = Sample {
            kind: w.kind,
            text: w
                .chosen
                .as_ref()
                .and_then(|c| w.text.iter().find(|s| &s.id == c))
                .cloned(),
            dimension: w
                .chosen
                .as_ref()
                .and_then(|c| w.dimension.iter().find(|s| &s.id == c))
                .cloned(),
            project,
            scale: settings.plot_scale,
            // Standart's height is the project's Ölçü height (docs/adr/0205 §1).
            standard_mm: {
                let kind = kentos_contracts::AnnotationKind::Dimension;
                settings
                    .annotation
                    .as_ref()
                    .map_or_else(|| kind.default_mm(), |a| a.mm(kind))
            },
            format: Format::of(settings),
            // The drawing's own ground and ink, as the web's preview reads its canvas palette.
            colors: {
                let canvas = self.canvas();
                crate::labels::colors(canvas, &crate::viewport::palette(canvas))
            },
        };
        let preview = container(
            canvas::Canvas::new(sample)
                .width(Fill)
                .height(typography::scaled(120.0)),
        )
        .style(style::container::field_box)
        .width(Fill);
        let main = Column::new()
            .spacing(12)
            .width(Fill)
            .push(form)
            .push(words::field("Önizleme", preview, None));
        let mut body = Column::new().spacing(12);
        let problem = w.problem();
        if let Some(p) = &problem {
            let mut first = p.chars();
            let p = first
                .next()
                .map(|c| c.to_uppercase().collect::<String>() + first.as_str())
                .unwrap_or_default();
            body = body.push(words::summary(vec![words::text_line(
                Line::Warn,
                format!("{p}."),
            )]));
        }
        let body = body.push(row![side, main].spacing(16));
        let can_save = problem.is_none() && self.styles_changed();
        let window = overlay::modal(
            Frame_::new(title)
                .push(body)
                .action(words::secondary(CANCEL, Some(msg(Event::Cancel))))
                .action(words::primary(SAVE, can_save.then(|| msg(Event::Save))))
                .width(760.0),
            msg(Event::Cancel),
        );
        let Some(id) = &w.asking else {
            return window;
        };
        // Sil asked: objects follow the style; they keep their look, without the link.
        let n = w.usage.get(id).copied().unwrap_or(0);
        let name = w
            .names()
            .into_iter()
            .find(|(i, _)| i == id)
            .map(|(_, n)| n)
            .unwrap_or_default();
        let what = match w.kind {
            Kind::Text => "yazı",
            Kind::Dimension => "ölçü",
        };
        let question = overlay::modal(
            Confirm::new("Stili sil", msg(Event::RemoveConfirmed), msg(Event::RemoveDropped))
                .message(format!(
                    "{n} {what} “{name}” stilini kullanıyor. Kaydedince görünüşleri kalır, stilsiz olurlar."
                ))
                .confirm(REMOVE)
                .destructive(),
            msg(Event::RemoveDropped),
        );
        iced::widget::stack![window, question].into()
    }

    fn number_field<'a>(
        &'a self,
        w: &'a Window,
        (key, label, hint): (&'static str, &'static str, &'static str),
    ) -> Element<'a, Message> {
        let typed = w.typed.get(key).cloned().unwrap_or_default();
        let wrong = number(&typed).is_err();
        let input = text_input("", &typed)
            .on_input(move |t| msg(Event::Number(key, t)))
            .padding([5, 8])
            .style(style::field::validated(wrong));
        words::field(label, focus_ring(input), Some(hint.to_owned()))
    }

    fn text_style_form<'a>(&'a self, w: &'a Window, s: &'a TextStyleDef) -> Element<'a, Message> {
        let name = text_input("Stilin adı", &s.name)
            .on_input(|t| msg(Event::Name(t)))
            .padding([5, 8])
            .style(style::field::input);
        let font = Select::new(
            DrawingFont::ALL.iter().map(|f| Choice::new(f.label())),
            DrawingFont::ALL.iter().position(|f| *f == s.font),
            |i| msg(Event::Font(DrawingFont::ALL.get(i).copied())),
        )
        .searchable(false);
        let numbers = TEXT_NUMBERS.iter().fold(row![].spacing(12), |r, n| {
            r.push(container(self.number_field(w, *n)).width(Fill))
        });
        let mut form = Column::new()
            .spacing(10)
            .push(words::field(NAME, focus_ring(name), None))
            .push(
                // Each with its label above, as the web's (Kalınlık, Biçim).
                row![
                    container(words::field(FONT, font, None)).width(Fill),
                    container(words::field(
                        "Kalınlık",
                        boxed(words::check(s.bold, BOLD, Some(msg(Event::Bold(!s.bold))))),
                        None
                    ))
                    .width(Fill),
                    container(words::field(
                        "Biçim",
                        boxed(words::check(
                            s.italic,
                            ITALIC,
                            Some(msg(Event::Italic(!s.italic)))
                        )),
                        None
                    ))
                    .width(Fill),
                ]
                .spacing(12)
                .align_y(iced::Top),
            )
            .push(numbers);
        if let Some(f) = &s.font_file {
            form = form.push(words::field(
                "Yazı tipi dosyası",
                label::caption(f.clone()),
                Some("DXF dosyasından; DXF’e aynen yazılır.".to_owned()),
            ));
        }
        form.into()
    }

    fn dimension_style_form<'a>(
        &'a self,
        w: &'a Window,
        s: &'a DimensionStyleDef,
        project: DrawingFont,
    ) -> Element<'a, Message> {
        let name = text_input("Stilin adı", &s.name)
            .on_input(|t| msg(Event::Name(t)))
            .padding([5, 8])
            .style(style::field::input);
        let arrows = Select::new(
            ARROW_CHOICES.iter().map(|(_, l)| Choice::new(*l)),
            ARROW_CHOICES.iter().position(|(a, _)| *a == s.arrow),
            |i| msg(Event::Arrow(ARROW_CHOICES.get(i).and_then(|(a, _)| *a))),
        )
        .searchable(false);
        let place = Select::new(
            [
                Choice::new("Çizginin üstünde"),
                Choice::new("Çizginin ortasında"),
            ],
            Some(usize::from(s.text_place.is_some())),
            |i| msg(Event::Centre(i == 1)),
        )
        .searchable(false);
        let units = Select::new(
            UNIT_CHOICES.iter().map(|(_, l)| Choice::new(*l)),
            UNIT_CHOICES.iter().position(|(u, _)| *u == s.unit),
            |i| msg(Event::Unit(UNIT_CHOICES.get(i).and_then(|(u, _)| *u))),
        )
        .searchable(false);
        let fonts = Select::new(
            std::iter::once(Choice::new(format!("Projenin ({})", project.label())))
                .chain(DrawingFont::ALL.iter().map(|f| Choice::new(f.label()))),
            Some(
                s.font
                    .and_then(|f| DrawingFont::ALL.iter().position(|x| *x == f))
                    .map_or(0, |i| i + 1),
            ),
            |i| {
                msg(Event::Font(
                    i.checked_sub(1)
                        .and_then(|i| DrawingFont::ALL.get(i).copied()),
                ))
            },
        )
        .searchable(false);
        let affix = |label: &'static str, value: &'a Option<String>, on: fn(String) -> Event| {
            let input = text_input("", value.as_deref().unwrap_or_default())
                .on_input(move |t| msg(on(t)))
                .padding([5, 8])
                .style(style::field::input);
            container(words::field(
                label,
                focus_ring(input),
                Some("Boş: yok".to_owned()),
            ))
            .width(Fill)
        };
        let n = |i: usize| container(self.number_field(w, DIMENSION_NUMBERS[i])).width(Fill);
        // The rows' labels on one line and their fields on the next, hints or not.
        Column::new()
            .spacing(10)
            .push(words::field(NAME, focus_ring(name), None))
            .push(
                row![
                    n(0),
                    container(words::field(ARROWS, arrows, None)).width(Fill),
                    n(1)
                ]
                .spacing(12)
                .align_y(iced::Top),
            )
            .push(row![n(2), n(3), n(4)].spacing(12))
            .push(
                row![
                    container(words::field(PLACE, place, None)).width(Fill),
                    n(5),
                    container(words::field(UNIT, units, None)).width(Fill),
                ]
                .spacing(12)
                .align_y(iced::Top),
            )
            .push(
                row![
                    affix(PREFIX, &s.prefix, Event::Prefix),
                    affix(SUFFIX, &s.suffix, Event::Suffix),
                    container(words::field(FONT, fonts, None)).width(Fill),
                ]
                .spacing(12)
                .align_y(iced::Top),
            )
            // Its lines (docs/adr/0205 §6), as the web's Çizgiler.
            .push(kentos_ui::label::strong("Çizgiler"))
            .push(
                row![
                    container(words::field(
                        "Ölçü çizgisi",
                        color_select(Part::Line, s.dim_line_color.as_deref()),
                        None
                    ))
                    .width(Fill),
                    container(words::field(
                        "Kalınlık",
                        weight_select(Part::Line, s.dim_line_weight),
                        None
                    ))
                    .width(Fill),
                    container(words::field(
                        "Tip",
                        type_select(Part::Line, s.dim_line_type),
                        None
                    ))
                    .width(Fill),
                ]
                .spacing(12)
                .align_y(iced::Top),
            )
            .push(
                row![
                    container(words::field(
                        "Uzatma çizgileri",
                        color_select(Part::Ext, s.ext_color.as_deref()),
                        None
                    ))
                    .width(Fill),
                    container(words::field(
                        "Kalınlık",
                        weight_select(Part::Ext, s.ext_weight),
                        None
                    ))
                    .width(Fill),
                    container(words::field(
                        "Tip",
                        type_select(Part::Ext, s.ext_line_type),
                        None
                    ))
                    .width(Fill),
                ]
                .spacing(12)
                .align_y(iced::Top),
            )
            .push(
                row![
                    container(words::field(
                        "Değer",
                        color_select(Part::Value, s.text_color.as_deref()),
                        None
                    ))
                    .width(Fill),
                    iced::widget::space::horizontal(),
                    iced::widget::space::horizontal(),
                ]
                .spacing(12)
                .align_y(iced::Top),
            )
            .into()
    }

    /// The window's controls by their words (a trace's `dialog` step): a
    /// style's row by its name, the fields by their labels, the buttons.
    pub(crate) fn annotation_styles_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.annotation_styles else {
            return Err(format!("{TEXT_TITLE} penceresi açık değil"));
        };
        let title = match w.kind {
            Kind::Text => TEXT_TITLE,
            Kind::Dimension => DIMENSION_TITLE,
        };
        let numbers: Vec<(&'static str, &'static str, &'static str)> = match w.kind {
            Kind::Text => TEXT_NUMBERS.to_vec(),
            Kind::Dimension => DIMENSION_NUMBERS.to_vec(),
        };
        let fonts = |item: &str| DrawingFont::ALL.into_iter().find(|f| f.label() == item);
        Ok(match control {
            Control::Fill(NAME, text) => Some(msg(Event::Name(text.to_owned()))),
            Control::Fill(PREFIX, text) => Some(msg(Event::Prefix(text.to_owned()))),
            Control::Fill(SUFFIX, text) => Some(msg(Event::Suffix(text.to_owned()))),
            Control::Fill(label, text) => match numbers.iter().find(|(_, l, _)| *l == label) {
                Some((key, _, _)) => Some(msg(Event::Number(key, text.to_owned()))),
                None => return Err(format!("“{title}” penceresinde “{label}” alanı yok")),
            },
            Control::Check(BOLD, on) => Some(msg(Event::Bold(on))),
            Control::Check(ITALIC, on) => Some(msg(Event::Italic(on))),
            Control::Pick(FONT, item) => match (w.kind, fonts(item)) {
                (_, Some(f)) => Some(msg(Event::Font(Some(f)))),
                (Kind::Dimension, None) if item.starts_with("Projenin") => {
                    Some(msg(Event::Font(None)))
                }
                _ => return Err(format!("“{FONT}” listesinde “{item}” yok")),
            },
            Control::Pick(ARROWS, item) => match ARROW_CHOICES.iter().find(|(_, l)| *l == item) {
                Some((a, _)) => Some(msg(Event::Arrow(*a))),
                None => return Err(format!("“{ARROWS}” listesinde “{item}” yok")),
            },
            Control::Pick(PLACE, item) => Some(msg(Event::Centre(item == "Çizginin ortasında"))),
            Control::Pick(UNIT, item) => match UNIT_CHOICES.iter().find(|(_, l)| *l == item) {
                Some((u, _)) => Some(msg(Event::Unit(*u))),
                None => return Err(format!("“{UNIT}” listesinde “{item}” yok")),
            },
            Control::Press(NEW) => Some(msg(Event::New)),
            Control::Press(REMOVE) if w.asking.is_some() => Some(msg(Event::RemoveConfirmed)),
            Control::Press(REMOVE) => w.chosen.as_ref().map(|_| msg(Event::Remove)),
            Control::Press(SAVE) => Some(msg(Event::Save)),
            Control::Press(CANCEL) => Some(msg(Event::Cancel)),
            Control::Press(STANDARD_STYLE) => Some(msg(Event::Choose(None))),
            Control::Press(words) => match w.names().into_iter().find(|(_, n)| n == words) {
                Some((id, _)) => Some(msg(Event::Choose(Some(id)))),
                None => return Err(format!("“{title}” penceresinde “{words}” düğmesi yok")),
            },
            other => return Err(format!("“{title}” penceresinde {other} yok")),
        })
    }
}

/// The preview: a sample text in the chosen style, or an aligned dimension
/// 60 mm long on the paper at the project's scale, drawn as the drawing draws
/// them, on the drawing's ground.
struct Sample {
    kind: Kind,
    text: Option<TextStyleDef>,
    dimension: Option<DimensionStyleDef>,
    project: DrawingFont,
    scale: f64,
    /// Standart's value height on paper, mm (the project's Ölçü height).
    standard_mm: f64,
    format: Format,
    colors: crate::labels::Colors,
}

impl canvas::Program<Message> for Sample {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        _theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let colors = self.colors;
        let ink = (colors.label, Color::TRANSPARENT);
        let mut frame = Frame::new(renderer, bounds.size());
        let (w, h) = (bounds.width, bounds.height);
        frame.fill_rectangle(Point::ORIGIN, bounds.size(), colors.halo);
        match self.kind {
            Kind::Text => {
                let face = self
                    .text
                    .as_ref()
                    .map(TextStyleDef::face)
                    .unwrap_or_default();
                let factor = self
                    .text
                    .as_ref()
                    .and_then(|s| s.width_factor)
                    .unwrap_or(1.0) as f32;
                crate::labels::sample_text(
                    &mut frame,
                    "Ada 104 · Parsel 12",
                    Point::new(16.0, h / 2.0 + 30.0 * 0.35),
                    30.0,
                    &face,
                    factor,
                    self.project,
                    ink,
                );
            }
            Kind::Dimension => {
                use kentos_geometry_core::geom::dimension::{DimensionGeom, layout_dimension};
                let s = self.dimension.as_ref();
                let look = s.map(DimensionStyleDef::look).unwrap_or_default();
                let height = s.map_or(self.standard_mm, |s| s.height) / 1000.0 * self.scale;
                let len = 60.0 / 1000.0 * self.scale;
                let g = DimensionGeom {
                    a: kentos_geometry_core::vec2::Vec2::new(0.0, 0.0),
                    b: kentos_geometry_core::vec2::Vec2::new(len, 0.0),
                    offset: 12.0 / 1000.0 * self.scale,
                    height,
                    style: None,
                    angle: None,
                    c: None,
                    za: None,
                    zb: None,
                    look: kentos_native_application::geometry::core_look(&look),
                };
                let Some(l) = layout_dimension(&g) else {
                    return vec![frame.into_geometry()];
                };
                let pts: Vec<_> = l
                    .lines
                    .iter()
                    .flatten()
                    .chain(l.fills.iter().flatten().flatten())
                    .chain(std::iter::once(&l.text_at))
                    .copied()
                    .collect();
                let (min_x, max_x) = pts
                    .iter()
                    .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.x), b.max(p.x)));
                let (min_y, max_y) = pts
                    .iter()
                    .fold((f64::MAX, f64::MIN), |(a, b), p| (a.min(p.y), b.max(p.y)));
                let max_y = max_y + height * 1.2;
                let k = ((f64::from(w) - 32.0) / (max_x - min_x).max(1e-9))
                    .min((f64::from(h) - 24.0) / (max_y - min_y).max(1e-9));
                let left = 16.0 + (f64::from(w) - 32.0 - (max_x - min_x) * k) / 2.0;
                let at = |p: kentos_geometry_core::vec2::Vec2| {
                    Point::new(
                        (left + (p.x - min_x) * k) as f32,
                        (f64::from(h) - 12.0 - (p.y - min_y) * k) as f32,
                    )
                };
                // Its lines as its look names them (docs/adr/0205 §6): colour, weight at the
                // preview's paper size and type, as the web's preview draws them.
                let paper = |mm: f64| ((mm / 1000.0 * self.scale * k) as f32).max(1.0);
                let pen = |color: &Option<String>,
                           weight: Option<f64>,
                           kind: Option<kentos_contracts::LineType>| {
                    let dash: &'static [f64] = match kind {
                        Some(kentos_contracts::LineType::Dashed) => &[3.0, 1.5],
                        Some(kentos_contracts::LineType::Dashdot) => &[5.0, 1.2, 0.6, 1.2],
                        Some(kentos_contracts::LineType::Dotted) => &[0.6, 1.2],
                        _ => &[],
                    };
                    (
                        color
                            .as_deref()
                            .map_or(colors.label, crate::view::hex_color),
                        weight.map_or(1.2, paper),
                        dash.iter().map(|mm| paper(*mm)).collect::<Vec<f32>>(),
                    )
                };
                let line = pen(
                    &look.dim_line_color,
                    look.dim_line_weight,
                    look.dim_line_type,
                );
                let ext = pen(&look.ext_color, look.ext_weight, look.ext_line_type);
                for (i, [p, q]) in l.lines.iter().enumerate() {
                    let (color, width, dash) = if l.ext.contains(&i) { &ext } else { &line };
                    let stroke = Stroke {
                        line_dash: canvas::LineDash {
                            segments: dash,
                            offset: 0,
                        },
                        ..Stroke::default().with_width(*width).with_color(*color)
                    };
                    frame.stroke(&Path::line(at(*p), at(*q)), stroke);
                }
                for ring in l.fills.iter().flatten() {
                    let path = Path::new(|b| {
                        for (i, p) in ring.iter().enumerate() {
                            if i == 0 {
                                b.move_to(at(*p));
                            } else {
                                b.line_to(at(*p));
                            }
                        }
                        b.close();
                    });
                    frame.fill(&path, line.0);
                }
                let text = self.format.dimension_in(l.prefix, l.unit, l.value, &look);
                crate::labels::sample_value(
                    &mut frame,
                    &text,
                    at(l.text_at),
                    -(l.rotation.to_radians() as f32),
                    (height * k) as f32,
                    look.font.unwrap_or(self.project),
                    look.text_place.is_some(),
                    (
                        look.text_color
                            .as_deref()
                            .map_or(colors.label, crate::view::hex_color),
                        colors.halo,
                    ),
                );
            }
        }
        vec![frame.into_geometry()]
    }
}

/// A colour of a dimension's line: none (the object's), a drawing colour
/// but ink, or the one it has (the web's `colorSelect`).
fn color_select<'a>(part: Part, value: Option<&str>) -> Element<'a, Message> {
    let colors = crate::ribbon_panels::line_colors();
    let mut values: Vec<Option<String>> = vec![None];
    let mut labels: Vec<String> = vec!["Nesnenin rengi".to_owned()];
    for (name, v) in colors {
        values.push(Some(v.to_owned()));
        labels.push(name.to_owned());
    }
    if let Some(v) = value
        && !values.iter().flatten().any(|c| c.eq_ignore_ascii_case(v))
    {
        values.push(Some(v.to_owned()));
        labels.push(v.to_uppercase());
    }
    let chosen = values.iter().position(|c| match (c, value) {
        (None, None) => true,
        (Some(c), Some(v)) => c.eq_ignore_ascii_case(v),
        _ => false,
    });
    Select::new(labels.into_iter().map(Choice::new), chosen, move |i| {
        msg(Event::LineColor(part, values.get(i).cloned().flatten()))
    })
    .searchable(false)
    .into()
}

/// A weight of a dimension's line on paper: none (a hairline) or one of the
/// drawing's weights (the web's `weightSelect`).
fn weight_select<'a>(part: Part, value: Option<f64>) -> Element<'a, Message> {
    let weights = crate::ribbon_panels::LINE_WEIGHTS;
    let mut values: Vec<Option<f64>> = std::iter::once(None)
        .chain(weights.iter().map(|w| Some(*w)))
        .collect();
    if let Some(v) = value
        && !weights.contains(&v)
    {
        values.push(Some(v));
    }
    let labels = values.iter().map(|w| match w {
        None => "Kılcal".to_owned(),
        Some(w) => crate::ribbon_panels::weight_text(*w),
    });
    let chosen = values.iter().position(|w| *w == value);
    Select::new(
        labels.map(Choice::new).collect::<Vec<_>>(),
        chosen,
        move |i| msg(Event::LineWeight(part, values.get(i).copied().flatten())),
    )
    .searchable(false)
    .into()
}

/// A type of a dimension's line: none (continuous) or another (the web's `typeSelect`).
fn type_select<'a>(part: Part, value: Option<kentos_contracts::LineType>) -> Element<'a, Message> {
    use crate::ribbon_panels::LINE_TYPES;
    use kentos_contracts::LineType;
    let chosen = LINE_TYPES
        .iter()
        .position(|(t, _)| *t == value.unwrap_or(LineType::Continuous));
    Select::new(
        LINE_TYPES.iter().map(|(_, label)| Choice::new(*label)),
        chosen,
        move |i| {
            let t = LINE_TYPES
                .get(i)
                .map(|(t, _)| *t)
                .filter(|t| *t != LineType::Continuous);
            msg(Event::LineType(part, t))
        },
    )
    .searchable(false)
    .into()
}
