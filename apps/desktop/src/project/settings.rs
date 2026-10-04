//! Dosya → Proje ayarları (the web's `ui/settings/ProjectSettingsDialog.ts`):
//! stored in the project file, shared by everyone who opens it. The window
//! works on a draft: Kaydet assigns it (an edit, not an undo step), Vazgeç
//! leaves the drawing as it was. A new coordinate system is assigned, never
//! a transformation of the coordinates (CLAUDE.md §5).

use std::fmt;

use iced::widget::{Column, button, column, container, row, scrollable, text, text_input};
use iced::{Element, Fill, Task};
use kentos_contracts::{AngleUnit, AreaUnit, DrawingFont, DrawingUnit, ProjectSettings, Workspace};
use kentos_interaction::{Format, Level};
use kentos_ui::theme::typography;
use kentos_ui::widget::number::NumberInput;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Dialog, overlay};
use kentos_ui::{label, style};

use super::{Event as ProjectEvent, Window, group, message, modes, scales, setting};
use crate::app::{App, Message};
use crate::crs;
use crate::document::Document;
use crate::exchange::words;

/// The window's sections, as the web's navigation lists them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Section {
    General,
    Crs,
    Units,
}

impl Section {
    const ALL: [Section; 3] = [Section::General, Section::Crs, Section::Units];

    fn label(self) -> &'static str {
        match self {
            Section::General => "Genel",
            Section::Crs => "Koordinat sistemi",
            Section::Units => "Birimler ve hassasiyet",
        }
    }

    /// Its icon in the list, as the web's.
    fn icon(self) -> kentos_ui::icon::Icon {
        use kentos_ui::icon::Icon;
        match self {
            Section::General => Icon::Properties,
            Section::Crs => Icon::Globe,
            Section::Units => Icon::Ruler,
        }
    }

    fn lead(self) -> &'static str {
        match self {
            Section::General => {
                "Projenin adı, türü, çizim ölçeği ve çizimdeki yazıların yazı tipi."
            }
            Section::Crs => {
                "Bu projenin konum referansı. Koordinatlar bu sistemde saklanır, ölçülür ve dışa aktarılır."
            }
            Section::Units => {
                "Bu projede panellerde, komut satırında ve ölçüm etiketlerinde sayıların nasıl gösterileceği."
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

impl fmt::Display for Angle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self.0 {
            AngleUnit::Grad => "Grad",
            AngleUnit::Deg => "Derece",
        })
    }
}

#[derive(Debug, Clone)]
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
}

#[derive(Debug, Clone)]
pub enum Event {
    Section(Section),
    Name(String),
    Scale(f64),
    Mode(Workspace),
    Font(DrawingFont),
    Crs(u32),
    Search(String),
    LengthDecimals(u32),
    AreaDecimals(u32),
    AreaUnit(AreaUnit),
    AngleUnit(AngleUnit),
    /// A local project's drawing unit (docs/adr/0165 §2); metres are kept as none.
    DrawingUnit(DrawingUnit),
    /// İkinci koordinat sistemi (docs/adr/0167 §1); none: Yok. Either takes
    /// the place of a second definition (docs/adr/0168 §1).
    Second(Option<u32>),
    /// The second definition the project has, kept.
    SecondDefined,
    /// Kaldır on a grid's row: asked in the row; none: Vazgeç.
    GridAsk(Option<String>),
    /// Izgaralar's library (grids.rs).
    Grid(crate::grids::Event),
    /// The shown section's own values back to their defaults (the web's
    /// “Bu bölümü varsayılana döndür”).
    ResetSection,
    /// Uygulama ayarları over this window, which comes back when it closes.
    OpenApp,
    Save,
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
        }
    }
}

impl App {
    pub(super) fn project_settings_event(&mut self, e: Event) -> Task<Message> {
        if let Event::Save = e {
            self.save_project_settings();
            return Task::none();
        }
        if let Event::OpenApp = e {
            // This window waits under Uygulama ayarları, as it is (app.rs `dialog_back`).
            self.open_settings_at(crate::settings_sections::Section::NewProjects);
            self.dialog_under = Some(crate::app::Dialog::Project);
            return Task::none();
        }
        let Some(Window::Settings(s)) = &mut self.project else {
            return Task::none();
        };
        if let Event::Grid(e) = e {
            s.removing = None;
            return self.grid_event(e);
        }
        let d = &mut s.settings;
        match e {
            Event::Section(section) => s.section = section,
            Event::Name(name) => s.name = name,
            Event::Scale(v) => d.plot_scale = v,
            Event::Mode(w) => d.workspace = Some(w),
            Event::Font(f) => d.drawing_font = Some(f),
            Event::Crs(srid) => d.srid = srid,
            Event::Search(query) => {
                let digits: String = query.chars().filter(char::is_ascii_digit).collect();
                if let Ok(srid) = digits.parse::<u32>()
                    && digits.len() >= 4
                    && crs::system(srid).is_some()
                {
                    d.srid = srid;
                }
                s.query = query;
            }
            Event::Second(second) => {
                d.second_srid = second;
                d.second_custom_crs = None;
            }
            Event::SecondDefined => {}
            Event::GridAsk(id) => s.removing = id,
            Event::Grid(_) => {}
            Event::LengthDecimals(n) => d.length_decimals = n.min(4),
            Event::AreaDecimals(n) => d.area_decimals = n.min(4),
            Event::AreaUnit(u) => d.area_unit = u,
            Event::AngleUnit(u) => d.angle_unit = u,
            Event::DrawingUnit(u) => d.drawing_unit = (u != DrawingUnit::M).then_some(u),
            Event::ResetSection => reset_section(s.section, d),
            Event::OpenApp | Event::Save => {}
        }
        // The project's own system, or none, is no second system (docs/adr/0167 §1).
        d.second_srid = d.second();
        Task::none()
    }

    /// Kaydet: the draft goes into the drawing (the web's `onSave`).
    fn save_project_settings(&mut self) {
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
        // A drawing unit is a local project's (docs/adr/0165 §2): one given a coordinate system is in metres.
        if settings.has_system() {
            settings.drawing_unit = None;
        }
        doc.model.set_settings(settings);
        let assigned = (s.settings.srid != s.initial.srid).then_some(s.settings.srid);
        let second = doc.settings().second_srid;
        let defined = doc.settings().second_custom_crs.as_ref();
        let second_changed =
            (second, defined) != (s.initial.second_srid, s.initial.second_custom_crs.as_ref());
        let defined = defined.map(kentos_project::systems::definition_title);
        self.close_project_window();
        if let Some(srid) = assigned {
            self.say(
                Level::Success,
                format!(
                    "Proje koordinat sistemi {} olarak atandı. Koordinat değerleri değiştirilmedi.",
                    crs::title_of(srid)
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
        self.say(
            Level::Success,
            "Proje ayarları kaydedildi. Proje dosyasıyla birlikte saklanacak.",
        );
    }

    pub(super) fn project_settings_view<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let Some(doc) = &self.document else {
            return text("").into();
        };
        let nav = Section::ALL
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
                    .style(style::button::navigation(*section == s.section)),
                )
            });
        let page: Element<'a, Message> = match s.section {
            Section::General => self.general(s, doc),
            Section::Crs => self.crs_section(s),
            Section::Units => units(s),
        };
        let content = column![
            text(s.section.label())
                .font(typography::ui_strong())
                .size(typography::body()),
            label::caption(s.section.lead()),
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
        let own = s.section != Section::Crs;
        let reset = button(label::body("Bu bölümü varsayılana döndür"))
            .on_press_maybe(own.then_some(event(Event::ResetSection)))
            .padding([5, 16])
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
                .action(words::primary("Kaydet", Some(event(Event::Save))))
                .width(900.0)
                .max_height(760.0),
        )
    }

    fn general<'a>(&'a self, s: &'a State, doc: &'a Document) -> Element<'a, Message> {
        let name = text_input("Proje adı", &s.name)
            .on_input(|t| event(Event::Name(t)))
            .padding([5, 8])
            .size(typography::body())
            .width(260)
            .style(style::field::input);
        let name = kentos_ui::widget::focus_ring(name);
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
                    .push(setting("Proje adı", Some("Dosya adı olarak da kullanılır."), name))
                    .push(setting(
                        "Çizim ölçeği",
                        Some("Yazı yükseklikleri ve pafta çıktıları bu ölçeğe göre hesaplanır."),
                        scales(s.settings.plot_scale, |v| event(Event::Scale(v))),
                    )),
            ))
            .push(group(
                "Proje türü",
                column![
                    label::caption("CAD ya da CBS: sahnesi, eksen ve açı düzeni ve şeridi türe göredir; veri değişmez. Şeritte olmayan komutlar komut satırından yine çalışır."),
                    modes(
                        crate::catalog::effective_mode(s.settings.workspace).id,
                        true,
                        false,
                        |w| event(Event::Mode(w)),
                    ),
                ]
                .spacing(8),
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
            ),
            second_group(s),
            self.grids_group(s),
            group(
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
            ),
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
                        .padding([5, 16])
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
            list = list.push(label::caption("Bu cihazda NTv2 ızgarası yok."));
        }
        let add = words::secondary("Ekle…", Some(event(Event::Grid(crate::grids::Event::Add))));
        group(
            "Izgaralar",
            column![
                label::caption(
                    "NTv2 ızgaraları bu cihazda saklanır, projeyle paylaşılmaz: projenin datum seçimi onları SHA-256'larıyla anar, projeyi açan başka cihaza da eklenmeleri gerekir."
                ),
                list,
                row![add],
            ]
            .spacing(10),
        )
    }
}

/// İkinci koordinat sistemi (docs/adr/0167 §1): the registry's systems but
/// the project's own, grouped by datum, and Yok (the web's `secondGroup`).
/// Its values show beside the project's in the status bar and Koordinat
/// oku; the drawing is not transformed.
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
    // Each row's choice: Yok, the project's second definition (docs/adr/0168 §1), a datum's
    // header, a system of the registry.
    #[derive(Clone, Copy, PartialEq)]
    enum Pick {
        Yok,
        Defined,
        Header,
        Srid(u32),
    }
    let defined = d
        .second_custom_crs
        .as_ref()
        .filter(|_| d.second().is_none());
    let mut rows = vec![Pick::Yok];
    let mut list = vec![Choice::new("Yok")];
    if let Some(def) = defined {
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
    let current = match (d.second(), defined) {
        (Some(srid), _) => Pick::Srid(srid),
        (None, Some(_)) => Pick::Defined,
        (None, None) => Pick::Yok,
    };
    let selected = rows.iter().position(|r| *r == current);
    let pick = Select::new(list, selected, move |i| {
        event(match rows.get(i) {
            Some(Pick::Srid(srid)) => Event::Second(Some(*srid)),
            Some(Pick::Defined) => Event::SecondDefined,
            _ => Event::Second(None),
        })
    });
    group(
        "İkinci koordinat sistemi",
        setting("İkinci sistem", Some(what), container(pick).width(300)),
    )
}

fn units<'a>(s: &'a State) -> Element<'a, Message> {
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
        ],
        Unit(d.drawing_unit.unwrap_or_default()),
        |u| event(Event::DrawingUnit(u.0)),
    );
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
    let lengths = lengths.push(setting(
        "Ondalık basamak",
        Some("Koordinatlar, kenar uzunlukları ve mesafeler."),
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
        .push(group("Alan", areas))
        .push(group(
            "Açı",
            setting("Açı birimi", Some("Semt açıları kuzeyden saat yönünde ölçülür."), angle),
        ))
        .push(preview)
        .push(Banner::info(
            "Ondalık ayırıcı her zaman noktadır; komut satırına aynı biçimde yazılabilir (Y,X virgülle ayrılır).",
        ))
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
            d.plot_scale = default("project.plotScale")
                .and_then(|v| v.as_f64())
                .unwrap_or(1000.0);
            d.workspace = read("project.workspace");
            d.drawing_font = read("project.drawingFont");
        }
        Section::Units => {
            d.drawing_unit = None;
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
        Section::Crs => {}
    }
}
