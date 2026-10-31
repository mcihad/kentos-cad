//! Dosya → Proje ayarları (the web's `ui/settings/ProjectSettingsDialog.ts`):
//! stored in the project file, shared by everyone who opens it. The window
//! works on a draft: Kaydet assigns it (an edit, not an undo step), Vazgeç
//! leaves the drawing as it was. A new coordinate system is assigned, never
//! a transformation of the coordinates (CLAUDE.md §5).

use std::fmt;

use iced::widget::{Column, button, column, container, row, scrollable, text};
use iced::{Element, Fill, Task};
use kentos_commands::{
    AngleUnit, AreaUnit, CrsDefinition, DrawingFont, DrawingUnit, ProjectSettings, Workspace,
};
use kentos_interaction::{Format, Level};
use kentos_ui::theme::typography;
use kentos_ui::widget::number::NumberInput;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Dialog, overlay};
use kentos_ui::{label, style};

use kentos_project::{survey_form, variable_form};

use super::custom_crs::{self, Outcome, Target};
use super::{Event as ProjectEvent, Window, group, message, modes, setting};
use crate::app::{App, Message};
use crate::crs;
use crate::document::Document;
use crate::exchange::words;

/// The window's sections, as the web's navigation lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    Crs,
    Heights,
    /// Birim ve ölçek (docs/adr/0626 §4): the drawing's unit and decimals, its drawing scale and the
    /// annotations' heights on paper (docs/adr/0205 §1; scale.rs), the areas' and angles' units.
    Units,
    /// Ölçme: the survey settings (docs/adr/0169 §3; survey.rs).
    Survey,
    /// Değişkenler: the project's `@` values (docs/adr/0214 §4; variables.rs).
    Variables,
    /// Mimari: the levels and the walls' detail (docs/adr/0339 §3; arch.rs).
    Arch,
}

impl Section {
    /// Every section in the list's order; a module's program shows its own of them (module_surfaces.rs).
    pub(crate) const ALL: [Section; 7] = [
        Section::General,
        Section::Units,
        Section::Crs,
        Section::Heights,
        Section::Survey,
        Section::Variables,
        Section::Arch,
    ];

    fn label(self) -> &'static str {
        match self {
            Section::General => "Genel",
            Section::Crs => "Koordinat sistemi",
            Section::Heights => "Kotlar ve epok",
            Section::Units => "Birim ve ölçek",
            Section::Survey => "Ölçme",
            Section::Variables => "Değişkenler",
            Section::Arch => "Mimari",
        }
    }

    /// Its icon in the list, as the web's.
    fn icon(self) -> kentos_ui::icon::Icon {
        use kentos_ui::icon::Icon;
        match self {
            Section::General => Icon::Properties,
            Section::Crs => Icon::Globe,
            Section::Heights => crate::icons::from_web(Some("setElevation")),
            Section::Units => crate::icons::from_web(Some("plotScale")),
            Section::Survey => crate::icons::from_web(Some("surveyPolar")),
            Section::Variables => crate::icons::from_web(Some("projectVariables")),
            Section::Arch => crate::icons::from_web(Some("archSettings")),
        }
    }

    fn lead(self) -> &'static str {
        match self {
            Section::General => "Projenin adı, türü, seçimi ve çizimdeki yazıların yazı tipi.",
            Section::Heights => {
                "Kotların ne olduğu (elipsoit ya da ortometrik), düşey sistem, geoit modeli ve koordinat epoku; dönüşümün kaydı."
            }
            Section::Crs => {
                "Bu projenin konum referansı. Koordinatlar bu sistemde saklanır, ölçülür ve dışa aktarılır."
            }
            Section::Units => {
                "Uzunlukların, alanların ve açıların nasıl yazılıp gösterileceği; çizim ölçeği ve yazıların kâğıttaki boyları. Değişen ölçeğe genel boydaki yazılar ve ölçüler uyar, elle değiştirilenler kalır."
            }
            Section::Survey => {
                "Saha ölçülerinin indirgenmesindeki katsayı, zemin değerlerinin yüksekliği ve karnenin denetlendiği toleranslar."
            }
            Section::Arch => {
                "Katlar, mahal ad/kullanım listeleri, numara önekleri ve duvar ayrıntısı. Bu tercihler projeyle saklanır; mevcut mahal adları ve numaraları korunur."
            }
            Section::Variables => {
                "Projenin ifadelerde @ad olarak okunan değerleri: iş numarası, idare, katsayı, teslim tarihi … Projeyle saklanır, projeyi açan herkes aynısını okur."
            }
        }
    }
}

/// The drawing typefaces a project may name (the settings schema's list).
pub(super) const FONTS: [(DrawingFont, &str); 7] = [
    (DrawingFont::Barlow, "Barlow"),
    (DrawingFont::Arimo, "Arimo"),
    (DrawingFont::Overpass, "Overpass"),
    (DrawingFont::Quicksand, "Quicksand"),
    (DrawingFont::ArchitectsDaughter, "Architects Daughter"),
    (DrawingFont::CourierPrime, "Courier Prime"),
    (DrawingFont::PlexMono, "IBM Plex Mono"),
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Area(AreaUnit);

impl fmt::Display for Area {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            AreaUnit::M2 => "m²",
            AreaUnit::Donum => "Dönüm",
            AreaUnit::Ha => "Hektar",
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Angle(AngleUnit);

/// A drawing unit as the segmented control writes it (docs/adr/0165 §2).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Unit(DrawingUnit);

impl fmt::Display for Unit {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0.mark())
    }
}

/// A length style as the segmented control writes it (docs/adr/0262 §2).
#[derive(Debug, Clone, Copy, PartialEq)]
struct Style(kentos_commands::LengthStyle);

impl fmt::Display for Style {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        use kentos_commands::LengthStyle as L;
        f.write_str(match self.0 {
            L::Decimal => "Ondalık",
            L::Architectural => "Mimari",
            L::Engineering => "Mühendislik",
            L::Fractional => "Kesirli",
        })
    }
}

impl fmt::Display for Angle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            AngleUnit::Grad => "Grad",
            AngleUnit::Deg => "Derece",
        })
    }
}

#[derive(Debug)]
pub struct State {
    /// The page shown; Koordinat sistemi… opens on `Crs`.
    pub(super) section: Section,
    name: String,
    settings: ProjectSettings,
    initial_name: String,
    initial: ProjectSettings,
    query: String,
    /// The grid whose Kaldır asks in its row (Izgaralar).
    removing: Option<String>,
    /// Datum dönüşümleri's forms as typed (docs/adr/0168 §3).
    choices: super::choices::Choices,
    /// Özel koordinat sistemi over this window (custom_crs.rs).
    pub(super) custom: Option<custom_crs::Editor>,
    /// The project's own definition and its second's (docs/adr/0168 §1),
    /// kept while another system is chosen: their rows stay in the lists
    /// until Kaydet, which keeps only the chosen.
    defined: Option<CrsDefinition>,
    second_defined: Option<CrsDefinition>,
    /// Ölçme's texts as typed (k, the six tolerances and the ground height;
    /// survey.rs).
    survey: [String; survey_form::FIELDS],
    /// Whether the survey windows reduce lengths to the grid (docs/adr/0171
    /// §4): beside the texts, so a height typed wrong for a while does not
    /// lose it.
    reduce: bool,
    /// Ölçek ve yazılar' scale denominator as typed (docs/adr/0205 §4).
    scale_typed: String,
    /// Değişkenler' rows as typed (docs/adr/0214 §4; variables.rs).
    variables: Vec<variable_form::Row>,
    /// Mimari's levels as typed and the walls' detail (docs/adr/0339 §3; arch.rs); Kaydet writes them through
    /// `cad.settings.set`.
    arch: Vec<super::arch::Row>,
    arch_detail: Option<kentos_commands::WallDetail>,
    /// Mimari: the spaces' measurement method (docs/adr/0342 §1).
    arch_method: Option<kentos_commands::SpaceMethod>,
    /// Kotlar ve epok as typed (docs/adr/0519 §5; heights.rs); Kaydet writes them through `cad.settings.set`.
    heights: super::heights::Texts,
    /// Kaydet's question when a local project's unit changes with a drawing
    /// in it (docs/adr/0256 §2; unit_change.rs), over this window.
    pub(super) unit_question: Option<super::unit_change::Question>,
}

#[derive(Debug, Clone)]
pub enum Event {
    Section(Section),
    Name(String),
    Scale(f64),
    /// The scale's denominator typed; one that holds is the draft's.
    ScaleTyped(String),
    /// A kind's height on paper, mm (docs/adr/0205 §1).
    Height(kentos_commands::AnnotationKind, f64),
    /// Varsayılanlara dön: every kind its default.
    ResetHeights,
    Mode(Workspace),
    Font(DrawingFont),
    /// Otomatik seçim (docs/adr/0622 §5): any shown layer's objects selected, or the active layer's alone.
    AutoSelect(bool),
    Crs(u32),
    Search(String),
    LengthDecimals(u32),
    AreaDecimals(u32),
    AreaUnit(AreaUnit),
    AngleUnit(AngleUnit),
    /// A local project's drawing unit (docs/adr/0165 §2); metres are kept as none.
    DrawingUnit(DrawingUnit),
    /// How lengths are written in an inch or foot drawing (docs/adr/0262 §2); decimal is kept as none.
    LengthStyle(kentos_commands::LengthStyle),
    /// İkinci koordinat sistemi (docs/adr/0167 §1); none: Yok. Either takes
    /// the place of a second definition (docs/adr/0168 §1).
    Second(Option<u32>),
    /// The second definition, chosen again.
    SecondDefined,
    /// The project's own definition, chosen again.
    Defined,
    /// Özel sistem…: a new definition of the project's own system or of its second.
    NewDefinition(Target),
    /// Düzenle: the definition there is.
    EditDefinition(Target),
    /// Özel koordinat sistemi's fields and buttons.
    Custom(custom_crs::Event),
    /// Kaldır on a grid's row: asked in the row; none: Vazgeç.
    GridAsk(Option<String>),
    /// Datum dönüşümleri: a pair's form changed.
    Choice(usize, super::choices::Edit),
    /// Izgaralar's library (grids.rs).
    Grid(crate::grids::Event),
    /// Ölçme: a field typed (docs/adr/0169 §3).
    Survey(survey_form::Field, String),
    /// Değişkenler: a row changed (docs/adr/0214 §4).
    Variable(usize, super::variables::Edit),
    /// Değişken ekle: a new row, the keyboard to its name.
    AddVariable,
    /// Mimari: a level's row changed, Kat ekle, the walls' detail (docs/adr/0339 §3).
    Level(usize, super::arch::Edit),
    AddLevel,
    /// Nesneleri seç beside a level taken away that objects name (docs/adr/0720 §3): they are selected, the window
    /// closes as Vazgeç would, for them to be given another level.
    SelectLevelUsers,
    ArchDetail(Option<kentos_commands::WallDetail>),
    ArchMethod(Option<kentos_commands::SpaceMethod>),
    RoomChoices(super::arch::ChoiceEvent),
    /// Kotlar ve epok: a field changed (docs/adr/0519 §5).
    Heights(super::heights::Edit),
    /// Uzunlukları projeksiyona indir (docs/adr/0171 §4).
    Reduce(bool),
    /// The shown section's own values back to their defaults (the web's
    /// “Bu bölümü varsayılana döndür”).
    ResetSection,
    /// Uygulama ayarları over this window, which comes back when it closes.
    OpenApp,
    Save,
    /// The unit question's answer (docs/adr/0256 §2): the sizes or the
    /// numbers kept; none: Vazgeç, back to the window.
    UnitAnswer(Option<kentos_commands::UnitGeometry>),
}

fn event(e: Event) -> Message {
    message(ProjectEvent::Settings(e))
}

impl State {
    pub fn new(doc: &Document) -> Self {
        Self {
            section: Section::General,
            name: doc.name().to_owned(),
            settings: doc.settings().clone(),
            initial_name: doc.name().to_owned(),
            initial: doc.settings().clone(),
            query: String::new(),
            removing: None,
            choices: super::choices::Choices::of(&doc.settings().datum_transforms),
            custom: None,
            defined: doc
                .settings()
                .custom_crs
                .clone()
                .filter(|_| doc.settings().srid == crs::LOCAL_SRID),
            second_defined: doc.settings().second_custom_crs.clone(),
            survey: survey_form::texts(doc.settings().survey.as_ref(), doc.settings().angle_unit),
            reduce: doc.settings().reduces_to_grid(),
            scale_typed: kentos_processing::text::js_number(doc.settings().plot_scale),
            variables: variable_form::rows(&doc.settings().variables),
            arch: super::arch::rows(doc.settings().arch.as_ref()),
            arch_detail: doc.settings().arch.as_ref().and_then(|a| a.detail),
            arch_method: doc.settings().arch.as_ref().and_then(|a| a.space_method),
            heights: super::heights::texts(
                doc.settings().heights.as_ref(),
                doc.settings().coordinate_epoch,
            ),
            unit_question: None,
        }
    }

    /// The draft's survey settings (Ölçme).
    #[cfg(test)]
    pub(super) fn draft_survey(&self) -> Option<kentos_commands::SurveySettings> {
        self.settings.survey.clone()
    }

    /// The draft's settings as they stand.
    #[cfg(test)]
    pub(super) fn draft_settings(&self) -> ProjectSettings {
        self.settings.clone()
    }

    /// Whether Ölçme has a text to put right: Kaydet waits.
    fn survey_blocked(&self) -> bool {
        survey_form::read(&self.survey, self.settings.angle_unit).blocked()
    }

    /// Whether a variable's row has something to put right: Kaydet waits.
    fn variables_blocked(&self) -> bool {
        variable_form::read(&self.variables).blocked()
    }

    /// Whether Mimari's levels have something to put right: Kaydet waits.
    /// Whether Kotlar ve epok has something to put right: Kaydet waits.
    fn heights_blocked(&self) -> bool {
        super::heights::read(&self.heights, self.settings.has_system()).is_err()
    }

    /// A level the rows take away that objects or a sheet's map name (docs/adr/0720 §3): why it stays; Kaydet waits.
    pub(super) fn levels_in_use(
        &self,
        doc: &kentos_domain::Document,
    ) -> Option<kentos_commands::CommandError> {
        let arch = super::arch::read(
            &self.arch,
            self.arch_detail,
            self.arch_method,
            self.settings.arch.as_ref(),
        )
        .ok()?;
        kentos_native_application::settings_set::levels_in_use(doc, &arch)
    }

    fn arch_blocked(&self) -> bool {
        super::arch::read(
            &self.arch,
            self.arch_detail,
            self.arch_method,
            self.settings.arch.as_ref(),
        )
        .is_err()
    }

    /// The draft's system: its SRID, and whether it is the project's own definition.
    #[cfg(test)]
    pub(super) fn draft_srid(&self) -> (u32, bool) {
        (self.settings.srid, self.settings.custom_crs.is_some())
    }

    /// Esc: the window over this one, or the question in a row, goes first.
    pub(super) fn dismiss_inner(&mut self) -> bool {
        self.unit_question.take().is_some()
            || self.custom.take().is_some()
            || self.removing.take().is_some()
    }
}

impl App {
    pub(super) fn project_settings_event(&mut self, e: Event) -> Task<Message> {
        if let Event::Save = e {
            self.save_project_settings(None);
            return Task::none();
        }
        if let Event::UnitAnswer(answer) = e {
            if let Some(Window::Settings(s)) = &mut self.project {
                s.unit_question = None;
            }
            if answer.is_some() {
                self.save_project_settings(answer);
            }
            return Task::none();
        }
        if let Event::OpenApp = e {
            // This window waits under Uygulama ayarları, as it is (app.rs `dialog_back`); on Yeni projeler where the
            // program has it (module_surfaces.rs).
            let section = crate::settings_sections::Section::NewProjects;
            self.open_settings_at(if self.shows_settings_section(section) {
                section
            } else {
                crate::settings_sections::Section::Appearance
            });
            self.dialog_under = Some(crate::app::Dialog::Project);
            return Task::none();
        }
        if let Event::Custom(e) = e {
            return self.custom_crs_event(e);
        }
        if let Event::SelectLevelUsers = e {
            self.select_level_users();
            return Task::none();
        }
        // A project keeps the type it has (docs/adr/0622 §1): only one not asked its type yet takes one here.
        if let Event::Mode(_) = e
            && self
                .document
                .as_ref()
                .is_some_and(|d| d.settings().project_type().is_some())
        {
            return Task::none();
        }
        // The grids a datum choice may name and the user's notation, before the window is borrowed.
        let grids = self.choice_grids();
        let notation = self.draft.geographic;
        let self_sections = self.project_sections();
        let Some(Window::Settings(s)) = &mut self.project else {
            return Task::none();
        };
        // A section the program has not (an opener set it) is Genel, as the window shows it.
        if !self_sections.contains(&s.section) {
            s.section = Section::General;
        }
        if let Event::Grid(e) = e {
            s.removing = None;
            return self.grid_event(e);
        }
        let d = &mut s.settings;
        match e {
            // Only a section this program shows (module_surfaces.rs); the list offers no other.
            Event::Section(section) => {
                if self_sections.contains(&section) {
                    s.section = section;
                }
            }
            Event::Name(name) => s.name = name,
            Event::Scale(v) => {
                d.plot_scale = v;
                s.scale_typed = kentos_processing::text::js_number(v);
            }
            Event::ScaleTyped(t) => {
                if let Some(n) = crate::annotation_scale::typed_plot_scale(&t) {
                    d.plot_scale = n;
                }
                s.scale_typed = t;
            }
            Event::Height(kind, mm) => {
                d.annotation = d
                    .annotation
                    .clone()
                    .unwrap_or_default()
                    .with(kind, Some(mm))
                    .sanitized();
            }
            Event::ResetHeights => d.annotation = None,
            Event::Mode(w) => d.workspace = Some(w),
            Event::Font(f) => d.drawing_font = Some(f),
            Event::AutoSelect(on) => d.auto_select = Some(on),
            // A system of the registry takes the definition's place (docs/adr/0168 §1).
            Event::Crs(srid) => {
                d.srid = srid;
                d.custom_crs = None;
            }
            Event::Search(query) => {
                let digits: String = query.chars().filter(char::is_ascii_digit).collect();
                if let Ok(srid) = digits.parse::<u32>()
                    && digits.len() >= 4
                    && crs::system(srid).is_some()
                {
                    d.srid = srid;
                    d.custom_crs = None;
                }
                s.query = query;
            }
            Event::Defined => {
                if let Some(def) = &s.defined {
                    d.srid = crs::LOCAL_SRID;
                    d.custom_crs = Some(def.clone());
                }
            }
            Event::Second(second) => {
                d.second_srid = second;
                d.second_custom_crs = None;
            }
            Event::SecondDefined => {
                if let Some(def) = &s.second_defined {
                    d.second_srid = None;
                    d.second_custom_crs = Some(def.clone());
                }
            }
            Event::NewDefinition(target) => {
                let context = custom_context(target, d, &s.initial, notation);
                s.custom = Some(custom_crs::Editor::new(target, None, context));
            }
            Event::EditDefinition(target) => {
                let context = custom_context(target, d, &s.initial, notation);
                let existing = match target {
                    Target::Own => s.defined.as_ref(),
                    Target::Second => s.second_defined.as_ref(),
                };
                s.custom = Some(custom_crs::Editor::new(target, existing, context));
            }
            Event::Custom(_) => {}
            Event::GridAsk(id) => s.removing = id,
            Event::Choice(i, edit) => s.choices.edit(i, edit, &grids, &mut d.datum_transforms),
            Event::Grid(_) => {}
            Event::LengthDecimals(n) => d.length_decimals = n.min(4),
            Event::AreaDecimals(n) => d.area_decimals = n.min(4),
            Event::AreaUnit(u) => d.area_unit = u,
            // The tolerances are typed in the unit's cc or ″: their texts follow it.
            Event::AngleUnit(u) => {
                d.angle_unit = u;
                if !survey_form::read(&s.survey, u).blocked() {
                    s.survey = survey_form::texts(d.survey.as_ref(), u);
                }
            }
            Event::Survey(field, t) => {
                if let Some(i) = survey_form::Field::ALL.iter().position(|f| *f == field) {
                    s.survey[i] = t;
                }
                // The reduction to the grid rides beside the texts (docs/adr/0171 §4).
                d.survey = survey_form::with_reduction(
                    survey_form::read(&s.survey, d.angle_unit).survey,
                    s.reduce,
                );
            }
            Event::Reduce(on) => {
                s.reduce = on;
                d.survey = survey_form::with_reduction(
                    survey_form::read(&s.survey, d.angle_unit).survey,
                    s.reduce,
                );
            }
            Event::DrawingUnit(u) => {
                d.drawing_unit = (u != DrawingUnit::M).then_some(u);
                // A metric unit writes decimal (docs/adr/0262 §2).
                if !u.imperial() {
                    d.length_style = None;
                }
            }
            Event::LengthStyle(style) => {
                d.length_style = (style != kentos_commands::LengthStyle::Decimal).then_some(style);
            }
            // The draft takes the rows that hold (docs/adr/0214 §4).
            Event::Variable(i, edit) => {
                super::variables::edit(&mut s.variables, i, edit);
                d.variables = variable_form::read(&s.variables).variables;
            }
            Event::AddVariable => {
                s.variables.push(variable_form::new_row(&s.variables));
                d.variables = variable_form::read(&s.variables).variables;
                let id = super::variables::name_id(s.variables.len() - 1);
                return Task::batch([
                    iced::widget::operation::focus(id.clone()),
                    iced::widget::operation::select_all(id),
                ]);
            }
            Event::Level(i, edit) => super::arch::edit(&mut s.arch, i, edit),
            Event::AddLevel => {
                let row = super::arch::new_row(&s.arch);
                s.arch.push(row);
            }
            Event::ArchDetail(detail) => s.arch_detail = detail,
            Event::ArchMethod(method) => s.arch_method = method,
            Event::RoomChoices(edit) => {
                super::arch::edit_choices(d.arch.get_or_insert_with(Default::default), edit);
            }
            Event::Heights(edit) => super::heights::edit(&mut s.heights, edit),
            Event::ResetSection if s.section == Section::Heights => {
                s.heights = super::heights::Texts::default();
            }
            Event::ResetSection if s.section == Section::Arch => {
                s.arch.clear();
                s.arch_detail = None;
                s.arch_method = None;
                d.arch = None;
            }
            Event::ResetSection => {
                reset_section(s.section, d);
                s.survey = survey_form::texts(d.survey.as_ref(), d.angle_unit);
                s.reduce = d.reduces_to_grid();
                s.variables = variable_form::rows(&d.variables);
            }
            Event::OpenApp | Event::Save | Event::UnitAnswer(_) | Event::SelectLevelUsers => {}
        }
        // The project's own system, or none, is no second system (docs/adr/0167 §1).
        d.second_srid = d.second();
        Task::none()
    }

    /// Özel koordinat sistemi's fields and buttons: Tamam puts the definition
    /// in the draft (Kaydet assigns it), Kayıttakini seç the registry's
    /// system; a `.prj` file is asked for, a text goes to the clipboard.
    fn custom_crs_event(&mut self, e: custom_crs::Event) -> Task<Message> {
        let Some(Window::Settings(s)) = &mut self.project else {
            return Task::none();
        };
        let Some(editor) = s.custom.as_mut() else {
            return Task::none();
        };
        let d = &mut s.settings;
        match editor.edit(e) {
            Outcome::Keep => return Task::none(),
            Outcome::Close => s.custom = None,
            Outcome::Done(Target::Own, def) => {
                s.custom = None;
                d.srid = crs::LOCAL_SRID;
                d.custom_crs = Some(def.clone());
                s.defined = Some(def);
            }
            Outcome::Done(Target::Second, def) => {
                s.custom = None;
                d.second_srid = None;
                d.second_custom_crs = Some(def.clone());
                s.second_defined = Some(def);
            }
            // The registry's system takes the place of a definition (docs/adr/0168 §1, §6).
            Outcome::Registry(Target::Own, srid) => {
                s.custom = None;
                d.srid = srid;
                d.custom_crs = None;
            }
            Outcome::Registry(Target::Second, srid) => {
                s.custom = None;
                d.second_srid = Some(srid);
                d.second_custom_crs = None;
            }
            Outcome::PickFile => {
                return Task::perform(
                    async {
                        rfd::AsyncFileDialog::new()
                            .set_title("Koordinat sistemi tanımı al")
                            .add_filter(
                                "Koordinat sistemi tanımı (.prj, .wkt, .txt)",
                                &["prj", "PRJ", "wkt", "WKT", "txt", "TXT"],
                            )
                            .pick_file()
                            .await
                            .map(|f| f.path().to_path_buf())
                    },
                    |path| event(Event::Custom(custom_crs::Event::Picked(path))),
                );
            }
            Outcome::Copy(text, what) => {
                self.say(Level::Success, format!("{what} panoya kopyalandı."));
                return iced::clipboard::write(text);
            }
            Outcome::Focus(task) => return task,
        }
        // The project's own system, or none, is no second system (docs/adr/0167 §1).
        d.second_srid = d.second();
        Task::none()
    }

    /// The draft's unit change, when a local project stays local and its unit
    /// changes (docs/adr/0256 §2): with a drawing in it, asked about first
    /// (`unit` none: the question opens, Kaydet waits); then written by
    /// `cad.units.change` as one undo step, the sizes kept on an empty
    /// drawing. Whether Kaydet goes on: a refusal is said and the window stays.
    fn change_unit(&mut self, unit: Option<kentos_commands::UnitGeometry>) -> bool {
        use kentos_commands::{CommandResult, UnitGeometry, UnitsChange};
        let (Some(Window::Settings(s)), Some(doc)) = (&mut self.project, &mut self.document) else {
            return true;
        };
        let (from, to) = (
            s.initial.drawing_unit.unwrap_or(DrawingUnit::M),
            s.settings.drawing_unit.unwrap_or(DrawingUnit::M),
        );
        if from == to || s.initial.has_system() || s.settings.has_system() {
            return true;
        }
        let model = &doc.model;
        let held = !model.is_empty() || !model.blocks().is_empty();
        let Some(geometry) = unit.or((!held).then_some(UnitGeometry::Keep)) else {
            s.unit_question = Some(super::unit_change::Question::of(model, from, to));
            return false;
        };
        let input = UnitsChange {
            from,
            to,
            geometry,
            expected_revision: None,
        };
        let result = kentos_native_application::units_change::execute(
            &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
            input,
        );
        match result {
            CommandResult::Completed { output, warnings } => {
                let said = match output.geometry {
                    UnitGeometry::Keep => {
                        format!("Çizim birimi {} oldu; boylar korundu.", output.to.mark())
                    }
                    UnitGeometry::Scale => format!(
                        "Çizim birimi {} oldu; sayılar korundu, çizim ölçeklendi ({} nesne, {} blok tanımı).",
                        output.to.mark(),
                        output.objects,
                        output.blocks
                    ),
                };
                self.say(Level::Success, said);
                for w in warnings {
                    self.warn(w.message);
                }
                true
            }
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => {
                self.warn(error.message);
                false
            }
            CommandResult::Queued { .. } | CommandResult::Cancelled => false,
        }
    }

    /// Kaydet: the draft goes into the drawing (the web's `onSave`). A local
    /// project's unit changing with a drawing in it is asked about first
    /// (`unit`: the answer, docs/adr/0256 §2) and written by `cad.units.change`.
    fn save_project_settings(&mut self, unit: Option<kentos_commands::UnitGeometry>) {
        if !self.change_unit(unit) {
            return;
        }
        let (Some(Window::Settings(s)), Some(doc)) = (&self.project, &mut self.document) else {
            return;
        };
        // A datum choice, an Ölçme field or a variable with something to put right is not saved (its button waits too).
        if s.choices.blocked()
            || s.survey_blocked()
            || s.variables_blocked()
            || s.arch_blocked()
            || s.heights_blocked()
        {
            return;
        }
        // Mimari through `cad.settings.set` first (docs/adr/0339 §3): a level a wall names is kept, and nothing is
        // saved then.
        let mut arch = super::arch::read(
            &s.arch,
            s.arch_detail,
            s.arch_method,
            s.settings.arch.as_ref(),
        )
        .unwrap_or_default();
        // The phase view is Mimari › Evre's, not this form's (docs/adr/0346 §2): it stays as it was.
        arch.phase_view = s.initial.arch.as_ref().and_then(|a| a.phase_view);
        // So are Denetim's exceptions (docs/adr/0732 §3).
        arch.audit_exceptions = s
            .initial
            .arch
            .as_ref()
            .map_or_else(Vec::new, |a| a.audit_exceptions.clone());
        // So are the door and window types (docs/adr/0772 §1).
        arch.opening_types = s
            .initial
            .arch
            .as_ref()
            .map_or_else(Vec::new, |a| a.opening_types.clone());
        // So is the building's program (docs/adr/0773): Bina programı writes it.
        arch.program = s.initial.arch.as_ref().map(|a| a.program.clone()).unwrap_or_default();
        // So are the room modules (docs/adr/0775).
        arch.modules = s.initial.arch.as_ref().map_or_else(Vec::new, |a| a.modules.clone());
        // So are the building materials (docs/adr/0785): Malzemeler writes them.
        arch.materials = s.initial.arch.as_ref().map_or_else(Vec::new, |a| a.materials.clone());
        // So are the level and view the drawing opens with (docs/adr/0720 §1), but not a level taken away here.
        if !arch.levels.is_empty() {
            let kept = |id: &String| arch.levels.iter().any(|l| l.id == *id);
            arch.active_level = s
                .initial
                .arch
                .as_ref()
                .and_then(|a| a.active_level.clone())
                .filter(|a| a.as_ref().is_none_or(kept));
            arch.level_view = s.initial.arch.as_ref().and_then(|a| a.level_view);
        }
        if Some(&arch) != s.initial.arch.as_ref() && !(arch.is_empty() && s.initial.arch.is_none())
        {
            let input = kentos_commands::SettingsSet {
                arch: Some(arch.clone()),
                ..Default::default()
            };
            let result = kentos_native_application::settings_set::execute(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                input,
            );
            if let kentos_commands::CommandResult::Failed { error }
            | kentos_commands::CommandResult::Conflict { error }
            | kentos_commands::CommandResult::NeedsInput { error } = result
            {
                // A level objects name stays (docs/adr/0720 §3): they are selected, to give them another level.
                let named: Vec<kentos_domain::Slot> =
                    if error.code == kentos_commands::codes::LEVEL_IN_USE {
                        error
                            .uids
                            .iter()
                            .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                            .filter_map(|u| doc.model.slot_of(u))
                            .collect()
                    } else {
                        Vec::new()
                    };
                if named.is_empty() {
                    self.warn(error.message);
                } else {
                    let n = named.len();
                    self.selection.set(named);
                    self.warn(format!("{} Andığı {n} nesne seçildi.", error.message));
                }
                return;
            }
        }
        let (Some(Window::Settings(s)), Some(doc)) = (&self.project, &mut self.document) else {
            return;
        };
        let name = s.name.trim();
        let name = if name.is_empty() {
            s.initial_name.clone()
        } else {
            name.to_owned()
        };
        doc.model.set_name(&name);
        let mut settings = s.settings.clone();
        settings.arch = doc.model.settings().arch.clone();
        // Kotlar ve epok go through `cad.settings.set` after the system (docs/adr/0519 §5): they rest on it.
        settings.heights = doc.model.settings().heights.clone();
        settings.coordinate_epoch = doc.model.settings().coordinate_epoch;
        let heights = super::heights::read(&s.heights, s.settings.has_system()).unwrap_or_default();
        // A drawing unit is a local project's (docs/adr/0165 §2): one given a coordinate system is in metres.
        if settings.has_system() {
            settings.drawing_unit = None;
        }
        // A reduction to the grid the project cannot take is not kept (docs/adr/0171 §4).
        if kentos_interaction::ground::why_not_grid(&settings).is_some()
            && let Some(survey) = settings.survey.take()
        {
            settings.survey = kentos_commands::SurveySettings {
                reduce_to_grid: None,
                ..survey
            }
            .sanitized();
        }
        // What the annotations' heights hung on before, for them to follow (docs/adr/0205 §3).
        let change = kentos_commands::ScaleChange {
            from_scale: s.initial.plot_scale,
            to_scale: settings.plot_scale,
            from: s.initial.annotation.clone().unwrap_or_default(),
            to: settings.annotation.clone().unwrap_or_default(),
        };
        doc.model.set_settings(settings);
        if (heights.0.as_ref(), heights.1)
            != (
                doc.model.settings().heights.as_ref(),
                doc.model.settings().coordinate_epoch,
            )
        {
            let input = kentos_commands::SettingsSet {
                heights: Some(
                    heights
                        .0
                        .clone()
                        .unwrap_or(kentos_commands::HeightReference {
                            kind: kentos_commands::HeightKind::Unknown,
                            vertical_srid: None,
                            geoid: None,
                        }),
                ),
                coordinate_epoch: Some(heights.1.unwrap_or(0.0)),
                ..Default::default()
            };
            let result = kentos_native_application::settings_set::execute(
                &mut kentos_native_application::ExecutionContext::new(&mut doc.model),
                input,
            );
            if let kentos_commands::CommandResult::Failed { error }
            | kentos_commands::CommandResult::Conflict { error }
            | kentos_commands::CommandResult::NeedsInput { error } = result
            {
                // A level objects name stays (docs/adr/0720 §3): they are selected, to give them another level.
                let named: Vec<kentos_domain::Slot> =
                    if error.code == kentos_commands::codes::LEVEL_IN_USE {
                        error
                            .uids
                            .iter()
                            .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
                            .filter_map(|u| doc.model.slot_of(u))
                            .collect()
                    } else {
                        Vec::new()
                    };
                if named.is_empty() {
                    self.warn(error.message);
                } else {
                    let n = named.len();
                    self.selection.set(named);
                    self.warn(format!("{} Andığı {n} nesne seçildi.", error.message));
                }
                return;
            }
        }
        let (Some(Window::Settings(s)), Some(doc)) = (&self.project, &mut self.document) else {
            return;
        };
        // The project's system, a definition's too (docs/adr/0168 §1).
        let assigned = (s.settings.srid, s.settings.custom_crs.as_ref())
            != (s.initial.srid, s.initial.custom_crs.as_ref());
        let title = crs::project_title(doc.settings());
        let second = doc.settings().second_srid;
        let defined = doc.settings().second_custom_crs.as_ref();
        let second_changed =
            (second, defined) != (s.initial.second_srid, s.initial.second_custom_crs.as_ref());
        let defined = defined.map(kentos_project::systems::definition_title);
        self.close_project_window();
        if assigned {
            self.say(
                Level::Success,
                format!(
                    "Proje koordinat sistemi {title} olarak atandı. Koordinat değerleri değiştirilmedi."
                ),
            );
        }
        if second_changed {
            self.say(
                Level::Success,
                match (second, defined) {
                    (None, None) => "İkinci koordinat sistemi kaldırıldı.".to_owned(),
                    (Some(srid), _) => format!(
                        "İkinci koordinat sistemi: {}. Çizim dönüştürülmedi.",
                        crs::title_of(srid)
                    ),
                    (None, Some(title)) => {
                        format!("İkinci koordinat sistemi: {title}. Çizim dönüştürülmedi.")
                    }
                },
            );
        }
        self.announce(
            Level::Success,
            "Proje ayarları kaydedildi. Proje dosyasıyla birlikte saklanacak.",
        );
        // The annotations at the old general height take the new one, in one step.
        self.follow_annotations(change);
    }

    pub(super) fn project_settings_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(doc) = &self.document else {
            return text("").into();
        };
        // Özel koordinat sistemi over this window, which waits under it.
        if let Some(editor) = &s.custom {
            return custom_crs::view(editor, |e| event(Event::Custom(e)));
        }
        // The unit question over it too (docs/adr/0256 §2).
        if let Some(q) = &s.unit_question {
            return super::unit_change::view(q, |a| event(Event::UnitAnswer(a)));
        }
        // The program's own sections (module_surfaces.rs); one it has not shows Genel.
        let sections = self.project_sections();
        let current = if sections.contains(&s.section) {
            s.section
        } else {
            Section::General
        };
        let nav = sections
            .iter()
            .fold(Column::new().spacing(2).width(190), |nav, section| {
                nav.push(
                    button(
                        row![
                            kentos_ui::icon::icon(section.icon()).size(16.0),
                            label::body(section.label())
                        ]
                        .spacing(8)
                        .align_y(iced::Center),
                    )
                    .on_press(event(Event::Section(*section)))
                    .padding([6, 10])
                    .width(Fill)
                    .style(style::button::navigation(*section == current)),
                )
            });
        // A level the rows take away that objects name (docs/adr/0720 §3): said under the levels, Kaydet waits.
        let waiting = s.levels_in_use(&doc.model);
        let in_use = waiting.clone();
        let page: Element<'a, Message> = match current {
            Section::General => self.general(s, doc),
            Section::Crs => self.crs_section(s),
            Section::Heights => super::heights::view(
                &s.heights,
                s.settings.has_system(),
                self.second_record(s),
                self.grids
                    .entries
                    .iter()
                    .filter(|e| {
                        e.model == Some(kentos_native_application::crs_models::ModelKind::Geoid)
                    })
                    .map(|e| (e.file.clone(), e.id.clone()))
                    .collect(),
                |e| event(Event::Heights(e)),
            ),
            Section::Units => {
                let (scale, heights) = super::scale::view(
                    s.settings.plot_scale,
                    s.settings.annotation.as_ref(),
                    crate::catalog::effective_mode(s.settings.workspace).id == Workspace::Cad,
                    s.settings.unit(),
                    &s.scale_typed,
                    kentos_interaction::Format::of(&s.settings),
                    |v| event(Event::Scale(v)),
                    |t| event(Event::ScaleTyped(t)),
                    |kind, mm| event(Event::Height(kind, mm)),
                    event(Event::ResetHeights),
                );
                units(s, scale, heights)
            }
            Section::Survey => super::survey::view(
                &s.survey,
                s.settings.angle_unit,
                s.reduce,
                kentos_interaction::ground::why_not_grid(&s.settings),
                |f, t| event(Event::Survey(f, t)),
                |on| event(Event::Reduce(on)),
            ),
            // The built-in values as they are now: the draft's name, system and scale.
            Section::Variables => super::variables::view(
                &s.variables,
                self.variables_of(&s.name, &s.settings),
                |i, e| event(Event::Variable(i, e)),
                event(Event::AddVariable),
            ),
            Section::Arch => column![
                super::arch::view(
                    &s.arch,
                    s.arch_detail,
                    |i, e| event(Event::Level(i, e)),
                    event(Event::AddLevel),
                    |d| event(Event::ArchDetail(d)),
                    s.arch_method,
                    |m| event(Event::ArchMethod(m)),
                ),
                super::arch::in_use_view(in_use.as_ref(), event(Event::SelectLevelUsers)),
                super::arch::choices_view(s.settings.arch.as_ref(), |e| event(Event::RoomChoices(
                    e
                ))),
            ]
            .spacing(14)
            .into(),
        };
        let content = column![
            text(current.label())
                .font(typography::ui_strong())
                .size(typography::body()),
            label::caption(current.lead()),
            // The page scrolls; Vazgeç and Kaydet stay in view whatever the window's height.
            scrollable(page)
                .direction(style::field::body_scrollbar())
                .height(Fill),
        ]
        .spacing(10)
        .width(Fill);
        let scope = row![
            label::caption("Proje dosyasına kaydedilir:"),
            label::caption(doc.name().to_owned()),
        ]
        .spacing(6);
        // A section without values of its own (Koordinat sistemi) has nothing to reset.
        let own = current != Section::Crs;
        let reset = button(label::body("Bu bölümü varsayılana döndür"))
            .on_press_maybe(own.then_some(event(Event::ResetSection)))
            .padding(kentos_ui::theme::metrics::action())
            .style(style::button::ghost);
        let reset: Element<'a, Message> = if own {
            reset.into()
        } else {
            kentos_ui::widget::tip(
                reset,
                kentos_ui::widget::Tip::new("Bu bölümde varsayılana dönecek ayar yok."),
                iced::widget::tooltip::Position::Top,
            )
        };
        overlay::blocking(
            Dialog::new("Proje ayarları")
                .push(column![row![nav, content].spacing(16), scope].spacing(12))
                .aside(reset)
                .action(words::secondary(
                    "Vazgeç",
                    Some(message(ProjectEvent::Close)),
                ))
                // A datum choice, an Ölçme field, a variable or a level in use with something to put right keeps it
                // waiting.
                .action(words::primary(
                    "Kaydet",
                    (!s.choices.blocked()
                        && !s.survey_blocked()
                        && !s.variables_blocked()
                        && !s.arch_blocked()
                        && waiting.is_none()
                        && !s.heights_blocked())
                    .then(|| event(Event::Save)),
                ))
                .width(900.0)
                .max_height(760.0)
                .on_close(Some(message(ProjectEvent::Close))),
        )
    }

    /// The way from the draft's system to its second as `cad.crs.transform` records it, at the drawing's origin
    /// (docs/adr/0519 §5): its title and the answer or why there is none; none without a second system of the
    /// registry.
    fn second_record(
        &self,
        s: &State,
    ) -> Option<(String, Result<kentos_commands::CrsTransformed, String>)> {
        let doc = self.document.as_ref()?;
        let second = s.settings.second()?;
        if s.settings.custom_crs.is_some() || crs::system(s.settings.srid).is_none() {
            return None;
        }
        let origin = doc.model.origin();
        let heights = super::heights::read(&s.heights, s.settings.has_system())
            .ok()
            .and_then(|(h, _)| h);
        let input = kentos_commands::CrsTransform {
            points: vec![kentos_commands::CrsPoint {
                x: origin.x,
                y: origin.y,
                z: None,
            }],
            from: Some(kentos_commands::CrsSide {
                srid: Some(s.settings.srid),
                heights,
                epoch: None,
            }),
            to: Some(kentos_commands::CrsSide {
                srid: Some(second),
                heights: None,
                epoch: None,
            }),
            uncertainty: None,
            velocity: None,
        };
        let title = format!(
            "{} → {}, çizimin orijininde",
            crs::title_of(s.settings.srid),
            crs::title_of(second)
        );
        Some((
            title,
            match kentos_native_application::crs_transform::answer(&doc.model, &input) {
                kentos_commands::CommandResult::Completed { output, .. } => Ok(output),
                kentos_commands::CommandResult::Failed { error }
                | kentos_commands::CommandResult::Conflict { error }
                | kentos_commands::CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Dönüşüm hesaplanamadı.".into()),
            },
        ))
    }

    fn general<'a>(&'a self, s: &'a State, doc: &'a Document) -> Element<'a, Message> {
        let name = kentos_ui::widget::text_field("Proje adı", &s.name)
            .on_input(|t| event(Event::Name(t)))
            .width(260);
        let name = name;
        let font = Select::new(
            FONTS.iter().map(|(_, name)| Choice::new(*name)),
            FONTS.iter().position(|(f, _)| {
                Some(*f) == s.settings.drawing_font.or(Some(DrawingFont::Barlow))
            }),
            |i| event(Event::Font(FONTS[i.min(FONTS.len() - 1)].0)),
        )
        .searchable(false);
        let origin = doc.model.origin();
        Column::new()
            .spacing(16)
            .push(group(
                "Proje",
                Column::new()
                    .spacing(10)
                    .push(setting("Proje adı", Some("Dosya adı olarak da kullanılır."), name)),
            ))
            .push(group("Proje türü", project_type(doc.settings().project_type(), s)))
            // Otomatik seçim (layer_types.rs, docs/adr/0622 §5).
            .push(group(
                "Seçim",
                setting(
                    "Otomatik seçim",
                    Some("Açıkken tıklama ve kutu bütün görünen katmanların nesnelerini seçer; kapalıyken yalnız etkin katmanın nesnelerini, CBS'de bir katman düzenlenirken olduğu gibi. CBS projesinde kapalı, CAD projesinde açık başlar."),
                    kentos_ui::widget::Switch::new(s.settings.auto_select(), |on| event(Event::AutoSelect(on))),
                ),
            ))
            .push(group(
                "Çizim yazı tipi",
                column![
                    label::caption("Çizimdeki yazılar, ölçü değerleri ve etiketler bu yazı tipiyle çizilir; projeyi açan herkes aynısını görür. Arayüzün yazı tipi Uygulama ayarlarındadır."),
                    container(font).width(260),
                ]
                .spacing(8),
            ))
            .push(group(
                "Özet",
                Column::new()
                    .spacing(8)
                    .push(setting("Nesne sayısı", None, label::body(doc.entity_count().to_string())))
                    .push(setting("Katman sayısı", None, label::body(doc.layer_count().to_string())))
                    .push(setting(
                        "Yerel çizim orijini",
                        Some("Büyük TM koordinatları ekran kartında bu noktaya göre çizilir; hassasiyet kaybını önler."),
                        label::mono({
                            // East and north as the project's type names them (docs/adr/0165 §4).
                            let f = kentos_interaction::Format::of(doc.settings());
                            format!(
                                "{} {}  {} {}",
                                f.east_label(),
                                kentos_interaction::fixed(origin.x, 0),
                                f.north_label(),
                                kentos_interaction::fixed(origin.y, 0)
                            )
                        }),
                    )),
            ))
            .into()
    }

    fn crs_section<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let open_app = words::secondary("Uygulama ayarlarını aç", Some(event(Event::OpenApp)));
        let default_srid = self.settings.number("newProjects.srid") as u32;
        let default = crs::system(default_srid)
            .map(|c| format!("{}. Uygulama ayarlarından değiştirilir.", crs::title(c)));
        column![
            crs::picker(
                s.settings.srid,
                s.initial.srid,
                default_srid,
                &s.query,
                |srid| event(Event::Crs(srid)),
                |q| event(Event::Search(q)),
                crs::Defined {
                    definition: s.defined.as_ref(),
                    chosen: s.settings.custom_crs.is_some() && s.settings.srid == crs::LOCAL_SRID,
                    was: s.initial.custom_crs.is_some() && s.initial.srid == crs::LOCAL_SRID,
                    changed: (s.settings.srid, s.settings.custom_crs.as_ref())
                        != (s.initial.srid, s.initial.custom_crs.as_ref()),
                    on_pick: event(Event::Defined),
                    on_new: event(Event::NewDefinition(Target::Own)),
                    on_edit: event(Event::EditDefinition(Target::Own)),
                },
            ),
            // The second system and the new projects' default where the program has them (module_surfaces.rs).
            self.shows_cell(crate::module_surfaces::Cell::SecondCrs)
                .then(|| second_group(s)),
            super::choices::view(&s.choices, self.choice_grids(), |i, e| {
                event(Event::Choice(i, e))
            }),
            self.grids_group(s),
            self.shows_setting(crate::module_surfaces::AppSetting::NewProjects)
                .then(|| group(
                    "Yeni projeler",
                    row![
                        column![
                            label::body("Yeni projelerin varsayılanı"),
                            label::caption(default.unwrap_or_default()),
                        ]
                        .spacing(2)
                        .width(Fill),
                        open_app,
                    ]
                    .spacing(16)
                    .align_y(iced::Center),
                )),
        ]
        .spacing(16)
        .into()
    }
}

impl App {
    /// Izgaralar (docs/adr/0168 §4, §6): the device's NTv2 grids, each with
    /// what its header says and Kaldır (asked in its row), the grids the
    /// project's datum choices name that this device does not have, and
    /// Ekle…. The grids are the device's, not the project's.
    fn grids_group<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let entries = &self.grids.entries;
        let mut list = Column::new().spacing(10);
        for e in entries {
            let about = column![
                label::body(e.file.clone()),
                label::caption(crate::grids::grid_line(e))
            ]
            .spacing(2)
            .width(Fill);
            let side: Element<'a, Message> = if s.removing.as_deref() == Some(e.id.as_str()) {
                row![
                    label::caption("Cihazdan silinsin mi?"),
                    button(label::body("Sil"))
                        .on_press(event(Event::Grid(crate::grids::Event::Remove(
                            e.id.clone()
                        ))))
                        .padding(kentos_ui::theme::metrics::action())
                        .style(style::button::danger),
                    words::secondary("Vazgeç", Some(event(Event::GridAsk(None)))),
                ]
                .spacing(8)
                .align_y(iced::Center)
                .into()
            } else {
                words::secondary("Kaldır", Some(event(Event::GridAsk(Some(e.id.clone())))))
            };
            list = list.push(row![about, side].spacing(16).align_y(iced::Center));
        }
        let missing: Vec<&str> = s
            .settings
            .datum_transforms
            .iter()
            .filter_map(|t| t.grid.as_ref())
            .filter(|g| !entries.iter().any(|e| e.id == g.id))
            .map(|g| g.file.as_str())
            .collect();
        for file in &missing {
            list = list.push(Banner::warning(format!(
                "{file}: projenin datum seçimi bu ızgarayı istiyor, bu cihazda yok. Aynı dosyayı Ekle… ile ekleyin."
            )));
        }
        if entries.is_empty() && missing.is_empty() {
            list = list.push(label::caption("Bu cihazda ızgara yok."));
        }
        let add = words::secondary("Ekle…", Some(event(Event::Grid(crate::grids::Event::Add))));
        group(
            "Izgaralar",
            column![
                label::caption(
                    "NTv2 ızgaraları, geoit modelleri (GTX, GeoTIFF) ve hız ızgaraları bu cihazda saklanır, projeyle paylaşılmaz: proje onları SHA-256'larıyla anar, projeyi açan başka cihaza da eklenmeleri gerekir."
                ),
                list,
                row![add],
            ]
            .spacing(10),
        )
    }
}

/// Where Özel koordinat sistemi's trial point is compared (custom_crs.rs):
/// the project's system as it was saved for its own definition, the draft's
/// own system for the second; the draft's datum choices; the project's way
/// of writing points.
fn custom_context(
    target: Target,
    draft: &ProjectSettings,
    initial: &ProjectSettings,
    notation: kentos_interaction::second::Notation,
) -> custom_crs::Context {
    let reference = match target {
        Target::Own if initial.custom_crs.is_none() => kentos_project::systems::own(initial),
        Target::Own => None,
        Target::Second => kentos_project::systems::own(draft),
    }
    .and_then(|n| Some((n.name, n.system?)));
    custom_crs::Context {
        reference,
        choices: kentos_project::systems::choices(draft),
        format: crate::calc::convert::ConvertFormat::of(draft, notation),
    }
}

/// İkinci koordinat sistemi (docs/adr/0167 §1): the registry's systems but
/// the project's own, grouped by datum, the project's second definition
/// (docs/adr/0168 §1) and Yok (the web's `secondGroup`); Özel sistem… under
/// the list defines one, Düzenle beside it edits the one chosen. Its values
/// show beside the project's in the status bar and Koordinat oku; the
/// drawing is not transformed.
fn second_group<'a>(s: &'a State) -> Element<'a, Message> {
    let d = &s.settings;
    let what = "Durum çubuğunda ve Koordinat oku’da projeninkilerin yanında bu sistemin değerleri de gösterilir; çizim dönüştürülmez. ED50 değerleri EPSG’nin ±2 m’lik dönüşümüyledir, resmî dönüşüm değildir.";
    if !d.has_system() {
        return group(
            "İkinci koordinat sistemi",
            setting(
                "İkinci sistem",
                Some(
                    "Yerel projenin ikinci sistemi olmaz; önce projeye bir koordinat sistemi atayın.",
                ),
                label::body("Yok"),
            ),
        );
    }
    // Each row's choice: Yok, the project's second definition, a datum's header, a system of the registry.
    #[derive(Clone, Copy, PartialEq)]
    enum Pick {
        Yok,
        Defined,
        Header,
        Srid(u32),
    }
    let mut rows = vec![Pick::Yok];
    let mut list = vec![Choice::new("Yok")];
    if let Some(def) = &s.second_defined {
        rows.push(Pick::Defined);
        list.push(
            Choice::new(def.name.clone())
                .detail(kentos_project::systems::DEFINITION_CODE)
                .shown(kentos_project::systems::definition_title(def)),
        );
    }
    for (datum, systems) in crate::second_crs::choices(d.srid) {
        rows.push(Pick::Header);
        list.push(Choice::header(datum));
        for c in systems {
            rows.push(Pick::Srid(c.srid));
            list.push(
                Choice::new(c.name.clone())
                    .detail(format!("EPSG:{}", c.srid))
                    .shown(crate::crs::title(c)),
            );
        }
    }
    let defined = d.second_custom_crs.is_some() && d.second().is_none();
    let current = match d.second() {
        Some(srid) => Pick::Srid(srid),
        None if defined => Pick::Defined,
        None => Pick::Yok,
    };
    let selected = rows.iter().position(|r| *r == current);
    let pick = Select::new(list, selected, move |i| {
        event(match rows.get(i) {
            Some(Pick::Srid(srid)) => Event::Second(Some(*srid)),
            Some(Pick::Defined) => Event::SecondDefined,
            _ => Event::Second(None),
        })
    })
    .action(
        kentos_ui::icon::Icon::Plus,
        "Özel sistem…",
        event(Event::NewDefinition(Target::Second)),
    );
    let mut value = row![container(pick).width(300)]
        .spacing(8)
        .align_y(iced::Center);
    if defined {
        value = value.push(words::secondary(
            "Düzenle",
            Some(event(Event::EditDefinition(Target::Second))),
        ));
    }
    group(
        "İkinci koordinat sistemi",
        setting("İkinci sistem", Some(what), value),
    )
}

/// The project's type: said, never switched, once the project has one
/// (docs/adr/0622 §1); a project not asked its type yet chooses it here once.
fn project_type<'a>(saved: Option<Workspace>, s: &'a State) -> Element<'a, Message> {
    match saved {
        Some(kind) => {
            let mode = crate::catalog::effective_mode(Some(kind));
            // A CAD project's work goes with its type (docs/adr/0801).
            let work = s.settings.cad_work();
            let title = match work {
                Some(w) => format!("{} · {} · {}", mode.label, mode.title, w.name()),
                None => format!("{} · {}", mode.label, mode.title),
            };
            let note = match (kind, work) {
                (Workspace::Cad, Some(_)) => "Proje türü ve işi projeyle başlar ve değişmez: sahnesi, eksen ve açı düzeni ve şeridi türe, şeridin sekmeleri ve projenin KentOS programı işe göredir. Türler ve işler arasında geçiş ileride projeyi dönüştürerek olacak.",
                (Workspace::Cad, None) => "Proje türü projeyle başlar ve değişmez. Bu CAD çiziminin işi henüz seçilmedi: açılışta ve durum çubuğundaki türe basılınca bir kez sorulur.",
                _ => "Proje türü projeyle başlar ve değişmez: sahnesi, eksen ve açı düzeni ve şeridi türe göredir. CAD ile CBS arasında geçiş ileride projeyi dönüştürerek olacak.",
            };
            column![
                row![
                    kentos_ui::icon::icon(crate::modes::mode_icon(kind))
                        .size(16.0)
                        .tone(kentos_ui::icon::Tone::Accent),
                    text(title)
                        .font(typography::ui_strong())
                        .size(typography::body()),
                ]
                .spacing(8)
                .align_y(iced::Center),
                label::caption(note),
            ]
            .spacing(6)
            .into()
        }
        None => column![
            label::caption("Projenin türü henüz seçilmedi; CBS gösteriliyor. Tür bir kez seçilir ve sonra değişmez."),
            modes(
                crate::catalog::effective_mode(s.settings.workspace).id,
                true,
                false,
                |w| event(Event::Mode(w)),
            ),
        ]
        .spacing(8)
        .into(),
    }
}

/// Birim ve ölçek (docs/adr/0626 §4): the lengths' unit and decimals, the drawing scale (`scale`), the areas'
/// and the angles', the preview, then the annotations' heights on paper (`heights`).
fn units<'a>(
    s: &'a State,
    scale: Element<'a, Message>,
    heights: Element<'a, Message>,
) -> Element<'a, Message> {
    let d = &s.settings;
    let decimals = |value: u32, on: fn(u32) -> Event| {
        NumberInput::new(f64::from(value), move |v| {
            event(on(v.round().clamp(0.0, 4.0) as u32))
        })
        .range(0.0..=4.0)
        .step(1.0)
        .decimals(0)
        .width(120)
    };
    let area = Segmented::new(
        [
            Area(AreaUnit::M2),
            Area(AreaUnit::Donum),
            Area(AreaUnit::Ha),
        ],
        Area(d.area_unit),
        |a| event(Event::AreaUnit(a.0)),
    );
    let angle = Segmented::new(
        [Angle(AngleUnit::Grad), Angle(AngleUnit::Deg)],
        Angle(d.angle_unit),
        |a| event(Event::AngleUnit(a.0)),
    );
    // A local project's unit (docs/adr/0165 §2); a project with a coordinate system is in its metres.
    let local = !d.has_system();
    let unit = Segmented::new(
        [
            Unit(DrawingUnit::Mm),
            Unit(DrawingUnit::Cm),
            Unit(DrawingUnit::M),
            Unit(DrawingUnit::In),
            Unit(DrawingUnit::Ft),
        ],
        Unit(d.drawing_unit.unwrap_or_default()),
        |u| event(Event::DrawingUnit(u.0)),
    );
    let style = {
        use kentos_commands::LengthStyle as L;
        Segmented::new(
            [
                Style(L::Decimal),
                Style(L::Architectural),
                Style(L::Engineering),
                Style(L::Fractional),
            ],
            Style(d.length_style.unwrap_or(L::Decimal)),
            |s| event(Event::LengthStyle(s.0)),
        )
    };
    let f = Format::of(d);
    let preview = container(
        Column::new()
            .spacing(4)
            .push(
                text("Önizleme")
                    .font(typography::ui_strong())
                    .size(typography::body()),
            )
            .push(
                row![
                    container(label::caption("Koordinat")).width(90),
                    label::mono(f.point(kentos_render_wgpu::Vec2::new(
                        486_512.345_67,
                        4_420_118.920_61
                    )))
                ]
                .spacing(8),
            )
            .push(
                row![
                    container(label::caption("Kenar")).width(90),
                    label::mono(f.length(23.412_34))
                ]
                .spacing(8),
            )
            .push(
                row![
                    container(label::caption("Alan")).width(90),
                    label::mono(f.area(12_997.304))
                ]
                .spacing(8),
            )
            .push(
                row![
                    container(label::caption("Semt")).width(90),
                    label::mono(f.bearing(132.452_137))
                ]
                .spacing(8),
            ),
    )
    .padding(10)
    .width(Fill)
    .style(style::container::bordered);
    let mut lengths = Column::new().spacing(10);
    if local {
        lengths = lengths.push(setting(
            "Çizim birimi",
            Some("Uzunluklar, koordinatlar ve alanlar bu birimle yazılır ve gösterilir; çizimin kendisi değişmez."),
            unit,
        ));
    }
    // Feet and inches (docs/adr/0262 §2): an inch or foot drawing's style, its precision a fraction's.
    let imperial = local && d.drawing_unit.is_some_and(|u| u.imperial());
    if imperial {
        lengths = lengths.push(setting(
            "Uzunluk biçimi",
            Some("Mimari 12'-6 1/2\", mühendislik 12'-6.50\", kesirli 150 1/2\"; ondalık birimin kendisiyle."),
            style,
        ));
    }
    let precision = if imperial && d.length_style.is_some() {
        Some("Kesirli biçimlerde paydanın üssü: 0 tam inç, 4 on altıda bir.")
    } else {
        Some("Koordinatlar, kenar uzunlukları ve mesafeler.")
    };
    let lengths = lengths.push(setting(
        if imperial && d.length_style.is_some() {
            "Duyarlık"
        } else {
            "Ondalık basamak"
        },
        precision,
        decimals(d.length_decimals, Event::LengthDecimals),
    ));
    // A local project in millimetres or centimetres reads its areas in the unit squared: no area unit to choose.
    let mut areas = Column::new().spacing(10);
    if f.unit == DrawingUnit::M {
        areas = areas.push(setting(
            "Alan birimi",
            Some("Parsel ve kapalı alanlarda gösterilen birim. Metrekare her zaman öznitelik panelinde de yer alır."),
            area,
        ));
    }
    let areas = areas.push(setting(
        "Ondalık basamak",
        None,
        decimals(d.area_decimals, Event::AreaDecimals),
    ));
    Column::new()
        .spacing(16)
        .push(group("Uzunluk ve koordinat", lengths))
        .push(scale)
        .push(group("Alan", areas))
        .push(group(
            "Açı",
            setting("Açı birimi", Some("Semt açıları kuzeyden saat yönünde ölçülür."), angle),
        ))
        .push(preview)
        .push(Banner::info(
            "Ondalık ayırıcı her zaman noktadır; komut satırına aynı biçimde yazılabilir (Y,X virgülle ayrılır).",
        ))
        .push(heights)
        .into()
}
/// A section's own values back to the defaults the web's plan names
/// (`PROJECT_SETTINGS_DEFAULTS`, the settings schema's `project.*`); the
/// name and the coordinate system stay.
fn reset_section(section: Section, d: &mut ProjectSettings) {
    fn default(key: &str) -> Option<serde_json::Value> {
        crate::settings::schema()
            .get(key)
            .map(|def| def.default.clone())
    }
    fn read<T: serde::de::DeserializeOwned>(key: &str) -> Option<T> {
        default(key).and_then(|v| serde_json::from_value(v).ok())
    }
    match section {
        Section::General => {
            d.workspace = read("project.workspace");
            d.drawing_font = read("project.drawingFont");
            d.auto_select = None;
        }
        Section::Units => {
            d.annotation = None;
            d.drawing_unit = None;
            // The drawing scale its type and unit start at (docs/adr/0626 §1): a local CAD drawing's its unit's.
            d.plot_scale = if crate::catalog::effective_mode(d.workspace).id == Workspace::Cad
                && !d.has_system()
            {
                kentos_project::wizard::cad_default_scale(d.unit())
            } else {
                default("project.plotScale")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(1000.0)
            };
            d.length_decimals = default("project.lengthDecimals")
                .and_then(|v| v.as_u64())
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or(3);
            d.area_decimals = default("project.areaDecimals")
                .and_then(|v| v.as_u64())
                .and_then(|n| u32::try_from(n).ok())
                .unwrap_or(2);
            if let Some(unit) = read("project.areaUnit") {
                d.area_unit = unit;
            }
            if let Some(unit) = read("project.angleUnit") {
                d.angle_unit = unit;
            }
        }
        Section::Survey => d.survey = None,
        Section::Variables => d.variables = Vec::new(),
        Section::Arch => d.arch = None,
        Section::Crs | Section::Heights => {}
    }
}

impl App {
    /// Nesneleri seç (docs/adr/0720 §3): what names the level the rows take away is selected and the window closes as
    /// Vazgeç would; giving them another level (Öznitelikler › Kat) lets the level go.
    fn select_level_users(&mut self) {
        let Some(Window::Settings(s)) = &self.project else {
            return;
        };
        let Some(doc) = &self.document else {
            return;
        };
        let Some(error) = s.levels_in_use(&doc.model) else {
            return;
        };
        let named: Vec<kentos_domain::Slot> = error
            .uids
            .iter()
            .filter_map(|u| kentos_domain::Uuid::parse_str(u).ok())
            .filter_map(|u| doc.model.slot_of(u))
            .collect();
        let n = named.len();
        self.close_project_window();
        self.selection.set(named);
        self.say(
            kentos_interaction::Level::Info,
            format!("Katı anan {n} nesne seçildi: Öznitelikler › Kat'tan onlara başka kat verin, sonra katı kaldırın."),
        );
    }
}
