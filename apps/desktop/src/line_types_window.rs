//! Çizgi tipleri (docs/adr/0313 §7; AutoCAD's LINETYPE manager): the project's line types with patterns, each with its
//! name, unit, source and its pattern drawn as the drawing draws it (paper millimetres, a text or a shape as the room
//! it leaves), what it cannot draw said; LIN'den yükle… and Dışa aktar… (a LIN file), Sil, and the project's line type
//! scale. Every write goes through `cad.linetypes.edit`: the table is a project setting, no undo step.

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{Canvas, Column, button, container, row, text, text_input};
use iced::{Center, Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_contracts::{
    CommandResult, LineTypeDef, LineTypesEdit, LineTypesEdited, LineTypesOperation, PatternUnit,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, line_types_edit};
use kentos_native_style::line_types::{MAX_DASHES, dashes_of};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame2, Elided, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const LINE_TYPES_TITLE: &str = "Çizgi tipleri";
const LOAD: &str = "LIN'den yükle…";
const EXPORT: &str = "Dışa aktar…";
const REMOVE: &str = "Sil";
const CLOSE: &str = "Kapat";
const ROW: f32 = 34.0;

#[derive(Debug, Clone, Default)]
pub struct Window {
    /// The type chosen in the list, by name.
    pub chosen: Option<String>,
    /// The line type scale as typed.
    pub scale: String,
    /// What the last write said, shown under the list.
    said: Option<(Kind, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(String),
    Load,
    Loaded(Option<(String, String)>),
    Export,
    Exported(Option<std::path::PathBuf>),
    Remove,
    Scale(String),
    ScaleSubmit,
    Close,
}

fn msg(event: Event) -> Message {
    Message::LineTypes(event)
}

/// A type's pattern drawn three periods long, millimetres on paper, fitted into the box.
struct Pattern {
    dash: Vec<f64>,
}

impl<M> canvas::Program<M> for Pattern {
    type State = ();

    fn draw(
        &self,
        _state: &(),
        renderer: &Renderer,
        theme: &Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<Geometry> {
        let mut frame = Frame::new(renderer, bounds.size());
        let ink = Tokens::of(theme).text;
        let y = bounds.height / 2.0;
        let stroke = Stroke::default().with_color(ink).with_width(1.5);
        let period: f64 = self.dash.iter().sum();
        if self.dash.is_empty() || period <= 0.0 {
            frame.stroke(
                &Path::line(Point::new(2.0, y), Point::new(bounds.width - 2.0, y)),
                stroke,
            );
            return vec![frame.into_geometry()];
        }
        let width = f64::from(bounds.width) - 4.0;
        let k = width / (3.0 * period);
        let mut at = 0.0;
        let mut i = 0usize;
        while at < width {
            let l = self.dash[i % self.dash.len()] * k;
            if i.is_multiple_of(2) {
                let end = (at + l.max(1.0)).min(width);
                frame.stroke(
                    &Path::line(
                        Point::new((2.0 + at) as f32, y),
                        Point::new((2.0 + end) as f32, y),
                    ),
                    stroke,
                );
            }
            at += l;
            i += 1;
            if i > 4096 {
                break;
            }
        }
        vec![frame.into_geometry()]
    }
}

impl App {
    /// Çizgi tipleri (`lineTypes.manage`).
    pub(crate) fn open_line_types(&mut self) {
        let Some(doc) = &self.document else {
            self.output("Açık çizim yok.");
            return;
        };
        let scale = doc.model.settings().line_type_scale.unwrap_or(1.0);
        self.line_types_window = Some(Window {
            chosen: None,
            scale: crate::properties::rows::plain_number(scale),
            said: None,
        });
        self.dialog = Some(Dialog::LineTypes);
    }

    fn line_types(&self) -> Vec<LineTypeDef> {
        self.document
            .as_ref()
            .map(|d| d.model.settings().line_types.clone())
            .unwrap_or_default()
    }

    /// Runs the command, says what it did; whether it wrote.
    fn edit_line_types(&mut self, input: LineTypesEdit) -> Option<LineTypesEdited> {
        let doc = self.document.as_mut()?;
        let result = line_types_edit::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let done = match result {
            CommandResult::Completed { output, .. } => output,
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                if let Some(w) = self.line_types_window.as_mut() {
                    w.said = Some((Kind::Warn, error.message.clone()));
                }
                self.warn(error.message);
                return None;
            }
            _ => return None,
        };
        Some(done)
    }

    pub(crate) fn line_types_event(&mut self, event: Event) -> Task<Message> {
        if self.line_types_window.is_none() {
            return Task::none();
        }
        match event {
            Event::Choose(name) => {
                if let Some(w) = self.line_types_window.as_mut() {
                    w.chosen = Some(name);
                }
            }
            Event::Load => {
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Çizgi tiplerini LIN dosyasından yükle")
                            .add_filter("Çizgi tipi dosyası (.lin)", &["lin", "LIN"])
                            .pick_file()
                            .await?;
                        let bytes = file.read().await;
                        Some((
                            file.file_name(),
                            String::from_utf8_lossy(&bytes).into_owned(),
                        ))
                    },
                    |read| msg(Event::Loaded(read)),
                );
            }
            Event::Loaded(None) | Event::Exported(None) => {}
            Event::Loaded(Some((name, text))) => {
                let input = LineTypesEdit {
                    operation: LineTypesOperation::Load,
                    lin: Some(text),
                    source: Some(name.clone()),
                    names: None,
                    replace: false,
                    line_type: None,
                    name: None,
                    scale: None,
                    expected_revision: None,
                };
                if let Some(done) = self.edit_line_types(input) {
                    let words = loaded_words(&name, &done);
                    let kind = if done.problems.is_empty() {
                        Kind::Ok
                    } else {
                        Kind::Warn
                    };
                    self.say(Level::Info, words.clone());
                    if let Some(w) = self.line_types_window.as_mut() {
                        w.said = Some((kind, words));
                    }
                }
            }
            Event::Export => {
                let name = self
                    .document
                    .as_ref()
                    .map_or_else(|| "cizim".to_owned(), |d| d.model.name().to_owned());
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Çizgi tiplerini dışa aktar")
                            .add_filter("Çizgi tipi dosyası (.lin)", &["lin"])
                            .set_file_name(format!("{name}.lin"))
                            .save_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| msg(Event::Exported(path)),
                );
            }
            Event::Exported(Some(path)) => {
                let input = LineTypesEdit {
                    operation: LineTypesOperation::Export,
                    lin: None,
                    source: None,
                    names: None,
                    replace: false,
                    line_type: None,
                    name: None,
                    scale: None,
                    expected_revision: None,
                };
                if let Some(text) = self.edit_line_types(input).and_then(|d| d.lin) {
                    match std::fs::write(&path, text) {
                        Ok(()) => self.say(
                            Level::Success,
                            format!("Çizgi tipleri {} dosyasına yazıldı.", path.display()),
                        ),
                        Err(e) => self.warn(format!(
                            "{} yazılamadı: {e}. Başka bir klasör seçin.",
                            path.display()
                        )),
                    }
                }
            }
            Event::Remove => {
                let Some(name) = self
                    .line_types_window
                    .as_ref()
                    .and_then(|w| w.chosen.clone())
                else {
                    return Task::none();
                };
                let input = LineTypesEdit {
                    operation: LineTypesOperation::Remove,
                    lin: None,
                    source: None,
                    names: None,
                    replace: false,
                    line_type: None,
                    name: Some(name.clone()),
                    scale: None,
                    expected_revision: None,
                };
                if self.edit_line_types(input).is_some()
                    && let Some(w) = self.line_types_window.as_mut()
                {
                    w.chosen = None;
                    w.said = Some((Kind::Ok, format!("“{name}” silindi.")));
                }
            }
            Event::Scale(t) => {
                if let Some(w) = self.line_types_window.as_mut() {
                    w.scale = t;
                }
            }
            Event::ScaleSubmit => {
                let typed = self
                    .line_types_window
                    .as_ref()
                    .map(|w| w.scale.trim().replace(',', "."))
                    .unwrap_or_default();
                let Ok(scale) = typed.parse::<f64>() else {
                    if let Some(w) = self.line_types_window.as_mut() {
                        w.said = Some((
                            Kind::Warn,
                            format!(
                                "“{typed}” bir sayı değil; 0'dan büyük, en çok 1000 bir ölçek yazın."
                            ),
                        ));
                    }
                    return Task::none();
                };
                let input = LineTypesEdit {
                    operation: LineTypesOperation::Scale,
                    lin: None,
                    source: None,
                    names: None,
                    replace: false,
                    line_type: None,
                    name: None,
                    scale: Some(scale),
                    expected_revision: None,
                };
                if let Some(done) = self.edit_line_types(input)
                    && let Some(w) = self.line_types_window.as_mut()
                {
                    let now = done.scale.unwrap_or(1.0);
                    w.scale = crate::properties::rows::plain_number(now);
                    w.said = Some((
                        Kind::Ok,
                        format!(
                            "Çizgi tipi ölçeği {}: bütün desenler bu kadar uzun çizilir.",
                            crate::properties::rows::plain_number(now)
                        ),
                    ));
                }
            }
            Event::Close => {
                self.line_types_window = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn line_types_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.line_types_window else {
            return text("").into();
        };
        let settings = self.document.as_ref().map(|d| d.model.settings());
        let (plot, unit) = settings.map_or((1000.0, (1.0, 1.0)), |s| {
            (
                s.plot_scale,
                s.drawing_unit
                    .unwrap_or(kentos_contracts::DrawingUnit::M)
                    .metres(),
            )
        });
        let types = self.line_types();
        let mut list = Column::new().spacing(1).padding(2);
        if types.is_empty() {
            list = list.push(
                container(label::caption(
                    "Projede desenli çizgi tipi yok. LIN'den yükle… ile bir LIN dosyasının tiplerini alın; yerleşik tipler (Sürekli, Kesikli, Noktalı kesik, Noktalı) her zaman vardır.",
                ))
                .padding(10),
            );
        }
        for t in &types {
            let dashes = dashes_of(t, plot, unit);
            let (dash, lossy) = dashes.map_or((Vec::new(), false), |d| (d.dash, d.lossy));
            let unit_words = match t.unit {
                PatternUnit::Paper => "kâğıtta",
                PatternUnit::Drawing => "çizimde",
            };
            let note = match (lossy, t.elements.iter().any(|e| e.is_complex())) {
                (_, true) => "yazı ve şekil: boşluk",
                (true, false) => "ilk dört çizgi ve boşluk çizilir",
                _ => "",
            };
            let meta = [unit_words, t.source.as_deref().unwrap_or(""), note]
                .iter()
                .filter(|s| !s.is_empty())
                .copied()
                .collect::<Vec<_>>()
                .join(" · ");
            let name = Elided::new(t.name.clone())
                .size(typography::body())
                .font(typography::ui())
                .width(typography::scaled(110.0));
            let description = column_of(t.description.clone(), meta);
            let picture = container(
                Canvas::new(Pattern { dash })
                    .width(Fill)
                    .height(typography::scaled(14.0)),
            )
            .width(typography::scaled(180.0));
            let chosen = w.chosen.as_deref() == Some(t.name.as_str());
            let face = row![name, description, picture]
                .spacing(10)
                .align_y(Center)
                .height(typography::scaled(ROW));
            list = list.push(
                button(face)
                    .on_press(msg(Event::Choose(t.name.clone())))
                    .padding([2, 8])
                    .width(Fill)
                    .style(style::button::row(chosen)),
            );
        }
        let table = container(iced::widget::scrollable(list).height(typography::scaled(220.0)))
            .style(style::container::field_box)
            .width(Fill);
        let actions = row![
            words::secondary(LOAD, Some(msg(Event::Load))),
            words::secondary(EXPORT, (!types.is_empty()).then(|| msg(Event::Export))),
            words::secondary(REMOVE, w.chosen.is_some().then(|| msg(Event::Remove))),
        ]
        .spacing(8);
        let scale = row![
            container(
                text_input("1", &w.scale)
                    .on_input(|t| msg(Event::Scale(t)))
                    .on_submit(msg(Event::ScaleSubmit))
                    .padding([4, 6])
                    .size(typography::caption())
                    .style(style::field::input)
            )
            .width(typography::scaled(80.0)),
            words::secondary("Uygula", Some(msg(Event::ScaleSubmit))),
        ]
        .spacing(8)
        .align_y(Center);
        let mut body = Column::new()
            .spacing(12)
            .push(words::field(
                "Projenin çizgi tipleri",
                table,
                Some(format!(
                    "Bir ekranın desenini en çok {} çizgi ve boşluk (dört çift) çizer; pafta ve PDF de aynısını çizer.",
                    MAX_DASHES
                )),
            ))
            .push(actions)
            .push(words::field(
                "Çizgi tipi ölçeği (LTSCALE)",
                scale,
                Some("Bütün desenler bu sayıyla çarpılır; nesnenin kendi ölçeği ayrıca çarpılır.".to_owned()),
            ));
        if let Some((kind, said)) = &w.said {
            body = body.push(words::summary(vec![words::text_line(*kind, said.clone())]));
        }
        overlay::modal(
            Frame2::new(LINE_TYPES_TITLE)
                .push(body)
                .action(words::primary(CLOSE, Some(msg(Event::Close))))
                .width(640.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn line_types_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.line_types_window.is_none() {
            return Err(format!("{LINE_TYPES_TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Fill("Çizgi tipi ölçeği", t) => Some(msg(Event::Scale(t.to_owned()))),
            other => return Err(format!("“{LINE_TYPES_TITLE}” penceresinde {other} yok")),
        })
    }
}

fn column_of<'a>(description: String, meta: String) -> Element<'a, Message> {
    let mut c = Column::new().spacing(1).width(Fill);
    if !description.is_empty() {
        c = c.push(
            Elided::new(description)
                .size(typography::caption())
                .font(typography::ui())
                .width(Fill),
        );
    }
    // One line, cut with an ellipsis: the row keeps its height whatever the file's name.
    c.push(
        Elided::new(meta)
            .size(typography::caption())
            .font(typography::ui())
            .width(Fill)
            .style(|theme: &Theme| iced::widget::text::Style {
                color: Some(Tokens::of(theme).muted),
            }),
    )
    .into()
}

/// What a load did, in a sentence.
fn loaded_words(file: &str, done: &LineTypesEdited) -> String {
    let mut parts = Vec::new();
    if !done.added.is_empty() {
        parts.push(format!("{} tip eklendi", done.added.len()));
    }
    if !done.replaced.is_empty() {
        parts.push(format!("{} tip değişti", done.replaced.len()));
    }
    if !done.skipped.is_empty() {
        parts.push(format!(
            "{} tip projede vardı, kaldı ({})",
            done.skipped.len(),
            done.skipped.join(", ")
        ));
    }
    if !done.problems.is_empty() {
        let first = &done.problems[0];
        parts.push(format!(
            "{} satır okunmadı (ilki {}. satır: {})",
            done.problems.len(),
            first.line,
            first.message
        ));
    }
    if parts.is_empty() {
        parts.push("alınacak tip yok".to_owned());
    }
    format!("{file}: {}.", parts.join("; "))
}
