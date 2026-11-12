//! Tarama desenleri (docs/adr/0331 §1; AutoCAD's PAT files): the project's own hatch patterns, each with its name,
//! unit, source and a sample of its families drawn by the core's pattern pieces; PAT'tan yükle… (the file's numbers in
//! the unit chosen above the list), Dışa aktar… (a PAT file) and Sil. Every write goes through
//! `cad.hatchpatterns.edit`: the table is a project setting, no undo step. Tarama's Desen menu offers them after the
//! library's.

use iced::widget::canvas::{self, Frame, Geometry, Path, Stroke};
use iced::widget::{Canvas, Column, button, container, row, text};
use iced::{Center, Element, Fill, Point, Rectangle, Renderer, Task, Theme, mouse};
use kentos_commands::{
    CommandResult, HatchPattern, HatchPatternDef, HatchPatternType, HatchPatternUnit,
    HatchPatternsEdit, HatchPatternsEdited, HatchPatternsOperation,
};
use kentos_geometry_core::Vec2;
use kentos_geometry_core::geom::hatch_pattern::pattern_pieces;
use kentos_interaction::Level;
use kentos_native_application::geometry::core_pattern;
use kentos_native_application::{ExecutionContext, hatch_patterns_edit};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame2, Elided, Segmented};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const HATCH_PATTERNS_TITLE: &str = "Tarama desenleri";
const LOAD: &str = "PAT'tan yükle…";
const EXPORT: &str = "Dışa aktar…";
const REMOVE: &str = "Sil";
const CLOSE: &str = "Kapat";
const ROW: f32 = 40.0;

/// What a PAT file's numbers are in, as the window offers it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Unit(pub HatchPatternUnit);

impl std::fmt::Display for Unit {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(unit_words(self.0))
    }
}

fn unit_words(u: HatchPatternUnit) -> &'static str {
    match u {
        HatchPatternUnit::Mm => "mm (kâğıtta)",
        HatchPatternUnit::Inch => "inç (kâğıtta)",
        HatchPatternUnit::Drawing => "çizim birimi",
    }
}

#[derive(Debug, Clone)]
pub struct Window {
    /// The pattern chosen in the list, by name.
    pub chosen: Option<String>,
    /// What a file loaded next is in.
    pub unit: HatchPatternUnit,
    /// What the last write said, shown under the list.
    said: Option<(Kind, String)>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Choose(String),
    Unit(HatchPatternUnit),
    Load,
    Loaded(Option<(String, String)>),
    Export,
    Exported(Option<std::path::PathBuf>),
    Remove,
    Close,
}

fn msg(event: Event) -> Message {
    Message::HatchPatterns(event)
}

/// A pattern's sample: its families cut to the box by the core (`pattern_pieces`), about seven pixels apart.
struct Sample {
    pattern: kentos_geometry_core::entity::HatchPattern,
}

impl Sample {
    fn of(def: &HatchPatternDef) -> Self {
        let widest = def
            .lines
            .iter()
            .map(|l| l.offset[1].abs())
            .fold(0.0_f64, f64::max);
        let k = if widest > 0.0 { 7.0 / widest } else { 1.0 };
        let pattern = HatchPattern {
            kind: HatchPatternType::Pattern,
            angle: 0.0,
            spacing: 1.0,
            name: Some(def.name.clone()),
            scale: Some(k),
            lines: Some(def.lines.clone()),
            gradient: None,
            origin: None,
        };
        Self {
            pattern: core_pattern(&pattern),
        }
    }
}

impl<M> canvas::Program<M> for Sample {
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
        let tokens = Tokens::of(theme);
        let (w, h) = (f64::from(bounds.width), f64::from(bounds.height));
        let ring = [
            Vec2::new(0.0, 0.0),
            Vec2::new(w, 0.0),
            Vec2::new(w, h),
            Vec2::new(0.0, h),
        ];
        let pieces = pattern_pieces(&ring, &[], &self.pattern, 4000);
        let ink = Stroke::default().with_color(tokens.text).with_width(1.0);
        // The pattern's y goes up; the canvas's down.
        let at = |p: Vec2| Point::new(p.x as f32, (h - p.y) as f32);
        for [a, b] in &pieces.segments {
            frame.stroke(&Path::line(at(*a), at(*b)), ink);
        }
        for d in &pieces.dots {
            frame.fill(&Path::circle(at(*d), 0.9), tokens.text);
        }
        frame.stroke(
            &Path::rectangle(Point::ORIGIN, bounds.size()),
            Stroke::default().with_color(tokens.border).with_width(1.0),
        );
        vec![frame.into_geometry()]
    }
}

impl App {
    /// Tarama desenleri (`hatchPatterns.manage`).
    pub(crate) fn open_hatch_patterns(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        self.hatch_patterns_window = Some(Window {
            chosen: None,
            unit: HatchPatternUnit::Mm,
            said: None,
        });
        self.dialog = Some(Dialog::HatchPatterns);
    }

    fn hatch_patterns(&self) -> Vec<HatchPatternDef> {
        self.document
            .as_ref()
            .map(|d| d.model.settings().hatch_patterns.clone())
            .unwrap_or_default()
    }

    /// Runs the command, says what it refused; what it did.
    fn edit_hatch_patterns(&mut self, input: HatchPatternsEdit) -> Option<HatchPatternsEdited> {
        let doc = self.document.as_mut()?;
        match hatch_patterns_edit::execute(&mut ExecutionContext::new(&mut doc.model), input) {
            CommandResult::Completed { output, .. } => Some(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                if let Some(w) = self.hatch_patterns_window.as_mut() {
                    w.said = Some((Kind::Warn, error.message.clone()));
                }
                self.warn(error.message);
                None
            }
            _ => None,
        }
    }

    fn request(operation: HatchPatternsOperation) -> HatchPatternsEdit {
        HatchPatternsEdit {
            operation,
            pat: None,
            unit: None,
            source: None,
            names: None,
            replace: false,
            pattern: None,
            name: None,
            expected_revision: None,
        }
    }

    pub(crate) fn hatch_patterns_event(&mut self, event: Event) -> Task<Message> {
        if self.hatch_patterns_window.is_none() {
            return Task::none();
        }
        match event {
            Event::Choose(name) => {
                if let Some(w) = self.hatch_patterns_window.as_mut() {
                    w.chosen = Some(name);
                }
            }
            Event::Unit(u) => {
                if let Some(w) = self.hatch_patterns_window.as_mut() {
                    w.unit = u;
                }
            }
            Event::Load => {
                return Task::perform(
                    async move {
                        let file = rfd::AsyncFileDialog::new()
                            .set_title("Tarama desenlerini PAT dosyasından yükle")
                            .add_filter("Tarama deseni dosyası (.pat)", &["pat", "PAT"])
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
                let unit = self.hatch_patterns_window.as_ref().map(|w| w.unit);
                let input = HatchPatternsEdit {
                    pat: Some(text),
                    unit,
                    source: Some(name.clone()),
                    ..Self::request(HatchPatternsOperation::Load)
                };
                if let Some(done) = self.edit_hatch_patterns(input) {
                    let words = loaded_words(&name, &done);
                    let kind = if done.problems.is_empty() {
                        Kind::Ok
                    } else {
                        Kind::Warn
                    };
                    self.say(Level::Info, words.clone());
                    if let Some(w) = self.hatch_patterns_window.as_mut() {
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
                            .set_title("Tarama desenlerini dışa aktar")
                            .add_filter("Tarama deseni dosyası (.pat)", &["pat"])
                            .set_file_name(format!("{name}.pat"))
                            .save_file()
                            .await?;
                        Some(file.path().to_path_buf())
                    },
                    |path| msg(Event::Exported(path)),
                );
            }
            Event::Exported(Some(path)) => {
                let input = Self::request(HatchPatternsOperation::Export);
                if let Some(text) = self.edit_hatch_patterns(input).and_then(|d| d.pat) {
                    match std::fs::write(&path, text) {
                        Ok(()) => self.say(
                            Level::Success,
                            format!("Tarama desenleri {} dosyasına yazıldı.", path.display()),
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
                    .hatch_patterns_window
                    .as_ref()
                    .and_then(|w| w.chosen.clone())
                else {
                    return Task::none();
                };
                let input = HatchPatternsEdit {
                    name: Some(name.clone()),
                    ..Self::request(HatchPatternsOperation::Remove)
                };
                if self.edit_hatch_patterns(input).is_some()
                    && let Some(w) = self.hatch_patterns_window.as_mut()
                {
                    w.chosen = None;
                    w.said = Some((
                        Kind::Ok,
                        format!(
                            "“{name}” silindi; onunla çizilmiş taramalar kendi tanımlarını taşır, değişmez."
                        ),
                    ));
                }
            }
            Event::Close => {
                self.hatch_patterns_window = None;
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn hatch_patterns_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.hatch_patterns_window else {
            return text("").into();
        };
        let defs = self.hatch_patterns();
        let mut list = Column::new().spacing(1).padding(2);
        if defs.is_empty() {
            list = list.push(
                container(label::caption(
                    "Projede kendi tarama deseni yok. PAT'tan yükle… ile bir PAT dosyasının desenlerini alın; kitaplığın 28 deseni her zaman vardır.",
                ))
                .padding(10),
            );
        }
        for d in &defs {
            let families = format!("{} aile", d.lines.len());
            let meta = [
                unit_words(d.unit),
                d.source.as_deref().unwrap_or(""),
                families.as_str(),
            ]
            .iter()
            .filter(|s| !s.is_empty())
            .copied()
            .collect::<Vec<_>>()
            .join(" · ");
            let name = Elided::new(d.name.clone())
                .size(typography::body())
                .font(typography::ui())
                .width(typography::scaled(110.0));
            let description = column_of(d.description.clone(), meta);
            let picture = container(
                Canvas::new(Sample::of(d))
                    .width(Fill)
                    .height(typography::scaled(28.0)),
            )
            .width(typography::scaled(120.0));
            let chosen = w.chosen.as_deref() == Some(d.name.as_str());
            let face = row![name, description, picture]
                .spacing(10)
                .align_y(Center)
                .height(typography::scaled(ROW));
            list = list.push(
                button(face)
                    .on_press(msg(Event::Choose(d.name.clone())))
                    .padding([2, 8])
                    .width(Fill)
                    .style(style::button::row(chosen)),
            );
        }
        let table = container(iced::widget::scrollable(list).height(typography::scaled(240.0)))
            .style(style::container::field_box)
            .width(Fill);
        let unit = Segmented::new(
            [
                Unit(HatchPatternUnit::Mm),
                Unit(HatchPatternUnit::Inch),
                Unit(HatchPatternUnit::Drawing),
            ],
            Unit(w.unit),
            |u: Unit| msg(Event::Unit(u.0)),
        );
        let actions = row![
            words::secondary(LOAD, Some(msg(Event::Load))),
            words::secondary(EXPORT, (!defs.is_empty()).then(|| msg(Event::Export))),
            words::secondary(REMOVE, w.chosen.is_some().then(|| msg(Event::Remove))),
        ]
        .spacing(8);
        let mut body = Column::new()
            .spacing(12)
            .push(words::field(
                "Projenin tarama desenleri",
                table,
                Some("Tarama'nın Desen menüsünde kitaplığın desenlerinden sonra gelir; aynı adlı desen kitaplığınkinin yerine geçer.".to_owned()),
            ))
            .push(words::field(
                "PAT dosyasının birimi",
                Element::from(unit),
                Some("PAT dosyası birim söylemez: acadiso.pat mm, acad.pat inçtir; çizim birimiyle yazılmış dosya çizimin uzunluklarındadır.".to_owned()),
            ))
            .push(actions);
        if let Some((kind, said)) = &w.said {
            body = body.push(words::summary(vec![words::text_line(*kind, said.clone())]));
        }
        Frame2::new(HATCH_PATTERNS_TITLE)
            .push(body)
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(640.0)
            .modal(msg(Event::Close))
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn hatch_patterns_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.hatch_patterns_window.is_none() {
            return Err(format!("{HATCH_PATTERNS_TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(REMOVE) => Some(msg(Event::Remove)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            other => return Err(format!("“{HATCH_PATTERNS_TITLE}” penceresinde {other} yok")),
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
fn loaded_words(file: &str, done: &HatchPatternsEdited) -> String {
    let mut parts = Vec::new();
    if !done.added.is_empty() {
        parts.push(format!("{} desen eklendi", done.added.len()));
    }
    if !done.replaced.is_empty() {
        parts.push(format!("{} desen değişti", done.replaced.len()));
    }
    if !done.skipped.is_empty() {
        parts.push(format!(
            "{} desen projede vardı, kaldı ({})",
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
        parts.push("alınacak desen yok".to_owned());
    }
    format!("{file}: {}.", parts.join("; "))
}
