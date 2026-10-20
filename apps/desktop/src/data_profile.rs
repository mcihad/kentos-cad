//! Veri profili (docs/adr/0541; plan/gis.md CBS-34): a window with a data profile, where the drawing's data departs from
//! it (`cad.profiles.check`, read again after every write) and what Uygula would add (`cad.profiles.apply`'s plan).
//! Uygula writes it: the layers and fields as one undo step “Veri profili”, the value lists, relations and address data
//! as project settings; the findings are read again and the window stays. A finding's row selects its objects.

use iced::widget::{Column, container, row, text};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    BUILTIN_PROFILES, CommandResult, ProfileFinding, ProfileFindingKind, ProfilesApplied,
    ProfilesApply, ProfilesCheck, ProfilesReport, builtin_profile,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, profiles};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Dialog as Frame, Elided, VirtualList, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const PROFILE_TITLE: &str = "Veri profili";
const APPLY: &str = "Uygula";
const CLOSE: &str = "Kapat";
const ROW: f32 = 26.0;

#[derive(Debug, Clone)]
pub struct Window {
    /// The built-in profile's name.
    pub profile: String,
    report: Result<ProfilesReport, String>,
    preview: Result<ProfilesApplied, String>,
    /// The finding pressed: its whole message shown under the table.
    chosen: Option<usize>,
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A finding's row pressed: its objects selected and shown.
    Pick(usize),
    Apply,
    Close,
}

fn msg(event: Event) -> Message {
    Message::DataProfile(event)
}

/// A finding's kind in a few words.
fn kind_words(kind: ProfileFindingKind) -> &'static str {
    use ProfileFindingKind as K;
    match kind {
        K::MissingLayer => "Eksik katman",
        K::MissingField => "Eksik alan",
        K::FieldKind => "Alan türü",
        K::MissingDomain => "Eksik değer listesi",
        K::MissingRelation => "Eksik ilişki",
        K::MissingAddress => "Adres verisi",
        K::WrongGeometry => "Geometri",
        K::RequiredEmpty => "Boş zorunlu alan",
        K::BadValue => "Kurala uymayan değer",
        K::DuplicateKey => "Yinelenen kimlik",
        K::AreaDifference => "Alan farkı",
        K::RelationBroken => "Kopuk bağ",
    }
}

impl App {
    /// Veri profili (`profile.window`): the window on the first built-in profile.
    pub(crate) fn open_data_profile(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let profile = BUILTIN_PROFILES
            .first()
            .map(|(n, _)| (*n).to_owned())
            .unwrap_or_default();
        self.data_profile = Some(Window {
            profile,
            report: Err(String::new()),
            preview: Err(String::new()),
            chosen: None,
        });
        self.dialog = Some(Dialog::DataProfile);
        self.refresh_profile();
    }

    /// The findings and the preview read again.
    fn refresh_profile(&mut self) {
        let (Some(w), Some(doc)) = (&self.data_profile, &self.document) else {
            return;
        };
        let report = match profiles::check::run(
            &doc.model,
            &ProfilesCheck {
                name: Some(w.profile.clone()),
                profile: None,
            },
        ) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err("Profil denetlenemedi.".to_owned()),
        };
        let preview = if let Some(locked) = self.tree_locked() {
            Err(locked.to_owned())
        } else {
            match profiles::apply::plan_in(&doc.model, &apply_input(&w.profile)) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Profil planlanamadı.".to_owned()),
            }
        };
        if let Some(w) = self.data_profile.as_mut() {
            let count = report.as_ref().map_or(0, |r| r.findings.len());
            w.chosen = w.chosen.filter(|i| *i < count);
            w.report = report;
            w.preview = preview;
        }
    }

    pub(crate) fn data_profile_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.data_profile.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Pick(i) => {
                w.chosen = Some(i);
                let finding = w.report.as_ref().ok().and_then(|r| r.findings.get(i)).cloned();
                if let Some(f) = finding {
                    self.show_finding(&f);
                }
                return Task::none();
            }
            Event::Apply => {
                self.apply_profile();
            }
            Event::Close => {
                self.data_profile = None;
                self.dialog = None;
                return Task::none();
            }
        }
        self.refresh_profile();
        Task::none()
    }

    /// A finding's objects selected and brought into view; one without objects chooses its layer in Katmanlar.
    fn show_finding(&mut self, f: &ProfileFinding) {
        let slots: Vec<_> = self.document.as_ref().map_or_else(Vec::new, |d| {
            f.uids
                .iter()
                .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                .filter_map(|u| d.model.slot_of(u))
                .collect()
        });
        if !slots.is_empty() {
            let ids: Vec<f64> = slots.iter().map(|s| f64::from(s.0)).collect();
            self.selection.set(slots);
            self.zoom_to_objects(&ids);
        } else if let Some(id) = f.layer.clone() {
            self.selected_layer = Some(id.clone());
            self.layers_follow = false;
            self.layer_reveal = Some(id);
        }
    }

    /// Writes the profile's missing parts; whether it wrote (what it did said).
    pub(crate) fn apply_profile(&mut self) -> bool {
        let Some(input) = self.data_profile.as_ref().map(|w| apply_input(&w.profile)) else {
            return false;
        };
        if let Some(locked) = self.tree_locked() {
            self.warn(locked);
            return false;
        }
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let result = profiles::apply::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(done) = self.answered(result) else {
            return false;
        };
        if !done.changed {
            self.say(
                Level::Info,
                "Çizimde profilin bütün parçaları var; değişen olmadı.".to_owned(),
            );
            return true;
        }
        self.say(Level::Success, applied_words(&done));
        true
    }

    pub(crate) fn data_profile_view(&self) -> Element<'_, Message> {
        let Some(w) = &self.data_profile else {
            return text("").into();
        };
        let profile = builtin_profile(&w.profile);
        let about = profile
            .as_ref()
            .map_or_else(String::new, |p| format!("{} (sürüm {}): {}", p.title, p.version, p.description));
        let basis = profile.as_ref().map_or_else(String::new, |p| p.basis.clone());
        let (lines, findings): (Vec<Element<'_, Message>>, Vec<ProfileFinding>) = match &w.report {
            Err(why) if why.is_empty() => (Vec::new(), Vec::new()),
            Err(why) => (vec![words::text_line(Kind::Warn, why.clone())], Vec::new()),
            Ok(r) if r.clean => (
                vec![words::text_line(
                    Kind::Ok,
                    format!("Çizimin verisi {} ile ayrılmıyor (bu, mevzuata uygunluk değildir).", r.title),
                )],
                Vec::new(),
            ),
            Ok(r) => (
                vec![words::text_line(Kind::Info, report_words(r))],
                r.findings.clone(),
            ),
        };
        let class_title = |id: &Option<String>| {
            id.as_deref()
                .and_then(|c| profile.as_ref()?.classes.iter().find(|x| x.id == c))
                .map_or_else(String::new, |c| c.title.clone())
        };
        let classes: Vec<String> = findings.iter().map(|f| class_title(&f.class)).collect();
        let chosen = w.chosen;
        let head = row![
            container(label::caption("Tür")).width(typography::scaled(150.0)),
            container(label::caption("Sınıf")).width(typography::scaled(120.0)),
            container(label::caption("Konu")).width(Fill),
            container(label::caption("Nesne")).width(typography::scaled(56.0)),
            container(label::caption("")).width(typography::scaled(24.0)),
        ]
        .spacing(8)
        .padding([4, 8]);
        let detail = chosen
            .and_then(|i| findings.get(i))
            .map(|f| f.message.clone());
        let count = findings.len();
        let list = VirtualList::new(count, typography::scaled(ROW), move |i| {
            let f = &findings[i];
            let mark: Element<'_, Message> = if f.fixable {
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
            let subject = match (&f.subject, &f.value) {
                (Some(s), Some(v)) => format!("{s}: {v}"),
                (Some(s), None) => s.clone(),
                (None, _) => String::new(),
            };
            let objects = if f.count > 0 { f.count.to_string() } else { String::new() };
            let face = row![
                container(label::caption(kind_words(f.kind))).width(typography::scaled(150.0)),
                container(cell(classes[i].clone())).width(typography::scaled(120.0)),
                cell(subject),
                container(label::caption(objects)).width(typography::scaled(56.0)),
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
        .height(typography::scaled(208.0));
        let table = container(Column::new().push(head).push(list))
            .style(style::container::field_box)
            .width(Fill);
        let hint = detail.unwrap_or_else(|| {
            "Bulguya basınca tam iletisi burada görünür, nesneleri seçilip gösterilir. ✓ olanları Uygula ekler; öbürleri verinin kendisinde düzeltilir.".to_owned()
        });
        let (preview, ready) = match &w.preview {
            Err(why) if why.is_empty() => (Vec::new(), false),
            Err(why) => (vec![words::text_line(Kind::Warn, why.clone())], false),
            Ok(p) if !p.changed => (
                vec![words::text_line(
                    Kind::Info,
                    "Uygula'nın ekleyeceği bir şey yok.".to_owned(),
                )],
                false,
            ),
            Ok(p) => (vec![words::text_line(Kind::Info, planned_words(p))], true),
        };
        let body = Column::new()
            .spacing(12)
            .push(words::field("Profil", label::body(about), Some(basis)))
            .push(words::summary(lines))
            .push(words::field("Bulgular", table, Some(hint)))
            .push(words::summary(preview));
        overlay::modal(
            Frame::new(PROFILE_TITLE)
                .push(body)
                .action(words::secondary(CLOSE, Some(msg(Event::Close))))
                .action(words::primary(APPLY, ready.then(|| msg(Event::Apply))))
                .width(760.0),
            msg(Event::Close),
        )
    }

    /// The first finding of a kind, by its place in the window's table (the pictures press it).
    #[cfg(test)]
    pub(crate) fn data_profile_finding(&self, kind: &str) -> Option<usize> {
        let r = self.data_profile.as_ref()?.report.as_ref().ok()?;
        r.findings.iter().position(|f| {
            serde_json::to_value(f.kind).ok().and_then(|v| v.as_str().map(str::to_owned)).as_deref() == Some(kind)
        })
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn data_profile_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        if self.data_profile.is_none() {
            return Err(format!("{PROFILE_TITLE} penceresi açık değil"));
        }
        Ok(match control {
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            other => return Err(format!("“{PROFILE_TITLE}” penceresinde {other} yok")),
        })
    }
}

fn apply_input(name: &str) -> ProfilesApply {
    ProfilesApply {
        name: Some(name.to_owned()),
        profile: None,
        expected_revision: None,
    }
}

/// The report in a sentence.
fn report_words(r: &ProfilesReport) -> String {
    let n = r.findings.len();
    let fixable = r.findings.iter().filter(|f| f.fixable).count();
    let objects: u32 = r.classes.iter().map(|c| c.objects).sum();
    let head = format!("{} sınıfta {objects} nesne; {n} bulgu", r.classes.len());
    match fixable {
        0 => format!("{head}; hiçbiri Uygula ile düzelmez, verinin kendisinde düzeltin."),
        f if f == n => format!("{head}; hepsini Uygula ekler."),
        f => format!("{head}; {f} tanesini Uygula ekler, {} tanesi veride düzeltilir.", n - f),
    }
}

/// What the plan would do, in a sentence.
fn planned_words(p: &ProfilesApplied) -> String {
    let mut parts = Vec::new();
    if !p.layers.is_empty() {
        parts.push(format!("{} katman ve grup eklenir", p.layers.len()));
    }
    if !p.fields.is_empty() {
        parts.push(format!("var olan katmanlara {} alan eklenir", p.fields.len()));
    }
    if !p.domains.is_empty() {
        parts.push(format!("{} değer listesi", p.domains.len()));
    }
    if !p.relations.is_empty() {
        parts.push(format!("{} ilişki", p.relations.len()));
    }
    if p.address {
        parts.push("adres verisi".to_owned());
    }
    format!(
        "Uygula: {}. Katmanlar ve alanları tek adımda geri alınır.",
        parts.join(", ")
    )
}

/// What the write did, in a sentence.
fn applied_words(p: &ProfilesApplied) -> String {
    let mut parts = Vec::new();
    if !p.layers.is_empty() {
        parts.push(format!("{} katman ve grup eklendi", p.layers.len()));
    }
    if !p.fields.is_empty() {
        parts.push(format!("{} alan eklendi", p.fields.len()));
    }
    if !p.domains.is_empty() || !p.relations.is_empty() || p.address {
        parts.push("değer listeleri, ilişkiler ve adres verisi yazıldı".to_owned());
    }
    format!("Veri profili uygulandı: {}.", parts.join(", "))
}
