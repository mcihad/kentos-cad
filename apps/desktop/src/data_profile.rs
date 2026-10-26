//! Veri profili (docs/adr/0541, 0558; plan/gis.md CBS-34): a window with a data profile, where the drawing's data departs
//! from it (`cad.profiles.check`, read again after every write) and what Uygula would add (`cad.profiles.apply`'s plan).
//! Uygula writes it: the layers and fields as one undo step “Veri profili”, the value lists, relations and address data
//! as project settings; the findings are read again and the window stays. A finding's row selects its objects.
//!
//! The profile is one of KentOS's or of the project's own (the files of the `profiller` folder beside the drawing);
//! İçe aktar… brings a profile or a mapping JSON into that folder (`cad.profiles.import`). A mapping makes the profile
//! read with the institution's layer, field and code names (check and Uygula work through it; nothing is renamed). A
//! profile with later releases offers its data migration (`cad.profiles.migrate`): the release the data is at, what the
//! steps would do, and after it the values that did not fit.

use std::path::PathBuf;

use iced::widget::{Column, container, row};
use iced::{Center, Element, Fill, Task};
use kentos_commands::{
    CommandResult, DataProfile, ProfileFileKind, ProfileFinding, ProfileFindingKind,
    ProfileImported, ProfileListed, ProfilesApplied, ProfilesApply, ProfilesCheck, ProfilesImport,
    ProfilesList, ProfilesMigrate, ProfilesMigrated, ProfilesReport,
};
use kentos_interaction::Level;
use kentos_native_application::{ExecutionContext, profile_files, profiles};
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::{Tokens, typography};
use kentos_ui::widget::{Choice, Dialog as Frame, Elided, Select, VirtualList, overlay};
use kentos_ui::{label, style};

use crate::app::{App, Dialog, Message};
use crate::exchange::words::{self, Kind};
use crate::traces::Control;

/// The window's title, which a trace names it by.
pub const PROFILE_TITLE: &str = "Veri profili";
const APPLY: &str = "Uygula";
const CLOSE: &str = "Kapat";
const IMPORT: &str = "İçe aktar…";
const MIGRATE: &str = "Veriyi taşı";
const OVERWRITE: &str = "Aynı adlı dosyanın üzerine yaz";
const PROFILE_LIST: &str = "Profil";
const MAPPING_LIST: &str = "Eşleme";
const FROM_LIST: &str = "Verinin sürümü";
const NO_MAPPING: &str = "Eşleme yok";
const ROW: f32 = 26.0;
/// The most loss lines the window says; the rest is counted.
const LOSS_LINES: usize = 4;

#[derive(Debug, Clone)]
pub struct Window {
    /// The profile's name: a built-in's or a file's of the project.
    pub profile: String,
    /// The mapping's name the profile is read through, when one is chosen.
    mapping: Option<String>,
    /// The built-in profiles and the project's files, read again with the findings.
    listed: Vec<ProfileListed>,
    /// The release the data is at: where a migration starts.
    from: u32,
    /// İçe aktar writes over a file of the same name.
    overwrite: bool,
    report: Result<ProfilesReport, String>,
    preview: Result<ProfilesApplied, String>,
    /// What the migration would do; an empty error: the profile has no later release.
    migration: Result<ProfilesMigrated, String>,
    /// What the last migration did: its losses are said until the window changes profile.
    last: Option<ProfilesMigrated>,
    /// The finding pressed: its whole message shown under the table.
    chosen: Option<usize>,
}

impl Window {
    /// The profiles that can be chosen: the ones that read.
    fn profiles(&self) -> Vec<&ProfileListed> {
        self.listed
            .iter()
            .filter(|p| p.kind == ProfileFileKind::Profile && p.problem.is_none())
            .collect()
    }

    /// The mappings of the chosen profile that hold.
    fn mappings(&self) -> Vec<&ProfileListed> {
        self.listed
            .iter()
            .filter(|m| {
                m.kind == ProfileFileKind::Mapping
                    && m.problem.is_none()
                    && m.profile.as_deref() == Some(self.profile.as_str())
            })
            .collect()
    }
}

#[derive(Debug, Clone)]
pub enum Event {
    /// A finding's row pressed: its objects selected and shown.
    Pick(usize),
    Apply,
    Close,
    /// The n-th profile of the list chosen.
    Profile(usize),
    /// The n-th mapping chosen, 0 being none.
    Mapping(usize),
    /// The release the data is at, chosen.
    From(u32),
    Overwrite(bool),
    /// İçe aktar… pressed: a file asked for.
    Import,
    Imported(Option<PathBuf>),
    /// The data moved to the profile's release.
    Migrate,
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

/// A profile in the list: its title, and for the project's own whose it is and its release.
fn profile_label(p: &ProfileListed) -> String {
    match p.source {
        kentos_commands::ProfileSource::Builtin => p.title.clone(),
        kentos_commands::ProfileSource::Project => {
            format!("{} (projenin, sürüm {})", p.title, p.release)
        }
    }
}

impl App {
    /// Veri profili (`profile.window`): the window on the first built-in profile.
    pub(crate) fn open_data_profile(&mut self) {
        if self.document.is_none() {
            self.output("Açık çizim yok.");
            return;
        }
        let profile = kentos_commands::BUILTIN_PROFILES
            .first()
            .map(|(n, _)| (*n).to_owned())
            .unwrap_or_default();
        self.data_profile = Some(Window {
            profile,
            mapping: None,
            listed: Vec::new(),
            from: 1,
            overwrite: false,
            report: Err(String::new()),
            preview: Err(String::new()),
            migration: Err(String::new()),
            last: None,
            chosen: None,
        });
        self.dialog = Some(Dialog::DataProfile);
        self.refresh_profile();
    }

    /// The open drawing's own file, an absolute path: the project's `profiller` folder is beside it.
    fn profile_drawing(&self) -> Option<String> {
        let path = self.document.as_ref()?.path.as_ref()?;
        std::path::absolute(path)
            .ok()
            .map(|p| p.display().to_string())
    }

    /// The list, the findings, the preview and the migration's plan read again.
    fn refresh_profile(&mut self) {
        let drawing = self.profile_drawing();
        let locked = self.tree_locked().map(str::to_owned);
        let (Some(w), Some(doc)) = (self.data_profile.as_mut(), self.document.as_ref()) else {
            return;
        };
        w.listed = profile_files::list::run(&ProfilesList {
            drawing: drawing.clone(),
        })
        .items;
        // What was chosen is still there: a profile, and a mapping of it.
        let names: Vec<String> = w.profiles().iter().map(|p| p.name.clone()).collect();
        if !names.contains(&w.profile)
            && let Some(first) = names.first()
        {
            w.profile.clone_from(first);
        }
        let maps: Vec<String> = w.mappings().iter().map(|m| m.name.clone()).collect();
        if w.mapping.as_ref().is_some_and(|m| !maps.contains(m)) {
            w.mapping = None;
        }
        let profile = profile_files::find_profile(drawing.as_deref(), &w.profile);
        let release = profile.as_ref().map_or(1, |p| p.release);
        w.from = w.from.clamp(1, release.saturating_sub(1).max(1));
        let report = match profiles::check::run(
            &doc.model,
            &ProfilesCheck {
                name: Some(w.profile.clone()),
                profile: None,
                mapping: None,
                mapping_name: w.mapping.clone(),
                drawing: drawing.clone(),
            },
        ) {
            CommandResult::Completed { output, .. } => Ok(output),
            CommandResult::Failed { error }
            | CommandResult::Conflict { error }
            | CommandResult::NeedsInput { error } => Err(error.message),
            _ => Err("Profil denetlenemedi.".to_owned()),
        };
        let preview = if let Some(locked) = locked {
            Err(locked)
        } else {
            match profiles::apply::plan_in(
                &doc.model,
                &apply_input(&w.profile, w.mapping.clone(), drawing.clone()),
            ) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Profil planlanamadı.".to_owned()),
            }
        };
        let migration = if release > 1 {
            match profile_files::migrate::plan_in(
                &doc.model,
                &migrate_input(&w.profile, w.mapping.clone(), w.from, drawing),
            ) {
                CommandResult::Completed { output, .. } => Ok(output),
                CommandResult::Failed { error }
                | CommandResult::Conflict { error }
                | CommandResult::NeedsInput { error } => Err(error.message),
                _ => Err("Göç planlanamadı.".to_owned()),
            }
        } else {
            Err(String::new())
        };
        let count = report.as_ref().map_or(0, |r| r.findings.len());
        w.chosen = w.chosen.filter(|i| *i < count);
        w.report = report;
        w.preview = preview;
        w.migration = migration;
    }

    pub(crate) fn data_profile_event(&mut self, event: Event) -> Task<Message> {
        let Some(w) = self.data_profile.as_mut() else {
            return Task::none();
        };
        match event {
            Event::Pick(i) => {
                w.chosen = Some(i);
                let finding = w
                    .report
                    .as_ref()
                    .ok()
                    .and_then(|r| r.findings.get(i))
                    .cloned();
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
            Event::Profile(i) => {
                let name = w.profiles().get(i).map(|p| p.name.clone());
                if let Some(name) = name
                    && name != w.profile
                {
                    w.profile = name;
                    w.mapping = None;
                    w.from = 1;
                    w.last = None;
                    w.chosen = None;
                }
            }
            Event::Mapping(i) => {
                w.mapping = i
                    .checked_sub(1)
                    .and_then(|k| w.mappings().get(k).map(|m| m.name.clone()));
                w.chosen = None;
            }
            Event::From(r) => {
                w.from = r;
                w.last = None;
            }
            Event::Overwrite(on) => {
                w.overwrite = on;
                return Task::none();
            }
            Event::Import => return self.import_dialog(),
            Event::Imported(None) => return Task::none(),
            Event::Imported(Some(path)) => {
                self.import_profile(&path);
            }
            Event::Migrate => {
                self.migrate_data();
            }
        }
        self.refresh_profile();
        Task::none()
    }

    /// The file to bring: a profile's or a mapping's JSON.
    fn import_dialog(&self) -> Task<Message> {
        Task::perform(
            async {
                let file = rfd::AsyncFileDialog::new()
                    .set_title("Veri profili ya da eşleme")
                    .add_filter("Veri profili ya da eşleme (.json)", &["json"])
                    .pick_file()
                    .await?;
                Some(file.path().to_path_buf())
            },
            |path| msg(Event::Imported(path)),
        )
    }

    /// `cad.profiles.import` of a file: what it brought said, and chosen in the window.
    fn import_profile(&mut self, path: &std::path::Path) {
        let drawing = self.profile_drawing();
        let overwrite = self.data_profile.as_ref().is_some_and(|w| w.overwrite);
        let Some(doc) = self.document.as_mut() else {
            return;
        };
        let result = profile_files::import::execute(
            &mut ExecutionContext::new(&mut doc.model),
            ProfilesImport {
                source: Some(path.display().to_string()),
                drawing,
                overwrite: Some(overwrite),
                ..ProfilesImport::default()
            },
        );
        let Some(done) = self.answered(result) else {
            return;
        };
        self.say(Level::Success, imported_words(&done));
        // The list is read again, then what came in is chosen.
        self.refresh_profile();
        let Some(w) = self.data_profile.as_mut() else {
            return;
        };
        match done.kind {
            ProfileFileKind::Profile => {
                w.profile = done.name;
                w.mapping = None;
                w.from = 1;
                w.last = None;
            }
            ProfileFileKind::Mapping => {
                let of = w
                    .listed
                    .iter()
                    .find(|m| m.kind == ProfileFileKind::Mapping && m.name == done.name)
                    .and_then(|m| m.profile.clone());
                if let Some(of) = of
                    && w.profiles().iter().any(|p| p.name == of)
                {
                    w.profile = of;
                    w.mapping = Some(done.name);
                }
            }
        }
    }

    /// `cad.profiles.migrate`: the data moved from the chosen release; what it did said, its losses kept in the window.
    pub(crate) fn migrate_data(&mut self) -> bool {
        let Some(w) = self.data_profile.as_ref() else {
            return false;
        };
        let input = migrate_input(
            &w.profile,
            w.mapping.clone(),
            w.from,
            self.profile_drawing(),
        );
        if let Some(locked) = self.tree_locked() {
            self.warn(locked);
            return false;
        }
        let Some(doc) = self.document.as_mut() else {
            return false;
        };
        let result =
            profile_files::migrate::execute(&mut ExecutionContext::new(&mut doc.model), input);
        let Some(done) = self.answered(result) else {
            return false;
        };
        self.say(Level::Success, migrated_words(&done));
        if let Some(w) = self.data_profile.as_mut() {
            w.last = Some(done);
        }
        true
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
        let drawing = self.profile_drawing();
        let Some(input) = self
            .data_profile
            .as_ref()
            .map(|w| apply_input(&w.profile, w.mapping.clone(), drawing))
        else {
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
            return iced::widget::text("").into();
        };
        let drawing = self.profile_drawing();
        let profile = profile_files::find_profile(drawing.as_deref(), &w.profile);
        let about = profile.as_ref().map_or_else(String::new, |p| {
            format!("{} (sürüm {}): {}", p.title, p.release, p.description)
        });
        let basis = profile
            .as_ref()
            .map_or_else(String::new, |p| p.basis.clone());
        let (lines, findings): (Vec<Element<'_, Message>>, Vec<ProfileFinding>) = match &w.report {
            Err(why) if why.is_empty() => (Vec::new(), Vec::new()),
            Err(why) => (vec![words::text_line(Kind::Warn, why.clone())], Vec::new()),
            Ok(r) if r.clean => (
                vec![words::text_line(
                    Kind::Ok,
                    format!(
                        "Çizimin verisi {} ile ayrılmıyor (bu, mevzuata uygunluk değildir).",
                        r.title
                    ),
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
            let objects = if f.count > 0 {
                f.count.to_string()
            } else {
                String::new()
            };
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
        .height(typography::scaled(150.0));
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
        let (migration, can_migrate) = self.migration_part(w, profile.as_ref());
        let mut body = Column::new()
            .spacing(12)
            .push(self.profile_pickers(w))
            .push(label::caption(about))
            .push(label::caption(basis))
            .push(words::summary(lines))
            .push(words::field("Bulgular", table, Some(hint)))
            .push(words::summary(preview));
        if let Some(m) = migration {
            body = body.push(m);
        }
        overlay::modal(
            Frame::new(PROFILE_TITLE)
                .scroll(body)
                .action(words::secondary(CLOSE, Some(msg(Event::Close))))
                .action(words::secondary(
                    MIGRATE,
                    can_migrate.then(|| msg(Event::Migrate)),
                ))
                .action(words::primary(APPLY, ready.then(|| msg(Event::Apply))))
                .width(820.0)
                .max_height(760.0),
            msg(Event::Close),
        )
    }

    /// The profile and the mapping to choose, and İçe aktar….
    fn profile_pickers<'a>(&self, w: &'a Window) -> Element<'a, Message> {
        let profiles = w.profiles();
        let profile_select = Select::new(
            profiles.iter().map(|p| Choice::new(profile_label(p))),
            profiles.iter().position(|p| p.name == w.profile),
            |i| msg(Event::Profile(i)),
        )
        .searchable(false);
        let mappings = w.mappings();
        let mapping_select = Select::new(
            std::iter::once(Choice::new(NO_MAPPING))
                .chain(mappings.iter().map(|m| Choice::new(m.title.clone()))),
            Some(
                w.mapping
                    .as_ref()
                    .and_then(|n| mappings.iter().position(|m| &m.name == n))
                    .map_or(0, |k| k + 1),
            ),
            |i| msg(Event::Mapping(i)),
        )
        .searchable(false);
        let drawing = self.profile_drawing().is_some();
        let hint = if drawing {
            "Profil ve eşleme çizimin yanındaki “profiller” klasöründedir; eşleme profili kurumun katman, alan ve kod adlarıyla okutur, hiçbir ad kendiliğinden değişmez."
        } else {
            "Çizim henüz bir dosya değil: kurumun profilini ve eşlemesini almak için önce çizimi kaydedin."
        };
        let bring = row![
            words::ghost(IMPORT, drawing.then(|| msg(Event::Import))),
            words::check(
                w.overwrite,
                OVERWRITE,
                Some(msg(Event::Overwrite(!w.overwrite)))
            ),
        ]
        .spacing(12)
        .align_y(Center);
        Column::new()
            .spacing(6)
            .push(
                row![
                    container(words::field(PROFILE_LIST, profile_select, None)).width(Fill),
                    container(words::field(MAPPING_LIST, mapping_select, None)).width(Fill),
                ]
                .spacing(12),
            )
            .push(bring)
            .push(label::caption(hint))
            .into()
    }

    /// The migration's part: the release the data is at, what the steps would do and what the last one did; whether
    /// it can be run. None for a profile that has no later release.
    fn migration_part<'a>(
        &self,
        w: &'a Window,
        profile: Option<&DataProfile>,
    ) -> (Option<Element<'a, Message>>, bool) {
        let release = profile.map_or(1, |p| p.release);
        if release <= 1 {
            return (None, false);
        }
        let from_select = Select::new(
            (1..release).map(|r| Choice::new(format!("{r}. sürüm"))),
            Some((w.from.saturating_sub(1)) as usize),
            |i| msg(Event::From(i as u32 + 1)),
        )
        .searchable(false);
        let mut lines: Vec<Element<'a, Message>> = Vec::new();
        let mut can = false;
        match &w.migration {
            Err(why) if why.is_empty() => {}
            Err(why) => lines.push(words::text_line(Kind::Warn, why.clone())),
            Ok(p) if !p.changed => lines.push(words::text_line(
                Kind::Info,
                "Verinin taşınacak bir yeri yok: adımlar uygulanmış ya da çizimde olmayan katman ve alanlara ait.".to_owned(),
            )),
            Ok(p) => {
                can = true;
                lines.push(words::text_line(Kind::Info, planned_migration_words(p)));
            }
        }
        if let Some(last) = &w.last {
            lines.push(words::text_line(Kind::Ok, migrated_words(last)));
            for l in last.losses.iter().take(LOSS_LINES) {
                lines.push(words::text_line(Kind::Warn, l.message.clone()));
            }
            if last.losses.len() > LOSS_LINES {
                lines.push(words::text_line(
                    Kind::Warn,
                    format!(
                        "… ve {} değer daha (cevabın losses listesinde).",
                        last.losses.len() - LOSS_LINES
                    ),
                ));
            }
        }
        let body = row![
            container(from_select).width(typography::scaled(150.0)),
            container(words::summary(lines)).width(Fill),
        ]
        .spacing(12);
        (
            Some(words::field(
                FROM_LIST,
                body,
                Some("Veri profilin hangi sürümündeyse onu seçin: göç oradan profilin sürümüne kadar profilin kendi adımlarıyla yürür. Katman adları geri alınmaz; alanlar, değerler ve listeler tek adımda geri alınır.".to_owned()),
            )),
            can,
        )
    }

    /// The first finding of a kind, by its place in the window's table (the pictures press it).
    #[cfg(test)]
    pub(crate) fn data_profile_finding(&self, kind: &str) -> Option<usize> {
        let r = self.data_profile.as_ref()?.report.as_ref().ok()?;
        r.findings.iter().position(|f| {
            serde_json::to_value(f.kind)
                .ok()
                .and_then(|v| v.as_str().map(str::to_owned))
                .as_deref()
                == Some(kind)
        })
    }

    /// The window's controls by their words (a trace's `dialog` step).
    pub(crate) fn data_profile_control(
        &self,
        control: Control<'_>,
    ) -> Result<Option<Message>, String> {
        let Some(w) = &self.data_profile else {
            return Err(format!("{PROFILE_TITLE} penceresi açık değil"));
        };
        Ok(match control {
            Control::Press(APPLY) => Some(msg(Event::Apply)),
            Control::Press(CLOSE) => Some(msg(Event::Close)),
            Control::Press(IMPORT) => Some(msg(Event::Import)),
            Control::Press(MIGRATE) => Some(msg(Event::Migrate)),
            Control::Check(OVERWRITE, on) => Some(msg(Event::Overwrite(on))),
            Control::Pick(PROFILE_LIST, item) => w
                .profiles()
                .iter()
                .position(|p| profile_label(p) == item || p.name == item)
                .map(|i| msg(Event::Profile(i))),
            Control::Pick(MAPPING_LIST, item) if item == NO_MAPPING => Some(msg(Event::Mapping(0))),
            Control::Pick(MAPPING_LIST, item) => w
                .mappings()
                .iter()
                .position(|m| m.title == item || m.name == item)
                .map(|i| msg(Event::Mapping(i + 1))),
            Control::Pick(FROM_LIST, item) => item
                .trim_end_matches(". sürüm")
                .parse::<u32>()
                .ok()
                .map(|r| msg(Event::From(r))),
            other => return Err(format!("“{PROFILE_TITLE}” penceresinde {other} yok")),
        })
    }
}

fn apply_input(name: &str, mapping: Option<String>, drawing: Option<String>) -> ProfilesApply {
    ProfilesApply {
        name: Some(name.to_owned()),
        profile: None,
        mapping: None,
        mapping_name: mapping,
        drawing,
        expected_revision: None,
    }
}

fn migrate_input(
    name: &str,
    mapping: Option<String>,
    from: u32,
    drawing: Option<String>,
) -> ProfilesMigrate {
    ProfilesMigrate {
        name: Some(name.to_owned()),
        profile: None,
        from,
        to: None,
        drawing,
        expected_revision: None,
        mapping: None,
        mapping_name: mapping,
    }
}

/// The report in a sentence.
fn report_words(r: &ProfilesReport) -> String {
    let n = r.findings.len();
    let fixable = r.findings.iter().filter(|f| f.fixable).count();
    let objects: u32 = r.classes.iter().map(|c| c.objects).sum();
    let through = r
        .mapping
        .as_ref()
        .map_or_else(String::new, |m| format!(" (“{m}” eşlemesiyle)"));
    let head = format!(
        "{} sınıfta {objects} nesne{through}; {n} bulgu",
        r.classes.len()
    );
    match fixable {
        0 => format!("{head}; hiçbiri Uygula ile düzelmez, verinin kendisinde düzeltin."),
        f if f == n => format!("{head}; hepsini Uygula ekler."),
        f => format!(
            "{head}; {f} tanesini Uygula ekler, {} tanesi veride düzeltilir.",
            n - f
        ),
    }
}

/// What the plan would do, in a sentence.
fn planned_words(p: &ProfilesApplied) -> String {
    let mut parts = Vec::new();
    if !p.layers.is_empty() {
        parts.push(format!("{} katman ve grup eklenir", p.layers.len()));
    }
    if !p.fields.is_empty() {
        parts.push(format!(
            "var olan katmanlara {} alan eklenir",
            p.fields.len()
        ));
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

/// What a file that came in is, in a sentence.
fn imported_words(p: &ProfileImported) -> String {
    match p.kind {
        ProfileFileKind::Profile => format!(
            "“{}” profili projeye alındı (sürüm {}, {} sınıf, {} göç): {}",
            p.title, p.release, p.classes, p.migrations, p.path
        ),
        ProfileFileKind::Mapping => format!(
            "“{}” eşlemesi projeye alındı ({} sınıf): {}",
            p.title, p.classes, p.path
        ),
    }
}

/// What the migration's plan would do, in a sentence.
fn planned_migration_words(p: &ProfilesMigrated) -> String {
    let applied = p.steps.iter().filter(|s| s.applied).count();
    let unfit: u32 = p.losses.iter().map(|l| l.count).sum();
    let mut text = format!(
        "{}. sürümden {}. sürüme: {applied} adım uygulanır, {} nesneye yazılır",
        p.from, p.to, p.objects
    );
    if unfit > 0 {
        text.push_str(&format!(
            "; {unfit} nesnenin değeri yeni kurala uymuyor, olduğu gibi kalır ya da boşaltılır ve değer değer raporlanır"
        ));
    }
    text.push('.');
    text
}

/// What the migration did, in a sentence.
fn migrated_words(p: &ProfilesMigrated) -> String {
    let applied = p.steps.iter().filter(|s| s.applied).count();
    let unfit: u32 = p.losses.iter().map(|l| l.count).sum();
    let mut text = format!(
        "Veri {}. sürümden {}. sürüme taşındı: {applied} adım uygulandı, {} nesneye yazıldı",
        p.from, p.to, p.objects
    );
    if unfit > 0 {
        text.push_str(&format!("; {unfit} nesnenin değeri yeni kurala uymadı"));
    }
    text.push('.');
    text
}
