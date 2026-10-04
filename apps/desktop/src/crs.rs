//! “Bu koordinatlar hangi sistemde?” (the web's `CrsQuestion`,
//! apps/web/src/ui/io/common.ts). A file that says nothing of its system
//! (DXF, a coordinate list, a Shapefile without .prj) starts at the
//! project's system; one that says (GeoJSON, a .prj, a Netcad NCZ's MPROJ;
//! [`CrsQuestion::declare`])
//! starts at what it says, or at no choice when that could not be read: a
//! statement is never guessed into a system. Another system than the
//! project's blocks the import: datum and zone transformations do not exist
//! yet and coordinates are never reprojected silently (CLAUDE.md §5). The
//! user may say the file is in the project's system after all (a file that
//! claims WGS 84 but holds TM coordinates); the window then says what that
//! means. Where the extent is known, coordinates that cannot be in the
//! chosen system are pointed out. The systems are the web's registry
//! (fixtures/crs/v1/registry.json), in its order and grouped by datum as the
//! web's list is; a new project and the project settings choose from the
//! same list.

use iced::widget::{Column, button, column, container, row, scrollable, text, text_input};
use iced::{Center, Element, Fill, Length};
use kentos_contracts::{Bounds, CrsDefinition, CrsSource, DeclaredCrs, ProjectSettings};
use kentos_interaction::Format;
use kentos_ui::icon::{Icon, Tone, icon};
use kentos_ui::theme::typography;
use kentos_ui::widget::Banner;
use kentos_ui::widget::select::{Choice, Select};
use kentos_ui::{label, style};

pub use kentos_project::crs::{LOCAL_SRID, System, code, datum_label, system, systems, title};
use kentos_project::systems::{DEFINITION_CODE, definition_title};

/// The project's own definition, when its system is one (docs/adr/0168 §1).
fn own_definition(settings: &ProjectSettings) -> Option<&CrsDefinition> {
    settings
        .custom_crs
        .as_ref()
        .filter(|_| settings.srid == LOCAL_SRID)
}

/// The project's system's name: “TUREF / TM30”, its definition's “Şantiye”;
/// an unknown one by its EPSG code.
pub fn project_name(settings: &ProjectSettings) -> String {
    match own_definition(settings) {
        Some(d) => d.name.clone(),
        None => system(settings.srid)
            .map_or_else(|| format!("EPSG:{}", settings.srid), |s| s.name.clone()),
    }
}

/// The project's system as a sentence names it: “TUREF / TM30 (EPSG:5254)”,
/// “Şantiye (özel sistem)”, “Yerel (koordinat sistemi yok)”.
pub fn project_title(settings: &ProjectSettings) -> String {
    own_definition(settings).map_or_else(|| title_of(settings.srid), definition_title)
}

/// The project's system's code: “EPSG:5254”, “Özel sistem”, “SRID 0”.
pub fn project_code(settings: &ProjectSettings) -> String {
    own_definition(settings).map_or_else(|| code_of(settings.srid), |_| DEFINITION_CODE.to_owned())
}

/// The title of a project's system by its SRID; an unknown one by its EPSG code.
pub fn title_of(srid: u32) -> String {
    system(srid).map_or_else(|| format!("EPSG:{srid}"), title)
}

/// The code of a project's system by its SRID (Öznitelikler's SRID row).
pub fn code_of(srid: u32) -> String {
    system(srid).map_or_else(|| format!("EPSG:{srid}"), code)
}

/// Where a file's statement of its system comes from (the web's `CrsStatement.source`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Said {
    /// RFC 7946 GeoJSON: no `crs` member, so WGS 84 longitude, latitude.
    Rfc7946,
    /// A GeoJSON file's legacy `crs` member.
    GeoJsonCrs,
    /// A Shapefile's .prj.
    Prj,
    /// A Netcad NCZ's MPROJ and TILED_XML blocks (docs/adr/0138).
    Ncz,
    /// The file says nothing (a Shapefile without .prj).
    Nothing,
}

/// What a file says of its coordinate system (GeoJSON, Shapefile; docs/adr/0046).
#[derive(Debug, Clone, PartialEq)]
pub struct Statement {
    /// The EPSG code the statement names, when it could be read.
    pub srid: Option<u32>,
    /// The statement as the file writes it; empty when the file says nothing.
    pub text: String,
    pub source: Said,
}

impl Statement {
    /// What a reader found (the web's `statement`): a GeoJSON file that
    /// declares nothing is RFC 7946's; a Shapefile without .prj says nothing.
    pub fn of(declared: Option<&DeclaredCrs>, shapefile: bool) -> Self {
        let Some(d) = declared else {
            return Self {
                srid: None,
                text: String::new(),
                source: if shapefile {
                    Said::Nothing
                } else {
                    Said::Rfc7946
                },
            };
        };
        Self {
            srid: d.srid,
            text: d.text.clone(),
            source: match d.source {
                CrsSource::Rfc7946 => Said::Rfc7946,
                CrsSource::GeoJsonCrs => Said::GeoJsonCrs,
                CrsSource::Prj => Said::Prj,
                CrsSource::Ncz => Said::Ncz,
            },
        }
    }
}

/// A line under the list: a hint, or a note that informs or warns.
#[derive(Debug, Clone, PartialEq)]
pub enum Note {
    Hint(String),
    Info(String),
    Warn(String),
}

/// The project's system as the question sees it: its SRID and, when its
/// system is its own definition (then the SRID is 0, docs/adr/0168 §1), the
/// definition's title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub srid: u32,
    pub defined: Option<String>,
}

impl Project {
    pub fn of(settings: &ProjectSettings) -> Self {
        Self {
            srid: settings.srid,
            defined: own_definition(settings).map(definition_title),
        }
    }

    /// A project without a coordinate system: it takes any file's
    /// coordinates as they are (docs/adr/0165 §2).
    fn local(&self) -> bool {
        self.srid == LOCAL_SRID && self.defined.is_none()
    }

    /// Its system's name.
    fn name(&self) -> String {
        match &self.defined {
            Some(title) => title.clone(),
            None => {
                system(self.srid).map_or_else(|| format!("EPSG:{}", self.srid), |s| s.name.clone())
            }
        }
    }
}

impl From<u32> for Project {
    fn from(srid: u32) -> Self {
        Self {
            srid,
            defined: None,
        }
    }
}

/// The answer to the question: the source's system.
#[derive(Debug, Clone, PartialEq)]
pub struct CrsQuestion {
    /// The chosen system; none yet when the file's statement could not be read.
    pub srid: Option<u32>,
    statement: Option<Statement>,
    bounds: Option<Bounds>,
}

impl CrsQuestion {
    /// The project's system, the default answer.
    pub fn new(project: u32) -> Self {
        Self {
            srid: Some(project),
            statement: None,
            bounds: None,
        }
    }

    /// The user's answer.
    pub fn pick(&mut self, srid: u32) {
        self.srid = Some(srid);
    }

    /// What the file says of its system, and the extent of its coordinates:
    /// the choice starts at the statement's system (the project's when the
    /// file says nothing, none when the statement cannot be read).
    pub fn declare(&mut self, project: u32, statement: Option<Statement>, bounds: Option<Bounds>) {
        self.srid = match &statement {
            Some(st) if st.source != Said::Nothing => {
                st.srid.filter(|srid| system(*srid).is_some())
            }
            _ => Some(project),
        };
        self.statement = statement;
        self.bounds = bounds;
    }

    /// Whether the file can be imported: its system is the project's, or the
    /// project is local (no coordinate system), which takes any file's
    /// coordinates as they are (docs/adr/0165 §2). A project whose system is
    /// its own definition takes only coordinates said to be in it.
    pub fn matches(&self, project: &Project) -> bool {
        self.srid.is_some() && (self.srid == Some(project.srid) || project.local())
    }

    /// What the file said, as a line.
    fn said(&self) -> Option<String> {
        let st = self.statement.as_ref()?;
        let code = st
            .srid
            .map_or_else(String::new, |srid| format!(" (EPSG:{srid})"));
        Some(match (st.source, st.srid) {
            (Said::Rfc7946, _) => "Dosya: RFC 7946 GeoJSON, crs üyesi yok; koordinatları WGS 84 boylam, enlem (EPSG:4326) olmalı.".to_owned(),
            (Said::GeoJsonCrs, Some(_)) => format!("Dosyanın crs üyesi: “{}”{code}.", st.text),
            (Said::GeoJsonCrs, None) => format!(
                "Dosyanın crs üyesi okunamadı ({}); koordinatların sistemini siz seçin.",
                st.text
            ),
            (Said::Prj, Some(_)) => format!("Dosyanın .prj'si: “{}”{code}.", st.text),
            (Said::Prj, None) => format!(
                "Dosyanın .prj'si tanınmadı (“{}”); koordinatların sistemini siz seçin.",
                st.text
            ),
            (Said::Ncz, Some(_)) => format!("Dosyanın bildirdiği sistem: “{}”{code}.", st.text),
            (Said::Ncz, None) => format!(
                "Dosyanın bildirdiği sistem bir EPSG koduna karşılık gelmiyor (“{}”); koordinatların sistemini siz seçin.",
                st.text
            ),
            (Said::Nothing, _) => "Dosya koordinat sistemini belirtmiyor (.prj yok): projenin sistemi seçili; koordinatların bu sistemde olduğundan emin olun.".to_owned(),
        })
    }

    /// Coordinates that cannot be in the chosen system (degrees in a metre system, or the reverse).
    fn implausible(&self, chosen: &System, format: &Format) -> Option<String> {
        let b = self.bounds.as_ref()?;
        let degrees = b.min_x.abs() <= 180.0
            && b.max_x.abs() <= 180.0
            && b.min_y.abs() <= 90.0
            && b.max_y.abs() <= 90.0;
        if chosen.kind == "geographic" && !degrees {
            return Some(format!(
                "Koordinatlar boylam, enlem aralığının dışında ({} … {}): {} olamaz. Dosyayı yazan program metre koordinatı yazmış olabilir; sistemini biliyorsanız onu seçin.",
                format.coord(b.min_x),
                format.coord(b.max_x),
                chosen.name
            ));
        }
        if chosen.kind == "projected" && degrees {
            return Some(format!(
                "Koordinatların hepsi ±180, ±90 içinde: derece (boylam, enlem) gibi görünüyor; {} sisteminde anlamsız bir yere düşer. Dosyanın sistemini denetleyin.",
                chosen.name
            ));
        }
        None
    }

    /// The lines under the list (the web's `render`).
    pub fn notes(&self, own: &Project, format: &Format) -> Vec<Note> {
        let project = own.srid;
        let project_name = own.name();
        let mut notes: Vec<Note> = self.said().map(Note::Hint).into_iter().collect();
        let st = self.statement.as_ref();
        if let Some(srid) = st.and_then(|st| st.srid)
            && system(srid).is_none()
        {
            notes.push(Note::Warn(format!(
                "Dosyanın dediği EPSG:{srid} KentOS'ta tanımlı değil. Koordinatlar gerçekte projenin sisteminde ise onu seçin; değilse bu dosya dönüştürülmeden alınamaz."
            )));
        }
        let Some(source) = self.srid.and_then(system) else {
            if self.srid.is_none() {
                notes.push(Note::Info("Koordinatların hangi sistemde olduğunu seçin; içe aktarma ancak seçilen sistem projeninki olunca açılır.".to_owned()));
            } else {
                notes.push(Note::Hint(format!(
                    "Projenin sistemi ({project_name}). Koordinat sistemi dönüştürülmez; değerler yuvarlanmaz."
                )));
            }
            return notes;
        };
        if let Some(st) = st
            && st.source != Said::Nothing
            && let Some(said) = st.srid
            && said != source.srid
        {
            notes.push(Note::Warn(format!(
                "Dosyanın dediğinden başka bir sistem seçtiniz. Dosya EPSG:{said} diyor; koordinatlar {} (EPSG:{}) sayılacak, dönüştürülmeden. Yalnız dosyanın gerçekte bu sistemde olduğunu biliyorsanız seçin.",
                source.name, source.srid
            )));
        }
        match system(project) {
            _ if own.local() && source.srid != project => notes.push(Note::Info(format!(
                "Proje yerel (koordinat sistemi yok): {} koordinatları olduğu gibi alınır; dönüştürülmez, yuvarlanmaz ve bir konuma bağlanmaz.",
                title(source)
            ))),
            _ if self.matches(own) => notes.push(Note::Hint(format!(
                "Projenin sistemi ({project_name}). Koordinat sistemi dönüştürülmez; değerler yuvarlanmaz."
            ))),
            // The project's own definition (docs/adr/0168 §1): no file's EPSG code is it.
            _ if own.defined.is_some() => notes.push(Note::Warn(format!(
                "Koordinatlar dönüştürülemez. Projenin sistemi kendi tanımı: {project_name}. {} koordinatları içe aktarılırken bu sisteme dönüştürülmez; bu yüzden içe aktarma kapalı. Dosya aslında projenin sistemindeyse listede onu seçin.",
                source.name
            ))),
            None => notes.push(Note::Warn(format!(
                "Proje EPSG:{project} sisteminde; bu sistem bu sürümde tanımlı değil. Koordinatlar dönüştürülemez, bu yüzden içe aktarma kapalı."
            ))),
            Some(target) => {
                let what = if source.datum != target.datum {
                    format!(
                        "{} → {} datum dönüşümü",
                        datum_label(&source.datum),
                        datum_label(&target.datum)
                    )
                } else if source.kind != target.kind {
                    "coğrafi ile projeksiyonlu sistem arasında dönüşüm".to_owned()
                } else {
                    "dilim dönüşümü".to_owned()
                };
                notes.push(Note::Warn(format!(
                    "Koordinatlar dönüştürülemez. Proje {project_name} (EPSG:{project}) sisteminde. {} koordinatlarını almak için {what} gerekir; bu dönüşüm henüz yok (geliştirme aşamasında) ve koordinatlar sessizce dönüştürülmez, bu yüzden içe aktarma kapalı. Dosya aslında projenin sistemindeyse onu seçin; proje de bu sistemdeyse projenin sistemini Proje ayarları → Koordinat sistemi'nden atayın.",
                    source.name
                )));
            }
        }
        if let Some(odd) = self.implausible(source, format) {
            notes.push(Note::Warn(odd));
        }
        notes
    }

    /// The question: the list of systems and the notes under it.
    pub fn view<'a, Message: Clone + 'a>(
        &self,
        own: &Project,
        format: &Format,
        on_pick: impl Fn(u32) -> Message + 'a,
    ) -> Element<'a, Message> {
        let project = own.srid;
        let list = systems();
        let said = self
            .statement
            .as_ref()
            .filter(|st| st.source != Said::Nothing)
            .and_then(|st| st.srid);
        let choices = list.iter().map(|s| {
            let is_own = if s.srid == project {
                ", projenin sistemi"
            } else {
                ""
            };
            let file = if said == Some(s.srid) {
                ", dosyanın dediği"
            } else {
                ""
            };
            // SRID 0 is the project's own definition when it has one (docs/adr/0168 §1).
            let name = match &own.defined {
                Some(defined) if s.is_local() => defined.clone(),
                _ => title(s),
            };
            Choice::new(format!("{name}{is_own}{file}")).detail(datum_label(&s.datum))
        });
        let selected = self
            .srid
            .and_then(|srid| list.iter().position(|s| s.srid == srid));
        let pick = Select::new(choices, selected, move |i| {
            on_pick(list.get(i).map_or(project, |s| s.srid))
        })
        .placeholder("Sistemi seçin…");
        let mut parts = column![label::caption("Bu koordinatlar hangi sistemde?"), pick].spacing(6);
        for note in self.notes(own, format) {
            parts = parts.push(match note {
                Note::Hint(hint) => Element::from(label::caption(hint)),
                Note::Info(info) => Banner::info(info).into(),
                Note::Warn(warning) => Banner::warning(warning).into(),
            });
        }
        parts.into()
    }
}

/// The systems a search finds (the web's `searchCrs`): the start of the
/// SRID (`epsg:` left out), or a part of the name or the area.
pub fn search(query: &str) -> Vec<&'static System> {
    let q = query.trim().to_lowercase();
    let q = q
        .strip_prefix("epsg:")
        .or_else(|| q.strip_prefix("epsg"))
        .unwrap_or(&q)
        .trim();
    systems()
        .iter()
        .filter(|s| {
            q.is_empty()
                || s.srid.to_string().starts_with(q)
                || s.name.to_lowercase().contains(q)
                || s.area
                    .as_deref()
                    .is_some_and(|a| a.to_lowercase().contains(q))
        })
        .collect()
}

/// The searchable list of systems grouped by datum, the chosen one's card
/// and its parameters, and the notes (the web's `crsPicker`): the open
/// project's system, assigned on save, never a transformation. A new
/// project chooses its system in the wizard (project/wizard/).
pub fn picker<'a, Message: Clone + 'a>(
    value: u32,
    initial: u32,
    default_srid: u32,
    query: &str,
    on_pick: impl Fn(u32) -> Message + 'a,
    on_search: impl Fn(String) -> Message + 'a,
) -> Element<'a, Message> {
    let Some(chosen) = system(value) else {
        return label::caption(format!("EPSG:{value} bu sürümde tanımlı değil.")).into();
    };
    let changed = value != initial;
    let heading = if changed {
        "Kaydedince projeye atanacak sistem"
    } else {
        "Projenin koordinat sistemi"
    };
    let current = container(
        column![
            label::caption(heading),
            row![
                text(chosen.name.clone())
                    .font(typography::ui_strong())
                    .size(typography::body()),
                container(label::mono_caption(code(chosen)))
                    .padding([1, 6])
                    .style(style::container::badge),
            ]
            .spacing(8)
            .align_y(Center),
        ]
        .spacing(4),
    )
    .padding([8, 10])
    .width(Fill)
    .style(move |theme: &iced::Theme| {
        let base = style::container::bordered(theme);
        if !changed {
            return base;
        }
        // About to change: the web's amber edge (“değişecek”).
        iced::widget::container::Style {
            border: iced::Border {
                color: kentos_ui::theme::Tokens::of(theme).warning,
                ..base.border
            },
            ..base
        }
    });

    let found = search(query);
    let digits: String = query.chars().filter(char::is_ascii_digit).collect();
    let unknown = !digits.is_empty()
        && digits.len() >= 4
        && query
            .trim()
            .trim_start_matches(|c: char| !c.is_ascii_digit())
            .chars()
            .all(|c| c.is_ascii_digit())
        && digits.parse::<u32>().ok().and_then(system).is_none();
    let mut list = Column::new().spacing(2);
    let mut datum = "";
    for s in &found {
        if s.datum != datum {
            datum = &s.datum;
            list = list.push(container(label::caption(datum_label(datum))).padding(
                iced::Padding {
                    top: 6.0,
                    right: 4.0,
                    bottom: 2.0,
                    left: 4.0,
                },
            ));
        }
        let tag: Element<'a, Message> = if s.srid == default_srid {
            container(label::caption("varsayılan"))
                .padding([0, 5])
                .style(style::container::badge)
                .into()
        } else {
            label::caption("").into()
        };
        let face = row![
            container(label::mono(s.srid.to_string())).width(52),
            row![label::body(s.name.clone()), tag]
                .spacing(6)
                .align_y(Center)
                .width(Fill),
            label::caption(s.area.clone().unwrap_or_default()),
        ]
        .spacing(8)
        .align_y(Center);
        list = list.push(
            button(face)
                .on_press(on_pick(s.srid))
                .padding([4, 6])
                .width(Fill)
                .style(style::button::list_item(s.srid == value)),
        );
    }
    if found.is_empty() {
        list = list.push(container(label::muted("Eşleşen koordinat sistemi yok.")).padding(8));
    }
    let search_box = text_input("SRID ya da ad yazın, ör. 5256 veya TM36", query)
        .on_input(on_search)
        .padding([5, 8])
        .size(typography::body())
        .style(style::field::input);
    let search_box = kentos_ui::widget::focus_ring(search_box);
    let mut browser = Column::new().spacing(6).push(
        row![icon(Icon::Search).size(14.0).tone(Tone::Muted), search_box]
            .spacing(6)
            .align_y(Center),
    );
    if unknown {
        browser = browser.push(label::caption(format!(
            "EPSG:{digits} bu sürümde tanımlı değil. Listedeki sistemlerden birini seçin."
        )));
    }
    let browser = browser.push(
        container(
            scrollable(list)
                .direction(style::field::body_scrollbar())
                .height(Length::Fixed(230.0)),
        )
        .style(style::container::bordered),
    );

    let mut details: Vec<(&str, String)> = vec![(
        "Tür",
        match chosen.kind.as_str() {
            "projected" => "Projeksiyonlu (metre)",
            "local" => "Yerel: koordinat sistemi yok (metre)",
            _ => "Coğrafi (derece)",
        }
        .to_owned(),
    )];
    if !chosen.is_local() {
        details.push(("Datum", datum_label(&chosen.datum).to_owned()));
    }
    if let Some(e) = &chosen.ellipsoid {
        details.push(("Elipsoid", e.clone()));
    }
    if let Some(p) = &chosen.projection {
        details.push((
            "Projeksiyon",
            if p == "UTM" {
                "UTM (Transverse Mercator)".to_owned()
            } else {
                p.clone()
            },
        ));
    }
    if let Some(cm) = chosen.central_meridian {
        details.push(("Orta meridyen", format!("{}° D", js_number(cm))));
    }
    if let Some(k) = chosen.scale_factor {
        details.push(("Ölçek faktörü", js_number(k)));
    }
    if let Some(e) = chosen.false_easting {
        details.push(("Sağa öteleme", format!("{} m", grouped(e))));
    }
    details.push(("Kapsam", chosen.area.clone().unwrap_or_default()));
    let mut card = Column::new().spacing(4).push(
        text(chosen.name.clone())
            .font(typography::ui_strong())
            .size(typography::body()),
    );
    for (k, v) in details {
        card = card.push(row![container(label::caption(k)).width(110), label::body(v)].spacing(8));
    }
    if chosen.kind == "projected" {
        card = card.push(label::caption(
            "Eksen sırası: Y sağa değer, X yukarı değer.",
        ));
    }
    let card = container(card)
        .padding(10)
        .width(Fill)
        .style(style::container::bordered);

    let mut notes = Column::new().spacing(6);
    if changed {
        let before = system(initial);
        let datum_note = match before {
            Some(b) if b.datum != chosen.datum => format!(
                " {} → {} geçişi için datum dönüşümü gerekir (geliştirme aşamasında).",
                datum_label(&b.datum),
                datum_label(&chosen.datum)
            ),
            _ => String::new(),
        };
        notes = notes.push(Banner::warning(format!(
            "Koordinatlar dönüştürülmez. Kaydettiğinizde proje yalnızca {} olarak etiketlenir; mevcut Y/X değerleri aynı kalır.{datum_note}",
            chosen.name
        )));
    }
    if chosen.kind == "geographic" {
        notes = notes.push(Banner::info(
            "Coğrafi sistemlerde birim derecedir. Çizim ve ölçüm araçları metre cinsinden projeksiyonlu bir sistem bekler.",
        ));
    }
    if chosen.is_local() {
        notes = notes.push(Banner::info(
            "Yerel: koordinatlar bir konuma bağlı değildir (teknik çizim, başlangıç 0,0). Koordinat sistemi taşıyan CBS verisi olduğu gibi gelir; dışa aktarılan CBS dosyalarında koordinat sistemi yazılmaz. Gerçek konum için bir sistem seçin.",
        ));
    }
    column![
        current,
        row![
            container(browser).width(Length::FillPortion(3)),
            container(card).width(Length::FillPortion(2))
        ]
        .spacing(12),
        notes
    ]
    .spacing(10)
    .into()
}

/// A number as JavaScript writes it (`${27}` → “27”, `${0.9996}` → “0.9996”).
fn js_number(v: f64) -> String {
    if v.fract() == 0.0 && v.abs() < 1e15 {
        format!("{}", v as i64)
    } else {
        format!("{v}")
    }
}

/// A whole number in Turkish grouping (`500000` → “500.000”, the web's `toLocaleString('tr-TR')`).
pub fn grouped(v: f64) -> String {
    let n = v.round() as i64;
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push('.');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_list_is_the_web_registry_grouped_by_datum() {
        let datums: Vec<&str> = systems().iter().map(|s| s.datum.as_str()).collect();
        // The local system first (docs/adr/0165 §2), then the datums.
        assert_eq!(datums[0], "LOCAL");
        let first_ed50 = datums.iter().position(|d| *d == "ED50").expect("ED50");
        let first_wgs = datums.iter().position(|d| *d == "WGS84").expect("WGS 84");
        assert!(datums[1..first_ed50].iter().all(|d| *d == "TUREF"));
        assert!(datums[first_ed50..first_wgs].iter().all(|d| *d == "ED50"));
        assert!(datums[first_wgs..].iter().all(|d| *d == "WGS84"));
        assert_eq!(system(5256).map(|s| s.name.as_str()), Some("TUREF / TM36"));
    }

    #[test]
    fn a_search_finds_by_code_name_or_area_as_on_the_web() {
        let codes = |q: &str| search(q).iter().map(|s| s.srid).collect::<Vec<_>>();
        assert_eq!(codes("EPSG:5256"), [5256]);
        assert!(codes("tm36").contains(&5256) && codes("tm36").contains(&2322));
        assert_eq!(codes("").len(), systems().len());
        assert!(codes("coğrafi").contains(&4326));
        assert_eq!(grouped(500_000.0), "500.000");
        assert_eq!(grouped(25_000.0), "25.000");
    }

    fn warnings(q: &CrsQuestion, project: u32) -> Vec<String> {
        q.notes(&project.into(), &Format::default())
            .into_iter()
            .filter_map(|n| match n {
                Note::Warn(w) => Some(w),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn only_the_projects_system_imports_and_the_reason_is_named() {
        let q = CrsQuestion::new(5256);
        assert!(q.matches(&5256.into()) && warnings(&q, 5256).is_empty());
        let mut zone = CrsQuestion::new(5256);
        zone.pick(5254);
        let zone = warnings(&zone, 5256).join(" ");
        assert!(zone.contains("dilim dönüşümü"), "{zone}");
        let mut datum = CrsQuestion::new(5256);
        datum.pick(2322);
        let datum = warnings(&datum, 5256).join(" ");
        assert!(
            datum.contains("ED50 → TUREF (ITRF96) datum dönüşümü"),
            "{datum}"
        );
        let mut kind = CrsQuestion::new(5256);
        kind.pick(5252);
        let kind = warnings(&kind, 5256).join(" ");
        assert!(kind.contains("coğrafi ile projeksiyonlu"), "{kind}");
    }

    /// A local project (SRID 0, docs/adr/0165 §2) takes any file's coordinates
    /// as they are, and says so; it is named as no coordinate system.
    #[test]
    fn a_local_project_takes_any_files_coordinates_as_they_are() {
        let mut q = CrsQuestion::new(LOCAL_SRID);
        q.declare(
            LOCAL_SRID,
            statement(Some(5256), "EPSG:5256", Said::Prj),
            None,
        );
        assert!(q.matches(&LOCAL_SRID.into()));
        assert!(warnings(&q, LOCAL_SRID).is_empty());
        let notes = q.notes(&LOCAL_SRID.into(), &Format::default());
        assert!(
            notes.iter().any(|n| matches!(n, Note::Info(i)
                if i.starts_with("Proje yerel (koordinat sistemi yok): TUREF / TM36 (EPSG:5256)"))),
            "{notes:?}"
        );
        let local = system(LOCAL_SRID).expect("the local system");
        assert!(local.is_local() && local.ellipsoid.is_none());
        assert_eq!(title(local), "Yerel (koordinat sistemi yok)");
        assert_eq!(
            (code_of(LOCAL_SRID), title_of(5256)),
            ("SRID 0".to_owned(), "TUREF / TM36 (EPSG:5256)".to_owned())
        );
    }

    /// A project whose system is its own definition (docs/adr/0168 §1) is
    /// named by it, and takes only coordinates said to be in it: SRID 0's
    /// place in the list is the definition's.
    #[test]
    fn a_projects_own_definition_takes_only_its_own_coordinates() {
        let settings: ProjectSettings = serde_json::from_str(
            r#"{"srid":0,"lengthDecimals":3,"areaDecimals":2,"areaUnit":"m2","angleUnit":"grad","plotScale":1000,
                "customCrs":{"name":"Şantiye","system":{"kind":"tm","datum":"TUREF","centralMeridian":33,
                "scaleFactor":1,"falseEasting":500000,"falseNorthing":0}}}"#,
        )
        .expect("settings");
        assert_eq!(
            (
                project_name(&settings),
                project_title(&settings),
                project_code(&settings)
            ),
            (
                "Şantiye".to_owned(),
                "Şantiye (özel sistem)".to_owned(),
                "Özel sistem".to_owned()
            )
        );
        let own = Project::of(&settings);
        let mut q = CrsQuestion::new(LOCAL_SRID);
        assert!(q.matches(&own));
        assert!(
            q.notes(&own, &Format::default()).iter().any(
                |n| matches!(n, Note::Hint(h) if h.starts_with("Projenin sistemi (Şantiye (özel sistem))"))
            )
        );
        q.declare(
            LOCAL_SRID,
            statement(Some(5256), "EPSG:5256", Said::Prj),
            None,
        );
        assert!(!q.matches(&own));
        let w = q
            .notes(&own, &Format::default())
            .into_iter()
            .filter_map(|n| match n {
                Note::Warn(w) => Some(w),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join(" ");
        assert!(
            w.contains(
                "Projenin sistemi kendi tanımı: Şantiye (özel sistem). TUREF / TM36 koordinatları"
            ),
            "{w}"
        );
        // Without the definition the project has no system.
        let local = ProjectSettings {
            custom_crs: None,
            ..settings
        };
        assert_eq!(
            (project_title(&local), project_code(&local)),
            (
                "Yerel (koordinat sistemi yok)".to_owned(),
                "SRID 0".to_owned()
            )
        );
        assert!(CrsQuestion::new(LOCAL_SRID).matches(&Project::of(&local)));
    }

    fn statement(srid: Option<u32>, text: &str, source: Said) -> Option<Statement> {
        Some(Statement {
            srid,
            text: text.to_owned(),
            source,
        })
    }

    #[test]
    fn a_files_statement_is_the_first_answer_and_never_a_guess() {
        let degrees = Bounds {
            min_x: 32.8,
            min_y: 39.9,
            max_x: 32.9,
            max_y: 40.0,
        };
        // RFC 7946: WGS 84, which a TM36 project cannot take in.
        let mut q = CrsQuestion::new(5256);
        q.declare(
            5256,
            statement(Some(4326), "", Said::Rfc7946),
            Some(degrees),
        );
        assert_eq!(q.srid, Some(4326));
        assert!(!q.matches(&5256.into()));
        let notes = q.notes(&5256.into(), &Format::default());
        assert!(matches!(&notes[0], Note::Hint(h) if h.starts_with("Dosya: RFC 7946")));
        // The user says it is TM36 after all: allowed, and what it means is said;
        // degrees in a metre system are pointed out.
        q.pick(5256);
        assert!(q.matches(&5256.into()));
        let w = warnings(&q, 5256).join(" ");
        assert!(w.contains("Dosya EPSG:4326 diyor"), "{w}");
        assert!(w.contains("derece (boylam, enlem) gibi görünüyor"), "{w}");
        // A .prj that could not be read: no answer until the user gives one.
        q.declare(5256, statement(None, "Garip_Sistem", Said::Prj), None);
        assert_eq!(q.srid, None);
        assert!(!q.matches(&5256.into()));
        assert!(
            q.notes(&5256.into(), &Format::default()).iter().any(
                |n| matches!(n, Note::Info(i) if i.starts_with("Koordinatların hangi sistemde"))
            )
        );
        // A system KentOS does not know: said, and no answer.
        q.declare(
            5256,
            statement(Some(2193), "EPSG:2193", Said::GeoJsonCrs),
            None,
        );
        assert_eq!(q.srid, None);
        assert!(
            warnings(&q, 5256)
                .join(" ")
                .contains("EPSG:2193 KentOS'ta tanımlı değil")
        );
        // Nothing said (no .prj): the project's system, and the user is asked to be sure.
        q.declare(5256, statement(None, "", Said::Nothing), None);
        assert!(q.matches(&5256.into()));
        assert!(
            matches!(&q.notes(&5256.into(), &Format::default())[0], Note::Hint(h) if h.contains(".prj yok"))
        );
        // Metres in a geographic system.
        let metres = Bounds {
            min_x: 452_000.0,
            min_y: 4_420_000.0,
            max_x: 453_000.0,
            max_y: 4_421_000.0,
        };
        q.declare(5256, statement(Some(4326), "", Said::Rfc7946), Some(metres));
        assert!(
            warnings(&q, 5256)
                .join(" ")
                .contains("boylam, enlem aralığının dışında")
        );
    }
}
