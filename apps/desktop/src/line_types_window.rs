//! Çizgi tipleri (docs/adr/0313 §7; AutoCAD's LINETYPE manager): the project's line types with patterns, each with its
//! name, unit, source and its pattern drawn as the drawing draws it (paper millimetres, a text or a shape as the room
//! it leaves), what it cannot draw said; LIN'den yükle… and Dışa aktar… (a LIN file), Sil, and the project's line type
//! scale. Previews share the drawing renderer and cached frozen glyphs (ADR 0930). Every write goes through `cad.linetypes.edit`: the table is a project setting, no undo step.

use crate::style::thumbs::Look;
use iced::widget::{Column, button, container, row, text};
use iced::{Center, Element, Fill, Task, Theme};
use kentos_commands::{
    CommandResult, LineTypeDef, LineTypesEdit, LineTypesEdited, LineTypesOperation, PatternUnit,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, line_types_edit};
use kentos_native_style::line_types::{LineTypes, dashes_of};
use kentos_native_style::preview::Geometry;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame2, Elided};
use kentos_ui::{label, style};
use serde_json::{Value, json};
use std::collections::HashMap;

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const LINE_TYPES_TITLE: &str = "Çizgi tipleri";
const LOAD: &str = "LIN'den yükle…";
const EXPORT: &str = "Dışa aktar…";
const REMOVE: &str = "Sil";
const IMPACT: &str = "Etkisi…";
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
    /// Shape once on opening/editing, never on pointer moves or redraws.
    previews: HashMap<String, Preview>,
}

#[derive(Debug, Clone)]
struct Preview {
    symbol: Value,
    period: f64,
    loss: Option<String>,
}
impl Preview {
    fn px_per_mm(&self, width: f32) -> f64 {
        let length = 3. * self.period.max(1e-6);
        let sample = if length / 0.76 < 50. {
            length / 0.76
        } else {
            length + 12.
        };
        f64::from(width) / sample
    }
}
fn previews(settings: &kentos_commands::ProjectSettings) -> HashMap<String, Preview> {
    let compiled = LineTypes::of(settings);
    let unit = settings
        .drawing_unit
        .unwrap_or(kentos_commands::DrawingUnit::M)
        .metres();
    settings.line_types.iter().map(|t| {
        let period = dashes_of(t, settings.plot_scale, unit).map_or_else(|| match t.unit {
            PatternUnit::Paper => t.period(),
            PatternUnit::Drawing => t.period() * unit.0 / unit.1 * 1000. / settings.plot_scale.max(1e-6),
        }, |d| d.dash.iter().sum()) * compiled.scale;
        let symbols = compiled.apply(Some(&t.name), json!({"line":{
            "type":"line","layers":[{"id":"linetype-preview","type":"simpleLine","color":"fg","width":0.,"unit":"mm","cap":"butt","join":"miter"}]
        }}));
        (t.name.clone(), Preview { symbol:symbols["line"].clone(), period, loss:compiled.loss(&t.name) })
    }).collect()
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
            previews: previews(doc.model.settings()),
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
        if let Some(w) = self.line_types_window.as_mut() {
            w.previews = previews(doc.model.settings());
        }
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
        let palette = self.style_palette();
        let look = Look {
            palette: &palette,
            library: &self.styles.library,
            images: &self.styles.images,
        };
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
            let preview = w.previews.get(&t.name);
            let unit_words = match t.unit {
                PatternUnit::Paper => "kâğıtta",
                PatternUnit::Drawing => "çizimde",
            };
            let note = preview.and_then(|p| p.loss.as_deref()).unwrap_or("");
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
            let size = (typography::scaled(180.), typography::scaled(24.));
            let picture: Element<'_, Message> = match preview {
                Some(p) if p.loss.is_none() => self.styles.thumbs.picture(
                    &p.symbol,
                    Some(Geometry::Line),
                    size,
                    Some(p.px_per_mm(size.0)),
                    &look,
                ),
                Some(_) => container(
                    row![
                        icon(Icon::Warning).size(14.).tone(Tone::Warning),
                        label::caption("Kaynak sorunu")
                    ]
                    .spacing(6)
                    .align_y(Center),
                )
                .width(size.0)
                .into(),
                None => container(text("Önizleme yok")).width(size.0).into(),
            };
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
        let table_height = ((typography::scaled(ROW) + 5.) * types.len().min(6) as f32 + 4.)
            .clamp(typography::scaled(84.), typography::scaled(220.));
        let table = container(iced::widget::scrollable(list).height(table_height))
            .style(style::container::field_box)
            .width(Fill);
        let actions = row![
            words::secondary(LOAD, Some(msg(Event::Load))),
            words::secondary(EXPORT, (!types.is_empty()).then(|| msg(Event::Export))),
            words::secondary(REMOVE, w.chosen.is_some().then(|| msg(Event::Remove))),
            // What a change of the chosen type would reach (docs/adr/0314 §4).
            words::secondary(
                IMPACT,
                w.chosen.clone().map(|name| {
                    Message::StyleImpact(crate::style_impact::Event::Open(
                        kentos_commands::ImpactTarget::LineType,
                        name,
                    ))
                }),
            ),
        ]
        .spacing(8);
        let scale = row![
            container(
                kentos_ui::widget::text_field("1", &w.scale)
                    .on_input(|t| msg(Event::Scale(t)))
                    .on_submit(msg(Event::ScaleSubmit))
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
                Some("Çizgi ve boşluk deseninin tamamı ekranda, paftada ve PDF'de kullanılır. Desteklenmeyen yazı/şekil işaretleri çıktı ön denetiminde bildirilir.".into()),
            ))
            .push(actions)
            .push(words::field(
                "Çizgi tipi ölçeği (LTSCALE)",
                scale,
                Some("Bütün desenler bu sayıyla çarpılır; nesnenin kendi ölçeği ayrıca çarpılır.".to_owned()),
            ));
        if let Some(preview) = w.chosen.as_ref().and_then(|name| w.previews.get(name)) {
            let size = (typography::scaled(536.), typography::scaled(64.));
            if preview.loss.is_none() {
                body = body.push(self.styles.thumbs.picture(
                    &preview.symbol,
                    Some(Geometry::Line),
                    size,
                    Some(preview.px_per_mm(size.0)),
                    &look,
                ));
            }
            if let Some(loss) = &preview.loss {
                body = body.push(words::summary(vec![words::text_line(
                    Kind::Warn,
                    format!("{loss} Çıktı ön denetiminde bildirilir."),
                )]));
            }
        }
        if let Some((kind, said)) = &w.said {
            body = body.push(words::summary(vec![words::text_line(*kind, said.clone())]));
        }
        Frame2::new(LINE_TYPES_TITLE)
            .push(body)
            .action(words::primary(CLOSE, Some(msg(Event::Close))))
            .width(640.0)
            .modal(msg(Event::Close))
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

#[cfg(test)]
mod preview_tests {
    use super::*;
    #[test]
    fn preview_uses_resolved_text_and_resource_specific_loss() {
        let settings:kentos_commands::ProjectSettings=serde_json::from_value(json!({
            "srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"deg","plotScale":50.,
            "lineTypes":[
                {"name":"HW","unit":"paper","elements":[{"length":5.},{"length":-4.},{"length":0.,"text":{"text":"HW","style":"STANDARD","scale":1.5}},{"length":-4.}]},
                {"name":"SHX","unit":"paper","elements":[{"length":5.},{"length":-4.},{"length":0.,"shape":{"name":"FENCE","file":"missing.shx"}},{"length":-4.}]}
            ]
        })).unwrap();
        let got = previews(&settings);
        let text = &got["HW"];
        assert!(text.loss.is_none());
        assert_eq!(text.period, 13.);
        assert_eq!(text.symbol["layers"][0]["width"], 0.);
        assert_eq!(
            text.symbol["layers"][0]["cadMarks"]["marks"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
        assert!(got["SHX"].loss.as_ref().unwrap().contains("missing.shx"));
        assert!(got["SHX"].symbol["layers"][0]["lineTypeLoss"].is_string());
        let rendered = kentos_native_style::preview::preview(
            &text.symbol,
            Some(Geometry::Line),
            (180., 24.),
            Some(text.px_per_mm(180.)),
            &kentos_native_style::StylePalette::graphite(),
            &kentos_native_style::library::StyleLibrary::default(),
        )
        .unwrap();
        assert!(
            rendered
                .layer
                .batches
                .iter()
                .any(|b| matches!(b.kind, kentos_native_style::batches::BatchKind::Fill { .. }))
        );
    }
}
