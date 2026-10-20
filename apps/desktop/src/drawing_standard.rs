//! Çizim standardı (docs/adr/0312 §7; AutoCAD's CAD Standards): a window
//! with the standard, what the drawing breaks of it (`cad.standards.check`,
//! read again after every write), the four parts of the fix and its preview
//! (`cad.standards.apply`'s plan). Uygula writes it: the layers as one undo
//! step “Standardı uygula”, the fonts and styles as project settings; the
//! findings are read again and the window stays.

use iced::widget::{Column, container, row, text};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, StandardIssue, StandardIssueKind, StandardsApplied, StandardsApply,
    StandardsCheck, StandardsReport, built_in_standard, built_in_standards,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, standards};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame, Elided, VirtualList, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const STANDARD_TITLE: &str = "Çizim standardı";
const APPLY: &str = "Uygula";
const CLOSE: &str = "Kapat";
const LAYERS: &str = "Eksik katmanları ekle";
const RESTYLE: &str = "Katmanların görünüşünü düzelt";
const RENAME: &str = "Eski katmanları birleştir";
const STYLES: &str = "Yazı tipi ve stiller";
const ROW: f32 = 26.0;

#[derive(Debug, Clone)]
pub struct Window {
    /// The built-in standard's id.
    pub standard: String,
    pub layers: bool,
    pub restyle: bool,
    pub rename: bool,
    pub styles: bool,
    report: Result<StandardsReport, String>,
    preview: Result<StandardsApplied, String>,
    /// The finding pressed: its whole message shown under the table.
    chosen: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Layers(bool),
    Restyle(bool),
    Rename(bool),
    Styles(bool),
    /// A finding's row pressed: its layer chosen in Katmanlar.
    Pick(usize),
    Apply,
    Close,
}

fn msg(event: Event) -> Message {
    Message::DrawingStandard(event)
}

/// A finding's kind in a few words.
fn kind_words(kind: StandardIssueKind) -> &'static str {
    use StandardIssueKind as K;
    match kind {
        K::MissingLayer => "Eksik katman",
        K::LayerName => "Katman adı",
        K::LayerColor => "Katman rengi",
        K::LayerLineType => "Çizgi tipi",
        K::LayerWeight => "Kalınlık",
        K::LayerPlot => "Yazdır",
        K::ProjectFont => "Yazı tipi",
        K::TextFont => "Stilin yazı tipi",
        K::MissingTextStyle => "Eksik yazı stili",
        K::TextStyleName => "Yazı stili",
        K::MissingDimensionStyle => "Eksik ölçü stili",
        K::DimensionStyleName => "Ölçü stili",
        K::ObjectLineType => "Nesne çizgi tipi",
        K::OwnLook => "Nesne görünüşü",
    }
}

impl Window {
    fn apply_input(&self) -> StandardsApply {
        StandardsApply {
            name: Some(self.standard.clone()),
            layers: self.layers,
            restyle: self.restyle,
            rename: self.rename,
            styles: self.styles,
            ..StandardsApply::default()
        }
    }
}

impl App {
    /// Çizim standardı (`standards.check`, `standards.apply`): the window on the first built-in standard.
    pub(crate) fn open_drawing_standard(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let standard = built_in_standards()
            .first()
            .map(|s| (*s).to_owned())
            .unwrap_or_default();
        self.drawing_standard = Some(Window {
            standard,
            layers: true,
            restyle: true,
            rename: true,
            styles: true,
            report: Err(String::new()),
            preview: Err(String::new()),
            chosen: None,
        });
        self.dialog = Some(Dialog::DrawingStandard);
        self.refresh_standard();
    }

    /// The findings and the preview read again.
    fn refresh_standard(&mut self) {
        let (Some(w), Some(doc)) = (&self.drawing_standard, &self.document) else {
            return;
        };
        let mut copy = doc.model.reading_copy();
        let report = match standards::plan_check(
            &ExecutionContext::new(&mut copy),
            &StandardsCheck {
                name: Some(w.standard.clone()),
                standard: None,
            },
        ) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err("Standart denetlenemedi.".to_owned()),
        };
        let preview = if let Some(locked) = self.tree_locked() {
            Err(locked.to_owned())
        } else {
            match standards::plan_apply(&ExecutionContext::new(&mut copy), &w.apply_input()) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Düzeltme planlanamadı.".to_owned()),
            }
        };
        if let Some(w) = self.drawing_standard.as_mut() {
            let count = report.as_ref().map_or(0, |r| r.issues.len());
            w.chosen = w.chosen.filter(|i| *i < count);
            w.report = report;
            w.preview = preview;
        }
    }

    pub(crate) fn drawing_standard_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.drawing_standard.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Layers(v) => w.layers = v,
            Event::Restyle(v) => w.restyle = v,
            Event::Rename(v) => w.rename = v,
            Event::Styles(v) => w.styles = v,
            Event::Pick(i) => {
                w.chosen = Some(i);
                let layer = w
                    .report
                    .as_ref()
                    .ok()
                    .and_then(|r| r.issues.get(i))
                    .and_then(|issue| issue.layer.clone());
                if let Some(id) = layer {
                    // Its row chosen in Katmanlar and scrolled into view.
                    self.selected_layer = Some(id.clone());
                    self.layers_follow = false;
                    self.layer_reveal = Some(id);
                }
                return Task::none();
            }
            Event::Apply => {
                self.apply_standard();
            }
            Event::Close => {
                self.drawing_standard = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_standard();
        Task::none()
    }

    /// Writes the window's fix; whether it wrote (what it did said).
    pub(crate) fn apply_standard(&mut self) -> bool {
        let Some(input) = self.drawing_standard.as_ref().map(Window::apply_input) else {
            return false;
        };
        if let Some(locked) = self.tree_locked() {
            self.warn(locked);
            return false;
        }
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let result = standards::execute_apply(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(done) = self.answered(result) else {
            return false;
        };
        if !done.changed {
            self.say(
                Level::Info,
                "Çizim standarda uyuyor; değişen olmadı.".to_owned(),
            );
            return true;
        }
        self.say(Level::Success, applied_words(&done));
        true
    }

    pub(crate) fn drawing_standard_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.drawing_standard else {
            return text("").into();
        };
        let standard = built_in_standard(&w.standard);
        let about = standard
            .as_ref()
            .map_or_else(String::new, |s| format!("{}: {}", s.name, s.description));
        let (lines, issues): (Vec<Element<'_, Message>>, Vec<StandardIssue>) = match &w.report {
            Err(why) if why.is_empty() => (Vec::new(), Vec::new()),
            Err(why) => (vec![words::text_line(Kind::Warn, why.clone())], Vec::new()),
            Ok(r) if r.issues.is_empty() => (
                vec![words::text_line(
                    Kind::Ok,
                    format!("Çizim {} standardına uyuyor.", r.standard),
                )],
                Vec::new(),
            ),
            Ok(r) => (
                vec![words::text_line(Kind::Info, report_words(r))],
                r.issues.clone(),
            ),
        };
        let chosen = w.chosen;
        let head = row![
            container(label::caption("Tür")).width(typography::scaled(120.0)),
            container(label::caption("Konu")).width(Fill),
            container(label::caption("Çizimde → standartta")).width(Fill),
            container(label::caption("")).width(typography::scaled(24.0)),
        ]
        .spacing(8)
        .padding([4, 8]);
        let detail = chosen
            .and_then(|i| issues.get(i))
            .map(|issue| issue.message.clone());
        let count = issues.len();
        let list = VirtualList::new(count, typography::scaled(ROW), move |i| {
            let issue = &issues[i];
            let mark: Element<'_, Message> = if issue.fixable {
                icon(Icon::Check).size(12.0).tone(Tone::Success).into()
            } else {
                icon(Icon::Warning).size(12.0).tone(Tone::Muted).into()
            };
            let cell = |t: String| {
                Elided::new(t)
                    .size(typography::caption())
                    .font(typography::ui())
                    .style(|theme: &iced::Theme| iced::widget::text::Style {
                        color: Some(Tokens::of(theme).text),
                    })
                    .width(Fill)
            };
            let face = row![
                container(label::caption(kind_words(issue.kind))).width(typography::scaled(120.0)),
                cell(issue.subject.clone()),
                cell(change_words(issue)),
                container(mark)
                    .width(typography::scaled(24.0))
                    .align_x(Center),
            ]
            .spacing(8)
            .padding([0, 8])
            .height(typography::scaled(ROW))
            .align_y(Center);
            iced::widget::button(face)
                .on_press(msg(Event::Pick(i)))
                .padding(0)
                .width(Fill)
                .style(style::button::row(chosen == Some(i)))
                .into()
        })
        .height(typography::scaled(156.0));
        let table = container(Column::new().push(head).push(list))
            .style(style::container::field_box)
            .width(Fill);
        let hint = detail.unwrap_or_else(|| {
            "Bulguya basınca tam iletisi burada görünür, katmanı Katmanlar'da seçilir. ✓ olanları Uygula düzeltir.".to_owned()
        });
        let parts = Column::new()
            .spacing(6)
            .push(
                row![
                    container(words::check(
                        w.rename,
                        RENAME,
                        Some(msg(Event::Rename(!w.rename)))
                    ))
                    .width(Fill),
                    container(words::check(
                        w.layers,
                        LAYERS,
                        Some(msg(Event::Layers(!w.layers)))
                    ))
                    .width(Fill),
                ]
                .spacing(12),
            )
            .push(
                row![
                    container(words::check(
                        w.restyle,
                        RESTYLE,
                        Some(msg(Event::Restyle(!w.restyle)))
                    ))
                    .width(Fill),
                    container(words::check(
                        w.styles,
                        STYLES,
                        Some(msg(Event::Styles(!w.styles)))
                    ))
                    .width(Fill),
                ]
                .spacing(12),
            );
        let (preview, ready) = match &w.preview {
            Err(why) if why.is_empty() => (Vec::new(), false),
            Err(why) => (vec![words::text_line(Kind::Warn, why.clone())], false),
            Ok(p) if !p.changed => (
                vec![words::text_line(
                    Kind::Info,
                    "Seçilen düzeltmelerle değişecek bir şey yok.".to_owned(),
                )],
                false,
            ),
            Ok(p) => (vec![words::text_line(Kind::Info, planned_words(p))], true),
        };
        let body = Column::new()
            .spacing(12)
            .push(words::field("Standart", label::body(about), None))
            .push(words::summary(lines))
            .push(words::field("Bulgular", table, Some(hint)))
            .push(words::field("Düzeltme", parts, None))
            .push(words::summary(preview));
        overlay::modal(
            Frame::new(STANDARD_TITLE)
                .push(body)
                .action(words::secondary(CLOSE, Some(msg(Event::Close))))
                .action(words::primary(APPLY, ready.then(|| msg(Event::Apply))))
                .width(680.0),
            msg(Event::Close),
        )
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn drawing_standard_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.drawing_standard else {
            return Err(format!("{STANDARD_TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(LAYERS) => Some(msg(Event::Layers(!w.layers))),
            Control::Press(RESTYLE) => Some(msg(Event::Restyle(!w.restyle))),
            Control::Press(RENAME) => Some(msg(Event::Rename(!w.rename))),
            Control::Press(STYLES) => Some(msg(Event::Styles(!w.styles))),
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            other => return Err(format!("“{STANDARD_TITLE}” penceresinde {other} yok")),
        })
    }
}

/// The report in a sentence.
fn report_words(r: &StandardsReport) -> String {
    let n = r.issues.len();
    match r.fixable as usize {
        0 => format!("{n} bulgu; hiçbiri Uygula ile düzelmez, elle düzeltin."),
        f if f == n => format!("{n} bulgu; hepsi Uygula ile düzelir."),
        f => format!(
            "{n} bulgu; {f} tanesi Uygula ile düzelir, {} tanesi elle.",
            n - f
        ),
    }
}

/// A finding's change in a few words: what the drawing has and what the standard asks, or how many objects.
fn change_words(issue: &StandardIssue) -> String {
    match (&issue.found, &issue.expected) {
        _ if issue.objects > 0 => format!("{} nesne", issue.objects),
        (Some(f), Some(e)) => format!("{f} → {e}"),
        (None, Some(e)) => format!("yok → {e}"),
        (Some(f), None) => format!("{f} → standartta yok"),
        (None, None) => String::new(),
    }
}

/// What the plan would do, in a sentence.
fn planned_words(p: &StandardsApplied) -> String {
    let mut parts = Vec::new();
    if !p.translations.is_empty() {
        parts.push(format!(
            "{} eski katman birleşir ({} nesne taşınır)",
            p.translations.len(),
            p.objects
        ));
    }
    if !p.created.is_empty() {
        parts.push(format!("{} katman eklenir", p.created.len()));
    }
    if !p.restyled.is_empty() {
        parts.push(format!("{} katmanın görünüşü düzelir", p.restyled.len()));
    }
    if p.font {
        parts.push("projenin yazı tipi değişir".to_owned());
    }
    if !p.text_styles.is_empty() || !p.dimension_styles.is_empty() {
        parts.push(format!(
            "{} stil eklenir ya da düzelir",
            p.text_styles.len() + p.dimension_styles.len()
        ));
    }
    format!(
        "Uygula: {}. Katmanlar tek adımda geri alınır.",
        parts.join(", ")
    )
}

/// What the write did, in a sentence.
fn applied_words(p: &StandardsApplied) -> String {
    let mut parts = Vec::new();
    if !p.translations.is_empty() {
        parts.push(format!("{} eski katman birleşti", p.translations.len()));
    }
    if !p.created.is_empty() {
        parts.push(format!("{} katman eklendi", p.created.len()));
    }
    if !p.restyled.is_empty() {
        parts.push(format!("{} katman düzeldi", p.restyled.len()));
    }
    if p.font || !p.text_styles.is_empty() || !p.dimension_styles.is_empty() {
        parts.push("yazı tipi ve stiller düzeldi".to_owned());
    }
    format!("Standart uygulandı: {}.", parts.join(", "))
}
