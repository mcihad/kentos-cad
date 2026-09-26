//! Dosya → Proje ayarları (the web's `ui/settings/ProjectSettingsDialog.ts`):
//! stored in the project file, shared by everyone who opens it. The window
//! works on a draft: Kaydet assigns it (an edit, not an undo step), Vazgeç
//! leaves the drawing as it was. A new coordinate system is assigned, never
//! a transformation of the coordinates (CLAUDE.md §5).

use std::fmt;

use iced::widget::{Column, button, column, container, row, scrollable, text, text_input};
use iced::{Element, Fill, Length};
use kentos_contracts::{AngleUnit, AreaUnit, DrawingFont, ProjectSettings, Workspace};
use kentos_interaction::{Format, Level};
use kentos_ui::theme::typography;
use kentos_ui::widget::number::NumberInput;
use kentos_ui::widget::segmented::Segmented;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::widget::{Banner, Dialog, overlay};
use kentos_ui::{label, style};

use super::{Event as ProjectEvent, Window, group, message, modes, scales, setting};
use crate::app::{App, Message};
use crate::crs::{self, PickFor};
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

    fn lead(self) -> &'static str {
        match self {
            Section::General => {
                "Projenin adı, çalışma modu, çizim ölçeği ve çizimdeki yazıların yazı tipi."
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
    section: Section,
    name: String,
    settings: ProjectSettings,
    initial_name: String,
    initial: ProjectSettings,
    query: String,
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
        }
    }
}

impl App {
    pub(super) fn project_settings_event(&mut self, e: Event) {
        if let Event::Save = e {
            self.save_project_settings();
            return;
        }
        let Some(Window::Settings(s)) = &mut self.project else {
            return;
        };
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
            Event::LengthDecimals(n) => d.length_decimals = n.min(4),
            Event::AreaDecimals(n) => d.area_decimals = n.min(4),
            Event::AreaUnit(u) => d.area_unit = u,
            Event::AngleUnit(u) => d.angle_unit = u,
            Event::Save => {}
        }
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
        doc.model.set_settings(s.settings.clone());
        let assigned = (s.settings.srid != s.initial.srid).then_some(s.settings.srid);
        self.close_project_window();
        if let Some(srid) = assigned {
            let system =
                crs::system(srid).map_or_else(|| format!("EPSG:{srid}"), |c| c.name.clone());
            self.say(
                Level::Success,
                format!(
                    "Proje koordinat sistemi {system} (EPSG:{srid}) olarak atandı. Koordinat değerleri değiştirilmedi."
                ),
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
                    button(label::body(section.label()))
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
            container(scrollable(page).height(Length::Shrink)).max_height(560),
        ]
        .spacing(10)
        .width(Fill);
        let scope = row![
            label::caption("Proje dosyasına kaydedilir:"),
            label::caption(doc.name().to_owned()),
        ]
        .spacing(6);
        overlay::blocking(
            Dialog::new("Proje ayarları")
                .push(column![row![nav, content].spacing(16), scope].spacing(12))
                .action(words::secondary(
                    "Vazgeç",
                    Some(message(ProjectEvent::Close)),
                ))
                .action(words::primary("Kaydet", Some(event(Event::Save))))
                .width(900.0),
        )
    }

    fn general<'a>(&'a self, s: &'a State, doc: &'a Document) -> Element<'a, Message> {
        let name = text_input("Proje adı", &s.name)
            .on_input(|t| event(Event::Name(t)))
            .padding([5, 8])
            .size(typography::body())
            .width(260)
            .style(style::field::input);
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
                "Çalışma modu",
                column![
                    label::caption("Hangi menülerin, şerit sekmelerinin ve araçların görüneceğini seçer; veriyi değiştirmez. Gizlenen komutlar komut satırından yine çalışır."),
                    modes(
                        s.settings.workspace.unwrap_or(Workspace::Hybrid),
                        true,
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
                        label::mono(format!("Y {:.0}  X {:.0}", origin.x, origin.y)),
                    )),
            ))
            .into()
    }

    fn crs_section<'a>(&'a self, s: &'a State) -> Element<'a, Message> {
        let default_srid = self.settings.number("newProjects.srid") as u32;
        let default = crs::system(default_srid).map(|c| {
            format!(
                "{} (EPSG:{}). Uygulama ayarlarından değiştirilir.",
                c.name, c.srid
            )
        });
        column![
            crs::picker(
                s.settings.srid,
                s.initial.srid,
                default_srid,
                PickFor::Assign,
                &s.query,
                |srid| event(Event::Crs(srid)),
                |q| event(Event::Search(q)),
            ),
            group(
                "Yeni projeler",
                setting(
                    "Yeni projelerin varsayılanı",
                    None,
                    label::caption(default.unwrap_or_default()),
                ),
            ),
        ]
        .spacing(16)
        .into()
    }
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
    Column::new()
        .spacing(16)
        .push(group(
            "Uzunluk ve koordinat",
            setting(
                "Ondalık basamak",
                Some("Koordinatlar, kenar uzunlukları ve mesafeler."),
                decimals(d.length_decimals, Event::LengthDecimals),
            ),
        ))
        .push(group(
            "Alan",
            Column::new()
                .spacing(10)
                .push(setting(
                    "Alan birimi",
                    Some("Parsel ve kapalı alanlarda gösterilen birim. Metrekare her zaman öznitelik panelinde de yer alır."),
                    area,
                ))
                .push(setting("Ondalık basamak", None, decimals(d.area_decimals, Event::AreaDecimals))),
        ))
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
