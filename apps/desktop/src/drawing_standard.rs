//! Çizim standardı (docs/adr/0312 §7; AutoCAD's CAD Standards): a window
//! with the standard, what the drawing breaks of it (`cad.standards.check`,
//! read again after every write), the four parts of the fix and its preview
//! (`cad.standards.apply`'s plan). Uygula writes it: the layers as one undo
//! step “Standardı uygula”, the fonts and styles as project settings; the
//! findings are read again and the window stays.

use iced::widget::{Column, container, row, text};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, DrawingStandard, ProjectStandardDocument, SettingsSet, StandardIssue,
    StandardIssueKind, StandardsApplied, StandardsApply, StandardsCheck, StandardsConflicts,
    StandardsConflictsReport, StandardsReport, built_in_standard, built_in_standards,
    parse_standard_document,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, settings_set, standards};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame, Elided, VirtualList};
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
    /// A temporary file standard; absent means the built-in standard.
    custom: Option<SelectedStandard>,
    choices: Vec<ProjectChoice>,
    load_error: Option<String>,
    /// Unreadable embedded documents remain selectable for safe removal.
    invalid_bound: Option<(String, String)>,
    confirm_remove: bool,
    pub layers: bool,
    pub restyle: bool,
    pub rename: bool,
    pub styles: bool,
    report: Result<StandardsReport, String>,
    preview: Result<StandardsApplied, String>,
    conflicts: Result<StandardsConflictsReport, String>,
    /// The finding pressed: its whole message shown under the table.
    chosen: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Load,
    Loaded(Result<Option<SelectedStandard>, String>),
    BuiltIn,
    Project(String),
    Attach,
    Move(i8),
    RemoveAsk,
    RemoveCancel,
    RemoveConfirm,
    Layers(bool),
    Restyle(bool),
    Rename(bool),
    Styles(bool),
    /// A finding's row pressed: its layer chosen in Katmanlar.
    Pick(usize),
    Apply,
    Close,
}

#[derive(Debug, Clone)]
pub struct SelectedStandard {
    label: String,
    standard: DrawingStandard,
    /// Exact source text is retained only for a versioned document.
    document: Option<String>,
    bound_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ProjectChoice {
    id: String,
    label: String,
}

impl std::fmt::Display for ProjectChoice {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.label)
    }
}

/// Parse once per report refresh, not on every rendered frame.
fn project_choices(documents: &[ProjectStandardDocument]) -> Vec<ProjectChoice> {
    documents
        .iter()
        .filter_map(|document| {
            let value: serde_json::Value = serde_json::from_str(&document.text).ok()?;
            let id = value.get("id")?.as_str()?.to_owned();
            let revision = value.get("revision")?.as_str()?;
            let invalid = parse_standard_document(document.text.as_bytes()).is_err();
            Some(ProjectChoice {
                label: format!(
                    "{id} · {revision}{}",
                    if invalid { " · geçersiz" } else { "" }
                ),
                id,
            })
        })
        .collect()
}

fn selected_project(document: &ProjectStandardDocument) -> Result<SelectedStandard, String> {
    let (standard, package) = parse_standard_document(document.text.as_bytes())?;
    let package = package.ok_or("Bağlı standart sürümlü değil.")?;
    Ok(SelectedStandard {
        label: format!("Projeye bağlı: {} · sürüm {}", package.id, package.revision),
        standard,
        document: Some(document.text.clone()),
        bound_id: Some(package.id),
    })
}

fn invalid_identity(document: &ProjectStandardDocument) -> Option<(String, String)> {
    let value: serde_json::Value = serde_json::from_str(&document.text).ok()?;
    Some((value.get("id")?.as_str()?.to_owned(), document.text.clone()))
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
        K::TextStyleHeight => "Yazı yüksekliği",
        K::MissingDimensionStyle => "Eksik ölçü stili",
        K::DimensionStyleName => "Ölçü stili",
        K::DimensionStyleHeight => "Ölçü yazısı yüksekliği",
        K::ObjectLineType => "Nesne çizgi tipi",
        K::OwnLook => "Nesne görünüşü",
    }
}

impl Window {
    fn apply_input(&self) -> StandardsApply {
        StandardsApply {
            name: self.custom.is_none().then(|| self.standard.clone()),
            standard: self
                .custom
                .as_ref()
                .map(|selected| selected.standard.clone()),
            layers: self.layers,
            restyle: self.restyle,
            rename: self.rename,
            styles: self.styles,
            expected_revision: self
                .report
                .as_ref()
                .ok()
                .map(|report| report.revision.clone()),
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
        let first = self
            .document
            .as_ref()
            .and_then(|doc| doc.model.settings().standard_docs.first());
        let (custom, load_error, invalid_bound) = match first {
            None => (None, None, None),
            Some(document) => match selected_project(document) {
                Ok(selected) => (Some(selected), None, None),
                Err(why) => (
                    None,
                    Some(format!("İlk bağlı standart geçersiz: {why}")),
                    invalid_identity(document),
                ),
            },
        };
        self.drawing_standard = Some(Window {
            standard,
            custom,
            choices: Vec::new(),
            load_error,
            invalid_bound,
            confirm_remove: false,
            layers: true,
            restyle: true,
            rename: true,
            styles: true,
            report: Err(String::new()),
            preview: Err(String::new()),
            conflicts: Err(String::new()),
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
        let choices = project_choices(&doc.model.settings().standard_docs);
        let bound_changed =
            w.custom.as_ref().is_some_and(|selected| {
                selected.bound_id.is_some()
                    && !doc.model.settings().standard_docs.iter().any(|document| {
                        selected.document.as_deref() == Some(document.text.as_str())
                    })
            });
        let mut copy = doc.model.reading_copy();
        let conflicts = match standards::plan_conflicts(
            &ExecutionContext::new(&mut copy),
            &StandardsConflicts {},
        ) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err("Standartlar karşılaştırılamadı.".into()),
        };
        let report = match standards::plan_check(
            &ExecutionContext::new(&mut copy),
            &StandardsCheck {
                name: w.custom.is_none().then(|| w.standard.clone()),
                standard: w.custom.as_ref().map(|selected| selected.standard.clone()),
            },
        ) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err("Standart denetlenemedi.".to_owned()),
        };
        let preview = if report.is_err() {
            Err("Standart denetimi tamamlanmadan düzeltme planlanamaz.".to_owned())
        } else if let Some(locked) = self.tree_locked() {
            Err(locked.to_owned())
        } else {
            let mut input = w.apply_input();
            input.expected_revision = report.as_ref().ok().map(|r| r.revision.clone());
            match standards::plan_apply(&ExecutionContext::new(&mut copy), &input) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Düzeltme planlanamadı.".to_owned()),
            }
        };
        if let Some(w) = self.drawing_standard.as_mut() {
            w.choices = choices;
            w.conflicts = conflicts;
            if bound_changed {
                w.load_error =
                    Some("Bağlı standart değişti veya kaldırıldı; projeden yeniden seçin.".into());
            }
            // An index belongs to the old report; after a fix the same index
            // can describe a different finding even when it remains in range.
            let same_findings = match (&w.report, &report) {
                (Ok(previous), Ok(next)) => previous.issues == next.issues,
                _ => false,
            };
            if !same_findings {
                w.chosen = None;
            }
            w.report = report;
            w.preview = preview;
        }
    }

    pub(crate) fn drawing_standard_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.drawing_standard.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Load => {
                return Task::perform(
                    async {
                        let Some(file) = rfd::AsyncFileDialog::new()
                            .set_title("Çizim standardı seç")
                            .add_filter("Standart tanımı (.json)", &["json"])
                            .pick_file()
                            .await
                        else {
                            return Ok(None);
                        };
                        use std::io::Read;
                        let mut bytes = Vec::new();
                        std::fs::File::open(file.path())
                            .map_err(|e| e.to_string())?
                            .take(1_048_577)
                            .read_to_end(&mut bytes)
                            .map_err(|e| e.to_string())?;
                        if bytes.len() > 1_048_576 {
                            return Err("Standart dosyası 1 MiB sınırını aşıyor.".into());
                        }
                        let (standard, package) = parse_standard_document(&bytes)?;
                        let name = match &package {
                            Some(package) => format!(
                                "{} · {} · sürüm {}",
                                file.file_name(),
                                package.id,
                                package.revision
                            ),
                            None => file.file_name(),
                        };
                        let document = if package.is_some() {
                            Some(String::from_utf8(bytes).map_err(|e| e.to_string())?)
                        } else {
                            None
                        };
                        Ok(Some(SelectedStandard {
                            label: name,
                            standard,
                            document,
                            bound_id: None,
                        }))
                    },
                    |result| msg(Event::Loaded(result)),
                );
            }
            Event::Loaded(Ok(None)) => return Task::none(),
            Event::Loaded(Err(error)) => {
                w.load_error = Some(error);
                return Task::none();
            }
            Event::Loaded(Ok(Some(standard))) => {
                w.custom = Some(standard);
                w.load_error = None;
                w.invalid_bound = None;
                w.chosen = None;
                w.confirm_remove = false;
            }
            Event::BuiltIn => {
                w.custom = None;
                w.load_error = None;
                w.invalid_bound = None;
                w.chosen = None;
                w.confirm_remove = false;
            }
            Event::Project(id) => {
                w.confirm_remove = false;
                let selected = self.document.as_ref().and_then(|doc| {
                    doc.model
                        .settings()
                        .standard_docs
                        .iter()
                        .find(|document| {
                            serde_json::from_str::<serde_json::Value>(&document.text)
                                .ok()
                                .and_then(|value| {
                                    value.get("id").and_then(|v| v.as_str()).map(str::to_owned)
                                })
                                .as_deref()
                                == Some(id.as_str())
                        })
                        .cloned()
                });
                match selected {
                    Some(document) => match selected_project(&document) {
                        Ok(selected) => {
                            w.custom = Some(selected);
                            w.load_error = None;
                            w.invalid_bound = None;
                            w.chosen = None;
                        }
                        Err(why) => {
                            w.custom = None;
                            w.invalid_bound = invalid_identity(&document);
                            w.load_error = Some(format!("Bağlı standart geçersiz: {why}"));
                        }
                    },
                    None => {
                        w.invalid_bound = None;
                        w.load_error = Some("Bağlı standart artık bulunamıyor.".into());
                    }
                }
            }
            Event::Attach => {
                self.attach_standard();
            }
            Event::Move(direction) => self.edit_bound_standard(Some(direction)),
            Event::RemoveAsk => w.confirm_remove = true,
            Event::RemoveCancel => w.confirm_remove = false,
            Event::RemoveConfirm => self.edit_bound_standard(None),
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

    /// Associates the selected versioned file without applying its rules.
    fn attach_standard(&mut self) {
        let Some(window) = self.drawing_standard.as_ref() else {
            return;
        };
        let Some(selected) = window.custom.as_ref() else {
            return;
        };
        let Some(text) = selected.document.clone() else {
            return;
        };
        if selected.bound_id.is_some() || window.load_error.is_some() {
            return;
        }
        let Ok((_, Some(package))) = parse_standard_document(text.as_bytes()) else {
            return;
        };
        let Some(revision) = window
            .report
            .as_ref()
            .ok()
            .map(|report| report.revision.clone())
        else {
            return;
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let mut documents = doc.model.settings().standard_docs.clone();
        if documents.iter().any(|existing| {
            parse_standard_document(existing.text.as_bytes())
                .ok()
                .and_then(|(_, package)| package)
                .is_some_and(|existing| existing.id == package.id)
        }) {
            self.warn(format!(
                "“{}” kimliği projeye zaten bağlı; önce bağlı standardı inceleyin.",
                package.id
            ));
            return;
        }
        documents.push(ProjectStandardDocument { text });
        let result = settings_set::execute(
            &mut ExecutionContext::new(&mut doc.model),
            SettingsSet {
                standard_docs: Some(documents),
                expected_revision: Some(revision),
                ..SettingsSet::default()
            },
        );
        if self.answered(result).is_some() {
            if let Some(selected) = self
                .drawing_standard
                .as_mut()
                .and_then(|w| w.custom.as_mut())
            {
                selected.label =
                    format!("Projeye bağlı: {} · sürüm {}", package.id, package.revision);
                selected.bound_id = Some(package.id);
            }
            self.say(
                Level::Success,
                "Standart projeye bağlandı; çizimi kaydedin.".to_owned(),
            );
        }
    }

    /// Reorder or explicitly remove the selected embedded document through
    /// the same revision-checked command as Python and MCP.
    fn edit_bound_standard(&mut self, direction: Option<i8>) {
        let Some(window) = self.drawing_standard.as_ref() else {
            return;
        };
        if (window.load_error.is_some() && (direction.is_some() || window.invalid_bound.is_none()))
            || (direction.is_none() && !window.confirm_remove)
        {
            return;
        }
        let text = window
            .invalid_bound
            .as_ref()
            .map(|(_, text)| text.as_str())
            .or_else(|| {
                window
                    .custom
                    .as_ref()
                    .filter(|selected| selected.bound_id.is_some())
                    .and_then(|selected| selected.document.as_deref())
            });
        let Some(text) = text else {
            return;
        };
        let Some(revision) = window
            .report
            .as_ref()
            .ok()
            .map(|report| report.revision.clone())
        else {
            return;
        };
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let mut documents = doc.model.settings().standard_docs.clone();
        let Some(index) = documents.iter().position(|entry| entry.text == text) else {
            return;
        };
        if let Some(direction) = direction {
            let other = match direction {
                -1 if index > 0 => index - 1,
                1 if index + 1 < documents.len() => index + 1,
                _ => return,
            };
            documents.swap(index, other);
        } else {
            documents.remove(index);
        }
        let next = direction.is_none().then(|| documents.first().cloned());
        let result = settings_set::execute(
            &mut ExecutionContext::new(&mut doc.model),
            SettingsSet {
                standard_docs: Some(documents),
                expected_revision: Some(revision),
                ..SettingsSet::default()
            },
        );
        if self.answered(result).is_some() {
            if let Some(w) = self.drawing_standard.as_mut() {
                w.confirm_remove = false;
                if let Some(next) = next {
                    w.invalid_bound = None;
                    match next.as_ref().map(selected_project) {
                        Some(Ok(selected)) => {
                            w.custom = Some(selected);
                            w.load_error = None;
                        }
                        Some(Err(why)) => {
                            w.custom = None;
                            w.invalid_bound = next.as_ref().and_then(invalid_identity);
                            w.load_error = Some(format!("İlk bağlı standart geçersiz: {why}"));
                        }
                        None => {
                            w.custom = None;
                            w.load_error = None;
                        }
                    }
                }
            }
            self.say(
                Level::Success,
                if direction.is_some() {
                    "Standart sırası değiştirildi; çizimi kaydedin."
                } else {
                    "Standart proje listesinden çıkarıldı; çizimi kaydedin."
                }
                .to_owned(),
            );
        }
    }

    /// Writes the window's fix; whether it wrote (what it did said).
    pub(crate) fn apply_standard(&mut self) -> bool {
        if self
            .drawing_standard
            .as_ref()
            .is_some_and(|w| w.load_error.is_some() || w.report.is_err())
        {
            return false;
        }
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
        let standard = w
            .custom
            .as_ref()
            .map(|selected| selected.standard.clone())
            .or_else(|| built_in_standard(&w.standard));
        let mut about = standard.as_ref().map_or_else(String::new, |s| {
            if s.description.trim().is_empty() {
                s.name.clone()
            } else {
                format!("{}: {}", s.name, s.description)
            }
        });
        if w.invalid_bound.is_some() {
            about = "Geçersiz bağlı standart".into();
        }
        let (lines, issues): (Vec<Element<'_, Message>>, Vec<StandardIssue>) = if w
            .invalid_bound
            .is_some()
        {
            (vec![words::text_line(Kind::Warn, "Bu standart denetlenemiyor; projeden çıkarabilir veya başka bir standart seçebilirsiniz.".to_owned())], Vec::new())
        } else {
            match &w.report {
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
            }
        };
        // Keep the command report order intact; show findings needing a person
        // first while retaining their original indices for selection and details.
        let mut issues: Vec<_> = issues.into_iter().enumerate().collect();
        issues.sort_by_key(|(_, issue)| issue.fixable);
        let chosen = w.chosen;
        let head = kentos_ui::widget::table::band(
            row![
                container(kentos_ui::widget::table::title("Tür")).width(typography::scaled(120.0)),
                container(kentos_ui::widget::table::title("Konu")).width(Fill),
                container(kentos_ui::widget::table::title("Çizimde → standartta")).width(Fill),
                container(kentos_ui::widget::table::title("")).width(typography::scaled(24.0)),
            ]
            .spacing(8)
            .padding([0, 8]),
        );
        let detail = chosen
            .and_then(|i| issues.iter().find(|(original, _)| *original == i))
            .map(|(_, issue)| issue.message.clone());
        let count = issues.len();
        let list = VirtualList::new(count, typography::scaled(ROW), move |i| {
            let (original, issue) = &issues[i];
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
                .on_press(msg(Event::Pick(*original)))
                .padding(0)
                .width(Fill)
                .style(style::button::row(chosen == Some(*original)))
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
        let (preview, ready) = if w.invalid_bound.is_some() {
            (Vec::new(), false)
        } else {
            match &w.preview {
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
            }
        };
        let source = if let Some((id, _)) = &w.invalid_bound {
            format!("Projeye bağlı: {id} · geçersiz")
        } else {
            w.custom.as_ref().map_or_else(
                || "Yerleşik standart".to_owned(),
                |selected| {
                    if selected.bound_id.is_some() {
                        selected.label.clone()
                    } else {
                        format!("Geçici: {}", selected.label)
                    }
                },
            )
        };
        let choices = w.choices.clone();
        let current = w
            .invalid_bound
            .as_ref()
            .map(|(id, _)| id)
            .or_else(|| {
                w.custom
                    .as_ref()
                    .and_then(|selected| selected.bound_id.as_ref())
            })
            .and_then(|id| choices.iter().find(|choice| &choice.id == id))
            .cloned();
        let position = current
            .as_ref()
            .and_then(|selected| choices.iter().position(|choice| choice.id == selected.id));
        let count = choices.len();
        let editable = w.load_error.is_none() && w.report.is_ok();
        let removable = w.report.is_ok() && (editable || w.invalid_bound.is_some());
        let attach = w.custom.as_ref().is_some_and(|selected| {
            selected.document.is_some()
                && selected.bound_id.is_none()
                && w.load_error.is_none()
                && w.report.is_ok()
        });
        let mut body = Column::new()
            .spacing(12)
            .push(words::field("Standart", label::body(about), None))
            .push(
                row![
                    words::secondary("Dosyadan seç…", Some(msg(Event::Load))),
                    words::secondary(
                        "Yerleşik standarda dön",
                        (w.custom.is_some() || w.load_error.is_some()).then(|| msg(Event::BuiltIn))
                    ),
                    Elided::new(source)
                        .size(typography::caption())
                        .font(typography::ui())
                        .width(Fill),
                ]
                .spacing(8)
                .align_y(Center),
            )
            .push(words::field(
                "Projeye bağlı standartlar",
                row![
                    iced::widget::pick_list(choices, current, |choice: ProjectChoice| msg(
                        Event::Project(choice.id)
                    ))
                    .placeholder("Bağlı standart seçin")
                    .style(style::field::pick_list)
                    .menu_style(style::field::menu)
                    .width(Fill),
                    words::secondary("Projeye bağla", attach.then(|| msg(Event::Attach))),
                ]
                .spacing(8)
                .align_y(Center),
                Some(
                    "İlk standart pencere açılınca seçilir. Denetim yalnız seçili standardı inceler; bağlamak kuralları uygulamaz."
                        .into(),
                ),
            ));
        if let Some(index) = position {
            body = if w.confirm_remove {
                body.push(words::text_line(
                    Kind::Warn,
                    "Bu standart çizimden çıkarılacak; kaynak dosya değiştirilmez.".to_owned(),
                ))
                .push(
                    row![
                        words::secondary("Vazgeç", Some(msg(Event::RemoveCancel))),
                        words::secondary(
                            "Çıkarmayı onayla",
                            removable.then(|| msg(Event::RemoveConfirm))
                        ),
                    ]
                    .spacing(8),
                )
            } else {
                body.push(
                    row![
                        words::secondary(
                            "Yukarı taşı",
                            (editable && index > 0).then(|| msg(Event::Move(-1)))
                        ),
                        words::secondary(
                            "Aşağı taşı",
                            (editable && index + 1 < count).then(|| msg(Event::Move(1)))
                        ),
                        words::secondary(
                            "Projeden çıkar…",
                            removable.then(|| msg(Event::RemoveAsk))
                        ),
                    ]
                    .spacing(8),
                )
            };
        }
        if count > 1 {
            match &w.conflicts {
                Err(_) => body = body.push(words::text_line(Kind::Warn,
                    "Bağlı standartların tamamı okunamadığı için farklı tanımlar karşılaştırılamıyor.")),
                Ok(report) if !report.conflicts.is_empty() => {
                    let conflicts = report.conflicts.clone();
                    let count = conflicts.len();
                    let list = VirtualList::new(count, typography::scaled(ROW), move |i| {
                        let item = &conflicts[i];
                        let kind = match item.kind.as_str() {
                            "layer" => "Katman", "textStyle" => "Yazı stili", "dimensionStyle" => "Ölçü stili", _ => "Tanım",
                        };
                        let fields = item.fields.iter().map(|field| match field.as_str() {
                            "color" => "renk", "lineType" => "çizgi tipi", "lineWeight" => "kalınlık", "plot" => "yazdırma", "from" => "kaynak eşlemesi", "font" => "yazı tipi", "height" => "yükseklik", _ => "özellik",
                        }).collect::<Vec<_>>().join(", ");
                        let line = format!("{kind} · {}: {} ↔ {} · {fields}", item.name, item.first_id, item.later_id);
                        container(Elided::new(line).size(typography::caption()).font(typography::ui()).width(Fill))
                            .padding([0, 8]).height(typography::scaled(ROW)).into()
                    }).height(typography::scaled(104.0));
                    body = body.push(words::field(
                        "Aynı adlı farklı tanımlar",
                        container(list).style(style::container::field_box).width(Fill),
                        Some(format!("{count} fark. Yalnız katman, yazı ve ölçü stili tanımları karşılaştırılır; denetim seçili standardı kullanır.")),
                    ));
                }
                _ => {}
            }
        }
        body = body
            .push(words::summary(lines))
            .push(words::field("Bulgular", table, Some(hint)))
            .push(words::field("Düzeltme", parts, None))
            .push(words::summary(preview));
        if let Some(error) = &w.load_error
            && w.invalid_bound.is_none()
        {
            body = body.push(words::text_line(Kind::Warn, error.clone()));
        }
        if w.custom
            .as_ref()
            .is_some_and(|selected| selected.bound_id.is_none())
        {
            body = body.push(label::caption(
                "Bu dosya seçimi pencere kapanınca unutulur; çizime kaydedilmez.",
            ));
        }
        Frame::new(STANDARD_TITLE)
            .scroll(body)
            .action(words::secondary(CLOSE, Some(msg(Event::Close))))
            .action(words::primary(
                APPLY,
                (ready && w.load_error.is_none()).then(|| msg(Event::Apply)),
            ))
            .width(680.0)
            .modal(msg(Event::Close))
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

#[cfg(test)]
mod tests {
    #[test]
    fn project_standards_can_be_reordered_and_removed_only_after_confirmation() {
        use super::Event;
        use kentos_commands::{
            ProjectStandardDocument, STANDARD_PACKAGE_FORMAT, STANDARD_PACKAGE_VERSION,
            StandardPackage, built_in_standard,
        };
        let mut app = crate::selecting::tests::objects();
        crate::files_testing::make_cad(&mut app);
        let item = |id: &str| ProjectStandardDocument {
            text: serde_json::to_string(&StandardPackage {
                format: STANDARD_PACKAGE_FORMAT.into(),
                version: STANDARD_PACKAGE_VERSION,
                id: id.into(),
                revision: "1".into(),
                standard: built_in_standard("mimari-ofis").unwrap(),
            })
            .unwrap(),
        };
        let doc = &mut app.document.as_mut().unwrap().model;
        let mut settings = doc.settings().clone();
        settings.standard_docs = vec![item("a"), item("b")];
        doc.set_settings(settings);
        app.open_drawing_standard();
        let doc = &mut app.document.as_mut().unwrap().model;
        let layer = doc.layers().active().to_owned();
        let line = serde_json::from_value(serde_json::json!({"kind":"line","id":0,"layerId":layer,
            "attrs":{},"a":{"x":0,"y":0},"b":{"x":1,"y":0}}))
        .unwrap();
        doc.add(line).unwrap();
        let changed = doc.to_snapshot_v2();
        // A stale preview cannot reorder a document changed by another action.
        let _ = app.drawing_standard_event(Event::Move(1));
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            changed
        );
        // The failed attempt refreshes the report; the explicit next request succeeds.
        let _ = app.drawing_standard_event(Event::Move(1));
        let docs = &app
            .document
            .as_ref()
            .unwrap()
            .model
            .settings()
            .standard_docs;
        let ids: Vec<String> = docs
            .iter()
            .map(|d| {
                serde_json::from_str::<serde_json::Value>(&d.text).unwrap()["id"]
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(ids, ["b", "a"]);
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.drawing_standard_event(Event::RemoveConfirm);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        let _ = app.drawing_standard_event(Event::RemoveAsk);
        let _ = app.drawing_standard_event(Event::RemoveCancel);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        let _ = app.drawing_standard_event(Event::RemoveAsk);
        let _ = app.drawing_standard_event(Event::RemoveConfirm);
        let after = app.document.as_ref().unwrap().model.to_snapshot_v2();
        assert_eq!(after.settings.standard_docs.len(), 1);
        assert_eq!(after.entities, before.entities);
        assert_eq!(after.layers, before.layers);
        assert_eq!(
            app.drawing_standard
                .as_ref()
                .unwrap()
                .custom
                .as_ref()
                .unwrap()
                .bound_id
                .as_deref(),
            Some("b")
        );
    }

    #[test]
    fn invalid_embedded_standard_can_be_removed_but_not_applied() {
        use super::Event;
        use kentos_commands::ProjectStandardDocument;
        let mut app = crate::selecting::tests::objects();
        crate::files_testing::make_cad(&mut app);
        let document = &mut app.document.as_mut().unwrap().model;
        let mut settings = document.settings().clone();
        let invalid = ProjectStandardDocument {
            text: serde_json::json!({"format":"kentos.drawing-standard", "version":1,
                "id":"bad", "revision":"1", "standard":{"unsupported_rule":true}})
            .to_string(),
        };
        let invalid_second = ProjectStandardDocument {
            text: invalid.text.replace("\"bad\"", "\"bad-second\""),
        };
        settings.standard_docs = vec![invalid, invalid_second.clone()];
        document.set_settings(settings);
        app.open_drawing_standard();
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        assert!(app.drawing_standard.as_ref().unwrap().load_error.is_some());
        assert!(!app.apply_standard());
        let _ = app.drawing_standard_event(Event::RemoveConfirm);
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            before
        );
        let _ = app.drawing_standard_event(Event::RemoveAsk);
        let _ = app.drawing_standard_event(Event::RemoveConfirm);
        let after = app.document.as_ref().unwrap().model.to_snapshot_v2();
        assert_eq!(after.settings.standard_docs[0].text, invalid_second.text);
        assert_eq!(after.entities, before.entities);
        assert_eq!(after.layers, before.layers);
        assert!(app.drawing_standard.as_ref().unwrap().load_error.is_some());
        let _ = app.drawing_standard_event(Event::RemoveAsk);
        let _ = app.drawing_standard_event(Event::RemoveConfirm);
        assert!(
            app.document
                .as_ref()
                .unwrap()
                .model
                .settings()
                .standard_docs
                .is_empty()
        );
    }

    #[test]
    fn versioned_standard_attaches_and_selects_without_applying_changes() {
        use super::{Event, SelectedStandard};
        use kentos_commands::{
            STANDARD_PACKAGE_FORMAT, STANDARD_PACKAGE_VERSION, StandardPackage, built_in_standard,
        };
        let mut app = crate::selecting::tests::objects();
        crate::files_testing::make_cad(&mut app);
        app.open_drawing_standard();
        let standard = built_in_standard("mimari-ofis").unwrap();
        let package = StandardPackage {
            format: STANDARD_PACKAGE_FORMAT.into(),
            version: STANDARD_PACKAGE_VERSION,
            id: "ofis".into(),
            revision: "1".into(),
            standard: standard.clone(),
        };
        let text = serde_json::to_string(&package).unwrap();
        let before = app.document.as_ref().unwrap().model.to_snapshot_v2();
        let _ = app.drawing_standard_event(Event::Loaded(Ok(Some(SelectedStandard {
            label: "ofis.json".into(),
            standard,
            document: Some(text.clone()),
            bound_id: None,
        }))));
        let _ = app.drawing_standard_event(Event::Attach);
        let after = app.document.as_ref().unwrap().model.to_snapshot_v2();
        assert_eq!(after.entities, before.entities);
        assert_eq!(after.layers, before.layers);
        assert_eq!(after.settings.standard_docs[0].text, text);
        let _ = app.drawing_standard_event(Event::BuiltIn);
        let _ = app.drawing_standard_event(Event::Loaded(Ok(Some(SelectedStandard {
            label: "aynı.json".into(),
            standard: package.standard.clone(),
            document: Some(text),
            bound_id: None,
        }))));
        let _ = app.drawing_standard_event(Event::Attach);
        assert_eq!(
            app.document
                .as_ref()
                .unwrap()
                .model
                .settings()
                .standard_docs
                .len(),
            1
        );
        let _ = app.drawing_standard_event(Event::Close);
        app.open_drawing_standard();
        assert_eq!(
            app.drawing_standard
                .as_ref()
                .unwrap()
                .custom
                .as_ref()
                .unwrap()
                .bound_id
                .as_deref(),
            Some("ofis")
        );
        let _ = app.drawing_standard_event(Event::Project("ofis".into()));
        assert_eq!(
            app.drawing_standard
                .as_ref()
                .unwrap()
                .custom
                .as_ref()
                .unwrap()
                .bound_id
                .as_deref(),
            Some("ofis")
        );
        // A removed binding cannot remain silently selected and be applied.
        let doc = &mut app.document.as_mut().unwrap().model;
        let mut settings = doc.settings().clone();
        settings.standard_docs.clear();
        doc.set_settings(settings);
        app.refresh_standard();
        assert!(app.drawing_standard.as_ref().unwrap().load_error.is_some());
        assert!(!app.apply_standard());
    }

    #[test]
    fn stale_standard_preview_cannot_apply_to_changed_document() {
        let mut app = crate::selecting::tests::objects();
        crate::files_testing::make_cad(&mut app);
        app.open_drawing_standard();
        let revision = app
            .drawing_standard
            .as_ref()
            .unwrap()
            .report
            .as_ref()
            .unwrap()
            .revision
            .clone();
        let model = &mut app.document.as_mut().unwrap().model;
        let layer = model.layers().active().to_owned();
        let line = serde_json::from_value(serde_json::json!({"kind":"line","id":0,"layerId":layer,
            "attrs":{},"a":{"x":0,"y":0},"b":{"x":1,"y":0}}))
        .unwrap();
        model.add(line).unwrap();
        assert_ne!(model.revision().to_string(), revision);
        let changed = model.to_snapshot_v2();
        assert!(!app.apply_standard());
        assert_eq!(
            app.document.as_ref().unwrap().model.to_snapshot_v2(),
            changed
        );
        app.refresh_standard();
        assert_eq!(
            app.drawing_standard
                .as_ref()
                .unwrap()
                .apply_input()
                .expected_revision,
            Some(app.document.as_ref().unwrap().model.revision().to_string())
        );
    }
}
