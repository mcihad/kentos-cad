//! Kodla çiz (docs/adr/1360; `map.featuresCode`): coded survey points through a data profile's classes, by
//! `map.features.code`. The window is the preview: the command's plan says, class by class, how many points go on the
//! class's layer and how many lines and areas are made, and which codes no class has; a class without its layer offers
//! Profili uygula (`cad.profiles.apply`); Uygula writes one undo step “Kodla çiz”. The selected points, in their order,
//! when some are selected; else every coded point on a shown, unlocked layer.

use iced::widget::{Column, row};
use iced::{Center, Element, Task};
use kentos_commands::{CommandResult, FeaturesCode, FeaturesCoded, ProfilesApply, ProfilesList};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, map_features, profile_files, profiles};
use kentos_ui::label;
use kentos_ui::widget::Dialog as Frame;

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

pub const TITLE: &str = "Kodla çiz";
const APPLY: &str = "Uygula";
const CANCEL: &str = "Vazgeç";
const APPLY_PROFILE: &str = "Profili uygula";
/// The profile the window starts with: Harita's field-coded assets.
const FIRST_PROFILE: &str = "harita-varliklari";

#[derive(Clone, Debug, Default)]
pub struct State {
    pub profile: String,
    pub field: String,
    /// The selected points' lasting ids, in the selection's order; none: every coded point.
    pub uids: Option<Vec<String>>,
    /// The profiles to choose from: name and title.
    pub profiles: Vec<(String, String)>,
    /// The command's plan and its warnings, or its refusal (code, words).
    pub plan: Option<Result<(FeaturesCoded, Vec<String>), (String, String)>>,
}

#[derive(Clone, Debug)]
pub enum Event {
    Profile(String),
    Field(String),
    ApplyProfile,
    Apply,
    Close,
}

fn msg(e: Event) -> Message {
    Message::FeaturesCode(e)
}

impl App {
    fn features_input(&self) -> FeaturesCode {
        let field = self.features_code.field.trim();
        FeaturesCode {
            name: self.features_code.profile.clone(),
            drawing: self.features_drawing(),
            field: (!field.is_empty() && field != kentos_commands::DEFAULT_CODE_FIELD).then(|| field.to_owned()),
            uids: self.features_code.uids.clone(),
            expected_revision: None,
        }
    }

    /// The open drawing's file, absolute: the profiles beside it are found.
    fn features_drawing(&self) -> Option<String> {
        self.document.as_ref()?.path.as_ref().filter(|p| p.is_absolute()).map(|p| p.display().to_string())
    }

    fn features_plan(&mut self) {
        let input = self.features_input();
        let Some(doc) = self.document.as_mut() else { return };
        self.features_code.plan = Some(match map_features::plan(&ExecutionContext::new(&mut doc.model), &input) {
            CommandResult::Completed { output, warnings } => {
                Ok((output, warnings.into_iter().map(|w| w.message).collect()))
            }
            CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                Err((error.code, error.message))
            }
            _ => Err((String::new(), "Önizleme alınamadı.".to_owned())),
        });
    }

    /// Kodla çiz (`map.featuresCode`): the profiles listed, the selected points (or all coded ones), previewed.
    pub(crate) fn open_features_code(&mut self) -> Task<Message> {
        let Some(doc) = self.document.as_mut() else {
            self.output("Açık çizim yok.");
            return Task::none();
        };
        let points: Vec<String> = self
            .selection
            .ids()
            .iter()
            .filter(|s| matches!(doc.model.get(**s), Some(kentos_commands::Entity::Point(_))))
            .filter_map(|s| doc.model.uid(*s))
            .map(|u| u.to_string())
            .collect();
        let drawing = doc.path.as_ref().filter(|p| p.is_absolute()).map(|p| p.display().to_string());
        let listed = match profile_files::list::plan(
            &ExecutionContext::new(&mut doc.model),
            &ProfilesList { drawing, ..Default::default() },
        ) {
            CommandResult::Completed { output, .. } => output.items,
            _ => Vec::new(),
        };
        let profiles: Vec<(String, String)> = listed
            .into_iter()
            .filter(|i| i.kind == kentos_commands::ProfileFileKind::Profile && i.problem.is_none())
            .map(|i| (i.name.clone(), if i.title.is_empty() { i.name } else { i.title }))
            .collect();
        self.features_code = State {
            profile: FIRST_PROFILE.to_owned(),
            field: kentos_commands::DEFAULT_CODE_FIELD.to_owned(),
            uids: (!points.is_empty()).then_some(points),
            profiles,
            plan: None,
        };
        self.features_plan();
        self.dialog = Some(Dialog::FeaturesCode);
        Task::none()
    }

    pub(crate) fn features_code_event(&mut self, event: Event) -> Task<Message> {
        match event {
            Event::Profile(name) => {
                self.features_code.profile = name;
                self.features_plan();
            }
            Event::Field(f) => {
                self.features_code.field = f;
                self.features_plan();
            }
            Event::ApplyProfile => {
                let input = ProfilesApply {
                    name: Some(self.features_code.profile.clone()),
                    profile: None,
                    mapping: None,
                    mapping_name: None,
                    drawing: self.features_drawing(),
                    expected_revision: None,
                };
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                match profiles::apply::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { .. } => {
                        self.say(Level::Success, format!("“{}” profili uygulandı: sınıfların katmanları eklendi.", self.features_code.profile));
                    }
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message));
                    }
                    _ => {}
                }
                self.features_plan();
            }
            Event::Apply => {
                let input = self.features_input();
                let Some(doc) = self.document.as_mut() else { return Task::none() };
                match map_features::execute(&mut ExecutionContext::new(&mut doc.model), input) {
                    CommandResult::Completed { output, warnings } => {
                        let moved: u32 = output.classes.iter().map(|c| c.moved).sum();
                        let lines: u32 = output.classes.iter().map(|c| c.lines).sum();
                        let areas: u32 = output.classes.iter().map(|c| c.areas).sum();
                        self.say(
                            Level::Success,
                            format!("Kodla çiz: {moved} nokta sınıfının katmanına, {lines} çizgi, {areas} alan (tek adımda geri alınır)."),
                        );
                        for w in warnings {
                            self.warn(format!("{TITLE}: {}", w.message));
                        }
                        self.features_code = State::default();
                        self.dialog = None;
                    }
                    CommandResult::Failed { error } | CommandResult::Conflict { error } | CommandResult::NeedsInput { error } => {
                        self.warn(format!("{TITLE}: {}", error.message));
                    }
                    _ => {}
                }
            }
            Event::Close => {
                self.features_code = State::default();
                self.dialog = None;
            }
        }
        Task::none()
    }

    pub(crate) fn features_code_view(&self) -> Element<'_, Message> {
        let s = &self.features_code;
        let choices: Vec<(String, String)> = s.profiles.clone();
        let pick = crate::style::fields::select_owned(choices, &s.profile, |n| msg(Event::Profile(n)));
        let field = crate::style::fields::input(None, "Kod", &s.field, false, false).on_input(|f| msg(Event::Field(f)));
        let head = row![
            label::body("Profil"),
            iced::widget::container(pick).width(260),
            label::body("Kod alanı"),
            iced::widget::container(field).width(110),
        ]
        .spacing(10)
        .align_y(Center);
        let scope = match &s.uids {
            Some(u) => format!("Seçili {} nokta, seçim sırasıyla.", u.len()),
            None => "Görünen ve kilitsiz katmanlardaki bütün kodlu noktalar, çizimin sırasıyla.".to_owned(),
        };
        let mut lines = vec![words::text_line(
            Kind::Info,
            "Kodun ardındaki rakam dizinin numarasıdır (BRD1, BRD2); B yeni dizi başlatır, K kapatır, S bitirir. Noktalar yerinde kalır.",
        )];
        let mut missing_layer = false;
        match &s.plan {
            Some(Ok((p, warnings))) => {
                for c in &p.classes {
                    let mut w = format!("{}: {} nokta", c.title, c.points);
                    if c.moved > 0 {
                        w.push_str(&format!(", {} katmanına taşınacak", c.moved));
                    }
                    if c.lines > 0 {
                        w.push_str(&format!(" → {} çizgi", c.lines));
                    }
                    if c.areas > 0 {
                        w.push_str(&format!(" → {} alan", c.areas));
                    }
                    lines.push(words::text_line(Kind::Ok, w));
                }
                for w in warnings {
                    lines.push(words::text_line(Kind::Warn, w.clone()));
                }
            }
            Some(Err((code, why))) => {
                missing_layer = code == "class_layer_missing";
                lines.push(words::text_line(Kind::Error, why.clone()));
            }
            None => lines.push(words::text_line(Kind::Info, "Önizleme yok.")),
        }
        let body = Column::new().spacing(12).push(head).push(label::caption(scope)).push(words::summary(lines));
        let ready = matches!(s.plan, Some(Ok((ref p, _))) if p.classes.iter().any(|c| c.moved + c.lines + c.areas > 0));
        let mut frame = Frame::new(TITLE).push(body).aside(words::ghost(CANCEL, Some(msg(Event::Close))));
        if missing_layer {
            frame = frame.aside(words::ghost(APPLY_PROFILE, Some(msg(Event::ApplyProfile))));
        }
        frame.action(words::primary(APPLY, ready.then(|| msg(Event::Apply)))).width(620.0).modal(msg(Event::Close))
    }

    pub(crate) fn features_code_control(&self, control: Control<'_>) -> Result<Option<Message>, String> {
        Ok(match control {
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(APPLY_PROFILE) => Some(msg(Event::ApplyProfile)),
            Control::Press(CANCEL) => Some(msg(Event::Close)),
            other => return Err(format!("“{TITLE}” penceresinde {other} yok")),
        })
    }
}
